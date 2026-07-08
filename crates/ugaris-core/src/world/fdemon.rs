//! Area 8 (`src/area/8/fdemon.c`) Fire Demon hunt-waypoint graph.
//!
//! Ports C's file-static `struct waypoint wp[MAXWAY]`/`maxway` (`fdemon.c:
//! 2492-2503`) plus `find_waypoints`/`dist_to_waypoint`/
//! `find_closest_waypoint`/`find_way_to_waypoint`/`add_enemy_to_waypoint`/
//! `may_hunt_there`/`hunt_driver` (`:2505-2739`). One `World::
//! fdemon_waypoints` per area-process matches C's one-process-per-area
//! file-static architecture (see `World::area_id`'s doc comment) - same
//! precedent as `world::pents`.
//!
//! The graph is built lazily the first time any `CDR_FDEMON_DEMON`
//! character's tick runs (C's own `if (maxway==1) find_waypoints();` guard
//! inside `fdemon_demon`, checked - and a no-op after the first real build -
//! every tick), scanning every placed `IDR_FDEMONWAYPOINT` map item
//! (`item_driver::area8_fdemon`) and connecting pairs roughly 40 tiles apart
//! (either axis) that have a walkable path between them, exactly like C.
//! Index `0` is an unused sentinel matching C's 1-based `wp[]`/`maxway`
//! indexing (`0` in a `left`/`right`/`up`/`down` slot means "no
//! connection", matching C's `if (wp[to].right)` truthiness checks on a
//! zero-initialized `int`).
//!
//! `find_way_to_waypoint`'s C implementation is a reverse-BFS-from-goal
//! search using an array-based queue that gets fully re-`qsort`ed by cost
//! after every expansion (`findwaycmp`, ascending); since every edge has
//! uniform weight 1, this always degenerates to plain BFS order by
//! distance-from-goal, and C's `qsort` is not guaranteed stable, so the
//! *exact* tie-break order among multiple equally-short paths is already
//! unspecified in the original. This port uses a plain `VecDeque`-based BFS
//! (same shortest-hop-count result; ties may differ from C in which
//! neighbor happens to be discovered first - undocumented in C either).

use std::collections::VecDeque;

use super::*;
use crate::item_driver::IDR_FDEMONWAYPOINT;

/// C `#define MAXWAY 50`.
const MAXWAY: usize = 50;

/// C `wp[n].last_enemy && ticker - wp[n].last_enemy < TICKS*60` window used
/// by `hunt_driver`'s own best-candidate scan (`fdemon.c:2705`).
const HUNT_STALE_TICKS: i32 = (TICKS_PER_SECOND * 60) as i32;

#[derive(Debug, Clone, Copy, Default)]
pub struct FdemonWaypoint {
    pub x: u16,
    pub y: u16,
    /// `0` means "no sighting recorded yet", matching C's zero-initialized
    /// `int last_enemy` (a real `ticker` value is never exactly `0` in a
    /// running server, same convention C itself relies on).
    pub last_enemy_tick: i32,
    /// `0` means "no connection" (1-based waypoint index otherwise),
    /// matching C's zero-initialized `int` connection slots.
    pub left: usize,
    pub right: usize,
    pub up: usize,
    pub down: usize,
}

/// C `may_hunt_there(cn, x, y)` (`fdemon.c:2686-2702`): is `(x, y)` within
/// hunting range of `cn`'s home (`ch[cn].tmpx`/`tmpy`, this port's
/// `Character::rest_x`/`rest_y`)? Deliberately asymmetric bounds (up to 30
/// tiles east/south of home, up to 70 tiles west/north), ported digit for
/// digit rather than "fixed".
pub(crate) fn fdemon_may_hunt_there(home_x: u16, home_y: u16, x: u16, y: u16) -> bool {
    let (home_x, home_y, x, y) = (
        i32::from(home_x),
        i32::from(home_y),
        i32::from(x),
        i32::from(y),
    );
    if x - home_x > 30 {
        return false;
    }
    if y - home_y > 30 {
        return false;
    }
    if home_x - x > 70 {
        return false;
    }
    if home_y - y > 70 {
        return false;
    }
    true
}

