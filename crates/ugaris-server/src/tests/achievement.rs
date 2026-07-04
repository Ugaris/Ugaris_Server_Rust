use super::*;
use ugaris_core::achievement::{AccountAchievements, AchievementStats, AchievementType};

#[test]
fn achievement_data_byte_layout_matches_c_offsets() {
    // Verified against `achievement.h`'s structs via a throwaway
    // `sizeof`/`offsetof` C probe on 64-bit Linux (the legacy server's
    // target): `Achievement` is 56 bytes, `AccountAchievements` is 7176.
    let data = AccountAchievements::default();
    let encoded = encode_legacy_achievement_data(&data);
    assert_eq!(encoded.len(), 7176);

    let mut data = AccountAchievements::default();
    data.achievements[0].timestamp = 1_700_000_000;
    data.achievements[0].achieved_by = "Hero".to_string();
    let encoded = encode_legacy_achievement_data(&data);
    // version (u32) @0, 4 bytes padding, achievements[0] @8: timestamp @8
    let timestamp_bytes = &encoded[8..16];
    assert_eq!(
        i64::from_le_bytes(timestamp_bytes.try_into().unwrap()),
        1_700_000_000
    );
    // achievements[0].achieved_by @8+16=24
    assert_eq!(&encoded[24..28], b"Hero");
}

#[test]
fn achievement_data_roundtrips_through_bytes() {
    let mut data = AccountAchievements::default();
    data.version = 3;
    data.award(AchievementType::FirstBlood, "Hero", 42);
    data.add_progress(AchievementType::DemonSlayer, 5, "Hero", 99);
    let last = ugaris_core::achievement::MAX_ACHIEVEMENTS - 1;
    data.achievements[last].progress = 7;
    data.achievements[last].target = 10;

    let encoded = encode_legacy_achievement_data(&data);
    let decoded = decode_legacy_achievement_data(&encoded).expect("decode");
    assert_eq!(decoded, data);
}

#[test]
fn achievement_data_decode_rejects_short_buffer() {
    assert!(decode_legacy_achievement_data(&[0u8; 10]).is_none());
}

#[test]
fn achievement_stats_byte_layout_matches_c_offsets() {
    let mut stats = AchievementStats::default();
    stats.flowers_picked = 1;
    stats.demons_per_area = [10, 20, 30, 40];
    stats.gold_earned = 0x0102_0304_0506_0708;
    let encoded = encode_legacy_achievement_stats(&stats);
    assert_eq!(encoded.len(), 176);
    // demons_per_area starts at offset 24, 8 bytes each.
    assert_eq!(u64::from_le_bytes(encoded[24..32].try_into().unwrap()), 10);
    assert_eq!(u64::from_le_bytes(encoded[32..40].try_into().unwrap()), 20);
    // gold_earned at offset 128.
    assert_eq!(
        u64::from_le_bytes(encoded[128..136].try_into().unwrap()),
        0x0102_0304_0506_0708
    );
}

#[test]
fn achievement_stats_roundtrips_through_bytes() {
    let mut stats = AchievementStats::default();
    stats.flowers_picked = 10;
    stats.mushrooms_picked = 20;
    stats.berries_picked = 30;
    stats.potions_brewed = 40;
    stats.demons_defeated = 50;
    stats.demons_per_area = [1, 2, 3, 4];
    stats.enemies_killed = 60;
    stats.pvp_kills = 70;
    stats.pents_solved = 80;
    stats.pents_per_area = [5, 6, 7, 8];
    stats.lucky_pents_hit = 90;
    stats.chests_opened = 100;
    stats.earth_stones = 110;
    stats.fire_stones = 120;
    stats.ice_stones = 130;
    stats.military_missions = 140;
    stats.tunnel_levels = 150;
    stats.silver_mined = 160;
    stats.gold_mined = 170;
    stats.gold_earned = 180;
    stats.play_time_minutes = 190;
    stats.login_streak = 200;
    stats.last_login_day = 210;

    let encoded = encode_legacy_achievement_stats(&stats);
    let decoded = decode_legacy_achievement_stats(&encoded).expect("decode");
    assert_eq!(decoded, stats);
}

