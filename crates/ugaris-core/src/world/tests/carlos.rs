use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_CARLOS, NT_CHAR, NT_GIVE};
use crate::item_driver::{
    IID_CARLOS_DOOR, IID_MAX_CHRONICLES, IID_MAX_RITUAL, IID_STAFF_DRAGONKEY1,
    IID_STAFF_DRAGONSTAFF,
};
use crate::world::carlos::{CarlosDriverData, CarlosOutcomeEvent, CarlosPlayerFacts};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn carlos_npc(id: u32) -> Character {
    let mut carlos = character(id);
    carlos.name = "Carlos".into();
    carlos.driver = CDR_CARLOS;
    carlos.driver_state = Some(CharacterDriverState::Carlos(CarlosDriverData::default()));
    carlos
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    carlos_state: i32,
    carlos2_state: i32,
    level: u32,
    quest61_count: u8,
) -> HashMap<CharacterId, CarlosPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        CarlosPlayerFacts {
            carlos_state,
            carlos2_state,
            level,
            quest61_count,
        },
    );
    map
}

fn spawn_pair(world: &mut World) {
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(carlos_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));
    world.tick = Tick(BASELINE_TICK);
    if let Some(carlos) = world.characters.get_mut(&CharacterId(1)) {
        carlos.push_driver_message(NT_CHAR, 2, 0, 0);
    }
}

#[test]
fn carlos_ritual_state0_low_level_uses_driver_memory_instead_of_repeating() {
    let mut world = World::default();
    spawn_pair(&mut world);

    // Below the ritual-quest greeting gate (level 26), quest 61 untouched.
    let events = world.process_carlos_actions(&facts(CharacterId(2), 0, 0, 10, 0), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("grown more powerful")));

    // A second visit within the memory window does not repeat the line.
    if let Some(carlos) = world.characters.get_mut(&CharacterId(1)) {
        carlos.push_driver_message(NT_CHAR, 2, 0, 0);
    }
    world.tick = Tick(BASELINE_TICK + TICKS_PER_SECOND * 6);
    world.process_carlos_actions(&facts(CharacterId(2), 0, 0, 10, 0), 1);
    let texts = world.drain_pending_area_texts();
    assert!(texts.is_empty());
}

