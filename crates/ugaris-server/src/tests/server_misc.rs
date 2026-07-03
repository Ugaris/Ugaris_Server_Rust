use super::*;

#[test]
fn shout_and_holler_apply_legacy_endurance_costs() {
    let mut world = World::default();
    let sender_id = CharacterId(7);
    let mut sender = login_character(sender_id, &login_block("Alice"), 1, 10, 10);
    sender.endurance = GameSettings::default().shout_cost;
    sender.x = 10;
    sender.y = 10;
    world.add_character(sender);
    let mut runtime = ServerRuntime::default();
    runtime.players.insert(1, PlayerRuntime::connected(1, 0));
    runtime.players.get_mut(&1).unwrap().character_id = Some(sender_id);

    let shouted = apply_local_speech_command(&mut world, &runtime, sender_id, "/shout Hi", 77)
        .expect("shout should be recognized");
    assert_eq!(shouted.delivered_message_bytes.len(), 1);
    let sender = world.characters.get(&sender_id).unwrap();
    assert_eq!(sender.endurance, 0);
    assert_eq!(sender.regen_ticker, 77);

    let tired = apply_local_speech_command(&mut world, &runtime, sender_id, "/holler Hi", 78)
        .expect("holler should be recognized");
    assert_eq!(
        tired.sender_messages,
        vec!["You're too exhausted to holler."]
    );
    assert!(tired.delivered_message_bytes.is_empty());
}

#[test]
fn completed_map_use_clears_pending_use_action() {
    let mut runtime = ServerRuntime::default();
    let character_id = CharacterId(7);
    runtime.players.insert(1, PlayerRuntime::connected(1, 0));
    let player = runtime.players.get_mut(&1).unwrap();
    player.character_id = Some(character_id);
    player.set_pending_action(QueuedAction {
        action: PlayerActionCode::Use,
        arg1: 10,
        arg2: 10,
    });
    let completions = [WorldActionCompletion {
        character_id,
        action_id: action::USE,
        action_item_id: Some(ItemId(10)),
        ok: true,
        legacy_return_code: 1,
        item_use: None,
        old_x: 10,
        old_y: 10,
        new_x: 10,
        new_y: 10,
    }];

    clear_completed_use_actions(&mut runtime, &completions);

    assert_eq!(runtime.players[&1].action.action, PlayerActionCode::Idle);
}

#[test]
fn initial_map_payloads_chunk_modern_view_distance_under_frame_limit() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, LOGIN_SPAWN_X, LOGIN_SPAWN_Y);
    character.x = LOGIN_SPAWN_X as u16;
    character.y = LOGIN_SPAWN_Y as u16;
    let mut world = World::default();
    assert!(world.spawn_character(character.clone(), LOGIN_SPAWN_X, LOGIN_SPAWN_Y));

    let pk_relations = PkRelationSnapshot::default();
    let payloads = initial_map_payloads(&world, &character, &pk_relations, 40);

    assert!(payloads.len() > 1);
    assert!(payloads
        .iter()
        .all(|payload| payload.len() <= MAP_BOOTSTRAP_CHUNK_TARGET));
}

#[test]
fn map_refresh_payloads_start_with_origin_then_map_chunks() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, LOGIN_SPAWN_X, LOGIN_SPAWN_Y);
    character.x = LOGIN_SPAWN_X as u16;
    character.y = LOGIN_SPAWN_Y as u16;
    let mut world = World::default();
    assert!(world.spawn_character(character.clone(), LOGIN_SPAWN_X, LOGIN_SPAWN_Y));

    let pk_relations = PkRelationSnapshot::default();
    let payloads = map_refresh_payloads(&world, &character, &pk_relations, 1);

    assert_eq!(&payloads[0][..], &[SV_ORIGIN, 128, 0, 128, 0]);
    assert_eq!(
        payloads[1][0],
        SV_MAP01 | SV_MAPPOS | MAP_EFFECT_0 | MAP_EFFECT_1 | MAP_EFFECT_2 | MAP_EFFECT_3,
        "full refresh stomps every cell starting with its effect pointers"
    );
}

