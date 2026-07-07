use super::*;

/// `#acstatus <name>`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment for why this needs a Postgres
/// round trip in this codebase where C reads its `player[nr]->ac` struct
/// synchronously. Reproduces `ac_cmd_status`'s display block
/// (`anticheat.c:492-516`) as a sequence of `World::queue_system_text`
/// calls, one per line (matching that C function's own one-`log_char`-
/// per-line shape) - see `ac_status_lines` for the exact text. A session
/// row that no longer exists (deleted, or a stale id) is silently
/// skipped, matching every other offline-DB-lookup event in this file.
pub(crate) async fn apply_ac_status_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_status_lookups();
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
        for line in ac_status_lines(&lookup.target_name, &info) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_ac_status_events`], split out
/// so it can be unit-tested without a live database. C `ac_cmd_status`
/// (`anticheat.c:492-516`) - color wrapping dropped, matching `/global`'s
/// established plain-text simplification for admin-only displays.
pub(crate) fn ac_status_lines(
    target_name: &str,
    info: &ugaris_db::AntiCheatSessionInfo,
) -> Vec<String> {
    let mut lines = vec![
        format!("--- Anti-Cheat Status for {target_name} ---"),
        format!("Status: {}", ac_status_string(info.status)),
        format!("Heartbeat violations: {}", info.heartbeat_violations),
        format!("State violations: {}", info.state_violations),
        format!("Challenge failures: {}", info.challenge_failures),
        format!("Bot score: {:.2}", info.bot_score),
        format!("Timeout count: {}", info.timeout_count),
    ];
    if let (Some(major), Some(minor), Some(patch)) =
        (info.mod_major, info.mod_minor, info.mod_patch)
    {
        lines.push(format!("Mod version: {major}.{minor}.{patch}"));
        let os_name = match info.os_type {
            Some(1) => "Windows",
            Some(2) => "Linux",
            Some(3) => "macOS",
            _ => "Unknown",
        };
        lines.push(format!("OS: {os_name}"));
        lines.push(format!(
            "Screen: {}x{}",
            info.screen_w.unwrap_or(0),
            info.screen_h.unwrap_or(0)
        ));
    } else {
        lines.push("Fingerprint: not received".to_string());
    }
    lines
}

/// `#acstats`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_stats`
/// (`anticheat.c:604-628`): per-status tallies over every online
/// `CF_PLAYER` character with a known anticheat session (see the module
/// doc comment for why a session-less online player is simply omitted,
/// not counted as "unverified" by default), plus the single highest
/// `bot_score` and its owner's name. A target whose session row has
/// vanished between the command and this tick is omitted from every
/// tally, matching `find_sessions`'s own silent-omission contract.
pub(crate) async fn apply_ac_stats_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_stats_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let session_ids: Vec<i64> = lookup
            .targets
            .iter()
            .map(|target| target.session_id)
            .collect();
        let Ok(sessions) = repository.find_sessions(&session_ids).await else {
            continue;
        };
        let sessions_by_id: HashMap<i64, ugaris_db::AntiCheatSessionInfo> =
            sessions.into_iter().collect();

        let mut total_players = 0;
        let mut verified = 0;
        let mut unverified = 0;
        let mut suspicious = 0;
        let mut flagged = 0;
        let mut with_fingerprint = 0;
        let mut max_bot_score = 0.0f32;
        let mut max_bot_player = String::new();
        for target in &lookup.targets {
            let Some(info) = sessions_by_id.get(&target.session_id) else {
                continue;
            };
            total_players += 1;
            match info.status {
                1 => verified += 1,
                0 => unverified += 1,
                2 => suspicious += 1,
                3 => flagged += 1,
                _ => {}
            }
            if info.mod_major.is_some() {
                with_fingerprint += 1;
            }
            if info.bot_score > max_bot_score {
                max_bot_score = info.bot_score;
                max_bot_player = target.name.clone();
            }
        }

        world.queue_system_text(
            lookup.caller_id,
            "--- Anti-Cheat Global Statistics ---".to_string(),
        );
        world.queue_system_text(lookup.caller_id, format!("Total players: {total_players}"));
        world.queue_system_text(lookup.caller_id, format!("Verified: {verified}"));
        world.queue_system_text(lookup.caller_id, format!("Unverified: {unverified}"));
        world.queue_system_text(lookup.caller_id, format!("Suspicious: {suspicious}"));
        world.queue_system_text(lookup.caller_id, format!("Flagged: {flagged}"));
        world.queue_system_text(
            lookup.caller_id,
            format!("With fingerprint: {with_fingerprint}"),
        );
        if max_bot_score > 0.0 {
            world.queue_system_text(
                lookup.caller_id,
                format!("Highest bot score: {max_bot_score:.2} ({max_bot_player})"),
            );
        }
        applied += 1;
    }
    applied
}

/// `#aclist`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_list`
/// (`anticheat.c:721-753`): one line per online `CF_PLAYER` character
/// with a known anticheat session (padding/color dropped, matching
/// `/global`'s established plain-text simplification), in the same
/// ascending-character-id order the command handler gathered `targets`
/// in, followed by a trailing "Total: N players" count.
pub(crate) async fn apply_ac_list_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_list_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let session_ids: Vec<i64> = lookup
            .targets
            .iter()
            .map(|target| target.session_id)
            .collect();
        let Ok(sessions) = repository.find_sessions(&session_ids).await else {
            continue;
        };
        let sessions_by_id: HashMap<i64, ugaris_db::AntiCheatSessionInfo> =
            sessions.into_iter().collect();

        world.queue_system_text(
            lookup.caller_id,
            "--- Online Players AC Status ---".to_string(),
        );
        let mut count = 0;
        for target in &lookup.targets {
            let Some(info) = sessions_by_id.get(&target.session_id) else {
                continue;
            };
            world.queue_system_text(
                lookup.caller_id,
                format!(
                    "{:<16} {:<10} Bot:{:.2} HB:{} St:{} Ch:{}",
                    target.name,
                    ac_status_string(info.status),
                    info.bot_score,
                    info.heartbeat_violations,
                    info.state_violations,
                    info.challenge_failures
                ),
            );
            count += 1;
        }
        world.queue_system_text(lookup.caller_id, format!("Total: {count} players"));
        applied += 1;
    }
    applied
}

