use super::*;

#[test]
fn world_palace_key_final_combine_consumes_cursor_and_creates_final_key() {
    let mut world = World::default();
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(7));
    character.cursor_item = Some(ItemId(8));
    let mut carried = item(7, ItemFlags::USED | ItemFlags::USE);
    carried.carried_by = Some(CharacterId(1));
    carried.driver = IDR_PALACEKEY;
    carried.template_id = crate::item_driver::IID_AREA11_PALACEKEYPART;
    carried.sprite = 51015;
    let mut cursor = item(8, ItemFlags::USED | ItemFlags::USE);
    cursor.carried_by = Some(CharacterId(1));
    cursor.driver = IDR_PALACEKEY;
    cursor.template_id = crate::item_driver::IID_AREA11_PALACEKEYPART;
    cursor.sprite = 51039;
    world.add_character(character);
    world.add_item(carried);
    world.add_item(cursor);

    let outcome = world.execute_item_driver_request_with_context(
        ItemDriverRequest::Driver {
            driver: IDR_PALACEKEY,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        11,
        &ItemDriverContext {
            cursor_template_id: Some(crate::item_driver::IID_AREA11_PALACEKEYPART),
            cursor_sprite: Some(51039),
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::PalaceKeyCombine {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            cursor_item_id: ItemId(8),
            result_sprite: 51014,
            final_key: true,
        }
    );
    assert!(!world.items.contains_key(&ItemId(8)));
    let carried = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(carried.sprite, 51014);
    assert_eq!(
        carried.template_id,
        crate::item_driver::IID_AREA11_PALACEKEY
    );
    assert_eq!(carried.driver, 0);
    assert!(!carried.flags.contains(ItemFlags::USE));
    assert_eq!(carried.name, "Palace Key");
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.cursor_item, None);
    assert!(character.flags.contains(CharacterFlags::ITEMS));
}

#[test]
fn world_applies_torch_orb_extraction_to_inventory() {
    let mut world = World::default();
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(7));
    let mut torch = item(7, ItemFlags::USED | ItemFlags::USE);
    torch.carried_by = Some(CharacterId(1));
    torch.driver = IDR_TORCH;
    torch.modifier_index[1] = CharacterValue::Speed as i16;
    torch.modifier_value[1] = 2;
    let mut orb = item(8, ItemFlags::USED | ItemFlags::USE);
    orb.name = "Orb of Speed".to_string();
    orb.carried_by = Some(CharacterId(1));
    orb.driver_data = vec![CharacterValue::Speed as u8, 1];
    world.add_character(character);
    world.add_item(torch);

    assert!(world.apply_torch_extract_orb(ItemId(7), CharacterId(1), 1, orb));

    let torch = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(torch.modifier_value[1], 1);
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.inventory[31], Some(ItemId(8)));
    assert!(character.flags.contains(CharacterFlags::ITEMS));
    assert_eq!(
        world.items.get(&ItemId(8)).unwrap().carried_by,
        Some(CharacterId(1))
    );
}

#[test]
fn world_enchants_cursor_equipment_and_consumes_orb() {
    let mut world = World::default();
    let mut character = character(1);
    character.cursor_item = Some(ItemId(8));
    character.inventory[30] = Some(ItemId(7));
    let mut orb = item(7, ItemFlags::USED | ItemFlags::USE);
    orb.carried_by = Some(CharacterId(1));
    orb.driver = IDR_ENCHANTITEM;
    orb.driver_data = vec![CharacterValue::Sword as u8, 3];
    let equipment = item(8, ItemFlags::USED | ItemFlags::WNNECK);
    world.add_character(character);
    world.add_item(orb);
    world.add_item(equipment);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_ENCHANTITEM,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::EnchantCursorItem { .. }
    ));
    assert!(!world.items.contains_key(&ItemId(7)));
    let equipment = world.items.get(&ItemId(8)).unwrap();
    assert_eq!(equipment.modifier_index[0], CharacterValue::Sword as i16);
    assert_eq!(equipment.modifier_value[0], 3);
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.inventory[30], None);
    assert_eq!(character.cursor_item, Some(ItemId(8)));
    assert!(character.flags.contains(CharacterFlags::ITEMS));
}

