use std::collections::HashMap;

use super::*;
use crate::character_driver::{ThomasDriverData, CDR_THOMAS, NT_CHAR, NT_GIVE};
use crate::world::thomas::{ThomasOutcomeEvent, ThomasPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn thomas_npc(id: u32) -> Character {
    let mut thomas = character(id);
    thomas.name = "Thomas".into();
    thomas.driver = CDR_THOMAS;
    thomas.driver_state = Some(CharacterDriverState::Thomas(ThomasDriverData::default()));
    thomas
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
    level: u32,
) -> HashMap<CharacterId, ThomasPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, ThomasPlayerFacts { crypt_state, level });
    map
}

fn thomas_state(world: &World, thomas_id: CharacterId) -> ThomasDriverData {
    match world
        .characters
        .get(&thomas_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Thomas(data)) => data,
        _ => panic!("expected thomas driver state"),
    }
}

#[test]
fn thomas_greets_high_level_player_and_advances_crypt_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thomas_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thomas) = world.characters.get_mut(&CharacterId(1)) {
        thomas.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_thomas_actions(&facts(CharacterId(2), 0, 19), 1);
    assert!(events.contains(&ThomasOutcomeEvent::UpdateCryptState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("my master wishes to talk to thee")));
    assert_eq!(
        thomas_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn thomas_ignores_player_below_level_19() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thomas_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thomas) = world.characters.get_mut(&CharacterId(1)) {
        thomas.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_thomas_actions(&facts(CharacterId(2), 0, 18), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn thomas_state1_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thomas_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(thomas) = world.characters.get_mut(&CharacterId(1)) {
        thomas.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_thomas_actions(&facts(CharacterId(2), 1, 30), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn thomas_text_repeat_resets_crypt_state_to_zero_when_in_range() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(thomas_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(thomas) = world.characters.get_mut(&CharacterId(1)) {
        thomas.driver_state = Some(CharacterDriverState::Thomas(ThomasDriverData {
            last_talk: 0,
            current_victim: None,
        }));
        thomas.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_thomas_actions(&facts(CharacterId(2), 1, 30), 1);
    assert!(events.contains(&ThomasOutcomeEvent::UpdateCryptState {
        player_id: CharacterId(2),
        new_state: 0,
    }));
    assert_eq!(
        thomas_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn thomas_give_hands_item_back_to_giver() {
    let mut world = World::default();
    let mut thomas = thomas_npc(1);
    thomas.cursor_item = Some(ItemId(50));
    world.add_character(thomas);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(thomas) = world.characters.get_mut(&CharacterId(1)) {
        thomas.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_thomas_actions(&HashMap::new(), 1);
    assert!(events.is_empty());

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    // C's `thomas_driver` calls plain `give_char_item`, not `give_char_
    // item_smart` - the item lands on the (empty) cursor, not inventory.
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}
