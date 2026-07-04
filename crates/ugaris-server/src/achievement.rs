//! Persistence wiring for `crate::achievement`'s (`ugaris-core`) leaf data
//! model: the `AccountAchievements`/`AchievementStats` legacy byte layout
//! (`achievement.h:218-276`) and the `DRD_ACHIEVEMENT_DATA`/`DRD_
//! ACHIEVEMENT_STATS` subscriber-blob block codecs (`achievement.c:358-372`,
//! `set_data(cn, DRD_ACHIEVEMENT_DATA/STATS, ...)`), following the exact
//! pattern `depot.rs` established for `DRD_ACCOUNT_WIDE_DEPOT`.
//!
//! Byte offsets below were verified against the C structs with a throwaway
//! `sizeof`/`offsetof` probe compiled from `achievement.h`'s definitions
//! (64-bit Linux, the legacy server's target): `Achievement` is 56 bytes
//! (`time_t timestamp` @0 (8 bytes), `progress` @8, `target` @12,
//! `achieved_by[40]` @16); `AccountAchievements` is 7176 bytes (`version`
//! @0, 4 bytes of `time_t`-alignment padding, `achievements[128]` @8);
//! `AchievementStats` is 176 bytes (4 leading `u32`s, then 8-byte-aligned
//! `u64` fields, then more `u32`s, then a `reserved[6]` `u32` tail plus 4
//! bytes of trailing alignment padding - offsets recorded inline below).
//!
//! Both C `DRD_*` ids carry `PERSISTENT_SUBSCRIBER_DATA`
//! (`drdata.h:266-267`), i.e. this is nominally account-wide (not
//! per-character) data; matching `crate::achievement`'s module doc note,
//! this codebase persists it in the per-character `subscriber_blob` column
//! for now (same scoping compromise `DRD_ACCOUNT_WIDE_DEPOT` already
//! makes), pending an actual multi-character-per-account model.

use super::*;
use ugaris_core::achievement::{
    AccountAchievements, Achievement, AchievementStats, MAX_ACHIEVEMENTS,
};

/// C `Achievement` (`achievement.h:218-223`): `time_t timestamp` (8) +
/// `unsigned int progress` (4) + `unsigned int target` (4) +
/// `char achieved_by[40]` (40) = 56 bytes, no padding.
const LEGACY_ACHIEVEMENT_ENTRY_SIZE: usize = 56;
const ACHIEVEMENT_ENTRY_TIMESTAMP_OFFSET: usize = 0;
const ACHIEVEMENT_ENTRY_PROGRESS_OFFSET: usize = 8;
const ACHIEVEMENT_ENTRY_TARGET_OFFSET: usize = 12;
const ACHIEVEMENT_ENTRY_ACHIEVED_BY_OFFSET: usize = 16;
const ACHIEVEMENT_ENTRY_ACHIEVED_BY_LEN: usize = 40;

/// C `AccountAchievements` (`achievement.h:226-229`): `unsigned int
/// version` @0 followed by 4 bytes of alignment padding (the array member
/// needs 8-byte, `time_t`-driven alignment), then `achievements[128]` @8.
const ACHIEVEMENT_DATA_VERSION_OFFSET: usize = 0;
const ACHIEVEMENT_DATA_ACHIEVEMENTS_OFFSET: usize = 8;
const LEGACY_ACHIEVEMENT_DATA_SIZE: usize =
    ACHIEVEMENT_DATA_ACHIEVEMENTS_OFFSET + MAX_ACHIEVEMENTS * LEGACY_ACHIEVEMENT_ENTRY_SIZE;

