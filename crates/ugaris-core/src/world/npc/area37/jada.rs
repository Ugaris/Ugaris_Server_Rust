//! Jada NPC (`CDR_JADA`), the Arkhata mystic who runs "The Source" (quest
//! 72).
//!
//! Ports `src/area/37/arkhata.c::jada_driver` (`:2835-3001`) plus the
//! shared `analyse_text_driver`/`qa[]` table (`:115-169`, ported as
//! [`super::ARKHATA_QA`] in `world::npc::area37`). Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area37::
//! ramin`/`jaz`: the caller supplies a per-player fact snapshot
//! ([`JadaPlayerFacts`]) up front and applies the returned
//! [`JadaOutcomeEvent`]s afterwards, since `arkhata_ppd.jada_state` and
//! sibling fields live on `crate::player::PlayerRuntime`, not `World`.
//!
//! `jada_driver`'s six-state (`0`-`5`) dialogue chain is the simplest one
//! in this file:
//! - `0` needs `arkhata_ppd.ramin_state >= 12` (`world::npc::area37::
//!   ramin`'s own progress) to advance; C's own `case 0` falls through
//!   into `case 1`'s speech/`questlog_open(72)`/advance-to-`2` in the
//!   same tick - collapsed into one `rs == 0` arm here, same "fallthrough
//!   lands on the next case's action" precedent as `world::npc::area37::
//!   ramin`'s own `rs == 0`/`9`/`11` arms. State `1` itself is never
//!   independently stored (the collapse always lands on `2`), so no
//!   separate arm exists for it.
//! - `4` is a pure wait state: waiting for the player to bring the evil
//!   blade (`IID_ARKHATA_BLADE`), handled entirely by this file's own
//!   `NT_GIVE` branch - no cross-driver dependency.
//! - `5` is a pure wait state: quest already completed, nothing left to
//!   say.
//!
//! Deviations/gaps (documented, not silent):
//! - Like every other `arkhata.c` NPC driver's own `NT_TEXT` branch, this
//!   driver has no `dat->current_victim` staleness-reset preamble and no
//!   victim-mismatch early-out - reproduced verbatim (matches
//!   `world::npc::area37::rammy`/`jaz`/`ramin`'s identical observation
//!   for that file's shared driver shape).
//! - `NT_GIVE`'s blade turn-in (`:2965-2974`) is the only `arkhata_ppd`
//!   write this driver itself performs directly. Unlike `rammy`/`jaz`'s
//!   silent quest-item success, this one speaks on success (`say`,
//!   ported as `npc_quiet_say` for consistency with every other
//!   dialogue line in this file - see `world::npc::area37::ramin`'s own
//!   module doc comment for the same "`say()` in C, `npc_quiet_say`
//!   here" precedent), matching `ramin`'s own letter-turn-in shape. The
//!   fallback branch (wrong item, or state out of the `1..=4` turn-in
//!   window) uses `say` (ported as `npc_say`, the "give the item back"
//!   fallback precedent every other driver in this file shares).
//! - No self-defense/regen/spell-self cascade exists in C's `jada_driver`
//!   body at all (matching the `rammy`/`jaz`/`ramin` "pure talker" NPC
//!   precedent) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:3001`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_LOSTCON};
use crate::drvlib::offset2dx;
use crate::item_driver::IID_ARKHATA_BLADE;
use crate::world::*;

use super::ARKHATA_QA;

/// C `char_dist(cn, co) > 10` (`arkhata.c:2884`, sibling drivers' own
/// identical guard).
const JADA_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`arkhata.c:197`, the shared
/// `analyse_text_driver` copy's own guard).
const JADA_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`arkhata.c:2867`).
const JADA_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`arkhata.c:2872`).
const JADA_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`arkhata.c:2995`): idle "return to post" threshold.
const JADA_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C quest 72, "The Source".
const QLOG_JADA_SOURCE: usize = 72;

/// Per-player facts [`World::process_jada_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JadaPlayerFacts {
    /// `PlayerRuntime::arkhata_jada_state()`.
    pub jada_state: i32,
    /// `PlayerRuntime::arkhata_ramin_state()` (`ppd->ramin_state`,
    /// `arkhata.c:2894`): gates `rs` `0`.
    pub ramin_state: i32,
}

