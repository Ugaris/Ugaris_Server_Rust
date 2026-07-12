//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn caligar_guard_driver_161(
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
    // C `guard_driver`: Eulc/Margana, the "backwards is the key to entry"
    // riddle guards at Caligar's gate (`src/area/36/caligar.c`).
    let caligar_guard_facts = caligar_guard_player_facts(runtime);
    let caligar_guard_events = world.process_caligar_guard_actions(
        &caligar_guard_facts,
        current_unix_time() as i32,
        config.area_id,
    );
    let caligar_guard_events_applied = apply_caligar_guard_events(runtime, caligar_guard_events);
    if caligar_guard_events_applied != 0 {
        info!(
            caligar_guard_events_applied,
            tick = world.tick.0,
            "applied caligar guard dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn caligar_guard2_driver_162(
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
    // C `guard2_driver`: a combat-capable Caligar guard that taunts
    // before falling through to plain `CDR_SIMPLEBADDY` AI
    // (`src/area/36/caligar.c`).
    let caligar_guard2_facts = caligar_guard2_player_facts(runtime);
    let caligar_guard2_events =
        world.process_caligar_guard2_actions(&caligar_guard2_facts, current_unix_time() as i32);
    let caligar_guard2_events_applied = apply_caligar_guard2_events(runtime, caligar_guard2_events);
    if caligar_guard2_events_applied != 0 {
        info!(
            caligar_guard2_events_applied,
            tick = world.tick.0,
            "applied caligar guard2 taunt events"
        );
    }
}
