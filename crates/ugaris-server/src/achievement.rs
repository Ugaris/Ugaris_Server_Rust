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
use ugaris_db::AchievementRepository;
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

// C: only used by the retired `encode_legacy_achievement_*_subscriber_blob`
// encoders below and their round-trip test coverage now (migration 0020's
// `player_state_json` is the sole write target - see the "Retire legacy
// blob writes" PORTING_TODO.md task), hence `#[allow(dead_code)]` in
// non-test builds.
#[allow(dead_code)]
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

#[allow(dead_code)]
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
/// Read fallback only (migration 0020's `player_state_json` is
/// authoritative now; `ugaris-server`'s `snapshots.rs` no longer writes
/// `subscriber_blob`). Kept for pre-0020 rows that haven't been backfilled
/// yet - see the "Retire legacy blob writes" `PORTING_TODO.md` task.
#[deprecated(note = "read-fallback for pre-migration-0020 rows only")]
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
/// Read fallback only (migration 0020's `player_state_json` is
/// authoritative now; `ugaris-server`'s `snapshots.rs` no longer writes
/// `subscriber_blob`). Kept for pre-0020 rows that haven't been backfilled
/// yet - see the "Retire legacy blob writes" `PORTING_TODO.md` task.
#[deprecated(note = "read-fallback for pre-migration-0020 rows only")]
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
///
/// No longer called from any save path (migration 0020's
/// `player_state_json` is authoritative - see the "Retire legacy blob
/// writes" `PORTING_TODO.md` task); kept for round-trip test coverage of
/// [`decode_legacy_achievement_data_subscriber_blob`]'s byte layout.
#[allow(dead_code)]
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
/// `encode_legacy_achievement_data_subscriber_blob` for the pattern
/// (including the "no longer called from any save path" note).
#[allow(dead_code)]
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
/// only ever runs for connected player slots). Also records the DB
/// first-unlock/grats-announce tail (`record_achievement_firsts_and_
/// announce`) for anything newly unlocked, matching C's `achievement_
/// award` doing both in one call.
pub(crate) async fn award_play_time_minute(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
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
    for ty in &unlocked {
        let payload = achievement_unlock_payload(*ty, now);
        for (sid, _) in runtime.sessions_for_character(character_id) {
            runtime.send_to_session(sid, payload.clone());
        }
    }
    record_achievement_firsts_and_announce(world, repository, character_id, &name, &unlocked).await;
}

/// C `kill_char` (`src/system/death.c:417-422`): `if (ch[co].flags &
/// CF_PLAYER) { achievement_add_enemy_killed(co); if (ch[cn].flags &
/// CF_DEMON) achievement_add_demons(co, areaID, 1); }` - runs for every kill
/// scored by a player character, independent of the target being a player
/// (unlike the sibling `give_exp` kill-experience path). A no-op if the
/// killer has no live `PlayerRuntime` (mirrors C's `CF_PLAYER` gate). Also
/// records the DB first-unlock/grats-announce tail for anything newly
/// unlocked (see `award_play_time_minute`'s doc comment).
pub(crate) async fn award_enemy_killed_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
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
    for ty in &unlocked {
        let payload = achievement_unlock_payload(*ty, now);
        for (sid, _) in runtime.sessions_for_character(killer_id) {
            runtime.send_to_session(sid, payload.clone());
        }
    }
    record_achievement_firsts_and_announce(world, repository, killer_id, &name, &unlocked).await;
}

/// C `check_levelup` (`src/system/tool.c:1352-1354`): `if (ch[cn].flags &
/// CF_PLAYER) { achievement_check_level(cn, ch[cn].level); }`, fired once
/// per level gained (queued as [`ugaris_core::LevelAchievementCheck`] by
/// `World::check_levelup`, already `CharacterFlags::PLAYER`-gated there). A
/// no-op if the character has no live `PlayerRuntime` (mirrors C's
/// `CF_PLAYER` gate as a defense in depth, even though the queue is already
/// filtered). Also records the DB first-unlock/grats-announce tail for
/// anything newly unlocked (see `award_play_time_minute`'s doc comment).
pub(crate) async fn award_level_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    character_id: CharacterId,
    level: i32,
    is_hardcore: bool,
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
    let unlocked = ugaris_core::achievement::check_level(
        &mut player.achievement_data,
        level,
        is_hardcore,
        &name,
        now,
    );
    for ty in &unlocked {
        let payload = achievement_unlock_payload(*ty, now);
        for (sid, _) in runtime.sessions_for_character(character_id) {
            runtime.send_to_session(sid, payload.clone());
        }
    }
    record_achievement_firsts_and_announce(world, repository, character_id, &name, &unlocked).await;
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
/// `PlayerRuntime` (mirrors C's `CF_PLAYER` gate). Also records the DB
/// first-unlock/grats-announce tail for anything newly unlocked (see
/// `award_play_time_minute`'s doc comment).
pub(crate) async fn award_gathering_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
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
    for ty in &unlocked {
        let payload = achievement_unlock_payload(*ty, now);
        for (sid, _) in runtime.sessions_for_character(character_id) {
            runtime.send_to_session(sid, payload.clone());
        }
    }
    record_achievement_firsts_and_announce(world, repository, character_id, &name, &unlocked).await;
}

