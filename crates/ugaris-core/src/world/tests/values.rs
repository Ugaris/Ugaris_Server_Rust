use super::*;

fn world_with_character(id: u32) -> World {
    let mut world = World::default();
    world.characters.insert(CharacterId(id), character(id));
    world
}

#[test]
fn queue_showvalues_command_empty_argument_reports_no_player() {
    let mut world = world_with_character(1);
    world.queue_showvalues_command(CharacterId(1), "");
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].message, "No player by that name.");
    assert!(world.drain_pending_showvalues_requests().is_empty());
}

#[test]
fn queue_showvalues_command_invalid_shape_reports_no_player_immediately() {
    let mut world = world_with_character(1);
    // Embedded space fails `lookup_name`'s isalpha-only gate, same as
    // C's own inline scan.
    world.queue_showvalues_command(CharacterId(1), "a b");
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].message, "No player by that name.");
    assert!(world.drain_pending_showvalues_requests().is_empty());
}

#[test]
fn queue_showvalues_command_valid_shape_queues_a_request() {
    let mut world = world_with_character(1);
    world.queue_showvalues_command(CharacterId(1), "  Someone");
    assert!(world.drain_pending_system_texts().is_empty());
    let requests = world.drain_pending_showvalues_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].caller_id, CharacterId(1));
    assert_eq!(requests[0].target_name, "Someone");
    assert!(world.drain_pending_showvalues_requests().is_empty());
}

fn set_value(character: &mut Character, value: CharacterValue, present: i32, base: i32) {
    character.values[1][value as usize] = present as i16;
    character.values[0][value as usize] = base as i16;
}

#[test]
fn show_values_lines_warrior_header_and_first_line_match_c_shape() {
    let mut actor = character(1);
    actor.name = "Conan".into();
    actor.level = 12;
    actor.flags.insert(CharacterFlags::WARRIOR);
    set_value(&mut actor, CharacterValue::Hp, 50, 45);
    set_value(&mut actor, CharacterValue::Endurance, 30, 25);
    set_value(&mut actor, CharacterValue::Wisdom, 20, 15);

    let lines = show_values_lines(&actor, &HashMap::new());
    assert_eq!(lines[0], "Conan, Level 12, Warrior");
    assert_eq!(
        lines[1],
        "Hitpoints: 50/45 \u{8}Endurance: 30/25 \u{10}Wisdom: 20/15"
    );
    // Header + 8 triple lines + 1 pair line + armor line + offence line.
    assert_eq!(lines.len(), 12);
}

#[test]
fn show_values_lines_warrior_final_pair_line_uses_two_columns_only() {
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::WARRIOR);
    set_value(&mut actor, CharacterValue::Rage, 7, 3);
    set_value(&mut actor, CharacterValue::Profession, 2, 1);

    let lines = show_values_lines(&actor, &HashMap::new());
    // Warrior branch: header (1) + 8 triple lines = lines[0..=8], then the
    // final Rage/Profession pair line at index 9.
    assert_eq!(lines[9], "Rage: 7/3 \u{8}Profession: 2/1");
}

#[test]
fn show_values_lines_arch_prefix_is_included() {
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::WARRIOR);
    actor.flags.insert(CharacterFlags::ARCH);
    let lines = show_values_lines(&actor, &HashMap::new());
    assert_eq!(lines[0], "Character, Level 1, Arch-Warrior");
}

#[test]
fn show_values_lines_seyandu_uses_both_flags_and_full_line_count() {
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::WARRIOR);
    actor.flags.insert(CharacterFlags::MAGE);
    let lines = show_values_lines(&actor, &HashMap::new());
    assert_eq!(lines[0], "Character, Level 1, Seyan'Du");
    // Header + 11 triple lines + armor line + offence line.
    assert_eq!(lines.len(), 14);
}

#[test]
fn show_values_lines_mage_only_uses_mage_class_and_line_count() {
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::MAGE);
    let lines = show_values_lines(&actor, &HashMap::new());
    assert_eq!(lines[0], "Character, Level 1, Mage");
    // Header + 8 triple lines + armor line + offence line.
    assert_eq!(lines.len(), 11);
}

const DAY: i64 = 60 * 60 * 24;

