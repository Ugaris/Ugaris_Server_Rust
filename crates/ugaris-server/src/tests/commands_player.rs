use super::*;
use ugaris_core::player::OrbSpawnAccess;

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

/// C `cmdcmp(ptr, "complain", 4)`: any prefix from `"comp"` up to the full
/// word matches case-insensitively; anything shorter is not recognized.
#[test]
fn complain_command_recognizes_legacy_abbreviations_and_rejects_short_prefixes() {
    let mut world = World::default();
    let mut player = PlayerRuntime::connected(1, 0);
    let character_id = CharacterId(7);

    assert!(apply_complain_command(
        &mut world,
        &mut player,
        character_id,
        "/complain Someone",
        false,
        1_000
    )
    .is_some());
    assert!(apply_complain_command(
        &mut world,
        &mut player,
        character_id,
        "/COMP Someone",
        false,
        1_000
    )
    .is_some());
    assert!(apply_complain_command(
        &mut world,
        &mut player,
        character_id,
        "/com Someone",
        false,
        1_000
    )
    .is_none());
}

/// Empty argument: the "need at least the name" message, no PPD write
/// (C `command.c:2292-2296`).
#[test]
fn complain_command_with_no_argument_replies_immediately_without_touching_the_ppd() {
    let mut world = World::default();
    let mut player = PlayerRuntime::connected(1, 0);
    let character_id = CharacterId(7);

    let result = apply_complain_command(
        &mut world,
        &mut player,
        character_id,
        "/complain",
        false,
        1_000,
    )
    .expect("complain command should be recognized");
    assert_eq!(
        result.messages,
        vec![
            "Sorry, you need to enter at least the name of the player you're complaining about."
                .to_string()
        ]
    );
    assert_eq!(player.complaint_date(), 0);
}

/// First-ever `/complain`: the one-time `COL_LIGHT_RED` disclaimer,
/// stamping `complaint_date = 1` (C `command.c:2298-2308`).
#[test]
fn complain_command_shows_the_one_time_disclaimer_and_stamps_complaint_date_to_one() {
    let mut world = World::default();
    let mut player = PlayerRuntime::connected(1, 0);
    let character_id = CharacterId(7);

    let result = apply_complain_command(
        &mut world,
        &mut player,
        character_id,
        "/complain Someone",
        false,
        1_000,
    )
    .expect("complain command should be recognized");
    assert!(result.messages.is_empty());
    assert_eq!(result.message_bytes.len(), 1);
    assert_eq!(
        result.message_bytes[0],
        legacy_light_red_text_bytes(
            "Complaints are meant as a way to complain about verbal attacks by another \
             player, or to report a scam. If you wish to complain about something else, \
             please email game@ugaris.com. No complaint has been sent. Repeat the command \
             if you still want to send your complaint."
        )
    );
    assert_eq!(player.complaint_date(), 1);
    assert!(world.drain_pending_complain_lookups().is_empty());
}

/// A non-`CF_GOD` caller retrying within 60 seconds is rate-limited, and
/// - a genuine C quirk - `complaint_date` is restamped to the rejected
/// attempt's own timestamp (`command.c:2306-2309`), not left alone.
#[test]
fn complain_command_rate_limits_non_god_callers_and_restamps_on_rejection() {
    let mut world = World::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.record_complaint(1_000);
    let character_id = CharacterId(7);

    let result = apply_complain_command(
        &mut world,
        &mut player,
        character_id,
        "/complain Someone",
        false,
        1_030,
    )
    .expect("complain command should be recognized");
    assert_eq!(
        result.messages,
        vec!["Sorry, we do not accept more than one complaint per minute.".to_string()]
    );
    assert_eq!(player.complaint_date(), 1_030);
}

/// `CF_GOD` callers bypass the rate limit entirely (C `command.c:2305`'s
/// `!(ch[cn].flags & CF_GOD)` guard).
#[test]
fn complain_command_exempts_god_callers_from_the_rate_limit() {
    let mut world = World::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.record_complaint(1_000);
    let character_id = CharacterId(7);

    let result = apply_complain_command(
        &mut world,
        &mut player,
        character_id,
        "/complain Someone",
        true,
        1_005,
    )
    .expect("complain command should be recognized");
    assert!(result.messages.is_empty());
    let queued = world.drain_pending_complain_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].target_name, "Someone");
}

/// The `"lag"`/`"laggy"` name blocklist (`command.c:2332-2335`) - a
/// distinct message from the generic not-found rejection, no PPD write.
#[test]
fn complain_command_rejects_lag_complaints_with_a_dedicated_message() {
    let mut world = World::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.record_complaint(1_000);
    let character_id = CharacterId(7);

    let result = apply_complain_command(
        &mut world,
        &mut player,
        character_id,
        "/complain laggy",
        false,
        2_000,
    )
    .expect("complain command should be recognized");
    assert_eq!(
        result.messages,
        vec![
            "Sorry, the complaint command is meant to complain about players, not lag.".to_string()
        ]
    );
    assert!(world.drain_pending_complain_lookups().is_empty());
}

