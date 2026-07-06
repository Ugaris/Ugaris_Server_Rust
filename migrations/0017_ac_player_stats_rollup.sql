-- Extends `ac_player_stats` (see `0014_ac_player_stats.sql`'s doc comment
-- for why this table is deliberately grown column-by-column rather than
-- porting C's whole `ac_player_stats` schema at once) with the columns
-- C's `db_ac_update_player_stats` (`src/system/database/database_
-- anticheat.c:480-517`) actually reads/writes: the lifetime session-
-- rollup counters and the derived `risk_level` classification, wired at
-- disconnect (`ac_player_disconnect`, `src/module/anticheat/anticheat.c:
-- 141-163`) right after the per-session `anticheat_sessions` row is
-- closed out. `first_seen`/`last_mod_version_*`/`last_ip_address` are
-- deliberately NOT added here - those belong to the sibling
-- `db_ac_update_player_fingerprint` mutation, a separate still-unported
-- call the same C function makes only when a fingerprint was actually
-- received; a future iteration porting that half should extend this same
-- table again rather than replacing it.
alter table ac_player_stats
    add column if not exists total_sessions integer not null default 0;
alter table ac_player_stats
    add column if not exists flagged_sessions integer not null default 0;
alter table ac_player_stats
    add column if not exists suspicious_sessions integer not null default 0;
alter table ac_player_stats
    add column if not exists total_heartbeat_violations integer not null default 0;
alter table ac_player_stats
    add column if not exists total_state_violations integer not null default 0;
alter table ac_player_stats
    add column if not exists total_challenge_failures integer not null default 0;
alter table ac_player_stats
    add column if not exists total_anomalies integer not null default 0;
alter table ac_player_stats
    add column if not exists lifetime_bot_score real not null default 0;
alter table ac_player_stats
    add column if not exists max_session_bot_score real not null default 0;
alter table ac_player_stats
    add column if not exists avg_session_bot_score real not null default 0;
alter table ac_player_stats
    add column if not exists risk_level text not null default 'low';
alter table ac_player_stats
    add column if not exists last_seen timestamptz;
