//! Rammy NPC (`CDR_RAMMY`), the ruler of Arkhata who runs "Rammy's Crown"
//! (quest 65) and "Entrance Passes" (quest 71).
//!
//! Ports `src/area/37/arkhata.c::rammy_driver` (`:287-569`) plus the shared
//! `analyse_text_driver`/`qa[]` table (`:109-169`, ported as
//! [`super::ARKHATA_QA`] in `world::npc::area37`). Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area29::
//! spiritbran`: the caller supplies a per-player fact snapshot
//! ([`RammyPlayerFacts`]) up front and applies the returned
//! [`RammyOutcomeEvent`]s afterwards, since `arkhata_ppd.rammy_state` and
//! the two `QLOG` entries live on `crate::player::PlayerRuntime`, not
//! `World`.
//!
//! `rammy_driver`'s twenty-state (`0`-`19`) dialogue chain, gated at three
//! points on cross-driver state this file cannot see directly (all read
//! via [`RammyPlayerFacts`]):
//! - `0` needs `staffer_ppd.guardbran_state >= 2` (`world::npc::area29::
//!   guardbran`'s own progress) to even start greeting.
//! - `6` needs `guardbran_state >= 7` to advance; C's own `case 6` falls
//!   through into `case 7`'s speech/`questlog_open(65)`/advance-to-`8` in
//!   the same tick - collapsed into one `rs == 6` arm here, same
//!   "fallthrough lands on the next case's action" precedent as
//!   `world::npc::area36::smith`. State `7` itself is never independently
//!   stored (the collapse always lands on `8`), so no separate arm exists
//!   for it.
//! - `13` needs `ch[co].level >= 54 && arkhata_ppd.monk_state >= 20` (the
//!   still-unported `arkhatamonk_driver`'s own progress) to advance,
//!   `questlog_open(71)`, then falls through into `case 14`'s speech/
//!   advance-to-`15` the same way - collapsed into one `rs == 13` arm.
//!   State `14` itself *can* be independently stored, unlike `7`/`18`: the
//!   `NT_TEXT` "repeat" reset (`case 2`, below) can rewind straight to
//!   `14`, so a dedicated `rs == 14` arm exists for that rewound case.
//! - `17` needs `arkhata_ppd.letter_bits == (2|4|8)` (bits written by the
//!   still-unported `ramin_driver`/`captain_driver` and the now-ported
//!   `world::npc::area29::countbran` cross-area write) to advance,
//!   `questlog_done(71)`, then falls through into `case 18`'s speech/
//!   advance-to-`19` - collapsed into one `rs == 17` arm; state `18` is
//!   never independently stored (no reset path targets it either), so no
//!   separate arm exists for it.
//!
//! Deviations/gaps (documented, not silent):
//! - Like every other `arkhata.c` NPC driver's own `NT_TEXT` branch, this
//!   driver has no `dat->current_victim` staleness-reset preamble and no
//!   victim-mismatch early-out - reproduced verbatim (matches `world::npc::
//!   area31::dwarfshaman`'s identical observation for that file's shared
//!   driver shape).
//! - `NT_GIVE`'s successful crown turn-in (`:534-541`) is silent in C - no
//!   `say`/`quiet_say` call at all, unlike almost every other quest-item
//!   turn-in in this codebase. Reproduced verbatim: only the gold/item
//!   fallback branch speaks.
//! - `case 16`'s fortress-key/letter1 hand-out (`:448-459`) each has its
//!   own independent `has_item` guard - a player who already carries one
//!   (e.g. from a previous partial run reset by god command) is not handed
//!   a duplicate.
//! - No self-defense/regen/spell-self cascade exists in C's `rammy_driver`
//!   body at all (matching the `brannington.c` "pure talker" NPC
//!   precedent) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:568`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::world::*;

use super::ARKHATA_QA;

