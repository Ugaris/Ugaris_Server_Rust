use super::*;

/// `#acflag <player>`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_flag`
/// (`anticheat.c:568-593`): sets the target session's `status` to
/// `AC_STATUS_FLAGGED` (`AntiCheatRepository::set_status`). C's
/// confirmation is unconditional and same-thread; here the "Manually
/// flagged {name} for review." message is only queued once the async
/// update actually reports a row was touched, matching every other
/// offline-DB-mutation event in this file's silent-skip-on-failure
/// convention.
pub(crate) async fn apply_ac_flag_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_flag_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(true) = repository
            .set_status(lookup.session_id, AC_STATUS_FLAGGED)
            .await
        else {
            continue;
        };
        world.queue_system_text(
            lookup.caller_id,
            format!("Manually flagged {} for review.", lookup.target_name),
        );
        applied += 1;
    }
    applied
}

/// `#acunflag <player>`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_unflag`
/// (`anticheat.c:790-823`): unlike every other member of this family,
/// C's own handler gates on the target's *current* status before
/// mutating anything (`status != AC_STATUS_FLAGGED` -> "is not flagged",
/// a synchronous in-memory read there) - here that gate has to happen
/// after the async `find_session` round trip instead, since this
/// codebase has no in-memory struct to read status from synchronously.
/// A vanished session row is silently skipped (matching every other
/// offline-DB-lookup event's convention), but a session that exists and
/// simply isn't flagged still gets the "is not flagged" reply - a
/// genuine (documented) deviation from the pure silent-skip convention,
/// justified because C's own equivalent branch produces user-facing
/// text too, not a silent no-op. Once past the gate: restores `status`
/// to `AC_STATUS_VERIFIED` (`AntiCheatRepository::set_status`, same as
/// `#acreset`) and flips `ac_player_stats.is_flagged` to `false` for the
/// target's subscriber id (`AntiCheatRepository::set_flagged`, resolved
/// via `account_id_for_session` - see that method's doc comment for why
/// account id isn't threaded through `PlayerRuntime` instead). C's own
/// confirmation is unconditional once past the status gate, even when
/// `target_subscriber <= 0` skips the DB writes entirely
/// (`anticheat.c:816-821`); reproduced here by queuing the confirmation
/// regardless of whether `account_id_for_session` resolved anything,
/// since only the session-status update (guaranteed to succeed, the row
/// having just been read a moment earlier) gates the reply, matching
/// C's real branching exactly rather than this file's usual "reply only
/// once the mutation succeeds" simplification.
pub(crate) async fn apply_ac_unflag_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_unflag_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(Some(info)) = repository.find_session(lookup.session_id).await else {
            continue;
        };
        if info.status != AC_STATUS_FLAGGED {
            world.queue_system_text(
                lookup.caller_id,
                format!("Player '{}' is not flagged.", lookup.target_name),
            );
            continue;
        }
        let Ok(true) = repository
            .set_status(lookup.session_id, AC_STATUS_VERIFIED)
            .await
        else {
            continue;
        };
        if let Ok(Some(account_id)) = repository.account_id_for_session(lookup.session_id).await {
            let _ = repository.set_flagged(account_id, false).await;
        }
        world.queue_system_text(
            lookup.caller_id,
            format!("Removed flagged status from {}.", lookup.target_name),
        );
        applied += 1;
    }
    applied
}

