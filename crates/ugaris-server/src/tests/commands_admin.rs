use super::*;

#[test]
fn random_shrine_edge_blocks_without_marking_for_no_saves_or_noexp() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut no_saves = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);

    let result = apply_random_shrine_edge(&mut player, &mut no_saves, 31, 20);

    assert_eq!(result, RandomShrineEdgeApplyResult::AlreadyOnEdge);
    assert!(!player.has_used_random_shrine(31));

    let mut noexp = login_character(CharacterId(8), &login_block("Lisa"), 14, 10, 10);
    noexp.saves = 1;
    noexp.flags.insert(CharacterFlags::NOEXP);

    let result = apply_random_shrine_edge(&mut player, &mut noexp, 32, 20);

    assert_eq!(result, RandomShrineEdgeApplyResult::NoExp);
    assert_eq!(noexp.saves, 1);
    assert!(!player.has_used_random_shrine(32));
}

#[test]
fn random_shrine_vitality_blocks_noexp_and_capped_without_marking() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut noexp = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);
    noexp
        .flags
        .insert(CharacterFlags::WARRIOR | CharacterFlags::NOEXP);

    let result = apply_random_shrine_vitality(&mut player, &mut noexp, 50);

    assert_eq!(result, RandomShrineVitalityApplyResult::NoExp);
    assert!(!player.has_used_random_shrine(50));

    let mut capped = login_character(CharacterId(8), &login_block("Lisa"), 14, 10, 10);
    capped.values[1][CharacterValue::Mana as usize] = 115;

    let result = apply_random_shrine_vitality(&mut player, &mut capped, 50);

    assert_eq!(result, RandomShrineVitalityApplyResult::Capped);
    assert!(!player.has_used_random_shrine(50));
}

#[test]
fn gold_command_moves_character_gold_to_cursor_money_item() {
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
fn gold_command_preserves_c_guard_order_and_atoi_prefix() {
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
fn create_command_instantiates_template_on_god_cursor() {
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
fn create_command_is_god_only_and_preserves_legacy_feedback() {
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
fn create_orb_command_supports_random_skill_and_valued_skill() {
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
fn create_orb_command_is_god_only_and_silent_on_bad_args() {
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
fn ggold_command_is_god_only_and_uses_atoi_prefix() {
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

#[test]
fn laugh_command_is_god_only_and_queues_legacy_sound() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character.clone());

    assert!(apply_laugh_command(&mut world, character_id, "/laugh").is_none());
    assert!(world.drain_pending_sound_specials().is_empty());

    character.flags.insert(CharacterFlags::GOD);
    world.characters.insert(character_id, character);
    let result = apply_laugh_command(&mut world, character_id, "/laugh")
        .expect("god laugh command should be recognized");

    assert!(result.messages.is_empty());
    let sounds = world.drain_pending_sound_specials();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].character_id, character_id);
    assert_eq!(sounds[0].special.special_type, 13);
}

#[test]
fn status_command_shows_represented_lostcon_and_account_state() {
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    character
        .flags
        .insert(CharacterFlags::PAID | CharacterFlags::NOBLESS);
    character.values[1][CharacterValue::Bless as usize] = 10;
    character.values[1][CharacterValue::Pulse as usize] = 8;
    character.values[1][CharacterValue::Fireball as usize] = 5;
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_max_lag_seconds(12);
    player.autoturn_enabled = true;

    let result = apply_status_command(&character, &player, "/status")
        .expect("status command should be recognized");

    assert_eq!(result.messages[0], "Lag Control Settings:");
    assert!(result
        .messages
        .contains(&"Max. Lag [/MAXLAG]: 12 sec.".to_string()));
    assert!(result
        .messages
        .contains(&"Don't use Bless [/NOBLESS]: Off.".to_string()));
    assert!(result
        .messages
        .contains(&"Don't use Fireball [/NOFIREBALL]: Off.".to_string()));
    assert!(result
        .messages
        .contains(&"Automatic Pulse [/AUTOPULSE]: Off.".to_string()));
    assert!(result
        .messages
        .contains(&"Automatic Turning [/AUTOTURN]: On.".to_string()));
    assert!(result
        .messages
        .contains(&"Allow others to bless me [/ALLOWBLESS]: No.".to_string()));
    assert!(result.messages.contains(&"Account Status:".to_string()));
    assert!(result.messages.contains(&"Paid Account".to_string()));
}

#[test]
fn status_command_preserves_cmdcmp_prefix_shape() {
    let character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    let player = PlayerRuntime::connected(1, 0);

    assert!(apply_status_command(&character, &player, "/s").is_some());
    assert!(apply_status_command(&character, &player, "/stat").is_some());
    assert!(apply_status_command(&character, &player, "/statusx").is_none());
}

#[test]
fn saves_command_is_god_only_and_uses_legacy_prefix_parsing() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.saves = 3;
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/saves 12", 1)
            .is_none()
    );
    assert_eq!(world.characters.get(&character_id).unwrap().saves, 3);

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/save 12abc", 1)
            .expect("god saves command should be recognized by minlen 4 abbreviation");

    assert!(result.messages.is_empty());
    assert!(!result.inventory_changed);
    assert_eq!(world.characters.get(&character_id).unwrap().saves, 12);

    apply_admin_character_command(&mut world, &mut runtime, character_id, "/saves nope", 1)
        .expect("god saves command should be recognized");
    assert_eq!(world.characters.get(&character_id).unwrap().saves, 0);
}

#[test]
fn god_visibility_toggle_commands_preserve_legacy_feedback() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let xray = apply_admin_character_command(&mut world, &mut runtime, character_id, "/xray", 1)
        .expect("god xray command should be recognized");
    assert_eq!(xray.messages, vec!["Turned x-ray mode on."]);
    assert!(xray.inventory_changed);
    assert!(world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::XRAY));

    let spy = apply_admin_character_command(&mut world, &mut runtime, character_id, "/spy", 1)
        .expect("god spy command should be recognized");
    assert_eq!(
        spy.messages,
        vec!["Turned spy mode on. You will now see all tells, clan, alliance, club, area, and mirror chat."]
    );
    assert!(world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::SPY));

    let spy_off = apply_admin_character_command(&mut world, &mut runtime, character_id, "/spy", 1)
        .expect("god spy command should toggle off");
    assert_eq!(
        spy_off.messages,
        vec!["Turned spy mode off. You will no longer see all tells, clan, alliance, club, area, and mirror chat."]
    );
}

#[test]
fn god_dlight_and_showattack_commands_mutate_runtime_without_feedback() {
    let mut world = World::default();
    world.date = GameDate::calculate(START_TIME + HOUR_LEN * 12, 23, None);
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let dlight =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/dlight 123abc", 1)
            .expect("god dlight command should be recognized");
    assert!(dlight.messages.is_empty());
    assert_eq!(runtime.dlight_override, 123);
    assert_eq!(world.date.daylight, 123);

    apply_admin_character_command(&mut world, &mut runtime, character_id, "/dlight 0", 23)
        .expect("god dlight zero command should clear the override");
    assert_eq!(runtime.dlight_override, 0);
    assert_eq!(
        world.date.daylight,
        GameDate::calculate(START_TIME + HOUR_LEN * 12, 23, None).daylight
    );

    let showattack =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/showat", 1)
            .expect("god showattack command should allow C minlen 6 abbreviation");
    assert!(showattack.messages.is_empty());
    assert!(runtime.show_attack);
    assert!(world.show_attack_debug);

    apply_admin_character_command(&mut world, &mut runtime, character_id, "/showattack", 1)
        .expect("god showattack command should toggle back off");
    assert!(!runtime.show_attack);
    assert!(!world.show_attack_debug);
}

