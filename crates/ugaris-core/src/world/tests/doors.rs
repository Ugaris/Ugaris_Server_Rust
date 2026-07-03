use super::*;

#[test]
fn world_dlight_wrappers_mark_changed_indoor_tiles_dirty() {
    let mut world = World {
        tick: Tick(31),
        map: MapGrid::new(32, 32),
        ..World::default()
    };
    world.map.set_flags(10, 10, MapFlags::INDOORS);

    assert!(world.compute_dlight_at(10, 10));
    assert_eq!(world.map.tile(10, 10).unwrap().daylight, 63);
    assert_eq!(world.skip_x_sector(10, 10, 31), 0);

    let mut world = World {
        tick: Tick(37),
        map: MapGrid::new(32, 32),
        ..World::default()
    };
    for y in 8..=10 {
        for x in 8..=10 {
            world.map.set_flags(x, y, MapFlags::INDOORS);
        }
    }

    assert!(world.reset_dlight_around(10, 10));
    assert!(world.map.tile(10, 10).unwrap().daylight > 0);
    assert_eq!(world.skip_x_sector(10, 10, 37), 0);
}

#[test]
fn world_tracks_area3_onofflight_counts_and_gate_window() {
    let mut world = World::default();
    world.tick = Tick(100);
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    let mut light = item(7, ItemFlags::USED | ItemFlags::USE);
    light.driver = IDR_ONOFFLIGHT;
    light.driver_data = vec![1, 14, 0, 0, 0, 0, 1];
    light.modifier_index[0] = CharacterValue::Light as i16;
    light.modifier_value[0] = 14;
    light.sprite = 101;
    light.x = 10;
    light.y = 10;
    world.map.tile_mut(10, 10).unwrap().item = 7;
    world.add_character(character);
    world.add_item(light);

    let request = ItemDriverRequest::Driver {
        driver: IDR_ONOFFLIGHT,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };
    let off = world.execute_item_driver_request(request, 3);
    assert_eq!(
        off,
        ItemDriverOutcome::OnOffLightChanged {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            now_on: false,
            remaining_off: None,
            gates_opened: false,
        }
    );
    assert_eq!(world.area3_palace_lamps.switched_off_count, 1);
    assert_eq!(world.map.tile(10, 10).unwrap().light, 0);

    let on = world.execute_item_driver_request(request, 3);
    assert_eq!(
        on,
        ItemDriverOutcome::OnOffLightChanged {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            now_on: true,
            remaining_off: Some(0),
            gates_opened: true,
        }
    );
    assert_eq!(world.area3_palace_lamps.switched_on_count, 1);
    assert_eq!(
        world.area3_palace_lamps.keep_open_until_tick,
        100 + TICKS_PER_SECOND as u64 * 60 * 3
    );
    assert_eq!(world.timers.used_timers(), 1);
    assert_eq!(world.map.tile(10, 10).unwrap().light, 14);
}

#[test]
fn world_schedules_registered_area3_lamps_for_extinguish_when_gates_open() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    world.add_character(character);

    for id in [7, 9] {
        let mut light = item(id, ItemFlags::USED | ItemFlags::USE);
        light.driver = IDR_ONOFFLIGHT;
        light.driver_data = vec![0, 10, 0, 0, 0, 0, 1];
        light.x = 10 + id as u16;
        light.y = 10;
        world.add_item(light);
    }
    world.area3_palace_lamps.switched_off_count = 1;

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_ONOFFLIGHT,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        3,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::OnOffLightChanged {
            now_on: true,
            gates_opened: true,
            ..
        }
    ));
    assert_eq!(world.timers.used_timers(), 2);
}

#[test]
fn world_area3_palace_gate_opens_and_closes_from_keepopen_window() {
    let mut world = World::default();
    world.tick = Tick(100);
    world.area3_palace_lamps.keep_open_until_tick = 200;
    let mut gate = item(
        7,
        ItemFlags::USED | ItemFlags::MOVEBLOCK | ItemFlags::SIGHTBLOCK | ItemFlags::DOOR,
    );
    gate.driver = IDR_PALACEGATE;
    gate.driver_data = vec![0];
    gate.sprite = 500;
    gate.x = 10;
    gate.y = 10;
    world.map.tile_mut(10, 10).unwrap().item = 7;
    world.map.tile_mut(10, 10).unwrap().flags =
        MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK | MapFlags::DOOR;
    world.add_item(gate);

    assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));
    world.advance();
    let open_outcome = world.process_due_timers(3).remove(0);

    assert_eq!(
        open_outcome,
        ItemDriverOutcome::PalaceGateTick {
            item_id: ItemId(7),
            opened: true,
            closed: false,
            blocked: false,
        }
    );
    let gate = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(gate.driver_data[0], 1);
    assert_eq!(gate.sprite, 501);
    assert!(!gate
        .flags
        .intersects(ItemFlags::MOVEBLOCK | ItemFlags::SIGHTBLOCK | ItemFlags::DOOR));
    assert!(!world
        .map
        .tile(10, 10)
        .unwrap()
        .flags
        .intersects(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK | MapFlags::DOOR));

    world.tick = Tick(250);
    assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));
    world.advance();
    let close_outcome = world.process_due_timers(3).remove(0);

    assert_eq!(
        close_outcome,
        ItemDriverOutcome::PalaceGateTick {
            item_id: ItemId(7),
            opened: false,
            closed: true,
            blocked: false,
        }
    );
    let gate = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(gate.driver_data[0], 0);
    assert_eq!(gate.sprite, 500);
    assert!(gate.flags.contains(ItemFlags::MOVEBLOCK));
    assert!(world
        .map
        .tile(10, 10)
        .unwrap()
        .flags
        .contains(MapFlags::TMOVEBLOCK));
}