/// `#acsuspicious`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_suspicious`
/// (`anticheat.c:754-780`): one line per online `CF_PLAYER` character
/// with a known anticheat session whose status is
/// `>= AC_STATUS_SUSPICIOUS` (padding/color dropped, matching `/global`'s
/// established plain-text simplification), in the same ascending-
/// character-id order the command handler gathered `targets` in,
/// followed by a trailing "Total: N players" count - or, if none
/// qualify, C's own "No suspicious or flagged players online." (the
/// zero-count message is genuinely different text from `#aclist`'s,
/// copied letter for letter).
pub(crate) async fn apply_ac_suspicious_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_suspicious_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let session_ids: Vec<i64> = lookup
            .targets
            .iter()
            .map(|target| target.session_id)
            .collect();
        let Ok(sessions) = repository.find_sessions(&session_ids).await else {
            continue;
        };
        let sessions_by_id: HashMap<i64, ugaris_db::AntiCheatSessionInfo> =
            sessions.into_iter().collect();

        world.queue_system_text(
            lookup.caller_id,
            "--- Suspicious/Flagged Players ---".to_string(),
        );
        let mut count = 0;
        for target in &lookup.targets {
            let Some(info) = sessions_by_id.get(&target.session_id) else {
                continue;
            };
            if info.status < AC_STATUS_SUSPICIOUS {
                continue;
            }
            world.queue_system_text(
                lookup.caller_id,
                format!(
                    "{} - {} (Bot: {:.2}, HB: {}, State: {}, Chal: {})",
                    target.name,
                    ac_status_string(info.status),
                    info.bot_score,
                    info.heartbeat_violations,
                    info.state_violations,
                    info.challenge_failures
                ),
            );
            count += 1;
        }
        if count == 0 {
            world.queue_system_text(
                lookup.caller_id,
                "No suspicious or flagged players online.".to_string(),
            );
        } else {
            world.queue_system_text(lookup.caller_id, format!("Total: {count} players"));
        }
        applied += 1;
    }
    applied
}

/// `#accleanup <days>`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_cleanup`
/// (`anticheat.c:1267-1285`): deletes `anticheat_sessions` rows older
/// than `days` (`AntiCheatRepository::cleanup_old_records`, already
/// ported in iteration 196) and reports the row count back to the
/// caller. C also deletes from a separate `ac_heartbeat_log` table
/// (`db_ac_cleanup_heartbeat_logs`) this codebase has no equivalent of
/// (heartbeat counters live on the session row itself) - the reported
/// count for that half is always `0`, matching C's own always-present
/// "%d heartbeat logs deleted" clause rather than dropping it. A failed
/// delete (DB error) is silently skipped, matching every other offline-
/// DB-lookup event in this file - no error message reaches the caller,
/// same as a vanished session row elsewhere in this module.
pub(crate) async fn apply_ac_cleanup_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_cleanup_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(deleted) = repository.cleanup_old_records(lookup.days).await else {
            continue;
        };
        let heartbeat_logs_deleted = 0;
        world.queue_system_text(
            lookup.caller_id,
            format!(
                "Cleanup complete: {deleted} sessions, {heartbeat_logs_deleted} heartbeat logs deleted."
            ),
        );
        applied += 1;
    }
    applied
}

/// `#acreset <player>`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_reset`
/// (`anticheat.c:527-561`): zeroes the target session's violation
/// counters/bot score and restores `status` to `AC_STATUS_VERIFIED`
/// (`AntiCheatRepository::reset_session`). C's confirmation is
/// unconditional and same-thread (mutating an in-memory struct always
/// succeeds); here the "Reset anti-cheat data for {name}." message is
/// only queued once the async update actually reports a row was
/// touched, matching every other offline-DB-mutation event in this
/// file's silent-skip-on-failure convention (a vanished session row
/// between the command and the tick loop draining it, or a DB error,
/// produces no reply at all).
pub(crate) async fn apply_ac_reset_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_reset_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(true) = repository.reset_session(lookup.session_id).await else {
            continue;
        };
        world.queue_system_text(
            lookup.caller_id,
            format!("Reset anti-cheat data for {}.", lookup.target_name),
        );
        applied += 1;
    }
    applied
}

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

/// `#acsessions <player>`'s async DB round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_sessions`
/// (`anticheat.c:975-1017`): resolves the subscriber id the same way
/// `apply_ac_trust_events` does (`account_id_for_session`), then queries
/// up to 10 recent sessions (`AntiCheatRepository::recent_sessions`,
/// matching C's own `sessions[10]` stack array / `db_ac_get_recent_
/// sessions(..., 10)` call). An unresolvable subscriber id is silently
/// skipped (no reply at all), matching the module doc comment's
/// established "row deleted or unknown id -> silent skip" convention
/// (unlike `#acwarn`, this command has no C-side `subscriber_id <= 0`
/// branch to reproduce, since C's own `ac_find_player` guarantees an
/// online connection exists and thus always has *some* row).
pub(crate) async fn apply_ac_sessions_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_sessions_lookups();
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
        let Ok(rows) = repository.recent_sessions(account_id, 10).await else {
            continue;
        };
        for line in ac_sessions_lines(&lookup.target_name, &rows) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_ac_sessions_events`], split
/// out so it can be unit-tested without a live database - same
/// established convention as `ac_status_lines`. C `ac_cmd_sessions`
/// (`anticheat.c:993-1017`) - color wrapping dropped, matching `ac_
/// status_lines`'s/`/global`'s plain-text simplification for admin-only
/// displays.
pub(crate) fn ac_sessions_lines(
    target_name: &str,
    rows: &[ugaris_db::AntiCheatSessionHistoryRow],
) -> Vec<String> {
    if rows.is_empty() {
        return vec![format!("No sessions found for {target_name}.")];
    }
    let mut lines = vec![format!("--- Recent Sessions for {target_name} ---")];
    for row in rows {
        lines.push(format!(
            "{} ({}m) {} Bot:{:.2} V:{}/{}/{}/{}",
            row.start_time,
            row.duration_minutes,
            ac_status_string(row.status),
            row.bot_score,
            row.heartbeat_violations,
            row.state_violations,
            row.challenge_failures,
            row.anomaly_count,
        ));
    }
    lines
}

