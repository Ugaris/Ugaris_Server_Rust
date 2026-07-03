use super::*;

#[test]
fn bookcase_driver_ports_locked_and_text_boundaries() {
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    let request = ItemDriverRequest::Driver {
        driver: IDR_BOOKCASE,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };
    let mut bookcase = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BOOKCASE);
    bookcase.driver_data = vec![1];

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut bookcase,
            request,
            17,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::BookcaseLocked {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    let context = ItemDriverContext {
        has_area17_library_key: true,
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut bookcase, request, 17, false, &context),
        ItemDriverOutcome::BookcaseText {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            kind: 1,
        }
    );
}

#[test]
fn bookcase_text_matches_legacy_title_format() {
    let locked = bookcase_locked_text_lines();
    assert_eq!(
        locked[0],
        "The bookcase is locked and you do not have the right key."
    );
    assert_eq!(
        bookcase_text_line_bytes(3, 0, 2, false),
        [
            crate::text::COL_LIGHT_GREEN,
            b"A Green Day in the Life of a Warrior by C. O. Nan.",
            crate::text::COL_RESET,
            b" After reading the title you put the book back.",
        ]
        .concat()
    );
    assert_eq!(
        bookcase_text_line_bytes(0, 3, 1, false),
        [
            crate::text::COL_LIGHT_GREEN,
            b"Secrets of Adygalah Alchemy by Leonarda.",
            crate::text::COL_RESET,
            b" One recipe most mages will find useful uses Adygalah, Bhalkissa and Firuba, plus one berry and one or two mushrooms.",
        ]
        .concat()
    );
    assert_eq!(bookcase_library_exp(1), 3);
    assert_eq!(bookcase_library_exp(60), 80_000);
}

