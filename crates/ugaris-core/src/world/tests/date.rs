//! Tests for `World::advance_date` (C `tick_date()`, `src/system/date.c:267`).

use super::*;
use crate::game_time::{GameDate, DAY_LEN, HOUR_LEN, START_TIME, UNDERGROUND_AREA_LIGHT};

#[test]
fn advance_date_delegates_to_game_date_calculate() {
    let mut world = World::default();
    let unix_time = START_TIME + HOUR_LEN * 12;

    world.advance_date(unix_time, 1, None);

    assert_eq!(world.date, GameDate::calculate(unix_time, 1, None));
    assert_eq!(world.date.hour, 12);
}

#[test]
fn advance_date_reports_no_change_when_daylight_is_unchanged() {
    let mut world = World::default();
    // Midnight on the winter solstice: sun and moon are both below the
    // horizon (`newmoon` at `START_TIME`, see `game_time.rs` tests), so
    // `daylight` stays at its `Default` value of 0 - no change reported.
    let changed = world.advance_date(START_TIME, 1, None);

    assert!(!changed);
    assert_eq!(world.date.daylight, 0);
}

#[test]
fn advance_date_reports_change_across_sunrise_boundary() {
    let mut world = World::default();
    let sunrise = GameDate::calculate(START_TIME, 1, None).sunrise;

    // Just before sunrise: still dark (matches the `Default::default()`
    // starting daylight of 0), so the first tick reports no change.
    let before_sunrise = START_TIME + sunrise - 1;
    assert!(!world.advance_date(before_sunrise, 1, None));
    assert_eq!(world.date.daylight, 0);

    // One real hour past sunrise: `calculate_light_levels`'s ramp
    // (`daytime - sunrise) * MAX_LIGHT_LEVEL / HOURLEN`) has fully reached
    // `MAX_LIGHT_LEVEL`, so the daylight value changes and is reported.
    let after_sunrise = START_TIME + sunrise + HOUR_LEN;
    assert!(world.advance_date(after_sunrise, 1, None));
    assert_eq!(world.date.daylight, 255);

    // Calling again with the same time reports no further change.
    assert!(!world.advance_date(after_sunrise, 1, None));
}

#[test]
fn advance_date_respects_dlight_override_like_admin_dlight_command() {
    let mut world = World::default();
    let noon = START_TIME + HOUR_LEN * 12;

    let changed = world.advance_date(noon, 1, Some(42));

    assert!(changed);
    assert_eq!(world.date.daylight, 42);
}

#[test]
fn advance_date_uses_area_specific_light_override_table() {
    let mut world = World::default();
    let noon = START_TIME + HOUR_LEN * 12;

    world.advance_date(noon, 23, None);

    // Area 23 (underground) always reports the fixed underground light
    // level regardless of the natural sun/moon computation.
    assert_eq!(world.date.daylight, UNDERGROUND_AREA_LIGHT);
}

#[test]
fn advance_date_advances_at_the_c_rate_of_one_game_day_per_day_len_real_seconds() {
    let mut world = World::default();
    world.advance_date(START_TIME, 1, None);
    let day0 = world.date.yday;

    world.advance_date(START_TIME + DAY_LEN, 1, None);
    let day1 = world.date.yday;

    assert_eq!(day1, day0 + 1);
}
