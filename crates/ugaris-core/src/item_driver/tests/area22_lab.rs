use super::*;

#[test]
fn labtorch_is_area22_guarded_like_legacy_module() {
    let mut actor = character(2);
    let mut torch = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LABTORCH);

    assert_eq!(
        execute_item_driver(
            &mut actor,
            &mut torch,
            ItemDriverRequest::Driver {
                driver: IDR_LABTORCH,
                item_id: ItemId(7),
                character_id: CharacterId(2),
                spec: 0,
            },
            21,
            false,
        ),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_LABTORCH,
            item_id: ItemId(7),
            character_id: CharacterId(2),
            required_area: 22,
        }
    );
}

#[test]
fn labtorch_timer_call_stores_current_light_value() {
    let mut timer = character(0);
    let mut torch = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LABTORCH);
    torch.modifier_index[0] = V_LIGHT;
    torch.modifier_value[0] = 42;

    let outcome = execute_item_driver(
        &mut timer,
        &mut torch,
        ItemDriverRequest::Driver {
            driver: IDR_LABTORCH,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        22,
        false,
    );

    assert_eq!(outcome, ItemDriverOutcome::Noop);
    assert_eq!(torch.driver_data[1], 42);
}

#[test]
fn labtorch_player_cannot_light_unlit_torch() {
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    let mut torch = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LABTORCH);
    torch.sprite = 100;
    torch.driver_data = vec![0, 55];

    let outcome = execute_item_driver(
        &mut player,
        &mut torch,
        ItemDriverRequest::Driver {
            driver: IDR_LABTORCH,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
        false,
    );

    assert_eq!(outcome, ItemDriverOutcome::Noop);
    assert_eq!(torch.sprite, 100);
    assert_eq!(torch.driver_data, vec![0, 55]);
    assert_eq!(torch.modifier_value[0], 0);
}

#[test]
fn labtorch_npc_lights_unlit_torch() {
    let mut npc = character(2);
    let mut torch = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LABTORCH);
    torch.sprite = 100;
    torch.driver_data = vec![0, 55];

    let outcome = execute_item_driver(
        &mut npc,
        &mut torch,
        ItemDriverRequest::Driver {
            driver: IDR_LABTORCH,
            item_id: ItemId(7),
            character_id: CharacterId(2),
            spec: 0,
        },
        22,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::LightChanged {
            item_id: ItemId(7),
            character_id: CharacterId(2),
            schedule_after_ticks: None,
        }
    );
    assert_eq!(torch.sprite, 101);
    assert_eq!(torch.driver_data[0], 1);
    assert_eq!(torch.modifier_index[0], V_LIGHT);
    assert_eq!(torch.modifier_value[0], 55);
}

#[test]
fn labtorch_character_extinguishes_lit_torch() {
    let mut npc = character(2);
    let mut torch = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LABTORCH);
    torch.sprite = 101;
    torch.driver_data = vec![1, 55];
    torch.modifier_index[0] = V_LIGHT;
    torch.modifier_value[0] = 55;

    let outcome = execute_item_driver(
        &mut npc,
        &mut torch,
        ItemDriverRequest::Driver {
            driver: IDR_LABTORCH,
            item_id: ItemId(7),
            character_id: CharacterId(2),
            spec: 0,
        },
        22,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::LightChanged {
            item_id: ItemId(7),
            character_id: CharacterId(2),
            schedule_after_ticks: None,
        }
    );
    assert_eq!(torch.sprite, 100);
    assert_eq!(torch.driver_data[0], 0);
    assert_eq!(torch.modifier_value[0], 0);
}

#[test]
fn lab3_berry_driver_decodes_yellow_white_and_brown() {
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(8));
    let request = ItemDriverRequest::Driver {
        driver: IDR_LAB3_PLANT,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    let mut yellow = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB3_PLANT);
    yellow.carried_by = Some(CharacterId(1));
    yellow.driver_data = vec![5, 3, 4];
    assert_eq!(
        execute_item_driver(&mut character, &mut yellow, request, 22, false),
        ItemDriverOutcome::Lab3YellowBerry {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            duration_ticks: 45 * TICKS_PER_SECOND,
            installed: false,
        }
    );

    let mut brown = yellow.clone();
    brown.driver_data = vec![11];
    assert_eq!(
        execute_item_driver(&mut character, &mut brown, request, 22, false),
        ItemDriverOutcome::Lab3BrownBerry {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            duration_ticks: 10 * TICKS_PER_SECOND,
            installed: false,
        }
    );

    let mut white = yellow;
    white.driver_data = vec![6, 1, 2];
    assert_eq!(
        execute_item_driver(&mut character, &mut white, request, 22, false),
        ItemDriverOutcome::Lab3WhiteBerry {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            light_power: 40,
            started_emit: false,
            installed: false,
        }
    );

    let mut light = white.clone();
    light.driver_data = vec![10];
    let mut timer_character = character.clone();
    timer_character.id = CharacterId(0);
    let timer_request = ItemDriverRequest::Driver {
        driver: IDR_LAB3_PLANT,
        item_id: ItemId(8),
        character_id: CharacterId(0),
        spec: 0,
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut light,
            timer_request,
            22,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::Lab3WhiteBerryLightTick {
            item_id: ItemId(8),
            destroyed: false,
        }
    );
}

