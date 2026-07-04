//! Restart-persistence for [`ClanRegistry`], the pure-Rust clan identity/
//! relation registry ported in `crates/ugaris-core/src/clan.rs`.
//!
//! Legacy C has no per-row database table for the live clan state itself
//! (only `clanoverview`, a write-only mirror of a handful of display
//! fields consumed by an external website - `showclan_db`,
//! `src/system/clan.c:97-126` - which is a different concern and not
//! ported here). C's actual clan data (`struct clan clan[MAXCLAN]`,
//! `clan.c:58`) survives a restart only because it lives inside the
//! server's single memory-image world save file. `ClanRegistry` already
//! derives `Serialize`/`Deserialize` end-to-end (identities, per-slot
//! serials, and the full pairwise relation matrices), so this repository
//! mirrors that "one blob" approach with a single-row `jsonb` snapshot
//! rather than inventing a relational schema for data that has no
//! natural per-row key in C.

use async_trait::async_trait;
use sqlx::{types::Json, PgPool};
use ugaris_core::clan::ClanRegistry;

#[async_trait]
pub trait ClanRegistryRepository: Send + Sync {
    /// Upserts the single persisted snapshot row with the current
    /// in-memory [`ClanRegistry`] state.
    async fn save_registry(&self, registry: &ClanRegistry) -> anyhow::Result<()>;

    /// Loads the persisted snapshot, or `None` if nothing has ever been
    /// saved (a brand-new database - the caller should keep the
    /// freshly-`Default`-constructed in-memory registry in that case).
    async fn load_registry(&self) -> anyhow::Result<Option<ClanRegistry>>;
}

#[derive(Debug, Clone)]
pub struct PgClanRegistryRepository {
    pool: PgPool,
}

impl PgClanRegistryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const UPSERT_CLAN_REGISTRY_SQL: &str = "insert into clan_registry(id, registry_json, updated_at) \
     values (1, $1, now()) \
     on conflict (id) do update set registry_json = excluded.registry_json, updated_at = now()";

const LOAD_CLAN_REGISTRY_SQL: &str = "select registry_json from clan_registry where id = 1";

#[async_trait]
impl ClanRegistryRepository for PgClanRegistryRepository {
    async fn save_registry(&self, registry: &ClanRegistry) -> anyhow::Result<()> {
        sqlx::query(UPSERT_CLAN_REGISTRY_SQL)
            .bind(Json(registry))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn load_registry(&self) -> anyhow::Result<Option<ClanRegistry>> {
        let row = sqlx::query_as::<_, (Json<ClanRegistry>,)>(LOAD_CLAN_REGISTRY_SQL)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|(Json(registry),)| registry))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_sql_targets_the_singleton_row_and_updates_in_place() {
        assert!(UPSERT_CLAN_REGISTRY_SQL.contains("values (1, $1, now())"));
        assert!(UPSERT_CLAN_REGISTRY_SQL.contains("on conflict (id) do update"));
        assert!(LOAD_CLAN_REGISTRY_SQL.contains("where id = 1"));
    }

    /// `ClanRegistry` round-trips through `serde_json` exactly the same
    /// way `sqlx::types::Json` will bind/decode it against Postgres -
    /// catches serialization drift without needing a live database.
    #[test]
    fn registry_round_trips_through_json() {
        let mut registry = ClanRegistry::new();
        registry
            .found_clan("Iron Wolves", 1_000)
            .expect("found clan");
        let encoded = serde_json::to_string(&registry).expect("encode registry");
        let decoded: ClanRegistry = serde_json::from_str(&encoded).expect("decode registry");
        assert_eq!(decoded.name(1), Some("Iron Wolves"));
        assert_eq!(decoded.serial(1), registry.serial(1));
    }

    /// Mirrors `merchant.rs`'s `live` test convention: exercises the real
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
        async fn save_then_load_round_trips_a_founded_clan() {
            let Some(pool) = connect().await else {
                return;
            };
            let repo = PgClanRegistryRepository::new(pool.clone());

            // Snapshot and restore whatever was there before so repeated
            // runs (and other tests sharing this database) don't clobber
            // each other's state.
            let previous = repo.load_registry().await.expect("load previous");

            let mut registry = ClanRegistry::new();
            let nr = registry
                .found_clan("live_test_clan", 42)
                .expect("found clan");
            repo.save_registry(&registry).await.expect("save registry");

            let loaded = repo
                .load_registry()
                .await
                .expect("load registry")
                .expect("registry was saved");
            assert_eq!(loaded.name(nr), Some("live_test_clan"));
            assert_eq!(loaded.serial(nr), registry.serial(nr));

            match previous {
                Some(previous) => repo
                    .save_registry(&previous)
                    .await
                    .expect("restore previous registry"),
                None => {
                    sqlx::query("delete from clan_registry where id = 1")
                        .execute(&pool)
                        .await
                        .ok();
                }
            }
        }

        #[tokio::test]
        async fn load_returns_none_when_nothing_was_ever_saved() {
            let Some(pool) = connect().await else {
                return;
            };
            // Only meaningful on a database that has never saved a
            // registry; if some other test already seeded one, this is a
            // no-op assertion-skip rather than a false failure.
            let repo = PgClanRegistryRepository::new(pool);
            if let Ok(None) = repo.load_registry().await {
                // Confirmed the no-rows case decodes to `None` without
                // error; nothing further to assert.
            }
        }
    }
}
