use super::*;

// C `exp2level`/`level2exp` (`src/system/tool.c:1272-1279`):
// level2exp(level) = level^4, exp2level(exp) = max(1, floor(exp^0.25)).
#[test]
fn exp2level_and_level2exp_match_legacy_fourth_power_formula() {
    assert_eq!(level2exp(1), 1);
    assert_eq!(level2exp(2), 16);
    assert_eq!(level2exp(3), 81);
    assert_eq!(level2exp(20), 160_000);

    assert_eq!(exp2level(0), 1, "C: max(1, ...) floors negative/zero at 1");
    assert_eq!(exp2level(15), 1);
    assert_eq!(exp2level(16), 2);
    assert_eq!(exp2level(80), 2);
    assert_eq!(exp2level(81), 3);
    assert_eq!(exp2level(160_000), 20);
}

// C `level_value(level)` (`tool.c:1282`): pow(level+1,4) - pow(level,4).
#[test]
fn level_value_is_the_gap_between_consecutive_level2exp() {
    assert_eq!(level_value(1), level2exp(2) - level2exp(1));
    assert_eq!(level_value(10), level2exp(11) - level2exp(10));
}

#[test]
fn check_levelup_grants_one_level_and_a_save_with_feedback_text() {
    let mut world = World::default();
    let mut player = character(1);
    player.exp = 16; // exp2level(16) == 2
    assert!(world.spawn_character(player, 10, 10));

    let leveled = world.check_levelup(CharacterId(1));

    assert!(leveled);
    let character = &world.characters[&CharacterId(1)];
    assert_eq!(character.level, 2);
    assert_eq!(character.saves, 1);
    let feedback = world.drain_pending_system_texts();
    assert!(feedback
        .iter()
        .any(|t| t.message == "Thou gained a level! Thou art level 2 now."));
    assert!(feedback
        .iter()
        .any(|t| t.message.contains("pleased Ishtar")));
    assert!(feedback
        .iter()
        .any(|t| t.message == "Thou hast one saves now."));
}

#[test]
fn check_levelup_grants_multiple_levels_in_one_call() {
    let mut world = World::default();
    let mut player = character(1);
    player.exp = 81; // exp2level(81) == 3, two levels above the starting 1.
    assert!(world.spawn_character(player, 10, 10));

    let leveled = world.check_levelup(CharacterId(1));

    assert!(leveled);
    let character = &world.characters[&CharacterId(1)];
    assert_eq!(character.level, 3);
    assert_eq!(character.saves, 2);
    let feedback = world.drain_pending_system_texts();
    assert!(feedback
        .iter()
        .any(|t| t.message == "Thou gained a level! Thou art level 2 now."));
    assert!(feedback
        .iter()
        .any(|t| t.message == "Thou gained a level! Thou art level 3 now."));
}

#[test]
fn check_levelup_hardcore_resets_saves_instead_of_granting_them() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags |= CharacterFlags::HARDCORE;
    player.saves = 5;
    player.exp = 16;
    assert!(world.spawn_character(player, 10, 10));

    world.check_levelup(CharacterId(1));

    let character = &world.characters[&CharacterId(1)];
    assert_eq!(character.saves, 0, "C: hardcore levelup resets saves to 0");
    let feedback = world.drain_pending_system_texts();
    assert!(
        !feedback.iter().any(|t| t.message.contains("Ishtar")),
        "hardcore characters don't get the save-grant text"
    );
}

#[test]
fn check_levelup_caps_saves_at_ten() {
    let mut world = World::default();
    let mut player = character(1);
    player.saves = 10;
    player.exp = 16;
    assert!(world.spawn_character(player, 10, 10));

    world.check_levelup(CharacterId(1));

    let character = &world.characters[&CharacterId(1)];
    assert_eq!(character.saves, 10);
}

#[test]
fn check_levelup_unlocks_profession_choice_at_level_twenty() {
    let mut world = World::default();
    let mut player = character(1);
    player.level = 19;
    player.exp = level2exp(20);
    assert!(world.spawn_character(player, 10, 10));

    world.check_levelup(CharacterId(1));

    let character = &world.characters[&CharacterId(1)];
    assert_eq!(character.level, 20);
    assert_eq!(character.values[1][CharacterValue::Profession as usize], 1);
    let feedback = world.drain_pending_system_texts();
    assert!(feedback
        .iter()
        .any(|t| t.message == "Thou mayest now choose to learn a profession."));
}

#[test]
fn check_levelup_does_not_overwrite_an_already_chosen_profession() {
    let mut world = World::default();
    let mut player = character(1);
    player.level = 19;
    player.exp = level2exp(20);
    player.values[1][CharacterValue::Profession as usize] = 3; // already chosen
    assert!(world.spawn_character(player, 10, 10));

    world.check_levelup(CharacterId(1));

    let character = &world.characters[&CharacterId(1)];
    assert_eq!(character.values[1][CharacterValue::Profession as usize], 3);
    let feedback = world.drain_pending_system_texts();
    assert!(!feedback
        .iter()
        .any(|t| t.message.contains("choose to learn a profession")));
}

#[test]
fn check_levelup_uses_the_higher_of_exp_and_exp_used() {
    let mut world = World::default();
    let mut player = character(1);
    // C: `experience = max(ch[cn].exp, ch[cn].exp_used)`.
    player.exp = 0;
    player.exp_used = 16;
    assert!(world.spawn_character(player, 10, 10));

    let leveled = world.check_levelup(CharacterId(1));

    assert!(leveled);
    assert_eq!(world.characters[&CharacterId(1)].level, 2);
}

#[test]
fn check_levelup_is_a_noop_when_exp_does_not_exceed_current_level() {
    let mut world = World::default();
    let mut player = character(1);
    player.exp = 15; // exp2level(15) == 1, same as starting level.
    assert!(world.spawn_character(player, 10, 10));

    let leveled = world.check_levelup(CharacterId(1));

    assert!(!leveled);
    let character = &world.characters[&CharacterId(1)];
    assert_eq!(character.level, 1);
    assert_eq!(character.saves, 0);
    assert!(world.drain_pending_system_texts().is_empty());
}

#[test]
fn check_levelup_returns_false_for_unknown_character() {
    let mut world = World::default();

    assert!(!world.check_levelup(CharacterId(42)));
}
