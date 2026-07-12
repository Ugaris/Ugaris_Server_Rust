//! Captain NPC (`CDR_CAPTAIN`), the Arkhata Fortress Captain who kicks off
//! the entrance-pass-system chain that continues through `judge_driver`
//! (`world::npc::area37::judge`).
//!
//! Ports `src/area/37/arkhata.c::captain_driver` (`:2087-2290`) plus the
//! shared `analyse_text_driver`/`qa[]` table (`:115-169`, ported as
//! [`super::ARKHATA_QA`] in `world::npc::area37`). Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area37::
//! ramin`/`rammy`: the caller supplies a per-player fact snapshot
//! ([`CaptainPlayerFacts`]) up front and applies the returned
//! [`CaptainOutcomeEvent`]s afterwards, since `arkhata_ppd.captain_state`
//! and sibling fields live on `crate::player::PlayerRuntime`, not `World`.
//!
//! `captain_driver`'s eleven-state (`0`-`10`) dialogue chain has one
//! cross-driver gate at `4`, read via [`CaptainPlayerFacts`]: it needs
//! `ch[co].level >= 53 && judge_state >= 6 && letter_bits == (2|4|8)`
//! (`judge_state` is `world::npc::area37::judge`'s own progress,
//! `letter_bits` accumulates as `judge_driver`/`ramin_driver` hand out
//! letters 2/3/4) to advance, then falls through into `case 5`'s speech/
//! advance-to-`6` in the same tick - collapsed into one `rs == 4` arm,
//! same "fallthrough lands on the next case's action" precedent as
//! `world::npc::area37::rammy`'s own `rs == 6`/`13`/`17` arms and
//! `world::npc::area37::ramin`'s own `rs == 0`/`9`/`11` arms.
//!
//! Deviations/gaps (documented, not silent):
//! - Like every other `arkhata.c` NPC driver's own `NT_TEXT` branch, this
//!   driver has no `dat->current_victim` staleness-reset preamble and no
//!   victim-mismatch early-out - reproduced verbatim (matches
//!   `world::npc::area37::rammy`/`jaz`/`ramin`'s identical observation for
//!   that file's shared driver shape).
//! - `NT_GIVE` has two special-case branches: the letter-1 turn-in
//!   (`captain_state == 0`, silently advances to `1`, no dialogue at all -
//!   reproduced verbatim, C truly has no `say`/`quiet_say` call on this
//!   path) and the letter-4 turn-in (`!(letter_bits & 8)`, `quiet_say`s
//!   and sets bit `8`, consumed by this driver's own `rs == 4` gate). The
//!   fallback branch (wrong item, or bit already set) uses `say`, matching
//!   `ramin`/`rammy`/`jaz`'s own fallback precedent exactly.
//! - No self-defense/regen/spell-self cascade exists in C's
//!   `captain_driver` body at all (matching the `rammy`/`jaz`/`ramin`
//!   "pure talker" NPC precedent) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:2289`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_LOSTCON};
use crate::drvlib::offset2dx;
use crate::item_driver::{IID_ARKHATA_LETTER1, IID_ARKHATA_LETTER4};
use crate::world::*;

use super::ARKHATA_QA;

/// C `char_dist(cn, co) > 10` (`arkhata.c:2136`, sibling drivers' own
/// identical guard).
const CAPTAIN_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`arkhata.c:197`, the shared
/// `analyse_text_driver` copy's own guard).
const CAPTAIN_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`arkhata.c:2119`).
const CAPTAIN_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`arkhata.c:2124`).
const CAPTAIN_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`arkhata.c:2283`): idle "return to post" threshold.
const CAPTAIN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `ppd->letter_bits |= 8` / `!(ppd->letter_bits & 8)` (`arkhata.c:
/// 2255-2261`).
const CAPTAIN_LETTER4_BIT: i32 = 8;
/// C `ppd->letter_bits == (2 | 4 | 8)` (`arkhata.c:2166`).
const ALL_LETTER_BITS: i32 = 2 | 4 | 8;

