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
            if caster.flags.contains(CharacterFlags::IDEMON)
                && curse_strength > 0
                && self.install_curse_spell(target_id, curse_strength, curse_strength * 50)
            {
                self.pending_system_texts.push(WorldSystemText {
                    character_id: target_id,
                    message: format!(
                        "You have been frozen by {}. You feel like you'll never thaw again.",
                        caster.name
                    ),
                });
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
}
