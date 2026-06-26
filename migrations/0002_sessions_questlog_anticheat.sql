create table if not exists character_questlog (
    character_id bigint not null references characters(id) on delete cascade,
    quest_id integer not null,
    done_count smallint not null default 0,
    flags smallint not null default 0,
    updated_at timestamptz not null default now(),
    primary key (character_id, quest_id),
    constraint character_questlog_done_count_range check (done_count between 0 and 63),
    constraint character_questlog_quest_id_range check (quest_id between 0 and 99)
);

create table if not exists login_sessions (
    id bigserial primary key,
    character_id bigint references characters(id),
    account_id bigint references accounts(id),
    character_name text,
    ip_address integer not null,
    area_id integer not null,
    mirror_id integer not null,
    client_vendor integer not null default 0,
    client_version integer,
    unique_id integer not null default 0,
    started_at timestamptz not null default now(),
    ended_at timestamptz
);

create table if not exists anticheat_sessions (
    id bigserial primary key,
    login_session_id bigint references login_sessions(id),
    account_id bigint references accounts(id),
    character_id bigint references characters(id),
    ip_address integer not null,
    area_id integer not null,
    status integer not null default 0,
    mod_major integer,
    mod_minor integer,
    mod_patch integer,
    os_type integer,
    screen_w integer,
    screen_h integer,
    hardware_hash bigint,
    code_hash bigint,
    bot_score real not null default 0,
    max_bot_score real not null default 0,
    heartbeat_violations integer not null default 0,
    state_violations integer not null default 0,
    challenge_failures integer not null default 0,
    anomaly_count integer not null default 0,
    timeout_count integer not null default 0,
    started_at timestamptz not null default now(),
    ended_at timestamptz
);

create table if not exists anticheat_events (
    id bigserial primary key,
    session_id bigint references anticheat_sessions(id) on delete cascade,
    event_type text not null,
    severity integer not null default 0,
    details text,
    data jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now()
);
