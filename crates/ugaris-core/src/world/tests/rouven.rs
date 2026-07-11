use std::collections::HashMap;

use super::*;
use crate::character_driver::{RouvenDriverData, CDR_ROUVEN, NT_CHAR, NT_GIVE};
use crate::item_driver::{IID_MAX_CHRONICLES, IID_MAX_RITUAL, IID_MAX_VAULTKEY};
use crate::world::npc::area26::rouven::{RouvenOutcomeEvent, RouvenPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn rouven_npc(id: u32) -> Character {
    let mut rouven = character(id);
    rouven.name = "Rouven".into();
    rouven.driver = CDR_ROUVEN;
    rouven.driver_state = Some(CharacterDriverState::Rouven(RouvenDriverData::default()));
    rouven
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    rouven_state: i32,
    carlos2_state: i32,
) -> HashMap<CharacterId, RouvenPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        RouvenPlayerFacts {
            rouven_state,
            carlos2_state,
        },
    );
    map
}

fn rouven_state(world: &World, rouven_id: CharacterId) -> RouvenDriverData {
    match world
        .characters
        .get(&rouven_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Rouven(data)) => data,
        _ => panic!("expected rouven driver state"),
    }
}

#[test]
fn state0_without_carlos2_state_speaks_only_once_via_driver_memory() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rouven_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(rouven) = world.characters.get_mut(&CharacterId(1)) {
        rouven.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    let events = world.process_rouven_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Please talk to Carlos first")));

    // Second sighting (past the min-talk-interval guard): the "check back
    // later" line is not repeated (`mem_check_driver` now returns true).
    world.tick = Tick(BASELINE_TICK + TICKS_PER_SECOND * 5);
    if let Some(rouven) = world.characters.get_mut(&CharacterId(1)) {
        rouven.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    let events = world.process_rouven_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state0_with_carlos2_state_greets_opens_quest62_and_advances_to_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rouven_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(rouven) = world.characters.get_mut(&CharacterId(1)) {
        rouven.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_rouven_actions(&facts(CharacterId(2), 0, 1), 1);
    assert!(events.contains(&RouvenOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 62,
    }));
    assert!(events.contains(&RouvenOutcomeEvent::UpdateRouvenState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("Hail")));
    assert_eq!(
        rouven_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn states1_through_4_advance_one_state_each_with_dialogue() {
    let cases = [
        (1, 2, "locate the source of the curse"),
        (2, 3, "It was in the armory"),
        (3, 4, "through the left door"),
        (4, 5, "Good luck"),
    ];
    for (state, next_state, snippet) in cases {
        let mut world = World::default();
        world.map.tile_mut(12, 10).unwrap().light = 255;
        assert!(world.spawn_character(rouven_npc(1), 10, 10));
        assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

        world.tick = Tick(BASELINE_TICK);
        if let Some(rouven) = world.characters.get_mut(&CharacterId(1)) {
            rouven.push_driver_message(NT_CHAR, 2, 0, 0);
        }

        let events = world.process_rouven_actions(&facts(CharacterId(2), state, 1), 1);
        assert!(
            events.contains(&RouvenOutcomeEvent::UpdateRouvenState {
                player_id: CharacterId(2),
                new_state: next_state,
            }),
            "state {state} should advance to {next_state}"
        );
        let texts = world.drain_pending_area_texts();
        assert!(
            texts.iter().any(|text| text.message.contains(snippet)),
            "state {state} should speak {snippet:?}"
        );
    }
}

#[test]
fn state5_is_a_silent_no_op_waiting_for_the_skull() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rouven_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(rouven) = world.characters.get_mut(&CharacterId(1)) {
        rouven.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_rouven_actions(&facts(CharacterId(2), 5, 1), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state6_speaks_opens_quest63_and_advances_to_7() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rouven_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(rouven) = world.characters.get_mut(&CharacterId(1)) {
        rouven.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_rouven_actions(&facts(CharacterId(2), 6, 1), 1);
    assert!(events.contains(&RouvenOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 63,
    }));
    assert!(events.contains(&RouvenOutcomeEvent::UpdateRouvenState {
        player_id: CharacterId(2),
        new_state: 7,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("burrowed in from the underground")));
}

#[test]
fn states7_and_8_advance_with_dialogue() {
    let cases = [
        (7, 8, "retrieve the chronicles of Seyan"),
        (8, 9, "stored in the archives to the right"),
    ];
    for (state, next_state, snippet) in cases {
        let mut world = World::default();
        world.map.tile_mut(12, 10).unwrap().light = 255;
        assert!(world.spawn_character(rouven_npc(1), 10, 10));
        assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

        world.tick = Tick(BASELINE_TICK);
        if let Some(rouven) = world.characters.get_mut(&CharacterId(1)) {
            rouven.push_driver_message(NT_CHAR, 2, 0, 0);
        }

        let events = world.process_rouven_actions(&facts(CharacterId(2), state, 1), 1);
        assert!(events.contains(&RouvenOutcomeEvent::UpdateRouvenState {
            player_id: CharacterId(2),
            new_state: next_state,
        }));
        let texts = world.drain_pending_area_texts();
        assert!(texts.iter().any(|text| text.message.contains(snippet)));
    }
}

#[test]
fn state9_is_a_silent_no_op_waiting_for_the_chronicles() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rouven_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(rouven) = world.characters.get_mut(&CharacterId(1)) {
        rouven.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_rouven_actions(&facts(CharacterId(2), 9, 1), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn states10_and_11_advance_with_dialogue() {
    let cases = [
        (10, 11, "rebuilding efforts"),
        (11, 12, "emperor's personal vault"),
    ];
    for (state, next_state, snippet) in cases {
        let mut world = World::default();
        world.map.tile_mut(12, 10).unwrap().light = 255;
        assert!(world.spawn_character(rouven_npc(1), 10, 10));
        assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

        world.tick = Tick(BASELINE_TICK);
        if let Some(rouven) = world.characters.get_mut(&CharacterId(1)) {
            rouven.push_driver_message(NT_CHAR, 2, 0, 0);
        }

        let events = world.process_rouven_actions(&facts(CharacterId(2), state, 1), 1);
        assert!(events.contains(&RouvenOutcomeEvent::UpdateRouvenState {
            player_id: CharacterId(2),
            new_state: next_state,
        }));
        let texts = world.drain_pending_area_texts();
        assert!(texts.iter().any(|text| text.message.contains(snippet)));
    }
}

#[test]
fn state12_grants_vault_key_when_not_already_carried_and_advances_to_13() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rouven_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(rouven) = world.characters.get_mut(&CharacterId(1)) {
        rouven.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_rouven_actions(&facts(CharacterId(2), 12, 1), 1);
    assert!(events.contains(&RouvenOutcomeEvent::GrantVaultKey {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&RouvenOutcomeEvent::UpdateRouvenState {
        player_id: CharacterId(2),
        new_state: 13,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Take this key")));
}

#[test]
fn state12_does_not_grant_a_second_vault_key_when_already_carried() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rouven_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.inventory[0] = Some(ItemId(50));
    assert!(world.spawn_character(godmode, 12, 10));
    let mut key = item(50, ItemFlags::empty());
    key.template_id = IID_MAX_VAULTKEY;
    key.carried_by = Some(CharacterId(2));
    world.add_item(key);

    world.tick = Tick(BASELINE_TICK);
    if let Some(rouven) = world.characters.get_mut(&CharacterId(1)) {
        rouven.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_rouven_actions(&facts(CharacterId(2), 12, 1), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, RouvenOutcomeEvent::GrantVaultKey { .. })));
    assert!(events.contains(&RouvenOutcomeEvent::UpdateRouvenState {
        player_id: CharacterId(2),
        new_state: 13,
    }));
}

#[test]
fn state13_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rouven_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(rouven) = world.characters.get_mut(&CharacterId(1)) {
        rouven.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_rouven_actions(&facts(CharacterId(2), 13, 1), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn text_repeat_resets_state_within_disjoint_ranges() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rouven_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(rouven) = world.characters.get_mut(&CharacterId(1)) {
        rouven.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_rouven_actions(&facts(CharacterId(2), 8, 1), 1);
    assert!(events.contains(&RouvenOutcomeEvent::UpdateRouvenState {
        player_id: CharacterId(2),
        new_state: 6,
    }));

    if let Some(rouven) = world.characters.get_mut(&CharacterId(1)) {
        rouven.push_driver_text_message(CharacterId(2), "restart");
    }
    let events = world.process_rouven_actions(&facts(CharacterId(2), 12, 1), 1);
    assert!(events.contains(&RouvenOutcomeEvent::UpdateRouvenState {
        player_id: CharacterId(2),
        new_state: 10,
    }));

    if let Some(rouven) = world.characters.get_mut(&CharacterId(1)) {
        rouven.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_rouven_actions(&facts(CharacterId(2), 3, 1), 1);
    assert!(events.contains(&RouvenOutcomeEvent::UpdateRouvenState {
        player_id: CharacterId(2),
        new_state: 0,
    }));
}

#[test]
fn text_reset_me_has_no_handler_but_still_counts_as_didsay() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(rouven_npc(1), 10, 10));
    let mut god = player(2, "Godmode");
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 12, 10));

    if let Some(rouven) = world.characters.get_mut(&CharacterId(1)) {
        rouven.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_rouven_actions(&facts(CharacterId(2), 9, 1), 1);
    assert!(events.is_empty());
    assert_eq!(
        rouven_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn give_chronicles_in_range_completes_quest63_and_jumps_to_10() {
    let mut world = World::default();
    let mut rouven = rouven_npc(1);
    rouven.cursor_item = Some(ItemId(50));
    world.add_character(rouven);
    let mut chronicles = item(50, ItemFlags::empty());
    chronicles.name = "the chronicles of Seyan I".into();
    chronicles.template_id = IID_MAX_CHRONICLES;
    chronicles.carried_by = Some(CharacterId(1));
    world.add_item(chronicles);
    world.add_character(player(2, "Godmode"));

    if let Some(rouven) = world.characters.get_mut(&CharacterId(1)) {
        rouven.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_rouven_actions(&facts(CharacterId(2), 7, 1), 1);
    assert!(events.contains(&RouvenOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&RouvenOutcomeEvent::UpdateRouvenState {
        player_id: CharacterId(2),
        new_state: 10,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Thank you for the book")));
    assert!(world.items.get(&ItemId(50)).is_none());
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}

#[test]
fn give_chronicles_outside_range_is_handed_back() {
    let mut world = World::default();
    let mut rouven = rouven_npc(1);
    rouven.cursor_item = Some(ItemId(50));
    world.add_character(rouven);
    let mut chronicles = item(50, ItemFlags::empty());
    chronicles.template_id = IID_MAX_CHRONICLES;
    chronicles.carried_by = Some(CharacterId(1));
    world.add_item(chronicles);
    world.add_character(player(2, "Godmode"));

    if let Some(rouven) = world.characters.get_mut(&CharacterId(1)) {
        rouven.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // rouven_state == 12, outside the `6..=9` acceptance window.
    let events = world.process_rouven_actions(&facts(CharacterId(2), 12, 1), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, RouvenOutcomeEvent::QuestDone { .. })));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn give_ritual_points_the_player_back_to_carlos() {
    let mut world = World::default();
    let mut rouven = rouven_npc(1);
    rouven.cursor_item = Some(ItemId(50));
    world.add_character(rouven);
    let mut ritual = item(50, ItemFlags::empty());
    ritual.template_id = IID_MAX_RITUAL;
    ritual.carried_by = Some(CharacterId(1));
    world.add_item(ritual);
    world.add_character(player(2, "Godmode"));

    if let Some(rouven) = world.characters.get_mut(&CharacterId(1)) {
        rouven.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_rouven_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Please take the ritual to Carlos")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn give_unrelated_item_is_handed_back() {
    let mut world = World::default();
    let mut rouven = rouven_npc(1);
    rouven.cursor_item = Some(ItemId(50));
    world.add_character(rouven);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(rouven) = world.characters.get_mut(&CharacterId(1)) {
        rouven.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_rouven_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
