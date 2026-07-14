//! Auction house business logic and the `/ah` text command.
//!
//! Ports `src/system/auction/auction_house.c` (fee/bid math, delivery
//! claim processing) and `src/system/auction/auction_cmd.c` (the `/ah`
//! text command state machine), on top of the already-ported DB layer in
//! `ugaris_db::auction` (slice 2 of this task, see `PORTING_LEDGER.md`).
//! `src/system/auction/auction_client.c`'s `CL_MOD3`/`SV_MOD3` mod-GUI
//! packet protocol remains N/A: the community client's `amod.c` only ever
//! handles `SV_MOD1`, never `SV_MOD3`, so no client exists to receive it.
//!
//! Unlike merchant stores (`world/merchant.rs`), auctions have **no**
//! in-memory `World` representation at all - every operation reads/writes
//! `PgAuctionRepository` directly, matching the DB layer's own design
//! rationale ("all-DB backed, immune to struct-layout drift"). This means
//! `/ah` commands are unavailable when the server is started without
//! `--database-url`, unlike merchant/bank/trader which keep functioning
//! (just without persistence) - documented gap, matches the DB-only
//! nature of the feature.
//!
//! C's `auction_house.c` business-logic functions (`auction_bid`,
//! `auction_buyout`, `auction_cancel`) call `log_char` directly for most
//! error cases, and `auction_cmd.c`'s command wrappers *also* log a
//! (usually near-duplicate) message from their own `switch` on the
//! returned status code - a player hitting e.g. "bid on your own auction"
//! in C sees two lines back to back. This port keeps only one message per
//! error, picking whichever of the two C messages carries the most
//! specific information (e.g. `auction_bid`'s exact minimum-bid amount
//! over `cmd_auction_bid`'s generic "5% increment" text) rather than
//! literally duplicating every line.
//!
//! One further gap remains, tracked in `PORTING_TODO.md`:
//! `auction_check_deliveries_login` (a login-time "you have N deliveries
//! waiting" notice) is not wired to the existing-but-unused
//! `PlayerRuntime::deferred_init`/`DEFERRED_AUCTION` hook in
//! `ugaris-core`. `init_auction_house`/`update_auction_house`/
//! `shutdown_auction_house` (`auction_house.c:37-52,1050-1052,1330-1340`)
//! *are* wired, in `main.rs`, to a startup sweep, a 60-real-second
//! periodic sweep (C's `maintenance_60s_task`), and a shutdown sweep,
//! respectively - all three are just `cleanup_expired_auctions` calls at
//! different points in the server lifecycle.

use super::*;

use ugaris_db::{
    AuctionFilter, AuctionRecord, AuctionRepository, AuctionSortBy, DeliveryReason, NewAuction,
    NewDelivery, PgAuctionRepository,
};

/// C `auction_data.h` constants.
const MAX_AUCTIONS_PER_PLAYER: i64 = 50;
const MIN_AUCTION_DURATION_SECONDS: i64 = 60 * 60;
const MAX_AUCTION_DURATION_SECONDS: i64 = 7 * 24 * 60 * 60;
const AUCTION_FEE_PERCENTAGE: u64 = 5;
const MIN_BID_INCREMENT_PERCENTAGE: u64 = 5;

/// C `auction_house.h`'s `enum auction_status` error codes (`AUCTION_SUCCESS`
/// is represented as `Ok(())`/`Ok(value)` on the Rust side instead of a
/// variant, so only the error codes 1-10 are modeled here).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AuctionError {
    Internal = 1,
    InvalidItem = 2,
    TooManyAuctions = 3,
    InsufficientFunds = 4,
    AuctionEnded = 5,
    BidTooLow = 6,
    CantBidOwn = 7,
    NotFound = 8,
    InvalidDuration = 9,
    InvalidPrice = 10,
}

/// C `format_money_string`/`auction_cmd.c`'s local `format_money` (the two
/// were byte-for-byte identical: gold+silver above 99, plain silver
/// otherwise).
pub(crate) fn format_money(amount: u64) -> String {
    let gold = amount / 100;
    let silver = amount % 100;
    if gold > 0 {
        format!("{gold} gold, {silver} silver")
    } else {
        format!("{silver} silver")
    }
}

/// C `validate_auction_item` (`auction_house.c:1275-1298`).
pub(crate) fn validate_auction_item(item: &Item) -> bool {
    if !item.flags.contains(ItemFlags::TAKE) {
        return false;
    }
    if item.flags.contains(ItemFlags::QUEST) {
        return false;
    }
    if item.flags.contains(ItemFlags::NODROP) {
        return false;
    }
    if item.flags.contains(ItemFlags::BONDTAKE) {
        return false;
    }
    if item.flags.contains(ItemFlags::LABITEM) {
        return false;
    }
    if item.flags.contains(ItemFlags::NODEPOT) {
        return false;
    }
    true
}

/// C `calculate_auction_fee` (`auction_house.c:1300-1304`): 5% of the
/// start price, floored at 100 (1 gold).
pub(crate) fn calculate_auction_fee(start_price: u64) -> u64 {
    let fee = start_price * AUCTION_FEE_PERCENTAGE / 100;
    fee.max(100)
}

/// C `auction_bid`'s inline minimum-bid computation
/// (`auction_house.c:261-278`), including its overflow-protected
/// increment (`saturating_add` here stands in for C's manual
/// `ULLONG_MAX` overflow check).
pub(crate) fn calculate_min_bid(current_bid: Option<u64>, start_price: u64) -> u64 {
    match current_bid {
        Some(bid) if bid > 0 => {
            let mut increment = bid / 100 * MIN_BID_INCREMENT_PERCENTAGE;
            if increment == 0 {
                increment = 1;
            }
            bid.saturating_add(increment)
        }
        _ => start_price,
    }
}