/// The generic-word blocklist (`command.c:2336-2339`) - these common
/// English words parse as a plausible-looking alpha "name" but are
/// rejected before ever reaching the DB lookup.
#[test]
fn complain_command_rejects_generic_words_as_names() {
    let mut world = World::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.record_complaint(1_000);
    let character_id = CharacterId(7);

    let result = apply_complain_command(
        &mut world,
        &mut player,
        character_id,
        "/complain why did you do that",
        false,
        2_000,
    )
    .expect("complain command should be recognized");
    assert_eq!(
        result.messages,
        vec!["Sorry, no player by the name 'why' found.".to_string()]
    );
    assert!(world.drain_pending_complain_lookups().is_empty());
}

/// A plausible name is queued for the async DB round-trip (see
/// `World::queue_complain_lookup`), producing no immediate reply.
#[test]
fn complain_command_queues_valid_names_without_an_immediate_reply() {
    let mut world = World::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.record_complaint(1_000);
    let character_id = CharacterId(7);

    let result = apply_complain_command(
        &mut world,
        &mut player,
        character_id,
        "/complain Godmode being a jerk",
        false,
        2_000,
    )
    .expect("complain command should be recognized");
    assert!(result.messages.is_empty());

    let queued = world.drain_pending_complain_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].requester_id, character_id);
    assert_eq!(queued[0].target_name, "Godmode");
    assert!(world.drain_pending_system_texts().is_empty());
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

/// C `cmdcmp(ptr, "orbs", 4)`: `minlen` equals the full word length, so
/// only the exact word (case-insensitively) matches.
#[test]
fn orbs_command_requires_the_full_word_and_ignores_case() {
    let (world, runtime) = connected_player_at_level(CharacterId(1), 30);

    assert!(apply_orbs_command(&world, &runtime, CharacterId(1), "/orbs").is_some());
    assert!(apply_orbs_command(&world, &runtime, CharacterId(1), "/ORBS").is_some());
    assert!(apply_orbs_command(&world, &runtime, CharacterId(1), "/orb").is_none());
    assert!(apply_orbs_command(&world, &runtime, CharacterId(1), "/time").is_none());
}

/// C `command.c:8905-8917`'s dispatcher gate: below `exp == 81000` (level
/// 30), the caller gets a plain, uncolored rejection and `cmd_orbs` never
/// runs.
#[test]
fn orbs_command_below_level_30_reports_plain_rejection() {
    let mut world = World::default();
    let character = login_character(CharacterId(1), &login_block("Hero"), 1, 10, 10);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(1, commands, 0);
    runtime.players.get_mut(&1).unwrap().character_id = Some(CharacterId(1));

    let result = apply_orbs_command(&world, &runtime, CharacterId(1), "/orbs")
        .expect("orbs command should be recognized");
    assert_eq!(
        result.messages,
        vec![
            "Thou hast to reach level 30 to fathom understanding the mysteries of the orbs and their timers."
                .to_string()
        ]
    );
    assert!(result.message_bytes.is_empty());
}

/// C `cmd_orbs` (`command.c:1517-1521`): a level-30+ caller with zero
/// discovered orbs gets only the `COL_LIGHT_RED` "not yet discovered" line.
#[test]
fn orbs_command_reports_no_orbs_discovered() {
    let mut world = World::default();
    let mut character = login_character(CharacterId(1), &login_block("Hero"), 1, 10, 10);
    character.exp = 81000;
    world.add_character(character);
    let mut runtime = ServerRuntime::default();
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(1, commands, 0);
    runtime.players.get_mut(&1).unwrap().character_id = Some(CharacterId(1));

    let result = apply_orbs_command(&world, &runtime, CharacterId(1), "/orbs")
        .expect("orbs command should be recognized");
    assert_eq!(result.message_bytes.len(), 1);
    let mut expected = Vec::new();
    expected.extend_from_slice(COL_LIGHT_RED);
    expected.extend_from_slice(b"Ye have not yet discovered any orbs, brave adventurer.");
    expected.extend_from_slice(COL_RESET);
    assert_eq!(result.message_bytes[0], expected);
}

