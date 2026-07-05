use crate::{
    entity::{Character, CharacterFlags, CharacterValue, Item, ItemFlags, SpeedMode},
    legacy::{profession, DIST_MAX},
    map::{chebyshev_distance, MapFlags, MapGrid, MapTile},
};

pub fn check_light(tile: &MapTile, global_daylight: i32) -> i32 {
    let mixed_daylight = (global_daylight * i32::from(tile.daylight)) / 256;
    i32::from(tile.light).max(mixed_daylight).clamp(0, 255)
}

pub fn char_see_char_nolos(
    viewer: &Character,
    target: &Character,
    map: &MapGrid,
    global_daylight: i32,
) -> bool {
    if target.flags.is_empty() {
        return false;
    }
    if viewer.id == target.id {
        return true;
    }
    if target.flags.contains(CharacterFlags::INVISIBLE) {
        return false;
    }
    if viewer
        .flags
        .contains(CharacterFlags::GOD | CharacterFlags::INFRARED)
    {
        return true;
    }

    let target_x = usize::from(target.x);
    let target_y = usize::from(target.y);
    let Some(tile) = map.tile(target_x, target_y) else {
        return false;
    };

    let dist = chebyshev_distance(
        usize::from(viewer.x),
        usize::from(viewer.y),
        target_x,
        target_y,
    ) + 1;
    let mut light = check_light(tile, global_daylight);

    if viewer
        .flags
        .intersects(CharacterFlags::INFRARED | CharacterFlags::INFRAVISION)
    {
        light = light.max(32);
    }
    if profession_value(viewer, profession::DARK) >= 30
        && target.flags.contains(CharacterFlags::ALIVE)
    {
        light = light.max(32);
    }
    if profession_value(viewer, profession::LIGHT) >= 30
        && target.flags.contains(CharacterFlags::UNDEAD)
    {
        light = light.max(32);
    }

    let dx = viewer.x.abs_diff(target.x);
    let dy = viewer.y.abs_diff(target.y);
    if light == 0 && (dx > 1 || dy > 1) {
        return false;
    }

    if dist < 3
        && (target.speed_mode != SpeedMode::Stealth
            || profession_value(target, profession::THIEF) == 0
            || !target.flags.contains(CharacterFlags::THIEFMODE))
    {
        return true;
    }

    let dist = (dist * dist) as i32;
    let light_penalty = (32 - light).max(0) * 2;
    let stealth = if target.speed_mode == SpeedMode::Stealth {
        character_value(target, CharacterValue::Stealth)
    } else {
        0
    };
    let target_score = if stealth != 0 {
        light_penalty + stealth + dist
    } else {
        0
    };
    let viewer_score = character_value(viewer, CharacterValue::Percept) + 16 + 49;

    viewer_score >= target_score
}

pub fn char_see_char(
    viewer: &Character,
    target: &Character,
    map: &MapGrid,
    global_daylight: i32,
) -> bool {
    if target.flags.is_empty() {
        return false;
    }
    if viewer.id == target.id {
        return true;
    }
    if target.flags.contains(CharacterFlags::INVISIBLE) {
        return false;
    }
    if !map.can_see(
        usize::from(viewer.x),
        usize::from(viewer.y),
        usize::from(target.x),
        usize::from(target.y),
        DIST_MAX,
    ) {
        return false;
    }

    char_see_char_nolos(viewer, target, map, global_daylight)
}

pub fn char_see_item(viewer: &Character, item: &Item, map: &MapGrid, global_daylight: i32) -> bool {
    if item.flags.is_empty() || item.carried_by.is_some() {
        return false;
    }

    let item_x = usize::from(item.x);
    let item_y = usize::from(item.y);
    if !map.can_see(
        usize::from(viewer.x),
        usize::from(viewer.y),
        item_x,
        item_y,
        DIST_MAX,
    ) {
        return false;
    }

    if item.flags.contains(ItemFlags::FRONTWALL)
        && item_frontwall_side_blocked(viewer, map, item_x + 1, item_y)
        && item_frontwall_side_blocked(viewer, map, item_x, item_y + 1)
    {
        return false;
    }

    if !item.flags.contains(ItemFlags::TAKE) {
        return true;
    }

    let Some(tile) = map.tile(item_x, item_y) else {
        return false;
    };
    let mut light = check_light(tile, global_daylight);
    if viewer
        .flags
        .intersects(CharacterFlags::INFRARED | CharacterFlags::INFRAVISION)
    {
        light = light.max(32);
    }

    light >= 1
}

