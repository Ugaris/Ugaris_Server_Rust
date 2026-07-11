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
//! Sixteenth slice done: [`AiData::assign_tasks_to_workers`]
//! (`strategy.c:2674-2796`) - the panic/non-panic "assign tasks to
//! workers" loop, the core of `ai_main`'s per-tick planning: panic sends
//! every non-eternal-guard NPC to fight at `pplace`; otherwise each NPC
//! keeps its current job if it's still productive/safe, gets promoted to
//! (or recalled from) elite-guard duty via `wantguardcnt`, gets
//! redirected to a busier parent place, or falls back to searching for
//! the nearest depot to take/mine-or-transfer place with spare capacity,
//! else goes idle. New [`AiData::ragnarok`]/[`AiData::nogoldleft`]
//! committed fields (read here as the *previous* tick's values, per C's
//! own commit-at-the-very-end ordering) and a new
//! [`AiPlaceRefreshResult::mindist`] field (previously silently discarded
//! by [`World::ai_refresh_places`]) support this. See this method's own
//! doc comment for the one deviation (`ap[-1]` OOB read in C, treated as
//! "no parent threat" here).
//!
//! Seventeenth slice done: the final per-npc task-dispatch `switch`
//! (`strategy.c:2932-2972`) is now ported as
//! [`World::ai_dispatch_tasks`] (`crate::world::strategy_ai_tasks`) -
//! dispatches every roster NPC to its already-ported `task_*` function
//! by [`AiTask`] (the [`AiTask::EGuard`] train-vs-idle-vs-guard nested
//! `if` kept verbatim), then writes the resulting raw `order`/`or1`/
//! `or2` back onto the live worker's typed `StrategyWorkerOrder` via a
//! new `raw_to_strategy_worker_order` (the inverse of this file's
//! existing `strategy_worker_order_to_raw`), auto-vivifying driver state
//! same as [`World::ai_task_idle`].
//!
//! Eighteenth slice done: split this file into the pure `AiData`/
//! `AiPlace`/`AiNpc` types file plus the sibling
//! `crate::world::strategy_ai_tasks` carrying every `impl World` method
//! over them (see the file-size note below), then ported `ai_main`'s
//! "create new workers" loop's eligibility/spend half (`:2644-2672`) -
//! [`AiData::register_new_worker`] plus `World::ai_wants_more_workers`/
//! `World::ai_plan_worker_spawn` (`crate::world::strategy_ai_tasks`) -
//! the `NPCPRICE`-deduction half; the actual `ZoneLoader`-needing
//! character-creation tail is deliberately deferred until a live
//! `ai_main` call site exists to call it, avoiding a dead-code function
//! in the `ugaris-server` binary crate.
//!
//! Nineteenth slice done: [`World::ai_threat_and_worklevel_tick`]
//! (`crate::world::strategy_ai_tasks`, `strategy.c:2798-2916`) - the
//! "find places with too little workers"/threat-list maintenance
//! (expire/record/sort-via-`tcomp`/dispatch-via-[`World::
//! ai_assign_guards`]/truncate)/worklevel-adjustment tail that runs
//! right after [`AiData::assign_tasks_to_workers`]. New [`AiThreat`]
//! type plus [`AiData::threats`]/`lastchange` fields. See that method's
//! own doc comment for the full C-order breakdown and the two real
//! `tcomp` comparator bugs preserved verbatim (kept, not "fixed").
//!
//! REMAINING (tracked in `PORTING_TODO.md`, left `[~]` on purpose):
//! `ai_main`'s own outer per-tick body still needs: the actual
//! `ZoneLoader`-needing tail of worker spawning (building a real
//! character from [`World::ai_plan_worker_spawn`]'s plan, same split as
//! the player-triggered spawner), and the "place eternal guards" block
//! (`:2892-2911`, needs a still-unported `create_eguard`, which itself
//! needs `ZoneLoader`). Also still open: actually assembling all of
//! these ported pieces (plus [`World::ai_refresh_places`]/
//! [`World::ai_update_npc_list`]/[`AiData::assign_tasks_to_workers`]/
//! [`World::ai_threat_and_worklevel_tick`]/[`World::ai_dispatch_tasks`]/
//! [`World::ai_nag_attack`], the last of which still isn't called from
//! anywhere in C order either) into one real `ai_main` call, threading
//! [`World::ai_update_npc_list`]'s real `cantrain` return value into
//! `ai_refresh_places` instead of that function's own cached-level
//! stand-in - deferred until a live spawn/tick call site for an
//! AI-controlled party exists to actually call it (`ai_init`/`ai_main`
//! are both still not called from anywhere outside tests).
//!
//! File-size note: `World`'s own methods over `AiData` (the
//! `task_*` order-resolution functions, place-graph navigation,
//! defense allocation/nag-attack, `ai_init`, `ai_refresh_places`,
//! `ai_dispatch_tasks`, worker-spawn planning, and
//! `ai_threat_and_worklevel_tick`) live in the sibling
//! `crate::world::strategy_ai_tasks` file, split out once this file
//! crossed ~1,900 lines.

