use super::*;

pub(crate) struct RuntimePlayerAttackPolicy<'a> {
    pub(crate) attacker_runtime: &'a PlayerRuntime,
}

impl ClanAttackPolicy for RuntimePlayerAttackPolicy<'_> {
    fn has_pk_hate(&self, _attacker: &Character, defender: &Character) -> bool {
        self.attacker_runtime.has_pk_hate_for(defender.id.0)
    }
}

impl World {
    pub fn complete_attack_with_rolls(
        &mut self,
        attacker_id: CharacterId,
        defender_id: CharacterId,
        d100_roll: i32,
        d6_roll: i32,
    ) -> bool {
        self.complete_attack_with_rolls_and_clash_roll(
            attacker_id,
            defender_id,
            d100_roll,
            d6_roll,
            d100_roll.rem_euclid(2),
        )
    }

    pub fn complete_attack_with_rolls_and_clash_roll(
        &mut self,
        attacker_id: CharacterId,
        defender_id: CharacterId,
        d100_roll: i32,
        d6_roll: i32,
        clash_roll: i32,
    ) -> bool {
        if attacker_id == defender_id {
            return false;
        }
        let Some((attacker_x, attacker_y, attacker_rhand)) =
            self.characters.get(&attacker_id).map(|attacker| {
                (
                    usize::from(attacker.x),
                    usize::from(attacker.y),
                    attacker.inventory[worn_slot::RIGHT_HAND].is_some(),
                )
            })
        else {
            return false;
        };
        let Some(mut defender) = self.characters.remove(&defender_id) else {
            return false;
        };
        let defender_rhand = defender.inventory[worn_slot::RIGHT_HAND].is_some();
        let resolution = self.characters.get_mut(&attacker_id).and_then(|attacker| {
            act_attack(attacker, &mut defender, &self.map, d100_roll, d6_roll)
        });
        self.characters.insert(defender_id, defender);
        let Some(resolution) = resolution else {
            return false;
        };
        if self.show_attack_debug {
            if let Some(defender) = self.characters.get(&defender_id) {
                self.pending_system_texts.push(WorldSystemText {
                    character_id: attacker_id,
                    message: format!(
                        "attack {}, diff={} ({} {}), chan={}, percent={}, dam={}",
                        defender.name,
                        resolution.attack_skill - resolution.parry_skill,
                        resolution.attack_skill,
                        resolution.parry_skill,
                        resolution.hit_chance,
                        resolution.armor_percent,
                        resolution.raw_damage * resolution.armor_divisor / POWERSCALE,
                    ),
                });
            }
        }
        let sound_type = if resolution.hit {
            7
        } else if !attacker_rhand || !defender_rhand {
            8
        } else if clash_roll.rem_euclid(2) == 0 {
            34
        } else {
            35
        };
        self.queue_sound_area(attacker_x, attacker_y, sound_type);
        if resolution.hit {
            self.apply_legacy_hurt(
                defender_id,
                Some(attacker_id),
                resolution.raw_damage,
                resolution.armor_divisor,
                resolution.armor_percent,
                resolution.shield_percent,
            );
        }
        // C `act_attack` (act.c:763-793): after `sub_attack` (ported above as
        // `apply_legacy_hurt`), checks `if (!ch[cn].flags) return 0` - guards
        // against the attacker having died mid-attack (e.g. a future
        // reflect-damage effect); no such effect exists yet, so this always
        // passes today. Then two `sub_surround` calls fire for
        // `V_SURROUND` weapons (not ported - `V_SURROUND`/`sub_surround` do
        // not exist on `Character`/`World` yet) and `increase_rage` (not
        // ported - no `rage` field on `Character` yet, see `world/regen.rs`
        // doc comment). Finally `notify_area(ch[cn].x, ch[cn].y, NT_CHAR, cn,
        // 0, 0)` fires gated on `!CF_NONOTIFY`, regardless of whether the
        // attack hit or missed.
        if let Some(attacker) = self.characters.get(&attacker_id) {
            if !attacker.flags.contains(CharacterFlags::DEAD)
                && !attacker.flags.contains(CharacterFlags::NONOTIFY)
            {
                let (x, y) = (attacker.x, attacker.y);
                self.notify_area(x, y, NT_CHAR, attacker_id.0 as i32, 0, 0);
            }
        }
        true
    }

