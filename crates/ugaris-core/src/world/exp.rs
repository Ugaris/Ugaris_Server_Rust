//! Experience/level math and the level-up side effects.
//!
//! Ports `exp2level`/`level2exp`/`level_value` (`src/system/tool.c:1272-1283`)
//! and `check_levelup` (`src/system/tool.c:1318-1356`) from the legacy C
//! server. These pure formulas used to be duplicated in three different
//! spots across the workspace (`ugaris-server/src/spawns.rs`,
//! `ugaris-server/src/area_apply.rs`, `ugaris-core/src/item_driver/
//! helpers.rs`) - this module is now the single canonical copy; all three
//! former duplicates delegate here.
//!
//! C `give_exp` (`tool.c:1371-1423`) itself stays split across crates: the
//! two runtime-tunable multipliers (`exp_modifier`, `hardcore_exp_bonus`)
//! are live-adjustable via admin commands and live on `ServerRuntime` in the
//! server crate (not available to `ugaris-core`), so the full `give_exp`
//! wrapper that applies them stays in
//! `ugaris-server/src/commands_admin.rs::give_exp_with_runtime_modifiers`,
//! which now calls `World::check_levelup` after updating `exp` - matching
//! C's `if (!(ch[cn].flags & CF_NOLEVEL)) check_levelup(cn);` tail call.

use super::*;

/// Queued `achievement_check_level(cn, level)` check (`tool.c:1352-1354`),
/// fired once per level gained inside `check_levelup`'s loop, gated on
/// `CharacterFlags::PLAYER` matching C's `if (ch[cn].flags & CF_PLAYER)`
/// guard. The server crate drains this and applies the actual
/// `AccountAchievements`/`PlayerRuntime` state update (`ugaris-core` has no
/// access to `PlayerRuntime`), mirroring the `KillAchievementAward`/
/// `FirstKillCheck` queue pattern in `world/death.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LevelAchievementCheck {
    pub character_id: CharacterId,
    pub level: u32,
    pub is_hardcore: bool,
}

/// C `exp2level(val)` (`src/system/tool.c:1272`):
/// `max(1, (int)(sqrt(sqrt(val))))`.
pub fn exp2level(exp: u32) -> u32 {
    (exp as f64).sqrt().sqrt().floor().max(1.0) as u32
}

/// C `level2exp(level)` (`src/system/tool.c:1277`): `pow(level, 4)`.
pub fn level2exp(level: u32) -> u32 {
    level.saturating_pow(4)
}

/// C `level_value(level)` (`src/system/tool.c:1282`):
/// `pow(level + 1, 4) - pow(level, 4)`.
pub fn level_value(level: u32) -> u32 {
    let next = level.saturating_add(1);
    next.saturating_pow(4)
        .saturating_sub(level.saturating_pow(4))
}

/// C `level2maxitem(level)` (`src/system/tool.c:2516-2577`): the "max item
/// modifier tier a player of this level should reasonably own" step
/// function, used to scale special/computed equipment bonuses (e.g. the
/// dungeon-guard `equip1`/`equip2` spell items in `area/13/dungeon.c`'s
/// `build_warrior`/`build_mage`/`build_seyan`). Ported as an explicit
/// ascending threshold ladder identical to the C `if (level < N) return
/// V;` chain, including its final unconditional `return 20`.
pub fn level2maxitem(level: i32) -> i32 {
    const THRESHOLDS: [(i32, i32); 19] = [
        (2, 0),
        (3, 1),
        (5, 2),
        (10, 3),
        (15, 4),
        (17, 5),
        (20, 6),
        (23, 7),
        (26, 8),
        (30, 9),
        (33, 10),
        (36, 11),
        (40, 12),
        (43, 13),
        (46, 14),
        (50, 15),
        (53, 16),
        (56, 17),
        (60, 18),
    ];
    for (bound, value) in THRESHOLDS {
        if level < bound {
            return value;
        }
    }
    if level < 63 {
        return 19;
    }
    20
}

impl World {
    /// C `give_exp(cn, val)` (`src/system/tool.c:1371-1423`): the canonical
    /// experience-grant entry point. Applies the hardcore/global exp
    /// multipliers (`self.settings.hardcore_exp_bonus`/`exp_modifier`),
    /// respects `CF_NOEXP` and the area-21 (arena) exp-disabled zone,
    /// clamps `CF_NOLEVEL` characters to their current level's exp band,
    /// prevents an unexpected decrease from a positive grant, and calls
    /// `check_levelup` unless `CF_NOLEVEL` is set - matching the C function
    /// line for line.
    ///
    /// This is the single copy for use both from server-crate call sites
    /// (which previously duplicated this logic in
    /// `ugaris-server/src/commands_admin.rs::give_exp_with_runtime_modifiers`,
    /// now a thin wrapper) and from `ugaris-core` item drivers, which only
    /// have `&mut World` (not `ServerRuntime`) available.
    ///
    /// C's trailing `if (addedExp > 0) macro_track_exp_gain(cn)` is
    /// queued via `pending_exp_gain_events` (`World` has no access to the
    /// session-owned `PlayerRuntime` that owns `MacroPpd`) for
    /// `ugaris-server`'s `apply_macro_activity_events` to drain and stamp.
    pub fn give_exp(&mut self, character_id: CharacterId, base_exp: i64, area_id: u32) {
        let Some(character) = self.characters.get(&character_id) else {
            return;
        };

        let mut added_exp = base_exp as f64;
        if character.flags.contains(CharacterFlags::HARDCORE) {
            added_exp *= self.settings.hardcore_exp_bonus;
        }
        added_exp *= self.settings.exp_modifier;
        let added_exp = added_exp as i64;

        if character.flags.contains(CharacterFlags::NOEXP) || area_id == 21 {
            return;
        }

        let current_exp = i64::from(character.exp);
        let mut new_exp = current_exp.saturating_add(added_exp);

        let no_level = character.flags.contains(CharacterFlags::NOLEVEL);
        if no_level {
            let current_level_exp = i64::from(level2exp(character.level));
            let next_level_exp = i64::from(level2exp(character.level.saturating_add(1)));
            if new_exp >= next_level_exp {
                new_exp = next_level_exp.saturating_sub(1);
            } else if new_exp < current_level_exp {
                new_exp = current_level_exp;
            }
        }

        new_exp = new_exp.clamp(i64::from(i32::MIN), i64::from(i32::MAX));

        if new_exp < current_exp && added_exp > 0 {
            new_exp = current_exp;
        }

        let Some(character) = self.characters.get_mut(&character_id) else {
            return;
        };
        character.exp = new_exp.clamp(0, i64::from(u32::MAX)) as u32;
        character.flags.insert(CharacterFlags::UPDATE);

        if !no_level {
            self.check_levelup(character_id);
        }

        if added_exp > 0 {
            self.pending_exp_gain_events.push(character_id);
        }
    }

