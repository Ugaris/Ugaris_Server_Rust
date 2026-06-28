use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{
    attack::{
        apply_facing_attack_bonus, attack_chance_for_diff, attack_roll_hits,
        direct_attack_damage_units, direct_attack_shield_percent,
        reduce_hurt_by_armor_and_lifeshield, scaled_direct_attack_damage, ATTACK_DIV,
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
        true
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NoClanAttackPolicy;

impl ClanAttackPolicy for NoClanAttackPolicy {}

pub fn do_idle(character: &mut Character, duration: i32) -> Result<(), DoError> {
    let max_duration = (TICKS_PER_SECOND as i32) * 2;
    let duration = duration.clamp(2, max_duration);

    character.action = action::IDLE;
    character.duration = duration;
    character.act1 = duration;

    Ok(())
}

pub fn turn(character: &mut Character, direction: u8) -> Result<bool, DoError> {
    if !(1..=8).contains(&direction) {
        return Err(DoError::IllegalDirection);
    }
    if character.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }

    let changed = character.dir != direction;
    character.dir = direction;

    Ok(changed)
}

pub fn speed_ticks(speedy: i32, mode: SpeedMode, ticks: i32) -> i32 {
    let mut speedy = if speedy > 0 {
        speedy / 2
    } else {
        ((speedy as f64) * 0.75) as i32
    };

    if mode == SpeedMode::Fast {
        speedy += 40;
    }
    if mode == SpeedMode::Stealth {
        speedy -= 40;
    }

    let f = (0.75 + speedy as f64 / 288.0).clamp(0.2, 2.0);
    ((ticks as f64 / f) as i32).clamp(2, 255)
}

pub fn speed_ticks_inverse(speedy: i32, mode: SpeedMode, ticks: i32) -> i32 {
    let mut speedy = if speedy > 0 {
        speedy / 2
    } else {
        ((speedy as f64) * 0.75) as i32
    };

    if mode == SpeedMode::Fast {
        speedy += 40;
    }
    if mode == SpeedMode::Stealth {
        speedy -= 40;
    }

    let f = (0.75 + speedy as f64 / 288.0).clamp(0.2, 2.0);
    ((ticks as f64 * f).ceil() as i32).clamp(2, 255)
}

pub fn endurance_cost(character: &Character) -> i32 {
    const END_COST: i32 = POWERSCALE / 4;
    let athlete = character
        .professions
        .get(profession::ATHLETE)
        .copied()
        .unwrap_or_default() as i32;

    if athlete != 0 {
        END_COST - (athlete * END_COST / 45)
    } else {
        END_COST
    }
}

pub fn do_walk(
    character: &mut Character,
    map: &mut MapGrid,
    direction: u8,
    area_id: u16,
) -> Result<(), DoError> {
    if character.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }

    let direction = Direction::try_from(direction).map_err(|_| DoError::IllegalDirection)?;
    let (dx, dy) = direction.delta();
    let diag = dx != 0 && dy != 0;
    let current_x = usize::from(character.x);
    let current_y = usize::from(character.y);
    let target_x = offset(current_x, dx).ok_or(DoError::IllegalCoords)?;
    let target_y = offset(current_y, dy).ok_or(DoError::IllegalCoords)?;

    if !map.legacy_inner_bounds(target_x, target_y) {
        return Err(DoError::IllegalCoords);
    }

    let current_tile = map
        .tile(current_x, current_y)
        .ok_or(DoError::IllegalCoords)?;
    let mut cost = movement_cost(character, current_tile, area_id);

    let target_tile = map.tile(target_x, target_y).ok_or(DoError::IllegalCoords)?;
    if target_tile
        .flags
        .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
    {
        return Err(DoError::Blocked);
    }

    if diag {
        let side_x = offset(current_x, dx).ok_or(DoError::IllegalCoords)?;
        let side_y = offset(current_y, dy).ok_or(DoError::IllegalCoords)?;
        if map.tile(side_x, current_y).is_none_or(|tile| {
            tile.flags
                .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
        }) || map.tile(current_x, side_y).is_none_or(|tile| {
            tile.flags
                .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
        }) {
            return Err(DoError::Blocked);
        }
        cost += cost / 2;
    }

    if target_tile.character != 0 {
        return Err(DoError::Confused);
    }

    map.tile_mut(target_x, target_y)
        .expect("target bounds already checked")
        .flags
        .insert(MapFlags::TMOVEBLOCK);

    character.action = action::WALK;
    character.duration = speed_ticks(
        character_value(character, CharacterValue::Speed),
        character.speed_mode,
        cost,
    );
    if character.speed_mode == SpeedMode::Fast {
        character.endurance -= endurance_cost(character);
    }
    character.tox = target_x as u16;
    character.toy = target_y as u16;
    character.dir = direction as u8;

    Ok(())
}

pub fn do_take(
    character: &mut Character,
    map: &MapGrid,
    item: &Item,
    direction: u8,
    can_carry: bool,
) -> Result<(), DoError> {
    if character.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }
    let (x, y) = action_target(character, direction)?;
    if !map.legacy_inner_bounds(x, y) {
        return Err(DoError::IllegalCoords);
    }
    let tile_item = map.tile(x, y).map(|tile| tile.item).unwrap_or_default();
    if tile_item == 0 {
        return Err(DoError::NoItem);
    }
    if tile_item != item.id.0 {
        return Err(DoError::Confused);
    }
    if character.cursor_item.is_some() {
        return Err(DoError::HaveCursorItem);
    }
    if !item.flags.contains(ItemFlags::TAKE) || !can_carry {
        return Err(DoError::NotTakeable);
    }

    set_timed_item_action(character, action::TAKE, item, direction, DUR_MISC_ACTION, 0);
    Ok(())
}

pub fn do_use(
    character: &mut Character,
    map: &MapGrid,
    item: &Item,
    direction: u8,
    spec: i32,
) -> Result<(), DoError> {
    if character.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }
    let (x, y) = action_target(character, direction)?;
    if !map.legacy_inner_bounds(x, y) {
        return Err(DoError::IllegalCoords);
    }
    let tile_item = map.tile(x, y).map(|tile| tile.item).unwrap_or_default();
    if tile_item == 0 {
        return Err(DoError::NoItem);
    }
    if tile_item != item.id.0 {
        return Err(DoError::Confused);
    }
    if !item.flags.contains(ItemFlags::USE) {
        return Err(DoError::NotUsable);
    }

    set_timed_item_action(
        character,
        action::USE,
        item,
        direction,
        DUR_USE_ACTION,
        spec,
    );
    Ok(())
}

pub fn do_drop(
    character: &mut Character,
    map: &MapGrid,
    item: &Item,
    direction: u8,
) -> Result<(), DoError> {
    if character.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }
    let (x, y) = action_target(character, direction)?;
    if character.cursor_item != Some(item.id) {
        return Err(DoError::NoCursorItem);
    }
    if item.flags.intersects(ItemFlags::QUEST | ItemFlags::NODROP) {
        return Err(DoError::QuestItem);
    }
    if !map.legacy_inner_bounds(x, y) {
        return Err(DoError::IllegalCoords);
    }
    let Some(tile) = map.tile(x, y) else {
        return Err(DoError::IllegalCoords);
    };
    if tile.item != 0 {
        return Err(DoError::HaveItem);
    }
    if tile
        .flags
        .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
    {
        return Err(DoError::Blocked);
    }

    set_timed_item_action(character, action::DROP, item, direction, DUR_MISC_ACTION, 0);
    Ok(())
}

