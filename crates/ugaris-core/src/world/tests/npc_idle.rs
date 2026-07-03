use super::*;

#[test]
fn simple_baddy_fight_tasks_skip_regeneration_in_area_33_like_c() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.hp = POWERSCALE;
    npc.mana = POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 10;
    npc.values[0][CharacterValue::Mana as usize] = 10;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);

    let area_one_tasks = world.simple_baddy_fight_tasks(
        CharacterId(1),
        world.characters.get(&CharacterId(2)).unwrap(),
        1,
        false,
    );
    let area_thirty_three_tasks = world.simple_baddy_fight_tasks(
        CharacterId(1),
        world.characters.get(&CharacterId(2)).unwrap(),
        33,
        false,
    );

    assert!(area_one_tasks
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Regenerate));
    assert!(!area_thirty_three_tasks
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Regenerate));
}

#[test]
fn simple_baddy_attack_action_idles_to_regenerate_during_fight() {
    let mut world = World::default();
    world.tick = Tick(453);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.hp = 90 * POWERSCALE;
    npc.mana = 100 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 100;
    npc.values[0][CharacterValue::Mana as usize] = 100;
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
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::IDLE);
    assert_eq!(npc.duration, (TICKS_PER_SECOND / 2) as i32);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 453);
}

#[test]
fn simple_baddy_noncombat_action_idles_shortly_after_creation() {
    let mut world = World::default();
    world.tick = Tick(3);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        creation_time: 0,
        ..SimpleBaddyDriverData::default()
    }));
    world.spawn_character(npc, 10, 10);

    assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::IDLE);
    assert_eq!(npc.duration, (TICKS_PER_SECOND / 4) as i32);
}

#[test]
fn simple_baddy_noncombat_action_teleports_to_night_post_and_sets_home() {
    let mut world = World::default();
    world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
    world.date.hour = 21;
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        dayx: 20,
        dayy: 10,
        nightx: 15,
        nighty: 10,
        teleport: 1,
        ..SimpleBaddyDriverData::default()
    }));
    world.spawn_character(npc, 10, 10);

    assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((npc.x, npc.y), (15, 10));
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!((data.home_x, data.home_y), (15, 10));
}

#[test]
fn simple_baddy_noncombat_action_turns_to_day_post_direction() {
    let mut world = World::default();
    world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
    world.date.hour = 12;
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.dir = Direction::Left as u8;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        dayx: 10,
        dayy: 10,
        daydir: Direction::Down as i32,
        nightx: 15,
        nighty: 10,
        nightdir: Direction::Up as i32,
        ..SimpleBaddyDriverData::default()
    }));
    world.spawn_character(npc, 10, 10);

    assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.dir, Direction::Down as u8);
    assert_eq!(npc.action, action::IDLE);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!((data.home_x, data.home_y), (10, 10));
}

#[test]
fn simple_baddy_noncombat_action_walks_back_to_rest_home() {
    let mut world = World::default();
    world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.rest_x = 15;
    npc.rest_y = 10;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    world.spawn_character(npc, 10, 10);

    assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!((npc.tox, npc.toy), (11, 10));
    assert_eq!(npc.dir, Direction::Right as u8);
}

#[test]
fn secure_move_driver_turns_at_target_without_claiming_action() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.dir = Direction::Left as u8;
    world.spawn_character(npc, 10, 10);

    assert!(!world.secure_move_driver(CharacterId(1), 10, 10, Direction::Down as u8, 0, 0, 1,));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.dir, Direction::Down as u8);
    assert_eq!(npc.action, 0);
}

#[test]
fn secure_move_driver_skips_move_after_blocked_use_and_teleports() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.values[0][CharacterValue::Speed as usize] = 50;
    world.spawn_character(npc, 10, 10);
    world
        .map
        .tile_mut(11, 10)
        .unwrap()
        .flags
        .insert(MapFlags::MOVEBLOCK);

    assert!(world.secure_move_driver(
        CharacterId(1),
        12,
        10,
        Direction::Right as u8,
        2,
        action::USE,
        1,
    ));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((npc.x, npc.y), (12, 10));
    assert_eq!(npc.action, 0);
}