#[test]
fn god_joinclan_and_joinclub_commands_mutate_identity_without_feedback() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let joined_clan =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/joinclan 3abc", 1)
            .expect("god joinclan command should be recognized");
    assert!(joined_clan.messages.is_empty());
    assert!(joined_clan.name_changed);
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.clan, 3);
    assert_eq!(character.clan_rank, 4);
    assert_eq!(character.clan_serial, 0);

    let joined_club =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/joinclub 5", 1)
            .expect("god joinclub command should be recognized");
    assert!(joined_club.messages.is_empty());
    assert!(joined_club.name_changed);
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.clan, 1029);
    assert_eq!(character.clan_rank, 2);
    assert_eq!(character.clan_serial, 0);

    apply_admin_character_command(&mut world, &mut runtime, character_id, "/joinclan 32", 1)
        .expect("out-of-range joinclan is still handled like C");
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.clan, 1029);
    assert_eq!(character.clan_rank, 2);
}

#[test]
fn joinclan_and_joinclub_require_exact_god_commands() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/joinclan 1",
        1,
    )
    .is_none());

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/joincla 1", 1,)
            .is_none()
    );
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/joinclu 1", 1,)
            .is_none()
    );
}

#[test]
fn god_killclan_command_deletes_an_existing_clan_immediately() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let nr = world.clan_registry.found_clan("Doomed", 0).unwrap();
    assert!(world.clan_registry.exists(nr));

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/killclan {nr}"),
        1,
    )
    .expect("god killclan command should be recognized");
    assert!(result.messages.is_empty(), "C emits no feedback either");
    assert!(!world.clan_registry.exists(nr));
}

#[test]
fn killclan_requires_god_and_ignores_out_of_range_numbers() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/killclan 1",
        1
    )
    .is_none());

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let nr = world.clan_registry.found_clan("Safe", 0).unwrap();
    apply_admin_character_command(&mut world, &mut runtime, character_id, "/killclan 0", 1)
        .expect("still recognized, just a no-op for out-of-range numbers");
    assert!(world.clan_registry.exists(nr));
}

#[test]
fn staff_renclan_command_renames_an_existing_clan_in_aston() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::STAFF);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let nr = world.clan_registry.found_clan("Old Name", 0).unwrap();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/renclan {nr} New Name"),
        3,
    )
    .expect("staff renclan command should be recognized");
    assert_eq!(
        result.messages,
        vec![format!("Clan {nr} name changed to \"New Name\".")]
    );
    assert_eq!(world.clan_registry.name(nr), Some("New Name"));
}

#[test]
fn renclan_is_rejected_outside_aston_and_for_unknown_clans() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let nr = world.clan_registry.found_clan("Old Name", 0).unwrap();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/renclan {nr} New Name"),
        1,
    )
    .expect("renclan should still be recognized outside Aston");
    assert_eq!(
        result.messages,
        vec!["Sorry, this command only works in Aston."]
    );
    assert_eq!(world.clan_registry.name(nr), Some("Old Name"));

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/renclan 9 Ghost Clan",
        3,
    )
    .expect("renclan should still be recognized for unknown clans");
    assert_eq!(result.messages, vec!["No clan by that number (9)."]);
}

#[test]
fn renclan_requires_staff_or_god() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/renclan 1 Name",
        3,
    )
    .is_none());
}

#[test]
fn god_setxmas_command_sets_runtime_christmas_override() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();
    runtime.xmas_special_override = Some(0);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/setxmas 1abc", 1)
            .expect("god setxmas command should be recognized");
    assert_eq!(
        result.messages,
        vec!["Setting christmas special to 1, old value was 0."]
    );
    assert_eq!(runtime.xmas_special_override, Some(1));
    assert_eq!(runtime_effective_xmas_event(&runtime).0, true);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/setxmas 0", 1)
            .expect("god setxmas command should accept zero");
    assert_eq!(
        result.messages,
        vec!["Setting christmas special to 0, old value was 1."]
    );
    assert_eq!(runtime.xmas_special_override, Some(0));
    assert_eq!(runtime_effective_xmas_event(&runtime).0, false);
}

#[test]
fn god_prof_command_reports_empty_profile_boundary_like_c() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/prof", 1)
        .expect("legacy cmdcmp accepts prof prefix length four");

    assert_eq!(result.messages, vec!["--- Profile ---", "---------------"]);
    assert!(apply_admin_character_command(&mut world, &mut runtime, god_id, "/pro", 1).is_none());

    let mortal_id = CharacterId(8);
    world.add_character(login_character(
        mortal_id,
        &login_block("Mortal"),
        1,
        11,
        10,
    ));
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, mortal_id, "/prof", 1).is_none()
    );
}

#[test]
fn god_staffcode_command_sets_runtime_code_with_legacy_parsing() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let staff_id = CharacterId(8);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut staff = login_character(staff_id, &login_block("Staffer"), 1, 11, 10);
    staff.flags.insert(CharacterFlags::STAFF);
    world.add_character(staff);
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/staffc Staffer xy", 1)
            .expect("legacy cmdcmp accepts staffcode prefix length six");

    assert_eq!(result.messages, vec!["Set Staffer's staff code to XY."]);
    assert_eq!(runtime_staff_code(&runtime, staff_id), "XY");
    assert_eq!(world.characters.get(&staff_id).unwrap().staff_code, "XY");

    let defaulted =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/staffcode Staffer 7", 1)
            .expect("god staffcode command should be recognized");
    assert_eq!(defaulted.messages, vec!["Set Staffer's staff code to AA."]);
    assert_eq!(runtime_staff_code(&runtime, staff_id), "AA");
    assert_eq!(world.characters.get(&staff_id).unwrap().staff_code, "AA");

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        staff_id,
        "/staffcode Staffer zz",
        1,
    )
    .is_none());
}

#[test]
fn god_reset_command_clamps_target_values_like_c() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let target_id = CharacterId(8);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut target = login_character(target_id, &login_block("Target"), 1, 11, 10);
    target.values[1][CharacterValue::Hp as usize] = 50;
    target.values[1][CharacterValue::Strength as usize] = 18;
    target.values[1][CharacterValue::Armor as usize] = 9;
    target.values[1][CharacterValue::Immunity as usize] = 4;
    target.values[1][CharacterValue::Demon as usize] = 7;
    target.values[1][CharacterValue::Duration as usize] = 6;
    target.values[1][CharacterValue::Rage as usize] = 5;
    target.exp_used = 12345;
    world.add_character(target);
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/reset Target ignored", 1)
            .expect("god reset should be recognized");

    assert!(result.messages.is_empty());
    let target = world.characters.get(&target_id).unwrap();
    assert_eq!(target.values[1][CharacterValue::Hp as usize], 10);
    assert_eq!(target.values[1][CharacterValue::Strength as usize], 10);
    assert_eq!(target.values[1][CharacterValue::Armor as usize], 1);
    assert_eq!(target.values[1][CharacterValue::Immunity as usize], 1);
    assert_eq!(target.values[1][CharacterValue::Demon as usize], 7);
    assert_eq!(target.values[1][CharacterValue::Duration as usize], 1);
    assert_eq!(target.values[1][CharacterValue::Rage as usize], 1);
    assert_eq!(target.exp_used, 0);
    assert!(target.flags.contains(CharacterFlags::UPDATE));
}

