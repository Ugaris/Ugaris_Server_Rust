use super::*;

#[test]
fn parkshrine_driver_ports_area2_memorize_boundary() {
    let mut actor = character(1);
    let mut shrine = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_PARKSHRINE);
    shrine.driver_data = vec![2];
    let request = ItemDriverRequest::Driver {
        driver: IDR_PARKSHRINE,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(IDR_PARKSHRINE, 23);
    assert_eq!(
        execute_item_driver(&mut actor, &mut shrine, request, 2, false),
        ItemDriverOutcome::ParkShrine {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            shrine: 2,
        }
    );

    shrine.driver_data = vec![4];
    assert_eq!(
        execute_item_driver(&mut actor, &mut shrine, request, 2, false),
        ItemDriverOutcome::ParkShrineBug {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            shrine: 4,
        }
    );

    let mut timer_character = character(0);
    assert_eq!(
        execute_item_driver(&mut timer_character, &mut shrine, request, 2, false),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn zombie_shrine_requires_matching_skull_on_cursor() {
    let mut character = character(1);
    let mut shrine = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SHRINE);
    shrine.driver_data = vec![1];
    let request = ItemDriverRequest::Driver {
        driver: IDR_SHRINE,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut shrine,
            request,
            2,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::ZombieShrineNeedsOffering {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            shrine_type: 1,
        }
    );

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut shrine,
            request,
            2,
            false,
            &ItemDriverContext {
                cursor_template_id: Some(IID_AREA2_ZOMBIESKULL2),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::ZombieShrine {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            shrine_type: 1,
        }
    );
}

#[test]
fn special_shrine_dispatches_hc_to_sc_kind() {
    let mut character = character(3);
    let mut shrine = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPECIAL_SHRINE);
    shrine.driver_data = vec![0x0A];

    assert_eq!(
        execute_item_driver(
            &mut character,
            &mut shrine,
            ItemDriverRequest::Driver {
                driver: IDR_SPECIAL_SHRINE,
                item_id: ItemId(7),
                character_id: CharacterId(3),
                spec: 0,
            },
            1,
            false,
        ),
        ItemDriverOutcome::SpecialShrine {
            item_id: ItemId(7),
            character_id: CharacterId(3),
            kind: 0x0A,
        }
    );
}

#[test]
fn demonshrine_dispatches_location_and_level_gate() {
    let mut character = character(3);
    character.level = 9;
    let mut shrine = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DEMONSHRINE);
    shrine.min_level = 10;
    shrine.x = 12;
    shrine.y = 34;

    let request = ItemDriverRequest::Driver {
        driver: IDR_DEMONSHRINE,
        item_id: ItemId(7),
        character_id: CharacterId(3),
        spec: 0,
    };
    assert_eq!(
        execute_item_driver(&mut character, &mut shrine, request, 5, false),
        ItemDriverOutcome::BlockedByRequirements {
            item_id: ItemId(7),
            character_id: CharacterId(3),
        }
    );

    character.level = 10;
    assert_eq!(
        execute_item_driver(&mut character, &mut shrine, request, 5, false),
        ItemDriverOutcome::DemonShrine {
            item_id: ItemId(7),
            character_id: CharacterId(3),
            location_id: 12 + (34 << 8) + (5 << 16),
        }
    );
}
