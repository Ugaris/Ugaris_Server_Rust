//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn camhermit_driver_41(
    mut world: &mut World,
    mut runtime: &mut ServerRuntime,
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
    // C `camhermit_driver`: area 1's forest hermit quest NPC
    // (`src/area/1/gwendylon.c`).
    let camhermit_facts = camhermit_player_facts(&runtime);
    let camhermit_events = world.process_camhermit_actions(
        &camhermit_facts,
        current_unix_time() as i32,
        config.area_id,
    );
    let camhermit_events_applied = apply_camhermit_events(
        &mut world,
        &mut runtime,
        &achievement_repository,
        camhermit_events,
    )
    .await;
    if camhermit_events_applied != 0 {
        info!(
            camhermit_events_applied,
            tick = world.tick.0,
            "applied camp hermit dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn yoakin_driver_42(
    mut world: &mut World,
    mut runtime: &mut ServerRuntime,
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
    // C `yoakin_driver`: area 1's hunter/bear-hunt quest NPC
    // (`src/area/1/gwendylon.c`).
    let yoakin_facts = yoakin_player_facts(&world, &runtime);
    let yoakin_events =
        world.process_yoakin_actions(&yoakin_facts, current_unix_time() as i32, config.area_id);
    let yoakin_events_applied = apply_yoakin_events(
        &mut world,
        &mut runtime,
        &achievement_repository,
        yoakin_events,
    )
    .await;
    if yoakin_events_applied != 0 {
        info!(
            yoakin_events_applied,
            tick = world.tick.0,
            "applied yoakin dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn terion_driver_43(
    world: &mut World,
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
    // C `terion_driver`: area 1's ambient lore/storyteller NPC
    // (`src/area/1/gwendylon.c`).
    let terion_facts = terion_player_facts(&runtime);
    let terion_events = world.process_terion_actions(&terion_facts, config.area_id);
    let terion_events_applied = apply_terion_events(&mut runtime, terion_events);
    if terion_events_applied != 0 {
        info!(
            terion_events_applied,
            tick = world.tick.0,
            "applied terion dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn gwendylon_driver_44(
    mut world: &mut World,
    mut runtime: &mut ServerRuntime,
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
    // C `gwendylon_driver`: area 1's main quest-giver mage NPC
    // (`src/area/1/gwendylon.c`).
    let gwendylon_facts = gwendylon_player_facts(&runtime);
    let gwendylon_events = world.process_gwendylon_actions(
        &gwendylon_facts,
        current_unix_time() as i32,
        config.area_id,
    );
    let gwendylon_events_applied = apply_gwendylon_events(
        &mut world,
        &mut runtime,
        &achievement_repository,
        gwendylon_events,
    )
    .await;
    if gwendylon_events_applied != 0 {
        info!(
            gwendylon_events_applied,
            tick = world.tick.0,
            "applied gwendylon dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn gwendylon_driver_45(
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
    // `gwendylon_driver`'s `IID_CALIGARLETTER` cross-area
    // hand-off to area 36 (C `change_area(co, 36, 240, 10)`,
    // `src/area/1/gwendylon.c:637`), queued above when the
    // teleport letter is handed in.
    let gwendylon_cross_area_transfers_applied = apply_gwendylon_cross_area_transfers(
        &mut world,
        &mut runtime,
        &character_repository,
        &area_repository,
        config.area_id,
        config.mirror_id,
    )
    .await;
    if gwendylon_cross_area_transfers_applied != 0 {
        info!(
            gwendylon_cross_area_transfers_applied,
            tick = world.tick.0,
            "applied gwendylon cross-area transfers"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn greeter_driver_46(
    world: &mut World,
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
    // C `greeter_driver`: area 1's tutorial-town greeter NPC
    // (Cameron the Governor, `src/area/1/gwendylon.c`).
    let greeter_facts = greeter_player_facts(&runtime);
    let greeter_events =
        world.process_greeter_actions(&greeter_facts, current_unix_time() as i32, config.area_id);
    let greeter_events_applied = apply_greeter_events(&mut runtime, greeter_events);
    if greeter_events_applied != 0 {
        info!(
            greeter_events_applied,
            tick = world.tick.0,
            "applied greeter dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn jessica_driver_47(
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
    // C `jessica_driver`: area 1's robber-operations quest NPC
    // (`src/area/1/gwendylon.c`).
    let jessica_facts = jessica_player_facts(&runtime);
    let jessica_events =
        world.process_jessica_actions(&jessica_facts, current_unix_time() as i32, config.area_id);
    let jessica_events_applied = apply_jessica_events(&mut world, &mut runtime, jessica_events);
    if jessica_events_applied != 0 {
        info!(
            jessica_events_applied,
            tick = world.tick.0,
            "applied jessica dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn jiu_driver_48(
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
    // C `jiu_driver`: area 1's riverbeast quest-giving pilgrim
    // NPC (`src/area/1/gwendylon.c`).
    let jiu_facts = jiu_player_facts(&runtime);
    let jiu_events =
        world.process_jiu_actions(&jiu_facts, current_unix_time() as i32, config.area_id);
    let jiu_events_applied = apply_jiu_events(&mut world, &mut runtime, jiu_events);
    if jiu_events_applied != 0 {
        info!(
            jiu_events_applied,
            tick = world.tick.0,
            "applied jiu dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn forest_ranger_driver_49(
    world: &mut World,
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
    // C `forest_ranger_driver`: area 1's bear-attack warning
    // sentry NPC (`src/area/1/gwendylon.c`).
    let forest_ranger_facts = forest_ranger_player_facts(&runtime);
    let forest_ranger_events = world.process_forest_ranger_actions(
        &forest_ranger_facts,
        current_unix_time() as i32,
        config.area_id,
    );
    let forest_ranger_events_applied =
        apply_forest_ranger_events(&mut runtime, forest_ranger_events);
    if forest_ranger_events_applied != 0 {
        info!(
            forest_ranger_events_applied,
            tick = world.tick.0,
            "applied forest ranger dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn gate_welcome_driver_50(
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
    // C `gate_welcome_driver`: the Ishtar labyrinth gatekeeper
    // greeter NPC (`src/system/gatekeeper.c`).
    let gate_welcome_facts = gate_welcome_player_facts(&runtime);
    let gate_welcome_events =
        world.process_gate_welcome_actions(&gate_welcome_facts, config.area_id);
    let gate_welcome_events_applied = apply_gate_welcome_events(
        &mut runtime,
        &mut world,
        &mut zone_loader,
        gate_welcome_events,
    );
    if gate_welcome_events_applied != 0 {
        info!(
            gate_welcome_events_applied,
            tick = world.tick.0,
            "applied gate-welcome dialogue state updates"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn brithildie_driver_62(
    world: &mut World,
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
    // C `brithildie_driver`: area 1's Governor's-mother ambient lore
    // NPC (`src/area/1/gwendylon.c`).
    let brithildie_facts = brithildie_player_facts(&runtime);
    let brithildie_events = world.process_brithildie_actions(
        &brithildie_facts,
        current_unix_time() as i32,
        config.area_id,
    );
    let brithildie_events_applied = apply_brithildie_events(&mut runtime, brithildie_events);
    if brithildie_events_applied != 0 {
        info!(
            brithildie_events_applied,
            tick = world.tick.0,
            "applied brithildie dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn nook_driver_63(
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
    // C `nook_driver`: area 1's identity-crisis judge/knight/jester NPC
    // (`src/area/1/gwendylon.c`).
    let nook_facts = nook_player_facts(&runtime);
    let nook_events = world.process_nook_actions(&nook_facts, config.area_id);
    let nook_events_applied = apply_nook_events(&mut world, &mut runtime, nook_events);
    if nook_events_applied != 0 {
        info!(
            nook_events_applied,
            tick = world.tick.0,
            "applied nook dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn lydia_driver_64(
    mut world: &mut World,
    mut runtime: &mut ServerRuntime,
    mut zone_loader: &mut ZoneLoader,
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
    // C `lydia_driver`: area 1's hungover mage's-daughter quest NPC
    // (`src/area/1/gwendylon.c`).
    let lydia_facts = lydia_player_facts(&runtime);
    let lydia_events =
        world.process_lydia_actions(&lydia_facts, current_unix_time() as i32, config.area_id);
    let lydia_events_applied = apply_lydia_events(
        &mut world,
        &mut runtime,
        &mut zone_loader,
        &achievement_repository,
        lydia_events,
    )
    .await;
    if lydia_events_applied != 0 {
        info!(
            lydia_events_applied,
            tick = world.tick.0,
            "applied lydia dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn robber_driver_65(
    world: &mut World,
    _runtime: &mut ServerRuntime,
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
    // C `robber_driver`: area 1's midnight-meeting forest patrol NPC
    // (`src/area/1/gwendylon.c`).
    world.process_robber_actions(&mut zone_loader, config.area_id);
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn sanoa_driver_66(
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
    // C `sanoa_driver`: area 1's dialogue-free twelve-waypoint city walker
    // (`src/area/1/gwendylon.c`).
    world.process_sanoa_actions(config.area_id);
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn asturin_driver_67(
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
    // C `asturin_driver`: area 1's private-quarters guard NPC
    // (`src/area/1/gwendylon.c`).
    let asturin_facts = asturin_player_facts(runtime);
    let asturin_events =
        world.process_asturin_actions(&asturin_facts, current_unix_time() as i32, config.area_id);
    let asturin_events_applied = apply_asturin_events(runtime, asturin_events);
    if asturin_events_applied != 0 {
        info!(
            asturin_events_applied,
            tick = world.tick.0,
            "applied asturin dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn reskin_driver_68(
    mut world: &mut World,
    mut runtime: &mut ServerRuntime,
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
    // C `reskin_driver`: area 1's tavern-keeper/alchemy-turn-in NPC
    // (`src/area/1/gwendylon.c`).
    let reskin_facts = reskin_player_facts(&runtime);
    let reskin_events =
        world.process_reskin_actions(&reskin_facts, current_unix_time() as i32, config.area_id);
    let reskin_events_applied = apply_reskin_events(
        &mut world,
        &mut runtime,
        &achievement_repository,
        reskin_events,
    )
    .await;
    if reskin_events_applied != 0 {
        info!(
            reskin_events_applied,
            tick = world.tick.0,
            "applied reskin dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn guiwynn_driver_69(
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
    // C `guiwynn_driver`: area 1's town-mage "Order of Mages" quest NPC
    // (`src/area/1/gwendylon.c`).
    let guiwynn_facts = guiwynn_player_facts(&runtime);
    let guiwynn_events =
        world.process_guiwynn_actions(&guiwynn_facts, current_unix_time() as i32, config.area_id);
    let guiwynn_events_applied =
        apply_guiwynn_events(&mut world, &mut runtime, &mut zone_loader, guiwynn_events).await;
    if guiwynn_events_applied != 0 {
        info!(
            guiwynn_events_applied,
            tick = world.tick.0,
            "applied guiwynn dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn james_driver_70(
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
    // C `james_driver`: area 1's town-drunkard Lydia-quest hand-off/
    // hardcore-recruiter/paid-advice NPC (`src/area/1/gwendylon.c`).
    let james_facts = james_player_facts(runtime);
    let james_events = world.process_james_actions(&james_facts, config.area_id);
    let james_events_applied = apply_james_events(runtime, james_events);
    if james_events_applied != 0 {
        info!(
            james_events_applied,
            tick = world.tick.0,
            "applied james dialogue events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn balltrap_driver_71(
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
    // C `balltrap_skelly_driver`: area 1's stationary ball-trap-mechanism
    // guard skeleton (`src/area/1/gwendylon.c`).
    world.process_balltrap_actions(config.area_id);
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn logain_driver_72(
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
    // C `logain_driver`: area 1's retired knight-trainer quest-giver NPC,
    // the last driver in `ch_driver`'s dispatch table
    // (`src/area/1/gwendylon.c`).
    let logain_facts = logain_player_facts(&runtime);
    let logain_events =
        world.process_logain_actions(&logain_facts, current_unix_time() as i32, config.area_id);
    let logain_events_applied =
        apply_logain_events(&mut world, &mut runtime, &mut zone_loader, logain_events).await;
    if logain_events_applied != 0 {
        info!(
            logain_events_applied,
            tick = world.tick.0,
            "applied logain dialogue events"
        );
    }
}
