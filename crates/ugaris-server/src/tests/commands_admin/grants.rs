use super::*;

#[test]
pub(crate) fn gold_command_moves_character_gold_to_cursor_money_item() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.gold = 12_500;
    world.add_character(character);

    let result = apply_gold_command(&mut world, &mut loader, character_id, "/gold 12")
        .expect("gold command should be recognized");

    assert!(result.messages.is_empty());
    assert!(result.inventory_changed);
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.gold, 11_300);
    let money_id = character.cursor_item.expect("money should be on cursor");
    let money = world.items.get(&money_id).unwrap();
    assert!(money.flags.contains(ItemFlags::MONEY));
    assert_eq!(money.value, 1_200);
    assert_eq!(money.carried_by, Some(character_id));
}

#[test]
pub(crate) fn gold_command_preserves_c_guard_order_and_atoi_prefix() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.gold = 500;
    world.add_character(character);

    let invalid = apply_gold_command(&mut world, &mut loader, character_id, "/gold abc")
        .expect("gold command should be recognized");
    assert_eq!(invalid.messages, vec!["Hu?"]);

    let too_much = apply_gold_command(&mut world, &mut loader, character_id, "/gold 6")
        .expect("gold command should be recognized");
    assert_eq!(too_much.messages, vec!["You do not have that much gold."]);

    world.characters.get_mut(&character_id).unwrap().gold = 1_000;
    let cursor_item = test_item(ItemId(99), 100, ItemFlags::TAKE);
    world.add_item(cursor_item);
    world.characters.get_mut(&character_id).unwrap().cursor_item = Some(ItemId(99));
    let occupied = apply_gold_command(&mut world, &mut loader, character_id, "/gold 6abc")
        .expect("gold command should be recognized");
    assert_eq!(
        occupied.messages,
        vec!["Please free your hand (mouse cursor) first."]
    );
}

#[test]
pub(crate) fn create_command_instantiates_template_on_god_cursor() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"test_sword: name="Test Sword" description="Created" ID=01001234 sprite=4321 flag=IF_TAKE ;"#,
        )
        .unwrap();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("God"), 1, 10, 10);
    character
        .flags
        .insert(CharacterFlags::GOD | CharacterFlags::PLAYER);
    world.add_character(character);

    let result = apply_create_command(&mut world, &mut loader, character_id, "/cre test_sword")
        .expect("legacy create prefix should be recognized");

    assert!(result.messages.is_empty());
    assert!(result.inventory_changed);
    let character = world.characters.get(&character_id).unwrap();
    assert!(character.flags.contains(CharacterFlags::ITEMS));
    let item_id = character
        .cursor_item
        .expect("created item should be on cursor");
    let item = world.items.get(&item_id).unwrap();
    assert_eq!(item.name, "Test Sword");
    assert_eq!(item.description, "Created");
    assert_eq!(item.template_id, 0x0100_1234);
    assert_eq!(item.sprite, 4321);
    assert_eq!(item.carried_by, Some(character_id));
}

#[test]
pub(crate) fn create_command_is_god_only_and_preserves_legacy_feedback() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(r#"test_item: name="Test Item" flag=IF_TAKE ;"#)
        .unwrap();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::PLAYER);
    world.add_character(character);

    assert!(
        apply_create_command(&mut world, &mut loader, character_id, "/create test_item").is_none()
    );

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let missing = apply_create_command(&mut world, &mut loader, character_id, "/create missing")
        .expect("god create should handle missing templates");
    assert_eq!(missing.messages, vec!["No such template exists."]);

    let cursor_id = ItemId(99);
    world.add_item(test_item(cursor_id, 1234, ItemFlags::TAKE));
    world.characters.get_mut(&character_id).unwrap().cursor_item = Some(cursor_id);
    let occupied = apply_create_command(&mut world, &mut loader, character_id, "/create test_item")
        .expect("god create should handle occupied cursor");
    assert_eq!(
        occupied.messages,
        vec!["Please empty your mouse cursor first."]
    );
}

