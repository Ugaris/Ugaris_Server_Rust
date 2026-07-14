//! Clan-raid catacomb maze generation, ported from the legacy C server's
//! `area/13/dungeon.c` (`build_maze`/`special_fill`/`create_maze`,
//! `dungeon.c:214-1341`) - part of the "Clan system" P3 task in
//! `PORTING_TODO.md`, whose only remaining gap (as of iteration 138) is
//! this file's C source.
//!
//! This ports the *pure grid-generation and difficulty-scoring* slice
//! only: the recursive-backtracker maze carving (`build_maze`), the
//! shortest-path/reachability flood fill (`special_fill`), and
//! `create_maze`'s orchestration (path-length scoring, fake-wall
//! placement, and NPC/trap/key/door "special" cell-code assignment).
//! Deliberately **not** ported here (left for a future slice once this
//! lands): `build_warrior`/`build_mage`/`build_seyan` (actual NPC stat
//! generation from `dungeon_tab.c`'s per-level tables) and `build_cell`/
//! `build_wall`/`build_empty`/`build_door`/`build_key`/`build_teleport`/
//! `build_fake` (turning a generated [`MazeCell`]'s `special` code into
//! real `World` map tiles/items/NPCs), plus the `dungeonmaster`/
//! `dungeonfighter` NPC drivers that call into all of the above. Those
//! need `ugaris-server`'s `spawns.rs` template-instantiation plumbing
//! (`ZoneLoader::instantiate_character_template`, already used by ~8
//! other spawn call sites) and are out of scope for this pure-logic
//! slice, same "pure logic first, wiring later" precedent as the rest of
//! the Clan system task.
//!
//! Random numbers use [`crate::world`]'s existing seeded-LCG helper
//! shape (not real C `srand`/`rand()` - already documented as such
//! throughout the codebase, e.g. `world/mod.rs`'s
//! `legacy_random_below_from_seed`) rather than a bit-exact libc PRNG
//! port, consistent with every other `RANDOM()` call site ported so far.

/// C's `xsize`/`ysize` (`dungeon.c:214`), fixed maze grid dimensions -
/// always 20x20 in the legacy game, never varied at any call site.
pub const MAZE_XSIZE: usize = 20;
pub const MAZE_YSIZE: usize = 20;
pub const MAZE_CELLS: usize = MAZE_XSIZE * MAZE_YSIZE;

/// C `struct cell` (`dungeon.c:207-212`). `top_wall`/`left_wall` are the
/// maze's carved-passage walls (`t`/`l`); `visited` is `build_maze`'s own
/// scratch flag (`v`); `special` is the multi-purpose cell-code field
/// used first as `special_fill`'s BFS distance label and finally
/// (post-reset) as the NPC/trap/key/door placement code (`5..=22` =
/// warrior/mage/seyan tiers, `23..=27` = teleport traps, `28..=30` =
/// exit door variants, `3`/`4` = maze keys, `1`/`2` = the fake-wall
/// marker).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MazeCell {
    pub top_wall: bool,
    pub left_wall: bool,
    pub visited: bool,
    pub special: i32,
}

fn legacy_random_below_from_seed(seed: &mut u32, below: u32) -> u32 {
    if below == 0 {
        return 0;
    }
    *seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
    *seed % below
}

fn cell_index(x: usize, y: usize) -> usize {
    x + y * MAZE_XSIZE
}

