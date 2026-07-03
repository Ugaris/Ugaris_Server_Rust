use super::*;

#[test]
fn character_snapshot_restores_active_legacy_shutup_ppd() {
    let target_id = CharacterId(8);
    let character = login_character(target_id, &login_block("Target"), 1, 11, 10);

    let mut persisted = PlayerRuntime::connected(99, 0);
    persisted.shutup_until_seconds = 700;
    let ppd_blob = persisted.encode_legacy_ppd_blob(&[]);

    let snapshot = CharacterSnapshot {
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
