use super::*;

#[test]
fn keyring_command_requires_keyring_on_cursor() {
    let login = login_block("Tester");
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login, 1, 10, 10);
    let mut world = World::default();
    world.add_character(character);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(character_id);
    let mut loader = ZoneLoader::new();

    let result = apply_keyring_command(
        &mut world,
        &mut loader,
        &mut player,
        character_id,
        "#keyring",
    )
    .expect("keyring command should be recognized");

    assert_eq!(
        result.messages,
        vec!["You need to hold a keyring on your cursor to use this command."]
    );
    assert!(!result.inventory_changed);
}

#[test]
fn keyring_command_addallkeys_requires_staff_and_uses_registered_templates() {
    let login = login_block("Tester");
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login, 1, 10, 10);
    let keyring_id = ItemId(90);
    character.cursor_item = Some(keyring_id);
    let mut world = World::default();
    world.add_character(character);
    let mut keyring = test_item(keyring_id, 500, ItemFlags::USE);
    keyring.template_id = IID_KEY_RING;
    keyring.driver = IDR_KEY_RING;
    world.add_item(keyring);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(character_id);
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
                CopperKey:
                  name="Copper Key"
                  ID=1000002
                  flag=IF_TAKE
                ;
                UnregisteredKey:
                  name="Unregistered Key"
                  ID=55667788
                  flag=IF_TAKE
                ;
                "#,
        )
        .unwrap();

    let denied = apply_keyring_command(
        &mut world,
        &mut loader,
        &mut player,
        character_id,
        "#keyring addallkeys",
    )
    .expect("keyring command should be recognized");
    assert_eq!(
        denied.messages,
        vec!["This command requires staff privileges."]
    );
    assert_eq!(player.keyring.len(), 0);

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::STAFF);
    let added = apply_keyring_command(
        &mut world,
        &mut loader,
        &mut player,
        character_id,
        "#keyring addallkeys",
    )
    .expect("keyring command should be recognized");

    assert_eq!(
        added.messages,
        vec![
            "Adding all registered keys to keyring...",
            "Added 1 keys to your keyring (total: 1/100).",
        ]
    );
    assert_eq!(
        player.keyring_key_name(IID_AREA1_SKELKEY1),
        Some("Copper Key")
    );
    assert_eq!(player.keyring.len(), 1);
}

#[test]
fn keyring_command_remove_and_auto_match_legacy_feedback() {
    let login = login_block("Tester");
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login, 1, 10, 10);
    let keyring_id = ItemId(90);
    character.cursor_item = Some(keyring_id);
    let mut world = World::default();
    world.add_character(character);
    let mut keyring = test_item(keyring_id, 500, ItemFlags::USE);
    keyring.template_id = IID_KEY_RING;
    keyring.driver = IDR_KEY_RING;
    world.add_item(keyring);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(character_id);
    let mut loader = ZoneLoader::new();
    assert_eq!(
        player.add_keyring_key(0x1122_3344, "Copper Key"),
        KeyringAddResult::Added
    );

    let removed = apply_keyring_command(
        &mut world,
        &mut loader,
        &mut player,
        character_id,
        "#keyring remove 1",
    )
    .expect("keyring command should be recognized");
    let auto = apply_keyring_command(
        &mut world,
        &mut loader,
        &mut player,
        character_id,
        "#keyring auto",
    )
    .expect("keyring command should be recognized");

    assert_eq!(
        removed.messages,
        vec!["Removed Copper Key from your keyring."]
    );
    assert!(removed.inventory_changed);
    assert_eq!(player.keyring_key_name(0x1122_3344), None);
    let character = world.characters.get(&character_id).unwrap();
    let restored_key_id = character.inventory[30].expect("removed key should be restored");
    let restored_key = world.items.get(&restored_key_id).unwrap();
    assert_eq!(restored_key.template_id, 0x1122_3344);
    assert_eq!(restored_key.name, "Copper Key");
    assert_eq!(restored_key.carried_by, Some(character_id));
    assert_eq!(
        auto.messages,
        vec!["Auto-add keys enabled. Keys will be automatically added to your keyring when picked up."]
    );
    assert!(player.keyring_auto_add());
}

