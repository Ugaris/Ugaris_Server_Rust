use super::*;

#[test]
fn execute_stat_scroll_raises_value_grants_exp_and_consumes_item() {
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(7));
    character.values[0][CharacterValue::Sword as usize] = 10;
    character.values[1][CharacterValue::Sword as usize] = 10;
    let mut scroll = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_STATSCROLL);
    scroll.carried_by = Some(CharacterId(1));
    scroll.driver_data = vec![CharacterValue::Sword as u8, 2];

    let outcome = execute_item_driver(
        &mut character,
        &mut scroll,
        ItemDriverRequest::Driver {
            driver: IDR_STATSCROLL,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::StatScrollUsed {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            value: CharacterValue::Sword as u8,
            raised: 2,
            exp_cost: 746,
        }
    );
    assert_eq!(character.values[1][CharacterValue::Sword as usize], 12);
    assert_eq!(character.values[0][CharacterValue::Sword as usize], 12);
    assert_eq!(character.exp, 746);
    assert_eq!(character.exp_used, 746);
    assert_eq!(character.inventory[30], None);
    assert!(!scroll.flags.contains(ItemFlags::USED));
}

#[test]
fn execute_stat_scroll_blocks_unusable_cases_without_consuming() {
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(7));
    character.values[0][CharacterValue::Armor as usize] = 10;
    character.values[1][CharacterValue::Armor as usize] = 10;
    let mut scroll = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_STATSCROLL);
    scroll.carried_by = Some(CharacterId(1));
    scroll.driver_data = vec![CharacterValue::Armor as u8, 1];
    let request = ItemDriverRequest::Driver {
        driver: IDR_STATSCROLL,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut scroll, request, 1, false),
        ItemDriverOutcome::BlockedByRequirements {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
    assert_eq!(character.inventory[30], Some(ItemId(7)));
    assert!(scroll.flags.contains(ItemFlags::USED));

    scroll.driver_data = vec![CharacterValue::Sword as u8, 1];
    character.values[1][CharacterValue::Sword as usize] = 10;
    character.flags.insert(CharacterFlags::NOEXP);
    assert_eq!(
        execute_item_driver(&mut character, &mut scroll, request, 1, false),
        ItemDriverOutcome::BlockedByRequirements {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );

    character.flags.remove(CharacterFlags::NOEXP);
    scroll.carried_by = None;
    assert_eq!(
        execute_item_driver(&mut character, &mut scroll, request, 1, false),
        ItemDriverOutcome::Noop
    );
}
