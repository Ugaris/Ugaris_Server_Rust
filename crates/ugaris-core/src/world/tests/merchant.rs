use super::*;
use crate::character_driver::{parse_merchant_driver_args, MerchantDriverData, CDR_MERCHANT};

fn merchant_npc(id: u32, pricemulti: i32) -> Character {
    let mut merchant = character(id);
    merchant.name = "Dolf".into();
    merchant.driver = CDR_MERCHANT;
    merchant.driver_state = Some(CharacterDriverState::Merchant(MerchantDriverData {
        pricemulti,
        ..MerchantDriverData::default()
    }));
    merchant
}

fn player(id: u32, barter: i16) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.values[0][CharacterValue::Barter as usize] = barter;
    player
}

#[test]
fn merchant_prices_match_c_store_formulas() {
    // C salesprice: max(v * 1.25, v * pricemulti / (barter + 100 + trader * 5)).
    assert_eq!(merchant_sales_price(1000, 400, 0, 0), 4000);
    assert_eq!(merchant_sales_price(1000, 400, 100, 0), 2000);
    assert_eq!(merchant_sales_price(1000, 400, 220, 0), 1250);
    assert_eq!(merchant_sales_price(1000, 400, 100, 20), 1333);
    // C buyprice: min(v * 0.8, v * (barter + 100 + trader * 5) / 400).
    assert_eq!(merchant_buy_price(1000, false, 0, 0), 250);
    assert_eq!(merchant_buy_price(1000, false, 100, 0), 500);
    assert_eq!(merchant_buy_price(1000, false, 250, 0), 800);
    assert_eq!(merchant_buy_price(1000, true, 0, 0), 1000);
}

#[test]
fn merchant_driver_args_parse_c_fields() {
    let data = parse_merchant_driver_args("dir=3;pricemulti=600;ignore=2;special=1;");
    assert_eq!(data.dir, 3);
    assert_eq!(data.pricemulti, 600);
    assert_eq!(data.ignore, 2);
    assert_eq!(data.special, 1);
    // C defaults opening hours before parsing.
    assert_eq!(data.open, 6);
    assert_eq!(data.close, 23);
}

#[test]
fn merchant_store_created_from_carried_inventory() {
    let mut world = World::default();
    let mut merchant = merchant_npc(1, 600);
    merchant.inventory[30] = Some(ItemId(900));
    merchant.inventory[31] = Some(ItemId(901));
    assert!(world.spawn_character(merchant, 10, 10));
    for id in [900u32, 901] {
        let mut ware = item(id, ItemFlags::TAKE);
        ware.value = 100 * id;
        ware.carried_by = Some(CharacterId(1));
        world.items.insert(ItemId(id), ware);
    }

    assert!(world.ensure_merchant_store(CharacterId(1)));

    let store = world.merchant_stores.get(&CharacterId(1)).unwrap();
    assert_eq!(store.price_multi, 600);
    let ware = store.wares[0].as_ref().unwrap();
    assert!(ware.always, "template stock never sells out");
    assert_eq!(ware.count, 1);
    assert!(store.wares[1].is_some());
    assert!(
        !world.items.contains_key(&ItemId(900)),
        "store stock is copied out of the live item table like C"
    );
    let merchant = world.characters.get(&CharacterId(1)).unwrap();
    assert!(merchant.inventory[30].is_none());
}

#[test]
fn merchant_trade_text_activates_store_for_speaker() {
    let mut world = World::default();
    assert!(world.spawn_character(merchant_npc(1, 400), 10, 10));
    assert!(world.spawn_character(player(2, 50), 11, 10));

    if let Some(merchant) = world.characters.get_mut(&CharacterId(1)) {
        merchant.push_driver_text_message(CharacterId(2), "Dolf, trade!");
    }
    world.process_merchant_actions();

    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().merchant,
        Some(CharacterId(1)),
        "C: saying '<name> ... trade' opens the store"
    );
}

#[test]
fn merchant_ignores_trade_text_for_other_names() {
    let mut world = World::default();
    assert!(world.spawn_character(merchant_npc(1, 400), 10, 10));
    assert!(world.spawn_character(player(2, 50), 11, 10));

    if let Some(merchant) = world.characters.get_mut(&CharacterId(1)) {
        merchant.push_driver_text_message(CharacterId(2), "Egbert, trade!");
    }
    world.process_merchant_actions();

    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().merchant,
        None
    );
}

