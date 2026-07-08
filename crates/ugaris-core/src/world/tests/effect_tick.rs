use super::*;

#[test]
fn world_create_explosion_effect_matches_legacy_shape_and_expires() {
    let mut world = World::default();

    let effect_id = world.create_explosion_effect(10, 10, 8, 50450);

    let effect = world.effects.get(&effect_id).unwrap();
    assert_eq!(effect.effect_type, EF_EXPLODE);
    assert_eq!(effect.strength, 8);
    assert_eq!(effect.light, 200);
    assert_eq!(effect.base_sprite, 50450);
    assert_eq!(effect.stop_tick, 8);
    assert_eq!(world.map.tile(10, 10).unwrap().effects[0], effect_id as u16);
    assert_eq!(world.map.tile(10, 10).unwrap().light, 200);

    for _ in 0..8 {
        world.advance();
    }
    world.tick_effects();

    assert_ne!(
        world
            .effects
            .get(&effect_id)
            .map(|effect| effect.effect_type),
        Some(EF_FIREBALL)
    );
    assert_eq!(world.map.tile(10, 10).unwrap().effects[0], 0);
    assert_eq!(world.map.tile(10, 10).unwrap().light, 0);
}

#[test]
fn world_create_earthrain_places_3x3_except_sight_blocked_tiles() {
    let mut world = World::default();
    world.map.set_flags(11, 10, MapFlags::SIGHTBLOCK);
    world.map.set_flags(9, 9, MapFlags::TSIGHTBLOCK);

    let effect_id = world.create_earthrain_effect(10, 10, 7);

    let effect = world.effects.get(&effect_id).unwrap();
    assert_eq!(effect.effect_type, EF_EARTHRAIN);
    assert_eq!(effect.light, 10);
    assert_eq!(effect.strength, 7);
    assert_eq!(effect.stop_tick, TICKS_PER_SECOND as i32 * 60);
    assert_eq!(effect.fields.len(), 7);
    assert_eq!(world.map.tile(10, 10).unwrap().effects[0], effect_id as u16);
    assert_eq!(world.map.tile(11, 10).unwrap().effects[0], 0);
    assert_eq!(world.map.tile(9, 9).unwrap().effects[0], 0);
    assert!(world.map.tile(10, 10).unwrap().light >= 10);
}

#[test]
fn earthrain_tick_damages_players_using_legacy_demon_reduction() {
    let mut world = World::default();
    let effect_id = world.create_earthrain_effect(10, 10, 7);
    let mut target = character(1);
    target.flags |= CharacterFlags::PLAYER;
    target.hp = 10_000;
    target.values[0][CharacterValue::Demon as usize] = 2;
    assert!(world.spawn_character(target, 10, 10));

    world.tick_effects_with_random(|_| 0);

    let target = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(target.hp, 10_000 - (7 - 2) * 150);
    assert!(target.flags.contains(CharacterFlags::UPDATE));
    assert!(world.effects.contains_key(&effect_id));
}

#[test]
fn earthrain_tick_skips_non_players_roll_misses_and_full_demon_reduction() {
    let mut world = World::default();
    world.create_earthrain_effect(10, 10, 4);
    let mut non_player = character(1);
    non_player.hp = 10_000;
    assert!(world.spawn_character(non_player, 10, 10));
    let mut demon_player = character(2);
    demon_player.flags |= CharacterFlags::PLAYER;
    demon_player.hp = 10_000;
    demon_player.values[0][CharacterValue::Demon as usize] = 4;
    assert!(world.spawn_character(demon_player, 11, 10));
    let mut missed_player = character(3);
    missed_player.flags |= CharacterFlags::PLAYER;
    missed_player.hp = 10_000;
    assert!(world.spawn_character(missed_player, 10, 11));

    world.tick_effects_with_random(|_| 1);

    assert_eq!(world.characters.get(&CharacterId(1)).unwrap().hp, 10_000);
    assert_eq!(world.characters.get(&CharacterId(2)).unwrap().hp, 10_000);
    assert_eq!(world.characters.get(&CharacterId(3)).unwrap().hp, 10_000);
}

#[test]
fn world_create_earthmud_avoids_duplicate_effect_type_on_tile() {
    let mut world = World::default();

    let first_id = world.create_earthmud_effect(10, 10, 4);
    let second_id = world.create_earthmud_effect(11, 10, 9);

    assert_eq!(world.map.tile(10, 10).unwrap().effects[0], first_id as u16);
    assert!(world.map.tile(10, 10).unwrap().effects[1..]
        .iter()
        .all(|&slot| slot != second_id as u16));
    assert_eq!(world.effects[&first_id].fields.len(), 9);
    assert!(world.effects[&second_id].fields.len() < 9);
}

#[test]
fn world_balltrap_creates_retained_ball_effect() {
    let mut world = World::default();
    let mut trigger = character(1);
    trigger.flags.remove(CharacterFlags::PLAYER);
    world.add_character(trigger);
    let mut trap = item(7, ItemFlags::USED | ItemFlags::USE);
    trap.driver = IDR_BALLTRAP;
    trap.x = 10;
    trap.y = 20;
    trap.driver_data = vec![130, 125, 42];
    world.add_item(trap);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_BALLTRAP,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::BallTrapProjectile {
            start_x: 11,
            start_y: 19,
            target_x: 12,
            target_y: 17,
            power: 42,
            ..
        }
    ));
    assert_eq!(world.effects.len(), 1);
    let effect = world.effects.values().next().unwrap();
    assert_eq!(effect.effect_type, EF_BALL);
    assert_eq!(effect.strength, 42);
    assert_eq!(effect.light, 80);
    assert_eq!((effect.from_x, effect.from_y), (11, 19));
    assert_eq!((effect.to_x, effect.to_y), (12, 17));
    assert_eq!((effect.x, effect.y), (11 * 1024 + 512, 19 * 1024 + 512));
    assert_eq!(effect.caster, None);
    assert_eq!(effect.stop_tick, (TICKS_PER_SECOND * 5) as i32);
}

