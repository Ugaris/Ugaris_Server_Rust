//! Clan activity log persistence.
//!
//! Ports `src/system/database/database_notes.c::add_clanlog`/
//! `lookup_clanlog`/`db_read_clanlog` (the append-only `clanlog` SQL
//! table backing `/clanlog`, `src/system/clanlog.c`) and `command.c`'s
//! `/clearclanlog` GM command. C keys entries by `(clan, serial)` pairs
//! so a deleted-and-refounded clan number's old entries still read back
//! as "Former clan N" (`db_read_clanlog`, `database_notes.c:308-312`)
//! rather than being misattributed to whatever refounded the slot -
//! matching that requires the caller to compare a returned entry's
//! `serial` against the *current* `ClanRegistry::serial(clan)`, which
//! only `crates/ugaris-server/src/clan_log.rs` (holding a live `&World`)
//! can do, so this repository just stores/returns the raw columns.
//!
//! `content` is plain UTF-8 text (C escapes/unescapes for MySQL string
//! literals via `escape_string`; Postgres parameter binding makes that
//! unnecessary here).

use async_trait::async_trait;
use sqlx::PgPool;
use ugaris_core::ids::CharacterId;

/// C's `LIMIT 51` (`lookup_clanlog`, `database_notes.c:141`): fetch one
/// more row than the 50 actually displayed so `db_read_clanlog` can tell
/// "more entries exist" apart from "exactly 50 entries exist" and print
/// the "Not all entries displayed. Use -s N to continue." hint off the
/// 51st row's timestamp.
pub const CLAN_LOG_FETCH_LIMIT: i64 = 51;
/// How many of the fetched rows are actually displayed
/// (`db_read_clanlog`'s `if (cnt > 50) { ...; break; }`,
/// `database_notes.c:322-327`).
pub const CLAN_LOG_DISPLAY_LIMIT: usize = 50;

/// One row of the C `clanlog` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClanLogEntry {
    pub time_unix: i64,
    pub clan: u16,
    pub serial: u32,
    /// C's `cID`; `0` for system-generated entries with no acting
    /// character (e.g. the daily relation-tick transitions,
    /// `clan.c:983` etc., which log with `cID=0`).
    pub character_id: CharacterId,
    pub prio: u8,
    pub content: String,
}

/// Mirrors `lookup_clanlog`'s (`database_notes.c:124-153`) dynamic
/// `WHERE` clause: a field set to `0` means "no filter on this column",
/// exactly like C's `if (clanNr)`/`if (serial)`/`if (coID)` guards (C's
/// `atoi`-parsed numbers can never legitimately be `0` for a real clan,
/// serial, or character id, so `0` is a safe "unset" sentinel on both
/// sides).
#[derive(Debug, Clone, Copy)]
pub struct ClanLogFilter {
    pub clan: u16,
    pub serial: u32,
    pub character_id: u32,
    /// C's `prio` is always applied (`if (prio)` - and `clanlog_prio`
    /// validates it into `1..=100`, so it is never actually `0` in
    /// practice); kept as a plain filter rather than `Option` to match.
    pub prio: u8,
    pub from_time: i64,
    pub to_time: i64,
}

#[async_trait]
pub trait ClanLogRepository: Send + Sync {
    /// C `add_clanlog` (`database_notes.c:74-104`).
    async fn add_entry(
        &self,
        clan: u16,
        serial: u32,
        character_id: CharacterId,
        prio: u8,
        content: &str,
        now_unix: i64,
    ) -> anyhow::Result<()>;

    /// C `lookup_clanlog` (`database_notes.c:124-153`), ordered by time
    /// ascending and capped at [`CLAN_LOG_FETCH_LIMIT`] rows, matching C
    /// exactly.
    async fn lookup(&self, filter: &ClanLogFilter) -> anyhow::Result<Vec<ClanLogEntry>>;

    /// C `/clearclanlog`'s `DELETE FROM clanlog WHERE clan=%d`
    /// (`command.c:7550-7551`). Returns the number of rows deleted.
    async fn clear_clan(&self, clan: u16) -> anyhow::Result<u64>;
}

#[derive(Debug, Clone)]
pub struct PgClanLogRepository {
    pool: PgPool,
}

impl PgClanLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const ADD_ENTRY_SQL: &str =
    "insert into clan_log(created_at, clan, serial, character_id, prio, content) \
     values ($1, $2, $3, $4, $5, $6)";

const CLEAR_CLAN_SQL: &str = "delete from clan_log where clan = $1";