/// `#acviolations <player>`'s async round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment for the general async-DB-
/// round-trip pattern this family shares, and `AcViolationsLookup`'s own
/// doc comment for why this is the same single-name-target resolution
/// shape as `#acsessions`.
pub(crate) async fn apply_ac_violations_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_violations_lookups();
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
        let Ok(rows) = repository.recent_violations(account_id, 15).await else {
            continue;
        };
        for line in ac_violations_lines(&lookup.target_name, &rows) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_ac_violations_events`], split
/// out so it can be unit-tested without a live database - same
/// established convention as `ac_sessions_lines`. C `ac_cmd_violations`
/// (`anticheat.c:1043-1053`) - color wrapping (severity-based
/// red/orange/yellow) dropped, matching `ac_sessions_lines`'s/`ac_
/// status_lines`'s plain-text simplification for admin-only displays;
/// the numeric severity is kept in the line itself instead so the
/// information isn't lost entirely.
pub(crate) fn ac_violations_lines(
    target_name: &str,
    rows: &[ugaris_db::AntiCheatViolationRow],
) -> Vec<String> {
    if rows.is_empty() {
        return vec![format!("No violations found for {target_name}.")];
    }
    let mut lines = vec![format!("--- Recent Violations for {target_name} ---")];
    for row in rows {
        lines.push(format!(
            "{} [{}] sev={} {}",
            row.detected_at,
            row.type_name,
            row.severity,
            row.details.as_deref().unwrap_or(""),
        ));
    }
    lines
}

/// `#achistory <player>`'s async round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment for the general async-DB-
/// round-trip pattern this family shares, and `AcHistoryLookup`'s own
/// doc comment for why this is the same single-name-target resolution
/// shape as `#acsessions`/`#acviolations`. Unlike those two siblings,
/// this reads a single lifetime rollup row
/// (`AntiCheatRepository::find_player_stats`) rather than a list of
/// per-event rows.
pub(crate) async fn apply_ac_history_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_history_lookups();
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
        let Ok(stats) = repository.find_player_stats(account_id).await else {
            continue;
        };
        for line in ac_history_lines(&lookup.target_name, account_id, stats.as_ref()) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_ac_history_events`], split out
/// so it can be unit-tested without a live database - same established
/// convention as `ac_sessions_lines`/`ac_violations_lines`. C `ac_cmd_
/// history` (`anticheat.c:924-972`) - color wrapping (risk-level-based
/// red/orange/yellow/green) dropped, matching this file's established
/// plain-text simplification for admin-only displays. Reproduces C's
/// exact 7-line body (plus the header) digit for digit, including the
/// `%d flagged, %d suspicious` comma placement.
pub(crate) fn ac_history_lines(
    target_name: &str,
    subscriber_id: i64,
    stats: Option<&ugaris_db::AntiCheatPlayerStatsRow>,
) -> Vec<String> {
    let Some(stats) = stats else {
        return vec![format!("No AC history found for {target_name}.")];
    };
    vec![
        format!("--- AC History for {target_name} (ID: {subscriber_id}) ---"),
        format!(
            "Sessions: {} total, {} flagged, {} suspicious",
            stats.total_sessions, stats.flagged_sessions, stats.suspicious_sessions
        ),
        format!(
            "Violations: HB={}, State={}, Challenge={}, Anomalies={}",
            stats.total_heartbeat_violations,
            stats.total_state_violations,
            stats.total_challenge_failures,
            stats.total_anomalies
        ),
        format!(
            "Bot Score: max={:.2}, avg={:.2}",
            stats.max_session_bot_score, stats.avg_session_bot_score
        ),
        format!("Risk Level: {}", stats.risk_level),
        format!(
            "Flagged: {}, Trusted: {}, Warnings: {}",
            if stats.is_flagged { "YES" } else { "no" },
            if stats.is_trusted { "YES" } else { "no" },
            stats.warnings_issued
        ),
        format!("First seen: {}", stats.first_seen),
        format!("Last seen: {}", stats.last_seen.as_deref().unwrap_or("")),
    ]
}

