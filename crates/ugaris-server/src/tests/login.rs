use super::*;

#[test]
fn legacy_questlog_payload_packs_c_bitfield_shape() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.quest_log.open(0);
    player.quest_log.mark_done(1);
    player.mark_random_shrine_used(3);
    player.mark_random_shrine_used(64);

    let payload = legacy_questlog_payload(&player);

    assert_eq!(payload.len(), 1 + 100 + 36);
    assert_eq!(payload[0], SV_QUESTLOG);
    assert_eq!(payload[1], 0x40);
    assert_eq!(payload[2], 0x81);
    assert!(payload[3..101].iter().all(|byte| *byte == 0));
    assert_eq!(&payload[101..105], &(1u32 << 3).to_le_bytes());
    assert_eq!(&payload[109..113], &1u32.to_le_bytes());
    assert!(payload[105..109].iter().all(|byte| *byte == 0));
    assert!(payload[113..].iter().all(|byte| *byte == 0));
}

#[test]
fn login_payload_sends_legacy_session_start_packets() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, LOGIN_SPAWN_X, LOGIN_SPAWN_Y);
    character.x = LOGIN_SPAWN_X as u16;
    character.y = LOGIN_SPAWN_Y as u16;
    let world = World::default();
    let payload = login_payload(&world, &character, 2, 0x0102_0304);

    assert_eq!(payload[0], SV_LOGINDONE);
    assert_eq!(payload[1], SV_TICKER);
    assert_eq!(&payload[2..6], &[3, 3, 2, 1]);
    assert_eq!(payload[6], SV_MIRROR);
    assert_eq!(&payload[7..11], &[2, 0, 0, 0]);
    assert_eq!(payload[11], SV_PROTOCOL);
    assert_eq!(payload[13], SV_ORIGIN);
    assert_eq!(&payload[14..18], &[128, 0, 128, 0]);
    assert_eq!(payload[18], SV_SETVAL0);
    assert_eq!(payload[22], SV_SETVAL1);
    let first_resource_offset = 18 + ugaris_core::entity::CHARACTER_VALUE_COUNT * 8;
    assert_eq!(payload[first_resource_offset], SV_SETHP);
    assert_eq!(
        payload[payload.len() - LOGIN_ACCEPTED_MESSAGE.len() - 3],
        SV_TEXT
    );
}

#[test]
fn live_quest_toggle_commands_preserve_legacy_gates_and_feedback() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::LQMASTER);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/im", 1).is_none()
    );

    let immortal = apply_admin_character_command(&mut world, &mut runtime, character_id, "/im", 20)
        .expect("area-20 lqmaster immortal command should be recognized");
    assert_eq!(immortal.messages, vec!["Immortal is on."]);
    assert!(world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::IMMORTAL));

    let infrared =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/inf", 20)
            .expect("area-20 lqmaster infrared command should be recognized");
    assert_eq!(infrared.messages, vec!["Infrared is on."]);
    assert!(world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::INFRARED));

    let invisible =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/inv", 20)
            .expect("area-20 lqmaster invisible command should be recognized");
    assert_eq!(invisible.messages, vec!["Invisible is on."]);
    assert!(world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::INVISIBLE));
}

#[test]
fn login_bootstrap_payloads_include_visible_client_effect_slots() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
    character.x = 10;
    character.y = 10;
    let mut world = World::default();
    assert!(world.spawn_character(character.clone(), 10, 10));
    let mut effect = Effect::new(EF_FIREBALL, 123, 55, 65);
    effect.from_x = 10;
    effect.from_y = 10;
    effect.to_x = 12;
    effect.to_y = 10;
    effect.x = 11 * 1024 + 512;
    effect.y = 10 * 1024 + 512;
    world.effects.insert(123, effect);
    let mut effect_cache = ClientEffectCache::default();

    let pk_relations = PkRelationSnapshot::default();
    let weather = WeatherState::default();
    let payloads = login_bootstrap_payloads(
        &world,
        &character,
        &pk_relations,
        1,
        10,
        2,
        &mut effect_cache,
        &weather,
    );

    assert!(payloads.iter().any(|payload| {
        payload.first().copied() == Some(ugaris_protocol::packet::SV_CEFFECT)
            && payload.get(1).copied() == Some(0)
            && payload[2..].starts_with(&ugaris_protocol::packet::ceffect_fireball(
                123, 55, 10, 10, 12, 10,
            ))
    }));
    assert!(payloads
        .iter()
        .any(|payload| &payload[..] == &ugaris_protocol::packet::used_effects(1)[..]));
    assert!(client_effect_payloads(&world, &character, 2, &mut effect_cache).is_empty());
}

