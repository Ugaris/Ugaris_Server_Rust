use super::*;

pub const MAX_PLAYERS: usize = 512;

pub const OUTPUT_BUFFER_SIZE: usize = 16_384 * 2;

pub const MAX_SCROLLBACK: usize = 8192;

pub const MAX_PLAYER_EFFECTS: usize = 64;

pub const COMMAND_QUEUE_SIZE: usize = 16;

pub const KEYRING_MAX_KEYS: usize = 100;

pub const KEYRING_KEY_NAME_LEN: usize = 40;

pub const KEYRING_KEY_DESC_LEN: usize = 80;

pub const KEYRING_KEY_DRDATA_LEN: usize = 16;

pub const LEGACY_KEYRING_PPD_SIZE: usize = 15_912;

pub const TREASURE_CHEST_PPD_ENTRIES: usize = 200;

pub const LEGACY_TREASURE_CHEST_PPD_SIZE: usize = TREASURE_CHEST_PPD_ENTRIES * 4;

pub const LEGACY_TRANSPORT_PPD_SIZE: usize = 8;

pub const RANDCHEST_MAX_ENTRIES: usize = 100;

pub const LEGACY_RANDCHEST_PPD_SIZE: usize = RANDCHEST_MAX_ENTRIES * 4 * 2;

pub const RATCHEST_MAX_ENTRIES: usize = 100;

pub const LEGACY_RATCHEST_PPD_SIZE: usize = RATCHEST_MAX_ENTRIES * 4 * 2 + 3 * 4;

pub const ORBSPAWN_MAX_ENTRIES: usize = 100;

pub const LEGACY_ORBSPAWN_PPD_SIZE: usize = ORBSPAWN_MAX_ENTRIES * 4 * 2;

pub const FLOWER_MAX_ENTRIES: usize = 100;

pub const LEGACY_FLOWER_PPD_SIZE: usize = FLOWER_MAX_ENTRIES * 4 * 2;

pub const DEMONSHRINE_MAX_ENTRIES: usize = 100;

pub const LEGACY_DEMONSHRINE_PPD_SIZE: usize = DEMONSHRINE_MAX_ENTRIES * 4;

pub const RANDOMSHRINE_USED_WORDS: usize = 256 / 32;

pub const LEGACY_RANDOMSHRINE_PPD_SIZE: usize = RANDOMSHRINE_USED_WORDS * 4 + 1;

pub const TREASURE_DIG_PPD_ENTRIES: usize = 5;

pub const LEGACY_TREASURE_DIG_PPD_SIZE: usize = TREASURE_DIG_PPD_ENTRIES * 4;

pub const LEGACY_MISC_PPD_SIZE: usize = 36;

/// C `struct firstkill_ppd` (`src/system/death.c:164-167`): `unsigned int
/// kill[32]` - a flat 1024-bit (0..1023 `ch.class` range) bitmask, one bit
/// per unique NPC class this character has ever killed.
pub const LEGACY_FIRSTKILL_PPD_SIZE: usize = 32 * 4;

/// C `struct arena_ppd` (`src/system/arena.c:204-211`): `score, fights,
/// wins, losses, lastfight` - 5 flat `int` fields, 20 bytes.
pub const LEGACY_ARENA_PPD_SIZE: usize = 5 * 4;

pub(crate) const ARENA_PPD_SCORE_OFFSET: usize = 0 * 4;

pub(crate) const ARENA_PPD_FIGHTS_OFFSET: usize = 1 * 4;

pub(crate) const ARENA_PPD_WINS_OFFSET: usize = 2 * 4;

pub(crate) const ARENA_PPD_LOSSES_OFFSET: usize = 3 * 4;

pub(crate) const ARENA_PPD_LASTFIGHT_OFFSET: usize = 4 * 4;

/// C `score_fight`'s first-fight seed score (`arena.c:437,441`): a brand
/// new arena record starts at -2000, not 0.
pub const ARENA_PPD_NEWCOMER_SCORE: i32 = -2000;

/// C `struct military_ppd` (`src/module/military.h:28-60`): 6 flat header
/// `int`s (`current_pts`/`master_state`/`current_advisor`/`advisor_state`/
/// `advisor_cost`/`advisor_storage_nr`), `advisor_last[MAXADVISOR]`
/// (`MAXADVISOR` = 20), `military_pts`/`normal_exp`/`mission_yday` (3),
/// `mis[5]` (5 `struct single_mission`, 5 `int`s each), `took_mission`/
/// `took_yday`/`solved_mission`/`solved_yday`/`recommend`/
/// `mission_type_preference`/`mission_difficulty_preference`/
/// `temp_mission_type`/`temp_mission_difficulty`/`reroll_yday` (10) = 6 +
/// 20 + 3 + 25 + 10 = 64 `int`s, 256 bytes. Only the mission-progress
/// fields this iteration's `check_military_solve` port needs
/// (`mis[5]`/`took_mission`/`solved_mission`) have named accessors below;
/// the rest of the struct round-trips as opaque bytes until the mission-
/// offer/accept/complete wrappers land (see `PORTING_TODO.md`'s "Military
/// ranks" entry).
pub const LEGACY_MILITARY_PPD_SIZE: usize = 64 * 4;

pub const MILITARY_PPD_MAXADVISOR: usize = 20;

pub(crate) const MILITARY_PPD_MISSION_COUNT: usize = 5;

pub(crate) const MILITARY_PPD_MIS_BASE_OFFSET: usize = (6 + MILITARY_PPD_MAXADVISOR + 3) * 4;

pub(crate) const MILITARY_PPD_MIS_ENTRY_SIZE: usize = 5 * 4;

pub(crate) const MILITARY_PPD_TOOK_MISSION_OFFSET: usize =
    MILITARY_PPD_MIS_BASE_OFFSET + MILITARY_PPD_MISSION_COUNT * MILITARY_PPD_MIS_ENTRY_SIZE;

pub(crate) const MILITARY_PPD_TOOK_YDAY_OFFSET: usize = MILITARY_PPD_TOOK_MISSION_OFFSET + 4;

pub(crate) const MILITARY_PPD_SOLVED_MISSION_OFFSET: usize = MILITARY_PPD_TOOK_MISSION_OFFSET + 8;

pub(crate) const MILITARY_PPD_SOLVED_YDAY_OFFSET: usize = MILITARY_PPD_SOLVED_MISSION_OFFSET + 4;

pub(crate) const MILITARY_PPD_RECOMMEND_OFFSET: usize = MILITARY_PPD_SOLVED_MISSION_OFFSET + 8;

// `current_pts` is `military_ppd`'s very first field (offset 0).
pub(crate) const MILITARY_PPD_CURRENT_PTS_OFFSET: usize = 0;

pub(crate) const MILITARY_PPD_MISSION_TYPE_PREFERENCE_OFFSET: usize =
    MILITARY_PPD_RECOMMEND_OFFSET + 4;

pub(crate) const MILITARY_PPD_MISSION_DIFFICULTY_PREFERENCE_OFFSET: usize =
    MILITARY_PPD_MISSION_TYPE_PREFERENCE_OFFSET + 4;

pub(crate) const MILITARY_PPD_MISSION_YDAY_OFFSET: usize = MILITARY_PPD_MIS_BASE_OFFSET - 4;

/// C `military_ppd::advisor_last[MAXADVISOR]` (`military.h:37`): header
/// offset 24 bytes (6 leading `int`s: `current_pts`/`master_state`/
/// `current_advisor`/`advisor_state`/`advisor_cost`/`advisor_storage_nr`).
pub(crate) const MILITARY_PPD_ADVISOR_LAST_BASE_OFFSET: usize = 6 * 4;

/// C `military_ppd::master_state` (`military.h:29`): second header field.
pub(crate) const MILITARY_PPD_MASTER_STATE_OFFSET: usize = 1 * 4;

/// C `military_ppd::current_advisor` (`military.h:31`): "re-using storage
/// ID" per its own comment - the advisor NPC's `storage_ID` that most
/// recently interacted with this player.
pub(crate) const MILITARY_PPD_CURRENT_ADVISOR_OFFSET: usize = 2 * 4;

/// C `military_ppd::advisor_state` (`military.h:32`).
pub(crate) const MILITARY_PPD_ADVISOR_STATE_OFFSET: usize = 3 * 4;

/// C `military_ppd::advisor_cost` (`military.h:33`).
pub(crate) const MILITARY_PPD_ADVISOR_COST_OFFSET: usize = 4 * 4;

/// C `military_ppd::advisor_storage_nr` (`military.h:34`).
pub(crate) const MILITARY_PPD_ADVISOR_STORAGE_NR_OFFSET: usize = 5 * 4;

/// C `military_ppd::military_pts` (`military.h:39`): exp gained towards
/// ranks, immediately after `advisor_last[MAXADVISOR]`.
pub(crate) const MILITARY_PPD_MILITARY_PTS_OFFSET: usize =
    MILITARY_PPD_ADVISOR_LAST_BASE_OFFSET + MILITARY_PPD_MAXADVISOR * 4;

/// C `military_ppd::normal_exp` (`military.h:40`): exp given out.
pub(crate) const MILITARY_PPD_NORMAL_EXP_OFFSET: usize = MILITARY_PPD_MILITARY_PTS_OFFSET + 4;

/// C `military_ppd::temp_mission_type` (`military.h:56`), immediately
/// after `mission_difficulty_preference`.
pub(crate) const MILITARY_PPD_TEMP_MISSION_TYPE_OFFSET: usize =
    MILITARY_PPD_MISSION_DIFFICULTY_PREFERENCE_OFFSET + 4;

/// C `military_ppd::temp_mission_difficulty` (`military.h:57`).
pub(crate) const MILITARY_PPD_TEMP_MISSION_DIFFICULTY_OFFSET: usize =
    MILITARY_PPD_TEMP_MISSION_TYPE_OFFSET + 4;

/// C `military_ppd::reroll_yday` (`military.h:59`): the very last field of
/// the struct, immediately after `temp_mission_difficulty`.
pub(crate) const MILITARY_PPD_REROLL_YDAY_OFFSET: usize = LEGACY_MILITARY_PPD_SIZE - 4;

/// C `struct tunnel_ppd { int clevel; unsigned char used[204]; }`
/// (`src/area/33/tunnel.h:6-9`): one leading `int` (4 bytes) followed by
/// the 204-byte `used[]` completion-count array (`MAX_TUNNEL_LEVEL` = 200,
/// so indices `0..=203` cover every valid level with room to spare, no
/// struct padding since 4 + 204 = 208 is already a multiple of 4).
pub const LEGACY_TUNNEL_PPD_SIZE: usize = 4 + 204;

pub(crate) const TUNNEL_PPD_USED_BASE_OFFSET: usize = 4;

/// C `struct gorwin_ppd { int tunnel_level; }` (`src/area/33/tunnel.h:
/// 11-13`): a single `int`.
pub const LEGACY_GORWIN_PPD_SIZE: usize = 4;

/// C `#define MIN_TUNNEL_LEVEL 10` (`src/area/33/tunnel.h:29`).
pub const MIN_TUNNEL_LEVEL: i32 = 10;

/// C `#define MAX_TUNNEL_LEVEL 200` (`src/area/33/tunnel.h:28`).
pub const MAX_TUNNEL_LEVEL: i32 = 200;

/// C `#define MAX_TUNNEL_USES 10` (`src/area/33/tunnel.h:30`).
pub const MAX_TUNNEL_USES: u8 = 10;

/// C `struct area3_ppd` (`src/area/3/area3.h:18-35` /
/// `src/system/game/ppd_structs.h:109-127`): 18 `int` fields (`imp_kills,
/// imp_flags;` declares two on one line). Was previously `17 * 4` (a
/// missing-field size bug that happened to go unnoticed because
/// `kassim_item_wait_starttime`, the 18th/last field, had no accessor
/// yet); fixed while adding the `questlog_init_area3` accessors below.
pub const LEGACY_AREA3_PPD_SIZE: usize = 18 * 4;

/// C `struct area1_ppd` (`src/area/1/area1.h:24-75`): 39 `int` fields.
pub const LEGACY_AREA1_PPD_SIZE: usize = 39 * 4;

