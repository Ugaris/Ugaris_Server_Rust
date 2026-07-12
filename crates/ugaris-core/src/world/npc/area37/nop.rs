//! Arkhata "nop" stationary NPC (`CDR_NOP`), the Fighting School
//! background "Student" template (`zones/37/Fighting_School.chr`).
//!
//! Ports `src/area/37/arkhata.c::nop_driver`/`nop_driver_parse`
//! (`:1286-1336`). Despite the name (a leftover copy-paste from the
//! shared `struct std_npc_driver_data`, whose `current_victim` field this
//! driver repurposes to store a fixed facing direction instead), this is
//! one of the simplest drivers in the file: parse a `dir=` zone-file arg
//! once at spawn, answer small talk via the shared [`super::ARKHATA_QA`]
//! table (discarding `analyse_text_driver`'s return value entirely -
//! `arkhata.c:1319` never assigns it, unlike every other driver in this
//! file that switches on it), then walk back to its post facing that
//! direction, try a self-buff, and idle. No `arg=` argument other than
//! `dir` exists in C (`nop_driver_parse`'s `else { elog(...); }` branch is
//! log-only), and C never calls `fight_driver_update`/`regenerate_driver`
//! here at all - this driver has no combat/regen behavior of its own.
//! `ret`/`lastact` are passed as `0` to `secure_move_driver`, the same
//! simplification already accepted for this class of driver (see
//! `world::npc::area22::gate_welcome`'s module doc comment).

use crate::character_driver::{
    analyse_text_qa, next_legacy_name_value, TextAnalysisOutcome, CDR_NOP,
};
use crate::world::*;

use super::ARKHATA_QA;

/// C `struct std_npc_driver_data::current_victim`, repurposed by
/// `nop_driver`/`nop_driver_parse` as a fixed facing/return-to-post
/// direction (`arkhata.c:281-296`) rather than an actual victim
/// reference - see the module doc comment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NopDriverData {
    pub facing_direction: u8,
}

/// C `nop_driver_parse` (`arkhata.c:1286-1296`): only the `dir` key is
/// recognized (`else { elog(...) }` for anything else, log-only).
pub fn parse_nop_driver_args(args: &str) -> NopDriverData {
    let mut data = NopDriverData::default();
    let mut rest = args;
    while let Some((name, value, next)) = next_legacy_name_value(rest) {
        if name == "dir" {
            data.facing_direction = value.parse::<i32>().unwrap_or(0) as u8;
        }
        rest = next;
    }
    data
}

impl World {
    /// C `nop_driver`'s per-tick body (`arkhata.c:1298-1336`).
    pub fn process_nop_actions(&mut self, area_id: u16) {
        let nop_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_NOP
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        for nop_id in nop_ids {
            self.process_nop_tick(nop_id, area_id);
        }
    }

    fn process_nop_tick(&mut self, nop_id: CharacterId, area_id: u16) {
        let Some(CharacterDriverState::Nop(data)) = self
            .characters
            .get(&nop_id)
            .and_then(|c| c.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&nop_id)
            .map(|c| std::mem::take(&mut c.driver_messages))
            .unwrap_or_default();

        // C `if (msg->type == NT_TEXT) { co = msg->dat3; analyse_text_
        // driver(cn, msg->dat1, (char *)msg->dat2, co); }` (`arkhata.c:
        // 1317-1320`) - the return value is discarded, so only qa rows
        // with a canned `answer` (which `analyse_text_driver` speaks
        // itself, C: `say(cn, qa[q].answer, ...)`) have any visible
        // effect here. No cooldown/current-victim gate exists for this
        // driver's `NT_TEXT` handling at all (unlike every other
        // `analyse_text_driver` caller in this codebase).
        for message in &messages {
            if message.message_type != NT_TEXT {
                continue;
            }
            let Some(text) = message.text.as_deref() else {
                continue;
            };
            let speaker_id = CharacterId(message.dat3.max(0) as u32);
            let Some(nop) = self.characters.get(&nop_id).cloned() else {
                return;
            };
            let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
                continue;
            };
            // C `analyse_text_driver`'s own guard clauses (`arkhata.c:
            // 183-203`): ignore our own talk, non-players, distance > 12,
            // not-visible (the log-type/`LOG_SYSTEM`/`LOG_INFO` guard
            // doesn't apply - Rust `push_driver_text_message` only ever
            // emits plain speech, same precedent as `world::npc::
            // area22::gate_welcome`).
            if nop_id == speaker_id || !speaker.flags.contains(CharacterFlags::PLAYER) {
                continue;
            }
            if char_dist(&nop, &speaker) > 12
                || !char_see_char(&nop, &speaker, &self.map, self.date.daylight)
            {
                continue;
            }
            if let TextAnalysisOutcome::Said(reply) =
                analyse_text_qa(text, &nop.name, &speaker.name, ARKHATA_QA)
            {
                self.npc_say(nop_id, &reply);
            }
        }

        // C `if (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, dat->
        // current_victim, ret, lastact)) return;` (`arkhata.c:1328-1330`)
        // - `tmpx`/`tmpy` reuse `rest_x`/`rest_y`, same substitution every
        // other stationary NPC in this codebase makes.
        let (post_x, post_y) = self
            .characters
            .get(&nop_id)
            .map(|nop| (nop.rest_x, nop.rest_y))
            .unwrap_or_default();
        if self.secure_move_driver(nop_id, post_x, post_y, data.facing_direction, 0, 0, area_id) {
            return;
        }
        // C `if (spell_self_driver(cn)) return;` (`arkhata.c:1331-1333`).
        self.spell_self_simple_baddy(nop_id);
        // C `do_idle(cn, TICKS*2);` (`arkhata.c:1335`) - not modeled, same
        // precedent as every other stationary dialogue-only NPC in this
        // codebase.
    }
}
