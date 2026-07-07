use super::*;

#[test]
fn treasure_chest_ppd_codec_matches_legacy_c_layout() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.mark_chest_access(0, 1234);
    player.mark_chest_access(63, 86_400);
    player.mark_chest_access(199, i32::MAX as u64 + 99);

    let bytes = player.encode_legacy_treasure_chest_ppd();
    assert_eq!(bytes.len(), LEGACY_TREASURE_CHEST_PPD_SIZE);
    assert_eq!(read_i32(&bytes, 0), 1234);
    assert_eq!(read_i32(&bytes, 63 * 4), 86_400);
    assert_eq!(read_i32(&bytes, 199 * 4), i32::MAX);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_treasure_chest_ppd(&bytes));
    assert_eq!(decoded.chest_last_access_seconds(0), 1234);
    assert_eq!(decoded.chest_last_access_seconds(63), 86_400);
    assert_eq!(decoded.chest_last_access_seconds(199), i32::MAX as u64);
    assert_eq!(decoded.chest_last_access_seconds(1), 0);
}

#[test]
fn randchest_ppd_codec_matches_legacy_c_layout() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.mark_random_chest_used(0x0001_0506, 1234);
    player.mark_random_chest_used(0x0001_0708, i32::MAX as u64 + 99);

    let bytes = player.encode_legacy_randchest_ppd();
    assert_eq!(bytes.len(), LEGACY_RANDCHEST_PPD_SIZE);
    assert_eq!(read_i32(&bytes, 0), 0x0001_0506);
    assert_eq!(read_i32(&bytes, 4), 0x0001_0708);
    assert_eq!(read_i32(&bytes, RANDCHEST_PPD_LAST_USED_OFFSET), 1234);
    assert_eq!(
        read_i32(&bytes, RANDCHEST_PPD_LAST_USED_OFFSET + 4),
        i32::MAX
    );

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_randchest_ppd(&bytes));
    assert_eq!(
        decoded.random_chest_last_used_seconds(0x0001_0506),
        Some(1234)
    );
    assert_eq!(
        decoded.random_chest_last_used_seconds(0x0001_0708),
        Some(i32::MAX as u64)
    );
    assert_eq!(decoded.random_chest_last_used_seconds(0x0001_090a), None);
}

#[test]
fn treasure_chest_ppd_blob_round_trips_with_legacy_block_framing() {
    let unknown_id = make_drd(DEV_ID_DB, 222 | PERSISTENT_PLAYER_DATA);
    let mut existing = Vec::new();
    write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
    write_ppd_block(
        &mut existing,
        DRD_TREASURE_CHEST_PPD,
        &[0; LEGACY_TREASURE_CHEST_PPD_SIZE],
    );

    let mut player = PlayerRuntime::connected(1, 0);
    player.mark_chest_access(17, 777);

    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), unknown_id);
    assert_eq!(read_u32(&encoded, 12), DRD_TREASURE_CHEST_PPD);
    assert_eq!(
        read_u32(&encoded, 16),
        LEGACY_TREASURE_CHEST_PPD_SIZE as u32
    );
    assert_eq!(read_i32(&encoded, 20 + 17 * 4), 777);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.chest_last_access_seconds(17), 777);
}

#[test]
fn ppd_blob_appends_treasure_chests_without_existing_block() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.mark_chest_access(5, 55);

    let encoded = player.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&encoded, 0), DRD_TREASURE_CHEST_PPD);
    assert_eq!(read_u32(&encoded, 4), LEGACY_TREASURE_CHEST_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 8 + 5 * 4), 55);
}

#[test]
fn randchest_ppd_blob_round_trips_with_legacy_block_framing() {
    let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
    let mut existing_randchest = vec![0; LEGACY_RANDCHEST_PPD_SIZE];
    write_i32(
        &mut existing_randchest,
        RANDCHEST_PPD_IDS_OFFSET,
        0x0001_0203,
    );
    write_i32(&mut existing_randchest, RANDCHEST_PPD_LAST_USED_OFFSET, 44);

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
    write_ppd_block(&mut existing, DRD_RANDCHEST_PPD, &existing_randchest);

    let mut player = PlayerRuntime::connected(1, 0);
    player.mark_random_chest_used(0x0001_0506, 777);

    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), unknown_id);
    assert_eq!(read_u32(&encoded, 12), DRD_RANDCHEST_PPD);
    assert_eq!(read_u32(&encoded, 16), LEGACY_RANDCHEST_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 20), 0x0001_0506);
    assert_eq!(read_i32(&encoded, 20 + RANDCHEST_PPD_LAST_USED_OFFSET), 777);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(
        decoded.random_chest_last_used_seconds(0x0001_0506),
        Some(777)
    );
    assert_eq!(decoded.random_chest_last_used_seconds(0x0001_0203), None);
}