#[test]
fn simple_baddy_noncombat_threads_failed_use_into_secure_move() {
    let mut world = World::default();
    world.tick = Tick((TICKS_PER_SECOND * 20) as u64);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.rest_x = 12;
    npc.rest_y = 10;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    world.spawn_character(npc, 10, 10);

    let completions = [WorldActionCompletion {
        character_id: CharacterId(1),
        action_id: action::USE,
        action_item_id: None,
        ok: false,
        legacy_return_code: 2,
        item_use: None,
        old_x: 10,
        old_y: 10,
        new_x: 10,
        new_y: 10,
    }];

    assert_eq!(
        world.process_simple_baddy_noncombat_actions_with_completions(1, &completions),
        1
    );

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((npc.x, npc.y), (12, 10));
    assert_eq!(npc.action, 0);
}

#[test]
fn simple_baddy_noncombat_failed_use_without_retry_code_still_walks() {
    let mut world = World::default();
    world.tick = Tick((TICKS_PER_SECOND * 20) as u64);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.rest_x = 12;
    npc.rest_y = 10;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    world.spawn_character(npc, 10, 10);

    let completions = [WorldActionCompletion {
        character_id: CharacterId(1),
        action_id: action::USE,
        action_item_id: None,
        ok: false,
        legacy_return_code: 0,
        item_use: None,
        old_x: 10,
        old_y: 10,
        new_x: 10,
        new_y: 10,
    }];

    assert_eq!(
        world.process_simple_baddy_noncombat_actions_with_completions(1, &completions),
        1
    );

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!((npc.tox, npc.toy), (11, 10));
}

#[test]
fn secure_move_driver_walks_before_teleport_when_not_blocked_use() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.values[0][CharacterValue::Speed as usize] = 50;
    world.spawn_character(npc, 10, 10);

    assert!(world.secure_move_driver(CharacterId(1), 12, 10, Direction::Right as u8, 0, 0, 1,));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!((npc.tox, npc.toy), (11, 10));
}

#[test]
fn simple_baddy_scavenger_idles_on_legacy_random_gate() {
    let mut world = World::default();
    world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.rest_x = 10;
    npc.rest_y = 10;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        scavenger: 4,
        dir: 0,
        ..SimpleBaddyDriverData::default()
    }));
    world.spawn_character(npc, 10, 10);

    assert!(world.process_simple_baddy_noncombat_action_with_random(CharacterId(1), 1, |_| 0));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::IDLE);
    assert_eq!(npc.duration, TICKS_PER_SECOND as i32);
}

#[test]
fn simple_baddy_scavenger_uses_secure_move_when_returning_home() {
    let mut world = World::default();
    world.tick = Tick((TICKS_PER_SECOND * 20) as u64);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.rest_x = 12;
    npc.rest_y = 10;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        scavenger: 2,
        ..SimpleBaddyDriverData::default()
    }));
    world.spawn_character(npc, 10, 10);
    world
        .map
        .tile_mut(11, 10)
        .unwrap()
        .flags
        .insert(MapFlags::MOVEBLOCK);

    let completions = [WorldActionCompletion {
        character_id: CharacterId(1),
        action_id: action::USE,
        action_item_id: None,
        ok: false,
        legacy_return_code: 2,
        item_use: None,
        old_x: 10,
        old_y: 10,
        new_x: 10,
        new_y: 10,
    }];

    assert_eq!(
        world.process_simple_baddy_noncombat_actions_with_completions(1, &completions),
        1
    );

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((npc.x, npc.y), (12, 10));
    assert_eq!(npc.action, action::IDLE);
}

