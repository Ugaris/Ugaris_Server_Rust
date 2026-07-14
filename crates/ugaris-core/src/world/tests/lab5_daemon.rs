// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use super::*;
use crate::character_driver::{
    apply_lab5_daemon_create_message, Lab5DaemonDriverData, CDR_LAB5DAEMON, NT_CHAR, NT_CREATE,
    NT_GOTHIT,
};
use crate::item_driver::IID_LAB5_WEAPON;

/// C `WN_RHAND`, same slot index every other ported area-8/22 driver uses.
const WN_RHAND: usize = 6;

fn daemon_npc(id: u32, daemon_type: u8) -> Character {
    let mut daemon = character(id);
    daemon.name = "Asfaloth".into();
    daemon.driver = CDR_LAB5DAEMON;
    daemon.group = 1;
    apply_lab5_daemon_create_message(&mut daemon, Some(&format!("type={daemon_type};")));
    daemon
}

fn daemon_state(world: &World, id: CharacterId) -> Lab5DaemonDriverData {
    match world
        .characters
        .get(&id)
        .and_then(|c| c.driver_state.clone())
    {
        Some(CharacterDriverState::Lab5Daemon(data)) => data,
        _ => panic!("expected lab5 daemon driver state"),
    }
}

#[test]
fn apply_lab5_daemon_create_message_parses_type_and_pushes_nt_create() {
    let mut character = character(1);
    apply_lab5_daemon_create_message(&mut character, Some("type=1;"));

    let Some(CharacterDriverState::Lab5Daemon(data)) = character.driver_state else {
        panic!("expected lab5 daemon driver state");
    };
    assert_eq!(data.daemon_type, 1);
    assert_eq!(data.dir, Direction::Down as u8);
    assert_eq!(
        character
            .driver_messages
            .iter()
            .filter(|message| message.message_type == NT_CREATE)
            .count(),
        1
    );
}

#[test]
fn apply_lab5_daemon_create_message_gunned_type_faces_left() {
    let mut character = character(1);
    apply_lab5_daemon_create_message(&mut character, Some("type=2;"));

    let Some(CharacterDriverState::Lab5Daemon(data)) = character.driver_state else {
        panic!("expected lab5 daemon driver state");
    };
    assert_eq!(data.daemon_type, 2);
    assert_eq!(data.dir, Direction::Left as u8);
}

#[test]
fn apply_lab5_daemon_create_message_no_args_defaults_to_type_0() {
    let mut character = character(1);
    apply_lab5_daemon_create_message(&mut character, None);
    let Some(CharacterDriverState::Lab5Daemon(data)) = character.driver_state else {
        panic!("expected lab5 daemon driver state");
    };
    assert_eq!(data.daemon_type, 0);
}

#[test]
fn nt_create_master_sets_attackstart_from_ticker() {
    let mut world = World::default();
    world.tick = Tick(1000);
    assert!(world.spawn_character(daemon_npc(1, 1), 20, 20));

    world.process_lab5_daemon_actions(1);

    let state = daemon_state(&world, CharacterId(1));
    assert_eq!(state.attackstart, 1000);
}

#[test]
fn nt_create_gunned_never_becomes_aggressive() {
    let mut world = World::default();
    world.tick = Tick(1000);
    assert!(world.spawn_character(daemon_npc(1, 2), 20, 20));

    world.process_lab5_daemon_actions(1);
    let state = daemon_state(&world, CharacterId(1));
    assert_eq!(state.attackstart, u64::MAX);

    // Ticking forward far still never crosses `u64::MAX`.
    world.tick = Tick(50_000);
    world.process_lab5_daemon_actions(1);
    let state = daemon_state(&world, CharacterId(1));
    assert!(!state.aggressive);
}

#[test]
fn servant_becomes_aggressive_once_ticker_passes_attackstart() {
    let mut world = World::default();
    world.tick = Tick(1000);
    assert!(world.spawn_character(daemon_npc(1, 0), 20, 20));
    world.process_lab5_daemon_actions(1); // NT_CREATE: attackstart = 1000

    world.tick = Tick(1001);
    world.process_lab5_daemon_actions(1);
    let state = daemon_state(&world, CharacterId(1));
    assert!(state.aggressive);
}

#[test]
fn master_without_sacred_weapon_becomes_immortal() {
    let mut world = World::default();
    let mut daemon = daemon_npc(1, 1);
    daemon.driver_state = Some(CharacterDriverState::Lab5Daemon(Lab5DaemonDriverData {
        daemon_type: 1,
        ..Default::default()
    }));
    daemon.x = 20;
    daemon.y = 20;
    world.add_character(daemon);

    let mut player = character(2);
    player.x = 20;
    player.y = 21;
    player.flags |= CharacterFlags::PLAYER;
    world.add_character(player);

    world.map.tile_mut(20, 20).unwrap().light = 255;
    world.map.tile_mut(20, 21).unwrap().light = 255;

    if let Some(daemon) = world.characters.get_mut(&CharacterId(1)) {
        daemon.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_lab5_daemon_actions(1);

    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .flags
        .contains(CharacterFlags::IMMORTAL));
}

