use super::*;

#[test]
fn login_payload_sends_inventory_item_sprites() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
    let item_id = ugaris_core::ids::ItemId(99);
    character.inventory[30] = Some(item_id);

    let mut world = World::default();
    world.add_item(ugaris_core::entity::Item {
        id: item_id,
        name: "Torch".into(),
        description: String::new(),
        flags: ItemFlags::TAKE | ItemFlags::USE,
        sprite: 1234,
        value: 0,
        min_level: 0,
        max_level: 0,
        needs_class: 0,
        template_id: 0,
        owner_id: 0,
        modifier_index: [0; ugaris_core::entity::MAX_MODIFIERS],
        modifier_value: [0; ugaris_core::entity::MAX_MODIFIERS],
        x: 0,
        y: 0,
        carried_by: Some(character.id),
        contained_in: None,
        content_id: 0,
        driver: 0,
        driver_data: Vec::new(),
        serial: 1,
    });

    let payload = login_payload(&world, &character, 1, 0);
    let expected = [
        SV_SETITEM,
        30,
        0xd2,
        0x04,
        0,
        0,
        (ItemFlags::TAKE | ItemFlags::USE).bits() as u8,
        0,
        0,
        0,
    ];

    assert!(payload
        .windows(expected.len())
        .any(|window| window == expected));
}

#[test]
fn inventory_snapshot_payload_sends_cursor_and_inventory() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
    character.gold = 12345;
    let cursor_id = ugaris_core::ids::ItemId(98);
    let slot_id = ugaris_core::ids::ItemId(99);
    character.cursor_item = Some(cursor_id);
    character.inventory[30] = Some(slot_id);

    let mut world = World::default();
    world.add_item(test_item(cursor_id, 5000, ItemFlags::TAKE));
    world.add_item(test_item(slot_id, 1234, ItemFlags::TAKE | ItemFlags::USE));

    let payload = inventory_snapshot_payload(&world, &character);

    assert_eq!(&payload[..9], &[SV_SETCITEM, 0x88, 0x13, 0, 0, 8, 0, 0, 0]);
    assert!(payload.windows(10).any(|window| {
        window
            == [
                SV_SETITEM,
                30,
                0xd2,
                0x04,
                0,
                0,
                (ItemFlags::TAKE | ItemFlags::USE).bits() as u8,
                0,
                0,
                0,
            ]
    }));
    assert!(payload
        .windows(5)
        .any(|window| window == [SV_GOLD, 0x39, 0x30, 0, 0]));
}

#[test]
fn legacy_item_look_text_includes_c_shaped_modifiers_requirements_and_flags() {
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    character.values[1][CharacterValue::Strength as usize] = 12;

    let mut item = test_item(ItemId(99), 59210, ItemFlags::QUEST | ItemFlags::NOENHANCE);
    item.name = "Fine Sword".to_string();
    item.description = "A carefully balanced blade.".to_string();
    item.modifier_index = [
        CharacterValue::Armor as i16,
        CharacterValue::Sword as i16,
        -(CharacterValue::Strength as i16),
        0,
        0,
    ];
    item.modifier_value = [15, 3, 20, 0, 0];
    item.min_level = 4;
    item.needs_class = 1 | 4;

    let text = legacy_item_look_text(&item, &character);

    assert_eq!(
        text,
        "Fine Sword:\nA carefully balanced blade.\nModifiers:\nArmor Value +0.75\nSword +3\nRequirements:\nStrength 20 (you have 12)\nMinimum Level: 4\nOnly usable by a Warrior.\nOnly usable by a Seyan'Du.\nThis is a quest item. You cannot drop it or give it away.\nThis item resists magic, so you cannot enhance it using orbs, metals or shrines.\nThe item has been gilded."
    );
}

