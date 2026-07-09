//! Per-tick queued-client-action processing phase: the part of the legacy
//! tick loop that drains `ServerRuntime::drain_actions_for_tick` and
//! dispatches each queued `ClientAction` (text commands, container/swap/
//! gold/junk/speed/fightmode/raise/look/questlog/reopen-quest/ping/nop/
//! client-info/log/mod-packet variants), then flushes the accumulated
//! feedback/inventory/container/name-refresh packets. Extracted verbatim
//! from `main()`'s `tick.tick()` arm (P0.5 "Finish main() phase
//! decomposition") with a superset-params signature, preserving exact
//! execution order; `main.rs` must not grow when this phase changes.
//! Runs between `tick_world::world_step` (game clock/regen/weather/effects)
//! and the completed-action-outcome handling that still lives inline in
//! `main.rs`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn process_queued_client_actions(
    mut world: &mut World,
    mut runtime: &mut ServerRuntime,
    mut zone_loader: &mut ZoneLoader,
    config: &ServerConfig,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
    clan_log_repository: &Option<ugaris_db::PgClanLogRepository>,
    merchant_repository: &Option<ugaris_db::PgMerchantRepository>,
    auction_repository: &Option<ugaris_db::PgAuctionRepository>,
    pentagram_record_repository: &Option<ugaris_db::PgPentagramRecordRepository>,
) {
    let queued = runtime.drain_actions_for_tick();
    if !queued.is_empty() {
        info!(
            count = queued.len(),
            tick = world.tick.0,
            "drained queued client actions"
        );
    }
    let mut command_feedback = Vec::new();
    let mut command_feedback_bytes = Vec::new();
    let mut command_inventory_refresh = Vec::new();
    let mut command_container_refresh = Vec::new();
    let mut command_name_refresh = Vec::new();
    for (character_id, message) in drain_expired_tell_feedback(&world, &mut runtime, world.tick.0) {
        command_feedback_bytes.push((character_id, message));
    }
    let realtime_seconds = world.tick.0 / TICKS_PER_SECOND;
    for (character_id, message) in
        drain_expired_shutup_feedback(&mut world, &mut runtime, realtime_seconds)
    {
        command_feedback_bytes.push((character_id, message));
    }
    for (session_id, action) in queued {
        let Some(player) = runtime.players.get(&session_id) else {
            continue;
        };
        let Some(character_id) = player.character_id else {
            continue;
        };
        match action {
            ClientAction::Text(bytes) => {
                let Some(mut command) = normalize_text_command(&bytes) else {
                    continue;
                };
                {
                    let Some(player) = runtime.players.get_mut(&session_id) else {
                        continue;
                    };
                    if let Some(result) = apply_alias_command(player, &command) {
                        for message in result.messages {
                            command_feedback.push((character_id, message));
                        }
                        continue;
                    }
                    command = player.expand_aliases(&command);
                }
                if command.eq_ignore_ascii_case("sort") {
                    inventory_sort(&mut world, character_id);
                    command_inventory_refresh.push(character_id);
                    continue;
                }
                if command.eq_ignore_ascii_case("accountdepotsort") {
                    if account_depot_sort_if_open(&mut world, &mut runtime, character_id) {
                        command_container_refresh.push(character_id);
                        command_feedback.push((character_id, "Account depot sorted.".to_string()));
                    } else {
                        command_feedback.push((
                            character_id,
                            "You must have the account depot open to use this command.".to_string(),
                        ));
                    }
                    continue;
                }
                if command.eq_ignore_ascii_case("depotsort") {
                    // C `cmdcmp(ptr, "depotsort", 6)` ->
                    // `depot_sort(cn)` (`command.c:9350-9357`):
                    // unlike `/accountdepotsort`, this never
                    // checks whether the depot is currently
                    // open and never sends a confirmation
                    // message - it unconditionally sorts the
                    // character's own `DRD_DEPOT_PPD` block.
                    personal_depot_sort_command(&mut runtime, character_id);
                    command_container_refresh.push(character_id);
                    continue;
                }
                let character_flags = world
                    .characters
                    .get(&character_id)
                    .map(|character| character.flags)
                    .unwrap_or_else(CharacterFlags::empty);
                let weather_before_admin_command = runtime.weather.clone();
                if let Some(result) = apply_weather_admin_command(
                    &world,
                    character_id,
                    &mut runtime.weather,
                    &command,
                ) {
                    // C `cmd_setweather`/`cmd_clearweather`/
                    // `cmd_setareaweather` (`command.c`) each
                    // call `broadcast_weather_packet()`
                    // immediately on success (not just on the
                    // next `update_weather()` tick).
                    if runtime.weather != weather_before_admin_command {
                        broadcast_weather_packet(&world, &mut runtime, config.area_id);
                    }
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    continue;
                }
                if let Some(result) = apply_weather_command(
                    &world,
                    character_id,
                    config.area_id,
                    &runtime.weather,
                    &command,
                ) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    continue;
                }
                if let Some(result) = apply_time_command(world.date, &command) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    continue;
                }
                if let Some(result) =
                    apply_help_command(&command, character_flags, u32::from(config.area_id))
                {
                    if result.message_bytes.is_empty() {
                        for message in result.messages {
                            command_feedback.push((character_id, message));
                        }
                    } else {
                        for message in result.message_bytes {
                            command_feedback_bytes.push((character_id, message));
                        }
                    }
                    continue;
                }
                if let Some(result) = apply_color_command(&mut world, character_id, &command) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    if result.name_changed {
                        command_name_refresh.push(character_id);
                    }
                    continue;
                }
                if let Some(result) = apply_description_command(&mut world, character_id, &command)
                {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    continue;
                }
                if let Some(result) =
                    apply_create_orb_command(&mut world, &mut zone_loader, character_id, &command)
                {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    if result.inventory_changed {
                        command_inventory_refresh.push(character_id);
                    }
                    continue;
                }
                if let Some(result) =
                    apply_create_command(&mut world, &mut zone_loader, character_id, &command)
                {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    if result.inventory_changed {
                        command_inventory_refresh.push(character_id);
                    }
                    continue;
                }
                if let Some(result) = apply_setseyan_command(
                    &mut world,
                    &zone_loader,
                    &mut runtime,
                    character_id,
                    &command,
                ) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    for (target_id, message) in result.other_messages {
                        command_feedback.push((target_id, message));
                    }
                    if result.inventory_changed {
                        command_inventory_refresh.push(character_id);
                    }
                    if result.name_changed {
                        command_name_refresh.push(character_id);
                    }
                    continue;
                }
                if let Some(result) = apply_admin_character_command(
                    &mut world,
                    &mut runtime,
                    character_id,
                    &command,
                    u32::from(config.area_id),
                ) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    for message in result.message_bytes {
                        command_feedback_bytes.push((character_id, message));
                    }
                    for (target_id, message) in result.other_messages {
                        command_feedback.push((target_id, message));
                    }
                    if result.inventory_changed {
                        command_inventory_refresh.push(character_id);
                    }
                    if result.name_changed {
                        command_name_refresh.push(character_id);
                    }
                    if let Some(mirror) = result.mirror_changed {
                        // C `/goto`/`/jump` (`command.c`):
                        // `ch[cn].mirror = m;` takes effect
                        // immediately, matching the same-area
                        // transport-travel mirror-change path
                        // above (not deferred to next tick).
                        if let Some(player) = runtime.player_for_character_mut(character_id) {
                            player.set_current_mirror(mirror);
                        }
                        let mut builder = PacketBuilder::new();
                        builder.mirror(mirror);
                        let payload = builder.into_payload();
                        for (session_id, _) in runtime.sessions_for_character(character_id) {
                            runtime.send_to_session(session_id, payload.clone());
                        }
                    }
                    if let Some(target_id) = result.kick_target {
                        // C `/kick` (`command.c:8668-8698`):
                        // `exit_char` saves the target at its
                        // rest position then despawns it, then
                        // `player_client_exit` sends `SV_EXIT`
                        // with the kick reason and drops the
                        // connection - identical teardown to
                        // `/logout` above, but targeting the
                        // kicked character instead of the
                        // command caller.
                        if let Some(character) = world.characters.get(&target_id) {
                            if let Some(repository) = &character_repository {
                                if let Some(player) = runtime.player_for_character(target_id) {
                                    let account_depot =
                                        runtime.account_depots.get(&target_id).cloned();
                                    let mut save_character = character.clone();
                                    save_character.x = character.rest_x;
                                    save_character.y = character.rest_y;
                                    let request = character_save_request(
                                        &world,
                                        player,
                                        &save_character,
                                        account_depot.as_ref(),
                                        config.area_id,
                                        config.mirror_id,
                                    );
                                    match repository.save_character_snapshot(request).await {
                                        Ok(true) => {
                                            info!(
                                                character_id = target_id.0,
                                                "saved DB-backed character snapshot on /kick"
                                            );
                                        }
                                        Ok(false) => {
                                            warn!(character_id = target_id.0, "DB character snapshot save was skipped by area guard on /kick");
                                        }
                                        Err(err) => {
                                            warn!(character_id = target_id.0, error = %err, "failed to save DB-backed character snapshot on /kick");
                                        }
                                    }
                                }
                            }
                        }
                        runtime.account_depots.remove(&target_id);
                        world.remove_character(target_id);
                        debug!(target: "client_log", character_id = target_id.0, "Used /kick");
                        let mut builder = PacketBuilder::new();
                        builder.exit("You have been kicked by game administration.");
                        let payload = builder.into_payload();
                        for (sid, _) in runtime.sessions_for_character(target_id) {
                            runtime.send_to_session(sid, payload.clone());
                            runtime.flush_session(sid);
                            if let Some(commands) = runtime.sessions.get(&sid) {
                                let _ = commands.try_send(SessionCommand::Disconnect);
                            }
                        }
                    }
                    if let Some((clan_nr, serial, prio, content)) = result.clan_log_entry {
                        // C `/setclanjewels` (`command.c:
                        // 7563-7596`): `add_clanlog(clan_nr,
                        // clan[clan_nr].status.serial, ch[cn].
                        // ID, 1, ...)` when the optional
                        // `do_log` arg is nonzero.
                        clan_log::write_clan_log_entry(
                            &clan_log_repository,
                            clan_nr,
                            serial,
                            character_id,
                            prio,
                            content,
                            current_unix_time(),
                        )
                        .await;
                    }
                    if result.save_all_requested {
                        // C `/saveall` (`command.c:7460-7473`):
                        // `backup_players()` saves exactly one
                        // online player per call (round-robin
                        // cursor, see
                        // `next_backup_rotation_target`'s doc
                        // comment), then `save_all_merchants()`
                        // resaves every live merchant store.
                        if let Some(target_id) = runtime.next_backup_rotation_target() {
                            if let Some(character) = world.characters.get(&target_id) {
                                if let Some(repository) = &character_repository {
                                    if let Some(player) = runtime.player_for_character(target_id) {
                                        let account_depot =
                                            runtime.account_depots.get(&target_id).cloned();
                                        let request = character_backup_save_request(
                                            &world,
                                            player,
                                            character,
                                            account_depot.as_ref(),
                                            config.area_id,
                                            config.mirror_id,
                                        );
                                        match repository.save_character_snapshot(request).await {
                                            Ok(true) => {
                                                info!(character_id = target_id.0, "saved DB-backed character snapshot on /saveall");
                                            }
                                            Ok(false) => {
                                                warn!(character_id = target_id.0, "DB character snapshot save was skipped by area guard on /saveall");
                                            }
                                            Err(err) => {
                                                warn!(character_id = target_id.0, error = %err, "failed to save DB-backed character snapshot on /saveall");
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        if let Some(repository) = &merchant_repository {
                            let merchant_ids: Vec<CharacterId> =
                                world.merchant_stores.keys().copied().collect();
                            for merchant_id in merchant_ids {
                                if let Some(snapshot) = merchant_store_snapshot(&world, merchant_id)
                                {
                                    let name = snapshot.merchant_name.clone();
                                    match repository.save_store(&snapshot).await {
                                        Ok(()) => {
                                            info!(merchant = %name, "saved merchant store on /saveall");
                                        }
                                        Err(err) => {
                                            warn!(merchant = %name, error = %err, "failed to save merchant store on /saveall");
                                        }
                                    }
                                }
                            }
                        }
                        // C `/saveall` (`command.c:7470`):
                        // `save_pentagram_record_scheduled()`.
                        crate::pents::save_pentagram_record_scheduled(
                            &world,
                            &pentagram_record_repository,
                        )
                        .await;
                    }
                    if let Some(merchant_id) = result.clear_merchant_store_requested {
                        // C `/clearmerchantstores` (`command.c:
                        // 7510-7538`): `save_merchant_inventory
                        // (merchant_cn)` persists the cleared
                        // store right after the mutation.
                        save_merchant_store_if_configured(
                            &world,
                            &merchant_repository,
                            merchant_id,
                        )
                        .await;
                    }
                    if let Some((target_area, target_x, target_y)) = result.cross_area_transfer {
                        // C `/office` (`command.c:9670-9676`)
                        // and `/goto`/`/jump` (`command.c:
                        // 8537-8567`/`8608-8625`): `change_area
                        // (cn, a, x, y)` reads `ch[cn].mirror`
                        // as the target mirror, which is set
                        // to `m` just beforehand when `m` is a
                        // valid mirror (`mirror_changed`
                        // above); otherwise the target mirror
                        // is the caller's own current area's
                        // mirror.
                        let target_mirror =
                            result.mirror_changed.unwrap_or(u32::from(config.mirror_id));
                        let transferred = attempt_cross_area_transfer(
                            &mut world,
                            &mut runtime,
                            &character_repository,
                            &area_repository,
                            config.area_id,
                            config.mirror_id,
                            character_id,
                            target_area,
                            target_mirror,
                            target_x,
                            target_y,
                        )
                        .await;
                        if !transferred {
                            command_feedback.push((
                                character_id,
                                "Nothing happens - target area server is down.".to_string(),
                            ));
                        }
                    }
                    continue;
                }
                if let Some(result) = apply_logout_command(&world, character_id, &command) {
                    if result.logout_requested {
                        // C `cmd_logout` (`player.c:4457-4471`):
                        // `exit_char` saves the character at its
                        // rest position (`tmpx/tmpy = restx/
                        // resty` before `kick_char`'s save),
                        // then despawns it (no lostcon linger,
                        // unlike a network disconnect), then
                        // `player_client_exit` sends `SV_EXIT`
                        // and drops the connection.
                        if let Some(character) = world.characters.get(&character_id) {
                            if let Some(repository) = &character_repository {
                                if let Some(player) = runtime.players.get(&session_id) {
                                    let account_depot =
                                        runtime.account_depots.get(&character_id).cloned();
                                    let mut save_character = character.clone();
                                    save_character.x = character.rest_x;
                                    save_character.y = character.rest_y;
                                    let request = character_save_request(
                                        &world,
                                        player,
                                        &save_character,
                                        account_depot.as_ref(),
                                        config.area_id,
                                        config.mirror_id,
                                    );
                                    match repository.save_character_snapshot(request).await {
                                        Ok(true) => {
                                            info!(
                                                character_id = character_id.0,
                                                "saved DB-backed character snapshot on /logout"
                                            );
                                        }
                                        Ok(false) => {
                                            warn!(character_id = character_id.0, "DB character snapshot save was skipped by area guard on /logout");
                                        }
                                        Err(err) => {
                                            warn!(character_id = character_id.0, error = %err, "failed to save DB-backed character snapshot on /logout");
                                        }
                                    }
                                }
                            }
                        }
                        runtime.account_depots.remove(&character_id);
                        world.remove_character(character_id);
                        debug!(target: "client_log", character_id = character_id.0, "Used /logout");
                        let mut builder = PacketBuilder::new();
                        builder.exit("Logout upon player request.");
                        let payload = builder.into_payload();
                        for (sid, _) in runtime.sessions_for_character(character_id) {
                            runtime.send_to_session(sid, payload.clone());
                            runtime.flush_session(sid);
                            if let Some(commands) = runtime.sessions.get(&sid) {
                                let _ = commands.try_send(SessionCommand::Disconnect);
                            }
                        }
                    } else {
                        for message in result.messages {
                            command_feedback.push((character_id, message));
                        }
                    }
                    continue;
                }
                if let Some(result) = apply_achievement_command(
                    &mut world,
                    &mut runtime,
                    &achievement_repository,
                    character_id,
                    &command,
                    current_unix_time(),
                )
                .await
                {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    for message in result.message_bytes {
                        command_feedback_bytes.push((character_id, message));
                    }
                    for (target_id, message) in result.target_message_bytes {
                        command_feedback_bytes.push((target_id, message));
                    }
                    continue;
                }
                if let Some(result) =
                    apply_demonlords_command(&world, &runtime, character_id, &command)
                {
                    for message in result.message_bytes {
                        command_feedback_bytes.push((character_id, message));
                    }
                    continue;
                }
                if let Some(result) = apply_orbs_command(&world, &runtime, character_id, &command) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    for message in result.message_bytes {
                        command_feedback_bytes.push((character_id, message));
                    }
                    continue;
                }
                if let Some(result) =
                    apply_treasures_command(&world, &runtime, character_id, &command)
                {
                    for message in result.message_bytes {
                        command_feedback_bytes.push((character_id, message));
                    }
                    continue;
                }
                if let Some(result) = apply_tunnel_command(&world, &runtime, character_id, &command)
                {
                    for message in result.message_bytes {
                        command_feedback_bytes.push((character_id, message));
                    }
                    continue;
                }
                if let Some(result) =
                    apply_tunnellist_command(&world, &runtime, character_id, &command)
                {
                    for message in result.message_bytes {
                        command_feedback_bytes.push((character_id, message));
                    }
                    continue;
                }
                if let Some(result) = apply_shutup_command(
                    &mut world,
                    &mut runtime,
                    character_id,
                    &command,
                    realtime_seconds,
                ) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    for (target_id, message) in result.target_message_bytes {
                        command_feedback_bytes.push((target_id, message));
                    }
                    continue;
                }
                if let Some(result) = apply_notells_command(&mut world, character_id, &command) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    continue;
                }
                if let Some(result) = apply_thief_command(&mut world, character_id, &command) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    continue;
                }
                if let Some(result) = apply_channels_command(&command) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    continue;
                }
                let character_flags = world
                    .characters
                    .get(&character_id)
                    .map(|character| character.flags)
                    .unwrap_or_else(CharacterFlags::empty);
                if let Some(player) = runtime.player_for_character_mut(character_id) {
                    if let Some(result) =
                        apply_join_leave_chat_command(player, character_flags, &command)
                    {
                        for message in result.messages {
                            command_feedback.push((character_id, message));
                        }
                        continue;
                    }
                }
                if let Some(result) =
                    apply_clearignore_command(&mut runtime, character_id, &command)
                {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    continue;
                }
                if let Some(result) =
                    apply_ignore_command(&world, &mut runtime, character_id, &command)
                {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    continue;
                }
                if let Some(result) = apply_tell_command(
                    &world,
                    &mut runtime,
                    character_id,
                    &command,
                    world.tick.0,
                    u64::from(current_realtime_seconds()),
                ) {
                    for message in result.sender_messages {
                        command_feedback.push((character_id, message));
                    }
                    for (target_id, message) in result.delivered_messages {
                        command_feedback.push((target_id, message));
                    }
                    for (target_id, message) in result.delivered_message_bytes {
                        command_feedback_bytes.push((target_id, message));
                    }
                    continue;
                }
                let current_tick = world.tick.0;
                if let Some(result) = apply_local_speech_command(
                    &mut world,
                    &mut runtime,
                    character_id,
                    &command,
                    current_tick,
                    u64::from(current_realtime_seconds()),
                ) {
                    for message in result.sender_messages {
                        command_feedback.push((character_id, message));
                    }
                    for (target_id, message) in result.delivered_message_bytes {
                        command_feedback_bytes.push((target_id, message));
                    }
                    continue;
                }
                if let Some(result) = apply_chat_command(
                    &world,
                    &mut runtime,
                    character_id,
                    &command,
                    config.area_id,
                    u64::from(current_realtime_seconds()),
                ) {
                    for message in result.sender_messages {
                        command_feedback.push((character_id, message));
                    }
                    for (target_id, message) in result.delivered_message_bytes {
                        command_feedback_bytes.push((target_id, message));
                    }
                    continue;
                }
                if let Some(result) = apply_nowho_command(&mut world, character_id, &command) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    continue;
                }
                if let Some(result) =
                    apply_who_command(&world, Some(&runtime), character_flags, &command)
                {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    continue;
                }
                let Some(player) = runtime.players.get_mut(&session_id) else {
                    continue;
                };
                let realtime_seconds = world.tick.0 / TICKS_PER_SECOND;
                if let Some(result) = apply_pk_hate_command(
                    &mut world,
                    player,
                    character_id,
                    &command,
                    realtime_seconds,
                ) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    command_name_refresh.extend(result.name_refresh);
                    continue;
                }
                if let Some(result) = apply_steal_command(
                    &mut world,
                    player,
                    character_id,
                    &command,
                    realtime_seconds,
                ) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    for (target_id, message) in result.target_message_bytes {
                        command_feedback_bytes.push((target_id, message));
                    }
                    if result.inventory_changed {
                        command_inventory_refresh.push(character_id);
                    }
                    continue;
                }
                if let Some(result) = apply_maxlag_command(player, &command) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    continue;
                }
                if let Some(result) = apply_hints_command(player, &command) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    continue;
                }
                if let Some(result) = apply_wimp_command(&command) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    continue;
                }
                if let Some(character) = world.characters.get(&character_id) {
                    if let Some(result) = apply_autoturn_command(character, player, &command) {
                        for message in result.messages {
                            command_feedback.push((character_id, message));
                        }
                        continue;
                    }
                }
                if let Some(result) = apply_swap_command(&mut world, player, character_id, &command)
                {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    continue;
                }
                if let Some(character) = world.characters.get(&character_id) {
                    if let Some(result) =
                        apply_lag_control_toggle_command(character, player, &command)
                    {
                        for message in result.messages {
                            command_feedback.push((character_id, message));
                        }
                        continue;
                    }
                }
                if let Some(result) = apply_lag_command(&mut world, player, character_id, &command)
                {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    continue;
                }
                if let Some(result) =
                    apply_allowbless_command(&mut world, player, character_id, &command)
                {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    continue;
                }
                if let Some(result) = apply_killbless_command(&mut world, character_id, &command) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    if result.inventory_changed {
                        command_inventory_refresh.push(character_id);
                    }
                    continue;
                }
                if let Some(result) = apply_lastseen_command(&mut world, character_id, &command) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    continue;
                }
                {
                    let is_god = world
                        .characters
                        .get(&character_id)
                        .is_some_and(|character| character.flags.contains(CharacterFlags::GOD));
                    if let Some(result) = apply_complain_command(
                        &mut world,
                        player,
                        character_id,
                        &command,
                        is_god,
                        realtime_seconds,
                    ) {
                        for message in result.messages {
                            command_feedback.push((character_id, message));
                        }
                        for message in result.message_bytes {
                            command_feedback_bytes.push((character_id, message));
                        }
                        continue;
                    }
                }
                if let Some(character) = world.characters.get(&character_id) {
                    if let Some(result) = apply_status_command(character, player, &command) {
                        for message in result.messages {
                            command_feedback.push((character_id, message));
                        }
                        continue;
                    }
                }
                if let Some(result) =
                    apply_gold_command(&mut world, &mut zone_loader, character_id, &command)
                {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    if result.inventory_changed {
                        command_inventory_refresh.push(character_id);
                    }
                    continue;
                }
                if let Some(result) = apply_laugh_command(&mut world, character_id, &command) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    continue;
                }
                if let Some(result) = apply_keyring_command(
                    &mut world,
                    &mut zone_loader,
                    player,
                    character_id,
                    &command,
                ) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    if result.inventory_changed {
                        command_inventory_refresh.push(character_id);
                    }
                    continue;
                }
                if let Some(result) = auction::apply_auction_command(
                    &mut world,
                    &auction_repository,
                    character_id,
                    current_unix_time(),
                    &command,
                )
                .await
                {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    for message in result.message_bytes {
                        command_feedback_bytes.push((character_id, message));
                    }
                    for (target_id, message) in result.other_messages {
                        command_feedback.push((target_id, message));
                    }
                    if result.inventory_changed {
                        command_inventory_refresh.push(character_id);
                    }
                }
                if let Some(result) = clan_log::apply_clan_log_command(
                    &mut world,
                    &clan_log_repository,
                    character_id,
                    current_unix_time(),
                    &command,
                )
                .await
                {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    for message in result.message_bytes {
                        command_feedback_bytes.push((character_id, message));
                    }
                }
                if let Some(result) = clan_command::apply_clan_command(
                    &mut world,
                    character_id,
                    &command,
                    current_unix_time(),
                ) {
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    for message in result.message_bytes {
                        command_feedback_bytes.push((character_id, message));
                    }
                }
                // C `special_driver`'s `CDR_LQPARSER` admin command table
                // (`system/command.c:5855-5859` area-gates into
                // `lq.c:2505-2742`). All feedback is queued directly onto
                // `World`'s pending system-text queues and flushed by
                // `tick_sync::sync_phase` later this same tick.
                // `#nspawn`/`#thrall` are the two commands in this table
                // needing `ZoneLoader` (a brand new character), so they
                // are checked first via their own dispatchers - see
                // `ugaris_core::world::LqNspawnDispatch`/`LqThrallDispatch`'s
                // doc comments.
                match world.try_dispatch_lq_nspawn(character_id, config.area_id, &command) {
                    ugaris_core::world::LqNspawnDispatch::NotMatched => {
                        match world.try_dispatch_lq_thrall(character_id, config.area_id, &command) {
                            ugaris_core::world::LqThrallDispatch::NotMatched => {
                                world.apply_lq_admin_command(
                                    character_id,
                                    config.area_id,
                                    &command,
                                );
                            }
                            ugaris_core::world::LqThrallDispatch::Rejected => {}
                            ugaris_core::world::LqThrallDispatch::Requests(requests) => {
                                for request in &requests {
                                    crate::spawns::spawn_lq_npc_character(
                                        &mut world,
                                        &mut zone_loader,
                                        &mut runtime,
                                        request,
                                    );
                                }
                            }
                        }
                    }
                    ugaris_core::world::LqNspawnDispatch::Rejected => {}
                    ugaris_core::world::LqNspawnDispatch::Requests(requests) => {
                        let mut spawned = 0usize;
                        for request in &requests {
                            if crate::spawns::spawn_lq_npc_character(
                                &mut world,
                                &mut zone_loader,
                                &mut runtime,
                                request,
                            ) {
                                spawned += 1;
                            }
                        }
                        world.report_lq_nspawn_result(character_id, spawned);
                    }
                }
            }
            ClientAction::Container { .. } | ClientAction::LookContainer { .. } => {
                // C cl_container: validate and prefer the active
                // merchant store before item containers.
                world.check_merchant(character_id);
                let active_merchant = world
                    .characters
                    .get(&character_id)
                    .and_then(|character| character.merchant);
                if let Some(merchant_id) = active_merchant {
                    let result = apply_merchant_container_command(
                        &mut world,
                        character_id,
                        merchant_id,
                        &action,
                    );
                    for message in result.messages {
                        command_feedback.push((character_id, message));
                    }
                    if result.changed {
                        command_inventory_refresh.push(character_id);
                        command_container_refresh.push(character_id);
                        save_merchant_store_if_configured(
                            &world,
                            &merchant_repository,
                            merchant_id,
                        )
                        .await;
                    }
                    continue;
                }
                let current_container = world
                    .characters
                    .get(&character_id)
                    .and_then(|character| character.current_container);
                let is_account_depot = current_container.is_some_and(|container_id| {
                    world
                        .items
                        .get(&container_id)
                        .is_some_and(|item| item.driver == IDR_ACCOUNT_DEPOT)
                });
                let is_personal_depot = !is_account_depot
                    && current_container.is_some_and(|container_id| {
                        world
                            .items
                            .get(&container_id)
                            .is_some_and(|item| item.flags.contains(ItemFlags::DEPOT))
                    });
                let result = if is_account_depot {
                    let depot = runtime.ensure_account_depot(character_id);
                    apply_account_depot_command(&mut world, depot, character_id, &action)
                } else if is_personal_depot {
                    match runtime.player_for_character_mut(character_id) {
                        Some(player) => apply_personal_depot_command(
                            &mut world,
                            &mut player.depot,
                            character_id,
                            &action,
                        ),
                        None => AccountDepotCommandResult::Ignored,
                    }
                } else {
                    apply_item_container_command(&mut world, character_id, &action)
                };
                match result {
                    AccountDepotCommandResult::Changed => {
                        command_inventory_refresh.push(character_id);
                        command_container_refresh.push(character_id);
                    }
                    AccountDepotCommandResult::Look(message)
                    | AccountDepotCommandResult::Blocked(message) => {
                        command_feedback.push((character_id, message));
                    }
                    AccountDepotCommandResult::Ignored => {}
                }
            }
            ClientAction::FastSell { slot } => {
                // C `cl_fastsell`: quick-sell an inventory slot
                // straight to the active merchant.
                let result = apply_fast_sell(&mut world, character_id, usize::from(slot));
                for message in result.messages {
                    command_feedback.push((character_id, message));
                }
                if result.inventory_changed {
                    command_inventory_refresh.push(character_id);
                }
                if result.sold {
                    command_container_refresh.push(character_id);
                    let merchant_id = world
                        .characters
                        .get(&character_id)
                        .and_then(|character| character.merchant);
                    if let Some(merchant_id) = merchant_id {
                        save_merchant_store_if_configured(
                            &world,
                            &merchant_repository,
                            merchant_id,
                        )
                        .await;
                    }
                }
            }
            ClientAction::Swap { .. }
            | ClientAction::UseInventory { .. }
            | ClientAction::LookInventory { .. }
            | ClientAction::LookItem { .. } => {
                let result = apply_inventory_client_action(
                    &mut world,
                    runtime.player_for_character(character_id),
                    character_id,
                    &action,
                    config.area_id,
                );
                match result {
                    InventoryCommandResult::Changed => {
                        command_inventory_refresh.push(character_id);
                    }
                    InventoryCommandResult::MoneyConverted { price } => {
                        command_inventory_refresh.push(character_id);
                        award_swap_money_converted_achievement(
                            &mut world,
                            &mut runtime,
                            &achievement_repository,
                            character_id,
                            price,
                        )
                        .await;
                    }
                    InventoryCommandResult::ContainerOpened { account_depot } => {
                        if account_depot {
                            runtime.ensure_account_depot(character_id);
                        }
                        command_inventory_refresh.push(character_id);
                        command_container_refresh.push(character_id);
                    }
                    InventoryCommandResult::Look(message) => {
                        command_feedback.push((character_id, message));
                    }
                    InventoryCommandResult::Ignored => {}
                }
            }
            ClientAction::TakeGold { .. } | ClientAction::DropGold => {
                if apply_gold_client_action(&mut world, &mut zone_loader, character_id, &action) {
                    command_inventory_refresh.push(character_id);
                }
            }
            ClientAction::JunkItem => {
                if apply_junk_item_client_action(&mut world, character_id) {
                    command_inventory_refresh.push(character_id);
                }
            }
            ClientAction::Speed { mode } => {
                // C `cl_speed` (`src/system/player.c`): silently
                // ignores invalid mode bytes and fast-mode
                // requests without enough endurance - no
                // feedback packet either way.
                world.set_speed_mode(character_id, mode);
            }
            ClientAction::FightMode { .. } => {
                // C `cl_fightmode` (`src/system/player.c`) is a
                // no-op stub (`return;`); `ch[cn].fight_mode` is
                // otherwise unused in the C tree. Consume the
                // packet without acting on it, matching C.
            }
            ClientAction::Raise { value } => {
                // C `cl_raise` (`src/system/player.c`) calls
                // `raise_value` and discards the result - no
                // feedback packet on failure, only the updated
                // value/exp on success.
                if let RaiseSkillOutcome::Raised {
                    value,
                    bare,
                    effective,
                    exp,
                    exp_used,
                } = world.raise_skill(character_id, value)
                {
                    let mut builder = PacketBuilder::new();
                    builder
                        .set_value0(value as u8, effective)
                        .set_value1(value as u8, bare)
                        .exp(exp)
                        .exp_used(exp_used);
                    runtime.send_to_session(session_id, builder.into_payload());
                    // C `raise_value` (`src/system/skill.c:256-
                    // 259`): `if (ch[cn].flags & CF_PLAYER) {
                    // achievement_check_skill(cn, v,
                    // ch[cn].value[1][v]); }`.
                    award_skill_achievement(
                        &mut world,
                        &mut runtime,
                        &achievement_repository,
                        character_id,
                        value as i32,
                        bare as i32,
                    )
                    .await;
                }
            }
            ClientAction::LookCharacter { character } => {
                // C `cl_look_char` (`src/system/player.c`):
                // bounds-checks the target, gates on
                // `char_see_char`, then `look_char`
                // (`src/system/tool.c`) sends `#1`/`#2` text
                // plus the `SV_LOOKINV` paperdoll. `character
                // == 0` mirrors C's `co < 1` bounds check.
                if character != 0 {
                    let target_id = CharacterId(u32::from(character));
                    let target_is_brave = runtime
                        .player_for_character(target_id)
                        .is_some_and(|player| player.has_used_random_shrine(51));
                    let target_mirror = runtime
                        .player_for_character(target_id)
                        .map(|player| u32::from(player.current_mirror_id))
                        .unwrap_or(0);
                    let (target_pk_kills, target_pk_deaths) = runtime
                        .player_for_character(target_id)
                        .map(|player| (player.pk_kills, player.pk_deaths))
                        .unwrap_or((0, 0));
                    if let Some(text) = world.look_character_text(
                        character_id,
                        target_id,
                        target_is_brave,
                        target_mirror,
                        target_pk_kills,
                        target_pk_deaths,
                    ) {
                        command_feedback.push((character_id, text.header));
                        if let Some(paperdoll) = world.look_character_paperdoll(target_id) {
                            let mut builder = PacketBuilder::new();
                            builder.look_inventory(
                                paperdoll.sprite,
                                paperdoll.colors,
                                paperdoll.worn_sprites,
                            );
                            runtime.send_to_session(session_id, builder.into_payload());
                        }
                        command_feedback.push((character_id, text.body));
                    }
                }
            }
            ClientAction::GetQuestLog => {
                if let Some(player) = runtime.players.get(&session_id) {
                    let payload = legacy_questlog_payload(player);
                    runtime.send_to_session(session_id, payload);
                }
            }
            ClientAction::ReopenQuest { quest } => {
                // C `questlog_reopen` (`src/system/questlog.c:613-826`):
                // `sendquestlog` fires unconditionally once the
                // generic preconditions pass (`Reopened`,
                // `SeriesConflict`, and `NoEffect` all reach the
                // per-quest switch), even when the switch leaves
                // `ret` falsy and nothing actually reopens.
                let result_and_payload = runtime.players.get_mut(&session_id).map(|player| {
                    let result = player.reopen_quest_legacy(quest as usize);
                    let payload = (!matches!(
                        result,
                        QuestReopenResult::CannotOpenAgain
                            | QuestReopenResult::CannotOpenNow
                            | QuestReopenResult::InvalidQuest
                    ))
                    .then(|| legacy_questlog_payload(player));
                    (result, payload)
                });
                if let Some((result, payload)) = result_and_payload {
                    if let Some(payload) = payload {
                        runtime.send_to_session(session_id, payload);
                    }
                    // C `questlog_reopen` (`src/system/
                    // questlog.c:815-822`): when `ret` stayed
                    // truthy (our `Reopened` case) and the
                    // character is a player (always true here),
                    // `achievement_award(cn,
                    // ACHIEVEMENT_QUESTER, 1)` fires.
                    if matches!(result, QuestReopenResult::Reopened) {
                        let name = world
                            .characters
                            .get(&character_id)
                            .map(|character| character.name.clone());
                        if let (Some(name), Some(player)) =
                            (name, runtime.player_for_character_mut(character_id))
                        {
                            let now = current_unix_time();
                            if player
                                .achievement_data
                                .award(AchievementType::Quester, &name, now)
                            {
                                let payload =
                                    achievement_unlock_payload(AchievementType::Quester, now);
                                for (sid, _) in runtime.sessions_for_character(character_id) {
                                    runtime.send_to_session(sid, payload.clone());
                                }
                                record_achievement_firsts_and_announce(
                                    &mut world,
                                    &achievement_repository,
                                    character_id,
                                    &name,
                                    &[AchievementType::Quester],
                                )
                                .await;
                            }
                        }
                    }
                    match result {
                        QuestReopenResult::Reopened | QuestReopenResult::NoEffect => {}
                        QuestReopenResult::SeriesConflict => command_feedback.push((
                            character_id,
                            "Cannot re-open more than one quest from a series.".to_string(),
                        )),
                        QuestReopenResult::CannotOpenAgain => command_feedback.push((
                            character_id,
                            "You cannot open this quest again.".to_string(),
                        )),
                        QuestReopenResult::CannotOpenNow => command_feedback.push((
                            character_id,
                            "You cannot open this quest at the moment.".to_string(),
                        )),
                        QuestReopenResult::InvalidQuest => {}
                    }
                }
            }
            ClientAction::Ping { value } => {
                // C `cl_ping` (`src/system/player.c`) blindly
                // echoes the client's opaque 4-byte value back
                // prefixed with `SV_PING` - no character/world
                // state involved, pure transport round trip.
                let mut builder = PacketBuilder::new();
                builder.ping(value);
                runtime.send_to_session(session_id, builder.into_payload());
            }
            ClientAction::Nop => {
                // C `cl_nop` (`src/system/player.c`) is a
                // genuine no-op used only as a keep-alive
                // filler packet - no logging in C either.
            }
            ClientAction::ClientInfo(_) => {
                // C `cl_clientinfo` (`src/system/player.c`)
                // has its entire body commented out: the
                // `client_info` payload (skip/idle counters,
                // sysmem/vidmem, display surfaces) is parsed
                // off the wire and discarded. Matches C.
            }
            ClientAction::Log(bytes) => {
                // C `cl_log` (`src/system/player.c`) writes
                // the client-supplied message to the server
                // log via `charlog`. Port that as a `debug`
                // trace line instead of silently dropping it.
                let name = world
                    .characters
                    .get(&character_id)
                    .map(|character| character.name.as_str())
                    .unwrap_or("ILLEGAL CN");
                let message = String::from_utf8_lossy(&bytes);
                debug!(
                    target: "client_log",
                    "{}",
                    format_client_log_message(name, character_id.0, &message)
                );
            }
            ClientAction::ModPacket {
                packet_type,
                subtype,
                ..
            } => {
                // C `cl_mod1`/`cl_mod3` (`src/system/player.c`)
                // route known handshake subtypes (0x01-0x0F:
                // mod version/ready/pong) to a blind
                // acknowledge ("For now, just acknowledge we
                // received them"); other subtypes get an
                // `SV_MOD1`/`SV_SYS_ERROR` reply via
                // `mod_send_error_by_slot`, not ported yet.
                // Log and no-op for now, matching the C
                // oracle's own "future work" stub.
                debug!(
                    character_id = character_id.0,
                    packet_type, subtype, "mod packet received (not yet implemented, logged no-op)"
                );
            }
            _ => {}
        }
    }
    if !command_feedback.is_empty()
        || !command_feedback_bytes.is_empty()
        || !command_inventory_refresh.is_empty()
        || !command_container_refresh.is_empty()
        || !command_name_refresh.is_empty()
    {
        let mut feedback_sessions = 0;
        for (character_id, message) in command_feedback {
            let payload = ugaris_protocol::packet::system_text(&message);
            for (session_id, _) in runtime.sessions_for_character(character_id) {
                if runtime.send_to_session(session_id, payload.clone()) {
                    feedback_sessions += 1;
                }
            }
        }
        for (character_id, message) in command_feedback_bytes {
            let payload = ugaris_protocol::packet::system_text_bytes(&message);
            for (session_id, _) in runtime.sessions_for_character(character_id) {
                if runtime.send_to_session(session_id, payload.clone()) {
                    feedback_sessions += 1;
                }
            }
        }
        let mut inventory_sessions = 0;
        command_inventory_refresh.sort_unstable_by_key(|id| id.0);
        command_inventory_refresh.dedup();
        for character_id in command_inventory_refresh {
            let Some(character) = world.characters.get(&character_id) else {
                continue;
            };
            let payload = inventory_snapshot_payload(&world, character);
            for (session_id, _) in runtime.sessions_for_character(character_id) {
                if runtime.send_to_session(session_id, payload.clone()) {
                    inventory_sessions += 1;
                }
            }
        }
        let mut container_sessions = 0;
        command_container_refresh.sort_unstable_by_key(|id| id.0);
        command_container_refresh.dedup();
        for character_id in command_container_refresh {
            // Active merchant stores refresh with prices instead
            // of the ordinary container view.
            world.check_merchant(character_id);
            let payload = if world
                .characters
                .get(&character_id)
                .is_some_and(|character| character.merchant.is_some())
            {
                merchant_store_payload(&mut world, character_id)
            } else {
                if !check_current_container(&mut world, character_id) {
                    continue;
                }
                current_container_payload(
                    &world,
                    runtime.account_depots.get(&character_id),
                    runtime
                        .player_for_character(character_id)
                        .map(|player| player.depot.as_slice()),
                    character_id,
                )
            };
            let Some(payload) = payload else { continue };
            for (session_id, _) in runtime.sessions_for_character(character_id) {
                if runtime.send_to_session(session_id, payload.clone()) {
                    container_sessions += 1;
                }
            }
        }
        let mut name_sessions = 0;
        command_name_refresh.sort_unstable_by_key(|id| id.0);
        command_name_refresh.dedup();
        let pk_relations = PkRelationSnapshot::from_runtime(&runtime);
        for character_id in command_name_refresh {
            let Some(character) = world.characters.get(&character_id).cloned() else {
                continue;
            };
            for (session_id, payload) in
                runtime.refresh_known_character_name(&world, &pk_relations, &character)
            {
                if runtime.send_to_session(session_id, payload.clone()) {
                    name_sessions += 1;
                }
            }
        }
        info!(
            feedback_sessions,
            inventory_sessions,
            container_sessions,
            name_sessions,
            tick = world.tick.0,
            "processed text/container commands"
        );
    }
}
