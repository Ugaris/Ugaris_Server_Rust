use super::*;
use crate::character_driver::{BalltrapDriverData, CDR_BALLTRAP, NT_GOTHIT};

fn balltrap_npc(id: u32) -> Character {
    let mut balltrap = character(id);
    balltrap.name = "Wood Skelly".into();
    balltrap.driver = CDR_BALLTRAP;
    balltrap.driver_state = Some(CharacterDriverState::Balltrap(BalltrapDriverData::default()));
    balltrap
}

fn balltrap_state(world: &World, id: CharacterId) -> BalltrapDriverData {
    match world
        .characters
        .get(&id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Balltrap(data)) => data,
        _ => panic!("expected balltrap driver state"),
    }
}

/// `spawn_character` does not set `rest_x`/`rest_y` (C `tmpx`/`tmpy`) the
/// way a real zone load does; every test that needs the NPC "at its post"
/// sets them explicitly to the spawn tile, same precedent as
/// `world::tests::asturin`.
fn spawn_balltrap_at_post(world: &mut World, id: u32, x: usize, y: usize) {
    let mut balltrap = balltrap_npc(id);
    balltrap.rest_x = x as u16;
    balltrap.rest_y = y as u16;
    assert!(world.spawn_character(balltrap, x, y));
}

#[test]
fn balltrap_walks_back_to_post_when_away_from_home() {
    let mut world = World::default();
    spawn_balltrap_at_post(&mut world, 1, 35, 227);
    assert!(world.teleport_character(CharacterId(1), 40, 227, false));

    let acted = world.process_balltrap_actions(1);

    assert_eq!(acted, 1);
    let balltrap = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(balltrap.action, action::WALK);
}

#[test]
fn balltrap_fires_left_item_after_three_seconds_at_post() {
    let mut world = World::default();
    spawn_balltrap_at_post(&mut world, 1, 35, 227);
    world.tick.0 = TICKS_PER_SECOND * 3 + 1;

    let mut trap = item(9, ItemFlags::USE);
    trap.driver = crate::item_driver::IDR_BALLTRAP;
    trap.driver_data = vec![148, 128, 4]; // dx=20,dy=0,power=4 (arbitrary).
    assert!(world.map.set_item_map(&mut trap, 34, 227));
    world.items.insert(ItemId(9), trap);

    let acted = world.process_balltrap_actions(1);

    assert_eq!(acted, 1);
    let balltrap = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(balltrap.action, action::USE);
    assert_eq!(
        balltrap_state(&world, CharacterId(1)).last_fire,
        world.tick.0
    );
}

#[test]
fn balltrap_does_not_fire_before_three_seconds_elapsed() {
    let mut world = World::default();
    spawn_balltrap_at_post(&mut world, 1, 35, 227);
    world.tick.0 = TICKS_PER_SECOND; // less than 3 seconds since last_fire=0.

    let mut trap = item(9, ItemFlags::USE);
    trap.driver = crate::item_driver::IDR_BALLTRAP;
    assert!(world.map.set_item_map(&mut trap, 34, 227));
    world.items.insert(ItemId(9), trap);

    let acted = world.process_balltrap_actions(1);

    // No usable action fired: falls through to `do_idle`.
    assert!(acted == 1);
    let balltrap = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(balltrap.action, action::IDLE);
    assert_eq!(balltrap_state(&world, CharacterId(1)).last_fire, 0);
}

#[test]
fn balltrap_idles_when_no_item_to_the_left() {
    let mut world = World::default();
    spawn_balltrap_at_post(&mut world, 1, 35, 227);
    world.tick.0 = TICKS_PER_SECOND * 3 + 1;

    let acted = world.process_balltrap_actions(1);

    assert_eq!(acted, 1);
    let balltrap = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(balltrap.action, action::IDLE);
    // `do_use` failed (no item there), but the timer still advances since
    // C sets `dat->last_fire = ticker` unconditionally before the
    // `if (do_use(...))` check.
    assert_eq!(
        balltrap_state(&world, CharacterId(1)).last_fire,
        world.tick.0
    );
}

#[test]
fn balltrap_tracks_victim_from_gothit_message_and_attacks_when_adjacent() {
    let mut world = World::default();
    let mut balltrap = balltrap_npc(1);
    balltrap.group = 0;
    assert!(world.spawn_character(balltrap, 10, 10));
    let mut attacker = character(2);
    attacker.group = 1;
    attacker.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(attacker, 11, 10));
    if let Some(balltrap) = world.characters.get_mut(&CharacterId(1)) {
        balltrap.push_driver_message(NT_GOTHIT, 2, 5, 0);
    }

    let acted = world.process_balltrap_actions(1);

    assert_eq!(acted, 1);
    assert_eq!(
        balltrap_state(&world, CharacterId(1)).victim,
        Some(CharacterId(2))
    );
    let balltrap = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(balltrap.action, action::ATTACK1);
}

#[test]
fn balltrap_does_not_track_victim_from_same_group_gothit() {
    // C `if (ch[cn].group == ch[co].group) break;` - both default to
    // group 0, same precedent as `world::npc::area1::robber`'s own test.
    let mut world = World::default();
    let balltrap = balltrap_npc(1);
    assert!(world.spawn_character(balltrap, 10, 10));
    let attacker = character(2);
    assert!(world.spawn_character(attacker, 11, 10));
    if let Some(balltrap) = world.characters.get_mut(&CharacterId(1)) {
        balltrap.push_driver_message(NT_GOTHIT, 2, 5, 0);
    }

    world.process_balltrap_actions(1);

    assert_eq!(balltrap_state(&world, CharacterId(1)).victim, None);
}