#[test]
fn legacy_item_look_text_includes_bonding_duration_and_sprite_notes() {
    let character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    let mut item = test_item(
        ItemId(99),
        53026,
        ItemFlags::BONDTAKE | ItemFlags::BONDWEAR | ItemFlags::BEYONDMAXMOD,
    );
    item.name = "Frozen Charm".to_string();
    item.owner_id = 12345;
    item.driver = IDR_DECAYITEM;
    item.modifier_index[0] = CharacterValue::Speed as i16;
    item.modifier_value[0] = 5;
    item.driver_data = vec![0; 7];
    item.driver_data[2] = 253_u8;
    item.driver_data[3..5].copy_from_slice(&30_u16.to_le_bytes());
    item.driver_data[5..7].copy_from_slice(&1800_u16.to_le_bytes());

    let text = legacy_item_look_text(&item, &character);

    assert_eq!(
        text,
        "Frozen Charm:\nModifiers:\nSpeed +5 (active: -3)\nThis item is bonded to somebody else. Only he can take it.\nThis item is bonded to somebody else. Only he can wear it.\nThis item goes beyond maximum modifier limits.\nDuration: 0:01:00 of 1:00:00 active time used up.\nThis is part of an ice demon suit."
    );
}

#[test]
fn inventory_swap_moves_cursor_and_slot_items() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(10));
    character.inventory[30] = Some(ItemId(11));
    world.add_character(character);
    world.add_item(test_item(
        ItemId(10),
        100,
        ItemFlags::USED | ItemFlags::TAKE,
    ));
    world.add_item(test_item(
        ItemId(11),
        200,
        ItemFlags::USED | ItemFlags::TAKE,
    ));

    let result = apply_inventory_client_action(
        &mut world,
        None,
        character_id,
        &ClientAction::Swap { slot: 30 },
        1,
    );

    assert_eq!(result, InventoryCommandResult::Changed);
    let character = &world.characters[&character_id];
    assert_eq!(character.cursor_item, Some(ItemId(11)));
    assert_eq!(character.inventory[30], Some(ItemId(10)));
    assert_eq!(world.items[&ItemId(10)].carried_by, Some(character_id));
    assert_eq!(world.items[&ItemId(11)].carried_by, Some(character_id));
}

#[test]
fn inventory_swap_wears_item_with_matching_worn_slot_flag() {
    // C `swap` (`src/system/do.c:1239-1243`): `pos < 12` requires
    // `can_wear`, which for `WN_HEAD` requires `IF_WNHEAD`.
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(10));
    world.add_character(character);
    world.add_item(test_item(
        ItemId(10),
        100,
        ItemFlags::USED | ItemFlags::WNHEAD,
    ));

    let result = apply_inventory_client_action(
        &mut world,
        None,
        character_id,
        &ClientAction::Swap {
            slot: worn_slot::HEAD as u8,
        },
        1,
    );

    assert_eq!(result, InventoryCommandResult::Changed);
    let character = &world.characters[&character_id];
    assert_eq!(character.cursor_item, None);
    assert_eq!(character.inventory[worn_slot::HEAD], Some(ItemId(10)));
}

#[test]
fn inventory_swap_rejects_item_without_matching_worn_slot_flag() {
    // C `can_wear` (`src/system/tool.c:1026-1029`): `WN_HEAD` requires
    // `IF_WNHEAD`; a body-only item must be rejected and left untouched.
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(10));
    world.add_character(character);
    world.add_item(test_item(
        ItemId(10),
        100,
        ItemFlags::USED | ItemFlags::WNBODY,
    ));

    let result = apply_inventory_client_action(
        &mut world,
        None,
        character_id,
        &ClientAction::Swap {
            slot: worn_slot::HEAD as u8,
        },
        1,
    );

    assert_eq!(result, InventoryCommandResult::Ignored);
    let character = &world.characters[&character_id];
    assert_eq!(character.cursor_item, Some(ItemId(10)));
    assert_eq!(character.inventory[worn_slot::HEAD], None);
}

#[test]
fn inventory_swap_rejects_item_below_minimum_level() {
    // C `check_requirements` (`src/system/tool.c:966-968`): `min_level`
    // gates wearing even when the slot flag matches.
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.level = 3;
    character.cursor_item = Some(ItemId(10));
    world.add_character(character);
    let mut item = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::WNHEAD);
    item.min_level = 10;
    world.add_item(item);

    let result = apply_inventory_client_action(
        &mut world,
        None,
        character_id,
        &ClientAction::Swap {
            slot: worn_slot::HEAD as u8,
        },
        1,
    );

    assert_eq!(result, InventoryCommandResult::Ignored);
    assert_eq!(
        world.characters[&character_id].inventory[worn_slot::HEAD],
        None
    );
}

