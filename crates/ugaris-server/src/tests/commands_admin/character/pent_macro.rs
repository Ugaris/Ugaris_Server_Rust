use super::*;

// C `command.c:1136-1360`/`10416-10465`: the `/pentinfo`, `/setpentcount`,
// `/setpentstatus`, `/setpentbonus` and `/resetpent` GOD debug commands
// over the `DRD_PENT_NPPD` scratch struct (`PlayerRuntime::pentagram_
// debug`).

#[test]
pub(crate) fn pent_debug_commands_are_god_only() {
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
pub(crate) fn pentinfo_requires_a_player_name_and_reports_unknown_players() {
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
pub(crate) fn pentinfo_shows_empty_data_then_active_pentagrams_after_mutation() {
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
pub(crate) fn setpentcount_requires_both_a_name_and_an_integer_value() {
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
pub(crate) fn resetpent_requires_a_name_and_zeroes_every_field() {
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
pub(crate) fn pent_debug_commands_report_missing_runtime_for_online_character() {
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

// C `command.c:660-1123`: the macro-daemon admin/debug commands over the
// `DRD_MACRO_PPD` persistent struct (`PlayerRuntime::macro_ppd`).

#[test]
pub(crate) fn macro_god_only_commands_require_god_not_just_staff() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    // `target_id` has neither GOD nor STAFF, so every gate below should
    // reject it uniformly.
    for command in [
        "/summonmacro Target",
        "/macroimmune Target 5",
        "/macrosuspicion Target 5",
        "/macrokarma Target 5",
        "/macrofailures Target 5",
        "/macroreset Target",
    ] {
        assert!(
            apply_admin_character_command(&mut world, &mut runtime, target_id, command, 1)
                .is_none(),
            "{command} should be CF_GOD-gated"
        );
    }
}

#[test]
pub(crate) fn macro_staff_or_god_commands_accept_staff_without_god() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let god_id = CharacterId(7);
    let staff_id = CharacterId(8);
    let target_id = CharacterId(9);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut staff = login_character(staff_id, &login_block("Staffer"), 1, 10, 10);
    staff.flags.insert(CharacterFlags::STAFF);
    world.add_character(staff);
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

    for command in ["/macrostats Target", "/macrohistory Target", "/macrolist"] {
        assert!(
            apply_admin_character_command(&mut world, &mut runtime, staff_id, command, 1).is_some(),
            "{command} should accept CF_STAFF alone"
        );
        assert!(
            apply_admin_character_command(&mut world, &mut runtime, god_id, command, 1).is_some(),
            "{command} should also accept CF_GOD alone"
        );
    }
}

#[test]
pub(crate) fn macrostats_requires_a_name_and_reports_unknown_players() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let usage = apply_admin_character_command(&mut world, &mut runtime, god_id, "/macrostats", 1)
        .expect("god macrostats should be recognized");
    assert_eq!(usage.messages, vec!["Usage: /macrostats <player>"]);

    let missing =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/macrostats Nobody", 1)
            .expect("god macrostats missing target should be handled");
    assert_eq!(missing.messages, vec!["Player 'Nobody' not found online."]);
}

#[test]
pub(crate) fn macrostats_shows_fresh_state_then_every_conditional_line() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let fresh =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/macrostats Target", 1)
            .expect("god macrostats should be recognized");
    assert_eq!(
        fresh.messages,
        vec![
            "=== Macro Daemon Stats: Target ===",
            "Karma: 0 | Suspicion: 0",
            "Challenges - Passed: 0 | Failed: 0 | Consecutive Fails: 0",
            "Last Activity:",
            "  Exp Gain: never | Combat: never | Gold Change: never",
        ]
    );

    world.date.realtime = 1_000;
    {
        let player = runtime.player_for_character_mut(target_id).unwrap();
        player.macro_ppd.karma = 70;
        player.macro_ppd.suspicion = 20;
        player.macro_ppd.total_passed = 3;
        player.macro_ppd.total_failed = 1;
        player.macro_ppd.challenge_failures = 1;
        player.macro_ppd.last_exp_gain = 940;
        player.macro_ppd.last_combat = 970;
        player.macro_ppd.last_gold_change = 1_000;
        player.macro_ppd.immune_until = 1_120;
        player.macro_ppd.immune_by = 7;
        player.macro_ppd.force_summon = true;
        player.macro_ppd.summoned_by = 7;
        player.macro_ppd.in_challenge_room = true;
    }

    let full =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/macrostats Target", 1)
            .expect("god macrostats should be recognized");
    assert_eq!(
        full.messages,
        vec![
            "=== Macro Daemon Stats: Target ===",
            "Karma: 70 | Suspicion: 20",
            "Challenges - Passed: 3 | Failed: 1 | Consecutive Fails: 1",
            "Last Activity:",
            "  Exp Gain: 60s ago | Combat: 30s ago | Gold Change: 0s ago",
            "IMMUNE for 2 minutes (granted by ID 7)",
            "FORCE SUMMON PENDING (requested by ID 7)",
            "Currently in challenge room",
        ]
    );
}

#[test]
pub(crate) fn macrohistory_requires_a_name_and_reports_empty_then_populated_history() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let usage = apply_admin_character_command(&mut world, &mut runtime, god_id, "/macrohistory", 1)
        .expect("god macrohistory should be recognized");
    assert_eq!(usage.messages, vec!["Usage: /macrohistory <player>"]);

    let missing =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/macrohistory Nobody", 1)
            .expect("god macrohistory missing target should be handled");
    assert_eq!(missing.messages, vec!["Player 'Nobody' not found online."]);

    let empty =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/macrohistory Target", 1)
            .expect("god macrohistory should be recognized");
    assert_eq!(
        empty.messages,
        vec![
            "=== Challenge History: Target ===",
            "No challenge history recorded.",
        ]
    );

    world.date.realtime = 10_000;
    {
        let player = runtime.player_for_character_mut(target_id).unwrap();
        // `history_index` points at the *next* write slot, so the most
        // recently written entry lives at `history_index - 1`
        // (`history[1]` here) - it must carry the more recent timestamp
        // for the scenario to be internally consistent.
        player.macro_ppd.history[0] = MacroHistoryEntry {
            timestamp: 9_400,
            challenge_type: 2,
            passed: false,
            response_time: 0,
        };
        player.macro_ppd.history[1] = MacroHistoryEntry {
            timestamp: 9_880,
            challenge_type: 0,
            passed: true,
            response_time: 12,
        };
        player.macro_ppd.history_count = 2;
        player.macro_ppd.history_index = 2;
    }

    let filled =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/macrohistory Target", 1)
            .expect("god macrohistory should be recognized");
    assert_eq!(
        filled.messages,
        vec![
            "=== Challenge History: Target ===",
            "1. [Math] PASS - 12s response (2 min ago)",
            "2. [Reverse] FAIL (10 min ago)",
            "Total challenges: 2",
        ]
    );
}

#[test]
pub(crate) fn summonmacro_requires_a_name_and_sets_force_summon() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let usage = apply_admin_character_command(&mut world, &mut runtime, god_id, "/summonmacro", 1)
        .expect("god summonmacro should be recognized");
    assert_eq!(usage.messages, vec!["Usage: /summonmacro <player>"]);

    let missing =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/summonmacro Nobody", 1)
            .expect("god summonmacro missing target should be handled");
    assert_eq!(missing.messages, vec!["Player 'Nobody' not found online."]);

    let ok =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/summonmacro Target", 1)
            .expect("god summonmacro should be recognized");
    assert_eq!(
        ok.messages,
        vec!["Macro daemon will summon Target on next check."]
    );
    let ppd = &runtime.player_for_character(target_id).unwrap().macro_ppd;
    assert!(ppd.force_summon);
    assert_eq!(ppd.summoned_by, 7);
}

