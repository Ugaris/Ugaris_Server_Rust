//! Achievement system core data model and pure logic
//! (`src/module/achievements/achievement.c`,
//! `src/module/achievements/achievement.h` in the legacy C server).
//!
//! This module ports the *data model and stat-driven award logic* of the
//! legacy achievement system as a leaf module with no access to
//! `World`/`PlayerRuntime`/networking, matching the pattern used by
//! `crate::quest` before it was wired into live call sites. What is ported
//! here: the full 127-entry `AchievementType` enum and `achievement_defs`
//! table (`achievement.c:44-326`, copied digit for digit including Steam
//! API id strings, display names/descriptions, categories and progress
//! targets), the `Achievement`/`AccountAchievements`/`AchievementStats`
//! storage structs (`achievement.h:217-276`), and every stat-update /
//! award-check function (`achievement_add_flowers` .. `achievement_add_
//! play_time`, `achievement_check_login_streak`, `achievement_check_level`,
//! `achievement_check_skill`, `achievement_check_profession`,
//! `achievement_check_exploration`, `achievement_get_stat_progress`,
//! `achievement_area_to_pent_index`, `achievement_clear_all`) as pure
//! functions returning the list of newly-unlocked achievements for the
//! caller to route through logging/Steam-sync/DB "first unlock" side
//! effects it can't reach from here.
//!
//! NOT ported yet (left for the caller-side wiring task, tracked as
//! REMAINING on the "Achievements" P3 `PORTING_TODO.md` entry):
//! `achievement_send_to_client`/`achievement_sync_all` (the `SV_ACH_*`
//! mod-packet wire format from `mod_achievements.h` - no Rust protocol
//! definitions exist for it yet), `db_achievement_record_unlock`/the
//! "first player globally" DB tracking and cross-server `server_chat`
//! announcement (`database_achievement.c`), `achievement_list`/
//! `achievement_show_stats`/`achievement_fix_all` (text-formatting
//! functions that belong in `ugaris-server`'s command layer once the
//! `/achievements`/`/achstats`/`/achfix`/`/achclear`/`/achsync`/`/achgive`
//! commands - currently help-text-only stubs in `commands_player.rs` - are
//! wired up), and persistence (no PPD/DB column exists yet for
//! `AccountAchievements`/`AchievementStats`; `crate::player`'s existing
//! `AchievementState` is a small pre-existing ad hoc subset - chests +
//! transport exploration markers only - and is left untouched by this
//! change to avoid an unrelated refactor). No call site anywhere in the
//! Rust tree constructs or mutates `AccountAchievements`/`AchievementStats`
//! yet; this module is data-and-logic-only until that wiring lands.

mod defs;
mod state;
mod stats;
mod types;

pub use defs::*;
pub use state::*;
pub use stats::*;
pub use types::*;

#[cfg(test)]
// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#[allow(clippy::field_reassign_with_default)]
mod tests;
