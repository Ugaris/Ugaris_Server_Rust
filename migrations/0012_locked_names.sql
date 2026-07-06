-- Mirrors legacy `badname` table (src/system/database/database_admin.c
-- db_lockname/db_unlockname, inferred from its own `INSERT INTO badname
-- VALUES(0,'%s')` / `DELETE FROM badname WHERE bad='%s'` queries - no
-- CREATE TABLE for it exists anywhere in the legacy C tree). `/lockname`
-- and `/unlockname` (`command.c:2679-2701`) are the only legacy callers;
-- nothing in this codebase (or, as far as can be told, the legacy C tree
-- either) consults this table at character-creation time - it exists
-- purely as an admin-facing audit/blocklist record, and this port
-- matches that scope exactly.
create table if not exists locked_names (
    id bigserial primary key,
    name text not null unique,
    created_at timestamptz not null default now()
);
