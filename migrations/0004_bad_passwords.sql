-- Mirrors legacy `badip` table (src/system/badip.c): one row per failed
-- password attempt, keyed by source IP and timestamp. `is_badpass_ip`
-- counts rows within sliding windows to rate-limit repeated bad-password
-- login attempts from the same IP.
create table if not exists bad_passwords (
    id bigserial primary key,
    ip integer not null,
    created_at timestamptz not null default now()
);

create index if not exists bad_passwords_ip_created_idx on bad_passwords(ip, created_at);
