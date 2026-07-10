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

#[test]
fn deathfibrin_shrine_gives_a_new_staff_when_cursor_is_empty() {
    let mut player = character(1);
    let mut shrine = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DEATHFIBRIN);
    shrine.sprite = 10428;

    let outcome = execute_item_driver(
        &mut player,
        &mut shrine,
        ItemDriverRequest::Driver {
            driver: IDR_DEATHFIBRIN,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::DeathfibrinShrineGive {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn deathfibrin_shrine_refuses_when_cursor_is_occupied() {
    let mut player = character(1);
    player.cursor_item = Some(ItemId(99));
    let mut shrine = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DEATHFIBRIN);
    shrine.sprite = 10428;

    let outcome = execute_item_driver(
        &mut player,
        &mut shrine,
        ItemDriverRequest::Driver {
            driver: IDR_DEATHFIBRIN,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::DeathfibrinShrineOccupied {
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn deathfibrin_staff_needs_carrying() {
    let mut player = character(1);
    let mut staff = item(
        7,
        ItemFlags::USED | ItemFlags::TAKE | ItemFlags::USE,
        0,
        IDR_DEATHFIBRIN,
    );
    staff.sprite = 10418;

    let outcome = execute_item_driver(
        &mut player,
        &mut staff,
        ItemDriverRequest::Driver {
            driver: IDR_DEATHFIBRIN,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::DeathfibrinNeedsCarry {
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn deathfibrin_staff_reports_no_master_nearby() {
    let mut player = character(1);
    let mut staff = item(
        7,
        ItemFlags::USED | ItemFlags::TAKE | ItemFlags::USE,
        0,
        IDR_DEATHFIBRIN,
    );
    staff.sprite = 10418;
    staff.carried_by = Some(CharacterId(1));

    let outcome = execute_item_driver_with_context(
        &mut player,
        &mut staff,
        ItemDriverRequest::Driver {
            driver: IDR_DEATHFIBRIN,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
        false,
        &ItemDriverContext {
            deathfibrin_master: None,
            deathfibrin_tile_light: 7,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::DeathfibrinNoMaster {
            character_id: CharacterId(1),
            tile_light: 7,
        }
    );
}

#[test]
fn deathfibrin_staff_first_strike_lazily_initializes_to_full_charge() {
    let mut player = character(1);
    let mut staff = item(
        7,
        ItemFlags::USED | ItemFlags::TAKE | ItemFlags::USE,
        0,
        IDR_DEATHFIBRIN,
    );
    staff.sprite = 10418;
    staff.carried_by = Some(CharacterId(1));

    let outcome = execute_item_driver_with_context(
        &mut player,
        &mut staff,
        ItemDriverRequest::Driver {
            driver: IDR_DEATHFIBRIN,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
        false,
        &ItemDriverContext {
            deathfibrin_master: Some(CharacterId(9)),
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::DeathfibrinStrike {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            master_id: CharacterId(9),
            item_name: outcome_item_name(&staff.name),
            vanished: false,
        }
    );
    // Amount is 10000 - 1000 = 9000 (90%).
    assert_eq!(
        u32::from_le_bytes(staff.driver_data[0..4].try_into().unwrap()),
        9000
    );
    assert_eq!(staff.description, "Staff containing 90% Deathfibrin");
}

#[test]
fn deathfibrin_staff_strike_that_empties_the_charge_vanishes() {
    let mut player = character(1);
    let mut staff = item(
        7,
        ItemFlags::USED | ItemFlags::TAKE | ItemFlags::USE,
        0,
        IDR_DEATHFIBRIN,
    );
    staff.sprite = 10418;
    staff.carried_by = Some(CharacterId(1));
    // Already initialized, one strike away from empty.
    staff.driver_data = vec![0, 0, 0, 0, 1];
    staff.driver_data[0..4].copy_from_slice(&1000u32.to_le_bytes());

    let outcome = execute_item_driver_with_context(
        &mut player,
        &mut staff,
        ItemDriverRequest::Driver {
            driver: IDR_DEATHFIBRIN,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
        false,
        &ItemDriverContext {
            deathfibrin_master: Some(CharacterId(9)),
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::DeathfibrinStrike {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            master_id: CharacterId(9),
            item_name: outcome_item_name(&staff.name),
            vanished: true,
        }
    );
}

#[test]
fn lab3_special_zero_character_is_a_no_op() {
    let mut timer = character(0);
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB3_SPECIAL);
    door.driver_data = vec![1, 5, 250, 1];

    let outcome = execute_item_driver(
        &mut timer,
        &mut door,
        ItemDriverRequest::Driver {
            driver: IDR_LAB3_SPECIAL,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        22,
        false,
    );

    assert_eq!(outcome, ItemDriverOutcome::Noop);
}

#[test]
fn lab3_special_teleport_door_decodes_signed_offsets_and_password_flag() {
    let mut actor = character(1);
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB3_SPECIAL);
    // type=1 (teleport door), dx=5, dy=-6 (250 as signed byte), not
    // password protected.
    door.driver_data = vec![1, 5, 250, 0];

    let outcome = execute_item_driver(
        &mut actor,
        &mut door,
        ItemDriverRequest::Driver {
            driver: IDR_LAB3_SPECIAL,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab3TeleportDoor {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            dx: 5,
            dy: -6,
            password_protected: false,
            extinguished_count: 0,
        }
    );
}

#[test]
fn lab3_special_password_door_blocks_until_guard_talkstep_reaches_20() {
    let mut actor = character(1);
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB3_SPECIAL);
    door.driver_data = vec![1, 1, 0, 1];
    let request = ItemDriverRequest::Driver {
        driver: IDR_LAB3_SPECIAL,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    // No context at all (defaults to "not opened yet", same as C's
    // freshly-allocated `struct lab_ppd`).
    assert_eq!(
        execute_item_driver(&mut actor, &mut door, request, 22, false),
        ItemDriverOutcome::Lab3TeleportDoorLocked {
            character_id: CharacterId(1),
        }
    );

    // Mid-challenge (1..6) still blocks.
    let mid_challenge = ItemDriverContext {
        lab3_guard_talkstep: Some(4),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut door, request, 22, false, &mid_challenge),
        ItemDriverOutcome::Lab3TeleportDoorLocked {
            character_id: CharacterId(1),
        }
    );

    // Passworded (20) opens the door.
    let passworded = ItemDriverContext {
        lab3_guard_talkstep: Some(20),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut door, request, 22, false, &passworded),
        ItemDriverOutcome::Lab3TeleportDoor {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            dx: 1,
            dy: 0,
            password_protected: true,
            extinguished_count: 0,
        }
    );
}

#[test]
fn lab3_special_note_giving_skeleton_blocks_on_occupied_cursor() {
    let mut actor = character(1);
    actor.cursor_item = Some(ItemId(99));
    let mut skeleton_item = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB3_SPECIAL);
    skeleton_item.driver_data = vec![2, 3];

    let outcome = execute_item_driver(
        &mut actor,
        &mut skeleton_item,
        ItemDriverRequest::Driver {
            driver: IDR_LAB3_SPECIAL,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab3NoteGivingBlocked {
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn lab3_special_note_giving_skeleton_returns_note_value_when_free() {
    let mut actor = character(1);
    let mut skeleton_item = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB3_SPECIAL);
    skeleton_item.driver_data = vec![2, 20];

    let outcome = execute_item_driver(
        &mut actor,
        &mut skeleton_item,
        ItemDriverRequest::Driver {
            driver: IDR_LAB3_SPECIAL,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab3NoteGivingSkeleton {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            note_value: 20,
        }
    );
}

#[test]
fn lab3_special_note_read_returns_its_own_note_value() {
    let mut actor = character(1);
    let mut note = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB3_SPECIAL);
    note.driver_data = vec![3, 21];

    let outcome = execute_item_driver(
        &mut actor,
        &mut note,
        ItemDriverRequest::Driver {
            driver: IDR_LAB3_SPECIAL,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab3NoteRead {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            note_value: 21,
        }
    );
}

#[test]
fn lab4_item_zero_character_is_a_no_op() {
    let mut timer = character(0);
    let mut key_fireplace = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB4_ITEM);
    key_fireplace.driver_data = vec![1];

    let outcome = execute_item_driver(
        &mut timer,
        &mut key_fireplace,
        ItemDriverRequest::Driver {
            driver: IDR_LAB4_ITEM,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        22,
        false,
    );

    assert_eq!(outcome, ItemDriverOutcome::Noop);
}

#[test]
fn lab4_item_wrong_drdata_is_a_no_op() {
    let mut player = character(1);
    let mut key_fireplace = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB4_ITEM);
    key_fireplace.driver_data = vec![0];

    let outcome = execute_item_driver(
        &mut player,
        &mut key_fireplace,
        ItemDriverRequest::Driver {
            driver: IDR_LAB4_ITEM,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
        false,
    );

    assert_eq!(outcome, ItemDriverOutcome::Noop);
}

#[test]
fn lab4_item_fireplace_key_blocked_when_cursor_occupied() {
    let mut player = character(1);
    player.cursor_item = Some(ItemId(99));
    let mut key_fireplace = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB4_ITEM);
    key_fireplace.driver_data = vec![1];

    let outcome = execute_item_driver(
        &mut player,
        &mut key_fireplace,
        ItemDriverRequest::Driver {
            driver: IDR_LAB4_ITEM,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab4FireplaceKeyBlocked {
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn lab4_item_fireplace_key_gives_key_when_cursor_empty() {
    let mut player = character(1);
    let mut key_fireplace = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB4_ITEM);
    key_fireplace.driver_data = vec![1];

    let outcome = execute_item_driver(
        &mut player,
        &mut key_fireplace,
        ItemDriverRequest::Driver {
            driver: IDR_LAB4_ITEM,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab4FireplaceKeyGive {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn deathfibrin_timer_call_is_a_no_op() {
    let mut timer = character(0);
    let mut staff = item(
        7,
        ItemFlags::USED | ItemFlags::TAKE | ItemFlags::USE,
        0,
        IDR_DEATHFIBRIN,
    );
    staff.sprite = 10418;
    staff.carried_by = Some(CharacterId(1));

    let outcome = execute_item_driver_with_context(
        &mut timer,
        &mut staff,
        ItemDriverRequest::Driver {
            driver: IDR_DEATHFIBRIN,
            item_id: ItemId(7),
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

    assert_eq!(outcome, ItemDriverOutcome::Noop);
}

// -- lab5_item -------------------------------------------------------------

fn lab5_item(id: u32, drdata: Vec<u8>) -> Item {
    let mut lab5 = item(id, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB5_ITEM);
    lab5.driver_data = drdata;
    lab5
}

fn lab5_request(item_id: u32, character_id: u32) -> ItemDriverRequest {
    ItemDriverRequest::Driver {
        driver: IDR_LAB5_ITEM,
        item_id: ItemId(item_id),
        character_id: CharacterId(character_id),
        spec: 0,
    }
}

#[test]
fn lab5_obelisk_fully_heals_and_reports_for_sound() {
    let mut actor = character(1);
    actor.hp = 1;
    actor.mana = 1;
    actor.endurance = 1;
    actor.lifeshield = 1;
    actor.values[0][CharacterValue::Hp as usize] = 100;
    actor.values[0][CharacterValue::Mana as usize] = 50;
    actor.values[0][CharacterValue::Endurance as usize] = 60;
    actor.values[0][CharacterValue::MagicShield as usize] = 30;
    let mut obelisk = lab5_item(7, vec![1]);

    let outcome = execute_item_driver(&mut actor, &mut obelisk, lab5_request(7, 1), 22, false);

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab5Obelisk {
            character_id: CharacterId(1)
        }
    );
    assert_eq!(actor.hp, 100 * POWERSCALE);
    assert_eq!(actor.mana, 50 * POWERSCALE);
    assert_eq!(actor.endurance, 60 * POWERSCALE);
    assert_eq!(actor.lifeshield, 30 * POWERSCALE);
}

#[test]
fn lab5_combopotion_heals_lifeshield_only_when_magicshield_present() {
    let mut actor = character(1);
    actor.values[0][CharacterValue::Hp as usize] = 100;
    actor.values[0][CharacterValue::Mana as usize] = 50;
    actor.values[0][CharacterValue::Endurance as usize] = 60;
    actor.values[0][CharacterValue::MagicShield as usize] = 30;
    // C reads `value[1][V_MAGICSHIELD]` for the gate, distinct from the
    // `value[0]` amount used for the actual heal.
    actor.values[1][CharacterValue::MagicShield as usize] = 1;
    let mut potion = lab5_item(7, vec![4]);

    let outcome = execute_item_driver(&mut actor, &mut potion, lab5_request(7, 1), 22, false);

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab5PotionDrunk {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(actor.hp, 100 * POWERSCALE);
    assert_eq!(actor.lifeshield, 30 * POWERSCALE);
}

#[test]
fn lab5_combopotion_skips_lifeshield_without_magicshield_gate() {
    let mut actor = character(1);
    actor.values[0][CharacterValue::MagicShield as usize] = 30;
    // `value[1]` (the gate) stays 0 even though `value[0]` is nonzero.
    let mut potion = lab5_item(7, vec![4]);

    execute_item_driver(&mut actor, &mut potion, lab5_request(7, 1), 22, false);

    assert_eq!(actor.lifeshield, 0);
}

#[test]
fn lab5_manapotion_only_restores_mana() {
    let mut actor = character(1);
    actor.hp = 5;
    actor.values[0][CharacterValue::Hp as usize] = 100;
    actor.values[0][CharacterValue::Mana as usize] = 50;
    let mut potion = lab5_item(7, vec![12]);

    let outcome = execute_item_driver(&mut actor, &mut potion, lab5_request(7, 1), 22, false);

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab5PotionDrunk {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(actor.mana, 50 * POWERSCALE);
    assert_eq!(actor.hp, 5, "manapotion must not touch hp");
}

#[test]
fn lab5_chestbox_blocks_on_occupied_cursor_or_already_open_sprite() {
    let mut actor = character(1);
    actor.cursor_item = Some(ItemId(99));
    let mut chest = lab5_item(7, vec![3, 1, 0, 0]);

    assert_eq!(
        execute_item_driver(&mut actor, &mut chest, lab5_request(7, 1), 22, false),
        ItemDriverOutcome::Noop
    );

    actor.cursor_item = None;
    chest.driver_data[3] = 1;
    assert_eq!(
        execute_item_driver(&mut actor, &mut chest, lab5_request(7, 1), 22, false),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn lab5_chestbox_reports_already_opened_from_context() {
    let mut actor = character(1);
    let mut chest = lab5_item(7, vec![3, 1, 0, 0]);
    let context = ItemDriverContext {
        lab5_chestbox_already_opened: true,
        ..ItemDriverContext::default()
    };

    let outcome = execute_item_driver_with_context(
        &mut actor,
        &mut chest,
        lab5_request(7, 1),
        22,
        false,
        &context,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab5ChestboxAlreadyOpened {
            character_id: CharacterId(1)
        }
    );
}

#[test]
fn lab5_chestbox_opens_and_marks_sprite_and_driver_data() {
    let mut actor = character(1);
    let mut chest = lab5_item(7, vec![3, 6, 0, 0]);
    chest.sprite = 500;

    let outcome = execute_item_driver(&mut actor, &mut chest, lab5_request(7, 1), 22, false);

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab5ChestboxOpen {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            reward: 6,
        }
    );
    assert_eq!(chest.driver_data[3], 1);
    assert_eq!(chest.sprite, 501);
}

#[test]
fn lab5_chestbox_timer_closes_only_when_open() {
    let mut timer = character(0);
    let mut closed_chest = lab5_item(7, vec![3, 1, 0, 0]);
    closed_chest.sprite = 500;
    assert_eq!(
        execute_item_driver(&mut timer, &mut closed_chest, lab5_request(7, 0), 22, false),
        ItemDriverOutcome::Noop
    );
    assert_eq!(closed_chest.sprite, 500);

    let mut open_chest = lab5_item(7, vec![3, 1, 0, 1]);
    open_chest.sprite = 501;
    assert_eq!(
        execute_item_driver(&mut timer, &mut open_chest, lab5_request(7, 0), 22, false),
        ItemDriverOutcome::Lab5ChestboxClose { item_id: ItemId(7) }
    );
    assert_eq!(open_chest.driver_data[3], 0);
    assert_eq!(open_chest.sprite, 500);
}

#[test]
fn lab5_nameplate_starts_ritual_when_untouched_and_hurts_otherwise() {
    let mut actor = character(1);
    let mut plate = lab5_item(7, vec![5, 2]);

    // ritualstate defaults to 0 (no context override).
    assert_eq!(
        execute_item_driver(&mut actor, &mut plate, lab5_request(7, 1), 22, false),
        ItemDriverOutcome::Lab5RitualStart {
            character_id: CharacterId(1),
            daemon: 2,
        }
    );

    let context = ItemDriverContext {
        lab5_ritual_state: Some(1),
        lab5_ritual_daemon: Some(1),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut plate,
            lab5_request(7, 1),
            22,
            false,
            &context,
        ),
        ItemDriverOutcome::Lab5RitualHurtAtItem {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            stored_daemon: 1,
        }
    );
}

#[test]
fn lab5_realnameplate_covers_nothing_progress_and_hurt() {
    let mut actor = character(1);
    let mut plate = lab5_item(7, vec![6, 2]);

    // ritualstate == 0: "Nothing happens.".
    assert_eq!(
        execute_item_driver(&mut actor, &mut plate, lab5_request(7, 1), 22, false),
        ItemDriverOutcome::Lab5RitualNothing {
            character_id: CharacterId(1)
        }
    );

    // ritualstate == 1 and matching daemon: progresses to state 2.
    let matching = ItemDriverContext {
        lab5_ritual_state: Some(1),
        lab5_ritual_daemon: Some(2),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut plate,
            lab5_request(7, 1),
            22,
            false,
            &matching,
        ),
        ItemDriverOutcome::Lab5RitualProgress {
            character_id: CharacterId(1),
            daemon: 2,
            new_state: 2,
        }
    );

    // ritualstate == 1 but mismatched daemon: hurts, using the stored
    // (not the plate's) daemon.
    let mismatched = ItemDriverContext {
        lab5_ritual_state: Some(1),
        lab5_ritual_daemon: Some(3),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut plate,
            lab5_request(7, 1),
            22,
            false,
            &mismatched,
        ),
        ItemDriverOutcome::Lab5RitualHurtAtItem {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            stored_daemon: 3,
        }
    );
}

#[test]
fn lab5_entrance_untouched_is_silent_progresses_or_hurts() {
    let mut actor = character(1);
    let mut entrance = lab5_item(7, vec![7, 2]);

    assert_eq!(
        execute_item_driver(&mut actor, &mut entrance, lab5_request(7, 1), 22, false),
        ItemDriverOutcome::Noop
    );

    let matching = ItemDriverContext {
        lab5_ritual_state: Some(2),
        lab5_ritual_daemon: Some(2),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut entrance,
            lab5_request(7, 1),
            22,
            false,
            &matching,
        ),
        ItemDriverOutcome::Lab5RitualProgress {
            character_id: CharacterId(1),
            daemon: 2,
            new_state: 3,
        }
    );

    // Wrong entrance (`drdata[1]==2`) additionally forces the "strange
    // power" message in the resolver.
    let mut forced_entrance = lab5_item(7, vec![7, 2]);
    let mismatched = ItemDriverContext {
        lab5_ritual_state: Some(3),
        lab5_ritual_daemon: Some(1),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut forced_entrance,
            lab5_request(7, 1),
            22,
            false,
            &mismatched,
        ),
        ItemDriverOutcome::Lab5EntranceRitualHurt {
            character_id: CharacterId(1),
            entrance_index: 2,
            stored_daemon: 1,
            forced_message: true,
        }
    );

    let mut plain_entrance = lab5_item(7, vec![7, 1]);
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut plain_entrance,
            lab5_request(7, 1),
            22,
            false,
            &mismatched,
        ),
        ItemDriverOutcome::Lab5EntranceRitualHurt {
            character_id: CharacterId(1),
            entrance_index: 1,
            stored_daemon: 1,
            forced_message: false,
        }
    );
}

#[test]
fn lab5_backdoor_always_reports_the_teleport_attempt() {
    let mut actor = character(1);
    let mut door = lab5_item(7, vec![8]);

    assert_eq!(
        execute_item_driver(&mut actor, &mut door, lab5_request(7, 1), 22, false),
        ItemDriverOutcome::Lab5Backdoor {
            character_id: CharacterId(1)
        }
    );
}

#[test]
fn lab5_gun_locked_then_fires_and_reloads() {
    let mut actor = character(1);
    let mut gun = lab5_item(7, vec![9, 0]);
    gun.x = 100;
    gun.y = 50;
    gun.sprite = 200;

    let outcome = execute_item_driver(&mut actor, &mut gun, lab5_request(7, 1), 22, false);
    assert_eq!(
        outcome,
        ItemDriverOutcome::FireballMachineProjectile {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            start_x: 102,
            start_y: 50,
            target_x: 160,
            target_y: 50,
            power: 100,
            schedule_after_ticks: Some(TICKS_PER_SECOND * 2 / 3),
        }
    );
    assert_eq!(gun.driver_data[1], 7);
    assert_eq!(gun.sprite, 207);

    // Locked while reloading.
    assert_eq!(
        execute_item_driver(&mut actor, &mut gun, lab5_request(7, 1), 22, false),
        ItemDriverOutcome::Lab5GunLocked {
            character_id: CharacterId(1)
        }
    );
}

#[test]
fn lab5_gun_reload_timer_decrements_and_reschedules_until_empty() {
    let mut timer = character(0);
    let mut gun = lab5_item(7, vec![9, 2]);
    gun.sprite = 207;

    let outcome = execute_item_driver(&mut timer, &mut gun, lab5_request(7, 0), 22, false);
    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab5GunReloadTick {
            item_id: ItemId(7),
            schedule_after_ticks: Some(TICKS_PER_SECOND * 2 / 3),
        }
    );
    assert_eq!(gun.driver_data[1], 1);
    assert_eq!(gun.sprite, 206);

    let outcome = execute_item_driver(&mut timer, &mut gun, lab5_request(7, 0), 22, false);
    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab5GunReloadTick {
            item_id: ItemId(7),
            schedule_after_ticks: None,
        }
    );
    assert_eq!(gun.driver_data[1], 0);
    assert_eq!(gun.sprite, 205);

    assert_eq!(
        execute_item_driver(&mut timer, &mut gun, lab5_request(7, 0), 22, false),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn lab5_pike_always_hurts_and_arms_once() {
    let mut actor = character(1);
    let mut pike = lab5_item(7, vec![10, 0]);
    pike.sprite = 300;

    let outcome = execute_item_driver(&mut actor, &mut pike, lab5_request(7, 1), 22, false);
    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab5PikeHurt {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            arming: true,
        }
    );
    assert_eq!(pike.driver_data[1], 1);
    assert_eq!(pike.sprite, 301);

    // Already armed: still hurts, but does not re-arm/re-schedule.
    let outcome = execute_item_driver(&mut actor, &mut pike, lab5_request(7, 1), 22, false);
    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab5PikeHurt {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            arming: false,
        }
    );
    assert_eq!(pike.sprite, 301);
}

#[test]
fn lab5_pike_timer_resets_only_when_armed() {
    let mut timer = character(0);
    let mut idle_pike = lab5_item(7, vec![10, 0]);
    idle_pike.sprite = 300;
    assert_eq!(
        execute_item_driver(&mut timer, &mut idle_pike, lab5_request(7, 0), 22, false),
        ItemDriverOutcome::Noop
    );

    let mut armed_pike = lab5_item(7, vec![10, 1]);
    armed_pike.sprite = 301;
    assert_eq!(
        execute_item_driver(&mut timer, &mut armed_pike, lab5_request(7, 0), 22, false),
        ItemDriverOutcome::Lab5PikeReset { item_id: ItemId(7) }
    );
    assert_eq!(armed_pike.driver_data[1], 0);
    assert_eq!(armed_pike.sprite, 300);
}

#[test]
fn lab5_no_potion_door_blocks_only_when_carrying_a_potion_from_the_west() {
    let mut actor = character(1);
    actor.x = 5;
    let mut door = lab5_item(7, vec![11]);
    door.x = 10;
    door.y = 20;

    let carrying_potion = ItemDriverContext {
        has_potion: true,
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut door,
            lab5_request(7, 1),
            22,
            false,
            &carrying_potion,
        ),
        ItemDriverOutcome::Lab5NoPotionDoorBlocked {
            character_id: CharacterId(1)
        }
    );

    assert_eq!(
        execute_item_driver(&mut actor, &mut door, lab5_request(7, 1), 22, false),
        ItemDriverOutcome::Lab5NoPotionDoorPass {
            character_id: CharacterId(1),
            target_x: 1,
            target_y: 13,
        }
    );

    actor.x = 15;
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut door,
            lab5_request(7, 1),
            22,
            false,
            &carrying_potion,
        ),
        ItemDriverOutcome::Lab5NoPotionDoorPass {
            character_id: CharacterId(1),
            target_x: 19,
            target_y: 27,
        }
    );
}
