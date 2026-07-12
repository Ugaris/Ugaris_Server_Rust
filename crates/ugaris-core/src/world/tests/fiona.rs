use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_FIONA, NTID_GLADIATOR, NT_CHAR, NT_GIVE, NT_NPC};
use crate::item_driver::IID_ARKHATA_RING;
use crate::world::npc::area37::fiona::{FionaDriverData, FionaOutcomeEvent, FionaPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn fiona_npc(id: u32) -> Character {
    let mut fiona = character(id);
    fiona.name = "Queen Fiona".into();
    fiona.driver = CDR_FIONA;
    fiona.driver_state = Some(CharacterDriverState::Fiona(FionaDriverData::default()));
    fiona
}

fn player(id: u32, name: &str, level: u32) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player.level = level;
    player
}

fn facts(player_id: CharacterId, fiona_state: i32) -> HashMap<CharacterId, FionaPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, FionaPlayerFacts { fiona_state });
    map
}

#[test]
fn state0_below_level_50_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(fiona_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 10), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(fiona) = world.characters.get_mut(&CharacterId(1)) {
        fiona.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_fiona_actions(&facts(CharacterId(2), 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state0_at_level_50_greets_opens_quest67_and_collapses_to_2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(fiona_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 50), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(fiona) = world.characters.get_mut(&CharacterId(1)) {
        fiona.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_fiona_actions(&facts(CharacterId(2), 0), 1);
    assert!(events.contains(&FionaOutcomeEvent::QuestOpen67 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&FionaOutcomeEvent::UpdateFionaState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Academy of the Fighting Arts")));
}

#[test]
fn state5_above_level_80_skips_straight_to_19() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(fiona_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 90), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(fiona) = world.characters.get_mut(&CharacterId(1)) {
        fiona.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_fiona_actions(&facts(CharacterId(2), 5), 1);
    assert!(events.contains(&FionaOutcomeEvent::UpdateFionaState {
        player_id: CharacterId(2),
        new_state: 19,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("no challenge")));
}

#[test]
fn state5_at_or_below_level_80_offers_the_challenge() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(fiona_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 80), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(fiona) = world.characters.get_mut(&CharacterId(1)) {
        fiona.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_fiona_actions(&facts(CharacterId(2), 5), 1);
    assert!(events.contains(&FionaOutcomeEvent::UpdateFionaState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
}

#[test]
fn text_enter_at_fighting_state_dispatches_fight_student() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(fiona_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 60), 12, 10));

    if let Some(fiona) = world.characters.get_mut(&CharacterId(1)) {
        fiona.push_driver_text_message(CharacterId(2), "enter");
    }
    // `fiona_state` 9 -> nr = 9 - 6 = 3.
    let events = world.process_fiona_actions(&facts(CharacterId(2), 9), 1);
    assert!(events.contains(&FionaOutcomeEvent::FightStudent {
        fiona_id: CharacterId(1),
        player_id: CharacterId(2),
        nr: 3,
    }));
}

#[test]
fn text_enter_outside_fighting_range_is_a_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(fiona_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 60), 12, 10));

    if let Some(fiona) = world.characters.get_mut(&CharacterId(1)) {
        fiona.push_driver_text_message(CharacterId(2), "enter");
    }
    let events = world.process_fiona_actions(&facts(CharacterId(2), 20), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, FionaOutcomeEvent::FightStudent { .. })));
}

#[test]
fn text_raise_at_state18_with_enough_gold_raises_and_advances_to_19() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(fiona_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode", 60);
    godmode.gold = 10000 * 100;
    godmode.values[1][CharacterValue::Attack as usize] = 10;
    godmode.values[0][CharacterValue::Attack as usize] = 10;
    assert!(world.spawn_character(godmode, 12, 10));

    if let Some(fiona) = world.characters.get_mut(&CharacterId(1)) {
        fiona.push_driver_text_message(CharacterId(2), "raise attack");
    }
    let events = world.process_fiona_actions(&facts(CharacterId(2), 18), 1);
    assert!(events.contains(&FionaOutcomeEvent::UpdateFionaState {
        player_id: CharacterId(2),
        new_state: 19,
    }));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert!(godmode.gold < 10000 * 100);
}

