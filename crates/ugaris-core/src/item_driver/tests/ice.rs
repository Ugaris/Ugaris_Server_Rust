use super::*;

#[test]
fn ice_shared_itemspawn_maps_legacy_templates_and_blocks_cursor() {
    let mut actor = character(1);
    let mut spawner = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_ITEMSPAWN);
    spawner.driver_data = vec![17];
    let request = ItemDriverRequest::Driver {
        driver: IDR_ITEMSPAWN,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut spawner, request, 11, false),
        ItemDriverOutcome::IceItemSpawn {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            template: "palace_bomb",
        }
    );

    actor.cursor_item = Some(ItemId(9));
    assert_eq!(
        execute_item_driver(&mut actor, &mut spawner, request, 11, false),
        ItemDriverOutcome::IceItemSpawnCursorOccupied {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    actor.cursor_item = None;
    spawner.driver_data = vec![19];
    assert_eq!(
        execute_item_driver(&mut actor, &mut spawner, request, 10, false),
        ItemDriverOutcome::IceItemSpawnBug {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            kind: 19,
        }
    );
}

#[test]
fn ice_shared_warmfire_reports_scroll_and_curse_branches() {
    let mut actor = character(1);
    let mut fire = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_WARMFIRE);
    fire.driver_data = vec![0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_WARMFIRE,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut fire,
            request,
            10,
            false,
            &ItemDriverContext {
                has_curse_spell: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::WarmFire {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            create_scroll: true,
            removed_curse: true,
        }
    );

    actor.cursor_item = Some(ItemId(9));
    assert_eq!(
        execute_item_driver(&mut actor, &mut fire, request, 10, false),
        ItemDriverOutcome::WarmFireCursorOccupied {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn ice_shared_backtofire_and_melting_key_match_timer_core() {
    let mut actor = character(1);
    let mut scroll = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_BACKTOFIRE);
    scroll.carried_by = Some(CharacterId(1));
    scroll.driver_data = vec![123, 45];
    assert_eq!(
        execute_item_driver(
            &mut actor,
            &mut scroll,
            ItemDriverRequest::Driver {
                driver: IDR_BACKTOFIRE,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            10,
            false,
        ),
        ItemDriverOutcome::BackToFire {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            x: 123,
            y: 45,
        }
    );

    let mut timer = character(0);
    let mut key = item(8, ItemFlags::USED, 0, IDR_MELTINGKEY);
    key.carried_by = Some(CharacterId(1));
    key.sprite = 50494;
    key.driver_data = vec![5, 0];
    assert_eq!(
        execute_item_driver(
            &mut timer,
            &mut key,
            ItemDriverRequest::Driver {
                driver: IDR_MELTINGKEY,
                item_id: ItemId(8),
                character_id: CharacterId(0),
                spec: 0,
            },
            11,
            false,
        ),
        ItemDriverOutcome::MeltingKeyTick {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            melted: false,
            started_melting: true,
            schedule_after_ticks: Some(TICKS_PER_SECOND * 10),
        }
    );
    assert_eq!(key.driver_data[1], 1);
    assert_eq!(key.sprite, 50495);

    key.driver_data = vec![5, 4];
    assert_eq!(
        execute_item_driver(
            &mut timer,
            &mut key,
            ItemDriverRequest::Driver {
                driver: IDR_MELTINGKEY,
                item_id: ItemId(8),
                character_id: CharacterId(0),
                spec: 0,
            },
            11,
            false,
        ),
        ItemDriverOutcome::MeltingKeyTick {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            melted: true,
            started_melting: false,
            schedule_after_ticks: None,
        }
    );
}

#[test]
fn nomad_dice_driver_requires_carried_item_and_reports_luck() {
    let mut character = character(1);
    let mut dice = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_NOMADDICE);
    set_drdata(&mut dice, 0, 2);
    let request = ItemDriverRequest::Driver {
        driver: IDR_NOMADDICE,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut dice, request, 19, false),
        ItemDriverOutcome::Noop
    );

    dice.carried_by = Some(CharacterId(1));
    assert_eq!(
        execute_item_driver(&mut character, &mut dice, request, 19, false),
        ItemDriverOutcome::NomadDice {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            luck: 2,
        }
    );

    assert_eq!(
        execute_item_driver(&mut character, &mut dice, request, 1, false),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_NOMADDICE,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            required_area: 19,
        }
    );
}

#[test]
fn execute_freakdoor_driver_returns_link_metadata() {
    let mut character = character(1);
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FREAKDOOR);
    door.driver_data = vec![0; 16];
    door.driver_data[8] = 42;
    door.driver_data[10..14].copy_from_slice(&99_u32.to_le_bytes());
    door.driver_data[14] = 1;
    door.driver_data[15] = 1;
    let request = ItemDriverRequest::Driver {
        driver: IDR_FREAKDOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut door, request, 10, false),
        ItemDriverOutcome::FreakDoorUse {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            link_group: 42,
            one_way: true,
            recursion_guard: false,
            cached_partner_id: Some(ItemId(99)),
            no_target: true,
        }
    );
}
