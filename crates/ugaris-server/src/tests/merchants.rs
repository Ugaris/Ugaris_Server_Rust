use super::*;

use ugaris_core::character_driver::{CharacterDriverState, MerchantDriverData, CDR_MERCHANT};

fn merchant_character(id: CharacterId, price_multi: i32) -> Character {
    let mut merchant = login_character(id, &login_block("Dolf"), 1, 10, 10);
    merchant.flags = CharacterFlags::USED | CharacterFlags::ALIVE;
    merchant.driver = CDR_MERCHANT;
    merchant.driver_state = Some(CharacterDriverState::Merchant(MerchantDriverData {
        pricemulti: price_multi,
        ..MerchantDriverData::default()
    }));
    merchant
}

/// Spawns a merchant and a shopping player standing next to each other with
/// an active store, mirroring the state `cl_container`'s "trade" text
/// creates before `CL_FASTSELL` packets arrive.
fn merchant_and_shopper(price_multi: i32, barter: i16) -> (World, CharacterId, CharacterId) {
    let mut world = World::default();
    let merchant_id = CharacterId(1);
    let player_id = CharacterId(2);
    assert!(world.spawn_character(merchant_character(merchant_id, price_multi), 10, 10));
    let mut player = login_character(player_id, &login_block("Tester"), 1, 11, 10);
    player.values[0][CharacterValue::Barter as usize] = barter;
    assert!(world.spawn_character(player, 11, 10));
    assert!(world.ensure_merchant_store(merchant_id));
    world.characters.get_mut(&player_id).unwrap().merchant = Some(merchant_id);
    (world, merchant_id, player_id)
}

#[test]
fn fast_sell_sells_inventory_slot_to_active_merchant() {
    let (mut world, merchant_id, player_id) = merchant_and_shopper(400, 100);
    let item_id = ItemId(900);
    let mut sold = test_item(item_id, 1234, ItemFlags::TAKE | ItemFlags::USED);
    sold.value = 1000;
    sold.carried_by = Some(player_id);
    world.add_item(sold);
    world.characters.get_mut(&player_id).unwrap().inventory[30] = Some(item_id);

    let result = apply_fast_sell(&mut world, player_id, 30);

    // C buyprice(1000, barter=100, trader=0) = min(800, 1000*200/400) = 500.
    assert!(result.inventory_changed);
    assert!(result.sold);
    assert_eq!(result.messages, vec!["Sold for 5G 0S".to_string()]);

    let player = world.characters.get(&player_id).unwrap();
    assert_eq!(player.gold, 500);
    assert_eq!(player.inventory[30], None);
    assert_eq!(player.cursor_item, None);
    assert!(!world.items.contains_key(&item_id));
    let store = world.merchant_stores.get(&merchant_id).unwrap();
    assert!(
        store
            .wares
            .iter()
            .flatten()
            .any(|ware| ware.item.value == 1000),
        "sold item is stocked for resale like C `buy()`"
    );
}

#[test]
fn fast_sell_swaps_cursor_item_into_slot_when_slot_was_empty() {
    // C `swap()`: with an empty slot and something already on the cursor,
    // the cursor item goes into the slot and citem becomes 0, so the sale
    // attempt is a no-op (matches `if (!(in = ch[cn].citem)) return;`).
    let (mut world, merchant_id, player_id) = merchant_and_shopper(400, 100);
    let cursor_id = ItemId(901);
    let mut cursor_item = test_item(cursor_id, 42, ItemFlags::TAKE | ItemFlags::USED);
    cursor_item.carried_by = Some(player_id);
    world.add_item(cursor_item);
    world.characters.get_mut(&player_id).unwrap().cursor_item = Some(cursor_id);

    let result = apply_fast_sell(&mut world, player_id, 30);

    assert!(result.inventory_changed);
    assert!(!result.sold);
    assert!(result.messages.is_empty());
    let player = world.characters.get(&player_id).unwrap();
    assert_eq!(player.inventory[30], Some(cursor_id));
    assert_eq!(player.cursor_item, None);
    assert_eq!(player.gold, 0);
    let store = world.merchant_stores.get(&merchant_id).unwrap();
    assert!(store.wares.iter().all(Option::is_none));
}

#[test]
fn fast_sell_blocks_quest_items_with_c_message_and_leaves_item_on_cursor() {
    let (mut world, merchant_id, player_id) = merchant_and_shopper(400, 100);
    let item_id = ItemId(902);
    let mut quest_item = test_item(
        item_id,
        55,
        ItemFlags::TAKE | ItemFlags::USED | ItemFlags::QUEST,
    );
    quest_item.value = 1000;
    quest_item.carried_by = Some(player_id);
    world.add_item(quest_item);
    world.characters.get_mut(&player_id).unwrap().inventory[30] = Some(item_id);

    let result = apply_fast_sell(&mut world, player_id, 30);

    assert!(
        result.inventory_changed,
        "swap() already ran before the guard"
    );
    assert!(!result.sold);
    assert_eq!(
        result.messages,
        vec![
            "You cannot quick-sell quest items (hold down SHIFT and LEFT-CLICK on the merchant's windows to go ahead)."
                .to_string()
        ]
    );
    let player = world.characters.get(&player_id).unwrap();
    assert_eq!(
        player.cursor_item,
        Some(item_id),
        "item stays on the cursor"
    );
    assert_eq!(player.inventory[30], None);
    assert_eq!(player.gold, 0);
    let store = world.merchant_stores.get(&merchant_id).unwrap();
    assert!(store.wares.iter().all(Option::is_none));
}

#[test]
fn fast_sell_ignores_illegal_slots_like_c_bounds_check() {
    let (mut world, _merchant_id, player_id) = merchant_and_shopper(400, 100);

    // C: `pos >= 12 && pos <= 29` is the equip/spell range, rejected.
    let result = apply_fast_sell(&mut world, player_id, 12);
    assert!(!result.inventory_changed);
    assert!(result.messages.is_empty());
}

#[test]
fn fast_sell_is_a_no_op_without_an_active_merchant() {
    let (mut world, _merchant_id, player_id) = merchant_and_shopper(400, 100);
    world.characters.get_mut(&player_id).unwrap().merchant = None;
    let item_id = ItemId(903);
    let mut item = test_item(item_id, 10, ItemFlags::TAKE | ItemFlags::USED);
    item.carried_by = Some(player_id);
    world.add_item(item);
    world.characters.get_mut(&player_id).unwrap().inventory[30] = Some(item_id);

    let result = apply_fast_sell(&mut world, player_id, 30);

    // C: swap() still moves the item onto the cursor before the merchant
    // check fails; without a store the sell attempt is skipped.
    assert!(result.inventory_changed);
    assert!(!result.sold);
    assert!(result.messages.is_empty());
    let player = world.characters.get(&player_id).unwrap();
    assert_eq!(player.cursor_item, Some(item_id));
    assert_eq!(player.inventory[30], None);
}
