use super::*;

/// C `command.c`'s `lastseen:` handler's async DB round-trip
/// (`lastseen`/`db_lastseen`, `database_lookup.c:142-157` +
/// `database_notes.c:352-390`): resolves every `World::
/// drain_pending_lastseen_lookups` entry (queued by validly-shaped
/// `/lastseen <name>` arguments - see `World::queue_lastseen_lookup`'s
/// doc comment for the synchronous invalid-name fast path) against the
/// DB and delivers the reply via `World::queue_system_text` (C's
/// `tell_chat(0, rID, 1, ...)`, this codebase's direct-to-character
/// system-text channel).
///
/// Message shape mirrors `db_lastseen` exactly:
/// - no DB row -> "No character by the name %s." - the exact same text
///   the command dispatcher's own `lookup_name` `== -1` branch uses
///   (`command.c:9041`), since a player can't tell the two cases apart.
/// - `CF_GOD` row -> "%s was seen quite recently." (C never computes an
///   elapsed time for staff, `database_notes.c:378-379`).
/// - otherwise -> "%s was last seen %d days, %d hours, %d minutes ago.",
///   from `now - last_activity` where `last_activity` is `LastSeenInfo::
///   last_activity_unix` (already `max(login_time, logout_time,
///   created_at)`, computed in SQL - see `ugaris-db`'s `FIND_LAST_SEEN_SQL`
///   doc comment).
///
/// No-ops entirely (silent, matching every other offline-DB-lookup event
/// in this file) when no `character_repository` is configured or the
/// query itself errors.
pub(crate) async fn apply_lastseen_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    now_unix: i64,
) -> usize {
    let lookups = world.drain_pending_lastseen_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let reply = match repository.find_last_seen(&lookup.target_name).await {
            Ok(Some(info)) => lastseen_reply_message(&info, now_unix),
            Ok(None) => format!("No character by the name {}.", lookup.target_name),
            Err(_) => continue,
        };
        world.queue_system_text(lookup.requester_id, reply);
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_lastseen_events`], split out so
/// the day/hour/minute arithmetic (`database_notes.c:381-386`) can be
/// unit-tested without a live database.
pub(crate) fn lastseen_reply_message(info: &ugaris_db::LastSeenInfo, now_unix: i64) -> String {
    if info.is_god {
        return format!("{} was seen quite recently.", info.name);
    }
    let elapsed = now_unix - info.last_activity_unix;
    format!(
        "{} was last seen {} days, {} hours, {} minutes ago.",
        info.name,
        elapsed / (60 * 60 * 24),
        (elapsed / (60 * 60)) % 24,
        (elapsed / 60) % 60
    )
}

/// `#querystats`/`/querystats`'s async round trip - see `ugaris-core`'s
/// `world/querystats.rs` module doc comment for exactly which C counters
/// this scoped-down port tracks (and why the rest are omitted rather than
/// faked). `PgCharacterRepository::query_stats` is a synchronous
/// in-memory atomic read, not a real query, but is still routed through
/// this tick-loop drain (rather than answered directly in
/// `commands_admin.rs`) since command dispatch has no visibility into
/// `character_repository` - the same architectural constraint every
/// other DB-backed command in this file works around.
///
/// No-ops entirely (silent) when no `character_repository` is configured,
/// matching every sibling offline-DB-lookup event's convention.
pub(crate) fn apply_querystats_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let lookups = world.drain_pending_querystats_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let stats = repository.query_stats();
        for line in querystats_lines(stats) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure formatting half of `apply_querystats_events`, split out for unit
/// testing without needing a live `PgCharacterRepository` - matches
/// `ac_status_lines`'s established pattern for this file. Reproduces C's
/// `"Database Query Statistics:"` header and `"Character operations:"`
/// subheader/line verbatim (`command.c:6596,6601-604`); every other C
/// line (`Total queries`/`Average query time`/`Other operations`/`Query
/// type statistics`) is omitted, not faked, since nothing in `ugaris-db`
/// increments those counters - see `ugaris-core`'s `world/querystats.rs`
/// module doc comment.
pub(crate) fn querystats_lines(stats: ugaris_db::CharacterQueryStats) -> Vec<String> {
    vec![
        "Database Query Statistics:".to_string(),
        "Character operations:".to_string(),
        format!(
            "Save chars: {}, Exit chars: {}, Load chars: {}",
            stats.save_char_cnt, stats.exit_char_cnt, stats.load_char_cnt
        ),
    ]
}

/// `/jail`/`/unjail`'s async DB round trip (C `lookup_name`,
/// `system/lookup.c:42-98` + `system/database/database_lookup.c:57-83`):
/// resolves every `World::drain_pending_jail_lookups` entry (queued by a
/// validly-shaped `/jail`/`/unjail <name>` argument - see `World::
/// queue_jail_lookup`'s and `apply_admin_character_command`'s doc
/// comments) against the DB.
///
/// - no DB row -> "No character by the name %s." (C's dispatcher-level
///   `lookup_name == -1` branch, `command.c:9041`-equivalent for
///   `jail`/`unjail`).
/// - a row found -> hands off to `World::resolve_jail_lookup`, which
///   reproduces `cmd_jail_player`/`cmd_unjail_player`'s own separate
///   online-only `CF_PLAYER` name scan and, on a match, applies the
///   jail/unjail mutation (no match -> "No player by that name.", the
///   exact text both C functions share).
///
/// No-ops entirely (silent) when no `character_repository` is configured
/// or a query errors, matching every sibling offline-DB-lookup event.
pub(crate) async fn apply_jail_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let lookups = world.drain_pending_jail_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        match repository.find_login_target(&lookup.target_name).await {
            Ok(Some(_)) => {
                world.resolve_jail_lookup(lookup.caller_id, &lookup.target_name, lookup.action);
            }
            Ok(None) => {
                world.queue_system_text(
                    lookup.caller_id,
                    format!("No character by the name {}.", lookup.target_name),
                );
            }
            Err(_) => continue,
        }
        applied += 1;
    }
    applied
}

/// `/jail`/`/unjail`'s cross-area hand-off (C `change_area(cn, resta,
/// restx, resty)`, `src/system/tool.c:4392-4425`'s tail): resolves every
/// `World::drain_pending_jail_cross_area_transfers` entry (queued by
/// `World::apply_jail_action` when the jail/aston destination area
/// differs from this area server's own `area_id` - see `world/jail.rs`'s
/// module doc comment) via the shared `attempt_cross_area_transfer`
/// helper, same as the `TransportTravel`/`ClanSpawnExit`/`MineGateway`/
/// `/office`+`/goto` call sites. The destination mirror always equals
/// this process's own `mirror_id`: neither jail nor aston locations carry
/// a mirror field of their own (matching C's `change_area` reading
/// `ch[cn].mirror`, i.e. the target character's *own current* mirror,
/// which under this codebase's single-process-per-area-mirror stance is
/// always this process's `mirror_id`).
pub(crate) async fn apply_jail_cross_area_transfers(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
    area_id: u16,
    mirror_id: u16,
) -> usize {
    let transfers = world.drain_pending_jail_cross_area_transfers();
    if transfers.is_empty() {
        return 0;
    }
    let mut applied = 0;
    for transfer in transfers {
        let transferred = attempt_cross_area_transfer(
            world,
            runtime,
            character_repository,
            area_repository,
            area_id,
            mirror_id,
            transfer.target_id,
            transfer.target_area,
            u32::from(mirror_id),
            transfer.target_x,
            transfer.target_y,
        )
        .await;
        if !transferred {
            world.queue_system_text(
                transfer.caller_id,
                "Nothing happens - target area server is down.".to_string(),
            );
        }
        applied += 1;
    }
    applied
}

