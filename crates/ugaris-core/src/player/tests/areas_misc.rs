use super::*;

#[test]
fn saltmine_ladder_cooldown_tracks_legacy_reuse_window() {
    let mut player = PlayerRuntime::connected(1, 0);

    assert!(player.saltmine_ladder_ready(3, 1_000));
    assert!(player.mark_saltmine_ladder_used(3, 1_000));
    assert!(!player.saltmine_ladder_ready(3, 1_000 + 60 * 60 * 24 - 1));
    assert!(player.saltmine_ladder_ready(3, 1_000 + 60 * 60 * 24));
    assert!(!player.mark_saltmine_ladder_used(20, 1_000));
}

#[test]
fn saltmine_ppd_layout_matches_c_struct() {
    assert_eq!(DEV_ID_MR, 2);
    assert_eq!(
        DRD_SALTMINE_PPD,
        make_drd(DEV_ID_MR, 13 | PERSISTENT_PLAYER_DATA)
    );
    assert_eq!(LEGACY_SALTMINE_PPD_SIZE, 88);

    let mut player = PlayerRuntime::connected(1, 0);
    player.saltmine_ladder_last_seconds[0] = 123;
    player.saltmine_ladder_last_seconds[19] = 456;
    player.saltmine_pending_salt = 7;

    let bytes = player.encode_legacy_saltmine_ppd();
    assert_eq!(bytes.len(), LEGACY_SALTMINE_PPD_SIZE);
    assert_eq!(bytes[0], LEGACY_SALTMINE_PPD_VERSION);
    assert_eq!(&bytes[1..4], &[0, 0, 0]);
    assert_eq!(read_i32(&bytes, 4), 123);
    assert_eq!(read_i32(&bytes, 4 + 19 * 4), 456);
    assert_eq!(read_i32(&bytes, 4 + SALTMINE_LADDER_COUNT * 4), 7);

    let mut decoded = PlayerRuntime::connected(1, 0);
    assert!(decoded.decode_legacy_saltmine_ppd(&bytes));
    assert_eq!(decoded.saltmine_ladder_last_seconds[0], 123);
    assert_eq!(decoded.saltmine_ladder_last_seconds[19], 456);
    assert_eq!(decoded.saltmine_pending_salt, 7);
}

#[test]
fn saltmine_ppd_version_mismatch_resets_like_c_set_data() {
    let mut bytes = vec![0; LEGACY_SALTMINE_PPD_SIZE];
    bytes[0] = LEGACY_SALTMINE_PPD_VERSION + 1;
    write_i32(&mut bytes, 4, 123);
    write_i32(&mut bytes, 4 + SALTMINE_LADDER_COUNT * 4, 7);

    let mut player = PlayerRuntime::connected(1, 0);
    player.saltmine_ladder_last_seconds[0] = 1;
    player.saltmine_pending_salt = 1;
    assert!(player.decode_legacy_saltmine_ppd(&bytes));

    assert_eq!(
        player.saltmine_ladder_last_seconds,
        [0; SALTMINE_LADDER_COUNT]
    );
    assert_eq!(player.saltmine_pending_salt, 0);
}

#[test]
fn ppd_blob_replaces_and_appends_saltmine_block() {
    let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
    let mut existing_saltmine = vec![0; LEGACY_SALTMINE_PPD_SIZE];
    existing_saltmine[0] = LEGACY_SALTMINE_PPD_VERSION;
    write_i32(&mut existing_saltmine, 4, 11);
    write_i32(&mut existing_saltmine, 4 + SALTMINE_LADDER_COUNT * 4, 2);
    let mut existing = Vec::new();
    write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
    write_ppd_block(&mut existing, DRD_SALTMINE_PPD, &existing_saltmine);

    let mut player = PlayerRuntime::connected(1, 0);
    assert!(player.decode_legacy_ppd_blob(&existing));
    assert_eq!(player.saltmine_ladder_last_seconds[0], 11);
    assert_eq!(player.saltmine_pending_salt, 2);
    player.saltmine_ladder_last_seconds[0] = 99;
    player.saltmine_pending_salt = 5;

    let encoded = player.encode_legacy_ppd_blob(&existing);
    let blocks: Vec<_> = LegacyPpdBlocks::parse(&encoded)
        .map(|block| block.unwrap())
        .collect();
    assert_eq!(blocks[0].id, unknown_id);
    assert_eq!(blocks[0].data, &[1, 2, 3, 4]);
    assert_eq!(blocks[1].id, DRD_SALTMINE_PPD);
    assert_eq!(read_i32(blocks[1].data, 4), 99);
    assert_eq!(read_i32(blocks[1].data, 4 + SALTMINE_LADDER_COUNT * 4), 5);

    let mut append_player = PlayerRuntime::connected(1, 0);
    append_player.saltmine_ladder_last_seconds[3] = 77;
    let appended = append_player.encode_legacy_ppd_blob(&[]);
    let blocks: Vec<_> = LegacyPpdBlocks::parse(&appended)
        .map(|block| block.unwrap())
        .collect();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].id, DRD_SALTMINE_PPD);
    assert_eq!(read_i32(blocks[0].data, 4 + 3 * 4), 77);
}

