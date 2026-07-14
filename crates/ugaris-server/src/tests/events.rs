// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use super::*;
use crate::events::*;

#[test]
fn week_number_matches_glibc_strftime_percent_w() {
    // Reference values captured via `date -u -d "<date>" +%W` (glibc
    // `strftime`'s Monday-first, "week 0 before the first Monday" rule).
    assert_eq!(week_number(2024, 1, 1), 1);
    assert_eq!(week_number(2024, 1, 7), 1);
    assert_eq!(week_number(2024, 1, 8), 2);
    assert_eq!(week_number(2024, 1, 14), 2);
    assert_eq!(week_number(2024, 1, 15), 3);
    assert_eq!(week_number(2024, 12, 29), 52);
    assert_eq!(week_number(2024, 12, 30), 53);
    assert_eq!(week_number(2024, 12, 31), 53);
    assert_eq!(week_number(2025, 1, 1), 0);
    assert_eq!(week_number(2025, 1, 5), 0);
    assert_eq!(week_number(2025, 1, 6), 1);
    assert_eq!(week_number(2020, 1, 1), 0);
    assert_eq!(week_number(2023, 1, 1), 0);
}

#[test]
fn weekday_from_days_matches_known_epoch_anchor() {
    // 1970-01-01 (days == 0) was a Thursday.
    assert_eq!(weekday_from_days(0), 4);
    // 1970-01-04 was a Sunday.
    assert_eq!(weekday_from_days(3), 0);
    // 1970-01-10 was a Saturday.
    assert_eq!(weekday_from_days(9), 6);
}

#[test]
fn is_date_in_range_handles_wraparound_like_c() {
    // Dec 20 -> Jan 7 wraparound (Christmas's own window, ported
    // independently in xmas.rs, but the same generic primitive here).
    assert!(is_date_in_range(12, 20, 1, 7, 12, 25));
    assert!(is_date_in_range(12, 20, 1, 7, 1, 7));
    assert!(is_date_in_range(12, 20, 1, 7, 12, 20));
    assert!(!is_date_in_range(12, 20, 1, 7, 12, 19));
    assert!(!is_date_in_range(12, 20, 1, 7, 1, 8));
    assert!(!is_date_in_range(12, 20, 1, 7, 6, 15));

    // Same-month range.
    assert!(is_date_in_range(3, 10, 3, 20, 3, 15));
    assert!(!is_date_in_range(3, 10, 3, 20, 3, 9));
    assert!(!is_date_in_range(3, 10, 3, 20, 3, 21));

    // Multi-month non-wraparound range.
    assert!(is_date_in_range(3, 1, 6, 1, 4, 15));
    assert!(is_date_in_range(3, 1, 6, 1, 3, 1));
    assert!(is_date_in_range(3, 1, 6, 1, 6, 1));
    assert!(!is_date_in_range(3, 1, 6, 1, 2, 28));
    assert!(!is_date_in_range(3, 1, 6, 1, 7, 1));
}

#[test]
fn is_time_in_range_handles_overnight_wraparound() {
    assert!(is_time_in_range(0, 0, 23, 59, 12, 30));
    assert!(!is_time_in_range(9, 0, 17, 0, 8, 59));
    assert!(is_time_in_range(9, 0, 17, 0, 17, 0));
    // Overnight: 22:00 -> 06:00.
    assert!(is_time_in_range(22, 0, 6, 0, 23, 30));
    assert!(is_time_in_range(22, 0, 6, 0, 2, 0));
    assert!(!is_time_in_range(22, 0, 6, 0, 12, 0));
}

#[test]
fn is_day_matching_reads_the_bitmap() {
    assert!(is_day_matching(DOW_BITMAP_THURSDAY, 4));
    assert!(!is_day_matching(DOW_BITMAP_THURSDAY, 3));
    assert!(is_day_matching(DOW_BITMAP_WEEKEND, 0));
    assert!(is_day_matching(DOW_BITMAP_WEEKEND, 6));
    assert!(!is_day_matching(DOW_BITMAP_WEEKEND, 3));
}