#[test]
pub(crate) fn macroimmune_grants_and_removes_immunity() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    world.date.realtime = 1_000;

    let usage = apply_admin_character_command(&mut world, &mut runtime, god_id, "/macroimmune", 1)
        .expect("god macroimmune should be recognized");
    assert_eq!(
        usage.messages,
        vec![
            "Usage: /macroimmune <player> <minutes>",
            "Use 0 minutes to remove immunity.",
        ]
    );

    let bad_args = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/macroimmune Target abc",
        1,
    )
    .expect("god macroimmune should be recognized");
    assert_eq!(
        bad_args.messages,
        vec!["Usage: /macroimmune <player> <minutes>"]
    );

    let missing =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/macroimmune Nobody 5", 1)
            .expect("god macroimmune missing target should be handled");
    assert_eq!(missing.messages, vec!["Player 'Nobody' not found online."]);

    let granted = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/macroimmune Target 10",
        1,
    )
    .expect("god macroimmune should be recognized");
    assert_eq!(
        granted.messages,
        vec!["Granted Target immunity from macro daemon for 10 minutes."]
    );
    {
        let ppd = &runtime.player_for_character(target_id).unwrap().macro_ppd;
        assert_eq!(ppd.immune_until, 1_000 + 10 * 60);
        assert_eq!(ppd.immune_by, 7);
    }

    let removed =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/macroimmune Target 0", 1)
            .expect("god macroimmune should be recognized");
    assert_eq!(
        removed.messages,
        vec!["Removed macro daemon immunity from Target."]
    );
    let ppd = &runtime.player_for_character(target_id).unwrap().macro_ppd;
    assert_eq!(ppd.immune_until, 0);
    assert_eq!(ppd.immune_by, 0);
}

