//! Movement action family: idle, turn, walk, speed/endurance costs.

use super::*;

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
    speed_ticks_with_weather_movement(speedy, mode, ticks, 100)
}

/// C `speed()` (`tool.c:118-160`) folds `modify_movement_speed`'s weather
/// multiplier (`module/weather/weather.c:477-493`) directly into `speedy`
/// right before the final tick-scaling divisor `f` is computed:
/// `speedy = modify_movement_speed(cn, speedy)` runs after the mode
/// adjustment but before `f = 0.75 + speedy / 288.0`. `weather_movement_percent`
/// is the already-resolved percentage (100 = no change) after the caller has
/// applied C's `MOD_WEATHER_EFFECT_SLOW` flag gate and the indoor-tile check
/// (`map[m].flags & MF_INDOORS`) - `do_walk` resolves the indoor check itself
/// since it already has the character's current tile; the weather-flag gate
/// and `move_mod` table lookup live in `ugaris-server`'s weather module,
/// which has no `ugaris-core` visibility.
pub fn speed_ticks_with_weather_movement(
    speedy: i32,
    mode: SpeedMode,
    ticks: i32,
    weather_movement_percent: i32,
) -> i32 {
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

    speedy = speedy * weather_movement_percent / 100;

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
    weather_movement_percent: i32,
    earthmud_extra_cost: i32,
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
    let mut cost = movement_cost(character, current_tile, area_id, earthmud_extra_cost);
    // C `modify_movement_speed` (`module/weather/weather.c:477-493`) checks
    // the character's *current* (pre-move) tile - captured now before the
    // later mutable borrow of `map` for the target tile's `TMOVEBLOCK` flag.
    let current_tile_indoors = current_tile.flags.contains(MapFlags::INDOORS);

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

    let effective_weather_movement_percent = if current_tile_indoors {
        100
    } else {
        weather_movement_percent
    };
    character.action = action::WALK;
    character.duration = speed_ticks_with_weather_movement(
        character_value(character, CharacterValue::Speed),
        character.speed_mode,
        cost,
        effective_weather_movement_percent,
    );
    if character.speed_mode == SpeedMode::Fast {
        character.endurance -= endurance_cost(character);
    }
    character.tox = target_x as u16;
    character.toy = target_y as u16;
    character.dir = direction as u8;

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

/// C `do_walk` (`system/do.c:86-99`): base cost `8`, plus (for non-earth-
/// demons) `edemon_reduction(cn, ef[fn].strength) * 2` for every active
/// `EF_EARTHMUD` map effect on the character's current tile - the earthmud
/// spell's slowdown, from which earth demons are immune (they don't get
/// slowed by their own mud). `earthmud_extra_cost` is pre-computed by the
/// `World`-level caller (already `0` when the walker has `CF_EDEMON`,
/// mirroring C's `if (!(ch[cn].flags & CF_EDEMON))` gate around the whole
/// scan) since `do_action.rs` has no access to `World::effects`. The swamp-
/// sprite/underwater branches below *replace* `cost` outright for players,
/// exactly like C's unconditional `cost = ...` assignments - so on a muddy
/// swamp tile the earthmud bonus is silently discarded for players, an
/// authentic C quirk preserved here by adding `earthmud_extra_cost` first.
fn movement_cost(
    character: &Character,
    tile: &crate::map::MapTile,
    area_id: u16,
    earthmud_extra_cost: i32,
) -> i32 {
    let mut cost = 8 + earthmud_extra_cost;

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