    /// Drains the queue [`Self::give_exp`] fills for every character that
    /// just gained a positive amount of experience - see
    /// `pending_exp_gain_events`'s doc comment (`world/mod.rs`).
    pub fn drain_exp_gain_events(&mut self) -> Vec<CharacterId> {
        self.pending_exp_gain_events.drain(..).collect()
    }

    /// C `check_levelup(cn)` (`src/system/tool.c:1318-1356`): loop while
    /// `exp2level(max(exp, exp_used)) > level`, granting one level per
    /// iteration. Returns whether any level was gained (C's `flag`).
    ///
    /// Ported per iteration: the level increment, the "Thou gained a
    /// level!" text, save-count grant/reset (hardcore resets `saves` to 0;
    /// everyone else gets `saves + 1` capped at 10, with the two matching
    /// feedback lines), the level-20 profession unlock (`value[1]
    /// [V_PROFESSION] = 1`) with its text, the level-10-multiple
    /// server-wide "Grats: NAME is level N now!" channel-6 broadcast (C
    /// `server_chat(6, ...)`, queued via `queue_channel_broadcast` -
    /// `ugaris-server`'s tick loop drains it and fans it out to every
    /// session whose `PlayerRuntime::chat_channels` has channel 6 joined,
    /// matching `apply_chat_command`'s channel delivery rule), and the map
    /// dirty-sector refresh (C `set_sector`).
    ///
    /// `achievement_check_level(cn, level)` (`CF_PLAYER`-gated) is queued
    /// via [`LevelAchievementCheck`] for the server crate to drain and
    /// apply (`ugaris-core` has no access to `PlayerRuntime`'s achievement
    /// state).
    ///
    /// Documented gaps (not silently dropped, matching C `check_levelup`
    /// exactly otherwise):
    /// - `reset_name(cn)` (name-color-by-level refresh) is unported;
    /// - `dlog(cn, 0, "gained a level")` debug-log call is skipped (no Rust
    ///   `dlog` sink exists).
    pub fn check_levelup(&mut self, character_id: CharacterId) -> bool {
        let mut leveled = false;
        loop {
            let Some(character) = self.characters.get(&character_id) else {
                break;
            };
            let experience = character.exp.max(character.exp_used);
            if exp2level(experience) <= character.level {
                break;
            }

            let Some(character) = self.characters.get_mut(&character_id) else {
                break;
            };
            leveled = true;
            character.level += 1;
            let level = character.level;
            let (x, y) = (character.x, character.y);
            let is_hardcore = character.flags.contains(CharacterFlags::HARDCORE);
            let is_player = character.flags.contains(CharacterFlags::PLAYER);

            let mut messages = vec![format!("Thou gained a level! Thou art level {level} now.")];

            if is_hardcore {
                character.saves = 0;
            } else {
                character.saves = character.saves.saturating_add(1).min(10);
                messages.push(
                    "Thy persistence has pleased Ishtar. He will save thee when thou art in need."
                        .to_string(),
                );
                messages.push(format!(
                    "Thou hast {} saves now.",
                    legacy_save_number(character.saves)
                ));
            }

            if level >= 20 {
                let profession_idx = CharacterValue::Profession as usize;
                let has_profession = character
                    .values
                    .get(1)
                    .and_then(|values| values.get(profession_idx))
                    .copied()
                    .unwrap_or_default()
                    != 0;
                if !has_profession {
                    if let Some(slot) = character
                        .values
                        .get_mut(1)
                        .and_then(|values| values.get_mut(profession_idx))
                    {
                        *slot = 1;
                    }
                    messages.push("Thou mayest now choose to learn a profession.".to_string());
                }
            }

            if level % 10 == 0 {
                let mut broadcast = b"0000000000".to_vec();
                broadcast.extend_from_slice(crate::text::COL_CHAT_GRATS);
                broadcast.extend_from_slice(
                    format!("Grats: {} is level {level} now!", character.name).as_bytes(),
                );
                self.queue_channel_broadcast(6, broadcast);
            }

            for message in messages {
                self.queue_system_text(character_id, message);
            }
            self.mark_dirty_sector(usize::from(x), usize::from(y));

            if is_player {
                self.pending_level_achievements.push(LevelAchievementCheck {
                    character_id,
                    level,
                    is_hardcore,
                });
            }
        }
        leveled
    }

    /// Drains achievement-level checks queued by [`Self::check_levelup`].
    pub fn drain_pending_level_achievements(&mut self) -> Vec<LevelAchievementCheck> {
        std::mem::take(&mut self.pending_level_achievements)
    }
}
