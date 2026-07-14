use std::collections::HashMap;

use super::*;
use crate::character_driver::{Lab4SeyanDriverData, CDR_LAB4SEYAN, NT_CHAR, NT_GIVE};
use crate::item_driver::{IID_LAB4_CROWN, IID_LAB4_SZEPTER};
use crate::world::npc::area22::lab4_seyan::{Lab4SeyanOutcomeEvent, Lab4SeyanPlayerFacts};

fn seyan_npc(id: u32) -> Character {
    let mut seyan = character(id);
    seyan.name = "Observer".into();
    seyan.driver = CDR_LAB4SEYAN;
    seyan.driver_state = Some(CharacterDriverState::Lab4Seyan(
        Lab4SeyanDriverData::default(),
    ));
    seyan
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    seyan4state: u8,
    seyan4got: u8,
) -> HashMap<CharacterId, Lab4SeyanPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        Lab4SeyanPlayerFacts {
            seyan4state,
            seyan4got,
        },
    );
    map
}

fn seyan_state(world: &World, seyan_id: CharacterId) -> Lab4SeyanDriverData {
    match world
        .characters
        .get(&seyan_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Lab4Seyan(data)) => data,
        _ => panic!("expected lab4 seyan driver state"),
    }
}

#[test]
fn lab4_seyan_entry_state_greets_and_advances_to_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    world.add_character(seyan_npc(1));
    world.add_character(player(2, "Godmode"));

    world.tick = Tick(200);
    if let Some(seyan) = world.characters.get_mut(&CharacterId(1)) {
        seyan.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab4_seyan_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.contains(&Lab4SeyanOutcomeEvent::SetPlayerData {
        player_id: CharacterId(2),
        seyan4state: 1,
        seyan4got: 0,
    }));

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("This is thy first mission")));

    let state = seyan_state(&world, CharacterId(1));
    assert_eq!(state.lasttalk, 200);
    assert_eq!(state.cv_co, Some(CharacterId(2)));
}

