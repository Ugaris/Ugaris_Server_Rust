use super::*;

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