pub fn do_attack(
    attacker: &mut Character,
    map: &MapGrid,
    defender: &Character,
    direction: u8,
    attack_variant: u16,
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

    attacker.action = attack_variant.clamp(action::ATTACK1, action::ATTACK3);
    attacker.act1 = defender.id.0 as i32;
    attacker.duration = speed_ticks(
        character_value(attacker, CharacterValue::Speed),
        attacker.speed_mode,
        DUR_COMBAT_ACTION,
    );
    if attacker.speed_mode == SpeedMode::Fast {
        attacker.endurance -= endurance_cost(attacker) * 2;
    }
    attacker.dir = direction;

    Ok(())
}

pub fn do_magicshield(character: &mut Character) -> Result<(), DoError> {
    if character.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }
    if character.flags.contains(CharacterFlags::NOMAGIC)
        && !character.flags.contains(CharacterFlags::NONOMAGIC)
    {
        return Err(DoError::Unconscious);
    }

    let skill = character_value(character, CharacterValue::MagicShield);
    if skill == 0 {
        return Err(DoError::UnknownSpell);
    }
    let Some(spend) = magicshield_spend(skill, character.lifeshield, character.mana) else {
        return Err(if character.mana < POWERSCALE {
            DoError::ManaLow
        } else {
            DoError::NoEffect
        });
    };

    character.mana -= spend.mana_cost;
    character.action = action::MAGICSHIELD;
    character.act1 = spend.amount;
    character.duration = speed_ticks(
        character_value(character, CharacterValue::Speed),
        character.speed_mode,
        DUR_MAGIC_ACTION,
    );
    if character.speed_mode == SpeedMode::Fast {
        character.endurance -= endurance_cost(character);
    }
    character.dir = bigdir(character.dir);
    Ok(())
}

pub fn do_pulse(character: &mut Character) -> Result<(), DoError> {
    if character.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }
    if character.flags.contains(CharacterFlags::NOMAGIC)
        && !character.flags.contains(CharacterFlags::NONOMAGIC)
    {
        return Err(DoError::Unconscious);
    }

    let pulse_power = spell_power(
        character_value(character, CharacterValue::Pulse),
        character_value(character, CharacterValue::Tactics),
    );
    if character_value(character, CharacterValue::Pulse) == 0 {
        return Err(DoError::UnknownSpell);
    }
    let Some(spend) = pulse_spend(pulse_power, character.mana) else {
        return Err(DoError::ManaLow);
    };

    character.mana -= spend.mana_cost;
    character.action = action::PULSE;
    character.act1 = spend.amount;
    character.duration = speed_ticks(
        character_value(character, CharacterValue::Speed),
        character.speed_mode,
        DUR_MAGIC_ACTION,
    );
    if character.speed_mode == SpeedMode::Fast {
        character.endurance -= endurance_cost(character);
    }
    character.dir = bigdir(character.dir);
    Ok(())
}

pub fn do_warcry(character: &mut Character, items: &HashMap<ItemId, Item>) -> Result<(), DoError> {
    if character.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }
    if character.flags.contains(CharacterFlags::NOMAGIC)
        && !character.flags.contains(CharacterFlags::NONOMAGIC)
    {
        return Err(DoError::Unconscious);
    }
    if warcried(character, items) {
        return Err(DoError::Unconscious);
    }

    let warcry = character_value(character, CharacterValue::Warcry);
    if warcry == 0 {
        return Err(DoError::UnknownSpell);
    }
    let warcry_endurance_cost = warcry * POWERSCALE / 3;
    if character.endurance < warcry_endurance_cost {
        return Err(DoError::ManaLow);
    }

    character.endurance -= warcry_endurance_cost;
    character.action = action::WARCRY;
    character.duration = speed_ticks(
        character_value(character, CharacterValue::Speed),
        character.speed_mode,
        DUR_MAGIC_ACTION,
    );
    if character.speed_mode == SpeedMode::Fast {
        character.endurance -= endurance_cost(character);
    }
    Ok(())
}

fn warcried(character: &Character, items: &HashMap<ItemId, Item>) -> bool {
    character.inventory[12..30]
        .iter()
        .flatten()
        .filter_map(|item_id| items.get(item_id))
        .find(|item| item.driver == IDR_WARCRY)
        .is_some_and(|item| item.modifier_value[0] < -100)
}

pub fn do_freeze(character: &mut Character) -> Result<(), DoError> {
    if character.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }
    if character.flags.contains(CharacterFlags::NOMAGIC)
        && !character.flags.contains(CharacterFlags::NONOMAGIC)
    {
        return Err(DoError::Unconscious);
    }
    if character_value(character, CharacterValue::Freeze) == 0 {
        return Err(DoError::UnknownSpell);
    }
    if character.mana < FREEZE_COST {
        return Err(DoError::ManaLow);
    }

    character.mana -= FREEZE_COST;
    character.action = action::FREEZE;
    character.duration = speed_ticks(
        character_value(character, CharacterValue::Speed),
        character.speed_mode,
        DUR_MAGIC_ACTION,
    );
    if character.speed_mode == SpeedMode::Fast {
        character.endurance -= endurance_cost(character);
    }
    character.dir = bigdir(character.dir);
    Ok(())
}

pub fn do_flash(
    character: &mut Character,
    items: &HashMap<ItemId, Item>,
    current_tick: u32,
) -> Result<(), DoError> {
    if character.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }
    if character.flags.contains(CharacterFlags::NOMAGIC)
        && !character.flags.contains(CharacterFlags::NONOMAGIC)
    {
        return Err(DoError::Unconscious);
    }
    if character_value(character, CharacterValue::Flash) == 0 {
        return Err(DoError::UnknownSpell);
    }
    if character.mana < FLASH_COST {
        return Err(DoError::ManaLow);
    }
    if may_add_spell(character, items, IDR_FLASH, current_tick).is_none() {
        return Err(DoError::AlreadyWorking);
    }

    character.mana -= FLASH_COST;
    character.action = action::FLASH;
    character.duration = speed_ticks(
        character_value(character, CharacterValue::Speed),
        character.speed_mode,
        DUR_MAGIC_ACTION,
    );
    if character.speed_mode == SpeedMode::Fast {
        character.endurance -= endurance_cost(character);
    }
    character.dir = bigdir(character.dir);
    Ok(())
}

pub fn do_firering(
    character: &mut Character,
    items: &HashMap<ItemId, Item>,
    current_tick: u32,
) -> Result<(), DoError> {
    if character.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }
    if character.flags.contains(CharacterFlags::NOMAGIC)
        && !character.flags.contains(CharacterFlags::NONOMAGIC)
    {
        return Err(DoError::Unconscious);
    }
    if warcried(character, items) {
        return Err(DoError::Unconscious);
    }
    if character_value(character, CharacterValue::Fireball) == 0 {
        return Err(DoError::UnknownSpell);
    }
    if character.mana < FIREBALL_COST {
        return Err(DoError::ManaLow);
    }
    if may_add_spell(character, items, IDR_FIRERING, current_tick).is_none() {
        return Err(DoError::AlreadyWorking);
    }

    character.mana -= FIREBALL_COST;
    character.action = action::FIRERING;
    character.duration = speed_ticks(
        character_value(character, CharacterValue::Speed),
        character.speed_mode,
        DUR_MAGIC_ACTION,
    );
    if character.speed_mode == SpeedMode::Fast {
        character.endurance -= endurance_cost(character);
    }
    character.dir = bigdir(character.dir);
    Ok(())
}

