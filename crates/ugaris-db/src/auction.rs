//! Auction house storage.
//!
//! Ports the database layer of `src/system/auction/auction_db.c`
//! (slice 2 of the aclerk/auction task - see `PORTING_TODO.md`). The
//! business logic (`auction_house.c`: fee/bid math, the `/ah` text
//! command state machine in `auction_cmd.c`, and the mod-GUI packet
//! formatting in `auction_client.c`) is not ported yet; this module only
//! covers the CRUD/search operations C exposes through `auction_db.h` so a
//! future slice can build the `auction_create`/`auction_bid`/
//! `auction_buyout`/`auction_cancel`/`/ah` command flow on top of it,
//! matching how `merchant.rs` predates `world/merchant.rs`'s driver logic.
//!
//! C stores each auctioned item as a raw `struct item` BLOB and filters/
//! sorts on fields inside it via `CAST(SUBSTRING(...))` at byte offsets
//! computed with `offsetof`; Rust instead stores the item as `jsonb` (same
//! convention as `merchant.rs`'s `wares_json`) and filters/sorts on its
//! `name`/`min_level`/`max_level` keys directly - simpler and immune to
//! struct-layout drift. `item_template` is kept as its own column (like C)
//! purely so future code can index/browse by template without touching
//! the JSON body.
//!
//! `db_get_character_name` (`auction_db.h`) is not ported: grepping the
//! full C tree shows it is declared and defined but never called anywhere
//! (dead code, presumably left over from an earlier mod-GUI iteration).
//! Everywhere else the C code needs a seller name it fetches it via the
//! `LEFT JOIN chars c ON a.seller_id = c.ID` already inlined into
//! `db_get_auction`/`db_search_auctions`/`db_get_player_auctions`, which
//! this port replicates by joining Postgres's `characters` table in the
//! equivalent queries below.

use async_trait::async_trait;
use sqlx::{types::Json, PgPool, Row};
use ugaris_core::{entity::Item, ids::CharacterId};

/// C `enum` values baked into the `auction_items.status` MySQL `ENUM`
/// (`auction_db.c::init_auction_database`) - kept as `text` + a `check`
/// constraint in Postgres (see `migrations/0006_auction_house.sql`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuctionStatus {
    Active,
    Sold,
    Expired,
    Cancelled,
}

impl AuctionStatus {
    fn as_db_str(self) -> &'static str {
        match self {
            AuctionStatus::Active => "active",
            AuctionStatus::Sold => "sold",
            AuctionStatus::Expired => "expired",
            AuctionStatus::Cancelled => "cancelled",
        }
    }

    fn from_db_str(value: &str) -> Self {
        match value {
            "sold" => AuctionStatus::Sold,
            "expired" => AuctionStatus::Expired,
            "cancelled" => AuctionStatus::Cancelled,
            _ => AuctionStatus::Active,
        }
    }
}

/// C `AUCTION_REASON_*` string constants (`auction_data.h`), used verbatim
/// as the `auction_deliveries.reason` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryReason {
    Won,
    Expired,
    Cancelled,
    Sold,
    Outbid,
}

impl DeliveryReason {
    fn as_db_str(self) -> &'static str {
        match self {
            DeliveryReason::Won => "won",
            DeliveryReason::Expired => "expired",
            DeliveryReason::Cancelled => "cancelled",
            DeliveryReason::Sold => "sold",
            DeliveryReason::Outbid => "outbid",
        }
    }

    fn from_db_str(value: &str) -> Self {
        match value {
            "expired" => DeliveryReason::Expired,
            "cancelled" => DeliveryReason::Cancelled,
            "sold" => DeliveryReason::Sold,
            "outbid" => DeliveryReason::Outbid,
            _ => DeliveryReason::Won,
        }
    }
}

/// C `AH_SORT_*` constants used by `db_search_auctions`'s `ORDER BY`
/// switch (`auction_db.c:233-260`). The numeric comment values in C are
/// preserved in this enum's doc order (0-5); anything else falls back to
/// `TimeLeft`, matching C's `default:` case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AuctionSortBy {
    /// AH_SORT_TIME_LEFT (0, default)
    #[default]
    TimeLeft,
    /// AH_SORT_PRICE_LOW (1)
    PriceLow,
    /// AH_SORT_PRICE_HIGH (2)
    PriceHigh,
    /// AH_SORT_LEVEL_LOW (3)
    LevelLow,
    /// AH_SORT_LEVEL_HIGH (4)
    LevelHigh,
    /// AH_SORT_NAME (5)
    Name,
}

