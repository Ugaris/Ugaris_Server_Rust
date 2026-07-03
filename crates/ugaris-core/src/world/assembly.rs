use super::*;

impl World {
    pub(crate) fn apply_enchant_cursor_item(
        &mut self,
        orb_item_id: ItemId,
        character_id: CharacterId,
        target_item_id: ItemId,
        modifier: i16,
        amount: i16,
    ) -> bool {
        if amount <= 0 || !self.character_holds_cursor_item(character_id, target_item_id) {
            return false;
        }

        let Some(target) = self.items.get(&target_item_id) else {
            return false;
        };
        if !target.flags.intersects(ItemFlags::WEAR)
            || target.flags.contains(ItemFlags::NOENHANCE)
            || target.flags.contains(ItemFlags::WNLHAND)
        {
            return false;
        }

        let current = current_modifier_value(target, modifier).unwrap_or_default();
        let new_value = current.saturating_add(amount);
        if new_value > 20 {
            return false;
        }
        if current == 0 && counted_enhancement_modifiers(target) >= 3 {
            return false;
        }
        let Some(slot) = modifier_slot_for_write(target, modifier) else {
            return false;
        };

        if !self.destroy_item(orb_item_id) {
            return false;
        }
        let Some(target) = self.items.get_mut(&target_item_id) else {
            return false;
        };
        target.modifier_index[slot] = modifier;
        target.modifier_value[slot] = new_value;
        if let Some(character) = self.characters.get_mut(&character_id) {
            character.flags.insert(CharacterFlags::ITEMS);
        }
        true
    }

    pub(crate) fn apply_anti_enchant_cursor_item(
        &mut self,
        anti_orb_item_id: ItemId,
        character_id: CharacterId,
        target_item_id: ItemId,
        modifier: i16,
        amount: i16,
    ) -> bool {
        if amount <= 0 || !self.character_holds_cursor_item(character_id, target_item_id) {
            return false;
        }
        if matches!(modifier, x if x == CharacterValue::Armor as i16 || x == CharacterValue::Weapon as i16)
        {
            return false;
        }

        let Some(target) = self.items.get(&target_item_id) else {
            return false;
        };
        if !target.flags.intersects(ItemFlags::WEAR) || target.flags.contains(ItemFlags::NOENHANCE)
        {
            return false;
        }
        let Some(slot) = modifier_slot_with_positive_value(target, modifier) else {
            return false;
        };

        if !self.destroy_item(anti_orb_item_id) {
            return false;
        }
        let Some(target) = self.items.get_mut(&target_item_id) else {
            return false;
        };
        let new_value = target.modifier_value[slot] - amount;
        if new_value <= 0 {
            target.modifier_index[slot] = 0;
            target.modifier_value[slot] = 0;
        } else {
            target.modifier_value[slot] = new_value;
        }
        if let Some(character) = self.characters.get_mut(&character_id) {
            character.flags.insert(CharacterFlags::ITEMS);
        }
        true
    }

    pub(crate) fn apply_shrike_amulet_assemble(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        combined_bits: u8,
    ) -> bool {
        if !self.character_holds_cursor_item(character_id, cursor_item_id) {
            return false;
        }
        if !self.items.contains_key(&cursor_item_id) {
            return false;
        }
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        if item.carried_by != Some(character_id) {
            return false;
        }
        item.driver_data.resize(1, 0);
        item.driver_data[0] = combined_bits;
        item.sprite = 51617 + i32::from(combined_bits);
        match combined_bits {
            3 => {
                item.name = "Crystal on Chain".to_string();
                item.description = "A light blue crystal on a silver chain.".to_string();
            }
            5 => {
                item.name = "Crystal on Charm".to_string();
                item.description = "A light blue crystal on a silver crescent charm.".to_string();
            }
            6 => {
                item.name = "Charm on Chain".to_string();
                item.description = "A silver crescent charm on a silver chain.".to_string();
            }
            7 => {
                item.name = "Talisman".to_string();
                item.description = "A silver talisman.".to_string();
            }
            _ => {}
        }
        self.destroy_item(cursor_item_id)
    }

    pub(crate) fn apply_mine_gateway_key_assemble(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        combined_bits: u8,
    ) -> bool {
        if !self.character_holds_cursor_item(character_id, cursor_item_id) {
            return false;
        }
        if !self.items.contains_key(&cursor_item_id) {
            return false;
        }
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        item.driver_data.resize(1, 0);
        item.driver_data[0] = combined_bits;
        item.description = "A partially assembled key.".to_string();
        item.sprite = match combined_bits {
            1 => 52201,
            2 => 52202,
            3 => 52205,
            4 => 52203,
            5 => 52206,
            6 => 52209,
            7 => 52213,
            8 => 52204,
            9 => 52210,
            10 => 52207,
            11 => 52212,
            12 => 52208,
            13 => 52214,
            14 => 52211,
            15 => {
                item.flags.remove(ItemFlags::USE);
                item.template_id = IID_MINEGATEWAY;
                item.name = "Mine gateway key".to_string();
                item.description = "A fully assembled key.".to_string();
                52200
            }
            _ => item.sprite,
        };
        self.destroy_item(cursor_item_id)
    }

