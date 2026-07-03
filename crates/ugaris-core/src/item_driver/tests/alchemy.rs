use super::*;

#[test]
fn flask_driver_ports_ingredient_gates_and_add_outcome() {
    let mut actor = character(1);
    actor.cursor_item = Some(ItemId(9));
    let mut flask = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLASK);
    flask.carried_by = Some(CharacterId(1));
    flask.driver_data = vec![2, 1, 0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_FLASK,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(IID_ALCHEMY_INGREDIENT, (1 << 24) | 0x43);
    assert_eq!(
        execute_item_driver_with_context(
            &mut actor,
            &mut flask,
            request,
            1,
            false,
            &ItemDriverContext::default(),
        ),
        ItemDriverOutcome::FlaskWrongCursor {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );

    let context = ItemDriverContext {
        cursor_template_id: Some(IID_ALCHEMY_INGREDIENT),
        cursor_drdata0: Some(7),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut flask, request, 1, false, &context),
        ItemDriverOutcome::FlaskIngredientAdded {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(9),
            ingredient_kind: 7,
        }
    );

    flask.driver_data[1] = 6;
    assert_eq!(
        execute_item_driver_with_context(&mut actor, &mut flask, request, 1, false, &context),
        ItemDriverOutcome::FlaskFull {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn flask_driver_ports_finished_potion_use_boundary() {
    let mut actor = character(1);
    actor.level = 10;
    actor.flags.insert(CharacterFlags::WARRIOR);
    let mut flask = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLASK);
    flask.carried_by = Some(CharacterId(1));
    flask.min_level = 10;
    flask.driver_data = vec![2, 3, 1, 20];
    flask.modifier_index = [CharacterValue::Strength as i16, 0, 0, 0, 0];
    flask.modifier_value = [7, 0, 0, 0, 0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_FLASK,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut flask, request, 1, false),
        ItemDriverOutcome::AlchemyFlaskPotion {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            duration_minutes: 20,
            modifier_index: [CharacterValue::Strength as i16, 0, 0, 0, 0],
            modifier_value: [7, 0, 0, 0, 0],
        }
    );

    actor.level = 9;
    assert_eq!(
        execute_item_driver(&mut actor, &mut flask, request, 1, false),
        ItemDriverOutcome::BlockedByRequirements {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn flask_driver_ports_successful_shake_recipe_mix() {
    let mut actor = character(1);
    let mut flask = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLASK);
    flask.carried_by = Some(CharacterId(1));
    flask.driver_data = vec![
        2, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1,
    ];
    flask.driver_data[12] = 1;
    flask.driver_data[13] = 1;
    flask.driver_data[14] = 1;
    let request = ItemDriverRequest::Driver {
        driver: IDR_FLASK,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut flask, request, 1, false),
        ItemDriverOutcome::FlaskMixed {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            ingredient_counts: {
                let mut counts = [0; 29];
                counts[1] = 1;
                counts[2] = 1;
                counts[3] = 1;
                counts[7] = 1;
                counts[8] = 1;
                counts[17] = 1;
                counts
            },
        }
    );
    assert_eq!(flask.driver_data[2], 1);
    assert_eq!(flask.driver_data[3], 10);
    assert_eq!(flask.modifier_index[0], CharacterValue::Attack as i16);
    assert_eq!(flask.modifier_value[0], 3);
    assert_eq!(flask.value, 3 * 7 * 13 + 50);
    assert_eq!(flask.needs_class, 0);
    assert_eq!(flask.name, "Magical Potion");
    assert_eq!(flask.sprite, 50214);
    assert_eq!(flask.description, "A flask containing a magical liquid.");
}

#[test]
fn flask_driver_ports_c_empty_modifier_slots_for_smaller_recipes() {
    let mut actor = character(1);
    let request = ItemDriverRequest::Driver {
        driver: IDR_FLASK,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    let mut double_recipe = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLASK);
    double_recipe.carried_by = Some(CharacterId(1));
    double_recipe.driver_data.resize(31, 0);
    double_recipe.driver_data[0] = 2;
    double_recipe.driver_data[1] = 4;
    double_recipe.driver_data[11] = 1;
    double_recipe.driver_data[12] = 1;
    double_recipe.driver_data[13] = 1;
    double_recipe.driver_data[14] = 1;
    double_recipe.driver_data[18] = 1;
    double_recipe.driver_data[28] = 1;

    assert!(matches!(
        execute_item_driver(&mut actor, &mut double_recipe, request, 1, false),
        ItemDriverOutcome::FlaskMixed { .. }
    ));
    assert_eq!(
        double_recipe.modifier_index[0..3],
        [
            CharacterValue::Attack as i16,
            CharacterValue::Parry as i16,
            -1,
        ]
    );
    assert_eq!(double_recipe.modifier_value[0..3], [1, 1, 0]);

    let mut single_recipe = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLASK);
    single_recipe.carried_by = Some(CharacterId(1));
    single_recipe.driver_data.resize(29, 0);
    single_recipe.driver_data[0] = 1;
    single_recipe.driver_data[1] = 3;
    single_recipe.driver_data[14] = 2;
    single_recipe.driver_data[17] = 1;
    single_recipe.driver_data[18] = 1;
    single_recipe.driver_data[28] = 1;

    assert!(matches!(
        execute_item_driver(&mut actor, &mut single_recipe, request, 1, false),
        ItemDriverOutcome::FlaskMixed { .. }
    ));
    assert_eq!(
        single_recipe.modifier_index[0..3],
        [CharacterValue::Pulse as i16, -1, -1]
    );
    assert_eq!(single_recipe.modifier_value[0..3], [2, 0, 0]);
}

#[test]
fn flask_driver_ports_failed_shake_reset_to_empty_bottle() {
    let mut actor = character(1);
    let mut flask = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLASK);
    flask.carried_by = Some(CharacterId(1));
    flask.name = "Unfinished Potion".to_string();
    flask.description = "A flask containing some strange liquid.".to_string();
    flask.sprite = 50209;
    flask.value = 123;
    flask.needs_class = 8;
    flask.driver_data.resize(35, 0);
    flask.driver_data[0] = 2;
    flask.driver_data[1] = 1;
    flask.driver_data[11] = 1;
    flask.modifier_index[0] = CharacterValue::Wisdom as i16;
    flask.modifier_value[0] = 1;
    let request = ItemDriverRequest::Driver {
        driver: IDR_FLASK,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut actor, &mut flask, request, 1, false),
        ItemDriverOutcome::FlaskRuined {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            ingredient_counts: {
                let mut counts = [0; 29];
                counts[0] = 1;
                counts
            },
        }
    );
    assert_eq!(flask.name, "Empty Potion");
    assert_eq!(flask.sprite, 10294);
    assert_eq!(flask.description, "A flask made of glass.");
    assert_eq!(flask.driver_data, vec![2]);
    assert_eq!(flask.modifier_index, [0; MAX_MODIFIERS]);
    assert_eq!(flask.modifier_value, [0; MAX_MODIFIERS]);
    assert_eq!(flask.value, 10);
    assert_eq!(flask.needs_class, 0);
}

