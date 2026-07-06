-- `/exterminate <name>`'s IP-ban half (C `db_exterminate`, `src/system/
-- database/database_admin.c:29-95`: `INSERT ipban SELECT 0,ip,now,now+4
-- weeks FROM iplog WHERE sID=%d`). This codebase has no separate `iplog`
-- table - `login_sessions.ip_address` already records every IP a
-- character's account has logged in from (see migration `0002`), so
-- `/exterminate` populates this table from that history instead of a
-- dedicated log. Checked at login time (`is_ip_banned` in
-- `crates/ugaris-db/src/character.rs`, matching C's `isbanned_iplog`
-- gate in `load_char_pwd`) independent of which account is attempting
-- to log in, exactly like C's IP-keyed (not account-keyed) `ipban`
-- table - this is the real mechanism the pre-existing `accounts.
-- ip_locked` static per-account flag only approximated.
create table if not exists ip_bans (
    id bigserial primary key,
    ip integer not null,
    banned_until timestamptz not null,
    created_at timestamptz not null default now()
);

create index if not exists ip_bans_ip_idx on ip_bans(ip);
