use super::*;

#[test]
fn world_moves_edemon_block_on_character_use_and_blocks_bad_target() {
    let mut world = World::default();
    let mut player = character(1);
    player.dir = Direction::Right as u8;
    assert!(world.spawn_character(player, 9, 10));
    let mut block = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK);
    block.driver = IDR_EDEMONBLOCK;
    assert!(world.map.set_item_map(&mut block, 10, 10));
    world.map.tile_mut(11, 10).unwrap().ground_sprite = 12150;
    world.add_item(block);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_EDEMONBLOCK,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        6,
    );

    assert!(matches!(outcome, ItemDriverOutcome::EdemonBlockMove { .. }));
    assert_eq!(world.map.tile(10, 10).unwrap().item, 0);
    assert!(!world
        .map
        .tile(10, 10)
        .unwrap()
        .flags
        .contains(MapFlags::TMOVEBLOCK));
    assert_eq!(world.map.tile(11, 10).unwrap().item, 7);
    assert!(world
        .map
        .tile(11, 10)
        .unwrap()
        .flags
        .contains(MapFlags::TMOVEBLOCK));
    assert_eq!(
        (world.items[&ItemId(7)].x, world.items[&ItemId(7)].y),
        (11, 10)
    );

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_EDEMONBLOCK,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        6,
    );
    assert!(matches!(outcome, ItemDriverOutcome::Noop));
    assert_eq!(
        (world.items[&ItemId(7)].x, world.items[&ItemId(7)].y),
        (11, 10)
    );
}

#[test]
fn world_applies_fdemon_blood_fill_and_flask_destruction() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.cursor_item = Some(ItemId(9));
    world.add_character(player);
    let mut blood = item(7, ItemFlags::USED | ItemFlags::USE);
    blood.driver = IDR_FDEMONBLOOD;
    assert!(world.map.set_item_map(&mut blood, 10, 10));
    world.add_item(blood);
    let mut container = item(9, ItemFlags::USED);
    container.template_id = 0x0100004B;
    container.driver_data = vec![2];
    container.sprite = 100;
    container.carried_by = Some(CharacterId(1));
    world.add_item(container);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_FDEMONBLOOD,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        8,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::FdemonBloodFilled { amount: 3, .. }
    ));
    assert!(!world.items.contains_key(&ItemId(7)));
    assert_eq!(world.map.tile(10, 10).unwrap().item, 0);
    let container = &world.items[&ItemId(9)];
    assert_eq!(container.driver_data[0], 3);
    assert_eq!(container.sprite, 101);
    assert_eq!(
        container.description,
        "A container holding 3 parts golem blood."
    );

    let mut blood = item(11, ItemFlags::USED | ItemFlags::USE);
    blood.driver = IDR_FDEMONBLOOD;
    world.add_item(blood);
    let mut flask = item(12, ItemFlags::USED);
    flask.driver = IDR_FLASK;
    flask.carried_by = Some(CharacterId(1));
    world.add_item(flask);
    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .cursor_item = Some(ItemId(12));

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_FDEMONBLOOD,
            item_id: ItemId(11),
            character_id: CharacterId(1),
            spec: 0,
        },
        8,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::FdemonBloodDestroyedFlask { .. }
    ));
    assert!(!world.items.contains_key(&ItemId(12)));
    assert_eq!(world.characters[&CharacterId(1)].cursor_item, None);
    assert_eq!(world.items[&ItemId(11)].sprite, 14348);
}

