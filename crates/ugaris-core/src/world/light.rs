use super::*;

pub(crate) fn item_light_may_have_changed(outcome: &ItemDriverOutcome) -> bool {
    matches!(
        outcome,
        ItemDriverOutcome::LightChanged { .. }
            | ItemDriverOutcome::OnOffLightChanged { .. }
            | ItemDriverOutcome::FlameThrowerPulse { .. }
            | ItemDriverOutcome::FlameThrowerExtinguished { .. }
            | ItemDriverOutcome::SwampWhispPulse {
                light_changed: true,
                ..
            }
            | ItemDriverOutcome::DecayItemToggled { .. }
    )
}

pub(crate) fn item_light_value(item: &Item) -> i16 {
    item.modifier_index
        .iter()
        .zip(item.modifier_value.iter())
        .filter_map(|(&index, &value)| (index == CharacterValue::Light as i16).then_some(value))
        .sum()
}

pub(crate) fn character_light_value(character: &Character) -> i16 {
    character
        .values
        .first()
        .and_then(|values| values.get(CharacterValue::Light as usize))
        .copied()
        .unwrap_or_default()
}

pub(crate) fn integer_sqrt_for_light(strength: i16) -> usize {
    let strength = i32::from(strength).unsigned_abs().min(100) as usize;
    (strength.saturating_sub(1) as f64).sqrt() as usize + 1
}

impl World {
    pub fn skip_x_sector(&self, x: isize, y: isize, ticker: u64) -> usize {
        self.dirty_sectors.skip_x_sector(x, y, ticker)
    }

    pub(crate) fn mark_dirty_sector(&mut self, x: usize, y: usize) {
        self.dirty_sectors
            .set_sector(x as isize, y as isize, self.tick.0.max(1) as u64);
    }

    pub(crate) fn mark_light_area(&mut self, x: usize, y: usize, strength: i16) {
        if strength == 0 || self.map.tile(x, y).is_none() {
            return;
        }
        let radius = integer_sqrt_for_light(strength).min(LIGHT_DISTANCE);
        let min_x = x.saturating_sub(radius);
        let min_y = y.saturating_sub(radius);
        let max_x = x
            .saturating_add(radius)
            .min(self.map.width().saturating_sub(1));
        let max_y = y
            .saturating_add(radius)
            .min(self.map.height().saturating_sub(1));
        for ty in min_y..=max_y {
            for tx in min_x..=max_x {
                self.mark_dirty_sector(tx, ty);
            }
        }
    }

    pub(crate) fn mark_character_light_area(&mut self, character: &Character) {
        self.mark_light_area(
            usize::from(character.x),
            usize::from(character.y),
            character_light_value(character),
        );
    }

    pub(crate) fn mark_item_light_area(&mut self, item: &Item) {
        if item.x == 0 || item.y == 0 {
            return;
        }
        self.mark_light_area(
            usize::from(item.x),
            usize::from(item.y),
            item_light_value(item),
        );
    }

    pub fn compute_groundlight_at(&mut self, x: usize, y: usize) -> bool {
        let old_light = self.map.tile(x, y).map(|tile| tile.light);
        compute_groundlight(&mut self.map, x, y);
        let changed = self.map.tile(x, y).map(|tile| tile.light) != old_light;
        if changed {
            self.mark_dirty_sector(x, y);
        }
        changed
    }

    pub fn compute_shadow_at(&mut self, x: usize, y: usize) -> bool {
        self.compute_shadow_at_with_random(x, y, |_| 0)
    }

    pub fn compute_shadow_at_with_random(
        &mut self,
        x: usize,
        y: usize,
        random_below: impl FnMut(i32) -> i32,
    ) -> bool {
        let old_daylight = self.map.tile(x, y).map(|tile| tile.daylight);
        compute_shadow_with_random(&mut self.map, x, y, random_below);
        let changed = self.map.tile(x, y).map(|tile| tile.daylight) != old_daylight;
        if changed {
            self.mark_dirty_sector(x, y);
        }
        changed
    }

    pub fn compute_dlight_at(&mut self, x: usize, y: usize) -> bool {
        let changed = compute_dlight(&mut self.map, x, y);
        if changed {
            self.mark_dirty_sector(x, y);
        }
        changed
    }

