use super::*;
use ugaris_core::world::SingleMission;

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
fn status_command_reflects_enabled_lag_control_toggles() {
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    character.values[1][CharacterValue::Bless as usize] = 10;
    let mut player = PlayerRuntime::connected(1, 0);
    player.no_bless = true;
    player.no_life = true;
    player.no_move = true;
    player.autobless_enabled = true;

    let result = apply_status_command(&character, &player, "/status")
        .expect("status command should be recognized");

    assert!(result
        .messages
        .contains(&"Don't use Bless [/NOBLESS]: On.".to_string()));
    assert!(result
        .messages
        .contains(&"Don't use Healing Potions [/NOLIFE]: On.".to_string()));
    assert!(result
        .messages
        .contains(&"Don't Move [/NOMOVE]: On.".to_string()));
    assert!(result
        .messages
        .contains(&"Automatic Re-Bless [/AUTOBLESS]: On.".to_string()));
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
fn saveall_command_is_god_only_and_disambiguated_from_saves() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/saveall", 1)
            .is_none()
    );

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);

    // Exactly "/save" (minlen 4) is claimed by the `saves` stat setter,
    // matching C's `cmdcmp(ptr, "saves", 4)` appearing before `cmdcmp(ptr,
    // "saveall", 4)` in `command.c` - it must not trigger `save_all_requested`.
    let saves_result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/save 9", 1)
            .expect("god saves command should still win on exact minlen-4 abbreviation");
    assert!(!saves_result.save_all_requested);
    assert_eq!(world.characters.get(&character_id).unwrap().saves, 9);

    for command in ["/savea", "/saveal", "/saveall"] {
        let result =
            apply_admin_character_command(&mut world, &mut runtime, character_id, command, 1)
                .unwrap_or_else(|| panic!("{command} should be recognized as /saveall"));
        assert!(
            result.save_all_requested,
            "{command} should request save-all"
        );
        assert_eq!(
            result.messages,
            vec![
                "Forcing save of all players...".to_string(),
                "Player data saved".to_string(),
                "Forcing save of merchant inventories...".to_string(),
                "Merchant data saved".to_string(),
            ]
        );
    }
}

#[test]
fn backup_rotation_cursor_cycles_through_connected_players_deterministically() {
    let mut runtime = ServerRuntime::default();
    // No connected players yet.
    assert_eq!(runtime.next_backup_rotation_target(), None);

    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(1, commands, 0);
    runtime.players.get_mut(&1).unwrap().character_id = Some(CharacterId(5));
    let (commands, _rx2) = mpsc::channel(16);
    runtime.connect(2, commands, 0);
    runtime.players.get_mut(&2).unwrap().character_id = Some(CharacterId(3));
    let (commands, _rx3) = mpsc::channel(16);
    runtime.connect(3, commands, 0);
    runtime.players.get_mut(&3).unwrap().character_id = None;

    // Sorted by CharacterId: 3, then 5. Session 3 has no character_id and
    // is skipped, matching C's `player[n] && cn` guard.
    assert_eq!(runtime.next_backup_rotation_target(), Some(CharacterId(3)));
    assert_eq!(runtime.next_backup_rotation_target(), Some(CharacterId(5)));
    // Cursor wraps back to the start once every connected player has been
    // visited once, matching C's `n = 1;` reset at the end of the scan.
    assert_eq!(runtime.next_backup_rotation_target(), Some(CharacterId(3)));
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
fn god_killclub_command_bankrupts_an_existing_club_without_deleting_it() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let nr = world.club_registry.create_club("Doomed", 0).unwrap();
    assert!(world.club_registry.exists(nr));

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/killclub {nr}"),
        1,
    )
    .expect("god killclub command should be recognized");
    assert!(result.messages.is_empty(), "C emits no feedback either");
    // C's `kill_club` doesn't clear the name - it zeroes `money`/sets
    // `paid = 1` so the next weekly `tick_billing` deletes it for
    // nonpayment, matching `killclan`'s own delayed-deletion trick.
    assert!(world.club_registry.exists(nr));
    assert_eq!(
        world.club_registry.tick_billing(3, 1),
        Some(ugaris_core::club::ClubBillingEvent::Deleted {
            club: nr,
            name: "Doomed".to_string(),
        })
    );
    assert!(!world.club_registry.exists(nr));
}

#[test]
fn killclub_requires_god_and_ignores_numbers_at_or_past_the_buggy_maxclan_cap() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/killclub 1",
        1
    )
    .is_none());

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    // C bug: bounds-checked against MAXCLAN (32), not MAXCLUB (16384), so
    // a club number of exactly 32 (a legal club number) is silently
    // ignored by this command.
    let nr = world.club_registry.create_club("Safe", 0).unwrap();
    apply_admin_character_command(&mut world, &mut runtime, character_id, "/killclub 32", 1)
        .expect("still recognized, just a no-op past the buggy cap");
    assert!(world.club_registry.exists(nr));
}

#[test]
fn god_setclanjewels_changes_jewels_and_reports_a_default_log_entry() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let nr = world.clan_registry.found_clan("Jewelers", 0).unwrap();
    world.clan_registry.add_jewel(nr).unwrap();
    world.clan_registry.add_jewel(nr).unwrap();
    assert_eq!(world.clan_registry.jewel_count(nr), 2);

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/setclanjewels {nr} 100"),
        1,
    )
    .expect("god setclanjewels command should be recognized");
    assert_eq!(
        result.messages,
        vec![format!("Clan {nr} (Jewelers) jewels changed from 2 to 100")]
    );
    assert_eq!(world.clan_registry.jewel_count(nr), 100);
    // `do_log` defaults to 1 (C `int do_log = 1;`), so a clan-log entry
    // must be queued for the call site to write.
    assert_eq!(
        result.clan_log_entry,
        Some((
            nr,
            world.clan_registry.serial(nr),
            1,
            "God Tester changed clan jewels from 2 to 100".to_string()
        ))
    );
}

#[test]
fn setclanjewels_do_log_zero_suppresses_the_clan_log_entry() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let nr = world.clan_registry.found_clan("Jewelers", 0).unwrap();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/setclanjewels {nr} 50 0"),
        1,
    )
    .expect("god setclanjewels command should be recognized");
    assert_eq!(
        result.messages,
        vec![format!("Clan {nr} (Jewelers) jewels changed from 0 to 50")]
    );
    assert_eq!(world.clan_registry.jewel_count(nr), 50);
    assert_eq!(result.clan_log_entry, None);
}

#[test]
fn setclanjewels_requires_god_and_rejects_bad_args() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    // Non-god: not recognized at all.
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setclanjewels 1 100",
        1
    )
    .is_none());

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);

    let nr = world.clan_registry.found_clan("Jewelers", 0).unwrap();

    // Negative jewel count is rejected, matching C's `jewels >= 0` guard.
    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/setclanjewels {nr} -5"),
        1,
    )
    .expect("still recognized, just reports the invalid-args message");
    assert_eq!(
        result.messages,
        vec!["Invalid clan number or jewel count".to_string()]
    );
    assert_eq!(world.clan_registry.jewel_count(nr), 0);

    // Out-of-range clan number.
    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setclanjewels 0 100",
        1,
    )
    .expect("still recognized, just reports the invalid-args message");
    assert_eq!(
        result.messages,
        vec!["Invalid clan number or jewel count".to_string()]
    );

    // In-range but nonexistent clan number: also reports the same
    // invalid-args message (see `ClanRegistry::set_jewels`'s doc comment
    // for why this diverges from C's silent nameless-slot write).
    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setclanjewels 31 100",
        1,
    )
    .expect("still recognized, just reports the invalid-args message");
    assert_eq!(
        result.messages,
        vec!["Invalid clan number or jewel count".to_string()]
    );
}

#[test]
fn staff_renclub_command_renames_an_existing_club_in_aston() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::STAFF);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let nr = world.club_registry.create_club("Old Name", 0).unwrap();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/renclub {nr} New Name"),
        3,
    )
    .expect("staff renclub command should be recognized");
    assert_eq!(
        result.messages,
        vec![format!("Club {nr} name changed to \"New Name\".")]
    );
    assert_eq!(world.club_registry.name(nr), Some("New Name"));
}

#[test]
fn renclub_is_rejected_outside_aston_and_for_illegal_names() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    let nr = world.club_registry.create_club("Old Name", 0).unwrap();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/renclub {nr} New Name"),
        1,
    )
    .expect("renclub should still be recognized outside Aston");
    assert_eq!(
        result.messages,
        vec!["Sorry, this command only works nearby a clubmaster."]
    );
    assert_eq!(world.club_registry.name(nr), Some("Old Name"));

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        &format!("/renclub {nr} Illegal123"),
        3,
    )
    .expect("renclub should still be recognized for illegal names");
    assert_eq!(
        result.messages,
        vec!["That didn't work. The name is either taken or illegal."]
    );
    assert_eq!(world.club_registry.name(nr), Some("Old Name"));
}

#[test]
fn renclub_requires_staff_or_god() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/renclub 1 Name",
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

fn seyan_m_loader() -> ZoneLoader {
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
fn god_setseyan_rerolls_target_and_messages_the_target_not_the_caller() {
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
fn setseyan_is_god_only_and_reports_missing_target() {
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
fn setseyan_requires_exact_full_word_no_abbreviation() {
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
    assert_eq!(world.settings.hardcore_military_exp_bonus, 1.5);
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
fn god_game_settings_int_tuning_commands_match_legacy_ranges_and_feedback() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    let lots =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setsplots 8000", 1)
            .expect("god setsplots should be recognized");
    assert_eq!(world.settings.sp_lots, 8000);
    assert_eq!(
        lots.messages,
        vec!["Special item probability 'lots' category changed from 5000 to 8000"]
    );

    let invalid_lots =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setsplots 999", 1)
            .expect("invalid setsplots should still be handled");
    assert_eq!(world.settings.sp_lots, 8000);
    assert_eq!(
        invalid_lots.messages,
        vec!["Invalid value. Please specify a value between 1000 and 10000"]
    );

    let dungeon =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setdungeontime 43200", 1)
            .expect("god setdungeontime should be recognized");
    assert_eq!(world.settings.dungeon_time, 43200);
    assert_eq!(
        dungeon.messages,
        vec!["Dungeon time limit changed from 86400 to 43200 ticks (60 to 30 minutes)"]
    );

    let invalid_dungeon =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setdungeontime 100", 1)
            .expect("invalid setdungeontime should still be handled");
    assert_eq!(world.settings.dungeon_time, 43200);
    assert_eq!(
        invalid_dungeon.messages,
        vec![
            "Invalid value. Please specify a time between 43200 and 172800 ticks (30-120 minutes)"
        ]
    );

    let jewel =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setmaxjewelcount 5", 1)
            .expect("god setmaxjewelcount should be recognized");
    assert_eq!(world.settings.max_jewel_count, 5);
    assert_eq!(
        jewel.messages,
        vec!["Maximum jewel count changed from 2 to 5"]
    );

    let drop_low =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setdropproblow 500", 1)
            .expect("god setdropproblow should be recognized");
    assert_eq!(world.settings.drop_prob_low_level, 500);
    assert_eq!(
        drop_low.messages,
        vec!["Drop probability (low level) changed from 1700 to 500"]
    );
}

#[test]
fn god_game_settings_float_tuning_commands_match_legacy_ranges_and_feedback() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    let divider = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/settunnelexpdivider 4.5",
        1,
    )
    .expect("god settunnelexpdivider should be recognized");
    assert_eq!(world.settings.tunnel_exp_base_value_divider, 4.5);
    assert_eq!(
        divider.messages,
        vec!["Tunnel experience base value divider changed from 5.00 to 4.50"]
    );

    let invalid_divider = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/settunnelexpdivider 0.5",
        1,
    )
    .expect("invalid settunnelexpdivider should still be handled");
    assert_eq!(world.settings.tunnel_exp_base_value_divider, 4.5);
    assert_eq!(
        invalid_divider.messages,
        vec!["Invalid value. Please specify a value between 1.0 and 10.0"]
    );

    let solve =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setexpsolve 1.5", 1)
            .expect("god setexpsolve should be recognized");
    assert_eq!(world.settings.exp_solve_multiplier, 1.5);
    assert_eq!(
        solve.messages,
        vec!["Experience solve multiplier changed from 0.66 to 1.50"]
    );

    let reflection = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setclanreflection 0.5",
        1,
    )
    .expect("god setclanreflection should be recognized");
    assert_eq!(world.settings.exp_clan_reflection_multiplier, 0.5);
    assert_eq!(
        reflection.messages,
        vec!["Clan reflection multiplier changed from 0.70 to 0.50"]
    );

    let rare_mult = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setraredropmultiplier 2.0",
        1,
    )
    .expect("god setraredropmultiplier should be recognized");
    assert_eq!(world.settings.rare_drop_multiplier, 2.0);
    assert_eq!(
        rare_mult.messages,
        vec!["Rare drop multiplier changed from 1.20 to 2.00"]
    );

    let invalid_rare_mult = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setraredropmultiplier 5.0",
        1,
    )
    .expect("invalid setraredropmultiplier should still be handled");
    assert_eq!(world.settings.rare_drop_multiplier, 2.0);
    assert_eq!(
        invalid_rare_mult.messages,
        vec!["Invalid value. Please specify a value between 1.0 and 3.0"]
    );
}

