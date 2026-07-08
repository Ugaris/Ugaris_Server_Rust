use std::collections::HashMap;

use super::*;
use crate::character_driver::{EngraveDriverData, KassimDriverData, CDR_KASSIM, NT_CHAR, NT_GIVE};
use crate::world::kassim::{KassimOutcomeEvent, KassimPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;
const KASSIM_GOLD_NEEDED_TO_ENGRAVE: u32 = 500 * 100;

fn kassim_npc(id: u32) -> Character {
    let mut kassim = character(id);
    kassim.name = "Kassim".into();
    kassim.driver = CDR_KASSIM;
    kassim.driver_state = Some(CharacterDriverState::Kassim(KassimDriverData::default()));
    kassim
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    kassim_state: i32,
    kassim_seen_timer: i32,
    kassim_item_wait_starttime: i32,
) -> HashMap<CharacterId, KassimPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        KassimPlayerFacts {
            kassim_state,
            kassim_seen_timer,
            kassim_item_wait_starttime,
        },
    );
    map
}

fn kassim_state(world: &World, kassim_id: CharacterId) -> KassimDriverData {
    match world
        .characters
        .get(&kassim_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Kassim(data)) => data,
        _ => panic!("expected kassim driver state"),
    }
}

#[test]
fn kassim_greets_player_when_repeat_entry_window_elapsed() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(kassim_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(kassim) = world.characters.get_mut(&CharacterId(1)) {
        kassim.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // now (1000) - seen_timer (0) > 120, so the greeting fires.
    let events = world.process_kassim_actions(&facts(CharacterId(2), 0, 0, 0), 1000, 1);
    assert!(events.contains(&KassimOutcomeEvent::UpdateSeenTimer {
        player_id: CharacterId(2),
        value: 1000,
    }));
    let texts = world.drain_pending_area_text_bytes();
    assert!(texts
        .iter()
        .any(|text| String::from_utf8_lossy(&text.message).contains("welcome to my humble abode")));
    assert_eq!(
        kassim_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn kassim_suppresses_repeat_greeting_within_time_window() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(kassim_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(kassim) = world.characters.get_mut(&CharacterId(1)) {
        kassim.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // now (100) - seen_timer (50) == 50, well under the 120s threshold.
    let events = world.process_kassim_actions(&facts(CharacterId(2), 0, 50, 0), 100, 1);
    // The seen-timer refresh is unconditional even when no greeting fires.
    assert!(events.contains(&KassimOutcomeEvent::UpdateSeenTimer {
        player_id: CharacterId(2),
        value: 100,
    }));
    assert!(world.drain_pending_area_texts().is_empty());
    // No `didsay`, so `current_victim` stays untouched.
    assert_eq!(kassim_state(&world, CharacterId(1)).current_victim, None);
}

#[test]
fn kassim_text_engrave_command_stashes_inscription_and_advances_state() {
    let mut world = World::default();
    assert!(world.spawn_character(kassim_npc(1), 10, 10));
    world.add_character(player(2, "Godmode"));

    if let Some(kassim) = world.characters.get_mut(&CharacterId(1)) {
        kassim.push_driver_text_message(CharacterId(2), "engrave: For Ugaris");
    }

    let events = world.process_kassim_actions(&facts(CharacterId(2), 0, 0, 0), 1000, 1);
    assert!(events.contains(&KassimOutcomeEvent::UpdateKassimState {
        player_id: CharacterId(2),
        new_state: 2, // KASSIM_STATE_ENGRAVE_TEXT
    }));
    match world
        .characters
        .get(&CharacterId(2))
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Engrave(data)) => assert_eq!(data.text, "For Ugaris"),
        other => panic!("expected engrave driver state, got {other:?}"),
    }
    // The `engrave:` branch never sets `didsay` in C - Kassim doesn't
    // remember the speaker as `current_victim` just from this command.
    assert_eq!(kassim_state(&world, CharacterId(1)).current_victim, None);
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn kassim_char_asks_for_item_when_inscription_is_long_enough() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(kassim_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    if let Some(player) = world.characters.get_mut(&CharacterId(2)) {
        player.driver_state = Some(CharacterDriverState::Engrave(EngraveDriverData {
            text: "Ugaris".into(),
        }));
    }

    world.tick = Tick(BASELINE_TICK);
    if let Some(kassim) = world.characters.get_mut(&CharacterId(1)) {
        kassim.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_kassim_actions(&facts(CharacterId(2), 2, 0, 0), 1000, 1);
    assert!(events.contains(&KassimOutcomeEvent::UpdateKassimState {
        player_id: CharacterId(2),
        new_state: 3, // KASSIM_STATE_ITEM_WAIT
    }));
    assert!(events.contains(&KassimOutcomeEvent::UpdateItemWaitStart {
        player_id: CharacterId(2),
        value: 1000,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hand me the item")));
}

#[test]
fn kassim_char_rejects_too_short_inscription() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(kassim_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    if let Some(player) = world.characters.get_mut(&CharacterId(2)) {
        player.driver_state = Some(CharacterDriverState::Engrave(EngraveDriverData {
            text: "Hi".into(),
        }));
    }

    world.tick = Tick(BASELINE_TICK);
    if let Some(kassim) = world.characters.get_mut(&CharacterId(1)) {
        kassim.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_kassim_actions(&facts(CharacterId(2), 2, 0, 0), 1000, 1);
    assert!(events.contains(&KassimOutcomeEvent::UpdateKassimState {
        player_id: CharacterId(2),
        new_state: 0, // KASSIM_STATE_ENTRY
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("at least count three letters")));
}

#[test]
fn kassim_give_engraves_wearable_item_and_charges_gold() {
    let mut world = World::default();
    let mut kassim = kassim_npc(1);
    kassim.cursor_item = Some(ItemId(50));
    world.add_character(kassim);
    let mut ring = item(50, ItemFlags::WNRRING);
    ring.name = "Ring".into();
    ring.carried_by = Some(CharacterId(1));
    world.add_item(ring);

    let mut giver = player(2, "Godmode");
    giver.gold = 100_000;
    giver.driver_state = Some(CharacterDriverState::Engrave(EngraveDriverData {
        text: "Hero of Ugaris".into(),
    }));
    world.add_character(giver);

    if let Some(kassim) = world.characters.get_mut(&CharacterId(1)) {
        kassim.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_kassim_actions(&facts(CharacterId(2), 3, 0, 0), 1000, 1);
    assert!(events.contains(&KassimOutcomeEvent::UpdateKassimState {
        player_id: CharacterId(2),
        new_state: 6, // KASSIM_STATE_ENGRAVE
    }));

    let ring = world.items.get(&ItemId(50)).unwrap();
    assert_eq!(ring.description, "Hero of Ugaris");
    assert!(ring.flags.contains(ItemFlags::ENGRAVED));

    let giver = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(giver.gold, 100_000 - KASSIM_GOLD_NEEDED_TO_ENGRAVE);
    assert_eq!(giver.cursor_item, Some(ItemId(50)));

    // The successful-engrave branch never sets `didsay` in C.
    assert_eq!(kassim_state(&world, CharacterId(1)).current_victim, None);
}

#[test]
fn kassim_give_insufficient_gold_returns_item_without_charging() {
    let mut world = World::default();
    let mut kassim = kassim_npc(1);
    kassim.cursor_item = Some(ItemId(50));
    world.add_character(kassim);
    let mut ring = item(50, ItemFlags::WNRRING);
    ring.carried_by = Some(CharacterId(1));
    world.add_item(ring);

    let mut giver = player(2, "Godmode");
    giver.gold = 100;
    giver.driver_state = Some(CharacterDriverState::Engrave(EngraveDriverData {
        text: "Hero".into(),
    }));
    world.add_character(giver);

    if let Some(kassim) = world.characters.get_mut(&CharacterId(1)) {
        kassim.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_kassim_actions(&facts(CharacterId(2), 3, 0, 0), 1000, 1);
    assert!(events.contains(&KassimOutcomeEvent::UpdateKassimState {
        player_id: CharacterId(2),
        new_state: 0, // KASSIM_STATE_ENTRY
    }));

    let giver = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(giver.gold, 100);
    assert_eq!(giver.cursor_item, Some(ItemId(50)));
    let ring = world.items.get(&ItemId(50)).unwrap();
    assert!(!ring.flags.contains(ItemFlags::ENGRAVED));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("cannot pay for mine services")));
}

#[test]
fn kassim_give_rejects_non_wearable_item() {
    let mut world = World::default();
    let mut kassim = kassim_npc(1);
    kassim.cursor_item = Some(ItemId(50));
    world.add_character(kassim);
    let mut rock = item(50, ItemFlags::empty());
    rock.carried_by = Some(CharacterId(1));
    world.add_item(rock);

    let mut giver = player(2, "Godmode");
    giver.driver_state = Some(CharacterDriverState::Engrave(EngraveDriverData {
        text: "Hero".into(),
    }));
    world.add_character(giver);

    if let Some(kassim) = world.characters.get_mut(&CharacterId(1)) {
        kassim.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_kassim_actions(&facts(CharacterId(2), 3, 0, 0), 1000, 1);
    assert!(events.contains(&KassimOutcomeEvent::UpdateKassimState {
        player_id: CharacterId(2),
        new_state: 0, // KASSIM_STATE_ENTRY
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Only wearable items")));
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().cursor_item,
        Some(ItemId(50))
    );
    assert_eq!(
        kassim_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn kassim_give_rejects_already_engraved_item() {
    let mut world = World::default();
    let mut kassim = kassim_npc(1);
    kassim.cursor_item = Some(ItemId(50));
    world.add_character(kassim);
    let mut ring = item(50, ItemFlags::WNRRING | ItemFlags::ENGRAVED);
    ring.carried_by = Some(CharacterId(1));
    world.add_item(ring);

    let mut giver = player(2, "Godmode");
    giver.driver_state = Some(CharacterDriverState::Engrave(EngraveDriverData {
        text: "Hero".into(),
    }));
    world.add_character(giver);

    if let Some(kassim) = world.characters.get_mut(&CharacterId(1)) {
        kassim.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_kassim_actions(&facts(CharacterId(2), 3, 0, 0), 1000, 1);
    assert!(events.contains(&KassimOutcomeEvent::UpdateKassimState {
        player_id: CharacterId(2),
        new_state: 0, // KASSIM_STATE_ENTRY
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("already has an engraving")));
}

#[test]
fn kassim_give_wrong_state_hands_item_back_silently() {
    let mut world = World::default();
    let mut kassim = kassim_npc(1);
    kassim.cursor_item = Some(ItemId(50));
    world.add_character(kassim);
    let mut ring = item(50, ItemFlags::WNRRING);
    ring.carried_by = Some(CharacterId(1));
    world.add_item(ring);
    world.add_character(player(2, "Godmode"));

    if let Some(kassim) = world.characters.get_mut(&CharacterId(1)) {
        kassim.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // kassim_state == 0 (ENTRY), not ITEM_WAIT.
    let events = world.process_kassim_actions(&facts(CharacterId(2), 0, 0, 0), 1000, 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().cursor_item,
        Some(ItemId(50))
    );
    assert_eq!(kassim_state(&world, CharacterId(1)).current_victim, None);
}