impl World {
    /// C `if (maxway==1) find_waypoints();` (`fdemon.c:2751-2753`), called
    /// every `fdemon_demon` tick but only actually (re)scans once.
    pub(crate) fn ensure_fdemon_waypoints_built(&mut self) {
        if !self.fdemon_waypoints.is_empty() {
            return;
        }
        // Index 0 sentinel, matching C's 1-based `wp[]`/`maxway` (`maxway`
        // starts at `1`, i.e. "no real waypoints yet").
        self.fdemon_waypoints.push(FdemonWaypoint::default());

        let mut positions: Vec<(u32, u16, u16)> = self
            .items
            .values()
            .filter(|item| item.driver == IDR_FDEMONWAYPOINT)
            .map(|item| (item.id.0, item.x, item.y))
            .collect();
        // C scans `it[]` in ascending item-slot order (`for (in=1;
        // in<MAXITEM; in++)`, `fdemon.c:2509`); ascending `ItemId` is this
        // port's equivalent creation-order proxy - *not* sorted by
        // position, since C's `left`/`right`/`up`/`down` connection scan
        // below is itself order-dependent (see `fdemon_waypoints_connected`
        // callers' asymmetric `dx`/`dy` sign checks, ported digit for
        // digit from C rather than "fixed" to be order-independent).
        positions.sort_by_key(|&(id, _, _)| id);
        for (_, x, y) in positions {
            if self.fdemon_waypoints.len() >= MAXWAY {
                break;
            }
            self.fdemon_waypoints.push(FdemonWaypoint {
                x,
                y,
                ..Default::default()
            });
        }

        let count = self.fdemon_waypoints.len();
        for n in 1..count {
            for m in (n + 1)..count {
                let (nx, ny) = (
                    i32::from(self.fdemon_waypoints[n].x),
                    i32::from(self.fdemon_waypoints[n].y),
                );
                let (mx, my) = (
                    i32::from(self.fdemon_waypoints[m].x),
                    i32::from(self.fdemon_waypoints[m].y),
                );
                let dx = nx - mx;
                let dy = ny - my;

                if dx > 35 && dx < 45 && dy.abs() < 10 && self.fdemon_waypoints_connected(n, m) {
                    self.fdemon_waypoints[n].left = m;
                    self.fdemon_waypoints[m].right = n;
                    continue;
                }
                if dy > 35 && dy < 45 && dx.abs() < 10 && self.fdemon_waypoints_connected(n, m) {
                    self.fdemon_waypoints[n].up = m;
                    self.fdemon_waypoints[m].down = n;
                }
            }
        }
    }

    fn fdemon_waypoints_connected(&self, n: usize, m: usize) -> bool {
        let (nx, ny) = (self.fdemon_waypoints[n].x, self.fdemon_waypoints[n].y);
        let (mx, my) = (self.fdemon_waypoints[m].x, self.fdemon_waypoints[m].y);
        pathfinder(
            &self.map,
            usize::from(nx),
            usize::from(ny),
            usize::from(mx),
            usize::from(my),
            1,
            Some(400),
        )
        .direction
        .is_some()
    }

    fn fdemon_dist_to_waypoint(&self, x: u16, y: u16, n: usize) -> i32 {
        i32::from(x.abs_diff(self.fdemon_waypoints[n].x))
            + i32::from(y.abs_diff(self.fdemon_waypoints[n].y))
    }

    /// C `find_closest_waypoint` (`fdemon.c:2554-2565`).
    pub(crate) fn fdemon_find_closest_waypoint(&self, x: u16, y: u16) -> usize {
        let mut best_dist = 99;
        let mut best_wp = 0;
        for n in 1..self.fdemon_waypoints.len() {
            let dist = self.fdemon_dist_to_waypoint(x, y, n);
            if dist < best_dist {
                best_dist = dist;
                best_wp = n;
            }
        }
        best_wp
    }

