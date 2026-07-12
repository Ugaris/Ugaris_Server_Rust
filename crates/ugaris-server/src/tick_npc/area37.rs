//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn nop_driver_167(
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
    // C `nop_driver`: the Fighting School's stationary background
    // "Student" NPCs (area 37, `src/area/37/arkhata.c`). `CDR_ARKHATAPRISON`
    // needs no pass here at all - it's a pure `CDR_SIMPLEBADDY` tail call,
    // already covered by the generic SimpleBaddy tick passes once its
    // driver gate was widened (`world::npc_fight`/`world::npc_idle`).
    world.process_nop_actions(config.area_id);
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn rammy_driver_168(
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
    // C `rammy_driver`: the ruler of Arkhata, quest 65 ("Rammy's Crown")
    // and quest 71 ("Entrance Passes") giver (area 37, `src/area/37/
    // arkhata.c`).
    let rammy_facts = rammy_player_facts(runtime);
    let rammy_events = world.process_rammy_actions(&rammy_facts, config.area_id);
    let rammy_events_applied = apply_rammy_events(world, runtime, zone_loader, rammy_events).await;
    if rammy_events_applied != 0 {
        info!(
            rammy_events_applied,
            tick = world.tick.0,
            "applied rammy dialogue events"
        );
    }
}
