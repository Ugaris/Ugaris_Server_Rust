// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use super::*;

#[test]
fn simple_baddy_attack_action_does_not_pulse_healthy_targets() {
    let mut world = World::default();
    world.tick = Tick(463);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = 100 * POWERSCALE;
    npc.values[0][CharacterValue::Mana as usize] = 100;
    npc.values[0][CharacterValue::Pulse as usize] = 200;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 11,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let mut target = character(2);
    target.hp = 100 * POWERSCALE;
    target.values[0][CharacterValue::Hp as usize] = 100;
    target.values[0][CharacterValue::Attack as usize] = 1;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::ATTACK1);
    assert_eq!(npc.mana, 100 * POWERSCALE);
}

#[test]
fn simple_baddy_attack_action_idles_when_already_at_flash_spacing_distance() {
    let mut world = World::default();
    world.tick = Tick(464);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = 4 * POWERSCALE;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.values[1][CharacterValue::Flash as usize] = 20;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 13,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let mut active_flash = item(50, ItemFlags::empty());
    active_flash.driver = IDR_FLASH;
    world.items.insert(active_flash.id, active_flash);
    npc.inventory[12] = Some(ItemId(50));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 13, 10);
    world.map.tile_mut(13, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::IDLE);
    assert_eq!(npc.duration, (TICKS_PER_SECOND / 4) as i32);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 464);
}

#[test]
fn simple_baddy_attack_action_does_not_distance_idle_without_active_flash_spell_slot() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = 4 * POWERSCALE;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.values[1][CharacterValue::Flash as usize] = 20;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 13,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 13, 10);
    world.map.tile_mut(13, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!(npc.tox, 11);
    assert_eq!(npc.toy, 10);
}

#[test]
fn simple_baddy_fireball_spacing_moves_toward_distance_seven() {
    let mut world = World::default();
    world.tick = Tick(466);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FIREBALL_COST + 1;
    npc.values[0][CharacterValue::Fireball as usize] = 1;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.values[1][CharacterValue::Fireball as usize] = 20;
    npc.values[1][CharacterValue::Flash as usize] = 5;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 20, 10);

    let target = world.characters.get(&CharacterId(2)).cloned().unwrap();
    assert!(world.setup_simple_baddy_fireball_distance_attack(CharacterId(1), &target, 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!(npc.tox, 11);
    assert_eq!(npc.toy, 10);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 466);
}

#[test]
fn simple_baddy_fireball_spacing_requires_fireball_above_flash() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FIREBALL_COST + 1;
    npc.values[0][CharacterValue::Fireball as usize] = 1;
    npc.values[1][CharacterValue::Fireball as usize] = 5;
    npc.values[1][CharacterValue::Flash as usize] = 5;
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 20, 10);

    let target = world.characters.get(&CharacterId(2)).cloned().unwrap();
    assert!(!world.setup_simple_baddy_fireball_distance_attack(CharacterId(1), &target, 1));
    assert_eq!(world.characters[&CharacterId(1)].action, 0);
}

#[test]
fn simple_baddy_distance_driver_uses_best_partial_when_exact_spacing_blocked() {
    let mut world = World::default();
    world.tick = Tick(467);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FIREBALL_COST + 1;
    npc.values[0][CharacterValue::Fireball as usize] = 1;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.values[1][CharacterValue::Fireball as usize] = 20;
    npc.values[1][CharacterValue::Flash as usize] = 5;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 20, 10);
    for y in 1..MAX_MAP - 1 {
        world.map.set_flags(13, y, MapFlags::MOVEBLOCK);
    }

    let target = world.characters.get(&CharacterId(2)).cloned().unwrap();
    assert!(world.setup_simple_baddy_fireball_distance_attack(CharacterId(1), &target, 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!(npc.tox, 11);
    assert_eq!(npc.toy, 10);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 467);
}