#[test]
fn world_area3_palace_gate_refuses_to_close_when_blocked() {
    let mut world = World::default();
    world.tick = Tick(250);
    let mut gate = item(7, ItemFlags::USED);
    gate.driver = IDR_PALACEGATE;
    gate.driver_data = vec![1];
    gate.driver_data.resize(40, 0);
    gate.driver_data[30..38].copy_from_slice(&ItemFlags::MOVEBLOCK.bits().to_le_bytes());
    gate.sprite = 501;
    gate.x = 10;
    gate.y = 10;
    world.map.tile_mut(10, 10).unwrap().item = 7;
    world.map.tile_mut(10, 10).unwrap().flags = MapFlags::MOVEBLOCK;
    world.add_item(gate);

    assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));
    world.advance();
    let outcome = world.process_due_timers(3).remove(0);

    assert_eq!(
        outcome,
        ItemDriverOutcome::PalaceGateTick {
            item_id: ItemId(7),
            opened: false,
            closed: false,
            blocked: true,
        }
    );
    let gate = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(gate.driver_data[0], 1);
    assert_eq!(gate.sprite, 501);
    assert_eq!(world.timers.used_timers(), 1);
}

#[test]
fn world_edemon_gate_timer_finds_stale_slot_and_reschedules() {
    let mut world = World::default();
    let mut gate = item(7, ItemFlags::USED);
    gate.driver = IDR_EDEMONGATE;
    gate.driver_data = vec![0];
    world.add_item(gate);
    assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));
    world.advance();

    let outcomes = world.process_due_timers(6);

    assert_eq!(outcomes.len(), 1);
    assert_eq!(
        outcomes[0],
        ItemDriverOutcome::EdemonGateSpawn {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            template: "edemon2s",
            slot: 0,
            x: 62,
            y: 157,
            schedule_after_ticks: TICKS_PER_SECOND * 10,
        }
    );
    assert_eq!(world.timers.used_timers(), 1);
}

#[test]
fn world_fdemon_gate_slots_use_legacy_character_serial_guards() {
    let mut world = World::default();
    world.add_character(character(0));
    let mut existing = character(1);
    existing.serial = 55;
    world.add_character(existing);
    let mut gate = item(7, ItemFlags::USED);
    gate.driver = IDR_FDEMONGATE;
    gate.x = 10;
    gate.y = 20;
    gate.driver_data = vec![2, 10, 0, 0, 1, 0, 55, 0];
    world.add_item(gate);

    let outcome = world.execute_item_driver_request_with_context(
        ItemDriverRequest::Driver {
            driver: IDR_FDEMONGATE,
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
        ItemDriverOutcome::FdemonGateSpawn { slot: 1, .. }
    ));
    assert!(world.apply_fdemon_gate_spawn_result(ItemId(7), 1, CharacterId(2), 77));
    let gate = &world.items[&ItemId(7)];
    assert_eq!(&gate.driver_data[8..12], &[2, 0, 77, 0]);

    world.characters.get_mut(&CharacterId(1)).unwrap().serial = 56;
    let outcome = world.execute_item_driver_request_with_context(
        ItemDriverRequest::Driver {
            driver: IDR_FDEMONGATE,
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
        ItemDriverOutcome::FdemonGateSpawn { slot: 0, .. }
    ));
}

#[test]
fn world_executes_teleport_door_to_exact_opposite_side() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 9;
    character.y = 10;
    world.map.tile_mut(9, 10).unwrap().character = 1;
    world
        .map
        .tile_mut(9, 10)
        .unwrap()
        .flags
        .insert(MapFlags::TMOVEBLOCK);
    world.add_character(character);
    let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
    item.driver = crate::item_driver::IDR_TELE_DOOR;
    item.x = 10;
    item.y = 10;
    world.add_item(item);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_TELE_DOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(outcome, ItemDriverOutcome::TeleportDoor { .. }));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((character.x, character.y), (11, 10));
    assert_eq!(world.map.tile(9, 10).unwrap().character, 0);
    assert_eq!(world.map.tile(11, 10).unwrap().character, 1);
}

#[test]
fn world_applies_pent_boss_door_teleport_and_reverses_cardinal_facing() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 11;
    character.y = 10;
    character.dir = Direction::Right as u8;
    world.map.tile_mut(11, 10).unwrap().character = 1;
    world
        .map
        .tile_mut(11, 10)
        .unwrap()
        .flags
        .insert(MapFlags::TMOVEBLOCK);
    world.add_character(character);

    let outcome = world.apply_item_driver_outcome(
        ItemDriverOutcome::PentBossDoor {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            x: 9,
            y: 10,
        },
        4,
    );

    assert!(matches!(outcome, ItemDriverOutcome::PentBossDoor { .. }));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((character.x, character.y), (9, 10));
    assert_eq!(character.dir, Direction::Left as u8);
    assert_eq!(world.map.tile(11, 10).unwrap().character, 0);
    assert_eq!(world.map.tile(9, 10).unwrap().character, 1);
}

#[test]
fn world_executes_freakdoor_partner_teleport_and_caches_partner() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    character.tox = 11;
    character.toy = 10;
    world.map.tile_mut(10, 10).unwrap().character = 1;
    world.add_character(character);

    let door_flags = ItemFlags::USED
        | ItemFlags::USE
        | ItemFlags::DOOR
        | ItemFlags::MOVEBLOCK
        | ItemFlags::SIGHTBLOCK;
    let mut first = item(7, door_flags);
    first.driver = crate::item_driver::IDR_FREAKDOOR;
    first.x = 10;
    first.y = 10;
    first.driver_data = vec![0; 16];
    first.driver_data[8] = 3;
    world.map.tile_mut(10, 10).unwrap().item = 7;
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK | MapFlags::DOOR);
    world.add_item(first);

    let mut second = item(8, door_flags);
    second.driver = crate::item_driver::IDR_FREAKDOOR;
    second.x = 20;
    second.y = 20;
    second.driver_data = vec![0; 16];
    second.driver_data[8] = 3;
    world.map.tile_mut(20, 20).unwrap().item = 8;
    world
        .map
        .tile_mut(20, 20)
        .unwrap()
        .flags
        .insert(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK | MapFlags::DOOR);
    world.add_item(second);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_FREAKDOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(outcome, ItemDriverOutcome::FreakDoorUse { .. }));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((character.x, character.y), (20, 20));
    assert_eq!((character.tox, character.toy), (21, 20));
    assert_eq!(world.map.tile(10, 10).unwrap().character, 0);
    assert_eq!(world.map.tile(20, 20).unwrap().character, 1);
    let first = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(read_u32_le_at(&first.driver_data, 10), 8);
    let second = world.items.get(&ItemId(8)).unwrap();
    assert!(door_open_state(second));
}