#[test]
fn god_setspecialdropmult_truncates_old_value_like_c() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    // Default `special_item_drop_multiplier` is 1.0; C stores the "old"
    // `double` into an `int` before formatting with `%d` (a genuine
    // truncating-assignment quirk in the C source) and prints the new
    // value with a bare `%f` (6 fractional digits).
    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setspecialdropmult 1500",
        1,
    )
    .expect("god setspecialdropmult should be recognized");
    assert_eq!(world.settings.special_item_drop_multiplier, 1500.0);
    assert_eq!(
        result.messages,
        vec!["Special item drop multiplier changed from 1 to 1500.000000"]
    );

    let invalid = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setspecialdropmult 10001",
        1,
    )
    .expect("invalid setspecialdropmult should still be handled");
    assert_eq!(world.settings.special_item_drop_multiplier, 1500.0);
    assert_eq!(
        invalid.messages,
        vec!["Invalid value. Please specify a value between 1 and 10000"]
    );
}

#[test]
fn god_setjaillocation_and_setastonlocation_update_settings_like_c() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    let jail = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setjaillocation 100 200 5",
        1,
    )
    .expect("god setjaillocation should be recognized");
    assert_eq!(
        (
            world.settings.jail_x,
            world.settings.jail_y,
            world.settings.jail_area
        ),
        (100, 200, 5)
    );
    assert_eq!(
        jail.messages,
        vec!["Jail location changed from 186,234 (area 3) to 100,200 (area 5)"]
    );

    let invalid_jail = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setjaillocation 0 200 5",
        1,
    )
    .expect("invalid setjaillocation should still be handled");
    assert_eq!(
        (
            world.settings.jail_x,
            world.settings.jail_y,
            world.settings.jail_area
        ),
        (100, 200, 5)
    );
    assert_eq!(
        invalid_jail.messages,
        vec!["Invalid coordinates or area. Format: /setjaillocation x y area"]
    );

    let aston = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setastonlocation 50 60 7",
        1,
    )
    .expect("god setastonlocation should be recognized");
    assert_eq!(
        (
            world.settings.aston_x,
            world.settings.aston_y,
            world.settings.aston_area
        ),
        (50, 60, 7)
    );
    assert_eq!(
        aston.messages,
        vec!["Aston location changed from 133,203 (area 3) to 50,60 (area 7)"]
    );
}

#[test]
fn game_settings_tuning_commands_are_god_only_and_resolve_ambiguous_abbreviations_by_source_order()
{
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

    // Non-god caller: recognized-but-gated commands must return `None`.
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setsplots 8000",
        1,
    )
    .is_none());
    assert_eq!(world.settings.sp_lots, GameSettings::default().sp_lots);

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);

    // `setpentmaxpower`'s C `cmdcmp` `minlen` is 15 - equal to the full
    // command's own length, i.e. no abbreviation is accepted at all.
    // Dropping the trailing "r" (14 characters) must not match anything
    // (C `cmdcmp` returns 0 when the matched length is short of `minlen`).
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setpentmaxpowe 5000",
        1,
    )
    .is_none());
    assert_eq!(
        world.settings.max_power_level,
        GameSettings::default().max_power_level
    );

    // "setmax" (6 chars) is a valid abbreviation-length prefix of
    // `setmaxjewelcount` (minlen 16, too short here), `setmaxsilvergolemtype`
    // (minlen 6, matches) and `setmaxclanbonus` (minlen 6, matches) - C's
    // first-declared-wins `if` chain resolves this to
    // `setmaxsilvergolemtype` (`command.c:7610`, declared before
    // `setmaxclanbonus` at `command.c:8008`), so the Rust port must too.
    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/setmax 15", 1)
            .expect("ambiguous setmax abbreviation should still resolve to a command");
    assert_eq!(world.settings.max_silver_golem_type, 15);
    assert_eq!(
        world.settings.max_clan_bonus_percent,
        GameSettings::default().max_clan_bonus_percent
    );
    assert_eq!(
        result.messages,
        vec!["Max silver golem type changed from 8 to 15"]
    );

    // Likewise "setpent" (7 chars) is too short for `setpentvismaxpents`
    // (minlen 18) and `setpentmaxpower` (minlen 15) but long enough for
    // `setpentvaluemultiplier` (minlen 6, declared next at
    // `command.c:7829`), which therefore wins.
    let pent_result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/setpent 99", 1)
            .expect("ambiguous setpent abbreviation should still resolve to a command");
    assert_eq!(world.settings.pentagram_value_multiplier, 99);
    assert_eq!(
        world.settings.max_visible_pents,
        GameSettings::default().max_visible_pents
    );
    assert_eq!(
        pent_result.messages,
        vec!["Pentagram value multiplier changed from 50 to 99"]
    );
}

#[test]
fn god_setlootmod_command_validates_and_stores_modifier() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setlootmod rare_golem_chance 2.5",
        1,
    )
    .expect("god setlootmod should be recognized");
    assert_eq!(world.settings.get_loot_modifier("rare_golem_chance"), 2.5);
    let mut expected = COL_LIGHT_GREEN.to_vec();
    expected.extend_from_slice(b"Loot modifier");
    expected.extend_from_slice(COL_RESET);
    expected.extend_from_slice(b" rare_golem_chance = 2.500");
    assert_eq!(result.message_bytes, vec![expected]);

    // Negative values are rejected without touching the stored modifier
    // (C: `modval < 0.0` guard before `loot_set_modifier` is called).
    let invalid = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setlootmod rare_golem_chance -1",
        1,
    )
    .expect("invalid setlootmod should still be handled");
    assert_eq!(world.settings.get_loot_modifier("rare_golem_chance"), 2.5);
    assert_eq!(invalid.messages, vec!["Usage: #setlootmod <name> <value>"]);

    // Missing name/value is rejected the same way (C: `!modname[0]` guard).
    let missing = apply_admin_character_command(&mut world, &mut runtime, god_id, "/setlootmod", 1)
        .expect("bare setlootmod should still be handled");
    assert_eq!(missing.messages, vec!["Usage: #setlootmod <name> <value>"]);

    // Non-god callers are gated out entirely, same as the other set*
    // knobs.
    let mut player = login_character(CharacterId(8), &login_block("Player"), 1, 11, 10);
    player.flags.remove(CharacterFlags::GOD);
    world.add_character(player);
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        CharacterId(8),
        "/setlootmod foo 1.0",
        1,
    )
    .is_none());
}

#[test]
fn god_reloadloot_command_clears_and_rescans_from_disk() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    // Seed the registry with a sentinel table id that cannot exist in the
    // real on-disk loot data, so a successful clear+rescan is observable
    // regardless of the real `ugaris_data/loot` directory's contents.
    world.loot_registry.load_str(
        r#"{"id":"__test_reloadloot_sentinel__","groups":[{"entries":[{"nothing":true}]}]}"#,
    );
    assert!(world
        .loot_registry
        .find("__test_reloadloot_sentinel__")
        .is_some());

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/reloadloot", 1)
        .expect("god reloadloot should be recognized");
    assert!(world
        .loot_registry
        .find("__test_reloadloot_sentinel__")
        .is_none());
    assert_eq!(result.message_bytes.len(), 1);
    let text = String::from_utf8_lossy(&result.message_bytes[0]).into_owned();
    let n = world.loot_registry.table_count();
    assert!(
        text.ends_with(&format!(" {n} active")) || text.contains("Loot reload failed"),
        "unexpected reloadloot message: {text}"
    );

    // Non-god callers are gated out entirely.
    let mut player = login_character(CharacterId(8), &login_block("Player"), 1, 11, 10);
    player.flags.remove(CharacterFlags::GOD);
    world.add_character(player);
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        CharacterId(8),
        "/reloadloot",
        1,
    )
    .is_none());
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
fn milexp_routes_its_fixed_one_exp_through_give_exp_and_honors_military_bonus() {
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
    world.settings.hardcore_military_exp_bonus = 2.0;

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
fn labsolved_command_is_god_only_and_supports_the_8_char_prefix() {
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
    let mut player = PlayerRuntime::connected(80, 0);
    player.character_id = Some(character_id);
    runtime.players.insert(80, player);

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/labsolved", 1)
            .is_none()
    );

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);

    // C `cmdcmp(ptr, "labsolved", 8)`: the 8-char prefix works too.
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/labsolve", 1)
            .is_some()
    );
    // But a 7-char prefix does not.
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/labsolv", 1)
            .is_none()
    );
}

#[test]
fn labsolved_command_toggles_bits_and_lists_solved_labs_for_self() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut god = login_character(character_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(80, 0);
    player.character_id = Some(character_id);
    runtime.players.insert(80, player);

    // No value: display-only, nothing solved yet.
    let empty =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/labsolved", 1)
            .expect("god labsolved should be recognized");
    assert!(empty.messages.is_empty());

    // Toggle lab 5 on.
    let toggled_on =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/labsolved 5", 1)
            .expect("god labsolved toggle should be recognized");
    assert_eq!(toggled_on.messages, vec!["Godmode has solved lab 5."]);
    assert_eq!(
        runtime
            .player_for_character(character_id)
            .unwrap()
            .lab_solved_bits,
        1u64 << 5
    );

    // Toggle lab 2 on too; both now list, lowest first.
    let toggled_second =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/labsolved 2", 1)
            .expect("god labsolved second toggle should be recognized");
    assert_eq!(
        toggled_second.messages,
        vec!["Godmode has solved lab 2.", "Godmode has solved lab 5."]
    );

    // Toggling lab 5 again un-solves it (XOR, not OR).
    let toggled_off =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/labsolved 5", 1)
            .expect("god labsolved re-toggle should be recognized");
    assert_eq!(toggled_off.messages, vec!["Godmode has solved lab 2."]);

    // Out-of-bounds value reports the error and does not toggle anything.
    let out_of_bounds =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/labsolved 64", 1)
            .expect("god labsolved out-of-bounds should be recognized");
    assert_eq!(
        out_of_bounds.messages,
        vec!["Lab number is out of bounds.", "Godmode has solved lab 2."]
    );
}