/// C `char_dist(cn, co) > 10` (`arkhata.c:337`/`:2136` sibling drivers).
const RAMMY_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`arkhata.c:197`, the shared
/// `analyse_text_driver` copy's own guard).
const RAMMY_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`arkhata.c:320`).
const RAMMY_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`arkhata.c:325`).
const RAMMY_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`arkhata.c:562`): idle "return to post" threshold.
const RAMMY_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `ppd->letter_bits == (2 | 4 | 8)` (`arkhata.c:464`/`:2166`).
const RAMMY_ALL_LETTER_BITS: i32 = 2 | 4 | 8;
/// C quest 65, "Rammy's Crown".
const QLOG_RAMMY_CROWN: usize = 65;
/// C quest 71, "Entrance Passes".
const QLOG_RAMMY_ENTRANCE_PASSES: usize = 71;

/// Per-player facts [`World::process_rammy_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RammyPlayerFacts {
    /// `PlayerRuntime::arkhata_rammy_state()`.
    pub rammy_state: i32,
    /// `PlayerRuntime::staffer_guardbran_state()` (`sppd->guardbran_state`,
    /// `arkhata.c:349`/`:388`): gates `rs` `0`/`6`.
    pub guardbran_state: i32,
    /// `PlayerRuntime::arkhata_monk_state()` (`ppd->monk_state`,
    /// `arkhata.c:425`): gates `rs` `13`.
    pub monk_state: i32,
    /// `PlayerRuntime::arkhata_letter_bits()` (`ppd->letter_bits`,
    /// `arkhata.c:464`): gates `rs` `17`.
    pub letter_bits: i32,
}