/// C `get_value_name` (`auction_house.c:512-643`)'s short lowercase
/// abbreviations, indexed by `V_*`/`CharacterValue` (0-42). Deliberately
/// separate from `entity::CHARACTER_VALUE_NAMES` (Title Case, full words)
/// which serves `legacy_item_look_text`'s different, more verbose display
/// convention.
const AUCTION_VALUE_ABBREV: [&str; ugaris_core::entity::CHARACTER_VALUE_COUNT] = [
    "hp",
    "end",
    "mana",
    "wis",
    "int",
    "agi",
    "str",
    "armor",
    "weapon",
    "light",
    "speed",
    "pulse",
    "dagger",
    "hand",
    "staff",
    "sword",
    "twohand",
    "armor skill",
    "attack",
    "parry",
    "warcry",
    "tactics",
    "surround",
    "bodycontrol",
    "speedskill",
    "barter",
    "perception",
    "stealth",
    "bless",
    "heal",
    "freeze",
    "m-shield",
    "flash",
    "fireball",
    "empty",
    "regen",
    "meditate",
    "immunity",
    "demon",
    "duration",
    "rage",
    "cold",
    "profession",
];

pub(crate) fn auction_value_name(index: i16) -> String {
    if index >= 0 && (index as usize) < AUCTION_VALUE_ABBREV.len() {
        AUCTION_VALUE_ABBREV[index as usize].to_string()
    } else {
        format!("mod{index}")
    }
}

/// C `format_item_modifiers` (`auction_house.c:655-713`): builds the
/// comma-separated requirement/modifier lists used by
/// `format_item_details`.
pub(crate) fn format_item_modifiers(item: &Item) -> (String, String) {
    let mut requirements = String::new();
    let mut modifiers = String::new();

    for (&index, &value) in item.modifier_index.iter().zip(item.modifier_value.iter()) {
        if index == 0 || value == 0 {
            continue;
        }
        if index < 0 {
            let name = auction_value_name(-index);
            if !requirements.is_empty() {
                requirements.push_str(", ");
            }
            requirements.push_str(&format!("{name} {value}"));
        } else {
            let name = auction_value_name(index);
            if !modifiers.is_empty() {
                modifiers.push_str(", ");
            }
            modifiers.push_str(&format!("+{value} {name}"));
        }
    }

    (requirements, modifiers)
}

/// C `format_item_details` (`auction_house.c:723-784`): a single colored
/// summary line combining name, requirements, modifiers, level, and
/// (conditionally) description. Returns raw legacy color-marker bytes
/// (`\xb0cN`), so callers must push it via a `message_bytes` channel, not
/// as a UTF-8 `String`.
pub(crate) fn format_item_details(item: &Item) -> Vec<u8> {
    let (requirements, modifiers) = format_item_modifiers(item);

    let color_name: &[u8] = if item.min_level >= 76 {
        COL_LIGHT_VIOLET
    } else if item.min_level >= 50 {
        COL_LIGHT_BLUE
    } else if item.min_level >= 30 {
        COL_LIGHT_GREEN
    } else if item.min_level >= 5 {
        COL_YELLOW
    } else {
        COL_RESET
    };

    let mut out = Vec::new();
    out.extend_from_slice(color_name);
    out.extend_from_slice(item.name.as_bytes());
    out.extend_from_slice(COL_RESET);

    if !requirements.is_empty() {
        out.extend_from_slice(b" - ");
        out.extend_from_slice(COL_LIGHT_RED);
        out.extend_from_slice(requirements.as_bytes());
        out.extend_from_slice(COL_RESET);
    }

    if !modifiers.is_empty() {
        out.extend_from_slice(b" (");
        out.extend_from_slice(COL_LIGHT_GREEN);
        out.extend_from_slice(modifiers.as_bytes());
        out.extend_from_slice(COL_RESET);
        out.extend_from_slice(b")");
    }

    if item.min_level > 0 {
        out.extend_from_slice(format!(". Level required: {}", item.min_level).as_bytes());
    }

    if !item.description.is_empty() && item.description != item.name {
        let has_reqs_or_mods = !requirements.is_empty() || !modifiers.is_empty();
        if !has_reqs_or_mods || item.description.len() < 20 {
            out.extend_from_slice(b" - ");
            out.extend_from_slice(item.description.as_bytes());
        }
    }

    out
}

/// C `format_time_left` (`auction_house.c:794-822`). Returns the
/// formatted text plus the legacy color marker C selects for it.
pub(crate) fn format_time_left(ends_at_unix: i64, now_unix: i64) -> (String, &'static [u8]) {
    if ends_at_unix > now_unix {
        let seconds_left = ends_at_unix - now_unix;
        let hours = seconds_left / 3600;
        let minutes = (seconds_left % 3600) / 60;
        if hours > 24 {
            (format!("{}d {}h", hours / 24, hours % 24), COL_VIOLET)
        } else if hours > 1 {
            (format!("{hours}h {minutes}m"), COL_LIGHT_GREEN)
        } else {
            (format!("{minutes}m"), COL_LIGHT_RED)
        }
    } else {
        ("Ended".to_string(), COL_DARK_GRAY)
    }
}

/// C `format_price` (`auction_house.c:832-861`).
pub(crate) fn format_price(auction: &AuctionRecord) -> (String, &'static [u8]) {
    let displayed = auction.current_bid.unwrap_or(auction.start_price);
    if let Some(buyout_price) = auction.buyout_price {
        (
            format!(
                "{} ({} buyout)",
                format_money(displayed),
                format_money(buyout_price)
            ),
            COL_YELLOW,
        )
    } else if auction
        .current_bid
        .is_some_and(|bid| bid > auction.start_price)
    {
        (format_money(displayed), COL_LIGHT_GREEN)
    } else {
        (format_money(displayed), COL_VIOLET)
    }
}

