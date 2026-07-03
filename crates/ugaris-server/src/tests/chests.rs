use super::*;

#[test]
fn grant_chest_treasure_instantiates_template_to_cursor() {
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
                treasure_9:
                    name="Coins"
                    sprite=105
                    flag=IF_TAKE
                    flag=IF_MONEY
                    value=2500
                ;
                "#,
        )
        .unwrap();
    let mut world = World::default();
    world.add_character(login_character(
        CharacterId(7),
        &login_block("Tester"),
        1,
        10,
        10,
    ));

    assert_eq!(
        grant_chest_treasure(&mut world, &mut loader, CharacterId(7), 9),
        Some("Coins".to_string())
    );

    let character = world.characters.get(&CharacterId(7)).unwrap();
    let item_id = character.cursor_item.unwrap();
    let item = world.items.get(&item_id).unwrap();
    assert_eq!(item.name, "Coins");
    assert_eq!(item.sprite, 105);
    assert_eq!(item.carried_by, Some(CharacterId(7)));
    assert!(character.flags.contains(CharacterFlags::ITEMS));
    assert_eq!(
        grant_chest_treasure(&mut world, &mut loader, CharacterId(7), 9),
        None
    );
}

#[test]
fn grant_template_item_to_cursor_supports_infinite_chest_runes() {
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
                rune4:
                    name="Rune IV"
                    sprite=444
                    flag=IF_TAKE
                ;
                "#,
        )
        .unwrap();
    let mut world = World::default();
    world.add_character(login_character(
        CharacterId(7),
        &login_block("Tester"),
        1,
        10,
        10,
    ));

    assert_eq!(
        grant_template_item_to_cursor(&mut world, &mut loader, CharacterId(7), "rune4"),
        Some("Rune IV".to_string())
    );

    let character = world.characters.get(&CharacterId(7)).unwrap();
    let item = world.items.get(&character.cursor_item.unwrap()).unwrap();
    assert_eq!(item.name, "Rune IV");
    assert_eq!(item.sprite, 444);
    assert_eq!(item.carried_by, Some(CharacterId(7)));
}

#[test]
fn infinite_chest_context_rejects_skeleton_key() {
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    character.inventory[30] = Some(ItemId(30));
    let mut key = test_item(ItemId(30), 1, ItemFlags::TAKE);
    key.template_id = IID_SKELETON_KEY;
    key.name = "Skeleton Key".to_string();
    let mut chest = test_item(ItemId(70), 1, ItemFlags::USE);
    chest.driver = ugaris_core::item_driver::IDR_INFINITE_CHEST;
    chest.driver_data = vec![1, 0x44, 0x33, 0x22, 0x11];

    let mut world = World::default();
    world.add_character(character);
    world.add_item(key);
    world.add_item(chest);

    let context = item_driver_context_for_request(
        &world,
        None,
        &ugaris_core::item_driver::ItemDriverRequest::Driver {
            driver: ugaris_core::item_driver::IDR_INFINITE_CHEST,
            item_id: ItemId(70),
            character_id: CharacterId(7),
            spec: 0,
        },
    );

    assert_eq!(context.door_key, None);
}

#[test]
fn apply_chest_treasure_tracks_legacy_hour_cooldown() {
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
    chest.driver_data = vec![9, 0, 0, 0, 0, 1, 0];
    world.add_item(chest);
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
            key_name: None,
        }
    );
    world
        .characters
        .get_mut(&CharacterId(7))
        .unwrap()
        .cursor_item = None;

    assert_eq!(
        apply_chest_treasure(
            &mut world,
            &mut loader,
            Some(&mut player),
            ItemId(10),
            CharacterId(7),
            9,
            100 + 3599,
        ),
        ChestTreasureApplyResult::Empty
    );

    assert_eq!(
        apply_chest_treasure(
            &mut world,
            &mut loader,
            Some(&mut player),
            ItemId(10),
            CharacterId(7),
            9,
            100 + 3600,
        ),
        ChestTreasureApplyResult::Granted {
            item_name: "Coins".to_string(),
            key_name: None,
        }
    );
}

