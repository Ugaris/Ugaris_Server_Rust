-- Extends `ac_player_stats` (see `0014_ac_player_stats.sql`'s doc comment
-- for why this table is deliberately scoped down column-by-column rather
-- than porting C's whole session-rollup schema at once) with the two
-- columns `#acwarn <player> [reason]` (`ac_cmd_warn`/`db_ac_issue_warning`,
-- `src/module/anticheat/anticheat.c:1291-1314` /
-- `src/system/database/database_anticheat.c:606-621`) reads/writes:
-- `warnings_issued` (incremented by one per warning) and `last_warning_at`
-- (stamped to the warning's timestamp). A future iteration porting
-- `#achistory`/`#acsessions`/etc. should keep extending this same table
-- rather than replacing it.
alter table ac_player_stats
    add column if not exists warnings_issued integer not null default 0;
alter table ac_player_stats
    add column if not exists last_warning_at timestamptz;
