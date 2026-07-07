use super::*;

// C `/showppd <name> <ppd>` (`command.c:8790-8837` dispatch,
// `cmd_showppd` `command.c:275-346`), `CF_GOD`-gated, online-only (not
// `lookup_name`-backed).

#[test]
pub(crate) fn showppd_is_god_only() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        target_id,
        "/showppd Target area1",
        1
    )
    .is_none());
}

#[test]
pub(crate) fn showppd_reports_offline_or_unknown_player_before_checking_the_ppd_name() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/showppd Nobody area1", 1)
            .expect("god showppd should be recognized");
    assert_eq!(
        result.messages,
        vec!["Sorry, no player by the name Nobody online (offline chars not possible)."]
    );
}

#[test]
pub(crate) fn showppd_reports_which_ppd_when_no_ppd_name_is_given() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/showppd Target", 1)
            .expect("god showppd should be recognized");
    assert_eq!(result.messages, vec!["Which ppd?"]);
}

#[test]
pub(crate) fn showppd_reports_unknown_ppd_names() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/showppd Target area2", 1)
            .expect("god showppd should be recognized");
    assert_eq!(result.messages, vec!["Sorry, no ppd by the name area2."]);
}

#[test]
pub(crate) fn showppd_area1_dumps_every_field_matching_the_c_format_strings() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    {
        let player = runtime
            .players
            .values_mut()
            .find(|player| player.character_id == Some(target_id))
            .unwrap();
        player.set_area1_yoakin_state(1);
        player.set_area1_yoakin_seen_timer(2);
        player.set_area1_greeter_state(3);
        player.set_area1_greeter_seen_timer(4);
        player.set_area1_aclerk_state(5);
        player.set_area1_aclerk_seen_timer(6);
        player.set_area1_camhermit_state(7);
        player.set_area1_camhermit_seen_timer(8);
        player.set_area1_camhermit_kills(9);
        player.set_area1_jessica_state(10);
        player.set_area1_jessica_seen_timer(11);
        player.set_area1_gwendy_state(12);
        player.set_area1_gwendy_seen_timer(13);
        player.set_area1_gerewin_state(14);
        player.set_area1_gerewin_seen_timer(15);
        player.set_area1_lydia_state(16);
        player.set_area1_lydia_seen_timer(17);
        player.set_area1_asturin_state(18);
        player.set_area1_asturin_seen_timer(19);
        player.set_area1_guiwynn_state(20);
        player.set_area1_guiwynn_seen_timer(21);
        player.set_area1_logain_state(22);
        player.set_area1_logain_seen_timer(23);
        player.set_area1_brithildie_state(24);
        player.set_area1_brithildie_seen_timer(25);
        player.set_area1_jiu_state(26);
        player.set_area1_jiu_seen_timer(27);
        player.set_area1_nook_state(28);
        player.set_area1_darkin_state(29);
        player.set_area1_terion_state(30);
        player.set_area1_shrike_state(31);
        player.set_area1_shrike_fails(32);
        player.set_area1_reskin_state(33);
        player.set_area1_reskin_seen_timer(34);
        player.set_area1_reskin_got_bits(35);
        player.set_area1_james_state(36);
        player.set_area1_flags(37);
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/showppd Target area1", 1)
            .expect("god showppd should be recognized");
    assert_eq!(
        result.messages,
        vec![
            "Area1 ppd of Target".to_string(),
            "Yoakin state: 1, Yoakin seen timer: 2, Greeter state: 3, Greeter seen timer: 4"
                .to_string(),
            "AClerk state: 5, AClerk seen timer: 6, Cameron Hermit state: 7, Cameron Hermit seen timer: 8, Cameron Hermit kill count: 9".to_string(),
            "Jessica state: 10, Jessica seen timer: 11, Gwendolyn state: 12, Gwendolyn seen timer: 13".to_string(),
            "Gerewin state: 14, Gerewin seen timer: 15, Lydia state: 16, Lydia seen timer: 17".to_string(),
            "Asturin state: 18, Asturin seen timer: 19, Guiwynn state: 20, Guiwynn seen timer: 21".to_string(),
            "Logain state: 22, Logain seen timer: 23, Brithildie state: 24, Brithildie seen timer: 25".to_string(),
            "Jiu state: 26, Jiu seen timer: 27, Nook state: 28, Darkin state: 29".to_string(),
            "Terion state: 30, Shrike state: 31, Shrike fails: 32".to_string(),
            "Reskin state: 33, Reskin seen timer: 34, Reskin got bits: 35".to_string(),
            "James state: 36, James flags: 37".to_string(),
        ]
    );
}

