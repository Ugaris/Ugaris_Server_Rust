// The PPD byte-offset constants and codecs in this module mirror the C
// `struct *_ppd` layouts verbatim as `<field index> * 4` products (so
// `0 * 4`, `1 * 4`, ... line up visually with the C struct order); keep
// clippy from "simplifying" the intentional identity/zero terms.
#![allow(clippy::identity_op, clippy::erasing_op)]

use super::*;

pub(crate) const ARENA_PPD_SCORE_OFFSET: usize = 0 * 4;

pub(crate) const ARENA_PPD_FIGHTS_OFFSET: usize = 1 * 4;

pub(crate) const ARENA_PPD_WINS_OFFSET: usize = 2 * 4;

pub(crate) const ARENA_PPD_LOSSES_OFFSET: usize = 3 * 4;

pub(crate) const ARENA_PPD_LASTFIGHT_OFFSET: usize = 4 * 4;

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

pub(crate) const TUNNEL_PPD_USED_BASE_OFFSET: usize = 4;

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

/// C `struct lab_ppd::herald_talkstep` (`src/system/lab.h:30`): offset 42
/// (after `solved_bits` (8) + `dummy[8]` (32) + `timesgotcryptgold` (1) +
/// `timesgotyardgold` (1)), one byte before `graveversion` at offset 43.
pub const LEGACY_LAB2_HERALD_TALKSTEP_OFFSET: usize = 42;

pub const LEGACY_LAB2_GRAVEVERSION_OFFSET: usize = 43;

pub const LEGACY_LAB2_GRAVEINDEX_OFFSET: usize = 44;

/// C `struct lab_ppd::password1` (`src/system/lab.h`): offset 80 (after
/// `graveindex[4]` (4, ending at 48) + `lab2_dummy[8]` (32, ending at 80)).
/// An 8-byte nul-terminated ASCII fragment (`lab3_passguard_driver`'s
/// `sprintf(password, "%s%s", ppd->password1, ppd->password2)`).
pub const LEGACY_LAB3_PASSWORD1_OFFSET: usize = 80;

/// C `struct lab_ppd::password2` (`src/system/lab.h`): offset 88, right
/// after `password1[8]`.
pub const LEGACY_LAB3_PASSWORD2_OFFSET: usize = 88;

/// C `struct lab_ppd::prisoner_talkstep` (`src/system/lab.h`): offset 96,
/// right after `password2[8]`.
pub const LEGACY_LAB3_PRISONER_TALKSTEP_OFFSET: usize = 96;

/// C `struct lab_ppd::guard_talkstep` (`src/system/lab.h`): offset 97.
pub const LEGACY_LAB3_GUARD_TALKSTEP_OFFSET: usize = 97;

pub(crate) const WARP_PPD_BASE_OFFSET: usize = 0;

pub(crate) const WARP_PPD_POINTS_OFFSET: usize = WARP_PPD_BASE_OFFSET + 4;

pub(crate) const WARP_PPD_BONUS_ID_OFFSET: usize = WARP_PPD_POINTS_OFFSET + 4;

pub(crate) const WARP_PPD_BONUS_LAST_USED_OFFSET: usize =
    WARP_PPD_BONUS_ID_OFFSET + WARP_BONUS_COUNT * 4;

pub(crate) const WARP_PPD_NOSTEPEXP_OFFSET: usize =
    WARP_PPD_BONUS_LAST_USED_OFFSET + WARP_BONUS_COUNT * 4;

pub(crate) const GATE_PPD_WELCOME_STATE_OFFSET: usize = 0;

pub(crate) const GATE_PPD_TARGET_CLASS_OFFSET: usize = 4;

pub(crate) const GATE_PPD_STEP_OFFSET: usize = 8;

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

// `struct caligar_ppd` field offsets (`src/area/36/caligar.c:216-232`),
// in declaration order: `guard_state`/`guard_last_talk`/`glori_state`/
// `glori_last_talk`/`watch_flag`/`obelisk_flag`/`arquin_state`/
// `arquin_last_talk`/`smith_state`/`smith_last_talk`/`homden_state`/
// `homden_last_talk`/`amazon_flag`/`guard2_last_talk`/`door_flag[4]`.
// `obelisk_flag`/`amazon_flag` are declared in C but never read or
// written by any driver in `caligar.c` (confirmed via full-source grep) -
// no accessor exists for them, their bytes are still preserved verbatim
// by the raw blob round-trip.
pub(crate) const CALIGAR_PPD_GUARD_STATE_OFFSET: usize = 0 * 4;

