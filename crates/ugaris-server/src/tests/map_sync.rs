use super::*;

#[test]
fn who_command_lists_visible_players_like_legacy_command() {
    let mut world = World::default();
    let mut warrior = login_character(CharacterId(7), &login_block("Warrior"), 1, 10, 10);
    warrior.level = 12;
    warrior.flags.insert(CharacterFlags::WARRIOR);
    world.add_character(warrior);

    let mut archmage = login_character(CharacterId(8), &login_block("Archmage"), 1, 11, 10);
    archmage.level = 33;
    archmage
        .flags
        .insert(CharacterFlags::ARCH | CharacterFlags::MAGE);
    world.add_character(archmage);

    let mut hidden = login_character(CharacterId(9), &login_block("Hidden"), 1, 12, 10);
    hidden.flags.insert(CharacterFlags::INVISIBLE);
    world.add_character(hidden);

    let mut staff_hidden = login_character(CharacterId(10), &login_block("Staff"), 1, 13, 10);
    staff_hidden
        .flags
        .insert(CharacterFlags::STAFF | CharacterFlags::NOWHO);
    world.add_character(staff_hidden);

    let mut npc = login_character(CharacterId(11), &login_block("Npc"), 1, 14, 10);
    npc.flags.remove(CharacterFlags::PLAYER);
    world.add_character(npc);

    let result = apply_who_command(&world, None, CharacterFlags::empty(), "/who")
        .expect("who command should be recognized");

    assert_eq!(
        result.messages,
        vec![
            "Currently online in this area:",
            "Warrior (W12)",
            "Archmage (AM33)",
        ]
    );
}

#[test]
fn staff_who_command_lists_visible_staff_with_legacy_prefix_gate() {
    let mut world = World::default();
    let caller_flags = CharacterFlags::STAFF;

    let mut staff = login_character(CharacterId(7), &login_block("Staffer"), 1, 10, 10);
    staff.flags.insert(CharacterFlags::STAFF);
    world.add_character(staff);

    let mut god = login_character(CharacterId(8), &login_block("LagGod"), 1, 11, 10);
    god.flags.insert(CharacterFlags::GOD);
    god.driver = 42;
    world.add_character(god);

    let mut hidden_staff = login_character(CharacterId(9), &login_block("Hidden"), 1, 12, 10);
    hidden_staff
        .flags
        .insert(CharacterFlags::STAFF | CharacterFlags::INVISIBLE);
    world.add_character(hidden_staff);

    let player = login_character(CharacterId(10), &login_block("Player"), 1, 13, 10);
    world.add_character(player);

    assert!(apply_who_command(&world, None, caller_flags, "/who").is_some());
    assert!(apply_who_command(&world, None, caller_flags, "/whoo").is_none());

    let result = apply_who_command(&world, None, caller_flags, "/whos")
        .expect("legacy cmdcmp accepts whostaff prefix length four");

    assert_eq!(result.messages, vec!["Staffer []", "LagGod [] (lagging)"]);

    let mut runtime = ServerRuntime::default();
    runtime.staff_codes.insert(CharacterId(7), "ST".to_string());
    runtime.staff_codes.insert(CharacterId(8), "GD".to_string());
    let result = apply_who_command(&world, Some(&runtime), caller_flags, "/whos")
        .expect("runtime whostaff should use stored staff codes");
    assert_eq!(
        result.messages,
        vec!["Staffer [ST]", "LagGod [GD] (lagging)"]
    );
}

#[test]
fn apply_arkhata_pool_consumes_scroll_without_reward() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(20));
    let mut world = World::default();
    world.add_character(character);
    let mut scroll = test_item(ItemId(20), 1, ItemFlags::USED | ItemFlags::TAKE);
    scroll.template_id = 0x0100_00C2;
    scroll.carried_by = Some(character_id);
    world.add_item(scroll);
    let mut loader = ZoneLoader::new();

    assert_eq!(
        apply_arkhata_pool(
            &mut world,
            &mut loader,
            character_id,
            ItemId(20),
            seed_for_legacy_random(70, 0)
        ),
        ArkhataPoolApplyResult::Vanished
    );
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.cursor_item, None);
    assert!(!world.items.contains_key(&ItemId(20)));
}

