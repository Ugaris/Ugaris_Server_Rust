use std::collections::HashMap;

use super::*;
use crate::character_driver::{Lab5SeyanDriverData, CDR_LAB5SEYAN, NT_CHAR, NT_GIVE};
use crate::item_driver::{IDR_POTION, IID_LAB5_HEAD1, IID_LAB5_HEAD3};
use crate::world::npc::area22::lab5_seyan::{Lab5SeyanOutcomeEvent, Lab5SeyanPlayerFacts};

fn seyan_npc(id: u32) -> Character {
    let mut seyan = character(id);
    seyan.name = "Laros".into();
    seyan.driver = CDR_LAB5SEYAN;
    seyan.driver_state = Some(CharacterDriverState::Lab5Seyan(
        Lab5SeyanDriverData::default(),
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
    seyanstate: u8,
    seyangot: u8,
) -> HashMap<CharacterId, Lab5SeyanPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        Lab5SeyanPlayerFacts {
            seyanstate,
            seyangot,
        },
    );
    map
}

fn seyan_state(world: &World, seyan_id: CharacterId) -> Lab5SeyanDriverData {
    match world
        .characters
        .get(&seyan_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Lab5Seyan(data)) => data,
        _ => panic!("expected lab5 seyan driver state"),
    }
}

#[test]
fn lab5_seyan_entry_state_greets_and_advances_to_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    world.add_character(seyan_npc(1));
    world.add_character(player(2, "Godmode"));

    world.tick = Tick(200);
    if let Some(seyan) = world.characters.get_mut(&CharacterId(1)) {
        seyan.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab5_seyan_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.contains(&Lab5SeyanOutcomeEvent::SetPlayerData {
        player_id: CharacterId(2),
        seyanstate: 1,
        seyangot: 0,
    }));

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("introduce thee to the quest")));

    let state = seyan_state(&world, CharacterId(1));
    assert_eq!(state.lasttalk, 200);
    assert_eq!(state.cv_co, Some(CharacterId(2)));
}

#[test]
fn lab5_seyan_state_3_holds_at_potion_gate_and_clears_victim() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    world.add_character(seyan_npc(1));
    let mut carrier = player(2, "Godmode");
    carrier.inventory[30] = Some(ItemId(50));
    world.add_character(carrier);
    let mut potion = item(50, ItemFlags::empty());
    potion.driver = IDR_POTION;
    potion.carried_by = Some(CharacterId(2));
    world.add_item(potion);

    world.tick = Tick(200);
    if let Some(seyan) = world.characters.get_mut(&CharacterId(1)) {
        seyan.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab5_seyan_actions(&facts(CharacterId(2), 3, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());

    let state = seyan_state(&world, CharacterId(1));
    assert_eq!(state.cv_co, None);
}

#[test]
fn lab5_seyan_state_3_without_potion_advances_to_4() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    world.add_character(seyan_npc(1));
    world.add_character(player(2, "Godmode"));

    world.tick = Tick(200);
    if let Some(seyan) = world.characters.get_mut(&CharacterId(1)) {
        seyan.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab5_seyan_actions(&facts(CharacterId(2), 3, 0), 1);
    assert!(events.contains(&Lab5SeyanOutcomeEvent::SetPlayerData {
        player_id: CharacterId(2),
        seyanstate: 4,
        seyangot: 0,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("fulfil thine destiny")));
}

#[test]
fn lab5_seyan_state_22_queues_lab_exit_level_15_and_advances_to_23() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    world.add_character(seyan_npc(1));
    world.add_character(player(2, "Godmode"));

    world.tick = Tick(500);
    if let Some(seyan) = world.characters.get_mut(&CharacterId(1)) {
        seyan.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_lab5_seyan_actions(&facts(CharacterId(2), 22, 7), 1);
    assert!(events.contains(&Lab5SeyanOutcomeEvent::SetPlayerData {
        player_id: CharacterId(2),
        seyanstate: 23,
        seyangot: 7,
    }));

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Mayest thou pass the last gate")));

    let spawns = world.drain_pending_lab_exit_spawns();
    assert_eq!(spawns.len(), 1);
    assert_eq!(spawns[0].killer_id, CharacterId(2));
    assert_eq!(spawns[0].level, 15);
}

#[test]
fn lab5_seyan_give_head1_sets_bit_and_advances_state() {
    let mut world = World::default();
    let mut seyan = seyan_npc(1);
    seyan.cursor_item = Some(ItemId(50));
    world.add_character(seyan);
    let mut head1 = item(50, ItemFlags::empty());
    head1.template_id = IID_LAB5_HEAD1;
    head1.carried_by = Some(CharacterId(1));
    world.add_item(head1);
    world.add_character(player(2, "Godmode"));

    if let Some(seyan) = world.characters.get_mut(&CharacterId(1)) {
        seyan.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_lab5_seyan_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.contains(&Lab5SeyanOutcomeEvent::SetPlayerData {
        player_id: CharacterId(2),
        seyanstate: 10,
        seyangot: 1,
    }));
    assert!(world.items.get(&ItemId(50)).is_none());
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}

#[test]
fn lab5_seyan_give_all_three_heads_reaches_done_state_20() {
    let mut world = World::default();
    let mut seyan = seyan_npc(1);
    seyan.cursor_item = Some(ItemId(51));
    world.add_character(seyan);
    let mut head3 = item(51, ItemFlags::empty());
    head3.template_id = IID_LAB5_HEAD3;
    head3.carried_by = Some(CharacterId(1));
    world.add_item(head3);
    world.add_character(player(2, "Godmode"));

    if let Some(seyan) = world.characters.get_mut(&CharacterId(1)) {
        seyan.push_driver_message(NT_GIVE, 2, 51, 0);
    }

    // Player already handed in head1 + head2 (`seyangot = 3`).
    let events = world.process_lab5_seyan_actions(&facts(CharacterId(2), 11, 3), 1);
    assert!(events.contains(&Lab5SeyanOutcomeEvent::SetPlayerData {
        player_id: CharacterId(2),
        seyanstate: 20,
        seyangot: 7,
    }));
}

#[test]
fn lab5_seyan_text_repeat_recomputes_state_from_got_bits() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    world.add_character(seyan_npc(1));
    world.add_character(player(2, "Godmode"));

    world.tick = Tick(200);
    if let Some(seyan) = world.characters.get_mut(&CharacterId(1)) {
        seyan.push_driver_text_message(CharacterId(2), "please repeat that");
    }

    // Player has head1 + head3 (bits 0 and 2) but not head2 - "some", not
    // "done".
    let events = world.process_lab5_seyan_actions(&facts(CharacterId(2), 255, 5), 1);
    assert!(events.contains(&Lab5SeyanOutcomeEvent::SetPlayerData {
        player_id: CharacterId(2),
        seyanstate: 10,
        seyangot: 5,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I will repeat, Godmode")));
}
