//! Areas 23/24 (`src/area/23_24/strategy.c`) AI-opponent driver
//! (`ai_init`/`ai_main`, `:2269-2994`) - the last unported piece of the
//! whole "Areas 23/24" P4 task (see `crate::world::strategy`'s own module
//! doc comment for everything else already ported: the mission economy,
//! `str_ticker`, the mission entry queue, the `#`/`/` player command
//! table, and - most importantly for this file - the recruitable worker
//! character driver itself, `crate::world::npc::area23_24::worker`, which
//! is what an AI opponent's NPCs actually run once spawned).
//!
//! This is a genuinely large brain-simulation subsystem (~725 lines
//! across `ai_init`+`ai_main`) that plans, every tick, which order every
//! worker/guard NPC belonging to one AI-controlled battleground slot
//! should carry out next - mine, haul, train, guard, or fight - based on
//! a small "place graph" (mine/depot/storage connectivity + pathing
//! distance) built once at `ai_init` time. This first slice ports the
//! structural pieces with no dependency on a live spawned AI army,
//! following the exact same "ported but not yet spawnable" precedent
//! `crate::world::strategy_worker`'s own module doc comment already
//! established for the (now-live) worker driver itself:
//!
//! - The `struct ai_npc`/`struct ai_place`/`struct ai_data` (`:1700-
//!   1787`) shapes, as [`AiNpc`]/[`AiPlace`]/[`AiData`] - `Vec`-backed
//!   (no fixed `MAXNPC`/`MAXPLACE` capacity like C's arrays) since every
//!   entry here is always a real, populated one (no C-style "slot 0 means
//!   unused" sentinel scheme is needed). `cserial` (C's recreate-detection
//!   guard against a stale `cn` slot reuse) is dropped - a Rust
//!   [`CharacterId`] is already a stable, never-reused identity, so no
//!   separate serial check is needed, matching the precedent already set
//!   for `ArenaContender`/`World::str_did_party_lose`'s own doc comment.
//!   `order`/`or1`/`or2` are kept as C's own raw order-code/two-`int`
//!   shape (not [`StrategyWorkerOrder`]) because the AI's own task
//!   assignment sometimes deliberately leaves a payload slot at a
//!   sentinel `0` that doesn't represent a real item/character
//!   (`task_take`'s `or2 = 0`, "no leader", below) - a shape
//!   [`StrategyWorkerOrder::Take`]'s `leader: CharacterId` field can't
//!   represent. Converting this raw form into a live worker's
//!   `StrategyWorkerOrder` (C's own end-of-`ai_main`-loop `dat->order =
//!   ad->an[n].order` sync, `:2967-2971`) is left for the future slice
//!   that actually wires a live AI-controlled worker character (needs
//!   `create_eguard`/an AI-side `spawner_sub` call, both still unported).
//!   The threat-detection fields (`threat`/`threatlevel`/`threatnlevel`/
//!   `threatncount`/`owned`/`enemy_possible`, `:1729-1737`) are not
//!   carried on [`AiPlace`] yet - nothing in this slice populates or
//!   reads them - except `threatcount`, which [`World::ai_nag_attack`]
//!   needs; add the rest alongside whatever future slice ports the
//!   per-tick threat scan (`ai_main`, `:2541-2630`) that populates them.
//! - [`World::ai_update_npc_place`]/[`World::ai_subtask_move`]: C
//!   `update_npc_place`/`subtask_move` (`:1797-1863`) - the place-graph
//!   navigation primitives every `task_*` function below shares.
//!   `ai_check_target` (`:1789-1795`, C's own pathfinder callback -
//!   passable unless `MF_MOVEBLOCK`) has no bespoke Rust equivalent: the
//!   existing [`pathfinder`] only exposes a fixed `MOVEBLOCK`-or-
//!   `TMOVEBLOCK` "Normal" mode (see `crate::path`'s own `PathBlockMode`),
//!   so this port reuses that mode rather than extend `crate::path`'s
//!   public API for this one narrow caller - a documented, minor
//!   deviation: a temporarily-blocked tile (e.g. a closed door an item
//!   drove `TMOVEBLOCK` onto) is treated as impassable here where C's own
//!   `ai_check_target` would still route through it.
//! - The seven `task_*` functions (`task_idle`/`task_take`/`task_guard`/
//!   `task_mine`/`task_transfer`/`task_train`/`task_fight`, `:1865-1994`)
//!   - each resolves one worker's next order given its current task/
//!   target/place. `task_idle` is the one exception needing `&mut World`
//!   rather than just `&World`: C's `restplace(cn, ..., dat)` reads *and
//!   mutates* the live worker character's own persisted `struct
//!   strategy_data::restplace` field (already ported as
//!   [`StrategyWorkerDriverData::restplace`]/[`World::
//!   strategy_worker_rest_place`]) - this slice reuses that machinery
//!   directly rather than duplicate it, auto-vivifying a default
//!   `StrategyWorkerDriverData` on the live character exactly like C's
//!   own `set_data` auto-allocates on first use (matching `crate::world::
//!   npc::area23_24::worker::process_strategy_worker_tick`'s own "missing
//!   state defaults, doesn't early-return" precedent, since a live
//!   AI-controlled worker's driver state is not guaranteed to have been
//!   touched by a real tick yet the first time `ai_main` itself runs).
//! - [`AiData::assign_npc`]/[`AiData::add_worker`]/[`AiData::
//!   add_etguard`]/[`AiData::add_guard`]/[`AiData::remove_guard`]/
//!   [`AiData::remove_worker`] (`:1996-2105`): the roster-bookkeeping
//!   primitives that hand a free worker/guard slot an assignment (or free
//!   one back up) - all pure `AiData`-only mutations, no live character
//!   access needed.
//! - [`AiData::wantguardcnt`]/[`World::ai_assign_guards`]/[`AiData::
//!   remove_free_guards`]/[`World::ai_nag_attack`] (`:2111-2267`,
//!   `:2432-2447`): the defense-allocation logic that decides how many
//!   guards a threatened place needs and which idle guards to send.
//!   `assign_guards`'s `THREAT(cn)` macro (`ch[cn].level` cubed) and its
//!   HP-readiness gate both read the *live* character directly rather
//!   than the cached [`AiNpc::level`] copy (a real, deliberate C
//!   distinction - the cached copy is only refreshed once per `ai_main`
//!   tick by the still-unported roster-refresh block, `:2461-2482`, so
//!   using it here would be one tick stale), so
//!   [`World::ai_assign_guards`] is a `World` method rather than a pure
//!   `AiData` one.
//!
//! Second slice done: [`World::ai_init`] (`strategy.c:2269-2427`) itself -
//! the place-graph construction from `IDR_STR_MINE`/`_DEPOT`/`_STORAGE`
//! items sharing the spawner's area slot, the `pathfinder`-based
//! distance/parent BFS connecting them all back to storage, the
//! `enemy_possible` up-propagation, and discovering the party's own
//! already-live `CDR_STRATEGY` roster (classifying each into
//! [`AiTask::Ignore`]/[`AiTask::EGuard`]/[`AiTask::Idle`] exactly like C,
//! including the subtle unconditional post-classification `used = -1`
//! reset - see [`World::ai_init`]'s own doc comment). Still not wired to
//! any live tick call site (no caller ever constructs a real
//! [`AiData`] outside tests yet - same "ported but not yet spawnable"
//! precedent as every other piece of this subsystem).
//!
//! REMAINING (tracked in `PORTING_TODO.md`, left `[~]` on purpose):
//! `ai_main`'s own outer per-tick body (`:2449-2973`) - the per-tick
//! roster/threat refresh (re-deriving `used`/`gcnt` from what `ai_init`
//! just seeded, the enemy-presence map scan that populates
//! `threat`/`threatlevel`/`threatcount`/`threatncount`/`owned`), worker
//! spawning, and the actual per-npc task-dispatch `switch` that calls the
//! `task_*` functions this file already ports - plus `create_eguard`
//! (`:2987-3040`, needs `ZoneLoader`) and the "place eternal guards" tail
//! that calls it (`:2892-2903`).

