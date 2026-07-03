use super::*;

pub(crate) const IID_REFLECT_FIREBALL: u32 = (0x01 << 24) | 0x00004E;

pub(crate) const IID_AREA6_GREENCRYSTAL: u32 = (0x01 << 24) | 0x000048;

impl World {
    pub(crate) fn find_edemonball_target_shot(
        &self,
        item_id: ItemId,
        strength: i32,
        base_sprite: i32,
    ) -> Option<ItemDriverOutcome> {
        let item = self.items.get(&item_id)?;
        let item_x = i32::from(item.x);
        let item_y = i32::from(item.y);

        let mut candidates: Vec<_> = self
            .characters
            .values()
            .filter(|character| {
                (i32::from(character.x) - item_x).abs() <= 10
                    && (i32::from(character.y) - item_y).abs() <= 10
            })
            .collect();
        candidates.sort_by_key(|character| character.id.0);

        for character in candidates {
            let (ox, oy) = if (i32::from(character.x) - item_x).abs()
                > (i32::from(character.y) - item_y).abs()
            {
                ((i32::from(character.x) - item_x).signum(), 0)
            } else {
                (0, (i32::from(character.y) - item_y).signum())
            };
            let (target_x, target_y) = self.predict_edemonball_target(item, character);
            let start_x = item_x + ox;
            let start_y = item_y + oy;
            if self.edemonball_can_hit(item_id, character.id, start_x, start_y, target_x, target_y)
            {
                return Some(ItemDriverOutcome::EdemonBallProjectile {
                    item_id,
                    character_id: CharacterId(0),
                    start_x: start_x.clamp(0, i32::from(u16::MAX)) as u16,
                    start_y: start_y.clamp(0, i32::from(u16::MAX)) as u16,
                    target_x: target_x.clamp(0, i32::from(u16::MAX)) as u16,
                    target_y: target_y.clamp(0, i32::from(u16::MAX)) as u16,
                    strength,
                    base_sprite,
                    schedule_after_ticks: TICKS_PER_SECOND * 8,
                });
            }
        }

        None
    }

    pub(crate) fn predict_edemonball_target(
        &self,
        item: &Item,
        character: &Character,
    ) -> (i32, i32) {
        if character.action != action::WALK {
            return (i32::from(character.x), i32::from(character.y));
        }

        let Ok(direction) = Direction::try_from(character.dir) else {
            return (i32::from(character.x), i32::from(character.y));
        };
        let (dx, dy) = direction.delta();
        let dist = map_dist(item.x, item.y, character.x, character.y);
        let mut eta = dist * 3 / 2;
        eta -= character.duration - character.step;
        if eta <= 0 {
            return (i32::from(character.tox), i32::from(character.toy));
        }

        for step in 1..10 {
            eta -= character.duration;
            if eta <= 0 {
                return (
                    i32::from(character.x) + i32::from(dx) * step,
                    i32::from(character.y) + i32::from(dy) * step,
                );
            }
        }

        (i32::from(character.x), i32::from(character.y))
    }

    pub(crate) fn edemonball_can_hit(
        &self,
        item_id: ItemId,
        target_id: CharacterId,
        from_x: i32,
        from_y: i32,
        target_x: i32,
        target_y: i32,
    ) -> bool {
        let mut x = from_x * 1024 + 512;
        let mut y = from_y * 1024 + 512;
        let mut dx = target_x - from_x;
        let mut dy = target_y - from_y;

        if dx.abs() < 2 && dy.abs() < 2 {
            return false;
        }

        if dx.abs() > dy.abs() {
            dy = dy * 256 / dx.abs();
            dx = dx * 256 / dx.abs();
        } else {
            dx = dx * 256 / dy.abs();
            dy = dy * 256 / dy.abs();
        }

        for _ in 0..48 {
            x += dx;
            y += dy;
            let tile_x = x / 1024;
            let tile_y = y / 1024;
            if tile_x == target_x && tile_y == target_y {
                return true;
            }

            let (Ok(tile_x_usize), Ok(tile_y_usize)) =
                (usize::try_from(tile_x), usize::try_from(tile_y))
            else {
                return false;
            };
            let Some(tile) = self.map.tile(tile_x_usize, tile_y_usize) else {
                return false;
            };
            let item_blocks = tile.item != 0
                && tile.item != item_id.0
                && tile.flags.contains(MapFlags::TMOVEBLOCK);
            let map_blocks = !tile.flags.contains(MapFlags::FIRETHRU)
                && tile.flags.contains(MapFlags::MOVEBLOCK);
            let blocked = tile.character != 0 || item_blocks || map_blocks;
            if blocked {
                return tile.character == target_id.0 as u16;
            }
        }

        true
    }

