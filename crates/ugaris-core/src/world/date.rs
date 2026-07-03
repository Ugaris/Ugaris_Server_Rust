//! Game clock advancement.
//!
//! Ports `tick_date()` (`src/system/date.c:267`), called once per real
//! server tick from the C main loop (`src/server.c:618`) with
//! `time_now = time(NULL)` - i.e. the game clock is not incremented at some
//! fixed per-tick rate, it is *recomputed from real wall-clock time* every
//! tick via `GameDate::calculate` (`crates/ugaris-core/src/game_time.rs`),
//! which already ports `calculate_time_units`/`calculate_seasonal_events`/
//! `calculate_sun_times`/`calculate_moon_times`/`calculate_light_levels`
//! and the `/dlight` override (`dlight_override`, `command.c:6386`).
//!
//! `World` has no `area_id` field (it is threaded through as a parameter
//! everywhere else, e.g. `process_due_timers(area_id)`), so this is a
//! separate method rather than folded into the no-argument `World::advance`
//! (which only bumps the tick counter and is called from ~30 test sites).

use super::*;

impl World {
    /// Recomputes `self.date` from live server time and area context,
    /// mirroring C `tick_date()`. Returns `true` if the resulting `daylight`
    /// value (the `dlight` global) changed since the previous tick, which is
    /// the signal C's `plr_map_update` uses (`player.c:2357`) to force a full
    /// visible-map redraw for every player instead of the incremental
    /// per-sector diff.
    pub fn advance_date(
        &mut self,
        unix_time: i64,
        area_id: u16,
        dlight_override: Option<i32>,
    ) -> bool {
        let previous_daylight = self.date.daylight;
        self.date = GameDate::calculate(unix_time, i32::from(area_id), dlight_override);
        self.date.daylight != previous_daylight
    }
}
