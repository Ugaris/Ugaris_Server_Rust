use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_BROKLIN, NT_CHAR, NT_GIVE};
use crate::item_driver::{IDR_ENHANCE, IID_STAFF_PICKAXE, IID_STAFF_SEWERKEY};
use crate::world::npc::area29::broklin::{
    BroklinDriverData, BroklinOutcomeEvent, BroklinPlayerFacts, BroklinTradeReward,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn broklin_npc(id: u32) -> Character {
    let mut broklin = character(id);
    broklin.name = "Broklin".into();
    broklin.driver = CDR_BROKLIN;
    broklin.driver_state = Some(CharacterDriverState::Broklin(BroklinDriverData::default()));
    broklin
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    broklin_state: i32,
    quest46_is_done: bool,
) -> HashMap<CharacterId, BroklinPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        BroklinPlayerFacts {
            broklin_state,
            quest46_is_done,
        },
    );
    map
}

fn enhance_stack(item_id: u32, character_id: u32, kind: u8, amount: u32) -> Item {
    let mut it = item(item_id, ItemFlags::USED);
    it.driver = IDR_ENHANCE;
    it.driver_data = vec![0; 5];
    it.driver_data[0] = kind;
    it.driver_data[1..5].copy_from_slice(&amount.to_le_bytes());
    it.carried_by = Some(CharacterId(character_id));
    it
}

