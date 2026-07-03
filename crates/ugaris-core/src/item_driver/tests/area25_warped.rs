use super::*;

#[test]
fn warpteleport_non_keyed_destinations_match_legacy_table() {
    let mut actor = character(1);
    let mut portal = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_WARPTELEPORT);
    set_drdata(&mut portal, 1, 3);
    let request = ItemDriverRequest::Driver {
        driver: IDR_WARPTELEPORT,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut portal, request, 25, false),
        ItemDriverOutcome::Teleport {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            x: 251,
            y: 16,
            area_id: 25,
            stop_driver: true,
            quiet: true,
        }
    );
}

#[test]
fn warpteleport_keyed_sphere_destinations_match_legacy_table() {
    let mut actor = character(1);
    actor.cursor_item = Some(ItemId(9));
    let mut portal = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_WARPTELEPORT);
    set_drdata(&mut portal, 0, 4);
    let request = ItemDriverRequest::Driver {
        driver: IDR_WARPTELEPORT,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };
    let context = ItemDriverContext {
        cursor_template_id: Some(IID_AREA25_TELEKEY),
        cursor_drdata0: Some(2),
        ..ItemDriverContext::default()
    };

    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut portal, request, 25, false, &context),
        ItemDriverOutcome::WarpTeleportSpheres {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(9),
            x: 207,
            y: 227,
        }
    );
}

#[test]
fn warpteleport_keyed_requires_cursor_sphere() {
    let mut actor = character(1);
    let mut portal = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_WARPTELEPORT);
    set_drdata(&mut portal, 0, 1);
    let request = ItemDriverRequest::Driver {
        driver: IDR_WARPTELEPORT,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut portal, request, 25, false),
        ItemDriverOutcome::WarpTeleportMissingSphere {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn warpteleport_invalid_plain_portal_reports_legacy_bug() {
    let mut actor = character(1);
    let mut portal = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_WARPTELEPORT);
    set_drdata(&mut portal, 1, 9);
    let request = ItemDriverRequest::Driver {
        driver: IDR_WARPTELEPORT,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut portal, request, 25, false),
        ItemDriverOutcome::WarpTeleportBug {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn warpteleport_is_area25_libload_gated() {
    let mut actor = character(1);
    let mut portal = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_WARPTELEPORT);
    set_drdata(&mut portal, 1, 1);
    let request = ItemDriverRequest::Driver {
        driver: IDR_WARPTELEPORT,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut portal, request, 1, false),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_WARPTELEPORT,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            required_area: 25,
        }
    );
}

#[test]
fn warpbonus_dispatch_ports_level_and_location_state() {
    let mut actor = character(1);
    actor.level = 70;
    let mut bonus = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_WARPBONUS);
    bonus.x = 12;
    bonus.y = 34;
    let request = ItemDriverRequest::Driver {
        driver: IDR_WARPBONUS,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };
    let context = ItemDriverContext {
        warp_bonus_base: Some(50),
        warp_bonus_points: 3,
        ..ItemDriverContext::default()
    };

    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut bonus, request, 25, false, &context),
        ItemDriverOutcome::WarpBonus {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            location_id: 12 + (34 << 8) + (25 << 16),
            base: 50,
            next_points: 4,
            advanced: false,
            reward_sphere_kind: None,
            reward_level: 40,
        }
    );
}

