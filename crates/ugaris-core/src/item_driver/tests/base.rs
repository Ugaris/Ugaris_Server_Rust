use super::*;

#[test]
fn item_driver_constants_cover_legacy_drvlib_surface() {
    assert_eq!(IDR_CLANSPAWN, 20);
    assert_eq!(IDR_CLANVAULT, 22);
    assert_eq!(IDR_CHESTSPAWN, 27);
    assert_eq!(IDR_PENT, 30);
    assert_eq!(IDR_EDEMONSWITCH, 37);
    assert_eq!(IDR_EDEMONTUBE, 43);
    assert_eq!(IDR_FDEMONLIGHT, 44);
    assert_eq!(IDR_FDEMONLAVA, 51);
    assert_eq!(IDR_ITEMSPAWN, 53);
    assert_eq!(IDR_WARMFIRE, 54);
    assert_eq!(IDR_FREAKDOOR, 58);
    assert_eq!(IDR_MINEWALL, 60);
    assert_eq!(IDR_TOPLIST, 63);
    assert_eq!(IDR_DUNGEONTELE, 65);
    assert_eq!(IDR_SWAMPWHISP, 74);
    assert_eq!(IDR_SWAMPSPAWN, 75);
    assert_eq!(IDR_PALACEDOOR, 76);
    assert_eq!(IDR_BONEHOLDER, 91);
    assert_eq!(IDR_LFREDUCT, 97);
    assert_eq!(IDR_LQ_KEY, 100);
    assert_eq!(IDR_STR_DEPOT, 108);
    assert_eq!(IDR_RATCHEST, 111);
    assert_eq!(IDR_WARPTRIALDOOR, 113);
    assert_eq!(IDR_WARPBONUS, 114);
    assert_eq!(IDR_WARPKEYDOOR, 116);
    assert_eq!(IDR_STAFFER, 121);
    assert_eq!(IDR_MINEGATEWAY, 127);
    assert_eq!(IDR_ISLENADOOR, 138);
    assert_eq!(IDR_TEUFELARENAEXIT, 141);
    assert_eq!(IDR_SALTMINE_ITEM, 188);
    assert_eq!(IDR_LAB5_ITEM, 190);
    assert_eq!(IDR_LABTORCH, 199);
    assert_eq!(IDR_SKELETON_KEY, 201);
}

#[test]
fn use_item_opens_container_before_driver_dispatch() {
    let mut character = character(1);
    let mut item = item(7, ItemFlags::USED | ItemFlags::USE, 22, IDR_POTION);

    let outcome = use_item(&mut character, &item, request(1, 7, 0), false).unwrap();

    assert_eq!(
        outcome,
        UseItemOutcome::OpenContainer { item_id: ItemId(7) }
    );
    assert_eq!(character.current_container, Some(ItemId(7)));

    item.content_id = 0;
    let outcome = use_item(&mut character, &item, request(1, 7, 5), false).unwrap();
    assert_eq!(
        outcome,
        UseItemOutcome::Dispatch(ItemDriverRequest::Driver {
            driver: IDR_POTION,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 5,
        })
    );
}

#[test]
fn use_item_opens_depot_and_account_depot_like_legacy_order() {
    let mut character = character(1);
    let depot = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::DEPOT, 0, 0);
    let outcome = use_item(&mut character, &depot, request(1, 7, 0), false).unwrap();
    assert_eq!(outcome, UseItemOutcome::OpenDepot { item_id: ItemId(7) });

    let account_depot = item(
        8,
        ItemFlags::USED | ItemFlags::USE | ItemFlags::DEPOT,
        0,
        IDR_ACCOUNT_DEPOT,
    );
    assert_eq!(
        use_item(&mut character, &account_depot, request(1, 8, 0), false),
        Err(UseItemError::AccountDepotUnavailable)
    );
    assert_eq!(
        use_item(&mut character, &account_depot, request(1, 8, 0), true).unwrap(),
        UseItemOutcome::OpenAccountDepot { item_id: ItemId(8) }
    );
    assert_eq!(character.current_container, Some(ItemId(8)));
}

