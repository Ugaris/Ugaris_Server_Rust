//! Completed-action-outcome handling: the chest-family `ItemDriverOutcome`
//! variants (treasure chests, random/rat chests, infinite chests, forest
//! chests, pick-lockable chests, and chest-spawner triggers). Split out
//! of the giant `match outcome { ... }` block that still lives inline in
//! `main.rs`'s `tick.tick()` arm (P0.5 "Finish main() phase
//! decomposition" - REMAINING note: the completed-action-outcome
//! handling needs splitting by completed-action-kind family across
//! several files, not just relocation, because the whole match is too
//! large to move verbatim into one file). This is the second such family
//! slice (`tick_item_use_warp` was the first); the rest of the match
//! (dungeon, teufel, skel-raise, transport, clan-spawn, lq, arena,
//! shrines, xmas, swamp, edemon/fdemon, burndown, palace doors,
//! key-assembly, and the no-op catch-all) is still inline in `main.rs`
//! pending further slices.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn dispatch_chest_outcome(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    config: &ServerConfig,
    realtime_seconds: u64,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
    failed: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::ChestTreasure {
            item_id,
            character_id,
            treasure_index,
        } => {
            match apply_chest_treasure(
                world,
                zone_loader,
                runtime.player_for_character_mut(character_id),
                item_id,
                character_id,
                treasure_index,
                realtime_seconds,
            ) {
                ChestTreasureApplyResult::Granted {
                    item_name,
                    key_name,
                } => {
                    if let Some(key_name) = key_name {
                        feedback.push((
                            character_id,
                            format!("You use {key_name} to unlock the chest."),
                        ));
                    }
                    feedback.push((character_id, format!("You got a {item_name}.")));
                    *executed += 1;
                    award_chest_opened_achievement(
                        world,
                        runtime,
                        achievement_repository,
                        character_id,
                        Some(treasure_index),
                    )
                    .await;
                }
                ChestTreasureApplyResult::Empty => {
                    feedback.push((character_id, CHEST_EMPTY_MESSAGE.to_string()));
                    *blocked += 1;
                }
                ChestTreasureApplyResult::KeyRequired => {
                    feedback.push((character_id, CHEST_KEY_REQUIRED_MESSAGE.to_string()));
                    *blocked += 1;
                }
                ChestTreasureApplyResult::CursorOccupied => {
                    feedback.push((character_id, CHEST_CURSOR_OCCUPIED_MESSAGE.to_string()));
                    *blocked += 1;
                }
                ChestTreasureApplyResult::MissingPlayer => {
                    *failed += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::RandomChest {
            item_id,
            character_id,
        } => {
            let random_seed =
                world.tick.0 ^ (u64::from(item_id.0) << 16) ^ u64::from(character_id.0);
            match apply_random_chest(
                world,
                zone_loader,
                runtime.player_for_character_mut(character_id),
                item_id,
                character_id,
                config.area_id,
                realtime_seconds,
                random_seed,
            ) {
                RandomChestApplyResult::Money { amount } => {
                    feedback.push((
                        character_id,
                        format!("You found some money ({:.2}G)!", f64::from(amount) / 100.0),
                    ));
                    *executed += 1;
                    award_chest_opened_achievement(
                        world,
                        runtime,
                        achievement_repository,
                        character_id,
                        None,
                    )
                    .await;
                }
                RandomChestApplyResult::Item { item_name } => {
                    feedback.push((character_id, format!("You found a {item_name}.")));
                    *executed += 1;
                    award_chest_opened_achievement(
                        world,
                        runtime,
                        achievement_repository,
                        character_id,
                        None,
                    )
                    .await;
                }
                RandomChestApplyResult::Empty => {
                    feedback.push((character_id, RANDCHEST_EMPTY_MESSAGE.to_string()));
                    *blocked += 1;
                }
                RandomChestApplyResult::CursorOccupied => {
                    feedback.push((character_id, RANDCHEST_CURSOR_OCCUPIED_MESSAGE.to_string()));
                    *blocked += 1;
                }
                RandomChestApplyResult::MissingPlayer => {
                    *failed += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::RatChest {
            item_id,
            character_id,
        } => {
            let random_seed = world.tick.0
                ^ (u64::from(item_id.0) << 16)
                ^ u64::from(character_id.0)
                ^ 0x5241_5443_4845_5354;
            match apply_rat_chest(
                world,
                zone_loader,
                runtime.player_for_character_mut(character_id),
                item_id,
                character_id,
                config.area_id,
                realtime_seconds,
                random_seed,
            ) {
                RatChestApplyResult::Money { amount } => {
                    feedback.push((
                        character_id,
                        format!("You found some money ({:.2}G)!", f64::from(amount) / 100.0),
                    ));
                    *executed += 1;
                }
                RatChestApplyResult::Treasure { item_name } => {
                    feedback.push((character_id, format!("You found a {item_name}.")));
                    *executed += 1;
                }
                RatChestApplyResult::Empty => {
                    feedback.push((character_id, RANDCHEST_EMPTY_MESSAGE.to_string()));
                    *blocked += 1;
                }
                RatChestApplyResult::CursorOccupied => {
                    feedback.push((character_id, RANDCHEST_CURSOR_OCCUPIED_MESSAGE.to_string()));
                    *blocked += 1;
                }
                RatChestApplyResult::MissingPlayer => {
                    *failed += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::InfiniteChest {
            character_id,
            template,
            key_name,
            ..
        } => {
            match grant_template_item_to_cursor(world, zone_loader, character_id, template.as_str())
            {
                Some(item_name) => {
                    if let Some(key_name) = key_name {
                        let key_name = outcome_item_name_text(&key_name);
                        feedback.push((
                            character_id,
                            format!("You use {key_name} to open the chest."),
                        ));
                    }
                    feedback.push((character_id, format!("You got a {item_name}.")));
                    *executed += 1;
                }
                None => {
                    feedback.push((
                    character_id,
                    "Congratulations, you have just discovered bug #4744C, please report it to the authorities!".to_string(),
                ));
                    *failed += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::InfiniteChestCursorOccupied {
            character_id,
            ..
        } => {
            feedback.push((character_id, CHEST_CURSOR_OCCUPIED_MESSAGE.to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::InfiniteChestKeyRequired {
            character_id,
            ..
        } => {
            feedback.push((character_id, CHEST_KEY_REQUIRED_MESSAGE.to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::InfiniteChestUnknown {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "Congratulations, you have just discovered bug #4744B, please report it to the authorities!".to_string(),
            ));
            *failed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ForestChest {
            character_id,
            amount,
            imp_flag_mask,
            ..
        } => {
            match apply_forest_chest(
                world,
                zone_loader,
                runtime.player_for_character_mut(character_id),
                character_id,
                amount,
                imp_flag_mask,
            ) {
                ForestChestApplyResult::FoundMoney { .. } => {
                    feedback.push((character_id, "You found a nice sum of money!".to_string()));
                    *executed += 1;
                }
                ForestChestApplyResult::Empty => {
                    feedback.push((character_id, "The chest is empty.".to_string()));
                    *blocked += 1;
                }
                ForestChestApplyResult::CursorOccupied => {
                    feedback.push((
                        character_id,
                        "Please empty your hand (mouse cursor) first.".to_string(),
                    ));
                    *blocked += 1;
                }
                ForestChestApplyResult::MissingPlayer => {
                    *failed += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::ForestChestCursorOccupied {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "Please empty your hand (mouse cursor) first.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ForestChestLocked { character_id, .. } => {
            feedback.push((
                character_id,
                "The chest is locked and you don't have the right key.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::PickChest {
            character_id,
            template,
            ..
        } => {
            match grant_template_item_to_cursor(world, zone_loader, character_id, template.as_str())
            {
                Some(item_name) => {
                    world.notify_twocity_pick_from_character(character_id);
                    feedback.push((character_id, "You pick the lock.".to_string()));
                    feedback.push((
                        character_id,
                        format!("You found a {}.", item_name.to_ascii_lowercase()),
                    ));
                    *executed += 1;
                }
                None => {
                    feedback.push((character_id, "You've found bug #8331.".to_string()));
                    *failed += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::PickChestCursorOccupied {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "Please empty your hand (mouse cursor) first.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::PickChestLocked {
            item_id,
            character_id,
        } => {
            let item_name = world
                .items
                .get(&item_id)
                .map(|item| item.name.to_ascii_lowercase())
                .unwrap_or_else(|| "chest".to_string());
            feedback.push((
                character_id,
                format!("The {item_name} is locked and you don't have the right key."),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::PickChestBug { character_id, .. } => {
            feedback.push((character_id, "You've found bug #8331.".to_string()));
            *failed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ChestSpawn {
            item_id,
            character_id: _,
            template,
            x,
            y,
            ..
        } => {
            if spawn_chestspawn_character(world, zone_loader, runtime, item_id, template, x, y) {
                *executed += 1;
            } else {
                *failed += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::ChestSpawnCheck { .. } => {}
        ugaris_core::item_driver::ItemDriverOutcome::MissionChestOpen {
            item_id,
            character_id,
        } => {
            match apply_mission_chest_open(
                world,
                zone_loader,
                runtime.player_for_character_mut(character_id),
                item_id,
                character_id,
            ) {
                MissionChestApplyResult::Granted {
                    item_name,
                    key_name,
                    status_lines,
                    solved_message,
                } => {
                    if let Some(key_name) = key_name {
                        feedback.push((
                            character_id,
                            format!("You use {key_name} to unlock the chest."),
                        ));
                    }
                    for line in status_lines {
                        feedback.push((character_id, line));
                    }
                    feedback.push((character_id, format!("You got a {item_name}.")));
                    if let Some(solved_message) = solved_message {
                        feedback.push((character_id, solved_message));
                    }
                    *executed += 1;
                }
                MissionChestApplyResult::Empty => {
                    feedback.push((character_id, "The chest is empty.".to_string()));
                    *blocked += 1;
                }
                MissionChestApplyResult::KeyRequired => {
                    feedback.push((
                        character_id,
                        "You need a key to open this chest.".to_string(),
                    ));
                    *blocked += 1;
                }
                MissionChestApplyResult::CursorOccupied { key_name } => {
                    if let Some(key_name) = key_name {
                        feedback.push((
                            character_id,
                            format!("You use {key_name} to unlock the chest."),
                        ));
                    }
                    feedback.push((
                        character_id,
                        "Please empty your 'hand' (mouse cursor) first.".to_string(),
                    ));
                    *blocked += 1;
                }
                MissionChestApplyResult::MissingPlayer => {
                    *failed += 1;
                }
            }
        }
        _ => {}
    }
}
