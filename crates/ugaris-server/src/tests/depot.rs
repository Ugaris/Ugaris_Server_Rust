use super::*;
use ugaris_core::player::MAXDEPOT;

#[test]
fn account_depot_swap_moves_cursor_item_into_snapshot_slot() {
    let character_id = CharacterId(7);
    let cursor_id = ItemId(20);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.current_container = Some(ItemId(10));
    character.cursor_item = Some(cursor_id);

    let mut world = World::default();
    world.add_character(character);
    let mut depot_item = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::USE);
    depot_item.driver = IDR_ACCOUNT_DEPOT;
    world.add_item(depot_item);
    let mut cursor = test_item(cursor_id, 1234, ItemFlags::USED | ItemFlags::TAKE);
    cursor.carried_by = Some(character_id);
    world.add_item(cursor);
    let mut depot = AccountDepotState::default();

    assert_eq!(
        apply_account_depot_command(
            &mut world,
            &mut depot,
            character_id,
            &ClientAction::Container {
                slot: 3,
                fast: false,
            },
        ),
        AccountDepotCommandResult::Changed
    );

    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.cursor_item, None);
    assert!(!world.items.contains_key(&cursor_id));
    assert_eq!(depot.slots[3].as_ref().unwrap().sprite, 1234);
    assert_eq!(
        depot.slots[3].as_ref().unwrap().contained_in,
        Some(ItemId(10))
    );
}

#[test]
fn account_depot_swap_withdraws_snapshot_to_cursor_with_new_live_id() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.current_container = Some(ItemId(10));

    let mut world = World::default();
    world.add_character(character);
    let mut depot_item = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::USE);
    depot_item.driver = IDR_ACCOUNT_DEPOT;
    world.add_item(depot_item);
    let mut stored = test_item(ItemId(99), 2222, ItemFlags::USED | ItemFlags::TAKE);
    stored.name = "Stored".to_string();
    let mut depot = AccountDepotState::default();
    depot.slots[4] = Some(stored);

    assert_eq!(
        apply_account_depot_command(
            &mut world,
            &mut depot,
            character_id,
            &ClientAction::Container {
                slot: 4,
                fast: false,
            },
        ),
        AccountDepotCommandResult::Changed
    );

    let cursor_id = world
        .characters
        .get(&character_id)
        .unwrap()
        .cursor_item
        .unwrap();
    assert_ne!(cursor_id, ItemId(99));
    let cursor = world.items.get(&cursor_id).unwrap();
    assert_eq!(cursor.name, "Stored");
    assert_eq!(cursor.carried_by, Some(character_id));
    assert!(depot.slots[4].is_none());
}

#[test]
fn account_depot_blocks_quest_and_nodepot_items() {
    let character_id = CharacterId(7);
    let cursor_id = ItemId(20);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.current_container = Some(ItemId(10));
    character.cursor_item = Some(cursor_id);

    let mut world = World::default();
    world.add_character(character);
    let mut depot_item = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::USE);
    depot_item.driver = IDR_ACCOUNT_DEPOT;
    world.add_item(depot_item);
    let mut cursor = test_item(cursor_id, 1234, ItemFlags::USED | ItemFlags::QUEST);
    cursor.carried_by = Some(character_id);
    world.add_item(cursor);
    let mut depot = AccountDepotState::default();

    assert_eq!(
        apply_account_depot_command(
            &mut world,
            &mut depot,
            character_id,
            &ClientAction::Container {
                slot: 0,
                fast: false,
            },
        ),
        AccountDepotCommandResult::Blocked("You cannot store this item in the depot.".to_string())
    );
    assert_eq!(
        world.characters.get(&character_id).unwrap().cursor_item,
        Some(cursor_id)
    );
    assert!(depot.slots[0].is_none());
}

#[test]
fn account_depot_payload_matches_legacy_container_header_and_slots() {
    let mut depot = AccountDepotState::default();
    depot.slots[2] = Some(test_item(ItemId(99), 0x11223344, ItemFlags::USED));

    let payload = account_depot_payload(&depot);

    assert_eq!(&payload[..2], &[SV_CONTYPE, 1]);
    assert_eq!(payload[2], SV_CONNAME);
    assert!(payload.windows(2).any(|window| window == [SV_CONCNT, 110]));
    assert!(payload
        .windows(6)
        .any(|window| { window == [SV_CONTAINER, 2, 0x44, 0x33, 0x22, 0x11] }));
}

