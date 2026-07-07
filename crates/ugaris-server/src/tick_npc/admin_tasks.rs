//! Tick passes extracted from `main()`; called in the
//! original order by `tick_npc::run_all`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn lastseen_15(
    mut world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
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
    // `/lastseen <name>`'s async DB round-trip (C `lastseen`/
    // `db_lastseen`, `database_lookup.c:142-157` +
    // `database_notes.c:352-390`), queued by
    // `apply_lastseen_command` above.
    let lastseen_events_applied =
        apply_lastseen_events(&mut world, &character_repository, current_unix_time()).await;
    if lastseen_events_applied != 0 {
        info!(
            lastseen_events_applied,
            tick = world.tick.0,
            "applied /lastseen lookups"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn acstatus_16(
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
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    _notes_repository: &Option<ugaris_db::PgNotesRepository>,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // `#acstatus`/`#acstats`/`#aclist`'s async DB round-trips
    // (C's synchronous in-memory `player[nr]->ac` struct read
    // in this codebase's architecture - see `ugaris-core`'s
    // `world/anticheat.rs` module doc comment), queued by
    // `apply_admin_character_command` above.
    let ac_status_events_applied = apply_ac_status_events(&mut world, &anticheat_repository).await;
    if ac_status_events_applied != 0 {
        info!(
            ac_status_events_applied,
            tick = world.tick.0,
            "applied #acstatus lookups"
        );
    }
    let ac_stats_events_applied = apply_ac_stats_events(&mut world, &anticheat_repository).await;
    if ac_stats_events_applied != 0 {
        info!(
            ac_stats_events_applied,
            tick = world.tick.0,
            "applied #acstats lookups"
        );
    }
    let ac_list_events_applied = apply_ac_list_events(&mut world, &anticheat_repository).await;
    if ac_list_events_applied != 0 {
        info!(
            ac_list_events_applied,
            tick = world.tick.0,
            "applied #aclist lookups"
        );
    }
    let ac_suspicious_events_applied =
        apply_ac_suspicious_events(&mut world, &anticheat_repository).await;
    if ac_suspicious_events_applied != 0 {
        info!(
            ac_suspicious_events_applied,
            tick = world.tick.0,
            "applied #acsuspicious lookups"
        );
    }
    let ac_cleanup_events_applied =
        apply_ac_cleanup_events(&mut world, &anticheat_repository).await;
    if ac_cleanup_events_applied != 0 {
        info!(
            ac_cleanup_events_applied,
            tick = world.tick.0,
            "applied #accleanup lookups"
        );
    }
    let ac_reset_events_applied = apply_ac_reset_events(&mut world, &anticheat_repository).await;
    if ac_reset_events_applied != 0 {
        info!(
            ac_reset_events_applied,
            tick = world.tick.0,
            "applied #acreset lookups"
        );
    }
    let ac_flag_events_applied = apply_ac_flag_events(&mut world, &anticheat_repository).await;
    if ac_flag_events_applied != 0 {
        info!(
            ac_flag_events_applied,
            tick = world.tick.0,
            "applied #acflag lookups"
        );
    }
    let ac_unflag_events_applied = apply_ac_unflag_events(&mut world, &anticheat_repository).await;
    if ac_unflag_events_applied != 0 {
        info!(
            ac_unflag_events_applied,
            tick = world.tick.0,
            "applied #acunflag lookups"
        );
    }
    let ac_trust_events_applied = apply_ac_trust_events(&mut world, &anticheat_repository).await;
    if ac_trust_events_applied != 0 {
        info!(
            ac_trust_events_applied,
            tick = world.tick.0,
            "applied #actrust lookups"
        );
    }
    let ac_untrust_events_applied =
        apply_ac_untrust_events(&mut world, &anticheat_repository).await;
    if ac_untrust_events_applied != 0 {
        info!(
            ac_untrust_events_applied,
            tick = world.tick.0,
            "applied #acuntrust lookups"
        );
    }
    let ac_warn_events_applied = apply_ac_warn_events(&mut world, &anticheat_repository).await;
    if ac_warn_events_applied != 0 {
        info!(
            ac_warn_events_applied,
            tick = world.tick.0,
            "applied #acwarn lookups"
        );
    }
    let ac_sessions_events_applied =
        apply_ac_sessions_events(&mut world, &anticheat_repository).await;
    if ac_sessions_events_applied != 0 {
        info!(
            ac_sessions_events_applied,
            tick = world.tick.0,
            "applied #acsessions lookups"
        );
    }
    let ac_violations_events_applied =
        apply_ac_violations_events(&mut world, &anticheat_repository).await;
    if ac_violations_events_applied != 0 {
        info!(
            ac_violations_events_applied,
            tick = world.tick.0,
            "applied #acviolations lookups"
        );
    }
    let ac_history_events_applied =
        apply_ac_history_events(&mut world, &anticheat_repository).await;
    if ac_history_events_applied != 0 {
        info!(
            ac_history_events_applied,
            tick = world.tick.0,
            "applied #achistory lookups"
        );
    }
    let ac_sharedip_events_applied =
        apply_ac_sharedip_events(&mut world, &anticheat_repository).await;
    if ac_sharedip_events_applied != 0 {
        info!(
            ac_sharedip_events_applied,
            tick = world.tick.0,
            "applied #acsharedip lookups"
        );
    }
    let ac_sharedhw_events_applied =
        apply_ac_sharedhw_events(&mut world, &anticheat_repository).await;
    if ac_sharedhw_events_applied != 0 {
        info!(
            ac_sharedhw_events_applied,
            tick = world.tick.0,
            "applied #acsharedhw lookups"
        );
    }
    let ac_highrisk_events_applied =
        apply_ac_highrisk_events(&mut world, &anticheat_repository).await;
    if ac_highrisk_events_applied != 0 {
        info!(
            ac_highrisk_events_applied,
            tick = world.tick.0,
            "applied #achighrisk lookups"
        );
    }
    let ac_lookup_events_applied = apply_ac_lookup_events(&mut world, &anticheat_repository).await;
    if ac_lookup_events_applied != 0 {
        info!(
            ac_lookup_events_applied,
            tick = world.tick.0,
            "applied #aclookup lookups"
        );
    }
    let ac_siglist_events_applied =
        apply_ac_siglist_events(&mut world, &anticheat_repository).await;
    if ac_siglist_events_applied != 0 {
        info!(
            ac_siglist_events_applied,
            tick = world.tick.0,
            "applied #acsiglist lookups"
        );
    }
    let ac_sigadd_events_applied = apply_ac_sigadd_events(&mut world, &anticheat_repository).await;
    if ac_sigadd_events_applied != 0 {
        info!(
            ac_sigadd_events_applied,
            tick = world.tick.0,
            "applied #acsigadd lookups"
        );
    }
    let ac_sigdel_events_applied = apply_ac_sigdel_events(&mut world, &anticheat_repository).await;
    if ac_sigdel_events_applied != 0 {
        info!(
            ac_sigdel_events_applied,
            tick = world.tick.0,
            "applied #acsigdel lookups"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn querystats_17(
    mut world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
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
    // `#querystats`/`/querystats`'s round trip against the
    // live `PgCharacterRepository`'s in-memory counters -
    // see `ugaris-core`'s `world/querystats.rs` module doc
    // comment, queued by `apply_admin_character_command`
    // above.
    let querystats_events_applied = apply_querystats_events(&mut world, &character_repository);
    if querystats_events_applied != 0 {
        info!(
            querystats_events_applied,
            tick = world.tick.0,
            "applied /querystats lookups"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn jail_18(
    mut world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
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
    // `/jail`/`/unjail <name>`'s async DB round-trip (C
    // `lookup_name`, `system/lookup.c:42-98` + `system/
    // database/database_lookup.c:57-83`), queued by
    // `apply_admin_character_command` above.
    let jail_events_applied = apply_jail_events(&mut world, &character_repository).await;
    if jail_events_applied != 0 {
        info!(
            jail_events_applied,
            tick = world.tick.0,
            "applied /jail lookups"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn jail_19(
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
    // `/jail`/`/unjail`'s cross-area hand-off (C
    // `change_area`, `tool.c:4392-4425`'s tail), queued by
    // `World::apply_jail_action` above when the destination
    // area differs from this area server's own `area_id`.
    let jail_cross_area_transfers_applied = apply_jail_cross_area_transfers(
        &mut world,
        &mut runtime,
        &character_repository,
        &area_repository,
        config.area_id,
        config.mirror_id,
    )
    .await;
    if jail_cross_area_transfers_applied != 0 {
        info!(
            jail_cross_area_transfers_applied,
            tick = world.tick.0,
            "applied /jail cross-area transfers"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn change_area_20(
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
    // The Macro Daemon's cross-server "challenge room"
    // hand-off (C `change_area`, `src/module/base.c:1110`/
    // `848-850`), queued by `apply_macro_events` above when
    // the challenge-room/original-area destination differs
    // from this area server's own `area_id`.
    let macro_cross_area_transfers_applied = apply_macro_cross_area_transfers(
        &mut world,
        &mut runtime,
        &character_repository,
        &area_repository,
        config.area_id,
        config.mirror_id,
    )
    .await;
    if macro_cross_area_transfers_applied != 0 {
        info!(
            macro_cross_area_transfers_applied,
            tick = world.tick.0,
            "applied Macro Daemon cross-area challenge-room transfers"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn rmdeath_22(
    mut world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
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
    // `/rmdeath <name>`'s async DB round-trip (C
    // `lookup_name`, `system/lookup.c:42-98` + `system/
    // database/database_lookup.c:57-83`), queued by
    // `apply_admin_character_command` above.
    let rmdeath_events_applied = apply_rmdeath_events(&mut world, &character_repository).await;
    if rmdeath_events_applied != 0 {
        info!(
            rmdeath_events_applied,
            tick = world.tick.0,
            "applied /rmdeath lookups"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn complain_23(
    mut world: &mut World,
    mut runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
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
    // `/complain <name>`'s async DB round-trip (C
    // `cmd_complain`'s `lookup_name`/`db_lookup_name`,
    // `system/lookup.c:42-98` + `system/database/
    // database_lookup.c:57-83`), queued by
    // `apply_complain_command` above.
    let complain_events_applied = apply_complain_events(
        &mut world,
        &mut runtime,
        &character_repository,
        current_unix_time(),
    )
    .await;
    if complain_events_applied != 0 {
        info!(
            complain_events_applied,
            tick = world.tick.0,
            "applied /complain lookups"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn god_24(
    mut world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
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
    // `/god`/`/setsir`/`/staff`/`/emaster`/`/devel`/
    // `/hardcore`/`/qmaster`'s async DB round-trip (C
    // `cmd_flag`'s offline fallback, `task_set_flags`/
    // `set_flags`, `task.c:198-211,385-394`), queued by
    // `World::apply_cmd_flag_command` above.
    let admin_flag_events_applied =
        apply_admin_flag_events(&mut world, &character_repository).await;
    if admin_flag_events_applied != 0 {
        info!(
            admin_flag_events_applied,
            tick = world.tick.0,
            "applied admin flag-toggle events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn rename_25(
    mut world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
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
    // `/rename <from> <to>`'s async DB round trip (C
    // `do_rename`/`db_rename`, `src/system/database/
    // database_admin.c:291-355`), queued by
    // `World::queue_rename_command` above.
    let rename_events_applied = apply_rename_events(&mut world, &character_repository).await;
    if rename_events_applied != 0 {
        info!(
            rename_events_applied,
            tick = world.tick.0,
            "applied /rename lookups"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn lockname_26(
    mut world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
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
    // `/lockname`/`/unlockname <name>`'s async DB round trip
    // (C `do_lockname`/`do_unlockname`, `src/system/database/
    // database_admin.c:357-434`), queued by `World::
    // queue_lockname_command`/`queue_unlockname_command`
    // above.
    let lockname_events_applied = apply_lockname_events(&mut world, &character_repository).await;
    if lockname_events_applied != 0 {
        info!(
            lockname_events_applied,
            tick = world.tick.0,
            "applied /lockname lookups"
        );
    }
    let unlockname_events_applied =
        apply_unlockname_events(&mut world, &character_repository).await;
    if unlockname_events_applied != 0 {
        info!(
            unlockname_events_applied,
            tick = world.tick.0,
            "applied /unlockname lookups"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn punish_27(
    mut world: &mut World,
    mut runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // `/punish <name> <level> <reason>`'s async DB round trip
    // (C `task_punish_player`/`punish_player`/`punish`,
    // `src/system/task.c` + `src/system/punish.c`), queued by
    // `World::queue_punish_command` above.
    let punish_events_applied = apply_punish_events(
        &mut world,
        &mut runtime,
        &character_repository,
        &notes_repository,
        current_unix_time(),
    )
    .await;
    if punish_events_applied != 0 {
        info!(
            punish_events_applied,
            tick = world.tick.0,
            "applied /punish events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn unpunish_28(
    mut world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // `/unpunish <name> <note id>`'s async DB round trip (C
    // `task_unpunish_player`/`unpunish_player`/`unpunish`,
    // `src/system/task.c` + `src/system/punish.c`), queued by
    // `World::queue_unpunish_command` above.
    let unpunish_events_applied =
        apply_unpunish_events(&mut world, &character_repository, &notes_repository).await;
    if unpunish_events_applied != 0 {
        info!(
            unpunish_events_applied,
            tick = world.tick.0,
            "applied /unpunish events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn exterminate_29(
    mut world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
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
    // `/exterminate <name>`'s async DB round trip (C
    // `exterminate`/`db_exterminate`, `src/system/database/
    // database_admin.c:29-95,503-507`), queued by
    // `World::queue_exterminate_command` above.
    let exterminate_events_applied =
        apply_exterminate_events(&mut world, &character_repository).await;
    if exterminate_events_applied != 0 {
        info!(
            exterminate_events_applied,
            tick = world.tick.0,
            "applied /exterminate events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn look_30(
    mut world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // `/look <name>`'s async DB round trip (C `read_notes`/
    // `db_read_notes`/`list_punishment`), queued by
    // `World::queue_look_command` above.
    let look_events_applied =
        apply_look_events(&mut world, &character_repository, &notes_repository).await;
    if look_events_applied != 0 {
        info!(
            look_events_applied,
            tick = world.tick.0,
            "applied /look events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn klog_31(
    mut world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    _area_repository: &Option<ugaris_db::PgAreaRepository>,
    _clan_repository: &Option<ugaris_db::PgClanRegistryRepository>,
    _clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    _merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    _military_master_storage_repository: &Option<ugaris_db::PgMilitaryMasterStorageRepository>,
    _military_advisor_storage_repository: &Option<ugaris_db::PgMilitaryAdvisorStorageRepository>,
    notes_repository: &Option<ugaris_db::PgNotesRepository>,
    _anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
    _auction_repository: &Option<ugaris_db::PgAuctionRepository>,
) {
    // `/klog`'s async DB round trip (C `karmalog`/
    // `db_karmalog`/`karmalog_s`), queued by
    // `World::queue_klog_command` above.
    let klog_events_applied = apply_klog_events(
        &mut world,
        &character_repository,
        &notes_repository,
        current_unix_time(),
    )
    .await;
    if klog_events_applied != 0 {
        info!(
            klog_events_applied,
            tick = world.tick.0,
            "applied /klog events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn showvalues_32(
    mut world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
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
    // `/showvalues <name>`'s async DB round trip (C
    // `show_values`/`show_values_bg`), queued by `World::
    // queue_showvalues_command` above.
    let showvalues_events_applied =
        apply_showvalues_events(&mut world, &character_repository).await;
    if showvalues_events_applied != 0 {
        info!(
            showvalues_events_applied,
            tick = world.tick.0,
            "applied /showvalues events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn values_33(
    mut world: &mut World,
    mut runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
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
    // `/values <name>`'s async DB round trip (C
    // `look_values`/`look_values_bg`), queued by `World::
    // queue_values_command` above.
    let values_events_applied = apply_values_events(
        &mut world,
        &mut runtime,
        &character_repository,
        config.area_id,
        config.mirror_id,
        current_unix_time(),
    )
    .await;
    if values_events_applied != 0 {
        info!(
            values_events_applied,
            tick = world.tick.0,
            "applied /values events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn allow_34(
    mut world: &mut World,
    _runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    config: &ServerConfig,
    _args: &Args,
    _completed_actions: &[WorldActionCompletion],
    _achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
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
    // `/allow <name>`'s async DB round trip (C `allow_body`/
    // `allow_body_db`), queued by `World::
    // queue_allow_command` above.
    let allow_events_applied =
        apply_allow_events(&mut world, &character_repository, config.area_id).await;
    if allow_events_applied != 0 {
        info!(
            allow_events_applied,
            tick = world.tick.0,
            "applied /allow events"
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn player_update_59(
    mut world: &mut World,
    mut runtime: &mut ServerRuntime,
    _zone_loader: &mut ZoneLoader,
    _config: &ServerConfig,
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
    // C `player_update` (`player.c:3448-3462`): every player
    // slot gets `achievement_add_play_time(cn, 1)` plus
    // `stats_update(cn, 1, 0)` (`src/system/statistics.c:
    // 23-45`) once per real-time minute, staggered across
    // ticks via `nr % (TICKS * 60)`. Rust has no stable
    // per-player slot index to replicate that stagger, so
    // this fires for all logged-in characters on the same
    // once-a-minute tick gate already used for auction
    // cleanup above - same net rate (1 minute credited per
    // minute of uptime), just synchronized instead of spread
    // across the 60 ticks.
    if world.tick.0 % (TICKS_PER_SECOND * 60) == 0 {
        let play_time_characters: Vec<CharacterId> = runtime
            .players
            .values()
            .filter_map(|player| player.character_id)
            .collect();
        for character_id in play_time_characters {
            award_play_time_minute(
                &mut world,
                &mut runtime,
                &achievement_repository,
                character_id,
            )
            .await;
            // `stats_update`'s `.online` half (the only field
            // this codebase reads anywhere, via `PlayerRuntime::
            // stats_online_time`, `/values`' "Playing for %d
            // hours." line - not yet wired, see
            // `PORTING_TODO.md`'s "Cross-area transfer" task).
            // The sibling `stats_update(cn, 0, price)` call
            // sites (`store.c:381`/`do.c:1282`) feed `.gold`,
            // which nothing in this codebase reads yet, so
            // they are left unwired (see `PlayerRuntime::
            // stats_update`'s doc comment).
            let character_exp = world
                .characters
                .get(&character_id)
                .map(|character| character.exp as i32)
                .unwrap_or(0);
            if let Some(player) = runtime.player_for_character_mut(character_id) {
                player.stats_update(character_exp, 1, 0, current_unix_time());
            }
        }
    }
}