/// A side effect [`World::process_rammy_actions`] could not apply directly
/// because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RammyOutcomeEvent {
    /// Write the new `arkhata_ppd.rammy_state` back.
    UpdateRammyState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 65)` (`arkhata.c:395`).
    QuestOpen65 { player_id: CharacterId },
    /// C `questlog_done(co, 65)` (`arkhata.c:535`), the `NT_GIVE` crown
    /// turn-in.
    QuestDone65 { player_id: CharacterId },
    /// C `questlog_open(co, 71)` (`arkhata.c:426`).
    QuestOpen71 { player_id: CharacterId },
    /// C `questlog_done(co, 71)` (`arkhata.c:471`).
    QuestDone71 { player_id: CharacterId },
    /// C `case 16:`'s item hand-out (`arkhata.c:448-459`):
    /// `create_item("key14_13_main")`/`create_item("letter1")`, each only
    /// if the player doesn't already carry one.
    GiveFortressKeyAndLetter {
        player_id: CharacterId,
        give_key: bool,
        give_letter: bool,
    },
}

impl World {
    /// C `rammy_driver`'s per-tick body (`arkhata.c:287-569`).
    pub fn process_rammy_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, RammyPlayerFacts>,
        area_id: u16,
    ) -> Vec<RammyOutcomeEvent> {
        let rammy_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_RAMMY
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for rammy_id in rammy_ids {
            self.process_rammy_messages(rammy_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_rammy_messages(
        &mut self,
        rammy_id: CharacterId,
        player_facts: &HashMap<CharacterId, RammyPlayerFacts>,
        area_id: u16,
        events: &mut Vec<RammyOutcomeEvent>,
    ) {
        let Some(rammy_name) = self
            .characters
            .get(&rammy_id)
            .map(|rammy| rammy.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Rammy(mut data)) = self
            .characters
            .get(&rammy_id)
            .and_then(|rammy| rammy.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&rammy_id)
            .map(|rammy| std::mem::take(&mut rammy.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.rammy_handle_char_message(
                    rammy_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.rammy_handle_text_message(
                    rammy_id,
                    &rammy_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.rammy_handle_give_message(rammy_id, message, player_facts, events),
                _ => {}
            }
        }

        if let Some(rammy) = self.characters.get_mut(&rammy_id) {
            rammy.driver_state = Some(CharacterDriverState::Rammy(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`arkhata.c:558-560`).
        if let (Some(rammy), Some((tx, ty))) =
            (self.characters.get(&rammy_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(rammy.x), i32::from(rammy.y), tx, ty) {
                if let Some(rammy_mut) = self.characters.get_mut(&rammy_id) {
                    let _ = turn(rammy_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_DOWN, ret,
        // lastact)) return; }` (`arkhata.c:562-566`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other stationary NPC in this codebase makes.
        let last_talk = if let Some(rammy) = self.characters.get(&rammy_id) {
            match rammy.driver_state.as_ref() {
                Some(CharacterDriverState::Rammy(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + RAMMY_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(rammy) = self.characters.get(&rammy_id) else {
                return;
            };
            let (post_x, post_y) = (rammy.rest_x, rammy.rest_y);
            self.secure_move_driver(
                rammy_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `rammy_driver`'s `NT_CHAR` branch (`arkhata.c:304-484`).
    #[allow(clippy::too_many_arguments)]
    fn rammy_handle_char_message(
        &mut self,
        rammy_id: CharacterId,
        data: &mut RammyDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, RammyPlayerFacts>,
        events: &mut Vec<RammyOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(rammy) = self.characters.get(&rammy_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER))` (`arkhata.c:308`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON)` (`arkhata.c:314`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5)` (`arkhata.c:320`).
        if tick < data.last_talk + RAMMY_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co)` (`arkhata.c:325`).
        if tick < data.last_talk + RAMMY_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co)` (`arkhata.c:331`).
        if rammy_id == player_id || !char_see_char(&rammy, &player, &self.map, self.date.daylight) {
            return;
        }
        // C `if (char_dist(cn, co) > 10)` (`arkhata.c:337`).
        if char_dist(&rammy, &player) > RAMMY_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.rammy_state;
        match facts.rammy_state {
            // C `case 0:` (`arkhata.c:346-357`): needs the town guard's
            // own progress first.
            0 if facts.guardbran_state >= 2 => {
                self.npc_quiet_say(rammy_id, "Hold! Stranger, where art thou from?");
                new_state = 1;
                didsay = true;
            }
            0 => {}
            // C `case 1:` (`arkhata.c:358-363`).
            1 => {
                self.npc_quiet_say(
                    rammy_id,
                    "Oh... I see that thou art a messenger from the Count Brannington, that is a most pleasent surprise to learn that he still holds the city.",
                );
                new_state = 2;
                didsay = true;
            }
            // C `case 2:` (`arkhata.c:364-369`).
            2 => {
                self.npc_quiet_say(
                    rammy_id,
                    "We haven't heard from the outside world in ages, are the demons still roaming wild out there?",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`arkhata.c:370-374`).
            3 => {
                self.npc_quiet_say(
                    rammy_id,
                    "So the Imperial Army has managed to keep them at bay for now? That is good to know.",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4:` (`arkhata.c:375-380`).
            4 => {
                self.npc_quiet_say(
                    rammy_id,
                    "Perhaps it is time for Arkhata to re-establish contact with the outside world. Please report back to the guard that we wish to open our passages to the city of Brannington.",
                );
                new_state = 5;
                didsay = true;
            }
            // C `case 5:` (`arkhata.c:381-385`).
            5 => {
                self.npc_quiet_say(
                    rammy_id,
                    "And come back here afterwards, I'm in need of thy help with another problem.",
                );
                new_state = 6;
                didsay = true;
            }
            // C `case 6:` falling through into `case 7:` (`arkhata.c:386-
            // 398`) - see the module doc comment.
            6 if facts.guardbran_state >= 7 => {
                self.npc_quiet_say(rammy_id, "Welcome back, friend!");
                events.push(RammyOutcomeEvent::QuestOpen65 { player_id });
                new_state = 8;
                didsay = true;
            }
            6 => {}
            // C `case 8:` (`arkhata.c:399-404`).
            8 => {
                self.npc_quiet_say(
                    rammy_id,
                    "A gang of bandits robbed me of my most precious item, my crown! They traveled south, past the fortress.",
                );
                new_state = 9;
                didsay = true;
            }
            // C `case 9:` (`arkhata.c:405-409`).
            9 => {
                self.npc_quiet_say(
                    rammy_id,
                    "Now, dear Adventurer, I ask thee to go there and return my crown. I would reward thee!",
                );
                new_state = 10;
                didsay = true;
            }
            // C `case 10: break;` (`arkhata.c:410-411`): waiting for the
            // crown, handled by `NT_GIVE`.
            10 => {}
            // C `case 11:` (`arkhata.c:413-417`).
            11 => {
                self.npc_quiet_say(rammy_id, "May Ishtar be with thee, I am forever grateful.");
                new_state = 12;
                didsay = true;
            }
            // C `case 12:` (`arkhata.c:418-423`).
            12 => {
                self.npc_quiet_say(
                    rammy_id,
                    "My friend Jaz in the town has asked for assistance, but I have no guards to spare. Wouldst thou be kind enough to visit him?",
                );
                new_state = 13;
                didsay = true;
            }
            // C `case 13:` falling through into `case 14:` (`arkhata.c:
            // 424-438`) - see the module doc comment.
            13 if player.level >= 54 && facts.monk_state >= 20 => {
                events.push(RammyOutcomeEvent::QuestOpen71 { player_id });
                self.npc_quiet_say(
                    rammy_id,
                    &format!(
                        "Hello again, {}! We have a problem with the guards in the fortress. They wont let people just pass through of course, but they are attacking everyone!",
                        player.name
                    ),
                );
                new_state = 15;
                didsay = true;
            }
            13 => {}
            // C `case 14:` (`arkhata.c:431-438`) reached directly only via
            // the `NT_TEXT` "repeat" rewind (see the module doc comment) -
            // `case 13`'s own fallthrough always lands on `15` in one tick.
            14 => {
                self.npc_quiet_say(
                    rammy_id,
                    &format!(
                        "Hello again, {}! We have a problem with the guards in the fortress. They wont let people just pass through of course, but they are attacking everyone!",
                        player.name
                    ),
                );
                new_state = 15;
                didsay = true;
            }
            // C `case 15:` (`arkhata.c:439-444`).
            15 => {
                self.npc_quiet_say(
                    rammy_id,
                    "It is not their fault as they have been trained for that. They know only citizens of Arkhata, and enemies.",
                );
                new_state = 16;
                didsay = true;
            }
            // C `case 16:` (`arkhata.c:445-462`).
            16 => {
                self.npc_quiet_say(
                    rammy_id,
                    "You must go see the captain in the fortress, hand him this letter. It will tell him who you are, and to find a solution to this issue",
                );
                let give_key =
                    !self.character_has_item_template(player_id, IID_ARKHATA_FORTRESSKEY);
                let give_letter = !self.character_has_item_template(player_id, IID_ARKHATA_LETTER1);
                if give_key || give_letter {
                    events.push(RammyOutcomeEvent::GiveFortressKeyAndLetter {
                        player_id,
                        give_key,
                        give_letter,
                    });
                }
                new_state = 17;
                didsay = true;
            }
            // C `case 17:` falling through into `case 18:` (`arkhata.c:
            // 463-474`) - see the module doc comment.
            17 if facts.letter_bits == RAMMY_ALL_LETTER_BITS => {
                self.npc_quiet_say(
                    rammy_id,
                    "Yet again thou hast been of great aid. The trade route is open and safe thanks to thee.",
                );
                events.push(RammyOutcomeEvent::QuestDone71 { player_id });
                new_state = 19;
                didsay = true;
            }
            17 => {}
            // C `case 19: break;` (`arkhata.c:475-476`): all done.
            19 => {}
            _ => {}
        }

        if new_state != facts.rammy_state {
            events.push(RammyOutcomeEvent::UpdateRammyState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`arkhata.c:478-483`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `rammy_driver`'s `NT_TEXT` branch (`arkhata.c:487-524`), wired
    /// through the generic `analyse_text_qa` matcher.
    #[allow(clippy::too_many_arguments)]
    fn rammy_handle_text_message(
        &mut self,
        rammy_id: CharacterId,
        rammy_name: &str,
        data: &mut RammyDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, RammyPlayerFacts>,
        events: &mut Vec<RammyOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // { dat->current_victim = 0; }` (`arkhata.c:490-492`).
        let tick = self.tick.0;
        if tick > data.last_talk + RAMMY_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co)`
        // (`arkhata.c:494`).
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
        let Some(rammy) = self.characters.get(&rammy_id).cloned() else {
            return;
        };
        // C `analyse_text_driver`'s own guard clauses (`arkhata.c:189-
        // 203`): ignore our own talk, distance > 12, not-visible.
        if rammy_id == speaker_id {
            return;
        }
        if char_dist(&rammy, &speaker) > RAMMY_QA_DISTANCE
            || !char_see_char(&rammy, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let rammy_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.rammy_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, rammy_name, &speaker.name, ARKHATA_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(rammy_id, &reply);
                didsay = true;
            }
            // "repeat"/"restart" (`arkhata.c:499-518`): rewind to the start
            // of whichever mini-block is in progress.
            TextAnalysisOutcome::Matched(2) => {
                if rammy_state <= 6 {
                    data.last_talk = 0;
                    events.push(RammyOutcomeEvent::UpdateRammyState {
                        player_id: speaker_id,
                        new_state: 0,
                    });
                } else if (7..=10).contains(&rammy_state) {
                    data.last_talk = 0;
                    events.push(RammyOutcomeEvent::UpdateRammyState {
                        player_id: speaker_id,
                        new_state: 7,
                    });
                } else if (12..=13).contains(&rammy_state) {
                    data.last_talk = 0;
                    events.push(RammyOutcomeEvent::UpdateRammyState {
                        player_id: speaker_id,
                        new_state: 12,
                    });
                } else if (14..=17).contains(&rammy_state) {
                    data.last_talk = 0;
                    events.push(RammyOutcomeEvent::UpdateRammyState {
                        player_id: speaker_id,
                        new_state: 14,
                    });
                }
                didsay = true;
            }
            // Every other matched code (`enter`(5)/`aye`(6)/`watch`(7), the
            // 40 `"raise <skill>"` codes) is unhandled by rammy's own
            // `switch` but still counts as `didsay` (C: `switch (didsay =
            // analyse_text_driver(...))` - any nonzero return is truthy).
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`arkhata.c:520-523`) - note this does *not* touch `dat->
        // last_talk` (except `case 2`'s own explicit resets above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `rammy_driver`'s `NT_GIVE` branch (`arkhata.c:527-550`).
    fn rammy_handle_give_message(
        &mut self,
        rammy_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, RammyPlayerFacts>,
        events: &mut Vec<RammyOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&rammy_id)
            .and_then(|rammy| rammy.cursor_item.take())
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

        // C `if (it[in].ID == IID_ARKHATA_CROWN && ppd->rammy_state == 10)`
        // (`arkhata.c:534`): silent on success - see the module doc
        // comment.
        if item.template_id == IID_ARKHATA_CROWN
            && is_player
            && facts.is_some_and(|facts| facts.rammy_state == 10)
        {
            events.push(RammyOutcomeEvent::QuestDone65 {
                player_id: giver_id,
            });
            self.destroy_items_by_template_id(giver_id, IID_ARKHATA_CROWN);
            events.push(RammyOutcomeEvent::UpdateRammyState {
                player_id: giver_id,
                new_state: 11,
            });
            self.destroy_item(item_id);
            return;
        }

        // C's fallback `else` branch (`arkhata.c:543-548`): hand the item
        // back to the giver.
        self.npc_say(
            rammy_id,
            "Thou hast better use for this than I do. Well, if there is a use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::{CDR_LOSTCON, CDR_RAMMY};
use crate::item_driver::{IID_ARKHATA_CROWN, IID_ARKHATA_FORTRESSKEY, IID_ARKHATA_LETTER1};

/// C `struct std_npc_driver_data` (`arkhata.c:281-285`, shared by every
/// dialogue NPC in this file). `misc` is declared in C but never read or
/// written by `rammy_driver` itself - no field for it here, same "only
/// port fields the driver actually uses" precedent as `world::npc::
/// area36`'s `caligar_ppd` doc comment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RammyDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}

/// Exposes [`QLOG_RAMMY_CROWN`] to `ugaris-server`'s `apply_rammy_events`.
pub const fn qlog_rammy_crown() -> usize {
    QLOG_RAMMY_CROWN
}

/// Exposes [`QLOG_RAMMY_ENTRANCE_PASSES`] to `ugaris-server`'s
/// `apply_rammy_events`.
pub const fn qlog_rammy_entrance_passes() -> usize {
    QLOG_RAMMY_ENTRANCE_PASSES
}
