-- Extends `ac_player_stats` (see `0014_ac_player_stats.sql`'s doc comment
-- for why this table is deliberately grown column-by-column) with the
-- `first_seen` column `#achistory`'s backing query reads
-- (`db_ac_get_player_stats`, `src/system/database/database_anticheat.c:
-- 829-880`). `0017_ac_player_stats_rollup.sql`'s doc comment explicitly
-- deferred this column to "a future iteration porting that half" - this
-- is that iteration. Defaults to `now()` so a row created by
-- `update_player_stats`'s insert branch always has a value; the same
-- method's `on conflict ... do update` clause must NOT touch this
-- column, matching C's own `db_ac_ensure_player_stats` "set once, never
-- overwritten" semantics for the equivalent field.
alter table ac_player_stats
    add column if not exists first_seen timestamptz not null default now();
