//! `#acstatus`, `#acstats`, `#aclist`, `#acsuspicious`, `#accleanup`
//! admin/staff text commands (C `command.c:10149-10192,10314-10319`
//! dispatch -> `ac_cmd_status`/`ac_cmd_stats`/`ac_cmd_list`/
//! `ac_cmd_suspicious`/`ac_cmd_cleanup`,
//! `src/module/anticheat/anticheat.c:473-543,604-628,721-780,1267-1285`),
//! all `CF_GOD|CF_STAFF`-gated except `#accleanup` (`CF_GOD`-only),
//! exact-word only (`cmdcmp`'s `minlen` equals each command's full
//! length). `#achelp` (a sixth member of this same slice) is pure static
//! text and needs no queue at all - it lives directly in
//! `commands_admin.rs`.
//!
//! C's `ac_cmd_*` family reads its data straight out of the in-memory
//! `player[nr]->ac` struct, kept live by the detection engine
//! (`anticheat_heartbeat.c`/`anticheat_state.c`/etc.) - a whole unported
//! subsystem (see `PORTING_TODO.md`'s "Remaining `/` and `#` text
//! commands" REMAINING note, gap (a)). This codebase has no equivalent
//! in-memory struct: the only place `bot_score`/violation counters live
//! is the `anticheat_sessions` Postgres row created at login
//! (`ugaris-server`'s `SessionEvent::Login` handling, wired in iteration
//! 196) and referenced by `PlayerRuntime::anticheat_session_id`. So,
//! like `/lastseen`, what's a synchronous struct read in C becomes an
//! async DB round trip here - except the name-to-session-id resolution
//! (C's `ac_find_player`, an online-only `CF_PLAYER` name scan) has to
//! happen in `ugaris-server::commands_admin` rather than in `World`
//! itself, since `World` has no visibility into `PlayerRuntime` (a
//! `ugaris-server`-owned type) - a real deviation from `/jail`'s/
//! `/rmdeath`'s "queue the raw name, resolve online-ness later" pattern,
//! forced by this being the first async-DB command whose *input* (a
//! session id) isn't itself queryable by name.
//!
//! Every session lookup that comes back empty (row deleted, or a
//! genuinely unknown id) is simply omitted from the reply, matching every
//! other offline-DB-lookup event in this codebase's silent-skip
//! convention - there is no "no data" message in the C original either,
//! since C's in-memory struct always exists once a connection does.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcStatusLookup {
    pub caller_id: CharacterId,
    pub target_name: String,
    pub session_id: i64,
}

/// One online `CF_PLAYER` character with a known anticheat session,
/// gathered synchronously by `commands_admin.rs` before queuing
/// `#acstats`/`#aclist` (see module doc comment).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcOnlineTarget {
    pub name: String,
    pub session_id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcListLookup {
    pub caller_id: CharacterId,
    pub targets: Vec<AcOnlineTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcStatsLookup {
    pub caller_id: CharacterId,
    pub targets: Vec<AcOnlineTarget>,
}

/// `#acsuspicious` (`ac_cmd_suspicious`, `anticheat.c:754-780`) - same
/// gather shape as `#aclist`/`#acstats` (every online `CF_PLAYER`
/// character with a known anticheat session), filtered down to
/// suspicious-or-worse status after the DB round trip returns each
/// session's current status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcSuspiciousLookup {
    pub caller_id: CharacterId,
    pub targets: Vec<AcOnlineTarget>,
}

/// `#accleanup <days>` (`ac_cmd_cleanup`, `anticheat.c:1267-1285`) - a
/// pure maintenance action with no name/session resolution at all (unlike
/// every other member of this module), so it needs no synchronous
/// pre-gather step in `commands_admin.rs` beyond parsing `days` itself.
/// C also deletes rows from a separate `ac_heartbeat_log` table
/// (`db_ac_cleanup_heartbeat_logs`); this codebase folds heartbeat
/// counters into `anticheat_sessions` itself (see that table's
/// `heartbeat_violations` column) rather than a standalone log table, so
/// there is nothing to delete there - `heartbeat_logs_deleted` is always
/// `0`, reported as such rather than omitted, matching C's own always-
/// present "%d heartbeat logs deleted" clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcCleanupLookup {
    pub caller_id: CharacterId,
    pub days: i32,
}

