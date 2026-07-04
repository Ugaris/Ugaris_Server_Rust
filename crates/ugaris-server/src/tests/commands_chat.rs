use super::*;

#[test]
fn shutup_command_is_staff_only_and_toggles_target_flag() {
    let mut world = World::default();
    let staff_id = CharacterId(7);
    let mut staff = login_character(staff_id, &login_block("Staffer"), 1, 10, 10);
    staff.flags.insert(CharacterFlags::STAFF);
    world.add_character(staff);

    let player_id = CharacterId(8);
    let player = login_character(player_id, &login_block("Target"), 1, 11, 10);
    world.add_character(player);

    let ordinary_id = CharacterId(9);
    let ordinary = login_character(ordinary_id, &login_block("Ordinary"), 1, 12, 10);
    world.add_character(ordinary);
    let mut runtime = ServerRuntime::default();
    runtime.players.insert(1, PlayerRuntime::connected(1, 0));
    runtime.players.insert(2, PlayerRuntime::connected(2, 0));
    runtime.players.get_mut(&1).unwrap().character_id = Some(player_id);
    runtime.players.get_mut(&2).unwrap().character_id = Some(staff_id);

    assert!(apply_shutup_command(
        &mut world,
        &mut runtime,
        ordinary_id,
        "/shutup Target 10",
        100
    )
    .is_none());
    assert!(!world
        .characters
        .get(&player_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::SHUTUP));

    let result = apply_shutup_command(&mut world, &mut runtime, staff_id, "/shutup Target", 100)
        .expect("staff shutup command should be recognized");
    assert!(result.messages.is_empty());
    assert_eq!(
        runtime
            .player_for_character(player_id)
            .unwrap()
            .shutup_until_seconds,
        700
    );
    assert_eq!(result.target_message_bytes.len(), 1);
    assert!(world
        .characters
        .get(&player_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::SHUTUP));

    apply_shutup_command(&mut world, &mut runtime, staff_id, "/shutup Target 0", 101)
        .expect("zero minutes should disable shutup");
    assert_eq!(
        runtime
            .player_for_character(player_id)
            .unwrap()
            .shutup_until_seconds,
        0
    );
    assert!(!world
        .characters
        .get(&player_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::SHUTUP));
}

#[test]
fn shutup_command_preserves_legacy_name_and_range_errors() {
    let mut world = World::default();
    let staff_id = CharacterId(7);
    let mut staff = login_character(staff_id, &login_block("Staffer"), 1, 10, 10);
    staff.flags.insert(CharacterFlags::GOD);
    world.add_character(staff);
    let mut runtime = ServerRuntime::default();

    let result = apply_shutup_command(
        &mut world,
        &mut runtime,
        staff_id,
        "/shutup Missing 10",
        100,
    )
    .expect("god shutup command should be recognized");
    assert_eq!(
        result.messages,
        vec!["Sorry, no player by the name Missing."]
    );

    let target_id = CharacterId(8);
    let target = login_character(target_id, &login_block("Alpha"), 1, 11, 10);
    world.add_character(target);

    let result = apply_shutup_command(
        &mut world,
        &mut runtime,
        staff_id,
        "/shutup Alpha 61abc",
        100,
    )
    .expect("out-of-range shutup should still be handled");
    assert_eq!(
        result.messages,
        vec!["Sorry, can only shutup for 0 to 60 minutes (use 0 to disable)."]
    );
    assert!(!world
        .characters
        .get(&target_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::SHUTUP));

    assert!(
        apply_shutup_command(&mut world, &mut runtime, staff_id, "/shut Alpha 10", 100).is_none()
    );
}

#[test]
fn shutup_expiry_clears_flag_and_notifies_target() {
    let mut world = World::default();
    let target_id = CharacterId(8);
    let mut target = login_character(target_id, &login_block("Target"), 1, 11, 10);
    target.flags.insert(CharacterFlags::SHUTUP);
    world.add_character(target);

    let mut runtime = ServerRuntime::default();
    runtime.players.insert(1, PlayerRuntime::connected(1, 0));
    runtime.players.get_mut(&1).unwrap().character_id = Some(target_id);
    runtime.players.get_mut(&1).unwrap().shutup_until_seconds = 700;

    assert!(drain_expired_shutup_feedback(&mut world, &mut runtime, 699).is_empty());
    assert!(world
        .characters
        .get(&target_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::SHUTUP));

    let feedback = drain_expired_shutup_feedback(&mut world, &mut runtime, 700);
    assert_eq!(feedback.len(), 1);
    assert_eq!(feedback[0].0, target_id);
    assert!(feedback[0].1.starts_with(COL_LIGHT_RED));
    assert!(feedback[0].1.ends_with(COL_RESET));
    assert_eq!(
        runtime
            .player_for_character(target_id)
            .unwrap()
            .shutup_until_seconds,
        0
    );
    assert!(!world
        .characters
        .get(&target_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::SHUTUP));
}

