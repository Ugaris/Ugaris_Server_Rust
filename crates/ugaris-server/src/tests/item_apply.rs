use super::*;

#[test]
fn islena_door_context_scans_legacy_room_state() {
    let mut world = World::default();
    let mut islena = login_character(CharacterId(57), &login_block("Islena"), 11, 143, 55);
    islena.driver = CDR_PALACEISLENA;
    islena.flags.remove(CharacterFlags::PLAYER);
    islena.x = 143;
    islena.y = 55;
    islena.values[0][CharacterValue::Hp as usize] = 100;
    islena.values[0][CharacterValue::Mana as usize] = 80;
    islena.hp = 100 * POWERSCALE;
    islena.mana = 79 * POWERSCALE;
    world.characters.insert(islena.id, islena);

    let mut other_player = login_character(CharacterId(8), &login_block("Ralph"), 11, 140, 50);
    other_player.flags.insert(CharacterFlags::PLAYER);
    other_player.x = 140;
    other_player.y = 50;
    world.characters.insert(other_player.id, other_player);

    assert_eq!(islena_door_room_context(&world), (true, true, true));

    let player = world.characters.get_mut(&CharacterId(8)).unwrap();
    player.x = 130;
    player.y = 50;
    let islena = world.characters.get_mut(&CharacterId(57)).unwrap();
    islena.mana = 80 * POWERSCALE;

    assert_eq!(islena_door_room_context(&world), (false, true, false));
}

#[test]
fn random_shrine_braveness_requires_death_shrine_then_grants_exp_gold() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut character = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);
    character.level = 7;

    let result = apply_random_shrine_braveness(&mut player, &mut character, 52, 20);

    assert_eq!(result, RandomShrineBravenessApplyResult::Coward);
    assert!(!player.has_used_random_shrine(52));

    player.mark_random_shrine_used(51);
    let result = apply_random_shrine_braveness(&mut player, &mut character, 52, 20);

    let expected = level_value(12);
    assert_eq!(
        result,
        RandomShrineBravenessApplyResult::Used {
            exp: expected,
            gold: expected / 10,
        }
    );
    assert_eq!(character.exp, expected);
    assert_eq!(character.gold, expected / 10);
    assert!(character
        .flags
        .contains(CharacterFlags::ITEMS | CharacterFlags::UPDATE));
    assert!(player.has_used_random_shrine(52));
}

#[test]
fn timer_outcome_feedback_matches_legacy_torch_messages() {
    let feedback = timer_outcome_feedback(&[
        ugaris_core::item_driver::ItemDriverOutcome::TorchExtinguishedUnderwater {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            schedule_after_ticks: 30 * ugaris_core::tick::TICKS_PER_SECOND,
        },
        ugaris_core::item_driver::ItemDriverOutcome::TorchExpired {
            item_id: ItemId(8),
            character_id: CharacterId(2),
            item_name: ugaris_core::item_driver::outcome_item_name("torch"),
        },
    ]);

    assert_eq!(
        feedback,
        vec![
            (CharacterId(1), TORCH_HISS_MESSAGE.to_string()),
            (CharacterId(2), "Your torch expired.".to_string()),
        ]
    );
}

#[test]
fn special_potion_fun_message_matches_legacy_text() {
    let mut world = World::default();
    let login = login_block("Ralph");
    let mut character = login_character(CharacterId(1), &login, 1, 10, 10);
    character.flags |= CharacterFlags::MALE;
    world.add_character(character);

    assert_eq!(
        special_potion_fun_message(&world, CharacterId(1), 8).as_deref(),
        Some("You see Ralph hit himself on the head with a mug.")
    );
    assert_eq!(
        special_potion_fun_message(&world, CharacterId(1), 9).as_deref(),
        Some("Ralph suddenly starts singing in a slurred tongue... Dogs start howling...")
    );
    assert_eq!(
        special_potion_fun_message(&world, CharacterId(1), 10).as_deref(),
        Some("Ralph's hair suddenly shoots up as if hit by electricity.")
    );
    assert_eq!(
        special_potion_fun_message(&world, CharacterId(1), 11).as_deref(),
        Some("Ralph seems to be enjoying a fine ale.")
    );
    assert_eq!(
        special_potion_fun_message(&world, CharacterId(1), 12).as_deref(),
        Some("Ralph drinks a delicious apple juice.")
    );
    assert_eq!(
        special_potion_fun_message(&world, CharacterId(1), 13).as_deref(),
        Some("Ralph feels refreshed.")
    );
    assert_eq!(
        special_potion_fun_message(&world, CharacterId(1), 14).as_deref(),
        Some("Ralph cracks his strong knuckles.")
    );
    assert_eq!(
        special_potion_fun_message(&world, CharacterId(1), 15).as_deref(),
        Some("Ralph starts frothing at the mouth.")
    );
    assert_eq!(special_potion_fun_message(&world, CharacterId(1), 7), None);
}