#[test]
pub(crate) fn showppd_area3_reports_only_kassim_state() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    {
        let player = runtime
            .players
            .values_mut()
            .find(|player| player.character_id == Some(target_id))
            .unwrap();
        player.set_area3_kassim_state(42);
        // Not read by C's `cmd_showppd` "area3" branch at all - confirms
        // the port doesn't accidentally leak other area3 fields either.
        player.set_area3_seymour_state(99);
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/showppd Target area3", 1)
            .expect("god showppd should be recognized");
    assert_eq!(result.messages, vec!["Kassim state: 42"]);
}

// C `/punish <name> <level> <reason>` (`command.c:6500-6507` dispatch ->
// `cmd_punish`, `command.c:2354-2406`), `CF_GOD|CF_STAFF`-gated, full-word
// only.

#[test]
pub(crate) fn punish_command_requires_god_or_staff() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/punish Baddie 3 being quite mean",
        3
    )
    .is_none());
}

#[test]
pub(crate) fn punish_command_accepts_staff_alone() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::STAFF);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/punish Baddie 3 being quite mean",
        3,
    )
    .expect("staff punish command should be recognized");
    assert!(result.messages.is_empty());
    let queued = world.drain_pending_punish_requests();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].target_name, "Baddie");
    assert_eq!(queued[0].level, 3);
    assert_eq!(queued[0].reason, "being quite mean");
}

#[test]
pub(crate) fn punish_command_rejects_invalid_name_immediately() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/punish A 3 being quite mean",
        3,
    )
    .expect("god punish command should be recognized");
    assert_eq!(result.messages, vec!["Sorry, no player by the name A."]);
    assert!(world.drain_pending_punish_requests().is_empty());
}

#[test]
pub(crate) fn punish_command_rejects_short_reason() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/punish Baddie 3 bad",
        3,
    )
    .expect("god punish command should be recognized");
    assert_eq!(result.messages, vec!["Sorry, the reason bad is too short."]);
    assert!(world.drain_pending_punish_requests().is_empty());
}

#[test]
pub(crate) fn punish_command_rejects_out_of_bounds_level() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/punish Baddie 9 being quite mean",
        3,
    )
    .expect("god punish command should be recognized");
    assert_eq!(
        result.messages,
        vec!["Sorry, the level is out of bounds (0-6)."]
    );
    assert!(world.drain_pending_punish_requests().is_empty());
}

#[test]
pub(crate) fn punish_command_abbreviation_is_not_recognized() {
    // C `cmdcmp(ptr, "punish", 6)` requires the full six-letter word.
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/punis Baddie 3 being quite mean",
        3
    )
    .is_none());
}

// C `/unpunish <name> <note id>` (`command.c:6541-6547` dispatch ->
// `cmd_unpunish`, `command.c:2706-2731`), `CF_GOD`-only-gated, full-word
// only.

#[test]
pub(crate) fn unpunish_command_requires_god_not_just_staff() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::STAFF);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/unpunish Baddie 42",
        3
    )
    .is_none());
}

#[test]
pub(crate) fn unpunish_command_queues_a_valid_request_with_no_immediate_reply() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/unpunish Baddie 42",
        3,
    )
    .expect("god unpunish command should be recognized");
    assert!(result.messages.is_empty());
    let queued = world.drain_pending_unpunish_requests();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].target_name, "Baddie");
    assert_eq!(queued[0].note_id, 42);
}

