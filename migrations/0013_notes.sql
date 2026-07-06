-- Mirrors the legacy `notes` table (`src/system/database/
-- database_notes.c::add_note`/`db_unpunish`, plus the read side
-- `db_read_notes`), a generic per-character note/log record used today
-- only by `/punish`'s punishment records (`kind = 1`, C `struct
-- punishment { level, exp, karma, reason[80] }`, stored as an opaque
-- binary blob exactly like this codebase's various PPD blobs - see
-- `ugaris-core`'s `world/punish.rs` for the encode/decode).
--
-- C's `INSERT INTO notes VALUES(0,%d,%d,%d,%d,'%s')` binds
-- `(uID, kind, cID, date, content)` after the auto id; `db_unpunish`
-- (`/unpunish`'s DB half) looks a row up by its bare `id` (not scoped to
-- `uID` at all - a genuine C quirk, preserved as-is: any valid note id
-- can be "unpunished" regardless of which character it was actually
-- filed against) and deletes it after reading `content` back.
create table if not exists notes (
    id bigserial primary key,
    character_id bigint not null,
    kind smallint not null,
    creator_id bigint not null,
    created_at bigint not null,
    content bytea not null
);

create index if not exists notes_character_idx on notes(character_id);