#[test]
fn account_depot_sort_command_requires_open_account_depot() {
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    let mut world = World::default();
    world.add_character(character);
    let mut runtime = ServerRuntime::default();
    let depot = runtime.ensure_account_depot(character_id);
    depot.slots[0] = Some(test_item(ItemId(20), 100, ItemFlags::USED));
    depot.slots[1] = Some(test_item(ItemId(21), 200, ItemFlags::USED));

    assert!(!account_depot_sort_if_open(
        &mut world,
        &mut runtime,
        character_id
    ));

    let depot = runtime.account_depots.get(&character_id).unwrap();
    assert_eq!(depot.slots[0].as_ref().unwrap().sprite, 100);
    assert_eq!(depot.slots[1].as_ref().unwrap().sprite, 200);
}

#[test]
fn account_depot_sort_command_sorts_when_account_depot_is_open() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.current_container = Some(ItemId(10));
    let mut world = World::default();
    world.add_character(character);
    let mut depot_item = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::USE);
    depot_item.driver = IDR_ACCOUNT_DEPOT;
    world.add_item(depot_item);
    let mut runtime = ServerRuntime::default();
    let depot = runtime.ensure_account_depot(character_id);
    depot.slots[0] = Some(test_item(ItemId(20), 100, ItemFlags::USED));
    depot.slots[1] = Some(test_item(ItemId(21), 200, ItemFlags::USED));

    assert!(account_depot_sort_if_open(
        &mut world,
        &mut runtime,
        character_id
    ));

    let depot = runtime.account_depots.get(&character_id).unwrap();
    assert_eq!(depot.slots[0].as_ref().unwrap().sprite, 200);
    assert_eq!(depot.slots[1].as_ref().unwrap().sprite, 100);
}

#[test]
fn account_depot_blob_encodes_c_struct_item_layout() {
    let mut depot = AccountDepotState::default();
    let mut item = test_item(
        ItemId(99),
        -12345,
        ItemFlags::USED | ItemFlags::TAKE | ItemFlags::NODEPOT,
    );
    item.name = "Long Stored Relic Name That Fits".to_string();
    item.description = "A relic in the account depot.".to_string();
    item.value = 12_345;
    item.min_level = 7;
    item.max_level = 77;
    item.needs_class = 3;
    item.owner_id = -44;
    item.modifier_index = [1, -2, 3, -4, 5];
    item.modifier_value = [10, 20, 30, 40, 50];
    item.content_id = 17;
    item.driver = IDR_TORCH;
    item.driver_data = (0..50).collect();
    item.template_id = 0x0102_0304;
    item.serial = 0xAABB_CCDD;
    depot.slots[5] = Some(item);

    let bytes = encode_legacy_account_depot_blob(&depot);

    assert_eq!(bytes.len(), LEGACY_ACCOUNT_DEPOT_ITEM_SIZE);
    assert_eq!(
        u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
        (ItemFlags::USED | ItemFlags::TAKE | ItemFlags::NODEPOT).bits()
    );
    assert_eq!(
        &bytes[LEGACY_ACCOUNT_DEPOT_NAME_OFFSET..LEGACY_ACCOUNT_DEPOT_NAME_OFFSET + 4],
        b"Long"
    );
    assert_eq!(
        u32::from_le_bytes(
            bytes[LEGACY_ACCOUNT_DEPOT_VALUE_OFFSET..LEGACY_ACCOUNT_DEPOT_VALUE_OFFSET + 4]
                .try_into()
                .unwrap()
        ),
        12_345
    );
    assert_eq!(bytes[LEGACY_ACCOUNT_DEPOT_MIN_LEVEL_OFFSET], 7);
    assert_eq!(
        i16::from_le_bytes(
            bytes[LEGACY_ACCOUNT_DEPOT_MOD_INDEX_OFFSET + 2
                ..LEGACY_ACCOUNT_DEPOT_MOD_INDEX_OFFSET + 4]
                .try_into()
                .unwrap()
        ),
        -2
    );
    assert_eq!(
        u16::from_le_bytes(
            bytes[LEGACY_ACCOUNT_DEPOT_DRIVER_OFFSET..LEGACY_ACCOUNT_DEPOT_DRIVER_OFFSET + 2]
                .try_into()
                .unwrap()
        ),
        IDR_TORCH
    );
    assert_eq!(
        &bytes[LEGACY_ACCOUNT_DEPOT_DRDATA_OFFSET..LEGACY_ACCOUNT_DEPOT_DRDATA_OFFSET + 40],
        &(0u8..40).collect::<Vec<_>>()[..]
    );
    assert_eq!(
        u32::from_le_bytes(
            bytes[LEGACY_ACCOUNT_DEPOT_TEMPLATE_ID_OFFSET
                ..LEGACY_ACCOUNT_DEPOT_TEMPLATE_ID_OFFSET + 4]
                .try_into()
                .unwrap()
        ),
        0x0102_0304
    );
    assert_eq!(
        i32::from_le_bytes(
            bytes[LEGACY_ACCOUNT_DEPOT_SPRITE_OFFSET..LEGACY_ACCOUNT_DEPOT_SPRITE_OFFSET + 4]
                .try_into()
                .unwrap()
        ),
        -12345
    );
    assert!(bytes[LEGACY_ACCOUNT_DEPOT_ITEM_PERSISTED_PREFIX..]
        .iter()
        .all(|&b| b == 0));
}