#[test]
fn apply_arkhata_pool_consumes_scroll_and_grants_reward() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(20));
    let mut world = World::default();
    world.add_character(character);
    let mut scroll = test_item(ItemId(20), 1, ItemFlags::USED | ItemFlags::TAKE);
    scroll.template_id = 0x0100_00C2;
    scroll.carried_by = Some(character_id);
    world.add_item(scroll);
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(r#"Red_Scroll: name="Red Scroll" ID=010000C3 flag=IF_TAKE ;"#)
        .unwrap();

    assert_eq!(
        apply_arkhata_pool(
            &mut world,
            &mut loader,
            character_id,
            ItemId(20),
            seed_for_legacy_random(70, 22)
        ),
        ArkhataPoolApplyResult::Gift("Red Scroll".to_string())
    );
    assert!(!world.items.contains_key(&ItemId(20)));
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.cursor_item, None);
    assert!(character.inventory.iter().flatten().any(|item_id| {
        world
            .items
            .get(item_id)
            .is_some_and(|item| item.name == "Red Scroll")
    }));
}

#[test]
fn character_name_packet_uses_legacy_color_words() {
    let mut character = login_character(CharacterId(0x1234), &login_block("Tester"), 1, 10, 10);
    character.c1 = 0x0443;
    character.c2 = 0x0884;
    character.c3 = 0x0cc5;

    let packet = character_name_packet(&character);

    assert_eq!(packet[0], ugaris_protocol::packet::SV_NAME);
    assert_eq!(&packet[1..3], &0x1234_u16.to_le_bytes());
    assert_eq!(&packet[4..6], &0x0443_u16.to_le_bytes());
    assert_eq!(&packet[6..8], &0x0884_u16.to_le_bytes());
    assert_eq!(&packet[8..10], &0x0cc5_u16.to_le_bytes());
}

#[test]
fn character_name_packet_uses_legacy_clan_and_demon_color_rules() {
    let mut character = login_character(CharacterId(0x1234), &login_block("Tester"), 1, 10, 10);
    character.c1 = 0x0443;
    character.c2 = 0x0884;
    character.c3 = 0x0cc5;
    character.clan = 42;

    let packet = character_name_packet(&character);
    assert_eq!(packet[10], 42);
    assert_eq!(&packet[4..10], &[0x43, 0x04, 0x84, 0x08, 0xc5, 0x0c]);

    character.sprite = 27;
    let demon_packet = character_name_packet(&character);
    assert_eq!(&demon_packet[4..10], &[0, 0, 0, 0, 0, 0]);
    assert_eq!(demon_packet[10], 42);
}

#[test]
fn character_name_packet_uses_viewer_specific_pk_relation() {
    let mut viewer = login_character(CharacterId(7), &login_block("Viewer"), 1, 10, 10);
    let mut target = login_character(CharacterId(8), &login_block("Target"), 1, 11, 10);
    viewer.flags |= CharacterFlags::PK;
    target.flags |= CharacterFlags::PK;

    let mut relations = PkRelationSnapshot::default();
    assert_eq!(
        character_name_packet_for_viewer(&relations, &viewer, &target)[11],
        2
    );

    relations
        .hate_by_character
        .insert(viewer.id, vec![target.id.0]);
    assert_eq!(
        character_name_packet_for_viewer(&relations, &viewer, &target)[11],
        3
    );

    relations.hate_by_character.clear();
    relations
        .hate_by_character
        .insert(target.id, vec![viewer.id.0]);
    assert_eq!(
        character_name_packet_for_viewer(&relations, &viewer, &target)[11],
        4
    );

    relations
        .hate_by_character
        .insert(viewer.id, vec![target.id.0]);
    assert_eq!(
        character_name_packet_for_viewer(&relations, &viewer, &target)[11],
        5
    );
}