#[test]
fn reset_command_is_god_only_and_reports_missing_target_like_c() {
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

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/reset Missing",
        1,
    )
    .is_none());

    world
        .characters
        .get_mut(&caller_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let missing =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/reset Missing", 1)
            .expect("god reset missing target should be handled");
    assert_eq!(
        missing.messages,
        vec!["Sorry, no one by the name Missing around."]
    );
}

#[test]
fn god_resetgift_clears_xmas_tree_area_bit_with_legacy_feedback() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let target_id = CharacterId(8);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    world.add_character(login_character(
        target_id,
        &login_block("Target"),
        1,
        11,
        10,
    ));
    let mut runtime = ServerRuntime::default();
    let mut target_player = PlayerRuntime::connected(80, 0);
    target_player.character_id = Some(target_id);
    assert_eq!(
        target_player.touch_xmas_tree(29, 2026, true, true),
        XmasTreeResult::GiftGranted
    );
    runtime.players.insert(80, target_player);

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/resetgift Target 29abc",
        1,
    )
    .expect("god resetgift should be recognized");

    assert_eq!(
        result.messages,
        vec!["Reset gift flag for Target in area 29 (was set)."]
    );
    assert!(!runtime
        .player_for_character(target_id)
        .unwrap()
        .xmas_tree_marked(29));

    let repeat =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/resetgift Target 29", 1)
            .expect("god resetgift repeat should be recognized");
    assert_eq!(
        repeat.messages,
        vec!["Reset gift flag for Target in area 29 (was not set)."]
    );
}

#[test]
fn resetgift_is_god_only_checks_target_and_area() {
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

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/resetgift Missing 1",
        1,
    )
    .is_none());

    world
        .characters
        .get_mut(&caller_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let missing = apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/resetgift Missing 1",
        1,
    )
    .expect("god resetgift missing target should be handled");
    assert_eq!(
        missing.messages,
        vec!["Sorry, no one by the name Missing around."]
    );

    let target_id = CharacterId(8);
    world.add_character(login_character(
        target_id,
        &login_block("Target"),
        1,
        11,
        10,
    ));
    let invalid_area = apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/resetgift Target 64",
        1,
    )
    .expect("god resetgift invalid area should be handled");
    assert_eq!(
        invalid_area.messages,
        vec!["Invalid area ID. Must be between 0 and 63."]
    );

    let no_runtime = apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/resetgift Target 1",
        1,
    )
    .expect("god resetgift missing player data should be handled");
    assert_eq!(no_runtime.messages, vec!["Could not retrieve player data."]);
}

#[test]
fn god_questlog_lists_flagged_quests_like_c() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let target_id = CharacterId(8);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    world.add_character(login_character(
        target_id,
        &login_block("Target"),
        1,
        11,
        10,
    ));
    let mut runtime = ServerRuntime::default();
    let mut target_player = PlayerRuntime::connected(80, 0);
    target_player.character_id = Some(target_id);
    target_player.quest_log.open(3);
    target_player.quest_log.mark_done(4);
    runtime.players.insert(80, target_player);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/quest Target", 1)
            .expect("legacy cmdcmp accepts questlog prefix length five");

    assert_eq!(
        result.messages,
        vec![
            "Quest log for Target:",
            "Quest #3: Open, Done level: 0",
            "Quest #4: Closed, Done level: 1",
        ]
    );
}

#[test]
fn questlog_is_god_only_and_reports_missing_data_like_c() {
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
        "/questlog Target",
        1,
    )
    .is_none());

    world
        .characters
        .get_mut(&caller_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let no_runtime =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/questlog Target", 1)
            .expect("god questlog should be handled");
    assert_eq!(
        no_runtime.messages,
        vec!["Failed to get quest data for Target"]
    );

    let missing =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/questlog Missing", 1)
            .expect("god questlog missing target should be handled");
    assert_eq!(missing.messages, vec!["Character Missing not found"]);
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/ques Target", 1,)
            .is_none()
    );
}

#[test]
fn dlight_and_showattack_are_god_only_and_keep_full_dlight_minlen() {
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

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/dlight 1", 1,)
            .is_none()
    );
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/showattack",
        1,
    )
    .is_none());
    assert_eq!(runtime.dlight_override, 0);
    assert!(!runtime.show_attack);

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/dligh 7", 1,)
            .is_none()
    );
    assert_eq!(runtime.dlight_override, 0);
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/showa", 1,)
            .is_none()
    );
    assert!(!runtime.show_attack);
}

#[test]
fn god_sprite_command_sets_character_sprite_silently() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Godmode"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    character.sprite = 100;
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/sprite 27abc", 1)
            .expect("god sprite command should be recognized");

    assert_eq!(world.characters[&character_id].sprite, 27);
    assert!(result.messages.is_empty());
    assert!(result.inventory_changed);
    assert!(result.name_changed);
}

#[test]
fn sprite_command_is_god_only_and_requires_full_name() {
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

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/sprite 42", 1,)
            .is_none()
    );

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/sprit 42", 1,)
            .is_none()
    );
}

#[test]
fn god_itemname_and_itemdesc_mutate_cursor_item_with_look_feedback() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    character.cursor_item = Some(ItemId(99));
    world.add_character(character);
    world.add_item(Item {
        id: ItemId(99),
        name: "Old Name".to_string(),
        description: "Old description".to_string(),
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

    let name = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/itemname Renamed Cursor Item",
        1,
    )
    .expect("god itemname should be recognized");
    assert!(name.inventory_changed);
    assert_eq!(name.messages[0], "Renamed Cursor Item:");
    assert_eq!(name.messages[1], "Old description");
    assert_eq!(
        world.items.get(&ItemId(99)).unwrap().name,
        "Renamed Cursor Item"
    );

    let long_desc = format!("{}tail", "x".repeat(79));
    let desc = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/itemdesc {long_desc}"),
        1,
    )
    .expect("god itemdesc should be recognized");
    assert!(desc.inventory_changed);
    assert_eq!(desc.messages[0], "Renamed Cursor Item:");
    assert_eq!(desc.messages[1], "x".repeat(79));
    assert_eq!(world.items.get(&ItemId(99)).unwrap().description.len(), 79);
}

