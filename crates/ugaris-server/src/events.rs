use super::*;

// ============================================================================
// C `src/module/events/events.c`/`events.h`: the generic recurring/seasonal
// event scheduler. This slice ports the calendar-matching primitives
// (`is_date_in_range`/`is_time_in_range`/`is_day_matching`/
// `is_week_matching`/`should_event_be_active`'s `RECUR_WEEKLY`/
// `RECUR_BIWEEKLY` branches) plus the five recurring boosted-rate events
// under `src/module/events/recurring/*` and their modifier hooks. The
// Christmas seasonal event (`src/module/events/seasonal/christmas_event.c`)
// already has its own independently-ported date logic in `xmas.rs` and is
// left as-is; Easter and the generic `EventDecoration`/date-range
// scheduling needed for other seasonal events are not part of this slice
// (see `PORTING_TODO.md`).
//
// Like `xmas::current_xmas_event`, calendar math runs in UTC rather than
// replicating C's `localtime(time(NULL))` host-timezone lookup - the same
// simplification already established for Christmas.
// ============================================================================

/// C `events.h`'s `DOW_BITMAP_*` macros (`1 << DOW_<day>`, `DOW_SUNDAY == 0`).
pub(crate) const DOW_BITMAP_SUNDAY: u8 = 1 << 0;
pub(crate) const DOW_BITMAP_MONDAY: u8 = 1 << 1;
pub(crate) const DOW_BITMAP_TUESDAY: u8 = 1 << 2;
pub(crate) const DOW_BITMAP_WEDNESDAY: u8 = 1 << 3;
pub(crate) const DOW_BITMAP_THURSDAY: u8 = 1 << 4;
pub(crate) const DOW_BITMAP_SATURDAY: u8 = 1 << 6;
pub(crate) const DOW_BITMAP_WEEKEND: u8 = DOW_BITMAP_SATURDAY | DOW_BITMAP_SUNDAY;

/// C `src/system/loot/loot.c`'s `event_drop_rate` modifier name, the one
/// named scalar the recurring drop-rate events reference (`loot.h:20,36`).
const EVENT_DROP_RATE_MODIFIER: &str = "event_drop_rate";

// ----------------------------------------------------------------- calendar

/// Inverse of `xmas::civil_from_unix_seconds`'s day-to-civil conversion:
/// Howard Hinnant's `days_from_civil` algorithm, giving days-since-unix-
/// epoch for a proleptic-Gregorian civil date.
pub(crate) fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let y = if month <= 2 {
        i64::from(year) - 1
    } else {
        i64::from(year)
    };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let mp = if month > 2 {
        i64::from(month) - 3
    } else {
        i64::from(month) + 9
    };
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

/// C `is_day_matching`'s `tm_wday` convention: 0 = Sunday .. 6 = Saturday.
/// 1970-01-01 (`days == 0`) was a Thursday.
pub(crate) fn weekday_from_days(days: i64) -> u32 {
    (((days % 7) + 7 + 4) % 7) as u32
}

/// glibc `strftime("%W")` semantics (matched against real `date +%W`
/// output): week number of the year with Monday as the first day of the
/// week; every day before the year's first Monday is week 0. Used by
/// `is_week_matching` (C's `RECUR_BIWEEKLY` branch).
pub(crate) fn week_number(year: i32, month: u32, day: u32) -> i32 {
    let days = days_from_civil(year, month, day);
    let jan1_days = days_from_civil(year, 1, 1);
    let yday = days - jan1_days;
    let jan1_weekday = i64::from(weekday_from_days(jan1_days));
    let first_monday_yday = (8 - jan1_weekday) % 7;
    if yday < first_monday_yday {
        0
    } else {
        ((yday - first_monday_yday) / 7 + 1) as i32
    }
}

/// C `is_date_in_range` (`events.c:28-52`), transcribed with its exact
/// (slightly asymmetric) wraparound handling: for a wraparound range
/// (`start_month > end_month`) only months strictly between `end_month`
/// and `start_month` are rejected up front; the trailing generic checks
/// below apply no upper bound to the `current_month == start_month` case
/// (matching C, since a different month covers the end boundary).
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn is_date_in_range(
    start_month: u32,
    start_day: u32,
    end_month: u32,
    end_day: u32,
    current_month: u32,
    current_day: u32,
) -> bool {
    if start_month > end_month {
        if current_month < start_month && current_month > end_month {
            return false;
        }
    } else if start_month == end_month {
        return current_month == start_month && current_day >= start_day && current_day <= end_day;
    }

    if current_month == start_month {
        current_day >= start_day
    } else if current_month == end_month {
        current_day <= end_day
    } else {
        current_month > start_month && current_month < end_month
    }
}

