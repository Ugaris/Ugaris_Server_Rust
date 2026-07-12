//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn military_master_driver_35(
    mut world: &mut World,
    mut runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    config: &ServerConfig,
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
    // C `military_master_driver`: the mission-giving Military
    // Master NPC (`src/module/military.c`).
    world.process_military_master_actions(config.area_id, current_unix_time());
    let military_master_events_applied = apply_military_master_events(
        &mut world,
        &mut runtime,
        &achievement_repository,
        config.area_id,
    )
    .await;
    if military_master_events_applied != 0 {
        info!(
            military_master_events_applied,
            tick = world.tick.0,
            "applied military master mission-dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn world_57(
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
    military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // Restart-persistence for `world.military_master_storage`:
    // same once-a-minute cadence and `dirty`-gating as the
    // clan registry save above, for the same reason (no C
    // equivalent flush task to mirror - see
    // `crates/ugaris-db/src/military.rs`'s doc comment).
    if world.tick.0 % (TICKS_PER_SECOND * 60) == 0 && world.military_master_storage.dirty() {
        if let Some(repository) = &military_master_storage_repository {
            match repository
                .save_registry(&world.military_master_storage)
                .await
            {
                Ok(()) => world.military_master_storage.clear_dirty(),
                Err(err) => {
                    warn!(error = %err, "failed to save military master storage registry to database")
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn world_58(
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
    military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // Restart-persistence for `world.military_advisor_storage`:
    // same once-a-minute cadence and `dirty`-gating as the
    // Military Master storage save above.
    if world.tick.0 % (TICKS_PER_SECOND * 60) == 0 && world.military_advisor_storage.dirty() {
        if let Some(repository) = &military_advisor_storage_repository {
            match repository
                .save_registry(&world.military_advisor_storage)
                .await
            {
                Ok(()) => world.military_advisor_storage.clear_dirty(),
                Err(err) => {
                    warn!(error = %err, "failed to save military advisor storage registry to database")
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn mission_giver_driver_147(
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
    // C `mission_giver_driver`: "Mister Jones", the governor's job-board
    // NPC (`src/area/32/missions.c`). The special-offer regen check runs
    // first, matching C's own single-function ordering - see
    // `regenerate_mission_giver_special_offers`'s doc comment.
    regenerate_mission_giver_special_offers(world, zone_loader);
    let mission_giver_facts = mission_giver_player_facts(runtime);
    let mission_giver_events = world.process_mission_giver_actions(
        &mission_giver_facts,
        config.area_id,
        current_unix_time().max(0) as u64,
    );
    let mission_giver_events_applied =
        apply_mission_giver_events(world, runtime, zone_loader, mission_giver_events);
    if mission_giver_events_applied != 0 {
        info!(
            mission_giver_events_applied,
            tick = world.tick.0,
            "applied mission giver dialogue events"
        );
    }
}
