use super::*;

impl World {
    pub fn add_item(&mut self, item: Item) {
        if let Some(old) = self.items.remove(&item.id) {
            remove_item_light(&mut self.map, &old);
            self.mark_item_light_area(&old);
        }
        add_item_light(&mut self.map, &item);
        self.mark_item_light_area(&item);
        self.items.insert(item.id, item);
    }

    pub(crate) fn move_item_map_slot(&mut self, item_id: ItemId, from: (u16, u16), to: (u16, u16)) {
        let Some(item) = self.items.get(&item_id) else {
            return;
        };
        let item_flags = item.flags;
        let from_x = usize::from(from.0);
        let from_y = usize::from(from.1);
        let to_x = usize::from(to.0);
        let to_y = usize::from(to.1);

        if let Some(source) = self.map.tile_mut(from_x, from_y) {
            if source.item == item_id.0 {
                source.item = 0;
                if item_flags.contains(ItemFlags::MOVEBLOCK) {
                    source.flags.remove(MapFlags::TMOVEBLOCK);
                }
                if item_flags.contains(ItemFlags::SIGHTBLOCK) {
                    source.flags.remove(MapFlags::TSIGHTBLOCK);
                }
                self.mark_dirty_sector(from_x, from_y);
            }
        }

        if let Some(target) = self.map.tile_mut(to_x, to_y) {
            target.item = item_id.0;
            if item_flags.contains(ItemFlags::MOVEBLOCK) {
                target.flags.insert(MapFlags::TMOVEBLOCK);
            }
            if item_flags.contains(ItemFlags::SIGHTBLOCK) {
                target.flags.insert(MapFlags::TSIGHTBLOCK);
            }
            self.mark_dirty_sector(to_x, to_y);
        }
    }

    pub(crate) fn character_has_template_id(
        &self,
        character_id: CharacterId,
        template_id: u32,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        character
            .cursor_item
            .into_iter()
            .chain(character.inventory.iter().flatten().copied())
            .any(|item_id| {
                self.items
                    .get(&item_id)
                    .is_some_and(|item| item.template_id == template_id)
            })
    }

    pub(crate) fn character_inventory_item_by_template(
        &self,
        character_id: CharacterId,
        template_id: u32,
    ) -> Option<(ItemId, String)> {
        let character = self.characters.get(&character_id)?;
        character.inventory.iter().flatten().find_map(|item_id| {
            let item = self.items.get(item_id)?;
            (item.template_id == template_id).then(|| (*item_id, item.name.clone()))
        })
    }

    pub(crate) fn item_can_be_set_on_map(&self, item: &Item, x: usize, y: usize) -> bool {
        if x < 1
            || y < 1
            || x >= self.map.width()
            || y >= self.map.height()
            || item.flags.is_empty()
        {
            return false;
        }
        self.map.tile(x, y).is_some_and(|tile| {
            tile.item == 0
                && !tile
                    .flags
                    .intersects(MapFlags::TMOVEBLOCK | MapFlags::MOVEBLOCK)
        })
    }

    pub(crate) fn character_holds_cursor_item(
        &self,
        character_id: CharacterId,
        item_id: ItemId,
    ) -> bool {
        self.characters
            .get(&character_id)
            .is_some_and(|character| character.cursor_item == Some(item_id))
    }

    pub fn destroy_item(&mut self, item_id: ItemId) -> bool {
        let Some(mut item) = self.items.remove(&item_id) else {
            return false;
        };

        if let Some(character_id) = item.carried_by {
            if let Some(character) = self.characters.get_mut(&character_id) {
                if character.cursor_item == Some(item_id) {
                    character.cursor_item = None;
                }
                for slot in &mut character.inventory {
                    if *slot == Some(item_id) {
                        *slot = None;
                    }
                }
                character.flags.insert(CharacterFlags::ITEMS);
            }
        }

        if item.x != 0 {
            self.map.remove_item_map(&mut item);
        }
        true
    }

    pub(crate) fn transfer_cursor_item(
        &mut self,
        giver_id: CharacterId,
        receiver_id: CharacterId,
    ) -> bool {
        if giver_id == receiver_id {
            return false;
        }
        let Some(giver) = self.characters.get(&giver_id) else {
            return false;
        };
        let Some(receiver) = self.characters.get(&receiver_id) else {
            return false;
        };
        if receiver
            .flags
            .intersects(CharacterFlags::DEAD | CharacterFlags::NOGIVE)
        {
            return false;
        }
        let Some(item_id) = giver.cursor_item else {
            return false;
        };
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        if item.flags.contains(ItemFlags::QUEST)
            && !giver
                .flags
                .intersects(CharacterFlags::QUESTITEM | CharacterFlags::GOD)
            && !receiver
                .flags
                .intersects(CharacterFlags::QUESTITEM | CharacterFlags::GOD)
        {
            return false;
        }
        if !can_receive_given_item(receiver) {
            return false;
        }

        let Some(receiver) = self.characters.get_mut(&receiver_id) else {
            return false;
        };
        if receiver.cursor_item.is_none() {
            receiver.cursor_item = Some(item_id);
        } else if receiver.flags.contains(CharacterFlags::PLAYER) {
            let Some(slot) = receiver
                .inventory
                .iter_mut()
                .skip(INVENTORY_START_INVENTORY)
                .find(|slot| slot.is_none())
            else {
                return false;
            };
            *slot = Some(item_id);
        } else {
            return false;
        }
        receiver.flags.insert(CharacterFlags::ITEMS);

        let Some(giver) = self.characters.get_mut(&giver_id) else {
            return false;
        };
        if giver.cursor_item != Some(item_id) {
            return false;
        }
        giver.cursor_item = None;
        giver.flags.insert(CharacterFlags::ITEMS);

        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        item.carried_by = Some(receiver_id);
        if let Some(receiver) = self.characters.get_mut(&receiver_id) {
            receiver.push_driver_message(NT_GIVE, giver_id.0 as i32, item_id.0 as i32, 0);
        }
        true
    }

