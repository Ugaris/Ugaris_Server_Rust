use super::*;

#[test]
fn execute_edemon_door_driver_ports_key_power_and_timer_gates() {
    let mut ch = character(1);
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_EDEMONDOOR);
    door.x = 10;
    door.y = 11;
    door.driver_data = vec![0, 0x44, 0x33, 0x22, 0x11, 0, 4];
    let request = ItemDriverRequest::Driver {
        driver: IDR_EDEMONDOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut ch,
            &mut door,
            request,
            6,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::EdemonDoorLocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    let keyring_context = ItemDriverContext {
        door_key: Some(DoorKeyAccess {
            key_id: 0x1122_3344,
            name: "Copper Key".to_string(),
            source: DoorKeySource::Keyring,
        }),
        edemon_section_power: Some(10),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(&mut ch, &mut door, request, 6, false, &keyring_context,),
        ItemDriverOutcome::EdemonDoorLocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    let carried_key_context = ItemDriverContext {
        door_key: Some(DoorKeyAccess {
            key_id: 0x1122_3344,
            name: "Copper Key".to_string(),
            source: DoorKeySource::Carried,
        }),
        edemon_section_power: Some(0),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut ch,
            &mut door,
            request,
            6,
            false,
            &carried_key_context,
        ),
        ItemDriverOutcome::EdemonDoorLifeless {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    let powered_context = ItemDriverContext {
        edemon_section_power: Some(10),
        ..carried_key_context
    };
    assert_eq!(
        execute_item_driver_with_context(&mut ch, &mut door, request, 6, false, &powered_context,),
        ItemDriverOutcome::EdemonDoorToggle {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            key_name: Some(outcome_item_name("Copper Key")),
            locking: false,
        }
    );

    door.driver_data[6] = 0;
    let section_zero_unpowered = ItemDriverContext {
        door_key: Some(DoorKeyAccess {
            key_id: 0x1122_3344,
            name: "Copper Key".to_string(),
            source: DoorKeySource::Carried,
        }),
        edemon_section_power: Some(0),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut ch,
            &mut door,
            request,
            6,
            false,
            &section_zero_unpowered,
        ),
        ItemDriverOutcome::EdemonDoorLifeless {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    door.driver_data[0] = 1;
    door.driver_data[39] = 1;
    let mut timer_character = character(0);
    let timer_context = ItemDriverContext {
        timer_call: true,
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut door,
            ItemDriverRequest::Driver {
                driver: IDR_EDEMONDOOR,
                item_id: ItemId(7),
                character_id: CharacterId(0),
                spec: 0,
            },
            6,
            false,
            &timer_context,
        ),
        ItemDriverOutcome::EdemonDoorToggle {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            key_name: None,
            locking: false,
        }
    );
    assert_eq!(door.driver_data[39], 0);
}

#[test]
fn edemon_switch_use_disables_fire_and_schedules_reenable() {
    let mut character = character(1);
    let mut lever = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_EDEMONSWITCH);
    lever.sprite = 100;
    lever.driver_data = vec![1, 0, 0, 0, 0];
    lever.modifier_index[0] = V_LIGHT;
    lever.modifier_value[0] = 64;
    let request = ItemDriverRequest::Driver {
        driver: IDR_EDEMONSWITCH,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut lever,
            request,
            6,
            false,
            &ItemDriverContext {
                current_tick: 123,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::LightChanged {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            schedule_after_ticks: Some(EDEMON_SWITCH_COOLDOWN_TICKS + 1),
        }
    );
    assert_eq!(lever.driver_data[0], 0);
    assert_eq!(
        u32::from_le_bytes(lever.driver_data[1..5].try_into().unwrap()),
        123 + EDEMON_SWITCH_COOLDOWN_TICKS as u32
    );
    assert_eq!(lever.modifier_value[0], 0);
    assert_eq!(lever.sprite, 101);
}

#[test]
fn edemon_switch_timer_reenables_after_cooldown() {
    let mut timer_character = character(0);
    let mut lever = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_EDEMONSWITCH);
    lever.sprite = 101;
    lever.driver_data = vec![0, 10, 0, 0, 0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_EDEMONSWITCH,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut lever,
            request,
            6,
            false,
            &ItemDriverContext {
                timer_call: true,
                current_tick: 10,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::Noop
    );

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut lever,
            request,
            6,
            false,
            &ItemDriverContext {
                timer_call: true,
                current_tick: 11,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::LightChanged {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            schedule_after_ticks: None,
        }
    );
    assert_eq!(lever.driver_data[0], 1);
    assert_eq!(lever.modifier_index[0], V_LIGHT);
    assert_eq!(lever.modifier_value[0], 64);
    assert_eq!(lever.sprite, 100);
}

#[test]
fn edemon_gate_timer_requests_one_spawn_and_reschedule() {
    let mut timer_character = character(0);
    let mut gate = item(7, ItemFlags::USED, 0, IDR_EDEMONGATE);
    gate.driver_data = vec![0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_EDEMONGATE,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    let outcome = execute_item_driver_with_context(
        &mut timer_character,
        &mut gate,
        request,
        6,
        false,
        &ItemDriverContext {
            timer_call: true,
            edemon_gate_spawn: Some(EdemonGateSpawnContext {
                slot: 2,
                x: 62,
                y: 174,
            }),
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::EdemonGateSpawn {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            template: "edemon2s",
            slot: 2,
            x: 62,
            y: 174,
            schedule_after_ticks: TICKS_PER_SECOND * 10,
        }
    );
}

#[test]
fn edemon_switch_reports_stuck_while_fire_is_disabled() {
    let mut character = character(1);
    let mut lever = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_EDEMONSWITCH);
    lever.sprite = 100;
    lever.driver_data = vec![0, 0, 0, 0, 0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_EDEMONSWITCH,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut lever, request, 6, false),
        ItemDriverOutcome::EdemonSwitchStuck {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(lever.sprite, 100);
}

#[test]
fn edemon_block_character_use_targets_facing_tile_and_stores_touch_tick() {
    let mut character = character(1);
    character.dir = crate::direction::Direction::Right as u8;
    let mut block = item(
        7,
        ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK,
        0,
        IDR_EDEMONBLOCK,
    );
    block.x = 10;
    block.y = 11;
    let request = ItemDriverRequest::Driver {
        driver: IDR_EDEMONBLOCK,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut block,
            request,
            6,
            false,
            &ItemDriverContext {
                current_tick: 77,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::EdemonBlockMove {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            target_x: 11,
            target_y: 11,
            schedule_after_ticks: None,
        }
    );
    assert_eq!(drdata_u32(&block, 0), 77);
}

#[test]
fn edemon_block_timer_remembers_origin_and_returns_after_idle_timeout() {
    let mut timer_character = character(0);
    let mut block = item(
        7,
        ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK,
        0,
        IDR_EDEMONBLOCK,
    );
    block.x = 10;
    block.y = 11;
    let request = ItemDriverRequest::Driver {
        driver: IDR_EDEMONBLOCK,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut block,
            request,
            6,
            false,
            &ItemDriverContext {
                timer_call: true,
                current_tick: 1,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::LightChanged {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            schedule_after_ticks: Some(TICKS_PER_SECOND * 5),
        }
    );
    assert_eq!(drdata_u16(&block, 4), 10);
    assert_eq!(drdata_u16(&block, 6), 11);

    block.x = 12;
    block.y = 11;
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut block,
            request,
            6,
            false,
            &ItemDriverContext {
                timer_call: true,
                current_tick: (TICKS_PER_SECOND * 60 * 15 + 2) as u32,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::EdemonBlockMove {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            target_x: 10,
            target_y: 11,
            schedule_after_ticks: Some(TICKS_PER_SECOND * 5),
        }
    );
}

#[test]
fn edemonball_timer_waits_while_global_fire_is_disabled() {
    let mut timer_character = character(0);
    let mut launcher = item(7, ItemFlags::USED, 0, IDR_EDEMONBALL);
    launcher.sprite = 14159;
    launcher.x = 10;
    launcher.y = 10;
    launcher.driver_data = vec![0, 1, 7, 0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_EDEMONBALL,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut launcher,
            request,
            6,
            false,
            &ItemDriverContext {
                timer_call: true,
                edemon_fire_enabled: Some(false),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::EdemonBallInactive {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            schedule_after_ticks: TICKS_PER_SECOND,
        }
    );
    assert_eq!(launcher.sprite, 14160);
    assert_eq!(launcher.driver_data[3], 0);
}

#[test]
fn edemonball_timer_uses_section_power_gate_for_part_five() {
    let mut timer_character = character(0);
    let mut launcher = item(7, ItemFlags::USED, 0, IDR_EDEMONBALL);
    launcher.sprite = 14160;
    launcher.x = 10;
    launcher.y = 10;
    launcher.driver_data = vec![2, 1, 7, 0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_EDEMONBALL,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut launcher,
            request,
            6,
            false,
            &ItemDriverContext {
                timer_call: true,
                edemon_section_power: Some(0),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::EdemonBallInactive {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            schedule_after_ticks: TICKS_PER_SECOND,
        }
    );
    assert_eq!(launcher.sprite, 14160);
    assert_eq!(launcher.driver_data[3], 0);

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut launcher,
            request,
            6,
            false,
            &ItemDriverContext {
                timer_call: true,
                edemon_section_power: Some(42),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::EdemonBallProjectile {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            start_x: 10,
            start_y: 11,
            target_x: 10,
            target_y: 20,
            strength: 7,
            base_sprite: 1,
            schedule_after_ticks: TICKS_PER_SECOND * 16,
        }
    );
    assert_eq!(launcher.sprite, 14161);
    assert_eq!(launcher.driver_data[3], 1);
}

#[test]
fn edemon_light_timer_follows_section_power_and_reschedules() {
    let mut timer_character = character(0);
    let mut light = item(7, ItemFlags::USED, 0, IDR_EDEMONLIGHT);
    light.sprite = 14189;
    let request = ItemDriverRequest::Driver {
        driver: IDR_EDEMONLIGHT,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(IDR_EDEMONLIGHT, 40);
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut light,
            request,
            6,
            false,
            &ItemDriverContext {
                timer_call: true,
                edemon_section_power: Some(42),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::LightChanged {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            schedule_after_ticks: Some(TICKS_PER_SECOND),
        }
    );
    assert_eq!(light.sprite, 14191);
    assert_eq!(light.modifier_index[0], V_LIGHT);
    assert_eq!(light.modifier_value[0], 200);

    execute_item_driver_with_context(
        &mut timer_character,
        &mut light,
        request,
        6,
        false,
        &ItemDriverContext {
            timer_call: true,
            edemon_section_power: Some(249),
            ..ItemDriverContext::default()
        },
    );
    assert_eq!(light.sprite, 14189);
    assert_eq!(light.modifier_value[0], 0);

    let mut user = character(1);
    assert_eq!(
        execute_item_driver_with_context(
            &mut user,
            &mut light,
            ItemDriverRequest::Driver {
                driver: IDR_EDEMONLIGHT,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            6,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn edemon_tube_timer_follows_section_power_and_remembers_target() {
    let mut timer_character = character(0);
    let mut tube = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_EDEMONTUBE);
    tube.sprite = 14137;
    tube.driver_data = vec![4, 0, 0, 0, 0, 0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_EDEMONTUBE,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(IDR_EDEMONTUBE, 43);
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut tube,
            request,
            6,
            false,
            &ItemDriverContext {
                timer_call: true,
                edemon_section_power: Some(42),
                edemon_tube_target: Some((50, 61)),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::LightChanged {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            schedule_after_ticks: Some(TICKS_PER_SECOND),
        }
    );
    assert_eq!(tube.sprite, 14138);
    assert_eq!(tube.modifier_index[0], V_LIGHT);
    assert_eq!(tube.modifier_value[0], 200);
    assert_eq!(drdata_u16(&tube, 2), 50);
    assert_eq!(drdata_u16(&tube, 4), 61);

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut tube,
            request,
            6,
            false,
            &ItemDriverContext {
                timer_call: true,
                edemon_section_power: Some(251),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::EdemonTubePulse {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            x: 50,
            y: 61,
            schedule_after_ticks: TICKS_PER_SECOND,
        }
    );

    execute_item_driver_with_context(
        &mut timer_character,
        &mut tube,
        request,
        6,
        false,
        &ItemDriverContext {
            timer_call: true,
            edemon_section_power: Some(249),
            ..ItemDriverContext::default()
        },
    );
    assert_eq!(tube.sprite, 14137);
    assert_eq!(tube.modifier_value[0], 0);
}

#[test]
fn edemon_tube_character_use_teleports_to_remembered_target() {
    let mut user = character(1);
    let mut tube = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_EDEMONTUBE);
    tube.driver_data = vec![4, 0, 50, 0, 61, 0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_EDEMONTUBE,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut user,
            &mut tube,
            request,
            6,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::TeleportDoor {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            x: 50,
            y: 61,
        }
    );
}

#[test]
fn edemon_loader_accepts_yellow_crystal_and_starts_animation() {
    let mut character = character(1);
    character.cursor_item = Some(ItemId(9));
    let mut loader = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_EDEMONLOADER);
    loader.driver_data = vec![2, 0, 0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_EDEMONLOADER,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    let outcome = execute_item_driver_with_context(
        &mut character,
        &mut loader,
        request,
        6,
        false,
        &ItemDriverContext {
            cursor_template_id: Some(IID_AREA6_YELLOWCRYSTAL),
            cursor_drdata0: Some(86),
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(character.cursor_item, None);
    assert!(character.flags.contains(CharacterFlags::ITEMS));
    assert_eq!(loader.driver_data, vec![2, 86, 7]);
    assert_eq!(loader.sprite, 14260);
    assert_eq!(
        outcome,
        ItemDriverOutcome::EdemonLoaderChanged {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            consumed_cursor_item_id: Some(ItemId(9)),
            ground_overlay_sprite: 14240,
            sound_type: Some(41),
            schedule_after_ticks: None,
        }
    );
}

#[test]
fn edemon_loader_timer_decays_power_animation_and_schedules() {
    let mut timer_character = character(0);
    let mut loader = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_EDEMONLOADER);
    loader.sprite = 14235;
    loader.driver_data = vec![2, 1, 1];
    let request = ItemDriverRequest::Driver {
        driver: IDR_EDEMONLOADER,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut loader,
            request,
            6,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::EdemonLoaderChanged {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            consumed_cursor_item_id: None,
            ground_overlay_sprite: 14240,
            sound_type: Some(43),
            schedule_after_ticks: Some(TICKS_PER_SECOND),
        }
    );
    assert_eq!(loader.driver_data, vec![2, 0, 0]);
    assert_eq!(loader.sprite, 14234);
}

#[test]
fn edemon_loader_blocks_missing_wrong_and_stuck_crystals() {
    let mut character = character(1);
    let mut loader = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_EDEMONLOADER);
    loader.driver_data = vec![2, 0, 0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_EDEMONLOADER,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut loader, request, 6, false),
        ItemDriverOutcome::EdemonLoaderBlocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            reason: EdemonLoaderBlockReason::NeedsCrystal,
        }
    );

    character.cursor_item = Some(ItemId(9));
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut loader,
            request,
            6,
            false,
            &ItemDriverContext {
                cursor_template_id: Some(123),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::EdemonLoaderBlocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            reason: EdemonLoaderBlockReason::WrongCrystal,
        }
    );

    loader.driver_data[1] = 4;
    assert_eq!(
        execute_item_driver(&mut character, &mut loader, request, 6, false),
        ItemDriverOutcome::EdemonLoaderBlocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            reason: EdemonLoaderBlockReason::CrystalAlreadyPresent,
        }
    );
    character.cursor_item = None;
    assert_eq!(
        execute_item_driver(&mut character, &mut loader, request, 6, false),
        ItemDriverOutcome::EdemonLoaderBlocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            reason: EdemonLoaderBlockReason::CrystalStuck,
        }
    );
}