#[test]
fn world_blocks_enchant_beyond_legacy_limits_without_consuming_orb() {
    let mut world = World::default();
    let mut character = character(1);
    character.cursor_item = Some(ItemId(8));
    character.inventory[30] = Some(ItemId(7));
    let mut orb = item(7, ItemFlags::USED | ItemFlags::USE);
    orb.carried_by = Some(CharacterId(1));
    orb.driver = IDR_ENCHANTITEM;
    orb.driver_data = vec![CharacterValue::Sword as u8, 2];
    let mut equipment = item(8, ItemFlags::USED | ItemFlags::WNNECK);
    equipment.modifier_index[0] = CharacterValue::Sword as i16;
    equipment.modifier_value[0] = 19;
    world.add_character(character);
    world.add_item(orb);
    world.add_item(equipment);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_ENCHANTITEM,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::BlockedByRequirements {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
    assert!(world.items.contains_key(&ItemId(7)));
    assert_eq!(world.items.get(&ItemId(8)).unwrap().modifier_value[0], 19);
}

#[test]
fn world_anti_enchant_reduces_or_removes_cursor_equipment_modifier() {
    let mut world = World::default();
    let mut character = character(1);
    character.cursor_item = Some(ItemId(8));
    character.inventory[30] = Some(ItemId(7));
    character.inventory[31] = Some(ItemId(9));
    let mut anti_orb = item(7, ItemFlags::USED | ItemFlags::USE);
    anti_orb.carried_by = Some(CharacterId(1));
    anti_orb.driver = IDR_ANTIENCHANTITEM;
    anti_orb.driver_data = vec![CharacterValue::Sword as u8, 2];
    let mut second_anti_orb = item(9, ItemFlags::USED | ItemFlags::USE);
    second_anti_orb.carried_by = Some(CharacterId(1));
    second_anti_orb.driver = IDR_ANTIENCHANTITEM;
    second_anti_orb.driver_data = vec![CharacterValue::Sword as u8, 3];
    let mut equipment = item(8, ItemFlags::USED | ItemFlags::WNNECK);
    equipment.modifier_index[0] = CharacterValue::Sword as i16;
    equipment.modifier_value[0] = 5;
    world.add_character(character);
    world.add_item(anti_orb);
    world.add_item(second_anti_orb);
    world.add_item(equipment);

    let request = |item_id| ItemDriverRequest::Driver {
        driver: IDR_ANTIENCHANTITEM,
        item_id: ItemId(item_id),
        character_id: CharacterId(1),
        spec: 0,
    };
    assert!(matches!(
        world.execute_item_driver_request(request(7), 1),
        ItemDriverOutcome::AntiEnchantCursorItem { .. }
    ));
    assert_eq!(world.items.get(&ItemId(8)).unwrap().modifier_value[0], 3);
    assert!(!world.items.contains_key(&ItemId(7)));

    assert!(matches!(
        world.execute_item_driver_request(request(9), 1),
        ItemDriverOutcome::AntiEnchantCursorItem { .. }
    ));
    let equipment = world.items.get(&ItemId(8)).unwrap();
    assert_eq!(equipment.modifier_index[0], 0);
    assert_eq!(equipment.modifier_value[0], 0);
    assert!(!world.items.contains_key(&ItemId(9)));
}

#[test]
fn world_applies_shrike_amulet_assembly() {
    let mut world = World::default();
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(7));
    character.cursor_item = Some(ItemId(8));
    world.add_character(character);

    let mut base = item(7, ItemFlags::USED | ItemFlags::USE);
    base.driver = crate::item_driver::IDR_SHRIKEAMULET;
    base.carried_by = Some(CharacterId(1));
    base.driver_data = vec![1];
    world.add_item(base);
    let mut cursor = item(8, ItemFlags::USED | ItemFlags::USE);
    cursor.driver = crate::item_driver::IDR_SHRIKEAMULET;
    cursor.carried_by = Some(CharacterId(1));
    cursor.driver_data = vec![2];
    world.add_item(cursor);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_SHRIKEAMULET,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        38,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::ShrikeAmuletAssemble { .. }
    ));
    let base = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(base.driver_data[0], 3);
    assert_eq!(base.sprite, 51620);
    assert_eq!(base.name, "Crystal on Chain");
    assert_eq!(base.description, "A light blue crystal on a silver chain.");
    assert!(!world.items.contains_key(&ItemId(8)));
    assert_eq!(world.characters[&CharacterId(1)].cursor_item, None);
}

