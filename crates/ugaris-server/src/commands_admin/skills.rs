use super::*;

pub(crate) fn legacy_lookup_skill(input: &str) -> Option<i16> {
    let token = input
        .chars()
        .take(19)
        .collect::<String>()
        .to_ascii_lowercase();
    let value = match token.as_str() {
        "endurance" => CharacterValue::Endurance,
        "hp" | "health" | "hitpoints" => CharacterValue::Hp,
        "mana" => CharacterValue::Mana,
        "wis" | "wisdom" => CharacterValue::Wisdom,
        "int" | "intuition" => CharacterValue::Intelligence,
        "agi" | "agility" => CharacterValue::Agility,
        "str" | "strength" => CharacterValue::Strength,
        "bart" | "bartering" => CharacterValue::Barter,
        "perc" | "perception" => CharacterValue::Percept,
        "stealth" => CharacterValue::Stealth,
        "hand" | "handtohand" | "hand-to-hand" | "hand2hand" => CharacterValue::Hand,
        "wc" | "warcry" => CharacterValue::Warcry,
        "sh" | "surround" | "surroundhit" => CharacterValue::Surround,
        "bc" | "bodycontrol" | "body-control" => CharacterValue::BodyControl,
        "ss" | "speedskill" | "speed" => CharacterValue::SpeedSkill,
        "heal" => CharacterValue::Heal,
        "fire" | "fireball" => CharacterValue::Fireball,
        "tactics" | "tac" | "tact" => CharacterValue::Tactics,
        "duration" | "dur" => CharacterValue::Duration,
        "rage" => CharacterValue::Rage,
        "bless" => CharacterValue::Bless,
        "freeze" | "frz" | "fre" => CharacterValue::Freeze,
        "ms" | "magicshield" => CharacterValue::MagicShield,
        "lf" | "lightning" | "flash" => CharacterValue::Flash,
        "pulse" | "pul" => CharacterValue::Pulse,
        "dagger" | "dag" => CharacterValue::Dagger,
        "staff" | "sta" => CharacterValue::Staff,
        "sword" | "sw" => CharacterValue::Sword,
        "twohand" | "twohanded" | "two-handed" | "two-hand" | "2hand" | "2h" | "th" => {
            CharacterValue::TwoHand
        }
        "attack" | "att" => CharacterValue::Attack,
        "parry" | "par" => CharacterValue::Parry,
        "immunity" | "imm" | "immy" => CharacterValue::Immunity,
        _ => return None,
    };
    Some(value as i16)
}

pub(crate) fn legacy_skill_start(value: usize) -> i32 {
    match value {
        0..=6 => 10,
        42 => -1,
        11..=41 => 1,
        _ => -1,
    }
}

pub(crate) fn legacy_skill_cost_factor(value: usize) -> i32 {
    match value {
        0..=2 | 42 => 3,
        3..=6 => 2,
        11..=37 | 39 | 40 => 1,
        _ => 0,
    }
}

pub(crate) fn legacy_skillmax(character: &Character) -> i32 {
    if !character.flags.contains(CharacterFlags::ARCH) {
        return 50;
    }
    if character.flags.contains(CharacterFlags::WARRIOR)
        && character.flags.contains(CharacterFlags::MAGE)
    {
        110
    } else {
        125
    }
}

pub(crate) fn legacy_raise_cost(value: usize, current: i32, seyan: bool) -> u32 {
    let nr = current - legacy_skill_start(value) + 1 + 5;
    let cost = nr * nr * nr * legacy_skill_cost_factor(value);
    let cost = if seyan { cost * 4 / 30 } else { cost / 10 };
    cost.max(1) as u32
}

pub(crate) fn legacy_supermax_canraise(value: usize) -> i32 {
    match value {
        3..=6 => 2,
        11 | 12..=24 | 25..=37 | 39 | 40 => 1,
        _ => 0,
    }
}

pub(crate) fn legacy_supermax_cost(character: &Character, value: usize, current: i32) -> u32 {
    let seyan = character.flags.contains(CharacterFlags::WARRIOR)
        && character.flags.contains(CharacterFlags::MAGE);
    (legacy_supermax_canraise(value) * 3_000_000) as u32 + legacy_raise_cost(value, current, seyan)
}

pub(crate) fn legacy_calc_exp_used(character: &Character) -> u32 {
    let seyan = character.flags.contains(CharacterFlags::WARRIOR)
        && character.flags.contains(CharacterFlags::MAGE);
    let Some(bare_values) = character.values.get(1) else {
        return 0;
    };
    let mut exp = 0_u32;
    for value in 0..CHARACTER_VALUE_NAMES.len() {
        let bare = i32::from(*bare_values.get(value).unwrap_or(&0));
        if bare == 0 || legacy_skill_cost_factor(value) == 0 {
            continue;
        }
        for n in (legacy_skill_start(value) + 1)..=bare {
            let current = n - 1;
            let cost = if character.flags.contains(CharacterFlags::PLAYER)
                && current >= legacy_skillmax(character)
            {
                legacy_supermax_cost(character, value, current)
            } else {
                legacy_raise_cost(value, current, seyan)
            };
            exp = exp.saturating_add(cost);
        }
    }
    exp
}
