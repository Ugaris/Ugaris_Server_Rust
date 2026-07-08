use super::*;
use crate::character_driver::{Astro1DriverData, CDR_ASTRO1};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn astro1_npc(id: u32) -> Character {
    let mut astro1 = character(id);
    astro1.name = "Astro1".into();
    astro1.driver = CDR_ASTRO1;
    astro1
}

fn astro1_state(world: &World, id: CharacterId) -> Astro1DriverData {
    match world
        .characters
        .get(&id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Astro1(data)) => data,
        _ => panic!("expected astro1 driver state"),
    }
}

#[test]
fn astro1_says_the_first_monologue_line_immediately() {
    let mut world = World::default();
    let mut astro1 = astro1_npc(1);
    astro1.rest_x = 10;
    astro1.rest_y = 10;
    assert!(world.spawn_character(astro1, 10, 10));
    world.tick = Tick(BASELINE_TICK);

    let acted = world.process_astro1_actions(1);

    assert_eq!(acted, 1);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("The moon, oh so bright and splendid it seemed.")));
    assert_eq!(astro1_state(&world, CharacterId(1)).state, 1);
}

#[test]
fn astro1_does_not_repeat_before_ten_seconds_pass() {
    let mut world = World::default();
    let mut astro1 = astro1_npc(1);
    astro1.rest_x = 10;
    astro1.rest_y = 10;
    assert!(world.spawn_character(astro1, 10, 10));
    world.tick = Tick(BASELINE_TICK);

    world.process_astro1_actions(1);
    world.drain_pending_area_texts();

    world.tick = Tick(world.tick.0 + 1);
    world.process_astro1_actions(1);

    assert!(world.drain_pending_area_texts().is_empty());
    assert_eq!(astro1_state(&world, CharacterId(1)).state, 1);
}

#[test]
fn astro1_advances_through_all_fifteen_states_and_wraps() {
    let mut world = World::default();
    let mut astro1 = astro1_npc(1);
    astro1.rest_x = 10;
    astro1.rest_y = 10;
    assert!(world.spawn_character(astro1, 10, 10));
    world.tick = Tick(BASELINE_TICK);

    for expected_state in 1..=15 {
        world.tick = Tick(world.tick.0 + TICKS_PER_SECOND * 10 + 1);
        world.process_astro1_actions(1);
        let expected = if expected_state == 15 {
            0
        } else {
            expected_state
        };
        assert_eq!(astro1_state(&world, CharacterId(1)).state, expected);
    }
}

#[test]
fn astro1_drains_incoming_messages_without_reacting() {
    let mut world = World::default();
    let mut astro1 = astro1_npc(1);
    astro1.rest_x = 10;
    astro1.rest_y = 10;
    assert!(world.spawn_character(astro1, 10, 10));
    assert!(world.spawn_character(character(2), 11, 10));
    if let Some(astro1) = world.characters.get_mut(&CharacterId(1)) {
        astro1.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_astro1_actions(1);

    assert!(world.characters[&CharacterId(1)].driver_messages.is_empty());
}

#[test]
fn astro1_returns_to_post_when_displaced() {
    let mut world = World::default();
    let mut astro1 = astro1_npc(1);
    astro1.rest_x = 10;
    astro1.rest_y = 10;
    assert!(world.spawn_character(astro1, 12, 10));

    let acted = world.process_astro1_actions(1);

    assert_eq!(acted, 1);
    let astro1 = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(astro1.action, action::WALK);
}
