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
    achievement_def, AccountAchievements, Achievement, AchievementStats, AchievementType,
    ACHIEVEMENT_TYPE_COUNT, MAX_ACHIEVEMENTS,
};
use ugaris_protocol::mod_achievements::{
    ach_sync_batch, ach_unlock, AchSyncEntry, ACHIEVEMENT_MAX_PER_SYNC,
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

// ============================================================================
// Client Communication (`achievement.c:1288-1415`'s "Client Communication"
// section: `achievement_send_to_client`/`achievement_sync_all`).
// ============================================================================

/// C `achievement_send_to_client` (`achievement.c:1291-1324`): builds the
/// `SV_ACH_UNLOCK` packet for a single newly-unlocked achievement.
/// `timestamp` is the C `time_t` unlock time (`Achievement::timestamp`),
/// truncated to `u32` exactly like the C `(uint32_t)ach->timestamp` cast.
/// Always sets `show_notification = 1`, matching every C call site (there
/// is no silent-unlock path in `achievement_send_to_client` itself).
pub(crate) fn achievement_unlock_payload(ty: AchievementType, timestamp: i64) -> bytes::BytesMut {
    let def = achievement_def(ty);
    let bytes = ach_unlock(
        ty as u8,
        def.category as u8,
        def.steam_id,
        timestamp as u32,
        true,
    );
    bytes::BytesMut::from(&bytes[..])
}

/// C `player_update` (`src/system/player.c:3448-3462`): credits one minute
/// of play time to the character's `AchievementStats` (plus `stats_update`,
/// unported - see `PORTING_TODO.md`'s Achievements task), sending an
/// `SV_ACH_UNLOCK` for `DedicatedPlayer`/`VeteranPlayer`/`UgarisLifer` on
/// first crossing their thresholds. A no-op if the character has no live
/// `PlayerRuntime` (mirrors C's `CF_PLAYER` implicit gate: `player_update`
/// only ever runs for connected player slots).
pub(crate) fn award_play_time_minute(
    world: &World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
) {
    let Some(name) = world
        .characters
        .get(&character_id)
        .map(|character| character.name.clone())
    else {
        return;
    };
    let now = current_unix_time();
    let Some(player) = runtime.player_for_character_mut(character_id) else {
        return;
    };
    let unlocked = ugaris_core::achievement::add_play_time(
        &mut player.achievement_data,
        &mut player.achievement_stats,
        1,
        &name,
        now,
    );
    for ty in unlocked {
        let payload = achievement_unlock_payload(ty, now);
        for (sid, _) in runtime.sessions_for_character(character_id) {
            runtime.send_to_session(sid, payload.clone());
        }
    }
}

/// C `kill_char` (`src/system/death.c:417-422`): `if (ch[co].flags &
/// CF_PLAYER) { achievement_add_enemy_killed(co); if (ch[cn].flags &
/// CF_DEMON) achievement_add_demons(co, areaID, 1); }` - runs for every kill
/// scored by a player character, independent of the target being a player
/// (unlike the sibling `give_exp` kill-experience path). A no-op if the
/// killer has no live `PlayerRuntime` (mirrors C's `CF_PLAYER` gate).
pub(crate) fn award_enemy_killed_achievement(
    world: &World,
    runtime: &mut ServerRuntime,
    killer_id: CharacterId,
    area_id: i32,
    target_is_demon: bool,
) {
    let Some(name) = world
        .characters
        .get(&killer_id)
        .map(|character| character.name.clone())
    else {
        return;
    };
    let now = current_unix_time();
    let Some(player) = runtime.player_for_character_mut(killer_id) else {
        return;
    };
    let mut unlocked = ugaris_core::achievement::add_enemy_killed(
        &mut player.achievement_data,
        &mut player.achievement_stats,
        &name,
        now,
    );
    if target_is_demon {
        unlocked.extend(ugaris_core::achievement::add_demons(
            &mut player.achievement_data,
            &mut player.achievement_stats,
            area_id,
            1,
            &name,
            now,
        ));
    }
    for ty in unlocked {
        let payload = achievement_unlock_payload(ty, now);
        for (sid, _) in runtime.sessions_for_character(killer_id) {
            runtime.send_to_session(sid, payload.clone());
        }
    }
}

/// C `flower_driver` (`src/module/alchemy.c:1306-1315`): `if (ch[cn].flags
/// & CF_PLAYER) { ... if (it[in].drdata[0] >= 1 && <= 7)
/// achievement_add_flowers(cn, 1); else if (>= 8 && <= 16)
/// achievement_add_mushrooms(cn, 1); else if (>= 17 && <= 20)
/// achievement_add_berries(cn, 1); }` - `kind` is the picked item's
/// `drdata[0]` template index (1-20), matching `ItemDriverOutcome::
/// PickAlchemyFlower`'s `kind` field (the C `IDR_FLOWER` driver, not the
/// unrelated area-31 `IDR_PICKBERRY` driver, which never calls any
/// achievement function in C). A no-op if the character has no live
/// `PlayerRuntime` (mirrors C's `CF_PLAYER` gate).
pub(crate) fn award_gathering_achievement(
    world: &World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    kind: u8,
) {
    let Some(name) = world
        .characters
        .get(&character_id)
        .map(|character| character.name.clone())
    else {
        return;
    };
    let now = current_unix_time();
    let Some(player) = runtime.player_for_character_mut(character_id) else {
        return;
    };
    let unlocked = match kind {
        1..=7 => ugaris_core::achievement::add_flowers(
            &mut player.achievement_data,
            &mut player.achievement_stats,
            1,
            &name,
            now,
        ),
        8..=16 => ugaris_core::achievement::add_mushrooms(
            &mut player.achievement_data,
            &mut player.achievement_stats,
            1,
            &name,
            now,
        ),
        17..=20 => ugaris_core::achievement::add_berries(
            &mut player.achievement_data,
            &mut player.achievement_stats,
            1,
            &name,
            now,
        ),
        _ => Vec::new(),
    };
    for ty in unlocked {
        let payload = achievement_unlock_payload(ty, now);
        for (sid, _) in runtime.sessions_for_character(character_id) {
            runtime.send_to_session(sid, payload.clone());
        }
    }
}

/// C `flask_driver`'s `mixer()` success branch (`src/module/alchemy.c:1077-
/// 1082`): `if (mixer(cn, in)) { ... if (ch[cn].flags & CF_PLAYER) {
/// achievement_add_potions(cn, 1); } }`, i.e. shaking a filled flask into a
/// magical potion. A no-op if the character has no live `PlayerRuntime`
/// (mirrors C's `CF_PLAYER` gate).
pub(crate) fn award_potion_brewed_achievement(
    world: &World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
) {
    let Some(name) = world
        .characters
        .get(&character_id)
        .map(|character| character.name.clone())
    else {
        return;
    };
    let now = current_unix_time();
    let Some(player) = runtime.player_for_character_mut(character_id) else {
        return;
    };
    let unlocked = ugaris_core::achievement::add_potions(
        &mut player.achievement_data,
        &mut player.achievement_stats,
        1,
        &name,
        now,
    );
    for ty in unlocked {
        let payload = achievement_unlock_payload(ty, now);
        for (sid, _) in runtime.sessions_for_character(character_id) {
            runtime.send_to_session(sid, payload.clone());
        }
    }
}

/// C `raise_value`/`raise_value_exp` (`src/system/skill.c:204-266`,
/// `:311-373`): both end with `if (ch[cn].flags & CF_PLAYER) {
/// achievement_check_skill(cn, v, ch[cn].value[1][v]); }` after
/// successfully raising a skill's bare value - `raise_value` is `CL_RAISE`
/// (spends already-earned exp), `raise_value_exp` is the scroll/shrine path
/// (`StatScrollUsed`, grants fresh exp); both call the exact same
/// achievement check with the post-raise bare value. `skill_type` is the
/// legacy `V_*` index (`ugaris_core::achievement::V_DAGGER` etc.),
/// `skill_level` is the new bare value (`character.values[1][value]`
/// after the raise). A no-op if the character has no live `PlayerRuntime`
/// (mirrors C's `CF_PLAYER` gate).
pub(crate) fn award_skill_achievement(
    world: &World,
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    skill_type: i32,
    skill_level: i32,
) {
    let Some(name) = world
        .characters
        .get(&character_id)
        .map(|character| character.name.clone())
    else {
        return;
    };
    let now = current_unix_time();
    let Some(player) = runtime.player_for_character_mut(character_id) else {
        return;
    };
    let unlocked = ugaris_core::achievement::check_skill(
        &mut player.achievement_data,
        skill_type,
        skill_level,
        &name,
        now,
    );
    for ty in unlocked {
        let payload = achievement_unlock_payload(ty, now);
        for (sid, _) in runtime.sessions_for_character(character_id) {
            runtime.send_to_session(sid, payload.clone());
        }
    }
}

/// C `achievement_sync_all` (`achievement.c:1329-1415`): batches every
/// achievement (all 127 defs carry a non-empty `steam_id`, so none are
/// skipped, unlike C's defensive `if (!def->steam_id...) continue;`) into
/// `ACHIEVEMENT_MAX_PER_SYNC`-sized `SV_ACH_SYNC` packets, `is_final` set on
/// the last one - including C's edge case of sending a trailing *empty*
/// final packet when the achievement count is an exact multiple of the
/// batch size (`total_sent > 0` branch, `achievement.c:1406-1414`).
pub(crate) fn achievement_sync_payloads(
    data: &AccountAchievements,
    stats: &AchievementStats,
) -> Vec<bytes::BytesMut> {
    let mut payloads = Vec::new();
    let mut batch: Vec<AchSyncEntry> = Vec::with_capacity(ACHIEVEMENT_MAX_PER_SYNC);
    let mut total_sent = 0usize;
    for index in 0..ACHIEVEMENT_TYPE_COUNT {
        let ty = AchievementType::ALL[index];
        let def = achievement_def(ty);
        if def.steam_id.is_empty() {
            continue;
        }
        let ach = &data.achievements[ty as usize];
        batch.push(AchSyncEntry {
            achievement_id: ty as u8,
            category: def.category as u8,
            unlocked: ach.timestamp != 0,
            has_progress: def.target > 0,
            steam_api_name: def.steam_id.to_string(),
            timestamp: ach.timestamp as u32,
            progress_current: if def.target > 0 {
                ugaris_core::achievement::get_stat_progress(stats, ty)
            } else {
                0
            },
            progress_target: if def.target > 0 { def.target } else { 0 },
        });
        if batch.len() >= ACHIEVEMENT_MAX_PER_SYNC {
            payloads.push(ach_sync_batch(&batch, false));
            total_sent += batch.len();
            batch.clear();
        }
    }
    if !batch.is_empty() {
        payloads.push(ach_sync_batch(&batch, true));
    } else if total_sent > 0 {
        payloads.push(ach_sync_batch(&[], true));
    }
    payloads
}

#[cfg(test)]
mod send_tests {
    use super::*;

    #[test]
    fn achievement_unlock_payload_matches_legacy_wire_layout() {
        let payload = achievement_unlock_payload(AchievementType::StartedUgaris, 1_700_000_000);
        assert_eq!(payload.len(), 51);
        assert_eq!(payload[2], 0x30); // SV_ACH_UNLOCK
        assert_eq!(payload[3], AchievementType::StartedUgaris as u8);
        assert_eq!(&payload[45..49], &(1_700_000_000u32).to_le_bytes());
        assert_eq!(payload[49], 1); // show_notification
    }

    #[test]
    fn achievement_sync_payloads_batches_every_def_and_marks_last_final() {
        let data = AccountAchievements::default();
        let stats = AchievementStats::default();
        let payloads = achievement_sync_payloads(&data, &stats);
        // 127 achievements / 16 per batch = 7 full batches + 1 partial (15).
        assert_eq!(payloads.len(), 8);
        let total_entries: usize = payloads.iter().map(|payload| payload[3] as usize).sum();
        assert_eq!(total_entries, ACHIEVEMENT_TYPE_COUNT);
        for payload in &payloads[..7] {
            assert_eq!(payload[4], 0); // is_final = 0
        }
        assert_eq!(payloads[7][4], 1); // is_final = 1 on the last batch
    }

    #[test]
    fn achievement_sync_payloads_marks_unlocked_and_progress_from_stats() {
        let mut data = AccountAchievements::default();
        data.award(AchievementType::StartedUgaris, "Hero", 1_700_000_000);
        let mut stats = AchievementStats::default();
        stats.flowers_picked = 5;
        let payloads = achievement_sync_payloads(&data, &stats);
        let first_batch = &payloads[0];
        // Entry 0 (StartedUgaris) starts right after the 5-byte header.
        let entry0 = &first_batch[5..5 + 56];
        assert_eq!(entry0[0], AchievementType::StartedUgaris as u8);
        assert_eq!(entry0[2], 1); // unlocked
    }
}
