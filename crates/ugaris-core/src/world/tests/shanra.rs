use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_SHANRA, NT_CHAR, NT_GIVE};
use crate::world::npc::area29::shanra::{ShanraDriverData, ShanraOutcomeEvent, ShanraPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn shanra_npc(id: u32) -> Character {
    let mut shanra = character(id);
    shanra.name = "Shanra the Mage".into();
    shanra.driver = CDR_SHANRA;
    shanra.driver_state = Some(CharacterDriverState::Shanra(ShanraDriverData::default()));
    shanra
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(player_id: CharacterId, shanra_state: i32) -> HashMap<CharacterId, ShanraPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, ShanraPlayerFacts { shanra_state });
    map
}

#[test]
fn state0_greets_and_advances_to_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(shanra_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(shanra) = world.characters.get_mut(&CharacterId(1)) {
        shanra.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_shanra_actions(&facts(CharacterId(2), 0), 1);
    assert!(events.contains(&ShanraOutcomeEvent::UpdateShanraState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Grimoire of Animation")));
}

#[test]
fn state1_teleports_to_the_basement_and_advances_to_2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(shanra_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(shanra) = world.characters.get_mut(&CharacterId(1)) {
        shanra.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_shanra_actions(&facts(CharacterId(2), 1), 1);
    assert!(events.contains(&ShanraOutcomeEvent::UpdateShanraState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("teleport you to the basement")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!((godmode.x, godmode.y), (5, 106));
}

#[test]
fn state2_is_a_silent_no_op_waiting_for_the_sentinels() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(shanra_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(shanra) = world.characters.get_mut(&CharacterId(1)) {
        shanra.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_shanra_actions(&facts(CharacterId(2), 2), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state3_congratulates_and_advances_to_4() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(shanra_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(shanra) = world.characters.get_mut(&CharacterId(1)) {
        shanra.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_shanra_actions(&facts(CharacterId(2), 3), 1);
    assert!(events.contains(&ShanraOutcomeEvent::UpdateShanraState {
        player_id: CharacterId(2),
        new_state: 4,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("learning about animation")));
}

#[test]
fn state4_teleports_back_to_the_ruins_and_advances_to_5() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(shanra_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(shanra) = world.characters.get_mut(&CharacterId(1)) {
        shanra.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_shanra_actions(&facts(CharacterId(2), 4), 1);
    assert!(events.contains(&ShanraOutcomeEvent::UpdateShanraState {
        player_id: CharacterId(2),
        new_state: 5,
    }));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!((godmode.x, godmode.y), (53, 129));
}

#[test]
fn state5_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(shanra_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(shanra) = world.characters.get_mut(&CharacterId(1)) {
        shanra.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_shanra_actions(&facts(CharacterId(2), 5), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn text_repeat_resets_state_to_0_when_not_yet_past_state2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(shanra_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(shanra) = world.characters.get_mut(&CharacterId(1)) {
        shanra.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_shanra_actions(&facts(CharacterId(2), 2), 1);
    assert!(events.contains(&ShanraOutcomeEvent::UpdateShanraState {
        player_id: CharacterId(2),
        new_state: 0,
    }));
}

#[test]
fn text_repeat_resets_state_to_3_when_between_3_and_5() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(shanra_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(shanra) = world.characters.get_mut(&CharacterId(1)) {
        shanra.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_shanra_actions(&facts(CharacterId(2), 5), 1);
    assert!(events.contains(&ShanraOutcomeEvent::UpdateShanraState {
        player_id: CharacterId(2),
        new_state: 3,
    }));
}

#[test]
fn text_reset_me_speaks_reset_done_and_pushes_reset_event_for_gods_only() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(shanra_npc(1), 10, 10));
    let mut god = player(2, "Godmode");
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 12, 10));

    if let Some(shanra) = world.characters.get_mut(&CharacterId(1)) {
        shanra.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_shanra_actions(&facts(CharacterId(2), 3), 1);
    assert!(events.contains(&ShanraOutcomeEvent::ResetShanra {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("reset done")));
}

#[test]
fn text_reset_me_is_ignored_for_non_gods() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(shanra_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(shanra) = world.characters.get_mut(&CharacterId(1)) {
        shanra.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_shanra_actions(&facts(CharacterId(2), 3), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn give_item_is_always_handed_back() {
    let mut world = World::default();
    let mut shanra = shanra_npc(1);
    shanra.cursor_item = Some(ItemId(50));
    world.add_character(shanra);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(shanra) = world.characters.get_mut(&CharacterId(1)) {
        shanra.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_shanra_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
