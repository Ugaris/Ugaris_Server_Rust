use std::collections::HashMap;

use super::*;
use crate::character_driver::{JiuDriverData, CDR_JIU, NT_CHAR, NT_GIVE};
use crate::world::jiu::{JiuOutcomeEvent, JiuPlayerFacts};

/// Same rationale as `world::yoakin`'s own `BASELINE_TICK`: keeps the
/// default `last_talk = 0` well clear of `JIU_TALK_MIN_TICKS`'s throttle
/// window so these boot-time tests don't get silently swallowed by it.
const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn jiu_npc(id: u32) -> Character {
    let mut jiu = character(id);
    jiu.name = "Jiu".into();
    jiu.driver = CDR_JIU;
    jiu.driver_state = Some(CharacterDriverState::Jiu(JiuDriverData::default()));
    jiu
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(player_id: CharacterId, state: i32) -> HashMap<CharacterId, JiuPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, JiuPlayerFacts { state });
    map
}

fn jiu_state(world: &World, jiu_id: CharacterId) -> JiuDriverData {
    match world
        .characters
        .get(&jiu_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Jiu(data)) => data,
        _ => panic!("expected jiu driver state"),
    }
}

#[test]
fn jiu_entry_greets_high_level_player_and_advances_to_story1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jiu_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.level = 39;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jiu) = world.characters.get_mut(&CharacterId(1)) {
        jiu.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jiu_actions(&facts(CharacterId(2), 0), 1_000, 1);
    assert!(events.contains(&JiuOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    assert!(events.contains(&JiuOutcomeEvent::UpdateSeenTimer {
        player_id: CharacterId(2),
        value: 1_000,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("difficult impediment")));
    assert_eq!(
        jiu_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn jiu_entry_greets_low_level_player_without_advancing() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jiu_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.level = 10;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jiu) = world.characters.get_mut(&CharacterId(1)) {
        jiu.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jiu_actions(&facts(CharacterId(2), 0), 0, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, JiuOutcomeEvent::UpdateState { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hail thee, traveler.")));
}

#[test]
fn jiu_story1_opens_quest_and_advances_to_wait_for_kill() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jiu_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jiu) = world.characters.get_mut(&CharacterId(1)) {
        jiu.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jiu_actions(&facts(CharacterId(2), 1), 0, 1);
    assert!(events.contains(&JiuOutcomeEvent::QuestOpen {
        player_id: CharacterId(2)
    }));
    assert!(events.contains(&JiuOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("kill the beast")));
}

#[test]
fn jiu_wait_for_kill_is_silent() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jiu_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jiu) = world.characters.get_mut(&CharacterId(1)) {
        jiu.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jiu_actions(&facts(CharacterId(2), 2), 0, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, JiuOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
    // The seen-timer still gets stamped even when silent.
    assert!(events.contains(&JiuOutcomeEvent::UpdateSeenTimer {
        player_id: CharacterId(2),
        value: 0,
    }));
}

#[test]
fn jiu_beast_killed_thanks_player_and_completes_quest() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jiu_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jiu) = world.characters.get_mut(&CharacterId(1)) {
        jiu.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jiu_actions(&facts(CharacterId(2), 3), 0, 1);
    assert!(events.contains(&JiuOutcomeEvent::QuestDone {
        player_id: CharacterId(2)
    }));
    assert!(events.contains(&JiuOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 4,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("honourable fighter")));
}

#[test]
fn jiu_done_is_silent_on_char_message() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jiu_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(jiu) = world.characters.get_mut(&CharacterId(1)) {
        jiu.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jiu_actions(&facts(CharacterId(2), 4), 0, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, JiuOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn jiu_text_repeat_resets_state_when_quest_not_yet_beaten() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jiu_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(jiu) = world.characters.get_mut(&CharacterId(1)) {
        jiu.driver_state = Some(CharacterDriverState::Jiu(JiuDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        jiu.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_jiu_actions(&facts(CharacterId(2), 2), 0, 1);
    assert!(events.contains(&JiuOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 0,
    }));
    assert_eq!(jiu_state(&world, CharacterId(1)).last_talk, 0);
    assert_eq!(
        jiu_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn jiu_text_repeat_after_quest_done_says_no_more_requests() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jiu_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(jiu) = world.characters.get_mut(&CharacterId(1)) {
        jiu.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_jiu_actions(&facts(CharacterId(2), 4), 0, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, JiuOutcomeEvent::UpdateState { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("no more requests for thee")));
}

#[test]
fn jiu_text_ignores_non_current_victim() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jiu_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    assert!(world.spawn_character(player(3, "Other"), 12, 10));

    if let Some(jiu) = world.characters.get_mut(&CharacterId(1)) {
        jiu.driver_state = Some(CharacterDriverState::Jiu(JiuDriverData {
            last_talk: TICKS_PER_SECOND * 1000,
            current_victim: Some(CharacterId(2)),
        }));
        jiu.push_driver_text_message(CharacterId(3), "hello");
    }
    world.tick = Tick(TICKS_PER_SECOND * 1000);

    let mut player_facts = facts(CharacterId(2), 4);
    player_facts.insert(CharacterId(3), JiuPlayerFacts { state: 4 });

    world.process_jiu_actions(&player_facts, 0, 1);
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn jiu_give_unrelated_item_hands_it_to_giver() {
    let mut world = World::default();
    let mut jiu = jiu_npc(1);
    jiu.cursor_item = Some(ItemId(50));
    world.add_character(jiu);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(jiu) = world.characters.get_mut(&CharacterId(1)) {
        jiu.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_jiu_actions(&HashMap::new(), 0, 1);
    assert!(events.is_empty());

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.inventory[30], Some(ItemId(50)));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}
