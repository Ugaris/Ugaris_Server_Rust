use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_THAIPAN, NT_CHAR, NT_GIVE};
use crate::item_driver::{IID_ARKHATA_BUDDA, IID_ARKHATA_SCROLL2};
use crate::world::npc::area37::thaipan::{
    ThaipanDriverData, ThaipanOutcomeEvent, ThaipanPlayerFacts,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn thaipan_npc(id: u32) -> Character {
    let mut thaipan = character(id);
    thaipan.name = "Thai Pan".into();
    thaipan.driver = CDR_THAIPAN;
    thaipan.driver_state = Some(CharacterDriverState::Thaipan(ThaipanDriverData::default()));
    thaipan
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    thai_state: i32,
    pot_state: i32,
    last_budda: i32,
) -> HashMap<CharacterId, ThaipanPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        ThaipanPlayerFacts {
            thai_state,
            pot_state,
            last_budda,
        },
    );
    map
}

#[test]
fn state0_without_level_or_pot_progress_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thaipan_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thaipan) = world.characters.get_mut(&CharacterId(1)) {
        thaipan.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_thaipan_actions(&facts(CharacterId(2), 0, 0, 0), 0, 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state0_with_level_and_pot_progress_greets_opens_quest74_and_collapses_to_2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thaipan_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.level = 49;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thaipan) = world.characters.get_mut(&CharacterId(1)) {
        thaipan.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_thaipan_actions(&facts(CharacterId(2), 0, 4, 0), 0, 1);
    assert!(events.contains(&ThaipanOutcomeEvent::QuestOpen74 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&ThaipanOutcomeEvent::UpdateThaiState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I'm Thai Pan")));
}

#[test]
fn state2_speaks_and_advances_to_3() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thaipan_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thaipan) = world.characters.get_mut(&CharacterId(1)) {
        thaipan.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_thaipan_actions(&facts(CharacterId(2), 2, 4, 0), 0, 1);
    assert!(events.contains(&ThaipanOutcomeEvent::UpdateThaiState {
        player_id: CharacterId(2),
        new_state: 3,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("many stories")));
}

#[test]
fn state8_is_a_silent_no_op_waiting_for_the_scroll() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thaipan_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thaipan) = world.characters.get_mut(&CharacterId(1)) {
        thaipan.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_thaipan_actions(&facts(CharacterId(2), 8, 4, 0), 0, 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state9_is_a_silent_no_op_all_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thaipan_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thaipan) = world.characters.get_mut(&CharacterId(1)) {
        thaipan.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_thaipan_actions(&facts(CharacterId(2), 9, 4, 0), 0, 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn give_scroll_while_turn_in_window_open_completes_quest_and_sets_state9() {
    let mut world = World::default();
    let mut thaipan = thaipan_npc(1);
    thaipan.cursor_item = Some(ItemId(50));
    world.add_character(thaipan);
    let mut scroll = item(50, ItemFlags::empty());
    scroll.name = "a red scroll".into();
    scroll.template_id = IID_ARKHATA_SCROLL2;
    scroll.carried_by = Some(CharacterId(1));
    world.add_item(scroll);
    world.add_character(player(2, "Godmode"));

    if let Some(thaipan) = world.characters.get_mut(&CharacterId(1)) {
        thaipan.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_thaipan_actions(&facts(CharacterId(2), 8, 4, 0), 0, 1);
    assert!(events.contains(&ThaipanOutcomeEvent::QuestDone74 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&ThaipanOutcomeEvent::UpdateThaiState {
        player_id: CharacterId(2),
        new_state: 9,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("the story is true")));
    assert!(!world.items.contains_key(&ItemId(50)));
}

#[test]
fn give_scroll_outside_turn_in_window_is_handed_back() {
    let mut world = World::default();
    let mut thaipan = thaipan_npc(1);
    thaipan.cursor_item = Some(ItemId(50));
    world.add_character(thaipan);
    let mut scroll = item(50, ItemFlags::empty());
    scroll.name = "a red scroll".into();
    scroll.template_id = IID_ARKHATA_SCROLL2;
    scroll.carried_by = Some(CharacterId(1));
    world.add_item(scroll);
    world.add_character(player(2, "Godmode"));

    if let Some(thaipan) = world.characters.get_mut(&CharacterId(1)) {
        thaipan.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // state 9 ("all done"): outside the `1..=8` turn-in window.
    let events = world.process_thaipan_actions(&facts(CharacterId(2), 9, 4, 0), 0, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, ThaipanOutcomeEvent::QuestDone74 { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn give_budda_with_negative_exp_and_cooldown_elapsed_grants_exp_and_stamps_cooldown() {
    let mut world = World::default();
    let mut thaipan = thaipan_npc(1);
    thaipan.cursor_item = Some(ItemId(50));
    world.add_character(thaipan);
    let mut statue = item(50, ItemFlags::empty());
    statue.name = "a buddah statue".into();
    statue.template_id = IID_ARKHATA_BUDDA;
    statue.carried_by = Some(CharacterId(1));
    world.add_item(statue);
    let mut godmode = player(2, "Godmode");
    godmode.exp = 100;
    godmode.exp_used = 300;
    world.add_character(godmode);

    if let Some(thaipan) = world.characters.get_mut(&CharacterId(1)) {
        thaipan.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // `now` far beyond the 24h cooldown from `last_budda == 0`.
    let now = 60 * 60 * 24 + 1;
    let events = world.process_thaipan_actions(&facts(CharacterId(2), 3, 4, 0), now, 1);
    assert!(events.contains(&ThaipanOutcomeEvent::UpdateLastBudda {
        player_id: CharacterId(2),
        realtime_seconds: now,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("find peace")));
    // `v = min(exp_used - exp, exp_used/200) = min(200, 1) = 1`.
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.exp, 101);
    assert!(!world.items.contains_key(&ItemId(50)));
}

#[test]
fn give_budda_without_negative_exp_says_no_negative_experience() {
    let mut world = World::default();
    let mut thaipan = thaipan_npc(1);
    thaipan.cursor_item = Some(ItemId(50));
    world.add_character(thaipan);
    let mut statue = item(50, ItemFlags::empty());
    statue.name = "a buddah statue".into();
    statue.template_id = IID_ARKHATA_BUDDA;
    statue.carried_by = Some(CharacterId(1));
    world.add_item(statue);
    let mut godmode = player(2, "Godmode");
    godmode.exp = 300;
    godmode.exp_used = 100;
    world.add_character(godmode);

    if let Some(thaipan) = world.characters.get_mut(&CharacterId(1)) {
        thaipan.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_thaipan_actions(&facts(CharacterId(2), 3, 4, 0), 100_000, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, ThaipanOutcomeEvent::UpdateLastBudda { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("not have any negative experience")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn give_budda_within_cooldown_says_once_per_day() {
    let mut world = World::default();
    let mut thaipan = thaipan_npc(1);
    thaipan.cursor_item = Some(ItemId(50));
    world.add_character(thaipan);
    let mut statue = item(50, ItemFlags::empty());
    statue.name = "a buddah statue".into();
    statue.template_id = IID_ARKHATA_BUDDA;
    statue.carried_by = Some(CharacterId(1));
    world.add_item(statue);
    let mut godmode = player(2, "Godmode");
    godmode.exp = 100;
    godmode.exp_used = 300;
    world.add_character(godmode);

    if let Some(thaipan) = world.characters.get_mut(&CharacterId(1)) {
        thaipan.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // `now` still within the 24h cooldown from `last_budda == 1000`.
    let events = world.process_thaipan_actions(&facts(CharacterId(2), 3, 4, 1000), 1500, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, ThaipanOutcomeEvent::UpdateLastBudda { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("once per day")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn give_unrelated_item_is_handed_back() {
    let mut world = World::default();
    let mut thaipan = thaipan_npc(1);
    thaipan.cursor_item = Some(ItemId(50));
    world.add_character(thaipan);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(thaipan) = world.characters.get_mut(&CharacterId(1)) {
        thaipan.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_thaipan_actions(&HashMap::new(), 0, 1);
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
    assert!(world.spawn_character(thaipan_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(thaipan) = world.characters.get_mut(&CharacterId(1)) {
        thaipan.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_thaipan_actions(&facts(CharacterId(2), 3, 4, 0), 0, 1);
    assert!(events.contains(&ThaipanOutcomeEvent::UpdateThaiState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
}

#[test]
fn text_repeat_is_a_no_op_outside_turn_in_window() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thaipan_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(thaipan) = world.characters.get_mut(&CharacterId(1)) {
        thaipan.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_thaipan_actions(&facts(CharacterId(2), 0, 0, 0), 0, 1);
    assert!(events.is_empty());
}
