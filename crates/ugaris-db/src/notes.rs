//! Generic per-character note persistence.
//!
//! Ports `src/system/database/database_notes.c::add_note`/`db_unpunish`/
//! `db_read_notes`/`db_karmalog` (the `notes` SQL table backing
//! `/punish`'s punishment records - see `ugaris-core`'s `world/punish.rs`
//! for the `kind = 1` `struct punishment` encode/decode - plus `/look`'s
//! staff notes viewer and `/klog`'s 24-hour karma log, see `ugaris-core`'s
//! `world/look.rs`). Every note `kind` other than `1` (punishment
//! records) is out of scope - see `PORTING_TODO.md`'s "Remaining `/` and
//! `#` text commands" task note.

use async_trait::async_trait;
use sqlx::PgPool;
use ugaris_core::ids::CharacterId;

/// One row of `db_read_notes`/`db_karmalog`'s `SELECT` (`database_notes.c:
/// 164-215`/`230-275`) - the columns both queries share. `db_read_notes`
/// doesn't select `uID` (it's already the caller-supplied filter), so
/// `target_id` is `None` for [`NotesRepository::list_notes_for_character`]
/// rows and always `Some` for [`NotesRepository::list_recent_notes`]
/// rows - see each method's doc comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteRow {
    pub id: i64,
    pub kind: i16,
    pub content: Vec<u8>,
    pub creator_id: CharacterId,
    pub created_at: i64,
    /// C `uID` - the character the note is filed *about*. `None` for
    /// `list_notes_for_character` (the caller already knows it - it's
    /// the filter argument), `Some` for `list_recent_notes` (`db_
    /// karmalog` additionally selects `uID` since it scans every
    /// character's notes, not one specific one).
    pub target_id: Option<CharacterId>,
}

#[async_trait]
pub trait NotesRepository: Send + Sync {
    /// C `add_note` (`database_notes.c:31-58`): `INSERT INTO notes
    /// VALUES(0,uID,kind,cID,date,content)`. `content` is stored as an
    /// opaque byte blob (C escapes it for a MySQL string literal;
    /// Postgres `bytea` parameter binding makes that unnecessary here).
    async fn add_note(
        &self,
        character_id: CharacterId,
        kind: i16,
        creator_id: CharacterId,
        content: &[u8],
        now_unix: i64,
    ) -> anyhow::Result<()>;

    /// C `db_unpunish` (`database_notes.c:407-451`): fetch a note's
    /// `content` by its bare `id` (no `uID` scoping - see this module's
    /// doc comment) and delete the row in the same call, returning
    /// `None` when no such row exists (C's "Failed to select"/"No
    /// content found" early-return paths, both of which map onto
    /// `/unpunish`'s "UnPunishment scheduled." going nowhere).
    async fn take_note(&self, note_id: i64) -> anyhow::Result<Option<Vec<u8>>>;

    /// C `db_read_notes` (`database_notes.c:164-215`): `SELECT kind,
    /// content, cID, date, ID FROM notes WHERE uID=%d`, every row for one
    /// character, in insertion (`id`) order - `/look`'s notes viewer.
    async fn list_notes_for_character(
        &self,
        character_id: CharacterId,
    ) -> anyhow::Result<Vec<NoteRow>>;

    /// C `db_karmalog` (`database_notes.c:230-275`): `SELECT kind,
    /// content, cID, date, ID, uID FROM notes WHERE date >= %d ORDER BY
    /// date DESC LIMIT 60` - `/klog`'s rolling 24-hour karma log, newest
    /// first. `since_unix` is the caller-computed `now - 86400` cutoff.
    async fn list_recent_notes(&self, since_unix: i64) -> anyhow::Result<Vec<NoteRow>>;
}

#[derive(Debug, Clone)]
pub struct PgNotesRepository {
    pool: PgPool,
}

impl PgNotesRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const ADD_NOTE_SQL: &str =
    "insert into notes(character_id, kind, creator_id, created_at, content) \
     values ($1, $2, $3, $4, $5)";

const TAKE_NOTE_SELECT_SQL: &str = "select content from notes where id = $1";
const TAKE_NOTE_DELETE_SQL: &str = "delete from notes where id = $1";