/// C `flask_driver`'s `mixer()` success branch (`src/module/alchemy.c:1077-
/// 1082`): `if (mixer(cn, in)) { ... if (ch[cn].flags & CF_PLAYER) {
/// achievement_add_potions(cn, 1); } }`, i.e. shaking a filled flask into a
/// magical potion. A no-op if the character has no live `PlayerRuntime`
/// (mirrors C's `CF_PLAYER` gate). Also records the DB first-unlock/
/// grats-announce tail for anything newly unlocked (see `award_play_time_
/// minute`'s doc comment).
pub(crate) async fn award_potion_brewed_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
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
    for ty in &unlocked {
        let payload = achievement_unlock_payload(*ty, now);
        for (sid, _) in runtime.sessions_for_character(character_id) {
            runtime.send_to_session(sid, payload.clone());
        }
    }
    record_achievement_firsts_and_announce(world, repository, character_id, &name, &unlocked).await;
}

/// C `handle_silver_find`'s `if (ch[cn].flags & CF_PLAYER)
/// achievement_add_silver_mined(cn, amount);` tail (`src/area/12/
/// mine.c:299-301`). A no-op if the character has no live `PlayerRuntime`
/// (mirrors C's `CF_PLAYER` gate) or `amount == 0`. Also records the DB
/// first-unlock/grats-announce tail (see `award_play_time_minute`'s doc
/// comment).
pub(crate) async fn award_silver_mined_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    character_id: CharacterId,
    amount: u32,
) {
    if amount == 0 {
        return;
    }
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
    let unlocked = ugaris_core::achievement::add_silver_mined(
        &mut player.achievement_data,
        &mut player.achievement_stats,
        amount,
        &name,
        now,
    );
    for ty in &unlocked {
        let payload = achievement_unlock_payload(*ty, now);
        for (sid, _) in runtime.sessions_for_character(character_id) {
            runtime.send_to_session(sid, payload.clone());
        }
    }
    record_achievement_firsts_and_announce(world, repository, character_id, &name, &unlocked).await;
}

/// C `handle_gold_find`'s `if (ch[cn].flags & CF_PLAYER)
/// achievement_add_gold_mined(cn, amount);` tail (`src/area/12/
/// mine.c:313-315`), same shape as [`award_silver_mined_achievement`].
pub(crate) async fn award_gold_mined_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    character_id: CharacterId,
    amount: u32,
) {
    if amount == 0 {
        return;
    }
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
    let unlocked = ugaris_core::achievement::add_gold_mined(
        &mut player.achievement_data,
        &mut player.achievement_stats,
        amount,
        &name,
        now,
    );
    for ty in &unlocked {
        let payload = achievement_unlock_payload(*ty, now);
        for (sid, _) in runtime.sessions_for_character(character_id) {
            runtime.send_to_session(sid, payload.clone());
        }
    }
    record_achievement_firsts_and_announce(world, repository, character_id, &name, &unlocked).await;
}

/// C `give_reward`'s `achievement_add_tunnel_level(cn);` tail
/// (`src/area/33/tunnel.c:557`), fired once per successfully-rewarded
/// `IDR_TUNNELDOOR` exit-pillar use. Same shape as
/// [`award_silver_mined_achievement`] minus the `amount` parameter - C's
/// own `achievement_add_tunnel_level` takes none either, it just
/// increments `stats->tunnel_levels` by 1.
pub(crate) async fn award_tunnel_level_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
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
    let unlocked = ugaris_core::achievement::add_tunnel_level(
        &mut player.achievement_data,
        &mut player.achievement_stats,
        &name,
        now,
    );
    for ty in &unlocked {
        let payload = achievement_unlock_payload(*ty, now);
        for (sid, _) in runtime.sessions_for_character(character_id) {
            runtime.send_to_session(sid, payload.clone());
        }
    }
    record_achievement_firsts_and_announce(world, repository, character_id, &name, &unlocked).await;
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
/// (mirrors C's `CF_PLAYER` gate). Also records the DB first-unlock/
/// grats-announce tail for anything newly unlocked (see `award_play_time_
/// minute`'s doc comment).
pub(crate) async fn award_skill_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
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
    for ty in &unlocked {
        let payload = achievement_unlock_payload(*ty, now);
        for (sid, _) in runtime.sessions_for_character(character_id) {
            runtime.send_to_session(sid, payload.clone());
        }
    }
    record_achievement_firsts_and_announce(world, repository, character_id, &name, &unlocked).await;
}

