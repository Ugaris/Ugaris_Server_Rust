use super::*;

pub(crate) fn round_down_to_granularity(value: u32, granularity: u32) -> u32 {
    if granularity == 0 {
        value
    } else {
        (value / granularity) * granularity
    }
}

pub(crate) const BOOKCASE_RANDOM_TITLES: [&str; 26] = [
    "Tales of Two Towns by Karl Dicker",
    "The Art of Warfare by Hun Yu",
    "Chris Maas visits Carol by Karl Dicker",
    "Secrets of Adygalah Alchemy by Leonarda",
    "The rise and fall of the Seyan Empire by Takitus",
    "History of Ancient Astonia by Chiasmaphora",
    "Treatise on the Mastery of Mana by Mage Niuma",
    "The Song of the Warrior by Sir Regis Le Voleir",
    "The Book of Ishtar, Anonymous",
    "Concessions to Fear by Kentindher",
    "Poems of War and Homecoming by Melthold of Anten",
    "Memoires of a Lady-in-Waiting by Dame Sakanor",
    "Comprehension and Expression by Master Getsades",
    "Great Astonian Thinkers by Master Riotan",
    "A Portrait of the Seyan'Du as A Young Mage by Esjamocey",
    "Critique of Pure Courage by Imanel Dique",
    "Collected Essays by Lindmar the Elder",
    "The Reforming of Curves by Master Elyosod",
    "Advanced Agility in Forty-two Steps by Seyan'Du Bartoshi",
    "The Oath by Sheney",
    "The Strife for Light by Father Ignato",
    "The Aston Years by Lord Ironborn",
    "Luctim - Superstition or Reality? by Mintu the Enlightened",
    "I Have, Alas by Goytila",
    "A Midwinter Day's Wake by Pearshaks",
    "Fama Fraternitatis by Valentin Andreae",
];

pub(crate) const DEV_ID_DB: u32 = 0x01;

pub(crate) const DEV_ID_WARR: u32 = 0x06;

pub(crate) const fn make_item_id(dev_id: u32, nr: u32) -> u32 {
    (dev_id << 24) | nr
}

pub(crate) fn drdata_u16(item: &Item, idx: usize) -> u16 {
    let lo = u16::from(drdata(item, idx));
    let hi = u16::from(drdata(item, idx + 1));
    lo | (hi << 8)
}

pub(crate) fn drdata_u32(item: &Item, idx: usize) -> u32 {
    u32::from_le_bytes([
        drdata(item, idx),
        drdata(item, idx + 1),
        drdata(item, idx + 2),
        drdata(item, idx + 3),
    ])
}

pub(crate) fn set_drdata_u16(item: &mut Item, idx: usize, value: u16) {
    set_drdata(item, idx, value as u8);
    set_drdata(item, idx + 1, (value >> 8) as u8);
}

pub(crate) fn set_drdata_u32(item: &mut Item, idx: usize, value: u32) {
    for (offset, byte) in value.to_le_bytes().into_iter().enumerate() {
        set_drdata(item, idx + offset, byte);
    }
}

pub(crate) const EDEMON_SWITCH_COOLDOWN_TICKS: u64 = TICKS_PER_SECOND * 60 * 5;

pub(crate) fn legacy_level_value(level: u32) -> u32 {
    let level = u64::from(level);
    let next = level.saturating_add(1);
    next.saturating_pow(4)
        .saturating_sub(level.saturating_pow(4))
        .min(u64::from(u32::MAX)) as u32
}

pub(crate) fn check_item_requirements(character: &Character, item: &Item) -> bool {
    if character.level < u32::from(item.min_level) {
        return false;
    }
    if item.max_level != 0 && character.level > u32::from(item.max_level) {
        return false;
    }
    if item.needs_class & 1 != 0 && !character.flags.contains(CharacterFlags::WARRIOR) {
        return false;
    }
    if item.needs_class & 2 != 0 && !character.flags.contains(CharacterFlags::MAGE) {
        return false;
    }
    if item.needs_class & 4 != 0
        && !(character.flags.contains(CharacterFlags::WARRIOR)
            && character.flags.contains(CharacterFlags::MAGE))
    {
        return false;
    }
    if item.needs_class & 8 != 0 && !character.flags.contains(CharacterFlags::ARCH) {
        return false;
    }

    item.modifier_index
        .iter()
        .zip(item.modifier_value.iter())
        .all(|(&index, &required)| {
            if index >= 0 || required <= 0 {
                return true;
            }
            let value = (-index) as usize;
            character
                .values
                .get(1)
                .and_then(|values| values.get(value))
                .copied()
                .unwrap_or_default()
                >= required
        })
}

pub(crate) fn capped_resource(current: i32, added_units: u8, max_units: i32) -> i32 {
    (current + i32::from(added_units) * POWERSCALE).min(max_units * POWERSCALE)
}

pub(crate) fn max_value(character: &Character, value: CharacterValue) -> i32 {
    character
        .values
        .first()
        .and_then(|values| values.get(value as usize))
        .copied()
        .unwrap_or_default() as i32
}

pub(crate) fn drdata(item: &Item, idx: usize) -> u8 {
    item.driver_data.get(idx).copied().unwrap_or_default()
}

pub(crate) fn set_drdata(item: &mut Item, idx: usize, value: u8) {
    if item.driver_data.len() <= idx {
        item.driver_data.resize(idx + 1, 0);
    }
    item.driver_data[idx] = value;
}

pub(crate) fn write_drdata_u32(item: &mut Item, idx: usize, value: u32) {
    if item.driver_data.len() <= idx + 3 {
        item.driver_data.resize(idx + 4, 0);
    }
    item.driver_data[idx..idx + 4].copy_from_slice(&value.to_le_bytes());
}

pub(crate) fn clamp_legacy_coordinate(value: i32) -> u16 {
    value.clamp(0, i32::from(u16::MAX)) as u16
}
