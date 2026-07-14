use super::*;

#[test]
fn character_snapshot_restores_active_legacy_shutup_ppd() {
    let target_id = CharacterId(8);
    let character = login_character(target_id, &login_block("Target"), 1, 11, 10);

    let mut persisted = PlayerRuntime::connected(99, 0);
    persisted.shutup_until_seconds = 700;
    let ppd_blob = persisted.encode_legacy_ppd_blob(&[]);

    let snapshot = CharacterSnapshot {
        player_state_json: None,
        character,
        items: Vec::new(),
        ppd_blob,
        subscriber_blob: Vec::new(),
        current_area: 1,
        current_mirror: 1,
        allowed_area: 1,
        mirror: 1,
    };
    let mut world = World::default();
    let mut player = PlayerRuntime::connected(1, 0);

    let result = apply_character_snapshot(&mut world, &mut player, snapshot, 11, 10, 100);
    assert!(result.loaded);
    assert_eq!(player.shutup_until_seconds, 700);
    assert!(world
        .characters
        .get(&target_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::SHUTUP));
}

/// Bad persisted data (hand-edited/corrupted `character_json` with
/// truncated `values`/`inventory`/`professions` vectors) must never crash
/// the server: `apply_character_snapshot` restores the fixed C-array
/// shapes before the character enters the per-tick/per-packet indexing
/// paths (`login_payload`'s `values[0][v]` loop, `inventory_swap_slot`'s
/// `inventory[slot]` write, ...).
#[test]
fn character_snapshot_with_malformed_shape_does_not_panic() {
    let target_id = CharacterId(8);
    let mut character = login_character(target_id, &login_block("Target"), 1, 11, 10);
    character.values = vec![vec![5; 3], vec![7; 3]]; // two short rows instead of 2 x 43
    character.professions = Vec::new();
    character.inventory = vec![None; 4]; // far short of INVENTORY_SIZE

    let snapshot = CharacterSnapshot {
        player_state_json: None,
        character,
        items: Vec::new(),
        ppd_blob: Vec::new(),
        subscriber_blob: Vec::new(),
        current_area: 1,
        current_mirror: 1,
        allowed_area: 1,
        mirror: 1,
    };
    let mut world = World::default();
    let mut player = PlayerRuntime::connected(1, 0);

    let result = apply_character_snapshot(&mut world, &mut player, snapshot, 11, 10, 100);
    assert!(result.loaded);

    let character = world.characters.get(&target_id).unwrap();
    assert_eq!(character.values.len(), 2);
    assert_eq!(
        character.values[0].len(),
        ugaris_core::entity::CHARACTER_VALUE_COUNT
    );
    assert_eq!(character.values[1][0], 7); // surviving bare prefix is preserved
    assert_eq!(
        character.professions.len(),
        ugaris_core::entity::PROFESSION_COUNT
    );
    assert_eq!(
        character.inventory.len(),
        ugaris_core::entity::INVENTORY_SIZE
    );

    // The formerly panicking per-connection/per-packet paths now run clean.
    let character = world.characters.get(&target_id).unwrap().clone();
    let _ = login_payload(&world, &character, 1, 1);
    let _ = inventory_swap_slot(&mut world, target_id, 35);
}

#[test]
fn logout_save_omits_arkhata_stopwatch_from_cursor_snapshot() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 37, 10, 10);
    character.cursor_item = Some(ItemId(10));
    character.current_container = Some(ItemId(10));
    world.add_character(character.clone());

    let mut stopwatch = test_item_with_driver(ItemId(10), IDR_ARKHATA);
    stopwatch.carried_by = Some(character_id);
    stopwatch.driver_data = vec![1];
    world.add_item(stopwatch);

    let request = character_save_request(
        &world,
        &PlayerRuntime::connected(1, 0),
        &character,
        None,
        37,
        0,
    );

    assert_eq!(request.character.cursor_item, None);
    assert_eq!(request.character.current_container, None);
    assert!(request.items.is_empty());
}