pub(crate) const CALIGAR_PPD_GUARD_LAST_TALK_OFFSET: usize = 1 * 4;

pub(crate) const CALIGAR_PPD_GLORI_STATE_OFFSET: usize = 2 * 4;

pub(crate) const CALIGAR_PPD_GLORI_LAST_TALK_OFFSET: usize = 3 * 4;

pub(crate) const CALIGAR_PPD_WATCH_FLAG_OFFSET: usize = 4 * 4;

pub(crate) const CALIGAR_PPD_ARQUIN_STATE_OFFSET: usize = 6 * 4;

pub(crate) const CALIGAR_PPD_ARQUIN_LAST_TALK_OFFSET: usize = 7 * 4;

pub(crate) const CALIGAR_PPD_SMITH_STATE_OFFSET: usize = 8 * 4;

pub(crate) const CALIGAR_PPD_SMITH_LAST_TALK_OFFSET: usize = 9 * 4;

pub(crate) const CALIGAR_PPD_HOMDEN_STATE_OFFSET: usize = 10 * 4;

pub(crate) const CALIGAR_PPD_HOMDEN_LAST_TALK_OFFSET: usize = 11 * 4;

pub(crate) const CALIGAR_PPD_GUARD2_LAST_TALK_OFFSET: usize = 13 * 4;

pub(crate) const CALIGAR_PPD_DOOR_FLAG_OFFSET: usize = 14 * 4;

pub const ARKHATA_PPD_CLERK_STATE_OFFSET: usize = 16 * 4;

pub const ARKHATA_PPD_CLERK_TIME_OFFSET: usize = 17 * 4;

/// C `struct arkhata_ppd::trainer_state` (`src/area/37/arkhata.h:19`,
/// field index 14): `trainer_driver`'s (`world::npc::area37::trainer`)
/// own quest-75 ("A Kidnapped Student") dialogue state.
pub const ARKHATA_PPD_TRAINER_STATE_OFFSET: usize = 14 * 4;

/// C `struct arkhata_ppd::kid_state` (`src/area/37/arkhata.h:20`, field
/// index 15): `kidnappee_driver`'s (`world::npc::area37::kidnappee`) own
/// rescue-progress state, read by `trainer_driver` at `trainer_state` `6`
/// to notice the rescue and by `kidnappee_driver` itself.
pub const ARKHATA_PPD_KID_STATE_OFFSET: usize = 15 * 4;

/// C `struct arkhata_ppd::clerk_bits` (`src/area/37/arkhata.h:23`, field
/// index 18): the three-note turn-in bitmask (`1`/`2`/`4`)
/// `clerk_driver`'s (`world::npc::area37::clerk`) own `NT_GIVE` handler
/// maintains; `== (1|2|4)` triggers quest 76's completion.
pub const ARKHATA_PPD_CLERK_BITS_OFFSET: usize = 18 * 4;

/// C `struct arkhata_ppd::krenach_state` (`src/area/37/arkhata.h:24`,
/// field index 19): `krenach_driver`'s (`world::npc::area37::krenach`)
/// own dialogue state.
pub const ARKHATA_PPD_KRENACH_STATE_OFFSET: usize = 19 * 4;

/// C `struct arkhata_ppd::krenach_time` (`src/area/37/arkhata.h:25`,
/// field index 20): `krenach_driver`'s own "already grumbled recently"
/// wall-clock throttle stamp (`realtime`, `arkhata.c:4260-4263`).
pub const ARKHATA_PPD_KRENACH_TIME_OFFSET: usize = 20 * 4;

/// C `struct arkhata_ppd::rammy_state` (`src/area/37/arkhata.h:5`, field
/// index 0, the first field): `rammy_driver`'s (`world::npc::
/// area37::rammy`) own dialogue state. Also read by
/// `guard_brannington_driver`'s (`world::npc::area29::guardbran`) "Finding
/// Arkhata" (quest 64) completion check (`ppd->rammy_state > 0`,
/// `brannington.c:1938`) - same "read state owned by another area's
/// driver" precedent as `PlayerRuntime::staffer_broklin_state`/
/// `staffer_carlos2_state`.
pub const ARKHATA_PPD_RAMMY_STATE_OFFSET: usize = 0 * 4;

