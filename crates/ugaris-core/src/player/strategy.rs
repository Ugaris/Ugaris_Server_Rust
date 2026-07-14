//! Areas 23/24 real-time-strategy minigame persistent player state.
//!
//! See `crate::world::strategy`'s module doc comment for the C source
//! reference, the ported/remaining slice breakdown, and the mission/
//! AI-preset content tables this struct's fields are indexed by.

use serde::{Deserialize, Serialize};

use crate::world::STRATEGY_MAXMISSION;

/// C `struct strategy_ppd` (`src/area/23_24/strategy.c:117-138`, backing
/// `DRD_STRATEGY_PPD`): a player's persistent Areas 23/24 strategy-game
/// profile.
///
/// `max_worker`/`max_level`/`trainspeed`/`income`/`endurance`/`warcry`/
/// `speed`/`eguardlvl` are upgrade levels raised with strategy exp via
/// `crate::world::str_raise` (not wired to any player-facing command
/// yet - that's the still-unported `strategy_boss` NPC dialogue driver's
/// job). `eguards` is a running count of "eternal guard" slots earned
/// (`reward_winner`, `strategy.c:445`), separate from `eguardlvl` (the
/// level cap raised guards spawn at). `exp`/`boss_exp`/`boss_msg_exp`/
/// `mis_cnt`/`won_cnt` are running totals; `current_mission` indexes
/// `crate::world::MISSIONS` (0 = none in progress). `npc_color` is the
/// player's chosen banner/name color for their spawned workers.
/// `boss_stage`/`boss_timer`/`init_done` back the still-unported
/// `strategy_boss` dialogue state machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StrategyPpd {
    pub max_worker: i32,
    pub max_level: i32,
    pub trainspeed: i32,
    pub income: i32,
    pub endurance: i32,
    pub warcry: i32,
    pub speed: i32,
    pub eguards: i32,
    pub eguardlvl: i32,
    pub exp: i32,
    pub boss_exp: i32,
    pub boss_msg_exp: i32,
    pub mis_cnt: i32,
    pub won_cnt: i32,
    pub current_mission: i32,
    pub npc_color: i32,
    pub boss_stage: i32,
    pub boss_timer: i32,
    pub init_done: i32,
    /// C `unsigned char solve_cnt[MAXMISSION]` (`MAXMISSION` = 64, see
    /// [`crate::world::STRATEGY_MAXMISSION`]): how many times each
    /// mission chain's `set_solve` milestone has been won. Stored as a
    /// lazily-grown `Vec<u8>` (only indices `0..=12`,
    /// `crate::world::MISSIONS`'s `set_solve` values, are ever written in
    /// practice) rather than a fixed 64-element array; use
    /// [`Self::solve_count`]/[`Self::increment_solve_count`] rather than
    /// indexing directly.
    #[serde(default)]
    pub solve_cnt: Vec<u8>,
}

impl StrategyPpd {
    /// C `ppd->solve_cnt[idx]`. Out-of-range/never-written indices read
    /// as `0`, matching C's zero-initialized array.
    pub fn solve_count(&self, idx: usize) -> u8 {
        self.solve_cnt.get(idx).copied().unwrap_or(0)
    }

    /// C `ppd->solve_cnt[mission[n].set_solve]++` (`reward_winner`,
    /// `strategy.c:446`). `idx` is clamped into `0..STRATEGY_MAXMISSION`
    /// like every other legacy fixed-array PPD accessor in this port, and
    /// the counter saturates instead of wrapping (C's `unsigned char`
    /// would wrap at 256, a practically unreachable win count for a
    /// single mission chain).
    pub fn increment_solve_count(&mut self, idx: usize) {
        let idx = idx.min(STRATEGY_MAXMISSION - 1);
        if self.solve_cnt.len() <= idx {
            self.solve_cnt.resize(idx + 1, 0);
        }
        self.solve_cnt[idx] = self.solve_cnt[idx].saturating_add(1);
    }
}