#[test]
fn special_potion_fun_message_uses_legacy_gender_pronouns() {
    let mut world = World::default();
    let login = login_block("Maggie");
    let mut female = login_character(CharacterId(1), &login, 1, 10, 10);
    female.flags |= CharacterFlags::FEMALE;
    world.add_character(female);
    let login = login_block("Snowball");
    world.add_character(login_character(CharacterId(2), &login, 1, 10, 11));

    assert_eq!(
        special_potion_fun_message(&world, CharacterId(1), 8).as_deref(),
        Some("You see Maggie hit herself on the head with a mug.")
    );
    assert_eq!(
        special_potion_fun_message(&world, CharacterId(1), 14).as_deref(),
        Some("Maggie cracks her strong knuckles.")
    );
    assert_eq!(
        special_potion_fun_message(&world, CharacterId(2), 8).as_deref(),
        Some("You see Snowball hit itself on the head with a mug.")
    );
    assert_eq!(
        special_potion_fun_message(&world, CharacterId(2), 14).as_deref(),
        Some("Snowball cracks its strong knuckles.")
    );
}

#[test]
fn lollipop_area_message_matches_legacy_text() {
    let mut world = World::default();
    let login = login_block("Ralph");
    world.add_character(login_character(CharacterId(1), &login, 1, 10, 10));

    assert_eq!(
        lollipop_area_message(&world, CharacterId(1)),
        "Ralph licks a lollipop."
    );
    assert_eq!(
        lollipop_area_message(&world, CharacterId(99)),
        "Someone licks a lollipop."
    );
}

#[test]
fn potion_area_message_matches_legacy_text() {
    let mut world = World::default();
    let login = login_block("Ralph");
    world.add_character(login_character(CharacterId(1), &login, 1, 10, 10));

    assert_eq!(
        potion_area_message(&world, CharacterId(1)),
        "Ralph drinks a potion."
    );
    assert_eq!(
        potion_area_message(&world, CharacterId(99)),
        "Someone drinks a potion."
    );
}

#[test]
fn apply_empty_potion_drink_replaces_carried_potion_with_template() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.hp = 20 * POWERSCALE;
    character.mana = 45 * POWERSCALE;
    character.endurance = 49 * POWERSCALE;
    character.inventory[INVENTORY_START_INVENTORY] = Some(ItemId(100));
    let mut world = World::default();
    world.add_character(character);

    let mut potion = test_item(ItemId(100), 1234, ItemFlags::USED | ItemFlags::USE);
    potion.driver = ugaris_core::item_driver::IDR_POTION;
    potion.carried_by = Some(character_id);
    potion.driver_data = vec![2, 10, 10, 10];
    world.add_item(potion);

    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(r#"empty_potion2: name="Empty Potion" sprite=5678 ;"#)
        .unwrap();

    assert!(apply_empty_potion_drink(
        &mut world,
        &mut loader,
        ItemId(100),
        character_id,
        2,
    ));

    assert!(!world.items.contains_key(&ItemId(100)));
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.hp, 30 * POWERSCALE);
    assert_eq!(character.mana, 50 * POWERSCALE);
    assert_eq!(character.endurance, 50 * POWERSCALE);
    let empty_id = character.inventory[INVENTORY_START_INVENTORY].unwrap();
    let empty = world.items.get(&empty_id).unwrap();
    assert_eq!(empty.name, "Empty Potion");
    assert_eq!(empty.sprite, 5678);
    assert_eq!(empty.carried_by, Some(character_id));
}

#[test]
fn christmas_pop_inspection_messages_match_legacy_text() {
    assert_eq!(
        christmas_pop_inspection_messages(),
        [
            "You notice a tiny inscription on the magical lollipop. It reads:",
            "\"Place me under a Christmas tree to receive a special gift from the gods.\"",
            "In shimmering letters below, you see:",
            "\"Each tree grants but one wish per adventurer.\"",
        ]
    );
}

#[test]
fn no_potion_area_feedback_applies_to_all_potion_items() {
    let mut world = World::default();
    world.add_item(test_item_with_driver(ItemId(1), IDR_SPECIAL_POTION));
    world.add_item(test_item_with_driver(ItemId(2), IDR_BEYONDPOTION));
    world.add_item(test_item_with_driver(
        ItemId(3),
        ugaris_core::item_driver::IDR_POTION,
    ));
    world.add_item(test_item_with_driver(ItemId(4), IDR_TORCH));

    assert!(is_no_potion_area_blocked_item(&world, ItemId(1)));
    assert!(is_no_potion_area_blocked_item(&world, ItemId(2)));
    assert!(is_no_potion_area_blocked_item(&world, ItemId(3)));
    assert!(!is_no_potion_area_blocked_item(&world, ItemId(4)));
    assert!(!is_no_potion_area_blocked_item(&world, ItemId(99)));
}

