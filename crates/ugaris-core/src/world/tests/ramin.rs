use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_RAMIN, NT_CHAR, NT_GIVE};
use crate::item_driver::IID_ARKHATA_LETTER2;
use crate::world::npc::area37::ramin::{RaminDriverData, RaminOutcomeEvent, RaminPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn ramin_npc(id: u32) -> Character {
    let mut ramin = character(id);
    ramin.name = "Ramin".into();
    ramin.driver = CDR_RAMIN;
    ramin.driver_state = Some(CharacterDriverState::Ramin(RaminDriverData::default()));
    ramin
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

#[allow(clippy::too_many_arguments)]
fn facts(
    player_id: CharacterId,
    ramin_state: i32,
    fiona_state: i32,
    monk_state: i32,
    rammy_state: i32,
    letter_bits: i32,
) -> HashMap<CharacterId, RaminPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        RaminPlayerFacts {
            ramin_state,
            fiona_state,
            monk_state,
            rammy_state,
            letter_bits,
        },
    );
    map
}

#[test]
fn state0_without_fiona_progress_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(ramin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(ramin) = world.characters.get_mut(&CharacterId(1)) {
        ramin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_ramin_actions(&facts(CharacterId(2), 0, 3, 0, 0, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state0_with_fiona_progress_greets_opens_quest68_and_collapses_to_2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(ramin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(ramin) = world.characters.get_mut(&CharacterId(1)) {
        ramin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_ramin_actions(&facts(CharacterId(2), 0, 4, 0, 0, 0), 1);
    assert!(events.contains(&RaminOutcomeEvent::QuestOpen68 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&RaminOutcomeEvent::UpdateRaminState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Tidings of thy deed")));
}

#[test]
fn state6_is_a_silent_no_op_waiting_for_the_skellies() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(ramin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(ramin) = world.characters.get_mut(&CharacterId(1)) {
        ramin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_ramin_actions(&facts(CharacterId(2), 6, 4, 0, 0, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state9_without_monk_progress_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(ramin_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.level = 54;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(ramin) = world.characters.get_mut(&CharacterId(1)) {
        ramin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_ramin_actions(&facts(CharacterId(2), 9, 4, 19, 0, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state9_with_monk_progress_and_low_rammy_state_speaks_and_advances_to_11() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(ramin_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.level = 54;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(ramin) = world.characters.get_mut(&CharacterId(1)) {
        ramin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_ramin_actions(&facts(CharacterId(2), 9, 4, 20, 10, 0), 1);
    assert!(events.contains(&RaminOutcomeEvent::UpdateRaminState {
        player_id: CharacterId(2),
        new_state: 11,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("trouble opening the fortress")));
}

#[test]
fn state9_with_monk_progress_and_high_rammy_state_advances_silently() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(ramin_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.level = 54;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(ramin) = world.characters.get_mut(&CharacterId(1)) {
        ramin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // C: dialogue is conditional on `rammy_state < 14` but the state
    // advance/`didsay` happen unconditionally either way.
    let events = world.process_ramin_actions(&facts(CharacterId(2), 9, 4, 20, 14, 0), 1);
    assert!(events.contains(&RaminOutcomeEvent::UpdateRaminState {
        player_id: CharacterId(2),
        new_state: 11,
    }));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state11_without_rammy_progress_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(ramin_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.level = 60;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(ramin) = world.characters.get_mut(&CharacterId(1)) {
        ramin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_ramin_actions(&facts(CharacterId(2), 11, 4, 20, 17, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state11_with_rammy_progress_greets_by_name_and_advances_to_13() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(ramin_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.level = 60;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(ramin) = world.characters.get_mut(&CharacterId(1)) {
        ramin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_ramin_actions(&facts(CharacterId(2), 11, 4, 20, 18, 0), 1);
    assert!(events.contains(&RaminOutcomeEvent::UpdateRaminState {
        player_id: CharacterId(2),
        new_state: 13,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("good to see you again, Godmode")));
}

#[test]
fn state16_is_a_silent_no_op_all_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(ramin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(ramin) = world.characters.get_mut(&CharacterId(1)) {
        ramin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_ramin_actions(&facts(CharacterId(2), 16, 4, 20, 18, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn give_letter2_with_bit_unset_is_accepted_and_sets_the_bit() {
    let mut world = World::default();
    let mut ramin = ramin_npc(1);
    ramin.cursor_item = Some(ItemId(50));
    world.add_character(ramin);
    let mut letter = item(50, ItemFlags::empty());
    letter.name = "a worried letter".into();
    letter.template_id = IID_ARKHATA_LETTER2;
    letter.carried_by = Some(CharacterId(1));
    world.add_item(letter);
    world.add_character(player(2, "Godmode"));

    if let Some(ramin) = world.characters.get_mut(&CharacterId(1)) {
        ramin.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_ramin_actions(&facts(CharacterId(2), 16, 4, 20, 18, 0), 1);
    assert!(events.contains(&RaminOutcomeEvent::GiveLetter2Bit {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("comfort and solution")));
    assert!(!world.items.contains_key(&ItemId(50)));
}

#[test]
fn give_letter2_with_bit_already_set_is_handed_back_with_dialogue() {
    let mut world = World::default();
    let mut ramin = ramin_npc(1);
    ramin.cursor_item = Some(ItemId(50));
    world.add_character(ramin);
    let mut letter = item(50, ItemFlags::empty());
    letter.template_id = IID_ARKHATA_LETTER2;
    letter.carried_by = Some(CharacterId(1));
    world.add_item(letter);
    world.add_character(player(2, "Godmode"));

    if let Some(ramin) = world.characters.get_mut(&CharacterId(1)) {
        ramin.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // letter_bits already has bit 2 set.
    let events = world.process_ramin_actions(&facts(CharacterId(2), 16, 4, 20, 18, 2), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, RaminOutcomeEvent::GiveLetter2Bit { .. })));
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
    let mut ramin = ramin_npc(1);
    ramin.cursor_item = Some(ItemId(50));
    world.add_character(ramin);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(ramin) = world.characters.get_mut(&CharacterId(1)) {
        ramin.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_ramin_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn text_repeat_resets_to_0_when_at_or_below_state6() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(ramin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(ramin) = world.characters.get_mut(&CharacterId(1)) {
        ramin.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_ramin_actions(&facts(CharacterId(2), 4, 4, 0, 0, 0), 1);
    assert!(events.contains(&RaminOutcomeEvent::UpdateRaminState {
        player_id: CharacterId(2),
        new_state: 0,
    }));
}

#[test]
fn text_repeat_resets_to_7_when_between_states_7_and_9() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(ramin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(ramin) = world.characters.get_mut(&CharacterId(1)) {
        ramin.push_driver_text_message(CharacterId(2), "restart");
    }
    let events = world.process_ramin_actions(&facts(CharacterId(2), 9, 4, 0, 0, 0), 1);
    assert!(events.contains(&RaminOutcomeEvent::UpdateRaminState {
        player_id: CharacterId(2),
        new_state: 7,
    }));
}

#[test]
fn text_repeat_resets_to_10_when_between_states_10_and_11() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(ramin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(ramin) = world.characters.get_mut(&CharacterId(1)) {
        ramin.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_ramin_actions(&facts(CharacterId(2), 11, 4, 20, 0, 0), 1);
    assert!(events.contains(&RaminOutcomeEvent::UpdateRaminState {
        player_id: CharacterId(2),
        new_state: 10,
    }));
}

#[test]
fn text_repeat_resets_to_12_when_between_states_12_and_16() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(ramin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(ramin) = world.characters.get_mut(&CharacterId(1)) {
        ramin.push_driver_text_message(CharacterId(2), "restart");
    }
    let events = world.process_ramin_actions(&facts(CharacterId(2), 15, 4, 20, 18, 0), 1);
    assert!(events.contains(&RaminOutcomeEvent::UpdateRaminState {
        player_id: CharacterId(2),
        new_state: 12,
    }));
}
