use super::*;

#[test]
fn swear_ppd_codec_preserves_counters_and_maps_banned_till() {
    let mut bytes = vec![0; LEGACY_SWEAR_PPD_SIZE];
    write_i32(&mut bytes, 0, 11);
    write_i32(&mut bytes, 40, 22);
    bytes[44..49].copy_from_slice(b"hello");
    write_i32(&mut bytes, SWEAR_PPD_BANNED_TILL_OFFSET, 1234);

    let mut player = PlayerRuntime::connected(1, 0);
    assert!(player.decode_legacy_swear_ppd(&bytes));
    assert_eq!(player.shutup_until_seconds, 1234);

    player.shutup_until_seconds = 5678;
    let encoded = player.encode_legacy_swear_ppd();
    assert_eq!(encoded.len(), LEGACY_SWEAR_PPD_SIZE);
    assert_eq!(read_i32(&encoded, 0), 11);
    assert_eq!(read_i32(&encoded, 40), 22);
    assert_eq!(&encoded[44..49], b"hello");
    assert_eq!(read_i32(&encoded, SWEAR_PPD_BANNED_TILL_OFFSET), 5678);
}

#[test]
fn swear_ppd_outer_blob_replaces_appends_and_removes_empty_state() {
    let mut existing = Vec::new();
    let mut old_swear = vec![0; LEGACY_SWEAR_PPD_SIZE];
    write_i32(&mut old_swear, 0, 77);
    write_ppd_block(&mut existing, DRD_SWEAR_PPD, &old_swear);
    write_ppd_block(&mut existing, 0x5566_7788, &[3]);

    let mut player = PlayerRuntime::connected(1, 0);
    player.swear_ppd = old_swear;
    player.shutup_until_seconds = 600;
    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), DRD_SWEAR_PPD);
    assert_eq!(read_i32(&encoded, 8), 77);
    assert_eq!(read_i32(&encoded, 8 + SWEAR_PPD_BANNED_TILL_OFFSET), 600);
    assert_eq!(read_u32(&encoded, 8 + LEGACY_SWEAR_PPD_SIZE), 0x5566_7788);

    let mut appended = PlayerRuntime::connected(2, 0);
    appended.shutup_until_seconds = 700;
    let encoded = appended.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&encoded, 0), DRD_SWEAR_PPD);
    assert_eq!(read_i32(&encoded, 8 + SWEAR_PPD_BANNED_TILL_OFFSET), 700);

    let empty = PlayerRuntime::connected(3, 0);
    let encoded = empty.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), 0x5566_7788);
    assert!(!encoded
        .windows(4)
        .any(|window| window == DRD_SWEAR_PPD.to_le_bytes()));
}

#[test]
fn ignore_ppd_codec_matches_legacy_fixed_array() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(
        player.toggle_ignored_character(42),
        IgnoreToggleResult::Added
    );
    assert_eq!(
        player.toggle_ignored_character(99),
        IgnoreToggleResult::Added
    );
    assert!(player.ignores_character(42));

    let bytes = player.encode_legacy_ignore_ppd();
    assert_eq!(bytes.len(), LEGACY_IGNORE_PPD_SIZE);
    assert_eq!(read_i32(&bytes, 0), 42);
    assert_eq!(read_i32(&bytes, 4), 99);
    assert_eq!(read_i32(&bytes, 8), 0);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ignore_ppd(&bytes));
    assert_eq!(decoded.ignored_characters, vec![42, 99]);
    assert_eq!(
        decoded.toggle_ignored_character(42),
        IgnoreToggleResult::Removed
    );
    assert!(!decoded.ignores_character(42));
}

#[test]
fn ignore_ppd_outer_blob_replaces_and_removes_empty_lists() {
    let mut existing = Vec::new();
    write_ppd_block(&mut existing, DRD_IGNORE_PPD, &[1; LEGACY_IGNORE_PPD_SIZE]);
    write_ppd_block(&mut existing, 0x8765_4321, &[7]);

    let mut player = PlayerRuntime::connected(1, 0);
    player.ignored_characters.push(123);
    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), DRD_IGNORE_PPD);
    assert_eq!(read_i32(&encoded, 8), 123);
    assert_eq!(read_u32(&encoded, 8 + LEGACY_IGNORE_PPD_SIZE), 0x8765_4321);

    player.clear_ignored_characters();
    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), 0x8765_4321);
    assert!(!encoded
        .windows(4)
        .any(|window| window == DRD_IGNORE_PPD.to_le_bytes()));
}

#[test]
fn alias_ppd_codec_matches_legacy_fixed_arrays() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.aliases.push(CommandAlias {
        from: "tyvm123".to_string(),
        to: "Thank you very much for everything".to_string(),
    });

    let bytes = player.encode_legacy_alias_ppd();
    assert_eq!(bytes.len(), LEGACY_ALIAS_PPD_SIZE);
    assert_eq!(&bytes[..8], b"tyvm123\0");
    assert_eq!(&bytes[8..42], b"Thank you very much for everything");
    assert_eq!(bytes[42], 0);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_alias_ppd(&bytes));
    assert_eq!(decoded.aliases, player.aliases);
}

