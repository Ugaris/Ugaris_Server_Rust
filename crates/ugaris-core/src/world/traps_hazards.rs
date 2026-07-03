use super::*;

impl World {
    pub(crate) fn discover_steptrap_target(&mut self, item_id: ItemId) -> bool {
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        if item.driver != IDR_STEPTRAP || item.driver_data.first().copied().unwrap_or(0) != 0 {
            return false;
        }

        let origin_x = usize::from(item.x);
        let origin_y = usize::from(item.y);
        let target = [1_u8, 3, 5, 7].into_iter().find_map(|dir| {
            let direction = Direction::try_from(dir).ok()?;
            let (dx, dy) = direction.delta();
            [1_i16, 2].into_iter().find_map(|distance| {
                let x = offset_coordinate(origin_x, dx * distance)?;
                let y = offset_coordinate(origin_y, dy * distance)?;
                if !self.map.legacy_inner_bounds(x, y) {
                    return None;
                }
                let target_item_id = self.map.tile(x, y)?.item;
                let target_item = self.items.get(&ItemId(u32::from(target_item_id)))?;
                (target_item.driver != 0 && target_item.driver != IDR_STEPTRAP).then_some((x, y))
            })
        });

        let Some((x, y)) = target else {
            return false;
        };
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        item.driver_data.resize(2, 0);
        item.driver_data[0] = x as u8;
        item.driver_data[1] = y as u8;
        true
    }

    pub(crate) fn open_trapdoor(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        target_x: u16,
        target_y: u16,
        schedule_after_ticks: u64,
    ) -> bool {
        let Some((x, y)) = self
            .items
            .get(&item_id)
            .map(|item| (usize::from(item.x), usize::from(item.y)))
        else {
            return false;
        };
        if !self.teleport_character_exact(
            character_id,
            usize::from(target_x),
            usize::from(target_y),
        ) {
            return false;
        }
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        item.driver_data.resize(1, 0);
        item.driver_data[0] = 1;
        item.sprite += 1;
        if let Some(tile) = self.map.tile_mut(x, y) {
            tile.flags.insert(MapFlags::TMOVEBLOCK);
        }
        self.mark_dirty_sector(x, y);
        self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
        self.pending_system_texts.push(WorldSystemText {
            character_id,
            message: "A trapdoor opens under your feet, but you manage to jump back in time."
                .to_string(),
        });
        true
    }

    pub(crate) fn block_trapdoor(&mut self, item_id: ItemId, cursor_item_id: ItemId) -> bool {
        let Some((x, y)) = self.items.get(&item_id).map(|item| (item.x, item.y)) else {
            return false;
        };
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        item.driver_data.resize(1, 0);
        item.driver_data[0] = 2;
        item.sprite += 2;
        self.mark_dirty_sector(usize::from(x), usize::from(y));
        self.destroy_item(cursor_item_id)
    }

    pub(crate) fn close_trapdoor(&mut self, item_id: ItemId) -> bool {
        let Some((x, y)) = self.items.get(&item_id).map(|item| (item.x, item.y)) else {
            return false;
        };
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        if item.driver_data.first().copied().unwrap_or_default() != 1 {
            return false;
        }
        item.driver_data[0] = 0;
        item.sprite -= 1;
        if let Some(tile) = self.map.tile_mut(usize::from(x), usize::from(y)) {
            tile.flags.remove(MapFlags::TMOVEBLOCK);
        }
        self.mark_dirty_sector(usize::from(x), usize::from(y));
        true
    }

    pub(crate) fn apply_gastrap_foreground(&mut self, item_id: ItemId, animation: u8) -> bool {
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        let origin_x = usize::from(item.x);
        let origin_y = usize::from(item.y);
        let Some((x, y, base_sprite)) = [(0_i16, 0_i16), (1, 0), (-1, 0), (0, 1), (0, -1)]
            .into_iter()
            .filter_map(|(dx, dy)| {
                let x = offset_coordinate(origin_x, dx)?;
                let y = offset_coordinate(origin_y, dy)?;
                let sprite = self.map.tile(x, y)?.foreground_sprite;
                let base = match sprite {
                    15291..=15299 => 15291,
                    15300..=15308 => 15300,
                    15309..=15317 => 15309,
                    15318..=15326 => 15318,
                    _ => return None,
                };
                Some((x, y, base))
            })
            .next()
        else {
            return false;
        };
        if let Some(tile) = self.map.tile_mut(x, y) {
            tile.foreground_sprite = base_sprite + u32::from(animation);
            self.mark_dirty_sector(x, y);
            true
        } else {
            false
        }
    }