#[test]
fn god_listitem_reports_legacy_item_details() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let carrier_id = CharacterId(8);
    world.add_character(login_character(
        carrier_id,
        &login_block("Carrier"),
        1,
        11,
        10,
    ));
    let mut item = Item {
        id: ItemId(99),
        name: "Listed Item".to_string(),
        description: "Listed description".to_string(),
        flags: ItemFlags::TAKE | ItemFlags::USE,
        sprite: 1234,
        value: 0,
        min_level: 0,
        max_level: 0,
        needs_class: 0,
        template_id: 5678,
        owner_id: 0,
        modifier_index: [0; 5],
        modifier_value: [0; 5],
        x: 0,
        y: 0,
        carried_by: Some(carrier_id),
        contained_in: None,
        content_id: 0,
        driver: 42,
        driver_data: vec![0; 40],
        serial: 1,
    };
    item.modifier_index[1] = CharacterValue::Sword as i16;
    item.modifier_value[1] = 7;
    world.add_item(item);
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/listi 99xyz", 1)
        .expect("legacy cmdcmp accepts listitem prefix length five");

    assert_eq!(result.messages[0], "Item #99: Listed Item");
    assert_eq!(result.messages[1], "Description: Listed description");
    assert_eq!(result.messages[2], "Flags: 0x18");
    assert_eq!(result.messages[3], "Driver: 42, ID: 5678, Sprite: 1234");
    assert_eq!(result.messages[4], "Carried by: Carrier (8)");
    assert_eq!(result.messages[5], "Mod #1: +7 to Sword");

    let item = world.items.get_mut(&ItemId(99)).unwrap();
    item.carried_by = None;
    item.x = 12;
    item.y = 34;
    let positioned =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/listitem 99", 1)
            .expect("god listitem should be recognized");
    assert!(positioned
        .messages
        .iter()
        .any(|line| line == "Position: 12,34"));
}

#[test]
fn listitem_is_god_only_and_reports_invalid_ids() {
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
        "/listitem 99",
        1,
    )
    .is_none());

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let missing =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/listitem 99", 1)
            .expect("god listitem should report invalid IDs");
    assert_eq!(
        missing.messages,
        vec!["Invalid item number or item doesn't exist"]
    );
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/list 99", 1,)
            .is_none()
    );
}

#[test]
fn god_setkarma_mutates_online_target_with_legacy_feedback() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let target_id = CharacterId(8);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    let mut target = login_character(target_id, &login_block("Target"), 1, 11, 10);
    target.karma = -3;
    world.add_character(god);
    world.add_character(target);
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setka Target 42abc", 1)
            .expect("legacy cmdcmp accepts setkarma prefix length five");

    assert_eq!(
        result.messages,
        vec!["Changed Target's karma from -3 to 42"]
    );
    assert_eq!(world.characters.get(&target_id).unwrap().karma, 42);
}

#[test]
fn setkarma_is_god_only_and_reports_missing_target_like_c() {
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
        "/setkarma Missing 12",
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
        "/setkarma Missing 12",
        1,
    )
    .expect("god setkarma missing target should be handled");
    assert_eq!(missing.messages, vec!["Character Missing not found"]);
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setk Missing 12",
        1,
    )
    .is_none());
}

#[test]
fn god_setexpmod_updates_runtime_with_legacy_feedback() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setexpmod 2.5xyz", 1)
            .expect("god setexpmod should be recognized");

    assert_eq!(world.settings.exp_modifier, 2.5);
    assert_eq!(
        result.messages,
        vec!["Global experience modifier changed from 1.00 to 2.50"]
    );

    let invalid =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setexpmod 0.09", 1)
            .expect("god setexpmod should handle invalid values");
    assert_eq!(world.settings.exp_modifier, 2.5);
    assert_eq!(
        invalid.messages,
        vec!["Invalid value. Please specify a number between 0.1 and 1000.0"]
    );
}

#[test]
fn setexpmod_is_god_only_and_full_command_only() {
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
        "/setexpmod 2",
        1,
    )
    .is_none());

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setexpmo 2",
        1,
    )
    .is_none());
}

#[test]
fn god_sethardcore_bonus_commands_match_legacy_ranges_and_feedback() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    let exp = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/sethardcoreexpbonus 2.25tail",
        1,
    )
    .expect("god hardcore exp bonus command should be recognized");
    assert_eq!(world.settings.hardcore_exp_bonus, 2.25);
    assert_eq!(
        exp.messages,
        vec!["Hardcore experience bonus changed from 1.00 to 2.25"]
    );

    let milexp = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/sethardcoremilexpbonus 1.5",
        1,
    )
    .expect("god hardcore military exp bonus command should be recognized");
    assert_eq!(runtime.hardcore_military_exp_bonus, 1.5);
    assert_eq!(
        milexp.messages,
        vec!["Hardcore military experience bonus changed from 1.10 to 1.50"]
    );

    let kill = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/sethardcorekillexpbonus 3.0",
        1,
    )
    .expect("god hardcore kill exp bonus command should be recognized");
    assert_eq!(runtime.hardcore_kill_exp_bonus, 3.0);
    assert_eq!(
        kill.messages,
        vec!["Hardcore kill experience bonus changed from 1.30 to 3.00"]
    );

    let invalid_kill = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/sethardcorekillexpbonus 3.01",
        1,
    )
    .expect("invalid hardcore kill exp bonus should still be handled");
    assert_eq!(runtime.hardcore_kill_exp_bonus, 3.0);
    assert_eq!(
        invalid_kill.messages,
        vec!["Invalid value. Please specify a number between 1.0 and 3.0"]
    );

    let invalid_exp = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/sethardcoreexpbonus 0.09",
        1,
    )
    .expect("invalid hardcore exp bonus should still be handled");
    assert_eq!(world.settings.hardcore_exp_bonus, 2.25);
    assert_eq!(
        invalid_exp.messages,
        vec!["Invalid value. Please specify a number between 0.1 and 1000.0"]
    );
}

#[test]
fn hardcore_bonus_commands_are_god_only_and_full_command_only() {
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
        "/sethardcoreexpbonus 2",
        1,
    )
    .is_none());

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/sethardcoreexpbonu 2",
        1,
    )
    .is_none());
}

