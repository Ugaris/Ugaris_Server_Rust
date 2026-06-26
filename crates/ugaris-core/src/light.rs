use crate::{
    entity::{Character, CharacterValue, Item, ItemFlags},
    map::{MapFlags, MapGrid},
};

pub const LIGHT_DISTANCE: usize = 20;

pub fn add_light(map: &mut MapGrid, x: usize, y: usize, strength: i16) {
    if strength == 0 || map.tile(x, y).is_none() {
        return;
    }

    add_tile_light(map, x, y, strength);

    let removing = strength < 0;
    let strength = i32::from(strength).abs().min(100) as usize;
    let distance = integer_sqrt(strength.saturating_sub(1)) + 1;
    let xs = x.saturating_sub(distance);
    let ys = y.saturating_sub(distance);
    let xe = (x + 1 + distance).min(map.width().saturating_sub(1));
    let ye = (y + 1 + distance).min(map.height().saturating_sub(1));

    for ty in ys..ye {
        for tx in xs..xe {
            if (tx, ty) == (x, y) || !map.can_see(x, y, tx, ty, distance) {
                continue;
            }

            let dx = x.abs_diff(tx);
            let dy = y.abs_diff(ty);
            let falloff = strength / (dx * dx + dy * dy + 1);
            if falloff == 0 {
                continue;
            }
            let delta = if removing {
                -(falloff as i16)
            } else {
                falloff as i16
            };
            add_tile_light(map, tx, ty, delta);
        }
    }
}

pub fn add_character_light(map: &mut MapGrid, character: &Character) {
    let x = usize::from(character.x);
    let y = usize::from(character.y);
    if !map.legacy_inner_bounds(x, y) {
        return;
    }
    let light = character_light(character);
    if light > 0 && tile_allows_emitted_light(map, x, y) {
        add_light(map, x, y, light);
    }
}

pub fn remove_character_light(map: &mut MapGrid, character: &Character) {
    let x = usize::from(character.x);
    let y = usize::from(character.y);
    if !map.legacy_inner_bounds(x, y) {
        return;
    }
    let light = character_light(character);
    if light > 0 && tile_allows_emitted_light(map, x, y) {
        add_light(map, x, y, -light);
    }
}

pub fn add_item_light(map: &mut MapGrid, item: &Item) {
    apply_item_light(map, item, 1);
}

pub fn remove_item_light(map: &mut MapGrid, item: &Item) {
    apply_item_light(map, item, -1);
}

pub fn add_effect_light(map: &mut MapGrid, x: usize, y: usize, light: i16) {
    if map.legacy_inner_bounds(x, y) && light > 0 && tile_allows_emitted_light(map, x, y) {
        add_light(map, x, y, light);
    }
}

pub fn remove_effect_light(map: &mut MapGrid, x: usize, y: usize, light: i16) {
    if map.legacy_inner_bounds(x, y) && light > 0 && tile_allows_emitted_light(map, x, y) {
        add_light(map, x, y, -light);
    }
}

pub fn compute_groundlight(map: &mut MapGrid, x: usize, y: usize) {
    let Some(tile) = map.tile(x, y) else {
        return;
    };
    let sprite = tile.ground_sprite & 0xffff;
    if sprite == 14361 || sprite == 14353 || (12163..=12166).contains(&sprite) {
        add_light(map, x, y, 64);
    }
}

pub fn compute_dlight(map: &mut MapGrid, x: usize, y: usize) -> bool {
    let Some(tile) = map.tile(x, y) else {
        return false;
    };
    if !tile.flags.contains(MapFlags::INDOORS) {
        return false;
    }

    let xs = x.saturating_sub(LIGHT_DISTANCE);
    let ys = y.saturating_sub(LIGHT_DISTANCE);
    let xe = (x + 1 + LIGHT_DISTANCE).min(map.width().saturating_sub(1));
    let ye = (y + 1 + LIGHT_DISTANCE).min(map.height().saturating_sub(1));
    let mut best = 0_u16;

    'outer: for ty in ys..ye {
        for tx in xs..xe {
            let dx = x.abs_diff(tx);
            let dy = y.abs_diff(ty);
            if dx * dx + dy * dy > LIGHT_DISTANCE * LIGHT_DISTANCE + 1 {
                continue;
            }
            let Some(candidate) = map.tile(tx, ty) else {
                continue;
            };
            if candidate.flags.contains(MapFlags::INDOORS)
                || !map.can_see(x, y, tx, ty, LIGHT_DISTANCE)
            {
                continue;
            }
            let daylight = (256 / (dx * dx + dy * dy + 1)).min(63) as u16;
            best = best.max(daylight);
            if best > 63 {
                break 'outer;
            }
        }
    }

    let best = best.min(63);
    let Some(tile) = map.tile_mut(x, y) else {
        return false;
    };
    if tile.daylight == best {
        false
    } else {
        tile.daylight = best;
        true
    }
}

