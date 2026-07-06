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

/// The subset of an `anticheat_sessions` row displayed by the `#acstatus`/
/// `#acstats`/`#aclist` admin commands (`ac_cmd_status`/`ac_cmd_stats`/
/// `ac_cmd_list`, `src/module/anticheat/anticheat.c:473-543,604-628,
/// 721-753`). C reads these fields straight out of the in-memory
/// `player[nr]->ac` struct; this codebase has no such struct (see
/// `ugaris-core`'s `world/anticheat.rs` module doc comment), so they are
/// queried back out of the same row `create_session`/`increment_counters`/
/// `update_bot_score`/`set_fingerprint` already write to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AntiCheatSessionInfo {
    pub status: i32,
    pub bot_score: f32,
    pub heartbeat_violations: i32,
    pub state_violations: i32,
    pub challenge_failures: i32,
    pub timeout_count: i32,
    pub mod_major: Option<i32>,
    pub mod_minor: Option<i32>,
    pub mod_patch: Option<i32>,
    pub os_type: Option<i32>,
    pub screen_w: Option<i32>,
    pub screen_h: Option<i32>,
}

/// `#acsessions <player>`'s per-row shape (`db_ac_session_result`,
/// `database_anticheat.h:457-476`). C's own query reads from a separate
/// `ac_sessions` history table populated by `db_ac_session_create`/
/// `db_ac_session_end`/etc.; this codebase's `anticheat_sessions` table
/// (`migrations/0002_sessions_questlog_anticheat.sql`) already is that
/// same per-session history (one row per login, never overwritten in
/// place except by the same handful of columns C's `ac_sessions` table
/// tracks), so `recent_sessions` below reads it directly rather than
/// requiring a second table - see `PgAntiCheatRepository::recent_
/// sessions`'s doc comment for the column-by-column mapping.
#[derive(Debug, Clone, PartialEq)]
pub struct AntiCheatSessionHistoryRow {
    pub start_time: String,
    pub duration_minutes: i32,
    pub status: i32,
    pub bot_score: f32,
    pub heartbeat_violations: i32,
    pub state_violations: i32,
    pub challenge_failures: i32,
    pub anomaly_count: i32,
}

/// `#acviolations <player>`'s per-row shape (`db_ac_violation_result`,
/// `database_anticheat.h:481-`). C's own query reads from a separate
/// `ac_violations` table (populated by `db_ac_log_violation`) joined
/// against `ac_violation_types` for a human-readable `type_name`; this
/// codebase already has a per-session violation/event log in
/// `anticheat_events` (`migrations/0002_sessions_questlog_anticheat.
/// sql`, written by `AntiCheatRepository::log_event`) with `event_type`
/// stored directly as text rather than a foreign-key id, so no join (or
/// new `ac_violation_types`-equivalent table) is needed - see
/// `PgAntiCheatRepository::recent_violations`'s doc comment for the
/// column-by-column mapping, same reuse-over-new-schema approach
/// `recent_sessions` above already took for `#acsessions`.
#[derive(Debug, Clone, PartialEq)]
pub struct AntiCheatViolationRow {
    pub detected_at: String,
    pub type_name: String,
    pub severity: i32,
    pub details: Option<String>,
}

