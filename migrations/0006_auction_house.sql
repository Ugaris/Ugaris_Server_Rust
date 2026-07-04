-- Mirrors legacy `auction_items`/`auction_deliveries` tables
-- (src/system/auction/auction_db.c::init_auction_database). C stores each
-- auctioned item as a raw `struct item` BLOB and later `CAST(SUBSTRING(...))`
-- to filter/sort on fields inside it; Rust instead stores the item as a
-- `jsonb` document (same convention as `merchant_stores.wares_json`) and
-- filters/sorts on its `name`/`min_level`/`max_level` keys directly, which
-- is both simpler and safer than parsing raw struct offsets out of a blob.
-- `item_template` is kept as its own column (like C) purely so callers can
-- index/browse by template without touching the JSON body.
--
-- C's `status` is a MySQL `ENUM('active','sold','expired','cancelled')`;
-- kept as `text` + a `check` constraint here since Postgres enums are more
-- painful to migrate later.
--
-- C's `created_at`/`ends_at` are SQL `TIMESTAMP` columns read back through
-- a hand-rolled `sscanf` timestamp parser (`parse_mysql_timestamp`); Rust
-- keeps them as `timestamptz` and reads/writes them as unix-epoch seconds
-- via `extract(epoch from ...)`/`to_timestamp(...)`, the same convention
-- already used for `characters.login_time` in `character.rs`.
create table if not exists auctions (
    id bigserial primary key,
    seller_id bigint not null references characters(id),
    item_template bigint not null default 0,
    item_json jsonb not null,
    start_price bigint not null,
    buyout_price bigint,
    current_bid bigint,
    current_bidder_id bigint references characters(id),
    created_at timestamptz not null default now(),
    ends_at timestamptz not null,
    status text not null default 'active'
        check (status in ('active', 'sold', 'expired', 'cancelled'))
);

create index if not exists auctions_seller_id_idx on auctions(seller_id);
create index if not exists auctions_status_ends_at_idx on auctions(status, ends_at);
create index if not exists auctions_current_bidder_idx on auctions(current_bidder_id);

create table if not exists auction_deliveries (
    id bigserial primary key,
    character_id bigint not null references characters(id),
    item_json jsonb,
    gold_amount bigint not null default 0,
    reason text not null
        check (reason in ('won', 'expired', 'cancelled', 'sold', 'outbid')),
    created_at timestamptz not null default now(),
    claimed_at timestamptz
);

create index if not exists auction_deliveries_character_unclaimed_idx
    on auction_deliveries(character_id, claimed_at);
