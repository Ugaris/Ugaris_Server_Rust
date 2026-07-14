use std::collections::HashMap;

use super::*;
use crate::character_driver::{Astro2DriverData, CDR_ASTRO2, NT_CHAR, NT_GIVE};
use crate::item_driver::IID_AREA2_ASTRONOTE;
use crate::world::astro2::{Astro2OutcomeEvent, Astro2PlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn astro2_npc(id: u32) -> Character {
    let mut astro2 = character(id);
    astro2.name = "Astro2".into();
    astro2.driver = CDR_ASTRO2;
    astro2.driver_state = Some(CharacterDriverState::Astro2(Astro2DriverData::default()));
    astro2
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(player_id: CharacterId, astro2_state: i32) -> HashMap<CharacterId, Astro2PlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, Astro2PlayerFacts { astro2_state });
    map
}

fn astro2_state(world: &World, astro2_id: CharacterId) -> Astro2DriverData {
    match world
        .characters
        .get(&astro2_id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Astro2(data)) => data,
        _ => panic!("expected astro2 driver state"),
    }
}

#[test]
fn astro2_greets_new_player_opens_quest_and_advances_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(astro2_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(astro2) = world.characters.get_mut(&CharacterId(1)) {
        astro2.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_astro2_actions(&facts(CharacterId(2), 0), 1);
    assert!(events.contains(&Astro2OutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
    }));
    assert!(events.contains(&Astro2OutcomeEvent::UpdateAstro2State {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I am Astro2, the astronomer")));
    assert_eq!(
        astro2_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn astro2_state4_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(astro2_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(astro2) = world.characters.get_mut(&CharacterId(1)) {
        astro2.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_astro2_actions(&facts(CharacterId(2), 4), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn astro2_text_repeat_resets_state_to_zero_when_in_range() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(astro2_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(astro2) = world.characters.get_mut(&CharacterId(1)) {
        astro2.driver_state = Some(CharacterDriverState::Astro2(Astro2DriverData {
            last_talk: 0,
            current_victim: None,
        }));
        astro2.push_driver_text_message(CharacterId(2), "repeat");
    }

    let events = world.process_astro2_actions(&facts(CharacterId(2), 3), 1);
    assert!(events.contains(&Astro2OutcomeEvent::UpdateAstro2State {
        player_id: CharacterId(2),
        new_state: 0,
    }));
    assert_eq!(
        astro2_state(&world, CharacterId(1)).current_victim,
        Some(CharacterId(2))
    );
}

#[test]
fn astro2_receiving_astronote_completes_quest_and_destroys_item() {
    let mut world = World::default();
    let mut astro2 = astro2_npc(1);
    astro2.cursor_item = Some(ItemId(50));
    world.add_character(astro2);
    let mut note = item(50, ItemFlags::empty());
    note.name = "Astronomer's Notes".into();
    note.template_id = IID_AREA2_ASTRONOTE;
    note.carried_by = Some(CharacterId(1));
    world.add_item(note);
    world.add_character(player(2, "Godmode"));

    if let Some(astro2) = world.characters.get_mut(&CharacterId(1)) {
        astro2.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_astro2_actions(&facts(CharacterId(2), 4), 1);
    assert!(events.contains(&Astro2OutcomeEvent::UpdateAstro2State {
        player_id: CharacterId(2),
        new_state: 5,
    }));
    assert!(events.contains(&Astro2OutcomeEvent::QuestDone {
        player_id: CharacterId(2),
    }));

    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("jolly good")));
    // The NPC's cursor item (the notes) is destroyed, not handed back.
    assert!(!world.items.contains_key(&ItemId(50)));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}

#[test]
fn astro2_give_other_item_hands_it_back_to_giver() {
    let mut world = World::default();
    let mut astro2 = astro2_npc(1);
    astro2.cursor_item = Some(ItemId(50));
    world.add_character(astro2);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(astro2) = world.characters.get_mut(&CharacterId(1)) {
        astro2.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_astro2_actions(&facts(CharacterId(2), 4), 1);
    assert!(events.is_empty());

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    // C's `astro2_driver` calls plain `give_char_item`, not `give_char_
    // item_smart` - the item lands on the (empty) cursor, not inventory.
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
}

#[test]
fn astro2_ignores_astronote_after_already_done() {
    let mut world = World::default();
    let mut astro2 = astro2_npc(1);
    astro2.cursor_item = Some(ItemId(50));
    world.add_character(astro2);
    let mut note = item(50, ItemFlags::empty());
    note.name = "Astronomer's Notes".into();
    note.template_id = IID_AREA2_ASTRONOTE;
    note.carried_by = Some(CharacterId(1));
    world.add_item(note);
    world.add_character(player(2, "Godmode"));

    if let Some(astro2) = world.characters.get_mut(&CharacterId(1)) {
        astro2.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    // astro2_state == 5 means the quest is already done (C's `ppd->
    // astro2_state <= 4` guard fails), so the note is handed back like
    // any other item.
    let events = world.process_astro2_actions(&facts(CharacterId(2), 5), 1);
    assert!(events.is_empty());
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
}