    pub(crate) fn next_runtime_item_id(&self) -> ItemId {
        let next = self
            .items
            .keys()
            .map(|item_id| item_id.0)
            .max()
            .unwrap_or_default()
            .saturating_add(1)
            .max(1);
        ItemId(next)
    }

    /// C `can_wear` (`src/system/tool.c:994`): true if `item_id` may be
    /// placed into worn slot `pos` (`0..=11`, `WN_*`) for `character_id` -
    /// the item's `IF_WN*` slot flag must match `pos`, the hand slots
    /// additionally veto two-handed conflicts (`IF_WNTWOHANDED` in the
    /// opposite hand blocks `WN_LHAND`; a two-handed item is rejected for
    /// `WN_RHAND` if `WN_LHAND` is occupied at all), and
    /// `check_requirements` (min/max level, class gate, negative
    /// modifier-index stat requirements, `IF_BONDWEAR` ownership) must
    /// pass.
    pub fn can_wear(&self, character_id: CharacterId, item_id: ItemId, pos: usize) -> bool {
        if !LEGACY_EQUIPMENT_SLOTS.contains(&pos) {
            return false;
        }
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };

        let right_hand_is_two_handed = character
            .inventory
            .get(worn_slot::RIGHT_HAND)
            .copied()
            .flatten()
            .and_then(|id| self.items.get(&id))
            .is_some_and(|item| item.flags.contains(ItemFlags::WNTWOHANDED));
        let left_hand_occupied = character
            .inventory
            .get(worn_slot::LEFT_HAND)
            .copied()
            .flatten()
            .is_some();

        let slot_matches = match pos {
            worn_slot::HEAD => item.flags.contains(ItemFlags::WNHEAD),
            worn_slot::NECK => item.flags.contains(ItemFlags::WNNECK),
            worn_slot::BODY => item.flags.contains(ItemFlags::WNBODY),
            worn_slot::ARMS => item.flags.contains(ItemFlags::WNARMS),
            worn_slot::BELT => item.flags.contains(ItemFlags::WNBELT),
            worn_slot::LEGS => item.flags.contains(ItemFlags::WNLEGS),
            worn_slot::FEET => item.flags.contains(ItemFlags::WNFEET),
            worn_slot::CLOAK => item.flags.contains(ItemFlags::WNCLOAK),
            worn_slot::LEFT_RING => item.flags.contains(ItemFlags::WNLRING),
            worn_slot::RIGHT_RING => item.flags.contains(ItemFlags::WNRRING),
            worn_slot::LEFT_HAND => {
                !right_hand_is_two_handed && item.flags.contains(ItemFlags::WNLHAND)
            }
            worn_slot::RIGHT_HAND => {
                if item.flags.contains(ItemFlags::WNTWOHANDED) {
                    !left_hand_occupied
                } else {
                    item.flags.contains(ItemFlags::WNRHAND)
                }
            }
            _ => false,
        };
        if !slot_matches {
            return false;
        }

        check_requirements(character, item)
    }
}

/// C `check_requirements` (`src/system/tool.c:943`): negative
/// `modifier_index` entries are stat requirements checked against
/// `value[1]` (the base/raised value, not the equipment-modified
/// effective total), plus `min_level`/`max_level`/`needs_class` gates and
/// `IF_BONDWEAR` ownership.
pub(crate) fn check_requirements(character: &Character, item: &Item) -> bool {
    for (&mod_index, &mod_value) in item.modifier_index.iter().zip(item.modifier_value.iter()) {
        if mod_value == 0 || mod_index >= 0 {
            continue;
        }
        // C `check_requirements` (`src/system/tool.c:952-958`): out-of-range
        // indices (`v1 <= -V_MAX || v1 >= V_MAX`) are illegal data, cleared
        // and skipped rather than treated as a requirement.
        if mod_index <= -(CHARACTER_VALUE_COUNT as i16) {
            continue;
        }
        let required_index = (-mod_index) as usize;
        let current = character
            .values
            .get(1)
            .and_then(|values| values.get(required_index))
            .copied()
            .unwrap_or_default();
        if current < mod_value {
            return false;
        }
    }

    if item.min_level != 0 && character.level < u32::from(item.min_level) {
        return false;
    }
    if item.max_level != 0 && character.level > u32::from(item.max_level) {
        return false;
    }

    if item.needs_class & 1 != 0 && character.flags.contains(CharacterFlags::MAGE) {
        return false;
    }
    if item.needs_class & 2 != 0 && character.flags.contains(CharacterFlags::WARRIOR) {
        return false;
    }
    if item.needs_class & 4 != 0
        && !character
            .flags
            .contains(CharacterFlags::MAGE | CharacterFlags::WARRIOR)
    {
        return false;
    }
    if item.needs_class & 8 != 0 && !character.flags.contains(CharacterFlags::ARCH) {
        return false;
    }

    if item.flags.contains(ItemFlags::BONDWEAR) && item.owner_id != character.id.0 as i32 {
        return false;
    }

    true
}

pub(crate) fn can_receive_given_item(character: &Character) -> bool {
    if character.cursor_item.is_none() {
        return true;
    }
    character.flags.contains(CharacterFlags::PLAYER)
        && character
            .inventory
            .iter()
            .skip(INVENTORY_START_INVENTORY)
            .any(|slot| slot.is_none())
}
