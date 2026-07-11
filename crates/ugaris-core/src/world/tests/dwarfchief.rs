use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_DWARFCHIEF, NT_CHAR, NT_GIVE};
use crate::item_driver::IID_DWARFRECALL1;
use crate::world::npc::area31::dwarfchief::{
    DwarfChiefDriverData, DwarfRecallScroll, DwarfchiefOutcomeEvent, DwarfchiefPlayerFacts,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn dwarfchief_npc(id: u32) -> Character {
    let mut dwarfchief = character(id);
    dwarfchief.name = "Dwarven Chief".into();
    dwarfchief.driver = CDR_DWARFCHIEF;
    dwarfchief.driver_state = Some(CharacterDriverState::DwarfChief(
        DwarfChiefDriverData::default(),
    ));
    dwarfchief
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    dwarfchief_state: i32,
) -> HashMap<CharacterId, DwarfchiefPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        DwarfchiefPlayerFacts {
            dwarfchief_state,
            quest48_is_done: false,
            quest49_is_done: false,
            quest50_is_done: false,
        },
    );
    map
}

#[test]
fn state0_greets_opens_quest47_and_advances_to_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(dwarfchief_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(dwarfchief) = world.characters.get_mut(&CharacterId(1)) {
        dwarfchief.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_dwarfchief_actions(&facts(CharacterId(2), 0), 1);
    assert!(events.contains(&DwarfchiefOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 47,
    }));
    assert!(
        events.contains(&DwarfchiefOutcomeEvent::UpdateDwarfchiefState {
            player_id: CharacterId(2),
            new_state: 1,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("home of the dwarves")));
}

#[test]
fn state2_grants_first_recall_scroll_unless_already_carried() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(dwarfchief_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(dwarfchief) = world.characters.get_mut(&CharacterId(1)) {
        dwarfchief.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_dwarfchief_actions(&facts(CharacterId(2), 2), 1);
    assert!(events.contains(&DwarfchiefOutcomeEvent::GrantRecallScroll {
        player_id: CharacterId(2),
        scroll: DwarfRecallScroll::Recall90,
    }));
    assert!(
        events.contains(&DwarfchiefOutcomeEvent::UpdateDwarfchiefState {
            player_id: CharacterId(2),
            new_state: 3,
        })
    );
}

#[test]
fn state2_does_not_regrant_scroll_when_already_carried() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(dwarfchief_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.inventory[30] = Some(ItemId(50));
    world.spawn_character(godmode, 12, 10);
    let mut scroll = item(50, ItemFlags::empty());
    scroll.template_id = IID_DWARFRECALL1;
    scroll.carried_by = Some(CharacterId(2));
    world.add_item(scroll);

    world.tick = Tick(BASELINE_TICK);
    if let Some(dwarfchief) = world.characters.get_mut(&CharacterId(1)) {
        dwarfchief.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_dwarfchief_actions(&facts(CharacterId(2), 2), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, DwarfchiefOutcomeEvent::GrantRecallScroll { .. })));
}

#[test]
fn state3_is_a_silent_no_op_waiting_for_first_miner() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(dwarfchief_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(dwarfchief) = world.characters.get_mut(&CharacterId(1)) {
        dwarfchief.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_dwarfchief_actions(&facts(CharacterId(2), 3), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state4_completes_quest47_and_opens_quest48_when_not_fast_forwarding() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(dwarfchief_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(dwarfchief) = world.characters.get_mut(&CharacterId(1)) {
        dwarfchief.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_dwarfchief_actions(&facts(CharacterId(2), 4), 1);
    assert!(events.contains(&DwarfchiefOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest: 47,
    }));
    assert!(events.contains(&DwarfchiefOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 48,
    }));
    assert!(
        events.contains(&DwarfchiefOutcomeEvent::UpdateDwarfchiefState {
            player_id: CharacterId(2),
            new_state: 6,
        })
    );
}

#[test]
fn state4_fast_forwards_to_8_when_quest48_already_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(dwarfchief_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(dwarfchief) = world.characters.get_mut(&CharacterId(1)) {
        dwarfchief.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let mut facts_map = HashMap::new();
    facts_map.insert(
        CharacterId(2),
        DwarfchiefPlayerFacts {
            dwarfchief_state: 4,
            quest48_is_done: true,
            quest49_is_done: false,
            quest50_is_done: false,
        },
    );
    let events = world.process_dwarfchief_actions(&facts_map, 1);
    assert!(events.contains(&DwarfchiefOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest: 47,
    }));
    assert!(
        events.contains(&DwarfchiefOutcomeEvent::UpdateDwarfchiefState {
            player_id: CharacterId(2),
            new_state: 8,
        })
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, DwarfchiefOutcomeEvent::QuestOpen { quest: 48, .. })));
}

#[test]
fn state13_completes_quest50_and_says_final_thanks() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(dwarfchief_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(dwarfchief) = world.characters.get_mut(&CharacterId(1)) {
        dwarfchief.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_dwarfchief_actions(&facts(CharacterId(2), 13), 1);
    assert!(events.contains(&DwarfchiefOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest: 50,
    }));
    assert!(
        events.contains(&DwarfchiefOutcomeEvent::UpdateDwarfchiefState {
            player_id: CharacterId(2),
            new_state: 15,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Thank you for saving the last one")));
}

#[test]
fn text_repeat_resets_to_the_current_mini_quests_start() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(dwarfchief_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(dwarfchief) = world.characters.get_mut(&CharacterId(1)) {
        dwarfchief.push_driver_text_message(CharacterId(2), "repeat");
    }
    // state 9 sits in the 8..=9 range, so should reset to 8.
    let events = world.process_dwarfchief_actions(&facts(CharacterId(2), 9), 1);
    assert!(
        events.contains(&DwarfchiefOutcomeEvent::ResetToMiniQuestStart {
            player_id: CharacterId(2),
            new_state: 8,
        })
    );
}

#[test]
fn text_reset_me_speaks_reset_done_and_pushes_reset_event_for_gods_only() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(dwarfchief_npc(1), 10, 10));
    let mut god = player(2, "Godmode");
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 12, 10));

    if let Some(dwarfchief) = world.characters.get_mut(&CharacterId(1)) {
        dwarfchief.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_dwarfchief_actions(&facts(CharacterId(2), 3), 1);
    assert!(events.contains(&DwarfchiefOutcomeEvent::ResetDwarfchief {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("reset done")));
}

#[test]
fn text_reset_me_is_ignored_for_non_gods() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(dwarfchief_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(dwarfchief) = world.characters.get_mut(&CharacterId(1)) {
        dwarfchief.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_dwarfchief_actions(&facts(CharacterId(2), 3), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn give_any_item_is_always_handed_back() {
    let mut world = World::default();
    let mut dwarfchief = dwarfchief_npc(1);
    dwarfchief.cursor_item = Some(ItemId(50));
    world.add_character(dwarfchief);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(dwarfchief) = world.characters.get_mut(&CharacterId(1)) {
        dwarfchief.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_dwarfchief_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