fn push_colored_line(out: &mut Vec<Vec<u8>>, color: &[u8], text_str: &str) {
    let mut line = Vec::with_capacity(color.len() + text_str.len() + COL_RESET.len());
    line.extend_from_slice(color);
    line.extend_from_slice(text_str.as_bytes());
    line.extend_from_slice(COL_RESET);
    out.push(line);
}

/// C `auction_create` (`auction_house.c:117-225`). Unlike C (which stores
/// the item BLOB via a `db_create_auction` call happening *before*
/// `consume_item`, wrapped in an explicit SQL transaction), this port
/// performs the DB insert first and only mutates `world` (fee deduction +
/// `World::destroy_item`, C's `consume_item`) after it succeeds - the
/// Postgres insert is the only step that can meaningfully fail, so the
/// ordering still guarantees the item is never lost.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn auction_create(
    repository: &PgAuctionRepository,
    world: &mut World,
    character_id: CharacterId,
    item_id: ItemId,
    start_price: u64,
    buyout_price: Option<u64>,
    duration_hours: u32,
    now_unix: i64,
) -> Result<(), AuctionError> {
    let item = {
        let character = world
            .characters
            .get(&character_id)
            .ok_or(AuctionError::Internal)?;
        if character.cursor_item != Some(item_id) {
            return Err(AuctionError::InvalidItem);
        }
        let item = world.items.get(&item_id).ok_or(AuctionError::InvalidItem)?;
        if item.carried_by != Some(character_id) {
            return Err(AuctionError::InvalidItem);
        }
        if !validate_auction_item(item) {
            return Err(AuctionError::InvalidItem);
        }
        item.clone()
    };

    let duration_seconds = i64::from(duration_hours) * 3600;
    if !(MIN_AUCTION_DURATION_SECONDS..=MAX_AUCTION_DURATION_SECONDS).contains(&duration_seconds) {
        return Err(AuctionError::InvalidDuration);
    }
    if let Some(buyout_price) = buyout_price {
        if buyout_price <= start_price {
            return Err(AuctionError::InvalidPrice);
        }
    }

    let active_count = repository
        .count_active_auctions(character_id)
        .await
        .map_err(|_| AuctionError::Internal)?;
    if active_count >= MAX_AUCTIONS_PER_PLAYER {
        return Err(AuctionError::TooManyAuctions);
    }

    let fee = calculate_auction_fee(start_price);
    let gold = u64::from(
        world
            .characters
            .get(&character_id)
            .ok_or(AuctionError::Internal)?
            .gold,
    );
    if gold < fee {
        return Err(AuctionError::InsufficientFunds);
    }

    repository
        .create_auction(&NewAuction {
            seller_id: character_id,
            item_template: item.template_id,
            item: item.clone(),
            start_price,
            buyout_price,
            ends_at_unix: now_unix + duration_seconds,
        })
        .await
        .map_err(|_| AuctionError::Internal)?;

    if let Some(character) = world.characters.get_mut(&character_id) {
        character.gold = character.gold.saturating_sub(fee as u32);
        character.flags.insert(CharacterFlags::ITEMS);
    }
    world.destroy_item(item_id);

    Ok(())
}

/// Outcome of a successful bid, so the caller can notify a previous
/// bidder (C `notify_outbid`, `auction_house.c:1306-1317`) if they're
/// online.
pub(crate) struct AuctionBidOutcome {
    pub(crate) auction_id: i64,
    pub(crate) previous_bidder: Option<CharacterId>,
}

/// C `auction_bid` (`auction_house.c:235-342`).
pub(crate) async fn auction_bid(
    repository: &PgAuctionRepository,
    world: &mut World,
    character_id: CharacterId,
    auction_id: i64,
    bid_amount: u64,
    now_unix: i64,
) -> Result<AuctionBidOutcome, AuctionError> {
    let auction = repository
        .get_auction(auction_id)
        .await
        .map_err(|_| AuctionError::Internal)?
        .ok_or(AuctionError::NotFound)?;

    if auction.ends_at_unix <= now_unix {
        return Err(AuctionError::AuctionEnded);
    }
    if auction.seller_id == character_id {
        return Err(AuctionError::CantBidOwn);
    }

    let min_bid = calculate_min_bid(auction.current_bid, auction.start_price);
    if bid_amount < min_bid {
        return Err(AuctionError::BidTooLow);
    }

    let gold = u64::from(
        world
            .characters
            .get(&character_id)
            .ok_or(AuctionError::Internal)?
            .gold,
    );
    if gold < bid_amount {
        return Err(AuctionError::InsufficientFunds);
    }

    let previous_bidder = auction.current_bidder_id;
    let previous_bid = auction.current_bid.unwrap_or(0);
    if let Some(previous_bidder) = previous_bidder {
        repository
            .create_delivery(&NewDelivery {
                character_id: previous_bidder,
                item: None,
                gold_amount: previous_bid,
                reason: DeliveryReason::Outbid,
            })
            .await
            .map_err(|_| AuctionError::Internal)?;
    }

    repository
        .update_auction(
            auction_id,
            Some(bid_amount),
            Some(character_id),
            ugaris_db::AuctionStatus::Active,
        )
        .await
        .map_err(|_| AuctionError::Internal)?;

    if let Some(character) = world.characters.get_mut(&character_id) {
        character.gold = character.gold.saturating_sub(bid_amount as u32);
        character.flags.insert(CharacterFlags::ITEMS);
    }

    Ok(AuctionBidOutcome {
        auction_id,
        previous_bidder,
    })
}

