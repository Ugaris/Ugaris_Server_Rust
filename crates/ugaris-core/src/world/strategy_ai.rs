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
//! Third slice done: [`World::ai_refresh_places`] (`strategy.c:2505-
//! 2630`) - `ai_main`'s per-place worker/threat refresh loop (owned/
//! platin bookkeeping, the enemy-presence scan that populates each
//! [`AiPlace`]'s `threat`/`threatlevel`/`threatcount`/`threatncount`/
//! `threatnlevel`, threat propagation up/down the parent chain, `ad->
//! panic`/`pplace`/`pdist`), plus the following "project threats to
//! neighboring places" loop (`:2620-2630`). C's sector-grid scan
//! (`getfirst_char_sector`/`sec_next`, stepping by 8 within a
//! computed +-12 box) is replaced with a plain linear scan over
//! `self.characters` filtered by the same final `abs(...) < 10`
//! distance check - same observable result, matching the "no sector
//! index in this codebase" precedent already used elsewhere (e.g.
//! `world/npc/bank.rs`). C's `seen[MAXCHARS]` de-dupe array (shared
//! across every place in one `ai_main` call, not reset per place - a
//! character only ever contributes threat to the first place, in `n`
//! order, whose scan region contains it) is a `HashSet<CharacterId>`
//! local to the function with the same single-shared-scope lifetime.
//! `cantrain` (`:2452,2475-2477`, normally derived by the still-unported
//! "update npc list" loop from each live NPC's *current* level) is
//! instead derived here from each [`AiNpc`]'s cached `level`/`task`
//! fields - a best-effort stand-in that goes stale between roster
//! refreshes until that loop is ported (see REMAINING below).
//! `ragnarok`/`nogoldleft` are returned via [`AiPlaceRefreshResult`]
//! rather than written straight to `ad` fields: C only commits them to
//! `ad->ragnarok`/`ad->nogoldleft` at the very end of `ai_main`
//! (`:2926-2927`), *after* the still-unported worker-task-assignment/
//! threat-handling/nag-attack blocks that read the *previous* tick's
//! committed values mid-function - writing them early here would corrupt
//! that read-before-write ordering for whatever future slice ports the
//! rest of `ai_main` and wires the real commit.
//!
//! Fourteenth slice done: [`AiData::update_guard_list`]/[`AiData::
//! update_nag_guard`]/[`AiData::update_place_worker_and_eguard_counts`]/
//! [`AiData::update_free_npc_count`] (`strategy.c:2484-2500,2509-2520,
//! 2531-2539,2632-2642`) - the remaining pure roster-bookkeeping
//! refreshes from `ai_main`'s outer body that need no live-character/
//! item access (unlike [`World::ai_refresh_places`]'s own per-place
//! threat scan, which already covers the rest of that same per-place
//! `for` loop).
//!
//! Fifteenth slice done: [`World::ai_update_npc_list`] (`strategy.c:
//! 2461-2482`) - the "update npc list" NPC refresh itself, the one piece
//! of this per-tick refresh the previous slice's own doc comment had
//! flagged as not mapping cleanly onto the `Vec`-backed roster. Resolved
//! by widening [`AiNpc::cn`] to `Option<CharacterId>` (C's `an[n].cn = 0`
//! "slot emptied" sentinel) rather than attempting index-preserving `Vec`
//! removal - every other piece of `AiData`/`AiPlace` that stores plain
//! NPC-array indices (`worker[]`/`eguard`/`guard[]`/`nagguard`) keeps
//! working unchanged, since the slot itself never moves, only its `cn`
//! goes `None`. This also finally makes [`AiData::update_nag_guard`]'s
//! `!ad->an[i].cn` branch reachable (previously documented as dead code
//! in this port). [`World::ai_threat`]/[`World::ai_guard_ready`] widened
//! to accept `Option<CharacterId>` accordingly (a `None` contributes
//! nothing, same as an id that fails to resolve - no behavior change for
//! any existing caller, since every extant [`AiNpc::cn`] was always
//! `Some` before this slice).
//!
//! REMAINING (tracked in `PORTING_TODO.md`, left `[~]` on purpose):
//! `ai_main`'s own outer per-tick body still needs: worker spawning
//! (`:2644-2672`), the panic/non-panic task-assignment switch (`:2674-
//! 2924`, calls the already-ported `task_*` functions plus
//! `wantguardcnt`/`assign_guards`/`remove_free_guards`/`nag_attack`, also
//! all already ported), the final per-npc task-dispatch `switch` (`:2932-
//! 2972`) - plus `create_eguard` (`:2987-3040`, needs `ZoneLoader`) and
//! the "place eternal guards" tail that calls it (`:2892-2903`). Also
//! still open: actually assembling all of these ported pieces (plus
//! [`World::ai_refresh_places`]/[`World::ai_nag_attack`]/[`World::
//! ai_update_npc_list`]) into one real `ai_main` call in the exact C
//! order, threading [`World::ai_update_npc_list`]'s real `cantrain`
//! return value into `ai_refresh_places` instead of that function's own
//! cached-level stand-in - deferred until a live spawn/tick call site for
//! an AI-controlled party exists to actually call it (`ai_init`/
//! `ai_main` are both still `pub fn` with no caller outside tests).

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
    /// `None` is C's `an[n].cn = 0` "slot emptied" sentinel, written by
    /// [`World::ai_update_npc_list`] once the underlying character no
    /// longer exists. Every other field is left stale when this happens
    /// (matching C exactly - `ai_main`'s "update npc list" pass only
    /// ever touches `cn` itself in the removal branch, `strategy.c:
    /// 2464`), which is why every reader of this field elsewhere in this
    /// module (`ai_threat`/`ai_guard_ready`) treats a missing character
    /// as contributing nothing rather than panicking.
    pub cn: Option<CharacterId>,
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
            cn: Some(cn),
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
    /// C `int platin` (`:1720`): the AI's own smoothed running estimate
    /// of this place's currency (half of last tick's value plus the
    /// building item's current real balance) - a separate tracking
    /// variable from the item's own actual [`str_item_gold`] balance,
    /// refreshed by [`World::ai_refresh_places`].
    pub platin: i32,
    /// C `int threat` (`:1730`): this place's own smoothed threat score
    /// (halved each tick, then built back up from nearby enemy presence
    /// plus a share of its parent's threat), refreshed by
    /// [`World::ai_refresh_places`].
    pub threat: i32,
    /// C `int threatlevel` (`:1731`): highest enemy level seen at this
    /// place this tick (or last tick's, while `threat` is still nonzero).
    pub threatlevel: i32,
    /// C `int threatnlevel` (`:1732`): highest threat level seen at a
    /// *neighboring* place, projected by [`World::ai_refresh_places`]'s
    /// "project threats to neighboring places" pass.
    pub threatnlevel: i32,
    /// C `double threatcount` (`:1733`) - populated by
    /// [`World::ai_refresh_places`]'s enemy-presence scan.
    pub threatcount: f64,
    /// C `double threatncount` (`:1734`): this place's own threatcount
    /// contribution to/from its neighbors, projected by
    /// [`World::ai_refresh_places`]'s "project threats to neighboring
    /// places" pass.
    pub threatncount: f64,
    /// C `int owned` (`:1736`): does this party's own `code` currently
    /// own this place's building item (`it[in].drdata[0] == code`)?
    pub owned: bool,
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
            platin: 0,
            threat: 0,
            threatlevel: 0,
            threatnlevel: 0,
            threatcount: 0.0,
            threatncount: 0.0,
            owned: false,
            enemy_possible: false,
        }
    }
}

