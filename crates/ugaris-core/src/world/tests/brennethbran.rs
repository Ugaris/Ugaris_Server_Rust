use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_BRENNETHBRAN, NT_CHAR, NT_GIVE};
use crate::item_driver::{IID_STAFF_BRENNETHDAGGER, IID_STAFF_BRENNETHJOURNAL};
use crate::world::npc::area29::brennethbran::{
    BrennethBranDriverData, BrennethBranOutcomeEvent, BrennethBranPlayerFacts,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn brennethbran_npc(id: u32) -> Character {
    let mut brenneth = character(id);
    brenneth.name = "Brenneth Brannington".into();
    brenneth.driver = CDR_BRENNETHBRAN;
    brenneth.driver_state = Some(CharacterDriverState::BrennethBran(
        BrennethBranDriverData::default(),
    ));
    brenneth
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    brennethbran_state: i32,
) -> HashMap<CharacterId, BrennethBranPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        BrennethBranPlayerFacts {
            brennethbran_state,
            quest42_is_done: false,
            quest43_is_done: false,
        },
    );
    map
}

#[test]
fn state0_greets_opens_quest41_and_advances_to_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brennethbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(brenneth) = world.characters.get_mut(&CharacterId(1)) {
        brenneth.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_brennethbran_actions(&facts(CharacterId(2), 0), 1);
    assert!(events.contains(&BrennethBranOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 41,
    }));
    assert!(
        events.contains(&BrennethBranOutcomeEvent::UpdateBrennethBranState {
            player_id: CharacterId(2),
            new_state: 1,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I can't recall much")));
}

#[test]
fn state4_is_a_silent_no_op_waiting_for_the_dagger() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brennethbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(brenneth) = world.characters.get_mut(&CharacterId(1)) {
        brenneth.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_brennethbran_actions(&facts(CharacterId(2), 4), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state5_opens_quest42_and_advances_to_6_when_quest42_not_already_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brennethbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(brenneth) = world.characters.get_mut(&CharacterId(1)) {
        brenneth.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_brennethbran_actions(&facts(CharacterId(2), 5), 1);
    assert!(events.contains(&BrennethBranOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 42,
    }));
    assert!(
        events.contains(&BrennethBranOutcomeEvent::UpdateBrennethBranState {
            player_id: CharacterId(2),
            new_state: 6,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text
        .message
        .contains("don't recall anything of being a fighter")));
}

#[test]
fn state5_fast_forwards_to_9_when_quest42_already_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brennethbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(brenneth) = world.characters.get_mut(&CharacterId(1)) {
        brenneth.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let mut facts_map = HashMap::new();
    facts_map.insert(
        CharacterId(2),
        BrennethBranPlayerFacts {
            brennethbran_state: 5,
            quest42_is_done: true,
            quest43_is_done: false,
        },
    );
    let events = world.process_brennethbran_actions(&facts_map, 1);
    assert!(
        events.contains(&BrennethBranOutcomeEvent::UpdateBrennethBranState {
            player_id: CharacterId(2),
            new_state: 9,
        })
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, BrennethBranOutcomeEvent::QuestOpen { .. })));
    // No dialogue for the fast-forward path.
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state14_thanks_and_emotes_advancing_to_15() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brennethbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(brenneth) = world.characters.get_mut(&CharacterId(1)) {
        brenneth.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_brennethbran_actions(&facts(CharacterId(2), 14), 1);
    assert!(
        events.contains(&BrennethBranOutcomeEvent::UpdateBrennethBranState {
            player_id: CharacterId(2),
            new_state: 15,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Thank you for helping me")));
}

#[test]
fn text_repeat_resets_to_the_current_mini_quests_start() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brennethbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(brenneth) = world.characters.get_mut(&CharacterId(1)) {
        brenneth.push_driver_text_message(CharacterId(2), "repeat");
    }
    // state 7 sits in the 5..=8 range, so should reset to 5.
    let events = world.process_brennethbran_actions(&facts(CharacterId(2), 7), 1);
    assert!(
        events.contains(&BrennethBranOutcomeEvent::ResetToMiniQuestStart {
            player_id: CharacterId(2),
            new_state: 5,
        })
    );
}

#[test]
fn text_reset_me_speaks_reset_done_and_pushes_reset_event_for_gods_only() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brennethbran_npc(1), 10, 10));
    let mut god = player(2, "Godmode");
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 12, 10));

    if let Some(brenneth) = world.characters.get_mut(&CharacterId(1)) {
        brenneth.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_brennethbran_actions(&facts(CharacterId(2), 3), 1);
    assert!(
        events.contains(&BrennethBranOutcomeEvent::ResetBrennethBran {
            player_id: CharacterId(2),
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("reset done")));
}

#[test]
fn text_reset_me_is_ignored_for_non_gods() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(brennethbran_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(brenneth) = world.characters.get_mut(&CharacterId(1)) {
        brenneth.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_brennethbran_actions(&facts(CharacterId(2), 3), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn give_dagger_at_state4_completes_quest41_and_jumps_to_5() {
    let mut world = World::default();
    let mut brenneth = brennethbran_npc(1);
    brenneth.cursor_item = Some(ItemId(50));
    world.add_character(brenneth);
    let mut dagger = item(50, ItemFlags::empty());
    dagger.name = "Brenneth's Dagger".into();
    dagger.template_id = IID_STAFF_BRENNETHDAGGER;
    dagger.carried_by = Some(CharacterId(1));
    world.add_item(dagger);
    world.add_character(player(2, "Godmode"));

    if let Some(brenneth) = world.characters.get_mut(&CharacterId(1)) {
        brenneth.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_brennethbran_actions(&facts(CharacterId(2), 4), 1);
    assert!(events.contains(&BrennethBranOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest: 41,
    }));
    assert!(
        events.contains(&BrennethBranOutcomeEvent::UpdateBrennethBranState {
            player_id: CharacterId(2),
            new_state: 5,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("so this was my dagger")));
    assert!(!world.items.contains_key(&ItemId(50)));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}

#[test]
fn give_journal_outside_acceptance_window_is_handed_back() {
    let mut world = World::default();
    let mut brenneth = brennethbran_npc(1);
    brenneth.cursor_item = Some(ItemId(50));
    world.add_character(brenneth);
    let mut journal = item(50, ItemFlags::empty());
    journal.name = "Brenneth's Journal".into();
    journal.template_id = IID_STAFF_BRENNETHJOURNAL;
    journal.carried_by = Some(CharacterId(1));
    world.add_item(journal);
    world.add_character(player(2, "Godmode"));

    if let Some(brenneth) = world.characters.get_mut(&CharacterId(1)) {
        brenneth.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // state 4 is outside the 9..=11 acceptance window for the journal.
    let events = world.process_brennethbran_actions(&facts(CharacterId(2), 4), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, BrennethBranOutcomeEvent::QuestDone { .. })));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn give_unrelated_item_is_handed_back() {
    let mut world = World::default();
    let mut brenneth = brennethbran_npc(1);
    brenneth.cursor_item = Some(ItemId(50));
    world.add_character(brenneth);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(brenneth) = world.characters.get_mut(&CharacterId(1)) {
        brenneth.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_brennethbran_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
