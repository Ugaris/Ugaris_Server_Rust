//! Restart-persistence for the pentagram-quest lifetime "most pentagrams
//! activated in one run" record
//! (`crates/ugaris-core/src/world/pents.rs::PentagramQuestState::
//! pentagram_record`/`pentagram_record_holder`).
//!
//! Unlike every other repository in this crate, legacy C already has a
//! dedicated table for this exact state
//! (`src/system/database/database_pent_record.c`:
//! `load_pentagram_record`/`save_pentagram_record`), so this mirrors its
//! shape one-for-one instead of inventing a JSON-blob schema. See
//! `migrations/0021_pentagram_record.sql`'s doc comment for why
//! `char_id` is always stored as `0`.

use async_trait::async_trait;
use sqlx::PgPool;

/// One row of C's `pentagram_record` table (`database_pent_record.c`):
/// the current per-area lifetime-pentagram-count record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PentagramRecordRow {
    pub record_count: i32,
    pub char_name: String,
}

#[async_trait]
pub trait PentagramRecordRepository: Send + Sync {
    /// C `load_pentagram_record` (`database_pent_record.c:62-90`).
    /// Returns `None` when nothing has ever been saved for this area,
    /// matching C's `if (!row) return 0;`.
    async fn load(&self, area_id: i32) -> anyhow::Result<Option<PentagramRecordRow>>;

    /// C `save_pentagram_record` (`database_pent_record.c:103-121`): an
    /// upsert keyed by `area_id`.
    async fn save(&self, area_id: i32, row: &PentagramRecordRow) -> anyhow::Result<()>;
}

#[derive(Debug, Clone)]
pub struct PgPentagramRecordRepository {
    pool: PgPool,
}

impl PgPentagramRecordRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const UPSERT_SQL: &str =
    "insert into pentagram_record(area_id, char_id, char_name, record_count, record_date) \
     values ($1, 0, $2, $3, now()) \
     on conflict (area_id) do update set char_id = 0, char_name = excluded.char_name, \
     record_count = excluded.record_count, record_date = now()";

const LOAD_SQL: &str = "select char_name, record_count from pentagram_record where area_id = $1";

#[async_trait]
impl PentagramRecordRepository for PgPentagramRecordRepository {
    async fn load(&self, area_id: i32) -> anyhow::Result<Option<PentagramRecordRow>> {
        let row = sqlx::query_as::<_, (String, i32)>(LOAD_SQL)
            .bind(area_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|(char_name, record_count)| PentagramRecordRow {
            char_name,
            record_count,
        }))
    }

    async fn save(&self, area_id: i32, row: &PentagramRecordRow) -> anyhow::Result<()> {
        sqlx::query(UPSERT_SQL)
            .bind(area_id)
            .bind(&row.char_name)
            .bind(row.record_count)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_sql_targets_area_id_and_updates_in_place() {
        assert!(UPSERT_SQL.contains("values ($1, 0, $2, $3, now())"));
        assert!(UPSERT_SQL.contains("on conflict (area_id) do update"));
        assert!(LOAD_SQL.contains("from pentagram_record where area_id = $1"));
    }

    /// Mirrors `military.rs`/`clan.rs`'s own `live` test convention:
    /// exercises the real save/load round trip against a live Postgres
    /// instance when `DATABASE_URL` is set, and skips (never fails)
    /// otherwise so `cargo test --workspace` stays green without
    /// Postgres present.
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

        #[tokio::test]
        async fn save_then_load_round_trips_a_record_row() {
            let Some(pool) = connect().await else {
                return;
            };
            let repo = PgPentagramRecordRepository::new(pool.clone());

            // Use an area id extremely unlikely to collide with any
            // other test/real data sharing this database, and clean up
            // after ourselves.
            let area_id = 987_654_321i32;
            sqlx::query("delete from pentagram_record where area_id = $1")
                .bind(area_id)
                .execute(&pool)
                .await
                .ok();

            let row = PentagramRecordRow {
                record_count: 42,
                char_name: "Solveig".to_string(),
            };
            repo.save(area_id, &row).await.expect("save record");

            let loaded = repo.load(area_id).await.expect("load record");
            assert_eq!(loaded, Some(row.clone()));

            // Saving again with a new holder overwrites in place
            // (`on conflict (area_id) do update`), matching C's
            // `INSERT ... ON DUPLICATE KEY UPDATE`.
            let updated = PentagramRecordRow {
                record_count: 50,
                char_name: "Bragi".to_string(),
            };
            repo.save(area_id, &updated).await.expect("update record");
            let reloaded = repo.load(area_id).await.expect("reload record");
            assert_eq!(reloaded, Some(updated));

            sqlx::query("delete from pentagram_record where area_id = $1")
                .bind(area_id)
                .execute(&pool)
                .await
                .ok();
        }

        #[tokio::test]
        async fn load_returns_none_when_nothing_was_ever_saved_for_an_area() {
            let Some(pool) = connect().await else {
                return;
            };
            let area_id = 987_654_322i32;
            sqlx::query("delete from pentagram_record where area_id = $1")
                .bind(area_id)
                .execute(&pool)
                .await
                .ok();
            let repo = PgPentagramRecordRepository::new(pool);
            let loaded = repo.load(area_id).await.expect("load record");
            assert_eq!(loaded, None);
        }
    }
}