#[test]
fn world_executes_door_driver_open_and_close() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    actor.x = 10;
    actor.y = 10;
    world.add_character(actor);
    let mut door = item(
        7,
        ItemFlags::USED
            | ItemFlags::USE
            | ItemFlags::MOVEBLOCK
            | ItemFlags::SIGHTBLOCK
            | ItemFlags::SOUNDBLOCK
            | ItemFlags::DOOR,
    );
    door.driver = crate::item_driver::IDR_DOOR;
    door.sprite = 100;
    assert!(world.map.set_item_map(&mut door, 10, 10));
    world.add_item(door);

    let request = ItemDriverRequest::Driver {
        driver: crate::item_driver::IDR_DOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    let outcome = world.execute_item_driver_request(request, 1);
    assert_eq!(
        outcome,
        ItemDriverOutcome::DoorToggle {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
    let door = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(door.driver_data[0], 1);
    assert_eq!(door.sprite, 101);
    assert!(!door.flags.intersects(
        ItemFlags::MOVEBLOCK | ItemFlags::SIGHTBLOCK | ItemFlags::SOUNDBLOCK | ItemFlags::DOOR
    ));
    let tile = world.map.tile(10, 10).unwrap();
    assert!(!tile.flags.intersects(
        MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK | MapFlags::TSOUNDBLOCK | MapFlags::DOOR
    ));
    let sounds = world.drain_pending_sound_specials();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].character_id, CharacterId(1));
    assert_eq!(sounds[0].special.special_type, 3);
    assert_eq!(sounds[0].special.opt1, 0);
    assert_eq!(sounds[0].special.opt2, 0);

    let outcome = world.execute_item_driver_request(request, 1);
    assert!(matches!(outcome, ItemDriverOutcome::DoorToggle { .. }));
    let door = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(door.driver_data[0], 0);
    assert_eq!(door.sprite, 100);
    assert!(door.flags.contains(ItemFlags::MOVEBLOCK));
    assert!(door.flags.contains(ItemFlags::SIGHTBLOCK));
    assert!(door.flags.contains(ItemFlags::SOUNDBLOCK));
    assert!(door.flags.contains(ItemFlags::DOOR));
    let tile = world.map.tile(10, 10).unwrap();
    assert!(tile.flags.contains(MapFlags::TMOVEBLOCK));
    assert!(tile.flags.contains(MapFlags::TSIGHTBLOCK));
    assert!(tile.flags.contains(MapFlags::TSOUNDBLOCK));
    assert!(tile.flags.contains(MapFlags::DOOR));
    let sounds = world.drain_pending_sound_specials();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].character_id, CharacterId(1));
    assert_eq!(sounds[0].special.special_type, 3);
}

