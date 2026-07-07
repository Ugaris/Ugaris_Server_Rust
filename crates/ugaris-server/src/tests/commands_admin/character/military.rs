use super::*;

pub(crate) fn setup_god_and_target_with_military_ppd(
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
pub(crate) fn god_milinfo_command_reports_self_defaults_and_named_target_state() {
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
pub(crate) fn god_milpref_command_sets_preferences_and_replicates_missing_diff_quirk() {
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
pub(crate) fn milpref_is_god_only_and_reports_missing_target() {
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
pub(crate) fn god_milreset_command_clears_all_cooldowns_including_advisors() {
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
pub(crate) fn god_milpoints_command_grants_points_and_promotes_with_broadcast() {
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
pub(crate) fn milpoints_is_god_only_and_requires_name_and_nonzero_points() {
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
pub(crate) fn god_milrec_command_grants_recommendation_points() {
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
pub(crate) fn milrec_is_god_only_requires_name_and_nonzero_points() {
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
pub(crate) fn god_milstats_command_reports_missing_military_master_npc() {
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
pub(crate) fn milstats_is_god_only() {
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
pub(crate) fn god_milsolve_command_completes_mission_promotes_and_announces() {
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
pub(crate) fn milsolve_is_god_only_and_reports_missing_target() {
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
