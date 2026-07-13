use super::*;

fn shrike_item(id: u32, sub_driver: u8) -> Item {
    let mut it = item(id, ItemFlags::USED | ItemFlags::USE, 0, IDR_SHRIKE);
    it.driver_data = vec![sub_driver];
    it
}

fn shrike_request(item_id: u32, character_id: u32) -> ItemDriverRequest {
    ItemDriverRequest::Driver {
        driver: IDR_SHRIKE,
        item_id: ItemId(item_id),
        character_id: CharacterId(character_id),
        spec: 0,
    }
}

fn night_context() -> ItemDriverContext {
    ItemDriverContext {
        is_fullnight: true,
        ..ItemDriverContext::default()
    }
}

// C `shrike_driver`'s `switch (it[in].drdata[0])` (`shrike.c:356-377`)
// has no `default:` case, matching every other unrecognized-sub-driver
// dispatch in this codebase's `Unsupported` fallback.
#[test]
fn shrike_driver_rejects_unknown_sub_driver() {
    let mut character = character(1);
    let mut door = shrike_item(1, 99);
    assert_eq!(
        execute_item_driver(&mut character, &mut door, shrike_request(1, 1), 38, false),
        ItemDriverOutcome::Unsupported {
            driver: IDR_SHRIKE,
            item_id: ItemId(1),
            character_id: CharacterId(1),
        }
    );
}

// C `tree_driver`'s `!cn` branch (`shrike.c:88-104`): the ambient sprite
// refresh runs regardless of `is_fullnight()`, just picking between the
// two literal sprite/description pairs.
#[test]
fn tree_driver_automatic_call_refreshes_ambient_sprite_for_both_phases() {
    let mut character = character(0);
    let mut tree = shrike_item(1, 1);

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut tree,
            shrike_request(1, 0),
            38,
            false,
            &night_context(),
        ),
        ItemDriverOutcome::ShrikeAmbientRefresh {
            item_id: ItemId(1),
            x: 0,
            y: 0,
            kind: ShrikeAmbientKind::Tree,
            night: true,
            schedule_after_ticks: TICKS_PER_SECOND * 60,
        }
    );
    assert_eq!(
        execute_item_driver(&mut character, &mut tree, shrike_request(1, 0), 38, false),
        ItemDriverOutcome::ShrikeAmbientRefresh {
            item_id: ItemId(1),
            x: 0,
            y: 0,
            kind: ShrikeAmbientKind::Tree,
            night: false,
            schedule_after_ticks: TICKS_PER_SECOND * 60,
        }
    );
}

// C `tree_driver`'s player branch outside full night (`shrike.c:106`):
// silently does nothing.
#[test]
fn tree_driver_player_touch_is_noop_outside_full_night() {
    let mut character = character(1);
    let mut tree = shrike_item(1, 1);
    assert_eq!(
        execute_item_driver(&mut character, &mut tree, shrike_request(1, 1), 38, false),
        ItemDriverOutcome::Noop
    );
}

// C `tree_driver`'s occupied-cursor branch (`shrike.c:107-110`).
#[test]
fn tree_driver_full_night_with_occupied_hand_reports_needs_empty_hand() {
    let mut character = character(1);
    character.cursor_item = Some(ItemId(9));
    let mut tree = shrike_item(1, 1);
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut tree,
            shrike_request(1, 1),
            38,
            false,
            &night_context(),
        ),
        ItemDriverOutcome::ShrikeHandOccupied {
            item_id: ItemId(1),
            character_id: CharacterId(1),
        }
    );
}