#[test]
fn god_tick_tuning_commands_match_legacy_ranges_and_feedback() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();
    let ticks = TICKS_PER_SECOND as i32;

    let decay = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setdecaytime 1440tail",
        1,
    )
    .expect("god setdecaytime should be recognized");
    assert_eq!(runtime.item_decay_time, 1440);
    assert_eq!(
        decay.messages,
        vec!["Item decay time changed from 7200 to 1440 ticks (5 to 1 minutes)"]
    );

    let player_body = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setplayerbodytime 7200",
        1,
    )
    .expect("god setplayerbodytime should be recognized");
    assert_eq!(runtime.player_body_decay_time, 7200);
    assert_eq!(
        player_body.messages,
        vec!["Player body decay time changed from 43200 to 7200 ticks (30 to 5 minutes)"]
    );

    let npc_body =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setnpcbodytime 720", 1)
            .expect("god setnpcbodytime should be recognized");
    assert_eq!(runtime.npc_body_decay_time, 720);
    assert_eq!(
        npc_body.messages,
        vec!["NPC body decay time changed from 2880 to 720 ticks (2 to 0 minutes)"]
    );

    let area32_body = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setnpcbodytimearea32 7200",
        1,
    )
    .expect("god setnpcbodytimearea32 should be recognized");
    assert_eq!(runtime.npc_body_decay_time_area32, 7200);
    assert_eq!(
        area32_body.messages,
        vec!["NPC body decay time for area 32 changed from 21600 to 7200 ticks (15 to 5 minutes)"]
    );

    let respawn =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setrespawntime 720", 1)
            .expect("god setrespawntime should be recognized");
    assert_eq!(runtime.npc_respawn_timer, 720);
    assert_eq!(
        respawn.messages,
        vec!["NPC respawn time changed from 2880 to 720 ticks (2 to 0 minutes)"]
    );

    let sewer_respawn = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setsewerrespawntime 3600tail",
        1,
    )
    .expect("god setsewerrespawntime should be recognized");
    assert_eq!(runtime.sewer_item_respawn_time, 3600);
    assert_eq!(
        sewer_respawn.messages,
        vec!["Sewer item respawn time changed from 86400 to 3600 seconds (24 to 1 hours)"]
    );

    let lagout =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setlagouttime 1440", 1)
            .expect("god setlagouttime should be recognized");
    assert_eq!(runtime.lagout_time, 1440);
    assert_eq!(
        lagout.messages,
        vec!["Lagout time changed from 7200 to 1440 ticks (5 to 1 minutes)"]
    );

    let regen =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setregentime 12", 1)
            .expect("god setregentime should be recognized");
    assert_eq!(runtime.regen_time, 12);
    assert_eq!(
        regen.messages,
        vec!["Regeneration time changed from 96 to 12 ticks"]
    );

    let invalid =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setrespawntime 719", 1)
            .expect("invalid setrespawntime should still be handled");
    assert_eq!(runtime.npc_respawn_timer, 720);
    assert_eq!(
        invalid.messages,
        vec![format!(
            "Invalid value. Please specify a time between {} and {} ticks (0.5-10 minutes)",
            30 * ticks,
            10 * 60 * ticks
        )]
    );

    let invalid_sewer = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setsewerrespawntime 3599",
        1,
    )
    .expect("invalid setsewerrespawntime should still be handled");
    assert_eq!(runtime.sewer_item_respawn_time, 3600);
    assert_eq!(
        invalid_sewer.messages,
        vec!["Invalid value. Please specify a time between 3600 and 604800 seconds (1 hour to 7 days)"]
    );
}

#[test]
fn tick_tuning_commands_are_god_only_and_preserve_minimum_lengths() {
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
        "/setdecaytime 1440",
        1,
    )
    .is_none());
    assert_eq!(
        runtime.item_decay_time,
        GameSettings::default().item_decay_time
    );
    assert_eq!(
        runtime.sewer_item_respawn_time,
        GameSettings::default().sewer_item_respawn_time
    );

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setdecaytim 1440",
        1,
    )
    .is_none());
    assert_eq!(
        runtime.item_decay_time,
        GameSettings::default().item_decay_time
    );

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setnpcbodytim 720",
        1,
    )
    .is_none());
    assert_eq!(
        runtime.npc_body_decay_time,
        GameSettings::default().npc_body_decay_time
    );

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setsewerrespawntim 3600",
        1,
    )
    .is_none());
    assert_eq!(
        runtime.sewer_item_respawn_time,
        GameSettings::default().sewer_item_respawn_time
    );
}

#[test]
fn god_communication_tuning_commands_match_legacy_ranges_and_feedback() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    let holler =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/sethollerdist 75tail", 1)
            .expect("god sethollerdist should be recognized");
    assert_eq!(runtime.holler_dist, 75);
    assert_eq!(
        holler.messages,
        vec!["Holler distance changed from 75 to 75 tiles"]
    );

    let quiet =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setquietsaydist 4", 1)
            .expect("god setquietsaydist should be recognized");
    assert_eq!(runtime.quietsay_dist, 4);
    assert_eq!(
        quiet.messages,
        vec!["Quiet say distance changed from 8 to 4 tiles"]
    );

    let shout_cost =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setshoutcost 2000", 1)
            .expect("god setshoutcost should be recognized");
    assert_eq!(runtime.shout_cost, 2000);
    assert_eq!(
        shout_cost.messages,
        vec!["Shout cost changed from 6 to 2 endurance points"]
    );

    let invalid_whisper =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setwhisperdist 0", 1)
            .expect("invalid setwhisperdist should still be handled");
    assert_eq!(runtime.whisper_dist, GameSettings::default().whisper_dist);
    assert_eq!(
        invalid_whisper.messages,
        vec!["Invalid value. Please specify a distance between 1 and 12 tiles"]
    );
}

#[test]
fn communication_tuning_commands_are_god_only_and_preserve_minimum_lengths() {
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
        "/setshoutdist 50",
        1,
    )
    .is_none());
    assert_eq!(runtime.shout_dist, GameSettings::default().shout_dist);

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setshoutdis 50",
        1,
    )
    .is_none());
    assert_eq!(runtime.shout_dist, GameSettings::default().shout_dist);
}

#[test]
fn god_listchars_reports_active_players_and_npcs_like_c() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    god.level = 12;
    world.add_character(god);

    let mut player = login_character(CharacterId(8), &login_block("Player"), 1, 11, 10);
    player.level = 3;
    world.add_character(player);

    let mut npc = login_character(CharacterId(9), &login_block("Rat"), 1, 12, 10);
    npc.flags.remove(CharacterFlags::PLAYER);
    npc.level = 2;
    npc.driver = 17;
    world.add_character(npc);

    let mut runtime = ServerRuntime::default();
    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/listc", 1)
        .expect("legacy cmdcmp accepts listchars prefix length five");

    assert_eq!(
        result.messages,
        vec![
            "Active characters:",
            "Player:   7 - Godmode (L12)",
            "Player:   8 - Player (L3)",
            "NPC:      9 - Rat (L2, D:17)",
            "Total: 3 characters (2 players, 1 NPCs)",
        ]
    );
}

#[test]
fn listchars_is_god_only_and_rejects_too_short_prefix() {
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

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/listchars", 1,)
            .is_none()
    );

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/list", 1,)
            .is_none()
    );
}

#[test]
fn itemname_and_itemdesc_are_god_only_and_require_cursor_item() {
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
        "/itemname Nope",
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
        "/itemdesc Missing",
        1,
    )
    .expect("god itemdesc should be handled even without cursor item");
    assert_eq!(missing.messages, vec!["Need citem."]);
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/itemnam Nope",
        1,
    )
    .is_none());
}

