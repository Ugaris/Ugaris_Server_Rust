use super::*;

#[test]
pub(crate) fn god_itemname_and_itemdesc_mutate_cursor_item_with_look_feedback() {
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
pub(crate) fn god_listitem_reports_legacy_item_details() {
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
pub(crate) fn listitem_is_god_only_and_reports_invalid_ids() {
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
pub(crate) fn god_setkarma_mutates_online_target_with_legacy_feedback() {
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
pub(crate) fn setkarma_is_god_only_and_reports_missing_target_like_c() {
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
pub(crate) fn god_sethardcore_bonus_commands_match_legacy_ranges_and_feedback() {
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
pub(crate) fn hardcore_bonus_commands_are_god_only_and_full_command_only() {
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
pub(crate) fn god_setspecialdropmult_truncates_old_value_like_c() {
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
pub(crate) fn god_setlootmod_command_validates_and_stores_modifier() {
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
pub(crate) fn god_reloadloot_command_clears_and_rescans_from_disk() {
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
pub(crate) fn god_listchars_reports_active_players_and_npcs_like_c() {
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
pub(crate) fn listchars_is_god_only_and_rejects_too_short_prefix() {
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
pub(crate) fn clearmerchantstores_rejects_non_merchant_and_missing_ids() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);

    let non_merchant_id = CharacterId(8);
    world.add_character(login_character(
        non_merchant_id,
        &login_block("Player"),
        1,
        11,
        10,
    ));

    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/clearmerchantstores 8",
        1,
    )
    .expect("clearmerchantstores should still respond for a non-merchant target");
    assert_eq!(
        result.messages,
        vec!["Invalid merchant ID or not a merchant character"]
    );
    assert!(result.clear_merchant_store_requested.is_none());

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/clearmerchantstores 999",
        1,
    )
    .expect("clearmerchantstores should still respond for an unknown target");
    assert_eq!(
        result.messages,
        vec!["Invalid merchant ID or not a merchant character"]
    );
}

#[test]
pub(crate) fn clearmerchantstores_is_god_only_and_rejects_too_short_prefix() {
    use ugaris_core::character_driver::CDR_MERCHANT;

    let mut world = World::default();
    let character_id = CharacterId(7);
    world.add_character(login_character(
        character_id,
        &login_block("Tester"),
        1,
        10,
        10,
    ));
    let merchant_id = CharacterId(20);
    let mut merchant = login_character(merchant_id, &login_block("Dolf"), 1, 11, 10);
    merchant.driver = CDR_MERCHANT;
    world.add_character(merchant);
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/clearmerchantstores 20",
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
        "/clearmerc 20",
        1,
    )
    .is_none());
    assert!(world.merchant_stores.get(&merchant_id).is_none());
}

#[test]
pub(crate) fn god_checksanity_reports_zero_errors_on_a_clean_world() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);

    let mut runtime = ServerRuntime::default();
    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/checksanity", 1)
        .expect("god checksanity should be recognized");

    assert_eq!(
        result.messages,
        vec![
            "Running consistency checks...",
            "Item errors: 0",
            "Map errors: 0",
            "Character errors: 0",
            "Container errors: 0",
            "Consistency check complete",
        ]
    );
}

#[test]
pub(crate) fn god_checksanity_repairs_a_dangling_carried_item_and_reports_the_count() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);

    // A dangling item claims to be carried by a character that doesn't
    // exist (the same class of bug C's `consistency_check_items` fixes).
    let mut dangling = test_item(ItemId(900), 1234, ItemFlags::TAKE);
    dangling.carried_by = Some(CharacterId(999));
    world.add_item(dangling);

    let mut runtime = ServerRuntime::default();
    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/checksanity", 1)
        .expect("god checksanity should be recognized");

    assert_eq!(
        result.messages,
        vec![
            "Running consistency checks...",
            "Item errors: 1",
            "Map errors: 0",
            "Character errors: 0",
            "Container errors: 0",
            "Consistency check complete",
        ]
    );
    assert_eq!(world.items.get(&ItemId(900)).unwrap().carried_by, None);
}

#[test]
pub(crate) fn checksanity_is_god_only_and_rejects_too_short_prefix() {
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
        "/checksanity",
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
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/chec", 1,)
            .is_none()
    );
}

#[test]
pub(crate) fn itemname_and_itemdesc_are_god_only_and_require_cursor_item() {
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
pub(crate) fn labsolved_command_is_god_only_and_supports_the_8_char_prefix() {
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
pub(crate) fn labsolved_command_toggles_bits_and_lists_solved_labs_for_self() {
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
pub(crate) fn labsolved_command_reports_missing_runtime_for_online_character() {
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
