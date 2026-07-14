//! Magic action family: shields, pulses, warcry, freeze, flash, fire and earth spells, heal, bless.

use super::*;

pub fn do_magicshield(
    character: &mut Character,
    map: &MapGrid,
    weather_movement_percent: i32,
) -> Result<(), DoError> {
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
    character.duration = speed_ticks_with_weather_movement(
        character_value(character, CharacterValue::Speed),
        character.speed_mode,
        DUR_MAGIC_ACTION,
        resolve_weather_movement_percent(character, map, weather_movement_percent),
    );
    if character.speed_mode == SpeedMode::Fast {
        character.endurance -= endurance_cost(character);
    }
    character.dir = bigdir(character.dir);
    Ok(())
}

pub fn do_pulse(
    character: &mut Character,
    map: &MapGrid,
    weather_movement_percent: i32,
) -> Result<(), DoError> {
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
    character.duration = speed_ticks_with_weather_movement(
        character_value(character, CharacterValue::Speed),
        character.speed_mode,
        DUR_MAGIC_ACTION,
        resolve_weather_movement_percent(character, map, weather_movement_percent),
    );
    if character.speed_mode == SpeedMode::Fast {
        character.endurance -= endurance_cost(character);
    }
    character.dir = bigdir(character.dir);
    Ok(())
}