/// C `struct nomad_ppd` (`src/common/nomad_ppd.h:9-13`):
/// `nomad_state[MAXNOMAD]` + `nomad_win[MAXNOMAD]` (`MAXNOMAD` = 10) +
/// `open_roll1/2/3/open_bet` + `tribe_member` = 10+10+4+1 = 25 `int`s.
pub const LEGACY_NOMAD_PPD_SIZE: usize = 25 * 4;

/// C `struct quest quest[MAXQUEST]` (`src/system/questlog.h:19,36-39`):
/// `MAXQUEST` (100) 1-byte packed bitfields (`done:6`, `flags:2`).
pub const LEGACY_QUESTLOG_PPD_SIZE: usize = MAX_QUESTS;

pub const NOMAD_PPD_MAXNOMAD: usize = 10;

pub const LEGACY_CALIGAR_PPD_SIZE: usize = 14 * 4 + 4;

pub const LEGACY_ARKHATA_PPD_SIZE: usize = 25 * 4;

pub const LEGACY_STAFFER_PPD_SIZE: usize = 25 * 4;

pub const LEGACY_FARMY_PPD_SIZE: usize = 85 * 4;

pub const LEGACY_TEUFELRAT_PPD_SIZE: usize = 2 * 4;

/// C `#define MAXSTAT 365` (`src/system/statistics.h`): the rolling
/// window's day count for `struct stats_ppd`.
pub const STATS_PPD_MAXSTAT: usize = 365;

/// C `#define RESOLUTION (60*60*24)` (`src/system/statistics.h`): one
/// day, in seconds - the bucket width `stats_update` groups samples by.
pub const STATS_PPD_RESOLUTION_SECONDS: i64 = 60 * 60 * 24;

/// C `#define STARTTIME 978303600 // 01/01/2001` (`src/system/date.h`):
/// `realtime`'s epoch offset (`realtime = time_now - STARTTIME`,
/// `date.c:271`) - `stats_update`/`stats_online_time` subtract this from
/// the caller's wall-clock unix seconds before day-bucketing, matching
/// C's own `realtime` exactly (the offset has no gameplay effect beyond
/// which wall-clock day lands in bucket 0, but is cheap to reproduce
/// digit-for-digit).
pub const STATS_PPD_STARTTIME: i64 = 978_303_600;

/// One `struct stats { int exp; int gold; int online; }` day-sample
/// (`src/system/statistics.h`), 3 packed `i32`s.
pub(crate) const STATS_PPD_DAY_SIZE: usize = 12;

pub(crate) const STATS_PPD_DAY_EXP_OFFSET: usize = 0;

pub(crate) const STATS_PPD_DAY_GOLD_OFFSET: usize = 4;

pub(crate) const STATS_PPD_DAY_ONLINE_OFFSET: usize = 8;

pub(crate) const STATS_PPD_LAST_UPDATE_OFFSET: usize = 0;

pub(crate) const STATS_PPD_DAYS_OFFSET: usize = 4;

pub(crate) fn stats_ppd_day_offset(day: usize) -> usize {
    STATS_PPD_DAYS_OFFSET + day * STATS_PPD_DAY_SIZE
}

/// C `struct stats_ppd { int last_update; struct stats stats[MAXSTAT]; }`
/// (`src/system/statistics.h`): `4 + 365 * 12` bytes.
pub const LEGACY_STATS_PPD_SIZE: usize =
    STATS_PPD_DAYS_OFFSET + STATS_PPD_MAXSTAT * STATS_PPD_DAY_SIZE;

/// C `struct bank_ppd` (`src/module/bank.h`): a single `int imperial_gold`.
pub const LEGACY_BANK_PPD_SIZE: usize = 4;

pub const LEGACY_TWOCITY_PPD_SIZE: usize = 29 * 4;

pub const LEGACY_LAB_PPD_SIZE: usize = 360;

pub const LEGACY_SALTMINE_PPD_SIZE: usize = 4 + SALTMINE_LADDER_COUNT * 4 + 4;

pub const LEGACY_SALTMINE_PPD_VERSION: u8 = 1;

pub const LEGACY_LAB2_GRAVE_VERSION: u8 = 2;

pub const LEGACY_LAB2_GRAVEVERSION_OFFSET: usize = 43;

pub const LEGACY_LAB2_GRAVEINDEX_OFFSET: usize = 44;

pub const LAB2_GRAVE_BITSET_BYTES: usize = 256;

pub const LAB2_DESCRIBED_GRAVES: [((u16, u16), &str); 40] = [
    ((194, 183), "%s is buried in the third grave behind the chapel."),
    ((192, 183), "%s is buried at the left side of her husband John."),
    ((186, 183), "%s is buried in the seventh grave behind the chapel."),
    ((184, 183), "%s is buried at the left side of her husband John."),
    ((176, 194), "For his generosity %s is buried in the third grave of the first row next to the northwestern chapel aisle."),
    ((176, 196), "%s is buried at the left side of her husband John."),
    ((173, 191), "For his generosity %s is buried in the first grave of the second row next to the northwestern chapel aisle."),
    ((173, 193), "%s is buried at the left side of her husband John."),
    ((199, 195), "For his generosity %s is buried in the first grave of the second row next to the southeastern chapel aisle."),
    ((199, 193), "%s is buried at the left side of her husband John."),
    ((196, 196), "For his generosity %s is buried in the first grave of the first row next to the southeastern chapel aisle."),
    ((196, 194), "%s is buried at the left side of her husband John."),
    ((160, 233), "%s is buried in the fifth grave of the second row in the southwest section of the graveyard."),
    ((158, 233), "%s is buried at the left side of her husband John."),
    ((162, 230), "%s is buried in the fourth grave of the third row in the southwest section of the graveyard."),
    ((160, 230), "%s is buried at the left side of her husband John."),
    ((210, 244), "%s is buried in the fourth grave of the second row in the southeast section of the graveyard."),
    ((208, 244), "%s is buried at the left side of her husband John."),
    ((206, 232), "%s is buried in the sixth grave of the sixth row in the southeast section of the graveyard."),
    ((204, 232), "%s is buried at the left side of her husband John."),
    ((172, 228), "%s is buried in the fifth grave of the first row in the northwest entrance section of the graveyard."),
    ((172, 226), "%s is buried at the left side of her husband John."),
    ((181, 224), "%s is buried in the seventh grave of the last row in the northwest entrance section of the graveyard."),
    ((181, 222), "%s is buried at the left side of her husband John."),
    ((191, 222), "%s is buried in the first grave of the last row in the southeast entrance section of the graveyard."),
    ((191, 224), "%s is buried at the left side of her husband John."),
    ((197, 232), "%s is buried in the sixth grave of the second row in the southeast entrance section of the graveyard."),
    ((197, 234), "%s is buried at the left side of her husband John."),
    ((155, 211), "%s is buried in the second grave of the first row in the section with the cross in front of the administrative building."),
    ((155, 209), "%s is buried at the left side of her husband John."),
    ((164, 201), "%s is buried in the seventh grave of the last row in the section with the cross in front of the administrative building."),
    ((164, 199), "%s is buried at the left side of her husband John."),
    ((158, 189), "%s is buried in the second grave of the second row in the section without the cross in front of the administravive building."),
    ((158, 187), "%s is buried at the left side of her husband John."),
    ((161, 189), "%s is buried in the second grave of the third row in the section without the cross in front of the administravive building."),
    ((161, 187), "%s is buried at the left side of her husband John."),
    ((208, 182), "%s is buried in the first grave in the northeastern part of the northeast section of the graveyard."),
    ((210, 182), "%s is buried at the left side of her husband John."),
    ((214, 191), "%s is buried in the fourth grave of the last row in the northeastern part of the northeast section of the graveyard."),
    ((212, 191), "%s is buried at the left side of her husband John."),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaligarSkellyDeathResult {
    Unmapped { x: u16, y: u16 },
    AlreadyUnlocked { door_index: u8, bit: u8 },
    PartiallyUnlocked { door_index: u8, bit: u8 },
    FullyUnlocked { door_index: u8, bit: u8 },
}

pub const LEGACY_LOSTCON_PPD_SIZE: usize = 19 * 4;

pub const RUNE_USED_WORDS: usize = 1024 / 32;

pub const RUNE_SPECIAL_EXEC_COUNT: usize = 25;

pub const LEGACY_RUNE_PPD_SIZE: usize = RUNE_USED_WORDS * 4 + RUNE_SPECIAL_EXEC_COUNT * 4;

/// C `MAXRUNE` (`src/area/18/bones.c:80`): the exclusive upper bound for a
/// rune combination number, matching `rune_used_words`'s 32-word (1024-bit)
/// bitfield.
pub const MAXRUNE: i32 = 1024;

/// Result of `rune_check(cn, nr, ppd)` (`src/area/18/bones.c:285-299`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuneCheckResult {
    /// The combination has never been executed by this player.
    Ok,
    /// `nr` is outside `0..MAXRUNE` (C: "You have found bug #5136a.").
    OutOfRange,
    /// The bit for `nr` is already set (C: "You cannot use this
    /// combination again.").
    AlreadyUsed,
}

pub const PK_HATE_MAX_ENTRIES: usize = 50;

pub const LEGACY_PK_PPD_SIZE: usize = 4 * 4 + PK_HATE_MAX_ENTRIES * 4;

pub const IGNORE_MAX_ENTRIES: usize = 100;

pub const LEGACY_IGNORE_PPD_SIZE: usize = IGNORE_MAX_ENTRIES * 4;

pub const SWEAR_SENTENCE_COUNT: usize = 10;

pub const SWEAR_SENTENCE_LEN: usize = 80;

pub const LEGACY_SWEAR_PPD_SIZE: usize =
    10 * 4 + 4 + SWEAR_SENTENCE_COUNT * SWEAR_SENTENCE_LEN + 10 * 4 + 10 * 4 + 4 + 4;

pub const ALIAS_MAX_ENTRIES: usize = 32;

pub const ALIAS_FROM_LEN: usize = 8;

pub const ALIAS_TO_LEN: usize = 56;

pub const LEGACY_ALIAS_PPD_SIZE: usize = ALIAS_MAX_ENTRIES * (ALIAS_FROM_LEN + ALIAS_TO_LEN);

pub const WARP_BONUS_COUNT: usize = 50;

pub const LEGACY_WARP_PPD_SIZE: usize = 4 + 4 + WARP_BONUS_COUNT * 4 + WARP_BONUS_COUNT * 4 + 4;

pub const PERSISTENT_PLAYER_DATA: u32 = 1 << 31;

pub const PERSISTENT_SUBSCRIBER_DATA: u32 = 1 << 30;

pub const DEV_ID_DB: u32 = 1;

pub const DEV_ID_MR: u32 = 2;

pub const DEV_ID_ED: u32 = 59;

pub const DRD_JUNK_PPD: u32 = make_drd(DEV_ID_DB, 114 | PERSISTENT_PLAYER_DATA);

pub const DRD_AREA3_PPD: u32 = make_drd(DEV_ID_DB, 40 | PERSISTENT_PLAYER_DATA);

/// C `#define DRD_AREA1_PPD MAKE_DRD(DEV_ID_DB, 22 | PERSISTENT_PLAYER_DATA)`
/// (`src/system/drdata.h`). See `area1_ppd`/`encode_legacy_area1_ppd`.
pub const DRD_AREA1_PPD: u32 = make_drd(DEV_ID_DB, 22 | PERSISTENT_PLAYER_DATA);

/// C `#define DRD_NOMAD_PPD MAKE_DRD(DEV_ID_DB, 112 | PERSISTENT_PLAYER_DATA)`
/// (`src/system/drdata.h`). See `nomad_ppd`/`encode_legacy_nomad_ppd`.
pub const DRD_NOMAD_PPD: u32 = make_drd(DEV_ID_DB, 112 | PERSISTENT_PLAYER_DATA);

pub const DRD_TREASURE_CHEST_PPD: u32 = make_drd(DEV_ID_DB, 17 | PERSISTENT_PLAYER_DATA);

pub const DRD_TRANSPORT_PPD: u32 = make_drd(DEV_ID_DB, 44 | PERSISTENT_PLAYER_DATA);

pub const DRD_PK_PPD: u32 = make_drd(DEV_ID_DB, 47 | PERSISTENT_PLAYER_DATA);

pub const TRANSPORT_MAJOR_CITIES_MASK: u64 = 0x03E0_0205;

pub const TRANSPORT_ALL_TELEPORTS_MASK: u64 = 0x03F3_F7FF;

pub const TRANSPORT_EARTH_UNDERGROUND_MASK: u64 = 0x01F8;