#[test]
fn pick_chest_requires_lockpick_and_empty_cursor() {
    let mut character = character(1);
    let mut chest = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_PICKCHEST);
    chest.driver_data = vec![2];
    let request = ItemDriverRequest::Driver {
        driver: IDR_PICKCHEST,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut chest,
            request,
            17,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::PickChestLocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    character.cursor_item = Some(ItemId(9));
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut chest,
            request,
            17,
            false,
            &ItemDriverContext {
                has_area17_lockpick: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::PickChestCursorOccupied {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn pick_chest_maps_legacy_note_kinds() {
    let mut character = character(1);
    let mut chest = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_PICKCHEST);
    let request = ItemDriverRequest::Driver {
        driver: IDR_PICKCHEST,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };
    let context = ItemDriverContext {
        has_area17_lockpick: true,
        ..ItemDriverContext::default()
    };

    for (kind, template) in [
        (0, PickChestTemplate::PalaceNote1),
        (1, PickChestTemplate::PalaceNote2),
        (2, PickChestTemplate::PalaceNote3),
        (3, PickChestTemplate::MerchantNote1),
    ] {
        chest.driver_data = vec![kind];
        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut chest,
                request,
                17,
                false,
                &context,
            ),
            ItemDriverOutcome::PickChest {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                template,
            }
        );
    }

    chest.driver_data = vec![4];
    assert_eq!(
        execute_item_driver_with_context(&mut character, &mut chest, request, 17, false, &context,),
        ItemDriverOutcome::PickChestBug {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn pick_door_requires_area17_lockpick_for_players() {
    let mut character = character(1);
    character.flags.insert(CharacterFlags::PLAYER);
    let mut door = item(
        7,
        ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK | ItemFlags::DOOR,
        0,
        IDR_PICKDOOR,
    );
    door.driver_data = vec![0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_PICKDOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut door,
            request,
            17,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::PickDoorLocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut door,
            request,
            17,
            false,
            &ItemDriverContext {
                has_area17_lockpick: true,
                has_area17_cursor_lockpick: false,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::PickDoorLocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut door,
            request,
            17,
            false,
            &ItemDriverContext {
                has_area17_cursor_lockpick: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::PickDoorToggle {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            picked_lock: true,
        }
    );
}

#[test]
fn pick_door_timer_only_closes_open_doors() {
    let mut timer = character(0);
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_PICKDOOR);
    let request = ItemDriverRequest::Driver {
        driver: IDR_PICKDOOR,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    door.driver_data = vec![0];
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer,
            &mut door,
            request,
            17,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::Noop
    );

    door.driver_data = vec![1];
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer,
            &mut door,
            request,
            17,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::PickDoorToggle {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            picked_lock: false,
        }
    );
}

#[test]
fn burndown_driver_ports_touch_ignite_and_timer_gates() {
    let mut actor = character(1);
    let mut barrel = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_BURNDOWN);
    let request = ItemDriverRequest::Driver {
        driver: IDR_BURNDOWN,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    barrel.driver_data = vec![0];
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut barrel,
            request,
            17,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::BurndownTouch {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut barrel,
            request,
            17,
            false,
            &ItemDriverContext {
                cursor_driver: Some(IDR_TORCH),
                cursor_drdata0: Some(1),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::BurndownIgnite {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    barrel.driver_data = vec![16];
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut barrel,
            request,
            17,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::BurndownTooHot {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    barrel.driver_data = vec![1];
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut barrel,
            request,
            17,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::BurndownAlreadyBurned {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    let mut timer = character(0);
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer,
            &mut barrel,
            ItemDriverRequest::Driver {
                driver: IDR_BURNDOWN,
                item_id: ItemId(7),
                character_id: CharacterId(0),
                spec: 0,
            },
            17,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::BurndownTimerTick { item_id: ItemId(7) }
    );
}

#[test]
fn colortile_reports_legacy_row_and_color_for_players_only() {
    let mut character = character(1);
    character.flags.insert(CharacterFlags::PLAYER);
    let mut tile = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_COLORTILE);
    tile.driver_data = vec![3, 5];
    let request = ItemDriverRequest::Driver {
        driver: IDR_COLORTILE,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut tile,
            request,
            17,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::ColorTile {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            row: 3,
            color: 5,
        }
    );

    character.flags.remove(CharacterFlags::PLAYER);
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut tile,
            request,
            17,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn skelraise_dispatches_blood_bowl_and_dust_paths() {
    let mut character = character(42);
    character.flags.insert(CharacterFlags::PLAYER);
    let mut chair = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SKELRAISE);
    chair.driver_data = vec![2, 0, 0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_SKELRAISE,
        item_id: ItemId(7),
        character_id: CharacterId(42),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut chair, request, 17, false),
        ItemDriverOutcome::SkelRaiseDust {
            item_id: ItemId(7),
            character_id: CharacterId(42),
        }
    );

    character.cursor_item = Some(ItemId(9));
    let outcome = execute_item_driver_with_context(
        &mut character,
        &mut chair,
        request,
        17,
        false,
        &ItemDriverContext {
            cursor_template_id: Some(IID_AREA17_BLOODBOWL),
            ..ItemDriverContext::default()
        },
    );
    assert_eq!(
        outcome,
        ItemDriverOutcome::SkelRaiseRaise {
            item_id: ItemId(7),
            character_id: CharacterId(42),
            cursor_item_id: ItemId(9),
            template: "raised_skeleton_green_key",
        }
    );
}

#[test]
fn skelraise_active_chair_and_timer_paths_match_c_boundary() {
    let mut character = character(42);
    let mut chair = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SKELRAISE);
    chair.driver_data = vec![1, 0, 1];
    let request = ItemDriverRequest::Driver {
        driver: IDR_SKELRAISE,
        item_id: ItemId(7),
        character_id: CharacterId(42),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut chair, request, 17, false),
        ItemDriverOutcome::SkelRaiseTouch {
            item_id: ItemId(7),
            character_id: CharacterId(42),
        }
    );
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut chair,
            request,
            17,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::SkelRaiseTimer { item_id: ItemId(7) }
    );
}
