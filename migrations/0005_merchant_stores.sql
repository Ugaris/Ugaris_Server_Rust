-- Mirrors legacy `merchant_items`/`merchant_gold` tables
-- (src/system/database/database_merchant.c): persists merchant store
-- inventory and gold so a server restart does not reset stock the way an
-- in-memory-only store would. C keys rows by (merchant_name, merchant_x,
-- merchant_y) and stores each ware in its own row with `drdata`/modifiers
-- JSON-encoded by hand; Rust instead stores the merchant's whole ware list
-- (item + count + always flag) as a single `jsonb` array per merchant,
-- since the `Item` struct already round-trips through serde JSON elsewhere
-- (see `characters.character_json` / `character_items.item_json`).
create table if not exists merchant_stores (
    merchant_name text not null,
    merchant_x integer not null,
    merchant_y integer not null,
    gold bigint not null default 0,
    price_multi integer not null default 400,
    wares_json jsonb not null default '[]'::jsonb,
    updated_at timestamptz not null default now(),
    primary key (merchant_name, merchant_x, merchant_y)
);
