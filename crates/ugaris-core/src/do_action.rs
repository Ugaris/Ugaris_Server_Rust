use serde::{Deserialize, Serialize};

use crate::{
    direction::Direction,
    entity::{Character, CharacterFlags, CharacterValue, Item, ItemFlags, SpeedMode, POWERSCALE},
    ids::{CharacterId, ItemId},
    legacy::{action, profession},
    map::{MapFlags, MapGrid},
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
            deaths: 0,
            cursor_item: None,
            current_container: None,
            values: Character::empty_values(),
            professions: Character::empty_professions(),
            inventory: Character::empty_inventory(),
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
