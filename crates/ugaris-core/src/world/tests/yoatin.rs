use std::collections::HashMap;

use super::*;
use crate::character_driver::{YoatinDriverData, CDR_YOATIN, NT_CHAR, NT_GIVE};
use crate::item_driver::IID_STAFF_BEARHEAD;
use crate::world::npc::area28::yoatin::{YoatinOutcomeEvent, YoatinPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn yoatin_npc(id: u32) -> Character {
    let mut yoatin = character(id);
    yoatin.name = "Yoatin".into();
    yoatin.driver = CDR_YOATIN;
    yoatin.driver_state = Some(CharacterDriverState::Yoatin(YoatinDriverData::default()));
    yoatin
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(player_id: CharacterId, yoatin_state: i32) -> HashMap<CharacterId, YoatinPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, YoatinPlayerFacts { yoatin_state });
    map
}

fn yoatin_state(world: &World, yoatin_id: CharacterId) -> YoatinDriverData {
    match world
        .characters
        .get(&yoatin_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Yoatin(data)) => data,
        _ => panic!("expected yoatin driver state"),
    }
}

#[test]
fn state0_greets_opens_quest39_and_advances_to_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(yoatin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(yoatin) = world.characters.get_mut(&CharacterId(1)) {
        yoatin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_yoatin_actions(&facts(CharacterId(2), 0), 1);
    assert!(events.contains(&YoatinOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&YoatinOutcomeEvent::UpdateYoatinState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Greetings stranger!")));
    assert_eq!(
        yoatin_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn states1_through_7_advance_one_state_each_with_dialogue() {
    let cases = [
        (1, 2, "you must be Godmode"),
        (2, 3, "bears of Cameron"),
        (3, 4, "assist me with a problem"),
        (4, 5, "killed their son"),
        (5, 6, "bears scare the living daylights"),
        (6, 7, "reward thee greatly"),
        (7, 8, "full of bears and bear caves"),
    ];
    for (state, next_state, snippet) in cases {
        let mut world = World::default();
        world.map.tile_mut(12, 10).unwrap().light = 255;
        assert!(world.spawn_character(yoatin_npc(1), 10, 10));
        assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

        world.tick = Tick(BASELINE_TICK);
        if let Some(yoatin) = world.characters.get_mut(&CharacterId(1)) {
            yoatin.push_driver_message(NT_CHAR, 2, 0, 0);
        }

        let events = world.process_yoatin_actions(&facts(CharacterId(2), state), 1);
        assert!(
            events.contains(&YoatinOutcomeEvent::UpdateYoatinState {
                player_id: CharacterId(2),
                new_state: next_state,
            }),
            "state {state} should advance to {next_state}"
        );
        let texts = world.drain_pending_area_texts();
        assert!(
            texts.iter().any(|text| text.message.contains(snippet)),
            "state {state} should speak {snippet:?}"
        );
    }
}

#[test]
fn state8_is_a_silent_no_op_waiting_for_the_bearhead() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(yoatin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(yoatin) = world.characters.get_mut(&CharacterId(1)) {
        yoatin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_yoatin_actions(&facts(CharacterId(2), 8), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state9_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(yoatin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(yoatin) = world.characters.get_mut(&CharacterId(1)) {
        yoatin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_yoatin_actions(&facts(CharacterId(2), 9), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn text_repeat_resets_state_to_0_when_not_yet_past_state8() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(yoatin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(yoatin) = world.characters.get_mut(&CharacterId(1)) {
        yoatin.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_yoatin_actions(&facts(CharacterId(2), 5), 1);
    assert!(events.contains(&YoatinOutcomeEvent::UpdateYoatinState {
        player_id: CharacterId(2),
        new_state: 0,
    }));
}

#[test]
fn text_repeat_does_not_reset_once_past_state8() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(yoatin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(yoatin) = world.characters.get_mut(&CharacterId(1)) {
        yoatin.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_yoatin_actions(&facts(CharacterId(2), 9), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, YoatinOutcomeEvent::UpdateYoatinState { .. })));
}

#[test]
fn text_reset_me_speaks_reset_done_and_pushes_reset_event_for_gods_only() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(yoatin_npc(1), 10, 10));
    let mut god = player(2, "Godmode");
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 12, 10));

    if let Some(yoatin) = world.characters.get_mut(&CharacterId(1)) {
        yoatin.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_yoatin_actions(&facts(CharacterId(2), 5), 1);
    assert!(events.contains(&YoatinOutcomeEvent::ResetYoatin {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("reset done")));
}

#[test]
fn text_reset_me_is_ignored_for_non_gods() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(yoatin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(yoatin) = world.characters.get_mut(&CharacterId(1)) {
        yoatin.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_yoatin_actions(&facts(CharacterId(2), 5), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn give_bearhead_in_range_completes_quest39_and_jumps_to_9() {
    let mut world = World::default();
    let mut yoatin = yoatin_npc(1);
    yoatin.cursor_item = Some(ItemId(50));
    world.add_character(yoatin);
    let mut bearhead = item(50, ItemFlags::empty());
    bearhead.name = "Bear Head".into();
    bearhead.template_id = IID_STAFF_BEARHEAD;
    bearhead.carried_by = Some(CharacterId(1));
    world.add_item(bearhead);
    world.add_character(player(2, "Godmode"));

    if let Some(yoatin) = world.characters.get_mut(&CharacterId(1)) {
        yoatin.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_yoatin_actions(&facts(CharacterId(2), 8), 1);
    assert!(events.contains(&YoatinOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&YoatinOutcomeEvent::UpdateYoatinState {
        player_id: CharacterId(2),
        new_state: 9,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("take my belt")));
    assert!(world.items.get(&ItemId(50)).is_none());
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}

#[test]
fn give_bearhead_outside_range_is_handed_back() {
    let mut world = World::default();
    let mut yoatin = yoatin_npc(1);
    yoatin.cursor_item = Some(ItemId(50));
    world.add_character(yoatin);
    let mut bearhead = item(50, ItemFlags::empty());
    bearhead.template_id = IID_STAFF_BEARHEAD;
    bearhead.carried_by = Some(CharacterId(1));
    world.add_item(bearhead);
    world.add_character(player(2, "Godmode"));

    if let Some(yoatin) = world.characters.get_mut(&CharacterId(1)) {
        yoatin.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // yoatin_state == 9, outside the `<= 8` acceptance window.
    let events = world.process_yoatin_actions(&facts(CharacterId(2), 9), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, YoatinOutcomeEvent::QuestDone { .. })));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn give_unrelated_item_is_handed_back() {
    let mut world = World::default();
    let mut yoatin = yoatin_npc(1);
    yoatin.cursor_item = Some(ItemId(50));
    world.add_character(yoatin);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(yoatin) = world.characters.get_mut(&CharacterId(1)) {
        yoatin.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_yoatin_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