#[test]
fn god_itemmod_mutates_cursor_modifier_with_legacy_feedback() {
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
fn god_exp_command_reports_and_grants_self_or_named_target() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let target_id = CharacterId(8);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    god.exp = 100;
    let mut target = login_character(target_id, &login_block("Target"), 1, 11, 10);
    target.exp = 200;
    world.add_character(god);
    world.add_character(target);
    let mut runtime = ServerRuntime::default();

    let report = apply_admin_character_command(&mut world, &mut runtime, god_id, "/exp", 1)
        .expect("god exp should be recognized");
    assert_eq!(report.messages, vec!["Godmode has 100 exp."]);

    let self_grant = apply_admin_character_command(&mut world, &mut runtime, god_id, "/exp 25", 1)
        .expect("god exp self grant should be recognized");
    assert_eq!(self_grant.messages, vec!["Gave Godmode 25 exp."]);
    assert!(self_grant.inventory_changed);
    assert_eq!(world.characters.get(&god_id).unwrap().exp, 125);
    assert!(world
        .characters
        .get(&god_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::UPDATE));

    let target_grant =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/exp Target 50", 1)
            .expect("god exp target grant should be recognized");
    assert_eq!(target_grant.messages, vec!["Gave Target 50 exp."]);
    assert_eq!(world.characters.get(&target_id).unwrap().exp, 250);

    let target_report =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/exp Target", 1)
            .expect("god exp target report should be recognized");
    assert_eq!(target_report.messages, vec!["Target has 250 exp."]);
}

#[test]
fn god_exp_command_uses_runtime_exp_modifiers_and_legacy_gates() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let target_id = CharacterId(8);
    let blocked_id = CharacterId(9);
    let capped_id = CharacterId(10);

    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);

    let mut target = login_character(target_id, &login_block("Target"), 1, 11, 10);
    target.exp = 100;
    target.flags.insert(CharacterFlags::HARDCORE);
    world.add_character(target);

    let mut blocked = login_character(blocked_id, &login_block("Blocked"), 1, 12, 10);
    blocked.exp = 100;
    blocked.flags.insert(CharacterFlags::NOEXP);
    world.add_character(blocked);

    let mut capped = login_character(capped_id, &login_block("Capped"), 1, 13, 10);
    capped.level = 10;
    capped.exp = level2exp(10);
    capped.flags.insert(CharacterFlags::NOLEVEL);
    world.add_character(capped);

    let mut runtime = ServerRuntime::default();
    world.settings.exp_modifier = 2.0;
    world.settings.hardcore_exp_bonus = 1.5;

    apply_admin_character_command(&mut world, &mut runtime, god_id, "/exp Target 10", 1)
        .expect("god exp target grant should be recognized");
    let target = world.characters.get(&target_id).unwrap();
    assert_eq!(target.exp, 130);
    // C `give_exp` -> `check_levelup`: 130 exp crosses level2exp(3) == 81,
    // so the target levels up from 1 to 3 in the same call. Hardcore
    // characters reset `saves` to 0 on every level (already 0 here, so this
    // just confirms it stays 0 rather than incrementing).
    assert_eq!(target.level, 3);
    assert_eq!(target.saves, 0);

    apply_admin_character_command(&mut world, &mut runtime, god_id, "/exp Blocked 10", 1)
        .expect("god exp noexp target should be recognized");
    assert_eq!(world.characters.get(&blocked_id).unwrap().exp, 100);

    apply_admin_character_command(&mut world, &mut runtime, god_id, "/exp Capped 100000", 1)
        .expect("god exp nolevel target should be recognized");
    let capped = world.characters.get(&capped_id).unwrap();
    assert_eq!(capped.exp, level2exp(11) - 1);
    // C `give_exp`: `check_levelup` only runs `if (!(ch[cn].flags &
    // CF_NOLEVEL))`, so a NOLEVEL character never levels up even though its
    // capped exp is one shy of level2exp(11).
    assert_eq!(capped.level, 10);
}

#[test]
fn exp_command_is_god_only_uses_legacy_prefix_and_not_found_feedback() {
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

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/exp 10", 1)
            .is_none()
    );

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let missing =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/exp Missing 10", 1)
            .expect("god exp missing target should be handled");
    assert_eq!(
        missing.messages,
        vec!["Sorry, no one by the name Missing around."]
    );
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/ex 10", 1)
            .is_none()
    );
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/expx 10", 1)
            .is_none()
    );
}

#[test]
fn god_milexp_command_reports_and_grants_military_points() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let target_id = CharacterId(8);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    god.exp = 100;
    let mut target = login_character(target_id, &login_block("Target"), 1, 11, 10);
    target.exp = 200;
    target.flags.insert(CharacterFlags::HARDCORE);
    world.add_character(god);
    world.add_character(target);
    let mut runtime = ServerRuntime::default();

    let report = apply_admin_character_command(&mut world, &mut runtime, god_id, "/milexp", 1)
        .expect("god milexp should be recognized");
    assert_eq!(report.messages, vec!["Godmode has 100 exp."]);

    let self_grant =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/milexp 25", 1)
            .expect("god milexp self grant should be recognized");
    assert_eq!(self_grant.messages, vec!["Gave Godmode 25 military exp."]);
    assert!(self_grant.inventory_changed);
    let god = world.characters.get(&god_id).unwrap();
    assert_eq!(god.exp, 101);
    assert_eq!(god.military_normal_exp, 1);
    assert_eq!(god.military_points, 25);
    assert!(god.flags.contains(CharacterFlags::UPDATE));

    let target_grant =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/milexp Target 50", 1)
            .expect("god milexp target grant should be recognized");
    assert_eq!(target_grant.messages, vec!["Gave Target 50 military exp."]);
    let target = world.characters.get(&target_id).unwrap();
    assert_eq!(target.exp, 201);
    assert_eq!(target.military_normal_exp, 1);
    assert_eq!(target.military_points, 55);

    let target_report =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/milexp Target", 1)
            .expect("god milexp target report should be recognized");
    assert_eq!(target_report.messages, vec!["Target has 201 exp."]);
}

#[test]
fn milexp_routes_its_fixed_one_exp_through_give_exp_and_honors_runtime_military_bonus() {
    // C `cmd_milexp` -> `give_military_pts_no_npc(co, val, 1)`
    // (`command.c:3048`, `tool.c:3281-3299`): the exp side is always a
    // fixed `1` through `give_exp` (so `exp_modifier`/`hardcore_exp_bonus`
    // apply), while `military_points` uses the typed amount multiplied by
    // the separately-tunable `hardcore_military_exp_bonus`.
    let mut world = World::default();
    world.settings.exp_modifier = 3.0;
    let god_id = CharacterId(7);
    let target_id = CharacterId(8);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    let mut target = login_character(target_id, &login_block("Target"), 1, 11, 10);
    target.exp = 0;
    target.flags.insert(CharacterFlags::HARDCORE);
    world.add_character(god);
    world.add_character(target);
    let mut runtime = ServerRuntime::default();
    runtime.hardcore_military_exp_bonus = 2.0;

    apply_admin_character_command(&mut world, &mut runtime, god_id, "/milexp Target 50", 1)
        .expect("god milexp target grant should be recognized");

    let target = world.characters.get(&target_id).unwrap();
    // give_exp(co, 1) with exp_modifier 3.0 (no hardcore_exp_bonus set,
    // defaults to 1.0) -> +3, not the raw +1 a bare mutation would give.
    assert_eq!(target.exp, 3);
    assert_eq!(target.military_normal_exp, 1);
    // 50 * hardcore_military_exp_bonus(2.0) = 100, not the old hardcoded
    // 1.10 multiplier.
    assert_eq!(target.military_points, 100);
}

