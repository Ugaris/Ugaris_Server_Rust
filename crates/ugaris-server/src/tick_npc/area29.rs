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