#[test]
fn labsolved_command_supports_named_target_and_missing_lookup() {
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
    runtime.players.insert(80, target_player);

    let missing =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/labsolved Missing 3", 1)
            .expect("god labsolved missing target should be handled");
    assert_eq!(
        missing.messages,
        vec!["Sorry, no one by the name Missing around."]
    );

    let granted =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/labsolved Target 3", 1)
            .expect("god labsolved named target should be recognized");
    assert_eq!(granted.messages, vec!["Target has solved lab 3."]);
    assert_eq!(
        runtime
            .player_for_character(target_id)
            .unwrap()
            .lab_solved_bits,
        1u64 << 3
    );
}

#[test]
fn labsolved_command_reports_missing_runtime_for_online_character() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut god = login_character(character_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/labsolved", 1)
            .expect("god labsolved should be recognized even without a runtime");
    assert_eq!(result.messages, vec!["Could not get lab data for Godmode."]);
}

fn setup_god_and_target_with_military_ppd(
    world: &mut World,
    runtime: &mut ServerRuntime,
) -> (CharacterId, CharacterId) {
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
    let mut target_player = PlayerRuntime::connected(80, 0);
    target_player.character_id = Some(target_id);
    runtime.players.insert(80, target_player);
    (god_id, target_id)
}

#[test]
fn god_milinfo_command_reports_self_defaults_and_named_target_state() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_target_with_military_ppd(&mut world, &mut runtime);

    let self_report =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/milinfo", 1);
    assert_eq!(
        self_report,
        Some(KeyringCommandResult {
            messages: vec!["Could not get military data for Godmode.".to_string()],
            ..Default::default()
        })
    );

    let mut god_player = PlayerRuntime::connected(81, 0);
    god_player.character_id = Some(god_id);
    runtime.players.insert(81, god_player);

    let self_report =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/milinfo", 1)
            .expect("god milinfo self report should be recognized");
    assert_eq!(self_report.messages[0], "Military Info for Godmode:");
    assert_eq!(self_report.messages[1], "Rank: nobody (Military points: 0)");
    assert_eq!(self_report.messages[2], "Current recommendation points: 0");
    assert_eq!(
        self_report.messages[3],
        "Total military experience earned: 0"
    );
    assert_eq!(self_report.messages[4], "No active mission");
    assert_eq!(self_report.messages[5], "Mission type preference: 0 (None)");
    assert_eq!(
        self_report.messages[6],
        "Mission difficulty preference: 0 (easy)"
    );

    world
        .characters
        .get_mut(&target_id)
        .unwrap()
        .military_points = 100;
    world
        .characters
        .get_mut(&target_id)
        .unwrap()
        .military_normal_exp = 42;
    {
        let target_player = runtime.player_for_character_mut(target_id).unwrap();
        target_player.set_military_current_pts(5);
        target_player.set_military_mission(
            1,
            SingleMission {
                mission_type: 1,
                opt1: 3,
                opt2: 25,
                pts: 10,
                exp: 200,
            },
        );
        target_player.set_military_took_mission(2);
        target_player.set_mission_type_preference(2);
        target_player.set_mission_difficulty_preference(3);
    }

    let target_report =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/milinfo Target", 1)
            .expect("god milinfo target report should be recognized");
    assert_eq!(target_report.messages[0], "Military Info for Target:");
    assert_eq!(
        target_report.messages[1],
        "Rank: Corporal (Military points: 100)"
    );
    assert_eq!(
        target_report.messages[2],
        "Current recommendation points: 5"
    );
    assert_eq!(
        target_report.messages[3],
        "Total military experience earned: 42"
    );
    assert_eq!(
        target_report.messages[4],
        "Current mission: Demon Slaying (Difficulty: normal)"
    );
    assert_eq!(target_report.messages[5], "Target: 3 level 25 enemies");
    assert_eq!(
        target_report.messages[6],
        "Mission type preference: 2 (Ratling)"
    );
    assert_eq!(
        target_report.messages[7],
        "Mission difficulty preference: 3 (impossible)"
    );

    let missing =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/milinfo Missing", 1)
            .expect("god milinfo missing target should be handled");
    assert_eq!(
        missing.messages,
        vec!["Sorry, no one by the name Missing around."]
    );

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, target_id, "/milinfo", 1).is_none()
    );
}

#[test]
fn god_milpref_command_sets_preferences_and_replicates_missing_diff_quirk() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_target_with_military_ppd(&mut world, &mut runtime);
    runtime
        .player_for_character_mut(_target_id)
        .unwrap()
        .set_mission_yday(99);

    let missing_name =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/milpref", 1)
            .expect("god milpref usage should be recognized");
    assert_eq!(
        missing_name.messages,
        vec![
            "Usage: /milpref <character> <type> <difficulty>",
            "Types: 0=none, 1=demon, 2=ratling, 3=silver",
            "Difficulties: 0=easy, 1=normal, 2=hard, 3=impossible, 4=insane, -1=none",
        ]
    );

    let both =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/milpref Target 2 3", 1)
            .expect("god milpref with both args should be recognized");
    assert_eq!(
        both.messages,
        vec![
            "Set mission type preference to 2 (Ratling) for Target",
            "Set mission difficulty preference to 3 (impossible) for Target",
            "New missions will be generated with these preferences when player visits the Military Master",
        ]
    );
    let target_player = runtime.player_for_character(_target_id).unwrap();
    assert_eq!(target_player.mission_type_preference(), 2);
    assert_eq!(target_player.mission_difficulty_preference(), 3);
    assert_eq!(target_player.mission_yday(), 0);

    // Real C quirk: omitting the difficulty argument still overwrites the
    // preference to -1 ("None"), since C's `diff` default of -1 itself
    // satisfies the `diff>=-1 && diff<5` acceptance range.
    let type_only =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/milpref Target 1", 1)
            .expect("god milpref with only type should be recognized");
    assert_eq!(
        type_only.messages,
        vec![
            "Set mission type preference to 1 (Demon) for Target",
            "Set mission difficulty preference to -1 (None) for Target",
            "New missions will be generated with these preferences when player visits the Military Master",
        ]
    );
    let target_player = runtime.player_for_character(_target_id).unwrap();
    assert_eq!(target_player.mission_difficulty_preference(), -1);
}

#[test]
fn milpref_is_god_only_and_reports_missing_target() {
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
        "/milpref Missing 1 1",
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
        "/milpref Missing 1 1",
        1,
    )
    .expect("god milpref missing target should be handled");
    assert_eq!(
        missing.messages,
        vec!["Sorry, no one by the name Missing around."]
    );
}

#[test]
fn god_milreset_command_clears_all_cooldowns_including_advisors() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_target_with_military_ppd(&mut world, &mut runtime);

    {
        let target_player = runtime.player_for_character_mut(target_id).unwrap();
        target_player.set_mission_yday(50);
        target_player.set_military_solved_yday(49);
        target_player.set_military_took_mission(3);
        target_player.set_military_reroll_yday(48);
        for advisor in 0..MILITARY_PPD_MAXADVISOR {
            target_player.set_military_advisor_last(advisor, 10 + advisor as i32);
        }
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/milreset Target", 1)
            .expect("god milreset should be recognized");
    assert_eq!(
        result.messages,
        vec!["Reset all mission and advisor cooldowns for Target"]
    );

    let target_player = runtime.player_for_character(target_id).unwrap();
    assert_eq!(target_player.mission_yday(), 0);
    assert_eq!(target_player.military_solved_yday(), 0);
    assert_eq!(target_player.military_took_mission(), 0);
    assert_eq!(target_player.military_reroll_yday(), 0);
    for advisor in 0..MILITARY_PPD_MAXADVISOR {
        assert_eq!(target_player.military_advisor_last(advisor), 0);
    }
}

#[test]
fn god_milpoints_command_grants_points_and_promotes_with_broadcast() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_target_with_military_ppd(&mut world, &mut runtime);

    let missing_points =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/milpoints Target", 1)
            .expect("god milpoints without a value should be recognized");
    assert_eq!(
        missing_points.messages,
        vec!["Please specify number of points to grant."]
    );

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/milpoints Target 4096",
        1,
    )
    .expect("god milpoints should be recognized");
    assert_eq!(
        result.messages,
        vec!["Granted 4096 military points to Target, promoting to Brigadier General!"]
    );
    assert_eq!(
        world.characters.get(&target_id).unwrap().military_points,
        4096
    );

    let no_promotion =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/milpoints Target 1", 1)
            .expect("god milpoints without a rank change should be recognized");
    assert_eq!(
        no_promotion.messages,
        vec!["Granted 1 military points to Target (total: 4097)"]
    );
}

#[test]
fn milpoints_is_god_only_and_requires_name_and_nonzero_points() {
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
        "/milpoints Tester 10",
        1,
    )
    .is_none());

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let usage =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/milpoints", 1)
            .expect("god milpoints usage should be recognized");
    assert_eq!(
        usage.messages,
        vec!["Usage: /milpoints <character> <points>"]
    );

    let zero = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/milpoints Tester 0",
        1,
    )
    .expect("god milpoints zero points should be recognized");
    assert_eq!(
        zero.messages,
        vec!["Please specify number of points to grant."]
    );
}

#[test]
fn god_milrec_command_grants_recommendation_points() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_target_with_military_ppd(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/milrec Target 7", 1)
            .expect("god milrec should be recognized");
    assert_eq!(
        result.messages,
        vec!["Granted 7 recommendation points to Target (total: 7)"]
    );
    assert_eq!(
        runtime
            .player_for_character(target_id)
            .unwrap()
            .military_current_pts(),
        7
    );

    let second =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/milrec Target 3", 1)
            .expect("god milrec second grant should be recognized");
    assert_eq!(
        second.messages,
        vec!["Granted 3 recommendation points to Target (total: 10)"]
    );
}

#[test]
fn milrec_is_god_only_requires_name_and_nonzero_points() {
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
        "/milrec Tester 10",
        1,
    )
    .is_none());

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let usage = apply_admin_character_command(&mut world, &mut runtime, character_id, "/milrec", 1)
        .expect("god milrec usage should be recognized");
    assert_eq!(usage.messages, vec!["Usage: /milrec <character> <points>"]);
}

#[test]
fn god_milstats_command_reports_missing_military_master_npc() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/milstats", 1)
        .expect("god milstats should be recognized");
    assert_eq!(result.messages, vec!["Could not find Military Master NPC."]);
}

#[test]
fn milstats_is_god_only() {
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
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/milstats", 1)
            .is_none()
    );
}

#[test]
fn god_milsolve_command_completes_mission_promotes_and_announces() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_target_with_military_ppd(&mut world, &mut runtime);

    {
        let target_player = runtime.player_for_character_mut(target_id).unwrap();
        target_player.set_military_mission(
            0,
            SingleMission {
                mission_type: 1,
                opt1: 5,
                opt2: 30,
                pts: 4096,
                exp: 500,
            },
        );
        target_player.set_military_took_mission(1);
    }

    let no_active =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/milsolve Godmode", 1);
    // The god itself has no PlayerRuntime registered in `runtime.players`
    // at this point (only the target does), so this exercises the
    // "Could not get military data" branch instead.
    assert_eq!(
        no_active,
        Some(KeyringCommandResult {
            messages: vec!["Could not get military data for Godmode.".to_string()],
            ..Default::default()
        })
    );

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/milsolve Target announce",
        1,
    )
    .expect("god milsolve should be recognized");
    assert_eq!(
        result.messages,
        vec!["Completed easy Demon mission for Target! Rewards: 4096 mil pts, 500 exp. Promoted to Brigadier General!"]
    );
    let target = world.characters.get(&target_id).unwrap();
    assert_eq!(target.military_points, 4096);
    assert_eq!(target.military_normal_exp, 500);
    let target_player = runtime.player_for_character(target_id).unwrap();
    assert!(target_player.military_solved_mission());
    assert_eq!(target_player.military_took_mission(), 0);

    let no_mission =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/milsolve Target", 1)
            .expect("god milsolve without an active mission should be recognized");
    assert_eq!(
        no_mission.messages,
        vec!["Target does not have an active mission."]
    );
}

