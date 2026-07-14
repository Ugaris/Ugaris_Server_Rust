use std::collections::HashMap;

use super::*;
use crate::character_driver::{AristocratDriverData, CDR_ARISTOCRAT, NT_CHAR, NT_GIVE};
use crate::item_driver::{IID_STAFF_ARIAMULET, IID_STAFF_ARIKEY};
use crate::world::npc::area28::aristocrat::{AristocratOutcomeEvent, AristocratPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn aristocrat_npc(id: u32) -> Character {
    let mut aristocrat = character(id);
    aristocrat.name = "Aristocrat".into();
    aristocrat.driver = CDR_ARISTOCRAT;
    aristocrat.driver_state = Some(CharacterDriverState::Aristocrat(
        AristocratDriverData::default(),
    ));
    aristocrat
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    aristocrat_state: i32,
) -> HashMap<CharacterId, AristocratPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, AristocratPlayerFacts { aristocrat_state });
    map
}

fn aristocrat_state(world: &World, aristocrat_id: CharacterId) -> AristocratDriverData {
    match world
        .characters
        .get(&aristocrat_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Aristocrat(data)) => data,
        _ => panic!("expected aristocrat driver state"),
    }
}

#[test]
fn state0_greets_opens_quest38_and_advances_to_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(aristocrat_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(aristocrat) = world.characters.get_mut(&CharacterId(1)) {
        aristocrat.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_aristocrat_actions(&facts(CharacterId(2), 0), 1);
    assert!(events.contains(&AristocratOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
    }));
    assert!(
        events.contains(&AristocratOutcomeEvent::UpdateAristocratState {
            player_id: CharacterId(2),
            new_state: 1,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Greetings stranger!")));
    assert_eq!(
        aristocrat_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn states1_through_6_advance_one_state_each_with_dialogue() {
    let cases = [
        (1, 2, "buoyant adventurer"),
        (2, 3, "don't growl at me"),
        (3, 4, "large lake north of here"),
        (4, 5, "lurched out of the water"),
        (5, 6, "my Amulet was lost"),
        (6, 7, "reward you well"),
    ];
    for (state, next_state, snippet) in cases {
        let mut world = World::default();
        world.map.tile_mut(12, 10).unwrap().light = 255;
        assert!(world.spawn_character(aristocrat_npc(1), 10, 10));
        assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

        world.tick = Tick(BASELINE_TICK);
        if let Some(aristocrat) = world.characters.get_mut(&CharacterId(1)) {
            aristocrat.push_driver_message(NT_CHAR, 2, 0, 0);
        }

        let events = world.process_aristocrat_actions(&facts(CharacterId(2), state), 1);
        assert!(
            events.contains(&AristocratOutcomeEvent::UpdateAristocratState {
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
fn state7_is_a_silent_no_op_waiting_for_the_amulet() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(aristocrat_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(aristocrat) = world.characters.get_mut(&CharacterId(1)) {
        aristocrat.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_aristocrat_actions(&facts(CharacterId(2), 7), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state8_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(aristocrat_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(aristocrat) = world.characters.get_mut(&CharacterId(1)) {
        aristocrat.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_aristocrat_actions(&facts(CharacterId(2), 8), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn text_repeat_resets_state_to_0_when_not_yet_past_state7() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(aristocrat_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(aristocrat) = world.characters.get_mut(&CharacterId(1)) {
        aristocrat.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_aristocrat_actions(&facts(CharacterId(2), 4), 1);
    assert!(
        events.contains(&AristocratOutcomeEvent::UpdateAristocratState {
            player_id: CharacterId(2),
            new_state: 0,
        })
    );
}

#[test]
fn text_repeat_does_not_reset_once_past_state7() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(aristocrat_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(aristocrat) = world.characters.get_mut(&CharacterId(1)) {
        aristocrat.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_aristocrat_actions(&facts(CharacterId(2), 8), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, AristocratOutcomeEvent::UpdateAristocratState { .. })));
}

#[test]
fn text_reset_me_speaks_reset_done_and_pushes_reset_event_for_gods_only() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(aristocrat_npc(1), 10, 10));
    let mut god = player(2, "Godmode");
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 12, 10));

    if let Some(aristocrat) = world.characters.get_mut(&CharacterId(1)) {
        aristocrat.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_aristocrat_actions(&facts(CharacterId(2), 4), 1);
    assert!(events.contains(&AristocratOutcomeEvent::ResetAristocrat {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("reset done")));
}

#[test]
fn text_reset_me_is_ignored_for_non_gods() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(aristocrat_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(aristocrat) = world.characters.get_mut(&CharacterId(1)) {
        aristocrat.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_aristocrat_actions(&facts(CharacterId(2), 4), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn give_amulet_in_range_completes_quest38_destroys_arikey_and_jumps_to_8() {
    let mut world = World::default();
    let mut aristocrat = aristocrat_npc(1);
    aristocrat.cursor_item = Some(ItemId(50));
    world.add_character(aristocrat);
    let mut amulet = item(50, ItemFlags::empty());
    amulet.name = "The Family Amulet".into();
    amulet.template_id = IID_STAFF_ARIAMULET;
    amulet.carried_by = Some(CharacterId(1));
    world.add_item(amulet);
    let mut godmode = player(2, "Godmode");
    godmode.inventory[0] = Some(ItemId(51));
    world.add_character(godmode);
    let mut key = item(51, ItemFlags::empty());
    key.template_id = IID_STAFF_ARIKEY;
    key.carried_by = Some(CharacterId(2));
    world.add_item(key);

    if let Some(aristocrat) = world.characters.get_mut(&CharacterId(1)) {
        aristocrat.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_aristocrat_actions(&facts(CharacterId(2), 7), 1);
    assert!(events.contains(&AristocratOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
    }));
    assert!(
        events.contains(&AristocratOutcomeEvent::UpdateAristocratState {
            player_id: CharacterId(2),
            new_state: 8,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Please accept this reward")));
    assert!(!world.items.contains_key(&ItemId(50)));
    assert!(!world.items.contains_key(&ItemId(51)));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}

#[test]
fn give_amulet_outside_range_is_handed_back() {
    let mut world = World::default();
    let mut aristocrat = aristocrat_npc(1);
    aristocrat.cursor_item = Some(ItemId(50));
    world.add_character(aristocrat);
    let mut amulet = item(50, ItemFlags::empty());
    amulet.template_id = IID_STAFF_ARIAMULET;
    amulet.carried_by = Some(CharacterId(1));
    world.add_item(amulet);
    world.add_character(player(2, "Godmode"));

    if let Some(aristocrat) = world.characters.get_mut(&CharacterId(1)) {
        aristocrat.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // aristocrat_state == 8, outside the `<= 7` acceptance window.
    let events = world.process_aristocrat_actions(&facts(CharacterId(2), 8), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, AristocratOutcomeEvent::QuestDone { .. })));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn give_unrelated_item_is_handed_back() {
    let mut world = World::default();
    let mut aristocrat = aristocrat_npc(1);
    aristocrat.cursor_item = Some(ItemId(50));
    world.add_character(aristocrat);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(aristocrat) = world.characters.get_mut(&CharacterId(1)) {
        aristocrat.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_aristocrat_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
