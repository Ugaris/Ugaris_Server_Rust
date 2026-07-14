// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use super::*;

#[test]
fn simple_baddy_attack_action_self_heals_before_offense_when_badly_hurt() {
    let mut world = World::default();
    world.tick = Tick(450);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.regen_ticker = 450;
    npc.hp = 40 * POWERSCALE;
    npc.mana = 10 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 100;
    npc.values[0][CharacterValue::Mana as usize] = 10;
    npc.values[0][CharacterValue::Heal as usize] = 20;
    npc.values[0][CharacterValue::Fireball as usize] = 20;
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
    assert_eq!(npc.action, action::HEAL_SELF);
    assert!(npc.mana < 10 * POWERSCALE);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 450);
}

#[test]
fn simple_baddy_visible_attack_queues_legacy_start_combat_sound_after_delay() {
    let mut world = World::default();
    world.tick = Tick(TICKS_PER_SECOND * 11);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        lastfight: 0,
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
    target.flags.insert(CharacterFlags::PLAYER);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let sounds = world.drain_pending_sound_specials();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].character_id, CharacterId(2));
    assert_eq!(sounds[0].special.special_type, 1);

    let mut world = World::default();
    world.tick = Tick(TICKS_PER_SECOND * 11);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        lastfight: (TICKS_PER_SECOND * 11 - 1) as i32,
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
    target.flags.insert(CharacterFlags::PLAYER);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    assert!(world.drain_pending_sound_specials().is_empty());
}

#[test]
fn simple_baddy_attack_action_restores_magicshield_before_melee() {
    let mut world = World::default();
    world.tick = Tick(451);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = 10 * POWERSCALE;
    npc.lifeshield = 0;
    npc.values[0][CharacterValue::MagicShield as usize] = 20;
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
    assert_eq!(npc.action, action::MAGICSHIELD);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 451);
}

#[test]
fn simple_baddy_attack_action_self_blesses_when_unblessed() {
    let mut world = World::default();
    world.tick = Tick(452);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = BLESS_COST;
    npc.values[0][CharacterValue::Bless as usize] = 20;
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
    assert_eq!(npc.action, action::BLESS_SELF);
    assert_eq!(npc.mana, 0);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 452);
}

#[test]
fn simple_baddy_attack_action_earth_demon_casts_useful_earthmud() {
    let mut world = World::default();
    world.tick = Tick(454);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.flags.insert(CharacterFlags::EDEMON);
    npc.hp = 100 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 100;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.values[1][CharacterValue::Demon as usize] = 30;
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
    let mut target = character(2);
    target.action = action::WALK;
    target.tox = 16;
    target.toy = 10;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 15, 10);
    world.map.tile_mut(15, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::EARTHMUD);
    assert_eq!(npc.act1, 17 + 10 * MAX_MAP as i32);
    assert_eq!(npc.act2, 30);
    assert_eq!(npc.hp, 100 * POWERSCALE - 3000);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 454);
}

#[test]
fn simple_baddy_attack_action_skips_earthmud_without_useful_tiles() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.flags.insert(CharacterFlags::EDEMON);
    npc.hp = 100 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 100;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.values[1][CharacterValue::Demon as usize] = 30;
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
    for (x, y) in [(15, 10), (16, 10), (14, 10), (15, 11), (15, 9)] {
        world.map.set_flags(x, y, MapFlags::SIGHTBLOCK);
    }

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_ne!(npc.action, action::EARTHMUD);
}

#[test]
fn simple_baddy_fight_tasks_keep_c_commented_earthrain_disabled() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.flags.insert(CharacterFlags::EDEMON);
    npc.hp = 100 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 100;
    npc.values[1][CharacterValue::Demon as usize] = 30;
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

    let tasks = world.simple_baddy_fight_tasks(
        CharacterId(1),
        world.characters.get(&CharacterId(2)).unwrap(),
        1,
        false,
    );

    assert!(!tasks
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::EarthRain));
    assert!(tasks
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::EarthMud));
}

#[test]
fn simple_baddy_fight_tasks_add_c_low_hp_flee_branch() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.hp = POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 10;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);

    let tasks = world.simple_baddy_fight_tasks(
        CharacterId(1),
        world.characters.get(&CharacterId(2)).unwrap(),
        1,
        false,
    );

    assert!(tasks
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Flee));
}

#[test]
fn simple_baddy_attack_action_can_choose_low_hp_flee() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.hp = POWERSCALE;
    npc.endurance = 10 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 10;
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

    assert!(world.process_simple_baddy_attack_action_with_random(CharacterId(1), 1, |_| 0));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!(npc.speed_mode, SpeedMode::Fast);
    assert_ne!(npc.dir, Direction::Right as u8);
}

