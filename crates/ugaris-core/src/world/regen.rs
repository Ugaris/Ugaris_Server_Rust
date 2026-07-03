//! Passive character regeneration tick.
//!
//! Ports two C functions from `src/system/act.c` that both run once per real
//! tick, per character, from `tick_char()`:
//!
//! - `regenerate()` (act.c:2101) - skill-gated endurance and magic-shield
//!   lifeshield regen, self-throttled to once per real second via
//!   `ch.last_regen`. Runs unconditionally regardless of the character's
//!   current action.
//! - `act_idle()` (act.c:99) - HP/endurance/mana regen while resting, gated
//!   by `ticker > ch.regen_ticker + regen_time`. In C this only runs when the
//!   character's queued action reaches `AC_IDLE` completion, and the amount
//!   is scaled by `ch.act1` (the idle batch size in ticks).
//!
//! Rust's tick loop (`World::tick_basic_actions_with_attack_policy`) treats
//! `action == 0` (`action::IDLE`) as "nothing queued" and skips those
//! characters entirely, so there is no per-batch completion event to hook
//! the idle regen into. Instead this module applies the idle regen
//! continuously, once per real tick, using the equivalent per-tick amount
//! (C's `act1 * val * 15` with `act1` ticks worth of batching collapses to
//! `val * 15` per individual tick). The steady-state rate matches C exactly;
//! only the batching granularity differs.
//!
//! Not ported here (tracked separately):
//! - `reduce_rage`/`increase_rage` - the C `rage` field does not exist on
//!   the Rust `Character` yet.
//! - The `NT_CHAR` notify-area call at the end of `act_idle()` - tracked by
//!   the separate "NPC sighting messages" P0 task.
//! - `check_endurance()` (fast-mode revert on low endurance) - tracked by
//!   the "Speed mode" P0 task, which owns `speed_mode` side effects.

use super::*;

impl World {
    /// Runs the passive regeneration pass for every live character. Call
    /// once per tick, after `World::advance()`.
    ///
    /// `regen_time` mirrors the C global `regen_time` (admin-tunable via
    /// `/setregentime`); callers pass in their own tracked value since
    /// `World::settings.regen_time` is not kept in sync with runtime admin
    /// changes.
    pub fn regenerate_characters(&mut self, regen_time: i32, area_id: u16) {
        let character_ids: Vec<CharacterId> = self.characters.keys().copied().collect();
        for character_id in character_ids {
            self.regenerate_character(character_id, regen_time, area_id);
        }
    }