pub fn reset_dlight(map: &mut MapGrid, x: usize, y: usize) -> bool {
    let xs = x.saturating_sub(LIGHT_DISTANCE);
    let ys = y.saturating_sub(LIGHT_DISTANCE);
    let xe = (x + 1 + LIGHT_DISTANCE).min(map.width().saturating_sub(1));
    let ye = (y + 1 + LIGHT_DISTANCE).min(map.height().saturating_sub(1));
    let mut have_indoors = false;
    let mut have_outdoors = false;

    'scan: for ty in ys..ye {
        for tx in xs..xe {
            if map
                .tile(tx, ty)
                .is_some_and(|tile| tile.flags.contains(MapFlags::INDOORS))
            {
                have_indoors = true;
            } else {
                have_outdoors = true;
            }
            if have_indoors && have_outdoors {
                break 'scan;
            }
        }
    }

    if !have_indoors || !have_outdoors {
        return false;
    }

    let mut changed = false;
    for ty in ys..ye {
        for tx in xs..xe {
            if map
                .tile(tx, ty)
                .is_some_and(|tile| tile.flags.contains(MapFlags::INDOORS))
            {
                changed |= compute_dlight(map, tx, ty);
            }
        }
    }
    changed
}

fn apply_item_light(map: &mut MapGrid, item: &Item, sign: i16) {
    let x = usize::from(item.x);
    let y = usize::from(item.y);
    if !map.legacy_inner_bounds(x, y) {
        return;
    }
    let light = item_light(item);
    if light <= 0 {
        return;
    }
    let takeable_in_no_light =
        item.flags.contains(ItemFlags::TAKE) && !tile_allows_emitted_light(map, x, y);
    if !takeable_in_no_light {
        add_light(map, x, y, light * sign);
    }
}

fn item_light(item: &Item) -> i16 {
    item.modifier_index
        .iter()
        .zip(item.modifier_value.iter())
        .filter_map(|(&index, &value)| (index == CharacterValue::Light as i16).then_some(value))
        .sum()
}

fn character_light(character: &Character) -> i16 {
    character
        .values
        .first()
        .and_then(|values| values.get(CharacterValue::Light as usize))
        .copied()
        .unwrap_or_default()
}

fn tile_allows_emitted_light(map: &MapGrid, x: usize, y: usize) -> bool {
    map.tile(x, y)
        .is_some_and(|tile| !tile.flags.contains(MapFlags::NOLIGHT))
}

fn add_tile_light(map: &mut MapGrid, x: usize, y: usize, delta: i16) {
    if let Some(tile) = map.tile_mut(x, y) {
        tile.light = (i32::from(tile.light) + i32::from(delta)).max(0) as i16;
    }
}

fn integer_sqrt(value: usize) -> usize {
    (value as f64).sqrt() as usize
}

#[cfg(test)]
mod tests {
    use crate::{
        entity::{CharacterFlags, MAX_MODIFIERS},
        ids::{CharacterId, ItemId},
    };

    use super::*;

    #[test]
    fn add_light_uses_legacy_inverse_square_falloff_and_caps_strength() {
        let mut map = MapGrid::new(30, 30);

        add_light(&mut map, 10, 10, 64);

        assert_eq!(map.tile(10, 10).unwrap().light, 64);
        assert_eq!(map.tile(11, 10).unwrap().light, 32);
        assert_eq!(map.tile(12, 10).unwrap().light, 12);
        assert_eq!(map.tile(18, 10).unwrap().light, 0);

        add_light(&mut map, 10, 10, -64);
        assert_eq!(map.tile(10, 10).unwrap().light, 0);
        assert_eq!(map.tile(11, 10).unwrap().light, 0);
        assert_eq!(map.tile(12, 10).unwrap().light, 0);
    }

