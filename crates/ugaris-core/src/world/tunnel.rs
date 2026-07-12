//! Area 33 (`src/area/33/tunnel.c`) reward math: C `give_reward`
//! (`:527-601`), the `IDR_TUNNELDOOR` exit-pillar payout called from
//! `tunneldoor`'s `DOOR_EXIT_EXP`/`DOOR_EXIT_MILITARY` branches
//! (`:630-636`, `item_driver::area33_tunnel::tunneldoor_driver`).
//!
//! `give_reward` reads/writes both `tunnel_ppd::used[]` and
//! `gorwin_ppd::tunnel_level` - `PlayerRuntime`-only state `World` cannot
//! see - so this follows the same snapshot-in/events-out split already
//! established by `world::npc::area33::gorwin` (`GorwinPlayerFacts`/
//! `GorwinOutcomeEvent`): [`TunnelRewardFacts`] is the caller's
//! (`ugaris-server`) pre-fetched snapshot, [`TunnelRewardOutcome`] is what
//! the caller still needs to write back to `PlayerRuntime` plus the
//! player-facing message lines. The `give_exp`/`give_military_pts_no_npc`
//! payout itself *is* applied directly here, since both are plain `World`
//! methods.

use super::*;
use crate::player::{find_next_available_tunnel_level, MAX_TUNNEL_USES};

/// C `enum TunnelDoorType` (`src/area/33/tunnel.h:16`), duplicated from
/// `item_driver::area33_tunnel` (that module's consts are private to
/// `item_driver`, and pulling in the whole `item_driver` module here for
/// two bytes isn't worth it).
const DOOR_EXIT_EXP: u8 = 2;

/// C `DOOR_RANGE`/`DOOR_DEPTH` (`src/area/33/tunnel.h:31-32`), duplicated
/// from `item_driver::area33_tunnel` for the same reason as
/// [`DOOR_EXIT_EXP`] - `check_area_clear` needs `self.map`/
/// `self.characters`, which only `World` (this module) has access to.
const DOOR_RANGE: u16 = 4;
const DOOR_DEPTH: u16 = 20;

/// Snapshot of the `PlayerRuntime` fields C `give_reward` reads, matching
/// `world::npc::area33::gorwin::GorwinPlayerFacts`'s shape.
#[derive(Debug, Clone)]
pub struct TunnelRewardFacts {
    /// `gorwin_ppd::tunnel_level` (`PlayerRuntime::gorwin_tunnel_level`).
    pub reward_level: i32,
    /// `tunnel_ppd::used[]` (`PlayerRuntime::tunnel_used`), indexed by
    /// level directly, same shape as `GorwinPlayerFacts::tunnel_used`.
    pub tunnel_used: Vec<u8>,
}

impl TunnelRewardFacts {
    fn used_at(&self, level: i32) -> u8 {
        if level < 0 {
            return 0;
        }
        self.tunnel_used.get(level as usize).copied().unwrap_or(0)
    }
}

/// What [`World::apply_tunnel_reward`] could not apply directly, for
/// `ugaris-server` to finish (`PlayerRuntime` writes, player-facing
/// feedback delivery, and the `achievement_add_tunnel_level` DB/unlock
/// wiring `World` has no access to).
#[derive(Debug, Clone, Default)]
pub struct TunnelRewardOutcome {
    /// C `log_char(cn, LOG_SYSTEM, 0, ...)` lines, in call order. Color
    /// markers (`COL_HEADING`/`COL_YELLOW`/`COL_RESET`) around "Tunnel
    /// Mastery!"/the promoted level number are dropped - this outcome's
    /// plain `String` messages have no raw-byte counterpart to carry them
    /// in, matching the `dispatch_minewall_outcome`/`dispatch_lab_outcome`
    /// precedent for plain-text item-use feedback (documented deviation,
    /// same family as `WorldAreaText.message`'s).
    pub messages: Vec<String>,
    /// `PlayerRuntime::set_tunnel_used(level, value)` to apply, if the
    /// reward was actually granted (`used[reward_level] < MAX_TUNNEL_USES`
    /// on entry).
    pub new_used_count: Option<(i32, u8)>,
    /// `PlayerRuntime::set_gorwin_tunnel_level(next)` to apply, on either
    /// of C's two auto-promote branches (all-uses-exhausted-just-now, or
    /// already-maxed-on-entry).
    pub promote_gorwin_to: Option<i32>,
    /// Whether `achievement_add_tunnel_level(cn)` should fire (only when a
    /// reward was actually granted).
    pub award_achievement: bool,
}