/// C `struct arkhata_ppd::jaz_state` (`src/area/37/arkhata.h:6`, field
/// index 1): `jaz_driver`'s (`world::npc::area37::jaz`) own quest-66
/// ("Ishtar's Bracelet") dialogue state.
pub const ARKHATA_PPD_JAZ_STATE_OFFSET: usize = 1 * 4;

/// C `struct arkhata_ppd::letter_bits` (`src/area/37/arkhata.h:13`, field
/// index 8): the introduction-letter turn-in bitmask (`2`=`captain_state`
/// wrote it, `4`=`judge_state` wrote it, `8`=... - all three still-
/// unported drivers) `rammy_driver` (`world::npc::area37::rammy`) reads
/// at `rammy_state` 17 (`== (2|4|8)`) to advance to the "trade route is
/// open" completion line.
pub const ARKHATA_PPD_LETTER_BITS_OFFSET: usize = 8 * 4;

/// C `struct arkhata_ppd::monk_state` (`src/area/37/arkhata.h:9`, field
/// index 4): `arkhatamonk_driver`'s (`world::npc::area37::arkhatamonk`)
/// own dialogue state. `smith_driver`'s (`world::npc::area36::smith`)
/// "did you talk to the wise monk yet" gate (`caligar.c:1032-1037`,
/// `appd->monk_state > 20`) also reads it, same "read state owned by
/// another area's driver" precedent as `ARKHATA_PPD_RAMMY_STATE_OFFSET`
/// itself. `bookeater_dead`'s `ppd->monk_state = 20` (`arkhata.c:4350`,
/// gated on the prior value being exactly `19`) is the other writer - see
/// `PlayerRuntime::set_arkhata_monk_state`.
pub const ARKHATA_PPD_MONK_STATE_OFFSET: usize = 4 * 4;

/// C `struct arkhata_ppd::monk_bits` (`src/area/37/arkhata.h:10`, field
/// index 5): the per-persona key-part turn-in bitmask (`1`=Gregor,
/// `2`=Johan, `4`=Johnatan) `arkhatamonk_driver`'s own `NT_GIVE` handler
/// maintains; `== 7` triggers quest 69's completion.
pub const ARKHATA_PPD_MONK_BITS_OFFSET: usize = 5 * 4;

/// C `struct arkhata_ppd::ramin_state` (`src/area/37/arkhata.h:7`, field
/// index 3): the still-unported `ramin_driver`'s own dialogue state.
/// `arkhataskelly_dead` (`arkhata.c:1612-1646`) reads it (must be exactly
/// `6`, i.e. Ramin already sent the killer to clear the Fighting School's
/// skeleton infestation) and, once every arkhataskelly is dead, writes it
/// to `7` so the killer can report back - see
/// `PlayerRuntime::set_arkhata_ramin_state`.
pub const ARKHATA_PPD_RAMIN_STATE_OFFSET: usize = 3 * 4;

/// C `struct arkhata_ppd::fiona_state` (`src/area/37/arkhata.h:8`, field
/// index 2): `fiona_driver`'s (`world::npc::area37::fiona`) own quest-67
/// ("The Missing Ring") dialogue/student-challenge/skill-raise state.
pub const ARKHATA_PPD_FIONA_STATE_OFFSET: usize = 2 * 4;

/// C `struct arkhata_ppd::captain_state` (`src/area/37/arkhata.h:11`,
/// field index 6): `captain_driver`'s (`world::npc::area37::captain`) own
/// dialogue state - the Fortress Captain, the first stop of the
/// entrance-pass-system chain that continues through `judge_driver`.
pub const ARKHATA_PPD_CAPTAIN_STATE_OFFSET: usize = 6 * 4;

/// C `struct arkhata_ppd::judge_state` (`src/area/37/arkhata.h:12`, field
/// index 7): `judge_driver`'s (`world::npc::area37::judge`) own dialogue
/// state - reads `captain_state` to know when to start, hands out
/// letters 2/3/4/5.
pub const ARKHATA_PPD_JUDGE_STATE_OFFSET: usize = 7 * 4;

/// C `struct arkhata_ppd::jada_state` (`src/area/37/arkhata.h:14`, field
/// index 9): `jada_driver`'s (`world::npc::area37::jada`) own quest-72
/// ("The Source") dialogue state - gated on `ramin_state >= 12` to start.
pub const ARKHATA_PPD_JADA_STATE_OFFSET: usize = 9 * 4;

