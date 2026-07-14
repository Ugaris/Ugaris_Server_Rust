//! Weighted fight-task scoring and the `fight_driver_attack_enemy` task list.

use super::*;

impl World {
    /// C `fight_driver_attack_enemy`: score every applicable task, apply
    /// the level-based silliness rolls, sort, and run tasks in order until
    /// one succeeds. `suppressions` generalizes the `nomove`/`nobless`/...
    /// positional arguments so both the NPC driver (always all-`false`)
    /// and a future player/lostcon caller (`lostcon_ppd`-backed toggles)
    /// can share this engine.
    pub(crate) fn setup_weighted_fight_task(
        &mut self,
        character_id: CharacterId,
        target: &Character,
        area_id: u16,
        suppressions: FightDriverSuppressions,
        random: &mut impl FnMut(u32) -> u32,
    ) -> bool {
        let mut tasks = self.simple_baddy_fight_tasks(character_id, target, area_id, suppressions);
        let level = self
            .characters
            .get(&character_id)
            .map(|character| character.level as i32)
            .unwrap_or_default();
        order_fight_driver_tasks(&mut tasks, level, |below| random(below as u32) as i32);

        for (index, task) in tasks.iter().copied().enumerate() {
            let ret = match task.kind {
                FightDriverTaskKind::Freeze => {
                    self.setup_simple_baddy_freeze_attack(character_id, target)
                }
                FightDriverTaskKind::Heal => self.setup_simple_baddy_heal_action(character_id),
                FightDriverTaskKind::MagicShield => {
                    self.setup_simple_baddy_magicshield_action(character_id)
                }
                FightDriverTaskKind::EarthMud => {
                    self.setup_simple_baddy_earthmud_attack(character_id, target)
                }
                FightDriverTaskKind::Bless => {
                    self.setup_simple_baddy_self_bless_action(character_id)
                }
                FightDriverTaskKind::Fireball => {
                    self.setup_simple_baddy_fireball_attack(character_id, target, area_id)
                }
                FightDriverTaskKind::FireRing => {
                    self.setup_simple_baddy_firering_attack(character_id, target)
                }
                FightDriverTaskKind::Ball => {
                    self.setup_simple_baddy_ball_attack(character_id, target, random)
                }
                FightDriverTaskKind::Flash => {
                    self.setup_simple_baddy_flash_attack(character_id, target)
                }
                FightDriverTaskKind::Warcry => {
                    self.setup_simple_baddy_warcry_attack(character_id, target)
                }
                FightDriverTaskKind::Attack => {
                    self.setup_simple_baddy_attack_driver(character_id, target)
                        || self.setup_simple_baddy_attack_move(character_id, target, area_id)
                }
                FightDriverTaskKind::Regenerate => {
                    self.setup_simple_baddy_regenerate_action(character_id)
                }
                FightDriverTaskKind::Distance3 => {
                    self.setup_simple_baddy_distance_driver(character_id, target, 3, area_id, true)
                }
                FightDriverTaskKind::Distance7 => {
                    self.setup_simple_baddy_distance_driver(character_id, target, 7, area_id, false)
                }
                FightDriverTaskKind::Pulse => self.setup_simple_baddy_pulse_attack(character_id),
                FightDriverTaskKind::AttackBack => {
                    fight_driver_attackback_may_run(&tasks, index)
                        && self.setup_simple_baddy_attack_back_move(character_id, target, area_id)
                }
                FightDriverTaskKind::MoveRight => {
                    self.setup_simple_baddy_lane_walk(character_id, Direction::Right, area_id)
                }
                FightDriverTaskKind::MoveLeft => {
                    self.setup_simple_baddy_lane_walk(character_id, Direction::Left, area_id)
                }
                FightDriverTaskKind::MoveUp => {
                    self.setup_simple_baddy_lane_walk(character_id, Direction::Up, area_id)
                }
                FightDriverTaskKind::MoveDown => {
                    self.setup_simple_baddy_lane_walk(character_id, Direction::Down, area_id)
                }
                FightDriverTaskKind::Flee => {
                    self.setup_simple_baddy_flee_action(character_id, area_id)
                }
                FightDriverTaskKind::EarthRain => false,
            };
            if ret {
                return true;
            }
        }

        false
    }