/// C `give_money` (`src/system/tool.c:1459-1483`): adds `amount` silver to
/// the character's gold pouch, sets `CF_ITEMS`, and sends the "You
/// received ... It has been placed in your gold pouch." notice (colored
/// exactly like C's `COL_YELLOW`/`COL_RESET`-wrapped amount, `"%ds"` under
/// 100 silver, `"%.2fG"` otherwise). If `amount > 0` and the character has
/// a live `PlayerRuntime` (mirrors C's `CF_PLAYER` gate), also tracks the
/// `achievement_add_gold_earned` wealth ladder (`CoinCollector`/
/// `WealthyAdventurer`/`RichNoble`/`Millionaire`) with the silver amount
/// converted to whole gold units (`amount / 100`, integer division,
/// exactly like C's `(unsigned int)(val / 100)` cast). C's trailing `if
/// (val != 0) macro_track_gold_change(cn)` is applied here too (stamping
/// `MacroPpd::last_gold_change` directly, since this function already
/// has both `World` and `ServerRuntime` in scope - unlike
/// `World::gate_give_money_silent`, which has to queue it for
/// `ugaris-server`'s `apply_macro_activity_events` instead). `dlog` has no Rust
/// equivalent yet. Also records the DB first-unlock/
/// grats-announce tail for anything newly unlocked (see `award_play_time_
/// minute`'s doc comment).
pub(crate) async fn give_money(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    character_id: CharacterId,
    amount: u32,
    feedback_bytes: &mut Vec<(CharacterId, Vec<u8>)>,
) {
    let Some(character) = world.characters.get_mut(&character_id) else {
        return;
    };
    character.gold = character.gold.saturating_add(amount);
    character.flags.insert(CharacterFlags::ITEMS);
    let name = character.name.clone();

    let gold_str = if amount < 100 {
        format!("{amount}s")
    } else {
        format!("{:.2}G", f64::from(amount) / 100.0)
    };
    let mut message = Vec::with_capacity(64);
    message.extend_from_slice(b"You received");
    message.extend_from_slice(COL_YELLOW);
    message.push(b' ');
    message.extend_from_slice(gold_str.as_bytes());
    message.extend_from_slice(COL_RESET);
    message.extend_from_slice(b". It has been placed in your gold pouch.");
    feedback_bytes.push((character_id, message));

    if amount == 0 {
        return;
    }
    let now = current_unix_time();
    let Some(player) = runtime.player_for_character_mut(character_id) else {
        return;
    };
    player.macro_ppd.last_gold_change = now;
    let unlocked = ugaris_core::achievement::add_gold_earned(
        &mut player.achievement_data,
        &mut player.achievement_stats,
        amount / 100,
        &name,
        now,
    );
    for ty in &unlocked {
        let payload = achievement_unlock_payload(*ty, now);
        for (sid, _) in runtime.sessions_for_character(character_id) {
            runtime.send_to_session(sid, payload.clone());
        }
    }
    record_achievement_firsts_and_announce(world, repository, character_id, &name, &unlocked).await;
}

/// C `swap`'s `IF_MONEY` branch (`src/system/do.c:1276-1287`): dropping a
/// held money item into any inventory slot destroys it and credits its
/// value straight to `ch[cn].gold` instead of ever occupying a slot (the
/// gold-credit itself already happened synchronously in
/// `inventory_swap_slot`, which returned `MoneyConverted { price }`).
/// This helper handles the remaining `CF_PLAYER`-gated tail: `achievement_
/// add_gold_earned(cn, (unsigned int)(price / 100))` (integer division,
/// exactly like C's cast). `stats_update(cn, 0, price)` and the `dlog`
/// call have no Rust equivalent yet (same omission as `give_money`'s doc
/// comment). A no-op if the character has no live `PlayerRuntime`. Also
/// records the DB first-unlock/grats-announce tail for anything newly
/// unlocked (see `award_play_time_minute`'s doc comment).
pub(crate) async fn award_swap_money_converted_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    character_id: CharacterId,
    price: u32,
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
    let unlocked = ugaris_core::achievement::add_gold_earned(
        &mut player.achievement_data,
        &mut player.achievement_stats,
        price / 100,
        &name,
        now,
    );
    for ty in &unlocked {
        let payload = achievement_unlock_payload(*ty, now);
        for (sid, _) in runtime.sessions_for_character(character_id) {
            runtime.send_to_session(sid, payload.clone());
        }
    }
    record_achievement_firsts_and_announce(world, repository, character_id, &name, &unlocked).await;
}