#[test]
fn world_places_and_ages_area18_bone_bridge_segments() {
    let mut world = World::default();
    let mut character = character(1);
    character.cursor_item = Some(ItemId(8));
    assert!(world.spawn_character(character, 10, 10));

    let mut bridge_base = item(7, ItemFlags::USED | ItemFlags::USE);
    bridge_base.driver = IDR_BONEBRIDGE;
    bridge_base.x = 11;
    bridge_base.y = 10;
    world.map.tile_mut(11, 10).unwrap().item = 7;
    world.map.set_flags(12, 10, MapFlags::MOVEBLOCK);

    let mut bone = item(8, ItemFlags::USED | ItemFlags::TAKE | ItemFlags::USE);
    bone.driver = IDR_BONEBRIDGE;
    bone.template_id = IID_AREA18_BONE;
    bone.carried_by = Some(CharacterId(1));
    bone.driver_data = vec![5];
    world.add_item(bridge_base);
    world.add_item(bone);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_BONEBRIDGE,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        18,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::BoneBridgePlace {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(8),
        }
    );
    let tile = world.map.tile(12, 10).unwrap();
    assert_eq!(tile.item, 8);
    assert!(!tile.flags.contains(MapFlags::MOVEBLOCK));
    let bone = world.items.get(&ItemId(8)).unwrap();
    assert_eq!((bone.x, bone.y), (12, 10));
    assert_eq!(bone.carried_by, None);
    assert!(!bone.flags.contains(ItemFlags::TAKE));
    assert_eq!(bone.driver_data[1], 1);
    assert_eq!(bone.sprite, 13035);
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().cursor_item,
        None
    );
    assert_eq!(world.timers.used_timers(), 1);

    world.add_character(timer_callback_character());
    let outcome = world.execute_item_driver_request_with_context(
        ItemDriverRequest::Driver {
            driver: IDR_BONEBRIDGE,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        },
        18,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::BoneBridgeTimerTick { item_id: ItemId(8) }
    );
    assert!(world
        .map
        .tile(12, 10)
        .unwrap()
        .flags
        .contains(MapFlags::MOVEBLOCK));
    let bone = world.items.get(&ItemId(8)).unwrap();
    assert_eq!(bone.driver_data[1], 2);
    assert_eq!(bone.sprite, 13036);
    assert_eq!(world.timers.used_timers(), 2);
}

