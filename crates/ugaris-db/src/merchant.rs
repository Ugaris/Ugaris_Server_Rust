//! Merchant store persistence.
//!
//! Ports the read/write side of `src/system/database/database_merchant.c`
//! (`save_merchant_inventory`/`load_merchant_inventory`). C persists each
//! ware as its own row with hand-rolled JSON-encoded `drdata`/modifiers
//! columns keyed by `(merchant_name, merchant_x, merchant_y)`; Rust stores
//! the whole ware list for a merchant as one `jsonb` array in a single row
//! with the same key, since `Item` already serializes cleanly (see
//! `character.rs`'s `character_json`/`item_json` columns for precedent).
//! The incremental-change task queue (`merchant_tasks.c`,
//! `save_incremental_change`) is not ported: callers just re-save the full
//! snapshot on every mutation, matching C's own
//! `add_item_to_merchant`/`remove_item_from_merchant`/`update_merchant_item`
//! helpers, which are themselves "simple implementation - just save the
//! entire inventory".

use async_trait::async_trait;
use sqlx::{types::Json, PgPool};
use ugaris_core::entity::Item;

/// One store slot, mirroring C `struct store_item` (`item`, `cnt`,
/// `always`) as saved by `save_merchant_inventory`.
///
/// Note: `Item` (from `ugaris-core`) doesn't implement `PartialEq`, so
/// neither does this type or `MerchantStoreSnapshot` - tests compare via
/// `serde_json` serialization instead of `assert_eq!`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MerchantWareSnapshot {
    pub item: Item,
    pub count: u32,
    pub always: bool,
}

/// A full merchant store as persisted to/loaded from the database, keyed
/// like C by merchant name and spawn position.
#[derive(Debug, Clone)]
pub struct MerchantStoreSnapshot {
    pub merchant_name: String,
    pub x: i32,
    pub y: i32,
    pub gold: i64,
    pub price_multi: i32,
    /// Index matches the store's ware slot; `None` is an empty slot.
    pub wares: Vec<Option<MerchantWareSnapshot>>,
}

#[async_trait]
pub trait MerchantRepository: Send + Sync {
    /// C `save_merchant_inventory`: upsert the merchant's gold, pricemulti
    /// and full ware list.
    async fn save_store(&self, snapshot: &MerchantStoreSnapshot) -> anyhow::Result<()>;

    /// C `load_merchant_inventory`: fetch the persisted store for a
    /// merchant name/position, or `None` if nothing was ever saved for it
    /// (C: query returns no rows).
    async fn load_store(
        &self,
        merchant_name: &str,
        x: i32,
        y: i32,
    ) -> anyhow::Result<Option<MerchantStoreSnapshot>>;
}

#[derive(Debug, Clone)]
pub struct PgMerchantRepository {
    pool: PgPool,
}

impl PgMerchantRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const UPSERT_MERCHANT_STORE_SQL: &str =
    "insert into merchant_stores(merchant_name, merchant_x, merchant_y, gold, price_multi, wares_json, updated_at) \
     values ($1, $2, $3, $4, $5, $6, now()) \
     on conflict (merchant_name, merchant_x, merchant_y) do update set \
     gold = excluded.gold, price_multi = excluded.price_multi, wares_json = excluded.wares_json, updated_at = now()";

const LOAD_MERCHANT_STORE_SQL: &str = "select gold, price_multi, wares_json from merchant_stores \
     where merchant_name = $1 and merchant_x = $2 and merchant_y = $3";

