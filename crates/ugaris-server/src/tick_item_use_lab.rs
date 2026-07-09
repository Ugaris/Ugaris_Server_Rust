//! Completed-action-outcome handling: the labyrinth family of
//! `ItemDriverOutcome` variants (`src/area/22/lab*.c`'s Brannington
//! underwater berry, Lab3 yellow/white/brown berries and the white-berry
//! light timer, Lab2 water well/altar/drink/cursor, Lab2 step-action
//! clear/daemon-check/daemon-warning spawn, Lab2 grave clue-book/close/
//! check-open/open, the shared lab-entrance solved-all/too-low and
//! lab-exit wrong-owner blocks, and the shared `labexit` reward loop
//! itself, `LabExitUse` - C `set_solved_lab` (`src/system/lab.c:114-135`)
//! plus `labexit`'s trailing `change_area(cn, 3, 183, 199)`
//! (`src/module/base.c:4749-4778`), common to every one of the five
//! `lab*.c` files' `create_lab_exit` reward drops). Split out of the
//! giant `match outcome { ... }` block that still lives inline in
//! `main.rs`'s `tick.tick()` arm (P0.5 "Finish main() phase
//! decomposition" - REMAINING note: the completed-action-outcome
//! handling needs splitting by completed-action-kind family across
//! several files, not just relocation, because the whole match is too
//! large to move verbatim into one file). Warp, chests, dungeon, ice/
//! palace, Teufel, skel-raise, Edemon/Fdemon, transport, clan/LQ/arena,
//! shrines, burndown, xmas/swamp, Caligar, and key-assembly were sliced
//! first; this is the fifteenth family slice. The rest of the match (the
//! large no-op catch-all, ...) is still inline in `main.rs` pending
//! further slices.
//!
//! `LabExitAnimating`/`LabExitExpired` need no handling here: their
//! mutations (rescheduling the sprite-cycle timer / destroying the
//! expired gate) already happen in `ugaris-core`'s
//! `World::apply_item_driver_outcome` before this dispatcher ever sees
//! them, so they stay in `tick_item_use_completion.rs`'s generic
//! executed-only bucket. `LabExitUse` is different: C's `set_solved_lab`
//! needs the DB-backed `PlayerRuntime::lab_solved_bits`/`give_exp`
//! (`World` alone can't see `PlayerRuntime`), and `change_area` needs
//! the cross-area-transfer machinery - both only reachable from
//! `ugaris-server`, hence this dispatcher and not `apply_item_driver_
//! outcome`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn dispatch_lab_outcome(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
    config: &ServerConfig,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
    failed: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::BranningtonUnderwaterBerry {
            installed,
            ..
        } => {
            if installed {
                *executed += 1;
            } else {
                *blocked += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab3YellowBerry {
            character_id,
            installed,
            ..
        } => {
            if installed {
                *executed += 1;
            } else {
                feedback.push((
                    character_id,
                    "Due to some strange reasons thou canst not eat those berries now.".to_string(),
                ));
                *blocked += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab3WhiteBerry {
            character_id,
            installed,
            ..
        } => {
            if installed {
                *executed += 1;
            } else {
                feedback.push((
                    character_id,
                    "Due to some strange reasons thou canst not eat those berries now.".to_string(),
                ));
                *blocked += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab3WhiteBerryLightTick { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab3BrownBerry {
            character_id,
            installed,
            ..
        } => {
            if installed {
                *executed += 1;
            } else {
                feedback.push((
                    character_id,
                    "Thou art still chewing a brown berry.".to_string(),
                ));
                *blocked += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab2WaterWell { character_id, .. } => {
            if let Some(item_name) =
                grant_template_item_to_cursor(world, zone_loader, character_id, "lab2_waterbowl")
            {
                feedback.push((character_id, format!("You received a {item_name}.")));
                *executed += 1;
            } else {
                *failed += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab2WaterAltar { character_id, .. } => {
            match apply_lab2_water_altar(world, zone_loader, character_id) {
                Lab2WaterApplyResult::Converted(0) => {
                    feedback.push((
                        character_id,
                        "You feel the holyness of the Altar. Water would be holy now, if you had some."
                            .to_string(),
                    ));
                    *blocked += 1;
                }
                Lab2WaterApplyResult::Converted(1) => {
                    feedback.push((
                        character_id,
                        "The water inside your bowl is holy now.".to_string(),
                    ));
                    *executed += 1;
                }
                Lab2WaterApplyResult::Converted(count) => {
                    feedback.push((
                        character_id,
                        format!("The water inside your {count} bowls is holy now."),
                    ));
                    *executed += 1;
                }
                Lab2WaterApplyResult::MissingPlayer | Lab2WaterApplyResult::TemplateMissing => {
                    *failed += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab2WaterDrink { character_id, .. } => {
            feedback.push((character_id, "Skoll!".to_string()));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab2WaterCursorOccupied {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "You won't throw this into the water, will you?".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab2StepActionClear { .. }
        | ugaris_core::item_driver::ItemDriverOutcome::Lab2StepActionDaemonCheck { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab2StepActionDaemonWarning {
            x, y, ..
        } => {
            let character_id = runtime.allocate_character_id();
            match zone_loader.instantiate_character_template("lab2_daemon", character_id) {
                Ok((daemon, inventory_items)) => {
                    if world.spawn_character(daemon, usize::from(x), usize::from(y)) {
                        for item in inventory_items {
                            world.items.insert(item.id, item);
                        }
                        *executed += 1;
                    } else {
                        *failed += 1;
                    }
                }
                _ => {
                    *failed += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab2GraveClueBook {
            character_id,
            book,
            ..
        } => {
            let text = lab2_grave_clue_text(runtime, character_id, book);
            feedback.push((character_id, text));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab2GraveClose { .. }
        | ugaris_core::item_driver::ItemDriverOutcome::Lab2GraveCheckOpen { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab2GraveOpen {
            item_id,
            character_id,
            fixed_item,
        } => {
            if apply_lab2_grave_open(
                world,
                runtime,
                zone_loader,
                item_id,
                character_id,
                fixed_item,
            ) {
                *executed += 1;
            } else {
                *failed += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::LabEntranceSolvedAll {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "You have solved all existing labyrinths already. You can now fight the gatekeeper."
                    .to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::LabEntranceTooLow {
            character_id,
            required_level,
            ..
        } => {
            feedback.push((
                character_id,
                format!("You may not enter before reaching level {required_level}."),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::LabExitWrongOwner { character_id, .. } => {
            feedback.push((
                character_id,
                "This gate has not been created for you. You cannot use it.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::LabExitUse {
            character_id,
            lab_nr,
            target_area,
            target_x,
            target_y,
            ..
        } => {
            // C `set_solved_lab(cn, it[in].drdata[4])`
            // (`src/system/lab.c:114-135`): only the *first* use of a
            // given `lab_nr`'s gate awards exp/the congratulations
            // message - `ppd->solved_bits` gates both.
            let bit = 1u64 << (lab_nr & 63);
            let already_solved = runtime
                .player_for_character(character_id)
                .is_some_and(|player| player.lab_solved_bits & bit != 0);
            if !already_solved {
                if let Some(player) = runtime.player_for_character_mut(character_id) {
                    player.lab_solved_bits |= bit;
                }
                world.give_exp(
                    character_id,
                    i64::from(level_value(u32::from(lab_nr))) / 5,
                    u32::from(config.area_id),
                );
                if let Some(character) = world.characters.get(&character_id) {
                    let name = character.name.clone();
                    feedback.push((
                        character_id,
                        format!(
                            "Congratulations, {name}, you have solved this part of the labyrinth."
                        ),
                    ));
                }
            }

            // C `labexit`'s trailing `change_area(cn, 3, 183, 199)`
            // (`src/module/base.c:4776-4778`); the gate's own
            // `drdata[8]` close-frame write already happened inside
            // `labexit_driver`.
            let transferred = if target_area == config.area_id {
                world.teleport_character_same_area(character_id, target_x, target_y, false)
            } else {
                attempt_cross_area_transfer(
                    world,
                    runtime,
                    character_repository,
                    area_repository,
                    config.area_id,
                    config.mirror_id,
                    character_id,
                    target_area,
                    u32::from(config.mirror_id),
                    target_x,
                    target_y,
                )
                .await
            };
            if transferred {
                *executed += 1;
            } else {
                feedback.push((
                    character_id,
                    "Sorry, Aston is down. Please try again soon.".to_string(),
                ));
                *blocked += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::DeathfibrinShrineGive {
            character_id, ..
        } => {
            if let Some(item_name) =
                grant_template_item_to_cursor(world, zone_loader, character_id, "deathfibrin")
            {
                feedback.push((character_id, format!("You received a {item_name}.")));
                *executed += 1;
            } else {
                *failed += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::DeathfibrinShrineOccupied { character_id } => {
            feedback.push((
                character_id,
                "The Shrine of Deathfibrin seems to ignore everything. It may want to give you something."
                    .to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::DeathfibrinNeedsCarry { character_id } => {
            feedback.push((
                character_id,
                "You need to carry this to use it.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::DeathfibrinNoMaster {
            character_id,
            tile_light,
        } => {
            feedback.push((
                character_id,
                format!("Nothing happens. There is no immortal close enough. {tile_light}"),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::DeathfibrinStrike {
            character_id,
            item_name,
            vanished,
            ..
        } => {
            if vanished {
                let item_name = String::from_utf8_lossy(&item_name)
                    .trim_end_matches('\0')
                    .to_string();
                feedback.push((character_id, format!("Your {item_name} vanished.")));
            }
            *executed += 1;
        }
        _ => {}
    }
}
