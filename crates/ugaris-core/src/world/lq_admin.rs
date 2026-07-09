//! `CDR_LQPARSER` admin command table (`special_driver`, `src/area/20/
//! lq.c:2505-2742`): the area-gated (`areaID == 20 || areaID == 35`)
//! `#`/`/`-prefixed text-command interceptor every `CF_GOD`/`CF_LQMASTER`
//! character in the Live Quest area can use to author quest content.
//! `special_driver` is dispatched purely by `command()`'s own area gate
//! (`system/command.c:5855-5859`), independent of `CharacterDriverKind`/
//! spawned-character iteration - `CDR_LQPARSER` is never assigned to any
//! spawned character (see [`crate::world::npc::area20::lqnpc`]'s module
//! doc comment for the sibling `CDR_LQNPC` driver this table authors).
//!
//! First slice: the NPC-template CRUD family (`#npc`, `#npcname`,
//! `#npcgold`, `#npcsprite`, `#npcpos`, `#npcdescription`,
//! `#npcgreeting`, `#npcreply`, `#npclist`, `#npcdelete`,
//! `#npcwantitem`, `#npcitem`, `#npcshow`, `#npckillmark`,
//! `#npchurtmark`, `#npcrewarditem`, `#npcmodlevel`, `#npcrespawn` - 18
//! of the ~45-entry table). All mutate only [`crate::world::LqNpcState`]
//! template rows already modeled by `world::lq`; none of them need
//! `ZoneLoader`/`PlayerRuntime`, so the entire slice is pure `World`
//! logic, unlike most of the rest of this port.
//!
//! Deliberately NOT ported in this slice (see `PORTING_TODO.md`'s Area 20
//! entry for the follow-on plan):
//! - `#thrall`/`#killthrall` - need `DRD_LQ_NPC_DATA.thrallname`, not
//!   modeled on [`crate::world::npc::area20::lqnpc::LqNpcDriverData`].
//! - `#usurp`/`#follow`/`#stop`/`#exit`, and the possessed-NPC-relay
//!   plain-speech branch - need a new `PlayerRuntime.usurp` field.
//! - `#doorlist`/`#doorlock`, `#nspawn`/`#nremove`/`#nsay`/`#nimmortal`/
//!   `#nemote`/`#nattack`, `#wimp` - live-instance control, some need
//!   `ZoneLoader`/`ServerRuntime`.
//! - `#questsave`/`#questdelete`/`#questend`/`#questload`/`#questshow`/
//!   `#questreward`/`#questlevel`/`#questreset`/`#questentrance`/
//!   `#queststart`, `#xinfo` - quest-lifecycle state (`struct lq_data` has
//!   no `World` equivalent yet) plus (for save/delete/load) novel file
//!   I/O this codebase has no precedent for.
//!
//! Two C existence checks are permanently deferred rather than ported:
//! `cmd_npc`'s `lookup_char("lq_"+basename)` and `get_lq_item`'s
//! `lookup_item("lq_"+basename)` (used by `#npcitem`/`#npcrewarditem`) -
//! `World` has no character/item-template registry (only
//! `ugaris-server`'s `ZoneLoader` does). A bad basename is silently
//! accepted here and will simply no-op once the (still-unported)
//! `#nspawn` reaches `ZoneLoader::instantiate_character_template`,
//! matching the existing `spawns.rs::spawn_lq_npc_character` precedent
//! for the *scheduled*-respawn path.
//!
//! `#npcshow`'s hurt/killmark exp-preview line reads `lq_data.reward[]`/
//! `reward_desc[]` in C - genuinely always empty/zero here (`0 exp`, no
//! description) since `struct lq_data` and `#questreward` (the only C
//! code path that ever populates it) are not ported yet either, so this
//! is not a simplification, just an accurate reflection of a table with
//! no ported writer.

use super::*;

/// C `cmdcmp(ptr, cmd, minlen)` (`system/command.c:217-234`): `word` is a
/// case-insensitive prefix of `full`, at least `minlen` characters long.
fn cmd_word_matches(word: &str, full: &str, minlen: usize) -> bool {
    let lower = word.to_ascii_lowercase();
    lower.len() >= minlen && full.starts_with(&lower)
}