    pub(crate) fn apply_arkhata_key_assemble(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        result_template_id: u32,
        result_sprite: i32,
        final_key: bool,
    ) -> bool {
        if !self.character_holds_cursor_item(character_id, cursor_item_id) {
            return false;
        }
        if !self.items.contains_key(&cursor_item_id) {
            return false;
        }
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        if item.carried_by != Some(character_id) {
            return false;
        }

        item.sprite = result_sprite;
        item.template_id = result_template_id;
        if final_key {
            item.name = "Knoger Key 1".to_string();
            item.description =
                "A finished key. Should open something now. A door, perhaps.".to_string();
        }
        self.destroy_item(cursor_item_id)
    }

    pub(crate) fn apply_caligar_key_assemble(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        result_sprite: i32,
        final_key: bool,
    ) -> bool {
        if !self.character_holds_cursor_item(character_id, cursor_item_id) {
            return false;
        }
        if !self.items.contains_key(&cursor_item_id) {
            return false;
        }
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        if item.carried_by != Some(character_id) {
            return false;
        }

        if final_key {
            return true;
        }

        item.sprite = result_sprite;
        self.destroy_item(cursor_item_id)
    }

    pub(crate) fn apply_palace_key_combine(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        result_sprite: i32,
        final_key: bool,
    ) -> bool {
        if !self.character_holds_cursor_item(character_id, cursor_item_id) {
            return false;
        }
        if !self.items.contains_key(&cursor_item_id) {
            return false;
        }
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        if item.carried_by != Some(character_id) {
            return false;
        }

        item.sprite = result_sprite;
        if final_key {
            item.template_id = crate::item_driver::IID_AREA11_PALACEKEY;
            item.driver = 0;
            item.flags.remove(ItemFlags::USE);
            item.name = "Palace Key".to_string();
            item.description = "The key to the ice palace.".to_string();
        }
        self.destroy_item(cursor_item_id)
    }

    pub(crate) fn apply_lizard_flower_mixed(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        combined_bits: u8,
    ) -> bool {
        if !self.character_holds_cursor_item(character_id, cursor_item_id) {
            return false;
        }
        if !self.items.contains_key(&cursor_item_id) {
            return false;
        }
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        if item.carried_by != Some(character_id) {
            return false;
        }

        item.driver_data.resize(1, 0);
        item.driver_data[0] = combined_bits;
        if combined_bits == 7 {
            item.sprite = 11188;
            item.driver = crate::item_driver::IDR_OXYPOTION;
            item.name = "Scuba Potion".to_string();
            item.description = "A bubbly fluid in a nice bottle.".to_string();
        } else {
            item.sprite = 11189;
            item.description = "A partially finished scuba potion.".to_string();
        }
        self.destroy_item(cursor_item_id)
    }

    pub fn apply_torch_extract_orb(
        &mut self,
        torch_item_id: ItemId,
        character_id: CharacterId,
        modifier_slot: usize,
        mut orb: Item,
    ) -> bool {
        let Some(torch) = self.items.get(&torch_item_id) else {
            return false;
        };
        if torch.carried_by != Some(character_id)
            || modifier_slot >= torch.modifier_value.len()
            || torch.modifier_value[modifier_slot] <= 0
        {
            return false;
        }

        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        match give_item_to_character(
            character,
            &mut orb,
            GiveItemFlags::LOG.union(GiveItemFlags::ALLOW_DROP),
        ) {
            GiveItemResult::Ok => {}
            GiveItemResult::Dropped => {
                if !self.map.drop_item_extended(
                    &mut orb,
                    usize::from(character.x),
                    usize::from(character.y),
                    1,
                ) {
                    return false;
                }
            }
            GiveItemResult::Money => {}
            GiveItemResult::Full | GiveItemResult::Failed => return false,
        }

        let Some(torch) = self.items.get_mut(&torch_item_id) else {
            return false;
        };
        torch.modifier_value[modifier_slot] -= 1;
        self.add_item(orb);
        true
    }
}

pub(crate) fn current_modifier_value(item: &Item, modifier: i16) -> Option<i16> {
    item.modifier_index
        .iter()
        .zip(item.modifier_value.iter())
        .find_map(|(index, value)| (*index == modifier).then_some(*value))
}

pub(crate) fn modifier_slot_for_write(item: &Item, modifier: i16) -> Option<usize> {
    item.modifier_index
        .iter()
        .position(|index| *index == modifier)
        .or_else(|| item.modifier_value.iter().position(|value| *value == 0))
}

pub(crate) fn modifier_slot_with_positive_value(item: &Item, modifier: i16) -> Option<usize> {
    item.modifier_index
        .iter()
        .zip(item.modifier_value.iter())
        .position(|(index, value)| *index == modifier && *value > 0)
}

pub(crate) fn counted_enhancement_modifiers(item: &Item) -> usize {
    item.modifier_index
        .iter()
        .zip(item.modifier_value.iter())
        .filter(|(index, value)| {
            **value > 0
                && **index >= 0
                && !matches!(
                    **index,
                    x if x == CharacterValue::Weapon as i16
                        || x == CharacterValue::Armor as i16
                        || x == CharacterValue::Demon as i16
                        || x == CharacterValue::Light as i16
                )
        })
        .count()
}
