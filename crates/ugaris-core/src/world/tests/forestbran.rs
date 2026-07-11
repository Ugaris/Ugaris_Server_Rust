use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_FORESTBRAN, NT_CHAR, NT_GIVE};
use crate::item_driver::IID_STAFF_FORESTMAP;
use crate::world::npc::area29::forestbran::{
    ForestBranDriverData, ForestBranOutcomeEvent, ForestBranPlayerFacts,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn forestbran_npc(id: u32) -> Character {
    let mut forestbran = character(id);
    forestbran.name = "Forester Brannington".into();
    forestbran.driver = CDR_FORESTBRAN;
    forestbran.driver_state = Some(CharacterDriverState::ForestBran(
        ForestBranDriverData::default(),
    ));
    forestbran
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    forestbran_state: i32,
    forestbran_done: u8,
) -> HashMap<CharacterId, ForestBranPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        ForestBranPlayerFacts {
            forestbran_state,
            forestbran_done,
        },
    );
    map
}

#[test]
fn state0_greets_and_advances_to_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(forestbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(forestbran) = world.characters.get_mut(&CharacterId(1)) {
        forestbran.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forestbran_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(
        events.contains(&ForestBranOutcomeEvent::UpdateForestBranState {
            player_id: CharacterId(2),
            new_state: 1,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Welcome Godmode, how are you today?")));
}

#[test]
fn states1_through_3_advance_one_state_each_with_dialogue() {
    let cases = [
        (1, 2, "thought I might tell you something"),
        (2, 3, "thief mages talking about maps"),
        (3, 4, "I'll tell you where to dig"),
    ];
    for (state, next_state, snippet) in cases {
        let mut world = World::default();
        world.map.tile_mut(12, 10).unwrap().light = 255;
        assert!(world.spawn_character(forestbran_npc(1), 10, 10));
        assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

        world.tick = Tick(BASELINE_TICK);
        if let Some(forestbran) = world.characters.get_mut(&CharacterId(1)) {
            forestbran.push_driver_message(NT_CHAR, 2, 0, 0);
        }

        let events = world.process_forestbran_actions(&facts(CharacterId(2), state, 0), 1);
        assert!(
            events.contains(&ForestBranOutcomeEvent::UpdateForestBranState {
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
fn state4_is_a_silent_no_op_waiting_for_maps() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(forestbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(forestbran) = world.characters.get_mut(&CharacterId(1)) {
        forestbran.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forestbran_actions(&facts(CharacterId(2), 4, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn text_repeat_resets_state_to_0_when_not_yet_past_state4() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(forestbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(forestbran) = world.characters.get_mut(&CharacterId(1)) {
        forestbran.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_forestbran_actions(&facts(CharacterId(2), 3, 0), 1);
    assert!(
        events.contains(&ForestBranOutcomeEvent::UpdateForestBranState {
            player_id: CharacterId(2),
            new_state: 0,
        })
    );
}

#[test]
fn text_repeat_does_not_reset_once_past_state4() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(forestbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(forestbran) = world.characters.get_mut(&CharacterId(1)) {
        forestbran.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_forestbran_actions(&facts(CharacterId(2), 5, 0), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, ForestBranOutcomeEvent::UpdateForestBranState { .. })));
}

#[test]
fn text_reset_me_speaks_reset_done_and_pushes_reset_event_for_gods_only() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(forestbran_npc(1), 10, 10));
    let mut god = player(2, "Godmode");
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 12, 10));

    if let Some(forestbran) = world.characters.get_mut(&CharacterId(1)) {
        forestbran.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_forestbran_actions(&facts(CharacterId(2), 3, 2), 1);
    assert!(events.contains(&ForestBranOutcomeEvent::ResetForestBran {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("reset done")));
}

#[test]
fn text_reset_me_is_ignored_for_non_gods() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(forestbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(forestbran) = world.characters.get_mut(&CharacterId(1)) {
        forestbran.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_forestbran_actions(&facts(CharacterId(2), 3, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn give_forestmap_speaks_hint_for_current_forestbran_done_and_destroys_map() {
    // Each `forestbran_done` value 0..=4 maps to a distinct dig-location
    // hint; the map is always destroyed regardless.
    let cases = [
        (0u8, "beneath a dead tree"),
        (1, "under the heat of a fire"),
        (2, "next to an empty bucket"),
        (3, "inside a circle of stones"),
        (4, "next to a pair of bags"),
        (5, "found all the treasures"),
    ];
    for (forestbran_done, snippet) in cases {
        let mut world = World::default();
        let mut forestbran = forestbran_npc(1);
        forestbran.cursor_item = Some(ItemId(50));
        world.add_character(forestbran);
        let mut map_item = item(50, ItemFlags::empty());
        map_item.name = "a treasure map".into();
        map_item.template_id = IID_STAFF_FORESTMAP;
        map_item.carried_by = Some(CharacterId(1));
        world.add_item(map_item);
        world.add_character(player(2, "Godmode"));

        if let Some(forestbran) = world.characters.get_mut(&CharacterId(1)) {
            forestbran.push_driver_message(NT_GIVE, 2, 50, 0);
        }

        let events =
            world.process_forestbran_actions(&facts(CharacterId(2), 4, forestbran_done), 1);
        assert!(
            events.is_empty(),
            "forestbran_done {forestbran_done} should not push events"
        );
        let texts = world.drain_pending_area_texts();
        assert!(
            texts.iter().any(|text| text.message.contains(snippet)),
            "forestbran_done {forestbran_done} should speak {snippet:?}"
        );
        assert!(
            world.items.get(&ItemId(50)).is_none(),
            "forestbran_done {forestbran_done} should always destroy the map"
        );
        assert!(world
            .characters
            .get(&CharacterId(1))
            .unwrap()
            .cursor_item
            .is_none());
    }
}

#[test]
fn give_unrelated_item_is_handed_back() {
    let mut world = World::default();
    let mut forestbran = forestbran_npc(1);
    forestbran.cursor_item = Some(ItemId(50));
    world.add_character(forestbran);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(forestbran) = world.characters.get_mut(&CharacterId(1)) {
        forestbran.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_forestbran_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn give_forestmap_from_nonplayer_is_handed_back() {
    let mut world = World::default();
    let mut forestbran = forestbran_npc(1);
    forestbran.cursor_item = Some(ItemId(50));
    world.add_character(forestbran);
    let mut map_item = item(50, ItemFlags::empty());
    map_item.template_id = IID_STAFF_FORESTMAP;
    map_item.carried_by = Some(CharacterId(1));
    world.add_item(map_item);
    // Non-player giver: no `CharacterFlags::PLAYER`.
    world.add_character(character(2));

    if let Some(forestbran) = world.characters.get_mut(&CharacterId(1)) {
        forestbran.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_forestbran_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let giver = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(giver.cursor_item, Some(ItemId(50)));
}