/// C `auction_buyout` (`auction_house.c:351-428`).
pub(crate) async fn auction_buyout(
    repository: &PgAuctionRepository,
    world: &mut World,
    character_id: CharacterId,
    auction_id: i64,
    now_unix: i64,
) -> Result<u64, AuctionError> {
    let auction = repository
        .get_auction(auction_id)
        .await
        .map_err(|_| AuctionError::Internal)?
        .ok_or(AuctionError::NotFound)?;

    if auction.ends_at_unix <= now_unix {
        return Err(AuctionError::AuctionEnded);
    }
    let Some(buyout_price) = auction.buyout_price else {
        return Err(AuctionError::InvalidPrice);
    };
    if auction.seller_id == character_id {
        return Err(AuctionError::CantBidOwn);
    }

    let gold = u64::from(
        world
            .characters
            .get(&character_id)
            .ok_or(AuctionError::Internal)?
            .gold,
    );
    if gold < buyout_price {
        return Err(AuctionError::InsufficientFunds);
    }

    repository
        .create_delivery(&NewDelivery {
            character_id: auction.seller_id,
            item: None,
            gold_amount: buyout_price,
            reason: DeliveryReason::Sold,
        })
        .await
        .map_err(|_| AuctionError::Internal)?;
    repository
        .create_delivery(&NewDelivery {
            character_id,
            item: Some(auction.item.clone()),
            gold_amount: 0,
            reason: DeliveryReason::Won,
        })
        .await
        .map_err(|_| AuctionError::Internal)?;

    repository
        .delete_auction(auction_id)
        .await
        .map_err(|_| AuctionError::Internal)?;

    if let Some(character) = world.characters.get_mut(&character_id) {
        character.gold = character.gold.saturating_sub(buyout_price as u32);
        character.flags.insert(CharacterFlags::ITEMS);
    }

    Ok(buyout_price)
}

/// C `auction_cancel` (`auction_house.c:437-493`).
pub(crate) async fn auction_cancel(
    repository: &PgAuctionRepository,
    character_id: CharacterId,
    auction_id: i64,
    now_unix: i64,
) -> Result<(), AuctionError> {
    let auction = repository
        .get_auction(auction_id)
        .await
        .map_err(|_| AuctionError::Internal)?
        .ok_or(AuctionError::NotFound)?;

    if auction.ends_at_unix <= now_unix {
        return Err(AuctionError::AuctionEnded);
    }
    if auction.seller_id != character_id {
        return Err(AuctionError::CantBidOwn);
    }

    repository
        .create_delivery(&NewDelivery {
            character_id,
            item: Some(auction.item.clone()),
            gold_amount: 0,
            reason: DeliveryReason::Cancelled,
        })
        .await
        .map_err(|_| AuctionError::Internal)?;

    if let Some(bidder_id) = auction.current_bidder_id {
        repository
            .create_delivery(&NewDelivery {
                character_id: bidder_id,
                item: None,
                gold_amount: auction.current_bid.unwrap_or(0),
                reason: DeliveryReason::Outbid,
            })
            .await
            .map_err(|_| AuctionError::Internal)?;
    }

    repository
        .delete_auction(auction_id)
        .await
        .map_err(|_| AuctionError::Internal)?;

    Ok(())
}

/// C `process_delivery`/`process_gold_delivery`/`process_item_delivery`
/// (`auction_house.c:1061-1204`), driven by `auction_claim_deliveries`
/// (`auction_house.c:967-988`). Gold is always credited (mirrors C
/// crediting `ch[cn].gold` directly, outside the DB transaction); an item
/// delivery is only marked claimed if it actually fit in the character's
/// inventory/cursor (C's `GIVE_ITEM_FULL` case leaves the delivery row
/// untouched so the player can retry after freeing space).
pub(crate) async fn auction_claim_deliveries(
    repository: &PgAuctionRepository,
    world: &mut World,
    character_id: CharacterId,
) -> Result<(usize, bool), AuctionError> {
    let deliveries = repository
        .get_pending_deliveries(character_id)
        .await
        .map_err(|_| AuctionError::Internal)?;

    let mut claimed = 0usize;
    let mut inventory_full = false;

    for delivery in deliveries {
        let mut ok = true;

        if delivery.gold_amount > 0 {
            if let Some(character) = world.characters.get_mut(&character_id) {
                character.gold = character.gold.saturating_add(delivery.gold_amount as u32);
                character.flags.insert(CharacterFlags::ITEMS);
            }
        }

        if let Some(mut item) = delivery.item {
            item.id = next_runtime_item_id(world);
            item.flags.insert(ItemFlags::USED);
            let Some(character) = world.characters.get_mut(&character_id) else {
                // Character vanished mid-claim (should not happen for a
                // command reachable only by an online player) - stop
                // processing further deliveries this call; already-claimed
                // ones stay claimed, the rest remain pending for retry.
                break;
            };
            let result = give_item_to_character(character, &mut item, GiveItemFlags::LOG);
            match result {
                GiveItemResult::Ok | GiveItemResult::Dropped => {
                    world.add_item(item);
                }
                GiveItemResult::Money => {}
                GiveItemResult::Full | GiveItemResult::Failed => {
                    ok = false;
                    inventory_full = matches!(result, GiveItemResult::Full);
                }
            }
        }

        if ok {
            repository
                .mark_delivery_claimed(delivery.id)
                .await
                .map_err(|_| AuctionError::Internal)?;
            claimed += 1;
        }
    }

    Ok((claimed, inventory_full))
}

