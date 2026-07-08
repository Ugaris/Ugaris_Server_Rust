//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn fdemon_demon_driver_88(
    world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    config: &ServerConfig,
    _args: &Args,
    completed_actions: &[WorldActionCompletion],
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
    // C `ch_driver`'s `CDR_FDEMON_DEMON` case (`src/area/8/fdemon.c:
    // 3021-3023`) - the roaming Fire Demon/Fire Golem hunt AI. See
    // `world::npc::area8::fdemon_demon`'s module doc comment.
    let fdemon_demon_acted =
        world.process_fdemon_demon_actions_with_completions(config.area_id, completed_actions);
    if fdemon_demon_acted != 0 {
        info!(
            fdemon_demon_acted,
            tick = world.tick.0,
            "processed fdemon-demon hunt/gohome/wander actions"
        );
    }
}