/// C `struct arkhata_ppd::pot_state` (`src/area/37/arkhata.h:15`, field
/// index 10): `potmaker_driver`'s (`world::npc::area37::potmaker`) own
/// quest-73 ("A Special Pot") dialogue state - gated on `ch[co].level >=
/// 48` to start.
pub const ARKHATA_PPD_POT_STATE_OFFSET: usize = 10 * 4;

/// C `struct arkhata_ppd::hunter_state` (`src/area/37/arkhata.h:16`, field
/// index 11): `hunter_driver`'s (`world::npc::area37::hunter`) own quest-77
/// ("The Blue Harpy") dialogue state - gated on `pot_state > 0`
/// (`world::npc::area37::potmaker`'s own progress) to start.
pub const ARKHATA_PPD_HUNTER_STATE_OFFSET: usize = 11 * 4;

/// C `struct arkhata_ppd::thai_state` (`src/area/37/arkhata.h:17`, field
/// index 12): `thaipan_driver`'s (`world::npc::area37::thaipan`) own
/// quest-74 ("The Ancient Scroll") dialogue state - gated on
/// `ch[co].level >= 49` to start.
pub const ARKHATA_PPD_THAI_STATE_OFFSET: usize = 12 * 4;

/// C `struct arkhata_ppd::last_budda` (`src/area/37/arkhata.h:18`, field
/// index 13): wall-clock `realtime` seconds of the last successful
/// `IID_ARKHATA_BUDDA` "recover negative experience" hand-in, gating the
/// once-per-24h cooldown at `thaipan_driver` (`arkhata.c:3533-3552`).
pub const ARKHATA_PPD_LAST_BUDDA_OFFSET: usize = 13 * 4;

// `struct staffer_ppd` field offsets (`src/common/staffer_ppd.h:13-` /
// `src/system/game/ppd_structs.h:566-`), in declaration order. Only the
// fields consumed by `questlog_init_staff` (`src/system/questlog.c:1203-
// 1394`) plus the two pre-existing named fields below have accessors.
pub(crate) const STAFFER_PPD_SMUGGLECOM_STATE_OFFSET: usize = 0 * 4;

pub(crate) const STAFFER_PPD_SMUGGLECOM_BITS_OFFSET: usize = 1 * 4;

pub(crate) const STAFFER_PPD_CARLOS_STATE_OFFSET: usize = 2 * 4;

pub(crate) const STAFFER_PPD_COUNTBRAN_STATE_OFFSET: usize = 3 * 4;

pub(crate) const STAFFER_PPD_COUNTBRAN_BITS_OFFSET: usize = 4 * 4;

/// C `struct staffer_ppd::countessabran_state` (`src/common/staffer_ppd.h:
/// 21`, field index 5): `countessa_brannington_driver`'s (`world::npc::
/// area29::countessabran`) reward-dialogue state, gated on `countbran_bits`.
pub(crate) const STAFFER_PPD_COUNTESSABRAN_STATE_OFFSET: usize = 5 * 4;

/// C `struct staffer_ppd::daughterbran_state` (`src/common/staffer_ppd.h:
/// 22`, field index 6): `daughter_brannington_driver`'s (`world::npc::
/// area29::daughterbran`) reward-dialogue state, gated on `countbran_bits`.
pub(crate) const STAFFER_PPD_DAUGHTERBRAN_STATE_OFFSET: usize = 6 * 4;

pub(crate) const STAFFER_PPD_SPIRITBRAN_STATE_OFFSET: usize = 7 * 4;

/// C `struct staffer_ppd::guardbran_state` (`src/common/staffer_ppd.h:
/// 23`, field index 8): `guard_brannington_driver`'s (`world::npc::
/// area29::guardbran`) greeting/mission dialogue for "Finding Arkhata"
/// (quest 64), gated on `countbran_bits` (all three jewels) plus level 45.
pub(crate) const STAFFER_PPD_GUARDBRAN_STATE_OFFSET: usize = 8 * 4;

pub(crate) const STAFFER_PPD_BRENNETHBRAN_STATE_OFFSET: usize = 9 * 4;

/// C `struct staffer_ppd::forestbran_state` (`src/common/staffer_ppd.h:26`,
/// field index 10): `forest_brannington_driver`'s (`world::npc::area29::
/// forestbran`) five-state greeting/hint dialogue, separate from the
/// neighboring `forestbran_done` field (index 11, already exposed via
/// `PlayerRuntime::forestbran_done`/`set_forestbran_done`).
pub(crate) const STAFFER_PPD_FORESTBRAN_STATE_OFFSET: usize = 10 * 4;