#[test]
fn world_fireball_machine_timer_creates_retained_projectile_and_reschedules() {
    let mut world = World::default();
    let mut machine = item(7, ItemFlags::USED | ItemFlags::USE);
    machine.driver = IDR_FIREBALL;
    machine.x = 10;
    machine.y = 20;
    machine.driver_data = vec![130, 125, 42, 9];
    world.add_item(machine);
    let mut nearby = character(1);
    nearby.x = 20;
    nearby.y = 20;
    world.add_character(nearby);
    let mut far = character(2);
    far.x = 80;
    far.y = 20;
    world.add_character(far);
    assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));

    world.advance();
    let outcomes = world.process_due_timers(1);

    assert_eq!(outcomes.len(), 1);
    assert_eq!(
        outcomes[0],
        ItemDriverOutcome::FireballMachineProjectile {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            start_x: 11,
            start_y: 19,
            target_x: 12,
            target_y: 17,
            power: 42,
            schedule_after_ticks: Some(9),
        }
    );
    assert_eq!(world.effects.len(), 1);
    let effect = world.effects.values().next().unwrap();
    assert_eq!(effect.effect_type, EF_FIREBALL);
    assert_eq!(effect.strength, 42);
    assert_eq!(effect.light, 200);
    assert_eq!((effect.from_x, effect.from_y), (11, 19));
    assert_eq!((effect.to_x, effect.to_y), (12, 17));
    assert_eq!((effect.x, effect.y), (11 * 1024 + 512, 19 * 1024 + 512));
    assert_eq!(effect.caster, None);
    assert_eq!(effect.stop_tick, 1 + TICKS_PER_SECOND as i32);
    assert_eq!(
        world.characters[&CharacterId(1)].driver_messages,
        vec![crate::character_driver::CharacterDriverMessage {
            message_type: NT_SPELL,
            dat1: 0,
            dat2: V_FIREBALL,
            dat3: effect.serial,
            text: None,
        }]
    );
    assert!(world.characters[&CharacterId(2)].driver_messages.is_empty());

    for _ in 0..8 {
        world.advance();
    }
    assert!(world.process_due_timers(1).is_empty());
    world.advance();
    assert_eq!(world.process_due_timers(1).len(), 1);
}

#[test]
fn world_edemonball_timer_creates_retained_projectile_effect() {
    let mut world = World::default();
    let mut cannon = item(7, ItemFlags::USED | ItemFlags::USE);
    cannon.driver = IDR_EDEMONBALL;
    cannon.x = 10;
    cannon.y = 20;
    cannon.driver_data = vec![1, 2, 42, 0];
    world.add_item(cannon);
    assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));

    world.advance();
    let outcomes = world.process_due_timers(6);

    assert_eq!(outcomes.len(), 1);
    assert_eq!(
        &outcomes[0],
        &ItemDriverOutcome::EdemonBallProjectile {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            start_x: 10,
            start_y: 21,
            target_x: 10,
            target_y: 30,
            strength: 42,
            base_sprite: 2,
            schedule_after_ticks: TICKS_PER_SECOND * 16,
        }
    );
    let cannon = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(cannon.driver_data[3], 1);
    assert_eq!(world.effects.len(), 1);
    let effect = world.effects.values().next().unwrap();
    assert_eq!(effect.effect_type, EF_EDEMONBALL);
    assert_eq!(effect.strength, 42);
    assert_eq!(effect.base_sprite, 2);
    assert_eq!((effect.from_x, effect.from_y), (10, 21));
    assert_eq!((effect.to_x, effect.to_y), (10, 30));
    assert_eq!((effect.x, effect.y), (10 * 1024 + 512, 21 * 1024 + 512));
    assert_eq!(effect.stop_tick, 1 + (TICKS_PER_SECOND * 4) as i32);
}

#[test]
fn world_edemonball_timer_waits_when_area_fire_switch_is_disabled() {
    let mut world = World::default();
    let mut cannon = item(7, ItemFlags::USED | ItemFlags::USE);
    cannon.driver = IDR_EDEMONBALL;
    cannon.sprite = 14159;
    cannon.x = 10;
    cannon.y = 20;
    cannon.driver_data = vec![0, 2, 42, 0];
    world.add_item(cannon);

    let mut switch = item(8, ItemFlags::USED | ItemFlags::USE);
    switch.driver = IDR_EDEMONSWITCH;
    switch.driver_data = vec![0, 0, 0, 0, 0];
    world.items.insert(ItemId(8), switch);
    assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));

    world.advance();
    let outcomes = world.process_due_timers(6);

    assert_eq!(
        outcomes,
        vec![ItemDriverOutcome::EdemonBallInactive {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            schedule_after_ticks: TICKS_PER_SECOND,
        }]
    );
    assert!(world.effects.is_empty());
    assert_eq!(world.items[&ItemId(7)].sprite, 14160);
    assert_eq!(world.items[&ItemId(7)].driver_data[3], 0);
    assert_eq!(world.timers.used_timers(), 1);
}

#[test]
fn caligar_gun_timer_creates_fixed_edemonball_projectiles() {
    let mut world = World::default();
    let mut gun = item(7, ItemFlags::USED | ItemFlags::USE);
    gun.driver = IDR_CALIGAR;
    gun.x = 10;
    gun.y = 20;
    gun.driver_data = vec![9];
    world.add_item(gun);
    assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));

    world.advance();
    let outcomes = world.process_due_timers(36);

    assert_eq!(
        outcomes,
        vec![ItemDriverOutcome::CaligarGunProjectile {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            direction: 5,
            schedule_after_ticks: 12,
        }]
    );
    assert_eq!(world.effects.len(), 4);
    let mut shots: Vec<_> = world
        .effects
        .values()
        .map(|effect| {
            (
                effect.effect_type,
                effect.from_x,
                effect.from_y,
                effect.to_x,
                effect.to_y,
                effect.strength,
                effect.base_sprite,
            )
        })
        .collect();
    shots.sort();
    assert_eq!(
        shots,
        vec![
            (EF_EDEMONBALL, 9, 20, 0, 20, 50, 1),
            (EF_EDEMONBALL, 10, 19, 10, 10, 50, 1),
            (EF_EDEMONBALL, 10, 21, 10, 30, 50, 1),
            (EF_EDEMONBALL, 11, 20, 20, 20, 50, 1),
        ]
    );
    for _ in 0..12 {
        world.advance();
    }
    assert_eq!(world.process_due_timers(36).len(), 1);
}

#[test]
fn edemonball_timer_aims_at_nearby_character_before_fallback_rotation() {
    let mut world = World::default();
    let mut cannon = item(7, ItemFlags::USED | ItemFlags::USE);
    cannon.driver = IDR_EDEMONBALL;
    cannon.x = 10;
    cannon.y = 20;
    cannon.driver_data = vec![1, 2, 42, 0];
    world.add_item(cannon);
    assert!(world.spawn_character(character(1), 10, 25));
    assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));

    world.advance();
    let outcomes = world.process_due_timers(6);

    assert_eq!(outcomes.len(), 1);
    assert_eq!(
        &outcomes[0],
        &ItemDriverOutcome::EdemonBallProjectile {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            start_x: 10,
            start_y: 21,
            target_x: 10,
            target_y: 25,
            strength: 42,
            base_sprite: 2,
            schedule_after_ticks: TICKS_PER_SECOND * 8,
        }
    );
    let cannon = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(cannon.driver_data[3], 0);
    let effect = world.effects.values().next().unwrap();
    assert_eq!((effect.from_x, effect.from_y), (10, 21));
    assert_eq!((effect.to_x, effect.to_y), (10, 25));
}

