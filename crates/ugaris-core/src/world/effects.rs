use super::*;

impl World {
    pub(crate) fn next_effect_id(&self) -> u32 {
        self.effects.keys().copied().max().unwrap_or(0) + 1
    }

    pub(crate) fn create_fireball_effect(&mut self, caster: &Character) -> u32 {
        let effect_id = self.next_effect_id();
        let power = spell_power(
            character_value(caster, CharacterValue::Fireball),
            character_value(caster, CharacterValue::Tactics),
        );
        let mut effect = Effect::new(
            EF_FIREBALL,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(TICKS_PER_SECOND) as i32,
        );
        effect.strength = power;
        effect.light = 200;
        effect.from_x = i32::from(caster.x);
        effect.from_y = i32::from(caster.y);
        effect.to_x = caster.act1;
        effect.to_y = caster.act2;
        effect.caster = Some(caster.id);
        effect.caster_serial = caster.id.0 as i32;
        effect.x = i32::from(caster.x) * 1024 + 512;
        effect.y = i32::from(caster.y) * 1024 + 512;
        self.effects.insert(effect_id, effect);
        effect_id
    }

    pub(crate) fn create_reflected_fireball_effect(
        &mut self,
        reflector: &Character,
        caster: &Character,
        strength: i32,
    ) -> u32 {
        let effect_id = self.next_effect_id();
        let mut effect = Effect::new(
            EF_FIREBALL,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(TICKS_PER_SECOND) as i32,
        );
        effect.strength = strength;
        effect.light = 200;
        effect.from_x = i32::from(reflector.x);
        effect.from_y = i32::from(reflector.y);
        effect.to_x = i32::from(caster.x);
        effect.to_y = i32::from(caster.y);
        effect.caster = Some(reflector.id);
        effect.caster_serial = reflector.id.0 as i32;
        effect.x = i32::from(reflector.x) * 1024 + 512;
        effect.y = i32::from(reflector.y) * 1024 + 512;
        self.effects.insert(effect_id, effect);
        effect_id
    }

    pub(crate) fn create_ball_effect(&mut self, caster: &Character) -> u32 {
        let effect_id = self.next_effect_id();
        let power = spell_power(
            character_value(caster, CharacterValue::Flash),
            character_value(caster, CharacterValue::Tactics),
        );
        let mut effect = Effect::new(
            EF_BALL,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(TICKS_PER_SECOND * 5) as i32,
        );
        effect.strength = power;
        effect.light = 80;
        effect.from_x = i32::from(caster.x);
        effect.from_y = i32::from(caster.y);
        effect.to_x = caster.act1;
        effect.to_y = caster.act2;
        effect.caster = Some(caster.id);
        effect.caster_serial = caster.id.0 as i32;
        effect.x = i32::from(caster.x) * 1024 + 512;
        effect.y = i32::from(caster.y) * 1024 + 512;
        self.effects.insert(effect_id, effect);
        effect_id
    }

    pub(crate) fn create_ball_trap_effect(
        &mut self,
        start_x: u16,
        start_y: u16,
        target_x: u16,
        target_y: u16,
        power: u8,
    ) -> u32 {
        let effect_id = self.next_effect_id();
        let mut effect = Effect::new(
            EF_BALL,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(TICKS_PER_SECOND * 5) as i32,
        );
        effect.strength = i32::from(power);
        effect.light = 80;
        effect.from_x = i32::from(start_x);
        effect.from_y = i32::from(start_y);
        effect.to_x = i32::from(target_x);
        effect.to_y = i32::from(target_y);
        effect.x = i32::from(start_x) * 1024 + 512;
        effect.y = i32::from(start_y) * 1024 + 512;
        self.effects.insert(effect_id, effect);
        effect_id
    }

    pub(crate) fn create_fireball_machine_effect(
        &mut self,
        start_x: u16,
        start_y: u16,
        target_x: u16,
        target_y: u16,
        power: u8,
    ) -> u32 {
        let effect_id = self.next_effect_id();
        let mut effect = Effect::new(
            EF_FIREBALL,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(TICKS_PER_SECOND) as i32,
        );
        effect.strength = i32::from(power);
        effect.light = 200;
        effect.from_x = i32::from(start_x);
        effect.from_y = i32::from(start_y);
        effect.to_x = i32::from(target_x);
        effect.to_y = i32::from(target_y);
        effect.x = i32::from(start_x) * 1024 + 512;
        effect.y = i32::from(start_y) * 1024 + 512;
        self.effects.insert(effect_id, effect);
        effect_id
    }

