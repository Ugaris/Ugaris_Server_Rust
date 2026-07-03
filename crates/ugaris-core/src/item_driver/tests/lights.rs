use super::*;

#[test]
fn toylight_toggles_light_state_on_character_use() {
    let mut character = character(1);
    let mut light = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_TOYLIGHT);
    light.driver_data = vec![0, 12];
    let request = ItemDriverRequest::Driver {
        driver: IDR_TOYLIGHT,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut light, request, 1, false),
        ItemDriverOutcome::LightChanged {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            schedule_after_ticks: None,
        }
    );
    assert_eq!(light.driver_data[0], 1);
    assert_eq!(light.modifier_index[0], V_LIGHT);
    assert_eq!(light.modifier_value[0], 12);
    assert_eq!(light.sprite, 1);

    execute_item_driver(&mut character, &mut light, request, 1, false);
    assert_eq!(light.driver_data[0], 0);
    assert_eq!(light.modifier_value[0], 0);
    assert_eq!(light.sprite, 0);
}

#[test]
fn nightlight_timer_follows_daylight_threshold_and_reschedules() {
    let mut character = character(1);
    let mut light = item(7, ItemFlags::USED, 0, IDR_NIGHTLIGHT);
    light.driver_data = vec![0, 9];
    let request = ItemDriverRequest::Driver {
        driver: IDR_NIGHTLIGHT,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };
    let mut context = ItemDriverContext {
        timer_call: true,
        daylight: 79,
        ..ItemDriverContext::default()
    };

    assert_eq!(
        execute_item_driver_with_context(&mut character, &mut light, request, 1, false, &context),
        ItemDriverOutcome::LightChanged {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            schedule_after_ticks: Some(LIGHT_TIMER_TICKS),
        }
    );
    assert_eq!(light.driver_data[0], 1);
    assert_eq!(light.modifier_value[0], 9);
    assert_eq!(light.sprite, 1);

    context.daylight = 81;
    execute_item_driver_with_context(&mut character, &mut light, request, 1, false, &context);
    assert_eq!(light.driver_data[0], 0);
    assert_eq!(light.modifier_value[0], 0);
    assert_eq!(light.sprite, 0);
}

#[test]
fn onofflight_timer_registers_and_use_toggles_light_state() {
    let mut timer_character = character(0);
    let mut light = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_ONOFFLIGHT);
    light.driver_data = vec![1, 15];
    light.modifier_index[0] = V_LIGHT;
    light.modifier_value[0] = 15;
    light.sprite = 101;
    let request = ItemDriverRequest::Driver {
        driver: IDR_ONOFFLIGHT,
        item_id: ItemId(7),
        character_id: CharacterId(0),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver_with_context(
            &mut timer_character,
            &mut light,
            request,
            3,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        ),
        ItemDriverOutcome::Noop
    );
    assert_eq!(light.driver_data[6], 1);
    assert_eq!(light.driver_data[0], 1);
    assert_eq!(light.modifier_value[0], 15);
    assert_eq!(light.sprite, 101);

    let mut character = character(1);
    let request = ItemDriverRequest::Driver {
        driver: IDR_ONOFFLIGHT,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };
    assert_eq!(
        execute_item_driver(&mut character, &mut light, request, 3, false),
        ItemDriverOutcome::OnOffLightChanged {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            now_on: false,
            remaining_off: None,
            gates_opened: false,
        }
    );
    assert_eq!(light.driver_data[0], 0);
    assert_eq!(light.modifier_value[0], 0);
    assert_eq!(light.sprite, 100);

    execute_item_driver(&mut character, &mut light, request, 3, false);
    assert_eq!(light.driver_data[0], 1);
    assert_eq!(light.modifier_index[0], V_LIGHT);
    assert_eq!(light.modifier_value[0], 15);
    assert_eq!(light.sprite, 101);
}

#[test]
fn torch_user_use_lights_and_extinguishes_carried_torch() {
    let mut character = character(1);
    let mut torch = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_TORCH);
    torch.carried_by = Some(CharacterId(1));
    torch.driver_data = vec![0, 0, 10, 20];
    let request = ItemDriverRequest::Driver {
        driver: IDR_TORCH,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    execute_item_driver(&mut character, &mut torch, request, 1, false);
    assert_eq!(torch.driver_data[0], 1);
    assert_eq!(torch.modifier_index[0], V_LIGHT);
    assert_eq!(torch.modifier_value[0], 20.min(20 * 10 / 1 / 2));
    assert_eq!(torch.sprite, -1);
    assert!(torch.flags.contains(ItemFlags::NODECAY));
    assert!(character.flags.contains(CharacterFlags::ITEMS));

    execute_item_driver(&mut character, &mut torch, request, 1, false);
    assert_eq!(torch.driver_data[0], 0);
    assert_eq!(torch.modifier_value[0], 0);
    assert_eq!(torch.sprite, 0);
    assert!(!torch.flags.contains(ItemFlags::NODECAY));
}

#[test]
fn torch_user_use_extracts_non_light_modifier_before_toggling() {
    let mut character = character(1);
    let mut torch = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_TORCH);
    torch.carried_by = Some(CharacterId(1));
    torch.driver_data = vec![0, 0, 10, 20];
    torch.modifier_index[1] = CharacterValue::Speed as i16;
    torch.modifier_value[1] = 2;
    let request = ItemDriverRequest::Driver {
        driver: IDR_TORCH,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };

    assert_eq!(
        execute_item_driver(&mut character, &mut torch, request, 1, false),
        ItemDriverOutcome::TorchExtractOrb {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            modifier_slot: 1,
            modifier: CharacterValue::Speed as i16,
        }
    );
    assert_eq!(torch.driver_data[0], 0);
    assert_eq!(torch.modifier_value[1], 2);
}

#[test]
fn torch_timer_burns_down_marks_special_and_expires() {
    let mut character = character(1);
    let mut torch = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_TORCH);
    torch.carried_by = Some(CharacterId(1));
    torch.driver_data = vec![1, 1, 2, 20];
    torch.modifier_index[1] = CharacterValue::Speed as i16;
    torch.modifier_value[1] = 1;
    let request = ItemDriverRequest::Driver {
        driver: IDR_TORCH,
        item_id: ItemId(7),
        character_id: CharacterId(1),
        spec: 0,
    };
    let context = ItemDriverContext {
        timer_call: true,
        ..ItemDriverContext::default()
    };

    assert_eq!(
        execute_item_driver_with_context(&mut character, &mut torch, request, 1, false, &context),
        ItemDriverOutcome::LightChanged {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            schedule_after_ticks: Some(LIGHT_TIMER_TICKS),
        }
    );
    assert_eq!(torch.min_level, 200);
    assert_eq!(torch.driver_data[1], 2);
    assert_eq!(torch.modifier_value[0], 20.min(20 * 2 / 3 / 2));

    assert_eq!(
        execute_item_driver_with_context(&mut character, &mut torch, request, 1, false, &context),
        ItemDriverOutcome::TorchExpired {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            item_name: outcome_item_name("Item"),
        }
    );
}
