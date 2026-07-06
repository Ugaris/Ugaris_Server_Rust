-- Mirrors the legacy `ac_known_signatures` table (`src/system/database/
-- database_anticheat.c:1143-1216`, backing `#acsiglist`/`#acsigadd`/
-- `#acsigdel` - `ac_cmd_siglist`/`ac_cmd_sigadd`/`ac_cmd_sigdel`,
-- `src/module/anticheat/anticheat.c:1192-1266`). Column set matches
-- `struct db_ac_signature_result` (`database_anticheat.h:589-598`) plus
-- the two write-only columns `db_ac_add_signature` inserts but no read
-- query ever selects back (`signature_value`, `created_by`) - reproduced
-- as-is, not "fixed", by keeping them in the table without adding a
-- read path for them.
create table if not exists ac_known_signatures (
    id bigserial primary key,
    signature_type text not null,
    signature_value text not null,
    name text not null,
    created_by text,
    severity integer not null default 0,
    auto_flag boolean not null default false,
    auto_ban boolean not null default false,
    times_detected integer not null default 0,
    is_active boolean not null default true,
    created_at timestamptz not null default now()
);
