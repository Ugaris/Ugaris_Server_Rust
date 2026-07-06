//! `#acstatus`, `#acstats`, `#aclist`, `#acsuspicious`, `#accleanup`,
//! `#acreset`, `#acflag` admin/staff text commands (C
//! `command.c:10149-10192,10314-10319` dispatch -> `ac_cmd_status`/
//! `ac_cmd_stats`/`ac_cmd_list`/`ac_cmd_suspicious`/`ac_cmd_cleanup`/
//! `ac_cmd_reset`/`ac_cmd_flag`,
//! `src/module/anticheat/anticheat.c:473-593,604-628,721-780,1267-1285`),
//! all `CF_GOD|CF_STAFF`-gated except `#accleanup`/`#acreset`
//! (`CF_GOD`-only), exact-word only (`cmdcmp`'s `minlen` equals each
//! command's full length). `#achelp` (an extra member of this same
//! slice) is pure static text and needs no queue at all - it lives
//! directly in `commands_admin.rs`.
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

/// `#acflag <player>` (`ac_cmd_flag`, `anticheat.c:568-593`),
/// `CF_GOD|CF_STAFF`-gated (unlike `#acreset`'s `CF_GOD`-only). Same
/// single-name-target shape as `AcResetLookup`; the DB half sets
/// `status` to `AC_STATUS_FLAGGED` (`AntiCheatRepository::set_status`)
/// rather than resetting counters. C's own mutation is a plain
/// in-memory `player[nr]->ac.status = AC_STATUS_FLAGGED` assignment with
/// no DB write at all (unlike its `#acunflag` sibling, which does write
/// through `db_ac_session_set_status`/`db_ac_flag_player`/
/// `db_ac_log_admin_action`) - this codebase has no in-memory struct to
/// mutate, so the always-present `anticheat_sessions.status` column
/// stands in via the same `set_status` mutation `#acstatus` already
/// reads back from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcFlagLookup {
    pub caller_id: CharacterId,
    pub target_name: String,
    pub session_id: i64,
}

/// `#acunflag <player>` (`ac_cmd_unflag`, `anticheat.c:790-823`),
/// `CF_GOD`-only (unlike `#acflag`'s `CF_GOD|CF_STAFF`). Same
/// single-name-target shape as `AcFlagLookup`, but - unlike every other
/// member of this family - C's own handler gates on the target's
/// *current* status (`!= AC_STATUS_FLAGGED` -> "is not flagged", a
/// synchronous in-memory read in C) before mutating anything, so that
/// check has to happen inside `apply_ac_unflag_events` after the async
/// `find_session` round trip rather than in `commands_admin.rs`
/// alongside the online-name-scan (which only knows the session id
/// exists, not its status - see the module doc comment).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcUnflagLookup {
    pub caller_id: CharacterId,
    pub target_name: String,
    pub session_id: i64,
}

/// `#actrust <player>` (`ac_cmd_trust`, `anticheat.c:827-849`),
/// `CF_GOD`-only. Same single-name-target shape as `AcFlagLookup`, no
/// status gate (C's own handler has none - it unconditionally trusts
/// once a connection is found). The DB half
/// (`AntiCheatRepository::set_trusted`) needs the target's subscriber id
/// (`account_id`), resolved from `session_id` via `AntiCheatRepository::
/// account_id_for_session` inside `apply_ac_trust_events` rather than
/// threaded through `PlayerRuntime` - see that repository method's doc
/// comment for why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcTrustLookup {
    pub caller_id: CharacterId,
    pub target_name: String,
    pub session_id: i64,
}

/// `#acuntrust <player>` (`ac_cmd_untrust`, `anticheat.c:860-882`),
/// `CF_GOD`-only. Identical shape to `AcTrustLookup` (the "untrust" half
/// of the same `is_trusted` flag, `AntiCheatRepository::set_trusted`
/// called with `false` instead of `true`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcUntrustLookup {
    pub caller_id: CharacterId,
    pub target_name: String,
    pub session_id: i64,
}

/// `#acwarn <player> [reason]` (`ac_cmd_warn`, `anticheat.c:1291-1314`),
/// `CF_GOD|CF_STAFF`-gated, exact-word (`cmdcmp(ptr, "acwarn", 6)`).
/// Same single-name-target resolution as `#acflag`/`#actrust` above, but
/// unlike every other member of this family also carries the resolved
/// `target_id` (not just `target_name`) - the two-line warning message
/// (`log_char(co, ...)`) has to reach the *target* character directly,
/// something no sibling command needs (they only ever message the
/// caller). `reason` defaults to `"Anti-cheat warning"` when omitted,
/// matching C's `sscanf(args, "%39s %255[^\n]", target, reason)` pattern,
/// where `reason`'s buffer is pre-seeded with that exact default text
/// before the `sscanf` call, so a missing second token leaves it
/// untouched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcWarnLookup {
    pub caller_id: CharacterId,
    pub target_id: CharacterId,
    pub target_name: String,
    pub session_id: i64,
    pub reason: String,
}

