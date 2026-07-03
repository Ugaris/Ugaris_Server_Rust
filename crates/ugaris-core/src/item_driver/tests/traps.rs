use super::*;

#[test]
fn balltrap_non_player_launches_projectile_from_driver_data() {
    let mut character = character(1);
    let mut trap = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_BALLTRAP);
    trap.x = 100;
    trap.y = 100;
    trap.driver_data = vec![131, 126, 42];

    let outcome = execute_item_driver(
        &mut character,
        &mut trap,
        ItemDriverRequest::Driver {
            driver: IDR_BALLTRAP,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::BallTrapProjectile {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            start_x: 101,
            start_y: 99,
            target_x: 103,
            target_y: 98,
            power: 42,
        }
    );
}

#[test]
fn balltrap_ignores_timer_and_player_triggers() {
    let mut trap = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_BALLTRAP);
    trap.x = 100;
    trap.y = 100;
    trap.driver_data = vec![131, 126, 42];
    let request = ItemDriverRequest::Driver {
        driver: IDR_BALLTRAP,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    let mut timer_character = character(0);
    assert_eq!(
        execute_item_driver(&mut timer_character, &mut trap, request, 1, false),
        ItemDriverOutcome::Noop
    );

    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    assert_eq!(
        execute_item_driver(&mut player, &mut trap, request, 1, false),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn spiketrap_triggers_once_and_timer_resets() {
    let mut actor = character(1);
    let mut trap = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPIKETRAP);
    trap.driver_data = vec![0, 4];

    let outcome = execute_item_driver(
        &mut actor,
        &mut trap,
        ItemDriverRequest::Driver {
            driver: IDR_SPIKETRAP,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
        false,
    );
    assert_eq!(trap.sprite, 1);
    assert_eq!(trap.driver_data[0], 1);
    assert_eq!(
        outcome,
        ItemDriverOutcome::SpikeTrapTriggered {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            damage: 4 * crate::entity::POWERSCALE,
            reset_after_ticks: TICKS_PER_SECOND,
        }
    );

    let mut timer_character = character(0);
    let outcome = execute_item_driver_with_context(
        &mut timer_character,
        &mut trap,
        ItemDriverRequest::Driver {
            driver: IDR_SPIKETRAP,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        1,
        false,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );
    assert_eq!(trap.sprite, 0);
    assert_eq!(trap.driver_data[0], 0);
    assert_eq!(
        outcome,
        ItemDriverOutcome::SpikeTrapReset { item_id: ItemId(7) }
    );
}

#[test]
fn usetrap_schedules_target_item_with_using_character() {
    let mut character = character(1);
    let mut trap = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_USETRAP);
    trap.driver_data = vec![20, 30];

    let outcome = execute_item_driver(
        &mut character,
        &mut trap,
        ItemDriverRequest::Driver {
            driver: IDR_USETRAP,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::TriggerMapItem {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            x: 20,
            y: 30,
            target_character_id: CharacterId(1),
            delay_ticks: TICKS_PER_SECOND / 2,
        }
    );
}

#[test]
fn steptrap_timer_discovers_target_and_character_trigger_calls_target_without_character() {
    let mut timer_character = character(0);
    let mut trap = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_STEPTRAP);

    let outcome = execute_item_driver_with_context(
        &mut timer_character,
        &mut trap,
        ItemDriverRequest::Driver {
            driver: IDR_STEPTRAP,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        1,
        false,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );
    assert_eq!(
        outcome,
        ItemDriverOutcome::StepTrapDiscoverTarget { item_id: ItemId(7) }
    );

    let mut character = character(1);
    trap.driver_data = vec![20, 30];
    let outcome = execute_item_driver(
        &mut character,
        &mut trap,
        ItemDriverRequest::Driver {
            driver: IDR_STEPTRAP,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
        false,
    );
    assert_eq!(
        outcome,
        ItemDriverOutcome::TriggerMapItem {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            x: 20,
            y: 30,
            target_character_id: CharacterId(0),
            delay_ticks: 1,
        }
    );
}
