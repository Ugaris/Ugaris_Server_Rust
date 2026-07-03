use super::*;

#[test]
fn flask_driver_ports_empty_shake_teufelheim_arena_and_finished_blocks() {
    let mut actor = character(1);
    let mut flask = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLASK);
    flask.carried_by = Some(CharacterId(1));
    flask.driver_data = vec![1, 0, 0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_FLASK,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut flask, request, 1, true),
        ItemDriverOutcome::FlaskEmptyShaken {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(
        execute_item_driver(&mut actor, &mut flask, request, 34, true),
        ItemDriverOutcome::BlockedByArea {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(
        execute_item_driver(&mut actor, &mut flask, request, 1, false),
        ItemDriverOutcome::FlaskEmptyShaken {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    actor.cursor_item = Some(ItemId(9));
    flask.driver_data[2] = 1;
    assert_eq!(
        execute_item_driver(&mut actor, &mut flask, request, 1, false),
        ItemDriverOutcome::FlaskFinishedNoMoreIngredients {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn beyond_potion_blocks_failed_requirements_and_teufelheim_arena() {
    let mut character = character(3);
    character.level = 9;
    let mut potion = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_BEYONDPOTION);
    potion.carried_by = Some(character.id);
    potion.min_level = 10;
    let request = ItemDriverRequest::Driver {
        driver: IDR_BEYONDPOTION,
        item_id: ItemId(7),
        character_id: CharacterId(3),
        spec: 0,
    };

    assert!(matches!(
        execute_item_driver(&mut character, &mut potion, request, 1, false),
        ItemDriverOutcome::BlockedByRequirements { .. }
    ));

    character.level = 10;
    assert!(matches!(
        execute_item_driver(&mut character, &mut potion, request, 34, true),
        ItemDriverOutcome::BlockedByArea { .. }
    ));
}

#[test]
fn teufel_arena_exit_requires_full_health_and_targets_legacy_exit_tile() {
    let mut actor = character(1);
    actor.values[0][CharacterValue::Hp as usize] = 100;
    actor.hp = 99 * POWERSCALE;
    let mut exit = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_TEUFELARENAEXIT);
    let request = ItemDriverRequest::Driver {
        driver: IDR_TEUFELARENAEXIT,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut exit, request, 34, false),
        ItemDriverOutcome::TeufelArenaExitLowHealth {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    actor.hp = 100 * POWERSCALE;
    assert_eq!(
        execute_item_driver(&mut actor, &mut exit, request, 34, false),
        ItemDriverOutcome::TeufelArenaExit {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            x: 206,
            y: 231,
        }
    );
}

#[test]
fn teufel_arena_selects_legacy_destination_and_gates_suit_and_level() {
    let mut actor = character(1);
    actor.sprite = 1;
    actor.level = 38;
    let mut arena = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_TEUFELARENA);
    arena.driver_data = vec![1];
    let request = ItemDriverRequest::Driver {
        driver: IDR_TEUFELARENA,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut arena,
            request,
            34,
            false,
            &ItemDriverContext {
                teufel_arena_roll: Some(7),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::TeufelArenaNeedsSuit {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    actor.sprite = 27;
    actor.level = 39;
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut arena,
            request,
            34,
            false,
            &ItemDriverContext {
                teufel_arena_roll: Some(7),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::TeufelArenaLevelTooHigh {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    actor.level = 38;
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut arena,
            request,
            34,
            false,
            &ItemDriverContext {
                teufel_arena_roll: Some(7),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::TeufelArena {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            x: 237,
            y: 198,
        }
    );
}

#[test]
fn teufel_door_enforces_legacy_sprite_class_gates() {
    let mut actor = character(1);
    actor.sprite = 1;
    let mut door = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_TEUFELDOOR);
    door.driver_data = vec![0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_TEUFELDOOR,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut door, request, 34, false),
        ItemDriverOutcome::TeufelDoorNoHumans {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    actor.sprite = 27;
    door.driver_data[0] = 2;
    assert_eq!(
        execute_item_driver(&mut actor, &mut door, request, 34, false),
        ItemDriverOutcome::TeufelDoorNoBeggars {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    actor.sprite = 157;
    door.driver_data[0] = 3;
    assert_eq!(
        execute_item_driver(&mut actor, &mut door, request, 34, false),
        ItemDriverOutcome::TeufelDoorOnlyNobles {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn teufel_door_targets_opposite_cardinal_tile() {
    let mut actor = character(1);
    actor.sprite = 39;
    actor.x = 9;
    actor.y = 10;
    let mut door = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_TEUFELDOOR);
    door.x = 10;
    door.y = 10;
    door.driver_data = vec![3];

    assert_eq!(
        execute_item_driver(
            &mut actor,
            &mut door,
            ItemDriverRequest::Driver {
                driver: IDR_TEUFELDOOR,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            34,
            false,
        ),
        ItemDriverOutcome::TeufelDoor {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            x: 11,
            y: 10,
        }
    );

    actor.y = 9;
    assert_eq!(
        execute_item_driver(
            &mut actor,
            &mut door,
            ItemDriverRequest::Driver {
                driver: IDR_TEUFELDOOR,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            34,
            false,
        ),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn teufel_ratnest_timer_decays_wave_and_classifies_spawn_template() {
    let mut timer = character(0);
    let mut nest = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_TEUFELRATNEST);
    nest.sprite = 15280;
    set_drdata_u16(&mut nest, 0, 201);
    set_drdata(&mut nest, 4, 1);

    let outcome = execute_item_driver_with_context(
        &mut timer,
        &mut nest,
        ItemDriverRequest::Driver {
            driver: IDR_TEUFELRATNEST,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        },
        34,
        false,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(drdata_u16(&nest, 0), 200);
    assert_eq!(
        nest.description,
        "An Ice Rat nest[1]. You could destroy it...[200,78]"
    );
    assert_eq!(
        outcome,
        ItemDriverOutcome::TeufelRatNestSpawn {
            item_id: ItemId(8),
            nest_kind: 1,
            wave: 200,
            level: 78,
            template: "rat81",
            schedule_after_ticks: TICKS_PER_SECOND * 20,
        }
    );
}

#[test]
fn teufel_ratnest_timer_honors_destroy_cooldown_before_respawn() {
    let mut timer = character(0);
    let mut nest = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_TEUFELRATNEST);
    nest.sprite = 0;
    set_drdata(&mut nest, 2, 2);

    let request = ItemDriverRequest::Driver {
        driver: IDR_TEUFELRATNEST,
        item_id: ItemId(8),
        character_id: CharacterId(0),
        spec: 0,
    };
    let context = ItemDriverContext {
        timer_call: true,
        ..ItemDriverContext::default()
    };

    assert_eq!(
        execute_item_driver_with_context(&mut timer, &mut nest, request, 34, false, &context),
        ItemDriverOutcome::Noop
    );
    assert_eq!(drdata(&nest, 2), 1);
    assert_eq!(nest.sprite, 0);

    assert!(matches!(
        execute_item_driver_with_context(&mut timer, &mut nest, request, 34, false, &context),
        ItemDriverOutcome::TeufelRatNestSpawn {
            template: "rat70",
            ..
        }
    ));
    assert_eq!(drdata(&nest, 2), 0);
    assert_eq!(nest.sprite, 15281);
}

#[test]
fn teufel_ratnest_player_destroy_requires_no_live_guard() {
    let mut actor = character(1);
    let mut nest = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_TEUFELRATNEST);
    nest.sprite = 15281;
    set_drdata_u16(&mut nest, 0, 500);
    let request = ItemDriverRequest::Driver {
        driver: IDR_TEUFELRATNEST,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut nest,
            request,
            34,
            false,
            &ItemDriverContext {
                teufel_ratnest_guard_active: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::TeufelRatNestGuarded {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(drdata_u16(&nest, 0), 500);
    assert_eq!(nest.sprite, 15281);

    assert_eq!(
        execute_item_driver(&mut actor, &mut nest, request, 34, false),
        ItemDriverOutcome::TeufelRatNestDestroyed {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(drdata_u16(&nest, 0), 0);
    assert_eq!(drdata(&nest, 2), 5);
    assert_eq!(nest.sprite, 0);
}
