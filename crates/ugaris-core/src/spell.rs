use serde::{Deserialize, Serialize};

use crate::{
    combat::FIREBALL_DAMAGE,
    entity::{CharacterFlags, POWERSCALE},
    tick::TICKS_PER_SECOND,
};

pub const BLESS_COST: i32 = 2 * POWERSCALE;
pub const FREEZE_COST: i32 = 2 * POWERSCALE;
pub const FLASH_COST: i32 = 3 * POWERSCALE;
pub const FIREBALL_COST: i32 = 3 * POWERSCALE;

pub const WARCRY_DURATION: i32 = TICKS_PER_SECOND as i32 * 4;
pub const BLESS_DURATION: i32 = TICKS_PER_SECOND as i32 * 60 * 2;
pub const FLASH_DURATION: i32 = TICKS_PER_SECOND as i32 * 2;
pub const FREEZE_DURATION: i32 = TICKS_PER_SECOND as i32 * 4;
pub const DURATION_DIVISOR: i32 = 35;

pub const IDR_FIREBALL: u16 = 15;
pub const IDR_BLESS: u16 = 1000;
pub const IDR_FREEZE: u16 = 1001;
pub const IDR_FLASH: u16 = 1002;
pub const IDR_WARCRY: u16 = 1003;
pub const IDR_CURSE: u16 = 1010;
pub const IDR_FIRERING: u16 = 1015;

pub const EF_FIREBALL: i32 = 1;
pub const EF_MAGICSHIELD: i32 = 2;
pub const EF_BALL: i32 = 3;
pub const EF_FLASH: i32 = 5;
pub const EF_WARCRY: i32 = 8;
pub const EF_BLESS: i32 = 9;
pub const EF_FREEZE: i32 = 10;
pub const EF_HEAL: i32 = 11;
pub const EF_BURN: i32 = 12;
pub const EF_PULSE: i32 = 21;
pub const EF_PULSEBACK: i32 = 22;
pub const EF_FIRERING: i32 = 23;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpellSpend {
    pub amount: i32,
    pub mana_cost: i32,
}

pub fn tactics_to_spell(tactics: i32) -> i32 {
    tactics / 8
}

pub fn tactics_to_immunity(tactics: i32) -> i32 {
    tactics / 8
}

pub fn spell_power(skill: i32, tactics: i32) -> i32 {
    skill + tactics_to_spell(tactics)
}

pub fn duration_with_bonus(
    base_duration: i32,
    duration_skill: i32,
    has_duration_skill: bool,
    flags: CharacterFlags,
    level: i32,
) -> i32 {
    if has_duration_skill {
        base_duration + base_duration * duration_skill / DURATION_DIVISOR
    } else if flags.contains(CharacterFlags::ARCH) {
        base_duration + base_duration * level / DURATION_DIVISOR / 2
    } else {
        base_duration
    }
}

pub fn heal_spend(heal_skill: i32, target_missing_hp: i32, caster_mana: i32) -> Option<SpellSpend> {
    if caster_mana < POWERSCALE {
        return None;
    }

    let amount = (heal_skill * POWERSCALE / 2)
        .min(target_missing_hp)
        .min(caster_mana * 2);
    (amount >= 1).then_some(SpellSpend {
        amount,
        mana_cost: amount / 2,
    })
}

pub fn magicshield_spend(
    magicshield_skill: i32,
    current_lifeshield: i32,
    caster_mana: i32,
) -> Option<SpellSpend> {
    if caster_mana < POWERSCALE {
        return None;
    }

    let max_lifeshield = magicshield_skill * POWERSCALE;
    let amount = max_lifeshield
        .min(caster_mana * 2)
        .min(max_lifeshield - current_lifeshield);
    (amount >= 1).then_some(SpellSpend {
        amount,
        mana_cost: amount / 2,
    })
}

pub fn pulse_spend(pulse_power: i32, caster_mana: i32) -> Option<SpellSpend> {
    if caster_mana < POWERSCALE {
        return None;
    }

    let amount = pulse_power.min(caster_mana * 8 / POWERSCALE);
    (amount >= 1).then_some(SpellSpend {
        amount,
        mana_cost: amount * POWERSCALE / 8,
    })
}

pub fn effective_immunity(immunity: i32, tactics: i32, has_tactics_skill: bool) -> i32 {
    if has_tactics_skill {
        immunity + tactics_to_immunity(tactics + 14)
    } else {
        immunity
    }
}

pub fn immunity_reduction(
    skill_strength: i32,
    immunity: i32,
    tactics: i32,
    has_tactics_skill: bool,
) -> i32 {
    let immunity = effective_immunity(immunity, tactics, has_tactics_skill);
    let strength = POWERSCALE + (skill_strength - immunity) * 50 + skill_strength * 10;
    strength.max(0)
}

pub fn immunity_reduction_no_bonus(
    skill_strength: i32,
    immunity: i32,
    tactics: i32,
    has_tactics_skill: bool,
) -> i32 {
    let immunity = effective_immunity(immunity, tactics, has_tactics_skill);
    let strength = POWERSCALE + (skill_strength - immunity) * 40;
    strength.max(POWERSCALE / 10)
}