#[test]
fn text_raise_outside_state18_is_rejected() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(fiona_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode", 60);
    godmode.gold = 10000 * 100;
    assert!(world.spawn_character(godmode, 12, 10));

    if let Some(fiona) = world.characters.get_mut(&CharacterId(1)) {
        fiona.push_driver_text_message(CharacterId(2), "raise attack");
    }
    let events = world.process_fiona_actions(&facts(CharacterId(2), 17), 1);
    assert!(!events.iter().any(|event| matches!(
        event,
        FionaOutcomeEvent::UpdateFionaState { new_state: 19, .. }
    )));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("not open at the moment")));
}

#[test]
fn give_ring_at_state3_completes_quest67_and_jumps_to_4() {
    let mut world = World::default();
    let mut fiona = fiona_npc(1);
    fiona.cursor_item = Some(ItemId(50));
    world.add_character(fiona);
    let mut ring = item(50, ItemFlags::empty());
    ring.name = "Fiona's Ring".into();
    ring.template_id = IID_ARKHATA_RING;
    ring.carried_by = Some(CharacterId(1));
    world.add_item(ring);
    world.add_character(player(2, "Godmode", 60));

    if let Some(fiona) = world.characters.get_mut(&CharacterId(1)) {
        fiona.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_fiona_actions(&facts(CharacterId(2), 3), 1);
    assert!(events.contains(&FionaOutcomeEvent::QuestDone67 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&FionaOutcomeEvent::UpdateFionaState {
        player_id: CharacterId(2),
        new_state: 4,
    }));
    assert!(world.items.get(&ItemId(50)).is_none());
}

#[test]
fn give_unrelated_item_is_handed_back() {
    let mut world = World::default();
    let mut fiona = fiona_npc(1);
    fiona.cursor_item = Some(ItemId(50));
    world.add_character(fiona);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode", 60));

    if let Some(fiona) = world.characters.get_mut(&CharacterId(1)) {
        fiona.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_fiona_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn gladiator_win_notification_advances_state_and_teleports_killer() {
    let mut world = World::default();
    assert!(world.spawn_character(fiona_npc(1), 15, 232));
    assert!(world.spawn_character(player(2, "Godmode", 60), 20, 240));

    if let Some(fiona) = world.characters.get_mut(&CharacterId(1)) {
        fiona.push_driver_message(NT_NPC, NTID_GLADIATOR, 99, 2);
    }
    let events = world.process_fiona_actions(&facts(CharacterId(2), 9), 1);
    assert!(events.contains(&FionaOutcomeEvent::UpdateFionaState {
        player_id: CharacterId(2),
        new_state: 10,
    }));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!((godmode.x, godmode.y), (15, 235));
}

#[test]
fn gladiator_win_notification_outside_fighting_range_is_a_no_op() {
    let mut world = World::default();
    assert!(world.spawn_character(fiona_npc(1), 15, 232));
    assert!(world.spawn_character(player(2, "Godmode", 60), 20, 240));

    if let Some(fiona) = world.characters.get_mut(&CharacterId(1)) {
        fiona.push_driver_message(NT_NPC, NTID_GLADIATOR, 99, 2);
    }
    let events = world.process_fiona_actions(&facts(CharacterId(2), 20), 1);
    assert!(events.is_empty());
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!((godmode.x, godmode.y), (20, 240));
}

#[test]
fn arena_is_busy_detects_an_occupied_tile_inside_the_bounds() {
    let mut world = World::default();
    assert!(world.spawn_character(player(1, "Godmode", 60), 15, 245));
    assert!(world.arkhata_arena_is_busy());
}

#[test]
fn arena_is_not_busy_when_empty() {
    let world = World::default();
    assert!(!world.arkhata_arena_is_busy());
}