#[test]
fn compute_paid_till_never_paid_grants_28_day_free_grace_from_creation() {
    let created = 1_000_000_i64;
    let now = created + DAY * 3;
    let (t, is_paid) = compute_paid_till(None, created, now);
    assert!(!is_paid);
    // Rounded-up-to-even-day: (created + 28d + DAY - 1) & !1.
    let expected = (created + DAY * 28 + DAY - 1) & !1;
    assert_eq!(t, expected);
    assert_eq!(t % 2, 0);
}

#[test]
fn compute_paid_till_zero_raw_value_is_treated_as_never_paid() {
    let created = 500_000_i64;
    let now = created + DAY;
    let (t, is_paid) = compute_paid_till(Some(0), created, now);
    assert!(!is_paid);
    assert_eq!(t, (created + DAY * 28 + DAY - 1) & !1);
}

#[test]
fn compute_paid_till_future_expiration_rounds_up_to_even_day() {
    let created = 0_i64;
    let now = 10_000_i64;
    // An even raw value, in the future - regular paid account branch.
    let raw = now + DAY * 5 + 1234;
    let (t, is_paid) = compute_paid_till(Some(raw), created, now);
    assert!(is_paid);
    assert_eq!(t, (raw + DAY - 1) & !1);
    assert_eq!(t % 2, 0);
    assert!(t >= now);
}

#[test]
fn compute_paid_till_odd_raw_value_is_a_12_hour_account_passed_through() {
    let created = 0_i64;
    let now = 10_000_i64;
    // Odd raw value, in the future - the "12 hour paid account" marker;
    // C passes it straight through unrounded.
    let raw = now + 6 * 60 * 60 + 1;
    assert!(raw % 2 == 1);
    let (t, is_paid) = compute_paid_till(Some(raw), created, now);
    assert!(is_paid);
    assert_eq!(t, raw);
    assert_eq!(t % 2, 1);
}

#[test]
fn compute_paid_till_past_expiration_still_within_four_week_creation_window_counts_as_paid() {
    // C's second OR-branch: `paid_till > creation_time + 4 weeks` also
    // counts as paid even if `paid_till <= now` - covers the case of a
    // very-recently-created account whose raw paid_till already passed
    // but is still further out than 4 weeks from creation (a data
    // oddity C tolerates rather than rejects).
    let created = 0_i64;
    let now = DAY * 40;
    let raw = DAY * 35; // <= now (already "expired"), but > created + 28d.
    assert!(raw <= now);
    assert!(raw > created + DAY * 7 * 4);
    let (_, is_paid) = compute_paid_till(Some(raw), created, now);
    assert!(is_paid);
}

#[test]
fn compute_paid_till_past_expiration_outside_four_week_window_falls_back_to_free_grace() {
    let created = 0_i64;
    let now = DAY * 100;
    let raw = DAY * 10; // well in the past, well within 4 weeks of creation.
    let (t, is_paid) = compute_paid_till(Some(raw), created, now);
    assert!(!is_paid);
    assert_eq!(t, (created + DAY * 28 + DAY - 1) & !1);
}

#[test]
fn paid_player_line_even_t_reports_whole_days_left() {
    let now = 0_i64;
    let t = DAY * 5; // even
    assert_eq!(t % 2, 0);
    assert_eq!(
        paid_player_line(true, t, now),
        "Paying player: yes (5 days left)"
    );
    assert_eq!(
        paid_player_line(false, t, now),
        "Paying player: no (5 days left)"
    );
}

#[test]
fn paid_player_line_odd_t_reports_hh_mm_ss_countdown() {
    let now = 0_i64;
    let t = 6 * 60 * 60 + 30 * 60 + 15; // odd, 06:30:15
    assert_eq!(t % 2, 1);
    assert_eq!(
        paid_player_line(true, t, now),
        "Paying player: yes (06:30:15 hours left)"
    );
}

#[test]
fn show_values_lines_final_lines_report_armor_weapon_speed_and_combat_values() {
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::MAGE);
    set_value(&mut actor, CharacterValue::Armor, 0, 40);
    set_value(&mut actor, CharacterValue::Weapon, 0, 12);
    set_value(&mut actor, CharacterValue::Speed, 0, 5);

    let lines = show_values_lines(&actor, &HashMap::new());
    let armor_line = &lines[lines.len() - 2];
    assert_eq!(armor_line, "Armor: 2.00 \u{8}Weapon: 12 \u{10}Speed: 5");
    let offence_line = &lines[lines.len() - 1];
    assert!(offence_line.starts_with("Offence: "));
    assert!(offence_line.contains("\u{8}Defence: "));
}
