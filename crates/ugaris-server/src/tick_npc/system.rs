//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn death_1(
    world: &mut World,
    runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    _character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // C `death.c:1214-1217`'s `player_use_potion`/
    // `player_use_recall` reaction to any hp damage a
    // `CF_PLAYER`+`CDR_LOSTCON` character just took, wherever
    // in this tick it happened (combat, spells, weather,
    // traps, ...) - see `pending_lostcon_hurt_events`'s doc
    // comment (`ugaris-core::world::mod`) for why this reacts
    // once per tick here instead of running inline inside
    // `apply_legacy_hurt` itself.
    let lostcon_hurt_events = world.drain_lostcon_hurt_events();
    let mut lostcon_self_defense_reactions = 0;
    for character_id in lostcon_hurt_events {
        let Some(player) = runtime.lostcon_players.get(&character_id) else {
            continue;
        };
        let self_care_suppressions = player.lostcon_self_care_suppressions();
        let norecall = player.no_recall;
        if world.process_player_use_potion(character_id, config.area_id, self_care_suppressions) {
            lostcon_self_defense_reactions += 1;
        }
        if world.process_player_use_recall(character_id, config.area_id, norecall) {
            lostcon_self_defense_reactions += 1;
        }
    }
    if lostcon_self_defense_reactions != 0 {
        info!(
            lostcon_self_defense_reactions,
            tick = world.tick.0,
            "lostcon character(s) used a potion/recall in reaction to taking damage"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn macro_track_exp_gain_2(
    world: &mut World,
    runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    _character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // C `macro_track_exp_gain`/`macro_track_combat`/
    // `macro_track_gold_change` (`src/system/tool.c:385-426`,
    // `death.c:1112-1117`): stamp the Macro Daemon's
    // `MacroPpd::last_exp_gain`/`last_combat`/
    // `last_gold_change` activity fields for whatever exp/
    // combat/gold events this tick's `give_exp`/
    // `apply_legacy_hurt`/`gate_give_money_silent` calls
    // queued, wherever in this tick they happened.
    let macro_events_applied = apply_macro_activity_events(runtime, world, current_unix_time());
    if macro_events_applied != 0 {
        info!(
            macro_events_applied,
            tick = world.tick.0,
            "applied Macro Daemon activity-tracking events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn macro_driver_3(
    world: &mut World,
    runtime: &mut ServerRuntime,
    zone_loader: &mut ZoneLoader,
    config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    _character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // C `macro_driver`: the anti-macro/anti-bot "Macro Daemon"
    // NPC (`src/module/base.c`) - victim search (plus
    // `/summonmacro`'s forced-pickup), teleport-to-victim,
    // challenge asking/repeating/timeout, and reward granting.
    let (is_xmas, _xmas_event_year) = runtime_effective_xmas_event(runtime);
    let macro_daemon_events_applied = apply_macro_events(
        world,
        runtime,
        zone_loader,
        config.area_id,
        is_xmas,
        current_unix_time(),
    );
    if macro_daemon_events_applied != 0 {
        info!(
            macro_daemon_events_applied,
            tick = world.tick.0,
            "Macro Daemon NPC(s) advanced state this tick"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn merchant_actions_5(
    world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    _character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) -> std::collections::HashSet<CharacterId> {
    // C merchant_driver: store creation, greetings, and trade
    // activation, then push store views to players whose active
    // merchant changed.
    let merchants_before_tick: std::collections::HashSet<CharacterId> =
        world.merchant_stores.keys().copied().collect();
    world.process_merchant_actions();
    merchants_before_tick
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn bank_driver_7(
    world: &mut World,
    runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    _character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // C `bank_driver`: greetings, small talk, and
    // deposit/withdraw/balance text commands (`src/module/
    // bank.c`).
    world.process_bank_actions(config.area_id);
    let bank_events_applied = apply_bank_events(runtime, world);
    if bank_events_applied != 0 {
        info!(
            bank_events_applied,
            tick = world.tick.0,
            "applied bank account events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn trader_driver_8(
    world: &mut World,
    runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    _character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // C `trader_driver`: player-to-player trade middleman NPC
    // (`src/module/base.c`).
    world.process_trader_actions();
    let trader_events_applied = apply_trader_events(world, runtime, achievement_repository).await;
    if trader_events_applied != 0 {
        info!(
            trader_events_applied,
            tick = world.tick.0,
            "applied trader item-look events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn janitor_driver_52(
    world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    _character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // C `janitor_driver`: lamp-lighting/item-tidying NPC
    // (`src/module/base.c`).
    world.process_janitor_actions(config.area_id);
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn merchant_driver_53(
    world: &mut World,
    _runtime: &mut ServerRuntime,
    zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    _character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
    merchants_before_tick: &std::collections::HashSet<CharacterId>,
) {
    // C `merchant_driver`: seed/refresh "special" enchanted-item
    // stock (`add_special_store`, every 12h).
    let special_store_updates = world.refresh_special_stores(zone_loader);
    for merchant_id in special_store_updates {
        save_merchant_store_if_configured(world, merchant_repository, merchant_id).await;
    }
    if let Some(repository) = &merchant_repository {
        // C `create_store`: `load_merchant_inventory` on first
        // creation, or an initial `queue_merchant_full_save` if
        // nothing was persisted yet for this merchant.
        let newly_created_stores: Vec<CharacterId> = world
            .merchant_stores
            .keys()
            .copied()
            .filter(|id| !merchants_before_tick.contains(id))
            .collect();
        for merchant_id in newly_created_stores {
            let Some((name, x, y)) = world
                .characters
                .get(&merchant_id)
                .map(|merchant| (merchant.name.clone(), merchant.x, merchant.y))
            else {
                continue;
            };
            match repository
                .load_store(&name, i32::from(x), i32::from(y))
                .await
            {
                Ok(Some(snapshot)) => {
                    apply_merchant_store_snapshot(world, merchant_id, snapshot);
                    info!(merchant = %name, x, y, "loaded merchant store from database");
                }
                Ok(None) => {
                    if let Some(snapshot) = merchant_store_snapshot(world, merchant_id) {
                        match repository.save_store(&snapshot).await {
                            Ok(()) => {
                                info!(merchant = %name, x, y, "saved initial merchant store to database")
                            }
                            Err(err) => {
                                warn!(merchant = %name, x, y, error = %err, "failed to save initial merchant store")
                            }
                        }
                    }
                }
                Err(err) => {
                    warn!(merchant = %name, x, y, error = %err, "failed to load merchant store from database");
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn maintenance_60s_task_54(
    world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    config: &ServerConfig,
    args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    _character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // C `maintenance_60s_task` (`server.c:197-210`): re-run
    // `area_alive(0)` every 85 seconds
    // (`add_scheduled_task(maintenance_60s_task, 85,
    // "Maintenance", true)`) to keep this area server's
    // `area_servers` row fresh - closes the "periodic
    // `mark_alive` heartbeat (still startup-only)" gap the
    // "Cross-area transfer" task's Progress Log tracked.
    if world.tick.0 % (TICKS_PER_SECOND * 85) == 0 {
        if let Some(repository) = &area_repository {
            let public_addr = args.public_addr.unwrap_or(args.bind_addr);
            mark_area_alive(repository, config.area_id, config.mirror_id, public_addr).await;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn maintenance_60s_task_55(
    world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    _character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // C `maintenance_60s_task` (`server.c:197-210`):
    // `update_auction_house()` delivers expired auctions'
    // items/gold to their winners (or returns them to the
    // seller if unsold) roughly once a minute, not every tick.
    if world.tick.0 % (TICKS_PER_SECOND * 60) == 0 {
        if let Some(repository) = &auction_repository {
            match repository.cleanup_expired_auctions().await {
                Ok(processed) if processed > 0 => {
                    info!(processed, tick = world.tick.0, "processed expired auctions");
                }
                Ok(_) => {}
                Err(err) => {
                    warn!(error = %err, "failed to process expired auctions");
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn world_56(
    world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    _character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // Restart-persistence for `world.clan_registry`: C has no
    // direct equivalent (its clan data rides along inside the
    // whole-server memory-image save, not a dedicated flush
    // task), so this reuses the same once-a-minute cadence as
    // the auction/play-time maintenance above rather than
    // C's own `update_state`-driven storage state machine.
    // Gated on `ClanRegistry::dirty` (mirroring C's own
    // `clan_changed` check, `clan.c:415-418`) so an unchanged
    // registry doesn't get rewritten every minute.
    if world.tick.0 % (TICKS_PER_SECOND * 60) == 0 && world.clan_registry.dirty() {
        if let Some(repository) = &clan_repository {
            match repository.save_registry(&world.clan_registry).await {
                Ok(()) => world.clan_registry.clear_dirty(),
                Err(err) => {
                    warn!(error = %err, "failed to save clan registry to database")
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn init_event_system_60(
    world: &mut World,
    runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    _character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // C `init_event_system`'s `add_scheduled_task(check_events,
    // 60, "event_check", true)`: check every registered
    // recurring event's should-be-active state once a minute.
    if world.tick.0 % (TICKS_PER_SECOND * 60) == 0 {
        let now = events::CalendarNow::now();
        for (kind, started) in
            events::check_recurring_events(&mut world.settings, &mut runtime.recurring_events, &now)
        {
            if started {
                info!(event = kind.name(), "recurring event started");
            } else {
                info!(event = kind.name(), "recurring event ended");
            }
        }
        // C `easter_event.c`'s registration into the same
        // once-a-minute `check_events` task.
        match events::check_easter_event(&mut world.settings, &mut runtime.easter_event, &now) {
            Some(true) => info!(event = "Easter", "recurring event started"),
            Some(false) => info!(event = "Easter", "recurring event ended"),
            None => {}
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn tick_player_61(
    world: &mut World,
    runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    _character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // C `tick_player`'s deferred-init sweep (`player.c:3660-
    // 3676`): `ticks >= 2 && (deferred_init &
    // DEFERRED_ACHIEVEMENTS)` fires `achievement_sync_all` +
    // `achievement_award(ACHIEVEMENT_STARTED_UGARIS)` +
    // `achievement_check_level`/`_exploration`/
    // `_login_streak`. Each newly-unlocked achievement sends
    // its own `SV_ACH_UNLOCK` (`achievement_send_to_client`,
    // called from inside C's `achievement_award`).
    // `DEFERRED_MOTD`'s own gate is not ported here - MOTD
    // doesn't exist yet (see PORTING_TODO.md).
    {
        let due_achievement_notices: Vec<CharacterId> = runtime
            .players
            .values()
            .filter(|player| {
                player.deferred_init & DEFERRED_ACHIEVEMENTS != 0
                    && world.tick.0.saturating_sub(player.login_tick) >= 2
            })
            .filter_map(|player| player.character_id)
            .collect();
        for character_id in due_achievement_notices {
            let character_info = world.characters.get(&character_id).map(|character| {
                (
                    character.name.clone(),
                    character.level as i32,
                    character.flags.contains(CharacterFlags::HARDCORE),
                )
            });
            let area_id = world.area_id as i32;
            let mut payloads: Vec<bytes::BytesMut> = Vec::new();
            if let Some(player) = runtime.player_for_character_mut(character_id) {
                player.deferred_init &= !DEFERRED_ACHIEVEMENTS;
                payloads =
                    achievement_sync_payloads(&player.achievement_data, &player.achievement_stats);
                if let Some((name, level, is_hardcore)) = character_info {
                    let now = current_unix_time();
                    let mut unlocked = Vec::new();
                    if player
                        .achievement_data
                        .award(AchievementType::StartedUgaris, &name, now)
                    {
                        unlocked.push(AchievementType::StartedUgaris);
                    }
                    unlocked.extend(check_level(
                        &mut player.achievement_data,
                        level,
                        is_hardcore,
                        &name,
                        now,
                    ));
                    unlocked.extend(check_exploration(
                        &mut player.achievement_data,
                        area_id,
                        &name,
                        now,
                    ));
                    unlocked.extend(check_login_streak(
                        &mut player.achievement_data,
                        &mut player.achievement_stats,
                        &name,
                        now,
                    ));
                    if !unlocked.is_empty() {
                        record_achievement_firsts_and_announce(
                            world,
                            achievement_repository,
                            character_id,
                            &name,
                            &unlocked,
                        )
                        .await;
                    }
                    for ty in unlocked {
                        payloads.push(achievement_unlock_payload(ty, now));
                    }
                }
            }
            for payload in payloads {
                for (session_id, _) in runtime.sessions_for_character(character_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
            }
        }
    }
    if let Some(repository) = &auction_repository {
        let due_auction_notices: Vec<CharacterId> = runtime
            .players
            .values()
            .filter(|player| {
                player.deferred_init & DEFERRED_AUCTION != 0
                    && world.tick.0.saturating_sub(player.login_tick) >= 6
            })
            .filter_map(|player| player.character_id)
            .collect();
        for character_id in due_auction_notices {
            if let Some(player) = runtime.player_for_character_mut(character_id) {
                player.deferred_init &= !DEFERRED_AUCTION;
            }
            match auction::auction_login_notice(repository, character_id).await {
                Ok(Some(message)) => {
                    let payload = ugaris_protocol::packet::system_text_bytes(&message);
                    for (session_id, _) in runtime.sessions_for_character(character_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                }
                Ok(None) => {}
                Err(err) => {
                    warn!(character_id = character_id.0, error = %err, "failed to check pending auction deliveries");
                }
            }
        }
    }
    {
        let mut merchant_view_updates: Vec<(CharacterId, Option<bytes::BytesMut>)> = Vec::new();
        let session_characters: Vec<CharacterId> = runtime
            .players
            .values()
            .filter_map(|player| player.character_id)
            .collect();
        for character_id in session_characters {
            world.check_merchant(character_id);
            let current = world
                .characters
                .get(&character_id)
                .and_then(|character| character.merchant);
            let cached = runtime.merchant_views.get(&character_id).copied();
            match (current, cached) {
                (Some(merchant_id), cached) if cached != Some(merchant_id) => {
                    runtime.merchant_views.insert(character_id, merchant_id);
                    merchant_view_updates
                        .push((character_id, merchant_store_payload(world, character_id)));
                }
                (None, Some(_)) => {
                    runtime.merchant_views.remove(&character_id);
                    merchant_view_updates.push((character_id, Some(container_close_payload())));
                }
                _ => {}
            }
        }
        for (character_id, payload) in merchant_view_updates {
            let Some(payload) = payload else { continue };
            for (session_id, _) in runtime.sessions_for_character(character_id) {
                runtime.send_to_session(session_id, payload.clone());
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn professor_driver_186(
    world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    _character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // C `professor_driver`: generic profession-teacher NPC
    // (`src/common/professor.c`). The queued
    // `achievement_check_profession` calls it generates are drained in
    // `tick_world::world_step`, alongside every other pending achievement
    // check (`world::exp::LevelAchievementCheck` precedent).
    world.process_professor_actions(config.area_id);
}
