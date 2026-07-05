use super::*;

#[test]
fn character_fireball_command_queues_character_target_action() {
    let queued = action_to_queued(&ClientAction::CharacterSpell {
        spell: SpellAction::Fireball,
        character: 42,
    })
    .unwrap();

    assert_eq!(queued.action, PlayerActionCode::FireballCharacter);
    assert_eq!((queued.arg1, queued.arg2), (42, 0));
}

/// C `cl_kill`/`cl_give`/`player_driver_charspell` all capture
/// `ch[co].serial` synchronously while parsing the client packet. Live
/// dispatch (`apply_player_action`, unlike the pure `action_to_queued`
/// helper) must do the same lookup against the current world state instead
/// of leaving the serial at the `0` no-check sentinel.
#[test]
fn apply_player_action_kill_captures_live_target_serial() {
    let mut world = World::default();
    let target = login_character(CharacterId(2), &login_block("Target"), 1, 11, 10);
    world.add_character(target);

    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    apply_player_action(
        &mut player,
        &ClientAction::Kill { character: 2 },
        0,
        &world.characters,
    );

    assert_eq!(player.action.action, PlayerActionCode::Kill);
    assert_eq!((player.action.arg1, player.action.arg2), (2, 2));
}

#[test]
fn apply_player_action_kill_of_unknown_character_captures_zero_serial() {
    let world = World::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    apply_player_action(
        &mut player,
        &ClientAction::Kill { character: 99 },
        0,
        &world.characters,
    );

    assert_eq!(player.action.action, PlayerActionCode::Kill);
    assert_eq!((player.action.arg1, player.action.arg2), (99, 0));
}

#[test]
fn apply_player_action_give_captures_live_target_serial() {
    let mut world = World::default();
    let target = login_character(CharacterId(2), &login_block("Target"), 1, 11, 10);
    world.add_character(target);

    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    apply_player_action(
        &mut player,
        &ClientAction::Give { character: 2 },
        0,
        &world.characters,
    );

    assert_eq!(player.action.action, PlayerActionCode::Give);
    assert_eq!((player.action.arg1, player.action.arg2), (2, 2));
}

#[test]
fn apply_player_action_character_spell_captures_live_target_serial() {
    // Live traffic only produces `ClientAction::CharacterSpell` for
    // CL_BLESS/CL_HEAL (`crates/ugaris-protocol/src/command.rs`); both map
    // to a character-only `PlayerActionCode` regardless of the
    // `character_target` flag, so bless is the representative case here.
    let mut world = World::default();
    let target = login_character(CharacterId(2), &login_block("Target"), 1, 11, 10);
    world.add_character(target);

    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    apply_player_action(
        &mut player,
        &ClientAction::CharacterSpell {
            spell: SpellAction::Bless,
            character: 2,
        },
        0,
        &world.characters,
    );

    assert_eq!(player.queue.len(), 1);
    assert_eq!(player.queue[0].action, PlayerActionCode::Bless);
    assert_eq!((player.queue[0].arg1, player.queue[0].arg2), (2, 2));
}

#[test]
fn apply_player_action_map_spell_character_target_captures_live_serial() {
    let mut world = World::default();
    let target = login_character(CharacterId(2), &login_block("Target"), 1, 11, 10);
    world.add_character(target);

    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    apply_player_action(
        &mut player,
        &ClientAction::MapSpell {
            spell: SpellAction::Ball,
            x: 0,
            y: 2,
        },
        0,
        &world.characters,
    );

    assert_eq!(player.queue.len(), 1);
    assert_eq!(player.queue[0].action, PlayerActionCode::BallCharacter);
    assert_eq!((player.queue[0].arg1, player.queue[0].arg2), (2, 2));
}

#[test]
fn warp_trial_door_spawn_helper_instantiates_fighter_at_room_center() {
    let mut world = World::default();
    world.map = ugaris_core::map::MapGrid::new(20, 20);
    let mut loader = ZoneLoader::new();
    loader
        .load_character_templates_str(
            r#"
                warped_fighter:
                  name="Hrus-tak-lan"
                  description="A weird looking creature."
                  sprite=36
                  flag=CF_ALIVE
                  V_HP=10
                  V_ENDURANCE=8
                  V_MANA=0
                  V_MAGICSHIELD=3
                  driver=83
                ;
                "#,
        )
        .unwrap();
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(50);

    assert!(spawn_warp_trial_fighter(
        &mut world,
        &mut loader,
        &mut runtime,
        "warped_fighter",
        7,
        8,
    ));

    let fighter = world.characters.get(&CharacterId(50)).unwrap();
    assert_eq!((fighter.x, fighter.y), (7, 8));
    assert_eq!(fighter.name, "Hrus-tak-lan");
    assert_eq!(fighter.dir, Direction::RightDown as u8);
    assert_eq!(fighter.hp, 10 * POWERSCALE);
    assert_eq!(fighter.endurance, 8 * POWERSCALE);
    assert_eq!(fighter.lifeshield, 3 * POWERSCALE);
}

