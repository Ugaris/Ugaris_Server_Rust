// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use super::*;

#[test]
pub(crate) fn god_setxmas_command_sets_runtime_christmas_override() {
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
    assert!(runtime_effective_xmas_event(&runtime).0);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/setxmas 0", 1)
            .expect("god setxmas command should accept zero");
    assert_eq!(
        result.messages,
        vec!["Setting christmas special to 0, old value was 1."]
    );
    assert_eq!(runtime.xmas_special_override, Some(0));
    assert!(!runtime_effective_xmas_event(&runtime).0);
}

#[test]
pub(crate) fn god_prof_command_reports_empty_profile_boundary_like_c() {
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
pub(crate) fn god_profinfo_command_reports_header_only_like_c() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    // C's `show_prof()` body is entirely console-only `xlog`, so the
    // player only ever sees the one header line - this is not a stub,
    // it's the real C player-facing behavior.
    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/profinfo", 1)
        .expect("legacy cmdcmp accepts the full profinfo word");
    assert_eq!(result.messages, vec!["Profiling Information:"]);

    // minlen 5: "profi" is the shortest accepted abbreviation.
    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/profi", 1)
        .expect("legacy cmdcmp accepts prefix length five");
    assert_eq!(result.messages, vec!["Profiling Information:"]);

    // Shorter than minlen 5 doesn't reach `profinfo` at all (nor `prof`,
    // which requires the literal 4-letter word as its own prefix).
    assert!(apply_admin_character_command(&mut world, &mut runtime, god_id, "/prof", 1).is_some());
    let short = apply_admin_character_command(&mut world, &mut runtime, god_id, "/prof", 1)
        .expect("prof itself is still its own separate command");
    assert_eq!(short.messages, vec!["--- Profile ---", "---------------"]);

    let mortal_id = CharacterId(8);
    world.add_character(login_character(
        mortal_id,
        &login_block("Mortal"),
        1,
        11,
        10,
    ));
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, mortal_id, "/profinfo", 1)
            .is_none()
    );
}

#[test]
pub(crate) fn god_poolstats_command_reports_header_only_like_c() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    // C's `log_connection_pool_state()` body is entirely console-only
    // `xlog`, so the player only ever sees the one header line.
    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/poolstats", 1)
        .expect("legacy cmdcmp accepts the full poolstats word");
    assert_eq!(result.messages, vec!["Connection Pool Statistics:"]);

    // minlen 5: "pools" is the shortest accepted abbreviation.
    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/pools", 1)
        .expect("legacy cmdcmp accepts prefix length five");
    assert_eq!(result.messages, vec!["Connection Pool Statistics:"]);
    assert!(apply_admin_character_command(&mut world, &mut runtime, god_id, "/pool", 1).is_none());

    let mortal_id = CharacterId(8);
    world.add_character(login_character(
        mortal_id,
        &login_block("Mortal"),
        1,
        11,
        10,
    ));
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, mortal_id, "/poolstats", 1)
            .is_none()
    );
}

#[test]
pub(crate) fn god_memstats_command_reports_live_world_counts() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    world.add_character(login_character(
        CharacterId(8),
        &login_block("Bystander"),
        1,
        11,
        10,
    ));

    world.add_item(Item {
        id: ItemId(1),
        name: "Loose Item".to_string(),
        description: String::new(),
        flags: ItemFlags::empty(),
        sprite: 0,
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
        carried_by: None,
        contained_in: None,
        content_id: 0,
        driver: 0,
        driver_data: vec![0; 40],
        serial: 1,
    });
    world.add_item(Item {
        id: ItemId(2),
        name: "A Sack".to_string(),
        description: String::new(),
        flags: ItemFlags::empty(),
        sprite: 0,
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
        carried_by: None,
        contained_in: None,
        content_id: 42,
        driver: 0,
        driver_data: vec![0; 40],
        serial: 2,
    });
    world.effects.insert(1, Effect::new(EF_BURN, 1, 0, 100));
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/memstats", 1)
        .expect("legacy cmdcmp accepts the full memstats word");
    assert_eq!(
        result.messages,
        vec![
            "Memory Usage Statistics:",
            "Total memory usage: 0 KB",
            "Characters: 2 used",
            "Items: 2 used",
            "Effects: 1 used",
            "Containers: 1 used",
            "Messages: 0 used",
        ]
    );

    // minlen 5: "memst" is the shortest accepted abbreviation.
    assert!(apply_admin_character_command(&mut world, &mut runtime, god_id, "/memst", 1).is_some());
    assert!(apply_admin_character_command(&mut world, &mut runtime, god_id, "/mems", 1).is_none());

    let mortal_id = CharacterId(9);
    world.add_character(login_character(
        mortal_id,
        &login_block("Mortal"),
        1,
        12,
        10,
    ));
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, mortal_id, "/memstats", 1)
            .is_none()
    );
}

#[test]
pub(crate) fn god_staffcode_command_sets_runtime_code_with_legacy_parsing() {
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
pub(crate) fn god_reset_command_clamps_target_values_like_c() {
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
pub(crate) fn reset_command_is_god_only_and_reports_missing_target_like_c() {
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
pub(crate) fn god_resetgift_clears_xmas_tree_area_bit_with_legacy_feedback() {
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
pub(crate) fn resetgift_is_god_only_checks_target_and_area() {
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
pub(crate) fn god_questlog_lists_flagged_quests_like_c() {
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
pub(crate) fn questlog_is_god_only_and_reports_missing_data_like_c() {
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
pub(crate) fn dlight_and_showattack_are_god_only_and_keep_full_dlight_minlen() {
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
pub(crate) fn god_sprite_command_sets_character_sprite_silently() {
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
pub(crate) fn sprite_command_is_god_only_and_requires_full_name() {
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
