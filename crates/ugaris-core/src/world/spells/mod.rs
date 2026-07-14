use super::*;

mod casts;
mod install;
mod poison;

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
