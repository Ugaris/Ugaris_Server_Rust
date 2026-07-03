use super::*;

#[test]
fn legacy_lucky_die_uses_best_of_luck_plus_one_rolls() {
    assert_eq!(legacy_lucky_die_from_rolls(6, 0, [2, 6, 6]), 2);
    assert_eq!(legacy_lucky_die_from_rolls(6, 2, [2, 5, 3, 6]), 5);
    assert_eq!(legacy_lucky_die_from_rolls(6, 1, [0, 9]), 6);
    assert_eq!(
        legacy_nomad_dice_total(
            1,
            [
                [1, 6, 1, 1, 1, 1, 1, 1],
                [2, 4, 1, 1, 1, 1, 1, 1],
                [3, 1, 1, 1, 1, 1, 1, 1]
            ]
        ),
        13
    );
}

#[test]
fn nomad_stack_driver_dispatches_for_carried_items() {
    let mut character = character(1);
    let mut stack = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_NOMADSTACK);
    stack.carried_by = Some(character.id);
    character.inventory[30] = Some(stack.id);

    assert_eq!(
        execute_item_driver(
            &mut character,
            &mut stack,
            ItemDriverRequest::Driver {
                driver: IDR_NOMADSTACK,
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
