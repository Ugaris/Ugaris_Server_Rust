use super::*;
use ugaris_core::achievement::{AccountAchievements, AchievementStats, AchievementType};
use ugaris_protocol::mod_achievements::SV_ACH_UNLOCK;
use ugaris_protocol::packet::SV_MOD3;

fn connected_god(character_id: CharacterId) -> (World, ServerRuntime) {
    let mut world = World::default();
    let mut god = login_character(character_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(1, commands, 0);
    if let Some(player) = runtime.players.get_mut(&1) {
        player.character_id = Some(character_id);
    }
    (world, runtime)
}

fn add_connected_target(
    world: &mut World,
    runtime: &mut ServerRuntime,
    target_id: CharacterId,
    session_id: u64,
) {
    world.add_character(login_character(
        target_id,
        &login_block("Target"),
        1,
        11,
        10,
    ));
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(session_id, commands, 0);
    if let Some(player) = runtime.players.get_mut(&session_id) {
        player.character_id = Some(target_id);
    }
}

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

// ============================================================================
// `/achievements`/`/achstats`/`/achgive`/`/achfix`/`/achclear`/`/achsync`
// command dispatch (`achievement.c:1421-1810`, `command.c:9076-9227`).
// ============================================================================

#[test]
fn achievements_command_reports_no_unlocks_message_like_c() {
    let character_id = CharacterId(7);
    let (mut world, mut runtime) = connected_god(character_id);
    let result = apply_achievement_command(&world, &mut runtime, character_id, "/achievements", 1)
        .expect("achievements should be handled");
    assert_eq!(result.message_bytes.len(), 2);
    assert_eq!(
        result.message_bytes[1],
        b"You haven't unlocked any achievements yet. Keep playing!".to_vec()
    );
    // Header line carries the C `COL_ORANGE "=== Your Achievements ===" COL_RESET` bytes.
    assert!(result.message_bytes[0].starts_with(COL_ORANGE));
    assert!(result.message_bytes[0].ends_with(COL_RESET));
    let _ = &mut world;
}

#[test]
fn achievements_command_lists_unlocked_entries_with_date_and_unlock_count() {
    let character_id = CharacterId(7);
    let (world, mut runtime) = connected_god(character_id);
    runtime.players.get_mut(&1).unwrap().achievement_data.award(
        AchievementType::FirstBlood,
        "Godmode",
        1_700_000_000,
    );

    let result = apply_achievement_command(&world, &mut runtime, character_id, "/achievements", 1)
        .expect("achievements should be handled");
    assert_eq!(result.message_bytes.len(), 3);
    let entry = String::from_utf8_lossy(&result.message_bytes[1]).to_string();
    assert!(entry.contains("First Blood"));
    assert!(entry.contains("2023-11-14")); // 1_700_000_000 UTC.
    assert!(entry.contains("by Godmode"));
    let footer = String::from_utf8_lossy(&result.message_bytes[2]).to_string();
    assert!(footer.contains(&format!(
        "Unlocked: 1/{ACHIEVEMENT_TYPE_COUNT} achievements"
    )));
}

#[test]
fn achstats_command_lists_every_category_like_c() {
    let character_id = CharacterId(7);
    let (world, mut runtime) = connected_god(character_id);
    {
        let stats = &mut runtime.players.get_mut(&1).unwrap().achievement_stats;
        stats.flowers_picked = 5;
        stats.demons_per_area = [1, 2, 3, 4];
        stats.pents_per_area = [5, 6, 7, 8];
        stats.silver_mined = 100;
        stats.gold_mined = 200;
    }

    let result = apply_achievement_command(&world, &mut runtime, character_id, "/achstats", 1)
        .expect("achstats should be handled");
    let lines: Vec<String> = result
        .message_bytes
        .iter()
        .map(|line| String::from_utf8_lossy(line).to_string())
        .collect();
    assert!(lines[0].contains("Achievement Statistics"));
    assert!(lines.iter().any(|line| line == "  Flowers picked: 5"));
    assert!(lines
        .iter()
        .any(|line| line == "    Earth: 1, Fire: 2, Ice: 3, Hell: 4"));
    assert!(lines
        .iter()
        .any(|line| line == "    Earth: 5, Fire: 6, Ice: 7, Hell: 8"));
    assert!(lines.iter().any(|line| line == "  Silver mined: 100"));
    assert!(lines.iter().any(|line| line == "  Gold mined: 200"));
}

#[test]
fn achievements_and_achstats_respect_legacy_cmdcmp_prefix_lengths() {
    let character_id = CharacterId(7);
    let (world, mut runtime) = connected_god(character_id);
    // "achievements" has minlen 6: shorter abbreviations don't match.
    assert!(apply_achievement_command(&world, &mut runtime, character_id, "/achie", 1).is_none());
    assert!(apply_achievement_command(&world, &mut runtime, character_id, "/achiev", 1).is_some());
    // "achstats" has minlen 8 == its own length: no abbreviation at all.
    assert!(apply_achievement_command(&world, &mut runtime, character_id, "/achstat", 1).is_none());
    assert!(
        apply_achievement_command(&world, &mut runtime, character_id, "/achstats", 1).is_some()
    );
}

#[test]
fn achgive_requires_god_flag() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    world.add_character(login_character(
        character_id,
        &login_block("Mortal"),
        1,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();
    assert!(
        apply_achievement_command(&world, &mut runtime, character_id, "/achgive Mortal 0", 1)
            .is_none()
    );
}

#[test]
fn achgive_awards_and_notifies_target_session_with_unlock_and_congrats() {
    let god_id = CharacterId(7);
    let target_id = CharacterId(8);
    let (mut world, mut runtime) = connected_god(god_id);
    add_connected_target(&mut world, &mut runtime, target_id, 2);

    let result = apply_achievement_command(
        &world,
        &mut runtime,
        god_id,
        "/achgive Target 0", // 0 = StartedUgaris
        1_700_000_000,
    )
    .expect("achgive should be handled");
    assert_eq!(result.messages, vec!["Achievement 0 awarded to Target."]);

    let target_player = runtime.player_for_character(target_id).unwrap();
    assert!(target_player
        .achievement_data
        .is_unlocked(AchievementType::StartedUgaris));

    let payloads = runtime
        .tick_out
        .get(&2)
        .expect("target session got packets");
    assert_eq!(payloads.len(), 3);
    assert_eq!(payloads[0][0], SV_MOD3);
    assert_eq!(payloads[0][2], SV_ACH_UNLOCK);
    assert_eq!(payloads[0][3], AchievementType::StartedUgaris as u8);
    let line1 = text_payload_bytes(&payloads[1]);
    assert!(String::from_utf8_lossy(&line1).contains("Achievement Unlocked"));
    let line2 = text_payload_bytes(&payloads[2]);
    assert!(!line2.is_empty());
}

#[test]
fn achgive_rejects_unknown_player_and_bad_id() {
    let god_id = CharacterId(7);
    let (world, mut runtime) = connected_god(god_id);
    let missing =
        apply_achievement_command(&world, &mut runtime, god_id, "/achgive Ghost 0", 1).unwrap();
    assert_eq!(missing.messages, vec!["Player 'Ghost' not found."]);

    let mut world2 = world;
    add_connected_target(&mut world2, &mut runtime, CharacterId(8), 2);
    let bad_id =
        apply_achievement_command(&world2, &mut runtime, god_id, "/achgive Target 9999", 1)
            .unwrap();
    assert_eq!(
        bad_id.messages,
        vec![format!(
            "Invalid achievement ID. Range: 0-{}",
            ACHIEVEMENT_TYPE_COUNT - 1
        )]
    );
}

#[test]
fn achfix_awards_level_won_profession_and_stat_thresholds_for_self() {
    let character_id = CharacterId(7);
    let (mut world, mut runtime) = connected_god(character_id);
    {
        let character = world.characters.get_mut(&character_id).unwrap();
        character.level = 20;
        character.flags.insert(CharacterFlags::WON);
        character.professions[ugaris_core::achievement::P_MINER as usize] = 20;
    }
    runtime
        .players
        .get_mut(&1)
        .unwrap()
        .achievement_stats
        .chests_opened = 10;

    let result = apply_achievement_command(&world, &mut runtime, character_id, "/achfix", 1)
        .expect("achfix should be handled");
    assert_eq!(result.messages, vec!["Achievements fixed for Godmode."]);

    let player = runtime.player_for_character(character_id).unwrap();
    assert!(player
        .achievement_data
        .is_unlocked(AchievementType::ExperiencedHero)); // level >= 20
    assert!(player
        .achievement_data
        .is_unlocked(AchievementType::Ladykiller));
    assert!(player
        .achievement_data
        .is_unlocked(AchievementType::MasterMiner));
    assert!(player.achievement_data.is_unlocked(AchievementType::Looter));
}

#[test]
fn achclear_resets_data_and_stats_for_named_target() {
    let god_id = CharacterId(7);
    let target_id = CharacterId(8);
    let (mut world, mut runtime) = connected_god(god_id);
    add_connected_target(&mut world, &mut runtime, target_id, 2);
    {
        let target_player = runtime.player_for_character_mut(target_id).unwrap();
        target_player
            .achievement_data
            .award(AchievementType::FirstBlood, "Target", 1);
        target_player.achievement_stats.chests_opened = 5;
    }

    let result = apply_achievement_command(&world, &mut runtime, god_id, "/achclear Target", 1)
        .expect("achclear should be handled");
    assert_eq!(result.messages, vec!["Achievements cleared for Target."]);
    let target_player = runtime.player_for_character(target_id).unwrap();
    assert_eq!(
        target_player.achievement_data,
        AccountAchievements::default()
    );
    assert_eq!(target_player.achievement_stats, AchievementStats::default());
}

#[test]
fn achsync_sends_batched_payloads_to_named_target() {
    let god_id = CharacterId(7);
    let target_id = CharacterId(8);
    let (mut world, mut runtime) = connected_god(god_id);
    add_connected_target(&mut world, &mut runtime, target_id, 2);
    runtime
        .player_for_character_mut(target_id)
        .unwrap()
        .achievement_data
        .award(AchievementType::FirstBlood, "Target", 1);

    let result = apply_achievement_command(&world, &mut runtime, god_id, "/achsync Target", 1)
        .expect("achsync should be handled");
    assert_eq!(
        result.messages,
        vec!["Achievements synced to client for Target."]
    );
    let payloads = runtime
        .tick_out
        .get(&2)
        .expect("target session got sync packets");
    assert!(!payloads.is_empty());
    assert_eq!(
        payloads[0][2],
        ugaris_protocol::mod_achievements::SV_ACH_SYNC
    );
}

#[test]
fn achfix_achclear_achsync_are_god_only_and_full_word_only() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    world.add_character(login_character(
        character_id,
        &login_block("Mortal"),
        1,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();
    assert!(apply_achievement_command(&world, &mut runtime, character_id, "/achfix", 1).is_none());
    assert!(
        apply_achievement_command(&world, &mut runtime, character_id, "/achclear", 1).is_none()
    );
    assert!(apply_achievement_command(&world, &mut runtime, character_id, "/achsync", 1).is_none());

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    // Abbreviations below the full word length must not match
    // (`cmdcmp(..., "achclear", 8)` etc. require the exact word).
    assert!(apply_achievement_command(&world, &mut runtime, character_id, "/achclea", 1).is_none());
    assert!(
        apply_achievement_command(&world, &mut runtime, character_id, "/achclear", 1).is_some()
    );
}

fn connected_player(character_id: CharacterId, session_id: u64) -> (World, ServerRuntime) {
    let mut world = World::default();
    world.add_character(login_character(
        character_id,
        &login_block("Tester"),
        1,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();
    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(session_id, commands, 0);
    if let Some(player) = runtime.players.get_mut(&session_id) {
        player.character_id = Some(character_id);
    }
    (world, runtime)
}

#[test]
fn award_play_time_minute_bumps_stat_without_unlock_below_threshold() {
    let character_id = CharacterId(7);
    let (world, mut runtime) = connected_player(character_id, 1);

    award_play_time_minute(&world, &mut runtime, character_id);

    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.achievement_stats.play_time_minutes, 1);
    assert!(!player
        .achievement_data
        .is_unlocked(AchievementType::DedicatedPlayer));
    assert!(runtime.tick_out.get(&1).is_none());
}

#[test]
fn award_play_time_minute_unlocks_dedicated_player_at_1440_minutes_and_notifies_session() {
    let character_id = CharacterId(7);
    let (world, mut runtime) = connected_player(character_id, 1);
    runtime
        .player_for_character_mut(character_id)
        .unwrap()
        .achievement_stats
        .play_time_minutes = 1439;

    award_play_time_minute(&world, &mut runtime, character_id);

    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.achievement_stats.play_time_minutes, 1440);
    assert!(player
        .achievement_data
        .is_unlocked(AchievementType::DedicatedPlayer));

    let payloads = runtime
        .tick_out
        .get(&1)
        .expect("session should receive an unlock packet");
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0][0], SV_MOD3);
    assert_eq!(payloads[0][2], SV_ACH_UNLOCK);
    assert_eq!(payloads[0][3], AchievementType::DedicatedPlayer as u8);
}

#[test]
fn award_play_time_minute_is_a_noop_for_characters_without_a_player_runtime() {
    let character_id = CharacterId(9);
    let mut world = World::default();
    world.add_character(login_character(
        character_id,
        &login_block("Npc"),
        1,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();

    // Should not panic even though no session/PlayerRuntime exists for this
    // character (mirrors C's `player_update` only ever running for
    // connected player slots).
    award_play_time_minute(&world, &mut runtime, character_id);
    assert!(runtime.player_for_character(character_id).is_none());
}

// ============================================================================
// `award_enemy_killed_achievement` (`src/system/death.c:417-422`,
// `kill_char`'s `achievement_add_enemy_killed`/`achievement_add_demons`).
// ============================================================================

#[test]
fn award_enemy_killed_achievement_unlocks_first_blood_on_first_kill() {
    let character_id = CharacterId(7);
    let (world, mut runtime) = connected_player(character_id, 1);

    award_enemy_killed_achievement(&world, &mut runtime, character_id, 1, false);

    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.achievement_stats.enemies_killed, 1);
    assert!(player
        .achievement_data
        .is_unlocked(AchievementType::FirstBlood));

    let payloads = runtime
        .tick_out
        .get(&1)
        .expect("session should receive an unlock packet");
    assert_eq!(payloads.len(), 1);
    assert_eq!(payloads[0][0], SV_MOD3);
    assert_eq!(payloads[0][2], SV_ACH_UNLOCK);
    assert_eq!(payloads[0][3], AchievementType::FirstBlood as u8);
}

#[test]
fn award_enemy_killed_achievement_does_not_reunlock_first_blood_on_later_kills() {
    let character_id = CharacterId(7);
    let (world, mut runtime) = connected_player(character_id, 1);
    runtime
        .player_for_character_mut(character_id)
        .unwrap()
        .achievement_stats
        .enemies_killed = 1;
    runtime
        .player_for_character_mut(character_id)
        .unwrap()
        .achievement_data
        .award(AchievementType::FirstBlood, "Tester", 1);

    award_enemy_killed_achievement(&world, &mut runtime, character_id, 1, false);

    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.achievement_stats.enemies_killed, 2);
    assert!(runtime.tick_out.get(&1).is_none());
}

#[test]
fn award_enemy_killed_achievement_also_awards_demon_progress_when_target_is_demon() {
    let character_id = CharacterId(7);
    let (world, mut runtime) = connected_player(character_id, 1);

    // area_id 4 maps to `PentArea::Earth` (`achievement_area_to_pent_index`).
    award_enemy_killed_achievement(&world, &mut runtime, character_id, 4, true);

    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.achievement_stats.enemies_killed, 1);
    assert_eq!(player.achievement_stats.demons_defeated, 1);
    assert_eq!(player.achievement_stats.demons_per_area[0], 1);
    assert!(player
        .achievement_data
        .is_unlocked(AchievementType::FirstBlood));
}