#[test]
fn is_week_matching_matches_c_modulo() {
    assert!(is_week_matching(1, 2, 1));
    assert!(is_week_matching(1, 2, 3));
    assert!(!is_week_matching(1, 2, 2));
    assert!(!is_week_matching(1, 2, 0));
}

fn calendar(hour: u32, minute: u32, weekday: u32, week: i32) -> CalendarNow {
    CalendarNow {
        year: 2024,
        month: 1,
        day: 1,
        hour,
        minute,
        weekday,
        week,
    }
}

#[test]
fn double_exp_thursday_recurrence_only_matches_thursday() {
    let thursday = calendar(12, 0, 4, 10);
    let wednesday = calendar(12, 0, 3, 10);
    assert!(RecurringEventKind::DoubleExpThursday.should_be_active(&thursday));
    assert!(!RecurringEventKind::DoubleExpThursday.should_be_active(&wednesday));
}

#[test]
fn double_drop_tuesday_recurrence_only_matches_tuesday() {
    let tuesday = calendar(0, 0, 2, 10);
    let monday = calendar(0, 0, 1, 10);
    assert!(RecurringEventKind::DoubleDropTuesday.should_be_active(&tuesday));
    assert!(!RecurringEventKind::DoubleDropTuesday.should_be_active(&monday));
}

#[test]
fn mining_monday_and_wednesday_recurrence_match_their_weekday() {
    let monday = calendar(3, 0, 1, 10);
    let wednesday = calendar(3, 0, 3, 10);
    assert!(RecurringEventKind::MiningMonday.should_be_active(&monday));
    assert!(!RecurringEventKind::MiningMonday.should_be_active(&wednesday));
    assert!(RecurringEventKind::MiningWednesday.should_be_active(&wednesday));
    assert!(!RecurringEventKind::MiningWednesday.should_be_active(&monday));
}

#[test]
fn bonus_weekend_recurrence_requires_weekend_day_and_matching_biweekly_slot() {
    // C `boosted_weekend_event.c`: `week_number = 1`, `interval = 2` -
    // "on" weeks are those where `current_week % 2 == 1 % 2 == 1`.
    let on_week_saturday = calendar(0, 0, 6, 1);
    let on_week_sunday = calendar(0, 0, 0, 1);
    let off_week_saturday = calendar(0, 0, 6, 2);
    let on_week_weekday = calendar(0, 0, 3, 1);

    assert!(RecurringEventKind::BonusWeekend.should_be_active(&on_week_saturday));
    assert!(RecurringEventKind::BonusWeekend.should_be_active(&on_week_sunday));
    assert!(!RecurringEventKind::BonusWeekend.should_be_active(&off_week_saturday));
    assert!(!RecurringEventKind::BonusWeekend.should_be_active(&on_week_weekday));
}

#[test]
fn check_recurring_events_starts_and_ends_double_exp_thursday_restoring_snapshot() {
    let mut settings = GameSettings::default();
    settings.exp_modifier = 1.2; // simulate a prior admin override
    let mut state = RecurringEventsState::default();

    let thursday = calendar(12, 0, 4, 10);
    let transitions = check_recurring_events(&mut settings, &mut state, &thursday);
    assert_eq!(
        transitions,
        vec![(RecurringEventKind::DoubleExpThursday, true)]
    );
    assert!((settings.exp_modifier - 1.2 * 1.5).abs() < f64::EPSILON);

    // No transition while still Thursday.
    let transitions = check_recurring_events(&mut settings, &mut state, &thursday);
    assert!(transitions.is_empty());
    assert!((settings.exp_modifier - 1.2 * 1.5).abs() < f64::EPSILON);

    // Friday: event ends, restoring the exact pre-event value (not 1.0).
    let friday = calendar(0, 0, 5, 10);
    let transitions = check_recurring_events(&mut settings, &mut state, &friday);
    assert_eq!(
        transitions,
        vec![(RecurringEventKind::DoubleExpThursday, false)]
    );
    assert!((settings.exp_modifier - 1.2).abs() < f64::EPSILON);
}

