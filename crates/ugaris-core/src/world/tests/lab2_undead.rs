use super::*;

#[test]
fn give_completion_notifies_lab2_undead_receiver() {
    let mut world = World::default();
    let mut giver = character(1);
    giver.cursor_item = Some(ItemId(9));
    giver.dir = Direction::Right as u8;
    assert!(world.spawn_character(giver, 10, 10));
    let mut undead = character(2);
    undead.driver_state = Some(CharacterDriverState::Lab2Undead(
        Lab2UndeadDriverData::default(),
    ));
    assert!(world.spawn_character(undead, 11, 10));
    let mut holy_water = item(9, ItemFlags::USED);
    holy_water.carried_by = Some(CharacterId(1));
    holy_water.driver = IDR_LAB2_WATER;
    holy_water.driver_data = vec![5];
    world.add_item(holy_water);

    assert!(world.complete_give(CharacterId(1), CharacterId(2)));

    let undead = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(undead.cursor_item, Some(ItemId(9)));
    assert_eq!(undead.driver_messages.len(), 1);
    assert_eq!(undead.driver_messages[0].message_type, NT_GIVE);
    assert_eq!(undead.driver_messages[0].dat1, 1);
}

#[test]
fn lab2_undead_holy_water_damages_true_undead_and_delays_regen() {
    let mut world = World::default();
    world.tick = Tick(100);
    let mut giver = character(1);
    giver
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::NONOMAGIC);
    assert!(world.spawn_character(giver, 10, 10));

    let mut undead = character(2);
    undead.name = "Restless Undead".to_string();
    undead.flags.insert(CharacterFlags::NODEATH);
    undead.hp = 25 * POWERSCALE;
    undead.values[1][CharacterValue::Hp as usize] = 25;
    undead.driver_state = Some(CharacterDriverState::Lab2Undead(Lab2UndeadDriverData {
        undead: 1,
        regenerate_item_id: Some(ItemId(10)),
        ..Lab2UndeadDriverData::default()
    }));
    undead.cursor_item = Some(ItemId(9));
    undead.push_driver_message(NT_GIVE, 1, 9, 0);
    assert!(world.spawn_character(undead, 11, 10));

    let mut holy_water = item(9, ItemFlags::USED);
    holy_water.carried_by = Some(CharacterId(2));
    holy_water.driver = IDR_LAB2_WATER;
    holy_water.driver_data = vec![5];
    world.add_item(holy_water);
    let mut regen = item(10, ItemFlags::USED);
    regen.carried_by = Some(CharacterId(2));
    regen.driver = IDR_LAB2_REGENERATE;
    regen.driver_data = vec![0; 12];
    world.add_item(regen);

    assert_eq!(world.process_lab2_undead_message_actions(CharacterId(2)), 1);

    assert!(!world.items.contains_key(&ItemId(9)));
    let undead = world.characters.get(&CharacterId(2)).unwrap();
    assert!(!undead.flags.contains(CharacterFlags::NODEATH));
    assert_eq!(undead.hp, 5 * POWERSCALE);
    assert_eq!(world.effects.values().next().unwrap().effect_type, EF_MIST);
    let regen = &world.items[&ItemId(10)];
    assert_eq!(
        u32::from_le_bytes(regen.driver_data[8..12].try_into().unwrap()),
        100 + (TICKS_PER_SECOND * 20) as u32
    );
    assert_eq!(world.drain_pending_area_texts()[0].message, "Arrgh!");
    assert_eq!(
        world.drain_pending_system_texts()[0].message,
        "You spill the holy water all over the Restless Undead."
    );
}