pub const DRD_RANDCHEST_PPD: u32 = make_drd(DEV_ID_DB, 63 | PERSISTENT_PLAYER_DATA);

pub const DRD_RATCHEST_PPD: u32 = make_drd(DEV_ID_DB, 84 | PERSISTENT_PLAYER_DATA);

pub const DRD_DEMONSHRINE_PPD: u32 = make_drd(DEV_ID_DB, 68 | PERSISTENT_PLAYER_DATA);

pub const DRD_RANDOMSHRINE_PPD: u32 = make_drd(DEV_ID_DB, 86 | PERSISTENT_PLAYER_DATA);

pub const DRD_ORBSPAWN_PPD: u32 = make_drd(DEV_ID_DB, 105 | PERSISTENT_PLAYER_DATA);

pub const DRD_LOSTCON_PPD: u32 = make_drd(DEV_ID_DB, 91 | PERSISTENT_PLAYER_DATA);

pub const DRD_FLOWER_PPD: u32 = make_drd(DEV_ID_DB, 62 | PERSISTENT_PLAYER_DATA);

pub const DRD_MISC_PPD: u32 = make_drd(DEV_ID_DB, 113 | PERSISTENT_PLAYER_DATA);

pub const DRD_ALIAS_PPD: u32 = make_drd(DEV_ID_DB, 80 | PERSISTENT_PLAYER_DATA);

pub const DRD_IGNORE_PPD: u32 = make_drd(DEV_ID_DB, 100 | PERSISTENT_PLAYER_DATA);

pub const DRD_SWEAR_PPD: u32 = make_drd(DEV_ID_DB, 109 | PERSISTENT_PLAYER_DATA);

pub const DRD_TREASURE_DIG_PPD: u32 = make_drd(DEV_ID_ED, 5 | PERSISTENT_PLAYER_DATA);

pub const DRD_KEYRING_PPD: u32 = make_drd(DEV_ID_ED, 7 | PERSISTENT_PLAYER_DATA);

pub const DRD_RUNE_PPD: u32 = make_drd(DEV_ID_DB, 108 | PERSISTENT_PLAYER_DATA);

pub const DRD_CALIGAR_PPD: u32 = make_drd(DEV_ID_DB, 159 | PERSISTENT_PLAYER_DATA);

pub const DRD_ARKHATA_PPD: u32 = make_drd(DEV_ID_DB, 160 | PERSISTENT_PLAYER_DATA);

pub const DRD_STAFFER_PPD: u32 = make_drd(DEV_ID_DB, 130 | PERSISTENT_PLAYER_DATA);

pub const DRD_FARMY_PPD: u32 = make_drd(DEV_ID_DB, 77 | PERSISTENT_PLAYER_DATA);

pub const DRD_TEUFELRAT_PPD: u32 = make_drd(DEV_ID_DB, 157 | PERSISTENT_PLAYER_DATA);

/// C `#define DRD_STATS_PPD MAKE_DRD(DEV_ID_DB, 27 | PERSISTENT_PLAYER_DATA)`
/// (`src/system/drdata.h`). See `stats_ppd`/`encode_legacy_stats_ppd` and
/// `stats_update`/`stats_online_time` (`src/system/statistics.c`).
pub const DRD_STATS_PPD: u32 = make_drd(DEV_ID_DB, 27 | PERSISTENT_PLAYER_DATA);

/// The following ids (`src/system/drdata.h`) back systems that are not
/// modeled on `PlayerRuntime` at all yet (army rank, military points,
/// arena, sidestory, strategy game, and quest log). They exist here
/// solely so `turn_seyan` (`src/system/tool.c:4278-4389`, ported at
/// `World::apply_turn_seyan`) can `del_data` them exactly like C does, via
/// `PlayerRuntime::clear_turn_seyan_ppd`'s raw-block strip - see
/// `strip_ppd_blocks`. No decode/encode logic backs these ids since
/// nothing else in this codebase reads or writes them yet. `DRD_AREA1_PPD`
/// and `DRD_NOMAD_PPD` moved out of this group (see `area1_ppd`/
/// `nomad_ppd` below) once their questlog-init-required fields got real
/// accessors; `DRD_QUESTLOG_PPD` moved out the same way once `quest_log`
/// got a real PPD codec (see below); `DRD_FIRSTKILL_PPD` moved out the
/// same way once `first_kill_ppd` got a real codec (see
/// `encode_legacy_firstkill_ppd`/`decode_legacy_firstkill_ppd` below);
/// `DRD_TUNNEL_PPD` moved out the same way once `tunnel_ppd` got a real
/// codec (see `encode_legacy_tunnel_ppd`/`decode_legacy_tunnel_ppd`
/// below, backing the `/tunnel`/`/tunnels` commands); `DRD_DEPOT_PPD`
/// moved out the same way once `depot` got a real codec (see
/// `encode_legacy_depot_ppd`/`decode_legacy_depot_ppd` below, backing the
/// `/depotsort` command and the `IF_DEPOT` container-open path,
/// `ugaris-server::depot`).
pub const DRD_FIRSTKILL_PPD: u32 = make_drd(DEV_ID_DB, 18 | PERSISTENT_PLAYER_DATA);

pub const DRD_RANK_PPD: u32 = make_drd(DEV_ID_DB, 41 | PERSISTENT_PLAYER_DATA);

/// C `#define DRD_DEPOT_PPD MAKE_DRD(DEV_ID_DB, 67 | PERSISTENT_PLAYER_DATA)`
/// (`src/system/drdata.h:129`): `struct depot_ppd { struct item
/// itm[MAXDEPOT]; }` (`src/system/depot.h:19-23`), the character's own
/// 80-slot legacy storage depot (opened via any item with the `IF_DEPOT`
/// flag) - a distinct, older, per-character system from
/// `ugaris-server::depot`'s account-wide `AccountDepotState`
/// (`DRD_ACCOUNT_WIDE_DEPOT`). See
/// `encode_legacy_depot_ppd`/`decode_legacy_depot_ppd`.
pub const DRD_DEPOT_PPD: u32 = make_drd(DEV_ID_DB, 67 | PERSISTENT_PLAYER_DATA);

/// C `#define MAXDEPOT 80` (`src/system/depot.h:19`).
pub const MAXDEPOT: usize = 80;

pub const DRD_MILITARY_PPD: u32 = make_drd(DEV_ID_DB, 72 | PERSISTENT_PLAYER_DATA);

pub const DRD_ARENA_PPD: u32 = make_drd(DEV_ID_DB, 83 | PERSISTENT_PLAYER_DATA);

pub const DRD_STRATEGY_PPD: u32 = make_drd(DEV_ID_DB, 121 | PERSISTENT_PLAYER_DATA);

pub const DRD_SIDESTORY_PPD: u32 = make_drd(DEV_ID_DB, 124 | PERSISTENT_PLAYER_DATA);

/// C `#define DRD_TUNNEL_PPD MAKE_DRD(DEV_ID_DB, 154 |
/// PERSISTENT_PLAYER_DATA)` (`src/system/drdata.h:216`): `struct
/// tunnel_ppd { int clevel; unsigned char used[204]; }`
/// (`src/area/33/tunnel.h:6-9`). See
/// `encode_legacy_tunnel_ppd`/`decode_legacy_tunnel_ppd`.
pub const DRD_TUNNEL_PPD: u32 = make_drd(DEV_ID_DB, 154 | PERSISTENT_PLAYER_DATA);

/// C `#define DRD_GORWIN_PPD MAKE_DRD(DEV_ID_ED, 4 |
/// PERSISTENT_PLAYER_DATA)` (`src/system/drdata.h:257`): `struct
/// gorwin_ppd { int tunnel_level; }` (`src/area/33/tunnel.h:11-13`), the
/// Gorwin NPC's currently-offered tunnel level (not deleted by
/// `turn_seyan`, unlike `DRD_TUNNEL_PPD`). See
/// `encode_legacy_gorwin_ppd`/`decode_legacy_gorwin_ppd`.
pub const DRD_GORWIN_PPD: u32 = make_drd(DEV_ID_ED, 4 | PERSISTENT_PLAYER_DATA);

/// C `#define DRD_QUESTLOG_PPD MAKE_DRD(DEV_ID_DB, 158 |
/// PERSISTENT_PLAYER_DATA)` (`src/system/drdata.h:220`): the persisted
/// `struct quest quest[MAXQUEST]` array (`src/system/questlog.h:36-39`),
/// one packed byte per quest (`done:6` in the low bits, `flags:2` in the
/// high bits, matching x86 GCC's LSB-first bitfield allocation) - see
/// `encode_legacy_questlog_ppd`/`decode_legacy_questlog_ppd`.
pub const DRD_QUESTLOG_PPD: u32 = make_drd(DEV_ID_DB, 158 | PERSISTENT_PLAYER_DATA);

/// C `#define DRD_BANK_PPD MAKE_DRD(DEV_ID_DB, 38 | PERSISTENT_PLAYER_DATA)`
/// (`src/system/drdata.h:100`).
pub const DRD_BANK_PPD: u32 = make_drd(DEV_ID_DB, 38 | PERSISTENT_PLAYER_DATA);

pub const DRD_TWOCITY_PPD: u32 = make_drd(DEV_ID_DB, 97 | PERSISTENT_PLAYER_DATA);

pub const DRD_LAB_PPD: u32 = make_drd(DEV_ID_DB, 116 | PERSISTENT_PLAYER_DATA);

pub const DRD_WARP_PPD: u32 = make_drd(DEV_ID_DB, 127 | PERSISTENT_PLAYER_DATA);

/// C `#define DRD_GATE_PPD MAKE_DRD(DEV_ID_DB, 65 | PERSISTENT_PLAYER_DATA)`
/// (`src/system/drdata.h:127`): `struct gate_ppd { int welcome_state; int
/// target_class; int step; }` (`src/system/gatekeeper.c:221-225`), the
/// gatekeeper welcome-dialogue/test progress carried on the player.
pub const DRD_GATE_PPD: u32 = make_drd(DEV_ID_DB, 65 | PERSISTENT_PLAYER_DATA);

pub const SALTMINE_LADDER_COUNT: usize = 20;

pub const DRD_SALTMINE_PPD: u32 = make_drd(DEV_ID_MR, 13 | PERSISTENT_PLAYER_DATA);

pub const SPECIAL_SHRINE_HCSC_CUTOFF_SECONDS: u64 = 1_411_941_600;

pub const SPECIAL_SHRINE_CONFIRM_WINDOW_SECONDS: u64 = 10;

pub(crate) const WARP_PPD_BASE_OFFSET: usize = 0;

pub(crate) const WARP_PPD_POINTS_OFFSET: usize = WARP_PPD_BASE_OFFSET + 4;

pub(crate) const WARP_PPD_BONUS_ID_OFFSET: usize = WARP_PPD_POINTS_OFFSET + 4;

pub(crate) const WARP_PPD_BONUS_LAST_USED_OFFSET: usize =
    WARP_PPD_BONUS_ID_OFFSET + WARP_BONUS_COUNT * 4;

pub(crate) const WARP_PPD_NOSTEPEXP_OFFSET: usize =
    WARP_PPD_BONUS_LAST_USED_OFFSET + WARP_BONUS_COUNT * 4;

/// C `struct gate_ppd { int welcome_state; int target_class; int step; }`
/// (`src/system/gatekeeper.c:221-225`) - three packed `int`s, matching the
/// legacy PPD blob layout exactly.
pub const LEGACY_GATE_PPD_SIZE: usize = 12;

pub(crate) const GATE_PPD_WELCOME_STATE_OFFSET: usize = 0;

pub(crate) const GATE_PPD_TARGET_CLASS_OFFSET: usize = 4;

pub(crate) const GATE_PPD_STEP_OFFSET: usize = 8;

pub const fn make_drd(dev_id: u32, nr: u32) -> u32 {
    (dev_id << 24) | nr
}

pub(crate) const KEYRING_PPD_COUNT_OFFSET: usize = 0;

pub(crate) const KEYRING_PPD_KEYS_OFFSET: usize = 4;

pub(crate) const KEYRING_PPD_NAMES_OFFSET: usize = KEYRING_PPD_KEYS_OFFSET + KEYRING_MAX_KEYS * 4;

