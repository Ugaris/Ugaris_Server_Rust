use super::*;

#[test]
fn saltmine_door_blocks_non_worker_users_with_legacy_outcome() {
    let mut actor = character(1);
    let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SALTMINE_ITEM);
    set_drdata(&mut door, 0, 3);
    let request = ItemDriverRequest::Driver {
        driver: IDR_SALTMINE_ITEM,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    let outcome = execute_item_driver(&mut actor, &mut door, request, 1, false);

    assert_eq!(
        outcome,
        ItemDriverOutcome::SaltmineDoorBlocked {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(
        legacy_item_driver_return_code(Some(IDR_SALTMINE_ITEM), &outcome),
        1
    );
}

#[test]
fn saltmine_ladder_returns_player_runtime_outcome() {
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    let mut ladder = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SALTMINE_ITEM);
    set_drdata(&mut ladder, 0, 1);
    set_drdata(&mut ladder, 1, 4);
    let request = ItemDriverRequest::Driver {
        driver: IDR_SALTMINE_ITEM,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut ladder, request, 1, false),
        ItemDriverOutcome::SaltmineLadderUse {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            ladder_index: 4,
        }
    );
}

#[test]
fn saltmine_saltbag_returns_player_runtime_outcome() {
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    let mut saltbag = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SALTMINE_ITEM);
    set_drdata(&mut saltbag, 0, 2);
    let request = ItemDriverRequest::Driver {
        driver: IDR_SALTMINE_ITEM,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut saltbag, request, 1, false),
        ItemDriverOutcome::SaltmineSaltbagUse {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}