#[test]
fn apply_keyring_add_cursor_item_stores_key_and_consumes_cursor() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let key_item_id = ItemId(44);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(key_item_id);
    world.add_character(character);
    let mut key = test_item(key_item_id, 1200, ItemFlags::USED | ItemFlags::TAKE);
    key.name = "Copper Key".to_string();
    key.template_id = IID_AREA1_SKELKEY1;
    key.carried_by = Some(character_id);
    world.add_item(key);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(character_id);

    assert_eq!(
        apply_keyring_add_cursor_item(&mut world, Some(&mut player), character_id, key_item_id,),
        KeyringAddApplyResult::Added {
            key_name: "Copper Key".to_string(),
        }
    );

    assert_eq!(
        player.keyring_key_name(IID_AREA1_SKELKEY1),
        Some("Copper Key")
    );
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.cursor_item, None);
    assert!(character.flags.contains(CharacterFlags::ITEMS));
    let key = world.items.get(&key_item_id).unwrap();
    assert_eq!(key.carried_by, None);
    assert!(!key.flags.contains(ItemFlags::USED));
}

#[test]
fn apply_keyring_add_cursor_item_rejects_unregistered_key_like_item() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let key_item_id = ItemId(44);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(key_item_id);
    world.add_character(character);
    let mut key = test_item(key_item_id, 1200, ItemFlags::USED | ItemFlags::TAKE);
    key.name = "Decorative Key".to_string();
    key.template_id = 0x1122_3344;
    key.carried_by = Some(character_id);
    world.add_item(key);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(character_id);

    assert_eq!(
        apply_keyring_add_cursor_item(&mut world, Some(&mut player), character_id, key_item_id,),
        KeyringAddApplyResult::NotAKey
    );
    assert!(player.keyring.is_empty());
    assert_eq!(
        world.characters.get(&character_id).unwrap().cursor_item,
        Some(key_item_id)
    );
    assert!(world
        .items
        .get(&key_item_id)
        .unwrap()
        .flags
        .contains(ItemFlags::USED));
}

#[test]
fn apply_keyring_add_cursor_item_reports_duplicate_without_consuming() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let key_item_id = ItemId(44);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(key_item_id);
    world.add_character(character);
    let mut key = test_item(key_item_id, 1200, ItemFlags::USED | ItemFlags::TAKE);
    key.name = "Copper Key".to_string();
    key.template_id = IID_AREA1_SKELKEY1;
    key.carried_by = Some(character_id);
    world.add_item(key);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(character_id);
    assert_eq!(
        player.add_keyring_key(IID_AREA1_SKELKEY1, "Copper Key"),
        KeyringAddResult::Added
    );

    assert_eq!(
        apply_keyring_add_cursor_item(&mut world, Some(&mut player), character_id, key_item_id,),
        KeyringAddApplyResult::Duplicate
    );
    assert_eq!(
        world.characters.get(&character_id).unwrap().cursor_item,
        Some(key_item_id)
    );
    assert!(world
        .items
        .get(&key_item_id)
        .unwrap()
        .flags
        .contains(ItemFlags::USED));
}

