use super::*;

// C `#ls <name> <dir>` / `#cat <name> <file>` (`command.c:2794-2845`,
// `9237-9253`): a debug feature forwarding a raw `SV_LS`/`SV_CAT` request
// packet to the TARGET character's own connection.

/// Real session registration (`runtime.connect`, not the lighter
/// `insert_runtime_for`/`setup_god_and_online_target` helpers above) so
/// `send_to_session` actually queues onto `runtime.tick_out`, matching the
/// `achievement.rs` test precedent for asserting exact packet bytes reach
/// a target different from the caller.
pub(crate) fn connected_god_and_target(
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
    runtime.connect(1, commands, 0);
    runtime.players.get_mut(&1).unwrap().character_id = Some(god_id);
    let (commands, _rx2) = mpsc::channel(16);
    runtime.connect(2, commands, 0);
    runtime.players.get_mut(&2).unwrap().character_id = Some(target_id);
    (god_id, target_id)
}

#[test]
pub(crate) fn ls_and_cat_are_god_only() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = connected_god_and_target(&mut world, &mut runtime);

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        target_id,
        "#ls Target foo",
        1
    )
    .is_none());
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        target_id,
        "#cat Target foo",
        1
    )
    .is_none());
}

#[test]
pub(crate) fn ls_reports_no_one_by_that_name_when_target_is_offline() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = connected_god_and_target(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#ls Nobody foo", 1)
            .expect("god ls should be recognized");
    assert_eq!(
        result.messages,
        vec!["Sorry, no one by the name Nobody around."]
    );
    assert!(!runtime.tick_out.contains_key(&2));
}

#[test]
pub(crate) fn ls_sends_raw_sv_ls_packet_to_the_target_session_and_confirms_to_the_caller() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = connected_god_and_target(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#ls Target /home/user", 1)
            .expect("god ls should be recognized");
    assert_eq!(result.messages, vec!["ls /home/user scheduled on Target."]);

    let payloads = runtime
        .tick_out
        .get(&2)
        .expect("target session got a packet");
    assert_eq!(payloads.len(), 1);
    let payload = &payloads[0];
    assert_eq!(payload[0], SV_LS);
    assert_eq!(payload[1], b"/home/user".len() as u8);
    assert_eq!(&payload[2..], b"/home/user");
}

#[test]
pub(crate) fn cat_sends_raw_sv_cat_packet_to_the_target_session_and_confirms_to_the_caller() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = connected_god_and_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "#cat Target /etc/passwd",
        1,
    )
    .expect("god cat should be recognized");
    assert_eq!(
        result.messages,
        vec!["cat /etc/passwd scheduled on Target."]
    );

    let payloads = runtime
        .tick_out
        .get(&2)
        .expect("target session got a packet");
    assert_eq!(payloads.len(), 1);
    let payload = &payloads[0];
    assert_eq!(payload[0], SV_CAT);
    assert_eq!(payload[1], b"/etc/passwd".len() as u8);
    assert_eq!(&payload[2..], b"/etc/passwd");
}

#[test]
pub(crate) fn ls_confirms_to_caller_even_when_target_has_no_live_session() {
    // Matches C's unconditional `log_char(cn, ..., "ls %s scheduled on
    // %s.", ...)` after `plr_ls` - the confirmation is sent regardless of
    // whether the target actually has a connection to receive the packet
    // (`ch[co].player == 0` in C, no session in `runtime.sessions` here).
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
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(1, commands, 0);
    runtime.players.get_mut(&1).unwrap().character_id = Some(god_id);
    // Target has a `Character` (so name lookup succeeds, e.g. a loaded
    // NPC sharing the search loop's no-`CF_PLAYER`-filter behavior) but
    // no connected session at all.

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#ls Target foo", 1)
            .expect("god ls should be recognized");
    assert_eq!(result.messages, vec!["ls foo scheduled on Target."]);
}

