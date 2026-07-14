//! Simple-baddy self-support setup: earthmud, heal, magicshield, bless,
//! regeneration, freeze, ball, flash and warcry actions.

use super::*;

impl World {
    pub(crate) fn setup_simple_baddy_earthmud_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let max_hp = character_value(&attacker, CharacterValue::Hp) * POWERSCALE;
        let strength = character_value_present(&attacker, CharacterValue::Demon);
        if !attacker.flags.contains(CharacterFlags::EDEMON)
            || strength != 30
            || attacker.hp < max_hp / 2
            || self.simple_baddy_earthmud_value(target) == 0
        {
            return false;
        }

        let (target_x, target_y) = simple_baddy_earth_spell_target(target);
        let weather_movement_percent = self.settings.weather_movement_percent;
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if do_earthmud(
            character,
            &self.map,
            target_x,
            target_y,
            strength,
            weather_movement_percent,
        )
        .is_err()
        {
            return false;
        }
        if let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() {
            data.lastfight = self.tick.0 as i32;
        }
        true
    }

    pub(crate) fn simple_baddy_earthmud_value(&self, target: &Character) -> i32 {
        let (target_x, target_y) = simple_baddy_earth_spell_target(target);
        let mut good = 0;
        for (x, y) in [
            (target_x, target_y),
            (target_x.saturating_add(1), target_y),
            (target_x.saturating_sub(1), target_y),
            (target_x, target_y.saturating_add(1)),
            (target_x, target_y.saturating_sub(1)),
        ] {
            if self.simple_baddy_can_place_earthmud(x, y) {
                good += 1;
            }
        }

        if good > 0 {
            good
        } else {
            0
        }
    }

    pub(crate) fn simple_baddy_can_place_earthmud(&self, x: usize, y: usize) -> bool {
        self.map.tile(x, y).is_some_and(|tile| {
            !tile
                .flags
                .intersects(MapFlags::SIGHTBLOCK | MapFlags::TSIGHTBLOCK)
                && tile.effects.iter().all(|&effect_id| {
                    effect_id == 0
                        || self
                            .effects
                            .get(&u32::from(effect_id))
                            .is_none_or(|effect| effect.effect_type != EF_EARTHMUD)
                })
        })
    }

    pub(crate) fn simple_baddy_can_heal_self(&self, character: &Character) -> bool {
        character_value(character, CharacterValue::Heal) > 1
            && character.mana >= POWERSCALE * 2
            && character.hp < character_value(character, CharacterValue::Hp) * POWERSCALE / 2
    }

    pub(crate) fn setup_simple_baddy_heal_action(&mut self, character_id: CharacterId) -> bool {
        let Some(target) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if !self.simple_baddy_can_heal_self(&target) {
            return false;
        }
        self.setup_simple_baddy_spell_action(
            character_id,
            |character, _items, _tick, map, weather_movement_percent| {
                do_heal(character, &target, None, map, weather_movement_percent)
            },
        )
    }

    pub(crate) fn simple_baddy_can_magicshield_self(&self, character: &Character) -> bool {
        character_value(character, CharacterValue::MagicShield) > 1
            && character.mana >= POWERSCALE * 2
            && character.lifeshield
                < character_value(character, CharacterValue::MagicShield) * POWERSCALE / 2
    }

    pub(crate) fn setup_simple_baddy_magicshield_action(
        &mut self,
        character_id: CharacterId,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if !self.simple_baddy_can_magicshield_self(&character) {
            return false;
        }
        self.setup_simple_baddy_spell_action(
            character_id,
            |character, _items, _tick, map, weather_movement_percent| {
                do_magicshield(character, map, weather_movement_percent)
            },
        )
    }

    pub(crate) fn simple_baddy_can_bless_self(&self, character: &Character) -> bool {
        character_value(character, CharacterValue::Bless) > 1
            && character.mana >= BLESS_COST
            && may_add_spell(character, &self.items, IDR_BLESS, self.tick.0 as u32).is_some()
    }

    pub(crate) fn setup_simple_baddy_self_bless_action(
        &mut self,
        character_id: CharacterId,
    ) -> bool {
        let Some(target) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if !self.simple_baddy_can_bless_self(&target) {
            return false;
        }
        self.setup_simple_baddy_spell_action(
            character_id,
            |character, items, tick, map, weather_movement_percent| {
                do_bless(
                    character,
                    &target,
                    items,
                    tick,
                    None,
                    map,
                    weather_movement_percent,
                )
            },
        )
    }

    pub(crate) fn simple_baddy_needs_regeneration(&self, character: &Character) -> bool {
        character.mana < character_value(character, CharacterValue::Mana) * POWERSCALE
            || character.hp < character_value(character, CharacterValue::Hp) * POWERSCALE
    }

    pub(crate) fn setup_simple_baddy_regenerate_action(
        &mut self,
        character_id: CharacterId,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if !self.simple_baddy_needs_regeneration(&character) {
            return false;
        }
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if do_idle(character, (TICKS_PER_SECOND / 2) as i32).is_err() {
            return false;
        }
        if let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() {
            data.lastfight = self.tick.0 as i32;
        }
        true
    }

    pub(crate) fn simple_baddy_regenerate_task_value(&self, character: &Character) -> i32 {
        let base = character_value(character, CharacterValue::Fireball)
            .max(character_value(character, CharacterValue::Flash))
            .max(character_value(character, CharacterValue::Freeze))
            .max(character_value(character, CharacterValue::Attack))
            * 2;
        let last_hit = character
            .fight_driver
            .as_ref()
            .map(|data| data.last_hit)
            .unwrap_or(0);
        let tick = self.tick.0 as i32;
        let regen_time = TICKS_PER_SECOND as i32;
        let regen_diff = character.regen_ticker as i32 + regen_time - tick;
        if regen_diff <= 0 {
            return base + FIGHT_DRIVER_HIGH_PRIO;
        }
        let hit_diff = last_hit + regen_time * 2 - tick;
        if hit_diff <= 0 {
            return base + FIGHT_DRIVER_LOW_PRIO;
        }
        (base * regen_time * 2 - base * hit_diff) / (regen_time * 2) + FIGHT_DRIVER_LOW_PRIO
    }

    pub(crate) fn simple_baddy_freeze_modifier(
        &self,
        attacker: &Character,
        target: &Character,
    ) -> i32 {
        freeze_speed_modifier(
            spell_power(
                character_value(attacker, CharacterValue::Freeze),
                character_value(attacker, CharacterValue::Tactics),
            ),
            character_value(target, CharacterValue::Immunity),
            character_value(target, CharacterValue::Tactics),
            character_value_present(target, CharacterValue::Tactics) != 0,
            attacker.flags.contains(CharacterFlags::IDEMON),
            // C: freeze_value (tool.c) reads the caster's V_DEMON from value[1]
            // (the base/present value, not the sunlight/combat-reducible current
            // value[0]).
            character_value_present(attacker, CharacterValue::Demon),
            character_value(target, CharacterValue::Cold),
        )
    }

    pub(crate) fn setup_simple_baddy_freeze_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if character_value(&attacker, CharacterValue::Freeze) <= 1
            || attacker.mana < FREEZE_COST
            || tile_char_dist(&attacker, target) >= 4
            || may_add_spell(target, &self.items, IDR_FREEZE, self.tick.0 as u32).is_none()
            || self.simple_baddy_freeze_modifier(&attacker, target) >= -10
        {
            return false;
        }
        self.setup_simple_baddy_spell_action(
            character_id,
            |character, _items, _tick, map, weather_movement_percent| {
                do_freeze(character, map, weather_movement_percent)
            },
        )
    }

    pub(crate) fn setup_simple_baddy_ball_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        random: &mut impl FnMut(u32) -> u32,
    ) -> bool {
        let target_x = usize::from(target.x).saturating_sub(1)
            + usize::try_from(random(3).min(2)).unwrap_or(0);
        let target_y = usize::from(target.y).saturating_sub(1)
            + usize::try_from(random(3).min(2)).unwrap_or(0);
        self.setup_simple_baddy_spell_action(
            character_id,
            |character, items, tick, map, weather_movement_percent| {
                do_ball(
                    character,
                    items,
                    target_x,
                    target_y,
                    tick,
                    map,
                    weather_movement_percent,
                )
            },
        )
    }

    pub(crate) fn simple_baddy_calc_ball_steps(
        &self,
        caster_id: CharacterId,
        from_x: usize,
        from_y: usize,
        target_x: usize,
        target_y: usize,
    ) -> i32 {
        let mut dx = target_x as i32 - from_x as i32;
        let mut dy = target_y as i32 - from_y as i32;
        if dx == 0 && dy == 0 {
            return 0;
        }

        let mut x = from_x as i32 * 1024 + 512;
        let mut y = from_y as i32 * 1024 + 512;
        if dx.abs() > dy.abs() {
            dy = dy * 512 / dx.abs();
            dx = dx * 512 / dx.abs();
        } else {
            dx = dx * 512 / dy.abs();
            dy = dy * 512 / dy.abs();
        }

        let max_steps = (TICKS_PER_SECOND * 5 / 4) as i32;
        for step in 0..max_steps {
            x += dx;
            y += dy;
            let tile_x = x / 1024;
            let tile_y = y / 1024;
            if self.ball_path_blocked_for_caster(tile_x, tile_y, caster_id) {
                return step;
            }
        }
        max_steps
    }

    pub(crate) fn ball_path_blocked_for_caster(
        &self,
        x: i32,
        y: i32,
        caster_id: CharacterId,
    ) -> bool {
        let (Ok(x), Ok(y)) = (usize::try_from(x), usize::try_from(y)) else {
            return true;
        };
        let Some(tile) = self.map.tile(x, y) else {
            return true;
        };
        let map_blocks = tile.flags.contains(MapFlags::TMOVEBLOCK)
            || (!tile.flags.contains(MapFlags::FIRETHRU)
                && tile.flags.contains(MapFlags::MOVEBLOCK));
        map_blocks && tile.character != caster_id.0 as u16
    }

    pub(crate) fn setup_simple_baddy_flash_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if tile_char_dist(&attacker, target) >= 4
            || may_add_spell(&attacker, &self.items, IDR_FLASH, self.tick.0 as u32).is_none()
        {
            return false;
        }
        self.setup_simple_baddy_spell_action(
            character_id,
            |character, items, tick, map, weather_movement_percent| {
                do_flash(character, items, tick, map, weather_movement_percent)
            },
        )
    }

    pub(crate) fn simple_baddy_can_warcry(&self, attacker: &Character, target: &Character) -> bool {
        if character_value(attacker, CharacterValue::Warcry) <= 1
            || attacker.endurance
                <= character_value(attacker, CharacterValue::Warcry) * POWERSCALE / 3
            || char_dist(attacker, target) >= 8
        {
            return false;
        }
        let target_accepts =
            may_add_spell(target, &self.items, IDR_WARCRY, self.tick.0 as u32).is_some();
        let caster_needs_shield = character_value_present(attacker, CharacterValue::MagicShield)
            == 0
            && attacker.lifeshield
                < character_value(attacker, CharacterValue::Warcry) * POWERSCALE / 4;
        target_accepts || caster_needs_shield
    }

    pub(crate) fn setup_simple_baddy_warcry_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if !self.simple_baddy_can_warcry(&attacker, target) {
            return false;
        }
        self.setup_simple_baddy_spell_action(
            character_id,
            |character, items, _tick, map, weather_movement_percent| {
                do_warcry(character, items, map, weather_movement_percent)
            },
        )
    }
}
