//! Simple-baddy offensive spell setup: fireball, firering, fireball lanes,
//! earth spells and spell-task values.

use super::*;

impl World {
    pub(crate) fn setup_simple_baddy_fireball_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        area_id: u16,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if character_value(&attacker, CharacterValue::Fireball) <= 1
            || attacker.mana < FIREBALL_COST
        {
            return false;
        }

        let has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;
        let damage = fireball_damage(
            character_value(&attacker, CharacterValue::Fireball),
            character_value(target, CharacterValue::Immunity),
            character_value(target, CharacterValue::Tactics),
            has_tactics,
        );
        if damage < POWERSCALE {
            return false;
        }

        let target_dx = attacker.x.abs_diff(target.x);
        let target_dy = attacker.y.abs_diff(target.y);
        let (target_x, target_y) = if target_dx <= 1 && target_dy <= 1 {
            if may_add_spell(&attacker, &self.items, IDR_FIRERING, self.tick.0 as u32).is_none() {
                return false;
            }
            (usize::from(attacker.x), usize::from(attacker.y))
        } else {
            if !self.fireball_line_hits_target(
                character_id,
                target.id,
                usize::from(attacker.x),
                usize::from(attacker.y),
                usize::from(target.x),
                usize::from(target.y),
            ) {
                return self.setup_simple_baddy_fireball_lane_move(character_id, target, area_id);
            }
            predicted_fireball_target(&attacker, target)
        };

