//! C `src/module/weather/weather.c`'s per-character damage ticks
//! (`handle_weather_damage`/`handle_lightning_strike`, both called from
//! every player's per-tick `apply_weather_effects` hook at `act.c:2268`).
//!
//! The rest of the weather system (the autonomous weather cycle, the
//! `SV_MOD2`/`SV_VIS_WEATHER`/`SV_VIS_SFX` client packets, the effect
//! table) lives in `crates/ugaris-server/src/weather.rs` since it's
//! server-runtime state (`WeatherState`), not `World` state - see
//! `PORTING_TODO.md`'s Weather driver task for the split and what's still
//! unported (movement-speed/visibility-range/skill-value modifiers,
//! elemental debuffs).

use super::*;

impl World {
    /// Shared guard clauses used by both `handle_weather_damage`
    /// (`weather.c:441-448`) and `handle_lightning_strike`
    /// (`weather.c:540-547`): only `CF_PLAYER` characters are ever passed
    /// weather effects at all (`apply_weather_effects`'s own call site is
    /// player-only), gods/immortals are always immune, and nothing happens
    /// indoors. Exposed so the server-runtime per-tick loop
    /// (`crates/ugaris-server/src/main.rs`) can skip the `RANDOM()` roll
    /// entirely for ineligible characters, matching C's exact
    /// guard-before-roll order (no RNG call spent on gods/indoor players).
    pub fn character_weather_eligible(&self, character_id: CharacterId) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        if !character.flags.contains(CharacterFlags::PLAYER) {
            return false;
        }
        if character
            .flags
            .intersects(CharacterFlags::GOD | CharacterFlags::IMMORTAL)
        {
            return false;
        }
        !self
            .map
            .tile(usize::from(character.x), usize::from(character.y))
            .is_some_and(|tile| tile.flags.contains(MapFlags::INDOORS))
    }

    /// C `handle_weather_damage` (`weather.c:435-471`)'s damage-application
    /// half: `hurt(cn, damage*POWERSCALE, 0, 1, 50, 50)` maps onto
    /// [`World::apply_legacy_hurt`]'s `(target, cause=None, damage,
    /// armor_divisor=1, armor_percent=50, shield_percent=50)`. The caller
    /// (`crates/ugaris-server/src/weather.rs`) is responsible for the
    /// per-tick `RANDOM(TICKS*12)` gate and for resolving
    /// `damage_per_tick` from the current weather/intensity via
    /// `weather_damage_amount`; this method only re-ports
    /// [`Self::character_weather_eligible`]'s guard clauses.
    pub fn apply_weather_damage(
        &mut self,
        character_id: CharacterId,
        damage_per_tick: i32,
    ) -> Option<LegacyHurtOutcome> {
        if damage_per_tick <= 0 || !self.character_weather_eligible(character_id) {
            return None;
        }
        self.apply_legacy_hurt(character_id, None, damage_per_tick * POWERSCALE, 1, 50, 50)
    }

    /// C `handle_lightning_strike` (`weather.c:534-575`)'s
    /// damage-application half: `hurt(cn, base_damage*POWERSCALE, 0, 0, 50,
    /// 50)` - note `armor_divisor=0` (lightning bypasses armor entirely,
    /// unlike ordinary weather damage's `armor_divisor=1`). The caller is
    /// responsible for the `WEATHER_EFFECT_LIGHTNING` gate, the per-tick
    /// `RANDOM(100*TICKS*60) >= lightning_chance*100` roll, resolving
    /// `base_damage` from the current weather intensity, the "CRACK!
    /// Lightning strikes you!" `log_char`, and the SFX/thunder broadcast -
    /// all server-runtime concerns living in
    /// `crates/ugaris-server/src/weather.rs`/`main.rs`. This method only
    /// re-ports [`Self::character_weather_eligible`]'s guard clauses.
    pub fn apply_lightning_strike_damage(
        &mut self,
        character_id: CharacterId,
        base_damage: i32,
    ) -> Option<LegacyHurtOutcome> {
        if base_damage <= 0 || !self.character_weather_eligible(character_id) {
            return None;
        }
        self.apply_legacy_hurt(character_id, None, base_damage * POWERSCALE, 0, 50, 50)
    }
}
