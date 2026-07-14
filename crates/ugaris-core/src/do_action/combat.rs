//! Combat action family: attack setup/resolution and can-attack rules.

use super::*;

/// C `do_attack`/`act_attack` (`system/do.c:424`) calls `speed(cn, ...)`
/// unconditionally for every attack, folding `modify_movement_speed`'s
/// weather multiplier into the attack duration just like `do_walk`'s
/// `speed()` call - not just movement. `weather_movement_percent` is the
/// already-resolved percentage (100 = no change); the indoor check (C's
/// `map[m].flags & MF_INDOORS` in `modify_movement_speed`) is resolved here
/// from the attacker's *current* tile, mirroring `do_walk`.
pub fn do_attack(
    attacker: &mut Character,
    map: &MapGrid,
    defender: &Character,
    direction: u8,
    attack_variant: u16,
    weather_movement_percent: i32,
) -> Result<(), DoError> {
    if attacker.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }
    let (x, y) = action_target(attacker, direction)?;
    if !map.legacy_inner_bounds(x, y) {
        return Err(DoError::IllegalCoords);
    }
    if !character_reachable_around_tile(map, x, y, defender.id) {
        return Err(DoError::NoCharacter);
    }
    if defender.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }
    if !can_attack(attacker, defender, map) {
        return Err(DoError::IllegalAttack);
    }

    let attacker_indoors = map
        .tile(usize::from(attacker.x), usize::from(attacker.y))
        .is_some_and(|tile| tile.flags.contains(MapFlags::INDOORS));
    let effective_weather_movement_percent = if attacker_indoors {
        100
    } else {
        weather_movement_percent
    };

    attacker.action = attack_variant.clamp(action::ATTACK1, action::ATTACK3);
    attacker.act1 = defender.id.0 as i32;
    attacker.duration = speed_ticks_with_weather_movement(
        character_value(attacker, CharacterValue::Speed),
        attacker.speed_mode,
        DUR_COMBAT_ACTION,
        effective_weather_movement_percent,
    );
    if attacker.speed_mode == SpeedMode::Fast {
        attacker.endurance -= endurance_cost(attacker) * 2;
    }
    attacker.dir = direction;

    Ok(())
}