#[test]
pub(crate) fn unpunish_command_abbreviation_is_not_recognized() {
    // C `cmdcmp(ptr, "unpunish", 8)` requires the full eight-letter word.
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/unpunis Baddie 42",
        3
    )
    .is_none());
}

// C `/exterminate <name>` (`command.c:9657-9662` dispatch ->
// `cmd_exterminate`, `command.c:2639-2651`), `CF_STAFF|CF_GOD`-gated,
// full-word only.

#[test]
pub(crate) fn exterminate_command_requires_god_or_staff() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/exterminate Baddie",
        3
    )
    .is_none());
    assert!(world.drain_pending_exterminate_requests().is_empty());
}

#[test]
pub(crate) fn exterminate_command_accepts_staff_alone_and_queues_the_parsed_name() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::STAFF);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/exterminate Baddie",
        3,
    )
    .expect("staff exterminate command should be recognized");
    assert!(result.messages.is_empty());
    let queued = world.drain_pending_exterminate_requests();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, caller_id);
    assert_eq!(queued[0].target_name, "Baddie");
}

#[test]
pub(crate) fn exterminate_command_accepts_god_alone() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/exterminate Baddie",
        3,
    )
    .expect("god exterminate command should be recognized");
    assert!(result.messages.is_empty());
    assert_eq!(world.drain_pending_exterminate_requests().len(), 1);
}

#[test]
pub(crate) fn exterminate_command_abbreviation_is_not_recognized() {
    // C `cmdcmp(ptr, "exterminate", 11)` requires the full eleven-letter
    // word.
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/extermina Baddie",
        3
    )
    .is_none());
}

#[test]
pub(crate) fn exterminate_command_truncates_the_name_at_the_first_non_alpha_character() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/exterminate Bad123die",
        3,
    )
    .expect("god exterminate command should be recognized");
    let queued = world.drain_pending_exterminate_requests();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].target_name, "Bad");
}

// C `/look <name>` (`command.c:8990-9019`), `CF_GOD|CF_STAFF`-gated,
// full-word only.

#[test]
pub(crate) fn look_command_requires_god_or_staff() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/look Baddie", 3)
            .is_none()
    );
}

#[test]
pub(crate) fn look_command_accepts_staff_alone_and_queues_a_request() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::STAFF);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/look Baddie", 3)
            .expect("staff look command should be recognized");
    assert!(result.messages.is_empty());
    let queued = world.drain_pending_look_requests();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].target_name, "Baddie");
}

#[test]
pub(crate) fn look_command_empty_argument_replies_immediately_without_queuing() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    apply_admin_character_command(&mut world, &mut runtime, caller_id, "/look", 3)
        .expect("god look command should be recognized");
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].message, "Expected a character name.");
    assert!(world.drain_pending_look_requests().is_empty());
}

#[test]
pub(crate) fn look_command_abbreviation_is_not_recognized() {
    // C `cmdcmp(ptr, "look", 4)` requires the full four-letter word.
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/loo Baddie", 3)
            .is_none()
    );
}

// C `/klog` (`command.c:9022-9024` -> `karmalog`), `CF_GOD|CF_STAFF`-
// gated, full-word only, no argument.

#[test]
pub(crate) fn klog_command_requires_god_or_staff() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/klog", 3).is_none()
    );
}

#[test]
pub(crate) fn klog_command_accepts_staff_alone_and_queues_a_request() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::STAFF);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(&mut world, &mut runtime, caller_id, "/klog", 3)
        .expect("staff klog command should be recognized");
    assert!(result.messages.is_empty());
    let queued = world.drain_pending_klog_requests();
    assert_eq!(queued, vec![caller_id]);
}

#[test]
pub(crate) fn klog_command_abbreviation_is_not_recognized() {
    // C `cmdcmp(ptr, "klog", 4)` requires the full four-letter word.
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/klo", 3).is_none()
    );
}
