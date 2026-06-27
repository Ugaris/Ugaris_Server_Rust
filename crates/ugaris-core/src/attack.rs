use crate::entity::POWERSCALE;

pub const ATTACK_DIV: i32 = 5;
pub const DIRECT_ATTACK_ARMOR_DIV: i32 = ATTACK_DIV;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttackChance {
    pub hit_chance: i32,
    pub armor_percent: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HurtReduction {
    pub damage_after_armor: i32,
    pub shield_absorbed: i32,
    pub hp_damage: i32,
    pub remaining_lifeshield: i32,
}

pub fn tactics_to_melee(value: i32) -> i32 {
    (value as f64 * 0.375) as i32
}

pub fn tactics_to_spell(value: i32) -> i32 {
    (value as f64 * 0.125) as i32
}

pub fn spell_average(
    bless: i32,
    heal: i32,
    freeze: i32,
    magic_shield: i32,
    flash: i32,
    fireball: i32,
    pulse: i32,
) -> f64 {
    (bless + heal + freeze + magic_shield + flash + fireball + pulse) as f64 / 8.0
}

pub fn attack_skill(
    has_attack_base: bool,
    fight_skill: i32,
    attack_total: i32,
    tactics_total: i32,
    rage: i32,
    is_earth_demon: bool,
    level: i32,
    spell_average: f64,
) -> i32 {
    if has_attack_base {
        fight_skill + attack_total * 2 + tactics_to_melee(tactics_total) + rage / 5 / POWERSCALE
    } else if is_earth_demon {
        (fight_skill as f64 * 3.5) as i32
    } else {
        (fight_skill as f64 + spell_average * 2.0 - level as f64) as i32
    }
}

pub fn parry_skill(
    has_parry_base: bool,
    fight_skill: i32,
    parry_total: i32,
    tactics_total: i32,
    rage: i32,
    is_earth_demon: bool,
    has_magic_shield_base: bool,
    magic_shield_total: i32,
    spell_average: f64,
) -> i32 {
    if has_parry_base {
        fight_skill + parry_total * 2 + tactics_to_melee(tactics_total) + rage / 5 / POWERSCALE
    } else if is_earth_demon {
        (fight_skill as f64 * 3.5) as i32
    } else if has_magic_shield_base {
        fight_skill + magic_shield_total * 2
    } else {
        (fight_skill as f64 + spell_average * 2.0) as i32
    }
}

pub fn surround_attack_skill(
    has_surround_base: bool,
    fight_skill: i32,
    surround_total: i32,
    tactics_total: i32,
    rage: i32,
) -> Option<i32> {
    has_surround_base.then(|| {
        fight_skill + surround_total * 2 + tactics_to_melee(tactics_total) + rage / 5 / POWERSCALE
            - 12
    })
}

pub fn apply_facing_attack_bonus(
    attack_skill: i32,
    parry_skill: i32,
    target_is_facing_attacker: bool,
    attacker_is_behind_target: bool,
    assassin_profession: i32,
    target_is_idle: bool,
) -> (i32, i32) {
    let mut attack_skill = attack_skill;
    let mut parry_skill = parry_skill;

    if !target_is_facing_attacker {
        parry_skill -= 8;
        if assassin_profession != 0 {
            attack_skill += assassin_profession;
        }
    }
    if attacker_is_behind_target {
        parry_skill -= 8;
        if assassin_profession != 0 && target_is_idle {
            attack_skill += assassin_profession * 2;
        }
    }

    (attack_skill, parry_skill)
}

pub fn attack_chance_for_diff(diff: i32) -> AttackChance {
    let (hit_chance, armor_percent) = if diff < -146 {
        (10, 90)
    } else if diff < -128 {
        (11, 90)
    } else if diff < -112 {
        (12, 90)
    } else if diff < -96 {
        (13, 90)
    } else if diff < -80 {
        (14, 90)
    } else if diff < -72 {
        (15, 90)
    } else if diff < -64 {
        (16, 90)
    } else if diff < -56 {
        (17, 90)
    } else if diff < -48 {
        (18, 90)
    } else if diff < -40 {
        (19, 90)
    } else if diff < -36 {
        (20, 90)
    } else if diff < -32 {
        (22, 90)
    } else if diff < -28 {
        (24, 90)
    } else if diff < -24 {
        (26, 90)
    } else if diff < -20 {
        (28, 90)
    } else if diff < -18 {
        (30, 90)
    } else if diff < -16 {
        (32, 90)
    } else if diff < -14 {
        (34, 90)
    } else if diff < -12 {
        (36, 90)
    } else if diff < -10 {
        (38, 90)
    } else if diff < -8 {
        (40, 90)
    } else if diff < -6 {
        (42, 90)
    } else if diff < -4 {
        (44, 90)
    } else if diff < -2 {
        (46, 90)
    } else if diff < 0 {
        (48, 90)
    } else if diff == 0 {
        (50, 90)
    } else if diff < 2 {
        (52, 90)
    } else if diff < 4 {
        (54, 90)
    } else if diff < 6 {
        (56, 90)
    } else if diff < 8 {
        (58, 90)
    } else if diff < 10 {
        (60, 90)
    } else if diff < 12 {
        (62, 90)
    } else if diff < 14 {
        (64, 90)
    } else if diff < 16 {
        (66, 85)
    } else if diff < 18 {
        (68, 80)
    } else if diff < 20 {
        (70, 75)
    } else if diff < 24 {
        (72, 70)
    } else if diff < 28 {
        (74, 65)
    } else if diff < 32 {
        (76, 60)
    } else if diff < 36 {
        (78, 55)
    } else if diff < 40 {
        (80, 50)
    } else if diff < 44 {
        (81, 45)
    } else if diff < 48 {
        (82, 40)
    } else if diff < 52 {
        (83, 35)
    } else if diff < 56 {
        (84, 30)
    } else if diff < 60 {
        (85, 25)
    } else if diff < 64 {
        (86, 20)
    } else if diff < 68 {
        (87, 15)
    } else if diff < 72 {
        (89, 10)
    } else {
        (90, 5)
    };

    AttackChance {
        hit_chance,
        armor_percent,
    }
}

pub fn attack_roll_hits(d100_roll: i32, hit_chance: i32) -> bool {
    d100_roll < hit_chance
}

pub fn direct_attack_damage_units(
    weapon_value: i32,
    d6_roll: i32,
    assassin_profession: i32,
    attacker_is_behind_target: bool,
    target_is_idle: bool,
) -> i32 {
    let mut damage = weapon_value + d6_roll;
    if assassin_profession != 0 && attacker_is_behind_target && target_is_idle {
        damage += assassin_profession * 5;
    }
    damage.max(0)
}

pub fn scaled_direct_attack_damage(damage_units: i32) -> i32 {
    damage_units * POWERSCALE / ATTACK_DIV
}

pub fn direct_attack_shield_percent(armor_percent: i32) -> i32 {
    75 + armor_percent / 4
}

pub fn reduce_hurt_by_armor_and_lifeshield(
    damage: i32,
    armor_value: i32,
    armor_divisor: i32,
    armor_percent: i32,
    lifeshield: i32,
    shield_percent: i32,
) -> HurtReduction {
    let mut damage = reduce_hurt_by_armor(damage, armor_value, armor_divisor, armor_percent);
    let damage_after_armor = damage;

    let mut remaining_lifeshield = lifeshield.max(0);
    let mut shield_absorbed = 0;
    if damage != 0 && remaining_lifeshield != 0 && shield_percent > 0 {
        shield_absorbed = (damage * shield_percent / 100).min(remaining_lifeshield);
        remaining_lifeshield -= shield_absorbed;
        damage -= shield_absorbed;
    }

    HurtReduction {
        damage_after_armor,
        shield_absorbed,
        hp_damage: damage,
        remaining_lifeshield,
    }
}

pub fn reduce_hurt_by_armor(
    damage: i32,
    armor_value: i32,
    armor_divisor: i32,
    armor_percent: i32,
) -> i32 {
    let armor_divisor = armor_divisor.max(1);
    let mut damage = damage.max(0);
    let percent_damage = damage * armor_percent / 100;
    let armor_cap = armor_value * POWERSCALE / 20 / armor_divisor;
    damage -= percent_damage.min(armor_cap);
    damage.max(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_chance_breakpoints_match_act_c_table() {
        assert_eq!(
            attack_chance_for_diff(-147),
            AttackChance {
                hit_chance: 10,
                armor_percent: 90
            }
        );
        assert_eq!(
            attack_chance_for_diff(-146),
            AttackChance {
                hit_chance: 11,
                armor_percent: 90
            }
        );
        assert_eq!(
            attack_chance_for_diff(-1),
            AttackChance {
                hit_chance: 48,
                armor_percent: 90
            }
        );
        assert_eq!(
            attack_chance_for_diff(0),
            AttackChance {
                hit_chance: 50,
                armor_percent: 90
            }
        );
        assert_eq!(
            attack_chance_for_diff(15),
            AttackChance {
                hit_chance: 66,
                armor_percent: 85
            }
        );
        assert_eq!(
            attack_chance_for_diff(16),
            AttackChance {
                hit_chance: 68,
                armor_percent: 80
            }
        );
        assert_eq!(
            attack_chance_for_diff(72),
            AttackChance {
                hit_chance: 90,
                armor_percent: 5
            }
        );
    }

    #[test]
    fn attack_roll_uses_strict_less_than_legacy_rule() {
        assert!(attack_roll_hits(49, 50));
        assert!(!attack_roll_hits(50, 50));
    }

    #[test]
    fn skill_helpers_preserve_legacy_integer_truncation() {
        assert_eq!(tactics_to_melee(7), 2);
        assert_eq!(tactics_to_spell(15), 1);
        assert_eq!(spell_average(8, 8, 8, 8, 8, 8, 8), 7.0);
        assert_eq!(attack_skill(false, 10, 0, 0, 0, false, 5, 7.0), 19);
        assert_eq!(parry_skill(false, 10, 0, 0, 0, false, false, 0, 7.0), 24);
        assert_eq!(
            surround_attack_skill(true, 20, 7, 7, 5 * POWERSCALE),
            Some(25)
        );
    }

    #[test]
    fn facing_and_assassin_bonuses_match_act_attack() {
        assert_eq!(
            apply_facing_attack_bonus(50, 50, true, false, 4, true),
            (50, 50)
        );
        assert_eq!(
            apply_facing_attack_bonus(50, 50, false, false, 4, true),
            (54, 42)
        );
        assert_eq!(
            apply_facing_attack_bonus(50, 50, false, true, 4, true),
            (62, 34)
        );
    }

    #[test]
    fn direct_damage_and_hurt_reduction_match_legacy_units() {
        let damage_units = direct_attack_damage_units(10, 6, 3, true, true);
        assert_eq!(damage_units, 31);
        let scaled = scaled_direct_attack_damage(damage_units);
        assert_eq!(scaled, 6200);
        assert_eq!(direct_attack_shield_percent(90), 97);

        let reduced = reduce_hurt_by_armor_and_lifeshield(scaled, 20, ATTACK_DIV, 90, 3000, 97);
        assert_eq!(reduced.damage_after_armor, 6000);
        assert_eq!(reduced.shield_absorbed, 3000);
        assert_eq!(reduced.hp_damage, 3000);
        assert_eq!(reduced.remaining_lifeshield, 0);
    }
}
