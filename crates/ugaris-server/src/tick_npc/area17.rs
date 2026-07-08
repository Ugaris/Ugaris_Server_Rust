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

#[allow(clippy::too_many_arguments)]
pub(crate) async fn two_alchemist_driver_98(
    mut world: &mut World,
    mut runtime: &mut ServerRuntime,
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
    // C `alchemist`: area 17's spider-poison quest giver "Cervik"
    // (`src/area/17/two.c`).
    let two_alchemist_facts = two_alchemist_player_facts(runtime);
    let two_alchemist_events =
        world.process_two_alchemist_actions(&two_alchemist_facts, config.area_id);
    let two_alchemist_events_applied =
        apply_two_alchemist_events(&mut world, &mut runtime, zone_loader, two_alchemist_events);
    if two_alchemist_events_applied != 0 {
        info!(
            two_alchemist_events_applied,
            tick = world.tick.0,
            "applied two-city alchemist dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn two_sanwyn_driver_99(
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
    // C `sanwyn`: area 17's military quest giver "Sanwyn"
    // (`src/area/17/two.c`).
    let two_sanwyn_facts = two_sanwyn_player_facts(runtime);
    let two_sanwyn_events = world.process_two_sanwyn_actions(&two_sanwyn_facts, config.area_id);
    let two_sanwyn_events_applied =
        apply_two_sanwyn_events(&mut world, &mut runtime, two_sanwyn_events);
    if two_sanwyn_events_applied != 0 {
        info!(
            two_sanwyn_events_applied,
            tick = world.tick.0,
            "applied two-city sanwyn dialogue events"
        );
    }
}