/// C `atoi` (used throughout `lq.c` on already-tokenized single-word
/// strings, e.g. `n = atoi(nick)`): leading-whitespace-then-optional-
/// sign-then-digits, `0` if there are no leading digits at all.
fn legacy_atoi(input: &str) -> i64 {
    let trimmed = input.trim_start();
    let mut chars = trimmed.chars().peekable();
    let sign = match chars.peek() {
        Some('-') => {
            chars.next();
            -1
        }
        Some('+') => {
            chars.next();
            1
        }
        _ => 1,
    };
    let mut value: i64 = 0;
    let mut any_digit = false;
    for ch in chars {
        match ch.to_digit(10) {
            Some(digit) => {
                any_digit = true;
                value = value * 10 + i64::from(digit);
            }
            None => break,
        }
    }
    if any_digit {
        value * sign
    } else {
        0
    }
}

/// C `get_str`/`get_int`/`get_chr`/`check_anything` (`lq.c:238-317`): the
/// tiny space/quote-aware argument tokenizer shared by every `cmd_*`
/// handler in `special_driver`'s command table.
struct ArgReader<'a> {
    rest: &'a str,
}

impl<'a> ArgReader<'a> {
    fn new(input: &'a str) -> Self {
        ArgReader { rest: input }
    }

    fn skip_ws(&mut self) {
        self.rest = self.rest.trim_start();
    }

    /// C `get_str`.
    fn take_str(&mut self) -> Option<String> {
        self.skip_ws();
        if self.rest.is_empty() {
            return None;
        }
        let quoted = matches!(self.rest.chars().next(), Some('"') | Some('\''));
        let mut consumed = 0usize;
        if quoted {
            consumed += self.rest.chars().next().map(char::len_utf8).unwrap_or(0);
        }
        let mut out = String::new();
        for ch in self.rest[consumed..].chars() {
            if quoted && ch == '"' {
                break;
            }
            if !quoted && ch.is_whitespace() {
                break;
            }
            out.push(ch);
            consumed += ch.len_utf8();
        }
        // C also eats any remaining non-whitespace after the token
        // itself (`lq.c:285-287`) - only relevant for a truncated/
        // unterminated quoted token butchered by the length limit.
        let tail_start = self.rest[consumed..]
            .find(char::is_whitespace)
            .map(|offset| consumed + offset)
            .unwrap_or(self.rest.len());
        self.rest = &self.rest[tail_start..];
        Some(out)
    }

    /// C `get_int`.
    fn take_int(&mut self) -> Option<i64> {
        self.skip_ws();
        let negative = self.rest.starts_with('-');
        let digits_start = usize::from(negative);
        let has_digit = self.rest[digits_start..]
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit());
        if !has_digit {
            return None;
        }
        let mut end = digits_start;
        for ch in self.rest[digits_start..].chars() {
            if ch.is_ascii_digit() {
                end += ch.len_utf8();
            } else {
                break;
            }
        }
        let token = &self.rest[..end];
        let value = token.parse::<i64>().unwrap_or(0);
        self.rest = &self.rest[end..];
        Some(value)
    }

    /// C `get_chr`.
    fn take_chr(&mut self) -> Option<char> {
        self.skip_ws();
        let first = self.rest.chars().next()?;
        if !first.is_ascii_alphabetic() {
            return None;
        }
        let tail_start = self
            .rest
            .find(char::is_whitespace)
            .unwrap_or(self.rest.len());
        self.rest = &self.rest[tail_start..];
        Some(first)
    }

    /// C `check_anything`.
    fn has_trailing_garbage(&self) -> bool {
        !self.rest.trim_start().is_empty()
    }
}

impl World {
    /// C `find_free_npc` (`lq.c:221-230`).
    fn find_free_lq_npc_slot(&self) -> Option<usize> {
        (1..MAX_LQ_NPCS).find(|slot| !self.lq_npcs.iter().any(|npc| npc.slot == *slot))
    }