/// C `cmd_orbs` (`command.c:1522-1559`): one ready orb (elapsed time at or
/// past `base_orb_respawn_time_days`) and one pending orb, plus a summary
/// line whose average only counts the not-yet-ready orbs.
#[test]
fn orbs_command_lists_ready_and_pending_orbs_with_summary() {
    let mut world = World::default();
    let mut character = login_character(CharacterId(1), &login_block("Hero"), 1, 10, 10);
    character.exp = 81000;
    world.add_character(character);
    assert_eq!(world.settings.base_orb_respawn_time_days, 30);
    let realtime_seconds = 100u64 * 24 * 60 * 60;
    world.tick = ugaris_core::Tick(TICKS_PER_SECOND * realtime_seconds);

    let mut runtime = ServerRuntime::default();
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(1, commands, 0);
    let player = runtime.players.get_mut(&1).unwrap();
    player.character_id = Some(CharacterId(1));
    // Ready: last used at time 0, 100 days elapsed >= 30-day respawn.
    player.orb_spawns.push(OrbSpawnAccess {
        location_id: 10 + (20 << 8) + (1 << 16),
        last_used_seconds: 0,
    });
    // Pending: used 10 days ago, 20 days remain.
    player.orb_spawns.push(OrbSpawnAccess {
        location_id: 30 + (40 << 8) + (3 << 16),
        last_used_seconds: realtime_seconds - 10 * 24 * 60 * 60,
    });

    let result = apply_orbs_command(&world, &runtime, CharacterId(1), "/orbs")
        .expect("orbs command should be recognized");
    // Header + 2 orb lines + summary.
    assert_eq!(result.message_bytes.len(), 4);

    let mut expected_ready = Vec::new();
    expected_ready.extend_from_slice(b"Orb at ");
    expected_ready.extend_from_slice(COL_ORANGE);
    expected_ready.extend_from_slice(b"(10, 20)");
    expected_ready.extend_from_slice(COL_RESET);
    expected_ready.extend_from_slice(b" in ");
    expected_ready.extend_from_slice(COL_VIOLET);
    expected_ready.extend_from_slice(b"Cameron");
    expected_ready.extend_from_slice(COL_RESET);
    expected_ready.extend_from_slice(b" - ");
    expected_ready.extend_from_slice(COL_YELLOW);
    expected_ready.extend_from_slice(b"Ready to grab!");
    expected_ready.extend_from_slice(COL_RESET);
    assert_eq!(result.message_bytes[1], expected_ready);

    let mut expected_pending = Vec::new();
    expected_pending.extend_from_slice(b"Orb at ");
    expected_pending.extend_from_slice(COL_ORANGE);
    expected_pending.extend_from_slice(b"(30, 40)");
    expected_pending.extend_from_slice(COL_RESET);
    expected_pending.extend_from_slice(b" in ");
    expected_pending.extend_from_slice(COL_VIOLET);
    expected_pending.extend_from_slice(b"Aston");
    expected_pending.extend_from_slice(COL_RESET);
    expected_pending.extend_from_slice(b" - Ready in ");
    expected_pending.extend_from_slice(COL_LIGHT_RED);
    expected_pending.extend_from_slice(b" 20 days");
    expected_pending.extend_from_slice(COL_RESET);
    assert_eq!(result.message_bytes[2], expected_pending);

    let mut expected_summary = Vec::new();
    expected_summary.extend_from_slice(COL_ORANGE);
    expected_summary.extend_from_slice(b"Summary:");
    expected_summary.extend_from_slice(COL_RESET);
    expected_summary.extend_from_slice(b" 2 orbs total, ");
    expected_summary.extend_from_slice(COL_YELLOW);
    expected_summary.extend_from_slice(b" 1 ready ");
    expected_summary.extend_from_slice(COL_RESET);
    expected_summary.extend_from_slice(b", Average spawn time: ");
    expected_summary.extend_from_slice(COL_LIGHT_RED);
    expected_summary.extend_from_slice(b" 20.0 days");
    expected_summary.extend_from_slice(COL_RESET);
    assert_eq!(result.message_bytes[3], expected_summary);
}

/// A disconnected/never-logged-in character has no `PlayerRuntime`, so the
/// command falls through (`None`).
#[test]
fn orbs_command_falls_through_without_a_live_player_runtime() {
    let world = World::default();
    let runtime = ServerRuntime::default();
    assert!(apply_orbs_command(&world, &runtime, CharacterId(1), "/orbs").is_none());
}

/// C `cmdcmp(ptr, "treasures", 9)`: an exact case-insensitive word match.
#[test]
fn treasures_command_requires_the_full_word_and_ignores_case() {
    let (world, runtime) = connected_player_at_level(CharacterId(1), 1);

    assert!(apply_treasures_command(&world, &runtime, CharacterId(1), "/treasures").is_some());
    assert!(apply_treasures_command(&world, &runtime, CharacterId(1), "/TREASURES").is_some());
    assert!(apply_treasures_command(&world, &runtime, CharacterId(1), "/treasure").is_none());
    assert!(apply_treasures_command(&world, &runtime, CharacterId(1), "/time").is_none());
}