/// The Macro Daemon's cross-server "challenge room" hand-off (C
/// `change_area`, `src/module/base.c:1110` for the suspicion-triggered
/// banishment, `848-850` for the correct-answer return trip): resolves
/// every `World::drain_pending_macro_cross_area_transfers` entry (queued
/// by `ugaris-server/src/macro_daemon.rs` when the challenge-room/
/// original-area destination differs from this area server's own
/// `area_id` - see `world/macro_npc.rs`'s module doc comment) via the
/// shared `attempt_cross_area_transfer` helper, same as every other
/// cross-area call site. Like C's own `change_area` call sites here, a
/// failed hand-off is not specially handled - C never checks `change_
/// area`'s return value at either call site either, so a down target
/// area server simply leaves the character in place with no message
/// (weaker than `apply_dungeon_eviction_transfers`'s "system-triggered,
/// no caller to notify" precedent, which at least falls back to
/// `remove_character` - not needed here since `attempt_cross_area_
/// transfer` itself already guarantees no despawn happened on a lookup
/// failure, so "leave the character exactly where they were" is already
/// the correct fallback with no extra code).
pub(crate) async fn apply_macro_cross_area_transfers(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
    area_id: u16,
    mirror_id: u16,
) -> usize {
    let transfers = world.drain_pending_macro_cross_area_transfers();
    if transfers.is_empty() {
        return 0;
    }
    let mut applied = 0;
    for transfer in transfers {
        attempt_cross_area_transfer(
            world,
            runtime,
            character_repository,
            area_repository,
            area_id,
            mirror_id,
            transfer.character_id,
            transfer.target_area,
            u32::from(mirror_id),
            transfer.target_x,
            transfer.target_y,
        )
        .await;
        applied += 1;
    }
    applied
}

/// `build_remove_tile`'s evicted-player cross-area rescue (C
/// `change_area(cn, ch[cn].resta, ch[cn].restx, ch[cn].resty)`,
/// `src/area/13/dungeon.c:754`'s tail): resolves every `World::
/// drain_pending_dungeon_eviction_transfers` entry (queued by
/// `World::build_remove_tile` when the evicted player's own `rest_area`
/// differs from this area server's own `area_id` - see
/// `world/dungeon_master.rs`'s module doc comment) via the shared
/// `attempt_cross_area_transfer` helper, same as every other cross-area
/// call site. The destination mirror always equals this process's own
/// `mirror_id` (rest points carry no mirror field of their own, matching
/// C's `change_area` reading `ch[cn].mirror`). Unlike every other
/// call site, C's own fallback on failure is `exit_char(cn)` (no
/// message - the character has no "down" feedback path here since
/// `exit_char` disconnects them entirely), so a failed hand-off calls
/// `World::remove_character` instead of queuing a system text.
pub(crate) async fn apply_dungeon_eviction_transfers(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
    area_id: u16,
    mirror_id: u16,
) -> usize {
    let transfers = world.drain_pending_dungeon_eviction_transfers();
    if transfers.is_empty() {
        return 0;
    }
    let mut applied = 0;
    for transfer in transfers {
        let transferred = attempt_cross_area_transfer(
            world,
            runtime,
            character_repository,
            area_repository,
            area_id,
            mirror_id,
            transfer.character_id,
            transfer.target_area,
            u32::from(mirror_id),
            transfer.target_x,
            transfer.target_y,
        )
        .await;
        if !transferred {
            world.remove_character(transfer.character_id);
        }
        applied += 1;
    }
    applied
}

/// `/rmdeath`'s async DB round trip (C `lookup_name`, `system/lookup.c:
/// 42-98` + `system/database/database_lookup.c:57-83`): resolves every
/// `World::drain_pending_rmdeath_lookups` entry (queued by a
/// validly-shaped `/rmdeath <name>` argument - see `World::
/// queue_rmdeath_lookup`'s and `apply_admin_character_command`'s doc
/// comments) against the DB.
///
/// - no DB row -> "No character by the name %s." (C's dispatcher-level
///   `lookup_name == -1` branch, `command.c:8896`-equivalent).
/// - a row found -> hands off to `World::resolve_rmdeath_lookup`, which
///   reproduces `cmd_removedeath`'s online-only deviation (see
///   `world/rmdeath.rs`'s module doc comment) and, on a match, decrements
///   the target's `deaths` counter (no match -> "No player by that
///   name.").
///
/// No-ops entirely (silent) when no `character_repository` is configured
/// or a query errors, matching every sibling offline-DB-lookup event.
pub(crate) async fn apply_rmdeath_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let lookups = world.drain_pending_rmdeath_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        match repository.find_login_target(&lookup.target_name).await {
            Ok(Some(_)) => {
                world.resolve_rmdeath_lookup(lookup.caller_id, &lookup.target_name);
            }
            Ok(None) => {
                world.queue_system_text(
                    lookup.caller_id,
                    format!("No character by the name {}.", lookup.target_name),
                );
            }
            Err(_) => continue,
        }
        applied += 1;
    }
    applied
}

/// `cmd_complain`'s async DB round trip (C `command.c:2320-2350`,
/// `lookup_name`/`db_lookup_name`, `system/lookup.c:42-98` +
/// `system/database/database_lookup.c:57-83`): resolves every `World::
/// drain_pending_complain_lookups` entry (queued by a validly-shaped
/// `/complain <name>` argument - see `World::queue_complain_lookup`'s and
/// `ugaris-server`'s `apply_complain_command`'s doc comments for every
/// other, purely synchronous branch) against the DB.
///
/// - no DB row -> "Sorry, no player by the name '%s' found." delivered
///   via `World::queue_system_text` (matching `cmd_complain`'s own
///   `ret < 0` branch, `command.c:2341-2343`).
/// - a row found -> `ppd->complaint_date = realtime;` (`command.c:2346`)
///   is applied to the *requester's* own `PlayerRuntime` if they're still
///   online (a real gap from C, where the whole function runs inside one
///   blocking call so the caller can never have logged out mid-lookup;
///   silently skipped here otherwise, matching every other
///   offline-DB-lookup event in this file) plus the "Your complaint about
///   '%s' has been sent to game management." confirmation, using the
///   DB's properly-capitalized name (C's `realname` out-parameter).
///   `write_scrollback` (emailing the complaint) has no Rust equivalent -
///   see `apply_complain_command`'s doc comment.
///
/// No-ops entirely (silent) when no `character_repository` is configured
/// or a query errors, matching every sibling offline-DB-lookup event.
pub(crate) async fn apply_complain_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    now_unix: i64,
) -> usize {
    let lookups = world.drain_pending_complain_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let found_name = match repository.find_login_target(&lookup.target_name).await {
            Ok(Some(summary)) => summary.name,
            Ok(None) => {
                world.queue_system_text(
                    lookup.requester_id,
                    format!(
                        "Sorry, no player by the name '{}' found.",
                        lookup.target_name
                    ),
                );
                continue;
            }
            Err(_) => continue,
        };
        if let Some(player) = runtime.player_for_character_mut(lookup.requester_id) {
            player.record_complaint(now_unix as i32);
        }
        world.queue_system_text(
            lookup.requester_id,
            format!("Your complaint about '{found_name}' has been sent to game management."),
        );
        applied += 1;
    }
    applied
}

