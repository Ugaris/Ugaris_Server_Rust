use super::*;

pub(crate) const IID_HARDKILL: u32 = (0x01 << 24) | 0x00005D;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LegacyHurtOutcome {
    pub damage_after_armor: i32,
    pub shield_absorbed: i32,
    pub hp_damage: i32,
    pub killed: bool,
    pub nodeath_saved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacyHurtEvent {
    pub target_id: CharacterId,
    pub cause_id: CharacterId,
    pub outcome: LegacyHurtOutcome,
}

impl World {
    pub fn apply_legacy_hurt(
        &mut self,
        target_id: CharacterId,
        cause_id: Option<CharacterId>,
        damage: i32,
        armor_divisor: i32,
        armor_percent: i32,
        shield_percent: i32,
    ) -> Option<LegacyHurtOutcome> {
        let cause_id = cause_id.filter(|id| id.0 != 0 && self.characters.contains_key(id));
        let mut outcome = LegacyHurtOutcome::default();
        let show_attack_debug = self.show_attack_debug;
        let cause_position = cause_id.and_then(|id| {
            self.characters
                .get(&id)
                .map(|character| (character.x, character.y))
        });
        let cause_name = cause_id
            .and_then(|id| self.characters.get(&id))
            .map(|character| character.name.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let cause_hardkill_weapon = cause_id.and_then(|id| {
            let item_id = self
                .characters
                .get(&id)?
                .inventory
                .get(worn_slot::RIGHT_HAND)?
                .as_ref()?;
            self.items.get(item_id).map(|item| {
                (
                    item.template_id,
                    item.driver_data.get(37).copied().unwrap_or_default(),
                )
            })
        });
        let has_magicshield_effect = self.effects.values().any(|effect| {
            effect.effect_type == EF_MAGICSHIELD && effect.target_character == Some(target_id)
        });
        let mut create_magicshield_effect = false;

        let mut show_attack_messages = Vec::new();
        let (target_x, target_y, target_was_player, target_was_male) = {
            let target = self.characters.get_mut(&target_id)?;
            if target.flags.contains(CharacterFlags::DEAD) {
                return None;
            }
            let target_was_player = target.flags.contains(CharacterFlags::PLAYER);
            let target_was_male = target.flags.contains(CharacterFlags::MALE);

            let damage = damage.max(0);
            let armor_value = character_value(target, CharacterValue::Armor);
            let mut damage_after_armor =
                reduce_hurt_by_armor(damage, armor_value, armor_divisor, armor_percent);
            outcome.damage_after_armor = damage_after_armor;

            if show_attack_debug && damage != 0 {
                show_attack_messages.push(format!(
                    "hurt by {}, dam={:.2}, armor={:.2} armorper={} shieldper={}",
                    cause_name,
                    damage as f64 / f64::from(POWERSCALE),
                    armor_value as f64 / 20.0 / f64::from(armor_divisor.max(1)),
                    armor_percent,
                    shield_percent
                ));
                if damage_after_armor != 0 {
                    show_attack_messages.push(format!(
                        "dam after armor: {:.2}",
                        damage_after_armor as f64 / f64::from(POWERSCALE)
                    ));
                }
            }

            if target.flags.contains(CharacterFlags::FDEMON)
                && !cause_position.is_some_and(|(x, y)| is_back_attack_against_target(target, x, y))
            {
                damage_after_armor /= 100;
            }

            if target.flags.contains(CharacterFlags::HARDKILL)
                && !cause_hardkill_weapon.is_some_and(|(template_id, level)| {
                    template_id == IID_HARDKILL && u32::from(level) >= target.level
                })
            {
                damage_after_armor = 0;
            }

            if !target.flags.contains(CharacterFlags::IMMORTAL) {
                let mut hp_damage = damage_after_armor;
                if hp_damage != 0 && target.lifeshield != 0 && shield_percent > 0 {
                    let shield_absorbed = (hp_damage * shield_percent / 100).min(target.lifeshield);
                    target.lifeshield -= shield_absorbed;
                    hp_damage -= shield_absorbed;
                    outcome.shield_absorbed = shield_absorbed;
                    create_magicshield_effect = shield_absorbed > 0
                        && character_value_present(target, CharacterValue::MagicShield) != 0
                        && !has_magicshield_effect;
                }

                target.hp -= hp_damage;
                target.flags.insert(CharacterFlags::UPDATE);
                outcome.hp_damage = hp_damage;

                if target.hp < POWERSCALE / 2 {
                    if target.flags.contains(CharacterFlags::NODEATH) {
                        target.hp = 1;
                        outcome.nodeath_saved = true;
                    } else {
                        target.flags.insert(CharacterFlags::DEAD);
                        target.flags.remove(CharacterFlags::ALIVE);
                        target.deaths = target.deaths.saturating_add(1);
                        outcome.killed = true;
                    }
                }
            }

            target.regen_ticker = self.tick.0.min(u64::from(u32::MAX)) as u32;

            target.push_driver_message(
                NT_GOTHIT,
                cause_id.map(|id| id.0 as i32).unwrap_or_default(),
                outcome.hp_damage,
                0,
            );
            (target.x, target.y, target_was_player, target_was_male)
        };

        self.pending_system_texts
            .extend(
                show_attack_messages
                    .into_iter()
                    .map(|message| WorldSystemText {
                        character_id: target_id,
                        message,
                    }),
            );

        if target_was_player && outcome.hp_damage >= POWERSCALE {
            self.queue_sound_area(
                usize::from(target_x),
                usize::from(target_y),
                if target_was_male { 9 } else { 32 },
            );
        }
        if target_was_player && (outcome.killed || outcome.nodeath_saved) {
            self.queue_sound_area(
                usize::from(target_x),
                usize::from(target_y),
                if target_was_male { 4 } else { 33 },
            );
        }

        if create_magicshield_effect {
            self.create_show_effect(
                EF_MAGICSHIELD,
                target_id,
                self.tick.0 as u32,
                self.tick.0 as u32 + 3,
                16,
                0,
            );
        }

        if let Some(cause_id) = cause_id {
            if let Some(cause) = self.characters.get_mut(&cause_id) {
                cause.push_driver_message(NT_DIDHIT, target_id.0 as i32, outcome.hp_damage, 0);
            }
            self.pending_hurt_events.push(LegacyHurtEvent {
                target_id,
                cause_id,
                outcome,
            });
        }

        if outcome.killed {
            if let Some(cause_id) = cause_id {
                self.apply_character_death_driver(target_id, cause_id);
            }
            for character in self.characters.values_mut() {
                if character.x.abs_diff(target_x) <= 32 && character.y.abs_diff(target_y) <= 32 {
                    character.push_driver_message(
                        NT_DEAD,
                        target_id.0 as i32,
                        cause_id.map(|id| id.0 as i32).unwrap_or_default(),
                        0,
                    );
                }
            }
            // C `kill_char`: respawn registration, killer experience, and
            // the timed AC_DIE death animation.
            self.kill_character_followup(target_id, cause_id);
        }

        for (id, character) in self.characters.iter_mut() {
            if *id == target_id || Some(*id) == cause_id {
                continue;
            }
            if map_dist(character.x, character.y, target_x, target_y) <= 16 {
                character.push_driver_message(
                    NT_SEEHIT,
                    cause_id.map(|id| id.0 as i32).unwrap_or_default(),
                    target_id.0 as i32,
                    0,
                );
            }
        }

        Some(outcome)
    }

    pub fn drain_legacy_hurt_events(&mut self) -> Vec<LegacyHurtEvent> {
        self.pending_hurt_events.drain(..).collect()
    }

    pub fn apply_simple_baddy_death_driver(
        &mut self,
        dead_id: CharacterId,
        killer_id: CharacterId,
    ) -> Vec<u32> {
        let Some(dead) = self.characters.get(&dead_id).cloned() else {
            return Vec::new();
        };
        let Some(killer) = self.characters.get(&killer_id).cloned() else {
            return Vec::new();
        };
        if dead.driver != CDR_SIMPLEBADDY
            || !dead.flags.contains(CharacterFlags::EDEMON)
            || !char_see_char(&dead, &killer, &self.map, self.date.daylight)
        {
            return Vec::new();
        }

        let strength = character_value_present(&dead, CharacterValue::Demon);
        let mut effects = Vec::new();
        if strength > 5 {
            effects.push(self.create_earthmud_effect(
                i32::from(killer.x),
                i32::from(killer.y),
                strength,
            ));
        }
        effects.push(self.create_earthrain_effect(
            i32::from(killer.x),
            i32::from(killer.y),
            strength,
        ));
        effects
    }

    pub fn apply_character_death_driver(
        &mut self,
        dead_id: CharacterId,
        killer_id: CharacterId,
    ) -> Vec<u32> {
        let Some(driver) = self
            .characters
            .get(&dead_id)
            .map(|character| character.driver)
        else {
            return Vec::new();
        };

        match execute_character_died_driver(driver, killer_id.0) {
            CharacterDriverOutcome::SimpleBaddyDeath {
                killer_character_id,
            } => self.apply_simple_baddy_death_driver(dead_id, CharacterId(killer_character_id)),
            CharacterDriverOutcome::HandledStub { .. }
            | CharacterDriverOutcome::Unsupported { .. } => Vec::new(),
        }
    }

    pub fn apply_swamp_monster_death_driver(
        &mut self,
        dead_id: CharacterId,
        killer_id: CharacterId,
    ) -> bool {
        let Some(dead) = self.characters.get(&dead_id) else {
            return false;
        };
        if dead.driver != CDR_SWAMPMONSTER {
            return false;
        }
        let Some(killer) = self.characters.get(&killer_id) else {
            return false;
        };
        if !killer.flags.contains(CharacterFlags::PLAYER) {
            return false;
        }

        let bit = match (killer.x, killer.y) {
            (142..=153, 83..=92) => 1,
            (34..=44, 150..=160) => 2,
            (183..=192, 154..=162) => 4,
            _ => 0,
        };
        if self.date.hour != 0 || bit == 0 {
            return false;
        }

        let Some(item_id) = killer.inventory[worn_slot::RIGHT_HAND] else {
            return false;
        };
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        if item.driver != 0 || item.driver_data.get(36).copied().unwrap_or_default() & bit != 0 {
            return false;
        }
        if item.driver_data.len() <= 37 {
            item.driver_data.resize(38, 0);
        }
        item.template_id = IID_HARDKILL;
        item.driver_data[37] = item.driver_data[37].saturating_add(12);
        item.driver_data[36] |= bit;
        item.flags.insert(ItemFlags::QUEST);
        self.pending_system_texts.push(WorldSystemText {
            character_id: killer_id,
            message: format!("Your {} starts to glow.", item.name),
        });
        true
    }
}