pub(crate) const KEYRING_PPD_DESCS_OFFSET: usize =
    KEYRING_PPD_NAMES_OFFSET + KEYRING_MAX_KEYS * KEYRING_KEY_NAME_LEN;

pub(crate) const KEYRING_PPD_SPRITES_OFFSET: usize =
    KEYRING_PPD_DESCS_OFFSET + KEYRING_MAX_KEYS * KEYRING_KEY_DESC_LEN;

pub(crate) const KEYRING_PPD_FLAGS_OFFSET: usize =
    KEYRING_PPD_SPRITES_OFFSET + KEYRING_MAX_KEYS * 4 + 4;

pub(crate) const KEYRING_PPD_VALUES_OFFSET: usize = KEYRING_PPD_FLAGS_OFFSET + KEYRING_MAX_KEYS * 8;

pub(crate) const KEYRING_PPD_DRIVERS_OFFSET: usize =
    KEYRING_PPD_VALUES_OFFSET + KEYRING_MAX_KEYS * 4;

pub(crate) const KEYRING_PPD_DRDATA_OFFSET: usize =
    KEYRING_PPD_DRIVERS_OFFSET + KEYRING_MAX_KEYS * 2;

pub(crate) const KEYRING_PPD_EXPIRE_OFFSET: usize =
    KEYRING_PPD_DRDATA_OFFSET + KEYRING_MAX_KEYS * KEYRING_KEY_DRDATA_LEN;

pub(crate) const KEYRING_PPD_AUTO_ADD_OFFSET: usize = KEYRING_PPD_EXPIRE_OFFSET + KEYRING_MAX_KEYS;

pub(crate) const RANDCHEST_PPD_IDS_OFFSET: usize = 0;

pub(crate) const RANDCHEST_PPD_LAST_USED_OFFSET: usize =
    RANDCHEST_PPD_IDS_OFFSET + RANDCHEST_MAX_ENTRIES * 4;

pub(crate) const RATCHEST_PPD_IDS_OFFSET: usize = 0;

pub(crate) const RATCHEST_PPD_LAST_USED_OFFSET: usize =
    RATCHEST_PPD_IDS_OFFSET + RATCHEST_MAX_ENTRIES * 4;

pub(crate) const RATCHEST_PPD_TREASURE_X_OFFSET: usize =
    RATCHEST_PPD_LAST_USED_OFFSET + RATCHEST_MAX_ENTRIES * 4;

pub(crate) const RATCHEST_PPD_TREASURE_Y_OFFSET: usize = RATCHEST_PPD_TREASURE_X_OFFSET + 4;

pub(crate) const RATCHEST_PPD_LAST_TREASURE_OFFSET: usize = RATCHEST_PPD_TREASURE_Y_OFFSET + 4;

pub(crate) const ORBSPAWN_PPD_IDS_OFFSET: usize = 0;

pub(crate) const ORBSPAWN_PPD_LAST_USED_OFFSET: usize =
    ORBSPAWN_PPD_IDS_OFFSET + ORBSPAWN_MAX_ENTRIES * 4;

pub(crate) const FLOWER_PPD_IDS_OFFSET: usize = 0;

pub(crate) const FLOWER_PPD_LAST_USED_OFFSET: usize =
    FLOWER_PPD_IDS_OFFSET + FLOWER_MAX_ENTRIES * 4;

pub(crate) const AREA3_PPD_SEYMOUR_STATE_OFFSET: usize = 0 * 4;

pub(crate) const AREA3_PPD_KELLY_STATE_OFFSET: usize = 1 * 4;

pub(crate) const AREA3_PPD_KELLY_FOUND_CNT_OFFSET: usize = 2 * 4;

pub(crate) const AREA3_PPD_KELLY_FOUND1_OFFSET: usize = 3 * 4;

pub(crate) const AREA3_PPD_KELLY_FOUND2_OFFSET: usize = 4 * 4;

pub(crate) const AREA3_PPD_KELLY_FOUND3_OFFSET: usize = 5 * 4;

pub(crate) const AREA3_PPD_ASTRO2_STATE_OFFSET: usize = 6 * 4;

pub(crate) const AREA3_PPD_CRYPT_STATE_OFFSET: usize = 7 * 4;

pub(crate) const AREA3_PPD_CRYPT_BONUS_OFFSET: usize = 8 * 4;

pub(crate) const AREA3_PPD_CLARA_STATE_OFFSET: usize = 9 * 4;

pub(crate) const AREA3_PPD_IMP_STATE_OFFSET: usize = 10 * 4;

pub(crate) const AREA3_PPD_IMP_KILLS_OFFSET: usize = 11 * 4;

pub(crate) const AREA3_PPD_IMP_FLAGS_OFFSET: usize = 12 * 4;

pub(crate) const AREA3_PPD_WILLIAM_STATE_OFFSET: usize = 13 * 4;

pub(crate) const AREA3_PPD_HERMIT_STATE_OFFSET: usize = 14 * 4;

// Backs `cmd_showppd`'s `/showppd <name> area3` branch
// (`src/system/command.c:339-346`), plus `kassim_driver`
// (`src/area/3/area3.c::kassim_driver`).
pub(crate) const AREA3_PPD_KASSIM_STATE_OFFSET: usize = 15 * 4;

/// C `struct area3_ppd::kassim_seen_timer` (`src/area/3/area3.h:35`):
/// wall-clock `realtime` seconds at Kassim's last processed `NT_CHAR`.
pub(crate) const AREA3_PPD_KASSIM_SEEN_TIMER_OFFSET: usize = 16 * 4;

/// C `struct area3_ppd::kassim_item_wait_starttime` (`src/area/3/
/// area3.h:36`): wall-clock `realtime` seconds when Kassim started
/// waiting for the item to engrave.
pub(crate) const AREA3_PPD_KASSIM_ITEM_WAIT_STARTTIME_OFFSET: usize = 17 * 4;

// `struct area1_ppd` field offsets (`src/area/1/area1.h:24-75`), in
// declaration order (0-based `int` index * 4). Only the fields consumed by
// `questlog_init_area1` (`src/system/questlog.c:828-1039`) have named
// accessors so far; the rest round-trip as opaque bytes.
pub(crate) const AREA1_PPD_YOAKIN_STATE_OFFSET: usize = 0 * 4;

pub(crate) const AREA1_PPD_GWENDY_STATE_OFFSET: usize = 2 * 4;

pub(crate) const AREA1_PPD_JAMES_STATE_OFFSET: usize = 5 * 4;

pub(crate) const AREA1_PPD_NOOK_STATE_OFFSET: usize = 10 * 4;

pub(crate) const AREA1_PPD_LYDIA_STATE_OFFSET: usize = 11 * 4;

pub(crate) const AREA1_PPD_GUIWYNN_STATE_OFFSET: usize = 15 * 4;

pub(crate) const AREA1_PPD_LOGAIN_STATE_OFFSET: usize = 17 * 4;

pub(crate) const AREA1_PPD_RESKIN_STATE_OFFSET: usize = 19 * 4;

pub(crate) const AREA1_PPD_BRITHILDIE_STATE_OFFSET: usize = 24 * 4;

pub(crate) const AREA1_PPD_CAMHERMIT_STATE_OFFSET: usize = 32 * 4;

pub(crate) const AREA1_PPD_JESSICA_STATE_OFFSET: usize = 35 * 4;

// The remaining `area1_ppd` fields (`src/area/1/area1.h:24-75`) have no
// gameplay driver in Rust yet; these accessors exist solely to back
// `cmd_showppd` (`src/system/command.c:275-341`, `/showppd <name> area1`).
pub(crate) const AREA1_PPD_YOAKIN_SEEN_TIMER_OFFSET: usize = 1 * 4;

pub(crate) const AREA1_PPD_GWENDY_SEEN_TIMER_OFFSET: usize = 3 * 4;

pub(crate) const AREA1_PPD_TERION_STATE_OFFSET: usize = 4 * 4;

pub(crate) const AREA1_PPD_FLAGS_OFFSET: usize = 6 * 4;

pub(crate) const AREA1_PPD_DARKIN_STATE_OFFSET: usize = 7 * 4;

pub(crate) const AREA1_PPD_GEREWIN_STATE_OFFSET: usize = 8 * 4;

pub(crate) const AREA1_PPD_GEREWIN_SEEN_TIMER_OFFSET: usize = 9 * 4;

pub(crate) const AREA1_PPD_LYDIA_SEEN_TIMER_OFFSET: usize = 12 * 4;

pub(crate) const AREA1_PPD_ASTURIN_STATE_OFFSET: usize = 13 * 4;

pub(crate) const AREA1_PPD_ASTURIN_SEEN_TIMER_OFFSET: usize = 14 * 4;

pub(crate) const AREA1_PPD_GUIWYNN_SEEN_TIMER_OFFSET: usize = 16 * 4;

pub(crate) const AREA1_PPD_LOGAIN_SEEN_TIMER_OFFSET: usize = 18 * 4;

pub(crate) const AREA1_PPD_RESKIN_SEEN_TIMER_OFFSET: usize = 20 * 4;

pub(crate) const AREA1_PPD_RESKIN_GOT_BITS_OFFSET: usize = 21 * 4;

pub(crate) const AREA1_PPD_SHRIKE_STATE_OFFSET: usize = 22 * 4;

pub(crate) const AREA1_PPD_SHRIKE_FAILS_OFFSET: usize = 23 * 4;

pub(crate) const AREA1_PPD_BRITHILDIE_SEEN_TIMER_OFFSET: usize = 25 * 4;

pub(crate) const AREA1_PPD_JIU_STATE_OFFSET: usize = 26 * 4;

pub(crate) const AREA1_PPD_JIU_SEEN_TIMER_OFFSET: usize = 27 * 4;

pub(crate) const AREA1_PPD_GREETER_STATE_OFFSET: usize = 28 * 4;

pub(crate) const AREA1_PPD_GREETER_SEEN_TIMER_OFFSET: usize = 29 * 4;

pub(crate) const AREA1_PPD_ACLERK_STATE_OFFSET: usize = 30 * 4;

pub(crate) const AREA1_PPD_ACLERK_SEEN_TIMER_OFFSET: usize = 31 * 4;

pub(crate) const AREA1_PPD_CAMHERMIT_SEEN_TIMER_OFFSET: usize = 33 * 4;

pub(crate) const AREA1_PPD_CAMHERMIT_KILLS_OFFSET: usize = 34 * 4;

pub(crate) const AREA1_PPD_JESSICA_SEEN_TIMER_OFFSET: usize = 36 * 4;

pub(crate) const AREA1_PPD_FOREST_RANGER_STATE_OFFSET: usize = 37 * 4;

pub(crate) const AREA1_PPD_FOREST_RANGER_SEEN_TIMER_OFFSET: usize = 38 * 4;

// `struct nomad_ppd` field offsets (`src/common/nomad_ppd.h:9-13`):
// `nomad_state[MAXNOMAD]` then `nomad_win[MAXNOMAD]` then the four open-
// roll/bet ints then `tribe_member`.
pub(crate) const NOMAD_PPD_STATE_OFFSET: usize = 0;

pub(crate) const NOMAD_PPD_WIN_OFFSET: usize = NOMAD_PPD_MAXNOMAD * 4;

pub(crate) const NOMAD_PPD_OPEN_ROLL1_OFFSET: usize = NOMAD_PPD_WIN_OFFSET + NOMAD_PPD_MAXNOMAD * 4;

pub(crate) const NOMAD_PPD_OPEN_ROLL2_OFFSET: usize = NOMAD_PPD_OPEN_ROLL1_OFFSET + 4;

pub(crate) const NOMAD_PPD_OPEN_ROLL3_OFFSET: usize = NOMAD_PPD_OPEN_ROLL1_OFFSET + 8;

pub(crate) const NOMAD_PPD_OPEN_BET_OFFSET: usize = NOMAD_PPD_OPEN_ROLL1_OFFSET + 12;

pub(crate) const NOMAD_PPD_TRIBE_MEMBER_OFFSET: usize = NOMAD_PPD_OPEN_ROLL1_OFFSET + 16;

pub(crate) const CALIGAR_PPD_WATCH_FLAG_OFFSET: usize = 4 * 4;

pub(crate) const CALIGAR_PPD_DOOR_FLAG_OFFSET: usize = 14 * 4;

pub(crate) const CALIGAR_PPD_DOOR_FLAG_COUNT: usize = 4;

