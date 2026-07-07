use super::*;

/// Round-trips every `area1_ppd` field `cmd_showppd`'s `/showppd <name>
/// area1` branch reads (`src/system/command.c:303-336`) that had no
/// prior accessor - i.e. everything except the 10 quest-driver states
/// already covered by `area1_ppd_codec_matches_legacy_c_layout` and the
/// two `forest_ranger_*` fields, which `cmd_showppd` itself never reads
/// (confirmed by reading the whole function) even though they now back
/// a real gameplay driver - see `world::forest_ranger` and
/// `area1_ppd_codec_matches_legacy_c_layout`'s own `forest_ranger_state`
/// coverage.
#[test]
fn area1_ppd_exposes_remaining_fields_for_showppd() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_area1_yoakin_seen_timer(1);
    player.set_area1_gwendy_seen_timer(2);
    player.set_area1_terion_state(3);
    player.set_area1_flags(4);
    player.set_area1_darkin_state(5);
    player.set_area1_gerewin_state(6);
    player.set_area1_gerewin_seen_timer(7);
    player.set_area1_lydia_seen_timer(8);
    player.set_area1_asturin_state(9);
    player.set_area1_asturin_seen_timer(10);
    player.set_area1_guiwynn_seen_timer(11);
    player.set_area1_logain_seen_timer(12);
    player.set_area1_reskin_seen_timer(13);
    player.set_area1_reskin_got_bits(14);
    player.set_area1_shrike_state(15);
    player.set_area1_shrike_fails(16);
    player.set_area1_brithildie_seen_timer(17);
    player.set_area1_jiu_state(18);
    player.set_area1_jiu_seen_timer(19);
    player.set_area1_greeter_state(20);
    player.set_area1_greeter_seen_timer(21);
    player.set_area1_aclerk_state(22);
    player.set_area1_aclerk_seen_timer(23);
    player.set_area1_camhermit_seen_timer(24);
    player.set_area1_camhermit_kills(25);
    player.set_area1_jessica_seen_timer(26);
    player.set_area1_forest_ranger_seen_timer(27);

    let encoded = player.encode_legacy_area1_ppd();
    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_area1_ppd(&encoded));

    assert_eq!(decoded.area1_yoakin_seen_timer(), 1);
    assert_eq!(decoded.area1_gwendy_seen_timer(), 2);
    assert_eq!(decoded.area1_terion_state(), 3);
    assert_eq!(decoded.area1_flags(), 4);
    assert_eq!(decoded.area1_darkin_state(), 5);
    assert_eq!(decoded.area1_gerewin_state(), 6);
    assert_eq!(decoded.area1_gerewin_seen_timer(), 7);
    assert_eq!(decoded.area1_lydia_seen_timer(), 8);
    assert_eq!(decoded.area1_asturin_state(), 9);
    assert_eq!(decoded.area1_asturin_seen_timer(), 10);
    assert_eq!(decoded.area1_guiwynn_seen_timer(), 11);
    assert_eq!(decoded.area1_logain_seen_timer(), 12);
    assert_eq!(decoded.area1_reskin_seen_timer(), 13);
    assert_eq!(decoded.area1_reskin_got_bits(), 14);
    assert_eq!(decoded.area1_shrike_state(), 15);
    assert_eq!(decoded.area1_shrike_fails(), 16);
    assert_eq!(decoded.area1_brithildie_seen_timer(), 17);
    assert_eq!(decoded.area1_jiu_state(), 18);
    assert_eq!(decoded.area1_jiu_seen_timer(), 19);
    assert_eq!(decoded.area1_greeter_state(), 20);
    assert_eq!(decoded.area1_greeter_seen_timer(), 21);
    assert_eq!(decoded.area1_aclerk_state(), 22);
    assert_eq!(decoded.area1_aclerk_seen_timer(), 23);
    assert_eq!(decoded.area1_camhermit_seen_timer(), 24);
    assert_eq!(decoded.area1_camhermit_kills(), 25);
    assert_eq!(decoded.area1_jessica_seen_timer(), 26);
    assert_eq!(decoded.area1_forest_ranger_seen_timer(), 27);
}