/// C `cmd_treasure` (`command.c:1570-1704`): with nothing discovered, only
/// the header and a `0 discovered, 0 ready` summary are shown.
#[test]
fn treasures_command_reports_zero_discovered_summary() {
    let (world, runtime) = connected_player_at_level(CharacterId(1), 1);

    let result = apply_treasures_command(&world, &runtime, CharacterId(1), "/treasures")
        .expect("treasures command should be recognized");
    assert_eq!(result.message_bytes.len(), 2);

    let mut expected_header = Vec::new();
    expected_header.extend_from_slice(COL_ORANGE);
    expected_header.extend_from_slice(b"Treasures Status (only shows treasures ye have found):");
    expected_header.extend_from_slice(COL_RESET);
    assert_eq!(result.message_bytes[0], expected_header);

    let mut expected_summary = Vec::new();
    expected_summary.extend_from_slice(COL_ORANGE);
    expected_summary.extend_from_slice(b"Summary:");
    expected_summary.extend_from_slice(COL_RESET);
    expected_summary.extend_from_slice(b" 0 treasures discovered, ");
    expected_summary.extend_from_slice(COL_YELLOW);
    expected_summary.extend_from_slice(b" 0 ready to loot");
    expected_summary.extend_from_slice(COL_RESET);
    assert_eq!(result.message_bytes[1], expected_summary);
}

/// C `cmd_treasure` (`command.c:1631-1670`): a ready chest, a pending
/// chest (with a `days, hours, minutes` countdown), and a ready dig spot,
/// each using the fixed 365-day respawn.
#[test]
fn treasures_command_lists_ready_and_pending_chests_and_dig_spots() {
    let (mut world, mut runtime) = connected_player_at_level(CharacterId(1), 1);
    let realtime_seconds = 400u64 * 24 * 60 * 60;
    world.tick = ugaris_core::Tick(TICKS_PER_SECOND * realtime_seconds);

    let player = runtime.player_for_character_mut(CharacterId(1)).unwrap();
    // Treasure #63 (Mines level 80 (GU)): used at second 1 (long enough
    // ago that the 365-day respawn has elapsed by day 400) -> ready. `0`
    // is reserved as the "never discovered" sentinel, so `1` stands in
    // for "long ago" here.
    player.mark_chest_access(63, 1);
    // Treasure #56 (Mines level 10 (GU)): used 10 days ago -> 355 days,
    // 0 hours, 0 minutes remain.
    player.mark_chest_access(56, realtime_seconds - 10 * 24 * 60 * 60);
    // Dig spot #0 (Dead Tree): used at second 1 -> ready to dig.
    player.mark_treasure_dig(0, 1);

    let result = apply_treasures_command(&world, &runtime, CharacterId(1), "/treasures")
        .expect("treasures command should be recognized");
    // Header + chest 56 + chest 63 + dig spot 0 + summary.
    assert_eq!(result.message_bytes.len(), 5);

    let mut expected_pending = Vec::new();
    expected_pending.extend_from_slice(COL_LIGHT_GREEN);
    expected_pending.extend_from_slice(b"Mines level 10 (GU)");
    expected_pending.extend_from_slice(b":");
    expected_pending.extend_from_slice(COL_RESET);
    expected_pending.extend_from_slice(b" ");
    expected_pending.extend_from_slice(COL_LIGHT_RED);
    expected_pending.extend_from_slice(b" 355 days, 0 hours, 0 minutes");
    expected_pending.extend_from_slice(COL_RESET);
    expected_pending.extend_from_slice(b" remain");
    assert_eq!(result.message_bytes[1], expected_pending);

    let mut expected_ready_chest = Vec::new();
    expected_ready_chest.extend_from_slice(COL_LIGHT_GREEN);
    expected_ready_chest.extend_from_slice(b"Mines level 80 (GU)");
    expected_ready_chest.extend_from_slice(b": ");
    expected_ready_chest.extend_from_slice(COL_YELLOW);
    expected_ready_chest.extend_from_slice(b"Ready!");
    expected_ready_chest.extend_from_slice(COL_RESET);
    assert_eq!(result.message_bytes[2], expected_ready_chest);

    let mut expected_ready_dig = Vec::new();
    expected_ready_dig.extend_from_slice(COL_LIGHT_GREEN);
    expected_ready_dig.extend_from_slice(b"Brannington (Forester's Quest) Dead Tree");
    expected_ready_dig.extend_from_slice(b": ");
    expected_ready_dig.extend_from_slice(COL_YELLOW);
    expected_ready_dig.extend_from_slice(b"Ready to dig!");
    expected_ready_dig.extend_from_slice(COL_RESET);
    assert_eq!(result.message_bytes[3], expected_ready_dig);

    let mut expected_summary = Vec::new();
    expected_summary.extend_from_slice(COL_ORANGE);
    expected_summary.extend_from_slice(b"Summary:");
    expected_summary.extend_from_slice(COL_RESET);
    expected_summary.extend_from_slice(b" 3 treasures discovered, ");
    expected_summary.extend_from_slice(COL_YELLOW);
    expected_summary.extend_from_slice(b" 2 ready to loot");
    expected_summary.extend_from_slice(COL_RESET);
    assert_eq!(result.message_bytes[4], expected_summary);
}