#[test]
fn maxlag_command_matches_legacy_range_and_feedback() {
    let mut player = PlayerRuntime::connected(1, 0);

    let invalid = apply_maxlag_command(&mut player, "/maxlag 2")
        .expect("maxlag command should be recognized");
    assert_eq!(
        invalid.messages,
        vec!["Number must be between 3 and 20.".to_string()]
    );
    assert_eq!(player.max_lag_seconds, 0);

    let high = apply_maxlag_command(&mut player, "/maxlag 21")
        .expect("maxlag command should be recognized");
    assert_eq!(
        high.messages,
        vec!["Number must be between 3 and 20.".to_string()]
    );
    assert_eq!(player.max_lag_seconds, 0);

    let result = apply_maxlag_command(&mut player, "/maxl 12abc")
        .expect("legacy maxlag abbreviation should be recognized");
    assert_eq!(player.max_lag_seconds, 12);
    assert_eq!(
        result.messages,
        vec!["Set delay for lag control to kick in to 12 seconds.".to_string()]
    );

    assert!(apply_maxlag_command(&mut player, "/ma 12").is_none());
}

#[test]
fn hints_command_toggles_lostcon_hint_flag_with_legacy_feedback() {
    let mut player = PlayerRuntime::connected(1, 0);

    let off = apply_hints_command(&mut player, "/hint")
        .expect("legacy hints abbreviation should be recognized");
    assert!(player.hints_disabled);
    assert_eq!(off.messages, vec!["Hints turned off.".to_string()]);

    let on =
        apply_hints_command(&mut player, "/hints").expect("hints command should be recognized");
    assert!(!player.hints_disabled);
    assert_eq!(on.messages, vec!["Hints turned on.".to_string()]);

    assert!(apply_hints_command(&mut player, "/hin").is_none());
    assert!(apply_hints_command(&mut player, "/hintsx").is_none());
}

#[test]
fn swap_command_exchanges_positions_and_stamps_swapped_timestamp() {
    let mut world = World::default();
    world.tick.0 = 5 * TICKS_PER_SECOND;
    let mut actor = login_character(CharacterId(1), &login_block("Actor"), 1, 10, 10);
    actor.dir = Direction::Right as u8;
    assert!(world.spawn_character(actor, 10, 10));
    let target = login_character(CharacterId(2), &login_block("Target"), 1, 11, 10);
    assert!(world.spawn_character(target, 11, 10));

    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(player.swapped_at(), 0);

    let result = apply_swap_command(&mut world, &mut player, CharacterId(1), "/swap")
        .expect("swap command should be recognized");
    assert_eq!(result, KeyringCommandResult::default());

    assert_eq!(
        (
            world.characters[&CharacterId(1)].x,
            world.characters[&CharacterId(1)].y
        ),
        (11, 10)
    );
    assert_eq!(
        (
            world.characters[&CharacterId(2)].x,
            world.characters[&CharacterId(2)].y
        ),
        (10, 10)
    );
    assert_eq!(player.swapped_at(), 5);
}

#[test]
fn swap_command_requires_exact_word_and_is_a_silent_no_op_when_blocked() {
    let mut world = World::default();
    let mut player = PlayerRuntime::connected(1, 0);

    assert!(apply_swap_command(&mut world, &mut player, CharacterId(1), "/swa").is_none());
    assert!(apply_swap_command(&mut world, &mut player, CharacterId(1), "/swapx").is_none());

    // No character at all in the world: `char_swap` fails, but C's caller
    // (`command.c`'s bare `char_swap(cn); return 1;`) never inspects the
    // return value, so the command is still "recognized" with no feedback
    // and no timestamp stamped.
    let result = apply_swap_command(&mut world, &mut player, CharacterId(1), "/swap")
        .expect("full word should be recognized even when the swap itself fails");
    assert_eq!(result, KeyringCommandResult::default());
    assert_eq!(player.swapped_at(), 0);
}

