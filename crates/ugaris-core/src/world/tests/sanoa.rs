use super::*;
use crate::character_driver::{SanoaDriverData, CDR_SANOA, NT_GOTHIT};

fn sanoa_npc(id: u32) -> Character {
    let mut sanoa = character(id);
    sanoa.name = "Sanoa".into();
    sanoa.driver = CDR_SANOA;
    sanoa.driver_state = Some(CharacterDriverState::Sanoa(SanoaDriverData::default()));
    sanoa
}

fn sanoa_state(world: &World, id: CharacterId) -> SanoaDriverData {
    match world
        .characters
        .get(&id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Sanoa(data)) => data,
        _ => panic!("expected sanoa driver state"),
    }
}

#[test]
fn sanoa_walks_toward_guard_post_when_not_there() {
    let mut world = World::default();
    assert!(world.spawn_character(sanoa_npc(1), 10, 31));

    let acted = world.process_sanoa_actions(1);

    assert_eq!(acted, 1);
    let sanoa = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(sanoa.action, action::WALK);
    assert_eq!(sanoa_state(&world, CharacterId(1)).state, 0);
}

#[test]
fn sanoa_waits_at_post_before_clock_gate() {
    let mut world = World::default();
    assert!(world.spawn_character(sanoa_npc(1), 16, 31));
    world.date.hour = 20;
    world.date.minute = 0;

    let acted = world.process_sanoa_actions(1);

    assert_eq!(acted, 1);
    assert_eq!(sanoa_state(&world, CharacterId(1)).state, 0);
    let sanoa = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(sanoa.action, action::IDLE);
}

#[test]
fn sanoa_leaves_post_once_a_departure_window_passes() {
    let mut world = World::default();
    assert!(world.spawn_character(sanoa_npc(1), 16, 31));
    world.date.hour = 7;
    world.date.minute = 10;

    world.process_sanoa_actions(1);

    assert_eq!(sanoa_state(&world, CharacterId(1)).state, 1);
}

#[test]
fn sanoa_does_not_leave_post_outside_any_departure_window() {
    let mut world = World::default();
    assert!(world.spawn_character(sanoa_npc(1), 16, 31));
    world.date.hour = 7;
    world.date.minute = 45;

    world.process_sanoa_actions(1);

    assert_eq!(sanoa_state(&world, CharacterId(1)).state, 0);
}

#[test]
fn sanoa_advances_past_already_closed_door_without_toggling() {
    let mut world = World::default();
    let mut sanoa = sanoa_npc(1);
    sanoa.driver_state = Some(CharacterDriverState::Sanoa(SanoaDriverData {
        state: 3,
        ..SanoaDriverData::default()
    }));
    assert!(world.spawn_character(sanoa, 21, 25));

    let mut door = item(9, ItemFlags::USE | ItemFlags::DOOR);
    door.driver = crate::item_driver::IDR_DOOR;
    door.driver_data = vec![0]; // C `it[in].drdata[0]` unset: closed.
    assert!(world.map.set_item_map(&mut door, 21, 26));
    world.items.insert(ItemId(9), door);

    world.process_sanoa_actions(1);

    // C `if (!is_closed(21, 26) && use_item_at(...)) return; dat->state++;`:
    // the door is already closed, so `is_closed` is true, the whole
    // condition is false, and the state advances without toggling.
    assert_eq!(sanoa_state(&world, CharacterId(1)).state, 4);
    let door = world.items.get(&ItemId(9)).unwrap();
    assert_eq!(door.driver_data[0], 0);
}

#[test]
fn sanoa_toggles_an_open_door_at_the_use_point_and_stays_in_state() {
    let mut world = World::default();
    let mut sanoa = sanoa_npc(1);
    sanoa.driver_state = Some(CharacterDriverState::Sanoa(SanoaDriverData {
        state: 3,
        ..SanoaDriverData::default()
    }));
    assert!(world.spawn_character(sanoa, 21, 25));

    let mut door = item(9, ItemFlags::USE);
    door.driver = crate::item_driver::IDR_DOOR;
    door.driver_data = vec![1]; // open.
    assert!(world.map.set_item_map(&mut door, 21, 26));
    world.items.insert(ItemId(9), door);

    let acted = world.process_sanoa_actions(1);

    assert_eq!(acted, 1);
    // C `return;` on success: the state doesn't advance this tick, it's
    // re-evaluated (and will now see the door closed) next tick.
    assert_eq!(sanoa_state(&world, CharacterId(1)).state, 3);
    let door = world.items.get(&ItemId(9)).unwrap();
    assert_eq!(door.driver_data[0], 0);
}

#[test]
fn sanoa_tracks_victim_from_gothit_message_and_attacks_when_adjacent() {
    let mut world = World::default();
    let mut sanoa = sanoa_npc(1);
    sanoa.group = 0;
    assert!(world.spawn_character(sanoa, 10, 10));
    let mut attacker = character(2);
    attacker.group = 1;
    attacker.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(attacker, 11, 10));
    if let Some(sanoa) = world.characters.get_mut(&CharacterId(1)) {
        sanoa.push_driver_message(NT_GOTHIT, 2, 5, 0);
    }

    let acted = world.process_sanoa_actions(1);

    assert_eq!(acted, 1);
    assert_eq!(
        sanoa_state(&world, CharacterId(1)).victim,
        Some(CharacterId(2))
    );
    let sanoa = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(sanoa.action, action::ATTACK1);
}

#[test]
fn sanoa_does_not_track_victim_from_same_group_gothit() {
    // C `if (ch[cn].group == ch[co].group) break;` - both default to
    // group 0, so this self-defense branch stays inert (same precedent as
    // `world::npc::area1::robber`'s own test).
    let mut world = World::default();
    let sanoa = sanoa_npc(1);
    assert!(world.spawn_character(sanoa, 10, 10));
    let attacker = character(2);
    assert!(world.spawn_character(attacker, 11, 10));
    if let Some(sanoa) = world.characters.get_mut(&CharacterId(1)) {
        sanoa.push_driver_message(NT_GOTHIT, 2, 5, 0);
    }

    world.process_sanoa_actions(1);

    assert_eq!(sanoa_state(&world, CharacterId(1)).victim, None);
}
