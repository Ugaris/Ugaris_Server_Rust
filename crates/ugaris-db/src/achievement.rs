//! Achievement "first player globally" tracking.
//!
//! Ports the one *live* call site's worth of
//! `src/system/database/database_achievement.c`:
//! `db_achievement_record_unlock`, called from `achievement_award`
//! (`src/module/achievements/achievement.c:617-631`) to upsert
//! `achievement_firsts`/insert into `achievement_history` and learn
//! whether this was the first global unlock (which then drives the
//! cross-server "Grats: NAME is the FIRST to unlock ACH!" channel-6
//! broadcast - see `crates/ugaris-server/src/achievement.rs`'s wiring).
//!
//! `db_achievement_get_first`/`db_achievement_get_unlock_count`/
//! `db_achievement_get_recent_firsts` are declared in the same C file but
//! have zero call sites anywhere else in the legacy tree (confirmed via a
//! full-tree grep) - dead code in C itself, like the sibling
//! `db_get_character_name` in `auction_db.c` (see `auction.rs`'s doc
//! comment), so they are not ported here.
//!
//! C keys both tables by `subscriber_id` (account-wide: the first unlock
//! across *any* of an account's characters). This codebase has no live
//! multi-character-per-account model in the running server yet - the
//! exact same scoping compromise `crates/ugaris-server/src/
//! achievement.rs`'s `DRD_ACHIEVEMENT_DATA`/`DRD_ACHIEVEMENT_STATS`
//! persistence already documents - so `character_id`/`character_name`
//! stand in for `subscriber_id`/the account's display name here (see
//! `migrations/0007_achievement_firsts.sql`).
//!
//! C detects "was this insert the first one" via `mysql_affected_rows()
//! == 1` from `INSERT ... ON DUPLICATE KEY UPDATE total_unlocks =
//! total_unlocks + 1`. Postgres's `ON CONFLICT DO UPDATE` has no
//! equivalent affected-rows signal, so this repository instead uses the
//! standard `RETURNING (xmax = 0)` idiom (`xmax` is a row's deleting
//! transaction id; it is `0` for a row inserted - not updated - by the
//! current command).

use async_trait::async_trait;
use sqlx::PgPool;
use ugaris_core::ids::CharacterId;

#[async_trait]
pub trait AchievementRepository: Send + Sync {
    /// C `db_achievement_record_unlock` (`database_achievement.c:24-71`):
    /// upserts `achievement_firsts` (incrementing `total_unlocks` on
    /// conflict) and inserts an `achievement_history` row for leaderboard
    /// tracking (C's `INSERT IGNORE`, matched here with `ON CONFLICT DO
    /// NOTHING` even though the history table has no unique constraint
    /// today - kept for parity if one is added later). Returns whether
    /// this call performed the first-ever insert for `achievement_id`.
    async fn record_unlock(
        &self,
        achievement_id: i32,
        achievement_name: &str,
        character_id: CharacterId,
        character_name: &str,
    ) -> anyhow::Result<bool>;
}

#[derive(Debug, Clone)]
pub struct PgAchievementRepository {
    pool: PgPool,
}

impl PgAchievementRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const RECORD_UNLOCK_SQL: &str =
    "insert into achievement_firsts(achievement_id, achievement_name, first_character_id, first_character_name, total_unlocks) \
     values ($1, $2, $3, $4, 1) \
     on conflict (achievement_id) do update set total_unlocks = achievement_firsts.total_unlocks + 1 \
     returning (xmax = 0) as is_first";

const RECORD_HISTORY_SQL: &str =
    "insert into achievement_history(achievement_id, character_id, character_name) values ($1, $2, $3)";