#[test]
pub(crate) fn macrosuspicion_adjusts_and_clamps_between_0_and_100() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let usage =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/macrosuspicion", 1)
            .expect("god macrosuspicion should be recognized");
    assert_eq!(
        usage.messages,
        vec![
            "Usage: /macrosuspicion <player> <amount>",
            "Use negative amount to reduce suspicion.",
        ]
    );

    let missing = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/macrosuspicion Nobody 5",
        1,
    )
    .expect("god macrosuspicion missing target should be handled");
    assert_eq!(missing.messages, vec!["Player 'Nobody' not found online."]);

    let raised = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/macrosuspicion Target 90",
        1,
    )
    .expect("god macrosuspicion should be recognized");
    assert_eq!(raised.messages, vec!["Target suspicion: 0 -> 90"]);

    let clamped_high = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/macrosuspicion Target 50",
        1,
    )
    .expect("god macrosuspicion should be recognized");
    assert_eq!(clamped_high.messages, vec!["Target suspicion: 90 -> 100"]);

    let lowered = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/macrosuspicion Target -150",
        1,
    )
    .expect("god macrosuspicion should be recognized");
    assert_eq!(lowered.messages, vec!["Target suspicion: 100 -> 0"]);
}

#[test]
pub(crate) fn macrokarma_sets_and_clamps_between_0_and_100() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    let _ = target_id;

    let usage = apply_admin_character_command(&mut world, &mut runtime, god_id, "/macrokarma", 1)
        .expect("god macrokarma should be recognized");
    assert_eq!(
        usage.messages,
        vec![
            "Usage: /macrokarma <player> <value>",
            "Sets karma to specified value (0-100).",
        ]
    );

    let over = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/macrokarma Target 250",
        1,
    )
    .expect("god macrokarma should be recognized");
    assert_eq!(over.messages, vec!["Target karma: 0 -> 100"]);

    let under = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/macrokarma Target -10",
        1,
    )
    .expect("god macrokarma should be recognized");
    assert_eq!(under.messages, vec!["Target karma: 100 -> 0"]);
}