    pub(crate) fn mark_flamethrower_targets_for_burn(&mut self, item_id: ItemId, direction: u8) {
        let Some(item) = self.items.get(&item_id) else {
            return;
        };
        let Some(direction) = Direction::try_from(direction).ok() else {
            return;
        };
        let (dx, dy) = direction.delta();
        let origin_x = usize::from(item.x);
        let origin_y = usize::from(item.y);

        for distance in [1_i16, 2] {
            let Some(x) = offset_coordinate(origin_x, dx * distance) else {
                continue;
            };
            let Some(y) = offset_coordinate(origin_y, dy * distance) else {
                continue;
            };
            if !self.map.legacy_inner_bounds(x, y) {
                continue;
            }
            let Some(character_id) = self.map.tile(x, y).and_then(|tile| {
                (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
            }) else {
                continue;
            };
            self.burn_character(character_id);
        }
    }

    pub fn burn_character(&mut self, character_id: CharacterId) -> bool {
        if self.effects.values().any(|effect| {
            effect.effect_type == EF_BURN && effect.target_character == Some(character_id)
        }) {
            return false;
        }

        let effect_id = self.next_effect_id();
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        let mut effect = Effect::new(
            EF_BURN,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(TICKS_PER_SECOND * 60) as i32,
        );
        effect.light = 250;
        effect.strength = 1;
        effect.target_character = Some(character_id);
        effect.x = i32::from(character.x);
        effect.y = i32::from(character.y);
        self.effects.insert(effect_id, effect);
        self.apply_legacy_hurt(character_id, None, 20 * POWERSCALE, 1, 50, 75);
        true
    }

    pub(crate) fn apply_palace_bomb_explosion(
        &mut self,
        item_id: ItemId,
        owner_id: u32,
        x: u16,
        y: u16,
    ) {
        let x_usize = usize::from(x);
        let y_usize = usize::from(y);
        self.create_explosion_effect(i32::from(x), i32::from(y), 8, 50050);
        self.queue_sound_area(x_usize, y_usize, 6);

        let mut burn_targets = Vec::new();
        for dy in -1..=1 {
            for dx in -1..=1 {
                let Some(tx) = offset_coordinate(x_usize, dx) else {
                    continue;
                };
                let Some(ty) = offset_coordinate(y_usize, dy) else {
                    continue;
                };
                let Some(character_id) = self.map.tile(tx, ty).and_then(|tile| {
                    (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
                }) else {
                    continue;
                };
                let Some(character) = self.characters.get(&character_id) else {
                    continue;
                };
                if character.name == "Islena" {
                    continue;
                }
                if character.flags.contains(CharacterFlags::PLAYER) && character.id.0 != owner_id {
                    continue;
                }
                burn_targets.push(character_id);
            }
        }
        for character_id in burn_targets {
            self.create_palace_bomb_burn_effect(character_id);
        }
        self.destroy_item(item_id);
    }

    pub(crate) fn apply_palace_cap_timer(&mut self, item_id: ItemId, schedule_after_ticks: u64) {
        self.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);

        let Some(character_id) = self.items.get(&item_id).and_then(|item| item.carried_by) else {
            return;
        };
        let Some(character) = self.characters.get(&character_id) else {
            return;
        };
        if character.inventory[worn_slot::HEAD] != Some(item_id) {
            return;
        }

        let regen_end = u64::from(character.regen_ticker).saturating_add(TICKS_PER_SECOND * 4);
        let should_deactivate = self
            .items
            .get(&item_id)
            .is_some_and(|item| item.driver_data.first().copied().unwrap_or_default() != 0)
            && self.tick.0 < regen_end;
        let should_activate = self.tick.0 > regen_end && character.action == action::IDLE;

        if should_deactivate {
            if let Some(item) = self.items.get_mut(&item_id) {
                if item.driver_data.is_empty() {
                    item.driver_data.resize(1, 0);
                }
                item.driver_data[0] = 0;
                item.sprite = item.sprite.saturating_sub(1);
            }
            if let Some(character) = self.characters.get_mut(&character_id) {
                character.flags.insert(CharacterFlags::ITEMS);
            }
            return;
        }

        if !should_activate {
            return;
        }

        let mut changed = false;
        if let Some(item) = self.items.get_mut(&item_id) {
            if item.driver_data.is_empty() {
                item.driver_data.resize(1, 0);
            }
            if item.driver_data[0] == 0 {
                item.driver_data[0] = 1;
                item.sprite += 1;
                changed = true;
            }
        }
        if changed {
            if let Some(character) = self.characters.get_mut(&character_id) {
                character.flags.insert(CharacterFlags::ITEMS);
            }
        }
        self.create_or_refresh_cap_effect(character_id);
    }
}