#[test]
fn merchant_store_buy_moves_ware_to_cursor_for_gold() {
    let mut world = World::default();
    let mut merchant = merchant_npc(1, 400);
    merchant.inventory[30] = Some(ItemId(900));
    assert!(world.spawn_character(merchant, 10, 10));
    let mut ware = item(900, ItemFlags::TAKE);
    ware.value = 1000;
    ware.carried_by = Some(CharacterId(1));
    world.items.insert(ItemId(900), ware);
    let mut buyer = player(2, 100);
    buyer.gold = 5000;
    assert!(world.spawn_character(buyer, 11, 10));
    world.ensure_merchant_store(CharacterId(1));

    let result = world.merchant_store_buy(CharacterId(1), CharacterId(2), 0);

    assert_eq!(result, MerchantTradeResult::Traded(2000));
    let buyer = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(buyer.gold, 3000);
    let cursor = buyer.cursor_item.expect("bought item lands on the cursor");
    assert_eq!(world.items.get(&cursor).unwrap().value, 1000);
    let store = world.merchant_stores.get(&CharacterId(1)).unwrap();
    assert_eq!(store.gold, 2000);
    assert_eq!(
        store.wares[0].as_ref().unwrap().count,
        1,
        "always stock does not deplete"
    );
}

#[test]
fn merchant_store_buy_blocks_poor_and_occupied_cursor() {
    let mut world = World::default();
    let mut merchant = merchant_npc(1, 400);
    merchant.inventory[30] = Some(ItemId(900));
    assert!(world.spawn_character(merchant, 10, 10));
    let mut ware = item(900, ItemFlags::TAKE);
    ware.value = 1000;
    ware.carried_by = Some(CharacterId(1));
    world.items.insert(ItemId(900), ware);
    let mut buyer = player(2, 100);
    buyer.gold = 10;
    assert!(world.spawn_character(buyer, 11, 10));
    world.ensure_merchant_store(CharacterId(1));

    assert_eq!(
        world.merchant_store_buy(CharacterId(1), CharacterId(2), 0),
        MerchantTradeResult::TooExpensive
    );
    assert_eq!(
        world.merchant_store_buy(CharacterId(1), CharacterId(2), 5),
        MerchantTradeResult::SoldOut
    );
}

#[test]
fn merchant_store_sell_pays_gold_and_stocks_ware() {
    let mut world = World::default();
    assert!(world.spawn_character(merchant_npc(1, 400), 10, 10));
    let mut seller = player(2, 100);
    seller.cursor_item = Some(ItemId(900));
    assert!(world.spawn_character(seller, 11, 10));
    let mut sold = item(900, ItemFlags::TAKE);
    sold.value = 1000;
    sold.carried_by = Some(CharacterId(2));
    world.items.insert(ItemId(900), sold);
    world.ensure_merchant_store(CharacterId(1));

    let result = world.merchant_store_sell(CharacterId(1), CharacterId(2));

    assert_eq!(result, MerchantTradeResult::Traded(500));
    let seller = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(seller.gold, 500);
    assert_eq!(seller.cursor_item, None);
    assert!(!world.items.contains_key(&ItemId(900)));
    let store = world.merchant_stores.get(&CharacterId(1)).unwrap();
    assert_eq!(store.gold, -500);
    let stocked = store
        .wares
        .iter()
        .flatten()
        .find(|ware| ware.item.value == 1000)
        .expect("sold item is stocked for resale");
    assert!(!stocked.always);
}

#[test]
fn merchant_store_sell_never_stocks_quest_items() {
    let mut world = World::default();
    assert!(world.spawn_character(merchant_npc(1, 400), 10, 10));
    let mut seller = player(2, 100);
    seller.cursor_item = Some(ItemId(900));
    assert!(world.spawn_character(seller, 11, 10));
    let mut sold = item(900, ItemFlags::TAKE | ItemFlags::QUEST);
    sold.value = 1000;
    sold.carried_by = Some(CharacterId(2));
    world.items.insert(ItemId(900), sold);
    world.ensure_merchant_store(CharacterId(1));

    assert_eq!(
        world.merchant_store_sell(CharacterId(1), CharacterId(2)),
        MerchantTradeResult::Traded(500)
    );
    let store = world.merchant_stores.get(&CharacterId(1)).unwrap();
    assert!(store.wares.iter().all(Option::is_none));
}