#[test]
fn apply_chest_treasure_accepts_skeleton_key_for_keyed_chest() {
    let mut loader = chest_loader();
    let mut world = World::default();
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    character.inventory[30] = Some(ItemId(20));
    world.add_character(character);
    let mut chest = test_item(ItemId(10), 700, ItemFlags::USE);
    chest.driver_data = vec![9, 0x44, 0x33, 0x22, 0x11, 0, 0];
    world.add_item(chest);
    let mut key = test_item(ItemId(20), 701, ItemFlags::TAKE);
    key.name = "Skeleton Key".to_string();
    key.template_id = IID_SKELETON_KEY;
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
            key_name: Some("Skeleton Key".to_string()),
        }
    );
}

#[test]
fn apply_chest_treasure_blocks_keyed_chest_without_key() {
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
        apply_chest_treasure(
            &mut world,
            &mut loader,
            Some(&mut player),
            ItemId(10),
            CharacterId(7),
            9,
            100,
        ),
        ChestTreasureApplyResult::KeyRequired
    );
    assert_eq!(player.chest_last_access_seconds(9), 0);
}

#[test]
fn apply_chest_treasure_respects_death_gate() {
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
    chest.driver_data = vec![9, 0, 0, 0, 0, 0, 0, 2];
    world.add_item(chest);
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
        ChestTreasureApplyResult::Empty
    );
    assert_eq!(player.chest_last_access_seconds(9), 0);

    world.characters.get_mut(&CharacterId(7)).unwrap().deaths = 2;
    assert_eq!(
        apply_chest_treasure(
            &mut world,
            &mut loader,
            Some(&mut player),
            ItemId(10),
            CharacterId(7),
            9,
            101,
        ),
        ChestTreasureApplyResult::Granted {
            item_name: "Coins".to_string(),
            key_name: None,
        }
    );
}

#[test]
fn apply_chest_treasure_records_chest_achievements_only_on_success() {
    let mut loader = chest_loader_with_gold_room();
    let mut world = World::default();
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    character.deaths = 1;
    world.add_character(character);
    let mut gated_chest = test_item(ItemId(10), 700, ItemFlags::USE);
    gated_chest.driver_data = vec![9, 0, 0, 0, 0, 0, 0, 2];
    world.add_item(gated_chest);
    let mut gold_room_chest = test_item(ItemId(11), 701, ItemFlags::USE);
    gold_room_chest.driver_data = vec![63];
    world.add_item(gold_room_chest);
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
        ChestTreasureApplyResult::Empty
    );
    assert_eq!(player.achievements.chests_opened, 0);

    assert_eq!(
        apply_chest_treasure(
            &mut world,
            &mut loader,
            Some(&mut player),
            ItemId(11),
            CharacterId(7),
            63,
            101,
        ),
        ChestTreasureApplyResult::Granted {
            item_name: "Gold".to_string(),
            key_name: None,
        }
    );
    assert_eq!(player.achievements.chests_opened, 1);
    assert!(player.achievements.gold_looter);
}

#[test]
fn apply_random_chest_grants_money_and_enforces_daily_cooldown() {
    let mut loader = ZoneLoader::new();
    let mut world = random_chest_world(10, 0);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(7));
    let seed = seed_for_legacy_random(4, 0);

    let result = apply_random_chest(
        &mut world,
        &mut loader,
        Some(&mut player),
        ItemId(10),
        CharacterId(7),
        1,
        100,
        seed,
    );
    let RandomChestApplyResult::Money { amount } = result else {
        panic!("expected money result, got {result:?}");
    };
    assert_eq!(amount, random_chest_money_amount(10, seed));
    assert_eq!(player.achievements.chests_opened, 1);
    assert_eq!(
        player.random_chest_last_used_seconds(random_chest_location_id(5, 6, 1)),
        Some(100)
    );

    let character = world.characters.get_mut(&CharacterId(7)).unwrap();
    let money_id = character.cursor_item.take().unwrap();
    let money = world.items.get(&money_id).unwrap();
    assert!(money.flags.contains(ItemFlags::MONEY));
    assert_eq!(money.value, amount);

    assert_eq!(
        apply_random_chest(
            &mut world,
            &mut loader,
            Some(&mut player),
            ItemId(10),
            CharacterId(7),
            1,
            100 + RANDCHEST_COOLDOWN_SECONDS - 1,
            seed,
        ),
        RandomChestApplyResult::Empty
    );
    assert_eq!(player.achievements.chests_opened, 1);
}