#[test]
fn apply_keyring_auto_add_pickup_stores_registered_key_and_consumes_cursor() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let key_item_id = ItemId(44);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(key_item_id);
    world.add_character(character);
    let mut key = test_item(key_item_id, 1200, ItemFlags::USED | ItemFlags::TAKE);
    key.name = "Copper Key".to_string();
    key.template_id = IID_AREA1_SKELKEY1;
    key.carried_by = Some(character_id);
    world.add_item(key);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(character_id);
    player.set_keyring_auto_add(true);

    assert_eq!(
        apply_keyring_auto_add_pickup(&mut world, Some(&mut player), character_id, key_item_id,),
        Some(KeyringAutoAddPickupResult::Added {
            key_name: "Copper Key".to_string(),
        })
    );

    assert_eq!(
        player.keyring_key_name(IID_AREA1_SKELKEY1),
        Some("Copper Key")
    );
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.cursor_item, None);
    assert!(character.flags.contains(CharacterFlags::ITEMS));
    let key = world.items.get(&key_item_id).unwrap();
    assert_eq!(key.carried_by, None);
    assert!(!key.flags.contains(ItemFlags::USED));
}

#[test]
fn apply_keyring_auto_add_pickup_leaves_duplicate_key_on_cursor() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let key_item_id = ItemId(44);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(key_item_id);
    world.add_character(character);
    let mut key = test_item(key_item_id, 1200, ItemFlags::USED | ItemFlags::TAKE);
    key.name = "Copper Key".to_string();
    key.template_id = IID_AREA1_SKELKEY1;
    key.carried_by = Some(character_id);
    world.add_item(key);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(character_id);
    player.set_keyring_auto_add(true);
    assert_eq!(
        player.add_keyring_key(IID_AREA1_SKELKEY1, "Copper Key"),
        KeyringAddResult::Added
    );

    assert_eq!(
        apply_keyring_auto_add_pickup(&mut world, Some(&mut player), character_id, key_item_id,),
        Some(KeyringAutoAddPickupResult::Duplicate {
            key_name: "Copper Key".to_string(),
        })
    );

    assert_eq!(
        world.characters.get(&character_id).unwrap().cursor_item,
        Some(key_item_id)
    );
    assert!(world
        .items
        .get(&key_item_id)
        .unwrap()
        .flags
        .contains(ItemFlags::USED));
}

#[test]
fn apply_chest_treasure_accepts_keyring_key_for_keyed_chest() {
    let mut loader = chest_loader();
    let mut world = World::default();
    world.add_character(login_character(
        CharacterId(7),
        &login_block("Tester"),
        1,
        10,
        10,
    ));
    let mut chest = test_item(ItemId(10), 700, ItemFlags::USE);
    chest.driver_data = vec![9, 0x44, 0x33, 0x22, 0x11, 0, 0];
    world.add_item(chest);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(7));
    assert_eq!(
        player.add_keyring_key(0x1122_3344, "Keyring Key"),
        ugaris_core::player::KeyringAddResult::Added
    );

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
            key_name: Some("Keyring Key".to_string()),
        }
    );
}

#[test]
fn item_driver_context_supplies_keyring_key_for_keyed_door() {
    let mut world = World::default();
    world.add_character(login_character(
        CharacterId(7),
        &login_block("Tester"),
        1,
        10,
        10,
    ));
    let mut door = test_item(ItemId(10), 700, ItemFlags::USE);
    door.driver = ugaris_core::item_driver::IDR_DOOR;
    door.driver_data = vec![0, 0x44, 0x33, 0x22, 0x11];
    world.add_item(door);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(7));
    assert_eq!(
        player.add_keyring_key(0x1122_3344, "Keyring Key"),
        ugaris_core::player::KeyringAddResult::Added
    );

    let request = ugaris_core::item_driver::ItemDriverRequest::Driver {
        driver: ugaris_core::item_driver::IDR_DOOR,
        item_id: ItemId(10),
        character_id: CharacterId(7),
        spec: 0,
    };

    assert_eq!(
        item_driver_context_for_request(&world, Some(&player), &request).door_key,
        Some(ugaris_core::item_driver::DoorKeyAccess {
            key_id: 0x1122_3344,
            name: "Keyring Key".to_string(),
            source: ugaris_core::item_driver::DoorKeySource::Keyring,
        })
    );
}
