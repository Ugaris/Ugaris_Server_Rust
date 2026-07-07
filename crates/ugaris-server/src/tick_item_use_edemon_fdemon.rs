//! Completed-action-outcome handling: the Earth Demon (`src/area/6/
//! edemon.c`) and Fire Demon (`src/area/8/fdemon.c`) boss-machinery
//! families of `ItemDriverOutcome` variants (switch/door/block/tube
//! puzzle feedback, cannon-loader crystal checks, farm-harvest crystal
//! rewards, blood/lava container fills, and the shared door-toggle key
//! message). Split out of the giant `match outcome { ... }` block that
//! still lives inline in `main.rs`'s `tick.tick()` arm (P0.5 "Finish
//! main() phase decomposition" - REMAINING note: the completed-action-
//! outcome handling needs splitting by completed-action-kind family
//! across several files, not just relocation, because the whole match
//! is too large to move verbatim into one file). Warp, chests,
//! dungeon, ice/palace, Teufel, and skel-raise were sliced first; this
//! is the seventh family slice. The rest of the match (transport,
//! clan-spawn, lq, arena, shrines, xmas, swamp, burndown, key-
//! assembly, ...) is still inline in `main.rs` pending further slices.
//!
//! `EdemonDoorToggle` sits non-contiguously (right after the unrelated
//! `PotionDrunk` variant in the C-derived match order); it is combined
//! into this family's or-pattern at its first (Edemon/Fdemon-family)
//! call site in `main.rs`, and its original second-occurrence arm is
//! deleted, matching the precedent set by the chest family's non-
//! contiguous slice.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_edemon_fdemon_outcome(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::EdemonSwitchStuck { character_id, .. } => {
            feedback.push((character_id, "The lever seems stuck.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::EdemonDoorLocked { character_id, .. } => {
            feedback.push((character_id, "You need a key to use this door.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::EdemonDoorLifeless {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "The door won't move. It seems somehow lifeless.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::EdemonBlockBlocked {
            character_id, ..
        } => {
            feedback.push((character_id, "It won't move.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::EdemonBlockMove { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::EdemonTubePulse { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::FdemonLoaderBlocked {
            character_id,
            reason,
            ..
        } => {
            let text = match reason {
                ugaris_core::item_driver::FdemonLoaderBlockReason::CrystalAlreadyPresent => {
                    "There is already a crystal, you cannot add another item."
                }
                ugaris_core::item_driver::FdemonLoaderBlockReason::CrystalStuck => {
                    "The crystal is stuck."
                }
                ugaris_core::item_driver::FdemonLoaderBlockReason::NeedsCrystal => {
                    "Nothing happens."
                }
                ugaris_core::item_driver::FdemonLoaderBlockReason::WrongCrystal => {
                    "That doesn't fit."
                }
            };
            feedback.push((character_id, text.to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::FdemonCannonLifeless {
            character_id, ..
        } => {
            feedback.push((character_id, "It seems lifeless.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::EdemonLoaderBlocked {
            character_id,
            reason,
            ..
        } => {
            let text = match reason {
                ugaris_core::item_driver::EdemonLoaderBlockReason::CrystalAlreadyPresent => {
                    "There is already a crystal, you cannot add another item."
                }
                ugaris_core::item_driver::EdemonLoaderBlockReason::CrystalStuck => {
                    "The crystal is stuck."
                }
                ugaris_core::item_driver::EdemonLoaderBlockReason::NeedsCrystal => {
                    "Nothing happens."
                }
                ugaris_core::item_driver::EdemonLoaderBlockReason::WrongCrystal => {
                    "That doesn't fit."
                }
            };
            feedback.push((character_id, text.to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::FdemonFarmHarvest {
            character_id,
            template,
            ..
        } => {
            if grant_template_item_to_cursor(world, zone_loader, character_id, template.as_str())
                .is_some()
            {
                *executed += 1;
            } else {
                feedback.push((
                    character_id,
                    format!("BUG # 31992 mark {}", template.legacy_number()),
                ));
                *blocked += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::FdemonFarmCursorOccupied {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "Please empty your hand (mouse cursor) first.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::FdemonFarmNotReady {
            character_id,
            current,
            required,
            ..
        } => {
            feedback.push((
                character_id,
                format!("There's nothing to take yet ({} of {}).", current, required),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::FdemonFarmBug {
            character_id,
            crystal_number,
            ..
        } => {
            feedback.push((character_id, format!("BUG # 31992 mark {}", crystal_number)));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::FdemonBloodBlocked {
            character_id,
            reason,
            ..
        } => {
            let text = match reason {
                ugaris_core::item_driver::FdemonBloodBlockReason::BareHands => {
                    "You do not want to touch the liquid with your bare hands."
                }
                ugaris_core::item_driver::FdemonBloodBlockReason::WrongItem => "Hu?",
                ugaris_core::item_driver::FdemonBloodBlockReason::ContainerFull => {
                    "The container is full already."
                }
            };
            feedback.push((character_id, text.to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::FdemonBloodDestroyedFlask {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "The liquid burns through the flask and shatters it.".to_string(),
            ));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::FdemonBloodFilled { character_id, .. } => {
            if let Some(player) = runtime.player_for_character_mut(character_id) {
                if player.advance_farmy_blood_stage() {
                    feedback.push((
                        character_id,
                        "That's it. Now report to the commander.".to_string(),
                    ));
                }
            }
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::FdemonLavaBlocked {
            character_id,
            reason,
            ..
        } => {
            let text = match reason {
                ugaris_core::item_driver::FdemonLavaBlockReason::BareHands => {
                    "You do not want to touch burning lava with your bare hands, do you?"
                }
                ugaris_core::item_driver::FdemonLavaBlockReason::WrongItem => "Hu?",
                ugaris_core::item_driver::FdemonLavaBlockReason::EmptyContainer => {
                    "The container is empty, and it cannot hold lava."
                }
            };
            feedback.push((character_id, text.to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::FdemonLavaActivated {
            character_id, ..
        } => {
            if let Some(player) = runtime.player_for_character_mut(character_id) {
                if player.advance_farmy_lava_stage() {
                    feedback.push((
                        character_id,
                        "You got it. Now report to the commander.".to_string(),
                    ));
                }
            }
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::EdemonDoorToggle {
            character_id,
            key_name: Some(key_name),
            locking,
            ..
        } => {
            let action = if locking { "lock" } else { "unlock" };
            let key_name = outcome_item_name_text(&key_name);
            feedback.push((
                character_id,
                format!("You use {key_name} to {action} the door."),
            ));
            *executed += 1;
        }
        // C: `EdemonDoorToggle` without a key name falls through to the
        // same no-op-with-count-only handling as `FoodEaten`/`DoorToggle`/
        // ... in main.rs's large no-op catch-all (this variant was
        // originally listed twice: once with the `key_name: Some(..)`
        // guard here, once bare in that catch-all further down the same
        // match; both are preserved here since Rust match arm order still
        // holds this specific-before-general precedence).
        ugaris_core::item_driver::ItemDriverOutcome::EdemonDoorToggle { .. } => {
            *executed += 1;
        }
        _ => {}
    }
}
