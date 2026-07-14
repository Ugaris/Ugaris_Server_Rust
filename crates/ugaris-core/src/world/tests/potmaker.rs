use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_POTMAKER, NT_CHAR, NT_GIVE};
use crate::item_driver::IID_ARKHATA_IRONPOT;
use crate::world::npc::area37::potmaker::{
    PotmakerDriverData, PotmakerOutcomeEvent, PotmakerPlayerFacts,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn potmaker_npc(id: u32) -> Character {
    let mut potmaker = character(id);
    potmaker.name = "Potmaker".into();
    potmaker.driver = CDR_POTMAKER;
    potmaker.driver_state = Some(CharacterDriverState::Potmaker(PotmakerDriverData::default()));
    potmaker
}

fn player(id: u32, name: &str, level: u32) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player.level = level;
    player
}

fn facts(player_id: CharacterId, pot_state: i32) -> HashMap<CharacterId, PotmakerPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, PotmakerPlayerFacts { pot_state });
    map
}

#[test]
fn state0_below_level_48_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(potmaker_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 47), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(potmaker) = world.characters.get_mut(&CharacterId(1)) {
        potmaker.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_potmaker_actions(&facts(CharacterId(2), 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state0_at_level_48_greets_opens_quest73_and_collapses_to_2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(potmaker_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 48), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(potmaker) = world.characters.get_mut(&CharacterId(1)) {
        potmaker.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_potmaker_actions(&facts(CharacterId(2), 0), 1);
    assert!(events.contains(&PotmakerOutcomeEvent::QuestOpen73 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&PotmakerOutcomeEvent::UpdatePotState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("a rather special pot I made")));
}

#[test]
fn state2_speaks_and_advances_to_3() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(potmaker_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 48), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(potmaker) = world.characters.get_mut(&CharacterId(1)) {
        potmaker.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_potmaker_actions(&facts(CharacterId(2), 2), 1);
    assert!(events.contains(&PotmakerOutcomeEvent::UpdatePotState {
        player_id: CharacterId(2),
        new_state: 3,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("south of his temple")));
}

#[test]
fn state3_is_a_silent_no_op_waiting_for_the_pot() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(potmaker_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 48), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(potmaker) = world.characters.get_mut(&CharacterId(1)) {
        potmaker.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_potmaker_actions(&facts(CharacterId(2), 3), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state4_is_a_silent_no_op_all_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(potmaker_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 48), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(potmaker) = world.characters.get_mut(&CharacterId(1)) {
        potmaker.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_potmaker_actions(&facts(CharacterId(2), 4), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn give_ironpot_while_turn_in_window_open_completes_quest_and_sets_state4() {
    let mut world = World::default();
    let mut potmaker = potmaker_npc(1);
    potmaker.cursor_item = Some(ItemId(50));
    world.add_character(potmaker);
    let mut pot = item(50, ItemFlags::empty());
    pot.name = "an iron pot".into();
    pot.template_id = IID_ARKHATA_IRONPOT;
    pot.carried_by = Some(CharacterId(1));
    world.add_item(pot);
    world.add_character(player(2, "Godmode", 48));

    if let Some(potmaker) = world.characters.get_mut(&CharacterId(1)) {
        potmaker.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_potmaker_actions(&facts(CharacterId(2), 3), 1);
    assert!(
        events.contains(&PotmakerOutcomeEvent::QuestDone73GiveInfravisionPot {
            player_id: CharacterId(2),
        })
    );
    assert!(events.contains(&PotmakerOutcomeEvent::UpdatePotState {
        player_id: CharacterId(2),
        new_state: 4,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("blessed by all that is good")));
    assert!(!world.items.contains_key(&ItemId(50)));
}

#[test]
fn give_ironpot_outside_turn_in_window_is_handed_back() {
    let mut world = World::default();
    let mut potmaker = potmaker_npc(1);
    potmaker.cursor_item = Some(ItemId(50));
    world.add_character(potmaker);
    let mut pot = item(50, ItemFlags::empty());
    pot.name = "an iron pot".into();
    pot.template_id = IID_ARKHATA_IRONPOT;
    pot.carried_by = Some(CharacterId(1));
    world.add_item(pot);
    world.add_character(player(2, "Godmode", 48));

    if let Some(potmaker) = world.characters.get_mut(&CharacterId(1)) {
        potmaker.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // state 4 ("all done"): outside the `1..=3` turn-in window.
    let events = world.process_potmaker_actions(&facts(CharacterId(2), 4), 1);
    assert!(!events.iter().any(|event| matches!(
        event,
        PotmakerOutcomeEvent::QuestDone73GiveInfravisionPot { .. }
    )));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn give_unrelated_item_is_handed_back() {
    let mut world = World::default();
    let mut potmaker = potmaker_npc(1);
    potmaker.cursor_item = Some(ItemId(50));
    world.add_character(potmaker);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode", 48));

    if let Some(potmaker) = world.characters.get_mut(&CharacterId(1)) {
        potmaker.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_potmaker_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn text_repeat_resets_to_1_when_inside_turn_in_window() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(potmaker_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 48), 12, 10));

    if let Some(potmaker) = world.characters.get_mut(&CharacterId(1)) {
        potmaker.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_potmaker_actions(&facts(CharacterId(2), 3), 1);
    assert!(events.contains(&PotmakerOutcomeEvent::UpdatePotState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
}

#[test]
fn text_repeat_is_a_no_op_outside_turn_in_window() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(potmaker_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode", 48), 12, 10));

    if let Some(potmaker) = world.characters.get_mut(&CharacterId(1)) {
        potmaker.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_potmaker_actions(&facts(CharacterId(2), 0), 1);
    assert!(events.is_empty());
}
