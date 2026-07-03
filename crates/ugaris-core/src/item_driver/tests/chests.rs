use super::*;

#[test]
fn chestspawn_driver_ports_area2_spawn_and_timer_boundaries() {
    let mut actor = character(1);
    let mut spawner = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_CHESTSPAWN);
    spawner.x = 42;
    spawner.y = 43;
    spawner.driver_data = vec![0, 0, 0, 0, 0, 0, 0, 0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_CHESTSPAWN,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(IDR_CHESTSPAWN, 27);
    assert_eq!(
        execute_item_driver(&mut actor, &mut spawner, request, 2, false),
        ItemDriverOutcome::ChestSpawn {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            template: "normal_vampire",
            x: 42,
            y: 43,
            schedule_after_ticks: TICKS_PER_SECOND * 10,
        }
    );

    spawner.driver_data = vec![1, 0];
    assert_eq!(
        execute_item_driver(&mut actor, &mut spawner, request, 2, false),
        ItemDriverOutcome::Noop
    );

    spawner.driver_data = vec![0, 1, 0x34, 0x12, 0, 0, 0, 0];
    let mut timer_character = character(0);
    let timer_request = ItemDriverRequest::Driver {
        driver: IDR_CHESTSPAWN,
        item_id: ItemId(8),
        character_id: CharacterId(0),
        spec: 0,
    };
    assert_eq!(
        execute_item_driver(&mut timer_character, &mut spawner, timer_request, 2, false),
        ItemDriverOutcome::ChestSpawnCheck {
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spawned_character_id: CharacterId(0x1234),
            schedule_after_ticks: TICKS_PER_SECOND * 10,
        }
    );
}

#[test]
fn execute_chest_driver_returns_treasure_or_blocks() {
    let mut character = character(1);
    let mut chest = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_CHEST);
    chest.driver_data = vec![9];
    let request = ItemDriverRequest::Driver {
        driver: IDR_CHEST,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut chest, request, 1, false),
        ItemDriverOutcome::ChestTreasure {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            treasure_index: 9,
        }
    );

    character.cursor_item = Some(ItemId(99));
    assert_eq!(
        execute_item_driver(&mut character, &mut chest, request, 1, false),
        ItemDriverOutcome::BlockedByRequirements {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    character.cursor_item = None;
    chest.driver_data = vec![9, 1, 0, 0, 0];
    assert_eq!(
        execute_item_driver(&mut character, &mut chest, request, 1, false),
        ItemDriverOutcome::ChestTreasure {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            treasure_index: 9,
        }
    );
}

#[test]
fn execute_randchest_driver_returns_runtime_outcome_even_with_cursor_item() {
    let mut character = character(1);
    character.cursor_item = Some(ItemId(99));
    let mut chest = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_RANDCHEST);
    let request = ItemDriverRequest::Driver {
        driver: IDR_RANDCHEST,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut chest, request, 1, false),
        ItemDriverOutcome::RandomChest {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn execute_infinite_chest_maps_rune_kind_to_template() {
    let mut character = character(1);
    let mut chest = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_INFINITE_CHEST);
    chest.driver_data = vec![4];

    let outcome = execute_item_driver(
        &mut character,
        &mut chest,
        ItemDriverRequest::Driver {
            driver: IDR_INFINITE_CHEST,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::InfiniteChest {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            template: InfiniteChestTemplate::Rune4,
            key_name: None,
        }
    );
}

#[test]
fn execute_infinite_chest_requires_empty_cursor_before_key_checks() {
    let mut character = character(1);
    character.cursor_item = Some(ItemId(9));
    let mut chest = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_INFINITE_CHEST);
    chest.driver_data = vec![1, 0x44, 0x33, 0x22, 0x11];

    let outcome = execute_item_driver(
        &mut character,
        &mut chest,
        ItemDriverRequest::Driver {
            driver: IDR_INFINITE_CHEST,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::InfiniteChestCursorOccupied {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn execute_infinite_chest_requires_matching_key_when_configured() {
    let mut character = character(1);
    let mut chest = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_INFINITE_CHEST);
    chest.driver_data = vec![1, 0x44, 0x33, 0x22, 0x11];

    let missing = execute_item_driver(
        &mut character,
        &mut chest.clone(),
        ItemDriverRequest::Driver {
            driver: IDR_INFINITE_CHEST,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
        false,
    );
    assert_eq!(
        missing,
        ItemDriverOutcome::InfiniteChestKeyRequired {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    let outcome = execute_item_driver_with_context(
        &mut character,
        &mut chest,
        ItemDriverRequest::Driver {
            driver: IDR_INFINITE_CHEST,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
        false,
        &ItemDriverContext {
            door_key: Some(DoorKeyAccess {
                key_id: 0x1122_3344,
                name: "Palace Key".to_string(),
                source: DoorKeySource::Carried,
            }),
            ..ItemDriverContext::default()
        },
    );
    assert_eq!(
        outcome,
        ItemDriverOutcome::InfiniteChest {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            template: InfiniteChestTemplate::Rune1,
            key_name: Some(outcome_item_name("Palace Key")),
        }
    );
}

#[test]
fn execute_infinite_chest_rejects_skeleton_key() {
    let mut character = character(1);
    let mut chest = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_INFINITE_CHEST);
    chest.driver_data = vec![1, 0x44, 0x33, 0x22, 0x11];

    let outcome = execute_item_driver_with_context(
        &mut character,
        &mut chest,
        ItemDriverRequest::Driver {
            driver: IDR_INFINITE_CHEST,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
        false,
        &ItemDriverContext {
            door_key: Some(DoorKeyAccess {
                key_id: IID_SKELETON_KEY,
                name: "Skeleton Key".to_string(),
                source: DoorKeySource::Carried,
            }),
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::InfiniteChestKeyRequired {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn execute_keyring_driver_shows_or_requests_cursor_key_add() {
    let mut character = character(1);
    let mut keyring = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_KEY_RING);
    let request = ItemDriverRequest::Driver {
        driver: IDR_KEY_RING,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut keyring, request, 1, false),
        ItemDriverOutcome::KeyringShow {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    character.cursor_item = Some(ItemId(99));
    assert_eq!(
        execute_item_driver(&mut character, &mut keyring, request, 1, false),
        ItemDriverOutcome::KeyringAddCursorItem {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            key_item_id: ItemId(99),
        }
    );
}
