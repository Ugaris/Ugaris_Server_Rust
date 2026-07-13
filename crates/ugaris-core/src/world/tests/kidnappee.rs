use std::collections::HashMap;

use super::*;
use crate::character_driver::{CDR_KIDNAPPEE, NT_CHAR};
use crate::item_driver::IID_ARKHATA_IRONPOTION;
use crate::world::npc::area37::kidnappee::{
    KidnappeeDriverData, KidnappeeOutcomeEvent, KidnappeePlayerFacts,
};

const BASELINE_TICK: u64 = TICKS_PER_SECOND * 1000;

fn kidnappee_npc(id: u32) -> Character {
    let mut kidnappee = character(id);
    kidnappee.name = "Student".into();
    kidnappee.driver = CDR_KIDNAPPEE;
    kidnappee.driver_state = Some(CharacterDriverState::Kidnappee(
        KidnappeeDriverData::default(),
    ));
    kidnappee
}

fn player(id: u32, name: &str) -> Character {
    let mut player = character(id);
    player.flags |= CharacterFlags::PLAYER;
    player.name = name.into();
    player
}

fn facts(
    player_id: CharacterId,
    kid_state: i32,
    trainer_state: i32,
) -> HashMap<CharacterId, KidnappeePlayerFacts> {
    let mut map = HashMap::new();
    map.insert(
        player_id,
        KidnappeePlayerFacts {
            kid_state,
            trainer_state,
        },
    );
    map
}

#[test]
fn state0_without_trainer_progress_is_a_silent_no_op() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(kidnappee_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(kidnappee) = world.characters.get_mut(&CharacterId(1)) {
        kidnappee.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_kidnappee_actions(&facts(CharacterId(2), 0, 0), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn state0_with_trainer_progress_pleads_and_collapses_to_2() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(kidnappee_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(kidnappee) = world.characters.get_mut(&CharacterId(1)) {
        kidnappee.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_kidnappee_actions(&facts(CharacterId(2), 0, 1), 1);
    assert!(events.contains(&KidnappeeOutcomeEvent::UpdateKidState {
        player_id: CharacterId(2),
        new_state: 2,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("sent to rescue me")));
}

#[test]
fn state2_without_potion_asks_for_the_secret_and_advances_to_3() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(kidnappee_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(kidnappee) = world.characters.get_mut(&CharacterId(1)) {
        kidnappee.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_kidnappee_actions(&facts(CharacterId(2), 2, 1), 1);
    assert!(events.contains(&KidnappeeOutcomeEvent::UpdateKidState {
        player_id: CharacterId(2),
        new_state: 3,
    }));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Only their leader could seal")));
}

#[test]
fn state2_with_potion_opens_the_cage_and_jumps_to_4() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(kidnappee_npc(1), 10, 10));
    let mut godmode = player(2, "Godmode");
    godmode.inventory[0] = Some(ItemId(50));
    assert!(world.spawn_character(godmode, 12, 10));
    let mut potion = item(50, ItemFlags::empty());
    potion.template_id = IID_ARKHATA_IRONPOTION;
    potion.carried_by = Some(CharacterId(2));
    world.add_item(potion);

    world.tick = Tick(BASELINE_TICK);
    if let Some(kidnappee) = world.characters.get_mut(&CharacterId(1)) {
        kidnappee.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_kidnappee_actions(&facts(CharacterId(2), 2, 1), 1);
    assert!(events.contains(&KidnappeeOutcomeEvent::UpdateKidState {
        player_id: CharacterId(2),
        new_state: 4,
    }));
    assert!(world.items.get(&ItemId(50)).is_none());
}

#[test]
fn state4_thanks_the_player_advances_to_5_and_goes_invisible() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(kidnappee_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(kidnappee) = world.characters.get_mut(&CharacterId(1)) {
        kidnappee.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_kidnappee_actions(&facts(CharacterId(2), 4, 1), 1);
    assert!(events.contains(&KidnappeeOutcomeEvent::UpdateKidState {
        player_id: CharacterId(2),
        new_state: 5,
    }));
    let kidnappee = world.characters.get(&CharacterId(1)).unwrap();
    assert!(kidnappee.flags.contains(CharacterFlags::INVISIBLE));
}

#[test]
fn state5_is_a_silent_no_op_all_done() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(kidnappee_npc(1), 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(kidnappee) = world.characters.get_mut(&CharacterId(1)) {
        kidnappee.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_kidnappee_actions(&facts(CharacterId(2), 5, 1), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn invisible_kidnappee_ignores_char_messages() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    let mut kidnappee = kidnappee_npc(1);
    kidnappee.flags |= CharacterFlags::INVISIBLE;
    assert!(world.spawn_character(kidnappee, 10, 10));
    assert!(world.spawn_character(player(2, "Godmode"), 12, 10));

    world.tick = Tick(BASELINE_TICK);
    if let Some(kidnappee) = world.characters.get_mut(&CharacterId(1)) {
        kidnappee.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    let events = world.process_kidnappee_actions(&facts(CharacterId(2), 0, 1), 1);
    assert!(events.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}