/// C `cmd_flag`'s offline fallback, `task_set_flags`/`set_flags`
/// (`task.c:198-211,385-394`), resolved for every `World::
/// drain_pending_admin_flag_toggles` entry queued by `World::
/// apply_cmd_flag_command` (see that method's doc comment and
/// `world/admin_flag.rs`'s module doc comment for the full message-shape
/// breakdown):
/// - no DB row at all -> "Sorry, no player by the name %s." (C's
///   synchronous `lookup_name == -1` case, deferred here since this
///   codebase has no synchronous name-index cache to check first).
/// - a row found -> immediate "Update scheduled." feedback
///   (`command.c:2896`), sent regardless of whether the mutation below
///   actually succeeds (C's fire-and-forget `task_set_flags` semantics).
/// - target already online elsewhere -> silent no-op beyond the above
///   (C `set_task`'s "online somewhere else" guard, `task.c:250-253`,
///   only `xlog`s).
/// - otherwise -> mutate the flag, guarded save
///   (`CharacterSaveMode::Backup`, pinning the expected offline
///   `current_area`/`current_mirror` exactly like every other
///   offline-DB-mutation event in this file), then `"Set flag on %s to
///   %s."` (`task.c:208` - genuinely different wording from the online
///   branch's `"Set %s %s to %s."`, since `set_flags`'s task-queue
///   completion handler has no access to `cmd_flag`'s `fptr` name
///   lookup; preserved as-is, not "fixed").
pub(crate) async fn apply_admin_flag_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let toggles = world.drain_pending_admin_flag_toggles();
    if toggles.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for toggle in toggles {
        let Ok(Some(summary)) = repository.find_login_target(&toggle.target_name).await else {
            world.queue_system_text(
                toggle.caller_id,
                format!("Sorry, no player by the name {}.", toggle.target_name),
            );
            continue;
        };
        world.queue_system_text(toggle.caller_id, "Update scheduled.".to_string());

        let Ok(Some(snapshot)) = repository.load_character_snapshot(summary.id).await else {
            continue;
        };
        // C `set_task`'s "online somewhere else" guard (`task.c:250-253`):
        // silent no-op (only an `xlog`, no player-facing message).
        if snapshot.current_area != 0 {
            continue;
        }

        let mut character = snapshot.character;
        character.flags.toggle(toggle.flag);
        let state = if character.flags.contains(toggle.flag) {
            "on"
        } else {
            "off"
        };
        let target_display_name = character.name.clone();

        let request = ugaris_db::CharacterSaveRequest {
            character,
            items: snapshot.items,
            // Offline mutation: None preserves the stored JSON via coalesce.
            player_state_json: None,
            ppd_blob: snapshot.ppd_blob,
            subscriber_blob: snapshot.subscriber_blob,
            mode: ugaris_db::CharacterSaveMode::Backup {
                expected_current_area: snapshot.current_area,
                expected_current_mirror: snapshot.current_mirror,
                mirror: snapshot.mirror,
            },
        };
        if !matches!(repository.save_character_snapshot(request).await, Ok(true)) {
            continue;
        }

        world.queue_system_text(
            toggle.caller_id,
            format!("Set flag on {target_display_name} to {state}."),
        );
        applied += 1;
    }
    applied
}

