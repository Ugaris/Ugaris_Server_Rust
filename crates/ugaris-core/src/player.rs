use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{
    entity::{Character, CharacterFlags, CharacterValue, Item},
    ids::CharacterId,
    legacy::DIST_OLD,
    quest::QuestLog,
    tell::TellData,
    tick::TICKS_PER_SECOND,
};

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
pub const LEGACY_AREA3_PPD_SIZE: usize = 17 * 4;
/// C `struct area1_ppd` (`src/area/1/area1.h:24-75`): 39 `int` fields.
pub const LEGACY_AREA1_PPD_SIZE: usize = 39 * 4;
/// C `struct nomad_ppd` (`src/common/nomad_ppd.h:9-13`):
/// `nomad_state[MAXNOMAD]` + `nomad_win[MAXNOMAD]` (`MAXNOMAD` = 10) +
/// `open_roll1/2/3/open_bet` + `tribe_member` = 10+10+4+1 = 25 `int`s.
pub const LEGACY_NOMAD_PPD_SIZE: usize = 25 * 4;
pub const NOMAD_PPD_MAXNOMAD: usize = 10;
pub const LEGACY_CALIGAR_PPD_SIZE: usize = 14 * 4 + 4;
pub const LEGACY_ARKHATA_PPD_SIZE: usize = 25 * 4;
pub const LEGACY_STAFFER_PPD_SIZE: usize = 25 * 4;
pub const LEGACY_FARMY_PPD_SIZE: usize = 85 * 4;
pub const LEGACY_TEUFELRAT_PPD_SIZE: usize = 2 * 4;
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
/// The following 9 ids (`src/system/drdata.h`) back systems that are not
/// modeled on `PlayerRuntime` at all yet (first-kill tracking, army rank,
/// military points, arena, sidestory, tunnel, strategy game, quest log,
/// and the per-character legacy depot). They exist here solely so
/// `turn_seyan` (`src/system/tool.c:4278-4389`, ported at
/// `World::apply_turn_seyan`) can `del_data` them exactly like C does, via
/// `PlayerRuntime::clear_turn_seyan_ppd`'s raw-block strip - see
/// `strip_ppd_blocks`. No decode/encode logic backs these ids since
/// nothing else in this codebase reads or writes them yet. `DRD_AREA1_PPD`
/// and `DRD_NOMAD_PPD` moved out of this group (see `area1_ppd`/
/// `nomad_ppd` below) once their questlog-init-required fields got real
/// accessors.
pub const DRD_FIRSTKILL_PPD: u32 = make_drd(DEV_ID_DB, 18 | PERSISTENT_PLAYER_DATA);
pub const DRD_RANK_PPD: u32 = make_drd(DEV_ID_DB, 41 | PERSISTENT_PLAYER_DATA);
pub const DRD_DEPOT_PPD: u32 = make_drd(DEV_ID_DB, 67 | PERSISTENT_PLAYER_DATA);
pub const DRD_MILITARY_PPD: u32 = make_drd(DEV_ID_DB, 72 | PERSISTENT_PLAYER_DATA);
pub const DRD_ARENA_PPD: u32 = make_drd(DEV_ID_DB, 83 | PERSISTENT_PLAYER_DATA);
pub const DRD_STRATEGY_PPD: u32 = make_drd(DEV_ID_DB, 121 | PERSISTENT_PLAYER_DATA);
pub const DRD_SIDESTORY_PPD: u32 = make_drd(DEV_ID_DB, 124 | PERSISTENT_PLAYER_DATA);
pub const DRD_TUNNEL_PPD: u32 = make_drd(DEV_ID_DB, 154 | PERSISTENT_PLAYER_DATA);
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

const WARP_PPD_BASE_OFFSET: usize = 0;
const WARP_PPD_POINTS_OFFSET: usize = WARP_PPD_BASE_OFFSET + 4;
const WARP_PPD_BONUS_ID_OFFSET: usize = WARP_PPD_POINTS_OFFSET + 4;
const WARP_PPD_BONUS_LAST_USED_OFFSET: usize = WARP_PPD_BONUS_ID_OFFSET + WARP_BONUS_COUNT * 4;
const WARP_PPD_NOSTEPEXP_OFFSET: usize = WARP_PPD_BONUS_LAST_USED_OFFSET + WARP_BONUS_COUNT * 4;

/// C `struct gate_ppd { int welcome_state; int target_class; int step; }`
/// (`src/system/gatekeeper.c:221-225`) - three packed `int`s, matching the
/// legacy PPD blob layout exactly.
pub const LEGACY_GATE_PPD_SIZE: usize = 12;
const GATE_PPD_WELCOME_STATE_OFFSET: usize = 0;
const GATE_PPD_TARGET_CLASS_OFFSET: usize = 4;
const GATE_PPD_STEP_OFFSET: usize = 8;

pub const fn make_drd(dev_id: u32, nr: u32) -> u32 {
    (dev_id << 24) | nr
}

const KEYRING_PPD_COUNT_OFFSET: usize = 0;
const KEYRING_PPD_KEYS_OFFSET: usize = 4;
const KEYRING_PPD_NAMES_OFFSET: usize = KEYRING_PPD_KEYS_OFFSET + KEYRING_MAX_KEYS * 4;
const KEYRING_PPD_DESCS_OFFSET: usize =
    KEYRING_PPD_NAMES_OFFSET + KEYRING_MAX_KEYS * KEYRING_KEY_NAME_LEN;
const KEYRING_PPD_SPRITES_OFFSET: usize =
    KEYRING_PPD_DESCS_OFFSET + KEYRING_MAX_KEYS * KEYRING_KEY_DESC_LEN;
const KEYRING_PPD_FLAGS_OFFSET: usize = KEYRING_PPD_SPRITES_OFFSET + KEYRING_MAX_KEYS * 4 + 4;
const KEYRING_PPD_VALUES_OFFSET: usize = KEYRING_PPD_FLAGS_OFFSET + KEYRING_MAX_KEYS * 8;
const KEYRING_PPD_DRIVERS_OFFSET: usize = KEYRING_PPD_VALUES_OFFSET + KEYRING_MAX_KEYS * 4;
const KEYRING_PPD_DRDATA_OFFSET: usize = KEYRING_PPD_DRIVERS_OFFSET + KEYRING_MAX_KEYS * 2;
const KEYRING_PPD_EXPIRE_OFFSET: usize =
    KEYRING_PPD_DRDATA_OFFSET + KEYRING_MAX_KEYS * KEYRING_KEY_DRDATA_LEN;
const KEYRING_PPD_AUTO_ADD_OFFSET: usize = KEYRING_PPD_EXPIRE_OFFSET + KEYRING_MAX_KEYS;
const RANDCHEST_PPD_IDS_OFFSET: usize = 0;
const RANDCHEST_PPD_LAST_USED_OFFSET: usize = RANDCHEST_PPD_IDS_OFFSET + RANDCHEST_MAX_ENTRIES * 4;
const RATCHEST_PPD_IDS_OFFSET: usize = 0;
const RATCHEST_PPD_LAST_USED_OFFSET: usize = RATCHEST_PPD_IDS_OFFSET + RATCHEST_MAX_ENTRIES * 4;
const RATCHEST_PPD_TREASURE_X_OFFSET: usize =
    RATCHEST_PPD_LAST_USED_OFFSET + RATCHEST_MAX_ENTRIES * 4;
const RATCHEST_PPD_TREASURE_Y_OFFSET: usize = RATCHEST_PPD_TREASURE_X_OFFSET + 4;
const RATCHEST_PPD_LAST_TREASURE_OFFSET: usize = RATCHEST_PPD_TREASURE_Y_OFFSET + 4;
const ORBSPAWN_PPD_IDS_OFFSET: usize = 0;
const ORBSPAWN_PPD_LAST_USED_OFFSET: usize = ORBSPAWN_PPD_IDS_OFFSET + ORBSPAWN_MAX_ENTRIES * 4;
const FLOWER_PPD_IDS_OFFSET: usize = 0;
const FLOWER_PPD_LAST_USED_OFFSET: usize = FLOWER_PPD_IDS_OFFSET + FLOWER_MAX_ENTRIES * 4;
const AREA3_PPD_KELLY_STATE_OFFSET: usize = 1 * 4;
const AREA3_PPD_KELLY_FOUND1_OFFSET: usize = 3 * 4;
const AREA3_PPD_KELLY_FOUND2_OFFSET: usize = 4 * 4;
const AREA3_PPD_KELLY_FOUND3_OFFSET: usize = 5 * 4;
const AREA3_PPD_CLARA_STATE_OFFSET: usize = 9 * 4;
const AREA3_PPD_IMP_FLAGS_OFFSET: usize = 12 * 4;
// `struct area1_ppd` field offsets (`src/area/1/area1.h:24-75`), in
// declaration order (0-based `int` index * 4). Only the fields consumed by
// `questlog_init_area1` (`src/system/questlog.c:828-1039`) have named
// accessors so far; the rest round-trip as opaque bytes.
const AREA1_PPD_YOAKIN_STATE_OFFSET: usize = 0 * 4;
const AREA1_PPD_GWENDY_STATE_OFFSET: usize = 2 * 4;
const AREA1_PPD_NOOK_STATE_OFFSET: usize = 10 * 4;
const AREA1_PPD_LYDIA_STATE_OFFSET: usize = 11 * 4;
const AREA1_PPD_GUIWYNN_STATE_OFFSET: usize = 15 * 4;
const AREA1_PPD_LOGAIN_STATE_OFFSET: usize = 17 * 4;
const AREA1_PPD_RESKIN_STATE_OFFSET: usize = 19 * 4;
const AREA1_PPD_BRITHILDIE_STATE_OFFSET: usize = 24 * 4;
const AREA1_PPD_CAMHERMIT_STATE_OFFSET: usize = 32 * 4;
const AREA1_PPD_JESSICA_STATE_OFFSET: usize = 35 * 4;
// `struct nomad_ppd` field offsets (`src/common/nomad_ppd.h:9-13`):
// `nomad_state[MAXNOMAD]` then `nomad_win[MAXNOMAD]` then the four open-
// roll/bet ints then `tribe_member`.
const NOMAD_PPD_STATE_OFFSET: usize = 0;
const NOMAD_PPD_WIN_OFFSET: usize = NOMAD_PPD_MAXNOMAD * 4;
const CALIGAR_PPD_WATCH_FLAG_OFFSET: usize = 4 * 4;
const CALIGAR_PPD_DOOR_FLAG_OFFSET: usize = 14 * 4;
const CALIGAR_PPD_DOOR_FLAG_COUNT: usize = 4;
pub const ARKHATA_PPD_CLERK_STATE_OFFSET: usize = 16 * 4;
pub const ARKHATA_PPD_CLERK_TIME_OFFSET: usize = 17 * 4;
const STAFFER_PPD_SHANRA_STATE_OFFSET: usize = 16 * 4;
const FARMY_PPD_BOSS_STAGE_OFFSET: usize = 0;
const TEUFELRAT_PPD_KILLS_OFFSET: usize = 0;
const TEUFELRAT_PPD_SCORE_OFFSET: usize = 4;
const BANK_PPD_IMPERIAL_GOLD_OFFSET: usize = 0;
const TWOCITY_PPD_GOODTILE_OFFSET: usize = 19 * 4;
const TWOCITY_PPD_SOLVED_LIBRARY_OFFSET: usize = 24 * 4;
const TWOCITY_PPD_THIEF_STATE_OFFSET: usize = 8 * 4;
const TWOCITY_PPD_THIEF_KILLED_OFFSET: usize = 10 * 4;
const MISC_PPD_TREEDONE_OFFSET: usize = 24;
const MISC_PPD_GIFT_YEAR_OFFSET: usize = 32;
const LOSTCON_PPD_AUTOTURN_OFFSET: usize = 16 * 4;
const LOSTCON_PPD_MAXLAG_OFFSET: usize = 17 * 4;
const LOSTCON_PPD_HINTS_OFFSET: usize = 18 * 4;
const STAFFER_PPD_FORESTBRAN_DONE_OFFSET: usize = 11 * 4;
const PK_PPD_KILLS_OFFSET: usize = 0;
const PK_PPD_DEATHS_OFFSET: usize = 4;
const PK_PPD_LAST_KILL_OFFSET: usize = 8;
const PK_PPD_LAST_DEATH_OFFSET: usize = 12;
const PK_PPD_HATE_OFFSET: usize = 16;
const RUNE_PPD_SPECIAL_EXEC_OFFSET: usize = RUNE_USED_WORDS * 4;
const SWEAR_PPD_BANNED_TILL_OFFSET: usize = LEGACY_SWEAR_PPD_SIZE - 4;

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
pub struct QueuedAction {
    pub action: PlayerActionCode,
    pub arg1: i32,
    pub arg2: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyringEntry {
    pub template_id: u32,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub sprite: i32,
    #[serde(default)]
    pub flags: u64,
    #[serde(default)]
    pub value: u32,
    #[serde(default)]
    pub driver: u16,
    #[serde(default)]
    pub driver_data: Vec<u8>,
    #[serde(default)]
    pub expire_serial: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomChestAccess {
    pub location_id: u32,
    pub last_used_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RatChestAccess {
    pub location_id: u32,
    pub last_used_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrbSpawnAccess {
    pub location_id: u32,
    pub last_used_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowerAccess {
    pub location_id: u32,
    pub last_used_seconds: u64,
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AchievementState {
    pub chests_opened: u32,
    pub looter: bool,
    pub treasure_hunter: bool,
    pub treasure_master: bool,
    pub legendary_looter: bool,
    pub gold_looter: bool,
    #[serde(default)]
    pub traveller_of_astonia: bool,
    #[serde(default)]
    pub explorer_of_astonia: bool,
    #[serde(default)]
    pub underground_explorer: bool,
}

impl Default for QueuedAction {
    fn default() -> Self {
        Self {
            action: PlayerActionCode::Idle,
            arg1: 0,
            arg2: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRuntime {
    pub session_id: u64,
    pub state: PlayerConnectionState,
    pub client_version: u8,
    pub view_distance: usize,
    pub last_command_tick: u64,
    pub character_id: Option<CharacterId>,
    pub character_number: u32,
    pub command: Vec<u8>,
    pub action: QueuedAction,
    pub queue: VecDeque<QueuedAction>,
    pub client_ticker: u32,
    pub next_fightback_character: Option<CharacterId>,
    pub next_fightback_serial: u32,
    pub next_fightback_tick: u64,
    pub nofight_timer: u64,
    pub login_tick: u64,
    pub deferred_init: u32,
    pub scrollback: Vec<u8>,
    #[serde(default)]
    pub ppd_blob: Vec<u8>,
    #[serde(default)]
    pub subscriber_blob: Vec<u8>,
    pub chest_last_access_seconds: HashMap<u8, u64>,
    pub keyring: Vec<KeyringEntry>,
    pub random_chests: Vec<RandomChestAccess>,
    #[serde(default)]
    pub rat_chests: Vec<RatChestAccess>,
    #[serde(default)]
    pub rat_chest_treasure_x: u16,
    #[serde(default)]
    pub rat_chest_treasure_y: u16,
    #[serde(default)]
    pub rat_chest_last_treasure_seconds: u64,
    #[serde(default)]
    pub orb_spawns: Vec<OrbSpawnAccess>,
    #[serde(default)]
    pub flowers: Vec<FlowerAccess>,
    #[serde(default)]
    pub demonshrines: Vec<u32>,
    #[serde(default)]
    pub random_shrine_used_words: [u32; RANDOMSHRINE_USED_WORDS],
    #[serde(default)]
    pub random_shrine_continuity: u8,
    #[serde(default)]
    pub treasure_dig_last_seconds: [u64; TREASURE_DIG_PPD_ENTRIES],
    #[serde(default)]
    pub misc_ppd: Vec<u8>,
    #[serde(default)]
    pub area3_ppd: Vec<u8>,
    #[serde(default)]
    pub area1_ppd: Vec<u8>,
    #[serde(default)]
    pub nomad_ppd: Vec<u8>,
    #[serde(default)]
    pub caligar_ppd: Vec<u8>,
    #[serde(default)]
    pub arkhata_ppd: Vec<u8>,
    #[serde(default)]
    pub staffer_ppd: Vec<u8>,
    #[serde(default)]
    pub farmy_ppd: Vec<u8>,
    #[serde(default)]
    pub teufel_rat_kills: u32,
    #[serde(default)]
    pub teufel_rat_score: u32,
    /// C `struct bank_ppd { int imperial_gold; }` (`src/module/bank.h`):
    /// the player's Imperial Bank account balance, in the same silver-piece
    /// unit as `Character.gold` (`ch[cn].gold`).
    #[serde(default)]
    pub bank_gold: u32,
    #[serde(default)]
    pub twocity_ppd: Vec<u8>,
    #[serde(default)]
    pub lab_ppd: Vec<u8>,
    #[serde(default)]
    pub warp_ppd: Vec<u8>,
    #[serde(default)]
    pub warp_base: i32,
    #[serde(default)]
    pub warp_points: i32,
    #[serde(default)]
    pub warp_bonus_ids: Vec<i32>,
    #[serde(default)]
    pub warp_bonus_last_used: Vec<i32>,
    #[serde(default)]
    pub warp_nostepexp: i32,
    #[serde(default)]
    pub gate_ppd: Vec<u8>,
    /// C `gate_ppd.welcome_state` (`src/system/gatekeeper.c:222`): the
    /// `gate_welcome_driver` dialogue step, `0..=6`.
    #[serde(default)]
    pub gate_welcome_state: i32,
    /// C `gate_ppd.target_class` (`src/system/gatekeeper.c:223`): the
    /// class chosen for the test (`5` Arch-Warrior, `6` Arch-Mage, `7`
    /// Arch-Seyan'Du, `8` Seyan'Du).
    #[serde(default)]
    pub gate_target_class: i32,
    /// C `gate_ppd.step` (`src/system/gatekeeper.c:224`): unused by the
    /// ported logic so far (C never reads it either - set once on
    /// `enter_room` success and never consulted), kept for round-trip
    /// fidelity.
    #[serde(default)]
    pub gate_step: i32,
    #[serde(default)]
    pub lab_solved_bits: u64,
    #[serde(default)]
    pub lab2_grave_bits: Vec<u8>,
    #[serde(default)]
    pub pk_kills: u32,
    #[serde(default)]
    pub pk_deaths: u32,
    #[serde(default)]
    pub pk_last_kill: u32,
    #[serde(default)]
    pub pk_last_death: u32,
    #[serde(default)]
    pub pk_hate: Vec<u32>,
    pub achievements: AchievementState,
    #[serde(default)]
    pub keyring_auto_add: bool,
    #[serde(default)]
    pub current_section_id: u16,
    #[serde(default)]
    pub special_shrine_hcsc_last_touch_seconds: u64,
    #[serde(default)]
    pub transport_seen: u64,
    #[serde(default)]
    pub current_mirror_id: u16,
    #[serde(default)]
    pub max_lag_seconds: u8,
    #[serde(default)]
    pub hints_disabled: bool,
    #[serde(default)]
    pub autoturn_enabled: bool,
    #[serde(default)]
    pub shutup_until_seconds: u64,
    #[serde(default)]
    pub swear_ppd: Vec<u8>,
    #[serde(default)]
    pub tell_data: TellData,
    #[serde(default)]
    pub ignored_characters: Vec<u32>,
    #[serde(default)]
    pub chat_channels: u32,
    #[serde(default)]
    pub rune_used_words: [u32; RUNE_USED_WORDS],
    #[serde(default)]
    pub rune_special_exec: [i32; RUNE_SPECIAL_EXEC_COUNT],
    #[serde(default)]
    pub aliases: Vec<CommandAlias>,
    #[serde(default)]
    pub quest_log: QuestLog,
    #[serde(default)]
    pub twocity_goodtile: [u8; 5],
    #[serde(default)]
    pub twocity_solved_library: bool,
    #[serde(default)]
    pub saltmine_ladder_last_seconds: [u64; SALTMINE_LADDER_COUNT],
    #[serde(default)]
    pub saltmine_pending_salt: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandAlias {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgnoreToggleResult {
    Added,
    Removed,
    Full,
}

impl PlayerRuntime {
    pub fn connected(session_id: u64, current_tick: u64) -> Self {
        Self {
            session_id,
            state: PlayerConnectionState::Connect,
            client_version: 0,
            view_distance: DIST_OLD,
            last_command_tick: current_tick,
            character_id: None,
            character_number: 0,
            command: Vec::new(),
            action: QueuedAction::default(),
            queue: VecDeque::with_capacity(COMMAND_QUEUE_SIZE),
            client_ticker: 0,
            next_fightback_character: None,
            next_fightback_serial: 0,
            next_fightback_tick: 0,
            nofight_timer: 0,
            login_tick: current_tick,
            deferred_init: 0,
            scrollback: Vec::with_capacity(MAX_SCROLLBACK),
            ppd_blob: Vec::new(),
            subscriber_blob: Vec::new(),
            chest_last_access_seconds: HashMap::new(),
            keyring: Vec::new(),
            random_chests: Vec::new(),
            rat_chests: Vec::new(),
            rat_chest_treasure_x: 0,
            rat_chest_treasure_y: 0,
            rat_chest_last_treasure_seconds: 0,
            orb_spawns: Vec::new(),
            flowers: Vec::new(),
            demonshrines: Vec::new(),
            random_shrine_used_words: [0; RANDOMSHRINE_USED_WORDS],
            random_shrine_continuity: 0,
            treasure_dig_last_seconds: [0; TREASURE_DIG_PPD_ENTRIES],
            misc_ppd: Vec::new(),
            area3_ppd: Vec::new(),
            area1_ppd: Vec::new(),
            nomad_ppd: Vec::new(),
            caligar_ppd: Vec::new(),
            arkhata_ppd: Vec::new(),
            staffer_ppd: Vec::new(),
            farmy_ppd: Vec::new(),
            teufel_rat_kills: 0,
            teufel_rat_score: 0,
            bank_gold: 0,
            twocity_ppd: Vec::new(),
            lab_ppd: Vec::new(),
            warp_ppd: Vec::new(),
            warp_base: 0,
            warp_points: 0,
            warp_bonus_ids: vec![0; WARP_BONUS_COUNT],
            warp_bonus_last_used: vec![0; WARP_BONUS_COUNT],
            warp_nostepexp: 0,
            gate_ppd: Vec::new(),
            gate_welcome_state: 0,
            gate_target_class: 0,
            gate_step: 0,
            lab_solved_bits: 0,
            lab2_grave_bits: Vec::new(),
            pk_kills: 0,
            pk_deaths: 0,
            pk_last_kill: 0,
            pk_last_death: 0,
            pk_hate: Vec::new(),
            achievements: AchievementState::default(),
            keyring_auto_add: false,
            current_section_id: 0,
            special_shrine_hcsc_last_touch_seconds: 0,
            transport_seen: 0,
            current_mirror_id: 0,
            max_lag_seconds: 0,
            hints_disabled: false,
            autoturn_enabled: false,
            shutup_until_seconds: 0,
            swear_ppd: Vec::new(),
            tell_data: TellData::default(),
            ignored_characters: Vec::new(),
            chat_channels: 0,
            rune_used_words: [0; RUNE_USED_WORDS],
            rune_special_exec: [0; RUNE_SPECIAL_EXEC_COUNT],
            aliases: Vec::new(),
            quest_log: QuestLog::default(),
            twocity_goodtile: [0; 5],
            twocity_solved_library: false,
            saltmine_ladder_last_seconds: [0; SALTMINE_LADDER_COUNT],
            saltmine_pending_salt: 0,
        }
    }

    pub fn saltmine_ladder_ready(&self, ladder_index: u8, realtime_seconds: u64) -> bool {
        let Some(last_used) = self
            .saltmine_ladder_last_seconds
            .get(usize::from(ladder_index))
        else {
            return false;
        };
        *last_used == 0 || last_used.saturating_add(60 * 60 * 24) <= realtime_seconds
    }

    pub fn mark_saltmine_ladder_used(&mut self, ladder_index: u8, realtime_seconds: u64) -> bool {
        let Some(last_used) = self
            .saltmine_ladder_last_seconds
            .get_mut(usize::from(ladder_index))
        else {
            return false;
        };
        *last_used = realtime_seconds;
        true
    }

    pub fn encode_legacy_saltmine_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_SALTMINE_PPD_SIZE];
        bytes[0] = LEGACY_SALTMINE_PPD_VERSION;
        for (idx, seconds) in self.saltmine_ladder_last_seconds.iter().enumerate() {
            let value = (*seconds).min(i32::MAX as u64) as i32;
            write_i32(&mut bytes, 4 + idx * 4, value);
        }
        write_i32(
            &mut bytes,
            4 + SALTMINE_LADDER_COUNT * 4,
            self.saltmine_pending_salt.min(i32::MAX as u32) as i32,
        );
        bytes
    }

    pub fn decode_legacy_saltmine_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_SALTMINE_PPD_SIZE {
            return false;
        }
        if bytes[0] != LEGACY_SALTMINE_PPD_VERSION {
            self.saltmine_ladder_last_seconds = [0; SALTMINE_LADDER_COUNT];
            self.saltmine_pending_salt = 0;
            return true;
        }
        for idx in 0..SALTMINE_LADDER_COUNT {
            self.saltmine_ladder_last_seconds[idx] = read_i32(bytes, 4 + idx * 4).max(0) as u64;
        }
        self.saltmine_pending_salt = read_i32(bytes, 4 + SALTMINE_LADDER_COUNT * 4).max(0) as u32;
        true
    }

    pub fn ensure_twocity_goodtile_with<F>(&mut self, mut roll_color: F) -> [u8; 5]
    where
        F: FnMut() -> u8,
    {
        if self.twocity_goodtile[0] == 0 {
            for color in &mut self.twocity_goodtile {
                *color = roll_color().clamp(1, 6);
            }
        }
        self.twocity_goodtile
    }

    pub fn set_twocity_thief_state(&mut self, state: i32) {
        self.twocity_ppd.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        write_i32(&mut self.twocity_ppd, TWOCITY_PPD_THIEF_STATE_OFFSET, state);
    }

    pub fn twocity_thief_state(&self) -> i32 {
        if self.twocity_ppd.len() < LEGACY_TWOCITY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.twocity_ppd, TWOCITY_PPD_THIEF_STATE_OFFSET)
    }

    pub fn twocity_thief_killed(&self, index: usize) -> i32 {
        if index >= 6 || self.twocity_ppd.len() < LEGACY_TWOCITY_PPD_SIZE {
            return 0;
        }
        read_i32(
            &self.twocity_ppd,
            TWOCITY_PPD_THIEF_KILLED_OFFSET + index * 4,
        )
    }

    pub fn mark_twocity_burndown_kill(&mut self) -> bool {
        self.twocity_ppd.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        let thief_state = read_i32(&self.twocity_ppd, TWOCITY_PPD_THIEF_STATE_OFFSET);
        if thief_state != 13 && thief_state != 14 {
            return false;
        }
        write_i32(&mut self.twocity_ppd, TWOCITY_PPD_THIEF_STATE_OFFSET, 14);
        let killed = read_i32(&self.twocity_ppd, TWOCITY_PPD_THIEF_KILLED_OFFSET);
        write_i32(
            &mut self.twocity_ppd,
            TWOCITY_PPD_THIEF_KILLED_OFFSET,
            killed.saturating_add(1),
        );
        true
    }

    pub fn encode_legacy_twocity_ppd(&self) -> Vec<u8> {
        let mut bytes = self.twocity_ppd.clone();
        bytes.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        for (index, color) in self.twocity_goodtile.iter().copied().enumerate() {
            write_i32(
                &mut bytes,
                TWOCITY_PPD_GOODTILE_OFFSET + index * 4,
                color as i32,
            );
        }
        write_i32(
            &mut bytes,
            TWOCITY_PPD_SOLVED_LIBRARY_OFFSET,
            i32::from(self.twocity_solved_library),
        );
        bytes
    }

    pub fn decode_legacy_twocity_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_TWOCITY_PPD_SIZE {
            return false;
        }
        self.twocity_ppd = bytes[..LEGACY_TWOCITY_PPD_SIZE].to_vec();
        for index in 0..self.twocity_goodtile.len() {
            let color = read_i32(bytes, TWOCITY_PPD_GOODTILE_OFFSET + index * 4);
            self.twocity_goodtile[index] = if (0..=u8::MAX as i32).contains(&color) {
                color as u8
            } else {
                0
            };
        }
        self.twocity_solved_library = read_i32(bytes, TWOCITY_PPD_SOLVED_LIBRARY_OFFSET) != 0;
        true
    }

    pub fn encode_legacy_alias_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_ALIAS_PPD_SIZE];
        for (index, alias) in self.aliases.iter().take(ALIAS_MAX_ENTRIES).enumerate() {
            let offset = index * (ALIAS_FROM_LEN + ALIAS_TO_LEN);
            write_c_string(&mut bytes, offset, ALIAS_FROM_LEN, &alias.from);
            write_c_string(&mut bytes, offset + ALIAS_FROM_LEN, ALIAS_TO_LEN, &alias.to);
        }
        bytes
    }

    pub fn decode_legacy_alias_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_ALIAS_PPD_SIZE {
            return false;
        }
        self.aliases.clear();
        for index in 0..ALIAS_MAX_ENTRIES {
            let offset = index * (ALIAS_FROM_LEN + ALIAS_TO_LEN);
            let from = read_c_string(bytes, offset, ALIAS_FROM_LEN);
            if from.is_empty() {
                continue;
            }
            let to = read_c_string(bytes, offset + ALIAS_FROM_LEN, ALIAS_TO_LEN);
            self.aliases.push(CommandAlias { from, to });
        }
        true
    }

    pub fn expand_aliases(&self, source: &str) -> String {
        fn alias_stop(ch: char) -> bool {
            ch.is_whitespace() || (ch.is_ascii_punctuation() && ch != '\'')
        }

        let mut out = String::new();
        let mut token = String::new();
        for ch in source.chars() {
            if alias_stop(ch) {
                if token.is_empty() {
                    out.push(ch);
                    continue;
                }
                if let Some(alias) = self
                    .aliases
                    .iter()
                    .find(|alias| alias.from.eq_ignore_ascii_case(&token))
                {
                    out.push_str(&alias.to);
                } else {
                    out.push_str(&token);
                }
                token.clear();
                out.push(ch);
            } else {
                token.push(ch);
            }
            if out.len() > 198 {
                out.truncate(199);
                return out;
            }
        }
        if !token.is_empty() {
            if let Some(alias) = self
                .aliases
                .iter()
                .find(|alias| alias.from.eq_ignore_ascii_case(&token))
            {
                out.push_str(&alias.to);
            } else {
                out.push_str(&token);
            }
        }
        if out.len() > 199 {
            out.truncate(199);
        }
        out
    }

    pub fn ignores_character(&self, character_id: u32) -> bool {
        character_id != 0 && self.ignored_characters.contains(&character_id)
    }

    pub fn toggle_ignored_character(&mut self, character_id: u32) -> IgnoreToggleResult {
        if character_id == 0 {
            return IgnoreToggleResult::Full;
        }
        if let Some(index) = self
            .ignored_characters
            .iter()
            .position(|ignored| *ignored == character_id)
        {
            self.ignored_characters.remove(index);
            return IgnoreToggleResult::Removed;
        }
        if self.ignored_characters.len() >= IGNORE_MAX_ENTRIES {
            return IgnoreToggleResult::Full;
        }
        self.ignored_characters.push(character_id);
        IgnoreToggleResult::Added
    }

    pub fn clear_ignored_characters(&mut self) {
        self.ignored_characters.clear();
    }

    pub fn encode_legacy_ignore_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_IGNORE_PPD_SIZE];
        for (index, character_id) in self
            .ignored_characters
            .iter()
            .copied()
            .take(IGNORE_MAX_ENTRIES)
            .enumerate()
        {
            write_i32(
                &mut bytes,
                index * 4,
                character_id.min(i32::MAX as u32) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_ignore_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_IGNORE_PPD_SIZE {
            return false;
        }
        self.ignored_characters.clear();
        for index in 0..IGNORE_MAX_ENTRIES {
            let character_id = read_i32(bytes, index * 4);
            if character_id > 0 {
                self.ignored_characters.push(character_id as u32);
            }
        }
        true
    }

    pub fn ensure_rune_special_execs<F>(&mut self, mut random_below: F)
    where
        F: FnMut(u32) -> u32,
    {
        if self.rune_special_exec[0] != 0 {
            return;
        }

        const BADLIST: [i32; 15] = [555, 55, 5, 666, 66, 6, 777, 77, 7, 888, 88, 8, 999, 99, 9];
        for level in 5..10 {
            for offset in 0..5 {
                loop {
                    let value = random_below(level * 111) as i32;
                    if value < 100 || BADLIST.contains(&value) {
                        continue;
                    }
                    let base = (level - 5) as usize * 5;
                    if self.rune_special_exec[base..base + offset as usize].contains(&value) {
                        continue;
                    }
                    let digits = format!("{value:03}");
                    let level_digit = char::from_digit(level, 10).unwrap();
                    if digits.chars().any(|ch| ch == '0' || ch > level_digit) {
                        continue;
                    }
                    if !digits.chars().any(|ch| ch == level_digit) {
                        continue;
                    }
                    self.rune_special_exec[base + offset as usize] = value;
                    break;
                }
            }
        }
    }

    pub fn bone_hint<F>(&mut self, level: u8, nr: u8, pos: u8, random_below: F) -> BoneHintResult
    where
        F: FnMut(u32) -> u32,
    {
        self.ensure_rune_special_execs(random_below);
        let index = usize::from(level.saturating_sub(5)) * 5 + usize::from(nr);
        let value = self
            .rune_special_exec
            .get(index)
            .copied()
            .unwrap_or_default();
        let digits = value.to_string();
        let digit = digits
            .as_bytes()
            .get(usize::from(pos))
            .copied()
            .unwrap_or(b'0');
        let result = digit.saturating_sub(b'0');
        const RUNE_NAMES: [&str; 10] = [
            "none", "Ansuz", "Berkano", "Dagaz", "Ehwaz", "Fehu", "Hagalaz", "Isa", "Ingwaz",
            "Raidho",
        ];
        const POS_NAMES: [&str; 3] = ["first", "second", "third"];
        let Some(rune) = RUNE_NAMES.get(usize::from(result)).copied() else {
            return BoneHintResult::Bug {
                level,
                nr,
                pos,
                value,
            };
        };
        let Some(position) = POS_NAMES.get(usize::from(pos)).copied() else {
            return BoneHintResult::Bug {
                level,
                nr,
                pos,
                value,
            };
        };
        BoneHintResult::Hint {
            page: u16::from(level) * 10 + u16::from(nr),
            rune,
            position,
        }
    }

    pub fn encode_legacy_rune_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_RUNE_PPD_SIZE];
        for (index, word) in self.rune_used_words.iter().copied().enumerate() {
            write_u32(&mut bytes, index * 4, word);
        }
        for (index, value) in self.rune_special_exec.iter().copied().enumerate() {
            write_i32(&mut bytes, RUNE_PPD_SPECIAL_EXEC_OFFSET + index * 4, value);
        }
        bytes
    }

    pub fn decode_legacy_rune_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_RUNE_PPD_SIZE {
            return false;
        }
        for index in 0..RUNE_USED_WORDS {
            self.rune_used_words[index] = read_u32(bytes, index * 4);
        }
        for index in 0..RUNE_SPECIAL_EXEC_COUNT {
            self.rune_special_exec[index] =
                read_i32(bytes, RUNE_PPD_SPECIAL_EXEC_OFFSET + index * 4);
        }
        true
    }