#[test]
fn look_map_payload_hidden_target_matches_legacy_feedback() {
    let payloads = look_map_payloads(
        &World::default(),
        1,
        LookMapRequest {
            character_id: CharacterId(7),
            x: 12,
            y: 13,
            character_level: 0,
            visible: false,
        },
    );

    assert_eq!(text_payloads(&payloads), vec!["Too far away or hidden."]);
}

#[test]
fn runtime_finds_sessions_for_character_refresh() {
    let mut runtime = ServerRuntime::default();
    let (commands, _rx) = mpsc::channel(1);
    runtime.connect(5, commands, 10);
    let character_id = runtime.login(5, &login_block("Tester"), 11);

    assert_eq!(runtime.sessions_for_character(character_id), vec![(5, 40)]);
    assert!(runtime.sessions_for_character(CharacterId(99)).is_empty());
}

#[test]
fn runtime_character_ids_can_be_seeded_after_loaded_world_characters() {
    let mut runtime = ServerRuntime::default();
    runtime.set_next_character_id(189);
    let (commands, _rx) = mpsc::channel(1);
    runtime.connect(5, commands, 10);

    assert_eq!(
        runtime.login(5, &login_block("Tester"), 11),
        CharacterId(189)
    );
}

#[test]
fn next_available_character_id_follows_loaded_world_characters() {
    let mut world = World::default();
    assert!(world.spawn_character(
        login_character(CharacterId(12), &login_block("Npc"), 1, 10, 10),
        10,
        10,
    ));

    assert_eq!(next_available_character_id(&world), 13);
}

#[test]
fn resource_percent_matches_legacy_scaled_resource_math() {
    assert_eq!(resource_percent(50 * POWERSCALE, 50), 100);
    assert_eq!(resource_percent(25 * POWERSCALE, 50), 50);
    assert_eq!(resource_percent(-1, 50), 0);
}

#[test]
fn tick_flush_packs_buffered_payloads_into_one_legacy_frame() {
    let mut runtime = ServerRuntime::default();
    let (commands, mut rx) = mpsc::channel(16);
    runtime.connect(5, commands, 10);
    runtime.login(5, &login_block("Tester"), 11);

    assert!(runtime.send_to_session(5, bytes::BytesMut::from(&[1u8, 2][..])));
    assert!(runtime.send_to_session(5, bytes::BytesMut::from(&[3u8][..])));
    assert!(runtime.send_to_session(5, bytes::BytesMut::from(&[4u8, 5, 6][..])));
    // Nothing hits the wire before the flush.
    assert!(rx.try_recv().is_err());

    runtime.flush_tick_frames(true);

    match rx.try_recv() {
        Ok(ugaris_net::SessionCommand::Send(frame)) => {
            assert_eq!(&frame[..], &[1, 2, 3, 4, 5, 6]);
        }
        other => panic!("expected one packed frame, got {other:?}"),
    }
    assert!(
        rx.try_recv().is_err(),
        "one tick flush sends exactly one frame"
    );
}

#[test]
fn tick_flush_sends_empty_frame_for_idle_logged_in_sessions() {
    let mut runtime = ServerRuntime::default();
    let (commands, mut rx) = mpsc::channel(16);
    runtime.connect(5, commands, 10);
    runtime.login(5, &login_block("Tester"), 11);
    if let Some(player) = runtime.players.get_mut(&5) {
        player.state = ugaris_core::player::PlayerConnectionState::Normal;
    }

    runtime.flush_tick_frames(true);
    match rx.try_recv() {
        Ok(ugaris_net::SessionCommand::Send(frame)) => assert!(frame.is_empty()),
        other => panic!("expected empty tick frame, got {other:?}"),
    }

    // Out-of-tick flushes never inject fake empty ticks.
    runtime.flush_tick_frames(false);
    assert!(rx.try_recv().is_err());
}