#[test]
fn milsolve_is_god_only_and_reports_missing_target() {
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
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/milsolve", 1)
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
        "/milsolve Missing",
        1,
    )
    .expect("god milsolve missing target should be handled");
    assert_eq!(
        missing.messages,
        vec!["Sorry, no one by the name Missing around."]
    );
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
        seasonal_influence: SEASON_SPRING,
        elemental_debuff_last_notify: HashMap::new(),
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
        WEATHER_EFFECT_SLOW
            | WEATHER_EFFECT_BLIND
            | WEATHER_EFFECT_SLIP
            | WEATHER_EFFECT_SKILL
            | WEATHER_EFFECT_LIGHTNING
            | WEATHER_EFFECT_ELEMENTAL
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

fn goto_test_world() -> World {
    let mut world = World::default();
    world.map = ugaris_core::map::MapGrid::new(300, 300);
    // Past the `/jump` busy window (`ticker - ch[cn].regen_ticker < TICKS *
    // 3`) so freshly logged-in test characters (`regen_ticker: 0`) aren't
    // considered "still catching their breath".
    world.tick.0 = TICKS_PER_SECOND * 10;
    world
}

#[test]
fn goto_command_requires_lqmaster_permission() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    assert!(world.spawn_character(
        login_character(character_id, &login_block("Ralph"), 1, 10, 10),
        10,
        10
    ));
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/goto 50 60",
        1
    )
    .is_none());
    assert_eq!(world.characters.get(&character_id).unwrap().x, 10);

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
        "/goto 50 60",
        1
    )
    .is_some());
}

#[test]
fn goto_command_numeric_coordinates_teleport_same_area() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(character, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/goto 50 60", 1)
            .expect("god goto command should be recognized");
    assert!(result.messages.is_empty());
    assert_eq!(result.mirror_changed, None);
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (50, 60));
}

#[test]
fn goto_command_named_location_normalizes_to_same_area() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(character, 10, 10));
    let mut runtime = ServerRuntime::default();

    // "fort" is gl[]'s (126,179,1); area_id 1 matches the caller's current
    // area so C's `if (a == areaID && !m) a = 0;` normalizes this to a
    // plain same-area teleport (not a `change_area` handoff).
    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/goto fort", 1)
            .expect("god goto command should be recognized");
    assert!(result.messages.is_empty());
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (126, 179));
}

#[test]
fn goto_command_named_location_cross_area_reports_server_down() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(character, 10, 10));
    let mut runtime = ServerRuntime::default();

    // "aston" is gl[]'s (167,188,3); the caller is in area 1, so C would
    // call `change_area(cn, 3, 167, 188)` - real cross-process area
    // handoff isn't ported, so this resolves to the same no-op message
    // as every other cross-area teleport in this codebase.
    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/goto aston", 1)
            .expect("god goto command should be recognized");
    assert_eq!(
        result.messages,
        vec!["Nothing happens - target area server is down.".to_string()]
    );
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (10, 10));
}

#[test]
fn goto_command_non_god_lqmaster_ignores_cross_area_and_uses_local_coords() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 10, 10);
    character.flags.insert(CharacterFlags::EVENTMASTER);
    assert!(world.spawn_character(character, 10, 10));
    let mut runtime = ServerRuntime::default();

    // C `if (!(ch[cn].flags & CF_GOD)) a = 0;` forces non-GOD `is_lqmaster`
    // callers (here: `CF_EVENTMASTER`) to always land locally, using the
    // resolved x/y but ignoring the resolved area entirely - even though
    // "aston" nominally lives in a different area.
    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/goto aston", 1)
            .expect("eventmaster goto command should be recognized");
    assert!(result.messages.is_empty());
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (167, 188));
}

#[test]
fn goto_command_looks_up_online_character_by_name() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let target_id = CharacterId(2);
    assert!(world.spawn_character(
        login_character(target_id, &login_block("Lisa"), 1, 77, 88),
        77,
        88
    ));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/goto Lisa", 1)
            .expect("god goto command should be recognized");
    assert!(result.messages.is_empty());
    let caller = world.characters.get(&caller_id).unwrap();
    // Target's own tile is occupied, so `drop_char`'s neighbor fallback
    // (matching C's own `teleport_char_driver`) lands the caller on an
    // adjacent tile rather than exactly on top of the target.
    let dx = i32::from(caller.x) - 77;
    let dy = i32::from(caller.y) - 88;
    assert!(dx.abs() <= 1 && dy.abs() <= 1 && (dx, dy) != (0, 0));
}

#[test]
fn goto_command_direction_shorthand_offsets_from_caller_position() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 100, 100);
    character.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(character, 100, 100));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/goto 10 n", 1)
            .expect("god goto command should be recognized");
    assert!(result.messages.is_empty());
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (90, 90));
}

#[test]
fn goto_command_mirror_argument_always_forces_cross_area_handoff() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(character, 10, 10));
    let mut runtime = ServerRuntime::default();

    // C sets `ch[cn].mirror = m` unconditionally and forces `a = areaID`
    // when it was still 0, which then *fails* the `a == areaID && !m`
    // same-area normalization (because `m != 0`) - so requesting a mirror
    // always routes through `change_area`, even when the area number is
    // literally the caller's own current area. Copied as-is (a real C
    // quirk, not a Rust bug): the mirror still gets set even though the
    // teleport itself becomes a same-area-server-down no-op.
    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/goto 50 60 0 3", 1)
            .expect("god goto command should be recognized");
    assert_eq!(
        result.messages,
        vec!["Nothing happens - target area server is down.".to_string()]
    );
    assert_eq!(result.mirror_changed, Some(3));
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (10, 10));
}

#[test]
fn jump_command_requires_staff_or_god() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    assert!(world.spawn_character(
        login_character(character_id, &login_block("Ralph"), 1, 10, 10),
        10,
        10
    ));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/jump fort", 1)
            .is_none()
    );
}

#[test]
fn jump_command_refuses_while_busy() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 10, 10);
    character.flags.insert(CharacterFlags::STAFF);
    character.action = 1;
    assert!(world.spawn_character(character, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/jump fort", 1)
            .expect("staff jump command should be recognized");
    assert_eq!(result.messages, vec!["Pant, pant. Too tired.".to_string()]);
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (10, 10));
}

#[test]
fn jump_command_moves_staff_to_gotolist_entry_in_same_area() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 10, 10);
    character.flags.insert(CharacterFlags::STAFF);
    assert!(world.spawn_character(character, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/jump fort", 1)
            .expect("staff jump command should be recognized");
    assert!(result.messages.is_empty());
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (126, 179));
}

#[test]
fn jump_command_cross_area_is_not_restricted_to_god() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 10, 10);
    character.flags.insert(CharacterFlags::STAFF);
    assert!(world.spawn_character(character, 10, 10));
    let mut runtime = ServerRuntime::default();

    // Unlike `/goto`, C's `/jump` has no `CF_GOD`-only restriction on the
    // cross-area branch - a plain `CF_STAFF` caller jumping to a
    // different-area `gl[]` entry ("aston" is area 3) still reaches
    // `change_area` in C, so it gets the same server-down no-op here.
    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/jump aston", 1)
            .expect("staff jump command should be recognized");
    assert_eq!(
        result.messages,
        vec!["Nothing happens - target area server is down.".to_string()]
    );
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (10, 10));
}

#[test]
fn jump_command_unknown_location_reports_hu() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 10, 10);
    character.flags.insert(CharacterFlags::STAFF);
    assert!(world.spawn_character(character, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/jump nowhere", 1)
            .expect("staff jump command should be recognized");
    assert_eq!(result.messages, vec!["hu?".to_string()]);
}

#[test]
fn gotolist_command_is_god_only_and_lists_every_shortcut() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    assert!(world.spawn_character(
        login_character(character_id, &login_block("Ralph"), 1, 10, 10),
        10,
        10
    ));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/gotolist", 1)
            .is_none()
    );

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/gotolist", 1)
            .expect("god gotolist command should be recognized");
    assert_eq!(result.messages[0], "Available /goto locations:");
    assert!(result
        .messages
        .contains(&"aston (x:167, y:188, area:3)".to_string()));
    assert!(result
        .messages
        .contains(&"teufelearthgambler (x:248, y:238, area:34)".to_string()));
    assert_eq!(result.messages.len(), 1 + 79);
}

#[test]
fn gotosearch_command_is_case_sensitive_like_c_strstr() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(character, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/gotosearch teufel",
        1,
    )
    .expect("god gotosearch command should be recognized");
    assert_eq!(result.messages[0], "Matching /goto locations:");
    assert!(result
        .messages
        .contains(&"teufelicegambler (x:84, y:186, area:34)".to_string()));
    assert!(result
        .messages
        .contains(&"Found 5 matching locations.".to_string()));

    // C `strstr`, not `strcasestr` - an uppercase term matches nothing.
    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/gotosearch TEUFEL",
        1,
    )
    .expect("god gotosearch command should be recognized");
    assert_eq!(
        result.messages,
        vec![
            "Matching /goto locations:".to_string(),
            "No matching locations found.".to_string()
        ]
    );
}

#[test]
fn summon_command_requires_god() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    assert!(world.spawn_character(
        login_character(caller_id, &login_block("Ralph"), 1, 10, 10),
        10,
        10
    ));
    let target_id = CharacterId(2);
    assert!(world.spawn_character(
        login_character(target_id, &login_block("Lydia"), 1, 90, 90),
        90,
        90
    ));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/summon Lydia", 1)
            .is_none()
    );
    assert_eq!(world.characters.get(&target_id).unwrap().x, 90);
}

#[test]
fn summon_command_teleports_named_character_next_to_caller() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let target_id = CharacterId(2);
    assert!(world.spawn_character(
        login_character(target_id, &login_block("Lydia"), 1, 90, 90),
        90,
        90
    ));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/summon lydia", 1)
            .expect("god summon command should be recognized");
    assert!(result.messages.is_empty());
    let target = world.characters.get(&target_id).unwrap();
    assert!((i32::from(target.x) - 10).abs() + (i32::from(target.y) - 10).abs() < 2);
}

#[test]
fn summon_command_unknown_name_is_a_silent_no_op() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/summon Nobody", 1)
            .expect("god summon command should be recognized");
    assert!(result.messages.is_empty());
}

#[test]
fn kick_command_requires_staff_or_god() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    assert!(world.spawn_character(
        login_character(caller_id, &login_block("Ralph"), 1, 10, 10),
        10,
        10
    ));
    let target_id = CharacterId(2);
    assert!(world.spawn_character(
        login_character(target_id, &login_block("Lydia"), 1, 90, 90),
        90,
        90
    ));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/kick Lydia", 1)
            .is_none()
    );
    assert!(world.characters.contains_key(&target_id));
}

#[test]
fn kick_command_signals_target_teardown_for_staff() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::STAFF);
    assert!(world.spawn_character(caller, 10, 10));
    let target_id = CharacterId(2);
    assert!(world.spawn_character(
        login_character(target_id, &login_block("Lydia"), 1, 90, 90),
        90,
        90
    ));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/kick lydia", 1)
            .expect("staff kick command should be recognized");
    assert_eq!(result.messages, vec!["Kicked lydia.".to_string()]);
    assert_eq!(result.kick_target, Some(target_id));
    // Command dispatch only signals the teardown; the actual save/
    // despawn/disconnect happens at the async call site in main.rs, so
    // the character is still present here.
    assert!(world.characters.contains_key(&target_id));
}

