//! Lab 3 mute prisoner (`CDR_LAB3PRISONER`).
//!
//! Ports `src/area/22/lab3.c::lab3_prisoner_driver` (`:317-536`). There is
//! exactly one prisoner instance in the whole game (`ugaris_data/zones/22/
//! lab3.chr:337`), a wordless "Blub."-only NPC who, once given the
//! `IID_LAB3_PRISONKEY`, mimes out a note-giving sequence and hands the
//! player a note carrying half of `lab3_passguard`'s door password (the
//! other half comes from elsewhere in the lab; the note's own text-reveal
//! behavior lives in the still-unported `IDR_LAB3_SPECIAL` item driver -
//! see the module doc comment gap note below).
//!
//! Deviations/gaps (documented, not silent):
//! - `give_driver(cn, dat->give_target)` (C's generic "walk toward and
//!   `do_give` when adjacent" driver, `src/system/drvlib.c:350-406`) is
//!   simplified to a direct pathfind-and-walk-one-step-then-
//!   [`World::give_char_item`] call once adjacent, bypassing C's queued
//!   `AC_GIVE` action/`duration` timer - the prisoner has no fight/other
//!   competing action that timer could ever interrupt, so this is
//!   observably identical (just faster by the give action's own
//!   duration).
//! - `standard_message_driver(cn, msg, 0, 0)` is not reproduced (dead
//!   code for `agressive=0, helper=0`), same precedent as every other
//!   NPC's own module doc comment in this directory.
//! - **Known gap**: `IDR_LAB3_SPECIAL` (`lab3_special`'s `drdata[0]==3`
//!   note-reading logic, the half that actually reveals `password2` text
//!   to the reader) is not yet ported - the note item this driver hands
//!   out is fully created and given, but reading/using it is currently a
//!   no-op until that item driver is ported (`PORTING_TODO.md`'s Area 22
//!   entry).

use crate::direction::Direction;
use crate::drvlib::char_dist;
use crate::path::pathfinder;
use crate::see::char_see_char;
use crate::world::*;

/// C `TICKS * 3` (`lab3.c:402,410,418,427`): most talk-step advance
/// delays.
const LAB3_PRISONER_TALK_STEP_TICKS: u64 = TICKS_PER_SECOND * 3;
/// C `TICKS * 2` (`lab3.c:450`): case-21 advance delay.
const LAB3_PRISONER_TALK_STEP21_TICKS: u64 = TICKS_PER_SECOND * 2;
/// C `TICKS * 5` (`lab3.c:491`): "BLUB" reset delay.
const LAB3_PRISONER_BLUB_RESET_TICKS: u64 = TICKS_PER_SECOND * 5;
/// C `TICKS * 1` (`lab3.c:495`): "REPEAT" reset delay.
#[allow(clippy::identity_op)] // `* 1` kept to mirror C's `TICKS * 1` literally
const LAB3_PRISONER_REPEAT_RESET_TICKS: u64 = TICKS_PER_SECOND * 1;
/// C `char_dist(cn, co) > 10` (`lab3.c:383`).
const LAB3_PRISONER_TALK_DISTANCE: i32 = 10;
/// C `dist_from_home(cn, dat->give_target) > 8` (`lab3.c:505`).
const LAB3_PRISONER_GIVE_MAX_DIST: i32 = 8;
/// C `TICKS * 30` (`lab3.c:529`): idle "return to post" threshold.
const LAB3_PRISONER_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `SAYDIST * 2` (`lab3.c:398,406,414,423`): the prisoner's mime gestures
/// are described to a wider area than a normal `say`.
const LAB3_PRISONER_GESTURE_DIST: u16 = (crate::legacy::SAY_DIST as u16) * 2;

/// Per-player facts [`World::process_lab3_prisoner_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lab3PrisonerPlayerFacts {
    /// `PlayerRuntime::legacy_lab3_prisoner_talkstep()`.
    pub prisoner_talkstep: u8,
}

