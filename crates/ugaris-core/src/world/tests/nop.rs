use super::*;
use crate::character_driver::CDR_NOP;
use crate::direction::Direction;
use crate::world::npc::area37::nop::{parse_nop_driver_args, NopDriverData};

fn nop_npc(id: u32, facing_direction: u8) -> Character {
    let mut nop = character(id);
    nop.name = "Student".into();
    nop.driver = CDR_NOP;
    nop.driver_state = Some(CharacterDriverState::Nop(NopDriverData {
        facing_direction,
    }));
    nop.rest_x = nop.x;
    nop.rest_y = nop.y;
    nop
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

#[test]
fn parse_nop_driver_args_reads_the_dir_key() {
    let data = parse_nop_driver_args("dir=3;");
    assert_eq!(data.facing_direction, Direction::Down as u8);
}

#[test]
fn parse_nop_driver_args_ignores_unknown_keys() {
    // C `nop_driver_parse`'s `else { elog(...); }` branch (`arkhata.c:
    // 1292-1294`) is log-only - unknown keys leave the default (0).
    let data = parse_nop_driver_args("foo=9;dir=7;");
    assert_eq!(data.facing_direction, Direction::Up as u8);
}

#[test]
fn nop_answers_a_greeting_qa_row() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(nop_npc(1, Direction::Down as u8), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    if let Some(nop) = world.characters.get_mut(&CharacterId(1)) {
        nop.push_driver_text_message(CharacterId(2), "hello");
    }

    world.process_nop_actions(37);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hello, Godmode!")));
}

#[test]
fn nop_ignores_a_skill_raise_request_since_it_discards_the_return_code() {
    // C `nop_driver` never assigns `analyse_text_driver`'s return value
    // (`arkhata.c:1319`), unlike every other driver in this file - only
    // qa rows with a canned `answer` are ever visible here.
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(nop_npc(1, Direction::Down as u8), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    if let Some(nop) = world.characters.get_mut(&CharacterId(1)) {
        nop.push_driver_text_message(CharacterId(2), "raise attack");
    }

    world.process_nop_actions(37);

    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn nop_ignores_text_from_a_speaker_too_far_away() {
    let mut world = World::default();
    world.map.tile_mut(30, 10).unwrap().light = 255;
    assert!(world.spawn_character(nop_npc(1, Direction::Down as u8), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 30, 10));

    if let Some(nop) = world.characters.get_mut(&CharacterId(1)) {
        nop.push_driver_text_message(CharacterId(2), "hello");
    }

    world.process_nop_actions(37);

    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn nop_walks_back_to_its_post_when_displaced() {
    let mut world = World::default();
    let mut nop = nop_npc(1, Direction::Down as u8);
    nop.rest_x = 10;
    nop.rest_y = 10;
    assert!(world.spawn_character(nop, 12, 10));

    world.process_nop_actions(37);

    let nop = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(nop.action, action::WALK);
}
