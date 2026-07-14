use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
};

use serde::{Deserialize, Serialize};

use crate::{
    direction::Direction,
    map::{manhattan_distance, MapFlags, MapGrid},
};

pub const MAX_NODES: usize = 16_384;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PathBlockMode {
    Normal,
    IgnoreCharacters,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PathResult {
    pub direction: Option<Direction>,
    pub cost: i32,
    pub nodes: usize,
    pub best_x: usize,
    pub best_y: usize,
    pub best_direction: Option<Direction>,
    pub best_cost: i32,
    pub best_distance: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Node {
    x: usize,
    y: usize,
    first_dir: Option<Direction>,
    cost: i32,
    total_cost: i32,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .total_cost
            .cmp(&self.total_cost)
            .then_with(|| other.cost.cmp(&self.cost))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn legacy_path_cost(from_x: usize, from_y: usize, to_x: usize, to_y: usize) -> i32 {
    let dx = from_x.abs_diff(to_x) as i32;
    let dy = from_y.abs_diff(to_y) as i32;
    if dx > dy {
        (dx << 1) + dy
    } else {
        (dy << 1) + dx
    }
}

pub fn pathfinder(
    grid: &MapGrid,
    from_x: usize,
    from_y: usize,
    to_x: usize,
    to_y: usize,
    min_dist: usize,
    max_step_hint: Option<usize>,
) -> PathResult {
    pathfinder_with_mode(
        grid,
        from_x,
        from_y,
        to_x,
        to_y,
        min_dist,
        max_step_hint,
        PathBlockMode::Normal,
    )
}

pub fn pathfinder_ignore_characters(
    grid: &MapGrid,
    from_x: usize,
    from_y: usize,
    to_x: usize,
    to_y: usize,
    min_dist: usize,
    max_step_hint: Option<usize>,
) -> PathResult {
    pathfinder_with_mode(
        grid,
        from_x,
        from_y,
        to_x,
        to_y,
        min_dist,
        max_step_hint,
        PathBlockMode::IgnoreCharacters,
    )
}

#[allow(clippy::too_many_arguments)]
fn pathfinder_with_mode(
    grid: &MapGrid,
    from_x: usize,
    from_y: usize,
    to_x: usize,
    to_y: usize,
    min_dist: usize,
    max_step_hint: Option<usize>,
    block_mode: PathBlockMode,
) -> PathResult {
    if !grid.legacy_inner_bounds(from_x, from_y) || !grid.legacy_inner_bounds(to_x, to_y) {
        return PathResult::default();
    }
    if manhattan_distance(from_x, from_y, to_x, to_y) == min_dist {
        return PathResult::default();
    }
    if min_dist == 0 && blocks_movement(grid, to_x, to_y, block_mode) {
        return PathResult::default();
    }

    let distance = manhattan_distance(from_x, from_y, to_x, to_y);
    let mut max_steps = MAX_NODES.min(distance * 20 + 500);
    if let Some(hint) = max_step_hint {
        max_steps = max_steps.min(hint);
    }

    let mut open = BinaryHeap::new();
    let mut best_cost_by_pos: HashMap<(usize, usize), i32> = HashMap::new();
    let mut best = Node {
        x: from_x,
        y: from_y,
        first_dir: None,
        cost: 0,
        total_cost: legacy_path_cost(from_x, from_y, to_x, to_y),
    };
    open.push(best);
    best_cost_by_pos.insert((from_x, from_y), 0);

    let mut nodes = 0;
    while let Some(node) = open.pop() {
        nodes += 1;
        if nodes >= max_steps {
            break;
        }

        let dist = manhattan_distance(node.x, node.y, to_x, to_y);
        let best_dist = manhattan_distance(best.x, best.y, to_x, to_y);
        if dist < best_dist {
            best = node;
        }
        if dist == min_dist {
            return PathResult {
                direction: node.first_dir,
                cost: node.cost,
                nodes,
                best_x: best.x,
                best_y: best.y,
                best_direction: best.first_dir,
                best_cost: best.cost,
                best_distance: best_dist,
            };
        }

        for (dir, step_cost) in successors(grid, node.x, node.y, block_mode) {
            let (dx, dy) = dir.delta();
            let nx = (node.x as i16 + dx) as usize;
            let ny = (node.y as i16 + dy) as usize;
            let next_cost = node.cost + step_cost;
            if matches!(best_cost_by_pos.get(&(nx, ny)), Some(known) if *known <= next_cost) {
                continue;
            }
            best_cost_by_pos.insert((nx, ny), next_cost);
            open.push(Node {
                x: nx,
                y: ny,
                first_dir: node.first_dir.or(Some(dir)),
                cost: next_cost,
                total_cost: next_cost + legacy_path_cost(nx, ny, to_x, to_y),
            });
        }
    }

    PathResult {
        direction: None,
        cost: 0,
        nodes,
        best_x: best.x,
        best_y: best.y,
        best_direction: best.first_dir,
        best_cost: best.cost,
        best_distance: manhattan_distance(best.x, best.y, to_x, to_y),
    }
}

fn successors(
    grid: &MapGrid,
    x: usize,
    y: usize,
    block_mode: PathBlockMode,
) -> Vec<(Direction, i32)> {
    let candidates = [
        (Direction::Right, 2),
        (Direction::Left, 2),
        (Direction::Down, 2),
        (Direction::Up, 2),
        (Direction::RightDown, 3),
        (Direction::RightUp, 3),
        (Direction::LeftDown, 3),
        (Direction::LeftUp, 3),
    ];

    candidates
        .into_iter()
        .filter(|(dir, _)| can_step(grid, x, y, *dir, block_mode))
        .collect()
}

fn can_step(grid: &MapGrid, x: usize, y: usize, dir: Direction, block_mode: PathBlockMode) -> bool {
    let (dx, dy) = dir.delta();
    let nx = x as i16 + dx;
    let ny = y as i16 + dy;
    if nx < 0
        || ny < 0
        || !grid.legacy_inner_bounds(nx as usize, ny as usize)
        || blocks_movement(grid, nx as usize, ny as usize, block_mode)
    {
        return false;
    }

    if dx != 0 && dy != 0 {
        let side_x = (x as i16 + dx) as usize;
        let side_y = (y as i16 + dy) as usize;
        if blocks_movement(grid, side_x, y, block_mode)
            || blocks_movement(grid, x, side_y, block_mode)
        {
            return false;
        }
    }

    true
}

fn blocks_movement(grid: &MapGrid, x: usize, y: usize, block_mode: PathBlockMode) -> bool {
    let Some(tile) = grid.tile(x, y) else {
        return true;
    };
    if tile.flags.contains(MapFlags::MOVEBLOCK) {
        return true;
    }
    // C `normal_check_target` / `ignorechar_check_target`: door tiles are
    // pathable even while closed; `walk_or_use_driver` opens them when the
    // walker bumps into the temporary block.
    if tile.flags.contains(MapFlags::DOOR) {
        return false;
    }
    match block_mode {
        PathBlockMode::Normal => tile.flags.contains(MapFlags::TMOVEBLOCK),
        // C: temporary blocks only count when an item causes them, so
        // character blockers are ignored.
        PathBlockMode::IgnoreCharacters => {
            tile.flags.contains(MapFlags::TMOVEBLOCK) && tile.item != 0
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{legacy::MAX_MAP, map::MapFlags};

    use super::*;

    #[test]
    fn legacy_path_cost_matches_c_heuristic() {
        assert_eq!(legacy_path_cost(1, 1, 5, 2), 9);
        assert_eq!(legacy_path_cost(1, 1, 2, 5), 9);
    }

    #[test]
    fn pathfinder_returns_first_direction() {
        let grid = MapGrid::default();
        let result = pathfinder(&grid, 10, 10, 13, 10, 0, None);
        assert_eq!(result.direction, Some(Direction::Right));
        assert_eq!(result.cost, 6);
    }

    #[test]
    fn pathfinder_routes_around_movement_block() {
        let mut grid = MapGrid::default();
        grid.set_flags(11, 10, MapFlags::MOVEBLOCK);
        let result = pathfinder(&grid, 10, 10, 13, 10, 0, None);
        assert_ne!(result.direction, Some(Direction::Right));
        assert!(result.best_distance <= 3);
    }

    #[test]
    fn ignore_characters_mode_allows_paths_through_occupied_tiles() {
        let mut grid = MapGrid::default();
        for y in 1..MAX_MAP - 1 {
            grid.tile_mut(11, y).unwrap().character = y as u16;
            grid.tile_mut(11, y)
                .unwrap()
                .flags
                .insert(MapFlags::TMOVEBLOCK);
        }

        let blocked = pathfinder(&grid, 10, 10, 12, 10, 0, None);
        let ignoring = pathfinder_ignore_characters(&grid, 10, 10, 12, 10, 0, None);

        assert_eq!(blocked.direction, None);
        assert_eq!(ignoring.direction, Some(Direction::Right));
    }

    #[test]
    fn pathfinder_routes_through_closed_doors_like_c_check_target() {
        let mut grid = MapGrid::default();
        // Wall row with a closed door at (11,10): DOOR + TMOVEBLOCK.
        for y in 5..15 {
            if y != 10 {
                grid.set_flags(11, y, MapFlags::MOVEBLOCK);
            }
        }
        grid.tile_mut(11, 10)
            .unwrap()
            .flags
            .insert(MapFlags::DOOR | MapFlags::TMOVEBLOCK);

        let result = pathfinder(&grid, 10, 10, 12, 10, 0, None);
        assert_eq!(
            result.direction,
            Some(Direction::Right),
            "closed doors are pathable; the walker opens them on bump"
        );

        let ignoring = pathfinder_ignore_characters(&grid, 10, 10, 12, 10, 0, None);
        assert_eq!(ignoring.direction, Some(Direction::Right));
    }

    #[test]
    fn pathfinder_ignore_characters_still_blocks_item_tmoveblocks() {
        let mut grid = MapGrid::default();
        grid.tile_mut(11, 10)
            .unwrap()
            .flags
            .insert(MapFlags::TMOVEBLOCK);
        grid.tile_mut(11, 10).unwrap().item = 900;
        for y in 0..crate::legacy::MAX_MAP {
            if y != 10 {
                grid.set_flags(11, y, MapFlags::MOVEBLOCK);
            }
        }

        let result = pathfinder_ignore_characters(&grid, 10, 10, 12, 10, 0, None);
        assert_eq!(
            result.direction, None,
            "C ignorechar_check_target keeps item-caused temporary blocks"
        );
    }
}