    pub(crate) fn simple_baddy_fight_tasks(
        &self,
        character_id: CharacterId,
        target: &Character,
        area_id: u16,
        suppressions: impl Into<FightDriverSuppressions>,
    ) -> Vec<FightDriverTask> {
        let suppressions = suppressions.into();
        let Some(attacker) = self.characters.get(&character_id) else {
            return Vec::new();
        };
        let mut tasks = Vec::new();
        let character_distance = char_dist(attacker, target);
        let tile_distance = tile_char_dist(attacker, target);
        let current_tick = self.tick.0 as u32;
        let target_has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;

        if !suppressions.nofreeze
            && character_value(attacker, CharacterValue::Freeze) > 1
            && attacker.mana >= FREEZE_COST
            && tile_distance < 4
            && may_add_spell(target, &self.items, IDR_FREEZE, current_tick).is_some()
            && self.simple_baddy_freeze_modifier(attacker, target) < -10
        {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Freeze,
                value: FIGHT_DRIVER_HIGH_PRIO + character_value(attacker, CharacterValue::Freeze),
            });
        }
        if !suppressions.noheal && self.simple_baddy_can_heal_self(attacker) {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Heal,
                value: FIGHT_DRIVER_HIGH_PRIO + character_value(attacker, CharacterValue::Heal),
            });
        }
        if !suppressions.noshield && self.simple_baddy_can_magicshield_self(attacker) {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::MagicShield,
                value: FIGHT_DRIVER_HIGH_PRIO
                    + character_value(attacker, CharacterValue::MagicShield),
            });
        }
        let earthmud_good = self.simple_baddy_earthmud_value(target);
        if attacker.flags.contains(CharacterFlags::EDEMON)
            && character_value_present(attacker, CharacterValue::Demon) == 30
            && attacker.hp >= character_value(attacker, CharacterValue::Hp) * POWERSCALE / 2
            && earthmud_good > 0
        {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::EarthMud,
                value: simple_baddy_earth_task_value(earthmud_good),
            });
        }
        if !suppressions.nobless && self.simple_baddy_can_bless_self(attacker) {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Bless,
                value: FIGHT_DRIVER_HIGH_PRIO + character_value(attacker, CharacterValue::Bless),
            });
        }
        let fireball_damage_value = fireball_damage(
            character_value(attacker, CharacterValue::Fireball),
            character_value(target, CharacterValue::Immunity),
            character_value(target, CharacterValue::Tactics),
            target_has_tactics,
        );
        if !suppressions.nofireball
            && character_value(attacker, CharacterValue::Fireball) > 1
            && fireball_damage_value >= POWERSCALE
            && attacker.mana >= FIREBALL_COST
        {
            let fireball_value =
                FIGHT_DRIVER_MED_PRIO + character_value(attacker, CharacterValue::Fireball);
            if tile_distance < 2
                && may_add_spell(attacker, &self.items, IDR_FIRERING, current_tick).is_some()
            {
                tasks.push(FightDriverTask {
                    kind: FightDriverTaskKind::FireRing,
                    value: fireball_value,
                });
            } else if self.fireball_line_hits_target(
                character_id,
                target.id,
                usize::from(attacker.x),
                usize::from(attacker.y),
                usize::from(target.x),
                usize::from(target.y),
            ) {
                tasks.push(FightDriverTask {
                    kind: FightDriverTaskKind::Fireball,
                    value: fireball_value,
                });
            } else if let Some((kind, distance)) =
                self.simple_baddy_fireball_lane_task(attacker, target)
            {
                tasks.push(FightDriverTask {
                    kind,
                    value: fireball_value / distance + 1,
                });
            }
        }
        if character_value(attacker, CharacterValue::Flash) > 1
            && strike_damage(
                spell_power(
                    character_value(attacker, CharacterValue::Flash),
                    character_value(attacker, CharacterValue::Tactics),
                ),
                character_value(target, CharacterValue::Immunity),
                character_value(target, CharacterValue::Tactics),
                target_has_tactics,
            ) > POWERSCALE
            && attacker.mana >= FLASH_COST
        {
            let ball_reaches_target = self.simple_baddy_calc_ball_steps(
                attacker.id,
                usize::from(attacker.x),
                usize::from(attacker.y),
                usize::from(target.x),
                usize::from(target.y),
            ) > i32::from(tile_distance) * 2 - 5;
            if !suppressions.noball
                && character_distance > 10
                && character_distance < 30
                && ball_reaches_target
            {
                tasks.push(FightDriverTask {
                    kind: FightDriverTaskKind::Ball,
                    value: FIGHT_DRIVER_MED_PRIO + character_value(attacker, CharacterValue::Flash),
                });
            }
            if !suppressions.noflash
                && tile_distance < 4
                && may_add_spell(attacker, &self.items, IDR_FLASH, current_tick).is_some()
            {
                tasks.push(FightDriverTask {
                    kind: FightDriverTaskKind::Flash,
                    value: FIGHT_DRIVER_MED_PRIO
                        + character_value(attacker, CharacterValue::Flash)
                        + character_value(attacker, CharacterValue::Flash) / 2,
                });
            }
        }
        if !suppressions.nowarcry && self.simple_baddy_can_warcry(attacker, target) {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Warcry,
                value: FIGHT_DRIVER_HIGH_PRIO
                    + character_value(attacker, CharacterValue::Warcry) / 2,
            });
        }
        if !suppressions.nomove || character_distance == 2 {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Attack,
                value: simple_baddy_attack_task_value(attacker, &self.items),
            });
        }
        if area_id != 33 && self.simple_baddy_needs_regeneration(attacker) {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Regenerate,
                value: self.simple_baddy_regenerate_task_value(attacker),
            });
        }
        if !suppressions.nomove {
            let distance3 = self.simple_baddy_distance3_task_value(attacker, target, suppressions);
            if distance3 > 0 {
                tasks.push(FightDriverTask {
                    kind: FightDriverTaskKind::Distance3,
                    value: distance3,
                });
            }
            let distance7 = self.simple_baddy_distance7_task_value(attacker, target, suppressions);
            if distance7 > 0 {
                tasks.push(FightDriverTask {
                    kind: FightDriverTaskKind::Distance7,
                    value: distance7,
                });
            }
        }
        let pulse = self.simple_baddy_pulse_value(character_id);
        if !suppressions.nopulse && pulse > 0 {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Pulse,
                value: FIGHT_DRIVER_HIGH_PRIO + pulse,
            });
        }
        if !suppressions.nomove && self.simple_baddy_attackback_value(character_id, target) > 0 {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::AttackBack,
                value: FIGHT_DRIVER_HIGH_PRIO,
            });
        }
        if attacker.hp < character_value(attacker, CharacterValue::Hp) * POWERSCALE / 2 {
            tasks.push(FightDriverTask {
                kind: FightDriverTaskKind::Flee,
                value: FIGHT_DRIVER_HIGH_PRIO,
            });
        }

        tasks
    }

    pub(crate) fn simple_baddy_pulse_value(&self, character_id: CharacterId) -> i32 {
        let Some(caster) = self.characters.get(&character_id) else {
            return 0;
        };
        let pulse_value = character_value(caster, CharacterValue::Pulse);
        if pulse_value == 0 || caster.mana <= POWERSCALE {
            return 0;
        }

        let pulse_power = spell_power(
            pulse_value,
            character_value(caster, CharacterValue::Tactics),
        );
        let Some(spend) = pulse_spend(pulse_power, caster.mana) else {
            return 0;
        };

        let mut damageable_total = 0;
        for dx in -2..=2 {
            for dy in -2..=2 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                damageable_total +=
                    self.simple_baddy_pulse_field_value(caster, dx, dy, spend.amount);
            }
        }

        if damageable_total * 95 > spend.mana_cost * 100 {
            pulse_value
        } else {
            0
        }
    }

    pub(crate) fn simple_baddy_pulse_field_value(
        &self,
        caster: &Character,
        dx: i32,
        dy: i32,
        pulse_amount: i32,
    ) -> i32 {
        let x = i32::from(caster.x) + dx;
        let y = i32::from(caster.y) + dy;
        if x < 0 || y < 0 {
            return 0;
        }
        let Some(tile) = self.map.tile(x as usize, y as usize) else {
            return 0;
        };
        let target_id = CharacterId(u32::from(tile.character));
        let Some(target) = self.characters.get(&target_id) else {
            return 0;
        };
        if !can_attack(caster, target, &self.map)
            || !self.map.can_see(
                usize::from(caster.x),
                usize::from(caster.y),
                x as usize,
                y as usize,
                DIST_MAX,
            )
        {
            return 0;
        }
        if target.mana > POWERSCALE * 4
            && (character_value(target, CharacterValue::MagicShield) != 0
                || character_value(target, CharacterValue::Heal) != 0)
        {
            return 0;
        }
        if target.action == action::HEAL_SELF || target.action == action::MAGICSHIELD {
            return 0;
        }

        let has = target.hp + target.lifeshield;
        let total = character_value(target, CharacterValue::Hp) * POWERSCALE
            + character_value(target, CharacterValue::MagicShield) * POWERSCALE
            + 1;
        if total <= 0 || has * 100 / total > 72 {
            return 0;
        }

        let target_has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;
        let damage = pulse_damage(
            character_value(caster, CharacterValue::Pulse),
            pulse_amount,
            character_value(target, CharacterValue::Immunity),
            character_value(target, CharacterValue::Tactics),
            target_has_tactics,
        );
        if damage * 95 < has * 100 {
            return 0;
        }
        has
    }
}