#[test]
fn logout_command_requires_exact_word_and_is_a_silent_no_op_when_absent() {
    let world = World::default();

    assert!(apply_logout_command(&world, CharacterId(1), "/log").is_none());
    assert!(apply_logout_command(&world, CharacterId(1), "/logoutx").is_none());
    // No character at all in the world: C's own bounds/flag checks in
    // `cmd_logout` would read out-of-range `ch[cn]` state that never
    // happens in practice (a command always comes from a live character),
    // so this port just no-ops rather than guessing.
    assert!(apply_logout_command(&world, CharacterId(1), "/logout").is_none());
}

#[test]
fn logout_command_reports_not_on_blue_square_off_rest_area() {
    let mut world = World::default();
    let actor = login_character(CharacterId(1), &login_block("Actor"), 1, 10, 10);
    assert!(world.spawn_character(actor, 10, 10));

    let result = apply_logout_command(&world, CharacterId(1), "/logout")
        .expect("logout command should be recognized");
    assert_eq!(
        result,
        KeyringCommandResult {
            messages: vec!["You are not on a blue square.".to_string()],
            ..Default::default()
        }
    );
}

#[test]
fn logout_command_requests_logout_on_blue_square() {
    let mut world = World::default();
    let actor = login_character(CharacterId(1), &login_block("Actor"), 1, 10, 10);
    assert!(world.spawn_character(actor, 10, 10));
    world.map.set_flags(10, 10, MapFlags::RESTAREA);

    let result = apply_logout_command(&world, CharacterId(1), "/logout")
        .expect("logout command should be recognized");
    assert_eq!(
        result,
        KeyringCommandResult {
            logout_requested: true,
            ..Default::default()
        }
    );
}

#[test]
fn wimp_command_emits_non_live_quest_fallback() {
    let result = apply_wimp_command("/wimp").expect("wimp command should be recognized");
    assert_eq!(
        result.messages,
        vec!["You're not in the live quest area. You'll have to wimp out on your own here... That means: RUN!".to_string()]
    );

    assert!(apply_wimp_command("/wim").is_none());
    assert!(apply_wimp_command("/wimpx").is_none());
}

#[test]
fn autoturn_command_toggles_lostcon_flag_and_shows_status() {
    let character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    let mut player = PlayerRuntime::connected(1, 0);

    let enabled = apply_autoturn_command(&character, &mut player, "/autot")
        .expect("legacy autoturn abbreviation should be recognized");
    assert!(player.autoturn_enabled);
    assert!(enabled
        .messages
        .contains(&"Automatic Turning [/AUTOTURN]: On.".to_string()));
    assert_eq!(enabled.messages[0], "Lag Control Settings:");

    let disabled = apply_autoturn_command(&character, &mut player, "/autoturn")
        .expect("autoturn command should be recognized");
    assert!(!player.autoturn_enabled);
    assert!(disabled
        .messages
        .contains(&"Automatic Turning [/AUTOTURN]: Off.".to_string()));

    assert!(apply_autoturn_command(&character, &mut player, "/auto").is_none());
    assert!(apply_autoturn_command(&character, &mut player, "/autoturnx").is_none());
}

#[test]
fn lag_command_toggles_artificial_lag_with_legacy_feedback() {
    let mut world = World::default();
    world.map = ugaris_core::map::MapGrid::new(20, 20);
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.x = 10;
    character.y = 10;
    world.add_character(character);
    let player = PlayerRuntime::connected(1, 0);

    let enabled = apply_lag_command(&mut world, &player, character_id, "/lag")
        .expect("lag command should be recognized");
    assert!(world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::LAG));
    assert_eq!(
        enabled.messages,
        vec![
            "Turned artificial lag on.".to_string(),
            "PLEASE turn this option off (type /lag again) before you complain about lag!"
                .to_string(),
        ]
    );

    let disabled = apply_lag_command(&mut world, &player, character_id, "/lag")
        .expect("lag command should be recognized");
    assert!(!world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::LAG));
    assert_eq!(
        disabled.messages,
        vec!["Turned artificial lag off.".to_string()]
    );
    assert!(apply_lag_command(&mut world, &player, character_id, "/la").is_none());
}