#[test]
fn inventory_swap_rejects_item_restricted_to_other_class() {
    // C `check_requirements` (`src/system/tool.c:973-981`): `needs_class`
    // bit 2 ("Only usable by a Mage") rejects a `CF_WARRIOR` character.
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::WARRIOR);
    character.cursor_item = Some(ItemId(10));
    world.add_character(character);
    let mut item = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::WNHEAD);
    item.needs_class = 2;
    world.add_item(item);

    let result = apply_inventory_client_action(
        &mut world,
        None,
        character_id,
        &ClientAction::Swap {
            slot: worn_slot::HEAD as u8,
        },
        1,
    );

    assert_eq!(result, InventoryCommandResult::Ignored);
}

#[test]
fn inventory_swap_rejects_item_below_stat_requirement() {
    // C `check_requirements` (`src/system/tool.c:959-963`): negative
    // `mod_index` entries are read against `value[1]` (base, not
    // equipment-modified) and reject the wear if the character falls
    // short.
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.values[1][CharacterValue::Strength as usize] = 12;
    character.cursor_item = Some(ItemId(10));
    world.add_character(character);
    let mut item = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::WNHEAD);
    item.modifier_index[0] = -(CharacterValue::Strength as i16);
    item.modifier_value[0] = 20;
    world.add_item(item);

    let result = apply_inventory_client_action(
        &mut world,
        None,
        character_id,
        &ClientAction::Swap {
            slot: worn_slot::HEAD as u8,
        },
        1,
    );

    assert_eq!(result, InventoryCommandResult::Ignored);
}

#[test]
fn inventory_swap_rejects_two_handed_weapon_into_right_hand_when_left_hand_occupied() {
    // C `can_wear` (`src/system/tool.c:1086-1094`): a two-handed weapon
    // (`IF_WNTWOHANDED`) cannot go into `WN_RHAND` while `WN_LHAND` is
    // occupied by anything (a shield, torch, etc).
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(10));
    character.inventory[worn_slot::LEFT_HAND] = Some(ItemId(20));
    world.add_character(character);
    world.add_item(test_item(
        ItemId(10),
        100,
        ItemFlags::USED | ItemFlags::WNRHAND | ItemFlags::WNTWOHANDED,
    ));
    world.add_item(test_item(
        ItemId(20),
        200,
        ItemFlags::USED | ItemFlags::WNLHAND,
    ));

    let result = apply_inventory_client_action(
        &mut world,
        None,
        character_id,
        &ClientAction::Swap {
            slot: worn_slot::RIGHT_HAND as u8,
        },
        1,
    );

    assert_eq!(result, InventoryCommandResult::Ignored);
    assert_eq!(
        world.characters[&character_id].inventory[worn_slot::RIGHT_HAND],
        None
    );
}

#[test]
fn inventory_swap_allows_two_handed_weapon_into_right_hand_when_left_hand_empty() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(10));
    world.add_character(character);
    world.add_item(test_item(
        ItemId(10),
        100,
        ItemFlags::USED | ItemFlags::WNRHAND | ItemFlags::WNTWOHANDED,
    ));

    let result = apply_inventory_client_action(
        &mut world,
        None,
        character_id,
        &ClientAction::Swap {
            slot: worn_slot::RIGHT_HAND as u8,
        },
        1,
    );

    assert_eq!(result, InventoryCommandResult::Changed);
    assert_eq!(
        world.characters[&character_id].inventory[worn_slot::RIGHT_HAND],
        Some(ItemId(10))
    );
}

#[test]
fn inventory_swap_rejects_left_hand_item_when_right_hand_holds_two_handed_weapon() {
    // C `can_wear` (`src/system/tool.c:1077-1080`): `WN_LHAND` is blocked
    // outright whenever `WN_RHAND` holds an `IF_WNTWOHANDED` item.
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(10));
    character.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(20));
    world.add_character(character);
    world.add_item(test_item(
        ItemId(10),
        100,
        ItemFlags::USED | ItemFlags::WNLHAND,
    ));
    world.add_item(test_item(
        ItemId(20),
        200,
        ItemFlags::USED | ItemFlags::WNRHAND | ItemFlags::WNTWOHANDED,
    ));

    let result = apply_inventory_client_action(
        &mut world,
        None,
        character_id,
        &ClientAction::Swap {
            slot: worn_slot::LEFT_HAND as u8,
        },
        1,
    );

    assert_eq!(result, InventoryCommandResult::Ignored);
}

