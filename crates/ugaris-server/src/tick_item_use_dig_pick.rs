//! Completed-action-outcome handling: three small treasure-hunting/
//! lockpicking families that sit contiguously in the giant `match outcome
//! { ... }` block that still lives inline in `main.rs`'s `tick.tick()` arm
//! (P0.5 "Finish main() phase decomposition" - REMAINING note: the
//! completed-action-outcome handling needs splitting by completed-action-
//! kind family across several files, not just relocation, because the
//! whole match is too large to move verbatim into one file). Warp, chests,
//! dungeon, ice/palace, Teufel, skel-raise, edemon/fdemon, transport,
//! clan/lq/arena, shrines, burndown, xmas/swamp, Caligar, key-assembly,
//! labyrinth, and mine-wall were sliced first; this is the seventeenth
//! family slice. It covers:
//! - Area 16/1/29 forest spade digging (`src/area/16/forest.c`): buried
//!   note/treasure finds, collapse traps, empty holes, occupied cursor.
//! - Area 14 junkpile searching (`src/area/14/random.c`): found/nothing/
//!   occupied cursor.
//! - Area 17 Two-City pick-door lockpicking (`src/area/17/two.c`): toggle
//!   (with pick-the-lock flavor text) and locked-with-wrong-key.
//!   The rest of the match (the remaining no-op catch-all) is still inline
//!   in `main.rs` pending further slices.

use super::*;
use crate::area_apply::{
    apply_forest_spade_find, apply_junkpile_search, ForestSpadeApplyResult, JunkpileApplyResult,
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_dig_pick_outcome(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    realtime_seconds: u64,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
    failed: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::ForestSpadeFind {
            item_id,
            character_id,
            find,
        } => {
            let random_seed =
                world.tick.0 ^ (u64::from(item_id.0) << 16) ^ u64::from(character_id.0);
            match apply_forest_spade_find(
                world,
                zone_loader,
                runtime.player_for_character_mut(character_id),
                character_id,
                find,
                realtime_seconds,
                random_seed,
            ) {
                ForestSpadeApplyResult::Found { item_name } => {
                    feedback.push((character_id, format!("You found a {item_name}.")));
                    *executed += 1;
                }
                ForestSpadeApplyResult::FoundMoney { amount } => {
                    feedback.push((
                        character_id,
                        format!("You found a Money ({:.2}G).", f64::from(amount) / 100.0),
                    ));
                    *executed += 1;
                }
                ForestSpadeApplyResult::AlreadyDug => {
                    feedback.push((
                        character_id,
                        "You've already dug here. The treasure hasn't regrown yet.".to_string(),
                    ));
                    *blocked += 1;
                }
                ForestSpadeApplyResult::Nothing => {
                    feedback.push((
                        character_id,
                        "You dug a nice deep hole but you didn't find anything. Embarrassed you stop digging and fill the hole again.".to_string(),
                    ));
                    *blocked += 1;
                }
                ForestSpadeApplyResult::CursorOccupied => {
                    feedback.push((
                        character_id,
                        "Please empty your hand (mouse cursor) first.".to_string(),
                    ));
                    *blocked += 1;
                }
                ForestSpadeApplyResult::MissingPlayer => {
                    *failed += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::ForestSpadeCollapse {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "The floor collapses below your feet and you fall...".to_string(),
            ));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ForestSpadeNothing {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "You dug a nice deep hole but you didn't find anything. Embarrassed you stop digging and fill the hole again.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ForestSpadeCursorOccupied {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "Please empty your hand (mouse cursor) first.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::JunkpileSearch {
            item_id,
            character_id,
            level,
        } => {
            let random_seed =
                world.tick.0 ^ (u64::from(item_id.0) << 16) ^ u64::from(character_id.0);
            match apply_junkpile_search(
                world,
                zone_loader,
                item_id,
                character_id,
                level,
                random_seed,
            ) {
                JunkpileApplyResult::Found { .. } | JunkpileApplyResult::FoundMoney { .. } => {
                    feedback.push((
                        character_id,
                        "You found something between all that junk.".to_string(),
                    ));
                    *executed += 1;
                }
                JunkpileApplyResult::Nothing => {
                    *executed += 1;
                }
                JunkpileApplyResult::CursorOccupied => {
                    feedback.push((
                        character_id,
                        "Please empty your hand (mouse cursor) first.".to_string(),
                    ));
                    *blocked += 1;
                }
                JunkpileApplyResult::MissingPlayer => {
                    *failed += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::JunkpileCursorOccupied {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "Please empty your hand (mouse cursor) first.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::PickDoorToggle {
            character_id,
            picked_lock,
            ..
        } => {
            if picked_lock {
                feedback.push((character_id, "You pick the lock.".to_string()));
            }
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::PickDoorLocked { character_id, .. } => {
            feedback.push((
                character_id,
                "The door is locked and you don't have the right key.".to_string(),
            ));
            *blocked += 1;
        }
        _ => {}
    }
}
