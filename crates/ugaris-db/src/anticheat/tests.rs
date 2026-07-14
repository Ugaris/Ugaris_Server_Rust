use super::*;

#[test]
fn legacy_result_names_match_c_defaults() {
    assert_eq!(legacy_result_name(0), "pass");
    assert_eq!(legacy_result_name(1), "fail");
    assert_eq!(legacy_result_name(2), "timeout");
    assert_eq!(legacy_result_name(99), "pass");
}

#[test]
fn legacy_signature_action_names_match_c_defaults() {
    assert_eq!(legacy_signature_action_name(0), "none");
    assert_eq!(legacy_signature_action_name(1), "flagged");
    assert_eq!(legacy_signature_action_name(2), "warned");
    assert_eq!(legacy_signature_action_name(3), "banned");
    assert_eq!(legacy_signature_action_name(99), "none");
}

#[test]
fn legacy_risk_names_match_c_defaults() {
    assert_eq!(legacy_risk_name(0), "low");
    assert_eq!(legacy_risk_name(1), "medium");
    assert_eq!(legacy_risk_name(2), "high");
    assert_eq!(legacy_risk_name(3), "critical");
    assert_eq!(legacy_risk_name(99), "low");
}

/// Live-DB round trip tests for the session/event lifecycle wired up
/// in `ugaris-server` (session creation on login, `end_session` on
/// disconnect - see `main.rs`'s `SessionEvent::Login`/`Disconnected`
/// handlers). Skips (rather than fails) when `DATABASE_URL` is unset
/// or unreachable, matching `character.rs`'s `live_login` convention.
/// No foreign keys are exercised here (all optional columns left
/// `NULL`) since `anticheat_sessions` only needs to stand on its own
/// to prove the repository methods round-trip correctly; each test
/// deletes its own row(s) afterward so repeated runs don't accumulate
/// data, matching `merchant.rs`'s `live` convention (this repository
/// has no locked-transaction-rollback fixture like `character.rs`'s
/// `live_login`, since every method commits directly via `&self.pool`
/// with no transaction parameter to hook a rollback onto).
mod live {
    use super::*;
    use sqlx::PgPool;

    async fn connect() -> Option<PgPool> {
        let url = std::env::var("DATABASE_URL").ok()?;
        match PgPool::connect(&url).await {
            Ok(pool) => Some(pool),
            Err(err) => {
                eprintln!("skipping live DB test: could not connect to DATABASE_URL: {err}");
                None
            }
        }
    }

    async fn cleanup_session(pool: &PgPool, session_id: i64) {
        sqlx::query("delete from anticheat_sessions where id = $1")
            .bind(session_id)
            .execute(pool)
            .await
            .expect("cleanup live anticheat_sessions row");
    }

    /// `anticheat_sessions.character_id` is a real foreign key into
    /// `characters`, so `set_character` needs a genuine row to point
    /// at rather than an arbitrary id. Returns `(account_id,
    /// character_id)`; callers must delete both (character first) once
    /// done.
    async fn insert_fixture_account_and_character(pool: &PgPool) -> (i64, i64) {
        let (account_id,): (i64,) = sqlx::query_as(
            "insert into accounts(username, password_hash) values ($1, 'secret') returning id",
        )
        .bind("ac_live_fixture_acct")
        .fetch_one(pool)
        .await
        .expect("insert fixture account");
        let (character_id,): (i64,) =
            sqlx::query_as("insert into characters(account_id, name) values ($1, $2) returning id")
                .bind(account_id)
                .bind("AcLiveFixtureChar")
                .fetch_one(pool)
                .await
                .expect("insert fixture character");
        (account_id, character_id)
    }

