use super::*;

#[test]
fn chestspawn_timer_resets_when_spawn_is_gone() {
    let mut world = World::default();
    let mut spawner = item(8, ItemFlags::USE);
    spawner.driver = IDR_CHESTSPAWN;
    spawner.sprite = 1235;
    spawner.x = 10;
    spawner.y = 10;
    spawner.driver_data = vec![0, 1, 44, 0, 0, 0, 0, 0];
    world.items.insert(spawner.id, spawner);
    assert!(world.schedule_item_driver_timer(ItemId(8), CharacterId(0), 1));
    world.tick.0 = 1;

    let outcomes = world.process_due_timers(2);

    assert_eq!(outcomes.len(), 1);
    let spawner = &world.items[&ItemId(8)];
    assert_eq!(spawner.sprite, 1234);
    assert_eq!(spawner.driver_data[1], 0);
}

#[test]
fn swampspawn_timer_uses_nearby_player_and_records_spawn_result() {
    let mut world = World::default();
    let mut spawner = item(8, ItemFlags::USE);
    spawner.driver = IDR_SWAMPSPAWN;
    spawner.sprite = 21008;
    spawner.x = 10;
    spawner.y = 10;
    spawner.driver_data = vec![1, 1, 3];
    spawner.driver_data.resize(20, 0);
    spawner.driver_data[16..20].copy_from_slice(&21000_u32.to_le_bytes());
    world.items.insert(spawner.id, spawner);
    world.map.tile_mut(10, 10).unwrap().ground_sprite = 59423;
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    assert!(world.spawn_character(player, 13, 10));
    assert!(world.schedule_item_driver_timer(ItemId(8), CharacterId(0), 1));
    world.tick.0 = 1;

    let outcomes = world.process_due_timers(15);

    assert_eq!(outcomes.len(), 1);
    assert!(matches!(
        outcomes[0],
        ItemDriverOutcome::SwampSpawn {
            template: "swamp27n",
            x: 10,
            y: 10,
            schedule_after_ticks: 3,
            ..
        }
    ));
    assert!(world.apply_swampspawn_spawn_result(ItemId(8), CharacterId(44), 99));
    let spawner = &world.items[&ItemId(8)];
    assert_eq!(&spawner.driver_data[4..8], &44_u32.to_le_bytes());
    assert_eq!(&spawner.driver_data[8..12], &99_u32.to_le_bytes());
    assert_eq!(&spawner.driver_data[12..16], &1_u32.to_le_bytes());
}

#[test]
fn skelraise_timer_uses_spawned_character_serial_guard() {
    let mut world = World::default();
    world.characters.insert(CharacterId(1), character(1));
    let mut raised = character(2);
    raised.serial = 77;
    world.characters.insert(CharacterId(2), raised);
    world.items.insert(ItemId(9), item(9, ItemFlags::empty()));
    let mut chair = item(8, ItemFlags::USE);
    chair.driver = IDR_SKELRAISE;
    chair.sprite = 500;
    world.items.insert(ItemId(8), chair);

    assert!(world.apply_skelraise_raise(ItemId(8), CharacterId(1), ItemId(9), CharacterId(2), 77,));
    assert_eq!(
        &world.items[&ItemId(8)].driver_data[8..12],
        &77_u32.to_le_bytes()
    );
    assert_eq!(world.items[&ItemId(8)].sprite, 501);

    world.characters.get_mut(&CharacterId(2)).unwrap().serial = 78;
    assert!(world.apply_skelraise_timer(ItemId(8)));

    let chair = &world.items[&ItemId(8)];
    assert_eq!(chair.driver_data[2], 0);
    assert_eq!(chair.sprite, 500);
}

#[test]
fn skelraise_dust_activates_empty_chair_until_timer_reset() {
    let mut world = World::default();
    let mut chair = item(8, ItemFlags::USE);
    chair.driver = IDR_SKELRAISE;
    chair.sprite = 500;
    chair.driver_data = vec![0; 12];
    chair.driver_data[4..8].copy_from_slice(&123_u32.to_le_bytes());
    chair.driver_data[8..12].copy_from_slice(&456_u32.to_le_bytes());
    world.items.insert(ItemId(8), chair);

    assert!(world.apply_skelraise_dust(ItemId(8)));

    let chair = &world.items[&ItemId(8)];
    assert_eq!(chair.driver_data[2], 1);
    assert_eq!(&chair.driver_data[4..12], &[0; 8]);
    assert_eq!(chair.sprite, 501);

    assert!(world.apply_skelraise_timer(ItemId(8)));
    let chair = &world.items[&ItemId(8)];
    assert_eq!(chair.driver_data[2], 0);
    assert_eq!(chair.sprite, 500);
}