#[test]
fn clear_turn_seyan_ppd_resets_every_typed_field_it_covers() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.chest_last_access_seconds.insert(2, 12345);
    player.area3_ppd = vec![1, 2, 3];
    player.area1_ppd = vec![1, 2, 3];
    player.nomad_ppd = vec![4, 5, 6];
    player.random_shrine_used_words[0] = 7;
    player.random_shrine_continuity = 9;
    player.flowers.push(FlowerAccess {
        location_id: 1,
        last_used_seconds: 1,
    });
    player.random_chests.push(RandomChestAccess {
        location_id: 1,
        last_used_seconds: 1,
    });
    player.demonshrines.push(42);
    player.farmy_ppd = vec![4, 5, 6];
    player.twocity_ppd = vec![7, 8, 9];
    player.twocity_goodtile = [1, 2, 3, 4, 5];
    player.twocity_solved_library = true;
    player.orb_spawns.push(OrbSpawnAccess {
        location_id: 1,
        last_used_seconds: 1,
    });
    player.rune_used_words[0] = 3;
    player.rune_special_exec[0] = 11;
    player.lab_solved_bits = 0xFF;
    player.lab_ppd = vec![10, 11];
    player.rat_chests.push(RatChestAccess {
        location_id: 1,
        last_used_seconds: 1,
    });
    player.rat_chest_treasure_x = 5;
    player.rat_chest_treasure_y = 6;
    player.rat_chest_last_treasure_seconds = 100;
    player.staffer_ppd = vec![12, 13];
    player.arkhata_ppd = vec![14, 15];

    player.clear_turn_seyan_ppd();

    assert!(player.chest_last_access_seconds.is_empty());
    assert!(player.area3_ppd.is_empty());
    assert!(player.area1_ppd.is_empty());
    assert!(player.nomad_ppd.is_empty());
    assert_eq!(
        player.random_shrine_used_words,
        [0; RANDOMSHRINE_USED_WORDS]
    );
    assert_eq!(player.random_shrine_continuity, 0);
    assert!(player.flowers.is_empty());
    assert!(player.random_chests.is_empty());
    assert!(player.demonshrines.is_empty());
    assert!(player.farmy_ppd.is_empty());
    assert!(player.twocity_ppd.is_empty());
    assert_eq!(player.twocity_goodtile, [0; 5]);
    assert!(!player.twocity_solved_library);
    assert!(player.orb_spawns.is_empty());
    assert_eq!(player.rune_used_words, [0; RUNE_USED_WORDS]);
    assert_eq!(player.rune_special_exec, [0; RUNE_SPECIAL_EXEC_COUNT]);
    assert_eq!(player.lab_solved_bits, 0);
    assert!(player.lab_ppd.is_empty());
    assert!(player.rat_chests.is_empty());
    assert_eq!(player.rat_chest_treasure_x, 0);
    assert_eq!(player.rat_chest_treasure_y, 0);
    assert_eq!(player.rat_chest_last_treasure_seconds, 0);
    assert!(player.staffer_ppd.is_empty());
    assert!(player.arkhata_ppd.is_empty());
}

#[test]
fn clear_turn_seyan_ppd_strips_unmapped_ids_but_keeps_other_raw_blocks() {
    let unrelated_unknown_id = make_drd(DEV_ID_DB, 999 | PERSISTENT_PLAYER_DATA);
    let mut existing = Vec::new();
    write_ppd_block(&mut existing, DRD_RANK_PPD, &[1, 2, 3, 4]);
    write_ppd_block(&mut existing, DRD_DEPOT_PPD, &[5, 6]);
    write_ppd_block(&mut existing, unrelated_unknown_id, &[7, 8, 9]);

    let mut player = PlayerRuntime::connected(1, 0);
    player.ppd_blob = existing;

    player.clear_turn_seyan_ppd();

    let blocks: Vec<_> = LegacyPpdBlocks::parse(&player.ppd_blob)
        .map(|block| block.unwrap())
        .collect();
    // `DRD_RANK_PPD` is one of `turn_seyan`'s del_data targets and
    // is gone; `DRD_DEPOT_PPD` (mutated via the typed `self.depot`
    // field on decode/encode, not this raw-bytes `ppd_blob` strip
    // path - `self.depot` was never populated here since
    // `decode_legacy_ppd_blob` was never called) and any other
    // still-unrelated id round-trip untouched in the raw blob.
    assert!(!blocks.iter().any(|block| block.id == DRD_RANK_PPD));
    assert!(blocks.iter().any(|block| block.id == DRD_DEPOT_PPD));
    assert!(blocks
        .iter()
        .any(|block| block.id == unrelated_unknown_id && block.data == [7, 8, 9]));
}

#[test]
fn clear_turn_seyan_ppd_clears_first_kill_bitmask() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert!(player.mark_first_kill(60));
    assert!(!player.first_kill_ppd.is_empty());

    player.clear_turn_seyan_ppd();

    assert!(player.first_kill_ppd.is_empty());
    // Re-encoding after the clear should not emit a
    // `DRD_FIRSTKILL_PPD` block at all (matches the same "empty
    // means omitted" convention every other typed PPD in this file
    // follows).
    let encoded = player.encode_legacy_ppd_blob(&[]);
    assert!(!LegacyPpdBlocks::parse(&encoded)
        .map(|block| block.unwrap())
        .any(|block| block.id == DRD_FIRSTKILL_PPD));
}

#[test]
fn farmy_ppd_round_trips_through_outer_blob() {
    let mut farmy = vec![0; LEGACY_FARMY_PPD_SIZE];
    write_i32(&mut farmy, FARMY_PPD_BOSS_STAGE_OFFSET, 19);
    let mut blob = Vec::new();
    write_ppd_block(&mut blob, DRD_FARMY_PPD, &farmy);

    let mut player = PlayerRuntime::connected(1, 0);
    assert!(player.decode_legacy_ppd_blob(&blob));
    assert_eq!(player.farmy_boss_stage(), 19);

    assert!(player.advance_farmy_blood_stage());
    let encoded = player.encode_legacy_ppd_blob(&blob);
    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.farmy_boss_stage(), 21);
}

