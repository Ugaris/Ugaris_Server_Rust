use super::*;

#[test]
fn bonebridge_driver_requires_full_area18_bone_cursor_and_ports_timer_boundary() {
    let mut actor = character(1);
    actor.cursor_item = Some(ItemId(9));
    let mut bridge = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BONEBRIDGE);
    let request = ItemDriverRequest::Driver {
        driver: IDR_BONEBRIDGE,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut bridge,
            request,
            18,
            false,
            &ItemDriverContext {
                cursor_template_id: Some(IID_AREA18_BONE),
                cursor_drdata0: Some(4),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::Noop
    );

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut bridge,
            request,
            18,
            false,
            &ItemDriverContext {
                cursor_template_id: Some(IID_AREA18_BONE),
                cursor_drdata0: Some(5),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::BoneBridgePlace {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(9),
        }
    );

    bridge.driver_data = vec![0, 1];
    let mut timer_character = character(0);
    let timer_request = ItemDriverRequest::Driver {
        driver: IDR_BONEBRIDGE,
        item_id: ItemId(8),
        character_id: CharacterId(0),
        spec: 0,
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut bridge,
            timer_request,
            18,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::BoneBridgeTimerTick { item_id: ItemId(8) }
    );
}

#[test]
fn boneholder_driver_inserts_and_expires_owned_runes() {
    let mut actor = character(1);
    actor.cursor_item = Some(ItemId(9));
    let mut holder = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BONEHOLDER);
    let request = ItemDriverRequest::Driver {
        driver: IDR_BONEHOLDER,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(IDR_BONEHOLDER, 91);
    assert_eq!(IID_AREA18_RUNE1, 0x0100_0078);
    assert_eq!(IID_AREA18_RUNE9, 0x0100_0080);
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut holder,
            request,
            18,
            false,
            &ItemDriverContext {
                cursor_template_id: Some(IID_AREA18_RUNE1 + 3),
                current_tick: 100,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::BoneHolderInsertRune {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(9),
            rune: 4,
            owner_character_id: 1,
            placed_tick: 100,
            schedule_after_ticks: 2881,
        }
    );
    assert_eq!(drdata(&holder, 0), 4);
    assert_eq!(drdata_u32(&holder, 8), 1);
    assert_eq!(drdata_u32(&holder, 12), 100);

    let mut timer_character = character(0);
    let timer_request = ItemDriverRequest::Driver {
        driver: IDR_BONEHOLDER,
        item_id: ItemId(8),
        character_id: CharacterId(0),
        spec: 0,
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut holder,
            timer_request,
            18,
            false,
            &ItemDriverContext {
                timer_call: true,
                current_tick: 100 + 2880,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::BoneHolderExpired { item_id: ItemId(8) }
    );
    assert_eq!(drdata(&holder, 0), 0);
}

#[test]
fn boneholder_driver_ports_rejection_remove_and_activation_boundaries() {
    let mut actor = character(1);
    let mut holder = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BONEHOLDER);
    let request = ItemDriverRequest::Driver {
        driver: IDR_BONEHOLDER,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    actor.cursor_item = Some(ItemId(9));
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut holder,
            request,
            18,
            false,
            &ItemDriverContext {
                cursor_template_id: Some(IID_AREA18_BONE),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::BoneHolderBadCursor {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    actor.cursor_item = None;
    assert_eq!(
        execute_item_driver(&mut actor, &mut holder, request, 18, false),
        ItemDriverOutcome::BoneHolderEmptyTouch {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    set_drdata(&mut holder, 0, 7);
    set_drdata_u32(&mut holder, 8, 2);
    assert_eq!(
        execute_item_driver(&mut actor, &mut holder, request, 18, false),
        ItemDriverOutcome::BoneHolderWrongOwner {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    set_drdata_u32(&mut holder, 8, 1);
    assert_eq!(
        execute_item_driver(&mut actor, &mut holder, request, 18, false),
        ItemDriverOutcome::BoneHolderRemoveRune {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            rune: 7,
        }
    );
    assert_eq!(drdata(&holder, 0), 0);

    set_drdata(&mut holder, 1, 3);
    assert_eq!(
        execute_item_driver(&mut actor, &mut holder, request, 18, false),
        ItemDriverOutcome::BoneHolderActivate {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            last_holder: true,
        }
    );
}

#[test]
fn bonewall_driver_ports_area18_timer_and_active_guards() {
    let mut actor = character(1);
    let mut wall = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BONEWALL);
    let request = ItemDriverRequest::Driver {
        driver: IDR_BONEWALL,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut wall, request, 18, false),
        ItemDriverOutcome::BoneWallTick {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    wall.driver_data = vec![1];
    assert_eq!(
        execute_item_driver(&mut actor, &mut wall, request, 18, false),
        ItemDriverOutcome::Noop
    );

    let mut timer_character = character(0);
    let timer_request = ItemDriverRequest::Driver {
        driver: IDR_BONEWALL,
        item_id: ItemId(8),
        character_id: CharacterId(0),
        spec: 0,
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut wall,
            timer_request,
            18,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::BoneWallTick {
            item_id: ItemId(8),
            character_id: CharacterId(0),
        }
    );

    wall.driver_data = vec![0];
    assert!(matches!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut wall,
            timer_request,
            17,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::LibloadAreaBlocked {
            required_area: 18,
            ..
        }
    ));
}

#[test]
fn bonehint_driver_initializes_carried_diary_hint() {
    let mut character = character(1);
    let mut diary = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BONEHINT);
    diary.carried_by = Some(character.id);
    set_drdata(&mut diary, 0, 7);
    let request = ItemDriverRequest::Driver {
        driver: IDR_BONEHINT,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(IDR_BONEHINT, 94);
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut diary,
            request,
            18,
            false,
            &ItemDriverContext {
                bone_hint_nr: Some(3),
                bone_hint_pos: Some(2),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::BoneHint {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            level: 7,
            nr: 3,
            pos: 2,
        }
    );
    assert_eq!(drdata(&diary, 1), 1);
    assert_eq!(drdata(&diary, 2), 3);
    assert_eq!(drdata(&diary, 3), 2);
}

#[test]
fn bonehint_driver_requires_carried_item() {
    let mut character = character(1);
    let mut diary = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BONEHINT);
    let request = ItemDriverRequest::Driver {
        driver: IDR_BONEHINT,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut diary, request, 18, false),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn boneladder_driver_ports_paired_ladder_offsets() {
    let mut character = character(1);
    let mut ladder = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BONELADDER);
    ladder.x = 100;
    ladder.y = 80;
    let request = ItemDriverRequest::Driver {
        driver: IDR_BONELADDER,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(IDR_BONELADDER, 90);
    assert_eq!(
        execute_item_driver(&mut character, &mut ladder, request, 18, false),
        ItemDriverOutcome::Teleport {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            x: 104,
            y: 83,
            area_id: 0,
            stop_driver: false,
            quiet: false,
        }
    );

    set_drdata(&mut ladder, 0, 1);
    assert_eq!(
        execute_item_driver(&mut character, &mut ladder, request, 18, false),
        ItemDriverOutcome::Teleport {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            x: 96,
            y: 77,
            area_id: 0,
            stop_driver: false,
            quiet: false,
        }
    );
}

#[test]
fn boneladder_driver_preserves_area18_libload_guard() {
    let mut character = character(1);
    let mut ladder = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BONELADDER);
    let request = ItemDriverRequest::Driver {
        driver: IDR_BONELADDER,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut ladder, request, 1, false),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_BONELADDER,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            required_area: 18,
        }
    );
}