#[test]
fn world_edemon_tube_discovers_loader_target_on_timer() {
    let mut world = World::default();
    world.add_character(character(0));
    let mut tube = item(7, ItemFlags::USED | ItemFlags::USE);
    tube.driver = IDR_EDEMONTUBE;
    tube.driver_data = vec![4, 0, 0, 0, 0, 0];
    world.add_item(tube);
    let mut loader = item(8, ItemFlags::USED | ItemFlags::USE);
    loader.driver = IDR_EDEMONLOADER;
    loader.driver_data = vec![4, 42, 0];
    loader.x = 20;
    loader.y = 20;
    world.add_item(loader);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_EDEMONTUBE,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        6,
    );

    assert!(matches!(outcome, ItemDriverOutcome::LightChanged { .. }));
    let tube = &world.items[&ItemId(7)];
    assert_eq!(tube.sprite, 14138);
    assert_eq!(tube.modifier_value[0], 200);
    assert_eq!(
        u16::from_le_bytes([tube.driver_data[2], tube.driver_data[3]]),
        20
    );
    assert_eq!(
        u16::from_le_bytes([tube.driver_data[4], tube.driver_data[5]]),
        21
    );
}

#[test]
fn world_edemon_block_timer_returns_to_origin_and_is_scheduled_on_startup() {
    let mut world = World::default();
    world.add_character(character(0));
    let mut block = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK);
    block.driver = IDR_EDEMONBLOCK;
    block.driver_data = vec![0, 0, 0, 0, 10, 0, 10, 0];
    assert!(world.map.set_item_map(&mut block, 12, 10));
    world.map.tile_mut(10, 10).unwrap().ground_sprite = 12158;
    world.add_item(block);

    assert_eq!(world.schedule_existing_light_timers(), 1);
    world.tick.0 = TICKS_PER_SECOND * 60 * 15 + 3;
    let outcome = world.execute_item_driver_request_with_context(
        ItemDriverRequest::Driver {
            driver: IDR_EDEMONBLOCK,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        6,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert!(matches!(outcome, ItemDriverOutcome::EdemonBlockMove { .. }));
    assert_eq!(world.map.tile(12, 10).unwrap().item, 0);
    assert_eq!(world.map.tile(10, 10).unwrap().item, 7);
    assert_eq!(
        (world.items[&ItemId(7)].x, world.items[&ItemId(7)].y),
        (10, 10)
    );
    assert_eq!(world.timers.used_timers(), 2);
}

#[test]
fn world_applies_fdemon_waypoint_marker_and_timer() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.serial = 1234;
    world.add_character(player);
    let mut waypoint = item(7, ItemFlags::USED | ItemFlags::USE);
    waypoint.driver = crate::item_driver::IDR_FDEMONWAYPOINT;
    assert!(world.map.set_item_map(&mut waypoint, 10, 10));
    world.add_item(waypoint);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_FDEMONWAYPOINT,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        8,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::FdemonWaypoint {
            spotted_enemy: true,
            target_character_id: Some(CharacterId(1)),
            target_serial: Some(1234),
            ..
        }
    ));
    let waypoint = &world.items[&ItemId(7)];
    assert_eq!(waypoint.driver_data[0], 1);
    assert_eq!(
        u32::from_le_bytes(waypoint.driver_data[4..8].try_into().unwrap()),
        1
    );
    assert_eq!(
        u32::from_le_bytes(waypoint.driver_data[8..12].try_into().unwrap()),
        1234
    );
    assert_eq!(waypoint.sprite, 14200);
    assert_eq!(world.timers.used_timers(), 1);
    assert_eq!(world.characters[&CharacterId(1)].driver_messages.len(), 1);
    assert_eq!(
        world.characters[&CharacterId(1)].driver_messages[0].message_type,
        NT_NPC
    );
    assert_eq!(
        world.characters[&CharacterId(1)].driver_messages[0].dat1,
        NTID_FDEMON
    );
    assert_eq!(
        world.characters[&CharacterId(1)].driver_messages[0].dat2,
        FDEMON_MSG_WAYPOINT
    );
    assert_eq!(world.characters[&CharacterId(1)].driver_messages[0].dat3, 7);

    let mut demon = character(2);
    demon.flags.insert(CharacterFlags::FDEMON);
    demon.x = 9;
    demon.y = 10;
    world.add_character(demon);
    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_FDEMONWAYPOINT,
            item_id: ItemId(7),
            character_id: CharacterId(2),
            spec: 0,
        },
        8,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::FdemonWaypoint {
            spotted_enemy: false,
            ..
        }
    ));
    let waypoint = &world.items[&ItemId(7)];
    assert_eq!(waypoint.driver_data[0], 0);
    assert_eq!(
        u32::from_le_bytes(waypoint.driver_data[4..8].try_into().unwrap()),
        0
    );
    assert_eq!(
        u32::from_le_bytes(waypoint.driver_data[8..12].try_into().unwrap()),
        0
    );
    assert_eq!(waypoint.sprite, 14202);
    assert_eq!(world.characters[&CharacterId(2)].driver_messages.len(), 1);
    assert_eq!(
        world.characters[&CharacterId(2)].driver_messages[0].message_type,
        NT_NPC
    );
    assert_eq!(
        world.characters[&CharacterId(2)].driver_messages[0].dat1,
        NTID_FDEMON
    );
    assert_eq!(
        world.characters[&CharacterId(2)].driver_messages[0].dat2,
        FDEMON_MSG_WAYPOINT
    );
    assert_eq!(world.characters[&CharacterId(2)].driver_messages[0].dat3, 7);
}