use super::*;
use crate::character_driver::CDR_STRATEGY;
use crate::path::pathfinder;
use crate::player::StrategyPpd;

/// C `#define MAX_AI 32` (`strategy.c:1672`): how many concurrent
/// AI-controlled battleground parties can exist (kept for documentation/
/// parity only - nothing in this port allocates a fixed-size `[AiData;
/// MAX_AI]` array yet, since no caller stores one per-slot registry).
pub const MAX_AI: usize = 32;
/// C `#define MAXWORKER 4` (`:1687`): worker slots per [`AiPlace`].
pub const AI_MAXWORKER: usize = 4;
/// C `#define MAXGUARD 12` (`:1688`): guard slots per [`AiData`].
pub const AI_MAXGUARD: usize = 12;
/// C `#define MAXDISTANCE 64` (`:1693`): place-graph BFS depth cap used
/// by the still-unported `ai_init` connectivity scan.
pub const AI_MAXDISTANCE: i32 = 64;

/// C `#define PT_STORAGE 1`/`PT_DEPOT 2`/`PT_MINE 3` (`:1683-1685`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiPlaceType {
    Storage,
    Depot,
    Mine,
}

/// C `#define T_IDLE 0`/`T_MINE 1`/`T_TRANSFER 2`/`T_FIGHT 4`/`T_EGUARD
/// 5`/`T_IGNORE 6`/`T_TAKE 7` (`:1674-1681` - note C itself never defines
/// a `T_3`, an intentional gap in the original numbering carried here
/// only as a comment, not a variant).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiTask {
    Idle,
    Mine,
    Transfer,
    Fight,
    EGuard,
    Ignore,
    Take,
}

impl Default for AiTask {
    fn default() -> Self {
        AiTask::Idle
    }
}

/// C `#define WT_DIRECT 1`/`WT_UP 2`/`WT_DOWN 3` (`:1696-1698`): how
/// [`World::ai_subtask_move`] last routed a worker toward its target -
/// `None` matches C's zero-initialized "never set" default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiWalkType {
    Direct,
    Up,
    Down,
}

/// C `struct ai_npc` (`strategy.c:1700-1714`) - one live worker/guard
/// belonging to an AI-controlled party. See this module's doc comment for
/// why `order`/`or1`/`or2` stay raw ints and `cserial` is dropped.
#[derive(Debug, Clone)]
pub struct AiNpc {
    pub cn: CharacterId,
    pub x: u16,
    pub y: u16,
    pub platin: i32,
    pub level: i32,
    pub order: i32,
    pub or1: i32,
    pub or2: i32,
    pub task: AiTask,
    /// Place-graph index this NPC is trying to reach. Always a valid
    /// index into [`AiData::places`] (C's own zero-initialized default,
    /// place `0`, is always the party's storage).
    pub target: usize,
    /// Place-graph index this NPC is currently considered to be at,
    /// refreshed by [`World::ai_update_npc_place`].
    pub current: usize,
    pub walktype: Option<AiWalkType>,
    /// C's `-1` "free"/`0` "busy, no specific target (guard on standby)"/
    /// positive "assigned to this place index" tri-state (`:1712`) - kept
    /// as a raw `i32` rather than `Option<usize>` since `0` is a real,
    /// distinct state from both "free" and "assigned to place 0", a
    /// genuine ambiguity already present in the C source itself (harmless
    /// there because nothing ever reads `used` back as a place index).
    pub used: i32,
    /// C's `0` "no forced target"/positive "place index" pair (`:1713`),
    /// same raw-`i32` rationale as `used`.
    pub ftarget: i32,
}

impl AiNpc {
    /// A freshly-discovered/spawned NPC, matching C's zero-initialized
    /// `struct ai_npc` plus the `used = -1` (free) stamp every one of
    /// C's own NPC-registration call sites applies right after
    /// zero-initializing (`ai_init`'s NPC-scan loop, `:2423`; the
    /// still-unported worker-spawn loop, `:2665`).
    pub fn new(cn: CharacterId, x: u16, y: u16, level: i32) -> Self {
        Self {
            cn,
            x,
            y,
            platin: 0,
            level,
            order: OR_NONE,
            or1: 0,
            or2: 0,
            task: AiTask::Idle,
            target: 0,
            current: 0,
            walktype: None,
            used: -1,
            ftarget: 0,
        }
    }
}