#[test]
pub(crate) fn create_orb_command_supports_random_skill_and_valued_skill() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(r#"empty_orb: name="Empty Orb" flag=IF_TAKE ;"#)
        .unwrap();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("God"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);

    let skill =
        apply_create_orb_command(&mut world, &mut loader, character_id, "/create_orb sword")
            .expect("god create_orb should be recognized");
    assert!(skill.inventory_changed);
    let skill_item_id = world.characters[&character_id].inventory[30].unwrap();
    let skill_item = world.items.get(&skill_item_id).unwrap();
    assert_eq!(skill_item.name, "Orb of Sword");
    assert_eq!(skill_item.driver_data[0], CharacterValue::Sword as u8);
    assert_eq!(skill_item.driver_data[1], 1);

    let valued = apply_create_orb_command(
        &mut world,
        &mut loader,
        character_id,
        "/create_orb 5 immunity",
    )
    .expect("god create_orb valued skill should be recognized");
    assert!(valued.inventory_changed);
    let valued_item_id = world.characters[&character_id].inventory[31].unwrap();
    let valued_item = world.items.get(&valued_item_id).unwrap();
    assert_eq!(valued_item.name, "Orb of 5 Immunity");
    assert_eq!(valued_item.driver_data[0], CharacterValue::Immunity as u8);
    assert_eq!(valued_item.driver_data[1], 5);

    world.tick = ugaris_core::Tick(0);
    let random = apply_create_orb_command(&mut world, &mut loader, character_id, "/create_orb")
        .expect("god create_orb random should be recognized");
    assert!(random.inventory_changed);
    let random_item_id = world.characters[&character_id].inventory[32].unwrap();
    let random_item = world.items.get(&random_item_id).unwrap();
    assert!(random_item.name.starts_with("Orb of "));
    assert_eq!(random_item.driver_data[1], 1);
}

#[test]
pub(crate) fn create_orb_command_is_god_only_and_silent_on_bad_args() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(r#"empty_orb: name="Empty Orb" flag=IF_TAKE ;"#)
        .unwrap();
    let character_id = CharacterId(7);
    world.add_character(login_character(
        character_id,
        &login_block("Tester"),
        1,
        10,
        10,
    ));

    assert!(
        apply_create_orb_command(&mut world, &mut loader, character_id, "/create_orb sword")
            .is_none()
    );

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let bad = apply_create_orb_command(
        &mut world,
        &mut loader,
        character_id,
        "/create_orb nonsense",
    )
    .expect("god create_orb bad args should be handled");
    assert_eq!(bad, KeyringCommandResult::default());
    assert!(world.characters[&character_id].inventory[30].is_none());
}

#[test]
pub(crate) fn ggold_command_is_god_only_and_uses_atoi_prefix() {
    let mut world = World::default();
    let mut loader = ZoneLoader::new();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.gold = 500;
    world.add_character(character);

    assert!(apply_gold_command(&mut world, &mut loader, character_id, "/ggold 12").is_none());
    assert_eq!(world.characters.get(&character_id).unwrap().gold, 500);

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let result = apply_gold_command(&mut world, &mut loader, character_id, "/ggold 12abc")
        .expect("god gold command should be recognized");

    assert!(result.messages.is_empty());
    assert!(result.inventory_changed);
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.gold, 1_700);
    assert!(character.flags.contains(CharacterFlags::ITEMS));
}

pub(crate) fn seyan_m_loader() -> ZoneLoader {
    let mut loader = ZoneLoader::new();
    loader
        .load_character_templates_str(
            r#"
                seyan_m:
                  name="Seyan'Du"
                  description="A Seyan'Du"
                  V_HP=10
                  V_ENDURANCE=8
                  V_MANA=6
                ;
            "#,
        )
        .unwrap();
    loader
}

#[test]
pub(crate) fn god_setseyan_rerolls_target_and_messages_the_target_not_the_caller() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let target_id = CharacterId(8);
    let mut god = login_character(god_id, &login_block("Godmode"), 40, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut target = login_character(target_id, &login_block("Target"), 40, 11, 10);
    target.flags.insert(CharacterFlags::ARCH);
    target.exp = 500_000;
    world.add_character(target);

    let mut runtime = ServerRuntime::default();
    let mut target_player = PlayerRuntime::connected(80, 0);
    target_player.character_id = Some(target_id);
    target_player.demonshrines.push(77);
    runtime.players.insert(80, target_player);

    let loader = seyan_m_loader();

    let result = apply_setseyan_command(
        &mut world,
        &loader,
        &mut runtime,
        god_id,
        "/setseyan Target",
    )
    .expect("god setseyan should be recognized");

    assert!(result.messages.is_empty());
    assert_eq!(
        result.other_messages,
        vec![(target_id, "You are a Seyan'Du now.".to_string())]
    );
    assert!(!result.inventory_changed);
    assert!(!result.name_changed);

    let target = world.characters.get(&target_id).unwrap();
    assert_eq!(target.level, 1);
    assert_eq!(target.exp, 0);
    assert!(target.flags.contains(CharacterFlags::MAGE));
    assert!(target.flags.contains(CharacterFlags::WARRIOR));

    let player = runtime.player_for_character(target_id).unwrap();
    assert!(player.demonshrines.is_empty());
}