#[test]
fn simple_baddy_firering_helper_respects_active_spell_blocker() {
    let mut world = World::default();
    world.tick = Tick(456);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FIREBALL_COST;
    npc.values[0][CharacterValue::Fireball as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let mut blocker = item(20, ItemFlags::empty());
    blocker.driver = IDR_FIRERING;
    npc.inventory[SPELL_SLOT_START] = Some(blocker.id);
    let target = character(2);
    world.items.insert(blocker.id, blocker);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target.clone(), 11, 10);

    assert!(!world.setup_simple_baddy_firering_attack(CharacterId(1), &target));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, 0);
    assert_eq!(npc.mana, FIREBALL_COST);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 0);
}

#[test]
fn simple_baddy_fireball_repositions_for_blocked_line_of_fire() {
    let mut world = World::default();
    world.tick = Tick(467);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FIREBALL_COST;
    npc.values[0][CharacterValue::Fireball as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
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
    world
        .map
        .tile_mut(12, 10)
        .unwrap()
        .flags
        .insert(MapFlags::MOVEBLOCK);
    world.map.tile_mut(14, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!(npc.tox, 10);
    assert_eq!(npc.toy, 11);
    assert_eq!(npc.mana, FIREBALL_COST);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 467);
}

#[test]
fn simple_baddy_fireball_does_not_cast_through_blocked_line_without_lane() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FIREBALL_COST;
    npc.values[0][CharacterValue::Fireball as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
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
    for (x, y) in [(12, 10), (10, 9), (10, 11), (11, 10), (9, 10)] {
        world
            .map
            .tile_mut(x, y)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);
    }
    world.map.tile_mut(14, 10).unwrap().light = 255;

    let target = world.characters[&CharacterId(2)].clone();
    assert!(!world.setup_simple_baddy_fireball_attack(CharacterId(1), &target, 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, 0);
    assert_eq!(npc.mana, FIREBALL_COST);
}

#[test]
fn simple_baddy_fireball_line_rejects_friendly_blast() {
    let mut world = World::default();
    let mut npc = character(1);
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
    world.spawn_character(npc, 10, 10);
    world.spawn_character(character(2), 15, 10);
    world.spawn_character(character(3), 12, 11);
    world
        .map
        .tile_mut(12, 10)
        .unwrap()
        .flags
        .insert(MapFlags::MOVEBLOCK);

    assert!(!world.fireball_line_hits_target(CharacterId(1), CharacterId(2), 10, 10, 15, 10));
}

#[test]
fn simple_baddy_attack_action_applies_legacy_task_silliness_rolls() {
    let mut world = World::default();
    world.tick = Tick(459);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FLASH_COST;
    npc.values[0][CharacterValue::Attack as usize] = 100;
    npc.values[1][CharacterValue::Attack as usize] = 100;
    npc.values[0][CharacterValue::Flash as usize] = 26;
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
    world.map.tile_mut(11, 10).unwrap().light = 255;
    let mut rolls = [0, 4].into_iter();

    assert!(
        world.process_simple_baddy_attack_action_with_random(CharacterId(1), 1, |below| {
            assert_eq!(below, 5);
            rolls.next().unwrap_or(0)
        })
    );

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::ATTACK1);
    assert_eq!(npc.mana, FLASH_COST);
}

#[test]
fn simple_baddy_attack_task_uses_c_attack_skill_with_weapon_skill() {
    let mut character = character(1);
    character.level = 20;
    character.values[0][CharacterValue::Attack as usize] = 30;
    character.values[1][CharacterValue::Attack as usize] = 30;
    character.values[0][CharacterValue::Tactics as usize] = 12;
    character.values[0][CharacterValue::Hand as usize] = 5;
    character.values[0][CharacterValue::Sword as usize] = 40;
    character.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(7));
    let weapon = item(7, ItemFlags::SWORD);
    let items = HashMap::from([(weapon.id, weapon)]);

    assert_eq!(simple_baddy_attack_skill(&character, &items), 104);
    assert_eq!(simple_baddy_attack_task_value(&character, &items), 539);
}

#[test]
fn simple_baddy_attack_task_falls_back_to_hand_without_weapon() {
    let mut character = character(1);
    character.level = 20;
    character.values[0][CharacterValue::Hand as usize] = 9;
    character.values[0][CharacterValue::Bless as usize] = 8;
    character.values[0][CharacterValue::Heal as usize] = 8;
    character.values[0][CharacterValue::Freeze as usize] = 8;
    character.values[0][CharacterValue::MagicShield as usize] = 8;
    character.values[0][CharacterValue::Flash as usize] = 8;
    character.values[0][CharacterValue::Fireball as usize] = 8;
    character.values[0][CharacterValue::Pulse as usize] = 8;

    let items = HashMap::new();

    assert_eq!(simple_baddy_attack_skill(&character, &items), 3);
    assert_eq!(simple_baddy_attack_task_value(&character, &items), 2);
}

