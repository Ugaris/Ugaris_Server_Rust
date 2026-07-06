//! Generic per-character note persistence.
//!
//! Ports `src/system/database/database_notes.c::add_note`/`db_unpunish`
//! (the `notes` SQL table backing `/punish`'s punishment records today -
//! see `ugaris-core`'s `world/punish.rs` for the `kind = 1` `struct
//! punishment` encode/decode). `db_read_notes`/`list_punishment` (the
//! `/look` staff notes viewer) and every other note `kind` are out of
//! scope for this slice - see `PORTING_TODO.md`'s "Remaining `/` and `#`
//! text commands" task note.

use async_trait::async_trait;
use sqlx::PgPool;
use ugaris_core::ids::CharacterId;

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
    }
}
