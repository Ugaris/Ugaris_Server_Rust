use super::*;

#[test]
fn fireball_machine_decodes_projectile_and_timer_reschedule() {
    let mut timer_character = character(0);
    let mut machine = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FIREBALL);
    machine.x = 100;
    machine.y = 100;
    machine.driver_data = vec![131, 126, 42, 9];

    let outcome = execute_item_driver_with_context(
        &mut timer_character,
        &mut machine,
        ItemDriverRequest::Driver {
            driver: IDR_FIREBALL,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        2,
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
    let outcome = execute_item_driver(
        &mut player,
        &mut machine,
        ItemDriverRequest::Driver {
            driver: IDR_FIREBALL,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        2,
        false,
    );
    assert!(matches!(
        outcome,
        ItemDriverOutcome::FireballMachineProjectile {
            schedule_after_ticks: None,
            ..
        }
    ));
}

#[test]
fn flamethrower_timer_pulses_light_and_reschedules() {
    let mut character = character(0);
    let mut trap = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLAMETHROW);
    trap.driver_data = vec![2, 3, 0, 5];

    let outcome = execute_item_driver_with_context(
        &mut character,
        &mut trap,
        ItemDriverRequest::Driver {
            driver: IDR_FLAMETHROW,
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

    assert_eq!(trap.driver_data[0], 1);
    assert_eq!(trap.driver_data[2], 1);
    assert_eq!(trap.sprite, 1);
    assert_eq!(trap.modifier_index[4], V_LIGHT);
    assert_eq!(trap.modifier_value[4], 250);
    assert_eq!(
        outcome,
        ItemDriverOutcome::FlameThrowerPulse {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            direction: 3,
            schedule_after_ticks: 1,
        }
    );
}

#[test]
fn flamethrower_timer_extinguishes_and_uses_interval() {
    let mut character = character(0);
    let mut trap = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLAMETHROW);
    trap.sprite = 10;
    trap.modifier_index[4] = V_LIGHT;
    trap.modifier_value[4] = 250;
    trap.driver_data = vec![0, 3, 1, 5];

    let outcome = execute_item_driver_with_context(
        &mut character,
        &mut trap,
        ItemDriverRequest::Driver {
            driver: IDR_FLAMETHROW,
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

    assert_eq!(trap.sprite, 9);
    assert_eq!(&trap.driver_data[..3], &[TICKS_PER_SECOND as u8, 3, 0]);
    assert_eq!(trap.modifier_index[4], 0);
    assert_eq!(trap.modifier_value[4], 0);
    assert_eq!(
        outcome,
        ItemDriverOutcome::FlameThrowerExtinguished {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            schedule_after_ticks: Some(TICKS_PER_SECOND * 5),
        }
    );
}
