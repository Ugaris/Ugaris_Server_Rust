use super::*;

#[test]
fn swamparm_timer_animates_only_when_triggered() {
    let mut actor = character(0);
    let mut arm = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SWAMPARM);
    arm.sprite = 21000;
    let request = ItemDriverRequest::Driver {
        driver: IDR_SWAMPARM,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut arm,
            request,
            15,
            false,
            &ItemDriverContext {
                timer_call: true,
                swamp_arm_triggered: Some(false),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::SwampArmPulse {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            damage_now: false,
            schedule_after_ticks: 1,
        }
    );
    assert_eq!(arm.sprite, 21000);
    assert_eq!(arm.driver_data.first().copied().unwrap_or_default(), 0);

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut arm,
            request,
            15,
            false,
            &ItemDriverContext {
                timer_call: true,
                swamp_arm_triggered: Some(true),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::SwampArmPulse {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            damage_now: false,
            schedule_after_ticks: 1,
        }
    );
    assert_eq!(arm.sprite, 21001);
    assert_eq!(arm.driver_data[0], 1);
}

#[test]
fn swamparm_frame_twelve_reports_damage_and_frame_sixteen_resets() {
    let mut actor = character(0);
    let mut arm = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SWAMPARM);
    arm.sprite = 21011;
    arm.driver_data = vec![11];
    let request = ItemDriverRequest::Driver {
        driver: IDR_SWAMPARM,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut arm,
            request,
            15,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::SwampArmPulse {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            damage_now: true,
            schedule_after_ticks: 1,
        }
    );
    assert_eq!(arm.driver_data[0], 12);

    arm.driver_data[0] = 15;
    arm.sprite = 21015;
    execute_item_driver_with_context(
        &mut actor,
        &mut arm,
        request,
        15,
        false,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );
    assert_eq!(arm.driver_data[0], 0);
    assert_eq!(arm.sprite, 21000);
}

