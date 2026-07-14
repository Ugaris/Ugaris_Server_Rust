use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_DWARFSHAMAN, NT_CHAR, NT_GIVE};
use crate::item_driver::{IID_BROWNBERRY, IID_LIZARDTOOTH};
use crate::world::npc::area31::dwarfshaman::{
    DwarfShamanDriverData, DwarfshamanOutcomeEvent, DwarfshamanPlayerFacts,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn dwarfshaman_npc(id: u32) -> Character {
    let mut dwarfshaman = character(id);
    dwarfshaman.name = "Dwarven Shaman".into();
    dwarfshaman.driver = CDR_DWARFSHAMAN;
    dwarfshaman.driver_state = Some(CharacterDriverState::DwarfShaman(
        DwarfShamanDriverData::default(),
    ));
    dwarfshaman
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    dwarfshaman_state: i32,
    dwarfshaman_count: i32,
) -> HashMap<CharacterId, DwarfshamanPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        DwarfshamanPlayerFacts {
            dwarfshaman_state,
            dwarfshaman_count,
            quest52_is_done: false,
            quest53_is_done: false,
        },
    );
    map
}

#[test]
fn state0_greets_opens_quest51_and_advances_to_1() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(dwarfshaman_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(dwarfshaman) = world.characters.get_mut(&CharacterId(1)) {
        dwarfshaman.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_dwarfshaman_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.contains(&DwarfshamanOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 51,
    }));
    assert!(
        events.contains(&DwarfshamanOutcomeEvent::UpdateDwarfshamanState {
            player_id: CharacterId(2),
            new_state: 1,
        })
    );
}

#[test]
fn state2_is_a_silent_no_op_waiting_for_teeth() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(dwarfshaman_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(dwarfshaman) = world.characters.get_mut(&CharacterId(1)) {
        dwarfshaman.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_dwarfshaman_actions(&facts(CharacterId(2), 2, 0), 1);
    assert!(events.is_empty());
}

#[test]
fn giving_teeth_below_target_increments_count_and_reports_progress() {
    let mut world = World::default();
    let mut dwarfshaman = dwarfshaman_npc(1);
    dwarfshaman.cursor_item = Some(ItemId(50));
    world.add_character(dwarfshaman);
    let mut tooth = item(50, ItemFlags::empty());
    tooth.template_id = IID_LIZARDTOOTH;
    tooth.carried_by = Some(CharacterId(1));
    world.add_item(tooth);
    world.add_character(player(2, "Godmode"));

    if let Some(dwarfshaman) = world.characters.get_mut(&CharacterId(1)) {
        dwarfshaman.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_dwarfshaman_actions(&facts(CharacterId(2), 2, 3), 1);
    assert!(
        events.contains(&DwarfshamanOutcomeEvent::UpdateDwarfshamanCount {
            player_id: CharacterId(2),
            new_count: 4,
        })
    );
    assert!(!events
        .iter()
        .any(|event| matches!(event, DwarfshamanOutcomeEvent::QuestDone { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("4 done, 5 to go")));
    assert!(!world.items.contains_key(&ItemId(50)));
}

#[test]
fn giving_ninth_tooth_completes_quest51_and_resets_count() {
    let mut world = World::default();
    let mut dwarfshaman = dwarfshaman_npc(1);
    dwarfshaman.cursor_item = Some(ItemId(50));
    world.add_character(dwarfshaman);
    let mut tooth = item(50, ItemFlags::empty());
    tooth.template_id = IID_LIZARDTOOTH;
    tooth.carried_by = Some(CharacterId(1));
    world.add_item(tooth);
    world.add_character(player(2, "Godmode"));

    if let Some(dwarfshaman) = world.characters.get_mut(&CharacterId(1)) {
        dwarfshaman.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_dwarfshaman_actions(&facts(CharacterId(2), 2, 8), 1);
    assert!(
        events.contains(&DwarfshamanOutcomeEvent::UpdateDwarfshamanCount {
            player_id: CharacterId(2),
            new_count: 0,
        })
    );
    assert!(
        events.contains(&DwarfshamanOutcomeEvent::UpdateDwarfshamanState {
            player_id: CharacterId(2),
            new_state: 3,
        })
    );
    assert!(events.contains(&DwarfshamanOutcomeEvent::QuestDone {
        player_id: CharacterId(2),
        quest: 51,
    }));
}

#[test]
fn giving_berries_outside_the_acceptance_window_is_handed_back() {
    let mut world = World::default();
    let mut dwarfshaman = dwarfshaman_npc(1);
    dwarfshaman.cursor_item = Some(ItemId(50));
    world.add_character(dwarfshaman);
    let mut berry = item(50, ItemFlags::empty());
    berry.template_id = IID_BROWNBERRY;
    berry.carried_by = Some(CharacterId(1));
    world.add_item(berry);
    world.add_character(player(2, "Godmode"));

    if let Some(dwarfshaman) = world.characters.get_mut(&CharacterId(1)) {
        dwarfshaman.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // state 2 is outside the 3..=5 acceptance window for berries.
    let events = world.process_dwarfshaman_actions(&facts(CharacterId(2), 2, 0), 1);
    assert!(!events.iter().any(|event| matches!(
        event,
        DwarfshamanOutcomeEvent::UpdateDwarfshamanCount { .. }
    )));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn text_repeat_resets_to_the_current_mini_quests_start() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(dwarfshaman_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(dwarfshaman) = world.characters.get_mut(&CharacterId(1)) {
        dwarfshaman.push_driver_text_message(CharacterId(2), "repeat");
    }
    // state 7 sits in the 6..=8 range, so should reset to 6.
    let events = world.process_dwarfshaman_actions(&facts(CharacterId(2), 7, 0), 1);
    assert!(
        events.contains(&DwarfshamanOutcomeEvent::ResetToMiniQuestStart {
            player_id: CharacterId(2),
            new_state: 6,
        })
    );
}

#[test]
fn text_reset_me_wipes_state_and_count_for_gods_only() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(dwarfshaman_npc(1), 10, 10));
    let mut god = player(2, "Godmode");
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 12, 10));

    if let Some(dwarfshaman) = world.characters.get_mut(&CharacterId(1)) {
        dwarfshaman.push_driver_text_message(CharacterId(2), "reset me");
    }

    let events = world.process_dwarfshaman_actions(&facts(CharacterId(2), 4, 5), 1);
    assert!(events.contains(&DwarfshamanOutcomeEvent::ResetDwarfshaman {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("reset done")));
}
