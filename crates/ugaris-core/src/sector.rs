use serde::{Deserialize, Serialize};

use crate::{
    legacy::MAX_MAP,
    map::{MapFlags, MapGrid},
};

const SECTOR_LEVELS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirtySectors {
    width: usize,
    levels: Vec<Vec<u64>>,
}

impl Default for DirtySectors {
    fn default() -> Self {
        Self::new(MAX_MAP)
    }
}

impl DirtySectors {
    pub fn new(width: usize) -> Self {
        let levels = (0..SECTOR_LEVELS)
            .map(|level| {
                let stride = width >> level;
                vec![0; stride * stride]
            })
            .collect();

        Self { width, levels }
    }

    pub fn set_sector(&mut self, x: isize, y: isize, ticker: u64) {
        if !self.in_bounds(x, y) {
            return;
        }

        let x = x as usize;
        let y = y as usize;
        for level in 0..SECTOR_LEVELS {
            let stride = self.width >> level;
            let idx = (x >> level) + (y >> level) * stride;
            self.levels[level][idx] = ticker;
        }
    }

    pub fn skip_x_sector(&self, x: isize, y: isize, ticker: u64) -> usize {
        if !self.in_bounds(x, y) {
            return 0;
        }

        let x = x as usize;
        let y = y as usize;
        for level in (0..SECTOR_LEVELS).rev() {
            let stride = self.width >> level;
            let idx = (x >> level) + (y >> level) * stride;
            if self.levels[level][idx] < ticker {
                let size = 1 << level;
                return size - (x & (size - 1));
            }
        }

        0
    }