#[test]
pub(crate) fn ls_produces_no_packet_when_dir_exceeds_two_hundred_bytes_but_still_confirms() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = connected_god_and_target(&mut world, &mut runtime);
    let long_dir = "a".repeat(201);

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        &format!("#ls Target {long_dir}"),
        1,
    )
    .expect("god ls should be recognized");
    assert_eq!(
        result.messages,
        vec![format!("ls {long_dir} scheduled on Target.")]
    );
    assert!(!runtime.tick_out.contains_key(&2));
}

// C's Anti-Cheat Admin Commands (`command.c:10148-10192`): `#achelp`/
// `#acstatus <name>`/`#acstats`/`#aclist`/`#acsuspicious`,
// `CF_GOD|CF_STAFF`-gated.

#[test]
pub(crate) fn achelp_is_god_or_staff_only() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, target_id, "#achelp", 1).is_none()
    );
}

#[test]
pub(crate) fn achelp_lists_every_c_subcommand_verbatim() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "#achelp", 1)
        .expect("god achelp should be recognized");
    assert_eq!(result.messages[0], "--- Anti-Cheat Commands ---");
    assert!(result
        .messages
        .contains(&"#acstatus <name> - Show player's AC status".to_string()));
    assert!(result
        .messages
        .contains(&"#aclist - List online players with AC status".to_string()));
}

#[test]
pub(crate) fn achelp_is_available_to_staff_without_god() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let staff_id = CharacterId(9);
    let mut staff = login_character(staff_id, &login_block("Staffer"), 1, 12, 10);
    staff.flags.insert(CharacterFlags::STAFF);
    world.add_character(staff);

    let result = apply_admin_character_command(&mut world, &mut runtime, staff_id, "#achelp", 1)
        .expect("staff achelp should be recognized");
    assert_eq!(result.messages[0], "--- Anti-Cheat Commands ---");
}

#[test]
pub(crate) fn acstats_and_aclist_are_god_or_staff_only() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, target_id, "#acstats", 1).is_none()
    );
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, target_id, "#aclist", 1).is_none()
    );
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, target_id, "#acsuspicious", 1)
            .is_none()
    );
}

#[test]
pub(crate) fn acstats_and_aclist_only_gather_online_players_with_a_known_anticheat_session() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    // The caller (`god_id`) itself has no registered `PlayerRuntime` in
    // this lightweight helper, so it must be omitted from the gathered
    // targets exactly like a player with no anticheat session.
    for player in runtime.players.values_mut() {
        if player.character_id == Some(target_id) {
            player.anticheat_session_id = Some(555);
        }
    }

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "#acstats", 1)
        .expect("god acstats should be recognized");
    assert_eq!(result.messages, Vec::<String>::new());
    let queued = world.drain_pending_ac_stats_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, god_id);
    assert_eq!(
        queued[0].targets,
        vec![AcOnlineTarget {
            name: "Target".to_string(),
            session_id: 555,
        }]
    );

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "#aclist", 1)
        .expect("god aclist should be recognized");
    assert_eq!(result.messages, Vec::<String>::new());
    let queued = world.drain_pending_ac_list_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, god_id);
    assert_eq!(
        queued[0].targets,
        vec![AcOnlineTarget {
            name: "Target".to_string(),
            session_id: 555,
        }]
    );

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acsuspicious", 1)
            .expect("god acsuspicious should be recognized");
    assert_eq!(result.messages, Vec::<String>::new());
    let queued = world.drain_pending_ac_suspicious_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, god_id);
    assert_eq!(
        queued[0].targets,
        vec![AcOnlineTarget {
            name: "Target".to_string(),
            session_id: 555,
        }]
    );
}

#[test]
pub(crate) fn accleanup_is_god_only_unlike_its_staff_accessible_siblings() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    let mut staff = login_character(CharacterId(20), &login_block("Staffer"), 1, 12, 10);
    staff.flags.insert(CharacterFlags::STAFF);
    world.add_character(staff);

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, target_id, "#accleanup 30", 1)
            .is_none()
    );
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        CharacterId(20),
        "#accleanup 30",
        1
    )
    .is_none());
}

