// The PPD byte-offset constants and codecs in this module mirror the C
// `struct *_ppd` layouts verbatim as `<field index> * 4` products (so
// `0 * 4`, `1 * 4`, ... line up visually with the C struct order); keep
// clippy from "simplifying" the intentional identity/zero terms.
#![allow(clippy::identity_op, clippy::erasing_op)]

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

/// C `struct tunnel_ppd { int clevel; unsigned char used[204]; }`
/// (`src/area/33/tunnel.h:6-9`): one leading `int` (4 bytes) followed by
/// the 204-byte `used[]` completion-count array (`MAX_TUNNEL_LEVEL` = 200,
/// so indices `0..=203` cover every valid level with room to spare, no
/// struct padding since 4 + 204 = 208 is already a multiple of 4).
pub const LEGACY_TUNNEL_PPD_SIZE: usize = 4 + 204;

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

/// C `struct arkhata_ppd` (`src/area/37/arkhata.h:4-26`): 21 `int` fields.
/// Was previously `25 * 4` (a copy-paste-from-`LEGACY_NOMAD_PPD_SIZE`
/// size bug - `struct arkhata_ppd` has no relation to `struct
/// nomad_ppd`'s field count at all); fixed while porting `CDR_NOP`/
/// `CDR_ARKHATAPRISON` (`world::npc::area37`). All three existing
/// accessors' offsets (`ARKHATA_PPD_CLERK_STATE_OFFSET` at field 16,
/// `ARKHATA_PPD_MONK_STATE_OFFSET` at field 4) stay within the corrected
/// 21-field bound, so this only shrinks the size by the 4 unused trailing
/// `int`s (16 bytes) that were never actually part of the C struct and
/// were making legacy-blob decode reject genuine pre-migration
/// `DRD_ARKHATA_PPD` rows (only 84 bytes on disk) as too short.
pub const LEGACY_ARKHATA_PPD_SIZE: usize = 21 * 4;

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

pub const LEGACY_LAB3_PASSWORD_FIELD_LEN: usize = 8;

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

pub const LEGACY_LOSTCON_PPD_SIZE: usize = 19 * 4;

pub const RUNE_USED_WORDS: usize = 1024 / 32;

pub const RUNE_SPECIAL_EXEC_COUNT: usize = 25;

pub const LEGACY_RUNE_PPD_SIZE: usize = RUNE_USED_WORDS * 4 + RUNE_SPECIAL_EXEC_COUNT * 4;

/// C `MAXRUNE` (`src/area/18/bones.c:80`): the exclusive upper bound for a
/// rune combination number, matching `rune_used_words`'s 32-word (1024-bit)
/// bitfield.
pub const MAXRUNE: i32 = 1024;

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

/// C `struct gate_ppd { int welcome_state; int target_class; int step; }`
/// (`src/system/gatekeeper.c:221-225`) - three packed `int`s, matching the
/// legacy PPD blob layout exactly.
pub const LEGACY_GATE_PPD_SIZE: usize = 12;

pub const fn make_drd(dev_id: u32, nr: u32) -> u32 {
    (dev_id << 24) | nr
}

pub(crate) const CALIGAR_PPD_DOOR_FLAG_COUNT: usize = 4;