#[test]
fn farmy_boss_stage_timer_counter_reported_accessors_round_trip() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(player.farmy_boss_stage(), 0);
    assert_eq!(player.farmy_boss_timer(), 0);
    assert_eq!(player.farmy_boss_counter(), 0);
    assert_eq!(player.farmy_boss_reported(), 0);

    player.set_farmy_boss_stage(5);
    player.set_farmy_boss_timer(1_700_000);
    player.set_farmy_boss_counter(0b1011);
    player.set_farmy_boss_reported(0b0011);

    assert_eq!(player.farmy_boss_stage(), 5);
    assert_eq!(player.farmy_boss_timer(), 1_700_000);
    assert_eq!(player.farmy_boss_counter(), 0b1011);
    assert_eq!(player.farmy_boss_reported(), 0b0011);

    // Round-trips through the outer legacy blob, matching the layout
    // `PORTING_LEDGER.md` records: `boss_stage`@0, `boss_timer`@4,
    // `soldier[3]`@8..332, `boss_counter`@332, `boss_reported`@336.
    let encoded = player.encode_legacy_ppd_blob(&[]);
    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.farmy_boss_stage(), 5);
    assert_eq!(decoded.farmy_boss_timer(), 1_700_000);
    assert_eq!(decoded.farmy_boss_counter(), 0b1011);
    assert_eq!(decoded.farmy_boss_reported(), 0b0011);
}

#[test]
fn farmy_soldier_slot_accessors_round_trip_and_do_not_clobber_neighbors() {
    let mut player = PlayerRuntime::connected(1, 0);
    for slot in 0..3 {
        assert_eq!(player.farmy_soldier_type(slot), 0);
        assert_eq!(player.farmy_soldier_rank(slot), 0);
        assert_eq!(player.farmy_soldier_base(slot), 0);
        assert_eq!(player.farmy_soldier_profile(slot), 0);
        assert_eq!(player.farmy_soldier_exp(slot), 0);
        assert_eq!(player.farmy_soldier_cn(slot), 0);
        assert_eq!(player.farmy_soldier_serial(slot), 0);
    }

    player.set_farmy_soldier_type(0, 2);
    player.set_farmy_soldier_rank(0, 1);
    player.set_farmy_soldier_base(0, 47);
    player.set_farmy_soldier_profile(0, 9);
    player.set_farmy_soldier_exp(0, 12_345);
    player.set_farmy_soldier_cn(0, 555);
    player.set_farmy_soldier_serial(0, 42);

    player.set_farmy_soldier_type(2, 1);
    player.set_farmy_soldier_profile(2, 3);

    // Slot 0's fields don't bleed into slot 1 or slot 2.
    assert_eq!(player.farmy_soldier_type(0), 2);
    assert_eq!(player.farmy_soldier_rank(0), 1);
    assert_eq!(player.farmy_soldier_base(0), 47);
    assert_eq!(player.farmy_soldier_profile(0), 9);
    assert_eq!(player.farmy_soldier_exp(0), 12_345);
    assert_eq!(player.farmy_soldier_cn(0), 555);
    assert_eq!(player.farmy_soldier_serial(0), 42);
    assert_eq!(player.farmy_soldier_type(1), 0);
    assert_eq!(player.farmy_soldier_profile(1), 0);
    assert_eq!(player.farmy_soldier_type(2), 1);
    assert_eq!(player.farmy_soldier_profile(2), 3);

    // Out-of-range slots read as 0 and writes are a documented no-op.
    assert_eq!(player.farmy_soldier_type(3), 0);
    player.set_farmy_soldier_type(3, 99);
    assert_eq!(player.farmy_soldier_type(3), 0);

    // The soldier array sits strictly between boss_timer and boss_counter,
    // so writing soldier fields never disturbs the already-ported boss
    // fields, and vice versa.
    player.set_farmy_boss_stage(7);
    player.set_farmy_boss_counter(11);
    assert_eq!(player.farmy_soldier_type(0), 2);
    assert_eq!(player.farmy_boss_stage(), 7);
    assert_eq!(player.farmy_boss_counter(), 11);

    // Round-trips through the outer legacy blob.
    let encoded = player.encode_legacy_ppd_blob(&[]);
    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.farmy_soldier_type(0), 2);
    assert_eq!(decoded.farmy_soldier_exp(0), 12_345);
    assert_eq!(decoded.farmy_soldier_cn(0), 555);
    assert_eq!(decoded.farmy_soldier_serial(0), 42);
    assert_eq!(decoded.farmy_soldier_profile(2), 3);
    assert_eq!(decoded.farmy_boss_stage(), 7);
    assert_eq!(decoded.farmy_boss_counter(), 11);
}

