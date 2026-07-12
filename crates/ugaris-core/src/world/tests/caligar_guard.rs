use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_CALIGARGUARD, NT_CHAR};
use crate::world::npc::area36::caligar_guard::{CaligarGuardOutcomeEvent, CaligarGuardPlayerFacts};

fn guard_npc(id: u32, name: &str) -> Character {
    let mut guard = character(id);
    guard.name = name.into();
    guard.driver = CDR_CALIGARGUARD;
    guard
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    guard_state: i32,
    guard_last_talk: i32,
) -> HashMap<CharacterId, CaligarGuardPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        CaligarGuardPlayerFacts {
            guard_state,
            guard_last_talk,
        },
    );
    map
}

#[test]
fn state0_eulc_greets_and_advances_to_state1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guard_npc(1, "Eulc"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events =
        world.process_caligar_guard_actions(&facts(CharacterId(2), 0, 0), 100, world.area_id);
    assert!(
        events.contains(&CaligarGuardOutcomeEvent::AdvanceGuardTalk {
            player_id: CharacterId(2),
            new_state: 1,
            realtime_seconds: 100,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Human entry is not permitted")));
}

#[test]
fn state0_margana_stays_silent_until_her_turn() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guard_npc(1, "Margana"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events =
        world.process_caligar_guard_actions(&facts(CharacterId(2), 0, 0), 100, world.area_id);
    assert!(events.is_empty());
}

#[test]
fn talk_cooldown_of_3_seconds_blocks_repeated_lines() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guard_npc(1, "Eulc"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events =
        world.process_caligar_guard_actions(&facts(CharacterId(2), 0, 99), 100, world.area_id);
    assert!(events.is_empty());
}

#[test]
fn state5_resets_after_600_second_timeout_regardless_of_which_guard() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guard_npc(1, "Margana"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events =
        world.process_caligar_guard_actions(&facts(CharacterId(2), 5, 0), 601, world.area_id);
    assert!(
        events.contains(&CaligarGuardOutcomeEvent::ResetGuardStateTimeout {
            player_id: CharacterId(2),
        })
    );
}

#[test]
fn fence_check_ignores_players_on_the_far_side() {
    let mut world = World::default();
    world.map.tile_mut(10, 107).unwrap().light = 255;
    assert!(world.spawn_character(guard_npc(1, "Eulc"), 10, 100));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 107));

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events =
        world.process_caligar_guard_actions(&facts(CharacterId(2), 0, 0), 100, world.area_id);
    assert!(events.is_empty());
}

#[test]
fn text_repeat_queues_reset_if_state_three_event() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guard_npc(1, "Eulc"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events =
        world.process_caligar_guard_actions(&facts(CharacterId(2), 3, 0), 100, world.area_id);
    assert!(
        events.contains(&CaligarGuardOutcomeEvent::ResetGuardStateIfThree {
            player_id: CharacterId(2),
        })
    );
}

#[test]
fn text_hello_gets_a_canned_reply_and_no_state_event() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guard_npc(1, "Eulc"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_text_message(CharacterId(2), "hello");
    }

    let events =
        world.process_caligar_guard_actions(&facts(CharacterId(2), 0, 0), 100, world.area_id);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hello, Godmode!")));
}
