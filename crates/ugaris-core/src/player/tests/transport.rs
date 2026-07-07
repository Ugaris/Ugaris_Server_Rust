use super::*;

#[test]
fn transport_ppd_codec_matches_legacy_seen_mask_layout() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.transport_seen = 0x0102_0304_0506_0708;

    let encoded = player.encode_legacy_transport_ppd();
    assert_eq!(encoded, 0x0102_0304_0506_0708_u64.to_le_bytes());

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_transport_ppd(&encoded));
    assert_eq!(decoded.transport_seen, 0x0102_0304_0506_0708);
    assert!(!decoded.decode_legacy_transport_ppd(&encoded[..7]));
}

#[test]
fn transport_ppd_blob_round_trips_with_legacy_block_framing() {
    let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
    let mut existing_transport = vec![0; LEGACY_TRANSPORT_PPD_SIZE];
    write_u64(&mut existing_transport, 0, 0x0000_0000_0000_0004);

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
    write_ppd_block(&mut existing, DRD_TRANSPORT_PPD, &existing_transport);

    let mut player = PlayerRuntime::connected(1, 0);
    player.transport_seen = 0x0000_0000_0000_0021;

    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), unknown_id);
    assert_eq!(read_u32(&encoded, 12), DRD_TRANSPORT_PPD);
    assert_eq!(read_u32(&encoded, 16), LEGACY_TRANSPORT_PPD_SIZE as u32);
    assert_eq!(read_u64(&encoded, 20), 0x0000_0000_0000_0021);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.transport_seen, 0x0000_0000_0000_0021);
}

#[test]
fn ppd_blob_appends_transport_without_existing_block() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.touch_transport(5);

    let encoded = player.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&encoded, 0), DRD_TRANSPORT_PPD);
    assert_eq!(read_u32(&encoded, 4), LEGACY_TRANSPORT_PPD_SIZE as u32);
    assert_eq!(read_u64(&encoded, 8), 1_u64 << 5);
}

#[test]
fn transport_discovery_marks_legacy_exploration_achievement_thresholds() {
    let mut player = PlayerRuntime::connected(1, 0);
    for point in [0, 2, 9, 21, 22, 23, 24] {
        assert!(player.touch_transport(point));
    }
    assert!(!player.achievements.traveller_of_astonia);

    assert!(player.touch_transport(25));
    assert!(player.achievements.traveller_of_astonia);

    let mut underground = PlayerRuntime::connected(2, 0);
    for point in 3..=7 {
        assert!(underground.touch_transport(point));
    }
    assert!(!underground.achievements.underground_explorer);
    assert!(underground.touch_transport(8));
    assert!(underground.achievements.underground_explorer);

    let mut explorer = PlayerRuntime::connected(3, 0);
    for point in 0..=25 {
        if ![11, 18, 19].contains(&point) {
            assert!(explorer.touch_transport(point));
        }
    }
    assert!(explorer.achievements.explorer_of_astonia);
    assert_eq!(explorer.transport_seen & !TRANSPORT_ALL_TELEPORTS_MASK, 0);
}
