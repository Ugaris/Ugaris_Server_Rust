use super::*;

#[test]
fn orbspawn_driver_returns_typed_spawn_for_paid_eligible_character() {
    let mut character = character(1);
    character.flags.insert(CharacterFlags::PAID);
    character.level = 10;
    let mut spawner = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_ORBSPAWN);
    spawner.min_level = 5;
    let request = ItemDriverRequest::Driver {
        driver: IDR_ORBSPAWN,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut spawner, request, 1, false),
        ItemDriverOutcome::OrbSpawn {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            anti: false,
            special: false,
        }
    );
}

#[test]
fn anti_orbspawn_driver_blocks_unpaid_and_marks_special() {
    let mut character = character(1);
    let mut spawner = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_ANTIORBSPAWN);
    spawner.driver_data = vec![1];
    let request = ItemDriverRequest::Driver {
        driver: IDR_ANTIORBSPAWN,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut spawner, request, 1, false),
        ItemDriverOutcome::BlockedByRequirements {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    character.flags.insert(CharacterFlags::PAID);
    assert_eq!(
        execute_item_driver(&mut character, &mut spawner, request, 1, false),
        ItemDriverOutcome::OrbSpawn {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            anti: true,
            special: true,
        }
    );
}
