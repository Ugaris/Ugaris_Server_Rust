//! Trainer NPC (`CDR_TRAINER`), the Fighting School combat trainer who
//! runs "A Kidnapped Student" (quest 75).
//!
//! Ports `src/area/37/arkhata.c::trainer_driver` (`:3834-4013`) plus the
//! shared `analyse_text_driver`/`qa[]` table (`:115-169`, ported as
//! [`super::ARKHATA_QA`] in `world::npc::area37`). Follows the same
//! `World`/`PlayerRuntime` split established by `world::npc::area37::
//! ramin`/`thaipan`: the caller supplies a per-player fact snapshot
//! ([`TrainerPlayerFacts`]) up front and applies the returned
//! [`TrainerOutcomeEvent`]s afterwards, since `arkhata_ppd.trainer_state`
//! lives on `crate::player::PlayerRuntime`, not `World`.
//!
//! `trainer_driver`'s nine-state (`0`-`8`) dialogue chain, gated at two
//! points on cross-driver state this file cannot see directly (both read
//! via [`TrainerPlayerFacts`]):
//! - `0` needs `ch[co].level >= 53 && arkhata_ppd.fiona_state >= 4`
//!   (`world::npc::area37::fiona`'s own progress) to advance; C's own
//!   `case 0` falls through into `case 1`'s speech/`questlog_open(75)`/
//!   advance-to-`2` in the same tick - collapsed into one `rs == 0` arm
//!   here, same "fallthrough lands on the next case's action" precedent
//!   as `world::npc::area37::ramin`'s own `rs == 0`/`9`/`11` arms.
//! - `6` is a pure wait state gated on `arkhata_ppd.kid_state == 5`
//!   (`world::npc::area37::kidnappee`'s own "rescued" state): once true,
//!   C's `case 6` falls through into `case 7`'s speech/`questlog_done
//!   (75)`/advance-to-`8` the same way - collapsed into one `rs == 6` arm.
//!
//! Deviations/gaps (documented, not silent):
//! - Like every other `arkhata.c` NPC driver's own `NT_TEXT` branch, this
//!   driver has no `dat->current_victim` staleness-reset preamble and no
//!   victim-mismatch early-out - reproduced verbatim (matches
//!   `world::npc::area37::rammy`/`ramin`/`thaipan`'s identical
//!   observation for that file's shared driver shape).
//! - `NT_GIVE` never accepts any item - the only branch is the "hand it
//!   back" fallback (`arkhata.c:3986-3994`).
//! - No self-defense/regen/spell-self cascade exists in C's
//!   `trainer_driver` body at all (matching the `rammy`/`ramin`/`thaipan`
//!   "pure talker" NPC precedent) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:4012`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, CDR_LOSTCON};
use crate::drvlib::offset2dx;
use crate::world::*;

use super::ARKHATA_QA;

/// C `char_dist(cn, co) > 10` (`arkhata.c:3883`, sibling drivers' own
/// identical guard).
const TRAINER_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`arkhata.c:197`, the shared
/// `analyse_text_driver` copy's own guard).
const TRAINER_QA_DISTANCE: i32 = 12;
/// C `TICKS * 5` (`arkhata.c:3866`).
const TRAINER_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 10` (`arkhata.c:3871`).
const TRAINER_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`arkhata.c:4006`): idle "return to post" threshold.
const TRAINER_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C quest 75, "A Kidnapped Student".
const QLOG_TRAINER_STUDENT: usize = 75;

/// Per-player facts [`World::process_trainer_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrainerPlayerFacts {
    /// `PlayerRuntime::arkhata_trainer_state()`.
    pub trainer_state: i32,
    /// `PlayerRuntime::arkhata_fiona_state()` (`ppd->fiona_state`,
    /// `arkhata.c:3894`): gates `rs` `0`.
    pub fiona_state: i32,
    /// `PlayerRuntime::arkhata_kid_state()` (`ppd->kid_state`,
    /// `arkhata.c:3930`): gates `rs` `6`.
    pub kid_state: i32,
}

/// A side effect [`World::process_trainer_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrainerOutcomeEvent {
    /// Write the new `arkhata_ppd.trainer_state` back.
    UpdateTrainerState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `questlog_open(co, 75)` (`arkhata.c:3902`).
    QuestOpen75 { player_id: CharacterId },
    /// C `questlog_done(co, 75)` (`arkhata.c:3931`).
    QuestDone75 { player_id: CharacterId },
}