    pub(crate) fn create_edemonball_effect(
        &mut self,
        start_x: u16,
        start_y: u16,
        target_x: u16,
        target_y: u16,
        strength: i32,
        base_sprite: i32,
    ) -> u32 {
        let effect_id = self.next_effect_id();
        let mut effect = Effect::new(
            EF_EDEMONBALL,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(TICKS_PER_SECOND * 4) as i32,
        );
        effect.strength = strength;
        effect.from_x = i32::from(start_x);
        effect.from_y = i32::from(start_y);
        effect.to_x = i32::from(target_x);
        effect.to_y = i32::from(target_y);
        effect.x = i32::from(start_x) * 1024 + 512;
        effect.y = i32::from(start_y) * 1024 + 512;
        effect.base_sprite = base_sprite;
        self.effects.insert(effect_id, effect);
        effect_id
    }

    pub(crate) fn create_caligar_gun_effects(&mut self, item_id: ItemId, direction: u8) -> bool {
        let Some(item) = self.items.get(&item_id) else {
            return false;
        };
        let item_x = i32::from(item.x);
        let item_y = i32::from(item.y);
        let shots: &[(i32, i32, i32, i32)] = match direction {
            1 => &[(1, 0, 10, 0)],
            2 => &[(0, 1, 0, 10)],
            3 => &[(-1, 0, -10, 0)],
            4 => &[(0, -1, 0, -10)],
            5 => &[
                (0, 1, 0, 10),
                (1, 0, 10, 0),
                (0, -1, 0, -10),
                (-1, 0, -10, 0),
            ],
            _ => return false,
        };
        for (start_dx, start_dy, target_dx, target_dy) in shots {
            self.create_edemonball_effect(
                clamp_world_coordinate(item_x + start_dx),
                clamp_world_coordinate(item_y + start_dy),
                clamp_world_coordinate(item_x + target_dx),
                clamp_world_coordinate(item_y + target_dy),
                50,
                1,
            );
        }
        true
    }

    pub(crate) fn create_or_refresh_strike_effect(
        &mut self,
        target_id: CharacterId,
        x: i32,
        y: i32,
        strength: i32,
    ) -> u32 {
        let effect_id = self
            .effects
            .iter()
            .find_map(|(&effect_id, effect)| {
                (effect.effect_type == EF_STRIKE
                    && effect.target_character == Some(target_id)
                    && effect.x == x
                    && effect.y == y
                    && effect.strength == strength)
                    .then_some(effect_id)
            })
            .unwrap_or_else(|| {
                let effect_id = self.next_effect_id();
                let mut effect = Effect::new(
                    EF_STRIKE,
                    effect_id as i32,
                    self.tick.0 as i32,
                    self.tick.0.saturating_add(2) as i32,
                );
                effect.strength = strength;
                effect.light = 50;
                effect.x = x;
                effect.y = y;
                effect.target_character = Some(target_id);
                self.effects.insert(effect_id, effect);
                effect_id
            });

        if let Some(effect) = self.effects.get_mut(&effect_id) {
            effect.stop_tick = self.tick.0.saturating_add(2) as i32;
        }
        effect_id
    }

    pub(crate) fn create_pulse_effect(&mut self, x: u16, y: u16, strength: i32) -> u32 {
        let effect_id = self.next_effect_id();
        let mut effect = Effect::new(
            EF_PULSE,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(6) as i32,
        );
        effect.strength = strength;
        effect.x = i32::from(x);
        effect.y = i32::from(y);
        self.effects.insert(effect_id, effect);
        self.set_effect_on_map(effect_id, i32::from(x), i32::from(y));
        effect_id
    }

    pub(crate) fn create_pulseback_effect(
        &mut self,
        target_id: CharacterId,
        caster_x: u16,
        caster_y: u16,
        strength: i32,
    ) -> u32 {
        let effect_id = self.next_effect_id();
        let mut effect = Effect::new(
            EF_PULSEBACK,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(7) as i32,
        );
        effect.target_character = Some(target_id);
        effect.x = i32::from(caster_x);
        effect.y = i32::from(caster_y);
        effect.light = 20;
        effect.strength = strength;
        self.effects.insert(effect_id, effect);
        effect_id
    }

