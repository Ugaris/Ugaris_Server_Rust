use super::*;

#[test]
fn twocity_ppd_codec_matches_legacy_c_layout() {
    assert_eq!(DRD_TWOCITY_PPD, 0x8100_0061);
    assert_eq!(LEGACY_TWOCITY_PPD_SIZE, 116);
    assert_eq!(TWOCITY_PPD_GOODTILE_OFFSET, 76);
    assert_eq!(TWOCITY_PPD_SOLVED_LIBRARY_OFFSET, 96);

    let mut player = PlayerRuntime::connected(1, 0);
    player.twocity_ppd = vec![0; LEGACY_TWOCITY_PPD_SIZE];
    write_i32(&mut player.twocity_ppd, 0, 1234);
    player.twocity_goodtile = [1, 2, 3, 4, 5];
    player.twocity_solved_library = true;

    let encoded = player.encode_legacy_twocity_ppd();
    assert_eq!(read_i32(&encoded, 0), 1234);
    for (index, color) in [1, 2, 3, 4, 5].into_iter().enumerate() {
        assert_eq!(
            read_i32(&encoded, TWOCITY_PPD_GOODTILE_OFFSET + index * 4),
            color
        );
    }
    assert_eq!(read_i32(&encoded, TWOCITY_PPD_SOLVED_LIBRARY_OFFSET), 1);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_twocity_ppd(&encoded));
    assert_eq!(decoded.twocity_goodtile, [1, 2, 3, 4, 5]);
    assert!(decoded.twocity_solved_library);
    assert_eq!(read_i32(&decoded.twocity_ppd, 0), 1234);
    assert!(!decoded.decode_legacy_twocity_ppd(&encoded[..LEGACY_TWOCITY_PPD_SIZE - 1]));
}

#[test]
fn twocity_ppd_exposes_sanwyn_skelly_alchemist_quest_states() {
    assert_eq!(TWOCITY_PPD_SANWYN_STATE_OFFSET, 64);
    assert_eq!(TWOCITY_PPD_SKELLY_STATE_OFFSET, 108);
    assert_eq!(TWOCITY_PPD_ALCHEMIST_STATE_OFFSET, 112);

    let mut player = PlayerRuntime::connected(1, 0);
    player.set_twocity_thief_state(20);
    player.set_twocity_sanwyn_state(8);
    player.set_twocity_skelly_state(3);
    player.set_twocity_alchemist_state(5);

    let encoded = player.encode_legacy_twocity_ppd();
    assert_eq!(read_i32(&encoded, TWOCITY_PPD_SANWYN_STATE_OFFSET), 8);
    assert_eq!(read_i32(&encoded, TWOCITY_PPD_SKELLY_STATE_OFFSET), 3);
    assert_eq!(read_i32(&encoded, TWOCITY_PPD_ALCHEMIST_STATE_OFFSET), 5);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_twocity_ppd(&encoded));
    assert_eq!(decoded.twocity_sanwyn_state(), 8);
    assert_eq!(decoded.twocity_skelly_state(), 3);
    assert_eq!(decoded.twocity_alchemist_state(), 5);

    let state = decoded.twocity_quest_state();
    assert_eq!(state.thief_state, 20);
    assert_eq!(state.sanwyn_state, 8);
    assert_eq!(state.skelly_state, 3);
    assert_eq!(state.alchemist_state, 5);
}

#[test]
fn twocity_burndown_kill_updates_legacy_thief_fields() {
    assert_eq!(TWOCITY_PPD_THIEF_STATE_OFFSET, 32);
    assert_eq!(TWOCITY_PPD_THIEF_KILLED_OFFSET, 40);

    let mut player = PlayerRuntime::connected(1, 0);
    assert!(!player.mark_twocity_burndown_kill());
    assert_eq!(player.twocity_thief_state(), 0);
    assert_eq!(player.twocity_thief_killed(0), 0);

    player.set_twocity_thief_state(13);
    assert!(player.mark_twocity_burndown_kill());
    assert_eq!(player.twocity_thief_state(), 14);
    assert_eq!(player.twocity_thief_killed(0), 1);

    assert!(player.mark_twocity_burndown_kill());
    assert_eq!(player.twocity_thief_state(), 14);
    assert_eq!(player.twocity_thief_killed(0), 2);
    assert_eq!(player.twocity_thief_killed(6), 0);

    let encoded = player.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&encoded, 0), DRD_TWOCITY_PPD);
    assert_eq!(read_i32(&encoded, 8 + TWOCITY_PPD_THIEF_STATE_OFFSET), 14);
    assert_eq!(read_i32(&encoded, 8 + TWOCITY_PPD_THIEF_KILLED_OFFSET), 2);
}

#[test]
fn twocity_ppd_blob_replaces_and_appends_legacy_block() {
    let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
    let mut existing_twocity = vec![0; LEGACY_TWOCITY_PPD_SIZE];
    write_i32(&mut existing_twocity, 0, 777);
    write_i32(&mut existing_twocity, TWOCITY_PPD_GOODTILE_OFFSET, 6);

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
    write_ppd_block(&mut existing, DRD_TWOCITY_PPD, &existing_twocity);

    let mut player = PlayerRuntime::connected(1, 0);
    assert!(player.decode_legacy_ppd_blob(&existing));
    assert_eq!(player.twocity_goodtile[0], 6);
    player.twocity_goodtile = [2, 3, 4, 5, 6];
    player.twocity_solved_library = true;

    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), unknown_id);
    assert_eq!(read_u32(&encoded, 12), DRD_TWOCITY_PPD);
    assert_eq!(read_u32(&encoded, 16), LEGACY_TWOCITY_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 20), 777);
    assert_eq!(read_i32(&encoded, 20 + TWOCITY_PPD_GOODTILE_OFFSET), 2);
    assert_eq!(
        read_i32(&encoded, 20 + TWOCITY_PPD_SOLVED_LIBRARY_OFFSET),
        1
    );

    let mut appended_player = PlayerRuntime::connected(2, 0);
    appended_player.twocity_goodtile = [1, 1, 2, 2, 3];
    let appended = appended_player.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&appended, 0), DRD_TWOCITY_PPD);
    assert_eq!(read_u32(&appended, 4), LEGACY_TWOCITY_PPD_SIZE as u32);
    assert_eq!(read_i32(&appended, 8 + TWOCITY_PPD_GOODTILE_OFFSET), 1);
}