#[test]
fn area_transfer_save_request_writes_destination_coords_and_allowed_area() {
    // C `change_area` (`database_character.c:343-364`): `kick_char`'s
    // `save_char(cn, save_area)` writes the *destination* area into
    // `allowed_area`/`mirror` while the guard columns still check
    // against this server's own (sending) `current_area`/
    // `current_mirror` - never the destination's.
    let mut world = World::default();
    let character_id = CharacterId(9);
    let mut character = login_character(character_id, &login_block("Traveler"), 3, 50, 60);
    character.x = 50;
    character.y = 60;
    world.add_character(character.clone());

    let request = character_area_transfer_save_request(
        &world,
        &PlayerRuntime::connected(1, 0),
        &character,
        None,
        3,   // sending area_id
        1,   // sending mirror_id
        6,   // target area
        4,   // target mirror
        139, // target x
        75,  // target y
    );

    assert_eq!(request.character.x, 139);
    assert_eq!(request.character.y, 75);
    match request.mode {
        CharacterSaveMode::Logout {
            expected_current_area,
            expected_current_mirror,
            allowed_area,
            mirror,
        } => {
            assert_eq!(expected_current_area, 3);
            assert_eq!(expected_current_mirror, 1);
            assert_eq!(allowed_area, 6);
            assert_eq!(mirror, 4);
        }
        other => panic!("expected Logout mode, got {other:?}"),
    }
}

#[test]
fn area_transfer_save_request_also_strips_logout_vanishing_items() {
    // Reuses `character_logout_snapshot` exactly like a normal
    // same-area logout save - an arkhata stopwatch on the cursor still
    // vanishes on a cross-area transfer, since C's `kick_char` runs the
    // exact same `save_char` codepath regardless of *why* the character
    // is leaving.
    let mut world = World::default();
    let character_id = CharacterId(11);
    let mut character = login_character(character_id, &login_block("Traveler2"), 37, 10, 10);
    character.cursor_item = Some(ItemId(20));
    character.current_container = Some(ItemId(20));
    world.add_character(character.clone());

    let mut stopwatch = test_item_with_driver(ItemId(20), IDR_ARKHATA);
    stopwatch.carried_by = Some(character_id);
    stopwatch.driver_data = vec![1];
    world.add_item(stopwatch);

    let request = character_area_transfer_save_request(
        &world,
        &PlayerRuntime::connected(1, 0),
        &character,
        None,
        37,
        0,
        1,
        1,
        126,
        179,
    );

    assert_eq!(request.character.cursor_item, None);
    assert_eq!(request.character.current_container, None);
    assert!(request.items.is_empty());
    assert_eq!(request.character.x, 126);
    assert_eq!(request.character.y, 179);
}

#[test]
fn persisted_player_state_json_round_trips_typed_state() {
    let mut player = PlayerRuntime::connected(9, 100);
    player.character_id = Some(CharacterId(42));
    player.character_number = 4242;
    player.set_area1_gwendy_state(7);
    let _ = player.add_keyring_entry(ugaris_core::player::KeyringEntry {
        template_id: 0x0100_0002,
        name: "Copper Key".into(),
        description: "Opens something coppery.".into(),
        sprite: 12,
        flags: 0,
        value: 5,
        driver: 0,
        driver_data: vec![0; 16],
        expire_serial: 0,
    });
    let mut depot = AccountDepotState::default();
    let mut stored = test_item(ugaris_core::ids::ItemId(900), 12, ItemFlags::TAKE);
    stored.name = "Stored Sword".into();
    depot.slots[3] = Some(stored);

    let value = persisted_player_state_json(&player, Some(&depot)).expect("serializes");
    // Section names double as the public integration read schema.
    assert!(value["player"]["keyring"].is_array());
    assert!(value["account_depot"]["slots"].is_array());

    // A brand-new session restores the persisted state but keeps its own
    // connection identity.
    let mut live = PlayerRuntime::connected(77, 555);
    live.character_id = Some(CharacterId(42));
    live.character_number = 4242;
    live.mark_login_parsed(Some(3), 556);
    let persisted: PersistedPlayerState = serde_json::from_value(value).expect("deserializes");
    let restored_depot = restore_player_from_persisted(&mut live, persisted);

    assert_eq!(live.session_id, 77, "live session id preserved");
    assert_eq!(live.view_distance, 40, "negotiated view distance preserved");
    assert_eq!(live.area1_gwendy_state(), 7, "typed quest state restored");
    assert_eq!(live.keyring.len(), 1, "keyring restored");
    assert_eq!(
        restored_depot.expect("depot restored").slots[3]
            .as_ref()
            .map(|item| item.name.as_str()),
        Some("Stored Sword")
    );
}