#[test]
fn check_recurring_events_double_drop_tuesday_sets_and_clears_loot_modifier() {
    let mut settings = GameSettings::default();
    let mut state = RecurringEventsState::default();

    let tuesday = calendar(12, 0, 2, 10);
    check_recurring_events(&mut settings, &mut state, &tuesday);
    assert_eq!(settings.get_loot_modifier("event_drop_rate"), 2.0);

    let wednesday = calendar(12, 0, 3, 10);
    check_recurring_events(&mut settings, &mut state, &wednesday);
    assert_eq!(settings.get_loot_modifier("event_drop_rate"), 1.0);
}

#[test]
fn check_recurring_events_mining_monday_hardcodes_reset_not_snapshot_like_c() {
    let mut settings = GameSettings::default();
    // A prior admin override that C's own dead snapshot code would have
    // captured but never restores - `mining_monday_end` hardcodes `1.0`.
    settings.mining_silver_gold_multiplier = 3.0;
    settings.mining_cavein_multiplier = 3.0;
    settings.mining_golem_event_multiplier = 3.0;
    let mut state = RecurringEventsState::default();

    let monday = calendar(12, 0, 1, 10);
    check_recurring_events(&mut settings, &mut state, &monday);
    assert_eq!(settings.mining_silver_gold_multiplier, 2.0);
    assert_eq!(settings.mining_cavein_multiplier, 2.0);
    assert_eq!(settings.mining_golem_event_multiplier, 2.0);

    let tuesday = calendar(12, 0, 2, 10);
    check_recurring_events(&mut settings, &mut state, &tuesday);
    // Hardcoded back to 1.0, not the pre-event 3.0 snapshot.
    assert_eq!(settings.mining_silver_gold_multiplier, 1.0);
    assert_eq!(settings.mining_cavein_multiplier, 1.0);
    assert_eq!(settings.mining_golem_event_multiplier, 1.0);
}

#[test]
fn check_recurring_events_mining_wednesday_hardcodes_reset_not_snapshot_like_c() {
    let mut settings = GameSettings::default();
    settings.mining_artifact_multiplier = 3.0;
    let mut state = RecurringEventsState::default();

    let wednesday = calendar(12, 0, 3, 10);
    check_recurring_events(&mut settings, &mut state, &wednesday);
    assert_eq!(settings.mining_artifact_multiplier, 5.0);

    let thursday = calendar(12, 0, 4, 10);
    check_recurring_events(&mut settings, &mut state, &thursday);
    assert_eq!(settings.mining_artifact_multiplier, 1.0);
}

#[test]
fn check_recurring_events_bonus_weekend_stacks_exp_and_drop_rate_and_restores_on_end() {
    let mut settings = GameSettings::default();
    settings.exp_modifier = 1.0;
    let mut state = RecurringEventsState::default();

    let on_week_saturday = calendar(12, 0, 6, 1);
    let transitions = check_recurring_events(&mut settings, &mut state, &on_week_saturday);
    assert_eq!(transitions, vec![(RecurringEventKind::BonusWeekend, true)]);
    assert!((settings.exp_modifier - 1.5).abs() < f64::EPSILON);
    assert_eq!(settings.get_loot_modifier("event_drop_rate"), 2.0);

    let off_week_friday = calendar(12, 0, 5, 2);
    let transitions = check_recurring_events(&mut settings, &mut state, &off_week_friday);
    assert_eq!(transitions, vec![(RecurringEventKind::BonusWeekend, false)]);
    assert!((settings.exp_modifier - 1.0).abs() < f64::EPSILON);
    assert_eq!(settings.get_loot_modifier("event_drop_rate"), 1.0);
}