#[test]
fn ratchest_ppd_codec_matches_legacy_c_layout() {
    assert_eq!(LEGACY_RATCHEST_PPD_SIZE, 812);

    let mut player = PlayerRuntime::connected(1, 0);
    player.mark_rat_chest_used(0x0007_0506, 1234);
    player.mark_rat_chest_used(0x0007_0708, i32::MAX as u64 + 99);
    player.rat_chest_treasure_x = 321;
    player.rat_chest_treasure_y = 654;
    player.rat_chest_last_treasure_seconds = 9876;

    let bytes = player.encode_legacy_ratchest_ppd();
    assert_eq!(bytes.len(), LEGACY_RATCHEST_PPD_SIZE);
    assert_eq!(read_i32(&bytes, 0), 0x0007_0506);
    assert_eq!(read_i32(&bytes, 4), 0x0007_0708);
    assert_eq!(read_i32(&bytes, RATCHEST_PPD_LAST_USED_OFFSET), 1234);
    assert_eq!(
        read_i32(&bytes, RATCHEST_PPD_LAST_USED_OFFSET + 4),
        i32::MAX
    );
    assert_eq!(read_i32(&bytes, RATCHEST_PPD_TREASURE_X_OFFSET), 321);
    assert_eq!(read_i32(&bytes, RATCHEST_PPD_TREASURE_Y_OFFSET), 654);
    assert_eq!(read_i32(&bytes, RATCHEST_PPD_LAST_TREASURE_OFFSET), 9876);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ratchest_ppd(&bytes));
    assert_eq!(decoded.rat_chest_last_used_seconds(0x0007_0506), Some(1234));
    assert_eq!(
        decoded.rat_chest_last_used_seconds(0x0007_0708),
        Some(i32::MAX as u64)
    );
    assert_eq!(decoded.rat_chest_last_used_seconds(0x0007_090a), None);
    assert_eq!(decoded.rat_chest_treasure_x, 321);
    assert_eq!(decoded.rat_chest_treasure_y, 654);
    assert_eq!(decoded.rat_chest_last_treasure_seconds, 9876);
}

#[test]
fn ratchest_ppd_blob_round_trips_with_legacy_block_framing() {
    let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
    let mut existing_ratchest = vec![0; LEGACY_RATCHEST_PPD_SIZE];
    write_i32(&mut existing_ratchest, RATCHEST_PPD_IDS_OFFSET, 0x0007_0203);
    write_i32(&mut existing_ratchest, RATCHEST_PPD_LAST_USED_OFFSET, 44);

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
    write_ppd_block(&mut existing, DRD_RATCHEST_PPD, &existing_ratchest);

    let mut player = PlayerRuntime::connected(1, 0);
    player.mark_rat_chest_used(0x0007_0506, 777);
    player.rat_chest_treasure_x = 12;
    player.rat_chest_treasure_y = 34;
    player.rat_chest_last_treasure_seconds = 55;

    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), unknown_id);
    assert_eq!(read_u32(&encoded, 12), DRD_RATCHEST_PPD);
    assert_eq!(read_u32(&encoded, 16), LEGACY_RATCHEST_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 20), 0x0007_0506);
    assert_eq!(read_i32(&encoded, 20 + RATCHEST_PPD_LAST_USED_OFFSET), 777);
    assert_eq!(read_i32(&encoded, 20 + RATCHEST_PPD_TREASURE_X_OFFSET), 12);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.rat_chest_last_used_seconds(0x0007_0506), Some(777));
    assert_eq!(decoded.rat_chest_last_used_seconds(0x0007_0203), None);
    assert_eq!(decoded.rat_chest_treasure_y, 34);
    assert_eq!(decoded.rat_chest_last_treasure_seconds, 55);

    let mut appended = PlayerRuntime::connected(3, 0);
    appended.mark_rat_chest_used(0x0007_0203, 66);
    let appended_blob = appended.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&appended_blob, 0), DRD_RATCHEST_PPD);
    assert_eq!(read_i32(&appended_blob, 8), 0x0007_0203);
}

#[test]
fn ppd_blob_appends_randchests_without_existing_block() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.mark_random_chest_used(0x0001_0203, 55);

    let encoded = player.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&encoded, 0), DRD_RANDCHEST_PPD);
    assert_eq!(read_u32(&encoded, 4), LEGACY_RANDCHEST_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 8), 0x0001_0203);
    assert_eq!(read_i32(&encoded, 8 + RANDCHEST_PPD_LAST_USED_OFFSET), 55);
}

#[test]
fn chest_achievement_state_tracks_legacy_threshold_hooks() {
    let mut player = PlayerRuntime::connected(1, 0);

    for _ in 0..9 {
        player.record_chest_opened(1);
    }
    assert_eq!(player.achievements.chests_opened, 9);
    assert!(!player.achievements.looter);

    player.record_chest_opened(1);
    assert!(player.achievements.looter);
    assert!(!player.achievements.treasure_hunter);

    for _ in 10..50 {
        player.record_chest_opened(1);
    }
    assert!(player.achievements.treasure_hunter);
    assert!(!player.achievements.treasure_master);

    for _ in 50..100 {
        player.record_chest_opened(1);
    }
    assert!(player.achievements.treasure_master);
    assert!(!player.achievements.legendary_looter);

    for _ in 100..500 {
        player.record_chest_opened(1);
    }
    assert!(player.achievements.legendary_looter);

    player.record_chest_opened(63);
    assert!(player.achievements.gold_looter);
}

#[test]
fn random_chest_access_tracks_hundred_recent_locations() {
    let mut player = PlayerRuntime::connected(1, 0);

    player.mark_random_chest_used(7, 100);
    assert_eq!(player.random_chest_last_used_seconds(7), Some(100));
    player.mark_random_chest_used(7, 200);
    assert_eq!(player.random_chest_last_used_seconds(7), Some(200));

    for index in 1..RANDCHEST_MAX_ENTRIES {
        player.mark_random_chest_used(1_000 + index as u32, index as u64);
    }
    assert_eq!(player.random_chests.len(), RANDCHEST_MAX_ENTRIES);
    player.mark_random_chest_used(9_999, 300);
    assert_eq!(player.random_chests.len(), RANDCHEST_MAX_ENTRIES);
    assert_eq!(player.random_chest_last_used_seconds(9_999), Some(300));
}
