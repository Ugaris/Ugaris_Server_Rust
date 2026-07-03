use super::*;

#[test]
fn arkhata_key_assemble_ports_legacy_combinations() {
    let mut character = character(1);
    character.cursor_item = Some(ItemId(9));
    let mut key = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_ARKHATA);
    key.template_id = IID_ARKHATA_AKEY12;
    key.carried_by = Some(CharacterId(1));
    set_drdata(&mut key, 0, 2);
    let request = ItemDriverRequest::Driver {
        driver: IDR_ARKHATA,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(IID_ARKHATA_AKEY1, 0x0100_00CA);
    assert_eq!(IID_ARKHATA_AKEY, 0x3B00_0089);
    assert_eq!(
        execute_item_driver(&mut character, &mut key, request, 37, false),
        ItemDriverOutcome::ArkhataKeyDoesNotFit {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    let mut context = ItemDriverContext {
        cursor_template_id: Some(IID_ARKHATA_AKEY3),
        ..Default::default()
    };
    assert_eq!(
        execute_item_driver_with_context(&mut character, &mut key, request, 37, false, &context),
        ItemDriverOutcome::ArkhataKeyAssemble {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(9),
            result_template_id: IID_ARKHATA_AKEY,
            result_sprite: 13413,
            final_key: true,
        }
    );

    key.template_id = IID_ARKHATA_AKEY1;
    context.cursor_template_id = Some(IID_ARKHATA_AKEY2);
    assert_eq!(
        execute_item_driver_with_context(&mut character, &mut key, request, 37, false, &context),
        ItemDriverOutcome::ArkhataKeyAssemble {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(9),
            result_template_id: IID_ARKHATA_AKEY12,
            result_sprite: 13421,
            final_key: false,
        }
    );
}

#[test]
fn arkhata_pool_dispatch_ports_cursor_gates() {
    let mut character = character(1);
    let mut pool = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_ARKHATA);
    set_drdata(&mut pool, 0, 0);
    let request = ItemDriverRequest::Driver {
        driver: IDR_ARKHATA,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(IID_ARKHATA_SCROLL1, 0x0100_00C2);
    assert_eq!(
        execute_item_driver(&mut character, &mut pool, request, 37, false),
        ItemDriverOutcome::ArkhataPoolNeedsCursor {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    character.cursor_item = Some(ItemId(9));
    let wrong_context = ItemDriverContext {
        cursor_template_id: Some(0x0100_00C3),
        ..Default::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut pool,
            request,
            37,
            false,
            &wrong_context
        ),
        ItemDriverOutcome::ArkhataPoolWrongCursor {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(9),
        }
    );

    let scroll_context = ItemDriverContext {
        cursor_template_id: Some(IID_ARKHATA_SCROLL1),
        ..Default::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut pool,
            request,
            37,
            false,
            &scroll_context
        ),
        ItemDriverOutcome::ArkhataPool {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(9),
        }
    );
}

#[test]
fn arkhata_stopwatch_dispatch_is_timer_only_and_reschedules() {
    let mut character = character(0);
    let mut stopwatch = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_ARKHATA);
    stopwatch.carried_by = Some(CharacterId(7));
    set_drdata(&mut stopwatch, 0, 1);
    let request = ItemDriverRequest::Driver {
        driver: IDR_ARKHATA,
        item_id: ItemId(8),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut stopwatch, request, 37, false),
        ItemDriverOutcome::ArkhataStopwatch {
            item_id: ItemId(8),
            character_id: CharacterId(7),
            schedule_after_ticks: 10,
        }
    );

    character.id = CharacterId(7);
    assert_eq!(
        execute_item_driver(&mut character, &mut stopwatch, request, 37, false),
        ItemDriverOutcome::Noop
    );

    character.id = CharacterId(0);
    stopwatch.carried_by = None;
    assert_eq!(
        execute_item_driver(&mut character, &mut stopwatch, request, 37, false),
        ItemDriverOutcome::Noop
    );
}
