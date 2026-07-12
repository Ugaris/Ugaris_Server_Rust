use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_CALIGARGUARD2, NT_CHAR};
use crate::world::npc::area36::caligar_guard2::{
    CaligarGuard2OutcomeEvent, CaligarGuard2PlayerFacts,
};

fn guard2_npc(id: u32) -> Character {
    let mut guard = character(id);
    guard.name = "Caligar Guard".into();
    guard.driver = CDR_CALIGARGUARD2;
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
    guard2_last_talk: i32,
) -> HashMap<CharacterId, CaligarGuard2PlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, CaligarGuard2PlayerFacts { guard2_last_talk });
    map
}

#[test]
fn taunts_and_updates_last_talk_after_cooldown() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guard2_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_caligar_guard2_actions(&facts(CharacterId(2), 0), 100);
    assert!(
        events.contains(&CaligarGuard2OutcomeEvent::UpdateGuard2LastTalk {
            player_id: CharacterId(2),
            realtime_seconds: 100,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Halt! You will die where you stand!")));
}

#[test]
fn talk_cooldown_of_15_seconds_blocks_repeated_taunts() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guard2_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_caligar_guard2_actions(&facts(CharacterId(2), 90), 100);
    assert!(events.is_empty());
}

#[test]
fn distance_over_10_is_ignored() {
    let mut world = World::default();
    world.map.tile_mut(30, 10).unwrap().light = 255;
    assert!(world.spawn_character(guard2_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 30, 10));

    if let Some(guard) = world.characters.get_mut(&CharacterId(1)) {
        guard.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_caligar_guard2_actions(&facts(CharacterId(2), 0), 100);
    assert!(events.is_empty());
}