#[test]
fn world_adds_and_removes_bones_on_a_carried_area18_bridge() {
    let mut world = World::default();
    let mut character = character(1);
    character.cursor_item = Some(ItemId(9));
    assert!(world.spawn_character(character, 10, 10));

    // A carried, partially-built bridge (`drdata[0] == 3`, not yet placed).
    let mut bridge = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::TAKE);
    bridge.driver = IDR_BONEBRIDGE;
    bridge.template_id = IID_AREA18_BONE;
    bridge.carried_by = Some(CharacterId(1));
    bridge.driver_data = vec![3, 0];
    world.add_item(bridge);

    // A single bone on the cursor (`drdata[0] == 1`).
    let mut cursor_bone = item(9, ItemFlags::USED | ItemFlags::TAKE | ItemFlags::USE);
    cursor_bone.driver = IDR_BONEBRIDGE;
    cursor_bone.template_id = IID_AREA18_BONE;
    cursor_bone.carried_by = Some(CharacterId(1));
    cursor_bone.driver_data = vec![1, 0];
    world.add_item(cursor_bone);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_BONEBRIDGE,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        18,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::BoneBridgeAddBone {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(9),
        }
    );
    let bridge = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(bridge.driver_data[0], 4);
    assert_eq!(bridge.sprite, 13034);
    // The single cursor bone was exhausted and destroyed, freeing the cursor.
    assert!(!world.items.contains_key(&ItemId(9)));
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().cursor_item,
        None
    );

    // Now pull one bone back out with an empty cursor.
    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_BONEBRIDGE,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        18,
    );
    assert_eq!(
        outcome,
        ItemDriverOutcome::BoneBridgeRemoveBone {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
    let bridge = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(bridge.driver_data[0], 3);
    assert_eq!(bridge.sprite, 13033);
}

#[test]
fn world_opens_and_restores_area18_bone_walls_like_c() {
    let mut world = World::default();
    assert!(world.spawn_character(character(1), 10, 10));
    world.add_character(timer_callback_character());

    for (id, x) in [(7_u32, 11_usize), (8, 12)] {
        let mut wall = item(
            id,
            ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK | ItemFlags::SIGHTBLOCK,
        );
        wall.driver = IDR_BONEWALL;
        wall.x = x as u16;
        wall.y = 10;
        wall.sprite = 14000;
        world.map.tile_mut(x, 10).unwrap().item = id;
        world
            .map
            .tile_mut(x, 10)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK);
        world.add_item(wall);
    }

    let request = ItemDriverRequest::Driver {
        driver: IDR_BONEWALL,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };
    assert_eq!(
        world.execute_item_driver_request(request, 18),
        ItemDriverOutcome::BoneWallTick {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(world.items.get(&ItemId(7)).unwrap().driver_data[0], 1);
    assert_eq!(world.items.get(&ItemId(7)).unwrap().sprite, 14001);
    assert_eq!(world.timers.used_timers(), 2);

    world.items.get_mut(&ItemId(7)).unwrap().driver_data[0] = 5;
    let timer_request = ItemDriverRequest::Driver {
        driver: IDR_BONEWALL,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };
    let context = ItemDriverContext {
        timer_call: true,
        ..ItemDriverContext::default()
    };
    assert_eq!(
        world.execute_item_driver_request_with_context(timer_request, 18, &context),
        ItemDriverOutcome::BoneWallTick {
            item_id: ItemId(7),
            character_id: CharacterId(0),
        }
    );
    let tile = world.map.tile(11, 10).unwrap();
    assert_eq!(tile.item, 0);
    assert!(!tile.flags.contains(MapFlags::TMOVEBLOCK));
    assert!(!tile.flags.contains(MapFlags::TSIGHTBLOCK));
    let wall = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(wall.driver_data[0], 6);
    assert!(wall.flags.contains(ItemFlags::VOID));
    assert!(!wall.flags.contains(ItemFlags::USE));

    world.map.tile_mut(11, 10).unwrap().item = 99;
    assert_eq!(
        world.execute_item_driver_request_with_context(timer_request, 18, &context),
        ItemDriverOutcome::BoneWallTick {
            item_id: ItemId(7),
            character_id: CharacterId(0),
        }
    );
    assert_eq!(world.items.get(&ItemId(7)).unwrap().driver_data[0], 6);

    world.map.tile_mut(11, 10).unwrap().item = 0;
    assert_eq!(
        world.execute_item_driver_request_with_context(timer_request, 18, &context),
        ItemDriverOutcome::BoneWallTick {
            item_id: ItemId(7),
            character_id: CharacterId(0),
        }
    );
    let tile = world.map.tile(11, 10).unwrap();
    assert_eq!(tile.item, 7);
    assert!(tile.flags.contains(MapFlags::TMOVEBLOCK));
    assert!(tile.flags.contains(MapFlags::TSIGHTBLOCK));
    let wall = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(wall.driver_data[0], 0);
    assert_eq!(wall.sprite, 13996);
    assert!(wall.flags.contains(ItemFlags::USE));
    assert!(!wall.flags.contains(ItemFlags::VOID));
}

#[test]
fn world_blocks_teufel_arena_entry_with_overenhanced_equipment() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.sprite = 27;
    actor.level = 38;
    actor.inventory[0] = Some(ItemId(20));
    assert!(world.spawn_character(actor, 150, 220));

    let mut arena = item(8, ItemFlags::USED | ItemFlags::USE);
    arena.driver = crate::item_driver::IDR_TEUFELARENA;
    arena.driver_data = vec![1];
    world.add_item(arena);

    let mut worn = item(20, ItemFlags::WNHEAD);
    worn.name = "helmet".to_string();
    worn.carried_by = Some(CharacterId(1));
    worn.modifier_index[0] = CharacterValue::Attack as i16;
    worn.modifier_value[0] = 1;
    worn.modifier_index[1] = CharacterValue::Parry as i16;
    worn.modifier_value[1] = 1;
    world.add_item(worn);

    let outcome = world.execute_item_driver_request_with_context(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_TEUFELARENA,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        34,
        &ItemDriverContext {
            teufel_arena_roll: Some(0),
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::TeufelArenaEquipmentEnhanced {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((actor.x, actor.y), (150, 220));
    assert_eq!(
        world.drain_pending_system_texts(),
        vec![WorldSystemText {
            character_id: CharacterId(1),
            message:
                "You cannot enter while wearing your helmet. It has more than one enhancement."
                    .to_string(),
        }]
    );
}

#[test]
fn staffer2_mine_dig_clears_sightblock_then_opens_and_schedules_restore() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.endurance = 10 * POWERSCALE;
    world.add_character(player);
    let mut mine = item(8, ItemFlags::USED | ItemFlags::USE | ItemFlags::SIGHTBLOCK);
    mine.driver = IDR_STAFFER2;
    mine.driver_data = vec![2, 0, 0, 2];
    mine.sprite = 15072;
    assert!(world.map.set_item_map(&mut mine, 10, 10));
    world.add_item(mine);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_STAFFER2,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        29,
    );

    assert!(matches!(outcome, ItemDriverOutcome::StafferMineDig { .. }));
    assert!(!world
        .map
        .tile(10, 10)
        .unwrap()
        .flags
        .contains(MapFlags::TSIGHTBLOCK));
    assert!(!world
        .items
        .get(&ItemId(8))
        .unwrap()
        .flags
        .contains(ItemFlags::SIGHTBLOCK));

    for _ in 0..5 {
        world.execute_item_driver_request(
            ItemDriverRequest::Driver {
                driver: IDR_STAFFER2,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            29,
        );
    }
    let tile = world.map.tile(10, 10).unwrap();
    assert_eq!(tile.item, 0);
    assert!(!tile.flags.contains(MapFlags::TMOVEBLOCK));
    let item = world.items.get(&ItemId(8)).unwrap();
    assert_eq!(item.driver_data[3], 8);
    assert!(item.flags.contains(ItemFlags::VOID));
    assert!(!item.flags.contains(ItemFlags::USE));
}

#[test]
fn staffer2_block_move_reports_blocked_target() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.dir = Direction::Right as u8;
    world.add_character(player);
    let mut block = item(8, ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK);
    block.driver = IDR_STAFFER2;
    block.driver_data = vec![3];
    assert!(world.map.set_item_map(&mut block, 10, 10));
    world.add_item(block);
    world.map.tile_mut(11, 10).unwrap().ground_sprite = 30000;

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_STAFFER2,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        29,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::StafferBlockBlocked {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(world.map.tile(10, 10).unwrap().item, 8);
    assert_eq!(world.map.tile(11, 10).unwrap().item, 0);
}

#[test]
fn caligar_weight_move_reports_blocked_or_bad_floor_target() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.dir = Direction::Right as u8;
    world.add_character(player);
    let mut weight = item(8, ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK);
    weight.driver = IDR_CALIGAR;
    weight.driver_data = vec![4];
    assert!(world.map.set_item_map(&mut weight, 10, 10));
    world.add_item(weight);
    world.map.tile_mut(11, 10).unwrap().ground_sprite = 30000;

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_CALIGAR,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        36,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::CaligarWeightBlocked {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(world.map.tile(10, 10).unwrap().item, 8);
    assert_eq!(world.map.tile(11, 10).unwrap().item, 0);
}

#[test]
fn world_fdemon_cannon_zero_power_clears_active_sprite() {
    let mut world = World::default();
    let mut cannon = item(7, ItemFlags::USED);
    cannon.driver = IDR_FDEMONCANNON;
    cannon.x = 10;
    cannon.y = 10;
    cannon.sprite = 14211;
    cannon.driver_data = vec![0; 13];
    world.add_item(cannon);

    for (id, nr) in [(11, 1), (12, 2), (13, 3)] {
        let mut loader = item(id, ItemFlags::USED);
        loader.driver = IDR_FDEMONLOADER;
        loader.x = 8 + nr;
        loader.y = 12;
        loader.driver_data = vec![nr as u8, 0, 0];
        world.add_item(loader);
    }

    let outcome = world.execute_item_driver_timer_request(
        ItemDriverRequest::Driver {
            driver: IDR_FDEMONCANNON,
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
        ItemDriverOutcome::FdemonCannonPulse { .. }
    ));
    assert_eq!(world.items[&ItemId(7)].sprite & 1, 0);
    assert!(world.effects.is_empty());
}

#[test]
fn minewall_dig_stage_three_removes_sightblock() {
    let mut world = World::default();
    let mut wall = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::SIGHTBLOCK);
    wall.x = 10;
    wall.y = 10;
    wall.driver_data = vec![0, 0, 0, 3];
    world.add_item(wall);
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::TSIGHTBLOCK);

    let outcome = world.apply_item_driver_outcome(
        ItemDriverOutcome::MineWallDig {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            endurance_delta: 0,
            stage: 3,
            opened: false,
        },
        12,
    );

    assert!(matches!(outcome, ItemDriverOutcome::MineWallDig { .. }));
    assert!(!world
        .map
        .tile(10, 10)
        .unwrap()
        .flags
        .contains(MapFlags::TSIGHTBLOCK));
    assert!(!world.items[&ItemId(7)]
        .flags
        .contains(ItemFlags::SIGHTBLOCK));
}

#[test]
fn minewall_dig_stage_eight_opens_wall_and_schedules_collapse() {
    let mut world = World::default();
    let mut wall = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::SIGHTBLOCK);
    wall.x = 10;
    wall.y = 10;
    wall.driver = crate::item_driver::IDR_MINEWALL;
    wall.driver_data = vec![0, 0, 0, 8];
    world.add_item(wall);
    world.map.tile_mut(10, 10).unwrap().item = 7;
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK);

    let outcome = world.apply_item_driver_outcome(
        ItemDriverOutcome::MineWallDig {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            endurance_delta: 0,
            stage: 8,
            opened: true,
        },
        12,
    );

    assert!(matches!(outcome, ItemDriverOutcome::MineWallDig { .. }));
    let tile = world.map.tile(10, 10).unwrap();
    assert_eq!(tile.item, 0);
    assert!(!tile.flags.contains(MapFlags::TMOVEBLOCK));
    let wall = &world.items[&ItemId(7)];
    assert!(!wall.flags.contains(ItemFlags::USE));
    assert!(wall.flags.contains(ItemFlags::VOID));
    assert_eq!(world.timers.used_timers(), 1);
}

#[test]
fn minewall_collapse_restores_blockers_when_tile_is_free() {
    let mut world = World::default();
    let mut wall = item(7, ItemFlags::USED | ItemFlags::VOID);
    wall.x = 10;
    wall.y = 10;
    wall.sprite = 15078;
    wall.driver_data = vec![0, 0, 0, 8, 1];
    world.add_item(wall);

    let outcome = world.apply_item_driver_outcome(
        ItemDriverOutcome::MineWallCollapse {
            item_id: ItemId(7),
            schedule_after_ticks: 10,
        },
        12,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::MineWallCollapse { .. }
    ));
    let tile = world.map.tile(10, 10).unwrap();
    assert_eq!(tile.item, 7);
    assert!(tile
        .flags
        .contains(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK));
    let wall = &world.items[&ItemId(7)];
    assert_eq!(wall.sprite, 15070);
    assert_eq!(wall.driver_data[3], 0);
    assert!(wall.flags.contains(ItemFlags::USE | ItemFlags::SIGHTBLOCK));
    assert!(!wall.flags.contains(ItemFlags::VOID));
}