#[test]
fn inventory_swap_unequip_ignores_wear_requirements() {
    // C `swap`: `can_wear` is only invoked when the cursor holds an item
    // (`if ((in = ch[cn].citem))`); taking a worn item off (empty cursor)
    // never re-checks wear requirements.
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.inventory[worn_slot::HEAD] = Some(ItemId(10));
    world.add_character(character);
    // No WN* flags at all - could never have been worn through `can_wear`,
    // but an empty-cursor swap must still be able to take it off.
    world.add_item(test_item(ItemId(10), 100, ItemFlags::USED));

    let result = apply_inventory_client_action(
        &mut world,
        None,
        character_id,
        &ClientAction::Swap {
            slot: worn_slot::HEAD as u8,
        },
        1,
    );

    assert_eq!(result, InventoryCommandResult::Changed);
    let character = &world.characters[&character_id];
    assert_eq!(character.cursor_item, Some(ItemId(10)));
    assert_eq!(character.inventory[worn_slot::HEAD], None);
}

#[test]
fn inventory_swap_converts_held_money_item_into_gold() {
    // C `swap`'s `IF_MONEY` branch (`src/system/do.c:1276-1287`): a money
    // item held on the cursor is destroyed on swap and its `value`
    // credited straight to `ch[cn].gold` instead of landing in the target
    // slot.
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(10));
    character.gold = 500;
    world.add_character(character);
    let mut money = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::MONEY);
    money.value = 250;
    world.add_item(money);

    let result = apply_inventory_client_action(
        &mut world,
        None,
        character_id,
        &ClientAction::Swap { slot: 30 },
        1,
    );

    assert_eq!(
        result,
        InventoryCommandResult::MoneyConverted { price: 250 }
    );
    let character = &world.characters[&character_id];
    assert_eq!(character.gold, 750);
    assert_eq!(character.cursor_item, None);
    assert_eq!(character.inventory[30], None);
    assert!(!world.items.contains_key(&ItemId(10)));
}

#[test]
fn inventory_swap_converts_held_money_item_and_puts_slot_item_on_cursor() {
    // Same money-conversion branch, but the target slot already holds an
    // item: C's `ch[cn].citem = ch[cn].item[pos];` runs before the money
    // check, so the slot's original occupant still ends up on the cursor.
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(10));
    character.inventory[30] = Some(ItemId(11));
    world.add_character(character);
    let mut money = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::MONEY);
    money.value = 250;
    world.add_item(money);
    world.add_item(test_item(
        ItemId(11),
        200,
        ItemFlags::USED | ItemFlags::TAKE,
    ));

    let result = apply_inventory_client_action(
        &mut world,
        None,
        character_id,
        &ClientAction::Swap { slot: 30 },
        1,
    );

    assert_eq!(
        result,
        InventoryCommandResult::MoneyConverted { price: 250 }
    );
    let character = &world.characters[&character_id];
    assert_eq!(character.cursor_item, Some(ItemId(11)));
    assert_eq!(character.inventory[30], None);
    assert!(!world.items.contains_key(&ItemId(10)));
    assert_eq!(world.items[&ItemId(11)].carried_by, Some(character_id));
}

#[test]
fn inventory_swap_rejects_money_item_into_worn_slot_without_matching_wear_flags() {
    // Money items carry no `WN*` wear-slot flags, so `can_wear` rejects
    // them exactly like any other unwearable item for `pos < 12` - the
    // money conversion never runs.
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(10));
    character.gold = 500;
    world.add_character(character);
    let mut money = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::MONEY);
    money.value = 250;
    world.add_item(money);

    let result = apply_inventory_client_action(
        &mut world,
        None,
        character_id,
        &ClientAction::Swap {
            slot: worn_slot::HEAD as u8,
        },
        1,
    );

    assert_eq!(result, InventoryCommandResult::Ignored);
    let character = &world.characters[&character_id];
    assert_eq!(character.gold, 500);
    assert_eq!(character.cursor_item, Some(ItemId(10)));
    assert!(world.items.contains_key(&ItemId(10)));
}

