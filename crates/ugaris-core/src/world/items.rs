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

    /// C `destroy_item_byID(cn, ID)` (`src/system/questlog.c:1664-1696`):
    /// destroys *every* item matching `template_id` in `character_id`'s
    /// equipment slots (`0..12`), main inventory (`30..`), and cursor -
    /// deliberately skipping the spell slots (`12..30`), exactly like C's
    /// `if (n >= 12 && n < 30) continue`. Unlike C, this does not sweep
    /// the account depot (`DRD_DEPOT_PPD`) - that storage lives in
    /// `ugaris-server`'s `PlayerRuntime`/DB layer, not `World` (see
    /// `world::yoakin`'s module doc comment for why this gap is
    /// acceptable for its callers today).
    pub fn destroy_items_by_template_id(&mut self, character_id: CharacterId, template_id: u32) {
        let Some(character) = self.characters.get(&character_id) else {
            return;
        };
        let mut matching: Vec<ItemId> = character
            .inventory
            .iter()
            .enumerate()
            .filter(|(slot, _)| !(SPELL_SLOT_START..SPELL_SLOT_END).contains(slot))
            .filter_map(|(_, item_id)| *item_id)
            .collect();
        if let Some(cursor_item) = character.cursor_item {
            matching.push(cursor_item);
        }

        for item_id in matching {
            if self
                .items
                .get(&item_id)
                .is_some_and(|item| item.template_id == template_id)
            {
                self.destroy_item(item_id);
            }
        }
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

/// C `give_char_item_smart`'s return code (`src/system/tool.c:3396-3494`):
/// `1` (to inventory or hand) is collapsed into two explicit variants here
/// since callers may care which slot was used (C's own callers never do,
/// but the distinction is cheap to keep and matches the doc comment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GiveCharItemSmartResult {
    /// C return `3`: an `IF_MONEY` item was converted straight to gold.
    /// The caller (which has `PlayerRuntime`/DB access `World` lacks)
    /// should apply the `achievement_add_gold_earned` wealth ladder for
    /// this amount, matching every other `give_money`-derived reward in
    /// this codebase (see `World::complete_mission`'s doc comment for the
    /// established split).
    Money { amount: u32 },
    /// C return `1`, inventory branch.
    ToInventory,
    /// C return `1`, hand (`citem`) branch.
    ToHand,
    /// C return `2`: dropped on the ground next to the receiver.
    Dropped,
    /// C return `0`: destroyed (no space, `IF_NODROP`, or drop failure).
    Destroyed,
}

impl World {
    /// C `give_char_item_smart(cn, in, log_msg)` (`src/system/tool.c:3408-
    /// 3494`): tries the receiver's inventory, then hand, then a ground
    /// drop next to them, destroying the item only as a last resort (or
    /// immediately for `IF_MONEY`/`IF_NODROP` items, which are never
    /// dropped). `log_msg` gates the private `log_char`/`quiet`-style
    /// feedback the same way C's own flag does; the `dlog` audit-log
    /// lines have no Rust equivalent anywhere in this codebase (dev-only
    /// diagnostic, consistently unported) and are not reproduced here.
    /// Returns `false` only if `target_id`/`item_id` don't resolve to a
    /// live character/item (a state C can't represent since both are
    /// always-valid array indices there).
    pub fn give_char_item_smart(
        &mut self,
        target_id: CharacterId,
        item_id: ItemId,
        log_msg: bool,
    ) -> Option<GiveCharItemSmartResult> {
        let item = self.items.get(&item_id)?.clone();
        self.characters.get(&target_id)?;

        // C: `if (it[in].flags & IF_MONEY) { ... give_money(cn, amount,
        // ...); destroy_item(in); return 3; }` (`tool.c:3411-3432`).
        if item.flags.contains(ItemFlags::MONEY) {
            let amount = item.value;
            if let Some(character) = self.characters.get_mut(&target_id) {
                character.gold = character.gold.saturating_add(amount);
                character.flags.insert(CharacterFlags::ITEMS);
            }
            if log_msg {
                self.queue_system_text_bytes(target_id, give_money_message(amount));
            }
            self.destroy_item(item_id);
            return Some(GiveCharItemSmartResult::Money { amount });
        }

        // C: first try inventory (`tool.c:3434-3450`).
        if let Some(character) = self.characters.get_mut(&target_id) {
            if let Some(slot) = character
                .inventory
                .iter_mut()
                .skip(INVENTORY_START_INVENTORY)
                .find(|slot| slot.is_none())
            {
                *slot = Some(item_id);
                character.flags.insert(CharacterFlags::ITEMS);
                if let Some(item) = self.items.get_mut(&item_id) {
                    item.carried_by = Some(target_id);
                }
                if log_msg {
                    self.queue_system_text(target_id, format!("You received {}.", item.name));
                }
                return Some(GiveCharItemSmartResult::ToInventory);
            }
        }

        // C: inventory full, try hand (`tool.c:3452-3466`).
        if let Some(character) = self.characters.get_mut(&target_id) {
            if character.cursor_item.is_none() {
                character.cursor_item = Some(item_id);
                character.flags.insert(CharacterFlags::ITEMS);
                if let Some(item) = self.items.get_mut(&item_id) {
                    item.carried_by = Some(target_id);
                }
                if log_msg {
                    self.queue_system_text(
                        target_id,
                        format!("You received {} in your hand.", item.name),
                    );
                }
                return Some(GiveCharItemSmartResult::ToHand);
            }
        }

        // C: both full - `IF_NODROP` items are destroyed instead of
        // dropped (`tool.c:3467-3479`).
        if item.flags.contains(ItemFlags::NODROP) {
            if log_msg {
                self.queue_system_text(
                    target_id,
                    format!(
                        "You would have received {}, but it was destroyed as you have no space for it.",
                        item.name
                    ),
                );
            }
            self.destroy_item(item_id);
            return Some(GiveCharItemSmartResult::Destroyed);
        }

        // C: `drop_item`, falling back to `drop_item_extended(..., 1)`,
        // destroying only if both fail (`tool.c:3480-3502`).
        let (x, y) = self
            .characters
            .get(&target_id)
            .map(|character| (character.x, character.y))?;
        let mut item_mut = self.items.get(&item_id)?.clone();
        let dropped = self
            .map
            .drop_item(&mut item_mut, usize::from(x), usize::from(y))
            || self
                .map
                .drop_item_extended(&mut item_mut, usize::from(x), usize::from(y), 1);
        if dropped {
            self.items.insert(item_id, item_mut);
            if log_msg {
                self.queue_system_text(
                    target_id,
                    format!("You received {} (dropped at your feet).", item.name),
                );
            }
            Some(GiveCharItemSmartResult::Dropped)
        } else {
            if log_msg {
                self.queue_system_text(
                    target_id,
                    format!(
                        "You would have received {}, but it was destroyed as there was no space to drop it.",
                        item.name
                    ),
                );
            }
            self.destroy_item(item_id);
            Some(GiveCharItemSmartResult::Destroyed)
        }
    }