#[test]
pub(crate) fn macrofailures_sets_and_floors_at_0() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let usage =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/macrofailures", 1)
            .expect("god macrofailures should be recognized");
    assert_eq!(
        usage.messages,
        vec!["Usage: /macrofailures <player> <count>"]
    );

    let set = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/macrofailures Target 3",
        1,
    )
    .expect("god macrofailures should be recognized");
    assert_eq!(set.messages, vec!["Target consecutive failures: 0 -> 3"]);

    let floored = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/macrofailures Target -5",
        1,
    )
    .expect("god macrofailures should be recognized");
    assert_eq!(
        floored.messages,
        vec!["Target consecutive failures: 3 -> 0"]
    );
}

#[test]
pub(crate) fn macroreset_requires_a_name_and_only_resets_the_documented_fields() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    world.date.realtime = 5_000;

    let usage = apply_admin_character_command(&mut world, &mut runtime, god_id, "/macroreset", 1)
        .expect("god macroreset should be recognized");
    assert_eq!(usage.messages, vec!["Usage: /macroreset <player>"]);

    {
        let player = runtime.player_for_character_mut(target_id).unwrap();
        player.macro_ppd.karma = 10;
        player.macro_ppd.suspicion = 80;
        player.macro_ppd.challenge_failures = 4;
        player.macro_ppd.total_passed = 2;
        player.macro_ppd.total_failed = 6;
        player.macro_ppd.history_count = 3;
        player.macro_ppd.history_index = 3;
        player.macro_ppd.immune_until = 9_000;
        player.macro_ppd.immune_by = 3;
        player.macro_ppd.force_summon = true;
        player.macro_ppd.summoned_by = 3;
        // Untouched by C's own `macro_cmd_reset` - must survive the reset.
        player.macro_ppd.last_exp_gain = 4_000;
        player.macro_ppd.in_challenge_room = true;
    }

    let reset =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/macroreset Target", 1)
            .expect("god macroreset should be recognized");
    assert_eq!(reset.messages, vec!["Reset all macro stats for Target."]);

    let ppd = &runtime.player_for_character(target_id).unwrap().macro_ppd;
    assert_eq!(ppd.karma, 50);
    assert_eq!(ppd.suspicion, 0);
    assert_eq!(ppd.challenge_failures, 0);
    assert_eq!(ppd.total_passed, 0);
    assert_eq!(ppd.total_failed, 0);
    assert_eq!(ppd.history_count, 0);
    assert_eq!(ppd.history_index, 0);
    assert_eq!(ppd.immune_until, 0);
    assert_eq!(ppd.immune_by, 0);
    assert!(!ppd.force_summon);
    assert_eq!(ppd.summoned_by, 0);
    assert_eq!(ppd.nextcheck, 5_000 + 60 * 5);
    // Fields C's own `macro_cmd_reset` never touches:
    assert_eq!(ppd.last_exp_gain, 4_000);
    assert!(ppd.in_challenge_room);
}

#[test]
pub(crate) fn macro_debug_commands_report_missing_runtime_for_online_character_without_connecting()
{
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
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

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/macrostats Target", 1)
            .expect("god macrostats should be recognized even without a runtime");
    assert_eq!(
        result.messages,
        vec!["Error: Could not access macro data for Target."]
    );
}

// C `command.c:9049-9057`/`3163-3192` (`/noarch`) and `command.c:9226-9235`
// (`/noprof`).

#[test]
pub(crate) fn noarch_and_noprof_are_god_only() {
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
pub(crate) fn noarch_reports_no_one_by_that_name_and_sends_no_other_message() {
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
pub(crate) fn noarch_caps_values_up_to_immunity_and_clears_arch_flag_with_no_confirmation() {
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
pub(crate) fn noarch_does_not_lower_values_already_at_or_below_the_cap() {
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
pub(crate) fn noprof_zeroes_the_callers_own_professions_only_with_no_confirmation() {
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
pub(crate) fn noprof_ignores_any_trailing_argument_text() {
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