#[test]
fn help_command_includes_staff_and_god_sections_by_flag() {
    let staff = apply_help_command("/help", CharacterFlags::STAFF, 1)
        .expect("staff help should be recognized");

    assert!(staff
        .messages
        .contains(&"=== STAFF COMMANDS ===".to_string()));
    assert!(staff
        .messages
        .contains(&"/kick <name> - Disconnect a player from the server".to_string()));
    assert!(!staff.messages.contains(&"=== GOD COMMANDS ===".to_string()));

    let god =
        apply_help_command("/help", CharacterFlags::GOD, 1).expect("god help should be recognized");

    assert!(god.messages.contains(&"=== STAFF COMMANDS ===".to_string()));
    assert!(god
        .messages
        .contains(&"=== EVENT/QUEST MASTER COMMANDS ===".to_string()));
    assert!(god.messages.contains(&"=== GOD COMMANDS ===".to_string()));
    assert!(god
        .messages
        .contains(&"/clearmerchantstores <id> - Reset a merchant's inventory".to_string()));
}

#[test]
fn help_command_includes_event_and_live_quest_sections_by_flag() {
    let event = apply_help_command("/help", CharacterFlags::EVENTMASTER, 1)
        .expect("event help should be recognized");

    assert!(event
        .messages
        .contains(&"=== EVENT/QUEST MASTER COMMANDS ===".to_string()));
    assert!(event
        .messages
        .contains(&"== Event Master Commands ==".to_string()));
    assert!(!event
        .messages
        .contains(&"== Quest Master Commands ==".to_string()));

    let lq = apply_help_command("/help", CharacterFlags::LQMASTER, 20)
        .expect("lq help should be recognized");

    assert!(lq
        .messages
        .contains(&"== Quest Master Commands ==".to_string()));
    assert!(lq.messages.contains(
        &"Note: Additional LQ commands are available in the Live Quest area".to_string()
    ));
}

#[test]
fn initial_map_payloads_send_visible_diamond_and_center_character() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, LOGIN_SPAWN_X, LOGIN_SPAWN_Y);
    character.x = LOGIN_SPAWN_X as u16;
    character.y = LOGIN_SPAWN_Y as u16;
    let mut world = World::default();
    assert!(world.spawn_character(character.clone(), LOGIN_SPAWN_X, LOGIN_SPAWN_Y));

    let pk_relations = PkRelationSnapshot::default();
    let payloads = initial_map_payloads(&world, &character, &pk_relations, 1);
    assert_eq!(payloads.len(), 1);
    let payload = &payloads[0];

    assert_eq!(
        payload[0],
        SV_MAP01 | SV_MAPPOS | MAP_EFFECT_0 | MAP_EFFECT_1 | MAP_EFFECT_2 | MAP_EFFECT_3,
        "full refresh stomps every cell starting with its effect pointers"
    );
    assert!(payload.windows(16).any(|window| {
        window
            == [
                SV_MAP10
                    | SV_MAPPOS
                    | MAP_CHARACTER_SPRITE
                    | MAP_CHARACTER_ACTION
                    | MAP_CHARACTER_STATUS,
                4,
                0,
                1,
                0,
                0,
                0,
                7,
                0,
                0,
                0,
                0,
                0,
                100,
                100,
                0,
            ]
    }));
    assert!(payload_contains_character_name(payload, 7, "Tester"));
}

