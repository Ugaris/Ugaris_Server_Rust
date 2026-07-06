use std::collections::HashMap;

use super::*;
use crate::character_driver::{CamhermitDriverData, CDR_CAMHERMIT, NT_CHAR, NT_GIVE};
use crate::item_driver::IID_AREA1_SMALL_BEAR_TEETH;
use crate::quest::{
    CAMHERMIT_STATE_DONE, CAMHERMIT_STATE_ENTRY, CAMHERMIT_STATE_QUEST1DO,
    CAMHERMIT_STATE_QUEST1WAIT, CAMHERMIT_STATE_QUEST1_1, CAMHERMIT_STATE_QUEST2DO,
    CAMHERMIT_STATE_QUEST2WAIT, QLOG_HERMIT_QUEST1, QLOG_HERMIT_QUEST2,
};
use crate::world::camhermit::{CamhermitOutcomeEvent, CamhermitPlayerFacts};

/// C's `dat->current_victim != co` guard (`gwendylon.c:735-738`) is a
/// plain `!=` against `0`, not a truthy-gated check, so a genuinely fresh
/// NPC (`current_victim == 0`) refuses to greet anyone at all until the
/// global tick counter passes `TICKS*20` - a real but harmless boot-time-
/// only quirk (`ticker` is already enormous by the time any player can
/// log in on a live server). Tests use a baseline tick comfortably past
/// that window, matching real server uptime.
const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn camhermit_npc(id: u32) -> Character {
    let mut hermit = character(id);
    hermit.name = "Hermit".into();
    hermit.driver = CDR_CAMHERMIT;
    hermit.driver_state = Some(CharacterDriverState::Camhermit(
        CamhermitDriverData::default(),
    ));
    hermit
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
    kills: i32,
    quest2_done_count: u8,
) -> HashMap<CharacterId, CamhermitPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        CamhermitPlayerFacts {
            state,
            seen_timer,
            kills,
            quest2_done_count,
        },
    );
    map
}

fn camhermit_state(world: &World, hermit_id: CharacterId) -> CamhermitDriverData {
    match world
        .characters
        .get(&hermit_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Camhermit(data)) => data,
        _ => panic!("expected camhermit driver state"),
    }
}

