use super::*;

pub(crate) fn legacy_random(seed: u64, max: u32) -> u32 {
    if max == 0 {
        return 0;
    }
    let mut value = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    ((value ^ (value >> 31)) % u64::from(max)) as u32
}

pub(crate) fn legacy_nomad_dice_roll(seed: u64, luck: u8) -> ([u8; 3], u8) {
    let mut next_roll = 0_u64;
    let mut roll_die = || {
        let rolls = (0..=luck).map(|_| {
            let roll = legacy_random(seed.wrapping_add(next_roll), 6) as u8 + 1;
            next_roll = next_roll.wrapping_add(1);
            roll
        });
        legacy_lucky_die_from_rolls(6, luck, rolls)
    };

    let dice = [roll_die(), roll_die(), roll_die()];
    let total = dice.iter().copied().sum();
    (dice, total)
}

pub(crate) fn runtime_random_below(max: i32) -> i32 {
    if max <= 0 {
        return 0;
    }

    legacy_random(runtime_random_seed(), max as u32) as i32
}

pub(crate) fn runtime_random_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or_default()
}
