//! `#querystats` / `/querystats` admin text command (C `command.c:
//! 6588-6618`, `cmdcmp(ptr, "querystats", 5)`, `CF_GOD`-gated).
//!
//! C reads a dozen-plus process-global counters (`query_cnt`,
//! `query_long`, `query_long_max`, `query_time`, `save_char_cnt`,
//! `save_area_cnt`, `exit_char_cnt`, `save_storage_cnt`,
//! `save_subscriber_cnt`, `save_char_mirror_cnt`, `load_char_cnt`, and
//! the 20-entry `query_stat[]` array), incremented at 6+ call sites
//! spread across `src/system/database/database*.c` with no single choke
//! point (see `PORTING_TODO.md`'s "Remaining `/` and `#` text commands"
//! task, iteration 201's REMAINING note). Porting the full set means
//! designing a new cross-cutting counters mechanism threaded through
//! every `ugaris-db` repository method first - out of scope for one
//! slice.
//!
//! This module ports a deliberately scoped-down subset: the three
//! `CharacterRepository`-choke-point counters iteration 201 called out as
//! the quick win - `save_char_cnt`/`exit_char_cnt`/`load_char_cnt` (C
//! `database_character.c:221,243,1102`, all three incremented inside
//! `save_char`/`load_char`, which map directly onto
//! `ugaris_db::PgCharacterRepository::save_character_snapshot`/
//! `begin_login` - see that struct's own `query_stats` doc comment for
//! exactly where each counter is incremented). `query_cnt`/`query_long`/
//! `query_long_max`/`query_time`/`save_area_cnt`/`save_storage_cnt`/
//! `save_subscriber_cnt`/`save_char_mirror_cnt`/`query_stat[]` remain
//! unported - `ugaris-db` has no instrumentation for any of them - so
//! this port's reply omits the "Total queries"/"Average query time"/
//! "Other operations"/"Query type statistics" lines entirely rather than
//! faking zeroes for counters nothing increments.
//!
//! Like `/lastseen`/`#acstatus`, `World` has no DB handle, so the command
//! becomes a queue-then-drain async round trip: `queue_querystats_lookup`
//! here, resolved by `ugaris-server`'s `world_events.rs::
//! apply_querystats_events` against the live `PgCharacterRepository`
//! (a synchronous in-memory atomic read, not actually a query, but routed
//! through the same tick-loop drain as every other DB-backed command for
//! architectural consistency).
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueryStatsLookup {
    pub caller_id: CharacterId,
}

impl World {
    pub fn queue_querystats_lookup(&mut self, caller_id: CharacterId) {
        self.pending_querystats_lookups
            .push(QueryStatsLookup { caller_id });
    }

    pub fn drain_pending_querystats_lookups(&mut self) -> Vec<QueryStatsLookup> {
        self.pending_querystats_lookups.drain(..).collect()
    }
}