pub const ARKHATA_PPD_CLERK_STATE_OFFSET: usize = 16 * 4;

pub const ARKHATA_PPD_CLERK_TIME_OFFSET: usize = 17 * 4;

// `struct staffer_ppd` field offsets (`src/common/staffer_ppd.h:13-` /
// `src/system/game/ppd_structs.h:566-`), in declaration order. Only the
// fields consumed by `questlog_init_staff` (`src/system/questlog.c:1203-
// 1394`) plus the two pre-existing named fields below have accessors.
pub(crate) const STAFFER_PPD_SMUGGLECOM_STATE_OFFSET: usize = 0 * 4;

pub(crate) const STAFFER_PPD_SMUGGLECOM_BITS_OFFSET: usize = 1 * 4;

pub(crate) const STAFFER_PPD_CARLOS_STATE_OFFSET: usize = 2 * 4;

pub(crate) const STAFFER_PPD_COUNTBRAN_STATE_OFFSET: usize = 3 * 4;

pub(crate) const STAFFER_PPD_COUNTBRAN_BITS_OFFSET: usize = 4 * 4;

pub(crate) const STAFFER_PPD_SPIRITBRAN_STATE_OFFSET: usize = 7 * 4;

pub(crate) const STAFFER_PPD_BRENNETHBRAN_STATE_OFFSET: usize = 9 * 4;

pub(crate) const STAFFER_PPD_BROKLIN_STATE_OFFSET: usize = 12 * 4;

pub(crate) const STAFFER_PPD_ARISTOCRAT_STATE_OFFSET: usize = 13 * 4;

pub(crate) const STAFFER_PPD_YOATIN_STATE_OFFSET: usize = 14 * 4;

pub(crate) const STAFFER_PPD_SHANRA_STATE_OFFSET: usize = 16 * 4;

pub(crate) const STAFFER_PPD_DWARFCHIEF_STATE_OFFSET: usize = 18 * 4;

pub(crate) const STAFFER_PPD_DWARFSHAMAN_STATE_OFFSET: usize = 19 * 4;

/// C `struct staffer_ppd::carlos2_state` (`src/common/staffer_ppd.h:43`,
/// field index 23 - the last field before `rouven_state`), consumed by
/// `carlos_driver`'s Imperial Vault ritual quest (`src/area/3/area3.c`).
pub(crate) const STAFFER_PPD_CARLOS2_STATE_OFFSET: usize = 23 * 4;

pub(crate) const FARMY_PPD_BOSS_STAGE_OFFSET: usize = 0;

/// C `struct farmy_ppd::boss_timer` (`src/area/8/fdemon.c:362`, second
/// field). `fdemon_boss`'s per-NT_CHAR-sighting throttle (`realtime -
/// ppd->boss_timer > 5`).
pub(crate) const FARMY_PPD_BOSS_TIMER_OFFSET: usize = 4;

/// C `struct farmy_ppd::boss_counter` (`src/area/8/fdemon.c:366`): offset is
/// `2 ints` (`boss_stage`/`boss_timer`) plus `MAXSOLDIER(3) * sizeof(struct
/// soldier)`. `struct soldier` is `7 ints` (`type`/`rank`/`base`/`profile`/
/// `exp`/`cn`/`serial`) plus one embedded `struct emote` (`8 ints` +
/// `likes[MAXSOLDIER+1]`/`talked[MAXSOLDIER+1]` = `2*4 ints` + `4 ints`
/// (`answer_timer`/`answer_cn`/`answer_type`/`last_emote`) = `20 ints`), so
/// `struct soldier` is `27 ints` = `108` bytes; `3 * 108 = 324` bytes of
/// soldier array, giving `boss_counter` offset `8 + 324 = 332`. The
/// `type`/`rank`/`base`/`profile`/`exp`/`cn`/`serial` prefix of each
/// soldier slot is now exposed via typed accessors (see
/// `FARMY_SOLDIER_ARRAY_OFFSET`/`FARMY_SOLDIER_STRIDE` below and
/// `PlayerRuntime::farmy_soldier_type`/etc. in `areas_misc.rs`); the
/// embedded `struct emote` sub-fields (offsets `+28..+108` within each
/// soldier slot) remain unexposed pending the `CDR_FDEMON_ARMY` emote
/// engine - only their *size* matters here to keep `boss_counter`/
/// `boss_reported` at their correct legacy byte offsets so that block
/// round-trips unmodified through unrelated `PlayerRuntime` saves.
pub(crate) const FARMY_PPD_BOSS_COUNTER_OFFSET: usize = 332;

/// C `struct farmy_ppd::boss_reported` (`src/area/8/fdemon.c:367`, right
/// after `boss_counter`).
pub(crate) const FARMY_PPD_BOSS_REPORTED_OFFSET: usize = 336;

/// Byte offset of `struct farmy_ppd::soldier[0]` (`src/area/8/fdemon.c:364`):
/// right after `boss_stage`/`boss_timer` (`2 ints` = `8` bytes).
pub(crate) const FARMY_SOLDIER_ARRAY_OFFSET: usize = 8;

/// `sizeof(struct soldier)` (`src/area/8/fdemon.c:346-358`): `7 ints`
/// (`type`/`rank`/`base`/`profile`/`exp`/`cn`/`serial`) + embedded `struct
/// emote` (`20 ints`) = `27 ints` = `108` bytes. See
/// `FARMY_PPD_BOSS_COUNTER_OFFSET`'s doc comment for the full breakdown.
pub(crate) const FARMY_SOLDIER_STRIDE: usize = 108;

/// Field offsets within one `struct soldier` slot (`src/area/8/fdemon.c:
/// 346-358`), in declaration order.
pub(crate) const FARMY_SOLDIER_TYPE_FIELD: usize = 0;
pub(crate) const FARMY_SOLDIER_RANK_FIELD: usize = 4;
pub(crate) const FARMY_SOLDIER_BASE_FIELD: usize = 8;
pub(crate) const FARMY_SOLDIER_PROFILE_FIELD: usize = 12;
pub(crate) const FARMY_SOLDIER_EXP_FIELD: usize = 16;
pub(crate) const FARMY_SOLDIER_CN_FIELD: usize = 20;
pub(crate) const FARMY_SOLDIER_SERIAL_FIELD: usize = 24;

/// Field offsets within one `struct soldier` slot's embedded `struct emote`
/// (`src/area/8/fdemon.c:324-344`), in declaration order, starting right
/// after the `type`/`rank`/`base`/`profile`/`exp`/`cn`/`serial` prefix
/// (`7 ints` = `28` bytes - see `FARMY_SOLDIER_SERIAL_FIELD`). Consumed by
/// `PlayerRuntime::farmy_soldier_emote`/`set_farmy_soldier_emote`
/// (`areas_misc.rs`), which carry `struct emote` across a recruit/drop/
/// re-recruit cycle (C `take_soldiers`/`drop_soldiers`,
/// `fdemon.c:559-563,608-612`).
pub(crate) const FARMY_SOLDIER_EMOTE_CUDDLY_FIELD: usize = 28;
pub(crate) const FARMY_SOLDIER_EMOTE_LONELY_FIELD: usize = 32;
pub(crate) const FARMY_SOLDIER_EMOTE_ANGST_FIELD: usize = 36;
pub(crate) const FARMY_SOLDIER_EMOTE_FEAR_FIELD: usize = 40;
pub(crate) const FARMY_SOLDIER_EMOTE_BORE_FIELD: usize = 44;
pub(crate) const FARMY_SOLDIER_EMOTE_BOREDOM_FIELD: usize = 48;
pub(crate) const FARMY_SOLDIER_EMOTE_BIGMOUTH_FIELD: usize = 52;
pub(crate) const FARMY_SOLDIER_EMOTE_PRAISE_FIELD: usize = 56;
/// `int likes[MAXSOLDIER + 1]`: 4 consecutive ints starting here.
pub(crate) const FARMY_SOLDIER_EMOTE_LIKES_FIELD: usize = 60;
/// `int talked[MAXSOLDIER + 1]`: 4 consecutive ints starting here.
pub(crate) const FARMY_SOLDIER_EMOTE_TALKED_FIELD: usize = 76;
pub(crate) const FARMY_SOLDIER_EMOTE_ANSWER_TIMER_FIELD: usize = 92;
pub(crate) const FARMY_SOLDIER_EMOTE_ANSWER_CN_FIELD: usize = 96;
pub(crate) const FARMY_SOLDIER_EMOTE_ANSWER_TYPE_FIELD: usize = 100;
pub(crate) const FARMY_SOLDIER_EMOTE_LAST_EMOTE_FIELD: usize = 104;

pub(crate) const TEUFELRAT_PPD_KILLS_OFFSET: usize = 0;

pub(crate) const TEUFELRAT_PPD_SCORE_OFFSET: usize = 4;

pub(crate) const BANK_PPD_IMPERIAL_GOLD_OFFSET: usize = 0;

pub(crate) const TWOCITY_PPD_GOODTILE_OFFSET: usize = 19 * 4;

pub(crate) const TWOCITY_PPD_SOLVED_LIBRARY_OFFSET: usize = 24 * 4;

pub(crate) const TWOCITY_PPD_THIEF_STATE_OFFSET: usize = 8 * 4;

/// C `struct twocity_ppd::thief_last_seen` (`common/two_ppd.h:18`):
/// wall-clock `realtime` stamp of the last successful `thiefmaster`
/// greeting, read back by its own `thief_state == 9` waiting-for-mission
/// nag (`two.c:1849-1854`).
pub(crate) const TWOCITY_PPD_THIEF_LAST_SEEN_OFFSET: usize = 9 * 4;

pub(crate) const TWOCITY_PPD_THIEF_KILLED_OFFSET: usize = 10 * 4;

/// C `struct twocity_ppd::thief_bits` (`common/two_ppd.h:24`): a 4-bit
/// mask (`1`/`2`/`4`/`8`) of which of `thiefmaster`'s four lockpick-chain
/// missions (quests 25-28) have already been turned in. Write-only in C
/// itself beyond JSON debug serialization (`character.c:656`) - no other
/// C code reads it back - ported for `player_state_json` fidelity.
pub(crate) const TWOCITY_PPD_THIEF_BITS_OFFSET: usize = 18 * 4;

pub(crate) const TWOCITY_PPD_SANWYN_STATE_OFFSET: usize = 16 * 4;

pub(crate) const TWOCITY_PPD_SANWYN_BITS_OFFSET: usize = 17 * 4;

pub(crate) const TWOCITY_PPD_SKELLY_STATE_OFFSET: usize = 27 * 4;

pub(crate) const TWOCITY_PPD_ALCHEMIST_STATE_OFFSET: usize = 28 * 4;

/// C `struct twocity_ppd::legal_status` (`common/two_ppd.h:2`): `LS_CLEAN`
/// (`0`)/`LS_FINE` (`1`)/`LS_DEAD` (`2`), the player's standing with the
/// Exkordon city guard.
pub(crate) const TWOCITY_PPD_LEGAL_STATUS_OFFSET: usize = 0;

/// C `struct twocity_ppd::legal_fine` (`common/two_ppd.h:3`): accumulated
/// fine (in raw gold units), added to a guest pass's price while
/// `legal_status == LS_FINE`.
pub(crate) const TWOCITY_PPD_LEGAL_FINE_OFFSET: usize = 4;

/// C `struct twocity_ppd::citizen_status` (`common/two_ppd.h:5`):
/// `CS_ENEMY` (`0`)/`CS_GUEST` (`1`)/`CS_CITIZEN` (`2`)/`CS_HONOR` (`3`).
pub(crate) const TWOCITY_PPD_CITIZEN_STATUS_OFFSET: usize = 2 * 4;

/// C `struct twocity_ppd::barkeeper_state` (`common/two_ppd.h:11`): the
/// tavern barkeeper's 3-state greeting/guest-pass-offer ladder.
pub(crate) const TWOCITY_PPD_BARKEEPER_STATE_OFFSET: usize = 6 * 4;

/// C `struct twocity_ppd::barkeeper_last` (`common/two_ppd.h:29`): wall-
/// clock `realtime` stamp of the last guest-pass offer.
pub(crate) const TWOCITY_PPD_BARKEEPER_LAST_OFFSET: usize = 26 * 4;

