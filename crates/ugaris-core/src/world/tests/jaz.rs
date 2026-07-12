use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_JAZ, NT_CHAR, NT_GIVE};
use crate::item_driver::IID_ARKHATA_BRACELET;
use crate::world::npc::area37::jaz::{JazDriverData, JazOutcomeEvent, JazPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn jaz_npc(id: u32) -> Character {
    let mut jaz = character(id);
    jaz.name = "Jaz".into();
    jaz.driver = CDR_JAZ;
    jaz.driver_state = Some(CharacterDriverState::Jaz(JazDriverData::default()));
    jaz
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    jaz_state: i32,
    rammy_state: i32,
) -> HashMap<CharacterId, JazPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        JazPlayerFacts {
            jaz_state,
            rammy_state,
        },
    );
    map
}

#[test]
fn state0_without_rammy_progress_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jaz_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jaz) = world.characters.get_mut(&CharacterId(1)) {
        jaz.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jaz_actions(&facts(CharacterId(2), 0, 11), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state0_with_rammy_progress_greets_opens_quest66_and_collapses_to_2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jaz_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jaz) = world.characters.get_mut(&CharacterId(1)) {
        jaz.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jaz_actions(&facts(CharacterId(2), 0, 12), 1);
    assert!(events.contains(&JazOutcomeEvent::QuestOpen66 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&JazOutcomeEvent::UpdateJazState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Welcome to my home")));
}

#[test]
fn state5_is_a_silent_no_op_waiting_for_the_bracelet() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jaz_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jaz) = world.characters.get_mut(&CharacterId(1)) {
        jaz.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jaz_actions(&facts(CharacterId(2), 5, 12), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state6_thanks_male_player_as_brother_and_advances_to_7() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jaz_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.flags |= CharacterFlags::MALE;
    assert!(world.spawn_character(godmode, 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jaz) = world.characters.get_mut(&CharacterId(1)) {
        jaz.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jaz_actions(&facts(CharacterId(2), 6, 12), 1);
    assert!(events.contains(&JazOutcomeEvent::UpdateJazState {
        player_id: CharacterId(2),
        new_state: 7,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("brother")));
}

#[test]
fn state6_thanks_female_player_as_sister() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jaz_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jaz) = world.characters.get_mut(&CharacterId(1)) {
        jaz.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jaz_actions(&facts(CharacterId(2), 6, 12), 1);
    assert!(events.contains(&JazOutcomeEvent::UpdateJazState {
        player_id: CharacterId(2),
        new_state: 7,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("sister")));
}

#[test]
fn state7_is_a_silent_no_op_all_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jaz_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(jaz) = world.characters.get_mut(&CharacterId(1)) {
        jaz.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_jaz_actions(&facts(CharacterId(2), 7, 12), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn give_bracelet_at_state5_completes_quest66_silently_and_jumps_to_6() {
    let mut world = World::default();
    let mut jaz = jaz_npc(1);
    jaz.cursor_item = Some(ItemId(50));
    world.add_character(jaz);
    let mut bracelet = item(50, ItemFlags::empty());
    bracelet.name = "Ishtar's Bracelet".into();
    bracelet.template_id = IID_ARKHATA_BRACELET;
    bracelet.carried_by = Some(CharacterId(1));
    world.add_item(bracelet);
    let godmode = player(2, "Godmode");
    world.add_character(godmode);

    if let Some(jaz) = world.characters.get_mut(&CharacterId(1)) {
        jaz.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_jaz_actions(&facts(CharacterId(2), 5, 12), 1);
    assert!(events.contains(&JazOutcomeEvent::QuestDone66 {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&JazOutcomeEvent::UpdateJazState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
    // C's bracelet turn-in is silent - no dialogue at all.
    assert!(world.drain_pending_area_texts().is_empty());
    assert!(world.items.get(&ItemId(50)).is_none());
}

#[test]
fn give_bracelet_outside_state5_is_handed_back_with_dialogue() {
    let mut world = World::default();
    let mut jaz = jaz_npc(1);
    jaz.cursor_item = Some(ItemId(50));
    world.add_character(jaz);
    let mut bracelet = item(50, ItemFlags::empty());
    bracelet.template_id = IID_ARKHATA_BRACELET;
    bracelet.carried_by = Some(CharacterId(1));
    world.add_item(bracelet);
    world.add_character(player(2, "Godmode"));

    if let Some(jaz) = world.characters.get_mut(&CharacterId(1)) {
        jaz.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_jaz_actions(&facts(CharacterId(2), 4, 12), 1);
    assert!(!events
        .iter()
        .any(|event| matches!(event, JazOutcomeEvent::QuestDone66 { .. })));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}

#[test]
fn text_repeat_resets_to_0_when_at_or_below_state5() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jaz_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(jaz) = world.characters.get_mut(&CharacterId(1)) {
        jaz.push_driver_text_message(CharacterId(2), "repeat");
    }
    let events = world.process_jaz_actions(&facts(CharacterId(2), 3, 12), 1);
    assert!(events.contains(&JazOutcomeEvent::UpdateJazState {
        player_id: CharacterId(2),
        new_state: 0,
    }));
}

#[test]
fn text_repeat_resets_to_6_when_between_states_6_and_7() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(jaz_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(jaz) = world.characters.get_mut(&CharacterId(1)) {
        jaz.push_driver_text_message(CharacterId(2), "restart");
    }
    let events = world.process_jaz_actions(&facts(CharacterId(2), 7, 12), 1);
    assert!(events.contains(&JazOutcomeEvent::UpdateJazState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
}

#[test]
fn give_unrelated_item_is_handed_back() {
    let mut world = World::default();
    let mut jaz = jaz_npc(1);
    jaz.cursor_item = Some(ItemId(50));
    world.add_character(jaz);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(jaz) = world.characters.get_mut(&CharacterId(1)) {
        jaz.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_jaz_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
