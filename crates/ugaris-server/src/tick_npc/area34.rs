//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn teufelquest_driver_159(
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
    // C `teufelquest_driver`: the rat-hunt reward NPC in Teufelheim
    // (area 34, `src/area/34/teufel.c`).
    let teufelquest_facts = teufelquest_player_facts(runtime);
    let teufelquest_events =
        world.process_teufelquest_actions(zone_loader, &teufelquest_facts, config.area_id);
    let teufelquest_events_applied = apply_teufelquest_events(runtime, teufelquest_events);
    if teufelquest_events_applied != 0 {
        info!(
            teufelquest_events_applied,
            tick = world.tick.0,
            "applied teufelquest reward events"
        );
    }
}
