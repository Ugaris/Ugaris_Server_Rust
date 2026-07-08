//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn forest_imp_driver_94(
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
    // C `imp_driver`: area 16's treasure-hinting forest imp
    // (`src/area/16/forest.c`).
    let imp_facts = forest_imp_player_facts(world, runtime);
    let imp_events = world.process_forest_imp_actions(&imp_facts, config.area_id);
    let imp_events_applied = apply_forest_imp_events(&mut world, &mut runtime, imp_events);
    if imp_events_applied != 0 {
        info!(
            imp_events_applied,
            tick = world.tick.0,
            "applied forest imp dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn forest_william_driver_95(
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
    // C `william_driver`: area 16's bear-hunt/mantis-stew quest giver
    // (`src/area/16/forest.c`).
    let william_facts = forest_william_player_facts(runtime);
    let william_events = world.process_forest_william_actions(&william_facts, config.area_id);
    let william_events_applied = apply_forest_william_events(
        &mut world,
        &mut runtime,
        achievement_repository,
        william_events,
    )
    .await;
    if william_events_applied != 0 {
        info!(
            william_events_applied,
            tick = world.tick.0,
            "applied forest william dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn forest_hermit_driver_96(
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
    // C `hermit_driver`: area 16's spider-queen quest giver
    // (`src/area/16/forest.c`).
    let hermit_facts = forest_hermit_player_facts(runtime);
    let hermit_events = world.process_forest_hermit_actions(&hermit_facts, config.area_id);
    let hermit_events_applied = apply_forest_hermit_events(&mut world, &mut runtime, hermit_events);
    if hermit_events_applied != 0 {
        info!(
            hermit_events_applied,
            tick = world.tick.0,
            "applied forest hermit dialogue events"
        );
    }
}
