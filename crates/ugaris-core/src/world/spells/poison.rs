use super::*;

impl World {
    pub fn poison_character(
        &mut self,
        character_id: CharacterId,
        power: u16,
        poison_type: u16,
    ) -> bool {
        if poison_type > 3 {
            return false;
        }
        let driver = IDR_POISON0 + poison_type;
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&character, &self.items, driver, self.tick.0 as u32) else {
            return false;
        };

        let item_id = self.next_runtime_item_id();
        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add(POISON_DURATION as u32);
        let mut driver_data = Vec::with_capacity(12);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());
        driver_data.extend_from_slice(&power.to_le_bytes());
        driver_data.extend_from_slice(&9_u16.to_le_bytes());

        let item = Item {
            id: item_id,
            name: "Poison".to_string(),
            description: "A Spell of Poison.".to_string(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [CharacterValue::Hp as i16, 0, 0, 0, 0],
            modifier_value: [-1, 0, 0, 0, 0],
            x: 0,
            y: 0,
            carried_by: Some(character_id),
            contained_in: None,
            content_id: 0,
            driver,
            driver_data,
            serial: item_id.0,
        };

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
        // C `poison_someone` (`src/system/poison.c:61`): `update_char(cn)`.
        self.update_character(character_id);
        self.schedule_spell_remove_timer(character_id, item_id, slot, character_serial, item_id.0);
        self.schedule_poison_callback_timer(
            self.tick.0 + crate::tick::TICKS_PER_SECOND,
            character_id,
            item_id,
            slot,
            character_serial,
            item_id.0,
        );
        true
    }

    pub fn remove_poison(&mut self, character_id: CharacterId, poison_type: u16) -> bool {
        if poison_type > 3 {
            return false;
        }
        self.remove_poison_by_driver(character_id, IDR_POISON0 + poison_type)
    }

    pub fn remove_all_poison(&mut self, character_id: CharacterId) -> bool {
        let mut removed = false;
        for driver in IDR_POISON0..=IDR_POISON3 {
            removed |= self.remove_poison_by_driver(character_id, driver);
        }
        removed
    }

    pub(crate) fn remove_poison_by_driver(
        &mut self,
        character_id: CharacterId,
        driver: u16,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let slots: Vec<(usize, ItemId)> = character
            .inventory
            .iter()
            .copied()
            .enumerate()
            .skip(crate::spell::SPELL_SLOT_START)
            .take(crate::spell::SPELL_SLOT_END - crate::spell::SPELL_SLOT_START)
            .filter_map(|(slot, item_id)| {
                let item_id = item_id?;
                self.items
                    .get(&item_id)
                    .is_some_and(|item| item.driver == driver)
                    .then_some((slot, item_id))
            })
            .collect();
        if slots.is_empty() {
            return false;
        }
        let character = self
            .characters
            .get_mut(&character_id)
            .expect("checked above");
        for (slot, item_id) in slots {
            character.inventory[slot] = None;
            self.items.remove(&item_id);
        }
        character
            .flags
            .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        // C `remove_poison`/`remove_all_poison` (`src/system/poison.c:128`,
        // `:148`): `update_char(cn)` since an item was removed here.
        self.update_character(character_id);
        true
    }

    pub fn schedule_existing_spell_timers(&mut self) -> usize {
        let character_ids: Vec<_> = self.characters.keys().copied().collect();
        for character_id in character_ids {
            // Full recompute (rather than just the driver-spell flag subset)
            // so a character loaded with spell items already in inventory
            // has correct `value[0]` totals, matching what a live
            // `update_char` call would have already applied when each
            // spell was originally installed.
            self.update_character(character_id);
        }

        let mut spells = Vec::new();
        for (&character_id, character) in &self.characters {
            for (slot, item_id) in character.inventory.iter().copied().enumerate() {
                let Some(item_id) = item_id else {
                    continue;
                };
                let Some(item) = self.items.get(&item_id) else {
                    continue;
                };
                if !is_timed_spell_driver(item.driver) {
                    continue;
                }
                let Some(due) = read_spell_expire_tick(&item.driver_data) else {
                    continue;
                };
                spells.push((
                    character_id,
                    item_id,
                    slot,
                    character.id.0,
                    item.serial,
                    due as u64,
                    item.driver,
                    item.driver_data
                        .get(4..8)
                        .and_then(|bytes| Some(u32::from_le_bytes(bytes.try_into().ok()?))),
                    item.modifier_value[0],
                ));
            }
        }

        spells
            .into_iter()
            .filter(
                |&(
                    character_id,
                    item_id,
                    slot,
                    character_serial,
                    item_serial,
                    due,
                    driver,
                    start_tick,
                    modifier_value,
                )| {
                    let scheduled = self.set_spell_remove_timer(
                        due,
                        character_id,
                        item_id,
                        slot,
                        character_serial,
                        item_serial,
                    );
                    if scheduled {
                        if let Some(start_tick) = start_tick {
                            let stop_tick = due as u32;
                            match driver {
                                IDR_BLESS => {
                                    self.create_show_effect(
                                        EF_BLESS,
                                        character_id,
                                        start_tick,
                                        stop_tick,
                                        0,
                                        i32::from(modifier_value),
                                    );
                                }
                                IDR_FREEZE => {
                                    self.create_show_effect(
                                        EF_FREEZE,
                                        character_id,
                                        start_tick,
                                        stop_tick,
                                        0,
                                        0,
                                    );
                                }
                                IDR_WARCRY => {
                                    self.create_show_effect(
                                        EF_WARCRY,
                                        character_id,
                                        start_tick,
                                        stop_tick,
                                        0,
                                        0,
                                    );
                                }
                                IDR_POTION_SP => {
                                    self.create_show_effect(
                                        EF_POTION,
                                        character_id,
                                        start_tick,
                                        stop_tick,
                                        0,
                                        i32::from(modifier_value),
                                    );
                                }
                                IDR_CURSE => {
                                    self.create_show_effect(
                                        EF_CURSE,
                                        character_id,
                                        start_tick,
                                        stop_tick,
                                        0,
                                        -i32::from(modifier_value),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                    scheduled
                },
            )
            .count()
    }

    pub(crate) fn schedule_spell_remove_timer(
        &mut self,
        character_id: CharacterId,
        item_id: ItemId,
        slot: usize,
        character_serial: u32,
        item_serial: u32,
    ) -> bool {
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        let Some(due) = read_spell_expire_tick(&item.driver_data) else {
            return false;
        };
        if !is_timed_spell_driver(item.driver) {
            return false;
        }
        self.set_spell_remove_timer(
            due as u64,
            character_id,
            item_id,
            slot,
            character_serial,
            item_serial,
        )
    }

    pub(crate) fn set_spell_remove_timer(
        &mut self,
        due: u64,
        character_id: CharacterId,
        item_id: ItemId,
        slot: usize,
        character_serial: u32,
        item_serial: u32,
    ) -> bool {
        let Ok(slot) = i32::try_from(slot) else {
            return false;
        };
        self.timers.set_timer(
            due,
            REMOVE_SPELL_TIMER,
            TimerPayload([
                character_id.0 as i32,
                item_id.0 as i32,
                slot,
                character_serial as i32,
                item_serial as i32,
            ]),
        )
    }

    pub(crate) fn schedule_poison_callback_timer(
        &mut self,
        due: u64,
        character_id: CharacterId,
        item_id: ItemId,
        slot: usize,
        character_serial: u32,
        item_serial: u32,
    ) -> bool {
        let Ok(slot) = i32::try_from(slot) else {
            return false;
        };
        self.timers.set_timer(
            due,
            POISON_CALLBACK_TIMER,
            TimerPayload([
                character_id.0 as i32,
                item_id.0 as i32,
                slot,
                character_serial as i32,
                item_serial as i32,
            ]),
        )
    }

    pub(crate) fn poison_callback_from_timer(
        &mut self,
        character_id: CharacterId,
        item_id: ItemId,
        slot: usize,
        character_serial: u32,
        item_serial: u32,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        if !character.flags.contains(CharacterFlags::USED) || character.id.0 != character_serial {
            return false;
        }
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        if item.serial != item_serial || !matches!(item.driver, IDR_POISON0..=IDR_POISON3) {
            return false;
        }
        if character.inventory.get(slot).copied().flatten() != Some(item_id) {
            return false;
        }
        let Some(mut power) = read_poison_power(&item.driver_data) else {
            return false;
        };
        let Some(mut tick) = read_poison_tick(&item.driver_data) else {
            return false;
        };
        power = power.clamp(1, 20);

        if tick == 0 {
            if let Some(item) = self.items.get_mut(&item_id) {
                item.modifier_value[0] = item.modifier_value[0].saturating_sub(1).max(-1000);
            }
            if let Some(character) = self.characters.get_mut(&character_id) {
                character.flags.insert(CharacterFlags::UPDATE);
            }
        }

        self.apply_legacy_hurt(character_id, None, crate::entity::POWERSCALE / 3, 1, 0, 50);

        tick = if tick == 0 { 9 } else { tick - 1 };
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        write_poison_tick(&mut item.driver_data, tick);
        let due = self.tick.0 + (crate::tick::TICKS_PER_SECOND * 2 / u64::from(power));
        self.schedule_poison_callback_timer(
            due,
            character_id,
            item_id,
            slot,
            character_serial,
            item_serial,
        );
        true
    }

    pub(crate) fn remove_spell_from_timer(
        &mut self,
        character_id: CharacterId,
        item_id: ItemId,
        slot: usize,
        character_serial: u32,
        item_serial: u32,
    ) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if !character.flags.contains(CharacterFlags::USED) || character.id.0 != character_serial {
            return false;
        }
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        if item.serial != item_serial {
            return false;
        }
        let spell_driver = item.driver;
        if character.inventory.get(slot).copied().flatten() != Some(item_id) {
            return false;
        }

        let old_speed = character_value(character, CharacterValue::Speed);
        let old_duration = character.duration;
        character.inventory[slot] = None;
        character
            .flags
            .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        self.items.remove(&item_id);
        // C `remove_spell` (`src/system/tool.c:1591`): `update_char(cn)`
        // right after removing the item, before the freeze-duration
        // rescale below.
        self.update_character(character_id);
        if spell_driver == IDR_FREEZE && old_duration != 0 {
            let Some(character) = self.characters.get_mut(&character_id) else {
                return true;
            };
            let new_speed = character_value(character, CharacterValue::Speed);
            let real_duration = speed_ticks_inverse(old_speed, character.speed_mode, old_duration);
            let new_duration = speed_ticks(new_speed, character.speed_mode, real_duration);
            character.duration = new_duration;
            character.step = character.step * new_duration / old_duration;
        }
        true
    }

    pub(crate) fn complete_heal(&mut self, caster_id: CharacterId, target_id: CharacterId) -> bool {
        if caster_id == target_id {
            let Some(caster) = self.characters.get(&caster_id).cloned() else {
                return false;
            };
            if !self
                .characters
                .get_mut(&target_id)
                .is_some_and(|target| act_heal(&caster, target))
            {
                return false;
            }
            self.create_show_effect(
                EF_HEAL,
                target_id,
                self.tick.0 as u32,
                self.tick.0.saturating_add(8) as u32,
                0,
                0,
            );
            // C `act_heal` (`act.c:1671-1674`): `NT_CHAR` gated on
            // `CF_NONOTIFY`, then unconditional `NT_SPELL`, broadcast from
            // the caster's own position (not the target's).
            if !caster.flags.contains(CharacterFlags::NONOTIFY) {
                self.notify_area(caster.x, caster.y, NT_CHAR, caster_id.0 as i32, 0, 0);
            }
            self.notify_area(
                caster.x,
                caster.y,
                NT_SPELL,
                caster_id.0 as i32,
                CharacterValue::Heal as i32,
                0,
            );
            return true;
        }

        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if !self
            .characters
            .get_mut(&target_id)
            .is_some_and(|target| act_heal(&caster, target))
        {
            return false;
        }
        self.create_show_effect(
            EF_HEAL,
            target_id,
            self.tick.0 as u32,
            self.tick.0.saturating_add(8) as u32,
            0,
            0,
        );
        if !caster.flags.contains(CharacterFlags::NONOTIFY) {
            self.notify_area(caster.x, caster.y, NT_CHAR, caster_id.0 as i32, 0, 0);
        }
        self.notify_area(
            caster.x,
            caster.y,
            NT_SPELL,
            caster_id.0 as i32,
            CharacterValue::Heal as i32,
            0,
        );
        true
    }
}
