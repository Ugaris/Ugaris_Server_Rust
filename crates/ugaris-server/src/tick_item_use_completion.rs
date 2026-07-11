//! Completed-action-outcome handling phase: the part of the legacy tick
//! loop that runs immediately after `World::tick_basic_actions_with_attack_
//! policy` returns. Handles auto-keyring pickup (`keyring_try_auto_add`),
//! dispatches every completed `item_use` request through
//! `World::use_item_request` and the per-family `tick_item_use_*`
//! dispatchers (chests/dungeon/ice/Teufel/skel-raise/Edemon-Fdemon/
//! transport/clan-LQ-arena/shrines/burndown/xmas-swamp/Caligar/key-
//! assembly/labyrinth/mine-wall/forest-spade-junkpile-pick-door/lollipop-
//! potions-books/keyring-assemble-crafting), then flushes item-use
//! feedback/container-refresh packets, refreshes each session's map/
//! inventory/effects view for every completed action, and drains queued
//! sound specials. Extracted verbatim from `main()`'s `tick.tick()` arm
//! (P0.5 "Finish main() phase decomposition"): this was the last piece of
//! the "huge completed-action-outcome handling block" the task's REMAINING
//! note called out - the giant `match outcome { ... }` itself was already
//! sliced into one file per outcome family (see the `tick_item_use_*`
//! modules); this file is the surrounding scaffolding (keyring auto-add,
//! the item-use dispatch loop, and the post-completion map/sound sync)
//! that called those family dispatchers and could not itself be split by
//! outcome family. `main.rs` must not grow when this phase changes.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn process_completed_action_outcomes(
    mut world: &mut World,
    mut runtime: &mut ServerRuntime,
    mut zone_loader: &mut ZoneLoader,
    config: &ServerConfig,
    args: &Args,
    completed_actions: &mut Vec<WorldActionCompletion>,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
) {
    if !completed_actions.is_empty() {
        info!(
            count = completed_actions.len(),
            tick = world.tick.0,
            "completed world actions"
        );
        let mut auto_keyring_feedback = Vec::new();
        let mut auto_keyring_added = 0;
        let mut auto_keyring_kept = 0;
        let mut auto_keyring_failed = 0;
        for completion in completed_actions.iter() {
            if !completion.ok || completion.action_id != ugaris_core::legacy::action::TAKE {
                continue;
            }
            let Some(item_id) = completion.action_item_id else {
                continue;
            };
            let keyring_result = apply_keyring_auto_add_pickup(
                &mut world,
                runtime.player_for_character_mut(completion.character_id),
                completion.character_id,
                item_id,
            );
            // C `act_take` (`act.c:305-327`): the stone-pickup
            // achievement check only runs when
            // `keyring_try_auto_add` did NOT consume the item
            // (that branch `free_item`s it and `return`s early
            // in C before reaching this check).
            let stone_check_allowed = !matches!(
                keyring_result,
                Some(KeyringAutoAddPickupResult::Added { .. })
            );
            match keyring_result {
                Some(KeyringAutoAddPickupResult::Added { key_name }) => {
                    auto_keyring_feedback.push((
                        completion.character_id,
                        format!("{key_name} added to keyring."),
                    ));
                    auto_keyring_added += 1;
                }
                Some(KeyringAutoAddPickupResult::Duplicate { key_name }) => {
                    auto_keyring_feedback.push((
                        completion.character_id,
                        format!("{key_name} already in keyring, added to inventory."),
                    ));
                    auto_keyring_kept += 1;
                }
                Some(KeyringAutoAddPickupResult::Full { key_name }) => {
                    auto_keyring_feedback.push((
                        completion.character_id,
                        format!("Keyring full, {key_name} added to inventory."),
                    ));
                    auto_keyring_kept += 1;
                }
                Some(
                    KeyringAutoAddPickupResult::MissingPlayer
                    | KeyringAutoAddPickupResult::MissingCursorItem,
                ) => {
                    auto_keyring_failed += 1;
                }
                None => {}
            }
            if stone_check_allowed {
                if let Some(item) = world.items.get(&item_id) {
                    if item.template_id == ugaris_core::item_driver::IID_ALCHEMY_INGREDIENT {
                        let stone_drdata = item.driver_data.first().copied().unwrap_or_default();
                        award_stone_pickup_achievement(
                            &mut world,
                            &mut runtime,
                            &achievement_repository,
                            completion.character_id,
                            stone_drdata,
                        )
                        .await;
                    }
                }
            }
        }
        if !auto_keyring_feedback.is_empty() {
            let mut feedback_sessions = 0;
            for (character_id, message) in auto_keyring_feedback {
                let payload = ugaris_protocol::packet::system_text(&message);
                for (session_id, _) in runtime.sessions_for_character(character_id) {
                    if runtime.send_to_session(session_id, payload.clone()) {
                        feedback_sessions += 1;
                    }
                }
            }
            info!(
                auto_keyring_added,
                auto_keyring_kept,
                auto_keyring_failed,
                feedback_sessions,
                tick = world.tick.0,
                "processed keyring pickup auto-add"
            );
        }
        let item_use_requests: Vec<_> = completed_actions
            .iter()
            .enumerate()
            .filter_map(|(index, completion)| completion.item_use.map(|request| (index, request)))
            .collect();
        if !item_use_requests.is_empty() {
            let mut opened = 0;
            let mut executed = 0;
            let mut unsupported = 0;
            let mut deferred_templates = 0;
            let mut blocked = 0;
            let mut failed = 0;
            let realtime_seconds = world.tick.0 / TICKS_PER_SECOND;
            let mut feedback = Vec::new();
            let mut feedback_bytes = Vec::new();
            let mut special_feedback = Vec::new();
            let mut area_feedback = Vec::new();
            let mut container_refresh = Vec::new();
            for (completion_index, request) in item_use_requests {
                let use_character_id = request.character_id;
                match world.use_item_request(request, true) {
                    Ok(ugaris_core::item_driver::UseItemOutcome::OpenContainer { .. })
                    | Ok(ugaris_core::item_driver::UseItemOutcome::OpenDepot { .. }) => {
                        if let Some(completion) = completed_actions.get_mut(completion_index) {
                            completion.legacy_return_code = 1;
                        }
                        container_refresh.push(use_character_id);
                        opened += 1;
                    }
                    Ok(ugaris_core::item_driver::UseItemOutcome::OpenAccountDepot { .. }) => {
                        if let Some(completion) = completed_actions.get_mut(completion_index) {
                            completion.legacy_return_code = 1;
                        }
                        runtime.ensure_account_depot(use_character_id);
                        container_refresh.push(use_character_id);
                        opened += 1;
                    }
                    Ok(ugaris_core::item_driver::UseItemOutcome::Dispatch(request)) => {
                        let driver = match request {
                            ugaris_core::item_driver::ItemDriverRequest::Driver {
                                driver, ..
                            } => Some(driver),
                            ugaris_core::item_driver::ItemDriverRequest::AccountDepot {
                                ..
                            } => None,
                        };
                        let is_chest_request = matches!(
                            request,
                            ugaris_core::item_driver::ItemDriverRequest::Driver {
                                driver: ugaris_core::item_driver::IDR_CHEST,
                                ..
                            }
                        );
                        let request_character_id = match request {
                            ugaris_core::item_driver::ItemDriverRequest::Driver {
                                character_id,
                                ..
                            }
                            | ugaris_core::item_driver::ItemDriverRequest::AccountDepot {
                                character_id,
                                ..
                            } => character_id,
                        };
                        let driver_context = item_driver_context_for_request(
                            &world,
                            runtime.player_for_character(request_character_id),
                            &request,
                        );
                        let outcome = world.execute_item_driver_request_with_context(
                            request,
                            config.area_id,
                            &driver_context,
                        );
                        if let Some(completion) = completed_actions.get_mut(completion_index) {
                            completion.legacy_return_code =
                                ugaris_core::item_driver::legacy_item_driver_return_code(
                                    driver, &outcome,
                                );
                        }
                        match outcome {
                            outcome @ (ugaris_core::item_driver::ItemDriverOutcome::ChestTreasure { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::RandomChest { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::RatChest { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::InfiniteChest { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::InfiniteChestCursorOccupied { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::InfiniteChestKeyRequired { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::InfiniteChestUnknown { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ForestChest { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ForestChestCursorOccupied { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ForestChestLocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PickChest { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PickChestCursorOccupied { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PickChestLocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PickChestBug { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ChestSpawn { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ChestSpawnCheck { .. }) => {
                                tick_item_use_chests::dispatch_chest_outcome(
                                    &mut world,
                                    &mut zone_loader,
                                    &mut runtime,
                                    &achievement_repository,
                                    &config,
                                    realtime_seconds,
                                    outcome,
                                    &mut feedback,
                                    &mut executed,
                                    &mut blocked,
                                    &mut failed,
                                )
                                .await;
                            }
                            outcome @ (ugaris_core::item_driver::ItemDriverOutcome::IceItemSpawn { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::IceItemSpawnCursorOccupied { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::WarmFireCursorOccupied { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::IceItemSpawnBug { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::WarmFire { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BackToFire { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::MeltingKeyTick { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PalaceDoorKeyRequired { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::IslenaDoorBusy { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::IslenaDoorRespawning { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::IslenaDoorResting { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PalaceDoorTick { .. }) => {
                                tick_item_use_ice::dispatch_ice_outcome(
                                    &mut world,
                                    &mut zone_loader,
                                    outcome,
                                    &mut feedback,
                                    &mut executed,
                                    &mut blocked,
                                    &mut failed,
                                );
                            }
                            outcome @ (ugaris_core::item_driver::ItemDriverOutcome::DungeonTeleport { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::DungeonFake { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::DungeonKey { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::DungeonKeyCursorOccupied { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::DungeonDoorMissingKeys { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::DungeonDoorTooManyDefenders { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::DungeonDoorSolved { .. }) => {
                                tick_item_use_dungeon::dispatch_dungeon_outcome(
                                    &mut world,
                                    &mut zone_loader,
                                    outcome,
                                    &mut feedback,
                                    &mut executed,
                                    &mut blocked,
                                    &mut failed,
                                )
                                .await;
                            }
                            outcome @ (ugaris_core::item_driver::ItemDriverOutcome::ForestSpadeFind { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ForestSpadeCollapse { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ForestSpadeNothing { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ForestSpadeCursorOccupied { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::JunkpileSearch { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::JunkpileCursorOccupied { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PickDoorToggle { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PickDoorLocked { .. }) => {
                                tick_item_use_dig_pick::dispatch_dig_pick_outcome(
                                    &mut world,
                                    &mut zone_loader,
                                    &mut runtime,
                                    realtime_seconds,
                                    outcome,
                                    &mut feedback,
                                    &mut executed,
                                    &mut blocked,
                                    &mut failed,
                                );
                            }
                            outcome @ (ugaris_core::item_driver::ItemDriverOutcome::BurndownTooHot { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BurndownAlreadyBurned { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BurndownTouch { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BurndownIgnite { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BurndownTimerTick { .. }) => {
                                tick_item_use_burndown::dispatch_burndown_outcome(
                                    &mut runtime,
                                    outcome,
                                    &mut feedback,
                                    &mut executed,
                                    &mut blocked,
                                );
                            }
                            outcome @ (ugaris_core::item_driver::ItemDriverOutcome::TeufelArena { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaExit { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaNeedsSuit { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaLevelTooHigh { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaEquipmentEnhanced { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaEquipmentBound { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaBusy { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaExitLowHealth { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TeufelDoor { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TeufelDoorNoHumans { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TeufelDoorNoBeggars { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TeufelDoorOnlyNobles { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TeufelDoorBusy { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TeufelDoorBug { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TeufelRatNestSpawn { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TeufelRatNestDestroyed { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TeufelRatNestGuarded { .. }) => {
                                tick_item_use_teufel::dispatch_teufel_outcome(
                                    &mut world,
                                    &mut zone_loader,
                                    &mut runtime,
                                    outcome,
                                    &mut feedback,
                                    &mut executed,
                                    &mut blocked,
                                    &mut failed,
                                );
                            }
                            outcome @ (ugaris_core::item_driver::ItemDriverOutcome::SkelRaiseDust { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::SkelRaiseTouch { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::SkelRaiseRaise { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::SkelRaiseTimer { .. }) => {
                                tick_item_use_skelraise::dispatch_skelraise_outcome(
                                    &mut world,
                                    &mut zone_loader,
                                    &mut runtime,
                                    outcome,
                                    &mut feedback,
                                    &mut executed,
                                    &mut failed,
                                );
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::ColorTile { character_id, row, color, .. } => {
                                let matched = if let Some(player) = runtime.player_for_character_mut(character_id) {
                                    let colors = player.ensure_twocity_goodtile_with(|| {
                                        runtime_random_below(6) as u8 + 1
                                    });
                                    colors
                                        .get(usize::from(row))
                                        .is_some_and(|expected| *expected == color)
                                } else {
                                    false
                                };
                                if matched {
                                    executed += 1;
                                } else {
                                    if let Some(player) = runtime.player_for_character_mut(character_id) {
                                        for goodtile in &mut player.twocity_goodtile {
                                            *goodtile = runtime_random_below(6) as u8 + 1;
                                        }
                                    }
                                    feedback.push((character_id, "You see colors dancing before your eyes, and you sense that something has changed.".to_string()));
                                    if world.teleport_character_same_area(character_id, 5, 250, true) {
                                        executed += 1;
                                    } else {
                                        blocked += 1;
                                    }
                                }
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::OrbSpawn { item_id, character_id, anti, special } => {
                                let random_seed = world.tick.0
                                    ^ (u64::from(item_id.0) << 16)
                                    ^ u64::from(character_id.0);
                                match apply_orb_spawn(
                                    &mut world,
                                    &mut zone_loader,
                                    runtime.player_for_character_mut(character_id),
                                    item_id,
                                    character_id,
                                    config.area_id,
                                    realtime_seconds,
                                    anti,
                                    special,
                                    random_seed,
                                ) {
                                    OrbSpawnApplyResult::Granted { item_name, special } => {
                                        let prefix = if special { "An extracting" } else { "An" };
                                        feedback.push((character_id, format!("{prefix} {item_name} was created.")));
                                        executed += 1;
                                    }
                                    OrbSpawnApplyResult::Cooldown { days_left } => {
                                        feedback.push((character_id, format!("Nothing happens, days left: {days_left}")));
                                        blocked += 1;
                                    }
                                    OrbSpawnApplyResult::Nothing => {
                                        feedback.push((character_id, "Nothing happens.".to_string()));
                                        blocked += 1;
                                    }
                                    OrbSpawnApplyResult::CursorOccupied => {
                                        feedback.push((character_id, "Please empty your hand (mouse cursor) first.".to_string()));
                                        blocked += 1;
                                    }
                                    OrbSpawnApplyResult::MissingPlayer => {
                                        failed += 1;
                                    }
                                }
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::TorchExtractOrb {
                                item_id,
                                character_id,
                                modifier_slot,
                                modifier,
                            } => {
                                let granted = instantiate_orb_with_modifier(
                                    &mut zone_loader,
                                    character_id,
                                    modifier,
                                )
                                .is_some_and(|orb| {
                                    world.apply_torch_extract_orb(
                                        item_id,
                                        character_id,
                                        modifier_slot,
                                        orb,
                                    )
                                });
                                if granted {
                                    executed += 1;
                                } else {
                                    blocked += 1;
                                }
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::NomadStack { item_id, character_id } => {
                                match apply_nomad_stack(&mut world, &mut zone_loader, item_id, character_id) {
                                    NomadStackApplyResult::Split { left, right, unit } => {
                                        feedback.push((character_id, format!("Split into {left} {unit}s and {right} {unit}s.")));
                                        executed += 1;
                                    }
                                    NomadStackApplyResult::Merged { count, unit } => {
                                        feedback.push((character_id, format!("{count} {unit}s.")));
                                        executed += 1;
                                    }
                                    NomadStackApplyResult::CannotSplitOne { unit } => {
                                        feedback.push((character_id, format!("You cannot split 1 {unit}.")));
                                        blocked += 1;
                                    }
                                    NomadStackApplyResult::CannotMix => {
                                        feedback.push((character_id, "You cannot mix those.".to_string()));
                                        blocked += 1;
                                    }
                                    NomadStackApplyResult::EnhanceNeedsSilver => {
                                        feedback.push((character_id, "To enhance this item, you need silver.".to_string()));
                                        blocked += 1;
                                    }
                                    NomadStackApplyResult::EnhanceNeedsGold => {
                                        feedback.push((character_id, "This item has already been enhanced once. For further enhancements, you need gold.".to_string()));
                                        blocked += 1;
                                    }
                                    NomadStackApplyResult::EnhanceNotEnough { material, need } => {
                                        feedback.push((character_id, format!("You do not have enough {material} to enhance this item. You need {need} units.")));
                                        blocked += 1;
                                    }
                                    NomadStackApplyResult::EnhanceConfirmUnusable => {
                                        feedback.push((character_id, "Enhancing this item would make it unusable for you. Click again if this is what you want.".to_string()));
                                        blocked += 1;
                                    }
                                    NomadStackApplyResult::Enhanced { used, target_name } => {
                                        feedback.push((character_id, format!("You used {used} units to enhance your {target_name}.")));
                                        executed += 1;
                                    }
                                    NomadStackApplyResult::Bug(message) => {
                                        feedback.push((character_id, message.to_string()));
                                        failed += 1;
                                    }
                                    NomadStackApplyResult::MissingPlayer
                                    | NomadStackApplyResult::MissingItem => {
                                        failed += 1;
                                    }
                                }
                            }
                            outcome @ (ugaris_core::item_driver::ItemDriverOutcome::TransportOpen { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TransportInvalid { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TransportTravel { .. }) => {
                                tick_item_use_transport::dispatch_transport_outcome(
                                    &mut world,
                                    &mut runtime,
                                    &character_repository,
                                    &area_repository,
                                    &config,
                                    outcome,
                                    &mut feedback,
                                    &mut executed,
                                    &mut blocked,
                                    &mut failed,
                                )
                                .await;
                            }
                            outcome @ (ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnExit { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnExitBusy { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnLevelTooHigh { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnContested { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnCountdown { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnAward { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnTimer { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::LqTicker { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::StrTicker { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::LqEntranceClosed { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::LqEntranceLevelBlocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::LqEntranceUndefined { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::LqEntrancePenalty { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ArenaToplist { .. }) => {
                                tick_item_use_clan_lq_arena::dispatch_clan_lq_arena_outcome(
                                    &mut world,
                                    &mut runtime,
                                    &mut zone_loader,
                                    &character_repository,
                                    &area_repository,
                                    &config,
                                    outcome,
                                    &mut feedback,
                                    &mut executed,
                                    &mut blocked,
                                )
                                .await;
                            }
                            outcome @ (ugaris_core::item_driver::ItemDriverOutcome::StrMineLook { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::StrDepotLook { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::StrStorageInteract { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::StrMineWorkerDig { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::StrBuildingWorkerTransfer { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::StrDepotWorkerTakeover { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::StrSpawnerUse { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::StrSpawnerAmbientTick { .. }) => {
                                tick_item_use_strategy::dispatch_strategy_outcome(
                                    &mut world,
                                    &mut zone_loader,
                                    &mut runtime,
                                    outcome,
                                    &mut feedback,
                                    &mut executed,
                                );
                            }
                            outcome @ (ugaris_core::item_driver::ItemDriverOutcome::ZombieShrine { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ZombieShrineNeedsOffering { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::RandomShrineNeedsKey { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::RandomShrineAlreadyUsed { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::RandomShrineBug { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::RandomShrineUse { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::SpecialShrine { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::DemonShrine { .. }) => {
                                tick_item_use_shrines::dispatch_shrine_outcome(
                                    &mut world,
                                    &mut zone_loader,
                                    &mut runtime,
                                    &config,
                                    realtime_seconds,
                                    outcome,
                                    &mut feedback,
                                    &mut executed,
                                    &mut blocked,
                                    &mut failed,
                                );
                            }
                            outcome @ (ugaris_core::item_driver::ItemDriverOutcome::XmasMaker { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::SwampSpawn { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::SwampSpawnPulse { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::XmasTree { .. }) => {
                                tick_item_use_xmas_swamp::dispatch_xmas_swamp_outcome(
                                    &mut world,
                                    &mut zone_loader,
                                    &mut runtime,
                                    args.area_id,
                                    outcome,
                                    &mut feedback,
                                    &mut executed,
                                    &mut blocked,
                                    &mut failed,
                                );
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::BlockedByRequirements { item_id, character_id }
                                if is_chest_request =>
                            {
                                feedback.push((
                                    character_id,
                                    chest_blocked_message(&world, item_id, character_id).to_string(),
                                ));
                                blocked += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::OnOffLightChanged {
                                character_id,
                                now_on: true,
                                remaining_off: Some(0),
                                gates_opened: true,
                                ..
                            } => {
                                if character_id.0 != 0 {
                                    feedback.push((
                                        character_id,
                                        "The light has returned to the palace and the gates open.".to_string(),
                                    ));
                                }
                                executed += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::OnOffLightChanged {
                                character_id,
                                now_on: true,
                                remaining_off: Some(remaining),
                                ..
                            } => {
                                if character_id.0 != 0 {
                                    feedback.push((character_id, format!("{} remaining", remaining)));
                                }
                                executed += 1;
                            }
                            outcome @ (ugaris_core::item_driver::ItemDriverOutcome::EdemonSwitchStuck { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::EdemonDoorLocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::EdemonDoorLifeless { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::EdemonBlockBlocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::EdemonBlockMove { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::EdemonTubePulse { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FdemonLoaderBlocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FdemonCannonLifeless { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::EdemonLoaderBlocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FdemonFarmHarvest { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FdemonFarmCursorOccupied { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FdemonFarmNotReady { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FdemonFarmBug { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FdemonBloodBlocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FdemonBloodDestroyedFlask { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FdemonBloodFilled { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FdemonLavaBlocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FdemonLavaActivated { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FdemonLoaderChanged { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::EdemonDoorToggle { .. }) => {
                                tick_item_use_edemon_fdemon::dispatch_edemon_fdemon_outcome(
                                    &mut world,
                                    &mut zone_loader,
                                    &mut runtime,
                                    outcome,
                                    &mut feedback,
                                    &mut executed,
                                    &mut blocked,
                                );
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::PotionDrunk {
                                character_id,
                                ..
                            } => {
                                area_feedback.push((
                                    character_id,
                                    potion_area_message(&world, character_id),
                                    10,
                                ));
                                executed += 1;
                            }
                            outcome @ (ugaris_core::item_driver::ItemDriverOutcome::WarpTeleportMissingSphere { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::WarpTeleportBug { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::WarpTeleportBusy { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::WarpTeleportSpheres { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::WarpBonusFinished { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::WarpBonusAlreadyUsed { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::WarpBonusNeedsSphere { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::WarpBonus { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::WarpKeySpawnCursorOccupied { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::WarpKeySpawn { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::WarpKeyDoorMissingKey { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::WarpKeyDoorBug { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::WarpKeyDoor { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::WarpTrialDoorWrongSide { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::WarpTrialDoorBusy { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::WarpTrialDoorBug { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::WarpTrialDoor { .. }) => {
                                tick_item_use_warp::dispatch_warp_outcome(
                                    &mut world,
                                    &mut zone_loader,
                                    &mut runtime,
                                    &achievement_repository,
                                    &args,
                                    outcome,
                                    &mut feedback,
                                    &mut feedback_bytes,
                                    &mut executed,
                                    &mut blocked,
                                    &mut failed,
                                )
                                .await;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::StatScrollUsed {
                                character_id,
                                value,
                                ..
                            } => {
                                // C `raise_value_exp` (`src/
                                // system/skill.c:311-373`,
                                // called by the `IDR_STAT_SCROLL`
                                // driver): `if (ch[cn].flags &
                                // CF_PLAYER) {
                                // achievement_check_skill(cn, v,
                                // ch[cn].value[1][v]); }` after
                                // each successful raise - use the
                                // post-charge bare value already
                                // applied to `world.characters`.
                                if let Some(level) = world
                                    .characters
                                    .get(&character_id)
                                    .map(|character| character.values[1][value as usize])
                                {
                                    award_skill_achievement(
                                        &mut world,
                                        &mut runtime,
                                        &achievement_repository,
                                        character_id,
                                        value as i32,
                                        level as i32,
                                    )
                                    .await;
                                }
                                executed += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::FoodEaten { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::DoorToggle { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::DoubleDoorToggle { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FreakDoorUse { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Teleport { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TeleportDoor { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::MineDoorTeleport { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::MineDoorTimer { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Recall { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::CityRecall { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FireballMachineProjectile { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BallTrapProjectile { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::EdemonBallProjectile { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::EdemonGateSpawn { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FdemonCannonPulse { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FdemonGateSpawn { .. }
                               | ugaris_core::item_driver::ItemDriverOutcome::FdemonWaypoint { .. }
                                | ugaris_core::item_driver::ItemDriverOutcome::EdemonLoaderChanged { .. }
                                | ugaris_core::item_driver::ItemDriverOutcome::FdemonFarmChanged { .. }
                                | ugaris_core::item_driver::ItemDriverOutcome::FdemonLavaPulse { .. }
                               | ugaris_core::item_driver::ItemDriverOutcome::FlameThrowerPulse { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FlameThrowerExtinguished { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::SpikeTrapTriggered { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::SpikeTrapReset { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::SwampArmPulse { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::SwampWhispPulse { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TriggerMapItem { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::StepTrapDiscoverTarget { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BoneWallTick { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::LightChanged { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::OnOffLightChanged { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PalaceGateTick { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TorchExtinguishedUnderwater { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::DecayItemToggled { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::LabExitAnimating { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::LabExitExpired { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BeyondPotion { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::AlchemyFlaskPotion { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::OxygenPotion { .. }
                             | ugaris_core::item_driver::ItemDriverOutcome::AccountDepotOpened { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::LookItem { .. } => {
                                executed += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::MineDoorMissingTarget { .. } => {
                                blocked += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::NomadDice {
                                item_id,
                                character_id,
                                luck,
                            } => {
                                if let Some(character) = world.characters.get(&character_id) {
                                    let (x, y) = (character.x, character.y);
                                    let seed = world
                                        .tick
                                        .0
                                        .wrapping_mul(1_048_573)
                                        .wrapping_add(u64::from(character_id.0))
                                        .wrapping_add(u64::from(item_id.0) << 16);
                                    let ([d1, d2, d3], total) = legacy_nomad_dice_roll(seed, luck);
                                    area_feedback.push((
                                        character_id,
                                        format!(
                                            "{} rolled {}, {} and {} for a total of {}.",
                                            character.name, d1, d2, d3, total
                                        ),
                                        8,
                                    ));
                                    // C `notify_area(ch[cn].x, ch[cn].y,
                                    // NT_NPC, NTID_DICE, cn, d1+d2+d3)`
                                    // (`nomad.c:1174`): delivers the
                                    // player's own roll to whichever
                                    // nomad NPC is mid-game with them
                                    // (`World::nomad_handle_npc_message`).
                                    world.notify_area(
                                        x,
                                        y,
                                        ugaris_core::character_driver::NT_NPC,
                                        ugaris_core::character_driver::NTID_DICE,
                                        character_id.0 as i32,
                                        i32::from(total),
                                    );
                                }
                                executed += 1;
                            }
                            outcome @ (ugaris_core::item_driver::ItemDriverOutcome::LollipopLicked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::LollipopMemories { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ChristmasPopInspected { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::SpecialPotionDrunk { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::SpecialPotionAntidote { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::SpecialPotionInfravision { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::SpecialPotionSecurity { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::SpecialPotionProfessionReset { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::SpecialPotionBug { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BookText { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BookcaseText { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BookcaseLocked { .. }) => {
                                tick_item_use_books_potions::dispatch_books_potions_outcome(
                                    &mut world,
                                    &mut runtime,
                                    args.area_id,
                                    outcome,
                                    &mut feedback,
                                    &mut feedback_bytes,
                                    &mut special_feedback,
                                    &mut area_feedback,
                                    &mut executed,
                                    &mut blocked,
                                );
                            }
                            outcome @ (ugaris_core::item_driver::ItemDriverOutcome::StafferBookText { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::StafferAnimationBook { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::StafferMineExhausted { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::StafferBlockBlocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::StafferSpecDoorLocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::StafferMineDig { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::StafferMineTimer { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::StafferBlockMove { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::StafferBlockTimer { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::StafferSpecDoorToggle { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::SaltmineDoorBlocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::SaltmineLadderUse { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::SaltmineSaltbagUse { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BoneHint { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PalaceKeySplit { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PalaceKeyNeedsCursor { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PalaceKeyDoesNotFit { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PalaceKeyCombine { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::EnchantNeedsCursor { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::EnchantCursorItem { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::AntiEnchantCursorItem { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ShrikeAmuletNeedsCursor { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ShrikeAmuletDoesNotFit { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ShrikeAmuletAssemble { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::MineGatewayKeyNeedsCursor { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::MineGatewayKeyDoesNotFit { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::MineGatewayKeyAssemble { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::MineGateway { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::MineGatewayNeedsKey { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::MineGatewayBug { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::MineKeyDoorNeedsGold { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::MineKeyDoorBusy { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::MineKeyDoor { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::MineKeyDoorOpened { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ArkhataKeyNeedsCursor { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ArkhataKeyDoesNotFit { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ArkhataKeyAssemble { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ArkhataPool { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ArkhataPoolNeedsCursor { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ArkhataPoolWrongCursor { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ArkhataStopwatch { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BoneHolderBadCursor { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BoneHolderOccupied { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BoneHolderEmptyTouch { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BoneHolderWrongOwner { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BoneHolderInsertRune { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BoneHolderRemoveRune { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BoneHolderActivate { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BoneHolderActivateResolved { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BoneHolderExpired { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::LizardFlowerMixed { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::LizardFlowerNeedsCursor { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::LizardFlowerDoesNotFit { .. }) => {
                                tick_item_use_keyassembly::dispatch_keyassembly_outcome(
                                    &mut world,
                                    &mut zone_loader,
                                    &mut runtime,
                                    &character_repository,
                                    &area_repository,
                                    &config,
                                    &args,
                                    realtime_seconds,
                                    outcome,
                                    &mut feedback,
                                    &mut executed,
                                    &mut blocked,
                                    &mut failed,
                                )
                                .await;
                            }
                            outcome @ (ugaris_core::item_driver::ItemDriverOutcome::CaligarWeightBlocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::CaligarWeightDoorLocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::CaligarWeightDoorBusy { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::CaligarWeightMove { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::CaligarWeightDoor { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::CaligarWeightTimer { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::CaligarGunProjectile { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::CaligarKeyAssemble { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::CaligarKeyNeedsCursor { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::CaligarKeyDoesNotFit { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::CaligarSkellyDoor { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::CaligarSkellyDoorLocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::CaligarSkellyDoorBusy { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::CaligarTraining { .. }) => {
                                tick_item_use_caligar::dispatch_caligar_outcome(
                                    &mut world,
                                    &mut zone_loader,
                                    &mut runtime,
                                    outcome,
                                    &mut feedback,
                                    &mut executed,
                                    &mut blocked,
                                    &mut failed,
                                );
                            }
                            outcome @ (ugaris_core::item_driver::ItemDriverOutcome::KeyringShow { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Extinguish { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::KeyedDoorToggle { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::KeyringAddCursorItem { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::AssembleItem { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::AssembleNeedsCursor { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::AssembleDoesNotFit { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::AssembleUnknownItem { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ParkShrine { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::ParkShrineBug { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PickBerry { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PickBerryCursorOccupied { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PickAlchemyFlower { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PickAlchemyFlowerCursorOccupied { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FlaskIngredientAdded { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FlaskWrongCursor { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FlaskFull { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FlaskFinishedNoMoreIngredients { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FlaskEmptyShaken { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FlaskIngredientBug { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FlaskMixed { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::FlaskRuined { .. }) => {
                                tick_item_use_crafting::dispatch_crafting_outcome(
                                    &mut world,
                                    &mut zone_loader,
                                    &mut runtime,
                                    &achievement_repository,
                                    realtime_seconds,
                                    outcome,
                                    &driver_context,
                                    &mut feedback,
                                    &mut executed,
                                    &mut blocked,
                                    &mut failed,
                                )
                                .await;
                            }
                            outcome @ (ugaris_core::item_driver::ItemDriverOutcome::BranningtonUnderwaterBerry { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab3YellowBerry { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab3WhiteBerry { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab3WhiteBerryLightTick { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab3BrownBerry { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab2WaterWell { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab2WaterAltar { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab2WaterDrink { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab2WaterCursorOccupied { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab2StepActionClear { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab2StepActionDaemonCheck { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab2StepActionDaemonWarning { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab2GraveClueBook { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab2GraveClose { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab2GraveCheckOpen { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab2GraveOpen { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::LabEntranceSolvedAll { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::LabEntranceTooLow { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::LabExitWrongOwner { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::LabExitUse { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::DeathfibrinShrineGive { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::DeathfibrinShrineOccupied { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::DeathfibrinNeedsCarry { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::DeathfibrinNoMaster { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::DeathfibrinStrike { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab3TeleportDoorLocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab3TeleportDoorBusy { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab3TeleportDoor { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab3NoteGivingBlocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab3NoteGivingSkeleton { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab3NoteRead { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab4FireplaceKeyBlocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab4FireplaceKeyGive { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab5Obelisk { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab5PotionDrunk { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab5ChestboxAlreadyOpened { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab5ChestboxOpen { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab5ChestboxClose { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab5RitualStart { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab5RitualProgress { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab5RitualNothing { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab5RitualHurtAtItem { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab5EntranceRitualHurt { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab5Backdoor { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab5GunLocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab5GunReloadTick { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab5PikeHurt { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab5PikeReset { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab5NoPotionDoorBlocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::Lab5NoPotionDoorPass { .. }) => {
                                tick_item_use_lab::dispatch_lab_outcome(
                                    &mut world,
                                    &mut zone_loader,
                                    &mut runtime,
                                    &character_repository,
                                    &area_repository,
                                    &config,
                                    outcome,
                                    &mut feedback,
                                    &mut area_feedback,
                                    &mut executed,
                                    &mut blocked,
                                    &mut failed,
                                )
                                .await;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::EmptyPotionTemplateNeeded {
                                item_id,
                                character_id,
                                empty_kind,
                            } => {
                                if apply_empty_potion_drink(
                                    &mut world,
                                    &mut zone_loader,
                                    item_id,
                                    character_id,
                                    empty_kind,
                                ) {
                                    area_feedback.push((
                                        character_id,
                                        potion_area_message(&world, character_id),
                                        10,
                                    ));
                                    executed += 1;
                                } else {
                                    deferred_templates += 1;
                                }
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::BlockedByArea { item_id, character_id }
                                if is_no_potion_area_blocked_item(&world, item_id) =>
                            {
                                feedback.push((character_id, "You sense that the potion would not work.".to_string()));
                                blocked += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::LibloadAreaBlocked { character_id, .. } => {
                                feedback.push((character_id, "This does not work outside its area.".to_string()));
                                blocked += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::BlockedByArea { .. } => {
                                blocked += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::BlockedByRequirements { item_id, character_id }
                                if is_timed_potion_source_item(&world, item_id) =>
                            {
                                let message = if character_has_active_beyond_potion(&world, character_id) {
                                    "Another potion is still active."
                                } else {
                                    "You do not meet the requirements needed to use this potion."
                                };
                                feedback.push((
                                    character_id,
                                    message.to_string(),
                                ));
                                blocked += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::BlockedByRequirements { item_id, character_id }
                                if is_torch_item(&world, item_id) =>
                            {
                                feedback.push((character_id, TORCH_UNDERWATER_MESSAGE.to_string()));
                                blocked += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::BlockedByRequirements { item_id, character_id }
                                if is_demonshrine_item(&world, item_id) =>
                            {
                                feedback.push((character_id, "You're not powerful enough to read this book.".to_string()));
                                blocked += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::BlockedByRequirements { .. } => {
                                blocked += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::PentBossDoor { .. } => {
                                executed += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::PentagramActivate { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PentagramTimer { .. } => {
                                executed += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::PentagramAlreadyActive { .. } => {
                                blocked += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::PentBossDoorLocked { character_id, .. } => {
                                feedback.push((
                                    character_id,
                                    "The door won't open. It seems it is only accessible directly after a solve.".to_string(),
                                ));
                                blocked += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::PentBossDoorBusy { character_id, .. } => {
                                feedback.push((
                                    character_id,
                                    "Please try again soon. Target is busy.".to_string(),
                                ));
                                blocked += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::TrapdoorOpen { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TrapdoorBlocked { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::TrapdoorClose { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::GasTrapPulse { .. } => {
                                executed += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::EdemonBallInactive { .. } => {
                                executed += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::TrapdoorBusy { character_id, .. } => {
                                feedback.push((
                                    character_id,
                                    "You cannot do anything with it now.".to_string(),
                                ));
                                blocked += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::TrapdoorNeedsStick { character_id, .. } => {
                                feedback.push((
                                    character_id,
                                    "You'd need something like a hard stick to lock the door.".to_string(),
                                ));
                                blocked += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::BoneBridgePlace { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BoneBridgeTimerTick { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::BoneBridgeAddBone { .. } => {
                                executed += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::BoneBridgeFinished {
                                character_id, ..
                            } => {
                                feedback.push((
                                    character_id,
                                    "The bridge is finished. You cannot add more bones."
                                        .to_string(),
                                ));
                                blocked += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::BoneBridgeWrongCursorItem {
                                character_id, ..
                            }
                            | ugaris_core::item_driver::ItemDriverOutcome::BoneBridgeNotEnoughBones {
                                character_id, ..
                            } => {
                                feedback.push((character_id, "Hu?".to_string()));
                                blocked += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::BoneBridgeRemoveBone {
                                character_id, ..
                            } => {
                                // C `bonebridge:266-268`: `create_item("bone")` is
                                // unconditional in C (no failure check); the Rust
                                // `ZoneLoader` path can fail if the template is
                                // missing, so fall back to a no-op like the other
                                // `instantiate_item_template` call sites do.
                                match zone_loader
                                    .instantiate_item_template("bone", Some(character_id))
                                {
                                    Ok(item) => {
                                        let item_id = item.id;
                                        world.add_item(item);
                                        world.give_char_item(character_id, item_id);
                                        executed += 1;
                                    }
                                    Err(_) => {
                                        failed += 1;
                                    }
                                }
                            }
                            outcome @ (ugaris_core::item_driver::ItemDriverOutcome::MineWallInitialized { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::MineWallDig { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::MineWallCursorOccupied { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::MineWallExhausted { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::MineWallCollapse { .. }) => {
                                tick_item_use_minewall::dispatch_minewall_outcome(
                                    &mut world,
                                    &mut zone_loader,
                                    &mut runtime,
                                    &achievement_repository,
                                    config.area_id,
                                    outcome,
                                    &mut feedback,
                                    &mut executed,
                                    &mut blocked,
                                    &mut deferred_templates,
                                )
                                .await;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::IdentityTag { .. } => {
                                executed += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::Unsupported { .. } => {
                                unsupported += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::TorchExpired { .. } => {
                                executed += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::ClanJewelRescheduled { .. } => {
                                executed += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::Lab2RegenerateTick { .. } => {
                                executed += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::ClanJewelExpired { character_id, item_name, .. } => {
                                if let Some(character_id) = character_id {
                                    let item_name = String::from_utf8_lossy(&item_name)
                                        .trim_end_matches('\0')
                                        .to_string();
                                    feedback.push((character_id, format!("Your {item_name} expired.")));
                                }
                                executed += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::DecayItemExpired { character_id, item_name, .. } => {
                                let item_name = String::from_utf8_lossy(&item_name)
                                    .trim_end_matches('\0')
                                    .to_string();
                                feedback.push((character_id, format!("Your {item_name} expired.")));
                                executed += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::PalaceBombExplode { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PalaceBombTimer { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PalaceBombToggled { .. }
                            | ugaris_core::item_driver::ItemDriverOutcome::PalaceCapTimer { .. } => {
                                executed += 1;
                            }
                            ugaris_core::item_driver::ItemDriverOutcome::Noop => {
                                failed += 1;
                            }
                        }
                    }
                    // C `use_item` (`src/system/do.c:1504-
                    // 1508`): `log_char(cn, LOG_SYSTEM, 0,
                    // "Permission denied.");` - the
                    // grave-container access-denied reply.
                    Err(ugaris_core::item_driver::UseItemError::AccessDenied) => {
                        feedback.push((use_character_id, "Permission denied.".to_string()));
                        blocked += 1;
                    }
                    Err(_) => {
                        failed += 1;
                    }
                }
            }
            let mut feedback_sessions = 0;
            for (character_id, message) in feedback {
                let payload = ugaris_protocol::packet::system_text(&message);
                for (session_id, _) in runtime.sessions_for_character(character_id) {
                    if runtime.send_to_session(session_id, payload.clone()) {
                        feedback_sessions += 1;
                    }
                }
            }
            for (character_id, message) in feedback_bytes {
                let payload = ugaris_protocol::packet::system_text_bytes(&message);
                for (session_id, _) in runtime.sessions_for_character(character_id) {
                    if runtime.send_to_session(session_id, payload.clone()) {
                        feedback_sessions += 1;
                    }
                }
            }
            for (character_id, payload) in special_feedback {
                for (session_id, _) in runtime.sessions_for_character(character_id) {
                    if runtime.send_to_session(session_id, payload.clone()) {
                        feedback_sessions += 1;
                    }
                }
            }
            for (character_id, message, maxdist) in area_feedback {
                let payload = ugaris_protocol::packet::system_text(&message);
                for (session_id, _) in
                    runtime.sessions_for_area_message(&world, character_id, maxdist)
                {
                    if runtime.send_to_session(session_id, payload.clone()) {
                        feedback_sessions += 1;
                    }
                }
            }
            let mut container_sessions = 0;
            container_refresh.sort_unstable_by_key(|id| id.0);
            container_refresh.dedup();
            for character_id in container_refresh {
                let Some(payload) = current_container_payload(
                    &world,
                    runtime.account_depots.get(&character_id),
                    runtime
                        .player_for_character(character_id)
                        .map(|player| player.depot.as_slice()),
                    character_id,
                ) else {
                    continue;
                };
                for (session_id, _) in runtime.sessions_for_character(character_id) {
                    if runtime.send_to_session(session_id, payload.clone()) {
                        container_sessions += 1;
                    }
                }
            }
            info!(
                opened,
                executed,
                unsupported,
                deferred_templates,
                blocked,
                failed,
                feedback_sessions,
                container_sessions,
                tick = world.tick.0,
                "processed item-use requests"
            );
        }
        clear_completed_use_actions(&mut runtime, &completed_actions);
        let mut refreshed_sessions = 0;
        for completion in completed_actions.iter() {
            let Some(character) = world.characters.get(&completion.character_id) else {
                continue;
            };
            let walk_section_payload =
                if completion.ok && completion.action_id == ugaris_core::legacy::action::WALK {
                    runtime
                        .player_for_character_mut(completion.character_id)
                        .and_then(|player| walk_section_payload(config.area_id, player, character))
                } else {
                    None
                };
            let pk_relations = PkRelationSnapshot::from_runtime(&runtime);
            for (session_id, view_distance) in
                runtime.sessions_for_character(completion.character_id)
            {
                let mut payloads = if completion.ok
                    && completion.action_id == ugaris_core::legacy::action::WALK
                {
                    // Shift the session cache like the client
                    // shifts its map; the per-tick diff pass
                    // fills fringe and LOS changes afterwards.
                    let scroll_payload =
                        runtime.map_caches.get_mut(&session_id).and_then(|cache| {
                            movement_scroll_payload(
                                character,
                                completion.old_x,
                                completion.old_y,
                                view_distance,
                                cache,
                            )
                        });
                    match scroll_payload {
                        Some(payload) => vec![payload],
                        None => {
                            let payloads = map_refresh_payloads(
                                &world,
                                character,
                                &pk_relations,
                                view_distance,
                            );
                            runtime.map_caches.insert(
                                session_id,
                                visible_map_cache(&world, character, &pk_relations, view_distance),
                            );
                            payloads
                        }
                    }
                } else {
                    match runtime.map_caches.get_mut(&session_id) {
                        Some(cache) => map_diff_payloads(
                            &world,
                            character,
                            &pk_relations,
                            view_distance,
                            cache,
                        ),
                        None => {
                            let payloads = map_refresh_payloads(
                                &world,
                                character,
                                &pk_relations,
                                view_distance,
                            );
                            runtime.map_caches.insert(
                                session_id,
                                visible_map_cache(&world, character, &pk_relations, view_distance),
                            );
                            payloads
                        }
                    }
                };
                if completion.action_id != ugaris_core::legacy::action::WALK {
                    payloads.push(inventory_snapshot_payload(&world, character));
                }
                if let Some(payload) = &walk_section_payload {
                    payloads.push(payload.clone());
                }
                if completion.ok {
                    if let Some(payload) = area_sound_payload(
                        config.area_id,
                        character,
                        world.date.hour,
                        world
                            .tick
                            .0
                            .wrapping_add(u64::from(completion.character_id.0) << 32),
                    ) {
                        payloads.push(bytes::BytesMut::from(&payload[..]));
                    }
                }
                payloads.extend(client_effect_payloads(
                    &world,
                    character,
                    view_distance,
                    runtime.effect_caches.entry(session_id).or_default(),
                ));
                if runtime.send_many_to_session(session_id, payloads) {
                    refreshed_sessions += 1;
                }
            }
        }
        if refreshed_sessions != 0 {
            info!(
                refreshed_sessions,
                tick = world.tick.0,
                "queued map refreshes for completed actions"
            );
        }

        let mut sound_sessions = 0;
        for sound in world.drain_pending_sound_specials() {
            let payload = ugaris_protocol::packet::special(
                sound.special.special_type,
                sound.special.opt1 as u32,
                sound.special.opt2 as u32,
            );
            for (session_id, _) in runtime.sessions_for_character(sound.character_id) {
                if runtime.send_to_session(session_id, bytes::BytesMut::from(&payload[..])) {
                    sound_sessions += 1;
                }
            }
        }
        if sound_sessions != 0 {
            info!(
                sound_sessions,
                tick = world.tick.0,
                "queued legacy sound-area specials"
            );
        }

        crate::pents::process_pentagram_activations(world, runtime, achievement_repository).await;
        crate::pents::process_pentagram_demon_spawns(world, zone_loader, runtime);
        crate::pents::process_penter_demon_lords_demise_awards(
            world,
            runtime,
            achievement_repository,
        )
        .await;
        crate::area11::process_islena_ladykiller_awards(world, runtime, achievement_repository)
            .await;
    }
}
