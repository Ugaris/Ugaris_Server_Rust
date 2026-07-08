use std::collections::HashMap;

use super::*;
use crate::character_driver::{SirJonesDriverData, CDR_SIRJONES, NT_CHAR, NT_GIVE};
use crate::world::sir_jones::{SirJonesOutcomeEvent, SirJonesPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn sir_jones_npc(id: u32) -> Character {
    let mut sir_jones = character(id);
    sir_jones.name = "Sir Jones".into();
    sir_jones.driver = CDR_SIRJONES;
    sir_jones.driver_state = Some(CharacterDriverState::SirJones(SirJonesDriverData::default()));
    sir_jones
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    crypt_state: i32,
    crypt_bonus: i32,
    quest18_count: u8,
    quest19_done: bool,
) -> HashMap<CharacterId, SirJonesPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        SirJonesPlayerFacts {
            crypt_state,
            crypt_bonus,
            quest18_count,
            quest19_done,
        },
    );
    map
}

fn sir_jones_state(world: &World, sir_jones_id: CharacterId) -> SirJonesDriverData {
    match world
        .characters
        .get(&sir_jones_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::SirJones(data)) => data,
        _ => panic!("expected sir_jones driver state"),
    }
}

#[test]
fn sir_jones_state1_greets_opens_quest18_and_advances_to_2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(sir_jones_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(sir_jones) = world.characters.get_mut(&CharacterId(1)) {
        sir_jones.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_sir_jones_actions(&facts(CharacterId(2), 1, 0, 0, false), 1);
    assert!(events.contains(&SirJonesOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 18,
    }));
    assert!(events.contains(&SirJonesOutcomeEvent::UpdateCryptState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Welcome to my humble home")));
    assert_eq!(
        sir_jones_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn sir_jones_state2_mentions_warrior_for_warrior_flag() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(sir_jones_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.flags |= CharacterFlags::WARRIOR;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(sir_jones) = world.characters.get_mut(&CharacterId(1)) {
        sir_jones.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_sir_jones_actions(&facts(CharacterId(2), 2, 0, 0, false), 1);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("tough warrior")));
}

#[test]
fn sir_jones_state4_offer_carries_color_sentinels() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(sir_jones_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(sir_jones) = world.characters.get_mut(&CharacterId(1)) {
        sir_jones.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_sir_jones_actions(&facts(CharacterId(2), 4, 0, 0, false), 1);
    assert!(events.contains(&SirJonesOutcomeEvent::UpdateCryptState {
        player_id: CharacterId(2),
        new_state: 5,
    }));
    let texts = world.drain_pending_area_text_bytes();
    assert!(
        texts
            .iter()
            .any(|text| String::from_utf8_lossy(&text.message)
                .contains("Would thou be willing to go"))
    );
    assert!(texts
        .iter()
        .any(|text| text.message.windows(6).any(|w| w == b"\xb0c4Aye")));
}

#[test]
fn sir_jones_state10_double_increments_to_12_and_rewards_gold_when_bonus_earned() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(sir_jones_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(sir_jones) = world.characters.get_mut(&CharacterId(1)) {
        sir_jones.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // crypt_bonus set, quest18_count == 1 (first completion) -> gold reward.
    let events = world.process_sir_jones_actions(&facts(CharacterId(2), 10, 1, 1, false), 1);
    assert!(events.contains(&SirJonesOutcomeEvent::UpdateCryptState {
        player_id: CharacterId(2),
        new_state: 12,
    }));
    assert!(events.contains(&SirJonesOutcomeEvent::GoldEarned {
        player_id: CharacterId(2),
        amount: 2500,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("Well done")));
}

#[test]
fn sir_jones_state10_skips_gold_reward_without_bonus() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(sir_jones_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(sir_jones) = world.characters.get_mut(&CharacterId(1)) {
        sir_jones.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_sir_jones_actions(&facts(CharacterId(2), 10, 0, 1, false), 1);
    assert!(events.contains(&SirJonesOutcomeEvent::UpdateCryptState {
        player_id: CharacterId(2),
        new_state: 12,
    }));
    assert!(!events
        .iter()
        .any(|event| matches!(event, SirJonesOutcomeEvent::GoldEarned { .. })));
}

#[test]
fn sir_jones_state12_jumps_silently_to_14_when_quest19_already_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(sir_jones_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(sir_jones) = world.characters.get_mut(&CharacterId(1)) {
        sir_jones.driver_state = Some(CharacterDriverState::SirJones(SirJonesDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        sir_jones.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_sir_jones_actions(&facts(CharacterId(2), 12, 0, 1, true), 1);
    assert!(events.contains(&SirJonesOutcomeEvent::UpdateCryptState {
        player_id: CharacterId(2),
        new_state: 14,
    }));
    assert!(world.drain_pending_area_texts().is_empty());
    // C never sets `didsay = 1` for this sub-branch: `last_talk`/
    // `current_victim` stay untouched.
    assert_eq!(sir_jones_state(&world, CharacterId(1)).last_talk, 500);
    assert_eq!(sir_jones_state(&world, CharacterId(1)).current_victim, None);
}

#[test]
fn sir_jones_state12_opens_quest19_without_updating_last_talk_when_not_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(sir_jones_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(sir_jones) = world.characters.get_mut(&CharacterId(1)) {
        sir_jones.driver_state = Some(CharacterDriverState::SirJones(SirJonesDriverData {
            last_talk: 500,
            current_victim: None,
        }));
        sir_jones.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_sir_jones_actions(&facts(CharacterId(2), 12, 0, 1, false), 1);
    assert!(events.contains(&SirJonesOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 19,
    }));
    assert!(events.contains(&SirJonesOutcomeEvent::UpdateCryptState {
        player_id: CharacterId(2),
        new_state: 13,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("even tougher creature")));
    // Same "no didsay" quirk as the quest-19-already-done sub-branch.
    assert_eq!(sir_jones_state(&world, CharacterId(1)).last_talk, 500);
}

#[test]
fn sir_jones_text_aye_in_low_range_advances_to_8() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(sir_jones_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(sir_jones) = world.characters.get_mut(&CharacterId(1)) {
        sir_jones.push_driver_text_message(CharacterId(2), "aye");
    }

    let events = world.process_sir_jones_actions(&facts(CharacterId(2), 4, 0, 0, false), 1);
    assert!(events.contains(&SirJonesOutcomeEvent::UpdateCryptState {
        player_id: CharacterId(2),
        new_state: 8,
    }));
    assert!(!events
        .iter()
        .any(|event| matches!(event, SirJonesOutcomeEvent::SetCryptBonus { .. })));
}

#[test]
fn sir_jones_text_aye_in_sweetened_range_sets_bonus() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(sir_jones_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(sir_jones) = world.characters.get_mut(&CharacterId(1)) {
        sir_jones.push_driver_text_message(CharacterId(2), "aye");
    }

    let events = world.process_sir_jones_actions(&facts(CharacterId(2), 6, 0, 0, false), 1);
    assert!(events.contains(&SirJonesOutcomeEvent::UpdateCryptState {
        player_id: CharacterId(2),
        new_state: 8,
    }));
    assert!(events.contains(&SirJonesOutcomeEvent::SetCryptBonus {
        player_id: CharacterId(2),
    }));
}

#[test]
fn sir_jones_text_nay_in_low_range_resets_to_6() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(sir_jones_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(sir_jones) = world.characters.get_mut(&CharacterId(1)) {
        sir_jones.push_driver_text_message(CharacterId(2), "nay");
    }

    let events = world.process_sir_jones_actions(&facts(CharacterId(2), 4, 0, 0, false), 1);
    assert!(events.contains(&SirJonesOutcomeEvent::UpdateCryptState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
}

#[test]
fn sir_jones_text_repeat_in_high_range_resets_to_12() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(sir_jones_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(sir_jones) = world.characters.get_mut(&CharacterId(1)) {
        sir_jones.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_sir_jones_actions(&facts(CharacterId(2), 13, 0, 0, false), 1);
    assert!(events.contains(&SirJonesOutcomeEvent::UpdateCryptState {
        player_id: CharacterId(2),
        new_state: 12,
    }));
}

#[test]
fn sir_jones_give_hands_item_back_to_giver() {
    let mut world = World::default();
    let mut sir_jones = sir_jones_npc(1);
    sir_jones.cursor_item = Some(ItemId(50));
    world.add_character(sir_jones);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(sir_jones) = world.characters.get_mut(&CharacterId(1)) {
        sir_jones.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_sir_jones_actions(&HashMap::new(), 1);
    assert!(events.is_empty());

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    // C's `sir_jones_driver` calls plain `give_char_item`, not
    // `give_char_item_smart` - the item lands on the (empty) cursor.
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