/// Per-player facts [`World::process_captain_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptainPlayerFacts {
    /// `PlayerRuntime::arkhata_captain_state()`.
    pub captain_state: i32,
    /// `PlayerRuntime::arkhata_judge_state()` (`ppd->judge_state`,
    /// `arkhata.c:2166`): gates `rs` `4`.
    pub judge_state: i32,
    /// `PlayerRuntime::arkhata_letter_bits()` (`ppd->letter_bits`,
    /// `arkhata.c:2166,2255`): gates `rs` `4` and the `NT_GIVE` letter-4
    /// turn-in.
    pub letter_bits: i32,
}

/// A side effect [`World::process_captain_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptainOutcomeEvent {
    /// Write the new `arkhata_ppd.captain_state` back.
    UpdateCaptainState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `ppd->letter_bits |= 8` (`arkhata.c:2261`), the `NT_GIVE`
    /// letter-4 turn-in.
    GiveLetter4Bit { player_id: CharacterId },
}

impl World {
    /// C `captain_driver`'s per-tick body (`arkhata.c:2087-2290`).
    pub fn process_captain_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, CaptainPlayerFacts>,
        area_id: u16,
    ) -> Vec<CaptainOutcomeEvent> {
        let captain_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_CAPTAIN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for captain_id in captain_ids {
            self.process_captain_messages(captain_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_captain_messages(
        &mut self,
        captain_id: CharacterId,
        player_facts: &HashMap<CharacterId, CaptainPlayerFacts>,
        area_id: u16,
        events: &mut Vec<CaptainOutcomeEvent>,
    ) {
        let Some(CharacterDriverState::Captain(mut data)) = self
            .characters
            .get(&captain_id)
            .and_then(|captain| captain.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&captain_id)
            .map(|captain| std::mem::take(&mut captain.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.captain_handle_char_message(
                    captain_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.captain_handle_text_message(
                    captain_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => {
                    self.captain_handle_give_message(captain_id, message, player_facts, events)
                }
                _ => {}
            }
        }

        if let Some(captain) = self.characters.get_mut(&captain_id) {
            captain.driver_state = Some(CharacterDriverState::Captain(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`arkhata.c:2279-2281`).
        if let (Some(captain), Some((tx, ty))) =
            (self.characters.get(&captain_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(captain.x), i32::from(captain.y), tx, ty) {
                if let Some(captain_mut) = self.characters.get_mut(&captain_id) {
                    let _ = turn(captain_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)) return; }` (`arkhata.c:2283-2287`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other stationary NPC in this codebase makes.
        let last_talk = if let Some(captain) = self.characters.get(&captain_id) {
            match captain.driver_state.as_ref() {
                Some(CharacterDriverState::Captain(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + CAPTAIN_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(captain) = self.characters.get(&captain_id) else {
                return;
            };
            let (post_x, post_y) = (captain.rest_x, captain.rest_y);
            self.secure_move_driver(
                captain_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `captain_driver`'s `NT_CHAR` branch (`arkhata.c:2103-2209`).
    #[allow(clippy::too_many_arguments)]
    fn captain_handle_char_message(
        &mut self,
        captain_id: CharacterId,
        data: &mut CaptainDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, CaptainPlayerFacts>,
        events: &mut Vec<CaptainOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(captain) = self.characters.get(&captain_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER))` (`arkhata.c:2107`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON)` (`arkhata.c:2113`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5)` (`arkhata.c:2119`).
        if tick < data.last_talk + CAPTAIN_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co)` (`arkhata.c:2124`).
        if tick < data.last_talk + CAPTAIN_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co)` (`arkhata.c:2130`).
        if captain_id == player_id
            || !char_see_char(&captain, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10)` (`arkhata.c:2136`).
        if char_dist(&captain, &player) > CAPTAIN_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.captain_state;
        match facts.captain_state {
            // C `case 0: break;` (`arkhata.c:2146-2147`).
            0 => {}
            // C `case 1:` (`arkhata.c:2148-2152`).
            1 => {
                self.npc_say(
                    captain_id,
                    "I see, so Rammy sent thee. Well if he trusts in thee then so shall I!",
                );
                new_state = 2;
                didsay = true;
            }
            // C `case 2:` (`arkhata.c:2153-2158`).
            2 => {
                self.npc_say(
                    captain_id,
                    "I believe we will need an entrance pass system to allow travellers to pass through the fortress safely. I can inform the guards of this.",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`arkhata.c:2159-2164`).
            3 => {
                self.npc_say(
                    captain_id,
                    "I need you to speak with the judge so that he can write the formal authorization letters for this. He will also tell you who needs a copy.",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4:` falling through into `case 5:` (`arkhata.c:
            // 2165-2176`) - see the module doc comment.
            4 if player.level >= 53
                && facts.judge_state >= 6
                && facts.letter_bits == ALL_LETTER_BITS =>
            {
                self.npc_say(
                    captain_id,
                    "This fortress holds many secrets, known only to me and a select few of my guards for safety reasons. But now there is a leak.",
                );
                new_state = 6;
                didsay = true;
            }
            4 => {}
            // C `case 6:` (`arkhata.c:2177-2182`).
            6 => {
                self.npc_say(
                    captain_id,
                    "A traitor was caught last night, on his way to the bandits carrying classified notes stolen from our archive.",
                );
                new_state = 7;
                didsay = true;
            }
            // C `case 7:` (`arkhata.c:2183-2187`).
            7 => {
                self.npc_say(
                    captain_id,
                    "He was caught in the act, but even under torture we have failed to make him speak.",
                );
                new_state = 8;
                didsay = true;
            }
            // C `case 8:` (`arkhata.c:2188-2193`).
            8 => {
                self.npc_say(
                    captain_id,
                    "The clerk has reported more notes missing, so he was not alone in this treachery. You should speak with the clerk.",
                );
                new_state = 9;
                didsay = true;
            }
            // C `case 9:` (`arkhata.c:2194-2199`).
            9 => {
                self.npc_say(
                    captain_id,
                    "And a little warning. Your entrance pass is only valid in open areas of the fortress, some places my guard will attack you still.",
                );
                new_state = 10;
                didsay = true;
            }
            // C `case 10: break;` (`arkhata.c:2200-2201`): all done.
            10 => {}
            _ => {}
        }

        if new_state != facts.captain_state {
            events.push(CaptainOutcomeEvent::UpdateCaptainState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`arkhata.c:2203-2207`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `captain_driver`'s `NT_TEXT` branch (`arkhata.c:2212-2241`),
    /// wired through the generic `analyse_text_qa` matcher.
    #[allow(clippy::too_many_arguments)]
    fn captain_handle_text_message(
        &mut self,
        captain_id: CharacterId,
        data: &mut CaptainDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, CaptainPlayerFacts>,
        events: &mut Vec<CaptainOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // { dat->current_victim = 0; }` (`arkhata.c:2215-2217`).
        let tick = self.tick.0;
        if tick > data.last_talk + CAPTAIN_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co)`
        // (`arkhata.c:2219`).
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
        let Some(captain_name) = self
            .characters
            .get(&captain_id)
            .map(|captain| captain.name.clone())
        else {
            return;
        };
        let Some(captain) = self.characters.get(&captain_id).cloned() else {
            return;
        };
        // C `analyse_text_driver`'s own guard clauses (`arkhata.c:189-
        // 203`): ignore our own talk, distance > 12, not-visible.
        if captain_id == speaker_id {
            return;
        }
        if char_dist(&captain, &speaker) > CAPTAIN_QA_DISTANCE
            || !char_see_char(&captain, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let captain_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.captain_state)
            .unwrap_or(0);
        let judge_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.judge_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, &captain_name, &speaker.name, ARKHATA_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(captain_id, &reply);
                didsay = true;
            }
            // "repeat"/"restart" (`arkhata.c:2224-2235`).
            TextAnalysisOutcome::Matched(2) => {
                // C `if (ppd->captain_state>0 && ppd->captain_state<=4 &&
                // ppd->judge_state==0)` (`arkhata.c:2227`).
                if captain_state > 0 && captain_state <= 4 && judge_state == 0 {
                    data.last_talk = 0;
                    events.push(CaptainOutcomeEvent::UpdateCaptainState {
                        player_id: speaker_id,
                        new_state: 1,
                    });
                }
                // C `if (ppd->captain_state>=5 && ppd->captain_state<=10)`
                // (`arkhata.c:2231`).
                if (5..=10).contains(&captain_state) {
                    data.last_talk = 0;
                    events.push(CaptainOutcomeEvent::UpdateCaptainState {
                        player_id: speaker_id,
                        new_state: 5,
                    });
                }
                didsay = true;
            }
            // Every other matched code (the 40 `"raise <skill>"` codes,
            // `enter`(5)/`aye`(6)/`watch`(7)) is unhandled by captain's
            // own `switch` but still counts as `didsay` (C: `switch
            // (didsay = analyse_text_driver(...))` - any nonzero return is
            // truthy).
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`arkhata.c:2237-2240`) - note this does *not* touch `dat->
        // last_talk` (except `case 2`'s own explicit resets above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `captain_driver`'s `NT_GIVE` branch (`arkhata.c:2244-2272`).
    fn captain_handle_give_message(
        &mut self,
        captain_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, CaptainPlayerFacts>,
        events: &mut Vec<CaptainOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&captain_id)
            .and_then(|captain| captain.cursor_item.take())
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

        // C `if (it[in].ID == IID_ARKHATA_LETTER1 && ppd->captain_state ==
        // 0)` (`arkhata.c:2248`): silent turn-in, no `say`/`quiet_say`
        // call at all - reproduced verbatim.
        if item.template_id == IID_ARKHATA_LETTER1
            && is_player
            && facts.is_some_and(|facts| facts.captain_state == 0)
        {
            self.destroy_items_by_template_id(giver_id, IID_ARKHATA_LETTER1);
            events.push(CaptainOutcomeEvent::UpdateCaptainState {
                player_id: giver_id,
                new_state: 1,
            });
            self.destroy_item(item_id);
            return;
        }

        // C `else if (it[in].ID == IID_ARKHATA_LETTER4 && ppd &&
        // !(ppd->letter_bits & 8))` (`arkhata.c:2255`).
        if item.template_id == IID_ARKHATA_LETTER4
            && is_player
            && facts.is_some_and(|facts| facts.letter_bits & CAPTAIN_LETTER4_BIT == 0)
        {
            self.npc_quiet_say(
                captain_id,
                "The judge has not left anything out, perfect! And thank thee; Rammy was right to trust thee.",
            );
            self.destroy_items_by_template_id(giver_id, IID_ARKHATA_LETTER4);
            events.push(CaptainOutcomeEvent::GiveLetter4Bit {
                player_id: giver_id,
            });
            self.destroy_item(item_id);
            return;
        }

        // C's fallback `else` branch (`arkhata.c:2264-2270`): hand the
        // item back to the giver.
        self.npc_say(
            captain_id,
            "Thou hast better use for this than I do. Well, if there is a use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_CAPTAIN;

/// C `struct std_npc_driver_data` (`arkhata.c:281-285`, shared by every
/// dialogue NPC in this file). `misc` is declared in C but never read or
/// written by `captain_driver` itself - no field for it here, same "only
/// port fields the driver actually uses" precedent as `world::npc::
/// area37::ramin`'s `RaminDriverData` doc comment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CaptainDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