#[test]
fn world_applies_fdemon_farm_foreground_and_timer() {
    let mut world = World::default();
    world.add_character(character(0));
    let mut farm = item(7, ItemFlags::USED | ItemFlags::USE);
    farm.driver = IDR_FDEMONFARM;
    farm.driver_data = vec![5, 24, 24];
    assert!(world.map.set_item_map(&mut farm, 10, 10));
    world.map.tile_mut(10, 10).unwrap().foreground_sprite = 123;
    world.add_item(farm);

    let outcome = world.execute_item_driver_request_with_context(
        ItemDriverRequest::Driver {
            driver: IDR_FDEMONFARM,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        8,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::FdemonFarmChanged { .. }
    ));
    assert_eq!(
        world.map.tile(10, 10).unwrap().foreground_sprite,
        (59040 << 16) | 123
    );
    assert_eq!(world.timers.used_timers(), 1);
}

#[test]
fn world_applies_fdemon_lava_activation_and_timer_damage() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.cursor_item = Some(ItemId(9));
    world.add_character(player);
    let mut lava = item(7, ItemFlags::USED | ItemFlags::USE);
    lava.driver = IDR_FDEMONLAVA;
    assert!(world.map.set_item_map(&mut lava, 10, 10));
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::MOVEBLOCK);
    world.add_item(lava);
    let mut container = item(9, ItemFlags::USED);
    container.template_id = (0x01 << 24) | 0x00004B;
    container.driver_data = vec![2];
    container.sprite = 100;
    container.carried_by = Some(CharacterId(1));
    world.add_item(container);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_FDEMONLAVA,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        8,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::FdemonLavaActivated { amount: 1, .. }
    ));
    let lava_tile = world.map.tile(10, 10).unwrap();
    assert!(!lava_tile.flags.contains(MapFlags::MOVEBLOCK));
    assert_eq!(lava_tile.foreground_sprite, 1034 << 16);
    assert_eq!(world.items[&ItemId(7)].driver_data[0], 120);
    assert_eq!(world.items[&ItemId(7)].sprite, 14366);
    assert_eq!(world.items[&ItemId(9)].driver_data[0], 1);
    assert_eq!(world.items[&ItemId(9)].sprite, 99);
    assert_eq!(world.timers.used_timers(), 1);

    world.add_character(character(0));
    world.items.get_mut(&ItemId(7)).unwrap().driver_data[0] = 20;
    let mut target = character(2);
    target.x = 10;
    target.y = 10;
    target.hp = 20 * POWERSCALE;
    world.add_character(target);
    world.map.tile_mut(10, 10).unwrap().character = 2;

    let outcome = world.execute_item_driver_request_with_context(
        ItemDriverRequest::Driver {
            driver: IDR_FDEMONLAVA,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        8,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::FdemonLavaPulse {
            stage: 19,
            damage,
            armor_percent: 50,
            ..
        } if damage == 10 * POWERSCALE
    ));
    assert_eq!(world.items[&ItemId(7)].sprite, 14364);
    assert_eq!(
        world.map.tile(10, 10).unwrap().foreground_sprite,
        1024 << 16
    );
    assert_eq!(world.characters[&CharacterId(2)].hp, 10 * POWERSCALE);
}

#[test]
fn labtorch_extinguish_notifies_nearby_npcs() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.flags.remove(CharacterFlags::PLAYER);
    assert!(world.spawn_character(actor, 10, 10));
    assert!(world.spawn_character(character(2), 12, 10));
    assert!(world.spawn_character(character(3), 80, 80));

    let mut torch = item(7, ItemFlags::USED | ItemFlags::USE);
    torch.driver = IDR_LABTORCH;
    torch.x = 10;
    torch.y = 10;
    torch.sprite = 200;
    torch.driver_data = vec![1, 25];
    torch.modifier_index[0] = 9;
    torch.modifier_value[0] = 25;
    world.add_item(torch);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_LABTORCH,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
    );

    assert!(matches!(outcome, ItemDriverOutcome::LightChanged { .. }));
    assert_eq!(world.items[&ItemId(7)].driver_data[0], 0);
    assert_eq!(world.items[&ItemId(7)].modifier_value[0], 0);
    assert_eq!(
        world.characters[&CharacterId(2)].driver_messages,
        vec![crate::character_driver::CharacterDriverMessage {
            message_type: NT_NPC,
            dat1: NTID_LABGNOMETORCH,
            dat2: 7,
            dat3: 1,
            text: None,
        }]
    );
    assert!(world.characters[&CharacterId(3)].driver_messages.is_empty());
}