#[test]
fn account_depot_blob_decodes_items_into_dense_legacy_slots() {
    let mut item = test_item(ItemId(99), 4321, ItemFlags::USED | ItemFlags::TAKE);
    item.name = "Stored Gem".to_string();
    item.description = "It sparkles.".to_string();
    item.value = 88;
    item.modifier_index = [7, 0, 0, 0, 0];
    item.modifier_value = [9, 0, 0, 0, 0];
    item.driver = IDR_FOOD;
    item.driver_data = vec![3, 2, 1];
    item.template_id = 0x1234_5678;
    item.serial = 123;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&encode_legacy_account_depot_item(&item));
    bytes.extend_from_slice(&[0xFF; 17]);

    let depot = decode_legacy_account_depot_blob(&bytes);
    let decoded = depot.slots[0].as_ref().unwrap();

    assert_eq!(decoded.id, ItemId(1));
    assert_eq!(decoded.name, "Stored Gem");
    assert_eq!(decoded.description, "It sparkles.");
    assert_eq!(decoded.flags, ItemFlags::USED | ItemFlags::TAKE);
    assert_eq!(decoded.sprite, 4321);
    assert_eq!(decoded.value, 88);
    assert_eq!(decoded.modifier_index[0], 7);
    assert_eq!(decoded.modifier_value[0], 9);
    assert_eq!(decoded.driver, IDR_FOOD);
    assert_eq!(&decoded.driver_data[..3], &[3, 2, 1]);
    assert_eq!(decoded.template_id, 0x1234_5678);
    assert_eq!(decoded.serial, 123);
    assert_eq!(decoded.x, 0);
    assert_eq!(decoded.y, 0);
    assert_eq!(decoded.carried_by, None);
    assert_eq!(decoded.contained_in, None);
    assert!(depot.slots[1].is_none());
}

#[test]
fn account_depot_subscriber_blob_replaces_block_and_preserves_unknown() {
    let unknown_id = (77 << 24) | 9;
    let mut existing = Vec::new();
    write_legacy_subscriber_block(&mut existing, unknown_id, &[1, 2, 3]);
    write_legacy_subscriber_block(&mut existing, DRD_ACCOUNT_WIDE_DEPOT, &[9, 9, 9]);

    let mut depot = AccountDepotState::default();
    let mut item = test_item(ItemId(99), 1234, ItemFlags::USED | ItemFlags::TAKE);
    item.name = "Stored Gem".to_string();
    depot.slots[2] = Some(item);

    let encoded = encode_legacy_account_depot_subscriber_blob(&existing, Some(&depot));
    let blocks = parse_legacy_subscriber_blocks(&encoded).unwrap();

    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].id, unknown_id);
    assert_eq!(blocks[0].data, &[1, 2, 3]);
    assert_eq!(blocks[1].id, DRD_ACCOUNT_WIDE_DEPOT);
    let decoded = decode_legacy_account_depot_subscriber_blob(&encoded).unwrap();
    assert_eq!(decoded.slots[0].as_ref().unwrap().name, "Stored Gem");
}