#[test]
fn lag_command_blocks_enabling_in_arena_or_with_hate() {
    let mut world = World::default();
    world.map = ugaris_core::map::MapGrid::new(20, 20);
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.x = 10;
    character.y = 10;
    world.add_character(character);
    world.map.set_flags(10, 10, MapFlags::ARENA);
    let mut player = PlayerRuntime::connected(1, 0);

    let arena = apply_lag_command(&mut world, &player, character_id, "/lag")
        .expect("lag command should be recognized");
    assert_eq!(arena.messages, vec!["You cannot simulate lag in an arena."]);
    assert!(!world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::LAG));

    world.map.set_flags(10, 10, MapFlags::empty());
    assert!(player.add_pk_hate(99));
    let hate = apply_lag_command(&mut world, &player, character_id, "/lag")
        .expect("lag command should be recognized");
    assert_eq!(
        hate.messages,
        vec!["You cannot simulate lag while your hate list is not empty."]
    );
}

#[test]
fn lag_control_toggle_command_flips_field_and_shows_status() {
    let character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    let mut player = PlayerRuntime::connected(1, 0);

    let enabled = apply_lag_control_toggle_command(&character, &mut player, "/noball")
        .expect("noball command should be recognized");
    assert!(player.no_ball);
    assert_eq!(enabled.messages[0], "Lag Control Settings:");

    let disabled = apply_lag_control_toggle_command(&character, &mut player, "/noball")
        .expect("noball command should be recognized");
    assert!(!player.no_ball);
    let _ = disabled;
}

#[test]
fn lag_control_toggle_command_covers_every_family_member_with_legacy_minlen() {
    let character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    let mut player = PlayerRuntime::connected(1, 0);

    let cases: &[(&str, fn(&PlayerRuntime) -> bool)] = &[
        ("noball", |p| p.no_ball),
        ("nobless", |p| p.no_bless),
        ("nofireball", |p| p.no_fireball),
        ("noflash", |p| p.no_flash),
        ("nofreeze", |p| p.no_freeze),
        ("noheal", |p| p.no_heal),
        ("noshield", |p| p.no_shield),
        ("nowarcry", |p| p.no_warcry),
        ("nolife", |p| p.no_life),
        ("nomana", |p| p.no_mana),
        ("nocombo", |p| p.no_combo),
        ("nomove", |p| p.no_move),
        ("norecall", |p| p.no_recall),
        ("nopulse", |p| p.no_pulse),
        ("autobless", |p| p.autobless_enabled),
        ("autopulse", |p| p.autopulse_enabled),
    ];

    for (command, field) in cases {
        assert!(!field(&player), "{command} should start off");
        let full = format!("/{command}");
        apply_lag_control_toggle_command(&character, &mut player, &full)
            .unwrap_or_else(|| panic!("{command} should be recognized"));
        assert!(field(&player), "{command} should be toggled on");

        // The legacy `minlen` for every member of this family is 5
        // (`command.c:9397-9591`): a 5-char prefix must match, a 4-char
        // (or shorter) prefix must not.
        let short_prefix = &command[..4];
        let short = format!("/{short_prefix}");
        assert!(
            apply_lag_control_toggle_command(&character, &mut player, &short).is_none(),
            "{short} (4-char prefix) should not match"
        );

        // toggle back off via the full word for the next iteration.
        apply_lag_control_toggle_command(&character, &mut player, &full)
            .unwrap_or_else(|| panic!("{command} should be recognized"));
        assert!(!field(&player), "{command} should be toggled back off");
    }
}

#[test]
fn allowbless_command_toggles_nobless_flag_and_shows_status() {
    let mut world = World::default();
    world.map = ugaris_core::map::MapGrid::new(20, 20);
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);
    let player = PlayerRuntime::connected(1, 0);

    let result = apply_allowbless_command(&mut world, &player, character_id, "/allowbless")
        .expect("allowbless command should be recognized");
    assert!(world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::NOBLESS));
    assert!(result
        .messages
        .contains(&"Allow others to bless me [/ALLOWBLESS]: No.".to_string()));

    let result = apply_allowbless_command(&mut world, &player, character_id, "/allowbless")
        .expect("allowbless command should be recognized");
    assert!(!world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::NOBLESS));
    assert!(result
        .messages
        .contains(&"Allow others to bless me [/ALLOWBLESS]: Yes.".to_string()));

    assert!(apply_allowbless_command(&mut world, &player, character_id, "/allo").is_none());
}

/// C `cmdcmp(ptr, "lastseen", 4)`: any prefix from `"last"` up to the full
/// word matches case-insensitively; anything shorter (or a different
/// word entirely) is not recognized at all.
#[test]
fn lastseen_command_recognizes_legacy_abbreviations_and_rejects_short_prefixes() {
    let mut world = World::default();
    let character_id = CharacterId(7);

    assert!(apply_lastseen_command(&mut world, character_id, "/lastseen Godmode").is_some());
    assert!(apply_lastseen_command(&mut world, character_id, "/LAST Godmode").is_some());
    assert!(apply_lastseen_command(&mut world, character_id, "/lastse Godmode").is_some());
    assert!(apply_lastseen_command(&mut world, character_id, "/las Godmode").is_none());
    assert!(apply_lastseen_command(&mut world, character_id, "/lag").is_none());
}

