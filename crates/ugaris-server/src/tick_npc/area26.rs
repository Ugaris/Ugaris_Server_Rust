//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn smugglecom_driver_121(
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
    // C `smugglecom_driver`: the Imperial Commander who runs the
    // Contraband quest chain below Aston 2 (`src/area/26/staffer.c`).
    let smugglecom_facts = smugglecom_player_facts(runtime);
    let smugglecom_events = world.process_smugglecom_actions(&smugglecom_facts, config.area_id);
    let smugglecom_events_applied = apply_smugglecom_events(world, runtime, smugglecom_events);
    if smugglecom_events_applied != 0 {
        info!(
            smugglecom_events_applied,
            tick = world.tick.0,
            "applied smugglecom dialogue events"
        );
    }
}