#[test]
fn account_depot_subscriber_blob_omits_empty_depot_like_c_del_data() {
    let mut existing = Vec::new();
    write_legacy_subscriber_block(&mut existing, DRD_ACCOUNT_WIDE_DEPOT, &[9, 9, 9]);

    let encoded =
        encode_legacy_account_depot_subscriber_blob(&existing, Some(&AccountDepotState::default()));

    assert!(parse_legacy_subscriber_blocks(&encoded).unwrap().is_empty());
    assert!(decode_legacy_account_depot_subscriber_blob(&encoded).is_none());
}

// Personal (`IF_DEPOT`, `DRD_DEPOT_PPD`) legacy storage depot - a distinct,
// older, per-character system from the account-wide depot above.

fn depot_access_item(id: ItemId) -> Item {
    let mut item = test_item(id, 0, ItemFlags::USED | ItemFlags::USE | ItemFlags::DEPOT);
    item.content_id = 0;
    item
}

#[test]
fn personal_depot_swap_moves_cursor_item_into_slot() {
    let character_id = CharacterId(7);
    let cursor_id = ItemId(20);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.current_container = Some(ItemId(10));
    character.cursor_item = Some(cursor_id);

    let mut world = World::default();
    world.add_character(character);
    world.add_item(depot_access_item(ItemId(10)));
    let mut cursor = test_item(cursor_id, 1234, ItemFlags::USED | ItemFlags::TAKE);
    cursor.carried_by = Some(character_id);
    world.add_item(cursor);
    let mut depot: Vec<Option<Item>> = vec![None; MAXDEPOT];

    assert_eq!(
        apply_personal_depot_command(
            &mut world,
            &mut depot,
            character_id,
            &ClientAction::Container {
                slot: 3,
                fast: false,
            },
        ),
        AccountDepotCommandResult::Changed
    );

    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.cursor_item, None);
    assert!(!world.items.contains_key(&cursor_id));
    assert_eq!(depot[3].as_ref().unwrap().sprite, 1234);
}

#[test]
fn personal_depot_swap_withdraws_slot_to_cursor_with_new_live_id() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.current_container = Some(ItemId(10));

    let mut world = World::default();
    world.add_character(character);
    world.add_item(depot_access_item(ItemId(10)));
    let mut stored = test_item(ItemId(99), 2222, ItemFlags::USED | ItemFlags::TAKE);
    stored.name = "Stored".to_string();
    let mut depot: Vec<Option<Item>> = vec![None; MAXDEPOT];
    depot[4] = Some(stored);

    assert_eq!(
        apply_personal_depot_command(
            &mut world,
            &mut depot,
            character_id,
            &ClientAction::Container {
                slot: 4,
                fast: false,
            },
        ),
        AccountDepotCommandResult::Changed
    );

    let cursor_id = world
        .characters
        .get(&character_id)
        .unwrap()
        .cursor_item
        .unwrap();
    assert_ne!(cursor_id, ItemId(99));
    let cursor = world.items.get(&cursor_id).unwrap();
    assert_eq!(cursor.name, "Stored");
    assert_eq!(cursor.carried_by, Some(character_id));
    assert!(depot[4].is_none());
}

#[test]
fn personal_depot_blocks_nodepot_items_silently_unlike_account_depot() {
    let character_id = CharacterId(7);
    let cursor_id = ItemId(20);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.current_container = Some(ItemId(10));
    character.cursor_item = Some(cursor_id);

    let mut world = World::default();
    world.add_character(character);
    world.add_item(depot_access_item(ItemId(10)));
    let mut cursor = test_item(cursor_id, 1234, ItemFlags::USED | ItemFlags::NODEPOT);
    cursor.carried_by = Some(character_id);
    world.add_item(cursor);
    let mut depot: Vec<Option<Item>> = vec![None; MAXDEPOT];

    // No message (`Ignored`, not `Blocked`) - C's `swap_depot` never
    // `log_char`s on `IF_NODEPOT`, unlike `account_depot_swap`.
    assert_eq!(
        apply_personal_depot_command(
            &mut world,
            &mut depot,
            character_id,
            &ClientAction::Container {
                slot: 0,
                fast: false,
            },
        ),
        AccountDepotCommandResult::Ignored
    );
    assert_eq!(
        world.characters.get(&character_id).unwrap().cursor_item,
        Some(cursor_id)
    );
    assert!(depot[0].is_none());
}

