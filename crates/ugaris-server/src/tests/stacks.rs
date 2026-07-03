use super::*;

#[test]
fn enhance_xmas_item_uses_unique_legacy_skill_pool_and_caps_values() {
    let mut gift = test_item(ItemId(30), 1, ItemFlags::USED | ItemFlags::TAKE);
    gift.modifier_index = [CharacterValue::Armor as i16; ugaris_core::entity::MAX_MODIFIERS];
    gift.modifier_value = [99; ugaris_core::entity::MAX_MODIFIERS];
    let mut rng = XmasTreeRng::new(42);

    enhance_xmas_item(&mut gift, &mut rng);

    let mut seen = Vec::new();
    for (&index, &value) in gift.modifier_index.iter().zip(gift.modifier_value.iter()) {
        if value == 0 {
            assert_eq!(value, 0);
            continue;
        }
        assert!(value > 0 && value <= XMAS_MAX_SKILL_VALUE);
        assert!(XMAS_ENHANCE_SKILLS
            .iter()
            .any(|skill| *skill as i16 == index));
        assert!(!seen.contains(&index));
        seen.push(index);
    }
    assert!(seen.len() <= XMAS_MAX_SKILLS);
}

#[test]
fn nomad_stack_split_creates_cursor_stack_with_legacy_counts() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.inventory[30] = Some(ItemId(20));
    let mut world = World::default();
    world.add_character(character);
    let mut stack = test_item(ItemId(20), 13208, ItemFlags::USED | ItemFlags::USE);
    stack.name = "salt".to_string();
    stack.template_id = IID_AREA19_SALT;
    stack.driver = ugaris_core::item_driver::IDR_NOMADSTACK;
    stack.value = 1_000;
    stack.carried_by = Some(character_id);
    set_stack_count(&mut stack, 123, StackKind::Salt);
    world.add_item(stack);
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(r#"salt: name="salt" ID=0100008B flag=IF_TAKE driver=96 ;"#)
        .unwrap();

    assert_eq!(
        apply_nomad_stack(&mut world, &mut loader, ItemId(20), character_id),
        NomadStackApplyResult::Split {
            left: 73,
            right: 50,
            unit: "ounce",
        }
    );
    let character = world.characters.get(&character_id).unwrap();
    let cursor_id = character
        .cursor_item
        .expect("split stack should be on cursor");
    let carried = world.items.get(&ItemId(20)).unwrap();
    let cursor = world.items.get(&cursor_id).unwrap();
    assert_eq!(stack_count(carried), 73);
    assert_eq!(stack_count(cursor), 50);
    assert_eq!(carried.sprite, 13209);
    assert_eq!(cursor.description, "50 ounces of salt.");
}

#[test]
fn nomad_stack_merge_consumes_matching_cursor_stack() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.inventory[30] = Some(ItemId(20));
    character.cursor_item = Some(ItemId(21));
    let mut world = World::default();
    world.add_character(character);
    let mut carried = test_item(ItemId(20), 59655, ItemFlags::USED | ItemFlags::USE);
    carried.name = "skin".to_string();
    carried.template_id = IID_AREA19_WOLFSSKIN;
    carried.driver = ugaris_core::item_driver::IDR_NOMADSTACK;
    carried.value = 30;
    carried.carried_by = Some(character_id);
    set_stack_count(&mut carried, 3, StackKind::Skin1);
    world.add_item(carried);
    let mut cursor = test_item(ItemId(21), 59655, ItemFlags::USED | ItemFlags::USE);
    cursor.name = "skin".to_string();
    cursor.template_id = IID_AREA19_WOLFSSKIN;
    cursor.value = 20;
    cursor.carried_by = Some(character_id);
    set_stack_count(&mut cursor, 2, StackKind::Skin1);
    world.add_item(cursor);
    let mut loader = ZoneLoader::new();

    assert_eq!(
        apply_nomad_stack(&mut world, &mut loader, ItemId(20), character_id),
        NomadStackApplyResult::Merged {
            count: 5,
            unit: "skin",
        }
    );
    assert_eq!(
        world.characters.get(&character_id).unwrap().cursor_item,
        None
    );
    assert!(!world.items.contains_key(&ItemId(21)));
    let stack = world.items.get(&ItemId(20)).unwrap();
    assert_eq!(stack_count(stack), 5);
    assert_eq!(stack.value, 50);
    assert_eq!(stack.sprite, 59659);
}

#[test]
fn demon_chip_stack_split_uses_legacy_sprite_offsets() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.inventory[30] = Some(ItemId(20));
    let mut world = World::default();
    world.add_character(character);
    let mut stack = test_item(ItemId(20), 53019, ItemFlags::USED | ItemFlags::USE);
    stack.name = "Silver Chip".to_string();
    stack.template_id = IID_SILVERCHIP;
    stack.driver = ugaris_core::item_driver::IDR_DEMONCHIP;
    stack.value = 123_000;
    stack.carried_by = Some(character_id);
    set_stack_count(&mut stack, 123, StackKind::SilverChip);
    world.add_item(stack);
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"silverchip: name="Silver Chip" ID=010000AD flag=IF_TAKE driver=136 ;"#,
        )
        .unwrap();

    assert_eq!(
        apply_nomad_stack(&mut world, &mut loader, ItemId(20), character_id),
        NomadStackApplyResult::Split {
            left: 73,
            right: 50,
            unit: "chip",
        }
    );
    let cursor_id = world
        .characters
        .get(&character_id)
        .unwrap()
        .cursor_item
        .unwrap();
    let carried = world.items.get(&ItemId(20)).unwrap();
    let cursor = world.items.get(&cursor_id).unwrap();
    assert_eq!(stack_count(carried), 73);
    assert_eq!(stack_count(cursor), 50);
    assert_eq!(carried.sprite, 53024);
    assert_eq!(cursor.description, "50 Silver Chips.");
}