/// C `auction_check_deliveries_login` (`auction_house.c:1206-1270`): a
/// login-time-only "you have N deliveries waiting" notice. `count`/
/// `total_gold`/`has_items` come straight from `db_get_delivery_summary`
/// (`DeliverySummary`); returns `None` when there is nothing to report
/// (matching C's function returning without ever calling `log_char`).
/// C's `total_gold >= 100` gold-vs-silver split is exactly
/// `format_money`'s own `gold > 0` branch, so it's reused here rather than
/// duplicated. Wired to `PlayerRuntime::deferred_init`'s `DEFERRED_AUCTION`
/// bit in `main.rs`'s game loop (C's `tick_player`, `player.c:3681-3684`).
pub(crate) fn format_auction_login_notice(summary: &ugaris_db::DeliverySummary) -> Option<Vec<u8>> {
    if summary.pending_count <= 0 {
        return None;
    }
    let count = summary.pending_count;
    let noun = if count == 1 { "delivery" } else { "deliveries" };
    let text = if summary.has_items && summary.total_gold > 0 {
        format!(
            "You have {count} auction {noun} waiting - items and {}. Type '/ah claim' to receive them.",
            format_money(summary.total_gold)
        )
    } else if summary.has_items {
        format!(
            "You have {count} auction {noun} with items waiting. Type '/ah claim' to receive them."
        )
    } else if summary.total_gold > 0 {
        format!(
            "You have {count} auction {noun} with {} waiting. Type '/ah claim' to receive them.",
            format_money(summary.total_gold)
        )
    } else {
        // C leaves `buf` uninitialized in this unreachable combination
        // (`count > 0` but neither items nor gold pending) and calls
        // `log_char` with garbage anyway; this port simply skips the
        // notice instead of replicating the undefined behavior.
        return None;
    };
    let mut out = Vec::with_capacity(COL_YELLOW.len() + text.len() + COL_RESET.len());
    out.extend_from_slice(COL_YELLOW);
    out.extend_from_slice(text.as_bytes());
    out.extend_from_slice(COL_RESET);
    Some(out)
}

/// Fetches the delivery summary and formats the login notice in one call,
/// for `main.rs`'s deferred-init sweep.
pub(crate) async fn auction_login_notice(
    repository: &PgAuctionRepository,
    character_id: CharacterId,
) -> anyhow::Result<Option<Vec<u8>>> {
    let summary = repository.get_delivery_summary(character_id).await?;
    Ok(format_auction_login_notice(&summary))
}

/// Result of `/ah search`/`/ah list` (C `auction_search`,
/// `auction_house.c:872-959`), rendered by the command layer into
/// colored `message_bytes` lines.
pub(crate) async fn auction_search(
    repository: &PgAuctionRepository,
    name: Option<&str>,
    min_level: u8,
    max_level: u8,
) -> Result<Vec<AuctionRecord>, AuctionError> {
    let filter = AuctionFilter {
        name_pattern: name.filter(|n| !n.is_empty()).map(str::to_string),
        min_level: (min_level > 0).then_some(min_level),
        max_level: (max_level > 0).then_some(max_level),
        sort_by: AuctionSortBy::default(),
        offset: 0,
        limit: ugaris_db::MAX_SEARCH_RESULTS,
    };
    let result = repository
        .search_auctions(&filter)
        .await
        .map_err(|_| AuctionError::Internal)?;
    Ok(result.auctions)
}

/// Renders the C `auction_search` listing text (`auction_house.c:894-958`)
/// as one `message_bytes` line per `log_char` call.
pub(crate) fn render_auction_search(auctions: &[AuctionRecord], now_unix: i64) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    push_colored_line(
        &mut out,
        COL_LIGHT_VIOLET,
        " ------------ AUCTION HOUSE ------------",
    );
    push_colored_line(
        &mut out,
        COL_VIOLET,
        &format!(" Found {} matching auctions:", auctions.len()),
    );

    if auctions.is_empty() {
        push_colored_line(
            &mut out,
            COL_LIGHT_RED,
            " No auctions match your search criteria.",
        );
        push_colored_line(
            &mut out,
            COL_LIGHT_VIOLET,
            " --------------------------------------",
        );
        return out;
    }

    for auction in auctions {
        push_colored_line(
            &mut out,
            COL_VIOLET,
            " --------------------------------------",
        );

        let mut id_line = Vec::new();
        id_line.extend_from_slice(COL_LIGHT_BLUE);
        id_line.extend_from_slice(format!("ID: {}", auction.id).as_bytes());
        id_line.extend_from_slice(COL_RESET);
        id_line.extend_from_slice(b" | ");
        id_line.extend_from_slice(&format_item_details(&auction.item));
        out.push(id_line);

        let (price_str, price_color) = format_price(auction);
        let mut price_line = Vec::new();
        price_line.extend_from_slice(b"Price: ");
        price_line.extend_from_slice(price_color);
        price_line.extend_from_slice(price_str.as_bytes());
        price_line.extend_from_slice(COL_RESET);
        out.push(price_line);

        let (time_str, time_color) = format_time_left(auction.ends_at_unix, now_unix);
        let mut time_line = Vec::new();
        time_line.extend_from_slice(b"Time left: ");
        time_line.extend_from_slice(time_color);
        time_line.extend_from_slice(time_str.as_bytes());
        time_line.extend_from_slice(COL_RESET);
        out.push(time_line);
    }

    push_colored_line(
        &mut out,
        COL_YELLOW,
        " --------------------------------------",
    );

    let mut legend = Vec::new();
    legend.extend_from_slice(COL_VIOLET);
    legend.extend_from_slice(b" Legend: ");
    legend.extend_from_slice(COL_LIGHT_RED);
    legend.extend_from_slice(b" Requirements");
    legend.extend_from_slice(COL_RESET);
    legend.extend_from_slice(b" | ");
    legend.extend_from_slice(COL_LIGHT_GREEN);
    legend.extend_from_slice(b" Modifiers");
    legend.extend_from_slice(COL_RESET);
    legend.extend_from_slice(b" | ");
    legend.extend_from_slice(COL_LIGHT_RED);
    legend.extend_from_slice(b" Ending Soon");
    legend.extend_from_slice(COL_RESET);
    legend.extend_from_slice(b" | ");
    legend.extend_from_slice(COL_LIGHT_GREEN);
    legend.extend_from_slice(b" Hours Remaining");
    legend.extend_from_slice(COL_RESET);
    legend.extend_from_slice(b" | ");
    legend.extend_from_slice(COL_VIOLET);
    legend.extend_from_slice(b" Days Remaining");
    legend.extend_from_slice(COL_RESET);
    out.push(legend);

    push_colored_line(
        &mut out,
        COL_LIGHT_BLUE,
        " Type '/ah info <ID>' for details, '/ah bid <ID> <amount>' to bid",
    );

    out
}