#[test]
fn simple_baddy_attack_action_attacks_moving_target_destination() {
    let mut world = World::default();
    world.tick = Tick(457);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 12,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let mut target = character(2);
    target.tox = 11;
    target.toy = 10;
    target.values[0][CharacterValue::Attack as usize] = 1;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 12, 10);
    world.map.tile_mut(12, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::ATTACK1);
    assert_eq!(npc.act1, 2);
    assert_eq!(npc.dir, Direction::Right as u8);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 457);
}

#[test]
fn simple_baddy_attack_action_walks_toward_visible_non_adjacent_enemies() {
    let mut world = World::default();
    world.tick = Tick(458);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 15,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 15, 10);
    world.map.tile_mut(15, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!(npc.tox, 11);
    assert_eq!(npc.toy, 10);
    assert_eq!(npc.dir, Direction::Right as u8);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 458);
}

#[test]
fn simple_baddy_attack_action_ignores_hurtme_priority_for_visible_score_like_c() {
    let mut world = World::default();
    world.tick = Tick(459);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![
            SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 999,
                visible: true,
                last_x: 14,
                last_y: 10,
            },
            SimpleBaddyEnemy {
                target_id: CharacterId(3),
                priority: 0,
                last_seen_tick: 1,
                visible: true,
                last_x: 10,
                last_y: 11,
            },
        ],
        ..SimpleBaddyDriverData::default()
    }));
    let mut hurt_target = character(2);
    hurt_target.values[0][CharacterValue::Attack as usize] = 1;
    let mut seen_target = character(3);
    seen_target.values[0][CharacterValue::Attack as usize] = 1;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(hurt_target, 14, 10);
    world.spawn_character(seen_target, 10, 11);
    world.map.tile_mut(14, 10).unwrap().light = 255;
    world.map.tile_mut(10, 11).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::ATTACK1);
    assert_eq!(npc.act1, 3);
}

#[test]
fn simple_baddy_attack_action_moves_to_target_back_when_front_is_occupied() {
    let mut world = World::default();
    world.tick = Tick(458);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 10,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let mut target = character(2);
    target.dir = Direction::Right as u8;
    let front_blocker = character(3);
    world.spawn_character(npc, 9, 9);
    world.spawn_character(target, 10, 10);
    world.spawn_character(front_blocker, 11, 10);
    world.map.tile_mut(10, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!(npc.tox, 9);
    assert_eq!(npc.toy, 10);
    assert_eq!(npc.dir, Direction::Down as u8);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 458);
}

#[test]
fn simple_baddy_attack_action_skips_back_move_when_back_tile_is_blocked() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 10,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let mut target = character(2);
    target.dir = Direction::Right as u8;
    let front_blocker = character(3);
    world.spawn_character(npc, 9, 9);
    world.spawn_character(target, 10, 10);
    world.spawn_character(front_blocker, 11, 10);
    world
        .map
        .tile_mut(9, 10)
        .unwrap()
        .flags
        .insert(MapFlags::MOVEBLOCK);
    world.map.tile_mut(10, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_ne!((npc.tox, npc.toy), (9, 10));
}

#[test]
fn simple_baddy_attack_back_move_rejects_front_position_like_c() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.group = 7;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let mut target = character(2);
    target.dir = Direction::Right as u8;
    world.spawn_character(npc, 11, 10);
    world.spawn_character(target.clone(), 10, 10);
    target.x = 10;
    target.y = 10;

    assert!(!world.setup_simple_baddy_attack_back_move(CharacterId(1), &target, 1));
}

#[test]
fn simple_baddy_attack_back_move_rejects_same_group_side_occupant_like_c() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.group = 7;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let mut target = character(2);
    target.dir = Direction::Right as u8;
    let front_blocker = character(3);
    let mut side_ally = character(4);
    side_ally.group = 7;
    world.spawn_character(npc, 9, 9);
    world.spawn_character(target.clone(), 10, 10);
    world.spawn_character(front_blocker, 11, 10);
    world.spawn_character(side_ally, 10, 11);
    target.x = 10;
    target.y = 10;

    assert!(!world.setup_simple_baddy_attack_back_move(CharacterId(1), &target, 1));
}