pub fn act_attack(
    attacker: &mut Character,
    defender: &mut Character,
    map: &MapGrid,
    items: &HashMap<ItemId, Item>,
    d100_roll: i32,
    d6_roll: i32,
) -> Option<AttackResolution> {
    let Ok((x, y)) = action_target(attacker, attacker.dir) else {
        return None;
    };
    if !map.legacy_inner_bounds(x, y) || attacker.act1 != defender.id.0 as i32 {
        return None;
    }
    if !character_reachable_around_tile(map, x, y, defender.id)
        || !can_attack(attacker, defender, map)
    {
        return None;
    }

    // C `act_attack` (act.c:747-748): `vcn = get_attack_skill(cn); vco =
    // get_parry_skill(co);` - the effective to-hit skill is the weapon/
    // hand-to-hand fight skill plus the raised Attack/Parry stat (or the
    // earth-demon/magic-shield/spellcaster fallback), never the raw
    // `V_ATTACK`/`V_PARRY` stat alone.
    let attack = attack_skill(
        character_value_present(attacker, CharacterValue::Attack) != 0,
        simple_baddy_fight_skill(attacker, items),
        character_value(attacker, CharacterValue::Attack),
        character_value(attacker, CharacterValue::Tactics),
        0, // C `ch[cn].rage`: not yet ported on `Character` (see `values.rs` doc comment).
        attacker.flags.contains(CharacterFlags::EDEMON),
        attacker.level as i32,
        spell_average(
            character_value(attacker, CharacterValue::Bless),
            character_value(attacker, CharacterValue::Heal),
            character_value(attacker, CharacterValue::Freeze),
            character_value(attacker, CharacterValue::MagicShield),
            character_value(attacker, CharacterValue::Flash),
            character_value(attacker, CharacterValue::Fireball),
            character_value(attacker, CharacterValue::Pulse),
        ),
    );
    let parry = parry_skill(
        character_value_present(defender, CharacterValue::Parry) != 0,
        simple_baddy_fight_skill(defender, items),
        character_value(defender, CharacterValue::Parry),
        character_value(defender, CharacterValue::Tactics),
        0, // C `ch[co].rage`: same not-yet-ported gap as above.
        defender.flags.contains(CharacterFlags::EDEMON),
        character_value_present(defender, CharacterValue::MagicShield) != 0,
        character_value(defender, CharacterValue::MagicShield),
        spell_average(
            character_value(defender, CharacterValue::Bless),
            character_value(defender, CharacterValue::Heal),
            character_value(defender, CharacterValue::Freeze),
            character_value(defender, CharacterValue::MagicShield),
            character_value(defender, CharacterValue::Flash),
            character_value(defender, CharacterValue::Fireball),
            character_value(defender, CharacterValue::Pulse),
        ),
    );
    let (attack, parry) = apply_facing_attack_bonus(
        attack,
        parry,
        is_facing(defender, attacker),
        is_back(defender, attacker),
        attacker
            .professions
            .get(profession::ASSASSIN)
            .copied()
            .unwrap_or_default() as i32,
        defender.action == action::IDLE,
    );
    let chance = attack_chance_for_diff(attack - parry);
    if !attack_roll_hits(d100_roll, chance.hit_chance) {
        return Some(AttackResolution {
            hit: false,
            attack_skill: attack,
            parry_skill: parry,
            hit_chance: chance.hit_chance,
            raw_damage: 0,
            armor_divisor: ATTACK_DIV,
            armor_percent: chance.armor_percent,
            shield_percent: direct_attack_shield_percent(chance.armor_percent),
            hp_damage: 0,
            shield_absorbed: 0,
        });
    }

    let damage_units = direct_attack_damage_units(
        character_value(attacker, CharacterValue::Weapon),
        d6_roll,
        attacker
            .professions
            .get(profession::ASSASSIN)
            .copied()
            .unwrap_or_default() as i32,
        is_back(defender, attacker),
        defender.action == action::IDLE,
    );
    let raw_damage = scaled_direct_attack_damage(damage_units);
    let shield_percent = direct_attack_shield_percent(chance.armor_percent);
    let reduced = reduce_hurt_by_armor_and_lifeshield(
        raw_damage,
        character_value(defender, CharacterValue::Armor),
        ATTACK_DIV,
        chance.armor_percent,
        defender.lifeshield,
        shield_percent,
    );

    Some(AttackResolution {
        hit: true,
        attack_skill: attack,
        parry_skill: parry,
        hit_chance: chance.hit_chance,
        raw_damage,
        armor_divisor: ATTACK_DIV,
        armor_percent: chance.armor_percent,
        shield_percent,
        hp_damage: reduced.hp_damage,
        shield_absorbed: reduced.shield_absorbed,
    })
}

pub fn can_attack(attacker: &Character, defender: &Character, map: &MapGrid) -> bool {
    can_attack_internal(attacker, defender, map, None, &NoClanAttackPolicy)
}