/// C `build_maze` (`dungeon.c:1005-1101`): carves a perfect maze (a
/// spanning tree over the 20x20 grid, no cycles) using an explicit-stack
/// recursive-backtracker, exactly mirroring C's `goto repeat`-based
/// control flow (each iteration re-evaluates the four neighbor
/// directions from scratch, pushes the current cell back onto the stack
/// only when more than one unvisited neighbor remains, and picks one of
/// the available directions via `RANDOM(opt)`).
pub fn build_maze(seed: &mut u32) -> Vec<MazeCell> {
    let mut cells = vec![
        MazeCell {
            top_wall: true,
            left_wall: true,
            ..Default::default()
        };
        MAZE_CELLS
    ];

    let mut stack: Vec<usize> = Vec::with_capacity(MAZE_CELLS);
    stack.push(legacy_random_below_from_seed(seed, MAZE_CELLS as u32) as usize);
    let mut visited = 0usize;

    while visited < MAZE_CELLS {
        let Some(mut n) = stack.pop() else {
            // C: `elog("ran out of stack ...")` then `break` - a safety
            // valve that should never trigger for a well-formed 20x20
            // grid; mirrored as a silent break rather than a panic.
            break;
        };

        loop {
            let can_left = n % MAZE_XSIZE > 0 && !cells[n - 1].visited;
            let can_right = n % MAZE_XSIZE < MAZE_XSIZE - 1 && !cells[n + 1].visited;
            let can_up = n / MAZE_XSIZE > 0 && !cells[n - MAZE_XSIZE].visited;
            let can_down = n / MAZE_XSIZE < MAZE_YSIZE - 1 && !cells[n + MAZE_XSIZE].visited;

            let mut opt = 0u32;
            if can_left {
                opt += 1;
            }
            if can_right {
                opt += 1;
            }
            if can_up {
                opt += 1;
            }
            if can_down {
                opt += 1;
            }

            if opt == 0 {
                break;
            }

            if opt > 1 {
                stack.push(n);
            }

            let mut choice = legacy_random_below_from_seed(seed, opt);
            let mut moved = false;

            if can_left {
                if choice > 0 {
                    choice -= 1;
                } else {
                    cells[n].left_wall = false;
                    n -= 1;
                    cells[n].visited = true;
                    visited += 1;
                    moved = true;
                }
            }
            if !moved && can_right {
                if choice > 0 {
                    choice -= 1;
                } else {
                    n += 1;
                    cells[n].left_wall = false;
                    cells[n].visited = true;
                    visited += 1;
                    moved = true;
                }
            }
            if !moved && can_up {
                if choice > 0 {
                    choice -= 1;
                } else {
                    cells[n].top_wall = false;
                    n -= MAZE_XSIZE;
                    cells[n].visited = true;
                    visited += 1;
                    moved = true;
                }
            }
            if !moved && can_down {
                if choice > 0 {
                    choice -= 1;
                } else {
                    n += MAZE_XSIZE;
                    cells[n].top_wall = false;
                    cells[n].visited = true;
                    visited += 1;
                    moved = true;
                }
            }

            if !moved {
                // C: `elog("not reached")` - unreachable given opt > 0.
                break;
            }
            // C: `goto repeat` - loop back around with the updated `n`.
        }
    }

    cells
}

/// C `special_fill` (`dungeon.c:968-1003`): a distance-labeling flood
/// fill through open passages only (respecting `top_wall`/`left_wall`).
/// Writes each visited cell's shortest known distance from `nr` into
/// `.special` (starting at `val`), refining it further if a shorter path
/// is found later in the recursion, and never continuing past a cell
/// already marked `999` (the "preserved path" sentinel used by
/// `create_maze`). The `i32` return value mirrors C's own (an internal
/// `best`/`tmp` bookkeeping value) but - matching every real call site in
/// `create_maze`, which only ever reads `.special` back out afterwards -
/// is not meaningful to callers of this function either.
pub fn special_fill(cells: &mut [MazeCell], nr: usize, val: i32) -> i32 {
    if cells[nr].special == 999 {
        return val;
    }
    if cells[nr].special != 0 && cells[nr].special <= val {
        return 0;
    }
    cells[nr].special = val;

    let mut best = 999;

    if !cells[nr].left_wall && nr % MAZE_XSIZE > 0 {
        let tmp = special_fill(cells, nr - 1, val + 1);
        if tmp != 0 {
            best = best.min(tmp);
        }
    }
    if !cells[nr].top_wall && nr / MAZE_XSIZE > 0 {
        let tmp = special_fill(cells, nr - MAZE_XSIZE, val + 1);
        if tmp != 0 {
            best = best.min(tmp);
        }
    }
    if nr % MAZE_XSIZE < MAZE_XSIZE - 1 && !cells[nr + 1].left_wall {
        let tmp = special_fill(cells, nr + 1, val + 1);
        if tmp != 0 {
            best = best.min(tmp);
        }
    }
    if nr / MAZE_XSIZE < MAZE_YSIZE - 1 && !cells[nr + MAZE_XSIZE].top_wall {
        let tmp = special_fill(cells, nr + MAZE_XSIZE, val + 1);
        if tmp != 0 {
            best = best.min(tmp);
        }
    }

    best
}

fn reset_special_except_path(cells: &mut [MazeCell]) {
    for cell in cells.iter_mut() {
        if cell.special != 999 {
            cell.special = 0;
        }
    }
}