    pub fn create_explosion_effect(
        &mut self,
        x: i32,
        y: i32,
        max_age: u32,
        base_sprite: i32,
    ) -> u32 {
        let effect_id = self.next_effect_id();
        let mut effect = Effect::new(
            EF_EXPLODE,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(u64::from(max_age)) as i32,
        );
        effect.strength = max_age as i32;
        effect.light = 200;
        effect.base_sprite = base_sprite;
        self.effects.insert(effect_id, effect);
        self.set_effect_on_map(effect_id, x, y);
        effect_id
    }

    pub fn create_mist_effect(&mut self, x: i32, y: i32) -> u32 {
        self.create_map_effect(
            EF_MIST,
            x,
            y,
            self.tick.0 as i32,
            self.tick.0 as i32 + 24,
            0,
            0,
        )
    }

    pub fn create_map_effect(
        &mut self,
        effect_type: i32,
        x: i32,
        y: i32,
        start_tick: i32,
        stop_tick: i32,
        light: i32,
        strength: i32,
    ) -> u32 {
        let effect_id = self.next_effect_id();
        let mut effect = Effect::new(effect_type, effect_id as i32, start_tick, stop_tick);
        effect.light = light;
        effect.strength = strength;
        self.effects.insert(effect_id, effect);
        self.set_effect_on_map(effect_id, x, y);
        effect_id
    }

    pub fn create_bubble_effect(&mut self, x: i32, y: i32, y_offset: i32, duration: u32) -> u32 {
        self.create_map_effect(
            EF_BUBBLE,
            x,
            y,
            self.tick.0 as i32,
            self.tick.0.saturating_add(u64::from(duration)) as i32,
            0,
            y_offset,
        )
    }

    pub fn create_earthrain_effect(&mut self, x: i32, y: i32, strength: i32) -> u32 {
        self.create_area_map_effect(EF_EARTHRAIN, x, y, 10, strength)
    }

    pub fn create_earthmud_effect(&mut self, x: i32, y: i32, strength: i32) -> u32 {
        self.create_area_map_effect(EF_EARTHMUD, x, y, 0, strength)
    }

