//! `CDR_LQPARSER`'s "usurp" god/LQMaster NPC-possession mechanic
//! (`lq.c:2139-2276`, `2504-2742`, `2855-2868`): `#usurp`/`#follow`/
//! `#stop`/`#exit` (`CF_GOD`/`CF_LQMASTER`-gated), `#wimp` (any player, no
//! extra gate beyond the area-20/35 dispatch itself), the possessed-NPC
//! "me"/"emote" relay sub-command, the possessed-NPC plain-speech relay,
//! and the per-tick `domirror` movement-mirroring branch this all drives
//! (ported in `world::npc::area20::lqnpc::process_lqnpc_tick`). Split into
//! its own file/module from `world::lq_admin` (which was already near the
//! ~2,000-line hard cap) - see that module's doc comment for the rest of
//! the `CDR_LQPARSER` admin command table.
//!
//! Two related things are deliberately NOT ported here:
//! - The `c9`/`mirror` possessed-NPC relay sub-command (`lq.c:2710-2723`)
//!   needs `src/system/chat/chat.c`'s `server_chat`, which `AGENTS.md`'s
//!   "Not Applicable / Deferred" list permanently defers (cross-server
//!   chat transport; single-server setup for now).
//! - The LQ-area (20/35) "no real death" special case in `hurt_char`
//!   (`death.c:1238-1249`, also writing `ppd->last_lq_death`) is a
//!   completely different C function (damage application, not this
//!   admin command table) and an unrelated gap in the already-`[x]`
//!   P0 "Player death saves" task, not this one.
//!
//! The nearest-match name search in `cmd_usurp`/`cmd_follow`/`cmd_stop`
//! (`lq.c:2175-2189` etc.) is, in C, a sector-quantized double loop
//! (`getfirst_char_sector`, 8-tile strides covering every sector
//! overlapping a `[cn.x-12, cn.x+12] x [cn.y-12, cn.y+12]` box) - a pure
//! spatial-indexing optimization with no `World` equivalent (see the
//! deferred "Sector skip optimization" P3 task). This port uses a direct
//! linear scan over every live `CDR_LQNPC` character within the same
//! 12-tile Chebyshev box, which is observably identical for the search's
//! actual selection criteria (nearest `char_dist` match against a
//! case-insensitive substring of the *live character's* own `.name`,
//! matching C's `strcasestr(ch[co].name, name)` - not the `LqNpcState`
//! template's `nick[]`, which `#npcname`/`#npclist` use instead) modulo
//! C's own sector-boundary rounding artifacts at the very edge of the
//! box - not a behavior any admin macro or test can reasonably depend on.

use crate::character_driver::CDR_LQNPC;
use crate::drvlib::char_dist;

use super::lq_admin::{cmd_word_matches, ArgReader};
use super::*;

impl World {
    /// Top-level entry point, C `special_driver`'s slice covering both of
    /// its `#`/`/`-prefixed branch (`lq.c:2514-2727`, this module's
    /// commands only - `#npc`/`#thrall`/etc. are `world::lq_admin`'s own
    /// entry point, checked separately) and its plain-speech branch
    /// (`lq.c:2729-2739`). Returns `true` (C's `return 2`) once handled;
    /// `false` (C's `return 1`) for anything unmatched, area-gate-failed,
    /// or not gated by an active usurp - the caller should fall through
    /// to normal command/speech processing exactly as C does. Must be
    /// dispatched *before* any chat/tell/who/etc. command handling (C's
    /// `special_driver` runs before `command()`'s own giant switch,
    /// `system/command.c:5855-5859`) so the possessed-NPC plain-speech
    /// relay can actually intercept ordinary `say` text.
    pub fn apply_lq_usurp_command(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
        command: &str,
    ) -> bool {
        if area_id != 20 && area_id != 35 {
            return false;
        }

        let trimmed = command.trim_start();
        if let Some(rest) = trimmed
            .strip_prefix('#')
            .or_else(|| trimmed.strip_prefix('/'))
        {
            return self.apply_lq_usurp_slash_command(character_id, rest);
        }

        // C `lq.c:2729-2739`: plain (non-slash) speech, relayed as the
        // possessed NPC's own `say` instead of the player's own, only
        // while `CF_GOD`/`CF_LQMASTER` and actively usurping.
        let Some(flags) = self.characters.get(&character_id).map(|c| c.flags) else {
            return false;
        };
        if !flags.intersects(CharacterFlags::GOD | CharacterFlags::LQMASTER) {
            return false;
        }
        let Some(npc_id) = self.lq_active_usurp_target(character_id) else {
            return false;
        };
        self.npc_say(npc_id, command);
        true
    }