#[test]
fn swamparm_preserves_area15_and_timer_only_guards() {
    let mut actor = character(0);
    let mut arm = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SWAMPARM);
    let request = ItemDriverRequest::Driver {
        driver: IDR_SWAMPARM,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut arm,
            request,
            1,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_SWAMPARM,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            required_area: 15,
        }
    );

    let mut actor = character(1);
    let request = ItemDriverRequest::Driver {
        driver: IDR_SWAMPARM,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut arm,
            request,
            15,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn swampwhisp_initializes_and_moves_down_at_frame_twelve() {
    let mut actor = character(0);
    let mut whisp = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SWAMPWHISP);
    whisp.x = 10;
    whisp.y = 20;
    whisp.driver_data = vec![11];
    let request = ItemDriverRequest::Driver {
        driver: IDR_SWAMPWHISP,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    let outcome = execute_item_driver_with_context(
        &mut actor,
        &mut whisp,
        request,
        15,
        false,
        &ItemDriverContext {
            timer_call: true,
            swamp_whisp_move_succeeds: Some(true),
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(whisp.x, 10);
    assert_eq!(whisp.y, 21);
    assert_eq!(whisp.driver_data[0], 1);
    assert_eq!(whisp.driver_data[1], 10);
    assert_eq!(whisp.driver_data[2], 20);
    assert_eq!(whisp.driver_data[3], SWAMPWHISP_CIRCLE_LEFT);
    assert_eq!(whisp.sprite, 20935);
    assert_eq!(
        outcome,
        ItemDriverOutcome::SwampWhispPulse {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            moved_from: Some((10, 20)),
            moved_to: Some((10, 21)),
            light_changed: false,
            schedule_after_ticks: 2,
        }
    );
}

#[test]
fn swampwhisp_blocks_move_and_circles_opposite_direction() {
    let mut actor = character(0);
    let mut whisp = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SWAMPWHISP);
    whisp.x = 10;
    whisp.y = 20;
    whisp.driver_data = vec![11, 10, 20, crate::direction::Direction::Down as u8];
    let request = ItemDriverRequest::Driver {
        driver: IDR_SWAMPWHISP,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    let outcome = execute_item_driver_with_context(
        &mut actor,
        &mut whisp,
        request,
        15,
        false,
        &ItemDriverContext {
            timer_call: true,
            swamp_whisp_move_succeeds: Some(false),
            ..ItemDriverContext::default()
        },
    );

    assert_eq!((whisp.x, whisp.y), (10, 20));
    assert_eq!(whisp.driver_data[0], 12);
    assert_eq!(whisp.driver_data[3], SWAMPWHISP_CIRCLE_RIGHT);
    assert_eq!(
        outcome,
        ItemDriverOutcome::SwampWhispPulse {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            moved_from: None,
            moved_to: None,
            light_changed: false,
            schedule_after_ticks: 2,
        }
    );
}

#[test]
fn swampwhisp_daylight_darkens_and_reschedules_slowly() {
    let mut actor = character(0);
    let mut whisp = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SWAMPWHISP);
    whisp.sprite = 20940;
    whisp.modifier_value[0] = 100;
    whisp.driver_data = vec![6, 10, 20, SWAMPWHISP_CIRCLE_LEFT];
    let request = ItemDriverRequest::Driver {
        driver: IDR_SWAMPWHISP,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    let outcome = execute_item_driver_with_context(
        &mut actor,
        &mut whisp,
        request,
        15,
        false,
        &ItemDriverContext {
            timer_call: true,
            daylight: 60,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(whisp.driver_data[3], SWAMPWHISP_DARK);
    assert_eq!(whisp.sprite, 0);
    assert_eq!(whisp.modifier_value[0], 0);
    assert_eq!(
        outcome,
        ItemDriverOutcome::SwampWhispPulse {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            moved_from: None,
            moved_to: None,
            light_changed: true,
            schedule_after_ticks: TICKS_PER_SECOND,
        }
    );
}

#[test]
fn swampspawn_initializes_and_waits_without_player() {
    let mut actor = character(0);
    let mut spawn = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SWAMPSPAWN);
    spawn.sprite = 20999;
    let request = ItemDriverRequest::Driver {
        driver: IDR_SWAMPSPAWN,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    let outcome = execute_item_driver_with_context(
        &mut actor,
        &mut spawn,
        request,
        15,
        false,
        &ItemDriverContext {
            timer_call: true,
            swamp_spawn_player_close: Some(false),
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(spawn.sprite, 0);
    assert_eq!(spawn.driver_data[1], 1);
    assert_eq!(drdata_u32(&spawn, 16), 20991);
    assert_eq!(
        outcome,
        ItemDriverOutcome::SwampSpawnPulse {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            schedule_after_ticks: TICKS_PER_SECOND,
        }
    );
}

#[test]
fn swampspawn_animates_to_template_spawn() {
    let mut actor = character(0);
    let mut spawn = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SWAMPSPAWN);
    spawn.x = 12;
    spawn.y = 13;
    spawn.sprite = 0;
    spawn.driver_data = vec![2, 1, 3];
    set_drdata_u32(&mut spawn, 16, 21000);
    let request = ItemDriverRequest::Driver {
        driver: IDR_SWAMPSPAWN,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    let outcome = execute_item_driver_with_context(
        &mut actor,
        &mut spawn,
        request,
        15,
        false,
        &ItemDriverContext {
            timer_call: true,
            current_tick: 20_000,
            swamp_spawn_player_close: Some(true),
            swamp_spawn_ground_sprite: Some(59423),
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(spawn.driver_data[2], 0);
    assert_eq!(spawn.sprite, 0);
    assert_eq!(
        outcome,
        ItemDriverOutcome::SwampSpawn {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            template: "swamp29n",
            x: 12,
            y: 13,
            schedule_after_ticks: 3,
        }
    );
}

#[test]
fn swampspawn_respects_live_spawn_and_recent_cooldown() {
    let mut actor = character(0);
    let mut spawn = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SWAMPSPAWN);
    spawn.driver_data = vec![0, 1, 0];
    set_drdata_u32(&mut spawn, 12, 9_000);
    let request = ItemDriverRequest::Driver {
        driver: IDR_SWAMPSPAWN,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    let recent = execute_item_driver_with_context(
        &mut actor,
        &mut spawn,
        request,
        15,
        false,
        &ItemDriverContext {
            timer_call: true,
            current_tick: 9_500,
            swamp_spawn_player_close: Some(true),
            ..ItemDriverContext::default()
        },
    );
    assert_eq!(spawn.driver_data[2], 0);
    assert!(matches!(
        recent,
        ItemDriverOutcome::SwampSpawnPulse {
            schedule_after_ticks: TICKS_PER_SECOND,
            ..
        }
    ));

    let live = execute_item_driver_with_context(
        &mut actor,
        &mut spawn,
        request,
        15,
        false,
        &ItemDriverContext {
            timer_call: true,
            current_tick: 30_000,
            swamp_spawn_live: Some(true),
            swamp_spawn_player_close: Some(true),
            ..ItemDriverContext::default()
        },
    );
    assert_eq!(spawn.driver_data[2], 0);
    assert!(matches!(
        live,
        ItemDriverOutcome::SwampSpawnPulse {
            schedule_after_ticks: TICKS_PER_SECOND,
            ..
        }
    ));
}