#[test]
fn milexp_command_is_god_only_full_command_and_not_found_feedback() {
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

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/milexp 10", 1)
            .is_none()
    );

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
        "/milexp Missing 10",
        1,
    )
    .expect("god milexp missing target should be handled");
    assert_eq!(
        missing.messages,
        vec!["Sorry, no one by the name Missing around."]
    );
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/milex 10", 1)
            .is_none()
    );
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/milexpx 10",
        1
    )
    .is_none());
}

#[test]
fn itemmod_is_god_only_requires_cursor_and_checks_bounds() {
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
fn god_setskill_mutates_online_target_and_recalculates_exp_used() {
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
fn setskill_is_god_only_and_checks_target_position_and_value() {
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

#[test]
fn setlevel_is_god_only_and_requires_full_command() {
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
        "/setlevel 36",
        1,
    )
    .is_none());

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setleve 36",
        1,
    )
    .is_none());
}

#[test]
fn noexp_and_nolevel_toggle_legacy_flags_and_feedback() {
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

    let noexp_on =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/noexp", 1)
            .expect("noexp should be recognized");
    assert_eq!(noexp_on.messages, vec!["Turned NoExp mode on."]);
    assert!(noexp_on.inventory_changed);
    assert!(world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::NOEXP));

    let noexp_off =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/noexp", 1)
            .expect("noexp should toggle off");
    assert_eq!(noexp_off.messages, vec!["Turned NoExp mode off."]);
    assert!(!world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::NOEXP));

    let nolevel_on =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/nolevel", 1)
            .expect("nolevel should be recognized");
    assert_eq!(
        nolevel_on.messages,
        vec!["NoLevel mode enabled. You will not level up until you disable this mode."]
    );
    assert!(nolevel_on.inventory_changed);
    assert!(world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::NOLEVEL));

    let nolevel_off =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/nolevel", 1)
            .expect("nolevel should toggle off");
    assert_eq!(
        nolevel_off.messages,
        vec!["NoLevel mode disabled. You will now gain levels normally."]
    );
    assert!(!world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::NOLEVEL));

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/noex", 1,)
            .is_none()
    );
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/noleve", 1,)
            .is_none()
    );
}

#[test]
fn noexp_and_nolevel_cannot_be_enabled_in_gatekeeper_room() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 178, 196);
    character.x = 178;
    character.y = 196;
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let noexp = apply_admin_character_command(&mut world, &mut runtime, character_id, "/noexp", 3)
        .expect("noexp should be recognized");
    assert_eq!(
        noexp.messages,
        vec!["Cannot turn NoExp mode on while in Gatekeeper room."]
    );
    assert!(!noexp.inventory_changed);
    assert!(!world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::NOEXP));

    let nolevel =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/nolevel", 3)
            .expect("nolevel should be recognized");
    assert_eq!(
        nolevel.messages,
        vec!["Cannot turn NoLevel mode on while in Gatekeeper room."]
    );
    assert!(!nolevel.inventory_changed);
    assert!(!world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::NOLEVEL));

    let character = world.characters.get_mut(&character_id).unwrap();
    character
        .flags
        .insert(CharacterFlags::NOEXP | CharacterFlags::NOLEVEL);

    let noexp_off =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/noexp", 3)
            .expect("enabled noexp can be disabled in gatekeeper room");
    assert_eq!(noexp_off.messages, vec!["Turned NoExp mode off."]);
    assert!(!world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::NOEXP));

    let nolevel_off =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/nolevel", 3)
            .expect("enabled nolevel can be disabled in gatekeeper room");
    assert_eq!(
        nolevel_off.messages,
        vec!["NoLevel mode disabled. You will now gain levels normally."]
    );
    assert!(!world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::NOLEVEL));
}

#[test]
fn noexp_gatekeeper_room_guard_is_area_specific() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 178, 196);
    character.x = 178;
    character.y = 196;
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(&mut world, &mut runtime, character_id, "/noexp", 1)
        .expect("noexp outside area 3 should be recognized");
    assert_eq!(result.messages, vec!["Turned NoExp mode on."]);
    assert!(world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::NOEXP));
}

#[test]
fn join_chat_command_gates_staff_and_god_channels() {
    let mut player = PlayerRuntime::connected(1, 0);

    let staff_denied =
        apply_join_leave_chat_command(&mut player, CharacterFlags::PLAYER, "/join 31").unwrap();
    assert_eq!(
        staff_denied.messages,
        vec!["Permission denied to join channel 31 (Staff)."]
    );
    assert_eq!(player.chat_channels, 0);

    let staff_joined = apply_join_leave_chat_command(
        &mut player,
        CharacterFlags::PLAYER | CharacterFlags::EVENTMASTER,
        "/join 31",
    )
    .unwrap();
    assert_eq!(
        staff_joined.messages,
        vec!["You have joined channel 31 (Staff)."]
    );

    let god_denied =
        apply_join_leave_chat_command(&mut player, CharacterFlags::STAFF, "/join 32").unwrap();
    assert_eq!(
        god_denied.messages,
        vec!["Permission denied to join channel 32 (God)."]
    );

    let joined_all =
        apply_join_leave_chat_command(&mut player, CharacterFlags::PLAYER, "/joinall").unwrap();
    assert_eq!(joined_all.messages, vec!["You have joined all channels."]);
    for nr in 1..=13 {
        assert_ne!(player.chat_channels & (1_u32 << (nr - 1)), 0);
    }
}

#[test]
fn weather_command_reports_god_debug_info() {
    let mut world = World::default();
    world.tick = ugaris_core::Tick(24);
    let character_id = CharacterId(7);
    let mut god = login_character(character_id, &login_block("WeatherGod"), 1, 10, 10);
    god.x = 10;
    god.y = 10;
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let weather = WeatherState {
        current_weather: 1,
        weather_intensity: 2,
        weather_effects: WEATHER_EFFECT_DAMAGE,
        is_transitioning: true,
        transition_start: 0,
        transition_duration: 48,
        prev_weather: 0,
        weather_change_time: 240,
        affected_areas: vec![1, 3],
    };

    let result = apply_weather_command(&world, character_id, 1, &weather, "/weather")
        .expect("weather command should be recognized");

    assert_eq!(
        result.messages,
        vec![
            "Current weather in this area: Moderate rain",
            "Global Weather Debug Info:",
            "- Current Weather: Rain",
            "- Intensity: Moderate",
            "- Effects: 0x4",
            "- Transitioning: Yes (1 seconds left)",
            "- Previous Weather: Clear",
            "- Progress: 50.0%",
            "- Next Change: 9 seconds",
            "- Affected Areas (2):",
            "  1 3 ",
            "The weather is causing damage.",
        ]
    );
}

