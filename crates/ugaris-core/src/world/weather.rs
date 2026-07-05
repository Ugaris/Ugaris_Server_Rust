//! C `src/module/weather/weather.c`'s per-character damage tick
//! (`handle_weather_damage`, called from every player's per-tick
//! `apply_weather_effects` hook at `act.c:2268`).
//!
//! The rest of the weather system (the autonomous weather cycle, the
//! `SV_MOD2`/`SV_VIS_WEATHER` client packet, the effect table) lives in
//! `crates/ugaris-server/src/weather.rs` since it's server-runtime state
//! (`WeatherState`), not `World` state - see `PORTING_TODO.md`'s Weather
//! driver task for the split and what's still unported (movement-speed/
//! visibility-range modifiers, lightning strikes, elemental debuffs).

use super::*;

impl World {
    /// C `handle_weather_damage` (`weather.c:435-471`)'s damage-application
    /// half: `hurt(cn, damage*POWERSCALE, 0, 1, 50, 50)` maps onto
    /// [`World::apply_legacy_hurt`]'s `(target, cause=None, damage,
    /// armor_divisor=1, armor_percent=50, shield_percent=50)`. The caller
    /// (`crates/ugaris-server/src/weather.rs`) is responsible for the
    /// per-tick `RANDOM(TICKS*12)` gate and for resolving
    /// `damage_per_tick` from the current weather/intensity via
    /// `weather_damage_amount`; this method only re-ports the two
    /// per-character guard clauses that don't depend on that
    /// server-runtime state:
    /// - C's `if (ch[cn].flags & (CF_GOD | CF_IMMORTAL)) return;`
    /// - C's `if (map[m].flags & MF_INDOORS) return;` (no weather damage
    ///   indoors)
    ///
    /// Also gates on `CharacterFlags::PLAYER` like the C call site itself
    /// (`apply_weather_effects` is only ever invoked for `CF_PLAYER`
    /// characters), so NPCs are never hurt by weather.
    pub fn apply_weather_damage(
        &mut self,
        character_id: CharacterId,
        damage_per_tick: i32,
    ) -> Option<LegacyHurtOutcome> {
        if damage_per_tick <= 0 {
            return None;
        }
        let character = self.characters.get(&character_id)?;
        if !character.flags.contains(CharacterFlags::PLAYER) {
            return None;
        }
        if character
            .flags
            .intersects(CharacterFlags::GOD | CharacterFlags::IMMORTAL)
        {
            return None;
        }
        let indoors = self
            .map
            .tile(usize::from(character.x), usize::from(character.y))
            .is_some_and(|tile| tile.flags.contains(MapFlags::INDOORS));
        if indoors {
            return None;
        }
        self.apply_legacy_hurt(character_id, None, damage_per_tick * POWERSCALE, 1, 50, 50)
    }
}