#[test]
fn apply_random_chest_no_tier_empty_roll_consumes_daily_access() {
    let mut loader = ZoneLoader::new();
    let mut world = random_chest_world(10, 0);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(7));
    let seed = seed_for_legacy_random(4, 1);

    assert_eq!(
        apply_random_chest(
            &mut world,
            &mut loader,
            Some(&mut player),
            ItemId(10),
            CharacterId(7),
            1,
            100,
            seed,
        ),
        RandomChestApplyResult::Empty
    );
    assert_eq!(player.achievements.chests_opened, 0);
    assert_eq!(
        player.random_chest_last_used_seconds(random_chest_location_id(5, 6, 1)),
        Some(100)
    );
}

#[test]
fn apply_forest_chest_grants_money_and_marks_area3_imp_flag() {
    let mut loader = ZoneLoader::new();
    let mut world = World::default();
    world.add_character(login_character(
        CharacterId(7),
        &login_block("Tester"),
        16,
        10,
        10,
    ));
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(7));

    assert_eq!(
        apply_forest_chest(
            &mut world,
            &mut loader,
            Some(&mut player),
            CharacterId(7),
            9_733,
            1,
        ),
        ForestChestApplyResult::FoundMoney { amount: 9_733 }
    );
    assert_eq!(player.area3_imp_flags(), 1);
    let character = world.characters.get_mut(&CharacterId(7)).unwrap();
    let money_id = character.cursor_item.take().unwrap();
    let money = world.items.remove(&money_id).unwrap();
    assert!(money.flags.contains(ItemFlags::MONEY));
    assert_eq!(money.value, 9_733);

    assert_eq!(
        apply_forest_chest(
            &mut world,
            &mut loader,
            Some(&mut player),
            CharacterId(7),
            9_733,
            1,
        ),
        ForestChestApplyResult::Empty
    );
}

#[test]
fn apply_random_chest_can_grant_template_loot_for_tier_rolls() {
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
                healing_potion1:
                    name="Healing Potion"
                    sprite=200
                    flag=IF_TAKE
                ;
                "#,
        )
        .unwrap();
    let mut world = random_chest_world(10, 1);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(7));
    let seed = seed_for_legacy_random(28, 21);

    assert_eq!(
        apply_random_chest(
            &mut world,
            &mut loader,
            Some(&mut player),
            ItemId(10),
            CharacterId(7),
            1,
            100,
            seed,
        ),
        RandomChestApplyResult::Item {
            item_name: "Healing Potion".to_string()
        }
    );
    let item_id = world
        .characters
        .get(&CharacterId(7))
        .unwrap()
        .cursor_item
        .unwrap();
    assert_eq!(world.items.get(&item_id).unwrap().name, "Healing Potion");
    assert_eq!(player.achievements.chests_opened, 1);
}

#[test]
fn apply_chest_treasure_sees_cursor_key_but_keeps_cursor_occupied_rule() {
    let mut loader = chest_loader();
    let mut world = World::default();
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(20));
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
        ChestTreasureApplyResult::CursorOccupied
    );
    assert_eq!(player.chest_last_access_seconds(9), 0);
}

#[test]
fn apply_chest_treasure_reports_cursor_occupied_before_cooldown() {
    let mut loader = chest_loader();
    let mut world = World::default();
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(99));
    world.add_character(character);
    let mut chest = test_item(ItemId(10), 700, ItemFlags::USE);
    chest.driver_data = vec![9, 0, 0, 0, 0, 1, 0];
    world.add_item(chest);
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
        ChestTreasureApplyResult::CursorOccupied
    );
    assert_eq!(player.chest_last_access_seconds(9), 0);
}

#[test]
fn chest_blocked_message_prefers_key_requirement_like_legacy_driver() {
    let mut world = World::default();
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(99));
    world.add_character(character);
    let mut chest = test_item(ItemId(10), 700, ItemFlags::USE);
    chest.driver_data = vec![9, 1, 0, 0, 0];
    world.add_item(chest);

    assert_eq!(
        chest_blocked_message(&world, ItemId(10), CharacterId(7)),
        CHEST_KEY_REQUIRED_MESSAGE
    );
}