#[test]
fn area1_ppd_codec_matches_legacy_c_layout() {
    assert_eq!(
        DRD_AREA1_PPD,
        make_drd(DEV_ID_DB, 22 | PERSISTENT_PLAYER_DATA)
    );
    assert_eq!(LEGACY_AREA1_PPD_SIZE, 156);

    let mut player = PlayerRuntime::connected(1, 0);
    player.set_area1_yoakin_state(5);
    player.set_area1_gwendy_state(18);
    player.set_area1_nook_state(12);
    player.set_area1_lydia_state(6);
    player.set_area1_guiwynn_state(9);
    player.set_area1_logain_state(6);
    player.set_area1_reskin_state(8);
    player.set_area1_brithildie_state(21);
    player.set_area1_camhermit_state(13);
    player.set_area1_jessica_state(11);
    player.set_area1_forest_ranger_state(4);

    let encoded = player.encode_legacy_area1_ppd();
    assert_eq!(encoded.len(), LEGACY_AREA1_PPD_SIZE);
    assert_eq!(read_i32(&encoded, AREA1_PPD_YOAKIN_STATE_OFFSET), 5);
    assert_eq!(read_i32(&encoded, AREA1_PPD_GWENDY_STATE_OFFSET), 18);
    assert_eq!(read_i32(&encoded, AREA1_PPD_NOOK_STATE_OFFSET), 12);
    assert_eq!(read_i32(&encoded, AREA1_PPD_LYDIA_STATE_OFFSET), 6);
    assert_eq!(read_i32(&encoded, AREA1_PPD_GUIWYNN_STATE_OFFSET), 9);
    assert_eq!(read_i32(&encoded, AREA1_PPD_LOGAIN_STATE_OFFSET), 6);
    assert_eq!(read_i32(&encoded, AREA1_PPD_RESKIN_STATE_OFFSET), 8);
    assert_eq!(read_i32(&encoded, AREA1_PPD_BRITHILDIE_STATE_OFFSET), 21);
    assert_eq!(read_i32(&encoded, AREA1_PPD_CAMHERMIT_STATE_OFFSET), 13);
    assert_eq!(read_i32(&encoded, AREA1_PPD_JESSICA_STATE_OFFSET), 11);
    assert_eq!(read_i32(&encoded, AREA1_PPD_FOREST_RANGER_STATE_OFFSET), 4);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_area1_ppd(&encoded));
    assert_eq!(decoded.area1_yoakin_state(), 5);
    assert_eq!(decoded.area1_gwendy_state(), 18);
    assert_eq!(decoded.area1_nook_state(), 12);
    assert_eq!(decoded.area1_lydia_state(), 6);
    assert_eq!(decoded.area1_guiwynn_state(), 9);
    assert_eq!(decoded.area1_logain_state(), 6);
    assert_eq!(decoded.area1_reskin_state(), 8);
    assert_eq!(decoded.area1_brithildie_state(), 21);
    assert_eq!(decoded.area1_camhermit_state(), 13);
    assert_eq!(decoded.area1_forest_ranger_state(), 4);
    assert_eq!(decoded.area1_jessica_state(), 11);

    let state = decoded.area1_quest_state();
    assert_eq!(state.yoakin_state, 5);
    assert_eq!(state.gwendy_state, 18);
    assert_eq!(state.nook_state, 12);
    assert_eq!(state.lydia_state, 6);
    assert_eq!(state.guiwynn_state, 9);
    assert_eq!(state.logain_state, 6);
    assert_eq!(state.reskin_state, 8);
    assert_eq!(state.brithildie_state, 21);
    assert_eq!(state.camhermit_state, 13);
    assert_eq!(state.jessica_state, 11);
}

#[test]
fn area1_ppd_blob_replaces_and_appends_legacy_block() {
    let mut existing_area1 = vec![0; LEGACY_AREA1_PPD_SIZE];
    write_i32(&mut existing_area1, AREA1_PPD_LYDIA_STATE_OFFSET, 3);

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, 0x1122_3344, &[1, 2, 3]);
    write_ppd_block(&mut existing, DRD_AREA1_PPD, &existing_area1);

    let mut player = PlayerRuntime::connected(1, 0);
    player.set_area1_lydia_state(6);
    let encoded = player.encode_legacy_ppd_blob(&existing);

    assert_eq!(read_u32(&encoded, 0), 0x1122_3344);
    assert_eq!(read_u32(&encoded, 11), DRD_AREA1_PPD);
    assert_eq!(read_u32(&encoded, 15), LEGACY_AREA1_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 19 + AREA1_PPD_LYDIA_STATE_OFFSET), 6);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.area1_lydia_state(), 6);

    let appended = player.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&appended, 0), DRD_AREA1_PPD);
}