    /// C `add_enemy_to_waypoint` (`fdemon.c:2675-2684`): called with the
    /// *sighted character's* position, not the demon's own.
    pub(crate) fn add_fdemon_enemy_to_waypoint(&mut self, x: u16, y: u16) {
        let n = self.fdemon_find_closest_waypoint(x, y);
        if n == 0 {
            return;
        }
        let wp = &self.fdemon_waypoints[n];
        if x.abs_diff(wp.x) < 30 && y.abs_diff(wp.y) < 30 {
            let tick = self.tick.0 as i32;
            self.fdemon_waypoints[n].last_enemy_tick = tick;
        }
    }

    /// C `find_way_to_waypoint(from, to, flags)` (`fdemon.c:2586-2668`) -
    /// see module doc comment for the BFS-vs-qsort equivalence note. C's
    /// `FWW_NOENEMY` flag is never set by `hunt_driver`'s only call site
    /// (always `flags=0`), so it's ported as an always-`true` filter here
    /// rather than a real parameter.
    pub(crate) fn fdemon_find_way_to_waypoint(&self, from: usize, to: usize) -> usize {
        if from == 0 || to == 0 || from >= self.fdemon_waypoints.len() {
            return 0;
        }
        let mut seen = vec![false; self.fdemon_waypoints.len()];
        let mut queue: VecDeque<usize> = VecDeque::new();
        queue.push_back(to);
        seen[to] = true;

        while let Some(current) = queue.pop_front() {
            let wp = self.fdemon_waypoints[current];
            for next in [wp.right, wp.left, wp.up, wp.down] {
                if next == 0 {
                    continue;
                }
                if next == from {
                    return current;
                }
                if !seen[next] {
                    seen[next] = true;
                    queue.push_back(next);
                }
            }
        }
        0
    }

    /// C `hunt_driver(cn, dat)` (`fdemon.c:2704-2739`): walks the demon
    /// toward the most-recently-sighted enemy waypoint still within its
    /// home range. Returns `true` if a walk action was queued.
    pub(crate) fn fdemon_hunt_driver(&mut self, character_id: CharacterId, area_id: u16) -> bool {
        let Some(character) = self.characters.get(&character_id).cloned() else {
            return false;
        };
        let (home_x, home_y) = (character.rest_x, character.rest_y);
        let current = self.fdemon_find_closest_waypoint(character.x, character.y);

        let mut best_diff = HUNT_STALE_TICKS;
        let mut best_n = 0usize;
        let tick = self.tick.0 as i32;
        for n in 1..self.fdemon_waypoints.len() {
            let wp = self.fdemon_waypoints[n];
            if wp.last_enemy_tick == 0 || !fdemon_may_hunt_there(home_x, home_y, wp.x, wp.y) {
                continue;
            }
            let diff = tick - wp.last_enemy_tick;
            if diff < best_diff {
                best_diff = diff;
                best_n = n;
            }
        }
        if best_n == 0 {
            return false;
        }

        let target_wp = self.fdemon_waypoints[best_n];
        if current == best_n {
            if character.x.abs_diff(target_wp.x) as i32 + character.y.abs_diff(target_wp.y) as i32
                > 6
            {
                return self.setup_walk_toward(
                    character_id,
                    usize::from(target_wp.x),
                    usize::from(target_wp.y),
                    6,
                    area_id,
                    false,
                );
            }
            return false;
        }

        let next_hop = self.fdemon_find_way_to_waypoint(current, best_n);
        if next_hop == 0 {
            return false;
        }
        let hop = self.fdemon_waypoints[next_hop];
        self.setup_walk_toward(
            character_id,
            usize::from(hop.x),
            usize::from(hop.y),
            6,
            area_id,
            false,
        )
    }
}