#[test]
fn lab2_undead_holy_water_is_laughed_off_in_nomagic_without_nonomagic() {
    let mut world = World::default();
    world
        .map
        .tile_mut(11, 10)
        .unwrap()
        .flags
        .insert(MapFlags::NOMAGIC);
    let mut giver = character(1);
    giver.flags.insert(CharacterFlags::PLAYER);
    assert!(world.spawn_character(giver, 10, 10));
    let mut undead = character(2);
    undead.hp = 25 * POWERSCALE;
    undead.driver_state = Some(CharacterDriverState::Lab2Undead(Lab2UndeadDriverData {
        undead: 1,
        ..Lab2UndeadDriverData::default()
    }));
    undead.cursor_item = Some(ItemId(9));
    undead.push_driver_message(NT_GIVE, 1, 9, 0);
    assert!(world.spawn_character(undead, 11, 10));
    let mut holy_water = item(9, ItemFlags::USED);
    holy_water.carried_by = Some(CharacterId(2));
    holy_water.driver = IDR_LAB2_WATER;
    holy_water.driver_data = vec![5];
    world.add_item(holy_water);

    assert_eq!(world.process_lab2_undead_message_actions(CharacterId(2)), 1);

    assert_eq!(world.characters[&CharacterId(2)].hp, 25 * POWERSCALE);
    assert!(world.effects.is_empty());
    assert_eq!(
        world.drain_pending_area_texts()[0].message,
        "Mwahahahaha..."
    );
}

#[test]
fn lab2_undead_crypt_patrol_removes_visible_second_corridor_enemy() {
    let mut world = World::default();
    let mut undead = character(2);
    undead.driver = CDR_LAB2UNDEAD;
    undead.driver_state = Some(CharacterDriverState::Lab2Undead(Lab2UndeadDriverData {
        patrol: 2,
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(1),
            priority: 1,
            last_seen_tick: 90,
            visible: true,
            last_x: 170,
            last_y: 155,
        }],
        ..Lab2UndeadDriverData::default()
    }));
    undead.push_driver_message(NT_CHAR, 1, 0, 0);
    assert!(world.spawn_character(undead, 171, 156));
    assert!(world.spawn_character(character(1), 170, 155));

    assert_eq!(world.process_lab2_undead_message_actions(CharacterId(2)), 1);

    let undead = world.characters.get(&CharacterId(2)).unwrap();
    let Some(CharacterDriverState::Lab2Undead(data)) = undead.driver_state.as_ref() else {
        panic!("lab2 undead state missing");
    };
    assert!(data.enemies.is_empty());
}

#[test]
fn lab2_undead_crypt_patrol_keeps_enemy_outside_second_corridor() {
    let mut world = World::default();
    let mut undead = character(2);
    undead.driver = CDR_LAB2UNDEAD;
    undead.driver_state = Some(CharacterDriverState::Lab2Undead(Lab2UndeadDriverData {
        patrol: 2,
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(1),
            priority: 1,
            last_seen_tick: 90,
            visible: true,
            last_x: 170,
            last_y: 159,
        }],
        ..Lab2UndeadDriverData::default()
    }));
    undead.push_driver_message(NT_CHAR, 1, 0, 0);
    assert!(world.spawn_character(undead, 171, 156));
    assert!(world.spawn_character(character(1), 170, 159));

    assert_eq!(world.process_lab2_undead_message_actions(CharacterId(2)), 0);

    let undead = world.characters.get(&CharacterId(2)).unwrap();
    let Some(CharacterDriverState::Lab2Undead(data)) = undead.driver_state.as_ref() else {
        panic!("lab2 undead state missing");
    };
    assert_eq!(data.enemies.len(), 1);
}

#[test]
fn lab2_undead_patrol_advances_waypoint_and_waits_like_c() {
    let mut world = World::default();
    let mut undead = character(2);
    undead.driver = CDR_LAB2UNDEAD;
    undead.driver_state = Some(CharacterDriverState::Lab2Undead(Lab2UndeadDriverData {
        patrol: 2,
        pat: 3,
        patstep: 8,
        patx: [171, 138, 138, 165, 167, 138, 138, 171],
        paty: [164, 164, 146, 146, 146, 146, 164, 164],
        ..Lab2UndeadDriverData::default()
    }));
    assert!(world.spawn_character(undead, 165, 146));

    assert!(world.process_lab2_undead_patrol_action(CharacterId(2), 22));

    let undead = world.characters.get(&CharacterId(2)).unwrap();
    let Some(CharacterDriverState::Lab2Undead(data)) = undead.driver_state.as_ref() else {
        panic!("lab2 undead state missing");
    };
    assert_eq!(data.pat, 4);
    assert_eq!(undead.duration, (TICKS_PER_SECOND * 2) as i32);
    assert_eq!(
        world.drain_pending_area_texts()[0].message,
        "A gust of wind?"
    );
}