/// `#acsharedip <player>`'s async round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment for the general async-DB-
/// round-trip pattern this family shares, and `AcSharedIpLookup`'s own
/// doc comment for why this is the same single-name-target resolution
/// shape as `#acsessions`/`#acviolations`/`#achistory`.
pub(crate) async fn apply_ac_sharedip_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_sharedip_lookups();
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
        let Ok(rows) = repository.shared_ips(account_id, 20).await else {
            continue;
        };
        for line in ac_sharedip_lines(&lookup.target_name, &rows) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_ac_sharedip_events`], split
/// out so it can be unit-tested without a live database - same
/// established convention as `ac_sessions_lines`/`ac_violations_lines`.
/// C `ac_cmd_sharedip` (`anticheat.c:1058-1088`) - color wrapping
/// dropped, matching this file's established plain-text simplification;
/// the trailing "Found %d accounts sharing IPs." summary line is
/// reproduced as-is. `email` is replaced by `username` throughout - see
/// `AntiCheatSharedIpRow`'s doc comment.
pub(crate) fn ac_sharedip_lines(
    target_name: &str,
    rows: &[ugaris_db::AntiCheatSharedIpRow],
) -> Vec<String> {
    if rows.is_empty() {
        return vec![format!("No shared IPs found for {target_name}.")];
    }
    let mut lines = vec![format!("--- Accounts Sharing IP with {target_name} ---")];
    for row in rows {
        lines.push(format!(
            "{} - {} (sessions: {}, last: {})",
            row.username,
            std::net::Ipv4Addr::from(row.ip_address as u32),
            row.session_count,
            row.last_seen
        ));
    }
    lines.push(format!("Found {} accounts sharing IPs.", rows.len()));
    lines
}

/// `#acsharedhw <player>`'s async round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment for the general async-DB-
/// round-trip pattern this family shares, and `AcSharedHwLookup`'s own
/// doc comment for why this is the same single-name-target resolution
/// shape as `#acsharedip` above.
pub(crate) async fn apply_ac_sharedhw_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_sharedhw_lookups();
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
        let Ok(rows) = repository.shared_hardware(account_id, 20).await else {
            continue;
        };
        for line in ac_sharedhw_lines(&lookup.target_name, &rows) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_ac_sharedhw_events`], split
/// out so it can be unit-tested without a live database - same
/// established convention as `ac_sharedip_lines` above. C `ac_cmd_
/// sharedhw` (`anticheat.c:1096-1126`) - color wrapping dropped; `email`
/// replaced by `username`, matching `ac_sharedip_lines`.
pub(crate) fn ac_sharedhw_lines(
    target_name: &str,
    rows: &[ugaris_db::AntiCheatSharedHwRow],
) -> Vec<String> {
    if rows.is_empty() {
        return vec![format!("No shared hardware found for {target_name}.")];
    }
    let mut lines = vec![format!(
        "--- Accounts Sharing Hardware with {target_name} ---"
    )];
    for row in rows {
        lines.push(format!(
            "{} - Hash: {}, Screen: {}x{} (last: {})",
            row.username,
            row.hardware_hash,
            row.screen_w.unwrap_or(0),
            row.screen_h.unwrap_or(0),
            row.last_seen
        ));
    }
    lines.push(format!("Found {} accounts sharing hardware.", rows.len()));
    lines
}

/// `#achighrisk`'s async round trip - see `ugaris-core`'s `world/
/// anticheat.rs` module doc comment for the general async-DB-round-trip
/// pattern this family shares. No name/session resolution at all (unlike
/// every other member of the family except `#acsiglist`/`#accleanup`),
/// so this simply lists every high-risk `ac_player_stats` row.
pub(crate) async fn apply_ac_highrisk_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_highrisk_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(rows) = repository.high_risk_players(20).await else {
            continue;
        };
        for line in ac_highrisk_lines(&rows) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_ac_highrisk_events`], split
/// out so it can be unit-tested without a live database - same
/// established convention as `ac_siglist_lines`. C `ac_cmd_highrisk`
/// (`anticheat.c:1134-1157`) - risk-level-based color wrapping dropped;
/// `email` replaced by `username`, matching `ac_sharedip_lines`.
pub(crate) fn ac_highrisk_lines(rows: &[ugaris_db::AntiCheatHighRiskRow]) -> Vec<String> {
    if rows.is_empty() {
        return vec!["No high-risk players found.".to_string()];
    }
    let mut lines = vec!["--- High-Risk Players ---".to_string()];
    for row in rows {
        lines.push(format!(
            "[{}] {} - {} Bot:{:.2} Flag:{} (seen: {})",
            row.subscriber_id,
            row.username,
            row.risk_level,
            row.max_bot_score,
            row.flagged_sessions,
            row.last_seen.as_deref().unwrap_or("")
        ));
    }
    lines
}

/// `#aclookup <subscriber_id>`'s async round trip - see `ugaris-core`'s
/// `world/anticheat.rs` module doc comment for the general async-DB-
/// round-trip pattern this family shares, and `AcLookupLookup`'s own doc
/// comment for why `subscriber_id` is parsed directly rather than
/// resolved from an online character name.
pub(crate) async fn apply_ac_lookup_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_lookup_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(result) = repository.lookup_subscriber(lookup.subscriber_id).await else {
            continue;
        };
        for line in ac_lookup_lines(lookup.subscriber_id, result.as_ref()) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_ac_lookup_events`], split out
/// so it can be unit-tested without a live database - same established
/// convention as `ac_history_lines`. C `ac_cmd_lookup` (`anticheat.c:
/// 1158-1191`); the `"Email: %s"` line has no equivalent in this
/// codebase's schema (see `AntiCheatSubscriberLookup`'s doc comment) so
/// it is folded into the header line as `"--- Subscriber {id} ({
/// username}) ---"` instead of being dropped outright, keeping the
/// account's identity visible in the reply.
pub(crate) fn ac_lookup_lines(
    subscriber_id: i64,
    result: Option<&ugaris_db::AntiCheatSubscriberLookup>,
) -> Vec<String> {
    let Some(result) = result else {
        return vec![format!("Subscriber ID {subscriber_id} not found.")];
    };
    let mut lines = vec![format!(
        "--- Subscriber {subscriber_id} ({}) ---",
        result.username
    )];
    let Some(stats) = &result.stats else {
        lines.push("No AC data for this subscriber.".to_string());
        return lines;
    };
    lines.push(format!(
        "Sessions: {} total, {} flagged",
        stats.total_sessions, stats.flagged_sessions
    ));
    lines.push(format!(
        "Max Bot Score: {:.2}, Risk: {}",
        stats.max_session_bot_score, stats.risk_level
    ));
    lines.push(format!(
        "Flagged: {}, Trusted: {}",
        if stats.is_flagged { "YES" } else { "no" },
        if stats.is_trusted { "YES" } else { "no" }
    ));
    lines.push(format!(
        "First: {}, Last: {}",
        stats.first_seen,
        stats.last_seen.as_deref().unwrap_or("")
    ));
    lines
}