#[test]
fn edemonball_timer_predicts_walking_character_target_tile() {
    let mut world = World::default();
    let mut cannon = item(7, ItemFlags::USED | ItemFlags::USE);
    cannon.driver = IDR_EDEMONBALL;
    cannon.x = 10;
    cannon.y = 20;
    cannon.driver_data = vec![1, 2, 42, 0];
    world.add_item(cannon);
    let mut target = character(1);
    target.action = action::WALK;
    target.dir = Direction::Down as u8;
    target.duration = 10;
    target.step = 5;
    target.tox = 10;
    target.toy = 26;
    assert!(world.spawn_character(target, 10, 25));
    assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));

    world.advance();
    let outcomes = world.process_due_timers(6);

    assert!(matches!(
        outcomes.first(),
        Some(ItemDriverOutcome::EdemonBallProjectile {
            start_x: 10,
            start_y: 21,
            target_x: 10,
            target_y: 26,
            schedule_after_ticks,
            ..
        }) if *schedule_after_ticks == TICKS_PER_SECOND * 8
    ));
    let effect = world.effects.values().next().unwrap();
    assert_eq!((effect.to_x, effect.to_y), (10, 26));
}

#[test]
fn edemonball_effect_moves_by_legacy_quarter_tile_steps() {
    let mut world = World::default();
    let effect_id = world.create_edemonball_effect(10, 10, 10, 20, 7, 1);

    world.tick_effects();

    let effect = world.effects.get(&effect_id).unwrap();
    assert_eq!(effect.effect_type, EF_EDEMONBALL);
    assert_eq!((effect.x, effect.y), (10 * 1024 + 512, 10 * 1024 + 768));
    assert_eq!((effect.last_x, effect.last_y), (10, 10));
    assert!(world
        .map
        .tile(10, 10)
        .unwrap()
        .effects
        .contains(&(effect_id as u16)));
}

#[test]
fn edemonball_effect_explodes_on_character_and_applies_direct_damage() {
    let mut world = World::default();
    let mut target = character(1);
    target.hp = 10_000;
    assert!(world.spawn_character(target, 10, 12));
    let _effect_id = world.create_edemonball_effect(10, 10, 10, 20, 3, 0);

    for _ in 0..6 {
        world.tick_effects();
    }

    assert!(!world
        .effects
        .values()
        .any(|effect| effect.effect_type == EF_EDEMONBALL));
    let target = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(target.hp, 7_000);
    assert!(target.flags.contains(CharacterFlags::UPDATE));
    assert!(world.effects.values().any(|effect| {
        effect.effect_type == EF_EXPLODE
            && effect.base_sprite == 50450
            && effect
                .fields
                .iter()
                .any(|&field| field == world.map.legacy_index(10, 12).unwrap() as i32)
    }));
}

#[test]
fn edemonball_impact_uses_legacy_hurt_reduction() {
    let mut world = World::default();
    let mut target = character(1);
    target.hp = 10_000;
    target.lifeshield = POWERSCALE;
    target.values[0][CharacterValue::Armor as usize] = 60;
    assert!(world.spawn_character(target, 10, 12));
    let _effect_id = world.create_edemonball_effect(10, 10, 10, 20, 3, 1);

    for _ in 0..6 {
        world.tick_effects();
    }

    let target = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(target.hp, 8_500);
    assert_eq!(target.lifeshield, 0);
    assert_eq!(target.driver_messages[0].message_type, NT_GOTHIT);
    assert_eq!(target.driver_messages[0].dat1, 0);
    assert_eq!(target.driver_messages[0].dat2, 1_500);
}

#[test]
fn edemonball_green_base_is_absorbed_by_green_crystal() {
    let mut world = World::default();
    let mut target = character(1);
    target.hp = 10_000;
    target.inventory[30] = Some(ItemId(77));
    assert!(world.spawn_character(target, 10, 12));
    let mut crystal = item(77, ItemFlags::USED);
    crystal.carried_by = Some(CharacterId(1));
    crystal.template_id = IID_AREA6_GREENCRYSTAL;
    crystal.driver_data = vec![100];
    crystal.sprite = 50318;
    world.items.insert(ItemId(77), crystal);
    let _effect_id = world.create_edemonball_effect(10, 10, 10, 20, 30, 0);

    for _ in 0..6 {
        world.tick_effects();
    }

    let target = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(target.hp, 10_000);
    assert!(target.flags.contains(CharacterFlags::ITEMS));
    let crystal = world.items.get(&ItemId(77)).unwrap();
    assert_eq!(crystal.driver_data[0], 70);
    assert_eq!(crystal.sprite, 50322);
}

#[test]
fn edemonball_green_crystals_are_destroyed_until_damage_remaining() {
    let mut world = World::default();
    let mut target = character(1);
    target.hp = 10_000;
    target.cursor_item = Some(ItemId(77));
    target.inventory[30] = Some(ItemId(78));
    assert!(world.spawn_character(target, 10, 12));
    for (id, power) in [(77, 20), (78, 40)] {
        let mut crystal = item(id, ItemFlags::USED);
        crystal.carried_by = Some(CharacterId(1));
        crystal.template_id = IID_AREA6_GREENCRYSTAL;
        crystal.driver_data = vec![power];
        world.items.insert(ItemId(id), crystal);
    }
    let _effect_id = world.create_edemonball_effect(10, 10, 10, 20, 70, 0);

    for _ in 0..6 {
        world.tick_effects();
    }

    let target = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(target.hp, 0);
    assert_eq!(target.cursor_item, None);
    assert_eq!(target.inventory[30], None);
    assert!(!world.items.contains_key(&ItemId(77)));
    assert!(!world.items.contains_key(&ItemId(78)));
}

#[test]
fn edemonball_non_green_base_ignores_green_crystal() {
    let mut world = World::default();
    let mut target = character(1);
    target.hp = 10_000;
    target.inventory[30] = Some(ItemId(77));
    assert!(world.spawn_character(target, 10, 12));
    let mut crystal = item(77, ItemFlags::USED);
    crystal.carried_by = Some(CharacterId(1));
    crystal.template_id = IID_AREA6_GREENCRYSTAL;
    crystal.driver_data = vec![100];
    world.items.insert(ItemId(77), crystal);
    let _effect_id = world.create_edemonball_effect(10, 10, 10, 20, 3, 1);

    for _ in 0..6 {
        world.tick_effects();
    }

    let target = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(target.hp, 7_000);
    assert_eq!(world.items.get(&ItemId(77)).unwrap().driver_data[0], 100);
}

#[test]
fn edemonball_effect_explodes_on_wall_at_previous_tile() {
    let mut world = World::default();
    world
        .map
        .tile_mut(10, 11)
        .unwrap()
        .flags
        .insert(MapFlags::MOVEBLOCK);
    let _effect_id = world.create_edemonball_effect(10, 10, 10, 20, 3, 2);

    world.tick_effects();
    world.tick_effects();

    assert!(!world
        .effects
        .values()
        .any(|effect| effect.effect_type == EF_EDEMONBALL));
    assert!(world.effects.values().any(|effect| {
        effect.effect_type == EF_EXPLODE
            && effect.base_sprite == 50452
            && effect
                .fields
                .iter()
                .any(|&field| field == world.map.legacy_index(10, 10).unwrap() as i32)
    }));
}

