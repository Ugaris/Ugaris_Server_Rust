//! Restart-persistence for [`MilitaryMasterStorageRegistry`], the
//! per-NPC Military Master storage-blob registry ported in
//! `crates/ugaris-core/src/world/military.rs`.
//!
//! Legacy C persists these counters through the generic `storage` table
//! (`create_storage`/`read_storage`/`update_storage`,
//! `src/system/database/database_storage.c`), a single byte-blob-per-id
//! mechanism shared by many unrelated NPC drivers. Rust instead gives
//! this consumer its own typed table, `military_master_storage`, keyed
//! by `storage_id` (one row per Military Master NPC's zone-file
//! `storage=N;` id - unlike [`crate::clan::ClanRegistry`]'s single-row
//! whole-server blob, since Military Master storage genuinely isn't a
//! singleton).

use async_trait::async_trait;
use sqlx::{types::Json, PgPool};
use ugaris_core::world::{MilitaryMasterStorage, MilitaryMasterStorageRegistry};

#[async_trait]
pub trait MilitaryMasterStorageRepository: Send + Sync {
    /// Upserts every `(storage_id, storage)` row currently held by the
    /// in-memory registry.
    async fn save_registry(&self, registry: &MilitaryMasterStorageRegistry) -> anyhow::Result<()>;

    /// Loads every persisted row into a fresh registry. Returns an empty
    /// (`Default`) registry if nothing has ever been saved, matching a
    /// brand-new database.
    async fn load_registry(&self) -> anyhow::Result<MilitaryMasterStorageRegistry>;
}

#[derive(Debug, Clone)]
pub struct PgMilitaryMasterStorageRepository {
    pool: PgPool,
}

impl PgMilitaryMasterStorageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const UPSERT_STORAGE_SQL: &str =
    "insert into military_master_storage(storage_id, storage_json, updated_at) \
     values ($1, $2, now()) \
     on conflict (storage_id) do update set storage_json = excluded.storage_json, \
     updated_at = now()";

const LOAD_ALL_STORAGE_SQL: &str = "select storage_id, storage_json from military_master_storage";

#[async_trait]
impl MilitaryMasterStorageRepository for PgMilitaryMasterStorageRepository {
    async fn save_registry(&self, registry: &MilitaryMasterStorageRegistry) -> anyhow::Result<()> {
        // One upsert per row; a fresh registry with no entries yet is a
        // no-op, matching C's `create_storage` never being called until
        // the first counter actually changes.
        for (storage_id, storage) in registry.iter() {
            sqlx::query(UPSERT_STORAGE_SQL)
                .bind(storage_id)
                .bind(Json(storage))
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    async fn load_registry(&self) -> anyhow::Result<MilitaryMasterStorageRegistry> {
        let rows = sqlx::query_as::<_, (i32, Json<MilitaryMasterStorage>)>(LOAD_ALL_STORAGE_SQL)
            .fetch_all(&self.pool)
            .await?;
        Ok(MilitaryMasterStorageRegistry::from_rows(
            rows.into_iter()
                .map(|(storage_id, Json(storage))| (storage_id, storage)),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_sql_targets_storage_id_and_updates_in_place() {
        assert!(UPSERT_STORAGE_SQL.contains("values ($1, $2, now())"));
        assert!(UPSERT_STORAGE_SQL.contains("on conflict (storage_id) do update"));
        assert!(LOAD_ALL_STORAGE_SQL.contains("from military_master_storage"));
    }

    /// Builds a [`MilitaryMasterStorage`] with a single non-zero
    /// `clan_pts` slot via `serde_json` rather than calling any of its
    /// mutators (all crate-private in `ugaris-core` - only `World`'s own
    /// methods are meant to change these counters), exactly the way
    /// `sqlx::types::Json` decodes a real row's blob.
    fn storage_with_clan_pts(clan_nr: u16, pts: i32) -> MilitaryMasterStorage {
        let mut clan_pts = vec![0i32; 32];
        clan_pts[clan_nr as usize] = pts;
        serde_json::from_value(serde_json::json!({
            "clan_pts": clan_pts,
            "quests_given": [0, 0, 0, 0, 0],
            "quests_solved": [0, 0, 0, 0, 0],
            "exp_given": [0, 0, 0, 0, 0],
            "pts_given": [0, 0, 0, 0, 0],
        }))
        .expect("decode synthetic storage")
    }

    /// `MilitaryMasterStorageRegistry` round-trips through `serde_json`
    /// exactly the same way `sqlx::types::Json` will bind/decode each
    /// row's blob against Postgres - catches serialization drift without
    /// needing a live database.
    #[test]
    fn registry_rows_round_trip_through_json() {
        let storage = storage_with_clan_pts(3, 500);
        let registry = MilitaryMasterStorageRegistry::from_rows([(7, storage)]);
        let mut rows: Vec<(i32, MilitaryMasterStorage)> = registry
            .iter()
            .map(|(id, storage)| (id, storage.clone()))
            .collect();
        assert_eq!(rows.len(), 1);
        let (storage_id, storage) = rows.remove(0);
        let encoded = serde_json::to_string(&storage).expect("encode storage");
        let decoded: MilitaryMasterStorage =
            serde_json::from_str(&encoded).expect("decode storage");
        let reloaded = MilitaryMasterStorageRegistry::from_rows([(storage_id, decoded)]);
        assert_eq!(reloaded.clan_pts(storage_id, 3), 500);
    }

    /// Mirrors `clan.rs`'s `live` test convention: exercises the real
    /// save/load round trip against a live Postgres instance when
    /// `DATABASE_URL` is set, and skips (never fails) otherwise so
    /// `cargo test --workspace` stays green without Postgres present.
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
        async fn save_then_load_round_trips_a_storage_row() {
            let Some(pool) = connect().await else {
                return;
            };
            let repo = PgMilitaryMasterStorageRepository::new(pool.clone());

            // Use a storage id extremely unlikely to collide with any
            // other test/real data sharing this database, and clean up
            // after ourselves rather than snapshot/restore (unlike the
            // singleton clan registry, per-id rows are safe to add/
            // remove independently).
            let storage_id = 987_654_321i32;
            sqlx::query("delete from military_master_storage where storage_id = $1")
                .bind(storage_id)
                .execute(&pool)
                .await
                .ok();

            let storage = storage_with_clan_pts(5, 1200);
            let registry = MilitaryMasterStorageRegistry::from_rows([(storage_id, storage)]);
            repo.save_registry(&registry).await.expect("save registry");

            let loaded = repo.load_registry().await.expect("load registry");
            assert_eq!(loaded.clan_pts(storage_id, 5), 1200);

            sqlx::query("delete from military_master_storage where storage_id = $1")
                .bind(storage_id)
                .execute(&pool)
                .await
                .ok();
        }

        #[tokio::test]
        async fn load_returns_empty_registry_when_nothing_was_ever_saved_for_an_id() {
            let Some(pool) = connect().await else {
                return;
            };
            let storage_id = 987_654_322i32;
            sqlx::query("delete from military_master_storage where storage_id = $1")
                .bind(storage_id)
                .execute(&pool)
                .await
                .ok();
            let repo = PgMilitaryMasterStorageRepository::new(pool);
            let loaded = repo.load_registry().await.expect("load registry");
            assert_eq!(loaded.clan_pts(storage_id, 1), 0);
        }
    }
}