    pub fn tick_effects(&mut self) {
        self.tick_effects_with_attack_policy(|_caster_id, caster, target, map| {
            can_attack(caster, target, map)
        });
    }

    pub fn tick_effects_with_attack_policy(
        &mut self,
        mut can_effect_attack: impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) {
        let mut state = self.tick.0.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        self.tick_effects_with_random_and_attack_policy(
            |limit| {
                if limit <= 0 {
                    return 0;
                }
                state = state.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                (state % limit as u64) as i32
            },
            &mut can_effect_attack,
        );
    }

    pub fn tick_effects_with_random(&mut self, mut random_below: impl FnMut(i32) -> i32) {
        self.tick_effects_with_random_and_attack_policy(
            &mut random_below,
            |_, caster, target, map| can_attack(caster, target, map),
        );
    }

    pub fn tick_effects_with_random_and_attack_policy(
        &mut self,
        mut random_below: impl FnMut(i32) -> i32,
        mut can_effect_attack: impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) {
        let effect_ids: Vec<u32> = self.effects.keys().copied().collect();
        for effect_id in effect_ids {
            match self
                .effects
                .get(&effect_id)
                .map(|effect| effect.effect_type)
            {
                Some(EF_FIREBALL) => self.tick_fireball_effect(effect_id, &mut can_effect_attack),
                Some(EF_BALL) => self.tick_ball_effect(effect_id, &mut can_effect_attack),
                Some(EF_EDEMONBALL) => self.tick_edemonball_effect(effect_id),
                Some(EF_STRIKE | EF_PULSE) => self.tick_strike_effect(effect_id),
                Some(EF_BURN) => self.tick_burn_effect(effect_id),
                Some(EF_EARTHRAIN) => self.tick_earthrain_effect(effect_id, &mut random_below),
                Some(_) => self.tick_expiring_effect(effect_id),
                _ => {}
            }
        }
    }