#[test]
fn simple_baddy_scavenger_recent_fight_does_not_secure_teleport_home() {
    let mut world = World::default();
    world.tick = Tick((TICKS_PER_SECOND * 20) as u64);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.rest_x = 12;
    npc.rest_y = 10;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        scavenger: 2,
        dir: Direction::Left as i32,
        lastfight: (TICKS_PER_SECOND * 15) as i32,
        ..SimpleBaddyDriverData::default()
    }));
    world.spawn_character(npc, 10, 10);
    world
        .map
        .tile_mut(11, 10)
        .unwrap()
        .flags
        .insert(MapFlags::MOVEBLOCK);

    let completions = [WorldActionCompletion {
        character_id: CharacterId(1),
        action_id: action::USE,
        action_item_id: None,
        ok: false,
        legacy_return_code: 2,
        item_use: None,
        old_x: 10,
        old_y: 10,
        new_x: 10,
        new_y: 10,
    }];

    assert_eq!(
        world.process_simple_baddy_noncombat_actions_with_completions(1, &completions),
        1
    );

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((npc.x, npc.y), (10, 10));
    assert_eq!(npc.action, action::WALK);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.dir, 0);
}

#[test]
fn simple_baddy_scavenger_randomly_walks_inside_home_bounds() {
    let mut world = World::default();
    world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.rest_x = 10;
    npc.rest_y = 10;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        scavenger: 4,
        dir: 0,
        ..SimpleBaddyDriverData::default()
    }));
    world.spawn_character(npc, 10, 10);
    let mut rolls = [1, 0].into_iter();

    assert!(
        world.process_simple_baddy_noncombat_action_with_random(CharacterId(1), 1, |_| {
            rolls.next().unwrap_or(0)
        })
    );

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!((npc.tox, npc.toy), (11, 10));
    assert_eq!(npc.dir, Direction::Right as u8);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.dir, Direction::Right as i32);
    assert_eq!((data.home_x, data.home_y), (10, 10));
}

#[test]
fn simple_baddy_bulk_noncombat_uses_legacy_rng_seed_for_wander() {
    let mut world = World::default();
    world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
    world.legacy_random_seed = 0;
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.rest_x = 10;
    npc.rest_y = 10;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        scavenger: 4,
        dir: 0,
        ..SimpleBaddyDriverData::default()
    }));
    world.spawn_character(npc, 10, 10);

    assert_eq!(world.process_simple_baddy_noncombat_actions(1), 1);

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!((npc.tox, npc.toy), (10, 9));
    assert_eq!(npc.dir, Direction::Up as u8);
    assert_ne!(world.legacy_random_seed, 0);
}

#[test]
fn simple_baddy_scavenger_regenerates_before_random_wander() {
    let mut world = World::default();
    world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.rest_x = 10;
    npc.rest_y = 10;
    npc.hp = 9 * POWERSCALE;
    npc.mana = 10 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 10;
    npc.values[0][CharacterValue::Mana as usize] = 10;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        scavenger: 4,
        dir: 0,
        ..SimpleBaddyDriverData::default()
    }));
    world.spawn_character(npc, 10, 10);

    assert!(
        world.process_simple_baddy_noncombat_action_with_random(CharacterId(1), 1, |_| {
            panic!("regenerate_driver should run before RANDOM wander gates")
        })
    );

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::IDLE);
    assert_eq!(npc.duration, TICKS_PER_SECOND as i32);
}

