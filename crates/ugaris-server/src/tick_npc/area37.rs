//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn nop_driver_167(
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
    // C `nop_driver`: the Fighting School's stationary background
    // "Student" NPCs (area 37, `src/area/37/arkhata.c`). `CDR_ARKHATAPRISON`
    // needs no pass here at all - it's a pure `CDR_SIMPLEBADDY` tail call,
    // already covered by the generic SimpleBaddy tick passes once its
    // driver gate was widened (`world::npc_fight`/`world::npc_idle`).
    world.process_nop_actions(config.area_id);
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn rammy_driver_168(
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
    // C `rammy_driver`: the ruler of Arkhata, quest 65 ("Rammy's Crown")
    // and quest 71 ("Entrance Passes") giver (area 37, `src/area/37/
    // arkhata.c`).
    let rammy_facts = rammy_player_facts(runtime);
    let rammy_events = world.process_rammy_actions(&rammy_facts, config.area_id);
    let rammy_events_applied = apply_rammy_events(world, runtime, zone_loader, rammy_events).await;
    if rammy_events_applied != 0 {
        info!(
            rammy_events_applied,
            tick = world.tick.0,
            "applied rammy dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn jaz_driver_169(
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
    // C `jaz_driver`: the Arkhata townsman who runs "Ishtar's Bracelet"
    // (quest 66) (area 37, `src/area/37/arkhata.c`).
    let jaz_facts = jaz_player_facts(runtime);
    let jaz_events = world.process_jaz_actions(&jaz_facts, config.area_id);
    let jaz_events_applied = apply_jaz_events(world, runtime, jaz_events).await;
    if jaz_events_applied != 0 {
        info!(
            jaz_events_applied,
            tick = world.tick.0,
            "applied jaz dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn fiona_driver_170(
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
    // C `fiona_driver`: the Fighting School headmistress, quest 67 ("The
    // Missing Ring") giver and student-challenge/skill-raise NPC (area 37,
    // `src/area/37/arkhata.c`).
    let fiona_facts = fiona_player_facts(runtime);
    let fiona_events = world.process_fiona_actions(&fiona_facts, config.area_id);
    let fiona_events_applied = apply_fiona_events(world, runtime, zone_loader, fiona_events).await;
    if fiona_events_applied != 0 {
        info!(
            fiona_events_applied,
            tick = world.tick.0,
            "applied fiona dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn bridgeguard_driver_171(
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
    // C `bridgeguard_driver`: the bridge-crossing guards outside Arkhata
    // (area 37, `src/area/37/arkhata.c`).
    world.process_bridgeguard_actions(config.area_id);
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn gladiator_driver_172(
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
    // C `gladiator_driver`: Fiona's disposable student opponents (area 37,
    // `src/area/37/arkhata.c`).
    world.process_gladiator_actions(config.area_id);
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn ramin_driver_173(
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
    // C `ramin_driver`: the Arkhata civil officer who runs "A Shopkeeper's
    // Fright" (quest 68) (area 37, `src/area/37/arkhata.c`).
    let ramin_facts = ramin_player_facts(runtime);
    let ramin_events = world.process_ramin_actions(&ramin_facts, config.area_id);
    let ramin_events_applied = apply_ramin_events(runtime, ramin_events).await;
    if ramin_events_applied != 0 {
        info!(
            ramin_events_applied,
            tick = world.tick.0,
            "applied ramin dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn arkhatamonk_driver_174(
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
    // C `arkhatamonk_driver`: the four monk personas (Gregor/Johan/
    // Johnatan/Tracy) sharing one dialogue state machine, quests 69/70/78
    // (area 37, `src/area/37/arkhata.c`).
    let monk_facts = arkhatamonk_player_facts(runtime);
    let monk_events = world.process_arkhatamonk_actions(&monk_facts, config.area_id);
    let monk_events_applied = apply_arkhatamonk_events(world, runtime, monk_events).await;
    if monk_events_applied != 0 {
        info!(
            monk_events_applied,
            tick = world.tick.0,
            "applied arkhatamonk dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn captain_driver_175(
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
    // C `captain_driver`: the Fortress Captain, first stop of the
    // entrance-pass-system chain (area 37, `src/area/37/arkhata.c`).
    let captain_facts = captain_player_facts(runtime);
    let captain_events = world.process_captain_actions(&captain_facts, config.area_id);
    let captain_events_applied = apply_captain_events(runtime, captain_events).await;
    if captain_events_applied != 0 {
        info!(
            captain_events_applied,
            tick = world.tick.0,
            "applied captain dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn judge_driver_176(
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
    // C `judge_driver`: the fortress judge who writes the entrance-pass
    // letters (area 37, `src/area/37/arkhata.c`).
    let judge_facts = judge_player_facts(runtime);
    let judge_events = world.process_judge_actions(&judge_facts, config.area_id);
    let judge_events_applied = apply_judge_events(world, runtime, zone_loader, judge_events).await;
    if judge_events_applied != 0 {
        info!(
            judge_events_applied,
            tick = world.tick.0,
            "applied judge dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn jada_driver_177(
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
    // C `jada_driver`: the Arkhata mystic who runs "The Source" (quest 72)
    // (area 37, `src/area/37/arkhata.c`).
    let jada_facts = jada_player_facts(runtime);
    let jada_events = world.process_jada_actions(&jada_facts, config.area_id);
    let jada_events_applied = apply_jada_events(world, runtime, jada_events).await;
    if jada_events_applied != 0 {
        info!(
            jada_events_applied,
            tick = world.tick.0,
            "applied jada dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn potmaker_driver_178(
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
    // C `potmaker_driver`: the Arkhata craftsman who runs "A Special Pot"
    // (quest 73) (area 37, `src/area/37/arkhata.c`).
    let potmaker_facts = potmaker_player_facts(runtime);
    let potmaker_events = world.process_potmaker_actions(&potmaker_facts, config.area_id);
    let potmaker_events_applied =
        apply_potmaker_events(world, runtime, zone_loader, potmaker_events).await;
    if potmaker_events_applied != 0 {
        info!(
            potmaker_events_applied,
            tick = world.tick.0,
            "applied potmaker dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn hunter_driver_179(
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
    // C `hunter_driver`: the Arkhata hunter who runs "The Blue Harpy"
    // (quest 77) (area 37, `src/area/37/arkhata.c`).
    let hunter_facts = hunter_player_facts(runtime);
    let hunter_events = world.process_hunter_actions(&hunter_facts, config.area_id);
    let hunter_events_applied = apply_hunter_events(world, runtime, hunter_events).await;
    if hunter_events_applied != 0 {
        info!(
            hunter_events_applied,
            tick = world.tick.0,
            "applied hunter dialogue events"
        );
    }
}