#[test]
fn tell_command_delivers_local_private_message_and_acknowledges_receipt() {
    let mut world = World::default();
    let sender_id = CharacterId(7);
    let target_id = CharacterId(8);
    world.add_character(login_character(sender_id, &login_block("Alice"), 1, 10, 10));
    world.add_character(login_character(target_id, &login_block("Bob"), 1, 11, 10));

    let mut runtime = ServerRuntime::default();
    runtime.players.insert(1, PlayerRuntime::connected(1, 0));
    runtime.players.insert(2, PlayerRuntime::connected(2, 0));
    runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);
    runtime.players.get_mut(&1).unwrap().current_mirror_id = 3;
    runtime.players.get_mut(&2).unwrap().character_id = Some(target_id);

    let result = apply_tell_command(
        &world,
        &mut runtime,
        sender_id,
        "/tell Bob Hello MixedCase",
        100,
    )
    .expect("tell command should be recognized");

    assert_eq!(
        result.sender_messages,
        vec!["Told Bob: \"Hello MixedCase\""]
    );
    assert_eq!(
        result.delivered_messages,
        vec![(
            target_id,
            "Alice (3) tells you: \"Hello MixedCase\"".to_string()
        )]
    );
    assert!(drain_expired_tell_feedback(&world, &mut runtime, 112).is_empty());
}

#[test]
fn tell_command_includes_persisted_staff_code_for_staff_senders() {
    let mut world = World::default();
    let sender_id = CharacterId(7);
    let target_id = CharacterId(8);
    let mut sender = login_character(sender_id, &login_block("Helper"), 1, 10, 10);
    sender.flags.insert(CharacterFlags::STAFF);
    sender.staff_code = "HM".to_string();
    world.add_character(sender);
    world.add_character(login_character(target_id, &login_block("Bob"), 1, 11, 10));

    let mut runtime = ServerRuntime::default();
    runtime.players.insert(1, PlayerRuntime::connected(1, 0));
    runtime.players.insert(2, PlayerRuntime::connected(2, 0));
    runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);
    runtime.players.get_mut(&1).unwrap().current_mirror_id = 4;
    runtime.players.get_mut(&2).unwrap().character_id = Some(target_id);

    let result = apply_tell_command(&world, &mut runtime, sender_id, "/tell Bob Secret", 100)
        .expect("tell command should be recognized");

    assert_eq!(
        result.delivered_messages,
        vec![(
            target_id,
            "HELPER [HM] (4) tells you: \"Secret\"".to_string()
        )]
    );
}

#[test]
fn tell_command_preserves_legacy_errors_and_self_feedback() {
    let mut world = World::default();
    let sender_id = CharacterId(7);
    world.add_character(login_character(sender_id, &login_block("Alice"), 1, 10, 10));
    let mut runtime = ServerRuntime::default();
    runtime.players.insert(1, PlayerRuntime::connected(1, 0));
    runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);

    let missing = apply_tell_command(&world, &mut runtime, sender_id, "/tell Bob Hi", 100)
        .expect("tell command should be recognized");
    assert_eq!(
        missing.sender_messages,
        vec!["Sorry, no player by the name Bob."]
    );

    let empty = apply_tell_command(&world, &mut runtime, sender_id, "/tell Alice", 100)
        .expect("tell command should be recognized");
    assert_eq!(
        empty.sender_messages,
        vec!["Tell, yes, tell it will be, but tell what?"]
    );

    let self_tell = apply_tell_command(&world, &mut runtime, sender_id, "/tell Alice Hi", 100)
        .expect("tell command should be recognized");
    assert_eq!(
        self_tell.sender_messages,
        vec!["Told Alice: \"Hi\"", "Do you like talking to yourself?"]
    );
}

#[test]
fn swearing_filter_blocks_bad_words_and_followup_chat_like_c() {
    let mut world = World::default();
    let sender_id = CharacterId(7);
    let target_id = CharacterId(8);
    world.add_character(login_character(sender_id, &login_block("Alice"), 1, 10, 10));
    world.add_character(login_character(target_id, &login_block("Bob"), 1, 11, 10));

    let mut runtime = ServerRuntime::default();
    runtime.players.insert(1, PlayerRuntime::connected(1, 0));
    runtime.players.insert(2, PlayerRuntime::connected(2, 0));
    runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);
    runtime.players.get_mut(&2).unwrap().character_id = Some(target_id);

    let blocked = crate::apply_tell_command(
        &world,
        &mut runtime,
        sender_id,
        "/tell Bob fuck",
        100,
        1_000,
    )
    .expect("tell command should be recognized");
    assert!(blocked.sender_messages.is_empty());
    assert!(blocked.delivered_messages.is_empty());
    assert_eq!(blocked.delivered_message_bytes.len(), 2);
    assert_eq!(blocked.delivered_message_bytes[0].0, sender_id);
    assert!(
        String::from_utf8_lossy(&blocked.delivered_message_bytes[0].1)
            .contains("Swearing is illegal in this game.")
    );

    let followup = crate::apply_tell_command(
        &world,
        &mut runtime,
        sender_id,
        "/tell Bob hello",
        101,
        1_001,
    )
    .expect("tell command should be recognized");
    assert!(followup.sender_messages.is_empty());
    assert!(followup.delivered_messages.is_empty());
    assert_eq!(followup.delivered_message_bytes.len(), 1);
    assert!(
        String::from_utf8_lossy(&followup.delivered_message_bytes[0].1)
            .contains("Chat is blocked.")
    );
}

