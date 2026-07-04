use super::*;

use crate::auction::{
    apply_auction_command, auction_help_lines, auction_value_name, calculate_auction_fee,
    calculate_min_bid, format_auction_login_notice, format_item_details, format_item_modifiers,
    format_money, format_price, format_time_left, validate_auction_item,
};
use ugaris_core::entity::MAX_MODIFIERS;
use ugaris_db::{AuctionRecord, AuctionStatus, DeliverySummary};

fn sample_item(name: &str, min_level: u8) -> Item {
    Item {
        id: ItemId(1),
        name: name.to_string(),
        description: String::new(),
        flags: ItemFlags::TAKE | ItemFlags::USED,
        sprite: 1,
        value: 100,
        min_level,
        max_level: 0,
        needs_class: 0,
        template_id: 7,
        owner_id: 0,
        modifier_index: [0; MAX_MODIFIERS],
        modifier_value: [0; MAX_MODIFIERS],
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

fn sample_auction(item: Item) -> AuctionRecord {
    AuctionRecord {
        id: 42,
        seller_id: CharacterId(1),
        seller_name: "Seller".to_string(),
        item_template: item.template_id,
        item,
        start_price: 1000,
        buyout_price: None,
        current_bid: None,
        current_bidder_id: None,
        created_at_unix: 0,
        ends_at_unix: 1_000_000,
        status: AuctionStatus::Active,
    }
}

#[test]
fn format_money_shows_gold_and_silver_above_a_gold() {
    // C `format_money_string`/`format_money`: gold = amount/100, silver =
    // amount%100, gold+silver text only once gold > 0.
    assert_eq!(format_money(0), "0 silver");
    assert_eq!(format_money(50), "50 silver");
    assert_eq!(format_money(99), "99 silver");
    assert_eq!(format_money(100), "1 gold, 0 silver");
    assert_eq!(format_money(1234), "12 gold, 34 silver");
}

#[test]
fn calculate_auction_fee_floors_at_one_gold() {
    // C `calculate_auction_fee`: 5% of start price, floored at 100 (1 gold).
    assert_eq!(calculate_auction_fee(0), 100);
    assert_eq!(calculate_auction_fee(100), 100);
    assert_eq!(calculate_auction_fee(1_000), 100);
    assert_eq!(calculate_auction_fee(10_000), 500);
    assert_eq!(calculate_auction_fee(100_000), 5_000);
}

#[test]
fn calculate_min_bid_uses_start_price_when_no_bids_exist() {
    assert_eq!(calculate_min_bid(None, 1_000), 1_000);
    assert_eq!(calculate_min_bid(Some(0), 1_000), 1_000);
}

#[test]
fn calculate_min_bid_adds_five_percent_increment_with_one_copper_floor() {
    // C `auction_bid`: increment = current_bid/100*5, minimum 1 copper.
    assert_eq!(calculate_min_bid(Some(1_000), 500), 1_050);
    // Below 20 copper, 5% truncates to 0, so the floor of 1 applies.
    assert_eq!(calculate_min_bid(Some(10), 500), 11);
}

#[test]
fn calculate_min_bid_saturates_instead_of_overflowing() {
    assert_eq!(calculate_min_bid(Some(u64::MAX), 500), u64::MAX);
}

#[test]
fn validate_auction_item_requires_take_and_rejects_special_flags() {
    let takeable = sample_item("Sword", 0);
    assert!(validate_auction_item(&takeable));

    let mut not_takeable = takeable.clone();
    not_takeable.flags.remove(ItemFlags::TAKE);
    assert!(!validate_auction_item(&not_takeable));

    for flag in [
        ItemFlags::QUEST,
        ItemFlags::NODROP,
        ItemFlags::BONDTAKE,
        ItemFlags::LABITEM,
        ItemFlags::NODEPOT,
    ] {
        let mut item = takeable.clone();
        item.flags.insert(flag);
        assert!(
            !validate_auction_item(&item),
            "expected {flag:?} to disqualify an auction item"
        );
    }
}

#[test]
fn auction_value_name_matches_legacy_short_abbreviations() {
    // Spot-check against C's `get_value_name` switch (`auction_house.c`),
    // not the unrelated `CHARACTER_VALUE_NAMES` Title-Case table.
    assert_eq!(auction_value_name(0), "hp");
    assert_eq!(auction_value_name(6), "str");
    assert_eq!(auction_value_name(17), "armor skill");
    assert_eq!(auction_value_name(31), "m-shield");
    assert_eq!(auction_value_name(34), "empty");
    assert_eq!(auction_value_name(42), "profession");
    assert_eq!(auction_value_name(99), "mod99");
}

#[test]
fn format_item_modifiers_splits_positive_and_negative_indices() {
    let mut item = sample_item("Ring", 0);
    item.modifier_index[0] = 6; // +str modifier
    item.modifier_value[0] = 5;
    item.modifier_index[1] = -2; // mana requirement
    item.modifier_value[1] = 10;

    let (requirements, modifiers) = format_item_modifiers(&item);
    assert_eq!(requirements, "mana 10");
    assert_eq!(modifiers, "+5 str");
}

#[test]
fn format_item_details_colors_name_by_min_level_tier() {
    let low = sample_item("Dagger", 0);
    let details = format_item_details(&low);
    assert!(details.starts_with(COL_RESET));

    let high = sample_item("Ancient Blade", 80);
    let details = format_item_details(&high);
    assert!(details.starts_with(COL_LIGHT_VIOLET));
    assert!(details_contains(&details, "Level required: 80"));
}

fn details_contains(bytes: &[u8], needle: &str) -> bool {
    String::from_utf8_lossy(bytes).contains(needle)
}

#[test]
fn format_time_left_picks_color_tier_by_remaining_duration() {
    let (text_str, color) = format_time_left(1_000_000, 999_999);
    assert_eq!(text_str, "0m");
    assert_eq!(color, COL_LIGHT_RED);

    let (text_str, color) = format_time_left(1_000_000 + 7200, 1_000_000);
    assert_eq!(text_str, "2h 0m");
    assert_eq!(color, COL_LIGHT_GREEN);

    let (text_str, color) = format_time_left(1_000_000 + 100 * 3600, 1_000_000);
    assert_eq!(text_str, "4d 4h");
    assert_eq!(color, COL_VIOLET);

    let (text_str, color) = format_time_left(500, 1_000_000);
    assert_eq!(text_str, "Ended");
    assert_eq!(color, COL_DARK_GRAY);
}

#[test]
fn format_price_shows_buyout_suffix_when_present() {
    let mut auction = sample_auction(sample_item("Shield", 0));
    auction.buyout_price = Some(5_000);
    let (text_str, color) = format_price(&auction);
    assert_eq!(text_str, "10 gold, 0 silver (50 gold, 0 silver buyout)");
    assert_eq!(color, COL_YELLOW);
}

#[test]
fn format_price_colors_green_when_bid_above_start_and_violet_otherwise() {
    let mut auction = sample_auction(sample_item("Shield", 0));
    auction.current_bid = Some(2_000);
    let (_, color) = format_price(&auction);
    assert_eq!(color, COL_LIGHT_GREEN);

    let auction_no_bid = sample_auction(sample_item("Shield", 0));
    let (_, color) = format_price(&auction_no_bid);
    assert_eq!(color, COL_VIOLET);
}

#[test]
fn auction_help_lines_cover_every_subcommand() {
    let lines = auction_help_lines();
    assert_eq!(lines[0], "Available auction commands:");
    assert!(lines
        .iter()
        .any(|line| line == "/ah list - List all your active auctions"));
    assert!(lines.iter().any(|line| line
        == "/ah sell <start_price> [buyout_price] [duration] - Place an item up for auction. Duration in hours (default 24)"));
    assert!(lines
        .iter()
        .any(|line| line == "/ah claim - Claim items and gold from completed auctions"));
    assert_eq!(lines.len(), 9);
}

fn test_world_with_player() -> (World, CharacterId) {
    let mut world = World::default();
    let character_id = CharacterId(1);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.characters.insert(character_id, character);
    (world, character_id)
}

#[tokio::test]
async fn apply_auction_command_ignores_unrelated_verbs() {
    let (mut world, character_id) = test_world_with_player();
    let result = apply_auction_command(&mut world, &None, character_id, 0, "/tell foo bar").await;
    assert!(result.is_none());
}

#[tokio::test]
async fn apply_auction_command_shows_help_without_a_repository() {
    let (mut world, character_id) = test_world_with_player();
    let result = apply_auction_command(&mut world, &None, character_id, 0, "/ah help")
        .await
        .expect("ah help should always be recognized");
    assert_eq!(result.messages[0], "Available auction commands:");
}

#[tokio::test]
async fn apply_auction_command_reports_unavailable_without_a_repository() {
    let (mut world, character_id) = test_world_with_player();
    let result = apply_auction_command(&mut world, &None, character_id, 0, "/ah sell 10")
        .await
        .expect("ah sell should be recognized even without a repository");
    assert_eq!(
        result.messages,
        vec!["The auction house is currently unavailable.".to_string()]
    );
}

#[tokio::test]
async fn apply_auction_command_accepts_the_long_form_verb() {
    let (mut world, character_id) = test_world_with_player();
    let result = apply_auction_command(&mut world, &None, character_id, 0, "/auctionhouse help")
        .await
        .expect("auctionhouse should be recognized as an /ah alias");
    assert_eq!(result.messages[0], "Available auction commands:");
}

#[tokio::test]
async fn apply_auction_command_defaults_to_help_with_no_subcommand() {
    let (mut world, character_id) = test_world_with_player();
    let result = apply_auction_command(&mut world, &None, character_id, 0, "/ah")
        .await
        .expect("bare /ah should show help");
    assert_eq!(result.messages[0], "Available auction commands:");
}

fn notice_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn format_auction_login_notice_is_none_without_pending_deliveries() {
    let summary = DeliverySummary {
        pending_count: 0,
        total_gold: 5_000,
        has_items: true,
    };
    assert!(format_auction_login_notice(&summary).is_none());
}

#[test]
fn format_auction_login_notice_reports_items_and_gold_above_a_gold() {
    // C `auction_check_deliveries_login`'s `has_items && total_gold > 0`
    // branch, `total_gold >= 100` sub-case.
    let summary = DeliverySummary {
        pending_count: 2,
        total_gold: 1_234,
        has_items: true,
    };
    let notice = format_auction_login_notice(&summary).expect("expected a notice");
    assert!(notice.starts_with(COL_YELLOW));
    assert!(notice.ends_with(COL_RESET));
    assert!(notice_text(&notice).contains(
        "You have 2 auction deliveries waiting - items and 12 gold, 34 silver. Type '/ah claim' to receive them."
    ));
}

#[test]
fn format_auction_login_notice_reports_items_and_gold_below_a_gold() {
    let summary = DeliverySummary {
        pending_count: 1,
        total_gold: 50,
        has_items: true,
    };
    let notice = notice_text(&format_auction_login_notice(&summary).unwrap());
    assert!(notice.contains(
        "You have 1 auction delivery waiting - items and 50 silver. Type '/ah claim' to receive them."
    ));
}

#[test]
fn format_auction_login_notice_reports_items_only() {
    let summary = DeliverySummary {
        pending_count: 3,
        total_gold: 0,
        has_items: true,
    };
    let notice = notice_text(&format_auction_login_notice(&summary).unwrap());
    assert!(notice.contains(
        "You have 3 auction deliveries with items waiting. Type '/ah claim' to receive them."
    ));
}

#[test]
fn format_auction_login_notice_reports_gold_only() {
    let summary = DeliverySummary {
        pending_count: 1,
        total_gold: 250,
        has_items: false,
    };
    let notice = notice_text(&format_auction_login_notice(&summary).unwrap());
    assert!(notice.contains(
        "You have 1 auction delivery with 2 gold, 50 silver waiting. Type '/ah claim' to receive them."
    ));
}

#[test]
fn format_auction_login_notice_is_none_when_neither_items_nor_gold_pending() {
    // C leaves `buf` uninitialized in this unreachable combination; this
    // port skips the notice instead of replicating the undefined behavior.
    let summary = DeliverySummary {
        pending_count: 1,
        total_gold: 0,
        has_items: false,
    };
    assert!(format_auction_login_notice(&summary).is_none());
}