#[test]
fn initial_map_payloads_send_visible_map_effect_slots() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, LOGIN_SPAWN_X, LOGIN_SPAWN_Y);
    character.x = LOGIN_SPAWN_X as u16;
    character.y = LOGIN_SPAWN_Y as u16;
    let mut world = World::default();
    world
        .map
        .tile_mut(LOGIN_SPAWN_X, LOGIN_SPAWN_Y)
        .unwrap()
        .effects = [42, 0, 77, 0];
    assert!(world.spawn_character(character.clone(), LOGIN_SPAWN_X, LOGIN_SPAWN_Y));

    let pk_relations = PkRelationSnapshot::default();
    let payloads = initial_map_payloads(&world, &character, &pk_relations, 1);
    let payload = &payloads[0];

    assert!(payload.windows(19).any(|window| {
        window
            == [
                SV_MAP01 | SV_MAPPOS | MAP_EFFECT_0 | MAP_EFFECT_1 | MAP_EFFECT_2 | MAP_EFFECT_3,
                4,
                0,
                42,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                77,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ]
    }));
}

#[test]
fn map_diff_payloads_clear_removed_map_effect_slots() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
    character.x = 10;
    character.y = 10;
    let mut world = World::default();
    world.map.tile_mut(10, 10).unwrap().effects = [42, 0, 77, 0];
    assert!(world.spawn_character(character.clone(), 10, 10));
    let pk_relations = PkRelationSnapshot::default();
    let mut cache = visible_map_cache(&world, &character, &pk_relations, 1);
    world.map.tile_mut(10, 10).unwrap().effects = [0; 4];

    let payloads = map_diff_payloads(&world, &character, &pk_relations, 1, &mut cache);
    let payload = payloads.concat();

    assert!(payload.windows(19).any(|window| {
        window
            == [
                SV_MAP01 | SV_MAPPOS | MAP_EFFECT_0 | MAP_EFFECT_1 | MAP_EFFECT_2 | MAP_EFFECT_3,
                4,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ]
    }));
}

#[test]
fn map_diff_payloads_send_only_changed_same_origin_cells() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
    character.x = 10;
    character.y = 10;
    let mut world = World::default();
    assert!(world.spawn_character(character.clone(), 10, 10));
    let pk_relations = PkRelationSnapshot::default();
    let mut cache = visible_map_cache(&world, &character, &pk_relations, 1);

    world.map.tile_mut(11, 10).unwrap().ground_sprite = 777;
    let payloads = map_diff_payloads(&world, &character, &pk_relations, 1, &mut cache);

    assert_eq!(payloads.len(), 1);
    let payload = &payloads[0];
    assert_ne!(payload.first().copied(), Some(SV_ORIGIN));
    assert!(payload.windows(17).any(|window| {
        window
            == [
                SV_MAP11
                    | SV_MAPPOS
                    | MAP_TILE_GSPRITE
                    | MAP_TILE_FSPRITE
                    | MAP_TILE_ISPRITE
                    | MAP_TILE_FLAGS,
                5,
                0,
                9,
                3,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                30,
                0,
            ]
    }));
    assert!(map_diff_payloads(&world, &character, &pk_relations, 1, &mut cache).is_empty());
}

#[test]
fn map_diff_payloads_clear_removed_visible_character() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
    character.x = 10;
    character.y = 10;
    let mut other = login_character(CharacterId(8), &login, 1, 11, 10);
    other.x = 11;
    other.y = 10;
    let mut world = World::default();
    assert!(world.spawn_character(character.clone(), 10, 10));
    assert!(world.spawn_character(other, 11, 10));
    let pk_relations = PkRelationSnapshot::default();
    let mut cache = visible_map_cache(&world, &character, &pk_relations, 1);

    world.remove_character(CharacterId(8));
    let payloads = map_diff_payloads(&world, &character, &pk_relations, 1, &mut cache);

    assert_eq!(payloads.len(), 1);
    assert!(payloads[0]
        .windows(3)
        .any(|window| { window == [SV_MAP10 | SV_MAPPOS | MAP_CHARACTER_CLEAR, 5, 0] }));
}