/// C `struct twocity_ppd::current_guard` (`common/two_ppd.h:10`): the
/// character id of the guard currently pursuing/warning this player
/// (`0` if none), so only that guard (or one whose claim has timed out)
/// re-triggers the leave/fine warning ladder.
pub(crate) const TWOCITY_PPD_CURRENT_GUARD_OFFSET: usize = 3 * 4;

/// C `struct twocity_ppd::current_guard_time` (`common/two_ppd.h:11`):
/// wall-clock `realtime` stamp of `current_guard`'s claim.
pub(crate) const TWOCITY_PPD_CURRENT_GUARD_TIME_OFFSET: usize = 4 * 4;

/// C `struct twocity_ppd::last_attack` (`common/two_ppd.h:12`): wall-clock
/// `realtime` stamp of the last guard-fine-triggering event (attacking a
/// guard, attacking a bystander under guard protection, lockpicking,
/// killing a guard), rate-limiting repeated fines.
pub(crate) const TWOCITY_PPD_LAST_ATTACK_OFFSET: usize = 5 * 4;

/// C `struct twocity_ppd::guard_intro` (`common/two_ppd.h:15`): whether
/// `guard_driver` has already given the one-time "thou art here on a
/// guest pass" warning speech.
pub(crate) const TWOCITY_PPD_GUARD_INTRO_OFFSET: usize = 7 * 4;

pub(crate) const MISC_PPD_COMPLAINT_DATE_OFFSET: usize = 4;

/// C `struct misc_ppd::supermax_state` (`src/common/misc_ppd.h:28`): the
/// `supermax_driver` (`src/area/3/area3.c`) greeting-sequence counter
/// (0..3, plateauing at 4 once the full greeting has played).
pub(crate) const MISC_PPD_SUPERMAX_STATE_OFFSET: usize = 12;

/// C `struct misc_ppd::supermax_gold` (`src/common/misc_ppd.h:29`):
/// cumulative gold this player has paid `supermax_driver` for
/// past-max raises.
pub(crate) const MISC_PPD_SUPERMAX_GOLD_OFFSET: usize = 16;

pub(crate) const MISC_PPD_SWAPPED_OFFSET: usize = 20;

pub(crate) const MISC_PPD_TREEDONE_OFFSET: usize = 24;

pub(crate) const MISC_PPD_GIFT_YEAR_OFFSET: usize = 32;

// `struct lostcon_ppd` field offsets (`src/module/lostcon.h:18-36`), in
// declaration order (0-based `int` index * 4).
pub(crate) const LOSTCON_PPD_AUTOBLESS_OFFSET: usize = 0 * 4;

pub(crate) const LOSTCON_PPD_AUTOPULSE_OFFSET: usize = 1 * 4;

pub(crate) const LOSTCON_PPD_NOBLESS_OFFSET: usize = 2 * 4;

pub(crate) const LOSTCON_PPD_NOHEAL_OFFSET: usize = 3 * 4;

pub(crate) const LOSTCON_PPD_NOFLASH_OFFSET: usize = 4 * 4;

pub(crate) const LOSTCON_PPD_NOFIREBALL_OFFSET: usize = 5 * 4;

pub(crate) const LOSTCON_PPD_NOBALL_OFFSET: usize = 6 * 4;

pub(crate) const LOSTCON_PPD_NOSHIELD_OFFSET: usize = 7 * 4;

pub(crate) const LOSTCON_PPD_NOWARCRY_OFFSET: usize = 8 * 4;

pub(crate) const LOSTCON_PPD_NOFREEZE_OFFSET: usize = 9 * 4;

pub(crate) const LOSTCON_PPD_NOMANA_OFFSET: usize = 10 * 4;

pub(crate) const LOSTCON_PPD_NOLIFE_OFFSET: usize = 11 * 4;

pub(crate) const LOSTCON_PPD_NOCOMBO_OFFSET: usize = 12 * 4;

pub(crate) const LOSTCON_PPD_NOMOVE_OFFSET: usize = 13 * 4;

pub(crate) const LOSTCON_PPD_NOPULSE_OFFSET: usize = 14 * 4;

pub(crate) const LOSTCON_PPD_NORECALL_OFFSET: usize = 15 * 4;

pub(crate) const LOSTCON_PPD_AUTOTURN_OFFSET: usize = 16 * 4;

pub(crate) const LOSTCON_PPD_MAXLAG_OFFSET: usize = 17 * 4;

pub(crate) const LOSTCON_PPD_HINTS_OFFSET: usize = 18 * 4;

pub(crate) const STAFFER_PPD_FORESTBRAN_DONE_OFFSET: usize = 11 * 4;

pub(crate) const PK_PPD_KILLS_OFFSET: usize = 0;

pub(crate) const PK_PPD_DEATHS_OFFSET: usize = 4;

pub(crate) const PK_PPD_LAST_KILL_OFFSET: usize = 8;

pub(crate) const PK_PPD_LAST_DEATH_OFFSET: usize = 12;

pub(crate) const PK_PPD_HATE_OFFSET: usize = 16;

pub(crate) const RUNE_PPD_SPECIAL_EXEC_OFFSET: usize = RUNE_USED_WORDS * 4;

pub(crate) const SWEAR_PPD_BANNED_TILL_OFFSET: usize = LEGACY_SWEAR_PPD_SIZE - 4;

// `struct item` byte layout (`src/system/server.h`) as persisted inside a
// `depot_ppd` slot - see `PlayerRuntime::encode_legacy_depot_item`. Same
// physical layout `ugaris-server::depot`'s `legacy_account_depot_codec`
// module independently encodes for `AccountDepotState`.
pub(crate) const DEPOT_PPD_ITEM_SIZE: usize = 232;

pub(crate) const DEPOT_PPD_ITEM_PERSISTED_PREFIX: usize = 224;

pub(crate) const DEPOT_PPD_ITEM_FLAGS_OFFSET: usize = 0;

pub(crate) const DEPOT_PPD_ITEM_NAME_OFFSET: usize = 8;

pub(crate) const DEPOT_PPD_ITEM_NAME_LEN: usize = 40;

pub(crate) const DEPOT_PPD_ITEM_DESCRIPTION_OFFSET: usize = 48;

pub(crate) const DEPOT_PPD_ITEM_DESCRIPTION_LEN: usize = 80;

pub(crate) const DEPOT_PPD_ITEM_VALUE_OFFSET: usize = 128;

pub(crate) const DEPOT_PPD_ITEM_MIN_LEVEL_OFFSET: usize = 132;

pub(crate) const DEPOT_PPD_ITEM_MAX_LEVEL_OFFSET: usize = 133;

pub(crate) const DEPOT_PPD_ITEM_NEEDS_CLASS_OFFSET: usize = 134;

pub(crate) const DEPOT_PPD_ITEM_OWNER_OFFSET: usize = 136;

pub(crate) const DEPOT_PPD_ITEM_MOD_INDEX_OFFSET: usize = 140;

pub(crate) const DEPOT_PPD_ITEM_MOD_VALUE_OFFSET: usize = 150;

pub(crate) const DEPOT_PPD_ITEM_CONTENT_OFFSET: usize = 168;

pub(crate) const DEPOT_PPD_ITEM_DRIVER_OFFSET: usize = 170;

pub(crate) const DEPOT_PPD_ITEM_DRDATA_OFFSET: usize = 172;

pub(crate) const DEPOT_PPD_ITEM_DRDATA_LEN: usize = 40;

pub(crate) const DEPOT_PPD_ITEM_TEMPLATE_ID_OFFSET: usize = 212;

pub(crate) const DEPOT_PPD_ITEM_SERIAL_OFFSET: usize = 216;

pub(crate) const DEPOT_PPD_ITEM_SPRITE_OFFSET: usize = 220;

pub const DEFERRED_ACHIEVEMENTS: u32 = 1 << 0;

pub const DEFERRED_MOTD: u32 = 1 << 1;

