//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn golemkeyhold_driver_92(
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
    // C `keyhold_fight_driver`: area 12's mine-vault keyholder golem
    // (`src/area/12/mine.c`), spawned by
    // `crate::mine::spawn_keyholder_golem`. No death-reward hook exists
    // (C's own `ch_died_driver` case for this driver is a no-op `return
    // 1;`), unlike `gate_fight_driver_51` above.
    let golemkeyhold_acted = world.process_golemkeyhold_actions(config.area_id);
    if golemkeyhold_acted != 0 {
        info!(
            golemkeyhold_acted,
            tick = world.tick.0,
            "processed keyholder-golem actions"
        );
    }
}
