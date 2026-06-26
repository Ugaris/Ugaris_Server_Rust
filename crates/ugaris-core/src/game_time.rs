use serde::{Deserialize, Serialize};

pub const START_TIME: i64 = 978_303_600;
pub const DAY_LEN: i64 = 60 * 60 * 2;
pub const HOUR_LEN: i64 = DAY_LEN / 24;
pub const MIN_LEN: i64 = HOUR_LEN / 60;
pub const WEEK_LEN: i64 = DAY_LEN * 7;
pub const MONTH_LEN: i64 = DAY_LEN * 30;
pub const YEAR_LEN: i64 = MONTH_LEN * 12;
pub const MOON_LEN: i64 = DAY_LEN * 28;

pub const SECONDS_PER_MINUTE: i64 = 60;
pub const MINUTES_PER_HOUR: i64 = 60;
pub const HOURS_PER_DAY: i64 = 24;
pub const DAYS_PER_WEEK: i64 = 7;
pub const DAYS_PER_MONTH: i64 = 30;
pub const DAYS_PER_YEAR: i64 = 360;
pub const MONTHS_PER_YEAR: i64 = 12;
pub const DAYS_PER_MOON_CYCLE: i64 = 28;
pub const HALF_MOON_CYCLE: i64 = 14;

pub const WINTER_SOLSTICE_DAY: i64 = 0;
pub const SPRING_EQUINOX_DAY: i64 = 90;
pub const SUMMER_SOLSTICE_DAY: i64 = 180;
pub const FALL_EQUINOX_DAY: i64 = 270;

pub const MAX_LIGHT_LEVEL: i32 = 255;
pub const MAX_SUNRISE_SUNSET_DIFF_MINUTES: i64 = 90;
pub const UNDERGROUND_AREA_LIGHT: i32 = 32;
pub const DEEP_CAVE_AREA_LIGHT: i32 = 64;
pub const TWILIGHT_AREA_LIGHT: i32 = 96;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameDate {
    pub daylight: i32,
    pub newmoon: bool,
    pub fullmoon: bool,
    pub moonsize: i32,
    pub solstice: bool,
    pub equinox: bool,
    pub summer_solstice: bool,
    pub winter_solstice: bool,
    pub spring_equinox: bool,
    pub fall_equinox: bool,
    pub sunrise: i64,
    pub sunset: i64,
    pub moonrise: i64,
    pub moonset: i64,
    pub moonlight: i32,
    pub sunlight: i32,
    pub minute: i64,
    pub hour: i64,
    pub mday: i64,
    pub wday: i64,
    pub yday: i64,
    pub week: i64,
    pub month: i64,
    pub year: i64,
    pub moon: i64,
    pub moonday: i64,
    pub realtime: i64,
}

impl GameDate {
    pub fn calculate(unix_time: i64, area_id: i32, daylight_override: Option<i32>) -> Self {
        let game_time = unix_time - START_TIME;
        let mut date = Self {
            realtime: game_time,
            ..Self::default()
        };
        date.calculate_time_units(game_time);
        date.calculate_seasonal_events();
        date.calculate_sun_times();
        date.calculate_moon_times();
        date.calculate_light_levels(game_time.rem_euclid(DAY_LEN));
        let natural_light = date.sunlight.max(date.moonlight);
        date.daylight = daylight_override.unwrap_or_else(|| area_light(area_id, natural_light));
        date
    }

    fn calculate_time_units(&mut self, game_time: i64) {
        self.minute = (game_time / MIN_LEN) % MINUTES_PER_HOUR;
        self.hour = (game_time / HOUR_LEN) % HOURS_PER_DAY;
        self.week = game_time / WEEK_LEN;
        self.wday = (game_time - self.week * WEEK_LEN) / DAY_LEN;
        self.month = game_time / MONTH_LEN;
        self.mday = (game_time - self.month * MONTH_LEN) / DAY_LEN;
        self.month %= MONTHS_PER_YEAR;
        self.year = game_time / YEAR_LEN;
        self.yday = (game_time - self.year * YEAR_LEN) / DAY_LEN;
        self.moon = game_time / MOON_LEN;
        self.moonday = (game_time - self.moon * MOON_LEN) / DAY_LEN;
    }

    fn calculate_seasonal_events(&mut self) {
        self.winter_solstice = self.yday == WINTER_SOLSTICE_DAY;
        self.spring_equinox = self.yday == SPRING_EQUINOX_DAY;
        self.summer_solstice = self.yday == SUMMER_SOLSTICE_DAY;
        self.fall_equinox = self.yday == FALL_EQUINOX_DAY;
        self.solstice = self.winter_solstice || self.summer_solstice;
        self.equinox = self.spring_equinox || self.fall_equinox;
    }