#[test]
fn world_flamethrower_timer_burns_forward_characters_and_reschedules() {
    let mut world = World::default();
    let mut trap = item(7, ItemFlags::USED | ItemFlags::USE);
    trap.driver = IDR_FLAMETHROW;
    trap.x = 10;
    trap.y = 10;
    trap.driver_data = vec![1, 3, 0, 0];
    let mut first = character(1);
    first.x = 10;
    first.y = 11;
    let mut second = character(2);
    second.x = 10;
    second.y = 12;
    world.add_item(trap);
    world.add_character(first);
    world.add_character(second);
    world.map.tile_mut(10, 10).unwrap().item = 7;
    world.map.tile_mut(10, 11).unwrap().character = 1;
    world.map.tile_mut(10, 12).unwrap().character = 2;
    assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));

    world.advance();
    let outcomes = world.process_due_timers(1);

    assert_eq!(outcomes.len(), 1);
    assert!(matches!(
        outcomes[0],
        ItemDriverOutcome::FlameThrowerPulse { .. }
    ));
    assert_eq!(world.items.get(&ItemId(7)).unwrap().driver_data[0], 0);
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .flags
        .contains(CharacterFlags::UPDATE));
    assert!(world
        .characters
        .get(&CharacterId(2))
        .unwrap()
        .flags
        .contains(CharacterFlags::UPDATE));
    assert_eq!(world.effects.len(), 2);
    assert!(world.effects.values().any(|effect| {
        effect.effect_type == EF_BURN && effect.target_character == Some(CharacterId(1))
    }));
    assert!(world.effects.values().any(|effect| {
        effect.effect_type == EF_BURN && effect.target_character == Some(CharacterId(2))
    }));
    assert_eq!(world.timers.used_timers(), 1);
}

#[test]
fn burn_character_suppresses_duplicates_and_expires() {
    let mut world = World::default();
    let mut character = character(1);
    character.hp = 50 * POWERSCALE;
    world.add_character(character);

    assert!(world.burn_character(CharacterId(1)));
    assert!(!world.burn_character(CharacterId(1)));
    assert_eq!(world.effects.len(), 1);
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().hp,
        30 * POWERSCALE
    );

    world.tick = Tick(TICKS_PER_SECOND * 60);
    world.tick_effects();

    assert!(world.effects.is_empty());
}

#[test]
fn burn_character_damage_uses_legacy_hurt_reduction() {
    let mut world = World::default();
    let mut character = character(1);
    character.hp = 50 * POWERSCALE;
    character.lifeshield = 5 * POWERSCALE;
    character.values[0][CharacterValue::Armor as usize] = 100;
    world.add_character(character);

    assert!(world.burn_character(CharacterId(1)));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.hp, 40 * POWERSCALE);
    assert_eq!(character.lifeshield, 0);
    assert_eq!(character.driver_messages[0].message_type, NT_GOTHIT);
    assert_eq!(character.driver_messages[0].dat2, 10 * POWERSCALE);
}

#[test]
fn burn_effect_tick_applies_recurring_legacy_hurt_damage() {
    let mut world = World::default();
    let mut character = character(1);
    character.hp = 50 * POWERSCALE;
    character.lifeshield = POWERSCALE;
    world.add_character(character);

    assert!(world.burn_character(CharacterId(1)));
    let hp_after_initial_burn = world.characters[&CharacterId(1)].hp;
    let shield_after_initial_burn = world.characters[&CharacterId(1)].lifeshield;

    world.tick_effects();

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.hp, hp_after_initial_burn - 167);
    assert_eq!(character.lifeshield, shield_after_initial_burn);
    assert_eq!(
        character.driver_messages.last().unwrap().message_type,
        NT_GOTHIT
    );
    assert_eq!(character.driver_messages.last().unwrap().dat2, 167);
    assert_eq!(world.effects.len(), 1);
}

#[test]
fn burn_effect_tick_removes_stale_attached_effect() {
    let mut world = World::default();
    assert!(!world.burn_character(CharacterId(1)));

    let effect_id = world.next_effect_id();
    let mut effect = Effect::new(EF_BURN, effect_id as i32, 0, TICKS_PER_SECOND as i32 * 60);
    effect.target_character = Some(CharacterId(99));
    effect.strength = 1;
    world.effects.insert(effect_id, effect);

    world.tick_effects();

    assert!(world.effects.is_empty());
}

#[test]
fn extinguish_driver_removes_burn_effect() {
    let mut world = World::default();
    let mut character = character(1);
    character.hp = 50 * POWERSCALE;
    world.add_character(character);
    let mut water = item(7, ItemFlags::USED | ItemFlags::USE);
    water.driver = crate::item_driver::IDR_EXTINGUISH;
    world.add_item(water);
    assert!(world.burn_character(CharacterId(1)));

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_EXTINGUISH,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        2,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::Extinguish {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            extinguished: true,
        }
    );
    assert!(world.effects.is_empty());
}

#[test]
fn extinguish_driver_reports_refreshing_when_not_burning() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut water = item(7, ItemFlags::USED | ItemFlags::USE);
    water.driver = crate::item_driver::IDR_EXTINGUISH;
    world.add_item(water);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_EXTINGUISH,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        2,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::Extinguish {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            extinguished: false,
        }
    );
}

#[test]
fn world_timer_callback_expires_and_destroys_burned_out_torch() {
    let mut world = World::default();
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(7));
    let mut torch = item(7, ItemFlags::USED | ItemFlags::USE);
    torch.carried_by = Some(CharacterId(1));
    torch.driver = IDR_TORCH;
    torch.driver_data = vec![0, 1, 1, 20];
    world.add_character(character);
    world.add_item(torch);

    world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_TORCH,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );
    world.tick.0 = 30 * crate::tick::TICKS_PER_SECOND;
    let outcomes = world.process_due_timers(1);

    assert_eq!(outcomes.len(), 1);
    assert!(matches!(
        outcomes[0],
        ItemDriverOutcome::TorchExpired { item_name: _, .. }
    ));
    assert!(!world.items.contains_key(&ItemId(7)));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.inventory[30], None);
    assert!(character.flags.contains(CharacterFlags::ITEMS));
}

#[test]
fn world_timer_extinguishes_burning_torch_underwater() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    character.inventory[30] = Some(ItemId(7));
    let mut torch = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::NODECAY);
    torch.carried_by = Some(CharacterId(1));
    torch.driver = IDR_TORCH;
    torch.driver_data = vec![1, 0, 10, 20];
    torch.modifier_value[0] = 20;
    torch.sprite = -1;
    world.add_character(character);
    world.add_item(torch);
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::UNDERWATER);

    let outcome = world.execute_item_driver_request_with_context(
        ItemDriverRequest::Driver {
            driver: IDR_TORCH,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::TorchExtinguishedUnderwater {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            schedule_after_ticks: 30 * crate::tick::TICKS_PER_SECOND,
        }
    );
    let torch = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(torch.driver_data[0], 0);
    assert_eq!(torch.modifier_value[0], 0);
    assert_eq!(torch.sprite, 0);
    assert!(!torch.flags.contains(ItemFlags::NODECAY));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert!(character.flags.contains(CharacterFlags::ITEMS));
    assert_eq!(world.timers.used_timers(), 1);
}