pub fn do_fireball(
    character: &mut Character,
    items: &HashMap<ItemId, Item>,
    target_x: usize,
    target_y: usize,
    current_tick: u32,
) -> Result<(), DoError> {
    if character.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }
    if character.flags.contains(CharacterFlags::NOMAGIC)
        && !character.flags.contains(CharacterFlags::NONOMAGIC)
    {
        return Err(DoError::Unconscious);
    }
    if warcried(character, items) {
        return Err(DoError::Unconscious);
    }
    if character_value(character, CharacterValue::Fireball) == 0 {
        return Err(DoError::UnknownSpell);
    }
    if target_x < 1 || target_x >= MAX_MAP - 1 || target_y < 1 || target_y >= MAX_MAP - 1 {
        return Err(DoError::IllegalCoords);
    }
    if character.mana < FIREBALL_COST {
        return Err(DoError::ManaLow);
    }

    let direction = offset_to_direction(
        usize::from(character.x),
        usize::from(character.y),
        target_x,
        target_y,
    );
    if let Some(direction) = direction {
        character.action = action::FIREBALL1;
        character.act1 = target_x as i32;
        character.act2 = target_y as i32;
        character.duration = speed_ticks(
            character_value(character, CharacterValue::Speed),
            character.speed_mode,
            DUR_MAGIC_ACTION / 2,
        );
        character.dir = direction as u8;
    } else {
        if may_add_spell(character, items, IDR_FIRERING, current_tick).is_none() {
            return Err(DoError::AlreadyWorking);
        }
        character.action = action::FIRERING;
        character.duration = speed_ticks(
            character_value(character, CharacterValue::Speed),
            character.speed_mode,
            DUR_MAGIC_ACTION,
        );
        character.dir = bigdir(character.dir);
    }

    character.mana -= FIREBALL_COST;
    if character.speed_mode == SpeedMode::Fast {
        character.endurance -= endurance_cost(character);
    }
    Ok(())
}

pub fn do_ball(
    character: &mut Character,
    items: &HashMap<ItemId, Item>,
    target_x: usize,
    target_y: usize,
    current_tick: u32,
) -> Result<(), DoError> {
    if character.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }
    if character.flags.contains(CharacterFlags::NOMAGIC)
        && !character.flags.contains(CharacterFlags::NONOMAGIC)
    {
        return Err(DoError::Unconscious);
    }
    if warcried(character, items) {
        return Err(DoError::Unconscious);
    }
    if character_value(character, CharacterValue::Flash) == 0 {
        return Err(DoError::UnknownSpell);
    }
    if target_x < 1 || target_x >= MAX_MAP - 1 || target_y < 1 || target_y >= MAX_MAP - 1 {
        return Err(DoError::IllegalCoords);
    }
    if character.mana < FLASH_COST {
        return Err(DoError::ManaLow);
    }

    let direction = offset_to_direction(
        usize::from(character.x),
        usize::from(character.y),
        target_x,
        target_y,
    );
    if let Some(direction) = direction {
        character.action = action::BALL1;
        character.act1 = target_x as i32;
        character.act2 = target_y as i32;
        character.duration = speed_ticks(
            character_value(character, CharacterValue::Speed),
            character.speed_mode,
            DUR_MAGIC_ACTION / 2,
        );
        character.dir = direction as u8;
    } else {
        if may_add_spell(character, items, IDR_FLASH, current_tick).is_none() {
            return Err(DoError::AlreadyWorking);
        }
        character.action = action::FLASH;
        character.duration = speed_ticks(
            character_value(character, CharacterValue::Speed),
            character.speed_mode,
            DUR_MAGIC_ACTION,
        );
        character.dir = bigdir(character.dir);
    }

    character.mana -= FLASH_COST;
    if character.speed_mode == SpeedMode::Fast {
        character.endurance -= endurance_cost(character);
    }
    Ok(())
}

pub fn do_earthrain(
    character: &mut Character,
    target_x: usize,
    target_y: usize,
    strength: i32,
) -> Result<(), DoError> {
    do_earth_spell(character, target_x, target_y, strength, action::EARTHRAIN)
}

pub fn do_earthmud(
    character: &mut Character,
    target_x: usize,
    target_y: usize,
    strength: i32,
) -> Result<(), DoError> {
    do_earth_spell(character, target_x, target_y, strength, action::EARTHMUD)
}

fn do_earth_spell(
    character: &mut Character,
    target_x: usize,
    target_y: usize,
    strength: i32,
    action_id: u16,
) -> Result<(), DoError> {
    if character.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }
    if target_x < 1 || target_x >= MAX_MAP - 1 || target_y < 1 || target_y >= MAX_MAP - 1 {
        return Err(DoError::IllegalCoords);
    }
    let Some(direction) = offset_to_direction(
        usize::from(character.x),
        usize::from(character.y),
        target_x,
        target_y,
    ) else {
        return Err(DoError::SelfTarget);
    };
    if character.hp - POWERSCALE < strength * 100 {
        return Err(DoError::ManaLow);
    }

    character.hp -= strength * 100;
    character.action = action_id;
    character.act1 = (target_x + target_y * MAX_MAP) as i32;
    character.act2 = strength;
    character.duration = speed_ticks(
        character_value(character, CharacterValue::Speed),
        character.speed_mode,
        DUR_MAGIC_ACTION,
    );
    if character.speed_mode == SpeedMode::Fast {
        character.endurance -= endurance_cost(character);
    }
    character.dir = direction as u8;
    Ok(())
}

pub fn do_heal(
    caster: &mut Character,
    target: &Character,
    direction: Option<u8>,
) -> Result<(), DoError> {
    if caster.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }
    if caster.flags.contains(CharacterFlags::NOMAGIC)
        && !caster.flags.contains(CharacterFlags::NONOMAGIC)
    {
        return Err(DoError::Unconscious);
    }
    if target.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }
    if character_value(caster, CharacterValue::Heal) == 0 {
        return Err(DoError::UnknownSpell);
    }
    let missing_hp = character_value(target, CharacterValue::Hp) * POWERSCALE - target.hp;
    let Some(spend) = heal_spend(
        character_value(caster, CharacterValue::Heal),
        missing_hp,
        caster.mana,
    ) else {
        return Err(if caster.mana < POWERSCALE {
            DoError::ManaLow
        } else {
            DoError::NoEffect
        });
    };

    caster.mana -= spend.mana_cost;
    caster.act1 = target.id.0 as i32;
    caster.act2 = spend.amount;
    caster.dir = direction.unwrap_or_else(|| bigdir(caster.dir));
    if caster.id == target.id {
        caster.action = action::HEAL_SELF;
        caster.duration = speed_ticks(
            character_value(caster, CharacterValue::Speed),
            caster.speed_mode,
            DUR_MAGIC_ACTION,
        );
    } else {
        caster.action = action::HEAL1;
        caster.duration = speed_ticks(
            character_value(caster, CharacterValue::Speed),
            caster.speed_mode,
            DUR_MAGIC_ACTION / 2,
        );
    }
    if caster.speed_mode == SpeedMode::Fast {
        caster.endurance -= endurance_cost(caster);
    }
    Ok(())
}