#[test]
fn labexit_timer_animates_reschedules_and_expires() {
    let mut timer_character = character(0);
    let mut gate = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LABEXIT);
    set_drdata_u32(&mut gate, 8, 23);
    let timer_context = ItemDriverContext {
        timer_call: true,
        ..ItemDriverContext::default()
    };
    let request = ItemDriverRequest::Driver {
        driver: IDR_LABEXIT,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut gate,
            request,
            2,
            false,
            &timer_context,
        ),
        ItemDriverOutcome::LabExitAnimating {
            item_id: ItemId(7),
            sprite: 1083,
            frame: 24,
            schedule_after_ticks: 2,
        }
    );
    assert_eq!(drdata_u32(&gate, 8), 24);

    set_drdata_u32(&mut gate, 8, 264);
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut gate,
            request,
            2,
            false,
            &timer_context,
        ),
        ItemDriverOutcome::LabExitExpired { item_id: ItemId(7) }
    );
}

#[test]
fn labexit_use_requires_owner_and_returns_area_exit() {
    let mut character = character(42);
    let mut gate = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LABEXIT);
    set_drdata_u32(&mut gate, 0, 41);
    set_drdata(&mut gate, 4, 9);
    set_drdata_u32(&mut gate, 8, 35);
    let request = ItemDriverRequest::Driver {
        driver: IDR_LABEXIT,
        item_id: ItemId(7),
        character_id: CharacterId(42),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut gate, request, 2, false),
        ItemDriverOutcome::LabExitWrongOwner {
            item_id: ItemId(7),
            character_id: CharacterId(42),
        }
    );

    set_drdata_u32(&mut gate, 0, 42);
    assert_eq!(
        execute_item_driver(&mut character, &mut gate, request, 2, false),
        ItemDriverOutcome::LabExitUse {
            item_id: ItemId(7),
            character_id: CharacterId(42),
            lab_nr: 9,
            frame: 227,
            target_area: 3,
            target_x: 183,
            target_y: 199,
        }
    );
    assert_eq!(drdata_u32(&gate, 8), 227);
}

#[test]
fn labentrance_selects_next_unsolved_legacy_lab() {
    let mut character = character(42);
    character.level = 25;
    let mut entrance = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LABENTRANCE);
    let request = ItemDriverRequest::Driver {
        driver: IDR_LABENTRANCE,
        item_id: ItemId(7),
        character_id: CharacterId(42),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut entrance,
            request,
            3,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::Teleport {
            item_id: ItemId(7),
            character_id: CharacterId(42),
            x: 27,
            y: 242,
            area_id: 22,
            stop_driver: true,
            quiet: false,
        }
    );

    let context = ItemDriverContext {
        lab_solved_bits: 1_u64 << 10,
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut entrance,
            request,
            3,
            false,
            &context,
        ),
        ItemDriverOutcome::Teleport {
            item_id: ItemId(7),
            character_id: CharacterId(42),
            x: 69,
            y: 105,
            area_id: 22,
            stop_driver: true,
            quiet: false,
        }
    );
}

#[test]
fn labentrance_ports_level_gate_and_all_solved_feedback_boundary() {
    let mut character = character(42);
    character.level = 11;
    let mut entrance = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LABENTRANCE);
    let request = ItemDriverRequest::Driver {
        driver: IDR_LABENTRANCE,
        item_id: ItemId(7),
        character_id: CharacterId(42),
        spec: 0,
    };
    let context = ItemDriverContext {
        lab_solved_bits: 1_u64 << 10,
        ..ItemDriverContext::default()
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut entrance,
            request,
            3,
            false,
            &context,
        ),
        ItemDriverOutcome::LabEntranceTooLow {
            item_id: ItemId(7),
            character_id: CharacterId(42),
            required_level: 12,
        }
    );

    character.level = 30;
    let context = ItemDriverContext {
        lab_solved_bits: (1_u64 << 10)
            | (1_u64 << 15)
            | (1_u64 << 20)
            | (1_u64 << 25)
            | (1_u64 << 30),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut entrance,
            request,
            3,
            false,
            &context,
        ),
        ItemDriverOutcome::LabEntranceSolvedAll {
            item_id: ItemId(7),
            character_id: CharacterId(42),
        }
    );
}