/// C `is_time_in_range` (`events.c:55-70`): minute-of-day comparison with
/// overnight wraparound (`start > end`) support.
pub(crate) fn is_time_in_range(
    start_hour: u32,
    start_minute: u32,
    end_hour: u32,
    end_minute: u32,
    current_hour: u32,
    current_minute: u32,
) -> bool {
    let current = current_hour * 60 + current_minute;
    let start = start_hour * 60 + start_minute;
    let end = end_hour * 60 + end_minute;
    if start > end {
        current >= start || current <= end
    } else {
        current >= start && current <= end
    }
}

/// C `is_day_matching` (`events.c:73-79`).
pub(crate) fn is_day_matching(days_bitmap: u8, weekday: u32) -> bool {
    (days_bitmap & (1 << weekday)) != 0
}

/// C `is_week_matching` (`events.c:82-93`): `(current_week % interval) ==
/// (week_number % interval)`.
pub(crate) fn is_week_matching(week_number_cfg: i32, interval: i32, current_week: i32) -> bool {
    (current_week % interval) == (week_number_cfg % interval)
}

/// A resolved instant in the real-world (UTC) wall-clock calendar, as read
/// by the C event system's repeated `localtime(time(NULL))` calls.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CalendarNow {
    pub(crate) hour: u32,
    pub(crate) minute: u32,
    pub(crate) weekday: u32,
    pub(crate) week: i32,
}

impl CalendarNow {
    pub(crate) fn from_unix_seconds(seconds: u64) -> Self {
        let (year, month, day) = crate::xmas::civil_from_unix_seconds(seconds);
        let days = (seconds / 86_400) as i64;
        let weekday = weekday_from_days(days);
        let week = week_number(year, month, day);
        let seconds_of_day = seconds % 86_400;
        let hour = (seconds_of_day / 3600) as u32;
        let minute = ((seconds_of_day % 3600) / 60) as u32;
        Self {
            hour,
            minute,
            weekday,
            week,
        }
    }

    pub(crate) fn now() -> Self {
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default();
        Self::from_unix_seconds(seconds)
    }
}

// ------------------------------------------------------------- recurrence

/// C `events.h`'s `RecurrenceType` values actually used by a currently-
/// ported recurring event (`RECUR_WEEKLY`/`RECUR_BIWEEKLY`). Add
/// `Daily`/`Monthly`/`Yearly`/`None` branches (mirroring `RECUR_DAILY`/
/// `RECUR_MONTHLY`/`RECUR_YEARLY`/`RECUR_NONE`) when a slice needing them
/// (e.g. Easter, a `RECUR_NONE` date-range event) is ported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecurrenceType {
    Weekly,
    Biweekly,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RecurrencePattern {
    pub(crate) recur_type: RecurrenceType,
    pub(crate) days: u8,
    pub(crate) week_number: i32,
    pub(crate) interval: i32,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TimeRange {
    pub(crate) start_hour: u32,
    pub(crate) start_minute: u32,
    pub(crate) end_hour: u32,
    pub(crate) end_minute: u32,
}

const ALL_DAY: TimeRange = TimeRange {
    start_hour: 0,
    start_minute: 0,
    end_hour: 23,
    end_minute: 59,
};

/// C `should_event_be_active`'s `RECUR_WEEKLY`/`RECUR_BIWEEKLY` branches
/// (`events.c:104-137`), the only recurrence kinds any currently-ported
/// recurring event uses, plus the trailing time-constraint check
/// (`events.c:150-155`; every currently-ported recurring event sets
/// `has_time_constraint = true` with an all-day range).
pub(crate) fn should_recurring_event_be_active(
    pattern: &RecurrencePattern,
    time_range: &TimeRange,
    now: &CalendarNow,
) -> bool {
    if !is_day_matching(pattern.days, now.weekday) {
        return false;
    }
    if pattern.recur_type == RecurrenceType::Biweekly
        && !is_week_matching(pattern.week_number, pattern.interval, now.week)
    {
        return false;
    }
    is_time_in_range(
        time_range.start_hour,
        time_range.start_minute,
        time_range.end_hour,
        time_range.end_minute,
        now.hour,
        now.minute,
    )
}

// --------------------------------------------------------- recurring events

/// The five recurring boosted-rate events under
/// `src/module/events/recurring/*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecurringEventKind {
    /// `boosted_experience_event.c` (`EVENT_DOUBLE_XP`).
    DoubleExpThursday,
    /// `boosted_droprate_event.c` (`EVENT_DOUBLE_LOOT`).
    DoubleDropTuesday,
    /// `boosted_weekend_event.c` (`EVENT_BONUS_LOOT`).
    BonusWeekend,
    /// `boosted_mining_event.c` (`EVENT_MINING_MONDAY`).
    MiningMonday,
    /// `boosted_artifacts_mining_event.c` (`EVENT_MINING_WEDNESDAY`).
    MiningWednesday,
}