#[test]
fn notells_blocks_non_staff_tells_until_timeout_feedback() {
    let mut world = World::default();
    let sender_id = CharacterId(7);
    let target_id = CharacterId(8);
    world.add_character(login_character(sender_id, &login_block("Alice"), 1, 10, 10));
    world.add_character(login_character(target_id, &login_block("Bob"), 1, 11, 10));
    let mut runtime = ServerRuntime::default();
    runtime.players.insert(1, PlayerRuntime::connected(1, 0));
    runtime.players.insert(2, PlayerRuntime::connected(2, 0));
    runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);
    runtime.players.get_mut(&2).unwrap().character_id = Some(target_id);

    let toggle = apply_notells_command(&mut world, target_id, "/not").unwrap();
    assert_eq!(toggle.messages, vec!["Turned no-tell mode on."]);

    let result = apply_tell_command(&world, &mut runtime, sender_id, "/tell Bob Hi", 100)
        .expect("tell command should be recognized");
    assert!(result.delivered_messages.is_empty());
    assert!(drain_expired_tell_feedback(&world, &mut runtime, 100 + TICKS_PER_SECOND).is_empty());
    assert_eq!(
        drain_expired_tell_feedback(&world, &mut runtime, 100 + TICKS_PER_SECOND + 1),
        vec![(sender_id, b"Bob is not listening.".to_vec())]
    );
}

#[test]
fn channels_command_lists_legacy_channel_table() {
    let result = apply_channels_command("/chan").expect("channels prefix should be recognized");

    assert_eq!(
        result.messages[0],
        " 0: Announce   - Announcements from management - NOLEAVE"
    );
    assert!(result
        .messages
        .contains(&" 7: Clan2      - Channel only visible to members of your clan".to_string()));
    assert_eq!(
        result.messages.last().unwrap(),
        "32: God        - Ye God's private channel"
    );
    assert!(apply_channels_command("/clearhate").is_none());
}

#[test]
fn join_leave_chat_commands_preserve_legacy_feedback_and_bits() {
    let mut player = PlayerRuntime::connected(1, 0);

    let joined = apply_join_leave_chat_command(&mut player, CharacterFlags::PLAYER, "/join 2extra")
        .expect("join should be recognized");
    assert_eq!(joined.messages, vec!["You have joined channel 2 (Gossip)."]);
    assert_eq!(player.chat_channels, 1_u32 << 1);

    let duplicate =
        apply_join_leave_chat_command(&mut player, CharacterFlags::PLAYER, "/join 2").unwrap();
    assert_eq!(
        duplicate.messages,
        vec!["You have already joined channel 2 (Gossip)."]
    );

    let left =
        apply_join_leave_chat_command(&mut player, CharacterFlags::PLAYER, "/leave 2").unwrap();
    assert_eq!(left.messages, vec!["You have left channel 2 (Gossip)."]);
    assert_eq!(player.chat_channels, 0);

    let left_again =
        apply_join_leave_chat_command(&mut player, CharacterFlags::PLAYER, "/leave 2").unwrap();
    assert_eq!(
        left_again.messages,
        vec!["You have already left channel 2 (Gossip)."]
    );
}

#[test]
fn chat_command_delivers_joined_local_channel_with_legacy_formatting() {
    let mut world = World::default();
    let sender_id = CharacterId(7);
    let target_id = CharacterId(8);
    world.add_character(login_character(sender_id, &login_block("Alice"), 1, 10, 10));
    world.add_character(login_character(target_id, &login_block("Bob"), 1, 11, 10));

    let mut runtime = ServerRuntime::default();
    runtime.players.insert(1, PlayerRuntime::connected(1, 0));
    runtime.players.insert(2, PlayerRuntime::connected(2, 0));
    runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);
    runtime.players.get_mut(&1).unwrap().current_mirror_id = 3;
    runtime.players.get_mut(&1).unwrap().chat_channels = 1_u32 << 1;
    runtime.players.get_mut(&2).unwrap().character_id = Some(target_id);
    runtime.players.get_mut(&2).unwrap().chat_channels = 1_u32 << 1;

    let result = apply_chat_command(&world, &mut runtime, sender_id, "/gossip Hello", 1)
        .expect("chat channel command should be recognized");

    assert!(result.sender_messages.is_empty());
    assert_eq!(result.delivered_message_bytes.len(), 2);
    let text = String::from_utf8_lossy(&result.delivered_message_bytes[0].1);
    assert!(text.contains("Gossip: "));
    assert!(text.contains("Alice"));
    assert!(text.contains("(3) says: \"Hello\""));
    assert!(result.delivered_message_bytes[0]
        .1
        .starts_with(&[0xb0, b'c']));
}

#[test]
fn chat_command_includes_persisted_staff_code_for_staff_senders() {
    let mut world = World::default();
    let sender_id = CharacterId(7);
    let target_id = CharacterId(8);
    let mut sender = login_character(sender_id, &login_block("Helper"), 1, 10, 10);
    sender.flags.insert(CharacterFlags::STAFF);
    sender.staff_code = "HM".to_string();
    world.add_character(sender);
    world.add_character(login_character(target_id, &login_block("Bob"), 1, 11, 10));

    let mut runtime = ServerRuntime::default();
    runtime.players.insert(1, PlayerRuntime::connected(1, 0));
    runtime.players.insert(2, PlayerRuntime::connected(2, 0));
    runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);
    runtime.players.get_mut(&1).unwrap().current_mirror_id = 4;
    runtime.players.get_mut(&1).unwrap().chat_channels = 1_u32 << 1;
    runtime.players.get_mut(&2).unwrap().character_id = Some(target_id);
    runtime.players.get_mut(&2).unwrap().chat_channels = 1_u32 << 1;

    let result = apply_chat_command(&world, &mut runtime, sender_id, "/gossip Hello", 1)
        .expect("chat channel command should be recognized");

    let text = String::from_utf8_lossy(&result.delivered_message_bytes[0].1);
    assert!(text.contains("HELPER"));
    assert!(text.contains("[HM]"));
    assert!(text.contains("(4) says: \"Hello\""));
}