    pub(crate) fn create_area_map_effect(
        &mut self,
        effect_type: i32,
        x: i32,
        y: i32,
        light: i32,
        strength: i32,
    ) -> u32 {
        let effect_id = self.next_effect_id();
        let mut effect = Effect::new(
            effect_type,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(TICKS_PER_SECOND * 60) as i32,
        );
        effect.light = light;
        effect.strength = strength;
        self.effects.insert(effect_id, effect);

        self.add_area_effect_map_tile(effect_id, x, y, effect_type);
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let tx = x + dx;
                let ty = y + dy;
                if !self.map_tile_blocks_sight(tx, ty) {
                    self.add_area_effect_map_tile(effect_id, tx, ty, effect_type);
                }
            }
        }
        effect_id
    }

    pub(crate) fn add_area_effect_map_tile(
        &mut self,
        effect_id: u32,
        x: i32,
        y: i32,
        effect_type: i32,
    ) -> bool {
        let (Ok(x_usize), Ok(y_usize)) = (usize::try_from(x), usize::try_from(y)) else {
            return false;
        };
        if self.map.tile(x_usize, y_usize).is_some_and(|tile| {
            tile.effects.iter().any(|&slot| {
                slot != 0
                    && self
                        .effects
                        .get(&u32::from(slot))
                        .is_some_and(|effect| effect.effect_type == effect_type)
            })
        }) {
            return false;
        }
        self.set_effect_on_map(effect_id, x, y)
    }

    pub(crate) fn map_tile_blocks_sight(&self, x: i32, y: i32) -> bool {
        let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
            return true;
        };
        self.map.tile(x, y).is_none_or(|tile| {
            tile.flags
                .intersects(MapFlags::SIGHTBLOCK | MapFlags::TSIGHTBLOCK)
        })
    }

    pub(crate) fn create_show_effect(
        &mut self,
        effect_type: i32,
        target_id: CharacterId,
        start_tick: u32,
        stop_tick: u32,
        light: i32,
        strength: i32,
    ) -> u32 {
        let effect_id = self.next_effect_id();
        let mut effect = Effect::new(
            effect_type,
            effect_id as i32,
            start_tick as i32,
            stop_tick as i32,
        );
        effect.target_character = Some(target_id);
        effect.strength = strength;
        effect.light = light;
        if let Some(target) = self.characters.get(&target_id) {
            effect.x = i32::from(target.x);
            effect.y = i32::from(target.y);
        }
        self.effects.insert(effect_id, effect);
        effect_id
    }

    pub(crate) fn remove_show_effect_type(&mut self, target_id: CharacterId, effect_type: i32) {
        self.effects.retain(|_, effect| {
            !(effect.effect_type == effect_type && effect.target_character == Some(target_id))
        });
    }

    pub(crate) fn set_effect_on_map(&mut self, effect_id: u32, x: i32, y: i32) -> bool {
        if effect_id == 0 || effect_id > u32::from(u16::MAX) {
            return false;
        }
        let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
            return false;
        };
        if !self.map.legacy_inner_bounds(x, y) {
            return false;
        }
        let Some(effect) = self.effects.get_mut(&effect_id) else {
            return false;
        };
        let light = effect.light;
        if effect.fields.len() >= MAX_FIELD {
            return false;
        }
        let Some(tile) = self.map.tile_mut(x, y) else {
            return false;
        };
        let Some(slot) = tile.effects.iter_mut().find(|slot| **slot == 0) else {
            return false;
        };
        *slot = effect_id as u16;
        if let Some(index) = self.map.legacy_index(x, y) {
            effect.fields.push(index as i32);
        }
        add_effect_light(&mut self.map, x, y, light as i16);
        self.mark_light_area(x, y, light as i16);
        true
    }

    pub(crate) fn remove_effect_from_map(&mut self, effect_id: u32) {
        let Some(effect) = self.effects.get_mut(&effect_id) else {
            return;
        };
        let light = effect.light;
        let fields = std::mem::take(&mut effect.fields);
        for index in fields {
            if index < 0 {
                continue;
            }
            let index = index as usize;
            let x = index % self.map.width();
            let y = index / self.map.width();
            if let Some(tile) = self.map.tile_mut(x, y) {
                for slot in &mut tile.effects {
                    if *slot == effect_id as u16 {
                        *slot = 0;
                        break;
                    }
                }
            }
            remove_effect_light(&mut self.map, x, y, light as i16);
            self.mark_light_area(x, y, light as i16);
        }
    }

    pub(crate) fn create_palace_bomb_burn_effect(&mut self, character_id: CharacterId) -> bool {
        let effect_id = self.next_effect_id();
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let mut effect = Effect::new(
            EF_BURN,
            effect_id as i32,
            self.tick.0 as i32,
            self.tick.0.saturating_add(TICKS_PER_SECOND * 60) as i32,
        );
        effect.light = 250;
        effect.strength = POWERSCALE * 2;
        effect.target_character = Some(character_id);
        effect.x = i32::from(character.x);
        effect.y = i32::from(character.y);
        self.effects.insert(effect_id, effect);
        true
    }

    pub(crate) fn create_or_refresh_cap_effect(&mut self, character_id: CharacterId) {
        let stop_tick = self.tick.0.saturating_add(TICKS_PER_SECOND / 4 + 1) as i32;
        if let Some(effect) = self.effects.values_mut().find(|effect| {
            effect.effect_type == EF_CAP && effect.target_character == Some(character_id)
        }) {
            effect.stop_tick = stop_tick;
            return;
        }

        self.create_show_effect(
            EF_CAP,
            character_id,
            self.tick.0 as u32,
            stop_tick as u32,
            0,
            1,
        );
    }

    pub fn remove_character_burn_effect(&mut self, character_id: CharacterId) -> bool {
        let Some(effect_id) = self.effects.iter().find_map(|(&effect_id, effect)| {
            (effect.effect_type == EF_BURN && effect.target_character == Some(character_id))
                .then_some(effect_id)
        }) else {
            return false;
        };
        self.remove_effect_from_map(effect_id);
        self.effects.remove(&effect_id);
        true
    }
}
