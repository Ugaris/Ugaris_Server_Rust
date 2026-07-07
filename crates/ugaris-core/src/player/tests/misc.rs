use super::*;

#[test]
fn player_constants_match_c_header() {
    assert_eq!(MAX_PLAYERS, 512);
    assert_eq!(PlayerConnectionState::Connect as u8, 1);
    assert_eq!(PlayerConnectionState::Normal as u8, 2);
    assert_eq!(PlayerConnectionState::Exit as u8, 3);
    assert_eq!(PlayerActionCode::WalkDir as u8, 20);
    assert_eq!(MAX_PLAYER_EFFECTS, 64);
    assert_eq!(DRD_JUNK_PPD, 0x8100_0072);
    assert_eq!(DRD_TREASURE_CHEST_PPD, 0x8100_0011);
    assert_eq!(DRD_RANDCHEST_PPD, 0x8100_003f);
    assert_eq!(DRD_DEMONSHRINE_PPD, 0x8100_0044);
    assert_eq!(DRD_RANDOMSHRINE_PPD, 0x8100_0056);
    assert_eq!(DRD_MISC_PPD, 0x8100_0071);
    assert_eq!(DRD_ALIAS_PPD, 0x8100_0050);
    assert_eq!(DRD_IGNORE_PPD, 0x8100_0064);
    assert_eq!(DRD_SWEAR_PPD, 0x8100_006d);
    assert_eq!(DRD_STAFFER_PPD, 0x8100_0082);
    assert_eq!(DRD_FARMY_PPD, 0x8100_004d);
    assert_eq!(DRD_KEYRING_PPD, 0xbb00_0007);
    assert_eq!(LEGACY_TREASURE_CHEST_PPD_SIZE, 800);
    assert_eq!(LEGACY_RANDCHEST_PPD_SIZE, 800);
    assert_eq!(LEGACY_DEMONSHRINE_PPD_SIZE, 400);
    assert_eq!(LEGACY_RANDOMSHRINE_PPD_SIZE, 33);
    assert_eq!(LEGACY_MISC_PPD_SIZE, 36);
    assert_eq!(LEGACY_IGNORE_PPD_SIZE, 400);
    assert_eq!(LEGACY_SWEAR_PPD_SIZE, 932);
    assert_eq!(LEGACY_STAFFER_PPD_SIZE, 100);
    assert_eq!(LEGACY_FARMY_PPD_SIZE, 340);
    assert_eq!(SALTMINE_LADDER_COUNT, 20);
}

#[test]
fn reclaim_for_session_keeps_ppd_state_and_resets_session_bookkeeping() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(42));
    player.character_number = 42;
    player.ppd_blob = vec![1, 2, 3];
    player.keyring.push(KeyringEntry {
        template_id: 99,
        name: "Test Key".into(),
        description: String::new(),
        sprite: 0,
        flags: 0,
        value: 0,
        driver: 0,
        driver_data: Vec::new(),
        expire_serial: 0,
    });
    player.client_version = 4;
    player.view_distance = 40;
    player.scrollback = vec![9, 9, 9];
    player.queue.push_back(QueuedAction::default());
    player.nofight_timer = 500;

    let session_id = 2;
    let current_tick = 1_000;
    let reclaimed = player.reclaim_for_session(session_id, current_tick);

    // Session-transient bookkeeping resets like a fresh connection.
    assert_eq!(reclaimed.session_id, session_id);
    assert_eq!(reclaimed.state, PlayerConnectionState::Connect);
    assert_eq!(reclaimed.client_version, 0);
    assert_eq!(reclaimed.view_distance, DIST_OLD);
    assert_eq!(reclaimed.last_command_tick, current_tick);
    assert_eq!(reclaimed.login_tick, current_tick);
    assert!(reclaimed.queue.is_empty());
    assert!(reclaimed.scrollback.is_empty());
    assert_eq!(reclaimed.nofight_timer, 0);

    // Persistent PPD-backed state survives the reconnect untouched.
    assert_eq!(reclaimed.character_id, Some(CharacterId(42)));
    assert_eq!(reclaimed.character_number, 42);
    assert_eq!(reclaimed.ppd_blob, vec![1, 2, 3]);
    assert_eq!(reclaimed.keyring.len(), 1);
}

