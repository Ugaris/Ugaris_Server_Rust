//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn strategy_boss_driver_118(
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
    // C `ch_driver`'s `CDR_STRATEGY_BOSS` case (`src/area/23_24/
    // strategy.c:1614-1616`) - Cinciac's mission-giver dialogue chain. See
    // `world::npc::area23_24::boss`'s module doc comment.
    let strategy_boss_applied = crate::area23_24::apply_strategy_boss_tick(world, runtime, config);
    if strategy_boss_applied != 0 {
        info!(
            strategy_boss_applied,
            tick = world.tick.0,
            "applied strategy-boss dialogue events"
        );
    }

    // C `ch_driver`'s `CDR_STRATEGY` case (`src/area/23_24/strategy.c:
    // 1611-1613`) - every live worker/fighter/miner's per-tick body. See
    // `world::npc::area23_24::worker`'s module doc comment for why no
    // live worker can exist yet (still-unported spawning).
    let strategy_worker_acted = crate::area23_24::apply_strategy_worker_tick(world, config);
    if strategy_worker_acted != 0 {
        info!(
            strategy_worker_acted,
            tick = world.tick.0,
            "applied strategy-worker actions"
        );
    }
}
