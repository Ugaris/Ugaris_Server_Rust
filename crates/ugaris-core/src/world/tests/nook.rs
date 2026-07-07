use std::collections::HashMap;

use super::*;
use crate::character_driver::{NookDriverData, CDR_NOOK, NT_CHAR, NT_GIVE};
use crate::item_driver::{IID_AREA1_JESTERCAP, IID_AREA1_ROBBERKEY1};
use crate::quest::{GWENDYLON_STATE_SECOND_SKULL_DONE, GWENDYLON_STATE_THIRD_SKULL_DONE};
use crate::world::nook::{NookOutcomeEvent, NookPlayerFacts};

/// Same rationale as `world::jessica`'s own `BASELINE_TICK` (its module's
/// C source, `gwendylon.c`, shares the same `dat->current_victim != co`
/// boot-time-only quirk).
const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn nook_npc(id: u32) -> Character {
    let mut nook = character(id);
    nook.name = "Nook".into();
    nook.driver = CDR_NOOK;
    nook.driver_state = Some(CharacterDriverState::Nook(NookDriverData::default()));
    nook
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    state: i32,
    gwendy_state: i32,
) -> HashMap<CharacterId, NookPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        NookPlayerFacts {
            state,
            gwendy_state,
        },
    );
    map
}

fn nook_state(world: &World, nook_id: CharacterId) -> NookDriverData {
    match world
        .characters
        .get(&nook_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Nook(data)) => data,
        _ => panic!("expected nook driver state"),
    }
}

#[test]
fn nook_entry_greets_and_advances() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(nook_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(nook) = world.characters.get_mut(&CharacterId(1)) {
        nook.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_nook_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.contains(&NookOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I am Nook, the judge")));
}

#[test]
fn nook_state4_stays_silent_until_second_skull_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(nook_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(nook) = world.characters.get_mut(&CharacterId(1)) {
        nook.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_nook_actions(&facts(CharacterId(2), 4, 0), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, NookOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn nook_state4_advances_and_talks_once_second_skull_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(nook_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(nook) = world.characters.get_mut(&CharacterId(1)) {
        nook.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_nook_actions(
        &facts(CharacterId(2), 4, GWENDYLON_STATE_SECOND_SKULL_DONE),
        1,
    );
    assert!(events.contains(&NookOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 5,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("skeletons coming out")));
}

#[test]
fn nook_state5_opens_quest_once_third_skull_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(nook_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(nook) = world.characters.get_mut(&CharacterId(1)) {
        nook.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_nook_actions(
        &facts(CharacterId(2), 5, GWENDYLON_STATE_THIRD_SKULL_DONE),
        1,
    );
    assert!(events.contains(&NookOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&NookOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
}

#[test]
fn nook_state11_is_a_silent_waiting_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(nook_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(nook) = world.characters.get_mut(&CharacterId(1)) {
        nook.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_nook_actions(&facts(CharacterId(2), 11, 0), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, NookOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn nook_state12_advances_silently_once_gwendy_done_bless() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(nook_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(nook) = world.characters.get_mut(&CharacterId(1)) {
        nook.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // GWENDYLON_STATE_DONE_BLESS == 19 (see nook.rs's local re-declaration).
    let events = world.process_nook_actions(&facts(CharacterId(2), 12, 19), 1);
    assert!(events.contains(&NookOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 16,
    }));
    // C: this transition never calls `quiet_say`, so no message fires.
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn nook_give_jestercap_finishes_quest_in_state_range() {
    let mut world = World::default();
    let mut nook = nook_npc(1);
    nook.cursor_item = Some(ItemId(50));
    world.add_character(nook);
    let mut cap = item(50, ItemFlags::empty());
    cap.template_id = IID_AREA1_JESTERCAP;
    cap.carried_by = Some(CharacterId(1));
    world.add_item(cap);
    world.add_character(player(2, "Godmode"));

    if let Some(nook) = world.characters.get_mut(&CharacterId(1)) {
        nook.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_nook_actions(&facts(CharacterId(2), 7, 0), 1);
    assert!(events.contains(&NookOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&NookOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 12,
    }));
    assert!(world.items.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("There it is! My cap!")));
    assert!(texts
        .iter()
        .any(|text| text.message.contains("eternal gratitude")));
}

#[test]
fn nook_give_jestercap_outside_state_range_is_a_normal_give_back() {
    let mut world = World::default();
    let mut nook = nook_npc(1);
    nook.cursor_item = Some(ItemId(50));
    world.add_character(nook);
    let mut cap = item(50, ItemFlags::empty());
    cap.template_id = IID_AREA1_JESTERCAP;
    cap.carried_by = Some(CharacterId(1));
    world.add_item(cap);
    world.add_character(player(2, "Godmode"));

    if let Some(nook) = world.characters.get_mut(&CharacterId(1)) {
        nook.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // state 0: outside the [5, 11] range the C code checks.
    let events = world.process_nook_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, NookOutcomeEvent::QuestDone { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn nook_give_robberkey_alone_is_destroyed_by_template_sweep_on_turn_in() {
    // Sanity: IID_AREA1_ROBBERKEY1 is distinct from IID_AREA1_JESTERCAP.
    assert_ne!(IID_AREA1_ROBBERKEY1, IID_AREA1_JESTERCAP);
}

#[test]
fn nook_text_repeat_resets_early_states_to_zero() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(nook_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(nook) = world.characters.get_mut(&CharacterId(1)) {
        nook.driver_state = Some(CharacterDriverState::Nook(NookDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        nook.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_nook_actions(&facts(CharacterId(2), 3, 0), 1);
    assert!(events.contains(&NookOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 0,
    }));
    assert_eq!(nook_state(&world, CharacterId(1)).last_talk, 0);
    assert_eq!(
        nook_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn nook_text_repeat_resets_middle_states_to_five() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(nook_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(nook) = world.characters.get_mut(&CharacterId(1)) {
        nook.driver_state = Some(CharacterDriverState::Nook(NookDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        nook.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_nook_actions(&facts(CharacterId(2), 9, 0), 1);
    assert!(events.contains(&NookOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 5,
    }));
    assert_eq!(nook_state(&world, CharacterId(1)).last_talk, 0);
}

#[test]
fn nook_text_repeat_resets_late_states_to_twelve() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(nook_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(nook) = world.characters.get_mut(&CharacterId(1)) {
        nook.driver_state = Some(CharacterDriverState::Nook(NookDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        nook.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_nook_actions(&facts(CharacterId(2), 15, 0), 1);
    assert!(events.contains(&NookOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 12,
    }));
    assert_eq!(nook_state(&world, CharacterId(1)).last_talk, 0);
}

#[test]
fn nook_text_ignores_non_current_victim() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(nook_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    assert!(world.spawn_character(player(3, "Other"), 12, 10));

    if let Some(nook) = world.characters.get_mut(&CharacterId(1)) {
        nook.driver_state = Some(CharacterDriverState::Nook(NookDriverData {
            last_talk: BASELINE_TICK,
            current_victim: Some(CharacterId(2)),
        }));
        nook.push_driver_text_message(CharacterId(3), "hello");
    }
    world.tick = Tick(BASELINE_TICK);

    let mut player_facts = facts(CharacterId(2), 4, 0);
    player_facts.insert(
        CharacterId(3),
        NookPlayerFacts {
            state: 4,
            gwendy_state: 0,
        },
    );

    world.process_nook_actions(&player_facts, 1);
    assert!(world.drain_pending_area_texts().is_empty());
}