#[test]
fn demon_chip_stack_invalid_template_reports_legacy_chip_bug() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.inventory[30] = Some(ItemId(20));
    let mut world = World::default();
    world.add_character(character);
    let mut stack = test_item(ItemId(20), 53007, ItemFlags::USED | ItemFlags::USE);
    stack.template_id = 0xDEAD_BEEF;
    stack.driver = IDR_DEMONCHIP;
    stack.carried_by = Some(character_id);
    world.add_item(stack);
    let mut loader = ZoneLoader::new();

    assert_eq!(
        apply_nomad_stack(&mut world, &mut loader, ItemId(20), character_id),
        NomadStackApplyResult::Bug("Bug #1445y")
    );
}

#[test]
fn enhance_silver_stack_split_uses_legacy_drdata_count_offset() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.inventory[30] = Some(ItemId(20));
    let mut world = World::default();
    world.add_character(character);
    let mut stack = test_item(ItemId(20), 51054, ItemFlags::USED | ItemFlags::USE);
    stack.name = "Silver".to_string();
    stack.driver = IDR_ENHANCE;
    stack.value = 1_230;
    stack.carried_by = Some(character_id);
    stack.driver_data = vec![1, 123, 0, 0, 0];
    world.add_item(stack);
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"silver: name="Silver" ID=01010101 flag=IF_TAKE flag=IF_USE driver=61 arg="0101000000" ;"#,
        )
        .unwrap();

    assert_eq!(
        apply_nomad_stack(&mut world, &mut loader, ItemId(20), character_id),
        NomadStackApplyResult::Split {
            left: 73,
            right: 50,
            unit: "unit",
        }
    );
    let cursor_id = world
        .characters
        .get(&character_id)
        .unwrap()
        .cursor_item
        .unwrap();
    let carried = world.items.get(&ItemId(20)).unwrap();
    let cursor = world.items.get(&cursor_id).unwrap();
    assert_eq!(&carried.driver_data[..5], &[1, 73, 0, 0, 0]);
    assert_eq!(&cursor.driver_data[..5], &[1, 50, 0, 0, 0]);
    assert_eq!(carried.description, "73 units of Silver.");
    assert_eq!(cursor.description, "50 units of Silver.");
}

#[test]
fn enhance_material_enhances_cursor_item_with_legacy_sprite_and_cost() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.inventory[30] = Some(ItemId(20));
    character.cursor_item = Some(ItemId(21));
    let mut world = World::default();
    world.add_character(character);
    let mut material = test_item(ItemId(20), 51054, ItemFlags::USED | ItemFlags::USE);
    material.name = "Silver".to_string();
    material.driver = IDR_ENHANCE;
    material.value = 1_000;
    material.carried_by = Some(character_id);
    material.driver_data = vec![1, 244, 1, 0, 0];
    world.add_item(material);
    let mut target = test_item(ItemId(21), 10120, ItemFlags::USED | ItemFlags::TAKE);
    target.name = "Sword".to_string();
    target.carried_by = Some(character_id);
    target.modifier_index[0] = CharacterValue::Wisdom as i16;
    target.modifier_value[0] = 2;
    target.modifier_index[1] = CharacterValue::Weapon as i16;
    target.modifier_value[1] = 12;
    world.add_item(target);
    let mut loader = ZoneLoader::new();

    assert_eq!(
        apply_nomad_stack(&mut world, &mut loader, ItemId(20), character_id),
        NomadStackApplyResult::Enhanced {
            used: 300,
            target_name: "Sword".to_string(),
        }
    );
    let material = world.items.get(&ItemId(20)).unwrap();
    assert_eq!(stack_count(material), 200);
    assert_eq!(material.value, 400);
    let target = world.items.get(&ItemId(21)).unwrap();
    assert_eq!(target.sprite, 59300);
    assert_eq!(target.value, 600);
    assert_eq!(target.modifier_value[0], 3);
    assert_eq!(target.modifier_value[1], 22);
    assert!(world
        .characters
        .get(&character_id)
        .unwrap()
        .flags
        .contains(CharacterFlags::ITEMS));
}

#[test]
fn enhance_material_prompts_before_making_item_unusable() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.inventory[30] = Some(ItemId(20));
    character.cursor_item = Some(ItemId(21));
    character.values[1][CharacterValue::Sword as usize] = 14;
    let mut world = World::default();
    world.add_character(character);
    let mut material = test_item(ItemId(20), 51054, ItemFlags::USED | ItemFlags::USE);
    material.name = "Silver".to_string();
    material.driver = IDR_ENHANCE;
    material.carried_by = Some(character_id);
    material.driver_data = vec![1, 244, 1, 0, 0];
    world.add_item(material);
    let mut target = test_item(ItemId(21), 10120, ItemFlags::USED | ItemFlags::TAKE);
    target.carried_by = Some(character_id);
    target.modifier_index[0] = -(CharacterValue::Sword as i16);
    target.modifier_value[0] = 5;
    world.add_item(target);
    let mut loader = ZoneLoader::new();

    assert_eq!(
        apply_nomad_stack(&mut world, &mut loader, ItemId(20), character_id),
        NomadStackApplyResult::EnhanceConfirmUnusable
    );
    let material = world.items.get(&ItemId(20)).unwrap();
    assert_eq!(read_driver_data_u32(material, 8), ItemId(21).0);
    assert_eq!(stack_count(material), 500);
    assert_eq!(world.items.get(&ItemId(21)).unwrap().sprite, 10120);
}
