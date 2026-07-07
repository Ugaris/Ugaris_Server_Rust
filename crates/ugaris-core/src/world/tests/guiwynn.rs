use std::collections::HashMap;

use super::*;
use crate::character_driver::{GuiwynnDriverData, CDR_GUIWYNN, NT_CHAR, NT_GIVE};
use crate::item_driver::{IID_AREA1_MADKEY1, IID_AREA1_MADNOTE, IID_AREA1_MADPOTION};
use crate::world::guiwynn::{GuiwynnOutcomeEvent, GuiwynnPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn guiwynn_npc(id: u32) -> Character {
    let mut guiwynn = character(id);
    guiwynn.name = "Guiwynn".into();
    guiwynn.driver = CDR_GUIWYNN;
    guiwynn.driver_state = Some(CharacterDriverState::Guiwynn(GuiwynnDriverData::default()));
    guiwynn
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
    seen_timer: i32,
    gwendy_state: i32,
    quest8_done: bool,
) -> HashMap<CharacterId, GuiwynnPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        GuiwynnPlayerFacts {
            state,
            seen_timer,
            gwendy_state,
            quest8_done,
        },
    );
    map
}

fn guiwynn_state(world: &World, guiwynn_id: CharacterId) -> GuiwynnDriverData {
    match world
        .characters
        .get(&guiwynn_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Guiwynn(data)) => data,
        _ => panic!("expected guiwynn driver state"),
    }
}