#[test]
fn calendar_now_from_unix_seconds_matches_reference_dates() {
    // 2024-01-04 12:34:00 UTC was a Thursday, week 01 (see week-number
    // fixture above); cross-checked against `date -u -d @<epoch>`.
    let epoch = 1_704_371_640u64; // 2024-01-04T12:34:00Z
    let now = CalendarNow::from_unix_seconds(epoch);
    assert_eq!(now.year, 2024);
    assert_eq!(now.month, 1);
    assert_eq!(now.day, 4);
    assert_eq!(now.weekday, 4);
    assert_eq!(now.hour, 12);
    assert_eq!(now.minute, 34);
    assert_eq!(now.week, 1);
}

#[test]
fn calculate_easter_date_matches_known_reference_years() {
    // Reference Easter Sundays (Gregorian), cross-checked against
    // published dates.
    assert_eq!(calculate_easter_date(2024), (3, 31));
    assert_eq!(calculate_easter_date(2025), (4, 20));
    assert_eq!(calculate_easter_date(2026), (4, 5));
    assert_eq!(calculate_easter_date(2000), (4, 23));
    assert_eq!(calculate_easter_date(2018), (4, 1));
    assert_eq!(calculate_easter_date(2019), (4, 21));
}

#[test]
fn easter_date_range_spans_one_week_before_and_after_with_month_rollover() {
    // 2024 Easter is March 31: window should roll from March into April.
    assert_eq!(easter_date_range(2024), (3, 24, 4, 7));
    // 2018 Easter is April 1: window should roll from March into April.
    assert_eq!(easter_date_range(2018), (3, 25, 4, 8));
    // 2025 Easter is April 20: no month rollover on either side.
    assert_eq!(easter_date_range(2025), (4, 13, 4, 27));
}

#[test]
fn check_easter_event_starts_and_ends_halving_and_restoring_lucky_pentagram_chance() {
    let mut settings = GameSettings::default();
    settings.lucky_pentagram_chance = 50;
    let mut state = EasterEventState::default();

    // 2024-03-31 (Easter Sunday itself): within the window.
    let easter_sunday = CalendarNow {
        year: 2024,
        month: 3,
        day: 31,
        hour: 12,
        minute: 0,
        weekday: 0,
        week: 13,
    };
    let transition = check_easter_event(&mut settings, &mut state, &easter_sunday);
    assert_eq!(transition, Some(true));
    assert_eq!(settings.lucky_pentagram_chance, 25);

    // Still within window: no transition.
    let transition = check_easter_event(&mut settings, &mut state, &easter_sunday);
    assert_eq!(transition, None);
    assert_eq!(settings.lucky_pentagram_chance, 25);

    // Outside the window (well before start): event ends, restoring the
    // exact pre-event value.
    let outside_window = CalendarNow {
        year: 2024,
        month: 2,
        day: 1,
        hour: 12,
        minute: 0,
        weekday: 4,
        week: 5,
    };
    let transition = check_easter_event(&mut settings, &mut state, &outside_window);
    assert_eq!(transition, Some(false));
    assert_eq!(settings.lucky_pentagram_chance, 50);
}

fn decoration_test_loader() -> ZoneLoader {
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
                xmastree_deco:
                    name="Christmas Tree"
                    sprite=900
                    flag=IF_MOVEBLOCK
                ;
                "#,
        )
        .unwrap();
    loader
}

#[test]
fn spawn_event_decoration_places_item_and_records_its_id() {
    let mut world = World::default();
    world.map = ugaris_core::map::MapGrid::new(20, 20);
    let mut loader = decoration_test_loader();
    let mut decoration = EventDecoration::new("xmastree_deco", 5, 5, 0);

    assert!(spawn_event_decoration(
        &mut world,
        &mut loader,
        &mut decoration
    ));

    let item_id = decoration
        .spawned_item
        .expect("decoration should be spawned");
    let item = world.items.get(&item_id).unwrap();
    assert_eq!(item.name, "Christmas Tree");
    assert_eq!((item.x, item.y), (5, 5));
    assert_eq!(world.map.tile(5, 5).unwrap().item, item_id.0);
}

