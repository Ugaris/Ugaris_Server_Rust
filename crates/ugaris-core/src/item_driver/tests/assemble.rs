use super::*;

#[test]
fn execute_assemble_driver_maps_legacy_combinations() {
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(7));
    character.cursor_item = Some(ItemId(8));
    let mut item = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_ASSEMBLE);
    item.carried_by = Some(CharacterId(1));
    item.template_id = IID_AREA2_SUN1;
    let request = ItemDriverRequest::Driver {
        driver: IDR_ASSEMBLE,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };
    let context = ItemDriverContext {
        door_key: None,
        cursor_template_id: Some(IID_AREA2_SUN23),
        ..ItemDriverContext::default()
    };

    assert_eq!(
        execute_item_driver_with_context(&mut character, &mut item, request, 1, false, &context),
        ItemDriverOutcome::AssembleItem {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(8),
            template: AssembleTemplate::SunAmulet123,
        }
    );

    item.template_id = IID_STAFF_REDKEY2;
    let context = ItemDriverContext {
        door_key: None,
        cursor_template_id: Some(IID_STAFF_REDKEY13),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(&mut character, &mut item, request, 1, false, &context),
        ItemDriverOutcome::AssembleItem {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(8),
            template: AssembleTemplate::WarrRedkey123,
        }
    );
}

#[test]
fn execute_assemble_driver_reports_legacy_failures() {
    let mut character = character(1);
    let mut item = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_ASSEMBLE);
    item.carried_by = Some(CharacterId(1));
    item.template_id = IID_AREA2_SUN1;
    let request = ItemDriverRequest::Driver {
        driver: IDR_ASSEMBLE,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut item, request, 1, false),
        ItemDriverOutcome::AssembleNeedsCursor {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    character.cursor_item = Some(ItemId(8));
    assert_eq!(
        execute_item_driver(&mut character, &mut item, request, 1, false),
        ItemDriverOutcome::AssembleDoesNotFit {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    item.template_id = 0xDEAD_BEEF;
    assert_eq!(
        execute_item_driver(&mut character, &mut item, request, 1, false),
        ItemDriverOutcome::AssembleUnknownItem {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn execute_shrike_amulet_driver_combines_non_overlapping_parts() {
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(7));
    character.cursor_item = Some(ItemId(8));
    let mut amulet = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SHRIKEAMULET);
    amulet.carried_by = Some(CharacterId(1));
    amulet.driver_data = vec![1];
    let request = ItemDriverRequest::Driver {
        driver: IDR_SHRIKEAMULET,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    let outcome = execute_item_driver_with_context(
        &mut character,
        &mut amulet,
        request,
        38,
        false,
        &ItemDriverContext {
            cursor_driver: Some(IDR_SHRIKEAMULET),
            cursor_drdata0: Some(2),
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::ShrikeAmuletAssemble {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(8),
            combined_bits: 3,
        }
    );

    let outcome = execute_item_driver_with_context(
        &mut character,
        &mut amulet,
        request,
        38,
        false,
        &ItemDriverContext {
            cursor_driver: Some(IDR_SHRIKEAMULET),
            cursor_drdata0: Some(1),
            ..ItemDriverContext::default()
        },
    );
    assert_eq!(
        outcome,
        ItemDriverOutcome::ShrikeAmuletDoesNotFit {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}