/// `#acsiglist`'s async round trip - see `ugaris-core`'s `world/
/// anticheat.rs` module doc comment for the general async-DB-round-trip
/// pattern this family shares. No name/session resolution at all (unlike
/// every other member of the family except `#accleanup`), so this simply
/// lists every row in the new `ac_known_signatures` table.
pub(crate) async fn apply_ac_siglist_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_siglist_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(rows) = repository.list_signatures(20).await else {
            continue;
        };
        for line in ac_siglist_lines(&rows) {
            world.queue_system_text(lookup.caller_id, line);
        }
        applied += 1;
    }
    applied
}

/// Pure message-formatting half of [`apply_ac_siglist_events`], split out
/// so it can be unit-tested without a live database - same established
/// convention as `ac_sessions_lines`/`ac_violations_lines`. C `ac_cmd_
/// siglist` (`anticheat.c:1192-1215`) - color wrapping (severity-based
/// red/orange/yellow highlighting on the name/severity) dropped, matching
/// this file's established plain-text simplification for admin-only
/// displays; the literal double-space quirk before `Det:` when a
/// signature has neither `auto_flag` nor `auto_ban` set (C's own format
/// string has a bare `" "` literal immediately followed by the two
/// optional `"Flag "`/`"Ban "` tokens, then another literal `" Det:"`) is
/// reproduced as-is, not "cleaned up".
pub(crate) fn ac_siglist_lines(rows: &[ugaris_db::AntiCheatSignatureRow]) -> Vec<String> {
    if rows.is_empty() {
        return vec!["No signatures defined.".to_string()];
    }
    let mut lines = vec!["--- Known Bad Signatures ---".to_string()];
    for row in rows {
        let flag = if row.auto_flag { "Flag " } else { "" };
        let ban = if row.auto_ban { "Ban " } else { "" };
        lines.push(format!(
            "[{}] {} ({}) Sev:{} {}{} Det:{}",
            row.id, row.name, row.signature_type, row.severity, flag, ban, row.times_detected,
        ));
    }
    lines
}

/// `#acsigadd <type> <value> <name>`'s async round trip - see `ugaris-
/// core`'s `world/anticheat.rs` module doc comment. Reproduces `ac_cmd_
/// sigadd` (`anticheat.c:1216-1245`): inserts a new `ac_known_signatures`
/// row (`AntiCheatRepository::add_signature`). C's confirmation is
/// unconditional and same-thread; here the "Added signature: ..." message
/// is only queued once the async insert actually succeeds, matching every
/// other offline-DB-mutation event in this file's silent-skip-on-failure
/// convention (C's own "Failed to add signature." error path is likewise
/// only reachable when the query itself fails, so silently skipping the
/// reply on an `Err` here - rather than sending that exact text - loses
/// no user-facing branch C didn't already gate the same way).
pub(crate) async fn apply_ac_sigadd_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_sigadd_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        if repository
            .add_signature(
                &lookup.sig_type,
                &lookup.sig_value,
                &lookup.name,
                &lookup.created_by,
            )
            .await
            .is_err()
        {
            continue;
        }
        world.queue_system_text(
            lookup.caller_id,
            format!(
                "Added signature: {} ({}) = {}",
                lookup.name, lookup.sig_type, lookup.sig_value
            ),
        );
        applied += 1;
    }
    applied
}

/// `#acsigdel <id>`'s async round trip - see `ugaris-core`'s `world/
/// anticheat.rs` module doc comment. Reproduces `ac_cmd_sigdel`
/// (`anticheat.c:1246-1266`): deletes the named `ac_known_signatures` row
/// (`AntiCheatRepository::delete_signature`). Unlike most siblings in
/// this family, C's own "not found" branch (`affected == 0`) is itself
/// user-facing text, not a silent skip - reproduced here by checking the
/// mutator's `bool` result rather than only its `Result::Ok`-ness.
pub(crate) async fn apply_ac_sigdel_events(
    world: &mut World,
    anticheat_repository: &Option<ugaris_db::PgAntiCheatRepository>,
) -> usize {
    let lookups = world.drain_pending_ac_sigdel_lookups();
    if lookups.is_empty() {
        return 0;
    }
    let Some(repository) = anticheat_repository else {
        return 0;
    };
    let mut applied = 0;
    for lookup in lookups {
        let Ok(found) = repository.delete_signature(lookup.signature_id).await else {
            continue;
        };
        world.queue_system_text(
            lookup.caller_id,
            if found {
                format!("Deleted signature ID {}.", lookup.signature_id)
            } else {
                format!("Signature ID {} not found.", lookup.signature_id)
            },
        );
        applied += 1;
    }
    applied
}

#[cfg(test)]
mod ac_status_tests {
    use super::*;
    use ugaris_db::AntiCheatSessionInfo;

    fn info(status: i32, bot_score: f32) -> AntiCheatSessionInfo {
        AntiCheatSessionInfo {
            status,
            bot_score,
            heartbeat_violations: 1,
            state_violations: 2,
            challenge_failures: 3,
            timeout_count: 4,
            mod_major: None,
            mod_minor: None,
            mod_patch: None,
            os_type: None,
            screen_w: None,
            screen_h: None,
        }
    }

    #[test]
    fn without_fingerprint_shows_not_received() {
        let lines = ac_status_lines("Baddie", &info(2, 0.75));
        assert_eq!(
            lines,
            vec![
                "--- Anti-Cheat Status for Baddie ---".to_string(),
                "Status: suspicious".to_string(),
                "Heartbeat violations: 1".to_string(),
                "State violations: 2".to_string(),
                "Challenge failures: 3".to_string(),
                "Bot score: 0.75".to_string(),
                "Timeout count: 4".to_string(),
                "Fingerprint: not received".to_string(),
            ]
        );
    }