#[test]
fn local_speech_command_delivers_legacy_say_to_nearby_players() {
    let mut world = World::default();
    let sender_id = CharacterId(7);
    let nearby_id = CharacterId(8);
    let far_id = CharacterId(9);
    let mut sender = login_character(sender_id, &login_block("Alice"), 1, 10, 10);
    sender.x = 10;
    sender.y = 10;
    world.add_character(sender);
    let mut nearby = login_character(nearby_id, &login_block("Bob"), 1, 11, 10);
    nearby.x = 11;
    nearby.y = 10;
    world.add_character(nearby);
    let mut far = login_character(far_id, &login_block("Far"), 1, 250, 250);
    far.x = 250;
    far.y = 250;
    world.add_character(far);

    let mut runtime = ServerRuntime::default();
    for (session, id) in [(1, sender_id), (2, nearby_id), (3, far_id)] {
        runtime
            .players
            .insert(session, PlayerRuntime::connected(session, 0));
        runtime.players.get_mut(&session).unwrap().character_id = Some(id);
    }

    let result = apply_local_speech_command(
        &mut world,
        &runtime,
        sender_id,
        "/say Hello \"quoted\"",
        123,
    )
    .expect("say command should be recognized");

    assert!(result.sender_messages.is_empty());
    let mut deliveries = result
        .delivered_message_bytes
        .iter()
        .map(|(id, bytes)| (*id, String::from_utf8_lossy(bytes).into_owned()))
        .collect::<Vec<_>>();
    deliveries.sort_by_key(|(id, _)| id.0);
    assert_eq!(
        deliveries,
        vec![
            (sender_id, "Alice says: \"Hello \"quoted\"\"".to_string()),
            (nearby_id, "Alice says: \"Hello \"quoted\"\"".to_string()),
        ]
    );
}

#[test]
fn plain_local_speech_delivers_legacy_say() {
    let mut world = World::default();
    let sender_id = CharacterId(7);
    let mut sender = login_character(sender_id, &login_block("Alice"), 1, 10, 10);
    sender.x = 10;
    sender.y = 10;
    world.add_character(sender);
    let mut runtime = ServerRuntime::default();
    runtime.players.insert(1, PlayerRuntime::connected(1, 0));
    runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);

    let result = apply_local_speech_command(&mut world, &runtime, sender_id, "Hello", 123)
        .expect("plain text should be treated as local say");

    assert!(result.sender_messages.is_empty());
    assert_eq!(result.delivered_message_bytes.len(), 1);
    assert_eq!(
        String::from_utf8_lossy(&result.delivered_message_bytes[0].1),
        "Alice says: \"Hello\""
    );
}

#[test]
fn plain_speech_matching_demon_ritual_updates_protection() {
    let mut world = World::default();
    let sender_id = CharacterId(7);
    let mut sender = login_character(sender_id, &login_block("Alice"), 1, 10, 10);
    sender.values[1][CharacterValue::Demon as usize] = 17;
    sender.values[0][CharacterValue::Demon as usize] = 0;
    sender.x = 10;
    sender.y = 10;
    let ritual = ugaris_core::item_driver::demon_ritual_words(sender.id.0, 1);
    world.add_character(sender);
    let mut runtime = ServerRuntime::default();
    runtime.players.insert(1, PlayerRuntime::connected(1, 0));
    runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);

    let result = apply_local_speech_command(&mut world, &runtime, sender_id, &ritual, 123)
        .expect("plain ritual speech should still be local say");

    assert_eq!(
        result.sender_messages,
        vec![
            "You intone the protective ritual.",
            "You sense that this ritual cannot utilize your full knowledge."
        ]
    );
    let sender = world.characters.get(&sender_id).unwrap();
    assert_eq!(sender.values[0][CharacterValue::Demon as usize], 10);
    assert!(sender.flags.contains(CharacterFlags::UPDATE));
    assert_eq!(result.delivered_message_bytes.len(), 1);
}

#[test]
fn underwater_plain_speech_skips_demon_ritual() {
    let mut world = World::default();
    let sender_id = CharacterId(7);
    let mut sender = login_character(sender_id, &login_block("Alice"), 1, 10, 10);
    sender.values[1][CharacterValue::Demon as usize] = 17;
    sender.x = 10;
    sender.y = 10;
    let ritual = ugaris_core::item_driver::demon_ritual_words(sender.id.0, 1);
    world.add_character(sender);
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::UNDERWATER);
    let mut runtime = ServerRuntime::default();
    runtime.players.insert(1, PlayerRuntime::connected(1, 0));
    runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);

    let result = apply_local_speech_command(&mut world, &runtime, sender_id, &ritual, 123)
        .expect("plain underwater speech should still be handled");

    assert!(result.sender_messages.is_empty());
    assert_eq!(
        world.characters.get(&sender_id).unwrap().values[0][CharacterValue::Demon as usize],
        0
    );
    assert_eq!(
        String::from_utf8_lossy(&result.delivered_message_bytes[0].1),
        "Alice says: \"Blub.\""
    );
}

