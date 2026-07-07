use super::*;

#[test]
pub(crate) fn god_querystats_command_queues_a_lookup() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    // No immediate reply - the actual data needs a `PgCharacterRepository`
    // read, resolved by `apply_querystats_events` once queued (see
    // `ugaris-core`'s `world/querystats.rs` module doc comment).
    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/querystats", 1)
        .expect("legacy cmdcmp accepts the full querystats word");
    assert!(result.messages.is_empty());
    let queued = world.drain_pending_querystats_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, god_id);

    // minlen 5: "query" is the shortest accepted abbreviation.
    apply_admin_character_command(&mut world, &mut runtime, god_id, "/query", 1)
        .expect("legacy cmdcmp accepts prefix length five");
    assert_eq!(world.drain_pending_querystats_lookups().len(), 1);
    // Shorter than minlen 5 doesn't reach `querystats` at all.
    assert!(apply_admin_character_command(&mut world, &mut runtime, god_id, "/quer", 1).is_none());

    let mortal_id = CharacterId(8);
    world.add_character(login_character(
        mortal_id,
        &login_block("Mortal"),
        1,
        11,
        10,
    ));
    assert!(
        apply_admin_character_command(&mut world, &mut runtime, mortal_id, "/querystats", 1)
            .is_none()
    );
    assert!(world.drain_pending_querystats_lookups().is_empty());
}

#[test]
pub(crate) fn labsolved_command_supports_named_target_and_missing_lookup() {
    let mut world = World::default();
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
    let mut runtime = ServerRuntime::default();
    let mut target_player = PlayerRuntime::connected(80, 0);
    target_player.character_id = Some(target_id);
    runtime.players.insert(80, target_player);

    let missing =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/labsolved Missing 3", 1)
            .expect("god labsolved missing target should be handled");
    assert_eq!(
        missing.messages,
        vec!["Sorry, no one by the name Missing around."]
    );

    let granted =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/labsolved Target 3", 1)
            .expect("god labsolved named target should be recognized");
    assert_eq!(granted.messages, vec!["Target has solved lab 3."]);
    assert_eq!(
        runtime
            .player_for_character(target_id)
            .unwrap()
            .lab_solved_bits,
        1u64 << 3
    );
}

#[test]
pub(crate) fn jail_command_queues_a_lookup_for_a_valid_name() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::STAFF);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/jail Baddie", 3)
            .expect("staff jail command should be recognized");
    assert!(result.messages.is_empty());
    let queued = world.drain_pending_jail_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, caller_id);
    assert_eq!(queued[0].target_name, "Baddie");
    assert_eq!(queued[0].action, ugaris_core::world::JailAction::Jail);
}

#[test]
pub(crate) fn unjail_command_queues_a_lookup_with_the_unjail_action() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/unjail Baddie", 3)
            .expect("god unjail command should be recognized");
    assert!(result.messages.is_empty());
    let queued = world.drain_pending_jail_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].target_name, "Baddie");
    assert_eq!(queued[0].action, ugaris_core::world::JailAction::Unjail);
}

#[test]
pub(crate) fn rmdeath_command_queues_a_lookup_for_a_valid_name() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/rmdeath Baddie", 3)
            .expect("god rmdeath command should be recognized");
    assert!(result.messages.is_empty());
    let queued = world.drain_pending_rmdeath_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, caller_id);
    assert_eq!(queued[0].target_name, "Baddie");
}

#[test]
pub(crate) fn accleanup_at_or_above_the_minimum_queues_the_lookup_and_confirms() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "#accleanup 7", 1)
        .expect("god accleanup should be recognized");
    assert_eq!(
        result.messages,
        vec!["Cleaning up records older than 7 days..."]
    );
    let queued = world.drain_pending_ac_cleanup_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, god_id);
    assert_eq!(queued[0].days, 7);
}

#[test]
pub(crate) fn acsiglist_is_god_only_unlike_its_staff_accessible_siblings_and_queues_the_lookup() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    let mut staff = login_character(CharacterId(20), &login_block("Staffer"), 1, 12, 10);
    staff.flags.insert(CharacterFlags::STAFF);
    world.add_character(staff);

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, target_id, "#acsiglist", 1)
            .is_none()
    );
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        CharacterId(20),
        "#acsiglist",
        1
    )
    .is_none());

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "#acsiglist", 1)
        .expect("god acsiglist should be recognized");
    assert!(result.messages.is_empty());
    let queued = world.drain_pending_ac_siglist_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, god_id);
}

