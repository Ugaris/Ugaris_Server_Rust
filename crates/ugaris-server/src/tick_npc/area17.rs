//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn two_skelly_driver_97(
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
    // C `skelly`: area 17's raised-skeleton governor's-ghost quest giver
    // (`src/area/17/two.c`).
    let two_skelly_facts = two_skelly_player_facts(runtime);
    let two_skelly_events = world.process_two_skelly_actions(&two_skelly_facts, config.area_id);
    let two_skelly_events_applied =
        apply_two_skelly_events(&mut world, &mut runtime, two_skelly_events);
    if two_skelly_events_applied != 0 {
        info!(
            two_skelly_events_applied,
            tick = world.tick.0,
            "applied two-city skelly dialogue events"
        );
    }
}