// C `tree_driver`'s success branch (`shrike.c:113-123`, `create_item(
// "shrike_amulet2")`) and `pede_driver`'s (`:156-166`, `"shrike_
// amulet1"`).
#[test]
fn tree_and_pede_driver_full_night_empty_hand_gives_matching_amulet_piece() {
    let mut character = character(1);
    let mut tree = shrike_item(1, 1);
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut tree,
            shrike_request(1, 1),
            38,
            false,
            &night_context(),
        ),
        ItemDriverOutcome::ShrikeGiveAmuletPiece {
            item_id: ItemId(1),
            character_id: CharacterId(1),
            piece: ShrikeAmuletPiece::Chain,
        }
    );

    let mut pede = shrike_item(2, 6);
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut pede,
            shrike_request(2, 1),
            38,
            false,
            &night_context(),
        ),
        ItemDriverOutcome::ShrikeGiveAmuletPiece {
            item_id: ItemId(2),
            character_id: CharacterId(1),
            piece: ShrikeAmuletPiece::Crystal,
        }
    );
}

// C `rock_driver`'s three player-touch branches (`shrike.c:189-212`).
#[test]
fn rock_driver_requires_a_carried_forestspade() {
    let mut character = character(1);
    let mut rock = shrike_item(1, 2);

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut rock,
            shrike_request(1, 1),
            38,
            false,
            &night_context(),
        ),
        ItemDriverOutcome::ShrikeRockNoTool {
            character_id: CharacterId(1),
        }
    );

    character.cursor_item = Some(ItemId(9));
    let wrong_tool_context = ItemDriverContext {
        is_fullnight: true,
        cursor_driver: Some(IDR_TORCH),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut rock,
            shrike_request(1, 1),
            38,
            false,
            &wrong_tool_context,
        ),
        ItemDriverOutcome::ShrikeRockWrongTool {
            character_id: CharacterId(1),
        }
    );

    let spade_context = ItemDriverContext {
        is_fullnight: true,
        cursor_driver: Some(IDR_FORESTSPADE),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut rock,
            shrike_request(1, 1),
            38,
            false,
            &spade_context,
        ),
        ItemDriverOutcome::ShrikeRockDigSuccess {
            item_id: ItemId(1),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(9),
            piece: ShrikeAmuletPiece::Charm,
        }
    );
}

// C `door_driver` (`shrike.c:224-260`): level gate, then talisman gate,
// then the teleport outcome.
#[test]
fn door_driver_gates_level_then_talisman() {
    let mut character = character(1);
    character.level = 40;
    let mut door = shrike_item(1, 3);
    assert_eq!(
        execute_item_driver(&mut character, &mut door, shrike_request(1, 1), 38, false),
        ItemDriverOutcome::ShrikeDoorTooWeak {
            character_id: CharacterId(1),
        }
    );

    character.level = 65;
    assert_eq!(
        execute_item_driver(&mut character, &mut door, shrike_request(1, 1), 38, false),
        ItemDriverOutcome::ShrikeDoorNeedsTalisman {
            character_id: CharacterId(1),
        }
    );

    let talisman_context = ItemDriverContext {
        cursor_template_id: Some(IID_SHRIKE_TALISMAN),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut door,
            shrike_request(1, 1),
            38,
            false,
            &talisman_context,
        ),
        ItemDriverOutcome::ShrikeDoorEnter {
            character_id: CharacterId(1),
        }
    );
}

// C `pool_driver` (`shrike.c:262-281`): `if (!cn) return;` first, then
// the empty-cursor/not-ready/success ladder.
#[test]
fn pool_driver_ignores_automatic_calls_and_gates_on_amulet_and_night() {
    let mut timer_character = character(0);
    let mut pool = shrike_item(1, 4);
    assert_eq!(
        execute_item_driver(
            &mut timer_character,
            &mut pool,
            shrike_request(1, 0),
            38,
            false
        ),
        ItemDriverOutcome::Noop
    );

    let mut character = character(1);
    assert_eq!(
        execute_item_driver(&mut character, &mut pool, shrike_request(1, 1), 38, false),
        ItemDriverOutcome::ShrikePoolSweetWater {
            character_id: CharacterId(1),
        }
    );

    character.cursor_item = Some(ItemId(9));
    let not_amulet_context = ItemDriverContext {
        is_fullnight: true,
        cursor_driver: Some(IDR_TORCH),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut pool,
            shrike_request(1, 1),
            38,
            false,
            &not_amulet_context,
        ),
        ItemDriverOutcome::ShrikePoolWetItem {
            character_id: CharacterId(1),
            cursor_item_id: ItemId(9),
        }
    );

    let ready_context = ItemDriverContext {
        is_fullnight: true,
        cursor_driver: Some(IDR_SHRIKEAMULET),
        cursor_drdata0: Some(7),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut pool,
            shrike_request(1, 1),
            38,
            false,
            &ready_context,
        ),
        ItemDriverOutcome::ShrikePoolTalismanCreated {
            character_id: CharacterId(1),
            cursor_item_id: ItemId(9),
        }
    );
}