/// A valid-looking name is queued for the async DB round-trip (see
/// `World::queue_lastseen_lookup`), producing no immediate reply.
#[test]
fn lastseen_command_queues_valid_names_without_an_immediate_reply() {
    let mut world = World::default();
    let character_id = CharacterId(7);

    let result = apply_lastseen_command(&mut world, character_id, "/lastseen Godmode")
        .expect("lastseen command should be recognized");
    assert!(result.messages.is_empty());

    let queued = world.drain_pending_lastseen_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].requester_id, character_id);
    assert_eq!(queued[0].target_name, "Godmode");
    assert!(world.drain_pending_system_texts().is_empty());
}

/// Only leading whitespace is trimmed (`while (isspace(*ptr)) ptr++;`,
/// `command.c:9033-9035`) - an invalidly-shaped name (empty, or one
/// containing a non-alphabetic byte) is answered immediately via
/// `World::queue_system_text` instead of being queued, matching C's
/// synchronous `lookup_name` `-1` fast path.
#[test]
fn lastseen_command_with_no_argument_replies_immediately() {
    let mut world = World::default();
    let character_id = CharacterId(7);

    let result = apply_lastseen_command(&mut world, character_id, "/lastseen")
        .expect("lastseen command should be recognized");
    assert!(result.messages.is_empty());
    assert!(world.drain_pending_lastseen_lookups().is_empty());

    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].character_id, character_id);
    assert_eq!(texts[0].message, "No character by the name .");
}

#[test]
fn description_command_sanitizes_and_reports_legacy_text() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    world.add_character(login_character(
        character_id,
        &login_block("Tester"),
        1,
        10,
        10,
    ));

    let result = apply_description_command(
        &mut world,
        character_id,
        "/desc I am \"great\" and 100% ready",
    )
    .expect("description minlen prefix should be recognized");

    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.description, "I am 'great' and 100  ready");
    assert_eq!(
        result.messages,
        vec!["Your description reads now: I am 'great' and 100  ready"]
    );
}

#[test]
fn description_command_preserves_empty_and_truncation_guards() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    world.add_character(login_character(
        character_id,
        &login_block("Tester"),
        1,
        10,
        10,
    ));

    let empty = apply_description_command(&mut world, character_id, "/description   ")
        .expect("description command should be recognized");
    assert_eq!(empty.messages, vec!["Sorry, you need to enter some text."]);
    assert!(world
        .characters
        .get(&character_id)
        .unwrap()
        .description
        .is_empty());

    let long_text = "x".repeat(200);
    apply_description_command(
        &mut world,
        character_id,
        &format!("/description {long_text}"),
    )
    .expect("description command should be recognized");
    assert_eq!(
        world
            .characters
            .get(&character_id)
            .unwrap()
            .description
            .len(),
        159
    );
    assert!(apply_description_command(&mut world, character_id, "/de Text").is_none());
    assert!(apply_description_command(&mut world, character_id, "/descriptionx Text").is_none());
}

#[test]
fn time_command_preserves_legacy_showtime_output_and_prefix_gate() {
    let date = GameDate::calculate(ugaris_core::game_time::START_TIME, 1, None);

    let result = apply_time_command(date, "/ti").expect("legacy cmdcmp accepts /ti");

    assert_eq!(
        result.messages,
        vec![
            "It's 00:00 on the 1/1/0. Sunrise is at 07:30, sunset at 16:30. Moonrise is at 06:00, moonset at 18:00.",
            "Be careful, New Moon tonight!",
            "It's a scary day, it's Winter Solstice today!",
            "Next full moon is in 14 days.",
            "Spring Equinox will be in 90 days.",
        ]
    );
    assert!(apply_time_command(date, "/t").is_none());
    assert!(apply_time_command(date, "/timex").is_none());
}

#[test]
fn time_command_reports_legacy_moon_phase_and_next_events() {
    let date = GameDate::calculate(
        ugaris_core::game_time::START_TIME + ugaris_core::game_time::DAY_LEN * 7,
        1,
        None,
    );

    let result = apply_time_command(date, "/time").expect("time command should match");

    assert!(result.messages.contains(&"Half Moon.".to_string()));
    assert!(result
        .messages
        .contains(&"Next full moon is in 7 days.".to_string()));
    assert!(result
        .messages
        .contains(&"Spring Equinox will be in 83 days.".to_string()));
}

