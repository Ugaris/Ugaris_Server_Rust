-- Mirrors the legacy `ac_player_stats` table (`src/system/database/
-- database_anticheat.c`, keyed by `subscriber_id` - C's
-- `get_subscriberId_from_character`/`chars.sID`, the owning account,
-- exactly equivalent to this codebase's existing `characters.account_id`
-- column, so no new "subscriber id" concept is introduced here).
--
-- C's real `ac_player_stats` table also carries a whole session-rollup
-- history (`total_sessions`, `flagged_sessions`, `lifetime_bot_score`,
-- `risk_level`, fingerprint-on-login columns, etc. - see
-- `database_anticheat.c:482-524,526-550`), populated by the unported
-- detection engine and read by the still-unported `#achistory`/
-- `#acsessions`/`#acviolations`/`#acsharedip`/`#acsharedhw`/
-- `#achighrisk`/`#aclookup` admin commands. This migration deliberately
-- only ports the two columns `#acunflag`/`#actrust`/`#acuntrust` (the
-- current slice) actually read or write - `is_flagged`/`is_trusted` -
-- rather than inventing placeholder columns for a rollup engine that
-- doesn't exist yet; a future iteration porting the aggregate-query
-- commands should `ALTER TABLE` this same table rather than replacing
-- it, matching how `anticheat_sessions` itself was scoped down in
-- iteration 196.
create table if not exists ac_player_stats (
    subscriber_id bigint primary key references accounts(id) on delete cascade,
    is_flagged boolean not null default false,
    is_trusted boolean not null default false,
    updated_at timestamptz not null default now()
);