#[test]
fn personal_depot_allows_quest_items_unlike_account_depot() {
    let character_id = CharacterId(7);
    let cursor_id = ItemId(20);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.current_container = Some(ItemId(10));
    character.cursor_item = Some(cursor_id);

    let mut world = World::default();
    world.add_character(character);
    world.add_item(depot_access_item(ItemId(10)));
    let mut cursor = test_item(cursor_id, 1234, ItemFlags::USED | ItemFlags::QUEST);
    cursor.carried_by = Some(character_id);
    world.add_item(cursor);
    let mut depot: Vec<Option<Item>> = vec![None; MAXDEPOT];

    assert_eq!(
        apply_personal_depot_command(
            &mut world,
            &mut depot,
            character_id,
            &ClientAction::Container {
                slot: 0,
                fast: false,
            },
        ),
        AccountDepotCommandResult::Changed
    );
    assert!(depot[0].as_ref().unwrap().flags.contains(ItemFlags::QUEST));
}

#[test]
fn personal_depot_fast_store_finds_first_empty_slot_ignoring_clicked_slot() {
    let character_id = CharacterId(7);
    let cursor_id = ItemId(20);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.current_container = Some(ItemId(10));
    character.cursor_item = Some(cursor_id);

    let mut world = World::default();
    world.add_character(character);
    world.add_item(depot_access_item(ItemId(10)));
    let mut cursor = test_item(cursor_id, 1234, ItemFlags::USED | ItemFlags::TAKE);
    cursor.carried_by = Some(character_id);
    world.add_item(cursor);
    let mut depot: Vec<Option<Item>> = vec![None; MAXDEPOT];
    depot[0] = Some(test_item(ItemId(50), 1, ItemFlags::USED));
    depot[1] = Some(test_item(ItemId(51), 2, ItemFlags::USED));

    // Clicked slot 40, but slot 2 is the first empty one and must be used
    // instead (C ignores the clicked `nr` entirely in this branch).
    assert_eq!(
        apply_personal_depot_command(
            &mut world,
            &mut depot,
            character_id,
            &ClientAction::Container {
                slot: 40,
                fast: true,
            },
        ),
        AccountDepotCommandResult::Changed
    );
    assert_eq!(depot[2].as_ref().unwrap().sprite, 1234);
    assert!(depot[40].is_none());
}

#[test]
fn personal_depot_fast_store_is_noop_when_depot_is_full() {
    let character_id = CharacterId(7);
    let cursor_id = ItemId(20);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.current_container = Some(ItemId(10));
    character.cursor_item = Some(cursor_id);

    let mut world = World::default();
    world.add_character(character);
    world.add_item(depot_access_item(ItemId(10)));
    let mut cursor = test_item(cursor_id, 1234, ItemFlags::USED | ItemFlags::TAKE);
    cursor.carried_by = Some(character_id);
    world.add_item(cursor);
    let mut depot: Vec<Option<Item>> = (0..MAXDEPOT)
        .map(|index| {
            Some(test_item(
                ItemId(1000 + index as u32),
                index as i32,
                ItemFlags::USED,
            ))
        })
        .collect();

    assert_eq!(
        apply_personal_depot_command(
            &mut world,
            &mut depot,
            character_id,
            &ClientAction::Container {
                slot: 0,
                fast: true,
            },
        ),
        AccountDepotCommandResult::Ignored
    );
    assert_eq!(
        world.characters.get(&character_id).unwrap().cursor_item,
        Some(cursor_id)
    );
}

#[test]
fn personal_depot_fast_withdraw_stores_directly_into_first_free_inventory_slot() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.current_container = Some(ItemId(10));
    for slot in character.inventory[INVENTORY_START_INVENTORY..].iter_mut() {
        *slot = Some(ItemId(9999));
    }
    character.inventory[INVENTORY_START_INVENTORY + 5] = None;

    let mut world = World::default();
    world.add_character(character);
    world.add_item(depot_access_item(ItemId(10)));
    let mut stored = test_item(ItemId(99), 2222, ItemFlags::USED | ItemFlags::TAKE);
    stored.name = "Stored".to_string();
    let mut depot: Vec<Option<Item>> = vec![None; MAXDEPOT];
    depot[4] = Some(stored);

    assert_eq!(
        apply_personal_depot_command(
            &mut world,
            &mut depot,
            character_id,
            &ClientAction::Container {
                slot: 4,
                fast: true,
            },
        ),
        AccountDepotCommandResult::Changed
    );

    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.cursor_item, None);
    let stored_item_id = character.inventory[INVENTORY_START_INVENTORY + 5].unwrap();
    assert_eq!(world.items.get(&stored_item_id).unwrap().name, "Stored");
    assert!(depot[4].is_none());
}

