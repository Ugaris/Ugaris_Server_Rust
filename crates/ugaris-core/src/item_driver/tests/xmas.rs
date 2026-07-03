use super::*;

#[test]
fn xmasmaker_driver_only_dispatches_for_staff_or_god() {
    let mut character = character(1);
    let mut maker = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_XMASMAKER);

    let request = ItemDriverRequest::Driver {
        driver: IDR_XMASMAKER,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };
    assert_eq!(
        execute_item_driver(&mut character, &mut maker, request, 1, false),
        ItemDriverOutcome::Noop
    );

    character.flags.insert(CharacterFlags::STAFF);
    assert_eq!(
        execute_item_driver(&mut character, &mut maker, request, 1, false),
        ItemDriverOutcome::XmasMaker {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn xmastree_driver_dispatches_for_character_use() {
    let mut character = character(1);
    let mut tree = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_XMASTREE);

    assert_eq!(
        execute_item_driver(
            &mut character,
            &mut tree,
            ItemDriverRequest::Driver {
                driver: IDR_XMASTREE,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        ),
        ItemDriverOutcome::XmasTree {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
}