#[test]
fn kick_command_ignores_npcs_by_name() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let npc_id = CharacterId(2);
    let mut npc = login_character(npc_id, &login_block("Goblin"), 1, 90, 90);
    npc.flags.remove(CharacterFlags::PLAYER);
    npc.flags.insert(CharacterFlags::ALIVE);
    assert!(world.spawn_character(npc, 90, 90));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/kick goblin", 1)
            .expect("god kick command should be recognized");
    assert_eq!(
        result.messages,
        vec!["No player by the name goblin.".to_string()]
    );
    assert_eq!(result.kick_target, None);
}

#[test]
fn kick_command_unknown_name_reports_not_found() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/kick Nobody", 1)
            .expect("god kick command should be recognized");
    assert_eq!(
        result.messages,
        vec!["No player by the name Nobody.".to_string()]
    );
}

#[test]
fn summonall_command_requires_god() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    assert!(world.spawn_character(
        login_character(caller_id, &login_block("Ralph"), 1, 10, 10),
        10,
        10
    ));
    let other_id = CharacterId(2);
    assert!(world.spawn_character(
        login_character(other_id, &login_block("Lydia"), 1, 90, 90),
        90,
        90
    ));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/summonall", 1)
            .is_none()
    );
    assert_eq!(world.characters.get(&other_id).unwrap().x, 90);
}

#[test]
fn summonall_command_teleports_every_player_next_to_caller() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let other_a = CharacterId(2);
    assert!(world.spawn_character(
        login_character(other_a, &login_block("Lydia"), 1, 90, 90),
        90,
        90
    ));
    let other_b = CharacterId(3);
    assert!(world.spawn_character(
        login_character(other_b, &login_block("Gwendylon"), 1, 200, 200),
        200,
        200
    ));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/summonall", 1)
            .expect("god summonall command should be recognized");
    assert!(result.messages.is_empty());
    for id in [other_a, other_b] {
        let character = world.characters.get(&id).unwrap();
        assert!((i32::from(character.x) - 10).abs() + (i32::from(character.y) - 10).abs() < 2);
    }
    // The caller themselves stays put (`teleport_char_driver` is a no-op
    // under Manhattan distance 2, and the caller is already at (10,10)).
    assert_eq!(world.characters.get(&caller_id).unwrap().x, 10);
}

#[test]
fn summonall_command_does_not_teleport_npcs() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let npc_id = CharacterId(2);
    let mut npc = login_character(npc_id, &login_block("Goblin"), 1, 90, 90);
    npc.flags.remove(CharacterFlags::PLAYER);
    npc.flags.insert(CharacterFlags::ALIVE);
    assert!(world.spawn_character(npc, 90, 90));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/summonall", 1)
            .expect("god summonall command should be recognized");
    assert!(result.messages.is_empty());
    assert_eq!(world.characters.get(&npc_id).unwrap().x, 90);
}

#[test]
fn office_command_requires_god() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    assert!(world.spawn_character(
        login_character(caller_id, &login_block("Ralph"), 3, 10, 10),
        10,
        10
    ));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/office", 3).is_none()
    );
    assert_eq!(world.characters.get(&caller_id).unwrap().x, 10);
}

#[test]
fn office_command_teleports_within_aston() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(&mut world, &mut runtime, caller_id, "/office", 3)
        .expect("god office command should be recognized");
    assert!(result.messages.is_empty());
    let character = world.characters.get(&caller_id).unwrap();
    assert_eq!((character.x, character.y), (11, 195));
}

#[test]
fn office_command_from_another_area_reports_cross_area_no_op() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(&mut world, &mut runtime, caller_id, "/office", 1)
        .expect("god office command should be recognized");
    assert_eq!(
        result.messages,
        vec!["Nothing happens - target area server is down.".to_string()]
    );
    // Position is unaffected since the cross-area handoff is unported.
    let character = world.characters.get(&caller_id).unwrap();
    assert_eq!((character.x, character.y), (10, 10));
}

#[test]
fn office_command_abbreviation_is_not_recognized() {
    // C `cmdcmp(ptr, "office", 6)` requires the full six-letter word;
    // there is no shorter valid abbreviation.
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/offic", 3).is_none()
    );
    assert_eq!(world.characters.get(&caller_id).unwrap().x, 10);
}

// C `/jail`/`/unjail <name>` (`command.c:8839-8882`), `CF_STAFF|CF_GOD`-
// gated, full-word only. Both commands defer to `World::
// queue_jail_lookup`/`apply_jail_events` for the actual DB round trip and
// online-scan/mutation (see `world/jail.rs`'s tests for that half), so
// these dispatch-level tests only cover permission gating, exact-word
// matching, and that a valid-looking name is queued rather than answered
// immediately.

#[test]
fn jail_command_requires_staff_or_god() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    assert!(world.spawn_character(
        login_character(caller_id, &login_block("Ralph"), 3, 10, 10),
        10,
        10
    ));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/jail Baddie", 3)
            .is_none()
    );
    assert!(world.drain_pending_jail_lookups().is_empty());
}

#[test]
fn jail_command_queues_a_lookup_for_a_valid_name() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::STAFF);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/jail Baddie", 3)
            .expect("staff jail command should be recognized");
    assert!(result.messages.is_empty());
    let queued = world.drain_pending_jail_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, caller_id);
    assert_eq!(queued[0].target_name, "Baddie");
    assert_eq!(queued[0].action, ugaris_core::world::JailAction::Jail);
}

#[test]
fn unjail_command_queues_a_lookup_with_the_unjail_action() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/unjail Baddie", 3)
            .expect("god unjail command should be recognized");
    assert!(result.messages.is_empty());
    let queued = world.drain_pending_jail_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].target_name, "Baddie");
    assert_eq!(queued[0].action, ugaris_core::world::JailAction::Unjail);
}

#[test]
fn jail_command_with_an_invalid_name_is_rejected_immediately() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    // C `lookup_name`'s `strlen(name) < 2` gate (`lookup.c:57-59`).
    let result = apply_admin_character_command(&mut world, &mut runtime, caller_id, "/jail A", 3)
        .expect("god jail command should be recognized");
    assert!(result.messages.is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].character_id, caller_id);
    assert_eq!(texts[0].message, "No character by the name A.");
    assert!(world.drain_pending_jail_lookups().is_empty());
}

#[test]
fn jail_command_abbreviation_is_not_recognized() {
    // C `cmdcmp(ptr, "jail", 4)` requires the full four-letter word.
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/jai Baddie", 3)
            .is_none()
    );
}

#[test]
fn unjail_command_abbreviation_is_not_recognized() {
    // C `cmdcmp(ptr, "unjail", 6)` requires the full six-letter word.
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/unjai Baddie", 3)
            .is_none()
    );
}

// C `cmd_flag` (`command.c:2870-2937`), shared by `/god`, `/setsir`,
// `/staff`, `/emaster`, `/devel`, `/hardcore`, and `/qmaster`
// (`command.c:9257-9337`).

#[test]
fn god_command_requires_god_permission() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    assert!(world.spawn_character(
        login_character(caller_id, &login_block("Ralph"), 1, 10, 10),
        10,
        10
    ));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/god Ralph", 1)
            .is_none()
    );
}

#[test]
fn god_command_toggles_a_named_online_character_and_names_the_flag() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Caller"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let target_id = CharacterId(2);
    assert!(world.spawn_character(
        login_character(target_id, &login_block("Target"), 1, 20, 20),
        20,
        20
    ));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/god Target", 1)
            .expect("god command should be recognized");
    assert_eq!(result.messages, vec!["Set Target god to on.".to_string()]);
    assert!(world.characters[&target_id]
        .flags
        .contains(CharacterFlags::GOD));

    // Toggling again turns it back off and reports "off".
    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/god Target", 1)
            .expect("god command should be recognized");
    assert_eq!(result.messages, vec!["Set Target god to off.".to_string()]);
    assert!(!world.characters[&target_id]
        .flags
        .contains(CharacterFlags::GOD));
}

#[test]
fn god_command_with_invalid_shape_name_reports_no_player_immediately() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Caller"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    // C's `isalpha`-only name scan stops at the first non-alphabetic
    // byte (`command.c:2874-2876`), so `/god a1` only ever sees `"a"`.
    let result = apply_admin_character_command(&mut world, &mut runtime, caller_id, "/god a1", 1)
        .expect("god command should be recognized");
    assert_eq!(
        result.messages,
        vec!["Sorry, no player by the name a.".to_string()]
    );
    assert!(world.drain_pending_admin_flag_toggles().is_empty());
}

#[test]
fn god_command_with_validly_shaped_unmatched_name_is_queued_with_no_immediate_message() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Caller"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/god Nobodyhome", 1)
            .expect("god command should be recognized");
    assert!(result.messages.is_empty());
    let queued = world.drain_pending_admin_flag_toggles();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, caller_id);
    assert_eq!(queued[0].target_name, "Nobodyhome");
    assert_eq!(queued[0].flag, CharacterFlags::GOD);
}

#[test]
fn setsir_command_toggles_won_and_reports_sir_lady_flag_name() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Caller"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let target_id = CharacterId(2);
    assert!(world.spawn_character(
        login_character(target_id, &login_block("Target"), 1, 20, 20),
        20,
        20
    ));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/setsir Target", 1)
            .expect("setsir command should be recognized");
    assert_eq!(
        result.messages,
        vec!["Set Target sir/lady to on.".to_string()]
    );
    assert!(world.characters[&target_id]
        .flags
        .contains(CharacterFlags::WON));
}

#[test]
fn staff_emaster_devel_hardcore_qmaster_toggle_their_own_flags() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Caller"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let target_id = CharacterId(2);
    assert!(world.spawn_character(
        login_character(target_id, &login_block("Target"), 1, 20, 20),
        20,
        20
    ));
    let mut runtime = ServerRuntime::default();

    let cases: [(&str, CharacterFlags, &str); 5] = [
        ("/staff Target", CharacterFlags::STAFF, "staff"),
        (
            "/emaster Target",
            CharacterFlags::EVENTMASTER,
            "master of events",
        ),
        ("/devel Target", CharacterFlags::DEVELOPER, "developer"),
        ("/hardcore Target", CharacterFlags::HARDCORE, "hardcore"),
        ("/qmaster Target", CharacterFlags::LQMASTER, "qmaster"),
    ];
    for (command, flag, flag_name) in cases {
        let result = apply_admin_character_command(&mut world, &mut runtime, caller_id, command, 1)
            .unwrap_or_else(|| panic!("{command} should be recognized"));
        assert_eq!(
            result.messages,
            vec![format!("Set Target {flag_name} to on.")],
            "{command}"
        );
        assert!(
            world.characters[&target_id].flags.contains(flag),
            "{command}"
        );
    }
}

#[test]
fn god_command_abbreviation_is_not_recognized() {
    // C `cmdcmp(ptr, "god", 3)` requires the full three-letter word.
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Caller"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/go Target", 1)
            .is_none()
    );
}