#[test]
fn award_enemy_killed_achievement_skips_demon_progress_when_target_is_not_demon() {
    let character_id = CharacterId(7);
    let (world, mut runtime) = connected_player(character_id, 1);

    award_enemy_killed_achievement(&world, &mut runtime, character_id, 4, false);

    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.achievement_stats.demons_defeated, 0);
}

#[test]
fn award_enemy_killed_achievement_is_a_noop_for_characters_without_a_player_runtime() {
    let character_id = CharacterId(9);
    let mut world = World::default();
    world.add_character(login_character(
        character_id,
        &login_block("Npc"),
        1,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();

    award_enemy_killed_achievement(&world, &mut runtime, character_id, 1, false);
    assert!(runtime.player_for_character(character_id).is_none());
}

// ============================================================================
// `award_gathering_achievement` (`src/module/alchemy.c:1306-1315`,
// `flower_driver`'s `achievement_add_flowers`/`_mushrooms`/`_berries`).
// ============================================================================

#[test]
fn award_gathering_achievement_credits_flowers_for_kind_1_through_7() {
    let character_id = CharacterId(7);
    let (world, mut runtime) = connected_player(character_id, 1);
    runtime
        .player_for_character_mut(character_id)
        .unwrap()
        .achievement_stats
        .flowers_picked = 9;

    award_gathering_achievement(&world, &mut runtime, character_id, 7);

    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.achievement_stats.flowers_picked, 10);
    assert_eq!(player.achievement_stats.mushrooms_picked, 0);
    assert_eq!(player.achievement_stats.berries_picked, 0);
    assert!(player
        .achievement_data
        .is_unlocked(AchievementType::GreenThumb));
    let payloads = runtime
        .tick_out
        .get(&1)
        .expect("session should receive an unlock packet");
    assert_eq!(payloads[0][3], AchievementType::GreenThumb as u8);
}

#[test]
fn award_gathering_achievement_credits_mushrooms_for_kind_8_through_16() {
    let character_id = CharacterId(7);
    let (world, mut runtime) = connected_player(character_id, 1);
    runtime
        .player_for_character_mut(character_id)
        .unwrap()
        .achievement_stats
        .mushrooms_picked = 9;

    award_gathering_achievement(&world, &mut runtime, character_id, 16);

    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.achievement_stats.mushrooms_picked, 10);
    assert_eq!(player.achievement_stats.flowers_picked, 0);
    assert!(player
        .achievement_data
        .is_unlocked(AchievementType::MushroomHunter));
}