/// A side effect [`World::process_lab3_prisoner_actions`] could not apply
/// directly because it touches `PlayerRuntime`/needs `ZoneLoader`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lab3PrisonerOutcomeEvent {
    /// Write the new `ppd->prisoner_talkstep` back.
    SetPrisonerTalkstep { player_id: CharacterId, value: u8 },
    /// C `case 20:` note creation (`lab3.c:430-444`): needs `ZoneLoader`
    /// to instantiate `"lab3_note_generic"` onto the prisoner's own
    /// cursor - applied by `ugaris-server`'s `area22::
    /// create_lab3_note_on_cursor`.
    CreateNoteOnCursor { npc_id: CharacterId },
}

impl World {
    /// C `lab3_prisoner_driver`'s per-tick body (`lab3.c:317-536`).
    pub fn process_lab3_prisoner_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, Lab3PrisonerPlayerFacts>,
        area_id: u16,
    ) -> Vec<Lab3PrisonerOutcomeEvent> {
        let prisoner_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_LAB3PRISONER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for prisoner_id in prisoner_ids {
            self.process_lab3_prisoner_tick(prisoner_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_lab3_prisoner_tick(
        &mut self,
        prisoner_id: CharacterId,
        player_facts: &HashMap<CharacterId, Lab3PrisonerPlayerFacts>,
        area_id: u16,
        events: &mut Vec<Lab3PrisonerOutcomeEvent>,
    ) {
        let Some(CharacterDriverState::Lab3Prisoner(mut data)) = self
            .characters
            .get(&prisoner_id)
            .and_then(|prisoner| prisoner.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&prisoner_id)
            .map(|prisoner| std::mem::take(&mut prisoner.driver_messages))
            .unwrap_or_default();

        let mut talkdir: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_GIVE => self.lab3_prisoner_handle_give(prisoner_id, message, events),
                NT_CHAR => self.lab3_prisoner_handle_char(
                    prisoner_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut talkdir,
                ),
                NT_TEXT => {
                    self.lab3_prisoner_handle_text(prisoner_id, message, player_facts, events)
                }
                _ => {}
            }
        }

        // C: `if (dat->give_target) { ... }` (`:503-523`).
        let gave_this_tick = self.lab3_prisoner_process_give(prisoner_id, &mut data, events);

        if let Some(prisoner) = self.characters.get_mut(&prisoner_id) {
            prisoner.driver_state = Some(CharacterDriverState::Lab3Prisoner(data));
        }

        if gave_this_tick {
            return;
        }

        // C `if (talkdir) turn(cn, talkdir);` (`:525-527`).
        if let (Some(prisoner), Some((tx, ty))) =
            (self.characters.get(&prisoner_id).cloned(), talkdir)
        {
            if let Some(direction) =
                crate::drvlib::offset2dx(i32::from(prisoner.x), i32::from(prisoner.y), tx, ty)
            {
                if let Some(prisoner_mut) = self.characters.get_mut(&prisoner_id) {
                    let _ = turn(prisoner_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, tmpx, tmpy, DX_DOWN, ret, lastact))
        // return; }` (`:529-533`). `tmpx`/`tmpy` reuse `rest_x`/`rest_y`.
        let data = match self
            .characters
            .get(&prisoner_id)
            .and_then(|prisoner| prisoner.driver_state.as_ref())
        {
            Some(CharacterDriverState::Lab3Prisoner(data)) => *data,
            _ => return,
        };
        if data.last_talk_tick + LAB3_PRISONER_RETURN_TO_POST_TICKS < self.tick.0 {
            let (post_x, post_y) = self
                .characters
                .get(&prisoner_id)
                .map(|prisoner| (prisoner.rest_x, prisoner.rest_y))
                .unwrap_or_default();
            let _ = self.secure_move_driver(
                prisoner_id,
                post_x,
                post_y,
                Direction::Down as u8,
                0,
                0,
                area_id,
            );
        }
        // C `do_idle(cn, TICKS);` (`:535`) - not modeled, same precedent
        // as every other stationary dialogue-only NPC in this codebase.
    }

    /// C `lab3_prisoner_driver`'s `NT_GIVE` branch (`:335-353`).
    fn lab3_prisoner_handle_give(
        &mut self,
        prisoner_id: CharacterId,
        message: &CharacterDriverMessage,
        events: &mut Vec<Lab3PrisonerOutcomeEvent>,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get(&prisoner_id)
            .and_then(|prisoner| prisoner.cursor_item)
        else {
            return;
        };
        // C `if (ppd && ch[co].flags & CF_PLAYER && it[in].ID ==
        // IID_LAB3_PRISONKEY) { ppd->prisoner_talkstep = 20; }` (`:342-348`).
        let is_key = self
            .items
            .get(&item_id)
            .is_some_and(|item| item.template_id == crate::item_driver::IID_LAB3_PRISONKEY);
        let giver_is_player = self
            .characters
            .get(&giver_id)
            .is_some_and(|giver| giver.flags.contains(CharacterFlags::PLAYER));
        if is_key && giver_is_player {
            events.push(Lab3PrisonerOutcomeEvent::SetPrisonerTalkstep {
                player_id: giver_id,
                value: 20,
            });
        }
        // C `destroy_item(ch[cn].citem); ch[cn].citem = 0;` (`:350-352`):
        // destroy everything we get, key or not.
        self.destroy_item(item_id);
        if let Some(prisoner) = self.characters.get_mut(&prisoner_id) {
            prisoner.cursor_item = None;
        }
    }

    /// C `lab3_prisoner_driver`'s `NT_CHAR` branch (`:355-458`).
    #[allow(clippy::too_many_arguments)]
    fn lab3_prisoner_handle_char(
        &mut self,
        prisoner_id: CharacterId,
        data: &mut Lab3PrisonerDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, Lab3PrisonerPlayerFacts>,
        events: &mut Vec<Lab3PrisonerOutcomeEvent>,
        talkdir: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };
        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`:359-362`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`:365-368`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        // C `if (ticker < dat->next_talk) { remove_message; continue; }`
        // (`:371-374`).
        let tick = self.tick.0;
        if tick < data.next_talk_tick {
            return;
        }
        let Some(prisoner) = self.characters.get(&prisoner_id).cloned() else {
            return;
        };
        // C `if (!char_see_char(cn, co) || cn == co) { remove_message;
        // continue; }` (`:377-380`).
        if prisoner_id == player_id
            || !char_see_char(&prisoner, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) { remove_message; continue; }`
        // (`:383-386`).
        if char_dist(&prisoner, &player) > LAB3_PRISONER_TALK_DISTANCE {
            return;
        }
        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let name = &player.name;
        // C `switch (ppd->prisoner_talkstep) { ... }` (`:395-452`).
        match facts.prisoner_talkstep {
            0 => {
                self.npc_say(prisoner_id, "Blub.");
                self.log_area_gesture(
                    &prisoner,
                    &format!("The Prisoner looks glad to see thee, {name}."),
                );
                didsay = true;
                events.push(Lab3PrisonerOutcomeEvent::SetPrisonerTalkstep {
                    player_id,
                    value: 1,
                });
                data.next_talk_tick = tick + LAB3_PRISONER_TALK_STEP_TICKS;
            }
            1 => {
                self.log_area_gesture(
                    &prisoner,
                    &format!(
                        "The Prisoner points to thee, {name}, and moves his Hands like unlocking a door."
                    ),
                );
                didsay = true;
                events.push(Lab3PrisonerOutcomeEvent::SetPrisonerTalkstep {
                    player_id,
                    value: 2,
                });
                data.next_talk_tick = tick + LAB3_PRISONER_TALK_STEP_TICKS;
            }
            2 => {
                self.log_area_gesture(
                    &prisoner,
                    "He points to the imaginary key, then he points to himself.",
                );
                didsay = true;
                events.push(Lab3PrisonerOutcomeEvent::SetPrisonerTalkstep {
                    player_id,
                    value: 3,
                });
                data.next_talk_tick = tick + LAB3_PRISONER_TALK_STEP_TICKS;
            }
            3 => {
                self.log_area_gesture(
                    &prisoner,
                    "Now he makes signs like giving something to thee.",
                );
                didsay = true;
                events.push(Lab3PrisonerOutcomeEvent::SetPrisonerTalkstep {
                    player_id,
                    value: 255,
                });
                data.next_talk_tick = tick + LAB3_PRISONER_TALK_STEP_TICKS;
            }
            // C `case 20: // GIVE NOTE` (`:430-444`).
            20 => {
                if data.give_target.is_none()
                    && self
                        .characters
                        .get(&prisoner_id)
                        .is_some_and(|prisoner| prisoner.cursor_item.is_none())
                {
                    events.push(Lab3PrisonerOutcomeEvent::CreateNoteOnCursor {
                        npc_id: prisoner_id,
                    });
                    data.give_target = Some(player_id);
                    data.give_serial = player.serial;
                }
            }
            21 => {
                self.npc_say(prisoner_id, "Blub.");
                didsay = true;
                events.push(Lab3PrisonerOutcomeEvent::SetPrisonerTalkstep {
                    player_id,
                    value: 255,
                });
                data.next_talk_tick = tick + LAB3_PRISONER_TALK_STEP21_TICKS;
            }
            _ => {}
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir =
        // offset2dx(...); }` (`:454-457`).
        if didsay {
            data.last_talk_tick = tick;
            *talkdir = Some((i32::from(player.x), i32::from(player.y)));
        }
    }

    /// C `lab3_prisoner_driver`'s `NT_TEXT` branch (`:460-497`).
    fn lab3_prisoner_handle_text(
        &mut self,
        prisoner_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, Lab3PrisonerPlayerFacts>,
        events: &mut Vec<Lab3PrisonerOutcomeEvent>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        let Some(text) = message.text.as_deref() else {
            return;
        };
        if let Some(speaker) = self.characters.get(&speaker_id).cloned() {
            self.apply_tabunga_text_notification(prisoner_id, speaker_id, text);
            // C `if (co == cn) { remove_message; continue; }` (`:468-471`).
            if speaker_id == prisoner_id {
                return;
            }
            // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message;
            // continue; }` (`:472-475`).
            if !speaker.flags.contains(CharacterFlags::PLAYER) {
                return;
            }
            let Some(prisoner) = self.characters.get(&prisoner_id).cloned() else {
                return;
            };
            // C `if (!char_see_char(cn, co)) { remove_message; continue; }`
            // (`:476-479`).
            if !char_see_char(&prisoner, &speaker, &self.map, self.date.daylight) {
                return;
            }
            if player_facts.get(&speaker_id).is_none() {
                return;
            }

            let text_lower = text.to_lowercase();
            if text_lower.contains("blub") {
                self.npc_say(prisoner_id, "Blub!");
                events.push(Lab3PrisonerOutcomeEvent::SetPrisonerTalkstep {
                    player_id: speaker_id,
                    value: 0,
                });
                if let Some(CharacterDriverState::Lab3Prisoner(data)) = self
                    .characters
                    .get_mut(&prisoner_id)
                    .and_then(|prisoner| prisoner.driver_state.as_mut())
                {
                    data.next_talk_tick = self.tick.0 + LAB3_PRISONER_BLUB_RESET_TICKS;
                }
            } else if text_lower.contains("repeat") {
                self.npc_say(prisoner_id, "Blub.");
                events.push(Lab3PrisonerOutcomeEvent::SetPrisonerTalkstep {
                    player_id: speaker_id,
                    value: 0,
                });
                if let Some(CharacterDriverState::Lab3Prisoner(data)) = self
                    .characters
                    .get_mut(&prisoner_id)
                    .and_then(|prisoner| prisoner.driver_state.as_mut())
                {
                    data.next_talk_tick = self.tick.0 + LAB3_PRISONER_REPEAT_RESET_TICKS;
                }
            }
        }
    }

    /// C: "if (dat->give_target) { ... }" (`:503-523`): walk toward and
    /// give the pending note, or cancel/destroy it if the target left.
    /// Returns `true` if this tick's action was consumed by a pursuit
    /// move (matching C's `if (give_driver(...)) return;`).
    fn lab3_prisoner_process_give(
        &mut self,
        prisoner_id: CharacterId,
        data: &mut Lab3PrisonerDriverData,
        events: &mut Vec<Lab3PrisonerOutcomeEvent>,
    ) -> bool {
        let Some(target_id) = data.give_target else {
            return false;
        };
        let Some(prisoner) = self.characters.get(&prisoner_id).cloned() else {
            return false;
        };
        let target = self.characters.get(&target_id).cloned();
        let target_gone = target
            .as_ref()
            .is_none_or(|target| target.serial != data.give_serial);
        let too_far = target.as_ref().is_some_and(|target| {
            dist_from_home(target, prisoner.rest_x, prisoner.rest_y) > LAB3_PRISONER_GIVE_MAX_DIST
        });
        if target_gone || too_far {
            data.give_target = None;
            if let Some(item_id) = self
                .characters
                .get(&prisoner_id)
                .and_then(|prisoner| prisoner.cursor_item)
            {
                self.destroy_item(item_id);
                if let Some(prisoner) = self.characters.get_mut(&prisoner_id) {
                    prisoner.cursor_item = None;
                }
            }
            return false;
        }
        let target = target.unwrap();

        let Some(item_id) = self
            .characters
            .get(&prisoner_id)
            .and_then(|prisoner| prisoner.cursor_item)
        else {
            // Nothing left to give (already given last tick, or the
            // `CreateNoteOnCursor` event hasn't been applied yet).
            return false;
        };

        if adjacent_direction(
            prisoner.x,
            prisoner.y,
            usize::from(target.x),
            usize::from(target.y),
        )
        .is_some()
        {
            if self.give_char_item(target_id, item_id) {
                if let Some(prisoner) = self.characters.get_mut(&prisoner_id) {
                    prisoner.cursor_item = None;
                }
                events.push(Lab3PrisonerOutcomeEvent::SetPrisonerTalkstep {
                    player_id: target_id,
                    value: 21,
                });
                data.give_target = None;
                data.last_talk_tick = self
                    .tick
                    .0
                    .saturating_sub(LAB3_PRISONER_TALK_STEP_TICKS * 28);
                return false;
            }
            return false;
        }

        let (fx, fy) = (usize::from(prisoner.x), usize::from(prisoner.y));
        let (tx, ty) = (usize::from(target.x), usize::from(target.y));
        let path = pathfinder(&self.map, fx, fy, tx, ty, 1, None);
        let Some(direction) = path.direction else {
            return false;
        };
        self.walk_or_use_driver(prisoner_id, direction, self.area_id)
    }

    /// C `log_area(x, y, LOG_SYSTEM, 0, SAYDIST*2, "...", ...)` (the
    /// prisoner's mime-gesture narration, e.g. `lab3.c:398-399`): a plain
    /// area broadcast with no `"<name> says:"` prefix.
    fn log_area_gesture(&mut self, character: &Character, message: &str) {
        self.pending_area_texts.push(WorldAreaText {
            x: character.x,
            y: character.y,
            max_distance: LAB3_PRISONER_GESTURE_DIST,
            message: message.to_string(),
        });
    }
}

/// C `dist_from_home(cn, co)` (`src/system/drvlib.c:2366-2377`).
fn dist_from_home(character: &Character, home_x: u16, home_y: u16) -> i32 {
    let dx = (i32::from(character.x) - i32::from(home_x)).abs();
    let dy = (i32::from(character.y) - i32::from(home_y)).abs();
    if dx > dy {
        (dx << 1) + dy
    } else {
        (dy << 1) + dx
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct lab3_prisoner_driver_data` (`lab3.c:311-315`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Lab3PrisonerDriverData {
    #[serde(default)]
    pub last_talk_tick: u64,
    #[serde(default)]
    pub next_talk_tick: u64,
    #[serde(default)]
    pub give_target: Option<CharacterId>,
    #[serde(default)]
    pub give_serial: u32,
}
