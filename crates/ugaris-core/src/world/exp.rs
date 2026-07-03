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

impl World {
    /// C `check_levelup(cn)` (`src/system/tool.c:1318-1356`): loop while
    /// `exp2level(max(exp, exp_used)) > level`, granting one level per
    /// iteration. Returns whether any level was gained (C's `flag`).
    ///
    /// Ported per iteration: the level increment, the "Thou gained a
    /// level!" text, save-count grant/reset (hardcore resets `saves` to 0;
    /// everyone else gets `saves + 1` capped at 10, with the two matching
    /// feedback lines), the level-20 profession unlock (`value[1]
    /// [V_PROFESSION] = 1`) with its text, and the map dirty-sector refresh
    /// (C `set_sector`).
    ///
    /// Documented gaps (not silently dropped, matching C `check_levelup`
    /// exactly otherwise):
    /// - the level-10-multiple server-wide "Grats: NAME is level N now!"
    ///   broadcast (`server_chat(6, ...)`) has no Rust equivalent yet - no
    ///   fan-out-to-all-sessions primitive exists in `ugaris-core` (session
    ///   management is a server-crate concept);
    /// - `achievement_check_level(cn, level)` has no Rust equivalent (the
    ///   existing `AchievementState` only tracks chest/transport
    ///   milestones, not level);
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

            let mut messages = vec![format!("Thou gained a level! Thou art level {level} now.")];

            if character.flags.contains(CharacterFlags::HARDCORE) {
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

            for message in messages {
                self.queue_system_text(character_id, message);
            }
            self.mark_dirty_sector(usize::from(x), usize::from(y));
        }
        leveled
    }
}
