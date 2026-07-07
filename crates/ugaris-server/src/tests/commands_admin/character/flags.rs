use super::*;

// C `cmd_flag` (`command.c:2870-2937`), shared by `/god`, `/setsir`,
// `/staff`, `/emaster`, `/devel`, `/hardcore`, and `/qmaster`
// (`command.c:9257-9337`).

#[test]
pub(crate) fn god_command_requires_god_permission() {
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
pub(crate) fn god_command_toggles_a_named_online_character_and_names_the_flag() {
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
pub(crate) fn god_command_with_invalid_shape_name_reports_no_player_immediately() {
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
pub(crate) fn god_command_with_validly_shaped_unmatched_name_is_queued_with_no_immediate_message() {
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
pub(crate) fn setsir_command_toggles_won_and_reports_sir_lady_flag_name() {
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
pub(crate) fn staff_emaster_devel_hardcore_qmaster_toggle_their_own_flags() {
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
pub(crate) fn god_command_abbreviation_is_not_recognized() {
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
pub(crate) fn god_global_command_dumps_every_setting_like_c() {
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
pub(crate) fn showflags_requires_god_and_full_word() {
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
pub(crate) fn showflags_reports_no_one_by_that_name_for_an_unloaded_character() {
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
pub(crate) fn showflags_lists_every_set_flag_in_legacy_declaration_order() {
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
pub(crate) fn toggleflag_requires_god_and_full_word() {
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
pub(crate) fn toggleflag_reports_no_one_by_that_name_for_an_unloaded_character() {
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
pub(crate) fn toggleflag_reports_unknown_flag_and_leaves_flags_untouched() {
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
pub(crate) fn toggleflag_toggles_named_flag_on_then_off_case_insensitively() {
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