/// `/punish <name> <level> <reason>`'s async DB round trip (C
/// `task_punish_player`/`punish_player`/`punish`, `src/system/task.c:
/// 171-188,213-295,358-373` + `src/system/punish.c:41-107`): resolves
/// every `World::drain_pending_punish_requests` entry (queued by
/// `World::queue_punish_command` - see `world/punish.rs`'s module doc
/// comment) the same "online (any loaded character) first, else read/
/// mutate/write the persisted row, else silently no-op if logged in
/// elsewhere" way `apply_admin_flag_events` already established, with
/// [`apply_punishment`] providing the shared karma/exp mutation for both
/// branches.
///
/// - no DB row at all -> "Sorry, no player by the name %s." (C's
///   synchronous `lookup_name == -1` case).
/// - online target -> mutated immediately in `World::characters`; if the
///   result triggers a lock or kick (`PunishmentOutcome::lock`/`kick`)
///   and the target has a live session, sends the exit message and
///   requests a disconnect - this funnels through the exact same
///   `SessionEvent::Disconnected` -> `enter_lostcon_on_disconnect`
///   machinery a real network drop uses, matching C `kick_player`
///   (`player.c:174-202`) far more closely than a `/kick`-style full
///   `exit_char` teardown would (see `world/punish.rs`'s module doc
///   comment).
/// - offline target already logged in elsewhere (`current_area != 0`) ->
///   silent no-op (C `set_task`'s "online somewhere else" guard,
///   `task.c:238-243`, only `xlog`s).
/// - offline target -> loaded, mutated, and saved back
///   (`CharacterSaveMode::Backup`, pinning the expected offline
///   `current_area`/`current_mirror` like every other offline-DB-
///   mutation event in this file); a lock/kick outcome only updates the
///   persisted `locked` column here (there is no live session to
///   disconnect).
///
/// Both branches write the `kind = 1` punishment `notes` row (best
/// effort - a write failure does not roll back the mutation or suppress
/// the player-facing messages, see the module doc comment in
/// `world/punish.rs` for why) and message the caller with "Punished %s
/// with a level %d punishment for %s"; an online target additionally
/// gets the level-specific warning/punishment text (C `punish_player`,
/// `task.c:171-188`) - an offline target has no live session to deliver
/// that second message to, so it is silently skipped (matching every
/// other offline-mutation event's caller-only feedback in this file,
/// e.g. `apply_rename_events`).
///
/// No-ops entirely (silent, but still drains the queue) when no
/// `character_repository` is configured, matching every sibling
/// offline-DB-mutation event in this file.
pub(crate) async fn apply_punish_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    notes_repository: &Option<ugaris_db::PgNotesRepository>,
    now_unix: i64,
) -> usize {
    let requests = world.drain_pending_punish_requests();
    if requests.is_empty() {
        return 0;
    }
    let Some(character_repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for request in requests {
        if let Some(target_id) = world.find_punish_target_online(&request.target_name) {
            let Some(character) = world.characters.get_mut(&target_id) else {
                continue;
            };
            let outcome = apply_punishment(character, request.level);
            let target_name = character.name.clone();
            let paid = character.flags.contains(CharacterFlags::PAID);
            let karma_after = character.karma;

            if let Some(notes_repository) = notes_repository {
                let note = PunishmentNote {
                    level: request.level as i32,
                    exp: outcome.exp_loss as i32,
                    karma: outcome.karma_loss,
                    reason: request.reason.clone(),
                };
                let _ = notes_repository
                    .add_note(
                        target_id,
                        PUNISHMENT_NOTE_KIND,
                        request.caller_id,
                        &encode_punishment_note(&note),
                        now_unix,
                    )
                    .await;
            }

            world.queue_system_text(
                request.caller_id,
                format!(
                    "Punished {target_name} with a level {} punishment for {}",
                    request.level, request.reason
                ),
            );
            if request.level == 0 {
                world.queue_system_text(
                    target_id,
                    format!(
                        "You have been warned for {}. You will not be warned again. Next time you will lose experience and karma.",
                        request.reason
                    ),
                );
            } else {
                let threshold = if paid { -12 } else { -5 };
                world.queue_system_text(
                    target_id,
                    format!(
                        "You have just been punished for {}. You have lost experience and karma. Your karma is now down to {karma_after}. If your karma reaches {threshold}, you will be banned from this game.",
                        request.reason
                    ),
                );
            }

            if outcome.lock || outcome.kick {
                let _ = character_repository
                    .set_character_locked(target_id, true)
                    .await;
                let mut builder = PacketBuilder::new();
                builder.exit("You have been locked as a result of your punishment.");
                let payload = builder.into_payload();
                for (session_id, _) in runtime.sessions_for_character(target_id) {
                    runtime.send_to_session(session_id, payload.clone());
                    runtime.flush_session(session_id);
                    if let Some(commands) = runtime.sessions.get(&session_id) {
                        let _ = commands.try_send(SessionCommand::Disconnect);
                    }
                }
            }
            applied += 1;
            continue;
        }

        let Ok(Some(summary)) = character_repository
            .find_login_target(&request.target_name)
            .await
        else {
            world.queue_system_text(
                request.caller_id,
                format!("Sorry, no player by the name {}.", request.target_name),
            );
            continue;
        };
        let Ok(Some(snapshot)) = character_repository
            .load_character_snapshot(summary.id)
            .await
        else {
            continue;
        };
        // C `set_task`'s "online somewhere else" guard (`task.c:238-243`):
        // silent no-op (only an `xlog`, no player-facing message).
        if snapshot.current_area != 0 {
            continue;
        }

        let mut character = snapshot.character;
        let outcome = apply_punishment(&mut character, request.level);
        let target_name = character.name.clone();
        let target_id = character.id;

        let save_request = ugaris_db::CharacterSaveRequest {
            character,
            items: snapshot.items,
            // Offline mutation: None preserves the stored JSON via coalesce.
            player_state_json: None,
            ppd_blob: snapshot.ppd_blob,
            subscriber_blob: snapshot.subscriber_blob,
            mode: ugaris_db::CharacterSaveMode::Backup {
                expected_current_area: snapshot.current_area,
                expected_current_mirror: snapshot.current_mirror,
                mirror: snapshot.mirror,
            },
        };
        if !matches!(
            character_repository
                .save_character_snapshot(save_request)
                .await,
            Ok(true)
        ) {
            continue;
        }

        if let Some(notes_repository) = notes_repository {
            let note = PunishmentNote {
                level: request.level as i32,
                exp: outcome.exp_loss as i32,
                karma: outcome.karma_loss,
                reason: request.reason.clone(),
            };
            let _ = notes_repository
                .add_note(
                    target_id,
                    PUNISHMENT_NOTE_KIND,
                    request.caller_id,
                    &encode_punishment_note(&note),
                    now_unix,
                )
                .await;
        }
        if outcome.lock || outcome.kick {
            let _ = character_repository
                .set_character_locked(target_id, true)
                .await;
        }

        world.queue_system_text(
            request.caller_id,
            format!(
                "Punished {target_name} with a level {} punishment for {}",
                request.level, request.reason
            ),
        );
        applied += 1;
    }
    applied
}

/// `/unpunish <name> <note id>`'s async DB round trip (C
/// `task_unpunish_player`/`unpunish_player`/`unpunish`, `src/system/
/// task.c:171,190-193,213-295,374-382` + `src/system/punish.c:109-131`):
/// resolves every `World::drain_pending_unpunish_requests` entry (queued
/// by `World::queue_unpunish_command`) the same online-first/offline-
/// fallback way [`apply_punish_events`] does.
///
/// - no DB row at all -> "Sorry, no player by the name %s.".
/// - a row found -> "UnPunishment scheduled." (C's unconditional,
///   fire-and-forget acknowledgement, `command.c:2729`), then:
///   - no `notes` row exists for `note_id` (already unpunished, wrong
///     id, or a note against a *different* character - C's `db_unpunish`
///     has no `uID` scoping either, see `crates/ugaris-db/src/notes.rs`'s
///     module doc comment) -> no further mutation or message (C's
///     `unpunish()` returning `0` short-circuits `unpunish_player`'s own
///     "UnPunished %s ID %d." message too).
///   - a row exists -> refunds the exp/karma it recorded
///     ([`apply_unpunishment`]), unconditionally unlocks the account
///     (C `plock = -1`, `punish.c:127-129`), and messages the caller
///     "UnPunished %s ID %d." (no message to the target - a real
///     asymmetry with `/punish`, preserved as-is).
///
/// No-ops entirely (silent, but still drains the queue) when no
/// `character_repository` is configured.
pub(crate) async fn apply_unpunish_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    notes_repository: &Option<ugaris_db::PgNotesRepository>,
) -> usize {
    let requests = world.drain_pending_unpunish_requests();
    if requests.is_empty() {
        return 0;
    }
    let Some(character_repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for request in requests {
        if let Some(target_id) = world.find_punish_target_online(&request.target_name) {
            let Some(character) = world.characters.get_mut(&target_id) else {
                continue;
            };
            let target_name = character.name.clone();
            world.queue_system_text(request.caller_id, "UnPunishment scheduled.".to_string());

            let Some(notes_repository) = notes_repository else {
                continue;
            };
            let Ok(Some(content)) = notes_repository.take_note(request.note_id).await else {
                continue;
            };
            let Some(note) = decode_punishment_note(&content) else {
                continue;
            };
            let Some(character) = world.characters.get_mut(&target_id) else {
                continue;
            };
            apply_unpunishment(character, &note);
            let _ = character_repository
                .set_character_locked(target_id, false)
                .await;
            world.queue_system_text(
                request.caller_id,
                format!("UnPunished {target_name} ID {}.", request.note_id),
            );
            applied += 1;
            continue;
        }

        let Ok(Some(summary)) = character_repository
            .find_login_target(&request.target_name)
            .await
        else {
            world.queue_system_text(
                request.caller_id,
                format!("Sorry, no player by the name {}.", request.target_name),
            );
            continue;
        };
        world.queue_system_text(request.caller_id, "UnPunishment scheduled.".to_string());

        let Ok(Some(snapshot)) = character_repository
            .load_character_snapshot(summary.id)
            .await
        else {
            continue;
        };
        if snapshot.current_area != 0 {
            continue;
        }
        let Some(notes_repository) = notes_repository else {
            continue;
        };
        let Ok(Some(content)) = notes_repository.take_note(request.note_id).await else {
            continue;
        };
        let Some(note) = decode_punishment_note(&content) else {
            continue;
        };

        let mut character = snapshot.character;
        apply_unpunishment(&mut character, &note);
        let target_name = character.name.clone();
        let target_id = character.id;

        let save_request = ugaris_db::CharacterSaveRequest {
            character,
            items: snapshot.items,
            // Offline mutation: None preserves the stored JSON via coalesce.
            player_state_json: None,
            ppd_blob: snapshot.ppd_blob,
            subscriber_blob: snapshot.subscriber_blob,
            mode: ugaris_db::CharacterSaveMode::Backup {
                expected_current_area: snapshot.current_area,
                expected_current_mirror: snapshot.current_mirror,
                mirror: snapshot.mirror,
            },
        };
        if !matches!(
            character_repository
                .save_character_snapshot(save_request)
                .await,
            Ok(true)
        ) {
            continue;
        }
        let _ = character_repository
            .set_character_locked(target_id, false)
            .await;
        world.queue_system_text(
            request.caller_id,
            format!("UnPunished {target_name} ID {}.", request.note_id),
        );
        applied += 1;
    }
    applied
}

/// `/look <name>`'s async DB round trip (C `command.c:8990-9019`'s inline
/// handler + `read_notes`/`db_read_notes`/`list_punishment`,
/// `src/system/database/database_lookup.c:116-124` + `database_notes.c:
/// 164-215` + `src/system/punish.c:26-38`): resolves every `World::
/// drain_pending_look_requests` entry (queued by `World::
/// queue_look_command`) by name via `find_login_target` (C's synchronous
/// `lookup_name`), then lists every `kind = 1` note filed against the
/// resolved character, each row's creator name resolved via
/// `find_name_by_id` (C `lookup_ID`).
///
/// - no matching character -> "No character by the name %s." (folds C's
///   `ID == -1` case; C's `ID == 0` "lookup in progress" case has no
///   analogue here, see `World::queue_look_command`'s doc comment).
/// - a match -> "Looking up character: %s (ID: %d)" (C's own immediate
///   confirmation line, `command.c:9016`), then "Start of Notes:", one
///   `format_look_note_line` per `kind = 1` row (oldest first, matching
///   `NotesRepository::list_notes_for_character`'s `order by id`), then
///   "End of Notes" - every other note `kind` is silently skipped (C's
///   own `default: xlog(...)` branch never reaches the player either).
///
/// No-ops entirely (silent, but still drains the queue) when either
/// `character_repository` or `notes_repository` is unconfigured.
pub(crate) async fn apply_look_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    notes_repository: &Option<ugaris_db::PgNotesRepository>,
) -> usize {
    let requests = world.drain_pending_look_requests();
    if requests.is_empty() {
        return 0;
    }
    let Some(character_repository) = character_repository else {
        return 0;
    };
    let Some(notes_repository) = notes_repository else {
        return 0;
    };
    let mut applied = 0;
    for request in requests {
        let Ok(Some(summary)) = character_repository
            .find_login_target(&request.target_name)
            .await
        else {
            world.queue_system_text(
                request.requester_id,
                format!("No character by the name {}.", request.target_name),
            );
            continue;
        };
        world.queue_system_text(
            request.requester_id,
            format!(
                "Looking up character: {} (ID: {})",
                summary.name, summary.id.0
            ),
        );
        let Ok(notes) = notes_repository.list_notes_for_character(summary.id).await else {
            continue;
        };
        world.queue_system_text(request.requester_id, "Start of Notes:".to_string());
        for note in &notes {
            if note.kind != PUNISHMENT_NOTE_KIND {
                continue;
            }
            let Some(punishment) = decode_punishment_note(&note.content) else {
                continue;
            };
            let creator_name = match character_repository.find_name_by_id(note.creator_id).await {
                Ok(Some(name)) => name,
                _ => "*unknown*".to_string(),
            };
            world.queue_system_text(
                request.requester_id,
                format_look_note_line(note.id, &punishment, &creator_name, note.created_at),
            );
        }
        world.queue_system_text(request.requester_id, "End of Notes".to_string());
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_look_events`], split out so
/// the date arithmetic can be unit-tested without a live database. C
/// `list_punishment` (`src/system/punish.c:26-38`)'s `localtime` is
/// approximated in UTC, matching this codebase's established convention
/// (see `clan_log.rs`'s `format_clan_log_entries` doc comment) since no
/// `chrono`/timezone-database dependency exists in this workspace.
pub(crate) fn format_look_note_line(
    note_id: i64,
    note: &PunishmentNote,
    creator_name: &str,
    created_at: i64,
) -> String {
    let (year, month, day) = civil_from_unix_seconds(created_at.max(0) as u64);
    let seconds_of_day = created_at.max(0) as u64 % 86_400;
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;
    format!(
        "P{note_id}: Level: {}, Exp: {}, Karma: {}, Creator: {creator_name}, Date: {month:02}/{day:02}/{year:04} {hour:02}:{minute:02}:{second:02}, Reason: {}",
        note.level, note.exp, note.karma, note.reason
    )
}

/// `/klog`'s async DB round trip (C `command.c:9022-9024` -> `karmalog`
/// -> `db_karmalog`/`karmalog_s`, `src/system/database/database_notes.c:
/// 230-275`): resolves every `World::drain_pending_klog_requests` entry
/// (queued by `World::queue_klog_command`, which takes no argument -
/// unlike `/look`, there is nothing to validate before queuing) against
/// a single shared `NotesRepository::list_recent_notes` query (the last
/// 24 hours, matching C's `date >= now - 86400` cutoff), reused across
/// every requester in the drained batch rather than re-querying per
/// caller.
///
/// Replies "Karmalog:", one `format_klog_line` per `kind = 1` row (newest
/// first, matching `list_recent_notes`'s `order by date desc`), then
/// "---" (C's own trailing separator, `database_notes.c:273`) - every
/// other note `kind` is silently skipped, same as `/look`. A row whose
/// target or creator id no longer resolves to a name falls back to
/// `"*unknown*"`, matching C `lookup_ID`'s own `"*unknown*"` fallback for
/// a cache slot with no name recorded (this codebase has no analogue of
/// C's separate `"**deleted**"` case, since it has no in-memory
/// name/ID cache to distinguish "never resolved" from "resolved to a
/// numeric placeholder" - see `CharacterRepository::find_name_by_id`'s
/// doc comment).
///
/// No-ops entirely (silent, but still drains the queue) when either
/// `character_repository` or `notes_repository` is unconfigured.
pub(crate) async fn apply_klog_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    notes_repository: &Option<ugaris_db::PgNotesRepository>,
    now_unix: i64,
) -> usize {
    let requesters = world.drain_pending_klog_requests();
    if requesters.is_empty() {
        return 0;
    }
    let Some(character_repository) = character_repository else {
        return 0;
    };
    let Some(notes_repository) = notes_repository else {
        return 0;
    };
    let since_unix = now_unix - 60 * 60 * 24;
    let Ok(notes) = notes_repository.list_recent_notes(since_unix).await else {
        return 0;
    };
    let mut applied = 0;
    for requester_id in requesters {
        world.queue_system_text(requester_id, "Karmalog:".to_string());
        for note in &notes {
            if note.kind != PUNISHMENT_NOTE_KIND {
                continue;
            }
            let Some(target_id) = note.target_id else {
                continue;
            };
            let Some(punishment) = decode_punishment_note(&note.content) else {
                continue;
            };
            let offender_name = match character_repository.find_name_by_id(target_id).await {
                Ok(Some(name)) => name,
                _ => "*unknown*".to_string(),
            };
            let creator_name = match character_repository.find_name_by_id(note.creator_id).await {
                Ok(Some(name)) => name,
                _ => "*unknown*".to_string(),
            };
            world.queue_system_text(
                requester_id,
                format_klog_line(
                    &offender_name,
                    punishment.karma,
                    &creator_name,
                    &punishment.reason,
                    note.created_at,
                ),
            );
        }
        world.queue_system_text(requester_id, "---".to_string());
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_klog_events`] - see that
/// function's doc comment. C `karmalog_s` (`database_notes.c:227-244`)
/// prints only the time of day, not the full date (unlike
/// [`format_look_note_line`]'s sibling `list_punishment` format).
pub(crate) fn format_klog_line(
    offender_name: &str,
    karma: i32,
    creator_name: &str,
    reason: &str,
    created_at: i64,
) -> String {
    let seconds_of_day = created_at.max(0) as u64 % 86_400;
    let hour = seconds_of_day / 3600;
    let minute = (seconds_of_day % 3600) / 60;
    let second = seconds_of_day % 60;
    format!(
        "{offender_name}, {karma} Karma from {creator_name} for {reason} at {hour:02}:{minute:02}:{second:02}."
    )
}

/// `/showvalues <name>`'s async DB round trip (C `command.c:8401-8409` ->
/// `show_values`, `command.c:521-537` + its `server_chat` body
/// `show_values_bg`, `src/system/tool.c:2940-3096`): resolves every
/// `World::drain_pending_showvalues_requests` entry (queued by `World::
/// queue_showvalues_command`) by name via `find_login_target` (C's
/// synchronous `lookup_name`).
///
/// - no matching character -> "No player by that name." (C's `ID == -1`
///   branch; C's `ID == 0` "lookup in progress" case has no analogue
///   here, same as every sibling name-lookup command in this codebase).
/// - a match -> the caller always gets the "Sent." confirmation (C logs
///   this unconditionally once `lookup_name` succeeds, regardless of
///   which area server - if any - currently has the target loaded), then
///   the caller's own `show_values_lines` stat block is delivered to the
///   target *only if the target happens to be loaded in this process's
///   `World`* - C's real delivery goes through `tell_chat`'s own
///   cross-area chat relay, which this codebase does not have yet (see
///   the "Cross-area transfer" `PORTING_TODO.md` entry's gap (2) and
///   `world/values.rs`'s module doc comment for the full single-process
///   caveat).
///
/// No-ops entirely (silent, but still drains the queue) when
/// `character_repository` is unconfigured.
pub(crate) async fn apply_showvalues_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let requests = world.drain_pending_showvalues_requests();
    if requests.is_empty() {
        return 0;
    }
    let Some(character_repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for request in requests {
        let Ok(Some(summary)) = character_repository
            .find_login_target(&request.target_name)
            .await
        else {
            world.queue_system_text(request.caller_id, "No player by that name.".to_string());
            continue;
        };
        let Some(caller) = world.characters.get(&request.caller_id) else {
            continue;
        };
        let lines = show_values_lines(caller, &world.items);
        world.queue_system_text(request.caller_id, "Sent.".to_string());
        if world.characters.contains_key(&summary.id) {
            for line in lines {
                world.queue_system_text(summary.id, line);
            }
        }
        applied += 1;
    }
    applied
}

/// `/values <name>`'s async DB round trip (C `command.c:8391-8399` ->
/// `look_values`, `command.c:501-519` + its `server_chat` body
/// `look_values_bg`, `src/system/tool.c:2882-2939`): resolves every
/// `World::drain_pending_values_requests` entry (queued by `World::
/// queue_values_command`) by name via `find_login_target` (C's
/// synchronous `lookup_name`), same as `/showvalues` above.
///
/// Unlike `/showvalues`'s caller/target role swap, `/values` keeps the
/// caller as the caller throughout: every reply line goes back to
/// `request.caller_id`, showing the *resolved target's* stats (see
/// `world/values.rs`'s module doc comment for the contrast, and C's own
/// `tell_chat(0, cnID, 1, ...)` calls in `look_values_bg`, all addressed
/// to the caller `cnID`, never the target `coID`).
///
/// - no matching character -> "No player by that name." (C's `ID == -1`
///   branch).
/// - a match not currently loaded in this process's `World` -> silent
///   no-op (C's `if (!co) return;` in `look_values_bg` - no message at
///   all, matching this codebase's single-process-only cross-area chat
///   caveat, see `world/values.rs`'s module doc comment).
/// - a match with no resolvable `find_paid_until_info` row (a data
///   inconsistency - a live `World` character with no matching DB
///   `accounts` join - never hit for a real player) -> silent no-op,
///   same as the offline case.
/// - a loaded match -> the caller receives every `values_lines` line:
///   `PlayerRuntime::stats_online_time`/`bank_gold`/`current_mirror_id`
///   come from `ServerRuntime::player_for_character` when the target has
///   a live session (defaulting to `0`/`0`/this server's own `mirror_id`
///   when it does not - e.g. an offline-but-somehow-`World`-resident
///   NPC, never hit for a real logged-in player); the mirror-area
///   section name comes from `section_at(area_id, x, y)` (C's
///   `get_section_name`, implicitly scoped to this server process's own
///   `areaID`), falling back to `""` when no section matches (see
///   `values_lines`'s own doc comment for why).
///
/// No-ops entirely (silent, but still drains the queue) when
/// `character_repository` is unconfigured.
pub(crate) async fn apply_values_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_id: u16,
    mirror_id: u16,
    now_unix: i64,
) -> usize {
    let requests = world.drain_pending_values_requests();
    if requests.is_empty() {
        return 0;
    }
    let Some(character_repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for request in requests {
        let Ok(Some(summary)) = character_repository
            .find_login_target(&request.target_name)
            .await
        else {
            world.queue_system_text(request.caller_id, "No player by that name.".to_string());
            continue;
        };
        let Some(target) = world.characters.get(&summary.id).cloned() else {
            continue;
        };
        let Ok(Some(paid_info)) = character_repository.find_paid_until_info(summary.id).await
        else {
            continue;
        };
        let (paid_till, is_paid) = compute_paid_till(
            paid_info.raw_paid_until_unix,
            paid_info.account_created_at_unix,
            now_unix,
        );
        let (online_minutes, bank_gold, current_mirror) = runtime
            .player_for_character(summary.id)
            .map(|player| {
                (
                    player.stats_online_time(),
                    player.bank_gold,
                    player.current_mirror_id,
                )
            })
            .unwrap_or((0, 0, mirror_id));
        let section_name = section_at(area_id, usize::from(target.x), usize::from(target.y))
            .map(|section| section.name)
            .unwrap_or("");
        let lines = values_lines(
            &target,
            &world.items,
            is_paid,
            paid_till,
            now_unix,
            online_minutes,
            bank_gold,
            current_mirror,
            mirror_id,
            area_id,
            section_name,
        );
        for line in lines {
            world.queue_system_text(request.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// `/allow <name>`'s async DB round trip (C `command.c:8371-8378` ->
/// `allow_body`, `src/system/death.c:1013-1029` + its `server_chat` body
/// `allow_body_db`, `death.c:1045-1067`): resolves every `World::
/// drain_pending_allow_requests` entry (queued by `World::
/// queue_allow_command`) by name via `find_login_target` (C's
/// synchronous `lookup_name`), then grants the resolved target access to
/// every grave `World::grant_grave_access_to` finds owned by the caller
/// in this process's own `World` (see `world/allow.rs`'s module doc
/// comment for the single-process-only caveat shared with every other
/// name-lookup command here).
///
/// - no matching character -> "No player by that name." (C's `coID ==
///   -1` branch).
/// - a match -> C's `allow_body` unconditionally logs "Order
///   scheduled." once `lookup_name` resolves, then `allow_body_db`
///   (run per-area-server against the broadcast) replies "Area %d:
///   Allowed access to %d corpses." with its own local count - both
///   lines are sent here, in that order, once resolution completes
///   (this codebase collapses C's two-step broadcast-then-local-reply
///   into one async round trip, matching every other documented
///   cross-area gap).
///
/// No-ops entirely (silent, but still drains the queue) when
/// `character_repository` is unconfigured.
pub(crate) async fn apply_allow_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_id: u16,
) -> usize {
    let requests = world.drain_pending_allow_requests();
    if requests.is_empty() {
        return 0;
    }
    let Some(character_repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for request in requests {
        let Ok(Some(summary)) = character_repository
            .find_login_target(&request.target_name)
            .await
        else {
            world.queue_system_text(request.caller_id, "No player by that name.".to_string());
            continue;
        };
        world.queue_system_text(request.caller_id, "Order scheduled.".to_string());
        let count = world.grant_grave_access_to(request.caller_id, summary.id);
        world.queue_system_text(
            request.caller_id,
            format!("Area {area_id}: Allowed access to {count} corpses."),
        );
        applied += 1;
    }
    applied
}

/// `/rename <from> <to>`'s async DB round trip (C `do_rename`/
/// `db_rename`, `src/system/database/database_admin.c:291-355`):
/// resolves every `World::drain_pending_rename_lookups` entry (queued by
/// a validly-shaped `to` name - see `World::queue_rename_command`'s and
/// `world/rename.rs`'s module doc comment) against `PgCharacterRepository
/// ::rename_character`.
///
/// - a query error (including a unique-name-constraint violation on
///   `to`, which C's own query would likewise fail on if `chars.name` is
///   unique) -> "Failed to change name."
/// - no row matched `from` -> "Didn't work, most probable cause: %s not
///   found."
/// - success -> "Changed %s to %s. The change will be visible after the
///   next login."
///
/// No-ops entirely (silent, but still drains the queue) when no
/// `character_repository` is configured, matching every sibling
/// offline-DB-mutation event in this file.
pub(crate) async fn apply_rename_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let lookups = world.drain_pending_rename_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        match repository
            .rename_character(&lookup.from_name, &lookup.to_name)
            .await
        {
            Ok(true) => {
                world.queue_system_text(
                    lookup.requester_id,
                    format!(
                        "Changed {} to {}. The change will be visible after the next login.",
                        lookup.from_name, lookup.to_name
                    ),
                );
            }
            Ok(false) => {
                world.queue_system_text(
                    lookup.requester_id,
                    format!(
                        "Didn't work, most probable cause: {} not found.",
                        lookup.from_name
                    ),
                );
            }
            Err(_) => {
                world.queue_system_text(lookup.requester_id, "Failed to change name.".to_string());
            }
        }
        applied += 1;
    }
    applied
}

/// `/lockname <name>`'s async DB round trip (C `do_lockname`/
/// `db_lockname`, `src/system/database/database_admin.c:357-398`):
/// resolves every `World::drain_pending_lockname_lookups` entry against
/// `PgCharacterRepository::lock_name` - see `world/lockname.rs`'s module
/// doc comment for the shared validation this queue entry already
/// passed.
///
/// - a query error -> "Failed to insert name."
/// - already locked (no new row inserted) -> "Didn't work, most probable
///   cause: %s already in bad name database."
/// - success -> "Added %s to bad name database."
///
/// Every message uses the *original* (un-lowercased) name, matching C's
/// own `name` parameter (not its `lowercase_name` scratch buffer). No-ops
/// entirely (silent, but still drains the queue) when no
/// `character_repository` is configured, matching every sibling
/// offline-DB-mutation event in this file.
pub(crate) async fn apply_lockname_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let lookups = world.drain_pending_lockname_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        match repository.lock_name(&lookup.lookup_name).await {
            Ok(true) => {
                world.queue_system_text(
                    lookup.requester_id,
                    format!("Added {} to bad name database.", lookup.original_name),
                );
            }
            Ok(false) => {
                world.queue_system_text(
                    lookup.requester_id,
                    format!(
                        "Didn't work, most probable cause: {} already in bad name database.",
                        lookup.original_name
                    ),
                );
            }
            Err(_) => {
                world.queue_system_text(lookup.requester_id, "Failed to insert name.".to_string());
            }
        }
        applied += 1;
    }
    applied
}

/// `/unlockname <name>`'s async DB round trip (C `do_unlockname`/
/// `db_unlockname`, `src/system/database/database_admin.c:436-467`), the
/// mirror image of [`apply_lockname_events`].
///
/// - a query error -> "Failed to delete name."
/// - not locked (no row deleted) -> "Didn't work, most probable cause:
///   %s not in bad name database."
/// - success -> "Deleted %s from bad name database."
pub(crate) async fn apply_unlockname_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let lookups = world.drain_pending_unlockname_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        match repository.unlock_name(&lookup.lookup_name).await {
            Ok(true) => {
                world.queue_system_text(
                    lookup.requester_id,
                    format!("Deleted {} from bad name database.", lookup.original_name),
                );
            }
            Ok(false) => {
                world.queue_system_text(
                    lookup.requester_id,
                    format!(
                        "Didn't work, most probable cause: {} not in bad name database.",
                        lookup.original_name
                    ),
                );
            }
            Err(_) => {
                world.queue_system_text(lookup.requester_id, "Failed to delete name.".to_string());
            }
        }
        applied += 1;
    }
    applied
}

/// `/exterminate <name>`'s async DB round trip (C `exterminate`/
/// `db_exterminate`, `src/system/database/database_admin.c:29-95,
/// 503-507`) - see `world/exterminate.rs`'s module doc comment for why
/// this is a direct account lock + IP ban rather than a `server_chat`
/// relay.
///
/// - target not found -> "Player '%s' not found." (C's exact text,
///   `database_admin.c:92`).
/// - query error -> "Failed to exterminate %s." (this codebase's own
///   error-path convention, matching `apply_lockname_events`/
///   `apply_rename_events` - C has no equivalent distinct message since
///   `db_exterminate` only ever `elog`s and returns on a query failure).
/// - success -> "Locked %d accounts and %d IP addresses." (C's exact
///   wording, `database_admin.c:83`, `nrc`/`nrb` renamed to this
///   codebase's `locked_accounts`/`banned_ips`).
pub(crate) async fn apply_exterminate_events(
    world: &mut World,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) -> usize {
    let requests = world.drain_pending_exterminate_requests();
    if requests.is_empty() {
        return 0;
    }
    let Some(repository) = character_repository else {
        return 0;
    };
    let mut applied = 0;
    for request in requests {
        match repository.exterminate_account(&request.target_name).await {
            Ok(Some(outcome)) => {
                world.queue_system_text(
                    request.caller_id,
                    format!(
                        "Locked {} accounts and {} IP addresses.",
                        outcome.locked_accounts, outcome.banned_ips
                    ),
                );
            }
            Ok(None) => {
                world.queue_system_text(
                    request.caller_id,
                    format!("Player '{}' not found.", request.target_name),
                );
            }
            Err(_) => {
                world.queue_system_text(
                    request.caller_id,
                    format!("Failed to exterminate {}.", request.target_name),
                );
            }
        }
        applied += 1;
    }
    applied
}

#[cfg(test)]
mod lastseen_tests {
    use super::*;
    use ugaris_db::LastSeenInfo;

    #[test]
    fn god_characters_get_the_fixed_recently_message() {
        let info = LastSeenInfo {
            name: "Godmode".to_string(),
            is_god: true,
            last_activity_unix: 0,
        };
        assert_eq!(
            lastseen_reply_message(&info, 1_000_000),
            "Godmode was seen quite recently."
        );
    }

    #[test]
    fn elapsed_time_is_broken_into_days_hours_minutes() {
        let info = LastSeenInfo {
            name: "Player".to_string(),
            is_god: false,
            last_activity_unix: 0,
        };
        // 2 days, 3 hours, 4 minutes = 2*86400 + 3*3600 + 4*60 seconds.
        let now = 2 * 86_400 + 3 * 3_600 + 4 * 60;
        assert_eq!(
            lastseen_reply_message(&info, now),
            "Player was last seen 2 days, 3 hours, 4 minutes ago."
        );
    }

    #[test]
    fn recently_active_player_reports_zero_across_the_board() {
        let info = LastSeenInfo {
            name: "Player".to_string(),
            is_god: false,
            last_activity_unix: 500,
        };
        assert_eq!(
            lastseen_reply_message(&info, 500),
            "Player was last seen 0 days, 0 hours, 0 minutes ago."
        );
    }
}

#[cfg(test)]
mod querystats_tests {
    use super::*;

    #[test]
    fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_querystats_events(&mut world, &None);
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[test]
    fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_querystats_lookup(CharacterId(7));

        let applied = apply_querystats_events(&mut world, &None);
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_querystats_lookups().is_empty());
    }

    #[test]
    fn querystats_lines_reproduce_the_scoped_c_header_and_counters() {
        let stats = ugaris_db::CharacterQueryStats {
            save_char_cnt: 12,
            exit_char_cnt: 3,
            load_char_cnt: 7,
        };
        assert_eq!(
            querystats_lines(stats),
            vec![
                "Database Query Statistics:".to_string(),
                "Character operations:".to_string(),
                "Save chars: 12, Exit chars: 3, Load chars: 7".to_string(),
            ]
        );
    }

    #[test]
    fn querystats_lines_reports_zero_counters_faithfully() {
        let stats = ugaris_db::CharacterQueryStats::default();
        assert_eq!(
            querystats_lines(stats),
            vec![
                "Database Query Statistics:".to_string(),
                "Character operations:".to_string(),
                "Save chars: 0, Exit chars: 0, Load chars: 0".to_string(),
            ]
        );
    }
}