/// A disconnected/never-logged-in character has no `PlayerRuntime`, so the
/// command falls through (`None`).
#[test]
fn treasures_command_falls_through_without_a_live_player_runtime() {
    let world = World::default();
    let runtime = ServerRuntime::default();
    assert!(apply_treasures_command(&world, &runtime, CharacterId(1), "/treasures").is_none());
}

/// C `cmdcmp(ptr, "tunnel", 6)`: since `minlen` equals the full word
/// length, `/tunnel` matches exactly but `/tunnels` does not (the C
/// `cmdcmp` loop fails to match the literal's terminating null against
/// the extra trailing `s`), so the two commands are mutually exclusive.
#[test]
fn tunnel_command_requires_the_full_word_and_ignores_case() {
    let (world, runtime) = connected_player_at_level(CharacterId(1), 5);

    assert!(apply_tunnel_command(&world, &runtime, CharacterId(1), "/tunnel").is_some());
    assert!(apply_tunnel_command(&world, &runtime, CharacterId(1), "/TUNNEL").is_some());
    assert!(apply_tunnel_command(&world, &runtime, CharacterId(1), "/tunnels").is_none());
    assert!(apply_tunnel_command(&world, &runtime, CharacterId(1), "/tunn").is_none());
    assert!(apply_tunnel_command(&world, &runtime, CharacterId(1), "/time").is_none());
}

/// A disconnected/never-logged-in character has no `PlayerRuntime`, so the
/// command falls through (`None`).
#[test]
fn tunnel_command_falls_through_without_a_live_player_runtime() {
    let world = World::default();
    let runtime = ServerRuntime::default();
    assert!(apply_tunnel_command(&world, &runtime, CharacterId(1), "/tunnel").is_none());
}

/// C `cmd_tunnel` (`command.c:1722-1727`): with `gorwin_ppd::tunnel_level`
/// unset (`0`) and a character level below 20, the default level is
/// always 10, regardless of the character's actual level.
#[test]
fn tunnel_command_default_level_below_20_is_always_ten() {
    let (world, runtime) = connected_player_at_level(CharacterId(1), 5);

    let result = apply_tunnel_command(&world, &runtime, CharacterId(1), "/tunnel")
        .expect("tunnel command should be recognized");
    assert_eq!(result.message_bytes.len(), 3);

    let mut expected1 = Vec::new();
    expected1.extend_from_slice(b"Your current tunnel level is: ");
    expected1.extend_from_slice(COL_ORANGE);
    expected1.extend_from_slice(b" 10");
    expected1.extend_from_slice(COL_RESET);
    assert_eq!(result.message_bytes[0], expected1);

    let mut expected2 = Vec::new();
    expected2.extend_from_slice(b"You have completed this level ");
    expected2.extend_from_slice(COL_LIGHT_GREEN);
    expected2.extend_from_slice(b" 0");
    expected2.extend_from_slice(COL_RESET);
    expected2.extend_from_slice(b" times.");
    assert_eq!(result.message_bytes[1], expected2);

    let mut expected3 = Vec::new();
    expected3.extend_from_slice(b"You can complete this level ");
    expected3.extend_from_slice(COL_LIGHT_GREEN);
    expected3.extend_from_slice(b" 10");
    expected3.extend_from_slice(COL_RESET);
    expected3.extend_from_slice(b" more times for rewards.");
    assert_eq!(result.message_bytes[2], expected3);
}

/// C `cmd_tunnel` (`command.c:1728-1729`): for a character level in
/// `20..=100`, the default level is `char_level - 10`.
#[test]
fn tunnel_command_default_level_between_20_and_100_is_level_minus_ten() {
    let (world, mut runtime) = connected_player_at_level(CharacterId(1), 50);
    runtime.players.get_mut(&1).unwrap().set_tunnel_used(40, 3);

    let result = apply_tunnel_command(&world, &runtime, CharacterId(1), "/tunnel")
        .expect("tunnel command should be recognized");
    let mut expected1 = Vec::new();
    expected1.extend_from_slice(b"Your current tunnel level is: ");
    expected1.extend_from_slice(COL_ORANGE);
    expected1.extend_from_slice(b" 40");
    expected1.extend_from_slice(COL_RESET);
    assert_eq!(result.message_bytes[0], expected1);

    let mut expected2 = Vec::new();
    expected2.extend_from_slice(b"You have completed this level ");
    expected2.extend_from_slice(COL_LIGHT_GREEN);
    expected2.extend_from_slice(b" 3");
    expected2.extend_from_slice(COL_RESET);
    expected2.extend_from_slice(b" times.");
    assert_eq!(result.message_bytes[1], expected2);

    let mut expected3 = Vec::new();
    expected3.extend_from_slice(b"You can complete this level ");
    expected3.extend_from_slice(COL_LIGHT_GREEN);
    expected3.extend_from_slice(b" 7");
    expected3.extend_from_slice(COL_RESET);
    expected3.extend_from_slice(b" more times for rewards.");
    assert_eq!(result.message_bytes[2], expected3);
}

