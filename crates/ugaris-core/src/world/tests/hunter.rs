use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_HUNTER, NT_CHAR, NT_GIVE};
use crate::item_driver::IID_ARKHATA_HARPY;
use crate::world::npc::area37::hunter::{HunterDriverData, HunterOutcomeEvent, HunterPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn hunter_npc(id: u32) -> Character {
    let mut hunter = character(id);
    hunter.name = "Hunter".into();
    hunter.driver = CDR_HUNTER;
    hunter.driver_state = Some(CharacterDriverState::Hunter(HunterDriverData::default()));
    hunter
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    hunter_state: i32,
    pot_state: i32,
) -> HashMap<CharacterId, HunterPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        HunterPlayerFacts {
            hunter_state,
            pot_state,
        },
    );
    map
}

#[test]
fn state0_without_pot_progress_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(hunter_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(hunter) = world.characters.get_mut(&CharacterId(1)) {
        hunter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_hunter_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state0_with_pot_progress_greets_and_collapses_to_2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(hunter_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(hunter) = world.characters.get_mut(&CharacterId(1)) {
        hunter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_hunter_actions(&facts(CharacterId(2), 0, 1), 1);
    assert!(events.contains(&HunterOutcomeEvent::UpdateHunterState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    assert!(!events
        .iter()
        .any(|event| matches!(event, HunterOutcomeEvent::QuestOpen77 { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("odd coincidence")));
}

#[test]
fn state1_reached_only_via_repeat_greets_and_advances_to_2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(hunter_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(hunter) = world.characters.get_mut(&CharacterId(1)) {
        hunter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_hunter_actions(&facts(CharacterId(2), 1, 1), 1);
    assert!(events.contains(&HunterOutcomeEvent::UpdateHunterState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("odd coincidence")));
}

#[test]
fn state4_without_level_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(hunter_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.level = 40;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(hunter) = world.characters.get_mut(&CharacterId(1)) {
        hunter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_hunter_actions(&facts(CharacterId(2), 4, 1), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state4_with_level_opens_quest77_and_collapses_to_6() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(hunter_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.level = 58;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(hunter) = world.characters.get_mut(&CharacterId(1)) {
        hunter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_hunter_actions(&facts(CharacterId(2), 4, 1), 1);
    assert!(events.contains(&HunterOutcomeEvent::QuestOpen77 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&HunterOutcomeEvent::UpdateHunterState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("reached even me")));
}

#[test]
fn state9_is_a_silent_no_op_waiting_for_the_harpy_skin() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(hunter_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(hunter) = world.characters.get_mut(&CharacterId(1)) {
        hunter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_hunter_actions(&facts(CharacterId(2), 9, 1), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state10_is_a_silent_no_op_all_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(hunter_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(hunter) = world.characters.get_mut(&CharacterId(1)) {
        hunter.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_hunter_actions(&facts(CharacterId(2), 10, 1), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn give_harpy_skin_within_range_completes_quest77_pays_gold_and_jumps_to_10() {
    let mut world = World::default();
    let mut hunter = hunter_npc(1);
    hunter.cursor_item = Some(ItemId(50));
    world.add_character(hunter);
    let mut skin = item(50, ItemFlags::empty());
    skin.name = "Skin".into();
    skin.template_id = IID_ARKHATA_HARPY;
    skin.carried_by = Some(CharacterId(1));
    world.add_item(skin);
    let godmode = player(2, "Godmode");
    world.add_character(godmode);

    if let Some(hunter) = world.characters.get_mut(&CharacterId(1)) {
        hunter.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let starting_gold = world.characters.get(&CharacterId(2)).unwrap().gold;
    let events = world.process_hunter_actions(&facts(CharacterId(2), 7, 1), 1);
    assert!(events.contains(&HunterOutcomeEvent::QuestDone77 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&HunterOutcomeEvent::UpdateHunterState {
        player_id: CharacterId(2),
        new_state: 10,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("150 gold")));
    assert!(!world.items.contains_key(&ItemId(50)));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.gold, starting_gold + 150 * 100);
}

#[test]
fn give_harpy_skin_outside_range_is_handed_back_with_dialogue() {
    let mut world = World::default();
    let mut hunter = hunter_npc(1);
    hunter.cursor_item = Some(ItemId(50));
    world.add_character(hunter);
    let mut skin = item(50, ItemFlags::empty());
    skin.template_id = IID_ARKHATA_HARPY;
    skin.carried_by = Some(CharacterId(1));
    world.add_item(skin);
    world.add_character(player(2, "Godmode"));

    if let Some(hunter) = world.characters.get_mut(&CharacterId(1)) {
        hunter.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_hunter_actions(&facts(CharacterId(2), 4, 1), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, HunterOutcomeEvent::QuestDone77 { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn text_repeat_resets_to_1_when_between_states_1_and_4() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(hunter_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(hunter) = world.characters.get_mut(&CharacterId(1)) {
        hunter.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_hunter_actions(&facts(CharacterId(2), 3, 1), 1);
    assert!(events.contains(&HunterOutcomeEvent::UpdateHunterState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
}

#[test]
fn text_repeat_is_a_no_op_at_state0() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(hunter_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(hunter) = world.characters.get_mut(&CharacterId(1)) {
        hunter.push_driver_text_message(CharacterId(2), "restart");
    }
    let events = world.process_hunter_actions(&facts(CharacterId(2), 0, 1), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, HunterOutcomeEvent::UpdateHunterState { .. })));
}

#[test]
fn give_unrelated_item_is_handed_back() {
    let mut world = World::default();
    let mut hunter = hunter_npc(1);
    hunter.cursor_item = Some(ItemId(50));
    world.add_character(hunter);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(hunter) = world.characters.get_mut(&CharacterId(1)) {
        hunter.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_hunter_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