fn can_attack_internal(
    attacker: &Character,
    defender: &Character,
    map: &MapGrid,
    area_id: Option<u16>,
    clan_policy: &impl ClanAttackPolicy,
) -> bool {
    if defender.id == attacker.id || defender.flags.is_empty() {
        return false;
    }
    if defender
        .flags
        .intersects(CharacterFlags::DEAD | CharacterFlags::NOATTACK)
    {
        return false;
    }
    if attacker.id.0 == 0 {
        return true;
    }
    if attacker
        .flags
        .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
        && defender.flags.contains(CharacterFlags::NOPLRATT)
    {
        return false;
    }
    if defender
        .flags
        .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
        && attacker.flags.contains(CharacterFlags::NOPLRATT)
    {
        return false;
    }
    let attacker_flags = map
        .tile(usize::from(attacker.x), usize::from(attacker.y))
        .map(|tile| tile.flags)
        .unwrap_or_else(MapFlags::empty);
    let defender_flags = map
        .tile(usize::from(defender.x), usize::from(defender.y))
        .map(|tile| tile.flags)
        .unwrap_or_else(MapFlags::empty);
    if attacker_flags.contains(MapFlags::PEACE) || defender_flags.contains(MapFlags::PEACE) {
        return false;
    }
    if attacker_flags.contains(MapFlags::ARENA) || defender_flags.contains(MapFlags::ARENA) {
        let same_arena = attacker_flags.contains(MapFlags::ARENA)
            && defender_flags.contains(MapFlags::ARENA)
            && arena_tiles_connected(
                map,
                usize::from(attacker.x),
                usize::from(attacker.y),
                usize::from(defender.x),
                usize::from(defender.y),
            );
        if !same_arena {
            return false;
        }
        if attacker_flags.contains(MapFlags::CLAN)
            && defender_flags.contains(MapFlags::CLAN)
            && attacker.clan != 0
            && defender.clan != 0
            && (attacker.clan == defender.clan
                || clan_policy.are_allied(attacker.clan, defender.clan))
        {
            return false;
        }
        return true;
    }

    if attacker.clan != 0 && defender.clan != 0 && area_id != Some(1) {
        if attacker_flags.contains(MapFlags::CLAN)
            && defender_flags.contains(MapFlags::CLAN)
            && clan_policy.can_attack_inside_clan_area(attacker.clan, defender.clan)
        {
            return true;
        }
        if clan_policy.can_attack_outside_clan_area(attacker.clan, defender.clan)
            && attacker.level.abs_diff(defender.level) <= 3
        {
            return true;
        }
    }

    if attacker
        .flags
        .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
        && defender.flags.contains(CharacterFlags::PLAYERLIKE)
    {
        return false;
    }
    if defender
        .flags
        .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
        && attacker.flags.contains(CharacterFlags::PLAYERLIKE)
    {
        return false;
    }

    if attacker.flags.contains(CharacterFlags::PLAYER)
        && defender.flags.contains(CharacterFlags::PLAYER)
    {
        if let Some(area_id) = area_id {
            if area_id == 1 {
                return false;
            }
            if !attacker.flags.contains(CharacterFlags::PK)
                || !defender.flags.contains(CharacterFlags::PK)
            {
                return false;
            }
            if attacker.level.abs_diff(defender.level) > 3 {
                return false;
            }
            if !clan_policy.has_pk_hate(attacker, defender) {
                return false;
            }
        }
        return true;
    }

    if attacker.group != 0 && attacker.group == defender.group {
        return false;
    }
    if attacker.clan != 0 && attacker.clan == defender.clan {
        return false;
    }
    if attacker.clan != 0
        && defender.clan != 0
        && clan_policy.are_allied(attacker.clan, defender.clan)
    {
        return false;
    }
    true
}

fn arena_tiles_connected(
    map: &MapGrid,
    from_x: usize,
    from_y: usize,
    to_x: usize,
    to_y: usize,
) -> bool {
    if !map.legacy_inner_bounds(from_x, from_y) || !map.legacy_inner_bounds(to_x, to_y) {
        return false;
    }
    let Some(start) = map.tile(from_x, from_y) else {
        return false;
    };
    let Some(target) = map.tile(to_x, to_y) else {
        return false;
    };
    if !start.flags.contains(MapFlags::ARENA) || !target.flags.contains(MapFlags::ARENA) {
        return false;
    }

    let mut visited = vec![false; map.width() * map.height()];
    let mut queue = VecDeque::new();
    visited[from_x + from_y * map.width()] = true;
    queue.push_back((from_x, from_y));

    while let Some((x, y)) = queue.pop_front() {
        if x == to_x && y == to_y {
            return true;
        }
        for dy in -1isize..=1 {
            for dx in -1isize..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if nx < 0 || ny < 0 {
                    continue;
                }
                let nx = nx as usize;
                let ny = ny as usize;
                if !map.legacy_inner_bounds(nx, ny) {
                    continue;
                }
                let idx = nx + ny * map.width();
                if visited[idx] {
                    continue;
                }
                if map
                    .tile(nx, ny)
                    .is_some_and(|tile| tile.flags.contains(MapFlags::ARENA))
                {
                    visited[idx] = true;
                    queue.push_back((nx, ny));
                }
            }
        }
    }

    false
}

