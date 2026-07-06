use std::collections::HashMap;

use super::*;
use crate::character_driver::{JessicaDriverData, CDR_JESSICA, NT_CHAR, NT_GIVE};
use crate::item_driver::IID_AREA1_ROBBER2NOTE;
use crate::quest::{QLOG_JESSICA_KILL, QLOG_JESSICA_ROBBER_NOTE};
use crate::world::jessica::{JessicaOutcomeEvent, JessicaPlayerFacts};

/// Same rationale as `world::camhermit`'s own `BASELINE_TICK` (its
/// module's C source, `gwendylon.c`, shares the same `dat->current_victim
/// != co` boot-time-only quirk).
const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn jessica_npc(id: u32) -> Character {
    let mut jessica = character(id);
    jessica.name = "Jessica".into();
    jessica.driver = CDR_JESSICA;
    jessica.driver_state = Some(CharacterDriverState::Jessica(JessicaDriverData::default()));
    jessica
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
    nook_quest_done: bool,
) -> HashMap<CharacterId, JessicaPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        JessicaPlayerFacts {
            state,
            seen_timer,
            nook_quest_done,
        },
    );
    map
}

fn jessica_state(world: &World, jessica_id: CharacterId) -> JessicaDriverData {
    match world
        .characters
        .get(&jessica_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Jessica(data)) => data,
        _ => panic!("expected jessica driver state"),
    }
}

#[test]
fn jessica_entry_stays_silent_until_nook_quest_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jessica_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jessica) = world.characters.get_mut(&CharacterId(1)) {
        jessica.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // Nook quest not done, seen_timer=0, now=61 > 60 -> reminder line, no
    // state transition.
    let events = world.process_jessica_actions(&facts(CharacterId(2), 0, 0, false), 61, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, JessicaOutcomeEvent::UpdateState { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Nook has some work for thee")));
}

#[test]
fn jessica_entry_advances_once_nook_quest_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jessica_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jessica) = world.characters.get_mut(&CharacterId(1)) {
        jessica.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jessica_actions(&facts(CharacterId(2), 0, 0, true), 0, 1);
    assert!(events.contains(&JessicaOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    // C: entering state 1 is a bare transition this same tick (no dialogue
    // fires yet - state 1's own dialogue only fires on the *next* visit).
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn jessica_quest1_give_1_opens_quest_and_advances() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jessica_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jessica) = world.characters.get_mut(&CharacterId(1)) {
        jessica.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jessica_actions(&facts(CharacterId(2), 1, 0, true), 0, 1);
    assert!(events.contains(&JessicaOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: QLOG_JESSICA_ROBBER_NOTE,
    }));
    assert!(events.contains(&JessicaOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("My cousin Nook has spoken well")));
}

#[test]
fn jessica_quest1_do_reminds_after_sixty_seconds() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jessica_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jessica) = world.characters.get_mut(&CharacterId(1)) {
        jessica.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jessica_actions(&facts(CharacterId(2), 6, 0, true), 61, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, JessicaOutcomeEvent::UpdateState { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("proof of the robber's operations")));
}

#[test]
fn jessica_quest1_do_silent_within_sixty_seconds() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jessica_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jessica) = world.characters.get_mut(&CharacterId(1)) {
        jessica.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jessica_actions(&facts(CharacterId(2), 6, 0, true), 30, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, JessicaOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
    // The seen-timer still gets stamped even when silent.
    assert!(events.contains(&JessicaOutcomeEvent::UpdateSeenTimer {
        player_id: CharacterId(2),
        value: 30,
    }));
}

#[test]
fn jessica_quest1_finish_completes_quest_and_gives_quest2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jessica_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jessica) = world.characters.get_mut(&CharacterId(1)) {
        jessica.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jessica_actions(&facts(CharacterId(2), 7, 0, true), 0, 1);
    assert!(events.contains(&JessicaOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest: QLOG_JESSICA_ROBBER_NOTE,
    }));
    assert!(events.contains(&JessicaOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 8,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("done well in damaging the robber's operations")));
}