#[test]
fn alias_command_create_list_replace_delete_and_clear_match_legacy_feedback() {
    let mut player = PlayerRuntime::connected(1, 0);

    let created = apply_alias_command(&mut player, "/alias thanks Thank you!")
        .expect("alias command should be recognized");
    let listed =
        apply_alias_command(&mut player, "/alias").expect("alias list should be recognized");
    let replaced = apply_alias_command(&mut player, "/alias thanks Thanks!")
        .expect("alias replacement should be recognized");
    let erased = apply_alias_command(&mut player, "/alias thanks")
        .expect("alias deletion should be recognized");
    let missing = apply_alias_command(&mut player, "/alias thanks")
        .expect("missing alias deletion should be recognized");
    let cleared = apply_alias_command(&mut player, "/clearaliases")
        .expect("clearaliases should be recognized");

    assert_eq!(created.messages, vec!["Created thanks -> Thank you!."]);
    assert_eq!(listed.messages, vec!["thanks -> Thank you!"]);
    assert_eq!(replaced.messages, vec!["Replaced thanks -> Thanks!."]);
    assert_eq!(erased.messages, vec!["Erased thanks -> Thanks!."]);
    assert_eq!(
        missing.messages,
        vec!["Alias thanks not found, could not delete."]
    );
    assert_eq!(cleared.messages, vec!["Done. All gone now."]);
}

#[test]
fn alias_command_truncates_legacy_from_and_to_lengths() {
    let mut player = PlayerRuntime::connected(1, 0);
    let result = apply_alias_command(
        &mut player,
        "/alias abcdefghijklmnopqrstuvwxyz 123456789012345678901234567890123456789012345678901234567890",
    )
    .expect("alias command should be recognized");

    assert_eq!(player.aliases[0].from, "abcdefg");
    assert_eq!(player.aliases[0].to.len(), 55);
    assert_eq!(
        result.messages[0],
        format!("Created abcdefg -> {}.", player.aliases[0].to)
    );
    assert!(apply_alias_command(&mut player, "/al abc").is_some());
    assert!(apply_alias_command(&mut player, "/a abc").is_none());
}

#[test]
fn help_command_includes_legacy_pk_security_lines() {
    let result = apply_help_command("/help", CharacterFlags::empty(), 1)
        .expect("help command should be recognized");

    assert_eq!(result.messages[0], "=== PLAYER COMMANDS ===");
    assert_eq!(
        result.message_bytes[0],
        b"\xb0c3=== PLAYER COMMANDS ===\xb0c0".to_vec()
    );
    assert!(result
        .messages
        .contains(&"== Communication Commands ==".to_string()));
    assert!(result.messages.contains(
        &"/holler <text> - Say something with very long range (costs endurance points)".to_string()
    ));
    assert!(result
        .messages
        .contains(&"/playerkiller - Toggle player killing mode on/off".to_string()));
    assert!(result
        .messages
        .contains(&"/iwilldie <id> - Confirm enabling player killer mode".to_string()));
    assert!(result
        .messages
        .contains(&"/clearhate - Clear your entire PK list at once".to_string()));
    assert!(result
        .messages
        .contains(&"== Miscellaneous Commands ==".to_string()));
    assert!(result
        .messages
        .contains(&"/help - Display this help text".to_string()));
    let help_line_index = result
        .messages
        .iter()
        .position(|message| message == "/help - Display this help text")
        .expect("help line should be present");
    assert_eq!(
        result.message_bytes[help_line_index],
        b"\xb0c4/help\xb0c0 - Display this help text".to_vec()
    );
    assert!(result.messages.contains(
        &"Type a command without parameters to get more information in some cases.".to_string()
    ));
    assert!(!result
        .messages
        .contains(&"=== STAFF COMMANDS ===".to_string()));
    assert!(apply_help_command("/hel", CharacterFlags::empty(), 1).is_none());
    assert!(!result.inventory_changed);
}