#[test]
fn spawn_event_decoration_is_a_noop_when_already_spawned() {
    let mut world = World::default();
    world.map = ugaris_core::map::MapGrid::new(20, 20);
    let mut loader = decoration_test_loader();
    let mut decoration = EventDecoration::new("xmastree_deco", 5, 5, 0);

    assert!(spawn_event_decoration(
        &mut world,
        &mut loader,
        &mut decoration
    ));
    let first_id = decoration.spawned_item;
    assert!(!spawn_event_decoration(
        &mut world,
        &mut loader,
        &mut decoration
    ));
    assert_eq!(decoration.spawned_item, first_id);
    // Only one item was ever created.
    assert_eq!(world.items.len(), 1);
}

#[test]
fn spawn_event_decoration_respects_area_gate() {
    let mut world = World::default();
    world.area_id = 3;
    world.map = ugaris_core::map::MapGrid::new(20, 20);
    let mut loader = decoration_test_loader();
    let mut wrong_area = EventDecoration::new("xmastree_deco", 5, 5, 7);

    assert!(!spawn_event_decoration(
        &mut world,
        &mut loader,
        &mut wrong_area
    ));
    assert!(wrong_area.spawned_item.is_none());
    assert!(world.items.is_empty());

    let mut right_area = EventDecoration::new("xmastree_deco", 6, 6, 3);
    assert!(spawn_event_decoration(
        &mut world,
        &mut loader,
        &mut right_area
    ));
    assert!(right_area.spawned_item.is_some());

    let mut any_area = EventDecoration::new("xmastree_deco", 7, 7, 0);
    assert!(spawn_event_decoration(
        &mut world,
        &mut loader,
        &mut any_area
    ));
    assert!(any_area.spawned_item.is_some());
}

#[test]
fn remove_event_decoration_destroys_item_and_clears_map_tile() {
    let mut world = World::default();
    world.map = ugaris_core::map::MapGrid::new(20, 20);
    let mut loader = decoration_test_loader();
    let mut decoration = EventDecoration::new("xmastree_deco", 5, 5, 0);
    spawn_event_decoration(&mut world, &mut loader, &mut decoration);
    let item_id = decoration.spawned_item.unwrap();

    remove_event_decoration(&mut world, &mut decoration);

    assert!(decoration.spawned_item.is_none());
    assert!(!world.items.contains_key(&item_id));
    assert_eq!(world.map.tile(5, 5).unwrap().item, 0);
}

#[test]
fn remove_event_decoration_is_a_noop_when_not_spawned_or_already_removed() {
    let mut world = World::default();
    world.map = ugaris_core::map::MapGrid::new(20, 20);
    let mut decoration = EventDecoration::new("xmastree_deco", 5, 5, 0);

    // Never spawned: no panic, stays `None`.
    remove_event_decoration(&mut world, &mut decoration);
    assert!(decoration.spawned_item.is_none());

    // Spawned then already removed by some other means (e.g. destroyed
    // directly): matches C's `it[decoration->in].flags` liveness guard.
    let mut loader = decoration_test_loader();
    spawn_event_decoration(&mut world, &mut loader, &mut decoration);
    let item_id = decoration.spawned_item.unwrap();
    world.destroy_item(item_id);
    remove_event_decoration(&mut world, &mut decoration);
    assert!(decoration.spawned_item.is_none());
}

#[test]
fn remove_event_decoration_respects_area_gate() {
    let mut world = World::default();
    world.area_id = 3;
    world.map = ugaris_core::map::MapGrid::new(20, 20);
    let mut loader = decoration_test_loader();
    let mut decoration = EventDecoration::new("xmastree_deco", 5, 5, 3);
    spawn_event_decoration(&mut world, &mut loader, &mut decoration);
    let item_id = decoration.spawned_item.unwrap();

    world.area_id = 4; // area changed out from under the decoration
    remove_event_decoration(&mut world, &mut decoration);

    // Not removed: the decoration still thinks it's spawned, and the item
    // is untouched, matching C's early-return on an area mismatch.
    assert_eq!(decoration.spawned_item, Some(item_id));
    assert!(world.items.contains_key(&item_id));
}