#[test]
fn world_executes_area17_burndown_barrel_ignite_and_timer() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.x = 8;
    actor.y = 8;
    world.add_character(actor);
    let mut nearby_npc = character(2);
    nearby_npc.x = 8;
    nearby_npc.y = 24;
    world.add_character(nearby_npc);
    let mut barrel = item(7, ItemFlags::USED | ItemFlags::USE);
    barrel.driver = crate::item_driver::IDR_BURNDOWN;
    barrel.sprite = 51076;
    barrel.driver_data = vec![0];
    assert!(world.map.set_item_map(&mut barrel, 10, 10));
    world.add_item(barrel);

    let request = ItemDriverRequest::Driver {
        driver: crate::item_driver::IDR_BURNDOWN,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        world.execute_item_driver_request_with_context(
            request,
            17,
            &ItemDriverContext {
                cursor_driver: Some(crate::item_driver::IDR_TORCH),
                cursor_drdata0: Some(1),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::BurndownIgnite {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
    let barrel = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(barrel.driver_data[0], 20);
    assert_eq!(barrel.sprite, 51077);
    assert_eq!(barrel.modifier_index[0], CharacterValue::Light as i16);
    assert_eq!(barrel.modifier_value[0], 200);
    assert_eq!(
        world.map.tile(10, 10).unwrap().foreground_sprite,
        1024 << 16
    );
    assert_eq!(world.timers.used_timers(), 1);
    let nearby_messages = &world
        .characters
        .get(&CharacterId(2))
        .unwrap()
        .driver_messages;
    assert_eq!(nearby_messages.len(), 1);
    assert_eq!(nearby_messages[0].message_type, NT_NPC);
    assert_eq!(nearby_messages[0].dat1, NTID_TWOCITY_PICK);
    assert_eq!(nearby_messages[0].dat2, 1);

    world.tick = Tick(TICKS_PER_SECOND * 5);
    let outcomes = world.process_due_timers(17);
    assert_eq!(
        outcomes,
        vec![ItemDriverOutcome::BurndownTimerTick { item_id: ItemId(7) }]
    );
    let barrel = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(barrel.driver_data[0], 19);
    assert_eq!(barrel.sprite, 51078);
    assert_eq!(world.timers.used_timers(), 1);

    world.items.get_mut(&ItemId(7)).unwrap().driver_data[0] = 16;
    world.tick = Tick(TICKS_PER_SECOND * 10);
    let outcomes = world.process_due_timers(17);
    assert_eq!(outcomes.len(), 1);
    let barrel = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(barrel.driver_data[0], 15);
    assert_eq!(barrel.modifier_value[0], 0);
    assert_eq!(world.map.tile(10, 10).unwrap().foreground_sprite, 0);
}

#[test]
fn targeted_fireball_sets_up_projectile_action() {
    let mut world = World::default();
    world.tick = Tick(240);
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.mana = 10 * POWERSCALE;
    caster.values[0][CharacterValue::Fireball as usize] = 50;
    caster.values[0][CharacterValue::Tactics as usize] = 24;
    world.spawn_character(caster, 10, 10);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Fireball,
        arg1: 15,
        arg2: 10,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));

    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(caster.action, action::FIREBALL1);
    assert_eq!(caster.act1, 15);
    assert_eq!(caster.act2, 10);
    assert_eq!(caster.dir, Direction::Right as u8);
    assert_eq!(caster.mana, 7 * POWERSCALE);

    world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
    assert!(world.tick_basic_actions()[0].ok);
    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(caster.action, action::FIREBALL2);
    assert_eq!(caster.step, 0);
    let effect = world.effects.values().next().unwrap();
    assert_eq!(effect.effect_type, EF_FIREBALL);
    assert_eq!(effect.serial, 1);
    assert_eq!(effect.start_tick, 240);
    assert_eq!(effect.stop_tick, 240 + TICKS_PER_SECOND as i32);
    assert_eq!(effect.strength, 53);
    assert_eq!(effect.light, 200);
    assert_eq!(effect.caster, Some(CharacterId(1)));
    assert_eq!(effect.caster_serial, 1);
    assert_eq!((effect.from_x, effect.from_y), (10, 10));
    assert_eq!((effect.to_x, effect.to_y), (15, 10));
    assert_eq!((effect.x, effect.y), (10 * 1024 + 512, 10 * 1024 + 512));
    // C `act_fireball` (`act.c:955-960`): `NT_CHAR` gated on `CF_NONOTIFY`,
    // then unconditional `NT_SPELL` carrying the fireball effect id.
    assert_eq!(caster.driver_messages[0].message_type, NT_CHAR);
    assert_eq!(caster.driver_messages[0].dat1, 1);
    assert_eq!(caster.driver_messages[1].message_type, NT_SPELL);
    assert_eq!(caster.driver_messages[1].dat1, 1);
    assert_eq!(
        caster.driver_messages[1].dat2,
        CharacterValue::Fireball as i32
    );
    assert_eq!(caster.driver_messages[1].dat3, effect.serial);
}

#[test]
fn fireball_effect_moves_one_tile_per_tick_and_marks_map_slot() {
    let mut world = World::default();
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.x = 10;
    caster.y = 10;
    caster.act1 = 13;
    caster.act2 = 10;
    caster.values[0][CharacterValue::Fireball as usize] = 50;
    world.spawn_character(caster, 10, 10);
    let caster = world.characters.get(&CharacterId(1)).unwrap().clone();
    let effect_id = world.create_fireball_effect(&caster);

    world.tick_effects();

    let effect = world.effects.get(&effect_id).unwrap();
    assert_eq!((effect.x, effect.y), (11 * 1024 + 512, 10 * 1024 + 512));
    assert_eq!((effect.last_x, effect.last_y), (11, 10));
    assert_eq!(effect.fields, vec![11 + 10 * world.map.width() as i32]);
    assert_eq!(world.map.tile(11, 10).unwrap().effects[0], effect_id as u16);
}

#[test]
fn fireball_effect_explodes_on_character_block_and_applies_area_damage() {
    let mut world = World::default();
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.x = 10;
    caster.y = 10;
    caster.act1 = 15;
    caster.act2 = 10;
    caster.values[0][CharacterValue::Fireball as usize] = 50;
    caster.values[0][CharacterValue::Tactics as usize] = 24;
    let mut target = character(2);
    target.flags.insert(CharacterFlags::ALIVE);
    target.hp = 30 * POWERSCALE;
    target.values[0][CharacterValue::Immunity as usize] = 20;
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 12, 10);
    let caster = world.characters.get(&CharacterId(1)).unwrap().clone();
    let effect_id = world.create_fireball_effect(&caster);

    world.tick_effects();
    world.tick_effects();

    assert_ne!(
        world
            .effects
            .get(&effect_id)
            .map(|effect| effect.effect_type),
        Some(EF_FIREBALL)
    );
    assert_eq!(world.map.tile(11, 10).unwrap().effects, [0; 4]);
    assert!(world
        .effects
        .values()
        .any(|effect| effect.effect_type == EF_EXPLODE
            && effect.base_sprite == 50050
            && effect.strength == 8));
    assert_eq!(
        world.drain_pending_sound_specials()[0].special.special_type,
        6
    );
    let target = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(target.hp, 14_100);
    assert!(target.flags.contains(CharacterFlags::UPDATE));
}

