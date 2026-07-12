use super::*;
use crate::player::MAX_TUNNEL_LEVEL;

fn facts(reward_level: i32, used: &[(i32, u8)]) -> TunnelRewardFacts {
    let mut tunnel_used = vec![0u8; (MAX_TUNNEL_LEVEL as usize) + 1];
    for &(level, value) in used {
        tunnel_used[level as usize] = value;
    }
    TunnelRewardFacts {
        reward_level,
        tunnel_used,
    }
}

// C `give_reward`'s `DOOR_EXIT_EXP` branch (`tunnel.c:542-547`): a fresh
// (never-completed) level grants `level_value(reward_level) /
// tunnel_exp_base_value_divider / (used[reward_level] + 9)` exp - the
// denominator reads the *post*-increment `used` count (`1 + 9 = 10` here).
#[test]
fn exit_exp_first_completion_grants_expected_exp_and_progress_message() {
    let mut world = World::default();
    let mut player = character(1);
    player.level = 60;
    assert!(world.spawn_character(player, 10, 10));

    let facts = facts(50, &[]);
    let outcome = world.apply_tunnel_reward(CharacterId(1), &facts, 2, 33);

    assert_eq!(outcome.new_used_count, Some((50, 1)));
    assert_eq!(outcome.promote_gorwin_to, None);
    assert!(outcome.award_achievement);
    assert_eq!(
        outcome.messages,
        vec![
            "You have been given experience.".to_string(),
            "Completions at level 50: 1/10 (9 remaining).".to_string(),
        ]
    );
    // level_value(50) = 51^4 - 50^4 = 515201; /5.0 (default divider) =
    // 103040.2; /(1+9) = 10304.02 -> truncated to 10304.
    assert_eq!(world.characters[&CharacterId(1)].exp, 10304);
}

// C `give_reward`'s `DOOR_EXIT_MILITARY` branch (`tunnel.c:548-554`):
// `(tunnel_mill_exp_base_value + reward_level^2/10) / (used + 9)`, all
// integer math.
#[test]
fn exit_military_first_completion_grants_expected_points() {
    let mut world = World::default();
    let mut player = character(1);
    player.level = 60;
    assert!(world.spawn_character(player, 10, 10));

    let facts = facts(50, &[]);
    let outcome = world.apply_tunnel_reward(CharacterId(1), &facts, 3, 33);

    assert_eq!(outcome.new_used_count, Some((50, 1)));
    assert!(outcome.award_achievement);
    assert_eq!(outcome.messages[0], "You have been given military rank.");
    // (100 + 50*50/10) / (1+9) = (100+250)/10 = 35.
    assert_eq!(world.characters[&CharacterId(1)].military_points, 35);
}

// C `give_reward`'s auto-promote-on-mastery branch (`tunnel.c:559-580`),
// "next level found" arm.
#[test]
fn exit_reward_auto_promotes_to_next_available_level_on_final_use() {
    let mut world = World::default();
    let mut player = character(1);
    player.level = 100;
    assert!(world.spawn_character(player, 10, 10));

    // 9 completions already recorded - this reward is the 10th (final).
    let facts = facts(50, &[(50, 9)]);
    let outcome = world.apply_tunnel_reward(CharacterId(1), &facts, 2, 33);

    assert_eq!(outcome.new_used_count, Some((50, 10)));
    assert_eq!(outcome.promote_gorwin_to, Some(51));
    assert!(outcome.award_achievement);
    assert_eq!(
        outcome.messages,
        vec![
            "You have been given experience.".to_string(),
            "Tunnel Mastery! Thou hast conquered all 10 challenges at level 50.".to_string(),
            "Gorwin has advanced thy tunnel level to 51. Onward and upward!".to_string(),
        ]
    );
}