impl RecurringEventKind {
    pub(crate) const ALL: [RecurringEventKind; 5] = [
        RecurringEventKind::DoubleExpThursday,
        RecurringEventKind::DoubleDropTuesday,
        RecurringEventKind::BonusWeekend,
        RecurringEventKind::MiningMonday,
        RecurringEventKind::MiningWednesday,
    ];

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::DoubleExpThursday => "Double Experience Thursday",
            Self::DoubleDropTuesday => "Double Drop Rate Tuesday",
            Self::BonusWeekend => "Double Experience & Drop Rate Weekend",
            Self::MiningMonday => "Mining Monday",
            Self::MiningWednesday => "Mining Wednesday",
        }
    }

    fn recurrence(self) -> RecurrencePattern {
        match self {
            Self::DoubleExpThursday => RecurrencePattern {
                recur_type: RecurrenceType::Weekly,
                days: DOW_BITMAP_THURSDAY,
                week_number: 0,
                interval: 1,
            },
            Self::DoubleDropTuesday => RecurrencePattern {
                recur_type: RecurrenceType::Weekly,
                days: DOW_BITMAP_TUESDAY,
                week_number: 0,
                interval: 1,
            },
            // C `boosted_weekend_event.c`'s `recurrence.week_number = 1`,
            // `interval = 2` ("every other weekend").
            Self::BonusWeekend => RecurrencePattern {
                recur_type: RecurrenceType::Biweekly,
                days: DOW_BITMAP_WEEKEND,
                week_number: 1,
                interval: 2,
            },
            Self::MiningMonday => RecurrencePattern {
                recur_type: RecurrenceType::Weekly,
                days: DOW_BITMAP_MONDAY,
                week_number: 0,
                interval: 1,
            },
            Self::MiningWednesday => RecurrencePattern {
                recur_type: RecurrenceType::Weekly,
                days: DOW_BITMAP_WEDNESDAY,
                week_number: 0,
                interval: 1,
            },
        }
    }

    pub(crate) fn should_be_active(self, now: &CalendarNow) -> bool {
        should_recurring_event_be_active(&self.recurrence(), &ALL_DAY, now)
    }
}

/// Per-event `is_active`/snapshot runtime state (C's per-file `static Event
/// <name>_event;` plus each file's `static <Type>Data event_data;`).
///
/// Two genuine C quirks are preserved rather than "fixed": `boosted_mining_
/// event.c`/`boosted_artifacts_mining_event.c` both capture an `original_*`
/// snapshot in their `_start` hook that their `_end` hook never reads -
/// `_end` hardcodes the multipliers back to `1.0` instead - so Mining
/// Monday/Wednesday need no snapshot storage at all here. Only
/// `boosted_experience_event.c`/`boosted_weekend_event.c` actually restore
/// their captured `original_exp_modifier` snapshot on end.
#[derive(Debug, Clone, Default)]
pub(crate) struct RecurringEventsState {
    double_xp_active: bool,
    double_xp_original_exp_modifier: f64,
    double_drop_active: bool,
    weekend_active: bool,
    weekend_original_exp_modifier: f64,
    mining_monday_active: bool,
    mining_wednesday_active: bool,
}

impl RecurringEventsState {
    fn is_active(&self, kind: RecurringEventKind) -> bool {
        match kind {
            RecurringEventKind::DoubleExpThursday => self.double_xp_active,
            RecurringEventKind::DoubleDropTuesday => self.double_drop_active,
            RecurringEventKind::BonusWeekend => self.weekend_active,
            RecurringEventKind::MiningMonday => self.mining_monday_active,
            RecurringEventKind::MiningWednesday => self.mining_wednesday_active,
        }
    }

