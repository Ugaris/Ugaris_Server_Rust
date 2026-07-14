use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaligarSkellyDeathResult {
    Unmapped { x: u16, y: u16 },
    AlreadyUnlocked { door_index: u8, bit: u8 },
    PartiallyUnlocked { door_index: u8, bit: u8 },
    FullyUnlocked { door_index: u8, bit: u8 },
}

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