#[test]
fn fireball_effect_respects_runtime_attack_policy() {
    let mut world = World::default();
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.x = 10;
    caster.y = 10;
    caster.act1 = 15;
    caster.act2 = 10;
    caster.values[0][CharacterValue::Fireball as usize] = 50;
    let mut target = character(2);
    target
        .flags
        .insert(CharacterFlags::ALIVE | CharacterFlags::PLAYER);
    target.hp = 30 * POWERSCALE;
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 12, 10);
    let caster = world.characters.get(&CharacterId(1)).unwrap().clone();
    world.create_fireball_effect(&caster);

    world.tick_effects_with_attack_policy(|_, _, _, _| false);
    world.tick_effects_with_attack_policy(|_, _, _, _| false);

    let target = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(target.hp, 30 * POWERSCALE);
    assert!(!target.flags.contains(CharacterFlags::UPDATE));
}

#[test]
fn fireball_impact_uses_legacy_hurt_reduction() {
    let mut world = World::default();
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.x = 10;
    caster.y = 10;
    caster.act1 = 15;
    caster.act2 = 10;
    caster.values[0][CharacterValue::Fireball as usize] = 50;
    caster.values[0][CharacterValue::Tactics as usize] = 24;
    let mut target = character(2);
    target.flags.insert(CharacterFlags::ALIVE);
    target.hp = 30 * POWERSCALE;
    target.lifeshield = POWERSCALE;
    target.values[0][CharacterValue::Armor as usize] = 20;
    target.values[0][CharacterValue::Immunity as usize] = 20;
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 12, 10);
    let caster = world.characters.get(&CharacterId(1)).unwrap().clone();
    world.create_fireball_effect(&caster);

    world.tick_effects();
    world.tick_effects();

    let target = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(target.hp, 15_200);
    assert_eq!(target.lifeshield, 0);
    assert_eq!(target.driver_messages[0].message_type, NT_GOTHIT);
    assert_eq!(target.driver_messages[0].dat1, 1);
    assert_eq!(target.driver_messages[0].dat2, 14_800);
    assert_eq!(
        world.characters[&CharacterId(1)].driver_messages[0].message_type,
        NT_DIDHIT
    );
}

#[test]
fn fireball_hit_earth_demon_shoots_weaker_fireball_back() {
    let mut world = World::default();
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.x = 10;
    caster.y = 10;
    caster.act1 = 15;
    caster.act2 = 10;
    caster.values[0][CharacterValue::Fireball as usize] = 50;
    caster.values[0][CharacterValue::Tactics as usize] = 24;
    let mut target = character(2);
    target
        .flags
        .insert(CharacterFlags::ALIVE | CharacterFlags::EDEMON);
    target.hp = 30 * POWERSCALE;
    target.values[0][CharacterValue::Immunity as usize] = 20;
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 12, 10);
    let caster = world.characters.get(&CharacterId(1)).unwrap().clone();
    world.create_fireball_effect(&caster);

    world.tick_effects();
    world.tick_effects();

    let shootback = world
        .effects
        .values()
        .find(|effect| effect.effect_type == EF_FIREBALL)
        .unwrap();
    assert_eq!(shootback.strength, 52);
    assert_eq!(shootback.caster, Some(CharacterId(2)));
    assert_eq!((shootback.from_x, shootback.from_y), (12, 10));
    assert_eq!((shootback.to_x, shootback.to_y), (10, 10));
    let target = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(target.hp, 14_100);
}

#[test]
fn fireball_reflect_item_reduces_charges_and_shoots_back() {
    let mut world = World::default();
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.x = 10;
    caster.y = 10;
    caster.act1 = 15;
    caster.act2 = 10;
    caster.values[0][CharacterValue::Fireball as usize] = 50;
    caster.values[0][CharacterValue::Tactics as usize] = 24;
    let mut target = character(2);
    target.flags.insert(CharacterFlags::ALIVE);
    target.hp = 30 * POWERSCALE;
    target.inventory[0] = Some(ItemId(70));
    let mut reflector = item(70, ItemFlags::USED);
    reflector.template_id = IID_REFLECT_FIREBALL;
    reflector.carried_by = Some(CharacterId(2));
    reflector.driver_data = 100_u32.to_le_bytes().to_vec();
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 12, 10);
    world.items.insert(ItemId(70), reflector);
    let caster = world.characters.get(&CharacterId(1)).unwrap().clone();
    world.create_fireball_effect(&caster);

    world.tick_effects();
    world.tick_effects();

    let target = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(target.hp, 30 * POWERSCALE);
    assert_eq!(target.inventory[0], Some(ItemId(70)));
    let reflector = world.items.get(&ItemId(70)).unwrap();
    assert_eq!(read_u32_le_prefix(&reflector.driver_data), 47);
    assert_eq!(reflector.description, "47 units left.");
    let reflected = world.effects.values().next().unwrap();
    assert_eq!(reflected.effect_type, EF_FIREBALL);
    assert_eq!(reflected.strength, 52);
    assert_eq!(reflected.caster, Some(CharacterId(2)));
    assert_eq!((reflected.from_x, reflected.from_y), (12, 10));
    assert_eq!((reflected.to_x, reflected.to_y), (10, 10));
}

#[test]
fn fireball_reflect_item_is_destroyed_when_charges_are_used_up() {
    let mut world = World::default();
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.x = 10;
    caster.y = 10;
    caster.act1 = 15;
    caster.act2 = 10;
    caster.values[0][CharacterValue::Fireball as usize] = 50;
    caster.values[0][CharacterValue::Tactics as usize] = 24;
    let mut target = character(2);
    target.flags.insert(CharacterFlags::ALIVE);
    target.hp = 30 * POWERSCALE;
    target.inventory[0] = Some(ItemId(71));
    let mut reflector = item(71, ItemFlags::USED);
    reflector.template_id = IID_REFLECT_FIREBALL;
    reflector.carried_by = Some(CharacterId(2));
    reflector.driver_data = 10_u32.to_le_bytes().to_vec();
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 12, 10);
    world.items.insert(ItemId(71), reflector);
    let caster = world.characters.get(&CharacterId(1)).unwrap().clone();
    world.create_fireball_effect(&caster);

    world.tick_effects();
    world.tick_effects();

    let target = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(target.hp, 30 * POWERSCALE);
    assert_eq!(target.inventory[0], None);
    assert!(!world.items.contains_key(&ItemId(71)));
    let reflected = world.effects.values().next().unwrap();
    assert_eq!(reflected.effect_type, EF_FIREBALL);
    assert_eq!(reflected.strength, 52);
    assert_eq!(reflected.caster, Some(CharacterId(2)));
}

