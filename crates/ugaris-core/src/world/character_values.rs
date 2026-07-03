use super::*;

pub(crate) fn character_value(character: &Character, value: CharacterValue) -> i32 {
    character
        .values
        .first()
        .and_then(|values| values.get(value as usize))
        .copied()
        .unwrap_or_default() as i32
}

pub(crate) fn bonus_spell_shape(driver: u16) -> Option<(&'static str, CharacterValue)> {
    Some(match driver {
        IDR_ARMOR => ("Armor", CharacterValue::Armor),
        IDR_WEAPON => ("Weapon", CharacterValue::Weapon),
        IDR_MANA => ("Mana", CharacterValue::Mana),
        IDR_HP => ("HP", CharacterValue::Hp),
        _ => return None,
    })
}

pub(crate) fn add_character_value_delta(
    character: &mut Character,
    value: CharacterValue,
    delta: i32,
) {
    if let Some(slot) = character
        .values
        .get_mut(0)
        .and_then(|values| values.get_mut(value as usize))
    {
        *slot = (i32::from(*slot) + delta).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
    }
}

pub(crate) fn apply_item_modifier_deltas(character: &mut Character, item: &Item, sign: i32) {
    for (&modifier_index, &modifier_value) in
        item.modifier_index.iter().zip(item.modifier_value.iter())
    {
        if modifier_value == 0 || modifier_index < 0 {
            continue;
        }
        let Ok(value_index) = usize::try_from(modifier_index) else {
            continue;
        };
        if value_index >= CHARACTER_VALUE_COUNT {
            continue;
        }
        let Some(value) = character_value_from_index(value_index) else {
            continue;
        };
        add_character_value_delta(character, value, i32::from(modifier_value) * sign);
    }
}

pub(crate) fn refresh_driver_spell_flags(character: &mut Character, items: &HashMap<ItemId, Item>) {
    let mut has_infravision_spell = false;
    let mut has_nonomagic_spell = false;
    let mut has_oxygen_spell = false;

    for item_id in character.inventory.iter().take(30).flatten() {
        let Some(item) = items.get(item_id) else {
            continue;
        };
        match item.driver {
            IDR_INFRARED => has_infravision_spell = true,
            IDR_NONOMAGIC => has_nonomagic_spell = true,
            IDR_OXYGEN => has_oxygen_spell = true,
            _ => {}
        }
    }

    let old_flags = character.flags;
    character
        .flags
        .set(CharacterFlags::INFRAVISION, has_infravision_spell);
    character
        .flags
        .set(CharacterFlags::NONOMAGIC, has_nonomagic_spell);
    character
        .flags
        .set(CharacterFlags::OXYGEN, has_oxygen_spell);
    if character.flags != old_flags {
        character.flags.insert(CharacterFlags::UPDATE);
    }
}

pub(crate) fn character_value_from_index(index: usize) -> Option<CharacterValue> {
    Some(match index {
        0 => CharacterValue::Hp,
        1 => CharacterValue::Endurance,
        2 => CharacterValue::Mana,
        3 => CharacterValue::Wisdom,
        4 => CharacterValue::Intelligence,
        5 => CharacterValue::Agility,
        6 => CharacterValue::Strength,
        7 => CharacterValue::Armor,
        8 => CharacterValue::Weapon,
        9 => CharacterValue::Light,
        10 => CharacterValue::Speed,
        11 => CharacterValue::Pulse,
        12 => CharacterValue::Dagger,
        13 => CharacterValue::Hand,
        14 => CharacterValue::Staff,
        15 => CharacterValue::Sword,
        16 => CharacterValue::TwoHand,
        17 => CharacterValue::ArmorSkill,
        18 => CharacterValue::Attack,
        19 => CharacterValue::Parry,
        20 => CharacterValue::Warcry,
        21 => CharacterValue::Tactics,
        22 => CharacterValue::Surround,
        23 => CharacterValue::BodyControl,
        24 => CharacterValue::SpeedSkill,
        25 => CharacterValue::Barter,
        26 => CharacterValue::Percept,
        27 => CharacterValue::Stealth,
        28 => CharacterValue::Bless,
        29 => CharacterValue::Heal,
        30 => CharacterValue::Freeze,
        31 => CharacterValue::MagicShield,
        32 => CharacterValue::Flash,
        33 => CharacterValue::Fireball,
        34 => CharacterValue::Empty,
        35 => CharacterValue::Regenerate,
        36 => CharacterValue::Meditate,
        37 => CharacterValue::Immunity,
        38 => CharacterValue::Demon,
        39 => CharacterValue::Duration,
        40 => CharacterValue::Rage,
        41 => CharacterValue::Cold,
        42 => CharacterValue::Profession,
        _ => return None,
    })
}

pub(crate) fn character_value_present(character: &Character, value: CharacterValue) -> i32 {
    character
        .values
        .get(1)
        .and_then(|values| values.get(value as usize))
        .copied()
        .unwrap_or_default() as i32
}

pub(crate) fn character_value_base(character: &Character, value: CharacterValue) -> i32 {
    character
        .values
        .first()
        .and_then(|values| values.get(value as usize))
        .copied()
        .unwrap_or_default() as i32
}

pub(crate) fn character_profession(character: &Character, index: usize) -> i32 {
    character
        .professions
        .get(index)
        .copied()
        .unwrap_or_default() as i32
}