    #[tokio::test]
    async fn create_update_and_end_session_round_trip() {
        let Some(pool) = connect().await else {
            return;
        };
        let repo = PgAntiCheatRepository::new(pool.clone());

        // Fixtures use unique names, so stray rows from a previously
        // aborted run (rather than this test's own cleanup at the end)
        // would collide; clear them defensively first.
        sqlx::query("delete from characters where name = 'AcLiveFixtureChar'")
            .execute(&pool)
            .await
            .expect("pre-clean fixture character");
        sqlx::query("delete from accounts where username = 'ac_live_fixture_acct'")
            .execute(&pool)
            .await
            .expect("pre-clean fixture account");
        let (account_id, character_id) = insert_fixture_account_and_character(&pool).await;

        let session_id = repo
            .create_session(AntiCheatSessionCreate {
                login_session_id: None,
                account_id: Some(account_id),
                character_id: None,
                ip_address: 0x0a00_0001,
                area_id: 3,
            })
            .await
            .expect("create_session");
        assert!(session_id > 0, "expected a positive session id");

        let (status, bot_score, ended): (i32, f32, bool) = sqlx::query_as(
            "select status, bot_score, ended_at is not null from anticheat_sessions where id = $1",
        )
        .bind(session_id)
        .fetch_one(&pool)
        .await
        .expect("fetch fresh session row");
        assert_eq!(status, 0, "AC_STATUS_UNVERIFIED is the SQL default");
        assert_eq!(bot_score, 0.0);
        assert!(!ended, "a freshly created session must not be ended yet");

        let character_id = CharacterId(character_id as u32);
        assert!(repo
            .set_character(session_id, character_id)
            .await
            .expect("set_character"));

        assert!(repo
            .set_fingerprint(
                session_id,
                AntiCheatFingerprint {
                    mod_major: 1,
                    mod_minor: 2,
                    mod_patch: 3,
                    os_type: 4,
                    screen_w: 1920,
                    screen_h: 1080,
                    hardware_hash: 0xdead_beef,
                    code_hash: 0xcafe_babe,
                },
            )
            .await
            .expect("set_fingerprint"));

        assert!(repo.set_status(session_id, 2).await.expect("set_status"));

        assert!(repo
            .update_bot_score(session_id, 0.5, true)
            .await
            .expect("update_bot_score"));

        assert!(repo
            .increment_counters(
                session_id,
                AntiCheatCounters {
                    heartbeat_delta: 1,
                    state_delta: 2,
                    challenge_delta: 3,
                    anomaly_delta: 4,
                    timeout_delta: 5,
                },
            )
            .await
            .expect("increment_counters"));

        let (
            character_id_col,
            mod_major,
            screen_w,
            hardware_hash,
            status,
            bot_score,
            max_bot_score,
            hb_violations,
            state_violations,
            challenge_failures,
            anomaly_count,
            timeout_count,
        ): (i64, i32, i32, i64, i32, f32, f32, i32, i32, i32, i32, i32) = sqlx::query_as(
            "select character_id, mod_major, screen_w, hardware_hash, status, bot_score, \
                 max_bot_score, heartbeat_violations, state_violations, challenge_failures, \
                 anomaly_count, timeout_count from anticheat_sessions where id = $1",
        )
        .bind(session_id)
        .fetch_one(&pool)
        .await
        .expect("fetch updated session row");
        assert_eq!(character_id_col, i64::from(character_id.0));
        assert_eq!(mod_major, 1);
        assert_eq!(screen_w, 1920);
        assert_eq!(hardware_hash, 0xdead_beef_i64);
        assert_eq!(status, 2);
        assert_eq!(bot_score, 0.5);
        assert_eq!(max_bot_score, 0.5);
        assert_eq!(hb_violations, 1);
        assert_eq!(state_violations, 2);
        assert_eq!(challenge_failures, 3);
        assert_eq!(anomaly_count, 4);
        assert_eq!(timeout_count, 5);

        let info = repo
            .find_session(session_id)
            .await
            .expect("find_session")
            .expect("session must exist");
        assert_eq!(info.status, 2);
        assert_eq!(info.bot_score, 0.5);
        assert_eq!(info.heartbeat_violations, 1);
        assert_eq!(info.state_violations, 2);
        assert_eq!(info.challenge_failures, 3);
        assert_eq!(info.timeout_count, 5);
        assert_eq!(info.mod_major, Some(1));
        assert_eq!(info.mod_minor, Some(2));
        assert_eq!(info.mod_patch, Some(3));
        assert_eq!(info.os_type, Some(4));
        assert_eq!(info.screen_w, Some(1920));
        assert_eq!(info.screen_h, Some(1080));

        let batch = repo
            .find_sessions(&[session_id, i64::MAX - 1])
            .await
            .expect("find_sessions");
        assert_eq!(batch.len(), 1, "the nonexistent id must be omitted");
        assert_eq!(batch[0].0, session_id);
        assert_eq!(batch[0].1.bot_score, 0.5);

        let mut data = BTreeMap::new();
        data.insert("delta_x".to_string(), "500".to_string());
        let event_id = repo
            .log_event(AntiCheatEvent {
                session_id,
                event_type: "speedhack".to_string(),
                severity: 2,
                details: Some("teleported too far".to_string()),
                data,
            })
            .await
            .expect("log_event");
        assert!(event_id > 0);

        let (event_session_id, event_type, severity): (i64, String, i32) = sqlx::query_as(
            "select session_id, event_type, severity from anticheat_events where id = $1",
        )
        .bind(event_id)
        .fetch_one(&pool)
        .await
        .expect("fetch logged event");
        assert_eq!(event_session_id, session_id);
        assert_eq!(event_type, "speedhack");
        assert_eq!(severity, 2);

        assert!(repo
            .end_session(session_id, 0.75)
            .await
            .expect("end_session"));

        let (final_bot_score, ended): (f32, bool) = sqlx::query_as(
            "select bot_score, ended_at is not null from anticheat_sessions where id = $1",
        )
        .bind(session_id)
        .fetch_one(&pool)
        .await
        .expect("fetch ended session row");
        assert_eq!(final_bot_score, 0.75);
        assert!(ended, "end_session must set ended_at");

        sqlx::query("delete from anticheat_events where session_id = $1")
            .bind(session_id)
            .execute(&pool)
            .await
            .expect("cleanup live anticheat_events rows");
        cleanup_session(&pool, session_id).await;
        sqlx::query("delete from characters where id = $1")
            .bind(character_id.0 as i64)
            .execute(&pool)
            .await
            .expect("cleanup fixture character");
        sqlx::query("delete from accounts where id = $1")
            .bind(account_id)
            .execute(&pool)
            .await
            .expect("cleanup fixture account");
    }

