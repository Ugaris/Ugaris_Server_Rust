use super::*;

fn connected_normal_session(
    session_id: u64,
    character_id: CharacterId,
) -> (ServerRuntime, mpsc::Receiver<ugaris_net::SessionCommand>) {
    let mut runtime = ServerRuntime::default();
    let (commands, rx) = mpsc::channel(16);
    runtime.connect(session_id, commands, 10);
    if let Some(player) = runtime.players.get_mut(&session_id) {
        player.state = PlayerConnectionState::Normal;
        player.character_id = Some(character_id);
        player.character_number = character_id.0;
    }
    (runtime, rx)
}

fn recv_frame(rx: &mut mpsc::Receiver<ugaris_net::SessionCommand>) -> Vec<u8> {
    match rx.try_recv() {
        Ok(ugaris_net::SessionCommand::Send(frame)) => frame.to_vec(),
        other => panic!("expected a sent frame, got {other:?}"),
    }
}

#[test]
fn resource_sync_sends_nothing_when_neither_flag_is_set() {
    let character_id = CharacterId(7);
    let (mut runtime, _rx) = connected_normal_session(5, character_id);
    let mut world = World::default();
    world.add_character(login_character(
        character_id,
        &login_block("Tester"),
        1,
        10,
        10,
    ));

    let sent = queue_resource_sync_frames(&mut runtime, &mut world);
    assert_eq!(sent, 0);

    // Nothing was queued for the session.
    runtime.flush_tick_frames(false);
    assert!(runtime.tick_out.get(&5).is_none_or(Vec::is_empty));
}

#[test]
fn resource_sync_sends_values_hp_and_exp_on_update_flag_and_clears_it() {
    let character_id = CharacterId(7);
    let (mut runtime, mut rx) = connected_normal_session(5, character_id);
    let mut world = World::default();
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::UPDATE);
    character.hp = 42 * POWERSCALE;
    character.exp = 1234;
    character.values[0][CharacterValue::Hp as usize] = 77;
    character.values[1][CharacterValue::Hp as usize] = 60;
    world.add_character(character);

    let sent = queue_resource_sync_frames(&mut runtime, &mut world);
    assert_eq!(sent, 1);

    runtime.flush_tick_frames(true);
    let frame = recv_frame(&mut rx);

    // SV_SETVAL0 for the Hp slot: [opcode, slot, value_lo, value_hi].
    let hp_slot = CharacterValue::Hp as u8;
    let value0_packet = [SV_SETVAL0, hp_slot, 77, 0];
    let value1_packet = [SV_SETVAL1, hp_slot, 60, 0];
    assert!(
        frame.windows(4).any(|window| window == value0_packet),
        "expected SV_SETVAL0 for hp slot in {frame:?}"
    );
    assert!(
        frame.windows(4).any(|window| window == value1_packet),
        "expected SV_SETVAL1 for hp slot in {frame:?}"
    );

    // SV_SETHP: [opcode, hp_lo, hp_hi] with hp scaled down by POWERSCALE.
    let hp_packet = [SV_SETHP, 42, 0];
    assert!(
        frame.windows(3).any(|window| window == hp_packet),
        "expected SV_SETHP(42) in {frame:?}"
    );

    let character = world.characters.get(&character_id).unwrap();
    assert!(!character.flags.contains(CharacterFlags::UPDATE));
    // ITEMS was never set, so it should not have been touched.
    assert!(!character.flags.contains(CharacterFlags::ITEMS));
}

#[test]
fn resource_sync_sends_inventory_and_gold_on_items_flag_and_clears_it() {
    let character_id = CharacterId(7);
    let (mut runtime, mut rx) = connected_normal_session(5, character_id);
    let mut world = World::default();
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::ITEMS);
    character.gold = 555;
    world.add_character(character);

    let sent = queue_resource_sync_frames(&mut runtime, &mut world);
    assert_eq!(sent, 1);

    runtime.flush_tick_frames(true);
    let frame = recv_frame(&mut rx);

    assert!(frame.contains(&SV_SETCITEM));
    assert!(frame.contains(&SV_GOLD));

    let character = world.characters.get(&character_id).unwrap();
    assert!(!character.flags.contains(CharacterFlags::ITEMS));
    // UPDATE was never set, so it should not have been touched.
    assert!(!character.flags.contains(CharacterFlags::UPDATE));
}

#[test]
fn resource_sync_handles_both_flags_in_one_frame_and_clears_both() {
    let character_id = CharacterId(7);
    let (mut runtime, mut rx) = connected_normal_session(5, character_id);
    let mut world = World::default();
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character
        .flags
        .insert(CharacterFlags::UPDATE | CharacterFlags::ITEMS);
    world.add_character(character);

    let sent = queue_resource_sync_frames(&mut runtime, &mut world);
    assert_eq!(sent, 1);

    runtime.flush_tick_frames(true);
    let frame = recv_frame(&mut rx);
    assert!(frame.contains(&SV_SETCITEM));
    assert!(frame.contains(&SV_SETVAL0));

    let character = world.characters.get(&character_id).unwrap();
    assert!(!character.flags.contains(CharacterFlags::UPDATE));
    assert!(!character.flags.contains(CharacterFlags::ITEMS));
}

#[test]
fn resource_sync_skips_sessions_not_in_normal_state() {
    let character_id = CharacterId(7);
    let mut runtime = ServerRuntime::default();
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(5, commands, 10);
    if let Some(player) = runtime.players.get_mut(&5) {
        player.character_id = Some(character_id);
        // Left in the default (non-Normal) connection state.
    }
    let mut world = World::default();
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::UPDATE);
    world.add_character(character);

    let sent = queue_resource_sync_frames(&mut runtime, &mut world);
    assert_eq!(sent, 0);

    // The flag stays set since the session was never flushed to.
    let character = world.characters.get(&character_id).unwrap();
    assert!(character.flags.contains(CharacterFlags::UPDATE));
}