#[test]
fn farmy_soldier_emote_round_trips_and_does_not_clobber_neighbors_or_prefix() {
    use crate::world::npc::area8::fdemon_army_emote::SoldierEmote;

    let mut player = PlayerRuntime::connected(1, 0);
    for slot in 0..3 {
        assert_eq!(player.farmy_soldier_emote(slot), SoldierEmote::default());
    }

    let emote0 = SoldierEmote {
        cuddly: 20,
        lonely: 3,
        angst: 10,
        fear: 4,
        bore: 20,
        boredom: 5,
        bigmouth: 5,
        praise: 6,
        likes: [1, 2, 3, 4],
        talked: [5, 6, 7, 8],
        answer_timer: 1_700_000,
        answer_cn: 55,
        answer_type: 2,
        last_emote: 1_600_000,
    };
    player.set_farmy_soldier_emote(0, &emote0);
    // Slot-0-prefix fields (type/rank/base/...) and slot 2's emote must stay
    // untouched by writing slot 0's emote.
    player.set_farmy_soldier_type(0, 2);
    player.set_farmy_soldier_cn(0, 555);
    let emote2 = SoldierEmote {
        cuddly: 1,
        ..SoldierEmote::default()
    };
    player.set_farmy_soldier_emote(2, &emote2);

    assert_eq!(player.farmy_soldier_emote(0), emote0);
    assert_eq!(player.farmy_soldier_type(0), 2);
    assert_eq!(player.farmy_soldier_cn(0), 555);
    assert_eq!(player.farmy_soldier_emote(1), SoldierEmote::default());
    assert_eq!(player.farmy_soldier_emote(2), emote2);

    // Out-of-range slots read as default and writes are a documented no-op.
    assert_eq!(player.farmy_soldier_emote(3), SoldierEmote::default());
    player.set_farmy_soldier_emote(3, &emote0);
    assert_eq!(player.farmy_soldier_emote(3), SoldierEmote::default());

    // Round-trips through the outer legacy blob.
    let encoded = player.encode_legacy_ppd_blob(&[]);
    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.farmy_soldier_emote(0), emote0);
    assert_eq!(decoded.farmy_soldier_emote(2), emote2);
}

#[test]
fn teufelrat_ppd_codec_matches_legacy_rat_data_layout() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(player.add_teufel_rat_kill(80, false), (1, 64));
    assert_eq!(player.add_teufel_rat_kill(90, true), (2, 65));

    let encoded = player.encode_legacy_teufelrat_ppd();
    assert_eq!(encoded.len(), LEGACY_TEUFELRAT_PPD_SIZE);
    assert_eq!(read_i32(&encoded, TEUFELRAT_PPD_KILLS_OFFSET), 2);
    assert_eq!(read_i32(&encoded, TEUFELRAT_PPD_SCORE_OFFSET), 65);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_teufelrat_ppd(&encoded));
    assert_eq!(decoded.teufel_rat_kills, 2);
    assert_eq!(decoded.teufel_rat_score, 65);
    assert!(!decoded.decode_legacy_teufelrat_ppd(&encoded[..LEGACY_TEUFELRAT_PPD_SIZE - 1]));
}

#[test]
fn teufelrat_ppd_blob_round_trips_with_legacy_block_framing() {
    let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
    let mut existing_rat = vec![0; LEGACY_TEUFELRAT_PPD_SIZE];
    write_i32(&mut existing_rat, TEUFELRAT_PPD_KILLS_OFFSET, 5);
    write_i32(&mut existing_rat, TEUFELRAT_PPD_SCORE_OFFSET, 55);

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
    write_ppd_block(&mut existing, DRD_TEUFELRAT_PPD, &existing_rat);

    let mut player = PlayerRuntime::connected(1, 0);
    player.teufel_rat_kills = 7;
    player.teufel_rat_score = 99;

    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), unknown_id);
    assert_eq!(read_u32(&encoded, 12), DRD_TEUFELRAT_PPD);
    assert_eq!(read_u32(&encoded, 16), LEGACY_TEUFELRAT_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 20 + TEUFELRAT_PPD_KILLS_OFFSET), 7);
    assert_eq!(read_i32(&encoded, 20 + TEUFELRAT_PPD_SCORE_OFFSET), 99);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.teufel_rat_kills, 7);
    assert_eq!(decoded.teufel_rat_score, 99);

    let mut appended = PlayerRuntime::connected(3, 0);
    appended.teufel_rat_kills = 1;
    appended.teufel_rat_score = 1;
    let appended_blob = appended.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&appended_blob, 0), DRD_TEUFELRAT_PPD);
    assert_eq!(read_i32(&appended_blob, 8), 1);
    assert_eq!(read_i32(&appended_blob, 12), 1);
}

#[test]
fn warp_ppd_fixed_layout_round_trips() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.warp_base = 55;
    player.warp_points = 7;
    player.warp_bonus_ids[0] = 0x0019_0203;
    player.warp_bonus_ids[49] = 0x0019_0405;
    player.warp_bonus_last_used[0] = 40;
    player.warp_bonus_last_used[49] = 50;
    player.warp_nostepexp = 1;

    let encoded = player.encode_legacy_warp_ppd();
    assert_eq!(encoded.len(), LEGACY_WARP_PPD_SIZE);
    assert_eq!(read_i32(&encoded, WARP_PPD_BASE_OFFSET), 55);
    assert_eq!(read_i32(&encoded, WARP_PPD_POINTS_OFFSET), 7);
    assert_eq!(read_i32(&encoded, WARP_PPD_BONUS_ID_OFFSET), 0x0019_0203);
    assert_eq!(
        read_i32(&encoded, WARP_PPD_BONUS_ID_OFFSET + 49 * 4),
        0x0019_0405
    );
    assert_eq!(read_i32(&encoded, WARP_PPD_BONUS_LAST_USED_OFFSET), 40);
    assert_eq!(
        read_i32(&encoded, WARP_PPD_BONUS_LAST_USED_OFFSET + 49 * 4),
        50
    );
    assert_eq!(read_i32(&encoded, WARP_PPD_NOSTEPEXP_OFFSET), 1);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_warp_ppd(&encoded));
    assert_eq!(decoded.warp_base, 55);
    assert_eq!(decoded.warp_points, 7);
    assert_eq!(decoded.warp_bonus_ids[0], 0x0019_0203);
    assert_eq!(decoded.warp_bonus_ids[49], 0x0019_0405);
    assert_eq!(decoded.warp_bonus_last_used[0], 40);
    assert_eq!(decoded.warp_bonus_last_used[49], 50);
    assert_eq!(decoded.warp_nostepexp, 1);
}

