use super::*;

/// Both caller and target get a live session, unlike
/// `setup_god_and_target_with_military_ppd`, since `/setrd`/`/clearrd`/
/// `/solverd` resend the quest log to the ACTING character's own session
/// (`sendquestlog(cn, ch[cn].player)` in C - see the port's doc comment).
pub(crate) fn setup_god_and_target_with_shrine_ppd(
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
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(90, commands, 0);
    runtime.players.get_mut(&90).unwrap().character_id = Some(god_id);
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(91, commands, 0);
    runtime.players.get_mut(&91).unwrap().character_id = Some(target_id);
    (god_id, target_id)
}

#[test]
pub(crate) fn god_setrd_command_sets_continuity_on_self_and_resends_questlog_to_caller() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/setrd 42", 1)
        .expect("god setrd should be recognized");
    assert_eq!(
        result.messages,
        vec!["Set continuity shrine for Godmode to RD 42."]
    );
    assert_eq!(
        runtime
            .player_for_character(god_id)
            .unwrap()
            .random_shrine_continuity,
        42
    );

    // C `sendquestlog(cn, ch[cn].player)` always targets the acting
    // character's own session.
    let payloads = runtime
        .tick_out
        .get(&90)
        .expect("caller session got the questlog resend");
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0][0], ugaris_protocol::packet::SV_QUESTLOG);
    assert!(!runtime.tick_out.contains_key(&91));
}

#[test]
pub(crate) fn god_setrd_command_sets_continuity_on_named_target() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setrd Target 17", 1)
            .expect("god setrd should be recognized");
    assert_eq!(
        result.messages,
        vec!["Set continuity shrine for Target to RD 17."]
    );
    assert_eq!(
        runtime
            .player_for_character(target_id)
            .unwrap()
            .random_shrine_continuity,
        17
    );
    // The caller's own continuity is untouched.
    assert_eq!(
        runtime
            .player_for_character(god_id)
            .unwrap()
            .random_shrine_continuity,
        0
    );
}

#[test]
pub(crate) fn setrd_rejects_rd_number_out_of_10_to_99_range() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    for command in ["/setrd 9", "/setrd 100", "/setrd Target 5"] {
        let result = apply_admin_character_command(&mut world, &mut runtime, god_id, command, 1)
            .expect("setrd should be recognized even with an invalid rd number");
        assert_eq!(
            result.messages,
            vec!["RD number must be between 10 and 99."]
        );
    }
}

#[test]
pub(crate) fn setrd_reports_unknown_name() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setrd Nobody 42", 1)
            .expect("setrd should be recognized");
    assert_eq!(
        result.messages,
        vec!["Sorry, no one by the name Nobody around."]
    );
}

#[test]
pub(crate) fn setrd_reports_failed_player_data_when_target_has_no_live_session() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let offline_id = CharacterId(9);
    world.add_character(login_character(
        offline_id,
        &login_block("Offline"),
        1,
        12,
        10,
    ));
    let mut runtime = ServerRuntime::default();
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(90, commands, 0);
    runtime.players.get_mut(&90).unwrap().character_id = Some(god_id);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setrd Offline 42", 1)
            .expect("setrd should be recognized");
    assert_eq!(result.messages, vec!["Failed to get player data."]);
}

#[test]
pub(crate) fn setrd_is_god_only() {
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
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/setrd 42", 1)
            .is_none()
    );
}

#[test]
pub(crate) fn god_clearrd_command_clears_all_ten_shrines_for_the_rd_level() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    // RD 12 covers shrine indices (12-10)*10..(12-10)*10+10 = 20..30.
    {
        let target_player = runtime.player_for_character_mut(target_id).unwrap();
        for shrine in 20..30u8 {
            target_player.mark_random_shrine_used(shrine);
        }
        // A neighboring RD level's shrine must survive untouched.
        target_player.mark_random_shrine_used(30);
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/clearrd Target 12", 1)
            .expect("god clearrd should be recognized");
    assert_eq!(
        result.messages,
        vec!["Cleared all used shrines for Target in RD 12."]
    );

    let target_player = runtime.player_for_character(target_id).unwrap();
    for shrine in 20..30u8 {
        assert!(!target_player.has_used_random_shrine(shrine));
    }
    assert!(target_player.has_used_random_shrine(30));
}

#[test]
pub(crate) fn god_solverd_command_marks_all_but_the_continuity_shrine_used() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/solverd Target 12", 1)
            .expect("god solverd should be recognized");
    assert_eq!(
        result.messages,
        vec!["Marked all non-continuity shrines as used for Target in RD 12."]
    );

    let target_player = runtime.player_for_character(target_id).unwrap();
    // Shrine indices 20..29 (i = 0..9) get marked; index 29 (i == 9, the
    // continuity shrine) is deliberately skipped.
    for shrine in 20..29u8 {
        assert!(target_player.has_used_random_shrine(shrine));
    }
    assert!(!target_player.has_used_random_shrine(29));
}