impl World {
    /// C `trainer_driver`'s per-tick body (`arkhata.c:3834-4013`).
    pub fn process_trainer_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, TrainerPlayerFacts>,
        area_id: u16,
    ) -> Vec<TrainerOutcomeEvent> {
        let trainer_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_TRAINER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for trainer_id in trainer_ids {
            self.process_trainer_messages(trainer_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_trainer_messages(
        &mut self,
        trainer_id: CharacterId,
        player_facts: &HashMap<CharacterId, TrainerPlayerFacts>,
        area_id: u16,
        events: &mut Vec<TrainerOutcomeEvent>,
    ) {
        let Some(trainer_name) = self
            .characters
            .get(&trainer_id)
            .map(|trainer| trainer.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Trainer(mut data)) = self
            .characters
            .get(&trainer_id)
            .and_then(|trainer| trainer.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&trainer_id)
            .map(|trainer| std::mem::take(&mut trainer.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.trainer_handle_char_message(
                    trainer_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.trainer_handle_text_message(
                    trainer_id,
                    &trainer_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.trainer_handle_give_message(trainer_id, message),
                _ => {}
            }
        }

        if let Some(trainer) = self.characters.get_mut(&trainer_id) {
            trainer.driver_state = Some(CharacterDriverState::Trainer(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`arkhata.c:4002-4004`).
        if let (Some(trainer), Some((tx, ty))) =
            (self.characters.get(&trainer_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(trainer.x), i32::from(trainer.y), tx, ty) {
                if let Some(trainer_mut) = self.characters.get_mut(&trainer_id) {
                    let _ = turn(trainer_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)) return; }` (`arkhata.c:4006-4010`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution every other stationary NPC in this codebase makes.
        let last_talk = if let Some(trainer) = self.characters.get(&trainer_id) {
            match trainer.driver_state.as_ref() {
                Some(CharacterDriverState::Trainer(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + TRAINER_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(trainer) = self.characters.get(&trainer_id) else {
                return;
            };
            let (post_x, post_y) = (trainer.rest_x, trainer.rest_y);
            self.secure_move_driver(
                trainer_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `trainer_driver`'s `NT_CHAR` branch (`arkhata.c:3850-3951`).
    #[allow(clippy::too_many_arguments)]
    fn trainer_handle_char_message(
        &mut self,
        trainer_id: CharacterId,
        data: &mut TrainerDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TrainerPlayerFacts>,
        events: &mut Vec<TrainerOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(trainer) = self.characters.get(&trainer_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER))` (`arkhata.c:3854`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON)` (`arkhata.c:3860`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*5)` (`arkhata.c:3866`).
        if tick < data.last_talk + TRAINER_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // && dat->current_victim != co)` (`arkhata.c:3871`).
        if tick < data.last_talk + TRAINER_TALK_VICTIM_TICKS
            && data.current_victim.is_some()
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co)` (`arkhata.c:3877`).
        if trainer_id == player_id
            || !char_see_char(&trainer, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10)` (`arkhata.c:3883`).
        if char_dist(&trainer, &player) > TRAINER_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.trainer_state;
        match facts.trainer_state {
            // C `case 0:` falling through into `case 1:` (`arkhata.c:
            // 3893-3905`) - see the module doc comment.
            0 if player.level >= 53 && facts.fiona_state >= 4 => {
                self.npc_quiet_say(
                    trainer_id,
                    "Greetings, adventurer who returned the ring to my Queen Fiona. As one who may train in this academy now, your loyalty is expected in return.",
                );
                events.push(TrainerOutcomeEvent::QuestOpen75 { player_id });
                new_state = 2;
                didsay = true;
            }
            0 => {}
            // C `case 2:` (`arkhata.c:3906-3911`).
            2 => {
                self.npc_quiet_say(
                    trainer_id,
                    "And I'm in need of thine services. One of my students has been kidnapped by the gang who has their guild east of our fine establishment.",
                );
                new_state = 3;
                didsay = true;
            }
            // C `case 3:` (`arkhata.c:3912-3917`).
            3 => {
                self.npc_quiet_say(
                    trainer_id,
                    "Usually the evil gang doesn't trouble us, but one of my more reckless students ventured too far on his own.",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4:` (`arkhata.c:3918-3922`).
            4 => {
                self.npc_quiet_say(
                    trainer_id,
                    "Now I believe the gang leader is torturing him, or worse, to learn our fighting secrets.",
                );
                new_state = 5;
                didsay = true;
            }
            // C `case 5:` (`arkhata.c:3923-3928`).
            5 => {
                self.npc_quiet_say(
                    trainer_id,
                    "I need you to go get the student back, before our secrets are given away. Time is short, so go now!",
                );
                new_state = 6;
                didsay = true;
            }
            // C `case 6:` falling through into `case 7:` (`arkhata.c:
            // 3929-3941`) - see the module doc comment.
            6 if facts.kid_state == 5 => {
                events.push(TrainerOutcomeEvent::QuestDone75 { player_id });
                self.npc_quiet_say(
                    trainer_id,
                    "Thank thee great fighter. Now that my student is safe I can sleep at night. You have proven your skills once again. The door to the academy will always be open to you.",
                );
                new_state = 8;
                didsay = true;
            }
            6 => {}
            // C `case 8: break;` (`arkhata.c:3942-3943`): all done.
            8 => {}
            _ => {}
        }

        if new_state != facts.trainer_state {
            events.push(TrainerOutcomeEvent::UpdateTrainerState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`arkhata.c:3945-3949`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `trainer_driver`'s `NT_TEXT` branch (`arkhata.c:3954-3983`),
    /// wired through the generic `analyse_text_qa` matcher.
    #[allow(clippy::too_many_arguments)]
    fn trainer_handle_text_message(
        &mut self,
        trainer_id: CharacterId,
        trainer_name: &str,
        data: &mut TrainerDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TrainerPlayerFacts>,
        events: &mut Vec<TrainerOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        // C `if (ticker > dat->last_talk + TICKS*10 && dat->current_victim)
        // { dat->current_victim = 0; }` (`arkhata.c:3957-3959`).
        let tick = self.tick.0;
        if tick > data.last_talk + TRAINER_TALK_VICTIM_TICKS && data.current_victim.is_some() {
            data.current_victim = None;
        }
        // C `if (dat->current_victim && dat->current_victim != co)`
        // (`arkhata.c:3961`).
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
        let Some(trainer) = self.characters.get(&trainer_id).cloned() else {
            return;
        };
        // C `analyse_text_driver`'s own guard clauses (`arkhata.c:189-
        // 203`): ignore our own talk, distance > 12, not-visible.
        if trainer_id == speaker_id {
            return;
        }
        if char_dist(&trainer, &speaker) > TRAINER_QA_DISTANCE
            || !char_see_char(&trainer, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let trainer_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.trainer_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, trainer_name, &speaker.name, ARKHATA_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(trainer_id, &reply);
                didsay = true;
            }
            // "repeat"/"restart" (`arkhata.c:3966-3977`): rewind to the
            // start of whichever mini-block is in progress. C's two `if`s
            // are independent but mutually exclusive in practice (a state
            // can never satisfy both ranges at once).
            TextAnalysisOutcome::Matched(2) => {
                if trainer_state > 0 && trainer_state <= 6 {
                    data.last_talk = 0;
                    events.push(TrainerOutcomeEvent::UpdateTrainerState {
                        player_id: speaker_id,
                        new_state: 1,
                    });
                }
                if (7..=8).contains(&trainer_state) {
                    data.last_talk = 0;
                    events.push(TrainerOutcomeEvent::UpdateTrainerState {
                        player_id: speaker_id,
                        new_state: 7,
                    });
                }
                didsay = true;
            }
            // Every other matched code (the 40 `"raise <skill>"` codes,
            // `enter`(5)/`aye`(6)/`watch`(7)) is unhandled by trainer's
            // own `switch` but still counts as `didsay` (C: `switch
            // (didsay = analyse_text_driver(...))` - any nonzero return
            // is truthy).
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`arkhata.c:3979-3982`) - note this does *not* touch `dat->
        // last_talk` (except the "repeat" branch's own explicit resets
        // above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `trainer_driver`'s `NT_GIVE` branch (`arkhata.c:3986-3994`): the
    /// only behavior it has is handing the item straight back.
    fn trainer_handle_give_message(
        &mut self,
        trainer_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&trainer_id)
            .and_then(|trainer| trainer.cursor_item.take())
        else {
            return;
        };
        self.npc_say(
            trainer_id,
            "Thou hast better use for this than I do. Well, if there is a use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_TRAINER;

/// C `struct std_npc_driver_data` (`arkhata.c:281-285`, shared by every
/// dialogue NPC in this file). `misc` is declared in C but never read or
/// written by `trainer_driver` itself - no field for it here, same "only
/// port fields the driver actually uses" precedent as `world::npc::
/// area37::ramin`'s `RaminDriverData` doc comment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TrainerDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}

/// Exposes [`QLOG_TRAINER_STUDENT`] to `ugaris-server`'s
/// `apply_trainer_events`.
pub const fn qlog_trainer_student() -> usize {
    QLOG_TRAINER_STUDENT
}