pub fn do_bless(
    caster: &mut Character,
    target: &Character,
    items: &HashMap<ItemId, Item>,
    current_tick: u32,
    direction: Option<u8>,
) -> Result<(), DoError> {
    if caster.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }
    if caster.flags.contains(CharacterFlags::NOMAGIC)
        && !caster.flags.contains(CharacterFlags::NONOMAGIC)
    {
        return Err(DoError::Unconscious);
    }
    if target.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }
    if character_value(caster, CharacterValue::Bless) == 0 {
        return Err(DoError::UnknownSpell);
    }
    if caster.mana < BLESS_COST {
        return Err(DoError::ManaLow);
    }
    if caster.flags.contains(CharacterFlags::PLAYER)
        && !target
            .flags
            .intersects(CharacterFlags::PLAYER | CharacterFlags::PLAYERLIKE)
    {
        return Err(DoError::NotPlayer);
    }
    if may_add_spell(target, items, IDR_BLESS, current_tick).is_none() {
        return Err(DoError::AlreadyWorking);
    }
    if caster.id != target.id
        && caster.flags.contains(CharacterFlags::PLAYER)
        && target.flags.contains(CharacterFlags::NOBLESS)
    {
        return Err(DoError::IllegalAttack);
    }

    caster.mana -= BLESS_COST;
    caster.act1 = target.id.0 as i32;
    caster.dir = direction.unwrap_or_else(|| bigdir(caster.dir));
    if caster.id == target.id {
        caster.action = action::BLESS_SELF;
        caster.duration = speed_ticks(
            character_value(caster, CharacterValue::Speed),
            caster.speed_mode,
            DUR_MAGIC_ACTION,
        );
    } else {
        caster.action = action::BLESS1;
        caster.duration = speed_ticks(
            character_value(caster, CharacterValue::Speed),
            caster.speed_mode,
            DUR_MAGIC_ACTION / 2,
        );
    }
    if caster.speed_mode == SpeedMode::Fast {
        caster.endurance -= endurance_cost(caster);
    }
    Ok(())
}

pub fn act_walk(character: &mut Character, map: &mut MapGrid) -> bool {
    let from_x = usize::from(character.x);
    let from_y = usize::from(character.y);
    let to_x = usize::from(character.tox);
    let to_y = usize::from(character.toy);

    if !map.legacy_inner_bounds(to_x, to_y) {
        character.tox = 0;
        character.toy = 0;
        return false;
    }

    if let Some(tile) = map.tile_mut(from_x, from_y) {
        if tile.character == character.id.0 as u16 {
            tile.character = 0;
            tile.flags.remove(MapFlags::TMOVEBLOCK);
        }
    }

    character.x = character.tox;
    character.y = character.toy;
    character.tox = 0;
    character.toy = 0;

    if let Some(tile) = map.tile_mut(to_x, to_y) {
        tile.character = character.id.0 as u16;
        tile.flags.insert(MapFlags::TMOVEBLOCK);
        if tile.flags.contains(MapFlags::NOMAGIC) {
            character.flags.insert(CharacterFlags::NOMAGIC);
        } else {
            character.flags.remove(CharacterFlags::NOMAGIC);
        }
        true
    } else {
        false
    }
}

pub fn act_take(
    character: &mut Character,
    map: &mut MapGrid,
    item: &mut Item,
    can_carry: bool,
) -> bool {
    if character.cursor_item.is_some() {
        return false;
    }
    let Ok((x, y)) = action_target(character, character.dir) else {
        return false;
    };
    if !map.legacy_inner_bounds(x, y) {
        return false;
    }
    if map.tile(x, y).map(|tile| tile.item) != Some(item.id.0) {
        return false;
    }
    if character.act1 != item.id.0 as i32 || !item.flags.contains(ItemFlags::TAKE) || !can_carry {
        return false;
    }
    if !map.remove_item_map(item) {
        return false;
    }

    character.cursor_item = Some(item.id);
    item.carried_by = Some(character.id);
    character.flags.insert(CharacterFlags::ITEMS);
    true
}

pub fn act_drop(character: &mut Character, map: &mut MapGrid, item: &mut Item) -> bool {
    if character.cursor_item != Some(item.id) || character.act1 != item.id.0 as i32 {
        return false;
    }
    if item.flags.intersects(ItemFlags::QUEST | ItemFlags::NODROP) {
        return false;
    }
    let Ok((x, y)) = action_target(character, character.dir) else {
        return false;
    };
    let Some(tile) = map.tile(x, y) else {
        return false;
    };
    if tile.item != 0
        || tile
            .flags
            .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK)
    {
        return false;
    }
    if !map.set_item_map(item, x, y) {
        return false;
    }

    character.cursor_item = None;
    character.flags.insert(CharacterFlags::ITEMS);
    true
}

pub fn act_use(character: &mut Character, map: &MapGrid, item: &Item) -> Option<ItemUseRequest> {
    let Ok((x, y)) = action_target(character, character.dir) else {
        return None;
    };
    if !map.legacy_inner_bounds(x, y) {
        return None;
    }
    if map.tile(x, y).map(|tile| tile.item) != Some(item.id.0) {
        return None;
    }
    if character.act1 != item.id.0 as i32 || !item.flags.contains(ItemFlags::USE) {
        return None;
    }

    Some(ItemUseRequest {
        character_id: character.id,
        item_id: item.id,
        spec: character.act2,
    })
}

pub fn act_attack(
    attacker: &mut Character,
    defender: &mut Character,
    map: &MapGrid,
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

    let attack = character_value(attacker, CharacterValue::Attack);
    let parry = character_value(defender, CharacterValue::Parry);
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
        raw_damage,
        armor_divisor: ATTACK_DIV,
        armor_percent: chance.armor_percent,
        shield_percent,
        hp_damage: reduced.hp_damage,
        shield_absorbed: reduced.shield_absorbed,
    })
}

pub fn act_magicshield(character: &mut Character) -> bool {
    if character.act1 < 1 {
        return false;
    }
    if character.flags.contains(CharacterFlags::NOMAGIC)
        && !character.flags.contains(CharacterFlags::NONOMAGIC)
    {
        return false;
    }
    let max_lifeshield = character_value(character, CharacterValue::MagicShield) * POWERSCALE;
    character.lifeshield = max_lifeshield.min(character.lifeshield + character.act1);
    true
}