    /// C `log_sys(cn, COL_LIGHT_RED "...", ...)` - `COL_LIGHT_RED`
    /// (`\xb0c3`) is not valid UTF-8, so error text goes through the
    /// byte-payload sibling of [`Self::queue_system_text`] (see
    /// [`WorldSystemTextBytes`]'s own doc comment for the same pattern
    /// used by `give_money_message`).
    fn queue_lq_error(&mut self, character_id: CharacterId, message: impl AsRef<str>) {
        let mut bytes = crate::text::COL_LIGHT_RED.to_vec();
        bytes.extend_from_slice(message.as_ref().as_bytes());
        self.queue_system_text_bytes(character_id, bytes);
    }

    /// The nick-or-slot-ID target resolution idiom repeated in (almost)
    /// every `cmd_*` handler: `n = atoi(nick); if (n>0 && n<MAXLQNPC) {
    /// single slot, only if populated } else { scan for a nick[0]/nick[1]
    /// case-insensitive match, optionally also literal "all" }`.
    fn resolve_lq_npc_slots(&self, nick: &str, allow_all: bool) -> Vec<usize> {
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
    fn lq_admin_apply_to_targets(
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
    fn lq_admin_parse_item(
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

        false
    }

    /// C `cmd_npc` (`lq.c:357-425`).
    fn lq_admin_cmd_npc(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str =
            "/npc <base:str> <level:int> <mode:chr> <respawn:int> [nick1:str] [nick2:str]";
        let mut reader = ArgReader::new(args);
        let Some(basename) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing base. Usage is: {USAGE}."));
            return;
        };
        let Some(level) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing level. Usage is: {USAGE}."));
            return;
        };
        let Some(mode) = reader.take_chr() else {
            self.queue_lq_error(character_id, format!("Missing mode. Usage is: {USAGE}."));
            return;
        };
        let Some(respawn) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing respawn. Usage is: {USAGE}."));
            return;
        };
        let mut nick0 = reader.take_str().unwrap_or_default();
        let mut nick1 = reader.take_str().unwrap_or_default();
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        nick0.truncate(39);
        nick1.truncate(39);

        let Some(caller) = self.characters.get(&character_id) else {
            return;
        };
        let (x, y, dir) = (caller.x, caller.y, caller.dir);

        if let Some(existing) = self.lq_npcs.iter().find(|npc| npc.x == x && npc.y == y) {
            let message = format!(
                " {} {} {} is already at this position",
                existing.slot, existing.nick[0], existing.nick[1]
            );
            self.queue_lq_error(character_id, message);
            return;
        }

        let Some(slot) = self.find_free_lq_npc_slot() else {
            self.queue_system_text(character_id, "No free NPC slots left.");
            return;
        };