#[test]
fn targeted_ball_sets_up_projectile_action() {
    let mut world = World::default();
    world.tick = Tick(300);
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.mana = 10 * POWERSCALE;
    caster.values[0][CharacterValue::Flash as usize] = 50;
    caster.values[0][CharacterValue::Tactics as usize] = 24;
    world.spawn_character(caster, 10, 10);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Ball,
        arg1: 15,
        arg2: 10,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));

    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(caster.action, action::BALL1);
    assert_eq!((caster.act1, caster.act2), (15, 10));
    assert_eq!(caster.dir, Direction::Right as u8);
    assert_eq!(caster.mana, 7 * POWERSCALE);

    world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
    assert!(world.tick_basic_actions()[0].ok);
    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(caster.action, action::BALL2);
    assert_eq!(caster.step, 0);
    let effect = world.effects.values().next().unwrap();
    assert_eq!(effect.effect_type, EF_BALL);
    assert_eq!(effect.stop_tick, 300 + TICKS_PER_SECOND as i32 * 5);
    assert_eq!(effect.strength, 53);
    assert_eq!(effect.light, 80);
    assert_eq!((effect.from_x, effect.from_y), (10, 10));
    assert_eq!((effect.to_x, effect.to_y), (15, 10));
    // C `act_ball` (`act.c:1057-1061`): `NT_CHAR` gated on `CF_NONOTIFY`,
    // then unconditional `NT_SPELL` with the ball effect id - note the C
    // source uses `V_FLASH` (not a `V_BALL`, which doesn't exist) as the
    // payload, matching `create_ball`'s own `spellpower(cn, V_FLASH)`.
    assert_eq!(caster.driver_messages[0].message_type, NT_CHAR);
    assert_eq!(caster.driver_messages[1].message_type, NT_SPELL);
    assert_eq!(caster.driver_messages[1].dat2, CharacterValue::Flash as i32);
    assert_eq!(caster.driver_messages[1].dat3, effect.serial);
}

#[test]
fn earthrain_action_completion_creates_legacy_area_effect() {
    let mut world = World::default();
    world.tick = Tick(400);
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.hp = 10 * POWERSCALE;

    crate::do_action::do_earthrain(&mut caster, &world.map, 12, 10, 7, 100).unwrap();
    caster.duration = 1;
    world.spawn_character(caster, 10, 10);

    let completion = world.tick_basic_actions().pop().unwrap();
    assert!(completion.ok);

    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(caster.action, action::IDLE);
    let effect = world.effects.values().next().unwrap();
    assert_eq!(effect.effect_type, EF_EARTHRAIN);
    assert_eq!(effect.strength, 7);
    assert_eq!(effect.light, 10);
    assert_eq!(effect.stop_tick, 400 + TICKS_PER_SECOND as i32 * 60);
    assert_eq!(
        world.map.tile(12, 10).unwrap().effects[0],
        effect.serial as u16
    );
}

#[test]
fn earthmud_action_completion_creates_legacy_area_effect() {
    let mut world = World::default();
    let mut caster = character(1);
    caster.hp = 10 * POWERSCALE;

    crate::do_action::do_earthmud(&mut caster, &world.map, 12, 10, 4, 100).unwrap();
    caster.duration = 1;
    world.spawn_character(caster, 10, 10);

    let completion = world.tick_basic_actions().pop().unwrap();
    assert!(completion.ok);

    let effect = world.effects.values().next().unwrap();
    assert_eq!(effect.effect_type, EF_EARTHMUD);
    assert_eq!(effect.strength, 4);
    assert_eq!(effect.light, 0);
    assert_eq!(
        world.map.tile(12, 10).unwrap().effects[0],
        effect.serial as u16
    );
}

#[test]
fn ball_effect_moves_slowly_and_strikes_nearby_targets() {
    let mut world = World::default();
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.x = 10;
    caster.y = 10;
    caster.act1 = 15;
    caster.act2 = 10;
    caster.values[0][CharacterValue::Flash as usize] = 50;
    caster.values[0][CharacterValue::Tactics as usize] = 24;
    let mut target = character(2);
    target.flags.insert(CharacterFlags::ALIVE);
    target.hp = 30 * POWERSCALE;
    target.values[0][CharacterValue::Immunity as usize] = 20;
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 12, 10);
    let caster = world.characters.get(&CharacterId(1)).unwrap().clone();
    let effect_id = world.create_ball_effect(&caster);

    world.tick_effects();

    let effect = world.effects.get(&effect_id).unwrap();
    assert_eq!((effect.x, effect.y), (10 * 1024 + 640, 10 * 1024 + 512));
    assert_eq!(effect.number_of_enemies, 1);
    let strike = world
        .effects
        .values()
        .find(|effect| effect.effect_type == EF_STRIKE)
        .unwrap();
    assert_eq!(strike.light, 50);
    assert_eq!(strike.strength, 53);
    assert_eq!(strike.target_character, Some(CharacterId(2)));
    assert_eq!((strike.x, strike.y), (10, 10));
    assert_eq!(strike.stop_tick, 2);
    let target = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(target.hp, 28_675);
    assert!(target.flags.contains(CharacterFlags::UPDATE));
    let sounds = world.drain_pending_sound_specials();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].character_id, CharacterId(1));
    assert_eq!(sounds[0].special.special_type, 30);
}

#[test]
fn ball_effect_earth_demon_target_takes_reduced_strike_damage() {
    // C `check_strike_near` (`system/effect.c:864`): earth demons don't
    // suffer as much damage from a nearby ball/flash strike - compares
    // against the identical non-edemon setup in
    // `ball_effect_moves_slowly_and_strikes_nearby_targets` above (which
    // takes the target to `28_675` hp, i.e. `1325` raw damage).
    let mut world = World::default();
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.x = 10;
    caster.y = 10;
    caster.act1 = 15;
    caster.act2 = 10;
    caster.values[0][CharacterValue::Flash as usize] = 50;
    caster.values[0][CharacterValue::Tactics as usize] = 24;
    let mut target = character(2);
    target
        .flags
        .insert(CharacterFlags::ALIVE | CharacterFlags::EDEMON);
    target.hp = 30 * POWERSCALE;
    target.values[0][CharacterValue::Immunity as usize] = 20;
    // Trained (`value[1]`) `V_DEMON` gap of 30 vs the caster's 0 hits the
    // `min(dam/4, dam*30/10)` cap, i.e. exactly a 25% reduction.
    target.values[1][CharacterValue::Demon as usize] = 30;
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 12, 10);
    let caster = world.characters.get(&CharacterId(1)).unwrap().clone();
    world.create_ball_effect(&caster);

    world.tick_effects();

    let target = world.characters.get(&CharacterId(2)).unwrap();
    let raw_damage = 30 * POWERSCALE - target.hp;
    assert_eq!(raw_damage, 1325 - 1325 / 4);
}