#[test]
fn warpbonus_requires_sphere_on_advancing_touch() {
    let mut actor = character(1);
    actor.level = 50;
    actor.cursor_item = Some(ItemId(9));
    let mut bonus = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_WARPBONUS);
    let request = ItemDriverRequest::Driver {
        driver: IDR_WARPBONUS,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };
    let missing_sphere = ItemDriverContext {
        warp_bonus_base: Some(40),
        warp_bonus_points: 9,
        ..ItemDriverContext::default()
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut bonus,
            request,
            25,
            false,
            &missing_sphere
        ),
        ItemDriverOutcome::WarpBonusNeedsSphere {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    let with_sphere = ItemDriverContext {
        warp_bonus_base: Some(40),
        warp_bonus_points: 9,
        cursor_template_id: Some(IID_AREA25_TELEKEY),
        cursor_drdata0: Some(4),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut bonus, request, 25, false, &with_sphere),
        ItemDriverOutcome::WarpBonus {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            location_id: 25 << 16,
            base: 40,
            next_points: 0,
            advanced: true,
            reward_sphere_kind: Some(4),
            reward_level: 32,
        }
    );
}

#[test]
fn warpbonus_blocks_finished_and_already_used_state() {
    let mut actor = character(1);
    let mut bonus = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_WARPBONUS);
    let request = ItemDriverRequest::Driver {
        driver: IDR_WARPBONUS,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut bonus,
            request,
            25,
            false,
            &ItemDriverContext {
                warp_bonus_base: Some(140),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::WarpBonusFinished {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut bonus,
            request,
            25,
            false,
            &ItemDriverContext {
                warp_bonus_base: Some(40),
                warp_bonus_used_at_base: Some(40),
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::WarpBonusAlreadyUsed {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn warpkeyspawn_creates_typed_template_outcome() {
    let mut actor = character(1);
    let mut spawner = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_WARPKEYSPAWN);
    set_drdata(&mut spawner, 0, 4);
    let request = ItemDriverRequest::Driver {
        driver: IDR_WARPKEYSPAWN,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut spawner, request, 25, false),
        ItemDriverOutcome::WarpKeySpawn {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            sphere_kind: 4,
        }
    );
}

#[test]
fn warpkeyspawn_preserves_legacy_dynamic_template_name_for_invalid_kind() {
    let mut actor = character(1);
    let mut spawner = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_WARPKEYSPAWN);
    set_drdata(&mut spawner, 0, 6);
    let request = ItemDriverRequest::Driver {
        driver: IDR_WARPKEYSPAWN,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut spawner, request, 25, false),
        ItemDriverOutcome::WarpKeySpawn {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            sphere_kind: 6,
        }
    );
}

#[test]
fn warpkeyspawn_requires_empty_cursor() {
    let mut actor = character(1);
    actor.cursor_item = Some(ItemId(99));
    let mut spawner = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_WARPKEYSPAWN);
    let request = ItemDriverRequest::Driver {
        driver: IDR_WARPKEYSPAWN,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut spawner, request, 25, false),
        ItemDriverOutcome::WarpKeySpawnCursorOccupied {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn warptrialdoor_returns_spawn_and_player_teleport_boundary() {
    let mut actor = character(1);
    actor.x = 9;
    actor.y = 12;
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_WARPTRIALDOOR);
    door.x = 10;
    door.y = 12;
    let request = ItemDriverRequest::Driver {
        driver: IDR_WARPTRIALDOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };
    let context = ItemDriverContext {
        warp_trial_door: Some(WarpTrialDoorContext {
            xs: 10,
            ys: 10,
            xe: 20,
            ye: 20,
            partner_x: 20,
            partner_y: 12,
            room_has_non_simple_baddy: false,
        }),
        ..ItemDriverContext::default()
    };

    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut door, request, 25, false, &context),
        ItemDriverOutcome::WarpTrialDoor {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spawn_x: 15,
            spawn_y: 15,
            player_x: 11,
            player_y: 12,
            fighter_target_x: 21,
            fighter_target_y: 12,
            xs: 10,
            ys: 10,
            xe: 20,
            ye: 20,
            template: "warped_fighter",
        }
    );
}

