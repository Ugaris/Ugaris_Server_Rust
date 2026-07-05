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

// C `level2maxitem(level)` (`tool.c:2516-2577`): ascending threshold ladder,
// every boundary checked on both sides.
#[test]
fn level2maxitem_matches_legacy_threshold_ladder() {
    assert_eq!(level2maxitem(0), 0);
    assert_eq!(level2maxitem(1), 0);
    assert_eq!(level2maxitem(2), 1);
    assert_eq!(level2maxitem(3), 2);
    assert_eq!(level2maxitem(4), 2);
    assert_eq!(level2maxitem(5), 3);
    assert_eq!(level2maxitem(9), 3);
    assert_eq!(level2maxitem(10), 4);
    assert_eq!(level2maxitem(14), 4);
    assert_eq!(level2maxitem(15), 5);
    assert_eq!(level2maxitem(16), 5);
    assert_eq!(level2maxitem(17), 6);
    assert_eq!(level2maxitem(19), 6);
    assert_eq!(level2maxitem(20), 7);
    assert_eq!(level2maxitem(22), 7);
    assert_eq!(level2maxitem(23), 8);
    assert_eq!(level2maxitem(25), 8);
    assert_eq!(level2maxitem(26), 9);
    assert_eq!(level2maxitem(29), 9);
    assert_eq!(level2maxitem(30), 10);
    assert_eq!(level2maxitem(32), 10);
    assert_eq!(level2maxitem(33), 11);
    assert_eq!(level2maxitem(35), 11);
    assert_eq!(level2maxitem(36), 12);
    assert_eq!(level2maxitem(39), 12);
    assert_eq!(level2maxitem(40), 13);
    assert_eq!(level2maxitem(42), 13);
    assert_eq!(level2maxitem(43), 14);
    assert_eq!(level2maxitem(45), 14);
    assert_eq!(level2maxitem(46), 15);
    assert_eq!(level2maxitem(49), 15);
    assert_eq!(level2maxitem(50), 16);
    assert_eq!(level2maxitem(52), 16);
    assert_eq!(level2maxitem(53), 17);
    assert_eq!(level2maxitem(55), 17);
    assert_eq!(level2maxitem(56), 18);
    assert_eq!(level2maxitem(59), 18);
    assert_eq!(level2maxitem(60), 19);
    assert_eq!(level2maxitem(62), 19);
    assert_eq!(level2maxitem(63), 20);
    assert_eq!(level2maxitem(200), 20);
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
fn check_levelup_queues_a_grats_channel_broadcast_at_level_ten() {
    let mut world = World::default();
    let mut player = character(1);
    player.level = 9;
    player.exp = level2exp(10);
    assert!(world.spawn_character(player, 10, 10));

    world.check_levelup(CharacterId(1));

    let broadcasts = world.drain_pending_channel_broadcasts();
    assert_eq!(broadcasts.len(), 1);
    assert_eq!(
        broadcasts[0].channel, 6,
        "C: server_chat(6, ...) is the Grats channel"
    );
    let mut expected = b"0000000000".to_vec();
    expected.extend_from_slice(crate::text::COL_CHAT_GRATS);
    expected.extend_from_slice(b"Grats: Character is level 10 now!");
    assert_eq!(broadcasts[0].message_bytes, expected);
}

#[test]
fn check_levelup_does_not_queue_a_grats_broadcast_for_non_multiple_of_ten_levels() {
    let mut world = World::default();
    let mut player = character(1);
    player.exp = 16; // exp2level(16) == 2, not a multiple of 10.
    assert!(world.spawn_character(player, 10, 10));

    world.check_levelup(CharacterId(1));

    assert!(world.drain_pending_channel_broadcasts().is_empty());
}

#[test]
fn check_levelup_queues_one_grats_broadcast_per_multiple_of_ten_when_gaining_several_levels() {
    let mut world = World::default();
    let mut player = character(1);
    player.level = 8;
    player.exp = level2exp(21); // gains levels 9..21, crossing 10 and 20.
    assert!(world.spawn_character(player, 10, 10));

    world.check_levelup(CharacterId(1));

    let broadcasts = world.drain_pending_channel_broadcasts();
    assert_eq!(broadcasts.len(), 2);
    assert!(broadcasts[0]
        .message_bytes
        .ends_with(b"Grats: Character is level 10 now!"));
    assert!(broadcasts[1]
        .message_bytes
        .ends_with(b"Grats: Character is level 20 now!"));
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

// C `give_exp(cn, val)` (`tool.c:1371-1423`).
#[test]
fn give_exp_applies_global_modifier_and_marks_update() {
    let mut world = World::default();
    world.settings.exp_modifier = 2.0;
    let mut player = character(1);
    player.exp = 10;
    assert!(world.spawn_character(player, 10, 10));

    world.give_exp(CharacterId(1), 5, 1);

    let character = &world.characters[&CharacterId(1)];
    assert_eq!(character.exp, 20); // 10 + 5*2.0
    assert!(character.flags.contains(CharacterFlags::UPDATE));
}

#[test]
fn give_exp_applies_hardcore_bonus_before_global_modifier() {
    let mut world = World::default();
    world.settings.exp_modifier = 2.0;
    world.settings.hardcore_exp_bonus = 1.5;
    let mut player = character(1);
    player.exp = 0;
    player.flags.insert(CharacterFlags::HARDCORE);
    assert!(world.spawn_character(player, 10, 10));

    world.give_exp(CharacterId(1), 10, 1);

    // C: addedExp = 10 * 1.5 (hardcore) * 2.0 (global) = 30.
    assert_eq!(world.characters[&CharacterId(1)].exp, 30);
}

#[test]
fn give_exp_is_a_noop_for_noexp_characters() {
    let mut world = World::default();
    let mut player = character(1);
    player.exp = 10;
    player.flags.insert(CharacterFlags::NOEXP);
    assert!(world.spawn_character(player, 10, 10));

    world.give_exp(CharacterId(1), 100, 1);

    assert_eq!(world.characters[&CharacterId(1)].exp, 10);
}

#[test]
fn give_exp_is_a_noop_in_area_21() {
    let mut world = World::default();
    let mut player = character(1);
    player.exp = 10;
    assert!(world.spawn_character(player, 10, 10));

    world.give_exp(CharacterId(1), 100, 21);

    assert_eq!(world.characters[&CharacterId(1)].exp, 10);
}

#[test]
fn give_exp_caps_nolevel_characters_at_the_next_level_threshold() {
    let mut world = World::default();
    let mut player = character(1);
    player.level = 1;
    player.exp = level2exp(1);
    player.flags.insert(CharacterFlags::NOLEVEL);
    assert!(world.spawn_character(player, 10, 10));

    // A huge grant would normally exceed level2exp(2) = 16.
    world.give_exp(CharacterId(1), 1000, 1);

    let character = &world.characters[&CharacterId(1)];
    assert_eq!(character.exp, level2exp(2) - 1);
    assert_eq!(character.level, 1, "NOLEVEL must never level up");
}

#[test]
fn give_exp_floors_nolevel_characters_back_to_their_level_band_on_negative_grants() {
    let mut world = World::default();
    let mut player = character(1);
    player.level = 2;
    player.exp = level2exp(2);
    player.flags.insert(CharacterFlags::NOLEVEL);
    assert!(world.spawn_character(player, 10, 10));

    world.give_exp(CharacterId(1), -1000, 1);

    assert_eq!(world.characters[&CharacterId(1)].exp, level2exp(2));
}

#[test]
fn give_exp_prevents_unexpected_decrease_from_a_positive_grant() {
    // C: `if (newExp < currentExp && addedExp > 0) newExp = currentExp;`
    // guards against the i64->u32 saturating clamp ever moving exp
    // backwards on an actual positive grant.
    let mut world = World::default();
    let mut player = character(1);
    player.exp = u32::MAX;
    assert!(world.spawn_character(player, 10, 10));

    world.give_exp(CharacterId(1), 5, 1);

    assert_eq!(world.characters[&CharacterId(1)].exp, u32::MAX);
}

#[test]
fn give_exp_triggers_check_levelup_unless_nolevel() {
    let mut world = World::default();
    let mut player = character(1);
    player.exp = 0;
    assert!(world.spawn_character(player, 10, 10));

    world.give_exp(CharacterId(1), 16, 1); // exp2level(16) == 2

    assert_eq!(world.characters[&CharacterId(1)].level, 2);
}

// C `check_levelup`'s `if (ch[cn].flags & CF_PLAYER) achievement_check_
// level(cn, ch[cn].level);` (`tool.c:1352-1354`), queued for the server
// crate as `LevelAchievementCheck`.
#[test]
fn check_levelup_queues_a_level_achievement_check_per_level_for_players() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.exp = 81; // exp2level(81) == 3, two levels above the starting 1.
    assert!(world.spawn_character(player, 10, 10));

    world.check_levelup(CharacterId(1));

    let checks = world.drain_pending_level_achievements();
    assert_eq!(checks.len(), 2);
    assert_eq!(checks[0].character_id, CharacterId(1));
    assert_eq!(checks[0].level, 2);
    assert!(!checks[0].is_hardcore);
    assert_eq!(checks[1].level, 3);
}

#[test]
fn check_levelup_does_not_queue_a_level_achievement_check_for_non_players() {
    let mut world = World::default();
    let mut player = character(1); // not `CharacterFlags::PLAYER` (NPC)
    player.exp = 16;
    assert!(world.spawn_character(player, 10, 10));

    world.check_levelup(CharacterId(1));

    assert!(world.drain_pending_level_achievements().is_empty());
}

#[test]
fn check_levelup_queues_hardcore_flag_on_the_level_achievement_check() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.flags.insert(CharacterFlags::HARDCORE);
    player.exp = 16;
    assert!(world.spawn_character(player, 10, 10));

    world.check_levelup(CharacterId(1));

    let checks = world.drain_pending_level_achievements();
    assert_eq!(checks.len(), 1);
    assert!(checks[0].is_hardcore);
}

#[test]
fn drain_pending_level_achievements_is_empty_when_nothing_leveled() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.level = 5;
    player.exp = 0;
    assert!(world.spawn_character(player, 10, 10));

    world.check_levelup(CharacterId(1));

    assert!(world.drain_pending_level_achievements().is_empty());
}