/// A side effect [`World::process_jada_actions`] could not apply directly
/// because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JadaOutcomeEvent {
    /// Write the new `arkhata_ppd.jada_state` back.
    UpdateJadaState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 72)` (`arkhata.c:2902`).
    QuestOpen72 { player_id: CharacterId },
    /// C `questlog_done(co, 72)` (`arkhata.c:2967`), the `NT_GIVE` blade
    /// turn-in.
    QuestDone72 { player_id: CharacterId },
}

impl World {
    /// C `jada_driver`'s per-tick body (`arkhata.c:2835-3001`).
    pub fn process_jada_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, JadaPlayerFacts>,
        area_id: u16,
    ) -> Vec<JadaOutcomeEvent> {
        let jada_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_JADA
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for jada_id in jada_ids {
            self.process_jada_messages(jada_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_jada_messages(
        &mut self,
        jada_id: CharacterId,
        player_facts: &HashMap<CharacterId, JadaPlayerFacts>,
        area_id: u16,
        events: &mut Vec<JadaOutcomeEvent>,
    ) {
        let Some(jada_name) = self.characters.get(&jada_id).map(|jada| jada.name.clone()) else {
            return;
        };
        let Some(CharacterDriverState::Jada(mut data)) = self
            .characters
            .get(&jada_id)
            .and_then(|jada| jada.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&jada_id)
            .map(|jada| std::mem::take(&mut jada.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.jada_handle_char_message(
                    jada_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.jada_handle_text_message(
                    jada_id,
                    &jada_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.jada_handle_give_message(jada_id, message, player_facts, events),
                _ => {}
            }
        }

        if let Some(jada) = self.characters.get_mut(&jada_id) {
            jada.driver_state = Some(CharacterDriverState::Jada(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`arkhata.c:2991-2993`).
        if let (Some(jada), Some((tx, ty))) = (self.characters.get(&jada_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(jada.x), i32::from(jada.y), tx, ty) {
                if let Some(jada_mut) = self.characters.get_mut(&jada_id) {
                    let _ = turn(jada_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; }` (`arkhata.c:2995-2999`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other stationary NPC in this codebase makes.
        let last_talk = if let Some(jada) = self.characters.get(&jada_id) {
            match jada.driver_state.as_ref() {
                Some(CharacterDriverState::Jada(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + JADA_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(jada) = self.characters.get(&jada_id) else {
                return;
            };
            let (post_x, post_y) = (jada.rest_x, jada.rest_y);
            self.secure_move_driver(
                jada_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `jada_driver`'s `NT_CHAR` branch (`arkhata.c:2851-2930`).
    #[allow(clippy::too_many_arguments)]
    fn jada_handle_char_message(
        &mut self,
        jada_id: CharacterId,
        data: &mut JadaDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, JadaPlayerFacts>,
        events: &mut Vec<JadaOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(jada) = self.characters.get(&jada_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER))` (`arkhata.c:2856`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON)` (`arkhata.c:2862`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5)` (`arkhata.c:2867`).
        if tick < data.last_talk + JADA_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co)` (`arkhata.c:2872`).
        if tick < data.last_talk + JADA_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co)` (`arkhata.c:2878`).
        if jada_id == player_id || !char_see_char(&jada, &player, &self.map, self.date.daylight) {
            return;
        }
        // C `if (char_dist(cn, co) > 10)` (`arkhata.c:2884`).
        if char_dist(&jada, &player) > JADA_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.jada_state;
        match facts.jada_state {
            // C `case 0:` falling through into `case 1:` (`arkhata.c:
            // 2893-2903`) - see the module doc comment.
            0 if facts.ramin_state >= 12 => {
                self.npc_quiet_say(
                    jada_id,
                    &format!(
                        "Hello there, {}. Thou hast been sent from Ramin I see.",
                        player.name
                    ),
                );
                events.push(JadaOutcomeEvent::QuestOpen72 { player_id });
                new_state = 2;
                didsay = true;
            }
            0 => {}
            // C `case 2:` (`arkhata.c:2906-2910`).
            2 => {
                self.npc_quiet_say(
                    jada_id,
                    "I have discovered that the source of this evil that seems to penetrate our fortress is placed somewhere below it, in the cave system.",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`arkhata.c:2912-2916`).
            3 => {
                self.npc_quiet_say(
                    jada_id,
                    "The hole in the corner there is our safe entrance to the cave system, we have only been able to search a small part of the caves. I ask you to go down there and find the source of this evil and bring it to me.",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4: break;` (`arkhata.c:2919-2920`): waiting for the
            // blade.
            4 => {}
            // C `case 5: break;` (`arkhata.c:2921-2922`): all done.
            5 => {}
            _ => {}
        }

        if new_state != facts.jada_state {
            events.push(JadaOutcomeEvent::UpdateJadaState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`arkhata.c:2924-2928`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `jada_driver`'s `NT_TEXT` branch (`arkhata.c:2933-2956`), wired
    /// through the generic `analyse_text_qa` matcher.
    #[allow(clippy::too_many_arguments)]
    fn jada_handle_text_message(
        &mut self,
        jada_id: CharacterId,
        jada_name: &str,
        data: &mut JadaDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, JadaPlayerFacts>,
        events: &mut Vec<JadaOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // { dat->current_victim = 0; }` (`arkhata.c:2936-2938`).
        let tick = self.tick.0;
        if tick > data.last_talk + JADA_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co)`
        // (`arkhata.c:2940`).
        if data.current_victim.is_some() && data.current_victim != Some(speaker_id) {
            return;
        }

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let Some(jada) = self.characters.get(&jada_id).cloned() else {
            return;
        };
        // C `analyse_text_driver`'s own guard clauses (`arkhata.c:189-
        // 203`): ignore our own talk, distance > 12, not-visible.
        if jada_id == speaker_id {
            return;
        }
        if char_dist(&jada, &speaker) > JADA_QA_DISTANCE
            || !char_see_char(&jada, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let jada_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.jada_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, jada_name, &speaker.name, ARKHATA_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(jada_id, &reply);
                didsay = true;
            }
            // "repeat"/"restart" (`arkhata.c:2945-2952`): rewind to state
            // 1 while the turn-in window (`1..=4`) is open.
            TextAnalysisOutcome::Matched(2) => {
                if jada_state > 0 && jada_state <= 4 {
                    data.last_talk = 0;
                    events.push(JadaOutcomeEvent::UpdateJadaState {
                        player_id: speaker_id,
                        new_state: 1,
                    });
                }
                didsay = true;
            }
            // Every other matched code (the 40 `"raise <skill>"` codes,
            // `enter`(5)/`aye`(6)/`watch`(7)) is unhandled by jada's own
            // `switch` but still counts as `didsay` (C: `switch (didsay =
            // analyse_text_driver(...))` - any nonzero return is truthy).
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`arkhata.c:2954-2956`) - note this does *not* touch `dat->
        // last_talk` (except the "repeat" branch's own explicit reset
        // above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `jada_driver`'s `NT_GIVE` branch (`arkhata.c:2961-2979`).
    fn jada_handle_give_message(
        &mut self,
        jada_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, JadaPlayerFacts>,
        events: &mut Vec<JadaOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&jada_id)
            .and_then(|jada| jada.cursor_item.take())
        else {
            return;
        };
        let Some(item) = self.items.get(&item_id).cloned() else {
            return;
        };
        let Some(giver) = self.characters.get(&giver_id).cloned() else {
            return;
        };
        let is_player = giver.flags.contains(CharacterFlags::PLAYER);
        let facts = player_facts.get(&giver_id).copied();

        // C `if (ppd && it[in].ID == IID_ARKHATA_BLADE && ppd->jada_state
        // > 0 && ppd->jada_state <= 4)` (`arkhata.c:2965`).
        if item.template_id == IID_ARKHATA_BLADE
            && is_player
            && facts.is_some_and(|facts| facts.jada_state > 0 && facts.jada_state <= 4)
        {
            self.destroy_items_by_template_id(giver_id, IID_ARKHATA_BLADE);
            events.push(JadaOutcomeEvent::QuestDone72 {
                player_id: giver_id,
            });
            events.push(JadaOutcomeEvent::UpdateJadaState {
                player_id: giver_id,
                new_state: 5,
            });
            self.npc_quiet_say(
                jada_id,
                "By the bless-swirls, this thing is a concentration of evil! I will have to ask the monks for help to contain it. Thank thee, thou hast most certainly saved us all!",
            );
            self.destroy_item(item_id);
            return;
        }

        // C's fallback `else` branch (`arkhata.c:2977-2978`): hand the
        // item back to the giver.
        self.npc_say(
            jada_id,
            "Thou hast better use for this than I do. Well, if there is a use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_JADA;

/// C `struct std_npc_driver_data` (`arkhata.c:281-285`, shared by every
/// dialogue NPC in this file). `misc` is declared in C but never read or
/// written by `jada_driver` itself - no field for it here, same "only
/// port fields the driver actually uses" precedent as `world::npc::
/// area37::rammy`'s `RammyDriverData` doc comment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JadaDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}

/// Exposes [`QLOG_JADA_SOURCE`] to `ugaris-server`'s `apply_jada_events`.
pub const fn qlog_jada_source() -> usize {
    QLOG_JADA_SOURCE
}