#[test]
fn carlos_ritual_state0_high_level_greets_opens_quest61_and_advances_to_1() {
    let mut world = World::default();
    spawn_pair(&mut world);

    let events = world.process_carlos_actions(&facts(CharacterId(2), 0, 0, 26, 0), 1);
    assert!(events.contains(&CarlosOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 61,
    }));
    assert!(events.contains(&CarlosOutcomeEvent::UpdateCarlos2State {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Chief Investigator of the Occult")));
}

#[test]
fn carlos_dragon_state0_greets_opens_quest20_and_advances_to_1() {
    let mut world = World::default();
    spawn_pair(&mut world);

    // quest61_count >= 1 -> dragon-staff (`carlos_state`) chain drives.
    let events = world.process_carlos_actions(&facts(CharacterId(2), 0, 0, 30, 1), 1);
    assert!(events.contains(&CarlosOutcomeEvent::QuestOpen {
        player_id: CharacterId(2),
        quest: 20,
    }));
    assert!(events.contains(&CarlosOutcomeEvent::UpdateCarlosState {
        player_id: CharacterId(2),
        new_state: 1,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("another mission")));
}

#[test]
fn carlos_dragon_state4_grants_key_when_missing_door() {
    let mut world = World::default();
    spawn_pair(&mut world);

    let events = world.process_carlos_actions(&facts(CharacterId(2), 4, 0, 30, 1), 1);
    assert!(events.contains(&CarlosOutcomeEvent::UpdateCarlosState {
        player_id: CharacterId(2),
        new_state: 5,
    }));
    assert!(events.contains(&CarlosOutcomeEvent::GrantCarlosKey {
        player_id: CharacterId(2),
        carlos_id: CharacterId(1),
    }));
}

#[test]
fn carlos_dragon_state4_skips_key_when_door_already_held() {
    let mut world = World::default();
    spawn_pair(&mut world);
    let mut door = item(60, ItemFlags::empty());
    door.template_id = IID_CARLOS_DOOR;
    door.carried_by = Some(CharacterId(2));
    world.add_item(door);
    if let Some(godmode) = world.characters.get_mut(&CharacterId(2)) {
        godmode.inventory[30] = Some(ItemId(60));
    }

    let events = world.process_carlos_actions(&facts(CharacterId(2), 4, 0, 30, 1), 1);
    assert!(events.contains(&CarlosOutcomeEvent::UpdateCarlosState {
        player_id: CharacterId(2),
        new_state: 5,
    }));
    assert!(!events
        .iter()
        .any(|event| matches!(event, CarlosOutcomeEvent::GrantCarlosKey { .. })));
}

#[test]
fn carlos_text_repeat_at_low_carlos_state_resets_carlos2_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(carlos_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(carlos) = world.characters.get_mut(&CharacterId(1)) {
        carlos.push_driver_text_message(CharacterId(2), "repeat");
    }

    // carlos_state <= 4 -> resets carlos2_state, per the C quirk (see
    // `world::carlos`'s module doc comment).
    let events = world.process_carlos_actions(&facts(CharacterId(2), 3, 2, 30, 1), 1);
    assert!(events.contains(&CarlosOutcomeEvent::UpdateCarlos2State {
        player_id: CharacterId(2),
        new_state: 0,
    }));
    assert!(!events
        .iter()
        .any(|event| matches!(event, CarlosOutcomeEvent::UpdateCarlosState { .. })));
}

#[test]
fn carlos_text_repeat_at_high_carlos_state_resets_carlos_state() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(carlos_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(carlos) = world.characters.get_mut(&CharacterId(1)) {
        carlos.push_driver_text_message(CharacterId(2), "repeat");
    }

    // carlos_state == 5 (not <= 4) -> falls to the else branch, resetting
    // carlos_state itself.
    let events = world.process_carlos_actions(&facts(CharacterId(2), 5, 0, 30, 1), 1);
    assert!(events.contains(&CarlosOutcomeEvent::UpdateCarlosState {
        player_id: CharacterId(2),
        new_state: 0,
    }));
    assert!(!events
        .iter()
        .any(|event| matches!(event, CarlosOutcomeEvent::UpdateCarlos2State { .. })));
}

#[test]
fn carlos_text_repeat_at_done_carlos_state_resets_nothing() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(carlos_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    if let Some(carlos) = world.characters.get_mut(&CharacterId(1)) {
        carlos.push_driver_text_message(CharacterId(2), "repeat");
    }

    // carlos_state == 6 (dragon-staff done) -> neither branch fires.
    let events = world.process_carlos_actions(&facts(CharacterId(2), 6, 0, 30, 1), 1);
    assert!(events.is_empty());
}

#[test]
fn carlos_receiving_dragonstaff_completes_quest20_and_advances_state() {
    let mut world = World::default();
    let mut carlos = carlos_npc(1);
    carlos.cursor_item = Some(ItemId(50));
    world.add_character(carlos);
    let mut staff = item(50, ItemFlags::empty());
    staff.name = "Dragon Staff".into();
    staff.template_id = IID_STAFF_DRAGONSTAFF;
    staff.carried_by = Some(CharacterId(1));
    world.add_item(staff);
    world.add_character(player(2, "Godmode"));

    if let Some(carlos) = world.characters.get_mut(&CharacterId(1)) {
        carlos.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_carlos_actions(&facts(CharacterId(2), 5, 0, 30, 1), 1);
    assert!(events.contains(&CarlosOutcomeEvent::UpdateCarlosState {
        player_id: CharacterId(2),
        new_state: 6,
    }));
    assert!(events.contains(&CarlosOutcomeEvent::DragonStaffQuestDone {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("Well done")));
    assert!(!world.items.contains_key(&ItemId(50)));
}

#[test]
fn carlos_receiving_dragonstaff_sweeps_remaining_dragon_keys() {
    let mut world = World::default();
    let mut carlos = carlos_npc(1);
    carlos.cursor_item = Some(ItemId(50));
    world.add_character(carlos);
    let mut staff = item(50, ItemFlags::empty());
    staff.template_id = IID_STAFF_DRAGONSTAFF;
    staff.carried_by = Some(CharacterId(1));
    world.add_item(staff);
    let mut godmode = player(2, "Godmode");
    godmode.inventory[30] = Some(ItemId(51));
    world.add_character(godmode);
    let mut key1 = item(51, ItemFlags::empty());
    key1.template_id = IID_STAFF_DRAGONKEY1;
    key1.carried_by = Some(CharacterId(2));
    world.add_item(key1);

    if let Some(carlos) = world.characters.get_mut(&CharacterId(1)) {
        carlos.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    world.process_carlos_actions(&facts(CharacterId(2), 5, 0, 30, 1), 1);
    assert!(!world.items.contains_key(&ItemId(51)));
}

#[test]
fn carlos_receiving_ritual_completes_quest61_and_advances_state() {
    let mut world = World::default();
    let mut carlos = carlos_npc(1);
    carlos.cursor_item = Some(ItemId(50));
    world.add_character(carlos);
    let mut ritual = item(50, ItemFlags::empty());
    ritual.name = "Ritual Scroll".into();
    ritual.template_id = IID_MAX_RITUAL;
    ritual.carried_by = Some(CharacterId(1));
    world.add_item(ritual);
    world.add_character(player(2, "Godmode"));

    if let Some(carlos) = world.characters.get_mut(&CharacterId(1)) {
        carlos.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_carlos_actions(&facts(CharacterId(2), 0, 4, 30, 0), 1);
    assert!(events.contains(&CarlosOutcomeEvent::UpdateCarlos2State {
        player_id: CharacterId(2),
        new_state: 5,
    }));
    assert!(events.contains(&CarlosOutcomeEvent::RitualQuestDone {
        player_id: CharacterId(2),
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message.contains("Well done")));
    assert!(!world.items.contains_key(&ItemId(50)));
}

#[test]
fn carlos_receiving_unrelated_item_hands_it_back() {
    let mut world = World::default();
    let mut carlos = carlos_npc(1);
    carlos.cursor_item = Some(ItemId(50));
    world.add_character(carlos);
    let mut trinket = item(50, ItemFlags::empty());
    trinket.name = "Trinket".into();
    trinket.carried_by = Some(CharacterId(1));
    world.add_item(trinket);
    world.add_character(player(2, "Godmode"));

    if let Some(carlos) = world.characters.get_mut(&CharacterId(1)) {
        carlos.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_carlos_actions(&HashMap::new(), 1);
    assert!(events.is_empty());
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("better use for this than I do")));
    let godmode = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(godmode.cursor_item, Some(ItemId(50)));
    // C also does not check `IID_MAX_CHRONICLES` since that's only swept
    // on ritual turn-in, not on unrelated-item turn-in.
    assert!(!world
        .items
        .get(&ItemId(50))
        .is_some_and(|item| item.template_id == IID_MAX_CHRONICLES));
}