#[test]
fn lab2_undead_patrol_walks_toward_current_waypoint() {
    let mut world = World::default();
    let mut undead = character(2);
    undead.driver = CDR_LAB2UNDEAD;
    undead.driver_state = Some(CharacterDriverState::Lab2Undead(Lab2UndeadDriverData {
        patrol: 1,
        pat: 0,
        patstep: 4,
        patx: [168, 168, 204, 204, 0, 0, 0, 0],
        paty: [178, 218, 218, 178, 0, 0, 0, 0],
        ..Lab2UndeadDriverData::default()
    }));
    assert!(world.spawn_character(undead, 166, 178));

    assert!(world.process_lab2_undead_patrol_action(CharacterId(2), 22));

    let undead = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(undead.action, action::WALK);
    assert_eq!(undead.dir, Direction::Right as u8);
    let Some(CharacterDriverState::Lab2Undead(data)) = undead.driver_state.as_ref() else {
        panic!("lab2 undead state missing");
    };
    assert_eq!(data.pat, 0);
}

#[test]
fn lab2_undead_dies_on_cathedral_ground_sprite() {
    let mut world = World::default();
    world.map.tile_mut(10, 10).unwrap().ground_sprite = 20456;
    let mut undead = character(2);
    undead.driver = CDR_LAB2UNDEAD;
    undead
        .flags
        .insert(CharacterFlags::ALIVE | CharacterFlags::NODEATH);
    undead.hp = 20 * POWERSCALE;
    undead.driver_state = Some(CharacterDriverState::Lab2Undead(
        Lab2UndeadDriverData::default(),
    ));
    assert!(world.spawn_character(undead, 10, 10));

    assert!(world.process_lab2_undead_cathedral_self_destruction(CharacterId(2)));

    let undead = world.characters.get(&CharacterId(2)).unwrap();
    assert!(undead.flags.contains(CharacterFlags::DEAD));
    assert!(!undead.flags.contains(CharacterFlags::NODEATH));
    assert_eq!(undead.hp, 0);
    assert_eq!(undead.deaths, 1);
    assert_eq!(world.effects.values().next().unwrap().effect_type, EF_MIST);
    assert_eq!(world.drain_pending_area_texts()[0].message, "Arrgh!");
}

#[test]
fn lab2_undead_cathedral_self_destruction_accepts_second_legacy_sprite() {
    let mut world = World::default();
    world.map.tile_mut(10, 10).unwrap().ground_sprite = 17062;
    let mut undead = character(2);
    undead.driver = CDR_LAB2UNDEAD;
    undead.driver_state = Some(CharacterDriverState::Lab2Undead(
        Lab2UndeadDriverData::default(),
    ));
    assert!(world.spawn_character(undead, 10, 10));

    assert_eq!(world.process_lab2_undead_cathedral_self_destructions(), 1);
    assert!(world.characters[&CharacterId(2)]
        .flags
        .contains(CharacterFlags::DEAD));
}

