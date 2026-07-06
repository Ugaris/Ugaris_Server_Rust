use std::collections::HashMap;

use super::*;
use crate::character_driver::{YoakinDriverData, CDR_YOAKIN, NT_CHAR, NT_GIVE};
use crate::item_driver::{IID_AREA1_BIGBEAR_TOOTH, IID_SHRIKE_TALISMAN};
use crate::world::yoakin::{YoakinOutcomeEvent, YoakinPlayerFacts};

/// Same rationale as `world::camhermit`'s own `BASELINE_TICK` (its
/// module's C source, `gwendylon.c`, shares the same `dat->current_victim
/// != co` boot-time-only quirk).
const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn yoakin_npc(id: u32) -> Character {
    let mut yoakin = character(id);
    yoakin.name = "Yoakin".into();
    yoakin.driver = CDR_YOAKIN;
    yoakin.driver_state = Some(CharacterDriverState::Yoakin(YoakinDriverData::default()));
    yoakin
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
    logain_state: i32,
    quest_done_count: u8,
) -> HashMap<CharacterId, YoakinPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        YoakinPlayerFacts {
            state,
            seen_timer,
            logain_state,
            quest_done_count,
            shrike_state: 0,
            shrike_fails: 0,
            level: 1,
        },
    );
    map
}

fn yoakin_state(world: &World, yoakin_id: CharacterId) -> YoakinDriverData {
    match world
        .characters
        .get(&yoakin_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Yoakin(data)) => data,
        _ => panic!("expected yoakin driver state"),
    }
}