#[test]
fn jessica_quest2_give_1_opens_kill_quest() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jessica_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jessica) = world.characters.get_mut(&CharacterId(1)) {
        jessica.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jessica_actions(&facts(CharacterId(2), 8, 0, true), 0, 1);
    assert!(events.contains(&JessicaOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: QLOG_JESSICA_KILL,
    }));
    assert!(events.contains(&JessicaOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 9,
    }));
}

#[test]
fn jessica_quest2_finish_completes_kill_quest_and_reaches_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jessica_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jessica) = world.characters.get_mut(&CharacterId(1)) {
        jessica.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jessica_actions(&facts(CharacterId(2), 11, 0, true), 0, 1);
    assert!(events.contains(&JessicaOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest: QLOG_JESSICA_KILL,
    }));
    assert!(events.contains(&JessicaOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 12,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Excellent work")));
}

#[test]
fn jessica_give_robber_note_finishes_quest1() {
    let mut world = World::default();
    let mut jessica = jessica_npc(1);
    jessica.cursor_item = Some(ItemId(50));
    world.add_character(jessica);
    let mut note = item(50, ItemFlags::empty());
    note.template_id = IID_AREA1_ROBBER2NOTE;
    note.carried_by = Some(CharacterId(1));
    world.add_item(note);
    world.add_character(player(2, "Godmode"));

    if let Some(jessica) = world.characters.get_mut(&CharacterId(1)) {
        jessica.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_jessica_actions(&facts(CharacterId(2), 6, 0, true), 0, 1);
    assert!(events.contains(&JessicaOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 7,
    }));
    assert!(world.items.is_empty());
}

#[test]
fn jessica_give_unrelated_item_hands_it_back_or_destroys_on_full_inventory() {
    let mut world = World::default();
    let mut jessica = jessica_npc(1);
    jessica.cursor_item = Some(ItemId(50));
    world.add_character(jessica);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(jessica) = world.characters.get_mut(&CharacterId(1)) {
        jessica.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_jessica_actions(&HashMap::new(), 0, 1);
    assert!(events.is_empty());

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}

#[test]
fn jessica_text_repeat_resets_quest1_state_to_give_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jessica_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(jessica) = world.characters.get_mut(&CharacterId(1)) {
        jessica.driver_state = Some(CharacterDriverState::Jessica(JessicaDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        jessica.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_jessica_actions(&facts(CharacterId(2), 4, 0, true), 0, 1);
    assert!(events.contains(&JessicaOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    assert_eq!(jessica_state(&world, CharacterId(1)).last_talk, 0);
    assert_eq!(
        jessica_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn jessica_text_repeat_resets_quest2_state_to_give_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jessica_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(jessica) = world.characters.get_mut(&CharacterId(1)) {
        jessica.driver_state = Some(CharacterDriverState::Jessica(JessicaDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        jessica.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_jessica_actions(&facts(CharacterId(2), 9, 0, true), 0, 1);
    assert!(events.contains(&JessicaOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 8,
    }));
    assert_eq!(jessica_state(&world, CharacterId(1)).last_talk, 0);
}

#[test]
fn jessica_text_ignores_non_current_victim() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jessica_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    assert!(world.spawn_character(player(3, "Other"), 12, 10));

    if let Some(jessica) = world.characters.get_mut(&CharacterId(1)) {
        jessica.driver_state = Some(CharacterDriverState::Jessica(JessicaDriverData {
            last_talk: BASELINE_TICK,
            current_victim: Some(CharacterId(2)),
        }));
        jessica.push_driver_text_message(CharacterId(3), "hello");
    }
    world.tick = Tick(BASELINE_TICK);

    let mut player_facts = facts(CharacterId(2), 4, 0, true);
    player_facts.insert(
        CharacterId(3),
        JessicaPlayerFacts {
            state: 4,
            seen_timer: 0,
            nook_quest_done: true,
        },
    );

    world.process_jessica_actions(&player_facts, 0, 1);
    assert!(world.drain_pending_area_texts().is_empty());
}
