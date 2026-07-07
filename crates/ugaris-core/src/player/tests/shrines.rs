use super::*;

#[test]
fn special_shrine_requires_confirmation_then_removes_hardcore() {
    let mut player = PlayerRuntime::connected(7, 11);
    let mut character = character(3);
    character.flags.insert(CharacterFlags::HARDCORE);
    character.creation_time = SPECIAL_SHRINE_HCSC_CUTOFF_SECONDS;

    assert_eq!(
        player.touch_special_shrine(&mut character, 0x0A, 100),
        SpecialShrineResult::ConfirmRequired,
    );
    assert!(character.flags.contains(CharacterFlags::HARDCORE));
    assert_eq!(
        player.touch_special_shrine(&mut character, 0x0A, 109),
        SpecialShrineResult::HardcoreRemoved,
    );
    assert!(!character.flags.contains(CharacterFlags::HARDCORE));
}

#[test]
fn special_shrine_blocks_non_hardcore_and_new_hardcore() {
    let mut player = PlayerRuntime::connected(7, 11);
    let mut softcore = character(3);
    assert_eq!(
        player.touch_special_shrine(&mut softcore, 0x0A, 100),
        SpecialShrineResult::NothingHere,
    );

    let mut new_hardcore = character(4);
    new_hardcore.flags.insert(CharacterFlags::HARDCORE);
    new_hardcore.creation_time = SPECIAL_SHRINE_HCSC_CUTOFF_SECONDS + 1;
    assert_eq!(
        player.touch_special_shrine(&mut new_hardcore, 0x0A, 100),
        SpecialShrineResult::NothingHere,
    );
    assert!(new_hardcore.flags.contains(CharacterFlags::HARDCORE));
}

#[test]
fn demonshrine_touch_updates_value_and_blocks_repeats() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut character = character(3);
    character.exp = 10_000;

    assert_eq!(
        player.touch_demonshrine(&mut character, 0x0001_0203),
        DemonShrineResult::Learned { exp_added: 350 }
    );
    assert_eq!(character.values[1][CharacterValue::Demon as usize], 1);
    // C `demonshrine_driver` (`base.c:3231-3235`) applies the returned
    // `exp_added` via `give_exp`/`update_char`, both of which need
    // `&mut World` and so are the caller's responsibility
    // (`World::give_exp`/`World::update_character`, wired at the
    // `ItemDriverOutcome::DemonShrine` call site in
    // `ugaris-server/src/main.rs`) - `touch_demonshrine` itself no
    // longer mutates `character.exp`, only the Demon value and
    // `CF_ITEMS`.
    assert_eq!(character.exp, 10_000);
    assert!(!character.flags.contains(CharacterFlags::UPDATE));
    assert!(character.flags.contains(CharacterFlags::ITEMS));
    assert_eq!(
        player.touch_demonshrine(&mut character, 0x0001_0203),
        DemonShrineResult::AlreadyKnown
    );
}

#[test]
fn demonshrine_ppd_blob_round_trips_with_legacy_block_framing() {
    let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
    let mut existing_demonshrine = vec![0; LEGACY_DEMONSHRINE_PPD_SIZE];
    write_i32(&mut existing_demonshrine, 0, 0x0001_0203);

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
    write_ppd_block(&mut existing, DRD_DEMONSHRINE_PPD, &existing_demonshrine);

    let mut player = PlayerRuntime::connected(1, 0);
    player.demonshrines.push(0x0001_0506);

    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), unknown_id);
    assert_eq!(read_u32(&encoded, 12), DRD_DEMONSHRINE_PPD);
    assert_eq!(read_u32(&encoded, 16), LEGACY_DEMONSHRINE_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 20), 0x0001_0506);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.demonshrines, vec![0x0001_0506]);
}

#[test]
fn randomshrine_ppd_blob_round_trips_c_used_bitset() {
    let mut existing_randomshrine = vec![0; LEGACY_RANDOMSHRINE_PPD_SIZE];
    write_u32(&mut existing_randomshrine, 0, 1 << 3);
    write_u32(&mut existing_randomshrine, 28, 1 << 31);
    existing_randomshrine[32] = 17;

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, DRD_RANDOMSHRINE_PPD, &existing_randomshrine);

    let mut player = PlayerRuntime::connected(1, 0);
    assert!(player.decode_legacy_ppd_blob(&existing));
    assert!(player.has_used_random_shrine(3));
    assert!(player.has_used_random_shrine(255));
    assert!(!player.has_used_random_shrine(4));
    assert_eq!(player.random_shrine_continuity, 17);

    player.mark_random_shrine_used(64);
    player.random_shrine_continuity = 18;
    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), DRD_RANDOMSHRINE_PPD);
    assert_eq!(read_u32(&encoded, 4), LEGACY_RANDOMSHRINE_PPD_SIZE as u32);
    assert_eq!(read_u32(&encoded, 8), 1 << 3);
    assert_eq!(read_u32(&encoded, 16), 1);
    assert_eq!(read_u32(&encoded, 36), 1 << 31);
    assert_eq!(encoded[40], 18);
}