/// C `cmd_tunnel` (`command.c:1730-1738`): for a character level above
/// 100, search upward from 90 for the first level with zero completions,
/// stopping early once one is found - copied including the exact search
/// bound (`n < ch.level - 10 && n < 200`).
#[test]
fn tunnel_command_default_level_above_100_stops_at_first_uncompleted_level() {
    let (world, mut runtime) = connected_player_at_level(CharacterId(1), 150);
    let player = runtime.players.get_mut(&1).unwrap();
    player.set_tunnel_used(90, 1);
    // 91 left at 0 (never completed) - the search should stop there.

    let result = apply_tunnel_command(&world, &runtime, CharacterId(1), "/tunnel")
        .expect("tunnel command should be recognized");
    let mut expected1 = Vec::new();
    expected1.extend_from_slice(b"Your current tunnel level is: ");
    expected1.extend_from_slice(COL_ORANGE);
    expected1.extend_from_slice(b" 91");
    expected1.extend_from_slice(COL_RESET);
    assert_eq!(result.message_bytes[0], expected1);
}

/// C `cmd_tunnel` (`command.c:1730-1738`): if every level from 90 up to
/// the search boundary is already completed at least once, the loop
/// exhausts its condition instead of breaking, landing `current_level` on
/// `min(char_level - 10, 200)`.
#[test]
fn tunnel_command_default_level_above_100_exhausts_search_at_boundary() {
    let (world, mut runtime) = connected_player_at_level(CharacterId(1), 130);
    let player = runtime.players.get_mut(&1).unwrap();
    for level in 90..120 {
        player.set_tunnel_used(level, 1);
    }

    let result = apply_tunnel_command(&world, &runtime, CharacterId(1), "/tunnel")
        .expect("tunnel command should be recognized");
    let mut expected1 = Vec::new();
    expected1.extend_from_slice(b"Your current tunnel level is: ");
    expected1.extend_from_slice(COL_ORANGE);
    expected1.extend_from_slice(b" 120");
    expected1.extend_from_slice(COL_RESET);
    assert_eq!(result.message_bytes[0], expected1);
}

/// C `cmd_tunnel` (`command.c:1722-1723`): a non-zero
/// `gorwin_ppd::tunnel_level` is used directly, skipping the whole
/// default-level computation, then clamped to `10..=200`.
#[test]
fn tunnel_command_uses_gorwin_level_directly_and_clamps() {
    let (world, mut runtime) = connected_player_at_level(CharacterId(1), 5);
    runtime
        .players
        .get_mut(&1)
        .unwrap()
        .set_gorwin_tunnel_level(250);

    let result = apply_tunnel_command(&world, &runtime, CharacterId(1), "/tunnel")
        .expect("tunnel command should be recognized");
    let mut expected1 = Vec::new();
    expected1.extend_from_slice(b"Your current tunnel level is: ");
    expected1.extend_from_slice(COL_ORANGE);
    expected1.extend_from_slice(b" 200");
    expected1.extend_from_slice(COL_RESET);
    assert_eq!(result.message_bytes[0], expected1);
}

/// C `cmd_tunnel` (`command.c:1747-1751`): once `used[level] >=
/// MAX_TUNNEL_USES` (10), the "maxed out" line replaces the "N more
/// times" line.
#[test]
fn tunnel_command_reports_maxed_out_current_level() {
    let (world, mut runtime) = connected_player_at_level(CharacterId(1), 5);
    runtime.players.get_mut(&1).unwrap().set_tunnel_used(10, 10);

    let result = apply_tunnel_command(&world, &runtime, CharacterId(1), "/tunnel")
        .expect("tunnel command should be recognized");
    let mut expected = Vec::new();
    expected.extend_from_slice(COL_LIGHT_RED);
    expected.extend_from_slice(
        b"You have reached the maximum number of rewarded completions for this level.",
    );
    expected.extend_from_slice(COL_RESET);
    assert_eq!(result.message_bytes[2], expected);
}