    pub fn set_max_lag_seconds(&mut self, seconds: u8) {
        self.max_lag_seconds = seconds;
    }

    pub fn toggle_hints(&mut self) -> bool {
        self.hints_disabled = !self.hints_disabled;
        self.hints_disabled
    }

    pub fn toggle_autoturn(&mut self) -> bool {
        self.autoturn_enabled = !self.autoturn_enabled;
        self.autoturn_enabled
    }

    pub fn encode_legacy_lostcon_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_LOSTCON_PPD_SIZE];
        write_i32(
            &mut bytes,
            LOSTCON_PPD_AUTOTURN_OFFSET,
            i32::from(self.autoturn_enabled),
        );
        write_i32(
            &mut bytes,
            LOSTCON_PPD_MAXLAG_OFFSET,
            i32::from(self.max_lag_seconds),
        );
        write_i32(
            &mut bytes,
            LOSTCON_PPD_HINTS_OFFSET,
            i32::from(self.hints_disabled),
        );
        bytes
    }

    pub fn decode_legacy_lostcon_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_LOSTCON_PPD_SIZE {
            return false;
        }
        self.max_lag_seconds =
            read_i32(bytes, LOSTCON_PPD_MAXLAG_OFFSET).clamp(0, i32::from(u8::MAX)) as u8;
        self.hints_disabled = read_i32(bytes, LOSTCON_PPD_HINTS_OFFSET) != 0;
        self.autoturn_enabled = read_i32(bytes, LOSTCON_PPD_AUTOTURN_OFFSET) != 0;
        true
    }

    pub fn set_current_mirror(&mut self, mirror_id: u32) {
        self.current_mirror_id = mirror_id.min(u32::from(u16::MAX)) as u16;
    }

    pub fn touch_transport(&mut self, point: u8) -> bool {
        if point >= 64 {
            return false;
        }
        let bit = 1_u64 << point;
        let newly_seen = self.transport_seen & bit == 0;
        self.transport_seen |= bit;
        if newly_seen {
            self.update_transport_achievement_markers();
        }
        newly_seen
    }

    fn update_transport_achievement_markers(&mut self) {
        if (self.transport_seen & TRANSPORT_MAJOR_CITIES_MASK) == TRANSPORT_MAJOR_CITIES_MASK {
            self.achievements.traveller_of_astonia = true;
        }
        if (self.transport_seen & TRANSPORT_ALL_TELEPORTS_MASK) == TRANSPORT_ALL_TELEPORTS_MASK {
            self.achievements.explorer_of_astonia = true;
        }
        if (self.transport_seen & TRANSPORT_EARTH_UNDERGROUND_MASK)
            == TRANSPORT_EARTH_UNDERGROUND_MASK
        {
            self.achievements.underground_explorer = true;
        }
    }

    pub fn encode_legacy_transport_ppd(&self) -> Vec<u8> {
        self.transport_seen.to_le_bytes().to_vec()
    }

    pub fn decode_legacy_transport_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_TRANSPORT_PPD_SIZE {
            return false;
        }
        self.transport_seen = read_u64(bytes, 0);
        true
    }

    pub fn encode_legacy_lab_ppd(&self) -> Vec<u8> {
        let mut bytes = if self.lab_ppd.len() >= LEGACY_LAB_PPD_SIZE {
            self.lab_ppd.clone()
        } else {
            let mut bytes = vec![0; LEGACY_LAB_PPD_SIZE];
            let copy_len = self.lab_ppd.len().min(LEGACY_LAB_PPD_SIZE);
            bytes[..copy_len].copy_from_slice(&self.lab_ppd[..copy_len]);
            bytes
        };
        write_u64(&mut bytes, 0, self.lab_solved_bits);
        bytes
    }

    pub fn decode_legacy_lab_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < 8 {
            return false;
        }
        self.lab_ppd = bytes.to_vec();
        self.lab_solved_bits = read_u64(bytes, 0);
        true
    }

    pub fn encode_legacy_warp_ppd(&self) -> Vec<u8> {
        let mut bytes = self.warp_ppd.clone();
        bytes.resize(LEGACY_WARP_PPD_SIZE, 0);
        write_i32(&mut bytes, WARP_PPD_BASE_OFFSET, self.warp_base);
        write_i32(&mut bytes, WARP_PPD_POINTS_OFFSET, self.warp_points);
        for index in 0..WARP_BONUS_COUNT {
            write_i32(
                &mut bytes,
                WARP_PPD_BONUS_ID_OFFSET + index * 4,
                self.warp_bonus_ids.get(index).copied().unwrap_or_default(),
            );
        }
        for index in 0..WARP_BONUS_COUNT {
            write_i32(
                &mut bytes,
                WARP_PPD_BONUS_LAST_USED_OFFSET + index * 4,
                self.warp_bonus_last_used
                    .get(index)
                    .copied()
                    .unwrap_or_default(),
            );
        }
        write_i32(&mut bytes, WARP_PPD_NOSTEPEXP_OFFSET, self.warp_nostepexp);
        bytes
    }

    pub fn decode_legacy_warp_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_WARP_PPD_SIZE {
            return false;
        }
        self.warp_ppd = bytes[..LEGACY_WARP_PPD_SIZE].to_vec();
        self.warp_base = read_i32(&self.warp_ppd, WARP_PPD_BASE_OFFSET);
        self.warp_points = read_i32(&self.warp_ppd, WARP_PPD_POINTS_OFFSET);
        self.warp_bonus_ids.resize(WARP_BONUS_COUNT, 0);
        self.warp_bonus_last_used.resize(WARP_BONUS_COUNT, 0);
        for index in 0..WARP_BONUS_COUNT {
            self.warp_bonus_ids[index] =
                read_i32(&self.warp_ppd, WARP_PPD_BONUS_ID_OFFSET + index * 4);
            self.warp_bonus_last_used[index] =
                read_i32(&self.warp_ppd, WARP_PPD_BONUS_LAST_USED_OFFSET + index * 4);
        }
        self.warp_nostepexp = read_i32(&self.warp_ppd, WARP_PPD_NOSTEPEXP_OFFSET);
        true
    }

    pub fn encode_legacy_gate_ppd(&self) -> Vec<u8> {
        let mut bytes = self.gate_ppd.clone();
        bytes.resize(LEGACY_GATE_PPD_SIZE, 0);
        write_i32(
            &mut bytes,
            GATE_PPD_WELCOME_STATE_OFFSET,
            self.gate_welcome_state,
        );
        write_i32(
            &mut bytes,
            GATE_PPD_TARGET_CLASS_OFFSET,
            self.gate_target_class,
        );
        write_i32(&mut bytes, GATE_PPD_STEP_OFFSET, self.gate_step);
        bytes
    }

    pub fn decode_legacy_gate_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_GATE_PPD_SIZE {
            return false;
        }
        self.gate_ppd = bytes[..LEGACY_GATE_PPD_SIZE].to_vec();
        self.gate_welcome_state = read_i32(&self.gate_ppd, GATE_PPD_WELCOME_STATE_OFFSET);
        self.gate_target_class = read_i32(&self.gate_ppd, GATE_PPD_TARGET_CLASS_OFFSET);
        self.gate_step = read_i32(&self.gate_ppd, GATE_PPD_STEP_OFFSET);
        true
    }

    pub fn ensure_legacy_lab2_described_graves(&mut self) -> [u8; 4] {
        self.ensure_legacy_lab2_described_graves_with_indices([0, 4, 8, 9])
    }

    pub fn ensure_legacy_lab2_described_graves_with_indices(
        &mut self,
        indices: [u8; 4],
    ) -> [u8; 4] {
        if self.lab_ppd.len() < LEGACY_LAB_PPD_SIZE {
            self.lab_ppd.resize(LEGACY_LAB_PPD_SIZE, 0);
        }
        if self.lab_ppd[LEGACY_LAB2_GRAVEVERSION_OFFSET] != LEGACY_LAB2_GRAVE_VERSION {
            self.lab_ppd[LEGACY_LAB2_GRAVEVERSION_OFFSET] = LEGACY_LAB2_GRAVE_VERSION;
            self.lab_ppd[LEGACY_LAB2_GRAVEINDEX_OFFSET..LEGACY_LAB2_GRAVEINDEX_OFFSET + 4]
                .copy_from_slice(&indices);
        }
        self.legacy_lab2_grave_indices()
    }

    pub fn legacy_lab2_grave_indices(&self) -> [u8; 4] {
        if self.lab_ppd.len() < LEGACY_LAB2_GRAVEINDEX_OFFSET + 4 {
            return [0, 0, 0, 0];
        }
        let mut indices = [0u8; 4];
        indices.copy_from_slice(
            &self.lab_ppd[LEGACY_LAB2_GRAVEINDEX_OFFSET..LEGACY_LAB2_GRAVEINDEX_OFFSET + 4],
        );
        indices
    }

    pub fn legacy_lab2_grave_clue_text(&mut self, book: u8) -> Option<String> {
        let indices = self.ensure_legacy_lab2_described_graves();
        let (slot, name) = match book {
            1 => (0, "Henry"),
            2 => (1, "Eldrick"),
            3 => (2, "John"),
            4 => (3, "Mariah"),
            _ => return None,
        };
        let description = LAB2_DESCRIBED_GRAVES
            .get(indices[slot] as usize)
            .map(|(_, description)| *description)
            .unwrap_or("%s is buried in an unknown grave.");
        Some(description.replace("%s", name))
    }

    pub fn legacy_lab2_special_grave_kind_at(&mut self, x: u16, y: u16) -> Option<u8> {
        let indices = self.ensure_legacy_lab2_described_graves();
        indices.into_iter().enumerate().find_map(|(slot, index)| {
            let ((grave_x, grave_y), _) = *LAB2_DESCRIBED_GRAVES.get(index as usize)?;
            (grave_x == x && grave_y == y).then_some(slot as u8 + 1)
        })
    }

    pub fn legacy_lab2_grave_cleared(&self, grave_number: usize) -> bool {
        let byte = grave_number / 8;
        let bit = grave_number % 8;
        self.lab2_grave_bits
            .get(byte)
            .is_some_and(|value| value & (1 << bit) != 0)
    }

    pub fn mark_legacy_lab2_grave_cleared(&mut self, grave_number: usize) -> bool {
        let byte = grave_number / 8;
        let bit = grave_number % 8;
        if byte >= LAB2_GRAVE_BITSET_BYTES {
            return false;
        }
        if self.lab2_grave_bits.len() <= byte {
            self.lab2_grave_bits.resize(byte + 1, 0);
        }
        let was_cleared = self.lab2_grave_bits[byte] & (1 << bit) != 0;
        self.lab2_grave_bits[byte] |= 1 << bit;
        !was_cleared
    }

    pub fn encode_legacy_pk_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_PK_PPD_SIZE];
        write_i32(
            &mut bytes,
            PK_PPD_KILLS_OFFSET,
            self.pk_kills.min(i32::MAX as u32) as i32,
        );
        write_i32(
            &mut bytes,
            PK_PPD_DEATHS_OFFSET,
            self.pk_deaths.min(i32::MAX as u32) as i32,
        );
        write_i32(
            &mut bytes,
            PK_PPD_LAST_KILL_OFFSET,
            self.pk_last_kill.min(i32::MAX as u32) as i32,
        );
        write_i32(
            &mut bytes,
            PK_PPD_LAST_DEATH_OFFSET,
            self.pk_last_death.min(i32::MAX as u32) as i32,
        );
        for (index, character_id) in self
            .pk_hate
            .iter()
            .copied()
            .take(PK_HATE_MAX_ENTRIES)
            .enumerate()
        {
            write_i32(
                &mut bytes,
                PK_PPD_HATE_OFFSET + index * 4,
                character_id.min(i32::MAX as u32) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_pk_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_PK_PPD_SIZE {
            return false;
        }

        self.pk_kills = read_i32(bytes, PK_PPD_KILLS_OFFSET).max(0) as u32;
        self.pk_deaths = read_i32(bytes, PK_PPD_DEATHS_OFFSET).max(0) as u32;
        self.pk_last_kill = read_i32(bytes, PK_PPD_LAST_KILL_OFFSET).max(0) as u32;
        self.pk_last_death = read_i32(bytes, PK_PPD_LAST_DEATH_OFFSET).max(0) as u32;
        self.pk_hate.clear();
        for index in 0..PK_HATE_MAX_ENTRIES {
            let character_id = read_i32(bytes, PK_PPD_HATE_OFFSET + index * 4);
            self.pk_hate.push(character_id.max(0) as u32);
        }
        Self::trim_pk_hate_slots(&mut self.pk_hate);
        true
    }

    fn trim_pk_hate_slots(slots: &mut Vec<u32>) {
        while slots.last().copied() == Some(0) {
            slots.pop();
        }
    }

    pub fn has_any_pk_hate(&self) -> bool {
        self.pk_hate.iter().any(|hate_id| *hate_id != 0)
    }

    pub fn active_pk_hate_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.pk_hate.iter().copied().filter(|hate_id| *hate_id != 0)
    }

    pub fn has_pk_hate_for(&self, character_id: u32) -> bool {
        character_id != 0 && self.pk_hate.iter().any(|hate_id| *hate_id == character_id)
    }

    pub fn add_pk_hate(&mut self, character_id: u32) -> bool {
        if character_id == 0 {
            return false;
        }

        let mut slots = [0_u32; PK_HATE_MAX_ENTRIES];
        for (index, hate_id) in self
            .pk_hate
            .iter()
            .copied()
            .take(PK_HATE_MAX_ENTRIES)
            .enumerate()
        {
            slots[index] = hate_id;
        }

        let position = (0..PK_HATE_MAX_ENTRIES - 1).find(|index| slots[*index] == character_id);
        let newly_added = position.is_none();
        let shift_count = position.unwrap_or(PK_HATE_MAX_ENTRIES - 1);
        for index in (1..=shift_count).rev() {
            slots[index] = slots[index - 1];
        }
        slots[0] = character_id;

        self.pk_hate = slots.to_vec();
        Self::trim_pk_hate_slots(&mut self.pk_hate);
        newly_added
    }

    pub fn add_pk_hate_from_hit(
        &mut self,
        character: &mut Character,
        attacker_character_id: u32,
    ) -> bool {
        let newly_added = self.add_pk_hate(attacker_character_id);
        if attacker_character_id != 0 {
            character.flags.remove(CharacterFlags::LAG);
        }
        newly_added
    }

    pub fn add_pk_kill(&mut self, realtime_seconds: u64) {
        self.pk_kills = self.pk_kills.saturating_add(1);
        self.pk_last_kill = realtime_seconds.min(i32::MAX as u64) as u32;
    }

    pub fn add_pk_death(&mut self, realtime_seconds: u64) {
        self.pk_deaths = self.pk_deaths.saturating_add(1);
        self.pk_last_death = realtime_seconds.min(i32::MAX as u64) as u32;
    }

    pub fn remove_pk_hate(&mut self, character_id: u32) -> bool {
        let Some(position) = self
            .pk_hate
            .iter()
            .position(|hate_id| *hate_id == character_id)
        else {
            return false;
        };
        self.pk_hate[position] = 0;
        Self::trim_pk_hate_slots(&mut self.pk_hate);
        true
    }

    pub fn touch_special_shrine(
        &mut self,
        character: &mut Character,
        kind: u8,
        realtime_seconds: u64,
    ) -> SpecialShrineResult {
        if kind != 0x0A {
            return SpecialShrineResult::Unsupported;
        }
        if !character.flags.contains(CharacterFlags::HARDCORE)
            || character.creation_time > SPECIAL_SHRINE_HCSC_CUTOFF_SECONDS
        {
            return SpecialShrineResult::NothingHere;
        }
        if self.special_shrine_hcsc_last_touch_seconds == 0
            || realtime_seconds.saturating_sub(self.special_shrine_hcsc_last_touch_seconds)
                > SPECIAL_SHRINE_CONFIRM_WINDOW_SECONDS
        {
            self.special_shrine_hcsc_last_touch_seconds = realtime_seconds;
            return SpecialShrineResult::ConfirmRequired;
        }

        character.flags.remove(CharacterFlags::HARDCORE);
        self.special_shrine_hcsc_last_touch_seconds = 0;
        SpecialShrineResult::HardcoreRemoved
    }

    pub fn chest_last_access_seconds(&self, treasure_index: u8) -> u64 {
        self.chest_last_access_seconds
            .get(&treasure_index)
            .copied()
            .unwrap_or_default()
    }

    pub fn mark_chest_access(&mut self, treasure_index: u8, realtime_seconds: u64) {
        self.chest_last_access_seconds
            .insert(treasure_index, realtime_seconds);
    }

    pub fn encode_legacy_treasure_chest_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_TREASURE_CHEST_PPD_SIZE];
        for (&treasure_index, &last_access_seconds) in &self.chest_last_access_seconds {
            let index = usize::from(treasure_index);
            if index >= TREASURE_CHEST_PPD_ENTRIES {
                continue;
            }
            write_i32(
                &mut bytes,
                index * 4,
                last_access_seconds.min(i32::MAX as u64) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_treasure_chest_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_TREASURE_CHEST_PPD_SIZE {
            return false;
        }

        self.chest_last_access_seconds.clear();
        for index in 0..TREASURE_CHEST_PPD_ENTRIES {
            let last_access_seconds = read_i32(bytes, index * 4);
            if last_access_seconds > 0 {
                self.chest_last_access_seconds
                    .insert(index as u8, last_access_seconds as u64);
            }
        }
        true
    }

    pub fn encode_legacy_keyring_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_KEYRING_PPD_SIZE];
        let count = self.keyring.len().min(KEYRING_MAX_KEYS);
        write_i32(&mut bytes, KEYRING_PPD_COUNT_OFFSET, count as i32);

        for (index, key) in self.keyring.iter().take(KEYRING_MAX_KEYS).enumerate() {
            write_u32(
                &mut bytes,
                KEYRING_PPD_KEYS_OFFSET + index * 4,
                key.template_id,
            );
            write_c_string(
                &mut bytes,
                KEYRING_PPD_NAMES_OFFSET + index * KEYRING_KEY_NAME_LEN,
                KEYRING_KEY_NAME_LEN,
                &key.name,
            );
            write_c_string(
                &mut bytes,
                KEYRING_PPD_DESCS_OFFSET + index * KEYRING_KEY_DESC_LEN,
                KEYRING_KEY_DESC_LEN,
                &key.description,
            );
            write_i32(
                &mut bytes,
                KEYRING_PPD_SPRITES_OFFSET + index * 4,
                key.sprite,
            );
            write_u64(&mut bytes, KEYRING_PPD_FLAGS_OFFSET + index * 8, key.flags);
            write_u32(&mut bytes, KEYRING_PPD_VALUES_OFFSET + index * 4, key.value);
            write_u16(
                &mut bytes,
                KEYRING_PPD_DRIVERS_OFFSET + index * 2,
                key.driver,
            );

            let drdata_offset = KEYRING_PPD_DRDATA_OFFSET + index * KEYRING_KEY_DRDATA_LEN;
            let drdata_len = key.driver_data.len().min(KEYRING_KEY_DRDATA_LEN);
            bytes[drdata_offset..drdata_offset + drdata_len]
                .copy_from_slice(&key.driver_data[..drdata_len]);
            bytes[KEYRING_PPD_EXPIRE_OFFSET + index] = key.expire_serial as u8;
        }

        write_i32(
            &mut bytes,
            KEYRING_PPD_AUTO_ADD_OFFSET,
            i32::from(self.keyring_auto_add),
        );
        bytes
    }

    pub fn decode_legacy_keyring_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_KEYRING_PPD_SIZE {
            return false;
        }

        let raw_count = read_i32(bytes, KEYRING_PPD_COUNT_OFFSET);
        let count = raw_count.clamp(0, KEYRING_MAX_KEYS as i32) as usize;
        let mut keyring = Vec::with_capacity(count);
        for index in 0..count {
            let driver_data_offset = KEYRING_PPD_DRDATA_OFFSET + index * KEYRING_KEY_DRDATA_LEN;
            keyring.push(KeyringEntry {
                template_id: read_u32(bytes, KEYRING_PPD_KEYS_OFFSET + index * 4),
                name: read_c_string(
                    bytes,
                    KEYRING_PPD_NAMES_OFFSET + index * KEYRING_KEY_NAME_LEN,
                    KEYRING_KEY_NAME_LEN,
                ),
                description: read_c_string(
                    bytes,
                    KEYRING_PPD_DESCS_OFFSET + index * KEYRING_KEY_DESC_LEN,
                    KEYRING_KEY_DESC_LEN,
                ),
                sprite: read_i32(bytes, KEYRING_PPD_SPRITES_OFFSET + index * 4),
                flags: read_u64(bytes, KEYRING_PPD_FLAGS_OFFSET + index * 8),
                value: read_u32(bytes, KEYRING_PPD_VALUES_OFFSET + index * 4),
                driver: read_u16(bytes, KEYRING_PPD_DRIVERS_OFFSET + index * 2),
                driver_data: bytes[driver_data_offset..driver_data_offset + KEYRING_KEY_DRDATA_LEN]
                    .to_vec(),
                expire_serial: u32::from(bytes[KEYRING_PPD_EXPIRE_OFFSET + index]),
            });
        }

        self.keyring = keyring;
        self.keyring_auto_add = read_i32(bytes, KEYRING_PPD_AUTO_ADD_OFFSET) != 0;
        true
    }

    pub fn encode_legacy_randchest_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_RANDCHEST_PPD_SIZE];
        for (index, entry) in self
            .random_chests
            .iter()
            .take(RANDCHEST_MAX_ENTRIES)
            .enumerate()
        {
            write_i32(
                &mut bytes,
                RANDCHEST_PPD_IDS_OFFSET + index * 4,
                entry.location_id.min(i32::MAX as u32) as i32,
            );
            write_i32(
                &mut bytes,
                RANDCHEST_PPD_LAST_USED_OFFSET + index * 4,
                entry.last_used_seconds.min(i32::MAX as u64) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_randchest_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_RANDCHEST_PPD_SIZE {
            return false;
        }

        self.random_chests.clear();
        for index in 0..RANDCHEST_MAX_ENTRIES {
            let location_id = read_i32(bytes, RANDCHEST_PPD_IDS_OFFSET + index * 4);
            let last_used_seconds = read_i32(bytes, RANDCHEST_PPD_LAST_USED_OFFSET + index * 4);
            if location_id > 0 && last_used_seconds > 0 {
                self.random_chests.push(RandomChestAccess {
                    location_id: location_id as u32,
                    last_used_seconds: last_used_seconds as u64,
                });
            }
        }
        true
    }

    pub fn encode_legacy_ratchest_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_RATCHEST_PPD_SIZE];
        for (index, entry) in self
            .rat_chests
            .iter()
            .take(RATCHEST_MAX_ENTRIES)
            .enumerate()
        {
            write_i32(
                &mut bytes,
                RATCHEST_PPD_IDS_OFFSET + index * 4,
                entry.location_id.min(i32::MAX as u32) as i32,
            );
            write_i32(
                &mut bytes,
                RATCHEST_PPD_LAST_USED_OFFSET + index * 4,
                entry.last_used_seconds.min(i32::MAX as u64) as i32,
            );
        }
        write_i32(
            &mut bytes,
            RATCHEST_PPD_TREASURE_X_OFFSET,
            i32::from(self.rat_chest_treasure_x),
        );
        write_i32(
            &mut bytes,
            RATCHEST_PPD_TREASURE_Y_OFFSET,
            i32::from(self.rat_chest_treasure_y),
        );
        write_i32(
            &mut bytes,
            RATCHEST_PPD_LAST_TREASURE_OFFSET,
            self.rat_chest_last_treasure_seconds.min(i32::MAX as u64) as i32,
        );
        bytes
    }

    pub fn decode_legacy_ratchest_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_RATCHEST_PPD_SIZE {
            return false;
        }

        self.rat_chests.clear();
        for index in 0..RATCHEST_MAX_ENTRIES {
            let location_id = read_i32(bytes, RATCHEST_PPD_IDS_OFFSET + index * 4);
            let last_used_seconds = read_i32(bytes, RATCHEST_PPD_LAST_USED_OFFSET + index * 4);
            if location_id > 0 && last_used_seconds > 0 {
                self.rat_chests.push(RatChestAccess {
                    location_id: location_id as u32,
                    last_used_seconds: last_used_seconds as u64,
                });
            }
        }
        self.rat_chest_treasure_x = read_i32(bytes, RATCHEST_PPD_TREASURE_X_OFFSET).max(0) as u16;
        self.rat_chest_treasure_y = read_i32(bytes, RATCHEST_PPD_TREASURE_Y_OFFSET).max(0) as u16;
        self.rat_chest_last_treasure_seconds =
            read_i32(bytes, RATCHEST_PPD_LAST_TREASURE_OFFSET).max(0) as u64;
        true
    }

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

    pub fn encode_legacy_demonshrine_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_DEMONSHRINE_PPD_SIZE];
        for (index, location_id) in self
            .demonshrines
            .iter()
            .copied()
            .take(DEMONSHRINE_MAX_ENTRIES)
            .enumerate()
        {
            write_i32(
                &mut bytes,
                index * 4,
                location_id.min(i32::MAX as u32) as i32,
            );
        }
        bytes
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

    pub fn decode_legacy_demonshrine_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_DEMONSHRINE_PPD_SIZE {
            return false;
        }

        self.demonshrines.clear();
        for index in 0..DEMONSHRINE_MAX_ENTRIES {
            let location_id = read_i32(bytes, index * 4);
            if location_id > 0 {
                self.demonshrines.push(location_id as u32);
            }
        }
        true
    }

    pub fn encode_legacy_randomshrine_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_RANDOMSHRINE_PPD_SIZE];
        for (index, word) in self.random_shrine_used_words.iter().copied().enumerate() {
            write_u32(&mut bytes, index * 4, word);
        }
        bytes[RANDOMSHRINE_USED_WORDS * 4] = self.random_shrine_continuity;
        bytes
    }

    pub fn decode_legacy_randomshrine_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_RANDOMSHRINE_PPD_SIZE {
            return false;
        }
        for index in 0..RANDOMSHRINE_USED_WORDS {
            self.random_shrine_used_words[index] = read_u32(bytes, index * 4);
        }
        self.random_shrine_continuity = bytes[RANDOMSHRINE_USED_WORDS * 4];
        true
    }

    pub fn has_used_random_shrine(&self, shrine: u8) -> bool {
        let word = usize::from(shrine / 32);
        let bit = 1u32 << (shrine & 31);
        self.random_shrine_used_words[word] & bit != 0
    }

    pub fn mark_random_shrine_used(&mut self, shrine: u8) {
        let word = usize::from(shrine / 32);
        let bit = 1u32 << (shrine & 31);
        self.random_shrine_used_words[word] |= bit;
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

    pub fn encode_legacy_area3_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_AREA3_PPD_SIZE];
        let copy_len = self.area3_ppd.len().min(LEGACY_AREA3_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.area3_ppd[..copy_len]);
        bytes
    }

    pub fn decode_legacy_area3_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_AREA3_PPD_SIZE {
            return false;
        }
        self.area3_ppd = bytes[..LEGACY_AREA3_PPD_SIZE].to_vec();
        true
    }

    pub fn area3_imp_flags(&self) -> u32 {
        if self.area3_ppd.len() < LEGACY_AREA3_PPD_SIZE {
            return 0;
        }
        read_i32(&self.area3_ppd, AREA3_PPD_IMP_FLAGS_OFFSET).max(0) as u32
    }

    pub fn area3_kelly_state(&self) -> i32 {
        self.read_area3_i32(AREA3_PPD_KELLY_STATE_OFFSET)
    }

    pub fn set_area3_kelly_state(&mut self, state: i32) {
        self.write_area3_i32(AREA3_PPD_KELLY_STATE_OFFSET, state);
    }

    pub fn area3_clara_state(&self) -> i32 {
        self.read_area3_i32(AREA3_PPD_CLARA_STATE_OFFSET)
    }

    pub fn set_area3_clara_state(&mut self, state: i32) {
        self.write_area3_i32(AREA3_PPD_CLARA_STATE_OFFSET, state);
    }

    fn read_area3_i32(&self, offset: usize) -> i32 {
        if self.area3_ppd.len() < LEGACY_AREA3_PPD_SIZE {
            return 0;
        }
        read_i32(&self.area3_ppd, offset)
    }

    fn write_area3_i32(&mut self, offset: usize, value: i32) {
        if self.area3_ppd.len() < LEGACY_AREA3_PPD_SIZE {
            self.area3_ppd.resize(LEGACY_AREA3_PPD_SIZE, 0);
        }
        write_i32(&mut self.area3_ppd, offset, value);
    }

    pub fn mark_area3_imp_flag(&mut self, mask: u32) -> bool {
        if self.area3_ppd.len() < LEGACY_AREA3_PPD_SIZE {
            self.area3_ppd.resize(LEGACY_AREA3_PPD_SIZE, 0);
        }
        let current = self.area3_imp_flags();
        if current & mask != 0 {
            return false;
        }
        write_i32(
            &mut self.area3_ppd,
            AREA3_PPD_IMP_FLAGS_OFFSET,
            (current | mask) as i32,
        );
        true
    }

    pub fn encode_legacy_area1_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_AREA1_PPD_SIZE];
        let copy_len = self.area1_ppd.len().min(LEGACY_AREA1_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.area1_ppd[..copy_len]);
        bytes
    }

    pub fn decode_legacy_area1_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_AREA1_PPD_SIZE {
            return false;
        }
        self.area1_ppd = bytes[..LEGACY_AREA1_PPD_SIZE].to_vec();
        true
    }

    fn read_area1_i32(&self, offset: usize) -> i32 {
        if self.area1_ppd.len() < LEGACY_AREA1_PPD_SIZE {
            return 0;
        }
        read_i32(&self.area1_ppd, offset)
    }

    fn write_area1_i32(&mut self, offset: usize, value: i32) {
        if self.area1_ppd.len() < LEGACY_AREA1_PPD_SIZE {
            self.area1_ppd.resize(LEGACY_AREA1_PPD_SIZE, 0);
        }
        write_i32(&mut self.area1_ppd, offset, value);
    }

    pub fn area1_yoakin_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_YOAKIN_STATE_OFFSET)
    }

    pub fn set_area1_yoakin_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_YOAKIN_STATE_OFFSET, state);
    }

    pub fn area1_gwendy_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_GWENDY_STATE_OFFSET)
    }

    pub fn set_area1_gwendy_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_GWENDY_STATE_OFFSET, state);
    }

    pub fn area1_nook_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_NOOK_STATE_OFFSET)
    }

    pub fn set_area1_nook_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_NOOK_STATE_OFFSET, state);
    }

    pub fn area1_lydia_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_LYDIA_STATE_OFFSET)
    }

    pub fn set_area1_lydia_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_LYDIA_STATE_OFFSET, state);
    }

    pub fn area1_guiwynn_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_GUIWYNN_STATE_OFFSET)
    }

    pub fn set_area1_guiwynn_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_GUIWYNN_STATE_OFFSET, state);
    }

    pub fn area1_logain_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_LOGAIN_STATE_OFFSET)
    }

    pub fn set_area1_logain_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_LOGAIN_STATE_OFFSET, state);
    }

    pub fn area1_reskin_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_RESKIN_STATE_OFFSET)
    }

    pub fn set_area1_reskin_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_RESKIN_STATE_OFFSET, state);
    }

    pub fn area1_brithildie_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_BRITHILDIE_STATE_OFFSET)
    }

    pub fn set_area1_brithildie_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_BRITHILDIE_STATE_OFFSET, state);
    }

    pub fn area1_camhermit_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_CAMHERMIT_STATE_OFFSET)
    }

    pub fn set_area1_camhermit_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_CAMHERMIT_STATE_OFFSET, state);
    }

    pub fn area1_jessica_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_JESSICA_STATE_OFFSET)
    }

    pub fn set_area1_jessica_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_JESSICA_STATE_OFFSET, state);
    }

    /// Snapshot of the `area1_ppd` fields consumed by
    /// `questlog_init_area1` (`src/system/questlog.c:828-1039`), for
    /// `crate::quest::init_area1_quests`.
    pub fn area1_quest_state(&self) -> crate::quest::Area1QuestState {
        crate::quest::Area1QuestState {
            lydia_state: self.area1_lydia_state(),
            gwendy_state: self.area1_gwendy_state(),
            yoakin_state: self.area1_yoakin_state(),
            nook_state: self.area1_nook_state(),
            guiwynn_state: self.area1_guiwynn_state(),
            logain_state: self.area1_logain_state(),
            reskin_state: self.area1_reskin_state(),
            jessica_state: self.area1_jessica_state(),
            brithildie_state: self.area1_brithildie_state(),
            camhermit_state: self.area1_camhermit_state(),
        }
    }

    pub fn encode_legacy_nomad_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_NOMAD_PPD_SIZE];
        let copy_len = self.nomad_ppd.len().min(LEGACY_NOMAD_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.nomad_ppd[..copy_len]);
        bytes
    }

    pub fn decode_legacy_nomad_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_NOMAD_PPD_SIZE {
            return false;
        }
        self.nomad_ppd = bytes[..LEGACY_NOMAD_PPD_SIZE].to_vec();
        true
    }

    /// C `nomad_state[MAXNOMAD]` element read (`src/common/nomad_ppd.h:10`).
    pub fn nomad_state(&self, index: usize) -> i32 {
        if index >= NOMAD_PPD_MAXNOMAD || self.nomad_ppd.len() < LEGACY_NOMAD_PPD_SIZE {
            return 0;
        }
        read_i32(&self.nomad_ppd, NOMAD_PPD_STATE_OFFSET + index * 4)
    }

    pub fn set_nomad_state(&mut self, index: usize, value: i32) {
        if index >= NOMAD_PPD_MAXNOMAD {
            return;
        }
        if self.nomad_ppd.len() < LEGACY_NOMAD_PPD_SIZE {
            self.nomad_ppd.resize(LEGACY_NOMAD_PPD_SIZE, 0);
        }
        write_i32(
            &mut self.nomad_ppd,
            NOMAD_PPD_STATE_OFFSET + index * 4,
            value,
        );
    }

    /// C `nomad_win[MAXNOMAD]` element read (`src/common/nomad_ppd.h:11`).
    pub fn nomad_win(&self, index: usize) -> i32 {
        if index >= NOMAD_PPD_MAXNOMAD || self.nomad_ppd.len() < LEGACY_NOMAD_PPD_SIZE {
            return 0;
        }
        read_i32(&self.nomad_ppd, NOMAD_PPD_WIN_OFFSET + index * 4)
    }

    pub fn set_nomad_win(&mut self, index: usize, value: i32) {
        if index >= NOMAD_PPD_MAXNOMAD {
            return;
        }
        if self.nomad_ppd.len() < LEGACY_NOMAD_PPD_SIZE {
            self.nomad_ppd.resize(LEGACY_NOMAD_PPD_SIZE, 0);
        }
        write_i32(&mut self.nomad_ppd, NOMAD_PPD_WIN_OFFSET + index * 4, value);
    }

    /// Snapshot of the `nomad_state[]` array consumed by
    /// `questlog_init_nomad` (`src/system/questlog.c:1571-1607`), for
    /// `crate::quest::init_nomad_quests`.
    pub fn nomad_quest_state(&self) -> crate::quest::NomadQuestState {
        let mut nomad_state = [0i32; NOMAD_PPD_MAXNOMAD];
        for (index, slot) in nomad_state.iter_mut().enumerate() {
            *slot = self.nomad_state(index);
        }
        crate::quest::NomadQuestState { nomad_state }
    }

    pub fn encode_legacy_caligar_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_CALIGAR_PPD_SIZE];
        let copy_len = self.caligar_ppd.len().min(LEGACY_CALIGAR_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.caligar_ppd[..copy_len]);
        bytes
    }

    pub fn decode_legacy_caligar_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_CALIGAR_PPD_SIZE {
            return false;
        }
        self.caligar_ppd = bytes[..LEGACY_CALIGAR_PPD_SIZE].to_vec();
        true
    }

    pub fn encode_legacy_arkhata_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_ARKHATA_PPD_SIZE];
        let copy_len = self.arkhata_ppd.len().min(LEGACY_ARKHATA_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.arkhata_ppd[..copy_len]);
        bytes
    }

    pub fn decode_legacy_arkhata_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_ARKHATA_PPD_SIZE {
            return false;
        }
        self.arkhata_ppd = bytes[..LEGACY_ARKHATA_PPD_SIZE].to_vec();
        true
    }

    pub fn encode_legacy_staffer_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_STAFFER_PPD_SIZE];
        let len = self.staffer_ppd.len().min(LEGACY_STAFFER_PPD_SIZE);
        bytes[..len].copy_from_slice(&self.staffer_ppd[..len]);
        bytes
    }

    pub fn decode_legacy_staffer_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_STAFFER_PPD_SIZE {
            return false;
        }
        self.staffer_ppd = bytes[..LEGACY_STAFFER_PPD_SIZE].to_vec();
        true
    }

    pub fn forestbran_done(&self) -> u8 {
        if self.staffer_ppd.len() < LEGACY_STAFFER_PPD_SIZE {
            return 0;
        }
        read_i32(&self.staffer_ppd, STAFFER_PPD_FORESTBRAN_DONE_OFFSET).clamp(0, 5) as u8
    }

    pub fn set_forestbran_done(&mut self, dig_index: u8) -> Option<u8> {
        if dig_index >= TREASURE_DIG_PPD_ENTRIES as u8 {
            return None;
        }
        let done = dig_index + 1;
        if self.staffer_ppd.len() < LEGACY_STAFFER_PPD_SIZE {
            self.staffer_ppd.resize(LEGACY_STAFFER_PPD_SIZE, 0);
        }
        write_i32(
            &mut self.staffer_ppd,
            STAFFER_PPD_FORESTBRAN_DONE_OFFSET,
            i32::from(done),
        );
        Some(done)
    }

    pub fn mark_staffer_animation_book_seen(&mut self) -> bool {
        if self.staffer_ppd.len() < LEGACY_STAFFER_PPD_SIZE {
            self.staffer_ppd.resize(LEGACY_STAFFER_PPD_SIZE, 0);
        }
        let state = read_i32(&self.staffer_ppd, STAFFER_PPD_SHANRA_STATE_OFFSET);
        if state >= 3 {
            return false;
        }
        write_i32(&mut self.staffer_ppd, STAFFER_PPD_SHANRA_STATE_OFFSET, 3);
        true
    }

    pub fn encode_legacy_farmy_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_FARMY_PPD_SIZE];
        let len = self.farmy_ppd.len().min(LEGACY_FARMY_PPD_SIZE);
        bytes[..len].copy_from_slice(&self.farmy_ppd[..len]);
        bytes
    }

    pub fn decode_legacy_farmy_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_FARMY_PPD_SIZE {
            return false;
        }
        self.farmy_ppd = bytes[..LEGACY_FARMY_PPD_SIZE].to_vec();
        true
    }

    pub fn farmy_boss_stage(&self) -> i32 {
        if self.farmy_ppd.len() < LEGACY_FARMY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.farmy_ppd, FARMY_PPD_BOSS_STAGE_OFFSET)
    }

    pub fn advance_farmy_blood_stage(&mut self) -> bool {
        let stage = self.farmy_boss_stage();
        if !(19..=20).contains(&stage) {
            return false;
        }
        if self.farmy_ppd.len() < LEGACY_FARMY_PPD_SIZE {
            self.farmy_ppd.resize(LEGACY_FARMY_PPD_SIZE, 0);
        }
        write_i32(&mut self.farmy_ppd, FARMY_PPD_BOSS_STAGE_OFFSET, 21);
        true
    }

    pub fn advance_farmy_lava_stage(&mut self) -> bool {
        let stage = self.farmy_boss_stage();
        if !(22..=23).contains(&stage) {
            return false;
        }
        if self.farmy_ppd.len() < LEGACY_FARMY_PPD_SIZE {
            self.farmy_ppd.resize(LEGACY_FARMY_PPD_SIZE, 0);
        }
        write_i32(&mut self.farmy_ppd, FARMY_PPD_BOSS_STAGE_OFFSET, 24);
        true
    }

    pub fn encode_legacy_teufelrat_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_TEUFELRAT_PPD_SIZE];
        write_i32(
            &mut bytes,
            TEUFELRAT_PPD_KILLS_OFFSET,
            self.teufel_rat_kills.min(i32::MAX as u32) as i32,
        );
        write_i32(
            &mut bytes,
            TEUFELRAT_PPD_SCORE_OFFSET,
            self.teufel_rat_score.min(i32::MAX as u32) as i32,
        );
        bytes
    }

    pub fn decode_legacy_teufelrat_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_TEUFELRAT_PPD_SIZE {
            return false;
        }
        self.teufel_rat_kills = read_i32(bytes, TEUFELRAT_PPD_KILLS_OFFSET).max(0) as u32;
        self.teufel_rat_score = read_i32(bytes, TEUFELRAT_PPD_SCORE_OFFSET).max(0) as u32;
        true
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

    pub fn add_teufel_rat_kill(&mut self, rat_level: u32, reduced_score: bool) -> (u32, u32) {
        let score = if reduced_score {
            1
        } else {
            rat_level.saturating_mul(rat_level) / 100
        };
        self.teufel_rat_kills = self.teufel_rat_kills.saturating_add(1);
        self.teufel_rat_score = self.teufel_rat_score.saturating_add(score);
        (self.teufel_rat_kills, self.teufel_rat_score)
    }

    /// The `PlayerRuntime` half of `turn_seyan`'s ~22 `del_data` calls
    /// (`src/system/tool.c:4331-4353`; the character-only half is
    /// `World::apply_turn_seyan`). 16 of the cleared ids have dedicated
    /// typed fields here - reset each to its empty/default state so
    /// `encode_legacy_ppd_blob` naturally omits the block on next save,
    /// exactly like a character that never touched that system. The
    /// remaining 8 non-depot ids (`DRD_FIRSTKILL_PPD`, `DRD_RANK_PPD`,
    /// `DRD_MILITARY_PPD`, `DRD_ARENA_PPD`, `DRD_SIDESTORY_PPD`,
    /// `DRD_TUNNEL_PPD`, `DRD_STRATEGY_PPD`, `DRD_QUESTLOG_PPD`) have no
    /// Rust representation at all, so they're stripped straight out of the
    /// raw `ppd_blob` via `strip_ppd_blocks` (the same byte-level
    /// mechanism that already round-trips every other still-unmodeled
    /// id). `DRD_DEPOT_PPD`'s "clear `IF_QUEST` flags from the 80 depot
    /// item slots" is a documented gap - see `World::apply_turn_seyan`'s
    /// doc comment; no per-character legacy depot exists in Rust yet
    /// (`AccountDepotState`, `ugaris-server::depot`, is a distinct, newer
    /// system).
    pub fn clear_turn_seyan_ppd(&mut self) {
        self.chest_last_access_seconds.clear();
        self.area3_ppd.clear();
        self.area1_ppd.clear();
        self.nomad_ppd.clear();
        self.random_shrine_used_words = [0; RANDOMSHRINE_USED_WORDS];
        self.random_shrine_continuity = 0;
        self.flowers.clear();
        self.random_chests.clear();
        self.demonshrines.clear();
        self.farmy_ppd.clear();
        self.twocity_ppd.clear();
        self.twocity_goodtile = [0; 5];
        self.twocity_solved_library = false;
        self.orb_spawns.clear();
        self.rune_used_words = [0; RUNE_USED_WORDS];
        self.rune_special_exec = [0; RUNE_SPECIAL_EXEC_COUNT];
        self.lab_solved_bits = 0;
        self.lab_ppd.clear();
        self.rat_chests.clear();
        self.rat_chest_treasure_x = 0;
        self.rat_chest_treasure_y = 0;
        self.rat_chest_last_treasure_seconds = 0;
        self.staffer_ppd.clear();
        self.arkhata_ppd.clear();

        self.ppd_blob = strip_ppd_blocks(
            &self.ppd_blob,
            &[
                DRD_FIRSTKILL_PPD,
                DRD_RANK_PPD,
                DRD_MILITARY_PPD,
                DRD_ARENA_PPD,
                DRD_SIDESTORY_PPD,
                DRD_TUNNEL_PPD,
                DRD_STRATEGY_PPD,
                DRD_QUESTLOG_PPD,
            ],
        );
    }

    pub fn arkhata_clerk_state(&self) -> i32 {
        if self.arkhata_ppd.len() < LEGACY_ARKHATA_PPD_SIZE {
            return 0;
        }
        read_i32(&self.arkhata_ppd, ARKHATA_PPD_CLERK_STATE_OFFSET)
    }

    pub fn arkhata_clerk_time_seconds(&self) -> i32 {
        if self.arkhata_ppd.len() < LEGACY_ARKHATA_PPD_SIZE {
            return 0;
        }
        read_i32(&self.arkhata_ppd, ARKHATA_PPD_CLERK_TIME_OFFSET)
    }

    pub fn set_arkhata_clerk_timer(&mut self, state: i32, realtime_seconds: i32) {
        if self.arkhata_ppd.len() < LEGACY_ARKHATA_PPD_SIZE {
            self.arkhata_ppd.resize(LEGACY_ARKHATA_PPD_SIZE, 0);
        }
        write_i32(&mut self.arkhata_ppd, ARKHATA_PPD_CLERK_STATE_OFFSET, state);
        write_i32(
            &mut self.arkhata_ppd,
            ARKHATA_PPD_CLERK_TIME_OFFSET,
            realtime_seconds,
        );
    }

    pub fn encode_legacy_swear_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_SWEAR_PPD_SIZE];
        let copy_len = self.swear_ppd.len().min(LEGACY_SWEAR_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.swear_ppd[..copy_len]);
        write_i32(
            &mut bytes,
            SWEAR_PPD_BANNED_TILL_OFFSET,
            self.shutup_until_seconds.min(i32::MAX as u64) as i32,
        );
        bytes
    }

    pub fn decode_legacy_swear_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_SWEAR_PPD_SIZE {
            return false;
        }
        self.swear_ppd = bytes[..LEGACY_SWEAR_PPD_SIZE].to_vec();
        self.shutup_until_seconds = read_i32(bytes, SWEAR_PPD_BANNED_TILL_OFFSET).max(0) as u64;
        true
    }

    pub fn decode_legacy_ppd_blob(&mut self, bytes: &[u8]) -> bool {
        for block in LegacyPpdBlocks::parse(bytes) {
            let Some(block) = block else {
                return false;
            };
            match block.id {
                DRD_KEYRING_PPD => {
                    if !self.decode_legacy_keyring_ppd(block.data) {
                        return false;
                    }
                }
                DRD_TREASURE_CHEST_PPD => {
                    if !self.decode_legacy_treasure_chest_ppd(block.data) {
                        return false;
                    }
                }
                DRD_TRANSPORT_PPD => {
                    if !self.decode_legacy_transport_ppd(block.data) {
                        return false;
                    }
                }
                DRD_LAB_PPD => {
                    if !self.decode_legacy_lab_ppd(block.data) {
                        return false;
                    }
                }
                DRD_WARP_PPD => {
                    if !self.decode_legacy_warp_ppd(block.data) {
                        return false;
                    }
                }
                DRD_GATE_PPD => {
                    if !self.decode_legacy_gate_ppd(block.data) {
                        return false;
                    }
                }
                DRD_PK_PPD => {
                    if !self.decode_legacy_pk_ppd(block.data) {
                        return false;
                    }
                }
                DRD_RANDCHEST_PPD => {
                    if !self.decode_legacy_randchest_ppd(block.data) {
                        return false;
                    }
                }
                DRD_RATCHEST_PPD => {
                    if !self.decode_legacy_ratchest_ppd(block.data) {
                        return false;
                    }
                }
                DRD_DEMONSHRINE_PPD => {
                    if !self.decode_legacy_demonshrine_ppd(block.data) {
                        return false;
                    }
                }
                DRD_RANDOMSHRINE_PPD => {
                    if !self.decode_legacy_randomshrine_ppd(block.data) {
                        return false;
                    }
                }
                DRD_ORBSPAWN_PPD => {
                    if !self.decode_legacy_orbspawn_ppd(block.data) {
                        return false;
                    }
                }
                DRD_LOSTCON_PPD => {
                    if !self.decode_legacy_lostcon_ppd(block.data) {
                        return false;
                    }
                }
                DRD_FLOWER_PPD => {
                    if !self.decode_legacy_flower_ppd(block.data) {
                        return false;
                    }
                }
                DRD_AREA3_PPD => {
                    if !self.decode_legacy_area3_ppd(block.data) {
                        return false;
                    }
                }
                DRD_AREA1_PPD => {
                    if !self.decode_legacy_area1_ppd(block.data) {
                        return false;
                    }
                }
                DRD_NOMAD_PPD => {
                    if !self.decode_legacy_nomad_ppd(block.data) {
                        return false;
                    }
                }
                DRD_CALIGAR_PPD => {
                    if !self.decode_legacy_caligar_ppd(block.data) {
                        return false;
                    }
                }
                DRD_ARKHATA_PPD => {
                    if !self.decode_legacy_arkhata_ppd(block.data) {
                        return false;
                    }
                }
                DRD_STAFFER_PPD => {
                    if !self.decode_legacy_staffer_ppd(block.data) {
                        return false;
                    }
                }
                DRD_FARMY_PPD => {
                    if !self.decode_legacy_farmy_ppd(block.data) {
                        return false;
                    }
                }
                DRD_TEUFELRAT_PPD => {
                    if !self.decode_legacy_teufelrat_ppd(block.data) {
                        return false;
                    }
                }
                DRD_BANK_PPD => {
                    if !self.decode_legacy_bank_ppd(block.data) {
                        return false;
                    }
                }
                DRD_TWOCITY_PPD => {
                    if !self.decode_legacy_twocity_ppd(block.data) {
                        return false;
                    }
                }
                DRD_SALTMINE_PPD => {
                    if !self.decode_legacy_saltmine_ppd(block.data) {
                        return false;
                    }
                }
                DRD_TREASURE_DIG_PPD => {
                    if !self.decode_legacy_treasure_dig_ppd(block.data) {
                        return false;
                    }
                }
                DRD_MISC_PPD => {
                    if !self.decode_legacy_misc_ppd(block.data) {
                        return false;
                    }
                }
                DRD_RUNE_PPD => {
                    if !self.decode_legacy_rune_ppd(block.data) {
                        return false;
                    }
                }
                DRD_ALIAS_PPD => {
                    if !self.decode_legacy_alias_ppd(block.data) {
                        return false;
                    }
                }
                DRD_IGNORE_PPD => {
                    if !self.decode_legacy_ignore_ppd(block.data) {
                        return false;
                    }
                }
                DRD_SWEAR_PPD => {
                    if !self.decode_legacy_swear_ppd(block.data) {
                        return false;
                    }
                }
                _ => {}
            }
        }
        true
    }

    pub fn encode_legacy_ppd_blob(&self, existing: &[u8]) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(existing.len().max(LEGACY_KEYRING_PPD_SIZE + 8));
        let mut had_keyring = false;
        let mut had_treasure_chest = false;
        let mut had_transport = false;
        let mut had_lab = false;
        let mut had_warp = false;
        let mut had_gate = false;
        let mut had_pk = false;
        let mut had_randchest = false;
        let mut had_ratchest = false;
        let mut had_demonshrine = false;
        let mut had_randomshrine = false;
        let mut had_orbspawn = false;
        let mut had_lostcon = false;
        let mut had_flower = false;
        let mut had_area3 = false;
        let mut had_area1 = false;
        let mut had_nomad = false;
        let mut had_caligar = false;
        let mut had_arkhata = false;
        let mut had_staffer = false;
        let mut had_farmy = false;
        let mut had_teufelrat = false;
        let mut had_bank = false;
        let mut had_twocity = false;
        let mut had_saltmine = false;
        let mut had_treasure_dig = false;
        let mut had_misc = false;
        let mut had_rune = false;
        let mut had_alias = false;
        let mut had_ignore = false;
        let mut had_swear = false;
        let mut existing_was_valid = true;

        for block in LegacyPpdBlocks::parse(existing) {
            let Some(block) = block else {
                existing_was_valid = false;
                break;
            };
            if block.id == DRD_JUNK_PPD {
                continue;
            }
            if block.id == DRD_KEYRING_PPD {
                had_keyring = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_KEYRING_PPD,
                    &self.encode_legacy_keyring_ppd(),
                );
            } else if block.id == DRD_TREASURE_CHEST_PPD {
                had_treasure_chest = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_TREASURE_CHEST_PPD,
                    &self.encode_legacy_treasure_chest_ppd(),
                );
            } else if block.id == DRD_TRANSPORT_PPD {
                had_transport = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_TRANSPORT_PPD,
                    &self.encode_legacy_transport_ppd(),
                );
            } else if block.id == DRD_LAB_PPD {
                had_lab = true;
                write_ppd_block(&mut encoded, DRD_LAB_PPD, &self.encode_legacy_lab_ppd());
            } else if block.id == DRD_WARP_PPD {
                had_warp = true;
                write_ppd_block(&mut encoded, DRD_WARP_PPD, &self.encode_legacy_warp_ppd());
            } else if block.id == DRD_GATE_PPD {
                had_gate = true;
                write_ppd_block(&mut encoded, DRD_GATE_PPD, &self.encode_legacy_gate_ppd());
            } else if block.id == DRD_PK_PPD {
                had_pk = true;
                write_ppd_block(&mut encoded, DRD_PK_PPD, &self.encode_legacy_pk_ppd());
            } else if block.id == DRD_RANDCHEST_PPD {
                had_randchest = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_RANDCHEST_PPD,
                    &self.encode_legacy_randchest_ppd(),
                );
            } else if block.id == DRD_RATCHEST_PPD {
                had_ratchest = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_RATCHEST_PPD,
                    &self.encode_legacy_ratchest_ppd(),
                );
            } else if block.id == DRD_DEMONSHRINE_PPD {
                had_demonshrine = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_DEMONSHRINE_PPD,
                    &self.encode_legacy_demonshrine_ppd(),
                );
            } else if block.id == DRD_RANDOMSHRINE_PPD {
                had_randomshrine = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_RANDOMSHRINE_PPD,
                    &self.encode_legacy_randomshrine_ppd(),
                );
            } else if block.id == DRD_ORBSPAWN_PPD {
                had_orbspawn = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_ORBSPAWN_PPD,
                    &self.encode_legacy_orbspawn_ppd(),
                );
            } else if block.id == DRD_LOSTCON_PPD {
                had_lostcon = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_LOSTCON_PPD,
                    &self.encode_legacy_lostcon_ppd(),
                );
            } else if block.id == DRD_FLOWER_PPD {
                had_flower = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_FLOWER_PPD,
                    &self.encode_legacy_flower_ppd(),
                );
            } else if block.id == DRD_AREA3_PPD {
                had_area3 = true;
                write_ppd_block(&mut encoded, DRD_AREA3_PPD, &self.encode_legacy_area3_ppd());
            } else if block.id == DRD_AREA1_PPD {
                had_area1 = true;
                write_ppd_block(&mut encoded, DRD_AREA1_PPD, &self.encode_legacy_area1_ppd());
            } else if block.id == DRD_NOMAD_PPD {
                had_nomad = true;
                write_ppd_block(&mut encoded, DRD_NOMAD_PPD, &self.encode_legacy_nomad_ppd());
            } else if block.id == DRD_CALIGAR_PPD {
                had_caligar = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_CALIGAR_PPD,
                    &self.encode_legacy_caligar_ppd(),
                );
            } else if block.id == DRD_ARKHATA_PPD {
                had_arkhata = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_ARKHATA_PPD,
                    &self.encode_legacy_arkhata_ppd(),
                );
            } else if block.id == DRD_STAFFER_PPD {
                had_staffer = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_STAFFER_PPD,
                    &self.encode_legacy_staffer_ppd(),
                );
            } else if block.id == DRD_FARMY_PPD {
                had_farmy = true;
                write_ppd_block(&mut encoded, DRD_FARMY_PPD, &self.encode_legacy_farmy_ppd());
            } else if block.id == DRD_TEUFELRAT_PPD {
                had_teufelrat = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_TEUFELRAT_PPD,
                    &self.encode_legacy_teufelrat_ppd(),
                );
            } else if block.id == DRD_BANK_PPD {
                had_bank = true;
                write_ppd_block(&mut encoded, DRD_BANK_PPD, &self.encode_legacy_bank_ppd());
            } else if block.id == DRD_TWOCITY_PPD {
                had_twocity = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_TWOCITY_PPD,
                    &self.encode_legacy_twocity_ppd(),
                );
            } else if block.id == DRD_SALTMINE_PPD {
                had_saltmine = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_SALTMINE_PPD,
                    &self.encode_legacy_saltmine_ppd(),
                );
            } else if block.id == DRD_TREASURE_DIG_PPD {
                had_treasure_dig = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_TREASURE_DIG_PPD,
                    &self.encode_legacy_treasure_dig_ppd(),
                );
            } else if block.id == DRD_MISC_PPD {
                had_misc = true;
                write_ppd_block(&mut encoded, DRD_MISC_PPD, &self.encode_legacy_misc_ppd());
            } else if block.id == DRD_RUNE_PPD {
                had_rune = true;
                write_ppd_block(&mut encoded, DRD_RUNE_PPD, &self.encode_legacy_rune_ppd());
            } else if block.id == DRD_ALIAS_PPD {
                had_alias = true;
                if !self.aliases.is_empty() {
                    write_ppd_block(&mut encoded, DRD_ALIAS_PPD, &self.encode_legacy_alias_ppd());
                }
            } else if block.id == DRD_IGNORE_PPD {
                had_ignore = true;
                if !self.ignored_characters.is_empty() {
                    write_ppd_block(
                        &mut encoded,
                        DRD_IGNORE_PPD,
                        &self.encode_legacy_ignore_ppd(),
                    );
                }
            } else if block.id == DRD_SWEAR_PPD {
                had_swear = true;
                if !self.swear_ppd.is_empty() || self.shutup_until_seconds != 0 {
                    write_ppd_block(&mut encoded, DRD_SWEAR_PPD, &self.encode_legacy_swear_ppd());
                }
            } else {
                write_ppd_block(&mut encoded, block.id, block.data);
            }
        }

        if !had_keyring && (existing_was_valid || existing.is_empty()) {
            if !self.keyring.is_empty() || self.keyring_auto_add {
                write_ppd_block(
                    &mut encoded,
                    DRD_KEYRING_PPD,
                    &self.encode_legacy_keyring_ppd(),
                );
            }
        }
        if !had_treasure_chest && (existing_was_valid || existing.is_empty()) {
            if !self.chest_last_access_seconds.is_empty() {
                write_ppd_block(
                    &mut encoded,
                    DRD_TREASURE_CHEST_PPD,
                    &self.encode_legacy_treasure_chest_ppd(),
                );
            }
        }
        if !had_transport && (existing_was_valid || existing.is_empty()) && self.transport_seen != 0
        {
            write_ppd_block(
                &mut encoded,
                DRD_TRANSPORT_PPD,
                &self.encode_legacy_transport_ppd(),
            );
        }
        if !had_lab
            && (existing_was_valid || existing.is_empty())
            && (self.lab_solved_bits != 0 || !self.lab_ppd.is_empty())
        {
            write_ppd_block(&mut encoded, DRD_LAB_PPD, &self.encode_legacy_lab_ppd());
        }
        if !had_warp && (existing_was_valid || existing.is_empty()) {
            if self.warp_base != 0
                || self.warp_points != 0
                || self.warp_nostepexp != 0
                || self.warp_bonus_ids.iter().any(|value| *value != 0)
                || self.warp_bonus_last_used.iter().any(|value| *value != 0)
                || !self.warp_ppd.is_empty()
            {
                write_ppd_block(&mut encoded, DRD_WARP_PPD, &self.encode_legacy_warp_ppd());
            }
        }
        if !had_gate && (existing_was_valid || existing.is_empty()) {
            if self.gate_welcome_state != 0
                || self.gate_target_class != 0
                || self.gate_step != 0
                || !self.gate_ppd.is_empty()
            {
                write_ppd_block(&mut encoded, DRD_GATE_PPD, &self.encode_legacy_gate_ppd());
            }
        }
        if !had_pk && (existing_was_valid || existing.is_empty()) {
            if self.pk_kills != 0
                || self.pk_deaths != 0
                || self.pk_last_kill != 0
                || self.pk_last_death != 0
                || self.has_any_pk_hate()
            {
                write_ppd_block(&mut encoded, DRD_PK_PPD, &self.encode_legacy_pk_ppd());
            }
        }
        if !had_randchest && (existing_was_valid || existing.is_empty()) {
            if !self.random_chests.is_empty() {
                write_ppd_block(
                    &mut encoded,
                    DRD_RANDCHEST_PPD,
                    &self.encode_legacy_randchest_ppd(),
                );
            }
        }
        if !had_ratchest && (existing_was_valid || existing.is_empty()) {
            if !self.rat_chests.is_empty()
                || self.rat_chest_treasure_x != 0
                || self.rat_chest_treasure_y != 0
                || self.rat_chest_last_treasure_seconds != 0
            {
                write_ppd_block(
                    &mut encoded,
                    DRD_RATCHEST_PPD,
                    &self.encode_legacy_ratchest_ppd(),
                );
            }
        }
        if !had_demonshrine && (existing_was_valid || existing.is_empty()) {
            if !self.demonshrines.is_empty() {
                write_ppd_block(
                    &mut encoded,
                    DRD_DEMONSHRINE_PPD,
                    &self.encode_legacy_demonshrine_ppd(),
                );
            }
        }
        if !had_randomshrine && (existing_was_valid || existing.is_empty()) {
            if self.random_shrine_used_words.iter().any(|word| *word != 0)
                || self.random_shrine_continuity != 0
            {
                write_ppd_block(
                    &mut encoded,
                    DRD_RANDOMSHRINE_PPD,
                    &self.encode_legacy_randomshrine_ppd(),
                );
            }
        }
        if !had_orbspawn && (existing_was_valid || existing.is_empty()) {
            if !self.orb_spawns.is_empty() {
                write_ppd_block(
                    &mut encoded,
                    DRD_ORBSPAWN_PPD,
                    &self.encode_legacy_orbspawn_ppd(),
                );
            }
        }
        if !had_lostcon
            && (existing_was_valid || existing.is_empty())
            && (self.max_lag_seconds != 0 || self.hints_disabled || self.autoturn_enabled)
        {
            write_ppd_block(
                &mut encoded,
                DRD_LOSTCON_PPD,
                &self.encode_legacy_lostcon_ppd(),
            );
        }
        if !had_flower && (existing_was_valid || existing.is_empty()) && !self.flowers.is_empty() {
            write_ppd_block(
                &mut encoded,
                DRD_FLOWER_PPD,
                &self.encode_legacy_flower_ppd(),
            );
        }
        if !had_area3 && (existing_was_valid || existing.is_empty()) && !self.area3_ppd.is_empty() {
            write_ppd_block(&mut encoded, DRD_AREA3_PPD, &self.encode_legacy_area3_ppd());
        }
        if !had_area1 && (existing_was_valid || existing.is_empty()) && !self.area1_ppd.is_empty() {
            write_ppd_block(&mut encoded, DRD_AREA1_PPD, &self.encode_legacy_area1_ppd());
        }
        if !had_nomad && (existing_was_valid || existing.is_empty()) && !self.nomad_ppd.is_empty() {
            write_ppd_block(&mut encoded, DRD_NOMAD_PPD, &self.encode_legacy_nomad_ppd());
        }
        if !had_caligar
            && (existing_was_valid || existing.is_empty())
            && !self.caligar_ppd.is_empty()
        {
            write_ppd_block(
                &mut encoded,
                DRD_CALIGAR_PPD,
                &self.encode_legacy_caligar_ppd(),
            );
        }
        if !had_arkhata
            && (existing_was_valid || existing.is_empty())
            && !self.arkhata_ppd.is_empty()
        {
            write_ppd_block(
                &mut encoded,
                DRD_ARKHATA_PPD,
                &self.encode_legacy_arkhata_ppd(),
            );
        }
        if !had_staffer
            && (existing_was_valid || existing.is_empty())
            && !self.staffer_ppd.is_empty()
        {
            write_ppd_block(
                &mut encoded,
                DRD_STAFFER_PPD,
                &self.encode_legacy_staffer_ppd(),
            );
        }
        if !had_farmy && (existing_was_valid || existing.is_empty()) && !self.farmy_ppd.is_empty() {
            write_ppd_block(&mut encoded, DRD_FARMY_PPD, &self.encode_legacy_farmy_ppd());
        }
        if !had_teufelrat && (existing_was_valid || existing.is_empty()) {
            if self.teufel_rat_kills != 0 || self.teufel_rat_score != 0 {
                write_ppd_block(
                    &mut encoded,
                    DRD_TEUFELRAT_PPD,
                    &self.encode_legacy_teufelrat_ppd(),
                );
            }
        }
        if !had_bank && (existing_was_valid || existing.is_empty()) && self.bank_gold != 0 {
            write_ppd_block(&mut encoded, DRD_BANK_PPD, &self.encode_legacy_bank_ppd());
        }
        if !had_twocity && (existing_was_valid || existing.is_empty()) {
            if !self.twocity_ppd.is_empty()
                || self.twocity_goodtile.iter().any(|color| *color != 0)
                || self.twocity_solved_library
            {
                write_ppd_block(
                    &mut encoded,
                    DRD_TWOCITY_PPD,
                    &self.encode_legacy_twocity_ppd(),
                );
            }
        }
        if !had_saltmine && (existing_was_valid || existing.is_empty()) {
            if self
                .saltmine_ladder_last_seconds
                .iter()
                .any(|seconds| *seconds != 0)
                || self.saltmine_pending_salt != 0
            {
                write_ppd_block(
                    &mut encoded,
                    DRD_SALTMINE_PPD,
                    &self.encode_legacy_saltmine_ppd(),
                );
            }
        }
        if !had_treasure_dig && (existing_was_valid || existing.is_empty()) {
            if self
                .treasure_dig_last_seconds
                .iter()
                .any(|seconds| *seconds != 0)
            {
                write_ppd_block(
                    &mut encoded,
                    DRD_TREASURE_DIG_PPD,
                    &self.encode_legacy_treasure_dig_ppd(),
                );
            }
        }
        if !had_misc && (existing_was_valid || existing.is_empty()) && !self.misc_ppd.is_empty() {
            write_ppd_block(&mut encoded, DRD_MISC_PPD, &self.encode_legacy_misc_ppd());
        }
        if !had_rune && (existing_was_valid || existing.is_empty()) {
            if self.rune_used_words.iter().any(|word| *word != 0)
                || self.rune_special_exec.iter().any(|value| *value != 0)
            {
                write_ppd_block(&mut encoded, DRD_RUNE_PPD, &self.encode_legacy_rune_ppd());
            }
        }
        if !had_alias && (existing_was_valid || existing.is_empty()) && !self.aliases.is_empty() {
            write_ppd_block(&mut encoded, DRD_ALIAS_PPD, &self.encode_legacy_alias_ppd());
        }
        if !had_ignore
            && (existing_was_valid || existing.is_empty())
            && !self.ignored_characters.is_empty()
        {
            write_ppd_block(
                &mut encoded,
                DRD_IGNORE_PPD,
                &self.encode_legacy_ignore_ppd(),
            );
        }
        if !had_swear
            && (existing_was_valid || existing.is_empty())
            && self.shutup_until_seconds != 0
        {
            write_ppd_block(&mut encoded, DRD_SWEAR_PPD, &self.encode_legacy_swear_ppd());
        }

        encoded
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

    pub fn memorize_park_shrine(&mut self, shrine: u8) -> Option<bool> {
        let offset = match shrine {
            1 => AREA3_PPD_KELLY_FOUND1_OFFSET,
            2 => AREA3_PPD_KELLY_FOUND2_OFFSET,
            3 => AREA3_PPD_KELLY_FOUND3_OFFSET,
            _ => return None,
        };
        if self.area3_ppd.len() < LEGACY_AREA3_PPD_SIZE {
            self.area3_ppd.resize(LEGACY_AREA3_PPD_SIZE, 0);
        }
        let was_new = read_i32(&self.area3_ppd, offset) == 0;
        write_i32(&mut self.area3_ppd, offset, 1);
        Some(was_new)
    }

    pub fn observe_caligar_training(&mut self, lesson: u8) -> Option<bool> {
        let bit = match lesson {
            1 => 1,
            2 => 4,
            3 => 2,
            _ => return None,
        };
        if self.caligar_ppd.len() < LEGACY_CALIGAR_PPD_SIZE {
            self.caligar_ppd.resize(LEGACY_CALIGAR_PPD_SIZE, 0);
        }
        let watch_flag = read_i32(&self.caligar_ppd, CALIGAR_PPD_WATCH_FLAG_OFFSET);
        let was_new = watch_flag & bit == 0;
        write_i32(
            &mut self.caligar_ppd,
            CALIGAR_PPD_WATCH_FLAG_OFFSET,
            watch_flag | bit,
        );
        Some(was_new)
    }

    pub fn caligar_skelly_door_unlocked(&self, door_index: u8) -> bool {
        let idx = usize::from(door_index);
        idx < CALIGAR_PPD_DOOR_FLAG_COUNT
            && self.caligar_ppd.len() >= LEGACY_CALIGAR_PPD_SIZE
            && self.caligar_ppd[CALIGAR_PPD_DOOR_FLAG_OFFSET + idx] & 0x07 == 0x07
    }

    pub fn mark_caligar_skelly_death(
        &mut self,
        home_x: u16,
        home_y: u16,
    ) -> CaligarSkellyDeathResult {
        let (door_index, lock_number) = match (home_x, home_y) {
            (103, 224) => (0, 0),
            (103, 211) => (0, 1),
            (103, 198) => (0, 2),
            (145, 225) => (1, 0),
            (145, 212) => (1, 1),
            (145, 186) => (1, 2),
            (226 | 227, 158) => (2, 0),
            (226 | 227, 145) => (2, 1),
            (226 | 227, 132) => (2, 2),
            _ => {
                return CaligarSkellyDeathResult::Unmapped {
                    x: home_x,
                    y: home_y,
                };
            }
        };

        if self.caligar_ppd.len() < LEGACY_CALIGAR_PPD_SIZE {
            self.caligar_ppd.resize(LEGACY_CALIGAR_PPD_SIZE, 0);
        }

        let bit = 1u8 << lock_number;
        let offset = CALIGAR_PPD_DOOR_FLAG_OFFSET + door_index;
        if self.caligar_ppd[offset] & bit != 0 {
            return CaligarSkellyDeathResult::AlreadyUnlocked {
                door_index: door_index as u8,
                bit,
            };
        }

        self.caligar_ppd[offset] |= bit;
        if self.caligar_ppd[offset] & 0x07 == 0x07 {
            CaligarSkellyDeathResult::FullyUnlocked {
                door_index: door_index as u8,
                bit,
            }
        } else {
            CaligarSkellyDeathResult::PartiallyUnlocked {
                door_index: door_index as u8,
                bit,
            }
        }
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

    pub fn add_keyring_key(
        &mut self,
        template_id: u32,
        name: impl Into<String>,
    ) -> KeyringAddResult {
        self.add_keyring_entry(KeyringEntry {
            template_id,
            name: name.into(),
            description: String::new(),
            sprite: 0,
            flags: 0,
            value: 0,
            driver: 0,
            driver_data: Vec::new(),
            expire_serial: 0,
        })
    }

    pub fn touch_demonshrine(
        &mut self,
        character: &mut Character,
        location_id: u32,
    ) -> DemonShrineResult {
        if self.demonshrines.iter().any(|&id| id == location_id) {
            return DemonShrineResult::AlreadyKnown;
        }
        if self.demonshrines.len() >= DEMONSHRINE_MAX_ENTRIES {
            return DemonShrineResult::Full;
        }

        self.demonshrines.push(location_id);
        let demon_index = CharacterValue::Demon as usize;
        let demon_value = character
            .values
            .get_mut(1)
            .and_then(|values| values.get_mut(demon_index));
        let new_demon = if let Some(value) = demon_value {
            *value = value.saturating_add(1);
            u32::from((*value).max(0) as u16)
        } else {
            0
        };
        let exp_added =
            (250_u32.saturating_add(new_demon.saturating_mul(100))).min(character.exp / 25);
        // C `demonshrine_driver` (`base.c:3231-3235`) also calls
        // `update_char(cn)` (Demon value changed) and `give_exp(cn, ...)`
        // after this point; this function only has `&mut Character`
        // (`PlayerData` is not `World`), so both are applied by the caller
        // (`World::give_exp`/`World::update_character`) using the returned
        // `exp_added`, matching the `ItemDriverOutcome::LollipopLicked`
        // pattern in `world/item_outcomes.rs`.
        character.flags.insert(CharacterFlags::ITEMS);
        DemonShrineResult::Learned { exp_added }
    }

    pub fn add_keyring_item(&mut self, item: &Item) -> KeyringAddResult {
        let driver_data_len = item.driver_data.len().min(KEYRING_KEY_DRDATA_LEN);
        self.add_keyring_entry(KeyringEntry {
            template_id: item.template_id,
            name: item.name.clone(),
            description: item.description.clone(),
            sprite: item.sprite,
            flags: item.flags.bits(),
            value: item.value,
            driver: item.driver,
            driver_data: item.driver_data[..driver_data_len].to_vec(),
            expire_serial: item.serial,
        })
    }

    pub fn add_keyring_entry(&mut self, entry: KeyringEntry) -> KeyringAddResult {
        if self
            .keyring
            .iter()
            .any(|key| key.template_id == entry.template_id)
        {
            return KeyringAddResult::Duplicate;
        }
        if self.keyring.len() >= KEYRING_MAX_KEYS {
            return KeyringAddResult::Full;
        }
        self.keyring.push(entry);
        KeyringAddResult::Added
    }

    pub fn keyring_auto_add(&self) -> bool {
        self.keyring_auto_add
    }

    pub fn set_keyring_auto_add(&mut self, enabled: bool) {
        self.keyring_auto_add = enabled;
    }

    pub fn keyring_key_name(&self, template_id: u32) -> Option<&str> {
        self.keyring
            .iter()
            .find(|key| key.template_id == template_id)
            .map(|key| key.name.as_str())
    }

    pub fn remove_keyring_key_at(&mut self, index: usize) -> Option<KeyringEntry> {
        if index >= self.keyring.len() {
            return None;
        }
        Some(self.keyring.remove(index))
    }

    pub fn keyring_display_lines(&self) -> Vec<String> {
        if self.keyring.is_empty() {
            return vec!["Your keyring is empty.".to_string()];
        }

        let mut lines = Vec::with_capacity(self.keyring.len() + 3);
        lines.push(format!(
            "=== Keyring ({}/{KEYRING_MAX_KEYS} keys) ===",
            self.keyring.len()
        ));
        for (index, key) in self.keyring.iter().enumerate() {
            if key.name.is_empty() {
                lines.push(format!(
                    " {}. Unknown Key (ID: {})",
                    index + 1,
                    key.template_id
                ));
            } else {
                lines.push(format!(" {}. {}", index + 1, key.name));
            }
        }
        lines.push("Use a key on the keyring to add it.".to_string());
        lines.push("Type '#keyring remove <number>' to remove a key.".to_string());
        lines.push("Type '#keyring addall' to add all keys from inventory.".to_string());
        lines
    }

    pub fn record_chest_opened(&mut self, treasure_index: u8) {
        self.achievements.chests_opened = self.achievements.chests_opened.saturating_add(1);
        if self.achievements.chests_opened >= 10 {
            self.achievements.looter = true;
        }
        if self.achievements.chests_opened >= 50 {
            self.achievements.treasure_hunter = true;
        }
        if self.achievements.chests_opened >= 100 {
            self.achievements.treasure_master = true;
        }
        if self.achievements.chests_opened >= 500 {
            self.achievements.legendary_looter = true;
        }
        if treasure_index == 63 {
            self.achievements.gold_looter = true;
        }
    }

    pub fn random_chest_last_used_seconds(&self, location_id: u32) -> Option<u64> {
        self.random_chests
            .iter()
            .find(|entry| entry.location_id == location_id)
            .map(|entry| entry.last_used_seconds)
    }

    pub fn mark_random_chest_used(&mut self, location_id: u32, realtime_seconds: u64) {
        if let Some(entry) = self
            .random_chests
            .iter_mut()
            .find(|entry| entry.location_id == location_id)
        {
            entry.last_used_seconds = realtime_seconds;
            return;
        }
        if self.random_chests.len() < RANDCHEST_MAX_ENTRIES {
            self.random_chests.push(RandomChestAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            });
            return;
        }
        if let Some(oldest) = self
            .random_chests
            .iter_mut()
            .min_by_key(|entry| entry.last_used_seconds)
        {
            *oldest = RandomChestAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            };
        }
    }

    pub fn rat_chest_last_used_seconds(&self, location_id: u32) -> Option<u64> {
        self.rat_chests
            .iter()
            .find(|entry| entry.location_id == location_id)
            .map(|entry| entry.last_used_seconds)
    }

    pub fn mark_rat_chest_used(&mut self, location_id: u32, realtime_seconds: u64) {
        if let Some(entry) = self
            .rat_chests
            .iter_mut()
            .find(|entry| entry.location_id == location_id)
        {
            entry.last_used_seconds = realtime_seconds;
            return;
        }
        if self.rat_chests.len() < RATCHEST_MAX_ENTRIES {
            self.rat_chests.push(RatChestAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            });
            return;
        }
        if let Some(oldest) = self
            .rat_chests
            .iter_mut()
            .min_by_key(|entry| entry.last_used_seconds)
        {
            *oldest = RatChestAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            };
        }
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

    pub fn mark_login_parsed(&mut self, client_version: Option<u8>, current_tick: u64) {
        self.client_version = client_version.unwrap_or_default();
        self.view_distance = if self.client_version >= 3 {
            40
        } else {
            DIST_OLD
        };
        self.login_tick = current_tick;
    }

    /// Reattaches a `CDR_LOSTCON`-lingering player's runtime state to a new
    /// reconnecting session. C's reclaim (`tick_login`/`read_login`,
    /// `src/system/database/database_character.c:1164`,
    /// `src/system/player.c:493`) keeps the character's PPD-backed data
    /// (`ppd_blob`, keyring, chest access history, achievements, etc.)
    /// untouched across the reconnect and only resets the socket-session
    /// bookkeeping, mirroring `PlayerRuntime::connected`'s transient fields.
    pub fn reclaim_for_session(mut self, session_id: u64, current_tick: u64) -> Self {
        self.session_id = session_id;
        self.state = PlayerConnectionState::Connect;
        self.client_version = 0;
        self.view_distance = DIST_OLD;
        self.last_command_tick = current_tick;
        self.command.clear();
        self.action = QueuedAction::default();
        self.queue.clear();
        self.client_ticker = 0;
        self.next_fightback_character = None;
        self.next_fightback_serial = 0;
        self.next_fightback_tick = 0;
        self.nofight_timer = 0;
        self.login_tick = current_tick;
        self.deferred_init = 0;
        self.scrollback.clear();
        self
    }

    pub fn set_pending_action(&mut self, action: QueuedAction) {
        self.action = action;
    }

    pub fn push_queued_action(&mut self, action: QueuedAction) {
        if self.queue.len() == COMMAND_QUEUE_SIZE {
            self.queue.pop_front();
        }
        self.queue.push_back(action);
    }

    pub fn driver_stop(&mut self, current_tick: u64, nofight: bool) {
        self.queue.clear();
        self.action = QueuedAction::default();
        self.next_fightback_character = None;
        self.next_fightback_serial = 0;
        self.next_fightback_tick = 0;
        if nofight {
            self.nofight_timer = current_tick;
        }
    }

    pub fn driver_halt(&mut self) {
        self.action = QueuedAction::default();
        self.next_fightback_character = None;
        self.next_fightback_serial = 0;
        self.next_fightback_tick = 0;
    }

    pub fn driver_move(&mut self, x: i32, y: i32) {
        self.action = QueuedAction {
            action: PlayerActionCode::Move,
            arg1: x,
            arg2: y,
        };
    }

    pub fn driver_take(&mut self, item: i32, serial: u32) {
        self.action = QueuedAction {
            action: PlayerActionCode::Take,
            arg1: item,
            arg2: serial as i32,
        };
    }

    pub fn driver_drop(&mut self, x: i32, y: i32) {
        self.action = QueuedAction {
            action: PlayerActionCode::Drop,
            arg1: x,
            arg2: y,
        };
    }

    pub fn driver_use(&mut self, item: i32, serial: u32) {
        self.action = QueuedAction {
            action: PlayerActionCode::Use,
            arg1: item,
            arg2: serial as i32,
        };
    }

    pub fn driver_teleport(&mut self, teleport: i32) {
        self.action = QueuedAction {
            action: PlayerActionCode::Teleport,
            arg1: teleport,
            arg2: 0,
        };
    }

    pub fn driver_kill(&mut self, character: CharacterId, serial: u32) {
        self.action = QueuedAction {
            action: PlayerActionCode::Kill,
            arg1: character.0 as i32,
            arg2: serial as i32,
        };
    }

    pub fn driver_give(&mut self, character: CharacterId, serial: u32) {
        self.action = QueuedAction {
            action: PlayerActionCode::Give,
            arg1: character.0 as i32,
            arg2: serial as i32,
        };
    }

    pub fn driver_charspell(
        &mut self,
        spell: PlayerActionCode,
        character: CharacterId,
        serial: u32,
    ) {
        self.insert_driver_queue(QueuedAction {
            action: spell,
            arg1: character.0 as i32,
            arg2: serial as i32,
        });
    }

    pub fn driver_mapspell(&mut self, spell: PlayerActionCode, x: i32, y: i32) {
        self.insert_driver_queue(QueuedAction {
            action: spell,
            arg1: x,
            arg2: y,
        });
    }

    pub fn driver_selfspell(&mut self, spell: PlayerActionCode) {
        self.insert_driver_queue(QueuedAction {
            action: spell,
            arg1: 0,
            arg2: 0,
        });
    }

    pub fn apply_got_hit_fightback(
        &mut self,
        attacker: CharacterId,
        attacker_serial: u32,
        legacy_distance: i32,
        current_tick: u64,
    ) -> bool {
        if attacker.0 == 0
            || legacy_distance >= 3
            || current_tick.saturating_sub(self.nofight_timer) <= TICKS_PER_SECOND * 3
        {
            return false;
        }

        match self.action.action {
            PlayerActionCode::Idle => {
                self.driver_kill(attacker, attacker_serial);
                true
            }
            PlayerActionCode::Kill => false,
            _ => {
                self.next_fightback_character = Some(attacker);
                self.next_fightback_serial = attacker_serial;
                self.next_fightback_tick = current_tick;
                true
            }
        }
    }

    pub fn apply_deferred_fightback(&mut self, current_tick: u64) -> bool {
        if self.action.action != PlayerActionCode::Idle
            || current_tick.saturating_sub(self.next_fightback_tick) >= TICKS_PER_SECOND
            || current_tick.saturating_sub(self.nofight_timer) <= TICKS_PER_SECOND * 3
        {
            return false;
        }
        let Some(attacker) = self.next_fightback_character else {
            return false;
        };

        self.driver_kill(attacker, self.next_fightback_serial);
        true
    }

    fn insert_driver_queue(&mut self, action: QueuedAction) {
        if self.queue.len() == COMMAND_QUEUE_SIZE {
            if let Some(back) = self.queue.back_mut() {
                *back = action;
            }
        } else {
            self.queue.push_back(action);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LegacyPpdBlock<'a> {
    id: u32,
    data: &'a [u8],
}

struct LegacyPpdBlocks<'a> {
    bytes: &'a [u8],
    offset: usize,
    failed: bool,
}