#[test]
fn local_speech_command_preserves_mute_and_quote_rejection() {
    let mut world = World::default();
    let sender_id = CharacterId(7);
    let mut sender = login_character(sender_id, &login_block("Alice"), 1, 10, 10);
    sender.flags.insert(CharacterFlags::SHUTUP);
    sender.x = 10;
    sender.y = 10;
    world.add_character(sender);
    let mut runtime = ServerRuntime::default();
    runtime.players.insert(1, PlayerRuntime::connected(1, 0));
    runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);

    let muted = apply_local_speech_command(&mut world, &runtime, sender_id, "/whisper Hi", 1)
        .expect("whisper should be recognized");
    assert_eq!(
        muted.sender_messages,
        vec!["Sorry, you cannot say anything right now."]
    );

    world
        .characters
        .get_mut(&sender_id)
        .unwrap()
        .flags
        .remove(CharacterFlags::SHUTUP);
    let quoted =
        apply_local_speech_command(&mut world, &runtime, sender_id, "/whisper bad\"quote", 1)
            .expect("whisper should be recognized");
    assert!(quoted.sender_messages.is_empty());
    assert!(quoted.delivered_message_bytes.is_empty());
}

#[test]
fn local_speech_uses_runtime_communication_settings() {
    let mut world = World::default();
    let sender_id = CharacterId(7);
    let near_id = CharacterId(8);
    let far_id = CharacterId(9);
    let mut sender = login_character(sender_id, &login_block("Alice"), 1, 10, 10);
    sender.endurance = 2 * POWERSCALE;
    sender.x = 10;
    sender.y = 10;
    world.add_character(sender);
    let mut near = login_character(near_id, &login_block("Bob"), 1, 14, 10);
    near.x = 14;
    near.y = 10;
    world.add_character(near);
    let mut far = login_character(far_id, &login_block("Cara"), 1, 16, 10);
    far.x = 16;
    far.y = 10;
    world.add_character(far);

    let mut runtime = ServerRuntime::default();
    runtime.shout_dist = 4;
    runtime.quietsay_dist = 4;
    runtime.shout_cost = 2 * POWERSCALE;
    for (session, id) in [(1, sender_id), (2, near_id), (3, far_id)] {
        runtime
            .players
            .insert(session, PlayerRuntime::connected(session, 0));
        runtime.players.get_mut(&session).unwrap().character_id = Some(id);
    }

    let shouted = apply_local_speech_command(&mut world, &runtime, sender_id, "/shout Hi", 91)
        .expect("shout should be recognized");
    let mut shout_targets = shouted
        .delivered_message_bytes
        .iter()
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    shout_targets.sort_by_key(|id| id.0);
    assert_eq!(shout_targets, vec![sender_id, near_id]);
    assert_eq!(world.characters.get(&sender_id).unwrap().endurance, 0);

    world.characters.get_mut(&sender_id).unwrap().endurance = 0;
    let tired = apply_local_speech_command(&mut world, &runtime, sender_id, "/shout Again", 92)
        .expect("shout should be recognized");
    assert_eq!(
        tired.sender_messages,
        vec!["You're too exhausted to shout."]
    );
    assert!(tired.delivered_message_bytes.is_empty());

    let murmured = apply_local_speech_command(&mut world, &runtime, sender_id, "/murmur hush", 93)
        .expect("murmur should be recognized");
    let mut murmur_targets = murmured
        .delivered_message_bytes
        .iter()
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    murmur_targets.sort_by_key(|id| id.0);
    assert_eq!(murmur_targets, vec![sender_id, near_id]);
}

#[test]
fn underwater_speech_falls_back_to_blub_without_shout_cost() {
    let mut world = World::default();
    let sender_id = CharacterId(7);
    let mut sender = login_character(sender_id, &login_block("Alice"), 1, 10, 10);
    sender.endurance = 0;
    sender.x = 10;
    sender.y = 10;
    world.add_character(sender);
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::UNDERWATER);
    let mut runtime = ServerRuntime::default();
    runtime.players.insert(1, PlayerRuntime::connected(1, 0));
    runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);

    let result = apply_local_speech_command(&mut world, &runtime, sender_id, "/shout Help", 77)
        .expect("shout should be recognized");
    assert_eq!(result.sender_messages, Vec::<String>::new());
    assert_eq!(
        String::from_utf8_lossy(&result.delivered_message_bytes[0].1),
        "Alice says: \"Blub.\""
    );
    assert_eq!(world.characters.get(&sender_id).unwrap().endurance, 0);
}