pub fn fireball_damage(
    spell_power: i32,
    immunity: i32,
    tactics: i32,
    has_tactics_skill: bool,
) -> i32 {
    immunity_reduction(spell_power, immunity, tactics, has_tactics_skill) * FIREBALL_DAMAGE
}

pub fn warcry_damage(
    spell_power: i32,
    immunity: i32,
    tactics: i32,
    has_tactics_skill: bool,
) -> i32 {
    immunity_reduction(spell_power, immunity, tactics, has_tactics_skill)
}

pub fn pulse_damage(
    pulse_skill: i32,
    pulse_spend_amount: i32,
    immunity: i32,
    tactics: i32,
    has_tactics_skill: bool,
) -> i32 {
    let reduced = immunity_reduction_no_bonus(pulse_skill, immunity, tactics, has_tactics_skill);
    if reduced < 1 {
        return 0;
    }
    pulse_spend_amount * reduced / 10
}

pub fn freeze_speed_modifier(
    freeze_power: i32,
    target_immunity: i32,
    target_tactics: i32,
    target_has_tactics_skill: bool,
    caster_is_ice_demon: bool,
    caster_demon_base: i32,
    target_cold: i32,
) -> i32 {
    let mut modifier = -(200 + freeze_power * 11 - target_immunity * 11);
    if target_has_tactics_skill {
        modifier += tactics_to_immunity(target_tactics + 14) * 11;
    }
    if caster_is_ice_demon && target_cold > caster_demon_base {
        modifier += (target_cold - caster_demon_base) * 10;
    }
    modifier
}

pub fn warcry_speed_modifier(
    warcry_power: i32,
    target_immunity: i32,
    target_tactics: i32,
    has_tactics_skill: bool,
) -> i32 {
    let mut modifier = -(100 + warcry_power * 6 - target_immunity * 6);
    if has_tactics_skill {
        modifier += tactics_to_immunity(target_tactics + 14) * 6;
    }
    modifier
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spell_constants_match_legacy_headers() {
        assert_eq!(BLESS_COST, 2000);
        assert_eq!(FREEZE_COST, 2000);
        assert_eq!(FLASH_COST, 3000);
        assert_eq!(FIREBALL_COST, 3000);
        assert_eq!(BLESS_DURATION, 2880);
        assert_eq!(FLASH_DURATION, 48);
        assert_eq!(FREEZE_DURATION, 96);
        assert_eq!(WARCRY_DURATION, 96);
        assert_eq!(IDR_BLESS, 1000);
        assert_eq!(IDR_WARCRY, 1003);
        assert_eq!(IDR_CURSE, 1010);
        assert_eq!(IDR_FIRERING, 1015);
        assert_eq!(EF_PULSEBACK, 22);
    }

    #[test]
    fn spellpower_and_duration_follow_legacy_integer_math() {
        assert_eq!(tactics_to_spell(14), 1);
        assert_eq!(spell_power(50, 24), 53);
        assert_eq!(
            duration_with_bonus(FREEZE_DURATION, 35, true, CharacterFlags::empty(), 80),
            192
        );
        assert_eq!(
            duration_with_bonus(FLASH_DURATION, 0, false, CharacterFlags::ARCH, 70),
            96
        );
    }

    #[test]
    fn variable_spends_match_do_c_formulas() {
        assert_eq!(
            heal_spend(20, 20_000, 6_000),
            Some(SpellSpend {
                amount: 10_000,
                mana_cost: 5_000,
            })
        );
        assert_eq!(
            magicshield_spend(10, 7_500, 10_000),
            Some(SpellSpend {
                amount: 2_500,
                mana_cost: 1_250,
            })
        );
        assert_eq!(
            pulse_spend(50, 4_000),
            Some(SpellSpend {
                amount: 32,
                mana_cost: 4_000,
            })
        );
        assert_eq!(heal_spend(20, 0, 6_000), None);
    }

    #[test]
    fn combat_spell_formulas_match_tool_c_math() {
        assert_eq!(immunity_reduction(80, 40, 0, false), 3_800);
        assert_eq!(immunity_reduction_no_bonus(80, 120, 0, false), 100);
        assert_eq!(fireball_damage(80, 40, 0, false), 19_000);
        assert_eq!(warcry_damage(80, 40, 0, false), 3_800);
        assert_eq!(pulse_damage(80, 20, 40, 0, false), 52_000 / 10);
    }

    #[test]
    fn speed_modifier_formulas_match_tool_c_math() {
        assert_eq!(freeze_speed_modifier(50, 30, 0, false, false, 0, 0), -420);
        assert_eq!(freeze_speed_modifier(50, 30, 26, true, true, 10, 13), -335);
        assert_eq!(warcry_speed_modifier(50, 30, 0, false), -220);
        assert_eq!(warcry_speed_modifier(50, 30, 26, true), -190);
    }
}