        let weather_movement_percent = self.settings.weather_movement_percent;
        let Some(attacker_mut) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if do_fireball(
            attacker_mut,
            &self.items,
            target_x,
            target_y,
            self.tick.0 as u32,
            &self.map,
            weather_movement_percent,
        )
        .is_err()
        {
            return false;
        }
        if let Some(CharacterDriverState::SimpleBaddy(data)) = attacker_mut.driver_state.as_mut() {
            data.lastfight = self.tick.0 as i32;
        }
        true
    }

    pub(crate) fn setup_simple_baddy_firering_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if character_value(&attacker, CharacterValue::Fireball) <= 1
            || attacker.mana < FIREBALL_COST
            || tile_char_dist(&attacker, target) >= 2
            || may_add_spell(&attacker, &self.items, IDR_FIRERING, self.tick.0 as u32).is_none()
        {
            return false;
        }

        let has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;
        let damage = fireball_damage(
            character_value(&attacker, CharacterValue::Fireball),
            character_value(target, CharacterValue::Immunity),
            character_value(target, CharacterValue::Tactics),
            has_tactics,
        );
        if damage < POWERSCALE {
            return false;
        }

        let weather_movement_percent = self.settings.weather_movement_percent;
        let Some(attacker_mut) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if do_fireball(
            attacker_mut,
            &self.items,
            usize::from(attacker.x),
            usize::from(attacker.y),
            self.tick.0 as u32,
            &self.map,
            weather_movement_percent,
        )
        .is_err()
        {
            return false;
        }
        if let Some(CharacterDriverState::SimpleBaddy(data)) = attacker_mut.driver_state.as_mut() {
            data.lastfight = self.tick.0 as i32;
        }
        true
    }

    pub(crate) fn setup_simple_baddy_fireball_lane_move(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        area_id: u16,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let directions = [
            Direction::Right,
            Direction::Left,
            Direction::Down,
            Direction::Up,
        ];
        let mut blocked_directions = [false; 4];

        for distance in 1..5 {
            for (index, direction) in directions.into_iter().enumerate() {
                if blocked_directions[index] {
                    continue;
                }
                let (dx, dy) = direction.delta();
                let Some(x) = offset_coordinate(usize::from(attacker.x), dx * distance as i16)
                else {
                    blocked_directions[index] = true;
                    continue;
                };
                let Some(y) = offset_coordinate(usize::from(attacker.y), dy * distance as i16)
                else {
                    blocked_directions[index] = true;
                    continue;
                };
                if x >= MAX_MAP || y >= MAX_MAP || self.map.blocks_movement(x, y) {
                    blocked_directions[index] = true;
                    continue;
                }
                if !self.fireball_line_hits_target(
                    character_id,
                    target.id,
                    x,
                    y,
                    usize::from(target.x),
                    usize::from(target.y),
                ) {
                    continue;
                }
                if self.setup_walk_direction(character_id, direction, area_id) {
                    if let Some(character) = self.characters.get_mut(&character_id) {
                        if let Some(CharacterDriverState::SimpleBaddy(data)) =
                            character.driver_state.as_mut()
                        {
                            data.lastfight = self.tick.0 as i32;
                        }
                    }
                    return true;
                }
            }
        }

        false
    }

    pub(crate) fn simple_baddy_fireball_lane_task(
        &self,
        attacker: &Character,
        target: &Character,
    ) -> Option<(FightDriverTaskKind, i32)> {
        let directions = [
            (Direction::Right, FightDriverTaskKind::MoveRight),
            (Direction::Left, FightDriverTaskKind::MoveLeft),
            (Direction::Down, FightDriverTaskKind::MoveDown),
            (Direction::Up, FightDriverTaskKind::MoveUp),
        ];
        let mut blocked_directions = [false; 4];

        for distance in 1..5 {
            for (index, (direction, kind)) in directions.into_iter().enumerate() {
                if blocked_directions[index] {
                    continue;
                }
                let (dx, dy) = direction.delta();
                let Some(x) = offset_coordinate(usize::from(attacker.x), dx * distance as i16)
                else {
                    blocked_directions[index] = true;
                    continue;
                };
                let Some(y) = offset_coordinate(usize::from(attacker.y), dy * distance as i16)
                else {
                    blocked_directions[index] = true;
                    continue;
                };
                if x >= MAX_MAP || y >= MAX_MAP || self.map.blocks_movement(x, y) {
                    blocked_directions[index] = true;
                    continue;
                }
                if self.fireball_line_hits_target(
                    attacker.id,
                    target.id,
                    x,
                    y,
                    usize::from(target.x),
                    usize::from(target.y),
                ) {
                    return Some((kind, distance));
                }
            }
        }

        None
    }

    pub(crate) fn setup_simple_baddy_lane_walk(
        &mut self,
        character_id: CharacterId,
        direction: Direction,
        area_id: u16,
    ) -> bool {
        if !self.setup_walk_direction(character_id, direction, area_id) {
            return false;
        }
        if let Some(character) = self.characters.get_mut(&character_id) {
            if let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() {
                data.lastfight = self.tick.0 as i32;
            }
        }
        true
    }

    pub(crate) fn fireball_line_hits_target(
        &self,
        attacker_id: CharacterId,
        target_id: CharacterId,
        from_x: usize,
        from_y: usize,
        target_x: usize,
        target_y: usize,
    ) -> bool {
        let mut x = from_x as i32 * 1024 + 512;
        let mut y = from_y as i32 * 1024 + 512;
        let mut dx = target_x as i32 - from_x as i32;
        let mut dy = target_y as i32 - from_y as i32;

        if dx.abs() < 2 && dy.abs() < 2 {
            return false;
        }
        if dx.abs() > dy.abs() {
            dy = dy * 512 / dx.abs();
            dx = dx * 512 / dx.abs();
        } else {
            dx = dx * 512 / dy.abs();
            dy = dy * 512 / dy.abs();
        }

        for _ in 0..48 {
            let cx = x / 1024;
            let cy = y / 1024;
            let Ok(cx_usize) = usize::try_from(cx) else {
                return false;
            };
            let Ok(cy_usize) = usize::try_from(cy) else {
                return false;
            };
            let Some(tile) = self.map.tile(cx_usize, cy_usize) else {
                return false;
            };
            let fire_block = tile.flags.contains(MapFlags::TMOVEBLOCK)
                || (!tile.flags.contains(MapFlags::FIRETHRU)
                    && tile.flags.contains(MapFlags::MOVEBLOCK));
            if fire_block && tile.character != attacker_id.0 as u16 {
                return self.fireball_block_hits_recorded_enemy(
                    attacker_id,
                    target_id,
                    cx_usize,
                    cy_usize,
                );
            }
            x += dx;
            y += dy;
        }

        false
    }

    pub(crate) fn fireball_block_hits_recorded_enemy(
        &self,
        attacker_id: CharacterId,
        target_id: CharacterId,
        x: usize,
        y: usize,
    ) -> bool {
        let recorded_enemies = self.simple_baddy_recorded_enemy_ids(attacker_id);
        let mut hits_enemy = false;
        for (dx, dy) in [
            (0, 0),
            (1, 0),
            (0, 1),
            (-1, 0),
            (0, -1),
            (1, 1),
            (-1, 1),
            (1, -1),
            (-1, -1),
        ] {
            let Some(check_x) = offset_coordinate(x, dx) else {
                continue;
            };
            let Some(check_y) = offset_coordinate(y, dy) else {
                continue;
            };
            let Some(character_id) = self
                .map
                .tile(check_x, check_y)
                .map(|tile| CharacterId(u32::from(tile.character)))
                .filter(|id| id.0 != 0)
            else {
                continue;
            };
            if character_id == attacker_id {
                continue;
            }
            if character_id == target_id || recorded_enemies.contains(&character_id) {
                hits_enemy = true;
            } else {
                return false;
            }
        }
        hits_enemy
    }

    #[allow(dead_code)]
    pub(crate) fn setup_simple_baddy_spell_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        random: &mut impl FnMut(u32) -> u32,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let current_tick = self.tick.0 as u32;
        let target_has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;
        let tile_distance = tile_char_dist(&attacker, target);
        let character_distance = char_dist(&attacker, target);

        if character_value(&attacker, CharacterValue::Freeze) > 1
            && attacker.mana >= FREEZE_COST
            && tile_distance < 4
            && may_add_spell(target, &self.items, IDR_FREEZE, current_tick).is_some()
        {
            let modifier = freeze_speed_modifier(
                spell_power(
                    character_value(&attacker, CharacterValue::Freeze),
                    character_value(&attacker, CharacterValue::Tactics),
                ),
                character_value(target, CharacterValue::Immunity),
                character_value(target, CharacterValue::Tactics),
                target_has_tactics,
                attacker.flags.contains(CharacterFlags::IDEMON),
                // C: freeze_value (tool.c) reads the caster's V_DEMON from value[1].
                character_value_present(&attacker, CharacterValue::Demon),
                character_value(target, CharacterValue::Cold),
            );
            if modifier < -10 {
                return self.setup_simple_baddy_spell_action(
                    character_id,
                    |character, _items, _tick, map, weather_movement_percent| {
                        do_freeze(character, map, weather_movement_percent)
                    },
                );
            }
        }

        if character_value(&attacker, CharacterValue::Flash) > 1
            && attacker.mana >= FLASH_COST
            && strike_damage(
                spell_power(
                    character_value(&attacker, CharacterValue::Flash),
                    character_value(&attacker, CharacterValue::Tactics),
                ),
                character_value(target, CharacterValue::Immunity),
                character_value(target, CharacterValue::Tactics),
                target_has_tactics,
            ) > POWERSCALE
        {
            if character_distance > 10 && character_distance < 30 {
                let target_x = usize::from(target.x).saturating_sub(1)
                    + usize::try_from(random(3).min(2)).unwrap_or(0);
                let target_y = usize::from(target.y).saturating_sub(1)
                    + usize::try_from(random(3).min(2)).unwrap_or(0);
                return self.setup_simple_baddy_spell_action(
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
                );
            }

            if tile_distance < 4
                && may_add_spell(&attacker, &self.items, IDR_FLASH, current_tick).is_some()
            {
                return self.setup_simple_baddy_spell_action(
                    character_id,
                    |character, items, tick, map, weather_movement_percent| {
                        do_flash(character, items, tick, map, weather_movement_percent)
                    },
                );
            }
        }

        if character_value(&attacker, CharacterValue::Warcry) > 1
            && attacker.endurance
                > character_value(&attacker, CharacterValue::Warcry) * POWERSCALE / 3
            && character_distance < 8
        {
            let modifier = warcry_speed_modifier(
                spell_power(
                    character_value(&attacker, CharacterValue::Warcry),
                    character_value(&attacker, CharacterValue::Tactics),
                ),
                character_value(target, CharacterValue::Immunity),
                character_value(target, CharacterValue::Tactics),
                target_has_tactics,
            );
            let target_accepts_warcry = modifier < -10
                && may_add_spell(target, &self.items, IDR_WARCRY, current_tick).is_some();
            let caster_needs_shield =
                character_value_present(&attacker, CharacterValue::MagicShield) == 0
                    && attacker.lifeshield
                        < character_value(&attacker, CharacterValue::Warcry) * POWERSCALE / 4;
            if target_accepts_warcry || caster_needs_shield {
                return self.setup_simple_baddy_spell_action(
                    character_id,
                    |character, items, _tick, map, weather_movement_percent| {
                        do_warcry(character, items, map, weather_movement_percent)
                    },
                );
            }
        }

        false
    }

    pub(crate) fn setup_simple_baddy_spell_action(
        &mut self,
        character_id: CharacterId,
        action: impl FnOnce(
            &mut Character,
            &HashMap<ItemId, Item>,
            u32,
            &MapGrid,
            i32,
        ) -> Result<(), crate::do_action::DoError>,
    ) -> bool {
        let weather_movement_percent = self.settings.weather_movement_percent;
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if action(
            character,
            &self.items,
            self.tick.0 as u32,
            &self.map,
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

    pub(crate) fn simple_baddy_distance3_task_value(
        &self,
        attacker: &Character,
        target: &Character,
        suppressions: FightDriverSuppressions,
    ) -> i32 {
        let current_tick = self.tick.0 as u32;
        let mut value = 0;
        if !suppressions.nofreeze
            && attacker.mana > POWERSCALE * 3
            && character_value_present(attacker, CharacterValue::Freeze) != 0
            && tile_char_dist(attacker, target) > 3
            && may_add_spell(target, &self.items, IDR_FREEZE, current_tick).is_some()
            && self.simple_baddy_freeze_modifier(attacker, target) < -10
        {
            value += if character_value_present(attacker, CharacterValue::Attack) != 0 {
                FIGHT_DRIVER_LOW_PRIO + character_value(attacker, CharacterValue::Freeze)
            } else {
                FIGHT_DRIVER_MED_PRIO + character_value(attacker, CharacterValue::Freeze)
            };
        }
        if !suppressions.noflash
            && attacker.mana > POWERSCALE * 3
            && character_value_present(attacker, CharacterValue::Flash) != 0
            && may_add_spell(attacker, &self.items, IDR_FLASH, current_tick).is_none()
        {
            value += if character_value_present(attacker, CharacterValue::Attack) != 0 {
                FIGHT_DRIVER_LOW_PRIO + character_value(attacker, CharacterValue::Flash)
            } else {
                FIGHT_DRIVER_MED_PRIO + character_value(attacker, CharacterValue::Flash)
            };
        }
        value
    }

    pub(crate) fn simple_baddy_distance7_task_value(
        &self,
        attacker: &Character,
        target: &Character,
        suppressions: FightDriverSuppressions,
    ) -> i32 {
        if suppressions.nofireball
            || attacker.mana <= FIREBALL_COST
            || character_value_present(attacker, CharacterValue::Fireball) == 0
            || character_value_present(attacker, CharacterValue::Fireball)
                <= character_value_present(attacker, CharacterValue::Flash)
            || may_add_spell(attacker, &self.items, IDR_FLASH, self.tick.0 as u32).is_none()
        {
            return 0;
        }
        let damage = fireball_damage(
            character_value(attacker, CharacterValue::Fireball),
            character_value(target, CharacterValue::Immunity),
            character_value(target, CharacterValue::Tactics),
            character_value_present(target, CharacterValue::Tactics) != 0,
        );
        if damage < POWERSCALE {
            return 0;
        }
        if character_value_present(attacker, CharacterValue::Attack) != 0 {
            FIGHT_DRIVER_LOW_PRIO + character_value(attacker, CharacterValue::Fireball)
        } else {
            FIGHT_DRIVER_MED_PRIO + character_value(attacker, CharacterValue::Fireball)
        }
    }

    pub(crate) fn simple_baddy_attackback_value(
        &self,
        character_id: CharacterId,
        target: &Character,
    ) -> i32 {
        let Some(attacker) = self.characters.get(&character_id) else {
            return 0;
        };
        let Ok(direction) = Direction::try_from(target.dir) else {
            return 0;
        };
        let (dx, dy) = direction.delta();
        if dx != 0 && dy != 0 {
            return 0;
        }
        let Some(back_x) = offset_coordinate(usize::from(target.x), -dx) else {
            return 0;
        };
        let Some(back_y) = offset_coordinate(usize::from(target.y), -dy) else {
            return 0;
        };
        if back_x < 1 || back_y < 1 || back_x >= MAX_MAP || back_y >= MAX_MAP {
            return 0;
        }
        if self.map.blocks_movement(back_x, back_y) {
            return 0;
        }
        let Some(front_x) = offset_coordinate(usize::from(target.x), dx) else {
            return 0;
        };
        let Some(front_y) = offset_coordinate(usize::from(target.y), dy) else {
            return 0;
        };
        if usize::from(attacker.x) == front_x && usize::from(attacker.y) == front_y {
            return 0;
        }
        if target.action == action::IDLE
            && self.tick.0.saturating_sub(u64::from(target.regen_ticker)) > TICKS_PER_SECOND / 2
        {
            return FIGHT_DRIVER_HIGH_PRIO;
        }
        let front_occupied = self
            .map
            .tile(front_x, front_y)
            .is_some_and(|tile| tile.character != 0);
        if !front_occupied {
            return 0;
        }
        let Some(side_x) = offset_coordinate(usize::from(target.x), dy) else {
            return 0;
        };
        let Some(side_y) = offset_coordinate(usize::from(target.y), dx) else {
            return 0;
        };
        let same_group_side_occupied = self
            .map
            .tile(side_x, side_y)
            .and_then(|tile| {
                (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
            })
            .and_then(|side_id| self.characters.get(&side_id))
            .is_some_and(|side_character| {
                side_character.id != character_id && side_character.group == attacker.group
            });
        if same_group_side_occupied {
            0
        } else {
            FIGHT_DRIVER_HIGH_PRIO
        }
    }

    pub(crate) fn simple_baddy_enemy_tracking(
        &self,
        character_id: CharacterId,
        target_id: CharacterId,
    ) -> Option<(bool, u16, u16)> {
        let character = self.characters.get(&character_id)?;
        let target = self.characters.get(&target_id)?;
        let visible = char_see_char(character, target, &self.map, self.date.daylight);
        Some((visible, target.x, target.y))
    }
}