#[test]
fn lab2_undead_cathedral_self_destruction_ignores_other_tiles() {
    let mut world = World::default();
    world.map.tile_mut(10, 10).unwrap().ground_sprite = 20455;
    let mut undead = character(2);
    undead.driver = CDR_LAB2UNDEAD;
    undead.driver_state = Some(CharacterDriverState::Lab2Undead(
        Lab2UndeadDriverData::default(),
    ));
    assert!(world.spawn_character(undead, 10, 10));

    assert!(!world.process_lab2_undead_cathedral_self_destruction(CharacterId(2)));
    assert!(!world.characters[&CharacterId(2)]
        .flags
        .contains(CharacterFlags::DEAD));
    assert!(world.effects.is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn lab2_undead_crypt_patrol_closes_nearby_open_door() {
    let mut world = World::default();
    let mut undead = character(2);
    undead.driver = CDR_LAB2UNDEAD;
    undead.driver_state = Some(CharacterDriverState::Lab2Undead(Lab2UndeadDriverData {
        patrol: 2,
        ..Lab2UndeadDriverData::default()
    }));
    assert!(world.spawn_character(undead, 166, 156));

    let mut door = item(7, ItemFlags::USED | ItemFlags::USE);
    door.driver = IDR_DOOR;
    door.sprite = 101;
    door.driver_data = vec![1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    assert!(world.map.set_item_map(&mut door, 168, 156));
    world.add_item(door);

    assert!(world.process_lab2_undead_crypt_door_action(CharacterId(2)));

    let door = &world.items[&ItemId(7)];
    assert_eq!(door.driver_data[0], 0);
    assert_eq!(door.sprite, 100);
    assert!(world.drain_pending_sound_specials().is_empty());
}

#[test]
fn lab2_undead_crypt_patrol_does_not_close_from_wrong_side() {
    let mut world = World::default();
    let mut undead = character(2);
    undead.driver = CDR_LAB2UNDEAD;
    undead.driver_state = Some(CharacterDriverState::Lab2Undead(Lab2UndeadDriverData {
        patrol: 2,
        ..Lab2UndeadDriverData::default()
    }));
    assert!(world.spawn_character(undead, 169, 156));

    let mut door = item(7, ItemFlags::USED | ItemFlags::USE);
    door.driver = IDR_DOOR;
    door.sprite = 101;
    door.driver_data = vec![1];
    assert!(world.map.set_item_map(&mut door, 168, 156));
    world.add_item(door);

    assert!(!world.process_lab2_undead_crypt_door_action(CharacterId(2)));
    assert_eq!(world.items[&ItemId(7)].driver_data[0], 1);
}

#[test]
fn world_lab2_regenerate_timer_heals_target_and_reschedules() {
    let mut world = World::default();
    world.tick = Tick(120);
    let mut target = character(3);
    target.values[0][CharacterValue::Hp as usize] = 20;
    target.hp = 10 * POWERSCALE;
    world.add_character(target);

    let mut spell = item(8, ItemFlags::USED | ItemFlags::USE);
    spell.driver = IDR_LAB2_REGENERATE;
    spell.carried_by = Some(CharacterId(3));
    spell.driver_data = vec![12, 64, 0, 0, 3, 0, 0, 0, 120, 0, 0, 0];
    world.add_item(spell);

    let outcome = world.execute_item_driver_timer_request(
        ItemDriverRequest::Driver {
            driver: IDR_LAB2_REGENERATE,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        },
        22,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::Lab2RegenerateTick { .. }
    ));
    let target = world.characters.get(&CharacterId(3)).unwrap();
    assert_eq!(target.hp, 12 * POWERSCALE + POWERSCALE / 2);
    assert!(target.flags.contains(CharacterFlags::NODEATH));

    world.tick = Tick(132);
    let due = world.process_due_timers(22);
    assert_eq!(due.len(), 1);
}

#[test]
fn world_lab2_grave_empty_open_timer_closes_grave() {
    let mut world = World::default();
    let mut grave = item(8, ItemFlags::USED | ItemFlags::USE);
    grave.driver = crate::item_driver::IDR_LAB2_GRAVE;
    grave.sprite = 1201;
    grave.x = 4;
    grave.y = 5;
    grave.driver_data = vec![0; 16];
    grave.driver_data[4..8].copy_from_slice(&(-1_i32).to_le_bytes());
    grave.driver_data[8..12].copy_from_slice(&(-1_i32).to_le_bytes());
    world.add_item(grave);

    let outcome = world.execute_item_driver_timer_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_LAB2_GRAVE,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        },
        22,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab2GraveClose { item_id: ItemId(8) }
    );
    let grave = &world.items[&ItemId(8)];
    assert_eq!(grave.sprite, 1200);
    assert_eq!(
        i32::from_le_bytes(grave.driver_data[4..8].try_into().unwrap()),
        0
    );
    assert_eq!(
        i32::from_le_bytes(grave.driver_data[8..12].try_into().unwrap()),
        0
    );
}