#[test]
fn emote_commands_preserve_legacy_shortcuts_and_quotes() {
    let mut world = World::default();
    let sender_id = CharacterId(7);
    let target_id = CharacterId(8);
    let mut sender = login_character(sender_id, &login_block("Alice"), 1, 10, 10);
    sender.x = 10;
    sender.y = 10;
    world.add_character(sender);
    let mut target = login_character(target_id, &login_block("Bob"), 1, 11, 10);
    target.x = 11;
    target.y = 10;
    world.add_character(target);
    let mut runtime = ServerRuntime::default();
    for (session, id) in [(1, sender_id), (2, target_id)] {
        runtime
            .players
            .insert(session, PlayerRuntime::connected(session, 0));
        runtime.players.get_mut(&session).unwrap().character_id = Some(id);
    }

    let emote = apply_local_speech_command(&mut world, &runtime, sender_id, "/em jumps", 1)
        .expect("legacy /em prefix should be recognized");
    assert_eq!(
        String::from_utf8_lossy(&emote.delivered_message_bytes[0].1),
        "Alice jumps."
    );

    let slap = apply_local_speech_command(&mut world, &runtime, sender_id, "/slap Bob", 1)
        .expect("slap should be recognized");
    assert_eq!(
        String::from_utf8_lossy(&slap.delivered_message_bytes[0].1),
        "Alice slaps Bob around a bit with a large trout."
    );

    let wave = apply_local_speech_command(&mut world, &runtime, sender_id, "/wa", 1)
        .expect("legacy wave abbreviation should be recognized");
    assert_eq!(
        String::from_utf8_lossy(&wave.delivered_message_bytes[0].1),
        "Alice waves happily."
    );

    let quoted = apply_local_speech_command(&mut world, &runtime, sender_id, "/me bad\"quote", 1)
        .expect("me should be recognized");
    assert!(quoted.sender_messages.is_empty());
    assert!(quoted.delivered_message_bytes.is_empty());
}

#[test]
fn underwater_emote_uses_legacy_feels_wet_text() {
    let mut world = World::default();
    let sender_id = CharacterId(7);
    let mut sender = login_character(sender_id, &login_block("Alice"), 1, 10, 10);
    sender.x = 10;
    sender.y = 10;
    world.add_character(sender);
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::UNDERWATER);
    let mut runtime = ServerRuntime::default();
    runtime.players.insert(1, PlayerRuntime::connected(1, 0));
    runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);

    let result = apply_local_speech_command(&mut world, &runtime, sender_id, "/me waves", 1)
        .expect("underwater emote should be recognized");

    assert_eq!(
        String::from_utf8_lossy(&result.delivered_message_bytes[0].1),
        "Alice feels wet."
    );
}

#[test]
fn chat_command_preserves_join_and_access_gates() {
    let mut world = World::default();
    let sender_id = CharacterId(7);
    world.add_character(login_character(sender_id, &login_block("Alice"), 1, 10, 10));
    let mut runtime = ServerRuntime::default();
    runtime.players.insert(1, PlayerRuntime::connected(1, 0));
    runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);

    let unjoined = apply_chat_command(&world, &mut runtime, sender_id, "/gossip Hello", 1)
        .expect("chat command should be recognized");
    assert_eq!(
        unjoined.sender_messages,
        vec!["You must join a channel before you can use it."]
    );

    runtime.players.get_mut(&1).unwrap().chat_channels = 1_u32 << 31;
    let god_denied = apply_chat_command(&world, &mut runtime, sender_id, "/god Hello", 1)
        .expect("god chat command should be recognized");
    assert_eq!(god_denied.sender_messages, vec!["Access denied."]);

    let empty = apply_chat_command(&world, &mut runtime, sender_id, "/c2   ", 1)
        .expect("c2 chat command should be recognized");
    assert_eq!(
        empty.sender_messages,
        vec!["You cannot send empty chat messages."]
    );
}

#[test]
fn chat_command_filters_ignored_sender_and_private_clan_channel() {
    let mut world = World::default();
    let sender_id = CharacterId(7);
    let target_id = CharacterId(8);
    let outsider_id = CharacterId(9);
    let mut sender = login_character(sender_id, &login_block("Alice"), 1, 10, 10);
    sender.clan = 4;
    world.add_character(sender);
    let mut target = login_character(target_id, &login_block("Bob"), 1, 11, 10);
    target.clan = 4;
    world.add_character(target);
    let mut outsider = login_character(outsider_id, &login_block("Eve"), 1, 12, 10);
    outsider.clan = 5;
    world.add_character(outsider);

    let mut runtime = ServerRuntime::default();
    for (session, id) in [(1, sender_id), (2, target_id), (3, outsider_id)] {
        runtime
            .players
            .insert(session, PlayerRuntime::connected(session, 0));
        let player = runtime.players.get_mut(&session).unwrap();
        player.character_id = Some(id);
        player.chat_channels = 1_u32 << 6;
    }
    runtime
        .player_for_character_mut(target_id)
        .unwrap()
        .ignored_characters
        .push(sender_id.0);

    let result = apply_chat_command(&world, &mut runtime, sender_id, "/clan2 Secret", 1)
        .expect("clan chat command should be recognized");

    assert_eq!(
        result
            .delivered_message_bytes
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>(),
        vec![sender_id]
    );
}