#[test]
fn account_depot_driver_request_is_supported_for_non_use_paths() {
    let mut character = character(1);
    let mut depot = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_ACCOUNT_DEPOT);

    assert_eq!(
        execute_item_driver_with_context(
            &mut character,
            &mut depot,
            ItemDriverRequest::AccountDepot {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            },
            1,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::AccountDepotOpened {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn identity_tag_item_drivers_are_handled_noops_like_legacy_libload() {
    let mut character = character(1);
    let mut tagged = item(8, ItemFlags::USED | ItemFlags::USE, 0, 1000);
    let request = ItemDriverRequest::Driver {
        driver: 1000,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    let outcome = execute_item_driver(&mut character, &mut tagged, request, 1, false);

    assert_eq!(
        outcome,
        ItemDriverOutcome::IdentityTag {
            driver: 1000,
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(legacy_item_driver_return_code(Some(1000), &outcome), 1);
}

#[test]
fn libload_area_guards_block_outside_legacy_area() {
    let mut character = character(1);
    let mut bridge = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BONEBRIDGE);
    let request = ItemDriverRequest::Driver {
        driver: IDR_BONEBRIDGE,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(IDR_BONEBRIDGE, 89);
    assert_eq!(IDR_BONEHINT, 94);
    assert_eq!(IDR_NOMADDICE, 95);
    assert_eq!(IDR_STAFFER2, 122);
    assert_eq!(IDR_CALIGAR, 144);
    assert_eq!(IDR_CALIGARFLAME, 145);
    assert_eq!(IDR_ARKHATA, 146);
    assert_eq!(
        execute_item_driver(&mut character, &mut bridge, request, 1, false),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_BONEBRIDGE,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            required_area: 18,
        }
    );
}

#[test]
fn libload_area_guards_fall_through_inside_legacy_area() {
    let mut character = character(1);
    let mut bridge = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BONEBRIDGE);
    let request = ItemDriverRequest::Driver {
        driver: IDR_BONEBRIDGE,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut bridge, request, 18, false),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn legacy_item_driver_return_code_matches_c_driver_contract() {
    assert_eq!(
        legacy_item_driver_return_code(None, &ItemDriverOutcome::Noop),
        0
    );
    assert_eq!(
        legacy_item_driver_return_code(
            Some(IDR_POTION),
            &ItemDriverOutcome::Unsupported {
                driver: IDR_POTION,
                item_id: ItemId(7),
                character_id: CharacterId(1),
            },
        ),
        0
    );
    assert_eq!(
        legacy_item_driver_return_code(Some(IDR_DOOR), &ItemDriverOutcome::Noop),
        2
    );
    assert_eq!(
        legacy_item_driver_return_code(Some(IDR_CLANVAULT), &ItemDriverOutcome::Noop),
        1
    );
    for driver in [
        IDR_STR_MINE,
        IDR_STR_STORAGE,
        IDR_STR_SPAWNER,
        IDR_STR_DEPOT,
        IDR_STR_TICKER,
        IDR_NOSNOW,
    ] {
        assert_eq!(
            legacy_item_driver_return_code(Some(driver), &ItemDriverOutcome::Noop),
            1
        );
    }
    assert_eq!(
        legacy_item_driver_return_code(
            Some(IDR_DOOR),
            &ItemDriverOutcome::DoorToggle {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            },
        ),
        1
    );
    assert_eq!(
        legacy_item_driver_return_code(
            Some(IDR_DOUBLE_DOOR),
            &ItemDriverOutcome::DoubleDoorToggle {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            },
        ),
        1
    );
    assert_eq!(
        legacy_item_driver_return_code(
            Some(IDR_CHEST),
            &ItemDriverOutcome::ChestTreasure {
                item_id: ItemId(9),
                character_id: CharacterId(1),
                treasure_index: 3,
            },
        ),
        1
    );
    assert_eq!(
        legacy_item_driver_return_code(
            Some(IDR_BOOK),
            &ItemDriverOutcome::BookText {
                item_id: ItemId(10),
                character_id: CharacterId(1),
                kind: 8,
                demon_value: 0,
            },
        ),
        1
    );
}

#[test]
fn clan_vault_dispatch_is_legacy_handled_noop_in_area30() {
    let mut character = character(1);
    let mut vault = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_CLANVAULT);
    let request = ItemDriverRequest::Driver {
        driver: IDR_CLANVAULT,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    let outcome = execute_item_driver(&mut character, &mut vault, request, 30, false);

    assert_eq!(outcome, ItemDriverOutcome::Noop);
    assert_eq!(
        legacy_item_driver_return_code(Some(IDR_CLANVAULT), &outcome),
        1
    );
}

#[test]
fn strategy_item_dispatch_boundary_is_legacy_handled_noop() {
    for driver in [
        IDR_STR_MINE,
        IDR_STR_STORAGE,
        IDR_STR_SPAWNER,
        IDR_STR_DEPOT,
        IDR_STR_TICKER,
        IDR_NOSNOW,
    ] {
        let mut character = character(1);
        let mut strategy_item = item(7, ItemFlags::USED | ItemFlags::USE, 0, driver);
        let request = ItemDriverRequest::Driver {
            driver,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        let outcome = execute_item_driver(&mut character, &mut strategy_item, request, 23, false);

        assert_eq!(outcome, ItemDriverOutcome::Noop);
        assert_eq!(legacy_item_driver_return_code(Some(driver), &outcome), 1);
    }
}

#[test]
fn enhance_material_driver_dispatches_for_carried_items() {
    let mut character = character(1);
    let mut stack = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_ENHANCE);
    stack.carried_by = Some(character.id);
    stack.driver_data = vec![1, 100, 0, 0, 0];
    character.inventory[30] = Some(stack.id);

    assert_eq!(
        execute_item_driver(
            &mut character,
            &mut stack,
            ItemDriverRequest::Driver {
                driver: IDR_ENHANCE,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            12,
            false,
        ),
        ItemDriverOutcome::NomadStack {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn demon_chip_driver_dispatches_to_stack_outcome() {
    let mut character = character(1);
    let mut stack = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DEMONCHIP);
    stack.carried_by = Some(character.id);
    character.inventory[30] = Some(stack.id);

    assert_eq!(
        execute_item_driver(
            &mut character,
            &mut stack,
            ItemDriverRequest::Driver {
                driver: IDR_DEMONCHIP,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        ),
        ItemDriverOutcome::NomadStack {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}
