use super::*;

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