#[test]
fn runtime_login_allocates_character_and_disconnect_returns_it() {
    let mut runtime = ServerRuntime::default();
    let (commands, _rx) = mpsc::channel(1);

    runtime.connect(5, commands, 10);
    let character_id = runtime.login(5, &login_block("Tester"), 11);

    let player = runtime.players.get(&5).unwrap();
    assert_eq!(character_id, CharacterId(1));
    assert_eq!(player.character_id, Some(CharacterId(1)));
    assert_eq!(player.character_number, 1);
    assert_eq!(player.state, PlayerConnectionState::Normal);
    assert_eq!(
        runtime.disconnect(5).and_then(|player| player.character_id),
        Some(CharacterId(1))
    );
}

#[test]
fn character_save_request_encodes_runtime_ppd_and_carried_items() {
    let login = login_block("Tester");
    let mut character = login_character(CharacterId(7), &login, 1, 10, 10);
    character.inventory[30] = Some(ItemId(101));
    character.cursor_item = Some(ItemId(102));

    let mut inventory_item = test_item(ItemId(101), 1, ItemFlags::TAKE);
    inventory_item.carried_by = Some(character.id);
    let mut cursor_item = test_item(ItemId(102), 2, ItemFlags::TAKE);
    cursor_item.carried_by = Some(character.id);
    let ground_item = test_item(ItemId(103), 3, ItemFlags::TAKE);

    let mut world = World::default();
    world.add_character(character.clone());
    world.add_item(inventory_item);
    world.add_item(cursor_item);
    world.add_item(ground_item);

    let mut player = PlayerRuntime::connected(5, 0);
    player.add_keyring_key(0x3b000001, "Copper Key");
    player.mark_chest_access(9, 1234);
    let mut depot = AccountDepotState::default();
    let mut stored = test_item(ItemId(201), 4321, ItemFlags::USED | ItemFlags::TAKE);
    stored.name = "Depot Relic".to_string();
    depot.slots[4] = Some(stored);

    let request = character_save_request(&world, &player, &character, Some(&depot), 1, 2);

    assert_eq!(request.items.len(), 2);
    assert!(request.items.iter().any(|item| item.id == ItemId(101)));
    assert!(request.items.iter().any(|item| item.id == ItemId(102)));
    assert!(matches!(
        request.mode,
        ugaris_db::character::CharacterSaveMode::Logout { mirror: 2, .. }
    ));
    let mut decoded = PlayerRuntime::connected(6, 0);
    assert!(decoded.decode_legacy_ppd_blob(&request.ppd_blob));
    assert_eq!(decoded.keyring.len(), 1);
    assert_eq!(decoded.chest_last_access_seconds(9), 1234);
    let decoded_depot = decode_legacy_account_depot_subscriber_blob(&request.subscriber_blob)
        .expect("account depot subscriber block");
    assert_eq!(decoded_depot.slots[0].as_ref().unwrap().name, "Depot Relic");
}