#[test]
fn master_with_sacred_weapon_in_rhand_is_not_immortal() {
    let mut world = World::default();
    let mut daemon = daemon_npc(1, 1);
    daemon.driver_state = Some(CharacterDriverState::Lab5Daemon(Lab5DaemonDriverData {
        daemon_type: 1,
        ..Default::default()
    }));
    // Start immortal (as if a previous tick had no visible unarmed
    // player) to prove the weapon check actively clears it.
    daemon.flags.insert(CharacterFlags::IMMORTAL);
    daemon.x = 20;
    daemon.y = 20;
    world.add_character(daemon);

    let mut player = character(2);
    player.x = 20;
    player.y = 21;
    player.flags |= CharacterFlags::PLAYER;
    player.inventory[WN_RHAND] = Some(ItemId(50));
    world.add_character(player);
    let mut weapon = item(50, ItemFlags::empty());
    weapon.template_id = IID_LAB5_WEAPON;
    weapon.carried_by = Some(CharacterId(2));
    world.add_item(weapon);

    world.map.tile_mut(20, 20).unwrap().light = 255;
    world.map.tile_mut(20, 21).unwrap().light = 255;

    if let Some(daemon) = world.characters.get_mut(&CharacterId(1)) {
        daemon.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_lab5_daemon_actions(1);

    assert!(!world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .flags
        .contains(CharacterFlags::IMMORTAL));
}

#[test]
fn servant_never_becomes_immortal() {
    let mut world = World::default();
    let mut daemon = daemon_npc(1, 0);
    daemon.driver_state = Some(CharacterDriverState::Lab5Daemon(Lab5DaemonDriverData {
        daemon_type: 0,
        ..Default::default()
    }));
    daemon.flags.insert(CharacterFlags::IMMORTAL);
    daemon.x = 20;
    daemon.y = 20;
    world.add_character(daemon);

    let mut player = character(2);
    player.x = 20;
    player.y = 21;
    player.flags |= CharacterFlags::PLAYER;
    world.add_character(player);

    world.map.tile_mut(20, 20).unwrap().light = 255;
    world.map.tile_mut(20, 21).unwrap().light = 255;

    if let Some(daemon) = world.characters.get_mut(&CharacterId(1)) {
        daemon.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_lab5_daemon_actions(1);

    assert!(!world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .flags
        .contains(CharacterFlags::IMMORTAL));
}

#[test]
fn gunned_demon_adds_visible_player_north_of_aggro_line_as_enemy() {
    let mut world = World::default();
    let mut daemon = daemon_npc(1, 2);
    daemon.driver_state = Some(CharacterDriverState::Lab5Daemon(Lab5DaemonDriverData {
        daemon_type: 2,
        dir: Direction::Left as u8,
        attackstart: u64::MAX,
        ..Default::default()
    }));
    daemon.x = 20;
    daemon.y = 20;
    world.add_character(daemon);

    let mut player = character(2);
    player.x = 20;
    player.y = 21;
    player.flags |= CharacterFlags::PLAYER;
    world.add_character(player);

    world.map.tile_mut(20, 20).unwrap().light = 255;
    world.map.tile_mut(20, 21).unwrap().light = 255;

    if let Some(daemon) = world.characters.get_mut(&CharacterId(1)) {
        daemon.push_driver_message(NT_CHAR, 2, 0, 0);
    }

    world.process_lab5_daemon_actions(1);

    let state = daemon_state(&world, CharacterId(1));
    assert_eq!(state.victim, Some(CharacterId(2)));
}

#[test]
fn gothit_sets_victim_for_self_defense() {
    let mut world = World::default();
    let mut daemon = daemon_npc(1, 0);
    daemon.driver_state = Some(CharacterDriverState::Lab5Daemon(Lab5DaemonDriverData {
        daemon_type: 0,
        ..Default::default()
    }));
    daemon.group = 1;
    world.add_character(daemon);

    let mut attacker = character(2);
    attacker.group = 2;
    attacker.flags |= CharacterFlags::PLAYER;
    world.add_character(attacker);

    if let Some(daemon) = world.characters.get_mut(&CharacterId(1)) {
        daemon.push_driver_message(NT_GOTHIT, 2, 0, 0);
    }

    world.process_lab5_daemon_actions(1);

    let state = daemon_state(&world, CharacterId(1));
    assert_eq!(state.victim, Some(CharacterId(2)));
}

#[test]
fn process_lab5_daemon_actions_ignores_non_lab5_daemon_characters() {
    let mut world = World::default();
    world.add_character(character(1));
    let acted = world.process_lab5_daemon_actions(1);
    assert_eq!(acted, 0);
}