/// C `cmd_tunnel` (`command.c:1755-1774`): a valid explicit level
/// argument (`10..=200`) reports its own completion count and remaining-
/// uses line as two additional messages.
#[test]
fn tunnel_command_with_valid_explicit_level_argument() {
    let (world, mut runtime) = connected_player_at_level(CharacterId(1), 5);
    runtime.players.get_mut(&1).unwrap().set_tunnel_used(75, 4);

    let result = apply_tunnel_command(&world, &runtime, CharacterId(1), "/tunnel 75")
        .expect("tunnel command should be recognized");
    assert_eq!(result.message_bytes.len(), 5);

    let mut expected4 = Vec::new();
    expected4.extend_from_slice(b"Tunnel level ");
    expected4.extend_from_slice(COL_ORANGE);
    expected4.extend_from_slice(b" 75");
    expected4.extend_from_slice(COL_RESET);
    expected4.extend_from_slice(b": completed ");
    expected4.extend_from_slice(COL_LIGHT_GREEN);
    expected4.extend_from_slice(b" 4");
    expected4.extend_from_slice(COL_RESET);
    expected4.extend_from_slice(b" times.");
    assert_eq!(result.message_bytes[3], expected4);

    let mut expected5 = Vec::new();
    expected5.extend_from_slice(b"This level can be completed ");
    expected5.extend_from_slice(COL_LIGHT_GREEN);
    expected5.extend_from_slice(b" 6");
    expected5.extend_from_slice(COL_RESET);
    expected5.extend_from_slice(b" more times for rewards.");
    assert_eq!(result.message_bytes[4], expected5);
}

/// C `cmd_tunnel` (`command.c:1770-1773`): an out-of-range explicit level
/// argument (outside `10..=200`) reports a rejection instead.
#[test]
fn tunnel_command_with_invalid_explicit_level_argument() {
    let (world, runtime) = connected_player_at_level(CharacterId(1), 5);

    let result = apply_tunnel_command(&world, &runtime, CharacterId(1), "/tunnel 300")
        .expect("tunnel command should be recognized");
    assert_eq!(result.message_bytes.len(), 4);
    let mut expected = Vec::new();
    expected.extend_from_slice(COL_LIGHT_RED);
    expected.extend_from_slice(b"Invalid tunnel level. Please choose a level between 10 and 200.");
    expected.extend_from_slice(COL_RESET);
    assert_eq!(result.message_bytes[3], expected);
}

/// C `cmdcmp(ptr, "tunnels", 7)`: exact case-insensitive word match, the
/// counterpart of `/tunnel`'s own exact-match gate.
#[test]
fn tunnellist_command_requires_the_full_word_and_ignores_case() {
    let (world, runtime) = connected_player_at_level(CharacterId(1), 5);

    assert!(apply_tunnellist_command(&world, &runtime, CharacterId(1), "/tunnels").is_some());
    assert!(apply_tunnellist_command(&world, &runtime, CharacterId(1), "/TUNNELS").is_some());
    assert!(apply_tunnellist_command(&world, &runtime, CharacterId(1), "/tunnel").is_none());
    assert!(apply_tunnellist_command(&world, &runtime, CharacterId(1), "/time").is_none());
}

/// C `cmd_tunnellist` (`command.c:1802-1809`): with zero levels ever
/// completed, only the rejection line is shown.
#[test]
fn tunnellist_command_rejects_when_nothing_completed() {
    let (world, runtime) = connected_player_at_level(CharacterId(1), 5);

    let result = apply_tunnellist_command(&world, &runtime, CharacterId(1), "/tunnels")
        .expect("tunnels command should be recognized");
    assert_eq!(result.message_bytes.len(), 1);
    let mut expected = Vec::new();
    expected.extend_from_slice(COL_LIGHT_RED);
    expected
        .extend_from_slice(b"Ye must complete at least one tunnel before thou canst check this.");
    expected.extend_from_slice(COL_RESET);
    assert_eq!(result.message_bytes[0], expected);
}

/// C `cmd_tunnellist` (`command.c:1811-1832`): lists every level from 10
/// up to `max(highest_completed, max(10, char_level - 10))`, coloring
/// each entry violet (maxed), light red (partial), or plain (untouched).
#[test]
fn tunnellist_command_lists_status_colored_by_completion() {
    let (world, mut runtime) = connected_player_at_level(CharacterId(1), 15);
    let player = runtime.players.get_mut(&1).unwrap();
    player.set_tunnel_used(10, 10); // maxed
    player.set_tunnel_used(11, 3); // partial
                                   // 12 stays untouched (never completed).

    let result = apply_tunnellist_command(&world, &runtime, CharacterId(1), "/tunnels")
        .expect("tunnels command should be recognized");
    assert_eq!(result.message_bytes.len(), 2);

    let mut expected_header = Vec::new();
    expected_header.extend_from_slice(COL_ORANGE);
    expected_header.extend_from_slice(b"Tunnels status:");
    expected_header.extend_from_slice(COL_RESET);
    assert_eq!(result.message_bytes[0], expected_header);

    // char_level (15) - 10 = 5, so max(10, 5) = 10; highest_completed = 11
    // (level 11 is the highest with used >= 1); max_level = max(11, 10) = 11.
    let mut expected_body = Vec::new();
    expected_body.extend_from_slice(COL_VIOLET);
    expected_body.extend_from_slice(b" 10");
    expected_body.extend_from_slice(COL_RESET);
    expected_body.extend_from_slice(b" ");
    expected_body.extend_from_slice(COL_LIGHT_RED);
    expected_body.extend_from_slice(b" 11");
    expected_body.extend_from_slice(COL_RESET);
    expected_body.extend_from_slice(b" ");
    assert_eq!(result.message_bytes[1], expected_body);
}

