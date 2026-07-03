//! Tests for `CL_SPEED` (`World::set_speed_mode`, C `cl_speed`).

use super::*;

#[test]
fn set_speed_mode_normal_always_succeeds() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.speed_mode = SpeedMode::Fast;
    npc.endurance = 0;
    assert!(world.spawn_character(npc, 10, 10));

    assert!(world.set_speed_mode(CharacterId(1), 0));
    assert_eq!(
        world.characters[&CharacterId(1)].speed_mode,
        SpeedMode::Normal
    );
}

#[test]
fn set_speed_mode_stealth_always_succeeds() {
    let mut world = World::default();
    let npc = character(1);
    assert!(world.spawn_character(npc, 10, 10));

    assert!(world.set_speed_mode(CharacterId(1), 2));
    assert_eq!(
        world.characters[&CharacterId(1)].speed_mode,
        SpeedMode::Stealth
    );
}

#[test]
fn set_speed_mode_fast_requires_powerscale_endurance() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.endurance = POWERSCALE - 1;
    assert!(world.spawn_character(npc, 10, 10));

    // C: `mode == SM_FAST && ch[cn].endurance < POWERSCALE` -> ignored.
    assert!(!world.set_speed_mode(CharacterId(1), 1));
    assert_eq!(
        world.characters[&CharacterId(1)].speed_mode,
        SpeedMode::Normal
    );
}

#[test]
fn set_speed_mode_fast_succeeds_at_exactly_powerscale_endurance() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.endurance = POWERSCALE;
    assert!(world.spawn_character(npc, 10, 10));

    assert!(world.set_speed_mode(CharacterId(1), 1));
    assert_eq!(
        world.characters[&CharacterId(1)].speed_mode,
        SpeedMode::Fast
    );
}

#[test]
fn set_speed_mode_rejects_invalid_mode_byte() {
    let mut world = World::default();
    let npc = character(1);
    assert!(world.spawn_character(npc, 10, 10));

    assert!(!world.set_speed_mode(CharacterId(1), 3));
    assert_eq!(
        world.characters[&CharacterId(1)].speed_mode,
        SpeedMode::Normal
    );
}

#[test]
fn set_speed_mode_ignores_unknown_character() {
    let mut world = World::default();
    assert!(!world.set_speed_mode(CharacterId(99), 0));
}