    #[tokio::test]
    async fn updates_on_an_unknown_session_id_report_not_found() {
        let Some(pool) = connect().await else {
            return;
        };
        let repo = PgAntiCheatRepository::new(pool);

        // A session id that was never created (or already deleted)
        // must report `Ok(false)` ("no row matched"), not an error and
        // not a silent `Ok(true)`.
        assert!(!repo
            .set_status(i64::MAX - 1, 1)
            .await
            .expect("set_status on unknown id"));
        assert!(!repo
            .end_session(i64::MAX - 1, 0.0)
            .await
            .expect("end_session on unknown id"));
        assert!(repo
            .find_session(i64::MAX - 1)
            .await
            .expect("find_session on unknown id")
            .is_none());
        assert!(repo
            .find_sessions(&[i64::MAX - 1])
            .await
            .expect("find_sessions on unknown ids")
            .is_empty());
        assert!(repo
            .account_id_for_session(i64::MAX - 1)
            .await
            .expect("account_id_for_session on unknown id")
            .is_none());
    }

    /// `#acunflag`/`#actrust`/`#acuntrust`'s backing methods
    /// (`account_id_for_session`, `set_flagged`, `set_trusted`):
    /// a session created with a real `account_id` resolves back to
    /// it, and the `ac_player_stats` upsert (`db_ac_flag_player`/
    /// `db_ac_trust_player`'s ensure-then-update shape) round-trips
    /// both the initial insert and a later flip, exactly like
    /// `#acunflag` flipping a player from flagged to verified and
    /// back would in practice.
    #[tokio::test]
    async fn subscriber_flag_and_trust_round_trip() {
        let Some(pool) = connect().await else {
            return;
        };
        let repo = PgAntiCheatRepository::new(pool.clone());

        sqlx::query("delete from characters where name = 'AcSubscriberFixtureChar'")
            .execute(&pool)
            .await
            .expect("pre-clean fixture character");
        sqlx::query("delete from accounts where username = 'ac_subscriber_fixture_acct'")
            .execute(&pool)
            .await
            .expect("pre-clean fixture account");
        let (account_id,): (i64,) = sqlx::query_as(
            "insert into accounts(username, password_hash) values ($1, 'secret') returning id",
        )
        .bind("ac_subscriber_fixture_acct")
        .fetch_one(&pool)
        .await
        .expect("insert fixture account");

        let session_id = repo
            .create_session(AntiCheatSessionCreate {
                login_session_id: None,
                account_id: Some(account_id),
                character_id: None,
                ip_address: 0x0a00_0002,
                area_id: 1,
            })
            .await
            .expect("create_session");

        assert_eq!(
            repo.account_id_for_session(session_id)
                .await
                .expect("account_id_for_session"),
            Some(account_id)
        );

        // First touch: no `ac_player_stats` row exists yet - both
        // `set_flagged`/`set_trusted` must create it (C's
        // `db_ac_ensure_player_stats` pre-step), not error.
        repo.set_flagged(account_id, true)
            .await
            .expect("set_flagged insert");
        repo.set_trusted(account_id, true)
            .await
            .expect("set_trusted insert");

        let (is_flagged, is_trusted): (bool, bool) = sqlx::query_as(
            "select is_flagged, is_trusted from ac_player_stats where subscriber_id = $1",
        )
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .expect("fetch ac_player_stats row");
        assert!(is_flagged);
        assert!(is_trusted);

        // Second touch: the row already exists - both calls must
        // update in place (`on conflict ... do update`), not error on
        // a duplicate key.
        repo.set_flagged(account_id, false)
            .await
            .expect("set_flagged update");
        repo.set_trusted(account_id, false)
            .await
            .expect("set_trusted update");

        let (is_flagged, is_trusted): (bool, bool) = sqlx::query_as(
            "select is_flagged, is_trusted from ac_player_stats where subscriber_id = $1",
        )
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .expect("re-fetch ac_player_stats row");
        assert!(!is_flagged);
        assert!(!is_trusted);

        // `#acwarn`'s backing method (`issue_warning`): the same
        // `ac_player_stats` row is reused (already created above by
        // `set_flagged`), so this call must only increment
        // `warnings_issued` and stamp `last_warning_at`, not disturb
        // `is_flagged`/`is_trusted`; a second call must increment
        // again rather than overwrite, matching C's own `warnings_
        // issued = warnings_issued + 1`.
        repo.issue_warning(account_id)
            .await
            .expect("issue_warning first call");
        repo.issue_warning(account_id)
            .await
            .expect("issue_warning second call");

        let (warnings_issued, last_warning_at_is_set, is_flagged, is_trusted): (
            i32,
            bool,
            bool,
            bool,
        ) = sqlx::query_as(
            "select warnings_issued, last_warning_at is not null, is_flagged, is_trusted \
                 from ac_player_stats where subscriber_id = $1",
        )
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .expect("fetch ac_player_stats row after issue_warning");
        assert_eq!(warnings_issued, 2);
        assert!(last_warning_at_is_set);
        assert!(!is_flagged);
        assert!(!is_trusted);

        sqlx::query("delete from ac_player_stats where subscriber_id = $1")
            .bind(account_id)
            .execute(&pool)
            .await
            .expect("cleanup ac_player_stats row");
        cleanup_session(&pool, session_id).await;
        sqlx::query("delete from accounts where id = $1")
            .bind(account_id)
            .execute(&pool)
            .await
            .expect("cleanup fixture account");
    }