#[test]
fn simple_baddy_flee_action_scores_blocked_escape_path() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.endurance = 5 * POWERSCALE;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 13,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 13, 10);
    world.map.tile_mut(13, 10).unwrap().light = 255;
    world
        .map
        .tile_mut(8, 10)
        .unwrap()
        .flags
        .insert(MapFlags::MOVEBLOCK);

    assert!(world.setup_simple_baddy_flee_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert!(npc.tox < 10);
}

#[test]
fn simple_baddy_attack_action_uses_best_partial_path_when_target_unreachable() {
    let mut world = World::default();
    world.tick = Tick(460);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 15,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 15, 10);
    world.map.tile_mut(15, 10).unwrap().light = 255;
    for y in 1..MAX_MAP - 1 {
        world
            .map
            .tile_mut(12, y)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);
    }

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!((npc.tox, npc.toy), (11, 10));
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 460);
}

#[test]
fn simple_baddy_attack_action_uses_adjacent_blocker_when_path_fails() {
    let mut world = World::default();
    world.tick = Tick(461);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 13,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    let mut blocker = item(10, ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK);
    blocker.x = 11;
    blocker.y = 10;

    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 13, 10);
    world.map.tile_mut(13, 10).unwrap().light = 255;
    world.items.insert(blocker.id, blocker);
    let tile = world.map.tile_mut(11, 10).unwrap();
    tile.item = 10;
    tile.flags.insert(MapFlags::TMOVEBLOCK);
    for y in 1..MAX_MAP - 1 {
        world
            .map
            .tile_mut(12, y)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);
    }

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::USE);
    assert_eq!(npc.dir, Direction::Right as u8);
    assert_eq!(npc.act1, 10);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 461);
}

#[test]
fn simple_baddy_attack_action_idles_when_unreachable_path_does_not_improve() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 15,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 15, 10);
    world.map.tile_mut(15, 10).unwrap().light = 255;
    for y in 1..MAX_MAP - 1 {
        world
            .map
            .tile_mut(11, y)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);
    }

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::IDLE);
    assert_eq!(npc.duration, (TICKS_PER_SECOND / 4) as i32);
}

#[test]
fn distance_driver_prefers_moving_target_position_like_c() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.values[0][CharacterValue::Speed as usize] = 50;
    let mut target = character(2);
    target.tox = 10;
    target.toy = 14;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 16, 10);
    world.map.tile_mut(16, 10).unwrap().light = 255;

    assert!(world.distance_driver(CharacterId(1), CharacterId(2), 1, 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!((npc.tox, npc.toy), (10, 11));
    assert_eq!(npc.dir, Direction::Down as u8);
}

#[test]
fn distance_driver_returns_false_when_already_at_requested_distance() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.values[0][CharacterValue::Speed as usize] = 50;
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 18, 10);
    world.map.tile_mut(18, 10).unwrap().light = 255;

    assert!(!world.distance_driver(CharacterId(1), CharacterId(2), 8, 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, 0);
}

#[test]
fn distance_driver_uses_best_partial_path_when_exact_distance_unreachable() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.values[0][CharacterValue::Speed as usize] = 50;
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 15, 10);
    world.map.tile_mut(15, 10).unwrap().light = 255;
    for y in 1..MAX_MAP - 1 {
        world
            .map
            .tile_mut(12, y)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);
    }

    assert!(world.distance_driver(CharacterId(1), CharacterId(2), 1, 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!((npc.tox, npc.toy), (11, 10));
}

#[test]
fn simple_baddy_attack_action_uses_explicit_fight_driver_home_for_stop_distance() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.rest_x = 10;
    npc.rest_y = 10;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        stopdist: 6,
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 14,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 14, 10);
    world.map.tile_mut(14, 10).unwrap().light = 255;
    assert!(world.set_simple_baddy_home(CharacterId(1), 14, 10));

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    let data = npc
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert_eq!(data.home_x, 14);
    assert_eq!(data.home_y, 10);
    assert_eq!(data.enemies.len(), 1);
}