    #[test]
    fn light_does_not_pass_sightblockers() {
        let mut map = MapGrid::new(30, 30);
        map.set_flags(11, 10, MapFlags::SIGHTBLOCK);

        add_light(&mut map, 10, 10, 64);

        assert_eq!(map.tile(11, 10).unwrap().light, 32);
        assert_eq!(map.tile(12, 10).unwrap().light, 0);
    }

    #[test]
    fn item_light_sums_light_modifiers_and_respects_takeable_nolight_rule() {
        let mut map = MapGrid::new(20, 20);
        let mut item = item(7);
        item.x = 10;
        item.y = 10;
        item.modifier_index[0] = CharacterValue::Light as i16;
        item.modifier_value[0] = 10;
        item.modifier_index[1] = CharacterValue::Light as i16;
        item.modifier_value[1] = 5;

        add_item_light(&mut map, &item);
        assert_eq!(map.tile(10, 10).unwrap().light, 15);
        remove_item_light(&mut map, &item);
        assert_eq!(map.tile(10, 10).unwrap().light, 0);

        item.flags.insert(ItemFlags::TAKE);
        map.set_flags(10, 10, MapFlags::NOLIGHT);
        add_item_light(&mut map, &item);
        assert_eq!(map.tile(10, 10).unwrap().light, 0);
    }

    #[test]
    fn character_and_effect_light_respect_nolight_tiles() {
        let mut map = MapGrid::new(20, 20);
        let mut character = character(9);
        character.x = 10;
        character.y = 10;
        character.values[0][CharacterValue::Light as usize] = 25;

        map.set_flags(10, 10, MapFlags::NOLIGHT);
        add_character_light(&mut map, &character);
        add_effect_light(&mut map, 10, 10, 50);
        assert_eq!(map.tile(10, 10).unwrap().light, 0);

        map.set_flags(10, 10, MapFlags::empty());
        add_character_light(&mut map, &character);
        remove_character_light(&mut map, &character);
        add_effect_light(&mut map, 10, 10, 50);
        remove_effect_light(&mut map, 10, 10, 50);
        assert_eq!(map.tile(10, 10).unwrap().light, 0);
    }

    #[test]
    fn groundlight_matches_lava_sprite_table() {
        let mut map = MapGrid::new(20, 20);
        map.tile_mut(10, 10).unwrap().ground_sprite = 12164;

        compute_groundlight(&mut map, 10, 10);

        assert_eq!(map.tile(10, 10).unwrap().light, 64);
    }

    #[test]
    fn compute_dlight_sets_best_visible_outdoor_daylight() {
        let mut map = MapGrid::new(40, 40);
        for y in 0..40 {
            for x in 0..40 {
                map.set_flags(x, y, MapFlags::INDOORS);
            }
        }
        map.set_flags(12, 10, MapFlags::empty());
        map.set_flags(16, 10, MapFlags::empty());

        assert!(compute_dlight(&mut map, 10, 10));
        assert_eq!(map.tile(10, 10).unwrap().daylight, 51);
        assert!(!compute_dlight(&mut map, 10, 10));
    }

    #[test]
    fn reset_dlight_only_updates_mixed_indoor_outdoor_area() {
        let mut all_indoor = MapGrid::new(20, 20);
        for y in 0..20 {
            for x in 0..20 {
                all_indoor.set_flags(x, y, MapFlags::INDOORS);
            }
        }
        assert!(!reset_dlight(&mut all_indoor, 10, 10));

        let mut mixed = all_indoor.clone();
        mixed.set_flags(12, 10, MapFlags::empty());
        assert!(reset_dlight(&mut mixed, 10, 10));
        assert_eq!(mixed.tile(10, 10).unwrap().daylight, 51);
    }

    fn character(id: u32) -> Character {
        Character {
            id: CharacterId(id),
            name: String::new(),
            description: String::new(),
            flags: CharacterFlags::USED,
            sprite: 0,
            speed_mode: Default::default(),
            x: 0,
            y: 0,
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
            level: 0,
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

    fn item(id: u32) -> Item {
        Item {
            id: ItemId(id),
            name: String::new(),
            description: String::new(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [0; MAX_MODIFIERS],
            modifier_value: [0; MAX_MODIFIERS],
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