const LIST_NOTES_FOR_CHARACTER_SQL: &str = "select kind, content, creator_id, created_at, id \
     from notes where character_id = $1 order by id";

const LIST_RECENT_NOTES_SQL: &str =
    "select kind, content, character_id, created_at, id, creator_id \
     from notes where created_at >= $1 order by created_at desc limit 60";

#[async_trait]
impl NotesRepository for PgNotesRepository {
    async fn add_note(
        &self,
        character_id: CharacterId,
        kind: i16,
        creator_id: CharacterId,
        content: &[u8],
        now_unix: i64,
    ) -> anyhow::Result<()> {
        sqlx::query(ADD_NOTE_SQL)
            .bind(character_id.0 as i64)
            .bind(kind)
            .bind(creator_id.0 as i64)
            .bind(now_unix)
            .bind(content)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn take_note(&self, note_id: i64) -> anyhow::Result<Option<Vec<u8>>> {
        let row = sqlx::query_as::<_, (Vec<u8>,)>(TAKE_NOTE_SELECT_SQL)
            .bind(note_id)
            .fetch_optional(&self.pool)
            .await?;
        let Some((content,)) = row else {
            return Ok(None);
        };
        sqlx::query(TAKE_NOTE_DELETE_SQL)
            .bind(note_id)
            .execute(&self.pool)
            .await?;
        Ok(Some(content))
    }

    async fn list_notes_for_character(
        &self,
        character_id: CharacterId,
    ) -> anyhow::Result<Vec<NoteRow>> {
        let rows = sqlx::query_as::<_, (i16, Vec<u8>, i64, i64, i64)>(LIST_NOTES_FOR_CHARACTER_SQL)
            .bind(character_id.0 as i64)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|(kind, content, creator_id, created_at, id)| NoteRow {
                id,
                kind,
                content,
                creator_id: CharacterId(creator_id as u32),
                created_at,
                target_id: None,
            })
            .collect())
    }

    async fn list_recent_notes(&self, since_unix: i64) -> anyhow::Result<Vec<NoteRow>> {
        let rows = sqlx::query_as::<_, (i16, Vec<u8>, i64, i64, i64, i64)>(LIST_RECENT_NOTES_SQL)
            .bind(since_unix)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(
                |(kind, content, target_id, created_at, id, creator_id)| NoteRow {
                    id,
                    kind,
                    content,
                    creator_id: CharacterId(creator_id as u32),
                    created_at,
                    target_id: Some(CharacterId(target_id as u32)),
                },
            )
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_note_sql_inserts_all_five_columns() {
        assert!(ADD_NOTE_SQL
            .contains("insert into notes(character_id, kind, creator_id, created_at, content)"));
        assert!(ADD_NOTE_SQL.contains("values ($1, $2, $3, $4, $5)"));
    }

    #[test]
    fn take_note_sql_selects_then_deletes_by_bare_id() {
        assert_eq!(
            TAKE_NOTE_SELECT_SQL,
            "select content from notes where id = $1"
        );
        assert_eq!(TAKE_NOTE_DELETE_SQL, "delete from notes where id = $1");
    }

    #[test]
    fn list_notes_for_character_sql_filters_by_character_and_orders_by_id() {
        assert!(LIST_NOTES_FOR_CHARACTER_SQL.contains("where character_id = $1"));
        assert!(LIST_NOTES_FOR_CHARACTER_SQL.contains("order by id"));
        assert!(LIST_NOTES_FOR_CHARACTER_SQL.contains("kind, content, creator_id, created_at, id"));
    }

    #[test]
    fn list_recent_notes_sql_filters_by_date_orders_newest_first_and_caps_at_60() {
        assert!(LIST_RECENT_NOTES_SQL.contains("where created_at >= $1"));
        assert!(LIST_RECENT_NOTES_SQL.contains("order by created_at desc limit 60"));
        assert!(LIST_RECENT_NOTES_SQL
            .contains("kind, content, character_id, created_at, id, creator_id"));
    }

    mod live {
        use super::*;

        async fn connect() -> Option<PgPool> {
            let url = std::env::var("DATABASE_URL").ok()?;
            match PgPool::connect(&url).await {
                Ok(pool) => Some(pool),
                Err(err) => {
                    eprintln!("skipping live DB test: could not connect to DATABASE_URL: {err}");
                    None
                }
            }
        }

        /// A character id far outside any realistic seeded test range so
        /// this test can never collide with real note rows or other
        /// tests sharing the database.
        const TEST_CHARACTER: CharacterId = CharacterId(9_001);

        #[tokio::test]
        async fn add_then_take_note_round_trips_and_deletes() {
            let Some(pool) = connect().await else {
                return;
            };
            let repo = PgNotesRepository::new(pool.clone());

            repo.add_note(TEST_CHARACTER, 1, CharacterId(1), b"hello", 1_000)
                .await
                .expect("add note");

            let (id,): (i64,) = sqlx::query_as(
                "select id from notes where character_id = $1 order by id desc limit 1",
            )
            .bind(TEST_CHARACTER.0 as i64)
            .fetch_one(&pool)
            .await
            .expect("select inserted id");

            let taken = repo.take_note(id).await.expect("take note");
            assert_eq!(taken, Some(b"hello".to_vec()));

            // Second take: row was deleted, so this returns None.
            let taken_again = repo.take_note(id).await.expect("take note again");
            assert_eq!(taken_again, None);
        }

        #[tokio::test]
        async fn take_note_unknown_id_returns_none() {
            let Some(pool) = connect().await else {
                return;
            };
            let repo = PgNotesRepository::new(pool);
            let taken = repo.take_note(-1).await.expect("take note");
            assert_eq!(taken, None);
        }

        /// A second dedicated character id (distinct from
        /// `TEST_CHARACTER`, which `add_then_take_note_round_trips_and_
        /// deletes` mutates/deletes) so this test's rows survive
        /// independently of test execution order.
        const LIST_TEST_CHARACTER: CharacterId = CharacterId(9_002);

        #[tokio::test]
        async fn list_notes_for_character_returns_rows_in_id_order() {
            let Some(pool) = connect().await else {
                return;
            };
            let repo = PgNotesRepository::new(pool.clone());
            sqlx::query("delete from notes where character_id = $1")
                .bind(LIST_TEST_CHARACTER.0 as i64)
                .execute(&pool)
                .await
                .expect("clear prior rows");

            repo.add_note(LIST_TEST_CHARACTER, 1, CharacterId(1), b"first", 1_000)
                .await
                .expect("add first note");
            repo.add_note(LIST_TEST_CHARACTER, 1, CharacterId(2), b"second", 2_000)
                .await
                .expect("add second note");

            let rows = repo
                .list_notes_for_character(LIST_TEST_CHARACTER)
                .await
                .expect("list notes");
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].content, b"first");
            assert_eq!(rows[0].creator_id, CharacterId(1));
            assert_eq!(rows[0].target_id, None);
            assert_eq!(rows[1].content, b"second");
            assert_eq!(rows[1].creator_id, CharacterId(2));
        }

        #[tokio::test]
        async fn list_recent_notes_orders_newest_first_and_includes_target_id() {
            let Some(pool) = connect().await else {
                return;
            };
            let repo = PgNotesRepository::new(pool.clone());
            sqlx::query("delete from notes where character_id = $1")
                .bind(LIST_TEST_CHARACTER.0 as i64)
                .execute(&pool)
                .await
                .expect("clear prior rows");

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time after epoch")
                .as_secs() as i64;
            repo.add_note(LIST_TEST_CHARACTER, 1, CharacterId(1), b"older", now - 10)
                .await
                .expect("add older note");
            repo.add_note(LIST_TEST_CHARACTER, 1, CharacterId(2), b"newer", now)
                .await
                .expect("add newer note");

            let rows = repo
                .list_recent_notes(now - 3600)
                .await
                .expect("list recent notes");
            let ours: Vec<_> = rows
                .into_iter()
                .filter(|row| row.target_id == Some(LIST_TEST_CHARACTER))
                .collect();
            assert_eq!(ours.len(), 2);
            assert_eq!(ours[0].content, b"newer");
            assert_eq!(ours[1].content, b"older");
        }
    }
}
