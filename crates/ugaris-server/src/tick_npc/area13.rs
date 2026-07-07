//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn dungeonmaster_11(
    mut world: &mut World,
    mut runtime: &mut ServerRuntime,
    mut zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    _character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // C `dungeonmaster`: the clan-raid catacomb reception NPC
    // (`src/area/13/dungeon.c`) - `attack`/`enter`/`list`/
    // (GM-only) `destroy` text commands, the per-slot expiry/
    // warning tick, and the greeting.
    world.process_dungeonmaster_actions();
    let dungeonmaster_events_applied = apply_dungeonmaster_events(
        &mut world,
        &mut zone_loader,
        &mut runtime,
        &clan_log_repository,
        current_unix_time(),
    )
    .await;
    if dungeonmaster_events_applied != 0 {
        info!(
            dungeonmaster_events_applied,
            tick = world.tick.0,
            "applied dungeon-raid catacomb build events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn dungeondoor_12(
    mut world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    _character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // C `dungeondoor`'s `first_solve` jewel-steal clan-log
    // writes (`area/13/dungeon.c:1855-1891` via `clan.c:1343-
    // 1372`'s `'J'` chat-channel handler) - the economy
    // mutation/messages/notify already happened synchronously
    // in `World::resolve_dungeon_door_first_solve` whenever a
    // catacomb door was solved this tick; only the DB-backed
    // clan-log entries remain queued.
    let dungeon_jewel_steal_events_applied =
        apply_dungeon_jewel_steal_events(&mut world, &clan_log_repository, current_unix_time())
            .await;
    if dungeon_jewel_steal_events_applied != 0 {
        info!(
            dungeon_jewel_steal_events_applied,
            tick = world.tick.0,
            "applied dungeon catacomb jewel-steal clan-log events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn dungeonfighter_13(
    world: &mut World,
    _runtime: &mut ServerRuntime,
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
    // C `dungeonfighter`/`dungeon_potion`: the `CDR_DUNGEONFIGHTER`
    // warrior/mage/seyan raid-guard NPCs' potion-drinking driver
    // (`src/area/13/dungeon.c:1956-2161`).
    world.process_dungeonfighter_actions();
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn build_remove_tile_21(
    mut world: &mut World,
    mut runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // `build_remove_tile`'s evicted-player cross-area rescue
    // (C `change_area`, `area/13/dungeon.c:754`'s tail),
    // queued by `World::build_remove_tile` when the evicted
    // player's own `rest_area` differs from this area
    // server's own `area_id`.
    let dungeon_eviction_transfers_applied = apply_dungeon_eviction_transfers(
        &mut world,
        &mut runtime,
        &character_repository,
        &area_repository,
        config.area_id,
        config.mirror_id,
    )
    .await;
    if dungeon_eviction_transfers_applied != 0 {
        info!(
            dungeon_eviction_transfers_applied,
            tick = world.tick.0,
            "applied dungeon-eviction cross-area transfers"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn tick_clan_37(
    mut world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    _character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // C `tick_clan` states 3/4 (`clan.c:358-436,936-1182`):
    // the daily relation escalation/de-escalation tick, the
    // weekly treasury tick (bonus affordability, upkeep,
    // debt, bankrupt-clan deletion), and the hourly dungeon
    // training-score decay tick.
    let clan_economy_events_applied =
        apply_clan_economy_tick(&mut world, &clan_log_repository, current_unix_time()).await;
    if clan_economy_events_applied != 0 {
        info!(
            clan_economy_events_applied,
            tick = world.tick.0,
            "applied clan relation/treasury economy events"
        );
    }
}