#[test]
fn camhermit_entry_greets_and_advances_to_quest1wait() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(camhermit_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(hermit) = world.characters.get_mut(&CharacterId(1)) {
        hermit.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_camhermit_actions(
        &facts(CharacterId(2), CAMHERMIT_STATE_ENTRY, 0, 0, 0),
        1_000,
        1,
    );
    assert!(events.contains(&CamhermitOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: CAMHERMIT_STATE_QUEST1WAIT,
    }));
    assert!(events.contains(&CamhermitOutcomeEvent::UpdateSeenTimer {
        player_id: CharacterId(2),
        value: 1_000,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Who enters my domain")));
    assert_eq!(
        camhermit_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn camhermit_quest1wait_advances_silently_once_level_is_high_enough() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(camhermit_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.level = 9;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(hermit) = world.characters.get_mut(&CharacterId(1)) {
        hermit.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_camhermit_actions(
        &facts(CharacterId(2), CAMHERMIT_STATE_QUEST1WAIT, 0, 0, 0),
        0,
        1,
    );
    assert!(events.contains(&CamhermitOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: CAMHERMIT_STATE_QUEST1_1,
    }));
    // No dialogue for this transition (C has no `quiet_say` in this
    // branch), so `current_victim`/`last_talk` stay untouched.
    assert!(world.drain_pending_area_texts().is_empty());
    assert_eq!(camhermit_state(&world, CharacterId(1)).current_victim, None);
}

#[test]
fn camhermit_quest1wait_stays_put_below_level_9() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(camhermit_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(hermit) = world.characters.get_mut(&CharacterId(1)) {
        hermit.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_camhermit_actions(
        &facts(CharacterId(2), CAMHERMIT_STATE_QUEST1WAIT, 0, 0, 0),
        0,
        1,
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, CamhermitOutcomeEvent::UpdateState { .. })));
}

#[test]
fn camhermit_quest1do_completes_quest_once_ten_bears_are_killed() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(camhermit_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(hermit) = world.characters.get_mut(&CharacterId(1)) {
        hermit.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_camhermit_actions(
        &facts(CharacterId(2), CAMHERMIT_STATE_QUEST1DO, 0, 10, 0),
        0,
        1,
    );
    assert!(events.contains(&CamhermitOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest: QLOG_HERMIT_QUEST1,
    }));
    assert!(events.contains(&CamhermitOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: CAMHERMIT_STATE_QUEST2WAIT,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("brought some fear")));
}

#[test]
fn camhermit_quest1do_reminds_after_sixty_seconds_without_enough_kills() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(camhermit_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(hermit) = world.characters.get_mut(&CharacterId(1)) {
        hermit.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // `seen_timer = 0`, `now = 61` -> more than 60 seconds since last seen.
    let events = world.process_camhermit_actions(
        &facts(CharacterId(2), CAMHERMIT_STATE_QUEST1DO, 0, 3, 0),
        61,
        1,
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, CamhermitOutcomeEvent::UpdateState { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Didst thou understand")));
}

#[test]
fn camhermit_quest2do_rewards_gold_and_completes_quest_for_ten_teeth() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(camhermit_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    for slot in 30..40 {
        godmode.inventory[slot] = Some(ItemId(100 + slot as u32));
    }
    assert!(world.spawn_character(godmode, 12, 10));
    for slot in 30..40 {
        let item_id = ItemId(100 + slot as u32);
        let mut teeth = item(item_id.0, ItemFlags::empty());
        teeth.template_id = IID_AREA1_SMALL_BEAR_TEETH;
        teeth.carried_by = Some(CharacterId(2));
        world.add_item(teeth);
    }

    world.tick = Tick(BASELINE_TICK);
    if let Some(hermit) = world.characters.get_mut(&CharacterId(1)) {
        hermit.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_camhermit_actions(
        &facts(CharacterId(2), CAMHERMIT_STATE_QUEST2DO, 0, 0, 0),
        0,
        1,
    );
    assert!(events.contains(&CamhermitOutcomeEvent::GoldEarned {
        player_id: CharacterId(2),
        amount: 1_500,
    }));
    assert!(events.contains(&CamhermitOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest: QLOG_HERMIT_QUEST2,
    }));
    assert!(events.contains(&CamhermitOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: CAMHERMIT_STATE_DONE,
    }));

    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.gold, 1_500);
    assert!(godmode.inventory[30..40].iter().all(|slot| slot.is_none()));
    assert!(world.items.is_empty());
}

#[test]
fn camhermit_quest2do_reminds_without_enough_teeth() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(camhermit_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(hermit) = world.characters.get_mut(&CharacterId(1)) {
        hermit.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_camhermit_actions(
        &facts(CharacterId(2), CAMHERMIT_STATE_QUEST2DO, 0, 0, 0),
        0,
        1,
    );
    // C still transitions to `CAMHERMIT_STATE_QUEST2DO_WAIT` even without
    // enough teeth (`gwendylon.c:889-893`) - only the gold/quest-done
    // events are absent.
    assert!(events.contains(&CamhermitOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: crate::quest::CAMHERMIT_STATE_QUEST2DO_WAIT,
    }));
    assert!(!events
        .iter()
        .any(|event| matches!(event, CamhermitOutcomeEvent::GoldEarned { .. })));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.gold, 0);
}

#[test]
fn camhermit_text_repeat_resets_quest1do_state_and_zeroes_last_talk() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(camhermit_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(hermit) = world.characters.get_mut(&CharacterId(1)) {
        hermit.driver_state = Some(CharacterDriverState::Camhermit(CamhermitDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        hermit.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_camhermit_actions(
        &facts(CharacterId(2), CAMHERMIT_STATE_QUEST1DO, 0, 0, 0),
        0,
        1,
    );
    assert!(events.contains(&CamhermitOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: CAMHERMIT_STATE_QUEST1_1,
    }));
    assert_eq!(camhermit_state(&world, CharacterId(1)).last_talk, 0);
    assert_eq!(
        camhermit_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn camhermit_give_message_places_item_in_giver_inventory() {
    let mut world = World::default();
    let mut hermit = camhermit_npc(1);
    hermit.cursor_item = Some(ItemId(50));
    world.add_character(hermit);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(hermit) = world.characters.get_mut(&CharacterId(1)) {
        hermit.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_camhermit_actions(&HashMap::new(), 0, 1);
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
