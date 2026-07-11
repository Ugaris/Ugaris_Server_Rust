//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn aristocrat_driver_131(
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
    // C `aristocrat_driver`: the robbed noble in Brannington Forest who
    // runs "The Family Heirloom" (quest 38) (`src/area/28/
    // brannington_forest.c`).
    let aristocrat_facts = aristocrat_player_facts(runtime);
    let aristocrat_events = world.process_aristocrat_actions(&aristocrat_facts, config.area_id);
    let aristocrat_events_applied =
        apply_aristocrat_events(world, runtime, zone_loader, aristocrat_events);
    if aristocrat_events_applied != 0 {
        info!(
            aristocrat_events_applied,
            tick = world.tick.0,
            "applied aristocrat dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn yoatin_driver_132(
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
    // C `yoatin_driver`: the timid hunter in Brannington Forest who runs
    // "Bear Hunt - Again" (quest 39) (`src/area/28/brannington_forest.c`).
    let yoatin_facts = yoatin_player_facts(runtime);
    let yoatin_events = world.process_yoatin_actions(&yoatin_facts, config.area_id);
    let yoatin_events_applied = apply_yoatin_events(world, runtime, zone_loader, yoatin_events);
    if yoatin_events_applied != 0 {
        info!(
            yoatin_events_applied,
            tick = world.tick.0,
            "applied yoatin dialogue events"
        );
    }
}
