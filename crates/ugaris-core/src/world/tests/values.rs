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