/// `#acsiglist`'s per-row shape (`db_ac_signature_result`,
/// `database_anticheat.h:589-598`), backed by the new `ac_known_
/// signatures` table (`migrations/0016_ac_known_signatures.sql`) - C's
/// own `ac_known_signatures` table existed only as a name referenced by
/// `db_ac_get_signatures`/`db_ac_add_signature`/`db_ac_delete_signature`,
/// never itself defined anywhere in this codebase before this slice.
/// `signature_value` is deliberately absent: C's own `db_ac_get_
/// signatures` query never selects it either (only `#acsigadd` ever
/// writes it - no admin command reads it back), reproduced as-is rather
/// than "fixed".
#[derive(Debug, Clone, PartialEq)]
pub struct AntiCheatSignatureRow {
    pub id: i64,
    pub signature_type: String,
    pub name: String,
    pub severity: i32,
    pub auto_flag: bool,
    pub auto_ban: bool,
    pub times_detected: i32,
    pub is_active: bool,
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
    /// `#acreset <player>`'s backing mutation (`ac_cmd_reset`,
    /// `anticheat.c:527-561`): zeroes the same fields C's in-memory reset
    /// touches (`hb_violations`, `state_violations`,
    /// `challenge_fail_count`, `bot_score`, `timeout_count`) and restores
    /// `status` to `AC_STATUS_VERIFIED` (1). C does not touch
    /// `max_bot_score_session`/`anomaly_count`, so this query leaves this
    /// row's `max_bot_score`/`anomaly_count` columns untouched too.
    async fn reset_session(&self, session_id: i64) -> anyhow::Result<bool>;
    /// `#acstatus <name>`'s backing query: a single session row by id
    /// (`PlayerRuntime::anticheat_session_id`, already known synchronously
    /// by the caller - see `world/anticheat.rs`'s module doc comment for
    /// why the name-to-session-id resolution happens before this call
    /// rather than inside it). `None` when the row no longer exists.
    async fn find_session(&self, session_id: i64) -> anyhow::Result<Option<AntiCheatSessionInfo>>;
    /// `#acstats`/`#aclist`'s backing query: every session row named by
    /// `session_ids`, batched into one round trip (C's `ac_cmd_stats`/
    /// `ac_cmd_list` instead re-read the in-memory `player[nr]->ac` struct
    /// once per online player in a single-process loop - see the same
    /// `world/anticheat.rs` module doc comment). Rows for an id that no
    /// longer exists are simply omitted, not padded with defaults.
    async fn find_sessions(
        &self,
        session_ids: &[i64],
    ) -> anyhow::Result<Vec<(i64, AntiCheatSessionInfo)>>;
    /// `#acunflag`/`#actrust`/`#acuntrust`'s subscriber-id resolution: C's
    /// `get_subscriberId_from_character` reads `chars.sID` (the owning
    /// account, exactly this codebase's `characters.account_id`); since
    /// this codebase never threads `account_id` through `World`/
    /// `PlayerRuntime` after login (see `ugaris-core`'s `world/
    /// anticheat.rs` module doc comment addendum), the same value is
    /// instead read back out of `anticheat_sessions.account_id`, already
    /// stored there by `create_session` at login. `None` when the row is
    /// gone or was created with no account (matching C's `target_
    /// subscriber <= 0` "not a real account" branch).
    async fn account_id_for_session(&self, session_id: i64) -> anyhow::Result<Option<i64>>;
    /// `#acunflag`'s/`#acflag`-sibling's persistent half
    /// (`db_ac_flag_player`, `database_anticheat.c:553-570`): upserts
    /// `ac_player_stats.is_flagged` for the subscriber, creating the row
    /// on first touch exactly like C's own `db_ac_ensure_player_stats`
    /// pre-step. `db_ac_log_admin_action`'s audit-trail row is skipped
    /// (no `ac_admin_actions` table exists in this codebase - the same
    /// skip-untracked-audit-log convention `/kick`'s dropped `dlog` call
    /// already established).
    async fn set_flagged(&self, subscriber_id: i64, is_flagged: bool) -> anyhow::Result<()>;
    /// `#actrust`/`#acuntrust`'s persistent half (`db_ac_trust_player`,
    /// `database_anticheat.c:572-589`): upserts `ac_player_stats.
    /// is_trusted` for the subscriber, same ensure-then-update shape and
    /// same skipped `db_ac_log_admin_action` audit row as `set_flagged`.
    async fn set_trusted(&self, subscriber_id: i64, is_trusted: bool) -> anyhow::Result<()>;
    /// `#acwarn <player> [reason]`'s persistent half (`db_ac_issue_
    /// warning`, `database_anticheat.c:606-621`): upserts `ac_player_
    /// stats.warnings_issued`/`last_warning_at` for the subscriber,
    /// incrementing the counter (not overwriting it) on every call - same
    /// ensure-then-update shape and same skipped `db_ac_log_admin_action`
    /// audit row as `set_flagged`/`set_trusted`.
    async fn issue_warning(&self, subscriber_id: i64) -> anyhow::Result<()>;
    /// `#acsessions <player>`'s backing query (`db_ac_get_recent_
    /// sessions`, `database_anticheat.c:883-919`): the `max_count` most
    /// recent `anticheat_sessions` rows for the subscriber's account,
    /// newest first - see `AntiCheatSessionHistoryRow`'s doc comment for
    /// why this reads the existing `anticheat_sessions` table (C's own
    /// `ac_sessions`) rather than a new one. `duration_minutes` mirrors
    /// C's `TIMESTAMPDIFF(MINUTE, session_start, COALESCE(session_end,
    /// NOW()))` (an in-progress session's duration is measured against
    /// "now"); `status`/`bot_score`/violation counters read back exactly
    /// the same columns `#acstatus`'s `find_session` does.
    async fn recent_sessions(
        &self,
        account_id: i64,
        max_count: i64,
    ) -> anyhow::Result<Vec<AntiCheatSessionHistoryRow>>;
    /// `#acviolations <player>`'s backing query (`db_ac_get_recent_
    /// violations`, `database_anticheat.c:922-955`): the `max_count` most
    /// recent `anticheat_events` rows across every one of the
    /// subscriber's `anticheat_sessions`, newest first - see
    /// `AntiCheatViolationRow`'s doc comment for why this reads the
    /// existing `anticheat_events` table (C's own `ac_violations`,
    /// joined against `ac_violation_types` there only because C stores
    /// the type as a numeric foreign key; this codebase's `event_type`
    /// column is already the human-readable name) rather than a new
    /// pair of tables.
    async fn recent_violations(
        &self,
        account_id: i64,
        max_count: i64,
    ) -> anyhow::Result<Vec<AntiCheatViolationRow>>;
    /// `#acsiglist`'s backing query (`db_ac_get_signatures`,
    /// `database_anticheat.c:1143-1180`): every row in `ac_known_
    /// signatures`, ordered by `times_detected` descending (matching C's
    /// own `ORDER BY times_detected DESC`), capped at `max_count` (C's
    /// own `results[20]` stack array) - see `AntiCheatSignatureRow`'s doc
    /// comment for why `signature_value` is never selected.
    async fn list_signatures(&self, max_count: i64) -> anyhow::Result<Vec<AntiCheatSignatureRow>>;
    /// `#acsigadd <type> <value> <name>`'s backing mutation
    /// (`db_ac_add_signature`, `database_anticheat.c:1182-1206`): a plain
    /// insert with no upsert/conflict handling, matching C's own query
    /// exactly - adding the same signature twice creates two rows, a
    /// genuine quirk preserved as-is.
    async fn add_signature(
        &self,
        signature_type: &str,
        signature_value: &str,
        name: &str,
        created_by: &str,
    ) -> anyhow::Result<()>;
    /// `#acsigdel <id>`'s backing mutation (`db_ac_delete_signature`,
    /// `database_anticheat.c:1208-1216`): `Ok(false)` when no row with
    /// that id exists (C's own `affected == 0` branch), matching every
    /// other `bool`-returning mutator in this trait.
    async fn delete_signature(&self, signature_id: i64) -> anyhow::Result<bool>;
    /// The session-end lifetime-rollup half of `ac_player_disconnect`
    /// (`db_ac_update_player_stats`, `database_anticheat.c:480-517`),
    /// called right after `end_session` with the same pre-mutation
    /// session snapshot `#acstatus`'s `find_session` already reads (C
    /// reads the equivalent fields straight out of the in-memory
    /// `player[nr]->ac` struct before either DB call touches it). Folds
    /// C's `db_ac_ensure_player_stats`-then-atomic-`UPDATE` two-step into
    /// one upsert, same shape as `set_flagged`/`set_trusted`/
    /// `issue_warning`. `session_status` is the raw `AC_STATUS_*` value
    /// (`0`=unverified, `1`=verified, `2`=suspicious, `3`=flagged); C's
    /// own `was_flagged`/`was_suspicious` booleans (`session_status == 3`/
    /// `== 2`) are derived from it inside the implementation, not by the
    /// caller. `anomalies` is always `0` at the only current call site,
    /// matching C's own literal `0` argument ("anomaly count is tracked
    /// separately") - the parameter exists for a future detection-engine
    /// slice, not because any caller currently supplies a nonzero value.
    /// Reproduces C's `risk_level` thresholds (`max_session_bot_score >=
    /// 1.0 OR flagged_sessions >= 3` -> `critical`, `>= 0.8 OR >= 1` ->
    /// `high`, `>= 0.5 OR suspicious_sessions >= 3` -> `medium`, else
    /// `low`) via Postgres `GREATEST`/`CASE` over `excluded.*` plus the
    /// pre-update column value, instead of MySQL's `@variable` trick - no
    /// read-your-own-write ordering issue here since every intermediate
    /// is recomputed directly from the same two operands in each branch.
    #[allow(clippy::too_many_arguments)]
    async fn update_player_stats(
        &self,
        subscriber_id: i64,
        session_bot_score: f32,
        session_status: i32,
        heartbeat_violations: i32,
        state_violations: i32,
        challenge_failures: i32,
        anomalies: i32,
    ) -> anyhow::Result<()>;
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

