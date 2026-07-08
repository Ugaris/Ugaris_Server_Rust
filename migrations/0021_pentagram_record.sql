-- Restart-persistence for the pentagram-quest lifetime "most pentagrams
-- activated in one run" record (`static int pentagram_record`/
-- `pentagram_record_ID`/`pentagram_record_holder`, `src/area/4/pents.c:
-- 92-93`, ported as `crates/ugaris-core/src/world/pents.rs::
-- PentagramQuestState::pentagram_record`/`pentagram_record_holder`).
--
-- Unlike every other repository in this crate, C already has a
-- dedicated table for this exact state
-- (`src/system/database/database_pent_record.c:33-41`), so this mirrors
-- its shape one-for-one: one row per `area_id`, upserted by
-- `save_pentagram_record`/loaded once at startup by
-- `load_pentagram_record`. `char_id` is always `0` here - this Rust
-- port's in-memory `CharacterId` is a per-session allocation, not a
-- persistent save-file identity like C's `ch[player_id].ID` (see
-- `PentagramQuestState::pentagram_record`'s doc comment), so record
-- ownership is tracked by `char_name` only, matching
-- `World::arena_toplist`'s own name-keyed precedent.
create table if not exists pentagram_record (
    area_id integer primary key,
    char_id integer not null default 0,
    char_name varchar(40) not null,
    record_count integer not null,
    record_date timestamptz not null default now()
);