#[test]
fn warp_ppd_blob_round_trips_with_legacy_block_framing() {
    let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
    let mut existing_warp = vec![0; LEGACY_WARP_PPD_SIZE];
    write_i32(&mut existing_warp, WARP_PPD_BASE_OFFSET, 40);
    write_i32(&mut existing_warp, WARP_PPD_BONUS_ID_OFFSET, 0x0019_0101);

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
    write_ppd_block(&mut existing, DRD_WARP_PPD, &existing_warp);

    let mut player = PlayerRuntime::connected(1, 0);
    player.warp_base = 60;
    player.warp_points = 3;
    player.warp_bonus_ids[1] = 0x0019_0203;
    player.warp_bonus_last_used[1] = 55;

    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), unknown_id);
    assert_eq!(read_u32(&encoded, 12), DRD_WARP_PPD);
    assert_eq!(read_u32(&encoded, 16), LEGACY_WARP_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 20 + WARP_PPD_BASE_OFFSET), 60);
    assert_eq!(read_i32(&encoded, 20 + WARP_PPD_POINTS_OFFSET), 3);
    assert_eq!(read_i32(&encoded, 20 + WARP_PPD_BONUS_ID_OFFSET), 0);
    assert_eq!(
        read_i32(&encoded, 20 + WARP_PPD_BONUS_ID_OFFSET + 4),
        0x0019_0203
    );
    assert_eq!(
        read_i32(&encoded, 20 + WARP_PPD_BONUS_LAST_USED_OFFSET + 4),
        55
    );

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.warp_base, 60);
    assert_eq!(decoded.warp_points, 3);
    assert_eq!(decoded.warp_bonus_ids[1], 0x0019_0203);
    assert_eq!(decoded.warp_bonus_last_used[1], 55);
}

#[test]
fn ppd_blob_appends_warp_without_existing_block() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.warp_base = 40;

    let encoded = player.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&encoded, 0), DRD_WARP_PPD);
    assert_eq!(read_u32(&encoded, 4), LEGACY_WARP_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 8 + WARP_PPD_BASE_OFFSET), 40);
}

#[test]
fn gate_ppd_fixed_layout_round_trips() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.gate_welcome_state = 3;
    player.gate_target_class = 7;
    player.gate_step = 1;

    let encoded = player.encode_legacy_gate_ppd();
    assert_eq!(encoded.len(), LEGACY_GATE_PPD_SIZE);
    assert_eq!(read_i32(&encoded, GATE_PPD_WELCOME_STATE_OFFSET), 3);
    assert_eq!(read_i32(&encoded, GATE_PPD_TARGET_CLASS_OFFSET), 7);
    assert_eq!(read_i32(&encoded, GATE_PPD_STEP_OFFSET), 1);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_gate_ppd(&encoded));
    assert_eq!(decoded.gate_welcome_state, 3);
    assert_eq!(decoded.gate_target_class, 7);
    assert_eq!(decoded.gate_step, 1);
    assert!(!decoded.decode_legacy_gate_ppd(&encoded[..7]));
}

#[test]
fn gate_ppd_blob_round_trips_with_legacy_block_framing() {
    let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
    let mut existing_gate = vec![0; LEGACY_GATE_PPD_SIZE];
    write_i32(&mut existing_gate, GATE_PPD_WELCOME_STATE_OFFSET, 2);

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
    write_ppd_block(&mut existing, DRD_GATE_PPD, &existing_gate);

    let mut player = PlayerRuntime::connected(1, 0);
    player.gate_welcome_state = 6;
    player.gate_target_class = 8;
    player.gate_step = 1;

    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), unknown_id);
    assert_eq!(read_u32(&encoded, 12), DRD_GATE_PPD);
    assert_eq!(read_u32(&encoded, 16), LEGACY_GATE_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 20 + GATE_PPD_WELCOME_STATE_OFFSET), 6);
    assert_eq!(read_i32(&encoded, 20 + GATE_PPD_TARGET_CLASS_OFFSET), 8);
    assert_eq!(read_i32(&encoded, 20 + GATE_PPD_STEP_OFFSET), 1);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.gate_welcome_state, 6);
    assert_eq!(decoded.gate_target_class, 8);
    assert_eq!(decoded.gate_step, 1);
}

#[test]
fn ppd_blob_appends_gate_without_existing_block() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.gate_welcome_state = 1;

    let encoded = player.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&encoded, 0), DRD_GATE_PPD);
    assert_eq!(read_u32(&encoded, 4), LEGACY_GATE_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 8 + GATE_PPD_WELCOME_STATE_OFFSET), 1);
}