#[test]
fn world_lab2_grave_live_timer_reschedules_while_undead_exists() {
    let mut world = World::default();
    world.tick = Tick(100);
    let mut undead = character(77);
    undead.serial = 123;
    world.add_character(undead);

    let mut grave = item(8, ItemFlags::USED | ItemFlags::USE);
    grave.driver = crate::item_driver::IDR_LAB2_GRAVE;
    grave.sprite = 1201;
    grave.driver_data = vec![0; 16];
    grave.driver_data[4..8].copy_from_slice(&77_i32.to_le_bytes());
    grave.driver_data[8..12].copy_from_slice(&123_i32.to_le_bytes());
    world.add_item(grave);

    let outcome = world.execute_item_driver_timer_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_LAB2_GRAVE,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        },
        22,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::Lab2GraveCheckOpen { .. }
    ));
    assert_eq!(world.items[&ItemId(8)].sprite, 1201);
    assert_eq!(world.timers.used_timers(), 1);
}

#[test]
fn world_lab2_grave_live_timer_closes_when_undead_serial_is_stale() {
    let mut world = World::default();
    let mut undead = character(77);
    undead.serial = 124;
    world.add_character(undead);

    let mut grave = item(8, ItemFlags::USED | ItemFlags::USE);
    grave.driver = crate::item_driver::IDR_LAB2_GRAVE;
    grave.sprite = 1201;
    grave.driver_data = vec![0; 16];
    grave.driver_data[4..8].copy_from_slice(&77_i32.to_le_bytes());
    grave.driver_data[8..12].copy_from_slice(&123_i32.to_le_bytes());
    world.add_item(grave);

    let outcome = world.execute_item_driver_timer_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_LAB2_GRAVE,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        },
        22,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::Lab2GraveCheckOpen { .. }
    ));
    let grave = &world.items[&ItemId(8)];
    assert_eq!(grave.sprite, 1200);
    assert_eq!(
        i32::from_le_bytes(grave.driver_data[4..8].try_into().unwrap()),
        0
    );
    assert_eq!(world.timers.used_timers(), 0);
}

#[test]
fn world_lab2_regenerate_timer_clears_nodeath_before_start() {
    let mut world = World::default();
    world.tick = Tick(100);
    let mut target = character(3);
    target.flags.insert(CharacterFlags::NODEATH);
    target.values[0][CharacterValue::Hp as usize] = 20;
    target.hp = 10 * POWERSCALE;
    world.add_character(target);

    let mut spell = item(8, ItemFlags::USED | ItemFlags::USE);
    spell.driver = IDR_LAB2_REGENERATE;
    spell.carried_by = Some(CharacterId(3));
    spell.driver_data = vec![12, 64, 0, 0, 3, 0, 0, 0, 120, 0, 0, 0];
    world.add_item(spell);

    let outcome = world.execute_item_driver_timer_request(
        ItemDriverRequest::Driver {
            driver: IDR_LAB2_REGENERATE,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        },
        22,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::Lab2RegenerateTick { .. }
    ));
    let target = world.characters.get(&CharacterId(3)).unwrap();
    assert_eq!(target.hp, 10 * POWERSCALE);
    assert!(!target.flags.contains(CharacterFlags::NODEATH));
}

#[test]
fn world_lab2_stepaction_daemon_check_notifies_nearby_drivers() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    world.add_character(actor);

    let mut nearby = character(2);
    nearby.x = 110;
    nearby.y = 110;
    world.add_character(nearby);

    let mut far = character(3);
    far.x = 200;
    far.y = 200;
    world.add_character(far);

    let mut step = item(8, ItemFlags::USED | ItemFlags::USE);
    step.driver = IDR_LAB2_STEPACTION;
    step.x = 100;
    step.y = 100;
    step.driver_data = vec![2];
    world.add_item(step);

    let outcome = world.execute_item_driver_timer_request(
        ItemDriverRequest::Driver {
            driver: IDR_LAB2_STEPACTION,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
        &ItemDriverContext::default(),
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::Lab2StepActionDaemonCheck { .. }
    ));
    let nearby_messages = &world.characters[&CharacterId(2)].driver_messages;
    assert_eq!(nearby_messages.len(), 1);
    assert_eq!(nearby_messages[0].message_type, NT_NPC);
    assert_eq!(nearby_messages[0].dat1, NTID_LAB2_DEAMONCHECK);
    assert_eq!(nearby_messages[0].dat2, 1);
    assert!(world.characters[&CharacterId(3)].driver_messages.is_empty());
}
