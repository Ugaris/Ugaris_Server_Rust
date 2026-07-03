use super::*;

#[test]
fn container_look_uses_legacy_item_text() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.current_container = Some(ItemId(10));
    world.add_character(character);

    let mut container = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::USE);
    container.content_id = 1;
    world.add_item(container);
    let mut stored = test_item(ItemId(20), 1234, ItemFlags::USED | ItemFlags::TAKE);
    stored.name = "Stored Gem".to_string();
    stored.description = "It sparkles.".to_string();
    stored.contained_in = Some(ItemId(10));
    world.add_item(stored);

    let result = apply_item_container_command(
        &mut world,
        character_id,
        &ClientAction::LookContainer { slot: 0 },
    );

    assert_eq!(
        result,
        AccountDepotCommandResult::Look("Stored Gem:\nIt sparkles.".to_string())
    );
}

#[test]
fn container_access_is_cleared_when_player_is_busy_like_c_check_container_item() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.current_container = Some(ItemId(10));
    character.action = action::USE;
    world.add_character(character);

    let mut container = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::USE);
    container.content_id = 1;
    world.add_item(container);

    let result = apply_item_container_command(
        &mut world,
        character_id,
        &ClientAction::LookContainer { slot: 0 },
    );

    assert_eq!(result, AccountDepotCommandResult::Ignored);
    assert_eq!(world.characters[&character_id].current_container, None);
}

#[test]
fn map_container_access_requires_facing_same_item_like_c_check_container_item() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.dir = Direction::Left as u8;
    character.current_container = Some(ItemId(10));
    world.add_character(character);

    let mut container = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::USE);
    container.content_id = 1;
    assert!(world.map.set_item_map(&mut container, 11, 10));
    world.add_item(container);

    let result = apply_item_container_command(
        &mut world,
        character_id,
        &ClientAction::LookContainer { slot: 0 },
    );

    assert_eq!(result, AccountDepotCommandResult::Ignored);
    assert_eq!(world.characters[&character_id].current_container, None);
}

#[test]
fn inventory_use_opens_carried_container() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.inventory[30] = Some(ItemId(10));
    world.add_character(character);
    let mut container = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::USE);
    container.content_id = 1;
    container.carried_by = Some(character_id);
    world.add_item(container);

    let result = apply_inventory_client_action(
        &mut world,
        None,
        character_id,
        &ClientAction::UseInventory { slot: 30 },
        1,
    );

    assert_eq!(
        result,
        InventoryCommandResult::ContainerOpened {
            account_depot: false
        }
    );
    assert_eq!(
        world.characters[&character_id].current_container,
        Some(ItemId(10))
    );
}

#[test]
fn generic_container_payload_uses_open_item_description_and_clears_empty_slots() {
    let mut world = World::default();
    let mut container = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::USE);
    container.description = "Opened Chest".to_string();
    container.content_id = 22;
    world.add_item(container);
    let mut stored = test_item(ItemId(20), 0x11223344, ItemFlags::USED | ItemFlags::TAKE);
    stored.contained_in = Some(ItemId(10));
    world.add_item(stored);

    let payload = generic_container_payload(&world, ItemId(10));

    assert_eq!(&payload[..2], &[SV_CONTYPE, 1]);
    assert!(payload.windows(14).any(|window| {
        window
            == [
                SV_CONNAME, 12, b'O', b'p', b'e', b'n', b'e', b'd', b' ', b'C', b'h', b'e', b's',
                b't',
            ]
    }));
    assert!(payload.windows(2).any(|window| window == [SV_CONCNT, 108]));
    assert!(payload
        .windows(6)
        .any(|window| window == [SV_CONTAINER, 0, 0x44, 0x33, 0x22, 0x11]));
    assert!(payload
        .windows(6)
        .any(|window| window == [SV_CONTAINER, 1, 0, 0, 0, 0]));
}

#[test]
fn generic_container_swap_exchanges_cursor_and_container_item() {
    let character_id = CharacterId(7);
    let cursor_id = ItemId(30);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.current_container = Some(ItemId(10));
    character.cursor_item = Some(cursor_id);

    let mut world = World::default();
    world.add_character(character);
    let mut container = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::USE);
    container.content_id = 22;
    world.add_item(container);
    let mut stored = test_item(ItemId(20), 2222, ItemFlags::USED | ItemFlags::TAKE);
    stored.contained_in = Some(ItemId(10));
    world.add_item(stored);
    let mut cursor = test_item(cursor_id, 3333, ItemFlags::USED | ItemFlags::TAKE);
    cursor.carried_by = Some(character_id);
    world.add_item(cursor);

    assert_eq!(
        apply_item_container_command(
            &mut world,
            character_id,
            &ClientAction::Container {
                slot: 0,
                fast: false,
            },
        ),
        AccountDepotCommandResult::Changed
    );

    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.cursor_item, Some(ItemId(20)));
    assert_eq!(
        world.items.get(&ItemId(20)).unwrap().carried_by,
        Some(character_id)
    );
    assert_eq!(
        world.items.get(&cursor_id).unwrap().contained_in,
        Some(ItemId(10))
    );
}

#[test]
fn generic_container_fast_swap_stores_withdrawn_item_in_inventory() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.current_container = Some(ItemId(10));
    let mut world = World::default();
    world.add_character(character);
    let mut container = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::USE);
    container.content_id = 22;
    world.add_item(container);
    let mut stored = test_item(ItemId(20), 2222, ItemFlags::USED | ItemFlags::TAKE);
    stored.contained_in = Some(ItemId(10));
    world.add_item(stored);

    assert_eq!(
        apply_item_container_command(
            &mut world,
            character_id,
            &ClientAction::Container {
                slot: 0,
                fast: true,
            },
        ),
        AccountDepotCommandResult::Changed
    );

    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.cursor_item, None);
    assert_eq!(
        character.inventory[INVENTORY_START_INVENTORY],
        Some(ItemId(20))
    );
    assert_eq!(
        world.items.get(&ItemId(20)).unwrap().carried_by,
        Some(character_id)
    );
}

#[test]
fn generic_container_blocks_quest_cursor_storage() {
    let character_id = CharacterId(7);
    let cursor_id = ItemId(30);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.current_container = Some(ItemId(10));
    character.cursor_item = Some(cursor_id);
    let mut world = World::default();
    world.add_character(character);
    let mut container = test_item(ItemId(10), 100, ItemFlags::USED | ItemFlags::USE);
    container.content_id = 22;
    world.add_item(container);
    let mut cursor = test_item(cursor_id, 3333, ItemFlags::USED | ItemFlags::QUEST);
    cursor.carried_by = Some(character_id);
    world.add_item(cursor);

    assert_eq!(
        apply_item_container_command(
            &mut world,
            character_id,
            &ClientAction::Container {
                slot: 0,
                fast: false,
            },
        ),
        AccountDepotCommandResult::Blocked(
            "You cannot store quest items in a container.".to_string()
        )
    );
    assert_eq!(
        world.characters.get(&character_id).unwrap().cursor_item,
        Some(cursor_id)
    );
    assert_eq!(world.items.get(&cursor_id).unwrap().contained_in, None);
}