    pub fn reset_dlight_around(&mut self, x: usize, y: usize) -> bool {
        if self.map.tile(x, y).is_none() {
            return false;
        }

        let xs = x.saturating_sub(LIGHT_DISTANCE);
        let ys = y.saturating_sub(LIGHT_DISTANCE);
        let xe = (x + 1 + LIGHT_DISTANCE).min(self.map.width().saturating_sub(1));
        let ye = (y + 1 + LIGHT_DISTANCE).min(self.map.height().saturating_sub(1));

        let mut before = HashMap::new();
        for ty in ys..ye {
            for tx in xs..xe {
                if let Some(tile) = self.map.tile(tx, ty) {
                    before.insert((tx, ty), tile.daylight);
                }
            }
        }

        if !reset_dlight(&mut self.map, x, y) {
            return false;
        }

        for ((tx, ty), old_daylight) in before {
            if self
                .map
                .tile(tx, ty)
                .is_some_and(|tile| tile.daylight != old_daylight)
            {
                self.mark_dirty_sector(tx, ty);
            }
        }
        true
    }

    pub fn schedule_existing_light_timers(&mut self) -> usize {
        let item_ids: Vec<ItemId> = self
            .items
            .iter()
            .filter_map(|(&item_id, item)| match item.driver {
                IDR_NIGHTLIGHT => Some(item_id),
                IDR_ONOFFLIGHT if item.driver_data.first().copied().unwrap_or(0) != 0 => {
                    Some(item_id)
                }
                IDR_TORCH if item.driver_data.first().copied().unwrap_or(0) != 0 => Some(item_id),
                IDR_CLANSPAWN => Some(item_id),
                IDR_FLAMETHROW | IDR_CALIGARFLAME | IDR_EDEMONLIGHT | IDR_EDEMONLOADER
                | IDR_EDEMONBLOCK | IDR_EDEMONTUBE | IDR_FDEMONLIGHT | IDR_FDEMONLOADER
                | IDR_FDEMONGATE | IDR_FDEMONFARM | IDR_MINEDOOR => Some(item_id),
                IDR_CALIGAR if matches!(item.driver_data.first().copied(), Some(2 | 4)) => {
                    Some(item_id)
                }
                _ => None,
            })
            .collect();

        item_ids
            .into_iter()
            .filter(|&item_id| {
                let character_id = self
                    .items
                    .get(&item_id)
                    .and_then(|item| item.carried_by)
                    .unwrap_or(CharacterId(0));
                self.schedule_item_driver_timer(item_id, character_id, 1)
            })
            .count()
    }

    pub(crate) fn schedule_registered_area3_lamp_extinguish(&mut self) -> usize {
        let mut item_ids: Vec<ItemId> = self
            .items
            .iter()
            .filter_map(|(&item_id, item)| {
                (item.driver == IDR_ONOFFLIGHT
                    && item.driver_data.get(6).copied().unwrap_or_default() != 0)
                    .then_some(item_id)
            })
            .collect();
        item_ids.sort_by_key(|item_id| item_id.0);

        item_ids
            .into_iter()
            .enumerate()
            .filter(|(index, item_id)| {
                self.schedule_item_driver_timer(*item_id, CharacterId(0), (*index as u64) + 1)
            })
            .count()
    }

    pub fn refresh_character_light_after_value_change(
        &mut self,
        character_id: CharacterId,
        old_light: i16,
    ) -> bool {
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        let new_light = character_light_value(character);
        if old_light == new_light {
            return false;
        }

        let mut before = character.clone();
        if let Some(values) = before.values.get_mut(0) {
            if let Some(light) = values.get_mut(CharacterValue::Light as usize) {
                *light = old_light;
            }
        }
        remove_character_light(&mut self.map, &before);
        add_character_light(&mut self.map, character);
        character.flags.insert(CharacterFlags::UPDATE);
        let after = character.clone();
        self.mark_character_light_area(&before);
        self.mark_character_light_area(&after);
        true
    }

    pub(crate) fn refresh_item_light_after_mutation(&mut self, before: &Item, item_id: ItemId) {
        remove_item_light(&mut self.map, before);
        self.mark_item_light_area(before);
        if let Some(after) = self.items.get(&item_id) {
            let after = after.clone();
            add_item_light(&mut self.map, &after);
            self.mark_item_light_area(&after);
        }
    }
}