#[test]
fn login_reject_message_matches_legacy_find_login_switch() {
    // C `read_login` (`src/system/player.c:396-444`): every non-`Ready`
    // `find_login` code maps to an exact reject string sent via
    // `player_client_exit`. `Ready`/`Waiting` never reject.
    assert_eq!(
        login_reject_message(&LoginOutcome::Ready {
            character_id: CharacterId(1),
            character_number: 1,
            mirror: 1,
            unique: 0,
        }),
        None
    );
    assert_eq!(login_reject_message(&LoginOutcome::Waiting), None);
    assert_eq!(
        login_reject_message(&LoginOutcome::InternalError),
        Some("Internal error. Please try again. If several retries fail email game@ugaris.com.")
    );
    assert_eq!(
        login_reject_message(&LoginOutcome::Locked),
        Some("You have been banned. Please email game@ugaris.com for details.")
    );
    assert_eq!(
        login_reject_message(&LoginOutcome::WrongPassword),
        Some("Username or password wrong.")
    );
    assert_eq!(
        login_reject_message(&LoginOutcome::Duplicate),
        Some("Duplicate login. Please make sure no other character from your account is active.")
    );
    assert_eq!(
        login_reject_message(&LoginOutcome::NotPaid),
        Some("Your account has not been paid.")
    );
    assert_eq!(
        login_reject_message(&LoginOutcome::Shutdown),
        Some("The server is being shut down. Please try again in a few minutes.")
    );
    assert_eq!(
        login_reject_message(&LoginOutcome::IpLocked),
        Some(
            "Your IP address is banned. Please email game@ugaris.com with your account ID and ask for an exception to be made."
        )
    );
    assert_eq!(
        login_reject_message(&LoginOutcome::AccountNotFixed),
        Some(
            "Please log onto your account management on www.ugaris.com and update the account ownership information. Scroll down to 'Address Information' and choose 'Edit'."
        )
    );
    assert_eq!(
        login_reject_message(&LoginOutcome::TooManyBadPasswords),
        Some("Too many tries with bad passwords. Please come back later.")
    );
    assert_eq!(
        login_reject_message(&LoginOutcome::NewArea {
            character_id: CharacterId(1),
            area_id: 2,
            mirror: 1,
            unique: 0,
        }),
        Some(
            "Target area server is down. Your character is being transfered to a different area. Please try again."
        )
    );
}

#[test]
fn runtime_login_rejects_wrong_password_with_sv_exit_and_disconnects() {
    // End-to-end wiring check for the `SessionEvent::Login` handler's reject
    // path: build the exact `SV_EXIT` payload the way `main.rs` does for a
    // rejected DB login and confirm it matches the legacy byte layout (C
    // `player_client_exit`, `src/system/player.c:260`) instead of a
    // scaffold login-accepted payload.
    let mut runtime = ServerRuntime::default();
    let (commands, mut rx) = mpsc::channel(4);
    runtime.connect(5, commands, 10);

    let reason = login_reject_message(&LoginOutcome::WrongPassword).unwrap();
    let mut builder = PacketBuilder::new();
    builder.exit(reason);
    let payload = builder.into_payload();
    runtime.send_to_session(5, payload.clone());
    runtime.flush_session(5);
    if let Some(commands) = runtime.sessions.get(&5) {
        let _ = commands.try_send(SessionCommand::Disconnect);
    }

    assert_eq!(payload[0], ugaris_protocol::packet::SV_EXIT);
    assert_eq!(payload[1] as usize, reason.len());
    assert_eq!(&payload[2..], reason.as_bytes());

    let SessionCommand::Send(sent_frame) = rx.try_recv().expect("exit frame queued") else {
        panic!("expected a Send command carrying the exit frame");
    };
    assert!(sent_frame
        .windows(reason.len())
        .any(|w| w == reason.as_bytes()));
    assert!(matches!(rx.try_recv(), Ok(SessionCommand::Disconnect)));
}

#[test]
fn login_character_uses_full_scaled_resources() {
    let character = login_character(CharacterId(3), &login_block("Tester"), 12, 42, 43);

    assert_eq!(character.name, "Tester");
    assert!(character.flags.contains(CharacterFlags::PLAYER));
    assert_eq!(character.sprite, 1);
    assert_eq!(character.rest_area, 12);
    assert_eq!((character.rest_x, character.rest_y), (42, 43));
    assert_eq!(character.hp, 50 * POWERSCALE);
    assert_eq!(character.values[0][CharacterValue::Hp as usize], 50);
    assert_eq!(character.values[1][CharacterValue::Hp as usize], 50);
}