/// C `struct ai_place` (`strategy.c:1716-1738`) - one mine/depot/storage
/// node in a party's place graph. See this module's doc comment for why
/// the threat-detection fields besides `threatcount` aren't carried yet.
#[derive(Debug, Clone)]
pub struct AiPlace {
    pub place_type: AiPlaceType,
    pub item: ItemId,
    pub x: u16,
    pub y: u16,
    /// BFS depth from this place to the party's storage; `-1` means
    /// "not yet connected" (C's zero-initialized... actually explicitly
    /// `-1`-stamped default before the still-unported `ai_init` BFS
    /// runs).
    pub dist: i32,
    /// Place-graph index one step closer to storage; `-1` means "no
    /// parent yet" (storage's own parent, matching C's `ap[0].parent =
    /// -1`).
    pub parent: i32,
    pub wcnt: i32,
    /// C `int worker[MAXWORKER]`: NPC-array indices assigned here, `-1`
    /// for an empty slot.
    pub worker: [i32; AI_MAXWORKER],
    /// NPC-array index of this place's eternal guard, `-1` if none.
    pub eguard: i32,
    /// C `double threatcount` (`:1733`) - not yet populated by any
    /// ported code (see module doc comment); carried so
    /// [`World::ai_nag_attack`] can read it once a future slice's threat
    /// scan starts writing it.
    pub threatcount: f64,
    /// C's `enemy_possible` field (`:1732`): could an enemy ever
    /// approach through this place? Stamped `true` directly on every
    /// non-owned enemy storage place [`World::ai_init`] discovers
    /// (`:2346`), then propagated up the whole parent chain toward
    /// storage (`:2388-2395`).
    pub enemy_possible: bool,
}

impl AiPlace {
    /// C's zero-initialized `struct ai_place` plus the explicit `-1`
    /// stamps every one of `ai_init`'s own place-registration call sites
    /// applies to `worker[]`/`eguard` (`:2300-2302`,`:2320-2322`,
    /// `:2332-2334`,`:2344-2347`) right after zero-initializing.
    pub fn new(place_type: AiPlaceType, item: ItemId, x: u16, y: u16) -> Self {
        Self {
            place_type,
            item,
            x,
            y,
            dist: -1,
            parent: -1,
            wcnt: 0,
            worker: [-1; AI_MAXWORKER],
            eguard: -1,
            threatcount: 0.0,
            enemy_possible: false,
        }
    }
}

/// C's file-static `struct ai_data ai_data[MAX_AI], *ad` (`strategy.c:
/// 1748-1787`) - one AI-controlled battleground party's full brain state.
#[derive(Debug, Clone)]
pub struct AiData {
    pub storage_item: ItemId,
    pub worklevel: i32,
    pub places: Vec<AiPlace>,
    pub npcs: Vec<AiNpc>,
    pub free_workers: i32,
    /// C `int guard[MAXGUARD]`: NPC-array indices on eternal-guard duty,
    /// `-1` for an empty slot.
    pub guard: [i32; AI_MAXGUARD],
    pub gcnt: i32,
    /// C `ad->etguardcnt` (`:2413`): how many roster entries are
    /// permanently-stationed eternal guards ([`AiTask::Ignore`]) -
    /// checked against `ppd.eguards` by the still-unported "place
    /// eternal guards" tail of `ai_main` (`:2892-2903`) to decide whether
    /// more should be created via the still-unported `create_eguard`.
    pub etguardcnt: i32,
    pub lastnag: i64,
    pub nagplace: i32,
    /// `-1` means "no guard currently nagging" (C's own `ad->nagguard =
    /// -1;` `ai_init` stamp, `:2288`).
    pub nagguard: i32,
    pub partner: Vec<ItemId>,
    pub ppd: StrategyPpd,
}

impl AiData {
    /// C `ai_init`'s standard-value setup (`strategy.c:2282-2290`), minus
    /// the storage/place-graph/NPC-roster discovery this slice doesn't
    /// port yet (see module doc comment) - callers build `places`/`npcs`
    /// themselves for now (directly, in tests, or - eventually - via the
    /// still-unported full `ai_init`).
    pub fn new(ppd: StrategyPpd) -> Self {
        Self {
            storage_item: ItemId(0),
            worklevel: 1,
            places: Vec::new(),
            npcs: Vec::new(),
            free_workers: 0,
            guard: [-1; AI_MAXGUARD],
            gcnt: 0,
            etguardcnt: 0,
            lastnag: 0,
            nagplace: 0,
            nagguard: -1,
            partner: Vec::new(),
            ppd,
        }
    }

    /// C `update_npc_place(int n)` (`strategy.c:1797-1814`): is NPC `n`
    /// still within 10 tiles (either axis) of the place it's considered
    /// "at"? If not, scan every place for one that now qualifies. C's
    /// `xlog("could not determine place...")` fallback (no match found at
    /// all) has no persisted-log sink in this port, same precedent as
    /// every other bare `xlog` call already documented elsewhere in this
    /// codebase - `current` is simply left unchanged.
    pub fn update_npc_place(&mut self, n: usize) {
        let t = self.npcs[n].current;
        let (nx, ny) = (i32::from(self.npcs[n].x), i32::from(self.npcs[n].y));
        let (tx, ty) = (i32::from(self.places[t].x), i32::from(self.places[t].y));
        if (nx - tx).abs() < 10 && (ny - ty).abs() < 10 {
            return;
        }

        for m in 0..self.places.len() {
            let (mx, my) = (i32::from(self.places[m].x), i32::from(self.places[m].y));
            if (nx - mx).abs() < 10 && (ny - my).abs() < 10 {
                self.npcs[n].current = m;
                return;
            }
        }
    }

