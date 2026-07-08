use super::*;

pub(crate) fn stat_scroll_driver(character: &mut Character, item: &mut Item) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }
    if character.flags.contains(CharacterFlags::NOEXP) {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let value = usize::from(drdata(item, 0));
    let requested = drdata(item, 1);
    if requested == 0 || value >= CHARACTER_VALUE_COUNT || bare_value(character, value) <= 0 {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let mut raised = 0_u8;
    let mut exp_cost = 0_u32;
    for _ in 0..requested {
        let Some(cost) = raise_value_exp(character, value) else {
            break;
        };
        raised = raised.saturating_add(1);
        exp_cost = exp_cost.saturating_add(cost);
    }

    if raised == 0 {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }

    consume_item(character, item);
    ItemDriverOutcome::StatScrollUsed {
        item_id: item.id,
        character_id: character.id,
        value: value as u8,
        raised,
        exp_cost,
    }
}

pub(crate) fn raise_value_exp(character: &mut Character, value: usize) -> Option<u32> {
    if value >= CHARACTER_VALUE_COUNT || skill_raise_cost_factor(value) == 0 {
        return None;
    }
    let current = bare_value(character, value);
    if current <= 0 || current >= skillmax(character) {
        return None;
    }
    if value == CharacterValue::Profession as usize && current > 99 {
        return None;
    }

    let seyan = character.flags.contains(CharacterFlags::WARRIOR)
        && character.flags.contains(CharacterFlags::MAGE);
    let cost = raise_cost(value, current, seyan);
    character.exp_used = character.exp_used.saturating_add(cost);
    character.exp = character.exp.saturating_add(cost);
    character.values[1][value] = character.values[1][value].saturating_add(1);
    if character.values[0][value] < character.values[1][value] {
        character.values[0][value] = character.values[1][value];
    }
    Some(cost)
}

/// Raises the bare value of `value` by 1, spending already-earned exp
/// (`exp_used` vs `exp`) without granting new exp.
///
/// Mirrors C `raise_value` (`src/system/skill.c`), which is what `cl_raise`
/// (`src/system/player.c`) calls for `CL_RAISE`. Unlike `raise_value_exp`
/// (used by scrolls/shrines), this does not add to `character.exp` and it
/// checks `CF_NOEXP` itself (the scroll path checks it before calling
/// `raise_value_exp`, but `cl_raise` has no such caller-side guard).
pub(crate) fn raise_value(character: &mut Character, value: usize) -> Option<u32> {
    if character.flags.contains(CharacterFlags::NOEXP) {
        return None;
    }
    if value >= CHARACTER_VALUE_COUNT || skill_raise_cost_factor(value) == 0 {
        return None;
    }
    let current = bare_value(character, value);
    if current <= 0 || current >= skillmax(character) {
        return None;
    }
    if value == CharacterValue::Profession as usize && current > 99 {
        return None;
    }

    let seyan = character.flags.contains(CharacterFlags::WARRIOR)
        && character.flags.contains(CharacterFlags::MAGE);
    let cost = raise_cost(value, current, seyan);
    // C: `if (ch[cn].exp_used + cost > ch[cn].exp) return 0;`
    if character.exp_used.saturating_add(cost) > character.exp {
        return None;
    }

    character.exp_used = character.exp_used.saturating_add(cost);
    character.values[1][value] = character.values[1][value].saturating_add(1);
    if character.values[0][value] < character.values[1][value] {
        character.values[0][value] = character.values[1][value];
    }
    Some(cost)
}

pub(crate) fn lower_value(character: &mut Character, value: usize) -> Option<u32> {
    if character.flags.contains(CharacterFlags::NOEXP)
        || value >= CHARACTER_VALUE_COUNT
        || skill_raise_cost_factor(value) == 0
    {
        return None;
    }
    let current = bare_value(character, value);
    if i32::from(current) <= skill_start(value) {
        return None;
    }

    let seyan = character.flags.contains(CharacterFlags::WARRIOR)
        && character.flags.contains(CharacterFlags::MAGE);
    let lowered = current.saturating_sub(1);
    character.values[1][value] = lowered;
    let cost = raise_cost(value, lowered, seyan);
    character.exp_used = character.exp_used.saturating_sub(cost);
    character.flags.insert(CharacterFlags::UPDATE);
    Some(cost)
}

pub(crate) fn bare_value(character: &Character, value: usize) -> i16 {
    character
        .values
        .get(1)
        .and_then(|values| values.get(value))
        .copied()
        .unwrap_or_default()
}

pub(crate) fn skillmax(character: &Character) -> i16 {
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

pub(crate) fn raise_cost(value: usize, current: i16, seyan: bool) -> u32 {
    let nr = i32::from(current) - skill_start(value) + 1 + 5;
    let cost = nr * nr * nr * i32::from(skill_raise_cost_factor(value));
    let cost = if seyan { cost * 4 / 30 } else { cost / 10 };
    cost.max(1) as u32
}

pub(crate) fn skill_start(value: usize) -> i32 {
    match value {
        0..=6 => 10,
        42 => -1,
        11..=41 => 1,
        _ => -1,
    }
}

pub(crate) fn skill_raise_cost_factor(value: usize) -> i16 {
    match value {
        0..=2 | 42 => 3,
        3..=6 => 2,
        11..=37 | 39 | 40 => 1,
        _ => 0,
    }
}

/// C `supermax_canraise` (`src/system/skill.c:103-144`): which skills
/// `supermax_driver` (`src/area/3/area3.c`) will raise past `skillmax`,
/// and the `V_MAX`-multiplier weight (`2` for the four attributes, `1`
/// for the fighting/misc/spell skills, `0` for everything else -
/// notably `V_EMPTY`(34) and `V_DEMON`(38) are excluded even though
/// they fall inside the `11..=40` skill range).
pub(crate) fn supermax_canraise(value: usize) -> i32 {
    match value {
        3..=6 => 2,
        11..=33 | 35..=37 | 39 | 40 => 1,
        _ => 0,
    }
}

/// C `supermax_cost` (`src/system/skill.c:146-156`).
pub(crate) fn supermax_cost(character: &Character, value: usize, current: i16) -> u32 {
    let seyan = character.flags.contains(CharacterFlags::WARRIOR)
        && character.flags.contains(CharacterFlags::MAGE);
    (supermax_canraise(value) as u32).saturating_mul(3_000_000) + raise_cost(value, current, seyan)
}