#[test]
fn inventory_look_uses_legacy_item_text() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.inventory[30] = Some(ItemId(10));
    world.add_character(character);
    let mut item = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::TAKE);
    item.name = "Fine Sword".to_string();
    item.description = "A carefully balanced blade.".to_string();
    world.add_item(item);

    let result = apply_inventory_client_action(
        &mut world,
        None,
        character_id,
        &ClientAction::LookInventory { slot: 30 },
        1,
    );

    assert_eq!(
        result,
        InventoryCommandResult::Look("Fine Sword:\nA carefully balanced blade.".to_string())
    );
}

#[test]
fn look_item_uses_legacy_item_text_when_visible_on_map() {
    // C `cl_look_item` (`src/system/player.c:764`): resolves `map[m].it`,
    // gates on `char_see_item`, then calls `look_item(cn, it+in, -1)`.
    let mut world = World::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);

    let mut item = test_item(ItemId(10), 100, ItemFlags::USED);
    item.name = "Fine Sword".to_string();
    item.description = "A carefully balanced blade.".to_string();
    assert!(world.map.set_item_map(&mut item, 11, 10));
    world.add_item(item);

    let result = apply_inventory_client_action(
        &mut world,
        None,
        character_id,
        &ClientAction::LookItem { x: 11, y: 10 },
        1,
    );

    assert_eq!(
        result,
        InventoryCommandResult::Look("Fine Sword:\nA carefully balanced blade.".to_string())
    );
}

#[test]
fn look_item_ignores_out_of_bounds_coordinates() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);

    // C: `x<1||x>=MAXMAP-1||y<1||y>=MAXMAP-1` bounds guard in `cl_look_item`.
    let result = apply_inventory_client_action(
        &mut world,
        None,
        character_id,
        &ClientAction::LookItem { x: 0, y: 10 },
        1,
    );

    assert_eq!(result, InventoryCommandResult::Ignored);
}

#[test]
fn look_item_ignores_empty_tile() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);

    // C: `if (!(in = map[m].it)) { return; }`
    let result = apply_inventory_client_action(
        &mut world,
        None,
        character_id,
        &ClientAction::LookItem { x: 11, y: 10 },
        1,
    );

    assert_eq!(result, InventoryCommandResult::Ignored);
}

#[test]
fn look_item_ignores_carried_items() {
    // C `char_see_item` (`src/system/see.c:159`): `if (it[in].carried) return 0;`
    // A carried item never rests on `map[m].it`, so the tile lookup itself
    // already fails; assert the end-to-end handler still no-ops rather than
    // panicking if item bookkeeping ever regresses.
    let mut world = World::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);

    let mut item = test_item(ItemId(10), 100, ItemFlags::USED);
    item.carried_by = Some(character_id);
    world.add_item(item);

    let result = apply_inventory_client_action(
        &mut world,
        None,
        character_id,
        &ClientAction::LookItem { x: 11, y: 10 },
        1,
    );

    assert_eq!(result, InventoryCommandResult::Ignored);
}

#[test]
fn look_item_ignores_items_out_of_line_of_sight_range() {
    // C `char_see_item` gates via `los_can_see(..., DISTMAX)`; a character
    // far outside `DIST_MAX` (40) tiles cannot look at a map item.
    let mut world = World::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);

    let mut item = test_item(ItemId(10), 100, ItemFlags::USED);
    item.name = "Fine Sword".to_string();
    assert!(world.map.set_item_map(&mut item, 200, 200));
    world.add_item(item);

    let result = apply_inventory_client_action(
        &mut world,
        None,
        character_id,
        &ClientAction::LookItem { x: 200, y: 200 },
        1,
    );

    assert_eq!(result, InventoryCommandResult::Ignored);
}

