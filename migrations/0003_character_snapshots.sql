alter table characters
    add column if not exists description text not null default '',
    add column if not exists flags_bits text not null default '0',
    add column if not exists speed_mode integer not null default 0,
    add column if not exists x integer not null default 0,
    add column if not exists y integer not null default 0,
    add column if not exists rest_area integer not null default 0,
    add column if not exists rest_x integer not null default 0,
    add column if not exists rest_y integer not null default 0,
    add column if not exists tox integer not null default 0,
    add column if not exists toy integer not null default 0,
    add column if not exists dir integer not null default 0,
    add column if not exists action integer not null default 0,
    add column if not exists duration integer not null default 0,
    add column if not exists step integer not null default 0,
    add column if not exists act1 integer not null default 0,
    add column if not exists act2 integer not null default 0,
    add column if not exists hp integer not null default 0,
    add column if not exists mana integer not null default 0,
    add column if not exists endurance integer not null default 0,
    add column if not exists lifeshield integer not null default 0,
    add column if not exists exp bigint not null default 0,
    add column if not exists exp_used bigint not null default 0,
    add column if not exists cursor_item_id bigint,
    add column if not exists current_container_item_id bigint,
    add column if not exists values_json jsonb not null default '[]'::jsonb,
    add column if not exists professions_json jsonb not null default '[]'::jsonb,
    add column if not exists inventory_json jsonb not null default '[]'::jsonb,
    add column if not exists character_json jsonb,
    add column if not exists logout_time timestamptz;

create table if not exists character_items (
    character_id bigint not null references characters(id) on delete cascade,
    item_id bigint not null,
    inventory_slot integer,
    is_cursor boolean not null default false,
    item_json jsonb not null,
    name text not null,
    description text not null,
    flags_bits text not null default '0',
    sprite integer not null default 0,
    item_value bigint not null default 0,
    min_level integer not null default 0,
    max_level integer not null default 0,
    needs_class integer not null default 0,
    owner_id integer not null default 0,
    modifier_index smallint[] not null default '{}',
    modifier_value smallint[] not null default '{}',
    x integer not null default 0,
    y integer not null default 0,
    carried_by bigint,
    contained_in bigint,
    content_id integer not null default 0,
    driver integer not null default 0,
    driver_data bytea not null default ''::bytea,
    serial bigint not null default 0,
    updated_at timestamptz not null default now(),
    primary key (character_id, item_id),
    constraint character_items_inventory_slot_range check (inventory_slot is null or inventory_slot between 0 and 109)
);

create unique index if not exists character_items_slot_idx
    on character_items(character_id, inventory_slot)
    where inventory_slot is not null;

create unique index if not exists character_items_cursor_idx
    on character_items(character_id)
    where is_cursor;

create index if not exists character_items_driver_idx on character_items(driver);