/// C `act_take` (`src/system/act.c:305-327`)'s stone-pickup block, which
/// only runs when `keyring_try_auto_add` did *not* consume the item (that
/// branch `return`s early in C before reaching this check): `if
/// (it[in].ID == IID_ALCHEMY_INGREDIENT) { int stone_drdata =
/// it[in].drdata[0]; if (stone_drdata == 23 || stone_drdata == 24)
/// achievement_add_stones(cn, 0, 1); else if (stone_drdata == 21)
/// achievement_add_stones(cn, 1, 1); else if (stone_drdata == 22)
/// achievement_add_stones(cn, 2, 1); }`. `stone_drdata` is the picked
/// item's `drdata[0]` (`Item::driver_data[0]`); 23/24 = Earth, 21 = Fire,
/// 22 = Ice - a no-op for any other value. A no-op if the character has no
/// live `PlayerRuntime` (mirrors C's `CF_PLAYER` gate). Also records the DB
/// first-unlock/grats-announce tail for anything newly unlocked (see
/// `award_play_time_minute`'s doc comment).
pub(crate) async fn award_stone_pickup_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    character_id: CharacterId,
    stone_drdata: u8,
) {
    let stone_type = match stone_drdata {
        23 | 24 => ugaris_core::achievement::STONE_TYPE_EARTH,
        21 => ugaris_core::achievement::STONE_TYPE_FIRE,
        22 => ugaris_core::achievement::STONE_TYPE_ICE,
        _ => return,
    };
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
    let unlocked = ugaris_core::achievement::add_stones(
        &mut player.achievement_data,
        &mut player.achievement_stats,
        stone_type,
        1,
        &name,
        now,
    );
    for ty in &unlocked {
        let payload = achievement_unlock_payload(*ty, now);
        for (sid, _) in runtime.sessions_for_character(character_id) {
            runtime.send_to_session(sid, payload.clone());
        }
    }
    record_achievement_firsts_and_announce(world, repository, character_id, &name, &unlocked).await;
}

/// Shared helper for every call site that mirrors C calling the bare
/// `achievement_award` primitive directly (no stat-based `add_*` helper
/// exists in C for these - `TrustButVerify`, `SlayerOfDemonLords` - just a
/// direct `achievement_award(cn, TYPE, 1)` call), matching `main.rs`'s
/// `Quester` award call site's shape too: awards `ty` to a single
/// character via `AccountAchievements::award`, sends the unlock packet to
/// every session for that character, and records the DB
/// first-unlock/grats-announce tail. A no-op if the character has no live
/// `PlayerRuntime` (mirrors C's `CF_PLAYER` gate - `find_char_byID`
/// returning nothing is C's equivalent no-op path).
async fn award_bare_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    character_id: CharacterId,
    ty: AchievementType,
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
    if !player.achievement_data.award(ty, &name, now) {
        return;
    }
    let payload = achievement_unlock_payload(ty, now);
    for (sid, _) in runtime.sessions_for_character(character_id) {
        runtime.send_to_session(sid, payload.clone());
    }
    record_achievement_firsts_and_announce(world, repository, character_id, &name, &[ty]).await;
}

/// C `trader_driver`'s "accept trade" success branch (`src/module/
/// base.c:4416-4428`): once both sides have accepted a trade,
/// `achievement_award(c1, ACHIEVEMENT_TRUST_BUT_VERIFY, 1)` and
/// `achievement_award(c2_trader, ACHIEVEMENT_TRUST_BUT_VERIFY, 1)` fire
/// independently for both traders (C guards each with its own
/// `find_char_byID` null check - our per-character no-op inside
/// `award_bare_achievement` is the equivalent). Consumes a
/// `TraderEvent::DealCompleted` queued by `World::process_trader_actions`.
pub(crate) async fn award_trader_deal_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    c1_id: CharacterId,
    c2_id: CharacterId,
) {
    award_bare_achievement(
        world,
        runtime,
        repository,
        c1_id,
        AchievementType::TrustButVerify,
    )
    .await;
    award_bare_achievement(
        world,
        runtime,
        repository,
        c2_id,
        AchievementType::TrustButVerify,
    )
    .await;
}