#[test]
fn world_usetrap_schedules_target_item_driver_timer() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut trap = item(7, ItemFlags::USED | ItemFlags::USE);
    trap.driver = IDR_USETRAP;
    trap.driver_data = vec![20, 30];
    let mut door = item(8, ItemFlags::USED | ItemFlags::USE);
    door.driver = IDR_DOOR;
    door.x = 20;
    door.y = 30;
    world.add_item(trap);
    world.add_item(door);
    world.map.tile_mut(20, 30).unwrap().item = 8;

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_USETRAP,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(outcome, ItemDriverOutcome::TriggerMapItem { .. }));
    assert_eq!(world.timers.used_timers(), 1);
    for _ in 0..(TICKS_PER_SECOND / 2) {
        world.advance();
    }
    let outcomes = world.process_due_timers(1);
    assert_eq!(outcomes.len(), 1);
    assert!(matches!(
        outcomes[0],
        ItemDriverOutcome::DoorToggle {
            item_id: ItemId(8),
            character_id: CharacterId(1)
        }
    ));
}

#[test]
fn world_steptrap_timer_discovers_nearby_non_steptrap_target() {
    let mut world = World::default();
    let mut trap = item(7, ItemFlags::USED | ItemFlags::USE);
    trap.driver = IDR_STEPTRAP;
    trap.x = 10;
    trap.y = 10;
    trap.driver_data = vec![0, 0];
    let mut target = item(8, ItemFlags::USED | ItemFlags::USE);
    target.driver = IDR_DOOR;
    target.x = 11;
    target.y = 10;
    world.add_item(trap);
    world.add_item(target);
    world.map.tile_mut(10, 10).unwrap().item = 7;
    world.map.tile_mut(11, 10).unwrap().item = 8;
    assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));

    world.advance();
    let outcomes = world.process_due_timers(1);

    assert_eq!(
        outcomes,
        vec![ItemDriverOutcome::StepTrapDiscoverTarget { item_id: ItemId(7) }]
    );
    let trap = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(&trap.driver_data[..2], &[11, 10]);
}

#[test]
fn world_spiketrap_damages_and_resets_on_timer() {
    let mut world = World::default();
    let mut character = character(1);
    character.hp = 10_000;
    world.add_character(character);
    let mut trap = item(7, ItemFlags::USED | ItemFlags::USE);
    trap.driver = IDR_SPIKETRAP;
    trap.driver_data = vec![0, 4];
    world.add_item(trap);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_SPIKETRAP,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::SpikeTrapTriggered { .. }
    ));
    assert_eq!(world.characters.get(&CharacterId(1)).unwrap().hp, 6_000);
    assert_eq!(world.items.get(&ItemId(7)).unwrap().driver_data[0], 1);
    for _ in 0..TICKS_PER_SECOND {
        world.advance();
    }
    let outcomes = world.process_due_timers(1);
    assert_eq!(
        outcomes,
        vec![ItemDriverOutcome::SpikeTrapReset { item_id: ItemId(7) }]
    );
    assert_eq!(world.items.get(&ItemId(7)).unwrap().driver_data[0], 0);
}

#[test]
fn world_retries_or_removes_area18_bone_bridge_timer_cleanup_like_c() {
    let mut world = World::default();
    world.add_character(timer_callback_character());
    let mut bone = item(8, ItemFlags::USED | ItemFlags::USE);
    bone.driver = IDR_BONEBRIDGE;
    bone.driver_data = vec![5, 1];
    bone.x = 12;
    bone.y = 10;
    world.map.tile_mut(12, 10).unwrap().item = 8;
    world.map.tile_mut(12, 10).unwrap().flags = MapFlags::TMOVEBLOCK;
    world.add_item(bone);

    let request = ItemDriverRequest::Driver {
        driver: IDR_BONEBRIDGE,
        item_id: ItemId(8),
        character_id: CharacterId(0),
        spec: 0,
    };
    let context = ItemDriverContext {
        timer_call: true,
        ..ItemDriverContext::default()
    };
    assert_eq!(
        world.execute_item_driver_request_with_context(request, 18, &context),
        ItemDriverOutcome::BoneBridgeTimerTick { item_id: ItemId(8) }
    );
    assert_eq!(world.items.get(&ItemId(8)).unwrap().driver_data[1], 1);
    assert_eq!(world.timers.used_timers(), 1);

    world.map.tile_mut(12, 10).unwrap().flags = MapFlags::empty();
    world.items.get_mut(&ItemId(8)).unwrap().driver_data[1] = 9;
    assert_eq!(
        world.execute_item_driver_request_with_context(request, 18, &context),
        ItemDriverOutcome::BoneBridgeTimerTick { item_id: ItemId(8) }
    );
    assert!(!world.items.contains_key(&ItemId(8)));
    let tile = world.map.tile(12, 10).unwrap();
    assert_eq!(tile.item, 0);
    assert!(tile.flags.contains(MapFlags::MOVEBLOCK));
}