#[test]
fn alias_ppd_outer_blob_replaces_and_removes_empty_aliases() {
    let mut existing = Vec::new();
    write_ppd_block(&mut existing, DRD_ALIAS_PPD, &[1; LEGACY_ALIAS_PPD_SIZE]);
    write_ppd_block(&mut existing, 0x1234_5678, &[9]);

    let mut player = PlayerRuntime::connected(1, 0);
    player.aliases.push(CommandAlias {
        from: "ty".to_string(),
        to: "Thank you!".to_string(),
    });
    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), DRD_ALIAS_PPD);
    assert_eq!(&encoded[8..11], b"ty\0");
    assert_eq!(read_u32(&encoded, 8 + LEGACY_ALIAS_PPD_SIZE), 0x1234_5678);

    player.aliases.clear();
    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), 0x1234_5678);
    assert!(!encoded
        .windows(4)
        .any(|window| window == DRD_ALIAS_PPD.to_le_bytes()));
}

#[test]
fn alias_expansion_matches_legacy_word_boundaries() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.aliases.push(CommandAlias {
        from: "ty".to_string(),
        to: "Thank you".to_string(),
    });
    player.aliases.push(CommandAlias {
        from: "don't".to_string(),
        to: "do not".to_string(),
    });

    assert_eq!(player.expand_aliases("ty!"), "Thank you!");
    assert_eq!(player.expand_aliases("pretty ty"), "pretty Thank you");
    assert_eq!(player.expand_aliases("don't stop"), "do not stop");
}

#[test]
fn lostcon_ppd_codec_matches_legacy_c_layout() {
    assert_eq!(LOSTCON_PPD_HINTS_OFFSET + 4, LEGACY_LOSTCON_PPD_SIZE);
    assert_eq!(LOSTCON_PPD_AUTOBLESS_OFFSET, 0);
    assert_eq!(LOSTCON_PPD_NORECALL_OFFSET + 4, LOSTCON_PPD_AUTOTURN_OFFSET);

    let mut player = PlayerRuntime::connected(1, 0);
    player.set_max_lag_seconds(17);
    player.hints_disabled = true;
    player.autoturn_enabled = true;
    player.autobless_enabled = true;
    player.autopulse_enabled = true;
    player.no_ball = true;
    player.no_bless = true;
    player.no_fireball = true;
    player.no_flash = true;
    player.no_freeze = true;
    player.no_heal = true;
    player.no_shield = true;
    player.no_warcry = true;
    player.no_life = true;
    player.no_mana = true;
    player.no_combo = true;
    player.no_move = true;
    player.no_pulse = true;
    player.no_recall = true;

    let encoded = player.encode_legacy_lostcon_ppd();
    assert_eq!(encoded.len(), LEGACY_LOSTCON_PPD_SIZE);
    assert_eq!(read_i32(&encoded, LOSTCON_PPD_AUTOBLESS_OFFSET), 1);
    assert_eq!(read_i32(&encoded, LOSTCON_PPD_AUTOPULSE_OFFSET), 1);
    assert_eq!(read_i32(&encoded, LOSTCON_PPD_NOBLESS_OFFSET), 1);
    assert_eq!(read_i32(&encoded, LOSTCON_PPD_NOHEAL_OFFSET), 1);
    assert_eq!(read_i32(&encoded, LOSTCON_PPD_NOFLASH_OFFSET), 1);
    assert_eq!(read_i32(&encoded, LOSTCON_PPD_NOFIREBALL_OFFSET), 1);
    assert_eq!(read_i32(&encoded, LOSTCON_PPD_NOBALL_OFFSET), 1);
    assert_eq!(read_i32(&encoded, LOSTCON_PPD_NOSHIELD_OFFSET), 1);
    assert_eq!(read_i32(&encoded, LOSTCON_PPD_NOWARCRY_OFFSET), 1);
    assert_eq!(read_i32(&encoded, LOSTCON_PPD_NOFREEZE_OFFSET), 1);
    assert_eq!(read_i32(&encoded, LOSTCON_PPD_NOMANA_OFFSET), 1);
    assert_eq!(read_i32(&encoded, LOSTCON_PPD_NOLIFE_OFFSET), 1);
    assert_eq!(read_i32(&encoded, LOSTCON_PPD_NOCOMBO_OFFSET), 1);
    assert_eq!(read_i32(&encoded, LOSTCON_PPD_NOMOVE_OFFSET), 1);
    assert_eq!(read_i32(&encoded, LOSTCON_PPD_NOPULSE_OFFSET), 1);
    assert_eq!(read_i32(&encoded, LOSTCON_PPD_NORECALL_OFFSET), 1);
    assert_eq!(read_i32(&encoded, LOSTCON_PPD_AUTOTURN_OFFSET), 1);
    assert_eq!(read_i32(&encoded, LOSTCON_PPD_MAXLAG_OFFSET), 17);
    assert_eq!(read_i32(&encoded, LOSTCON_PPD_HINTS_OFFSET), 1);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_lostcon_ppd(&encoded));
    assert_eq!(decoded.max_lag_seconds, 17);
    assert!(decoded.hints_disabled);
    assert!(decoded.autoturn_enabled);
    assert!(decoded.autobless_enabled);
    assert!(decoded.autopulse_enabled);
    assert!(decoded.no_ball);
    assert!(decoded.no_bless);
    assert!(decoded.no_fireball);
    assert!(decoded.no_flash);
    assert!(decoded.no_freeze);
    assert!(decoded.no_heal);
    assert!(decoded.no_shield);
    assert!(decoded.no_warcry);
    assert!(decoded.no_life);
    assert!(decoded.no_mana);
    assert!(decoded.no_combo);
    assert!(decoded.no_move);
    assert!(decoded.no_pulse);
    assert!(decoded.no_recall);
    assert!(!decoded.decode_legacy_lostcon_ppd(&encoded[..LEGACY_LOSTCON_PPD_SIZE - 1]));
}