#[test]
fn lab2_water_timer_initializes_kind_from_legacy_sprites() {
    for (sprite, expected_kind) in [
        (11008, 2),
        (11010, 2),
        (20793, 1),
        (20796, 1),
        (11011, 3),
        (11012, 4),
        (11013, 5),
    ] {
        let mut timer = character(0);
        let mut water = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB2_WATER);
        water.sprite = sprite;

        let outcome = execute_item_driver(
            &mut timer,
            &mut water,
            ItemDriverRequest::Driver {
                driver: IDR_LAB2_WATER,
                item_id: ItemId(8),
                character_id: CharacterId(0),
                spec: 0,
            },
            22,
            false,
        );

        assert_eq!(outcome, ItemDriverOutcome::Noop);
        assert_eq!(water.driver_data[0], expected_kind);
    }
}

#[test]
fn lab2_water_well_and_altar_return_typed_outcomes() {
    let mut actor = character(1);
    let mut well = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB2_WATER);
    well.driver_data = vec![1];

    assert_eq!(
        execute_item_driver(
            &mut actor,
            &mut well,
            ItemDriverRequest::Driver {
                driver: IDR_LAB2_WATER,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            22,
            false,
        ),
        ItemDriverOutcome::Lab2WaterWell {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    actor.cursor_item = Some(ItemId(99));
    assert_eq!(
        execute_item_driver(
            &mut actor,
            &mut well,
            ItemDriverRequest::Driver {
                driver: IDR_LAB2_WATER,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            22,
            false,
        ),
        ItemDriverOutcome::Lab2WaterCursorOccupied {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    let mut altar = item(9, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB2_WATER);
    altar.driver_data = vec![2];
    assert_eq!(
        execute_item_driver(
            &mut actor,
            &mut altar,
            ItemDriverRequest::Driver {
                driver: IDR_LAB2_WATER,
                item_id: ItemId(9),
                character_id: CharacterId(1),
                spec: 0,
            },
            22,
            false,
        ),
        ItemDriverOutcome::Lab2WaterAltar {
            item_id: ItemId(9),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn lab2_stepaction_timer_clears_marker_sprite() {
    let mut timer = character(0);
    let mut step = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB2_STEPACTION);
    step.sprite = 500;
    step.driver_data = vec![1];

    let outcome = execute_item_driver(
        &mut timer,
        &mut step,
        ItemDriverRequest::Driver {
            driver: IDR_LAB2_STEPACTION,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        },
        22,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab2StepActionClear { item_id: ItemId(8) }
    );
    assert_eq!(step.sprite, 0);
}

#[test]
fn lab2_stepaction_daemon_warning_requires_player_facing_up() {
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    actor.dir = Direction::Left as u8;
    let mut step = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB2_STEPACTION);
    step.x = 100;
    step.y = 120;
    step.driver_data = vec![1];
    let request = ItemDriverRequest::Driver {
        driver: IDR_LAB2_STEPACTION,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut step, request, 22, false),
        ItemDriverOutcome::Noop
    );

    actor.dir = Direction::Up as u8;
    assert_eq!(
        execute_item_driver(&mut actor, &mut step, request, 22, false),
        ItemDriverOutcome::Lab2StepActionDaemonWarning {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            x: 100,
            y: 115,
        }
    );
}

#[test]
fn lab2_stepaction_daemon_check_requires_player() {
    let mut actor = character(1);
    let mut step = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB2_STEPACTION);
    step.driver_data = vec![2];
    let request = ItemDriverRequest::Driver {
        driver: IDR_LAB2_STEPACTION,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut step, request, 22, false),
        ItemDriverOutcome::Noop
    );

    actor.flags.insert(CharacterFlags::PLAYER);
    assert_eq!(
        execute_item_driver(&mut actor, &mut step, request, 22, false),
        ItemDriverOutcome::Lab2StepActionDaemonCheck {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn lab2_grave_closed_timer_is_handled_like_legacy_module() {
    let mut timer = character(0);
    let mut grave = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB2_GRAVE);
    grave.driver_data = vec![0; 16];
    let request = ItemDriverRequest::Driver {
        driver: IDR_LAB2_GRAVE,
        item_id: ItemId(8),
        character_id: CharacterId(0),
        spec: 0,
    };

    let outcome = execute_item_driver_with_context(
        &mut timer,
        &mut grave,
        request,
        22,
        false,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(outcome, ItemDriverOutcome::Noop);
    assert_eq!(
        legacy_item_driver_return_code(Some(IDR_LAB2_GRAVE), &outcome),
        1
    );
}

#[test]
fn lab2_grave_empty_open_timer_requests_close() {
    let mut timer = character(0);
    let mut grave = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB2_GRAVE);
    grave.driver_data = vec![0; 16];
    grave.driver_data[4..8].copy_from_slice(&(-1_i32).to_le_bytes());
    grave.driver_data[8..12].copy_from_slice(&(-1_i32).to_le_bytes());
    let request = ItemDriverRequest::Driver {
        driver: IDR_LAB2_GRAVE,
        item_id: ItemId(8),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer,
            &mut grave,
            request,
            22,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::Lab2GraveClose { item_id: ItemId(8) }
    );
}

#[test]
fn lab2_grave_live_open_timer_checks_undead_serial() {
    let mut timer = character(0);
    let mut grave = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB2_GRAVE);
    grave.driver_data = vec![0; 16];
    grave.driver_data[4..8].copy_from_slice(&77_i32.to_le_bytes());
    grave.driver_data[8..12].copy_from_slice(&123_i32.to_le_bytes());
    let request = ItemDriverRequest::Driver {
        driver: IDR_LAB2_GRAVE,
        item_id: ItemId(8),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer,
            &mut grave,
            request,
            22,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::Lab2GraveCheckOpen {
            item_id: ItemId(8),
            undead_id: CharacterId(77),
            undead_serial: 123,
            schedule_after_ticks: TICKS_PER_SECOND * 5,
        }
    );
}

#[test]
fn lab2_grave_is_area22_guarded_and_player_use_opens_closed_grave() {
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    let mut grave = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB2_GRAVE);
    let request = ItemDriverRequest::Driver {
        driver: IDR_LAB2_GRAVE,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut grave, request, 1, false),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_LAB2_GRAVE,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            required_area: 22,
        }
    );

    assert_eq!(
        execute_item_driver(&mut actor, &mut grave, request, 22, false),
        ItemDriverOutcome::Lab2GraveOpen {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            fixed_item: 0,
        }
    );

    grave.driver_data = vec![0; 16];
    grave.driver_data[4..8].copy_from_slice(&77_i32.to_le_bytes());
    grave.driver_data[8..12].copy_from_slice(&123_i32.to_le_bytes());
    assert_eq!(
        execute_item_driver(&mut actor, &mut grave, request, 22, false),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn lab2_grave_player_use_preserves_fixed_special_item_kind() {
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    let mut grave = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB2_GRAVE);
    grave.driver_data = vec![0; 16];
    grave.driver_data[0] = 6;
    let request = ItemDriverRequest::Driver {
        driver: IDR_LAB2_GRAVE,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut grave, request, 22, false),
        ItemDriverOutcome::Lab2GraveOpen {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            fixed_item: 6,
        }
    );
}

#[test]
fn lab2_grave_clue_books_return_typed_book_outcome() {
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    let mut grave_book = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB2_GRAVE);
    grave_book.driver_data = vec![0; 16];
    grave_book.driver_data[0] = 3;
    let request = ItemDriverRequest::Driver {
        driver: IDR_LAB2_GRAVE,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut grave_book, request, 22, false),
        ItemDriverOutcome::Lab2GraveClueBook {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            book: 3,
        }
    );

    actor.flags.remove(CharacterFlags::PLAYER);
    assert_eq!(
        execute_item_driver(&mut actor, &mut grave_book, request, 22, false),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn lab2_regenerate_timer_decodes_legacy_spell_data() {
    let mut timer = character(0);
    let mut spell = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB2_REGENERATE);
    set_drdata(&mut spell, 0, 12);
    set_drdata(&mut spell, 1, 64);
    set_drdata_u32(&mut spell, 4, 3);
    set_drdata_u32(&mut spell, 8, 120);

    let outcome = execute_item_driver_with_context(
        &mut timer,
        &mut spell,
        ItemDriverRequest::Driver {
            driver: IDR_LAB2_REGENERATE,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        },
        22,
        false,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab2RegenerateTick {
            item_id: ItemId(8),
            target_id: CharacterId(3),
            start_tick: 120,
            regen_percent: 64,
            schedule_after_ticks: 12,
        }
    );
}