#[test]
fn god_global_command_dumps_every_setting_like_c() {
    // C `/global` (`command.c:8226-8322`), `cmdcmp(ptr, "global", 2)`,
    // `CF_GOD`-gated.
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    // Non-god callers are gated out entirely, exactly like the C `&&
    // (ch[cn].flags & CF_GOD)` guard.
    let mut player = login_character(CharacterId(8), &login_block("Player"), 1, 11, 10);
    player.flags.remove(CharacterFlags::GOD);
    world.add_character(player);
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, CharacterId(8), "/global", 1)
            .is_none()
    );

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/global", 1)
        .expect("god /global command should be recognized");

    // C `cmdcmp(ptr, "global", 2)` only requires a 2-letter prefix.
    let abbreviated = apply_admin_character_command(&mut world, &mut runtime, god_id, "/gl", 1)
        .expect("god /gl abbreviation should be recognized");
    assert_eq!(result.messages, abbreviated.messages);

    assert_eq!(result.messages.len(), 73);
    assert_eq!(result.messages[0], "=== Current Global Settings ===");
    assert_eq!(result.messages[1], "--- Core Server Settings ---");
    assert_eq!(
        result.messages[2],
        "Item decay time: 7200 ticks (5 minutes)"
    );
    assert_eq!(result.messages[9], "Sewer item respawn time: 24 hours");
    assert_eq!(result.messages[11], "Global EXP modifier: 1.00");
    assert_eq!(result.messages[16], "Holler distance: 75 tiles, Cost: 12");
    assert_eq!(result.messages[30], "Jail location: 186,234 (area 3)");
    assert_eq!(result.messages[34], "Maximum jewel count: 2");
    assert_eq!(result.messages[35], "Max clan bonus percent: 20%");
    assert!(result
        .messages
        .contains(&"--- Mine Settings ---".to_string()));
    assert!(result
        .messages
        .contains(&"Rare golem chance: 25".to_string()));
    assert!(result
        .messages
        .contains(&"--- Drop Probability Settings ---".to_string()));
    assert!(result
        .messages
        .contains(&"Drop probability (low level): 1700 - (default 1700)".to_string()));
    assert!(result
        .messages
        .contains(&"Drop probability (mid level): 800- (default 800)".to_string()));
    assert!(result
        .messages
        .contains(&"Drop probability (high level): 532- (default 532)".to_string()));

    // Changed settings are reflected live (read straight from
    // `world.settings`, not cached).
    world.settings.rare_golem_chance = 42;
    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/global", 1)
        .expect("god /global command should be recognized");
    assert!(result
        .messages
        .contains(&"Rare golem chance: 42".to_string()));
}

#[test]
fn showflags_requires_god_and_full_word() {
    // C `cmdcmp(ptr, "showflags", 9)`: `minlen == "showflags".len()`, so
    // no abbreviation is accepted.
    let mut world = World::default();
    let caller_id = CharacterId(1);
    let caller = login_character(caller_id, &login_block("Caller"), 1, 10, 10);
    world.add_character(caller);
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/showflags Caller",
        1
    )
    .is_none());
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/showflag Caller",
        1
    )
    .is_none());

    world.characters.get_mut(&caller_id).unwrap().flags |= CharacterFlags::GOD;
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/showflag Caller",
        1
    )
    .is_none());
}

#[test]
fn showflags_reports_no_one_by_that_name_for_an_unloaded_character() {
    let mut world = World::default();
    let god_id = CharacterId(1);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/showflags Nobodyhome", 1)
            .expect("showflags command should be recognized");
    assert_eq!(
        result.messages,
        vec!["Sorry, no one by the name Nobodyhome around.".to_string()]
    );
}

#[test]
fn showflags_lists_every_set_flag_in_legacy_declaration_order() {
    let mut world = World::default();
    let god_id = CharacterId(1);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let target_id = CharacterId(2);
    let mut target = login_character(target_id, &login_block("Target"), 1, 20, 20);
    // Set flags out of declaration order to prove the output is
    // re-sorted into C's fixed `if (flags & CF_X)` order, not insertion
    // order. `CF_SPY` is set too, to prove it is never reported (C never
    // checks it in `cmd_show_flags`).
    target.flags |= CharacterFlags::NOLEVEL
        | CharacterFlags::USED
        | CharacterFlags::PLAYER
        | CharacterFlags::SPY;
    world.add_character(target);
    let mut runtime = ServerRuntime::default();

    // Trailing non-alpha text after the name is ignored, matching C's
    // `isalpha`-only name scan.
    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/showflags Target99", 1)
            .expect("showflags command should be recognized");
    // `login_character` sets `CF_ALIVE` by default (living being), so it
    // shows up too, in its correct declaration-order slot.
    assert_eq!(
        result.messages,
        vec![
            "Flags for player Target:".to_string(),
            "USED".to_string(),
            "PLAYER".to_string(),
            "ALIVE".to_string(),
            "NOLEVEL".to_string(),
        ]
    );
}

#[test]
fn toggleflag_requires_god_and_full_word() {
    let mut world = World::default();
    let caller_id = CharacterId(1);
    let caller = login_character(caller_id, &login_block("Caller"), 1, 10, 10);
    world.add_character(caller);
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/toggleflag Caller NOEXP",
        1
    )
    .is_none());
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/toggleflagg Caller NOEXP",
        1
    )
    .is_none());
}

#[test]
fn toggleflag_reports_no_one_by_that_name_for_an_unloaded_character() {
    let mut world = World::default();
    let god_id = CharacterId(1);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/toggleflag Nobodyhome NOEXP",
        1,
    )
    .expect("toggleflag command should be recognized");
    assert_eq!(
        result.messages,
        vec!["Sorry, no one by the name Nobodyhome around.".to_string()]
    );
}

#[test]
fn toggleflag_reports_unknown_flag_and_leaves_flags_untouched() {
    let mut world = World::default();
    let god_id = CharacterId(1);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let target_id = CharacterId(2);
    let target = login_character(target_id, &login_block("Target"), 1, 20, 20);
    let before_flags = target.flags;
    world.add_character(target);
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/toggleflag Target NOTAREALFLAG",
        1,
    )
    .expect("toggleflag command should be recognized");
    assert_eq!(
        result.messages,
        vec!["Sorry, unknown flag: NOTAREALFLAG".to_string()]
    );
    assert_eq!(world.characters[&target_id].flags, before_flags);

    // C's flag-name token is a non-whitespace scan, not alpha-only, so a
    // missing argument yields an empty `flag_name` and the same
    // "unknown flag" message with a trailing empty name.
    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/toggleflag Target", 1)
            .expect("toggleflag command should be recognized");
    assert_eq!(result.messages, vec!["Sorry, unknown flag: ".to_string()]);
}

#[test]
fn toggleflag_toggles_named_flag_on_then_off_case_insensitively() {
    let mut world = World::default();
    let god_id = CharacterId(1);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let target_id = CharacterId(2);
    let target = login_character(target_id, &login_block("Target"), 1, 20, 20);
    assert!(!target.flags.contains(CharacterFlags::NOEXP));
    world.add_character(target);
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/toggleflag Target noexp",
        1,
    )
    .expect("toggleflag command should be recognized");
    assert_eq!(
        result.messages,
        vec!["Flag noexp turned ON for Target".to_string()]
    );
    assert!(world.characters[&target_id]
        .flags
        .contains(CharacterFlags::NOEXP));

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/toggleflag Target noexp",
        1,
    )
    .expect("toggleflag command should be recognized");
    assert_eq!(
        result.messages,
        vec!["Flag noexp turned OFF for Target".to_string()]
    );
    assert!(!world.characters[&target_id]
        .flags
        .contains(CharacterFlags::NOEXP));
}

/// Both caller and target get a live session, unlike
/// `setup_god_and_target_with_military_ppd`, since `/setrd`/`/clearrd`/
/// `/solverd` resend the quest log to the ACTING character's own session
/// (`sendquestlog(cn, ch[cn].player)` in C - see the port's doc comment).
fn setup_god_and_target_with_shrine_ppd(
    world: &mut World,
    runtime: &mut ServerRuntime,
) -> (CharacterId, CharacterId) {
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
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(90, commands, 0);
    runtime.players.get_mut(&90).unwrap().character_id = Some(god_id);
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(91, commands, 0);
    runtime.players.get_mut(&91).unwrap().character_id = Some(target_id);
    (god_id, target_id)
}

#[test]
fn god_setrd_command_sets_continuity_on_self_and_resends_questlog_to_caller() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/setrd 42", 1)
        .expect("god setrd should be recognized");
    assert_eq!(
        result.messages,
        vec!["Set continuity shrine for Godmode to RD 42."]
    );
    assert_eq!(
        runtime
            .player_for_character(god_id)
            .unwrap()
            .random_shrine_continuity,
        42
    );

    // C `sendquestlog(cn, ch[cn].player)` always targets the acting
    // character's own session.
    let payloads = runtime
        .tick_out
        .get(&90)
        .expect("caller session got the questlog resend");
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0][0], ugaris_protocol::packet::SV_QUESTLOG);
    assert!(runtime.tick_out.get(&91).is_none());
}

#[test]
fn god_setrd_command_sets_continuity_on_named_target() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setrd Target 17", 1)
            .expect("god setrd should be recognized");
    assert_eq!(
        result.messages,
        vec!["Set continuity shrine for Target to RD 17."]
    );
    assert_eq!(
        runtime
            .player_for_character(target_id)
            .unwrap()
            .random_shrine_continuity,
        17
    );
    // The caller's own continuity is untouched.
    assert_eq!(
        runtime
            .player_for_character(god_id)
            .unwrap()
            .random_shrine_continuity,
        0
    );
}

#[test]
fn setrd_rejects_rd_number_out_of_10_to_99_range() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    for command in ["/setrd 9", "/setrd 100", "/setrd Target 5"] {
        let result = apply_admin_character_command(&mut world, &mut runtime, god_id, command, 1)
            .expect("setrd should be recognized even with an invalid rd number");
        assert_eq!(
            result.messages,
            vec!["RD number must be between 10 and 99."]
        );
    }
}

#[test]
fn setrd_reports_unknown_name() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setrd Nobody 42", 1)
            .expect("setrd should be recognized");
    assert_eq!(
        result.messages,
        vec!["Sorry, no one by the name Nobody around."]
    );
}

#[test]
fn setrd_reports_failed_player_data_when_target_has_no_live_session() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let offline_id = CharacterId(9);
    world.add_character(login_character(
        offline_id,
        &login_block("Offline"),
        1,
        12,
        10,
    ));
    let mut runtime = ServerRuntime::default();
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(90, commands, 0);
    runtime.players.get_mut(&90).unwrap().character_id = Some(god_id);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setrd Offline 42", 1)
            .expect("setrd should be recognized");
    assert_eq!(result.messages, vec!["Failed to get player data."]);
}

#[test]
fn setrd_is_god_only() {
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
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/setrd 42", 1)
            .is_none()
    );
}

#[test]
fn god_clearrd_command_clears_all_ten_shrines_for_the_rd_level() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    // RD 12 covers shrine indices (12-10)*10..(12-10)*10+10 = 20..30.
    {
        let target_player = runtime.player_for_character_mut(target_id).unwrap();
        for shrine in 20..30u8 {
            target_player.mark_random_shrine_used(shrine);
        }
        // A neighboring RD level's shrine must survive untouched.
        target_player.mark_random_shrine_used(30);
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/clearrd Target 12", 1)
            .expect("god clearrd should be recognized");
    assert_eq!(
        result.messages,
        vec!["Cleared all used shrines for Target in RD 12."]
    );

    let target_player = runtime.player_for_character(target_id).unwrap();
    for shrine in 20..30u8 {
        assert!(!target_player.has_used_random_shrine(shrine));
    }
    assert!(target_player.has_used_random_shrine(30));
}

#[test]
fn god_solverd_command_marks_all_but_the_continuity_shrine_used() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/solverd Target 12", 1)
            .expect("god solverd should be recognized");
    assert_eq!(
        result.messages,
        vec!["Marked all non-continuity shrines as used for Target in RD 12."]
    );

    let target_player = runtime.player_for_character(target_id).unwrap();
    // Shrine indices 20..29 (i = 0..9) get marked; index 29 (i == 9, the
    // continuity shrine) is deliberately skipped.
    for shrine in 20..29u8 {
        assert!(target_player.has_used_random_shrine(shrine));
    }
    assert!(!target_player.has_used_random_shrine(29));
}

#[test]
fn god_changetunnel_command_sets_named_target_clevel_and_notifies_them() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/changetunnel Target 42",
        1,
    )
    .expect("god changetunnel should be recognized");
    assert_eq!(result.messages, vec!["Set Target's tunnel level to 42."]);
    assert_eq!(
        result.other_messages,
        vec![(
            target_id,
            "Your tunnel level has been set to 42 by a god.".to_string()
        )]
    );
    assert_eq!(
        runtime
            .player_for_character(target_id)
            .unwrap()
            .tunnel_clevel(),
        42
    );
    // The caller's own tunnel state is untouched.
    assert_eq!(
        runtime
            .player_for_character(god_id)
            .unwrap()
            .tunnel_clevel(),
        0
    );
}

