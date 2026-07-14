//! Simple-baddy distance-keeping: ranged attack setup, the distance driver,
//! attack-back moves, fleeing and pulse attacks.

use super::*;

impl World {
    #[allow(dead_code)]
    pub(crate) fn setup_simple_baddy_distance_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        area_id: u16,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let current_tick = self.tick.0 as u32;
        let target_has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;
        let tile_distance = tile_char_dist(&attacker, target);

        let freeze_spacing = character_value_present(&attacker, CharacterValue::Freeze) != 0
            && attacker.mana > POWERSCALE * 3
            && tile_distance > 3
            && may_add_spell(target, &self.items, IDR_FREEZE, current_tick).is_some()
            && freeze_speed_modifier(
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
            ) < -10;
        let flash_spacing = character_value_present(&attacker, CharacterValue::Flash) != 0
            && attacker.mana > POWERSCALE * 3
            && may_add_spell(&attacker, &self.items, IDR_FLASH, current_tick).is_none();

        if !freeze_spacing && !flash_spacing {
            return false;
        }

        self.setup_simple_baddy_distance_driver(character_id, target, 3, area_id, true)
    }

    #[allow(dead_code)]
    pub(crate) fn setup_simple_baddy_fireball_distance_attack(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        area_id: u16,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if attacker.mana <= FIREBALL_COST
            || character_value_present(&attacker, CharacterValue::Fireball) == 0
            || character_value_present(&attacker, CharacterValue::Fireball)
                <= character_value_present(&attacker, CharacterValue::Flash)
            || may_add_spell(&attacker, &self.items, IDR_FLASH, self.tick.0 as u32).is_none()
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

        self.setup_simple_baddy_distance_driver(character_id, target, 7, area_id, false)
    }

    pub(crate) fn setup_simple_baddy_distance_driver(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        distance: u16,
        area_id: u16,
        idle_when_already_there: bool,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        if step_char_dist(&attacker, target) == distance {
            if !idle_when_already_there {
                return false;
            }
            let Some(character) = self.characters.get_mut(&character_id) else {
                return false;
            };
            if do_idle(character, (TICKS_PER_SECOND / 4) as i32).is_err() {
                return false;
            }
            if let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() {
                data.lastfight = self.tick.0 as i32;
            }
            return true;
        }

        let target_positions = if target.tox != 0 {
            [
                (usize::from(target.tox), usize::from(target.toy)),
                (usize::from(target.x), usize::from(target.y)),
            ]
        } else {
            [
                (usize::from(target.x), usize::from(target.y)),
                (usize::from(target.x), usize::from(target.y)),
            ]
        };
        for (target_x, target_y) in target_positions {
            if self.setup_walk_toward(
                character_id,
                target_x,
                target_y,
                usize::from(distance),
                area_id,
                false,
            ) {
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

        let target_x = usize::from(target.x);
        let target_y = usize::from(target.y);
        let partial = pathfinder(
            &self.map,
            usize::from(attacker.x),
            usize::from(attacker.y),
            target_x,
            target_y,
            usize::from(distance),
            None,
        );
        if let Some(direction) = partial.best_direction {
            if self.walk_or_use_driver(character_id, direction, area_id) {
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

        false
    }

    pub fn distance_driver(
        &mut self,
        character_id: CharacterId,
        target_id: CharacterId,
        distance: u16,
        area_id: u16,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        if attacker.id == target.id
            || !char_see_char(&attacker, &target, &self.map, self.date.daylight)
        {
            return false;
        }
        if step_char_dist(&attacker, &target) == distance {
            return false;
        }

        if target.tox != 0
            && self.setup_walk_toward(
                character_id,
                usize::from(target.tox),
                usize::from(target.toy),
                usize::from(distance),
                area_id,
                false,
            )
        {
            return true;
        }
        if self.setup_walk_toward(
            character_id,
            usize::from(target.x),
            usize::from(target.y),
            usize::from(distance),
            area_id,
            false,
        ) {
            return true;
        }

        let partial = pathfinder(
            &self.map,
            usize::from(attacker.x),
            usize::from(attacker.y),
            usize::from(target.x),
            usize::from(target.y),
            usize::from(distance),
            None,
        );
        partial
            .best_direction
            .is_some_and(|direction| self.walk_or_use_driver(character_id, direction, area_id))
    }

    pub(crate) fn setup_simple_baddy_attack_back_move(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        area_id: u16,
    ) -> bool {
        let Ok(direction) = Direction::try_from(target.dir) else {
            return false;
        };
        let (dx, dy) = direction.delta();
        if dx != 0 && dy != 0 {
            return false;
        }

        let Some(back_x) = offset_coordinate(usize::from(target.x), -dx) else {
            return false;
        };
        let Some(back_y) = offset_coordinate(usize::from(target.y), -dy) else {
            return false;
        };
        if back_x < 1 || back_y < 1 || back_x >= MAX_MAP || back_y >= MAX_MAP {
            return false;
        }
        if self.map.blocks_movement(back_x, back_y) {
            return false;
        }

        let Some(front_x) = offset_coordinate(usize::from(target.x), dx) else {
            return false;
        };
        let Some(front_y) = offset_coordinate(usize::from(target.y), dy) else {
            return false;
        };
        if front_x < 1 || front_y < 1 || front_x >= MAX_MAP || front_y >= MAX_MAP {
            return false;
        }

        let front_occupied = self
            .map
            .tile(front_x, front_y)
            .is_some_and(|tile| tile.character != 0);
        if self.characters.get(&character_id).is_some_and(|attacker| {
            usize::from(attacker.x) == front_x && usize::from(attacker.y) == front_y
        }) {
            return false;
        }

        let Some(side_x) = offset_coordinate(usize::from(target.x), dy) else {
            return false;
        };
        let Some(side_y) = offset_coordinate(usize::from(target.y), dx) else {
            return false;
        };
        if side_x < 1 || side_y < 1 || side_x >= MAX_MAP || side_y >= MAX_MAP {
            return false;
        }
        let same_group_side_occupied = self
            .map
            .tile(side_x, side_y)
            .and_then(|tile| {
                (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
            })
            .and_then(|side_id| self.characters.get(&side_id))
            .is_some_and(|side_character| {
                side_character.id != character_id
                    && self
                        .characters
                        .get(&character_id)
                        .is_some_and(|attacker| side_character.group == attacker.group)
            });
        if same_group_side_occupied {
            return false;
        }

        let idle_target = target.action == action::IDLE
            && self.tick.0.saturating_sub(u64::from(target.regen_ticker)) > TICKS_PER_SECOND / 2;
        if !idle_target && !front_occupied {
            return false;
        }

        if !self.setup_walk_toward(character_id, back_x, back_y, 0, area_id, false) {
            return false;
        }
        if let Some(character) = self.characters.get_mut(&character_id) {
            if let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() {
                data.lastfight = self.tick.0 as i32;
            }
        }
        true
    }

    pub fn setup_simple_baddy_flee_action(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
    ) -> bool {
        let Some(attacker) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let enemies = self.refresh_simple_baddy_enemy_tracking(&attacker);
        let mut direction_scores = [0i32; 9];
        let mut min_distance = 99;

        for enemy in enemies.into_iter().filter(|enemy| enemy.visible) {
            let Some(target) = self.characters.get(&enemy.target_id) else {
                continue;
            };
            let distance = char_dist(&attacker, target);
            if distance > 30 {
                continue;
            }
            min_distance = min_distance.min(distance);
            let score = 5000 - distance * 50;
            let dx = i32::from(target.x) - i32::from(attacker.x);
            let dy = i32::from(target.y) - i32::from(attacker.y);
            let total_delta = dx.abs() + dy.abs();
            if total_delta == 0 {
                continue;
            }

            if dx > 0 {
                direction_scores[Direction::Right as usize] -= score * dx.abs() / total_delta;
                direction_scores[Direction::RightUp as usize] -= score * dx.abs() / total_delta / 2;
                direction_scores[Direction::RightDown as usize] -=
                    score * dx.abs() / total_delta / 2;
                direction_scores[Direction::Left as usize] += score * dx.abs() / total_delta / 4;
                direction_scores[Direction::LeftUp as usize] += score * dx.abs() / total_delta / 8;
                direction_scores[Direction::LeftDown as usize] +=
                    score * dx.abs() / total_delta / 8;
            }
            if dx < 0 {
                direction_scores[Direction::Left as usize] -= score * dx.abs() / total_delta;
                direction_scores[Direction::LeftUp as usize] -= score * dx.abs() / total_delta / 2;
                direction_scores[Direction::LeftDown as usize] -=
                    score * dx.abs() / total_delta / 2;
                direction_scores[Direction::Right as usize] -= score * dx.abs() / total_delta / 4;
                direction_scores[Direction::RightUp as usize] -= score * dx.abs() / total_delta / 8;
                direction_scores[Direction::RightDown as usize] -=
                    score * dx.abs() / total_delta / 8;
            }
            if dy > 0 {
                direction_scores[Direction::Down as usize] -= score * dy.abs() / total_delta;
                direction_scores[Direction::LeftDown as usize] -=
                    score * dy.abs() / total_delta / 2;
                direction_scores[Direction::RightDown as usize] -=
                    score * dy.abs() / total_delta / 2;
                direction_scores[Direction::Up as usize] -= score * dy.abs() / total_delta / 4;
                direction_scores[Direction::LeftUp as usize] -= score * dy.abs() / total_delta / 8;
                direction_scores[Direction::RightUp as usize] -= score * dy.abs() / total_delta / 8;
            }
            if dy < 0 {
                direction_scores[Direction::Up as usize] -= score * dy.abs() / total_delta;
                direction_scores[Direction::LeftUp as usize] -= score * dy.abs() / total_delta / 2;
                direction_scores[Direction::RightUp as usize] -= score * dy.abs() / total_delta / 2;
                direction_scores[Direction::Down as usize] -= score * dy.abs() / total_delta / 4;
                direction_scores[Direction::LeftDown as usize] -=
                    score * dy.abs() / total_delta / 8;
                direction_scores[Direction::RightDown as usize] -=
                    score * dy.abs() / total_delta / 8;
            }
        }
        if min_distance > 30 {
            return false;
        }

        if let Some(character) = self.characters.get_mut(&character_id) {
            if min_distance < 10
                && (character.endurance > 4 * POWERSCALE || character.speed_mode == SpeedMode::Fast)
            {
                character.speed_mode = SpeedMode::Fast;
            } else if min_distance < 10 {
                character.speed_mode = SpeedMode::Normal;
            } else {
                character.speed_mode = SpeedMode::Stealth;
            }
        }

        let mut best_direction = None;
        let mut best_score = i32::MIN;
        // Index-based loop kept: mirrors C's `for (dir = 1; dir < 9;
        // dir++)` over `dir_score[dir]` (`drvlib.c:1104-1109`); slot 0 of
        // `direction_scores` is C's unused `dir_score[0]`.
        #[allow(clippy::needless_range_loop)]
        for direction_id in 1..=8 {
            let direction =
                Direction::try_from(direction_id as u8).expect("valid legacy direction");
            let score = direction_scores[direction_id]
                + self.simple_baddy_flee_eval_path(
                    usize::from(attacker.x),
                    usize::from(attacker.y),
                    direction,
                );
            if score > best_score {
                best_direction = Some(direction);
                best_score = score;
            }
        }
        let Some(direction) = best_direction else {
            return false;
        };
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

    pub(crate) fn simple_baddy_flee_eval_path(
        &self,
        x: usize,
        y: usize,
        direction: Direction,
    ) -> i32 {
        let (dx, dy) = direction.delta();
        let mut x = x;
        let mut y = y;
        let mut score = 0;

        for _ in 0..10 {
            let Some(next_x) = offset_coordinate(x, dx) else {
                return score;
            };
            let Some(next_y) = offset_coordinate(y, dy) else {
                return score;
            };
            x = next_x;
            y = next_y;
            if !(1..MAX_MAP - 1).contains(&x) || !(1..MAX_MAP - 1).contains(&y) {
                return score;
            }
            let Some(tile) = self.map.tile(x, y) else {
                return score;
            };
            if tile
                .flags
                .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
            {
                return score;
            }
            let daylight = (i32::from(tile.daylight) * self.date.daylight) / 256;
            score += 300 - i32::from(tile.light).max(daylight);
        }

        score
    }

    pub(crate) fn setup_simple_baddy_pulse_attack(&mut self, character_id: CharacterId) -> bool {
        if self.simple_baddy_pulse_value(character_id) == 0 {
            return false;
        }

        let weather_movement_percent = self.settings.weather_movement_percent;
        let Some(character) = self.characters.get_mut(&character_id) else {
            return false;
        };
        if do_pulse(character, &self.map, weather_movement_percent).is_err() {
            return false;
        }
        if let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() {
            data.lastfight = self.tick.0 as i32;
        }
        true
    }
}