#[test]
fn rune_special_exec_generation_matches_legacy_constraints() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut seed = 0_u32;
    player.ensure_rune_special_execs(|limit| {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        seed % limit
    });

    for level in 5..10_u32 {
        let base = (level - 5) as usize * 5;
        let mut seen = Vec::new();
        for value in player.rune_special_exec[base..base + 5].iter().copied() {
            assert!(value >= 100);
            assert!(![555, 55, 5, 666, 66, 6, 777, 77, 7, 888, 88, 8, 999, 99, 9].contains(&value));
            let digits = format!("{value:03}");
            assert!(digits
                .chars()
                .all(|ch| ch != '0' && ch <= char::from_digit(level, 10).unwrap()));
            assert!(digits
                .chars()
                .any(|ch| ch == char::from_digit(level, 10).unwrap()));
            assert!(!seen.contains(&value));
            seen.push(value);
        }
    }
}

#[test]
fn rune_ppd_blob_replaces_and_appends_legacy_block() {
    let mut existing_rune = vec![0; LEGACY_RUNE_PPD_SIZE];
    write_u32(&mut existing_rune, 0, 0x8000_0001);
    write_i32(&mut existing_rune, RUNE_PPD_SPECIAL_EXEC_OFFSET, 555);

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, 0x1122_3344, &[1, 2, 3]);
    write_ppd_block(&mut existing, DRD_RUNE_PPD, &existing_rune);

    let mut player = PlayerRuntime::connected(1, 0);
    player.rune_used_words[0] = 0x8000_0002;
    player.rune_special_exec[0] = 654;
    let encoded = player.encode_legacy_ppd_blob(&existing);

    assert_eq!(read_u32(&encoded, 0), 0x1122_3344);
    assert_eq!(read_u32(&encoded, 11), DRD_RUNE_PPD);
    assert_eq!(read_u32(&encoded, 15), LEGACY_RUNE_PPD_SIZE as u32);
    assert_eq!(read_u32(&encoded, 19), 0x8000_0002);
    assert_eq!(read_i32(&encoded, 19 + RUNE_PPD_SPECIAL_EXEC_OFFSET), 654);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.rune_used_words[0], 0x8000_0002);
    assert_eq!(decoded.rune_special_exec[0], 654);
}

#[test]
fn nomad_ppd_codec_matches_legacy_c_layout() {
    assert_eq!(
        DRD_NOMAD_PPD,
        make_drd(DEV_ID_DB, 112 | PERSISTENT_PLAYER_DATA)
    );
    assert_eq!(LEGACY_NOMAD_PPD_SIZE, 100);

    let mut player = PlayerRuntime::connected(1, 0);
    player.set_nomad_state(1, 9);
    player.set_nomad_state(4, 4);
    player.set_nomad_state(5, 2);
    player.set_nomad_win(1, 3);

    let encoded = player.encode_legacy_nomad_ppd();
    assert_eq!(encoded.len(), LEGACY_NOMAD_PPD_SIZE);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_nomad_ppd(&encoded));
    assert_eq!(decoded.nomad_state(1), 9);
    assert_eq!(decoded.nomad_state(4), 4);
    assert_eq!(decoded.nomad_state(5), 2);
    assert_eq!(decoded.nomad_win(1), 3);
    assert_eq!(decoded.nomad_state(9), 0);
    // Out-of-range indices are ignored/read as 0, never panic.
    assert_eq!(decoded.nomad_state(10), 0);
    decoded.set_nomad_state(10, 42);
    assert_eq!(decoded.nomad_state(10), 0);

    let state = decoded.nomad_quest_state();
    assert_eq!(state.nomad_state[1], 9);
    assert_eq!(state.nomad_state[4], 4);
    assert_eq!(state.nomad_state[5], 2);
}

#[test]
fn nomad_ppd_blob_replaces_and_appends_legacy_block() {
    let mut existing_nomad = vec![0; LEGACY_NOMAD_PPD_SIZE];
    write_i32(&mut existing_nomad, NOMAD_PPD_STATE_OFFSET + 5 * 4, 1);

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, 0x2233_4455, &[9, 8, 7]);
    write_ppd_block(&mut existing, DRD_NOMAD_PPD, &existing_nomad);

    let mut player = PlayerRuntime::connected(1, 0);
    player.set_nomad_state(5, 4);
    let encoded = player.encode_legacy_ppd_blob(&existing);

    assert_eq!(read_u32(&encoded, 0), 0x2233_4455);
    assert_eq!(read_u32(&encoded, 11), DRD_NOMAD_PPD);
    assert_eq!(read_u32(&encoded, 15), LEGACY_NOMAD_PPD_SIZE as u32);
    assert_eq!(read_i32(&encoded, 19 + NOMAD_PPD_STATE_OFFSET + 5 * 4), 4);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&encoded));
    assert_eq!(decoded.nomad_state(5), 4);

    let appended = player.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&appended, 0), DRD_NOMAD_PPD);
}

#[test]
fn caligar_ppd_tracks_training_observations() {
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(player.observe_caligar_training(2), Some(true));
    assert_eq!(player.observe_caligar_training(2), Some(false));
    assert_eq!(player.observe_caligar_training(3), Some(true));
    assert_eq!(player.observe_caligar_training(4), None);

    let encoded = player.encode_legacy_caligar_ppd();
    assert_eq!(encoded.len(), LEGACY_CALIGAR_PPD_SIZE);
    assert_eq!(read_i32(&encoded, CALIGAR_PPD_WATCH_FLAG_OFFSET), 6);

    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_caligar_ppd(&encoded));
    assert_eq!(decoded.observe_caligar_training(1), Some(true));
    assert_eq!(decoded.observe_caligar_training(3), Some(false));
}