#[test]
fn simple_baddy_notsecure_day_post_walks_to_rest_home_like_c() {
    let mut world = World::default();
    world.tick = Tick(TICKS_PER_SECOND * 2);
    world.date.hour = 12;
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.rest_x = 15;
    npc.rest_y = 10;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        dayx: 30,
        dayy: 10,
        nightx: 35,
        nighty: 10,
        notsecure: 1,
        ..SimpleBaddyDriverData::default()
    }));
    world.spawn_character(npc, 10, 10);

    assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!((npc.tox, npc.toy), (11, 10));
    assert_eq!(npc.dir, Direction::Right as u8);
}

#[test]
fn simple_baddy_drinkspecial_removes_poison_when_poison0_is_active() {
    let mut world = World::default();
    world.tick = Tick(TICKS_PER_SECOND * 2);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.hp = 10 * POWERSCALE;
    npc.mana = 10 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 10;
    npc.values[0][CharacterValue::Mana as usize] = 10;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        drinkspecial: 1,
        ..SimpleBaddyDriverData::default()
    }));
    let mut poison0 = item(10, ItemFlags::empty());
    poison0.driver = IDR_POISON0;
    let mut poison1 = item(11, ItemFlags::empty());
    poison1.driver = IDR_POISON1;
    npc.inventory[SPELL_SLOT_START] = Some(poison0.id);
    npc.inventory[SPELL_SLOT_START + 1] = Some(poison1.id);
    world.items.insert(poison0.id, poison0);
    world.items.insert(poison1.id, poison1);
    world.spawn_character(npc, 10, 10);

    assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert!(npc.inventory[SPELL_SLOT_START].is_none());
    assert!(npc.inventory[SPELL_SLOT_START + 1].is_none());
    assert!(!world.items.contains_key(&ItemId(10)));
    assert!(!world.items.contains_key(&ItemId(11)));
    assert!(npc
        .flags
        .contains(CharacterFlags::ITEMS | CharacterFlags::UPDATE));
    assert_eq!(npc.action, action::IDLE);
    assert_eq!(
        world.drain_pending_area_texts(),
        vec![WorldAreaText {
            x: 10,
            y: 10,
            max_distance: (SAY_DIST / 2) as u16,
            message: "Character drinks a potion.".to_string(),
        }]
    );
}

#[test]
fn simple_baddy_at_day_post_drinkspecial_runs_before_idle() {
    let mut world = World::default();
    world.tick = Tick(TICKS_PER_SECOND * 2);
    world.date.hour = 12;
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.hp = 10 * POWERSCALE;
    npc.mana = 10 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 10;
    npc.values[0][CharacterValue::Mana as usize] = 10;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        dayx: 10,
        dayy: 10,
        daydir: Direction::Down as i32,
        drinkspecial: 1,
        ..SimpleBaddyDriverData::default()
    }));
    let mut poison0 = item(10, ItemFlags::empty());
    poison0.driver = IDR_POISON0;
    npc.inventory[SPELL_SLOT_START] = Some(poison0.id);
    world.items.insert(poison0.id, poison0);
    world.spawn_character(npc, 10, 10);

    assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.dir, Direction::Down as u8);
    assert!(npc.inventory[SPELL_SLOT_START].is_none());
    assert!(!world.items.contains_key(&ItemId(10)));
    assert_eq!(npc.action, action::IDLE);
}