#[test]
fn color_commands_pack_legacy_rgb_words_and_report_for_gods() {
    let character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    let mut world = World::default();
    world.add_character(character);

    let changed = apply_color_command(&mut world, CharacterId(7), "/col1 1 2 3")
        .expect("col1 should be recognized");
    assert!(changed.name_changed);
    let character = world.characters.get(&CharacterId(7)).unwrap();
    assert_eq!(character.c1, (1 << 10) + (2 << 5) + 3);
    assert_eq!(character.c2, 0);
    assert_eq!(character.c3, 0);

    assert!(apply_color_command(&mut world, CharacterId(7), "/color").is_none());
    world
        .characters
        .get_mut(&CharacterId(7))
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let report = apply_color_command(&mut world, CharacterId(7), "/color")
        .expect("god color command should be recognized");
    assert_eq!(report.messages, vec!["c1=443, c2=0, c3=0"]);
}

#[test]
fn color_commands_match_c_atoi_pointer_edges() {
    let character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    let mut world = World::default();
    world.add_character(character);

    apply_color_command(&mut world, CharacterId(7), "/col2 -1 2 3")
        .expect("col2 should be recognized");
    let character = world.characters.get(&CharacterId(7)).unwrap();
    assert_eq!(character.c2, ((-1_i64 << 10) + (-1_i64 << 5) - 1) as u16);

    apply_color_command(&mut world, CharacterId(7), "/col3 12x34 5 6")
        .expect("col3 should be recognized");
    let character = world.characters.get(&CharacterId(7)).unwrap();
    assert_eq!(character.c3, 12_u16 << 10);
}

#[test]
fn runtime_refreshes_known_character_name_cache_after_color_change() {
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    let old_packet = character_name_packet(&character).to_vec();
    let mut runtime = ServerRuntime::default();
    runtime.map_caches.insert(
        11,
        VisibleMapCache {
            center_x: 10,
            center_y: 10,
            view_distance: 8,
            cells: HashMap::new(),
            known_character_names: HashMap::from([(7, old_packet.clone())]),
        },
    );

    character.c1 = 0x0443;
    let pk_relations = PkRelationSnapshot::default();
    let sessions =
        runtime.refresh_known_character_name(&World::default(), &pk_relations, &character);

    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].0, 11);
    assert_ne!(
        runtime
            .map_caches
            .get(&11)
            .unwrap()
            .known_character_names
            .get(&7)
            .unwrap(),
        &old_packet
    );
}

#[test]
fn chest_helpers_decode_legacy_driver_data() {
    let mut chest = test_item(ItemId(10), 700, ItemFlags::USE);
    chest.driver_data = vec![9, 0x44, 0x33, 0x22, 0x11, 2, 0, 3];

    assert_eq!(chest_required_key_id(&chest), 0x1122_3344);
    assert_eq!(chest_timeout_seconds(&chest), 2 * 60 * 60);
    assert_eq!(chest_required_deaths(&chest), 3);
}

/// Connects a level-`level` player under `character_id`, mirroring
/// `achievement.rs` tests' `connected_god` helper minus the `CF_GOD` flag
/// (`/demonlords` has no permission gate in C).
fn connected_player_at_level(character_id: CharacterId, level: u32) -> (World, ServerRuntime) {
    let mut world = World::default();
    let mut character = login_character(character_id, &login_block("Hero"), 1, 10, 10);
    character.level = level;
    world.add_character(character);
    let mut runtime = ServerRuntime::default();
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(1, commands, 0);
    if let Some(player) = runtime.players.get_mut(&1) {
        player.character_id = Some(character_id);
    }
    (world, runtime)
}

/// C `cmdcmp(ptr, "demonlords", 10)`: since `minlen` equals the full word
/// length, only the exact word (case-insensitively) matches - no
/// abbreviation is accepted, unlike most other commands in this file.
#[test]
fn demonlords_command_requires_the_full_word_and_ignores_case() {
    let (world, runtime) = connected_player_at_level(CharacterId(1), 20);

    assert!(apply_demonlords_command(&world, &runtime, CharacterId(1), "/demonlords").is_some());
    assert!(apply_demonlords_command(&world, &runtime, CharacterId(1), "/DEMONLORDS").is_some());
    assert!(apply_demonlords_command(&world, &runtime, CharacterId(1), "/demonlord").is_none());
    assert!(apply_demonlords_command(&world, &runtime, CharacterId(1), "/demon").is_none());
    assert!(apply_demonlords_command(&world, &runtime, CharacterId(1), "/time").is_none());
}