#[test]
fn arkhata_ppd_round_trips_clerk_timer_fields() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(
        DRD_ARKHATA_PPD,
        make_drd(DEV_ID_DB, 160 | PERSISTENT_PLAYER_DATA)
    );

    player.set_arkhata_clerk_timer(5, 12_345);
    let encoded = player.encode_legacy_arkhata_ppd();
    assert_eq!(encoded.len(), LEGACY_ARKHATA_PPD_SIZE);
    assert_eq!(read_i32(&encoded, ARKHATA_PPD_CLERK_STATE_OFFSET), 5);
    assert_eq!(read_i32(&encoded, ARKHATA_PPD_CLERK_TIME_OFFSET), 12_345);

    let mut existing = Vec::new();
    write_ppd_block(&mut existing, 0x1122_3344, &[1, 2, 3]);
    let blob = player.encode_legacy_ppd_blob(&existing);
    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_ppd_blob(&blob));
    assert_eq!(decoded.arkhata_clerk_state(), 5);
    assert_eq!(decoded.arkhata_clerk_time_seconds(), 12_345);
}

#[test]
fn arkhata_ppd_exposes_rammy_state_read_only_independently_of_clerk_state() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(player.arkhata_rammy_state(), 0);

    player.set_arkhata_clerk_timer(5, 12_345);
    // `arkhata_ppd` is a raw blob with no `set_arkhata_rammy_state` writer
    // (see `ARKHATA_PPD_RAMMY_STATE_OFFSET`'s doc comment) - write the
    // field directly to simulate area 37's still-unported `rammy_driver`
    // having advanced it.
    write_i32(&mut player.arkhata_ppd, ARKHATA_PPD_RAMMY_STATE_OFFSET, 2);

    assert_eq!(player.arkhata_rammy_state(), 2);
    assert_eq!(player.arkhata_clerk_state(), 5);

    let encoded = player.encode_legacy_arkhata_ppd();
    let mut decoded = PlayerRuntime::connected(2, 0);
    assert!(decoded.decode_legacy_arkhata_ppd(&encoded));
    assert_eq!(decoded.arkhata_rammy_state(), 2);
    assert_eq!(decoded.arkhata_clerk_state(), 5);
}

#[test]
fn caligar_ppd_checks_skelly_door_unlock_flags() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert!(!player.caligar_skelly_door_unlocked(0));

    player.caligar_ppd.resize(LEGACY_CALIGAR_PPD_SIZE, 0);
    player.caligar_ppd[CALIGAR_PPD_DOOR_FLAG_OFFSET + 2] = 0x03;
    assert!(!player.caligar_skelly_door_unlocked(2));

    player.caligar_ppd[CALIGAR_PPD_DOOR_FLAG_OFFSET + 2] = 0x07;
    assert!(player.caligar_skelly_door_unlocked(2));
    assert!(!player.caligar_skelly_door_unlocked(4));
}

#[test]
fn caligar_ppd_marks_skelly_death_lock_bits_from_legacy_home_positions() {
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(
        player.mark_caligar_skelly_death(103, 224),
        CaligarSkellyDeathResult::PartiallyUnlocked {
            door_index: 0,
            bit: 1,
        }
    );
    assert_eq!(
        player.mark_caligar_skelly_death(103, 211),
        CaligarSkellyDeathResult::PartiallyUnlocked {
            door_index: 0,
            bit: 2,
        }
    );
    assert_eq!(
        player.mark_caligar_skelly_death(103, 198),
        CaligarSkellyDeathResult::FullyUnlocked {
            door_index: 0,
            bit: 4,
        }
    );
    assert!(player.caligar_skelly_door_unlocked(0));

    assert_eq!(
        player.mark_caligar_skelly_death(103, 198),
        CaligarSkellyDeathResult::AlreadyUnlocked {
            door_index: 0,
            bit: 4,
        }
    );
    assert_eq!(
        player.mark_caligar_skelly_death(200, 200),
        CaligarSkellyDeathResult::Unmapped { x: 200, y: 200 }
    );
}

#[test]
fn caligar_ppd_marks_skelly_death_third_door_dual_x_positions() {
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(
        player.mark_caligar_skelly_death(226, 158),
        CaligarSkellyDeathResult::PartiallyUnlocked {
            door_index: 2,
            bit: 1,
        }
    );
    assert_eq!(
        player.mark_caligar_skelly_death(227, 145),
        CaligarSkellyDeathResult::PartiallyUnlocked {
            door_index: 2,
            bit: 2,
        }
    );

    let encoded = player.encode_legacy_caligar_ppd();
    assert_eq!(encoded[CALIGAR_PPD_DOOR_FLAG_OFFSET + 2], 0x03);
}

#[test]
fn caligar_ppd_blob_replaces_and_appends_legacy_block() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(DRD_CALIGAR_PPD, 0x8100_009f);
    assert_eq!(player.observe_caligar_training(1), Some(true));

    let mut existing = Vec::new();
    let mut existing_caligar = vec![0; LEGACY_CALIGAR_PPD_SIZE];
    write_i32(&mut existing_caligar, CALIGAR_PPD_WATCH_FLAG_OFFSET, 4);
    write_ppd_block(&mut existing, DRD_CALIGAR_PPD, &existing_caligar);
    let encoded = player.encode_legacy_ppd_blob(&existing);
    assert_eq!(read_u32(&encoded, 0), DRD_CALIGAR_PPD);
    assert_eq!(read_i32(&encoded, 8 + CALIGAR_PPD_WATCH_FLAG_OFFSET), 1);

    let appended = player.encode_legacy_ppd_blob(&[]);
    assert_eq!(read_u32(&appended, 0), DRD_CALIGAR_PPD);
    assert_eq!(read_i32(&appended, 8 + CALIGAR_PPD_WATCH_FLAG_OFFSET), 1);
}

