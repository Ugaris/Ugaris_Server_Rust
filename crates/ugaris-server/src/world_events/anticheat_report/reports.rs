use super::*;

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