/// C `add_member`'s `ACHIEVEMENT_CLAN_MEMBER` award (`clan.c:1199-1205`,
/// `cnr < CLUBOFFSET` is always true for this driver - see
/// `crate::clanmaster`'s module doc comment) plus, for founding, the
/// clanmaster NPC's own explicit `ACHIEVEMENT_CLAN_MASTER` award
/// (`src/area/30/clanmaster.c:567`). Consumes a `ClanmasterEvent` queued
/// by `World::process_clanmaster_actions`.
pub(crate) async fn award_clanmaster_member_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    member_id: CharacterId,
) {
    award_bare_achievement(
        world,
        runtime,
        repository,
        member_id,
        AchievementType::ClanMember,
    )
    .await;
}

pub(crate) async fn award_clanmaster_master_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    founder_id: CharacterId,
) {
    award_bare_achievement(
        world,
        runtime,
        repository,
        founder_id,
        AchievementType::ClanMaster,
    )
    .await;
}

/// C `clubmaster_driver`'s `found:` success branch's `ACHIEVEMENT_CLUB_
/// MEMBER` award (`src/system/clubmaster.c:305`). Consumes a
/// `ClubmasterEvent` queued by `World::process_clubmaster_actions`.
pub(crate) async fn award_clubmaster_member_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    member_id: CharacterId,
) {
    award_bare_achievement(
        world,
        runtime,
        repository,
        member_id,
        AchievementType::ClubMember,
    )
    .await;
}

/// C `clubmaster_driver`'s `found:` success branch's `ACHIEVEMENT_CLUB_
/// MASTER` award (`src/system/clubmaster.c:306`).
pub(crate) async fn award_clubmaster_master_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    founder_id: CharacterId,
) {
    award_bare_achievement(
        world,
        runtime,
        repository,
        founder_id,
        AchievementType::ClubMaster,
    )
    .await;
}

/// C `lydia_driver`'s hangover-potion turn-in: `achievement_award(co,
/// ACHIEVEMENT_A_HELPING_HAND, 1)` (`src/area/1/gwendylon.c:3607`).
/// Consumes a `LydiaOutcomeEvent::QuestDone` queued by
/// `World::process_lydia_actions`.
pub(crate) async fn award_lydia_helping_hand_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    player_id: CharacterId,
) {
    award_bare_achievement(
        world,
        runtime,
        repository,
        player_id,
        AchievementType::AHelpingHand,
    )
    .await;
}

/// C `reskin_driver`'s alchemy-ingredient turn-in: `achievement_award(co,
/// ACHIEVEMENT_WELL_PAID_GATHERER, 1)` (`src/area/1/gwendylon.c:4351`),
/// fired once every alchemy-ingredient type (1-24) has been turned in.
/// Consumes a `ReskinOutcomeEvent::WellPaidGathererAchievement` queued by
/// `World::process_reskin_actions`.
pub(crate) async fn award_reskin_well_paid_gatherer_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    player_id: CharacterId,
) {
    award_bare_achievement(
        world,
        runtime,
        repository,
        player_id,
        AchievementType::WellPaidGatherer,
    )
    .await;
}

/// C `carlos_driver`'s dragon-staff turn-in: `achievement_award(co,
/// ACHIEVEMENT_DRAGONSBANE, 1)` (`src/area/3/area3.c:2267`), fired every
/// time (quest 20 is `QLF_REPEATABLE`, and C's call is unconditional, not
/// gated on first completion). Consumes a `CarlosOutcomeEvent::
/// DragonStaffQuestDone` queued by `World::process_carlos_actions`.
/// C `islena_dead`'s `achievement_award(co, ACHIEVEMENT_LADYKILLER, 1)`
/// (`src/area/11/palace.c:765`), fired the first time a player slays
/// Islena. Consumes a `CharacterId` queued by `World::apply_islena_death`
/// into `pending_islena_ladykiller_awards`, drained once per tick by
/// `crate::area11::process_islena_ladykiller_awards`.
pub(crate) async fn award_islena_ladykiller_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    player_id: CharacterId,
) {
    award_bare_achievement(
        world,
        runtime,
        repository,
        player_id,
        AchievementType::Ladykiller,
    )
    .await;
}

