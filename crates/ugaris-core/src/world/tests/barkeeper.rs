use std::collections::HashMap;

use super::*;
use crate::character_driver::{
    CharacterDriverMessage, TwoBarkeeperDriverData, CDR_TWOBARKEEPER, NT_CHAR, NT_GIVE, NT_TEXT,
};
use crate::world::npc::area17::{
    TwoBarkeeperOutcomeEvent, TwoBarkeeperPlayerFacts, CS_CITIZEN, CS_ENEMY, CS_GUEST, LS_CLEAN,
    LS_DEAD, LS_FINE,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn barkeeper_npc(id: u32) -> Character {
    let mut barkeeper = character(id);
    barkeeper.name = "Barkeeper".into();
    barkeeper.driver = CDR_TWOBARKEEPER;
    barkeeper.driver_state = Some(CharacterDriverState::TwoBarkeeper(
        TwoBarkeeperDriverData::default(),
    ));
    barkeeper
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    barkeeper_state: i32,
    citizen_status: i32,
    legal_status: i32,
    legal_fine: i32,
) -> HashMap<CharacterId, TwoBarkeeperPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        TwoBarkeeperPlayerFacts {
            barkeeper_state,
            citizen_status,
            legal_status,
            legal_fine,
        },
    );
    map
}

fn barkeeper_state(world: &World, barkeeper_id: CharacterId) -> TwoBarkeeperDriverData {
    match world
        .characters
        .get(&barkeeper_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::TwoBarkeeper(data)) => data,
        _ => panic!("expected two barkeeper driver state"),
    }
}

