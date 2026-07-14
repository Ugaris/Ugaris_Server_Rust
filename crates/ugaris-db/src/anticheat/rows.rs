use super::*;

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

/// `#achistory <player>`'s backing row shape (`db_ac_player_stats_
/// result`, `database_anticheat.h:426-452`). Unlike `AntiCheatSessionInfo`
/// (a single `anticheat_sessions` row), this reads the lifetime rollup
/// `ac_player_stats` row `update_player_stats` maintains - the same table
/// `set_flagged`/`set_trusted`/`issue_warning` upsert into, but this is
/// the first *read* of it. `first_seen`/`last_seen` are formatted as
/// `MM-DD HH:MI` text at the query layer (matching `recent_sessions`'s
/// own `to_char` convention) rather than returned as raw timestamps,
/// since no caller needs to do further date arithmetic with them.
#[derive(Debug, Clone, PartialEq)]
pub struct AntiCheatPlayerStatsRow {
    pub total_sessions: i32,
    pub flagged_sessions: i32,
    pub suspicious_sessions: i32,
    pub total_heartbeat_violations: i32,
    pub total_state_violations: i32,
    pub total_challenge_failures: i32,
    pub total_anomalies: i32,
    pub max_session_bot_score: f32,
    pub avg_session_bot_score: f32,
    pub risk_level: String,
    pub is_flagged: bool,
    pub is_trusted: bool,
    pub warnings_issued: i32,
    pub first_seen: String,
    pub last_seen: Option<String>,
}

/// `#acsharedip <player>`'s per-row shape (`db_ac_shared_ip_result`,
/// `database_anticheat.h:501-511`). C's own query reads from a dedicated
/// `ac_ip_history` aggregate table (one row per `(subscriber_id,
/// ip_address)` pair, incrementally maintained by a `db_ac_track_ip`
/// writer this codebase never ported); this codebase instead derives the
/// same shape live from `anticheat_sessions` (already every login's IP,
/// per `AntiCheatSessionHistoryRow`'s doc comment on reusing that table
/// over introducing new schema) by self-joining on `ip_address` and
/// grouping by the other account, matching `session_count`/`last_seen`
/// (`MAX(started_at)`) directly against what an incremental `ac_ip_
/// history` row would report. `email` has no equivalent column in this
/// codebase's `accounts` table (see this module's other `username`-for-
/// `email` substitutions); `accounts.username` - the same table/column
/// `#acstatus`'s sibling commands already treat as the Rust equivalent
/// of legacy `subscriber.ID`'s identity - stands in for it.
#[derive(Debug, Clone, PartialEq)]
pub struct AntiCheatSharedIpRow {
    pub username: String,
    pub ip_address: i32,
    pub session_count: i64,
    pub last_seen: String,
}

/// `#acsharedhw <player>`'s per-row shape (`db_ac_shared_hw_result`,
/// `database_anticheat.h:521-531`). Same live-derivation-over-a-
/// dedicated-history-table approach as `AntiCheatSharedIpRow` above, this
/// time self-joining `anticheat_sessions` on `hardware_hash` (already
/// captured per session by `set_fingerprint`) instead of reading a
/// `db_ac_track_hardware`-maintained `ac_hardware_history` row; grouped
/// by `(username, hardware_hash, screen_w, screen_h)` since a shared
/// hardware fingerprint's screen resolution can differ per session
/// (`ac_hardware_history` only ever stores its own history row's single
/// latest resolution, so this is a superset shape, not a narrower one).
#[derive(Debug, Clone, PartialEq)]
pub struct AntiCheatSharedHwRow {
    pub username: String,
    pub hardware_hash: i64,
    pub screen_w: Option<i32>,
    pub screen_h: Option<i32>,
    pub last_seen: String,
}

/// `#achighrisk`'s per-row shape (`db_ac_high_risk_result`,
/// `database_anticheat.h:541-552`). Reads the same `ac_player_stats`
/// table `#achistory`'s `AntiCheatPlayerStatsRow` reads, joined against
/// `accounts` for the `username`-for-`email` substitution (see
/// `AntiCheatSharedIpRow`'s doc comment); `total_anomalies`/`is_flagged`
/// are omitted since C's own format string (`ac_cmd_highrisk`,
/// `anticheat.c:1134-1157`) never displays them either (only
/// `is_flagged`'s effect on the `WHERE` filter matters, not its value).
#[derive(Debug, Clone, PartialEq)]
pub struct AntiCheatHighRiskRow {
    pub subscriber_id: i64,
    pub username: String,
    pub risk_level: String,
    pub max_bot_score: f32,
    pub flagged_sessions: i32,
    pub last_seen: Option<String>,
}

/// `#aclookup <subscriber_id>`'s backing query result (`db_ac_lookup_
/// subscriber`, `database_anticheat.c:1093-1140`). C's own `result.
/// found` flag (0 when `subscriber.ID` itself doesn't exist) is folded
/// into the outer `Option` (`None` = no such account); the inner
/// `stats: Option<AntiCheatPlayerStatsRow>` mirrors C's own `result.
/// total_sessions > 0` branch (a real account whose `ac_player_stats`
/// row - a `LEFT JOIN` in C's query - doesn't exist yet because it has
/// never triggered any anticheat activity).
#[derive(Debug, Clone, PartialEq)]
pub struct AntiCheatSubscriberLookup {
    pub username: String,
    pub stats: Option<AntiCheatPlayerStatsRow>,
}
