-- Mirrors legacy `achievement_firsts`/`achievement_history` tables
-- (src/system/database/database_achievement.c::db_achievement_record_unlock),
-- the "first player globally to unlock this achievement" tracker behind
-- `achievement_award`'s cross-server "Grats: NAME is the FIRST to unlock
-- ACH!" broadcast.
--
-- C keys both tables by `subscriber_id` (account-wide: the first unlock
-- across *any* of an account's characters). This codebase has no live
-- multi-character-per-account model in the running server yet - the exact
-- same scoping compromise `crates/ugaris-server/src/achievement.rs`'s
-- `DRD_ACHIEVEMENT_DATA`/`DRD_ACHIEVEMENT_STATS` persistence already
-- documents - so `character_id`/`character_name` stand in for
-- `subscriber_id`/the account's display name here.
--
-- C detects "was this insert the first one" via `mysql_affected_rows() ==
-- 1` from `INSERT ... ON DUPLICATE KEY UPDATE total_unlocks =
-- total_unlocks + 1`. Postgres's `ON CONFLICT DO UPDATE` has no equivalent
-- affected-rows signal, so the Rust repository instead uses the standard
-- `RETURNING (xmax = 0)` idiom (`xmax` is the deleting transaction id for
-- a row; it is `0` for a row inserted - not updated - by the current
-- command).
create table if not exists achievement_firsts (
    achievement_id smallint primary key,
    achievement_name text not null,
    first_character_id bigint not null references characters(id),
    first_character_name text not null,
    first_timestamp timestamptz not null default now(),
    total_unlocks bigint not null default 1
);

create table if not exists achievement_history (
    id bigserial primary key,
    achievement_id smallint not null,
    character_id bigint not null references characters(id),
    character_name text not null,
    unlocked_at timestamptz not null default now()
);

create index if not exists achievement_history_achievement_id_idx
    on achievement_history(achievement_id);
