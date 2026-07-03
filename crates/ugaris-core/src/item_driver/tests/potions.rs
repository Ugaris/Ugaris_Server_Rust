use super::*;

#[test]
fn execute_potion_driver_restores_resources_and_consumes_non_empty_potion() {
    let mut character = character(1);
    character.hp = 1_000;
    character.mana = 2_000;
    character.endurance = 3_000;
    character.values[0][CharacterValue::Hp as usize] = 10;
    character.values[0][CharacterValue::Mana as usize] = 10;
    character.values[0][CharacterValue::Endurance as usize] = 10;
    character.inventory[30] = Some(ItemId(7));
    let mut item = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_POTION);
    item.carried_by = Some(CharacterId(1));
    item.driver_data = vec![0, 20, 3, 4];

    let outcome = execute_item_driver(
        &mut character,
        &mut item,
        ItemDriverRequest::Driver {
            driver: IDR_POTION,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::PotionDrunk {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            hp_added: 9_000,
            mana_added: 3_000,
            endurance_added: 4_000,
        }
    );
    assert_eq!(
        (character.hp, character.mana, character.endurance),
        (10_000, 5_000, 7_000)
    );
    assert_eq!(character.inventory[30], None);
    assert!(!item.flags.contains(ItemFlags::USED));
}

#[test]
fn execute_potion_driver_defers_empty_bottle_template_creation() {
    let mut character = character(1);
    character.values[0][CharacterValue::Hp as usize] = 10;
    let mut item = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_POTION);
    item.carried_by = Some(CharacterId(1));
    item.driver_data = vec![2, 5, 0, 0];

    let outcome = execute_item_driver(
        &mut character,
        &mut item,
        ItemDriverRequest::Driver {
            driver: IDR_POTION,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::EmptyPotionTemplateNeeded {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            empty_kind: 2,
        }
    );
    assert!(item.flags.contains(ItemFlags::USED));
    assert_eq!(character.hp, 0);
}

#[test]
fn special_potion_type_7_resets_professions_and_lowers_profession_points() {
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(7));
    character.professions[0] = 2;
    character.professions[3] = 4;
    character.values[1][CharacterValue::Profession as usize] = 10;
    character.exp = 10_000;
    character.exp_used = 5_000;
    let mut potion = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPECIAL_POTION);
    potion.carried_by = Some(CharacterId(1));
    potion.driver_data = vec![7];

    let outcome = execute_item_driver(
        &mut character,
        &mut potion,
        ItemDriverRequest::Driver {
            driver: IDR_SPECIAL_POTION,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::SpecialPotionProfessionReset {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            used: true,
            professions_reset: 6,
            profession_points_lowered: 2,
            exp_refunded: 2_240,
        }
    );
    assert!(character.professions.iter().all(|&value| value == 0));
    assert_eq!(character.values[1][CharacterValue::Profession as usize], 8);
    assert_eq!((character.exp, character.exp_used), (7_760, 2_760));
    assert_eq!(character.inventory[30], None);
    assert!(!potion.flags.contains(ItemFlags::USED));
    assert!(character
        .flags
        .contains(CharacterFlags::PROF | CharacterFlags::UPDATE));
}

#[test]
fn special_potion_type_7_blocks_without_professions() {
    let mut character = character(1);
    let mut potion = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPECIAL_POTION);
    potion.carried_by = Some(CharacterId(1));
    potion.driver_data = vec![7];

    let outcome = execute_item_driver(
        &mut character,
        &mut potion,
        ItemDriverRequest::Driver {
            driver: IDR_SPECIAL_POTION,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::SpecialPotionProfessionReset {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            used: false,
            professions_reset: 0,
            profession_points_lowered: 0,
            exp_refunded: 0,
        }
    );
    assert!(potion.flags.contains(ItemFlags::USED));
}

#[test]
fn execute_decaying_item_toggles_carried_modifiers_and_schedules_timer() {
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(7));
    let mut decaying = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DECAYITEM);
    decaying.carried_by = Some(CharacterId(1));
    decaying.sprite = 100;
    decaying.modifier_value = [1, 0, 2, 0, 3];
    decaying.driver_data = vec![0, 4, 9, 0, 0, 2, 0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_DECAYITEM,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut decaying, request, 1, false),
        ItemDriverOutcome::DecayItemToggled {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            active: true,
            schedule_after_ticks: Some(TICKS_PER_SECOND * 2),
        }
    );
    assert_eq!(decaying.driver_data[0], 1);
    assert_eq!(decaying.sprite, 101);
    assert_eq!(decaying.modifier_value, [9, 0, 9, 0, 9]);
    assert!(character.flags.contains(CharacterFlags::ITEMS));

    assert_eq!(
        execute_item_driver(&mut character, &mut decaying, request, 1, false),
        ItemDriverOutcome::DecayItemToggled {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            active: false,
            schedule_after_ticks: None,
        }
    );
    assert_eq!(decaying.driver_data[0], 0);
    assert_eq!(decaying.sprite, 100);
    assert_eq!(decaying.modifier_value, [4, 0, 4, 0, 4]);
}