#[test]
fn simple_baddy_drinkspecial_requires_poison0_trigger() {
    let mut world = World::default();
    world.tick = Tick(TICKS_PER_SECOND * 2);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        drinkspecial: 1,
        ..SimpleBaddyDriverData::default()
    }));
    let mut poison1 = item(11, ItemFlags::empty());
    poison1.driver = IDR_POISON1;
    npc.inventory[SPELL_SLOT_START] = Some(poison1.id);
    world.items.insert(poison1.id, poison1);
    world.spawn_character(npc, 10, 10);

    assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.inventory[SPELL_SLOT_START], Some(ItemId(11)));
    assert!(world.items.contains_key(&ItemId(11)));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn simple_baddy_death_driver_creates_earth_demon_effects_at_killer() {
    let mut world = World::default();
    let mut dead = character(1);
    dead.driver = CDR_SIMPLEBADDY;
    dead.flags.insert(CharacterFlags::EDEMON);
    dead.flags.insert(CharacterFlags::GOD);
    dead.values[1][CharacterValue::Demon as usize] = 6;
    let killer = character(2);
    assert!(world.spawn_character(dead, 10, 10));
    assert!(world.spawn_character(killer, 12, 10));
    world.map.tile_mut(12, 10).unwrap().light = 255;

    let effect_ids = world.apply_character_death_driver(CharacterId(1), CharacterId(2));

    assert_eq!(effect_ids.len(), 2);
    let mud = world.effects.get(&effect_ids[0]).unwrap();
    assert_eq!(mud.effect_type, EF_EARTHMUD);
    assert_eq!(mud.strength, 6);
    let rain = world.effects.get(&effect_ids[1]).unwrap();
    assert_eq!(rain.effect_type, EF_EARTHRAIN);
    assert_eq!(rain.strength, 6);
    let killer_tile = world.map.tile(12, 10).unwrap();
    assert!(killer_tile.effects.contains(&(effect_ids[0] as u16)));
    assert!(killer_tile.effects.contains(&(effect_ids[1] as u16)));
}

#[test]
fn simple_baddy_death_driver_respects_earth_demon_gates() {
    let mut world = World::default();
    let mut dead = character(1);
    dead.driver = CDR_SIMPLEBADDY;
    dead.flags.insert(CharacterFlags::EDEMON);
    dead.flags.insert(CharacterFlags::GOD);
    dead.values[1][CharacterValue::Demon as usize] = 5;
    let killer = character(2);
    assert!(world.spawn_character(dead, 10, 10));
    assert!(world.spawn_character(killer, 12, 10));
    world.map.tile_mut(12, 10).unwrap().light = 255;

    let effect_ids = world.apply_simple_baddy_death_driver(CharacterId(1), CharacterId(2));

    assert_eq!(effect_ids.len(), 1);
    assert_eq!(world.effects[&effect_ids[0]].effect_type, EF_EARTHRAIN);

    world
        .map
        .tile_mut(11, 10)
        .unwrap()
        .flags
        .insert(MapFlags::SIGHTBLOCK);
    let effect_ids = world.apply_simple_baddy_death_driver(CharacterId(1), CharacterId(2));

    assert!(effect_ids.is_empty());
}

#[test]
fn legacy_hurt_invokes_simple_baddy_death_driver_for_earth_demons() {
    let mut world = World::default();
    let mut dead = character(1);
    dead.driver = CDR_SIMPLEBADDY;
    dead.flags.insert(CharacterFlags::EDEMON);
    dead.flags.insert(CharacterFlags::GOD);
    dead.values[1][CharacterValue::Demon as usize] = 6;
    dead.hp = POWERSCALE;
    let killer = character(2);
    assert!(world.spawn_character(dead, 10, 10));
    assert!(world.spawn_character(killer, 12, 10));
    world.map.tile_mut(12, 10).unwrap().light = 255;

    let outcome = world
        .apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), POWERSCALE, 1, 0, 0)
        .unwrap();

    assert!(outcome.killed);
    let dead = world.characters.get(&CharacterId(1)).unwrap();
    assert!(dead.flags.contains(CharacterFlags::DEAD));
    assert!(world
        .effects
        .values()
        .any(|effect| effect.effect_type == EF_EARTHRAIN && effect.strength == 6));
    assert!(world
        .effects
        .values()
        .any(|effect| effect.effect_type == EF_EARTHMUD && effect.strength == 6));
}