/// `#acsessions <player>` (`ac_cmd_sessions`, `anticheat.c:975-1017`),
/// `CF_GOD|CF_STAFF`-gated, exact-word (`cmdcmp(ptr, "acsessions", 10)`).
/// Same single-name-target resolution as `#actrust`/`#acuntrust` (the
/// caller resolves `session_id` synchronously via the online-name-scan;
/// `apply_ac_sessions_events` resolves the subscriber's `account_id` from
/// it via `account_id_for_session` before querying the full session
/// history), not just the current session - see `ugaris-db`'s
/// `AntiCheatSessionHistoryRow` doc comment for why no new table is
/// needed for the history query itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcSessionsLookup {
    pub caller_id: CharacterId,
    pub target_name: String,
    pub session_id: i64,
}

/// `#acviolations <player>` (`ac_cmd_violations`, `anticheat.c:1019-
/// 1053`), `CF_GOD|CF_STAFF`-gated, exact-word (`cmdcmp(ptr,
/// "acviolations", 12)`). Same single-name-target resolution as
/// `#acsessions` above (the caller resolves `session_id` synchronously
/// via the online-name-scan; `apply_ac_violations_events` resolves the
/// subscriber's `account_id` from it via `account_id_for_session`
/// before querying the violation history), not just the current
/// session - see `ugaris-db`'s `AntiCheatViolationRow` doc comment for
/// why no new table is needed for the history query itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcViolationsLookup {
    pub caller_id: CharacterId,
    pub target_name: String,
    pub session_id: i64,
}

/// `#achistory <player>` (`ac_cmd_history`, `anticheat.c:924-972`),
/// `CF_GOD|CF_STAFF`-gated, exact-word (`cmdcmp(ptr, "achistory", 9)`).
/// Same single-name-target resolution as `#acsessions`/`#acviolations`
/// above (the caller resolves `session_id` synchronously via the
/// online-name-scan; `apply_ac_history_events` resolves the subscriber's
/// `account_id` from it via `account_id_for_session` before reading the
/// lifetime `ac_player_stats` row), but unlike its two siblings this
/// reads a single rollup row (`AntiCheatRepository::find_player_stats`)
/// rather than a per-event history list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcHistoryLookup {
    pub caller_id: CharacterId,
    pub target_name: String,
    pub session_id: i64,
}

/// `#acsiglist` (`ac_cmd_siglist`, `anticheat.c:1192-1215`), `CF_GOD`-
/// only, exact-word (`cmdcmp(ptr, "acsiglist", 9)`). No player name to
/// resolve - a pure "list every row in the new `ac_known_signatures`
/// table" async DB round trip, same no-target shape as
/// `AcCleanupLookup`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcSiglistLookup {
    pub caller_id: CharacterId,
}

/// `#acsigadd <type> <value> <name>` (`ac_cmd_sigadd`, `anticheat.c:
/// 1216-1245`), `CF_GOD`-only, exact-word. `sig_type`/`sig_value`/`name`
/// are parsed and validated entirely synchronously in
/// `commands_admin.rs` (C's own `sscanf(args, "%31s %255s %63[^\n]", ...)`
/// three-token parse plus the fixed `sig_type` allow-list check, both
/// pure string logic with no DB dependency - see `apply_ac_sigadd_
/// events` for the async insert itself). `created_by` is the caller's
/// own name (C's `ch[cn].name`), captured here alongside `caller_id`
/// since the DB write needs the *name* string, not just the character
/// id `queue_system_text` addresses replies to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcSigaddLookup {
    pub caller_id: CharacterId,
    pub sig_type: String,
    pub sig_value: String,
    pub name: String,
    pub created_by: String,
}