#[test]
fn world_completes_use_as_pending_item_driver_request() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    character.dir = Direction::Right as u8;
    character.action = action::USE;
    character.duration = 1;
    character.act1 = 7;
    character.act2 = 42;
    world.add_character(character);
    let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
    assert!(world.map.set_item_map(&mut item, 11, 10));
    world.add_item(item);

    let completed = world.tick_basic_actions();
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].character_id, CharacterId(1));
    assert_eq!(completed[0].action_id, action::USE);
    assert!(completed[0].ok);
    assert_eq!(completed[0].item_use.unwrap().item_id, ItemId(7));
    assert_eq!(completed[0].item_use.unwrap().spec, 42);
}

#[test]
fn world_applies_pentagram_activate_and_timer_deactivate() {
    let mut world = World::default();
    let mut listener = character(1);
    listener.flags.insert(CharacterFlags::PLAYER);
    listener.x = 10;
    listener.y = 10;
    world.map.tile_mut(10, 10).unwrap().character = 1;
    world.add_character(listener);

    let mut pent = item(7, ItemFlags::USED | ItemFlags::USE);
    pent.driver = IDR_PENT;
    pent.x = 10;
    pent.y = 10;
    pent.sprite = 1000;
    pent.modifier_index[0] = CharacterValue::Light as i16;
    pent.modifier_value[0] = 10;
    pent.driver_data = vec![3, 0, 4, 0, 9];
    world.add_item(pent);

    let outcome = world.apply_item_driver_outcome(
        ItemDriverOutcome::PentagramActivate {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            level: 3,
            color: 4,
        },
        4,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::PentagramActivate { .. }
    ));
    let pent = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(pent.driver_data[1], 255);
    assert_eq!(pent.sprite, 1004);
    assert_eq!(pent.modifier_value[0], 100);
    assert_eq!(
        world.drain_pending_sound_specials()[0].special.special_type,
        42
    );

    let outcome = world.apply_item_driver_outcome(
        ItemDriverOutcome::PentagramTimer {
            item_id: ItemId(7),
            level: 3,
            status: 1,
            area_status: 9,
        },
        4,
    );

    assert!(matches!(outcome, ItemDriverOutcome::PentagramTimer { .. }));
    let pent = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(pent.driver_data[1], 0);
    assert_eq!(pent.driver_data[4], 0);
    assert_eq!(pent.sprite, 1000);
    assert_eq!(pent.modifier_value[0], 10);
}

#[test]
fn staffer2_mine_timer_restores_opened_mine_wall() {
    let mut world = World::default();
    let mut mine = item(8, ItemFlags::USED | ItemFlags::VOID);
    mine.driver = IDR_STAFFER2;
    mine.driver_data = vec![2, 0, 0, 8, 1];
    mine.sprite = 15078;
    mine.x = 10;
    mine.y = 10;
    world.add_item(mine);

    assert!(world.schedule_item_driver_timer(ItemId(8), CharacterId(0), 1));
    world.tick = Tick(1);
    let outcomes = world.process_due_timers(29);

    assert_eq!(
        outcomes,
        vec![ItemDriverOutcome::StafferMineTimer { item_id: ItemId(8) }]
    );
    let tile = world.map.tile(10, 10).unwrap();
    assert_eq!(tile.item, 8);
    assert!(tile
        .flags
        .contains(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK));
    let item = world.items.get(&ItemId(8)).unwrap();
    assert_eq!(item.driver_data[3], 0);
    assert!(item.flags.contains(ItemFlags::USE | ItemFlags::SIGHTBLOCK));
    assert!(!item.flags.contains(ItemFlags::VOID));
    assert_eq!(item.sprite, 15070);
}

#[test]
fn staffer2_block_move_pushes_block_and_timer_returns_home() {
    let mut world = World::default();
    world.tick = Tick(10);
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.dir = Direction::Right as u8;
    world.add_character(player);
    let mut block = item(8, ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK);
    block.driver = IDR_STAFFER2;
    block.driver_data = vec![3];
    assert!(world.map.set_item_map(&mut block, 10, 10));
    world.add_item(block);
    world.map.tile_mut(11, 10).unwrap().ground_sprite = 20291;

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_STAFFER2,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        29,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::StafferBlockMove { .. }
    ));
    assert_eq!(world.map.tile(10, 10).unwrap().item, 0);
    assert_eq!(world.map.tile(11, 10).unwrap().item, 8);
    assert_eq!(world.items.get(&ItemId(8)).unwrap().x, 11);

    world.tick = Tick(10 + TICKS_PER_SECOND * 60 * 3);
    assert!(world.schedule_item_driver_timer(ItemId(8), CharacterId(0), 1));
    world.advance();
    let outcomes = world.process_due_timers(29);
    assert_eq!(
        outcomes,
        vec![ItemDriverOutcome::StafferBlockTimer { item_id: ItemId(8) }]
    );
    assert_eq!(world.map.tile(11, 10).unwrap().item, 0);
    assert_eq!(world.map.tile(10, 10).unwrap().item, 8);
    assert_eq!(world.items.get(&ItemId(8)).unwrap().x, 10);
}