pub fn act_heal(caster: &Character, target: &mut Character) -> bool {
    if caster.act1 != target.id.0 as i32 || caster.act2 < 1 {
        return false;
    }
    if target.flags.contains(CharacterFlags::DEAD) {
        return false;
    }
    let max_hp = character_value(target, CharacterValue::Hp) * POWERSCALE;
    target.hp = max_hp.min(target.hp + caster.act2);
    true
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

    if let Some(area_id) = area_id {
        if attacker.flags.contains(CharacterFlags::PLAYER)
            && defender.flags.contains(CharacterFlags::PLAYER)
        {
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

pub fn advance_action_step(character: &mut Character) -> bool {
    character.step += 1;
    character.step >= character.duration
}

pub fn reset_action_after_act(character: &mut Character) {
    character.duration = 0;
    character.step = 0;
    character.action = 0;
}

fn movement_cost(character: &Character, tile: &crate::map::MapTile, area_id: u16) -> i32 {
    let mut cost = 8;

    if character.flags.contains(CharacterFlags::PLAYER) {
        let sprite = tile.ground_sprite & 0xffff;
        if (59405..=59413).contains(&sprite) {
            cost = 12;
        }
        if (59414..=59422).contains(&sprite) {
            cost = 16;
        }
        if (59423..=59431).contains(&sprite) {
            cost = 24;
        }
        if (20815..=20823).contains(&sprite) {
            cost = 36;
        }
        if (59706..=59709).contains(&sprite) && area_id == 29 {
            cost = 48;
        }
        if tile.flags.contains(MapFlags::UNDERWATER) {
            cost = 10;
        }
    }

    cost
}

fn set_timed_item_action(
    character: &mut Character,
    action_id: u16,
    item: &Item,
    direction: u8,
    duration: i32,
    act2: i32,
) {
    character.action = action_id;
    character.act1 = item.id.0 as i32;
    character.act2 = act2;
    character.duration = speed_ticks(
        character_value(character, CharacterValue::Speed),
        character.speed_mode,
        duration,
    );
    if character.speed_mode == SpeedMode::Fast {
        character.endurance -= endurance_cost(character);
    }
    character.dir = direction;
}

fn action_target(character: &Character, direction: u8) -> Result<(usize, usize), DoError> {
    let direction = Direction::try_from(direction).map_err(|_| DoError::IllegalDirection)?;
    let (dx, dy) = direction.delta();
    let x = offset(usize::from(character.x), dx).ok_or(DoError::IllegalCoords)?;
    let y = offset(usize::from(character.y), dy).ok_or(DoError::IllegalCoords)?;
    Ok((x, y))
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

fn bigdir(direction: u8) -> u8 {
    match Direction::try_from(direction) {
        Ok(Direction::RightUp | Direction::RightDown) => Direction::Right as u8,
        Ok(Direction::LeftUp | Direction::LeftDown) => Direction::Left as u8,
        Ok(direction) => direction as u8,
        Err(_) => direction,
    }
}

fn offset_to_direction(
    from_x: usize,
    from_y: usize,
    to_x: usize,
    to_y: usize,
) -> Option<Direction> {
    let mut dx = to_x as i32 - from_x as i32;
    let mut dy = to_y as i32 - from_y as i32;

    if dx.abs() / 2 > dy.abs() {
        dy = 0;
    }
    if dy.abs() / 2 > dx.abs() {
        dx = 0;
    }

    match (dx.signum(), dy.signum()) {
        (1, 1) => Some(Direction::RightDown),
        (1, -1) => Some(Direction::RightUp),
        (1, 0) => Some(Direction::Right),
        (-1, 1) => Some(Direction::LeftDown),
        (-1, -1) => Some(Direction::LeftUp),
        (-1, 0) => Some(Direction::Left),
        (0, 1) => Some(Direction::Down),
        (0, -1) => Some(Direction::Up),
        _ => None,
    }
}

fn offset(value: usize, delta: i16) -> Option<usize> {
    if delta.is_negative() {
        value.checked_sub(delta.unsigned_abs() as usize)
    } else {
        value.checked_add(delta as usize)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        entity::{CharacterFlags, SpeedMode},
        ids::CharacterId,
    };

    use super::*;

    #[test]
    fn error_codes_match_c_header() {
        assert_eq!(DoError::None as u8, 0);
        assert_eq!(DoError::Blocked as u8, 2);
        assert_eq!(DoError::Dead as u8, 16);
        assert_eq!(DoError::AlreadyThere as u8, 38);
    }

    #[test]
    fn action_duration_constants_match_do_c() {
        assert_eq!(DUR_COMBAT_ACTION, 12);
        assert_eq!(DUR_MISC_ACTION, 12);
        assert_eq!(DUR_USE_ACTION, 8);
        assert_eq!(DUR_MAGIC_ACTION, 12);
    }

    #[test]
    fn do_idle_clamps_duration_and_sets_action_fields() {
        let mut character = character();

        do_idle(&mut character, 1).unwrap();
        assert_eq!(character.action, action::IDLE);
        assert_eq!(character.duration, 2);
        assert_eq!(character.act1, 2);

        do_idle(&mut character, 1000).unwrap();
        assert_eq!(character.duration, 48);
        assert_eq!(character.act1, 48);
    }

    #[test]
    fn turn_rejects_invalid_direction_and_dead_characters() {
        let mut character = character();
        assert_eq!(turn(&mut character, 0), Err(DoError::IllegalDirection));
        assert_eq!(turn(&mut character, 9), Err(DoError::IllegalDirection));

        character.flags.insert(CharacterFlags::DEAD);
        assert_eq!(turn(&mut character, 1), Err(DoError::Dead));
    }

    #[test]
    fn turn_sets_direction_and_reports_sector_dirty_only_on_change() {
        let mut character = character();

        assert_eq!(turn(&mut character, 3), Ok(true));
        assert_eq!(character.dir, 3);
        assert_eq!(turn(&mut character, 3), Ok(false));
        assert_eq!(character.dir, 3);
    }

    #[test]
    fn speed_ticks_matches_legacy_formula_without_weather_modifier() {
        assert_eq!(speed_ticks(0, SpeedMode::Normal, 8), 10);
        assert_eq!(speed_ticks(40, SpeedMode::Fast, 8), 8);
        assert_eq!(speed_ticks(-40, SpeedMode::Stealth, 8), 15);
        assert_eq!(speed_ticks(1000, SpeedMode::Fast, 8), 4);
    }

    #[test]
    fn speed_ticks_inverse_matches_legacy_formula_without_weather_modifier() {
        assert_eq!(speed_ticks_inverse(0, SpeedMode::Normal, 50), 38);
        assert_eq!(speed_ticks_inverse(-420, SpeedMode::Normal, 50), 10);
        assert_eq!(speed_ticks_inverse(40, SpeedMode::Fast, 8), 8);
        assert_eq!(speed_ticks_inverse(1000, SpeedMode::Fast, 8), 16);
    }

    #[test]
    fn endurance_cost_uses_athlete_profession_reduction() {
        let mut character = character();
        assert_eq!(endurance_cost(&character), 250);
        character.professions[profession::ATHLETE] = 9;
        assert_eq!(endurance_cost(&character), 200);
    }

    #[test]
    fn do_walk_reserves_target_and_sets_action_fields() {
        let mut map = MapGrid::new(20, 20);
        let mut character = character();
        character.flags.insert(CharacterFlags::PLAYER);
        character.x = 10;
        character.y = 10;

        do_walk(&mut character, &mut map, Direction::Right as u8, 1).unwrap();

        assert_eq!(character.action, action::WALK);
        assert_eq!(character.tox, 11);
        assert_eq!(character.toy, 10);
        assert_eq!(character.dir, Direction::Right as u8);
        assert_eq!(character.duration, speed_ticks(0, SpeedMode::Normal, 8));
        assert!(map
            .tile(11, 10)
            .unwrap()
            .flags
            .contains(MapFlags::TMOVEBLOCK));
    }

    #[test]
    fn do_walk_rejects_blocked_and_corner_cutting_moves() {
        let mut map = MapGrid::new(20, 20);
        let mut character = character();
        character.x = 10;
        character.y = 10;
        map.set_flags(11, 10, MapFlags::MOVEBLOCK);

        assert_eq!(
            do_walk(&mut character, &mut map, Direction::Right as u8, 1),
            Err(DoError::Blocked)
        );

        map.set_flags(11, 10, MapFlags::empty());
        map.set_flags(10, 11, MapFlags::MOVEBLOCK);
        assert_eq!(
            do_walk(&mut character, &mut map, Direction::RightDown as u8, 1),
            Err(DoError::Blocked)
        );
    }

    #[test]
    fn do_walk_applies_terrain_and_fast_endurance_costs() {
        let mut map = MapGrid::new(20, 20);
        let mut character = character();
        character.flags.insert(CharacterFlags::PLAYER);
        character.speed_mode = SpeedMode::Fast;
        character.endurance = 1000;
        character.x = 10;
        character.y = 10;
        map.tile_mut(10, 10).unwrap().ground_sprite = 59423;

        do_walk(&mut character, &mut map, Direction::Right as u8, 1).unwrap();

        assert_eq!(character.duration, speed_ticks(0, SpeedMode::Fast, 24));
        assert_eq!(character.endurance, 750);
    }

    #[test]
    fn do_take_validates_cursor_item_and_takeable_flag() {
        let mut map = MapGrid::new(20, 20);
        let mut character = character();
        character.x = 10;
        character.y = 10;
        let item = item(7, ItemFlags::USED | ItemFlags::TAKE);
        map.tile_mut(11, 10).unwrap().item = item.id.0;

        do_take(&mut character, &map, &item, Direction::Right as u8, true).unwrap();
        assert_eq!(character.action, action::TAKE);
        assert_eq!(character.act1, 7);
        assert_eq!(
            character.duration,
            speed_ticks(0, SpeedMode::Normal, DUR_MISC_ACTION)
        );

        character.cursor_item = Some(item.id);
        assert_eq!(
            do_take(&mut character, &map, &item, Direction::Right as u8, true),
            Err(DoError::HaveCursorItem)
        );
    }

    #[test]
    fn do_use_sets_spec_and_rejects_non_useable_items() {
        let mut map = MapGrid::new(20, 20);
        let mut character = character();
        character.x = 10;
        character.y = 10;
        let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
        map.tile_mut(11, 10).unwrap().item = item.id.0;

        do_use(&mut character, &map, &item, Direction::Right as u8, 42).unwrap();
        assert_eq!(character.action, action::USE);
        assert_eq!((character.act1, character.act2), (7, 42));

        item.flags.remove(ItemFlags::USE);
        assert_eq!(
            do_use(&mut character, &map, &item, Direction::Right as u8, 0),
            Err(DoError::NotUsable)
        );
    }

    #[test]
    fn do_drop_validates_cursor_item_target_and_drop_flags() {
        let mut map = MapGrid::new(20, 20);
        let mut character = character();
        character.x = 10;
        character.y = 10;
        let item = item(7, ItemFlags::USED | ItemFlags::TAKE);

        assert_eq!(
            do_drop(&mut character, &map, &item, Direction::Right as u8),
            Err(DoError::NoCursorItem)
        );

        character.cursor_item = Some(item.id);
        do_drop(&mut character, &map, &item, Direction::Right as u8).unwrap();
        assert_eq!(character.action, action::DROP);
        assert_eq!(character.act1, 7);

        map.tile_mut(11, 10).unwrap().item = 99;
        assert_eq!(
            do_drop(&mut character, &map, &item, Direction::Right as u8),
            Err(DoError::HaveItem)
        );
    }

    #[test]
    fn act_walk_moves_character_from_reserved_target() {
        let mut map = MapGrid::new(20, 20);
        let mut character = character();
        character.x = 10;
        character.y = 10;
        character.tox = 11;
        character.toy = 10;
        map.tile_mut(10, 10).unwrap().character = character.id.0 as u16;
        map.tile_mut(10, 10)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK);
        map.tile_mut(11, 10)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK);

        assert!(act_walk(&mut character, &mut map));
        assert_eq!((character.x, character.y), (11, 10));
        assert_eq!((character.tox, character.toy), (0, 0));
        assert_eq!(map.tile(10, 10).unwrap().character, 0);
        assert!(!map
            .tile(10, 10)
            .unwrap()
            .flags
            .contains(MapFlags::TMOVEBLOCK));
        assert_eq!(map.tile(11, 10).unwrap().character, character.id.0 as u16);
    }

    #[test]
    fn act_take_removes_map_item_and_places_it_on_cursor() {
        let mut map = MapGrid::new(20, 20);
        let mut character = character();
        character.x = 10;
        character.y = 10;
        character.dir = Direction::Right as u8;
        character.act1 = 7;
        let mut item = item(7, ItemFlags::USED | ItemFlags::TAKE);
        assert!(map.set_item_map(&mut item, 11, 10));

        assert!(act_take(&mut character, &mut map, &mut item, true));
        assert_eq!(map.tile(11, 10).unwrap().item, 0);
        assert_eq!(character.cursor_item, Some(item.id));
        assert_eq!(item.carried_by, Some(character.id));
        assert!(character.flags.contains(CharacterFlags::ITEMS));
    }

    #[test]
    fn act_drop_places_cursor_item_on_map() {
        let mut map = MapGrid::new(20, 20);
        let mut character = character();
        character.x = 10;
        character.y = 10;
        character.dir = Direction::Right as u8;
        character.act1 = 7;
        let mut item = item(7, ItemFlags::USED | ItemFlags::TAKE);
        character.cursor_item = Some(item.id);
        item.carried_by = Some(character.id);

        assert!(act_drop(&mut character, &mut map, &mut item));
        assert_eq!(character.cursor_item, None);
        assert_eq!(item.carried_by, None);
        assert_eq!(map.tile(11, 10).unwrap().item, item.id.0);
        assert!(character.flags.contains(CharacterFlags::ITEMS));
    }

    #[test]
    fn act_use_validates_target_and_returns_driver_request() {
        let mut map = MapGrid::new(20, 20);
        let mut character = character();
        character.x = 10;
        character.y = 10;
        character.dir = Direction::Right as u8;
        character.act1 = 7;
        character.act2 = 42;
        let item = item(7, ItemFlags::USED | ItemFlags::USE);
        map.tile_mut(11, 10).unwrap().item = item.id.0;

        assert_eq!(
            act_use(&mut character, &map, &item),
            Some(ItemUseRequest {
                character_id: character.id,
                item_id: item.id,
                spec: 42,
            })
        );

        map.tile_mut(11, 10).unwrap().item = 0;
        assert_eq!(act_use(&mut character, &map, &item), None);
    }

    #[test]
    fn do_attack_sets_legacy_action_fields_and_fast_endurance_cost() {
        let mut map = MapGrid::new(20, 20);
        let mut attacker = character();
        let mut defender = character();
        defender.id = CharacterId(2);
        defender.x = 11;
        defender.y = 10;
        map.tile_mut(11, 10).unwrap().character = defender.id.0 as u16;
        attacker.speed_mode = SpeedMode::Fast;
        attacker.endurance = 1000;

        do_attack(
            &mut attacker,
            &map,
            &defender,
            Direction::Right as u8,
            action::ATTACK2,
        )
        .unwrap();

        assert_eq!(attacker.action, action::ATTACK2);
        assert_eq!(attacker.act1, defender.id.0 as i32);
        assert_eq!(attacker.dir, Direction::Right as u8);
        assert_eq!(
            attacker.duration,
            speed_ticks(0, SpeedMode::Fast, DUR_COMBAT_ACTION)
        );
        assert_eq!(attacker.endurance, 500);
    }

    #[test]
    fn do_attack_rejects_unreachable_dead_and_peace_zone_targets() {
        let mut map = MapGrid::new(20, 20);
        let mut attacker = character();
        let mut defender = character();
        defender.id = CharacterId(2);
        defender.x = 15;
        defender.y = 10;
        map.tile_mut(15, 10).unwrap().character = defender.id.0 as u16;

        assert_eq!(
            do_attack(
                &mut attacker,
                &map,
                &defender,
                Direction::Right as u8,
                action::ATTACK1,
            ),
            Err(DoError::NoCharacter)
        );

        defender.x = 11;
        defender.y = 10;
        map.tile_mut(15, 10).unwrap().character = 0;
        map.tile_mut(11, 10).unwrap().character = defender.id.0 as u16;
        defender.flags.insert(CharacterFlags::DEAD);
        assert_eq!(
            do_attack(
                &mut attacker,
                &map,
                &defender,
                Direction::Right as u8,
                action::ATTACK1,
            ),
            Err(DoError::Dead)
        );

        defender.flags.remove(CharacterFlags::DEAD);
        map.set_flags(10, 10, MapFlags::PEACE);
        assert_eq!(
            do_attack(
                &mut attacker,
                &map,
                &defender,
                Direction::Right as u8,
                action::ATTACK1,
            ),
            Err(DoError::IllegalAttack)
        );

        map.tile_mut(10, 10).unwrap().flags.remove(MapFlags::PEACE);
        attacker.clan = 42;
        defender.clan = 42;
        assert_eq!(
            do_attack(
                &mut attacker,
                &map,
                &defender,
                Direction::Right as u8,
                action::ATTACK1,
            ),
            Err(DoError::IllegalAttack)
        );
    }

    #[test]
    fn can_attack_in_area_blocks_legacy_area_one_player_vs_player() {
        let map = MapGrid::new(20, 20);
        let mut attacker = character();
        let mut defender = character();
        defender.id = CharacterId(2);
        defender.x = 11;
        defender.y = 10;
        attacker.flags.insert(CharacterFlags::PLAYER);
        defender.flags.insert(CharacterFlags::PLAYER);

        assert!(can_attack(&attacker, &defender, &map));
        assert!(!can_attack_in_area(&attacker, &defender, &map, 1));
    }

    #[test]
    fn can_attack_in_area_requires_pk_and_level_range_for_player_vs_player() {
        let map = MapGrid::new(20, 20);
        let mut attacker = character();
        let mut defender = character();
        defender.id = CharacterId(2);
        defender.x = 11;
        defender.y = 10;
        attacker.flags.insert(CharacterFlags::PLAYER);
        defender.flags.insert(CharacterFlags::PLAYER);

        assert!(!can_attack_in_area(&attacker, &defender, &map, 2));

        attacker.flags.insert(CharacterFlags::PK);
        assert!(!can_attack_in_area(&attacker, &defender, &map, 2));

        defender.flags.insert(CharacterFlags::PK);
        assert!(can_attack_in_area(&attacker, &defender, &map, 2));

        defender.level = attacker.level + 4;
        assert!(!can_attack_in_area(&attacker, &defender, &map, 2));
    }

    #[test]
    fn can_attack_requires_same_connected_arena() {
        let mut map = MapGrid::new(20, 20);
        let mut attacker = character();
        let mut defender = character();
        defender.id = CharacterId(2);
        attacker.x = 5;
        attacker.y = 5;
        defender.x = 6;
        defender.y = 5;

        map.tile_mut(5, 5).unwrap().flags.insert(MapFlags::ARENA);
        assert!(!can_attack(&attacker, &defender, &map));

        map.tile_mut(6, 5).unwrap().flags.insert(MapFlags::ARENA);
        assert!(can_attack(&attacker, &defender, &map));

        defender.x = 12;
        defender.y = 12;
        map.tile_mut(12, 12).unwrap().flags.insert(MapFlags::ARENA);
        assert!(!can_attack(&attacker, &defender, &map));

        for x in 5..=12 {
            map.tile_mut(x, 5).unwrap().flags.insert(MapFlags::ARENA);
        }
        for y in 5..=12 {
            map.tile_mut(12, y).unwrap().flags.insert(MapFlags::ARENA);
        }
        assert!(can_attack(&attacker, &defender, &map));
    }

    #[test]
    fn can_attack_blocks_same_group_npcs_outside_arena() {
        let map = MapGrid::new(20, 20);
        let mut attacker = character();
        let mut defender = character();
        defender.id = CharacterId(2);
        defender.x = 11;
        defender.y = 10;
        attacker.group = 7;
        defender.group = 7;

        assert!(!can_attack(&attacker, &defender, &map));
    }

    #[test]
    fn can_attack_arena_allows_same_clan_unless_clan_area() {
        let mut map = MapGrid::new(20, 20);
        let mut attacker = character();
        let mut defender = character();
        defender.id = CharacterId(2);
        attacker.x = 5;
        attacker.y = 5;
        defender.x = 6;
        defender.y = 5;
        attacker.clan = 42;
        defender.clan = 42;
        map.tile_mut(5, 5).unwrap().flags.insert(MapFlags::ARENA);
        map.tile_mut(6, 5).unwrap().flags.insert(MapFlags::ARENA);

        assert!(can_attack(&attacker, &defender, &map));

        map.tile_mut(5, 5).unwrap().flags.insert(MapFlags::CLAN);
        map.tile_mut(6, 5).unwrap().flags.insert(MapFlags::CLAN);

        assert!(!can_attack(&attacker, &defender, &map));
    }

    struct TestClanPolicy;

    impl ClanAttackPolicy for TestClanPolicy {
        fn are_allied(&self, attacker_clan: u16, defender_clan: u16) -> bool {
            (attacker_clan, defender_clan) == (10, 11)
        }

        fn can_attack_inside_clan_area(&self, attacker_clan: u16, defender_clan: u16) -> bool {
            (attacker_clan, defender_clan) == (20, 21)
        }

        fn can_attack_outside_clan_area(&self, attacker_clan: u16, defender_clan: u16) -> bool {
            (attacker_clan, defender_clan) == (30, 31)
        }

        fn has_pk_hate(&self, attacker: &Character, defender: &Character) -> bool {
            (attacker.id, defender.id) == (CharacterId(100), CharacterId(200))
        }
    }

    #[test]
    fn can_attack_clan_area_alliance_blocks_arena_attack() {
        let mut map = MapGrid::new(20, 20);
        let mut attacker = character();
        let mut defender = character();
        defender.id = CharacterId(2);
        attacker.x = 5;
        attacker.y = 5;
        defender.x = 6;
        defender.y = 5;
        attacker.clan = 10;
        defender.clan = 11;
        map.tile_mut(5, 5)
            .unwrap()
            .flags
            .insert(MapFlags::ARENA | MapFlags::CLAN);
        map.tile_mut(6, 5)
            .unwrap()
            .flags
            .insert(MapFlags::ARENA | MapFlags::CLAN);

        assert!(can_attack(&attacker, &defender, &map));
        assert!(!can_attack_in_area_with_clan_policy(
            &attacker,
            &defender,
            &map,
            2,
            &TestClanPolicy
        ));
    }

    #[test]
    fn can_attack_clan_war_inside_clan_area_before_pvp_pk_gate() {
        let mut map = MapGrid::new(20, 20);
        let mut attacker = character();
        let mut defender = character();
        defender.id = CharacterId(2);
        defender.x = 11;
        defender.y = 10;
        attacker.clan = 20;
        defender.clan = 21;
        attacker.flags.insert(CharacterFlags::PLAYER);
        defender.flags.insert(CharacterFlags::PLAYER);
        map.tile_mut(10, 10).unwrap().flags.insert(MapFlags::CLAN);
        map.tile_mut(11, 10).unwrap().flags.insert(MapFlags::CLAN);

        assert!(!can_attack_in_area(&attacker, &defender, &map, 2));
        assert!(can_attack_in_area_with_clan_policy(
            &attacker,
            &defender,
            &map,
            2,
            &TestClanPolicy
        ));
    }

    #[test]
    fn can_attack_clan_feud_outside_clan_area_uses_level_range_and_area_one_gate() {
        let map = MapGrid::new(20, 20);
        let mut attacker = character();
        let mut defender = character();
        defender.id = CharacterId(2);
        defender.x = 11;
        defender.y = 10;
        attacker.clan = 30;
        defender.clan = 31;
        attacker.flags.insert(CharacterFlags::PLAYER);
        defender.flags.insert(CharacterFlags::PLAYER);

        assert!(can_attack_in_area_with_clan_policy(
            &attacker,
            &defender,
            &map,
            2,
            &TestClanPolicy
        ));
        assert!(!can_attack_in_area_with_clan_policy(
            &attacker,
            &defender,
            &map,
            1,
            &TestClanPolicy
        ));

        defender.level = attacker.level + 4;
        assert!(!can_attack_in_area_with_clan_policy(
            &attacker,
            &defender,
            &map,
            2,
            &TestClanPolicy
        ));
    }

    #[test]
    fn can_attack_player_vs_player_uses_policy_hate_list_after_pk_checks() {
        let map = MapGrid::new(20, 20);
        let mut attacker = character();
        let mut defender = character();
        attacker.id = CharacterId(100);
        defender.id = CharacterId(200);
        defender.x = 11;
        defender.y = 10;
        attacker
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
        defender
            .flags
            .insert(CharacterFlags::PLAYER | CharacterFlags::PK);

        assert!(can_attack_in_area_with_clan_policy(
            &attacker,
            &defender,
            &map,
            2,
            &TestClanPolicy
        ));

        defender.id = CharacterId(201);
        assert!(!can_attack_in_area_with_clan_policy(
            &attacker,
            &defender,
            &map,
            2,
            &TestClanPolicy
        ));
    }

    #[test]
    fn act_attack_uses_strict_hit_roll_and_applies_damage() {
        let mut map = MapGrid::new(20, 20);
        let mut attacker = character();
        let mut defender = character();
        defender.id = CharacterId(2);
        defender.x = 11;
        defender.y = 10;
        defender.dir = Direction::Left as u8;
        defender.hp = 10_000;
        attacker.dir = Direction::Right as u8;
        attacker.act1 = defender.id.0 as i32;
        attacker.values[0][CharacterValue::Attack as usize] = 10;
        attacker.values[0][CharacterValue::Weapon as usize] = 10;
        defender.values[0][CharacterValue::Parry as usize] = 10;
        map.tile_mut(11, 10).unwrap().character = defender.id.0 as u16;

        assert_eq!(
            act_attack(&mut attacker, &mut defender, &map, 50, 6),
            Some(AttackResolution {
                hit: false,
                raw_damage: 0,
                armor_divisor: ATTACK_DIV,
                armor_percent: 90,
                shield_percent: 97,
                hp_damage: 0,
                shield_absorbed: 0,
            })
        );

        let result = act_attack(&mut attacker, &mut defender, &map, 49, 6).unwrap();
        assert!(result.hit);
        assert_eq!(result.raw_damage, 3200);
        assert_eq!(result.hp_damage, 3200);
        assert_eq!(defender.hp, 10_000);
    }

    #[test]
    fn do_fireball_sets_targeted_legacy_action() {
        let items = HashMap::new();
        let mut character = character();
        character.values[0][CharacterValue::Fireball as usize] = 50;
        character.mana = FIREBALL_COST;

        do_fireball(&mut character, &items, 15, 10, 0).unwrap();

        assert_eq!(character.action, action::FIREBALL1);
        assert_eq!(character.act1, 15);
        assert_eq!(character.act2, 10);
        assert_eq!(character.dir, Direction::Right as u8);
        assert_eq!(
            character.duration,
            speed_ticks(0, SpeedMode::Normal, DUR_MAGIC_ACTION / 2)
        );
        assert_eq!(character.mana, 0);
    }

    #[test]
    fn do_fireball_same_tile_sets_firering_action() {
        let items = HashMap::new();
        let mut character = character();
        character.values[0][CharacterValue::Fireball as usize] = 50;
        character.mana = FIREBALL_COST;
        character.dir = Direction::RightUp as u8;

        do_fireball(&mut character, &items, 10, 10, 0).unwrap();

        assert_eq!(character.action, action::FIRERING);
        assert_eq!(character.dir, Direction::Right as u8);
        assert_eq!(
            character.duration,
            speed_ticks(0, SpeedMode::Normal, DUR_MAGIC_ACTION)
        );
        assert_eq!(character.mana, 0);
    }

    #[test]
    fn do_earthrain_sets_legacy_action_and_hp_cost() {
        let mut character = character();
        character.hp = 10 * POWERSCALE;
        character.speed_mode = SpeedMode::Fast;

        do_earthrain(&mut character, 12, 10, 15).unwrap();

        assert_eq!(character.action, action::EARTHRAIN);
        assert_eq!(character.act1, (12 + 10 * MAX_MAP) as i32);
        assert_eq!(character.act2, 15);
        assert_eq!(character.dir, Direction::Right as u8);
        assert_eq!(character.hp, 10 * POWERSCALE - 1500);
        assert_eq!(
            character.duration,
            speed_ticks(0, SpeedMode::Fast, DUR_MAGIC_ACTION)
        );
        assert_eq!(character.endurance, -endurance_cost(&character));
    }

    #[test]
    fn do_earthmud_rejects_self_and_low_hp_like_c() {
        let mut character = character();
        character.hp = 2 * POWERSCALE;

        assert_eq!(
            do_earthmud(&mut character, 10, 10, 1),
            Err(DoError::SelfTarget)
        );
        assert_eq!(
            do_earthmud(&mut character, 11, 10, 11),
            Err(DoError::ManaLow)
        );
    }

    #[test]
    fn action_step_matches_tick_char_readiness_rule() {
        let mut character = character();
        character.duration = 2;

        assert!(!advance_action_step(&mut character));
        assert_eq!(character.step, 1);
        assert!(advance_action_step(&mut character));
        assert_eq!(character.step, 2);

        character.action = action::WALK;
        reset_action_after_act(&mut character);
        assert_eq!(character.action, 0);
        assert_eq!(character.duration, 0);
        assert_eq!(character.step, 0);
    }

    fn character() -> Character {
        Character {
            id: CharacterId(1),
            name: "Character".into(),
            description: String::new(),
            flags: CharacterFlags::USED,
            sprite: 0,
            driver: 0,
            group: 0,
            clan: 0,
            clan_rank: 0,
            clan_serial: 0,
            speed_mode: SpeedMode::Normal,
            x: 10,
            y: 10,
            rest_area: 0,
            rest_x: 0,
            rest_y: 0,
            tox: 0,
            toy: 0,
            dir: 0,
            action: 0,
            duration: 0,
            step: 0,
            act1: 0,
            act2: 0,
            hp: 0,
            mana: 0,
            endurance: 0,
            lifeshield: 0,
            level: 1,
            exp: 0,
            exp_used: 0,
            gold: 0,
            creation_time: 0,
            saves: 0,
            deaths: 0,
            regen_ticker: 0,
            cursor_item: None,
            current_container: None,
            values: Character::empty_values(),
            professions: Character::empty_professions(),
            inventory: Character::empty_inventory(),
            driver_state: None,
            driver_messages: Vec::new(),
        }
    }

    fn item(id: u32, flags: ItemFlags) -> Item {
        Item {
            id: crate::ids::ItemId(id),
            name: "Item".into(),
            description: String::new(),
            flags,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [0; crate::entity::MAX_MODIFIERS],
            modifier_value: [0; crate::entity::MAX_MODIFIERS],
            x: 0,
            y: 0,
            carried_by: None,
            contained_in: None,
            content_id: 0,
            driver: 0,
            driver_data: Vec::new(),
            serial: 0,
        }
    }
}
