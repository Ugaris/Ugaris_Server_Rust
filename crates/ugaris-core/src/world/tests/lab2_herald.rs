use std::collections::HashMap;

use super::*;
use crate::character_driver::{Lab2HeraldDriverData, CDR_LAB2HERALD, NT_CHAR, NT_GIVE};
use crate::item_driver::IID_LAB2_ARATHASRING;
use crate::world::npc::area22::lab2_herald::{Lab2HeraldOutcomeEvent, Lab2HeraldPlayerFacts};

fn herald_npc(id: u32) -> Character {
    let mut herald = character(id);
    herald.name = "Herald".into();
    herald.driver = CDR_LAB2HERALD;
    herald.driver_state = Some(CharacterDriverState::Lab2Herald(
        Lab2HeraldDriverData::default(),
    ));
    herald
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    herald_talkstep: u8,
) -> HashMap<CharacterId, Lab2HeraldPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, Lab2HeraldPlayerFacts { herald_talkstep });
    map
}

fn herald_state(world: &World, herald_id: CharacterId) -> Lab2HeraldDriverData {
    match world
        .characters
        .get(&herald_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Lab2Herald(data)) => data,
        _ => panic!("expected lab2 herald driver state"),
    }
}

#[test]
fn lab2_herald_entry_talkstep_greets_and_advances_to_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    world.add_character(herald_npc(1));
    world.add_character(player(2, "Godmode"));

    world.tick = Tick(100);
    if let Some(herald) = world.characters.get_mut(&CharacterId(1)) {
        herald.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab2_herald_actions(&facts(CharacterId(2), 0), 1);
    assert!(events.contains(&Lab2HeraldOutcomeEvent::UpdateTalkstep {
        player_id: CharacterId(2),
        new_value: 1,
    }));

    let texts = world.drain_pending_area_text_bytes();
    assert_eq!(texts.len(), 1);
    let text = String::from_utf8_lossy(&texts[0].message);
    assert!(text.contains("I am Herald, the Keeper of this graveyard"));
    assert!(text.contains("has caused this abomination"));
    // "Arathas" is wrapped in COL_LIGHT_BLUE/COL_RESET markers.
    assert!(texts[0].message.windows(10).any(|w| w == b"\xb0c4Arathas"));

    let state = herald_state(&world, CharacterId(1));
    assert_eq!(state.last_talk, 100);
    assert_eq!(state.next_talk, 100 + TICKS_PER_SECOND * 10);
}

#[test]
fn lab2_herald_talkstep_62_farewells_and_queues_lab_exit() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    world.add_character(herald_npc(1));
    world.add_character(player(2, "Godmode"));

    world.tick = Tick(500);
    if let Some(herald) = world.characters.get_mut(&CharacterId(1)) {
        herald.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab2_herald_actions(&facts(CharacterId(2), 62), 1);
    assert!(events.contains(&Lab2HeraldOutcomeEvent::UpdateTalkstep {
        player_id: CharacterId(2),
        new_value: 255,
    }));

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Mayest thou pass the last gate")));

    let spawns = world.drain_pending_lab_exit_spawns();
    assert_eq!(spawns.len(), 1);
    assert_eq!(spawns[0].killer_id, CharacterId(2));
    assert_eq!(spawns[0].level, 30);
}

#[test]
fn lab2_herald_char_message_respects_next_talk_cooldown() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    let mut herald = herald_npc(1);
    herald.driver_state = Some(CharacterDriverState::Lab2Herald(Lab2HeraldDriverData {
        last_talk: 0,
        next_talk: 1_000,
    }));
    world.add_character(herald);
    world.add_character(player(2, "Godmode"));

    world.tick = Tick(500);
    if let Some(herald) = world.characters.get_mut(&CharacterId(1)) {
        herald.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab2_herald_actions(&facts(CharacterId(2), 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn lab2_herald_char_message_ignores_players_out_of_range() {
    let mut world = World::default();
    world.map.tile_mut(25, 10).unwrap().light = 255;
    world.add_character(herald_npc(1));
    world.add_character(player(2, "Godmode"));
    if let Some(godmode) = world.characters.get_mut(&CharacterId(2)) {
        godmode.x = 25;
        godmode.y = 10;
    }

    world.tick = Tick(100);
    if let Some(herald) = world.characters.get_mut(&CharacterId(1)) {
        herald.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab2_herald_actions(&facts(CharacterId(2), 0), 1);
    assert!(events.is_empty());
}

#[test]
fn lab2_herald_text_arathas_keyword_advances_speaker_talkstep() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    world.add_character(herald_npc(1));
    world.add_character(player(2, "Godmode"));

    world.tick = Tick(200);
    if let Some(herald) = world.characters.get_mut(&CharacterId(1)) {
        herald.push_driver_text_message(CharacterId(2), "Tell me about Arathas");
    }

    let events = world.process_lab2_herald_actions(&facts(CharacterId(2), 255), 1);
    assert!(events.contains(&Lab2HeraldOutcomeEvent::UpdateTalkstep {
        player_id: CharacterId(2),
        new_value: 10,
    }));
    let state = herald_state(&world, CharacterId(1));
    assert_eq!(state.next_talk, 200 + TICKS_PER_SECOND / 2);
}

#[test]
fn lab2_herald_text_repeat_keyword_resets_talkstep_and_says_line() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    world.add_character(herald_npc(1));
    world.add_character(player(2, "Godmode"));

    world.tick = Tick(200);
    if let Some(herald) = world.characters.get_mut(&CharacterId(1)) {
        herald.push_driver_text_message(CharacterId(2), "please repeat that");
    }

    let events = world.process_lab2_herald_actions(&facts(CharacterId(2), 255), 1);
    assert!(events.contains(&Lab2HeraldOutcomeEvent::UpdateTalkstep {
        player_id: CharacterId(2),
        new_value: 0,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I will repeat, Godmode")));
}

#[test]
fn lab2_herald_give_ring_advances_talkstep_and_destroys_item() {
    let mut world = World::default();
    let mut herald = herald_npc(1);
    herald.cursor_item = Some(ItemId(50));
    world.add_character(herald);
    let mut ring = item(50, ItemFlags::empty());
    ring.template_id = IID_LAB2_ARATHASRING;
    ring.carried_by = Some(CharacterId(1));
    world.add_item(ring);
    world.add_character(player(2, "Godmode"));

    if let Some(herald) = world.characters.get_mut(&CharacterId(1)) {
        herald.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_lab2_herald_actions(&facts(CharacterId(2), 1), 1);
    assert!(events.contains(&Lab2HeraldOutcomeEvent::UpdateTalkstep {
        player_id: CharacterId(2),
        new_value: 60,
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
fn lab2_herald_give_non_ring_item_is_destroyed_without_talkstep_change() {
    let mut world = World::default();
    let mut herald = herald_npc(1);
    herald.cursor_item = Some(ItemId(51));
    world.add_character(herald);
    let mut junk = item(51, ItemFlags::empty());
    junk.carried_by = Some(CharacterId(1));
    world.add_item(junk);
    world.add_character(player(2, "Godmode"));

    if let Some(herald) = world.characters.get_mut(&CharacterId(1)) {
        herald.push_driver_message(NT_GIVE, 2, 51, 0);
    }

    let events = world.process_lab2_herald_actions(&facts(CharacterId(2), 1), 1);
    assert!(events.is_empty());
    assert!(!world.items.contains_key(&ItemId(51)));
}