#[test]
fn map_diff_payloads_send_name_for_new_visible_character() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
    character.x = 10;
    character.y = 10;
    let mut other = login_character(CharacterId(8), &login_block("Guard"), 1, 11, 10);
    other.x = 11;
    other.y = 10;
    let mut world = World::default();
    assert!(world.spawn_character(character.clone(), 10, 10));
    let pk_relations = PkRelationSnapshot::default();
    let mut cache = visible_map_cache(&world, &character, &pk_relations, 1);

    assert!(world.spawn_character(other, 11, 10));
    let payloads = map_diff_payloads(&world, &character, &pk_relations, 1, &mut cache);

    assert_eq!(payloads.len(), 1);
    assert!(payload_contains_character_name(&payloads[0], 8, "Guard"));
    assert!(payloads[0].windows(16).any(|window| {
        window[0]
            == SV_MAP10
                | SV_MAPPOS
                | MAP_CHARACTER_SPRITE
                | MAP_CHARACTER_ACTION
                | MAP_CHARACTER_STATUS
            && window[7] == 8
            && window[8] == 0
    }));
    assert!(map_diff_payloads(&world, &character, &pk_relations, 1, &mut cache).is_empty());
}

#[test]
fn client_effect_payloads_send_visible_effect_records_and_used_mask() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
    character.x = 10;
    character.y = 10;
    let mut world = World::default();
    let mut effect = Effect::new(EF_FIREBALL, 123, 55, 65);
    effect.from_x = 10;
    effect.from_y = 10;
    effect.to_x = 12;
    effect.to_y = 10;
    effect.x = 11 * 1024 + 512;
    effect.y = 10 * 1024 + 512;
    world.effects.insert(123, effect);
    let mut cache = ClientEffectCache::default();

    let payloads = client_effect_payloads(&world, &character, 2, &mut cache);

    assert_eq!(payloads.len(), 2);
    assert_eq!(payloads[0][0], ugaris_protocol::packet::SV_CEFFECT);
    assert_eq!(payloads[0][1], 0);
    assert_eq!(&payloads[0][2..10], &[123, 0, 0, 0, 4, 0, 0, 0]);
    assert_eq!(
        &payloads[1][..],
        &ugaris_protocol::packet::used_effects(1)[..]
    );
    assert!(client_effect_payloads(&world, &character, 2, &mut cache).is_empty());

    world.effects.clear();
    let payloads = client_effect_payloads(&world, &character, 2, &mut cache);
    assert_eq!(payloads.len(), 1);
    assert_eq!(
        &payloads[0][..],
        &ugaris_protocol::packet::used_effects(0)[..]
    );
    assert!(cache.slots.iter().all(Option::is_none));
}

#[test]
fn client_effect_payloads_send_visible_edemonball_records() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
    character.x = 10;
    character.y = 10;
    let mut world = World::default();
    let mut effect = Effect::new(EF_EDEMONBALL, 125, 55, 65);
    effect.base_sprite = 50050;
    effect.from_x = 10;
    effect.from_y = 10;
    effect.to_x = 12;
    effect.to_y = 10;
    effect.x = 11 * 1024 + 512;
    effect.y = 10 * 1024 + 512;
    world.effects.insert(125, effect);

    let payloads = client_effect_payloads(&world, &character, 2, &mut ClientEffectCache::default());

    assert_eq!(payloads.len(), 2);
    assert_eq!(payloads[0][0], ugaris_protocol::packet::SV_CEFFECT);
    assert_eq!(payloads[0][1], 0);
    assert_eq!(
        &payloads[0][2..],
        &ugaris_protocol::packet::ceffect_edemonball(125, 55, 50050, 10, 10, 12, 10)[..]
    );
    assert_eq!(
        &payloads[1][..],
        &ugaris_protocol::packet::used_effects(1)[..]
    );
}