/// `#acsigdel <id>` (`ac_cmd_sigdel`, `anticheat.c:1246-1266`), `CF_GOD`-
/// only, exact-word. `signature_id` is parsed and range-checked
/// synchronously in `commands_admin.rs` (C's own `atoi` + `== 0` invalid-
/// id check).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcSigdelLookup {
    pub caller_id: CharacterId,
    pub signature_id: i64,
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

    pub fn queue_ac_flag_lookup(
        &mut self,
        caller_id: CharacterId,
        target_name: String,
        session_id: i64,
    ) {
        self.pending_ac_flag_lookups.push(AcFlagLookup {
            caller_id,
            target_name,
            session_id,
        });
    }

    pub fn drain_pending_ac_flag_lookups(&mut self) -> Vec<AcFlagLookup> {
        self.pending_ac_flag_lookups.drain(..).collect()
    }

    pub fn queue_ac_unflag_lookup(
        &mut self,
        caller_id: CharacterId,
        target_name: String,
        session_id: i64,
    ) {
        self.pending_ac_unflag_lookups.push(AcUnflagLookup {
            caller_id,
            target_name,
            session_id,
        });
    }

    pub fn drain_pending_ac_unflag_lookups(&mut self) -> Vec<AcUnflagLookup> {
        self.pending_ac_unflag_lookups.drain(..).collect()
    }

    pub fn queue_ac_trust_lookup(
        &mut self,
        caller_id: CharacterId,
        target_name: String,
        session_id: i64,
    ) {
        self.pending_ac_trust_lookups.push(AcTrustLookup {
            caller_id,
            target_name,
            session_id,
        });
    }

    pub fn drain_pending_ac_trust_lookups(&mut self) -> Vec<AcTrustLookup> {
        self.pending_ac_trust_lookups.drain(..).collect()
    }

    pub fn queue_ac_untrust_lookup(
        &mut self,
        caller_id: CharacterId,
        target_name: String,
        session_id: i64,
    ) {
        self.pending_ac_untrust_lookups.push(AcUntrustLookup {
            caller_id,
            target_name,
            session_id,
        });
    }

    pub fn drain_pending_ac_untrust_lookups(&mut self) -> Vec<AcUntrustLookup> {
        self.pending_ac_untrust_lookups.drain(..).collect()
    }

    pub fn queue_ac_warn_lookup(
        &mut self,
        caller_id: CharacterId,
        target_id: CharacterId,
        target_name: String,
        session_id: i64,
        reason: String,
    ) {
        self.pending_ac_warn_lookups.push(AcWarnLookup {
            caller_id,
            target_id,
            target_name,
            session_id,
            reason,
        });
    }

    pub fn drain_pending_ac_warn_lookups(&mut self) -> Vec<AcWarnLookup> {
        self.pending_ac_warn_lookups.drain(..).collect()
    }

    pub fn queue_ac_sessions_lookup(
        &mut self,
        caller_id: CharacterId,
        target_name: String,
        session_id: i64,
    ) {
        self.pending_ac_sessions_lookups.push(AcSessionsLookup {
            caller_id,
            target_name,
            session_id,
        });
    }

    pub fn drain_pending_ac_sessions_lookups(&mut self) -> Vec<AcSessionsLookup> {
        self.pending_ac_sessions_lookups.drain(..).collect()
    }

    pub fn queue_ac_violations_lookup(
        &mut self,
        caller_id: CharacterId,
        target_name: String,
        session_id: i64,
    ) {
        self.pending_ac_violations_lookups.push(AcViolationsLookup {
            caller_id,
            target_name,
            session_id,
        });
    }

    pub fn drain_pending_ac_violations_lookups(&mut self) -> Vec<AcViolationsLookup> {
        self.pending_ac_violations_lookups.drain(..).collect()
    }

    pub fn queue_ac_history_lookup(
        &mut self,
        caller_id: CharacterId,
        target_name: String,
        session_id: i64,
    ) {
        self.pending_ac_history_lookups.push(AcHistoryLookup {
            caller_id,
            target_name,
            session_id,
        });
    }

    pub fn drain_pending_ac_history_lookups(&mut self) -> Vec<AcHistoryLookup> {
        self.pending_ac_history_lookups.drain(..).collect()
    }

    pub fn queue_ac_siglist_lookup(&mut self, caller_id: CharacterId) {
        self.pending_ac_siglist_lookups
            .push(AcSiglistLookup { caller_id });
    }

    pub fn drain_pending_ac_siglist_lookups(&mut self) -> Vec<AcSiglistLookup> {
        self.pending_ac_siglist_lookups.drain(..).collect()
    }

    pub fn queue_ac_sigadd_lookup(
        &mut self,
        caller_id: CharacterId,
        sig_type: String,
        sig_value: String,
        name: String,
        created_by: String,
    ) {
        self.pending_ac_sigadd_lookups.push(AcSigaddLookup {
            caller_id,
            sig_type,
            sig_value,
            name,
            created_by,
        });
    }

    pub fn drain_pending_ac_sigadd_lookups(&mut self) -> Vec<AcSigaddLookup> {
        self.pending_ac_sigadd_lookups.drain(..).collect()
    }

    pub fn queue_ac_sigdel_lookup(&mut self, caller_id: CharacterId, signature_id: i64) {
        self.pending_ac_sigdel_lookups.push(AcSigdelLookup {
            caller_id,
            signature_id,
        });
    }

    pub fn drain_pending_ac_sigdel_lookups(&mut self) -> Vec<AcSigdelLookup> {
        self.pending_ac_sigdel_lookups.drain(..).collect()
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

/// C `AC_STATUS_FLAGGED` (`anticheat.h:85`) - the status `#acflag`
/// (`ac_cmd_flag`, `anticheat.c:588`) sets a session to.
pub const AC_STATUS_FLAGGED: i32 = 3;
