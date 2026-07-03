use super::*;

#[test]
fn forest_spade_classifies_note_collapse_and_treasure_locations() {
    let mut character = character(42);
    let mut spade = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FORESTSPADE);
    spade.carried_by = Some(CharacterId(42));
    let request = ItemDriverRequest::Driver {
        driver: IDR_FORESTSPADE,
        item_id: ItemId(7),
        character_id: CharacterId(42),
        spec: 0,
    };

    character.x = 205;
    character.y = 234;
    assert_eq!(
        execute_item_driver(&mut character, &mut spade, request, 16, false),
        ItemDriverOutcome::ForestSpadeFind {
            item_id: ItemId(7),
            character_id: CharacterId(42),
            find: ForestSpadeFind::ForestNote1,
        }
    );

    character.x = 93;
    character.y = 36;
    assert_eq!(
        execute_item_driver(&mut character, &mut spade, request, 1, false),
        ItemDriverOutcome::ForestSpadeCollapse {
            item_id: ItemId(7),
            character_id: CharacterId(42),
            x: 106,
            y: 211,
        }
    );

    character.x = 214;
    character.y = 136;
    assert_eq!(
        execute_item_driver(&mut character, &mut spade, request, 29, false),
        ItemDriverOutcome::ForestSpadeFind {
            item_id: ItemId(7),
            character_id: CharacterId(42),
            find: ForestSpadeFind::BranningtonTreasure { dig_index: 2 },
        }
    );
}

#[test]
fn forest_spade_blocks_cursor_and_reports_empty_ground() {
    let mut character = character(42);
    character.cursor_item = Some(ItemId(9));
    let mut spade = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FORESTSPADE);
    spade.carried_by = Some(CharacterId(42));
    let request = ItemDriverRequest::Driver {
        driver: IDR_FORESTSPADE,
        item_id: ItemId(7),
        character_id: CharacterId(42),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut spade, request, 16, false),
        ItemDriverOutcome::ForestSpadeCursorOccupied {
            item_id: ItemId(7),
            character_id: CharacterId(42),
        }
    );

    character.cursor_item = None;
    assert_eq!(
        execute_item_driver(&mut character, &mut spade, request, 16, false),
        ItemDriverOutcome::ForestSpadeNothing {
            item_id: ItemId(7),
            character_id: CharacterId(42),
        }
    );
}

#[test]
fn forest_chest_ports_key_gates_and_reward_classification() {
    let mut character = character(42);
    let mut chest = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FORESTCHEST);
    let request = ItemDriverRequest::Driver {
        driver: IDR_FORESTCHEST,
        item_id: ItemId(7),
        character_id: CharacterId(42),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut chest,
            request,
            16,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::ForestChestLocked {
            item_id: ItemId(7),
            character_id: CharacterId(42),
        }
    );

    let context = ItemDriverContext {
        has_area16_robber_key: true,
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(&mut character, &mut chest, request, 16, false, &context,),
        ItemDriverOutcome::ForestChest {
            item_id: ItemId(7),
            character_id: CharacterId(42),
            amount: 9_733,
            imp_flag_mask: 1,
        }
    );

    chest.driver_data = vec![1];
    let context = ItemDriverContext {
        has_area16_skelly_key: true,
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(&mut character, &mut chest, request, 16, false, &context,),
        ItemDriverOutcome::ForestChest {
            item_id: ItemId(7),
            character_id: CharacterId(42),
            amount: 17_587,
            imp_flag_mask: 2,
        }
    );
}

#[test]
fn forest_chest_blocks_cursor_and_area16_libload_guard() {
    let mut character = character(42);
    character.cursor_item = Some(ItemId(9));
    let mut chest = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FORESTCHEST);
    let request = ItemDriverRequest::Driver {
        driver: IDR_FORESTCHEST,
        item_id: ItemId(7),
        character_id: CharacterId(42),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut chest,
            request,
            16,
            false,
            &ItemDriverContext {
                has_area16_robber_key: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::ForestChestCursorOccupied {
            item_id: ItemId(7),
            character_id: CharacterId(42),
        }
    );

    character.cursor_item = None;
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut chest,
            request,
            1,
            false,
            &ItemDriverContext {
                has_area16_robber_key: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_FORESTCHEST,
            item_id: ItemId(7),
            character_id: CharacterId(42),
            required_area: 16,
        }
    );
}