/// C `AchievementStats` (`achievement.h:232-276`) field offsets, in
/// declaration order; the trailing `reserved[6]` (offset 148, 24 bytes) has
/// no Rust-side field and is always encoded as zero.
const STATS_FLOWERS_PICKED_OFFSET: usize = 0;
const STATS_MUSHROOMS_PICKED_OFFSET: usize = 4;
const STATS_BERRIES_PICKED_OFFSET: usize = 8;
const STATS_POTIONS_BREWED_OFFSET: usize = 12;
const STATS_DEMONS_DEFEATED_OFFSET: usize = 16;
const STATS_DEMONS_PER_AREA_OFFSET: usize = 24;
const STATS_ENEMIES_KILLED_OFFSET: usize = 56;
const STATS_PVP_KILLS_OFFSET: usize = 60;
const STATS_PENTS_SOLVED_OFFSET: usize = 64;
const STATS_PENTS_PER_AREA_OFFSET: usize = 68;
const STATS_LUCKY_PENTS_HIT_OFFSET: usize = 84;
const STATS_CHESTS_OPENED_OFFSET: usize = 88;
const STATS_EARTH_STONES_OFFSET: usize = 92;
const STATS_FIRE_STONES_OFFSET: usize = 96;
const STATS_ICE_STONES_OFFSET: usize = 100;
const STATS_MILITARY_MISSIONS_OFFSET: usize = 104;
const STATS_TUNNEL_LEVELS_OFFSET: usize = 108;
const STATS_SILVER_MINED_OFFSET: usize = 112;
const STATS_GOLD_MINED_OFFSET: usize = 120;
const STATS_GOLD_EARNED_OFFSET: usize = 128;
const STATS_PLAY_TIME_MINUTES_OFFSET: usize = 136;
const STATS_LOGIN_STREAK_OFFSET: usize = 140;
const STATS_LAST_LOGIN_DAY_OFFSET: usize = 144;
const LEGACY_ACHIEVEMENT_STATS_SIZE: usize = 176;
const PENT_AREA_COUNT: usize = ugaris_core::achievement::PENT_AREA_COUNT;

pub(crate) fn encode_legacy_achievement_data(data: &AccountAchievements) -> Vec<u8> {
    let mut bytes = vec![0u8; LEGACY_ACHIEVEMENT_DATA_SIZE];
    bytes[ACHIEVEMENT_DATA_VERSION_OFFSET..ACHIEVEMENT_DATA_VERSION_OFFSET + 4]
        .copy_from_slice(&data.version.to_le_bytes());
    for (index, achievement) in data.achievements.iter().enumerate() {
        let base = ACHIEVEMENT_DATA_ACHIEVEMENTS_OFFSET + index * LEGACY_ACHIEVEMENT_ENTRY_SIZE;
        let ts = base + ACHIEVEMENT_ENTRY_TIMESTAMP_OFFSET;
        bytes[ts..ts + 8].copy_from_slice(&achievement.timestamp.to_le_bytes());
        let progress = base + ACHIEVEMENT_ENTRY_PROGRESS_OFFSET;
        bytes[progress..progress + 4].copy_from_slice(&achievement.progress.to_le_bytes());
        let target = base + ACHIEVEMENT_ENTRY_TARGET_OFFSET;
        bytes[target..target + 4].copy_from_slice(&achievement.target.to_le_bytes());
        let name = base + ACHIEVEMENT_ENTRY_ACHIEVED_BY_OFFSET;
        legacy_account_depot_codec::write_fixed_c_string(
            &mut bytes[name..name + ACHIEVEMENT_ENTRY_ACHIEVED_BY_LEN],
            &achievement.achieved_by,
        );
    }
    bytes
}

