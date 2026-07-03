use super::*;

#[test]
fn palace_bomb_toggles_carried_state_and_stores_owner() {
    let mut actor = character(1234);
    let mut bomb = item(
        7,
        ItemFlags::USED | ItemFlags::TAKE | ItemFlags::USE,
        0,
        IDR_PALACEBOMB,
    );
    bomb.carried_by = Some(CharacterId(1234));
    bomb.sprite = 500;
    let request = ItemDriverRequest::Driver {
        driver: IDR_PALACEBOMB,
        item_id: ItemId(7),
        character_id: CharacterId(1234),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut bomb,
            request,
            11,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::PalaceBombToggled {
            item_id: ItemId(7),
            character_id: CharacterId(1234),
            active: true,
        }
    );
    assert_eq!(bomb.driver_data[0], 1);
    assert_eq!(drdata_u32(&bomb, 1), 1234);
    assert_eq!(bomb.sprite, 501);

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut bomb,
            request,
            11,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::PalaceBombToggled {
            item_id: ItemId(7),
            character_id: CharacterId(1234),
            active: false,
        }
    );
    assert_eq!(bomb.driver_data[0], 0);
    assert_eq!(bomb.sprite, 500);
}

#[test]
fn palace_bomb_timer_arms_ground_bomb_then_exposes_explosion_outcome() {
    let mut timer = character(0);
    let mut bomb = item(
        7,
        ItemFlags::USED | ItemFlags::TAKE | ItemFlags::USE,
        0,
        IDR_PALACEBOMB,
    );
    bomb.driver_data = vec![1, 0x39, 0x30, 0, 0];
    bomb.sprite = 500;
    bomb.x = 10;
    bomb.y = 11;
    let request = ItemDriverRequest::Driver {
        driver: IDR_PALACEBOMB,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer,
            &mut bomb,
            request,
            11,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::PalaceBombTimer {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            armed: true,
            schedule_after_ticks: TICKS_PER_SECOND * 5,
        }
    );
    assert_eq!(bomb.driver_data[0], 2);
    assert_eq!(bomb.sprite, 501);
    assert!(bomb.flags.contains(ItemFlags::STEPACTION));
    assert!(!bomb.flags.intersects(ItemFlags::TAKE | ItemFlags::USE));

    let mut actor = character(99);
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut bomb,
            ItemDriverRequest::Driver {
                driver: IDR_PALACEBOMB,
                item_id: ItemId(7),
                character_id: CharacterId(99),
                spec: 0,
            },
            11,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::PalaceBombExplode {
            item_id: ItemId(7),
            character_id: CharacterId(99),
            owner_id: 12345,
            x: 10,
            y: 11,
        }
    );
}

#[test]
fn palace_cap_timer_reschedules_for_carried_cap_only_on_timer_calls() {
    let mut timer = character(0);
    let mut cap = item(7, ItemFlags::USED, 0, IDR_PALACECAP);
    cap.carried_by = Some(CharacterId(2));
    cap.driver_data = vec![1];
    let request = ItemDriverRequest::Driver {
        driver: IDR_PALACECAP,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer,
            &mut cap,
            request,
            11,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::PalaceCapTimer {
            item_id: ItemId(7),
            character_id: CharacterId(2),
            active: true,
            schedule_after_ticks: TICKS_PER_SECOND / 4,
        }
    );

    let mut actor = character(2);
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut cap,
            ItemDriverRequest::Driver {
                driver: IDR_PALACECAP,
                item_id: ItemId(7),
                character_id: CharacterId(2),
                spec: 0,
            },
            11,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::Noop,
    );
    assert_eq!(
        legacy_item_driver_return_code(Some(IDR_PALACECAP), &ItemDriverOutcome::Noop),
        1
    );
    assert_eq!(
        legacy_item_driver_return_code(Some(IDR_PALACEBOMB), &ItemDriverOutcome::Noop),
        1
    );
}

#[test]
fn palace_bomb_and_cap_keep_area_11_libload_guard() {
    let mut actor = character(1);
    let mut bomb = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_PALACEBOMB);
    let outcome = execute_item_driver_with_context(
        &mut actor,
        &mut bomb,
        ItemDriverRequest::Driver {
            driver: IDR_PALACEBOMB,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
        false,
        &ItemDriverContext::default(),
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_PALACEBOMB,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            required_area: 11,
        }
    );
    assert_eq!(
        legacy_item_driver_return_code(Some(IDR_PALACEBOMB), &outcome),
        1
    );
}

#[test]
fn palace_door_requires_key_and_starts_opening() {
    let mut actor = character(1);
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_PALACEDOOR);
    let request = ItemDriverRequest::Driver {
        driver: IDR_PALACEDOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut door,
            request,
            11,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::PalaceDoorKeyRequired {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    let context = ItemDriverContext {
        has_area11_palace_key: true,
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut door, request, 11, false, &context),
        ItemDriverOutcome::PalaceDoorTick {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            state: 3,
            frame: 0,
            sprite: door.sprite,
            set_tmoveblock: None,
            schedule_after_ticks: Some(2),
        }
    );
    assert_eq!(door.driver_data[1], 3);
}

