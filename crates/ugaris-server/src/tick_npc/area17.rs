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

#[allow(clippy::too_many_arguments)]
pub(crate) async fn two_barkeeper_driver_100(
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
    // C `barkeeper`: area 17's tavern barkeeper/guest-pass broker
    // (`src/area/17/two.c`).
    let two_barkeeper_facts = two_barkeeper_player_facts(runtime);
    let two_barkeeper_events = world.process_two_barkeeper_actions(
        &two_barkeeper_facts,
        current_unix_time() as i32,
        config.area_id,
    );
    let two_barkeeper_events_applied = apply_two_barkeeper_events(runtime, two_barkeeper_events);
    if two_barkeeper_events_applied != 0 {
        info!(
            two_barkeeper_events_applied,
            tick = world.tick.0,
            "applied two-city barkeeper dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn two_guard_driver_101(
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
    // C `guard_driver`: Exkordon's territory-enforcement city guard
    // patrol (`src/area/17/two.c`).
    let two_guard_facts = two_guard_player_facts(runtime);
    let two_guard_events = world.process_two_guard_actions(
        &two_guard_facts,
        current_unix_time() as i32,
        zone_loader,
        config.area_id,
    );
    let two_guard_events_applied = apply_two_guard_events(runtime, two_guard_events);
    if two_guard_events_applied != 0 {
        info!(
            two_guard_events_applied,
            tick = world.tick.0,
            "applied two-city guard events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn two_servant_driver_102(
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
    // C `servant`: area 17's forbidden-territory palace maids/mistress/
    // governor's-double NPCs (`src/area/17/two.c`).
    let two_servant_facts = two_servant_player_facts(runtime);
    let two_servant_events = world.process_two_servant_actions(&two_servant_facts, config.area_id);
    let two_servant_events_applied =
        apply_two_servant_events(world, zone_loader, two_servant_events);
    if two_servant_events_applied != 0 {
        info!(
            two_servant_events_applied,
            tick = world.tick.0,
            "applied two-city servant events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn two_thiefguard_driver_103(
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
    // C `thiefguard`: the Exkordon thieves-guild entrance guard
    // (`src/area/17/two.c`).
    let two_thiefguard_facts = two_thiefguard_player_facts(runtime);
    let two_thiefguard_events =
        world.process_two_thiefguard_actions(&two_thiefguard_facts, config.area_id);
    let two_thiefguard_events_applied = apply_two_thiefguard_events(runtime, two_thiefguard_events);
    if two_thiefguard_events_applied != 0 {
        info!(
            two_thiefguard_events_applied,
            tick = world.tick.0,
            "applied two-city thiefguard dialogue events"
        );
    }
}