#[test]
fn barkeeper_greets_new_player_and_advances_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(barkeeper_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(barkeeper) = world.characters.get_mut(&CharacterId(1)) {
        barkeeper.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_barkeeper_actions(
        &facts(CharacterId(2), 0, CS_ENEMY, LS_CLEAN, 0),
        0,
        17,
    );
    assert!(
        events.contains(&TwoBarkeeperOutcomeEvent::UpdateBarkeeperState {
            player_id: CharacterId(2),
            new_state: 1,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Welcome to the tavern of the Two Towns")));
    assert_eq!(
        barkeeper_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn barkeeper_state1_offers_guest_pass_when_not_yet_a_guest() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(barkeeper_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(barkeeper) = world.characters.get_mut(&CharacterId(1)) {
        barkeeper.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_barkeeper_actions(
        &facts(CharacterId(2), 1, CS_ENEMY, LS_CLEAN, 0),
        1234,
        17,
    );
    assert!(
        events.contains(&TwoBarkeeperOutcomeEvent::UpdateBarkeeperState {
            player_id: CharacterId(2),
            new_state: 2,
        })
    );
    assert!(
        events.contains(&TwoBarkeeperOutcomeEvent::UpdateBarkeeperLast {
            player_id: CharacterId(2),
            realtime: 1234,
        })
    );
    // The offer wraps "buy pass" in `COL_LIGHT_BLUE`/`COL_RESET` markers
    // (`two.c:845`); goes out via `npc_say_bytes`.
    let texts = world.drain_pending_area_text_bytes();
    assert!(texts.iter().any(|text| {
        let text = String::from_utf8_lossy(&text.message);
        text.contains("buy pass") && text.contains("150G")
    }));
}

#[test]
fn barkeeper_state1_fine_offer_includes_fine_total() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(barkeeper_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(barkeeper) = world.characters.get_mut(&CharacterId(1)) {
        barkeeper.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_barkeeper_actions(
        &facts(CharacterId(2), 1, CS_ENEMY, LS_FINE, 5000),
        1234,
        17,
    );
    assert!(
        events.contains(&TwoBarkeeperOutcomeEvent::UpdateBarkeeperState {
            player_id: CharacterId(2),
            new_state: 2,
        })
    );
    let texts = world.drain_pending_area_text_bytes();
    // C `ppd->legal_fine / 100 = 50`, total `50 + 150 = 200`.
    assert!(texts.iter().any(|text| {
        let text = String::from_utf8_lossy(&text.message);
        text.contains("pay 50G fines") && text.contains("200G")
    }));
}

#[test]
fn barkeeper_state1_dead_offer_asks_2500_gold() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(barkeeper_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(barkeeper) = world.characters.get_mut(&CharacterId(1)) {
        barkeeper.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_barkeeper_actions(
        &facts(CharacterId(2), 1, CS_ENEMY, LS_DEAD, 0),
        1234,
        17,
    );
    assert!(
        events.contains(&TwoBarkeeperOutcomeEvent::UpdateBarkeeperState {
            player_id: CharacterId(2),
            new_state: 2,
        })
    );
    let texts = world.drain_pending_area_text_bytes();
    assert!(texts.iter().any(|text| {
        let text = String::from_utf8_lossy(&text.message);
        text.contains("2500G") && text.contains("killed the governor's double")
    }));
}

#[test]
fn barkeeper_state1_stays_silent_once_already_a_guest() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(barkeeper_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(barkeeper) = world.characters.get_mut(&CharacterId(1)) {
        barkeeper.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_barkeeper_actions(
        &facts(CharacterId(2), 1, CS_GUEST, LS_CLEAN, 0),
        1234,
        17,
    );
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
    // Not marked as `didsay`, so `current_victim` stays unset.
    assert_eq!(barkeeper_state(&world, CharacterId(1)).current_victim, None);
}

#[test]
fn barkeeper_state2_is_a_permanent_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(barkeeper_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(barkeeper) = world.characters.get_mut(&CharacterId(1)) {
        barkeeper.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_two_barkeeper_actions(
        &facts(CharacterId(2), 2, CS_ENEMY, LS_CLEAN, 0),
        1234,
        17,
    );
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn barkeeper_repeat_command_resets_state_to_zero() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(barkeeper_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(barkeeper) = world.characters.get_mut(&CharacterId(1)) {
        barkeeper.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("repeat".to_string()),
        });
    }

    let events = world.process_two_barkeeper_actions(
        &facts(CharacterId(2), 2, CS_ENEMY, LS_CLEAN, 0),
        0,
        17,
    );
    assert!(
        events.contains(&TwoBarkeeperOutcomeEvent::UpdateBarkeeperState {
            player_id: CharacterId(2),
            new_state: 0,
        })
    );
}

#[test]
fn barkeeper_greeting_qa_reply_substitutes_speaker_name() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(barkeeper_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(barkeeper) = world.characters.get_mut(&CharacterId(1)) {
        barkeeper.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("hello".to_string()),
        });
    }

    world.process_two_barkeeper_actions(&facts(CharacterId(2), 0, CS_ENEMY, LS_CLEAN, 0), 0, 17);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hello, Godmode!")));
}

#[test]
fn barkeeper_buy_pass_succeeds_with_enough_gold() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(barkeeper_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.gold = 20000;
    assert!(world.spawn_character(godmode, 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(barkeeper) = world.characters.get_mut(&CharacterId(1)) {
        barkeeper.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("buy pass".to_string()),
        });
    }

    let events = world.process_two_barkeeper_actions(
        &facts(CharacterId(2), 2, CS_ENEMY, LS_CLEAN, 0),
        0,
        17,
    );
    assert!(events.contains(&TwoBarkeeperOutcomeEvent::BuyPass {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Thou canst now enter Exkordon")));
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 5000);
}

#[test]
fn barkeeper_buy_pass_fails_without_enough_gold() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(barkeeper_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.gold = 100;
    assert!(world.spawn_character(godmode, 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(barkeeper) = world.characters.get_mut(&CharacterId(1)) {
        barkeeper.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("buy pass".to_string()),
        });
    }

    let events = world.process_two_barkeeper_actions(
        &facts(CharacterId(2), 2, CS_ENEMY, LS_CLEAN, 0),
        0,
        17,
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, TwoBarkeeperOutcomeEvent::BuyPass { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("dost not have enough money")));
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 100);
}

#[test]
fn barkeeper_buy_pass_when_already_a_citizen_declines() {
    let mut world = World::default();
    world.map.tile_mut(11, 10).unwrap().light = 255;
    assert!(world.spawn_character(barkeeper_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.gold = 100000;
    assert!(world.spawn_character(godmode, 11, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(barkeeper) = world.characters.get_mut(&CharacterId(1)) {
        barkeeper.driver_messages.push(CharacterDriverMessage {
            message_type: NT_TEXT,
            dat1: 0,
            dat2: 0,
            dat3: 2,
            text: Some("buy pass".to_string()),
        });
    }

    let events = world.process_two_barkeeper_actions(
        &facts(CharacterId(2), 2, CS_CITIZEN, LS_CLEAN, 0),
        0,
        17,
    );
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("But thou hast a pass already")));
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().gold, 100000);
}

#[test]
fn barkeeper_receiving_any_item_destroys_it() {
    let mut world = World::default();
    let mut barkeeper = barkeeper_npc(1);
    barkeeper.cursor_item = Some(ItemId(50));
    world.add_character(barkeeper);

    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(barkeeper) = world.characters.get_mut(&CharacterId(1)) {
        barkeeper.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_two_barkeeper_actions(
        &facts(CharacterId(2), 0, CS_ENEMY, LS_CLEAN, 0),
        0,
        17,
    );
    assert!(events.is_empty());
    assert!(world.items.get(&ItemId(50)).is_none());
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().cursor_item,
        None
    );
}
