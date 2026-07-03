use super::*;

#[test]
fn execute_food_driver_consumes_simple_food_and_ports_special_food() {
    let mut character = character(1);
    character.cursor_item = Some(ItemId(7));
    let mut food = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FOOD);
    food.carried_by = Some(CharacterId(1));
    food.driver_data = vec![1];

    let outcome = execute_item_driver(
        &mut character,
        &mut food,
        ItemDriverRequest::Driver {
            driver: IDR_FOOD,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::FoodEaten {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            kind: 1,
        }
    );
    assert_eq!(character.cursor_item, None);
    assert!(!food.flags.contains(ItemFlags::USED));

    let mut lollipop = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FOOD);
    lollipop.carried_by = Some(CharacterId(1));
    lollipop.driver_data = vec![2, 0];
    lollipop.sprite = 100;
    lollipop.description = "A sweet lollipop.".to_string();
    character.level = 10;
    character.exp = 7;
    assert_eq!(
        execute_item_driver(
            &mut character,
            &mut lollipop,
            ItemDriverRequest::Driver {
                driver: IDR_FOOD,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        ),
        ItemDriverOutcome::LollipopLicked {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            exp_added: 6,
            lick_count: 1,
        }
    );
    assert_eq!(lollipop.sprite, 101);
    assert_eq!(lollipop.driver_data[1], 1);
    assert_eq!(
        lollipop.description,
        "A sweet lollipop. Well, it's already used."
    );
    assert_eq!(character.exp, 13);
    assert!(lollipop.flags.contains(ItemFlags::USED));

    lollipop.driver_data[1] = 7;
    assert_eq!(
        execute_item_driver(
            &mut character,
            &mut lollipop,
            ItemDriverRequest::Driver {
                driver: IDR_FOOD,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        ),
        ItemDriverOutcome::LollipopLicked {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            exp_added: 6,
            lick_count: 8,
        }
    );
    assert_eq!(lollipop.driver_data[1], 8);
    assert_eq!(lollipop.description, "A lollipop stick.");

    assert_eq!(
        execute_item_driver(
            &mut character,
            &mut lollipop,
            ItemDriverRequest::Driver {
                driver: IDR_FOOD,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        ),
        ItemDriverOutcome::LollipopMemories {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    let mut xmaspop = item(9, ItemFlags::USED | ItemFlags::USE, 0, IDR_FOOD);
    xmaspop.carried_by = Some(CharacterId(1));
    xmaspop.driver_data = vec![3];
    assert_eq!(
        execute_item_driver(
            &mut character,
            &mut xmaspop,
            ItemDriverRequest::Driver {
                driver: IDR_FOOD,
                item_id: ItemId(9),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        ),
        ItemDriverOutcome::ChristmasPopInspected {
            item_id: ItemId(9),
            character_id: CharacterId(1),
        }
    );
    assert!(xmaspop.flags.contains(ItemFlags::USED));
}
