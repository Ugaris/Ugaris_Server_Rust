use super::*;

#[test]
fn god_setlevel_mutates_self_and_clears_spell_slots_and_effects() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Godmode"), 1, 10, 10);
    character
        .flags
        .insert(CharacterFlags::GOD | CharacterFlags::MAGE | CharacterFlags::ARCH);
    character.values[1][CharacterValue::Duration as usize] = 9;
    character.values[1][CharacterValue::Rage as usize] = 8;
    character.inventory[12] = Some(ItemId(99));
    world.add_character(character);
    world.add_item(Item {
        id: ItemId(99),
        name: "Spell".to_string(),
        description: String::new(),
        flags: ItemFlags::TAKE,
        sprite: 1,
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
        driver: IDR_ARMOR,
        driver_data: vec![0; 40],
        serial: 1,
    });
    let mut effect = Effect::new(EF_BLESS, 1, 0, 10);
    effect.target_character = Some(character_id);
    world.effects.insert(1, effect);
    let mut runtime = ServerRuntime::default();

    let low =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/setlevel 29", 1)
            .expect("god setlevel should be recognized");
    assert!(low.messages.is_empty());
    assert!(low.inventory_changed);
    assert!(low.name_changed);
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.level, 29);
    assert_eq!(character.exp, legacy_level_exp(29));
    assert!(!character.flags.contains(CharacterFlags::ARCH));
    assert_eq!(character.values[1][CharacterValue::Duration as usize], 0);
    assert_eq!(character.values[1][CharacterValue::Rage as usize], 0);
    assert_eq!(character.inventory[12], None);
    assert!(!world.items.contains_key(&ItemId(99)));
    assert!(world.effects.is_empty());

    let high =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/setlevel 36", 1)
            .expect("god setlevel should be recognized");
    assert!(high.messages.is_empty());
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.level, 36);
    assert_eq!(character.exp, legacy_level_exp(36));
    assert!(character.flags.contains(CharacterFlags::ARCH));
    assert_eq!(character.values[1][CharacterValue::Duration as usize], 1);
    assert_eq!(character.values[1][CharacterValue::Rage as usize], 0);
}

#[test]
fn weather_command_reports_indoor_protection_and_outdoor_effects() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Weather"), 1, 10, 10);
    character.x = 10;
    character.y = 10;
    world.add_character(character);
    let weather = WeatherState {
        current_weather: 2,
        weather_intensity: 3,
        weather_effects: WEATHER_EFFECT_SLOW | WEATHER_EFFECT_BLIND | WEATHER_EFFECT_SLIP,
        affected_areas: vec![1],
        ..WeatherState::default()
    };

    let outdoor = apply_weather_command(&world, character_id, 1, &weather, "/weather")
        .expect("weather command should be recognized");
    assert_eq!(
        outdoor.messages,
        vec![
            "Current weather in this area: Heavy storm",
            "Movement is affected by the weather.",
            "Visibility is reduced by the weather.",
            "The weather makes the ground slippery.",
        ]
    );

    world.map.set_flags(10, 10, MapFlags::INDOORS);
    let indoor = apply_weather_command(&world, character_id, 1, &weather, "/weather")
        .expect("weather command should be recognized");
    assert_eq!(
        indoor.messages,
        vec![
            "Current weather in this area: Heavy storm",
            "You are indoors and protected from weather effects.",
        ]
    );
}

#[test]
fn client_effect_payloads_reuse_slot_after_effect_disappears() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
    character.x = 10;
    character.y = 10;
    let mut world = World::default();
    let mut first = Effect::new(EF_FIREBALL, 123, 55, 65);
    first.from_x = 10;
    first.from_y = 10;
    first.to_x = 12;
    first.to_y = 10;
    first.x = 11 * 1024 + 512;
    first.y = 10 * 1024 + 512;
    world.effects.insert(123, first);
    let mut cache = ClientEffectCache::default();

    let payloads = client_effect_payloads(&world, &character, 2, &mut cache);
    assert_eq!(payloads[0][1], 0);

    world.effects.clear();
    assert_eq!(
        &client_effect_payloads(&world, &character, 2, &mut cache)[0][..],
        &ugaris_protocol::packet::used_effects(0)[..]
    );

    let mut second = Effect::new(EF_BALL, 124, 56, 66);
    second.from_x = 10;
    second.from_y = 10;
    second.to_x = 12;
    second.to_y = 10;
    second.x = 11 * 1024 + 512;
    second.y = 10 * 1024 + 512;
    world.effects.insert(124, second);

    let payloads = client_effect_payloads(&world, &character, 2, &mut cache);
    assert_eq!(payloads.len(), 2);
    assert_eq!(payloads[0][0], ugaris_protocol::packet::SV_CEFFECT);
    assert_eq!(payloads[0][1], 0);
    assert_eq!(&payloads[0][2..10], &[124, 0, 0, 0, 2, 0, 0, 0]);
    assert_eq!(
        &payloads[1][..],
        &ugaris_protocol::packet::used_effects(1)[..]
    );
}