#[test]
fn world_executes_area17_pick_door_with_legacy_timer() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    actor.x = 8;
    actor.y = 8;
    world.add_character(actor);
    let mut nearby_npc = character(2);
    nearby_npc.x = 20;
    nearby_npc.y = 8;
    world.add_character(nearby_npc);
    let mut distant_npc = character(3);
    distant_npc.x = 60;
    distant_npc.y = 8;
    world.add_character(distant_npc);
    let mut door = item(
        7,
        ItemFlags::USED
            | ItemFlags::USE
            | ItemFlags::MOVEBLOCK
            | ItemFlags::SIGHTBLOCK
            | ItemFlags::SOUNDBLOCK
            | ItemFlags::DOOR,
    );
    door.driver = crate::item_driver::IDR_PICKDOOR;
    door.sprite = 100;
    assert!(world.map.set_item_map(&mut door, 10, 10));
    world.add_item(door);

    let request = ItemDriverRequest::Driver {
        driver: crate::item_driver::IDR_PICKDOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        world.execute_item_driver_request(request, 17),
        ItemDriverOutcome::PickDoorLocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    assert_eq!(
        world.execute_item_driver_request_with_context(
            request,
            17,
            &ItemDriverContext {
                has_area17_cursor_lockpick: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::PickDoorToggle {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            picked_lock: true,
        }
    );
    let door = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(door.driver_data[0], 1);
    assert_eq!(door.sprite, 101);
    assert!(!door.flags.intersects(
        ItemFlags::MOVEBLOCK | ItemFlags::SIGHTBLOCK | ItemFlags::SOUNDBLOCK | ItemFlags::DOOR
    ));
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
    assert!(world
        .characters
        .get(&CharacterId(3))
        .unwrap()
        .driver_messages
        .is_empty());

    world.tick = Tick(TICKS_PER_SECOND * 20);
    let outcomes = world.process_due_timers(17);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(
        outcomes[0],
        ItemDriverOutcome::PickDoorToggle {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            picked_lock: false,
        }
    );
    let door = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(door.driver_data[0], 0);
    assert_eq!(door.sprite, 100);
    assert!(door.flags.contains(ItemFlags::MOVEBLOCK));
    assert!(door.flags.contains(ItemFlags::SIGHTBLOCK));
    assert!(door.flags.contains(ItemFlags::SOUNDBLOCK));
    assert!(door.flags.contains(ItemFlags::DOOR));
    assert_eq!(
        world
            .characters
            .get(&CharacterId(2))
            .unwrap()
            .driver_messages
            .len(),
        1
    );
}

#[test]
fn world_executes_area12_mine_door_target_teleport() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    assert!(world.spawn_character(actor, 10, 10));

    let mut source = item(7, ItemFlags::USED | ItemFlags::USE);
    source.driver = crate::item_driver::IDR_MINEDOOR;
    source.x = 20;
    source.y = 20;
    source.driver_data = vec![4, 0, 7, 1];
    world.add_item(source);

    let mut target = item(8, ItemFlags::USED | ItemFlags::USE);
    target.driver = crate::item_driver::IDR_MINEDOOR;
    target.x = 30;
    target.y = 40;
    target.driver_data = vec![4, 1, 5, 0];
    world.add_item(target);

    let request = ItemDriverRequest::Driver {
        driver: crate::item_driver::IDR_MINEDOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        world.execute_item_driver_request(request, 12),
        ItemDriverOutcome::MineDoorTeleport {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            target_x: 31,
            target_y: 40,
            fallback_x: 230,
            fallback_y: 240,
        }
    );
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((actor.x, actor.y), (31, 40));
}

#[test]
fn world_mine_door_timer_opens_source_and_closes_previous_source() {
    let mut world = World::default();
    world.legacy_random_seed = 3;

    for (x, y) in mine_door_neighbor_points(20, 20)
        .into_iter()
        .chain(mine_door_neighbor_points(30, 30))
    {
        world
            .map
            .tile_mut(x, y)
            .unwrap()
            .flags
            .insert(MapFlags::SIGHTBLOCK | MapFlags::MOVEBLOCK);
    }

    let mut old_source = item(7, ItemFlags::USED | ItemFlags::USE);
    old_source.driver = crate::item_driver::IDR_MINEDOOR;
    old_source.x = 20;
    old_source.y = 20;
    old_source.sprite = 20124;
    old_source.driver_data = vec![4, 0, 7, 1];
    world.add_item(old_source);

    let mut new_source = item(8, ItemFlags::USED);
    new_source.driver = crate::item_driver::IDR_MINEDOOR;
    new_source.x = 30;
    new_source.y = 30;
    new_source.sprite = 15000;
    new_source.driver_data = vec![4, 0, 5, 0];
    world.add_item(new_source);

    let outcome = world
        .apply_item_driver_outcome(ItemDriverOutcome::MineDoorTimer { item_id: ItemId(8) }, 12);

    assert_eq!(
        outcome,
        ItemDriverOutcome::MineDoorTimer { item_id: ItemId(8) }
    );
    let old_source = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(old_source.sprite, 15000);
    assert!(!old_source.flags.contains(ItemFlags::USE));
    assert_eq!(old_source.driver_data[3], 0);
    let new_source = world.items.get(&ItemId(8)).unwrap();
    assert_eq!(new_source.sprite, 20122);
    assert!(new_source.flags.contains(ItemFlags::USE));
    assert_eq!(new_source.driver_data[3], 1);
}

#[test]
fn world_mine_door_timer_keeps_dug_out_doors_closed() {
    let mut world = World::default();
    let mut door = item(8, ItemFlags::USED);
    door.driver = crate::item_driver::IDR_MINEDOOR;
    door.x = 30;
    door.y = 30;
    door.sprite = 15000;
    door.driver_data = vec![4, 0, 5, 0];
    world.add_item(door);

    let outcome = world
        .apply_item_driver_outcome(ItemDriverOutcome::MineDoorTimer { item_id: ItemId(8) }, 12);

    assert_eq!(
        outcome,
        ItemDriverOutcome::MineDoorTimer { item_id: ItemId(8) }
    );
    let door = world.items.get(&ItemId(8)).unwrap();
    assert_eq!(door.sprite, 15000);
    assert!(!door.flags.contains(ItemFlags::USE));
    assert_eq!(door.driver_data[3], 0);
}

#[test]
fn schedule_existing_light_timers_includes_mine_doors() {
    let mut world = World::default();
    let mut door = item(8, ItemFlags::USED);
    door.driver = crate::item_driver::IDR_MINEDOOR;
    door.driver_data = vec![4, 0, 5, 0];
    world.add_item(door);

    assert_eq!(world.schedule_existing_light_timers(), 1);
    world.tick.0 = 1;

    assert_eq!(
        world.process_due_timers(12),
        vec![ItemDriverOutcome::MineDoorTimer { item_id: ItemId(8) }]
    );
}

#[test]
fn world_executes_teufel_door_and_reverses_cardinal_facing() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.sprite = 27;
    actor.dir = Direction::Right as u8;
    assert!(world.spawn_character(actor, 9, 10));
    let mut door = item(8, ItemFlags::USED | ItemFlags::USE);
    door.driver = crate::item_driver::IDR_TEUFELDOOR;
    door.driver_data = vec![0];
    world.map.set_item_map(&mut door, 10, 10);
    world.add_item(door);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_TEUFELDOOR,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        34,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::TeufelDoor {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            x: 11,
            y: 10,
        }
    );
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((actor.x, actor.y), (11, 10));
    assert_eq!(actor.dir, Direction::Left as u8);
}

#[test]
fn world_does_not_close_open_door_when_blocked() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::DOOR);
    door.driver = crate::item_driver::IDR_DOOR;
    door.sprite = 101;
    door.driver_data = vec![1];
    assert!(world.map.set_item_map(&mut door, 10, 10));
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::TMOVEBLOCK);
    world.add_item(door);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_DOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert_eq!(outcome, ItemDriverOutcome::Noop);
    let door = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(door.driver_data[0], 1);
    assert_eq!(door.sprite, 101);
}

#[test]
fn world_auto_closes_opened_door_from_timer() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    actor.x = 10;
    actor.y = 10;
    world.add_character(actor);
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::DOOR);
    door.driver = crate::item_driver::IDR_DOOR;
    door.sprite = 100;
    assert!(world.map.set_item_map(&mut door, 10, 10));
    world.add_item(door);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_DOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(outcome, ItemDriverOutcome::DoorToggle { .. }));
    let sounds = world.drain_pending_sound_specials();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].special.special_type, 3);
    assert_eq!(world.items.get(&ItemId(7)).unwrap().driver_data[39], 1);
    assert_eq!(world.timers.used_timers(), 1);

    for _ in 0..(TICKS_PER_SECOND * 10) {
        world.advance();
    }
    let outcomes = world.process_due_timers(1);

    assert_eq!(
        outcomes,
        vec![ItemDriverOutcome::DoorToggle {
            item_id: ItemId(7),
            character_id: CharacterId(0),
        }]
    );
    let door = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(door.driver_data[0], 0);
    assert_eq!(door.driver_data[39], 0);
    assert_eq!(door.sprite, 100);
    let sounds = world.drain_pending_sound_specials();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].special.special_type, 2);
}

