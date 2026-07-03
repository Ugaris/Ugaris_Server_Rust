use super::*;

#[test]
fn execute_door_driver_returns_toggle_or_key_block() {
    let mut character = character(1);
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DOOR);
    door.x = 10;
    door.y = 11;

    let request = ItemDriverRequest::Driver {
        driver: IDR_DOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut door, request, 1, false),
        ItemDriverOutcome::DoorToggle {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    door.driver_data = vec![0, 1, 0, 0, 0];
    assert_eq!(
        execute_item_driver(&mut character, &mut door, request, 1, false),
        ItemDriverOutcome::BlockedByRequirements {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    door.x = 0;
    door.driver_data.clear();
    assert_eq!(
        execute_item_driver(&mut character, &mut door, request, 1, false),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn execute_door_driver_accepts_key_context() {
    let mut character = character(1);
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DOOR);
    door.x = 10;
    door.y = 11;
    door.driver_data = vec![1, 0x44, 0x33, 0x22, 0x11];
    let request = ItemDriverRequest::Driver {
        driver: IDR_DOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };
    let context = ItemDriverContext {
        door_key: Some(DoorKeyAccess {
            key_id: 0x1122_3344,
            name: "Copper Key".to_string(),
            source: DoorKeySource::Keyring,
        }),
        cursor_template_id: None,
        ..ItemDriverContext::default()
    };

    assert_eq!(
        execute_item_driver_with_context(&mut character, &mut door, request, 1, false, &context,),
        ItemDriverOutcome::KeyedDoorToggle {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            key_id: 0x1122_3344,
            source: DoorKeySource::Keyring,
            locking: true,
        }
    );
}

#[test]
fn execute_double_door_driver_returns_typed_toggle() {
    let mut character = character(1);
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DOUBLE_DOOR);
    door.x = 10;
    door.y = 11;
    let request = ItemDriverRequest::Driver {
        driver: IDR_DOUBLE_DOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut door, request, 1, false),
        ItemDriverOutcome::DoubleDoorToggle {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    door.x = 0;
    assert_eq!(
        execute_item_driver(&mut character, &mut door, request, 1, false),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn execute_teleport_door_driver_moves_to_opposite_side() {
    let mut character = character(1);
    character.x = 9;
    character.y = 10;
    character.level = 5;
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_TELE_DOOR);
    door.x = 10;
    door.y = 10;
    door.driver_data = vec![0, 10];

    let request = ItemDriverRequest::Driver {
        driver: IDR_TELE_DOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut door, request, 1, false),
        ItemDriverOutcome::TeleportDoor {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            x: 11,
            y: 10,
        }
    );

    door.driver_data[0] = 2;
    assert_eq!(
        execute_item_driver(&mut character, &mut door, request, 1, false),
        ItemDriverOutcome::Noop
    );

    door.driver_data = vec![0, 4];
    assert_eq!(
        execute_item_driver(&mut character, &mut door, request, 1, false),
        ItemDriverOutcome::BlockedByRequirements {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}