        let mut basename = basename;
        basename.truncate(39);
        self.lq_npcs.push(LqNpcState {
            slot,
            basename,
            x,
            y,
            dir,
            level: level.clamp(0, i64::from(u16::MAX)) as u16,
            mode: (mode.to_ascii_lowercase() as u32) as u8,
            respawn_seconds: respawn.clamp(0, i64::from(u32::MAX)) as u32,
            name: String::new(),
            description: String::new(),
            nick: [nick0, nick1],
            character_id: None,
            character_serial: 0,
            sprite: 0,
            greeting: String::new(),
            trigger: [
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
            reply: [
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
            want_key_id: 0,
            reward_item: LqItemSpec::default(),
            reward_mark_id: 0,
            kill_mark_id: 0,
            hurt_mark_id: 0,
            carry_item: LqItemSpec::default(),
            carry_gold: 0,
        });
        self.lq_npcs.sort_by_key(|npc| npc.slot);

        self.queue_system_text(character_id, format!("Added NPC {slot}"));
    }

    /// C `cmd_npcname` (`lq.c:512-551`).
    fn lq_admin_cmd_npcname(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcname <npcID|nick> <name:str>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(name) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing name. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| {
            npc.name = name.clone();
            npc.name.truncate(39);
        });
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set name of {count} NPCs"));
        }
    }

    /// C `cmd_npcgold` (`lq.c:553-597`).
    fn lq_admin_cmd_npcgold(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcgold <npcID|nick> <gold:int>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(gold) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing gold. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        if gold > 2000 {
            self.queue_lq_error(character_id, "Too much gold.");
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| {
            npc.carry_gold = gold.max(0) as u32;
        });
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set gold of {count} NPCs"));
        }
    }

    /// C `cmd_npcsprite` (`lq.c:599-643`). The `usage`/"Missing gold"
    /// error strings are a verbatim copy-paste of `cmd_npcgold`'s in the
    /// C source (`lq.c:602,609`) - kept exactly, not "fixed".
    fn lq_admin_cmd_npcsprite(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcgold <npcID|nick> <sprite:int>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(sprite) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing gold. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        if sprite == 313 || sprite == 305 || sprite == 58 {
            self.queue_system_text(
                character_id,
                "Sorry, Islena is not available for Life Quests.",
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| {
            npc.sprite = sprite as i32;
        });
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set sprite of {count} NPCs"));
        }
    }

    /// C `cmd_npcdesc` (`lq.c:645-684`).
    fn lq_admin_cmd_npcdesc(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcdesc <npcID|nick> <description:str>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(desc) = reader.take_str() else {
            self.queue_lq_error(
                character_id,
                format!("Missing description. Usage is: {USAGE}."),
            );
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| {
            npc.description = desc.clone();
            npc.description.truncate(159);
        });
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set description of {count} NPCs"));
        }
    }

    /// C `cmd_npcgreet` (`lq.c:686-725`).
    fn lq_admin_cmd_npcgreet(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcgreet <npcID|nick> <text:str>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(text) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing text. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| {
            npc.greeting = text.clone();
            npc.greeting.truncate(255);
        });
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set greeting of {count} NPCs"));
        }
    }

    /// C `cmd_npckillmark` (`lq.c:727-771`).
    fn lq_admin_cmd_npckillmark(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npckillmark <npcID|nick> <mark:int>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(mark) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing mark. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        if mark < 1 || mark >= MAXLQMARK as i64 {
            self.queue_system_text(
                character_id,
                format!("Mark is out of bounds (1-{})", MAXLQMARK - 1),
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| {
            npc.kill_mark_id = (mark as i32) as u32;
        });
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set killmark of {count} NPCs"));
        }
    }

    /// C `cmd_npchurtmark` (`lq.c:773-817`).
    fn lq_admin_cmd_npchurtmark(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npchurtmark <npcID|nick> <mark:int>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(mark) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing mark. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        if mark < 1 || mark >= MAXLQMARK as i64 {
            self.queue_system_text(
                character_id,
                format!("Mark is out of bounds (1-{})", MAXLQMARK - 1),
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| {
            npc.hurt_mark_id = (mark as i32) as u32;
        });
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set hurtmark of {count} NPCs"));
        }
    }

    /// C `cmd_npcmodlevel` (`lq.c:819-878`).
    fn lq_admin_cmd_npcmodlevel(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcmodlevel <npcID|nick|all> <mod:int>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(modifier) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing mod. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, true);
        let mut clamp_messages = Vec::new();
        let mut count = 0usize;
        for slot in slots {
            let Some(npc) = self.lq_npcs.iter_mut().find(|npc| npc.slot == slot) else {
                continue;
            };
            count += 1;
            let mut new_level = i64::from(npc.level) + modifier;
            if new_level < 1 {
                new_level = 1;
                clamp_messages.push(format!(
                    "NPC {} ({} {} {}) set to level 1 to avoid negative level.",
                    slot, npc.name, npc.nick[0], npc.nick[1]
                ));
            }
            if new_level > 200 {
                new_level = 200;
                clamp_messages.push(format!(
                    "NPC {} ({} {} {}) set to level 200 to avoid too high levels.",
                    slot, npc.name, npc.nick[0], npc.nick[1]
                ));
            }
            npc.level = new_level as u16;
        }
        for message in clamp_messages {
            self.queue_system_text(character_id, message);
        }
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Changed level of {count} NPCs"));
        }
    }

    /// C `cmd_npcrespawn` (`lq.c:880-919`).
    fn lq_admin_cmd_npcrespawn(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcrespawn <npcID|nick|all> <mod:int>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(modifier) = reader.take_int() else {
            self.queue_lq_error(
                character_id,
                format!("Missing respawn time. Usage is: {USAGE}."),
            );
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, true);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| {
            npc.respawn_seconds = modifier.clamp(0, i64::from(u32::MAX)) as u32;
        });
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(
                character_id,
                format!("Changed respawn time of {count} NPCs to {modifier}"),
            );
        }
    }

    /// C `cmd_npcpos` (`lq.c:921-982`).
    fn lq_admin_cmd_npcpos(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcpos <npcID|nick> [x:int] [y:int]";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let mut x = reader.take_int().unwrap_or(0);
        let mut y = reader.take_int().unwrap_or(0);
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }

        let Some(caller) = self.characters.get(&character_id) else {
            return;
        };
        let (caller_x, caller_y, caller_dir) = (caller.x, caller.y, caller.dir);
        if x == 0 && y == 0 {
            x = i64::from(caller_x);
            y = i64::from(caller_y);
        }
        if x < 1
            || x >= i64::from(MAX_MAP as i32) - 1
            || y < 1
            || y >= i64::from(MAX_MAP as i32) - 1
        {
            self.queue_system_text(character_id, format!("Position {x},{y} is out of bounds."));
            return;
        }

        let numeric = legacy_atoi(&nick);
        let mut target_slot = if numeric >= 1
            && (numeric as usize) < MAX_LQ_NPCS
            && self.lq_npcs.iter().any(|npc| npc.slot == numeric as usize)
        {
            Some(numeric as usize)
        } else {
            None
        };
        if target_slot.is_none() {
            for npc in &self.lq_npcs {
                if npc.nick[0].eq_ignore_ascii_case(&nick)
                    || npc.nick[1].eq_ignore_ascii_case(&nick)
                {
                    if target_slot.is_some() {
                        self.queue_lq_error(
                            character_id,
                            "Cannot set the same position for multiple NPCs.",
                        );
                        return;
                    }
                    target_slot = Some(npc.slot);
                }
            }
        }
        let Some(target_slot) = target_slot else {
            self.queue_lq_error(character_id, "NPC not found.");
            return;
        };

        if let Some(conflict) = self
            .lq_npcs
            .iter()
            .find(|npc| npc.slot != target_slot && i64::from(npc.x) == x && i64::from(npc.y) == y)
        {
            let message = format!(
                " {} {} {} is already at this position",
                conflict.slot, conflict.nick[0], conflict.nick[1]
            );
            self.queue_lq_error(character_id, message);
            return;
        }

        if let Some(npc) = self.lq_npcs.iter_mut().find(|npc| npc.slot == target_slot) {
            npc.x = x as u16;
            npc.y = y as u16;
            npc.dir = caller_dir;
        }
        self.queue_system_text(character_id, format!("Set position to {x},{y}."));
    }

    /// C `cmd_npcreply` (`lq.c:984-1039`).
    fn lq_admin_cmd_npcreply(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcreply <npcID|nick> <nr:int> <trigger:str> <reply:str>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(nr) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing nr. Usage is: {USAGE}."));
            return;
        };
        let Some(trigger) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing trigger. Usage is: {USAGE}."));
            return;
        };
        let Some(reply) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing reply. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        let idx = nr - 1;
        if !(0..5).contains(&idx) {
            // C typo kept verbatim: "Nr %d it out of bounds." (`lq.c:1012`).
            self.queue_system_text(character_id, format!("Nr {nr} it out of bounds."));
            return;
        }
        let idx = idx as usize;
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| {
            let mut trigger = trigger.clone();
            trigger.truncate(39);
            let mut reply = reply.clone();
            reply.truncate(255);
            npc.trigger[idx] = trigger;
            npc.reply[idx] = reply;
        });
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set trigger/reply of {count} NPCs"));
        }
    }

    /// C `cmd_npcwantitem` (`lq.c:1041-1080`).
    fn lq_admin_cmd_npcwantitem(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcwantitem <npcID|nick> <ID:int>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(id) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing ID. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| {
            npc.want_key_id = (id as i32) as u32;
        });
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set wantitem of {count} NPCs"));
        }
    }

    /// C `cmd_npcitem` (`lq.c:1167-1202`).
    fn lq_admin_cmd_npcitem(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str =
            "/npcitem <npcID|nick> <base:str> [keyID:int] [name:str] [description:str]";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(item) = self.lq_admin_parse_item(character_id, &mut reader, USAGE) else {
            return;
        };
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| npc.carry_item = item.clone());
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set item of {count} NPCs"));
        }
    }

    /// C `cmd_npcrewarditem` (`lq.c:1204-1239`). C's own success message
    /// is a verbatim copy-paste of `cmd_npcitem`'s ("Set item of %d
    /// NPCs", not "Set reward item...") - kept exactly.
    fn lq_admin_cmd_npcrewarditem(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str =
            "/npcrewarditem <npcID|nick> <base:str> [keyID:int] [name:str] [description:str]";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        let Some(item) = self.lq_admin_parse_item(character_id, &mut reader, USAGE) else {
            return;
        };
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = self.lq_admin_apply_to_targets(&slots, |npc| npc.reward_item = item.clone());
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Set item of {count} NPCs"));
        }
    }

    /// C `show_npc` (`lq.c:1082-1128`).
    fn lq_admin_show_npc(&mut self, character_id: CharacterId, slot: usize) {
        let Some(npc) = self.lq_npcs.iter().find(|npc| npc.slot == slot) else {
            return;
        };
        let npc = npc.clone();
        self.queue_system_text(character_id, format!("Base: {}", npc.basename));
        self.queue_system_text(
            character_id,
            format!("Nicks: {}/{}", npc.nick[0], npc.nick[1]),
        );
        self.queue_system_text(character_id, format!("Level: {}", npc.level));
        self.queue_system_text(character_id, format!("Mode: {}", npc.mode as char));
        self.queue_system_text(character_id, format!("Respawn: {}", npc.respawn_seconds));
        if !npc.name.is_empty() {
            self.queue_system_text(character_id, format!("Name: {}", npc.name));
        }
        if !npc.description.is_empty() {
            self.queue_system_text(character_id, format!("Desc: {}", npc.description));
        }
        if !npc.greeting.is_empty() {
            self.queue_system_text(character_id, format!("Greeting: {}", npc.greeting));
        }
        for i in 0..5 {
            if !npc.trigger[i].is_empty() {
                self.queue_system_text(
                    character_id,
                    format!("Trigger/Reply {}: {}/{}", i, npc.trigger[i], npc.reply[i]),
                );
            }
        }
        if npc.carry_gold != 0 {
            self.queue_system_text(
                character_id,
                format!("Gold: {:.2}G", f64::from(npc.carry_gold) / 100.0),
            );
        }
        if !npc.carry_item.base.is_empty() {
            self.queue_system_text(
                character_id,
                format!(
                    "Carry Item: {} ID: {}",
                    npc.carry_item.base, npc.carry_item.key_id
                ),
            );
        }
        if npc.want_key_id != 0 {
            self.queue_system_text(character_id, format!("Wants ID: {}", npc.want_key_id));
        }
        if !npc.reward_item.base.is_empty() {
            self.queue_system_text(
                character_id,
                format!(
                    "Reward Item: {} ID: {}",
                    npc.reward_item.base, npc.reward_item.key_id
                ),
            );
        }
        if npc.hurt_mark_id != 0 {
            // C reads `lq_data.reward_desc[]`/`reward[]` here - see the
            // module doc comment for why this is genuinely always empty/0.
            self.queue_system_text(
                character_id,
                format!("Hurtmark ID:  ({}), 0 exp", npc.hurt_mark_id),
            );
        }
        if npc.kill_mark_id != 0 {
            self.queue_system_text(
                character_id,
                format!("Killmark ID:  ({}), 0 exp", npc.kill_mark_id),
            );
        }
    }

    /// C `cmd_npcshow` (`lq.c:1130-1165`).
    fn lq_admin_cmd_npcshow(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcshow <npcID|nick>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let count = slots.len();
        for slot in slots {
            self.lq_admin_show_npc(character_id, slot);
        }
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Showed {count} NPCs"));
        }
    }

    /// C `cmd_npclist` (`lq.c:1241-1274`).
    fn lq_admin_cmd_npclist(&mut self, character_id: CharacterId, args: &str) {
        let mut reader = ArgReader::new(args);
        let mut nick = reader.take_str().unwrap_or_default();
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                "Trailing garbage. Usage is: /npclist <nick|start>.",
            );
            return;
        }
        let start = legacy_atoi(&nick);
        if start != 0 {
            nick.clear();
        }
        let start_slot = start.max(1) as usize;

        let mut slots: Vec<usize> = self.lq_npcs.iter().map(|npc| npc.slot).collect();
        slots.sort_unstable();

        let mut lines = Vec::new();
        let mut count = 0usize;
        for slot in slots {
            if slot < start_slot {
                continue;
            }
            let Some(npc) = self.lq_npcs.iter().find(|npc| npc.slot == slot) else {
                continue;
            };
            if !nick.is_empty()
                && !npc.nick[0].eq_ignore_ascii_case(&nick)
                && !npc.nick[1].eq_ignore_ascii_case(&nick)
            {
                continue;
            }
            lines.push(format!(
                "NPC {:3}: base {}, level {}, nicks: {} {}, pos: {},{}",
                slot, npc.basename, npc.level, npc.nick[0], npc.nick[1], npc.x, npc.y
            ));
            count += 1;
            if count > 99 {
                break;
            }
        }
        for line in lines {
            self.queue_system_text(character_id, line);
        }
        self.queue_system_text(
            character_id,
            format!(
                "{} of {} NPCs ({}%)",
                count,
                MAX_LQ_NPCS - 1,
                100 * count / (MAX_LQ_NPCS - 1)
            ),
        );
    }

    /// C `remove_npc` (`lq.c:1839-1861`), called from `cmd_npcdel`. Any
    /// pending scheduled respawn is cleared unconditionally; a live
    /// instance is only destroyed if its serial still matches (the
    /// template wasn't already respawned into a different character).
    fn lq_admin_remove_npc_instance(&mut self, slot: usize) {
        self.lq_npc_respawns.retain(|(s, _)| *s != slot);
        let Some(npc) = self.lq_npcs.iter().find(|npc| npc.slot == slot) else {
            return;
        };
        let Some(character_id) = npc.character_id else {
            return;
        };
        let expected_serial = npc.character_serial;
        let live = self
            .characters
            .get(&character_id)
            .is_some_and(|character| character.serial == expected_serial);
        if live {
            self.remove_character(character_id);
        }
    }

    /// C `cmd_npcdel` (`lq.c:1276-1312`).
    fn lq_admin_cmd_npcdel(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/npcdel <npcID|nick>";
        let mut reader = ArgReader::new(args);
        let Some(nick) = reader.take_str() else {
            self.queue_lq_error(character_id, format!("Missing npcID. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        let slots = self.resolve_lq_npc_slots(&nick, false);
        let mut count = 0usize;
        for slot in slots {
            self.lq_admin_remove_npc_instance(slot);
            self.lq_npcs.retain(|npc| npc.slot != slot);
            count += 1;
        }
        if count == 0 {
            self.queue_lq_error(character_id, "NPC not found.");
        } else {
            self.queue_system_text(character_id, format!("Deleted {count} NPCs."));
        }
    }
}
