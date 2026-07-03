use super::*;

#[test]
fn rat_chest_dispatch_returns_typed_runtime_outcome() {
    let mut actor = character(1);
    let mut chest = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_RATCHEST);
    let request = ItemDriverRequest::Driver {
        driver: IDR_RATCHEST,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut chest, request, 1, false),
        ItemDriverOutcome::RatChest {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn rat_chest_requires_empty_cursor() {
    let mut actor = character(1);
    actor.cursor_item = Some(ItemId(99));
    let mut chest = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_RATCHEST);
    let request = ItemDriverRequest::Driver {
        driver: IDR_RATCHEST,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut chest, request, 1, false),
        ItemDriverOutcome::BlockedByRequirements {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}