    pub(crate) fn player_can_attack_target(
        &self,
        player: &mut PlayerRuntime,
        attacker_id: CharacterId,
        target_id: CharacterId,
        area_id: u16,
    ) -> bool {
        let Some(attacker) = self.characters.get(&attacker_id) else {
            return false;
        };
        let Some(target) = self.characters.get(&target_id) else {
            return false;
        };
        let attack_policy = RuntimePlayerAttackPolicy {
            attacker_runtime: player,
        };
        let can_attack = can_attack_in_area_with_clan_policy(
            attacker,
            target,
            &self.map,
            area_id,
            &attack_policy,
        );
        if !can_attack {
            self.remove_stale_pvp_hate_if_legacy_check_fails(
                player,
                attacker_id,
                target_id,
                area_id,
            );
        }
        can_attack
    }

    pub(crate) fn remove_stale_pvp_hate_if_legacy_check_fails(
        &self,
        player: &mut PlayerRuntime,
        attacker_id: CharacterId,
        target_id: CharacterId,
        area_id: u16,
    ) {
        if area_id == 1 {
            return;
        }
        let Some(attacker) = self.characters.get(&attacker_id) else {
            return;
        };
        let Some(target) = self.characters.get(&target_id) else {
            return;
        };
        if !attacker.flags.contains(CharacterFlags::PLAYER)
            || !target.flags.contains(CharacterFlags::PLAYER)
            || !attacker.flags.contains(CharacterFlags::PK)
        {
            return;
        }
        if attacker.id == target.id
            || !target.flags.contains(CharacterFlags::PK)
            || attacker.level.abs_diff(target.level) > 3
        {
            player.remove_pk_hate(target.id.0);
        }
    }
}

pub(crate) fn is_back_attack_against_target(
    target: &Character,
    attacker_x: u16,
    attacker_y: u16,
) -> bool {
    let target_x = i32::from(target.x);
    let target_y = i32::from(target.y);
    let attacker_x = i32::from(attacker_x);
    let attacker_y = i32::from(attacker_y);

    match Direction::try_from(target.dir).ok() {
        Some(Direction::Left) => target_x + 1 == attacker_x && target_y == attacker_y,
        Some(Direction::Right) => target_x - 1 == attacker_x && target_y == attacker_y,
        Some(Direction::Down) => target_x == attacker_x && target_y - 1 == attacker_y,
        Some(Direction::Up) => target_x == attacker_x && target_y + 1 == attacker_y,
        _ => false,
    }
}

pub(crate) fn character_is_facing(character: &Character, other: &Character) -> bool {
    let Ok(direction) = Direction::try_from(character.dir) else {
        return false;
    };
    let (dx, dy) = direction.delta();
    let delta_x = i32::from(other.x) - i32::from(character.x);
    let delta_y = i32::from(other.y) - i32::from(character.y);
    match (dx, dy) {
        (1, 0) => delta_x > 0 && delta_y.abs() <= delta_x,
        (-1, 0) => delta_x < 0 && delta_y.abs() <= -delta_x,
        (0, 1) => delta_y > 0 && delta_x.abs() <= delta_y,
        (0, -1) => delta_y < 0 && delta_x.abs() <= -delta_y,
        (1, 1) => delta_x > 0 && delta_y > 0,
        (-1, 1) => delta_x < 0 && delta_y > 0,
        (1, -1) => delta_x > 0 && delta_y < 0,
        (-1, -1) => delta_x < 0 && delta_y < 0,
        _ => false,
    }
}