pub(crate) async fn award_dragonsbane_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    player_id: CharacterId,
) {
    award_bare_achievement(
        world,
        runtime,
        repository,
        player_id,
        AchievementType::Dragonsbane,
    )
    .await;
}

/// C `guard_brannington_driver`'s `achievement_award(co,
/// ACHIEVEMENT_GREAT_EXPLORER, 1)` (`src/area/29/brannington.c:1942`),
/// fired unconditionally alongside `questlog_done(co, 64)` once "Finding
/// Arkhata" completes. Consumes `world::npc::area29::guardbran::
/// GuardBranOutcomeEvent::QuestDone`, applied by `ugaris-server`'s
/// `apply_guardbran_events`.
pub(crate) async fn award_great_explorer_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    player_id: CharacterId,
) {
    award_bare_achievement(
        world,
        runtime,
        repository,
        player_id,
        AchievementType::GreatExplorer,
    )
    .await;
}

/// C `handle_lucky_pentagram`'s `achievement_award(player_id,
/// ACHIEVEMENT_HAPPY_GO_LUCKY, 1)` (`src/area/4/pents.c:853`), fired every
/// time a player hits a lucky pentagram roll. Consumes the `lucky_hit`
/// flag on `ugaris_core::pentagram::AddPentagramOutcome`, queued by
/// `crate::pents::process_pentagram_activations`.
pub(crate) async fn award_pentagram_lucky_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    player_id: CharacterId,
) {
    award_bare_achievement(
        world,
        runtime,
        repository,
        player_id,
        AchievementType::HappyGoLucky,
    )
    .await;
}

/// C `handle_lucky_pentagram`'s second branch: `achievement_award(player_id,
/// ACHIEVEMENT_FAVORED_BY_FORTUNE, 1)` (`pents.c:856-858`), fired when a
/// player hits a second lucky pentagram within the same solve run.
/// Consumes the `second_lucky_hit` flag on `AddPentagramOutcome`.
pub(crate) async fn award_pentagram_favored_by_fortune_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    player_id: CharacterId,
) {
    award_bare_achievement(
        world,
        runtime,
        repository,
        player_id,
        AchievementType::FavoredByFortune,
    )
    .await;
}

/// C `distribute_rewards_to_player`'s `if (player_data->status == 1)
/// achievement_award(player_id, ACHIEVEMENT_FIVE_IN_A_ROW, 1)`
/// (`pents.c:606-609`), fired for every reward-eligible player whose
/// pentagram data had a live five-color combo at solve time. Consumes the
/// `had_combo` flag from `ugaris_core::pentagram::distribute_rewards_reset`.
pub(crate) async fn award_pentagram_five_in_a_row_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    player_id: CharacterId,
) {
    award_bare_achievement(
        world,
        runtime,
        repository,
        player_id,
        AchievementType::FiveInARow,
    )
    .await;
}

/// C `distribute_rewards_to_player`'s trailing `achievement_add_pents(
/// player_id, areaID, 1)` (`pents.c:646`), fired unconditionally for every
/// reward-eligible player on a solve (not just the solver) - mirrors
/// `award_enemy_killed_achievement`'s `add_demons` call for the kill-side
/// equivalent.
pub(crate) async fn award_pentagram_solve_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    player_id: CharacterId,
    area_id: i32,
) {
    let Some(name) = world
        .characters
        .get(&player_id)
        .map(|character| character.name.clone())
    else {
        return;
    };
    let now = current_unix_time();
    let Some(player) = runtime.player_for_character_mut(player_id) else {
        return;
    };
    let unlocked = ugaris_core::achievement::add_pents(
        &mut player.achievement_data,
        &mut player.achievement_stats,
        area_id,
        1,
        &name,
        now,
    );
    for ty in &unlocked {
        let payload = achievement_unlock_payload(*ty, now);
        for (sid, _) in runtime.sessions_for_character(player_id) {
            runtime.send_to_session(sid, payload.clone());
        }
    }
    record_achievement_firsts_and_announce(world, repository, player_id, &name, &unlocked).await;
}

/// C `handle_demon_death`'s `achievement_award(killer_id,
/// ACHIEVEMENT_DEMON_LORDS_DEMISE, 1)` (`src/area/4/pents.c:1402`), fired
/// when a player lands the killing blow on a `CDR_PENTER` demon whose
/// class fell in the demon-lord power-reduction range. Consumes a
/// `CharacterId` queued by `World::apply_penter_demon_death`, drained by
/// `crate::pents::process_penter_demon_lords_demise_awards`.
pub(crate) async fn award_pentagram_demon_lords_demise_achievement(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    player_id: CharacterId,
) {
    award_bare_achievement(
        world,
        runtime,
        repository,
        player_id,
        AchievementType::DemonLordsDemise,
    )
    .await;
}