use super::*;
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
/// C `#define WORKERPLATIN 200` (`:2107`): a place "has spare work" once
/// its smoothed [`AiPlace::platin`] exceeds this many currency units per
/// assigned worker - the threshold [`AiData::assign_tasks_to_workers`]
/// uses to decide whether a place needs (or has too many) workers.
pub const WORKERPLATIN: i32 = 200;

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

/// C `struct ai_threat` (`strategy.c:1740-1745`) - one active/expiring
/// threatened-place record [`World::ai_threat_and_worklevel_tick`]'s
/// threat-handling block tracks across ticks (populated from
/// [`AiPlace::threatcount`]/`threatlevel`/`threatnlevel`, not computed
/// fresh here). `place == 0` is C's own "slot empty" sentinel
/// (`ad->at[m].place` used as a bare truthiness test throughout
/// `strategy.c`) - preserved verbatim including the real C quirk this
/// creates: place index `0` is *also* the party's real storage place, so
/// a genuine threat recorded against storage itself is indistinguishable
/// from an empty slot everywhere this sentinel is tested (expiry, the
/// existing-entry search, `tcomp`, and the "reduce" loop's `if
/// (ad->at[m].place)` guard all silently treat a storage threat as "no
/// entry"). Not "fixed" - see [`World::ai_threat_and_worklevel_tick`]'s
/// own doc comment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AiThreat {
    pub place: i32,
    pub level: i32,
    pub count: f64,
    pub ticker: i64,
}