#[cfg(test)]
mod jail_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_jail_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        // Matches every other offline-DB-lookup event in this file: with
        // no `character_repository` configured, the queue is still
        // drained (so it doesn't grow unboundedly) but nothing is
        // resolved and no player-facing message is sent.
        let mut world = World::default();
        world.queue_jail_lookup(
            CharacterId(7),
            "Godmode",
            ugaris_core::world::JailAction::Jail,
        );

        let applied = apply_jail_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_jail_lookups().is_empty());
    }
}

#[cfg(test)]
mod jail_cross_area_transfer_tests {
    use super::*;

    #[tokio::test]
    async fn no_transfers_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        let applied =
            apply_jail_cross_area_transfers(&mut world, &mut runtime, &None, &None, 1, 0).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_pair_falls_back_to_the_shared_down_message() {
        // Mirrors `attempt_cross_area_transfer`'s own
        // `cross_area_transfer_stays_put_without_a_registered_repository_pair`
        // coverage (`tests/cross_area.rs`): without a live
        // `AreaRepository`/`CharacterRepository` pair, the shared helper
        // can't resolve the target, so the caller gets the legacy
        // "Nothing happens - target area server is down." text - the
        // exact fallback `World::apply_jail_action` used to send
        // eagerly before this hand-off was deferred.
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        world.area_id = 1; // current server is NOT the jail area
        world.settings.jail_x = 186;
        world.settings.jail_y = 234;
        world.settings.jail_area = 3;
        let login = LoginBlock {
            name: "Godmode".to_string(),
            password: String::new(),
            vendor: 0,
            client_version: Some(3),
            his_ip: 0,
            our_ip: 0,
            unique: 0,
        };
        world.add_character(login_character(CharacterId(1), &login, 1, 10, 10));
        let mut target_login = login.clone();
        target_login.name = "Baddie".to_string();
        world.add_character(login_character(CharacterId(2), &target_login, 1, 50, 50));
        world.resolve_jail_lookup(
            CharacterId(1),
            "Baddie",
            ugaris_core::world::JailAction::Jail,
        );
        // The synchronous jail/unjail messages (`You have jailed
        // .../You have been jailed by ...`) are not this hand-off's
        // concern - drain them so only the transfer's own feedback
        // remains below.
        world.drain_pending_system_texts();

        let applied =
            apply_jail_cross_area_transfers(&mut world, &mut runtime, &None, &None, 1, 0).await;
        assert_eq!(applied, 1);
        let texts = world.drain_pending_system_texts();
        assert_eq!(texts.len(), 1);
        assert_eq!(texts[0].character_id, CharacterId(1));
        assert_eq!(
            texts[0].message,
            "Nothing happens - target area server is down."
        );
        assert!(world.drain_pending_jail_cross_area_transfers().is_empty());
    }
}

