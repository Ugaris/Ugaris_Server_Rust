use super::*;
use crate::character_driver::{CDR_TESTER, NT_GOTHIT};
use crate::world::npc::area4::tester::TesterDriverData;

fn tester_npc(id: u32) -> Character {
    let mut tester = character(id);
    tester.name = "Tester".into();
    tester.driver = CDR_TESTER;
    tester.driver_state = Some(CharacterDriverState::Tester(TesterDriverData::default()));
    tester
}

fn tester_state(world: &World, id: CharacterId) -> TesterDriverData {
    match world
        .characters
        .get(&id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Tester(data)) => data,
        _ => panic!("expected tester driver state"),
    }
}

#[test]
fn tester_rerolls_destination_and_idles_on_first_tick() {
    // C: `tester_data` is zero-initialized on first `set_data`, so
    // `move_driver(cn, 0, 0, 2)` fails on the very first tick, a fresh
    // `dest_x`/`dest_y` is rolled near the current position, and
    // `do_idle(cn, TICKS)` fires - no move happens this tick.
    let mut world = World::default();
    assert!(world.spawn_character(tester_npc(1), 50, 50));

    let acted = world.process_tester_actions(1);

    assert_eq!(acted, 1);
    let tester = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(tester.action, action::IDLE);
    let state = tester_state(&world, CharacterId(1));
    assert_ne!((state.dest_x, state.dest_y), (0, 0));
}

#[test]
fn tester_tracks_victim_from_gothit_message_and_attacks_when_adjacent() {
    let mut world = World::default();
    let mut tester = tester_npc(1);
    tester.group = 0;
    assert!(world.spawn_character(tester, 10, 10));
    let mut attacker = character(2);
    attacker.group = 1;
    attacker.flags |= CharacterFlags::PLAYER;
    assert!(world.spawn_character(attacker, 11, 10));
    if let Some(tester) = world.characters.get_mut(&CharacterId(1)) {
        tester.push_driver_message(NT_GOTHIT, 2, 5, 0);
    }

    let acted = world.process_tester_actions(1);

    assert_eq!(acted, 1);
    assert_eq!(
        tester_state(&world, CharacterId(1)).victim,
        Some(CharacterId(2))
    );
    let tester = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(tester.action, action::ATTACK1);
}

#[test]
fn tester_does_not_track_victim_from_same_group_gothit() {
    let mut world = World::default();
    let tester = tester_npc(1);
    assert!(world.spawn_character(tester, 10, 10));
    let attacker = character(2);
    assert!(world.spawn_character(attacker, 11, 10));
    if let Some(tester) = world.characters.get_mut(&CharacterId(1)) {
        tester.push_driver_message(NT_GOTHIT, 2, 5, 0);
    }

    world.process_tester_actions(1);

    assert_eq!(tester_state(&world, CharacterId(1)).victim, None);
}

#[test]
fn tester_walks_toward_a_visible_ground_potion_to_pick_it_up() {
    let mut world = World::default();
    assert!(world.spawn_character(tester_npc(1), 10, 10));

    let mut potion = item(9, ItemFlags::TAKE);
    potion.driver = IDR_POTION;
    potion.driver_data = vec![0, 15, 0, 0]; // heals HP.
    assert!(world.map.set_item_map(&mut potion, 15, 10));
    world.items.insert(ItemId(9), potion);
    world.map.tile_mut(15, 10).unwrap().light = 255; // `char_see_item`'s light gate for `IF_TAKE` items.

    let acted = world.process_tester_actions(1);

    assert_eq!(acted, 1);
    let tester = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(tester.action, action::WALK);
    assert_eq!(
        tester_state(&world, CharacterId(1)).item_to_pickup,
        Some(ItemId(9))
    );
}

#[test]
fn tester_takes_an_adjacent_potion_directly() {
    let mut world = World::default();
    assert!(world.spawn_character(tester_npc(1), 10, 10));

    let mut potion = item(9, ItemFlags::TAKE);
    potion.driver = IDR_POTION;
    potion.driver_data = vec![0, 15, 0, 0];
    assert!(world.map.set_item_map(&mut potion, 11, 10));
    world.items.insert(ItemId(9), potion);
    world.map.tile_mut(11, 10).unwrap().light = 255; // `char_see_item`'s light gate for `IF_TAKE` items.

    let acted = world.process_tester_actions(1);

    assert_eq!(acted, 1);
    let tester = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(tester.action, action::TAKE);
}

#[test]
fn tester_uses_an_adjacent_unsolved_pentagram() {
    let mut world = World::default();
    assert!(world.spawn_character(tester_npc(1), 10, 10));

    let mut pent = item(9, ItemFlags::USE);
    pent.driver = IDR_PENT;
    pent.driver_data = vec![5, 0, 1, 0, 0]; // level 5, unsolved (drdata[1] == 0).
    assert!(world.map.set_item_map(&mut pent, 11, 10));
    world.items.insert(ItemId(9), pent);

    let acted = world.process_tester_actions(1);

    assert_eq!(acted, 1);
    let tester = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(tester.action, action::USE);
    assert_eq!(
        tester_state(&world, CharacterId(1)).item_to_use,
        Some(ItemId(9))
    );
}

#[test]
fn tester_ignores_an_already_activated_pentagram() {
    let mut world = World::default();
    assert!(world.spawn_character(tester_npc(1), 10, 10));

    let mut pent = item(9, ItemFlags::USE);
    pent.driver = IDR_PENT;
    pent.driver_data = vec![5, 255, 1, 0, 0]; // drdata[1] != 0: already active.
    assert!(world.map.set_item_map(&mut pent, 11, 10));
    world.items.insert(ItemId(9), pent);

    world.process_tester_actions(1);

    assert_eq!(tester_state(&world, CharacterId(1)).item_to_use, None);
}

#[test]
fn tester_regenerates_and_idles_instead_of_healing_when_hp_is_low() {
    // C: `regenerate_driver(cn)` (`hp < max_hp` -> idle) always runs
    // *before* `handle_tester_healing` (`hp < max_hp * heal_threshold` ->
    // drink), and the healing condition is strictly narrower, so
    // `regenerate_driver` always wins first - see module doc comment.
    let mut world = World::default();
    let mut tester = tester_npc(1);
    tester.values[0][CharacterValue::Hp as usize] = 10;
    tester.hp = POWERSCALE; // well below max, so regen intercepts first.
    assert!(world.spawn_character(tester, 10, 10));

    let acted = world.process_tester_actions(1);

    assert_eq!(acted, 1);
    let tester = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(tester.action, action::IDLE);
    assert_eq!(
        tester.hp, POWERSCALE,
        "regen only idles, it never grants hp directly"
    );
}