// C `cube_driver`'s player-push branch (`shrike.c:283-310`): blocked vs.
// a legal target tile.
#[test]
fn cube_driver_player_push_blocked_or_moves_to_context_target() {
    let mut character = character(1);
    let mut cube = shrike_item(1, 5);
    cube.x = 10;
    cube.y = 10;

    assert_eq!(
        execute_item_driver(&mut character, &mut cube, shrike_request(1, 1), 38, false),
        ItemDriverOutcome::ShrikeCubeBlocked {
            character_id: CharacterId(1),
        }
    );

    let push_context = ItemDriverContext {
        shrike_cube_push_target: Some((11, 10)),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut cube,
            shrike_request(1, 1),
            38,
            false,
            &push_context,
        ),
        ItemDriverOutcome::ShrikeCubePush {
            item_id: ItemId(1),
            character_id: CharacterId(1),
            from_x: 10,
            from_y: 10,
            to_x: 11,
            to_y: 10,
        }
    );
}

// C `cube_driver`'s `cn == 0` branch (`shrike.c:312-341`): the very
// first automatic tick remembers the current tile as the origin (and
// never resets, since it can't have moved away from itself yet).
#[test]
fn cube_driver_automatic_tick_records_origin_on_first_call() {
    let mut character = character(0);
    let mut cube = shrike_item(1, 5);
    cube.x = 10;
    cube.y = 10;
    assert_eq!(
        execute_item_driver(&mut character, &mut cube, shrike_request(1, 0), 38, false),
        ItemDriverOutcome::ShrikeCubeAmbientTick {
            item_id: ItemId(1),
            set_origin: Some((10, 10)),
            reset_to: None,
            schedule_after_ticks: TICKS_PER_SECOND * 5,
        }
    );
}

// C `cube_driver`'s 15-minute idle auto-reset (`shrike.c:334-341`):
// fires only once the cube has moved away from its recorded origin, sat
// untouched past the threshold, and the origin tile itself is clear.
#[test]
fn cube_driver_automatic_tick_resets_after_idle_threshold() {
    let mut character = character(0);
    let mut cube = shrike_item(1, 5);
    cube.x = 12;
    cube.y = 10;
    set_drdata_u16(&mut cube, 8, 10);
    set_drdata_u16(&mut cube, 10, 10);
    set_drdata_u32(&mut cube, 4, 100);

    let too_soon_context = ItemDriverContext {
        current_tick: 100 + (TICKS_PER_SECOND as u32) * 60 * 15,
        shrike_cube_origin_clear: Some(true),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut cube,
            shrike_request(1, 0),
            38,
            false,
            &too_soon_context,
        ),
        ItemDriverOutcome::ShrikeCubeAmbientTick {
            item_id: ItemId(1),
            set_origin: None,
            reset_to: None,
            schedule_after_ticks: TICKS_PER_SECOND * 5,
        }
    );

    let idle_context = ItemDriverContext {
        current_tick: 100 + (TICKS_PER_SECOND as u32) * 60 * 15 + 1,
        shrike_cube_origin_clear: Some(true),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut cube,
            shrike_request(1, 0),
            38,
            false,
            &idle_context,
        ),
        ItemDriverOutcome::ShrikeCubeAmbientTick {
            item_id: ItemId(1),
            set_origin: None,
            reset_to: Some((10, 10)),
            schedule_after_ticks: TICKS_PER_SECOND * 5,
        }
    );
}