#[test]
fn check_merchant_clears_when_busy_or_apart() {
    let mut world = World::default();
    assert!(world.spawn_character(merchant_npc(1, 400), 10, 10));
    let mut buyer = player(2, 100);
    buyer.merchant = Some(CharacterId(1));
    assert!(world.spawn_character(buyer, 11, 10));
    world.ensure_merchant_store(CharacterId(1));

    // Idle and adjacent: stays.
    world.check_merchant(CharacterId(2));
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().merchant,
        Some(CharacterId(1))
    );

    // Busy characters lose the store like C AC_IDLE/AC_BLESS_SELF gate.
    if let Some(buyer) = world.characters.get_mut(&CharacterId(2)) {
        buyer.action = action::WALK;
    }
    world.check_merchant(CharacterId(2));
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().merchant,
        None
    );
}

#[test]
fn merchant_greets_visible_players_once() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(merchant_npc(1, 400), 10, 10));
    let mut visitor = player(2, 0);
    visitor.name = "Godmode".into();
    assert!(world.spawn_character(visitor, 12, 10));

    world.process_merchant_actions();
    let texts = world.drain_pending_area_texts();
    assert_eq!(texts.len(), 1);
    assert!(texts[0].message.contains("Hello Godmode!"));
    assert!(texts[0].message.contains("Dolf, trade"));

    // Second pass: memory suppresses the repeat greeting.
    world.process_merchant_actions();
    assert!(world.drain_pending_area_texts().is_empty());
}

fn merchant_npc_already_greeted(id: u32, pricemulti: i32, greeted: u32) -> Character {
    let mut merchant = merchant_npc(id, pricemulti);
    if let Some(CharacterDriverState::Merchant(data)) = merchant.driver_state.as_mut() {
        data.greeted.push(greeted);
    }
    merchant
}

#[test]
fn merchant_replies_to_small_talk_keyword() {
    // C `analyse_text_driver` (`src/module/merchants/merchant.c`): saying
    // "hello" gets `quiet_say(cn, "Hello, %s!", ch[co].name, ch[cn].name)`.
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(merchant_npc_already_greeted(1, 400, 2), 10, 10));
    let mut visitor = player(2, 0);
    visitor.name = "Godmode".into();
    assert!(world.spawn_character(visitor, 12, 10));

    if let Some(merchant) = world.characters.get_mut(&CharacterId(1)) {
        merchant.push_driver_text_message(CharacterId(2), "hello");
    }
    world.process_merchant_actions();

    let texts = world.drain_pending_area_texts();
    assert!(
        texts
            .iter()
            .any(|text| text.message.contains("Hello, Godmode!")),
        "expected a qa reply among {texts:?}"
    );
}

#[test]
fn merchant_small_talk_ignores_non_player_speakers() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(merchant_npc_already_greeted(1, 400, 2), 10, 10));
    // A non-player character (no CF_PLAYER flag) at the same spot as id 2.
    let mut npc = character(2);
    npc.name = "Skelly".into();
    assert!(world.spawn_character(npc, 12, 10));

    if let Some(merchant) = world.characters.get_mut(&CharacterId(1)) {
        merchant.push_driver_text_message(CharacterId(2), "hello");
    }
    world.process_merchant_actions();

    let texts = world.drain_pending_area_texts();
    assert!(
        !texts.iter().any(|text| text.message.contains("Hello,")),
        "C: `if (!(ch[co].flags & CF_PLAYER)) return 0;`"
    );
}

#[test]
fn merchant_small_talk_ignores_speakers_beyond_analyse_text_distance() {
    // C `analyse_text_driver`: `if (char_dist(cn, co) > 12) return 0;`.
    let mut world = World::default();
    assert!(world.spawn_character(merchant_npc_already_greeted(1, 400, 2), 10, 10));
    let mut visitor = player(2, 0);
    visitor.name = "Godmode".into();
    assert!(world.spawn_character(visitor, 40, 10));

    if let Some(merchant) = world.characters.get_mut(&CharacterId(1)) {
        merchant.push_driver_text_message(CharacterId(2), "hello");
    }
    world.process_merchant_actions();

    let texts = world.drain_pending_area_texts();
    assert!(!texts.iter().any(|text| text.message.contains("Hello,")));
}