#[test]
fn client_effect_payloads_send_visible_map_anchored_effect_records() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
    character.x = 10;
    character.y = 10;
    let mut world = World::default();
    let mut effect = Effect::new(EF_EXPLODE, 90, 55, 63);
    effect.x = 11;
    effect.y = 10;
    effect.base_sprite = 50050;
    world.effects.insert(90, effect);

    let payloads = client_effect_payloads(&world, &character, 2, &mut ClientEffectCache::default());

    assert_eq!(payloads.len(), 2);
    assert_eq!(payloads[0][0], ugaris_protocol::packet::SV_CEFFECT);
    assert_eq!(payloads[0][1], 0);
    assert_eq!(
        &payloads[0][2..],
        &ugaris_protocol::packet::ceffect_explode(90, 55, 50050)[..]
    );
    assert_eq!(
        &payloads[1][..],
        &ugaris_protocol::packet::used_effects(1)[..]
    );
}

#[test]
fn client_effect_payloads_skip_effects_outside_visible_diamond() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
    character.x = 10;
    character.y = 10;
    let mut world = World::default();
    let mut effect = Effect::new(EF_BALL, 124, 55, 65);
    effect.x = 20 * 1024 + 512;
    effect.y = 20 * 1024 + 512;
    world.effects.insert(124, effect);

    assert!(
        client_effect_payloads(&world, &character, 2, &mut ClientEffectCache::default()).is_empty()
    );
}

#[test]
fn client_effect_payloads_send_visible_character_spell_effects() {
    let login = login_block("Tester");
    let mut viewer = login_character(CharacterId(7), &login, 1, 10, 10);
    viewer.x = 10;
    viewer.y = 10;
    let mut target = login_character(CharacterId(8), &login, 1, 11, 10);
    target.x = 11;
    target.y = 10;
    let mut world = World::default();
    world.characters.insert(target.id, target.clone());
    let mut effect = Effect::new(EF_BLESS, 77, 100, 200);
    effect.target_character = Some(target.id);
    effect.strength = 33;
    world.effects.insert(77, effect);

    let payloads = client_effect_payloads(&world, &viewer, 2, &mut ClientEffectCache::default());

    assert_eq!(payloads.len(), 2);
    assert_eq!(payloads[0][0], ugaris_protocol::packet::SV_CEFFECT);
    assert_eq!(payloads[0][1], 0);
    assert_eq!(
        &payloads[0][2..],
        &ugaris_protocol::packet::ceffect_bless(77, 8, 100, 200, 33)[..]
    );
    assert_eq!(
        &payloads[1][..],
        &ugaris_protocol::packet::used_effects(1)[..]
    );
}

#[test]
fn look_map_payload_visible_area1_section_reports_name_and_difficulty() {
    let payloads = look_map_payloads(
        &World::default(),
        1,
        LookMapRequest {
            character_id: CharacterId(7),
            x: 146,
            y: 115,
            character_level: 7,
            visible: true,
        },
    );

    assert_eq!(
        text_payloads(&payloads),
        vec!["Skellie I. This area is too easy for you. (146,115)"]
    );
}

#[test]
fn walk_section_payload_reports_entering_once_with_legacy_color() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, 146, 115);
    character.x = 146;
    character.y = 115;
    let mut player = PlayerRuntime::connected(1, 0);

    let payload = walk_section_payload(1, &mut player, &character).unwrap();

    assert_eq!(
        text_payload_bytes(&payload),
        b"\xb0c1Now entering Skellie I."
    );
    assert_eq!(special_payload(&payload), Some((1003, u32::MAX, 0)));
    assert_eq!(player.current_section_id, 46);
    assert!(walk_section_payload(1, &mut player, &character).is_none());
}

#[test]
fn walk_section_payload_reports_leaving_previous_section() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 99, 12, 13);
    character.x = 12;
    character.y = 13;
    let mut player = PlayerRuntime::connected(1, 0);
    player.current_section_id = 1;

    let payload = walk_section_payload(99, &mut player, &character).unwrap();

    assert_eq!(
        text_payload_bytes(&payload),
        b"\xb0c1Now leaving Skellie I."
    );
    assert_eq!(special_payload(&payload), None);
    assert_eq!(player.current_section_id, 0);
}