#[test]
fn world_applies_arkhata_key_final_assembly() {
    let mut world = World::default();
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(7));
    character.cursor_item = Some(ItemId(8));
    world.add_character(character);

    let mut base = item(7, ItemFlags::USED | ItemFlags::USE);
    base.driver = crate::item_driver::IDR_ARKHATA;
    base.carried_by = Some(CharacterId(1));
    base.template_id = 0x0100_00CD;
    base.driver_data = vec![2];
    world.add_item(base);
    let mut cursor = item(8, ItemFlags::USED | ItemFlags::USE);
    cursor.driver = crate::item_driver::IDR_ARKHATA;
    cursor.carried_by = Some(CharacterId(1));
    cursor.template_id = 0x0100_00CC;
    world.add_item(cursor);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_ARKHATA,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        37,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::ArkhataKeyAssemble {
            final_key: true,
            ..
        }
    ));
    let base = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(base.sprite, 13413);
    assert_eq!(base.template_id, 0x3B00_0089);
    assert_eq!(base.name, "Knoger Key 1");
    assert_eq!(
        base.description,
        "A finished key. Should open something now. A door, perhaps."
    );
    assert!(!world.items.contains_key(&ItemId(8)));
    assert_eq!(world.characters[&CharacterId(1)].cursor_item, None);
}

#[test]
fn lizard_flower_mixer_updates_carried_flower_and_consumes_cursor() {
    let mut world = World::default();
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(10));
    character.cursor_item = Some(ItemId(11));
    world.add_character(character);

    let mut carried = item(10, ItemFlags::USED | ItemFlags::USE);
    carried.carried_by = Some(CharacterId(1));
    carried.driver = IDR_LIZARDFLOWER;
    carried.driver_data = vec![1];
    carried.sprite = 11190;
    world.items.insert(ItemId(10), carried);

    let mut cursor = item(11, ItemFlags::USED | ItemFlags::USE);
    cursor.carried_by = Some(CharacterId(1));
    cursor.driver = IDR_LIZARDFLOWER;
    cursor.driver_data = vec![6];
    cursor.sprite = 11191;
    world.items.insert(ItemId(11), cursor);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_LIZARDFLOWER,
            item_id: ItemId(10),
            character_id: CharacterId(1),
            spec: 0,
        },
        31,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::LizardFlowerMixed {
            combined_bits: 7,
            complete: true,
            bottle_message: true,
            ..
        }
    ));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.cursor_item, None);
    assert!(character.flags.contains(CharacterFlags::ITEMS));
    assert!(!world.items.contains_key(&ItemId(11)));
    let item = world.items.get(&ItemId(10)).unwrap();
    assert_eq!(item.driver_data[0], 7);
    assert_eq!(item.sprite, 11188);
    assert_eq!(item.driver, IDR_OXYPOTION);
    assert_eq!(item.name, "Scuba Potion");
    assert_eq!(item.description, "A bubbly fluid in a nice bottle.");
}

#[test]
fn world_randomshrine_key_context_scans_inventory_and_cursor() {
    let mut world = World::default();
    world.map = MapGrid::new(20, 20);
    let mut shrine = item(8, ItemFlags::USED | ItemFlags::USE);
    shrine.driver = crate::item_driver::IDR_RANDOMSHRINE;
    shrine.driver_data = vec![53, 17];
    assert!(world.map.set_item_map(&mut shrine, 10, 10));
    world.add_item(shrine);

    let mut player = character(1);
    player.inventory[30] = Some(ItemId(9));
    assert!(world.spawn_character(player, 9, 10));
    let mut key = item(9, ItemFlags::USED);
    key.template_id = crate::item_driver::IID_AREA14_SHRINEKEY;
    key.driver_data = vec![17];
    key.carried_by = Some(CharacterId(1));
    world.add_item(key);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_RANDOMSHRINE,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        14,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::RandomShrineUse {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            shrine_type: 53,
            level: 17,
            kind: crate::item_driver::RandomShrineKind::Security,
        }
    );

    world.items.get_mut(&ItemId(9)).unwrap().driver_data[0] = 18;
    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_RANDOMSHRINE,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        14,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::RandomShrineNeedsKey { .. }
    ));
}