#[test]
fn world_respects_no_auto_close_door_flag() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::DOOR);
    door.driver = crate::item_driver::IDR_DOOR;
    door.driver_data.resize(6, 0);
    door.driver_data[5] = 1;
    assert!(world.map.set_item_map(&mut door, 10, 10));
    world.add_item(door);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_DOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(outcome, ItemDriverOutcome::DoorToggle { .. }));
    let door = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(door.driver_data[0], 1);
    assert_eq!(door.driver_data[39], 1);
    assert_eq!(world.timers.used_timers(), 0);
}

#[test]
fn world_retries_blocked_door_timer_close() {
    let mut world = World::default();
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::DOOR);
    door.driver = crate::item_driver::IDR_DOOR;
    door.sprite = 101;
    door.driver_data.resize(40, 0);
    door.driver_data[0] = 1;
    door.driver_data[39] = 1;
    assert!(world.map.set_item_map(&mut door, 10, 10));
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::TMOVEBLOCK);
    world.add_item(door);
    assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));

    world.advance();
    assert_eq!(world.process_due_timers(1), vec![ItemDriverOutcome::Noop]);
    let door = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(door.driver_data[0], 1);
    assert_eq!(door.driver_data[39], 1);
    assert_eq!(world.timers.used_timers(), 1);

    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .remove(MapFlags::TMOVEBLOCK);
    for _ in 0..(TICKS_PER_SECOND * 5) {
        world.advance();
    }
    let outcomes = world.process_due_timers(1);

    assert!(matches!(
        outcomes.as_slice(),
        [ItemDriverOutcome::DoorToggle { .. }]
    ));
    let door = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(door.driver_data[0], 0);
    assert_eq!(door.driver_data[39], 0);
    assert_eq!(door.sprite, 100);
}

#[test]
fn world_shifts_extended_door_foreground_sprites() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::DOOR);
    door.driver = crate::item_driver::IDR_DOOR;
    door.sprite = 100;
    door.driver_data.resize(8, 0);
    door.driver_data[7] = 1;
    assert!(world.map.set_item_map(&mut door, 10, 10));
    for (x, y, sprite) in [(11, 10, 20), (9, 10, 21), (10, 11, 22), (10, 9, 23)] {
        world.map.tile_mut(x, y).unwrap().foreground_sprite = sprite;
    }
    world.add_item(door);

    let request = ItemDriverRequest::Driver {
        driver: crate::item_driver::IDR_DOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert!(matches!(
        world.execute_item_driver_request(request, 1),
        ItemDriverOutcome::DoorToggle { .. }
    ));
    assert_eq!(world.map.tile(11, 10).unwrap().foreground_sprite, 21);
    assert_eq!(world.map.tile(9, 10).unwrap().foreground_sprite, 22);
    assert_eq!(world.map.tile(10, 11).unwrap().foreground_sprite, 23);
    assert_eq!(world.map.tile(10, 9).unwrap().foreground_sprite, 24);

    assert!(matches!(
        world.execute_item_driver_request(request, 1),
        ItemDriverOutcome::DoorToggle { .. }
    ));
    assert_eq!(world.map.tile(11, 10).unwrap().foreground_sprite, 20);
    assert_eq!(world.map.tile(9, 10).unwrap().foreground_sprite, 21);
    assert_eq!(world.map.tile(10, 11).unwrap().foreground_sprite, 22);
    assert_eq!(world.map.tile(10, 9).unwrap().foreground_sprite, 23);
}

#[test]
fn world_executes_double_door_and_syncs_adjacent_state() {
    let mut world = World::default();
    world.add_character(character(1));
    let closed_flags = ItemFlags::USED
        | ItemFlags::USE
        | ItemFlags::MOVEBLOCK
        | ItemFlags::SIGHTBLOCK
        | ItemFlags::SOUNDBLOCK
        | ItemFlags::DOOR;
    let mut primary = item(7, closed_flags);
    primary.driver = crate::item_driver::IDR_DOUBLE_DOOR;
    primary.sprite = 100;
    assert!(world.map.set_item_map(&mut primary, 10, 10));
    world.add_item(primary);

    let mut adjacent = item(8, closed_flags);
    adjacent.driver = crate::item_driver::IDR_DOOR;
    adjacent.sprite = 200;
    assert!(world.map.set_item_map(&mut adjacent, 10, 11));
    world.add_item(adjacent);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_DOUBLE_DOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::DoubleDoorToggle {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
    let primary = world.items.get(&ItemId(7)).unwrap();
    let adjacent = world.items.get(&ItemId(8)).unwrap();
    assert_eq!(primary.driver_data[0], 1);
    assert_eq!(adjacent.driver_data[0], 1);
    assert_eq!(primary.sprite, 101);
    assert_eq!(adjacent.sprite, 201);
    assert!(!world
        .map
        .tile(10, 10)
        .unwrap()
        .flags
        .contains(MapFlags::TMOVEBLOCK));
    assert!(!world
        .map
        .tile(10, 11)
        .unwrap()
        .flags
        .contains(MapFlags::TMOVEBLOCK));
}

#[test]
fn world_applies_mine_gateway_key_final_assembly() {
    let mut world = World::default();
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(7));
    character.cursor_item = Some(ItemId(8));
    world.add_character(character);

    let mut base = item(7, ItemFlags::USED | ItemFlags::USE);
    base.driver = crate::item_driver::IDR_MINEGATEWAYKEY;
    base.carried_by = Some(CharacterId(1));
    base.driver_data = vec![7];
    world.add_item(base);
    let mut cursor = item(8, ItemFlags::USED | ItemFlags::USE);
    cursor.driver = crate::item_driver::IDR_MINEGATEWAYKEY;
    cursor.carried_by = Some(CharacterId(1));
    cursor.driver_data = vec![8];
    world.add_item(cursor);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_MINEGATEWAYKEY,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::MineGatewayKeyAssemble { .. }
    ));
    let base = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(base.driver_data[0], 15);
    assert_eq!(base.sprite, 52200);
    assert_eq!(base.template_id, 0x01000098);
    assert_eq!(base.name, "Mine gateway key");
    assert_eq!(base.description, "A fully assembled key.");
    assert!(!base.flags.contains(ItemFlags::USE));
    assert!(!world.items.contains_key(&ItemId(8)));
    assert_eq!(world.characters[&CharacterId(1)].cursor_item, None);
}