/// C `struct auction_filter` (`auction_data.h`).
#[derive(Debug, Clone, Default)]
pub struct AuctionFilter {
    pub name_pattern: Option<String>,
    pub min_level: Option<u8>,
    pub max_level: Option<u8>,
    pub sort_by: AuctionSortBy,
    pub offset: i64,
    pub limit: i64,
}

/// Fields needed to create a new auction, mirroring what
/// `auction_house.c::auction_create` fills into `struct auction` before
/// calling `db_create_auction`. `ends_at_unix` is precomputed by the
/// caller (`time(NULL) + duration` in C) rather than a duration, so this
/// DB layer stays free of wall-clock logic.
#[derive(Debug, Clone)]
pub struct NewAuction {
    pub seller_id: CharacterId,
    pub item_template: u32,
    pub item: Item,
    pub start_price: u64,
    pub buyout_price: Option<u64>,
    pub ends_at_unix: i64,
}

/// C `struct auction` (`auction_data.h`), as read back from storage.
#[derive(Debug, Clone)]
pub struct AuctionRecord {
    pub id: i64,
    pub seller_id: CharacterId,
    /// Populated from the `LEFT JOIN ... COALESCE(c.name, 'Unknown')`
    /// C always performs alongside auction reads.
    pub seller_name: String,
    pub item_template: u32,
    pub item: Item,
    pub start_price: u64,
    pub buyout_price: Option<u64>,
    pub current_bid: Option<u64>,
    pub current_bidder_id: Option<CharacterId>,
    pub created_at_unix: i64,
    pub ends_at_unix: i64,
    pub status: AuctionStatus,
}

/// C `struct auction_search_result` (`auction_data.h`).
#[derive(Debug, Clone, Default)]
pub struct AuctionSearchResult {
    pub auctions: Vec<AuctionRecord>,
    pub total_matches: i64,
}

/// C `struct auction_delivery` (`auction_data.h`).
#[derive(Debug, Clone)]
pub struct AuctionDelivery {
    pub id: i64,
    pub item: Option<Item>,
    pub gold_amount: u64,
    pub reason: DeliveryReason,
}

/// A new delivery to insert, mirroring `db_create_delivery`'s parameters.
#[derive(Debug, Clone)]
pub struct NewDelivery {
    pub character_id: CharacterId,
    pub item: Option<Item>,
    pub gold_amount: u64,
    pub reason: DeliveryReason,
}

/// C `db_get_delivery_summary`'s three out-parameters bundled together.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeliverySummary {
    pub pending_count: i64,
    pub total_gold: u64,
    pub has_items: bool,
}

/// C `MAX_SEARCH_RESULTS` (`auction_data.h`), used by
/// `db_get_player_auctions` to clamp the caller-supplied limit.
pub const MAX_SEARCH_RESULTS: i64 = 50;

#[async_trait]
pub trait AuctionRepository: Send + Sync {
    /// C `db_create_auction`. Returns the new auction's id (C never
    /// retrieves this via `LAST_INSERT_ID()` - grepping the C tree shows
    /// no caller uses the created auction's id after insert - but
    /// Postgres's `RETURNING` makes it free, so it's exposed here for a
    /// future business-logic slice to use).
    async fn create_auction(&self, new: &NewAuction) -> anyhow::Result<i64>;

    /// C `db_update_auction`: updates bid/bidder/status only.
    async fn update_auction(
        &self,
        id: i64,
        current_bid: Option<u64>,
        current_bidder_id: Option<CharacterId>,
        status: AuctionStatus,
    ) -> anyhow::Result<bool>;

    /// C `db_get_auction`.
    async fn get_auction(&self, id: i64) -> anyhow::Result<Option<AuctionRecord>>;

    /// C `db_delete_auction`.
    async fn delete_auction(&self, id: i64) -> anyhow::Result<bool>;

    /// C `db_search_auctions`. Unlike `db_get_player_auctions`, C does not
    /// clamp `filter.limit` itself here - that's left to the caller
    /// (`auction_house.c`/`auction_client.c` both pass fixed page sizes).
    async fn search_auctions(&self, filter: &AuctionFilter) -> anyhow::Result<AuctionSearchResult>;

