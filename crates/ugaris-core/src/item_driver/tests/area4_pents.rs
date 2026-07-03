use super::*;

#[test]
fn pentagram_driver_ports_activation_and_timer_boundaries() {
    let mut actor = character(1);
    let mut pent = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_PENT);
    pent.driver_data = vec![12, 0, 4, 99, 8];
    let request = ItemDriverRequest::Driver {
        driver: IDR_PENT,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut pent,
            request,
            4,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::PentagramActivate {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            level: 12,
            color: 4,
        }
    );

    pent.driver_data[1] = 77;
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut pent,
            request,
            4,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::PentagramAlreadyActive {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    let mut timer = character(0);
    let timer_request = ItemDriverRequest::Driver {
        driver: IDR_PENT,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer,
            &mut pent,
            timer_request,
            4,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::PentagramTimer {
            item_id: ItemId(7),
            level: 12,
            status: 77,
            area_status: 8,
        }
    );
}

#[test]
fn pent_boss_door_preserves_legacy_access_and_position_checks() {
    let mut character = character(1);
    character.x = 11;
    character.y = 10;
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_PENTBOSSDOOR);
    door.x = 10;
    door.y = 10;
    let request = ItemDriverRequest::Driver {
        driver: IDR_PENTBOSSDOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut door,
            request,
            4,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::PentBossDoorLocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut door,
            request,
            4,
            false,
            &ItemDriverContext {
                current_tick: 1_000,
                pent_last_solve_tick: Some(900),
                pent_demon_lord_access_seconds: Some(120),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::PentBossDoor {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            x: 9,
            y: 10,
        }
    );

    character.y = 11;
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut door,
            request,
            4,
            false,
            &ItemDriverContext {
                current_tick: 1_000,
                pent_last_solve_tick: Some(900),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn pent_drivers_keep_area4_libload_guard() {
    let mut character = character(1);
    let mut pent = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_PENT);
    let pent_request = ItemDriverRequest::Driver {
        driver: IDR_PENT,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut pent,
            pent_request,
            1,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_PENT,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            required_area: 4,
        }
    );

    let mut door = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_PENTBOSSDOOR);
    let door_request = ItemDriverRequest::Driver {
        driver: IDR_PENTBOSSDOOR,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut door,
            door_request,
            1,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_PENTBOSSDOOR,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            required_area: 4,
        }
    );
}