    fn in_bounds(&self, x: isize, y: isize) -> bool {
        x >= 0 && x < self.width as isize && y >= 0 && y < self.width as isize
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterSectorLink {
    pub previous: Option<usize>,
    pub next: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterSectors {
    width: usize,
    heads: Vec<Option<usize>>,
    links: Vec<CharacterSectorLink>,
}

impl CharacterSectors {
    pub fn new(width: usize, max_characters: usize) -> Self {
        let sector_width = width >> 3;
        Self {
            width,
            heads: vec![None; sector_width * sector_width],
            links: vec![CharacterSectorLink::default(); max_characters],
        }
    }

    pub fn add_char_sector(&mut self, character_id: usize, x: usize, y: usize) {
        let Some(idx) = self.index(x, y) else {
            return;
        };
        if character_id >= self.links.len() {
            return;
        }

        let next = self.heads[idx];
        self.links[character_id] = CharacterSectorLink {
            previous: None,
            next,
        };
        if let Some(next) = next {
            self.links[next].previous = Some(character_id);
        }
        self.heads[idx] = Some(character_id);
    }

    pub fn del_char_sector(&mut self, character_id: usize, x: usize, y: usize) {
        if character_id >= self.links.len() {
            return;
        }

        let previous = self.links[character_id].previous;
        let next = self.links[character_id].next;

        if let Some(previous) = previous {
            self.links[previous].next = next;
        } else if let Some(idx) = self.index(x, y) {
            self.heads[idx] = next;
        }

        if let Some(next) = next {
            self.links[next].previous = previous;
        }

        self.links[character_id] = CharacterSectorLink::default();
    }

    pub fn getfirst_char_sector(&self, x: isize, y: isize) -> Option<usize> {
        if x < 0 || y < 0 {
            return None;
        }
        self.index(x as usize, y as usize)
            .and_then(|idx| self.heads[idx])
    }

    pub fn link(&self, character_id: usize) -> Option<CharacterSectorLink> {
        self.links.get(character_id).copied()
    }

    fn index(&self, x: usize, y: usize) -> Option<usize> {
        if x >= self.width || y >= self.width {
            return None;
        }

        let sector_width = self.width >> 3;
        Some((x >> 3) + (y >> 3) * sector_width)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct DoorPair {
    x: usize,
    y: usize,
    nr1: usize,
    nr2: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct DoorLink {
    x: usize,
    y: usize,
    to: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundSectors {
    width: usize,
    height: usize,
    sound_sector: Vec<usize>,
    shout_sector: Vec<usize>,
    doors: Vec<Vec<DoorLink>>,
}

impl SoundSectors {
    pub fn build(map: &MapGrid) -> Self {
        let width = map.width();
        let height = map.height();
        let mut sound_sector = vec![0; width * height];
        let mut shout_sector = vec![0; width * height];
        let mut door_pairs = Vec::new();

        let mut nr = 1;
        for y in 0..height {
            for x in 0..width {
                if shout_sector[x + y * width] == 0
                    && fill_shout_sector(map, &mut shout_sector, x, y, nr)
                {
                    nr += 1;
                }
            }
        }

        nr = 1;
        for y in 0..height {
            for x in 0..width {
                if sound_sector[x + y * width] == 0
                    && fill_sound_sector(map, &mut sound_sector, &mut door_pairs, x, y, nr)
                {
                    nr += 1;
                }
            }
        }

        let mut doors = vec![Vec::new(); nr];
        for pair in door_pairs {
            if pair.nr1 < doors.len() {
                doors[pair.nr1].push(DoorLink {
                    x: pair.x,
                    y: pair.y,
                    to: pair.nr2,
                });
            }
            if pair.nr2 < doors.len() {
                doors[pair.nr2].push(DoorLink {
                    x: pair.x,
                    y: pair.y,
                    to: pair.nr1,
                });
            }
        }

        Self {
            width,
            height,
            sound_sector,
            shout_sector,
            doors,
        }
    }

    pub fn sector_hear(
        &self,
        map: &MapGrid,
        from_x: usize,
        from_y: usize,
        to_x: usize,
        to_y: usize,
    ) -> bool {
        let Some(s1) = self.sound_sector_at(from_x, from_y) else {
            return false;
        };
        let Some(s2) = self.sound_sector_at(to_x, to_y) else {
            return false;
        };

        let mut stack = [0usize; 10];
        self.sector_follow_door(map, s1, s2, &mut stack, 0)
    }

    pub fn sector_hear_shout(
        &self,
        from_x: usize,
        from_y: usize,
        to_x: usize,
        to_y: usize,
    ) -> bool {
        match (
            self.shout_sector_at(from_x, from_y),
            self.shout_sector_at(to_x, to_y),
        ) {
            (Some(s1), Some(s2)) => s1 == s2,
            _ => false,
        }
    }

    fn sector_follow_door(
        &self,
        map: &MapGrid,
        s1: usize,
        s2: usize,
        stack: &mut [usize; 10],
        depth: usize,
    ) -> bool {
        if s1 == s2 {
            return true;
        }
        if depth == stack.len() || stack[..depth].contains(&s1) {
            return false;
        }

        stack[depth] = s1;
        for door in self.doors.get(s1).into_iter().flatten() {
            if map
                .tile(door.x, door.y)
                .is_some_and(|tile| tile.flags.contains(MapFlags::TSOUNDBLOCK))
            {
                continue;
            }
            if self.sector_follow_door(map, door.to, s2, stack, depth + 1) {
                return true;
            }
        }

        false
    }

    fn sound_sector_at(&self, x: usize, y: usize) -> Option<usize> {
        self.index(x, y).map(|idx| self.sound_sector[idx])
    }

    fn shout_sector_at(&self, x: usize, y: usize) -> Option<usize> {
        self.index(x, y).map(|idx| self.shout_sector[idx])
    }

    fn index(&self, x: usize, y: usize) -> Option<usize> {
        (x < self.width && y < self.height).then_some(x + y * self.width)
    }
}

fn fill_shout_sector(
    map: &MapGrid,
    shout_sector: &mut [usize],
    x: usize,
    y: usize,
    nr: usize,
) -> bool {
    if map
        .tile(x, y)
        .is_none_or(|tile| tile.flags.contains(MapFlags::SHOUTBLOCK))
    {
        return false;
    }

    let width = map.width();
    let mut stack = Vec::new();
    add_shout_pos(map, shout_sector, &mut stack, x, y, nr);

    while let Some((x, y)) = stack.pop() {
        add_shout_neighbor(
            map,
            shout_sector,
            &mut stack,
            x as isize + 1,
            y as isize,
            nr,
        );
        add_shout_neighbor(
            map,
            shout_sector,
            &mut stack,
            x as isize - 1,
            y as isize,
            nr,
        );
        add_shout_neighbor(
            map,
            shout_sector,
            &mut stack,
            x as isize,
            y as isize + 1,
            nr,
        );
        add_shout_neighbor(
            map,
            shout_sector,
            &mut stack,
            x as isize,
            y as isize - 1,
            nr,
        );
    }

    shout_sector[x + y * width] == nr
}

fn add_shout_neighbor(
    map: &MapGrid,
    shout_sector: &mut [usize],
    stack: &mut Vec<(usize, usize)>,
    x: isize,
    y: isize,
    nr: usize,
) {
    if x >= 0 && y >= 0 {
        add_shout_pos(map, shout_sector, stack, x as usize, y as usize, nr);
    }
}

fn add_shout_pos(
    map: &MapGrid,
    shout_sector: &mut [usize],
    stack: &mut Vec<(usize, usize)>,
    x: usize,
    y: usize,
    nr: usize,
) {
    let Some(tile) = map.tile(x, y) else {
        return;
    };
    if tile.flags.contains(MapFlags::SHOUTBLOCK) {
        return;
    }
    let idx = x + y * map.width();
    if shout_sector[idx] != 0 {
        return;
    }

    shout_sector[idx] = nr;
    stack.push((x, y));
}

fn fill_sound_sector(
    map: &MapGrid,
    sound_sector: &mut [usize],
    door_pairs: &mut Vec<DoorPair>,
    x: usize,
    y: usize,
    nr: usize,
) -> bool {
    let Some(tile) = map.tile(x, y) else {
        return false;
    };
    if tile.flags.contains(MapFlags::SOUNDBLOCK) {
        return false;
    }
    if tile.flags.contains(MapFlags::TSOUNDBLOCK) {
        sound_sector[x + y * map.width()] = nr;
        add_door(door_pairs, x, y, nr);
        return false;
    }

    let width = map.width();
    let mut stack = Vec::new();
    add_sound_pos(map, sound_sector, door_pairs, &mut stack, x, y, nr);

    while let Some((x, y)) = stack.pop() {
        add_sound_neighbor(
            map,
            sound_sector,
            door_pairs,
            &mut stack,
            x as isize + 1,
            y as isize,
            nr,
        );
        add_sound_neighbor(
            map,
            sound_sector,
            door_pairs,
            &mut stack,
            x as isize - 1,
            y as isize,
            nr,
        );
        add_sound_neighbor(
            map,
            sound_sector,
            door_pairs,
            &mut stack,
            x as isize,
            y as isize + 1,
            nr,
        );
        add_sound_neighbor(
            map,
            sound_sector,
            door_pairs,
            &mut stack,
            x as isize,
            y as isize - 1,
            nr,
        );
    }

    sound_sector[x + y * width] == nr
}

fn add_sound_neighbor(
    map: &MapGrid,
    sound_sector: &mut [usize],
    door_pairs: &mut Vec<DoorPair>,
    stack: &mut Vec<(usize, usize)>,
    x: isize,
    y: isize,
    nr: usize,
) {
    if x >= 0 && y >= 0 {
        add_sound_pos(
            map,
            sound_sector,
            door_pairs,
            stack,
            x as usize,
            y as usize,
            nr,
        );
    }
}

fn add_sound_pos(
    map: &MapGrid,
    sound_sector: &mut [usize],
    door_pairs: &mut Vec<DoorPair>,
    stack: &mut Vec<(usize, usize)>,
    x: usize,
    y: usize,
    nr: usize,
) {
    let Some(tile) = map.tile(x, y) else {
        return;
    };
    if tile.flags.contains(MapFlags::SOUNDBLOCK) {
        return;
    }

    let idx = x + y * map.width();
    if tile.flags.contains(MapFlags::TSOUNDBLOCK) {
        sound_sector[idx] = nr;
        add_door(door_pairs, x, y, nr);
        return;
    }
    if sound_sector[idx] != 0 {
        return;
    }

    sound_sector[idx] = nr;
    stack.push((x, y));
}

fn add_door(door_pairs: &mut Vec<DoorPair>, x: usize, y: usize, nr: usize) {
    if let Some(door) = door_pairs
        .iter_mut()
        .find(|door| door.x == x && door.y == y)
    {
        if door.nr1 == nr || door.nr2 == nr {
            return;
        }
        if door.nr2 == 0 {
            door.nr2 = nr;
        }
        return;
    }

    door_pairs.push(DoorPair {
        x,
        y,
        nr1: nr,
        nr2: 0,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirty_sector_skip_matches_legacy_level_scan() {
        let mut sectors = DirtySectors::default();

        assert_eq!(sectors.skip_x_sector(0, 0, 1), 128);
        sectors.set_sector(0, 0, 1);
        assert_eq!(sectors.skip_x_sector(0, 0, 1), 0);
        assert_eq!(sectors.skip_x_sector(1, 0, 2), 127);
        assert_eq!(sectors.skip_x_sector(-1, 0, 2), 0);
    }

    #[test]
    fn character_sector_links_match_legacy_head_insertion() {
        let mut sectors = CharacterSectors::new(MAX_MAP, 10);

        sectors.add_char_sector(1, 12, 12);
        sectors.add_char_sector(2, 13, 13);

        assert_eq!(sectors.getfirst_char_sector(12, 12), Some(2));
        assert_eq!(sectors.link(2).unwrap().next, Some(1));
        assert_eq!(sectors.link(1).unwrap().previous, Some(2));

        sectors.del_char_sector(2, 13, 13);
        assert_eq!(sectors.getfirst_char_sector(12, 12), Some(1));
        assert_eq!(sectors.link(1).unwrap().previous, None);
    }

    #[test]
    fn sound_sector_respects_soundblocks_and_open_doors() {
        let mut map = MapGrid::new(5, 3);
        map.set_flags(2, 0, MapFlags::SOUNDBLOCK);
        map.set_flags(2, 1, MapFlags::TSOUNDBLOCK);
        map.set_flags(2, 2, MapFlags::SOUNDBLOCK);

        let sectors = SoundSectors::build(&map);
        assert!(!sectors.sector_hear(&map, 1, 1, 3, 1));

        map.set_flags(2, 1, MapFlags::empty());
        assert!(sectors.sector_hear(&map, 1, 1, 3, 1));
    }

    #[test]
    fn shout_sector_only_uses_shoutblocks() {
        let mut map = MapGrid::new(5, 3);
        map.set_flags(2, 0, MapFlags::SHOUTBLOCK);
        map.set_flags(2, 1, MapFlags::SHOUTBLOCK);
        map.set_flags(2, 2, MapFlags::SHOUTBLOCK);

        let sectors = SoundSectors::build(&map);
        assert!(!sectors.sector_hear_shout(1, 1, 3, 1));
    }
}
