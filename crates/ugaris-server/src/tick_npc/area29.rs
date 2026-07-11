//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn spiritbran_driver_133(
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
    // C `spirit_brannington_driver`: the ghost NPC in Brannington who
    // explains the necromancer plot and runs "The Brannington Holy Relic"
    // (quest 44) (`src/area/29/brannington.c`).
    let spiritbran_facts = spiritbran_player_facts(runtime);
    let spiritbran_events = world.process_spiritbran_actions(&spiritbran_facts, config.area_id);
    let spiritbran_events_applied = apply_spiritbran_events(world, runtime, spiritbran_events);
    if spiritbran_events_applied != 0 {
        info!(
            spiritbran_events_applied,
            tick = world.tick.0,
            "applied spiritbran dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn guardbran_driver_140(
    world: &mut World,
    runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
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
    // C `guard_brannington_driver`: the town guard who greets new arrivals
    // and, once Count Brannington's family-heirloom chain is complete,
    // sends the player to investigate Arkhata for "Finding Arkhata" (quest
    // 64) (`src/area/29/brannington.c`).
    let guardbran_facts = guardbran_player_facts(runtime);
    let guardbran_events = world.process_guardbran_actions(&guardbran_facts, config.area_id);
    let guardbran_events_applied =
        apply_guardbran_events(world, runtime, achievement_repository, guardbran_events).await;
    if guardbran_events_applied != 0 {
        info!(
            guardbran_events_applied,
            tick = world.tick.0,
            "applied guardbran dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn brennethbran_driver_138(
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
    // C `brenneth_brannington_driver`: the memory-loss assassin NPC who
    // runs "A Grolm's Spoils"/"A Thief's Loot"/"A Necromancer's Notes"
    // (quests 41-43) (`src/area/29/brannington.c`).
    let brennethbran_facts = brennethbran_player_facts(runtime);
    let brennethbran_events =
        world.process_brennethbran_actions(&brennethbran_facts, config.area_id);
    let brennethbran_events_applied =
        apply_brennethbran_events(world, runtime, brennethbran_events);
    if brennethbran_events_applied != 0 {
        info!(
            brennethbran_events_applied,
            tick = world.tick.0,
            "applied brennethbran dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn countbran_driver_134(
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
    // C `count_brannington_driver`: Count Brannington, who runs "The
    // Jewels of Brannington" (quest 40) and hands out mausoleum keys
    // (`src/area/29/brannington.c`).
    let countbran_facts = countbran_player_facts(runtime);
    let countbran_events = world.process_countbran_actions(&countbran_facts, config.area_id);
    let countbran_events_applied =
        apply_countbran_events(world, runtime, zone_loader, countbran_events);
    if countbran_events_applied != 0 {
        info!(
            countbran_events_applied,
            tick = world.tick.0,
            "applied countbran dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn countessabran_driver_135(
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
    // C `countessa_brannington_driver`: the Count's wife, who hands out a
    // secondary quest-40 reward (`src/area/29/brannington.c`).
    let countessabran_facts = countessabran_player_facts(runtime);
    let countessabran_events =
        world.process_countessabran_actions(&countessabran_facts, config.area_id);
    let countessabran_events_applied = apply_countessabran_events(runtime, countessabran_events);
    if countessabran_events_applied != 0 {
        info!(
            countessabran_events_applied,
            tick = world.tick.0,
            "applied countessabran dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn daughterbran_driver_136(
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
    // C `daughter_brannington_driver`: the Count's daughter, who hands out
    // a secondary quest-40 reward (`src/area/29/brannington.c`).
    let daughterbran_facts = daughterbran_player_facts(runtime);
    let daughterbran_events =
        world.process_daughterbran_actions(&daughterbran_facts, config.area_id);
    let daughterbran_events_applied =
        apply_daughterbran_events(world, runtime, zone_loader, daughterbran_events);
    if daughterbran_events_applied != 0 {
        info!(
            daughterbran_events_applied,
            tick = world.tick.0,
            "applied daughterbran dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn forestbran_driver_137(
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
    // C `forest_brannington_driver`: the Brannington Forest hint giver who
    // decodes thief-mage treasure maps into dig locations, no quest of its
    // own (`src/area/29/brannington.c`).
    let forestbran_facts = forestbran_player_facts(runtime);
    let forestbran_events = world.process_forestbran_actions(&forestbran_facts, config.area_id);
    let forestbran_events_applied = apply_forestbran_events(runtime, forestbran_events);
    if forestbran_events_applied != 0 {
        info!(
            forestbran_events_applied,
            tick = world.tick.0,
            "applied forestbran dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn broklin_driver_139(
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
    // C `broklin_driver`: Brannington's Chief Miner, who runs "The Missing
    // Pickaxe"/"The Head Robber" (quests 45/46) and a permanent
    // gold<->silver trade service (`src/area/29/brannington.c`).
    let broklin_facts = broklin_player_facts(runtime);
    let broklin_events = world.process_broklin_actions(&broklin_facts, config.area_id);
    let broklin_events_applied = apply_broklin_events(world, runtime, zone_loader, broklin_events);
    if broklin_events_applied != 0 {
        info!(
            broklin_events_applied,
            tick = world.tick.0,
            "applied broklin dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn grinnich_driver_141(
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
    // C `grinnich_driver`: the hermit at the entrance of the Brannington
    // tower dungeon who hints at the buried tower and hands adventurers off
    // to Shanra in the basement (`src/area/29/brannington.c`).
    let grinnich_facts = grinnich_player_facts(runtime);
    let grinnich_events = world.process_grinnich_actions(&grinnich_facts, config.area_id);
    let grinnich_events_applied = apply_grinnich_events(runtime, grinnich_events);
    if grinnich_events_applied != 0 {
        info!(
            grinnich_events_applied,
            tick = world.tick.0,
            "applied grinnich dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn shanra_driver_142(
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
    // C `shanra_driver`: the storyteller in the Brannington tower dungeon's
    // basement who rewards the tower's sentinel gauntlet with the Grimoire
    // of Animation and teleports adventurers there and back
    // (`src/area/29/brannington.c`).
    let shanra_facts = shanra_player_facts(runtime);
    let shanra_events = world.process_shanra_actions(&shanra_facts, config.area_id);
    let shanra_events_applied = apply_shanra_events(runtime, shanra_events);
    if shanra_events_applied != 0 {
        info!(
            shanra_events_applied,
            tick = world.tick.0,
            "applied shanra dialogue events"
        );
    }
}