#[test]
fn simple_baddy_attack_action_uses_warcry_when_close_and_unshielded() {
    let mut world = World::default();
    world.tick = Tick(460);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.endurance = 10 * POWERSCALE;
    npc.values[0][CharacterValue::Warcry as usize] = 20;
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
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 12, 10);
    world.map.tile_mut(12, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WARCRY);
    assert_eq!(npc.endurance, 10 * POWERSCALE - 20 * POWERSCALE / 3);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 460);
}

#[test]
fn simple_baddy_warcry_task_does_not_precheck_modifier_like_c() {
    let mut world = World::default();
    world.tick = Tick(460);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.endurance = 10 * POWERSCALE;
    npc.lifeshield = 10 * POWERSCALE;
    npc.values[0][CharacterValue::Warcry as usize] = 2;
    npc.values[0][CharacterValue::MagicShield as usize] = 10;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.values[1][CharacterValue::MagicShield as usize] = 10;
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
    target.values[0][CharacterValue::Immunity as usize] = 100;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 12, 10);
    world.map.tile_mut(12, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WARCRY);
    assert_eq!(npc.endurance, 10 * POWERSCALE - 2 * POWERSCALE / 3);
}

#[test]
fn simple_baddy_warcry_task_requires_more_than_exact_endurance_cost_like_c() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Warcry as usize] = 9;
    npc.values[0][CharacterValue::MagicShield as usize] = 10;
    npc.values[1][CharacterValue::MagicShield as usize] = 10;
    npc.lifeshield = 0;
    npc.endurance = 9 * POWERSCALE / 3;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 12, 10);

    let exact_cost_tasks = world.simple_baddy_fight_tasks(
        CharacterId(1),
        world.characters.get(&CharacterId(2)).unwrap(),
        1,
        false,
    );

    assert!(!exact_cost_tasks
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Warcry));

    world.characters.get_mut(&CharacterId(1)).unwrap().endurance += 1;
    let above_cost_tasks = world.simple_baddy_fight_tasks(
        CharacterId(1),
        world.characters.get(&CharacterId(2)).unwrap(),
        1,
        false,
    );

    assert!(above_cost_tasks
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Warcry));
}

#[test]
fn simple_baddy_ball_task_requires_unblocked_legacy_intercept_steps() {
    let mut world = World::default();
    world.tick = Tick(461);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FLASH_COST;
    npc.values[0][CharacterValue::Flash as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 16,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 16, 10);
    world.map.tile_mut(16, 10).unwrap().light = 255;
    world
        .map
        .tile_mut(12, 10)
        .unwrap()
        .flags
        .insert(MapFlags::MOVEBLOCK);

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_ne!(npc.action, action::BALL1);
}

#[test]
fn simple_baddy_ball_attack_uses_legacy_random_target_offset() {
    let mut world = World::default();
    world.tick = Tick(461);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FLASH_COST;
    npc.values[0][CharacterValue::Flash as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 16,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 16, 10);
    world.map.tile_mut(16, 10).unwrap().light = 255;
    let mut rolls = [0, 0, 0, 2].into_iter();

    assert!(
        world.process_simple_baddy_attack_action_with_random(CharacterId(1), 1, |_| {
            rolls.next().unwrap()
        })
    );

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::BALL1);
    assert_eq!(npc.act1, 15);
    assert_eq!(npc.act2, 11);
}

#[test]
fn simple_baddy_attack_batch_threads_runtime_random() {
    let mut world = World::default();
    world.tick = Tick(461);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FLASH_COST;
    npc.values[0][CharacterValue::Flash as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 16,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 16, 10);
    world.map.tile_mut(16, 10).unwrap().light = 255;
    let mut rolls = [0, 0, 0, 2].into_iter();

    assert_eq!(
        world.process_simple_baddy_attack_actions_with_random(1, |_| rolls.next().unwrap()),
        1
    );

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::BALL1);
    assert_eq!(npc.act1, 15);
    assert_eq!(npc.act2, 11);
}

#[test]
fn simple_baddy_default_attack_action_consumes_world_rng_seed() {
    let mut world = World::default();
    world.tick = Tick(461);
    world.legacy_random_seed = 7;
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FLASH_COST;
    npc.values[0][CharacterValue::Flash as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 16,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 16, 10);
    world.map.tile_mut(16, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    assert_ne!(world.legacy_random_seed, 7);
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::BALL1);
}