#[test]
fn section_music_special_matches_legacy_music_switch() {
    assert_eq!(section_music_special(4), Some(1003));
    assert_eq!(section_music_special(57), Some(1010));
    assert_eq!(section_music_special(58), Some(1004));
    assert_eq!(section_music_special(60), Some(1002));
    assert_eq!(section_music_special(114), None);
}

#[test]
fn area_sound_payload_uses_section_and_legacy_special_layout() {
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 146, 115);
    character.x = 146;
    character.y = 115;
    let seed = seed_for_legacy_random(100, 10);

    let payload = area_sound_payload(1, &character, 12, seed).unwrap();

    assert_eq!(payload[0], SV_SPECIAL);
    assert_eq!(u32::from_le_bytes(payload[1..5].try_into().unwrap()), 14);
    assert_eq!(
        i32::from_le_bytes(payload[5..9].try_into().unwrap()),
        -(legacy_random(seed.wrapping_add(1), 1000) as i32 + 100)
    );
    assert_eq!(
        i32::from_le_bytes(payload[9..13].try_into().unwrap()),
        5000 - legacy_random(seed.wrapping_add(2), 10000) as i32
    );
}

#[test]
fn area_sound_payload_is_silent_outside_ambient_sections() {
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 12, 13);
    character.x = 12;
    character.y = 13;
    let seed = seed_for_legacy_random(100, 10);

    assert_eq!(area_sound_payload(99, &character, 12, seed), None);
}

#[test]
fn movement_scroll_payload_uses_scroll_origin_clear_and_center_update() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, 11, 10);
    character.x = 11;
    character.y = 10;
    let mut world = World::default();
    assert!(world.spawn_character(character.clone(), 11, 10));

    let pk_relations = PkRelationSnapshot::default();
    let mut cache = visible_map_cache(&world, &character, &pk_relations, 1);
    cache.center_x = 10; // pretend the cache was built before the step
    let payload = movement_scroll_payload(&character, 10, 10, 1, &mut cache).unwrap();

    assert_eq!(payload[0], SV_SCROLL_RIGHT);
    assert_eq!(payload[1], SV_ORIGIN);
    assert_eq!(&payload[2..6], &[11, 0, 10, 0]);
    assert_eq!(payload[6], SV_MAP10 | SV_MAPPOS | MAP_CHARACTER_CLEAR);
    assert_eq!(&payload[7..9], &[3, 0]);
    assert!(payload.windows(16).any(|window| {
        window
            == [
                SV_MAP10
                    | SV_MAPPOS
                    | MAP_CHARACTER_SPRITE
                    | MAP_CHARACTER_ACTION
                    | MAP_CHARACTER_STATUS,
                4,
                0,
                1,
                0,
                0,
                0,
                7,
                0,
                0,
                0,
                0,
                0,
                100,
                100,
                0,
            ]
    }));
    assert_eq!(cache.center_x, 11, "cache center follows the scroll");
}

#[test]
fn walk_scroll_then_diff_sends_fringe_character_with_name() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
    character.x = 10;
    character.y = 10;
    let mut other = login_character(CharacterId(8), &login_block("Guard"), 1, 13, 10);
    other.x = 13;
    other.y = 10;
    let mut world = World::default();
    assert!(world.spawn_character(character.clone(), 10, 10));
    assert!(world.spawn_character(other, 13, 10));
    // Light the guard tile so it is visible at distance two.
    world.map.tile_mut(13, 10).unwrap().light = 255;

    let pk_relations = PkRelationSnapshot::default();
    let mut cache = visible_map_cache(&world, &character, &pk_relations, 2);

    // Step right: guard moves from the diamond edge into clear view.
    let mut character = world.characters.get(&CharacterId(7)).unwrap().clone();
    character.x = 11;
    let scroll = movement_scroll_payload(&character, 10, 10, 2, &mut cache);
    assert!(scroll.is_some());

    let diff = map_diff_payloads(&world, &character, &pk_relations, 2, &mut cache);
    let bytes: Vec<u8> = diff.iter().flat_map(|payload| payload.to_vec()).collect();
    assert!(payload_contains_character_name(&bytes, 8, "Guard"));
}

