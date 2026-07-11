//! Grinnich NPC (`CDR_GRINNICH`), the hermit at the entrance of the
//! Brannington tower dungeon who hints at the buried tower and hands
//! adventurers off to Shanra in the basement.
//!
//! Ports `src/area/29/brannington.c::grinnich_driver` (`:2402-2557`) plus
//! the shared `analyse_text_driver`/`qa[]` table (`:86-206`, ported as
//! [`super::AREA29_QA`] in `world::npc::area29`, the same table every other
//! `brannington.c` NPC driver shares). Follows the exact same `World`-only
//! shape as `world::npc::area29::spiritbran`/`daughterbran` (no reward
//! needs `PlayerRuntime`-only application beyond the state itself, so the
//! only [`GrinnichOutcomeEvent`] variants are the state write and the
//! god-only reset).
//!
//! `grinnich_driver`'s five-state (`0`-`4`) dialogue chain: greeting ("this
//! is a tower!") -> "find the entrance, buried in the ground" -> (waiting:
//! state `2`, for the player to solve the tower and reach Shanra, who
//! bumps `grinnich_state` to `3` once she greets them, `world::npc::
//! area29::shanra`'s own `case 0`) -> "wasn't it worth it? Isn't Shanra
//! wonderful?" -> (state `4`: all done).
//!
//! Deviations/gaps (documented, not silent):
//! - Like `world::npc::area29::spiritbran`'s own `NT_TEXT` branch, this
//!   driver's own C body has no `dat->current_victim` staleness-reset
//!   preamble and no victim-mismatch early-out at all - reproduced
//!   verbatim: replies to *any* nearby player's matched small talk, not
//!   just its tracked victim.
//! - `case 2:` (`:2506-2510`) speaks a visible `say(cn, "reset done")`
//!   line (not `quiet_say`) before wiping the state - ported via
//!   [`crate::world::World::npc_say`], same precedent as `world::npc::
//!   area29::spiritbran`'s own god-only reset.
//! - No self-defense/regen/spell-self cascade exists in C's `grinnich_
//!   driver` body at all (matching the established "pure talker" NPC
//!   precedent) - this port omits it too.
//! - C's unconditional `do_idle(cn, TICKS)` tail call (`:2556`) is not
//!   ported, matching the established stationary-dialogue-NPC precedent.

use std::collections::HashMap;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};
use crate::drvlib::offset2dx;
use crate::world::*;

use super::AREA29_QA;

/// C `char_dist(cn, co) > 10` (`brannington.c:2450`).
const GRINNICH_GREET_DISTANCE: i32 = 10;
/// C `char_dist(cn, co) > 12` (`brannington.c:127`, the shared
/// `analyse_text_driver` copy's own guard).
const GRINNICH_QA_DISTANCE: i32 = 12;
/// C `TICKS * 4` (`brannington.c:2433`).
const GRINNICH_TALK_MIN_TICKS: u64 = TICKS_PER_SECOND * 4;
/// C `TICKS * 10` (`brannington.c:2438`).
const GRINNICH_TALK_VICTIM_TICKS: u64 = TICKS_PER_SECOND * 10;
/// C `TICKS * 30` (`brannington.c:2550`): idle "return to post" threshold.
const GRINNICH_RETURN_TO_POST_TICKS: u64 = TICKS_PER_SECOND * 30;

/// Per-player facts [`World::process_grinnich_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrinnichPlayerFacts {
    /// `PlayerRuntime::staffer_grinnich_state()`.
    pub grinnich_state: i32,
}

/// A side effect [`World::process_grinnich_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrinnichOutcomeEvent {
    /// Write the new `staffer_ppd.grinnich_state` back.
    UpdateGrinnichState {
        player_id: CharacterId,
        new_state: i32,
    },
    /// C `case 3:` (`brannington.c:2505-2511`): the god-only "reset me"
    /// state wipe.
    ResetGrinnich { player_id: CharacterId },
}