#[test]
fn guiwynn_entry_stays_silent_before_second_skull_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guiwynn_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(guiwynn) = world.characters.get_mut(&CharacterId(1)) {
        guiwynn.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // gwendy_state 10 < 17: no quest offer yet.
    let events = world.process_guiwynn_actions(&facts(CharacterId(2), 0, 0, 10, false), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, GuiwynnOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn guiwynn_entry_greets_opens_quest7_and_advances() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guiwynn_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(guiwynn) = world.characters.get_mut(&CharacterId(1)) {
        guiwynn.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_guiwynn_actions(&facts(CharacterId(2), 0, 0, 17, false), 1_000, 1);
    assert!(events.contains(&GuiwynnOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 7,
    }));
    assert!(events.contains(&GuiwynnOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    assert!(events.contains(&GuiwynnOutcomeEvent::UpdateSeenTimer {
        player_id: CharacterId(2),
        value: 1_000,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("please wait a moment")));
}

#[test]
fn guiwynn_state4_grants_key_when_missing() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guiwynn_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(guiwynn) = world.characters.get_mut(&CharacterId(1)) {
        guiwynn.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_guiwynn_actions(&facts(CharacterId(2), 4, 950, 17, false), 1_000, 1);
    assert!(events.contains(&GuiwynnOutcomeEvent::GrantKeyItem {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&GuiwynnOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 5,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("This key opens the front door")));
}

#[test]
fn guiwynn_state4_skips_key_grant_when_already_carried() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guiwynn_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    if let Some(player_char) = world.characters.get_mut(&CharacterId(2)) {
        player_char.cursor_item = Some(ItemId(99));
    }
    let mut key = item(99, ItemFlags::empty());
    key.template_id = IID_AREA1_MADKEY1;
    key.carried_by = Some(CharacterId(2));
    world.add_item(key);

    world.tick = Tick(BASELINE_TICK);
    if let Some(guiwynn) = world.characters.get_mut(&CharacterId(1)) {
        guiwynn.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_guiwynn_actions(&facts(CharacterId(2), 4, 950, 17, false), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, GuiwynnOutcomeEvent::GrantKeyItem { .. })));
}

#[test]
fn guiwynn_state5_reminds_after_gate_elapses() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guiwynn_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(guiwynn) = world.characters.get_mut(&CharacterId(1)) {
        guiwynn.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // seen_timer old (now - seen_timer > 60): reminder fires, no state change.
    let events = world.process_guiwynn_actions(&facts(CharacterId(2), 5, 900, 17, false), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, GuiwynnOutcomeEvent::UpdateState { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Didst thou find out anything about the Order")));
}

#[test]
fn guiwynn_state5_stays_silent_within_reminder_gate() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guiwynn_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(guiwynn) = world.characters.get_mut(&CharacterId(1)) {
        guiwynn.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_guiwynn_actions(&facts(CharacterId(2), 5, 990, 17, false), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, GuiwynnOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn guiwynn_state6_skips_to_11_when_quest8_already_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guiwynn_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(guiwynn) = world.characters.get_mut(&CharacterId(1)) {
        guiwynn.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_guiwynn_actions(&facts(CharacterId(2), 6, 0, 17, true), 1_000, 1);
    assert!(events.contains(&GuiwynnOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 11,
    }));
    assert!(!events
        .iter()
        .any(|event| matches!(event, GuiwynnOutcomeEvent::QuestOpen { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn guiwynn_state6_opens_quest8_when_not_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guiwynn_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(guiwynn) = world.characters.get_mut(&CharacterId(1)) {
        guiwynn.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_guiwynn_actions(&facts(CharacterId(2), 6, 0, 17, false), 1_000, 1);
    assert!(events.contains(&GuiwynnOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 8,
    }));
    assert!(events.contains(&GuiwynnOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 7,
    }));
}

#[test]
fn guiwynn_state11_is_a_reminder_only_done_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guiwynn_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(guiwynn) = world.characters.get_mut(&CharacterId(1)) {
        guiwynn.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_guiwynn_actions(&facts(CharacterId(2), 11, 900, 17, true), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, GuiwynnOutcomeEvent::UpdateState { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Nice to see you")));
}

#[test]
fn guiwynn_give_madpotion_finishes_quest7_in_state_range() {
    let mut world = World::default();
    let mut guiwynn = guiwynn_npc(1);
    guiwynn.cursor_item = Some(ItemId(50));
    world.add_character(guiwynn);
    let mut potion = item(50, ItemFlags::empty());
    potion.template_id = IID_AREA1_MADPOTION;
    potion.carried_by = Some(CharacterId(1));
    world.add_item(potion);
    world.add_character(player(2, "Godmode"));

    if let Some(guiwynn) = world.characters.get_mut(&CharacterId(1)) {
        guiwynn.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_guiwynn_actions(&facts(CharacterId(2), 3, 0, 17, false), 1_000, 1);
    assert!(events.contains(&GuiwynnOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest: 7,
    }));
    assert!(events.contains(&GuiwynnOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
    assert!(world.items.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("that might be what was looking for")));
}

#[test]
fn guiwynn_give_madnote_finishes_quest8_in_state_range() {
    let mut world = World::default();
    let mut guiwynn = guiwynn_npc(1);
    guiwynn.cursor_item = Some(ItemId(51));
    world.add_character(guiwynn);
    let mut note = item(51, ItemFlags::empty());
    note.template_id = IID_AREA1_MADNOTE;
    note.carried_by = Some(CharacterId(1));
    world.add_item(note);
    world.add_character(player(2, "Godmode"));

    if let Some(guiwynn) = world.characters.get_mut(&CharacterId(1)) {
        guiwynn.push_driver_message(NT_GIVE, 2, 51, 0);
    }

    let events = world.process_guiwynn_actions(&facts(CharacterId(2), 7, 0, 17, false), 1_000, 1);
    assert!(events.contains(&GuiwynnOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest: 8,
    }));
    assert!(events.contains(&GuiwynnOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 10,
    }));
    assert!(world.items.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("this is the recipe I was looking for")));
}

#[test]
fn guiwynn_give_madpotion_outside_state_range_is_a_normal_give_back() {
    let mut world = World::default();
    let mut guiwynn = guiwynn_npc(1);
    guiwynn.cursor_item = Some(ItemId(50));
    world.add_character(guiwynn);
    let mut potion = item(50, ItemFlags::empty());
    potion.template_id = IID_AREA1_MADPOTION;
    potion.carried_by = Some(CharacterId(1));
    world.add_item(potion);
    world.add_character(player(2, "Godmode"));

    if let Some(guiwynn) = world.characters.get_mut(&CharacterId(1)) {
        guiwynn.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // state 6: outside the `<= 5` range the C code checks for the potion.
    let events = world.process_guiwynn_actions(&facts(CharacterId(2), 6, 0, 17, false), 1_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, GuiwynnOutcomeEvent::QuestDone { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn guiwynn_text_repeat_resets_early_states_to_zero() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guiwynn_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(guiwynn) = world.characters.get_mut(&CharacterId(1)) {
        guiwynn.driver_state = Some(CharacterDriverState::Guiwynn(GuiwynnDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        guiwynn.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_guiwynn_actions(&facts(CharacterId(2), 3, 0, 17, false), 1_000, 1);
    assert!(events.contains(&GuiwynnOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 0,
    }));
    assert_eq!(guiwynn_state(&world, CharacterId(1)).last_talk, 0);
    assert_eq!(
        guiwynn_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn guiwynn_text_repeat_resets_mid_states_to_six() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guiwynn_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(guiwynn) = world.characters.get_mut(&CharacterId(1)) {
        guiwynn.driver_state = Some(CharacterDriverState::Guiwynn(GuiwynnDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        guiwynn.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_guiwynn_actions(&facts(CharacterId(2), 7, 0, 17, false), 1_000, 1);
    assert!(events.contains(&GuiwynnOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
    assert_eq!(guiwynn_state(&world, CharacterId(1)).last_talk, 0);
}

#[test]
fn guiwynn_text_repeat_resets_late_states_to_nine() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(guiwynn_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(guiwynn) = world.characters.get_mut(&CharacterId(1)) {
        guiwynn.driver_state = Some(CharacterDriverState::Guiwynn(GuiwynnDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        guiwynn.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_guiwynn_actions(&facts(CharacterId(2), 10, 0, 17, false), 1_000, 1);
    assert!(events.contains(&GuiwynnOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 9,
    }));
    assert_eq!(guiwynn_state(&world, CharacterId(1)).last_talk, 0);
}

#[test]
fn guiwynn_idle_moves_toward_post_after_talk_gate_elapses() {
    let mut world = World::default();
    assert!(world.spawn_character(guiwynn_npc(1), 10, 10));
    if let Some(guiwynn) = world.characters.get_mut(&CharacterId(1)) {
        guiwynn.rest_x = 10;
        guiwynn.rest_y = 10;
        guiwynn.x = 15;
        guiwynn.y = 15;
        guiwynn.driver_state = Some(CharacterDriverState::Guiwynn(GuiwynnDriverData {
            last_talk: 0,
            current_victim: None,
        }));
    }

    world.tick = Tick(TICKS_PER_SECOND * 100);
    world.process_guiwynn_actions(&HashMap::new(), 1_000, 1);
    // No panics, no crash: the idle move attempt runs to completion.
}