#[cfg(test)]
mod dungeon_eviction_transfer_tests {
    use super::*;

    #[tokio::test]
    async fn no_transfers_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        let applied =
            apply_dungeon_eviction_transfers(&mut world, &mut runtime, &None, &None, 1, 0).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_pair_falls_back_to_removing_the_character() {
        // Mirrors `attempt_cross_area_transfer`'s own
        // `cross_area_transfer_stays_put_without_a_registered_repository_pair`
        // coverage (`tests/cross_area.rs`): without a live
        // `AreaRepository`/`CharacterRepository` pair, the shared helper
        // can't resolve the target, so - unlike every other cross-area
        // call site, which sends "Nothing happens - target area server
        // is down." - this one mirrors C's `exit_char(cn)` fallback and
        // removes the character outright instead (see
        // `world/dungeon_master.rs`'s module doc comment).
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        world.area_id = 13;
        let login = LoginBlock {
            name: "Raider".to_string(),
            password: String::new(),
            vendor: 0,
            client_version: Some(3),
            his_ip: 0,
            our_ip: 0,
            unique: 0,
        };
        let mut raider = login_character(CharacterId(1), &login, 13, 10, 10);
        raider.rest_area = 3; // a different area - queues a cross-area transfer
        raider.rest_x = 50;
        raider.rest_y = 60;
        assert!(world.spawn_character(raider, 10, 10));
        for (x, y) in [(245, 250), (240, 250), (235, 250), (230, 250)] {
            for dx in -1..=1_i32 {
                for dy in -1..=1_i32 {
                    let tx = (x as i32 + dx) as usize;
                    let ty = (y as i32 + dy) as usize;
                    world.map.tile_mut(tx, ty).unwrap().flags |=
                        ugaris_core::map::MapFlags::MOVEBLOCK;
                }
            }
        }
        world.build_remove_tile(10, 10);
        world.drain_pending_system_texts();

        let applied =
            apply_dungeon_eviction_transfers(&mut world, &mut runtime, &None, &None, 13, 0).await;
        assert_eq!(applied, 1);
        assert!(!world.characters.contains_key(&CharacterId(1)));
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_dungeon_eviction_transfers().is_empty());
    }
}

