//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn caligar_guard_driver_161(
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
    // C `guard_driver`: Eulc/Margana, the "backwards is the key to entry"
    // riddle guards at Caligar's gate (`src/area/36/caligar.c`).
    let caligar_guard_facts = caligar_guard_player_facts(runtime);
    let caligar_guard_events = world.process_caligar_guard_actions(
        &caligar_guard_facts,
        current_unix_time() as i32,
        config.area_id,
    );
    let caligar_guard_events_applied = apply_caligar_guard_events(runtime, caligar_guard_events);
    if caligar_guard_events_applied != 0 {
        info!(
            caligar_guard_events_applied,
            tick = world.tick.0,
            "applied caligar guard dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn caligar_guard2_driver_162(
    world: &mut World,
    runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
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
    // C `guard2_driver`: a combat-capable Caligar guard that taunts
    // before falling through to plain `CDR_SIMPLEBADDY` AI
    // (`src/area/36/caligar.c`).
    let caligar_guard2_facts = caligar_guard2_player_facts(runtime);
    let caligar_guard2_events =
        world.process_caligar_guard2_actions(&caligar_guard2_facts, current_unix_time() as i32);
    let caligar_guard2_events_applied = apply_caligar_guard2_events(runtime, caligar_guard2_events);
    if caligar_guard2_events_applied != 0 {
        info!(
            caligar_guard2_events_applied,
            tick = world.tick.0,
            "applied caligar guard2 taunt events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn caligar_glori_driver_163(
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
    // C `glori_driver`: "First in charge" of the library, who runs the
    // quest-54-58 obelisk/key-part chain (`src/area/36/caligar.c`).
    let caligar_glori_facts = caligar_glori_player_facts(runtime);
    let caligar_glori_events = world.process_caligar_glori_actions(
        &caligar_glori_facts,
        current_unix_time() as i32,
        config.area_id,
    );
    let caligar_glori_events_applied =
        apply_caligar_glori_events(world, runtime, caligar_glori_events);
    if caligar_glori_events_applied != 0 {
        info!(
            caligar_glori_events_applied,
            tick = world.tick.0,
            "applied caligar glori dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn caligar_arquin_driver_164(
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
    // C `arquin_driver`: stationed outside the library, explains the
    // obelisks/dungeon key and points the player at Homden
    // (`src/area/36/caligar.c`).
    let caligar_arquin_facts = caligar_arquin_player_facts(runtime);
    let caligar_arquin_events = world.process_caligar_arquin_actions(
        &caligar_arquin_facts,
        current_unix_time() as i32,
        config.area_id,
    );
    let caligar_arquin_events_applied = apply_caligar_arquin_events(runtime, caligar_arquin_events);
    if caligar_arquin_events_applied != 0 {
        info!(
            caligar_arquin_events_applied,
            tick = world.tick.0,
            "applied caligar arquin dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn caligar_smith_driver_165(
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
    // C `smith_driver`: the dwarf blacksmith who forges the underground
    // key for 5,000 gold and later sells a dictionary for 10,000 gold
    // (`src/area/36/caligar.c`).
    let caligar_smith_facts = caligar_smith_player_facts(runtime);
    let caligar_smith_events = world.process_caligar_smith_actions(
        &caligar_smith_facts,
        current_unix_time() as i32,
        config.area_id,
    );
    let caligar_smith_events_applied =
        apply_caligar_smith_events(world, runtime, zone_loader, caligar_smith_events);
    if caligar_smith_events_applied != 0 {
        info!(
            caligar_smith_events_applied,
            tick = world.tick.0,
            "applied caligar smith forge/purchase events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn caligar_homden_driver_166(
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
    // C `homden_driver`: the banished Carmin Clan brother who opens quest
    // 59 (find his stolen ring) and narrates the palace/Emperor backstory
    // (`src/area/36/caligar.c`).
    let caligar_homden_facts = caligar_homden_player_facts(runtime);
    let caligar_homden_events = world.process_caligar_homden_actions(
        &caligar_homden_facts,
        current_unix_time() as i32,
        config.area_id,
    );
    let caligar_homden_events_applied =
        apply_caligar_homden_events(world, runtime, caligar_homden_events);
    if caligar_homden_events_applied != 0 {
        info!(
            caligar_homden_events_applied,
            tick = world.tick.0,
            "applied caligar homden dialogue events"
        );
    }
}
