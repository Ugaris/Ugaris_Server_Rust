use super::*;

#[test]
fn fdemon_light_timer_follows_loader_power_and_reschedules() {
    let mut timer_character = character(0);
    let mut light = item(7, ItemFlags::USED, 0, IDR_FDEMONLIGHT);
    light.sprite = 14189;
    let request = ItemDriverRequest::Driver {
        driver: IDR_FDEMONLIGHT,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut light,
            request,
            8,
            false,
            &ItemDriverContext {
                timer_call: true,
                fdemon_loader_power: Some(300),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::LightChanged {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            schedule_after_ticks: Some(TICKS_PER_SECOND),
        }
    );
    assert_eq!(light.sprite, 14192);
    assert_eq!(light.modifier_index[0], V_LIGHT);
    assert_eq!(light.modifier_value[0], 200);

    execute_item_driver_with_context(
        &mut timer_character,
        &mut light,
        request,
        8,
        false,
        &ItemDriverContext {
            timer_call: true,
            fdemon_loader_power: Some(0),
            ..ItemDriverContext::default()
        },
    );
    assert_eq!(light.sprite, 14189);
    assert_eq!(light.modifier_value[0], 0);
}

#[test]
fn fdemon_light_preserves_area8_libload_guard_and_player_noop() {
    let mut timer_character = character(0);
    let mut light = item(7, ItemFlags::USED, 0, IDR_FDEMONLIGHT);
    let request = ItemDriverRequest::Driver {
        driver: IDR_FDEMONLIGHT,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut light,
            request,
            6,
            false,
            &ItemDriverContext {
                timer_call: true,
                fdemon_loader_power: Some(300),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_FDEMONLIGHT,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            required_area: 8,
        }
    );

    let mut user = character(1);
    assert_eq!(
        execute_item_driver_with_context(
            &mut user,
            &mut light,
            ItemDriverRequest::Driver {
                driver: IDR_FDEMONLIGHT,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            8,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn fdemon_cannon_dispatches_timer_and_lifeless_use() {
    let mut timer_character = character(0);
    let mut cannon = item(7, ItemFlags::USED, 0, IDR_FDEMONCANNON);
    let timer_request = ItemDriverRequest::Driver {
        driver: IDR_FDEMONCANNON,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(IDR_FDEMONCANNON, 46);
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut cannon,
            timer_request,
            8,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::FdemonCannonPulse {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            schedule_after_ticks: TICKS_PER_SECOND,
        }
    );

    let mut user = character(1);
    assert_eq!(
        execute_item_driver_with_context(
            &mut user,
            &mut cannon,
            ItemDriverRequest::Driver {
                driver: IDR_FDEMONCANNON,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            8,
            false,
            &ItemDriverContext {
                fdemon_loader_power: Some(0),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::FdemonCannonLifeless {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    assert_eq!(
        execute_item_driver_with_context(
            &mut user,
            &mut cannon,
            ItemDriverRequest::Driver {
                driver: IDR_FDEMONCANNON,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            6,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_FDEMONCANNON,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            required_area: 8,
        }
    );
}

#[test]
fn fdemon_gate_timer_requests_one_spawn_and_reschedule() {
    let mut timer_character = character(0);
    let mut gate = item(7, ItemFlags::USED, 0, IDR_FDEMONGATE);
    gate.x = 100;
    gate.y = 101;
    gate.driver_data = vec![4, 7];
    let request = ItemDriverRequest::Driver {
        driver: IDR_FDEMONGATE,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(IDR_FDEMONGATE, 47);
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut gate,
            request,
            8,
            false,
            &ItemDriverContext {
                timer_call: true,
                fdemon_gate_spawn: Some(FdemonGateSpawnContext {
                    slot: 2,
                    x: 100,
                    y: 101,
                }),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::FdemonGateSpawn {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            level: 4,
            slot: 2,
            x: 100,
            y: 101,
            schedule_after_ticks: 7 * TICKS_PER_SECOND,
        }
    );
}

#[test]
fn fdemon_gate_preserves_area8_and_timer_only_guards() {
    let mut timer_character = character(0);
    let mut gate = item(7, ItemFlags::USED, 0, IDR_FDEMONGATE);
    gate.driver_data = vec![3, 5];
    let request = ItemDriverRequest::Driver {
        driver: IDR_FDEMONGATE,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut gate,
            request,
            6,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_FDEMONGATE,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            required_area: 8,
        }
    );

    let mut player = character(1);
    assert_eq!(
        execute_item_driver_with_context(
            &mut player,
            &mut gate,
            ItemDriverRequest::Driver {
                driver: IDR_FDEMONGATE,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            8,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn fdemon_waypoint_marks_player_or_fdemon_state_and_reschedules() {
    let mut user = character(1);
    user.serial = 1234;
    let mut waypoint = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FDEMONWAYPOINT);
    let request = ItemDriverRequest::Driver {
        driver: IDR_FDEMONWAYPOINT,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut user,
            &mut waypoint,
            request,
            8,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::FdemonWaypoint {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spotted_enemy: true,
            target_character_id: Some(CharacterId(1)),
            target_serial: Some(1234),
            schedule_after_ticks: TICKS_PER_SECOND * 3,
        }
    );

    user.flags.insert(CharacterFlags::FDEMON);
    assert_eq!(
        execute_item_driver_with_context(
            &mut user,
            &mut waypoint,
            request,
            8,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::FdemonWaypoint {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spotted_enemy: false,
            target_character_id: None,
            target_serial: None,
            schedule_after_ticks: TICKS_PER_SECOND * 3,
        }
    );

    let mut timer_character = character(0);
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut waypoint,
            ItemDriverRequest::Driver {
                driver: IDR_FDEMONWAYPOINT,
                item_id: ItemId(7),
                character_id: CharacterId(0),
                spec: 0,
            },
            8,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::FdemonWaypoint {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spotted_enemy: false,
            target_character_id: None,
            target_serial: None,
            schedule_after_ticks: TICKS_PER_SECOND * 3,
        }
    );
}

#[test]
fn fdemon_waypoint_preserves_area8_libload_guard() {
    let mut user = character(1);
    let mut waypoint = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FDEMONWAYPOINT);

    assert_eq!(
        execute_item_driver_with_context(
            &mut user,
            &mut waypoint,
            ItemDriverRequest::Driver {
                driver: IDR_FDEMONWAYPOINT,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            7,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_FDEMONWAYPOINT,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            required_area: 8,
        }
    );
}

#[test]
fn fdemon_farm_timer_grows_and_exposes_crystal_overlay() {
    let mut timer_character = character(0);
    let mut farm = item(7, ItemFlags::USED, 0, IDR_FDEMONFARM);
    farm.driver_data = vec![5, 24, 20];
    let request = ItemDriverRequest::Driver {
        driver: IDR_FDEMONFARM,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut farm,
            request,
            8,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::FdemonFarmChanged {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            foreground_sprite: 0,
            schedule_after_ticks: Some(TICKS_PER_SECOND * 2),
        }
    );
    assert_eq!(farm.driver_data[2], 25);

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut farm,
            request,
            8,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::FdemonFarmChanged {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            foreground_sprite: 59040,
            schedule_after_ticks: Some(TICKS_PER_SECOND * 2),
        }
    );
    assert_eq!(farm.driver_data[2], 25);
}

#[test]
fn fdemon_farm_player_harvest_and_block_messages_are_typed() {
    let mut user = character(1);
    let mut farm = item(7, ItemFlags::USED, 0, IDR_FDEMONFARM);
    farm.driver_data = vec![5, 48, 48];
    let request = ItemDriverRequest::Driver {
        driver: IDR_FDEMONFARM,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut user, &mut farm, request, 8, false),
        ItemDriverOutcome::FdemonFarmHarvest {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            template: FdemonCrystalTemplate::Giant,
            foreground_sprite: 0,
        }
    );
    assert_eq!(farm.driver_data[2], 0);

    assert_eq!(
        execute_item_driver(&mut user, &mut farm, request, 8, false),
        ItemDriverOutcome::FdemonFarmNotReady {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            current: 5,
            required: 48,
        }
    );

    user.cursor_item = Some(ItemId(99));
    assert_eq!(
        execute_item_driver(&mut user, &mut farm, request, 8, false),
        ItemDriverOutcome::FdemonFarmCursorOccupied {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn fdemon_loader_accepts_red_crystal_and_starts_animation() {
    let mut character = character(1);
    character.cursor_item = Some(ItemId(9));
    let mut loader = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FDEMONLOADER);
    let request = ItemDriverRequest::Driver {
        driver: IDR_FDEMONLOADER,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    let outcome = execute_item_driver_with_context(
        &mut character,
        &mut loader,
        request,
        8,
        false,
        &ItemDriverContext {
            cursor_template_id: Some(IID_AREA8_REDCRYSTAL),
            cursor_drdata0: Some(12),
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(character.cursor_item, None);
    assert!(character.flags.contains(CharacterFlags::ITEMS));
    assert_eq!(drdata_u16(&loader, 1), 0);
    assert_eq!(loader.driver_data[3], 7);
    assert_eq!(drdata_u16(&loader, 4), 1200);
    assert_eq!(loader.sprite, 59036);
    assert_eq!(
        outcome,
        ItemDriverOutcome::FdemonLoaderChanged {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            consumed_cursor_item_id: Some(ItemId(9)),
            ground_overlay_sprite: 59021,
            sound_type: Some(41),
            schedule_after_ticks: None,
        }
    );
}

#[test]
fn fdemon_loader_timer_counts_animation_and_power() {
    let mut timer_character = character(0);
    let mut loader = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FDEMONLOADER);
    loader.driver_data = vec![1, 0, 0, 1, 2, 0, 0];
    loader.sprite = 59030;
    let request = ItemDriverRequest::Driver {
        driver: IDR_FDEMONLOADER,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    let outcome = execute_item_driver_with_context(
        &mut timer_character,
        &mut loader,
        request,
        8,
        false,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(drdata_u16(&loader, 1), 1);
    assert_eq!(loader.driver_data[3], 0);
    assert_eq!(drdata_u16(&loader, 4), 1);
    assert_eq!(loader.sprite, 59039);
    assert_eq!(
        outcome,
        ItemDriverOutcome::FdemonLoaderChanged {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            consumed_cursor_item_id: None,
            ground_overlay_sprite: 59029,
            sound_type: None,
            schedule_after_ticks: Some(TICKS_PER_SECOND),
        }
    );
}

#[test]
fn fdemon_loader_blocks_wrong_or_missing_crystal() {
    let mut character = character(1);
    let mut loader = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FDEMONLOADER);
    let request = ItemDriverRequest::Driver {
        driver: IDR_FDEMONLOADER,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut loader, request, 8, false),
        ItemDriverOutcome::FdemonLoaderBlocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            reason: FdemonLoaderBlockReason::NeedsCrystal,
        }
    );

    character.cursor_item = Some(ItemId(9));
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut loader,
            request,
            8,
            false,
            &ItemDriverContext {
                cursor_template_id: Some(123),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::FdemonLoaderBlocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            reason: FdemonLoaderBlockReason::WrongCrystal,
        }
    );
}

#[test]
fn fdemon_blood_blocks_bare_wrong_and_full_cursor_items() {
    let mut character = character(1);
    let mut blood = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FDEMONBLOOD);
    let request = ItemDriverRequest::Driver {
        driver: IDR_FDEMONBLOOD,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(IDR_FDEMONBLOOD, 50);
    assert_eq!(
        execute_item_driver(&mut character, &mut blood, request, 8, false),
        ItemDriverOutcome::FdemonBloodBlocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            reason: FdemonBloodBlockReason::BareHands,
        }
    );

    character.cursor_item = Some(ItemId(9));
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut blood,
            request,
            8,
            false,
            &ItemDriverContext {
                cursor_template_id: Some(0x0100_004A),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::FdemonBloodBlocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            reason: FdemonBloodBlockReason::WrongItem,
        }
    );

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut blood,
            request,
            8,
            false,
            &ItemDriverContext {
                cursor_template_id: Some(IID_AREA8_BLOOD),
                cursor_drdata0: Some(3),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::FdemonBloodBlocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            reason: FdemonBloodBlockReason::ContainerFull,
        }
    );
}

#[test]
fn fdemon_blood_destroys_flasks_or_fills_blood_container() {
    let mut character = character(1);
    character.cursor_item = Some(ItemId(9));
    let mut blood = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FDEMONBLOOD);
    let request = ItemDriverRequest::Driver {
        driver: IDR_FDEMONBLOOD,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut blood,
            request,
            8,
            false,
            &ItemDriverContext {
                cursor_driver: Some(IDR_FLASK),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::FdemonBloodDestroyedFlask {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            flask_item_id: ItemId(9),
        }
    );
    assert_eq!(character.cursor_item, None);
    assert!(character.flags.contains(CharacterFlags::ITEMS));
    assert_eq!(blood.sprite, 14348);

    character.cursor_item = Some(ItemId(10));
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut blood,
            request,
            8,
            false,
            &ItemDriverContext {
                cursor_template_id: Some(IID_AREA8_BLOOD),
                cursor_drdata0: Some(2),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::FdemonBloodFilled {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            container_item_id: ItemId(10),
            amount: 3,
        }
    );
    assert!(character.flags.contains(CharacterFlags::ITEMS));
}

#[test]
fn fdemon_lava_blocks_wrong_or_empty_cursor_items() {
    let mut character = character(1);
    let mut lava = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FDEMONLAVA);
    let request = ItemDriverRequest::Driver {
        driver: IDR_FDEMONLAVA,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(IDR_FDEMONLAVA, 51);
    assert_eq!(
        execute_item_driver(&mut character, &mut lava, request, 8, false),
        ItemDriverOutcome::FdemonLavaBlocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            reason: FdemonLavaBlockReason::BareHands,
        }
    );

    character.cursor_item = Some(ItemId(9));
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut lava,
            request,
            8,
            false,
            &ItemDriverContext {
                cursor_template_id: Some(0x0100_004A),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::FdemonLavaBlocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            reason: FdemonLavaBlockReason::WrongItem,
        }
    );

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut lava,
            request,
            8,
            false,
            &ItemDriverContext {
                cursor_template_id: Some(IID_AREA8_BLOOD),
                cursor_drdata0: Some(0),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::FdemonLavaBlocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            reason: FdemonLavaBlockReason::EmptyContainer,
        }
    );
}

#[test]
fn fdemon_lava_activation_and_timer_stages_match_c_core() {
    let mut actor = character(1);
    actor.cursor_item = Some(ItemId(9));
    let mut lava = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FDEMONLAVA);
    let request = ItemDriverRequest::Driver {
        driver: IDR_FDEMONLAVA,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut lava,
            request,
            8,
            false,
            &ItemDriverContext {
                cursor_template_id: Some(IID_AREA8_BLOOD),
                cursor_drdata0: Some(2),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::FdemonLavaActivated {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            container_item_id: ItemId(9),
            amount: 1,
            schedule_after_ticks: TICKS_PER_SECOND,
        }
    );
    assert_eq!(lava.driver_data[0], 120);
    assert_eq!(lava.sprite, 14366);

    let mut timer = character(0);
    lava.driver_data[0] = 20;
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer,
            &mut lava,
            ItemDriverRequest::Driver {
                driver: IDR_FDEMONLAVA,
                item_id: ItemId(7),
                character_id: CharacterId(0),
                spec: 0,
            },
            8,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::FdemonLavaPulse {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            stage: 19,
            damage: 10 * POWERSCALE,
            armor_percent: 50,
            schedule_after_ticks: Some(TICKS_PER_SECOND),
        }
    );
    assert_eq!(lava.sprite, 14364);
}