#[async_trait]
impl MerchantRepository for PgMerchantRepository {
    async fn save_store(&self, snapshot: &MerchantStoreSnapshot) -> anyhow::Result<()> {
        sqlx::query(UPSERT_MERCHANT_STORE_SQL)
            .bind(&snapshot.merchant_name)
            .bind(snapshot.x)
            .bind(snapshot.y)
            .bind(snapshot.gold)
            .bind(snapshot.price_multi)
            .bind(Json(&snapshot.wares))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn load_store(
        &self,
        merchant_name: &str,
        x: i32,
        y: i32,
    ) -> anyhow::Result<Option<MerchantStoreSnapshot>> {
        let row = sqlx::query_as::<_, (i64, i32, Json<Vec<Option<MerchantWareSnapshot>>>)>(
            LOAD_MERCHANT_STORE_SQL,
        )
        .bind(merchant_name)
        .bind(x)
        .bind(y)
        .fetch_optional(&self.pool)
        .await?;

        Ok(
            row.map(|(gold, price_multi, Json(wares))| MerchantStoreSnapshot {
                merchant_name: merchant_name.to_string(),
                x,
                y,
                gold,
                price_multi,
                wares,
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_snapshot() -> MerchantStoreSnapshot {
        MerchantStoreSnapshot {
            merchant_name: "Dolf".to_string(),
            x: 100,
            y: 200,
            gold: 5_000,
            price_multi: 400,
            wares: vec![
                None,
                Some(MerchantWareSnapshot {
                    item: sample_item(),
                    count: 3,
                    always: true,
                }),
            ],
        }
    }

    fn sample_item() -> Item {
        Item {
            id: ugaris_core::ids::ItemId(42),
            name: "Sword".to_string(),
            description: "A sharp sword".to_string(),
            flags: ugaris_core::entity::ItemFlags::empty(),
            sprite: 123,
            value: 500,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [0; ugaris_core::entity::MAX_MODIFIERS],
            modifier_value: [0; ugaris_core::entity::MAX_MODIFIERS],
            x: 0,
            y: 0,
            carried_by: None,
            contained_in: None,
            content_id: 0,
            driver: 0,
            driver_data: Vec::new(),
            serial: 0,
        }
    }

    /// The `wares_json` column round-trips through `serde_json` exactly the
    /// same way `sqlx::types::Json` will bind/decode it against Postgres -
    /// this catches serialization drift without needing a live database.
    /// `Item` (and so `MerchantWareSnapshot`) has no `PartialEq`, so the
    /// round trip is verified by re-encoding the decoded value and
    /// comparing JSON text instead of the values directly.
    #[test]
    fn ware_snapshot_round_trips_through_json() {
        let snapshot = sample_snapshot();
        let encoded = serde_json::to_string(&snapshot.wares).expect("encode wares");
        let decoded: Vec<Option<MerchantWareSnapshot>> =
            serde_json::from_str(&encoded).expect("decode wares");
        let re_encoded = serde_json::to_string(&decoded).expect("re-encode wares");
        assert_eq!(re_encoded, encoded);
    }

    /// Mirrors `crate::character`'s `live_login` convention: exercises the
    /// real save/load round trip against a live Postgres instance when
    /// `DATABASE_URL` is set, and skips (never fails) otherwise so
    /// `cargo test --workspace` stays green without Postgres present.
    mod live {
        use super::*;
        use sqlx::PgPool;

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
        async fn save_then_load_round_trips_gold_pricemulti_and_wares() {
            let Some(pool) = connect().await else {
                return;
            };
            let repo = PgMerchantRepository::new(pool.clone());
            let snapshot = MerchantStoreSnapshot {
                merchant_name: "live_test_merchant".to_string(),
                x: 4242,
                y: 4343,
                ..sample_snapshot()
            };

            repo.save_store(&snapshot).await.expect("save store");
            let loaded = repo
                .load_store(&snapshot.merchant_name, snapshot.x, snapshot.y)
                .await
                .expect("load store")
                .expect("store was saved");

            assert_eq!(loaded.merchant_name, snapshot.merchant_name);
            assert_eq!(loaded.x, snapshot.x);
            assert_eq!(loaded.y, snapshot.y);
            assert_eq!(loaded.gold, snapshot.gold);
            assert_eq!(loaded.price_multi, snapshot.price_multi);
            assert_eq!(
                serde_json::to_string(&loaded.wares).expect("encode loaded wares"),
                serde_json::to_string(&snapshot.wares).expect("encode expected wares"),
            );

            // Clean up so repeated runs don't accumulate rows.
            sqlx::query(
                "delete from merchant_stores where merchant_name = $1 and merchant_x = $2 and merchant_y = $3",
            )
            .bind(&snapshot.merchant_name)
            .bind(snapshot.x)
            .bind(snapshot.y)
            .execute(&pool)
            .await
            .expect("cleanup live row");
        }

        #[tokio::test]
        async fn load_returns_none_for_unknown_merchant() {
            let Some(pool) = connect().await else {
                return;
            };
            let repo = PgMerchantRepository::new(pool);
            let loaded = repo
                .load_store("nonexistent_merchant_xyz", 1, 2)
                .await
                .expect("load store");
            assert!(loaded.is_none());
        }
    }
}