#[test]
pub(crate) fn setseyan_is_god_only_and_reports_missing_target() {
    let mut world = World::default();
    let caller_id = CharacterId(7);
    world.add_character(login_character(
        caller_id,
        &login_block("Tester"),
        1,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();
    let loader = seyan_m_loader();

    assert!(apply_setseyan_command(
        &mut world,
        &loader,
        &mut runtime,
        caller_id,
        "/setseyan Missing",
    )
    .is_none());

    world
        .characters
        .get_mut(&caller_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let missing = apply_setseyan_command(
        &mut world,
        &loader,
        &mut runtime,
        caller_id,
        "/setseyan Missing",
    )
    .expect("god setseyan missing target should be handled");
    assert_eq!(
        missing.messages,
        vec!["Sorry, no one by the name Missing around."]
    );
}

#[test]
pub(crate) fn setseyan_requires_exact_full_word_no_abbreviation() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();
    let loader = seyan_m_loader();

    // C `cmdcmp(ptr, "setseyan", 8)`: `minlen` equals the full command's
    // length, so an abbreviation like `/setsey` must not match at all.
    assert!(
        apply_setseyan_command(&mut world, &loader, &mut runtime, god_id, "/setsey Godmode")
            .is_none()
    );
}

#[test]
pub(crate) fn god_clearmerchantstores_resets_gold_and_clears_every_ware() {
    use ugaris_core::character_driver::CDR_MERCHANT;
    use ugaris_core::world::StoreWare;

    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);

    let merchant_id = CharacterId(20);
    let mut merchant = login_character(merchant_id, &login_block("Dolf"), 1, 11, 10);
    merchant.flags.remove(CharacterFlags::PLAYER);
    merchant.driver = CDR_MERCHANT;
    world.add_character(merchant);
    assert!(world.ensure_merchant_store(merchant_id));
    {
        let store = world.merchant_stores.get_mut(&merchant_id).unwrap();
        store.gold = 42;
        store.wares[0] = Some(StoreWare {
            item: test_item(ItemId(900), 1234, ItemFlags::TAKE),
            count: 3,
            always: true,
        });
    }

    let mut runtime = ServerRuntime::default();
    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/clearmerchantstores 20",
        1,
    )
    .expect("god clearmerchantstores should be recognized");

    assert_eq!(
        result.messages,
        vec!["Merchant Dolf (ID: 20) inventory cleared and gold reset"]
    );
    assert_eq!(result.clear_merchant_store_requested, Some(merchant_id));
    let store = world.merchant_stores.get(&merchant_id).unwrap();
    assert_eq!(store.gold, 10_000);
    assert!(store.wares.iter().all(Option::is_none));
}

#[test]
pub(crate) fn god_itemmod_mutates_cursor_modifier_with_legacy_feedback() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    character.cursor_item = Some(ItemId(99));
    world.add_character(character);
    world.add_item(Item {
        id: ItemId(99),
        name: "Modded Item".to_string(),
        description: String::new(),
        flags: ItemFlags::TAKE,
        sprite: 123,
        value: 0,
        min_level: 0,
        max_level: 0,
        needs_class: 0,
        template_id: 0,
        owner_id: 0,
        modifier_index: [0; 5],
        modifier_value: [0; 5],
        x: 0,
        y: 0,
        carried_by: Some(character_id),
        contained_in: None,
        content_id: 0,
        driver: 0,
        driver_data: vec![0; 40],
        serial: 1,
    });
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/itemmod 2 sword 7",
        1,
    )
    .expect("god itemmod should be recognized");
    assert!(result.inventory_changed);
    assert_eq!(result.messages[0], "Modded Item:");
    assert!(result.messages.iter().any(|line| line == "Sword +7"));
    assert_eq!(
        result.messages.last().unwrap(),
        "Item modified: Sword (skill 15) at pos 2 with value 7"
    );
    let item = world.items.get(&ItemId(99)).unwrap();
    assert_eq!(item.modifier_index[2], CharacterValue::Sword as i16);
    assert_eq!(item.modifier_value[2], 7);

    let numeric = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/itemmod 0 18 21",
        1,
    )
    .expect("numeric itemmod should be recognized");
    assert!(numeric.messages.iter().any(|line| line == "Attack +21"));
    let item = world.items.get(&ItemId(99)).unwrap();
    assert_eq!(item.modifier_index[0], CharacterValue::Attack as i16);
    assert_eq!(item.modifier_value[0], 21);
}