#[test]
pub(crate) fn accleanup_without_a_days_argument_shows_usage() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "#accleanup", 1)
        .expect("god accleanup should be recognized");
    assert_eq!(
        result.messages,
        vec![
            "Usage: #accleanup <days>",
            "Deletes AC records older than <days> days.",
        ]
    );
    assert!(world.drain_pending_ac_cleanup_lookups().is_empty());
}

#[test]
pub(crate) fn accleanup_below_the_seven_day_minimum_is_rejected() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    for days in ["0", "6", "-5", "notanumber"] {
        let result = apply_admin_character_command(
            &mut world,
            &mut runtime,
            god_id,
            &format!("#accleanup {days}"),
            1,
        )
        .expect("god accleanup should be recognized");
        assert_eq!(result.messages, vec!["Minimum retention is 7 days."]);
    }
    assert!(world.drain_pending_ac_cleanup_lookups().is_empty());
}

#[test]
pub(crate) fn acsigadd_is_god_only_unlike_its_staff_accessible_siblings() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    let mut staff = login_character(CharacterId(20), &login_block("Staffer"), 1, 12, 10);
    staff.flags.insert(CharacterFlags::STAFF);
    world.add_character(staff);

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        target_id,
        "#acsigadd hardware_hash deadbeef Bad Tool",
        1
    )
    .is_none());
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        CharacterId(20),
        "#acsigadd hardware_hash deadbeef Bad Tool",
        1
    )
    .is_none());
}

#[test]
pub(crate) fn acsigadd_without_any_arguments_shows_usage_and_types() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "#acsigadd", 1)
        .expect("god acsigadd should be recognized");
    assert_eq!(
        result.messages,
        vec![
            "Usage: #acsigadd <type> <value> <name>",
            "Types: hardware_hash, code_hash, dll_hash, process_name, hardware_id",
        ]
    );
    assert!(world.drain_pending_ac_sigadd_lookups().is_empty());
}

#[test]
pub(crate) fn acsigadd_with_fewer_than_three_tokens_shows_the_short_usage_message() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    for args in ["hardware_hash", "hardware_hash deadbeef"] {
        let result = apply_admin_character_command(
            &mut world,
            &mut runtime,
            god_id,
            &format!("#acsigadd {args}"),
            1,
        )
        .expect("god acsigadd should be recognized");
        assert_eq!(
            result.messages,
            vec!["Usage: #acsigadd <type> <value> <name>"]
        );
    }
    assert!(world.drain_pending_ac_sigadd_lookups().is_empty());
}

#[test]
pub(crate) fn acsigadd_rejects_a_type_outside_the_fixed_allow_list() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "#acsigadd bogus_type deadbeef Bad Tool",
        1,
    )
    .expect("god acsigadd should be recognized");
    assert_eq!(
        result.messages,
        vec!["Invalid type. Use: hardware_hash, code_hash, dll_hash, process_name, hardware_id"]
    );
    assert!(world.drain_pending_ac_sigadd_lookups().is_empty());
}

#[test]
pub(crate) fn acsigdel_is_god_only_unlike_its_staff_accessible_siblings() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    let mut staff = login_character(CharacterId(20), &login_block("Staffer"), 1, 12, 10);
    staff.flags.insert(CharacterFlags::STAFF);
    world.add_character(staff);

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, target_id, "#acsigdel 4", 1)
            .is_none()
    );
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        CharacterId(20),
        "#acsigdel 4",
        1
    )
    .is_none());
}

#[test]
pub(crate) fn acsigdel_without_an_id_shows_usage() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "#acsigdel", 1)
        .expect("god acsigdel should be recognized");
    assert_eq!(result.messages, vec!["Usage: #acsigdel <id>"]);
    assert!(world.drain_pending_ac_sigdel_lookups().is_empty());
}