    pub(crate) fn tick_earthrain_effect(
        &mut self,
        effect_id: u32,
        random_below: &mut impl FnMut(i32) -> i32,
    ) {
        let Some(effect) = self.effects.get(&effect_id).cloned() else {
            return;
        };

        let mut targets = Vec::new();
        for index in &effect.fields {
            if *index < 0 {
                continue;
            }
            let index = *index as usize;
            let x = index % self.map.width();
            let y = index / self.map.width();
            let Some(target_id) = self.map.tile(x, y).and_then(|tile| {
                (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
            }) else {
                continue;
            };
            let Some(target) = self.characters.get(&target_id) else {
                continue;
            };
            if !target.flags.contains(CharacterFlags::PLAYER) {
                continue;
            }
            let reduction =
                (effect.strength - character_value(target, CharacterValue::Demon)).max(0);
            let damage = reduction * 150;
            if damage == 0 || random_below(10) != 0 {
                continue;
            }
            let armor_percent = 50 - reduction.min(50);
            targets.push((target_id, damage, armor_percent));
        }

        for (target_id, damage, armor_percent) in targets {
            self.apply_legacy_hurt(
                target_id,
                None,
                damage,
                8,
                armor_percent,
                armor_percent + 25,
            );
        }

        self.tick_expiring_effect(effect_id);
    }

    pub(crate) fn tick_expiring_effect(&mut self, effect_id: u32) {
        if self
            .effects
            .get(&effect_id)
            .is_some_and(|effect| self.tick.0 >= effect.stop_tick as u64)
        {
            self.remove_effect_from_map(effect_id);
            self.effects.remove(&effect_id);
        }
    }

    pub(crate) fn tick_burn_effect(&mut self, effect_id: u32) {
        let Some(effect) = self.effects.get(&effect_id).cloned() else {
            return;
        };
        let Some(target_id) = effect.target_character else {
            self.effects.remove(&effect_id);
            return;
        };
        if self.tick.0 >= effect.stop_tick as u64
            || !self
                .characters
                .get(&target_id)
                .is_some_and(|character| character.flags.contains(CharacterFlags::USED))
        {
            self.effects.remove(&effect_id);
            return;
        }

        if effect.strength != 0 {
            self.apply_legacy_hurt(
                target_id,
                None,
                POWERSCALE / 6 + effect.strength,
                30,
                50,
                75,
            );
        }
    }

    pub(crate) fn tick_strike_effect(&mut self, effect_id: u32) {
        if self
            .effects
            .get(&effect_id)
            .is_some_and(|effect| self.tick.0 >= effect.stop_tick as u64)
        {
            self.remove_effect_from_map(effect_id);
            self.effects.remove(&effect_id);
        }
    }

    pub(crate) fn tick_ball_effect(
        &mut self,
        effect_id: u32,
        can_effect_attack: &mut impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) {
        let Some(effect) = self.effects.get(&effect_id).cloned() else {
            return;
        };

        if effect.caster.is_some_and(|caster_id| {
            !self
                .characters
                .get(&caster_id)
                .is_some_and(|caster| caster.flags.contains(CharacterFlags::USED))
        }) || self.tick.0 >= effect.stop_tick as u64
        {
            self.remove_effect_from_map(effect_id);
            self.effects.remove(&effect_id);
            return;
        }

        let old_x = effect.x / 1024;
        let old_y = effect.y / 1024;
        let raw_dx = effect.to_x - effect.from_x;
        let raw_dy = effect.to_y - effect.from_y;
        if raw_dx == 0 && raw_dy == 0 {
            self.remove_effect_from_map(effect_id);
            self.effects.remove(&effect_id);
            return;
        }

        let (step_x, step_y) = if raw_dx.abs() > raw_dy.abs() {
            (raw_dx * 128 / raw_dx.abs(), raw_dy * 128 / raw_dx.abs())
        } else {
            (raw_dx * 128 / raw_dy.abs(), raw_dy * 128 / raw_dy.abs())
        };
        let x = effect.x + step_x;
        let y = effect.y + step_y;
        let tile_x = x / 1024;
        let tile_y = y / 1024;

        if self.fire_map_blocked(tile_x, tile_y)
            && !effect
                .caster
                .and_then(|caster_id| self.characters.get(&caster_id))
                .is_some_and(|caster| {
                    (i32::from(caster.x), i32::from(caster.y)) == (tile_x, tile_y)
                })
        {
            self.remove_effect_from_map(effect_id);
            self.effects.remove(&effect_id);
            return;
        }

        if let Some(effect) = self.effects.get_mut(&effect_id) {
            effect.x = x;
            effect.y = y;
            effect.last_x = old_x;
            effect.last_y = old_y;
        }
        if old_x != tile_x || old_y != tile_y {
            self.remove_effect_from_map(effect_id);
            self.set_effect_on_map(effect_id, tile_x, tile_y);
        }
        self.apply_ball_strikes(effect_id, tile_x, tile_y, can_effect_attack);
    }

    pub(crate) fn apply_ball_strikes(
        &mut self,
        effect_id: u32,
        x: i32,
        y: i32,
        can_effect_attack: &mut impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) {
        let Some(effect) = self.effects.get(&effect_id).cloned() else {
            return;
        };
        let Some(caster_id) = effect.caster else {
            return;
        };
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return;
        };

        let mut targets = Vec::new();
        let min_x = (x - 5).max(1);
        let min_y = (y - 5).max(1);
        let max_x = (x + 5).min(self.map.width().saturating_sub(2) as i32);
        let max_y = (y + 5).min(self.map.height().saturating_sub(2) as i32);
        for target_y in min_y..max_y {
            for target_x in min_x..max_x {
                let (Ok(target_x_usize), Ok(target_y_usize)) =
                    (usize::try_from(target_x), usize::try_from(target_y))
                else {
                    continue;
                };
                let Some(target_id) =
                    self.map
                        .tile(target_x_usize, target_y_usize)
                        .and_then(|tile| {
                            (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
                        })
                else {
                    continue;
                };
                if target_id == caster_id {
                    continue;
                }
                let Some(target) = self.characters.get(&target_id) else {
                    continue;
                };
                if !can_effect_attack(caster_id, &caster, target, &self.map) {
                    continue;
                }
                let (Ok(ball_x), Ok(ball_y)) = (usize::try_from(x), usize::try_from(y)) else {
                    continue;
                };
                if !self
                    .map
                    .can_see(ball_x, ball_y, target_x_usize, target_y_usize, 5)
                {
                    continue;
                }
                if self.tick.0 & 3 == 0 {
                    let has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;
                    let damage = strike_damage(
                        effect.strength,
                        character_value(target, CharacterValue::Immunity),
                        character_value(target, CharacterValue::Tactics),
                        has_tactics,
                    ) * ball_target_damage_multiplier(effect.number_of_enemies)
                        / (25 * TICKS_PER_SECOND as i32 * 2);
                    targets.push((target_id, damage));
                } else {
                    targets.push((target_id, 0));
                }
            }
        }

        if let Some(effect) = self.effects.get_mut(&effect_id) {
            effect.number_of_enemies = targets.len() as i32;
        }
        if !targets.is_empty() && self.tick.0 & 7 == 0 {
            self.queue_sound_area(usize::from(caster.x), usize::from(caster.y), 30);
        }
        for (target_id, damage) in targets {
            self.create_or_refresh_strike_effect(target_id, x, y, effect.strength);
            if damage == 0 {
                continue;
            }
            self.apply_legacy_hurt(target_id, Some(caster_id), damage, 100, 30, 85);
        }
    }

    pub(crate) fn tick_fireball_effect(
        &mut self,
        effect_id: u32,
        can_effect_attack: &mut impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) {
        let Some(effect) = self.effects.get(&effect_id).cloned() else {
            return;
        };

        if effect.caster.is_some_and(|caster_id| {
            !self
                .characters
                .get(&caster_id)
                .is_some_and(|caster| caster.flags.contains(CharacterFlags::USED))
        }) || self.tick.0 >= effect.stop_tick as u64
        {
            self.remove_effect_from_map(effect_id);
            self.effects.remove(&effect_id);
            return;
        }

        self.remove_effect_from_map(effect_id);

        let raw_dx = effect.to_x - effect.from_x;
        let raw_dy = effect.to_y - effect.from_y;
        if raw_dx == 0 && raw_dy == 0 {
            self.explode_fireball_effect(
                effect_id,
                effect.x / 1024,
                effect.y / 1024,
                can_effect_attack,
            );
            return;
        }

        let (step_x, step_y) = if raw_dx.abs() > raw_dy.abs() {
            (raw_dx * 512 / raw_dx.abs(), raw_dy * 512 / raw_dx.abs())
        } else {
            (raw_dx * 512 / raw_dy.abs(), raw_dy * 512 / raw_dy.abs())
        };

        let mut x = effect.x;
        let mut y = effect.y;
        let mut last_x = effect.last_x;
        let mut last_y = effect.last_y;
        for _ in 0..2 {
            last_x = x / 1024;
            last_y = y / 1024;
            x += step_x;
            y += step_y;

            let tile_x = x / 1024;
            let tile_y = y / 1024;
            if self.fire_map_blocked(tile_x, tile_y)
                && !self.fire_tile_contains_caster(effect.caster, tile_x, tile_y)
            {
                if let Some(effect) = self.effects.get_mut(&effect_id) {
                    effect.x = x;
                    effect.y = y;
                    effect.last_x = last_x;
                    effect.last_y = last_y;
                }
                self.explode_fireball_effect(effect_id, tile_x, tile_y, can_effect_attack);
                return;
            }
        }

        if let Some(effect) = self.effects.get_mut(&effect_id) {
            effect.x = x;
            effect.y = y;
            effect.last_x = last_x;
            effect.last_y = last_y;
        }
        self.set_effect_on_map(effect_id, x / 1024, y / 1024);
    }

    pub(crate) fn tick_edemonball_effect(&mut self, effect_id: u32) {
        let Some(effect) = self.effects.get(&effect_id).cloned() else {
            return;
        };

        if effect.caster.is_some_and(|caster_id| {
            !self
                .characters
                .get(&caster_id)
                .is_some_and(|caster| caster.flags.contains(CharacterFlags::USED))
        }) || self.tick.0 >= effect.stop_tick as u64
        {
            self.remove_effect_from_map(effect_id);
            self.effects.remove(&effect_id);
            return;
        }

        self.remove_effect_from_map(effect_id);

        let raw_dx = effect.to_x - effect.from_x;
        let raw_dy = effect.to_y - effect.from_y;
        if raw_dx == 0 && raw_dy == 0 {
            self.explode_edemonball_effect(effect_id, effect.x / 1024, effect.y / 1024);
            return;
        }

        let (step_x, step_y) = if raw_dx.abs() > raw_dy.abs() {
            (raw_dx * 256 / raw_dx.abs(), raw_dy * 256 / raw_dx.abs())
        } else {
            (raw_dx * 256 / raw_dy.abs(), raw_dy * 256 / raw_dy.abs())
        };

        let last_x = effect.x / 1024;
        let last_y = effect.y / 1024;
        let x = effect.x + step_x;
        let y = effect.y + step_y;
        let tile_x = x / 1024;
        let tile_y = y / 1024;

        if self.edemonball_map_blocked(tile_x, tile_y)
            && !self.fire_tile_contains_caster(effect.caster, tile_x, tile_y)
        {
            if let Some(effect) = self.effects.get_mut(&effect_id) {
                effect.x = x;
                effect.y = y;
                effect.last_x = last_x;
                effect.last_y = last_y;
            }
            let has_character = self
                .map
                .tile(
                    usize::try_from(tile_x).unwrap_or_default(),
                    usize::try_from(tile_y).unwrap_or_default(),
                )
                .is_some_and(|tile| tile.character != 0);
            let (explode_x, explode_y) = if has_character {
                (tile_x, tile_y)
            } else {
                (last_x, last_y)
            };
            self.explode_edemonball_effect(effect_id, explode_x, explode_y);
            return;
        }

        if let Some(effect) = self.effects.get_mut(&effect_id) {
            effect.x = x;
            effect.y = y;
            effect.last_x = last_x;
            effect.last_y = last_y;
        }
        self.set_effect_on_map(effect_id, tile_x, tile_y);
    }

    pub(crate) fn fire_map_blocked(&self, x: i32, y: i32) -> bool {
        let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
            return true;
        };
        let Some(tile) = self.map.tile(x, y) else {
            return true;
        };
        tile.flags.contains(MapFlags::TMOVEBLOCK)
            || (!tile.flags.contains(MapFlags::FIRETHRU)
                && tile.flags.contains(MapFlags::MOVEBLOCK))
    }