/// C `give_first_kill`'s class-range congrats-message dispatch
/// (`death.c:213-253`). `has_name` gates on `ch[co].flags & CF_HASNAME`
/// (checked before any class range); the two subsequent `else if` chains
/// are mutually exclusive class-id ranges copied digit-for-digit from the
/// C source, and anything matching neither falls through to the generic
/// "first kill" message.
fn is_named_monster_first_kill_class(class: i32) -> bool {
    matches!(
        class,
        52..=84
            | 85..=100
            | 101..=106
            | 107..=138
            | 139..=170
            | 172..=181
            | 183..=202
            | 204..=214
            | 215..=220
            | 221..=228
            | 229..=236
            | 237..=244
            | 336..=365
            | 388..=403
    )
}

/// C `give_first_kill`'s demon-lord class ranges (`death.c:238`): Earth/
/// Fire/Ice demon lords (`258..=305`) and Hell demon lords (`404..=411`) -
/// the same ranges `count_demon_lord_kills` sums over.
fn is_demon_lord_first_kill_class(class: i32) -> bool {
    matches!(class, 258..=305 | 404..=411)
}

/// C `give_first_kill`'s `log_char` congrats text (`death.c:213-253`). The
/// demon-lord branch's `get_army_rank_int(cn)` check is `has_army_rank`
/// here (the killer's rank, derived from `Character.military_points` via
/// `army_rank_for_points` at the call site - see `apply_first_kill_check`).
fn first_kill_congrats_message(
    class: i32,
    has_name: bool,
    level: u32,
    name: &str,
    has_army_rank: bool,
) -> String {
    if has_name {
        format!("You just killed {name} for the first time. Congratulations!")
    } else if is_named_monster_first_kill_class(class) {
        format!("You just killed your first level {level} {name}. Congratulations!")
    } else if is_demon_lord_first_kill_class(class) {
        if has_army_rank {
            format!(
                "You just killed your first level {level} {name}! The Governor will be proud of you."
            )
        } else {
            format!("You just killed your first level {level} {name}!")
        }
    } else {
        format!("You just killed your first {name}. Congratulations!")
    }
}