#[cfg(test)]
mod macro_cross_area_transfer_tests {
    use super::*;

    #[tokio::test]
    async fn no_transfers_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        let applied =
            apply_macro_cross_area_transfers(&mut world, &mut runtime, &None, &None, 1, 0).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_pair_leaves_the_character_in_place_with_no_message() {
        // Mirrors `attempt_cross_area_transfer`'s own
        // `cross_area_transfer_stays_put_without_a_registered_repository_pair`
        // coverage (`tests/cross_area.rs`): without a live
        // `AreaRepository`/`CharacterRepository` pair, the shared helper
        // can't resolve the target and never despawns the character - C
        // never checks `change_area`'s return value at either macro-
        // daemon call site either, so this hand-off has no "target area
        // server is down" message to send and no fallback action beyond
        // leaving the character exactly where it already was.
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        world.area_id = 1;
        let login = LoginBlock {
            name: "Victim".to_string(),
            password: String::new(),
            vendor: 0,
            client_version: Some(3),
            his_ip: 0,
            our_ip: 0,
            unique: 0,
        };
        assert!(world.spawn_character(login_character(CharacterId(1), &login, 1, 10, 10), 10, 10));
        world.queue_macro_cross_area_transfer(CharacterId(1), 3, 178, 248);

        let applied =
            apply_macro_cross_area_transfers(&mut world, &mut runtime, &None, &None, 1, 0).await;
        assert_eq!(applied, 1);
        assert!(world.characters.contains_key(&CharacterId(1)));
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_macro_cross_area_transfers().is_empty());
    }
}

