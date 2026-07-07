//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn clanmaster_driver_9(
    mut world: &mut World,
    mut runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // C `clanmaster_driver`: the clan foundations NPC
    // (`src/area/30/clanmaster.c`).
    world.process_clanmaster_actions(config.area_id, current_unix_time());
    let clanmaster_events_applied = apply_clanmaster_events(
        &mut world,
        &mut runtime,
        &achievement_repository,
        &clan_log_repository,
        &character_repository,
        current_unix_time(),
    )
    .await;
    if clanmaster_events_applied != 0 {
        info!(
            clanmaster_events_applied,
            tick = world.tick.0,
            "applied clanmaster founding/membership events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn clanclerk_driver_10(
    mut world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    _character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // C `clanclerk_driver`: the clan administration/treasury
    // NPC (`src/area/30/clanmaster.c`).
    world.process_clanclerk_actions(config.area_id, current_unix_time());
    let clanclerk_events_applied =
        apply_clanclerk_events(&mut world, &clan_log_repository, current_unix_time()).await;
    if clanclerk_events_applied != 0 {
        info!(
            clanclerk_events_applied,
            tick = world.tick.0,
            "applied clanclerk treasury/admin events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn military_advisor_driver_36(
    mut world: &mut World,
    mut runtime: &mut ServerRuntime,
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
    // C `military_advisor_driver`: the paid mission-
    // recommendation Military Advisor NPC
    // (`src/module/military.c`).
    world.process_military_advisor_actions(config.area_id);
    let military_advisor_events_applied = apply_military_advisor_events(&mut world, &mut runtime);
    if military_advisor_events_applied != 0 {
        info!(
            military_advisor_events_applied,
            tick = world.tick.0,
            "applied military advisor favor/mission-recommendation events"
        );
    }
}