#[test]
fn world_mine_gateway_requires_assembled_key_and_teleports_same_area() {
    let mut world = World::default();
    let mut character = character(1);
    character.flags.insert(CharacterFlags::PLAYER);
    character.inventory[30] = Some(ItemId(8));
    world.spawn_character(character, 10, 10);

    let mut key = item(8, ItemFlags::USED);
    key.template_id = crate::item_driver::IID_MINEGATEWAY;
    key.carried_by = Some(CharacterId(1));
    world.add_item(key);

    let mut gateway = item(7, ItemFlags::USED | ItemFlags::USE);
    gateway.driver = crate::item_driver::IDR_MINEGATEWAY;
    gateway.driver_data = vec![20, 0, 21, 0, 12, 0];
    world.add_item(gateway);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_MINEGATEWAY,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        12,
    );

    assert!(matches!(outcome, ItemDriverOutcome::MineGateway { .. }));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((character.x, character.y), (20, 21));
}

#[test]
fn world_mine_key_door_consumes_gold_and_teleports_to_first_free_room() {
    let mut world = World::default();
    let mut character = character(1);
    character.cursor_item = Some(ItemId(8));
    world.spawn_character(character, 10, 10);

    let mut gold = item(8, ItemFlags::USED | ItemFlags::TAKE);
    gold.driver = crate::item_driver::IDR_ENHANCE;
    gold.carried_by = Some(CharacterId(1));
    gold.driver_data = vec![2, 0xD0, 0x07, 0, 0];
    world.add_item(gold);

    let mut door = item(7, ItemFlags::USED | ItemFlags::USE);
    door.driver = crate::item_driver::IDR_MINEKEYDOOR;
    door.driver_data = vec![3];
    world.add_item(door);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_MINEKEYDOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        12,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::MineKeyDoor { golem_nr: 3, .. }
    ));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((character.x, character.y), (3, 234));
    assert_eq!(character.cursor_item, None);
    assert!(!world.items.contains_key(&ItemId(8)));
}

#[test]
fn caligar_weight_door_requires_lock_weights_from_south() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    assert!(world.spawn_character(player, 10, 11));
    let mut door = item(8, ItemFlags::USED | ItemFlags::USE);
    door.driver = IDR_CALIGAR;
    door.driver_data = vec![3];
    assert!(world.map.set_item_map(&mut door, 10, 10));
    world.add_item(door);

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
        ItemDriverOutcome::CaligarWeightDoorLocked {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(world.characters.get(&CharacterId(1)).unwrap().x, 10);
    assert_eq!(world.characters.get(&CharacterId(1)).unwrap().y, 11);
}

#[test]
fn caligar_weight_door_teleports_to_opposite_side_and_reverses_facing() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.dir = Direction::Down as u8;
    assert!(world.spawn_character(player, 10, 9));
    let mut door = item(8, ItemFlags::USED | ItemFlags::USE);
    door.driver = IDR_CALIGAR;
    door.driver_data = vec![3];
    assert!(world.map.set_item_map(&mut door, 10, 10));
    world.add_item(door);

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
        ItemDriverOutcome::CaligarWeightDoor {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
    let player = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((player.x, player.y), (10, 11));
    assert_eq!(player.dir, Direction::Up as u8);
    assert_eq!(world.map.tile(10, 9).unwrap().character, 0);
    assert_eq!(world.map.tile(10, 11).unwrap().character, 1);
}

#[test]
fn caligar_weight_door_reports_busy_target() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    assert!(world.spawn_character(player, 10, 9));
    assert!(world.spawn_character(character(2), 10, 11));
    let mut door = item(8, ItemFlags::USED | ItemFlags::USE);
    door.driver = IDR_CALIGAR;
    door.driver_data = vec![3];
    assert!(world.map.set_item_map(&mut door, 10, 10));
    world.add_item(door);

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
        ItemDriverOutcome::CaligarWeightDoorBusy {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(world.characters.get(&CharacterId(1)).unwrap().y, 9);
}

#[test]
fn caligar_skelly_door_teleports_to_opposite_side_and_reverses_facing() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.dir = Direction::Right as u8;
    assert!(world.spawn_character(player, 9, 10));
    let mut door = item(8, ItemFlags::USED | ItemFlags::USE);
    door.driver = IDR_CALIGAR;
    door.driver_data = vec![12, 2];
    assert!(world.map.set_item_map(&mut door, 10, 10));
    world.add_item(door);

    let outcome = world.apply_caligar_skelly_door(ItemId(8), CharacterId(1), 2);

    assert_eq!(
        outcome,
        ItemDriverOutcome::CaligarSkellyDoor {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            door_index: 2,
        }
    );
    let player = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((player.x, player.y), (11, 10));
    assert_eq!(player.dir, Direction::Left as u8);
    assert_eq!(world.map.tile(9, 10).unwrap().character, 0);
    assert_eq!(world.map.tile(11, 10).unwrap().character, 1);
}