pub fn do_warcry(
    character: &mut Character,
    items: &HashMap<ItemId, Item>,
    map: &MapGrid,
    weather_movement_percent: i32,
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
    character.duration = speed_ticks_with_weather_movement(
        character_value(character, CharacterValue::Speed),
        character.speed_mode,
        DUR_MAGIC_ACTION,
        resolve_weather_movement_percent(character, map, weather_movement_percent),
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

pub fn do_freeze(
    character: &mut Character,
    map: &MapGrid,
    weather_movement_percent: i32,
) -> Result<(), DoError> {
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
    character.duration = speed_ticks_with_weather_movement(
        character_value(character, CharacterValue::Speed),
        character.speed_mode,
        DUR_MAGIC_ACTION,
        resolve_weather_movement_percent(character, map, weather_movement_percent),
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
    map: &MapGrid,
    weather_movement_percent: i32,
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
    character.duration = speed_ticks_with_weather_movement(
        character_value(character, CharacterValue::Speed),
        character.speed_mode,
        DUR_MAGIC_ACTION,
        resolve_weather_movement_percent(character, map, weather_movement_percent),
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
    map: &MapGrid,
    weather_movement_percent: i32,
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
    character.duration = speed_ticks_with_weather_movement(
        character_value(character, CharacterValue::Speed),
        character.speed_mode,
        DUR_MAGIC_ACTION,
        resolve_weather_movement_percent(character, map, weather_movement_percent),
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
    map: &MapGrid,
    weather_movement_percent: i32,
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
    if !(1..MAX_MAP - 1).contains(&target_x) || !(1..MAX_MAP - 1).contains(&target_y) {
        return Err(DoError::IllegalCoords);
    }
    if character.mana < FIREBALL_COST {
        return Err(DoError::ManaLow);
    }

    let weather_movement_percent =
        resolve_weather_movement_percent(character, map, weather_movement_percent);
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
        character.duration = speed_ticks_with_weather_movement(
            character_value(character, CharacterValue::Speed),
            character.speed_mode,
            DUR_MAGIC_ACTION / 2,
            weather_movement_percent,
        );
        character.dir = direction as u8;
    } else {
        if may_add_spell(character, items, IDR_FIRERING, current_tick).is_none() {
            return Err(DoError::AlreadyWorking);
        }
        character.action = action::FIRERING;
        character.duration = speed_ticks_with_weather_movement(
            character_value(character, CharacterValue::Speed),
            character.speed_mode,
            DUR_MAGIC_ACTION,
            weather_movement_percent,
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
    map: &MapGrid,
    weather_movement_percent: i32,
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
    if !(1..MAX_MAP - 1).contains(&target_x) || !(1..MAX_MAP - 1).contains(&target_y) {
        return Err(DoError::IllegalCoords);
    }
    if character.mana < FLASH_COST {
        return Err(DoError::ManaLow);
    }

    let weather_movement_percent =
        resolve_weather_movement_percent(character, map, weather_movement_percent);
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
        character.duration = speed_ticks_with_weather_movement(
            character_value(character, CharacterValue::Speed),
            character.speed_mode,
            DUR_MAGIC_ACTION / 2,
            weather_movement_percent,
        );
        character.dir = direction as u8;
    } else {
        if may_add_spell(character, items, IDR_FLASH, current_tick).is_none() {
            return Err(DoError::AlreadyWorking);
        }
        character.action = action::FLASH;
        character.duration = speed_ticks_with_weather_movement(
            character_value(character, CharacterValue::Speed),
            character.speed_mode,
            DUR_MAGIC_ACTION,
            weather_movement_percent,
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
    map: &MapGrid,
    target_x: usize,
    target_y: usize,
    strength: i32,
    weather_movement_percent: i32,
) -> Result<(), DoError> {
    do_earth_spell(
        character,
        map,
        target_x,
        target_y,
        strength,
        action::EARTHRAIN,
        weather_movement_percent,
    )
}

pub fn do_earthmud(
    character: &mut Character,
    map: &MapGrid,
    target_x: usize,
    target_y: usize,
    strength: i32,
    weather_movement_percent: i32,
) -> Result<(), DoError> {
    do_earth_spell(
        character,
        map,
        target_x,
        target_y,
        strength,
        action::EARTHMUD,
        weather_movement_percent,
    )
}

fn do_earth_spell(
    character: &mut Character,
    map: &MapGrid,
    target_x: usize,
    target_y: usize,
    strength: i32,
    action_id: u16,
    weather_movement_percent: i32,
) -> Result<(), DoError> {
    if character.flags.contains(CharacterFlags::DEAD) {
        return Err(DoError::Dead);
    }
    if !(1..MAX_MAP - 1).contains(&target_x) || !(1..MAX_MAP - 1).contains(&target_y) {
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
    character.duration = speed_ticks_with_weather_movement(
        character_value(character, CharacterValue::Speed),
        character.speed_mode,
        DUR_MAGIC_ACTION,
        resolve_weather_movement_percent(character, map, weather_movement_percent),
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
    map: &MapGrid,
    weather_movement_percent: i32,
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

    let weather_movement_percent =
        resolve_weather_movement_percent(caster, map, weather_movement_percent);
    caster.mana -= spend.mana_cost;
    caster.act1 = target.id.0 as i32;
    caster.act2 = spend.amount;
    caster.dir = direction.unwrap_or_else(|| bigdir(caster.dir));
    if caster.id == target.id {
        caster.action = action::HEAL_SELF;
        caster.duration = speed_ticks_with_weather_movement(
            character_value(caster, CharacterValue::Speed),
            caster.speed_mode,
            DUR_MAGIC_ACTION,
            weather_movement_percent,
        );
    } else {
        caster.action = action::HEAL1;
        caster.duration = speed_ticks_with_weather_movement(
            character_value(caster, CharacterValue::Speed),
            caster.speed_mode,
            DUR_MAGIC_ACTION / 2,
            weather_movement_percent,
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
    map: &MapGrid,
    weather_movement_percent: i32,
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

    let weather_movement_percent =
        resolve_weather_movement_percent(caster, map, weather_movement_percent);
    caster.mana -= BLESS_COST;
    caster.act1 = target.id.0 as i32;
    caster.dir = direction.unwrap_or_else(|| bigdir(caster.dir));
    if caster.id == target.id {
        caster.action = action::BLESS_SELF;
        caster.duration = speed_ticks_with_weather_movement(
            character_value(caster, CharacterValue::Speed),
            caster.speed_mode,
            DUR_MAGIC_ACTION,
            weather_movement_percent,
        );
    } else {
        caster.action = action::BLESS1;
        caster.duration = speed_ticks_with_weather_movement(
            character_value(caster, CharacterValue::Speed),
            caster.speed_mode,
            DUR_MAGIC_ACTION / 2,
            weather_movement_percent,
        );
    }
    if caster.speed_mode == SpeedMode::Fast {
        caster.endurance -= endurance_cost(caster);
    }
    Ok(())
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