/// C `Sirname(cn)` (`src/system/tool.c:1538-1546`), used by `case 1:`'s
/// greeting instead of the player's real name - same "private per-file
/// copy" precedent as `world::npc::area17::servant`/`world::npc::area11::
/// islena`.
fn grinnich_sirname(character: &Character) -> &'static str {
    if character.flags.contains(CharacterFlags::MALE) {
        "Sir"
    } else if character.flags.contains(CharacterFlags::FEMALE) {
        "Lady"
    } else {
        "Neuter"
    }
}

impl World {
    /// C `grinnich_driver`'s per-tick body (`brannington.c:2402-2557`).
    pub fn process_grinnich_actions(
        &mut self,
        player_facts: &HashMap<CharacterId, GrinnichPlayerFacts>,
        area_id: u16,
    ) -> Vec<GrinnichOutcomeEvent> {
        let grinnich_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_GRINNICH
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for grinnich_id in grinnich_ids {
            self.process_grinnich_messages(grinnich_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_grinnich_messages(
        &mut self,
        grinnich_id: CharacterId,
        player_facts: &HashMap<CharacterId, GrinnichPlayerFacts>,
        area_id: u16,
        events: &mut Vec<GrinnichOutcomeEvent>,
    ) {
        let Some(grinnich_name) = self
            .characters
            .get(&grinnich_id)
            .map(|grinnich| grinnich.name.clone())
        else {
            return;
        };
        let Some(CharacterDriverState::Grinnich(mut data)) = self
            .characters
            .get(&grinnich_id)
            .and_then(|grinnich| grinnich.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&grinnich_id)
            .map(|grinnich| std::mem::take(&mut grinnich.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => self.grinnich_handle_char_message(
                    grinnich_id,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_TEXT => self.grinnich_handle_text_message(
                    grinnich_id,
                    &grinnich_name,
                    &mut data,
                    message,
                    player_facts,
                    events,
                    &mut face_target,
                ),
                NT_GIVE => self.grinnich_handle_give_message(grinnich_id, message),
                _ => {}
            }
        }

        if let Some(grinnich) = self.characters.get_mut(&grinnich_id) {
            grinnich.driver_state = Some(CharacterDriverState::Grinnich(data));
        }

        // C `if (talkdir) turn(cn, talkdir);` (`brannington.c:2546-2548`).
        if let (Some(grinnich), Some((tx, ty))) =
            (self.characters.get(&grinnich_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(grinnich.x), i32::from(grinnich.y), tx, ty)
            {
                if let Some(grinnich_mut) = self.characters.get_mut(&grinnich_id) {
                    let _ = turn(grinnich_mut, direction as u8);
                }
            }
        }

        // C `if (dat->last_talk + TICKS*30 < ticker) { if
        // (secure_move_driver(cn, ch[cn].tmpx, ch[cn].tmpy, DX_RIGHT, ret,
        // lastact)) return; }` (`brannington.c:2550-2554`). The NPC's post
        // position (C's `tmpx`/`tmpy`) reuses `rest_x`/`rest_y`, the same
        // substitution `world::npc::area29::spiritbran` already uses.
        let last_talk = if let Some(grinnich) = self.characters.get(&grinnich_id) {
            match grinnich.driver_state.as_ref() {
                Some(CharacterDriverState::Grinnich(data)) => data.last_talk,
                _ => return,
            }
        } else {
            return;
        };
        if last_talk + GRINNICH_RETURN_TO_POST_TICKS < self.tick.0 {
            let Some(grinnich) = self.characters.get(&grinnich_id) else {
                return;
            };
            let (post_x, post_y) = (grinnich.rest_x, grinnich.rest_y);
            self.secure_move_driver(
                grinnich_id,
                post_x,
                post_y,
                Direction::Right as u8,
                0,
                0,
                area_id,
            );
        }
    }

    /// C `grinnich_driver`'s `NT_CHAR` branch (`brannington.c:2418-2493`).
    #[allow(clippy::too_many_arguments)]
    fn grinnich_handle_char_message(
        &mut self,
        grinnich_id: CharacterId,
        data: &mut GrinnichDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, GrinnichPlayerFacts>,
        events: &mut Vec<GrinnichOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let player_id = CharacterId(message.dat1.max(0) as u32);
        let Some(grinnich) = self.characters.get(&grinnich_id).cloned() else {
            return;
        };
        let Some(player) = self.characters.get(&player_id).cloned() else {
            return;
        };

        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`brannington.c:2421-2425`).
        if !player.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `if (ch[co].driver == CDR_LOSTCON) { remove_message; continue;
        // }` (`brannington.c:2427-2431`).
        if player.driver == CDR_LOSTCON {
            return;
        }
        let tick = self.tick.0;
        // C `if (ticker < dat->last_talk + TICKS*4) continue;`
        // (`brannington.c:2433-2437`).
        if tick < data.last_talk + GRINNICH_TALK_MIN_TICKS {
            return;
        }
        // C `if (ticker < dat->last_talk + TICKS*10 && dat->current_victim
        // != co) continue;` (`brannington.c:2439-2442`).
        if tick < data.last_talk + GRINNICH_TALK_VICTIM_TICKS
            && data.current_victim != Some(player_id)
        {
            return;
        }
        // C `if (!char_see_char(cn, co) || cn == co) continue;`
        // (`brannington.c:2444-2448`).
        if grinnich_id == player_id
            || !char_see_char(&grinnich, &player, &self.map, self.date.daylight)
        {
            return;
        }
        // C `if (char_dist(cn, co) > 10) continue;` (`brannington.c:2450-
        // 2454`).
        if char_dist(&grinnich, &player) > GRINNICH_GREET_DISTANCE {
            return;
        }

        let Some(facts) = player_facts.get(&player_id) else {
            return;
        };

        let mut didsay = false;
        let mut new_state = facts.grinnich_state;
        match facts.grinnich_state {
            // C `case 0:` (`brannington.c:2459-2466`).
            0 => {
                self.npc_quiet_say(
                    grinnich_id,
                    "Oh my! What brings the likes of you to this hermit's home? Adventure? Treasure? You sure do look like an adventurer... Well, if it's adventure you seek, then adventure you get! You see, this here is no ordinary place... It's a tower!",
                );
                new_state = 1;
                didsay = true;
            }
            // C `case 1:` (`brannington.c:2467-2475`).
            1 => {
                self.npc_quiet_say(
                    grinnich_id,
                    &format!(
                        "Oh no, I'm not crazy dear {}! It is! It's just buried into the ground. Find the entrance that leads to the tower, and venture deep into the earth. I tell you, you will not regret it at all!",
                        grinnich_sirname(&player)
                    ),
                );
                new_state = 2;
                didsay = true;
            }
            // C `case 2: break;` (`brannington.c:2476`): waiting for
            // completion of quest.
            2 => {}
            // C `case 3:` (`brannington.c:2481-2486`).
            3 => {
                self.npc_quiet_say(
                    grinnich_id,
                    "Didn't I tell you it was worth it? Oh, all that knowledge just makes the mind grow! Isn't Shanra wonderful? She can tell so many stories, more even than those books of her can...",
                );
                new_state = 4;
                didsay = true;
            }
            // C `case 4: break;` (`brannington.c:2487`): all done.
            4 => {}
            _ => {}
        }

        if new_state != facts.grinnich_state {
            events.push(GrinnichOutcomeEvent::UpdateGrinnichState {
                player_id,
                new_state,
            });
        }

        // C `if (didsay) { dat->last_talk = ticker; talkdir = ...;
        // dat->current_victim = co; }` (`brannington.c:2489-2492`).
        if didsay {
            data.last_talk = tick;
            *face_target = Some((i32::from(player.x), i32::from(player.y)));
            data.current_victim = Some(player_id);
        }
    }

    /// C `grinnich_driver`'s `NT_TEXT` branch (`brannington.c:2496-2521`).
    #[allow(clippy::too_many_arguments)]
    fn grinnich_handle_text_message(
        &mut self,
        grinnich_id: CharacterId,
        grinnich_name: &str,
        data: &mut GrinnichDriverData,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, GrinnichPlayerFacts>,
        events: &mut Vec<GrinnichOutcomeEvent>,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);

        let Some(text) = message.text.as_deref() else {
            return;
        };
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };

        // C `if (ch[co].flags & CF_PLAYER)` (`brannington.c:2499`).
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }

        // C `analyse_text_driver`'s own guard clauses (`brannington.c:113-
        // 133`): ignore our own talk, non-players, distance > 12, not-
        // visible.
        if grinnich_id == speaker_id {
            return;
        }
        let Some(grinnich) = self.characters.get(&grinnich_id).cloned() else {
            return;
        };
        if char_dist(&grinnich, &speaker) > GRINNICH_QA_DISTANCE
            || !char_see_char(&grinnich, &speaker, &self.map, self.date.daylight)
        {
            return;
        }

        let grinnich_state = player_facts
            .get(&speaker_id)
            .map(|facts| facts.grinnich_state)
            .unwrap_or(0);

        let mut didsay = false;
        match analyse_text_qa(text, grinnich_name, &speaker.name, AREA29_QA) {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say(grinnich_id, &reply);
                didsay = true;
            }
            // C `case 2:` (`brannington.c:2504-2512`): reset back to the
            // greeting if not yet past it, or back to "waiting for Shanra"
            // if in the completed range.
            TextAnalysisOutcome::Matched(2) => {
                if grinnich_state <= 2 {
                    data.last_talk = 0;
                    events.push(GrinnichOutcomeEvent::UpdateGrinnichState {
                        player_id: speaker_id,
                        new_state: 0,
                    });
                }
                if (3..=4).contains(&grinnich_state) {
                    data.last_talk = 0;
                    events.push(GrinnichOutcomeEvent::UpdateGrinnichState {
                        player_id: speaker_id,
                        new_state: 3,
                    });
                }
                didsay = true;
            }
            // C `case 3:` (`brannington.c:2513-2518`): the god-only "reset
            // me" wipe, which speaks a visible `say(cn, "reset done")` line
            // first (see the module doc comment).
            TextAnalysisOutcome::Matched(3) => {
                if speaker.flags.contains(CharacterFlags::GOD) {
                    self.npc_say(grinnich_id, "reset done");
                    events.push(GrinnichOutcomeEvent::ResetGrinnich {
                        player_id: speaker_id,
                    });
                }
                didsay = true;
            }
            // Every other matched code (the area-shared `4`/`5` gold/silver
            // trade codes, consumed only by `broklin_driver`, not handled
            // by grinnich's own `switch`) still counts as `didsay`.
            TextAnalysisOutcome::Matched(_) => {
                didsay = true;
            }
            TextAnalysisOutcome::NoMatch => {}
        }