#[test]
fn logout_save_omits_arkhata_stopwatch_from_inventory_snapshot() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 37, 10, 10);
    character.inventory[30] = Some(ItemId(10));
    character.inventory[31] = Some(ItemId(11));
    world.add_character(character.clone());

    let mut stopwatch = test_item_with_driver(ItemId(10), IDR_ARKHATA);
    stopwatch.carried_by = Some(character_id);
    stopwatch.driver_data = vec![1];
    world.add_item(stopwatch);
    let mut key_part = test_item_with_driver(ItemId(11), IDR_ARKHATA);
    key_part.carried_by = Some(character_id);
    key_part.driver_data = vec![2];
    world.add_item(key_part);

    let request = character_save_request(
        &world,
        &PlayerRuntime::connected(1, 0),
        &character,
        None,
        37,
        0,
    );

    assert_eq!(request.character.inventory[30], None);
    assert_eq!(request.character.inventory[31], Some(ItemId(11)));
    assert!(!request.items.iter().any(|item| item.id == ItemId(10)));
    assert!(request.items.iter().any(|item| item.id == ItemId(11)));
}

#[test]
fn area_leave_cleanup_removes_arkhata_stopwatch_from_live_inventory() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 37, 10, 10);
    character.inventory[30] = Some(ItemId(10));
    character.inventory[31] = Some(ItemId(11));
    world.add_character(character);

    let mut stopwatch = test_item_with_driver(ItemId(10), IDR_ARKHATA);
    stopwatch.carried_by = Some(character_id);
    stopwatch.driver_data = vec![1];
    world.add_item(stopwatch);
    let mut key_part = test_item_with_driver(ItemId(11), IDR_ARKHATA);
    key_part.carried_by = Some(character_id);
    key_part.driver_data = vec![2];
    world.add_item(key_part);

    let removed = remove_area_leave_vanishing_items(&mut world, character_id);

    assert_eq!(removed, vec![ItemId(10)]);
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.inventory[30], None);
    assert_eq!(character.inventory[31], Some(ItemId(11)));
    assert!(!world.items.contains_key(&ItemId(10)));
    assert!(world.items.contains_key(&ItemId(11)));
}

#[test]
fn grant_template_item_smart_places_xmaspop_in_inventory_first() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::STAFF);
    let mut world = World::default();
    world.add_character(character);
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(r#"xmaspop: name="Christmas Pop" flag=IF_TAKE driver=64 ;"#)
        .unwrap();

    assert_eq!(
        grant_template_item_smart(&mut world, &mut loader, character_id, "xmaspop"),
        Some("Christmas Pop".to_string())
    );
    let character = world.characters.get(&character_id).unwrap();
    let item_id = character.inventory[INVENTORY_START_INVENTORY].unwrap();
    let item = world.items.get(&item_id).unwrap();
    assert_eq!(item.name, "Christmas Pop");
    assert_eq!(item.carried_by, Some(character_id));
}

#[test]
fn grant_template_item_smart_uses_cursor_when_inventory_full() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    for slot in character
        .inventory
        .iter_mut()
        .skip(INVENTORY_START_INVENTORY)
    {
        *slot = Some(ItemId(99));
    }
    let mut world = World::default();
    world.add_character(character);
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(r#"xmaspop: name="Christmas Pop" flag=IF_TAKE driver=64 ;"#)
        .unwrap();

    assert_eq!(
        grant_template_item_smart(&mut world, &mut loader, character_id, "xmaspop"),
        Some("Christmas Pop".to_string())
    );
    let character = world.characters.get(&character_id).unwrap();
    let item_id = character.cursor_item.unwrap();
    assert_eq!(
        world.items.get(&item_id).unwrap().carried_by,
        Some(character_id)
    );
}