#[test]
fn caligar_skelly_door_reports_busy_target() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    assert!(world.spawn_character(player, 9, 10));
    assert!(world.spawn_character(character(2), 11, 10));
    let mut door = item(8, ItemFlags::USED | ItemFlags::USE);
    door.driver = IDR_CALIGAR;
    door.driver_data = vec![12, 1];
    assert!(world.map.set_item_map(&mut door, 10, 10));
    world.add_item(door);

    assert_eq!(
        world.apply_caligar_skelly_door(ItemId(8), CharacterId(1), 1),
        ItemDriverOutcome::CaligarSkellyDoorBusy {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(world.characters.get(&CharacterId(1)).unwrap().x, 9);
}

#[test]
fn caligar_skelly_door_diagonal_touch_preserves_retry_return_shape() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    assert!(world.spawn_character(player, 9, 9));
    let mut door = item(8, ItemFlags::USED | ItemFlags::USE);
    door.driver = IDR_CALIGAR;
    door.driver_data = vec![12, 1];
    assert!(world.map.set_item_map(&mut door, 10, 10));
    world.add_item(door);

    let outcome = world.apply_caligar_skelly_door(ItemId(8), CharacterId(1), 1);

    assert_eq!(
        outcome,
        ItemDriverOutcome::CaligarSkellyDoorBusy {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(
        crate::item_driver::legacy_item_driver_return_code(Some(IDR_CALIGAR), &outcome),
        2
    );
    let player = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((player.x, player.y), (9, 9));
}

#[test]
fn staffer2_spec_door_opens_schedules_and_timer_closes() {
    let mut world = World::default();
    world.map = MapGrid::new(300, 300);
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    world.add_character(player);
    let mut door = item(
        8,
        ItemFlags::USED
            | ItemFlags::USE
            | ItemFlags::MOVEBLOCK
            | ItemFlags::SIGHTBLOCK
            | ItemFlags::DOOR,
    );
    door.driver = IDR_STAFFER2;
    door.driver_data = vec![4, 0, 0, 0, 0, 0];
    door.sprite = 1200;
    assert!(world.map.set_item_map(&mut door, 10, 10));
    world.add_item(door);
    let mut marker = item(9, ItemFlags::USED);
    marker.sprite = 21203;
    assert!(world.map.set_item_map(&mut marker, 51, 234));
    world.add_item(marker);

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
        ItemDriverOutcome::StafferSpecDoorToggle {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            kind: 4,
        }
    );
    let door = world.items.get(&ItemId(8)).unwrap();
    assert_eq!(door.driver_data[1], 1);
    assert_eq!(door.driver_data[39], 1);
    assert_eq!(door.sprite, 1201);
    assert!(!door
        .flags
        .contains(ItemFlags::MOVEBLOCK | ItemFlags::SIGHTBLOCK));
    assert!(!world
        .map
        .tile(10, 10)
        .unwrap()
        .flags
        .contains(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK | MapFlags::DOOR));

    world.tick = Tick(TICKS_PER_SECOND * 10);
    let outcomes = world.process_due_timers(29);

    assert_eq!(
        outcomes,
        vec![ItemDriverOutcome::StafferSpecDoorToggle {
            item_id: ItemId(8),
            character_id: CharacterId(0),
            kind: 4,
        }]
    );
    let door = world.items.get(&ItemId(8)).unwrap();
    assert_eq!(door.driver_data[1], 0);
    assert_eq!(door.driver_data[39], 0);
    assert_eq!(door.sprite, 1200);
    assert!(door
        .flags
        .contains(ItemFlags::MOVEBLOCK | ItemFlags::SIGHTBLOCK));
    assert!(world
        .map
        .tile(10, 10)
        .unwrap()
        .flags
        .contains(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK | MapFlags::DOOR));
}

#[test]
fn staffer2_spec_door_reports_locked_without_marker_item() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    world.add_character(player);
    let mut door = item(8, ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK);
    door.driver = IDR_STAFFER2;
    door.driver_data = vec![5, 0, 0, 0, 0, 0];
    assert!(world.map.set_item_map(&mut door, 10, 10));
    world.add_item(door);

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
        ItemDriverOutcome::StafferSpecDoorLocked {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(world.items.get(&ItemId(8)).unwrap().driver_data[1], 0);
}

#[test]
fn world_applies_area14_trapdoor_stepback_open_and_timer_close() {
    let mut world = World::default();
    world.map = MapGrid::new(20, 20);
    let mut trapdoor = item(8, ItemFlags::USED | ItemFlags::USE);
    trapdoor.driver = crate::item_driver::IDR_TRAPDOOR;
    assert!(world.map.set_item_map(&mut trapdoor, 10, 10));
    world.add_item(trapdoor);

    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.dir = Direction::Right as u8;
    assert!(world.spawn_character(player, 10, 10));

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_TRAPDOOR,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        14,
    );

    assert!(matches!(outcome, ItemDriverOutcome::TrapdoorOpen { .. }));
    let player = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((player.x, player.y), (9, 10));
    let trapdoor = world.items.get(&ItemId(8)).unwrap();
    assert_eq!(trapdoor.driver_data[0], 1);
    assert_eq!(trapdoor.sprite, 1);
    assert!(world
        .map
        .tile(10, 10)
        .unwrap()
        .flags
        .contains(MapFlags::TMOVEBLOCK));
    assert_eq!(world.timers.used_timers(), 1);
    assert_eq!(
        world.drain_pending_system_texts(),
        vec![WorldSystemText {
            character_id: CharacterId(1),
            message: "A trapdoor opens under your feet, but you manage to jump back in time."
                .to_string(),
        }]
    );

    world.tick.0 = TICKS_PER_SECOND * 6;
    let outcomes = world.process_due_timers(14);
    assert!(matches!(
        outcomes.as_slice(),
        [ItemDriverOutcome::TrapdoorClose { item_id: ItemId(8) }]
    ));
    let trapdoor = world.items.get(&ItemId(8)).unwrap();
    assert_eq!(trapdoor.driver_data[0], 0);
    assert_eq!(trapdoor.sprite, 0);
    assert!(!world
        .map
        .tile(10, 10)
        .unwrap()
        .flags
        .contains(MapFlags::TMOVEBLOCK));
}

#[test]
fn world_applies_area14_trapdoor_steelbar_block() {
    let mut world = World::default();
    world.map = MapGrid::new(20, 20);
    let mut trapdoor = item(8, ItemFlags::USED | ItemFlags::USE);
    trapdoor.driver = crate::item_driver::IDR_TRAPDOOR;
    assert!(world.map.set_item_map(&mut trapdoor, 10, 10));
    world.add_item(trapdoor);

    let mut player = character(1);
    player.cursor_item = Some(ItemId(9));
    assert!(world.spawn_character(player, 9, 10));
    let mut steelbar = item(9, ItemFlags::USED);
    steelbar.template_id = crate::item_driver::IID_AREA14_STEELBAR;
    steelbar.carried_by = Some(CharacterId(1));
    world.add_item(steelbar);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_TRAPDOOR,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        14,
    );

    assert!(matches!(outcome, ItemDriverOutcome::TrapdoorBlocked { .. }));
    let trapdoor = world.items.get(&ItemId(8)).unwrap();
    assert_eq!(trapdoor.driver_data[0], 2);
    assert_eq!(trapdoor.sprite, 2);
    assert!(!world.items.contains_key(&ItemId(9)));
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().cursor_item,
        None
    );
}

