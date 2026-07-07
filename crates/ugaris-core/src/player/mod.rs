//! Per-player session/persistent runtime state.
//!
//! `PlayerRuntime` (this file) owns the struct plus the legacy outer PPD
//! blob framing; per-system accessors and their legacy `DRD_*_PPD` codecs
//! live in submodules named after the system (`area1`, `military`,
//! `keyring`, ...). New persistent state must be a typed serde field on
//! `PlayerRuntime` (JSONB persistence); legacy PPD codecs exist only to
//! read old blobs and to stay byte-compatible with the C oracle's layouts.

mod actions;
mod area1;
mod area3;
mod areas_misc;
mod arena;
mod chests;
mod keyring;
mod labs;
mod military;
mod misc;
mod pk;
mod questlog;
mod settings;
mod shrines;
mod staffer;
mod transport;
mod tunnel;
mod twocity;

pub use misc::*;
pub use settings::*;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{
    achievement::{AccountAchievements, AchievementStats},
    entity::{Character, CharacterFlags, CharacterValue, Item, ItemFlags, MAX_MODIFIERS},
    ids::{CharacterId, ItemId},
    legacy::DIST_OLD,
    quest::{QuestLog, MAX_QUESTS},
    tell::TellData,
    tick::TICKS_PER_SECOND,
};

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
    /// Row id of this connection's `anticheat_sessions` DB record (C
    /// `player[nr]->ac.session_id`, `anticheat.h`), set once
    /// `ac_player_login`'s Rust equivalent creates the session and cleared
    /// on reconnect/disconnect. `None` when running without a database or
    /// before the session row has been created.
    pub anticheat_session_id: Option<i64>,
    /// C `player[nr]->ac.watch_mode` (`anticheat.h`), toggled by
    /// `#acwatch <player>` (`ac_cmd_watch`, `anticheat.c:894-921`). Purely
    /// in-memory in C, kept live for the (unported) detection engine's
    /// verbose-logging check - this codebase has no such engine yet, so
    /// the flag currently has no behavioral effect beyond the toggle
    /// message itself, matching every other pre-wired-but-inert toggle in
    /// this struct (`no_ball` and siblings).
    #[serde(default)]
    pub ac_watch_enabled: bool,
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
    /// C `struct firstkill_ppd`'s `kill[32]` bitmask
    /// (`DRD_FIRSTKILL_PPD`): one bit per unique NPC `ch.class` (0..1023)
    /// this character has ever killed for the first time, backing
    /// `give_first_kill`'s per-class congrats gate.
    #[serde(default)]
    pub first_kill_ppd: Vec<u8>,
    /// C `struct arena_ppd` (`src/system/arena.c:204-211`, also declared
    /// identically in `game/ppd_structs.h:346-353`): the arena tournament
    /// ELO-like rating record (`score`/`fights`/`wins`/`losses`/
    /// `lastfight`), backing `IDR_TOPLIST` (`arena_toplist_lines`) and the
    /// (not yet ported) `score_fight` win/loss recording.
    #[serde(default)]
    pub arena_ppd: Vec<u8>,
    /// C `struct military_ppd` (`src/module/military.h:28-60`,
    /// `DRD_MILITARY_PPD`): mission-giver/advisor state plus the 5-slot
    /// mission offer table (`mis[5]`) and active-mission progress
    /// (`took_mission`/`solved_mission`). See
    /// [`LEGACY_MILITARY_PPD_SIZE`] for the byte layout.
    #[serde(default)]
    pub military_ppd: Vec<u8>,
    /// C `struct tunnel_ppd` (`src/area/33/tunnel.h:6-9`, also declared
    /// identically in `system/game/ppd_structs.h:629-632`): `{ int
    /// clevel; unsigned char used[204]; }`. `clevel` is a per-dungeon-
    /// entry scratch value (not yet meaningful without the unported
    /// tunnel dungeon runtime); `used[level]` is the completion counter
    /// `/tunnel` and `/tunnels` display, one byte per tunnel level
    /// (`MIN_TUNNEL_LEVEL..=MAX_TUNNEL_LEVEL`, i.e. `10..=200`). See
    /// [`LEGACY_TUNNEL_PPD_SIZE`] for the byte layout.
    #[serde(default)]
    pub tunnel_ppd: Vec<u8>,
    /// C `struct gorwin_ppd` (`src/area/33/tunnel.h:11-13`): `{ int
    /// tunnel_level; }`, the Gorwin NPC's currently-offered tunnel level
    /// (`0` means "not yet initialized" - see `initialize_gorwin_ppd`,
    /// not yet ported). See [`LEGACY_GORWIN_PPD_SIZE`].
    #[serde(default)]
    pub gorwin_ppd: Vec<u8>,
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
    /// C `struct stats_ppd` (`src/system/statistics.h`): a `MAXSTAT`(365)
    /// -day rolling ring buffer of daily exp/gold/online samples, stored
    /// as the raw legacy blob (see `encode_legacy_stats_ppd`/
    /// `decode_legacy_stats_ppd`, `stats_update`, `stats_online_time`).
    #[serde(default)]
    pub stats_ppd: Vec<u8>,
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
    /// C `struct AccountAchievements` PPD (`achievement.h:226-229`; see
    /// `crate::achievement` module doc for the account-vs-character scoping
    /// note). Per-unlock/progress storage for the 127-entry achievement
    /// table; kept separate from the pre-existing `achievements` field
    /// above (chest/transport exploration markers only) to avoid an
    /// unrelated refactor of that older, narrower model.
    #[serde(default)]
    pub achievement_data: AccountAchievements,
    /// C `struct AchievementStats` PPD (`achievement.h:232-276`): the
    /// running counters `achievement_add_*`/`achievement_check_*` update
    /// and `achievement_get_stat_progress` reads for progress-bar display.
    #[serde(default)]
    pub achievement_stats: AchievementStats,
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
    /// C `lostcon_ppd.autobless` (`command.c`'s `/autobless` toggle,
    /// `player_driver.c:1067`'s auto-rebless consumer - not yet wired,
    /// see `PORTING_TODO.md`).
    #[serde(default)]
    pub autobless_enabled: bool,
    /// C `lostcon_ppd.autopulse` (`command.c`'s `/autopulse` toggle,
    /// `player_driver.c:1070`'s auto-pulse consumer - not yet wired).
    #[serde(default)]
    pub autopulse_enabled: bool,
    /// C `lostcon_ppd.noball` (`command.c`'s `/noball` toggle): during the
    /// `CDR_LOSTCON` lag-simulation autopilot (`lostcon.c`, not yet
    /// ported), suppresses automatic Ball Lightning casting.
    #[serde(default)]
    pub no_ball: bool,
    /// C `lostcon_ppd.nobless` (`/nobless`).
    #[serde(default)]
    pub no_bless: bool,
    /// C `lostcon_ppd.nofireball` (`/nofireball`).
    #[serde(default)]
    pub no_fireball: bool,
    /// C `lostcon_ppd.noflash` (`/noflash`).
    #[serde(default)]
    pub no_flash: bool,
    /// C `lostcon_ppd.nofreeze` (`/nofreeze`).
    #[serde(default)]
    pub no_freeze: bool,
    /// C `lostcon_ppd.noheal` (`/noheal`).
    #[serde(default)]
    pub no_heal: bool,
    /// C `lostcon_ppd.noshield` (`/noshield`).
    #[serde(default)]
    pub no_shield: bool,
    /// C `lostcon_ppd.nowarcry` (`/nowarcry`).
    #[serde(default)]
    pub no_warcry: bool,
    /// C `lostcon_ppd.nolife` (`/nolife`).
    #[serde(default)]
    pub no_life: bool,
    /// C `lostcon_ppd.nomana` (`/nomana`).
    #[serde(default)]
    pub no_mana: bool,
    /// C `lostcon_ppd.nocombo` (`/nocombo`).
    #[serde(default)]
    pub no_combo: bool,
    /// C `lostcon_ppd.nomove` (`/nomove`).
    #[serde(default)]
    pub no_move: bool,
    /// C `lostcon_ppd.nopulse` (`/nopulse`).
    #[serde(default)]
    pub no_pulse: bool,
    /// C `lostcon_ppd.norecall` (`/norecall`).
    #[serde(default)]
    pub no_recall: bool,
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
    /// C `struct pent_debug_data`/`struct pentagram_player_data`
    /// (`command.c:1136-1143`, `area/4/pents.c:130-139`), stored at
    /// `DRD_PENT_NPPD`. Unlike every neighboring `_PPD` id in
    /// `drdata.h`, `DRD_PENT_NPPD` has no `PERSISTENT_PLAYER_DATA` bit,
    /// so C treats it as session-only scratch memory reset whenever the
    /// character isn't loaded; kept here as a plain `#[serde(default)]`
    /// field like the rest of `PlayerRuntime` since this port has no
    /// separate volatile-vs-persistent storage tier and the distinction
    /// has no observable effect on this debug-only feature. Backs the
    /// `/pentinfo`/`/setpentcount`/`/setpentstatus`/`/setpentbonus`/
    /// `/resetpent` GOD debug commands; the real Area 4 pentagram-solving
    /// gameplay (`pents.c`) that also reads/writes this same struct via
    /// `get_pent_data` is not yet ported (see `PORTING_TODO.md`'s Area 4
    /// task) - a future port of that gameplay should reuse these same
    /// fields rather than duplicating them.
    #[serde(default)]
    pub pentagram_debug: PentagramDebugData,
    /// C `struct macro_ppd` (`command.c:585-626`, backing `DRD_MACRO_PPD`,
    /// which - unlike `DRD_PENT_NPPD` - *does* carry the
    /// `PERSISTENT_PLAYER_DATA` bit). Mirrors every field of the real
    /// anti-macro/anti-bot "macro daemon" engine's per-player state
    /// (`src/module/base.c`'s `macro_driver`, ~800 lines: activity
    /// tracking, math/type-word/reverse/multiple-choice challenge
    /// generation and checking, reward/failure handling, and a
    /// cross-server "challenge room" teleport-and-restore flow) so this
    /// port's GOD/staff admin debug commands (`/macrostats`,
    /// `/macrohistory`, `/macrolist`, `/summonmacro`, `/macroimmune`,
    /// `/macrosuspicion`, `/macrokarma`, `/macrofailures`, `/macroreset`,
    /// `/macrohelp`, `command.c:660-1123`) can read and mutate the same
    /// storage a future port of `macro_driver` itself would need, rather
    /// than duplicating it - exactly the precedent already established by
    /// `pentagram_debug` above. That driver is NOT ported yet (no task
    /// exists for it in `PORTING_TODO.md` - add one before relying on any
    /// gameplay effect), so every field here stays at its `Default`
    /// (fresh/never-analyzed) value until this port gains code that
    /// actually challenges a player, which cannot happen today.
    #[serde(default)]
    pub macro_ppd: MacroPpd,
    /// C `struct depot_ppd { struct item itm[MAXDEPOT]; }`
    /// (`src/system/depot.h:19-23`, `DRD_DEPOT_PPD`): the character's own
    /// 80-slot legacy storage depot, opened via any item with the
    /// `IF_DEPOT` flag (`src/system/depot.c`'s `swap_depot`/
    /// `player_depot`/`depot_sort`). A distinct, older, per-character
    /// system from `ugaris-server::depot`'s account-wide
    /// `AccountDepotState`. See
    /// `encode_legacy_depot_ppd`/`decode_legacy_depot_ppd` for the byte
    /// layout (identical per-item layout to `AccountDepotState`'s own
    /// codec, since both persist the same C `struct item`).
    #[serde(default = "PlayerRuntime::default_depot")]
    pub depot: Vec<Option<Item>>,
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
            anticheat_session_id: None,
            ac_watch_enabled: false,
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
            first_kill_ppd: Vec::new(),
            arena_ppd: Vec::new(),
            military_ppd: Vec::new(),
            tunnel_ppd: Vec::new(),
            gorwin_ppd: Vec::new(),
            area3_ppd: Vec::new(),
            area1_ppd: Vec::new(),
            nomad_ppd: Vec::new(),
            caligar_ppd: Vec::new(),
            arkhata_ppd: Vec::new(),
            staffer_ppd: Vec::new(),
            farmy_ppd: Vec::new(),
            stats_ppd: Vec::new(),
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
            achievement_data: AccountAchievements::default(),
            achievement_stats: AchievementStats::default(),
            keyring_auto_add: false,
            current_section_id: 0,
            special_shrine_hcsc_last_touch_seconds: 0,
            transport_seen: 0,
            current_mirror_id: 0,
            max_lag_seconds: 0,
            hints_disabled: false,
            autoturn_enabled: false,
            autobless_enabled: false,
            autopulse_enabled: false,
            no_ball: false,
            no_bless: false,
            no_fireball: false,
            no_flash: false,
            no_freeze: false,
            no_heal: false,
            no_shield: false,
            no_warcry: false,
            no_life: false,
            no_mana: false,
            no_combo: false,
            no_move: false,
            no_pulse: false,
            no_recall: false,
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
            pentagram_debug: PentagramDebugData::default(),
            macro_ppd: MacroPpd::default(),
            depot: Self::default_depot(),
        }
    }

    pub(crate) fn default_depot() -> Vec<Option<Item>> {
        vec![None; MAXDEPOT]
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
                DRD_QUESTLOG_PPD => {
                    if !self.decode_legacy_questlog_ppd(block.data) {
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
                DRD_STATS_PPD => {
                    if !self.decode_legacy_stats_ppd(block.data) {
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
                DRD_FIRSTKILL_PPD => {
                    if !self.decode_legacy_firstkill_ppd(block.data) {
                        return false;
                    }
                }
                DRD_ARENA_PPD => {
                    if !self.decode_legacy_arena_ppd(block.data) {
                        return false;
                    }
                }
                DRD_MILITARY_PPD => {
                    if !self.decode_legacy_military_ppd(block.data) {
                        return false;
                    }
                }
                DRD_TUNNEL_PPD => {
                    if !self.decode_legacy_tunnel_ppd(block.data) {
                        return false;
                    }
                }
                DRD_GORWIN_PPD => {
                    if !self.decode_legacy_gorwin_ppd(block.data) {
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
                DRD_DEPOT_PPD => {
                    if !self.decode_legacy_depot_ppd(block.data) {
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
        let mut had_questlog = false;
        let mut had_caligar = false;
        let mut had_arkhata = false;
        let mut had_staffer = false;
        let mut had_farmy = false;
        let mut had_stats = false;
        let mut had_teufelrat = false;
        let mut had_bank = false;
        let mut had_twocity = false;
        let mut had_saltmine = false;
        let mut had_treasure_dig = false;
        let mut had_misc = false;
        let mut had_firstkill = false;
        let mut had_arena = false;
        let mut had_military = false;
        let mut had_tunnel = false;
        let mut had_gorwin = false;
        let mut had_rune = false;
        let mut had_alias = false;
        let mut had_ignore = false;
        let mut had_swear = false;
        let mut had_depot = false;
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
            } else if block.id == DRD_QUESTLOG_PPD {
                had_questlog = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_QUESTLOG_PPD,
                    &self.encode_legacy_questlog_ppd(),
                );
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
            } else if block.id == DRD_STATS_PPD {
                had_stats = true;
                write_ppd_block(&mut encoded, DRD_STATS_PPD, &self.encode_legacy_stats_ppd());
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
            } else if block.id == DRD_FIRSTKILL_PPD {
                had_firstkill = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_FIRSTKILL_PPD,
                    &self.encode_legacy_firstkill_ppd(),
                );
            } else if block.id == DRD_ARENA_PPD {
                had_arena = true;
                write_ppd_block(&mut encoded, DRD_ARENA_PPD, &self.encode_legacy_arena_ppd());
            } else if block.id == DRD_MILITARY_PPD {
                had_military = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_MILITARY_PPD,
                    &self.encode_legacy_military_ppd(),
                );
            } else if block.id == DRD_TUNNEL_PPD {
                had_tunnel = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_TUNNEL_PPD,
                    &self.encode_legacy_tunnel_ppd(),
                );
            } else if block.id == DRD_GORWIN_PPD {
                had_gorwin = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_GORWIN_PPD,
                    &self.encode_legacy_gorwin_ppd(),
                );
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
            } else if block.id == DRD_DEPOT_PPD {
                had_depot = true;
                write_ppd_block(&mut encoded, DRD_DEPOT_PPD, &self.encode_legacy_depot_ppd());
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
            && (self.max_lag_seconds != 0
                || self.hints_disabled
                || self.autoturn_enabled
                || self.has_nondefault_lag_control_toggle())
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
        if !had_questlog
            && (existing_was_valid || existing.is_empty())
            && self
                .quest_log
                .entries()
                .iter()
                .any(|entry| entry.done != 0 || entry.flags != 0)
        {
            write_ppd_block(
                &mut encoded,
                DRD_QUESTLOG_PPD,
                &self.encode_legacy_questlog_ppd(),
            );
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
        if !had_stats && (existing_was_valid || existing.is_empty()) && !self.stats_ppd.is_empty() {
            write_ppd_block(&mut encoded, DRD_STATS_PPD, &self.encode_legacy_stats_ppd());
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
        if !had_firstkill
            && (existing_was_valid || existing.is_empty())
            && !self.first_kill_ppd.is_empty()
        {
            write_ppd_block(
                &mut encoded,
                DRD_FIRSTKILL_PPD,
                &self.encode_legacy_firstkill_ppd(),
            );
        }
        if !had_arena && (existing_was_valid || existing.is_empty()) && !self.arena_ppd.is_empty() {
            write_ppd_block(&mut encoded, DRD_ARENA_PPD, &self.encode_legacy_arena_ppd());
        }
        if !had_military
            && (existing_was_valid || existing.is_empty())
            && !self.military_ppd.is_empty()
        {
            write_ppd_block(
                &mut encoded,
                DRD_MILITARY_PPD,
                &self.encode_legacy_military_ppd(),
            );
        }
        if !had_tunnel && (existing_was_valid || existing.is_empty()) && !self.tunnel_ppd.is_empty()
        {
            write_ppd_block(
                &mut encoded,
                DRD_TUNNEL_PPD,
                &self.encode_legacy_tunnel_ppd(),
            );
        }
        if !had_gorwin && (existing_was_valid || existing.is_empty()) && !self.gorwin_ppd.is_empty()
        {
            write_ppd_block(
                &mut encoded,
                DRD_GORWIN_PPD,
                &self.encode_legacy_gorwin_ppd(),
            );
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
        if !had_depot
            && (existing_was_valid || existing.is_empty())
            && self.depot.iter().any(Option::is_some)
        {
            write_ppd_block(&mut encoded, DRD_DEPOT_PPD, &self.encode_legacy_depot_ppd());
        }

        encoded
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
        // A reconnect is a brand-new physical connection (C `ac_player_
        // connect`/`ac_player_login` re-runs from scratch on every login),
        // so any anti-cheat session tied to the previous connection must
        // not be carried over; the new login path creates a fresh one.
        self.anticheat_session_id = None;
        self.ac_watch_enabled = false;
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
}
