use std::collections::HashMap;

use super::*;
use crate::character_driver::{GateWelcomeDriverData, CDR_GATE_WELCOME, NT_CHAR, NT_GIVE};
use crate::world::gatekeeper::{GateWelcomeOutcomeEvent, GateWelcomePlayerFacts};

const TALK_MIN: u64 = TICKS_PER_SECOND * 5;
const TALK_VICTIM: u64 = TICKS_PER_SECOND * 10;

fn gate_npc(id: u32) -> Character {
    let mut gate = character(id);
    gate.name = "Ishtar".into();
    gate.driver = CDR_GATE_WELCOME;
    gate.driver_state = Some(CharacterDriverState::GateWelcome(
        GateWelcomeDriverData::default(),
    ));
    gate
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    welcome_state: i32,
    needs_lab: bool,
) -> HashMap<CharacterId, GateWelcomePlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        GateWelcomePlayerFacts {
            welcome_state,
            needs_lab,
        },
    );
    map
}

fn gate_state(world: &World, gate_id: CharacterId) -> GateWelcomeDriverData {
    match world
        .characters
        .get(&gate_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::GateWelcome(data)) => data,
        _ => panic!("expected gate-welcome driver state"),
    }
}

#[test]
fn gate_welcome_greets_visible_player_and_advances_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(TALK_MIN);
    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_gate_welcome_actions(&facts(CharacterId(2), 0, false));
    assert_eq!(
        events,
        vec![GateWelcomeOutcomeEvent::UpdateWelcomeState {
            player_id: CharacterId(2),
            new_state: 1
        }]
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Be greeted, Godmode")));
    assert_eq!(
        gate_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn gate_welcome_ignores_players_out_of_range() {
    let mut world = World::default();
    world.map.tile_mut(25, 10).unwrap().light = 255;
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 25, 10));

    world.tick = Tick(TALK_MIN);
    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_gate_welcome_actions(&facts(CharacterId(2), 0, false));
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn gate_welcome_throttles_repeated_greetings() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.driver_state = Some(CharacterDriverState::GateWelcome(GateWelcomeDriverData {
            last_talk: 0,
            current_victim: Some(CharacterId(2)),
            amgivingback: 0,
        }));
        gate.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.tick = Tick(TALK_MIN - 1);

    let events = world.process_gate_welcome_actions(&facts(CharacterId(2), 0, false));
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn gate_welcome_ignores_a_different_player_while_a_victim_conversation_is_fresh() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    assert!(world.spawn_character(player(3, "Egbert"), 12, 10));

    // Within `TALK_MIN..TALK_VICTIM` of a real `current_victim`, C skips
    // any other player entirely (`gatekeeper.c:454-457`).
    world.tick = Tick(TALK_MIN);
    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.driver_state = Some(CharacterDriverState::GateWelcome(GateWelcomeDriverData {
            last_talk: 0,
            current_victim: Some(CharacterId(2)),
            amgivingback: 0,
        }));
        gate.push_driver_message(NT_CHAR, 3, 0, 0);
    }
    assert!(TALK_MIN < TALK_VICTIM);

    let events = world.process_gate_welcome_actions(&facts(CharacterId(3), 0, false));
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn gate_welcome_needs_lab_says_labyrinth_message_and_waits() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(TALK_MIN);
    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_gate_welcome_actions(&facts(CharacterId(2), 2, true));
    assert_eq!(
        events,
        vec![GateWelcomeOutcomeEvent::UpdateWelcomeState {
            player_id: CharacterId(2),
            new_state: 3
        }]
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("solve the Labyrinth built by Ishtar")));
}

#[test]
fn gate_welcome_replies_to_small_talk_keyword() {
    let mut world = World::default();
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.push_driver_text_message(CharacterId(2), "hello");
    }
    world.process_gate_welcome_actions(&facts(CharacterId(2), 0, false));

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hello, Godmode!")));
}

#[test]
fn gate_welcome_repeat_resets_welcome_state_below_seven() {
    let mut world = World::default();
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_gate_welcome_actions(&facts(CharacterId(2), 6, false));
    assert_eq!(
        events,
        vec![GateWelcomeOutcomeEvent::UpdateWelcomeState {
            player_id: CharacterId(2),
            new_state: 0
        }]
    );
}

#[test]
fn gate_welcome_god_reset_clears_lab_ppd() {
    let mut world = World::default();
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    let mut god = player(2, "Godmode");
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 10, 10));

    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.push_driver_text_message(CharacterId(2), "reset");
    }
    let events = world.process_gate_welcome_actions(&facts(CharacterId(2), 0, false));
    assert_eq!(
        events,
        vec![GateWelcomeOutcomeEvent::ResetLabPpd {
            player_id: CharacterId(2)
        }]
    );
}

#[test]
fn gate_welcome_non_god_reset_is_ignored() {
    let mut world = World::default();
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));

    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.push_driver_text_message(CharacterId(2), "reset");
    }
    let events = world.process_gate_welcome_actions(&facts(CharacterId(2), 0, false));
    assert!(events.is_empty());
}

#[test]
fn gate_welcome_gives_item_back_with_flavor_text_once() {
    let mut world = World::default();
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 10, 10));
    world.items.insert(ItemId(900), item(900, ItemFlags::TAKE));
    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.cursor_item = Some(ItemId(900));
        gate.push_driver_message(NT_GIVE, 2, 900, 0);
    }

    world.process_gate_welcome_actions(&facts(CharacterId(2), 0, false));

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("Thou hast better use for this than I do")));
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().cursor_item,
        Some(ItemId(900))
    );
    // `amgivingback` resets to `0` every tick (C `gatekeeper.c:621`), so a
    // second give-back on a later tick shows the flavor text again.
    assert_eq!(gate_state(&world, CharacterId(1)).amgivingback, 0);
}

#[test]
fn gate_welcome_destroys_item_when_giver_inventory_is_full() {
    let mut world = World::default();
    assert!(world.spawn_character(gate_npc(1), 10, 10));
    let mut full_player = player(2, "Godmode");
    full_player.cursor_item = Some(ItemId(1));
    for slot in full_player
        .inventory
        .iter_mut()
        .skip(INVENTORY_START_INVENTORY)
    {
        *slot = Some(ItemId(1));
    }
    assert!(world.spawn_character(full_player, 10, 10));
    world.items.insert(ItemId(900), item(900, ItemFlags::TAKE));
    if let Some(gate) = world.characters.get_mut(&CharacterId(1)) {
        gate.cursor_item = Some(ItemId(900));
        gate.push_driver_message(NT_GIVE, 2, 900, 0);
    }

    world.process_gate_welcome_actions(&facts(CharacterId(2), 0, false));

    assert!(world.items.get(&ItemId(900)).is_none());
}
