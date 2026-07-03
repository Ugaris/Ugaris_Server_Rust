use super::*;

#[test]
fn oxy_potion_driver_requires_area31_and_carried_item() {
    let mut character = character(1);
    let mut potion = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_OXYPOTION);
    let request = ItemDriverRequest::Driver {
        driver: IDR_OXYPOTION,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut potion, request, 30, false),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_OXYPOTION,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            required_area: 31,
        }
    );
    assert_eq!(
        execute_item_driver(&mut character, &mut potion, request, 31, false),
        ItemDriverOutcome::Noop
    );

    potion.carried_by = Some(CharacterId(1));
    character.inventory[30] = Some(ItemId(8));
    assert_eq!(
        execute_item_driver(&mut character, &mut potion, request, 31, false),
        ItemDriverOutcome::OxygenPotion {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            installed: false,
        }
    );
}

#[test]
fn pick_berry_driver_ports_area31_boundary_and_location_id() {
    let mut actor = character(1);
    let mut berry = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_PICKBERRY);
    berry.x = 12;
    berry.y = 34;
    berry.driver_data = vec![3];
    let request = ItemDriverRequest::Driver {
        driver: IDR_PICKBERRY,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(IDR_PICKBERRY, 129);
    assert_eq!(
        execute_item_driver(&mut actor, &mut berry, request, 30, false),
        ItemDriverOutcome::BlockedByArea {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    actor.cursor_item = Some(ItemId(99));
    assert_eq!(
        execute_item_driver(&mut actor, &mut berry, request, 31, false),
        ItemDriverOutcome::PickBerryCursorOccupied {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    actor.cursor_item = None;
    assert_eq!(
        execute_item_driver(&mut actor, &mut berry, request, 31, false),
        ItemDriverOutcome::PickBerry {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            kind: 3,
            location_id: 12 + (34 << 8) + (31 << 16),
        }
    );

    let mut timer_character = character(0);
    assert_eq!(
        execute_item_driver(&mut timer_character, &mut berry, request, 31, false),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn alchemy_flower_driver_ports_location_and_cursor_gate() {
    let mut actor = character(1);
    let mut flower = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLOWER);
    flower.x = 20;
    flower.y = 40;
    flower.driver_data = vec![17];
    let request = ItemDriverRequest::Driver {
        driver: IDR_FLOWER,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(IDR_FLASK, 32);
    assert_eq!(IDR_FLOWER, 33);
    assert_eq!(
        execute_item_driver(&mut actor, &mut flower, request, 7, false),
        ItemDriverOutcome::PickAlchemyFlower {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            kind: 17,
            location_id: 20 + (40 << 8) + (7 << 16),
        }
    );

    actor.cursor_item = Some(ItemId(99));
    assert_eq!(
        execute_item_driver(&mut actor, &mut flower, request, 7, false),
        ItemDriverOutcome::PickAlchemyFlowerCursorOccupied {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    let mut timer_character = character(0);
    assert_eq!(
        execute_item_driver(&mut timer_character, &mut flower, request, 7, false),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn lizard_flower_mixer_requires_cursor_flower_and_combines_bits() {
    let mut actor = character(1);
    actor.cursor_item = Some(ItemId(9));
    let mut flower = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_LIZARDFLOWER);
    flower.carried_by = Some(CharacterId(1));
    flower.driver_data = vec![1];
    flower.sprite = 11190;
    let request = ItemDriverRequest::Driver {
        driver: IDR_LIZARDFLOWER,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut flower,
            request,
            30,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_LIZARDFLOWER,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            required_area: 31,
        }
    );

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut flower,
            request,
            31,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::LizardFlowerDoesNotFit {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    let context = ItemDriverContext {
        cursor_driver: Some(IDR_LIZARDFLOWER),
        cursor_sprite: Some(11191),
        cursor_drdata0: Some(6),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut flower, request, 31, false, &context,),
        ItemDriverOutcome::LizardFlowerMixed {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(9),
            combined_bits: 7,
            complete: true,
            bottle_message: true,
        }
    );

    actor.cursor_item = None;
    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut flower, request, 31, false, &context,),
        ItemDriverOutcome::LizardFlowerNeedsCursor {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
}