#[test]
pub(crate) fn acsigdel_with_a_zero_or_non_numeric_id_is_rejected() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    for id in ["0", "notanumber"] {
        let result = apply_admin_character_command(
            &mut world,
            &mut runtime,
            god_id,
            &format!("#acsigdel {id}"),
            1,
        )
        .expect("god acsigdel should be recognized");
        assert_eq!(result.messages, vec!["Invalid signature ID."]);
    }
    assert!(world.drain_pending_ac_sigdel_lookups().is_empty());
}

#[test]
pub(crate) fn acreset_is_god_only_unlike_its_staff_accessible_siblings() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    let mut staff = login_character(CharacterId(20), &login_block("Staffer"), 1, 12, 10);
    staff.flags.insert(CharacterFlags::STAFF);
    world.add_character(staff);

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        target_id,
        "#acreset Target",
        1
    )
    .is_none());
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        CharacterId(20),
        "#acreset Target",
        1
    )
    .is_none());
}

#[test]
pub(crate) fn acreset_without_a_name_shows_usage() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "#acreset", 1)
        .expect("god acreset should be recognized");
    assert_eq!(result.messages, vec!["Usage: #acreset <player>"]);
}

#[test]
pub(crate) fn acreset_reports_not_found_online_for_an_unknown_name() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acreset Nobody", 1)
            .expect("god acreset should be recognized");
    assert_eq!(result.messages, vec!["Player 'Nobody' not found online."]);
}

#[test]
pub(crate) fn acreset_reports_no_connection_data_when_target_has_no_anticheat_session() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    // `setup_god_and_online_target` registers the target's `PlayerRuntime`
    // with `anticheat_session_id: None` (the default).

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acreset Target", 1)
            .expect("god acreset should be recognized");
    assert_eq!(
        result.messages,
        vec!["Player 'Target' has no connection data."]
    );
    assert!(world.drain_pending_ac_reset_lookups().is_empty());
}

#[test]
pub(crate) fn acflag_is_god_or_staff_unlike_acreset_which_is_god_only() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    let mut staff = login_character(CharacterId(20), &login_block("Staffer"), 1, 12, 10);
    staff.flags.insert(CharacterFlags::STAFF);
    world.add_character(staff);

    // Neither GOD nor STAFF -> not recognized.
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        target_id,
        "#acflag Target",
        1
    )
    .is_none());

    // STAFF alone is enough (unlike `#acreset`, which is GOD-only).
    for player in runtime.players.values_mut() {
        if player.character_id == Some(target_id) {
            player.anticheat_session_id = Some(4321);
        }
    }
    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        CharacterId(20),
        "#acflag Target",
        1,
    )
    .expect("staff acflag should be recognized");
    assert_eq!(result.messages, Vec::<String>::new());
    assert_eq!(world.drain_pending_ac_flag_lookups().len(), 1);
}

#[test]
pub(crate) fn acflag_without_a_name_shows_usage() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "#acflag", 1)
        .expect("god acflag should be recognized");
    assert_eq!(result.messages, vec!["Usage: #acflag <player>"]);
}

#[test]
pub(crate) fn acflag_reports_not_found_online_for_an_unknown_name() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acflag Nobody", 1)
            .expect("god acflag should be recognized");
    assert_eq!(result.messages, vec!["Player 'Nobody' not found online."]);
}

#[test]
pub(crate) fn acflag_reports_no_connection_data_when_target_has_no_anticheat_session() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    // `setup_god_and_online_target` registers the target's `PlayerRuntime`
    // with `anticheat_session_id: None` (the default).

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acflag Target", 1)
            .expect("god acflag should be recognized");
    assert_eq!(
        result.messages,
        vec!["Player 'Target' has no connection data."]
    );
    assert!(world.drain_pending_ac_flag_lookups().is_empty());
}