#[test]
fn achievement_stats_decode_rejects_short_buffer() {
    assert!(decode_legacy_achievement_stats(&[0u8; 10]).is_none());
}

#[test]
fn achievement_data_subscriber_blob_replaces_block_and_preserves_unknown() {
    let unknown_id = (77 << 24) | 9;
    let mut existing = Vec::new();
    write_legacy_subscriber_block(&mut existing, unknown_id, &[1, 2, 3]);
    write_legacy_subscriber_block(&mut existing, DRD_ACHIEVEMENT_DATA, &[9, 9, 9]);

    let mut data = AccountAchievements::default();
    data.award(AchievementType::FirstBlood, "Hero", 1);

    let encoded = encode_legacy_achievement_data_subscriber_blob(&existing, &data);
    let blocks = parse_legacy_subscriber_blocks(&encoded).unwrap();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].id, unknown_id);
    assert_eq!(blocks[0].data, &[1, 2, 3]);
    assert_eq!(blocks[1].id, DRD_ACHIEVEMENT_DATA);

    let decoded = decode_legacy_achievement_data_subscriber_blob(&encoded).unwrap();
    assert!(decoded.is_unlocked(AchievementType::FirstBlood));
}

#[test]
fn achievement_data_subscriber_blob_omits_default_data() {
    let mut existing = Vec::new();
    write_legacy_subscriber_block(&mut existing, DRD_ACHIEVEMENT_DATA, &[9, 9, 9]);

    let encoded =
        encode_legacy_achievement_data_subscriber_blob(&existing, &AccountAchievements::default());

    assert!(parse_legacy_subscriber_blocks(&encoded).unwrap().is_empty());
    assert!(decode_legacy_achievement_data_subscriber_blob(&encoded).is_none());
}

#[test]
fn achievement_stats_subscriber_blob_replaces_block_and_preserves_unknown() {
    let unknown_id = (77 << 24) | 9;
    let mut existing = Vec::new();
    write_legacy_subscriber_block(&mut existing, unknown_id, &[1, 2, 3]);
    write_legacy_subscriber_block(&mut existing, DRD_ACHIEVEMENT_STATS, &[9, 9, 9]);

    let mut stats = AchievementStats::default();
    stats.flowers_picked = 42;

    let encoded = encode_legacy_achievement_stats_subscriber_blob(&existing, &stats);
    let blocks = parse_legacy_subscriber_blocks(&encoded).unwrap();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].id, unknown_id);
    assert_eq!(blocks[1].id, DRD_ACHIEVEMENT_STATS);

    let decoded = decode_legacy_achievement_stats_subscriber_blob(&encoded).unwrap();
    assert_eq!(decoded.flowers_picked, 42);
}

#[test]
fn achievement_stats_subscriber_blob_omits_default_stats() {
    let mut existing = Vec::new();
    write_legacy_subscriber_block(&mut existing, DRD_ACHIEVEMENT_STATS, &[9, 9, 9]);

    let encoded =
        encode_legacy_achievement_stats_subscriber_blob(&existing, &AchievementStats::default());

    assert!(parse_legacy_subscriber_blocks(&encoded).unwrap().is_empty());
    assert!(decode_legacy_achievement_stats_subscriber_blob(&encoded).is_none());
}

#[test]
fn achievement_data_and_stats_blocks_coexist_with_account_depot_in_one_blob() {
    let mut data = AccountAchievements::default();
    data.award(AchievementType::FirstBlood, "Hero", 5);
    let mut stats = AchievementStats::default();
    stats.chests_opened = 3;

    let blob = encode_legacy_achievement_stats_subscriber_blob(
        &encode_legacy_achievement_data_subscriber_blob(&[], &data),
        &stats,
    );

    let decoded_data = decode_legacy_achievement_data_subscriber_blob(&blob).unwrap();
    let decoded_stats = decode_legacy_achievement_stats_subscriber_blob(&blob).unwrap();
    assert!(decoded_data.is_unlocked(AchievementType::FirstBlood));
    assert_eq!(decoded_stats.chests_opened, 3);
    // Account depot decode looks for its own block id and should find none.
    assert!(decode_legacy_account_depot_subscriber_blob(&blob).is_none());
}