#[test]
pub(crate) fn god_changetunnel_command_sets_named_target_clevel_and_notifies_them() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/changetunnel Target 42",
        1,
    )
    .expect("god changetunnel should be recognized");
    assert_eq!(result.messages, vec!["Set Target's tunnel level to 42."]);
    assert_eq!(
        result.other_messages,
        vec![(
            target_id,
            "Your tunnel level has been set to 42 by a god.".to_string()
        )]
    );
    assert_eq!(
        runtime
            .player_for_character(target_id)
            .unwrap()
            .tunnel_clevel(),
        42
    );
    // The caller's own tunnel state is untouched.
    assert_eq!(
        runtime
            .player_for_character(god_id)
            .unwrap()
            .tunnel_clevel(),
        0
    );
}

#[test]
pub(crate) fn changetunnel_rejects_out_of_range_level_and_unknown_name() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    let missing = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/changetunnel Ghost 42",
        1,
    )
    .expect("changetunnel should be recognized even for a missing target");
    assert_eq!(
        missing.messages,
        vec!["Sorry, no one by the name Ghost around."]
    );

    for command in ["/changetunnel Target 9", "/changetunnel Target 201"] {
        let result = apply_admin_character_command(&mut world, &mut runtime, god_id, command, 1)
            .expect("changetunnel should be recognized even with an invalid level");
        assert_eq!(
            result.messages,
            vec!["Invalid tunnel level. Must be between 10 and 200."]
        );
    }
}

#[test]
pub(crate) fn changetunnel_is_god_only() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        target_id,
        "/changetunnel Target 42",
        1
    )
    .is_none());
}

#[test]
pub(crate) fn god_settunnel_command_sets_completed_amount_for_level() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/settunnel Target 15 7",
        1,
    )
    .expect("god settunnel should be recognized");
    assert_eq!(
        result.messages,
        vec!["Set Target's completed amount for tunnel level 15 to 7."]
    );
    assert_eq!(
        result.other_messages,
        vec![(
            target_id,
            "Your completed amount for tunnel level 15 has been set to 7 by a god.".to_string()
        )]
    );
    assert_eq!(
        runtime
            .player_for_character(target_id)
            .unwrap()
            .tunnel_used(15),
        7
    );
}

#[test]
pub(crate) fn god_cleartunnel_command_clears_completed_amount_for_level() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);
    runtime
        .player_for_character_mut(target_id)
        .unwrap()
        .set_tunnel_used(15, 7);

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/cleartunnel Target 15",
        1,
    )
    .expect("god cleartunnel should be recognized");
    assert_eq!(
        result.messages,
        vec!["Cleared Target's completed amount for tunnel level 15."]
    );
    assert_eq!(
        result.other_messages,
        vec![(
            target_id,
            "Your completed amount for tunnel level 15 has been cleared by a god.".to_string()
        )]
    );
    assert_eq!(
        runtime
            .player_for_character(target_id)
            .unwrap()
            .tunnel_used(15),
        0
    );
}

#[test]
pub(crate) fn changetunnel_command_on_self_sends_no_other_message() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/changetunnel Godmode 30",
        1,
    )
    .expect("god changetunnel on self should be recognized");
    assert_eq!(result.messages, vec!["Set Godmode's tunnel level to 30."]);
    assert!(result.other_messages.is_empty());
}

#[test]
pub(crate) fn god_solvetunnel_command_reports_reward_kind_without_mutating_state() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    let exp = apply_admin_character_command(&mut world, &mut runtime, god_id, "/solvetunnel 0", 1)
        .expect("god solvetunnel should be recognized");
    assert_eq!(
        exp.messages,
        vec!["Solved current tunnel and granted experience reward."]
    );

    let mil = apply_admin_character_command(&mut world, &mut runtime, god_id, "/solvetunnel 1", 1)
        .expect("god solvetunnel should be recognized");
    assert_eq!(
        mil.messages,
        vec!["Solved current tunnel and granted military experience reward."]
    );

    let invalid =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/solvetunnel 2", 1)
            .expect("god solvetunnel should be recognized even with an invalid type");
    assert_eq!(
        invalid.messages,
        vec!["Invalid exp type. Must be 0 (exp) or 1 (military exp)."]
    );
}

#[test]
pub(crate) fn solvetunnel_is_god_only() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_target_with_shrine_ppd(&mut world, &mut runtime);

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        target_id,
        "/solvetunnel 0",
        1
    )
    .is_none());
}
