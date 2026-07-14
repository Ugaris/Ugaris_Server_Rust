use std::collections::HashMap;

use super::*;
use crate::character_driver::{ForestWilliamDriverData, CDR_FORESTWILLIAM, NT_CHAR, NT_GIVE};
use crate::item_driver::IID_AREA16_MANTIS;
use crate::world::npc::area16::william::{ForestWilliamOutcomeEvent, ForestWilliamPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn william_npc(id: u32) -> Character {
    let mut william = character(id);
    william.name = "William".into();
    william.driver = CDR_FORESTWILLIAM;
    william.driver_state = Some(CharacterDriverState::ForestWilliam(
        ForestWilliamDriverData::default(),
    ));
    william
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    william_state: i32,
    quest22_done: bool,
    quest23_done: bool,
) -> HashMap<CharacterId, ForestWilliamPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        ForestWilliamPlayerFacts {
            william_state,
            quest22_done,
            quest23_done,
        },
    );
    map
}

fn william_state(world: &World, william_id: CharacterId) -> ForestWilliamDriverData {
    match world
        .characters
        .get(&william_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::ForestWilliam(data)) => data,
        _ => panic!("expected forest william driver state"),
    }
}

#[test]
fn william_greets_new_player_opens_quest22_and_advances_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(william_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(william) = world.characters.get_mut(&CharacterId(1)) {
        william.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_william_actions(&facts(CharacterId(2), 0, false, false), 1);
    assert!(events.contains(&ForestWilliamOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 22,
    }));
    assert!(
        events.contains(&ForestWilliamOutcomeEvent::UpdateWilliamState {
            player_id: CharacterId(2),
            new_state: 1,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("So nice of thee to visit")));
    assert_eq!(
        william_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn william_state0_with_quest22_already_done_says_greeting_but_skips_didsay() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(william_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(william) = world.characters.get_mut(&CharacterId(1)) {
        william.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    // C's `case 0` says the greeting *before* checking `questlog_isdone`,
    // so the message still fires even though the quest is already done -
    // but `didsay` is never set on this branch, so `current_victim` stays
    // unset.
    let events = world.process_forest_william_actions(&facts(CharacterId(2), 0, true, false), 1);
    assert!(
        events.contains(&ForestWilliamOutcomeEvent::UpdateWilliamState {
            player_id: CharacterId(2),
            new_state: 3,
        })
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, ForestWilliamOutcomeEvent::QuestOpen { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("So nice of thee to visit")));
    assert_eq!(william_state(&world, CharacterId(1)).current_victim, None);
}

#[test]
fn william_state3_with_quest23_already_done_says_nothing_and_skips_to_state7() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(william_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(william) = world.characters.get_mut(&CharacterId(1)) {
        william.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_william_actions(&facts(CharacterId(2), 3, true, true), 1);
    assert!(
        events.contains(&ForestWilliamOutcomeEvent::UpdateWilliamState {
            player_id: CharacterId(2),
            new_state: 7,
        })
    );
    assert!(world.drain_pending_area_texts().is_empty());
    assert_eq!(william_state(&world, CharacterId(1)).current_victim, None);
}

#[test]
fn william_state3_with_quest23_pending_greets_and_opens_quest23() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(william_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(william) = world.characters.get_mut(&CharacterId(1)) {
        william.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_forest_william_actions(&facts(CharacterId(2), 3, true, false), 1);
    assert!(events.contains(&ForestWilliamOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 23,
    }));
    assert!(
        events.contains(&ForestWilliamOutcomeEvent::UpdateWilliamState {
            player_id: CharacterId(2),
            new_state: 4,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("done him a favor")));
}

#[test]
fn william_receiving_mantis_at_state6_completes_quest_and_advances_imp() {
    let mut world = World::default();
    let mut william = william_npc(1);
    william.cursor_item = Some(ItemId(50));
    world.add_character(william);
    let mut mantis = item(50, ItemFlags::empty());
    mantis.name = "Praying Mantis".into();
    mantis.template_id = IID_AREA16_MANTIS;
    mantis.carried_by = Some(CharacterId(1));
    world.add_item(mantis);
    world.add_character(player(2, "Godmode"));

    if let Some(william) = world.characters.get_mut(&CharacterId(1)) {
        william.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_forest_william_actions(&facts(CharacterId(2), 6, true, false), 1);
    assert!(
        events.contains(&ForestWilliamOutcomeEvent::UpdateWilliamState {
            player_id: CharacterId(2),
            new_state: 7,
        })
    );
    assert!(events.contains(&ForestWilliamOutcomeEvent::UpdateImpState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
    assert!(
        events.contains(&ForestWilliamOutcomeEvent::QuestDoneMantis {
            player_id: CharacterId(2),
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("nice stew")));
    assert!(!world.items.contains_key(&ItemId(50)));
}

#[test]
fn william_give_other_item_hands_it_back_to_giver() {
    let mut world = World::default();
    let mut william = william_npc(1);
    william.cursor_item = Some(ItemId(50));
    world.add_character(william);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(william) = world.characters.get_mut(&CharacterId(1)) {
        william.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_forest_william_actions(&facts(CharacterId(2), 6, true, false), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
