use super::*;

impl World {
    /// C `find_free_npc` (`lq.c:221-230`).
    pub(super) fn find_free_lq_npc_slot(&self) -> Option<usize> {
        (1..MAX_LQ_NPCS).find(|slot| !self.lq_npcs.iter().any(|npc| npc.slot == *slot))
    }

    /// C `log_sys(cn, COL_LIGHT_RED "...", ...)` - `COL_LIGHT_RED`
    /// (`\xb0c3`) is not valid UTF-8, so error text goes through the
    /// byte-payload sibling of [`Self::queue_system_text`] (see
    /// [`WorldSystemTextBytes`]'s own doc comment for the same pattern
    /// used by `give_money_message`).
    pub fn queue_lq_error(&mut self, character_id: CharacterId, message: impl AsRef<str>) {
        let mut bytes = crate::text::COL_LIGHT_RED.to_vec();
        bytes.extend_from_slice(message.as_ref().as_bytes());
        self.queue_system_text_bytes(character_id, bytes);
    }

    /// The nick-or-slot-ID target resolution idiom repeated in (almost)
    /// every `cmd_*` handler: `n = atoi(nick); if (n>0 && n<MAXLQNPC) {
    /// single slot, only if populated } else { scan for a nick[0]/nick[1]
    /// case-insensitive match, optionally also literal "all" }`.
    pub(super) fn resolve_lq_npc_slots(&self, nick: &str, allow_all: bool) -> Vec<usize> {
        let numeric = legacy_atoi(nick);
        if numeric > 0 {
            let slot = numeric as usize;
            return if slot < MAX_LQ_NPCS && self.lq_npcs.iter().any(|npc| npc.slot == slot) {
                vec![slot]
            } else {
                Vec::new()
            };
        }
        self.lq_npcs
            .iter()
            .filter(|npc| {
                npc.nick[0].eq_ignore_ascii_case(nick)
                    || npc.nick[1].eq_ignore_ascii_case(nick)
                    || (allow_all && nick.eq_ignore_ascii_case("all"))
            })
            .map(|npc| npc.slot)
            .collect()
    }

    /// Applies `f` to every `lq_npcs` entry named in `slots`, returning
    /// how many were found (C's `cnt`).
    pub(super) fn lq_admin_apply_to_targets(
        &mut self,
        slots: &[usize],
        mut f: impl FnMut(&mut LqNpcState),
    ) -> usize {
        let mut count = 0;
        for slot in slots {
            if let Some(npc) = self.lq_npcs.iter_mut().find(|npc| npc.slot == *slot) {
                f(npc);
                count += 1;
            }
        }
        count
    }

