use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_JADA, NT_CHAR, NT_GIVE};
use crate::item_driver::IID_ARKHATA_BLADE;
use crate::world::npc::area37::jada::{JadaDriverData, JadaOutcomeEvent, JadaPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn jada_npc(id: u32) -> Character {
    let mut jada = character(id);
    jada.name = "Jada".into();
    jada.driver = CDR_JADA;
    jada.driver_state = Some(CharacterDriverState::Jada(JadaDriverData::default()));
    jada
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    jada_state: i32,
    ramin_state: i32,
) -> HashMap<CharacterId, JadaPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        JadaPlayerFacts {
            jada_state,
            ramin_state,
        },
    );
    map
}

#[test]
fn state0_without_ramin_progress_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jada_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jada) = world.characters.get_mut(&CharacterId(1)) {
        jada.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jada_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state0_with_ramin_progress_greets_opens_quest72_and_collapses_to_2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jada_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jada) = world.characters.get_mut(&CharacterId(1)) {
        jada.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jada_actions(&facts(CharacterId(2), 0, 12), 1);
    assert!(events.contains(&JadaOutcomeEvent::QuestOpen72 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&JadaOutcomeEvent::UpdateJadaState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Thou hast been sent from Ramin")));
}

#[test]
fn state2_speaks_and_advances_to_3() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jada_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jada) = world.characters.get_mut(&CharacterId(1)) {
        jada.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jada_actions(&facts(CharacterId(2), 2, 12), 1);
    assert!(events.contains(&JadaOutcomeEvent::UpdateJadaState {
        player_id: CharacterId(2),
        new_state: 3,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("source of this evil")));
}

#[test]
fn state3_speaks_and_advances_to_4() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jada_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jada) = world.characters.get_mut(&CharacterId(1)) {
        jada.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jada_actions(&facts(CharacterId(2), 3, 12), 1);
    assert!(events.contains(&JadaOutcomeEvent::UpdateJadaState {
        player_id: CharacterId(2),
        new_state: 4,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("safe entrance to the cave system")));
}

#[test]
fn state4_is_a_silent_no_op_waiting_for_the_blade() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jada_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jada) = world.characters.get_mut(&CharacterId(1)) {
        jada.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jada_actions(&facts(CharacterId(2), 4, 12), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state5_is_a_silent_no_op_all_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jada_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jada) = world.characters.get_mut(&CharacterId(1)) {
        jada.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jada_actions(&facts(CharacterId(2), 5, 12), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn give_blade_while_turn_in_window_open_completes_quest_and_sets_state5() {
    let mut world = World::default();
    let mut jada = jada_npc(1);
    jada.cursor_item = Some(ItemId(50));
    world.add_character(jada);
    let mut blade = item(50, ItemFlags::empty());
    blade.name = "an evil blade".into();
    blade.template_id = IID_ARKHATA_BLADE;
    blade.carried_by = Some(CharacterId(1));
    world.add_item(blade);
    world.add_character(player(2, "Godmode"));

    if let Some(jada) = world.characters.get_mut(&CharacterId(1)) {
        jada.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_jada_actions(&facts(CharacterId(2), 4, 12), 1);
    assert!(events.contains(&JadaOutcomeEvent::QuestDone72 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&JadaOutcomeEvent::UpdateJadaState {
        player_id: CharacterId(2),
        new_state: 5,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("concentration of evil")));
    assert!(world.items.get(&ItemId(50)).is_none());
}

#[test]
fn give_blade_outside_turn_in_window_is_handed_back() {
    let mut world = World::default();
    let mut jada = jada_npc(1);
    jada.cursor_item = Some(ItemId(50));
    world.add_character(jada);
    let mut blade = item(50, ItemFlags::empty());
    blade.name = "an evil blade".into();
    blade.template_id = IID_ARKHATA_BLADE;
    blade.carried_by = Some(CharacterId(1));
    world.add_item(blade);
    world.add_character(player(2, "Godmode"));

    if let Some(jada) = world.characters.get_mut(&CharacterId(1)) {
        jada.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // state 5 ("all done"): outside the `1..=4` turn-in window.
    let events = world.process_jada_actions(&facts(CharacterId(2), 5, 12), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, JadaOutcomeEvent::QuestDone72 { .. })));
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
    let mut jada = jada_npc(1);
    jada.cursor_item = Some(ItemId(50));
    world.add_character(jada);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(jada) = world.characters.get_mut(&CharacterId(1)) {
        jada.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_jada_actions(&HashMap::new(), 1);
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
    assert!(world.spawn_character(jada_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(jada) = world.characters.get_mut(&CharacterId(1)) {
        jada.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_jada_actions(&facts(CharacterId(2), 3, 12), 1);
    assert!(events.contains(&JadaOutcomeEvent::UpdateJadaState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
}

#[test]
fn text_repeat_is_a_no_op_outside_turn_in_window() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jada_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(jada) = world.characters.get_mut(&CharacterId(1)) {
        jada.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_jada_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.is_empty());
}