pub fn can_attack_in_area(
    attacker: &Character,
    defender: &Character,
    map: &MapGrid,
    area_id: u16,
) -> bool {
    can_attack_in_area_with_clan_policy(attacker, defender, map, area_id, &NoClanAttackPolicy)
}

pub fn can_attack_in_area_with_clan_policy(
    attacker: &Character,
    defender: &Character,
    map: &MapGrid,
    area_id: u16,
    clan_policy: &impl ClanAttackPolicy,
) -> bool {
    can_attack_internal(attacker, defender, map, Some(area_id), clan_policy)
}

/// C `edemon_reduction` (`system/tool.c:3171-3173`): `max(0, str -
/// ch[cn].value[0][V_DEMON])`. Shared by `sub_attack`'s earth-demon hit/miss
/// adjustment (act.c:498-501, not yet wired - `act_attack` doesn't call
/// `get_attack_skill`/`get_parry_skill`) and `do_walk`'s earthmud slowdown
/// below; `current_demon_value` is always the target character's *current*
/// (`value[0]`) `V_DEMON`, matching C's `ch[cn].value[0][V_DEMON]` read.
pub(crate) fn edemon_reduction(strength: i32, current_demon_value: i32) -> i32 {
    (strength - current_demon_value).max(0)
}

fn character_reachable_around_tile(
    map: &MapGrid,
    center_x: usize,
    center_y: usize,
    character_id: CharacterId,
) -> bool {
    for dy in -1..=1 {
        for dx in -1..=1 {
            let Some(x) = offset(center_x, dx) else {
                continue;
            };
            let Some(y) = offset(center_y, dy) else {
                continue;
            };
            if map.tile(x, y).map(|tile| tile.character) == Some(character_id.0 as u16) {
                return true;
            }
        }
    }
    false
}

fn is_facing(character: &Character, other: &Character) -> bool {
    Direction::try_from(character.dir)
        .map(|direction| {
            let (dx, dy) = direction.delta();
            i32::from(character.x) + i32::from(dx) == i32::from(other.x)
                && i32::from(character.y) + i32::from(dy) == i32::from(other.y)
        })
        .unwrap_or(false)
}

fn is_back(character: &Character, other: &Character) -> bool {
    Direction::try_from(character.dir)
        .map(|direction| {
            let (dx, dy) = direction.delta();
            i32::from(character.x) - i32::from(dx) == i32::from(other.x)
                && i32::from(character.y) - i32::from(dy) == i32::from(other.y)
        })
        .unwrap_or(false)
}

fn character_value(character: &Character, value: CharacterValue) -> i32 {
    character
        .values
        .first()
        .and_then(|values| values.get(value as usize))
        .copied()
        .map(i32::from)
        .unwrap_or_default()
}

/// C `ch[cn].value[1][x]`: the "present" flag `get_attack_skill`/
/// `get_parry_skill` (`tool.c:1206-1244`) branch on, distinct from the raw
/// base skill points `character_value` above returns (`ch[cn].value[0][x]`).
fn character_value_present(character: &Character, value: CharacterValue) -> i32 {
    character
        .values
        .get(1)
        .and_then(|values| values.get(value as usize))
        .copied()
        .map(i32::from)
        .unwrap_or_default()
}