/// `#actrust <player>`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_trust`
/// (`anticheat.c:827-849`): no status gate at all (unlike `#acunflag`),
/// just flips `ac_player_stats.is_trusted` to `true` for the target's
/// subscriber id, resolved via `account_id_for_session` from the
/// already-known session id. Unlike `#acunflag`'s unconditional-once-
/// past-the-gate reply, this codebase's confirmation is only queued once
/// the subscriber id actually resolves and the write succeeds - a
/// documented simplification vs. C's true unconditional reply
/// (`anticheat.c:847-848`, sent even when `target_subscriber <= 0` skips
/// the DB write), justified because a real character's account id is
/// essentially always resolvable here (unlike C's genuinely-fallible
/// synchronous DB lookup at the time `ac_cmd_trust` runs), so the gap
/// only matters for an already-vanished session row - the same case
/// every other offline-DB-mutation event in this file silently skips.
pub(crate) async fn apply_ac_trust_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_trust_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(Some(account_id)) = repository.account_id_for_session(lookup.session_id).await
        else {
            continue;
        };
        if repository.set_trusted(account_id, true).await.is_err() {
            continue;
        }
        world.queue_system_text(
            lookup.caller_id,
            format!("Marked {} as trusted.", lookup.target_name),
        );
        applied += 1;
    }
    applied
}

/// `#acuntrust <player>`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. The "untrust" mirror of
/// `apply_ac_trust_events` (`ac_cmd_untrust`, `anticheat.c:860-882`):
/// identical shape, `set_trusted(account_id, false)` instead of `true`.
pub(crate) async fn apply_ac_untrust_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_untrust_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(Some(account_id)) = repository.account_id_for_session(lookup.session_id).await
        else {
            continue;
        };
        if repository.set_trusted(account_id, false).await.is_err() {
            continue;
        }
        world.queue_system_text(
            lookup.caller_id,
            format!("Removed trusted status from {}.", lookup.target_name),
        );
        applied += 1;
    }
    applied
}

/// `#acwarn <player> [reason]`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_warn`
/// (`anticheat.c:1291-1314`): resolves the subscriber id
/// (`get_subscriberId_from_character`, here `account_id_for_session`) -
/// a `None` result mirrors C's synchronous `subscriber_id <= 0` ->
/// "Could not find subscriber for '{name}'." branch, the one case this
/// event actually skips the rest of the work for. Once a subscriber id
/// is found, C calls `db_ac_issue_warning` *without checking its return
/// value* and then unconditionally sends all four messages (two to the
/// target, two to the caller) - reproduced as-is here too (the `issue_
/// warning` DB write's `Result` is deliberately ignored, matching C's own
/// disregard for it, rather than this file's usual "reply only once the
/// mutation succeeds" convention).
pub(crate) async fn apply_ac_warn_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_warn_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(Some(account_id)) = repository.account_id_for_session(lookup.session_id).await
        else {
            world.queue_system_text(
                lookup.caller_id,
                format!("Could not find subscriber for '{}'.", lookup.target_name),
            );
            continue;
        };
        let _ = repository.issue_warning(account_id).await;
        world.queue_system_text_bytes(
            lookup.target_id,
            legacy_light_red_text_bytes("*** WARNING ***"),
        );
        world.queue_system_text(
            lookup.target_id,
            format!("You have received an anti-cheat warning: {}", lookup.reason),
        );
        world.queue_system_text(
            lookup.target_id,
            "Further violations may result in suspension.".to_string(),
        );
        world.queue_system_text(
            lookup.caller_id,
            format!(
                "Issued warning to {}: {}",
                lookup.target_name, lookup.reason
            ),
        );
        applied += 1;
    }
    applied
}

#[cfg(test)]
mod ac_flag_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_flag_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_flag_lookup(CharacterId(7), "Baddie".to_string(), 30);

        let applied = apply_ac_flag_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_flag_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_unflag_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_unflag_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_unflag_lookup(CharacterId(7), "Baddie".to_string(), 30);

        let applied = apply_ac_unflag_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_unflag_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_trust_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_trust_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_trust_lookup(CharacterId(7), "Baddie".to_string(), 30);

        let applied = apply_ac_trust_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_trust_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_untrust_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_untrust_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_untrust_lookup(CharacterId(7), "Baddie".to_string(), 30);

        let applied = apply_ac_untrust_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_untrust_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_warn_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_warn_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_warn_lookup(
            CharacterId(7),
            CharacterId(8),
            "Baddie".to_string(),
            30,
            "Speedhacking".to_string(),
        );

        let applied = apply_ac_warn_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_warn_lookups().is_empty());
    }
}
