use super::*;

#[test]
fn keyring_tracks_legacy_key_ids_with_duplicate_and_capacity_rules() {
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(
        player.add_keyring_key(0x1122_3344, "Copper Key"),
        KeyringAddResult::Added
    );
    assert_eq!(player.keyring_key_name(0x1122_3344), Some("Copper Key"));
    assert_eq!(
        player.add_keyring_key(0x1122_3344, "Duplicate"),
        KeyringAddResult::Duplicate
    );

    for index in 1..KEYRING_MAX_KEYS {
        assert_eq!(
            player.add_keyring_key(index as u32, format!("Key {index}")),
            KeyringAddResult::Added
        );
    }
    assert_eq!(
        player.add_keyring_key(0x5566_7788, "Overflow"),
        KeyringAddResult::Full
    );
}

#[test]
fn keyring_item_storage_keeps_legacy_recreation_metadata() {
    let mut player = PlayerRuntime::connected(1, 0);
    let item = Item {
        id: ItemId(7),
        name: "Copper Key".into(),
        description: "Opens a copper lock".into(),
        flags: ItemFlags::USED | ItemFlags::TAKE | ItemFlags::QUEST,
        sprite: 1234,
        value: 55,
        min_level: 0,
        max_level: 0,
        needs_class: 0,
        template_id: 0x1122_3344,
        owner_id: 0,
        modifier_index: [0; MAX_MODIFIERS],
        modifier_value: [0; MAX_MODIFIERS],
        x: 0,
        y: 0,
        carried_by: None,
        contained_in: None,
        content_id: 0,
        driver: 77,
        driver_data: (0..32).collect(),
        serial: 9,
    };

    assert_eq!(player.add_keyring_item(&item), KeyringAddResult::Added);

    let stored = &player.keyring[0];
    assert_eq!(stored.template_id, 0x1122_3344);
    assert_eq!(stored.name, "Copper Key");
    assert_eq!(stored.description, "Opens a copper lock");
    assert_eq!(stored.sprite, 1234);
    assert_eq!(stored.flags, item.flags.bits());
    assert_eq!(stored.value, 55);
    assert_eq!(stored.driver, 77);
    assert_eq!(stored.driver_data, (0..16).collect::<Vec<_>>());
    assert_eq!(stored.expire_serial, 9);
}

#[test]
fn keyring_auto_add_setting_round_trips() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert!(!player.keyring_auto_add());
    player.set_keyring_auto_add(true);
    assert!(player.keyring_auto_add());
}

#[test]
fn keyring_ppd_codec_matches_legacy_c_layout() {
    assert_eq!(KEYRING_PPD_FLAGS_OFFSET % 8, 0);
    assert_eq!(KEYRING_PPD_AUTO_ADD_OFFSET + 4, LEGACY_KEYRING_PPD_SIZE);

    let mut player = PlayerRuntime::connected(1, 0);
    player.set_keyring_auto_add(true);
    assert_eq!(
        player.add_keyring_entry(KeyringEntry {
            template_id: 0x1122_3344,
            name: "A name that is deliberately longer than forty bytes".to_string(),
            description: "Opens a door and has a long legacy description".to_string(),
            sprite: -123,
            flags: 0x0102_0304_0506_0708,
            value: 99,
            driver: 77,
            driver_data: (0..32).collect(),
            expire_serial: 0x1234,
        }),
        KeyringAddResult::Added
    );

    let bytes = player.encode_legacy_keyring_ppd();
    assert_eq!(bytes.len(), LEGACY_KEYRING_PPD_SIZE);
    assert_eq!(read_i32(&bytes, KEYRING_PPD_COUNT_OFFSET), 1);
    assert_eq!(read_u32(&bytes, KEYRING_PPD_KEYS_OFFSET), 0x1122_3344);
    assert_eq!(
        bytes[KEYRING_PPD_NAMES_OFFSET + KEYRING_KEY_NAME_LEN - 1],
        0
    );
    assert_eq!(read_i32(&bytes, KEYRING_PPD_AUTO_ADD_OFFSET), 1);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_keyring_ppd(&bytes));
    assert!(decoded.keyring_auto_add());
    assert_eq!(decoded.keyring.len(), 1);
    assert_eq!(decoded.keyring[0].template_id, 0x1122_3344);
    assert_eq!(
        decoded.keyring[0].name,
        "A name that is deliberately longer than"
    );
    assert_eq!(decoded.keyring[0].sprite, -123);
    assert_eq!(decoded.keyring[0].flags, 0x0102_0304_0506_0708);
    assert_eq!(decoded.keyring[0].driver_data, (0..16).collect::<Vec<_>>());
    assert_eq!(decoded.keyring[0].expire_serial, 0x34);
}

#[test]
fn keyring_ppd_blob_round_trips_with_legacy_block_framing() {
    let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
    let mut existing = Vec::new();
    write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
    write_ppd_block(&mut existing, DRD_JUNK_PPD, &[9, 9, 9]);

    let mut player = PlayerRuntime::connected(1, 0);
    player.set_keyring_auto_add(true);
    assert_eq!(
        player.add_keyring_key(0x1122_3344, "Copper Key"),
        KeyringAddResult::Added
    );

    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), unknown_id);
    assert_eq!(read_u32(&encoded, 4), 4);
    assert_eq!(&encoded[8..12], &[1, 2, 3, 4]);
    assert_eq!(read_u32(&encoded, 12), DRD_KEYRING_PPD);
    assert_eq!(read_u32(&encoded, 16), LEGACY_KEYRING_PPD_SIZE as u32);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert!(decoded.keyring_auto_add());
    assert_eq!(decoded.keyring_key_name(0x1122_3344), Some("Copper Key"));
    assert!(!encoded
        .windows(4)
        .any(|window| window == DRD_JUNK_PPD.to_le_bytes()));
}

#[test]
fn keyring_display_lines_match_legacy_shape_and_remove_by_position() {
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(
        player.keyring_display_lines(),
        vec!["Your keyring is empty."]
    );
    assert_eq!(
        player.add_keyring_key(0x1122_3344, "Copper Key"),
        KeyringAddResult::Added
    );
    assert_eq!(
        player.add_keyring_key(0x5566_7788, "Silver Key"),
        KeyringAddResult::Added
    );

    assert_eq!(
        player.keyring_display_lines(),
        vec![
            "=== Keyring (2/100 keys) ===",
            " 1. Copper Key",
            " 2. Silver Key",
            "Use a key on the keyring to add it.",
            "Type '#keyring remove <number>' to remove a key.",
            "Type '#keyring addall' to add all keys from inventory.",
        ]
    );
    assert_eq!(
        player.remove_keyring_key_at(0).map(|key| key.name),
        Some("Copper Key".to_string())
    );
    assert_eq!(player.keyring_key_name(0x1122_3344), None);
    assert_eq!(player.keyring_key_name(0x5566_7788), Some("Silver Key"));
    assert_eq!(player.remove_keyring_key_at(99), None);
}
