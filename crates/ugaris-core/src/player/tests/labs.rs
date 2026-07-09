use super::*;

#[test]
fn lab_ppd_codec_preserves_legacy_solved_bits_and_payload() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.lab_ppd = vec![0xaa; LEGACY_LAB_PPD_SIZE];
    player.lab_solved_bits = (1_u64 << 10) | (1_u64 << 25);

    let encoded = player.encode_legacy_lab_ppd();
    assert_eq!(encoded.len(), LEGACY_LAB_PPD_SIZE);
    assert_eq!(read_u64(&encoded, 0), (1_u64 << 10) | (1_u64 << 25));
    assert_eq!(encoded[8], 0xaa);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_lab_ppd(&encoded));
    assert_eq!(decoded.lab_solved_bits, (1_u64 << 10) | (1_u64 << 25));
    assert_eq!(decoded.lab_ppd, encoded);
    assert!(!decoded.decode_legacy_lab_ppd(&encoded[..7]));
}

#[test]
fn lab2_described_graves_use_legacy_lab_ppd_offsets() {
    let mut player = PlayerRuntime::connected(1, 0);
    let indices = player.ensure_legacy_lab2_described_graves_with_indices([2, 6, 10, 11]);

    assert_eq!(indices, [2, 6, 10, 11]);
    assert_eq!(player.lab_ppd.len(), LEGACY_LAB_PPD_SIZE);
    assert_eq!(player.lab_ppd[LEGACY_LAB2_GRAVEVERSION_OFFSET], 2);
    assert_eq!(player.legacy_lab2_grave_indices(), [2, 6, 10, 11]);

    let preserved = player.ensure_legacy_lab2_described_graves_with_indices([0, 4, 8, 9]);
    assert_eq!(preserved, [2, 6, 10, 11]);
}

#[test]
fn lab2_grave_clue_text_uses_legacy_described_grave_table() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.ensure_legacy_lab2_described_graves_with_indices([0, 4, 8, 9]);

    assert_eq!(
        player.legacy_lab2_grave_clue_text(1).as_deref(),
        Some("Henry is buried in the third grave behind the chapel.")
    );
    assert_eq!(
        player.legacy_lab2_grave_clue_text(3).as_deref(),
        Some("For his generosity John is buried in the first grave of the second row next to the southeastern chapel aisle.")
    );
    assert_eq!(player.legacy_lab2_grave_clue_text(5), None);
}

#[test]
fn lab2_special_grave_kind_matches_player_specific_coordinates() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.ensure_legacy_lab2_described_graves_with_indices([0, 4, 8, 9]);

    assert_eq!(player.legacy_lab2_special_grave_kind_at(194, 183), Some(1));
    assert_eq!(player.legacy_lab2_special_grave_kind_at(199, 195), Some(3));
    assert_eq!(player.legacy_lab2_special_grave_kind_at(212, 191), None);
}

#[test]
fn lab2_grave_bitset_uses_legacy_one_bit_per_grave_layout() {
    let mut player = PlayerRuntime::connected(1, 0);

    assert!(!player.legacy_lab2_grave_cleared(9));
    assert!(player.mark_legacy_lab2_grave_cleared(9));
    assert!(!player.mark_legacy_lab2_grave_cleared(9));
    assert!(player.legacy_lab2_grave_cleared(9));
    assert_eq!(player.lab2_grave_bits[1], 0b0000_0010);
    assert!(!player.mark_legacy_lab2_grave_cleared(LAB2_GRAVE_BITSET_BYTES * 8));
}

#[test]
fn lab2_herald_talkstep_uses_legacy_lab_ppd_offset() {
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(player.legacy_lab2_herald_talkstep(), 0);
    player.set_legacy_lab2_herald_talkstep(62);
    assert_eq!(player.lab_ppd[LEGACY_LAB2_HERALD_TALKSTEP_OFFSET], 62);
    assert_eq!(player.legacy_lab2_herald_talkstep(), 62);

    // Also survives a full-size `lab_ppd` (e.g. after
    // `ensure_legacy_lab2_described_graves`), distinct from the
    // neighboring `graveversion` byte.
    player.ensure_legacy_lab2_described_graves();
    assert_eq!(player.lab_ppd.len(), LEGACY_LAB_PPD_SIZE);
    assert_eq!(player.legacy_lab2_herald_talkstep(), 62);
    assert_ne!(
        player.lab_ppd[LEGACY_LAB2_GRAVEVERSION_OFFSET],
        player.lab_ppd[LEGACY_LAB2_HERALD_TALKSTEP_OFFSET]
    );
}

#[test]
fn lab_ppd_blob_round_trips_with_legacy_block_framing() {
    let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
    let mut existing_lab = vec![0; LEGACY_LAB_PPD_SIZE];
    write_u64(&mut existing_lab, 0, 1_u64 << 10);

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
    write_ppd_block(&mut existing, DRD_LAB_PPD, &existing_lab);

    let mut player = PlayerRuntime::connected(1, 0);
    player.lab_solved_bits = (1_u64 << 15) | (1_u64 << 20);

    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), unknown_id);
    assert_eq!(read_u32(&encoded, 12), DRD_LAB_PPD);
    assert_eq!(read_u32(&encoded, 16), LEGACY_LAB_PPD_SIZE as u32);
    assert_eq!(read_u64(&encoded, 20), (1_u64 << 15) | (1_u64 << 20));

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.lab_solved_bits, (1_u64 << 15) | (1_u64 << 20));
}

#[test]
fn lab4_seyan_state_from_got_matches_c_set_seyan_state() {
    // C `set_seyan_state` (`src/area/22/lab4.c:94-104`).
    assert_eq!(lab4_seyan_state_from_got(0), 0);
    assert_eq!(lab4_seyan_state_from_got(1 << 0), 10); // crown only
    assert_eq!(lab4_seyan_state_from_got(1 << 1), 20); // szepter only
    assert_eq!(lab4_seyan_state_from_got((1 << 0) | (1 << 1)), 30); // both
}

#[test]
fn recompute_lab4_seyan_state_writes_derived_state_from_got() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.lab4_seyan_got = 1 << 1;
    player.recompute_lab4_seyan_state();
    assert_eq!(player.lab4_seyan_state, 20);

    player.lab4_seyan_got |= 1 << 0;
    player.recompute_lab4_seyan_state();
    assert_eq!(player.lab4_seyan_state, 30);
}