impl World {
    /// C `give_reward` (`src/area/33/tunnel.c:527-601`). `door_type` is
    /// the raw `it[in].drdata[0]` (`DOOR_EXIT_EXP` or `DOOR_EXIT_MILITARY`
    /// - any other value is a no-op, matching C's `if/else if` with no
    /// `else` branch).
    pub fn apply_tunnel_reward(
        &mut self,
        character_id: CharacterId,
        facts: &TunnelRewardFacts,
        door_type: u8,
        area_id: u32,
    ) -> TunnelRewardOutcome {
        let mut outcome = TunnelRewardOutcome::default();
        let reward_level = facts.reward_level;
        let char_level = self
            .characters
            .get(&character_id)
            .map(|character| character.level as i32)
            .unwrap_or(0);
        let used_before = facts.used_at(reward_level);

        if used_before < MAX_TUNNEL_USES {
            // C `ppd->used[reward_level]++;` (`:540`) - the reward-value
            // formulas below read the *post*-increment count.
            let used_after = used_before + 1;
            outcome.new_used_count = Some((reward_level, used_after));

            if door_type == DOOR_EXIT_EXP {
                // C `value = level_value(reward_level) /
                // tunnel_exp_base_value_divider / (ppd->used[reward_level]
                // + 9);` (`:543`) - fully double-precision until the
                // final `(int)value` assignment.
                let divider = self.settings.tunnel_exp_base_value_divider;
                let raw = f64::from(level_value(reward_level.max(0) as u32)) / divider;
                let value = (raw / (f64::from(used_after) + 9.0)) as i64;
                outcome
                    .messages
                    .push("You have been given experience.".to_string());
                self.give_exp(character_id, value, area_id);
            } else {
                // C `value = (tunnel_mill_exp_base_value + (reward_level *
                // reward_level / 10)) / (ppd->used[reward_level] + 9);`
                // (`:550`) - all-integer.
                let base = self.settings.tunnel_mill_exp_base_value;
                let value =
                    (base + (reward_level * reward_level / 10)) / (i32::from(used_after) + 9);
                outcome
                    .messages
                    .push("You have been given military rank.".to_string());
                self.give_military_pts(character_id, value, 1, area_id);
            }
            outcome.award_achievement = true;

            // C `if (ppd->used[reward_level] >= MAX_TUNNEL_USES) { ... }
            // else { ... }` (`:560-586`).
            if used_after >= MAX_TUNNEL_USES {
                outcome.messages.push(format!(
                    "Tunnel Mastery! Thou hast conquered all {MAX_TUNNEL_USES} challenges at level {reward_level}."
                ));
                match find_next_available_tunnel_level(&facts.tunnel_used, reward_level, char_level)
                {
                    Some(next) => {
                        outcome.promote_gorwin_to = Some(next);
                        outcome.messages.push(format!(
                            "Gorwin has advanced thy tunnel level to {next}. Onward and upward!"
                        ));
                    }
                    None => {
                        outcome.messages.push(
                            "There are no more tunnel levels available to thee. Thou art a true master of the depths!"
                                .to_string(),
                        );
                    }
                }
            } else {
                let remaining = MAX_TUNNEL_USES - used_after;
                outcome.messages.push(format!(
                    "Completions at level {reward_level}: {used_after}/{MAX_TUNNEL_USES} ({remaining} remaining)."
                ));
            }
        } else {
            // C `else { log_char(...); int next_level = ...; if
            // (next_level) { ... } }` (`:587-599`).
            outcome.messages.push(format!(
                "You have used all {MAX_TUNNEL_USES} completions at level {reward_level}. No reward given."
            ));
            if let Some(next) =
                find_next_available_tunnel_level(&facts.tunnel_used, reward_level, char_level)
            {
                outcome.promote_gorwin_to = Some(next);
                outcome.messages.push(format!(
                    "Gorwin has advanced thy tunnel level to {next}. Speak with him for details."
                ));
            }
        }

        outcome
    }

    /// C `check_area_clear(in)` (`src/area/33/tunnel.c:750-762`): scans the
    /// `DOOR_RANGE`-wide, `DOOR_DEPTH`-deep rectangle in front of a
    /// `IDR_TUNNELDOOR2` "mean door" (`x` ± `DOOR_RANGE`, `y+1` through
    /// `y+DOOR_DEPTH-1`) for any non-player character. Out-of-bounds tiles
    /// are skipped (C's raw `map[x+y*MAXMAP]` indexing has no such bounds
    /// check, but every real door placement keeps this rectangle on-map).
    pub(crate) fn tunnel_mean_door_area_clear(&self, x: u16, y: u16) -> bool {
        let x_start = x.saturating_sub(DOOR_RANGE);
        let x_end = x.saturating_add(DOOR_RANGE);
        let y_start = y.saturating_add(1);
        let y_end = y.saturating_add(DOOR_DEPTH);
        for ty in y_start..y_end {
            for tx in x_start..=x_end {
                let Some(tile) = self.map.tile(usize::from(tx), usize::from(ty)) else {
                    continue;
                };
                if tile.character == 0 {
                    continue;
                }
                let occupant_is_player = self
                    .characters
                    .get(&CharacterId(u32::from(tile.character)))
                    .is_some_and(|character| character.flags.contains(CharacterFlags::PLAYER));
                if !occupant_is_player {
                    return false;
                }
            }
        }
        true
    }
}