pub(crate) fn decode_legacy_achievement_data(bytes: &[u8]) -> Option<AccountAchievements> {
    if bytes.len() < LEGACY_ACHIEVEMENT_DATA_SIZE {
        return None;
    }
    let mut data = AccountAchievements::default();
    data.version = u32::from_le_bytes(
        bytes[ACHIEVEMENT_DATA_VERSION_OFFSET..ACHIEVEMENT_DATA_VERSION_OFFSET + 4]
            .try_into()
            .ok()?,
    );
    for index in 0..MAX_ACHIEVEMENTS {
        let base = ACHIEVEMENT_DATA_ACHIEVEMENTS_OFFSET + index * LEGACY_ACHIEVEMENT_ENTRY_SIZE;
        let ts = base + ACHIEVEMENT_ENTRY_TIMESTAMP_OFFSET;
        let timestamp = i64::from_le_bytes(bytes[ts..ts + 8].try_into().ok()?);
        let progress_off = base + ACHIEVEMENT_ENTRY_PROGRESS_OFFSET;
        let progress = u32::from_le_bytes(bytes[progress_off..progress_off + 4].try_into().ok()?);
        let target_off = base + ACHIEVEMENT_ENTRY_TARGET_OFFSET;
        let target = u32::from_le_bytes(bytes[target_off..target_off + 4].try_into().ok()?);
        let name_off = base + ACHIEVEMENT_ENTRY_ACHIEVED_BY_OFFSET;
        let achieved_by = legacy_account_depot_codec::read_fixed_c_string(
            &bytes[name_off..name_off + ACHIEVEMENT_ENTRY_ACHIEVED_BY_LEN],
        );
        data.achievements[index] = Achievement {
            timestamp,
            progress,
            target,
            achieved_by,
        };
    }
    Some(data)
}

pub(crate) fn encode_legacy_achievement_stats(stats: &AchievementStats) -> Vec<u8> {
    let mut bytes = vec![0u8; LEGACY_ACHIEVEMENT_STATS_SIZE];
    let write_u32 = |bytes: &mut [u8], offset: usize, value: u32| {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    };
    let write_u64 = |bytes: &mut [u8], offset: usize, value: u64| {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    };
    write_u32(
        &mut bytes,
        STATS_FLOWERS_PICKED_OFFSET,
        stats.flowers_picked,
    );
    write_u32(
        &mut bytes,
        STATS_MUSHROOMS_PICKED_OFFSET,
        stats.mushrooms_picked,
    );
    write_u32(
        &mut bytes,
        STATS_BERRIES_PICKED_OFFSET,
        stats.berries_picked,
    );
    write_u32(
        &mut bytes,
        STATS_POTIONS_BREWED_OFFSET,
        stats.potions_brewed,
    );
    write_u64(
        &mut bytes,
        STATS_DEMONS_DEFEATED_OFFSET,
        stats.demons_defeated,
    );
    for index in 0..PENT_AREA_COUNT {
        write_u64(
            &mut bytes,
            STATS_DEMONS_PER_AREA_OFFSET + index * 8,
            stats.demons_per_area[index],
        );
    }
    write_u32(
        &mut bytes,
        STATS_ENEMIES_KILLED_OFFSET,
        stats.enemies_killed,
    );
    write_u32(&mut bytes, STATS_PVP_KILLS_OFFSET, stats.pvp_kills);
    write_u32(&mut bytes, STATS_PENTS_SOLVED_OFFSET, stats.pents_solved);
    for index in 0..PENT_AREA_COUNT {
        write_u32(
            &mut bytes,
            STATS_PENTS_PER_AREA_OFFSET + index * 4,
            stats.pents_per_area[index],
        );
    }
    write_u32(
        &mut bytes,
        STATS_LUCKY_PENTS_HIT_OFFSET,
        stats.lucky_pents_hit,
    );
    write_u32(&mut bytes, STATS_CHESTS_OPENED_OFFSET, stats.chests_opened);
    write_u32(&mut bytes, STATS_EARTH_STONES_OFFSET, stats.earth_stones);
    write_u32(&mut bytes, STATS_FIRE_STONES_OFFSET, stats.fire_stones);
    write_u32(&mut bytes, STATS_ICE_STONES_OFFSET, stats.ice_stones);
    write_u32(
        &mut bytes,
        STATS_MILITARY_MISSIONS_OFFSET,
        stats.military_missions,
    );
    write_u32(&mut bytes, STATS_TUNNEL_LEVELS_OFFSET, stats.tunnel_levels);
    write_u64(&mut bytes, STATS_SILVER_MINED_OFFSET, stats.silver_mined);
    write_u64(&mut bytes, STATS_GOLD_MINED_OFFSET, stats.gold_mined);
    write_u64(&mut bytes, STATS_GOLD_EARNED_OFFSET, stats.gold_earned);
    write_u32(
        &mut bytes,
        STATS_PLAY_TIME_MINUTES_OFFSET,
        stats.play_time_minutes,
    );
    write_u32(&mut bytes, STATS_LOGIN_STREAK_OFFSET, stats.login_streak);
    write_u32(
        &mut bytes,
        STATS_LAST_LOGIN_DAY_OFFSET,
        stats.last_login_day,
    );
    bytes
}