impl Default for AiThreat {
    /// C's zero-initialized `struct ai_threat` (the whole `at[MAXTHREAT]`
    /// array starts this way, and expiry re-zeros just `place`, matching
    /// C's own `ad->at[m].place = 0;`, `:2814` - every other field stays
    /// stale in both C and here).
    fn default() -> Self {
        Self {
            place: 0,
            level: 0,
            count: 0.0,
            ticker: 0,
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
    /// C's `mindist` local, initialized `99` (`:2459`): BFS distance of
    /// the closest un-threatened place with gold - committed into
    /// `ad->pdist` (already ported, `ad.pdist.min(mindist)`) but *also*
    /// read directly by the still-unported rest of `ai_main` (the
    /// task-assignment switch, the "find places with too little
    /// workers"/threat-handling block); callers thread this straight into
    /// [`AiData::assign_tasks_to_workers`] rather than recomputing it.
    pub mindist: i32,
}

/// C's file-static `struct ai_data ai_data[MAX_AI], *ad` (`strategy.c:
/// 1748-1787`) - one AI-controlled battleground party's full brain state.
#[derive(Debug, Clone)]
pub struct AiData {
    pub storage_item: ItemId,
    pub worklevel: i32,
    pub places: Vec<AiPlace>,
    pub npcs: Vec<AiNpc>,
    /// C `int max_at; struct ai_threat at[MAXTHREAT];` (`:1776`) - `Vec`-
    /// backed like [`Self::places`]/[`Self::npcs`] (no fixed `MAXTHREAT =
    /// 256` capacity), with C's `max_at` bound replaced by `Vec::len`:
    /// [`World::ai_threat_and_worklevel_tick`]'s own "reduce" step
    /// `Vec::truncate`s this exactly where C shrinks `max_at`, which has
    /// the same observable effect (slots beyond the new bound become
    /// unreachable in both ports) even though C physically keeps the
    /// stale memory around and this port actually drops it.
    pub threats: Vec<AiThreat>,
    /// C `int lastchange` (`:1758`): tick of the last `worklevel`
    /// increase/decrease, gating how often
    /// [`World::ai_threat_and_worklevel_tick`] is allowed to adjust it
    /// again.
    pub lastchange: i64,
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
    /// C `int ragnarok` (`:1774`, the struct field - not to be confused
    /// with `ai_main`'s own same-named local): the *previous* tick's
    /// committed "no economy left, all-out defense" flag. C only commits
    /// [`World::ai_refresh_places`]'s freshly recomputed
    /// [`AiPlaceRefreshResult::ragnarok`] into this field at the very end
    /// of `ai_main` (`:2926`, still unported - see this module's doc
    /// comment), so [`AiData::assign_tasks_to_workers`] deliberately reads
    /// this stale, previous-tick value instead. Defaults `false`, matching
    /// C's zero-initialized global `ai_data[]` before the first `ai_main`
    /// call ever commits a real value.
    pub ragnarok: bool,
    /// C `int nogoldleft` (`:1774`): same previous-tick-committed
    /// semantics as [`Self::ragnarok`].
    pub nogoldleft: bool,
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
            threats: Vec::new(),
            lastchange: 0,
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
            ragnarok: false,
            nogoldleft: false,
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

    /// C `ai_main`'s panic/non-panic "assign tasks to workers" loop
    /// (`strategy.c:2674-2796`): decide every already-registered NPC's
    /// next [`AiTask`]/target for this tick, called after
    /// [`Self::update_free_npc_count`] (which supplies `panic`/`pplace`)
    /// with `mindist` from [`World::ai_refresh_places`]'s
    /// [`AiPlaceRefreshResult`]. Reads [`Self::ragnarok`]/
    /// [`Self::nogoldleft`] as the *previous* tick's committed values -
    /// see those fields' own doc comments. Stops right before C's "find
    /// places with too little workers"/threat-handling/"place eternal
    /// guards" tail (`:2798-2924`, still unported - needs new `at[]`/
    /// `max_at`/`lastchange` fields this port doesn't carry yet).
    ///
    /// Deviation: C's `i = ad->ap[m].parent;` can be `-1` (`m` is
    /// storage, whose own parent is `-1`) and the very next line
    /// unconditionally reads `ad->ap[i].threat` - an out-of-bounds read
    /// in C itself (harmless there only because `ai_place ap[]` sits
    /// inside a larger struct, so `ap[-1]` reads some other field's
    /// garbage rather than crashing). Rust can't reproduce an OOB read
    /// safely, so this port treats a `-1` parent as contributing no
    /// threat - the same "no parent means nothing to check" convention
    /// every other already-ported parent-chain walk in this module
    /// already uses (e.g. [`World::ai_refresh_places`]'s own `if
    /// ad.places[n].parent != -1` guards).
    pub fn assign_tasks_to_workers(&mut self, mindist: i32) {
        if self.panic {
            // C `:2674-2684`.
            let pplace = self.pplace as usize;
            for n in 0..self.npcs.len() {
                if self.npcs[n].task != AiTask::EGuard && self.npcs[n].task != AiTask::Ignore {
                    self.npcs[n].task = AiTask::Fight;
                    if self.npcs[n].used != -1 {
                        self.remove_worker(n);
                    }
                }
                self.npcs[n].target = pplace;
            }
            return;
        }

        // C `:2686-2796`.
        for n in 0..self.npcs.len() {
            if self.npcs[n].task == AiTask::EGuard
                && self.wantguardcnt(self.npc_cnt) < self.gcnt
                && !self.nogoldleft
                && !self.ragnarok
            {
                self.remove_guard(n);
            }

            // never touch elite guards
            if self.npcs[n].task == AiTask::EGuard {
                continue;
            }
            // we may not touch eternal guards
            if self.npcs[n].task == AiTask::Ignore {
                continue;
            }

            if (self.wantguardcnt(self.npc_cnt) > self.gcnt || self.nogoldleft)
                && self.gcnt < AI_MAXGUARD as i32
            {
                if self.npcs[n].used != -1 {
                    self.remove_worker(n);
                }
                self.npcs[n].task = AiTask::EGuard;
                self.npcs[n].target = 0;
                self.add_guard(n);
                continue;
            }

            let m = self.npcs[n].target;
            let i = self.places[m].parent;
            // See this method's own doc comment: `i == -1` (m is storage)
            // is treated as "no parent threat" rather than an OOB read.
            let parent_threat = i >= 0 && self.places[i as usize].threat != 0;

            if self.npcs[n].used != -1
                && self.npcs[n].platin != 0
                && (self.npcs[n].task == AiTask::Transfer || self.npcs[n].task == AiTask::Mine)
                && self.places[m].wcnt <= self.worklevel
                && self.places[m].threat == 0
                && self.places[m].dist <= mindist
            {
                continue;
            }

            if self.npcs[n].used != -1
                && self.npcs[n].task == AiTask::Take
                && !self.places[m].owned
                && self.places[m].threat == 0
                && self.places[m].dist <= mindist
            {
                continue;
            }

            if self.places[m].threat != 0 || parent_threat || self.places[m].dist > mindist {
                if self.npcs[n].used != -1 {
                    self.remove_worker(n);
                }
            } else if i > 0
                && self.places[m].wcnt > self.places[i as usize].wcnt
                && self.places[i as usize].platin
                    > (self.places[i as usize].wcnt + 1) * WORKERPLATIN
                && self.places[i as usize].wcnt < self.worklevel
            {
                if self.npcs[n].used != -1 {
                    self.remove_worker(n);
                }
                self.add_worker(AiTask::Transfer, n, i as usize);
                continue;
            } else if self.npcs[n].used != -1
                && self.npcs[n].task == AiTask::Transfer
                && self.places[m].platin != 0
                && self.places[m].wcnt <= self.worklevel
            {
                continue;
            } else if self.npcs[n].used != -1
                && self.npcs[n].task == AiTask::Mine
                && self.places[m].platin != 0
                && self.places[m].wcnt <= self.worklevel
            {
                continue;
            }

            // find nearest unowned, unthreatened, empty depot
            let mut bm = 0usize;
            let mut bd = 99i32;
            let mut bdiff = 0i32;
            for mm in 1..self.places.len() {
                let diff = (i32::from(self.npcs[n].x) - i32::from(self.places[mm].x)).abs()
                    + (i32::from(self.npcs[n].y) - i32::from(self.places[mm].y)).abs();
                if self.places[mm].place_type == AiPlaceType::Depot
                    && !self.places[mm].owned
                    && self.places[mm].threat == 0
                    && self.places[mm].wcnt == 0
                    && (bd > self.places[mm].dist || (bd == self.places[mm].dist && diff < bdiff))
                {
                    bm = mm;
                    bd = self.places[mm].dist;
                    bdiff = diff;
                }
            }
            if bm != 0 && bd <= mindist {
                self.remove_worker(n);
                self.add_worker(AiTask::Take, n, bm);
                continue;
            }

            // find nearest not-fully-worked place with spare platin
            let mut bm = 0usize;
            let mut bd = 99i32;
            let mut bdiff = 0i32;
            for mm in 1..self.places.len() {
                let diff = (i32::from(self.npcs[n].x) - i32::from(self.places[mm].x)).abs()
                    + (i32::from(self.npcs[n].y) - i32::from(self.places[mm].y)).abs();
                if self.places[mm].threat == 0
                    && self.places[mm].platin > self.places[mm].wcnt * WORKERPLATIN
                    && self.places[mm].wcnt < self.worklevel
                    && (bd > self.places[mm].dist || (bd == self.places[mm].dist && diff < bdiff))
                {
                    bm = mm;
                    bd = self.places[mm].dist;
                    bdiff = diff;
                }
            }
            if bm != 0 && bd <= mindist {
                self.remove_worker(n);
                if self.places[bm].place_type == AiPlaceType::Mine {
                    self.add_worker(AiTask::Mine, n, bm);
                } else {
                    self.add_worker(AiTask::Transfer, n, bm);
                }
                continue;
            }

            if self.npcs[n].used != -1 {
                self.remove_worker(n);
            }
            self.npcs[n].task = AiTask::Idle;
            self.npcs[n].target = 0;
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

    /// C's "add new npc to list" tail of `ai_main`'s "create new workers"
    /// loop (`strategy.c:2661-2669`), once a fresh worker character
    /// already exists (`ugaris-server` builds it via
    /// [`World::ai_plan_worker_spawn`]'s plan - same split as
    /// [`Self::register_npc`]/[`World::ai_init`]). Reuses the first
    /// empty roster slot (`cn.is_none()`, a hole [`World::
    /// ai_update_npc_list`] can leave mid-`Vec`) exactly like C's own
    /// linear `for (n = 0; n < MAXNPC; n++) if (!ad->an[n].cn) { ...;
    /// break; }` scan, falling back to a fresh slot if none is free
    /// (matching C's implicit "MAXNPC is large enough, this never
    /// actually falls through" assumption for this `Vec`-backed,
    /// unbounded roster). `level` is left at C's own not-set-here `0`
    /// (every other field this loop iteration doesn't explicitly write -
    /// `platin`/`or1`/`or2`/`walktype`/`ftarget` - is likewise left at
    /// [`AiNpc::new`]'s defaults, matching every field C's own loop body
    /// explicitly assigns: `order = 0`/`task = T_IDLE`/`target = 0`/
    /// `current = 0`/`used = -1`).
    pub fn register_new_worker(&mut self, cn: CharacterId, x: u16, y: u16) -> usize {
        let fresh = AiNpc::new(cn, x, y, 0);
        if let Some(slot) = self.npcs.iter().position(|npc| npc.cn.is_none()) {
            self.npcs[slot] = fresh;
            slot
        } else {
            self.npcs.push(fresh);
            self.npcs.len() - 1
        }
    }

    /// C's "add new npc to list" tail of `ai_main`'s "place eternal
    /// guards" block (`strategy.c:2899-2916`), once a fresh eternal-guard
    /// character already exists (`ugaris-server` builds it via
    /// [`World::ai_plan_eguard_spawn`]'s plan - same split as
    /// [`Self::register_new_worker`]). Reuses the first empty roster slot
    /// exactly like [`Self::register_new_worker`]'s own linear scan, but
    /// pre-seeds `order`/`or1`/`or2`/`task`/`target`/`current`/`used` to
    /// `place` *before* calling [`Self::add_etguard`] (matching C's own
    /// field-write order, `:2903-2911`, right before `add_etguard(i)` at
    /// `:2915`) - since the guard was just dropped at that very place's
    /// coordinates, `add_etguard`'s own `update_npc_place` call is a
    /// same-place no-op, matching C exactly. Also bumps
    /// [`AiData::etguardcnt`] (C's own `ad->etguardcnt++;`, `:2916`).
    pub fn register_new_eguard(&mut self, cn: CharacterId, x: u16, y: u16, place: usize) -> usize {
        let mut fresh = AiNpc::new(cn, x, y, 0);
        fresh.order = OR_ETERNALGUARD;
        fresh.or1 = i32::from(x);
        fresh.or2 = i32::from(y);
        fresh.task = AiTask::Ignore;
        fresh.target = place;
        fresh.current = place;
        fresh.used = place as i32;

        let slot = if let Some(slot) = self.npcs.iter().position(|npc| npc.cn.is_none()) {
            self.npcs[slot] = fresh;
            slot
        } else {
            self.npcs.push(fresh);
            self.npcs.len() - 1
        };

        self.add_etguard(slot);
        self.etguardcnt += 1;
        slot
    }
}