fn item_frontwall_side_blocked(viewer: &Character, map: &MapGrid, x: usize, y: usize) -> bool {
    map.tile(x, y).is_none_or(|tile| {
        tile.flags
            .intersects(MapFlags::SIGHTBLOCK | MapFlags::TSIGHTBLOCK)
    }) || !map.can_see(usize::from(viewer.x), usize::from(viewer.y), x, y, DIST_MAX)
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

fn profession_value(character: &Character, profession: usize) -> i16 {
    character
        .professions
        .get(profession)
        .copied()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use crate::ids::{CharacterId, ItemId};

    use super::*;

    fn character(id: u32, x: u16, y: u16, flags: CharacterFlags) -> Character {
        Character {
            merchant: None,
            template_key: String::new(),
            respawn_ticks: 0,
            id: CharacterId(id),
            serial: id,
            name: format!("Character{id}"),
            description: String::new(),
            flags,
            sprite: 0,
            c1: 0,
            c2: 0,
            c3: 0,
            driver: 0,
            group: 0,
            clan: 0,
            clan_rank: 0,
            clan_serial: 0,
            staff_code: String::new(),
            speed_mode: SpeedMode::Normal,
            x,
            y,
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
            military_points: 0,
            military_normal_exp: 0,
            gold: 0,
            karma: 0,
            creation_time: 0,
            saves: 0,
            got_saved: 0,
            deaths: 0,
            regen_ticker: 0,
            last_regen: 0,
            cursor_item: None,
            current_container: None,
            values: Character::empty_values(),
            professions: Character::empty_professions(),
            inventory: Character::empty_inventory(),
            driver_state: None,
            driver_messages: Vec::new(),
            driver_memory: crate::character_driver::DriverMemory::default(),
            class: 0,
            dungeonfighter: None,
        }
    }

    fn item(flags: ItemFlags, x: u16, y: u16) -> Item {
        Item {
            id: ItemId(1),
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
            modifier_index: [0; 5],
            modifier_value: [0; 5],
            x,
            y,
            carried_by: None,
            contained_in: None,
            content_id: 0,
            driver: 0,
            driver_data: Vec::new(),
            serial: 0,
        }
    }

    #[test]
    fn invisible_targets_are_not_seen() {
        let map = lit_map();
        let viewer = character(1, 10, 10, CharacterFlags::USED);
        let target = character(2, 11, 10, CharacterFlags::USED | CharacterFlags::INVISIBLE);

        assert!(!char_see_char(&viewer, &target, &map, 255));
    }

    #[test]
    fn darkness_hides_distant_characters_without_infravision() {
        let map = MapGrid::new(20, 20);
        let viewer = character(1, 10, 10, CharacterFlags::USED);
        let target = character(2, 13, 10, CharacterFlags::USED);

        assert!(!char_see_char(&viewer, &target, &map, 0));
    }

    #[test]
    fn stealth_uses_perception_score() {
        let map = lit_map();
        let viewer = character(1, 10, 10, CharacterFlags::USED);
        let mut target = character(2, 12, 10, CharacterFlags::USED | CharacterFlags::THIEFMODE);
        target.speed_mode = SpeedMode::Stealth;
        target.professions[profession::THIEF] = 1;
        target.values[0][CharacterValue::Stealth as usize] = 200;

        assert!(!char_see_char_nolos(&viewer, &target, &map, 255));
    }

    #[test]
    fn takeable_items_need_light() {
        let map = MapGrid::new(20, 20);
        let viewer = character(1, 10, 10, CharacterFlags::USED);
        let item = item(ItemFlags::USED | ItemFlags::TAKE, 11, 10);

        assert!(!char_see_item(&viewer, &item, &map, 0));
    }

    #[test]
    fn frontwall_item_is_hidden_when_both_sides_are_blocked() {
        let mut map = lit_map();
        map.set_flags(12, 10, MapFlags::SIGHTBLOCK);
        map.set_flags(11, 11, MapFlags::SIGHTBLOCK);
        let viewer = character(1, 10, 10, CharacterFlags::USED);
        let item = item(ItemFlags::USED | ItemFlags::FRONTWALL, 11, 10);

        assert!(!char_see_item(&viewer, &item, &map, 255));
    }

    fn lit_map() -> MapGrid {
        let mut map = MapGrid::new(20, 20);
        for y in 0..20 {
            for x in 0..20 {
                map.tile_mut(x, y).unwrap().daylight = 256;
            }
        }
        map
    }
}