/// C `cmd_demonlords` (`command.c:1414-1423`): with zero demon lords ever
/// killed, the only output is the light-red "not yet vanquished" line.
#[test]
fn demonlords_command_reports_no_kills_message() {
    let (world, runtime) = connected_player_at_level(CharacterId(1), 20);

    let result = apply_demonlords_command(&world, &runtime, CharacterId(1), "/demonlords")
        .expect("demonlords command should be recognized");
    assert_eq!(result.message_bytes.len(), 1);
    let mut expected = Vec::new();
    expected.extend_from_slice(COL_LIGHT_RED);
    expected.extend_from_slice(b"Thou hast not yet vanquished any demon lords, brave adventurer.");
    expected.extend_from_slice(COL_RESET);
    assert_eq!(result.message_bytes[0], expected);
}

/// C `cmd_demonlords` (`command.c:1425-1461`): once at least one demon
/// lord is killed, the header line plus grouped-by-3 rows are shown, gated
/// on `demon_lords[i].level <= player_level + 10`; killed lords render
/// `COL_VIOLET`, unkilled render `COL_LIGHT_RED`, and every group of 3
/// gets a trailing `\n` appended to the *same* line (`strncat(demon_buf,
/// "\n", ...)` before the flushing `log_char`).
#[test]
fn demonlords_command_lists_killed_and_available_lords_grouped_by_three() {
    let (world, mut runtime) = connected_player_at_level(CharacterId(1), 20);
    // Level 20 -> shows lords with level <= 30: classes 258..=269 (12
    // lords, levels 8..=30 step 2), grouped into 4 rows of 3.
    runtime
        .player_for_character_mut(CharacterId(1))
        .unwrap()
        .mark_first_kill(258); // Earth Demon Lord 8

    let result = apply_demonlords_command(&world, &runtime, CharacterId(1), "/demonlords")
        .expect("demonlords command should be recognized");

    let mut expected_header = Vec::new();
    expected_header.extend_from_slice(COL_ORANGE);
    expected_header.extend_from_slice(b"Demon Lords status:");
    expected_header.extend_from_slice(COL_RESET);
    assert_eq!(result.message_bytes[0], expected_header);

    // 12 eligible lords -> 4 full rows of 3, no leftover partial row.
    assert_eq!(result.message_bytes.len(), 1 + 4);

    let mut expected_row1 = Vec::new();
    expected_row1.extend_from_slice(COL_VIOLET);
    expected_row1.extend_from_slice(b"Earth Demon Lord 8");
    expected_row1.extend_from_slice(COL_RESET);
    expected_row1.push(b' ');
    expected_row1.extend_from_slice(COL_LIGHT_RED);
    expected_row1.extend_from_slice(b"Earth Demon Lord 10");
    expected_row1.extend_from_slice(COL_RESET);
    expected_row1.push(b' ');
    expected_row1.extend_from_slice(COL_LIGHT_RED);
    expected_row1.extend_from_slice(b"Earth Demon Lord 12");
    expected_row1.extend_from_slice(COL_RESET);
    expected_row1.push(b' ');
    expected_row1.push(b'\n');
    assert_eq!(result.message_bytes[1], expected_row1);

    let last_row = result.message_bytes.last().unwrap();
    assert!(last_row.ends_with(b"\n"));
}

/// C `cmd_demonlords`'s `if ((i + 1) % 3 == 0)` grouping leaves a shorter
/// final row (no trailing `\n`) when the eligible-lord count isn't a
/// multiple of 3.
#[test]
fn demonlords_command_flushes_a_short_final_row_without_trailing_newline() {
    // Level 6 -> only lords with level <= 16 are eligible: classes
    // 258..=262 (5 lords, levels 8/10/12/14/16) - not a multiple of 3, so
    // the final row (2 entries) has no trailing newline.
    let (world, mut runtime) = connected_player_at_level(CharacterId(1), 6);
    runtime
        .player_for_character_mut(CharacterId(1))
        .unwrap()
        .mark_first_kill(260);

    let result = apply_demonlords_command(&world, &runtime, CharacterId(1), "/demonlords")
        .expect("demonlords command should be recognized");

    // 5 eligible lords -> 1 full row of 3 + 1 partial row of 2.
    assert_eq!(result.message_bytes.len(), 1 + 2);
    let last_row = result.message_bytes.last().unwrap();
    assert!(!last_row.ends_with(b"\n"));
    assert!(last_row.ends_with(b"\xb0c0 "));
}

/// A disconnected/never-logged-in character has no `PlayerRuntime`, so the
/// command falls through (`None`) exactly like every other self-query
/// command in this file when `runtime.player_for_character` misses.
#[test]
fn demonlords_command_falls_through_without_a_live_player_runtime() {
    let world = World::default();
    let runtime = ServerRuntime::default();
    assert!(apply_demonlords_command(&world, &runtime, CharacterId(1), "/demonlords").is_none());
}