#[test]
fn chat_command_forwards_private_channel_to_spying_god() {
    let mut world = World::default();
    let sender_id = CharacterId(7);
    let same_clan_id = CharacterId(8);
    let spy_id = CharacterId(9);
    let mut sender = login_character(sender_id, &login_block("Alice"), 1, 10, 10);
    sender.clan = 4;
    world.add_character(sender);
    let mut same_clan = login_character(same_clan_id, &login_block("Bob"), 1, 11, 10);
    same_clan.clan = 4;
    world.add_character(same_clan);
    let mut spy = login_character(spy_id, &login_block("God"), 1, 12, 10);
    spy.flags.insert(CharacterFlags::GOD | CharacterFlags::SPY);
    world.add_character(spy);

    let mut runtime = ServerRuntime::default();
    for (session, id) in [(1, sender_id), (2, same_clan_id), (3, spy_id)] {
        runtime
            .players
            .insert(session, PlayerRuntime::connected(session, 0));
        let player = runtime.players.get_mut(&session).unwrap();
        player.character_id = Some(id);
        player.chat_channels = 1_u32 << 6;
    }

    let result = apply_chat_command(&world, &mut runtime, sender_id, "/clan2 Secret", 1)
        .expect("clan chat command should be recognized");

    let deliveries = result
        .delivered_message_bytes
        .iter()
        .map(|(id, bytes)| (*id, String::from_utf8_lossy(bytes).into_owned()))
        .collect::<Vec<_>>();
    assert_eq!(deliveries.len(), 3);
    assert!(deliveries
        .iter()
        .any(|(id, text)| *id == spy_id && text.contains("[SPY/CLAN]")));
    assert!(result
        .delivered_message_bytes
        .iter()
        .find(|(id, _)| *id == spy_id)
        .unwrap()
        .1
        .starts_with(COL_DARK_GRAY));
}

#[test]
fn chat_command_delivers_alliance_channel_to_allied_clan_not_just_own_clan() {
    // C `chat.c:284`: `channel == 12` delivery skips only when the target is
    // neither in the sender's own clan *nor* allied to it (`clan_alliance`).
    let mut world = World::default();
    let sender_id = CharacterId(7);
    let ally_id = CharacterId(8);
    let neutral_id = CharacterId(9);
    let mut sender = login_character(sender_id, &login_block("Alice"), 1, 10, 10);
    sender.clan = 1;
    world.add_character(sender);
    let mut ally = login_character(ally_id, &login_block("Bob"), 1, 11, 10);
    ally.clan = 2;
    world.add_character(ally);
    let mut neutral = login_character(neutral_id, &login_block("Eve"), 1, 12, 10);
    neutral.clan = 3;
    world.add_character(neutral);

    let relations = world.clan_registry.relations_mut();
    relations.found_clan(1, 0);
    relations.found_clan(2, 0);
    relations.found_clan(3, 0);
    relations
        .set_relation(1, 2, ugaris_core::clan::ClanRelation::Alliance, 0)
        .unwrap();
    relations
        .set_relation(2, 1, ugaris_core::clan::ClanRelation::Alliance, 0)
        .unwrap();
    relations.update(0);
    assert_eq!(
        relations.current_relation(1, 2),
        ugaris_core::clan::ClanRelation::Alliance
    );

    let mut runtime = ServerRuntime::default();
    for (session, id) in [(1, sender_id), (2, ally_id), (3, neutral_id)] {
        runtime
            .players
            .insert(session, PlayerRuntime::connected(session, 0));
        let player = runtime.players.get_mut(&session).unwrap();
        player.character_id = Some(id);
        player.chat_channels = 1_u32 << 11;
    }

    let result = apply_chat_command(&world, &mut runtime, sender_id, "/clana Secret", 1)
        .expect("alliance chat command should be recognized");

    let delivered_to: Vec<CharacterId> = result
        .delivered_message_bytes
        .iter()
        .map(|(id, _)| *id)
        .collect();
    assert!(delivered_to.contains(&sender_id));
    assert!(delivered_to.contains(&ally_id));
    assert!(!delivered_to.contains(&neutral_id));
}

#[test]
fn chat_command_skips_spy_forward_for_allied_clan_god_already_in_channel() {
    // C `chat.c:184-193`: a spying god who is already in the alliance
    // channel and either shares the sender's clan *or* is allied to it must
    // not get a duplicate `[SPY/ALLIANCE]` forward - they'd already see the
    // real message through the normal delivery loop.
    let mut world = World::default();
    let sender_id = CharacterId(7);
    let ally_god_id = CharacterId(8);
    let mut sender = login_character(sender_id, &login_block("Alice"), 1, 10, 10);
    sender.clan = 1;
    world.add_character(sender);
    let mut ally_god = login_character(ally_god_id, &login_block("God"), 1, 11, 10);
    ally_god.clan = 2;
    ally_god
        .flags
        .insert(CharacterFlags::GOD | CharacterFlags::SPY);
    world.add_character(ally_god);

    let relations = world.clan_registry.relations_mut();
    relations.found_clan(1, 0);
    relations.found_clan(2, 0);
    relations
        .set_relation(1, 2, ugaris_core::clan::ClanRelation::Alliance, 0)
        .unwrap();
    relations
        .set_relation(2, 1, ugaris_core::clan::ClanRelation::Alliance, 0)
        .unwrap();
    relations.update(0);

    let mut runtime = ServerRuntime::default();
    for (session, id) in [(1, sender_id), (2, ally_god_id)] {
        runtime
            .players
            .insert(session, PlayerRuntime::connected(session, 0));
        let player = runtime.players.get_mut(&session).unwrap();
        player.character_id = Some(id);
        player.chat_channels = 1_u32 << 11;
    }

    let result = apply_chat_command(&world, &mut runtime, sender_id, "/clana Secret", 1)
        .expect("alliance chat command should be recognized");

    // The allied god sees it once (via the normal alliance-channel delivery
    // loop), never a second `[SPY/ALLIANCE]` copy.
    let deliveries: Vec<_> = result
        .delivered_message_bytes
        .iter()
        .filter(|(id, _)| *id == ally_god_id)
        .collect();
    assert_eq!(deliveries.len(), 1);
    assert!(!String::from_utf8_lossy(&deliveries[0].1).contains("[SPY/"));
}