#[test]
fn execute_decaying_item_timer_ages_active_item_until_expiry() {
    let mut timer_character = character(0);
    let mut decaying = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DECAYITEM);
    decaying.name = "Vanishing Charm".into();
    decaying.carried_by = Some(CharacterId(1));
    decaying.driver_data = vec![1, 4, 9, 1, 0, 2, 0];
    let request = ItemDriverRequest::Driver {
        driver: IDR_DECAYITEM,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };
    let context = ItemDriverContext {
        timer_call: true,
        ..ItemDriverContext::default()
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut decaying,
            request,
            1,
            false,
            &context,
        ),
        ItemDriverOutcome::DecayItemToggled {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            active: true,
            schedule_after_ticks: Some(TICKS_PER_SECOND * 2),
        }
    );
    assert_eq!(decaying.driver_data[3], 2);
    assert_eq!(decaying.driver_data[4], 0);

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut decaying,
            request,
            1,
            false,
            &context,
        ),
        ItemDriverOutcome::DecayItemExpired {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            item_name: outcome_item_name("Vanishing Charm"),
        }
    );
    assert_eq!(decaying.driver_data[3], 3);
    assert_eq!(decaying.driver_data[4], 0);

    decaying.driver_data[0] = 0;
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut decaying,
            request,
            1,
            false,
            &context,
        ),
        ItemDriverOutcome::Noop
    );
}

#[test]
fn special_potion_fun_drinks_mutate_resources_and_consume_item() {
    let mut character = character(3);
    character.level = 10;
    character.hp = 15 * POWERSCALE;
    character.mana = 12 * POWERSCALE;
    character.endurance = 11 * POWERSCALE;
    character.values[0][CharacterValue::Hp as usize] = 20;
    character.inventory[30] = Some(ItemId(7));
    let mut potion = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPECIAL_POTION);
    potion.carried_by = Some(character.id);
    potion.driver_data = vec![8];

    let outcome = execute_item_driver_with_context(
        &mut character,
        &mut potion,
        ItemDriverRequest::Driver {
            driver: IDR_SPECIAL_POTION,
            item_id: ItemId(7),
            character_id: CharacterId(3),
            spec: 0,
        },
        1,
        false,
        &ItemDriverContext {
            current_tick: 12_345,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(character.hp, 5 * POWERSCALE);
    assert_eq!(character.mana, 2 * POWERSCALE);
    assert_eq!(character.endurance, POWERSCALE);
    assert_eq!(character.regen_ticker, 12_345);
    assert_eq!(character.inventory[30], None);
    assert!(!potion.flags.contains(ItemFlags::USED));
    assert_eq!(
        outcome,
        ItemDriverOutcome::SpecialPotionDrunk {
            item_id: ItemId(7),
            character_id: CharacterId(3),
            kind: 8,
            hp_delta: -10 * POWERSCALE,
            mana_delta: -10 * POWERSCALE,
            endurance_delta: -10 * POWERSCALE,
        }
    );
}

#[test]
fn special_potion_healing_caps_at_max_hp_and_area_blocks() {
    let mut character = character(3);
    character.level = 10;
    character.hp = 18 * POWERSCALE;
    character.values[0][CharacterValue::Hp as usize] = 20;
    character.inventory[30] = Some(ItemId(7));
    let mut potion = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPECIAL_POTION);
    potion.carried_by = Some(character.id);
    potion.driver_data = vec![14];
    let request = ItemDriverRequest::Driver {
        driver: IDR_SPECIAL_POTION,
        item_id: ItemId(7),
        character_id: CharacterId(3),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut potion, request, 1, false),
        ItemDriverOutcome::SpecialPotionDrunk {
            item_id: ItemId(7),
            character_id: CharacterId(3),
            kind: 14,
            hp_delta: 2 * POWERSCALE,
            mana_delta: 0,
            endurance_delta: 0,
        }
    );
    assert_eq!(character.hp, 20 * POWERSCALE);

    let mut blocked = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPECIAL_POTION);
    blocked.carried_by = Some(character.id);
    blocked.driver_data = vec![14];
    assert!(matches!(
        execute_item_driver(
            &mut character,
            &mut blocked,
            ItemDriverRequest::Driver {
                driver: IDR_SPECIAL_POTION,
                item_id: ItemId(8),
                character_id: CharacterId(3),
                spec: 0,
            },
            34,
            true,
        ),
        ItemDriverOutcome::BlockedByArea { .. }
    ));
}