#[test]
fn yoakin_entry_greets_opens_quest_and_advances_to_warned() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(yoakin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(yoakin) = world.characters.get_mut(&CharacterId(1)) {
        yoakin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_yoakin_actions(&facts(CharacterId(2), 0, 0, 0, 0), 1_000, 1);
    assert!(events.contains(&YoakinOutcomeEvent::QuestOpen {
        player_id: CharacterId(2)
    }));
    assert!(events.contains(&YoakinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    assert!(events.contains(&YoakinOutcomeEvent::UpdateSeenTimer {
        player_id: CharacterId(2),
        value: 1_000,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I am Yoakin, the hunter")));
    assert_eq!(
        yoakin_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn yoakin_state2_stays_put_below_logain_state_6() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(yoakin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(yoakin) = world.characters.get_mut(&CharacterId(1)) {
        yoakin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_yoakin_actions(&facts(CharacterId(2), 2, 0, 5, 0), 0, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, YoakinOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn yoakin_state2_advances_once_logain_state_reaches_6() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(yoakin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(yoakin) = world.characters.get_mut(&CharacterId(1)) {
        yoakin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_yoakin_actions(&facts(CharacterId(2), 2, 0, 6, 0), 0, 1);
    assert!(events.contains(&YoakinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 3,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("huge mother bear")));
}

#[test]
fn yoakin_state_reset_after_120_seconds_below_state4() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(yoakin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(yoakin) = world.characters.get_mut(&CharacterId(1)) {
        yoakin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // state=3, seen_timer=0, now=121 -> resets to state 1 first, then
    // state-1's own dialogue fires this same tick (matches C: the reset
    // happens before the `switch`, so the fresh state 1 branch runs).
    let events = world.process_yoakin_actions(&facts(CharacterId(2), 3, 0, 6, 0), 121, 1);
    assert!(events.contains(&YoakinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Be careful in the forest")));
}

#[test]
fn yoakin_state4_reminds_after_sixty_seconds() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(yoakin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(yoakin) = world.characters.get_mut(&CharacterId(1)) {
        yoakin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_yoakin_actions(&facts(CharacterId(2), 4, 0, 6, 0), 61, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, YoakinOutcomeEvent::UpdateState { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Didst thou find that big mother bear")));
}

#[test]
fn yoakin_state4_silent_within_sixty_seconds() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(yoakin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(yoakin) = world.characters.get_mut(&CharacterId(1)) {
        yoakin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_yoakin_actions(&facts(CharacterId(2), 4, 0, 6, 0), 30, 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, YoakinOutcomeEvent::UpdateState { .. })));
    assert!(world.drain_pending_area_texts().is_empty());
    // The seen-timer still gets stamped even when silent.
    assert!(events.contains(&YoakinOutcomeEvent::UpdateSeenTimer {
        player_id: CharacterId(2),
        value: 30,
    }));
}

#[test]
fn yoakin_give_bigbear_tooth_completes_quest_and_rewards_gold_first_time() {
    let mut world = World::default();
    let mut yoakin = yoakin_npc(1);
    yoakin.cursor_item = Some(ItemId(50));
    world.add_character(yoakin);
    let mut tooth = item(50, ItemFlags::empty());
    tooth.template_id = IID_AREA1_BIGBEAR_TOOTH;
    tooth.carried_by = Some(CharacterId(1));
    world.add_item(tooth);
    world.add_character(player(2, "Godmode"));

    if let Some(yoakin) = world.characters.get_mut(&CharacterId(1)) {
        yoakin.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_yoakin_actions(&facts(CharacterId(2), 4, 0, 6, 0), 0, 1);
    assert!(events.contains(&YoakinOutcomeEvent::QuestDone {
        player_id: CharacterId(2)
    }));
    assert!(events.contains(&YoakinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 5,
    }));
    assert!(events.contains(&YoakinOutcomeEvent::GoldEarned {
        player_id: CharacterId(2),
        amount: 500,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("The forest will be safer now")));

    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.gold, 500);
    assert!(world.items.is_empty());
}

#[test]
fn yoakin_give_bigbear_tooth_second_time_skips_gold_reward() {
    let mut world = World::default();
    let mut yoakin = yoakin_npc(1);
    yoakin.cursor_item = Some(ItemId(50));
    world.add_character(yoakin);
    let mut tooth = item(50, ItemFlags::empty());
    tooth.template_id = IID_AREA1_BIGBEAR_TOOTH;
    tooth.carried_by = Some(CharacterId(1));
    world.add_item(tooth);
    world.add_character(player(2, "Godmode"));

    if let Some(yoakin) = world.characters.get_mut(&CharacterId(1)) {
        yoakin.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // `quest_done_count = 1`: this player already completed "Bear Hunt"
    // once before (C's `tmp` would come back `2`, not `1`).
    let events = world.process_yoakin_actions(&facts(CharacterId(2), 4, 0, 6, 1), 0, 1);
    assert!(events.contains(&YoakinOutcomeEvent::QuestDone {
        player_id: CharacterId(2)
    }));
    assert!(!events
        .iter()
        .any(|event| matches!(event, YoakinOutcomeEvent::GoldEarned { .. })));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.gold, 0);
}

#[test]
fn yoakin_give_bigbear_tooth_sweeps_every_stray_tooth_in_inventory() {
    let mut world = World::default();
    let mut yoakin = yoakin_npc(1);
    yoakin.cursor_item = Some(ItemId(50));
    world.add_character(yoakin);
    let mut tooth = item(50, ItemFlags::empty());
    tooth.template_id = IID_AREA1_BIGBEAR_TOOTH;
    tooth.carried_by = Some(CharacterId(1));
    world.add_item(tooth);

    let mut godmode = player(2, "Godmode");
    godmode.inventory[30] = Some(ItemId(51));
    world.add_character(godmode);
    let mut stray_tooth = item(51, ItemFlags::empty());
    stray_tooth.template_id = IID_AREA1_BIGBEAR_TOOTH;
    stray_tooth.carried_by = Some(CharacterId(2));
    world.add_item(stray_tooth);

    if let Some(yoakin) = world.characters.get_mut(&CharacterId(1)) {
        yoakin.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    world.process_yoakin_actions(&facts(CharacterId(2), 4, 0, 6, 0), 0, 1);

    // C `destroy_item_byID` sweeps every matching item in inventory, not
    // just the one that was handed over (a genuine C quirk, preserved).
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert!(godmode.inventory[30].is_none());
    assert!(world.items.is_empty());
}

#[test]
fn yoakin_give_shrike_talisman_grants_full_exp_without_prior_fails() {
    let mut world = World::default();
    let mut yoakin = yoakin_npc(1);
    yoakin.cursor_item = Some(ItemId(50));
    world.add_character(yoakin);
    let mut talisman = item(50, ItemFlags::empty());
    talisman.template_id = IID_SHRIKE_TALISMAN;
    talisman.carried_by = Some(CharacterId(1));
    world.add_item(talisman);
    let mut godmode = player(2, "Godmode");
    godmode.level = 1;
    world.add_character(godmode);

    if let Some(yoakin) = world.characters.get_mut(&CharacterId(1)) {
        yoakin.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let mut player_facts = facts(CharacterId(2), 4, 0, 6, 0);
    player_facts.get_mut(&CharacterId(2)).unwrap().shrike_state = 0;
    player_facts.get_mut(&CharacterId(2)).unwrap().shrike_fails = 0;
    player_facts.get_mut(&CharacterId(2)).unwrap().level = 1;

    let events = world.process_yoakin_actions(&player_facts, 0, 1);
    assert!(events.contains(&YoakinOutcomeEvent::UpdateShrikeState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I'd have never thought")));
    assert!(!texts.iter().any(|text| text.message.contains("forgive")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert!(godmode.exp > 0);
    assert!(world.items.is_empty());
}

#[test]
fn yoakin_give_unrelated_item_hands_it_to_giver() {
    let mut world = World::default();
    let mut yoakin = yoakin_npc(1);
    yoakin.cursor_item = Some(ItemId(50));
    world.add_character(yoakin);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(yoakin) = world.characters.get_mut(&CharacterId(1)) {
        yoakin.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_yoakin_actions(&HashMap::new(), 0, 1);
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

#[test]
fn yoakin_text_repeat_resets_state_to_2_and_zeroes_last_talk() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(yoakin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(yoakin) = world.characters.get_mut(&CharacterId(1)) {
        yoakin.driver_state = Some(CharacterDriverState::Yoakin(YoakinDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        yoakin.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_yoakin_actions(&facts(CharacterId(2), 4, 0, 6, 0), 0, 1);
    assert!(events.contains(&YoakinOutcomeEvent::UpdateState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    assert_eq!(yoakin_state(&world, CharacterId(1)).last_talk, 0);
    assert_eq!(
        yoakin_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn yoakin_text_ignores_non_current_victim() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(yoakin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    assert!(world.spawn_character(player(3, "Other"), 12, 10));

    if let Some(yoakin) = world.characters.get_mut(&CharacterId(1)) {
        yoakin.driver_state = Some(CharacterDriverState::Yoakin(YoakinDriverData {
            last_talk: BASELINE_TICK,
            current_victim: Some(CharacterId(2)),
        }));
        yoakin.push_driver_text_message(CharacterId(3), "hello");
    }
    world.tick = Tick(BASELINE_TICK);

    let mut player_facts = facts(CharacterId(2), 4, 0, 6, 0);
    player_facts.insert(
        CharacterId(3),
        YoakinPlayerFacts {
            state: 4,
            seen_timer: 0,
            logain_state: 6,
            quest_done_count: 0,
            shrike_state: 0,
            shrike_fails: 0,
            level: 1,
        },
    );

    world.process_yoakin_actions(&player_facts, 0, 1);
    assert!(world.drain_pending_area_texts().is_empty());
}
