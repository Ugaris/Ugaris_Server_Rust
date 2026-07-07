use super::*;

#[test]
fn staffer_ppd_marks_animation_book_once() {
    let mut player = PlayerRuntime::connected(1, 0);

    assert!(player.mark_staffer_animation_book_seen());
    assert!(!player.mark_staffer_animation_book_seen());
    assert_eq!(
        read_i32(&player.staffer_ppd, STAFFER_PPD_SHANRA_STATE_OFFSET),
        3
    );

    let encoded = player.encode_legacy_staffer_ppd();
    assert_eq!(encoded.len(), LEGACY_STAFFER_PPD_SIZE);
    assert_eq!(read_i32(&encoded, STAFFER_PPD_SHANRA_STATE_OFFSET), 3);
}

#[test]
fn staffer_ppd_round_trips_through_outer_blob() {
    let mut staffer = vec![0; LEGACY_STAFFER_PPD_SIZE];
    write_i32(&mut staffer, STAFFER_PPD_SHANRA_STATE_OFFSET, 2);
    let mut blob = Vec::new();
    write_ppd_block(&mut blob, DRD_STAFFER_PPD, &staffer);

    let mut player = PlayerRuntime::connected(1, 0);
    assert!(player.decode_legacy_ppd_blob(&blob));
    assert_eq!(
        read_i32(&player.staffer_ppd, STAFFER_PPD_SHANRA_STATE_OFFSET),
        2
    );

    assert!(player.mark_staffer_animation_book_seen());
    let encoded = player.encode_legacy_ppd_blob(&blob);
    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(
        read_i32(&decoded.staffer_ppd, STAFFER_PPD_SHANRA_STATE_OFFSET),
        3
    );
}

#[test]
fn staffer_ppd_tracks_forestbran_done_from_treasure_dig() {
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(player.forestbran_done(), 0);
    assert_eq!(player.set_forestbran_done(2), Some(3));
    assert_eq!(player.set_forestbran_done(5), None);
    assert_eq!(player.forestbran_done(), 3);

    let encoded = player.encode_legacy_staffer_ppd();
    assert_eq!(encoded.len(), LEGACY_STAFFER_PPD_SIZE);
    assert_eq!(read_i32(&encoded, STAFFER_PPD_FORESTBRAN_DONE_OFFSET), 3);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_staffer_ppd(&encoded));
    assert_eq!(decoded.forestbran_done(), 3);
}

#[test]
fn staffer_ppd_exposes_quest_npc_states_for_questlog_init() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_staffer_carlos_state(6);
    player.set_staffer_smugglecom_state(10);
    player.set_staffer_aristocrat_state(8);
    player.set_staffer_yoatin_state(9);
    player.set_staffer_countbran_state(1);
    player.set_staffer_countbran_bits(1 | 2 | 4);
    player.set_staffer_brennethbran_state(12);
    player.set_staffer_spiritbran_state(5);
    player.set_staffer_broklin_state(11);
    player.set_staffer_dwarfchief_state(14);
    player.set_staffer_dwarfshaman_state(9);

    let encoded = player.encode_legacy_staffer_ppd();
    assert_eq!(encoded.len(), LEGACY_STAFFER_PPD_SIZE);
    assert_eq!(read_i32(&encoded, STAFFER_PPD_CARLOS_STATE_OFFSET), 6);
    assert_eq!(read_i32(&encoded, STAFFER_PPD_SMUGGLECOM_STATE_OFFSET), 10);
    assert_eq!(read_i32(&encoded, STAFFER_PPD_ARISTOCRAT_STATE_OFFSET), 8);
    assert_eq!(read_i32(&encoded, STAFFER_PPD_YOATIN_STATE_OFFSET), 9);
    assert_eq!(read_i32(&encoded, STAFFER_PPD_COUNTBRAN_STATE_OFFSET), 1);
    assert_eq!(read_i32(&encoded, STAFFER_PPD_COUNTBRAN_BITS_OFFSET), 7);
    assert_eq!(
        read_i32(&encoded, STAFFER_PPD_BRENNETHBRAN_STATE_OFFSET),
        12
    );
    assert_eq!(read_i32(&encoded, STAFFER_PPD_SPIRITBRAN_STATE_OFFSET), 5);
    assert_eq!(read_i32(&encoded, STAFFER_PPD_BROKLIN_STATE_OFFSET), 11);
    assert_eq!(read_i32(&encoded, STAFFER_PPD_DWARFCHIEF_STATE_OFFSET), 14);
    assert_eq!(read_i32(&encoded, STAFFER_PPD_DWARFSHAMAN_STATE_OFFSET), 9);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_staffer_ppd(&encoded));
    let state = decoded.staff_quest_state();
    assert_eq!(state.carlos_state, 6);
    assert_eq!(state.smugglecom_state, 10);
    assert_eq!(state.aristocrat_state, 8);
    assert_eq!(state.yoatin_state, 9);
    assert_eq!(state.countbran_state, 1);
    assert_eq!(state.countbran_bits, 7);
    assert_eq!(state.brennethbran_state, 12);
    assert_eq!(state.spiritbran_state, 5);
    assert_eq!(state.broklin_state, 11);
    assert_eq!(state.dwarfchief_state, 14);
    assert_eq!(state.dwarfshaman_state, 9);
}

#[test]
fn staffer_ppd_blob_replaces_and_appends_legacy_block() {
    let mut existing_staffer = vec![0; LEGACY_STAFFER_PPD_SIZE];
    write_i32(&mut existing_staffer, STAFFER_PPD_FORESTBRAN_DONE_OFFSET, 4);

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, DRD_STAFFER_PPD, &existing_staffer);

    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(player.set_forestbran_done(1), Some(2));
    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), DRD_STAFFER_PPD);
    assert_eq!(read_u32(&encoded, 4), LEGACY_STAFFER_PPD_SIZE as u32);
    assert_eq!(
        read_i32(&encoded, 8 + STAFFER_PPD_FORESTBRAN_DONE_OFFSET),
        2
    );

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.forestbran_done(), 2);

    let appended = player.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&appended, 0), DRD_STAFFER_PPD);
}
