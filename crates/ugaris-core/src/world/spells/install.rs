use super::*;

impl World {
    pub(crate) fn install_bless_spell(
        &mut self,
        target_id: CharacterId,
        strength: i32,
        duration: i32,
    ) -> bool {
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&target, &self.items, IDR_BLESS, self.tick.0 as u32) else {
            return false;
        };
        let old_item_id = target.inventory.get(slot).copied().flatten();
        if let Some(item_id) = old_item_id {
            self.items.remove(&item_id);
            self.remove_show_effect_type(target_id, EF_BLESS);
            // C `bless_someone`/`bless_self` (`act.c:1113-1117`,
            // `act.c:1156-1158`): `update_char(co)` right after destroying
            // the pre-existing bless item.
            self.update_character(target_id);
        }

        let item_id = self.next_runtime_item_id();
        let mut driver_data = Vec::with_capacity(12);
        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add(duration.max(0) as u32);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());
        driver_data.extend_from_slice(&strength.to_le_bytes());

        let item = Item {
            id: item_id,
            name: "Bless".to_string(),
            description: "A Spell of Bless.".to_string(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [
                CharacterValue::Intelligence as i16,
                CharacterValue::Wisdom as i16,
                CharacterValue::Agility as i16,
                CharacterValue::Strength as i16,
                0,
            ],
            modifier_value: [
                (strength / 4) as i16,
                (strength / 4) as i16,
                (strength / 4) as i16,
                (strength / 4) as i16,
                0,
            ],
            x: 0,
            y: 0,
            carried_by: Some(target_id),
            contained_in: None,
            content_id: 0,
            driver: IDR_BLESS,
            driver_data,
            serial: item_id.0,
        };

        self.items.insert(item_id, item);
        let character_serial;
        if let Some(target) = self.characters.get_mut(&target_id) {
            if target.inventory.len() <= slot {
                return false;
            }
            target.inventory[slot] = Some(item_id);
            character_serial = target.id.0;
            target
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        } else {
            return false;
        }
        // C `bless_someone`/`bless_self`: `update_char(co)` again after
        // installing the new bless item.
        self.update_character(target_id);
        self.schedule_spell_remove_timer(target_id, item_id, slot, character_serial, item_id.0);
        self.create_show_effect(
            EF_BLESS,
            target_id,
            start_tick,
            expire_tick,
            0,
            strength / 4,
        );
        true
    }

    /// C `/killbless` (`command.c:9605-9617`, `cmdcmp(ptr, "killbless",
    /// 5)`, no permission gate): scans equip slots 12..30 (`SPELL_SLOT_
    /// START..SPELL_SLOT_END`) for the first item with `IDR_BLESS`
    /// driver; if found, removes the `EF_BLESS` show-effect, destroys the
    /// item (which also clears the inventory slot, C's `ch[cn].item[n] =
    /// 0`), calls `update_char`, and returns `true` (caller logs "Done.").
    /// Returns `false` when no bless item is present (caller logs "No
    /// Bless found.").
    pub fn kill_bless_item(&mut self, character_id: CharacterId) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let bless_item_id = character.inventory[SPELL_SLOT_START..SPELL_SLOT_END]
            .iter()
            .flatten()
            .find(|&&item_id| {
                self.items
                    .get(&item_id)
                    .is_some_and(|item| item.driver == IDR_BLESS)
            })
            .copied();

        let Some(item_id) = bless_item_id else {
            return false;
        };

        self.remove_show_effect_type(character_id, EF_BLESS);
        self.destroy_item(item_id);
        self.update_character(character_id);
        true
    }

    pub fn install_bonus_spell(
        &mut self,
        target_id: CharacterId,
        driver: u16,
        strength: i32,
        duration: i32,
    ) -> bool {
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&target, &self.items, driver, self.tick.0 as u32) else {
            return false;
        };
        let Some((name, modifier_index)) = bonus_spell_shape(driver) else {
            return false;
        };

        let item_id = self.next_runtime_item_id();
        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add(duration.max(0) as u32);
        let mut driver_data = Vec::with_capacity(4);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());

        let item = Item {
            id: item_id,
            name: name.to_string(),
            description: format!("A Spell of {name}."),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [modifier_index as i16, 0, 0, 0, 0],
            modifier_value: [
                strength.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                0,
                0,
                0,
                0,
            ],
            x: 0,
            y: 0,
            carried_by: Some(target_id),
            contained_in: None,
            content_id: 0,
            driver,
            driver_data,
            serial: item_id.0,
        };

        self.items.insert(item_id, item);
        let character_serial;
        if let Some(target) = self.characters.get_mut(&target_id) {
            if target.inventory.len() <= slot {
                self.items.remove(&item_id);
                return false;
            }
            target.inventory[slot] = Some(item_id);
            character_serial = target.id.0;
            target
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        } else {
            self.items.remove(&item_id);
            return false;
        }
        // C `add_bonus_spell` (`src/system/drvlib.c:2646`): `update_char(cn)`.
        self.update_character(target_id);
        self.schedule_spell_remove_timer(target_id, item_id, slot, character_serial, item_id.0);
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn install_beyond_potion_spell(
        &mut self,
        character_id: CharacterId,
        potion_item_id: ItemId,
        duration_minutes: u8,
        modifier_index: [i16; MAX_MODIFIERS],
        modifier_value: [i16; MAX_MODIFIERS],
        beyond_max_mod: bool,
        consume_source_item: bool,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&character, &self.items, IDR_POTION_SP, self.tick.0 as u32)
        else {
            return false;
        };
        if !self
            .items
            .get(&potion_item_id)
            .is_some_and(|item| item.carried_by == Some(character_id))
        {
            return false;
        }

        let item_id = self.next_runtime_item_id();
        let start_tick = self.tick.0 as u32;
        let duration_ticks = u32::from(duration_minutes) * 60 * TICKS_PER_SECOND as u32;
        let expire_tick = start_tick.wrapping_add(duration_ticks);
        let mut driver_data = Vec::with_capacity(8);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());

        let mut flags = ItemFlags::USED;
        if beyond_max_mod {
            flags.insert(ItemFlags::BEYONDMAXMOD);
        }
        let item = Item {
            id: item_id,
            name: "Potion Spell".to_string(),
            description: "A potion spell.".to_string(),
            flags,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index,
            modifier_value,
            x: 0,
            y: 0,
            carried_by: Some(character_id),
            contained_in: None,
            content_id: 0,
            driver: IDR_POTION_SP,
            driver_data,
            serial: item_id.0,
        };

        if consume_source_item && !self.destroy_item(potion_item_id) {
            return false;
        }
        self.items.insert(item_id, item);
        let character_serial;
        if let Some(character) = self.characters.get_mut(&character_id) {
            if character.inventory.len() <= slot {
                self.items.remove(&item_id);
                return false;
            }
            character.inventory[slot] = Some(item_id);
            character_serial = character.id.0;
            character
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        } else {
            self.items.remove(&item_id);
            return false;
        }
        // C `add_potion_spell` (`src/module/alchemy.c:1007`): `update_char(cn)`.
        self.update_character(character_id);
        self.schedule_spell_remove_timer(character_id, item_id, slot, character_serial, item_id.0);
        self.create_show_effect(
            EF_POTION,
            character_id,
            start_tick,
            expire_tick,
            0,
            i32::from(modifier_value[0]),
        );
        true
    }

    pub(crate) fn install_speed_spell(
        &mut self,
        target_id: CharacterId,
        driver: u16,
        name: &str,
        speed_modifier: i32,
        duration: i32,
    ) -> bool {
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&target, &self.items, driver, self.tick.0 as u32) else {
            return false;
        };

        let item_id = self.next_runtime_item_id();
        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add(duration.max(0) as u32);
        let mut driver_data = Vec::with_capacity(8);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());

        let item = Item {
            id: item_id,
            name: name.to_string(),
            description: format!("A Spell of {name}."),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [CharacterValue::Speed as i16, 0, 0, 0, 0],
            modifier_value: [speed_modifier as i16, 0, 0, 0, 0],
            x: 0,
            y: 0,
            carried_by: Some(target_id),
            contained_in: None,
            content_id: 0,
            driver,
            driver_data,
            serial: item_id.0,
        };

        self.items.insert(item_id, item);
        let character_serial;
        if let Some(target) = self.characters.get_mut(&target_id) {
            if target.inventory.len() <= slot {
                self.items.remove(&item_id);
                return false;
            }
            target.inventory[slot] = Some(item_id);
            character_serial = target.id.0;
            target
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        } else {
            self.items.remove(&item_id);
            return false;
        }
        // C `warcry_someone`/`freeze_someone` (`act.c:1324`, `act.c:1522`):
        // `update_char(co)`.
        self.update_character(target_id);
        self.schedule_spell_remove_timer(target_id, item_id, slot, character_serial, item_id.0);
        match driver {
            IDR_FREEZE => {
                self.create_show_effect(EF_FREEZE, target_id, start_tick, expire_tick, 0, 0);
            }
            IDR_WARCRY => {
                self.create_show_effect(EF_WARCRY, target_id, start_tick, expire_tick, 0, 0);
            }
            _ => {}
        }
        true
    }

    pub(crate) fn install_curse_spell(
        &mut self,
        target_id: CharacterId,
        strength: i32,
        max_strength: i32,
    ) -> bool {
        if strength <= 0 || max_strength <= 0 {
            return false;
        }
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(slot) = add_same_spell_slot(&target, &self.items, IDR_CURSE) else {
            return false;
        };

        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add((30 * 60 * TICKS_PER_SECOND) as u32);
        if let Some(item_id) = target.inventory.get(slot).copied().flatten() {
            let Some(item) = self.items.get_mut(&item_id) else {
                return false;
            };
            let current_strength = -i32::from(item.modifier_value[0]);
            if current_strength >= max_strength {
                return false;
            }
            let added_strength = strength.min(max_strength - current_strength);
            for value in &mut item.modifier_value[..4] {
                *value = (i32::from(*value) - added_strength)
                    .clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
            let mut missing_effect = None;
            if let Some(effect) = self.effects.values_mut().find(|effect| {
                effect.effect_type == EF_CURSE && effect.target_character == Some(target_id)
            }) {
                effect.strength += added_strength;
            } else {
                missing_effect = Some((
                    read_spell_start_tick(&item.driver_data).unwrap_or(start_tick),
                    read_spell_expire_tick(&item.driver_data).unwrap_or(expire_tick),
                    -i32::from(item.modifier_value[0]),
                ));
            }
            if let Some((effect_start, effect_stop, effect_strength)) = missing_effect {
                self.create_show_effect(
                    EF_CURSE,
                    target_id,
                    effect_start,
                    effect_stop,
                    0,
                    effect_strength,
                );
            }
            if let Some(target) = self.characters.get_mut(&target_id) {
                target
                    .flags
                    .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
            }
            // C `ice_curse` (`src/system/act.c:1470`): `update_char(co)`
            // unconditionally at the end, for both the existing-item and
            // new-item branches.
            self.update_character(target_id);
            return true;
        }

        let item_id = self.next_runtime_item_id();
        let mut driver_data = Vec::with_capacity(8);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());
        let item = Item {
            id: item_id,
            name: "Curse".to_string(),
            description: "A Spell of Curse.".to_string(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [
                CharacterValue::Intelligence as i16,
                CharacterValue::Wisdom as i16,
                CharacterValue::Agility as i16,
                CharacterValue::Strength as i16,
                0,
            ],
            modifier_value: [
                (-strength).clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                (-strength).clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                (-strength).clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                (-strength).clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                0,
            ],
            x: 0,
            y: 0,
            carried_by: Some(target_id),
            contained_in: None,
            content_id: 0,
            driver: IDR_CURSE,
            driver_data,
            serial: item_id.0,
        };

        self.items.insert(item_id, item);
        let character_serial;
        if let Some(target) = self.characters.get_mut(&target_id) {
            if target.inventory.len() <= slot {
                self.items.remove(&item_id);
                return false;
            }
            target.inventory[slot] = Some(item_id);
            character_serial = target.id.0;
            target
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        } else {
            self.items.remove(&item_id);
            return false;
        }
        self.update_character(target_id);
        self.schedule_spell_remove_timer(target_id, item_id, slot, character_serial, item_id.0);
        self.create_show_effect(EF_CURSE, target_id, start_tick, expire_tick, 0, strength);
        true
    }

    pub(crate) fn install_firering_spell(&mut self, target_id: CharacterId) -> bool {
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&target, &self.items, IDR_FIRERING, self.tick.0 as u32)
        else {
            return false;
        };

        let item_id = self.next_runtime_item_id();
        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add(crate::tick::TICKS_PER_SECOND as u32);
        let mut driver_data = Vec::with_capacity(8);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());

        let item = Item {
            id: item_id,
            name: "Firering".to_string(),
            description: "A Spell of Firering.".to_string(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [0, 0, 0, 0, 0],
            modifier_value: [0, 0, 0, 0, 0],
            x: 0,
            y: 0,
            carried_by: Some(target_id),
            contained_in: None,
            content_id: 0,
            driver: IDR_FIRERING,
            driver_data,
            serial: item_id.0,
        };

        self.items.insert(item_id, item);
        let character_serial;
        if let Some(target) = self.characters.get_mut(&target_id) {
            if target.inventory.len() <= slot {
                self.items.remove(&item_id);
                return false;
            }
            target.inventory[slot] = Some(item_id);
            character_serial = target.id.0;
            target
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        } else {
            self.items.remove(&item_id);
            return false;
        }
        // Note: C `act_firering` does not call `update_char` (the item
        // carries no modifiers), but `update_character` is a superset of
        // the old `refresh_driver_spell_flags` call this replaces and is a
        // no-op for firering's flags, so this keeps identical observable
        // behavior while removing the last direct `refresh_driver_spell_flags`
        // call site in this file.
        self.update_character(target_id);
        self.schedule_spell_remove_timer(target_id, item_id, slot, character_serial, item_id.0);
        true
    }

    pub(crate) fn install_infravision_spell(&mut self, target_id: CharacterId) -> bool {
        self.install_timed_identity_spell(
            target_id,
            IDR_INFRARED,
            TICKS_PER_SECOND * 60 * 10,
            "Infravision",
            "A Spell of Infravision.",
        )
    }

    pub(crate) fn install_oxygen_spell(&mut self, target_id: CharacterId) -> bool {
        self.install_oxygen_spell_for_ticks(target_id, TICKS_PER_SECOND * 60)
    }

    pub(crate) fn install_oxygen_spell_for_ticks(
        &mut self,
        target_id: CharacterId,
        duration_ticks: u64,
    ) -> bool {
        self.install_timed_identity_spell(
            target_id,
            IDR_OXYGEN,
            duration_ticks,
            "Oxygen",
            "A Spell of Oxygen.",
        )
    }

    pub(crate) fn apply_lab3_whiteberry(
        &mut self,
        target_id: CharacterId,
        light_power: i16,
    ) -> (bool, bool) {
        if light_power <= 0 {
            return (false, false);
        }

        let existing_light_id = self.characters.get(&target_id).and_then(|character| {
            character.inventory[SPELL_SLOT_START..SPELL_SLOT_END]
                .iter()
                .filter_map(|slot| *slot)
                .find(|item_id| {
                    self.items.get(item_id).is_some_and(|item| {
                        item.driver == IDR_LAB3_PLANT && item.driver_data.first() == Some(&10)
                    })
                })
        });

        if let Some(item_id) = existing_light_id {
            let Some(old_item_light) = self.items.get(&item_id).map(|item| item.modifier_value[0])
            else {
                return (false, false);
            };
            let new_item_light = old_item_light.saturating_add(light_power).min(255);
            if let Some(item) = self.items.get_mut(&item_id) {
                item.modifier_value[0] = new_item_light;
            }
            let old_character_light = self
                .characters
                .get(&target_id)
                .map(character_light_value)
                .unwrap_or_default();
            if let Some(character) = self.characters.get_mut(&target_id) {
                if let Some(light) = character
                    .values
                    .get_mut(0)
                    .and_then(|values| values.get_mut(CharacterValue::Light as usize))
                {
                    *light = light.saturating_add(new_item_light - old_item_light);
                }
                character
                    .flags
                    .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
            }
            self.refresh_character_light_after_value_change(target_id, old_character_light);
            return (true, false);
        }

        let Some(slot) = self.characters.get(&target_id).and_then(|character| {
            character.inventory[SPELL_SLOT_START..SPELL_SLOT_END]
                .iter()
                .rposition(|slot| slot.is_none())
                .map(|offset| SPELL_SLOT_START + offset)
        }) else {
            return (false, false);
        };

        let item_light = light_power.saturating_mul(4).saturating_div(3).min(255);
        if item_light <= 0 {
            return (false, false);
        }

        let item_id = self.next_runtime_item_id();
        let item = Item {
            id: item_id,
            name: "Whiteberry Light".to_string(),
            description: "A whiteberry light spell.".to_string(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [CharacterValue::Light as i16, 0, 0, 0, 0],
            modifier_value: [item_light, 0, 0, 0, 0],
            x: 0,
            y: 0,
            carried_by: Some(target_id),
            contained_in: None,
            content_id: 0,
            driver: IDR_LAB3_PLANT,
            driver_data: vec![10, 0, 0, item_light as u8],
            serial: item_id.0,
        };

        let old_character_light = self
            .characters
            .get(&target_id)
            .map(character_light_value)
            .unwrap_or_default();
        self.items.insert(item_id, item);
        if let Some(character) = self.characters.get_mut(&target_id) {
            if character.inventory.len() <= slot {
                self.items.remove(&item_id);
                return (false, false);
            }
            character.inventory[slot] = Some(item_id);
            if let Some(light) = character
                .values
                .get_mut(0)
                .and_then(|values| values.get_mut(CharacterValue::Light as usize))
            {
                *light = light.saturating_add(item_light);
            }
            character
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        } else {
            self.items.remove(&item_id);
            return (false, false);
        }
        self.refresh_character_light_after_value_change(target_id, old_character_light);
        self.schedule_item_driver_timer_with_context(
            item_id,
            CharacterId(0),
            20 * TICKS_PER_SECOND,
            true,
        );
        (true, true)
    }

    pub(crate) fn decay_lab3_whiteberry_light(&mut self, item_id: ItemId) -> bool {
        let Some((target_id, old_item_light)) = self.items.get(&item_id).and_then(|item| {
            (item.driver == IDR_LAB3_PLANT && item.driver_data.first() == Some(&10))
                .then_some((item.carried_by?, item.modifier_value[0]))
        }) else {
            return false;
        };
        let old_character_light = self
            .characters
            .get(&target_id)
            .map(character_light_value)
            .unwrap_or_default();
        let new_item_light = 3 * old_item_light / 4;

        if new_item_light < 8 {
            if let Some(character) = self.characters.get_mut(&target_id) {
                if let Some(light) = character
                    .values
                    .get_mut(0)
                    .and_then(|values| values.get_mut(CharacterValue::Light as usize))
                {
                    *light = light.saturating_sub(old_item_light);
                }
                character
                    .flags
                    .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
            }
            self.destroy_item(item_id);
            self.refresh_character_light_after_value_change(target_id, old_character_light);
            return true;
        }

        if let Some(item) = self.items.get_mut(&item_id) {
            item.modifier_value[0] = new_item_light;
            if item.driver_data.len() < 4 {
                item.driver_data.resize(4, 0);
            }
            item.driver_data[3] = new_item_light as u8;
        }
        if let Some(character) = self.characters.get_mut(&target_id) {
            if let Some(light) = character
                .values
                .get_mut(0)
                .and_then(|values| values.get_mut(CharacterValue::Light as usize))
            {
                *light = light.saturating_add(new_item_light - old_item_light);
            }
            character.flags.insert(CharacterFlags::UPDATE);
        }
        self.refresh_character_light_after_value_change(target_id, old_character_light);
        self.schedule_item_driver_timer_with_context(
            item_id,
            CharacterId(0),
            20 * TICKS_PER_SECOND,
            true,
        );
        false
    }

    pub(crate) fn install_underwater_talk_spell(
        &mut self,
        target_id: CharacterId,
        duration_ticks: u64,
    ) -> bool {
        self.install_timed_identity_spell(
            target_id,
            IDR_UWTALK,
            duration_ticks,
            "Underwater Talk",
            "A Spell of Underwater Talk.",
        )
    }

    pub(crate) fn install_timed_identity_spell(
        &mut self,
        target_id: CharacterId,
        driver: u16,
        duration_ticks: u64,
        name: &str,
        description: &str,
    ) -> bool {
        if duration_ticks == 0 {
            return false;
        }
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&target, &self.items, driver, self.tick.0 as u32) else {
            return false;
        };

        let item_id = self.next_runtime_item_id();
        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add(duration_ticks as u32);
        let mut driver_data = Vec::with_capacity(8);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());

        let item = Item {
            id: item_id,
            name: name.to_string(),
            description: description.to_string(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [0, 0, 0, 0, 0],
            modifier_value: [0, 0, 0, 0, 0],
            x: 0,
            y: 0,
            carried_by: Some(target_id),
            contained_in: None,
            content_id: 0,
            driver,
            driver_data,
            serial: item_id.0,
        };

        self.items.insert(item_id, item);
        let character_serial;
        if let Some(target) = self.characters.get_mut(&target_id) {
            if target.inventory.len() <= slot {
                self.items.remove(&item_id);
                return false;
            }
            target.inventory[slot] = Some(item_id);
            character_serial = target.id.0;
            target
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        } else {
            self.items.remove(&item_id);
            return false;
        }
        // C `add_spell` (`src/system/tool.c:1683`): `update_char(cn)`.
        self.update_character(target_id);
        self.schedule_spell_remove_timer(target_id, item_id, slot, character_serial, item_id.0);
        true
    }

    pub(crate) fn remove_driver_spells(&mut self, target_id: CharacterId, driver: u16) {
        let item_ids: Vec<ItemId> = self
            .characters
            .get(&target_id)
            .map(|character| {
                character.inventory[SPELL_SLOT_START..SPELL_SLOT_END]
                    .iter()
                    .filter_map(|slot| *slot)
                    .filter(|item_id| {
                        self.items
                            .get(item_id)
                            .is_some_and(|item| item.driver == driver)
                    })
                    .collect()
            })
            .unwrap_or_default();

        let any_removed = !item_ids.is_empty();
        for item_id in item_ids {
            self.destroy_item(item_id);
        }
        // Mirrors C's `remove_poison`/`remove_all_poison` pattern
        // (`src/system/poison.c:117-150`): `update_char(cn)` only if an
        // item was actually removed.
        if any_removed {
            self.update_character(target_id);
        }
    }
}