#[test]
fn caligar_weight_move_pushes_weight_and_timer_returns_home() {
    let mut world = World::default();
    world.tick = Tick(10);
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.dir = Direction::Right as u8;
    world.add_character(player);
    let mut weight = item(8, ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK);
    weight.driver = IDR_CALIGAR;
    weight.driver_data = vec![2];
    assert!(world.map.set_item_map(&mut weight, 10, 10));
    world.add_item(weight);
    world.map.tile_mut(11, 10).unwrap().ground_sprite = 20797;

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_CALIGAR,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        36,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::CaligarWeightMove { .. }
    ));
    assert_eq!(world.map.tile(10, 10).unwrap().item, 0);
    assert_eq!(world.map.tile(11, 10).unwrap().item, 8);
    let moved = world.items.get(&ItemId(8)).unwrap();
    assert_eq!((moved.x, moved.y), (11, 10));
    assert_eq!(
        u32::from_le_bytes(moved.driver_data[4..8].try_into().unwrap()),
        10
    );
    assert_eq!(
        u16::from_le_bytes(moved.driver_data[8..10].try_into().unwrap()),
        10
    );
    assert_eq!(
        u16::from_le_bytes(moved.driver_data[10..12].try_into().unwrap()),
        10
    );

    world.tick = Tick(10 + TICKS_PER_SECOND * 60 * 5 + 1);
    assert!(world.schedule_item_driver_timer(ItemId(8), CharacterId(0), 1));
    world.advance();
    let outcomes = world.process_due_timers(36);
    assert_eq!(
        outcomes,
        vec![ItemDriverOutcome::CaligarWeightTimer { item_id: ItemId(8) }]
    );
    assert_eq!(world.map.tile(11, 10).unwrap().item, 0);
    assert_eq!(world.map.tile(10, 10).unwrap().item, 8);
    assert_eq!(world.items.get(&ItemId(8)).unwrap().x, 10);
}

#[test]
fn world_applies_area14_gastrap_damage_foreground_and_timers() {
    let mut world = World::default();
    world.map = MapGrid::new(20, 20);
    let mut trap = item(8, ItemFlags::USED | ItemFlags::USE);
    trap.driver = crate::item_driver::IDR_GASTRAP;
    trap.driver_data = vec![2, 0];
    assert!(world.map.set_item_map(&mut trap, 10, 10));
    world.add_item(trap);
    world.map.tile_mut(11, 10).unwrap().foreground_sprite = 15300;

    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.hp = 10 * POWERSCALE;
    assert!(world.spawn_character(player, 10, 10));

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_GASTRAP,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        14,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::GasTrapPulse {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            power: 2,
            schedule_initial_trigger: true,
            schedule_animation: true,
        }
    );
    assert_eq!(world.items.get(&ItemId(8)).unwrap().driver_data[1], 1);
    assert_eq!(world.map.tile(11, 10).unwrap().foreground_sprite, 15301);
    assert!(world.characters.get(&CharacterId(1)).unwrap().hp < 10 * POWERSCALE);
    assert_eq!(world.timers.used_timers(), 2);
}

#[test]
fn world_applies_area14_gastrap_timer_animation_reset() {
    let mut world = World::default();
    world.map = MapGrid::new(20, 20);
    world.add_character(character(0));
    let mut trap = item(8, ItemFlags::USED | ItemFlags::USE);
    trap.driver = crate::item_driver::IDR_GASTRAP;
    trap.driver_data = vec![2, 8];
    assert!(world.map.set_item_map(&mut trap, 10, 10));
    world.add_item(trap);
    world.map.tile_mut(10, 9).unwrap().foreground_sprite = 15318;
    assert!(world.schedule_item_driver_timer(ItemId(8), CharacterId(0), 1));

    world.tick = Tick(1);
    let outcomes = world.process_due_timers(14);

    assert_eq!(
        outcomes,
        vec![ItemDriverOutcome::GasTrapPulse {
            item_id: ItemId(8),
            character_id: CharacterId(0),
            power: 2,
            schedule_initial_trigger: false,
            schedule_animation: false,
        }]
    );
    assert_eq!(world.items.get(&ItemId(8)).unwrap().driver_data[1], 0);
    assert_eq!(world.map.tile(10, 9).unwrap().foreground_sprite, 15318);
    assert_eq!(world.timers.used_timers(), 0);
}

