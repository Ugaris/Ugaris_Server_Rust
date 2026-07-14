use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_LOSTDWARF, NT_CHAR, NT_GIVE};
use crate::item_driver::{IID_DWARFRECALL1, IID_DWARFRECALL2};
use crate::world::npc::area31::lostdwarf::{
    LostDwarfDriverData, LostdwarfOutcomeEvent, LostdwarfPlayerFacts,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn lostdwarf_npc(id: u32, nr: i32) -> Character {
    let mut lostdwarf = character(id);
    lostdwarf.name = "Lost Dwarf".into();
    lostdwarf.driver = CDR_LOSTDWARF;
    lostdwarf.driver_state = Some(CharacterDriverState::LostDwarf(LostDwarfDriverData {
        nr,
        ..Default::default()
    }));
    lostdwarf
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
) -> HashMap<CharacterId, LostdwarfPlayerFacts> {
    let mut map = HashMap::new();
    map.insert(player_id, LostdwarfPlayerFacts { dwarfchief_state });
    map
}

#[test]
fn nt_char_greets_when_no_recall_scroll_is_being_offered() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(lostdwarf_npc(1, 1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(lostdwarf) = world.characters.get_mut(&CharacterId(1)) {
        lostdwarf.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_lostdwarf_actions(&facts(CharacterId(2), 0), 1);
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("dwarven recall scroll")));
}

#[test]
fn giving_matching_recall1_at_state_le3_rescues_miner_1() {
    let mut world = World::default();
    let mut lostdwarf = lostdwarf_npc(1, 1);
    lostdwarf.cursor_item = Some(ItemId(50));
    lostdwarf.x = 10;
    lostdwarf.y = 10;
    world.add_character(lostdwarf);
    let mut scroll = item(50, ItemFlags::empty());
    scroll.template_id = IID_DWARFRECALL1;
    scroll.carried_by = Some(CharacterId(1));
    world.add_item(scroll);
    world.add_character(player(2, "Godmode"));

    if let Some(lostdwarf) = world.characters.get_mut(&CharacterId(1)) {
        lostdwarf.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_lostdwarf_actions(&facts(CharacterId(2), 3), 1);
    assert!(
        events.contains(&LostdwarfOutcomeEvent::UpdateDwarfchiefState {
            player_id: CharacterId(2),
            new_state: 4,
        })
    );
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("almost ate my beard")));
    assert!(texts.iter().any(|text| text
        .message
        .contains("uses a scroll of recall and vanishes")));
    let lostdwarf = world.characters.get(&CharacterId(1)).unwrap();
    assert!(lostdwarf.flags.contains(CharacterFlags::INVISIBLE));
    assert!(!world.items.contains_key(&ItemId(50)));
}

#[test]
fn giving_wrong_nr_recall_scroll_does_not_rescue() {
    let mut world = World::default();
    let mut lostdwarf = lostdwarf_npc(1, 2);
    lostdwarf.cursor_item = Some(ItemId(50));
    world.add_character(lostdwarf);
    // Miner nr=2 expects IID_DWARFRECALL2, not IID_DWARFRECALL1.
    let mut scroll = item(50, ItemFlags::empty());
    scroll.template_id = IID_DWARFRECALL1;
    scroll.carried_by = Some(CharacterId(1));
    world.add_item(scroll);
    world.add_character(player(2, "Godmode"));

    if let Some(lostdwarf) = world.characters.get_mut(&CharacterId(1)) {
        lostdwarf.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_lostdwarf_actions(&facts(CharacterId(2), 5), 1);
    assert!(events.is_empty());
    let lostdwarf = world.characters.get(&CharacterId(1)).unwrap();
    assert!(!lostdwarf.flags.contains(CharacterFlags::INVISIBLE));
    // The item is always destroyed regardless of a match (C's
    // unconditional trailing "let it vanish" catch-all).
    assert!(!world.items.contains_key(&ItemId(50)));
}

#[test]
fn giving_matching_recall2_at_state_5_to_6_rescues_miner_2_with_system_note() {
    let mut world = World::default();
    let mut lostdwarf = lostdwarf_npc(1, 2);
    lostdwarf.cursor_item = Some(ItemId(50));
    world.add_character(lostdwarf);
    let mut scroll = item(50, ItemFlags::empty());
    scroll.template_id = IID_DWARFRECALL2;
    scroll.carried_by = Some(CharacterId(1));
    world.add_item(scroll);
    world.add_character(player(2, "Godmode"));

    if let Some(lostdwarf) = world.characters.get_mut(&CharacterId(1)) {
        lostdwarf.push_driver_message(NT_GIVE, 2, 50, 0);
    }

    let events = world.process_lostdwarf_actions(&facts(CharacterId(2), 5), 1);
    assert!(
        events.contains(&LostdwarfOutcomeEvent::UpdateDwarfchiefState {
            player_id: CharacterId(2),
            new_state: 7,
        })
    );
}

#[test]
fn invisibility_clears_after_the_timer_expires() {
    let mut world = World::default();
    let mut lostdwarf = lostdwarf_npc(1, 1);
    lostdwarf.flags |= CharacterFlags::INVISIBLE;
    lostdwarf.driver_state = Some(CharacterDriverState::LostDwarf(LostDwarfDriverData {
        nr: 1,
        invis_tick: 10,
        last_talk: 0,
    }));
    world.add_character(lostdwarf);

    world.tick = Tick(20);
    world.process_lostdwarf_actions(&HashMap::new(), 1);

    let lostdwarf = world.characters.get(&CharacterId(1)).unwrap();
    assert!(!lostdwarf.flags.contains(CharacterFlags::INVISIBLE));
}