    #[test]
    fn with_fingerprint_shows_mod_version_os_and_screen() {
        let mut session_info = info(1, 0.0);
        session_info.mod_major = Some(1);
        session_info.mod_minor = Some(2);
        session_info.mod_patch = Some(3);
        session_info.os_type = Some(2);
        session_info.screen_w = Some(1920);
        session_info.screen_h = Some(1080);
        let lines = ac_status_lines("Godmode", &session_info);
        assert_eq!(lines[1], "Status: verified");
        assert_eq!(lines[7], "Mod version: 1.2.3");
        assert_eq!(lines[8], "OS: Linux");
        assert_eq!(lines[9], "Screen: 1920x1080");
    }

    #[test]
    fn unknown_os_type_falls_back_to_unknown() {
        let mut session_info = info(0, 0.0);
        session_info.mod_major = Some(0);
        session_info.mod_minor = Some(0);
        session_info.mod_patch = Some(0);
        session_info.os_type = Some(99);
        let lines = ac_status_lines("Player", &session_info);
        assert_eq!(lines[1], "Status: unverified");
        assert!(lines.contains(&"OS: Unknown".to_string()));
    }

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_status_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_status_lookup(CharacterId(7), "Baddie".to_string(), 42);

        let applied = apply_ac_status_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_status_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_stats_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_stats_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_stats_lookup(
            CharacterId(7),
            vec![AcOnlineTarget {
                name: "Baddie".to_string(),
                session_id: 42,
            }],
        );

        let applied = apply_ac_stats_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_stats_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_list_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_list_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_list_lookup(
            CharacterId(7),
            vec![AcOnlineTarget {
                name: "Baddie".to_string(),
                session_id: 42,
            }],
        );

        let applied = apply_ac_list_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_list_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_suspicious_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_suspicious_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_suspicious_lookup(
            CharacterId(7),
            vec![AcOnlineTarget {
                name: "Baddie".to_string(),
                session_id: 42,
            }],
        );

        let applied = apply_ac_suspicious_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_suspicious_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_cleanup_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_cleanup_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_cleanup_lookup(CharacterId(7), 30);

        let applied = apply_ac_cleanup_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_cleanup_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_reset_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_reset_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_reset_lookup(CharacterId(7), "Baddie".to_string(), 30);

        let applied = apply_ac_reset_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_reset_lookups().is_empty());
    }
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

#[cfg(test)]
mod ac_sessions_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_sessions_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_sessions_lookup(CharacterId(7), "Baddie".to_string(), 30);

        let applied = apply_ac_sessions_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_sessions_lookups().is_empty());
    }

    #[test]
    fn ac_sessions_lines_reports_no_sessions_when_empty() {
        let lines = ac_sessions_lines("Baddie", &[]);
        assert_eq!(lines, vec!["No sessions found for Baddie.".to_string()]);
    }

    #[test]
    fn ac_sessions_lines_formats_header_and_rows() {
        let rows = vec![
            ugaris_db::AntiCheatSessionHistoryRow {
                start_time: "07-06 10:00".to_string(),
                duration_minutes: 15,
                status: 3,
                bot_score: 0.91,
                heartbeat_violations: 2,
                state_violations: 3,
                challenge_failures: 4,
                anomaly_count: 5,
            },
            ugaris_db::AntiCheatSessionHistoryRow {
                start_time: "07-05 09:00".to_string(),
                duration_minutes: 60,
                status: 1,
                bot_score: 0.0,
                heartbeat_violations: 0,
                state_violations: 0,
                challenge_failures: 0,
                anomaly_count: 0,
            },
        ];
        let lines = ac_sessions_lines("Baddie", &rows);
        assert_eq!(
            lines,
            vec![
                "--- Recent Sessions for Baddie ---".to_string(),
                "07-06 10:00 (15m) flagged Bot:0.91 V:2/3/4/5".to_string(),
                "07-05 09:00 (60m) verified Bot:0.00 V:0/0/0/0".to_string(),
            ]
        );
    }
}

#[cfg(test)]
mod ac_violations_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_violations_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_violations_lookup(CharacterId(7), "Baddie".to_string(), 30);

        let applied = apply_ac_violations_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_violations_lookups().is_empty());
    }

    #[test]
    fn ac_violations_lines_reports_no_violations_when_empty() {
        let lines = ac_violations_lines("Baddie", &[]);
        assert_eq!(lines, vec!["No violations found for Baddie.".to_string()]);
    }

    #[test]
    fn ac_violations_lines_formats_header_and_rows() {
        let rows = vec![
            ugaris_db::AntiCheatViolationRow {
                detected_at: "07-06 10:00".to_string(),
                type_name: "teleport".to_string(),
                severity: 2,
                details: Some("impossible jump".to_string()),
            },
            ugaris_db::AntiCheatViolationRow {
                detected_at: "07-05 09:00".to_string(),
                type_name: "speedhack".to_string(),
                severity: 1,
                details: None,
            },
        ];
        let lines = ac_violations_lines("Baddie", &rows);
        assert_eq!(
            lines,
            vec![
                "--- Recent Violations for Baddie ---".to_string(),
                "07-06 10:00 [teleport] sev=2 impossible jump".to_string(),
                "07-05 09:00 [speedhack] sev=1 ".to_string(),
            ]
        );
    }
}