#[test]
fn cache_shift_replicates_client_memmove_and_drops_scrolled_in_cells() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
    character.x = 10;
    character.y = 10;
    let mut world = World::default();
    world.map.tile_mut(10, 10).unwrap().ground_sprite = 777;
    assert!(world.spawn_character(character.clone(), 10, 10));

    let pk_relations = PkRelationSnapshot::default();
    let mut cache = visible_map_cache(&world, &character, &pk_relations, 1);
    let side = 3u16;
    let center = 1 + 1 * side; // pos 4
    let center_cell = cache.cells.get(&center).cloned().unwrap();
    assert_eq!(center_cell.tile.gsprite, 777);

    // Move right: the center cell content lands one position to the left.
    cache.shift(1, 0, 11, 10);
    assert_eq!(
        cache.cells.get(&(center - 1)).map(|cell| cell.tile.gsprite),
        Some(777),
        "cells shift against the movement direction like the client memmove"
    );
    assert!(
        !cache.cells.contains_key(&8),
        "cells scrolled in from untracked positions are dropped for resend"
    );
}

/// Manual profiling harness for the deferred "Sector skip optimization
/// (`skipx_sector`)" P3 task in `PORTING_TODO.md`. C's `plr_map_update`
/// uses `skipx_sector` to skip re-scanning tiles in map sectors that
/// haven't changed since the viewer's own last tick, avoiding the
/// per-tile `char_see_char`/line-of-sight cost for a run of unchanged
/// tiles. Rust's `map_diff_payloads` has no equivalent skip and always
/// recomputes every tile in the viewer's diamond every tick. This
/// harness measures that unconditional recompute cost at a player count
/// well above any real Ugaris concurrent population, to decide whether
/// porting the skip is worth the (large, cross-cutting) `set_sector`
/// call-site integration it would require. Run explicitly with:
/// `cargo test --release -p ugaris-server profile_map_diff_payloads_cost -- --ignored --nocapture`
#[test]
#[ignore = "manual profiling harness, not part of the regular suite - see doc comment"]
fn profile_map_diff_payloads_cost_at_realistic_player_counts() {
    use std::time::Instant;

    // Legacy client default view range and a generously high player count
    // (real Ugaris areas typically see a handful of concurrent players).
    let view_distance = 15usize;
    let player_count = 100usize;
    let ticks = 50usize;

    let mut world = World::default();
    let login = login_block("Viewer");
    let mut character_ids = Vec::new();
    for n in 0..player_count {
        let x = 40 + (n % 20) * 8;
        let y = 40 + (n / 20) * 8;
        let id = CharacterId(100 + n as u32);
        let mut character = login_character(id, &login, 1, x, y);
        character.x = x as u16;
        character.y = y as u16;
        assert!(world.spawn_character(character, x, y));
        character_ids.push(id);
    }

    let pk_relations = PkRelationSnapshot::default();
    let mut caches: Vec<_> = character_ids
        .iter()
        .map(|id| {
            let character = world.characters.get(id).unwrap();
            visible_map_cache(&world, character, &pk_relations, view_distance)
        })
        .collect();

    let start = Instant::now();
    for _ in 0..ticks {
        for (id, cache) in character_ids.iter().zip(caches.iter_mut()) {
            let character = world.characters.get(id).unwrap();
            let _ = map_diff_payloads(&world, character, &pk_relations, view_distance, cache);
        }
    }
    let elapsed = start.elapsed();
    let per_tick_all_players = elapsed / ticks as u32;
    let per_player_per_tick = elapsed / (ticks * player_count) as u32;

    println!(
        "profile_map_diff_payloads_cost: {player_count} players, view_distance={view_distance}, \
         {ticks} ticks -> total={elapsed:?}, per-tick(all players)={per_tick_all_players:?}, \
         per-player-per-tick={per_player_per_tick:?}"
    );
}