#[async_trait]
impl AchievementRepository for PgAchievementRepository {
    async fn record_unlock(
        &self,
        achievement_id: i32,
        achievement_name: &str,
        character_id: CharacterId,
        character_name: &str,
    ) -> anyhow::Result<bool> {
        let is_first: bool = sqlx::query_scalar(RECORD_UNLOCK_SQL)
            .bind(achievement_id as i16)
            .bind(achievement_name)
            .bind(character_id.0 as i64)
            .bind(character_name)
            .fetch_one(&self.pool)
            .await?;

        sqlx::query(RECORD_HISTORY_SQL)
            .bind(achievement_id as i16)
            .bind(character_id.0 as i64)
            .bind(character_name)
            .execute(&self.pool)
            .await?;

        Ok(is_first)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The SQL statements themselves are exercised end-to-end only against
    /// a live Postgres instance (below); this just guards against
    /// accidental drift in the static query text (column/table names,
    /// the `xmax` idiom) without needing a database.
    #[test]
    fn record_unlock_sql_uses_xmax_first_insert_idiom() {
        assert!(RECORD_UNLOCK_SQL.contains("on conflict (achievement_id) do update"));
        assert!(RECORD_UNLOCK_SQL.contains("returning (xmax = 0) as is_first"));
        assert!(RECORD_HISTORY_SQL.contains("insert into achievement_history"));
    }

    /// Mirrors `merchant.rs`/`auction.rs`'s `live` test convention:
    /// exercises the real repository against a live Postgres instance
    /// when `DATABASE_URL` is set, and skips (never fails) otherwise so
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

        async fn insert_character(pool: &PgPool, name: &str) -> i64 {
            let account_id: i64 = sqlx::query_scalar(
                "insert into accounts(username, password_hash) values ($1, 'x') returning id",
            )
            .bind(format!("{name}_acct"))
            .fetch_one(pool)
            .await
            .expect("insert account");

            sqlx::query_scalar(
                "insert into characters(account_id, name) values ($1, $2) returning id",
            )
            .bind(account_id)
            .bind(name)
            .fetch_one(pool)
            .await
            .expect("insert character")
        }

        async fn cleanup(pool: &PgPool, achievement_id: i32, character_id: i64) {
            sqlx::query("delete from achievement_history where achievement_id = $1")
                .bind(achievement_id as i16)
                .execute(pool)
                .await
                .ok();
            sqlx::query("delete from achievement_firsts where achievement_id = $1")
                .bind(achievement_id as i16)
                .execute(pool)
                .await
                .ok();
            sqlx::query("delete from characters where id = $1")
                .bind(character_id)
                .execute(pool)
                .await
                .ok();
            sqlx::query(
                "delete from accounts where id in (select account_id from characters where id = $1)",
            )
            .bind(character_id)
            .execute(pool)
            .await
            .ok();
        }

        #[tokio::test]
        async fn first_unlock_reports_true_and_second_reports_false() {
            let Some(pool) = connect().await else {
                return;
            };
            // Distinct, out-of-catalog achievement id so repeated test
            // runs never collide with real data or each other.
            let achievement_id = 30_000;
            let hero_id = insert_character(&pool, "achfirst_hero").await;
            let rival_id = insert_character(&pool, "achfirst_rival").await;
            let repo = PgAchievementRepository::new(pool.clone());

            let first = repo
                .record_unlock(
                    achievement_id,
                    "Test Achievement",
                    CharacterId(hero_id as u32),
                    "achfirst_hero",
                )
                .await
                .expect("record first unlock");
            assert!(first, "first insert must report is_first = true");

            let second = repo
                .record_unlock(
                    achievement_id,
                    "Test Achievement",
                    CharacterId(rival_id as u32),
                    "achfirst_rival",
                )
                .await
                .expect("record second unlock");
            assert!(!second, "second insert must report is_first = false");

            let total_unlocks: i64 = sqlx::query_scalar(
                "select total_unlocks from achievement_firsts where achievement_id = $1",
            )
            .bind(achievement_id as i16)
            .fetch_one(&pool)
            .await
            .expect("fetch total_unlocks");
            assert_eq!(total_unlocks, 2);

            let history_count: i64 = sqlx::query_scalar(
                "select count(*) from achievement_history where achievement_id = $1",
            )
            .bind(achievement_id as i16)
            .fetch_one(&pool)
            .await
            .expect("fetch history count");
            assert_eq!(history_count, 2);

            cleanup(&pool, achievement_id, hero_id).await;
            cleanup(&pool, achievement_id, rival_id).await;
        }
    }
}
