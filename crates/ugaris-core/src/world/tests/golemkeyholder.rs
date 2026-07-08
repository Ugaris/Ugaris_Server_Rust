use super::*;
use crate::character_driver::{GolemKeyholdDriverData, CDR_GOLEMKEYHOLDER, NT_CREATE};

const SELF_DESTRUCT: u64 = TICKS_PER_SECOND * 60 * 5;

fn golem_npc(id: u32) -> Character {
    let mut golem = character(id);
    golem.name = "Gold Golem".into();
    golem.driver = CDR_GOLEMKEYHOLDER;
    golem.driver_state = Some(CharacterDriverState::GolemKeyhold(
        GolemKeyholdDriverData::default(),
    ));
    golem
}

fn golem_state(world: &World, id: CharacterId) -> GolemKeyholdDriverData {
    match world
        .characters
        .get(&id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::GolemKeyhold(data)) => data,
        _ => panic!("expected golem-keyhold driver state"),
    }
}

#[test]
fn golemkeyhold_sets_creation_time_from_nt_create() {
    let mut world = World::default();
    assert!(world.spawn_character(golem_npc(1), 10, 10));
    world.tick = Tick(42);
    if let Some(golem) = world.characters.get_mut(&CharacterId(1)) {
        golem.push_driver_message(NT_CREATE, 0, 0, 0);
    }

    world.process_golemkeyhold_actions(12);

    assert_eq!(golem_state(&world, CharacterId(1)).creation_time, 42);
}

#[test]
fn golemkeyhold_self_destructs_after_five_minutes() {
    let mut world = World::default();
    let golem = golem_npc(1);
    assert!(world.spawn_character(golem, 10, 10));
    world.tick = Tick(SELF_DESTRUCT + 1);

    let acted = world.process_golemkeyhold_actions(12);

    assert_eq!(acted, 1);
    assert!(world.characters.get(&CharacterId(1)).is_none());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Thats all folks!")));
}

#[test]
fn golemkeyhold_does_not_self_destruct_before_five_minutes() {
    let mut world = World::default();
    let golem = golem_npc(1);
    assert!(world.spawn_character(golem, 10, 10));
    world.tick = Tick(SELF_DESTRUCT - 1);

    world.process_golemkeyhold_actions(12);

    assert!(world.characters.get(&CharacterId(1)).is_some());
}

#[test]
fn golemkeyhold_attacks_adjacent_victim() {
    let mut world = World::default();
    let mut golem = golem_npc(1);
    golem.driver_state = Some(CharacterDriverState::GolemKeyhold(GolemKeyholdDriverData {
        victim: Some(CharacterId(2)),
        ..GolemKeyholdDriverData::default()
    }));
    assert!(world.spawn_character(golem, 10, 10));
    assert!(world.spawn_character(character(2), 11, 10));

    let acted = world.process_golemkeyhold_actions(12);

    assert_eq!(acted, 1);
    let golem = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(golem.action, action::ATTACK1);
    assert!(golem_state(&world, CharacterId(1)).victim_visible);
}

#[test]
fn golemkeyhold_walks_toward_visible_but_distant_victim() {
    let mut world = World::default();
    let mut golem = golem_npc(1);
    golem.driver_state = Some(CharacterDriverState::GolemKeyhold(GolemKeyholdDriverData {
        victim: Some(CharacterId(2)),
        ..GolemKeyholdDriverData::default()
    }));
    assert!(world.spawn_character(golem, 10, 10));
    assert!(world.spawn_character(character(2), 13, 10));
    world.map.tile_mut(13, 10).unwrap().light = 255;

    let acted = world.process_golemkeyhold_actions(12);

    assert_eq!(acted, 1);
    let golem = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(golem.action, action::WALK);
    assert_eq!((golem.tox, golem.toy), (11, 10));
}

#[test]
fn golemkeyhold_returns_to_post_when_no_victim() {
    let mut world = World::default();
    let mut golem = golem_npc(1);
    golem.rest_x = 15;
    golem.rest_y = 10;
    world.map.tile_mut(15, 10).unwrap().light = 255;
    assert!(world.spawn_character(golem, 10, 10));

    let acted = world.process_golemkeyhold_actions(12);

    assert_eq!(acted, 1);
    let golem = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(golem.action, action::WALK);
    assert_eq!((golem.tox, golem.toy), (11, 10));
}

#[test]
fn golemkeyhold_gives_up_chasing_invisible_victim_once_arrived() {
    let mut world = World::default();
    let mut golem = golem_npc(1);
    golem.driver_state = Some(CharacterDriverState::GolemKeyhold(GolemKeyholdDriverData {
        victim: Some(CharacterId(2)),
        victim_last_x: 10,
        victim_last_y: 10,
        victim_visible: false,
        ..GolemKeyholdDriverData::default()
    }));
    assert!(world.spawn_character(golem, 10, 10));
    // victim character no longer exists (dead/removed) - `process_
    // golemkeyhold_tick` treats this the same as C's stale/deleted
    // enemy-slot trash.

    world.process_golemkeyhold_actions(12);

    assert_eq!(golem_state(&world, CharacterId(1)).victim, None);
}