#[test]
fn lostcon_ppd_blob_round_trips_with_legacy_block_framing() {
    let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
    let mut existing_lostcon = vec![0; LEGACY_LOSTCON_PPD_SIZE];
    write_i32(&mut existing_lostcon, LOSTCON_PPD_MAXLAG_OFFSET, 9);

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
    write_ppd_block(&mut existing, DRD_LOSTCON_PPD, &existing_lostcon);

    let mut player = PlayerRuntime::connected(1, 0);
    player.set_max_lag_seconds(19);

    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), unknown_id);
    assert_eq!(read_u32(&encoded, 12), DRD_LOSTCON_PPD);
    assert_eq!(read_u32(&encoded, 16), LEGACY_LOSTCON_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 20 + LOSTCON_PPD_MAXLAG_OFFSET), 19);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.max_lag_seconds, 19);
}

#[test]
fn ppd_blob_appends_lostcon_without_existing_block() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.autoturn_enabled = true;

    let encoded = player.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&encoded, 0), DRD_LOSTCON_PPD);
    assert_eq!(read_u32(&encoded, 4), LEGACY_LOSTCON_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 8 + LOSTCON_PPD_AUTOTURN_OFFSET), 1);
    assert_eq!(read_i32(&encoded, 8 + LOSTCON_PPD_MAXLAG_OFFSET), 0);
    assert_eq!(read_i32(&encoded, 8 + LOSTCON_PPD_HINTS_OFFSET), 0);
}

#[test]
fn ppd_blob_appends_lostcon_for_a_lag_control_toggle_alone() {
    // A lone `/noball`-style toggle (no `autoturn`/`maxlag`/`hints`
    // touched) must still force a fresh `DRD_LOSTCON_PPD` block, not
    // silently no-op like an all-default PPD would.
    let mut player = PlayerRuntime::connected(1, 0);
    player.no_ball = true;

    let encoded = player.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&encoded, 0), DRD_LOSTCON_PPD);
    assert_eq!(read_i32(&encoded, 8 + LOSTCON_PPD_NOBALL_OFFSET), 1);

    let untouched = PlayerRuntime::connected(2, 0);
    assert!(untouched.encode_legacy_ppd_blob(&[]).is_empty());
}

#[test]
fn record_swap_stamps_and_reads_back_the_swapped_offset() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(player.swapped_at(), 0);

    player.record_swap(123_456);

    assert_eq!(player.swapped_at(), 123_456);
    assert_eq!(read_i32(&player.misc_ppd, MISC_PPD_SWAPPED_OFFSET), 123_456);
    // Unrelated fields untouched.
    assert_eq!(player.misc_ppd[MISC_PPD_TREEDONE_OFFSET], 0);
    assert_eq!(read_i32(&player.misc_ppd, MISC_PPD_GIFT_YEAR_OFFSET), 0);
}

#[test]
fn record_complaint_stamps_and_reads_back_the_complaint_date_offset() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(player.complaint_date(), 0);

    player.record_complaint(1);
    assert_eq!(player.complaint_date(), 1);

    player.record_complaint(123_456);
    assert_eq!(player.complaint_date(), 123_456);
    assert_eq!(
        read_i32(&player.misc_ppd, MISC_PPD_COMPLAINT_DATE_OFFSET),
        123_456
    );
    // Unrelated fields untouched.
    assert_eq!(read_i32(&player.misc_ppd, MISC_PPD_SWAPPED_OFFSET), 0);
    assert_eq!(player.misc_ppd[MISC_PPD_TREEDONE_OFFSET], 0);
}