    /// C `get_lq_item` (`lq.c:319-355`), minus the `lookup_item`
    /// existence check (see the module doc comment). Emits the same
    /// "Missing base"/"Trailing garbage" messages as the caller's own
    /// `usage` string on failure, matching C exactly.
    pub(super) fn lq_admin_parse_item(
        &mut self,
        character_id: CharacterId,
        reader: &mut ArgReader,
        usage: &str,
    ) -> Option<LqItemSpec> {
        let Some(base) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing base. Usage is: {usage}."));
            return None;
        };
        let key_id = reader.take_int().unwrap_or(0);
        let name = reader.take_str().unwrap_or_default();
        let description = reader.take_str().unwrap_or_default();
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {usage}."),
            );
            return None;
        }
        Some(LqItemSpec {
            base,
            name,
            description,
            key_id: (key_id as i32) as u32,
        })
    }

    /// Top-level entry point: `special_driver`'s `#`/`/`-prefixed,
    /// `CF_GOD|CF_LQMASTER`-gated branch (`lq.c:2514-2622`, this slice's
    /// portion of it). Returns `false` (C's `return 1`, "not handled")
    /// for anything outside area 20/35, not `#`/`/`-prefixed, unmatched,
    /// or typed by a character lacking the permission flags - the caller
    /// should fall through to normal command/speech processing exactly
    /// as C does. Returns `true` (C's `return 2`) once a command in this
    /// slice's table has run, with all caller-facing feedback already
    /// queued via [`Self::queue_system_text`]/[`Self::
    /// queue_system_text_bytes`].
    pub fn apply_lq_admin_command(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
        command: &str,
    ) -> bool {
        if area_id != 20 && area_id != 35 {
            return false;
        }
        let trimmed = command.trim_start();
        let Some(rest) = trimmed
            .strip_prefix('#')
            .or_else(|| trimmed.strip_prefix('/'))
        else {
            return false;
        };
        let mut reader = ArgReader::new(rest);
        let Some(word) = reader.take_str() else {
            return false;
        };
        let Some(flags) = self
            .characters
            .get(&character_id)
            .map(|character| character.flags)
        else {
            return false;
        };
        if !flags.intersects(CharacterFlags::GOD | CharacterFlags::LQMASTER) {
            return false;
        }
        let args = reader.rest;

        if cmd_word_matches(&word, "npc", 3) {
            self.lq_admin_cmd_npc(character_id, args);
            return true;
        }
        // C checks `thrall` here too (`lq.c:2531-2534`), but it needs a
        // fresh character (`spawn_npc`'s `isthrall` branch), so it is
        // dispatched by its own `World::try_dispatch_lq_thrall` first -
        // see that method's doc comment (same precedent as `#nspawn`).
        if cmd_word_matches(&word, "killthrall", 3) {
            self.lq_admin_cmd_killthrall(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "npcname", 5) {
            self.lq_admin_cmd_npcname(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "npcgold", 5) {
            self.lq_admin_cmd_npcgold(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "npcsprite", 5) {
            self.lq_admin_cmd_npcsprite(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "npcpos", 5) {
            self.lq_admin_cmd_npcpos(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "npcdescription", 5) {
            self.lq_admin_cmd_npcdesc(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "npcgreeting", 5) {
            self.lq_admin_cmd_npcgreet(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "npcreply", 5) {
            self.lq_admin_cmd_npcreply(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "npclist", 5) {
            self.lq_admin_cmd_npclist(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "npcdelete", 5) {
            self.lq_admin_cmd_npcdel(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "npcwantitem", 5) {
            self.lq_admin_cmd_npcwantitem(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "npcitem", 5) {
            self.lq_admin_cmd_npcitem(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "npcshow", 5) {
            self.lq_admin_cmd_npcshow(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "npckillmark", 5) {
            self.lq_admin_cmd_npckillmark(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "npchurtmark", 5) {
            self.lq_admin_cmd_npchurtmark(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "npcrewarditem", 5) {
            self.lq_admin_cmd_npcrewarditem(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "npcmodlevel", 5) {
            self.lq_admin_cmd_npcmodlevel(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "npcrespawn", 5) {
            self.lq_admin_cmd_npcrespawn(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "doorlist", 6) {
            self.lq_admin_cmd_doorlist(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "doorlock", 6) {
            self.lq_admin_cmd_doorlock(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "nremove", 5) {
            self.lq_admin_cmd_nremove(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "nsay", 4) {
            self.lq_admin_cmd_nsay(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "nimmortal", 4) {
            self.lq_admin_cmd_nimmortal(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "nemote", 4) {
            self.lq_admin_cmd_nemote(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "nattack", 4) {
            self.lq_admin_cmd_nattack(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "questlevel", 8) {
            self.lq_admin_cmd_questlevel(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "questreward", 8) {
            self.lq_admin_cmd_questreward(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "questshow", 8) {
            self.lq_admin_cmd_questshow(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "questentrance", 8) {
            self.lq_admin_cmd_questentrance(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "queststart", 8) {
            self.lq_admin_cmd_queststart(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "questreset", 10) {
            self.lq_admin_cmd_questreset(character_id, args);
            return true;
        }

        false
    }
}