    /// C `start_event` calling each event's `on_start` hook.
    fn start(&mut self, kind: RecurringEventKind, settings: &mut GameSettings) {
        match kind {
            RecurringEventKind::DoubleExpThursday => {
                // C `boosted_experience_start`: `exp_modifier *= 1.5`.
                self.double_xp_original_exp_modifier = settings.exp_modifier;
                settings.exp_modifier *= 1.5;
                self.double_xp_active = true;
            }
            RecurringEventKind::DoubleDropTuesday => {
                // C `boosted_droprate_start`: `loot_set_modifier(2.0)`.
                settings.set_loot_modifier(EVENT_DROP_RATE_MODIFIER, 2.0);
                self.double_drop_active = true;
            }
            RecurringEventKind::BonusWeekend => {
                // C `boosted_weekend_start`: `exp_modifier *= 1.5` +
                // `loot_set_modifier(2.0)`.
                self.weekend_original_exp_modifier = settings.exp_modifier;
                settings.exp_modifier *= 1.5;
                settings.set_loot_modifier(EVENT_DROP_RATE_MODIFIER, 2.0);
                self.weekend_active = true;
            }
            RecurringEventKind::MiningMonday => {
                // C `mining_monday_start`: silver/gold, cave-in, and golem
                // spawn multipliers all doubled.
                settings.mining_silver_gold_multiplier = 2.0;
                settings.mining_cavein_multiplier = 2.0;
                settings.mining_golem_event_multiplier = 2.0;
                self.mining_monday_active = true;
            }
            RecurringEventKind::MiningWednesday => {
                // C `boosted_mining_event_start`: `mining_artifact_multiplier
                // = 5.0` (5x artifact chance).
                settings.mining_artifact_multiplier = 5.0;
                self.mining_wednesday_active = true;
            }
        }
    }

    /// C `end_event` calling each event's `on_end` hook.
    fn end(&mut self, kind: RecurringEventKind, settings: &mut GameSettings) {
        match kind {
            RecurringEventKind::DoubleExpThursday => {
                settings.exp_modifier = self.double_xp_original_exp_modifier;
                self.double_xp_active = false;
            }
            RecurringEventKind::DoubleDropTuesday => {
                settings.set_loot_modifier(EVENT_DROP_RATE_MODIFIER, 1.0);
                self.double_drop_active = false;
            }
            RecurringEventKind::BonusWeekend => {
                settings.exp_modifier = self.weekend_original_exp_modifier;
                settings.set_loot_modifier(EVENT_DROP_RATE_MODIFIER, 1.0);
                self.weekend_active = false;
            }
            RecurringEventKind::MiningMonday => {
                // C `mining_monday_end`: hardcoded back to `1.0`, not a
                // snapshot restore (see doc comment above).
                settings.mining_silver_gold_multiplier = 1.0;
                settings.mining_cavein_multiplier = 1.0;
                settings.mining_golem_event_multiplier = 1.0;
                self.mining_monday_active = false;
            }
            RecurringEventKind::MiningWednesday => {
                // C `boosted_mining_event_end`: hardcoded back to `1.0`.
                settings.mining_artifact_multiplier = 1.0;
                self.mining_wednesday_active = false;
            }
        }
    }
}

/// C `check_events` (`events.c:274-292`), the once-a-minute scheduled task
/// (`add_scheduled_task(check_events, 60, "event_check", true)`) that
/// starts/ends each registered event on a should-be-active transition.
/// Returns the `(kind, started)` transitions this call produced, for the
/// caller to log (matching C's `xlog("%s event started/ended!", ...)`).
pub(crate) fn check_recurring_events(
    settings: &mut GameSettings,
    state: &mut RecurringEventsState,
    now: &CalendarNow,
) -> Vec<(RecurringEventKind, bool)> {
    let mut transitions = Vec::new();
    for kind in RecurringEventKind::ALL {
        let should_be_active = kind.should_be_active(now);
        let is_active = state.is_active(kind);
        if should_be_active && !is_active {
            state.start(kind, settings);
            transitions.push((kind, true));
        } else if !should_be_active && is_active {
            state.end(kind, settings);
            transitions.push((kind, false));
        }
    }
    transitions
}