    /// C `assign_npc(int n)` (`strategy.c:1996-2027`): hand place `n` the
    /// first free (`used == -1`) NPC as a worker (mine-type places get
    /// [`AiTask::Mine`], everything else [`AiTask::Transfer`] - matching
    /// C's own `else` covering both `PT_STORAGE` and `PT_DEPOT`).
    pub fn assign_npc(&mut self, place: usize) -> bool {
        let Some(m) = self.npcs.iter().position(|npc| npc.used == -1) else {
            return false;
        };

        self.npcs[m].task = if self.places[place].place_type == AiPlaceType::Mine {
            AiTask::Mine
        } else {
            AiTask::Transfer
        };
        self.npcs[m].target = place;

        if let Some(slot) = self.places[place].worker.iter_mut().find(|w| **w == -1) {
            *slot = m as i32;
        }
        self.places[place].wcnt += 1;

        self.npcs[m].used = place as i32;
        self.free_workers -= 1;
        true
    }

    /// C `add_worker(int task, int worker, int place)` (`:2029-2045`):
    /// same roster bookkeeping as [`Self::assign_npc`], but the caller
    /// picks the worker/task explicitly instead of scanning for a free
    /// one.
    pub fn add_worker(&mut self, task: AiTask, worker: usize, place: usize) {
        self.npcs[worker].task = task;
        self.npcs[worker].target = place;

        if let Some(slot) = self.places[place].worker.iter_mut().find(|w| **w == -1) {
            *slot = worker as i32;
        }
        self.places[place].wcnt += 1;

        self.npcs[worker].used = place as i32;
        self.free_workers -= 1;
    }

    /// C `add_etguard(int guard)` (`:2047-2055`): station NPC `guard` as
    /// the eternal guard of whichever place it's currently standing at.
    pub fn add_etguard(&mut self, guard: usize) {
        self.update_npc_place(guard);
        let t = self.npcs[guard].current;
        self.npcs[guard].target = t;
        self.places[t].eguard = guard as i32;
    }

    /// C `add_guard(int guard)` (`:2057-2070`): register NPC `guard` into
    /// the first free roving-guard slot, marking it "busy, standing by"
    /// (`used = 0`, see [`AiNpc::used`]'s own doc comment).
    pub fn add_guard(&mut self, guard: usize) -> bool {
        if let Some(slot) = self.guard.iter_mut().find(|g| **g == -1) {
            *slot = guard as i32;
        }
        self.gcnt += 1;
        self.npcs[guard].used = 0;
        self.free_workers -= 1;
        true
    }

    /// C `remove_guard(int guard)` (`:2072-2087`): the inverse of
    /// [`Self::add_guard`], freeing `guard` back to [`AiTask::Idle`].
    pub fn remove_guard(&mut self, guard: usize) -> bool {
        if let Some(slot) = self.guard.iter_mut().find(|g| **g == guard as i32) {
            *slot = -1;
        }
        self.gcnt -= 1;
        self.npcs[guard].used = -1;
        self.free_workers += 1;
        self.npcs[guard].task = AiTask::Idle;
        self.npcs[guard].target = 0;
        true
    }

    /// C `remove_worker(int worker)` (`:2089-2105`): the inverse of
    /// [`Self::assign_npc`]/[`Self::add_worker`].
    pub fn remove_worker(&mut self, worker: usize) -> bool {
        let place = self.npcs[worker].target;
        self.npcs[worker].task = AiTask::Idle;
        self.npcs[worker].target = 0;

        if let Some(slot) = self.places[place]
            .worker
            .iter_mut()
            .find(|w| **w == worker as i32)
        {
            *slot = -1;
        }
        self.places[place].wcnt -= 1;
        self.npcs[worker].used = 0;
        self.free_workers += 1;
        true
    }

    /// C `wantguardcnt(void)` (`strategy.c:2432-2447`): how many roving
    /// guards a party of this size wants, given its current live NPC
    /// count.
    pub fn wantguardcnt(&self, npc_cnt: i32) -> i32 {
        if npc_cnt <= 3 {
            return 0; // 3 - 0
        }
        if npc_cnt <= 4 {
            return 1; // 3 - 1
        }
        if npc_cnt <= 5 {
            return 2; // 3 - 2
        }
        if npc_cnt <= 6 {
            return 2; // 4 - 2
        }
        npc_cnt / 2
    }

    /// C `remove_free_guards(void)` (`:2195-2203`): recall every roving
    /// guard that isn't currently nagging and has no active target back
    /// to standby (`target = 0`).
    pub fn remove_free_guards(&mut self) {
        for n in 0..AI_MAXGUARD {
            let m = self.guard[n];
            if m != -1 && m != self.nagguard && self.npcs[m as usize].used == 0 {
                self.npcs[m as usize].target = 0;
            }
        }
    }

    /// C `ai_init`'s per-NPC roster-registration body (`strategy.c:2401-
    /// 2424`), factored out of [`World::ai_init`]'s discovery loop so it
    /// stays independently testable regardless of that loop's own
    /// `code`-vs-`Character::group` matching limitation (see
    /// [`World::ai_init`]'s own doc comment): push a freshly-discovered
    /// live NPC, classify it into [`AiTask::Ignore`] (already an eternal
    /// guard)/[`AiTask::EGuard`] (`has_exp` or `level > 50`)/
    /// [`AiTask::Idle`] (everything else), and apply C's unconditional
    /// post-classification `used = -1` reset.
    pub fn register_npc(
        &mut self,
        cn: CharacterId,
        x: u16,
        y: u16,
        level: i32,
        order: i32,
        or1: i32,
        or2: i32,
        has_exp: bool,
    ) -> usize {
        let mut npc = AiNpc::new(cn, x, y, level);
        npc.order = order;
        npc.or1 = or1;
        npc.or2 = or2;
        let m = self.npcs.len();
        self.npcs.push(npc);

        if order == OR_ETERNALGUARD {
            self.add_etguard(m);
            self.npcs[m].task = AiTask::Ignore;
            self.etguardcnt += 1;
        } else if has_exp || level > 50 {
            self.add_guard(m);
            self.npcs[m].task = AiTask::EGuard;
        } else {
            self.npcs[m].task = AiTask::Idle;
        }
        // C's unconditional post-classification `used = -1` reset
        // (`:2423`): deliberately undoes whatever `add_etguard`/
        // `add_guard` just set above - `ai_main`'s own "update guard
        // list" refresh (still unported), which always runs immediately
        // after `ai_init` in the same call, is what re-derives the real
        // `used`/`gcnt` state from this on the very next pass
        // (`:2484-2494`).
        self.npcs[m].used = -1;
        m
    }
}

