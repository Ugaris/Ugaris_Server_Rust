use std::collections::HashMap;

use super::*;
use crate::character_driver::{ForestHermitDriverData, CDR_FORESTHERMIT, NT_CHAR, NT_GIVE};
use crate::world::npc::area16::hermit::{ForestHermitOutcomeEvent, ForestHermitPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn hermit_npc(id: u32) -> Character {
    let mut hermit = character(id);
    hermit.name = "Hermit".into();
    hermit.driver = CDR_FORESTHERMIT;
    hermit.driver_state = Some(CharacterDriverState::ForestHermit(
        ForestHermitDriverData::default(),
    ));
    hermit
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    hermit_state: i32,
) -> HashMap<CharacterId, ForestHermitPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, ForestHermitPlayerFacts { hermit_state });
    map
}

fn hermit_state(world: &World, hermit_id: CharacterId) -> ForestHermitDriverData {
    match world
        .characters
        .get(&hermit_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::ForestHermit(data)) => data,
        _ => panic!("expected forest hermit driver state"),
    }
}

#[test]
fn hermit_greets_new_player_opens_quest_and_advances_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(hermit_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(hermit) = world.characters.get_mut(&CharacterId(1)) {
        hermit.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_hermit_actions(&facts(CharacterId(2), 0), 1);
    assert!(events.contains(&ForestHermitOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
    }));
    assert!(
        events.contains(&ForestHermitOutcomeEvent::UpdateHermitState {
            player_id: CharacterId(2),
            new_state: 1,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("most fortunate to see such a formidable hero")));
    assert_eq!(
        hermit_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn hermit_state4_is_a_silent_no_op_waiting_for_the_kill() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(hermit_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(hermit) = world.characters.get_mut(&CharacterId(1)) {
        hermit.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_hermit_actions(&facts(CharacterId(2), 4), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn hermit_state5_completes_quest_and_advances_to_state6() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(hermit_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(hermit) = world.characters.get_mut(&CharacterId(1)) {
        hermit.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_hermit_actions(&facts(CharacterId(2), 5), 1);
    assert!(
        events.contains(&ForestHermitOutcomeEvent::UpdateHermitState {
            player_id: CharacterId(2),
            new_state: 6,
        })
    );
    assert!(events.contains(&ForestHermitOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I thank thee")));
}

#[test]
fn hermit_text_repeat_resets_state_to_zero_when_at_or_below_four() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(hermit_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(hermit) = world.characters.get_mut(&CharacterId(1)) {
        hermit.driver_state = Some(CharacterDriverState::ForestHermit(ForestHermitDriverData {
            last_talk: 0,
            current_victim: None,
        }));
        hermit.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_forest_hermit_actions(&facts(CharacterId(2), 3), 1);
    assert!(
        events.contains(&ForestHermitOutcomeEvent::UpdateHermitState {
            player_id: CharacterId(2),
            new_state: 0,
        })
    );
}

#[test]
fn hermit_text_repeat_leaves_state_untouched_above_four() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(hermit_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(hermit) = world.characters.get_mut(&CharacterId(1)) {
        hermit.driver_state = Some(CharacterDriverState::ForestHermit(ForestHermitDriverData {
            last_talk: 0,
            current_victim: None,
        }));
        hermit.push_driver_text_message(CharacterId(2), "repeat");
    }

    // C's `case 2:` reset guard is `if (ppd->hermit_state <= 4)` only -
    // states 5-7 have no matching `else` branch, so they stay untouched.
    let events = world.process_forest_hermit_actions(&facts(CharacterId(2), 6), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, ForestHermitOutcomeEvent::UpdateHermitState { .. })));
}

#[test]
fn hermit_give_message_silently_destroys_any_item() {
    let mut world = World::default();
    let mut hermit = hermit_npc(1);
    hermit.cursor_item = Some(ItemId(50));
    world.add_character(hermit);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(hermit) = world.characters.get_mut(&CharacterId(1)) {
        hermit.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_forest_hermit_actions(&facts(CharacterId(2), 4), 1);
    assert!(events.is_empty());
    assert!(!world.items.contains_key(&ItemId(50)));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}