    /// `#acsessions`'s backing query (`recent_sessions`): two sessions
    /// for the same subscriber - an ended one (fixed
    /// `started_at`/`ended_at`, so `duration_minutes` is
    /// deterministic) and a still-open one (`ended_at` left `NULL`,
    /// so C's `COALESCE(session_end, NOW())` clause is exercised) -
    /// come back newest-first with every counter column intact, and a
    /// `limit` of `1` returns only the newest row.
    #[tokio::test]
    async fn recent_sessions_orders_newest_first_and_computes_duration() {
        let Some(pool) = connect().await else {
            return;
        };
        let repo = PgAntiCheatRepository::new(pool.clone());

        sqlx::query("delete from accounts where username = 'ac_sessions_fixture_acct'")
            .execute(&pool)
            .await
            .expect("pre-clean fixture account");
        let (account_id,): (i64,) = sqlx::query_as(
            "insert into accounts(username, password_hash) values ($1, 'secret') returning id",
        )
        .bind("ac_sessions_fixture_acct")
        .fetch_one(&pool)
        .await
        .expect("insert fixture account");

        let (older_id,): (i64,) = sqlx::query_as(
            "insert into anticheat_sessions(account_id, ip_address, area_id, status, \
                 bot_score, heartbeat_violations, state_violations, challenge_failures, \
                 anomaly_count, started_at, ended_at) \
                 values ($1, 1, 1, 1, 0.1, 1, 0, 0, 0, now() - interval '2 hours', \
                 now() - interval '1 hour') returning id",
        )
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .expect("insert older session");
        let (newer_id,): (i64,) = sqlx::query_as(
            "insert into anticheat_sessions(account_id, ip_address, area_id, status, \
                 bot_score, heartbeat_violations, state_violations, challenge_failures, \
                 anomaly_count, started_at, ended_at) \
                 values ($1, 1, 1, 3, 0.9, 2, 3, 4, 5, now() - interval '10 minutes', null) \
                 returning id",
        )
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .expect("insert newer (still open) session");

        let rows = repo
            .recent_sessions(account_id, 10)
            .await
            .expect("recent_sessions");
        assert_eq!(rows.len(), 2, "both sessions must come back");
        assert_eq!(rows[0].status, 3, "newest session must sort first");
        assert_eq!(rows[0].bot_score, 0.9);
        assert_eq!(rows[0].heartbeat_violations, 2);
        assert_eq!(rows[0].state_violations, 3);
        assert_eq!(rows[0].challenge_failures, 4);
        assert_eq!(rows[0].anomaly_count, 5);
        assert!(
            rows[0].duration_minutes >= 9 && rows[0].duration_minutes <= 11,
            "an open session's duration must be measured against now(), got {}",
            rows[0].duration_minutes
        );
        assert_eq!(rows[1].status, 1, "older session must sort second");
        assert_eq!(
            rows[1].duration_minutes, 60,
            "an ended session's duration must use its own ended_at, not now()"
        );

        let limited = repo
            .recent_sessions(account_id, 1)
            .await
            .expect("recent_sessions with limit 1");
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].status, 3, "limit must keep only the newest row");

        sqlx::query("delete from anticheat_sessions where id = any($1)")
            .bind([older_id, newer_id])
            .execute(&pool)
            .await
            .expect("cleanup fixture sessions");
        sqlx::query("delete from accounts where id = $1")
            .bind(account_id)
            .execute(&pool)
            .await
            .expect("cleanup fixture account");
    }

    /// `#acviolations`'s backing query (`recent_violations`): two
    /// `anticheat_events` rows logged against two different sessions
    /// belonging to the same subscriber (proving the join reaches
    /// across every one of the account's sessions, not just one),
    /// come back newest-first with `event_type`/`severity`/`details`
    /// intact, and a `limit` of `1` returns only the newest row.
    #[tokio::test]
    async fn recent_violations_orders_newest_first_across_sessions() {
        let Some(pool) = connect().await else {
            return;
        };
        let repo = PgAntiCheatRepository::new(pool.clone());

        sqlx::query("delete from accounts where username = 'ac_violations_fixture_acct'")
            .execute(&pool)
            .await
            .expect("pre-clean fixture account");
        let (account_id,): (i64,) = sqlx::query_as(
            "insert into accounts(username, password_hash) values ($1, 'secret') returning id",
        )
        .bind("ac_violations_fixture_acct")
        .fetch_one(&pool)
        .await
        .expect("insert fixture account");

        let (session_a,): (i64,) = sqlx::query_as(
            "insert into anticheat_sessions(account_id, ip_address, area_id) \
                 values ($1, 1, 1) returning id",
        )
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .expect("insert fixture session a");
        let (session_b,): (i64,) = sqlx::query_as(
            "insert into anticheat_sessions(account_id, ip_address, area_id) \
                 values ($1, 1, 1) returning id",
        )
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .expect("insert fixture session b");

        sqlx::query(
            "insert into anticheat_events(session_id, event_type, severity, details, \
                 created_at) values ($1, 'speedhack', 1, 'moved too fast', \
                 now() - interval '1 hour')",
        )
        .bind(session_a)
        .execute(&pool)
        .await
        .expect("insert older violation");
        sqlx::query(
            "insert into anticheat_events(session_id, event_type, severity, details, \
                 created_at) values ($1, 'teleport', 2, 'impossible jump', now())",
        )
        .bind(session_b)
        .execute(&pool)
        .await
        .expect("insert newer violation");

        let rows = repo
            .recent_violations(account_id, 15)
            .await
            .expect("recent_violations");
        assert_eq!(
            rows.len(),
            2,
            "violations from both sessions must come back"
        );
        assert_eq!(
            rows[0].type_name, "teleport",
            "newest violation sorts first"
        );
        assert_eq!(rows[0].severity, 2);
        assert_eq!(rows[0].details.as_deref(), Some("impossible jump"));
        assert_eq!(rows[1].type_name, "speedhack");
        assert_eq!(rows[1].details.as_deref(), Some("moved too fast"));

        let limited = repo
            .recent_violations(account_id, 1)
            .await
            .expect("recent_violations with limit 1");
        assert_eq!(limited.len(), 1);
        assert_eq!(
            limited[0].type_name, "teleport",
            "limit must keep only the newest row"
        );

        sqlx::query("delete from anticheat_events where session_id = any($1)")
            .bind([session_a, session_b])
            .execute(&pool)
            .await
            .expect("cleanup fixture events");
        sqlx::query("delete from anticheat_sessions where id = any($1)")
            .bind([session_a, session_b])
            .execute(&pool)
            .await
            .expect("cleanup fixture sessions");
        sqlx::query("delete from accounts where id = $1")
            .bind(account_id)
            .execute(&pool)
            .await
            .expect("cleanup fixture account");
    }

    /// `#acsigadd`/`#acsiglist`/`#acsigdel`'s full round trip: add two
    /// signatures with different `times_detected` (bumped directly by
    /// SQL, since `add_signature` itself always inserts a fresh row at
    /// `0` - matching C's own insert, which never seeds a detection
    /// count), confirm `list_signatures` orders by `times_detected`
    /// descending with a `limit` of `1` keeping only the highest one,
    /// then delete both and confirm `delete_signature` reports
    /// `Ok(false)` for an id that no longer exists (C's own `affected
    /// == 0` branch).
    #[tokio::test]
    async fn add_list_and_delete_signature_round_trip() {
        let Some(pool) = connect().await else {
            return;
        };
        let repo = PgAntiCheatRepository::new(pool.clone());

        sqlx::query("delete from ac_known_signatures where name = any($1)")
            .bind(["Sig Fixture Low", "Sig Fixture High"])
            .execute(&pool)
            .await
            .expect("pre-clean fixture signatures");

        repo.add_signature("hardware_hash", "deadbeef", "Sig Fixture Low", "TestGod")
            .await
            .expect("add low-detection signature");
        repo.add_signature(
            "process_name",
            "cheatengine.exe",
            "Sig Fixture High",
            "TestGod",
        )
        .await
        .expect("add high-detection signature");

        let (low_id, high_id): (i64, i64) = {
            let low: (i64,) = sqlx::query_as("select id from ac_known_signatures where name = $1")
                .bind("Sig Fixture Low")
                .fetch_one(&pool)
                .await
                .expect("fetch low fixture id");
            let high: (i64,) = sqlx::query_as("select id from ac_known_signatures where name = $1")
                .bind("Sig Fixture High")
                .fetch_one(&pool)
                .await
                .expect("fetch high fixture id");
            (low.0, high.0)
        };
        sqlx::query("update ac_known_signatures set times_detected = 5 where id = $1")
            .bind(high_id)
            .execute(&pool)
            .await
            .expect("bump high-detection fixture's counter");

        let rows = repo.list_signatures(50).await.expect("list_signatures");
        let low_row = rows
            .iter()
            .find(|row| row.id == low_id)
            .expect("low fixture row present");
        let high_row = rows
            .iter()
            .find(|row| row.id == high_id)
            .expect("high fixture row present");
        assert_eq!(low_row.signature_type, "hardware_hash");
        assert_eq!(low_row.severity, 0);
        assert!(!low_row.auto_flag);
        assert!(!low_row.auto_ban);
        assert_eq!(low_row.times_detected, 0);
        assert!(low_row.is_active);
        assert_eq!(high_row.signature_type, "process_name");
        assert_eq!(high_row.times_detected, 5);
        let high_index = rows.iter().position(|row| row.id == high_id).unwrap();
        let low_index = rows.iter().position(|row| row.id == low_id).unwrap();
        assert!(
            high_index < low_index,
            "higher times_detected must sort first"
        );

        let limited = repo
            .list_signatures(1)
            .await
            .expect("list_signatures with limit 1");
        assert_eq!(limited.len(), 1);
        assert_eq!(
            limited[0].id, high_id,
            "limit must keep only the highest-detection row"
        );

        assert!(
            repo.delete_signature(low_id).await.expect("delete low"),
            "deleting an existing row must report true"
        );
        assert!(
            repo.delete_signature(high_id).await.expect("delete high"),
            "deleting an existing row must report true"
        );
        assert!(
            !repo
                .delete_signature(low_id)
                .await
                .expect("delete already-deleted row"),
            "deleting a vanished id must report false, not error"
        );
    }

    /// `ac_player_disconnect`'s rollup half (`update_player_stats`):
    /// three sequential disconnects for the same subscriber exercise
    /// every `risk_level` tier C's `db_ac_update_player_stats`
    /// threshold `CASE` computes - `low` (a clean-ish first session),
    /// `high` (a flagged session whose bot score alone crosses `0.8`),
    /// then `critical` (a session whose bot score reaches `1.0`) -
    /// while also asserting the plain accumulator columns
    /// (`total_sessions`/violation totals/`lifetime_bot_score`) add up
    /// across calls rather than overwrite, and `max_session_bot_score`
    /// only ever grows (`GREATEST`, matching C's `@mbs := GREATEST(...
    /// )`).
    #[tokio::test]
    async fn update_player_stats_accumulates_and_classifies_risk_tiers() {
        let Some(pool) = connect().await else {
            return;
        };
        let repo = PgAntiCheatRepository::new(pool.clone());

        sqlx::query("delete from accounts where username = 'ac_rollup_fixture_acct'")
            .execute(&pool)
            .await
            .expect("pre-clean fixture account");
        let (account_id,): (i64,) = sqlx::query_as(
            "insert into accounts(username, password_hash) values ($1, 'secret') returning id",
        )
        .bind("ac_rollup_fixture_acct")
        .fetch_one(&pool)
        .await
        .expect("insert fixture account");

        // First touch: no `ac_player_stats` row exists yet - a clean,
        // unverified (status 0) session with a low bot score creates
        // one at the `low` tier.
        repo.update_player_stats(account_id, 0.4, 0, 1, 1, 1, 0)
            .await
            .expect("update_player_stats first call");

        let row: (i32, i32, i32, i32, i32, i32, i32, f32, f32, f32, String) = sqlx::query_as(
            "select total_sessions, flagged_sessions, suspicious_sessions, \
                 total_heartbeat_violations, total_state_violations, \
                 total_challenge_failures, total_anomalies, lifetime_bot_score, \
                 max_session_bot_score, avg_session_bot_score, risk_level \
                 from ac_player_stats where subscriber_id = $1",
        )
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .expect("fetch ac_player_stats row after first call");
        assert_eq!(row.0, 1, "total_sessions");
        assert_eq!(row.1, 0, "flagged_sessions");
        assert_eq!(row.2, 0, "suspicious_sessions");
        assert_eq!(row.3, 1, "total_heartbeat_violations");
        assert_eq!(row.4, 1, "total_state_violations");
        assert_eq!(row.5, 1, "total_challenge_failures");
        assert_eq!(row.6, 0, "total_anomalies");
        assert!((row.7 - 0.4).abs() < 0.001, "lifetime_bot_score");
        assert!((row.8 - 0.4).abs() < 0.001, "max_session_bot_score");
        assert!((row.9 - 0.4).abs() < 0.001, "avg_session_bot_score");
        assert_eq!(row.10, "low");

        // Second touch: a flagged (status 3) session with a bot score
        // crossing 0.8 must push the row to `high` (bot score alone,
        // since flagged_sessions is still only 1, below the `>= 3`
        // threshold) and accumulate every counter rather than
        // overwrite it.
        repo.update_player_stats(account_id, 0.9, 3, 2, 0, 0, 0)
            .await
            .expect("update_player_stats second call");

        let row: (i32, i32, i32, f32, f32, String) = sqlx::query_as(
            "select total_sessions, flagged_sessions, total_heartbeat_violations, \
                 lifetime_bot_score, max_session_bot_score, risk_level \
                 from ac_player_stats where subscriber_id = $1",
        )
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .expect("fetch ac_player_stats row after second call");
        assert_eq!(row.0, 2, "total_sessions accumulates");
        assert_eq!(row.1, 1, "flagged_sessions accumulates");
        assert_eq!(row.2, 3, "total_heartbeat_violations accumulates (1 + 2)");
        assert!(
            (row.3 - 1.3).abs() < 0.001,
            "lifetime_bot_score accumulates"
        );
        assert!(
            (row.4 - 0.9).abs() < 0.001,
            "max_session_bot_score takes the new higher value"
        );
        assert_eq!(row.5, "high");

        // Third touch: a session whose bot score reaches 1.0 must
        // push the row to `critical`, and a lower bot score on this
        // same call must NOT lower `max_session_bot_score` back down
        // (`GREATEST`, not overwrite).
        repo.update_player_stats(account_id, 1.0, 3, 0, 0, 0, 0)
            .await
            .expect("update_player_stats third call");
        repo.update_player_stats(account_id, 0.1, 1, 0, 0, 0, 0)
            .await
            .expect("update_player_stats fourth call (low score, verified status)");

        let row: (i32, i32, f32, String) = sqlx::query_as(
            "select total_sessions, flagged_sessions, max_session_bot_score, risk_level \
                 from ac_player_stats where subscriber_id = $1",
        )
        .bind(account_id)
        .fetch_one(&pool)
        .await
        .expect("fetch ac_player_stats row after fourth call");
        assert_eq!(row.0, 4, "total_sessions accumulates across all four calls");
        assert_eq!(
            row.1, 2,
            "flagged_sessions unaffected by a non-flagged call"
        );
        assert!(
            (row.2 - 1.0).abs() < 0.001,
            "max_session_bot_score never drops back down"
        );
        assert_eq!(
            row.3, "critical",
            "risk_level stays critical once max_session_bot_score hit 1.0"
        );

        sqlx::query("delete from ac_player_stats where subscriber_id = $1")
            .bind(account_id)
            .execute(&pool)
            .await
            .expect("cleanup ac_player_stats row");
        sqlx::query("delete from accounts where id = $1")
            .bind(account_id)
            .execute(&pool)
            .await
            .expect("cleanup fixture account");
    }

    /// `#achistory`'s backing read (`find_player_stats`): `None` when
    /// no `ac_player_stats` row exists yet (matching C's own null-
    /// `mysql_fetch_row` branch), then a full round trip of every
    /// column - including `first_seen` staying fixed across a second
    /// `update_player_stats` call on the same subscriber, matching
    /// C's set-once `db_ac_ensure_player_stats` semantics for the
    /// equivalent field (this migration's own doc comment explicitly
    /// requires `first_seen` to never be touched by the upsert's `do
    /// update` clause).
    #[tokio::test]
    async fn find_player_stats_reports_none_then_round_trips_every_column() {
        let Some(pool) = connect().await else {
            return;
        };
        let repo = PgAntiCheatRepository::new(pool.clone());

        sqlx::query("delete from accounts where username = 'ac_history_fixture_acct'")
            .execute(&pool)
            .await
            .expect("pre-clean fixture account");
        let (account_id,): (i64,) = sqlx::query_as(
            "insert into accounts(username, password_hash) values ($1, 'secret') returning id",
        )
        .bind("ac_history_fixture_acct")
        .fetch_one(&pool)
        .await
        .expect("insert fixture account");

        assert!(
            repo.find_player_stats(account_id)
                .await
                .expect("find_player_stats before any row exists")
                .is_none(),
            "no ac_player_stats row must report None, not an error"
        );

        repo.update_player_stats(account_id, 0.9, 3, 1, 2, 3, 0)
            .await
            .expect("update_player_stats first call");
        repo.set_trusted(account_id, true)
            .await
            .expect("set_trusted");
        repo.issue_warning(account_id).await.expect("issue_warning");

        let first_read = repo
            .find_player_stats(account_id)
            .await
            .expect("find_player_stats after first call")
            .expect("row must exist now");
        assert_eq!(first_read.total_sessions, 1);
        assert_eq!(first_read.flagged_sessions, 1);
        assert_eq!(first_read.suspicious_sessions, 0);
        assert_eq!(first_read.total_heartbeat_violations, 1);
        assert_eq!(first_read.total_state_violations, 2);
        assert_eq!(first_read.total_challenge_failures, 3);
        assert_eq!(first_read.total_anomalies, 0);
        assert!((first_read.max_session_bot_score - 0.9).abs() < 0.001);
        assert!((first_read.avg_session_bot_score - 0.9).abs() < 0.001);
        assert_eq!(first_read.risk_level, "high");
        assert!(
            !first_read.is_flagged,
            "set_trusted must not touch is_flagged"
        );
        assert!(first_read.is_trusted);
        assert_eq!(first_read.warnings_issued, 1);
        assert!(!first_read.first_seen.is_empty());
        assert!(first_read.last_seen.is_some());

        // A second rollup call must accumulate the counters but must
        // NOT move first_seen.
        repo.update_player_stats(account_id, 0.1, 0, 0, 0, 0, 0)
            .await
            .expect("update_player_stats second call");
        let second_read = repo
            .find_player_stats(account_id)
            .await
            .expect("find_player_stats after second call")
            .expect("row must still exist");
        assert_eq!(second_read.total_sessions, 2);
        assert_eq!(
            second_read.first_seen, first_read.first_seen,
            "first_seen must never move once set"
        );

        sqlx::query("delete from ac_player_stats where subscriber_id = $1")
            .bind(account_id)
            .execute(&pool)
            .await
            .expect("cleanup ac_player_stats row");
        sqlx::query("delete from accounts where id = $1")
            .bind(account_id)
            .execute(&pool)
            .await
            .expect("cleanup fixture account");
    }
}