impl World {
    /// C `subtask_move(int n)` (`strategy.c:1816-1863`): route NPC `n`
    /// one step closer to its target place through the place graph,
    /// setting [`AiNpc::order`]/`or1`/`or2` to an `OR_GUARD` waypoint and
    /// [`AiNpc::walktype`] to how it got there. A no-op if already within
    /// 5 tiles (either axis) of the target - matching C's own outer `if`
    /// guard exactly (everything below is that `if` body).
    pub fn ai_subtask_move(&self, ad: &mut AiData, n: usize) {
        let t = ad.npcs[n].target;
        let (nx, ny) = (i32::from(ad.npcs[n].x), i32::from(ad.npcs[n].y));
        let (tx, ty) = (i32::from(ad.places[t].x), i32::from(ad.places[t].y));
        if (nx - tx).abs() <= 5 && (ny - ty).abs() <= 5 {
            return;
        }

        // Can we go there without using waypoints?
        if (nx - tx).abs() < 20
            && (ny - ty).abs() < 20
            && pathfinder(
                &self.map,
                ad.npcs[n].x as usize,
                ad.npcs[n].y as usize,
                ad.places[t].x as usize,
                ad.places[t].y as usize,
                1,
                Some(500),
            )
            .direction
            .is_some()
        {
            ad.npcs[n].order = OR_GUARD;
            ad.npcs[n].or1 = i32::from(ad.places[t].x);
            ad.npcs[n].or2 = i32::from(ad.places[t].y);
            ad.npcs[n].walktype = Some(AiWalkType::Direct);
            return;
        }

        // We need waypoints: follow the parent path from target toward
        // storage until we find the place the NPC is currently at, then
        // go up (toward the target) one step from there.
        let mut last = t;
        let mut m = ad.places[t].parent;
        while m != -1 {
            let mu = m as usize;
            if mu == ad.npcs[n].current {
                ad.npcs[n].order = OR_GUARD;
                ad.npcs[n].or1 = i32::from(ad.places[last].x);
                ad.npcs[n].or2 = i32::from(ad.places[last].y);
                ad.npcs[n].walktype = Some(AiWalkType::Down);
                return;
            }
            last = mu;
            m = ad.places[mu].parent;
        }

        // NPC is not at any place on the path from target to storage:
        // make it go to storage (one step up from its own current place).
        let current = ad.npcs[n].current;
        let up = ad.places[current].parent;
        // C `xlog("NPC is lost: ...")` when `up == -1` - no persisted-log
        // sink, same precedent as `update_npc_place`'s own doc comment.
        let dest = if up != -1 { up as usize } else { current };
        ad.npcs[n].order = OR_GUARD;
        ad.npcs[n].or1 = i32::from(ad.places[dest].x);
        ad.npcs[n].or2 = i32::from(ad.places[dest].y);
        ad.npcs[n].walktype = Some(AiWalkType::Up);
    }

    /// C `update_npc_place`'s wrapper for callers that need `&World`
    /// anyway (every `task_*` function below) - delegates straight to
    /// [`AiData::update_npc_place`], which needs no `World` access at
    /// all.
    fn ai_update_npc_place(&self, ad: &mut AiData, n: usize) {
        ad.update_npc_place(n);
    }

    /// C `task_idle(int n)` (`strategy.c:1865-1886`): send an idle worker
    /// to its `restplace` beside its target place. See this module's doc
    /// comment for why this is the one `task_*` function needing `&mut
    /// World`.
    pub fn ai_task_idle(&mut self, ad: &mut AiData, n: usize) {
        let t = ad.npcs[n].target;
        self.ai_update_npc_place(ad, n);
        if t != ad.npcs[n].current {
            self.ai_subtask_move(ad, n);
            return;
        }

        let worker_id = ad.npcs[n].cn;
        if !self.characters.contains_key(&worker_id) {
            return;
        }
        let current_offset = match self
            .characters
            .get(&worker_id)
            .and_then(|c| c.driver_state.as_ref())
        {
            Some(CharacterDriverState::StrategyWorker(data)) => data.restplace,
            _ => None,
        };
        let (bx, by) = (ad.places[t].x, ad.places[t].y);
        let (new_offset, (x, y)) =
            self.strategy_worker_rest_place(worker_id, (bx, by), current_offset);
        if let Some(character) = self.characters.get_mut(&worker_id) {
            match character.driver_state.get_or_insert_with(|| {
                CharacterDriverState::StrategyWorker(StrategyWorkerDriverData::default())
            }) {
                CharacterDriverState::StrategyWorker(data) => data.restplace = new_offset,
                other => {
                    *other = CharacterDriverState::StrategyWorker(StrategyWorkerDriverData {
                        restplace: new_offset,
                        ..Default::default()
                    })
                }
            }
        }
        ad.npcs[n].order = OR_GUARD;
        ad.npcs[n].or1 = i32::from(x);
        ad.npcs[n].or2 = i32::from(y);
    }

    /// C `task_take(int n)` (`:1888-1904`).
    pub fn ai_task_take(&self, ad: &mut AiData, n: usize) {
        let t = ad.npcs[n].target;
        self.ai_update_npc_place(ad, n);
        if t != ad.npcs[n].current {
            self.ai_subtask_move(ad, n);
            return;
        }
        ad.npcs[n].order = OR_TAKE;
        ad.npcs[n].or1 = ad.places[t].item.0 as i32;
        ad.npcs[n].or2 = 0;
    }

    /// C `task_guard(int n)` (`:1906-1922`).
    pub fn ai_task_guard(&self, ad: &mut AiData, n: usize) {
        let t = ad.npcs[n].target;
        self.ai_update_npc_place(ad, n);
        if t != ad.npcs[n].current {
            self.ai_subtask_move(ad, n);
            return;
        }
        ad.npcs[n].order = OR_GUARD;
        ad.npcs[n].or1 = i32::from(ad.places[t].x);
        ad.npcs[n].or2 = i32::from(ad.places[t].y);
    }

