-- Restart-persistence for the Military Advisor NPC-scoped storage blobs
-- (`crates/ugaris-core/src/world/military.rs::MilitaryAdvisorStorageRegistry`),
-- which ports the sales-economy counters legacy
-- `struct military_advisor_data`'s `struct cost_data storage_data[5]`
-- (`src/module/military.c:374`) persists through the generic `storage`
-- table (`create_storage`/`read_storage`/`update_storage`,
-- `src/system/database/database_storage.c`).
--
-- Mirrors `0010_military_master_storage.sql`'s own table exactly (one
-- typed-struct-per-consumer table keyed per storage id, since every
-- Military Advisor NPC in the world has its own `storage=N;` zone-file
-- id and thus its own row here, same as the Master NPC's table).
create table if not exists military_advisor_storage (
    storage_id integer primary key,
    storage_json jsonb not null,
    updated_at timestamptz not null default now()
);