#[test]
fn swamparm_timer_damages_horizontal_targets_on_frame_twelve() {
    let mut world = World {
        map: MapGrid::new(20, 20),
        ..World::default()
    };
    world.add_character(character(0));
    let mut target = character(3);
    target.hp = 20 * POWERSCALE;
    assert!(world.spawn_character(target, 11, 10));
    let mut adjacent_row = character(4);
    adjacent_row.hp = 20 * POWERSCALE;
    assert!(world.spawn_character(adjacent_row, 11, 11));
    let mut arm = item(8, ItemFlags::USED | ItemFlags::USE);
    arm.driver = IDR_SWAMPARM;
    arm.x = 10;
    arm.y = 10;
    arm.sprite = 21011;
    arm.driver_data = vec![11];
    world.add_item(arm);

    let outcome = world.execute_item_driver_timer_request(
        ItemDriverRequest::Driver {
            driver: IDR_SWAMPARM,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        },
        15,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::SwampArmPulse {
            damage_now: true,
            ..
        }
    ));
    assert_eq!(world.characters[&CharacterId(3)].hp, 10 * POWERSCALE);
    assert_eq!(world.characters[&CharacterId(4)].hp, 20 * POWERSCALE);
}

#[test]
fn swampwhisp_timer_moves_item_map_slot_and_reschedules() {
    let mut world = World {
        map: MapGrid::new(20, 20),
        ..World::default()
    };
    world.add_character(character(0));
    let mut whisp = item(8, ItemFlags::USED | ItemFlags::USE);
    whisp.driver = IDR_SWAMPWHISP;
    whisp.sprite = 20945;
    whisp.driver_data = vec![11, 10, 10, Direction::Down as u8];
    assert!(world.map.set_item_map(&mut whisp, 10, 10));
    world.add_item(whisp);

    let outcome = world.execute_item_driver_timer_request(
        ItemDriverRequest::Driver {
            driver: IDR_SWAMPWHISP,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        },
        15,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::SwampWhispPulse {
            moved_from: Some((10, 10)),
            moved_to: Some((10, 11)),
            ..
        }
    ));
    assert_eq!(world.items[&ItemId(8)].y, 11);
    assert_eq!(world.map.tile(10, 10).unwrap().item, 0);
    assert_eq!(world.map.tile(10, 11).unwrap().item, 8);

    world.tick = Tick(2);
    let due = world.process_due_timers(15);
    assert_eq!(due.len(), 1);
}

#[test]
fn stat_scroll_use_triggers_update_character_recompute() {
    // C `raise_value_exp` (`src/system/skill.c:315-377`) calls
    // `update_char(cn)` after bumping `value[1][v]`; the stat scroll driver
    // (`base.c:6031` `IDR_STATSCROLL`) loops calling `raise_value_exp` per
    // scroll charge. Raising Body Control (index 23) should immediately
    // recompute the derived Armor bonus (`body_control * 5`,
    // `create.c:1710`), proving `World::update_character` is wired at this
    // outcome instead of only mutating the bare `values[1]` entry.
    let mut world = World::default();
    let mut owner = character(1);
    owner.values[0][CharacterValue::BodyControl as usize] = 10;
    owner.values[1][CharacterValue::BodyControl as usize] = 10;
    owner.inventory[30] = Some(ItemId(7));
    world.add_character(owner);

    let mut scroll = item(7, ItemFlags::USED | ItemFlags::USE);
    scroll.driver = crate::item_driver::IDR_STATSCROLL;
    scroll.carried_by = Some(CharacterId(1));
    scroll.driver_data = vec![CharacterValue::BodyControl as u8, 1];
    world.add_item(scroll);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_STATSCROLL,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::StatScrollUsed {
            value,
            raised: 1,
            ..
        } if value == CharacterValue::BodyControl as u8
    ));
    let owner = &world.characters[&CharacterId(1)];
    assert_eq!(owner.values[1][CharacterValue::BodyControl as usize], 11);
    // Armor's effective value must reflect the freshly raised Body Control
    // skill (11 * 5 = 55), not the stale pre-raise bonus (10 * 5 = 50).
    assert_eq!(owner.values[0][CharacterValue::Armor as usize], 55);
}

#[test]
fn stat_scroll_use_triggers_check_levelup() {
    // C `raise_value_exp` (`src/system/skill.c:315-361`) calls
    // `check_levelup(cn)` right after granting the raise cost as exp, for
    // every successful raise. Raising a cheap skill (Pulse, index 11,
    // `skill_start` 1, `skill_raise_cost_factor` 1) from a low bare value
    // costs enough exp in one charge to cross multiple level thresholds,
    // so the stat scroll driver must leave the character leveled up, not
    // just exp-richer.
    let mut world = World::default();
    let mut owner = character(1);
    owner.level = 1;
    owner.exp = 0;
    owner.exp_used = 0;
    owner.values[0][CharacterValue::Pulse as usize] = 1;
    owner.values[1][CharacterValue::Pulse as usize] = 1;
    owner.inventory[30] = Some(ItemId(7));
    world.add_character(owner);

    let mut scroll = item(7, ItemFlags::USED | ItemFlags::USE);
    scroll.driver = crate::item_driver::IDR_STATSCROLL;
    scroll.carried_by = Some(CharacterId(1));
    scroll.driver_data = vec![CharacterValue::Pulse as u8, 1];
    world.add_item(scroll);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_STATSCROLL,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::StatScrollUsed { raised: 1, .. }
    ));
    let owner = &world.characters[&CharacterId(1)];
    // raise_cost(11, 1, seyan=false): nr = 1 - 1 + 1 + 5 = 6, cost =
    // 6^3 / 10 = 21 exp granted, which crosses `exp2level` thresholds
    // 1 (exp 0) and 2 (exp 16), landing on level 2.
    assert_eq!(owner.exp, 21);
    assert_eq!(owner.level, 2);
}