    /// C `task_mine(int n)` (`:1924-1940`): unlike `task_idle`/`task_take`/
    /// `task_guard`, this (and every `task_*` function below) also
    /// accepts being at the target's *parent* place, not just the target
    /// itself - the worker's own per-tick `OR_MINE` order execution
    /// (already ported in `crate::world::npc::area23_24::worker`) handles
    /// shuttling between the two tiles.
    pub fn ai_task_mine(&self, ad: &mut AiData, n: usize) {
        let t = ad.npcs[n].target;
        self.ai_update_npc_place(ad, n);
        if t != ad.npcs[n].current && ad.places[t].parent != ad.npcs[n].current as i32 {
            self.ai_subtask_move(ad, n);
            return;
        }
        ad.npcs[n].order = OR_MINE;
        ad.npcs[n].or1 = ad.places[t].item.0 as i32;
        ad.npcs[n].or2 = ad.places[ad.places[t].parent as usize].item.0 as i32;
    }

    /// C `task_transfer(int n)` (`:1942-1958`).
    pub fn ai_task_transfer(&self, ad: &mut AiData, n: usize) {
        let t = ad.npcs[n].target;
        self.ai_update_npc_place(ad, n);
        if t != ad.npcs[n].current && ad.places[t].parent != ad.npcs[n].current as i32 {
            self.ai_subtask_move(ad, n);
            return;
        }
        ad.npcs[n].order = OR_TRANSFER;
        ad.npcs[n].or1 = ad.places[t].item.0 as i32;
        ad.npcs[n].or2 = ad.places[ad.places[t].parent as usize].item.0 as i32;
    }

    /// C `task_train(int n)` (`:1960-1976`).
    pub fn ai_task_train(&self, ad: &mut AiData, n: usize) {
        let t = ad.npcs[n].target;
        self.ai_update_npc_place(ad, n);
        if t != ad.npcs[n].current && ad.places[t].parent != ad.npcs[n].current as i32 {
            self.ai_subtask_move(ad, n);
            return;
        }
        ad.npcs[n].order = OR_TRAIN;
        ad.npcs[n].or1 = ad.places[t].item.0 as i32;
        ad.npcs[n].or2 = 0;
    }

    /// C `task_fight(int n)` (`:1978-1994`) - despite the name, this sets
    /// `OR_GUARD` at the target place's own coordinates, not a fight
    /// order; a real C quirk (`task_fight` is only ever used to send a
    /// panicking party's non-eternal-guards to defend `ad->pplace`, and
    /// standing guard there is enough - `strategy_driver`'s `OR_GUARD`
    /// order execution already fights back via its own self-defense),
    /// kept verbatim rather than "fixed".
    pub fn ai_task_fight(&self, ad: &mut AiData, n: usize) {
        let t = ad.npcs[n].target;
        self.ai_update_npc_place(ad, n);
        if t != ad.npcs[n].current && ad.places[t].parent != ad.npcs[n].current as i32 {
            self.ai_subtask_move(ad, n);
            return;
        }
        ad.npcs[n].order = OR_GUARD;
        ad.npcs[n].or1 = i32::from(ad.places[t].x);
        ad.npcs[n].or2 = i32::from(ad.places[t].y);
    }

    /// C `assign_guards(int place, double count, int level, int
    /// ragnarok)` (`strategy.c:2111-2193`): decide whether enough guard
    /// strength ([`THREAT`](Self::ai_threat)-summed) is already assigned
    /// or free to meet `place`'s defense `count` at `level`, dispatching
    /// free guards or recalling excess/under-leveled ones. Returns
    /// whether an attack was (or already is) committed. See this
    /// module's doc comment for why this needs `&self` (live character
    /// reads for `THREAT`/HP-readiness, unlike every other function in
    /// this file).
    pub fn ai_assign_guards(
        &self,
        ad: &mut AiData,
        place: usize,
        count: f64,
        level: i32,
        ragnarok: bool,
    ) -> bool {
        let mut have = 0.0f64;
        let mut used = [false; AI_MAXGUARD];

        // Already-assigned guards: keep them if still needed/qualified,
        // otherwise recall them to standby.
        for n in 0..AI_MAXGUARD {
            let m = ad.guard[n];
            if m == -1 || m == ad.nagguard {
                continue;
            }
            let mu = m as usize;
            if ad.npcs[mu].ftarget == place as i32 {
                if (ad.npcs[mu].level + 5 < level || have > count) && !ragnarok {
                    ad.npcs[mu].target = 0;
                    ad.npcs[mu].ftarget = 0;
                    ad.npcs[mu].used = 0;
                } else {
                    have += self.ai_threat(ad.npcs[mu].cn);
                    used[n] = true;
                }
            }
        }

        // Free guards: pick up more until we have enough (or take
        // everyone, in a Ragnarok all-out defense).
        for n in 0..AI_MAXGUARD {
            if !ragnarok && have >= count {
                break;
            }
            let m = ad.guard[n];
            if m == -1 {
                continue;
            }
            let mu = m as usize;
            if ad.npcs[mu].used != 0 {
                continue;
            }
            let qualifies = ragnarok
                || (ad.nagguard != m
                    && ad.npcs[mu].level + 5 >= level
                    && self.ai_guard_ready(ad.npcs[mu].cn));
            if qualifies {
                have += self.ai_threat(ad.npcs[mu].cn);
                used[n] = true;
            }
        }

        if have > count || ragnarok {
            // We have enough (or must send everyone): dispatch every
            // picked guard to `place`.
            let mut sent = 0.0f64;
            for n in 0..AI_MAXGUARD {
                if !ragnarok && sent >= count {
                    break;
                }
                let m = ad.guard[n];
                if m == -1 || !used[n] {
                    continue;
                }
                let mu = m as usize;
                ad.npcs[mu].ftarget = place as i32;
                ad.npcs[mu].target = place;
                ad.npcs[mu].used = place as i32;
                sent += self.ai_threat(ad.npcs[mu].cn);
            }
            true
        } else {
            // Not enough: recall the already-assigned ones we counted
            // above back to standby.
            let mut recalled = 0.0f64;
            for n in 0..AI_MAXGUARD {
                if recalled >= count {
                    break;
                }
                let m = ad.guard[n];
                if m == -1 || !used[n] {
                    continue;
                }
                let mu = m as usize;
                // Only the *already-assigned* branch's `used[n]` entries
                // are eligible for recall here (C's own `use[n] == 2`
                // distinction, collapsed in this port since - unlike C -
                // nothing else reads `used[n]`'s "how" after this point;
                // a free-guard pickup that never got dispatched this call
                // simply stays on standby, matching C's `use[n] == 1`
                // entries being silently ignored by this final loop too).
                ad.npcs[mu].target = 0;
                ad.npcs[mu].ftarget = 0;
                ad.npcs[mu].used = 0;
                recalled += self.ai_threat(ad.npcs[mu].cn);
            }
            false
        }
    }