#[test]
fn changetunnel_rejects_out_of_range_level_and_unknown_name() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    let missing = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/changetunnel Ghost 42",
        1,
    )
    .expect("changetunnel should be recognized even for a missing target");
    assert_eq!(
        missing.messages,
        vec!["Sorry, no one by the name Ghost around."]
    );

    for command in ["/changetunnel Target 9", "/changetunnel Target 201"] {
        let result = apply_admin_character_command(&mut world, &mut runtime, god_id, command, 1)
            .expect("changetunnel should be recognized even with an invalid level");
        assert_eq!(
            result.messages,
            vec!["Invalid tunnel level. Must be between 10 and 200."]
        );
    }
}

#[test]
fn changetunnel_is_god_only() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        target_id,
        "/changetunnel Target 42",
        1
    )
    .is_none());
}

#[test]
fn god_settunnel_command_sets_completed_amount_for_level() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/settunnel Target 15 7",
        1,
    )
    .expect("god settunnel should be recognized");
    assert_eq!(
        result.messages,
        vec!["Set Target's completed amount for tunnel level 15 to 7."]
    );
    assert_eq!(
        result.other_messages,
        vec![(
            target_id,
            "Your completed amount for tunnel level 15 has been set to 7 by a god.".to_string()
        )]
    );
    assert_eq!(
        runtime
            .player_for_character(target_id)
            .unwrap()
            .tunnel_used(15),
        7
    );
}

#[test]
fn god_cleartunnel_command_clears_completed_amount_for_level() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);
    runtime
        .player_for_character_mut(target_id)
        .unwrap()
        .set_tunnel_used(15, 7);

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/cleartunnel Target 15",
        1,
    )
    .expect("god cleartunnel should be recognized");
    assert_eq!(
        result.messages,
        vec!["Cleared Target's completed amount for tunnel level 15."]
    );
    assert_eq!(
        result.other_messages,
        vec![(
            target_id,
            "Your completed amount for tunnel level 15 has been cleared by a god.".to_string()
        )]
    );
    assert_eq!(
        runtime
            .player_for_character(target_id)
            .unwrap()
            .tunnel_used(15),
        0
    );
}

#[test]
fn changetunnel_command_on_self_sends_no_other_message() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/changetunnel Godmode 30",
        1,
    )
    .expect("god changetunnel on self should be recognized");
    assert_eq!(result.messages, vec!["Set Godmode's tunnel level to 30."]);
    assert!(result.other_messages.is_empty());
}

#[test]
fn god_solvetunnel_command_reports_reward_kind_without_mutating_state() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    let exp = apply_admin_character_command(&mut world, &mut runtime, god_id, "/solvetunnel 0", 1)
        .expect("god solvetunnel should be recognized");
    assert_eq!(
        exp.messages,
        vec!["Solved current tunnel and granted experience reward."]
    );

    let mil = apply_admin_character_command(&mut world, &mut runtime, god_id, "/solvetunnel 1", 1)
        .expect("god solvetunnel should be recognized");
    assert_eq!(
        mil.messages,
        vec!["Solved current tunnel and granted military experience reward."]
    );

    let invalid =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/solvetunnel 2", 1)
            .expect("god solvetunnel should be recognized even with an invalid type");
    assert_eq!(
        invalid.messages,
        vec!["Invalid exp type. Must be 0 (exp) or 1 (military exp)."]
    );
}

#[test]
fn solvetunnel_is_god_only() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        target_id,
        "/solvetunnel 0",
        1
    )
    .is_none());
}

fn setup_god_and_online_target(
    world: &mut World,
    runtime: &mut ServerRuntime,
) -> (CharacterId, CharacterId) {
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
    let mut target_player = PlayerRuntime::connected(80, 0);
    target_player.character_id = Some(target_id);
    runtime.players.insert(80, target_player);
    (god_id, target_id)
}

// C `command.c:1136-1360`/`10416-10465`: the `/pentinfo`, `/setpentcount`,
// `/setpentstatus`, `/setpentbonus` and `/resetpent` GOD debug commands
// over the `DRD_PENT_NPPD` scratch struct (`PlayerRuntime::pentagram_
// debug`).

#[test]
fn pent_debug_commands_are_god_only() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    for command in [
        "/pentinfo Target",
        "/setpentcount Target 3",
        "/setpentstatus Target 1",
        "/setpentbonus Target 100",
        "/resetpent Target",
    ] {
        assert!(
            apply_admin_character_command(&mut world, &mut runtime, target_id, command, 1)
                .is_none(),
            "{command} should be GOD-gated"
        );
    }
}

#[test]
fn pentinfo_requires_a_player_name_and_reports_unknown_players() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let usage = apply_admin_character_command(&mut world, &mut runtime, god_id, "/pentinfo", 1)
        .expect("god pentinfo should be recognized");
    assert_eq!(usage.messages, vec!["Usage: /pentinfo <player>"]);

    let missing =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/pentinfo Nobody", 1)
            .expect("god pentinfo missing target should be handled");
    assert_eq!(missing.messages, vec!["Player 'Nobody' not found online."]);
}

#[test]
fn pentinfo_shows_empty_data_then_active_pentagrams_after_mutation() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let empty =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/pentinfo Target", 1)
            .expect("god pentinfo should be recognized");
    assert_eq!(
        empty.messages,
        vec![
            "=== Pentagram Data for Target ===",
            "Status: 0 (0=normal, 1=5-of-color)",
            "Pent Count: 0 (current run)",
            "Lucky Pents: 0 (this solve)",
            "Bonus: 0 exp",
            "Active Pentagrams: 0/6",
        ]
    );

    {
        let player = runtime.player_for_character_mut(target_id).unwrap();
        player.pentagram_debug.pent_it[0] = 42;
        player.pentagram_debug.pent_color[0] = 2;
        player.pentagram_debug.pent_value[0] = 5;
        player.pentagram_debug.pent_worth[0] = 100;
        player.pentagram_debug.pent_it[3] = 7;
        player.pentagram_debug.pent_color[3] = 9; // out-of-range -> "?"
        player.pentagram_debug.pent_value[3] = 1;
        player.pentagram_debug.pent_worth[3] = 2;
    }

    let filled =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/pentinfo Target", 1)
            .expect("god pentinfo should be recognized");
    assert_eq!(
        filled.messages,
        vec![
            "=== Pentagram Data for Target ===",
            "Status: 0 (0=normal, 1=5-of-color)",
            "Pent Count: 0 (current run)",
            "Lucky Pents: 0 (this solve)",
            "Bonus: 0 exp",
            "Active Pentagrams: 2/6",
            "  [0] color=green value=5 worth=100",
            "  [3] color=? value=1 worth=2",
        ]
    );
}

#[test]
fn setpentcount_setpentstatus_setpentbonus_mutate_the_named_targets_data() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let count = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setpentcount Target 3",
        1,
    )
    .expect("god setpentcount should be recognized");
    assert_eq!(count.messages, vec!["Set pent_cnt for Target: 0 -> 3"]);
    assert_eq!(
        runtime
            .player_for_character(target_id)
            .unwrap()
            .pentagram_debug
            .pent_cnt,
        3
    );

    let status = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setpentstatus Target 1",
        1,
    )
    .expect("god setpentstatus should be recognized");
    assert_eq!(status.messages, vec!["Set pent status for Target: 0 -> 1"]);
    assert_eq!(
        runtime
            .player_for_character(target_id)
            .unwrap()
            .pentagram_debug
            .status,
        1
    );

    let bonus = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setpentbonus Target -50",
        1,
    )
    .expect("god setpentbonus should be recognized");
    assert_eq!(bonus.messages, vec!["Set pent bonus for Target: 0 -> -50"]);
    assert_eq!(
        runtime
            .player_for_character(target_id)
            .unwrap()
            .pentagram_debug
            .bonus,
        -50
    );
}

#[test]
fn setpentcount_requires_both_a_name_and_an_integer_value() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    for command in [
        "/setpentcount",
        "/setpentcount Target",
        "/setpentcount Target abc",
    ] {
        let result = apply_admin_character_command(&mut world, &mut runtime, god_id, command, 1)
            .expect("god setpentcount should always be recognized");
        assert_eq!(
            result.messages,
            vec!["Usage: /setpentcount <player> <count>"],
            "{command} should report usage"
        );
    }

    let missing = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setpentcount Nobody 3",
        1,
    )
    .expect("god setpentcount missing target should be handled");
    assert_eq!(missing.messages, vec!["Player 'Nobody' not found online."]);
}

#[test]
fn resetpent_requires_a_name_and_zeroes_every_field() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let usage = apply_admin_character_command(&mut world, &mut runtime, god_id, "/resetpent", 1)
        .expect("god resetpent should be recognized");
    assert_eq!(usage.messages, vec!["Usage: /resetpent <player>"]);

    let missing =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/resetpent Nobody", 1)
            .expect("god resetpent missing target should be handled");
    assert_eq!(missing.messages, vec!["Player 'Nobody' not found online."]);

    {
        let player = runtime.player_for_character_mut(target_id).unwrap();
        player.pentagram_debug.pent_cnt = 5;
        player.pentagram_debug.status = 1;
        player.pentagram_debug.bonus = 200;
        player.pentagram_debug.pent_it[0] = 1;
    }

    let reset =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/resetpent Target", 1)
            .expect("god resetpent should be recognized");
    assert_eq!(reset.messages, vec!["Reset all pentagram data for Target."]);
    assert_eq!(
        runtime
            .player_for_character(target_id)
            .unwrap()
            .pentagram_debug,
        PentagramDebugData::default()
    );
}

#[test]
fn pent_debug_commands_report_missing_runtime_for_online_character() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let god_id = CharacterId(7);
    let target_id = CharacterId(8);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    // Target exists in `world.characters` but has no `PlayerRuntime`
    // (never actually connected), matching C's "found in `ch[]` but
    // `set_data` fails" edge case.
    world.add_character(login_character(
        target_id,
        &login_block("Target"),
        1,
        11,
        10,
    ));

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/pentinfo Target", 1)
            .expect("god pentinfo should be recognized even without a runtime");
    assert_eq!(
        result.messages,
        vec!["Could not access pent data for Target."]
    );
}

// C `command.c:9049-9057`/`3163-3192` (`/noarch`) and `command.c:9226-9235`
// (`/noprof`).

#[test]
fn noarch_and_noprof_are_god_only() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    for command in ["/noarch Target", "/noprof"] {
        assert!(
            apply_admin_character_command(&mut world, &mut runtime, target_id, command, 1)
                .is_none(),
            "{command} should be GOD-gated"
        );
    }
}

#[test]
fn noarch_reports_no_one_by_that_name_and_sends_no_other_message() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let missing =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/noarch Nobody", 1)
            .expect("god noarch should be recognized");
    assert_eq!(
        missing.messages,
        vec!["Sorry, no one by the name Nobody around."]
    );

    // A bare `/noarch` with no name at all resolves an empty-string
    // lookup, which never matches any real character - C's own
    // `log_char` format string has a literal space before `%s`, so an
    // empty name produces a visible double space.
    let no_name = apply_admin_character_command(&mut world, &mut runtime, god_id, "/noarch", 1)
        .expect("god noarch should be recognized even with no argument");
    assert_eq!(no_name.messages, vec!["Sorry, no one by the name  around."]);
}