/// A disconnected/never-logged-in character has no `PlayerRuntime`, so the
/// command falls through (`None`).
#[test]
fn tunnellist_command_falls_through_without_a_live_player_runtime() {
    let world = World::default();
    let runtime = ServerRuntime::default();
    assert!(apply_tunnellist_command(&world, &runtime, CharacterId(1), "/tunnels").is_none());
}

/// C `cmdcmp(ptr, "steal", 5)`: `minlen == strlen("steal")`, an exact
/// case-insensitive word match, not a prefix abbreviation like `/thief`.
#[test]
fn steal_command_requires_exact_word_match() {
    let mut world = World::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));

    assert!(apply_steal_command(&mut world, &mut player, CharacterId(1), "/st", 0).is_none());
    assert!(apply_steal_command(&mut world, &mut player, CharacterId(1), "/stealing", 0).is_none());
    assert!(apply_steal_command(&mut world, &mut player, CharacterId(1), "/STEAL", 0).is_some());
}

#[test]
fn steal_command_reports_not_a_thief() {
    let mut world = World::default();
    let mut attacker = login_character(CharacterId(1), &login_block("Thief"), 1, 10, 10);
    attacker.dir = Direction::Right as u8;
    assert!(world.spawn_character(attacker, 10, 10));
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));

    let result = apply_steal_command(&mut world, &mut player, CharacterId(1), "/steal", 0)
        .expect("steal command should be recognized");

    assert_eq!(
        result.messages,
        vec!["You are not a thief, you cannot steal.".to_string()]
    );
}

/// End-to-end success path: the item moves, the victim gets a
/// `COL_LIGHT_RED` notification, the caller's inventory is flagged for
/// refresh, and (since both are `CF_PK`) `add_pk_steal` bumps
/// `pk_last_kill` (C's own quirky reuse of the kill timestamp field for
/// steal events, see `PlayerRuntime::add_pk_steal`'s doc comment).
#[test]
fn steal_command_success_notifies_victim_and_bumps_pk_steal_stat() {
    let mut world = World::default();
    // Brute-forced against the exact `legacy_random_below_from_seed` LCG:
    // with `chance == 50` (`stealth 20 - percept 0 == 20 -> diff 10 ->
    // 40+10`), seed `1` lands in the `diff >= 0` "unnoticed" bucket after
    // the item-pick draw (`RANDOM(1)`) and the dice draw (`RANDOM(100)`).
    world.legacy_random_seed = 1;
    world.tick = ugaris_core::Tick(TICKS_PER_SECOND * 10);

    let mut attacker = login_character(CharacterId(1), &login_block("Thief"), 1, 10, 10);
    attacker.professions[profession::THIEF] = 20;
    attacker.values[0][CharacterValue::Stealth as usize] = 20;
    attacker.dir = Direction::Right as u8;
    attacker.flags.insert(CharacterFlags::PK);
    assert!(world.spawn_character(attacker, 10, 10));

    let mut victim = login_character(CharacterId(2), &login_block("Victim"), 1, 11, 10);
    victim.flags.insert(CharacterFlags::PK);
    victim.inventory[30] = Some(ItemId(900));
    assert!(world.spawn_character(victim, 11, 10));

    let mut stolen_item = test_item(ItemId(900), 0, ItemFlags::USED);
    stolen_item.carried_by = Some(CharacterId(2));
    world.items.insert(ItemId(900), stolen_item);

    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));

    let result = apply_steal_command(&mut world, &mut player, CharacterId(1), "/steal", 500)
        .expect("steal command should be recognized");

    assert_eq!(
        result.messages,
        vec!["You stole a Item without Victim noticing.".to_string()]
    );
    assert!(result.target_message_bytes.is_empty());
    assert!(result.inventory_changed);
    assert_eq!(player.pk_last_kill, 500);

    let attacker = world.characters.get(&CharacterId(1)).unwrap();
    assert!(attacker.inventory.contains(&Some(ItemId(900))));
    let victim = world.characters.get(&CharacterId(2)).unwrap();
    assert!(victim.inventory[30].is_none());
}