    /// C `#define THREAT(cn) ((double)ch[cn].level * ch[cn].level *
    /// ch[cn].level)` (`strategy.c:2109`) - deliberately reads the *live*
    /// character's level, not the cached [`AiNpc::level`] copy (see
    /// module doc comment). A missing/despawned character contributes no
    /// threat.
    fn ai_threat(&self, cn: CharacterId) -> f64 {
        self.characters
            .get(&cn)
            .map(|c| f64::from(c.level).powi(3))
            .unwrap_or(0.0)
    }

    /// C's free-guard eligibility HP check (`strategy.c:2152`): `ch[cn].hp
    /// >= ch[cn].value[0][V_HP] * POWERSCALE`.
    fn ai_guard_ready(&self, cn: CharacterId) -> bool {
        self.characters
            .get(&cn)
            .is_some_and(|c| c.hp >= character_value(c, CharacterValue::Hp) * POWERSCALE)
    }

    /// C `nag_attack(void)` (`strategy.c:2231-2267`): every 5 minutes,
    /// send the single lowest-level idle guard to harass the closest
    /// threatened place, if at least 2 guards are idle and some place is
    /// actually threatened (`threatcount != 0`).
    pub fn ai_nag_attack(&self, ad: &mut AiData) {
        let tick = self.tick.0 as i64;
        if tick - ad.lastnag < (TICKS_PER_SECOND as i64) * 60 * 5 {
            return;
        }

        let mut minlevel = 115;
        let mut cnt = 0;
        let mut guard = 0usize;
        for n in 0..AI_MAXGUARD {
            let m = ad.guard[n];
            if m != -1 && ad.npcs[m as usize].target == 0 {
                if minlevel > ad.npcs[m as usize].level {
                    minlevel = ad.npcs[m as usize].level;
                    guard = m as usize;
                }
                cnt += 1;
            }
        }

        let mut mindist = 99;
        let mut place = 0usize;
        for n in 0..ad.places.len() {
            if ad.places[n].threatcount != 0.0 && ad.places[n].dist < mindist {
                mindist = ad.places[n].dist;
                place = n;
            }
        }

        if cnt > 1 && mindist < 99 {
            ad.lastnag = tick;
            ad.nagplace = place as i32;
            ad.nagguard = guard as i32;
            ad.npcs[guard].target = place;
            ad.npcs[guard].used = place as i32;
        }
    }