#[test]
fn special_potion_security_increments_saves_and_consumes_item() {
    let mut character = character(3);
    character.level = 10;
    character.saves = 9;
    character.inventory[30] = Some(ItemId(7));
    let mut potion = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPECIAL_POTION);
    potion.carried_by = Some(character.id);
    potion.driver_data = vec![5];

    let outcome = execute_item_driver(
        &mut character,
        &mut potion,
        ItemDriverRequest::Driver {
            driver: IDR_SPECIAL_POTION,
            item_id: ItemId(7),
            character_id: CharacterId(3),
            spec: 0,
        },
        1,
        false,
    );

    assert_eq!(character.saves, 10);
    assert_eq!(character.inventory[30], None);
    assert!(!potion.flags.contains(ItemFlags::USED));
    assert_eq!(
        outcome,
        ItemDriverOutcome::SpecialPotionSecurity {
            item_id: ItemId(7),
            character_id: CharacterId(3),
            used: true,
        }
    );
}

#[test]
fn special_potion_security_blocks_hardcore_or_capped_saves() {
    let request = ItemDriverRequest::Driver {
        driver: IDR_SPECIAL_POTION,
        item_id: ItemId(7),
        character_id: CharacterId(3),
        spec: 0,
    };

    let mut capped = character(3);
    capped.level = 10;
    capped.saves = 10;
    capped.inventory[30] = Some(ItemId(7));
    let mut capped_potion = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPECIAL_POTION);
    capped_potion.carried_by = Some(capped.id);
    capped_potion.driver_data = vec![5];
    let capped_outcome = execute_item_driver(&mut capped, &mut capped_potion, request, 1, false);

    assert_eq!(capped.saves, 10);
    assert_eq!(capped.inventory[30], Some(ItemId(7)));
    assert_eq!(
        capped_outcome,
        ItemDriverOutcome::SpecialPotionSecurity {
            item_id: ItemId(7),
            character_id: CharacterId(3),
            used: false,
        }
    );

    let mut hardcore = character(3);
    hardcore.level = 10;
    hardcore.flags.insert(CharacterFlags::HARDCORE);
    hardcore.inventory[30] = Some(ItemId(7));
    let mut hardcore_potion = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPECIAL_POTION);
    hardcore_potion.carried_by = Some(hardcore.id);
    hardcore_potion.driver_data = vec![5];
    let hardcore_outcome =
        execute_item_driver(&mut hardcore, &mut hardcore_potion, request, 1, false);

    assert_eq!(hardcore.saves, 0);
    assert_eq!(hardcore.inventory[30], Some(ItemId(7)));
    assert_eq!(
        hardcore_outcome,
        ItemDriverOutcome::SpecialPotionSecurity {
            item_id: ItemId(7),
            character_id: CharacterId(3),
            used: false,
        }
    );
}

#[test]
fn special_potion_unknown_kind_reports_legacy_bug_without_consuming() {
    let mut character = character(3);
    character.level = 10;
    character.inventory[30] = Some(ItemId(7));
    let mut potion = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPECIAL_POTION);
    potion.carried_by = Some(character.id);
    potion.driver_data = vec![99];

    let outcome = execute_item_driver(
        &mut character,
        &mut potion,
        ItemDriverRequest::Driver {
            driver: IDR_SPECIAL_POTION,
            item_id: ItemId(7),
            character_id: CharacterId(3),
            spec: 0,
        },
        1,
        false,
    );

    assert_eq!(character.inventory[30], Some(ItemId(7)));
    assert!(potion.flags.contains(ItemFlags::USED));
    assert_eq!(
        outcome,
        ItemDriverOutcome::SpecialPotionBug {
            item_id: ItemId(7),
            character_id: CharacterId(3),
        }
    );
}

#[test]
fn beyond_potion_dispatch_copies_modifiers_and_duration() {
    let mut character = character(3);
    character.level = 12;
    character.flags.insert(CharacterFlags::WARRIOR);
    let mut potion = item(
        7,
        ItemFlags::USED | ItemFlags::USE | ItemFlags::BEYONDMAXMOD,
        0,
        IDR_BEYONDPOTION,
    );
    potion.carried_by = Some(character.id);
    potion.min_level = 10;
    potion.driver_data = vec![15];
    potion.modifier_index = [
        CharacterValue::Strength as i16,
        CharacterValue::Agility as i16,
        0,
        0,
        0,
    ];
    potion.modifier_value = [3, 4, 0, 0, 0];

    assert_eq!(
        execute_item_driver(
            &mut character,
            &mut potion,
            ItemDriverRequest::Driver {
                driver: IDR_BEYONDPOTION,
                item_id: ItemId(7),
                character_id: CharacterId(3),
                spec: 0,
            },
            1,
            false,
        ),
        ItemDriverOutcome::BeyondPotion {
            item_id: ItemId(7),
            character_id: CharacterId(3),
            duration_minutes: 15,
            modifier_index: [
                CharacterValue::Strength as i16,
                CharacterValue::Agility as i16,
                0,
                0,
                0,
            ],
            modifier_value: [3, 4, 0, 0, 0],
            beyond_max_mod: true,
        }
    );
}