#[test]
fn depot_ppd_blob_round_trips_populated_and_empty_slots() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.depot[0] = Some(sample_depot_item(1, 0x1111, ItemFlags::USED));
    player.depot[42] = Some(sample_depot_item(2, 0x2222, ItemFlags::USED));

    let encoded = player.encode_legacy_depot_ppd();
    assert_eq!(encoded.len(), MAXDEPOT * DEPOT_PPD_ITEM_SIZE);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_depot_ppd(&encoded));
    assert_eq!(decoded.depot.len(), MAXDEPOT);

    let slot0 = decoded.depot[0].as_ref().expect("slot 0 populated");
    assert_eq!(slot0.template_id, 0x1111);
    assert_eq!(slot0.value, 100);
    assert_eq!(slot0.modifier_index, [1, 2, 3, 4, 5]);
    assert_eq!(slot0.modifier_value, [10, 20, 30, 40, 50]);
    assert_eq!(slot0.driver, 9);
    assert_eq!(slot0.driver_data, (0..40).collect::<Vec<u8>>());

    let slot42 = decoded.depot[42].as_ref().expect("slot 42 populated");
    assert_eq!(slot42.template_id, 0x2222);

    // Every other slot round-trips as empty (flags == 0).
    for (index, slot) in decoded.depot.iter().enumerate() {
        if index != 0 && index != 42 {
            assert!(slot.is_none(), "slot {index} should be empty");
        }
    }
}

#[test]
fn depot_ppd_outer_blob_wires_through_typed_decode_and_encode() {
    let mut source = PlayerRuntime::connected(1, 0);
    source.depot[3] = Some(sample_depot_item(9, 0x3333, ItemFlags::USED));
    let mut existing = Vec::new();
    write_ppd_block(
        &mut existing,
        DRD_DEPOT_PPD,
        &source.encode_legacy_depot_ppd(),
    );
    write_ppd_block(&mut existing, 0x5566_7788, &[3]);

    let mut player = PlayerRuntime::connected(2, 0);
    assert!(player.decode_legacy_ppd_blob(&existing));
    assert_eq!(
        player.depot[3]
            .as_ref()
            .expect("slot 3 populated")
            .template_id,
        0x3333
    );

    // Round trip through encode_legacy_ppd_blob: block is rewritten
    // (still present) since `had_depot` was true, and the unrelated
    // block after it is preserved untouched.
    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), DRD_DEPOT_PPD);
    assert_eq!(
        read_u32(&encoded, 8 + MAXDEPOT * DEPOT_PPD_ITEM_SIZE),
        0x5566_7788
    );

    // A character who never had the block and never touched the
    // depot doesn't grow one out of nothing.
    let fresh = PlayerRuntime::connected(3, 0);
    let encoded = fresh.encode_legacy_ppd_blob(&[]);
    assert!(!encoded
        .windows(4)
        .any(|window| window == DRD_DEPOT_PPD.to_le_bytes()));
}

#[test]
fn treasure_dig_ppd_codec_matches_legacy_c_layout() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert!(player.mark_treasure_dig(0, 1234));
    assert!(player.mark_treasure_dig(4, i32::MAX as u64 + 99));

    let bytes = player.encode_legacy_treasure_dig_ppd();
    assert_eq!(bytes.len(), LEGACY_TREASURE_DIG_PPD_SIZE);
    assert_eq!(read_i32(&bytes, 0), 1234);
    assert_eq!(read_i32(&bytes, 4), 0);
    assert_eq!(read_i32(&bytes, 4 * 4), i32::MAX);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_treasure_dig_ppd(&bytes));
    assert_eq!(decoded.treasure_dig_last_seconds(0), 1234);
    assert_eq!(decoded.treasure_dig_last_seconds(1), 0);
    assert_eq!(decoded.treasure_dig_last_seconds(4), i32::MAX as u64);
}