#[test]
fn client_take_gold_deposits_cursor_money_before_taking_requested_amount() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.gold = 500;
    character.cursor_item = Some(ItemId(99));
    world.add_character(character);
    let mut money = test_item(ItemId(99), 100, ItemFlags::MONEY | ItemFlags::TAKE);
    money.value = 700;
    money.carried_by = Some(character_id);
    world.add_item(money);

    assert!(apply_gold_client_action(
        &mut world,
        &mut loader,
        character_id,
        &ClientAction::TakeGold { amount: 600 }
    ));

    assert!(!world.items.contains_key(&ItemId(99)));
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.gold, 600);
    let cursor_id = character
        .cursor_item
        .expect("new money item should be on cursor");
    let cursor = world.items.get(&cursor_id).unwrap();
    assert!(cursor.flags.contains(ItemFlags::MONEY));
    assert_eq!(cursor.value, 600);
}

#[test]
fn client_drop_gold_only_deposits_money_cursor_items() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.gold = 500;
    character.cursor_item = Some(ItemId(99));
    world.add_character(character);
    let cursor_item = test_item(ItemId(99), 100, ItemFlags::TAKE);
    world.add_item(cursor_item);

    assert!(!apply_gold_client_action(
        &mut world,
        &mut loader,
        character_id,
        &ClientAction::DropGold
    ));
    assert_eq!(world.characters.get(&character_id).unwrap().gold, 500);
    assert_eq!(
        world.characters.get(&character_id).unwrap().cursor_item,
        Some(ItemId(99))
    );

    world
        .items
        .get_mut(&ItemId(99))
        .unwrap()
        .flags
        .insert(ItemFlags::MONEY);
    world.items.get_mut(&ItemId(99)).unwrap().value = 250;

    assert!(apply_gold_client_action(
        &mut world,
        &mut loader,
        character_id,
        &ClientAction::DropGold
    ));
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.gold, 750);
    assert_eq!(character.cursor_item, None);
    assert!(!world.items.contains_key(&ItemId(99)));
}

#[test]
fn client_junk_item_destroys_cursor_item_and_clears_cursor() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(99));
    world.add_character(character);
    let mut cursor_item = test_item(ItemId(99), 100, ItemFlags::TAKE);
    cursor_item.carried_by = Some(character_id);
    world.add_item(cursor_item);

    assert!(apply_junk_item_client_action(&mut world, character_id));

    assert!(!world.items.contains_key(&ItemId(99)));
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.cursor_item, None);
    assert!(character.flags.contains(CharacterFlags::ITEMS));
}

#[test]
fn client_junk_item_respects_legacy_nojunk_flag() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(99));
    world.add_character(character);
    let mut cursor_item = test_item(ItemId(99), 100, ItemFlags::TAKE | ItemFlags::NOJUNK);
    cursor_item.carried_by = Some(character_id);
    world.add_item(cursor_item);

    assert!(!apply_junk_item_client_action(&mut world, character_id));

    assert!(world.items.contains_key(&ItemId(99)));
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.cursor_item, Some(ItemId(99)));
}

#[test]
fn client_junk_item_is_noop_without_cursor_item() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);

    assert!(!apply_junk_item_client_action(&mut world, character_id));
}

#[test]
fn enhance_gold_stack_merge_consumes_matching_cursor_stack() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.inventory[30] = Some(ItemId(20));
    character.cursor_item = Some(ItemId(21));
    let mut world = World::default();
    world.add_character(character);
    let mut carried = test_item(ItemId(20), 51053, ItemFlags::USED | ItemFlags::USE);
    carried.name = "Gold".to_string();
    carried.driver = IDR_ENHANCE;
    carried.value = 100;
    carried.carried_by = Some(character_id);
    carried.driver_data = vec![2, 4, 0, 0, 0];
    world.add_item(carried);
    let mut cursor = test_item(ItemId(21), 51053, ItemFlags::USED | ItemFlags::USE);
    cursor.name = "Gold".to_string();
    cursor.driver = IDR_ENHANCE;
    cursor.value = 75;
    cursor.carried_by = Some(character_id);
    cursor.driver_data = vec![2, 3, 0, 0, 0];
    world.add_item(cursor);
    let mut loader = ZoneLoader::new();

    assert_eq!(
        apply_nomad_stack(&mut world, &mut loader, ItemId(20), character_id),
        NomadStackApplyResult::Merged {
            count: 7,
            unit: "unit",
        }
    );
    assert_eq!(
        world.characters.get(&character_id).unwrap().cursor_item,
        None
    );
    assert!(!world.items.contains_key(&ItemId(21)));
    let stack = world.items.get(&ItemId(20)).unwrap();
    assert_eq!(stack.driver_data, vec![2, 7, 0, 0, 0]);
    assert_eq!(stack.value, 175);
    assert_eq!(stack.description, "7 units of Gold.");
}