#[cfg(test)]
mod ac_history_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_history_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_history_lookup(CharacterId(7), "Baddie".to_string(), 30);

        let applied = apply_ac_history_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_history_lookups().is_empty());
    }

    #[test]
    fn ac_history_lines_reports_no_history_when_row_missing() {
        let lines = ac_history_lines("Baddie", 42, None);
        assert_eq!(lines, vec!["No AC history found for Baddie.".to_string()]);
    }

    #[test]
    fn ac_history_lines_formats_every_field() {
        let stats = ugaris_db::AntiCheatPlayerStatsRow {
            total_sessions: 12,
            flagged_sessions: 2,
            suspicious_sessions: 3,
            total_heartbeat_violations: 4,
            total_state_violations: 5,
            total_challenge_failures: 6,
            total_anomalies: 7,
            max_session_bot_score: 0.91,
            avg_session_bot_score: 0.4,
            risk_level: "high".to_string(),
            is_flagged: true,
            is_trusted: false,
            warnings_issued: 3,
            first_seen: "01-01 00:00".to_string(),
            last_seen: Some("07-06 10:00".to_string()),
        };
        let lines = ac_history_lines("Baddie", 42, Some(&stats));
        assert_eq!(
            lines,
            vec![
                "--- AC History for Baddie (ID: 42) ---".to_string(),
                "Sessions: 12 total, 2 flagged, 3 suspicious".to_string(),
                "Violations: HB=4, State=5, Challenge=6, Anomalies=7".to_string(),
                "Bot Score: max=0.91, avg=0.40".to_string(),
                "Risk Level: high".to_string(),
                "Flagged: YES, Trusted: no, Warnings: 3".to_string(),
                "First seen: 01-01 00:00".to_string(),
                "Last seen: 07-06 10:00".to_string(),
            ]
        );
    }

    #[test]
    fn ac_history_lines_handles_a_missing_last_seen() {
        let stats = ugaris_db::AntiCheatPlayerStatsRow {
            total_sessions: 1,
            flagged_sessions: 0,
            suspicious_sessions: 0,
            total_heartbeat_violations: 0,
            total_state_violations: 0,
            total_challenge_failures: 0,
            total_anomalies: 0,
            max_session_bot_score: 0.0,
            avg_session_bot_score: 0.0,
            risk_level: "low".to_string(),
            is_flagged: false,
            is_trusted: false,
            warnings_issued: 0,
            first_seen: "01-01 00:00".to_string(),
            last_seen: None,
        };
        let lines = ac_history_lines("Newbie", 7, Some(&stats));
        assert_eq!(lines.last().unwrap(), "Last seen: ");
    }
}

#[cfg(test)]
mod ac_sharedip_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_sharedip_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_sharedip_lookup(CharacterId(7), "Baddie".to_string(), 30);

        let applied = apply_ac_sharedip_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_sharedip_lookups().is_empty());
    }

    #[test]
    fn ac_sharedip_lines_reports_no_shared_ips_when_empty() {
        let lines = ac_sharedip_lines("Baddie", &[]);
        assert_eq!(lines, vec!["No shared IPs found for Baddie.".to_string()]);
    }

    #[test]
    fn ac_sharedip_lines_formats_header_rows_and_summary() {
        let rows = vec![
            ugaris_db::AntiCheatSharedIpRow {
                username: "altaccount".to_string(),
                ip_address: 0x7f00_0001u32 as i32, // 127.0.0.1
                session_count: 3,
                last_seen: "2026-07-06".to_string(),
            },
            ugaris_db::AntiCheatSharedIpRow {
                username: "another".to_string(),
                ip_address: 0xc0a8_0102u32 as i32, // 192.168.1.2
                session_count: 1,
                last_seen: "2026-07-01".to_string(),
            },
        ];
        let lines = ac_sharedip_lines("Baddie", &rows);
        assert_eq!(
            lines,
            vec![
                "--- Accounts Sharing IP with Baddie ---".to_string(),
                "altaccount - 127.0.0.1 (sessions: 3, last: 2026-07-06)".to_string(),
                "another - 192.168.1.2 (sessions: 1, last: 2026-07-01)".to_string(),
                "Found 2 accounts sharing IPs.".to_string(),
            ]
        );
    }
}

#[cfg(test)]
mod ac_sharedhw_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_sharedhw_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_sharedhw_lookup(CharacterId(7), "Baddie".to_string(), 30);

        let applied = apply_ac_sharedhw_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_sharedhw_lookups().is_empty());
    }

    #[test]
    fn ac_sharedhw_lines_reports_no_shared_hardware_when_empty() {
        let lines = ac_sharedhw_lines("Baddie", &[]);
        assert_eq!(
            lines,
            vec!["No shared hardware found for Baddie.".to_string()]
        );
    }

    #[test]
    fn ac_sharedhw_lines_formats_header_rows_and_summary() {
        let rows = vec![ugaris_db::AntiCheatSharedHwRow {
            username: "altaccount".to_string(),
            hardware_hash: 123456789,
            screen_w: Some(1920),
            screen_h: Some(1080),
            last_seen: "2026-07-06".to_string(),
        }];
        let lines = ac_sharedhw_lines("Baddie", &rows);
        assert_eq!(
            lines,
            vec![
                "--- Accounts Sharing Hardware with Baddie ---".to_string(),
                "altaccount - Hash: 123456789, Screen: 1920x1080 (last: 2026-07-06)".to_string(),
                "Found 1 accounts sharing hardware.".to_string(),
            ]
        );
    }

    #[test]
    fn ac_sharedhw_lines_defaults_missing_screen_dimensions_to_zero() {
        let rows = vec![ugaris_db::AntiCheatSharedHwRow {
            username: "altaccount".to_string(),
            hardware_hash: 42,
            screen_w: None,
            screen_h: None,
            last_seen: "2026-07-06".to_string(),
        }];
        let lines = ac_sharedhw_lines("Baddie", &rows);
        assert_eq!(
            lines[1],
            "altaccount - Hash: 42, Screen: 0x0 (last: 2026-07-06)"
        );
    }
}