        // C `if (didsay) { talkdir = ...; dat->current_victim = co; }`
        // (`brannington.c:2520-2523`) - note this does *not* touch `dat->
        // last_talk` (except `case 2`'s own explicit reset above).
        if didsay {
            *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));
            data.current_victim = Some(speaker_id);
        }
    }

    /// C `grinnich_driver`'s `NT_GIVE` branch (`brannington.c:2526-2540`):
    /// always hands the item straight back, no turn-in item is ever
    /// accepted by this NPC.
    fn grinnich_handle_give_message(
        &mut self,
        grinnich_id: CharacterId,
        message: &CharacterDriverMessage,
    ) {
        let giver_id = CharacterId(message.dat1.max(0) as u32);
        let Some(item_id) = self
            .characters
            .get_mut(&grinnich_id)
            .and_then(|grinnich| grinnich.cursor_item.take())
        else {
            return;
        };
        self.npc_quiet_say(
            grinnich_id,
            "Thou hast better use for this than I do. Well, if there is use for it at all.",
        );
        if !self.give_char_item(giver_id, item_id) {
            self.destroy_item(item_id);
        }
    }
}

// ---- legacy driver registry surface (moved from character_driver.rs) ----

use crate::character_driver::{CDR_GRINNICH, CDR_LOSTCON};

/// C `struct grinnich_data` (`src/area/29/brannington.c:2397-2400`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GrinnichDriverData {
    #[serde(default)]
    pub last_talk: u64,
    pub current_victim: Option<CharacterId>,
}