/// `#acreset <player>` (`ac_cmd_reset`, `anticheat.c:527-561`), `CF_GOD`-
/// only. Same single-name-target shape as `AcStatusLookup` (the online-
/// name-scan + `PlayerRuntime::anticheat_session_id` lookup happens
/// synchronously in `commands_admin.rs` before queuing, for the same
/// reason - see the module doc comment); the DB half is a mutation
/// (`AntiCheatRepository::reset_session`) rather than a read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcResetLookup {
    pub caller_id: CharacterId,
    pub target_name: String,
    pub session_id: i64,
}

impl World {
    pub fn queue_ac_status_lookup(
        &mut self,
        caller_id: CharacterId,
        target_name: String,
        session_id: i64,
    ) {
        self.pending_ac_status_lookups.push(AcStatusLookup {
            caller_id,
            target_name,
            session_id,
        });
    }

    pub fn drain_pending_ac_status_lookups(&mut self) -> Vec<AcStatusLookup> {
        self.pending_ac_status_lookups.drain(..).collect()
    }

    pub fn queue_ac_list_lookup(&mut self, caller_id: CharacterId, targets: Vec<AcOnlineTarget>) {
        self.pending_ac_list_lookups
            .push(AcListLookup { caller_id, targets });
    }

    pub fn drain_pending_ac_list_lookups(&mut self) -> Vec<AcListLookup> {
        self.pending_ac_list_lookups.drain(..).collect()
    }

    pub fn queue_ac_stats_lookup(&mut self, caller_id: CharacterId, targets: Vec<AcOnlineTarget>) {
        self.pending_ac_stats_lookups
            .push(AcStatsLookup { caller_id, targets });
    }

    pub fn drain_pending_ac_stats_lookups(&mut self) -> Vec<AcStatsLookup> {
        self.pending_ac_stats_lookups.drain(..).collect()
    }

    pub fn queue_ac_suspicious_lookup(
        &mut self,
        caller_id: CharacterId,
        targets: Vec<AcOnlineTarget>,
    ) {
        self.pending_ac_suspicious_lookups
            .push(AcSuspiciousLookup { caller_id, targets });
    }

    pub fn drain_pending_ac_suspicious_lookups(&mut self) -> Vec<AcSuspiciousLookup> {
        self.pending_ac_suspicious_lookups.drain(..).collect()
    }

    pub fn queue_ac_cleanup_lookup(&mut self, caller_id: CharacterId, days: i32) {
        self.pending_ac_cleanup_lookups
            .push(AcCleanupLookup { caller_id, days });
    }

    pub fn drain_pending_ac_cleanup_lookups(&mut self) -> Vec<AcCleanupLookup> {
        self.pending_ac_cleanup_lookups.drain(..).collect()
    }

    pub fn queue_ac_reset_lookup(
        &mut self,
        caller_id: CharacterId,
        target_name: String,
        session_id: i64,
    ) {
        self.pending_ac_reset_lookups.push(AcResetLookup {
            caller_id,
            target_name,
            session_id,
        });
    }

    pub fn drain_pending_ac_reset_lookups(&mut self) -> Vec<AcResetLookup> {
        self.pending_ac_reset_lookups.drain(..).collect()
    }
}

/// C `ac_status_string` (`anticheat.c:436-449`).
pub fn ac_status_string(status: i32) -> &'static str {
    match status {
        0 => "unverified",
        1 => "verified",
        2 => "suspicious",
        3 => "flagged",
        _ => "unknown",
    }
}

/// C `AC_STATUS_SUSPICIOUS` (`anticheat.h:84`) - the threshold
/// `ac_cmd_suspicious` (`anticheat.c:762`, `player[nr]->ac.status <
/// AC_STATUS_SUSPICIOUS`) filters against.
pub const AC_STATUS_SUSPICIOUS: i32 = 2;

/// C `AC_STATUS_VERIFIED` (`anticheat.h:83`) - the status `#acreset`
/// (`ac_cmd_reset`, `anticheat.c:557`) restores a session to.
pub const AC_STATUS_VERIFIED: i32 = 1;