pub(crate) fn decode_legacy_achievement_stats(bytes: &[u8]) -> Option<AchievementStats> {
    if bytes.len() < LEGACY_ACHIEVEMENT_STATS_SIZE {
        return None;
    }
    let read_u32 = |offset: usize| -> Option<u32> {
        Some(u32::from_le_bytes(
            bytes[offset..offset + 4].try_into().ok()?,
        ))
    };
    let read_u64 = |offset: usize| -> Option<u64> {
        Some(u64::from_le_bytes(
            bytes[offset..offset + 8].try_into().ok()?,
        ))
    };
    let mut demons_per_area = [0u64; PENT_AREA_COUNT];
    for (index, slot) in demons_per_area.iter_mut().enumerate() {
        *slot = read_u64(STATS_DEMONS_PER_AREA_OFFSET + index * 8)?;
    }
    let mut pents_per_area = [0u32; PENT_AREA_COUNT];
    for (index, slot) in pents_per_area.iter_mut().enumerate() {
        *slot = read_u32(STATS_PENTS_PER_AREA_OFFSET + index * 4)?;
    }
    Some(AchievementStats {
        flowers_picked: read_u32(STATS_FLOWERS_PICKED_OFFSET)?,
        mushrooms_picked: read_u32(STATS_MUSHROOMS_PICKED_OFFSET)?,
        berries_picked: read_u32(STATS_BERRIES_PICKED_OFFSET)?,
        potions_brewed: read_u32(STATS_POTIONS_BREWED_OFFSET)?,
        demons_defeated: read_u64(STATS_DEMONS_DEFEATED_OFFSET)?,
        demons_per_area,
        enemies_killed: read_u32(STATS_ENEMIES_KILLED_OFFSET)?,
        pvp_kills: read_u32(STATS_PVP_KILLS_OFFSET)?,
        pents_solved: read_u32(STATS_PENTS_SOLVED_OFFSET)?,
        pents_per_area,
        lucky_pents_hit: read_u32(STATS_LUCKY_PENTS_HIT_OFFSET)?,
        chests_opened: read_u32(STATS_CHESTS_OPENED_OFFSET)?,
        earth_stones: read_u32(STATS_EARTH_STONES_OFFSET)?,
        fire_stones: read_u32(STATS_FIRE_STONES_OFFSET)?,
        ice_stones: read_u32(STATS_ICE_STONES_OFFSET)?,
        military_missions: read_u32(STATS_MILITARY_MISSIONS_OFFSET)?,
        tunnel_levels: read_u32(STATS_TUNNEL_LEVELS_OFFSET)?,
        silver_mined: read_u64(STATS_SILVER_MINED_OFFSET)?,
        gold_mined: read_u64(STATS_GOLD_MINED_OFFSET)?,
        gold_earned: read_u64(STATS_GOLD_EARNED_OFFSET)?,
        play_time_minutes: read_u32(STATS_PLAY_TIME_MINUTES_OFFSET)?,
        login_streak: read_u32(STATS_LOGIN_STREAK_OFFSET)?,
        last_login_day: read_u32(STATS_LAST_LOGIN_DAY_OFFSET)?,
    })
}

