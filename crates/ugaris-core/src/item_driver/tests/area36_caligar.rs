use super::*;

#[test]
fn caligar_training_driver_ports_watch_lesson_boundary() {
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    let mut training = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_CALIGAR);
    training.driver_data = vec![1, 2];
    let request = ItemDriverRequest::Driver {
        driver: IDR_CALIGAR,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(IDR_CALIGAR, 144);
    assert_eq!(
        execute_item_driver(&mut actor, &mut training, request, 36, false),
        ItemDriverOutcome::CaligarTraining {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            lesson: 2,
        }
    );

    training.driver_data = vec![1, 9];
    assert_eq!(
        execute_item_driver(&mut actor, &mut training, request, 36, false),
        ItemDriverOutcome::Noop
    );

    training.driver_data = vec![2, 1];
    assert_eq!(
        execute_item_driver(&mut actor, &mut training, request, 36, false),
        ItemDriverOutcome::CaligarWeightMove {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    training.driver_data = vec![3, 0];
    assert_eq!(
        execute_item_driver(&mut actor, &mut training, request, 36, false),
        ItemDriverOutcome::CaligarWeightDoor {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(
        legacy_item_driver_return_code(
            Some(IDR_CALIGAR),
            &ItemDriverOutcome::CaligarWeightDoorLocked {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            },
        ),
        2
    );

    training.driver_data = vec![5, 0];
    assert_eq!(
        execute_item_driver(&mut actor, &mut training, request, 36, false),
        ItemDriverOutcome::CaligarGunProjectile {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            direction: 1,
            schedule_after_ticks: 12,
        }
    );

    training.driver_data = vec![9, 0];
    assert_eq!(
        execute_item_driver(&mut actor, &mut training, request, 36, false),
        ItemDriverOutcome::CaligarGunProjectile {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            direction: 5,
            schedule_after_ticks: 12,
        }
    );

    training.driver_data = vec![11, 0];
    assert_eq!(
        execute_item_driver(&mut actor, &mut training, request, 36, false),
        ItemDriverOutcome::Extinguish {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            extinguished: false,
        }
    );

    training.driver_data = vec![12, 3];
    assert_eq!(
        execute_item_driver(&mut actor, &mut training, request, 36, false),
        ItemDriverOutcome::CaligarSkellyDoor {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            door_index: 3,
        }
    );
    assert_eq!(
        legacy_item_driver_return_code(
            Some(IDR_CALIGAR),
            &ItemDriverOutcome::CaligarSkellyDoorLocked {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            },
        ),
        2
    );
    assert_eq!(
        legacy_item_driver_return_code(
            Some(IDR_CALIGAR),
            &ItemDriverOutcome::CaligarSkellyDoorBusy {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            },
        ),
        2
    );

    let mut timer_character = character(0);
    assert_eq!(
        execute_item_driver(&mut timer_character, &mut training, request, 36, false),
        ItemDriverOutcome::Noop
    );

    training.driver_data = vec![2, 1];
    let timer_request = ItemDriverRequest::Driver {
        driver: IDR_CALIGAR,
        item_id: ItemId(8),
        character_id: CharacterId(0),
        spec: 0,
    };
    assert_eq!(
        execute_item_driver(
            &mut timer_character,
            &mut training,
            timer_request,
            36,
            false
        ),
        ItemDriverOutcome::CaligarWeightTimer { item_id: ItemId(8) }
    );
}

#[test]
fn caligar_key_assembly_ports_piece_matrix() {
    let mut actor = character(1);
    actor.cursor_item = Some(ItemId(9));
    let mut key = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_CALIGAR);
    key.driver_data = vec![10];
    key.sprite = 13414;
    let request = ItemDriverRequest::Driver {
        driver: IDR_CALIGAR,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };
    let mut context = ItemDriverContext {
        cursor_template_id: Some(IID_CALIGAR_PALACE_KEY_PART),
        cursor_sprite: Some(13415),
        ..ItemDriverContext::default()
    };

    assert_eq!(IID_CALIGAR_PALACE_KEY_PART, 0x0100_00B3);
    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut key, request, 36, false, &context),
        ItemDriverOutcome::CaligarKeyAssemble {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(9),
            result_sprite: 13421,
            final_key: false,
        }
    );

    key.sprite = 13420;
    context.cursor_sprite = Some(13414);
    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut key, request, 36, false, &context),
        ItemDriverOutcome::CaligarKeyAssemble {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(9),
            result_sprite: 0,
            final_key: true,
        }
    );

    context.cursor_template_id = Some(IID_AREA11_PALACEKEYPART);
    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut key, request, 36, false, &context),
        ItemDriverOutcome::CaligarKeyNeedsCursor {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    context.cursor_template_id = Some(IID_CALIGAR_PALACE_KEY_PART);
    context.cursor_sprite = Some(13416);
    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut key, request, 36, false, &context),
        ItemDriverOutcome::CaligarKeyDoesNotFit {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn caligar_flame_uses_legacy_flamethrower_timer_path() {
    let mut timer_character = character(0);
    let mut flame = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_CALIGARFLAME);
    flame.driver_data = vec![2, 5, 0, 4];

    let outcome = execute_item_driver_with_context(
        &mut timer_character,
        &mut flame,
        ItemDriverRequest::Driver {
            driver: IDR_CALIGARFLAME,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        36,
        false,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(flame.driver_data[0], 1);
    assert_eq!(flame.driver_data[2], 1);
    assert_eq!(flame.sprite, 1);
    assert_eq!(flame.modifier_index[4], V_LIGHT);
    assert_eq!(flame.modifier_value[4], 250);
    assert_eq!(
        outcome,
        ItemDriverOutcome::FlameThrowerPulse {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            direction: 5,
            schedule_after_ticks: 1,
        }
    );

    let mut player = character(1);
    assert_eq!(
        execute_item_driver_with_context(
            &mut player,
            &mut flame,
            ItemDriverRequest::Driver {
                driver: IDR_CALIGARFLAME,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            36,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::Noop
    );
}