#[test]
fn palace_door_timer_animates_open_and_close() {
    let mut timer = character(0);
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_PALACEDOOR);
    let request = ItemDriverRequest::Driver {
        driver: IDR_PALACEDOOR,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };
    let context = ItemDriverContext {
        timer_call: true,
        ..ItemDriverContext::default()
    };

    door.driver_data = vec![14, 3];
    assert_eq!(
        execute_item_driver_with_context(&mut timer, &mut door, request, 11, false, &context),
        ItemDriverOutcome::PalaceDoorTick {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            state: 1,
            frame: 15,
            sprite: 15211,
            set_tmoveblock: Some(false),
            schedule_after_ticks: Some(TICKS_PER_SECOND * 10),
        }
    );

    assert_eq!(
        execute_item_driver_with_context(&mut timer, &mut door, request, 11, false, &context),
        ItemDriverOutcome::PalaceDoorTick {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            state: 2,
            frame: 15,
            sprite: 15211,
            set_tmoveblock: Some(true),
            schedule_after_ticks: Some(3),
        }
    );

    door.driver_data = vec![1, 2];
    assert_eq!(
        execute_item_driver_with_context(&mut timer, &mut door, request, 11, false, &context),
        ItemDriverOutcome::PalaceDoorTick {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            state: 0,
            frame: 0,
            sprite: 15196,
            set_tmoveblock: None,
            schedule_after_ticks: None,
        }
    );
}

#[test]
fn islena_door_ports_room_gates_and_teleports() {
    let mut actor = character(1);
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_ISLENADOOR);
    let request = ItemDriverRequest::Driver {
        driver: IDR_ISLENADOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    actor.x = 144;
    actor.y = 56;
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut door,
            request,
            11,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::TeleportDoor {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            x: 144,
            y: 58,
        }
    );

    actor.x = 144;
    actor.y = 58;
    let busy = ItemDriverContext {
        islena_room_has_player: true,
        islena_present: true,
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut door, request, 11, false, &busy),
        ItemDriverOutcome::IslenaDoorBusy {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut door,
            request,
            11,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::IslenaDoorRespawning {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    let resting = ItemDriverContext {
        islena_present: true,
        islena_resting: true,
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut door, request, 11, false, &resting),
        ItemDriverOutcome::IslenaDoorResting {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    let ready = ItemDriverContext {
        islena_present: true,
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut door, request, 11, false, &ready),
        ItemDriverOutcome::TeleportDoor {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            x: 143,
            y: 55,
        }
    );

    assert!(matches!(
        execute_item_driver_with_context(&mut actor, &mut door, request, 1, false, &ready),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_ISLENADOOR,
            required_area: 11,
            ..
        }
    ));
}

#[test]
fn execute_palace_key_driver_splits_and_combines_legacy_sprites() {
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(7));
    let mut key_part = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_PALACEKEY);
    key_part.carried_by = Some(CharacterId(1));
    key_part.template_id = IID_AREA11_PALACEKEYPART;
    key_part.sprite = 51021;
    let request = ItemDriverRequest::Driver {
        driver: IDR_PALACEKEY,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut key_part, request, 11, false),
        ItemDriverOutcome::PalaceKeySplit {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            cursor_part_sprite: 51015,
            carried_part_sprite: 51016,
        }
    );

    character.cursor_item = Some(ItemId(8));
    key_part.sprite = 51015;
    let context = ItemDriverContext {
        cursor_template_id: Some(IID_AREA11_PALACEKEYPART),
        cursor_sprite: Some(51039),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut key_part,
            request,
            11,
            false,
            &context,
        ),
        ItemDriverOutcome::PalaceKeyCombine {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(8),
            result_sprite: 51014,
            final_key: true,
        }
    );
}

#[test]
fn execute_palace_key_driver_reports_legacy_failures() {
    let mut character = character(1);
    let mut key_part = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_PALACEKEY);
    key_part.carried_by = Some(CharacterId(1));
    key_part.template_id = IID_AREA11_PALACEKEYPART;
    key_part.sprite = 51015;
    let request = ItemDriverRequest::Driver {
        driver: IDR_PALACEKEY,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut key_part, request, 11, false),
        ItemDriverOutcome::PalaceKeyNeedsCursor {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    character.cursor_item = Some(ItemId(8));
    assert_eq!(
        execute_item_driver(&mut character, &mut key_part, request, 11, false),
        ItemDriverOutcome::PalaceKeyDoesNotFit {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn palace_gate_only_dispatches_for_zero_character_timer_calls() {
    let mut timer_character = character(0);
    let mut gate = item(7, ItemFlags::USED, 0, IDR_PALACEGATE);
    let request = ItemDriverRequest::Driver {
        driver: IDR_PALACEGATE,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut gate,
            request,
            3,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::PalaceGateTick {
            item_id: ItemId(7),
            opened: false,
            closed: false,
            blocked: false,
        }
    );

    assert_eq!(
        execute_item_driver(&mut character(1), &mut gate, request, 3, false),
        ItemDriverOutcome::Noop
    );
}