/// Reads the `DRD_ACHIEVEMENT_DATA` block out of the subscriber blob, if
/// present. `None` covers both "block absent" (never awarded anything) and
/// a corrupt/short block.
pub(crate) fn decode_legacy_achievement_data_subscriber_blob(
    bytes: &[u8],
) -> Option<AccountAchievements> {
    parse_legacy_subscriber_blocks(bytes)?
        .into_iter()
        .find(|block| block.id == DRD_ACHIEVEMENT_DATA)
        .and_then(|block| decode_legacy_achievement_data(block.data))
}

/// Reads the `DRD_ACHIEVEMENT_STATS` block out of the subscriber blob, if
/// present.
pub(crate) fn decode_legacy_achievement_stats_subscriber_blob(
    bytes: &[u8],
) -> Option<AchievementStats> {
    parse_legacy_subscriber_blocks(bytes)?
        .into_iter()
        .find(|block| block.id == DRD_ACHIEVEMENT_STATS)
        .and_then(|block| decode_legacy_achievement_stats(block.data))
}

/// Rewrites the `DRD_ACHIEVEMENT_DATA` block in the subscriber blob,
/// leaving every other block (including `DRD_ACCOUNT_WIDE_DEPOT`) byte-for-
/// byte untouched, mirroring `encode_legacy_account_depot_subscriber_blob`.
/// The block is omitted entirely when `data` is the untouched default, so
/// players who never unlock anything don't grow the blob.
pub(crate) fn encode_legacy_achievement_data_subscriber_blob(
    existing: &[u8],
    data: &AccountAchievements,
) -> Vec<u8> {
    let is_default = *data == AccountAchievements::default();
    let mut encoded = Vec::with_capacity(existing.len());
    let Some(blocks) = parse_legacy_subscriber_blocks(existing) else {
        return existing.to_vec();
    };
    let mut had_block = false;
    for block in blocks {
        if block.id == DRD_ACHIEVEMENT_DATA {
            had_block = true;
            if !is_default {
                write_legacy_subscriber_block(
                    &mut encoded,
                    DRD_ACHIEVEMENT_DATA,
                    &encode_legacy_achievement_data(data),
                );
            }
        } else {
            write_legacy_subscriber_block(&mut encoded, block.id, block.data);
        }
    }
    if !had_block && !is_default {
        write_legacy_subscriber_block(
            &mut encoded,
            DRD_ACHIEVEMENT_DATA,
            &encode_legacy_achievement_data(data),
        );
    }
    encoded
}

/// Rewrites the `DRD_ACHIEVEMENT_STATS` block in the subscriber blob; see
/// `encode_legacy_achievement_data_subscriber_blob` for the pattern.
pub(crate) fn encode_legacy_achievement_stats_subscriber_blob(
    existing: &[u8],
    stats: &AchievementStats,
) -> Vec<u8> {
    let is_default = *stats == AchievementStats::default();
    let mut encoded = Vec::with_capacity(existing.len());
    let Some(blocks) = parse_legacy_subscriber_blocks(existing) else {
        return existing.to_vec();
    };
    let mut had_block = false;
    for block in blocks {
        if block.id == DRD_ACHIEVEMENT_STATS {
            had_block = true;
            if !is_default {
                write_legacy_subscriber_block(
                    &mut encoded,
                    DRD_ACHIEVEMENT_STATS,
                    &encode_legacy_achievement_stats(stats),
                );
            }
        } else {
            write_legacy_subscriber_block(&mut encoded, block.id, block.data);
        }
    }
    if !had_block && !is_default {
        write_legacy_subscriber_block(
            &mut encoded,
            DRD_ACHIEVEMENT_STATS,
            &encode_legacy_achievement_stats(stats),
        );
    }
    encoded
}
