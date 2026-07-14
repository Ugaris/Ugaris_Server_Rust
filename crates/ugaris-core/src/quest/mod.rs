mod init;
mod log;
mod table;

pub use init::*;
pub use log::*;
pub use table::*;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

pub const MAX_QUESTS: usize = 100;

pub const QF_OPEN: u8 = 1;

pub const QF_DONE: u8 = 2;

pub const QLF_REPEATABLE: u8 = 1 << 0;

pub const QLF_XREPEAT: u8 = 1 << 1;

pub const QLOG_LYDIA: usize = 0;

pub const QLOG_GWENDY_FIRST_SKULL: usize = 1;

pub const QLOG_GWENDY_SECOND_SKULL: usize = 2;

pub const QLOG_GWENDY_THIRD_SKULL: usize = 3;

pub const QLOG_GWENDY_FOUL_MAGICIAN: usize = 4;

/// C `questlog_open(co, 5)`/`questlog_done(co, 5)` (`src/area/1/
/// gwendylon.c:1063,1154`, `yoakin_driver`'s big-mother-bear-tooth quest).
/// No `#define QLOG_*` name exists for index 5 in `questlog.h` - C itself
/// only ever spells it out as the bare literal `5`.
pub const QLOG_YOAKIN: usize = 5;

pub const QLOG_NOOK: usize = 6;

pub const QLOG_JESSICA_ROBBER_NOTE: usize = 79;

pub const QLOG_JIU: usize = 80;

pub const QLOG_BRITHILDIE: usize = 81;

pub const QLOG_HERMIT_QUEST1: usize = 82;

pub const QLOG_HERMIT_QUEST2: usize = 83;

pub const QLOG_JESSICA_KILL: usize = 84;

/// C `questlog_open(co, 17)`/`questlog_done(co, 17)` (`src/area/1/
/// gwendylon.c:4216,4237`, `reskin_driver`'s "The Unwanted Tenants"
/// quest). No `#define QLOG_*` name exists for index 17 in
/// `questlog.h` - C itself only ever spells it out as the bare literal
/// `17`.
pub const QLOG_RESKIN: usize = 17;

/// C `struct questlog` (`src/system/questlog.c:98-105`): the static quest
/// metadata table entry (name/level-range/giver/area/nominal exp/flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestMeta {
    pub name: &'static str,
    pub min_level: u8,
    pub max_level: u8,
    pub giver: &'static str,
    pub area: &'static str,
    /// Nominal exp reward (C `questlog[qnr].exp`). `0` marks quests whose
    /// exp is awarded ad hoc by the driver instead (documented per entry
    /// below, matching the C source comments).
    pub exp: i64,
    pub flags: u8,
}

pub fn quest_meta(qnr: usize) -> Option<&'static QuestMeta> {
    QUEST_TABLE.get(qnr)
}

const QUESTLOG_FLAGS: [u8; MAX_QUESTS] = {
    let mut flags = [0u8; MAX_QUESTS];
    let mut i = 0;
    while i < QUEST_TABLE.len() {
        flags[i] = QUEST_TABLE[i].flags;
        i += 1;
    }
    flags
};

/// C `questlog_scale(cnt, ex)` (`src/system/questlog.c:240-265`): the
/// repeat-completion exp decay curve. `cnt` is the number of times the
/// quest had already been completed *before* this completion (C's
/// post-increment `quest[qnr].done++` read).
pub fn scale_exp(prior_completions: u8, base_exp: i64) -> i64 {
    match prior_completions {
        0 => base_exp,
        1 => base_exp * 82 / 100,
        2 => base_exp * 68 / 100,
        3 => base_exp * 56 / 100,
        4 => base_exp * 46 / 100,
        5 => base_exp * 38 / 100,
        6 => base_exp * 32 / 100,
        7 => base_exp * 26 / 100,
        8 => base_exp * 21 / 100,
        9 => base_exp * 18 / 100,
        _ => base_exp * 15 / 100,
    }
}

/// C `questlog_done`'s level-based taper (`src/system/questlog.c:286-295`):
/// "scale down by level for those rushing ahead". `level_value` must be the
/// caller's `ugaris_core::world::level_value(level)` result - this leaf
/// module intentionally takes it as a parameter instead of depending on
/// `world::exp` to avoid a `quest` -> `world` module dependency.
pub fn taper_exp_by_level(level: u32, level_value: u32, scaled_exp: i64) -> i64 {
    let level_value = i64::from(level_value);
    if level > 44 {
        scaled_exp.min(level_value / 6)
    } else if level > 19 {
        scaled_exp.min(level_value / 4)
    } else if level > 4 {
        scaled_exp.min(level_value / 2)
    } else {
        scaled_exp.min(level_value)
    }
}