/// The `ragnarok`/`nogoldleft` locals `ai_main` computes across its whole
/// per-place refresh loop (`strategy.c:2452-2617`) but only ever commits
/// to `ad->ragnarok`/`ad->nogoldleft` at the very end of the function
/// (`:2926-2927`) - see [`World::ai_refresh_places`]'s own doc comment
/// for why this port returns them instead of writing them straight to
/// [`AiData`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiPlaceRefreshResult {
    /// C's `ragnarok` local, initialized `1` (`:2452`): true unless some
    /// non-storage place still has un-threatened gold left (or storage
    /// itself has enough spare gold and an under-max-level eternal guard
    /// to train) - "no economy left to grow, throw everyone at the
    /// enemy" all-out-defense trigger for [`World::ai_assign_guards`].
    pub ragnarok: bool,
    /// C's `nogoldleft` local, initialized `1` (`:2452`): true unless
    /// some non-storage place still has un-threatened gold left.
    pub nogoldleft: bool,
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
    /// C `int npc_cnt` (`:1762`): live, non-eternal-guard NPC count,
    /// refreshed by [`AiData::update_free_npc_count`].
    pub npc_cnt: i32,
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
    /// C `int pdist` (`:1764`): the panic-defense distance threshold -
    /// any threatened place within this BFS depth of storage triggers
    /// `panic`. C `ai_init` seeds this to `3` (`:2290`); it can only
    /// shrink afterward (`ai_main`'s `ad->pdist = min(ad->pdist,
    /// mindist)`, ported as [`World::ai_refresh_places`]).
    pub pdist: i32,
    /// C `int panic` (`:1764`): is an enemy currently within `pdist` of
    /// storage? Refreshed every call by [`World::ai_refresh_places`].
    pub panic: bool,
    /// C `int pplace` (`:1764`): which place index triggered `panic`
    /// this tick, `-1` if none. Refreshed every call by
    /// [`World::ai_refresh_places`].
    pub pplace: i32,
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
            npc_cnt: 0,
            guard: [-1; AI_MAXGUARD],
            gcnt: 0,
            etguardcnt: 0,
            lastnag: 0,
            nagplace: 0,
            nagguard: -1,
            partner: Vec::new(),
            ppd,
            pdist: 3,
            panic: false,
            pplace: -1,
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

    /// C `ai_main`'s "update guard list" pass (`strategy.c:2484-2494`):
    /// recount `gcnt` from the `guard[]` slots that are still real elite
    /// guards on standby (`task == T_EGUARD && used == -1`, stamping them
    /// `used = 0` in the process), evicting any slot whose NPC no longer
    /// qualifies (was reassigned away from [`AiTask::EGuard`] by the
    /// still-unported task-assignment switch, or already claimed by a
    /// place this tick via [`Self::update_place_worker_and_eguard_counts`]).
    pub fn update_guard_list(&mut self) {
        self.gcnt = 0;
        for m in 0..AI_MAXGUARD {
            let i = self.guard[m];
            if i == -1 {
                continue;
            }
            let iu = i as usize;
            if self.npcs[iu].task == AiTask::EGuard && self.npcs[iu].used == -1 {
                self.gcnt += 1;
                self.npcs[iu].used = 0;
            } else {
                self.guard[m] = -1;
            }
        }
    }

    /// C `ai_main`'s "update nag guard" pass (`strategy.c:2496-2500`):
    /// clear [`Self::nagguard`] once its NPC is no longer a qualifying
    /// elite guard still assigned to [`Self::nagplace`], the nag has run
    /// for more than 90 seconds, or the NPC's slot was emptied outright
    /// (`!ad->an[i].cn` - reachable now that [`World::ai_update_npc_list`]
    /// actually clears [`AiNpc::cn`] to `None`).
    pub fn update_nag_guard(&mut self, tick: i64) {
        let i = self.nagguard;
        if i == -1 {
            return;
        }
        let iu = i as usize;
        let stale = tick - self.lastnag > (TICKS_PER_SECOND as i64) * 90;
        if self.npcs[iu].cn.is_none()
            || self.npcs[iu].task != AiTask::EGuard
            || self.npcs[iu].target as i32 != self.nagplace
            || stale
        {
            self.nagguard = -1;
        }
    }

    /// C `ai_main`'s "update worker/etguard counts on places" per-place
    /// half (`strategy.c:2509-2520,2531-2539`): recount each place's
    /// `wcnt` from its `worker[]` slots (keeping only workers still
    /// actually targeting this place and not yet claimed by another
    /// place this tick, stamping `used = n` in the process, dropping
    /// stale slots back to `-1`), and refresh its `eguard` slot the same
    /// way. Companion to [`World::ai_refresh_places`]'s own per-place
    /// loop (called immediately before it in C, `:2509` is the first line
    /// of the very same `for` loop `ai_refresh_places` ports the rest
    /// of) - kept as a separate, independently testable pass here since
    /// it needs no live-character/item access at all, unlike the rest of
    /// that loop.
    pub fn update_place_worker_and_eguard_counts(&mut self) {
        for n in 0..self.places.len() {
            self.places[n].wcnt = 0;
            for m in 0..AI_MAXWORKER {
                let i = self.places[n].worker[m];
                if i == -1 {
                    continue;
                }
                let iu = i as usize;
                if self.npcs[iu].target == n && self.npcs[iu].used == -1 {
                    self.places[n].wcnt += 1;
                    self.npcs[iu].used = n as i32;
                } else {
                    self.places[n].worker[m] = -1;
                }
            }

            let eguard = self.places[n].eguard;
            if eguard != -1 {
                let iu = eguard as usize;
                if self.npcs[iu].target == n && self.npcs[iu].used == -1 {
                    self.npcs[iu].used = n as i32;
                } else {
                    self.places[n].eguard = -1;
                }
            }
        }
    }

    /// C `ai_main`'s "update free NPC count" pass (`strategy.c:2632-
    /// 2642`): recompute `npc_cnt`/`free_workers` from every live,
    /// non-eternal-guard NPC's current `used` state.
    pub fn update_free_npc_count(&mut self) {
        self.npc_cnt = 0;
        self.free_workers = 0;
        for npc in &self.npcs {
            if npc.task != AiTask::Ignore {
                self.npc_cnt += 1;
                if npc.used == -1 {
                    self.free_workers += 1;
                }
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

        let Some(worker_id) = ad.npcs[n].cn else {
            return;
        };
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
    /// module doc comment). A missing/despawned character (`None`,
    /// [`AiNpc::cn`]'s "slot emptied" sentinel, or a stale id that
    /// somehow no longer resolves) contributes no threat.
    fn ai_threat(&self, cn: Option<CharacterId>) -> f64 {
        cn.and_then(|cn| self.characters.get(&cn))
            .map(|c| f64::from(c.level).powi(3))
            .unwrap_or(0.0)
    }

    /// C's free-guard eligibility HP check (`strategy.c:2152`): `ch[cn].hp
    /// >= ch[cn].value[0][V_HP] * POWERSCALE`.
    fn ai_guard_ready(&self, cn: Option<CharacterId>) -> bool {
        cn.and_then(|cn| self.characters.get(&cn))
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

    /// C `ai_main`'s "update npc list" pass (`strategy.c:2461-2482`), the
    /// very first thing the real per-tick body does after `ai_init`: for
    /// every roster entry still pointing at a live character, refresh its
    /// cached `x`/`y`/`level`/`platin` (the latter from the character's
    /// own [`StrategyWorkerDriverData::platin`], C's `set_data(...,
    /// DRD_STRATEGYDRIVER, ...)`) and reset `used` to "free" (`-1`) for
    /// this tick's later passes ([`AiData::update_guard_list`]/
    /// [`AiData::update_place_worker_and_eguard_counts`]/etc.) to
    /// re-derive; otherwise (the character no longer exists) empty the
    /// slot (`an[n].cn = 0`, ported as [`AiNpc::cn`] going `None` - see
    /// its own doc comment for why every other field is deliberately
    /// left stale, matching C exactly). C's extra `ch[cn].serial !=
    /// cserial` staleness re-check has no equivalent here: a Rust
    /// [`CharacterId`] is already a stable, never-reused identity (same
    /// precedent as every other `cserial`-dropping doc comment in this
    /// module), so existence in [`World::characters`] alone is the only
    /// signal needed. Returns C's `cantrain` local (`:2438,2472-2474`):
    /// true if any live eternal guard is still below `ppd.max_level` -
    /// the real, non-stale replacement for [`World::ai_refresh_places`]'s
    /// own documented cached-level stand-in (that function isn't wired to
    /// call this one yet, since nothing assembles a real `ai_main` call
    /// order across both methods; a future slice doing that assembly
    /// should thread this return value through instead).
    pub fn ai_update_npc_list(&self, ad: &mut AiData) -> bool {
        let mut cantrain = false;
        for n in 0..ad.npcs.len() {
            let Some(cn) = ad.npcs[n].cn else {
                continue;
            };
            let Some(character) = self.characters.get(&cn) else {
                ad.npcs[n].cn = None;
                continue;
            };
            ad.npcs[n].x = character.x;
            ad.npcs[n].y = character.y;
            ad.npcs[n].level = character.level as i32;
            ad.npcs[n].used = -1;
            if let Some(CharacterDriverState::StrategyWorker(data)) =
                character.driver_state.as_ref()
            {
                ad.npcs[n].platin = data.platin;
            }
            if ad.npcs[n].task == AiTask::EGuard && ad.npcs[n].level < ad.ppd.max_level {
                cantrain = true;
            }
        }
        cantrain
    }

    /// C `ai_main`'s per-place worker/threat refresh loop (`strategy.c:
    /// 2505-2630`): reset `panic`/`pplace`, then for every place update
    /// `owned`/`platin` from the building item's live state, decay/
    /// rebuild `threat`/`threatlevel`/`threatcount` from nearby enemy
    /// `CDR_STRATEGY` presence (propagating threat up to the parent and
    /// back down), track the closest un-threatened place with gold
    /// (`mindist`, committed into `ad.pdist`), and compute whether the
    /// party still has any economy left (`ragnarok`/`nogoldleft`,
    /// returned rather than committed - see this module's doc comment).
    /// Finally projects each place's `threatcount`/`threatlevel` onto its
    /// immediate neighbors' `threatncount`/`threatnlevel` (`:2620-2630`).
    /// See this module's doc comment for the sector-scan-to-linear-scan
    /// and `cantrain`-staleness deviations.
    pub fn ai_refresh_places(&self, ad: &mut AiData, code: u32) -> AiPlaceRefreshResult {
        // C `:2475-2477`: normally re-derived from each live NPC's
        // *current* level by the still-unported "update npc list" loop;
        // here derived from each `AiNpc`'s cached level (see module doc
        // comment).
        let cantrain = ad
            .npcs
            .iter()
            .any(|npc| npc.task == AiTask::EGuard && npc.level < ad.ppd.max_level);

        ad.panic = false;
        ad.pplace = -1;
        let mut seen: std::collections::HashSet<CharacterId> = std::collections::HashSet::new();

        let mut mindist = 99;
        let mut ragnarok = true;
        let mut nogoldleft = true;

        for n in 0..ad.places.len() {
            let item_id = ad.places[n].item;
            let (drdata4, drdata0) = match self.items.get(&item_id) {
                Some(item) => (str_item_gold(item), str_item_owner(item)),
                None => (0, 0),
            };
            ad.places[n].platin = ad.places[n].platin / 2 + drdata4 as i32;
            ad.places[n].owned = drdata0 == code;

            ad.places[n].threat /= 2;
            ad.places[n].threatcount = 0.0;
            ad.places[n].threatncount = 0.0;
            ad.places[n].threatnlevel = 0;
            if ad.places[n].threat == 0 {
                ad.places[n].threatlevel = 0;
            }

            // C's sector-grid scan (`getfirst_char_sector`/`sec_next`
            // over a +-12 box stepped by 8) is replaced with a plain
            // linear scan filtered by the same final `abs(...) < 10`
            // check - see module doc comment.
            let (px, py) = (i32::from(ad.places[n].x), i32::from(ad.places[n].y));
            for character in self.characters.values() {
                if character.driver != CDR_STRATEGY || u32::from(character.group) == code {
                    continue;
                }
                let (cx, cy) = (i32::from(character.x), i32::from(character.y));
                if (px - cx).abs() >= 10 || (py - cy).abs() >= 10 {
                    continue;
                }
                // C's `seen[MAXCHARS]`: shared across every place in this
                // call, not reset per place.
                if !seen.insert(character.id) {
                    continue;
                }

                ad.places[n].threatcount += self.ai_threat(Some(character.id)) * 1.25;
                ad.places[n].threatlevel = ad.places[n].threatlevel.max(character.level as i32);
                ad.places[n].threat += 100 + ad.places[n].threatlevel;
                if ad.places[n].dist <= ad.pdist {
                    ad.panic = true;
                    ad.pplace = n as i32;
                }
            }

            // move threat up the parent list
            if ad.places[n].parent != -1 {
                let parent = ad.places[n].parent as usize;
                ad.places[n].threat += ad.places[parent].threat / 2;
            }
            // move threat one down the parent list
            if ad.places[n].threatcount != 0.0 && ad.places[n].parent != -1 {
                let parent = ad.places[n].parent as usize;
                ad.places[parent].threat = ad.places[n].threat / 2;
            }

            if drdata4 > 0 && ad.places[n].wcnt > 0 {
                let mut m = ad.places[n].parent;
                while m != -1 && ad.places[m as usize].wcnt > 0 {
                    let mu = m as usize;
                    ad.places[mu].platin = ad.places[mu].platin.max(50);
                    m = ad.places[mu].parent;
                }
            }

            // find distance to closest mine
            if ad.places[n].place_type == AiPlaceType::Mine
                && ad.places[n].platin != 0
                && ad.places[n].threat == 0
                && ad.places[n].dist < mindist
            {
                mindist = ad.places[n].dist;
            }
            if ad.places[n].platin != 0 && ad.places[n].threat == 0 {
                if n > 0 {
                    nogoldleft = false;
                }
                if n == 0 {
                    if ad.places[n].platin / 2 > ad.ppd.max_level && cantrain {
                        ragnarok = false;
                    }
                } else {
                    ragnarok = false;
                }
            }
        }
        ad.pdist = ad.pdist.min(mindist);

        // project threats to neighboring places
        for n in 0..ad.places.len() {
            let parent = ad.places[n].parent;
            if ad.places[n].threatcount != 0.0 && parent != -1 {
                let p = parent as usize;
                ad.places[p].threatncount += ad.places[n].threatcount;
                ad.places[p].threatnlevel = ad.places[p].threatnlevel.max(ad.places[n].threatlevel);
            }
            if parent != -1 {
                let p = parent as usize;
                if ad.places[p].threatcount != 0.0 {
                    let (pcount, plevel) = (ad.places[p].threatcount, ad.places[p].threatlevel);
                    ad.places[n].threatncount += pcount;
                    ad.places[n].threatnlevel = ad.places[n].threatnlevel.max(plevel);
                }
            }
        }

        AiPlaceRefreshResult {
            ragnarok,
            nogoldleft,
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