#[test]
fn weather_admin_commands_mutate_runtime_state_with_legacy_feedback() {
    let mut world = World::default();
    world.tick = ugaris_core::Tick(48);
    let character_id = CharacterId(7);
    let mut god = login_character(character_id, &login_block("WeatherGod"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut weather = WeatherState::default();

    let set = apply_weather_admin_command(&world, character_id, &mut weather, "/setweather 2 3")
        .expect("setweather should be recognized");
    assert_eq!(set.messages, vec!["Weather changing to Heavy storm"]);
    assert_eq!(weather.current_weather, 2);
    assert_eq!(weather.weather_intensity, 3);
    assert_eq!(
        weather.weather_effects,
        WEATHER_EFFECT_SLOW | WEATHER_EFFECT_BLIND | WEATHER_EFFECT_SLIP
    );
    assert!(weather.is_transitioning);
    assert_eq!(weather.transition_start, 48);
    assert_eq!(weather.transition_duration, TICKS_PER_SECOND * 60);

    let area =
        apply_weather_admin_command(&world, character_id, &mut weather, "/setareaweather 1 2")
            .expect("setareaweather should be recognized");
    assert_eq!(area.messages, vec!["Set weather in area 1 to Storm"]);
    assert_eq!(weather.affected_areas, vec![1]);

    let clear_area =
        apply_weather_admin_command(&world, character_id, &mut weather, "/setareaweather 1 0")
            .expect("clear area weather should be recognized");
    assert_eq!(clear_area.messages, vec!["Set weather in area 1 to Clear"]);
    assert!(weather.affected_areas.is_empty());

    let clear = apply_weather_admin_command(&world, character_id, &mut weather, "/clearweather")
        .expect("clearweather should be recognized");
    assert_eq!(clear.messages, vec!["Weather clearing globally."]);
    assert_eq!(weather.current_weather, 0);
    assert_eq!(weather.weather_intensity, 1);
    assert_eq!(weather.weather_effects, 0);
}

#[test]
fn weather_admin_commands_preserve_legacy_gates_and_validation() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    world.add_character(login_character(
        character_id,
        &login_block("Weather"),
        1,
        10,
        10,
    ));
    let mut weather = WeatherState::default();

    let denied = apply_weather_admin_command(&world, character_id, &mut weather, "/setweather 1 1")
        .expect("setweather should be recognized");
    assert_eq!(
        denied.messages,
        vec!["You need to be a god to use this command."]
    );
    assert_eq!(weather, WeatherState::default());

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let bad_type =
        apply_weather_admin_command(&world, character_id, &mut weather, "/setweather 9 1")
            .expect("bad setweather should be recognized");
    assert_eq!(
        bad_type.messages[0],
        "Invalid weather type. Valid types are:"
    );

    let bad_intensity =
        apply_weather_admin_command(&world, character_id, &mut weather, "/setweather 1 4")
            .expect("bad intensity should be recognized");
    assert_eq!(
        bad_intensity.messages,
        vec!["Invalid intensity. Must be between 1 (Light) and 3 (Heavy)."]
    );

    let bad_area =
        apply_weather_admin_command(&world, character_id, &mut weather, "/setareaweather 300 1")
            .expect("bad area should be recognized");
    assert_eq!(
        bad_area.messages,
        vec!["Invalid area ID. Must be between 0 and 255."]
    );

    let disallowed =
        apply_weather_admin_command(&world, character_id, &mut weather, "/setareaweather 8 1")
            .expect("disallowed area weather should be recognized");
    assert_eq!(
        disallowed.messages,
        vec!["This weather type is not allowed in area 8."]
    );
}

#[test]
fn tell_command_forwards_to_spying_god_even_when_recipient_blocks() {
    let sender_id = CharacterId(7);
    let target_id = CharacterId(8);
    let spy_id = CharacterId(9);
    let mut world = World::default();
    world.add_character(login_character(
        sender_id,
        &login_block("Sender"),
        1,
        10,
        10,
    ));
    let mut target = login_character(target_id, &login_block("Target"), 1, 11, 10);
    target.flags.insert(CharacterFlags::NOTELL);
    world.add_character(target);
    let mut spy = login_character(spy_id, &login_block("God"), 1, 12, 10);
    spy.flags.insert(CharacterFlags::GOD | CharacterFlags::SPY);
    world.add_character(spy);

    let mut runtime = ServerRuntime::default();
    for (session, id) in [(1, sender_id), (2, target_id), (3, spy_id)] {
        runtime
            .players
            .insert(session, PlayerRuntime::connected(session, 0));
        runtime.players.get_mut(&session).unwrap().character_id = Some(id);
    }

    let result = apply_tell_command(&world, &mut runtime, sender_id, "/tell target secret", 10)
        .expect("tell should be recognized");

    assert!(result.delivered_messages.is_empty());
    assert_eq!(result.delivered_message_bytes.len(), 1);
    assert_eq!(result.delivered_message_bytes[0].0, spy_id);
    assert!(result.delivered_message_bytes[0]
        .1
        .starts_with(COL_DARK_GRAY));
    assert!(
        String::from_utf8_lossy(&result.delivered_message_bytes[0].1)
            .contains("[SPY/TELL] Sender (0) tells you: \"secret\"")
    );
}

#[test]
fn admin_subhelp_commands_match_legacy_privilege_gates_and_text() {
    assert!(apply_help_command("#achelp", CharacterFlags::empty(), 1).is_none());
    let ac = apply_help_command("#achelp", CharacterFlags::STAFF, 1)
        .expect("staff anti-cheat help should be recognized");
    assert_eq!(ac.messages[0], "--- Anti-Cheat Commands ---");
    assert_eq!(
        ac.message_bytes[0],
        b"\xb0c3--- Anti-Cheat Commands ---\xb0c0".to_vec()
    );
    assert!(ac
        .messages
        .contains(&"#acwarn <name> [reason] - Issue AC warning".to_string()));
    let acwarn_index = ac
        .messages
        .iter()
        .position(|message| message == "#acwarn <name> [reason] - Issue AC warning")
        .expect("acwarn line should be present");
    assert_eq!(
        ac.message_bytes[acwarn_index],
        b"\xb0c4#acwarn\xb0c0 \xb0c2<name>\xb0c0 [reason] - Issue AC warning".to_vec()
    );
    assert!(ac
        .messages
        .contains(&"#accleanup <days> - Cleanup old records (God)".to_string()));
    assert!(!ac.inventory_changed);

    assert!(apply_help_command("/macrohelp", CharacterFlags::empty(), 1).is_none());
    let macro_help = apply_help_command("/macrohelp", CharacterFlags::STAFF, 1)
        .expect("staff macro help should be recognized");
    assert_eq!(
        macro_help.messages[0],
        "=== Macro Daemon Admin Commands ==="
    );
    assert!(macro_help
        .messages
        .contains(&"/macroimmune <player> <mins> - Grant immunity (GOD only)".to_string()));
    assert!(macro_help
        .messages
        .contains(&"/macrohelp - Show this help".to_string()));

    assert!(apply_help_command("/penthelp", CharacterFlags::STAFF, 1).is_none());
    let pent = apply_help_command("/penthelp", CharacterFlags::GOD, 1)
        .expect("god pentagram help should be recognized");
    assert_eq!(pent.messages[0], "=== Pentagram Debug Commands (GOD) ===");
    assert!(pent
        .messages
        .contains(&"/setpentcount <player> <n> - Set pent_cnt (run count)".to_string()));
    assert!(pent
        .messages
        .contains(&"/penthelp - Show this help".to_string()));
}
