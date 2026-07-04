-- Restart-persistence for the Military Master NPC-scoped storage blobs
-- (`crates/ugaris-core/src/world/military.rs::MilitaryMasterStorageRegistry`),
-- which ports the counters legacy `struct military_master_storage`
-- (`src/module/military.c:346-352`) persists through the generic
-- `storage` table (`create_storage`/`read_storage`/`update_storage`,
-- `src/system/database/database_storage.c`).
--
-- C's `storage` table is a single generic byte-blob-per-id mechanism
-- shared by many unrelated NPC drivers; Rust instead gives each
-- consumer its own typed table, following the "Military ranks" task's
-- own researched recommendation (see `MilitaryMasterStorageRegistry`'s
-- doc comment) - a small typed-struct-per-consumer table keyed per
-- storage id, since these aren't singletons (unlike `clan_registry`,
-- which has exactly one row for the whole server, every Military Master
-- NPC in the world has its own `storage=N;` zone-file id and thus its
-- own row here).
create table if not exists military_master_storage (
    storage_id integer primary key,
    storage_json jsonb not null,
    updated_at timestamptz not null default now()
);