#[test]
fn noarch_caps_values_up_to_immunity_and_clears_arch_flag_with_no_confirmation() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    {
        let target = world.characters.get_mut(&target_id).unwrap();
        target.flags.insert(CharacterFlags::ARCH);
        for value in target.values[1].iter_mut() {
            *value = 100;
        }
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/noarch Target", 1)
            .expect("god noarch should be recognized");
    // C sends no confirmation message on success at all.
    assert!(result.messages.is_empty());

    let target = world.characters.get(&target_id).unwrap();
    assert!(!target.flags.contains(CharacterFlags::ARCH));
    for n in 0..=CharacterValue::Immunity as usize {
        assert_eq!(target.values[1][n], 50, "value index {n} should be capped");
    }
    // Everything past V_IMMUNITY is left untouched (C's loop is
    // `n <= V_IMMUNITY`, not the full array).
    for n in (CharacterValue::Immunity as usize + 1)..CHARACTER_VALUE_NAMES.len() {
        assert_eq!(
            target.values[1][n], 100,
            "value index {n} should be untouched"
        );
    }
}

#[test]
fn noarch_does_not_lower_values_already_at_or_below_the_cap() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    {
        let target = world.characters.get_mut(&target_id).unwrap();
        target.values[1][CharacterValue::Hp as usize] = 20;
    }

    apply_admin_character_command(&mut world, &mut runtime, god_id, "/noarch Target", 1)
        .expect("god noarch should be recognized");

    let target = world.characters.get(&target_id).unwrap();
    assert_eq!(target.values[1][CharacterValue::Hp as usize], 20);
}

#[test]
fn noprof_zeroes_the_callers_own_professions_only_with_no_confirmation() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    {
        let god = world.characters.get_mut(&god_id).unwrap();
        for profession in god.professions.iter_mut() {
            *profession = 15;
        }
    }
    {
        let target = world.characters.get_mut(&target_id).unwrap();
        for profession in target.professions.iter_mut() {
            *profession = 15;
        }
    }

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/noprof", 1)
        .expect("god noprof should be recognized");
    // C sends no confirmation message on success at all.
    assert!(result.messages.is_empty());

    let god = world.characters.get(&god_id).unwrap();
    assert!(god.professions.iter().all(|&value| value == 0));
    // Unlike `/noarch`, `/noprof` never resolves a target name - it always
    // acts on the caller, so an online bystander's own professions are
    // left completely untouched.
    let target = world.characters.get(&target_id).unwrap();
    assert!(target.professions.iter().all(|&value| value == 15));
}

#[test]
fn noprof_ignores_any_trailing_argument_text() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    {
        let god = world.characters.get_mut(&god_id).unwrap();
        god.professions[0] = 7;
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/noprof Target", 1)
            .expect("god noprof should be recognized even with trailing text");
    assert!(result.messages.is_empty());
    let god = world.characters.get(&god_id).unwrap();
    assert!(god.professions.iter().all(|&value| value == 0));
}

// C `command.c:9058-9066`/`3194-3218` (`/fixit`) and `command.c:9067-
// 9075`/`3221-3251` (`/questfix`).

#[test]
fn fixit_and_questfix_are_god_only() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    for command in ["/fixit Target", "/questfix Target"] {
        assert!(
            apply_admin_character_command(&mut world, &mut runtime, target_id, command, 1)
                .is_none(),
            "{command} should be GOD-gated"
        );
    }
}

#[test]
fn fixit_reports_no_one_by_that_name_when_target_is_offline() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let missing =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/fixit Nobody", 1)
            .expect("god fixit should be recognized");
    assert_eq!(
        missing.messages,
        vec!["Sorry, no one by the name Nobody around."]
    );
}

#[test]
fn fixit_wipes_and_reinitializes_the_targets_own_quest_log_with_no_confirmation() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    {
        let target_player = runtime.player_for_character_mut(target_id).unwrap();
        // Simulate a corrupted/stale quest log: already "initialized"
        // (sentinel set) but with a bogus entry that a fresh derive
        // would never produce.
        target_player.quest_log.mark_init_complete();
        target_player.quest_log.set_raw(0, 63, 3);
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/fixit Target", 1)
            .expect("god fixit should be recognized");
    // C sends no confirmation message to the caller at all.
    assert!(result.messages.is_empty());

    let target_player = runtime.player_for_character(target_id).unwrap();
    // The bogus entry is gone (wiped, then re-derived from scratch) and
    // the log is freshly marked complete again (re-init actually ran).
    assert_ne!(target_player.quest_log.entries()[0].done, 63);
    assert!(target_player.quest_log.is_init_complete());
}

#[test]
fn questfix_reports_no_one_by_that_name_when_target_is_offline() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let missing =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/questfix Nobody", 1)
            .expect("god questfix should be recognized");
    assert_eq!(
        missing.messages,
        vec!["Sorry, no one by the name Nobody around."]
    );
}

#[test]
fn questfix_clears_the_callers_own_sentinel_and_leaves_the_named_targets_log_untouched() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    // Give the calling GOD their own connected PlayerRuntime too (C's
    // real bug operates on `cn`, the caller, not the named target `co`).
    let mut god_player = PlayerRuntime::connected(90, 0);
    god_player.character_id = Some(god_id);
    god_player.quest_log.mark_init_complete();
    runtime.players.insert(90, god_player);
    {
        let target_player = runtime.player_for_character_mut(target_id).unwrap();
        target_player.quest_log.mark_init_complete();
        target_player.quest_log.set_raw(0, 5, 3);
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/questfix Target", 1)
            .expect("god questfix should be recognized");
    assert!(result.messages.is_empty());

    // The caller's own sentinel was cleared (marked for full re-derive on
    // next login) even though the command targeted "Target".
    assert!(!runtime
        .player_for_character(god_id)
        .unwrap()
        .quest_log
        .is_init_complete());
    // The named target's quest log is completely untouched - C's bug
    // means `questlog_init(co)` is a no-op since `co`'s sentinel was
    // already set.
    let target_player = runtime.player_for_character(target_id).unwrap();
    assert!(target_player.quest_log.is_init_complete());
    assert_eq!(target_player.quest_log.entries()[0].done, 5);
}

// C `/clearppd <ppdname> [player]` (`command.c:10144-10146` dispatch,
// `CF_GOD | CF_STAFF`-gated; `cmd_clearppd`, `command.c:4214-4288`).

#[test]
fn clearppd_requires_god_or_staff() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        target_id,
        "/clearppd keyring",
        1
    )
    .is_none());
}

/// Registers a connected `PlayerRuntime` for `character_id` on a fresh
/// session, so self-target `/clearppd` calls (whose caller is also the
/// target) have somewhere to read/write PPD fields.
fn insert_runtime_for(runtime: &mut ServerRuntime, session_id: u64, character_id: CharacterId) {
    let mut player = PlayerRuntime::connected(session_id, 0);
    player.character_id = Some(character_id);
    runtime.players.insert(session_id, player);
}

#[test]
fn clearppd_staff_without_god_is_accepted() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    insert_runtime_for(&mut runtime, 90, god_id);
    // Demote the caller to STAFF-only, matching C's `CF_GOD | CF_STAFF`
    // gate accepting either flag.
    {
        let god = world.characters.get_mut(&god_id).unwrap();
        god.flags.remove(CharacterFlags::GOD);
        god.flags.insert(CharacterFlags::STAFF);
    }
    let _ = target_id;

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/clearppd keyring", 1)
            .expect("STAFF-only caller should still be recognized");
    assert_eq!(result.messages, vec!["No keyring PPD found for Godmode."]);
}

#[test]
fn clearppd_with_no_arguments_shows_usage() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/clearppd", 1)
        .expect("god clearppd should be recognized");
    assert_eq!(
        result.messages,
        vec![
            "Usage: #clearppd <ppdname> [player]",
            "Available PPDs: keyring, questlog, alias"
        ]
    );
}

#[test]
fn clearppd_rejects_unknown_ppd_name() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/clearppd bogus", 1)
            .expect("god clearppd should be recognized");
    assert_eq!(
        result.messages,
        vec![
            "Unknown PPD: bogus",
            "Available PPDs: keyring, questlog, alias"
        ]
    );
}

#[test]
fn clearppd_reports_player_not_found_with_its_own_distinct_message() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/clearppd keyring Nobody",
        1,
    )
    .expect("god clearppd should be recognized");
    // Deliberately NOT "Sorry, no one by the name %s around." - C's
    // `cmd_clearppd` uses its own distinct wording.
    assert_eq!(result.messages, vec!["Player 'Nobody' not found."]);
}

#[test]
fn clearppd_keyring_reports_not_found_when_already_empty_and_clears_when_populated() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    insert_runtime_for(&mut runtime, 90, god_id);

    // Empty keyring (default) -> "No ... PPD found".
    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/clearppd keyring", 1)
            .expect("god clearppd should be recognized");
    assert_eq!(result.messages, vec!["No keyring PPD found for Godmode."]);

    // Populate it, then clear for real.
    {
        let god_player = runtime.player_for_character_mut(god_id).unwrap();
        god_player.keyring.push(ugaris_core::player::KeyringEntry {
            template_id: 1,
            name: "Test Key".to_string(),
            description: String::new(),
            sprite: 0,
            flags: 0,
            value: 0,
            driver: 0,
            driver_data: Vec::new(),
            expire_serial: 0,
        });
    }
    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/clearppd keyring", 1)
            .expect("god clearppd should be recognized");
    assert_eq!(result.messages, vec!["Cleared keyring PPD for Godmode."]);
    assert!(runtime
        .player_for_character(god_id)
        .unwrap()
        .keyring
        .is_empty());
}

#[test]
fn clearppd_targets_a_named_player_and_notifies_both_sides() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    {
        let target_player = runtime.player_for_character_mut(target_id).unwrap();
        target_player
            .aliases
            .push(ugaris_core::player::CommandAlias {
                from: "gg".to_string(),
                to: "grin".to_string(),
            });
    }

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/clearppd alias Target",
        1,
    )
    .expect("god clearppd should be recognized");
    assert_eq!(result.messages, vec!["Cleared alias PPD for Target."]);
    assert_eq!(
        result.other_messages,
        vec![(
            target_id,
            "Your alias data has been cleared by Godmode.".to_string()
        )]
    );
    assert!(runtime
        .player_for_character(target_id)
        .unwrap()
        .aliases
        .is_empty());
}

#[test]
fn clearppd_self_target_sends_no_other_message() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    insert_runtime_for(&mut runtime, 90, god_id);

    {
        let god_player = runtime.player_for_character_mut(god_id).unwrap();
        god_player.aliases.push(ugaris_core::player::CommandAlias {
            from: "gg".to_string(),
            to: "grin".to_string(),
        });
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/clearppd alias", 1)
            .expect("god clearppd should be recognized");
    assert_eq!(result.messages, vec!["Cleared alias PPD for Godmode."]);
    assert!(result.other_messages.is_empty());
}

#[test]
fn clearppd_questlog_clears_and_reports_success() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    insert_runtime_for(&mut runtime, 90, god_id);

    {
        let god_player = runtime.player_for_character_mut(god_id).unwrap();
        god_player.quest_log.set_raw(0, 1, 1);
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/clearppd questlog", 1)
            .expect("god clearppd should be recognized");
    assert_eq!(result.messages, vec!["Cleared questlog PPD for Godmode."]);
    assert!(runtime
        .player_for_character(god_id)
        .unwrap()
        .quest_log
        .is_empty());
}

#[test]
fn clearppd_only_matches_online_player_flagged_characters() {
    // A non-CF_PLAYER character sharing the target name must not match
    // (C's search loop skips any `co` without `CF_PLAYER`).
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    {
        let target = world.characters.get_mut(&target_id).unwrap();
        target.flags.remove(CharacterFlags::PLAYER);
    }

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/clearppd keyring Target",
        1,
    )
    .expect("god clearppd should be recognized");
    assert_eq!(result.messages, vec!["Player 'Target' not found."]);
}
