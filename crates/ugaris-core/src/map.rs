use bitflags::bitflags;
use serde::{Deserialize, Serialize};

use crate::{
    entity::{Character, CharacterFlags, Item, ItemFlags},
    legacy::MAX_MAP,
    path::pathfinder_ignore_characters,
};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct MapFlags: u32 {
        const MOVEBLOCK = 1 << 0;
        const SIGHTBLOCK = 1 << 1;
        const TMOVEBLOCK = 1 << 2;
        const TSIGHTBLOCK = 1 << 3;
        const INDOORS = 1 << 4;
        const RESTAREA = 1 << 5;
        const DOOR = 1 << 6;
        const SOUNDBLOCK = 1 << 7;
        const TSOUNDBLOCK = 1 << 8;
        const SHOUTBLOCK = 1 << 9;
        const CLAN = 1 << 10;
        const ARENA = 1 << 11;
        const PEACE = 1 << 12;
        const NEUTRAL = 1 << 13;
        const FIRETHRU = 1 << 14;
        const SLOWDEATH = 1 << 15;
        const NOLIGHT = 1 << 16;
        const NOMAGIC = 1 << 17;
        const UNDERWATER = 1 << 18;
        const NOREGEN = 1 << 19;
        const SINK_ANKLE = 1 << 20;
        const SINK_KNEE = 1 << 21;
        const SINK_BELLY = 1 << 22;
        const SINK_CHEST = 1 << 23;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapTile {
    pub ground_sprite: u32,
    pub foreground_sprite: u32,
    pub daylight: u16,
    pub light: i16,
    pub character: u16,
    pub item: u32,
    pub effects: [u16; 4],
    pub flags: MapFlags,
}

impl Default for MapTile {
    fn default() -> Self {
        Self {
            ground_sprite: 0,
            foreground_sprite: 0,
            daylight: 0,
            light: 0,
            character: 0,
            item: 0,
            effects: [0; 4],
            flags: MapFlags::empty(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapGrid {
    width: usize,
    height: usize,
    tiles: Vec<MapTile>,
}

impl Default for MapGrid {
    fn default() -> Self {
        Self::new(MAX_MAP, MAX_MAP)
    }
}

impl MapGrid {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            tiles: vec![MapTile::default(); width * height],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn legacy_index(&self, x: usize, y: usize) -> Option<usize> {
        (x < self.width && y < self.height).then_some(x + y * self.width)
    }

    pub fn legacy_inner_bounds(&self, x: usize, y: usize) -> bool {
        x >= 1 && x < self.width.saturating_sub(1) && y >= 1 && y < self.height.saturating_sub(1)
    }

    pub fn tile(&self, x: usize, y: usize) -> Option<&MapTile> {
        self.legacy_index(x, y).and_then(|idx| self.tiles.get(idx))
    }

    pub fn tile_mut(&mut self, x: usize, y: usize) -> Option<&mut MapTile> {
        let idx = self.legacy_index(x, y)?;
        self.tiles.get_mut(idx)
    }

    pub fn set_flags(&mut self, x: usize, y: usize, flags: MapFlags) {
        if let Some(tile) = self.tile_mut(x, y) {
            tile.flags = flags;
        }
    }

    pub fn set_item_map(&mut self, item: &mut Item, x: usize, y: usize) -> bool {
        if x < 1 || x >= self.width || y < 1 || y >= self.height || item.flags.is_empty() {
            return false;
        }

        let Some(tile) = self.tile_mut(x, y) else {
            return false;
        };
        if tile.item != 0
            || tile
                .flags
                .intersects(MapFlags::TMOVEBLOCK | MapFlags::MOVEBLOCK)
        {
            return false;
        }

        tile.item = item.id.0;
        if item.flags.contains(ItemFlags::MOVEBLOCK) {
            tile.flags.insert(MapFlags::TMOVEBLOCK);
        }
        if item.flags.contains(ItemFlags::SIGHTBLOCK) {
            tile.flags.insert(MapFlags::TSIGHTBLOCK);
        }

        item.x = x as u16;
        item.y = y as u16;
        item.carried_by = None;
        item.contained_in = None;
        true
    }

    pub fn remove_item_map(&mut self, item: &mut Item) -> bool {
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        if !self.legacy_inner_bounds(x, y) {
            item.x = 0;
            item.y = 0;
            return true;
        }

        let Some(tile) = self.tile_mut(x, y) else {
            item.x = 0;
            item.y = 0;
            return true;
        };
        if tile.item != item.id.0 {
            item.x = 0;
            item.y = 0;
            return true;
        }

        if item.flags.contains(ItemFlags::MOVEBLOCK) {
            tile.flags.remove(MapFlags::TMOVEBLOCK);
        }
        if item.flags.contains(ItemFlags::SIGHTBLOCK) {
            tile.flags.remove(MapFlags::TSIGHTBLOCK);
        }
        tile.item = 0;
        item.x = 0;
        item.y = 0;
        true
    }

    pub fn set_char(&mut self, character: &mut Character, x: usize, y: usize) -> bool {
        if x < 1 || x >= self.width || y < 1 || y >= self.height {
            return false;
        }

        let Some(tile) = self.tile_mut(x, y) else {
            return false;
        };
        if tile.character != 0
            || tile
                .flags
                .intersects(MapFlags::TMOVEBLOCK | MapFlags::MOVEBLOCK)
        {
            return false;
        }

        tile.character = character.id.0 as u16;
        tile.flags.insert(MapFlags::TMOVEBLOCK);
        character.x = x as u16;
        character.y = y as u16;

        if tile.flags.contains(MapFlags::NOMAGIC) {
            character.flags.insert(CharacterFlags::NOMAGIC);
        } else {
            character.flags.remove(CharacterFlags::NOMAGIC);
        }

        true
    }

    pub fn remove_char(&mut self, character: &mut Character) -> bool {
        let x = usize::from(character.x);
        let y = usize::from(character.y);

        if self.legacy_inner_bounds(x, y) {
            if let Some(tile) = self.tile_mut(x, y) {
                if tile.character == character.id.0 as u16 {
                    tile.flags.remove(MapFlags::TMOVEBLOCK);
                    tile.character = 0;
                }
            }
        }

        character.x = 0;
        character.y = 0;
        true
    }

    pub fn drop_item(&mut self, item: &mut Item, x: usize, y: usize) -> bool {
        for (dx, dy) in DROP_OFFSETS {
            let Some(nx) = offset_coordinate(x, dx) else {
                continue;
            };
            let Some(ny) = offset_coordinate(y, dy) else {
                continue;
            };
            if self.set_item_map(item, nx, ny) {
                return true;
            }
        }

        false
    }

    pub fn drop_char(&mut self, character: &mut Character, x: usize, y: usize) -> bool {
        for (dx, dy) in DROP_OFFSETS {
            let Some(nx) = offset_coordinate(x, dx) else {
                continue;
            };
            let Some(ny) = offset_coordinate(y, dy) else {
                continue;
            };
            if self.set_char(character, nx, ny) {
                return true;
            }
        }

        false
    }

    pub fn drop_char_from_item(&mut self, character: &mut Character, item: &Item) -> bool {
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        let candidates: &[(isize, isize)] = if item.flags.contains(ItemFlags::FRONTWALL) {
            &ITEM_DROP_CHAR_OFFSETS_FRONT_ONLY
        } else {
            &ITEM_DROP_CHAR_OFFSETS_WITH_BEHIND
        };

        for (dx, dy) in candidates {
            let Some(nx) = offset_coordinate(x, *dx) else {
                continue;
            };
            let Some(ny) = offset_coordinate(y, *dy) else {
                continue;
            };
            if self.set_char(character, nx, ny) {
                return true;
            }
        }

        false
    }

    pub fn drop_char_extended(
        &mut self,
        character: &mut Character,
        x: usize,
        y: usize,
        maxdist: usize,
    ) -> bool {
        if self.set_char(character, x, y) {
            return true;
        }

        for dx in 1..maxdist {
            if x + dx < self.width - 1
                && self.path_exists_ignoring_characters(x, y, x + dx, y)
                && self.set_char(character, x + dx, y)
            {
                return true;
            }
            if y + dx < self.height - 1
                && self.path_exists_ignoring_characters(x, y, x, y + dx)
                && self.set_char(character, x, y + dx)
            {
                return true;
            }
            if x > dx + 1
                && self.path_exists_ignoring_characters(x, y, x - dx, y)
                && self.set_char(character, x - dx, y)
            {
                return true;
            }
            if y > dx + 1
                && self.path_exists_ignoring_characters(x, y, x, y - dx)
                && self.set_char(character, x, y - dx)
            {
                return true;
            }

            for dy in 1..maxdist {
                if x + dx < self.width - 1
                    && y + dy < self.height - 1
                    && self.path_exists_ignoring_characters(x, y, x + dx, y + dy)
                    && self.set_char(character, x + dx, y + dy)
                {
                    return true;
                }
                if x > dx + 1
                    && y + dy < self.height - 1
                    && self.path_exists_ignoring_characters(x, y, x - dx, y + dy)
                    && self.set_char(character, x - dx, y + dy)
                {
                    return true;
                }
                if x + dx < self.width - 1
                    && y > dy + 1
                    && self.path_exists_ignoring_characters(x, y, x + dx, y - dy)
                    && self.set_char(character, x + dx, y - dy)
                {
                    return true;
                }
                if x > dx + 1
                    && y > dy + 1
                    && self.path_exists_ignoring_characters(x, y, x - dx, y - dy)
                    && self.set_char(character, x - dx, y - dy)
                {
                    return true;
                }
            }
        }

        false
    }

    pub fn drop_item_extended(
        &mut self,
        item: &mut Item,
        x: usize,
        y: usize,
        maxdist: usize,
    ) -> bool {
        if self.set_item_map(item, x, y) {
            return true;
        }

        for dx in 1..maxdist {
            if x + dx < self.width - 1
                && self.path_exists_ignoring_characters(x, y, x + dx, y)
                && self.set_item_map(item, x + dx, y)
            {
                return true;
            }
            if y + dx < self.height - 1
                && self.path_exists_ignoring_characters(x, y, x, y + dx)
                && self.set_item_map(item, x, y + dx)
            {
                return true;
            }
            if x > dx + 1
                && self.path_exists_ignoring_characters(x, y, x - dx, y)
                && self.set_item_map(item, x - dx, y)
            {
                return true;
            }
            if y > dx + 1
                && self.path_exists_ignoring_characters(x, y, x, y - dx)
                && self.set_item_map(item, x, y - dx)
            {
                return true;
            }

            for dy in 1..=dx {
                if x + dx < self.width - 1
                    && y + dy < self.height - 1
                    && self.path_exists_ignoring_characters(x, y, x + dx, y + dy)
                    && self.set_item_map(item, x + dx, y + dy)
                {
                    return true;
                }
                if x > dx + 1
                    && y + dy < self.height - 1
                    && self.path_exists_ignoring_characters(x, y, x - dx, y + dy)
                    && self.set_item_map(item, x - dx, y + dy)
                {
                    return true;
                }
                if x + dx < self.width - 1
                    && y > dy + 1
                    && self.path_exists_ignoring_characters(x, y, x + dx, y - dy)
                    && self.set_item_map(item, x + dx, y - dy)
                {
                    return true;
                }
                if x > dx + 1
                    && y > dy + 1
                    && self.path_exists_ignoring_characters(x, y, x - dx, y - dy)
                    && self.set_item_map(item, x - dx, y - dy)
                {
                    return true;
                }
                if x + dy < self.width - 1
                    && y + dx < self.height - 1
                    && self.path_exists_ignoring_characters(x, y, x + dy, y + dx)
                    && self.set_item_map(item, x + dy, y + dx)
                {
                    return true;
                }
                if x > dy + 1
                    && y + dx < self.height - 1
                    && self.path_exists_ignoring_characters(x, y, x - dy, y + dx)
                    && self.set_item_map(item, x - dy, y + dx)
                {
                    return true;
                }
                if x + dy < self.width - 1
                    && y > dx + 1
                    && self.path_exists_ignoring_characters(x, y, x + dy, y - dx)
                    && self.set_item_map(item, x + dy, y - dx)
                {
                    return true;
                }
                if x > dy + 1
                    && y > dx + 1
                    && self.path_exists_ignoring_characters(x, y, x - dy, y - dx)
                    && self.set_item_map(item, x - dy, y - dx)
                {
                    return true;
                }
            }
        }

        false
    }

    pub fn blocks_movement(&self, x: usize, y: usize) -> bool {
        match self.tile(x, y) {
            Some(tile) => tile
                .flags
                .intersects(MapFlags::MOVEBLOCK | MapFlags::TMOVEBLOCK),
            None => true,
        }
    }

    pub fn blocks_movement_ignoring_characters(&self, x: usize, y: usize) -> bool {
        match self.tile(x, y) {
            Some(tile) => {
                if tile.flags.contains(MapFlags::MOVEBLOCK) {
                    return true;
                }
                if tile.character != 0 {
                    return false;
                }
                tile.flags.contains(MapFlags::TMOVEBLOCK)
            }
            None => true,
        }
    }

    fn path_exists_ignoring_characters(
        &self,
        from_x: usize,
        from_y: usize,
        to_x: usize,
        to_y: usize,
    ) -> bool {
        pathfinder_ignore_characters(self, from_x, from_y, to_x, to_y, 0, Some(20))
            .direction
            .is_some()
    }

    pub fn blocks_sight(&self, x: usize, y: usize) -> bool {
        match self.tile(x, y) {
            Some(tile) => tile
                .flags
                .intersects(MapFlags::SIGHTBLOCK | MapFlags::TSIGHTBLOCK),
            None => true,
        }
    }

    pub fn can_see(
        &self,
        from_x: usize,
        from_y: usize,
        to_x: usize,
        to_y: usize,
        maxdist: usize,
    ) -> bool {
        if !self.legacy_index(from_x, from_y).is_some() || !self.legacy_index(to_x, to_y).is_some()
        {
            return false;
        }
        if chebyshev_distance(from_x, from_y, to_x, to_y) > maxdist {
            return false;
        }

        for (x, y) in line_points(
            from_x as isize,
            from_y as isize,
            to_x as isize,
            to_y as isize,
        ) {
            let x = x as usize;
            let y = y as usize;
            if (x, y) == (from_x, from_y) || (x, y) == (to_x, to_y) {
                continue;
            }
            if self.blocks_sight(x, y) {
                return false;
            }
        }
        true
    }
}

const DROP_OFFSETS: [(isize, isize); 9] = [
    (0, 0),
    (1, 0),
    (0, 1),
    (-1, 0),
    (0, -1),
    (1, 1),
    (-1, 1),
    (1, -1),
    (-1, -1),
];

const ITEM_DROP_CHAR_OFFSETS_WITH_BEHIND: [(isize, isize); 29] = [
    (0, 0),
    (1, 0),
    (0, 1),
    (1, 1),
    (-1, 0),
    (0, -1),
    (-1, -1),
    (-1, 1),
    (1, -1),
    (2, 0),
    (0, 2),
    (2, 1),
    (1, 2),
    (2, 2),
    (2, 2),
    (-1, 0),
    (0, -1),
    (-1, -1),
    (-2, 0),
    (0, -2),
    (-2, -1),
    (-1, -2),
    (-2, 1),
    (1, -2),
    (-1, 2),
    (2, -1),
    (-2, 2),
    (2, -2),
    (-2, -2),
];

const ITEM_DROP_CHAR_OFFSETS_FRONT_ONLY: [(isize, isize); 9] = [
    (0, 0),
    (1, 0),
    (0, 1),
    (1, 1),
    (2, 0),
    (0, 2),
    (2, 1),
    (1, 2),
    (2, 2),
];

fn offset_coordinate(value: usize, offset: isize) -> Option<usize> {
    if offset.is_negative() {
        value.checked_sub(offset.unsigned_abs())
    } else {
        value.checked_add(offset as usize)
    }
}

pub fn chebyshev_distance(ax: usize, ay: usize, bx: usize, by: usize) -> usize {
    ax.abs_diff(bx).max(ay.abs_diff(by))
}

pub fn manhattan_distance(ax: usize, ay: usize, bx: usize, by: usize) -> usize {
    ax.abs_diff(bx) + ay.abs_diff(by)
}

fn line_points(mut x0: isize, mut y0: isize, x1: isize, y1: isize) -> Vec<(isize, isize)> {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut points = Vec::new();

    loop {
        points.push((x0, y0));
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }

    points
}

#[cfg(test)]
mod tests {
    use crate::{
        entity::{SpeedMode, MAX_MODIFIERS},
        ids::{CharacterId, ItemId},
    };

    use super::*;

    #[test]
    fn map_flags_match_c_header_positions() {
        assert_eq!(MapFlags::MOVEBLOCK.bits(), 1 << 0);
        assert_eq!(MapFlags::UNDERWATER.bits(), 1 << 18);
        assert_eq!(MapFlags::SINK_CHEST.bits(), 1 << 23);
    }

    #[test]
    fn legacy_index_matches_c_formula() {
        let grid = MapGrid::default();
        assert_eq!(grid.legacy_index(0, 0), Some(0));
        assert_eq!(grid.legacy_index(5, 7), Some(5 + 7 * MAX_MAP));
        assert_eq!(grid.legacy_index(MAX_MAP, 0), None);
    }

    #[test]
    fn legacy_inner_bounds_match_player_handlers() {
        let grid = MapGrid::default();
        assert!(!grid.legacy_inner_bounds(0, 1));
        assert!(grid.legacy_inner_bounds(1, 1));
        assert!(!grid.legacy_inner_bounds(MAX_MAP - 1, 1));
    }

    #[test]
    fn line_of_sight_respects_sightblock() {
        let mut grid = MapGrid::default();
        assert!(grid.can_see(10, 10, 15, 10, 40));
        grid.set_flags(12, 10, MapFlags::SIGHTBLOCK);
        assert!(!grid.can_see(10, 10, 15, 10, 40));
    }

    #[test]
    fn set_and_remove_item_map_updates_tile_and_temporary_flags() {
        let mut grid = MapGrid::new(20, 20);
        let mut item = item(
            7,
            ItemFlags::USED | ItemFlags::MOVEBLOCK | ItemFlags::SIGHTBLOCK,
        );

        assert!(grid.set_item_map(&mut item, 10, 10));
        let tile = grid.tile(10, 10).unwrap();
        assert_eq!(tile.item, 7);
        assert!(tile.flags.contains(MapFlags::TMOVEBLOCK));
        assert!(tile.flags.contains(MapFlags::TSIGHTBLOCK));
        assert_eq!((item.x, item.y), (10, 10));

        assert!(grid.remove_item_map(&mut item));
        let tile = grid.tile(10, 10).unwrap();
        assert_eq!(tile.item, 0);
        assert!(!tile.flags.contains(MapFlags::TMOVEBLOCK));
        assert!(!tile.flags.contains(MapFlags::TSIGHTBLOCK));
        assert_eq!((item.x, item.y), (0, 0));
    }

    #[test]
    fn drop_item_uses_legacy_neighbor_order() {
        let mut grid = MapGrid::new(20, 20);
        let mut item = item(7, ItemFlags::USED);
        grid.tile_mut(10, 10).unwrap().item = 1;
        grid.tile_mut(11, 10).unwrap().item = 2;

        assert!(grid.drop_item(&mut item, 10, 10));
        assert_eq!((item.x, item.y), (10, 11));
    }

    #[test]
    fn set_and_remove_char_updates_tile_and_nomagic_flag() {
        let mut grid = MapGrid::new(20, 20);
        let mut character = character(3);
        grid.set_flags(10, 10, MapFlags::NOMAGIC);

        assert!(grid.set_char(&mut character, 10, 10));
        let tile = grid.tile(10, 10).unwrap();
        assert_eq!(tile.character, 3);
        assert!(tile.flags.contains(MapFlags::TMOVEBLOCK));
        assert!(character.flags.contains(CharacterFlags::NOMAGIC));

        assert!(grid.remove_char(&mut character));
        let tile = grid.tile(10, 10).unwrap();
        assert_eq!(tile.character, 0);
        assert!(!tile.flags.contains(MapFlags::TMOVEBLOCK));
        assert_eq!((character.x, character.y), (0, 0));
    }

    #[test]
    fn drop_char_uses_legacy_neighbor_order() {
        let mut grid = MapGrid::new(20, 20);
        let mut character = character(3);
        grid.tile_mut(10, 10).unwrap().character = 1;
        grid.tile_mut(10, 10)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK);
        grid.tile_mut(11, 10).unwrap().character = 2;
        grid.tile_mut(11, 10)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK);

        assert!(grid.drop_char(&mut character, 10, 10));
        assert_eq!((character.x, character.y), (10, 11));
    }

    #[test]
    fn drop_char_from_item_matches_legacy_front_then_behind_order() {
        let mut grid = MapGrid::new(20, 20);
        let mut character = character(3);
        for (x, y) in [(10, 10), (11, 10), (10, 11), (11, 11)] {
            grid.set_flags(x, y, MapFlags::MOVEBLOCK);
        }
        let mut item = item(7, ItemFlags::USED);
        item.x = 10;
        item.y = 10;

        assert!(grid.drop_char_from_item(&mut character, &item));
        assert_eq!((character.x, character.y), (9, 10));
    }

    #[test]
    fn drop_char_from_frontwall_item_skips_behind_tiles() {
        let mut grid = MapGrid::new(20, 20);
        let mut character = character(3);
        for (x, y) in [(10, 10), (11, 10), (10, 11), (11, 11)] {
            grid.set_flags(x, y, MapFlags::MOVEBLOCK);
        }
        let mut item = item(7, ItemFlags::USED | ItemFlags::FRONTWALL);
        item.x = 10;
        item.y = 10;

        assert!(grid.drop_char_from_item(&mut character, &item));
        assert_eq!((character.x, character.y), (12, 10));
    }

    #[test]
    fn ignore_character_blocker_matches_c_path_ignore_char() {
        let mut grid = MapGrid::new(20, 20);
        grid.tile_mut(10, 10)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK);
        assert!(grid.blocks_movement_ignoring_characters(10, 10));

        grid.tile_mut(10, 10).unwrap().character = 5;
        assert!(!grid.blocks_movement_ignoring_characters(10, 10));

        grid.tile_mut(10, 10)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);
        assert!(grid.blocks_movement_ignoring_characters(10, 10));
    }

    #[test]
    fn extended_drop_item_uses_legacy_scan_order() {
        let mut grid = MapGrid::new(20, 20);
        let mut item = item(7, ItemFlags::USED);
        for (x, y) in [(10, 10), (11, 10), (10, 11), (9, 10), (10, 9)] {
            grid.tile_mut(x, y).unwrap().item = 99;
        }

        assert!(grid.drop_item_extended(&mut item, 10, 10, 3));
        assert_eq!((item.x, item.y), (11, 11));
    }

    #[test]
    fn extended_drop_char_can_route_through_occupied_char_tiles() {
        let mut grid = MapGrid::new(20, 20);
        let mut character = character(9);
        for (idx, (x, y)) in [
            (10, 10),
            (11, 10),
            (10, 11),
            (9, 10),
            (10, 9),
            (11, 11),
            (9, 11),
            (11, 9),
            (9, 9),
        ]
        .into_iter()
        .enumerate()
        {
            let tile = grid.tile_mut(x, y).unwrap();
            tile.character = idx as u16 + 1;
            tile.flags.insert(MapFlags::TMOVEBLOCK);
        }

        assert!(grid.drop_char_extended(&mut character, 10, 10, 4));
        assert_eq!((character.x, character.y), (11, 12));
    }

    fn character(id: u32) -> Character {
        Character {
            merchant: None,
            template_key: String::new(),
            respawn_ticks: 0,
            id: CharacterId(id),
            serial: id,
            name: "Character".into(),
            description: String::new(),
            flags: CharacterFlags::USED,
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
            level: 1,
            exp: 0,
            exp_used: 0,
            military_points: 0,
            military_normal_exp: 0,
            gold: 0,
            karma: 0,
            creation_time: 0,
            saves: 0,
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
        }
    }

    fn item(id: u32, flags: ItemFlags) -> Item {
        Item {
            id: ItemId(id),
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
            modifier_index: [0; MAX_MODIFIERS],
            modifier_value: [0; MAX_MODIFIERS],
            x: 0,
            y: 0,
            carried_by: Some(CharacterId(1)),
            contained_in: Some(ItemId(2)),
            content_id: 0,
            driver: 0,
            driver_data: Vec::new(),
            serial: 0,
        }
    }
}
