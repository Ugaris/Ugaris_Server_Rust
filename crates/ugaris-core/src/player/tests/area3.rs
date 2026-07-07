use super::*;

#[test]
fn area3_ppd_tracks_park_shrine_memorization() {
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(
        DRD_AREA3_PPD,
        make_drd(DEV_ID_DB, 40 | PERSISTENT_PLAYER_DATA)
    );
    assert_eq!(player.memorize_park_shrine(2), Some(true));
    assert_eq!(player.memorize_park_shrine(2), Some(false));
    assert_eq!(player.memorize_park_shrine(4), None);

    let encoded = player.encode_legacy_area3_ppd();
    assert_eq!(encoded.len(), LEGACY_AREA3_PPD_SIZE);
    assert_eq!(read_i32(&encoded, AREA3_PPD_KELLY_FOUND1_OFFSET), 0);
    assert_eq!(read_i32(&encoded, AREA3_PPD_KELLY_FOUND2_OFFSET), 1);
    assert_eq!(read_i32(&encoded, AREA3_PPD_KELLY_FOUND3_OFFSET), 0);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_area3_ppd(&encoded));
    assert_eq!(decoded.memorize_park_shrine(2), Some(false));
    assert_eq!(decoded.memorize_park_shrine(3), Some(true));
}

#[test]
fn area3_ppd_exposes_clara_and_kelly_quest_states() {
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(player.area3_kelly_state(), 0);
    assert_eq!(player.area3_clara_state(), 0);

    player.set_area3_kelly_state(18);
    player.set_area3_clara_state(6);

    let encoded = player.encode_legacy_area3_ppd();
    assert_eq!(read_i32(&encoded, AREA3_PPD_KELLY_STATE_OFFSET), 18);
    assert_eq!(read_i32(&encoded, AREA3_PPD_CLARA_STATE_OFFSET), 6);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_area3_ppd(&encoded));
    assert_eq!(decoded.area3_kelly_state(), 18);
    assert_eq!(decoded.area3_clara_state(), 6);
}

#[test]
fn area3_ppd_exposes_seymour_astro2_crypt_william_hermit_states() {
    assert_eq!(LEGACY_AREA3_PPD_SIZE, 72);

    let mut player = PlayerRuntime::connected(1, 0);
    player.set_area3_seymour_state(16);
    player.set_area3_astro2_state(5);
    player.set_area3_crypt_state(15);
    player.set_area3_william_state(7);
    player.set_area3_hermit_state(5);

    let encoded = player.encode_legacy_area3_ppd();
    assert_eq!(read_i32(&encoded, AREA3_PPD_SEYMOUR_STATE_OFFSET), 16);
    assert_eq!(read_i32(&encoded, AREA3_PPD_ASTRO2_STATE_OFFSET), 5);
    assert_eq!(read_i32(&encoded, AREA3_PPD_CRYPT_STATE_OFFSET), 15);
    assert_eq!(read_i32(&encoded, AREA3_PPD_WILLIAM_STATE_OFFSET), 7);
    assert_eq!(read_i32(&encoded, AREA3_PPD_HERMIT_STATE_OFFSET), 5);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_area3_ppd(&encoded));
    assert_eq!(decoded.area3_seymour_state(), 16);
    assert_eq!(decoded.area3_astro2_state(), 5);
    assert_eq!(decoded.area3_crypt_state(), 15);
    assert_eq!(decoded.area3_william_state(), 7);
    assert_eq!(decoded.area3_hermit_state(), 5);

    let state = decoded.area3_quest_state();
    assert_eq!(state.seymour_state, 16);
    assert_eq!(state.astro2_state, 5);
    assert_eq!(state.crypt_state, 15);
    assert_eq!(state.william_state, 7);
    assert_eq!(state.hermit_state, 5);
}

#[test]
fn area3_ppd_exposes_kassim_state_for_showppd() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_area3_kassim_state(9);

    let encoded = player.encode_legacy_area3_ppd();
    assert_eq!(read_i32(&encoded, AREA3_PPD_KASSIM_STATE_OFFSET), 9);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_area3_ppd(&encoded));
    assert_eq!(decoded.area3_kassim_state(), 9);
}

#[test]
fn area3_ppd_tracks_forest_chest_imp_flags() {
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(player.area3_imp_flags(), 0);
    assert!(player.mark_area3_imp_flag(1));
    assert!(!player.mark_area3_imp_flag(1));
    assert!(player.mark_area3_imp_flag(2));
    assert_eq!(player.area3_imp_flags(), 3);

    let encoded = player.encode_legacy_area3_ppd();
    assert_eq!(read_i32(&encoded, AREA3_PPD_IMP_FLAGS_OFFSET), 3);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_area3_ppd(&encoded));
    assert_eq!(decoded.area3_imp_flags(), 3);
}

#[test]
fn reopen_william_resets_area3_imp_fields() {
    // Quest 22 ("Impish Bear Hunt") has table `flags == 0`, so
    // `reopen_quest_legacy(22)` can never reach the switch (confirmed
    // below) - the helper is exercised directly to verify the C
    // `questlog_reopen_q22` (`src/system/questlog.c:464-477`) side
    // effect is faithfully implemented regardless.
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_area3_william_state(3);
    player.set_area3_imp_state(2);
    player.set_area3_imp_kills(5);

    assert_eq!(player.reopen_william(), ReopenOutcome::Open);
    assert_eq!(player.area3_william_state(), 0);
    assert_eq!(player.area3_imp_state(), 0);
    assert_eq!(player.area3_imp_kills(), 0);

    let mut gated = PlayerRuntime::connected(2, 0);
    mark_reopenable(&mut gated, 22);
    assert_eq!(
        gated.reopen_quest_legacy(22),
        crate::quest::QuestReopenResult::CannotOpenAgain
    );
}

#[test]
fn area3_ppd_blob_replaces_and_appends_legacy_block() {
    let mut existing_area3 = vec![0; LEGACY_AREA3_PPD_SIZE];
    write_i32(&mut existing_area3, AREA3_PPD_KELLY_FOUND1_OFFSET, 1);

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, 0x1122_3344, &[1, 2, 3]);
    write_ppd_block(&mut existing, DRD_AREA3_PPD, &existing_area3);

    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(player.memorize_park_shrine(3), Some(true));
    let encoded = player.encode_legacy_ppd_blob(&existing);

    assert_eq!(read_u32(&encoded, 0), 0x1122_3344);
    assert_eq!(read_u32(&encoded, 11), DRD_AREA3_PPD);
    assert_eq!(read_u32(&encoded, 15), LEGACY_AREA3_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 19 + AREA3_PPD_KELLY_FOUND1_OFFSET), 0);
    assert_eq!(read_i32(&encoded, 19 + AREA3_PPD_KELLY_FOUND3_OFFSET), 1);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.memorize_park_shrine(3), Some(false));

    let appended = player.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&appended, 0), DRD_AREA3_PPD);
}
