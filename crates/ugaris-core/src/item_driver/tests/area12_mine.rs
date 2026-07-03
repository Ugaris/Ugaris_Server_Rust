use super::*;

#[test]
fn execute_mine_gateway_key_driver_combines_key_bits() {
    let mut character = character(1);
    character.cursor_item = Some(ItemId(8));
    let mut key = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_MINEGATEWAYKEY);
    key.driver_data = vec![1];
    let request = ItemDriverRequest::Driver {
        driver: IDR_MINEGATEWAYKEY,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut key,
            request,
            1,
            false,
            &ItemDriverContext {
                cursor_driver: Some(IDR_MINEGATEWAYKEY),
                cursor_drdata0: Some(14),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::MineGatewayKeyAssemble {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(8),
            combined_bits: 15,
        }
    );

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut key,
            request,
            1,
            false,
            &ItemDriverContext {
                cursor_driver: Some(IDR_SHRIKEAMULET),
                cursor_drdata0: Some(2),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::MineGatewayKeyDoesNotFit {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn execute_minewall_timer_initializes_legacy_sprite_and_collapse_boundary() {
    let mut timer = character(0);
    let mut wall = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_MINEWALL);
    wall.x = 10;
    wall.y = 12;
    wall.sprite = 1;
    let request = ItemDriverRequest::Driver {
        driver: IDR_MINEWALL,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut timer, &mut wall, request, 12, false),
        ItemDriverOutcome::MineWallInitialized {
            item_id: ItemId(7),
            sprite: 15078,
        }
    );
    assert_eq!(wall.sprite, 15078);
    assert_eq!(drdata(&wall, 4), 1);

    set_drdata(&mut wall, 3, 8);
    assert_eq!(
        execute_item_driver(&mut timer, &mut wall, request, 12, false),
        ItemDriverOutcome::MineWallCollapse {
            item_id: ItemId(7),
            schedule_after_ticks: TICKS_PER_SECOND as u32,
        }
    );
}

#[test]
fn execute_minewall_player_gates_and_dig_mutation() {
    let mut character = character(1);
    character.endurance = POWERSCALE;
    character.professions[profession::MINER] = 25;
    let mut wall = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_MINEWALL);
    wall.sprite = 15070;
    let request = ItemDriverRequest::Driver {
        driver: IDR_MINEWALL,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    character.cursor_item = Some(ItemId(8));
    assert_eq!(
        execute_item_driver(&mut character, &mut wall, request, 12, false),
        ItemDriverOutcome::MineWallCursorOccupied {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    character.cursor_item = None;
    character.endurance = POWERSCALE - 1;
    assert_eq!(
        execute_item_driver(&mut character, &mut wall, request, 12, false),
        ItemDriverOutcome::MineWallExhausted {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    character.endurance = POWERSCALE;
    set_drdata(&mut wall, 3, 7);
    assert_eq!(
        execute_item_driver(&mut character, &mut wall, request, 12, false),
        ItemDriverOutcome::MineWallDig {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            endurance_delta: 0,
            stage: 8,
            opened: true,
        }
    );
    assert_eq!(drdata(&wall, 3), 8);
    assert_eq!(drdata(&wall, 5), 0);
    assert_eq!(wall.sprite, 15071);
}

#[test]
fn execute_mine_gateway_driver_requires_key_and_decodes_destination() {
    let mut character = character(1);
    character.flags.insert(CharacterFlags::PLAYER);
    let mut gateway = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_MINEGATEWAY);
    set_drdata_u16(&mut gateway, 0, 42);
    set_drdata_u16(&mut gateway, 2, 43);
    set_drdata_u16(&mut gateway, 4, 12);
    let request = ItemDriverRequest::Driver {
        driver: IDR_MINEGATEWAY,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut gateway,
            request,
            12,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::MineGatewayNeedsKey {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut gateway,
            request,
            12,
            false,
            &ItemDriverContext {
                has_mine_gateway_key: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::MineGateway {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            x: 42,
            y: 43,
            area_id: 12,
        }
    );
}

#[test]
fn execute_mine_gateway_driver_reports_bad_destination() {
    let mut character = character(1);
    character.flags.insert(CharacterFlags::PLAYER);
    let mut gateway = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_MINEGATEWAY);
    set_drdata_u16(&mut gateway, 0, 0);
    set_drdata_u16(&mut gateway, 2, 43);
    set_drdata_u16(&mut gateway, 4, 12);

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut gateway,
            ItemDriverRequest::Driver {
                driver: IDR_MINEGATEWAY,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            12,
            false,
            &ItemDriverContext {
                has_mine_gateway_key: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::MineGatewayBug {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            x: 0,
            y: 43,
            area_id: 12,
        }
    );
}

#[test]
fn execute_mine_key_door_requires_2000_gold_cursor() {
    let mut character = character(1);
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_MINEKEYDOOR);
    let request = ItemDriverRequest::Driver {
        driver: IDR_MINEKEYDOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut door,
            request,
            12,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::MineKeyDoorNeedsGold {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    character.cursor_item = Some(ItemId(8));
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut door,
            request,
            12,
            false,
            &ItemDriverContext {
                cursor_driver: Some(IDR_ENHANCE),
                cursor_drdata0: Some(2),
                cursor_drdata1_u32: Some(1999),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::MineKeyDoorNeedsGold {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn execute_mine_key_door_decodes_golem_number() {
    let mut character = character(1);
    character.cursor_item = Some(ItemId(8));
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_MINEKEYDOOR);
    set_drdata(&mut door, 0, 4);
    let request = ItemDriverRequest::Driver {
        driver: IDR_MINEKEYDOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut door,
            request,
            12,
            false,
            &ItemDriverContext {
                cursor_driver: Some(IDR_ENHANCE),
                cursor_drdata0: Some(2),
                cursor_drdata1_u32: Some(2000),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::MineKeyDoor {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(8),
            golem_nr: 4,
        }
    );
}

#[test]
fn mine_door_ports_player_teleport_boundary() {
    let mut actor = character(1);
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_MINEDOOR);
    door.driver_data = vec![3, 0, 7, 1];
    let request = ItemDriverRequest::Driver {
        driver: IDR_MINEDOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut door,
            request,
            12,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::MineDoorMissingTarget {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut door,
            request,
            12,
            false,
            &ItemDriverContext {
                mine_door_target: Some((50, 60, 1)),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::MineDoorTeleport {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            target_x: 49,
            target_y: 60,
            fallback_x: 230,
            fallback_y: 240,
        }
    );

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut door,
            request,
            12,
            false,
            &ItemDriverContext {
                timer_call: true,
                mine_door_target: Some((50, 60, 3)),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::MineDoorTimer { item_id: ItemId(7) }
    );
}