#[test]
fn client_effect_payloads_send_legacy_curse_cap_and_lag_effects() {
    let login = login_block("Tester");
    let mut viewer = login_character(CharacterId(7), &login, 1, 10, 10);
    viewer.x = 10;
    viewer.y = 10;
    let mut target = login_character(CharacterId(8), &login, 1, 11, 10);
    target.x = 11;
    target.y = 10;
    let mut world = World::default();
    world.characters.insert(target.id, target.clone());

    let mut curse = Effect::new(EF_CURSE, 77, 100, 200);
    curse.target_character = Some(target.id);
    curse.strength = 33;
    world.effects.insert(77, curse);
    let mut cap = Effect::new(EF_CAP, 78, 101, 201);
    cap.target_character = Some(target.id);
    world.effects.insert(78, cap);
    let mut lag = Effect::new(EF_LAG, 79, 102, 202);
    lag.target_character = Some(target.id);
    world.effects.insert(79, lag);

    let payloads = client_effect_payloads(&world, &viewer, 2, &mut ClientEffectCache::default());

    assert_eq!(payloads.len(), 4);
    assert_eq!(
        &payloads[0][2..],
        &ugaris_protocol::packet::ceffect_curse(77, 8, 100, 200, 33)[..]
    );
    assert_eq!(
        &payloads[1][2..],
        &ugaris_protocol::packet::ceffect_cap(78, 8)[..]
    );
    assert_eq!(
        &payloads[2][2..],
        &ugaris_protocol::packet::ceffect_lag(79, 8)[..]
    );
    assert_eq!(
        &payloads[3][..],
        &ugaris_protocol::packet::used_effects(7)[..]
    );
}

#[test]
fn client_effect_payloads_hide_character_spell_effects_with_hidden_target() {
    let login = login_block("Tester");
    let mut viewer = login_character(CharacterId(7), &login, 1, 10, 10);
    viewer.x = 10;
    viewer.y = 10;
    let mut target = login_character(CharacterId(8), &login, 1, 20, 20);
    target.x = 20;
    target.y = 20;
    let mut world = World::default();
    world.characters.insert(target.id, target.clone());
    let mut effect = Effect::new(EF_HEAL, 77, 100, 200);
    effect.target_character = Some(target.id);
    world.effects.insert(77, effect);

    assert!(
        client_effect_payloads(&world, &viewer, 2, &mut ClientEffectCache::default()).is_empty()
    );
}

#[test]
fn retained_effect_policy_removes_stale_pk_hate_when_level_gate_fails() {
    let mut attacker = login_character(CharacterId(1), &login_block("Attacker"), 1, 10, 10);
    attacker.flags.insert(CharacterFlags::PK);
    attacker.level = 10;
    let mut target = login_character(CharacterId(2), &login_block("Target"), 1, 11, 10);
    target.flags.insert(CharacterFlags::PK);
    target.level = 14;
    let mut player = PlayerRuntime::connected(1, 0);
    assert!(player.add_pk_hate(2));

    remove_stale_pvp_hate_if_effect_check_fails(&mut player, &attacker, &target, 2);

    assert!(!player.has_pk_hate_for(2));
}

#[test]
fn retained_effect_policy_preserves_hate_for_area_one_town_block() {
    let mut attacker = login_character(CharacterId(1), &login_block("Attacker"), 1, 10, 10);
    attacker.flags.insert(CharacterFlags::PK);
    attacker.level = 10;
    let mut target = login_character(CharacterId(2), &login_block("Target"), 1, 11, 10);
    target.flags.insert(CharacterFlags::PK);
    target.level = 14;
    let mut player = PlayerRuntime::connected(1, 0);
    assert!(player.add_pk_hate(2));

    remove_stale_pvp_hate_if_effect_check_fails(&mut player, &attacker, &target, 1);

    assert!(player.has_pk_hate_for(2));
}