#[test]
fn warptrialdoor_blocks_inside_and_busy_rooms() {
    let mut actor = character(1);
    actor.x = 15;
    actor.y = 15;
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_WARPTRIALDOOR);
    door.x = 10;
    door.y = 12;
    let request = ItemDriverRequest::Driver {
        driver: IDR_WARPTRIALDOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };
    let trial = WarpTrialDoorContext {
        xs: 10,
        ys: 10,
        xe: 20,
        ye: 20,
        partner_x: 20,
        partner_y: 12,
        room_has_non_simple_baddy: false,
    };
    let context = ItemDriverContext {
        warp_trial_door: Some(trial),
        ..ItemDriverContext::default()
    };

    let wrong_side =
        execute_item_driver_with_context(&mut actor, &mut door, request, 25, false, &context);
    assert_eq!(
        wrong_side,
        ItemDriverOutcome::WarpTrialDoorWrongSide {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(
        legacy_item_driver_return_code(Some(IDR_WARPTRIALDOOR), &wrong_side),
        2
    );

    actor.x = 9;
    actor.y = 12;
    let busy_context = ItemDriverContext {
        warp_trial_door: Some(WarpTrialDoorContext {
            room_has_non_simple_baddy: true,
            ..trial
        }),
        ..ItemDriverContext::default()
    };
    let busy =
        execute_item_driver_with_context(&mut actor, &mut door, request, 25, false, &busy_context);
    assert_eq!(
        busy,
        ItemDriverOutcome::WarpTrialDoorBusy {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(
        legacy_item_driver_return_code(Some(IDR_WARPTRIALDOOR), &busy),
        2
    );
}

#[test]
fn warptrialdoor_zero_character_call_matches_c_noop_return_code() {
    let mut timer = character(0);
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_WARPTRIALDOOR);
    let request = ItemDriverRequest::Driver {
        driver: IDR_WARPTRIALDOOR,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    let outcome = execute_item_driver_with_context(
        &mut timer,
        &mut door,
        request,
        25,
        false,
        &ItemDriverContext::default(),
    );

    assert_eq!(outcome, ItemDriverOutcome::Noop);
    assert_eq!(
        legacy_item_driver_return_code(Some(IDR_WARPTRIALDOOR), &outcome),
        2
    );
}

#[test]
fn warpkeydoor_requires_exact_carried_key() {
    let mut actor = character(1);
    actor.x = 10;
    actor.y = 20;
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_WARPKEYDOOR);
    door.x = 11;
    door.y = 20;
    let request = ItemDriverRequest::Driver {
        driver: IDR_WARPKEYDOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    let outcome = execute_item_driver(&mut actor, &mut door, request, 25, false);

    assert_eq!(
        outcome,
        ItemDriverOutcome::WarpKeyDoorMissingKey {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(
        legacy_item_driver_return_code(Some(IDR_WARPKEYDOOR), &outcome),
        2
    );
}

#[test]
fn warpkeydoor_teleports_through_door_and_consumes_key() {
    let mut actor = character(1);
    actor.x = 10;
    actor.y = 20;
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_WARPKEYDOOR);
    door.x = 11;
    door.y = 20;
    let request = ItemDriverRequest::Driver {
        driver: IDR_WARPKEYDOOR,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };
    let context = ItemDriverContext {
        area25_door_key: Some((ItemId(9), "Warper Key".to_string())),
        ..ItemDriverContext::default()
    };

    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut door, request, 25, false, &context),
        ItemDriverOutcome::WarpKeyDoor {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            key_item_id: ItemId(9),
            key_name: outcome_item_name("Warper Key"),
            x: 12,
            y: 20,
        }
    );
}

#[test]
fn warpkeydoor_zero_character_call_is_c_handled_noop() {
    let mut actor = character(0);
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_WARPKEYDOOR);
    let request = ItemDriverRequest::Driver {
        driver: IDR_WARPKEYDOOR,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    let outcome = execute_item_driver(&mut actor, &mut door, request, 25, false);

    assert_eq!(outcome, ItemDriverOutcome::Noop);
    assert_eq!(
        legacy_item_driver_return_code(Some(IDR_WARPKEYDOOR), &outcome),
        1
    );
}