#[test]
fn personal_depot_look_returns_text_only_for_populated_slots() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.current_container = Some(ItemId(10));

    let mut world = World::default();
    world.add_character(character);
    world.add_item(depot_access_item(ItemId(10)));
    let mut depot: Vec<Option<Item>> = vec![None; MAXDEPOT];
    let mut item = test_item(ItemId(50), 1, ItemFlags::USED);
    item.name = "A Sword".to_string();
    depot[0] = Some(item);

    let result = apply_personal_depot_command(
        &mut world,
        &mut depot,
        character_id,
        &ClientAction::LookContainer { slot: 0 },
    );
    assert!(matches!(result, AccountDepotCommandResult::Look(text) if text.contains("A Sword")));

    let result = apply_personal_depot_command(
        &mut world,
        &mut depot,
        character_id,
        &ClientAction::LookContainer { slot: 1 },
    );
    assert_eq!(result, AccountDepotCommandResult::Ignored);
}

#[test]
fn personal_depot_payload_matches_legacy_container_header_and_slots() {
    let mut depot: Vec<Option<Item>> = vec![None; MAXDEPOT];
    depot[2] = Some(test_item(ItemId(99), 0x11223344, ItemFlags::USED));

    let payload = personal_depot_payload(&depot);

    assert_eq!(&payload[..2], &[SV_CONTYPE, 1]);
    assert_eq!(payload[2], SV_CONNAME);
    assert!(payload
        .windows(2)
        .any(|window| window == [SV_CONCNT, MAXDEPOT as u8]));
    assert!(payload
        .windows(6)
        .any(|window| { window == [SV_CONTAINER, 2, 0x44, 0x33, 0x22, 0x11] }));
}

#[test]
fn personal_depot_sort_command_sorts_regardless_of_open_state() {
    let character_id = CharacterId(7);
    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(character_id);
    player.depot[0] = Some(test_item(ItemId(20), 100, ItemFlags::USED));
    player.depot[1] = Some(test_item(ItemId(21), 200, ItemFlags::USED));
    runtime.players.insert(1, player);

    // Unlike `/accountdepotsort`, C's `depot_sort` never checks whether
    // the depot is currently open - it always sorts.
    personal_depot_sort_command(&mut runtime, character_id);

    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.depot[0].as_ref().unwrap().sprite, 200);
    assert_eq!(player.depot[1].as_ref().unwrap().sprite, 100);
}

#[test]
fn personal_depot_sort_orders_by_sprite_then_value_then_name_with_empties_last() {
    let mut depot: Vec<Option<Item>> = vec![None; MAXDEPOT];
    let mut low_sprite = test_item(ItemId(1), 10, ItemFlags::USED);
    low_sprite.value = 999;
    let mut high_sprite = test_item(ItemId(2), 20, ItemFlags::USED);
    high_sprite.value = 1;
    depot[0] = Some(low_sprite);
    depot[1] = Some(high_sprite);
    depot[5] = None;

    personal_depot_sort(&mut depot);

    // Highest sprite sorts first.
    assert_eq!(depot[0].as_ref().unwrap().sprite, 20);
    assert_eq!(depot[1].as_ref().unwrap().sprite, 10);
    assert!(depot[2..].iter().all(Option::is_none));
}

#[test]
fn depot_ppd_item_codec_round_trips_via_player_runtime() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut item = test_item(ItemId(1), -500, ItemFlags::USED | ItemFlags::NODEPOT);
    item.name = "Depot Item".to_string();
    item.value = 42;
    player.depot[7] = Some(item);

    let encoded = player.encode_legacy_depot_ppd();
    let mut decoded_player = PlayerRuntime::connected(2, 0);
    assert!(decoded_player.decode_legacy_depot_ppd(&encoded));

    let decoded = decoded_player.depot[7].as_ref().unwrap();
    assert_eq!(decoded.name, "Depot Item");
    assert_eq!(decoded.value, 42);
    assert_eq!(decoded.sprite, -500);
    assert!(decoded.flags.contains(ItemFlags::NODEPOT));
    assert!(decoded_player.depot[0].is_none());
}