#[cfg(test)]
mod rmdeath_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_rmdeath_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        // Matches every other offline-DB-lookup event in this file: with
        // no `character_repository` configured, the queue is still
        // drained (so it doesn't grow unboundedly) but nothing is
        // resolved and no player-facing message is sent.
        let mut world = World::default();
        world.queue_rmdeath_lookup(CharacterId(7), "Godmode");

        let applied = apply_rmdeath_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_rmdeath_lookups().is_empty());
    }
}

#[cfg(test)]
mod complain_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        let applied = apply_complain_events(&mut world, &mut runtime, &None, 1_000).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        // Matches every other offline-DB-lookup event in this file: with
        // no `character_repository` configured, the queue is still
        // drained (so it doesn't grow unboundedly) but nothing is
        // resolved and no player-facing message is sent.
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        world.queue_complain_lookup(CharacterId(7), "Godmode");

        let applied = apply_complain_events(&mut world, &mut runtime, &None, 1_000).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_complain_lookups().is_empty());
    }
}

#[cfg(test)]
mod admin_flag_tests {
    use super::*;

    #[tokio::test]
    async fn no_toggles_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_admin_flag_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_toggle_queued_state_untouched_but_drained() {
        // Matches every other offline-DB-lookup event in this file: with
        // no `character_repository` configured, the queue is still
        // drained (so it doesn't grow unboundedly) but nothing is
        // resolved and no player-facing message is sent.
        let mut world = World::default();
        let messages =
            world.apply_cmd_flag_command(CharacterId(1), "Nobodyhome", CharacterFlags::GOD, "god");
        assert!(messages.is_empty());

        let applied = apply_admin_flag_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_admin_flag_toggles().is_empty());
    }
}

#[cfg(test)]
mod rename_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_rename_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_queue_without_a_reply() {
        let mut world = World::default();
        world.queue_rename_command(CharacterId(1), "Baddie", "Newname");
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_rename_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_rename_lookups().is_empty());
    }
}

#[cfg(test)]
mod lockname_tests {
    use super::*;

    #[tokio::test]
    async fn no_lockname_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_lockname_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_lockname_queue_without_a_reply() {
        let mut world = World::default();
        world.queue_lockname_command(CharacterId(1), "BadName");
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_lockname_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_lockname_lookups().is_empty());
    }

    #[tokio::test]
    async fn no_unlockname_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_unlockname_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_unlockname_queue_without_a_reply() {
        let mut world = World::default();
        world.queue_unlockname_command(CharacterId(1), "BadName");
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_unlockname_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_unlockname_lookups().is_empty());
    }
}

#[cfg(test)]
mod exterminate_tests {
    use super::*;

    #[tokio::test]
    async fn no_requests_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_exterminate_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_queue_without_a_reply() {
        let mut world = World::default();
        world.queue_exterminate_command(CharacterId(1), "Baddie");
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_exterminate_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_exterminate_requests().is_empty());
    }
}

#[cfg(test)]
mod punish_tests {
    use super::*;

    #[tokio::test]
    async fn no_punish_requests_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        let applied = apply_punish_events(&mut world, &mut runtime, &None, &None, 1_000).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_punish_queue_without_a_reply() {
        let mut world = World::default();
        let mut runtime = ServerRuntime::default();
        world.queue_punish_command(CharacterId(1), "Baddie", 3, "being quite mean", false);
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_punish_events(&mut world, &mut runtime, &None, &None, 1_000).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_punish_requests().is_empty());
    }

    #[tokio::test]
    async fn no_unpunish_requests_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_unpunish_events(&mut world, &None, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_unpunish_queue_without_a_reply() {
        let mut world = World::default();
        world.queue_unpunish_command(CharacterId(1), "Baddie", 42);
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_unpunish_events(&mut world, &None, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_unpunish_requests().is_empty());
    }
}

#[cfg(test)]
mod klog_tests {
    use super::*;

    #[test]
    fn format_klog_line_matches_c_karmalog_s_shape_time_only_no_date() {
        // 1_000_000_000 unix seconds = 2001-09-09 01:46:40 UTC.
        let line = format_klog_line("Baddie", -4, "Godmode", "being mean", 1_000_000_000);
        assert_eq!(
            line,
            "Baddie, -4 Karma from Godmode for being mean at 01:46:40."
        );
    }

    #[tokio::test]
    async fn no_klog_requests_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_klog_events(&mut world, &None, &None, 1_000).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_klog_queue_without_a_reply() {
        let mut world = World::default();
        world.queue_klog_command(CharacterId(1));
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_klog_events(&mut world, &None, &None, 1_000).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_klog_requests().is_empty());
    }
}

#[cfg(test)]
mod showvalues_tests {
    use super::*;

    #[tokio::test]
    async fn no_showvalues_requests_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_showvalues_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_drains_the_showvalues_queue_without_a_reply() {
        let mut world = World::default();
        world.queue_showvalues_command(CharacterId(1), "Someone");
        assert!(world.drain_pending_system_texts().is_empty());

        let applied = apply_showvalues_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_showvalues_requests().is_empty());
    }
}
