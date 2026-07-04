-- Mirrors the legacy `clanlog` table (`src/system/database/
-- database_notes.c::add_clanlog`/`lookup_clanlog`, `src/system/
-- clanlog.c::cmd_clanlog`), an append-only activity log for clan events
-- (founding, membership changes, relation transitions, raids, rank/
-- website/message edits, ...) that clan members and staff can filter and
-- read back with `/clanlog`.
--
-- C's `INSERT INTO clanlog VALUES(0,%d,%d,%d,%d,%d,'%s')` binds
-- `(time, clanNr, serial, cID, prio, content)` after the auto id; `cID`
-- (the acting character's legacy ID) is `0` for system-generated entries
-- (e.g. the daily relation-tick transitions log with `cID=0`,
-- `clan.c:983` etc.), so `character_id` intentionally has no foreign key
-- to `characters(id)` (unlike `achievement_history`, which never logs a
-- system-only row).
create table if not exists clan_log (
    id bigserial primary key,
    created_at bigint not null,
    clan smallint not null,
    serial bigint not null,
    character_id bigint not null,
    prio smallint not null,
    content text not null
);

create index if not exists clan_log_clan_time_idx
    on clan_log(clan, created_at);