#[test]
fn enhance_material_requires_gold_for_already_silvered_items() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.inventory[30] = Some(ItemId(20));
    character.cursor_item = Some(ItemId(21));
    let mut world = World::default();
    world.add_character(character);
    let mut silver = test_item(ItemId(20), 51054, ItemFlags::USED | ItemFlags::USE);
    silver.name = "Silver".to_string();
    silver.driver = IDR_ENHANCE;
    silver.carried_by = Some(character_id);
    silver.driver_data = vec![1, 244, 1, 0, 0];
    world.add_item(silver);
    let mut target = test_item(ItemId(21), 59300, ItemFlags::USED | ItemFlags::TAKE);
    target.carried_by = Some(character_id);
    world.add_item(target);
    let mut loader = ZoneLoader::new();

    assert_eq!(
        apply_nomad_stack(&mut world, &mut loader, ItemId(20), character_id),
        NomadStackApplyResult::EnhanceNeedsGold
    );
    assert_eq!(world.items.get(&ItemId(21)).unwrap().sprite, 59300);
    assert_eq!(stack_count(world.items.get(&ItemId(20)).unwrap()), 500);
}

#[test]
fn junkpile_search_grants_money_for_roll_three() {
    let mut loader = ZoneLoader::new();
    let mut world = World::default();
    world.add_character(login_character(
        CharacterId(7),
        &login_block("Tester"),
        1,
        10,
        10,
    ));
    let mut pile = test_item(ItemId(70), 1, ItemFlags::USED | ItemFlags::USE);
    assert!(world.map.set_item_map(&mut pile, 10, 10));
    world.add_item(pile);
    let seed = seed_for_legacy_random(10, 3);
    let expected_amount = legacy_random(seed.wrapping_add(1), 700).saturating_add(7);

    let result =
        apply_junkpile_search(&mut world, &mut loader, ItemId(70), CharacterId(7), 7, seed);

    assert_eq!(
        result,
        JunkpileApplyResult::FoundMoney {
            amount: expected_amount,
        }
    );
    let character = world.characters.get(&CharacterId(7)).unwrap();
    let item = world.items.get(&character.cursor_item.unwrap()).unwrap();
    assert_eq!(item.name, "Money");
    assert_eq!(item.value, expected_amount);
    assert!(!world.items.contains_key(&ItemId(70)));
}

#[test]
fn edemon_door_context_uses_exact_carried_key_only() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.inventory[30] = Some(ItemId(30));
    character.inventory[31] = Some(ItemId(31));

    let mut skeleton = test_item(ItemId(30), 1, ItemFlags::TAKE);
    skeleton.template_id = IID_SKELETON_KEY;
    skeleton.name = "Skeleton Key".to_string();
    let mut exact = test_item(ItemId(31), 1, ItemFlags::TAKE);
    exact.template_id = 0x1122_3344;
    exact.name = "Earth Key".to_string();
    let mut door = test_item(ItemId(70), 1, ItemFlags::USE);
    door.driver = ugaris_core::item_driver::IDR_EDEMONDOOR;
    door.driver_data = vec![0, 0x44, 0x33, 0x22, 0x11];

    let mut world = World::default();
    world.add_character(character);
    world.add_item(skeleton);
    world.add_item(exact);
    world.add_item(door);
    let mut player = PlayerRuntime::connected(5, 0);
    player.add_keyring_key(0x1122_3344, "Keyring Earth Key");

    let request = ugaris_core::item_driver::ItemDriverRequest::Driver {
        driver: ugaris_core::item_driver::IDR_EDEMONDOOR,
        item_id: ItemId(70),
        character_id,
        spec: 0,
    };
    let context = item_driver_context_for_request(&world, Some(&player), &request);

    assert_eq!(context.door_key.unwrap().name, "Earth Key");

    world.characters.get_mut(&character_id).unwrap().inventory[31] = None;
    let context = item_driver_context_for_request(&world, Some(&player), &request);
    assert_eq!(context.door_key, None);
}