    async fn reset_session(&self, session_id: i64) -> anyhow::Result<bool> {
        let result = sqlx::query(
            "update anticheat_sessions set \
             status = 1, bot_score = 0, heartbeat_violations = 0, \
             state_violations = 0, challenge_failures = 0, timeout_count = 0 \
             where id = $1",
        )
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn find_session(&self, session_id: i64) -> anyhow::Result<Option<AntiCheatSessionInfo>> {
        let row = sqlx::query_as::<
            _,
            (
                i32,
                f32,
                i32,
                i32,
                i32,
                i32,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
            ),
        >(
            "select status, bot_score, heartbeat_violations, state_violations, \
             challenge_failures, timeout_count, mod_major, mod_minor, mod_patch, \
             os_type, screen_w, screen_h \
             from anticheat_sessions where id = $1",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(
            |(
                status,
                bot_score,
                heartbeat_violations,
                state_violations,
                challenge_failures,
                timeout_count,
                mod_major,
                mod_minor,
                mod_patch,
                os_type,
                screen_w,
                screen_h,
            )| AntiCheatSessionInfo {
                status,
                bot_score,
                heartbeat_violations,
                state_violations,
                challenge_failures,
                timeout_count,
                mod_major,
                mod_minor,
                mod_patch,
                os_type,
                screen_w,
                screen_h,
            },
        ))
    }

    async fn find_sessions(
        &self,
        session_ids: &[i64],
    ) -> anyhow::Result<Vec<(i64, AntiCheatSessionInfo)>> {
        if session_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<
            _,
            (
                i64,
                i32,
                f32,
                i32,
                i32,
                i32,
                i32,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
                Option<i32>,
            ),
        >(
            "select id, status, bot_score, heartbeat_violations, state_violations, \
             challenge_failures, timeout_count, mod_major, mod_minor, mod_patch, \
             os_type, screen_w, screen_h \
             from anticheat_sessions where id = any($1)",
        )
        .bind(session_ids)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    status,
                    bot_score,
                    heartbeat_violations,
                    state_violations,
                    challenge_failures,
                    timeout_count,
                    mod_major,
                    mod_minor,
                    mod_patch,
                    os_type,
                    screen_w,
                    screen_h,
                )| {
                    (
                        id,
                        AntiCheatSessionInfo {
                            status,
                            bot_score,
                            heartbeat_violations,
                            state_violations,
                            challenge_failures,
                            timeout_count,
                            mod_major,
                            mod_minor,
                            mod_patch,
                            os_type,
                            screen_w,
                            screen_h,
                        },
                    )
                },
            )
            .collect())
    }

    async fn account_id_for_session(&self, session_id: i64) -> anyhow::Result<Option<i64>> {
        let row = sqlx::query_as::<_, (Option<i64>,)>(
            "select account_id from anticheat_sessions where id = $1",
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.and_then(|(account_id,)| account_id))
    }

    async fn set_flagged(&self, subscriber_id: i64, is_flagged: bool) -> anyhow::Result<()> {
        sqlx::query(
            "insert into ac_player_stats (subscriber_id, is_flagged, updated_at) \
             values ($1, $2, now()) \
             on conflict (subscriber_id) do update set is_flagged = $2, updated_at = now()",
        )
        .bind(subscriber_id)
        .bind(is_flagged)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn set_trusted(&self, subscriber_id: i64, is_trusted: bool) -> anyhow::Result<()> {
        sqlx::query(
            "insert into ac_player_stats (subscriber_id, is_trusted, updated_at) \
             values ($1, $2, now()) \
             on conflict (subscriber_id) do update set is_trusted = $2, updated_at = now()",
        )
        .bind(subscriber_id)
        .bind(is_trusted)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn issue_warning(&self, subscriber_id: i64) -> anyhow::Result<()> {
        sqlx::query(
            "insert into ac_player_stats (subscriber_id, warnings_issued, last_warning_at, \
             updated_at) values ($1, 1, now(), now()) \
             on conflict (subscriber_id) do update set \
             warnings_issued = ac_player_stats.warnings_issued + 1, last_warning_at = now(), \
             updated_at = now()",
        )
        .bind(subscriber_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn recent_sessions(
        &self,
        account_id: i64,
        max_count: i64,
    ) -> anyhow::Result<Vec<AntiCheatSessionHistoryRow>> {
        let rows = sqlx::query_as::<_, (String, i32, i32, f32, i32, i32, i32, i32)>(
            "select to_char(started_at, 'MM-DD HH24:MI'), \
             (extract(epoch from (coalesce(ended_at, now()) - started_at)) / 60)::int, \
             status, bot_score, heartbeat_violations, state_violations, \
             challenge_failures, anomaly_count \
             from anticheat_sessions where account_id = $1 \
             order by started_at desc limit $2",
        )
        .bind(account_id)
        .bind(max_count)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    start_time,
                    duration_minutes,
                    status,
                    bot_score,
                    heartbeat_violations,
                    state_violations,
                    challenge_failures,
                    anomaly_count,
                )| AntiCheatSessionHistoryRow {
                    start_time,
                    duration_minutes,
                    status,
                    bot_score,
                    heartbeat_violations,
                    state_violations,
                    challenge_failures,
                    anomaly_count,
                },
            )
            .collect())
    }

    async fn recent_violations(
        &self,
        account_id: i64,
        max_count: i64,
    ) -> anyhow::Result<Vec<AntiCheatViolationRow>> {
        let rows = sqlx::query_as::<_, (String, String, i32, Option<String>)>(
            "select to_char(e.created_at, 'MM-DD HH24:MI'), e.event_type, e.severity, e.details \
             from anticheat_events e \
             join anticheat_sessions s on s.id = e.session_id \
             where s.account_id = $1 \
             order by e.created_at desc limit $2",
        )
        .bind(account_id)
        .bind(max_count)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(detected_at, type_name, severity, details)| AntiCheatViolationRow {
                    detected_at,
                    type_name,
                    severity,
                    details,
                },
            )
            .collect())
    }

    async fn list_signatures(&self, max_count: i64) -> anyhow::Result<Vec<AntiCheatSignatureRow>> {
        let rows = sqlx::query_as::<_, (i64, String, String, i32, bool, bool, i32, bool)>(
            "select id, signature_type, name, severity, auto_flag, auto_ban, times_detected, \
             is_active from ac_known_signatures order by times_detected desc limit $1",
        )
        .bind(max_count)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    signature_type,
                    name,
                    severity,
                    auto_flag,
                    auto_ban,
                    times_detected,
                    is_active,
                )| {
                    AntiCheatSignatureRow {
                        id,
                        signature_type,
                        name,
                        severity,
                        auto_flag,
                        auto_ban,
                        times_detected,
                        is_active,
                    }
                },
            )
            .collect())
    }

    async fn add_signature(
        &self,
        signature_type: &str,
        signature_value: &str,
        name: &str,
        created_by: &str,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "insert into ac_known_signatures (signature_type, signature_value, name, created_by) \
             values ($1, $2, $3, $4)",
        )
        .bind(signature_type)
        .bind(signature_value)
        .bind(name)
        .bind(created_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_signature(&self, signature_id: i64) -> anyhow::Result<bool> {
        let result = sqlx::query("delete from ac_known_signatures where id = $1")
            .bind(signature_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn update_player_stats(
        &self,
        subscriber_id: i64,
        session_bot_score: f32,
        session_status: i32,
        heartbeat_violations: i32,
        state_violations: i32,
        challenge_failures: i32,
        anomalies: i32,
    ) -> anyhow::Result<()> {
        let was_flagged: i32 = if session_status == 3 { 1 } else { 0 };
        let was_suspicious: i32 = if session_status == 2 { 1 } else { 0 };
        sqlx::query(
            "insert into ac_player_stats (\
             subscriber_id, total_sessions, flagged_sessions, suspicious_sessions, \
             total_heartbeat_violations, total_state_violations, total_challenge_failures, \
             total_anomalies, lifetime_bot_score, max_session_bot_score, \
             avg_session_bot_score, risk_level, last_seen, updated_at) \
             values ($1, 1, $3, $4, $5, $6, $7, $8, $2, $2, $2, \
             case \
                 when $2 >= 1.0 or $3 >= 3 then 'critical' \
                 when $2 >= 0.8 or $3 >= 1 then 'high' \
                 when $2 >= 0.5 or $4 >= 3 then 'medium' \
                 else 'low' \
             end, \
             now(), now()) \
             on conflict (subscriber_id) do update set \
             total_sessions = ac_player_stats.total_sessions + 1, \
             flagged_sessions = ac_player_stats.flagged_sessions + excluded.flagged_sessions, \
             suspicious_sessions = \
                 ac_player_stats.suspicious_sessions + excluded.suspicious_sessions, \
             total_heartbeat_violations = \
                 ac_player_stats.total_heartbeat_violations + excluded.total_heartbeat_violations, \
             total_state_violations = \
                 ac_player_stats.total_state_violations + excluded.total_state_violations, \
             total_challenge_failures = \
                 ac_player_stats.total_challenge_failures + excluded.total_challenge_failures, \
             total_anomalies = ac_player_stats.total_anomalies + excluded.total_anomalies, \
             lifetime_bot_score = ac_player_stats.lifetime_bot_score + excluded.lifetime_bot_score, \
             max_session_bot_score = \
                 greatest(ac_player_stats.max_session_bot_score, excluded.max_session_bot_score), \
             avg_session_bot_score = \
                 (ac_player_stats.lifetime_bot_score + excluded.lifetime_bot_score) \
                 / (ac_player_stats.total_sessions + 1), \
             risk_level = case \
                 when greatest(ac_player_stats.max_session_bot_score, \
                               excluded.max_session_bot_score) >= 1.0 \
                      or (ac_player_stats.flagged_sessions + excluded.flagged_sessions) >= 3 \
                 then 'critical' \
                 when greatest(ac_player_stats.max_session_bot_score, \
                               excluded.max_session_bot_score) >= 0.8 \
                      or (ac_player_stats.flagged_sessions + excluded.flagged_sessions) >= 1 \
                 then 'high' \
                 when greatest(ac_player_stats.max_session_bot_score, \
                               excluded.max_session_bot_score) >= 0.5 \
                      or (ac_player_stats.suspicious_sessions + excluded.suspicious_sessions) >= 3 \
                 then 'medium' \
                 else 'low' \
             end, \
             last_seen = now(), updated_at = now()",
        )
        .bind(subscriber_id)
        .bind(session_bot_score)
        .bind(was_flagged)
        .bind(was_suspicious)
        .bind(heartbeat_violations)
        .bind(state_violations)
        .bind(challenge_failures)
        .bind(anomalies)
        .execute(&self.pool)
        .await?;
        Ok(())
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
                let low: (i64,) =
                    sqlx::query_as("select id from ac_known_signatures where name = $1")
                        .bind("Sig Fixture Low")
                        .fetch_one(&pool)
                        .await
                        .expect("fetch low fixture id");
                let high: (i64,) =
                    sqlx::query_as("select id from ac_known_signatures where name = $1")
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
    }
}
