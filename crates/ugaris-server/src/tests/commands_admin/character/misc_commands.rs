use super::*;

#[test]
pub(crate) fn setlevel_is_god_only_and_requires_full_command() {
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
pub(crate) fn join_chat_command_gates_staff_and_god_channels() {
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
pub(crate) fn weather_command_reports_god_debug_info() {
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
pub(crate) fn weather_admin_commands_mutate_runtime_state_with_legacy_feedback() {
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
pub(crate) fn weather_admin_commands_preserve_legacy_gates_and_validation() {
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
pub(crate) fn tell_command_forwards_to_spying_god_even_when_recipient_blocks() {
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
pub(crate) fn admin_subhelp_commands_match_legacy_privilege_gates_and_text() {
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
