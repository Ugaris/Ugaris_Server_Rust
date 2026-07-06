//! `#acstatus`, `#acstats`, `#aclist` admin/staff text commands (C
//! `command.c:10149-10192` dispatch -> `ac_cmd_status`/`ac_cmd_stats`/
//! `ac_cmd_list`, `src/module/anticheat/anticheat.c:473-543,604-628,
//! 721-753`), all `CF_GOD|CF_STAFF`-gated, exact-word only (`cmdcmp`'s
//! `minlen` equals each command's full length). `#achelp` (a fourth
//! member of this same slice) is pure static text and needs no queue at
//! all - it lives directly in `commands_admin.rs`.
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
