//! Shanra NPC (`CDR_SHANRA`), the storyteller in the Brannington tower
//! dungeon's basement who rewards the tower's sentinel gauntlet with the
//! Grimoire of Animation and teleports adventurers there and back.
//!
//! Ports `src/area/29/brannington.c::shanra_driver` (`:2565-2720`) plus the
//! shared `analyse_text_driver`/`qa[]` table (`:86-206`, ported as
//! [`super::AREA29_QA`] in `world::npc::area29`, the same table every other
//! `brannington.c` NPC driver shares). Follows the exact same `World`-only
//! shape as `world::npc::area29::grinnich` (the sibling tower-flow NPC):
//! unlike most other `brannington.c` NPCs, `shanra_driver`'s own dialogue
//! chain performs [`World::teleport_char_driver`] calls directly at states
//! `1` and `4` - `World` owns the teleport machinery already, so no
//! `PlayerRuntime`-side outcome event is needed for them (same precedent as
//! `world::lab`/`world::jail`'s direct `self.teleport_char_driver(...)`
//! calls, unlike rewards that need `ZoneLoader`/DB access).
//!
//! `shanra_driver`'s six-state (`0`-`5`) dialogue chain: greeting ("made it
//! through my tower... work through some sentinels to find the Grimoire of
//! Animation") -> "I will now teleport you to the basement" (teleports to
//! `5,106`) -> (waiting: state `2`, for `CDR_CENTINEL`'s `centinel_dead`
//! kill-counter hook, `world::world_events::death_hooks::
//! apply_centinel_death_from_hurt_event`, to teleport the player back to
//! `33,143` on the 30th kill) -> "well done, learning about animation...
//! I will now send you back to the ruins above" -> "" (teleports to
//! `53,129`) -> (state `5`: all done).
//!
//! Deviations/gaps (documented, not silent):
//! - Like `world::npc::area29::grinnich`'s own `NT_TEXT` branch, this
//!   driver's own C body has no `dat->current_victim` staleness-reset
//!   preamble and no victim-mismatch early-out at all - reproduced
//!   verbatim: replies to *any* nearby player's matched small talk, not
//!   just its tracked victim.
//! - `case 2:` (`:2670-2677`) speaks a visible `say(cn, "reset done")`
//!   line (not `quiet_say`) before wiping the state - ported via
//!   [`crate::world::World::npc_say`], same precedent as `world::npc::
//!   area29::grinnich`'s own god-only reset.
//! - No self-defense/regen/spell-self cascade exists in C's `shanra_
//!   driver` body at all (matching the established "pure talker" NPC
//!   precedent) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:2719`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.
//! - C's `teleport_char_driver(co, ...)` return value is discarded at both
//!   call sites (no `if` guard) - reproduced verbatim: the state always
//!   advances and `didsay` is always set regardless of whether the
//!   teleport actually moved the player (same as C).

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::world::*;

use super::AREA29_QA;

/// C `char_dist(cn, co) > 10` (`brannington.c:2613`).
const SHANRA_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`brannington.c:127`, the shared
/// `analyse_text_driver` copy's own guard).
const SHANRA_QA_DISTANCE: i32 = 12;
/// C `TICKS * 4` (`brannington.c:2596`).
const SHANRA_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 4;
/// C `TICKS * 10` (`brannington.c:2601`).
const SHANRA_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`brannington.c:2713`): idle "return to post" threshold.
const SHANRA_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;
/// C `teleport_char_driver(co, 5, 106)` (`brannington.c:2643`): the tower
/// basement's dungeon entrance.
const SHANRA_TELEPORT_TO_BASEMENT: (u16, u16) = (5, 106);
/// C `teleport_char_driver(co, 53, 129)` (`brannington.c:2648`): back to
/// the Brannington ruins above.
const SHANRA_TELEPORT_TO_RUINS: (u16, u16) = (53, 129);

/// Per-player facts [`World::process_shanra_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShanraPlayerFacts {
    /// `PlayerRuntime::staffer_shanra_state()`.
    pub shanra_state: i32,
}