    pub(crate) fn edemonball_map_blocked(&self, x: i32, y: i32) -> bool {
        let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
            return true;
        };
        let Some(tile) = self.map.tile(x, y) else {
            return true;
        };
        tile.character != 0
            || (tile.item != 0 && tile.flags.contains(MapFlags::TMOVEBLOCK))
            || (!tile.flags.contains(MapFlags::FIRETHRU)
                && tile.flags.contains(MapFlags::MOVEBLOCK))
    }

    pub(crate) fn fire_tile_contains_caster(
        &self,
        caster_id: Option<CharacterId>,
        x: i32,
        y: i32,
    ) -> bool {
        let Some(caster_id) = caster_id else {
            return false;
        };
        let Some(caster) = self.characters.get(&caster_id) else {
            return false;
        };
        (i32::from(caster.x), i32::from(caster.y)) == (x, y)
            || (i32::from(caster.tox), i32::from(caster.toy)) == (x, y)
    }

    pub(crate) fn explode_fireball_effect(
        &mut self,
        effect_id: u32,
        x: i32,
        y: i32,
        can_effect_attack: &mut impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) {
        let Some(effect) = self.effects.get(&effect_id).cloned() else {
            return;
        };
        self.remove_effect_from_map(effect_id);
        self.effects.remove(&effect_id);

        let Some(caster_id) = effect.caster else {
            return;
        };
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return;
        };

        let mut targets = Vec::new();
        for dy in -1..=1 {
            for dx in -1..=1 {
                let target_x = x + dx;
                let target_y = y + dy;
                let (Ok(target_x_usize), Ok(target_y_usize)) =
                    (usize::try_from(target_x), usize::try_from(target_y))
                else {
                    continue;
                };
                if dx != 0 || dy != 0 {
                    let (Ok(last_x), Ok(last_y)) = (
                        usize::try_from(effect.last_x),
                        usize::try_from(effect.last_y),
                    ) else {
                        continue;
                    };
                    if !self
                        .map
                        .can_see(last_x, last_y, target_x_usize, target_y_usize, 5)
                    {
                        continue;
                    }
                }
                let Some(target_id) =
                    self.map
                        .tile(target_x_usize, target_y_usize)
                        .and_then(|tile| {
                            (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
                        })
                else {
                    continue;
                };
                if target_id == caster_id {
                    continue;
                }
                let Some(target) = self.characters.get(&target_id) else {
                    continue;
                };
                if !can_effect_attack(caster_id, &caster, target, &self.map) {
                    return;
                }
                let target = target.clone();
                if self.reflect_fireball_from_target(&target, &caster, effect.strength) {
                    return;
                }
                if target.flags.contains(CharacterFlags::EDEMON) {
                    self.create_reflected_fireball_effect(&target, &caster, effect.strength - 1);
                }
                let has_tactics = character_value_present(&target, CharacterValue::Tactics) != 0;
                let damage = fireball_damage(
                    effect.strength,
                    character_value(&target, CharacterValue::Immunity),
                    character_value(&target, CharacterValue::Tactics),
                    has_tactics,
                );
                targets.push((target_id, damage));
            }
        }

        for (target_id, damage) in targets {
            self.apply_legacy_hurt(target_id, Some(caster_id), damage, 10, 50, 70);
        }

        self.create_explosion_effect(x, y, 8, 50050);
        self.queue_sound_area(x as usize, y as usize, 6);
    }

    pub(crate) fn explode_edemonball_effect(&mut self, effect_id: u32, x: i32, y: i32) {
        let Some(effect) = self.effects.get(&effect_id).cloned() else {
            return;
        };
        self.remove_effect_from_map(effect_id);
        self.effects.remove(&effect_id);

        if x < 1 || x >= self.map.width().saturating_sub(1) as i32 {
            return;
        }
        if y < 1 || y >= self.map.height().saturating_sub(1) as i32 {
            return;
        }

        if let Some(target_id) = self.map.tile(x as usize, y as usize).and_then(|tile| {
            (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
        }) {
            let may_damage = self.characters.get(&target_id).is_some_and(|target| {
                if effect.base_sprite == 2
                    && target
                        .flags
                        .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
                {
                    return false;
                }
                match effect.caster {
                    Some(caster_id) => self
                        .characters
                        .get(&caster_id)
                        .is_some_and(|caster| can_attack(caster, target, &self.map)),
                    None => true,
                }
            });
            if may_damage {
                let strength = if effect.base_sprite == 0 {
                    self.absorb_edemonball_with_green_crystal(target_id, effect.strength)
                } else {
                    effect.strength
                };
                let damage = strength.saturating_mul(POWERSCALE);
                self.apply_legacy_hurt(target_id, effect.caster, damage, 6, 75, 50);
            }
        }

        self.create_explosion_effect(x, y, 8, 50450 + effect.base_sprite);
    }

    pub(crate) fn absorb_edemonball_with_green_crystal(
        &mut self,
        target_id: CharacterId,
        mut strength: i32,
    ) -> i32 {
        let Some(target) = self.characters.get(&target_id) else {
            return strength;
        };
        let mut candidates = Vec::with_capacity(INVENTORY_SIZE - 29);
        if let Some(item_id) = target.cursor_item {
            candidates.push(item_id);
        }
        candidates.extend(target.inventory[30..].iter().filter_map(|&item_id| item_id));

        for item_id in candidates {
            let Some(item) = self.items.get(&item_id) else {
                continue;
            };
            if item.template_id != IID_AREA6_GREENCRYSTAL {
                continue;
            }
            let crystal_power = item.driver_data.first().copied().unwrap_or_default() as i32;
            if strength > crystal_power {
                strength -= crystal_power;
                self.destroy_item(item_id);
                continue;
            }

            let mut sprite_changed = false;
            if let Some(item) = self.items.get_mut(&item_id) {
                item.driver_data.resize(1, 0);
                item.driver_data[0] = (crystal_power - strength).clamp(0, u8::MAX as i32) as u8;
                let sprite = 50318 + 5 - (i32::from(item.driver_data[0]) / 42);
                if item.sprite != sprite {
                    item.sprite = sprite;
                    sprite_changed = true;
                }
            }
            if sprite_changed {
                if let Some(target) = self.characters.get_mut(&target_id) {
                    target.flags.insert(CharacterFlags::ITEMS);
                }
            }
            return 0;
        }

        strength
    }

    pub(crate) fn reflect_fireball_from_target(
        &mut self,
        target: &Character,
        caster: &Character,
        strength: i32,
    ) -> bool {
        let Some((slot, item_id, charges)) = LEGACY_EQUIPMENT_SLOTS.clone().find_map(|slot| {
            let item_id = *target.inventory.get(slot)?.as_ref()?;
            let item = self.items.get(&item_id)?;
            (item.template_id == IID_REFLECT_FIREBALL)
                .then(|| (slot, item_id, read_u32_le_prefix(&item.driver_data)))
        }) else {
            return false;
        };

        let used_charges = strength.max(0) as u32;
        if charges <= used_charges {
            if let Some(target) = self.characters.get_mut(&target.id) {
                if target.inventory.get(slot) == Some(&Some(item_id)) {
                    target.inventory[slot] = None;
                }
            }
            self.items.remove(&item_id);
        } else if let Some(item) = self.items.get_mut(&item_id) {
            let remaining = charges - used_charges;
            write_u32_le_prefix(&mut item.driver_data, remaining);
            item.description = format!("{remaining} units left.");
            item.flags.insert(ItemFlags::FORCEUPDATE);
        }

        self.create_reflected_fireball_effect(target, caster, strength - 1);
        true
    }
}
