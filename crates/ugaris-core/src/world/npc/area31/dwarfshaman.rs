//! Dwarf Shaman NPC (`CDR_DWARFSHAMAN`), Grimroot's shaman, who runs
//! "Lizard's Teeth"/"Collecting Berries"/"Elitist Head" (quests 51-53).
//!
//! Ports `src/area/31/warrmines.c::dwarfshaman_driver` (`:467-716`) plus
//! the shared `analyse_text_driver`/`qa[]` table (`:70-194`, ported as
//! [`super::AREA31_QA`] in `world::npc::area31`). Follows the same
//! `World`/`PlayerRuntime` split as `world::npc::area31::dwarfchief`: the
//! caller supplies a per-player fact snapshot ([`DwarfshamanPlayerFacts`])
//! up front and applies the returned [`DwarfshamanOutcomeEvent`]s
//! afterwards, since `staffer_ppd.dwarfshaman_state`/`dwarfshaman_count`
//! and the `QLOG` 51-53 quest-log entries live on `crate::player::
//! PlayerRuntime`, not `World`.
//!
//! `dwarfshaman_driver`'s twelve-state (`0`-`11`) dialogue chain is three
//! back-to-back mini quests sharing one state counter: greeting (opens
//! quest 51) -> "bring me 9 lizard's teeth" (waiting: state `2`) ->
//! (`NT_GIVE`: nine `IID_LIZARDTOOTH` turn-ins, counted via
//! `dwarfshaman_count`, jump to state `3` and quest 51 done on the 9th) ->
//! if quest 52 already done, fast-forward to `6`; else "I've seen the
//! lizards..." (opens quest 52) -> "bring me 9 brown berries" (waiting:
//! state `5`) -> (`NT_GIVE`: nine `IID_BROWNBERRY` turn-ins, jump to state
//! `6` and quest 52 done on the 9th) -> if quest 53 already done,
//! fast-forward to `9`; else "thanks for the berries..." (opens quest 53)
//! -> "bring me the elite lizard's head" (waiting: state `8`) ->
//! (`NT_GIVE`: single `IID_LIZARDHEAD` turn-in, jump to state `9` and
//! quest 53 done) -> "this is quite amazing!" -> "thank you for helping
//! out!" -> done (state `11`).
//!
//! Deviations/gaps (documented, not silent):
//! - Like every other `warrmines.c` driver's own `NT_TEXT` branch, this
//!   driver has no `dat->current_victim` staleness-reset preamble and no
//!   victim-mismatch early-out - reproduced verbatim.
//! - C `case 2:` (`:619-636`) resets to whichever of the three mini
//!   quests' greeting state the player is currently mid-way through (four
//!   range checks: `<=2` -> `0`, `3..=5` -> `3`, `6..=8` -> `6`,
//!   `9..=11` -> `9`), ported as [`DwarfshamanOutcomeEvent::
//!   ResetToMiniQuestStart`].
//! - C `case 3:` (`:637-643`) speaks a visible `say(cn, "reset done")`
//!   line before wiping *both* `dwarfshaman_state` and `dwarfshaman_count`
//!   to `0` - only if the speaker is `CF_GOD`, ported as
//!   [`DwarfshamanOutcomeEvent::ResetDwarfshaman`] (unlike `dwarfchief`'s
//!   reset, which has no count to clear).
//! - `NT_GIVE`'s `IID_LIZARDHEAD` branch (`:680-685`) calls C
//!   `destroy_item_byID(co, IID_LIZARDHEAD)` - a *separate* scan of the
//!   giver's own remaining inventory for another copy of the same
//!   template, on top of the citem itself being destroyed by the trailing
//!   catch-all - preserved verbatim via [`World::destroy_items_by_
//!   template_id`] even though it is normally a no-op (the head was
//!   already handed over as the citem, not left in inventory).
//! - No self-defense/regen/spell-self cascade exists in C's
//!   `dwarfshaman_driver` body at all - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:715`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::world::*;

use super::AREA31_QA;

/// C `char_dist(cn, co) > 10` (`warrmines.c:516`).
const DWARFSHAMAN_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`warrmines.c:115`, the shared
/// `analyse_text_driver` copy's own guard).
const DWARFSHAMAN_QA_DISTANCE: i32 = 12;
/// C `TICKS * 4` (`warrmines.c:499`).
const DWARFSHAMAN_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 4;
/// C `TICKS * 10` (`warrmines.c:504`).
const DWARFSHAMAN_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`warrmines.c:709`): idle "return to post" threshold.
const DWARFSHAMAN_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `>= 9` (`warrmines.c:661`/`672`): turn-ins required per mini quest.
const DWARFSHAMAN_TURNIN_TARGET: i32 = 9;