#[test]
fn edemon_gate_spawn_slots_validate_character_serial_like_c() {
    let mut world = World::default();
    let mut gate = item(7, ItemFlags::USED);
    gate.driver = IDR_EDEMONGATE;
    gate.driver_data = vec![0];
    world.add_item(gate);

    assert!(world.apply_edemon_gate_spawn_result(ItemId(7), 0, CharacterId(2), 55));
    let mut demon = character(2);
    demon.serial = 55;
    assert!(world.spawn_character(demon, 62, 157));

    let context = world.edemon_gate_spawn_context(ItemId(7)).unwrap();
    assert_eq!(context.slot, 1);
    assert_eq!((context.x, context.y), (62, 164));

    world.characters.get_mut(&CharacterId(2)).unwrap().serial = 56;
    let context = world.edemon_gate_spawn_context(ItemId(7)).unwrap();
    assert_eq!(context.slot, 0);
    assert_eq!((context.x, context.y), (62, 157));
}

#[test]
fn world_warpkeydoor_teleports_consumes_key_and_flips_cardinal_direction() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    actor.dir = Direction::Right as u8;
    actor.inventory[30] = Some(ItemId(9));
    assert!(world.spawn_character(actor, 10, 20));

    let mut key = item(9, ItemFlags::USED);
    key.template_id = crate::item_driver::IID_AREA25_DOORKEY;
    key.carried_by = Some(CharacterId(1));
    world.add_item(key);

    let outcome = world.apply_item_driver_outcome(
        ItemDriverOutcome::WarpKeyDoor {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            key_item_id: ItemId(9),
            key_name: crate::item_driver::outcome_item_name("Warper Key"),
            x: 12,
            y: 20,
        },
        25,
    );

    assert!(matches!(outcome, ItemDriverOutcome::WarpKeyDoor { .. }));
    let actor = &world.characters[&CharacterId(1)];
    assert_eq!((actor.x, actor.y), (12, 20));
    assert_eq!(actor.dir, Direction::Left as u8);
    assert_eq!(actor.inventory[30], None);
    assert!(!world.items.contains_key(&ItemId(9)));
}

#[test]
fn world_warpkeydoor_direct_request_finds_inventory_key_not_cursor_key() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    actor.dir = Direction::Right as u8;
    actor.inventory[30] = Some(ItemId(9));
    assert!(world.spawn_character(actor, 10, 20));

    let mut key = item(9, ItemFlags::USED);
    key.template_id = crate::item_driver::IID_AREA25_DOORKEY;
    key.name = "Warper Key".into();
    key.carried_by = Some(CharacterId(1));
    world.add_item(key);

    let mut door = item(7, ItemFlags::USED | ItemFlags::USE);
    door.driver = crate::item_driver::IDR_WARPKEYDOOR;
    door.x = 11;
    door.y = 20;
    world.add_item(door);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_WARPKEYDOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        25,
    );

    assert!(matches!(outcome, ItemDriverOutcome::WarpKeyDoor { .. }));
    let actor = &world.characters[&CharacterId(1)];
    assert_eq!((actor.x, actor.y), (12, 20));
    assert_eq!(actor.inventory[30], None);
    assert!(!world.items.contains_key(&ItemId(9)));
}

#[test]
fn world_warpkeydoor_direct_request_ignores_cursor_key_like_c_has_item() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    actor.cursor_item = Some(ItemId(9));
    assert!(world.spawn_character(actor, 10, 20));

    let mut key = item(9, ItemFlags::USED);
    key.template_id = crate::item_driver::IID_AREA25_DOORKEY;
    key.carried_by = Some(CharacterId(1));
    world.add_item(key);

    let mut door = item(7, ItemFlags::USED | ItemFlags::USE);
    door.driver = crate::item_driver::IDR_WARPKEYDOOR;
    door.x = 11;
    door.y = 20;
    world.add_item(door);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_WARPKEYDOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        25,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::WarpKeyDoorMissingKey {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
    let actor = &world.characters[&CharacterId(1)];
    assert_eq!((actor.x, actor.y), (10, 20));
    assert_eq!(actor.cursor_item, Some(ItemId(9)));
    assert!(world.items.contains_key(&ItemId(9)));
}

#[test]
fn world_warptrialdoor_discovers_room_and_teleports_player() {
    let mut world = World {
        map: MapGrid::new(32, 32),
        ..World::default()
    };
    let mut actor = character(1);
    actor.x = 9;
    actor.y = 12;
    assert!(world.spawn_character(actor, 9, 12));

    let mut left = item(7, ItemFlags::USED | ItemFlags::USE);
    left.driver = crate::item_driver::IDR_WARPTRIALDOOR;
    assert!(world.map.set_item_map(&mut left, 10, 12));
    world.add_item(left);
    let mut right = item(8, ItemFlags::USED | ItemFlags::USE);
    right.driver = crate::item_driver::IDR_WARPTRIALDOOR;
    assert!(world.map.set_item_map(&mut right, 20, 12));
    world.add_item(right);

    for x in 11..20 {
        world.map.set_flags(x, 10, MapFlags::MOVEBLOCK);
        world.map.set_flags(x, 20, MapFlags::MOVEBLOCK);
    }

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_WARPTRIALDOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        25,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::WarpTrialDoor {
            spawn_x: 15,
            spawn_y: 15,
            fighter_target_x: 21,
            fighter_target_y: 12,
            ..
        }
    ));
    let actor = &world.characters[&CharacterId(1)];
    assert_eq!((actor.x, actor.y), (11, 12));
    assert_eq!(
        &world.items[&ItemId(7)].driver_data[2..8],
        &[10, 10, 20, 20, 8, 0]
    );
}