#[test]
fn flask_driver_ports_fallback_attribute_mix_and_stone_class() {
    let mut actor = character(1);
    actor.professions[P_ALCHEMIST] = 20;
    let mut flask = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLASK);
    flask.carried_by = Some(CharacterId(1));
    flask.driver_data.resize(35, 0);
    flask.driver_data[0] = 2;
    flask.driver_data[1] = 3;
    flask.driver_data[11] = 2;
    flask.driver_data[15] = 1;
    flask.driver_data[18] = 1;
    flask.driver_data[28] = 1;
    flask.driver_data[31] = 1;
    let request = ItemDriverRequest::Driver {
        driver: IDR_FLASK,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    execute_item_driver(&mut actor, &mut flask, request, 1, false);

    assert_eq!(flask.modifier_index[0], CharacterValue::Wisdom as i16);
    assert_eq!(flask.modifier_value[0], 2);
    assert_eq!(flask.modifier_index[1], CharacterValue::Hp as i16);
    assert_eq!(flask.modifier_value[1], 2);
    assert_eq!(flask.value, 15 * 13 + 50);
    assert_eq!(flask.needs_class, 8);
}

#[test]
fn flask_power_uses_legacy_time_and_alchemist_thresholds() {
    let mut actor = character(1);
    actor.professions[P_ALCHEMIST] = 50;
    let mut flask = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLASK);
    flask.carried_by = Some(CharacterId(1));
    flask.driver_data.resize(29, 0);
    flask.driver_data[0] = 2;
    flask.driver_data[1] = 3;
    flask.driver_data[12] = 1;
    flask.driver_data[13] = 1;
    flask.driver_data[14] = 1;
    flask.driver_data[25] = 1;
    flask.driver_data[26] = 1;
    flask.driver_data[28] = 1;
    let context = ItemDriverContext {
        fullmoon: true,
        ..ItemDriverContext::default()
    };
    let request = ItemDriverRequest::Driver {
        driver: IDR_FLASK,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    execute_item_driver_with_context(&mut actor, &mut flask, request, 1, false, &context);

    assert_eq!(flask.modifier_value[0], 44);
    assert_eq!(flask.value, 3 * 88 * 13 + 50);
}
