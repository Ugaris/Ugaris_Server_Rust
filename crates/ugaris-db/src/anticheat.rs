use async_trait::async_trait;
use sqlx::{types::Json, PgPool};
use std::collections::BTreeMap;
use ugaris_core::ids::CharacterId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AntiCheatSessionCreate {
    pub login_session_id: Option<i64>,
    pub account_id: Option<i64>,
    pub character_id: Option<CharacterId>,
    pub ip_address: i32,
    pub area_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AntiCheatFingerprint {
    pub mod_major: u8,
    pub mod_minor: u8,
    pub mod_patch: u8,
    pub os_type: u8,
    pub screen_w: u16,
    pub screen_h: u16,
    pub hardware_hash: u32,
    pub code_hash: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AntiCheatCounters {
    pub heartbeat_delta: i32,
    pub state_delta: i32,
    pub challenge_delta: i32,
    pub anomaly_delta: i32,
    pub timeout_delta: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AntiCheatEvent {
    pub session_id: i64,
    pub event_type: String,
    pub severity: i32,
    pub details: Option<String>,
    pub data: BTreeMap<String, String>,
}

#[async_trait]
pub trait AntiCheatRepository: Send + Sync {
    async fn create_session(&self, request: AntiCheatSessionCreate) -> anyhow::Result<i64>;
    async fn set_character(
        &self,
        session_id: i64,
        character_id: CharacterId,
    ) -> anyhow::Result<bool>;
    async fn set_fingerprint(
        &self,
        session_id: i64,
        fingerprint: AntiCheatFingerprint,
    ) -> anyhow::Result<bool>;
    async fn set_status(&self, session_id: i64, status: i32) -> anyhow::Result<bool>;
    async fn update_bot_score(
        &self,
        session_id: i64,
        bot_score: f32,
        is_max: bool,
    ) -> anyhow::Result<bool>;
    async fn increment_counters(
        &self,
        session_id: i64,
        counters: AntiCheatCounters,
    ) -> anyhow::Result<bool>;
    async fn end_session(&self, session_id: i64, final_bot_score: f32) -> anyhow::Result<bool>;
    async fn log_event(&self, event: AntiCheatEvent) -> anyhow::Result<i64>;
    async fn cleanup_old_records(&self, days_to_keep: i32) -> anyhow::Result<u64>;
}

#[derive(Debug, Clone)]
pub struct PgAntiCheatRepository {
    pool: PgPool,
}

impl PgAntiCheatRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AntiCheatRepository for PgAntiCheatRepository {
    async fn create_session(&self, request: AntiCheatSessionCreate) -> anyhow::Result<i64> {
        let (session_id,) = sqlx::query_as::<_, (i64,)>(
            "insert into anticheat_sessions(\
             login_session_id, account_id, character_id, ip_address, area_id) \
             values ($1, $2, $3, $4, $5) returning id",
        )
        .bind(request.login_session_id)
        .bind(request.account_id)
        .bind(request.character_id.map(|id| id.0 as i64))
        .bind(request.ip_address)
        .bind(request.area_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(session_id)
    }

    async fn set_character(
        &self,
        session_id: i64,
        character_id: CharacterId,
    ) -> anyhow::Result<bool> {
        let result = sqlx::query("update anticheat_sessions set character_id = $1 where id = $2")
            .bind(character_id.0 as i64)
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_fingerprint(
        &self,
        session_id: i64,
        fingerprint: AntiCheatFingerprint,
    ) -> anyhow::Result<bool> {
        let result = sqlx::query(
            "update anticheat_sessions set \
             mod_major = $1, mod_minor = $2, mod_patch = $3, os_type = $4, \
             screen_w = $5, screen_h = $6, hardware_hash = $7, code_hash = $8 \
             where id = $9",
        )
        .bind(i32::from(fingerprint.mod_major))
        .bind(i32::from(fingerprint.mod_minor))
        .bind(i32::from(fingerprint.mod_patch))
        .bind(i32::from(fingerprint.os_type))
        .bind(i32::from(fingerprint.screen_w))
        .bind(i32::from(fingerprint.screen_h))
        .bind(i64::from(fingerprint.hardware_hash))
        .bind(i64::from(fingerprint.code_hash))
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn set_status(&self, session_id: i64, status: i32) -> anyhow::Result<bool> {
        let result = sqlx::query("update anticheat_sessions set status = $1 where id = $2")
            .bind(status)
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn update_bot_score(
        &self,
        session_id: i64,
        bot_score: f32,
        is_max: bool,
    ) -> anyhow::Result<bool> {
        let sql = if is_max {
            "update anticheat_sessions set bot_score = $1, max_bot_score = greatest(max_bot_score, $1) where id = $2"
        } else {
            "update anticheat_sessions set bot_score = $1 where id = $2"
        };
        let result = sqlx::query(sql)
            .bind(bot_score)
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn increment_counters(
        &self,
        session_id: i64,
        counters: AntiCheatCounters,
    ) -> anyhow::Result<bool> {
        let result = sqlx::query(
            "update anticheat_sessions set \
             heartbeat_violations = heartbeat_violations + $1, \
             state_violations = state_violations + $2, \
             challenge_failures = challenge_failures + $3, \
             anomaly_count = anomaly_count + $4, timeout_count = timeout_count + $5 \
             where id = $6",
        )
        .bind(counters.heartbeat_delta)
        .bind(counters.state_delta)
        .bind(counters.challenge_delta)
        .bind(counters.anomaly_delta)
        .bind(counters.timeout_delta)
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn end_session(&self, session_id: i64, final_bot_score: f32) -> anyhow::Result<bool> {
        let result = sqlx::query(
            "update anticheat_sessions set ended_at = now(), bot_score = $1 where id = $2",
        )
        .bind(final_bot_score)
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn log_event(&self, event: AntiCheatEvent) -> anyhow::Result<i64> {
        let (event_id,) = sqlx::query_as::<_, (i64,)>(
            "insert into anticheat_events(session_id, event_type, severity, details, data) \
             values ($1, $2, $3, $4, $5) returning id",
        )
        .bind(event.session_id)
        .bind(event.event_type)
        .bind(event.severity)
        .bind(event.details)
        .bind(Json(event.data))
        .fetch_one(&self.pool)
        .await?;

        Ok(event_id)
    }

    async fn cleanup_old_records(&self, days_to_keep: i32) -> anyhow::Result<u64> {
        let result = sqlx::query(
            "delete from anticheat_sessions \
             where ended_at is not null and ended_at < now() - ($1 * interval '1 day')",
        )
        .bind(days_to_keep)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}

pub fn legacy_result_name(result: i32) -> &'static str {
    match result {
        0 => "pass",
        1 => "fail",
        2 => "timeout",
        _ => "pass",
    }
}

pub fn legacy_signature_action_name(action: i32) -> &'static str {
    match action {
        0 => "none",
        1 => "flagged",
        2 => "warned",
        3 => "banned",
        _ => "none",
    }
}

pub fn legacy_risk_name(risk: i32) -> &'static str {
    match risk {
        0 => "low",
        1 => "medium",
        2 => "high",
        3 => "critical",
        _ => "low",
    }
}

#[cfg(test)]
mod tests {
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
            let (character_id,): (i64,) = sqlx::query_as(
                "insert into characters(account_id, name) values ($1, $2) returning id",
            )
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
        }
    }
}
