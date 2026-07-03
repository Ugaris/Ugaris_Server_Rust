use super::*;

#[test]
fn brannington_underwater_berry_requires_area28_and_player() {
    let mut actor = character(1);
    let mut berry = item(
        8,
        ItemFlags::USED | ItemFlags::USE,
        0,
        IDR_BRANNINGTONFOREST,
    );
    berry.driver_data = vec![1];
    let request = ItemDriverRequest::Driver {
        driver: IDR_BRANNINGTONFOREST,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(IDR_BRANNINGTONFOREST, 123);
    assert_eq!(
        execute_item_driver(&mut actor, &mut berry, request, 1, false),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_BRANNINGTONFOREST,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            required_area: 28,
        }
    );

    assert_eq!(
        execute_item_driver(&mut actor, &mut berry, request, 28, false),
        ItemDriverOutcome::Noop
    );

    actor.flags.insert(CharacterFlags::PLAYER);
    assert_eq!(
        execute_item_driver(&mut actor, &mut berry, request, 28, false),
        ItemDriverOutcome::BranningtonUnderwaterBerry {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            duration_ticks: TICKS_PER_SECOND * 30,
            installed: false,
        }
    );
}