#[test]
fn orbspawn_ppd_codec_matches_legacy_c_layout() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.mark_orb_spawn_used(0x0001_0506, 1234);
    player.mark_orb_spawn_used(0x0001_0708, i32::MAX as u64 + 99);

    let bytes = player.encode_legacy_orbspawn_ppd();
    assert_eq!(bytes.len(), LEGACY_ORBSPAWN_PPD_SIZE);
    assert_eq!(read_i32(&bytes, 0), 0x0001_0506);
    assert_eq!(read_i32(&bytes, 4), 0x0001_0708);
    assert_eq!(read_i32(&bytes, ORBSPAWN_PPD_LAST_USED_OFFSET), 1234);
    assert_eq!(
        read_i32(&bytes, ORBSPAWN_PPD_LAST_USED_OFFSET + 4),
        i32::MAX
    );

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_orbspawn_ppd(&bytes));
    assert_eq!(decoded.orb_spawn_last_used_seconds(0x0001_0506), Some(1234));
    assert_eq!(
        decoded.orb_spawn_last_used_seconds(0x0001_0708),
        Some(i32::MAX as u64)
    );
    assert_eq!(decoded.orb_spawn_last_used_seconds(0x0001_090a), None);
}

#[test]
fn bank_ppd_codec_matches_legacy_c_layout() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.bank_gold = 3_800;

    let encoded = player.encode_legacy_bank_ppd();
    assert_eq!(encoded.len(), LEGACY_BANK_PPD_SIZE);
    assert_eq!(read_i32(&encoded, BANK_PPD_IMPERIAL_GOLD_OFFSET), 3_800);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_bank_ppd(&encoded));
    assert_eq!(decoded.bank_gold, 3_800);
    assert!(!decoded.decode_legacy_bank_ppd(&encoded[..LEGACY_BANK_PPD_SIZE - 1]));
}

#[test]
fn bank_ppd_blob_round_trips_with_legacy_block_framing() {
    let unknown_id = make_drd(DEV_ID_DB, 222 | PERSISTENT_PLAYER_DATA);
    let mut existing_bank = vec![0; LEGACY_BANK_PPD_SIZE];
    write_i32(&mut existing_bank, BANK_PPD_IMPERIAL_GOLD_OFFSET, 500);

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
    write_ppd_block(&mut existing, DRD_BANK_PPD, &existing_bank);

    let mut player = PlayerRuntime::connected(1, 0);
    player.bank_gold = 12_345;

    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), unknown_id);
    assert_eq!(read_u32(&encoded, 12), DRD_BANK_PPD);
    assert_eq!(read_u32(&encoded, 16), LEGACY_BANK_PPD_SIZE as u32);
    assert_eq!(
        read_i32(&encoded, 20 + BANK_PPD_IMPERIAL_GOLD_OFFSET),
        12_345
    );

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.bank_gold, 12_345);

    let mut appended = PlayerRuntime::connected(3, 0);
    appended.bank_gold = 700;
    let appended_blob = appended.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&appended_blob, 0), DRD_BANK_PPD);
    assert_eq!(read_i32(&appended_blob, 8), 700);

    // C: a zero balance is never written out (matches every other
    // "only append if nonzero" PPD block in `encode_legacy_ppd_blob`).
    let zero_balance = PlayerRuntime::connected(4, 0);
    assert!(zero_balance.encode_legacy_ppd_blob(&[]).is_empty());
}

#[test]
fn stats_ppd_update_accumulates_the_current_day_bucket() {
    let mut player = PlayerRuntime::connected(1, 0);
    let day0 = STATS_PPD_STARTTIME; // real_now == 0 -> day index 0
    player.stats_update(1_000, 1, 0, day0);
    player.stats_update(1_050, 1, 50, day0 + 30); // same day, still bucket 0
    assert_eq!(player.stats_online_time(), 2);
    assert_eq!(
        read_i32(
            &player.stats_ppd,
            stats_ppd_day_offset(0) + STATS_PPD_DAY_EXP_OFFSET
        ),
        1_050
    );
    assert_eq!(
        read_i32(
            &player.stats_ppd,
            stats_ppd_day_offset(0) + STATS_PPD_DAY_GOLD_OFFSET
        ),
        50
    );
    assert_eq!(
        read_i32(&player.stats_ppd, STATS_PPD_LAST_UPDATE_OFFSET),
        (day0 + 30 - STATS_PPD_STARTTIME) as i32
    );
}