impl<'a> LegacyPpdBlocks<'a> {
    fn parse(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            offset: 0,
            failed: false,
        }
    }
}

impl<'a> Iterator for LegacyPpdBlocks<'a> {
    type Item = Option<LegacyPpdBlock<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed || self.offset == self.bytes.len() {
            return None;
        }
        if self.bytes.len().saturating_sub(self.offset) < 8 {
            self.failed = true;
            return Some(None);
        }

        let id = read_u32(self.bytes, self.offset);
        let size = read_u32(self.bytes, self.offset + 4) as usize;
        self.offset += 8;
        if self.bytes.len().saturating_sub(self.offset) < size {
            self.failed = true;
            return Some(None);
        }

        let data = &self.bytes[self.offset..self.offset + size];
        self.offset += size;
        Some(Some(LegacyPpdBlock { id, data }))
    }
}

fn write_ppd_block(bytes: &mut Vec<u8>, id: u32, data: &[u8]) {
    bytes.extend_from_slice(&id.to_le_bytes());
    bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(data);
}

/// `del_data`-style block removal for DRD ids that have no dedicated typed
/// field on `PlayerRuntime` (so there's nothing to reset in memory - the
/// only representation of that data is the raw bytes carried in
/// `ppd_blob`). Parses `bytes` into blocks and re-emits every block whose
/// id is not in `remove_ids`, preserving order; stops re-emitting (like
/// `encode_legacy_ppd_blob`'s own `existing_was_valid` handling) once
/// parsing hits malformed trailing bytes, since nothing past that point is
/// safely recoverable anyway.
fn strip_ppd_blocks(bytes: &[u8], remove_ids: &[u32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    for block in LegacyPpdBlocks::parse(bytes) {
        let Some(block) = block else {
            break;
        };
        if remove_ids.contains(&block.id) {
            continue;
        }
        write_ppd_block(&mut out, block.id, block.data);
    }
    out
}

fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn write_c_string(bytes: &mut [u8], offset: usize, len: usize, value: &str) {
    let max_len = len.saturating_sub(1);
    let value_bytes = value.as_bytes();
    let copy_len = value_bytes.len().min(max_len);
    bytes[offset..offset + copy_len].copy_from_slice(&value_bytes[..copy_len]);
}

fn read_i32(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}

fn read_c_string(bytes: &[u8], offset: usize, len: usize) -> String {
    let raw = &bytes[offset..offset + len];
    let end = raw.iter().position(|byte| *byte == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use crate::{
        entity::{Character, CharacterFlags, ItemFlags, MAX_MODIFIERS},
        ids::ItemId,
    };

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
        write_ppd_block(&mut existing, DRD_FIRSTKILL_PPD, &[1, 2, 3, 4]);
        write_ppd_block(&mut existing, DRD_DEPOT_PPD, &[5, 6]);
        write_ppd_block(&mut existing, unrelated_unknown_id, &[7, 8, 9]);

        let mut player = PlayerRuntime::connected(1, 0);
        player.ppd_blob = existing;

        player.clear_turn_seyan_ppd();

        let blocks: Vec<_> = LegacyPpdBlocks::parse(&player.ppd_blob)
            .map(|block| block.unwrap())
            .collect();
        // `DRD_FIRSTKILL_PPD` is one of `turn_seyan`'s del_data targets and
        // is gone; `DRD_DEPOT_PPD` (the documented gap) and any other
        // still-unrelated id round-trip untouched.
        assert!(!blocks.iter().any(|block| block.id == DRD_FIRSTKILL_PPD));
        assert!(blocks.iter().any(|block| block.id == DRD_DEPOT_PPD));
        assert!(blocks
            .iter()
            .any(|block| block.id == unrelated_unknown_id && block.data == [7, 8, 9]));
    }

    #[test]
    fn staffer_ppd_marks_animation_book_once() {
        let mut player = PlayerRuntime::connected(1, 0);

        assert!(player.mark_staffer_animation_book_seen());
        assert!(!player.mark_staffer_animation_book_seen());
        assert_eq!(
            read_i32(&player.staffer_ppd, STAFFER_PPD_SHANRA_STATE_OFFSET),
            3
        );

        let encoded = player.encode_legacy_staffer_ppd();
        assert_eq!(encoded.len(), LEGACY_STAFFER_PPD_SIZE);
        assert_eq!(read_i32(&encoded, STAFFER_PPD_SHANRA_STATE_OFFSET), 3);
    }

    #[test]
    fn staffer_ppd_round_trips_through_outer_blob() {
        let mut staffer = vec![0; LEGACY_STAFFER_PPD_SIZE];
        write_i32(&mut staffer, STAFFER_PPD_SHANRA_STATE_OFFSET, 2);
        let mut blob = Vec::new();
        write_ppd_block(&mut blob, DRD_STAFFER_PPD, &staffer);

        let mut player = PlayerRuntime::connected(1, 0);
        assert!(player.decode_legacy_ppd_blob(&blob));
        assert_eq!(
            read_i32(&player.staffer_ppd, STAFFER_PPD_SHANRA_STATE_OFFSET),
            2
        );

        assert!(player.mark_staffer_animation_book_seen());
        let encoded = player.encode_legacy_ppd_blob(&blob);
        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(
            read_i32(&decoded.staffer_ppd, STAFFER_PPD_SHANRA_STATE_OFFSET),
            3
        );
    }

    #[test]
    fn farmy_ppd_advances_blood_and_lava_quest_stages() {
        let mut player = PlayerRuntime::connected(1, 0);
        assert_eq!(player.farmy_boss_stage(), 0);
        assert!(!player.advance_farmy_blood_stage());

        player.farmy_ppd.resize(LEGACY_FARMY_PPD_SIZE, 0);
        write_i32(&mut player.farmy_ppd, FARMY_PPD_BOSS_STAGE_OFFSET, 20);
        assert!(player.advance_farmy_blood_stage());
        assert_eq!(player.farmy_boss_stage(), 21);
        assert!(!player.advance_farmy_blood_stage());

        write_i32(&mut player.farmy_ppd, FARMY_PPD_BOSS_STAGE_OFFSET, 22);
        assert!(player.advance_farmy_lava_stage());
        assert_eq!(player.farmy_boss_stage(), 24);
        assert!(!player.advance_farmy_lava_stage());
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
    fn special_shrine_requires_confirmation_then_removes_hardcore() {
        let mut player = PlayerRuntime::connected(7, 11);
        let mut character = character(3);
        character.flags.insert(CharacterFlags::HARDCORE);
        character.creation_time = SPECIAL_SHRINE_HCSC_CUTOFF_SECONDS;

        assert_eq!(
            player.touch_special_shrine(&mut character, 0x0A, 100),
            SpecialShrineResult::ConfirmRequired,
        );
        assert!(character.flags.contains(CharacterFlags::HARDCORE));
        assert_eq!(
            player.touch_special_shrine(&mut character, 0x0A, 109),
            SpecialShrineResult::HardcoreRemoved,
        );
        assert!(!character.flags.contains(CharacterFlags::HARDCORE));
    }

    #[test]
    fn special_shrine_blocks_non_hardcore_and_new_hardcore() {
        let mut player = PlayerRuntime::connected(7, 11);
        let mut softcore = character(3);
        assert_eq!(
            player.touch_special_shrine(&mut softcore, 0x0A, 100),
            SpecialShrineResult::NothingHere,
        );

        let mut new_hardcore = character(4);
        new_hardcore.flags.insert(CharacterFlags::HARDCORE);
        new_hardcore.creation_time = SPECIAL_SHRINE_HCSC_CUTOFF_SECONDS + 1;
        assert_eq!(
            player.touch_special_shrine(&mut new_hardcore, 0x0A, 100),
            SpecialShrineResult::NothingHere,
        );
        assert!(new_hardcore.flags.contains(CharacterFlags::HARDCORE));
    }

    #[test]
    fn command_queue_keeps_legacy_capacity() {
        let mut player = PlayerRuntime::connected(1, 0);
        for n in 0..20 {
            player.push_queued_action(QueuedAction {
                action: PlayerActionCode::Move,
                arg1: n,
                arg2: 0,
            });
        }
        assert_eq!(player.queue.len(), COMMAND_QUEUE_SIZE);
        assert_eq!(player.queue.front().unwrap().arg1, 4);
    }

    #[test]
    fn keyring_tracks_legacy_key_ids_with_duplicate_and_capacity_rules() {
        let mut player = PlayerRuntime::connected(1, 0);

        assert_eq!(
            player.add_keyring_key(0x1122_3344, "Copper Key"),
            KeyringAddResult::Added
        );
        assert_eq!(player.keyring_key_name(0x1122_3344), Some("Copper Key"));
        assert_eq!(
            player.add_keyring_key(0x1122_3344, "Duplicate"),
            KeyringAddResult::Duplicate
        );

        for index in 1..KEYRING_MAX_KEYS {
            assert_eq!(
                player.add_keyring_key(index as u32, format!("Key {index}")),
                KeyringAddResult::Added
            );
        }
        assert_eq!(
            player.add_keyring_key(0x5566_7788, "Overflow"),
            KeyringAddResult::Full
        );
    }

    #[test]
    fn keyring_item_storage_keeps_legacy_recreation_metadata() {
        let mut player = PlayerRuntime::connected(1, 0);
        let item = Item {
            id: ItemId(7),
            name: "Copper Key".into(),
            description: "Opens a copper lock".into(),
            flags: ItemFlags::USED | ItemFlags::TAKE | ItemFlags::QUEST,
            sprite: 1234,
            value: 55,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0x1122_3344,
            owner_id: 0,
            modifier_index: [0; MAX_MODIFIERS],
            modifier_value: [0; MAX_MODIFIERS],
            x: 0,
            y: 0,
            carried_by: None,
            contained_in: None,
            content_id: 0,
            driver: 77,
            driver_data: (0..32).collect(),
            serial: 9,
        };

        assert_eq!(player.add_keyring_item(&item), KeyringAddResult::Added);

        let stored = &player.keyring[0];
        assert_eq!(stored.template_id, 0x1122_3344);
        assert_eq!(stored.name, "Copper Key");
        assert_eq!(stored.description, "Opens a copper lock");
        assert_eq!(stored.sprite, 1234);
        assert_eq!(stored.flags, item.flags.bits());
        assert_eq!(stored.value, 55);
        assert_eq!(stored.driver, 77);
        assert_eq!(stored.driver_data, (0..16).collect::<Vec<_>>());
        assert_eq!(stored.expire_serial, 9);
    }

    #[test]
    fn keyring_auto_add_setting_round_trips() {
        let mut player = PlayerRuntime::connected(1, 0);
        assert!(!player.keyring_auto_add());
        player.set_keyring_auto_add(true);
        assert!(player.keyring_auto_add());
    }

    #[test]
    fn keyring_ppd_codec_matches_legacy_c_layout() {
        assert_eq!(KEYRING_PPD_FLAGS_OFFSET % 8, 0);
        assert_eq!(KEYRING_PPD_AUTO_ADD_OFFSET + 4, LEGACY_KEYRING_PPD_SIZE);

        let mut player = PlayerRuntime::connected(1, 0);
        player.set_keyring_auto_add(true);
        assert_eq!(
            player.add_keyring_entry(KeyringEntry {
                template_id: 0x1122_3344,
                name: "A name that is deliberately longer than forty bytes".to_string(),
                description: "Opens a door and has a long legacy description".to_string(),
                sprite: -123,
                flags: 0x0102_0304_0506_0708,
                value: 99,
                driver: 77,
                driver_data: (0..32).collect(),
                expire_serial: 0x1234,
            }),
            KeyringAddResult::Added
        );

        let bytes = player.encode_legacy_keyring_ppd();
        assert_eq!(bytes.len(), LEGACY_KEYRING_PPD_SIZE);
        assert_eq!(read_i32(&bytes, KEYRING_PPD_COUNT_OFFSET), 1);
        assert_eq!(read_u32(&bytes, KEYRING_PPD_KEYS_OFFSET), 0x1122_3344);
        assert_eq!(
            bytes[KEYRING_PPD_NAMES_OFFSET + KEYRING_KEY_NAME_LEN - 1],
            0
        );
        assert_eq!(read_i32(&bytes, KEYRING_PPD_AUTO_ADD_OFFSET), 1);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_keyring_ppd(&bytes));
        assert!(decoded.keyring_auto_add());
        assert_eq!(decoded.keyring.len(), 1);
        assert_eq!(decoded.keyring[0].template_id, 0x1122_3344);
        assert_eq!(
            decoded.keyring[0].name,
            "A name that is deliberately longer than"
        );
        assert_eq!(decoded.keyring[0].sprite, -123);
        assert_eq!(decoded.keyring[0].flags, 0x0102_0304_0506_0708);
        assert_eq!(decoded.keyring[0].driver_data, (0..16).collect::<Vec<_>>());
        assert_eq!(decoded.keyring[0].expire_serial, 0x34);
    }

    #[test]
    fn treasure_chest_ppd_codec_matches_legacy_c_layout() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.mark_chest_access(0, 1234);
        player.mark_chest_access(63, 86_400);
        player.mark_chest_access(199, i32::MAX as u64 + 99);

        let bytes = player.encode_legacy_treasure_chest_ppd();
        assert_eq!(bytes.len(), LEGACY_TREASURE_CHEST_PPD_SIZE);
        assert_eq!(read_i32(&bytes, 0), 1234);
        assert_eq!(read_i32(&bytes, 63 * 4), 86_400);
        assert_eq!(read_i32(&bytes, 199 * 4), i32::MAX);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_treasure_chest_ppd(&bytes));
        assert_eq!(decoded.chest_last_access_seconds(0), 1234);
        assert_eq!(decoded.chest_last_access_seconds(63), 86_400);
        assert_eq!(decoded.chest_last_access_seconds(199), i32::MAX as u64);
        assert_eq!(decoded.chest_last_access_seconds(1), 0);
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
    fn randchest_ppd_codec_matches_legacy_c_layout() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.mark_random_chest_used(0x0001_0506, 1234);
        player.mark_random_chest_used(0x0001_0708, i32::MAX as u64 + 99);

        let bytes = player.encode_legacy_randchest_ppd();
        assert_eq!(bytes.len(), LEGACY_RANDCHEST_PPD_SIZE);
        assert_eq!(read_i32(&bytes, 0), 0x0001_0506);
        assert_eq!(read_i32(&bytes, 4), 0x0001_0708);
        assert_eq!(read_i32(&bytes, RANDCHEST_PPD_LAST_USED_OFFSET), 1234);
        assert_eq!(
            read_i32(&bytes, RANDCHEST_PPD_LAST_USED_OFFSET + 4),
            i32::MAX
        );

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_randchest_ppd(&bytes));
        assert_eq!(
            decoded.random_chest_last_used_seconds(0x0001_0506),
            Some(1234)
        );
        assert_eq!(
            decoded.random_chest_last_used_seconds(0x0001_0708),
            Some(i32::MAX as u64)
        );
        assert_eq!(decoded.random_chest_last_used_seconds(0x0001_090a), None);
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
    fn keyring_ppd_blob_round_trips_with_legacy_block_framing() {
        let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
        let mut existing = Vec::new();
        write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
        write_ppd_block(&mut existing, DRD_JUNK_PPD, &[9, 9, 9]);

        let mut player = PlayerRuntime::connected(1, 0);
        player.set_keyring_auto_add(true);
        assert_eq!(
            player.add_keyring_key(0x1122_3344, "Copper Key"),
            KeyringAddResult::Added
        );

        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), unknown_id);
        assert_eq!(read_u32(&encoded, 4), 4);
        assert_eq!(&encoded[8..12], &[1, 2, 3, 4]);
        assert_eq!(read_u32(&encoded, 12), DRD_KEYRING_PPD);
        assert_eq!(read_u32(&encoded, 16), LEGACY_KEYRING_PPD_SIZE as u32);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert!(decoded.keyring_auto_add());
        assert_eq!(decoded.keyring_key_name(0x1122_3344), Some("Copper Key"));
        assert!(!encoded
            .windows(4)
            .any(|window| window == DRD_JUNK_PPD.to_le_bytes()));
    }

    #[test]
    fn treasure_chest_ppd_blob_round_trips_with_legacy_block_framing() {
        let unknown_id = make_drd(DEV_ID_DB, 222 | PERSISTENT_PLAYER_DATA);
        let mut existing = Vec::new();
        write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
        write_ppd_block(
            &mut existing,
            DRD_TREASURE_CHEST_PPD,
            &[0; LEGACY_TREASURE_CHEST_PPD_SIZE],
        );

        let mut player = PlayerRuntime::connected(1, 0);
        player.mark_chest_access(17, 777);

        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), unknown_id);
        assert_eq!(read_u32(&encoded, 12), DRD_TREASURE_CHEST_PPD);
        assert_eq!(
            read_u32(&encoded, 16),
            LEGACY_TREASURE_CHEST_PPD_SIZE as u32
        );
        assert_eq!(read_i32(&encoded, 20 + 17 * 4), 777);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(decoded.chest_last_access_seconds(17), 777);
    }

    #[test]
    fn ppd_blob_appends_treasure_chests_without_existing_block() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.mark_chest_access(5, 55);

        let encoded = player.encode_legacy_ppd_blob(&[]);
        assert_eq!(read_u32(&encoded, 0), DRD_TREASURE_CHEST_PPD);
        assert_eq!(read_u32(&encoded, 4), LEGACY_TREASURE_CHEST_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 8 + 5 * 4), 55);
    }

    #[test]
    fn randchest_ppd_blob_round_trips_with_legacy_block_framing() {
        let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
        let mut existing_randchest = vec![0; LEGACY_RANDCHEST_PPD_SIZE];
        write_i32(
            &mut existing_randchest,
            RANDCHEST_PPD_IDS_OFFSET,
            0x0001_0203,
        );
        write_i32(&mut existing_randchest, RANDCHEST_PPD_LAST_USED_OFFSET, 44);

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
        write_ppd_block(&mut existing, DRD_RANDCHEST_PPD, &existing_randchest);

        let mut player = PlayerRuntime::connected(1, 0);
        player.mark_random_chest_used(0x0001_0506, 777);

        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), unknown_id);
        assert_eq!(read_u32(&encoded, 12), DRD_RANDCHEST_PPD);
        assert_eq!(read_u32(&encoded, 16), LEGACY_RANDCHEST_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 20), 0x0001_0506);
        assert_eq!(read_i32(&encoded, 20 + RANDCHEST_PPD_LAST_USED_OFFSET), 777);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(
            decoded.random_chest_last_used_seconds(0x0001_0506),
            Some(777)
        );
        assert_eq!(decoded.random_chest_last_used_seconds(0x0001_0203), None);
    }

    #[test]
    fn ratchest_ppd_codec_matches_legacy_c_layout() {
        assert_eq!(LEGACY_RATCHEST_PPD_SIZE, 812);

        let mut player = PlayerRuntime::connected(1, 0);
        player.mark_rat_chest_used(0x0007_0506, 1234);
        player.mark_rat_chest_used(0x0007_0708, i32::MAX as u64 + 99);
        player.rat_chest_treasure_x = 321;
        player.rat_chest_treasure_y = 654;
        player.rat_chest_last_treasure_seconds = 9876;

        let bytes = player.encode_legacy_ratchest_ppd();
        assert_eq!(bytes.len(), LEGACY_RATCHEST_PPD_SIZE);
        assert_eq!(read_i32(&bytes, 0), 0x0007_0506);
        assert_eq!(read_i32(&bytes, 4), 0x0007_0708);
        assert_eq!(read_i32(&bytes, RATCHEST_PPD_LAST_USED_OFFSET), 1234);
        assert_eq!(
            read_i32(&bytes, RATCHEST_PPD_LAST_USED_OFFSET + 4),
            i32::MAX
        );
        assert_eq!(read_i32(&bytes, RATCHEST_PPD_TREASURE_X_OFFSET), 321);
        assert_eq!(read_i32(&bytes, RATCHEST_PPD_TREASURE_Y_OFFSET), 654);
        assert_eq!(read_i32(&bytes, RATCHEST_PPD_LAST_TREASURE_OFFSET), 9876);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ratchest_ppd(&bytes));
        assert_eq!(decoded.rat_chest_last_used_seconds(0x0007_0506), Some(1234));
        assert_eq!(
            decoded.rat_chest_last_used_seconds(0x0007_0708),
            Some(i32::MAX as u64)
        );
        assert_eq!(decoded.rat_chest_last_used_seconds(0x0007_090a), None);
        assert_eq!(decoded.rat_chest_treasure_x, 321);
        assert_eq!(decoded.rat_chest_treasure_y, 654);
        assert_eq!(decoded.rat_chest_last_treasure_seconds, 9876);
    }

    #[test]
    fn ratchest_ppd_blob_round_trips_with_legacy_block_framing() {
        let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
        let mut existing_ratchest = vec![0; LEGACY_RATCHEST_PPD_SIZE];
        write_i32(&mut existing_ratchest, RATCHEST_PPD_IDS_OFFSET, 0x0007_0203);
        write_i32(&mut existing_ratchest, RATCHEST_PPD_LAST_USED_OFFSET, 44);

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
        write_ppd_block(&mut existing, DRD_RATCHEST_PPD, &existing_ratchest);

        let mut player = PlayerRuntime::connected(1, 0);
        player.mark_rat_chest_used(0x0007_0506, 777);
        player.rat_chest_treasure_x = 12;
        player.rat_chest_treasure_y = 34;
        player.rat_chest_last_treasure_seconds = 55;

        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), unknown_id);
        assert_eq!(read_u32(&encoded, 12), DRD_RATCHEST_PPD);
        assert_eq!(read_u32(&encoded, 16), LEGACY_RATCHEST_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 20), 0x0007_0506);
        assert_eq!(read_i32(&encoded, 20 + RATCHEST_PPD_LAST_USED_OFFSET), 777);
        assert_eq!(read_i32(&encoded, 20 + RATCHEST_PPD_TREASURE_X_OFFSET), 12);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(decoded.rat_chest_last_used_seconds(0x0007_0506), Some(777));
        assert_eq!(decoded.rat_chest_last_used_seconds(0x0007_0203), None);
        assert_eq!(decoded.rat_chest_treasure_y, 34);
        assert_eq!(decoded.rat_chest_last_treasure_seconds, 55);

        let mut appended = PlayerRuntime::connected(3, 0);
        appended.mark_rat_chest_used(0x0007_0203, 66);
        let appended_blob = appended.encode_legacy_ppd_blob(&[]);
        assert_eq!(read_u32(&appended_blob, 0), DRD_RATCHEST_PPD);
        assert_eq!(read_i32(&appended_blob, 8), 0x0007_0203);
    }

    #[test]
    fn ppd_blob_appends_randchests_without_existing_block() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.mark_random_chest_used(0x0001_0203, 55);

        let encoded = player.encode_legacy_ppd_blob(&[]);
        assert_eq!(read_u32(&encoded, 0), DRD_RANDCHEST_PPD);
        assert_eq!(read_u32(&encoded, 4), LEGACY_RANDCHEST_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 8), 0x0001_0203);
        assert_eq!(read_i32(&encoded, 8 + RANDCHEST_PPD_LAST_USED_OFFSET), 55);
    }

    #[test]
    fn transport_ppd_codec_matches_legacy_seen_mask_layout() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.transport_seen = 0x0102_0304_0506_0708;

        let encoded = player.encode_legacy_transport_ppd();
        assert_eq!(encoded, 0x0102_0304_0506_0708_u64.to_le_bytes());

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_transport_ppd(&encoded));
        assert_eq!(decoded.transport_seen, 0x0102_0304_0506_0708);
        assert!(!decoded.decode_legacy_transport_ppd(&encoded[..7]));
    }

    #[test]
    fn lab_ppd_codec_preserves_legacy_solved_bits_and_payload() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.lab_ppd = vec![0xaa; LEGACY_LAB_PPD_SIZE];
        player.lab_solved_bits = (1_u64 << 10) | (1_u64 << 25);

        let encoded = player.encode_legacy_lab_ppd();
        assert_eq!(encoded.len(), LEGACY_LAB_PPD_SIZE);
        assert_eq!(read_u64(&encoded, 0), (1_u64 << 10) | (1_u64 << 25));
        assert_eq!(encoded[8], 0xaa);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_lab_ppd(&encoded));
        assert_eq!(decoded.lab_solved_bits, (1_u64 << 10) | (1_u64 << 25));
        assert_eq!(decoded.lab_ppd, encoded);
        assert!(!decoded.decode_legacy_lab_ppd(&encoded[..7]));
    }

    #[test]
    fn lab2_described_graves_use_legacy_lab_ppd_offsets() {
        let mut player = PlayerRuntime::connected(1, 0);
        let indices = player.ensure_legacy_lab2_described_graves_with_indices([2, 6, 10, 11]);

        assert_eq!(indices, [2, 6, 10, 11]);
        assert_eq!(player.lab_ppd.len(), LEGACY_LAB_PPD_SIZE);
        assert_eq!(player.lab_ppd[LEGACY_LAB2_GRAVEVERSION_OFFSET], 2);
        assert_eq!(player.legacy_lab2_grave_indices(), [2, 6, 10, 11]);

        let preserved = player.ensure_legacy_lab2_described_graves_with_indices([0, 4, 8, 9]);
        assert_eq!(preserved, [2, 6, 10, 11]);
    }

    #[test]
    fn lab2_grave_clue_text_uses_legacy_described_grave_table() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.ensure_legacy_lab2_described_graves_with_indices([0, 4, 8, 9]);

        assert_eq!(
            player.legacy_lab2_grave_clue_text(1).as_deref(),
            Some("Henry is buried in the third grave behind the chapel.")
        );
        assert_eq!(
            player.legacy_lab2_grave_clue_text(3).as_deref(),
            Some("For his generosity John is buried in the first grave of the second row next to the southeastern chapel aisle.")
        );
        assert_eq!(player.legacy_lab2_grave_clue_text(5), None);
    }

    #[test]
    fn lab2_special_grave_kind_matches_player_specific_coordinates() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.ensure_legacy_lab2_described_graves_with_indices([0, 4, 8, 9]);

        assert_eq!(player.legacy_lab2_special_grave_kind_at(194, 183), Some(1));
        assert_eq!(player.legacy_lab2_special_grave_kind_at(199, 195), Some(3));
        assert_eq!(player.legacy_lab2_special_grave_kind_at(212, 191), None);
    }

    #[test]
    fn lab2_grave_bitset_uses_legacy_one_bit_per_grave_layout() {
        let mut player = PlayerRuntime::connected(1, 0);

        assert!(!player.legacy_lab2_grave_cleared(9));
        assert!(player.mark_legacy_lab2_grave_cleared(9));
        assert!(!player.mark_legacy_lab2_grave_cleared(9));
        assert!(player.legacy_lab2_grave_cleared(9));
        assert_eq!(player.lab2_grave_bits[1], 0b0000_0010);
        assert!(!player.mark_legacy_lab2_grave_cleared(LAB2_GRAVE_BITSET_BYTES * 8));
    }

    #[test]
    fn lab_ppd_blob_round_trips_with_legacy_block_framing() {
        let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
        let mut existing_lab = vec![0; LEGACY_LAB_PPD_SIZE];
        write_u64(&mut existing_lab, 0, 1_u64 << 10);

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
        write_ppd_block(&mut existing, DRD_LAB_PPD, &existing_lab);

        let mut player = PlayerRuntime::connected(1, 0);
        player.lab_solved_bits = (1_u64 << 15) | (1_u64 << 20);

        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), unknown_id);
        assert_eq!(read_u32(&encoded, 12), DRD_LAB_PPD);
        assert_eq!(read_u32(&encoded, 16), LEGACY_LAB_PPD_SIZE as u32);
        assert_eq!(read_u64(&encoded, 20), (1_u64 << 15) | (1_u64 << 20));

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(decoded.lab_solved_bits, (1_u64 << 15) | (1_u64 << 20));
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
    fn lostcon_ppd_codec_matches_legacy_c_layout() {
        assert_eq!(LOSTCON_PPD_HINTS_OFFSET + 4, LEGACY_LOSTCON_PPD_SIZE);

        let mut player = PlayerRuntime::connected(1, 0);
        player.set_max_lag_seconds(17);
        player.hints_disabled = true;
        player.autoturn_enabled = true;

        let encoded = player.encode_legacy_lostcon_ppd();
        assert_eq!(encoded.len(), LEGACY_LOSTCON_PPD_SIZE);
        assert_eq!(read_i32(&encoded, 0), 0);
        assert_eq!(read_i32(&encoded, LOSTCON_PPD_AUTOTURN_OFFSET), 1);
        assert_eq!(read_i32(&encoded, LOSTCON_PPD_MAXLAG_OFFSET), 17);
        assert_eq!(read_i32(&encoded, LOSTCON_PPD_HINTS_OFFSET), 1);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_lostcon_ppd(&encoded));
        assert_eq!(decoded.max_lag_seconds, 17);
        assert!(decoded.hints_disabled);
        assert!(decoded.autoturn_enabled);
        assert!(!decoded.decode_legacy_lostcon_ppd(&encoded[..LEGACY_LOSTCON_PPD_SIZE - 1]));
    }

    #[test]
    fn pk_ppd_codec_matches_legacy_c_layout() {
        assert_eq!(
            PK_PPD_HATE_OFFSET + PK_HATE_MAX_ENTRIES * 4,
            LEGACY_PK_PPD_SIZE
        );

        let mut player = PlayerRuntime::connected(1, 0);
        player.pk_kills = 3;
        player.pk_deaths = 4;
        player.pk_last_kill = 0x1122_3344;
        player.pk_last_death = i32::MAX as u32 + 99;
        assert!(player.add_pk_hate(1001));
        assert!(player.add_pk_hate(1002));
        assert!(!player.add_pk_hate(1002));

        let encoded = player.encode_legacy_pk_ppd();
        assert_eq!(encoded.len(), LEGACY_PK_PPD_SIZE);
        assert_eq!(read_i32(&encoded, PK_PPD_KILLS_OFFSET), 3);
        assert_eq!(read_i32(&encoded, PK_PPD_DEATHS_OFFSET), 4);
        assert_eq!(read_i32(&encoded, PK_PPD_LAST_KILL_OFFSET), 0x1122_3344);
        assert_eq!(read_i32(&encoded, PK_PPD_LAST_DEATH_OFFSET), i32::MAX);
        assert_eq!(read_i32(&encoded, PK_PPD_HATE_OFFSET), 1002);
        assert_eq!(read_i32(&encoded, PK_PPD_HATE_OFFSET + 4), 1001);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_pk_ppd(&encoded));
        assert_eq!(decoded.pk_kills, 3);
        assert_eq!(decoded.pk_deaths, 4);
        assert_eq!(decoded.pk_last_kill, 0x1122_3344);
        assert_eq!(decoded.pk_last_death, i32::MAX as u32);
        assert_eq!(decoded.pk_hate, vec![1002, 1001]);
        assert!(decoded.has_pk_hate_for(1001));
        assert!(!decoded.has_pk_hate_for(1003));
        assert!(!decoded.decode_legacy_pk_ppd(&encoded[..LEGACY_PK_PPD_SIZE - 1]));
    }

    #[test]
    fn pk_hate_helpers_preserve_legacy_front_priority_and_eviction() {
        let mut player = PlayerRuntime::connected(1, 0);
        assert!(!player.add_pk_hate(0));
        assert!(player.add_pk_hate(10));
        assert!(player.add_pk_hate(20));
        assert!(player.add_pk_hate(30));
        assert_eq!(player.pk_hate, vec![30, 20, 10]);

        assert!(!player.add_pk_hate(10));
        assert_eq!(player.pk_hate, vec![10, 30, 20]);

        assert!(player.remove_pk_hate(30));
        assert_eq!(player.pk_hate, vec![10, 0, 20]);
        assert!(player.has_any_pk_hate());
        assert!(!player.remove_pk_hate(30));

        let encoded = player.encode_legacy_pk_ppd();
        assert_eq!(read_i32(&encoded, PK_PPD_HATE_OFFSET), 10);
        assert_eq!(read_i32(&encoded, PK_PPD_HATE_OFFSET + 4), 0);
        assert_eq!(read_i32(&encoded, PK_PPD_HATE_OFFSET + 8), 20);

        for id in 100..(100 + PK_HATE_MAX_ENTRIES as u32 + 5) {
            player.add_pk_hate(id);
        }
        assert_eq!(player.pk_hate.len(), PK_HATE_MAX_ENTRIES);
        assert_eq!(player.pk_hate[0], 154);
        assert_eq!(player.pk_hate[PK_HATE_MAX_ENTRIES - 1], 105);
        assert!(!player.has_pk_hate_for(104));
    }

    #[test]
    fn pk_hate_decode_preserves_legacy_removed_slot_holes() {
        let mut bytes = vec![0; LEGACY_PK_PPD_SIZE];
        write_i32(&mut bytes, PK_PPD_HATE_OFFSET, 10);
        write_i32(&mut bytes, PK_PPD_HATE_OFFSET + 4, 0);
        write_i32(&mut bytes, PK_PPD_HATE_OFFSET + 8, 20);

        let mut player = PlayerRuntime::connected(1, 0);
        assert!(player.decode_legacy_pk_ppd(&bytes));
        assert_eq!(player.pk_hate, vec![10, 0, 20]);
        assert_eq!(
            player.active_pk_hate_ids().collect::<Vec<_>>(),
            vec![10, 20]
        );
        assert!(player.remove_pk_hate(10));
        assert_eq!(player.pk_hate, vec![0, 0, 20]);
        assert!(player.remove_pk_hate(20));
        assert!(player.pk_hate.is_empty());
        assert!(!player.has_any_pk_hate());
    }

    #[test]
    fn pk_hate_hit_helper_clears_legacy_lag_flag() {
        let mut player = PlayerRuntime::connected(1, 0);
        let mut character = character(1);
        character.flags.insert(CharacterFlags::LAG);

        assert!(player.add_pk_hate_from_hit(&mut character, 20));
        assert_eq!(player.pk_hate, vec![20]);
        assert!(!character.flags.contains(CharacterFlags::LAG));

        character.flags.insert(CharacterFlags::LAG);
        assert!(!player.add_pk_hate_from_hit(&mut character, 20));
        assert_eq!(player.pk_hate, vec![20]);
        assert!(!character.flags.contains(CharacterFlags::LAG));

        character.flags.insert(CharacterFlags::LAG);
        assert!(!player.add_pk_hate_from_hit(&mut character, 0));
        assert!(character.flags.contains(CharacterFlags::LAG));
    }

    #[test]
    fn pk_ppd_blob_round_trips_with_legacy_block_framing() {
        let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
        let mut existing_pk = vec![0; LEGACY_PK_PPD_SIZE];
        write_i32(&mut existing_pk, PK_PPD_KILLS_OFFSET, 1);
        write_i32(&mut existing_pk, PK_PPD_HATE_OFFSET, 999);

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
        write_ppd_block(&mut existing, DRD_PK_PPD, &existing_pk);

        let mut player = PlayerRuntime::connected(1, 0);
        player.pk_deaths = 2;
        assert!(player.add_pk_hate(1234));

        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), unknown_id);
        assert_eq!(read_u32(&encoded, 12), DRD_PK_PPD);
        assert_eq!(read_u32(&encoded, 16), LEGACY_PK_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 20 + PK_PPD_KILLS_OFFSET), 0);
        assert_eq!(read_i32(&encoded, 20 + PK_PPD_DEATHS_OFFSET), 2);
        assert_eq!(read_i32(&encoded, 20 + PK_PPD_HATE_OFFSET), 1234);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(decoded.pk_deaths, 2);
        assert_eq!(decoded.pk_hate, vec![1234]);
    }

    #[test]
    fn ppd_blob_appends_pk_without_existing_block() {
        let mut player = PlayerRuntime::connected(1, 0);
        assert!(player.add_pk_hate(777));

        let encoded = player.encode_legacy_ppd_blob(&[]);
        assert_eq!(read_u32(&encoded, 0), DRD_PK_PPD);
        assert_eq!(read_u32(&encoded, 4), LEGACY_PK_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 8 + PK_PPD_HATE_OFFSET), 777);
    }

    #[test]
    fn transport_ppd_blob_round_trips_with_legacy_block_framing() {
        let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
        let mut existing_transport = vec![0; LEGACY_TRANSPORT_PPD_SIZE];
        write_u64(&mut existing_transport, 0, 0x0000_0000_0000_0004);

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
        write_ppd_block(&mut existing, DRD_TRANSPORT_PPD, &existing_transport);

        let mut player = PlayerRuntime::connected(1, 0);
        player.transport_seen = 0x0000_0000_0000_0021;

        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), unknown_id);
        assert_eq!(read_u32(&encoded, 12), DRD_TRANSPORT_PPD);
        assert_eq!(read_u32(&encoded, 16), LEGACY_TRANSPORT_PPD_SIZE as u32);
        assert_eq!(read_u64(&encoded, 20), 0x0000_0000_0000_0021);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(decoded.transport_seen, 0x0000_0000_0000_0021);
    }

    #[test]
    fn ppd_blob_appends_transport_without_existing_block() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.touch_transport(5);

        let encoded = player.encode_legacy_ppd_blob(&[]);
        assert_eq!(read_u32(&encoded, 0), DRD_TRANSPORT_PPD);
        assert_eq!(read_u32(&encoded, 4), LEGACY_TRANSPORT_PPD_SIZE as u32);
        assert_eq!(read_u64(&encoded, 8), 1_u64 << 5);
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
    fn transport_discovery_marks_legacy_exploration_achievement_thresholds() {
        let mut player = PlayerRuntime::connected(1, 0);
        for point in [0, 2, 9, 21, 22, 23, 24] {
            assert!(player.touch_transport(point));
        }
        assert!(!player.achievements.traveller_of_astonia);

        assert!(player.touch_transport(25));
        assert!(player.achievements.traveller_of_astonia);

        let mut underground = PlayerRuntime::connected(2, 0);
        for point in 3..=7 {
            assert!(underground.touch_transport(point));
        }
        assert!(!underground.achievements.underground_explorer);
        assert!(underground.touch_transport(8));
        assert!(underground.achievements.underground_explorer);

        let mut explorer = PlayerRuntime::connected(3, 0);
        for point in 0..=25 {
            if ![11, 18, 19].contains(&point) {
                assert!(explorer.touch_transport(point));
            }
        }
        assert!(explorer.achievements.explorer_of_astonia);
        assert_eq!(explorer.transport_seen & !TRANSPORT_ALL_TELEPORTS_MASK, 0);
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
    fn demonshrine_touch_updates_value_and_blocks_repeats() {
        let mut player = PlayerRuntime::connected(1, 0);
        let mut character = character(3);
        character.exp = 10_000;

        assert_eq!(
            player.touch_demonshrine(&mut character, 0x0001_0203),
            DemonShrineResult::Learned { exp_added: 350 }
        );
        assert_eq!(character.values[1][CharacterValue::Demon as usize], 1);
        // C `demonshrine_driver` (`base.c:3231-3235`) applies the returned
        // `exp_added` via `give_exp`/`update_char`, both of which need
        // `&mut World` and so are the caller's responsibility
        // (`World::give_exp`/`World::update_character`, wired at the
        // `ItemDriverOutcome::DemonShrine` call site in
        // `ugaris-server/src/main.rs`) - `touch_demonshrine` itself no
        // longer mutates `character.exp`, only the Demon value and
        // `CF_ITEMS`.
        assert_eq!(character.exp, 10_000);
        assert!(!character.flags.contains(CharacterFlags::UPDATE));
        assert!(character.flags.contains(CharacterFlags::ITEMS));
        assert_eq!(
            player.touch_demonshrine(&mut character, 0x0001_0203),
            DemonShrineResult::AlreadyKnown
        );
    }

    #[test]
    fn demonshrine_ppd_blob_round_trips_with_legacy_block_framing() {
        let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
        let mut existing_demonshrine = vec![0; LEGACY_DEMONSHRINE_PPD_SIZE];
        write_i32(&mut existing_demonshrine, 0, 0x0001_0203);

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
        write_ppd_block(&mut existing, DRD_DEMONSHRINE_PPD, &existing_demonshrine);

        let mut player = PlayerRuntime::connected(1, 0);
        player.demonshrines.push(0x0001_0506);

        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), unknown_id);
        assert_eq!(read_u32(&encoded, 12), DRD_DEMONSHRINE_PPD);
        assert_eq!(read_u32(&encoded, 16), LEGACY_DEMONSHRINE_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 20), 0x0001_0506);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(decoded.demonshrines, vec![0x0001_0506]);
    }

    #[test]
    fn randomshrine_ppd_blob_round_trips_c_used_bitset() {
        let mut existing_randomshrine = vec![0; LEGACY_RANDOMSHRINE_PPD_SIZE];
        write_u32(&mut existing_randomshrine, 0, 1 << 3);
        write_u32(&mut existing_randomshrine, 28, 1 << 31);
        existing_randomshrine[32] = 17;

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, DRD_RANDOMSHRINE_PPD, &existing_randomshrine);

        let mut player = PlayerRuntime::connected(1, 0);
        assert!(player.decode_legacy_ppd_blob(&existing));
        assert!(player.has_used_random_shrine(3));
        assert!(player.has_used_random_shrine(255));
        assert!(!player.has_used_random_shrine(4));
        assert_eq!(player.random_shrine_continuity, 17);

        player.mark_random_shrine_used(64);
        player.random_shrine_continuity = 18;
        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), DRD_RANDOMSHRINE_PPD);
        assert_eq!(read_u32(&encoded, 4), LEGACY_RANDOMSHRINE_PPD_SIZE as u32);
        assert_eq!(read_u32(&encoded, 8), 1 << 3);
        assert_eq!(read_u32(&encoded, 16), 1);
        assert_eq!(read_u32(&encoded, 36), 1 << 31);
        assert_eq!(encoded[40], 18);
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
    fn misc_ppd_blob_preserves_non_tree_legacy_fields() {
        let mut existing_misc = vec![0; LEGACY_MISC_PPD_SIZE];
        write_i32(&mut existing_misc, 0, 123);
        write_i32(&mut existing_misc, 20, 456);
        write_i32(&mut existing_misc, MISC_PPD_GIFT_YEAR_OFFSET, 2024);

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, DRD_MISC_PPD, &existing_misc);

        let mut player = PlayerRuntime::connected(1, 0);
        assert!(player.decode_legacy_ppd_blob(&existing));
        assert_eq!(
            player.touch_xmas_tree(2, 2025, true, true),
            XmasTreeResult::GiftGranted
        );

        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), DRD_MISC_PPD);
        assert_eq!(read_u32(&encoded, 4), LEGACY_MISC_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 8), 123);
        assert_eq!(read_i32(&encoded, 28), 456);
        assert_eq!(encoded[8 + MISC_PPD_TREEDONE_OFFSET], 0b0000_0100);
        assert_eq!(read_i32(&encoded, 8 + MISC_PPD_GIFT_YEAR_OFFSET), 2025);
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
                assert!(
                    ![555, 55, 5, 666, 66, 6, 777, 77, 7, 888, 88, 8, 999, 99, 9].contains(&value)
                );
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
    fn area3_ppd_tracks_park_shrine_memorization() {
        let mut player = PlayerRuntime::connected(1, 0);

        assert_eq!(
            DRD_AREA3_PPD,
            make_drd(DEV_ID_DB, 40 | PERSISTENT_PLAYER_DATA)
        );
        assert_eq!(player.memorize_park_shrine(2), Some(true));
        assert_eq!(player.memorize_park_shrine(2), Some(false));
        assert_eq!(player.memorize_park_shrine(4), None);

        let encoded = player.encode_legacy_area3_ppd();
        assert_eq!(encoded.len(), LEGACY_AREA3_PPD_SIZE);
        assert_eq!(read_i32(&encoded, AREA3_PPD_KELLY_FOUND1_OFFSET), 0);
        assert_eq!(read_i32(&encoded, AREA3_PPD_KELLY_FOUND2_OFFSET), 1);
        assert_eq!(read_i32(&encoded, AREA3_PPD_KELLY_FOUND3_OFFSET), 0);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_area3_ppd(&encoded));
        assert_eq!(decoded.memorize_park_shrine(2), Some(false));
        assert_eq!(decoded.memorize_park_shrine(3), Some(true));
    }

    #[test]
    fn area3_ppd_exposes_clara_and_kelly_quest_states() {
        let mut player = PlayerRuntime::connected(1, 0);

        assert_eq!(player.area3_kelly_state(), 0);
        assert_eq!(player.area3_clara_state(), 0);

        player.set_area3_kelly_state(18);
        player.set_area3_clara_state(6);

        let encoded = player.encode_legacy_area3_ppd();
        assert_eq!(read_i32(&encoded, AREA3_PPD_KELLY_STATE_OFFSET), 18);
        assert_eq!(read_i32(&encoded, AREA3_PPD_CLARA_STATE_OFFSET), 6);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_area3_ppd(&encoded));
        assert_eq!(decoded.area3_kelly_state(), 18);
        assert_eq!(decoded.area3_clara_state(), 6);
    }

    #[test]
    fn area3_ppd_tracks_forest_chest_imp_flags() {
        let mut player = PlayerRuntime::connected(1, 0);

        assert_eq!(player.area3_imp_flags(), 0);
        assert!(player.mark_area3_imp_flag(1));
        assert!(!player.mark_area3_imp_flag(1));
        assert!(player.mark_area3_imp_flag(2));
        assert_eq!(player.area3_imp_flags(), 3);

        let encoded = player.encode_legacy_area3_ppd();
        assert_eq!(read_i32(&encoded, AREA3_PPD_IMP_FLAGS_OFFSET), 3);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_area3_ppd(&encoded));
        assert_eq!(decoded.area3_imp_flags(), 3);
    }

    #[test]
    fn area1_ppd_codec_matches_legacy_c_layout() {
        assert_eq!(
            DRD_AREA1_PPD,
            make_drd(DEV_ID_DB, 22 | PERSISTENT_PLAYER_DATA)
        );
        assert_eq!(LEGACY_AREA1_PPD_SIZE, 156);

        let mut player = PlayerRuntime::connected(1, 0);
        player.set_area1_yoakin_state(5);
        player.set_area1_gwendy_state(18);
        player.set_area1_nook_state(12);
        player.set_area1_lydia_state(6);
        player.set_area1_guiwynn_state(9);
        player.set_area1_logain_state(6);
        player.set_area1_reskin_state(8);
        player.set_area1_brithildie_state(21);
        player.set_area1_camhermit_state(13);
        player.set_area1_jessica_state(11);

        let encoded = player.encode_legacy_area1_ppd();
        assert_eq!(encoded.len(), LEGACY_AREA1_PPD_SIZE);
        assert_eq!(read_i32(&encoded, AREA1_PPD_YOAKIN_STATE_OFFSET), 5);
        assert_eq!(read_i32(&encoded, AREA1_PPD_GWENDY_STATE_OFFSET), 18);
        assert_eq!(read_i32(&encoded, AREA1_PPD_NOOK_STATE_OFFSET), 12);
        assert_eq!(read_i32(&encoded, AREA1_PPD_LYDIA_STATE_OFFSET), 6);
        assert_eq!(read_i32(&encoded, AREA1_PPD_GUIWYNN_STATE_OFFSET), 9);
        assert_eq!(read_i32(&encoded, AREA1_PPD_LOGAIN_STATE_OFFSET), 6);
        assert_eq!(read_i32(&encoded, AREA1_PPD_RESKIN_STATE_OFFSET), 8);
        assert_eq!(read_i32(&encoded, AREA1_PPD_BRITHILDIE_STATE_OFFSET), 21);
        assert_eq!(read_i32(&encoded, AREA1_PPD_CAMHERMIT_STATE_OFFSET), 13);
        assert_eq!(read_i32(&encoded, AREA1_PPD_JESSICA_STATE_OFFSET), 11);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_area1_ppd(&encoded));
        assert_eq!(decoded.area1_yoakin_state(), 5);
        assert_eq!(decoded.area1_gwendy_state(), 18);
        assert_eq!(decoded.area1_nook_state(), 12);
        assert_eq!(decoded.area1_lydia_state(), 6);
        assert_eq!(decoded.area1_guiwynn_state(), 9);
        assert_eq!(decoded.area1_logain_state(), 6);
        assert_eq!(decoded.area1_reskin_state(), 8);
        assert_eq!(decoded.area1_brithildie_state(), 21);
        assert_eq!(decoded.area1_camhermit_state(), 13);
        assert_eq!(decoded.area1_jessica_state(), 11);

        let state = decoded.area1_quest_state();
        assert_eq!(state.yoakin_state, 5);
        assert_eq!(state.gwendy_state, 18);
        assert_eq!(state.nook_state, 12);
        assert_eq!(state.lydia_state, 6);
        assert_eq!(state.guiwynn_state, 9);
        assert_eq!(state.logain_state, 6);
        assert_eq!(state.reskin_state, 8);
        assert_eq!(state.brithildie_state, 21);
        assert_eq!(state.camhermit_state, 13);
        assert_eq!(state.jessica_state, 11);
    }

    #[test]
    fn area1_ppd_blob_replaces_and_appends_legacy_block() {
        let mut existing_area1 = vec![0; LEGACY_AREA1_PPD_SIZE];
        write_i32(&mut existing_area1, AREA1_PPD_LYDIA_STATE_OFFSET, 3);

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, 0x1122_3344, &[1, 2, 3]);
        write_ppd_block(&mut existing, DRD_AREA1_PPD, &existing_area1);

        let mut player = PlayerRuntime::connected(1, 0);
        player.set_area1_lydia_state(6);
        let encoded = player.encode_legacy_ppd_blob(&existing);

        assert_eq!(read_u32(&encoded, 0), 0x1122_3344);
        assert_eq!(read_u32(&encoded, 11), DRD_AREA1_PPD);
        assert_eq!(read_u32(&encoded, 15), LEGACY_AREA1_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 19 + AREA1_PPD_LYDIA_STATE_OFFSET), 6);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(decoded.area1_lydia_state(), 6);

        let appended = player.encode_legacy_ppd_blob(&[]);
        assert_eq!(read_u32(&appended, 0), DRD_AREA1_PPD);
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
    fn staffer_ppd_tracks_forestbran_done_from_treasure_dig() {
        let mut player = PlayerRuntime::connected(1, 0);

        assert_eq!(player.forestbran_done(), 0);
        assert_eq!(player.set_forestbran_done(2), Some(3));
        assert_eq!(player.set_forestbran_done(5), None);
        assert_eq!(player.forestbran_done(), 3);

        let encoded = player.encode_legacy_staffer_ppd();
        assert_eq!(encoded.len(), LEGACY_STAFFER_PPD_SIZE);
        assert_eq!(read_i32(&encoded, STAFFER_PPD_FORESTBRAN_DONE_OFFSET), 3);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_staffer_ppd(&encoded));
        assert_eq!(decoded.forestbran_done(), 3);
    }

    #[test]
    fn staffer_ppd_blob_replaces_and_appends_legacy_block() {
        let mut existing_staffer = vec![0; LEGACY_STAFFER_PPD_SIZE];
        write_i32(&mut existing_staffer, STAFFER_PPD_FORESTBRAN_DONE_OFFSET, 4);

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, DRD_STAFFER_PPD, &existing_staffer);

        let mut player = PlayerRuntime::connected(1, 0);
        assert_eq!(player.set_forestbran_done(1), Some(2));
        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), DRD_STAFFER_PPD);
        assert_eq!(read_u32(&encoded, 4), LEGACY_STAFFER_PPD_SIZE as u32);
        assert_eq!(
            read_i32(&encoded, 8 + STAFFER_PPD_FORESTBRAN_DONE_OFFSET),
            2
        );

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(decoded.forestbran_done(), 2);

        let appended = player.encode_legacy_ppd_blob(&[]);
        assert_eq!(read_u32(&appended, 0), DRD_STAFFER_PPD);
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
    fn area3_ppd_blob_replaces_and_appends_legacy_block() {
        let mut existing_area3 = vec![0; LEGACY_AREA3_PPD_SIZE];
        write_i32(&mut existing_area3, AREA3_PPD_KELLY_FOUND1_OFFSET, 1);

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, 0x1122_3344, &[1, 2, 3]);
        write_ppd_block(&mut existing, DRD_AREA3_PPD, &existing_area3);

        let mut player = PlayerRuntime::connected(1, 0);
        assert_eq!(player.memorize_park_shrine(3), Some(true));
        let encoded = player.encode_legacy_ppd_blob(&existing);

        assert_eq!(read_u32(&encoded, 0), 0x1122_3344);
        assert_eq!(read_u32(&encoded, 11), DRD_AREA3_PPD);
        assert_eq!(read_u32(&encoded, 15), LEGACY_AREA3_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 19 + AREA3_PPD_KELLY_FOUND1_OFFSET), 0);
        assert_eq!(read_i32(&encoded, 19 + AREA3_PPD_KELLY_FOUND3_OFFSET), 1);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(decoded.memorize_park_shrine(3), Some(false));

        let appended = player.encode_legacy_ppd_blob(&[]);
        assert_eq!(read_u32(&appended, 0), DRD_AREA3_PPD);
    }

    #[test]
    fn twocity_ppd_codec_matches_legacy_c_layout() {
        assert_eq!(DRD_TWOCITY_PPD, 0x8100_0061);
        assert_eq!(LEGACY_TWOCITY_PPD_SIZE, 116);
        assert_eq!(TWOCITY_PPD_GOODTILE_OFFSET, 76);
        assert_eq!(TWOCITY_PPD_SOLVED_LIBRARY_OFFSET, 96);

        let mut player = PlayerRuntime::connected(1, 0);
        player.twocity_ppd = vec![0; LEGACY_TWOCITY_PPD_SIZE];
        write_i32(&mut player.twocity_ppd, 0, 1234);
        player.twocity_goodtile = [1, 2, 3, 4, 5];
        player.twocity_solved_library = true;

        let encoded = player.encode_legacy_twocity_ppd();
        assert_eq!(read_i32(&encoded, 0), 1234);
        for (index, color) in [1, 2, 3, 4, 5].into_iter().enumerate() {
            assert_eq!(
                read_i32(&encoded, TWOCITY_PPD_GOODTILE_OFFSET + index * 4),
                color
            );
        }
        assert_eq!(read_i32(&encoded, TWOCITY_PPD_SOLVED_LIBRARY_OFFSET), 1);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_twocity_ppd(&encoded));
        assert_eq!(decoded.twocity_goodtile, [1, 2, 3, 4, 5]);
        assert!(decoded.twocity_solved_library);
        assert_eq!(read_i32(&decoded.twocity_ppd, 0), 1234);
        assert!(!decoded.decode_legacy_twocity_ppd(&encoded[..LEGACY_TWOCITY_PPD_SIZE - 1]));
    }

    #[test]
    fn twocity_burndown_kill_updates_legacy_thief_fields() {
        assert_eq!(TWOCITY_PPD_THIEF_STATE_OFFSET, 32);
        assert_eq!(TWOCITY_PPD_THIEF_KILLED_OFFSET, 40);

        let mut player = PlayerRuntime::connected(1, 0);
        assert!(!player.mark_twocity_burndown_kill());
        assert_eq!(player.twocity_thief_state(), 0);
        assert_eq!(player.twocity_thief_killed(0), 0);

        player.set_twocity_thief_state(13);
        assert!(player.mark_twocity_burndown_kill());
        assert_eq!(player.twocity_thief_state(), 14);
        assert_eq!(player.twocity_thief_killed(0), 1);

        assert!(player.mark_twocity_burndown_kill());
        assert_eq!(player.twocity_thief_state(), 14);
        assert_eq!(player.twocity_thief_killed(0), 2);
        assert_eq!(player.twocity_thief_killed(6), 0);

        let encoded = player.encode_legacy_ppd_blob(&[]);
        assert_eq!(read_u32(&encoded, 0), DRD_TWOCITY_PPD);
        assert_eq!(read_i32(&encoded, 8 + TWOCITY_PPD_THIEF_STATE_OFFSET), 14);
        assert_eq!(read_i32(&encoded, 8 + TWOCITY_PPD_THIEF_KILLED_OFFSET), 2);
    }

    #[test]
    fn twocity_ppd_blob_replaces_and_appends_legacy_block() {
        let unknown_id = DRD_RANK_PPD; // still-unmodeled id, safe placeholder for round-trip tests
        let mut existing_twocity = vec![0; LEGACY_TWOCITY_PPD_SIZE];
        write_i32(&mut existing_twocity, 0, 777);
        write_i32(&mut existing_twocity, TWOCITY_PPD_GOODTILE_OFFSET, 6);

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
        write_ppd_block(&mut existing, DRD_TWOCITY_PPD, &existing_twocity);

        let mut player = PlayerRuntime::connected(1, 0);
        assert!(player.decode_legacy_ppd_blob(&existing));
        assert_eq!(player.twocity_goodtile[0], 6);
        player.twocity_goodtile = [2, 3, 4, 5, 6];
        player.twocity_solved_library = true;

        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), unknown_id);
        assert_eq!(read_u32(&encoded, 12), DRD_TWOCITY_PPD);
        assert_eq!(read_u32(&encoded, 16), LEGACY_TWOCITY_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 20), 777);
        assert_eq!(read_i32(&encoded, 20 + TWOCITY_PPD_GOODTILE_OFFSET), 2);
        assert_eq!(
            read_i32(&encoded, 20 + TWOCITY_PPD_SOLVED_LIBRARY_OFFSET),
            1
        );

        let mut appended_player = PlayerRuntime::connected(2, 0);
        appended_player.twocity_goodtile = [1, 1, 2, 2, 3];
        let appended = appended_player.encode_legacy_ppd_blob(&[]);
        assert_eq!(read_u32(&appended, 0), DRD_TWOCITY_PPD);
        assert_eq!(read_u32(&appended, 4), LEGACY_TWOCITY_PPD_SIZE as u32);
        assert_eq!(read_i32(&appended, 8 + TWOCITY_PPD_GOODTILE_OFFSET), 1);
    }

    #[test]
    fn malformed_ppd_blob_is_rejected() {
        let mut player = PlayerRuntime::connected(1, 0);
        let mut malformed = Vec::new();
        malformed.extend_from_slice(&DRD_KEYRING_PPD.to_le_bytes());
        malformed.extend_from_slice(&(LEGACY_KEYRING_PPD_SIZE as u32).to_le_bytes());
        malformed.extend_from_slice(&[0; 7]);

        assert!(!player.decode_legacy_ppd_blob(&malformed));
    }

    #[test]
    fn keyring_display_lines_match_legacy_shape_and_remove_by_position() {
        let mut player = PlayerRuntime::connected(1, 0);

        assert_eq!(
            player.keyring_display_lines(),
            vec!["Your keyring is empty."]
        );
        assert_eq!(
            player.add_keyring_key(0x1122_3344, "Copper Key"),
            KeyringAddResult::Added
        );
        assert_eq!(
            player.add_keyring_key(0x5566_7788, "Silver Key"),
            KeyringAddResult::Added
        );

        assert_eq!(
            player.keyring_display_lines(),
            vec![
                "=== Keyring (2/100 keys) ===",
                " 1. Copper Key",
                " 2. Silver Key",
                "Use a key on the keyring to add it.",
                "Type '#keyring remove <number>' to remove a key.",
                "Type '#keyring addall' to add all keys from inventory.",
            ]
        );
        assert_eq!(
            player.remove_keyring_key_at(0).map(|key| key.name),
            Some("Copper Key".to_string())
        );
        assert_eq!(player.keyring_key_name(0x1122_3344), None);
        assert_eq!(player.keyring_key_name(0x5566_7788), Some("Silver Key"));
        assert_eq!(player.remove_keyring_key_at(99), None);
    }

    #[test]
    fn chest_achievement_state_tracks_legacy_threshold_hooks() {
        let mut player = PlayerRuntime::connected(1, 0);

        for _ in 0..9 {
            player.record_chest_opened(1);
        }
        assert_eq!(player.achievements.chests_opened, 9);
        assert!(!player.achievements.looter);

        player.record_chest_opened(1);
        assert!(player.achievements.looter);
        assert!(!player.achievements.treasure_hunter);

        for _ in 10..50 {
            player.record_chest_opened(1);
        }
        assert!(player.achievements.treasure_hunter);
        assert!(!player.achievements.treasure_master);

        for _ in 50..100 {
            player.record_chest_opened(1);
        }
        assert!(player.achievements.treasure_master);
        assert!(!player.achievements.legendary_looter);

        for _ in 100..500 {
            player.record_chest_opened(1);
        }
        assert!(player.achievements.legendary_looter);

        player.record_chest_opened(63);
        assert!(player.achievements.gold_looter);
    }

    #[test]
    fn random_chest_access_tracks_hundred_recent_locations() {
        let mut player = PlayerRuntime::connected(1, 0);

        player.mark_random_chest_used(7, 100);
        assert_eq!(player.random_chest_last_used_seconds(7), Some(100));
        player.mark_random_chest_used(7, 200);
        assert_eq!(player.random_chest_last_used_seconds(7), Some(200));

        for index in 1..RANDCHEST_MAX_ENTRIES {
            player.mark_random_chest_used(1_000 + index as u32, index as u64);
        }
        assert_eq!(player.random_chests.len(), RANDCHEST_MAX_ENTRIES);
        player.mark_random_chest_used(9_999, 300);
        assert_eq!(player.random_chests.len(), RANDCHEST_MAX_ENTRIES);
        assert_eq!(player.random_chest_last_used_seconds(9_999), Some(300));
    }

    #[test]
    fn driver_stop_clears_action_queue_and_fightback_state() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.driver_move(10, 11);
        player.driver_selfspell(PlayerActionCode::Bless);
        player.next_fightback_character = Some(CharacterId(2));
        player.next_fightback_serial = 44;
        player.next_fightback_tick = 55;

        player.driver_stop(99, true);

        assert_eq!(player.action.action, PlayerActionCode::Idle);
        assert!(player.queue.is_empty());
        assert_eq!(player.next_fightback_character, None);
        assert_eq!(player.next_fightback_serial, 0);
        assert_eq!(player.next_fightback_tick, 0);
        assert_eq!(player.nofight_timer, 99);
    }

    #[test]
    fn driver_setters_match_c_action_payloads() {
        let mut player = PlayerRuntime::connected(1, 0);

        player.driver_take(7, 1234);
        assert_eq!(player.action.action, PlayerActionCode::Take);
        assert_eq!((player.action.arg1, player.action.arg2), (7, 1234));

        player.driver_kill(CharacterId(9), 4321);
        assert_eq!(player.action.action, PlayerActionCode::Kill);
        assert_eq!((player.action.arg1, player.action.arg2), (9, 4321));

        player.driver_drop(12, 13);
        assert_eq!(player.action.action, PlayerActionCode::Drop);
        assert_eq!((player.action.arg1, player.action.arg2), (12, 13));
    }

    #[test]
    fn got_hit_fightback_immediately_kills_when_idle_and_nearby() {
        let mut player = PlayerRuntime::connected(1, 0);

        assert!(player.apply_got_hit_fightback(CharacterId(2), 77, 2, TICKS_PER_SECOND * 3 + 1,));

        assert_eq!(player.action.action, PlayerActionCode::Kill);
        assert_eq!((player.action.arg1, player.action.arg2), (2, 77));
        assert_eq!(player.next_fightback_character, None);
    }

    #[test]
    fn got_hit_fightback_defers_while_busy_and_promotes_when_idle() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.driver_move(20, 21);
        let hit_tick = TICKS_PER_SECOND * 4;

        assert!(player.apply_got_hit_fightback(CharacterId(3), 88, 2, hit_tick));
        assert_eq!(player.action.action, PlayerActionCode::Move);
        assert_eq!(player.next_fightback_character, Some(CharacterId(3)));
        assert_eq!(player.next_fightback_serial, 88);
        assert_eq!(player.next_fightback_tick, hit_tick);

        player.driver_halt();
        player.next_fightback_character = Some(CharacterId(3));
        player.next_fightback_serial = 88;
        player.next_fightback_tick = hit_tick;

        assert!(player.apply_deferred_fightback(hit_tick + TICKS_PER_SECOND - 1));
        assert_eq!(player.action.action, PlayerActionCode::Kill);
        assert_eq!((player.action.arg1, player.action.arg2), (3, 88));
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

    #[test]
    fn deferred_fightback_expires_after_one_second() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.next_fightback_character = Some(CharacterId(2));
        player.next_fightback_serial = 77;
        player.next_fightback_tick = TICKS_PER_SECOND * 4;

        assert!(!player.apply_deferred_fightback(TICKS_PER_SECOND * 5));
        assert_eq!(player.action.action, PlayerActionCode::Idle);
    }

    #[test]
    fn driver_spell_queue_overwrites_last_slot_when_full() {
        let mut player = PlayerRuntime::connected(1, 0);
        for n in 0..COMMAND_QUEUE_SIZE {
            player.driver_mapspell(PlayerActionCode::Fireball, n as i32, 0);
        }

        player.driver_selfspell(PlayerActionCode::Bless);

        assert_eq!(player.queue.len(), COMMAND_QUEUE_SIZE);
        assert_eq!(player.queue.front().unwrap().arg1, 0);
        assert_eq!(player.queue.back().unwrap().action, PlayerActionCode::Bless);
    }

    fn character(id: u32) -> Character {
        Character {
            merchant: None,
            template_key: String::new(),
            respawn_ticks: 0,
            id: CharacterId(id),
            serial: id,
            name: "Character".into(),
            description: String::new(),
            flags: CharacterFlags::USED,
            sprite: 0,
            c1: 0,
            c2: 0,
            c3: 0,
            driver: 0,
            group: 0,
            clan: 0,
            clan_rank: 0,
            clan_serial: 0,
            staff_code: String::new(),
            speed_mode: crate::entity::SpeedMode::Normal,
            x: 0,
            y: 0,
            rest_area: 0,
            rest_x: 0,
            rest_y: 0,
            tox: 0,
            toy: 0,
            dir: 0,
            action: 0,
            duration: 0,
            step: 0,
            act1: 0,
            act2: 0,
            hp: 0,
            mana: 0,
            endurance: 0,
            lifeshield: 0,
            level: 1,
            exp: 0,
            exp_used: 0,
            military_points: 0,
            military_normal_exp: 0,
            gold: 0,
            karma: 0,
            creation_time: 0,
            saves: 0,
            got_saved: 0,
            deaths: 0,
            regen_ticker: 0,
            last_regen: 0,
            cursor_item: None,
            current_container: None,
            values: Character::empty_values(),
            professions: Character::empty_professions(),
            inventory: Character::empty_inventory(),
            driver_state: None,
            driver_messages: Vec::new(),
            driver_memory: crate::character_driver::DriverMemory::default(),
        }
    }
}
