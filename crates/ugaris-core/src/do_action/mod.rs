use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{
    attack::{
        apply_facing_attack_bonus, attack_chance_for_diff, attack_roll_hits, attack_skill,
        direct_attack_damage_units, direct_attack_shield_percent, parry_skill,
        reduce_hurt_by_armor_and_lifeshield, scaled_direct_attack_damage, spell_average,
        ATTACK_DIV,
    },
    direction::Direction,
    entity::{Character, CharacterFlags, CharacterValue, Item, ItemFlags, SpeedMode, POWERSCALE},
    ids::{CharacterId, ItemId},
    legacy::{action, profession, MAX_MAP},
    map::{MapFlags, MapGrid},
    spell::{
        heal_spend, magicshield_spend, may_add_spell, pulse_spend, spell_power, BLESS_COST,
        FIREBALL_COST, FLASH_COST, FREEZE_COST, IDR_BLESS, IDR_FIRERING, IDR_FLASH, IDR_WARCRY,
    },
    tick::TICKS_PER_SECOND,
    world::simple_baddy_fight_skill,
};

pub const DUR_COMBAT_ACTION: i32 = 12;
pub const DUR_MISC_ACTION: i32 = 12;
pub const DUR_USE_ACTION: i32 = 8;
pub const DUR_MAGIC_ACTION: i32 = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum DoError {
    None = 0,
    IllegalCoords = 1,
    Blocked = 2,
    IllegalCharacterNumber = 3,
    IllegalDirection = 4,
    Confused = 5,
    NoItem = 6,
    NotTakeable = 7,
    HaveCursorItem = 8,
    NoCursorItem = 9,
    HaveItem = 10,
    IllegalInventoryPosition = 11,
    Requirements = 12,
    NoCharacter = 13,
    IllegalAttack = 14,
    IllegalItemNumber = 15,
    Dead = 16,
    ManaLow = 17,
    SelfTarget = 18,
    IllegalHurt = 19,
    NotVisible = 20,
    Unconscious = 21,
    UnknownSpell = 22,
    NotUsable = 23,
    NotBody = 24,
    UnknownSkill = 25,
    IllegalPosition = 26,
    NotContainer = 27,
    AlreadyWorking = 28,
    IllegalStoreNumber = 29,
    IllegalStorePosition = 30,
    SoldOut = 31,
    GoldLow = 32,
    QuestItem = 33,
    AccessDenied = 34,
    NotIdle = 35,
    NotPlayer = 36,
    NoEffect = 37,
    AlreadyThere = 38,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemUseRequest {
    pub character_id: CharacterId,
    pub item_id: ItemId,
    pub spec: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttackResolution {
    pub hit: bool,
    pub attack_skill: i32,
    pub parry_skill: i32,
    pub hit_chance: i32,
    pub raw_damage: i32,
    pub armor_divisor: i32,
    pub armor_percent: i32,
    pub shield_percent: i32,
    pub hp_damage: i32,
    pub shield_absorbed: i32,
}

pub trait ClanAttackPolicy {
    fn are_allied(&self, _attacker_clan: u16, _defender_clan: u16) -> bool {
        false
    }

    fn can_attack_inside_clan_area(&self, _attacker_clan: u16, _defender_clan: u16) -> bool {
        false
    }

    fn can_attack_outside_clan_area(&self, _attacker_clan: u16, _defender_clan: u16) -> bool {
        false
    }

    fn has_pk_hate(&self, _attacker: &Character, _defender: &Character) -> bool {
        false
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NoClanAttackPolicy;

impl ClanAttackPolicy for NoClanAttackPolicy {}

/// Resolves the effective weather movement percent for `character`'s
/// *current* tile, mirroring C `modify_movement_speed`'s
/// (`module/weather/weather.c:477-493`) indoor check
/// (`map[m].flags & MF_INDOORS` -> no speed reduction) that every
/// `speed(cn, ...)` call site in `do.c` relies on (`cn > 0` unconditionally
/// applies the weather multiplier, not just for movement/melee) - see
/// `do_walk`/`do_attack` above for the two call sites that already inline
/// this same check.
fn resolve_weather_movement_percent(
    character: &Character,
    map: &MapGrid,
    weather_movement_percent: i32,
) -> i32 {
    let indoors = map
        .tile(usize::from(character.x), usize::from(character.y))
        .is_some_and(|tile| tile.flags.contains(MapFlags::INDOORS));
    if indoors {
        100
    } else {
        weather_movement_percent
    }
}

pub fn advance_action_step(character: &mut Character) -> bool {
    character.step += 1;
    character.step >= character.duration
}

pub fn reset_action_after_act(character: &mut Character) {
    character.duration = 0;
    character.step = 0;
    character.action = 0;
}

fn action_target(character: &Character, direction: u8) -> Result<(usize, usize), DoError> {
    let direction = Direction::try_from(direction).map_err(|_| DoError::IllegalDirection)?;
    let (dx, dy) = direction.delta();
    let x = offset(usize::from(character.x), dx).ok_or(DoError::IllegalCoords)?;
    let y = offset(usize::from(character.y), dy).ok_or(DoError::IllegalCoords)?;
    Ok((x, y))
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

fn offset(value: usize, delta: i16) -> Option<usize> {
    if delta.is_negative() {
        value.checked_sub(delta.unsigned_abs() as usize)
    } else {
        value.checked_add(delta as usize)
    }
}

mod combat;
mod items;
mod magic;
mod movement;

pub use combat::*;
pub use items::*;
pub use magic::*;
pub use movement::*;

#[cfg(test)]
mod tests;
