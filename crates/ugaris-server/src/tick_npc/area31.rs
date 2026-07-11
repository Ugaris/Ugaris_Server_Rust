//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn dwarfchief_driver_143(
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
    // C `dwarfchief_driver`: Grimroot's leader, who runs "A Miner's
    // Misery"/"A Miner's Bane"/"A Miner's Anguish"/"A Miner Lost" (quests
    // 47-50) (`src/area/31/warrmines.c`).
    let dwarfchief_facts = dwarfchief_player_facts(runtime);
    let dwarfchief_events = world.process_dwarfchief_actions(&dwarfchief_facts, config.area_id);
    let dwarfchief_events_applied =
        apply_dwarfchief_events(world, runtime, zone_loader, dwarfchief_events);
    if dwarfchief_events_applied != 0 {
        info!(
            dwarfchief_events_applied,
            tick = world.tick.0,
            "applied dwarfchief dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn lostdwarf_driver_144(
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
    // C `lostdwarf_driver`: the four missing miners `dwarfchief_driver`'s
    // quest chain sends the player to rescue (`src/area/31/warrmines.c`).
    let lostdwarf_facts = lostdwarf_player_facts(runtime);
    let lostdwarf_events = world.process_lostdwarf_actions(&lostdwarf_facts, config.area_id);
    let lostdwarf_events_applied = apply_lostdwarf_events(runtime, lostdwarf_events);
    if lostdwarf_events_applied != 0 {
        info!(
            lostdwarf_events_applied,
            tick = world.tick.0,
            "applied lostdwarf rescue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn dwarfshaman_driver_145(
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
    // C `dwarfshaman_driver`: Grimroot's shaman, who runs "Lizard's
    // Teeth"/"Collecting Berries"/"Elitist Head" (quests 51-53)
    // (`src/area/31/warrmines.c`).
    let dwarfshaman_facts = dwarfshaman_player_facts(runtime);
    let dwarfshaman_events = world.process_dwarfshaman_actions(&dwarfshaman_facts, config.area_id);
    let dwarfshaman_events_applied = apply_dwarfshaman_events(world, runtime, dwarfshaman_events);
    if dwarfshaman_events_applied != 0 {
        info!(
            dwarfshaman_events_applied,
            tick = world.tick.0,
            "applied dwarfshaman dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn dwarfsmith_driver_146(
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
    // C `dwarfsmith_driver`: Grimroot's blacksmith, who forges a
    // `lizard_elite_keyN` from a mold plus 5,000 silver
    // (`src/area/31/warrmines.c`).
    let dwarfsmith_facts = dwarfsmith_player_facts(runtime);
    let dwarfsmith_events = world.process_dwarfsmith_actions(&dwarfsmith_facts, config.area_id);
    let dwarfsmith_events_applied =
        apply_dwarfsmith_events(world, runtime, zone_loader, dwarfsmith_events);
    if dwarfsmith_events_applied != 0 {
        info!(
            dwarfsmith_events_applied,
            tick = world.tick.0,
            "applied dwarfsmith forge events"
        );
    }
}