#[test]
fn inventory_sort_matches_legacy_value_sprite_name_order() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.inventory[0] = Some(ItemId(99));
    character.inventory[30] = Some(ItemId(1));
    character.inventory[31] = None;
    character.inventory[32] = Some(ItemId(2));
    character.inventory[33] = Some(ItemId(3));
    character.inventory[34] = Some(ItemId(4));
    let mut world = World::default();
    world.add_character(character);

    let mut low_value = test_item(ItemId(1), 500, ItemFlags::USED);
    low_value.name = "Low".to_string();
    low_value.value = 10;
    world.add_item(low_value);
    let mut same_value_high_sprite = test_item(ItemId(2), 700, ItemFlags::USED);
    same_value_high_sprite.name = "Zulu".to_string();
    same_value_high_sprite.value = 50;
    world.add_item(same_value_high_sprite);
    let mut same_value_low_sprite_a = test_item(ItemId(3), 600, ItemFlags::USED);
    same_value_low_sprite_a.name = "Alpha".to_string();
    same_value_low_sprite_a.value = 50;
    world.add_item(same_value_low_sprite_a);
    let mut same_value_low_sprite_b = test_item(ItemId(4), 600, ItemFlags::USED);
    same_value_low_sprite_b.name = "Beta".to_string();
    same_value_low_sprite_b.value = 50;
    world.add_item(same_value_low_sprite_b);

    assert!(inventory_sort(&mut world, character_id));

    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.inventory[0], Some(ItemId(99)));
    assert_eq!(character.inventory[30], Some(ItemId(2)));
    assert_eq!(character.inventory[31], Some(ItemId(3)));
    assert_eq!(character.inventory[32], Some(ItemId(4)));
    assert_eq!(character.inventory[33], Some(ItemId(1)));
    assert_eq!(character.inventory[34], None);
    assert!(character.flags.contains(CharacterFlags::ITEMS));
}

#[test]
fn keyring_command_addall_consumes_inventory_keys() {
    let login = login_block("Tester");
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login, 1, 10, 10);
    let keyring_id = ItemId(90);
    let key_id = ItemId(91);
    let potion_id = ItemId(92);
    character.cursor_item = Some(keyring_id);
    character.inventory[30] = Some(key_id);
    character.inventory[31] = Some(potion_id);
    let mut world = World::default();
    world.add_character(character);
    let mut keyring = test_item(keyring_id, 500, ItemFlags::USE);
    keyring.template_id = IID_KEY_RING;
    keyring.driver = IDR_KEY_RING;
    let mut key = test_item(key_id, 501, ItemFlags::TAKE);
    key.template_id = IID_AREA1_SKELKEY1;
    key.name = "Copper Key".to_string();
    let mut potion = test_item(potion_id, 502, ItemFlags::TAKE);
    potion.template_id = 0x5566_7788;
    potion.name = "Potion".to_string();
    world.add_item(keyring);
    world.add_item(key);
    world.add_item(potion);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(character_id);
    let mut loader = ZoneLoader::new();

    let result = apply_keyring_command(
        &mut world,
        &mut loader,
        &mut player,
        character_id,
        "#keyring addall",
    )
    .expect("keyring command should be recognized");

    assert_eq!(result.messages, vec!["Added 1 keys to your keyring."]);
    assert!(result.inventory_changed);
    assert_eq!(
        player.keyring_key_name(IID_AREA1_SKELKEY1),
        Some("Copper Key")
    );
    assert_eq!(
        world.characters.get(&character_id).unwrap().inventory[30],
        None
    );
    assert_eq!(
        world.characters.get(&character_id).unwrap().inventory[31],
        Some(potion_id)
    );
    assert!(!world.items.contains_key(&key_id));
    assert!(world.items.contains_key(&potion_id));
}

#[test]
fn keyring_command_remove_keeps_entry_when_inventory_is_full() {
    let login = login_block("Tester");
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login, 1, 10, 10);
    let keyring_id = ItemId(90);
    character.cursor_item = Some(keyring_id);
    for slot in 30..character.inventory.len() {
        character.inventory[slot] = Some(ItemId(1_000 + slot as u32));
    }
    let mut world = World::default();
    world.add_character(character);
    let mut keyring = test_item(keyring_id, 500, ItemFlags::USE);
    keyring.template_id = IID_KEY_RING;
    keyring.driver = IDR_KEY_RING;
    world.add_item(keyring);
    for slot in 30..ugaris_core::entity::INVENTORY_SIZE {
        world.add_item(test_item(
            ItemId(1_000 + slot as u32),
            10,
            ItemFlags::USED | ItemFlags::TAKE,
        ));
    }
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(character_id);
    let mut loader = ZoneLoader::new();
    assert_eq!(
        player.add_keyring_key(0x1122_3344, "Copper Key"),
        KeyringAddResult::Added
    );

    let result = apply_keyring_command(
        &mut world,
        &mut loader,
        &mut player,
        character_id,
        "#keyring remove 1",
    )
    .expect("keyring command should be recognized");

    assert_eq!(result.messages, vec!["Your inventory is full."]);
    assert!(!result.inventory_changed);
    assert_eq!(player.keyring_key_name(0x1122_3344), Some("Copper Key"));
}