#[cfg(test)]
mod ac_highrisk_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_highrisk_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_highrisk_lookup(CharacterId(7));

        let applied = apply_ac_highrisk_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_highrisk_lookups().is_empty());
    }

    #[test]
    fn ac_highrisk_lines_reports_no_players_when_empty() {
        let lines = ac_highrisk_lines(&[]);
        assert_eq!(lines, vec!["No high-risk players found.".to_string()]);
    }

    #[test]
    fn ac_highrisk_lines_formats_header_and_rows() {
        let rows = vec![
            ugaris_db::AntiCheatHighRiskRow {
                subscriber_id: 3,
                username: "cheater".to_string(),
                risk_level: "critical".to_string(),
                max_bot_score: 1.0,
                flagged_sessions: 4,
                last_seen: Some("07-06 10:00".to_string()),
            },
            ugaris_db::AntiCheatHighRiskRow {
                subscriber_id: 5,
                username: "suspect".to_string(),
                risk_level: "high".to_string(),
                max_bot_score: 0.85,
                flagged_sessions: 1,
                last_seen: None,
            },
        ];
        let lines = ac_highrisk_lines(&rows);
        assert_eq!(
            lines,
            vec![
                "--- High-Risk Players ---".to_string(),
                "[3] cheater - critical Bot:1.00 Flag:4 (seen: 07-06 10:00)".to_string(),
                "[5] suspect - high Bot:0.85 Flag:1 (seen: )".to_string(),
            ]
        );
    }
}

#[cfg(test)]
mod ac_lookup_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_lookup_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_lookup_lookup(CharacterId(7), 99);

        let applied = apply_ac_lookup_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_lookup_lookups().is_empty());
    }

    #[test]
    fn ac_lookup_lines_reports_not_found_when_subscriber_missing() {
        let lines = ac_lookup_lines(99, None);
        assert_eq!(lines, vec!["Subscriber ID 99 not found.".to_string()]);
    }

    #[test]
    fn ac_lookup_lines_reports_no_ac_data_when_stats_missing() {
        let result = ugaris_db::AntiCheatSubscriberLookup {
            username: "newbie".to_string(),
            stats: None,
        };
        let lines = ac_lookup_lines(7, Some(&result));
        assert_eq!(
            lines,
            vec![
                "--- Subscriber 7 (newbie) ---".to_string(),
                "No AC data for this subscriber.".to_string(),
            ]
        );
    }

    #[test]
    fn ac_lookup_lines_formats_every_field_when_stats_present() {
        let stats = ugaris_db::AntiCheatPlayerStatsRow {
            total_sessions: 12,
            flagged_sessions: 2,
            suspicious_sessions: 3,
            total_heartbeat_violations: 4,
            total_state_violations: 5,
            total_challenge_failures: 6,
            total_anomalies: 7,
            max_session_bot_score: 0.91,
            avg_session_bot_score: 0.4,
            risk_level: "high".to_string(),
            is_flagged: true,
            is_trusted: false,
            warnings_issued: 3,
            first_seen: "01-01 00:00".to_string(),
            last_seen: Some("07-06 10:00".to_string()),
        };
        let result = ugaris_db::AntiCheatSubscriberLookup {
            username: "cheater".to_string(),
            stats: Some(stats),
        };
        let lines = ac_lookup_lines(3, Some(&result));
        assert_eq!(
            lines,
            vec![
                "--- Subscriber 3 (cheater) ---".to_string(),
                "Sessions: 12 total, 2 flagged".to_string(),
                "Max Bot Score: 0.91, Risk: high".to_string(),
                "Flagged: YES, Trusted: no".to_string(),
                "First: 01-01 00:00, Last: 07-06 10:00".to_string(),
            ]
        );
    }
}

#[cfg(test)]
mod ac_siglist_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_siglist_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_siglist_lookup(CharacterId(7));

        let applied = apply_ac_siglist_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_siglist_lookups().is_empty());
    }

    #[test]
    fn ac_siglist_lines_reports_no_signatures_when_empty() {
        let lines = ac_siglist_lines(&[]);
        assert_eq!(lines, vec!["No signatures defined.".to_string()]);
    }

    #[test]
    fn ac_siglist_lines_formats_header_and_rows_including_the_double_space_quirk() {
        let rows = vec![
            ugaris_db::AntiCheatSignatureRow {
                id: 3,
                signature_type: "hardware_hash".to_string(),
                name: "Known Cheat Tool".to_string(),
                severity: 2,
                auto_flag: true,
                auto_ban: true,
                times_detected: 12,
                is_active: true,
            },
            ugaris_db::AntiCheatSignatureRow {
                id: 5,
                signature_type: "process_name".to_string(),
                name: "cheatengine.exe".to_string(),
                severity: 0,
                auto_flag: false,
                auto_ban: false,
                times_detected: 0,
                is_active: true,
            },
        ];
        let lines = ac_siglist_lines(&rows);
        assert_eq!(
            lines,
            vec![
                "--- Known Bad Signatures ---".to_string(),
                "[3] Known Cheat Tool (hardware_hash) Sev:2 Flag Ban  Det:12".to_string(),
                "[5] cheatengine.exe (process_name) Sev:0  Det:0".to_string(),
            ]
        );
    }
}

#[cfg(test)]
mod ac_sigadd_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_sigadd_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_sigadd_lookup(
            CharacterId(7),
            "hardware_hash".to_string(),
            "deadbeef".to_string(),
            "Known Cheat Tool".to_string(),
            "TestGod".to_string(),
        );

        let applied = apply_ac_sigadd_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_sigadd_lookups().is_empty());
    }
}

#[cfg(test)]
mod ac_sigdel_tests {
    use super::*;

    #[tokio::test]
    async fn no_lookups_queued_is_a_cheap_no_op() {
        let mut world = World::default();
        let applied = apply_ac_sigdel_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
    }

    #[tokio::test]
    async fn missing_repository_leaves_the_lookup_queued_state_untouched_but_drained() {
        let mut world = World::default();
        world.queue_ac_sigdel_lookup(CharacterId(7), 42);

        let applied = apply_ac_sigdel_events(&mut world, &None).await;
        assert_eq!(applied, 0);
        assert!(world.drain_pending_system_texts().is_empty());
        assert!(world.drain_pending_ac_sigdel_lookups().is_empty());
    }
}
