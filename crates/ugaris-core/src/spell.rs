use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    combat::FIREBALL_DAMAGE,
    entity::{Character, CharacterFlags, Item, POWERSCALE},
    ids::ItemId,
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
pub const POISON_DURATION: i32 = TICKS_PER_SECOND as i32 * 60 * 60 * 2;
pub const DURATION_DIVISOR: i32 = 35;

pub const IDR_FIREBALL: u16 = 15;
pub const IDR_BLESS: u16 = 1000;
pub const IDR_FREEZE: u16 = 1001;
pub const IDR_FLASH: u16 = 1002;
pub const IDR_WARCRY: u16 = 1003;
pub const IDR_ARMOR: u16 = 1004;
pub const IDR_WEAPON: u16 = 1005;
pub const IDR_MANA: u16 = 1006;
pub const IDR_HP: u16 = 1007;
pub const IDR_POTION_SP: u16 = 1009;
pub const IDR_CURSE: u16 = 1010;
pub const IDR_POISON0: u16 = 1011;
pub const IDR_POISON1: u16 = 1012;
pub const IDR_POISON2: u16 = 1013;
pub const IDR_POISON3: u16 = 1014;
pub const IDR_FIRERING: u16 = 1015;
pub const IDR_INFRARED: u16 = 1016;
pub const IDR_NONOMAGIC: u16 = 1017;
pub const IDR_OXYGEN: u16 = 1018;
pub const IDR_UWTALK: u16 = 1019;

pub const SPELL_SLOT_START: usize = 12;
pub const SPELL_SLOT_END: usize = 30;
pub const BLESS_REFRESH_WINDOW_TICKS: u32 = TICKS_PER_SECOND as u32 * 30;

pub const EF_FIREBALL: i32 = 1;
pub const EF_MAGICSHIELD: i32 = 2;
pub const EF_BALL: i32 = 3;
pub const EF_STRIKE: i32 = 4;
pub const EF_FLASH: i32 = 5;
pub const EF_EXPLODE: i32 = 7;
pub const EF_WARCRY: i32 = 8;
pub const EF_BLESS: i32 = 9;
pub const EF_FREEZE: i32 = 10;
pub const EF_HEAL: i32 = 11;
pub const EF_BURN: i32 = 12;
pub const EF_MIST: i32 = 13;
pub const EF_POTION: i32 = 14;
pub const EF_EARTHRAIN: i32 = 15;
pub const EF_EARTHMUD: i32 = 16;
pub const EF_EDEMONBALL: i32 = 17;
pub const EF_CURSE: i32 = 18;
pub const EF_CAP: i32 = 19;
pub const EF_LAG: i32 = 20;
pub const EF_PULSE: i32 = 21;
pub const EF_PULSEBACK: i32 = 22;
pub const EF_FIRERING: i32 = 23;
pub const EF_BUBBLE: i32 = 24;

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

pub fn may_add_spell(
    character: &Character,
    items: &HashMap<ItemId, Item>,
    driver: u16,
    current_tick: u32,
) -> Option<usize> {
    let mut free_slot = None;

    for slot in SPELL_SLOT_START..SPELL_SLOT_END {
        match character.inventory.get(slot).copied().flatten() {
            Some(item_id) => {
                if items
                    .get(&item_id)
                    .is_some_and(|item| item.driver == driver)
                {
                    if driver == IDR_BLESS {
                        let expires_at = items
                            .get(&item_id)
                            .and_then(|item| read_spell_expire_tick(&item.driver_data));
                        if expires_at.is_some_and(|tick| {
                            tick.wrapping_sub(current_tick) < BLESS_REFRESH_WINDOW_TICKS
                        }) {
                            return Some(slot);
                        }
                    }
                    return None;
                }
            }
            None => free_slot = Some(slot),
        }
    }

    free_slot
}

pub fn add_same_spell_slot(
    character: &Character,
    items: &HashMap<ItemId, Item>,
    driver: u16,
) -> Option<usize> {
    let mut free_slot = None;

    for slot in SPELL_SLOT_START..SPELL_SLOT_END {
        match character.inventory.get(slot).copied().flatten() {
            Some(item_id) => {
                if items
                    .get(&item_id)
                    .is_some_and(|item| item.driver == driver)
                {
                    return Some(slot);
                }
            }
            None => free_slot = Some(slot),
        }
    }

    free_slot
}

