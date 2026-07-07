use super::*;

#[test]
fn pk_ppd_codec_matches_legacy_c_layout() {
    assert_eq!(
        PK_PPD_HATE_OFFSET + PK_HATE_MAX_ENTRIES * 4,
        LEGACY_PK_PPD_SIZE
    );

    let mut player = PlayerRuntime::connected(1, 0);
    player.pk_kills = 3;
    player.pk_deaths = 4;
    player.pk_last_kill = 0x1122_3344;
    player.pk_last_death = i32::MAX as u32 + 99;
    assert!(player.add_pk_hate(1001));
    assert!(player.add_pk_hate(1002));
    assert!(!player.add_pk_hate(1002));

    let encoded = player.encode_legacy_pk_ppd();
    assert_eq!(encoded.len(), LEGACY_PK_PPD_SIZE);
    assert_eq!(read_i32(&encoded, PK_PPD_KILLS_OFFSET), 3);
    assert_eq!(read_i32(&encoded, PK_PPD_DEATHS_OFFSET), 4);
    assert_eq!(read_i32(&encoded, PK_PPD_LAST_KILL_OFFSET), 0x1122_3344);
    assert_eq!(read_i32(&encoded, PK_PPD_LAST_DEATH_OFFSET), i32::MAX);
    assert_eq!(read_i32(&encoded, PK_PPD_HATE_OFFSET), 1002);
    assert_eq!(read_i32(&encoded, PK_PPD_HATE_OFFSET + 4), 1001);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_pk_ppd(&encoded));
    assert_eq!(decoded.pk_kills, 3);
    assert_eq!(decoded.pk_deaths, 4);
    assert_eq!(decoded.pk_last_kill, 0x1122_3344);
    assert_eq!(decoded.pk_last_death, i32::MAX as u32);
    assert_eq!(decoded.pk_hate, vec![1002, 1001]);
    assert!(decoded.has_pk_hate_for(1001));
    assert!(!decoded.has_pk_hate_for(1003));
    assert!(!decoded.decode_legacy_pk_ppd(&encoded[..LEGACY_PK_PPD_SIZE - 1]));
}

#[test]
fn pk_hate_helpers_preserve_legacy_front_priority_and_eviction() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert!(!player.add_pk_hate(0));
    assert!(player.add_pk_hate(10));
    assert!(player.add_pk_hate(20));
    assert!(player.add_pk_hate(30));
    assert_eq!(player.pk_hate, vec![30, 20, 10]);

    assert!(!player.add_pk_hate(10));
    assert_eq!(player.pk_hate, vec![10, 30, 20]);

    assert!(player.remove_pk_hate(30));
    assert_eq!(player.pk_hate, vec![10, 0, 20]);
    assert!(player.has_any_pk_hate());
    assert!(!player.remove_pk_hate(30));

    let encoded = player.encode_legacy_pk_ppd();
    assert_eq!(read_i32(&encoded, PK_PPD_HATE_OFFSET), 10);
    assert_eq!(read_i32(&encoded, PK_PPD_HATE_OFFSET + 4), 0);
    assert_eq!(read_i32(&encoded, PK_PPD_HATE_OFFSET + 8), 20);

    for id in 100..(100 + PK_HATE_MAX_ENTRIES as u32 + 5) {
        player.add_pk_hate(id);
    }
    assert_eq!(player.pk_hate.len(), PK_HATE_MAX_ENTRIES);
    assert_eq!(player.pk_hate[0], 154);
    assert_eq!(player.pk_hate[PK_HATE_MAX_ENTRIES - 1], 105);
    assert!(!player.has_pk_hate_for(104));
}

#[test]
fn pk_hate_decode_preserves_legacy_removed_slot_holes() {
    let mut bytes = vec![0; LEGACY_PK_PPD_SIZE];
    write_i32(&mut bytes, PK_PPD_HATE_OFFSET, 10);
    write_i32(&mut bytes, PK_PPD_HATE_OFFSET + 4, 0);
    write_i32(&mut bytes, PK_PPD_HATE_OFFSET + 8, 20);

    let mut player = PlayerRuntime::connected(1, 0);
    assert!(player.decode_legacy_pk_ppd(&bytes));
    assert_eq!(player.pk_hate, vec![10, 0, 20]);
    assert_eq!(
        player.active_pk_hate_ids().collect::<Vec<_>>(),
        vec![10, 20]
    );
    assert!(player.remove_pk_hate(10));
    assert_eq!(player.pk_hate, vec![0, 0, 20]);
    assert!(player.remove_pk_hate(20));
    assert!(player.pk_hate.is_empty());
    assert!(!player.has_any_pk_hate());
}

#[test]
fn pk_hate_hit_helper_clears_legacy_lag_flag() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut character = character(1);
    character.flags.insert(CharacterFlags::LAG);

    assert!(player.add_pk_hate_from_hit(&mut character, 20));
    assert_eq!(player.pk_hate, vec![20]);
    assert!(!character.flags.contains(CharacterFlags::LAG));

    character.flags.insert(CharacterFlags::LAG);
    assert!(!player.add_pk_hate_from_hit(&mut character, 20));
    assert_eq!(player.pk_hate, vec![20]);
    assert!(!character.flags.contains(CharacterFlags::LAG));

    character.flags.insert(CharacterFlags::LAG);
    assert!(!player.add_pk_hate_from_hit(&mut character, 0));
    assert!(character.flags.contains(CharacterFlags::LAG));
}

#[test]
fn pk_ppd_blob_round_trips_with_legacy_block_framing() {
    let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
    let mut existing_pk = vec![0; LEGACY_PK_PPD_SIZE];
    write_i32(&mut existing_pk, PK_PPD_KILLS_OFFSET, 1);
    write_i32(&mut existing_pk, PK_PPD_HATE_OFFSET, 999);

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
    write_ppd_block(&mut existing, DRD_PK_PPD, &existing_pk);

    let mut player = PlayerRuntime::connected(1, 0);
    player.pk_deaths = 2;
    assert!(player.add_pk_hate(1234));

    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), unknown_id);
    assert_eq!(read_u32(&encoded, 12), DRD_PK_PPD);
    assert_eq!(read_u32(&encoded, 16), LEGACY_PK_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 20 + PK_PPD_KILLS_OFFSET), 0);
    assert_eq!(read_i32(&encoded, 20 + PK_PPD_DEATHS_OFFSET), 2);
    assert_eq!(read_i32(&encoded, 20 + PK_PPD_HATE_OFFSET), 1234);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.pk_deaths, 2);
    assert_eq!(decoded.pk_hate, vec![1234]);
}

#[test]
fn ppd_blob_appends_pk_without_existing_block() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert!(player.add_pk_hate(777));

    let encoded = player.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&encoded, 0), DRD_PK_PPD);
    assert_eq!(read_u32(&encoded, 4), LEGACY_PK_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 8 + PK_PPD_HATE_OFFSET), 777);
}
