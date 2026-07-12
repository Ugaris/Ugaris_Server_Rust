use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_ARKHATAMONK, NT_CHAR, NT_GIVE};
use crate::item_driver::{IID_ARKHATA_DICTIONARY, IID_ARKHATA_MONKPART1, IID_ARKHATA_MONKPART2};
use crate::world::npc::area37::arkhatamonk::{
    ArkhatamonkDriverData, ArkhatamonkOutcomeEvent, ArkhatamonkPlayerFacts,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn monk_npc(id: u32, name: &str) -> Character {
    let mut monk = character(id);
    monk.name = name.into();
    monk.driver = CDR_ARKHATAMONK;
    monk.driver_state = Some(CharacterDriverState::Arkhatamonk(
        ArkhatamonkDriverData::default(),
    ));
    monk
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    monk_state: i32,
    monk_bits: i32,
    ramin_state: i32,
) -> HashMap<CharacterId, ArkhatamonkPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        ArkhatamonkPlayerFacts {
            monk_state,
            monk_bits,
            ramin_state,
        },
    );
    map
}

#[test]
fn state0_without_ramin_progress_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(monk_npc(1, "Gregor"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(monk) = world.characters.get_mut(&CharacterId(1)) {
        monk.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_arkhatamonk_actions(&facts(CharacterId(2), 0, 0, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state0_with_ramin_progress_and_gregor_present_speaks_and_advances_to_2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(monk_npc(1, "Gregor"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(monk) = world.characters.get_mut(&CharacterId(1)) {
        monk.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_arkhatamonk_actions(&facts(CharacterId(2), 0, 0, 7), 1);
    assert!(events.contains(&ArkhatamonkOutcomeEvent::UpdateMonkState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Dried leaves of lavender")));
}

#[test]
fn state0_with_ramin_progress_but_wrong_persona_collapses_to_1_silently() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(monk_npc(1, "Johan"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(monk) = world.characters.get_mut(&CharacterId(1)) {
        monk.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_arkhatamonk_actions(&facts(CharacterId(2), 0, 0, 7), 1);
    assert!(events.contains(&ArkhatamonkOutcomeEvent::UpdateMonkState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state6_greets_by_johan_and_opens_no_quest_yet() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(monk_npc(1, "Johan"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(monk) = world.characters.get_mut(&CharacterId(1)) {
        monk.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_arkhatamonk_actions(&facts(CharacterId(2), 6, 0, 7), 1);
    assert!(events.contains(&ArkhatamonkOutcomeEvent::UpdateMonkState {
        player_id: CharacterId(2),
        new_state: 7,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Ramin must have sent thee")));
}

#[test]
fn state7_with_johnatan_opens_quest69_and_advances_to_8() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(monk_npc(1, "Johnatan"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(monk) = world.characters.get_mut(&CharacterId(1)) {
        monk.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_arkhatamonk_actions(&facts(CharacterId(2), 7, 0, 7), 1);
    assert!(events.contains(&ArkhatamonkOutcomeEvent::QuestOpen69 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&ArkhatamonkOutcomeEvent::UpdateMonkState {
        player_id: CharacterId(2),
        new_state: 8,
    }));
}

#[test]
fn state11_is_a_silent_wait_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(monk_npc(1, "Johnatan"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(monk) = world.characters.get_mut(&CharacterId(1)) {
        monk.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_arkhatamonk_actions(&facts(CharacterId(2), 11, 0, 7), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state12_is_a_silent_wait_for_keyparts() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(monk_npc(1, "Johnatan"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(monk) = world.characters.get_mut(&CharacterId(1)) {
        monk.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_arkhatamonk_actions(&facts(CharacterId(2), 12, 3, 7), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state15_with_tracy_opens_quest70_and_advances_to_16() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(monk_npc(1, "Tracy"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(monk) = world.characters.get_mut(&CharacterId(1)) {
        monk.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_arkhatamonk_actions(&facts(CharacterId(2), 15, 7, 7), 1);
    assert!(events.contains(&ArkhatamonkOutcomeEvent::QuestOpen70 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&ArkhatamonkOutcomeEvent::UpdateMonkState {
        player_id: CharacterId(2),
        new_state: 16,
    }));
}

#[test]
fn state19_is_a_silent_wait_for_the_book_eater_kill() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(monk_npc(1, "Tracy"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(monk) = world.characters.get_mut(&CharacterId(1)) {
        monk.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_arkhatamonk_actions(&facts(CharacterId(2), 19, 7, 7), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state20_with_tracy_rewards_gold_and_advances_to_21() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(monk_npc(1, "Tracy"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(monk) = world.characters.get_mut(&CharacterId(1)) {
        monk.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_arkhatamonk_actions(&facts(CharacterId(2), 20, 7, 7), 1);
    assert!(events.contains(&ArkhatamonkOutcomeEvent::UpdateMonkState {
        player_id: CharacterId(2),
        new_state: 21,
    }));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.gold, 200 * 100);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("novels should be safe")));
}

#[test]
fn state21_with_johnatan_opens_quest78_and_advances_to_22() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(monk_npc(1, "Johnatan"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(monk) = world.characters.get_mut(&CharacterId(1)) {
        monk.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_arkhatamonk_actions(&facts(CharacterId(2), 21, 7, 7), 1);
    assert!(events.contains(&ArkhatamonkOutcomeEvent::QuestOpen78 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&ArkhatamonkOutcomeEvent::UpdateMonkState {
        player_id: CharacterId(2),
        new_state: 22,
    }));
}

#[test]
fn state28_is_a_silent_wait_for_corby() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(monk_npc(1, "Johnatan"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(monk) = world.characters.get_mut(&CharacterId(1)) {
        monk.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_arkhatamonk_actions(&facts(CharacterId(2), 28, 7, 7), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state29_with_johnatan_rewards_gold_and_advances_to_30() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(monk_npc(1, "Johnatan"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(monk) = world.characters.get_mut(&CharacterId(1)) {
        monk.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_arkhatamonk_actions(&facts(CharacterId(2), 29, 7, 7), 1);
    assert!(events.contains(&ArkhatamonkOutcomeEvent::UpdateMonkState {
        player_id: CharacterId(2),
        new_state: 30,
    }));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.gold, 3000 * 100);
}

#[test]
fn state30_is_a_silent_no_op_all_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(monk_npc(1, "Johnatan"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(monk) = world.characters.get_mut(&CharacterId(1)) {
        monk.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_arkhatamonk_actions(&facts(CharacterId(2), 30, 7, 7), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn give_gregor_keypart_at_state12_sets_bit_and_thanks() {
    let mut world = World::default();
    let mut gregor = monk_npc(1, "Gregor");
    gregor.cursor_item = Some(ItemId(50));
    world.add_character(gregor);
    let mut part = item(50, ItemFlags::empty());
    part.template_id = IID_ARKHATA_MONKPART1;
    part.carried_by = Some(CharacterId(1));
    world.add_item(part);
    world.add_character(player(2, "Godmode"));

    if let Some(gregor) = world.characters.get_mut(&CharacterId(1)) {
        gregor.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_arkhatamonk_actions(&facts(CharacterId(2), 12, 0, 7), 1);
    assert!(events.contains(&ArkhatamonkOutcomeEvent::UpdateMonkBits {
        player_id: CharacterId(2),
        new_bits: 1,
    }));
    assert!(!events
        .iter()
        .any(|event| matches!(event, ArkhatamonkOutcomeEvent::QuestDone69 { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("My key-part! I thank thee, Godmode")));
    assert!(world.items.get(&ItemId(50)).is_none());
}

#[test]
fn give_last_keypart_completes_all_bits_and_finishes_quest69() {
    let mut world = World::default();
    let mut johnatan = monk_npc(1, "Johnatan");
    johnatan.cursor_item = Some(ItemId(50));
    world.add_character(johnatan);
    let mut part = item(50, ItemFlags::empty());
    part.template_id = IID_ARKHATA_MONKPART2;
    part.carried_by = Some(CharacterId(1));
    world.add_item(part);
    world.add_character(player(2, "Godmode"));

    if let Some(johnatan) = world.characters.get_mut(&CharacterId(1)) {
        johnatan.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // monk_bits already has Gregor(1)+Johan(2) set; this turn-in completes it.
    let events = world.process_arkhatamonk_actions(&facts(CharacterId(2), 12, 3, 7), 1);
    assert!(events.contains(&ArkhatamonkOutcomeEvent::UpdateMonkBits {
        player_id: CharacterId(2),
        new_bits: 7,
    }));
    assert!(events.contains(&ArkhatamonkOutcomeEvent::QuestDone69 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&ArkhatamonkOutcomeEvent::UpdateMonkState {
        player_id: CharacterId(2),
        new_state: 13,
    }));
}

#[test]
fn give_dictionary_at_state28_grants_exp_and_advances() {
    let mut world = World::default();
    let mut johnatan = monk_npc(1, "Johnatan");
    johnatan.cursor_item = Some(ItemId(50));
    world.add_character(johnatan);
    let mut book = item(50, ItemFlags::empty());
    book.template_id = IID_ARKHATA_DICTIONARY;
    book.carried_by = Some(CharacterId(1));
    world.add_item(book);
    world.add_character(player(2, "Godmode"));

    if let Some(johnatan) = world.characters.get_mut(&CharacterId(1)) {
        johnatan.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_arkhatamonk_actions(&facts(CharacterId(2), 28, 7, 7), 1);
    assert!(events.contains(&ArkhatamonkOutcomeEvent::UpdateMonkState {
        player_id: CharacterId(2),
        new_state: 29,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("translate the language")));
    assert!(world.items.get(&ItemId(50)).is_none());
}

#[test]
fn give_unrelated_item_is_handed_back() {
    let mut world = World::default();
    let mut gregor = monk_npc(1, "Gregor");
    gregor.cursor_item = Some(ItemId(50));
    world.add_character(gregor);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(gregor) = world.characters.get_mut(&CharacterId(1)) {
        gregor.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_arkhatamonk_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn text_repeat_resets_to_8_when_between_states_8_and_12() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(monk_npc(1, "Johnatan"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(monk) = world.characters.get_mut(&CharacterId(1)) {
        monk.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_arkhatamonk_actions(&facts(CharacterId(2), 10, 0, 7), 1);
    assert!(events.contains(&ArkhatamonkOutcomeEvent::UpdateMonkState {
        player_id: CharacterId(2),
        new_state: 8,
    }));
}

#[test]
fn text_repeat_resets_to_15_when_between_states_15_and_19() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(monk_npc(1, "Tracy"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(monk) = world.characters.get_mut(&CharacterId(1)) {
        monk.push_driver_text_message(CharacterId(2), "restart");
    }
    let events = world.process_arkhatamonk_actions(&facts(CharacterId(2), 17, 7, 7), 1);
    assert!(events.contains(&ArkhatamonkOutcomeEvent::UpdateMonkState {
        player_id: CharacterId(2),
        new_state: 15,
    }));
}

#[test]
fn text_repeat_resets_to_21_when_between_states_21_and_28() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(monk_npc(1, "Johnatan"), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(monk) = world.characters.get_mut(&CharacterId(1)) {
        monk.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_arkhatamonk_actions(&facts(CharacterId(2), 25, 7, 7), 1);
    assert!(events.contains(&ArkhatamonkOutcomeEvent::UpdateMonkState {
        player_id: CharacterId(2),
        new_state: 21,
    }));
}
