//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn astro1_driver_77(
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
    // C `astro1_driver`: area 3's ambient moon-telescope astronomer NPC
    // (`src/area/3/area3.c`).
    world.process_astro1_actions(config.area_id);
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn astro2_driver_80(
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
    // C `astro2_driver`: area 3's lost-astronomer's-notes quest giver NPC
    // (`src/area/3/area3.c`).
    let astro2_facts = astro2_player_facts(runtime);
    let astro2_events = world.process_astro2_actions(&astro2_facts, config.area_id);
    let astro2_events_applied = apply_astro2_events(world, runtime, zone_loader, astro2_events);
    if astro2_events_applied != 0 {
        info!(
            astro2_events_applied,
            tick = world.tick.0,
            "applied astro2 dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn thomas_driver_78(
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
    // C `thomas_driver`: area 3's crypt entrance guard NPC
    // (`src/area/3/area3.c`).
    let thomas_facts = thomas_player_facts(world, runtime);
    let thomas_events = world.process_thomas_actions(&thomas_facts, config.area_id);
    let thomas_events_applied = apply_thomas_events(runtime, thomas_events);
    if thomas_events_applied != 0 {
        info!(
            thomas_events_applied,
            tick = world.tick.0,
            "applied thomas dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn sir_jones_driver_79(
    mut world: &mut World,
    mut runtime: &mut ServerRuntime,
    mut zone_loader: &mut ZoneLoader,
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
    // C `sir_jones_driver`: area 3's crypt quest giver NPC
    // (`src/area/3/area3.c`).
    let sir_jones_facts = sir_jones_player_facts(runtime);
    let sir_jones_events = world.process_sir_jones_actions(&sir_jones_facts, config.area_id);
    let sir_jones_events_applied =
        apply_sir_jones_events(&mut world, &mut runtime, &mut zone_loader, sir_jones_events).await;
    if sir_jones_events_applied != 0 {
        info!(
            sir_jones_events_applied,
            tick = world.tick.0,
            "applied sir jones dialogue events"
        );
    }
}
