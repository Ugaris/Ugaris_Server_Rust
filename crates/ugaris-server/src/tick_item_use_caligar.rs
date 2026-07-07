//! Completed-action-outcome handling: the Caligar-area (`src/area/36/
//! caligar.c`) family of `ItemDriverOutcome` variants (weight-puzzle
//! blocks/doors, the three-lock skeleton door, the final assembled key,
//! and the observation-lesson training posts). Split out of the giant
//! `match outcome { ... }` block that still lives inline in `main.rs`'s
//! `tick.tick()` arm (P0.5 "Finish main() phase decomposition" -
//! REMAINING note: the completed-action-outcome handling needs splitting
//! by completed-action-kind family across several files, not just
//! relocation, because the whole match is too large to move verbatim
//! into one file). Warp, chests, dungeon, ice/palace, Teufel, skel-
//! raise, Edemon/Fdemon, transport, clan/LQ/arena, shrines, burndown,
//! and xmas/swamp were sliced first; this is the thirteenth family
//! slice. Unlike those, this family's variants were scattered across 4
//! spots in `main.rs`'s match (two standalone arms, one field-guarded
//! variant living inside the shared no-op catch-all's or-pattern, and
//! `CaligarWeightMove`/`Door`/`Timer`/`GunProjectile` living inside a
//! small Staffer+Caligar shared no-op arm) - each spot's Caligar-only
//! lines were removed and replaced (at the first spot) with one combined
//! or-pattern call arm; `CaligarKeyAssemble`'s `final_key: true`/`false`
//! split is preserved as a field-guarded match inside this dispatcher
//! rather than in the outer `main.rs` match, since both guards share the
//! same outer variant name. The rest of the match (key-assembly for
//! other areas, staffer, saltmine, bone-holder, arkhata, lizard-flower,
//! lab2/lab3, ...) is still inline in `main.rs` pending further slices.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_caligar_outcome(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
    failed: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::CaligarWeightBlocked {
            character_id, ..
        } => {
            feedback.push((character_id, "It won't move.".to_string()));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::CaligarWeightDoorLocked {
            character_id,
            ..
        } => {
            feedback.push((character_id, "The door is locked.".to_string()));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::CaligarWeightDoorBusy {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "Please try again soon. Target is busy.".to_string(),
            ));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::CaligarWeightMove { .. }
        | ugaris_core::item_driver::ItemDriverOutcome::CaligarWeightDoor { .. }
        | ugaris_core::item_driver::ItemDriverOutcome::CaligarWeightTimer { .. }
        | ugaris_core::item_driver::ItemDriverOutcome::CaligarGunProjectile { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::CaligarKeyAssemble {
            final_key: false,
            ..
        } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::CaligarKeyAssemble {
            item_id,
            character_id,
            cursor_item_id,
            final_key: true,
            ..
        } => {
            match apply_caligar_key_final(world, zone_loader, item_id, character_id, cursor_item_id)
            {
                AssembleApplyResult::Assembled => {
                    *executed += 1;
                }
                AssembleApplyResult::TemplateUnavailable => {
                    feedback.push((character_id, "This does not seem to fit.".to_string()));
                    *blocked += 1;
                }
                AssembleApplyResult::MissingPlayer | AssembleApplyResult::MissingItem => {
                    *failed += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::CaligarKeyNeedsCursor {
            character_id, ..
        } => {
            feedback.push((character_id, "Nothing happens.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::CaligarKeyDoesNotFit {
            character_id, ..
        } => {
            feedback.push((character_id, "This does not seem to fit.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::CaligarSkellyDoor {
            item_id,
            character_id,
            door_index,
        } => {
            if runtime
                .player_for_character(character_id)
                .is_some_and(|player| player.caligar_skelly_door_unlocked(door_index))
            {
                match world.apply_caligar_skelly_door(item_id, character_id, door_index) {
                    ugaris_core::item_driver::ItemDriverOutcome::CaligarSkellyDoor { .. } => {
                        *executed += 1;
                    }
                    ugaris_core::item_driver::ItemDriverOutcome::CaligarSkellyDoorBusy {
                        character_id,
                        ..
                    } => {
                        feedback.push((
                            character_id,
                            "Please try again soon. Target is busy.".to_string(),
                        ));
                        *blocked += 1;
                    }
                    _ => {
                        *failed += 1;
                    }
                }
            } else {
                feedback.push((character_id, "The door appears to be locked by some strange mechanism. It seems you need to open three seperate locks.".to_string()));
                *blocked += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::CaligarSkellyDoorLocked {
            character_id,
            ..
        } => {
            feedback.push((character_id, "The door appears to be locked by some strange mechanism. It seems you need to open three seperate locks.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::CaligarSkellyDoorBusy {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "Please try again soon. Target is busy.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::CaligarTraining {
            character_id,
            lesson,
            ..
        } => {
            if let Some(player) = runtime.player_for_character_mut(character_id) {
                if player.observe_caligar_training(lesson).unwrap_or(false) {
                    let text = match lesson {
                        1 => "You observe the skeletons fighting techniques: Melee.",
                        2 => "You observe the vampires fighting techniques: Magic and Melee.",
                        3 => "You observe the zombies fighting techniques: Magic.",
                        _ => "",
                    };
                    if !text.is_empty() {
                        feedback.push((character_id, text.to_string()));
                    }
                }
                *executed += 1;
            } else {
                *failed += 1;
            }
        }
        _ => {}
    }
}