/// The `/ah help` text (C `cmd_auction_help`, `auction_cmd.c:50-60`).
pub(crate) fn auction_help_lines() -> Vec<String> {
    const COMMANDS: &[(&str, Option<&str>, &str)] = &[
        ("list", None, "List all your active auctions"),
        (
            "sell",
            Some("<start_price> [buyout_price] [duration]"),
            "Place an item up for auction. Duration in hours (default 24)",
        ),
        (
            "buy",
            Some("<auction_id>"),
            "Buy an item at its buyout price",
        ),
        (
            "bid",
            Some("<auction_id> <amount>"),
            "Place a bid on an auction",
        ),
        (
            "cancel",
            Some("<auction_id>"),
            "Cancel one of your auctions",
        ),
        (
            "search",
            Some("[name] [min_level] [max_level]"),
            "Search for items on auction",
        ),
        (
            "info",
            Some("<auction_id>"),
            "Get detailed info about an auction",
        ),
        (
            "claim",
            None,
            "Claim items and gold from completed auctions",
        ),
    ];

    let mut lines = vec!["Available auction commands:".to_string()];
    for (name, syntax, help) in COMMANDS {
        if let Some(syntax) = syntax {
            lines.push(format!("/ah {name} {syntax} - {help}"));
        } else {
            lines.push(format!("/ah {name} - {help}"));
        }
    }
    lines
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct AuctionCommandResult {
    pub(crate) messages: Vec<String>,
    pub(crate) message_bytes: Vec<Vec<u8>>,
    /// Outbid notification for a different, currently-online character
    /// (C `notify_outbid`).
    pub(crate) other_messages: Vec<(CharacterId, String)>,
    pub(crate) inventory_changed: bool,
}

fn simple_result(message: impl Into<String>) -> Option<AuctionCommandResult> {
    Some(AuctionCommandResult {
        messages: vec![message.into()],
        ..Default::default()
    })
}

/// C `auction_process_command`'s `/ah sell` handling
/// (`auction_cmd.c:70-160`).
async fn cmd_auction_sell(
    repository: &PgAuctionRepository,
    world: &mut World,
    character_id: CharacterId,
    args: &str,
    now_unix: i64,
) -> AuctionCommandResult {
    let cursor_item = world
        .characters
        .get(&character_id)
        .and_then(|character| character.cursor_item);
    let Some(item_id) = cursor_item else {
        return AuctionCommandResult {
            messages: vec!["Please put the item you want to sell on your cursor first.".to_string()],
            ..Default::default()
        };
    };

    let mut tokens = args.split_whitespace();
    let Some(price_token) = tokens.next() else {
        return AuctionCommandResult {
            messages: vec!["Usage: /ah sell <start_price> [buyout_price] [duration]".to_string()],
            ..Default::default()
        };
    };
    let buyout_token = tokens.next();
    let duration_token = tokens.next();

    let Some(start_price) = price_token.parse::<u64>().ok().map(|value| value * 100) else {
        return simple_result("Starting price must be greater than zero.").unwrap();
    };
    if start_price == 0 {
        return simple_result("Starting price must be greater than zero.").unwrap();
    }

    let buyout_price = match buyout_token {
        Some(token) => {
            let Some(buyout) = token.parse::<u64>().ok().map(|value| value * 100) else {
                return simple_result("Buyout price must be greater than starting price.").unwrap();
            };
            if buyout <= start_price {
                return simple_result("Buyout price must be greater than starting price.").unwrap();
            }
            Some(buyout)
        }
        None => None,
    };

    let max_duration_hours = MAX_AUCTION_DURATION_SECONDS / 3600;
    let duration = match duration_token {
        Some(token) => {
            let Ok(duration) = token.parse::<i64>() else {
                return simple_result(format!(
                    "Duration must be between 1 and {max_duration_hours} hours."
                ))
                .unwrap();
            };
            if !(1..=max_duration_hours).contains(&duration) {
                return simple_result(format!(
                    "Duration must be between 1 and {max_duration_hours} hours."
                ))
                .unwrap();
            }
            duration as u32
        }
        None => 24,
    };

    match auction_create(
        repository,
        world,
        character_id,
        item_id,
        start_price,
        buyout_price,
        duration,
        now_unix,
    )
    .await
    {
        Ok(()) => {
            let mut messages = vec![format!(
                "Item listed for auction starting at {} for {duration} hours.",
                format_money(start_price)
            )];
            if let Some(buyout_price) = buyout_price {
                messages.push(format!(
                    "Buyout price set to {}.",
                    format_money(buyout_price)
                ));
            }
            AuctionCommandResult {
                messages,
                inventory_changed: true,
                ..Default::default()
            }
        }
        Err(AuctionError::InvalidItem) => simple_result("This item cannot be auctioned.").unwrap(),
        Err(AuctionError::TooManyAuctions) => simple_result(format!(
            "You have too many active auctions (max {MAX_AUCTIONS_PER_PLAYER})."
        ))
        .unwrap(),
        Err(AuctionError::InsufficientFunds) => {
            // C recomputes the fee inline here without `calculate_auction_fee`'s
            // 100-copper floor - replicated verbatim (`auction_cmd.c:149-151`).
            let raw_fee = start_price * AUCTION_FEE_PERCENTAGE / 100;
            simple_result(format!(
                "You cannot afford the auction listing fee of {}.",
                format_money(raw_fee)
            ))
            .unwrap()
        }
        Err(_) => simple_result("Failed to create auction.").unwrap(),
    }
}

/// C `cmd_auction_buy` (`auction_cmd.c:162-205`).
async fn cmd_auction_buy(
    repository: &PgAuctionRepository,
    world: &mut World,
    character_id: CharacterId,
    args: &str,
    now_unix: i64,
) -> AuctionCommandResult {
    let Some(auction_id) = args.trim().parse::<i64>().ok().filter(|id| *id != 0) else {
        return simple_result("Invalid auction ID.").unwrap();
    };

    match auction_buyout(repository, world, character_id, auction_id, now_unix).await {
        Ok(_) => {
            let mut result = simple_result("Successfully bought out the auction.").unwrap();
            result.inventory_changed = true;
            result
        }
        Err(AuctionError::NotFound) => simple_result("Auction not found.").unwrap(),
        Err(AuctionError::AuctionEnded) => {
            simple_result("This auction has already ended.").unwrap()
        }
        Err(AuctionError::InsufficientFunds) => {
            simple_result("You cannot afford the buyout price.").unwrap()
        }
        Err(AuctionError::CantBidOwn) => {
            simple_result("You cannot buy out your own auction.").unwrap()
        }
        // C's `auction_buyout` (not `cmd_auction_buy`'s own switch, which
        // has no case for this and would otherwise fall through to the
        // generic message below) logs this specific text before returning
        // `AUCTION_ERROR_INVALID_PRICE`.
        Err(AuctionError::InvalidPrice) => {
            simple_result("This auction does not allow buyouts.").unwrap()
        }
        Err(_) => simple_result("Failed to buy out auction.").unwrap(),
    }
}

/// C `cmd_auction_bid` (`auction_cmd.c:207-260`).
async fn cmd_auction_bid(
    repository: &PgAuctionRepository,
    world: &mut World,
    character_id: CharacterId,
    args: &str,
    now_unix: i64,
) -> AuctionCommandResult {
    let mut tokens = args.split_whitespace();
    let (Some(id_token), Some(amount_token)) = (tokens.next(), tokens.next()) else {
        return simple_result("Usage: /ah bid <auction_id> <amount>").unwrap();
    };
    let Ok(auction_id) = id_token.parse::<i64>() else {
        return simple_result("Usage: /ah bid <auction_id> <amount>").unwrap();
    };
    let bid_amount = amount_token.parse::<u64>().unwrap_or(0) * 100;

    match auction_bid(
        repository,
        world,
        character_id,
        auction_id,
        bid_amount,
        now_unix,
    )
    .await
    {
        Ok(outcome) => {
            let mut result = simple_result(format!(
                "Successfully placed bid of {}.",
                format_money(bid_amount)
            ))
            .unwrap();
            if let Some(previous_bidder) = outcome.previous_bidder {
                result.other_messages.push((
                    previous_bidder,
                    format!("You have been outbid on auction #{}!", outcome.auction_id),
                ));
            }
            result
        }
        Err(AuctionError::NotFound) => simple_result("Auction not found.").unwrap(),
        Err(AuctionError::AuctionEnded) => {
            simple_result("This auction has already ended.").unwrap()
        }
        Err(AuctionError::BidTooLow) => {
            // C's `auction_bid` (not `cmd_auction_bid`'s own switch) logs
            // the exact minimum bid amount; re-fetch it here so the
            // consolidated single message keeps that more actionable
            // number instead of only the generic increment percentage.
            let min_bid = repository
                .get_auction(auction_id)
                .await
                .ok()
                .flatten()
                .map(|auction| calculate_min_bid(auction.current_bid, auction.start_price));
            let message = match min_bid {
                Some(min_bid) => format!("Minimum bid is {}.", format_money(min_bid)),
                None => format!(
                    "Your bid is too low. Minimum bid increment is {MIN_BID_INCREMENT_PERCENTAGE}%."
                ),
            };
            simple_result(message).unwrap()
        }
        Err(AuctionError::InsufficientFunds) => {
            simple_result("You cannot afford this bid.").unwrap()
        }
        Err(AuctionError::CantBidOwn) => {
            simple_result("You cannot bid on your own auction.").unwrap()
        }
        Err(_) => simple_result("Failed to place bid.").unwrap(),
    }
}

/// C `cmd_auction_cancel` (`auction_cmd.c:262-297`).
async fn cmd_auction_cancel(
    repository: &PgAuctionRepository,
    character_id: CharacterId,
    args: &str,
    now_unix: i64,
) -> AuctionCommandResult {
    let Some(auction_id) = args.trim().parse::<i64>().ok().filter(|id| *id != 0) else {
        return simple_result("Invalid auction ID.").unwrap();
    };

    match auction_cancel(repository, character_id, auction_id, now_unix).await {
        Ok(()) => simple_result("Successfully cancelled auction.").unwrap(),
        Err(AuctionError::NotFound) => simple_result("Auction not found.").unwrap(),
        Err(AuctionError::AuctionEnded) => {
            simple_result("This auction has already ended.").unwrap()
        }
        // C's `auction_cancel` (not `cmd_auction_cancel`'s own switch,
        // which has no case for this) logs this specific text before
        // returning `AUCTION_ERROR_CANT_BID_OWN` for a non-owner cancel.
        Err(AuctionError::CantBidOwn) => {
            simple_result("You can only cancel your own auctions.").unwrap()
        }
        Err(_) => simple_result("Failed to cancel auction.").unwrap(),
    }
}

/// C `cmd_auction_list` (`auction_cmd.c:299-303`): search with no
/// filters.
async fn cmd_auction_list(repository: &PgAuctionRepository, now_unix: i64) -> AuctionCommandResult {
    match auction_search(repository, None, 0, 0).await {
        Ok(auctions) => AuctionCommandResult {
            message_bytes: render_auction_search(&auctions, now_unix),
            ..Default::default()
        },
        Err(_) => simple_result("Failed to search auctions.").unwrap(),
    }
}

/// C `cmd_auction_search` (`auction_cmd.c:305-336`).
async fn cmd_auction_search(
    repository: &PgAuctionRepository,
    args: &str,
    now_unix: i64,
) -> AuctionCommandResult {
    let mut tokens = args.split_whitespace();
    let name = tokens.next();
    let min_level = tokens
        .next()
        .and_then(|token| token.parse::<u8>().ok())
        .unwrap_or(0);
    let max_level = tokens
        .next()
        .and_then(|token| token.parse::<u8>().ok())
        .unwrap_or(0);

    match auction_search(repository, name, min_level, max_level).await {
        Ok(auctions) => AuctionCommandResult {
            message_bytes: render_auction_search(&auctions, now_unix),
            ..Default::default()
        },
        Err(_) => simple_result("Failed to search auctions.").unwrap(),
    }
}

/// C `cmd_auction_info`/`auction_get_info` (`auction_cmd.c:338-359`,
/// `auction_house.c:997-1044`).
async fn cmd_auction_info(
    repository: &PgAuctionRepository,
    world: &World,
    character_id: CharacterId,
    args: &str,
    now_unix: i64,
) -> AuctionCommandResult {
    let Some(auction_id) = args.trim().parse::<i64>().ok().filter(|id| *id != 0) else {
        return simple_result("Invalid auction ID.").unwrap();
    };

    let auction = match repository.get_auction(auction_id).await {
        Ok(Some(auction)) => auction,
        Ok(None) => return simple_result("Auction not found.").unwrap(),
        Err(_) => return simple_result("Failed to get auction information.").unwrap(),
    };

    let mut messages = vec![format!("Auction #{auction_id}:")];
    if let Some(character) = world.characters.get(&character_id) {
        messages.extend(
            legacy_item_look_text(&auction.item, character)
                .lines()
                .map(str::to_string),
        );
    }
    messages.push(format!(
        "Starting Price: {}",
        format_money(auction.start_price)
    ));
    if let Some(buyout_price) = auction.buyout_price {
        messages.push(format!("Buyout Price: {}", format_money(buyout_price)));
    }
    if let Some(current_bid) = auction.current_bid {
        messages.push(format!("Current Bid: {}", format_money(current_bid)));
    }
    if auction.ends_at_unix > now_unix {
        let seconds_left = auction.ends_at_unix - now_unix;
        messages.push(format!(
            "Time Left: {} hours, {} minutes",
            seconds_left / 3600,
            (seconds_left % 3600) / 60
        ));
    } else {
        messages.push("Auction has ended".to_string());
    }

    AuctionCommandResult {
        messages,
        ..Default::default()
    }
}

/// C `cmd_auction_claim` (`auction_cmd.c:361-374`).
async fn cmd_auction_claim(
    repository: &PgAuctionRepository,
    world: &mut World,
    character_id: CharacterId,
) -> AuctionCommandResult {
    match auction_claim_deliveries(repository, world, character_id).await {
        Ok((0, inventory_full)) => {
            let message = if inventory_full {
                "Your inventory is full. Please make space and claim your delivery again."
                    .to_string()
            } else {
                "You have no pending deliveries.".to_string()
            };
            simple_result(message).unwrap()
        }
        Ok((count, _)) => AuctionCommandResult {
            messages: vec![format!(
                "Successfully claimed {count} {}.",
                if count == 1 { "delivery" } else { "deliveries" }
            )],
            inventory_changed: true,
            ..Default::default()
        },
        Err(_) => simple_result("Failed to claim deliveries.").unwrap(),
    }
}

/// C `auction_process_command` (`auction_cmd.c:377-447`), reached via
/// `command.c`'s `cmdcmp(ptr, "ah", 2)`/`cmdcmp(ptr, "auctionhouse", 10)`
/// dispatch (`command.c:10058-10082`).
pub(crate) async fn apply_auction_command(
    world: &mut World,
    repository: &Option<PgAuctionRepository>,
    character_id: CharacterId,
    now_unix: i64,
    command: &str,
) -> Option<AuctionCommandResult> {
    let (verb, rest) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let verb = verb
        .trim_start_matches('/')
        .trim_start_matches('#')
        .to_ascii_lowercase();
    if !(legacy_cmd_prefix(&verb, "ah", 2) || legacy_cmd_prefix(&verb, "auctionhouse", 10)) {
        return None;
    }

    let rest = rest.trim_start();
    let (sub_verb, args) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
    let sub_verb_lower = sub_verb.to_ascii_lowercase();

    if sub_verb_lower.is_empty() || sub_verb_lower == "help" {
        return Some(AuctionCommandResult {
            messages: auction_help_lines(),
            ..Default::default()
        });
    }

    let Some(repository) = repository else {
        return simple_result("The auction house is currently unavailable.");
    };

    // C's `commands[]` table (`auction_cmd.c:29-47`) with each entry's
    // `min_length` abbreviation floor.
    if legacy_cmd_prefix(&sub_verb_lower, "list", 1) {
        return Some(cmd_auction_list(repository, now_unix).await);
    }
    if legacy_cmd_prefix(&sub_verb_lower, "sell", 2) {
        return Some(cmd_auction_sell(repository, world, character_id, args, now_unix).await);
    }
    if legacy_cmd_prefix(&sub_verb_lower, "buy", 2) {
        return Some(cmd_auction_buy(repository, world, character_id, args, now_unix).await);
    }
    if legacy_cmd_prefix(&sub_verb_lower, "bid", 2) {
        return Some(cmd_auction_bid(repository, world, character_id, args, now_unix).await);
    }
    if legacy_cmd_prefix(&sub_verb_lower, "cancel", 2) {
        return Some(cmd_auction_cancel(repository, character_id, args, now_unix).await);
    }
    if legacy_cmd_prefix(&sub_verb_lower, "search", 2) {
        return Some(cmd_auction_search(repository, args, now_unix).await);
    }
    if legacy_cmd_prefix(&sub_verb_lower, "info", 1) {
        return Some(cmd_auction_info(repository, world, character_id, args, now_unix).await);
    }
    if legacy_cmd_prefix(&sub_verb_lower, "claim", 2) {
        return Some(cmd_auction_claim(repository, world, character_id).await);
    }

    simple_result("Unknown auction command. Type /ah help for help.")
}