#[test]
fn lab4_seyan_state_5_clears_current_victim_without_speaking() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    let mut seyan = seyan_npc(1);
    seyan.driver_state = Some(CharacterDriverState::Lab4Seyan(Lab4SeyanDriverData {
        cv_co: Some(CharacterId(2)),
        cv_serial: 2,
        lasttalk: 0,
    }));
    world.add_character(seyan);
    world.add_character(player(2, "Godmode"));

    world.tick = Tick(500);
    if let Some(seyan) = world.characters.get_mut(&CharacterId(1)) {
        seyan.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab4_seyan_actions(&facts(CharacterId(2), 5, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());

    let state = seyan_state(&world, CharacterId(1));
    assert_eq!(state.cv_co, None);
}

#[test]
fn lab4_seyan_state_32_queues_lab_exit_and_advances_to_33() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    world.add_character(seyan_npc(1));
    world.add_character(player(2, "Godmode"));

    world.tick = Tick(500);
    if let Some(seyan) = world.characters.get_mut(&CharacterId(1)) {
        seyan.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab4_seyan_actions(&facts(CharacterId(2), 32, 3), 1);
    assert!(events.contains(&Lab4SeyanOutcomeEvent::SetPlayerData {
        player_id: CharacterId(2),
        seyan4state: 33,
        seyan4got: 3,
    }));

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Mayest Thou Past The Last Gate")));

    let spawns = world.drain_pending_lab_exit_spawns();
    assert_eq!(spawns.len(), 1);
    assert_eq!(spawns[0].killer_id, CharacterId(2));
    assert_eq!(spawns[0].level, 10);
}

#[test]
fn lab4_seyan_char_message_ignores_players_out_of_range() {
    let mut world = World::default();
    world.map.tile_mut(25, 10).unwrap().light = 255;
    world.add_character(seyan_npc(1));
    world.add_character(player(2, "Godmode"));
    if let Some(godmode) = world.characters.get_mut(&CharacterId(2)) {
        godmode.x = 25;
        godmode.y = 10;
    }

    world.tick = Tick(200);
    if let Some(seyan) = world.characters.get_mut(&CharacterId(1)) {
        seyan.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab4_seyan_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.is_empty());
}

#[test]
fn lab4_seyan_char_message_respects_lasttalk_cooldown() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    let mut seyan = seyan_npc(1);
    seyan.driver_state = Some(CharacterDriverState::Lab4Seyan(Lab4SeyanDriverData {
        cv_co: None,
        cv_serial: 0,
        lasttalk: 500,
    }));
    world.add_character(seyan);
    world.add_character(player(2, "Godmode"));

    world.tick = Tick(501);
    if let Some(seyan) = world.characters.get_mut(&CharacterId(1)) {
        seyan.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab4_seyan_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn lab4_seyan_give_crown_sets_bit_and_advances_state() {
    let mut world = World::default();
    let mut seyan = seyan_npc(1);
    seyan.cursor_item = Some(ItemId(50));
    world.add_character(seyan);
    let mut crown = item(50, ItemFlags::empty());
    crown.template_id = IID_LAB4_CROWN;
    crown.carried_by = Some(CharacterId(1));
    world.add_item(crown);
    world.add_character(player(2, "Godmode"));

    if let Some(seyan) = world.characters.get_mut(&CharacterId(1)) {
        seyan.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_lab4_seyan_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.contains(&Lab4SeyanOutcomeEvent::SetPlayerData {
        player_id: CharacterId(2),
        seyan4state: 10,
        seyan4got: 1,
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
fn lab4_seyan_give_szepter_combined_with_crown_reaches_state_30() {
    let mut world = World::default();
    let mut seyan = seyan_npc(1);
    seyan.cursor_item = Some(ItemId(51));
    world.add_character(seyan);
    let mut szepter = item(51, ItemFlags::empty());
    szepter.template_id = IID_LAB4_SZEPTER;
    szepter.carried_by = Some(CharacterId(1));
    world.add_item(szepter);
    world.add_character(player(2, "Godmode"));

    if let Some(seyan) = world.characters.get_mut(&CharacterId(1)) {
        seyan.push_driver_message(NT_GIVE, 2, 51, 0);
    }

    // Player already handed in the crown (`seyan4got = 1`).
    let events = world.process_lab4_seyan_actions(&facts(CharacterId(2), 11, 1), 1);
    assert!(events.contains(&Lab4SeyanOutcomeEvent::SetPlayerData {
        player_id: CharacterId(2),
        seyan4state: 30,
        seyan4got: 3,
    }));
}

#[test]
fn lab4_seyan_give_non_quest_item_is_destroyed_without_state_change() {
    let mut world = World::default();
    let mut seyan = seyan_npc(1);
    seyan.cursor_item = Some(ItemId(52));
    world.add_character(seyan);
    let mut junk = item(52, ItemFlags::empty());
    junk.carried_by = Some(CharacterId(1));
    world.add_item(junk);
    world.add_character(player(2, "Godmode"));

    if let Some(seyan) = world.characters.get_mut(&CharacterId(1)) {
        seyan.push_driver_message(NT_GIVE, 2, 52, 0);
    }

    let events = world.process_lab4_seyan_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.is_empty());
    assert!(!world.items.contains_key(&ItemId(52)));
}

#[test]
fn lab4_seyan_text_repeat_recomputes_state_from_got_bits() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    world.add_character(seyan_npc(1));
    world.add_character(player(2, "Godmode"));

    world.tick = Tick(200);
    if let Some(seyan) = world.characters.get_mut(&CharacterId(1)) {
        seyan.push_driver_text_message(CharacterId(2), "please repeat that");
    }

    // Player already has the szepter (bit 1) but not the crown.
    let events = world.process_lab4_seyan_actions(&facts(CharacterId(2), 255, 2), 1);
    assert!(events.contains(&Lab4SeyanOutcomeEvent::SetPlayerData {
        player_id: CharacterId(2),
        seyan4state: 20,
        seyan4got: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I will repeat, Godmode")));
}
