//! `World::process_player_autobless_autopulse` (C `player_driver.c:1067-
//! 1070`'s normal-connected-player autobless/autopulse consumer).

use super::*;

fn player_character(id: u32) -> Character {
    let mut character = character(id);
    character.flags |= CharacterFlags::PLAYER | CharacterFlags::ALIVE;
    character
}

#[test]
fn autobless_casts_bless_self_when_enabled_and_affordable() {
    let mut world = World::default();
    let mut player = player_character(1);
    player.values[0][CharacterValue::Bless as usize] = 20;
    player.mana = BLESS_COST;
    assert!(world.spawn_character(player, 10, 10));

    assert!(world.process_player_autobless_autopulse(CharacterId(1), true, false));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::BLESS_SELF);
    assert_eq!(character.mana, 0);
}

#[test]
fn autobless_is_a_no_op_when_toggle_disabled() {
    let mut world = World::default();
    let mut player = player_character(1);
    player.values[0][CharacterValue::Bless as usize] = 20;
    player.mana = BLESS_COST;
    assert!(world.spawn_character(player, 10, 10));

    assert!(!world.process_player_autobless_autopulse(CharacterId(1), false, false));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, 0);
    assert_eq!(character.mana, BLESS_COST);
}

#[test]
fn autobless_is_a_no_op_without_the_bless_spell() {
    let mut world = World::default();
    let mut player = player_character(1);
    player.mana = BLESS_COST;
    assert!(world.spawn_character(player, 10, 10));

    assert!(!world.process_player_autobless_autopulse(CharacterId(1), true, false));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, 0);
}

#[test]
fn autobless_is_a_no_op_when_mana_is_too_low() {
    let mut world = World::default();
    let mut player = player_character(1);
    player.values[0][CharacterValue::Bless as usize] = 20;
    player.mana = BLESS_COST - 1;
    assert!(world.spawn_character(player, 10, 10));

    assert!(!world.process_player_autobless_autopulse(CharacterId(1), true, false));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, 0);
    assert_eq!(character.mana, BLESS_COST - 1);
}

#[test]
fn autopulse_casts_pulse_when_enabled_and_worthwhile() {
    let mut world = World::default();
    let mut player = player_character(1);
    player.mana = POWERSCALE + 1;
    player.values[0][CharacterValue::Mana as usize] = 1;
    player.values[0][CharacterValue::Pulse as usize] = 2_000;
    let mut target = character(2);
    target.flags.insert(CharacterFlags::ALIVE);
    target.hp = POWERSCALE + 100;
    target.lifeshield = 0;
    target.values[0][CharacterValue::Hp as usize] = 100;
    assert!(world.spawn_character(player, 10, 10));
    assert!(world.spawn_character(target, 12, 10));
    world.map.tile_mut(12, 10).unwrap().light = 255;

    assert!(world.process_player_autobless_autopulse(CharacterId(1), false, true));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::PULSE);
}

#[test]
fn autopulse_is_a_no_op_when_toggle_disabled() {
    let mut world = World::default();
    let mut player = player_character(1);
    player.mana = POWERSCALE + 1;
    player.values[0][CharacterValue::Mana as usize] = 1;
    player.values[0][CharacterValue::Pulse as usize] = 2_000;
    let mut target = character(2);
    target.flags.insert(CharacterFlags::ALIVE);
    target.hp = POWERSCALE + 100;
    target.lifeshield = 0;
    target.values[0][CharacterValue::Hp as usize] = 100;
    assert!(world.spawn_character(player, 10, 10));
    assert!(world.spawn_character(target, 12, 10));
    world.map.tile_mut(12, 10).unwrap().light = 255;

    assert!(!world.process_player_autobless_autopulse(CharacterId(1), false, false));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, 0);
}

#[test]
fn autopulse_is_a_no_op_when_no_target_makes_it_worthwhile() {
    let mut world = World::default();
    let mut player = player_character(1);
    player.mana = POWERSCALE + 1;
    player.values[0][CharacterValue::Mana as usize] = 1;
    player.values[0][CharacterValue::Pulse as usize] = 2_000;
    assert!(world.spawn_character(player, 10, 10));

    assert!(!world.process_player_autobless_autopulse(CharacterId(1), false, true));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, 0);
}

#[test]
fn bless_takes_priority_over_pulse_when_both_are_enabled_and_ready() {
    let mut world = World::default();
    let mut player = player_character(1);
    player.values[0][CharacterValue::Bless as usize] = 20;
    player.mana = POWERSCALE * 10;
    player.values[0][CharacterValue::Mana as usize] = 1;
    player.values[0][CharacterValue::Pulse as usize] = 2_000;
    let mut target = character(2);
    target.flags.insert(CharacterFlags::ALIVE);
    target.hp = POWERSCALE + 100;
    target.lifeshield = 0;
    target.values[0][CharacterValue::Hp as usize] = 100;
    assert!(world.spawn_character(player, 10, 10));
    assert!(world.spawn_character(target, 12, 10));
    world.map.tile_mut(12, 10).unwrap().light = 255;

    assert!(world.process_player_autobless_autopulse(CharacterId(1), true, true));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::BLESS_SELF);
}

#[test]
fn returns_false_for_a_missing_character() {
    let mut world = World::default();
    assert!(!world.process_player_autobless_autopulse(CharacterId(99), true, true));
}