#[test]
pub(crate) fn acunflag_is_god_only_unlike_acflags_god_or_staff_gate() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    let mut staff = login_character(CharacterId(20), &login_block("Staffer"), 1, 12, 10);
    staff.flags.insert(CharacterFlags::STAFF);
    world.add_character(staff);

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        target_id,
        "#acunflag Target",
        1
    )
    .is_none());
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        CharacterId(20),
        "#acunflag Target",
        1
    )
    .is_none());
}

#[test]
pub(crate) fn acunflag_without_a_name_shows_usage() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "#acunflag", 1)
        .expect("god acunflag should be recognized");
    assert_eq!(result.messages, vec!["Usage: #acunflag <player>"]);
}

#[test]
pub(crate) fn acunflag_reports_not_found_online_for_an_unknown_name() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acunflag Nobody", 1)
            .expect("god acunflag should be recognized");
    assert_eq!(result.messages, vec!["Player 'Nobody' not found online."]);
}

#[test]
pub(crate) fn acunflag_reports_no_connection_data_when_target_has_no_anticheat_session() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acunflag Target", 1)
            .expect("god acunflag should be recognized");
    assert_eq!(
        result.messages,
        vec!["Player 'Target' has no connection data."]
    );
    assert!(world.drain_pending_ac_unflag_lookups().is_empty());
}

#[test]
pub(crate) fn actrust_is_god_only() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    let mut staff = login_character(CharacterId(20), &login_block("Staffer"), 1, 12, 10);
    staff.flags.insert(CharacterFlags::STAFF);
    world.add_character(staff);

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        target_id,
        "#actrust Target",
        1
    )
    .is_none());
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        CharacterId(20),
        "#actrust Target",
        1
    )
    .is_none());
}

#[test]
pub(crate) fn actrust_without_a_name_shows_usage() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "#actrust", 1)
        .expect("god actrust should be recognized");
    assert_eq!(result.messages, vec!["Usage: #actrust <player>"]);
}

#[test]
pub(crate) fn actrust_reports_not_found_online_for_an_unknown_name() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#actrust Nobody", 1)
            .expect("god actrust should be recognized");
    assert_eq!(result.messages, vec!["Player 'Nobody' not found online."]);
}

#[test]
pub(crate) fn actrust_reports_no_connection_data_when_target_has_no_anticheat_session() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#actrust Target", 1)
            .expect("god actrust should be recognized");
    assert_eq!(
        result.messages,
        vec!["Player 'Target' has no connection data."]
    );
    assert!(world.drain_pending_ac_trust_lookups().is_empty());
}

#[test]
pub(crate) fn acuntrust_is_god_only() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    let mut staff = login_character(CharacterId(20), &login_block("Staffer"), 1, 12, 10);
    staff.flags.insert(CharacterFlags::STAFF);
    world.add_character(staff);

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        target_id,
        "#acuntrust Target",
        1
    )
    .is_none());
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        CharacterId(20),
        "#acuntrust Target",
        1
    )
    .is_none());
}

#[test]
pub(crate) fn acuntrust_without_a_name_shows_usage() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "#acuntrust", 1)
        .expect("god acuntrust should be recognized");
    assert_eq!(result.messages, vec!["Usage: #acuntrust <player>"]);
}

#[test]
pub(crate) fn acuntrust_reports_not_found_online_for_an_unknown_name() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acuntrust Nobody", 1)
            .expect("god acuntrust should be recognized");
    assert_eq!(result.messages, vec!["Player 'Nobody' not found online."]);
}

#[test]
pub(crate) fn acuntrust_reports_no_connection_data_when_target_has_no_anticheat_session() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acuntrust Target", 1)
            .expect("god acuntrust should be recognized");
    assert_eq!(
        result.messages,
        vec!["Player 'Target' has no connection data."]
    );
    assert!(world.drain_pending_ac_untrust_lookups().is_empty());
}

#[test]
pub(crate) fn acwatch_is_god_or_staff_only() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        target_id,
        "#acwatch Target",
        1
    )
    .is_none());
}

