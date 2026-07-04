-- Restart-persistence for the clan identity/relation registry
-- (`crates/ugaris-core/src/clan.rs::ClanRegistry`), which ports the parts
-- of legacy `struct clan clan[MAXCLAN]` (src/system/clan.c:58) that exist
-- in Rust today: per-clan identity (name/rank names/website/message,
-- `clan.h:88-101` minus the unported treasury/dungeon economy) and the
-- full pairwise relation state machine (`struct clan_status`,
-- `clan.h:59-64`).
--
-- C persists the whole `clan[MAXCLAN]` array as part of a single
-- memory-image world save file, not as individual relational rows (the
-- only *table* C has for clans is `clanoverview`, a write-only mirror for
-- an external website - see `showclan_db`, `clan.c:97-126` - which is not
-- what this migration is for). `ClanRegistry` already derives
-- `Serialize`/`Deserialize` end-to-end (identities, serials, and the
-- relation matrices), so this mirrors C's "one blob" approach with a
-- single-row `jsonb` snapshot instead of inventing a relational schema
-- for data that has no natural per-row key in C.
create table if not exists clan_registry (
    id smallint primary key default 1,
    registry_json jsonb not null,
    updated_at timestamptz not null default now(),
    constraint clan_registry_singleton check (id = 1)
);
