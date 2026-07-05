use super::*;

impl World {
    pub fn clear_character_spell_slots_and_effects(&mut self, character_id: CharacterId) {
        let spell_items = self
            .characters
            .get(&character_id)
            .map(|character| {
                character.inventory[12..30]
                    .iter()
                    .flatten()
                    .copied()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for item_id in spell_items {
            self.destroy_item(item_id);
        }

        let effect_ids = self
            .effects
            .iter()
            .filter_map(|(&effect_id, effect)| {
                (effect.target_character == Some(character_id)).then_some(effect_id)
            })
            .collect::<Vec<_>>();
        for effect_id in effect_ids {
            self.remove_effect_from_map(effect_id);
            self.effects.remove(&effect_id);
        }
    }

    pub(crate) fn setup_fireball_character(
        &mut self,
        caster_id: CharacterId,
        target_id: CharacterId,
        target_serial: u32,
    ) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        if !target.flags.contains(CharacterFlags::USED) {
            return false;
        }
        if target_serial != 0 && target.id.0 != target_serial {
            return false;
        }

        let (target_x, target_y) = predicted_fireball_target(&caster, &target);
        let current_tick = self.tick.0 as u32;
        let weather_movement_percent = self.settings.weather_movement_percent;
        self.characters.get_mut(&caster_id).is_some_and(|caster| {
            do_fireball(
                caster,
                &self.items,
                target_x,
                target_y,
                current_tick,
                &self.map,
                weather_movement_percent,
            )
            .is_ok()
        })
    }

    pub(crate) fn setup_ball_character(
        &mut self,
        caster_id: CharacterId,
        target_id: CharacterId,
        target_serial: u32,
    ) -> bool {
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        if !target.flags.contains(CharacterFlags::USED) {
            return false;
        }
        if target_serial != 0 && target.id.0 != target_serial {
            return false;
        }

        let current_tick = self.tick.0 as u32;
        let weather_movement_percent = self.settings.weather_movement_percent;
        self.characters.get_mut(&caster_id).is_some_and(|caster| {
            do_ball(
                caster,
                &self.items,
                usize::from(target.x),
                usize::from(target.y),
                current_tick,
                &self.map,
                weather_movement_percent,
            )
            .is_ok()
        })
    }

    pub(crate) fn setup_bless_spell(
        &mut self,
        caster_id: CharacterId,
        target_id: CharacterId,
    ) -> bool {
        let weather_movement_percent = self.settings.weather_movement_percent;
        if caster_id == target_id {
            let Some(target) = self.characters.get(&target_id).cloned() else {
                return false;
            };
            let current_tick = self.tick.0 as u32;
            return self.characters.get_mut(&caster_id).is_some_and(|caster| {
                do_bless(
                    caster,
                    &target,
                    &self.items,
                    current_tick,
                    None,
                    &self.map,
                    weather_movement_percent,
                )
                .is_ok()
            });
        }

        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(caster) = self.characters.get(&caster_id) else {
            return false;
        };
        let Some(direction) = offset_to_direction(
            usize::from(caster.x),
            usize::from(caster.y),
            usize::from(target.x),
            usize::from(target.y),
        ) else {
            return false;
        };
        let current_tick = self.tick.0 as u32;

        self.characters.get_mut(&caster_id).is_some_and(|caster| {
            do_bless(
                caster,
                &target,
                &self.items,
                current_tick,
                Some(direction as u8),
                &self.map,
                weather_movement_percent,
            )
            .is_ok()
        })
    }

    pub(crate) fn setup_heal_spell(
        &mut self,
        caster_id: CharacterId,
        target_id: CharacterId,
    ) -> bool {
        let weather_movement_percent = self.settings.weather_movement_percent;
        if caster_id == target_id {
            let Some(target) = self.characters.get(&target_id).cloned() else {
                return false;
            };
            return self.characters.get_mut(&caster_id).is_some_and(|caster| {
                do_heal(caster, &target, None, &self.map, weather_movement_percent).is_ok()
            });
        }

        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(caster) = self.characters.get(&caster_id) else {
            return false;
        };
        let Some(direction) = offset_to_direction(
            usize::from(caster.x),
            usize::from(caster.y),
            usize::from(target.x),
            usize::from(target.y),
        ) else {
            return false;
        };

        self.characters.get_mut(&caster_id).is_some_and(|caster| {
            do_heal(
                caster,
                &target,
                Some(direction as u8),
                &self.map,
                weather_movement_percent,
            )
            .is_ok()
        })
    }

    pub(crate) fn complete_bless(
        &mut self,
        caster_id: CharacterId,
        target_id: CharacterId,
    ) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if caster.flags.contains(CharacterFlags::NOMAGIC)
            && !caster.flags.contains(CharacterFlags::NONOMAGIC)
        {
            return false;
        }
        if caster.act1 != target_id.0 as i32 {
            return false;
        }
        let strength = character_value(&caster, CharacterValue::Bless);
        if strength <= 0 {
            return false;
        }
        let duration = spell_duration_ticks(&caster, BLESS_DURATION);
        let installed = self.install_bless_spell(target_id, strength, duration);
        if installed {
            // C `act_bless` (`act.c:1237-1241`): `NT_CHAR` gated on
            // `CF_NONOTIFY`, then unconditional `NT_SPELL`/`sound_area`.
            if !caster.flags.contains(CharacterFlags::NONOTIFY) {
                self.notify_area(caster.x, caster.y, NT_CHAR, caster_id.0 as i32, 0, 0);
            }
            self.notify_area(
                caster.x,
                caster.y,
                NT_SPELL,
                caster_id.0 as i32,
                CharacterValue::Bless as i32,
                0,
            );
            self.queue_sound_area(usize::from(caster.x), usize::from(caster.y), 29);
        }
        installed
    }

