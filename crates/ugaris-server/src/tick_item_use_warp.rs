//! Completed-action-outcome handling: the Warp-area (`src/area/25/
//! warped.c`) family of `ItemDriverOutcome` variants (teleport/bonus-
//! level/key-spawn/key-door/trial-door). Split out of the giant
//! `match outcome { ... }` block that still lives inline in `main.rs`'s
//! `tick.tick()` arm (P0.5 "Finish main() phase decomposition" -
//! REMAINING note: the completed-action-outcome handling needs
//! splitting by completed-action-kind family across several files, not
//! just relocation, because the whole match is too large to move
//! verbatim into one file). This is the first such family slice; the
//! rest of the match (chests, dungeon, teufel, edemon/fdemon, shrines,
//! ...) is still inline in `main.rs` pending further slices.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn dispatch_warp_outcome(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    args: &Args,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    feedback_bytes: &mut Vec<(CharacterId, Vec<u8>)>,
    executed: &mut i32,
    blocked: &mut i32,
    failed: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::WarpTeleportMissingSphere {
            character_id,
            ..
        } => {
            feedback.push((character_id, "Nothing happened.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::WarpTeleportBug { character_id, .. } => {
            feedback.push((character_id, "You found BUG #31as5.".to_string()));
            feedback.push((
                character_id,
                "Target is busy, please try again soon.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::WarpTeleportBusy { character_id, .. } => {
            feedback.push((
                character_id,
                "Target is busy, please try again soon.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::WarpTeleportSpheres {
            character_id, ..
        } => {
            feedback.push((character_id, "Your spheres vanished.".to_string()));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::WarpBonusFinished { character_id, .. } => {
            feedback.push((
                character_id,
                "You're done. Finished. It's over. You're there. You've solved the final level."
                    .to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::WarpBonusAlreadyUsed {
            character_id, ..
        } => {
            feedback.push((character_id, "Nothing happened.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::WarpBonusNeedsSphere {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "Nothing happened. You sense that you'll need one of the spheres this time."
                    .to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::WarpBonus {
            character_id,
            location_id,
            base,
            next_points,
            advanced,
            reward_sphere_kind,
            reward_level,
            ..
        } => {
            if let Some(player) = runtime.player_for_character_mut(character_id) {
                if player.warp_base <= 0 {
                    player.warp_base = 40;
                }
                let slot = player
                    .warp_bonus_ids
                    .iter()
                    .position(|stored| *stored == location_id as i32)
                    .or_else(|| {
                        player
                            .warp_bonus_last_used
                            .iter()
                            .enumerate()
                            .min_by_key(|(_, used)| **used)
                            .map(|(index, _)| index)
                    });
                if let Some(slot) = slot {
                    if slot >= player.warp_bonus_ids.len() {
                        player.warp_bonus_ids.resize(slot + 1, 0);
                    }
                    if slot >= player.warp_bonus_last_used.len() {
                        player.warp_bonus_last_used.resize(slot + 1, 0);
                    }
                    player.warp_bonus_ids[slot] = location_id as i32;
                    player.warp_bonus_last_used[slot] = base as i32;
                }
                player.warp_points = next_points as i32;
                if advanced {
                    player.warp_base = base as i32 + 5;
                    player.warp_nostepexp = 0;
                    if player.warp_base > 139 {
                        feedback
                            .push((character_id, "You've finished the final level.".to_string()));
                    } else if player.warp_base > 134 {
                        feedback
                            .push((character_id, "You've reached the final level.".to_string()));
                    } else {
                        feedback
                            .push((character_id, "You advanced a level! Take care!".to_string()));
                    }
                }
                let current_base = player.warp_base.max(40) as u32;
                let current_points = player.warp_points.max(0) as u32;
                let no_step_exp = player.warp_nostepexp != 0;
                if advanced {
                    // C `warpbonus_driver` (`area/25/warped.c:423-449`)
                    // grants the sphere-kind-1 case's exp via
                    // `give_exp(cn, ...)`, not a raw mutation.
                    match reward_sphere_kind {
                        Some(1) => {
                            world.give_exp(
                                character_id,
                                i64::from(level_value(reward_level) / 7),
                                u32::from(args.area_id),
                            );
                            feedback.push((character_id, "You received experience.".to_string()));
                        }
                        Some(2) => {
                            if let Some(character) = world.characters.get_mut(&character_id) {
                                if character.saves < 10
                                    && !character.flags.contains(CharacterFlags::HARDCORE)
                                {
                                    character.saves += 1;
                                    feedback
                                        .push((character_id, "You received a save.".to_string()));
                                }
                            }
                        }
                        Some(3) => {
                            // C `warpbonus_driver` (`area/25/
                            // warped.c:432-434`): `log_char(cn, ...,
                            // "You received military rank.");
                            // give_military_pts_no_npc(cn, level, 0);`
                            // - the fixed message first, then the
                            // shared point-award/promotion helper
                            // (`World::give_military_pts`, `crates/
                            // ugaris-core/src/world/military.rs`),
                            // which queues its own "You've been
                            // promoted..." feedback (and the above-
                            // Sergeant-Major server broadcast) if the
                            // grant crosses a rank threshold.
                            feedback
                                .push((character_id, "You received military rank.".to_string()));
                            world.give_military_pts(
                                character_id,
                                reward_level as i32,
                                0,
                                u32::from(args.area_id),
                            );
                        }
                        Some(4) => {
                            // C `warpbonus_driver` (`area/25/
                            // warped.c:434-436`): `give_money(cn,
                            // level * level * 10, "Warped area
                            // reward")`.
                            achievement::give_money(
                                world,
                                runtime,
                                achievement_repository,
                                character_id,
                                reward_level.saturating_mul(reward_level).saturating_mul(10),
                                feedback_bytes,
                            )
                            .await;
                        }
                        // Kept nested: `grant_template_item_smart` mutates
                        // world state, so hoisting it into a match guard
                        // would hide the side effect.
                        #[allow(clippy::collapsible_match)]
                        Some(5) => {
                            if grant_template_item_smart(
                                world,
                                zone_loader,
                                character_id,
                                "lollipop",
                            )
                            .is_some()
                            {
                                feedback
                                    .push((character_id, "You received a lollipop.".to_string()));
                            }
                        }
                        _ => {}
                    }
                } else if !no_step_exp {
                    // C `warpbonus_driver` (`area/25/warped.c:453`)
                    // grants the step exp via `give_exp(cn, ...)`.
                    world.give_exp(
                        character_id,
                        i64::from(level_value(reward_level) / 70),
                        u32::from(args.area_id),
                    );
                }
                if current_base <= 139 {
                    feedback.push((
                        character_id,
                        format!(
                            "You are at level {}, and you have {} of {} points.",
                            (current_base - 35) / 5,
                            current_points,
                            current_base / 4
                        ),
                    ));
                }
                *executed += 1;
            } else {
                *failed += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::WarpKeySpawnCursorOccupied {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "Please empty your hand (mouse cursor) first.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::WarpKeySpawn {
            character_id,
            sphere_kind,
            ..
        } => {
            let template = format!("warped_teleport_key{sphere_kind}");
            if grant_template_item_to_cursor(world, zone_loader, character_id, &template).is_some()
            {
                feedback.push((character_id, "You got a glowing half sphere.".to_string()));
                *executed += 1;
            } else {
                feedback.push((character_id, "It won't come off.".to_string()));
                *blocked += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::WarpKeyDoorMissingKey {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "The door is locked and you do not have the right key.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::WarpKeyDoorBug { character_id, .. } => {
            feedback.push((character_id, "Bug #329i, sorry.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::WarpKeyDoor {
            character_id,
            key_name,
            ..
        } => {
            let key_name = outcome_item_name_text(&key_name);
            feedback.push((character_id, format!("A {key_name} vanished.")));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::WarpTrialDoorWrongSide {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "You cannot open the door from this side.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::WarpTrialDoorBusy { character_id, .. } => {
            feedback.push((
                character_id,
                "You hear fighting noises and the door won't open.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::WarpTrialDoorBug { character_id, .. } => {
            feedback.push((character_id, "Bug #319i, sorry.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::WarpTrialDoor {
            character_id,
            spawn_x,
            spawn_y,
            player_x,
            player_y,
            fighter_target_x,
            fighter_target_y,
            xs,
            ys,
            xe,
            ye,
            template,
            ..
        } => {
            // C `warptrialdoor_driver` (`warped.c:764-813`): the player's
            // own `ppd->base` (defaulting to 40, same as `warpbonus_
            // driver`) scales the spawned fighter's skills.
            let base = runtime
                .player_for_character(character_id)
                .map(|player| {
                    if player.warp_base > 0 {
                        player.warp_base
                    } else {
                        40
                    }
                })
                .unwrap_or(40);
            let owner_serial = world
                .characters
                .get(&character_id)
                .map(|character| character.serial)
                .unwrap_or_default();
            if spawn_warp_trial_fighter(
                world,
                zone_loader,
                runtime,
                template,
                spawn_x,
                spawn_y,
                base,
                character_id,
                owner_serial,
                fighter_target_x,
                fighter_target_y,
                xs,
                xe,
                ys,
                ye,
            ) {
                // C `teleport_char_driver(cn, it[in].x + dx, it[in].y +
                // dy);` (`warped.c:813`): only teleport the player through
                // the door once the fighter is fully set up.
                world.teleport_char_driver(character_id, player_x, player_y);
                *executed += 1;
            } else {
                feedback.push((character_id, "Bug #319i, sorry.".to_string()));
                *failed += 1;
            }
        }
        _ => {}
    }
}