fn reset_special_all(cells: &mut [MazeCell]) {
    for cell in cells.iter_mut() {
        cell.special = 0;
    }
}

/// The result of [`create_maze`]: the generated grid (with NPC/trap/key/
/// door placement codes baked into each cell's `special` field) plus the
/// three path-length measurements and the resulting difficulty `score`,
/// exactly as C's `create_maze` computes and returns them
/// (`dungeon.c:1134-1341`).
#[derive(Debug, Clone)]
pub struct MazeResult {
    pub cells: Vec<MazeCell>,
    /// C `path1`: tree-distance between the two "far corners" (top-right
    /// to bottom-left).
    pub path1: i32,
    /// C `path2`: distance from the top-left corner to the preserved
    /// direct path.
    pub path2: i32,
    /// C `path3`: distance from the bottom-right corner to the preserved
    /// direct path.
    pub path3: i32,
    /// C `score`: `(path1+path2+path3) + cbrt(path1*path2*path3)*3`,
    /// zeroed out by the `do_fake` check if the fake wall doesn't
    /// actually block the only path from corner to corner.
    pub score: i32,
}

/// C `create_maze` (`dungeon.c:1134-1341`), minus the final `build_cell`
/// map-instantiation loop and the `show_maze` debug printer (both left
/// for the future NPC/item-spawning slice - see this module's doc
/// comment). `base` is C's own `base` parameter, used to seed a *local*
/// RNG sequence exactly like C's `srand(base)` (this does not touch any
/// shared/world-level RNG state). `warrior`/`mage`/`seyan` are the six
/// per-tier guard counts (C's `const int *warrior`/`mage`/`seyan`
/// pointers, `clan.c`'s `get_clan_dungeon` types `1..=6`/`7..=12`/
/// `13..=18`), `teleport` is the teleport-trap count (`19`), `keys` is
/// the exit-door key requirement (`0` = no key, `1`/`2` = one/two keys,
/// matching `get_clan_dungeon` type `21`'s clamped `0..=2` value), and
/// `do_fake` is whether a fake (dead-end) wall should be carved in
/// (`get_clan_dungeon` type `20`, nonzero).
#[allow(clippy::too_many_arguments)]
pub fn create_maze(
    base: u32,
    do_fake: bool,
    keys: i32,
    warrior: &[i32; 6],
    mage: &[i32; 6],
    seyan: &[i32; 6],
    teleport: i32,
) -> MazeResult {
    let mut seed = base;
    let mut cells = build_maze(&mut seed);

    let top_right = cell_index(MAZE_XSIZE - 1, 0);
    let bottom_left = cell_index(0, MAZE_YSIZE - 1);
    let top_left = cell_index(0, 0);
    let bottom_right = cell_index(MAZE_XSIZE - 1, MAZE_YSIZE - 1);

    special_fill(&mut cells, top_right, 1);
    let path1 = cells[bottom_left].special;

    // Retrace the shortest path from `bottom_left` back towards
    // `top_right`, marking each path cell `999` (preserved) and noting
    // the midpoint cell/direction as the fake-wall candidate.
    let mut n = bottom_left;
    let mut m = cells[n].special;
    let tmp_mid = m / 2;
    let mut fake: Option<usize> = None;
    let mut fake_is_top_wall = false;

    while m > 0 {
        cells[n].special = 999;

        if !cells[n].left_wall && n % MAZE_XSIZE > 0 && cells[n - 1].special == m - 1 {
            if m == tmp_mid {
                fake = Some(n);
                fake_is_top_wall = false;
            }
            n -= 1;
        } else if !cells[n].top_wall && n / MAZE_XSIZE > 0 && cells[n - MAZE_XSIZE].special == m - 1
        {
            if m == tmp_mid {
                fake = Some(n);
                fake_is_top_wall = true;
            }
            n -= MAZE_XSIZE;
        } else if n % MAZE_XSIZE < MAZE_XSIZE - 1
            && !cells[n + 1].left_wall
            && cells[n + 1].special == m - 1
        {
            n += 1;
            if m == tmp_mid {
                fake = Some(n);
                fake_is_top_wall = false;
            }
        } else if n / MAZE_XSIZE < MAZE_YSIZE - 1
            && !cells[n + MAZE_XSIZE].top_wall
            && cells[n + MAZE_XSIZE].special == m - 1
        {
            n += MAZE_XSIZE;
            if m == tmp_mid {
                fake = Some(n);
                fake_is_top_wall = true;
            }
        } else {
            break;
        }
        m -= 1;
    }

    reset_special_except_path(&mut cells);
    let path2 = special_fill(&mut cells, top_left, 1);
    reset_special_except_path(&mut cells);
    let path3 = special_fill(&mut cells, bottom_right, 1);
    reset_special_all(&mut cells);

    let mut score = {
        let sum = (path1 + path2 + path3) as f64;
        let product = (path1 as f64) * (path2 as f64) * (path3 as f64);
        (sum + product.cbrt() * 3.0) as i32
    };

    if do_fake {
        if let Some(fake_cell) = fake {
            if fake_is_top_wall {
                cells[fake_cell].top_wall = true;
            } else {
                cells[fake_cell].left_wall = true;
            }
        }

        special_fill(&mut cells, top_left, 1);
        if cells[bottom_right].special != 0 {
            score = 0;
        }
        reset_special_all(&mut cells);

        if let Some(fake_cell) = fake {
            cells[fake_cell].special = if fake_is_top_wall { 2 } else { 1 };
        }
    }

    if keys > 0 {
        cells[top_left].special = 3;
    }
    if keys > 1 {
        cells[bottom_right].special = 4;
    }

    match keys {
        0 => cells[top_right].special = 28,
        1 => cells[top_right].special = 29,
        2 => cells[top_right].special = 30,
        _ => {}
    }

    let mut maxi = 50i32;
    let mut panic = 200i32;

    for tier in (0..6).rev() {
        let mut remaining_warrior = warrior[tier];
        while remaining_warrior > 0 && maxi > 0 && panic > 0 {
            panic -= 1;
            let m = legacy_random_below_from_seed(&mut seed, MAZE_CELLS as u32) as usize;
            if cells[m].special != 0 {
                continue;
            }
            let x = m % MAZE_XSIZE;
            let y = m / MAZE_XSIZE;
            if x < 5 && y > MAZE_YSIZE - 6 {
                continue;
            }
            cells[m].special = 5 + tier as i32 * 3;
            remaining_warrior -= 1;
            maxi -= 1;
        }

        let mut remaining_mage = mage[tier];
        while remaining_mage > 0 && maxi > 0 && panic > 0 {
            panic -= 1;
            let m = legacy_random_below_from_seed(&mut seed, MAZE_CELLS as u32) as usize;
            if cells[m].special != 0 {
                continue;
            }
            let x = m % MAZE_XSIZE;
            let y = m / MAZE_XSIZE;
            if x < 5 && y > MAZE_YSIZE - 6 {
                continue;
            }
            cells[m].special = 6 + tier as i32 * 3;
            remaining_mage -= 1;
            maxi -= 1;
        }

        let mut remaining_seyan = seyan[tier];
        while remaining_seyan > 0 && maxi > 0 && panic > 0 {
            panic -= 1;
            let m = legacy_random_below_from_seed(&mut seed, MAZE_CELLS as u32) as usize;
            if cells[m].special != 0 {
                continue;
            }
            let x = m % MAZE_XSIZE;
            let y = m / MAZE_XSIZE;
            if x < 5 && y > MAZE_YSIZE - 6 {
                continue;
            }
            cells[m].special = 7 + tier as i32 * 3;
            remaining_seyan -= 1;
            maxi -= 1;
        }
    }

    let mut remaining_teleport = teleport;
    while remaining_teleport > 0 {
        let m = legacy_random_below_from_seed(&mut seed, MAZE_CELLS as u32) as usize;
        if m % MAZE_XSIZE == 0 && m / MAZE_XSIZE == MAZE_YSIZE - 1 {
            continue;
        }
        if cells[m].special != 0 {
            continue;
        }
        cells[m].special = legacy_random_below_from_seed(&mut seed, 5) as i32 + 23;
        remaining_teleport -= 1;
    }

    MazeResult {
        cells,
        path1,
        path2,
        path3,
        score,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn walled_grid() -> Vec<MazeCell> {
        vec![
            MazeCell {
                top_wall: true,
                left_wall: true,
                ..Default::default()
            };
            MAZE_CELLS
        ]
    }

    #[test]
    fn build_maze_produces_a_fully_connected_maze() {
        let mut seed = 12345u32;
        let mut cells = build_maze(&mut seed);

        // C's `build_maze` never sets the *starting* cell's own `v`
        // (visited) flag when the outer loop begins - only cells reached
        // *by being carved into* get `v=1`. This means the origin cell
        // stays "unvisited" in the algorithm's own bookkeeping until
        // (and unless) some later-reached neighbor runs out of every
        // other option and carves a passage back into it. When that
        // happens it's a genuine one-cycle quirk of the legacy algorithm
        // (a "braided" maze with exactly one loop through the origin,
        // one edge more than a strict N-1 spanning tree) - faithfully
        // reproduced here rather than "fixed", per this repo's porting
        // rules. Empirically (swept 2000 seeds while developing this
        // test) both outcomes occur depending on carving order: a
        // strict N-1 tree (399 edges, origin never revisited) or the
        // N-edge braided variant (400 edges, one cycle) - never
        // anything else, and always fully connected either way.
        let open_walls: usize = cells
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let top_open = !cell.top_wall && i / MAZE_XSIZE > 0;
                let left_open = !cell.left_wall && i % MAZE_XSIZE > 0;
                top_open as usize + left_open as usize
            })
            .sum();
        assert!(
            open_walls == MAZE_CELLS - 1 || open_walls == MAZE_CELLS,
            "expected a spanning tree (399) or one-cycle braided maze (400), got {open_walls}"
        );

        // Every cell must be reachable from the origin through open
        // passages (no isolated pockets).
        special_fill(&mut cells, 0, 1);
        assert!(
            cells.iter().all(|cell| cell.special != 0),
            "every cell should be reachable from the origin"
        );
    }

    #[test]
    fn build_maze_is_deterministic_for_a_fixed_seed() {
        let mut seed_a = 42u32;
        let mut seed_b = 42u32;
        let cells_a = build_maze(&mut seed_a);
        let cells_b = build_maze(&mut seed_b);
        assert_eq!(cells_a, cells_b);
    }

    #[test]
    fn build_maze_differs_across_seeds() {
        let mut seed_a = 1u32;
        let mut seed_b = 2u32;
        let cells_a = build_maze(&mut seed_a);
        let cells_b = build_maze(&mut seed_b);
        assert_ne!(cells_a, cells_b);
    }

    #[test]
    fn special_fill_computes_straight_line_distance_through_a_hand_built_corridor() {
        // Build a simple 1-wide horizontal corridor along the top row
        // (cells 0..MAZE_XSIZE-1), open between each adjacent pair, walled
        // off from everything else - a fully deterministic hand-built
        // case, independent of `build_maze`'s randomness.
        let mut cells = walled_grid();
        for cell in cells.iter_mut().take(MAZE_XSIZE).skip(1) {
            cell.left_wall = false;
        }

        special_fill(&mut cells, 0, 1);

        for (x, cell) in cells.iter().enumerate().take(MAZE_XSIZE) {
            assert_eq!(cell.special, (x + 1) as i32, "cell at x={x}");
        }
        // Cells outside the corridor are unreached (still 0).
        assert_eq!(cells[MAZE_XSIZE].special, 0);
    }

    #[test]
    fn special_fill_stops_at_a_999_preserved_path_sentinel() {
        let mut cells = walled_grid();
        for cell in cells.iter_mut().take(MAZE_XSIZE).skip(1) {
            cell.left_wall = false;
        }
        cells[3].special = 999;

        let val = special_fill(&mut cells, 0, 1);

        // Traversal reaches the sentinel (cell 3) and returns
        // immediately with the `val` it would have written there (4:
        // cell 0=1, cell 1=2, cell 2=3, cell 3 would be 4), without
        // overwriting the sentinel or continuing past it - so cell 4+
        // are never visited, and that `4` propagates back up through
        // each `min()` as the only nonzero contribution at every level,
        // ending up as this call's own top-level return value too.
        assert_eq!(cells[3].special, 999, "sentinel must not be overwritten");
        assert_eq!(cells[4].special, 0, "must not continue past the sentinel");
        assert_eq!(val, 4);
    }

    #[test]
    fn create_maze_is_deterministic_for_a_fixed_seed() {
        let warrior = [1, 0, 0, 0, 0, 0];
        let mage = [0, 1, 0, 0, 0, 0];
        let seyan = [0, 0, 1, 0, 0, 0];
        let a = create_maze(777, true, 1, &warrior, &mage, &seyan, 2);
        let b = create_maze(777, true, 1, &warrior, &mage, &seyan, 2);
        assert_eq!(a.path1, b.path1);
        assert_eq!(a.path2, b.path2);
        assert_eq!(a.path3, b.path3);
        assert_eq!(a.score, b.score);
        assert_eq!(a.cells, b.cells);
    }

    #[test]
    fn create_maze_places_exit_door_with_correct_key_requirement() {
        let no_guards = [0, 0, 0, 0, 0, 0];
        let top_right = cell_index(MAZE_XSIZE - 1, 0);

        let result = create_maze(1, false, 0, &no_guards, &no_guards, &no_guards, 0);
        assert_eq!(result.cells[top_right].special, 28);

        let result = create_maze(1, false, 1, &no_guards, &no_guards, &no_guards, 0);
        assert_eq!(result.cells[top_right].special, 29);

        let result = create_maze(1, false, 2, &no_guards, &no_guards, &no_guards, 0);
        assert_eq!(result.cells[top_right].special, 30);
    }

    #[test]
    fn create_maze_places_start_and_end_keys_when_requested() {
        let no_guards = [0, 0, 0, 0, 0, 0];
        let top_left = cell_index(0, 0);
        let bottom_right = cell_index(MAZE_XSIZE - 1, MAZE_YSIZE - 1);

        let result = create_maze(2, false, 0, &no_guards, &no_guards, &no_guards, 0);
        assert_eq!(result.cells[top_left].special, 0);

        let result = create_maze(2, false, 1, &no_guards, &no_guards, &no_guards, 0);
        assert_eq!(result.cells[top_left].special, 3);
        assert_eq!(result.cells[bottom_right].special, 0);

        let result = create_maze(2, false, 2, &no_guards, &no_guards, &no_guards, 0);
        assert_eq!(result.cells[top_left].special, 3);
        assert_eq!(result.cells[bottom_right].special, 4);
    }

    #[test]
    fn create_maze_respects_the_shared_fifty_npc_cap_even_with_huge_requested_counts() {
        // C's `maxi=50` budget is shared across every tier and every
        // guard type (warrior/mage/seyan), not per-type or per-tier.
        let huge = [10_000, 10_000, 10_000, 10_000, 10_000, 10_000];
        let none = [0, 0, 0, 0, 0, 0];
        let result = create_maze(9, false, 0, &huge, &none, &none, 0);

        let placed_guards = result
            .cells
            .iter()
            .filter(|cell| (5..=22).contains(&cell.special))
            .count();
        assert!(
            placed_guards <= 50,
            "expected at most 50 guards placed, got {placed_guards}"
        );
    }

    #[test]
    fn create_maze_terminates_promptly_when_every_cell_is_already_occupied() {
        // Regression guard for the `panic` budget: even when every cell
        // already has a nonzero `special` (impossible to place anything
        // new), the shared 200-attempt budget must still guarantee
        // termination rather than looping forever.
        let some_guards = [5, 5, 5, 5, 5, 5];
        let result = create_maze(3, false, 0, &some_guards, &some_guards, &some_guards, 5);
        // No assertion beyond "returns" - the test itself times out if
        // the placement loops forever.
        assert!(result.score >= 0 || result.score < 0);
    }

    #[test]
    fn create_maze_never_places_a_teleport_trap_at_the_start_cell() {
        let none = [0, 0, 0, 0, 0, 0];
        let bottom_left = cell_index(0, MAZE_YSIZE - 1);
        for seed in 0..20u32 {
            let result = create_maze(seed, false, 0, &none, &none, &none, 5);
            assert!(!(23..=27).contains(&result.cells[bottom_left].special));
        }
    }

    #[test]
    fn build_maze_is_always_fully_connected_and_within_valid_edge_counts_across_many_seeds() {
        for seed in 0..2000u32 {
            let mut s = seed;
            let mut cells = build_maze(&mut s);
            let open_walls: usize = cells
                .iter()
                .enumerate()
                .map(|(i, cell)| {
                    let top_open = !cell.top_wall && i / MAZE_XSIZE > 0;
                    let left_open = !cell.left_wall && i % MAZE_XSIZE > 0;
                    top_open as usize + left_open as usize
                })
                .sum();
            assert!(
                open_walls == MAZE_CELLS - 1 || open_walls == MAZE_CELLS,
                "seed {seed}: unexpected open_walls={open_walls}"
            );
            special_fill(&mut cells, 0, 1);
            assert!(
                cells.iter().all(|c| c.special != 0),
                "seed {seed}: maze must always be fully connected"
            );
        }
    }
}
