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

#[test]
fn vault_skull_opens_only_when_rouven_state_is_0_through_5() {
    let mut player = character(1);
    let mut skull = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_STAFFER);
    skull.driver_data = vec![4];

    for state in 0..=5 {
        let outcome = execute_item_driver_with_context(
            &mut player,
            &mut skull,
            ItemDriverRequest::Driver {
                driver: IDR_STAFFER,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            26,
            false,
            &ItemDriverContext {
                rouven_state: Some(state),
                ..ItemDriverContext::default()
            },
        );
        assert_eq!(
            outcome,
            ItemDriverOutcome::VaultSkullOpened {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            },
            "state {state} should open the skull"
        );
    }

    for state in [-1, 6, 13] {
        let outcome = execute_item_driver_with_context(
            &mut player,
            &mut skull,
            ItemDriverRequest::Driver {
                driver: IDR_STAFFER,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            26,
            false,
            &ItemDriverContext {
                rouven_state: Some(state),
                ..ItemDriverContext::default()
            },
        );
        assert_eq!(
            outcome,
            ItemDriverOutcome::Noop,
            "state {state} is out of range"
        );
    }

    // Timer calls (`cn == 0`) are always a no-op, matching C's `if (!cn)
    // return;` guard.
    let mut timer_character = character(0);
    let outcome = execute_item_driver_with_context(
        &mut timer_character,
        &mut skull,
        ItemDriverRequest::Driver {
            driver: IDR_STAFFER,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        26,
        false,
        &ItemDriverContext {
            rouven_state: Some(0),
            ..ItemDriverContext::default()
        },
    );
    assert_eq!(outcome, ItemDriverOutcome::Noop);
}

#[test]
fn vault_shelf_selects_reward_by_drdata1() {
    let mut player = character(1);

    let mut ritual_shelf = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_STAFFER);
    ritual_shelf.driver_data = vec![5, 2];
    assert_eq!(
        execute_item_driver(
            &mut player,
            &mut ritual_shelf,
            ItemDriverRequest::Driver {
                driver: IDR_STAFFER,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            26,
            false,
        ),
        ItemDriverOutcome::VaultShelfSearch {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            find: VaultShelfFind::Ritual,
        }
    );

    let mut journal_shelf = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_STAFFER);
    journal_shelf.driver_data = vec![5, 1];
    assert_eq!(
        execute_item_driver(
            &mut player,
            &mut journal_shelf,
            ItemDriverRequest::Driver {
                driver: IDR_STAFFER,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            26,
            false,
        ),
        ItemDriverOutcome::VaultShelfSearch {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            find: VaultShelfFind::Journal,
        }
    );

    let mut empty_shelf = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_STAFFER);
    empty_shelf.driver_data = vec![5, 0];
    assert_eq!(
        execute_item_driver(
            &mut player,
            &mut empty_shelf,
            ItemDriverRequest::Driver {
                driver: IDR_STAFFER,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            26,
            false,
        ),
        ItemDriverOutcome::VaultShelfSearch {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            find: VaultShelfFind::Nothing,
        }
    );

    // Timer calls (`cn == 0`) are always a no-op, matching C's `if (!cn)
    // return;` guard.
    let mut timer_character = character(0);
    assert_eq!(
        execute_item_driver(
            &mut timer_character,
            &mut ritual_shelf,
            ItemDriverRequest::Driver {
                driver: IDR_STAFFER,
                item_id: ItemId(7),
                character_id: CharacterId(0),
                spec: 0,
            },
            26,
            false,
        ),
        ItemDriverOutcome::Noop
    );
}
