create table if not exists accounts (
    id bigserial primary key,
    username text not null unique,
    password_hash text not null,
    locked boolean not null default false,
    ip_locked boolean not null default false,
    fixed boolean not null default true,
    paid_until timestamptz,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create table if not exists characters (
    id bigserial primary key,
    account_id bigint not null references accounts(id),
    name text not null unique,
    locked boolean not null default false,
    current_area integer not null default 0,
    allowed_area integer not null default 1,
    mirror integer not null default 0,
    current_mirror integer not null default 0,
    unique_id integer not null default 0,
    level integer not null default 1,
    gold bigint not null default 0,
    karma integer not null default 0,
    character_blob bytea not null default ''::bytea,
    item_blob bytea not null default ''::bytea,
    ppd_blob bytea not null default ''::bytea,
    subscriber_blob bytea not null default ''::bytea,
    login_time timestamptz,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index if not exists characters_account_id_idx on characters(account_id);
create index if not exists characters_area_idx on characters(allowed_area, mirror);

create table if not exists area_servers (
    area_id integer not null,
    mirror_id integer not null,
    server_addr integer not null default 0,
    server_port integer not null default 0,
    online boolean not null default false,
    last_seen timestamptz,
    primary key (area_id, mirror_id)
);

create table if not exists constants (
    name text primary key,
    value bigint not null
);

insert into constants(name, value)
values ('unique', 1)
on conflict (name) do nothing;