// Same branch, "no next level available" arm (`tunnel.c:572-577`): the
// character's own level caps how high `find_next_available_level` can
// search.
#[test]
fn exit_reward_on_final_use_with_no_higher_level_available_reports_mastery() {
    let mut world = World::default();
    let mut player = character(1);
    player.level = 50;
    assert!(world.spawn_character(player, 10, 10));

    let facts = facts(50, &[(50, 9)]);
    let outcome = world.apply_tunnel_reward(CharacterId(1), &facts, 2, 33);

    assert_eq!(outcome.promote_gorwin_to, None);
    assert_eq!(
        outcome.messages,
        vec![
            "You have been given experience.".to_string(),
            "Tunnel Mastery! Thou hast conquered all 10 challenges at level 50.".to_string(),
            "There are no more tunnel levels available to thee. Thou art a true master of the depths!"
                .to_string(),
        ]
    );
}

// C `give_reward`'s `else` branch (`tunnel.c:587-599`): the level was
// already fully completed before this use, so no reward is granted at
// all, but a still-reachable higher level auto-promotes anyway.
#[test]
fn exit_reward_on_already_maxed_level_grants_no_reward_but_still_promotes() {
    let mut world = World::default();
    let mut player = character(1);
    player.level = 100;
    assert!(world.spawn_character(player, 10, 10));

    let facts = facts(50, &[(50, 10)]);
    let outcome = world.apply_tunnel_reward(CharacterId(1), &facts, 2, 33);

    assert_eq!(outcome.new_used_count, None);
    assert!(!outcome.award_achievement);
    assert_eq!(outcome.promote_gorwin_to, Some(51));
    assert_eq!(
        outcome.messages,
        vec![
            "You have used all 10 completions at level 50. No reward given.".to_string(),
            "Gorwin has advanced thy tunnel level to 51. Speak with him for details.".to_string(),
        ]
    );
    // No exp/military points were granted.
    assert_eq!(world.characters[&CharacterId(1)].exp, 0);
    assert_eq!(world.characters[&CharacterId(1)].military_points, 0);
}

// Same "already maxed" branch, but no higher level exists either - only
// the "no reward given" line is emitted.
#[test]
fn exit_reward_on_already_maxed_level_with_no_promotion_available() {
    let mut world = World::default();
    let mut player = character(1);
    player.level = 50;
    assert!(world.spawn_character(player, 10, 10));

    let facts = facts(50, &[(50, 10)]);
    let outcome = world.apply_tunnel_reward(CharacterId(1), &facts, 2, 33);

    assert_eq!(outcome.promote_gorwin_to, None);
    assert_eq!(
        outcome.messages,
        vec!["You have used all 10 completions at level 50. No reward given.".to_string()]
    );
}

// C `check_area_clear` (`tunnel.c:750-762`): an empty rectangle in front
// of the door is clear.
#[test]
fn mean_door_area_clear_is_true_when_the_rectangle_ahead_is_empty() {
    let world = World::default();
    assert!(world.tunnel_mean_door_area_clear(10, 10));
}

// A non-player character anywhere in the `DOOR_RANGE`x`DOOR_DEPTH`
// rectangle blocks the door from opening.
#[test]
fn mean_door_area_clear_is_false_when_a_non_player_character_is_in_range() {
    let mut world = World::default();
    let mut baddy = character(1);
    baddy.flags = CharacterFlags::USED;
    assert!(world.spawn_character(baddy, 10, 15));

    assert!(!world.tunnel_mean_door_area_clear(10, 10));
}

// Players in the rectangle don't block the door - only non-player
// characters count (`ch[co].flags & CF_PLAYER`).
#[test]
fn mean_door_area_clear_ignores_player_characters_in_range() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags = CharacterFlags::USED | CharacterFlags::PLAYER;
    assert!(world.spawn_character(player, 10, 15));

    assert!(world.tunnel_mean_door_area_clear(10, 10));
}

// Characters outside the rectangle (too far horizontally, or above/at the
// door's own row) don't block it.
#[test]
fn mean_door_area_clear_ignores_characters_outside_the_rectangle() {
    let mut world = World::default();
    // Horizontally out of DOOR_RANGE (4) from x=10.
    let mut far = character(1);
    far.flags = CharacterFlags::USED;
    assert!(world.spawn_character(far, 20, 15));
    // At the door's own row (y+1 is the first checked row).
    let mut same_row = character(2);
    same_row.flags = CharacterFlags::USED;
    assert!(world.spawn_character(same_row, 10, 10));

    assert!(world.tunnel_mean_door_area_clear(10, 10));
}