    /// C `db_get_player_auctions`, including its `MAX_SEARCH_RESULTS`
    /// clamp on `limit`.
    async fn get_player_auctions(
        &self,
        seller_id: CharacterId,
        offset: i64,
        limit: i64,
    ) -> anyhow::Result<AuctionSearchResult>;

    /// C `db_count_active_auctions`.
    async fn count_active_auctions(&self, seller_id: CharacterId) -> anyhow::Result<i64>;

    /// C `db_create_delivery`. Returns the new delivery's id (same
    /// `RETURNING`-is-free rationale as `create_auction`).
    async fn create_delivery(&self, new: &NewDelivery) -> anyhow::Result<i64>;

    /// C `db_get_pending_deliveries`.
    async fn get_pending_deliveries(
        &self,
        character_id: CharacterId,
    ) -> anyhow::Result<Vec<AuctionDelivery>>;

    /// C `db_mark_delivery_claimed`.
    async fn mark_delivery_claimed(&self, delivery_id: i64) -> anyhow::Result<bool>;

    /// C `db_get_delivery_summary`.
    async fn get_delivery_summary(
        &self,
        character_id: CharacterId,
    ) -> anyhow::Result<DeliverySummary>;

    /// C `db_cleanup_expired_auctions`: for every `active` auction whose
    /// `ends_at` has passed, deliver the item+gold to the winner (or
    /// return the item to the seller if there was no bid) and mark the
    /// auction `sold`/`expired`. Returns the number of auctions processed
    /// (C's version is `void`; the count is exposed here purely so tests
    /// and future callers can assert something happened).
    async fn cleanup_expired_auctions(&self) -> anyhow::Result<usize>;
}

#[derive(Debug, Clone)]
pub struct PgAuctionRepository {
    pool: PgPool,
}

impl PgAuctionRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const CREATE_AUCTION_SQL: &str =
    "insert into auctions(seller_id, item_template, item_json, start_price, buyout_price, ends_at) \
     values ($1, $2, $3, $4, $5, to_timestamp($6)) returning id";

const UPDATE_AUCTION_SQL: &str =
    "update auctions set current_bid = $1, current_bidder_id = $2, status = $3 where id = $4";

const SELECT_AUCTION_COLUMNS: &str =
    "a.id, a.seller_id, a.item_template, a.item_json, a.start_price, a.buyout_price, \
     a.current_bid, a.current_bidder_id, extract(epoch from a.created_at)::bigint as created_at_unix, \
     extract(epoch from a.ends_at)::bigint as ends_at_unix, a.status, coalesce(c.name, 'Unknown') as seller_name";

fn get_auction_sql() -> String {
    format!(
        "select {SELECT_AUCTION_COLUMNS} from auctions a \
         left join characters c on a.seller_id = c.id where a.id = $1"
    )
}

const DELETE_AUCTION_SQL: &str = "delete from auctions where id = $1";

const COUNT_ACTIVE_AUCTIONS_SQL: &str =
    "select count(*) from auctions where seller_id = $1 and status = 'active'";

const CREATE_DELIVERY_SQL: &str =
    "insert into auction_deliveries(character_id, item_json, gold_amount, reason) \
     values ($1, $2, $3, $4) returning id";

const GET_PENDING_DELIVERIES_SQL: &str =
    "select id, item_json, gold_amount, reason from auction_deliveries \
     where character_id = $1 and claimed_at is null order by created_at asc";

const MARK_DELIVERY_CLAIMED_SQL: &str =
    "update auction_deliveries set claimed_at = now() where id = $1 and claimed_at is null";

const GET_DELIVERY_SUMMARY_SQL: &str = "select count(*), coalesce(sum(gold_amount), 0)::bigint, \
     coalesce(sum(case when item_json is not null then 1 else 0 end), 0)::bigint \
     from auction_deliveries where character_id = $1 and claimed_at is null";

const CLEANUP_SELECT_EXPIRED_SQL: &str =
    "select id, seller_id, item_json, current_bid, current_bidder_id from auctions \
     where status = 'active' and ends_at <= now()";