/// Per-player facts [`World::process_dwarfshaman_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DwarfshamanPlayerFacts {
    /// `PlayerRuntime::staffer_dwarfshaman_state()`.
    pub dwarfshaman_state: i32,
    /// `PlayerRuntime::staffer_dwarfshaman_count()`.
    pub dwarfshaman_count: i32,
    /// `PlayerRuntime::quest_log.is_done(52)` (C `questlog_isdone(co,
    /// 52)`, `warrmines.c:545`): `case 3`'s fast-forward guard.
    pub quest52_is_done: bool,
    /// `PlayerRuntime::quest_log.is_done(53)` (C `questlog_isdone(co,
    /// 53)`, `warrmines.c:567`): `case 6`'s fast-forward guard.
    pub quest53_is_done: bool,
}

/// A side effect [`World::process_dwarfshaman_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DwarfshamanOutcomeEvent {
    /// Write the new `staffer_ppd.dwarfshaman_state` back.
    UpdateDwarfshamanState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// Write the new `staffer_ppd.dwarfshaman_count` back.
    UpdateDwarfshamanCount {
        player_id: CharacterId,
        new_count: i32,
    },
    /// C `questlog_open(co, 51/52/53)`.
    QuestOpen { player_id: CharacterId, quest: u32 },
    /// C `questlog_done(co, 51/52/53)`.
    QuestDone { player_id: CharacterId, quest: u32 },
    /// C `case 2:` (`warrmines.c:619-636`): reset back to the start of
    /// whichever of the three mini quests the player is currently mid-way
    /// through. `new_state` is already resolved to the target state.
    ResetToMiniQuestStart {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `case 3:` (`warrmines.c:637-643`): the god-only "reset me" full
    /// state+count wipe.
    ResetDwarfshaman { player_id: CharacterId },
}