    /// C `ai_init(int in, unsigned int code)` (`strategy.c:2269-2427`):
    /// build a fresh AI party's place graph and discover its currently-
    /// live `CDR_STRATEGY` roster. `code` is the [`STR_OWNER_AI_BASE`]-
    /// range owner code identifying which AI slot this is - `code -
    /// STR_OWNER_AI_BASE` indexes both [`AI_PRESETS`] (`ad->ppd =
    /// preset[...].ppd`, `:2289`) and, eventually, a `[AiData; MAX_AI]`
    /// per-slot registry no caller allocates yet (still-unported
    /// `ai_main` outer body, see this module's doc comment).
    ///
    /// Returns `None` if `code` doesn't resolve to a real [`AI_PRESETS`]
    /// row, or if `spawner_item` isn't a real, placed `IDR_STR_SPAWNER`
    /// item with a storage item directly north of it - C has no such
    /// guards (a malformed `in`/`code` would simply read garbage), but
    /// every real caller only ever reaches this with a spawner
    /// `World::ensure_strategy_areas_initialized` already discovered and
    /// a `code` an actual mission handed out.
    pub fn ai_init(&self, spawner_item: ItemId, code: u32) -> Option<AiData> {
        let preset_index = code.checked_sub(STR_OWNER_AI_BASE)? as usize;
        let preset = AI_PRESETS.get(preset_index)?;
        let mut ad = AiData::new(preset.to_strategy_ppd());

        let spawner = self.items.get(&spawner_item)?;
        let area_slot = *spawner.driver_data.get(8).unwrap_or(&0);
        let storage_item = self.str_spawner_storage_item(spawner_item)?;
        let storage = self.items.get(&storage_item)?;
        ad.storage_item = storage_item;
        let storage_area_slot = *storage.driver_data.get(8).unwrap_or(&0);

        // Place 0 is always the party's own storage (`:2294-2303`).
        let mut storage_place =
            AiPlace::new(AiPlaceType::Storage, storage_item, storage.x, storage.y);
        storage_place.dist = 0;
        ad.places.push(storage_place);

        // Discover every mine/depot/(possibly-enemy) storage sharing this
        // spawner's area slot (`:2305-2355`), in ascending item-index
        // order for determinism (`self.items` is an unordered `HashMap` -
        // same precedent as `ensure_strategy_areas_initialized`'s own doc
        // comment).
        let mut item_ids: Vec<ItemId> = self
            .items
            .iter()
            .filter(|(_, item)| !item.flags.is_empty())
            .map(|(id, _)| *id)
            .collect();
        item_ids.sort_by_key(|id| id.0);

        for item_id in item_ids {
            let item = &self.items[&item_id];
            if *item.driver_data.get(8).unwrap_or(&0) != area_slot {
                continue;
            }
            match item.driver {
                IDR_STR_DEPOT => {
                    ad.places
                        .push(AiPlace::new(AiPlaceType::Depot, item_id, item.x, item.y));
                }
                IDR_STR_MINE => {
                    ad.places
                        .push(AiPlace::new(AiPlaceType::Mine, item_id, item.x, item.y));
                }
                IDR_STR_STORAGE if item_id != storage_item => {
                    let mut place = AiPlace::new(AiPlaceType::Storage, item_id, item.x, item.y);
                    place.enemy_possible = true;
                    if *item.driver_data.get(8).unwrap_or(&0) == storage_area_slot {
                        ad.partner.push(item_id);
                    }
                    ad.places.push(place);
                }
                _ => {}
            }
        }

        // Breadth-first depth/parent search over the place graph
        // (`:2357-2377`): repeatedly extend from every place at the
        // current depth to any not-yet-connected place within range and
        // reachable by `pathfinder`.
        for cdepth in 0..AI_MAXDISTANCE {
            for n in 0..ad.places.len() {
                if ad.places[n].dist != cdepth {
                    continue;
                }
                for i in 0..ad.places.len() {
                    if ad.places[i].dist != -1 {
                        continue;
                    }
                    let (nx, ny) = (i32::from(ad.places[n].x), i32::from(ad.places[n].y));
                    let (ix, iy) = (i32::from(ad.places[i].x), i32::from(ad.places[i].y));
                    if (ix - nx).abs() < 20
                        && (iy - ny).abs() < 20
                        && (ix - nx).abs() + (iy - ny).abs() < 25
                        && pathfinder(
                            &self.map,
                            ad.places[i].x as usize,
                            ad.places[i].y as usize,
                            ad.places[n].x as usize,
                            ad.places[n].y as usize,
                            0,
                            Some(200),
                        )
                        .direction
                        .is_some()
                    {
                        ad.places[i].dist = cdepth + 1;
                        ad.places[i].parent = n as i32;
                    }
                }
            }
        }
        // C's "check for map errors" `xlog` loop (`:2379-2385`) is pure
        // logging - no persisted-log sink in this port, same precedent as
        // every other bare `xlog` call already documented elsewhere.

        // Propagate `enemy_possible` up the parent chain from every
        // enemy-storage place (`:2387-2395`).
        for n in 0..ad.places.len() {
            if ad.places[n].enemy_possible {
                let mut m = n as i32;
                while m != -1 {
                    ad.places[m as usize].enemy_possible = true;
                    m = ad.places[m as usize].parent;
                }
            }
        }

        // Discover every live `CDR_STRATEGY` NPC already belonging to
        // this party (`:2397-2426`), registering each via
        // [`AiData::register_npc`]. C: `ch[n].group`/`code` are plain
        // `int`s that can theoretically hold any AI code; the Rust
        // `Character::group` field is narrowed to `u16` (see its own doc
        // comment), so an AI-range `code` (>= `STR_OWNER_AI_BASE`) can
        // never actually match a real character's `group` here - the
        // exact same pre-existing, documented gap already noted by
        // `World::str_did_party_lose`'s own doc comment, not a new one:
        // harmless in practice since no code path can spawn an AI-owned
        // worker yet (`ai_main`'s own worker-spawning tail, `:2644-2672`,
        // is still unported) - see [`AiData::register_npc`]'s own tests
        // for coverage of the classification logic itself, independent
        // of this filter's real-world reachability.
        let mut npc_ids: Vec<CharacterId> = self
            .characters
            .iter()
            .filter(|(_, c)| c.driver == CDR_STRATEGY && u32::from(c.group) == code)
            .map(|(id, _)| *id)
            .collect();
        npc_ids.sort_by_key(|id| id.0);

        for cn in npc_ids {
            let character = &self.characters[&cn];
            let (order, or1, or2) = match character.driver_state.as_ref() {
                Some(CharacterDriverState::StrategyWorker(data)) => {
                    strategy_worker_order_to_raw(data.order)
                }
                _ => (OR_NONE, 0, 0),
            };
            let has_exp = matches!(
                character.driver_state.as_ref(),
                Some(CharacterDriverState::StrategyWorker(data)) if data.exp != 0
            );

            ad.register_npc(
                cn,
                character.x,
                character.y,
                character.level as i32,
                order,
                or1,
                or2,
                has_exp,
            );
        }

        Some(ad)
    }
}

/// C `struct strategy_data.order`/`or1`/`or2` (`strategy.c:100-113`)
/// read back out of the typed [`StrategyWorkerOrder`] a live worker
/// carries - the inverse of the (unwritten, since no code path needs it
/// yet) conversion the other direction, needed only by
/// [`World::ai_init`]'s roster-discovery scan copying `dat->order`/
/// `or1`/`or2` into a fresh [`AiNpc`] entry (`:2403-2405`).
fn strategy_worker_order_to_raw(order: StrategyWorkerOrder) -> (i32, i32, i32) {
    match order {
        StrategyWorkerOrder::None => (OR_NONE, 0, 0),
        StrategyWorkerOrder::Mine {
            mine_item,
            depot_item,
        } => (OR_MINE, mine_item.0 as i32, depot_item.0 as i32),
        StrategyWorkerOrder::Follow { leader } => (OR_FOLLOW, leader.0 as i32, 0),
        StrategyWorkerOrder::Guard { x, y } => (OR_GUARD, i32::from(x), i32::from(y)),
        StrategyWorkerOrder::Fighter { leader } => (OR_FIGHTER, leader.0 as i32, 0),
        StrategyWorkerOrder::Take { depot_item, leader } => {
            (OR_TAKE, depot_item.0 as i32, leader.0 as i32)
        }
        StrategyWorkerOrder::Transfer { from_item, to_item } => {
            (OR_TRANSFER, from_item.0 as i32, to_item.0 as i32)
        }
        StrategyWorkerOrder::Train { storage_item } => (OR_TRAIN, storage_item.0 as i32, 0),
        StrategyWorkerOrder::EternalGuard { x, y } => (OR_ETERNALGUARD, i32::from(x), i32::from(y)),
    }
}