/// C `give_first_kill` (`src/system/death.c:196-254`): the killer-side
/// (`cn`) half of `kill_char`'s first-kill tracking, queued as a
/// [`FirstKillCheck`] by `World::kill_character_followup` whenever a
/// player kills an NPC with `ch.class` in `1..=1023` (the guard clauses -
/// `CF_PLAYER`, the class range - already ran there; this only re-runs the
/// bit-test/set + reward/message + achievement tail, which needs the
/// server-owned `PlayerRuntime::first_kill_ppd`). A no-op if the killer
/// has no live `PlayerRuntime`, or if this class was already recorded
/// (`PlayerRuntime::mark_first_kill` returning `false`, C's `if (ppd->
/// kill[index] & mask) return;`).
pub(crate) async fn apply_first_kill_check(
    world: &mut World,
    runtime: &mut ServerRuntime,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    area_id: i32,
    check: FirstKillCheck,
) {
    let Some((killer_level, killer_army_rank)) =
        world.characters.get(&check.killer_id).map(|character| {
            (
                character.level,
                army_rank_for_points(character.military_points),
            )
        })
    else {
        return;
    };
    let demon_lord_kills = {
        let Some(player) = runtime.player_for_character_mut(check.killer_id) else {
            return;
        };
        if !player.mark_first_kill(check.victim_class) {
            return;
        }
        player.count_demon_lord_kills()
    };

    // C: give_exp(cn, kill_score(co, cn) * 5);
    let bonus_exp = i64::from(ugaris_core::attack::kill_score_level(
        check.victim_level,
        killer_level,
    )) * 5;
    world.give_exp(check.killer_id, bonus_exp, area_id as u32);

    let message = first_kill_congrats_message(
        check.victim_class,
        check.victim_has_name,
        check.victim_level,
        &check.victim_name,
        killer_army_rank > 0,
    );
    world.queue_system_text(check.killer_id, message);

    // C: `if (get_army_rank_int(cn)) { ...; give_military_pts_no_npc(cn,
    // min(ch[co].level / 3, 10), kill_score(co, cn) * 15); }` - only
    // reachable inside the demon-lord class-range branch (`death.c:
    // 238-244`), gated on the killer already holding a rank above
    // "nobody".
    if is_demon_lord_first_kill_class(check.victim_class) && killer_army_rank > 0 {
        let pts = (check.victim_level / 3).min(10) as i32;
        let exps =
            ugaris_core::attack::kill_score_level(check.victim_level, killer_level) as i32 * 15;
        world.give_military_pts(check.killer_id, pts, exps, area_id as u32);
    }

    // C: `if (count_demon_lord_kills(ppd) >= 20) achievement_award(cn,
    // ACHIEVEMENT_SLAYER_OF_DEMON_LORDS, 1);` - only reachable inside the
    // demon-lord class-range branch in C (`death.c:238-247`), so the class
    // check is required here too, not just the count.
    if is_demon_lord_first_kill_class(check.victim_class) && demon_lord_kills >= 20 {
        award_bare_achievement(
            world,
            runtime,
            repository,
            check.killer_id,
            AchievementType::SlayerOfDemonLords,
        )
        .await;
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

/// C `achievement_award`'s DB-tracking + cross-server announce tail
/// (`src/module/achievements/achievement.c:610-631`): `subscriber_id =
/// get_subscriberId_from_character(cn); if (subscriber_id > 0) is_first =
/// db_achievement_record_unlock(type, def->name, subscriber_id,
/// ch[cn].name); if (is_first) achievement_announce_first(ch[cn].name,
/// def->name);`. This codebase has no live subscriber/account model
/// (`db_achievement_record_unlock`'s `subscriber_id` param is filled with
/// `character_id` instead - see `migrations/0007_achievement_firsts.sql`),
/// so the `subscriber_id > 0` gate is dropped (always "true" here).
///
/// A no-op when no `--database-url` was configured (`repository is
/// None`), matching the rest of this codebase's "persistence is optional"
/// convention - C always has a database connection, so there is no direct
/// analog to "gate skipped" here, just "feature unavailable".
///
/// Called once per newly-unlocked achievement in `unlocked`, in order,
/// after the caller has already queued/sent the `SV_ACH_UNLOCK` packet(s)
/// for them (this function only handles the DB record + the
/// `achievement_announce_first` broadcast, not the client packet).
pub(crate) async fn record_achievement_firsts_and_announce(
    world: &mut World,
    repository: &Option<ugaris_db::PgAchievementRepository>,
    character_id: CharacterId,
    character_name: &str,
    unlocked: &[AchievementType],
) {
    let Some(repository) = repository else {
        return;
    };
    for &ty in unlocked {
        let def = achievement_def(ty);
        let is_first = match repository
            .record_unlock(ty as i32, def.name, character_id, character_name)
            .await
        {
            Ok(is_first) => is_first,
            Err(err) => {
                tracing::warn!(
                    character = character_name,
                    achievement = def.name,
                    error = %err,
                    "failed to record achievement-firsts DB row"
                );
                continue;
            }
        };
        if is_first {
            // C `achievement_announce_first` (`achievement.c:540-549`):
            // `"0000000000" COL_MAUVE "Grats: %s is the FIRST to unlock
            // %s!"` on channel 6.
            let mut message = b"0000000000".to_vec();
            message.extend_from_slice(COL_MAUVE);
            message.extend_from_slice(
                format!(
                    "Grats: {character_name} is the FIRST to unlock {}!",
                    def.name
                )
                .as_bytes(),
            );
            world.queue_channel_broadcast(6, message);
        }
    }
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

    #[tokio::test]
    async fn record_achievement_firsts_and_announce_is_a_no_op_without_database() {
        let mut world = World::default();
        record_achievement_firsts_and_announce(
            &mut world,
            &None,
            CharacterId(1),
            "Hero",
            &[AchievementType::StartedUgaris, AchievementType::Quester],
        )
        .await;
        assert!(world.drain_pending_channel_broadcasts().is_empty());
    }

    #[tokio::test]
    async fn record_achievement_firsts_and_announce_is_a_no_op_for_empty_unlock_list() {
        // Guards against a future refactor accidentally awaiting/looping
        // when there is nothing to record, even with a `Some` repository
        // (can't construct a live `PgAchievementRepository` without a
        // pool here, but an empty `unlocked` slice must short-circuit
        // before ever touching it).
        let mut world = World::default();
        let empty: [AchievementType; 0] = [];
        record_achievement_firsts_and_announce(&mut world, &None, CharacterId(1), "Hero", &empty)
            .await;
        assert!(world.drain_pending_channel_broadcasts().is_empty());
    }
}
