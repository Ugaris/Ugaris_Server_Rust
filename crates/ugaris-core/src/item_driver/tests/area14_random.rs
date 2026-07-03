use super::*;

#[test]
fn trapdoor_step_opens_for_players_and_schedules_close() {
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    actor.x = 10;
    actor.y = 10;
    actor.dir = crate::direction::Direction::Right as u8;
    let mut trapdoor = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_TRAPDOOR);
    trapdoor.x = 10;
    trapdoor.y = 10;

    let outcome = execute_item_driver_with_context(
        &mut actor,
        &mut trapdoor,
        ItemDriverRequest::Driver {
            driver: IDR_TRAPDOOR,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        14,
        false,
        &ItemDriverContext::default(),
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::TrapdoorOpen {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            target_x: 9,
            target_y: 10,
            schedule_after_ticks: TICKS_PER_SECOND * 6,
        }
    );
}

#[test]
fn trapdoor_uses_cursor_steelbar_to_block() {
    let mut actor = character(1);
    actor.x = 9;
    actor.y = 10;
    actor.cursor_item = Some(ItemId(99));
    let mut trapdoor = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_TRAPDOOR);
    trapdoor.x = 10;
    trapdoor.y = 10;

    let outcome = execute_item_driver_with_context(
        &mut actor,
        &mut trapdoor,
        ItemDriverRequest::Driver {
            driver: IDR_TRAPDOOR,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        14,
        false,
        &ItemDriverContext {
            cursor_template_id: Some(IID_AREA14_STEELBAR),
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::TrapdoorBlocked {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(99),
        }
    );
}

#[test]
fn trapdoor_timer_closes_only_open_state() {
    let mut timer = character(0);
    let mut trapdoor = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_TRAPDOOR);
    trapdoor.driver_data = vec![1];

    let outcome = execute_item_driver_with_context(
        &mut timer,
        &mut trapdoor,
        ItemDriverRequest::Driver {
            driver: IDR_TRAPDOOR,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        },
        14,
        false,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::TrapdoorClose { item_id: ItemId(8) }
    );
}

#[test]
fn junkpile_search_requires_empty_cursor_and_carries_level() {
    let mut actor = character(1);
    let mut junkpile = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_JUNKPILE);
    junkpile.driver_data = vec![17];

    let outcome = execute_item_driver_with_context(
        &mut actor,
        &mut junkpile,
        ItemDriverRequest::Driver {
            driver: IDR_JUNKPILE,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        14,
        false,
        &ItemDriverContext::default(),
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::JunkpileSearch {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            level: 17,
        }
    );

    actor.cursor_item = Some(ItemId(99));
    let outcome = execute_item_driver_with_context(
        &mut actor,
        &mut junkpile,
        ItemDriverRequest::Driver {
            driver: IDR_JUNKPILE,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        14,
        false,
        &ItemDriverContext::default(),
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::JunkpileCursorOccupied {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn gastrap_trigger_and_timer_advance_animation_state() {
    let mut actor = character(1);
    let mut trap = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_GASTRAP);
    trap.driver_data = vec![4, 0];

    let outcome = execute_item_driver_with_context(
        &mut actor,
        &mut trap,
        ItemDriverRequest::Driver {
            driver: IDR_GASTRAP,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        14,
        false,
        &ItemDriverContext::default(),
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::GasTrapPulse {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            power: 4,
            schedule_initial_trigger: true,
            schedule_animation: true,
        }
    );
    assert_eq!(trap.driver_data[1], 1);

    let outcome = execute_item_driver_with_context(
        &mut actor,
        &mut trap,
        ItemDriverRequest::Driver {
            driver: IDR_GASTRAP,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        14,
        false,
        &ItemDriverContext::default(),
    );
    assert_eq!(outcome, ItemDriverOutcome::Noop);

    let mut timer = character(0);
    trap.driver_data[1] = 8;
    let outcome = execute_item_driver_with_context(
        &mut timer,
        &mut trap,
        ItemDriverRequest::Driver {
            driver: IDR_GASTRAP,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        },
        14,
        false,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );
    assert_eq!(
        outcome,
        ItemDriverOutcome::GasTrapPulse {
            item_id: ItemId(8),
            character_id: CharacterId(0),
            power: 4,
            schedule_initial_trigger: false,
            schedule_animation: false,
        }
    );
    assert_eq!(trap.driver_data[1], 0);
}

#[test]
fn randomshrine_requires_matching_key_before_effects() {
    let mut actor = character(1);
    let mut shrine = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_RANDOMSHRINE);
    shrine.driver_data = vec![54, 23];

    let outcome = execute_item_driver_with_context(
        &mut actor,
        &mut shrine,
        ItemDriverRequest::Driver {
            driver: IDR_RANDOMSHRINE,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        14,
        false,
        &ItemDriverContext::default(),
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::RandomShrineNeedsKey {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            shrine_type: 54,
            level: 23,
        }
    );
}

#[test]
fn randomshrine_classifies_legacy_type_ranges_and_repeats() {
    let mut actor = character(1);
    let mut shrine = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_RANDOMSHRINE);
    shrine.driver_data = vec![54, 23];

    let outcome = execute_item_driver_with_context(
        &mut actor,
        &mut shrine,
        ItemDriverRequest::Driver {
            driver: IDR_RANDOMSHRINE,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        14,
        false,
        &ItemDriverContext {
            has_matching_random_shrine_key: true,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::RandomShrineUse {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            shrine_type: 54,
            level: 23,
            kind: RandomShrineKind::Security,
        }
    );

    shrine.driver_data = vec![63, 23];
    let outcome = execute_item_driver_with_context(
        &mut actor,
        &mut shrine,
        ItemDriverRequest::Driver {
            driver: IDR_RANDOMSHRINE,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        14,
        false,
        &ItemDriverContext {
            has_matching_random_shrine_key: true,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::RandomShrineUse {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            shrine_type: 63,
            level: 23,
            kind: RandomShrineKind::Jobless,
        }
    );

    shrine.driver_data = vec![54, 23];
    let outcome = execute_item_driver_with_context(
        &mut actor,
        &mut shrine,
        ItemDriverRequest::Driver {
            driver: IDR_RANDOMSHRINE,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        14,
        false,
        &ItemDriverContext {
            has_matching_random_shrine_key: true,
            random_shrine_already_used: true,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::RandomShrineAlreadyUsed {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            shrine_type: 54,
            level: 23,
        }
    );
}

#[test]
fn randomshrine_continuity_uses_its_distinct_path() {
    let mut actor = character(1);
    let mut shrine = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_RANDOMSHRINE);
    shrine.driver_data = vec![255, 99];

    let outcome = execute_item_driver_with_context(
        &mut actor,
        &mut shrine,
        ItemDriverRequest::Driver {
            driver: IDR_RANDOMSHRINE,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        14,
        false,
        &ItemDriverContext {
            has_matching_random_shrine_key: true,
            random_shrine_already_used: true,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::RandomShrineUse {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            shrine_type: 255,
            level: 99,
            kind: RandomShrineKind::Continuity,
        }
    );
}