#[test]
pub(crate) fn acwatch_without_a_name_shows_usage() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "#acwatch", 1)
        .expect("god acwatch should be recognized");
    assert_eq!(result.messages, vec!["Usage: #acwatch <player>"]);
}

#[test]
pub(crate) fn acwatch_reports_not_found_online_for_an_unknown_name() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acwatch Nobody", 1)
            .expect("god acwatch should be recognized");
    assert_eq!(result.messages, vec!["Player 'Nobody' not found online."]);
}

#[test]
pub(crate) fn acwatch_reports_no_connection_data_when_target_has_no_player_runtime() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    // A player-flagged character with no registered `PlayerRuntime` at
    // all (unlike `setup_god_and_online_target`'s own target, which
    // always gets one) - the genuine "no connection data" case for a
    // purely in-memory command like `#acwatch`, distinct from
    // `#acstatus`/`#acreset`'s "known online but no anticheat session"
    // case.
    world.add_character(login_character(
        CharacterId(30),
        &login_block("Ghost"),
        1,
        13,
        10,
    ));

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acwatch Ghost", 1)
            .expect("god acwatch should be recognized");
    assert_eq!(
        result.messages,
        vec!["Player 'Ghost' has no connection data."]
    );
}

#[test]
pub(crate) fn acwatch_toggles_the_targets_watch_flag_and_confirms() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acwatch target", 1)
            .expect("god acwatch should be recognized");
    assert_eq!(
        result.messages,
        vec!["Now watching Target - detailed AC logging enabled."]
    );
    assert!(
        runtime
            .players
            .values()
            .find(|player| player.character_id == Some(target_id))
            .unwrap()
            .ac_watch_enabled
    );

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acwatch Target", 1)
            .expect("god acwatch should be recognized");
    assert_eq!(result.messages, vec!["Stopped watching Target."]);
    assert!(
        !runtime
            .players
            .values()
            .find(|player| player.character_id == Some(target_id))
            .unwrap()
            .ac_watch_enabled
    );
}

// C `#acwarn <player> [reason]` (`command.c:10323-10329` dispatch,
// `ac_cmd_warn`, `anticheat.c:1291-1314`), `CF_GOD|CF_STAFF`-gated
// (unlike `#actrust`/`#acuntrust`/`#acunflag`'s `CF_GOD`-only, but the
// same gate as `#acflag`).

#[test]
pub(crate) fn acwarn_is_god_or_staff() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    let mut staff = login_character(CharacterId(20), &login_block("Staffer"), 1, 12, 10);
    staff.flags.insert(CharacterFlags::STAFF);
    world.add_character(staff);

    // Neither GOD nor STAFF -> not recognized.
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        target_id,
        "#acwarn Target",
        1
    )
    .is_none());

    // STAFF alone is enough.
    for player in runtime.players.values_mut() {
        if player.character_id == Some(target_id) {
            player.anticheat_session_id = Some(4321);
        }
    }
    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        CharacterId(20),
        "#acwarn Target",
        1,
    )
    .expect("staff acwarn should be recognized");
    assert_eq!(result.messages, Vec::<String>::new());
    assert_eq!(world.drain_pending_ac_warn_lookups().len(), 1);
}

#[test]
pub(crate) fn acwarn_without_a_name_shows_usage() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "#acwarn", 1)
        .expect("god acwarn should be recognized");
    assert_eq!(result.messages, vec!["Usage: #acwarn <player> [reason]"]);
}

#[test]
pub(crate) fn acwarn_reports_not_found_online_for_an_unknown_name() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acwarn Nobody", 1)
            .expect("god acwarn should be recognized");
    assert_eq!(result.messages, vec!["Player 'Nobody' not found online."]);
}

#[test]
pub(crate) fn acwarn_reports_no_connection_data_when_target_has_no_anticheat_session() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acwarn Target", 1)
            .expect("god acwarn should be recognized");
    assert_eq!(
        result.messages,
        vec!["Player 'Target' has no connection data."]
    );
    assert!(world.drain_pending_ac_warn_lookups().is_empty());
}
