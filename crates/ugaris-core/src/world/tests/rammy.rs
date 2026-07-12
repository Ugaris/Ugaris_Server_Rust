use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_RAMMY, NT_CHAR, NT_GIVE};
use crate::item_driver::IID_ARKHATA_CROWN;
use crate::world::npc::area37::rammy::{RammyDriverData, RammyOutcomeEvent, RammyPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn rammy_npc(id: u32) -> Character {
    let mut rammy = character(id);
    rammy.name = "Rammy".into();
    rammy.driver = CDR_RAMMY;
    rammy.driver_state = Some(CharacterDriverState::Rammy(RammyDriverData::default()));
    rammy
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    rammy_state: i32,
    guardbran_state: i32,
    monk_state: i32,
    letter_bits: i32,
) -> HashMap<CharacterId, RammyPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        RammyPlayerFacts {
            rammy_state,
            guardbran_state,
            monk_state,
            letter_bits,
        },
    );
    map
}

#[test]
fn state0_without_guardbran_progress_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rammy_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(rammy) = world.characters.get_mut(&CharacterId(1)) {
        rammy.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_rammy_actions(&facts(CharacterId(2), 0, 1, 0, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state0_with_guardbran_progress_greets_and_advances_to_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rammy_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(rammy) = world.characters.get_mut(&CharacterId(1)) {
        rammy.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_rammy_actions(&facts(CharacterId(2), 0, 2, 0, 0), 1);
    assert!(events.contains(&RammyOutcomeEvent::UpdateRammyState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hold! Stranger")));
}

#[test]
fn state6_without_guardbran_progress_stalls() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rammy_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(rammy) = world.characters.get_mut(&CharacterId(1)) {
        rammy.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_rammy_actions(&facts(CharacterId(2), 6, 6, 0, 0), 1);
    assert!(events.is_empty());
}

#[test]
fn state6_with_guardbran_progress_opens_quest65_and_collapses_to_8() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rammy_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(rammy) = world.characters.get_mut(&CharacterId(1)) {
        rammy.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_rammy_actions(&facts(CharacterId(2), 6, 7, 0, 0), 1);
    assert!(events.contains(&RammyOutcomeEvent::QuestOpen65 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&RammyOutcomeEvent::UpdateRammyState {
        player_id: CharacterId(2),
        new_state: 8,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Welcome back, friend!")));
}

#[test]
fn state10_is_a_silent_no_op_waiting_for_the_crown() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rammy_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(rammy) = world.characters.get_mut(&CharacterId(1)) {
        rammy.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_rammy_actions(&facts(CharacterId(2), 10, 7, 0, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn give_crown_at_state10_completes_quest65_silently_and_jumps_to_11() {
    let mut world = World::default();
    let mut rammy = rammy_npc(1);
    rammy.cursor_item = Some(ItemId(50));
    world.add_character(rammy);
    let mut crown = item(50, ItemFlags::empty());
    crown.name = "Rammy's Crown".into();
    crown.template_id = IID_ARKHATA_CROWN;
    crown.carried_by = Some(CharacterId(1));
    world.add_item(crown);
    let godmode = player(2, "Godmode");
    world.add_character(godmode);

    if let Some(rammy) = world.characters.get_mut(&CharacterId(1)) {
        rammy.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_rammy_actions(&facts(CharacterId(2), 10, 7, 0, 0), 1);
    assert!(events.contains(&RammyOutcomeEvent::QuestDone65 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&RammyOutcomeEvent::UpdateRammyState {
        player_id: CharacterId(2),
        new_state: 11,
    }));
    // C's crown turn-in is silent - no dialogue at all.
    assert!(world.drain_pending_area_texts().is_empty());
    assert!(world.items.get(&ItemId(50)).is_none());
}

#[test]
fn give_crown_outside_state10_is_handed_back_with_dialogue() {
    let mut world = World::default();
    let mut rammy = rammy_npc(1);
    rammy.cursor_item = Some(ItemId(50));
    world.add_character(rammy);
    let mut crown = item(50, ItemFlags::empty());
    crown.template_id = IID_ARKHATA_CROWN;
    crown.carried_by = Some(CharacterId(1));
    world.add_item(crown);
    world.add_character(player(2, "Godmode"));

    if let Some(rammy) = world.characters.get_mut(&CharacterId(1)) {
        rammy.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_rammy_actions(&facts(CharacterId(2), 9, 7, 0, 0), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, RammyOutcomeEvent::QuestDone65 { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn state13_without_level_or_monk_gate_stalls() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rammy_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.level = 40;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(rammy) = world.characters.get_mut(&CharacterId(1)) {
        rammy.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_rammy_actions(&facts(CharacterId(2), 13, 7, 10, 0), 1);
    assert!(events.is_empty());
}

#[test]
fn state13_with_level_and_monk_gate_opens_quest71_and_collapses_to_15() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rammy_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.level = 54;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(rammy) = world.characters.get_mut(&CharacterId(1)) {
        rammy.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_rammy_actions(&facts(CharacterId(2), 13, 7, 20, 0), 1);
    assert!(events.contains(&RammyOutcomeEvent::QuestOpen71 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&RammyOutcomeEvent::UpdateRammyState {
        player_id: CharacterId(2),
        new_state: 15,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hello again, Godmode!")));
}

#[test]
fn state14_reached_via_text_reset_speaks_the_same_line_as_the_state13_collapse() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rammy_npc(1), 10, 10));
    let godmode = player(2, "Godmode");
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(rammy) = world.characters.get_mut(&CharacterId(1)) {
        rammy.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_rammy_actions(&facts(CharacterId(2), 14, 7, 20, 0), 1);
    assert!(events.contains(&RammyOutcomeEvent::UpdateRammyState {
        player_id: CharacterId(2),
        new_state: 15,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hello again, Godmode!")));
}

#[test]
fn state16_hands_out_fortress_key_and_letter_when_missing_both() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rammy_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(rammy) = world.characters.get_mut(&CharacterId(1)) {
        rammy.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_rammy_actions(&facts(CharacterId(2), 16, 7, 20, 0), 1);
    assert!(
        events.contains(&RammyOutcomeEvent::GiveFortressKeyAndLetter {
            player_id: CharacterId(2),
            give_key: true,
            give_letter: true,
        })
    );
    assert!(events.contains(&RammyOutcomeEvent::UpdateRammyState {
        player_id: CharacterId(2),
        new_state: 17,
    }));
}

#[test]
fn state17_without_all_letter_bits_stalls() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rammy_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(rammy) = world.characters.get_mut(&CharacterId(1)) {
        rammy.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_rammy_actions(&facts(CharacterId(2), 17, 7, 20, 2 | 4), 1);
    assert!(events.is_empty());
}

#[test]
fn state17_with_all_letter_bits_completes_quest71_and_collapses_to_19() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rammy_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(rammy) = world.characters.get_mut(&CharacterId(1)) {
        rammy.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_rammy_actions(&facts(CharacterId(2), 17, 7, 20, 2 | 4 | 8), 1);
    assert!(events.contains(&RammyOutcomeEvent::QuestDone71 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&RammyOutcomeEvent::UpdateRammyState {
        player_id: CharacterId(2),
        new_state: 19,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("trade route is open")));
}

#[test]
fn text_repeat_resets_to_0_when_at_or_below_state6() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rammy_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(rammy) = world.characters.get_mut(&CharacterId(1)) {
        rammy.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_rammy_actions(&facts(CharacterId(2), 4, 7, 0, 0), 1);
    assert!(events.contains(&RammyOutcomeEvent::UpdateRammyState {
        player_id: CharacterId(2),
        new_state: 0,
    }));
}

#[test]
fn text_repeat_resets_to_7_when_between_states_7_and_10() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rammy_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(rammy) = world.characters.get_mut(&CharacterId(1)) {
        rammy.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_rammy_actions(&facts(CharacterId(2), 9, 7, 0, 0), 1);
    assert!(events.contains(&RammyOutcomeEvent::UpdateRammyState {
        player_id: CharacterId(2),
        new_state: 7,
    }));
}

#[test]
fn text_repeat_resets_to_14_when_between_states_14_and_17() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rammy_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(rammy) = world.characters.get_mut(&CharacterId(1)) {
        rammy.push_driver_text_message(CharacterId(2), "restart");
    }
    let events = world.process_rammy_actions(&facts(CharacterId(2), 16, 7, 20, 0), 1);
    assert!(events.contains(&RammyOutcomeEvent::UpdateRammyState {
        player_id: CharacterId(2),
        new_state: 14,
    }));
}

#[test]
fn text_repeat_at_state11_does_not_reset_anything() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rammy_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(rammy) = world.characters.get_mut(&CharacterId(1)) {
        rammy.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_rammy_actions(&facts(CharacterId(2), 11, 7, 0, 0), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, RammyOutcomeEvent::UpdateRammyState { .. })));
}

#[test]
fn give_unrelated_item_is_handed_back() {
    let mut world = World::default();
    let mut rammy = rammy_npc(1);
    rammy.cursor_item = Some(ItemId(50));
    world.add_character(rammy);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(rammy) = world.characters.get_mut(&CharacterId(1)) {
        rammy.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_rammy_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