impl World {
    /// C `dwarfshaman_driver`'s per-tick body (`warrmines.c:467-716`).
    pub fn process_dwarfshaman_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, DwarfshamanPlayerFacts>,
        area_id: u16,
    ) -> Vec<DwarfshamanOutcomeEvent> {
        let dwarfshaman_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_DWARFSHAMAN
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for dwarfshaman_id in dwarfshaman_ids {
            self.process_dwarfshaman_messages(dwarfshaman_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_dwarfshaman_messages(
        &mut self,
        dwarfshaman_id: CharacterId,
        player_facts: &HashMap<CharacterId, DwarfshamanPlayerFacts>,
        area_id: u16,
        events: &mut Vec<DwarfshamanOutcomeEvent>,
    ) {
        let Some(dwarfshaman_name) = self
            .characters
            .get(&dwarfshaman_id)
            .map(|dwarfshaman| dwarfshaman.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::DwarfShaman(mut data)) = self
            .characters
            .get(&dwarfshaman_id)
            .and_then(|dwarfshaman| dwarfshaman.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&dwarfshaman_id)
            .map(|dwarfshaman| std::mem::take(&mut dwarfshaman.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.dwarfshaman_handle_char_message(
                    dwarfshaman_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.dwarfshaman_handle_text_message(
                    dwarfshaman_id,
                    &dwarfshaman_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.dwarfshaman_handle_give_message(
                    dwarfshaman_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                _ => {}
            }
        }

        if let Some(dwarfshaman) = self.characters.get_mut(&dwarfshaman_id) {
            dwarfshaman.driver_state = Some(CharacterDriverState::DwarfShaman(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`warrmines.c:705-707`).
        if let (Some(dwarfshaman), Some((tx, ty))) =
            (self.characters.get(&dwarfshaman_id).cloned(), face_target)
        {
            if let Some(direction) =
                offset2dx(i32::from(dwarfshaman.x), i32::from(dwarfshaman.y), tx, ty)
            {
                if let Some(dwarfshaman_mut) = self.characters.get_mut(&dwarfshaman_id) {
                    let _ = turn(dwarfshaman_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)) return; }` (`warrmines.c:709-713`).
        let last_talk = if let Some(dwarfshaman) = self.characters.get(&dwarfshaman_id) {
            match dwarfshaman.driver_state.as_ref() {
                Some(CharacterDriverState::DwarfShaman(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + DWARFSHAMAN_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(dwarfshaman) = self.characters.get(&dwarfshaman_id) else {
                return;
            };
            let (post_x, post_y) = (dwarfshaman.rest_x, dwarfshaman.rest_y);
            self.secure_move_driver(
                dwarfshaman_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `dwarfshaman_driver`'s `NT_CHAR` branch (`warrmines.c:483-609`).
    #[allow(clippy::too_many_arguments)]
    fn dwarfshaman_handle_char_message(
        &mut self,
        dwarfshaman_id: CharacterId,
        data: &mut DwarfShamanDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, DwarfshamanPlayerFacts>,
        events: &mut Vec<DwarfshamanOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(dwarfshaman) = self.characters.get(&dwarfshaman_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        if tick < data.last_talk + DWARFSHAMAN_TALK_MIN_TICKS {
            return;
        }
        if tick < data.last_talk + DWARFSHAMAN_TALK_VICTIM_TICKS
            && data.current_victim != Some(player_id)
        {
            return;
        }
        if dwarfshaman_id == player_id
            || !char_see_char(&dwarfshaman, &player, &self.map, self.date.daylight)
        {
            return;
        }
        if char_dist(&dwarfshaman, &player) > DWARFSHAMAN_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.dwarfshaman_state;
        match facts.dwarfshaman_state {
            // C `case 0:` (`warrmines.c:526-533`).
            0 => {
                self.npc_quiet_say(
                    dwarfshaman_id,
                    "Welcome to Grimroot stranger. To make it here you must have battled some strong foes, though they're nothing compared to what you're about to face, should you accept the quest I am about to give you.",
                );
                events.push(DwarfshamanOutcomeEvent::QuestOpen {
                    player_id,
                    quest: 51,
                });
                new_state = 1;
                didsay = true;
            }
            // C `case 1:` (`warrmines.c:534-540`).
            1 => {
                self.npc_quiet_say(
                    dwarfshaman_id,
                    "But before I give you the quest, I want to see if you can fight the lizards you will be facing. Bring me back 9 lizard's teeth, and I will see that as proof of your strength.",
                );
                new_state = 2;
                didsay = true;
            }
            // C `case 2: break;` (`warrmines.c:541-542`): waiting for
            // teeth.
            2 => {}
            // C `case 3:` (`warrmines.c:544-555`).
            3 => {
                if facts.quest52_is_done {
                    new_state = 6;
                } else {
                    self.npc_quiet_say(
                        dwarfshaman_id,
                        "Ah! I see you've come back with all your teeth, and those of the lizards. I guess you are strong enough after all to do the quest I am about to give you. You see, I've seen the lizards come out with brown berries out of the water.",
                    );
                    events.push(DwarfshamanOutcomeEvent::QuestOpen {
                        player_id,
                        quest: 52,
                    });
                    new_state = 4;
                    didsay = true;
                }
            }
            // C `case 4:` (`warrmines.c:556-562`).
            4 => {
                self.npc_quiet_say(
                    dwarfshaman_id,
                    "Since I hate water, I need others like you to grab them for me. If you want to breath underwater, you will have to combine 3 flowers. I'll leave it up to you to figure out which ones. Now go get me 9 brown berries!",
                );
                new_state = 5;
                didsay = true;
            }
            // C `case 5: break;` (`warrmines.c:563-564`): waiting for
            // berries.
            5 => {}
            // C `case 6:` (`warrmines.c:566-577`).
            6 => {
                if facts.quest53_is_done {
                    new_state = 9;
                } else {
                    self.npc_quiet_say(
                        dwarfshaman_id,
                        "It's good that you can swim, you have no idea how much I hate water. Thanks for the berries. As I suspected, they seem to have magic properties, which I may be able to use.",
                    );
                    events.push(DwarfshamanOutcomeEvent::QuestOpen {
                        player_id,
                        quest: 53,
                    });
                    new_state = 7;
                    didsay = true;
                }
            }
            // C `case 7:` (`warrmines.c:578-584`).
            7 => {
                self.npc_quiet_say(
                    dwarfshaman_id,
                    "Also, I managed to learn some of the lizard's tongue, and overheard them talking in fear of an 'elite lizard'... If you can find it and bring it's head to me, I can learn more about these lizards, and why they're so varied.",
                );
                new_state = 8;
                didsay = true;
            }
            // C `case 8: break;` (`warrmines.c:585-586`): waiting for elite
            // head.
            8 => {}
            // C `case 9:` (`warrmines.c:588-594`).
            9 => {
                self.npc_quiet_say(
                    dwarfshaman_id,
                    "This is quite amazing! The reason these lizards are so varied is due to them being able to somehow absorb magical energy. To much of it seems to affect their mind however, as was the case with this elite lizard.",
                );
                new_state = 10;
                didsay = true;
            }
            // C `case 10:` (`warrmines.c:595-600`).
            10 => {
                self.npc_quiet_say(
                    dwarfshaman_id,
                    "Thank you for helping out! I guess you are sturdier than you look, even though your kind looks skinnier than a dwarven skeleton!",
                );
                new_state = 11;
                didsay = true;
            }
            // C `case 11: break;` (`warrmines.c:601-602`): all done.
            11 => {}
            _ => {}
        }

        if new_state != facts.dwarfshaman_state {
            events.push(DwarfshamanOutcomeEvent::UpdateDwarfshamanState {
                player_id,
                new_state,
            });
        }

        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `dwarfshaman_driver`'s `NT_TEXT` branch (`warrmines.c:613-650`),
    /// wired through the generic `analyse_text_qa` matcher.
    #[allow(clippy::too_many_arguments)]
    fn dwarfshaman_handle_text_message(
        &mut self,
        dwarfshaman_id: CharacterId,
        dwarfshaman_name: &str,
        data: &mut DwarfShamanDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, DwarfshamanPlayerFacts>,
        events: &mut Vec<DwarfshamanOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }

        if dwarfshaman_id == speaker_id {
            return;
        }
        let Some(dwarfshaman) = self.characters.get(&dwarfshaman_id).cloned() else {
            return;
        };
        if char_dist(&dwarfshaman, &speaker) > DWARFSHAMAN_QA_DISTANCE
            || !char_see_char(&dwarfshaman, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let dwarfshaman_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.dwarfshaman_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, dwarfshaman_name, &speaker.name, AREA31_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(dwarfshaman_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`warrmines.c:619-636`): reset back to the start
            // of whichever of the three mini quests is in progress.
            TextAnalysisOutcome::Matched(2) => {
                let new_state = if dwarfshaman_state <= 2 {
                    Some(0)
                } else if (3..=5).contains(&dwarfshaman_state) {
                    Some(3)
                } else if (6..=8).contains(&dwarfshaman_state) {
                    Some(6)
                } else if (9..=11).contains(&dwarfshaman_state) {
                    Some(9)
                } else {
                    None
                };
                if let Some(new_state) = new_state {
                    data.last_talk = 0;
                    events.push(DwarfshamanOutcomeEvent::ResetToMiniQuestStart {
                        player_id: speaker_id,
                        new_state,
                    });
                }
                didsay = true;
            }
            // C `case 3:` (`warrmines.c:637-643`): the god-only "reset me"
            // wipe, which speaks a visible `say(cn, "reset done")` line
            // first and clears both `dwarfshaman_state` and
            // `dwarfshaman_count`.
            TextAnalysisOutcome::Matched(3) => {
                if speaker.flags.contains(CharacterFlags::GOD) {
                    self.npc_say(dwarfshaman_id, "reset done");
                    events.push(DwarfshamanOutcomeEvent::ResetDwarfshaman {
                        player_id: speaker_id,
                    });
                }
                didsay = true;
            }
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `dwarfshaman_driver`'s `NT_GIVE` branch (`warrmines.c:653-697`).
    #[allow(clippy::too_many_arguments)]
    fn dwarfshaman_handle_give_message(
        &mut self,
        dwarfshaman_id: CharacterId,
        data: &mut DwarfShamanDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, DwarfshamanPlayerFacts>,
        events: &mut Vec<DwarfshamanOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&dwarfshaman_id)
            .and_then(|dwarfshaman| dwarfshaman.cursor_item.take())
        else {
            return;
        };
        let Some(item) = self.items.get(&item_id).cloned() else {
            return;
        };
        let Some(giver) = self.characters.get(&giver_id).cloned() else {
            return;
        };
        let facts = player_facts.get(&giver_id).copied();

        // C `if (it[in].ID == IID_LIZARDTOOTH && ppd &&
        // ppd->dwarfshaman_state < 3)` (`warrmines.c:659`).
        if item.template_id == IID_LIZARDTOOTH
            && facts.is_some_and(|facts| facts.dwarfshaman_state < 3)
        {
            let count = facts.map(|facts| facts.dwarfshaman_count).unwrap_or(0) + 1;
            events.push(DwarfshamanOutcomeEvent::UpdateDwarfshamanCount {
                player_id: giver_id,
                new_count: if count >= DWARFSHAMAN_TURNIN_TARGET {
                    0
                } else {
                    count
                },
            });
            if count >= DWARFSHAMAN_TURNIN_TARGET {
                events.push(DwarfshamanOutcomeEvent::UpdateDwarfshamanState {
                    player_id: giver_id,
                    new_state: 3,
                });
                data.last_talk = 0;
                events.push(DwarfshamanOutcomeEvent::QuestDone {
                    player_id: giver_id,
                    quest: 51,
                });
            } else {
                self.npc_quiet_say(
                    dwarfshaman_id,
                    &format!("{count} done, {} to go.", DWARFSHAMAN_TURNIN_TARGET - count),
                );
            }
            *face_target = Some((i32::from(giver.x), i32::from(giver.y)));
            data.current_victim = Some(giver_id);
            self.destroy_item(item_id);
            return;
        }
        // C `else if (it[in].ID == IID_BROWNBERRY && ppd &&
        // ppd->dwarfshaman_state >= 3 && ppd->dwarfshaman_state <= 5)`
        // (`warrmines.c:669-670`).
        if item.template_id == IID_BROWNBERRY
            && facts.is_some_and(|facts| (3..=5).contains(&facts.dwarfshaman_state))
        {
            let count = facts.map(|facts| facts.dwarfshaman_count).unwrap_or(0) + 1;
            events.push(DwarfshamanOutcomeEvent::UpdateDwarfshamanCount {
                player_id: giver_id,
                new_count: if count >= DWARFSHAMAN_TURNIN_TARGET {
                    0
                } else {
                    count
                },
            });
            if count >= DWARFSHAMAN_TURNIN_TARGET {
                events.push(DwarfshamanOutcomeEvent::UpdateDwarfshamanState {
                    player_id: giver_id,
                    new_state: 6,
                });
                data.last_talk = 0;
                events.push(DwarfshamanOutcomeEvent::QuestDone {
                    player_id: giver_id,
                    quest: 52,
                });
            } else {
                self.npc_quiet_say(
                    dwarfshaman_id,
                    &format!("{count} done, {} to go.", DWARFSHAMAN_TURNIN_TARGET - count),
                );
            }
            *face_target = Some((i32::from(giver.x), i32::from(giver.y)));
            data.current_victim = Some(giver_id);
            self.destroy_item(item_id);
            return;
        }
        // C `else if (it[in].ID == IID_LIZARDHEAD && ppd &&
        // ppd->dwarfshaman_state >= 6 && ppd->dwarfshaman_state <= 8)`
        // (`warrmines.c:680-681`).
        if item.template_id == IID_LIZARDHEAD
            && facts.is_some_and(|facts| (6..=8).contains(&facts.dwarfshaman_state))
        {
            events.push(DwarfshamanOutcomeEvent::UpdateDwarfshamanState {
                player_id: giver_id,
                new_state: 9,
            });
            data.last_talk = 0;
            events.push(DwarfshamanOutcomeEvent::QuestDone {
                player_id: giver_id,
                quest: 53,
            });
            // C `destroy_item_byID(co, IID_LIZARDHEAD)` (`warrmines.c:685`):
            // a second, normally-no-op scan of the giver's own remaining
            // inventory (see the module doc comment).
            self.destroy_items_by_template_id(giver_id, IID_LIZARDHEAD);
            *face_target = Some((i32::from(giver.x), i32::from(giver.y)));
            data.current_victim = Some(giver_id);
            self.destroy_item(item_id);
            return;
        }

        // C's fallback `else if (give_char_item(co, in))` branch
        // (`warrmines.c:686-689`): hand the item back to the giver.
        self.npc_quiet_say(
            dwarfshaman_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{CDR_DWARFSHAMAN, CDR_LOSTCON};
use crate::item_driver::{IID_BROWNBERRY, IID_LIZARDHEAD, IID_LIZARDTOOTH};

/// C `struct dwarfshaman_data` (`src/area/31/warrmines.c:462-465`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DwarfShamanDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