#[test]
fn lollipop_lick_grants_exp_through_give_exp_not_a_raw_mutation() {
    // C `lollipop` (`base.c:3250`) calls `give_exp(cn, ...)`, so the
    // hardcore/global exp multipliers on `world.settings` must apply to a
    // lollipop lick, unlike a bare `character.exp +=`.
    let mut world = World::default();
    world.settings.exp_modifier = 2.0;
    let mut owner = character(1);
    owner.level = 10;
    owner.exp = 0;
    owner.inventory[30] = Some(ItemId(7));
    world.add_character(owner);

    let mut lollipop = item(7, ItemFlags::USED | ItemFlags::USE);
    lollipop.driver = crate::item_driver::IDR_FOOD;
    lollipop.carried_by = Some(CharacterId(1));
    lollipop.driver_data = vec![2, 0];
    world.add_item(lollipop);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_FOOD,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::LollipopLicked {
            exp_added: 6,
            lick_count: 1,
            ..
        }
    ));
    // lollipop_exp(level 10) == max(5, level_value(10)/750) == 6, doubled
    // by the 2.0 `exp_modifier` -> 12.
    let owner = &world.characters[&CharacterId(1)];
    assert_eq!(owner.exp, 12);
    assert!(owner.flags.contains(CharacterFlags::UPDATE));
}

// C `dungeondoor` (`area/13/dungeon.c:1807-1897`): confirms
// `apply_item_driver_outcome` wires `first_solve` into
// `World::resolve_dungeon_door_first_solve` (see `world::tests::
// dungeon_master` for that function's own detailed behavior coverage)
// before running the existing safe-zone teleport chain.
#[test]
fn dungeon_door_solved_first_solve_steals_jewels_before_teleporting() {
    let mut world = World::default();
    let attacker_clan = world.clan_registry.found_clan("Attacker", 0).unwrap();
    let defender_clan = world.clan_registry.found_clan("Defender", 0).unwrap();
    for _ in 0..14 {
        world.clan_registry.add_jewel(defender_clan).unwrap();
    }
    for _ in 0..12 {
        world.clan_registry.add_jewel(attacker_clan).unwrap();
    }

    let mut winner = character(1);
    winner.flags.insert(CharacterFlags::PLAYER);
    winner.clan = attacker_clan;
    winner.clan_serial = world.clan_registry.serial(attacker_clan);
    assert!(world.spawn_character(winner, 10, 10));

    let outcome = world.apply_item_driver_outcome(
        ItemDriverOutcome::DungeonDoorSolved {
            item_id: ItemId(9),
            character_id: CharacterId(1),
            clan_number: u32::from(defender_clan),
            catacomb: 0,
            first_solve: true,
        },
        0,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::DungeonDoorSolved { .. }
    ));
    // Stole `min(14-11,3) = 3` jewels.
    assert_eq!(world.clan_registry.jewel_count(attacker_clan), 15);
    assert_eq!(
        world
            .clan_registry
            .identity(defender_clan)
            .unwrap()
            .economy
            .training_score,
        150
    );
    // The existing safe-zone teleport chain still ran afterwards.
    let winner = &world.characters[&CharacterId(1)];
    assert_eq!((winner.x, winner.y), (245, 250));
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| text.message.starts_with("You won")));
}

#[test]
fn dungeon_door_solved_skips_first_solve_side_effects_when_already_solved() {
    let mut world = World::default();
    let mut winner = character(1);
    winner.flags.insert(CharacterFlags::PLAYER);
    assert!(world.spawn_character(winner, 10, 10));

    let outcome = world.apply_item_driver_outcome(
        ItemDriverOutcome::DungeonDoorSolved {
            item_id: ItemId(9),
            character_id: CharacterId(1),
            clan_number: 0,
            catacomb: 0,
            first_solve: false,
        },
        0,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::DungeonDoorSolved { .. }
    ));
    // No "You won"/catacomb-collapse feedback - only the teleport chain
    // ran, exactly matching the pre-existing (non-first-solve) behavior.
    assert!(world.drain_pending_system_texts().is_empty());
    let winner = &world.characters[&CharacterId(1)];
    assert_eq!((winner.x, winner.y), (245, 250));
}