    fn apply_lq_usurp_slash_command(&mut self, character_id: CharacterId, rest: &str) -> bool {
        let mut reader = ArgReader::new(rest);
        let Some(word) = reader.take_str() else {
            return false;
        };
        let args = reader.remaining();

        // C `lq.c:2517-2520`: `#wimp` needs no `CF_GOD`/`CF_LQMASTER` gate
        // - any player standing in the Live Quest area can use it.
        if cmd_word_matches(&word, "wimp", 4) {
            self.lq_cmd_wimp(character_id);
            return true;
        }

        let Some(flags) = self.characters.get(&character_id).map(|c| c.flags) else {
            return false;
        };
        if !flags.intersects(CharacterFlags::GOD | CharacterFlags::LQMASTER) {
            return false;
        }

        if cmd_word_matches(&word, "usurp", 3) {
            self.lq_cmd_usurp(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "follow", 3) {
            self.lq_cmd_follow(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "stop", 3) {
            self.lq_cmd_stop(character_id, args);
            return true;
        }
        if cmd_word_matches(&word, "exit", 3) {
            self.lq_cmd_exit(character_id, true);
            return true;
        }

        // C `lq.c:2704-2724`: the possessed-NPC relay sub-commands, only
        // reachable while actively usurping a live NPC that still names
        // this player back (mutual pairing).
        if let Some(npc_id) = self.lq_active_usurp_target(character_id) {
            if cmd_word_matches(&word, "me", 2) || cmd_word_matches(&word, "emote", 2) {
                // C `emote(co, "%s", ptr + len);` (`lq.c:2707`) - `ptr +
                // len` is *not* whitespace-trimmed (unlike the `c9`/
                // `mirror` branch), so `args` (which still carries any
                // leading space right after the matched word, same as
                // C's raw pointer) is passed through unmodified.
                self.npc_emote(npc_id, args);
                return true;
            }
            // `c9`/`mirror` (`lq.c:2710-2723`) - deliberately not ported,
            // see this module's doc comment.
        }

        false
    }

    /// C's mutual-pairing check, reused from the relay dispatch above and
    /// [`World::lq_usurp_possessor_of`]'s tick-time sibling:
    /// `Character::lq_usurp` names a live `CDR_LQNPC` character whose own
    /// `LqNpcDriverData::usurp` still names this player back
    /// (`lq.c:2704-2705,2734-2735`).
    fn lq_active_usurp_target(&self, character_id: CharacterId) -> Option<CharacterId> {
        let npc_id = self.characters.get(&character_id)?.lq_usurp?;
        match self
            .characters
            .get(&npc_id)
            .and_then(|character| character.driver_state.as_ref())
        {
            Some(CharacterDriverState::LqNpc(data)) if data.usurp == Some(character_id) => {
                Some(npc_id)
            }
            _ => None,
        }
    }

    /// C `cmd_exit`'s core (`lq.c:2139-2155`), shared by the standalone
    /// `#exit` command and `cmd_usurp`'s internal pre-clear (called with
    /// `ptr == NULL`, suppressing the "Done." message C would otherwise
    /// print).
    fn lq_clear_usurp(&mut self, character_id: CharacterId) -> Option<CharacterId> {
        let npc_id = self
            .characters
            .get_mut(&character_id)
            .and_then(|character| character.lq_usurp.take())?;
        if let Some(CharacterDriverState::LqNpc(data)) = self
            .characters
            .get_mut(&npc_id)
            .and_then(|character| character.driver_state.as_mut())
        {
            data.usurp = None;
        }
        Some(npc_id)
    }

    /// C `cmd_exit` (`lq.c:2139-2155`): `show_message` mirrors C's `if
    /// (ptr) log_sys(cn, "Done.")` - `true` when reached via the `#exit`
    /// dispatch (C's `ptr` is always non-`NULL` there, even if empty),
    /// `false` when called internally by `cmd_usurp` (C passes `NULL`).
    fn lq_cmd_exit(&mut self, character_id: CharacterId, show_message: bool) {
        self.lq_clear_usurp(character_id);
        if show_message {
            self.queue_system_text(character_id, "Done.");
        }
    }

    /// The bounding-box + name-substring candidate search shared by
    /// `cmd_usurp`/`cmd_follow`/`cmd_stop` - see this module's own doc
    /// comment for the sector-quantization simplification.
    fn lq_npc_name_candidates(&self, character_id: CharacterId, name: &str) -> Vec<CharacterId> {
        let Some(caller) = self.characters.get(&character_id) else {
            return Vec::new();
        };
        let (cx, cy) = (i32::from(caller.x), i32::from(caller.y));
        let lower = name.to_ascii_lowercase();
        self.characters
            .values()
            .filter(|character| {
                character.driver == CDR_LQNPC
                    && (i32::from(character.x) - cx).abs() <= 12
                    && (i32::from(character.y) - cy).abs() <= 12
                    && character.name.to_ascii_lowercase().contains(&lower)
            })
            .map(|character| character.id)
            .collect()
    }

    /// C `cmd_usurp` (`lq.c:2157-2204`).
    fn lq_cmd_usurp(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/usurp <name:str>";
        let mut reader = ArgReader::new(args);
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

        let Some(caller) = self.characters.get(&character_id).cloned() else {
            return;
        };
        let target_id = self
            .lq_npc_name_candidates(character_id, &name)
            .into_iter()
            .filter_map(|id| {
                self.characters
                    .get(&id)
                    .map(|target| (id, char_dist(&caller, target)))
            })
            .min_by_key(|(_, dist)| *dist)
            .map(|(id, _)| id);

        let Some(target_id) = target_id else {
            self.queue_system_text(character_id, "NPC not found.");
            return;
        };

        // C `cmd_exit(cn, NULL);` (`lq.c:2191`) - clear any pre-existing
        // usurp before establishing the new one.
        self.lq_clear_usurp(character_id);

        let Some((caller_x, caller_y)) = self.characters.get(&character_id).map(|c| (c.x, c.y))
        else {
            return;
        };
        let Some((target_x, target_y)) = self.characters.get(&target_id).map(|c| (c.x, c.y)) else {
            return;
        };
        if let Some(caller) = self.characters.get_mut(&character_id) {
            caller.lq_usurp = Some(target_id);
        }
        if let Some(CharacterDriverState::LqNpc(data)) = self
            .characters
            .get_mut(&target_id)
            .and_then(|character| character.driver_state.as_mut())
        {
            data.usurp = Some(character_id);
            data.udx = i32::from(caller_x) - i32::from(target_x);
            data.udy = i32::from(caller_y) - i32::from(target_y);
        }
        self.queue_system_text(character_id, "Done.");
    }

    /// C `cmd_follow` (`lq.c:2206-2240`).
    fn lq_cmd_follow(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/follow <name:str>";
        let mut reader = ArgReader::new(args);
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
        let targets = self.lq_npc_name_candidates(character_id, &name);
        let mut count = 0usize;
        for target_id in targets {
            if let Some(CharacterDriverState::LqNpc(data)) = self
                .characters
                .get_mut(&target_id)
                .and_then(|character| character.driver_state.as_mut())
            {
                data.follow = Some(character_id);
                count += 1;
            }
        }
        self.queue_system_text(character_id, format!("Set {count} NPCs to follow."));
    }

    /// C `cmd_stop` (`lq.c:2242-2276`).
    fn lq_cmd_stop(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/stop <name:str>";
        let mut reader = ArgReader::new(args);
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
        let targets = self.lq_npc_name_candidates(character_id, &name);
        let mut count = 0usize;
        for target_id in targets {
            if let Some(CharacterDriverState::LqNpc(data)) = self
                .characters
                .get_mut(&target_id)
                .and_then(|character| character.driver_state.as_mut())
            {
                data.follow = None;
                count += 1;
            }
        }
        self.queue_system_text(character_id, format!("Set {count} NPCs to stop."));
    }

    /// C `cmd_wimp` (`lq.c:2323-2334`): bounce the caller to the nearest
    /// free tile among a fixed 7-position candidate list, log the same
    /// message C does, and queue the `PlayerRuntime`-needing
    /// `ppd->last_lq_death = realtime` write (see
    /// [`Self::drain_pending_lq_wimps`]).
    fn lq_cmd_wimp(&mut self, character_id: CharacterId) {
        if !self.teleport_char_driver(character_id, 240, 240)
            && !self.teleport_char_driver(character_id, 235, 240)
            && !self.teleport_char_driver(character_id, 240, 235)
            && !self.teleport_char_driver(character_id, 235, 235)
            && !self.teleport_char_driver(character_id, 245, 240)
            && !self.teleport_char_driver(character_id, 240, 245)
        {
            self.teleport_char_driver(character_id, 245, 245);
        }
        self.queue_system_text(character_id, "You wimped out.");
        self.pending_lq_wimps.push(character_id);
    }

    /// Drains the `PlayerRuntime`-needing `last_lq_death` writes queued by
    /// `#wimp` - the caller (`ugaris-server`) should call
    /// `PlayerRuntime::set_last_lq_death(current_realtime_seconds())` for
    /// each returned character id.
    pub fn drain_pending_lq_wimps(&mut self) -> Vec<CharacterId> {
        std::mem::take(&mut self.pending_lq_wimps)
    }
}