pub(crate) const STAFFER_PPD_BROKLIN_STATE_OFFSET: usize = 12 * 4;

pub(crate) const STAFFER_PPD_ARISTOCRAT_STATE_OFFSET: usize = 13 * 4;

pub(crate) const STAFFER_PPD_YOATIN_STATE_OFFSET: usize = 14 * 4;

/// C `struct staffer_ppd::grinnich_state` (`src/common/staffer_ppd.h:33`,
/// field index 15): `grinnich_driver`'s (`world::npc::area29::grinnich`)
/// tower-entrance hint dialogue, mirrored by `shanra_state` below for the
/// tower's basement NPC.
pub(crate) const STAFFER_PPD_GRINNICH_STATE_OFFSET: usize = 15 * 4;

pub(crate) const STAFFER_PPD_SHANRA_STATE_OFFSET: usize = 16 * 4;

/// C `struct staffer_ppd::centinel_count` (`src/common/staffer_ppd.h:35`,
/// field index 17): `centinel_dead`'s (`src/area/29/brannington.c:2725-
/// 2758`) per-player sentinel kill counter, capped at 30, reset to `0`
/// once the level-30 teleport succeeds.
pub(crate) const STAFFER_PPD_CENTINEL_COUNT_OFFSET: usize = 17 * 4;

pub(crate) const STAFFER_PPD_DWARFCHIEF_STATE_OFFSET: usize = 18 * 4;

pub(crate) const STAFFER_PPD_DWARFSHAMAN_STATE_OFFSET: usize = 19 * 4;

/// C `struct staffer_ppd::dwarfshaman_count` (`src/common/staffer_ppd.h`,
/// field index 20): `dwarfshaman_driver`'s (`world::npc::area31::
/// dwarfshaman`) lizard-teeth/brown-berry turn-in counter, reset to `0`
/// each time it reaches 9 (`src/area/31/warrmines.c`).
pub(crate) const STAFFER_PPD_DWARFSHAMAN_COUNT_OFFSET: usize = 20 * 4;

/// C `struct staffer_ppd::dwarfsmith_state` (`src/common/staffer_ppd.h`,
/// field index 21): `dwarfsmith_driver`'s (`world::npc::area31::
/// dwarfsmith`) mold-for-key exchange state (`src/area/31/warrmines.c`).
pub(crate) const STAFFER_PPD_DWARFSMITH_STATE_OFFSET: usize = 21 * 4;

/// C `struct staffer_ppd::dwarfsmith_type` (`src/common/staffer_ppd.h`,
/// field index 22): the lizard-elite-key variant (`1`/`2`/`3`)
/// `dwarfsmith_driver` remembers between receiving the mold and receiving
/// the silver payment (`src/area/31/warrmines.c`).
pub(crate) const STAFFER_PPD_DWARFSMITH_TYPE_OFFSET: usize = 22 * 4;

/// C `struct staffer_ppd::carlos2_state` (`src/common/staffer_ppd.h:43`,
/// field index 23 - the last field before `rouven_state`), consumed by
/// `carlos_driver`'s Imperial Vault ritual quest (`src/area/3/area3.c`).
pub(crate) const STAFFER_PPD_CARLOS2_STATE_OFFSET: usize = 23 * 4;

/// C `struct staffer_ppd::rouven_state` (`src/common/staffer_ppd.h:44`,
/// field index 24, the last field): `rouven_driver`'s Imperial Vault
/// guard quest state (`src/area/26/staffer.c`), also read by `vault_skull`
/// (`IDR_STAFFER` `drdata[0]==4`).
pub(crate) const STAFFER_PPD_ROUVEN_STATE_OFFSET: usize = 24 * 4;

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

/// C `struct misc_ppd::last_lq_death` (`src/common/misc_ppd.h:26`):
/// real-time-seconds timestamp of this player's last Live Quest area
/// (20/35) death/`#wimp`, gating `lq_entrance`'s 5-minute re-entry
/// penalty (`lq.c:2979-2982` - not yet ported, see `PORTING_TODO.md`'s
/// Area 20 entry) and set by `cmd_wimp` (`lq.c:2323-2334`, ported in
/// `world::lq_usurp`) and the LQ-area no-real-death `hurt_char` branch
/// (`death.c:1238-1249` - not yet ported, an unrelated gap in the P1
/// player-death-saves system, not this admin-command-table task).
pub(crate) const MISC_PPD_LAST_LQ_DEATH_OFFSET: usize = 8;

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
