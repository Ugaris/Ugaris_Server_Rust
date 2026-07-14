use super::*;

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
    /// `#achistory <player>`'s backing query (`db_ac_get_player_stats`,
    /// `database_anticheat.c:829-880`): the subscriber's lifetime
    /// `ac_player_stats` row, or `None` when no row exists yet (C's own
    /// `mysql_fetch_row` returning null - a subscriber who has never
    /// triggered `update_player_stats`/`set_flagged`/`set_trusted`/
    /// `issue_warning`).
    async fn find_player_stats(
        &self,
        subscriber_id: i64,
    ) -> anyhow::Result<Option<AntiCheatPlayerStatsRow>>;
    /// `#acsharedip <player>`'s backing query (`db_ac_get_shared_ips`,
    /// `database_anticheat.c:963-999`): every other account that has
    /// logged in from one of `account_id`'s own IP addresses, newest
    /// shared session first - see `AntiCheatSharedIpRow`'s doc comment
    /// for why this is derived live from `anticheat_sessions` rather
    /// than a dedicated `ac_ip_history` table.
    async fn shared_ips(
        &self,
        account_id: i64,
        max_count: i64,
    ) -> anyhow::Result<Vec<AntiCheatSharedIpRow>>;
    /// `#acsharedhw <player>`'s backing query (`db_ac_get_shared_
    /// hardware`, `database_anticheat.c:1005-1040`): every other account
    /// that has logged in with one of `account_id`'s own hardware
    /// fingerprints, newest shared session first - see
    /// `AntiCheatSharedHwRow`'s doc comment for why this is derived live
    /// from `anticheat_sessions` rather than a dedicated `ac_hardware_
    /// history` table.
    async fn shared_hardware(
        &self,
        account_id: i64,
        max_count: i64,
    ) -> anyhow::Result<Vec<AntiCheatSharedHwRow>>;
    /// `#achighrisk`'s backing query (`db_ac_get_high_risk_players`,
    /// `database_anticheat.c:1048-1091`): every `ac_player_stats` row
    /// whose `risk_level` is `high`/`critical` or whose `is_flagged` is
    /// set, ordered by `max_session_bot_score` descending (matching C's
    /// own `ORDER BY`), capped at `max_count` (C's own `results[20]`
    /// stack array).
    async fn high_risk_players(&self, max_count: i64) -> anyhow::Result<Vec<AntiCheatHighRiskRow>>;
    /// `#aclookup <subscriber_id>`'s backing query (`db_ac_lookup_
    /// subscriber`, `database_anticheat.c:1093-1140`): `None` when no
    /// `accounts` row with that id exists at all (C's own `result.found
    /// == 0` branch); `Some` with a `None` `stats` field when the account
    /// exists but has never triggered any anticheat activity (C's own
    /// `LEFT JOIN ac_player_stats` producing a null-filled row) - see
    /// `AntiCheatSubscriberLookup`'s doc comment.
    async fn lookup_subscriber(
        &self,
        subscriber_id: i64,
    ) -> anyhow::Result<Option<AntiCheatSubscriberLookup>>;
}