#[test]
fn ball_effect_respects_runtime_attack_policy() {
    let mut world = World::default();
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.x = 10;
    caster.y = 10;
    caster.act1 = 15;
    caster.act2 = 10;
    caster.values[0][CharacterValue::Flash as usize] = 50;
    caster.values[0][CharacterValue::Tactics as usize] = 24;
    let mut target = character(2);
    target
        .flags
        .insert(CharacterFlags::ALIVE | CharacterFlags::PLAYER);
    target.hp = 30 * POWERSCALE;
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 12, 10);
    let caster = world.characters.get(&CharacterId(1)).unwrap().clone();
    let effect_id = world.create_ball_effect(&caster);

    world.tick_effects_with_attack_policy(|_, _, _, _| false);

    let effect = world.effects.get(&effect_id).unwrap();
    assert_eq!(effect.number_of_enemies, 0);
    assert!(!world
        .effects
        .values()
        .any(|effect| effect.effect_type == EF_STRIKE));
    let target = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(target.hp, 30 * POWERSCALE);
    assert!(!target.flags.contains(CharacterFlags::UPDATE));
}

#[test]
fn strike_effect_refreshes_matching_target_and_expires_after_two_ticks() {
    let mut world = World::default();

    let effect_id = world.create_or_refresh_strike_effect(CharacterId(2), 10, 11, 53);
    assert_eq!(world.effects.len(), 1);
    assert_eq!(world.effects.get(&effect_id).unwrap().stop_tick, 2);

    world.tick = Tick(1);
    let refreshed_id = world.create_or_refresh_strike_effect(CharacterId(2), 10, 11, 53);
    assert_eq!(refreshed_id, effect_id);
    assert_eq!(world.effects.len(), 1);
    assert_eq!(world.effects.get(&effect_id).unwrap().stop_tick, 3);

    world.tick = Tick(2);
    world.tick_effects();
    assert!(world.effects.contains_key(&effect_id));

    world.tick = Tick(3);
    world.tick_effects();
    assert!(!world.effects.contains_key(&effect_id));
}

#[test]
fn character_fireball_targets_stationary_character_position() {
    let mut world = World::default();
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.mana = 10 * POWERSCALE;
    caster.values[0][CharacterValue::Fireball as usize] = 50;
    let target = character(2);
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 15, 10);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::FireballCharacter,
        arg1: 2,
        arg2: 2,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));

    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(caster.action, action::FIREBALL1);
    assert_eq!((caster.act1, caster.act2), (15, 10));
    assert_eq!(caster.dir, Direction::Right as u8);
    assert_eq!(caster.mana, 7 * POWERSCALE);
}

#[test]
fn character_fireball_predicts_moving_target_like_c_fireball_driver() {
    let mut world = World::default();
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.mana = 10 * POWERSCALE;
    caster.values[0][CharacterValue::Fireball as usize] = 50;
    let mut target = character(2);
    target.action = action::WALK;
    target.dir = Direction::Right as u8;
    target.duration = 8;
    target.step = 1;
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 18, 10);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::FireballCharacter,
        arg1: 2,
        arg2: 2,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));

    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(caster.action, action::FIREBALL1);
    assert_eq!((caster.act1, caster.act2), (20, 10));
    assert_eq!(caster.dir, Direction::Right as u8);
}

#[test]
fn character_fireball_rejects_stale_serial_guard() {
    let mut world = World::default();
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.mana = 10 * POWERSCALE;
    caster.values[0][CharacterValue::Fireball as usize] = 50;
    let target = character(2);
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 15, 10);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::FireballCharacter,
        arg1: 2,
        arg2: 99,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));

    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(caster.action, action::IDLE);
    assert_eq!(caster.mana, 10 * POWERSCALE);
}

#[test]
fn self_targeted_fireball_sets_up_firering_and_damages_adjacent_targets() {
    let mut world = World::default();
    world.tick = Tick(250);
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.mana = 10 * POWERSCALE;
    caster.values[0][CharacterValue::Fireball as usize] = 50;
    caster.values[0][CharacterValue::Tactics as usize] = 24;
    let mut target = character(2);
    target.flags.insert(CharacterFlags::ALIVE);
    target.hp = 30 * POWERSCALE;
    target.lifeshield = POWERSCALE;
    target.values[0][CharacterValue::Armor as usize] = 20;
    target.values[0][CharacterValue::Immunity as usize] = 20;
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 11, 10);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Fireball,
        arg1: 10,
        arg2: 10,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(caster.action, action::FIRERING);
    assert_eq!(caster.mana, 7 * POWERSCALE);

    world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
    assert!(world.tick_basic_actions()[0].ok);

    let caster = world.characters.get(&CharacterId(1)).unwrap();
    let spell_id = caster.inventory[29].unwrap();
    let spell = world.items.get(&spell_id).unwrap();
    assert_eq!(spell.driver, IDR_FIRERING);
    assert_eq!(spell.carried_by, Some(CharacterId(1)));
    assert_eq!(spell.modifier_index, [0, 0, 0, 0, 0]);
    assert_eq!(
        u32::from_le_bytes(spell.driver_data[0..4].try_into().unwrap()),
        274
    );
    assert_eq!(
        u32::from_le_bytes(spell.driver_data[4..8].try_into().unwrap()),
        250
    );
    let target = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(target.hp, 15_200);
    assert_eq!(target.lifeshield, 0);
    assert!(target.flags.contains(CharacterFlags::UPDATE));
    assert_eq!(target.driver_messages[0].message_type, NT_GOTHIT);
    assert_eq!(target.driver_messages[0].dat1, 1);
    assert_eq!(target.driver_messages[0].dat2, 14_800);
    assert_eq!(
        world.characters[&CharacterId(1)].driver_messages[0].message_type,
        NT_DIDHIT
    );
    let firering_effect = world
        .effects
        .values()
        .find(|effect| effect.effect_type == EF_FIRERING)
        .unwrap();
    assert_eq!(firering_effect.target_character, Some(CharacterId(1)));
    assert_eq!(firering_effect.stop_tick, 257);
    assert_eq!(firering_effect.light, 20);
    assert_eq!(firering_effect.strength, 50);
    let burn_effect = world
        .effects
        .values()
        .find(|effect| effect.effect_type == EF_BURN)
        .unwrap();
    assert_eq!(burn_effect.target_character, Some(CharacterId(2)));
    assert_eq!(burn_effect.stop_tick, 258);
    assert_eq!(burn_effect.light, 20);
    assert_eq!(burn_effect.strength, 0);
    assert_eq!(world.timers.used_timers(), 1);
}