#[test]
fn award_gathering_achievement_credits_berries_for_kind_17_through_20() {
    let character_id = CharacterId(7);
    let (world, mut runtime) = connected_player(character_id, 1);
    runtime
        .player_for_character_mut(character_id)
        .unwrap()
        .achievement_stats
        .berries_picked = 9;

    award_gathering_achievement(&world, &mut runtime, character_id, 20);

    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.achievement_stats.berries_picked, 10);
    assert_eq!(player.achievement_stats.mushrooms_picked, 0);
    assert!(player
        .achievement_data
        .is_unlocked(AchievementType::BerryPicker));
}

#[test]
fn award_gathering_achievement_ignores_out_of_range_kind() {
    let character_id = CharacterId(7);
    let (world, mut runtime) = connected_player(character_id, 1);

    award_gathering_achievement(&world, &mut runtime, character_id, 0);
    award_gathering_achievement(&world, &mut runtime, character_id, 21);

    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.achievement_stats.flowers_picked, 0);
    assert_eq!(player.achievement_stats.mushrooms_picked, 0);
    assert_eq!(player.achievement_stats.berries_picked, 0);
}

#[test]
fn award_gathering_achievement_is_a_noop_for_characters_without_a_player_runtime() {
    let character_id = CharacterId(9);
    let mut world = World::default();
    world.add_character(login_character(
        character_id,
        &login_block("Npc"),
        1,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();

    award_gathering_achievement(&world, &mut runtime, character_id, 1);
    assert!(runtime.player_for_character(character_id).is_none());
}

// ============================================================================
// `award_potion_brewed_achievement` (`src/module/alchemy.c:1077-1082`,
// `flask_driver`'s `mixer()` success branch calling `achievement_add_
// potions`).
// ============================================================================

#[test]
fn award_potion_brewed_achievement_unlocks_alchemist_at_10_potions() {
    let character_id = CharacterId(7);
    let (world, mut runtime) = connected_player(character_id, 1);
    runtime
        .player_for_character_mut(character_id)
        .unwrap()
        .achievement_stats
        .potions_brewed = 9;

    award_potion_brewed_achievement(&world, &mut runtime, character_id);

    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.achievement_stats.potions_brewed, 10);
    assert!(player
        .achievement_data
        .is_unlocked(AchievementType::Alchemist));
    let payloads = runtime
        .tick_out
        .get(&1)
        .expect("session should receive an unlock packet");
    assert_eq!(payloads[0][3], AchievementType::Alchemist as u8);
}

#[test]
fn award_potion_brewed_achievement_bumps_stat_without_unlock_below_threshold() {
    let character_id = CharacterId(7);
    let (world, mut runtime) = connected_player(character_id, 1);

    award_potion_brewed_achievement(&world, &mut runtime, character_id);

    let player = runtime.player_for_character(character_id).unwrap();
    assert_eq!(player.achievement_stats.potions_brewed, 1);
    assert!(!player
        .achievement_data
        .is_unlocked(AchievementType::Alchemist));
    assert!(runtime.tick_out.get(&1).is_none());
}

#[test]
fn award_potion_brewed_achievement_is_a_noop_for_characters_without_a_player_runtime() {
    let character_id = CharacterId(9);
    let mut world = World::default();
    world.add_character(login_character(
        character_id,
        &login_block("Npc"),
        1,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();

    award_potion_brewed_achievement(&world, &mut runtime, character_id);
    assert!(runtime.player_for_character(character_id).is_none());
}
