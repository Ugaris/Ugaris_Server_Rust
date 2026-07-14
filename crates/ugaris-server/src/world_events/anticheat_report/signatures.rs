use super::*;

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