#[test]
fn sound_area_specials_match_legacy_distance_and_pan() {
    let mut world = World {
        map: MapGrid::new(40, 40),
        ..World::default()
    };
    let mut nearby = character(1);
    nearby.flags.insert(CharacterFlags::PLAYER);
    nearby.x = 13;
    nearby.y = 14;
    let mut outside = character(2);
    outside.flags.insert(CharacterFlags::PLAYER);
    outside.x = 31;
    outside.y = 10;
    let mut npc = character(3);
    npc.x = 12;
    npc.y = 10;

    world.add_character(nearby);
    world.add_character(outside);
    world.add_character(npc);

    let specials = world.sound_area_specials(10, 10, 7);

    assert_eq!(specials.len(), 1);
    assert_eq!(specials[0].character_id, CharacterId(1));
    assert_eq!(specials[0].special.special_type, 7);
    assert_eq!(specials[0].special.opt1, -250);
    assert_eq!(specials[0].special.opt2, 300);
}

// `process_lostcon_attack_action_with_random` (C `lostcon_driver`'s
// `fight_driver_update(cn); if (fight_driver_attack_visible(cn,
// ppd->nomove)) return; if (!ppd->nomove &&
// fight_driver_follow_invisible(cn)) return;` cascade, `lostcon.c:200-203`)
// reuses `fight_driver_attack_visible_and_follow`'s generalized engine
// (see `PORTING_TODO.md`'s "Player-side fight-driver auto-combat" task).

#[test]
fn lostcon_attack_action_ignores_a_normal_playing_character() {
    let mut world = World::default();
    let npc = character(1);
    world.spawn_character(npc, 10, 10);

    assert!(!world.process_lostcon_attack_action_with_random(
        CharacterId(1),
        1,
        FightDriverSuppressions::default(),
        |_| 0,
    ));
}

#[test]
fn lostcon_attack_action_fights_back_a_visible_enemy() {
    let mut world = World::default();
    let mut lingering = character(1);
    lingering.driver = CDR_LOSTCON;
    lingering.driver_state = Some(CharacterDriverState::Lostcon(LostconDriverData {
        deadline: 1_000,
    }));
    lingering.fight_driver = Some(FightDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 1,
            visible: true,
            last_x: 11,
            last_y: 10,
        }],
        ..FightDriverData::default()
    });
    let target = character(2);
    world.spawn_character(lingering, 10, 10);
    world.spawn_character(target, 11, 10);

    assert!(world.process_lostcon_attack_action_with_random(
        CharacterId(1),
        1,
        FightDriverSuppressions::default(),
        |_| 0,
    ));

    let lingering = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(lingering.action, action::ATTACK1);
}

#[test]
fn lostcon_attack_action_nomove_suppresses_attack_task_and_invisible_follow() {
    // C: `!ppd->nomove` gates both the `Attack` task inside
    // `fight_driver_attack_enemy` (`drvlib.c:1682`'s own `if (!nomove ||
    // dist(cn,co)==2)` guard) and the whole `fight_driver_follow_invisible`
    // call. A lone adjacent enemy with nothing else to do means "nomove"
    // leaves the lingering character with no action to take at all.
    let mut world = World::default();
    let mut lingering = character(1);
    lingering.driver = CDR_LOSTCON;
    lingering.driver_state = Some(CharacterDriverState::Lostcon(LostconDriverData {
        deadline: 1_000,
    }));
    lingering.fight_driver = Some(FightDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 1,
            visible: false,
            last_x: 15,
            last_y: 10,
        }],
        ..FightDriverData::default()
    });
    let mut target = character(2);
    target.flags.insert(CharacterFlags::INVISIBLE);
    world.spawn_character(lingering, 10, 10);
    world.spawn_character(target, 15, 10);

    assert!(!world.process_lostcon_attack_action_with_random(
        CharacterId(1),
        1,
        FightDriverSuppressions {
            nomove: true,
            ..FightDriverSuppressions::default()
        },
        |_| 0,
    ));

    let lingering = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(lingering.action, action::IDLE);
}

