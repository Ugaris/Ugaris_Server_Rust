//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn lqnpc_driver_107(
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
    // C `lqnpc`: area 20's admin-authored Live Quest NPCs
    // (`src/area/20/lq.c`).
    let events = world.process_lqnpc_actions(config.area_id);
    if !events.is_empty() {
        let applied = crate::area20::apply_lqnpc_events(world, runtime, zone_loader, events);
        if applied != 0 {
            info!(
                applied,
                tick = world.tick.0,
                "applied lqnpc dialogue events"
            );
        }
    }
}