    fn regenerate_character(&mut self, character_id: CharacterId, regen_time: i32, area_id: u16) {
        let tick = self.tick.0;

        let Some(character) = self.characters.get(&character_id) else {
            return;
        };
        // C: `ch[cn].x < 1 || ch[cn].x > MAXMAP - 2 || ch[cn].y < 1 || ch[cn].y > MAXMAP - 2`
        if character.x < 1
            || usize::from(character.x) > MAX_MAP - 2
            || character.y < 1
            || usize::from(character.y) > MAX_MAP - 2
        {
            return;
        }

        let no_regen_tile = self
            .map
            .tile(usize::from(character.x), usize::from(character.y))
            .is_some_and(|tile| tile.flags.contains(MapFlags::NOREGEN));
        // C: `!(map[m].flags & MF_NOREGEN) || !(ch[cn].flags & CF_PLAYER)`
        let regen_allowed_tile =
            !no_regen_tile || !character.flags.contains(CharacterFlags::PLAYER);

        let Some(character) = self.characters.get_mut(&character_id) else {
            return;
        };

        // C `regenerate()`: skill-gated endurance/lifeshield regen, throttled
        // to once per real second via `last_regen`.
        let diff = tick.saturating_sub(u64::from(character.last_regen)) / TICKS_PER_SECOND;
        if diff > 0 && regen_allowed_tile {
            let diff = diff as i32;
            // C: `speed_mode != SM_FAST && (speed_mode != SM_STEALTH || !(flags & CF_THIEFMODE))`
            if character.speed_mode != SpeedMode::Fast
                && (character.speed_mode != SpeedMode::Stealth
                    || !character.flags.contains(CharacterFlags::THIEFMODE))
            {
                if character_value_present(character, CharacterValue::Regenerate) != 0
                    && character.endurance
                        < character_value(character, CharacterValue::Endurance) * POWERSCALE
                {
                    let cap = character_value(character, CharacterValue::Endurance) * POWERSCALE;
                    let amount = (character_value(character, CharacterValue::Regenerate)
                        + character_value_present(character, CharacterValue::Regenerate))
                        * diff
                        * 5;
                    character.endurance = (character.endurance + amount).min(cap);
                    character.flags.insert(CharacterFlags::SMALLUPDATE);
                }

                if character_value_present(character, CharacterValue::MagicShield) != 0
                    && character_value_present(character, CharacterValue::Meditate) != 0
                    && character.lifeshield
                        < character_value(character, CharacterValue::MagicShield) * POWERSCALE
                    && area_id != 33
                {
                    let cap = character_value(character, CharacterValue::MagicShield) * POWERSCALE;
                    let amount = (character_value(character, CharacterValue::Meditate)
                        + character_value_present(character, CharacterValue::Meditate))
                        * diff
                        * 4;
                    character.lifeshield = (character.lifeshield + amount).min(cap);
                    character.flags.insert(CharacterFlags::SMALLUPDATE);
                }
            }

            // C: `if (areaID == 33) ch[cn].lifeshield = 0;`
            if area_id == 33 {
                character.lifeshield = 0;
            }

            let advance = i64::from(diff) * (TICKS_PER_SECOND as i64);
            character.last_regen = character
                .last_regen
                .saturating_add(u32::try_from(advance).unwrap_or(u32::MAX));
        }

        // C: safety clamp for negative lifeshield.
        if character.lifeshield < 0 {
            character.lifeshield = 0;
        }

        // C `act_idle()`: HP/endurance/mana regen while resting.
        if character.action == action::IDLE {
            let regen_time = u64::try_from(regen_time.max(0)).unwrap_or(0);
            let idle_gate = tick > u64::from(character.regen_ticker) + regen_time;
            if idle_gate && regen_allowed_tile {
                if area_id != 33 {
                    let hp_cap = character_value(character, CharacterValue::Hp) * POWERSCALE;
                    if character.hp < hp_cap {
                        let val = character_value(character, CharacterValue::Regenerate);
                        let val = if val == 0 { 7 } else { val };
                        character.hp = (character.hp + val * 15).min(hp_cap);
                        character.flags.insert(CharacterFlags::SMALLUPDATE);
                    }
                }

                let end_cap = character_value(character, CharacterValue::Endurance) * POWERSCALE;
                if character.endurance < end_cap {
                    // C: fixed val = 150.
                    character.endurance = (character.endurance + 150 * 15).min(end_cap);
                    character.flags.insert(CharacterFlags::SMALLUPDATE);
                }

                let mana_cap = character_value(character, CharacterValue::Mana) * POWERSCALE;
                if character.mana < mana_cap {
                    let val = character_value(character, CharacterValue::Meditate);
                    let val = if val == 0 { 7 } else { val };
                    character.mana = (character.mana + val * 15).min(mana_cap);
                    character.flags.insert(CharacterFlags::SMALLUPDATE);
                }

                // C: warriors with warcry (but no magicshield skill) leak
                // lifeshield while idle.
                if character_value_present(character, CharacterValue::MagicShield) == 0
                    && character.lifeshield > 0
                {
                    let regen_skill = character_value(character, CharacterValue::Regenerate);
                    let warcry_skill = character_value(character, CharacterValue::Warcry);
                    let base_reduction = ((warcry_skill - regen_skill) / 10 + 1).max(1);
                    if character.lifeshield > base_reduction {
                        character.lifeshield -= base_reduction;
                    } else {
                        character.lifeshield = 0;
                    }
                    character.flags.insert(CharacterFlags::SMALLUPDATE);
                }
            }
        }
    }
}