#[test]
fn who_command_preserves_legacy_short_prefix_match() {
    let mut world = World::default();
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    character.level = 9;
    world.add_character(character);

    let result = apply_who_command(&world, None, CharacterFlags::empty(), "/w")
        .expect("legacy cmdcmp accepts short who");

    assert_eq!(
        result.messages,
        vec!["Currently online in this area:", "Tester (9)"]
    );
    assert!(apply_who_command(&world, None, CharacterFlags::empty(), "/whostaff").is_none());
}

#[test]
fn nowho_command_toggles_staff_visibility_only_for_staff_or_gods() {
    let mut world = World::default();
    let mut staff = login_character(CharacterId(7), &login_block("Staffer"), 1, 10, 10);
    staff.flags.insert(CharacterFlags::STAFF);
    world.add_character(staff);

    let result = apply_nowho_command(&mut world, CharacterId(7), "/nowho")
        .expect("staff nowho should be recognized");
    assert_eq!(result.messages, vec!["NoWho enabled."]);
    assert!(world
        .characters
        .get(&CharacterId(7))
        .expect("staff exists")
        .flags
        .contains(CharacterFlags::NOWHO));

    let result = apply_nowho_command(&mut world, CharacterId(7), "/nowho")
        .expect("staff nowho should toggle off");
    assert_eq!(result.messages, vec!["NoWho disabled."]);

    let player = login_character(CharacterId(8), &login_block("Player"), 1, 11, 10);
    world.add_character(player);
    assert!(apply_nowho_command(&mut world, CharacterId(8), "/nowho").is_none());
    assert!(apply_nowho_command(&mut world, CharacterId(7), "/now").is_none());
}

#[test]
fn ignore_command_toggles_lists_and_clears_legacy_feedback() {
    let source_id = CharacterId(7);
    let target_id = CharacterId(8);
    let mut world = World::default();
    world.add_character(login_character(
        source_id,
        &login_block("Source"),
        1,
        10,
        10,
    ));
    world.add_character(login_character(
        target_id,
        &login_block("Target"),
        1,
        11,
        10,
    ));
    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(source_id);
    runtime.players.insert(1, player);

    let added = apply_ignore_command(&world, &mut runtime, source_id, "/ign target")
        .expect("ignore abbreviation should be recognized");
    let listed = apply_ignore_command(&world, &mut runtime, source_id, "/ignore")
        .expect("ignore list should be recognized");
    let removed = apply_ignore_command(&world, &mut runtime, source_id, "/ignore target")
        .expect("ignore toggle should be recognized");
    let missing = apply_ignore_command(&world, &mut runtime, source_id, "/ignore unknown")
        .expect("ignore missing target should be recognized");
    runtime
        .player_for_character_mut(source_id)
        .unwrap()
        .toggle_ignored_character(target_id.0);
    let cleared = apply_clearignore_command(&mut runtime, source_id, "/clearignore")
        .expect("clearignore should be recognized");

    assert_eq!(added.messages, vec!["Added to ignore list."]);
    assert_eq!(listed.messages, vec!["Ignoring: Target"]);
    assert_eq!(removed.messages, vec!["Deleted from ignore list."]);
    assert_eq!(missing.messages, vec!["No player by that name."]);
    assert_eq!(cleared.messages, vec!["Ignore list is now empty."]);
    assert!(runtime
        .player_for_character(source_id)
        .unwrap()
        .ignored_characters
        .is_empty());
}

#[test]
fn tell_command_respects_ignore_except_staff_mode() {
    let sender_id = CharacterId(7);
    let target_id = CharacterId(8);
    let mut world = World::default();
    world.add_character(login_character(
        sender_id,
        &login_block("Sender"),
        1,
        10,
        10,
    ));
    world.add_character(login_character(
        target_id,
        &login_block("Target"),
        1,
        11,
        10,
    ));
    let mut runtime = ServerRuntime::default();
    let mut sender = PlayerRuntime::connected(1, 0);
    sender.character_id = Some(sender_id);
    let mut target = PlayerRuntime::connected(2, 0);
    target.character_id = Some(target_id);
    target.toggle_ignored_character(sender_id.0);
    runtime.players.insert(1, sender);
    runtime.players.insert(2, target);

    let blocked = apply_tell_command(&world, &mut runtime, sender_id, "/tell target hello", 10)
        .expect("tell should be recognized");
    world
        .characters
        .get_mut(&sender_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::STAFF);
    let staff = apply_tell_command(&world, &mut runtime, sender_id, "/tell target hello", 11)
        .expect("staff tell should be recognized");

    assert_eq!(blocked.sender_messages, vec!["Told Target: \"hello\""]);
    assert!(blocked.delivered_messages.is_empty());
    assert_eq!(staff.delivered_messages.len(), 1);
    assert!(staff.delivered_messages[0].1.contains("SENDER"));
}
