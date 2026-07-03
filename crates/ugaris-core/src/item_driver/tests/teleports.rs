use super::*;

#[test]
fn transport_driver_opens_valid_points_and_rejects_invalid_points() {
    let mut character = character(1);
    let mut transport = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_TRANSPORT);
    let request = ItemDriverRequest::Driver {
        driver: IDR_TRANSPORT,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    transport.driver_data = vec![25];
    assert_eq!(
        execute_item_driver(&mut character, &mut transport, request, 1, false),
        ItemDriverOutcome::TransportOpen {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            point: 25,
        }
    );

    transport.driver_data = vec![26];
    assert_eq!(
        execute_item_driver(&mut character, &mut transport, request, 1, false),
        ItemDriverOutcome::TransportInvalid {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            point: 26,
        }
    );

    transport.driver_data = vec![LEGACY_TRANSPORT_CLAN_EXIT];
    assert!(matches!(
        execute_item_driver(&mut character, &mut transport, request, 1, false),
        ItemDriverOutcome::TransportOpen {
            point: LEGACY_TRANSPORT_CLAN_EXIT,
            ..
        }
    ));

    let travel_request = ItemDriverRequest::Driver {
        driver: IDR_TRANSPORT,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 22 + 5 * 256,
    };
    assert_eq!(
        execute_item_driver(&mut character, &mut transport, travel_request, 1, false),
        ItemDriverOutcome::TransportTravel {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 22 + 5 * 256,
        }
    );
}

#[test]
fn execute_teleport_driver_decodes_target_and_checks_requirements() {
    let mut character = character(1);
    character.level = 10;
    let mut item = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_TELEPORT);
    item.min_level = 5;
    item.max_level = 20;
    item.driver_data = vec![44, 1, 88, 2, 3, 0, 1, 0, 0, 0, 0, 0, 1];

    let outcome = execute_item_driver(
        &mut character,
        &mut item,
        ItemDriverRequest::Driver {
            driver: IDR_TELEPORT,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::Teleport {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            x: 300,
            y: 600,
            area_id: 3,
            stop_driver: true,
            quiet: true,
        }
    );

    item.driver_data[10] = 1;
    assert_eq!(
        execute_item_driver(
            &mut character,
            &mut item,
            ItemDriverRequest::Driver {
                driver: IDR_TELEPORT,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        ),
        ItemDriverOutcome::BlockedByRequirements {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn execute_recall_driver_targets_character_rest_area_and_checks_level() {
    let mut character = character(1);
    character.level = 10;
    character.rest_area = 3;
    character.rest_x = 44;
    character.rest_y = 55;
    let mut item = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_RECALL);
    item.carried_by = Some(CharacterId(1));
    item.driver_data = vec![20];

    let request = ItemDriverRequest::Driver {
        driver: IDR_RECALL,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };
    assert_eq!(
        execute_item_driver(&mut character, &mut item, request, 1, false),
        ItemDriverOutcome::Recall {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            x: 44,
            y: 55,
            area_id: 3,
        }
    );

    item.driver_data = vec![9];
    assert_eq!(
        execute_item_driver(&mut character, &mut item, request, 1, false),
        ItemDriverOutcome::BlockedByRequirements {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn execute_city_recall_driver_maps_scroll_types_and_blocks_arena() {
    let mut character = character(1);
    character.level = 99;
    let mut item = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_CITY_RECALL);
    item.carried_by = Some(CharacterId(1));
    item.driver_data = vec![7, 3];

    let request = ItemDriverRequest::Driver {
        driver: IDR_CITY_RECALL,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut item, request, 1, false),
        ItemDriverOutcome::CityRecall {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            x: 203,
            y: 227,
            area_id: 29,
        }
    );

    assert_eq!(
        execute_item_driver(&mut character, &mut item, request, 34, true),
        ItemDriverOutcome::BlockedByArea {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    item.driver_data = vec![99, 3];
    assert_eq!(
        execute_item_driver(&mut character, &mut item, request, 1, false),
        ItemDriverOutcome::BlockedByRequirements {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}