pub const DEFERRED_AUCTION: u32 = 1 << 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PlayerConnectionState {
    Connect = 1,
    Normal = 2,
    Exit = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PlayerActionCode {
    Idle = 0,
    Move = 1,
    Take = 2,
    Drop = 3,
    Kill = 4,
    Use = 5,
    Bless = 6,
    Heal = 7,
    Freeze = 8,
    Fireball = 9,
    Ball = 10,
    MagicShield = 11,
    Flash = 12,
    Warcry = 13,
    LookMap = 14,
    Give = 15,
    FireballCharacter = 16,
    BallCharacter = 17,
    Teleport = 18,
    Pulse = 19,
    WalkDir = 20,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyringAddResult {
    Added,
    Duplicate,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecialShrineResult {
    NothingHere,
    ConfirmRequired,
    HardcoreRemoved,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DemonShrineResult {
    Learned { exp_added: u32 },
    AlreadyKnown,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum XmasTreeResult {
    Dormant,
    AlreadyGranted,
    NeedsHolidayTreat,
    GiftGranted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoneHintResult {
    Hint {
        page: u16,
        rune: &'static str,
        position: &'static str,
    },
    Bug {
        level: u8,
        nr: u8,
        pos: u8,
        value: i32,
    },
}

/// See [`PlayerRuntime::pentagram_debug`]'s doc comment.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PentagramDebugData {
    pub status: i32,
    pub pent_it: [i32; 6],
    pub pent_color: [i32; 6],
    pub pent_value: [i32; 6],
    pub pent_worth: [i32; 6],
    pub bonus: i32,
    pub pent_cnt: i32,
    pub lucky_pents_this_solve: i32,
}

/// C `#define MACRO_HISTORY_SIZE 10` (`command.c:570`).
pub const MACRO_HISTORY_SIZE: usize = 10;

/// See [`PlayerRuntime::macro_ppd`]'s doc comment. One slot of
/// `macro_ppd.history[MACRO_HISTORY_SIZE]`, a circular buffer of the
/// player's most recent macro-daemon challenges (C `struct
/// macro_history_entry`, `command.c:578-583`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroHistoryEntry {
    pub timestamp: i64,
    pub challenge_type: i32,
    pub passed: bool,
    pub response_time: i32,
}

/// See [`PlayerRuntime::macro_ppd`]'s doc comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroPpd {
    pub nextcheck: i64,
    pub karma: i32,
    pub last_exp_gain: i64,
    pub last_combat: i64,
    pub last_gold_change: i64,
    pub suspicion: i32,
    pub challenge_failures: i32,
    pub original_x: i32,
    pub original_y: i32,
    pub original_area: i32,
    pub original_restx: i32,
    pub original_resty: i32,
    pub original_resta: i32,
    pub in_challenge_room: bool,
    pub history: [MacroHistoryEntry; MACRO_HISTORY_SIZE],
    pub history_count: i32,
    pub history_index: i32,
    pub total_passed: i32,
    pub total_failed: i32,
    pub immune_until: i64,
    /// C `int immune_by` - the character ID of whoever granted immunity, 0
    /// if none.
    pub immune_by: u32,
    pub force_summon: bool,
    /// C `int summoned_by` - the character ID of whoever requested the
    /// forced summon, 0 if none.
    pub summoned_by: u32,
    pub needs_challenge: bool,
    // Saved pentagram data, restored when returning from the (unported)
    // cross-server challenge room. Kept as its own separate fields
    // matching C's `struct macro_ppd` layout exactly rather than reusing
    // `PentagramDebugData`, since the C source itself duplicates these
    // fields across the two structs instead of sharing one.
    pub saved_pent_valid: bool,
    pub saved_pent_status: i32,
    pub saved_pent_it: [i32; 6],
    pub saved_pent_color: [i32; 6],
    pub saved_pent_value: [i32; 6],
    pub saved_pent_worth: [i32; 6],
    pub saved_pent_bonus: i32,
    pub saved_pent_cnt: i32,
    pub saved_pent_lucky: i32,
}

impl Default for MacroPpd {
    fn default() -> Self {
        MacroPpd {
            nextcheck: 0,
            karma: 0,
            last_exp_gain: 0,
            last_combat: 0,
            last_gold_change: 0,
            suspicion: 0,
            challenge_failures: 0,
            original_x: 0,
            original_y: 0,
            original_area: 0,
            original_restx: 0,
            original_resty: 0,
            original_resta: 0,
            in_challenge_room: false,
            history: [MacroHistoryEntry::default(); MACRO_HISTORY_SIZE],
            history_count: 0,
            history_index: 0,
            total_passed: 0,
            total_failed: 0,
            immune_until: 0,
            immune_by: 0,
            force_summon: false,
            summoned_by: 0,
            needs_challenge: false,
            saved_pent_valid: false,
            saved_pent_status: 0,
            saved_pent_it: [0; 6],
            saved_pent_color: [0; 6],
            saved_pent_value: [0; 6],
            saved_pent_worth: [0; 6],
            saved_pent_bonus: 0,
            saved_pent_cnt: 0,
            saved_pent_lucky: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgnoreToggleResult {
    Added,
    Removed,
    Full,
}

/// Per-quest outcome of the `questlog_reopen_qN` switch dispatch inside
/// `PlayerRuntime::reopen_quest_legacy` (`src/system/questlog.c:637-817`),
/// before it's translated into the public `QuestReopenResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReopenOutcome {
    Open,
    SeriesConflict,
    NoEffect,
}

impl PlayerRuntime {
    pub fn encode_legacy_orbspawn_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_ORBSPAWN_PPD_SIZE];
        for (index, entry) in self
            .orb_spawns
            .iter()
            .take(ORBSPAWN_MAX_ENTRIES)
            .enumerate()
        {
            write_i32(
                &mut bytes,
                ORBSPAWN_PPD_IDS_OFFSET + index * 4,
                entry.location_id.min(i32::MAX as u32) as i32,
            );
            write_i32(
                &mut bytes,
                ORBSPAWN_PPD_LAST_USED_OFFSET + index * 4,
                entry.last_used_seconds.min(i32::MAX as u64) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_orbspawn_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_ORBSPAWN_PPD_SIZE {
            return false;
        }

        self.orb_spawns.clear();
        for index in 0..ORBSPAWN_MAX_ENTRIES {
            let location_id = read_i32(bytes, ORBSPAWN_PPD_IDS_OFFSET + index * 4);
            let last_used_seconds = read_i32(bytes, ORBSPAWN_PPD_LAST_USED_OFFSET + index * 4);
            if location_id > 0 && last_used_seconds > 0 {
                self.orb_spawns.push(OrbSpawnAccess {
                    location_id: location_id as u32,
                    last_used_seconds: last_used_seconds as u64,
                });
            }
        }
        true
    }

    pub fn encode_legacy_misc_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_MISC_PPD_SIZE];
        let copy_len = self.misc_ppd.len().min(LEGACY_MISC_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.misc_ppd[..copy_len]);
        bytes
    }

    pub fn decode_legacy_misc_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_MISC_PPD_SIZE {
            return false;
        }

        self.misc_ppd = bytes[..LEGACY_MISC_PPD_SIZE].to_vec();
        true
    }

    pub fn encode_legacy_firstkill_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_FIRSTKILL_PPD_SIZE];
        let copy_len = self.first_kill_ppd.len().min(LEGACY_FIRSTKILL_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.first_kill_ppd[..copy_len]);
        bytes
    }

    pub fn decode_legacy_firstkill_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_FIRSTKILL_PPD_SIZE {
            return false;
        }
        self.first_kill_ppd = bytes[..LEGACY_FIRSTKILL_PPD_SIZE].to_vec();
        true
    }

    /// C `give_first_kill`'s bit-test/set (`death.c:196-222`): `index =
    /// ch[co].class / 32; offset = ch[co].class & 31; mask = 1 << offset;
    /// if (ppd->kill[index] & mask) return; ppd->kill[index] |= mask;` -
    /// reworked here as a flat byte/bit-in-byte pair (`class / 8`, `class %
    /// 8`), which addresses the exact same bit in a little-endian `u32[32]`
    /// laid out as 128 raw bytes. Returns `true` the first time `class` is
    /// killed (and records it), `false` on every repeat.
    pub fn mark_first_kill(&mut self, class: i32) -> bool {
        if !(1..=1023).contains(&class) {
            return false;
        }
        if self.first_kill_ppd.len() < LEGACY_FIRSTKILL_PPD_SIZE {
            self.first_kill_ppd.resize(LEGACY_FIRSTKILL_PPD_SIZE, 0);
        }
        let class = class as usize;
        let byte = class / 8;
        let bit = 1u8 << (class % 8);
        if self.first_kill_ppd[byte] & bit != 0 {
            return false;
        }
        self.first_kill_ppd[byte] |= bit;
        true
    }

    /// C `ppd->kill[index] & mask` bit-test (`death.c:196-197`, also
    /// inlined directly in `command.c:1193/1200` by `/pentinfo`'s sibling
    /// `cmd_demonlords`), exposed as a query so callers other than
    /// [`Self::mark_first_kill`] (which also sets the bit) can check
    /// without mutating. Out-of-range classes (matching `mark_first_kill`'s
    /// own guard) are always reported unkilled.
    pub fn has_first_kill(&self, class: i32) -> bool {
        if !(1..=1023).contains(&class) || self.first_kill_ppd.is_empty() {
            return false;
        }
        let class = class as usize;
        let byte = class / 8;
        byte < self.first_kill_ppd.len() && self.first_kill_ppd[byte] & (1 << (class % 8)) != 0
    }

    /// C `count_demon_lord_kills` (`death.c:169-190`): counts unique
    /// first-killed classes in `258..=305` (Earth/Fire/Ice demon lords) and
    /// `404..=411` (Hell demon lords).
    pub fn count_demon_lord_kills(&self) -> u32 {
        if self.first_kill_ppd.is_empty() {
            return 0;
        }
        (258..=305).filter(|&c| self.has_first_kill(c)).count() as u32
            + (404..=411).filter(|&c| self.has_first_kill(c)).count() as u32
    }

    pub fn treasure_dig_last_seconds(&self, dig_index: u8) -> u64 {
        self.treasure_dig_last_seconds
            .get(usize::from(dig_index))
            .copied()
            .unwrap_or_default()
    }

    pub fn mark_treasure_dig(&mut self, dig_index: u8, realtime_seconds: u64) -> bool {
        let Some(last_dig) = self
            .treasure_dig_last_seconds
            .get_mut(usize::from(dig_index))
        else {
            return false;
        };
        *last_dig = realtime_seconds;
        true
    }

    pub fn encode_legacy_treasure_dig_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_TREASURE_DIG_PPD_SIZE];
        for (index, last_dig_seconds) in self.treasure_dig_last_seconds.iter().copied().enumerate()
        {
            write_i32(
                &mut bytes,
                index * 4,
                last_dig_seconds.min(i32::MAX as u64) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_treasure_dig_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_TREASURE_DIG_PPD_SIZE {
            return false;
        }
        for index in 0..TREASURE_DIG_PPD_ENTRIES {
            self.treasure_dig_last_seconds[index] = read_i32(bytes, index * 4).max(0) as u64;
        }
        true
    }

    pub fn flower_last_used_seconds(&self, location_id: u32) -> Option<u64> {
        self.flowers
            .iter()
            .find(|entry| entry.location_id == location_id)
            .map(|entry| entry.last_used_seconds)
    }

    pub fn mark_flower_used(&mut self, location_id: u32, realtime_seconds: u64) {
        if let Some(entry) = self
            .flowers
            .iter_mut()
            .find(|entry| entry.location_id == location_id)
        {
            entry.last_used_seconds = realtime_seconds;
            return;
        }

        if self.flowers.len() < FLOWER_MAX_ENTRIES {
            self.flowers.push(FlowerAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            });
            return;
        }

        if let Some(oldest) = self
            .flowers
            .iter_mut()
            .min_by_key(|entry| entry.last_used_seconds)
        {
            *oldest = FlowerAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            };
        }
    }

    pub fn encode_legacy_flower_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_FLOWER_PPD_SIZE];
        for (index, entry) in self.flowers.iter().take(FLOWER_MAX_ENTRIES).enumerate() {
            write_i32(
                &mut bytes,
                FLOWER_PPD_IDS_OFFSET + index * 4,
                entry.location_id.min(i32::MAX as u32) as i32,
            );
            write_i32(
                &mut bytes,
                FLOWER_PPD_LAST_USED_OFFSET + index * 4,
                entry.last_used_seconds.min(i32::MAX as u64) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_flower_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_FLOWER_PPD_SIZE {
            return false;
        }
        self.flowers.clear();
        for index in 0..FLOWER_MAX_ENTRIES {
            let location_id = read_i32(bytes, FLOWER_PPD_IDS_OFFSET + index * 4);
            let last_used = read_i32(bytes, FLOWER_PPD_LAST_USED_OFFSET + index * 4);
            if location_id > 0 || last_used > 0 {
                self.flowers.push(FlowerAccess {
                    location_id: location_id.max(0) as u32,
                    last_used_seconds: last_used.max(0) as u64,
                });
            }
        }
        true
    }

    pub fn encode_legacy_stats_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_STATS_PPD_SIZE];
        let len = self.stats_ppd.len().min(LEGACY_STATS_PPD_SIZE);
        bytes[..len].copy_from_slice(&self.stats_ppd[..len]);
        bytes
    }

    pub fn decode_legacy_stats_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_STATS_PPD_SIZE {
            return false;
        }
        self.stats_ppd = bytes[..LEGACY_STATS_PPD_SIZE].to_vec();
        true
    }

    /// C `stats_update` (`src/system/statistics.c:23-45`): called once per
    /// real-time minute per connected player (`player_update`, `player.c:
    /// 3460`, `stats_update(cn, 1, 0)`, ported at `award_play_time_minute`'s
    /// call site in `ugaris-server`'s `main.rs`) plus on every store sale
    /// and money-item destruction (`store.c:381`/`do.c:1282`,
    /// `stats_update(cn, 0, price)` - not yet wired, see `PORTING_TODO.md`'s
    /// "Cross-area transfer" task's Progress Log: `.gold`/`.exp` are
    /// write-only fields nothing in this codebase reads yet, unlike
    /// `.online`, which `stats_online_time` sums). Maintains a
    /// `STATS_PPD_MAXSTAT`(365)-day rolling ring buffer of daily
    /// exp/gold/online samples, zeroing every day bucket skipped since the
    /// last update (a player who was offline for more than 365 days wraps
    /// all the way around, clearing the whole buffer - matching C's own
    /// `while (lidx != idx) { lidx = (lidx+1) % MAXSTAT; bzero(...); }`
    /// loop exactly, run against `self.stats_ppd`'s raw legacy bytes
    /// in-place rather than a decoded struct). `now`/`last_update` are
    /// wall-clock unix seconds (the caller's `current_unix_time()`);
    /// `STATS_PPD_STARTTIME` is subtracted first to match C's own
    /// `realtime = time_now - STARTTIME` day-bucketing exactly. Lazily
    /// zero-initializes `self.stats_ppd` on first use, mirroring C's
    /// `set_data` zero-allocating a fresh `stats_ppd` the first time any
    /// character calls this.
    pub fn stats_update(&mut self, character_exp: i32, online_minutes: i32, gold: i32, now: i64) {
        if self.stats_ppd.len() < LEGACY_STATS_PPD_SIZE {
            self.stats_ppd = vec![0; LEGACY_STATS_PPD_SIZE];
        }
        let real_now = now - STATS_PPD_STARTTIME;
        let idx = real_now
            .div_euclid(STATS_PPD_RESOLUTION_SECONDS)
            .rem_euclid(STATS_PPD_MAXSTAT as i64) as usize;
        let last_update = i64::from(read_i32(&self.stats_ppd, STATS_PPD_LAST_UPDATE_OFFSET));
        let mut lidx = last_update
            .div_euclid(STATS_PPD_RESOLUTION_SECONDS)
            .rem_euclid(STATS_PPD_MAXSTAT as i64) as usize;
        while lidx != idx {
            lidx = (lidx + 1) % STATS_PPD_MAXSTAT;
            let offset = stats_ppd_day_offset(lidx);
            write_i32(&mut self.stats_ppd, offset + STATS_PPD_DAY_EXP_OFFSET, 0);
            write_i32(&mut self.stats_ppd, offset + STATS_PPD_DAY_GOLD_OFFSET, 0);
            write_i32(&mut self.stats_ppd, offset + STATS_PPD_DAY_ONLINE_OFFSET, 0);
        }
        write_i32(
            &mut self.stats_ppd,
            STATS_PPD_LAST_UPDATE_OFFSET,
            real_now as i32,
        );
        let offset = stats_ppd_day_offset(idx);
        write_i32(
            &mut self.stats_ppd,
            offset + STATS_PPD_DAY_EXP_OFFSET,
            character_exp,
        );
        let gold_total =
            read_i32(&self.stats_ppd, offset + STATS_PPD_DAY_GOLD_OFFSET).saturating_add(gold);
        write_i32(
            &mut self.stats_ppd,
            offset + STATS_PPD_DAY_GOLD_OFFSET,
            gold_total,
        );
        let online_total = read_i32(&self.stats_ppd, offset + STATS_PPD_DAY_ONLINE_OFFSET)
            .saturating_add(online_minutes);
        write_i32(
            &mut self.stats_ppd,
            offset + STATS_PPD_DAY_ONLINE_OFFSET,
            online_total,
        );
    }

    /// C `stats_online_time` (`src/system/statistics.c:47-58`): sums every
    /// day bucket's `.online` sample across the whole 365-day ring buffer
    /// (`/values`' "Playing for %d hours." line, `tool.c:2917`, divides
    /// this by 60). Returns `0` for a character with no `stats_ppd` yet
    /// (mirrors C's `if (!ppd) return 0;`).
    pub fn stats_online_time(&self) -> i32 {
        if self.stats_ppd.len() < LEGACY_STATS_PPD_SIZE {
            return 0;
        }
        (0..STATS_PPD_MAXSTAT)
            .map(|day| {
                read_i32(
                    &self.stats_ppd,
                    stats_ppd_day_offset(day) + STATS_PPD_DAY_ONLINE_OFFSET,
                )
            })
            .sum()
    }

    pub fn encode_legacy_bank_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_BANK_PPD_SIZE];
        write_i32(
            &mut bytes,
            BANK_PPD_IMPERIAL_GOLD_OFFSET,
            self.bank_gold.min(i32::MAX as u32) as i32,
        );
        bytes
    }

    pub fn decode_legacy_bank_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_BANK_PPD_SIZE {
            return false;
        }
        self.bank_gold = read_i32(bytes, BANK_PPD_IMPERIAL_GOLD_OFFSET).max(0) as u32;
        true
    }

    /// Encodes one `struct item` (`src/system/server.h`) exactly like
    /// `ugaris-server::depot`'s `legacy_account_depot_codec` - both persist
    /// the same C struct, just for a different `DRD_*` id. Kept as an
    /// independent copy (crate-boundary duplication, not code reuse) since
    /// `ugaris-server` depends on `ugaris-core`, not the other way around.
    pub(crate) fn encode_legacy_depot_item(item: &Item) -> [u8; DEPOT_PPD_ITEM_SIZE] {
        let mut bytes = [0u8; DEPOT_PPD_ITEM_SIZE];
        write_u64(&mut bytes, DEPOT_PPD_ITEM_FLAGS_OFFSET, item.flags.bits());
        write_c_string(
            &mut bytes,
            DEPOT_PPD_ITEM_NAME_OFFSET,
            DEPOT_PPD_ITEM_NAME_LEN,
            &item.name,
        );
        write_c_string(
            &mut bytes,
            DEPOT_PPD_ITEM_DESCRIPTION_OFFSET,
            DEPOT_PPD_ITEM_DESCRIPTION_LEN,
            &item.description,
        );
        write_u32(&mut bytes, DEPOT_PPD_ITEM_VALUE_OFFSET, item.value);
        bytes[DEPOT_PPD_ITEM_MIN_LEVEL_OFFSET] = item.min_level;
        bytes[DEPOT_PPD_ITEM_MAX_LEVEL_OFFSET] = item.max_level;
        bytes[DEPOT_PPD_ITEM_NEEDS_CLASS_OFFSET] = item.needs_class;
        write_i32(&mut bytes, DEPOT_PPD_ITEM_OWNER_OFFSET, item.owner_id);
        for index in 0..MAX_MODIFIERS {
            let offset = DEPOT_PPD_ITEM_MOD_INDEX_OFFSET + index * 2;
            bytes[offset..offset + 2].copy_from_slice(&item.modifier_index[index].to_le_bytes());
            let offset = DEPOT_PPD_ITEM_MOD_VALUE_OFFSET + index * 2;
            bytes[offset..offset + 2].copy_from_slice(&item.modifier_value[index].to_le_bytes());
        }
        write_u16(&mut bytes, DEPOT_PPD_ITEM_CONTENT_OFFSET, item.content_id);
        write_u16(&mut bytes, DEPOT_PPD_ITEM_DRIVER_OFFSET, item.driver);
        let drdata_len = item.driver_data.len().min(DEPOT_PPD_ITEM_DRDATA_LEN);
        bytes[DEPOT_PPD_ITEM_DRDATA_OFFSET..DEPOT_PPD_ITEM_DRDATA_OFFSET + drdata_len]
            .copy_from_slice(&item.driver_data[..drdata_len]);
        write_u32(
            &mut bytes,
            DEPOT_PPD_ITEM_TEMPLATE_ID_OFFSET,
            item.template_id,
        );
        write_u32(&mut bytes, DEPOT_PPD_ITEM_SERIAL_OFFSET, item.serial);
        write_i32(&mut bytes, DEPOT_PPD_ITEM_SPRITE_OFFSET, item.sprite);
        bytes
    }

    /// Decodes one `struct item` slot; returns `None` for an empty slot
    /// (`flags == 0`, matching C's `if (ppd->itm[nr].flags)` emptiness
    /// check throughout `depot.c`) rather than `Some` with zeroed fields.
    pub(crate) fn decode_legacy_depot_item(bytes: &[u8], slot: usize) -> Option<Item> {
        if bytes.len() < DEPOT_PPD_ITEM_PERSISTED_PREFIX {
            return None;
        }
        let flags = read_u64(bytes, DEPOT_PPD_ITEM_FLAGS_OFFSET);
        if flags == 0 {
            return None;
        }
        let read_i16 = |offset: usize| i16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
        let mut modifier_index = [0i16; MAX_MODIFIERS];
        let mut modifier_value = [0i16; MAX_MODIFIERS];
        for index in 0..MAX_MODIFIERS {
            modifier_index[index] = read_i16(DEPOT_PPD_ITEM_MOD_INDEX_OFFSET + index * 2);
            modifier_value[index] = read_i16(DEPOT_PPD_ITEM_MOD_VALUE_OFFSET + index * 2);
        }
        Some(Item {
            id: ItemId((slot + 1) as u32),
            name: read_c_string(bytes, DEPOT_PPD_ITEM_NAME_OFFSET, DEPOT_PPD_ITEM_NAME_LEN),
            description: read_c_string(
                bytes,
                DEPOT_PPD_ITEM_DESCRIPTION_OFFSET,
                DEPOT_PPD_ITEM_DESCRIPTION_LEN,
            ),
            flags: ItemFlags::from_bits_retain(flags),
            sprite: read_i32(bytes, DEPOT_PPD_ITEM_SPRITE_OFFSET),
            value: read_u32(bytes, DEPOT_PPD_ITEM_VALUE_OFFSET),
            min_level: bytes[DEPOT_PPD_ITEM_MIN_LEVEL_OFFSET],
            max_level: bytes[DEPOT_PPD_ITEM_MAX_LEVEL_OFFSET],
            needs_class: bytes[DEPOT_PPD_ITEM_NEEDS_CLASS_OFFSET],
            template_id: read_u32(bytes, DEPOT_PPD_ITEM_TEMPLATE_ID_OFFSET),
            owner_id: read_i32(bytes, DEPOT_PPD_ITEM_OWNER_OFFSET),
            modifier_index,
            modifier_value,
            x: 0,
            y: 0,
            carried_by: None,
            contained_in: None,
            content_id: read_u16(bytes, DEPOT_PPD_ITEM_CONTENT_OFFSET),
            driver: read_u16(bytes, DEPOT_PPD_ITEM_DRIVER_OFFSET),
            driver_data: bytes[DEPOT_PPD_ITEM_DRDATA_OFFSET
                ..DEPOT_PPD_ITEM_DRDATA_OFFSET + DEPOT_PPD_ITEM_DRDATA_LEN]
                .to_vec(),
            serial: read_u32(bytes, DEPOT_PPD_ITEM_SERIAL_OFFSET),
        })
    }

    /// C `struct depot_ppd { struct item itm[MAXDEPOT]; }`: always encodes
    /// all `MAXDEPOT` fixed-size item records (unlike
    /// `ugaris-server::depot`'s `AccountDepotState`, which compacts empty
    /// slots out of its own variable-length subscriber-blob block), so a
    /// slot's index is preserved exactly across save/load.
    pub fn encode_legacy_depot_ppd(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(MAXDEPOT * DEPOT_PPD_ITEM_SIZE);
        for slot in 0..MAXDEPOT {
            match self.depot.get(slot).and_then(Option::as_ref) {
                Some(item) => bytes.extend_from_slice(&Self::encode_legacy_depot_item(item)),
                None => bytes.extend(std::iter::repeat_n(0u8, DEPOT_PPD_ITEM_SIZE)),
            }
        }
        bytes
    }

    pub fn decode_legacy_depot_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < MAXDEPOT * DEPOT_PPD_ITEM_SIZE {
            return false;
        }
        let mut depot = Self::default_depot();
        for (slot, chunk) in bytes
            .chunks_exact(DEPOT_PPD_ITEM_SIZE)
            .take(MAXDEPOT)
            .enumerate()
        {
            depot[slot] = Self::decode_legacy_depot_item(chunk, slot);
        }
        self.depot = depot;
        true
    }

    pub fn touch_xmas_tree(
        &mut self,
        area_id: u16,
        event_year: i32,
        is_xmas: bool,
        has_holiday_treat: bool,
    ) -> XmasTreeResult {
        if !is_xmas {
            return XmasTreeResult::Dormant;
        }
        if self.misc_ppd.len() < LEGACY_MISC_PPD_SIZE {
            self.misc_ppd.resize(LEGACY_MISC_PPD_SIZE, 0);
        }
        if read_i32(&self.misc_ppd, MISC_PPD_GIFT_YEAR_OFFSET) != event_year {
            for byte in &mut self.misc_ppd[MISC_PPD_TREEDONE_OFFSET..MISC_PPD_TREEDONE_OFFSET + 8] {
                *byte = 0;
            }
            write_i32(&mut self.misc_ppd, MISC_PPD_GIFT_YEAR_OFFSET, event_year);
        }

        let idx = usize::from(area_id / 8);
        let bit = 1u8 << (area_id % 8);
        if idx >= 8 || self.misc_ppd[MISC_PPD_TREEDONE_OFFSET + idx] & bit != 0 {
            return XmasTreeResult::AlreadyGranted;
        }
        if !has_holiday_treat {
            return XmasTreeResult::NeedsHolidayTreat;
        }

        self.misc_ppd[MISC_PPD_TREEDONE_OFFSET + idx] |= bit;
        XmasTreeResult::GiftGranted
    }

    pub fn unmark_xmas_tree(&mut self, area_id: u16) {
        if self.misc_ppd.len() < LEGACY_MISC_PPD_SIZE {
            return;
        }
        let idx = usize::from(area_id / 8);
        if idx < 8 {
            self.misc_ppd[MISC_PPD_TREEDONE_OFFSET + idx] &= !(1u8 << (area_id % 8));
        }
    }

    pub fn xmas_tree_marked(&self, area_id: u16) -> bool {
        if self.misc_ppd.len() < LEGACY_MISC_PPD_SIZE {
            return false;
        }
        let idx = usize::from(area_id / 8);
        idx < 8 && self.misc_ppd[MISC_PPD_TREEDONE_OFFSET + idx] & (1u8 << (area_id % 8)) != 0
    }

    pub fn orb_spawn_last_used_seconds(&self, location_id: u32) -> Option<u64> {
        self.orb_spawns
            .iter()
            .find(|entry| entry.location_id == location_id)
            .map(|entry| entry.last_used_seconds)
    }

    pub fn mark_orb_spawn_used(&mut self, location_id: u32, realtime_seconds: u64) {
        if let Some(entry) = self
            .orb_spawns
            .iter_mut()
            .find(|entry| entry.location_id == location_id)
        {
            entry.last_used_seconds = realtime_seconds;
            return;
        }
        if self.orb_spawns.len() < ORBSPAWN_MAX_ENTRIES {
            self.orb_spawns.push(OrbSpawnAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            });
            return;
        }
        if let Some(oldest) = self
            .orb_spawns
            .iter_mut()
            .min_by_key(|entry| entry.last_used_seconds)
        {
            *oldest = OrbSpawnAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            };
        }
    }
}