#[async_trait]
impl ClanLogRepository for PgClanLogRepository {
    async fn add_entry(
        &self,
        clan: u16,
        serial: u32,
        character_id: CharacterId,
        prio: u8,
        content: &str,
        now_unix: i64,
    ) -> anyhow::Result<()> {
        sqlx::query(ADD_ENTRY_SQL)
            .bind(now_unix)
            .bind(clan as i16)
            .bind(serial as i64)
            .bind(character_id.0 as i64)
            .bind(prio as i16)
            .bind(content)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn lookup(&self, filter: &ClanLogFilter) -> anyhow::Result<Vec<ClanLogEntry>> {
        // C builds these as literal-integer clauses appended to the SQL
        // string (`lookup_clanlog`, `database_notes.c:132-146`); binding
        // as parameters instead is behavior-identical and avoids
        // reformatting untrusted input (these are all already-validated
        // integers here, but parameter binding is the established
        // convention in this crate for anything beyond the simplest
        // static queries). Placeholder numbers are assigned in the order
        // clauses are actually appended, since an omitted filter must not
        // leave a gap in the `$N` sequence.
        let mut where_clause =
            "where created_at >= $1 and created_at <= $2 and prio <= $3".to_string();
        let mut next_placeholder = 4;
        if filter.clan != 0 {
            where_clause.push_str(&format!(" and clan = ${next_placeholder}"));
            next_placeholder += 1;
        }
        if filter.serial != 0 {
            where_clause.push_str(&format!(" and serial = ${next_placeholder}"));
            next_placeholder += 1;
        }
        if filter.character_id != 0 {
            where_clause.push_str(&format!(" and character_id = ${next_placeholder}"));
        }

        let query_sql = format!(
            "select created_at, clan, serial, character_id, prio, content from clan_log \
             {where_clause} order by created_at asc limit {CLAN_LOG_FETCH_LIMIT}"
        );

        let mut query = sqlx::query_as::<_, (i64, i16, i64, i64, i16, String)>(&query_sql)
            .bind(filter.from_time)
            .bind(filter.to_time)
            .bind(filter.prio as i16);
        if filter.clan != 0 {
            query = query.bind(filter.clan as i16);
        }
        if filter.serial != 0 {
            query = query.bind(filter.serial as i64);
        }
        if filter.character_id != 0 {
            query = query.bind(filter.character_id as i64);
        }

        let rows = query.fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .map(
                |(time_unix, clan, serial, character_id, prio, content)| ClanLogEntry {
                    time_unix,
                    clan: clan as u16,
                    serial: serial as u32,
                    character_id: CharacterId(character_id as u32),
                    prio: prio as u8,
                    content,
                },
            )
            .collect())
    }

    async fn clear_clan(&self, clan: u16) -> anyhow::Result<u64> {
        let result = sqlx::query(CLEAR_CLAN_SQL)
            .bind(clan as i16)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_entry_sql_inserts_all_six_columns() {
        assert!(ADD_ENTRY_SQL.contains(
            "insert into clan_log(created_at, clan, serial, character_id, prio, content)"
        ));
        assert!(ADD_ENTRY_SQL.contains("values ($1, $2, $3, $4, $5, $6)"));
    }

    #[test]
    fn clear_clan_sql_deletes_by_clan_number() {
        assert_eq!(CLEAR_CLAN_SQL, "delete from clan_log where clan = $1");
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

        /// Uses a clan number far outside the legacy `MAXCLAN=32` range
        /// so this test can never collide with real clan-log rows or
        /// other tests sharing the database.
        const TEST_CLAN: u16 = 9_001;

        #[tokio::test]
        async fn add_then_lookup_round_trips_an_entry() {
            let Some(pool) = connect().await else {
                return;
            };
            let repo = PgClanLogRepository::new(pool.clone());
            repo.clear_clan(TEST_CLAN).await.ok();

            repo.add_entry(
                TEST_CLAN,
                42,
                CharacterId(7),
                1,
                "Clan was founded by Tester",
                1_000,
            )
            .await
            .expect("add entry");

            let entries = repo
                .lookup(&ClanLogFilter {
                    clan: TEST_CLAN,
                    serial: 0,
                    character_id: 0,
                    prio: 20,
                    from_time: 0,
                    to_time: 2_000,
                })
                .await
                .expect("lookup");

            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].clan, TEST_CLAN);
            assert_eq!(entries[0].serial, 42);
            assert_eq!(entries[0].character_id, CharacterId(7));
            assert_eq!(entries[0].prio, 1);
            assert_eq!(entries[0].content, "Clan was founded by Tester");
            assert_eq!(entries[0].time_unix, 1_000);

            repo.clear_clan(TEST_CLAN).await.ok();
        }

        #[tokio::test]
        async fn lookup_respects_the_priority_filter() {
            let Some(pool) = connect().await else {
                return;
            };
            let repo = PgClanLogRepository::new(pool.clone());
            repo.clear_clan(TEST_CLAN).await.ok();

            repo.add_entry(TEST_CLAN, 1, CharacterId(1), 1, "important", 1_000)
                .await
                .expect("add important entry");
            repo.add_entry(TEST_CLAN, 1, CharacterId(1), 50, "trivial", 1_000)
                .await
                .expect("add trivial entry");

            let entries = repo
                .lookup(&ClanLogFilter {
                    clan: TEST_CLAN,
                    serial: 0,
                    character_id: 0,
                    prio: 20,
                    from_time: 0,
                    to_time: 2_000,
                })
                .await
                .expect("lookup");

            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].content, "important");

            repo.clear_clan(TEST_CLAN).await.ok();
        }

        #[tokio::test]
        async fn clear_clan_deletes_all_rows_for_that_clan_only() {
            let Some(pool) = connect().await else {
                return;
            };
            let repo = PgClanLogRepository::new(pool.clone());
            const OTHER_CLAN: u16 = 9_002;
            repo.clear_clan(TEST_CLAN).await.ok();
            repo.clear_clan(OTHER_CLAN).await.ok();

            repo.add_entry(TEST_CLAN, 1, CharacterId(1), 1, "entry", 1_000)
                .await
                .expect("add entry");
            repo.add_entry(OTHER_CLAN, 1, CharacterId(1), 1, "other entry", 1_000)
                .await
                .expect("add other entry");

            let deleted = repo.clear_clan(TEST_CLAN).await.expect("clear clan");
            assert_eq!(deleted, 1);

            let remaining = repo
                .lookup(&ClanLogFilter {
                    clan: OTHER_CLAN,
                    serial: 0,
                    character_id: 0,
                    prio: 100,
                    from_time: 0,
                    to_time: 2_000,
                })
                .await
                .expect("lookup other clan");
            assert_eq!(remaining.len(), 1);

            repo.clear_clan(OTHER_CLAN).await.ok();
        }
    }
}