#[test]
fn stats_ppd_update_advances_to_a_new_day_bucket_without_disturbing_others() {
    let mut player = PlayerRuntime::connected(1, 0);
    let day0 = STATS_PPD_STARTTIME;
    player.stats_update(1_000, 1, 0, day0);
    let next_day = day0 + STATS_PPD_RESOLUTION_SECONDS;
    player.stats_update(1_100, 1, 0, next_day);
    // Both days' `online` samples are summed by `stats_online_time`.
    assert_eq!(player.stats_online_time(), 2);
    assert_eq!(
        read_i32(
            &player.stats_ppd,
            stats_ppd_day_offset(0) + STATS_PPD_DAY_ONLINE_OFFSET
        ),
        1
    );
    assert_eq!(
        read_i32(
            &player.stats_ppd,
            stats_ppd_day_offset(1) + STATS_PPD_DAY_ONLINE_OFFSET
        ),
        1
    );
}

#[test]
fn stats_ppd_update_zeroes_skipped_days_when_resuming_after_a_gap() {
    let mut player = PlayerRuntime::connected(1, 0);
    let day0 = STATS_PPD_STARTTIME;
    player.stats_update(1_000, 5, 0, day0);
    // Resume 3 days later: day 1 and day 2 were skipped and must be
    // zeroed (C's `while (lidx != idx) { ...; bzero(...); }`), day 0's
    // sample is untouched, day 3's sample is freshly written.
    let day3 = day0 + STATS_PPD_RESOLUTION_SECONDS * 3;
    player.stats_update(1_200, 2, 0, day3);
    assert_eq!(
        read_i32(
            &player.stats_ppd,
            stats_ppd_day_offset(0) + STATS_PPD_DAY_ONLINE_OFFSET
        ),
        5
    );
    assert_eq!(
        read_i32(
            &player.stats_ppd,
            stats_ppd_day_offset(1) + STATS_PPD_DAY_ONLINE_OFFSET
        ),
        0
    );
    assert_eq!(
        read_i32(
            &player.stats_ppd,
            stats_ppd_day_offset(2) + STATS_PPD_DAY_ONLINE_OFFSET
        ),
        0
    );
    assert_eq!(
        read_i32(
            &player.stats_ppd,
            stats_ppd_day_offset(3) + STATS_PPD_DAY_ONLINE_OFFSET
        ),
        2
    );
    assert_eq!(player.stats_online_time(), 7);
}

#[test]
fn stats_ppd_update_wrapping_past_bucket_zero_clears_it() {
    // Reproduces C's `while (lidx != idx) { lidx = (lidx+1) % MAXSTAT;
    // bzero(...); }` walking *forward* (with wraparound) from the
    // previous update's day bucket to the new one: a gap that crosses
    // the day-350 -> day-5 boundary clears every skipped bucket in
    // between, including bucket 0, even though bucket 0 was never
    // `idx` on either of these two calls.
    let mut player = PlayerRuntime::connected(1, 0);
    let day0 = STATS_PPD_STARTTIME;
    player.stats_update(1_000, 7, 0, day0); // bucket 0 <- online 7
    let day350 = day0 + STATS_PPD_RESOLUTION_SECONDS * 350;
    player.stats_update(1_050, 9, 0, day350); // walks 1..=350, bucket 0 untouched
    assert_eq!(
        read_i32(
            &player.stats_ppd,
            stats_ppd_day_offset(0) + STATS_PPD_DAY_ONLINE_OFFSET
        ),
        7
    );
    let day5_next_cycle = day350 + STATS_PPD_RESOLUTION_SECONDS * 20; // idx (350+20)%365 == 5
    player.stats_update(1_100, 3, 0, day5_next_cycle); // walks 351..=364,0..=5
    assert_eq!(
        read_i32(
            &player.stats_ppd,
            stats_ppd_day_offset(0) + STATS_PPD_DAY_ONLINE_OFFSET
        ),
        0,
        "bucket 0 must be cleared once the forward walk passes through it"
    );
    assert_eq!(
        read_i32(
            &player.stats_ppd,
            stats_ppd_day_offset(350) + STATS_PPD_DAY_ONLINE_OFFSET
        ),
        9,
        "bucket 350 is never revisited by this second walk"
    );
    assert_eq!(
        read_i32(
            &player.stats_ppd,
            stats_ppd_day_offset(5) + STATS_PPD_DAY_ONLINE_OFFSET
        ),
        3
    );
    assert_eq!(player.stats_online_time(), 9 + 3);
}

