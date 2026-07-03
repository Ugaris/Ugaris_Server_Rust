use super::*;

#[test]
fn staffer_spiketrap_uses_area26_drdata_layout() {
    let mut player = character(1);
    let mut trap = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_STAFFER);
    trap.sprite = 100;
    trap.driver_data = vec![1, 0, 6];

    let request = ItemDriverRequest::Driver {
        driver: IDR_STAFFER,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };
    let outcome = execute_item_driver(&mut player, &mut trap, request, 26, false);

    assert_eq!(trap.sprite, 101);
    assert_eq!(trap.driver_data[1], 1);
    assert_eq!(
        outcome,
        ItemDriverOutcome::SpikeTrapTriggered {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            damage: 6 * POWERSCALE,
            reset_after_ticks: TICKS_PER_SECOND,
        }
    );

    let mut timer_character = character(0);
    let outcome = execute_item_driver(
        &mut timer_character,
        &mut trap,
        ItemDriverRequest::Driver {
            driver: IDR_STAFFER,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        26,
        false,
    );

    assert_eq!(trap.sprite, 100);
    assert_eq!(trap.driver_data[1], 0);
    assert_eq!(
        outcome,
        ItemDriverOutcome::SpikeTrapReset { item_id: ItemId(7) }
    );
}

#[test]
fn staffer_fireball_machine_is_timer_only_and_uses_shifted_drdata_layout() {
    let mut timer_character = character(0);
    let mut machine = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_STAFFER);
    machine.x = 100;
    machine.y = 100;
    machine.driver_data = vec![2, 131, 126, 42, 9];

    let outcome = execute_item_driver_with_context(
        &mut timer_character,
        &mut machine,
        ItemDriverRequest::Driver {
            driver: IDR_STAFFER,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        26,
        false,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::FireballMachineProjectile {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            start_x: 101,
            start_y: 99,
            target_x: 103,
            target_y: 98,
            power: 42,
            schedule_after_ticks: Some(9),
        }
    );

    let mut player = character(1);
    assert_eq!(
        execute_item_driver(
            &mut player,
            &mut machine,
            ItemDriverRequest::Driver {
                driver: IDR_STAFFER,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            26,
            false,
        ),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn staffer_block_delegates_to_existing_block_move_and_area_guard() {
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    let mut block = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_STAFFER);
    block.driver_data = vec![3];

    assert_eq!(
        execute_item_driver(
            &mut player,
            &mut block,
            ItemDriverRequest::Driver {
                driver: IDR_STAFFER,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        ),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_STAFFER,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            required_area: 26,
        }
    );

    assert_eq!(
        execute_item_driver(
            &mut player,
            &mut block,
            ItemDriverRequest::Driver {
                driver: IDR_STAFFER,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            26,
            false,
        ),
        ItemDriverOutcome::StafferBlockMove {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}