#[test]
fn state0_greets_opens_quest45_and_advances_to_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(broklin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(broklin) = world.characters.get_mut(&CharacterId(1)) {
        broklin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_broklin_actions(&facts(CharacterId(2), 0, false), 1);
    assert!(events.contains(&BroklinOutcomeEvent::QuestOpen45 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&BroklinOutcomeEvent::UpdateBroklinState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Chief Miner")));
}

#[test]
fn state4_is_a_silent_no_op_waiting_for_the_pickaxe() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(broklin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(broklin) = world.characters.get_mut(&CharacterId(1)) {
        broklin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_broklin_actions(&facts(CharacterId(2), 4, false), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state5_opens_quest46_and_advances_to_6_when_quest46_not_already_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(broklin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(broklin) = world.characters.get_mut(&CharacterId(1)) {
        broklin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_broklin_actions(&facts(CharacterId(2), 5, false), 1);
    assert!(events.contains(&BroklinOutcomeEvent::QuestOpen46 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&BroklinOutcomeEvent::UpdateBroklinState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("impose on your services")));
}

#[test]
fn state5_fast_forwards_to_11_when_quest46_already_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(broklin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(broklin) = world.characters.get_mut(&CharacterId(1)) {
        broklin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_broklin_actions(&facts(CharacterId(2), 5, true), 1);
    assert!(events.contains(&BroklinOutcomeEvent::UpdateBroklinState {
        player_id: CharacterId(2),
        new_state: 11,
    }));
    assert!(!events
        .iter()
        .any(|event| matches!(event, BroklinOutcomeEvent::QuestOpen46 { .. })));
    // No dialogue for the fast-forward path.
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state8_grants_sewer_key_when_not_already_carried() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(broklin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(broklin) = world.characters.get_mut(&CharacterId(1)) {
        broklin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_broklin_actions(&facts(CharacterId(2), 8, false), 1);
    assert!(events.contains(&BroklinOutcomeEvent::GrantSewerKey {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&BroklinOutcomeEvent::UpdateBroklinState {
        player_id: CharacterId(2),
        new_state: 9,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("lets you enter the sewers")));
}

#[test]
fn state8_skips_the_grant_when_sewer_key_already_carried() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(broklin_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.inventory[30] = Some(ItemId(50));
    assert!(world.spawn_character(godmode, 12, 10));
    let mut key = item(50, ItemFlags::empty());
    key.template_id = IID_STAFF_SEWERKEY;
    key.carried_by = Some(CharacterId(2));
    world.add_item(key);

    world.tick = Tick(BASELINE_TICK);
    if let Some(broklin) = world.characters.get_mut(&CharacterId(1)) {
        broklin.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_broklin_actions(&facts(CharacterId(2), 8, false), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, BroklinOutcomeEvent::GrantSewerKey { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("already have the key")));
}

#[test]
fn text_repeat_resets_to_the_current_dialogue_spans_start() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(broklin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(broklin) = world.characters.get_mut(&CharacterId(1)) {
        broklin.push_driver_text_message(CharacterId(2), "repeat");
    }
    // state 7 sits in the 5..=10 range, so should reset to 5.
    let events = world.process_broklin_actions(&facts(CharacterId(2), 7, false), 1);
    assert!(
        events.contains(&BroklinOutcomeEvent::ResetToMiniQuestStart {
            player_id: CharacterId(2),
            new_state: 5,
        })
    );
}

#[test]
fn text_reset_me_speaks_reset_done_and_pushes_reset_event_for_gods_only() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(broklin_npc(1), 10, 10));
    let mut god = player(2, "Godmode");
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 12, 10));

    if let Some(broklin) = world.characters.get_mut(&CharacterId(1)) {
        broklin.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_broklin_actions(&facts(CharacterId(2), 3, false), 1);
    assert!(events.contains(&BroklinOutcomeEvent::ResetBroklin {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("reset done")));
}

#[test]
fn text_reset_me_is_ignored_for_non_gods() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(broklin_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(broklin) = world.characters.get_mut(&CharacterId(1)) {
        broklin.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_broklin_actions(&facts(CharacterId(2), 3, false), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn give_pickaxe_at_state4_completes_quest45_and_jumps_to_5() {
    let mut world = World::default();
    let mut broklin = broklin_npc(1);
    broklin.cursor_item = Some(ItemId(50));
    world.add_character(broklin);
    let mut pickaxe = item(50, ItemFlags::empty());
    pickaxe.name = "Broklin's Pickaxe".into();
    pickaxe.template_id = IID_STAFF_PICKAXE;
    pickaxe.carried_by = Some(CharacterId(1));
    world.add_item(pickaxe);
    world.add_character(player(2, "Godmode"));

    if let Some(broklin) = world.characters.get_mut(&CharacterId(1)) {
        broklin.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_broklin_actions(&facts(CharacterId(2), 4, false), 1);
    assert!(events.contains(&BroklinOutcomeEvent::QuestDonePickaxe {
        player_id: CharacterId(2),
        broklin_id: CharacterId(1),
    }));
    assert!(events.contains(&BroklinOutcomeEvent::UpdateBroklinState {
        player_id: CharacterId(2),
        new_state: 5,
    }));
    assert!(!world.items.contains_key(&ItemId(50)));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}

#[test]
fn give_unrelated_item_is_handed_back() {
    let mut world = World::default();
    let mut broklin = broklin_npc(1);
    broklin.cursor_item = Some(ItemId(50));
    world.add_character(broklin);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(broklin) = world.characters.get_mut(&CharacterId(1)) {
        broklin.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_broklin_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn trade_gold_converts_a_sufficient_stack_and_grants_silver_4000() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(broklin_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.inventory[30] = Some(ItemId(50));
    assert!(world.spawn_character(godmode, 12, 10));
    world.add_item(enhance_stack(50, 2, 2, 1500));

    if let Some(broklin) = world.characters.get_mut(&CharacterId(1)) {
        broklin.push_driver_text_message(CharacterId(2), "thousand gold");
    }

    let events = world.process_broklin_actions(&facts(CharacterId(2), 11, false), 1);
    assert!(events.contains(&BroklinOutcomeEvent::GrantTradeReward {
        player_id: CharacterId(2),
        reward: BroklinTradeReward::Silver4000,
    }));
    let remaining = world.items.get(&ItemId(50)).unwrap();
    assert_eq!(
        remaining.driver_data[1..5],
        500u32.to_le_bytes(),
        "1500 - 1000 = 500 gold units left"
    );
    assert_eq!(remaining.value, 500 * 25);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Here you go")));
}

#[test]
fn trade_gold_destroys_the_stack_when_fully_consumed() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(broklin_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.inventory[30] = Some(ItemId(50));
    assert!(world.spawn_character(godmode, 12, 10));
    world.add_item(enhance_stack(50, 2, 2, 1000));

    if let Some(broklin) = world.characters.get_mut(&CharacterId(1)) {
        broklin.push_driver_text_message(CharacterId(2), "thousand gold");
    }

    let events = world.process_broklin_actions(&facts(CharacterId(2), 11, false), 1);
    assert!(events.contains(&BroklinOutcomeEvent::GrantTradeReward {
        player_id: CharacterId(2),
        reward: BroklinTradeReward::Silver4000,
    }));
    assert!(!world.items.contains_key(&ItemId(50)));
}

#[test]
fn trade_gold_without_enough_units_says_the_requirement_and_grants_nothing() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(broklin_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.inventory[30] = Some(ItemId(50));
    assert!(world.spawn_character(godmode, 12, 10));
    world.add_item(enhance_stack(50, 2, 2, 200));

    if let Some(broklin) = world.characters.get_mut(&CharacterId(1)) {
        broklin.push_driver_text_message(CharacterId(2), "thousand gold");
    }

    let events = world.process_broklin_actions(&facts(CharacterId(2), 11, false), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, BroklinOutcomeEvent::GrantTradeReward { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("need to have 1000 gold units")));
    // The stack is untouched.
    assert_eq!(
        world.items.get(&ItemId(50)).unwrap().driver_data[1..5],
        200u32.to_le_bytes()
    );
}

#[test]
fn trade_is_blocked_before_broklin_state_11() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(broklin_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.inventory[30] = Some(ItemId(50));
    assert!(world.spawn_character(godmode, 12, 10));
    world.add_item(enhance_stack(50, 2, 2, 5000));

    if let Some(broklin) = world.characters.get_mut(&CharacterId(1)) {
        broklin.push_driver_text_message(CharacterId(2), "thousand gold");
    }

    let events = world.process_broklin_actions(&facts(CharacterId(2), 5, false), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
    // The stack is untouched.
    assert_eq!(
        world.items.get(&ItemId(50)).unwrap().driver_data[1..5],
        5000u32.to_le_bytes()
    );
}

#[test]
fn trade_silver_converts_a_sufficient_stack_and_grants_gold_1000() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(broklin_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.inventory[30] = Some(ItemId(50));
    assert!(world.spawn_character(godmode, 12, 10));
    world.add_item(enhance_stack(50, 2, 1, 7000));

    if let Some(broklin) = world.characters.get_mut(&CharacterId(1)) {
        broklin.push_driver_text_message(CharacterId(2), "five thousand silver");
    }

    let events = world.process_broklin_actions(&facts(CharacterId(2), 11, false), 1);
    assert!(events.contains(&BroklinOutcomeEvent::GrantTradeReward {
        player_id: CharacterId(2),
        reward: BroklinTradeReward::Gold1000,
    }));
    let remaining = world.items.get(&ItemId(50)).unwrap();
    assert_eq!(
        remaining.driver_data[1..5],
        2000u32.to_le_bytes(),
        "7000 - 5000 = 2000 silver units left"
    );
    assert_eq!(remaining.value, 2000 * 10);
}