#[test]
fn stats_online_time_is_zero_before_any_update() {
    let player = PlayerRuntime::connected(1, 0);
    assert_eq!(player.stats_online_time(), 0);
}

#[test]
fn stats_ppd_codec_matches_legacy_c_layout() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.stats_update(500, 10, 25, STATS_PPD_STARTTIME);

    let encoded = player.encode_legacy_stats_ppd();
    assert_eq!(encoded.len(), LEGACY_STATS_PPD_SIZE);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_stats_ppd(&encoded));
    assert_eq!(decoded.stats_online_time(), 10);
    assert!(!decoded.decode_legacy_stats_ppd(&encoded[..LEGACY_STATS_PPD_SIZE - 1]));
}

#[test]
fn stats_ppd_blob_round_trips_with_legacy_block_framing() {
    let unknown_id = make_drd(DEV_ID_DB, 223 | PERSISTENT_PLAYER_DATA);
    let mut existing = Vec::new();
    write_ppd_block(&mut existing, unknown_id, &[9, 9, 9, 9]);

    let mut player = PlayerRuntime::connected(1, 0);
    player.stats_update(10, 4, 0, STATS_PPD_STARTTIME);

    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), unknown_id);
    assert_eq!(read_u32(&encoded, 12), DRD_STATS_PPD);
    assert_eq!(read_u32(&encoded, 16), LEGACY_STATS_PPD_SIZE as u32);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.stats_online_time(), 4);

    // C: an all-zero (never-updated) stats block is never written out
    // (matches every other "only append if nonempty" PPD block).
    let untouched = PlayerRuntime::connected(3, 0);
    assert!(untouched.encode_legacy_ppd_blob(&[]).is_empty());
}

#[test]
fn orbspawn_ppd_blob_round_trips_with_legacy_block_framing() {
    let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
    let mut existing_orbspawn = vec![0; LEGACY_ORBSPAWN_PPD_SIZE];
    write_i32(&mut existing_orbspawn, ORBSPAWN_PPD_IDS_OFFSET, 0x0001_0203);
    write_i32(&mut existing_orbspawn, ORBSPAWN_PPD_LAST_USED_OFFSET, 44);

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
    write_ppd_block(&mut existing, DRD_ORBSPAWN_PPD, &existing_orbspawn);

    let mut player = PlayerRuntime::connected(1, 0);
    player.mark_orb_spawn_used(0x0001_0506, 777);

    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), unknown_id);
    assert_eq!(read_u32(&encoded, 12), DRD_ORBSPAWN_PPD);
    assert_eq!(read_u32(&encoded, 16), LEGACY_ORBSPAWN_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 20), 0x0001_0506);
    assert_eq!(read_i32(&encoded, 20 + ORBSPAWN_PPD_LAST_USED_OFFSET), 777);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.orb_spawn_last_used_seconds(0x0001_0506), Some(777));
    assert_eq!(decoded.orb_spawn_last_used_seconds(0x0001_0203), None);
}

#[test]
fn ppd_blob_appends_orbspawns_without_existing_block() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.mark_orb_spawn_used(0x0001_0203, 55);

    let encoded = player.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&encoded, 0), DRD_ORBSPAWN_PPD);
    assert_eq!(read_u32(&encoded, 4), LEGACY_ORBSPAWN_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 8), 0x0001_0203);
    assert_eq!(read_i32(&encoded, 8 + ORBSPAWN_PPD_LAST_USED_OFFSET), 55);
}

#[test]
fn xmas_tree_touch_resets_by_event_year_and_blocks_repeats() {
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(
        player.touch_xmas_tree(1, 2025, false, true),
        XmasTreeResult::Dormant
    );
    assert_eq!(
        player.touch_xmas_tree(1, 2025, true, false),
        XmasTreeResult::NeedsHolidayTreat
    );
    assert_eq!(
        player.touch_xmas_tree(1, 2025, true, true),
        XmasTreeResult::GiftGranted
    );
    assert_eq!(
        player.touch_xmas_tree(1, 2025, true, true),
        XmasTreeResult::AlreadyGranted
    );
    assert_eq!(
        player.touch_xmas_tree(1, 2026, true, true),
        XmasTreeResult::GiftGranted
    );
    assert_eq!(read_i32(&player.misc_ppd, MISC_PPD_GIFT_YEAR_OFFSET), 2026);
    assert_eq!(player.misc_ppd[MISC_PPD_TREEDONE_OFFSET], 0b0000_0010);
}