#[test]
fn simple_baddy_scavenger_regenerates_before_drinkspecial_poison() {
    let mut world = World::default();
    world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.rest_x = 10;
    npc.rest_y = 10;
    npc.hp = 9 * POWERSCALE;
    npc.mana = 10 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 10;
    npc.values[0][CharacterValue::Mana as usize] = 10;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        scavenger: 4,
        drinkspecial: 1,
        ..SimpleBaddyDriverData::default()
    }));
    let mut poison0 = item(10, ItemFlags::empty());
    poison0.driver = IDR_POISON0;
    npc.inventory[SPELL_SLOT_START] = Some(poison0.id);
    world.items.insert(poison0.id, poison0);
    world.spawn_character(npc, 10, 10);

    assert!(
        world.process_simple_baddy_noncombat_action_with_random(CharacterId(1), 1, |_| {
            panic!("regenerate_driver should run before drinkspecial and RANDOM wander gates")
        })
    );

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::IDLE);
    assert_eq!(npc.inventory[SPELL_SLOT_START], Some(ItemId(10)));
    assert!(world.items.contains_key(&ItemId(10)));
}

#[test]
fn simple_baddy_noncombat_self_blesses_before_idle() {
    let mut world = World::default();
    world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = 10 * POWERSCALE;
    npc.values[0][CharacterValue::Bless as usize] = 20;
    npc.values[0][CharacterValue::MagicShield as usize] = 10;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    world.spawn_character(npc, 10, 10);

    assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::BLESS_SELF);
    assert_eq!(npc.act1, 1);
    assert_eq!(npc.mana, 8 * POWERSCALE);
}

#[test]
fn simple_baddy_noncombat_self_magicshields_when_bless_unavailable() {
    let mut world = World::default();
    world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = 10 * POWERSCALE;
    npc.values[0][CharacterValue::MagicShield as usize] = 8;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    world.spawn_character(npc, 10, 10);

    assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::MAGICSHIELD);
    assert_eq!(npc.act1, 8 * POWERSCALE);
    assert_eq!(npc.mana, 6 * POWERSCALE);
}

#[test]
fn simple_baddy_noncombat_regenerates_before_self_spells() {
    let mut world = World::default();
    world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = 10 * POWERSCALE;
    npc.hp = 9 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 10;
    npc.values[0][CharacterValue::Bless as usize] = 20;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    world.spawn_character(npc, 10, 10);

    assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::IDLE);
    assert_eq!(npc.duration, TICKS_PER_SECOND as i32);
    assert_eq!(npc.mana, 10 * POWERSCALE);
}

#[test]
fn simple_baddy_scavenger_clears_direction_when_walk_fails() {
    let mut world = World::default();
    world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.rest_x = 10;
    npc.rest_y = 10;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        scavenger: 4,
        dir: Direction::Right as i32,
        ..SimpleBaddyDriverData::default()
    }));
    world.spawn_character(npc, 10, 10);
    world
        .map
        .tile_mut(11, 10)
        .unwrap()
        .flags
        .insert(MapFlags::MOVEBLOCK);

    assert!(world.process_simple_baddy_noncombat_action_with_random(CharacterId(1), 1, |_| 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::IDLE);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.dir, 0);
}

#[test]
fn palace_cap_timer_deactivates_when_regeneration_resets() {
    let mut world = World::default();
    world.tick = Tick(60);
    world.add_character(character(0));
    let mut wearer = character(1);
    wearer.inventory[worn_slot::HEAD] = Some(ItemId(7));
    wearer.regen_ticker = 50;
    world.spawn_character(wearer, 10, 10);
    let mut cap = item(7, ItemFlags::USED);
    cap.driver = IDR_PALACECAP;
    cap.carried_by = Some(CharacterId(1));
    cap.driver_data = vec![1];
    cap.sprite = 12_346;
    world.items.insert(ItemId(7), cap);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_PALACECAP,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        11,
    );

    assert!(matches!(outcome, ItemDriverOutcome::PalaceCapTimer { .. }));
    let cap = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(cap.driver_data[0], 0);
    assert_eq!(cap.sprite, 12_345);
    assert!(world.characters[&CharacterId(1)]
        .flags
        .contains(CharacterFlags::ITEMS));
    assert!(world
        .effects
        .values()
        .all(|effect| effect.effect_type != EF_CAP));
}