fn row_to_auction_record(row: &sqlx::postgres::PgRow) -> anyhow::Result<AuctionRecord> {
    let Json(item): Json<Item> = row.try_get("item_json")?;
    let status: String = row.try_get("status")?;
    let seller_name: String = row.try_get("seller_name")?;
    let current_bidder_id: Option<i64> = row.try_get("current_bidder_id")?;
    let buyout_price: Option<i64> = row.try_get("buyout_price")?;
    let current_bid: Option<i64> = row.try_get("current_bid")?;
    Ok(AuctionRecord {
        id: row.try_get("id")?,
        seller_id: CharacterId(row.try_get::<i64, _>("seller_id")? as u32),
        seller_name,
        item_template: row.try_get::<i64, _>("item_template")? as u32,
        item,
        start_price: row.try_get::<i64, _>("start_price")? as u64,
        buyout_price: buyout_price.map(|v| v as u64),
        current_bid: current_bid.map(|v| v as u64),
        current_bidder_id: current_bidder_id.map(|v| CharacterId(v as u32)),
        created_at_unix: row.try_get("created_at_unix")?,
        ends_at_unix: row.try_get("ends_at_unix")?,
        status: AuctionStatus::from_db_str(&status),
    })
}

#[async_trait]
impl AuctionRepository for PgAuctionRepository {
    async fn create_auction(&self, new: &NewAuction) -> anyhow::Result<i64> {
        let row = sqlx::query(CREATE_AUCTION_SQL)
            .bind(new.seller_id.0 as i64)
            .bind(new.item_template as i64)
            .bind(Json(&new.item))
            .bind(new.start_price as i64)
            .bind(new.buyout_price.map(|v| v as i64))
            .bind(new.ends_at_unix)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.try_get("id")?)
    }

    async fn update_auction(
        &self,
        id: i64,
        current_bid: Option<u64>,
        current_bidder_id: Option<CharacterId>,
        status: AuctionStatus,
    ) -> anyhow::Result<bool> {
        let result = sqlx::query(UPDATE_AUCTION_SQL)
            .bind(current_bid.map(|v| v as i64))
            .bind(current_bidder_id.map(|c| c.0 as i64))
            .bind(status.as_db_str())
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn get_auction(&self, id: i64) -> anyhow::Result<Option<AuctionRecord>> {
        let row = sqlx::query(&get_auction_sql())
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(row_to_auction_record).transpose()
    }

    async fn delete_auction(&self, id: i64) -> anyhow::Result<bool> {
        let result = sqlx::query(DELETE_AUCTION_SQL)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn search_auctions(&self, filter: &AuctionFilter) -> anyhow::Result<AuctionSearchResult> {
        let mut where_clause = "where a.status = 'active'".to_string();
        if let Some(pattern) = filter.name_pattern.as_deref().filter(|p| !p.is_empty()) {
            where_clause.push_str(&format!(
                " and a.item_json->>'name' ilike '%{}%'",
                pattern.replace('\'', "''")
            ));
        }
        if let Some(min_level) = filter.min_level {
            where_clause.push_str(&format!(
                " and (a.item_json->>'max_level')::int >= {min_level}"
            ));
        }
        if let Some(max_level) = filter.max_level {
            where_clause.push_str(&format!(
                " and (a.item_json->>'min_level')::int <= {max_level}"
            ));
        }

        let order_by = match filter.sort_by {
            AuctionSortBy::TimeLeft => "a.ends_at asc",
            AuctionSortBy::PriceLow => "coalesce(a.current_bid, a.start_price) asc",
            AuctionSortBy::PriceHigh => "coalesce(a.current_bid, a.start_price) desc",
            AuctionSortBy::LevelLow => "(a.item_json->>'min_level')::int asc, a.ends_at asc",
            AuctionSortBy::LevelHigh => "(a.item_json->>'min_level')::int desc, a.ends_at asc",
            AuctionSortBy::Name => "a.item_json->>'name' asc, a.ends_at asc",
        };

        let count_sql = format!("select count(*) from auctions a {where_clause}");
        let total_matches: i64 = sqlx::query_scalar(&count_sql).fetch_one(&self.pool).await?;

        let query_sql = format!(
            "select {SELECT_AUCTION_COLUMNS} from auctions a \
             left join characters c on a.seller_id = c.id \
             {where_clause} order by {order_by} limit $1 offset $2"
        );
        let rows = sqlx::query(&query_sql)
            .bind(filter.limit)
            .bind(filter.offset)
            .fetch_all(&self.pool)
            .await?;

        let auctions = rows
            .iter()
            .map(row_to_auction_record)
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(AuctionSearchResult {
            auctions,
            total_matches,
        })
    }

    async fn get_player_auctions(
        &self,
        seller_id: CharacterId,
        offset: i64,
        limit: i64,
    ) -> anyhow::Result<AuctionSearchResult> {
        let limit = limit.min(MAX_SEARCH_RESULTS);

        let total_matches: i64 = sqlx::query_scalar(
            "select count(*) from auctions where seller_id = $1 and status = 'active'",
        )
        .bind(seller_id.0 as i64)
        .fetch_one(&self.pool)
        .await?;

        if total_matches == 0 {
            return Ok(AuctionSearchResult {
                auctions: Vec::new(),
                total_matches: 0,
            });
        }

        let query_sql = format!(
            "select {SELECT_AUCTION_COLUMNS} from auctions a \
             left join characters c on a.seller_id = c.id \
             where a.seller_id = $1 and a.status = 'active' \
             order by a.ends_at asc limit $2 offset $3"
        );
        let rows = sqlx::query(&query_sql)
            .bind(seller_id.0 as i64)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        let auctions = rows
            .iter()
            .map(row_to_auction_record)
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(AuctionSearchResult {
            auctions,
            total_matches,
        })
    }

    async fn count_active_auctions(&self, seller_id: CharacterId) -> anyhow::Result<i64> {
        let count: i64 = sqlx::query_scalar(COUNT_ACTIVE_AUCTIONS_SQL)
            .bind(seller_id.0 as i64)
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    async fn create_delivery(&self, new: &NewDelivery) -> anyhow::Result<i64> {
        let row = sqlx::query(CREATE_DELIVERY_SQL)
            .bind(new.character_id.0 as i64)
            .bind(new.item.as_ref().map(Json))
            .bind(new.gold_amount as i64)
            .bind(new.reason.as_db_str())
            .fetch_one(&self.pool)
            .await?;
        Ok(row.try_get("id")?)
    }

    async fn get_pending_deliveries(
        &self,
        character_id: CharacterId,
    ) -> anyhow::Result<Vec<AuctionDelivery>> {
        let rows = sqlx::query(GET_PENDING_DELIVERIES_SQL)
            .bind(character_id.0 as i64)
            .fetch_all(&self.pool)
            .await?;

        rows.iter()
            .map(|row| {
                let item_json: Option<Json<Item>> = row.try_get("item_json")?;
                let reason: String = row.try_get("reason")?;
                Ok(AuctionDelivery {
                    id: row.try_get("id")?,
                    item: item_json.map(|Json(item)| item),
                    gold_amount: row.try_get::<i64, _>("gold_amount")? as u64,
                    reason: DeliveryReason::from_db_str(&reason),
                })
            })
            .collect()
    }

    async fn mark_delivery_claimed(&self, delivery_id: i64) -> anyhow::Result<bool> {
        let result = sqlx::query(MARK_DELIVERY_CLAIMED_SQL)
            .bind(delivery_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn get_delivery_summary(
        &self,
        character_id: CharacterId,
    ) -> anyhow::Result<DeliverySummary> {
        let row = sqlx::query(GET_DELIVERY_SUMMARY_SQL)
            .bind(character_id.0 as i64)
            .fetch_one(&self.pool)
            .await?;
        let pending_count: i64 = row.try_get(0)?;
        let total_gold: i64 = row.try_get(1)?;
        let has_items: i64 = row.try_get(2)?;
        Ok(DeliverySummary {
            pending_count,
            total_gold: total_gold as u64,
            has_items: has_items > 0,
        })
    }

    async fn cleanup_expired_auctions(&self) -> anyhow::Result<usize> {
        let rows = sqlx::query(CLEANUP_SELECT_EXPIRED_SQL)
            .fetch_all(&self.pool)
            .await?;

        let mut processed = 0usize;
        for row in &rows {
            let auction_id: i64 = row.try_get("id")?;
            let seller_id: i64 = row.try_get("seller_id")?;
            let item_json: Json<Item> = row.try_get("item_json")?;
            let current_bid: Option<i64> = row.try_get("current_bid")?;
            let winner_id: Option<i64> = row.try_get("current_bidder_id")?;

            if let Some(winner_id) = winner_id {
                self.create_delivery(&NewDelivery {
                    character_id: CharacterId(winner_id as u32),
                    item: Some(item_json.0.clone()),
                    gold_amount: 0,
                    reason: DeliveryReason::Won,
                })
                .await?;
                self.create_delivery(&NewDelivery {
                    character_id: CharacterId(seller_id as u32),
                    item: None,
                    gold_amount: current_bid.unwrap_or(0) as u64,
                    reason: DeliveryReason::Sold,
                })
                .await?;
                sqlx::query("update auctions set status = 'sold' where id = $1")
                    .bind(auction_id)
                    .execute(&self.pool)
                    .await?;
            } else {
                self.create_delivery(&NewDelivery {
                    character_id: CharacterId(seller_id as u32),
                    item: Some(item_json.0.clone()),
                    gold_amount: 0,
                    reason: DeliveryReason::Expired,
                })
                .await?;
                sqlx::query("update auctions set status = 'expired' where id = $1")
                    .bind(auction_id)
                    .execute(&self.pool)
                    .await?;
            }
            processed += 1;
        }

        Ok(processed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_item(name: &str, min_level: u8, max_level: u8) -> Item {
        Item {
            id: ugaris_core::ids::ItemId(1),
            name: name.to_string(),
            description: "A test item".to_string(),
            flags: ugaris_core::entity::ItemFlags::empty(),
            sprite: 1,
            value: 100,
            min_level,
            max_level,
            needs_class: 0,
            template_id: 7,
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

    #[test]
    fn auction_status_round_trips_through_db_strings() {
        for status in [
            AuctionStatus::Active,
            AuctionStatus::Sold,
            AuctionStatus::Expired,
            AuctionStatus::Cancelled,
        ] {
            assert_eq!(AuctionStatus::from_db_str(status.as_db_str()), status);
        }
    }

    #[test]
    fn auction_status_unknown_string_falls_back_to_active_like_c_default_case() {
        // C's db_update_auction ternary chain has no explicit "cancelled"
        // branch and its from-string counterpart isn't literally present,
        // but the round-trip convention here mirrors C's own
        // `auction.status == 0 ? "active" : ...` default-active shape for
        // any value it doesn't recognize.
        assert_eq!(AuctionStatus::from_db_str("garbage"), AuctionStatus::Active);
    }

    #[test]
    fn delivery_reason_round_trips_through_legacy_auction_reason_strings() {
        // Matches AUCTION_REASON_* string constants in auction_data.h
        // digit-for-digit.
        assert_eq!(DeliveryReason::Won.as_db_str(), "won");
        assert_eq!(DeliveryReason::Expired.as_db_str(), "expired");
        assert_eq!(DeliveryReason::Cancelled.as_db_str(), "cancelled");
        assert_eq!(DeliveryReason::Sold.as_db_str(), "sold");
        assert_eq!(DeliveryReason::Outbid.as_db_str(), "outbid");
    }

    #[test]
    fn max_search_results_matches_legacy_constant() {
        // auction_data.h: #define MAX_SEARCH_RESULTS 50
        assert_eq!(MAX_SEARCH_RESULTS, 50);
    }

    #[test]
    fn sample_item_round_trips_through_json_like_wares_json_does() {
        let item = sample_item("Sword of Testing", 10, 20);
        let encoded = serde_json::to_string(&item).expect("encode item");
        let decoded: Item = serde_json::from_str(&encoded).expect("decode item");
        assert_eq!(decoded.name, item.name);
        assert_eq!(decoded.min_level, item.min_level);
        assert_eq!(decoded.max_level, item.max_level);
        assert_eq!(decoded.template_id, item.template_id);
    }

    /// Mirrors `merchant.rs`'s `live` test convention: exercises the real
    /// repository against a live Postgres instance when `DATABASE_URL` is
    /// set, and skips (never fails) otherwise so `cargo test --workspace`
    /// stays green without Postgres present.
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

        async fn cleanup(pool: &PgPool, character_ids: &[i64]) {
            for id in character_ids {
                sqlx::query("delete from auctions where seller_id = $1 or current_bidder_id = $1")
                    .bind(id)
                    .execute(pool)
                    .await
                    .ok();
                sqlx::query("delete from auction_deliveries where character_id = $1")
                    .bind(id)
                    .execute(pool)
                    .await
                    .ok();
                sqlx::query("delete from characters where id = $1")
                    .bind(id)
                    .execute(pool)
                    .await
                    .ok();
                sqlx::query("delete from accounts where id in (select account_id from characters where id = $1)")
                    .bind(id)
                    .execute(pool)
                    .await
                    .ok();
            }
        }

        #[tokio::test]
        async fn create_get_update_and_delete_round_trip() {
            let Some(pool) = connect().await else {
                return;
            };
            let seller_id = insert_character(&pool, "auction_live_seller").await;
            let bidder_id = insert_character(&pool, "auction_live_bidder").await;
            let repo = PgAuctionRepository::new(pool.clone());

            let auction_id = repo
                .create_auction(&NewAuction {
                    seller_id: CharacterId(seller_id as u32),
                    item_template: 7,
                    item: sample_item("Live Test Sword", 5, 50),
                    start_price: 1000,
                    buyout_price: Some(5000),
                    ends_at_unix: 4_000_000_000,
                })
                .await
                .expect("create auction");

            let fetched = repo
                .get_auction(auction_id)
                .await
                .expect("get auction")
                .expect("auction exists");
            assert_eq!(fetched.seller_id, CharacterId(seller_id as u32));
            assert_eq!(fetched.seller_name, "auction_live_seller");
            assert_eq!(fetched.start_price, 1000);
            assert_eq!(fetched.buyout_price, Some(5000));
            assert_eq!(fetched.item.name, "Live Test Sword");
            assert!(matches!(fetched.status, AuctionStatus::Active));

            let updated = repo
                .update_auction(
                    auction_id,
                    Some(1500),
                    Some(CharacterId(bidder_id as u32)),
                    AuctionStatus::Active,
                )
                .await
                .expect("update auction");
            assert!(updated);

            let refetched = repo
                .get_auction(auction_id)
                .await
                .expect("get auction")
                .expect("auction exists");
            assert_eq!(refetched.current_bid, Some(1500));
            assert_eq!(
                refetched.current_bidder_id,
                Some(CharacterId(bidder_id as u32))
            );

            let active_count = repo
                .count_active_auctions(CharacterId(seller_id as u32))
                .await
                .expect("count active auctions");
            assert_eq!(active_count, 1);

            let deleted = repo
                .delete_auction(auction_id)
                .await
                .expect("delete auction");
            assert!(deleted);
            assert!(repo
                .get_auction(auction_id)
                .await
                .expect("get auction after delete")
                .is_none());

            cleanup(&pool, &[seller_id, bidder_id]).await;
        }

        #[tokio::test]
        async fn search_filters_by_name_and_level_and_sorts_by_price() {
            let Some(pool) = connect().await else {
                return;
            };
            let seller_id = insert_character(&pool, "auction_live_search_seller").await;
            let repo = PgAuctionRepository::new(pool.clone());

            let cheap_id = repo
                .create_auction(&NewAuction {
                    seller_id: CharacterId(seller_id as u32),
                    item_template: 1,
                    item: sample_item("Zzz Cheap Blade", 1, 10),
                    start_price: 100,
                    buyout_price: None,
                    ends_at_unix: 4_000_000_000,
                })
                .await
                .expect("create cheap auction");
            let expensive_id = repo
                .create_auction(&NewAuction {
                    seller_id: CharacterId(seller_id as u32),
                    item_template: 1,
                    item: sample_item("Aaa Costly Blade", 1, 10),
                    start_price: 900,
                    buyout_price: None,
                    ends_at_unix: 4_000_000_000,
                })
                .await
                .expect("create expensive auction");

            let result = repo
                .search_auctions(&AuctionFilter {
                    name_pattern: Some("Blade".to_string()),
                    min_level: None,
                    max_level: None,
                    sort_by: AuctionSortBy::PriceLow,
                    offset: 0,
                    limit: 10,
                })
                .await
                .expect("search auctions");

            assert!(result.total_matches >= 2);
            let ids: Vec<i64> = result.auctions.iter().map(|a| a.id).collect();
            let cheap_pos = ids
                .iter()
                .position(|&id| id == cheap_id)
                .expect("cheap present");
            let expensive_pos = ids
                .iter()
                .position(|&id| id == expensive_id)
                .expect("expensive present");
            assert!(cheap_pos < expensive_pos, "expected ascending price order");

            sqlx::query("delete from auctions where id = any($1)")
                .bind(vec![cheap_id, expensive_id])
                .execute(&pool)
                .await
                .ok();
            cleanup(&pool, &[seller_id]).await;
        }

        #[tokio::test]
        async fn deliveries_round_trip_and_summary_matches_pending_rows() {
            let Some(pool) = connect().await else {
                return;
            };
            let character_id = insert_character(&pool, "auction_live_delivery").await;
            let repo = PgAuctionRepository::new(pool.clone());

            let gold_delivery_id = repo
                .create_delivery(&NewDelivery {
                    character_id: CharacterId(character_id as u32),
                    item: None,
                    gold_amount: 250,
                    reason: DeliveryReason::Sold,
                })
                .await
                .expect("create gold delivery");
            let item_delivery_id = repo
                .create_delivery(&NewDelivery {
                    character_id: CharacterId(character_id as u32),
                    item: Some(sample_item("Delivered Item", 1, 1)),
                    gold_amount: 0,
                    reason: DeliveryReason::Won,
                })
                .await
                .expect("create item delivery");

            let pending = repo
                .get_pending_deliveries(CharacterId(character_id as u32))
                .await
                .expect("get pending deliveries");
            assert_eq!(pending.len(), 2);

            let summary = repo
                .get_delivery_summary(CharacterId(character_id as u32))
                .await
                .expect("get delivery summary");
            assert_eq!(summary.pending_count, 2);
            assert_eq!(summary.total_gold, 250);
            assert!(summary.has_items);

            let claimed = repo
                .mark_delivery_claimed(gold_delivery_id)
                .await
                .expect("mark claimed");
            assert!(claimed);
            let claimed_again = repo
                .mark_delivery_claimed(gold_delivery_id)
                .await
                .expect("mark claimed again");
            assert!(
                !claimed_again,
                "already-claimed delivery should not re-claim"
            );

            let pending_after = repo
                .get_pending_deliveries(CharacterId(character_id as u32))
                .await
                .expect("get pending deliveries after claim");
            assert_eq!(pending_after.len(), 1);
            assert_eq!(pending_after[0].id, item_delivery_id);

            cleanup(&pool, &[character_id]).await;
        }

        #[tokio::test]
        async fn cleanup_expired_auctions_delivers_to_winner_and_returns_unsold_items() {
            let Some(pool) = connect().await else {
                return;
            };
            let seller_id = insert_character(&pool, "auction_live_cleanup_seller").await;
            let bidder_id = insert_character(&pool, "auction_live_cleanup_bidder").await;
            let repo = PgAuctionRepository::new(pool.clone());

            let won_id = repo
                .create_auction(&NewAuction {
                    seller_id: CharacterId(seller_id as u32),
                    item_template: 1,
                    item: sample_item("Won Item", 1, 1),
                    start_price: 100,
                    buyout_price: None,
                    ends_at_unix: 1, // already in the past
                })
                .await
                .expect("create won auction");
            repo.update_auction(
                won_id,
                Some(500),
                Some(CharacterId(bidder_id as u32)),
                AuctionStatus::Active,
            )
            .await
            .expect("bid on won auction");

            let expired_id = repo
                .create_auction(&NewAuction {
                    seller_id: CharacterId(seller_id as u32),
                    item_template: 1,
                    item: sample_item("Unsold Item", 1, 1),
                    start_price: 100,
                    buyout_price: None,
                    ends_at_unix: 1,
                })
                .await
                .expect("create expired auction");

            let processed = repo
                .cleanup_expired_auctions()
                .await
                .expect("cleanup expired auctions");
            assert!(processed >= 2);

            let won = repo
                .get_auction(won_id)
                .await
                .expect("get won auction")
                .expect("won auction still exists");
            assert!(matches!(won.status, AuctionStatus::Sold));

            let expired = repo
                .get_auction(expired_id)
                .await
                .expect("get expired auction")
                .expect("expired auction still exists");
            assert!(matches!(expired.status, AuctionStatus::Expired));

            let bidder_deliveries = repo
                .get_pending_deliveries(CharacterId(bidder_id as u32))
                .await
                .expect("get bidder deliveries");
            assert!(bidder_deliveries
                .iter()
                .any(|d| matches!(d.reason, DeliveryReason::Won) && d.item.is_some()));

            let seller_deliveries = repo
                .get_pending_deliveries(CharacterId(seller_id as u32))
                .await
                .expect("get seller deliveries");
            assert!(seller_deliveries
                .iter()
                .any(|d| matches!(d.reason, DeliveryReason::Sold) && d.gold_amount == 500));
            assert!(seller_deliveries
                .iter()
                .any(|d| matches!(d.reason, DeliveryReason::Expired) && d.item.is_some()));

            sqlx::query("delete from auctions where id = any($1)")
                .bind(vec![won_id, expired_id])
                .execute(&pool)
                .await
                .ok();
            cleanup(&pool, &[seller_id, bidder_id]).await;
        }
    }
}