/// A side effect [`World::process_shanra_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShanraOutcomeEvent {
    /// Write the new `staffer_ppd.shanra_state` back.
    UpdateShanraState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `case 2:` (`brannington.c:2669-2678`): the god-only "reset me"
    /// state wipe.
    ResetShanra { player_id: CharacterId },
}

impl World {
    /// C `shanra_driver`'s per-tick body (`brannington.c:2565-2720`).
    pub fn process_shanra_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, ShanraPlayerFacts>,
        area_id: u16,
    ) -> Vec<ShanraOutcomeEvent> {
        let shanra_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_SHANRA
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for shanra_id in shanra_ids {
            self.process_shanra_messages(shanra_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_shanra_messages(
        &mut self,
        shanra_id: CharacterId,
        player_facts: &HashMap<CharacterId, ShanraPlayerFacts>,
        area_id: u16,
        events: &mut Vec<ShanraOutcomeEvent>,
    ) {
        let Some(shanra_name) = self
            .characters
            .get(&shanra_id)
            .map(|shanra| shanra.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Shanra(mut data)) = self
            .characters
            .get(&shanra_id)
            .and_then(|shanra| shanra.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&shanra_id)
            .map(|shanra| std::mem::take(&mut shanra.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.shanra_handle_char_message(
                    shanra_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.shanra_handle_text_message(
                    shanra_id,
                    &shanra_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.shanra_handle_give_message(shanra_id, message),
                _ => {}
            }
        }

        if let Some(shanra) = self.characters.get_mut(&shanra_id) {
            shanra.driver_state = Some(CharacterDriverState::Shanra(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`brannington.c:2709-2711`).
        if let (Some(shanra), Some((tx, ty))) =
            (self.characters.get(&shanra_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(shanra.x), i32::from(shanra.y), tx, ty) {
                if let Some(shanra_mut) = self.characters.get_mut(&shanra_id) {
                    let _ = turn(shanra_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)) return; }` (`brannington.c:2713-2717`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::npc::area29::grinnich` already uses.
        let last_talk = if let Some(shanra) = self.characters.get(&shanra_id) {
            match shanra.driver_state.as_ref() {
                Some(CharacterDriverState::Shanra(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + SHANRA_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(shanra) = self.characters.get(&shanra_id) else {
                return;
            };
            let (post_x, post_y) = (shanra.rest_x, shanra.rest_y);
            self.secure_move_driver(
                shanra_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `shanra_driver`'s `NT_CHAR` branch (`brannington.c:2581-2656`).
    #[allow(clippy::too_many_arguments)]
    fn shanra_handle_char_message(
        &mut self,
        shanra_id: CharacterId,
        data: &mut ShanraDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ShanraPlayerFacts>,
        events: &mut Vec<ShanraOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(shanra) = self.characters.get(&shanra_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`brannington.c:2584-2588`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`brannington.c:2590-2594`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*4) continue;`
        // (`brannington.c:2596-2600`).
        if tick < data.last_talk + SHANRA_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`brannington.c:2602-2605`).
        if tick < data.last_talk + SHANRA_TALK_VICTIM_TICKS
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`brannington.c:2607-2611`).
        if shanra_id == player_id || !char_see_char(&shanra, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`brannington.c:2613-
        // 2617`).
        if char_dist(&shanra, &player) > SHANRA_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.shanra_state;
        match facts.shanra_state {
            // C `case 0:` (`brannington.c:2624-2629`).
            0 => {
                self.npc_quiet_say(
                    shanra_id,
                    "Welcome adventurer, Grinnich told me you were coming. You have made it through my tower, and so I will reward you. You will have to work your way through some sentinels to find the Grimoire of Animation",
                );
                new_state = 1;
                didsay = true;
            }
            // C `case 1:` (`brannington.c:2630-2635`).
            1 => {
                self.npc_quiet_say(shanra_id, "I will now teleport you to the basement.");
                self.teleport_char_driver(
                    player_id,
                    SHANRA_TELEPORT_TO_BASEMENT.0,
                    SHANRA_TELEPORT_TO_BASEMENT.1,
                );
                new_state = 2;
                didsay = true;
            }
            // C `case 2: break;` (`brannington.c:2636`).
            2 => {}
            // C `case 3:` (`brannington.c:2641-2646`).
            3 => {
                self.npc_quiet_say(
                    shanra_id,
                    "Well done! It is good to see others learning about animation. I can't teach you how to use it though, the magic is ancient, and takes a long time to learn and control. I will now send you back to the ruins above.",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4:` (`brannington.c:2647-2650`).
            4 => {
                self.teleport_char_driver(
                    player_id,
                    SHANRA_TELEPORT_TO_RUINS.0,
                    SHANRA_TELEPORT_TO_RUINS.1,
                );
                new_state = 5;
                didsay = true;
            }
            // C `case 5: break;` (`brannington.c:2651`): all done.
            5 => {}
            _ => {}
        }

        if new_state != facts.shanra_state {
            events.push(ShanraOutcomeEvent::UpdateShanraState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`brannington.c:2653-2656`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `shanra_driver`'s `NT_TEXT` branch (`brannington.c:2659-2684`).
    #[allow(clippy::too_many_arguments)]
    fn shanra_handle_text_message(
        &mut self,
        shanra_id: CharacterId,
        shanra_name: &str,
        data: &mut ShanraDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, ShanraPlayerFacts>,
        events: &mut Vec<ShanraOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `if (ch[co].flags & CF_PLAYER)` (`brannington.c:2662`).
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }

        // C `analyse_text_driver`'s own guard clauses (`brannington.c:113-
        // 133`): ignore our own talk, non-players, distance > 12, not-
        // visible.
        if shanra_id == speaker_id {
            return;
        }
        let Some(shanra) = self.characters.get(&shanra_id).cloned() else {
            return;
        };
        if char_dist(&shanra, &speaker) > SHANRA_QA_DISTANCE
            || !char_see_char(&shanra, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let shanra_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.shanra_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, shanra_name, &speaker.name, AREA29_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(shanra_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`brannington.c:2669-2678`): reset back to the
            // greeting if not yet past it, or back to "waiting to learn
            // animation" if in the completed range.
            TextAnalysisOutcome::Matched(2) => {
                if shanra_state <= 2 {
                    data.last_talk = 0;
                    events.push(ShanraOutcomeEvent::UpdateShanraState {
                        player_id: speaker_id,
                        new_state: 0,
                    });
                }
                if (3..=5).contains(&shanra_state) {
                    data.last_talk = 0;
                    events.push(ShanraOutcomeEvent::UpdateShanraState {
                        player_id: speaker_id,
                        new_state: 3,
                    });
                }
                didsay = true;
            }
            // C `case 3:` (`brannington.c:2679-2683`): the god-only "reset
            // me" wipe (`qa[]`'s `"reset me"` -> `answer_code == 3`, same
            // shared table entry `world::npc::area29::grinnich` uses),
            // which speaks a visible `say(cn, "reset done")` line first
            // (see the module doc comment).
            TextAnalysisOutcome::Matched(3) => {
                if speaker.flags.contains(CharacterFlags::GOD) {
                    self.npc_say(shanra_id, "reset done");
                    events.push(ShanraOutcomeEvent::ResetShanra {
                        player_id: speaker_id,
                    });
                }
                didsay = true;
            }
            // Every other matched code (the area-shared `4`/`5` gold/silver
            // trade codes, consumed only by `broklin_driver`, not handled
            // by shanra's own `switch`) still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`brannington.c:2685-2688`) - note this does *not* touch `dat->
        // last_talk` (except `case 2`'s own explicit reset above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `shanra_driver`'s `NT_GIVE` branch (`brannington.c:2691-2705`):
    /// always hands the item straight back, no turn-in item is ever
    /// accepted by this NPC.
    fn shanra_handle_give_message(
        &mut self,
        shanra_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&shanra_id)
            .and_then(|shanra| shanra.cursor_item.take())
        else {
            return;
        };
        self.npc_quiet_say(
            shanra_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{CDR_LOSTCON, CDR_SHANRA};

/// C `struct shanra_data` (`src/area/29/brannington.c:2560-2563`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ShanraDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