#[test]
pub(crate) fn itemmod_is_god_only_requires_cursor_and_checks_bounds() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    world.add_character(login_character(
        character_id,
        &login_block("Tester"),
        1,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/itemmod 0 sword 1",
        1,
    )
    .is_none());

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let missing = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/itemmod 0 sword 1",
        1,
    )
    .expect("god itemmod should handle missing cursor");
    assert_eq!(missing.messages, vec!["Need citem."]);

    world.characters.get_mut(&character_id).unwrap().cursor_item = Some(ItemId(99));
    assert_eq!(
        apply_admin_character_command(
            &mut world,
            &mut runtime,
            character_id,
            "/itemmod 5 sword 1",
            1,
        )
        .unwrap()
        .messages,
        vec!["Pos out of bounds."]
    );
    assert_eq!(
        apply_admin_character_command(
            &mut world,
            &mut runtime,
            character_id,
            "/itemmod 0 43 1",
            1,
        )
        .unwrap()
        .messages,
        vec!["Nr out of bounds."]
    );
    assert_eq!(
        apply_admin_character_command(
            &mut world,
            &mut runtime,
            character_id,
            "/itemmod 0 sword 22",
            1,
        )
        .unwrap()
        .messages,
        vec!["Val out of bounds."]
    );
}

#[test]
pub(crate) fn god_setskill_mutates_online_target_and_recalculates_exp_used() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let target_id = CharacterId(8);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut target = login_character(target_id, &login_block("Target"), 1, 11, 10);
    target.flags.insert(CharacterFlags::PLAYER);
    target.values[1][CharacterValue::Sword as usize] = 1;
    target.exp_used = legacy_calc_exp_used(&target);
    let old_exp_used = target.exp_used;
    world.add_character(target);
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setskill target sword 3",
        1,
    )
    .expect("god setskill should be recognized");
    assert_eq!(
        result.messages,
        vec!["Skill: Sword (pos 15), Old value: 1, New value: 3, exp used changed by 55."]
    );
    let target = world.characters.get(&target_id).unwrap();
    assert_eq!(target.values[1][CharacterValue::Sword as usize], 3);
    assert_eq!(target.exp_used, old_exp_used + 55);
    assert!(target.flags.contains(CharacterFlags::UPDATE));
    assert!(result.inventory_changed);
}

#[test]
pub(crate) fn setskill_is_god_only_and_checks_target_position_and_value() {
    let mut world = World::default();
    let caller_id = CharacterId(7);
    let target_id = CharacterId(8);
    world.add_character(login_character(
        caller_id,
        &login_block("Tester"),
        1,
        10,
        10,
    ));
    world.add_character(login_character(
        target_id,
        &login_block("Target"),
        1,
        11,
        10,
    ));
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/setskill Target sword 3",
        1,
    )
    .is_none());

    world
        .characters
        .get_mut(&caller_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    assert_eq!(
        apply_admin_character_command(
            &mut world,
            &mut runtime,
            caller_id,
            "/setskill Missing sword 3",
            1,
        )
        .unwrap()
        .messages,
        vec!["Sorry, no one by the name Missing around."]
    );
    assert_eq!(
        apply_admin_character_command(
            &mut world,
            &mut runtime,
            caller_id,
            "/setskill Target 43 3",
            1,
        )
        .unwrap()
        .messages,
        vec!["Position out of bounds."]
    );
    assert_eq!(
        apply_admin_character_command(
            &mut world,
            &mut runtime,
            caller_id,
            "/setskill Target sword 256",
            1,
        )
        .unwrap()
        .messages,
        vec!["Value out of bounds."]
    );
}