#[test]
fn lostcon_attack_action_follows_an_invisible_enemy_toward_its_last_position() {
    let mut world = World::default();
    let mut lingering = character(1);
    lingering.driver = CDR_LOSTCON;
    lingering.driver_state = Some(CharacterDriverState::Lostcon(LostconDriverData {
        deadline: 1_000,
    }));
    lingering.fight_driver = Some(FightDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 1,
            visible: false,
            last_x: 15,
            last_y: 10,
        }],
        ..FightDriverData::default()
    });
    let mut target = character(2);
    target.flags.insert(CharacterFlags::INVISIBLE);
    world.spawn_character(lingering, 10, 10);
    world.spawn_character(target, 15, 10);

    assert!(world.process_lostcon_attack_action_with_random(
        CharacterId(1),
        1,
        FightDriverSuppressions::default(),
        |_| 0,
    ));

    let lingering = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(lingering.action, action::WALK);
}

#[test]
fn two_robber_npc_attacks_visible_enemy_via_reused_simple_baddy_dispatch() {
    // C's `ch_driver`'s `CDR_TWOROBBER` dispatch (`two.c:3163-3165`) is an
    // unconditional tail call to `char_driver(CDR_SIMPLEBADDY, ...)`, so
    // `process_simple_baddy_attack_action`'s driver gate must accept
    // `CDR_TWOROBBER` (not just `CDR_SIMPLEBADDY`) for a robber to fight
    // back - same precedent as `CDR_PENTER`/`CDR_FORESTMONSTER` above.
    let mut world = World::default();
    let mut robber = character(1);
    robber.driver = crate::character_driver::CDR_TWOROBBER;
    robber.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 0,
            visible: true,
            last_x: 15,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    assert!(world.spawn_character(robber, 10, 10));
    assert!(world.spawn_character(target, 15, 10));
    world.map.tile_mut(15, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_ne!(npc.action, 0);

    // The aggregate dispatch must also pick up `CDR_TWOROBBER` characters.
    let mut world = World::default();
    let mut robber = character(1);
    robber.driver = crate::character_driver::CDR_TWOROBBER;
    robber.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 0,
            visible: true,
            last_x: 15,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    assert!(world.spawn_character(robber, 10, 10));
    assert!(world.spawn_character(target, 15, 10));
    world.map.tile_mut(15, 10).unwrap().light = 255;
    let attacks = world.process_simple_baddy_attack_actions_with_random(1, |_| 0);
    assert_eq!(attacks, 1);
}

#[test]
fn teufelrat_npc_attacks_visible_enemy_via_reused_simple_baddy_dispatch() {
    // C's `ch_driver`'s `CDR_TEUFELRAT` dispatch (`teufel.c:1610-1626`) is
    // effectively a pure unconditional tail call to
    // `char_driver(CDR_SIMPLEBADDY, ...)` (its own `NT_CHAR` case body is
    // empty), so `process_simple_baddy_attack_action`'s driver gate must
    // accept `CDR_TEUFELRAT` too - same precedent as `CDR_TEUFELDEMON`/
    // `CDR_TWOROBBER` above.
    let mut world = World::default();
    let mut rat = character(1);
    rat.driver = crate::character_driver::CDR_TEUFELRAT;
    rat.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 0,
            visible: true,
            last_x: 15,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    assert!(world.spawn_character(rat, 10, 10));
    assert!(world.spawn_character(target, 15, 10));
    world.map.tile_mut(15, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_ne!(npc.action, 0);

    // The aggregate dispatch must also pick up `CDR_TEUFELRAT` characters.
    let mut world = World::default();
    let mut rat = character(1);
    rat.driver = crate::character_driver::CDR_TEUFELRAT;
    rat.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 0,
            visible: true,
            last_x: 15,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    assert!(world.spawn_character(rat, 10, 10));
    assert!(world.spawn_character(target, 15, 10));
    world.map.tile_mut(15, 10).unwrap().light = 255;
    let attacks = world.process_simple_baddy_attack_actions_with_random(1, |_| 0);
    assert_eq!(attacks, 1);
}