#[test]
fn mark_first_kill_returns_true_only_on_the_first_kill_of_a_class() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert!(player.mark_first_kill(60));
    assert!(!player.mark_first_kill(60));
    // A different class in the same byte-adjacent word is unaffected.
    assert!(player.mark_first_kill(61));
    // Out-of-range classes (C: `< 1 || > 1023`) never record anything.
    assert!(!player.mark_first_kill(0));
    assert!(!player.mark_first_kill(1024));
    assert!(!player.mark_first_kill(-1));
}

#[test]
fn count_demon_lord_kills_only_counts_the_c_class_ranges() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(player.count_demon_lord_kills(), 0);
    for class in [258, 305, 404, 411] {
        assert!(player.mark_first_kill(class));
    }
    assert_eq!(player.count_demon_lord_kills(), 4);
    // Classes outside 258..=305 / 404..=411 (e.g. a regular pentagram
    // demon) never contribute to the count.
    assert!(player.mark_first_kill(60));
    assert_eq!(player.count_demon_lord_kills(), 4);
}

#[test]
fn firstkill_ppd_blob_round_trips_through_encode_decode() {
    let mut player = PlayerRuntime::connected(1, 0);
    for class in [60, 258, 404] {
        player.mark_first_kill(class);
    }

    let encoded = player.encode_legacy_ppd_blob(&[]);
    let mut round_tripped = PlayerRuntime::connected(1, 0);
    assert!(round_tripped.decode_legacy_ppd_blob(&encoded));
    assert_eq!(round_tripped.first_kill_ppd, player.first_kill_ppd);
    assert_eq!(round_tripped.count_demon_lord_kills(), 2);
    assert!(!round_tripped.mark_first_kill(60));
    assert!(!round_tripped.mark_first_kill(258));
    assert!(round_tripped.mark_first_kill(61));
}

#[test]
fn flower_ppd_codec_matches_legacy_fixed_arrays() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.mark_flower_used(0x001f_2030, 1234);
    player.mark_flower_used(0x001f_2031, 5678);

    let encoded = player.encode_legacy_flower_ppd();

    assert_eq!(encoded.len(), LEGACY_FLOWER_PPD_SIZE);
    assert_eq!(read_i32(&encoded, FLOWER_PPD_IDS_OFFSET), 0x001f_2030);
    assert_eq!(read_i32(&encoded, FLOWER_PPD_IDS_OFFSET + 4), 0x001f_2031);
    assert_eq!(read_i32(&encoded, FLOWER_PPD_LAST_USED_OFFSET), 1234);
    assert_eq!(read_i32(&encoded, FLOWER_PPD_LAST_USED_OFFSET + 4), 5678);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_flower_ppd(&encoded));
    assert_eq!(decoded.flower_last_used_seconds(0x001f_2030), Some(1234));
    assert_eq!(decoded.flower_last_used_seconds(0x001f_2031), Some(5678));
}

#[test]
fn flower_ppd_blob_replaces_and_appends_legacy_block() {
    let mut existing = Vec::new();
    write_ppd_block(&mut existing, 0x1122_3344, &[1, 2, 3]);

    let mut player = PlayerRuntime::connected(1, 0);
    player.mark_flower_used(7, 99);
    let encoded = player.encode_legacy_ppd_blob(&existing);

    assert_eq!(read_u32(&encoded, 0), 0x1122_3344);
    assert_eq!(read_u32(&encoded, 11), DRD_FLOWER_PPD);
    assert_eq!(read_u32(&encoded, 15), LEGACY_FLOWER_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 19), 7);
    assert_eq!(read_i32(&encoded, 19 + FLOWER_PPD_LAST_USED_OFFSET), 99);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.flower_last_used_seconds(7), Some(99));
}

#[test]
fn bone_hint_uses_generated_special_exec_digit() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.rune_special_exec[0] = 511;
    player.rune_special_exec[(7 - 5) * 5 + 2] = 731;

    assert_eq!(
        player.bone_hint(7, 2, 1, |_| 0),
        BoneHintResult::Hint {
            page: 72,
            rune: "Dagaz",
            position: "second",
        }
    );
}