    /// C `give_char_item(cn, in)` (`src/system/tool.c:3371-3394`): the
    /// plain (non-"smart") give - places `item_id` into `target_id`'s
    /// cursor if empty, else the first free inventory slot (index `>=
    /// 30`); fails (returns `false`) if both are full, with no ground-drop
    /// or `IF_MONEY`/`IF_NODROP` handling at all (unlike
    /// `give_char_item_smart`). Several NPC drivers call this exact
    /// simpler variant for their "unwanted item" give-back branch (e.g.
    /// `jessica_driver`, `gwendylon.c:2040`; `trader_driver`, `base.c:
    /// 4475-4487`) rather than `give_char_item_smart` - a genuine C
    /// behavioral difference (no ground-drop fallback), not a
    /// simplification, so it is kept as its own method rather than folded
    /// into `give_char_item_smart`. `pub` (not `pub(crate)`) since
    /// `ugaris-server`'s `area1.rs::apply_guiwynn_events` needs it outside
    /// the `ugaris_core` crate boundary for the `guiwynn_driver` money-item
    /// reward (`create_money_item`+plain `give_char_item`, not
    /// `give_char_item_smart`'s auto-gold-conversion `IF_MONEY` branch).
    pub fn give_char_item(&mut self, target_id: CharacterId, item_id: ItemId) -> bool {
        let Some(target) = self.characters.get_mut(&target_id) else {
            return false;
        };
        if target.cursor_item.is_none() {
            target.cursor_item = Some(item_id);
        } else {
            let Some(slot) = target
                .inventory
                .iter_mut()
                .skip(INVENTORY_START_INVENTORY)
                .find(|slot| slot.is_none())
            else {
                return false;
            };
            *slot = Some(item_id);
        }
        target.flags.insert(CharacterFlags::ITEMS);
        if let Some(item) = self.items.get_mut(&item_id) {
            item.carried_by = Some(target_id);
        }
        true
    }
}

/// C `give_money`'s message half (`src/system/tool.c:1460-1474`): `"%ds"`
/// under 100 silver, otherwise `"%.2fG"`.
pub(crate) fn give_money_message(amount: u32) -> Vec<u8> {
    let mut message = b"You received".to_vec();
    message.extend_from_slice(crate::text::COL_YELLOW);
    if amount < 100 {
        message.extend_from_slice(format!(" {amount}s").as_bytes());
    } else {
        message.extend_from_slice(format!(" {:.2}G", f64::from(amount) / 100.0).as_bytes());
    }
    message.extend_from_slice(crate::text::COL_RESET);
    message.extend_from_slice(b". It has been placed in your gold pouch.");
    message
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
