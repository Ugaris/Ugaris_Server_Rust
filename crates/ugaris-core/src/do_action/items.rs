//! Item action family: take, use, drop.

use super::*;

pub fn do_take(
    character: &mut Character,
    map: &MapGrid,
    item: &Item,
    direction: u8,
    can_carry: bool,
    weather_movement_percent: i32,
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

    let weather_movement_percent =
        resolve_weather_movement_percent(character, map, weather_movement_percent);
    set_timed_item_action(
        character,
        action::TAKE,
        item,
        direction,
        DUR_MISC_ACTION,
        0,
        weather_movement_percent,
    );
    Ok(())
}

pub fn do_use(
    character: &mut Character,
    map: &MapGrid,
    item: &Item,
    direction: u8,
    spec: i32,
    weather_movement_percent: i32,
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

    let weather_movement_percent =
        resolve_weather_movement_percent(character, map, weather_movement_percent);
    set_timed_item_action(
        character,
        action::USE,
        item,
        direction,
        DUR_USE_ACTION,
        spec,
        weather_movement_percent,
    );
    Ok(())
}

pub fn do_drop(
    character: &mut Character,
    map: &MapGrid,
    item: &Item,
    direction: u8,
    weather_movement_percent: i32,
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

    let weather_movement_percent =
        resolve_weather_movement_percent(character, map, weather_movement_percent);
    set_timed_item_action(
        character,
        action::DROP,
        item,
        direction,
        DUR_MISC_ACTION,
        0,
        weather_movement_percent,
    );
    Ok(())
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

fn set_timed_item_action(
    character: &mut Character,
    action_id: u16,
    item: &Item,
    direction: u8,
    duration: i32,
    act2: i32,
    weather_movement_percent: i32,
) {
    character.action = action_id;
    character.act1 = item.id.0 as i32;
    character.act2 = act2;
    character.duration = speed_ticks_with_weather_movement(
        character_value(character, CharacterValue::Speed),
        character.speed_mode,
        duration,
        weather_movement_percent,
    );
    if character.speed_mode == SpeedMode::Fast {
        character.endurance -= endurance_cost(character);
    }
    character.dir = direction;
}