pub fn read_spell_expire_tick(driver_data: &[u8]) -> Option<u32> {
    let bytes = driver_data.get(..4)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

pub fn is_timed_spell_driver(driver: u16) -> bool {
    matches!(
        driver,
        IDR_BLESS
            | IDR_WARCRY
            | IDR_FREEZE
            | IDR_FLASH
            | IDR_ARMOR
            | IDR_WEAPON
            | IDR_HP
            | IDR_MANA
            | IDR_POTION_SP
            | IDR_CURSE
            | IDR_POISON0
            | IDR_POISON1
            | IDR_POISON2
            | IDR_POISON3
            | IDR_NONOMAGIC
            | IDR_FIRERING
            | IDR_INFRARED
            | IDR_OXYGEN
            | IDR_UWTALK
    )
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

pub fn strike_damage(
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
    use crate::{
        entity::{CharacterValue, ItemFlags, INVENTORY_SIZE, MAX_MODIFIERS},
        ids::CharacterId,
    };

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
        assert_eq!(IDR_FREEZE, 1001);
        assert_eq!(IDR_FLASH, 1002);
        assert_eq!(IDR_WARCRY, 1003);
        assert_eq!(IDR_ARMOR, 1004);
        assert_eq!(IDR_WEAPON, 1005);
        assert_eq!(IDR_MANA, 1006);
        assert_eq!(IDR_HP, 1007);
        assert_eq!(IDR_POTION_SP, 1009);
        assert_eq!(IDR_CURSE, 1010);
        assert_eq!(IDR_POISON0, 1011);
        assert_eq!(IDR_POISON3, 1014);
        assert_eq!(IDR_FIRERING, 1015);
        assert_eq!(IDR_INFRARED, 1016);
        assert_eq!(IDR_NONOMAGIC, 1017);
        assert_eq!(IDR_OXYGEN, 1018);
        assert_eq!(IDR_UWTALK, 1019);
        assert_eq!(EF_CURSE, 18);
        assert_eq!(EF_CAP, 19);
        assert_eq!(EF_LAG, 20);
        assert_eq!(EF_PULSEBACK, 22);
    }

    #[test]
    fn timed_spell_driver_classifier_matches_create_spell_timer_core_cases() {
        for driver in [
            IDR_BLESS,
            IDR_WARCRY,
            IDR_FREEZE,
            IDR_FLASH,
            IDR_ARMOR,
            IDR_WEAPON,
            IDR_HP,
            IDR_MANA,
            IDR_POTION_SP,
            IDR_CURSE,
            IDR_POISON0,
            IDR_POISON1,
            IDR_POISON2,
            IDR_POISON3,
            IDR_NONOMAGIC,
            IDR_FIRERING,
            IDR_INFRARED,
            IDR_OXYGEN,
            IDR_UWTALK,
        ] {
            assert!(is_timed_spell_driver(driver));
        }
        assert!(!is_timed_spell_driver(IDR_FIREBALL));
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

    #[test]
    fn may_add_spell_returns_last_free_spell_slot() {
        let character = test_character();
        let items = HashMap::new();

        assert_eq!(
            may_add_spell(&character, &items, IDR_FLASH, 10_000),
            Some(29)
        );
    }

    #[test]
    fn may_add_spell_blocks_duplicate_active_driver() {
        let mut character = test_character();
        character.inventory[12] = Some(ItemId(1));
        character.inventory[29] = None;
        let items = HashMap::from([(ItemId(1), test_item(ItemId(1), IDR_FREEZE, 0))]);

        assert_eq!(may_add_spell(&character, &items, IDR_FREEZE, 10_000), None);
        assert_eq!(
            add_same_spell_slot(&character, &items, IDR_FREEZE),
            Some(12)
        );
    }

    #[test]
    fn may_add_spell_allows_near_expired_bless_refresh() {
        let mut character = test_character();
        character.inventory[18] = Some(ItemId(1));
        let items = HashMap::from([(ItemId(1), test_item(ItemId(1), IDR_BLESS, 10_500))]);

        assert_eq!(
            may_add_spell(&character, &items, IDR_BLESS, 10_000),
            Some(18)
        );
    }

    #[test]
    fn may_add_spell_blocks_bless_with_long_remaining_duration() {
        let mut character = test_character();
        character.inventory[18] = Some(ItemId(1));
        let items = HashMap::from([(ItemId(1), test_item(ItemId(1), IDR_BLESS, 11_000))]);

        assert_eq!(may_add_spell(&character, &items, IDR_BLESS, 10_000), None);
    }

    #[test]
    fn add_same_spell_slot_returns_free_slot_when_absent() {
        let character = test_character();
        let items = HashMap::new();

        assert_eq!(
            add_same_spell_slot(&character, &items, IDR_WARCRY),
            Some(29)
        );
    }

    fn test_character() -> Character {
        let mut values = Character::empty_values();
        values[0][CharacterValue::Hp as usize] = 10;
        Character {
            id: CharacterId(1),
            name: "tester".to_string(),
            description: String::new(),
            flags: CharacterFlags::USED,
            sprite: 0,
            driver: 0,
            speed_mode: crate::entity::SpeedMode::Normal,
            x: 0,
            y: 0,
            rest_area: 0,
            rest_x: 0,
            rest_y: 0,
            tox: 0,
            toy: 0,
            dir: 4,
            action: 0,
            duration: 0,
            step: 0,
            act1: 0,
            act2: 0,
            hp: POWERSCALE * 10,
            mana: POWERSCALE * 10,
            endurance: POWERSCALE * 10,
            lifeshield: 0,
            level: 1,
            exp: 0,
            exp_used: 0,
            gold: 0,
            creation_time: 0,
            saves: 0,
            deaths: 0,
            cursor_item: None,
            current_container: None,
            values,
            professions: Character::empty_professions(),
            inventory: vec![None; INVENTORY_SIZE],
            driver_state: None,
            driver_messages: Vec::new(),
        }
    }

    fn test_item(id: ItemId, driver: u16, expires_at: u32) -> Item {
        Item {
            id,
            name: "spell".to_string(),
            description: String::new(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [0; MAX_MODIFIERS],
            modifier_value: [0; MAX_MODIFIERS],
            x: 0,
            y: 0,
            carried_by: Some(CharacterId(1)),
            contained_in: None,
            content_id: 0,
            driver,
            driver_data: expires_at.to_le_bytes().to_vec(),
            serial: 0,
        }
    }
}