#[test]
fn login_character_from_template_uses_starter_inventory() {
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
                sword1q1: name="Sword" ;
                torch: name="Torch" ;
                armor1q1: name="Armor" ;
                leggings1q1: name="Leggings" ;
                sleeves1q1: name="Sleeves" ;
                helmet1q1: name="Helmet" ;
                healing_potion1: name="Potion" ;
                recall_scroll2: name="Recall" ;
                "#,
        )
        .unwrap();
    loader
        .load_character_templates_str(
            r#"
                seyan_m:
                    name="Newbie"
                    sprite=2
                    flag=CF_PLAYER
                    flag=CF_MALE
                    flag=CF_ALIVE
                    V_HP=10
                    V_ENDURANCE=10
                    WN_RHAND=sword1q1
                    WN_LHAND=torch
                    item=healing_potion1
                    item=recall_scroll2
                ;
                "#,
        )
        .unwrap();

    let (character, items) = login_character_from_template(
        &mut loader,
        CharacterId(77),
        &login_block("Tester"),
        12,
        42,
        43,
    )
    .unwrap();

    assert_eq!(character.id, CharacterId(77));
    assert_eq!(character.name, "Tester");
    assert_eq!(character.sprite, 2);
    assert!(character.flags.contains(CharacterFlags::WARRIOR));
    assert!(character.flags.contains(CharacterFlags::MAGE));
    assert_eq!(
        (character.rest_area, character.rest_x, character.rest_y),
        (12, 42, 43)
    );
    assert_eq!(character.values[1][CharacterValue::Hp as usize], 10);
    assert_eq!(character.inventory[6], Some(items[0].id));
    assert_eq!(character.inventory[8], Some(items[1].id));
    assert_eq!(character.inventory[30], Some(items[2].id));
    assert_eq!(character.inventory[31], Some(items[3].id));
}

#[test]
fn infinite_chest_context_uses_inventory_key_not_keyring() {
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    character.inventory[30] = Some(ItemId(30));
    let mut key = test_item(ItemId(30), 1, ItemFlags::TAKE);
    key.template_id = 0x1122_3344;
    key.name = "Palace Key".to_string();
    let mut chest = test_item(ItemId(70), 1, ItemFlags::USE);
    chest.driver = ugaris_core::item_driver::IDR_INFINITE_CHEST;
    chest.driver_data = vec![1, 0x44, 0x33, 0x22, 0x11];

    let mut world = World::default();
    world.add_character(character);
    world.add_item(key);
    world.add_item(chest);
    let mut player = PlayerRuntime::connected(5, 0);
    player.add_keyring_key(0x5566_7788, "Wrong Keyring Key");

    let context = item_driver_context_for_request(
        &world,
        Some(&player),
        &ugaris_core::item_driver::ItemDriverRequest::Driver {
            driver: ugaris_core::item_driver::IDR_INFINITE_CHEST,
            item_id: ItemId(70),
            character_id: CharacterId(7),
            spec: 0,
        },
    );

    assert_eq!(context.door_key.unwrap().name, "Palace Key");
}

#[test]
fn apply_chest_treasure_requires_and_accepts_exact_inventory_key() {
    let mut loader = chest_loader();
    let mut world = World::default();
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    character.inventory[30] = Some(ItemId(20));
    world.add_character(character);
    let mut chest = test_item(ItemId(10), 700, ItemFlags::USE);
    chest.driver_data = vec![9, 0x44, 0x33, 0x22, 0x11, 0, 0];
    world.add_item(chest);
    let mut key = test_item(ItemId(20), 701, ItemFlags::TAKE);
    key.name = "Copper Key".to_string();
    key.template_id = 0x1122_3344;
    world.add_item(key);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(7));

    assert_eq!(
        apply_chest_treasure(
            &mut world,
            &mut loader,
            Some(&mut player),
            ItemId(10),
            CharacterId(7),
            9,
            100,
        ),
        ChestTreasureApplyResult::Granted {
            item_name: "Coins".to_string(),
            key_name: Some("Copper Key".to_string()),
        }
    );
}