    pub(crate) fn complete_flash(&mut self, caster_id: CharacterId) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if caster.flags.contains(CharacterFlags::NOMAGIC)
            && !caster.flags.contains(CharacterFlags::NONOMAGIC)
        {
            return false;
        }
        let duration = spell_duration_ticks(&caster, FLASH_DURATION);
        if !self.install_speed_spell(caster_id, IDR_FLASH, "Flash", 100, duration) {
            return false;
        }
        self.create_show_effect(
            EF_FLASH,
            caster_id,
            self.tick.0 as u32,
            self.tick.0.saturating_add(duration.max(0) as u64) as u32,
            50,
            spell_power(
                character_value(&caster, CharacterValue::Flash),
                character_value(&caster, CharacterValue::Tactics),
            ),
        );
        // C `act_flash` (`act.c:1041-1044`): `NT_CHAR` gated on
        // `CF_NONOTIFY`, then unconditional `NT_SPELL`.
        if !caster.flags.contains(CharacterFlags::NONOTIFY) {
            self.notify_area(caster.x, caster.y, NT_CHAR, caster_id.0 as i32, 0, 0);
        }
        self.notify_area(
            caster.x,
            caster.y,
            NT_SPELL,
            caster_id.0 as i32,
            CharacterValue::Flash as i32,
            0,
        );
        true
    }

    pub(crate) fn complete_fireball(&mut self, caster_id: CharacterId) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if caster.flags.contains(CharacterFlags::NOMAGIC)
            && !caster.flags.contains(CharacterFlags::NONOMAGIC)
        {
            return false;
        }

        let effect_id = self.create_fireball_effect(&caster);
        if effect_id != 0 {
            // C `act_fireball` (`act.c:955-960`): `NT_CHAR` gated on
            // `CF_NONOTIFY`, then unconditional `NT_SPELL`/`sound_area`.
            if !caster.flags.contains(CharacterFlags::NONOTIFY) {
                self.notify_area(caster.x, caster.y, NT_CHAR, caster_id.0 as i32, 0, 0);
            }
            self.notify_area(
                caster.x,
                caster.y,
                NT_SPELL,
                caster_id.0 as i32,
                CharacterValue::Fireball as i32,
                effect_id as i32,
            );
            self.queue_sound_area(usize::from(caster.x), usize::from(caster.y), 5);
        }
        if let Some(caster) = self.characters.get_mut(&caster_id) {
            caster.action = action::FIREBALL2;
            caster.step = 0;
        }
        true
    }

    pub(crate) fn complete_ball(&mut self, caster_id: CharacterId) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if caster.flags.contains(CharacterFlags::NOMAGIC)
            && !caster.flags.contains(CharacterFlags::NONOMAGIC)
        {
            return false;
        }

        let effect_id = self.create_ball_effect(&caster);
        if effect_id != 0 {
            // C `act_ball` (`act.c:1057-1061`): `NT_CHAR` gated on
            // `CF_NONOTIFY`, then unconditional `NT_SPELL` - note C uses
            // `V_FLASH` (not `V_BALL`, there is no such value) as the
            // payload here, matching `create_ball`'s own `spellpower(cn,
            // V_FLASH)` power source.
            if !caster.flags.contains(CharacterFlags::NONOTIFY) {
                self.notify_area(caster.x, caster.y, NT_CHAR, caster_id.0 as i32, 0, 0);
            }
            self.notify_area(
                caster.x,
                caster.y,
                NT_SPELL,
                caster_id.0 as i32,
                CharacterValue::Flash as i32,
                effect_id as i32,
            );
        }
        if let Some(caster) = self.characters.get_mut(&caster_id) {
            caster.action = action::BALL2;
            caster.step = 0;
        }
        true
    }

    pub(crate) fn complete_earthrain(&mut self, caster_id: CharacterId) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if caster.act1 <= 0 {
            return false;
        }
        self.create_earthrain_effect(
            caster.act1 % MAX_MAP as i32,
            caster.act1 / MAX_MAP as i32,
            caster.act2,
        ) != 0
    }

    pub(crate) fn complete_earthmud(&mut self, caster_id: CharacterId) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if caster.act1 <= 0 {
            return false;
        }
        self.create_earthmud_effect(
            caster.act1 % MAX_MAP as i32,
            caster.act1 / MAX_MAP as i32,
            caster.act2,
        ) != 0
    }

    pub(crate) fn complete_firering(
        &mut self,
        caster_id: CharacterId,
        can_attack_target: &mut impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if caster.flags.contains(CharacterFlags::NOMAGIC)
            && !caster.flags.contains(CharacterFlags::NONOMAGIC)
        {
            return false;
        }

        let power = spell_power(
            character_value(&caster, CharacterValue::Fireball),
            character_value(&caster, CharacterValue::Tactics),
        );
        if !self.install_firering_spell(caster_id) {
            return false;
        }
        let effect_id = self.create_show_effect(
            EF_FIRERING,
            caster_id,
            self.tick.0 as u32,
            self.tick.0.saturating_add(7) as u32,
            20,
            50,
        );

        let caster_x = usize::from(caster.x);
        let caster_y = usize::from(caster.y);
        let min_x = caster_x.saturating_sub(1).max(1);
        let max_x = caster_x
            .saturating_add(1)
            .min(self.map.width().saturating_sub(2));
        let min_y = caster_y.saturating_sub(1).max(1);
        let max_y = caster_y
            .saturating_add(1)
            .min(self.map.height().saturating_sub(2));
        let mut targets = Vec::new();

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let Some(target_id) = self.map.tile(x, y).and_then(|tile| {
                    (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
                }) else {
                    continue;
                };
                if target_id == caster_id {
                    continue;
                }
                let Some(target) = self.characters.get(&target_id) else {
                    continue;
                };
                if !can_attack_target(caster_id, &caster, target, &self.map) {
                    continue;
                }
                let has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;
                let damage = fireball_damage(
                    power,
                    character_value(target, CharacterValue::Immunity),
                    character_value(target, CharacterValue::Tactics),
                    has_tactics,
                );
                targets.push((target_id, damage));
            }
        }

        for (target_id, damage) in targets {
            self.create_show_effect(
                EF_BURN,
                target_id,
                self.tick.0 as u32,
                self.tick.0.saturating_add(8) as u32,
                20,
                0,
            );
            self.apply_legacy_hurt(target_id, Some(caster_id), damage, 10, 30, 85);
        }

        // C `act_firering` (`act.c:935-941`): `if (ch[cn].flags)` guards
        // against `hurt` having killed the caster indirectly (e.g. a
        // char-dead driver reflecting damage); ported as a `!DEAD` check,
        // mirroring `complete_attack`'s equivalent guard (`world/combat.rs`).
        if let Some(caster) = self.characters.get(&caster_id) {
            let alive = !caster.flags.contains(CharacterFlags::DEAD);
            let notify_gated = !caster.flags.contains(CharacterFlags::NONOTIFY);
            let (x, y) = (caster.x, caster.y);
            if alive {
                if notify_gated {
                    self.notify_area(x, y, NT_CHAR, caster_id.0 as i32, 0, 0);
                }
                self.notify_area(
                    x,
                    y,
                    NT_SPELL,
                    caster_id.0 as i32,
                    CharacterValue::Fireball as i32,
                    effect_id as i32,
                );
                self.queue_sound_area(caster_x, caster_y, 5);
            }
        }

        true
    }

    pub(crate) fn complete_magicshield(&mut self, character_id: CharacterId) -> bool {
        if !self
            .characters
            .get_mut(&character_id)
            .is_some_and(act_magicshield)
        {
            return false;
        }
        self.create_show_effect(
            EF_MAGICSHIELD,
            character_id,
            self.tick.0 as u32,
            self.tick.0.saturating_add(3) as u32,
            25,
            0,
        );
        // C `act_magicshield` (`act.c:1090-1093`): `NT_CHAR` gated on
        // `CF_NONOTIFY`, then unconditional `NT_SPELL` with a `0` payload
        // (no effect id is passed, unlike fireball/firering/ball).
        if let Some(character) = self.characters.get(&character_id) {
            let (x, y) = (character.x, character.y);
            if !character.flags.contains(CharacterFlags::NONOTIFY) {
                self.notify_area(x, y, NT_CHAR, character_id.0 as i32, 0, 0);
            }
            self.notify_area(
                x,
                y,
                NT_SPELL,
                character_id.0 as i32,
                CharacterValue::MagicShield as i32,
                0,
            );
        }
        true
    }

    pub(crate) fn complete_pulse(
        &mut self,
        caster_id: CharacterId,
        can_attack_target: &mut impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if caster.flags.contains(CharacterFlags::NOMAGIC)
            && !caster.flags.contains(CharacterFlags::NONOMAGIC)
        {
            return false;
        }

        let caster_x = usize::from(caster.x);
        let caster_y = usize::from(caster.y);
        let min_x = caster_x.saturating_sub(2).max(1);
        let max_x = caster_x
            .saturating_add(2)
            .min(self.map.width().saturating_sub(2));
        let min_y = caster_y.saturating_sub(2).max(1);
        let max_y = caster_y
            .saturating_add(2)
            .min(self.map.height().saturating_sub(2));
        let mut targets = Vec::new();

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let Some(target_id) = self.map.tile(x, y).and_then(|tile| {
                    (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
                }) else {
                    continue;
                };
                if target_id == caster_id {
                    continue;
                }
                let Some(target) = self.characters.get(&target_id) else {
                    continue;
                };
                if !can_attack_target(caster_id, &caster, target, &self.map) {
                    continue;
                }
                if !self.map.can_see(caster_x, caster_y, x, y, DIST_MAX) {
                    continue;
                }
                let has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;
                let damage = pulse_damage(
                    character_value(&caster, CharacterValue::Pulse),
                    caster.act1,
                    character_value(target, CharacterValue::Immunity),
                    character_value(target, CharacterValue::Tactics),
                    has_tactics,
                );
                let had = target.hp.saturating_add(target.lifeshield);
                let total = character_value(target, CharacterValue::Hp) * POWERSCALE
                    + character_value(target, CharacterValue::MagicShield) * POWERSCALE
                    + 1;
                if had.saturating_mul(100) / total <= 75 && damage >= had {
                    targets.push((target_id, damage, had));
                }
            }
        }

        for (target_id, damage, had) in targets {
            self.create_pulseback_effect(target_id, caster.x, caster.y, caster.act1);
            if let Some(caster) = self.characters.get_mut(&caster_id) {
                let max_mana = character_value(caster, CharacterValue::Mana) * POWERSCALE;
                caster.mana = max_mana.min(caster.mana.saturating_add(damage.min(had)));
                caster.flags.insert(CharacterFlags::UPDATE);
            }
            self.apply_legacy_hurt(target_id, Some(caster_id), damage, 1, 0, 100);
        }

        self.create_pulse_effect(
            caster.x,
            caster.y,
            character_value(&caster, CharacterValue::Pulse),
        );
        // C `act_pulse` (`act.c:1637-1640`): `NT_CHAR` gated on
        // `CF_NONOTIFY`, then unconditional `NT_SPELL` with a `0` payload.
        if !caster.flags.contains(CharacterFlags::NONOTIFY) {
            self.notify_area(caster.x, caster.y, NT_CHAR, caster_id.0 as i32, 0, 0);
        }
        self.notify_area(
            caster.x,
            caster.y,
            NT_SPELL,
            caster_id.0 as i32,
            CharacterValue::Pulse as i32,
            0,
        );
        true
    }

    pub(crate) fn complete_freeze(
        &mut self,
        caster_id: CharacterId,
        can_attack_target: &mut impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if caster.flags.contains(CharacterFlags::NOMAGIC)
            && !caster.flags.contains(CharacterFlags::NONOMAGIC)
        {
            return false;
        }

        let caster_x = usize::from(caster.x);
        let caster_y = usize::from(caster.y);
        let min_x = caster_x.saturating_sub(3).max(1);
        let max_x = caster_x
            .saturating_add(3)
            .min(self.map.width().saturating_sub(2));
        let min_y = caster_y.saturating_sub(3).max(1);
        let max_y = caster_y
            .saturating_add(3)
            .min(self.map.height().saturating_sub(2));
        let mut targets = Vec::new();

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let Some(target_id) = self.map.tile(x, y).and_then(|tile| {
                    (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
                }) else {
                    continue;
                };
                if target_id == caster_id {
                    continue;
                }
                let Some(target) = self.characters.get(&target_id) else {
                    continue;
                };
                if !can_attack_target(caster_id, &caster, target, &self.map)
                    || !self.map.can_see(caster_x, caster_y, x, y, DIST_MAX)
                {
                    continue;
                }
                let modifier = freeze_speed_modifier(
                    spell_power(
                        character_value(&caster, CharacterValue::Freeze),
                        character_value(&caster, CharacterValue::Tactics),
                    ),
                    character_value(target, CharacterValue::Immunity),
                    character_value(target, CharacterValue::Tactics),
                    character_value_present(target, CharacterValue::Tactics) != 0,
                    caster.flags.contains(CharacterFlags::IDEMON),
                    character_value_present(&caster, CharacterValue::Demon),
                    character_value(target, CharacterValue::Cold),
                );
                if modifier < 0 {
                    targets.push((target_id, modifier));
                }
            }
        }

        let duration = spell_duration_ticks(&caster, FREEZE_DURATION);
        for (target_id, modifier) in targets {
            self.install_speed_spell(target_id, IDR_FREEZE, "Freeze", modifier, duration);
            let Some(target) = self.characters.get(&target_id) else {
                continue;
            };
            let curse_strength = character_value_present(&caster, CharacterValue::Demon)
                - character_value(target, CharacterValue::Cold);
            if caster.flags.contains(CharacterFlags::IDEMON) && curse_strength > 0 {
                if self.install_curse_spell(target_id, curse_strength, curse_strength * 50) {
                    self.pending_system_texts.push(WorldSystemText {
                        character_id: target_id,
                        message: format!(
                            "You have been frozen by {}. You feel like you'll never thaw again.",
                            caster.name
                        ),
                    });
                }
            }
        }
        // C `act_freeze` (`act.c:1556-1560`): `NT_CHAR` gated on
        // `CF_NONOTIFY`, then unconditional `NT_SPELL`/`sound_area`.
        if !caster.flags.contains(CharacterFlags::NONOTIFY) {
            self.notify_area(caster.x, caster.y, NT_CHAR, caster_id.0 as i32, 0, 0);
        }
        self.notify_area(
            caster.x,
            caster.y,
            NT_SPELL,
            caster_id.0 as i32,
            CharacterValue::Freeze as i32,
            0,
        );
        self.queue_sound_area(caster_x, caster_y, 31);
        true
    }

    pub(crate) fn complete_warcry(
        &mut self,
        caster_id: CharacterId,
        can_attack_target: &mut impl FnMut(CharacterId, &Character, &Character, &MapGrid) -> bool,
    ) -> bool {
        let Some(caster) = self.characters.get(&caster_id).cloned() else {
            return false;
        };
        if caster.flags.contains(CharacterFlags::NOMAGIC)
            && !caster.flags.contains(CharacterFlags::NONOMAGIC)
        {
            return false;
        }

        let caster_x = usize::from(caster.x);
        let caster_y = usize::from(caster.y);
        let min_x = caster_x.saturating_sub(10).max(1);
        let max_x = caster_x
            .saturating_add(10)
            .min(self.map.width().saturating_sub(2));
        let min_y = caster_y.saturating_sub(10).max(1);
        let max_y = caster_y
            .saturating_add(10)
            .min(self.map.height().saturating_sub(2));
        let sectors = SoundSectors::build(&self.map);
        let power = spell_power(
            character_value(&caster, CharacterValue::Warcry),
            character_value(&caster, CharacterValue::Tactics),
        );
        let duration = spell_duration_ticks(&caster, WARCRY_DURATION);
        let mut targets = Vec::new();

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let Some(target_id) = self.map.tile(x, y).and_then(|tile| {
                    (tile.character != 0).then_some(CharacterId(u32::from(tile.character)))
                }) else {
                    continue;
                };
                if target_id == caster_id
                    || !sectors.sector_hear(&self.map, caster_x, caster_y, x, y)
                {
                    continue;
                }
                let Some(target) = self.characters.get(&target_id) else {
                    continue;
                };
                if !can_attack_target(caster_id, &caster, target, &self.map) {
                    continue;
                }

                let has_tactics = character_value_present(target, CharacterValue::Tactics) != 0;
                let modifier = warcry_speed_modifier(
                    power,
                    character_value(target, CharacterValue::Immunity),
                    character_value(target, CharacterValue::Tactics),
                    has_tactics,
                );
                if modifier >= 0 {
                    continue;
                }
                let damage = warcry_damage(
                    power,
                    character_value(target, CharacterValue::Immunity),
                    character_value(target, CharacterValue::Tactics),
                    has_tactics,
                );
                targets.push((target_id, modifier, damage));
            }
        }

        for (target_id, modifier, damage) in targets {
            if !self.install_speed_spell(target_id, IDR_WARCRY, "Warcry", modifier, duration) {
                continue;
            }
            if damage > 0 {
                self.apply_legacy_hurt(target_id, Some(caster_id), damage, 1, 0, 0);
            }
        }

        // C `act_warcry` (`act.c:1399-1402`): `NT_CHAR` gated on
        // `CF_NONOTIFY`, then unconditional `NT_SPELL`, before the
        // lifeshield-grant tail below.
        if !caster.flags.contains(CharacterFlags::NONOTIFY) {
            self.notify_area(caster.x, caster.y, NT_CHAR, caster_id.0 as i32, 0, 0);
        }
        self.notify_area(
            caster.x,
            caster.y,
            NT_SPELL,
            caster_id.0 as i32,
            CharacterValue::Warcry as i32,
            0,
        );

        if character_value_present(&caster, CharacterValue::MagicShield) == 0 {
            if let Some(caster) = self.characters.get_mut(&caster_id) {
                let max_lifeshield = if character_value(caster, CharacterValue::MagicShield) != 0 {
                    character_value(caster, CharacterValue::MagicShield)
                } else {
                    character_value(caster, CharacterValue::Warcry)
                } * crate::entity::POWERSCALE;
                let gain =
                    character_value(caster, CharacterValue::Warcry) * crate::entity::POWERSCALE / 2;
                caster.lifeshield = max_lifeshield.min(caster.lifeshield + gain);
                caster.flags.insert(CharacterFlags::UPDATE);
            }
        }

        true
    }

    pub(crate) fn install_bless_spell(
        &mut self,
        target_id: CharacterId,
        strength: i32,
        duration: i32,
    ) -> bool {
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&target, &self.items, IDR_BLESS, self.tick.0 as u32) else {
            return false;
        };
        let old_item_id = target.inventory.get(slot).copied().flatten();
        if let Some(item_id) = old_item_id {
            self.items.remove(&item_id);
            self.remove_show_effect_type(target_id, EF_BLESS);
            // C `bless_someone`/`bless_self` (`act.c:1113-1117`,
            // `act.c:1156-1158`): `update_char(co)` right after destroying
            // the pre-existing bless item.
            self.update_character(target_id);
        }

        let item_id = self.next_runtime_item_id();
        let mut driver_data = Vec::with_capacity(12);
        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add(duration.max(0) as u32);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());
        driver_data.extend_from_slice(&strength.to_le_bytes());

        let item = Item {
            id: item_id,
            name: "Bless".to_string(),
            description: "A Spell of Bless.".to_string(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [
                CharacterValue::Intelligence as i16,
                CharacterValue::Wisdom as i16,
                CharacterValue::Agility as i16,
                CharacterValue::Strength as i16,
                0,
            ],
            modifier_value: [
                (strength / 4) as i16,
                (strength / 4) as i16,
                (strength / 4) as i16,
                (strength / 4) as i16,
                0,
            ],
            x: 0,
            y: 0,
            carried_by: Some(target_id),
            contained_in: None,
            content_id: 0,
            driver: IDR_BLESS,
            driver_data,
            serial: item_id.0,
        };

        self.items.insert(item_id, item);
        let character_serial;
        if let Some(target) = self.characters.get_mut(&target_id) {
            if target.inventory.len() <= slot {
                return false;
            }
            target.inventory[slot] = Some(item_id);
            character_serial = target.id.0;
            target
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        } else {
            return false;
        }
        // C `bless_someone`/`bless_self`: `update_char(co)` again after
        // installing the new bless item.
        self.update_character(target_id);
        self.schedule_spell_remove_timer(target_id, item_id, slot, character_serial, item_id.0);
        self.create_show_effect(
            EF_BLESS,
            target_id,
            start_tick,
            expire_tick,
            0,
            strength / 4,
        );
        true
    }

    /// C `/killbless` (`command.c:9605-9617`, `cmdcmp(ptr, "killbless",
    /// 5)`, no permission gate): scans equip slots 12..30 (`SPELL_SLOT_
    /// START..SPELL_SLOT_END`) for the first item with `IDR_BLESS`
    /// driver; if found, removes the `EF_BLESS` show-effect, destroys the
    /// item (which also clears the inventory slot, C's `ch[cn].item[n] =
    /// 0`), calls `update_char`, and returns `true` (caller logs "Done.").
    /// Returns `false` when no bless item is present (caller logs "No
    /// Bless found.").
    pub fn kill_bless_item(&mut self, character_id: CharacterId) -> bool {
        let Some(character) = self.characters.get(&character_id) else {
            return false;
        };
        let bless_item_id = character.inventory[SPELL_SLOT_START..SPELL_SLOT_END]
            .iter()
            .flatten()
            .find(|&&item_id| {
                self.items
                    .get(&item_id)
                    .is_some_and(|item| item.driver == IDR_BLESS)
            })
            .copied();

        let Some(item_id) = bless_item_id else {
            return false;
        };

        self.remove_show_effect_type(character_id, EF_BLESS);
        self.destroy_item(item_id);
        self.update_character(character_id);
        true
    }

    pub fn install_bonus_spell(
        &mut self,
        target_id: CharacterId,
        driver: u16,
        strength: i32,
        duration: i32,
    ) -> bool {
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&target, &self.items, driver, self.tick.0 as u32) else {
            return false;
        };
        let Some((name, modifier_index)) = bonus_spell_shape(driver) else {
            return false;
        };

        let item_id = self.next_runtime_item_id();
        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add(duration.max(0) as u32);
        let mut driver_data = Vec::with_capacity(4);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());

        let item = Item {
            id: item_id,
            name: name.to_string(),
            description: format!("A Spell of {name}."),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [modifier_index as i16, 0, 0, 0, 0],
            modifier_value: [
                strength.clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                0,
                0,
                0,
                0,
            ],
            x: 0,
            y: 0,
            carried_by: Some(target_id),
            contained_in: None,
            content_id: 0,
            driver,
            driver_data,
            serial: item_id.0,
        };

        self.items.insert(item_id, item);
        let character_serial;
        if let Some(target) = self.characters.get_mut(&target_id) {
            if target.inventory.len() <= slot {
                self.items.remove(&item_id);
                return false;
            }
            target.inventory[slot] = Some(item_id);
            character_serial = target.id.0;
            target
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        } else {
            self.items.remove(&item_id);
            return false;
        }
        // C `add_bonus_spell` (`src/system/drvlib.c:2646`): `update_char(cn)`.
        self.update_character(target_id);
        self.schedule_spell_remove_timer(target_id, item_id, slot, character_serial, item_id.0);
        true
    }

    pub(crate) fn install_beyond_potion_spell(
        &mut self,
        character_id: CharacterId,
        potion_item_id: ItemId,
        duration_minutes: u8,
        modifier_index: [i16; MAX_MODIFIERS],
        modifier_value: [i16; MAX_MODIFIERS],
        beyond_max_mod: bool,
        consume_source_item: bool,
    ) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&character, &self.items, IDR_POTION_SP, self.tick.0 as u32)
        else {
            return false;
        };
        if !self
            .items
            .get(&potion_item_id)
            .is_some_and(|item| item.carried_by == Some(character_id))
        {
            return false;
        }

        let item_id = self.next_runtime_item_id();
        let start_tick = self.tick.0 as u32;
        let duration_ticks = u32::from(duration_minutes) * 60 * TICKS_PER_SECOND as u32;
        let expire_tick = start_tick.wrapping_add(duration_ticks);
        let mut driver_data = Vec::with_capacity(8);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());

        let mut flags = ItemFlags::USED;
        if beyond_max_mod {
            flags.insert(ItemFlags::BEYONDMAXMOD);
        }
        let item = Item {
            id: item_id,
            name: "Potion Spell".to_string(),
            description: "A potion spell.".to_string(),
            flags,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index,
            modifier_value,
            x: 0,
            y: 0,
            carried_by: Some(character_id),
            contained_in: None,
            content_id: 0,
            driver: IDR_POTION_SP,
            driver_data,
            serial: item_id.0,
        };

        if consume_source_item && !self.destroy_item(potion_item_id) {
            return false;
        }
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
        // C `add_potion_spell` (`src/module/alchemy.c:1007`): `update_char(cn)`.
        self.update_character(character_id);
        self.schedule_spell_remove_timer(character_id, item_id, slot, character_serial, item_id.0);
        self.create_show_effect(
            EF_POTION,
            character_id,
            start_tick,
            expire_tick,
            0,
            i32::from(modifier_value[0]),
        );
        true
    }

    pub(crate) fn install_speed_spell(
        &mut self,
        target_id: CharacterId,
        driver: u16,
        name: &str,
        speed_modifier: i32,
        duration: i32,
    ) -> bool {
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&target, &self.items, driver, self.tick.0 as u32) else {
            return false;
        };

        let item_id = self.next_runtime_item_id();
        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add(duration.max(0) as u32);
        let mut driver_data = Vec::with_capacity(8);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());

        let item = Item {
            id: item_id,
            name: name.to_string(),
            description: format!("A Spell of {name}."),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [CharacterValue::Speed as i16, 0, 0, 0, 0],
            modifier_value: [speed_modifier as i16, 0, 0, 0, 0],
            x: 0,
            y: 0,
            carried_by: Some(target_id),
            contained_in: None,
            content_id: 0,
            driver,
            driver_data,
            serial: item_id.0,
        };

        self.items.insert(item_id, item);
        let character_serial;
        if let Some(target) = self.characters.get_mut(&target_id) {
            if target.inventory.len() <= slot {
                self.items.remove(&item_id);
                return false;
            }
            target.inventory[slot] = Some(item_id);
            character_serial = target.id.0;
            target
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        } else {
            self.items.remove(&item_id);
            return false;
        }
        // C `warcry_someone`/`freeze_someone` (`act.c:1324`, `act.c:1522`):
        // `update_char(co)`.
        self.update_character(target_id);
        self.schedule_spell_remove_timer(target_id, item_id, slot, character_serial, item_id.0);
        match driver {
            IDR_FREEZE => {
                self.create_show_effect(EF_FREEZE, target_id, start_tick, expire_tick, 0, 0);
            }
            IDR_WARCRY => {
                self.create_show_effect(EF_WARCRY, target_id, start_tick, expire_tick, 0, 0);
            }
            _ => {}
        }
        true
    }

    pub(crate) fn install_curse_spell(
        &mut self,
        target_id: CharacterId,
        strength: i32,
        max_strength: i32,
    ) -> bool {
        if strength <= 0 || max_strength <= 0 {
            return false;
        }
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(slot) = add_same_spell_slot(&target, &self.items, IDR_CURSE) else {
            return false;
        };

        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add((30 * 60 * TICKS_PER_SECOND) as u32);
        if let Some(item_id) = target.inventory.get(slot).copied().flatten() {
            let Some(item) = self.items.get_mut(&item_id) else {
                return false;
            };
            let current_strength = -i32::from(item.modifier_value[0]);
            if current_strength >= max_strength {
                return false;
            }
            let added_strength = strength.min(max_strength - current_strength);
            for value in &mut item.modifier_value[..4] {
                *value = (i32::from(*value) - added_strength)
                    .clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            }
            let mut missing_effect = None;
            if let Some(effect) = self.effects.values_mut().find(|effect| {
                effect.effect_type == EF_CURSE && effect.target_character == Some(target_id)
            }) {
                effect.strength += added_strength;
            } else {
                missing_effect = Some((
                    read_spell_start_tick(&item.driver_data).unwrap_or(start_tick),
                    read_spell_expire_tick(&item.driver_data).unwrap_or(expire_tick),
                    -i32::from(item.modifier_value[0]),
                ));
            }
            if let Some((effect_start, effect_stop, effect_strength)) = missing_effect {
                self.create_show_effect(
                    EF_CURSE,
                    target_id,
                    effect_start,
                    effect_stop,
                    0,
                    effect_strength,
                );
            }
            if let Some(target) = self.characters.get_mut(&target_id) {
                target
                    .flags
                    .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
            }
            // C `ice_curse` (`src/system/act.c:1470`): `update_char(co)`
            // unconditionally at the end, for both the existing-item and
            // new-item branches.
            self.update_character(target_id);
            return true;
        }

        let item_id = self.next_runtime_item_id();
        let mut driver_data = Vec::with_capacity(8);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());
        let item = Item {
            id: item_id,
            name: "Curse".to_string(),
            description: "A Spell of Curse.".to_string(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [
                CharacterValue::Intelligence as i16,
                CharacterValue::Wisdom as i16,
                CharacterValue::Agility as i16,
                CharacterValue::Strength as i16,
                0,
            ],
            modifier_value: [
                (-strength).clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                (-strength).clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                (-strength).clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                (-strength).clamp(i16::MIN as i32, i16::MAX as i32) as i16,
                0,
            ],
            x: 0,
            y: 0,
            carried_by: Some(target_id),
            contained_in: None,
            content_id: 0,
            driver: IDR_CURSE,
            driver_data,
            serial: item_id.0,
        };

        self.items.insert(item_id, item);
        let character_serial;
        if let Some(target) = self.characters.get_mut(&target_id) {
            if target.inventory.len() <= slot {
                self.items.remove(&item_id);
                return false;
            }
            target.inventory[slot] = Some(item_id);
            character_serial = target.id.0;
            target
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        } else {
            self.items.remove(&item_id);
            return false;
        }
        self.update_character(target_id);
        self.schedule_spell_remove_timer(target_id, item_id, slot, character_serial, item_id.0);
        self.create_show_effect(EF_CURSE, target_id, start_tick, expire_tick, 0, strength);
        true
    }

    pub(crate) fn install_firering_spell(&mut self, target_id: CharacterId) -> bool {
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&target, &self.items, IDR_FIRERING, self.tick.0 as u32)
        else {
            return false;
        };

        let item_id = self.next_runtime_item_id();
        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add(crate::tick::TICKS_PER_SECOND as u32);
        let mut driver_data = Vec::with_capacity(8);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());

        let item = Item {
            id: item_id,
            name: "Firering".to_string(),
            description: "A Spell of Firering.".to_string(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [0, 0, 0, 0, 0],
            modifier_value: [0, 0, 0, 0, 0],
            x: 0,
            y: 0,
            carried_by: Some(target_id),
            contained_in: None,
            content_id: 0,
            driver: IDR_FIRERING,
            driver_data,
            serial: item_id.0,
        };

        self.items.insert(item_id, item);
        let character_serial;
        if let Some(target) = self.characters.get_mut(&target_id) {
            if target.inventory.len() <= slot {
                self.items.remove(&item_id);
                return false;
            }
            target.inventory[slot] = Some(item_id);
            character_serial = target.id.0;
            target
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        } else {
            self.items.remove(&item_id);
            return false;
        }
        // Note: C `act_firering` does not call `update_char` (the item
        // carries no modifiers), but `update_character` is a superset of
        // the old `refresh_driver_spell_flags` call this replaces and is a
        // no-op for firering's flags, so this keeps identical observable
        // behavior while removing the last direct `refresh_driver_spell_flags`
        // call site in this file.
        self.update_character(target_id);
        self.schedule_spell_remove_timer(target_id, item_id, slot, character_serial, item_id.0);
        true
    }

    pub(crate) fn install_infravision_spell(&mut self, target_id: CharacterId) -> bool {
        self.install_timed_identity_spell(
            target_id,
            IDR_INFRARED,
            TICKS_PER_SECOND * 60 * 10,
            "Infravision",
            "A Spell of Infravision.",
        )
    }

    pub(crate) fn install_oxygen_spell(&mut self, target_id: CharacterId) -> bool {
        self.install_oxygen_spell_for_ticks(target_id, TICKS_PER_SECOND * 60)
    }

    pub(crate) fn install_oxygen_spell_for_ticks(
        &mut self,
        target_id: CharacterId,
        duration_ticks: u64,
    ) -> bool {
        self.install_timed_identity_spell(
            target_id,
            IDR_OXYGEN,
            duration_ticks,
            "Oxygen",
            "A Spell of Oxygen.",
        )
    }

    pub(crate) fn apply_lab3_whiteberry(
        &mut self,
        target_id: CharacterId,
        light_power: i16,
    ) -> (bool, bool) {
        if light_power <= 0 {
            return (false, false);
        }

        let existing_light_id = self.characters.get(&target_id).and_then(|character| {
            character.inventory[SPELL_SLOT_START..SPELL_SLOT_END]
                .iter()
                .filter_map(|slot| *slot)
                .find(|item_id| {
                    self.items.get(item_id).is_some_and(|item| {
                        item.driver == IDR_LAB3_PLANT && item.driver_data.first() == Some(&10)
                    })
                })
        });

        if let Some(item_id) = existing_light_id {
            let Some(old_item_light) = self.items.get(&item_id).map(|item| item.modifier_value[0])
            else {
                return (false, false);
            };
            let new_item_light = old_item_light.saturating_add(light_power).min(255);
            if let Some(item) = self.items.get_mut(&item_id) {
                item.modifier_value[0] = new_item_light;
            }
            let old_character_light = self
                .characters
                .get(&target_id)
                .map(character_light_value)
                .unwrap_or_default();
            if let Some(character) = self.characters.get_mut(&target_id) {
                if let Some(light) = character
                    .values
                    .get_mut(0)
                    .and_then(|values| values.get_mut(CharacterValue::Light as usize))
                {
                    *light = light.saturating_add(new_item_light - old_item_light);
                }
                character
                    .flags
                    .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
            }
            self.refresh_character_light_after_value_change(target_id, old_character_light);
            return (true, false);
        }

        let Some(slot) = self.characters.get(&target_id).and_then(|character| {
            character.inventory[SPELL_SLOT_START..SPELL_SLOT_END]
                .iter()
                .rposition(|slot| slot.is_none())
                .map(|offset| SPELL_SLOT_START + offset)
        }) else {
            return (false, false);
        };

        let item_light = light_power.saturating_mul(4).saturating_div(3).min(255);
        if item_light <= 0 {
            return (false, false);
        }

        let item_id = self.next_runtime_item_id();
        let item = Item {
            id: item_id,
            name: "Whiteberry Light".to_string(),
            description: "A whiteberry light spell.".to_string(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [CharacterValue::Light as i16, 0, 0, 0, 0],
            modifier_value: [item_light, 0, 0, 0, 0],
            x: 0,
            y: 0,
            carried_by: Some(target_id),
            contained_in: None,
            content_id: 0,
            driver: IDR_LAB3_PLANT,
            driver_data: vec![10, 0, 0, item_light as u8],
            serial: item_id.0,
        };

        let old_character_light = self
            .characters
            .get(&target_id)
            .map(character_light_value)
            .unwrap_or_default();
        self.items.insert(item_id, item);
        if let Some(character) = self.characters.get_mut(&target_id) {
            if character.inventory.len() <= slot {
                self.items.remove(&item_id);
                return (false, false);
            }
            character.inventory[slot] = Some(item_id);
            if let Some(light) = character
                .values
                .get_mut(0)
                .and_then(|values| values.get_mut(CharacterValue::Light as usize))
            {
                *light = light.saturating_add(item_light);
            }
            character
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        } else {
            self.items.remove(&item_id);
            return (false, false);
        }
        self.refresh_character_light_after_value_change(target_id, old_character_light);
        self.schedule_item_driver_timer_with_context(
            item_id,
            CharacterId(0),
            20 * TICKS_PER_SECOND,
            true,
        );
        (true, true)
    }

    pub(crate) fn decay_lab3_whiteberry_light(&mut self, item_id: ItemId) -> bool {
        let Some((target_id, old_item_light)) = self.items.get(&item_id).and_then(|item| {
            (item.driver == IDR_LAB3_PLANT && item.driver_data.first() == Some(&10))
                .then_some((item.carried_by?, item.modifier_value[0]))
        }) else {
            return false;
        };
        let old_character_light = self
            .characters
            .get(&target_id)
            .map(character_light_value)
            .unwrap_or_default();
        let new_item_light = 3 * old_item_light / 4;

        if new_item_light < 8 {
            if let Some(character) = self.characters.get_mut(&target_id) {
                if let Some(light) = character
                    .values
                    .get_mut(0)
                    .and_then(|values| values.get_mut(CharacterValue::Light as usize))
                {
                    *light = light.saturating_sub(old_item_light);
                }
                character
                    .flags
                    .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
            }
            self.destroy_item(item_id);
            self.refresh_character_light_after_value_change(target_id, old_character_light);
            return true;
        }

        if let Some(item) = self.items.get_mut(&item_id) {
            item.modifier_value[0] = new_item_light;
            if item.driver_data.len() < 4 {
                item.driver_data.resize(4, 0);
            }
            item.driver_data[3] = new_item_light as u8;
        }
        if let Some(character) = self.characters.get_mut(&target_id) {
            if let Some(light) = character
                .values
                .get_mut(0)
                .and_then(|values| values.get_mut(CharacterValue::Light as usize))
            {
                *light = light.saturating_add(new_item_light - old_item_light);
            }
            character.flags.insert(CharacterFlags::UPDATE);
        }
        self.refresh_character_light_after_value_change(target_id, old_character_light);
        self.schedule_item_driver_timer_with_context(
            item_id,
            CharacterId(0),
            20 * TICKS_PER_SECOND,
            true,
        );
        false
    }

    pub(crate) fn install_underwater_talk_spell(
        &mut self,
        target_id: CharacterId,
        duration_ticks: u64,
    ) -> bool {
        self.install_timed_identity_spell(
            target_id,
            IDR_UWTALK,
            duration_ticks,
            "Underwater Talk",
            "A Spell of Underwater Talk.",
        )
    }

    pub(crate) fn install_timed_identity_spell(
        &mut self,
        target_id: CharacterId,
        driver: u16,
        duration_ticks: u64,
        name: &str,
        description: &str,
    ) -> bool {
        if duration_ticks == 0 {
            return false;
        }
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return false;
        };
        let Some(slot) = may_add_spell(&target, &self.items, driver, self.tick.0 as u32) else {
            return false;
        };

        let item_id = self.next_runtime_item_id();
        let start_tick = self.tick.0 as u32;
        let expire_tick = start_tick.wrapping_add(duration_ticks as u32);
        let mut driver_data = Vec::with_capacity(8);
        driver_data.extend_from_slice(&expire_tick.to_le_bytes());
        driver_data.extend_from_slice(&start_tick.to_le_bytes());

        let item = Item {
            id: item_id,
            name: name.to_string(),
            description: description.to_string(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [0, 0, 0, 0, 0],
            modifier_value: [0, 0, 0, 0, 0],
            x: 0,
            y: 0,
            carried_by: Some(target_id),
            contained_in: None,
            content_id: 0,
            driver,
            driver_data,
            serial: item_id.0,
        };

        self.items.insert(item_id, item);
        let character_serial;
        if let Some(target) = self.characters.get_mut(&target_id) {
            if target.inventory.len() <= slot {
                self.items.remove(&item_id);
                return false;
            }
            target.inventory[slot] = Some(item_id);
            character_serial = target.id.0;
            target
                .flags
                .insert(CharacterFlags::ITEMS | CharacterFlags::UPDATE);
        } else {
            self.items.remove(&item_id);
            return false;
        }
        // C `add_spell` (`src/system/tool.c:1683`): `update_char(cn)`.
        self.update_character(target_id);
        self.schedule_spell_remove_timer(target_id, item_id, slot, character_serial, item_id.0);
        true
    }

    pub(crate) fn remove_driver_spells(&mut self, target_id: CharacterId, driver: u16) {
        let item_ids: Vec<ItemId> = self
            .characters
            .get(&target_id)
            .map(|character| {
                character.inventory[SPELL_SLOT_START..SPELL_SLOT_END]
                    .iter()
                    .filter_map(|slot| *slot)
                    .filter(|item_id| {
                        self.items
                            .get(item_id)
                            .is_some_and(|item| item.driver == driver)
                    })
                    .collect()
            })
            .unwrap_or_default();

        let any_removed = !item_ids.is_empty();
        for item_id in item_ids {
            self.destroy_item(item_id);
        }
        // Mirrors C's `remove_poison`/`remove_all_poison` pattern
        // (`src/system/poison.c:117-150`): `update_char(cn)` only if an
        // item was actually removed.
        if any_removed {
            self.update_character(target_id);
        }
    }

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

pub(crate) fn read_poison_power(driver_data: &[u8]) -> Option<u16> {
    let bytes = driver_data.get(8..10)?;
    Some(u16::from_le_bytes(bytes.try_into().ok()?))
}

pub(crate) fn read_spell_start_tick(driver_data: &[u8]) -> Option<u32> {
    let bytes = driver_data.get(4..8)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

pub(crate) fn read_poison_tick(driver_data: &[u8]) -> Option<u16> {
    let bytes = driver_data.get(10..12)?;
    Some(u16::from_le_bytes(bytes.try_into().ok()?))
}

pub(crate) fn write_poison_tick(driver_data: &mut Vec<u8>, tick: u16) {
    driver_data.resize(12, 0);
    driver_data[10..12].copy_from_slice(&tick.to_le_bytes());
}

pub(crate) fn spell_duration_ticks(character: &Character, base_duration: i32) -> i32 {
    if character_value_present(character, CharacterValue::Duration) != 0 {
        base_duration + base_duration * character_value(character, CharacterValue::Duration) / 35
    } else if character.flags.contains(CharacterFlags::ARCH) {
        base_duration + base_duration * character.level as i32 / 35 / 2
    } else {
        base_duration
    }
}