    fn calculate_sun_times(&mut self) {
        let base_sunrise = HOUR_LEN * 6;
        let base_sunset = HOUR_LEN * 18;
        let minutes_adjust = if self.yday == WINTER_SOLSTICE_DAY {
            MAX_SUNRISE_SUNSET_DIFF_MINUTES
        } else if self.yday < SPRING_EQUINOX_DAY {
            MAX_SUNRISE_SUNSET_DIFF_MINUTES
                - (self.yday * MAX_SUNRISE_SUNSET_DIFF_MINUTES / SPRING_EQUINOX_DAY)
        } else if self.yday == SPRING_EQUINOX_DAY {
            0
        } else if self.yday < SUMMER_SOLSTICE_DAY {
            -((self.yday - SPRING_EQUINOX_DAY) * MAX_SUNRISE_SUNSET_DIFF_MINUTES
                / (SUMMER_SOLSTICE_DAY - SPRING_EQUINOX_DAY))
        } else if self.yday == SUMMER_SOLSTICE_DAY {
            -MAX_SUNRISE_SUNSET_DIFF_MINUTES
        } else if self.yday < FALL_EQUINOX_DAY {
            -MAX_SUNRISE_SUNSET_DIFF_MINUTES
                + ((self.yday - SUMMER_SOLSTICE_DAY) * MAX_SUNRISE_SUNSET_DIFF_MINUTES
                    / (FALL_EQUINOX_DAY - SUMMER_SOLSTICE_DAY))
        } else if self.yday == FALL_EQUINOX_DAY {
            0
        } else {
            (self.yday - FALL_EQUINOX_DAY) * MAX_SUNRISE_SUNSET_DIFF_MINUTES
                / (DAYS_PER_YEAR - FALL_EQUINOX_DAY)
        };
        self.sunrise = base_sunrise + minutes_adjust * MIN_LEN;
        self.sunset = base_sunset - minutes_adjust * MIN_LEN;
    }

    fn calculate_moon_times(&mut self) {
        if self.moonday == 0 {
            self.moonrise = 6 * HOUR_LEN;
            self.moonset = (self.moonrise + HOUR_LEN * 12) % DAY_LEN;
            self.moonsize = 0;
            self.newmoon = true;
        } else if self.moonday < HALF_MOON_CYCLE {
            self.moonrise = 6 * HOUR_LEN + (self.moonday * HOUR_LEN * 12 / HALF_MOON_CYCLE);
            self.moonset = (self.moonrise + HOUR_LEN * 12) % DAY_LEN;
            self.moonsize = self.moonday as i32;
        } else if self.moonday == HALF_MOON_CYCLE {
            self.moonrise = 18 * HOUR_LEN;
            self.moonset = (self.moonrise + HOUR_LEN * 12) % DAY_LEN;
            self.moonsize = HALF_MOON_CYCLE as i32;
            self.fullmoon = true;
        } else {
            self.moonrise =
                (6 * HOUR_LEN + (self.moonday * HOUR_LEN * 12 / HALF_MOON_CYCLE)) % DAY_LEN;
            self.moonset = (self.moonrise + HOUR_LEN * 12) % DAY_LEN;
            self.moonsize = (DAYS_PER_MOON_CYCLE - self.moonday) as i32;
        }
    }

    fn calculate_light_levels(&mut self, daytime: i64) {
        self.sunlight = ramp_light(daytime, self.sunrise, self.sunset);
        self.moonlight = if self.moonrise < self.moonset {
            ramp_light(daytime, self.moonrise, self.moonset)
        } else if daytime <= self.moonset {
            MAX_LIGHT_LEVEL
        } else if daytime < self.moonset + HOUR_LEN {
            MAX_LIGHT_LEVEL - ((daytime - self.moonset) * MAX_LIGHT_LEVEL as i64 / HOUR_LEN) as i32
        } else if daytime < self.moonrise {
            0
        } else if daytime < self.moonrise + HOUR_LEN {
            ((daytime - self.moonrise) * MAX_LIGHT_LEVEL as i64 / HOUR_LEN) as i32
        } else {
            MAX_LIGHT_LEVEL
        };
        self.moonlight = self.moonlight * self.moonsize / ((DAYS_PER_MOON_CYCLE / 2 + 7) as i32);
    }
}

fn ramp_light(daytime: i64, rise: i64, set: i64) -> i32 {
    if daytime < rise {
        0
    } else if daytime < rise + HOUR_LEN {
        ((daytime - rise) * MAX_LIGHT_LEVEL as i64 / HOUR_LEN) as i32
    } else if daytime <= set {
        MAX_LIGHT_LEVEL
    } else if daytime > set + HOUR_LEN {
        0
    } else {
        MAX_LIGHT_LEVEL - ((daytime - set) * MAX_LIGHT_LEVEL as i64 / HOUR_LEN) as i32
    }
}

fn area_light(area_id: i32, calculated_light: i32) -> i32 {
    match area_id {
        2 => TWILIGHT_AREA_LIGHT,
        21 => DEEP_CAVE_AREA_LIGHT,
        23 | 24 => UNDERGROUND_AREA_LIGHT,
        _ => calculated_light,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_time_is_winter_solstice_new_moon_midnight() {
        let date = GameDate::calculate(START_TIME, 1, None);
        assert_eq!(date.year, 0);
        assert_eq!(date.yday, 0);
        assert!(date.winter_solstice);
        assert!(date.newmoon);
        assert_eq!(date.hour, 0);
    }

    #[test]
    fn area_light_overrides_match_c_switch() {
        assert_eq!(
            GameDate::calculate(START_TIME + HOUR_LEN * 12, 2, None).daylight,
            TWILIGHT_AREA_LIGHT
        );
        assert_eq!(
            GameDate::calculate(START_TIME + HOUR_LEN * 12, 23, None).daylight,
            UNDERGROUND_AREA_LIGHT
        );
        assert_eq!(
            GameDate::calculate(START_TIME + HOUR_LEN * 12, 21, Some(7)).daylight,
            7
        );
    }
}