#[test]
pub(crate) fn acsigadd_with_a_valid_call_queues_the_lookup_with_a_multi_word_name() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "#acsigadd hardware_hash deadbeef Known Cheat Tool",
        1,
    )
    .expect("god acsigadd should be recognized");
    assert!(result.messages.is_empty());
    let queued = world.drain_pending_ac_sigadd_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, god_id);
    assert_eq!(queued[0].sig_type, "hardware_hash");
    assert_eq!(queued[0].sig_value, "deadbeef");
    assert_eq!(queued[0].name, "Known Cheat Tool");
    assert_eq!(queued[0].created_by, "Godmode");
}

#[test]
pub(crate) fn acsigdel_with_a_valid_id_queues_the_lookup() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "#acsigdel 42", 1)
        .expect("god acsigdel should be recognized");
    assert!(result.messages.is_empty());
    let queued = world.drain_pending_ac_sigdel_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, god_id);
    assert_eq!(queued[0].signature_id, 42);
}

#[test]
pub(crate) fn acreset_queues_a_lookup_using_the_targets_anticheat_session_id() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    for player in runtime.players.values_mut() {
        if player.character_id == Some(target_id) {
            player.anticheat_session_id = Some(4321);
        }
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acreset target", 1)
            .expect("god acreset should be recognized");
    assert_eq!(result.messages, Vec::<String>::new());

    let queued = world.drain_pending_ac_reset_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, god_id);
    assert_eq!(queued[0].target_name, "Target");
    assert_eq!(queued[0].session_id, 4321);
}

#[test]
pub(crate) fn acflag_queues_a_lookup_using_the_targets_anticheat_session_id() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    for player in runtime.players.values_mut() {
        if player.character_id == Some(target_id) {
            player.anticheat_session_id = Some(4321);
        }
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acflag target", 1)
            .expect("god acflag should be recognized");
    assert_eq!(result.messages, Vec::<String>::new());

    let queued = world.drain_pending_ac_flag_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, god_id);
    assert_eq!(queued[0].target_name, "Target");
    assert_eq!(queued[0].session_id, 4321);
}

#[test]
pub(crate) fn acunflag_queues_a_lookup_using_the_targets_anticheat_session_id() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    for player in runtime.players.values_mut() {
        if player.character_id == Some(target_id) {
            player.anticheat_session_id = Some(4321);
        }
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acunflag target", 1)
            .expect("god acunflag should be recognized");
    assert_eq!(result.messages, Vec::<String>::new());

    let queued = world.drain_pending_ac_unflag_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, god_id);
    assert_eq!(queued[0].target_name, "Target");
    assert_eq!(queued[0].session_id, 4321);
}

#[test]
pub(crate) fn actrust_queues_a_lookup_using_the_targets_anticheat_session_id() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    for player in runtime.players.values_mut() {
        if player.character_id == Some(target_id) {
            player.anticheat_session_id = Some(4321);
        }
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#actrust target", 1)
            .expect("god actrust should be recognized");
    assert_eq!(result.messages, Vec::<String>::new());

    let queued = world.drain_pending_ac_trust_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, god_id);
    assert_eq!(queued[0].target_name, "Target");
    assert_eq!(queued[0].session_id, 4321);
}

#[test]
pub(crate) fn acuntrust_queues_a_lookup_using_the_targets_anticheat_session_id() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    for player in runtime.players.values_mut() {
        if player.character_id == Some(target_id) {
            player.anticheat_session_id = Some(4321);
        }
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acuntrust target", 1)
            .expect("god acuntrust should be recognized");
    assert_eq!(result.messages, Vec::<String>::new());

    let queued = world.drain_pending_ac_untrust_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, god_id);
    assert_eq!(queued[0].target_name, "Target");
    assert_eq!(queued[0].session_id, 4321);
}

#[test]
pub(crate) fn acwarn_queues_a_lookup_with_the_default_reason_when_omitted() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    for player in runtime.players.values_mut() {
        if player.character_id == Some(target_id) {
            player.anticheat_session_id = Some(4321);
        }
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acwarn target", 1)
            .expect("god acwarn should be recognized");
    assert_eq!(result.messages, Vec::<String>::new());

    let queued = world.drain_pending_ac_warn_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, god_id);
    assert_eq!(queued[0].target_id, target_id);
    assert_eq!(queued[0].target_name, "Target");
    assert_eq!(queued[0].session_id, 4321);
    assert_eq!(queued[0].reason, "Anti-cheat warning");
}

#[test]
pub(crate) fn acwarn_queues_a_lookup_with_the_given_reason() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    for player in runtime.players.values_mut() {
        if player.character_id == Some(target_id) {
            player.anticheat_session_id = Some(4321);
        }
    }

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "#acwarn target Speedhacking detected in area 3",
        1,
    )
    .expect("god acwarn should be recognized");
    assert_eq!(result.messages, Vec::<String>::new());

    let queued = world.drain_pending_ac_warn_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].reason, "Speedhacking detected in area 3");
}
