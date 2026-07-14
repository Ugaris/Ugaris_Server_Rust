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
