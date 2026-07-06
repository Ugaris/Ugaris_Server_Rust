//! Normal (connected) player driver (`src/system/player_driver.c`, the
//! `driver()` (`src/system/libload.c:186-192`) branch invoked once per
//! tick for every character with `ch[cn].driver == 0` - i.e. a normal
//! playing character, distinct from a lingering `CDR_LOSTCON` character
//! (`world/lostcon.rs`) or any NPC driver).
//!
//! Only the two-line autobless/autopulse consumer at
//! `player_driver.c:1067-1070` is ported here so far:
//! ```c
//! if (ch[cn].value[0][V_BLESS] && lppd && lppd->autobless &&
//!     may_add_spell(cn, IDR_BLESS) && do_bless(cn, cn)) {
//!     return;
//! }
//! if (ch[cn].value[0][V_PULSE] && lppd && lppd->autopulse &&
//!     fight_driver_pulse_value(cn) && do_pulse(cn)) {
//!     return;
//! }
//! ```
//! Everything else in `player_driver` (the packet-driven `PAC_USE`/
//! `PAC_KILL`/`PAC_MOVE`/... queued-action state machine, `run_queue`,
//! `do_idle`, the auto-fightback message loop, `player_driver_optimize_
//! surround`/`autoturn`) is event-driven from client network input in
//! this codebase rather than simulated per-tick from a single driver
//! function, and is out of scope for this slice.

use super::*;

impl World {
    /// C `player_driver.c:1067-1070`'s autobless/autopulse consumer. C's
    /// explicit `may_add_spell(cn, IDR_BLESS)` gate is redundant with
    /// `do_bless`'s own internal `may_add_spell` check (`do_action.rs`,
    /// returns `DoError::AlreadyWorking` otherwise), so it is not
    /// duplicated here - matching the `process_lostcon_self_care_
    /// postcascade` precedent for the same spell. `fight_driver_pulse_
    /// value` is the already-ported `simple_baddy_pulse_value` (a plain
    /// area-damage-worth-it calculation, not NPC-specific despite the
    /// name - see `npc_fight.rs`). C has no `ch[cn].action != 0` guard on
    /// this pair of checks at all (`do_bless`/`do_pulse` themselves don't
    /// check it either) - both spells fire unconditionally whenever
    /// enabled and affordable, pre-empting whatever action the packet-
    /// driven queued-action state machine was about to dispatch this
    /// tick, exactly like C's `return` skips the rest of `player_driver`.
    /// Returns `true` if either spell fired (caller should treat this
    /// exactly like C's `return` - skip dispatching any other queued
    /// action for this character this tick).
    pub fn process_player_autobless_autopulse(
        &mut self,
        character_id: CharacterId,
        autobless: bool,
        autopulse: bool,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let weather_movement_percent = self.settings.weather_movement_percent;
        let current_tick = self.tick.0 as u32;

        if autobless && character_value_base(&character, CharacterValue::Bless) != 0 {
            if let Some(caster) = self.characters.get_mut(&character_id) {
                if do_bless(
                    caster,
                    &character,
                    &self.items,
                    current_tick,
                    None,
                    &self.map,
                    weather_movement_percent,
                )
                .is_ok()
                {
                    return true;
                }
            }
        }

        if autopulse
            && character_value_base(&character, CharacterValue::Pulse) != 0
            && self.simple_baddy_pulse_value(character_id) != 0
        {
            if let Some(caster) = self.characters.get_mut(&character_id) {
                if do_pulse(caster, &self.map, weather_movement_percent).is_ok() {
                    return true;
                }
            }
        }

        false
    }
}