#[test]
fn caligar_ppd_guard_talk_state_round_trips() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(player.caligar_guard_state(), 0);
    assert_eq!(player.caligar_guard_last_talk(), 0);

    player.set_caligar_guard_talk(2, 555);
    assert_eq!(player.caligar_guard_state(), 2);
    assert_eq!(player.caligar_guard_last_talk(), 555);

    // C `case 5: ppd->guard_state = 0;` leaves `guard_last_talk` untouched.
    player.reset_caligar_guard_state_timeout();
    assert_eq!(player.caligar_guard_state(), 0);
    assert_eq!(player.caligar_guard_last_talk(), 555);
}

#[test]
fn caligar_ppd_guard_reset_if_state_three_only_fires_at_state_three() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_caligar_guard_talk(2, 100);
    player.reset_caligar_guard_if_state_three();
    assert_eq!(player.caligar_guard_state(), 2);

    player.set_caligar_guard_talk(3, 100);
    player.reset_caligar_guard_if_state_three();
    assert_eq!(player.caligar_guard_state(), 0);
    assert_eq!(player.caligar_guard_last_talk(), 0);
}

#[test]
fn caligar_ppd_guard2_last_talk_round_trips() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(player.caligar_guard2_last_talk(), 0);
    player.set_caligar_guard2_last_talk(321);
    assert_eq!(player.caligar_guard2_last_talk(), 321);
}

#[test]
fn caligar_ppd_glori_talk_state_and_mini_block_reset_round_trip() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_caligar_glori_talk(4, 42);
    assert_eq!(player.caligar_glori_state(), 4);
    assert_eq!(player.caligar_glori_last_talk(), 42);

    // C `analyse_text_driver` code 2: `1..=5 -> 1`.
    player.reset_caligar_glori_to_mini_block_start();
    assert_eq!(player.caligar_glori_state(), 1);
    assert_eq!(player.caligar_glori_last_talk(), 0);

    player.set_caligar_glori_talk(9, 42);
    player.reset_caligar_glori_to_mini_block_start();
    assert_eq!(player.caligar_glori_state(), 8);

    player.set_caligar_glori_talk(19, 42);
    player.reset_caligar_glori_to_mini_block_start();
    // state 19 is out of every reset window - unchanged.
    assert_eq!(player.caligar_glori_state(), 19);
}

#[test]
fn caligar_ppd_arquin_talk_state_and_mini_block_reset_round_trip() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_caligar_arquin_talk(2, 42);
    player.reset_caligar_arquin_to_mini_block_start();
    assert_eq!(player.caligar_arquin_state(), 1);

    player.set_caligar_arquin_talk(5, 42);
    player.reset_caligar_arquin_to_mini_block_start();
    assert_eq!(player.caligar_arquin_state(), 4);
}

#[test]
fn caligar_ppd_smith_talk_state_and_mini_block_reset_round_trip() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_caligar_smith_talk(2, 42);
    player.reset_caligar_smith_to_mini_block_start();
    assert_eq!(player.caligar_smith_state(), 1);

    player.set_caligar_smith_talk(6, 42);
    player.reset_caligar_smith_to_mini_block_start();
    assert_eq!(player.caligar_smith_state(), 3);
}

#[test]
fn caligar_ppd_homden_talk_state_and_mini_block_reset_round_trip() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_caligar_homden_talk(3, 42);
    player.reset_caligar_homden_to_mini_block_start();
    assert_eq!(player.caligar_homden_state(), 2);

    player.set_caligar_homden_talk(10, 42);
    player.reset_caligar_homden_to_mini_block_start();
    assert_eq!(player.caligar_homden_state(), 6);

    player.set_caligar_homden_talk(5, 42);
    player.reset_caligar_homden_to_mini_block_start();
    // state 5 is outside both reset windows - unchanged.
    assert_eq!(player.caligar_homden_state(), 5);
}

#[test]
fn caligar_ppd_watch_flag_reads_the_shared_offset_with_training() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(player.caligar_watch_flag(), 0);
    player.observe_caligar_training(1);
    player.observe_caligar_training(2);
    assert_eq!(player.caligar_watch_flag(), 1 | 4);
}

#[test]
fn arkhata_ppd_monk_state_is_read_only_here() {
    let mut player = PlayerRuntime::connected(1, 0);
    assert_eq!(player.arkhata_monk_state(), 0);

    let mut bytes = vec![0u8; LEGACY_ARKHATA_PPD_SIZE];
    write_i32(&mut bytes, ARKHATA_PPD_MONK_STATE_OFFSET, 25);
    assert!(player.decode_legacy_arkhata_ppd(&bytes));
    assert_eq!(player.arkhata_monk_state(), 25);
}

#[test]
fn got_hit_fightback_obeys_legacy_no_fight_and_distance_gates() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.driver_stop(TICKS_PER_SECOND * 10, true);

    assert!(!player.apply_got_hit_fightback(CharacterId(2), 77, 2, TICKS_PER_SECOND * 13,));
    assert_eq!(player.action.action, PlayerActionCode::Idle);

    assert!(!player.apply_got_hit_fightback(CharacterId(2), 77, 3, TICKS_PER_SECOND * 14,));
    assert_eq!(player.action.action, PlayerActionCode::Idle);
}
