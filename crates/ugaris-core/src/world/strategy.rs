//! Areas 23/24 (`src/area/23_24/strategy.c`, 3,632 lines) real-time-
//! strategy minigame: two 16-area-slot battlegrounds where a player (or
//! an AI opponent picked from [`AI_PRESETS`]) races to command NPC
//! workers/guards/fighters that mine platinum out of a `IDR_STR_MINE`,
//! haul it through a `IDR_STR_DEPOT` to their `IDR_STR_STORAGE`, and use
//! the accumulated storage gold to spawn more `IDR_STR_SPAWNER` troops,
//! while the opposing side tries to do the same first (or wipe the other
//! side out) - see [`MISSIONS`] for the mission ladder and
//! `src/area/23_24/strategy.c:713-1122` (`strategy_driver`) for the full
//! worker order/state machine this module does not port yet.
//!
//! This is a genuinely large subsystem (~50 functions, including a
//! 538-line AI-opponent driver `ai_main`) that the `PORTING_TODO.md` task
//! itself calls out as needing a plan before diving in. This first slice
//! ports only the parts that have no dependency on `World`'s map/
//! character/item state and are therefore safely testable in isolation:
//!
//! - The order-type constants (`OR_*`, `:91-98`) and the two static
//!   content tables the rest of the system indexes into: [`MISSIONS`]
//!   (C `struct mission mission[]`, `:214-239`) and [`AI_PRESETS`] (C
//!   `struct ai_preset preset[64]`, `:162-200` - only the first 24 of 64
//!   declared slots are ever initialized in C; the rest are implicitly
//!   all-zero/unused padding no mission's `enemy[]` index ever reaches,
//!   so this port only carries the 24 real presets).
//! - The player-upgrade economy: [`str_exp_cost`]/[`str_increment`]/
//!   [`str_raise`] (`:3041-3198`), the pure math behind the boss NPC's
//!   "raise a stat with strategy exp" dialogue command (not itself
//!   ported yet - see `crate::player::StrategyPpd`'s module doc comment).
//!
//! This slice additionally ports the `struct str_area area[MAX_STR_AREA]`
//! runtime registry + `init_areas` (`:154-155`/`:241-269`): a `World`-level
//! scan of every `IDR_STR_*` item on load that discovers which
//! `IDR_STR_SPAWNER`/`IDR_STR_STORAGE`/`IDR_STR_MINE`/`IDR_STR_DEPOT` items
//! belong to which of the 16 battleground slots (`it[].drdata[8]`),
//! mirroring `world::pents`'s slot bookkeeping - see
//! [`StrategyAreaRegistry`] and [`World::ensure_strategy_areas_initialized`].
//!
//! Second slice ports the per-tick mission-lifecycle driver itself:
//! [`World::str_ticker`] (`str_ticker`, `:456-506`), [`World::
//! str_did_party_lose`] (`did_party_lose`, `:382-413`), [`World::
//! str_remove_party`] (`remove_party`, `:271-333`), [`World::
//! str_close_area`] (`close_area`, `:417-426`), [`World::str_init_mission`]
//! (`init_mission`, `:337-379`), and the `reward_winner` (`:428-454`) split
//! into a pure [`apply_strategy_mission_win`] (over `StrategyPpd`) plus
//! [`World::str_reward_winner`]'s character-lookup/event-queue half - see
//! [`StrategyRewardEvent`]/[`World::drain_pending_strategy_rewards`] for
//! why (`World` can't reach session-owned `PlayerRuntime::strategy`, same
//! split as `world::military`'s `MilitaryMasterEvent`). `IDR_STR_TICKER`
//! now dispatches to a real `ItemDriverOutcome::StrTicker` (`item_driver::
//! area23_24::str_ticker_driver`) instead of a no-op, applied by `World::
//! apply_item_driver_outcome` the same way `ItemDriverOutcome::LqTicker`
//! calls `discover_lq_doors_once`. Not reachable in live gameplay yet -
//! nothing calls [`World::str_init_mission`] (C's own only caller,
//! `special_driver`'s "go" mission-join command, isn't ported), so no
//! spawner ever gets a real owner code and every slot's `used` scan is
//! currently a no-op; ported ahead of that caller anyway since it's
//! self-contained and testable in isolation, same precedent as several
//! `area8`/`lq` slices before their own live call sites landed. Like
//! `IDR_LQ_TICKER`, this port does not (yet) prime the very first
//! `schedule_item_driver_timer` call for a real `IDR_STR_TICKER` zone
//! item - the reschedule-on-fire machinery is in place and tested
//! directly, but nothing currently seeds its first tick from zone load.
//!
//! REMAINING (tracked in `PORTING_TODO.md`, left `[~]` on purpose): the
//! worker character driver (`strategy_driver`, order assignment via NPC
//! speech, `setname`/`restplace`), the `mine`/`storage`/`depot`/`spawner`/
//! `nosnow` item drivers (currently dispatched as C-parity no-ops, see
//! `item_driver::dispatch`), the full AI-opponent driver (`ai_init`/
//! `ai_main`, `:2277-2994`), the mission queue (`queue_*`, `:3200-3276`),
//! and the boss NPC dialogue driver (`strategy_boss`, `:1414-1616`, plus
//! `special_driver`'s player-facing `#`-style commands, `:3278-3632`).

use super::*;
use crate::player::StrategyPpd;

/// C `#define OR_MINE 1` etc. (`strategy.c:91-98`). `0` (no `#define`,
/// C's zero-initialized default) means "no order" (idle/freshly spawned).
pub const OR_NONE: i32 = 0;
pub const OR_MINE: i32 = 1;
pub const OR_FOLLOW: i32 = 2;
pub const OR_GUARD: i32 = 3;
pub const OR_FIGHTER: i32 = 4;
pub const OR_TAKE: i32 = 5;
pub const OR_TRANSFER: i32 = 6;
pub const OR_TRAIN: i32 = 7;
pub const OR_ETERNALGUARD: i32 = 8;

/// C `#define NPCPRICE 300` (`strategy.c:86`): storage gold cost to spawn
/// one worker at a `IDR_STR_SPAWNER` (`did_party_lose`/`spawner_sub`).
pub const NPCPRICE: i32 = 300;
/// C `#define TRAINMULTI 3` (`strategy.c:88`).
pub const TRAINMULTI: i32 = 3;
/// C `#define MAXMISSIONTRY 3` (`strategy.c:89`).
pub const MAXMISSIONTRY: i32 = 3;
/// C `#define MAXMISSION 64` (`strategy.c:115`): size of
/// `strategy_ppd::solve_cnt[]`. See [`crate::player::StrategyPpd`].
pub const STRATEGY_MAXMISSION: usize = 64;
/// C `#define MAX_STR_AREA 16` (`strategy.c:140`): number of independent
/// battleground slots (areas 23 and 24 each host several).
pub const MAX_STR_AREA: usize = 16;
/// C `#define MAXQUEUE 4` (`strategy.c:144`): per-area mission entry
/// queue depth.
pub const MAXQUEUE: usize = 4;
/// C `#define MAX_STR_ITEM 256` (`strategy.c:141`): per-area capacity of
/// `struct str_area::item[]`. Not enforced as a hard cap here (`Vec`
/// grows as needed) - kept only for documentation/parity with the C
/// `#define`.
pub const MAX_STR_ITEM: usize = 256;
/// C `#define MAX_STR_SPAWN 8` (`strategy.c:142`): per-area capacity of
/// `struct str_area::spawn[]`. Same "not enforced" note as
/// [`MAX_STR_ITEM`].
pub const MAX_STR_SPAWN: usize = 8;

/// C `struct str_area` (`strategy.c:146-152`): per-battleground-slot
/// item/spawn/queue bookkeeping the rest of the strategy subsystem
/// (worker driver, item drivers, `str_ticker`) indexes into via
/// `it[].drdata[8]`.
#[derive(Debug, Clone, Default)]
pub struct StrArea {
    pub used: bool,
    pub busy: bool,
    /// C `int spawn[MAX_STR_SPAWN]`/`int max_spawn`: every
    /// `IDR_STR_SPAWNER` item registered to this slot, in
    /// [`World::ensure_strategy_areas_initialized`]'s discovery order.
    /// `spawn.len()` is C's `max_spawn`.
    pub spawn: Vec<ItemId>,
    /// C `int item[MAX_STR_ITEM]`/`int max_item`: every
    /// `IDR_STR_SPAWNER`/`IDR_STR_STORAGE`/`IDR_STR_MINE`/`IDR_STR_DEPOT`
    /// item registered to this slot - C's own fallthrough `switch`
    /// (`init_areas`, no `break` after the `IDR_STR_SPAWNER` case) means
    /// spawners land in *both* `spawn` and `item`. `item.len()` is C's
    /// `max_item`.
    pub item: Vec<ItemId>,
    /// C `int q_playerID[MAXQUEUE], q_playercn[MAXQUEUE]` - the mission
    /// entry queue. Not populated by this port yet (`queue_*`,
    /// `strategy.c:3200-3276`, still unported - see this module's doc
    /// comment); carried here only so the struct shape matches C's.
    pub q_player_id: [u32; MAXQUEUE],
    pub q_player_cn: [Option<CharacterId>; MAXQUEUE],
}

/// C's file-static `struct str_area area[MAX_STR_AREA]`/`int area_init`
/// (`strategy.c:154-155`).
#[derive(Debug, Clone, Default)]
pub struct StrategyAreaRegistry {
    /// Always exactly [`MAX_STR_AREA`] entries once
    /// [`World::ensure_strategy_areas_initialized`] has run; empty before
    /// that (mirrors C's `area_init` guard).
    pub areas: Vec<StrArea>,
    initialized: bool,
}

/// C `#define TRAINPRICE(cn) ((ch[cn].level - 45) * 10)` (`strategy.c:87`).
pub fn train_price(level: i32) -> i32 {
    (level - 45) * 10
}

/// C `struct mission` (`strategy.c:202-212`), one entry per
/// [`MISSIONS`] row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissionDef {
    pub name: &'static str,
    /// Area code (1..=16) as used in `it[].drdata[8]` area-slot ids -
    /// C's own comment on the struct field.
    pub area: i32,
    /// Starting platinum in the mission's mine.
    pub mine_size: i32,
    /// Starting platinum in the mission's storage.
    pub storage_size: i32,
    /// [`AI_PRESETS`] indices for up to 4 AI-controlled enemy slots (0 =
    /// unused slot / player-controllable).
    pub enemy: [i32; 4],
    /// Which `strategy_ppd::solve_cnt[]` milestone slot this mission's
    /// win increments.
    pub set_solve: i32,
    /// `solve_cnt[need_solve]` must be nonzero to unlock this mission.
    pub need_solve: i32,
    /// Second unlock requirement (both must hold); mirrors `need_solve`
    /// for missions with two prerequisite chains.
    pub need_solve2: i32,
    /// Strategy exp awarded to `strategy_ppd::exp`/`boss_exp` on win.
    pub exp: i32,
}

/// C `struct mission mission[]` (`strategy.c:214-239`): the full Areas
/// 23/24 mission ladder, in table order (index = array position, not
/// `set_solve` - several missions share unlock chains). The two area-23
/// entry missions ("A-1"/"A-2") need no prerequisite; "J 2P" (a 2-player
/// co-op mission, `exp: 0`/`set_solve: 0`) awards no strategy exp and
/// unlocks nothing downstream, matching C's own all-zero trailing
/// columns.
pub const MISSIONS: [MissionDef; 14] = [
    MissionDef {
        name: "A-1",
        area: 1,
        mine_size: 1000,
        storage_size: 600,
        enemy: [1, 0, 0, 0],
        set_solve: 1,
        need_solve: 0,
        need_solve2: 0,
        exp: 10,
    },
    MissionDef {
        name: "A-2",
        area: 2,
        mine_size: 1000,
        storage_size: 600,
        enemy: [2, 0, 0, 0],
        set_solve: 1,
        need_solve: 0,
        need_solve2: 0,
        exp: 10,
    },
    MissionDef {
        name: "B",
        area: 5,
        mine_size: 1000,
        storage_size: 600,
        enemy: [3, 0, 0, 0],
        set_solve: 2,
        need_solve: 1,
        need_solve2: 1,
        exp: 25,
    },
    MissionDef {
        name: "C",
        area: 4,
        mine_size: 1000,
        storage_size: 600,
        enemy: [4, 5, 0, 0],
        set_solve: 3,
        need_solve: 1,
        need_solve2: 1,
        exp: 25,
    },
    MissionDef {
        name: "D",
        area: 3,
        mine_size: 1500,
        storage_size: 600,
        enemy: [6, 7, 0, 0],
        set_solve: 4,
        need_solve: 2,
        need_solve2: 3,
        exp: 25,
    },
    MissionDef {
        name: "E",
        area: 5,
        mine_size: 2000,
        storage_size: 900,
        enemy: [8, 0, 0, 0],
        set_solve: 5,
        need_solve: 3,
        need_solve2: 4,
        exp: 25,
    },
    MissionDef {
        name: "F",
        area: 4,
        mine_size: 2000,
        storage_size: 900,
        enemy: [9, 10, 0, 0],
        set_solve: 6,
        need_solve: 4,
        need_solve2: 5,
        exp: 25,
    },
    MissionDef {
        name: "G",
        area: 3,
        mine_size: 2000,
        storage_size: 900,
        enemy: [11, 12, 0, 0],
        set_solve: 7,
        need_solve: 5,
        need_solve2: 6,
        exp: 25,
    },
    MissionDef {
        name: "H",
        area: 7,
        mine_size: 2000,
        storage_size: 900,
        enemy: [13, 14, 0, 0],
        set_solve: 8,
        need_solve: 6,
        need_solve2: 7,
        exp: 25,
    },
    MissionDef {
        name: "I",
        area: 8,
        mine_size: 2000,
        storage_size: 300,
        enemy: [15, 0, 0, 0],
        set_solve: 9,
        need_solve: 7,
        need_solve2: 8,
        exp: 25,
    },
    MissionDef {
        name: "J 2P",
        area: 9,
        mine_size: 2000,
        storage_size: 300,
        enemy: [16, 0, 0, 0],
        set_solve: 0,
        need_solve: 0,
        need_solve2: 0,
        exp: 0,
    },
    MissionDef {
        name: "K",
        area: 10,
        mine_size: 2000,
        storage_size: 900,
        enemy: [17, 18, 19, 0],
        set_solve: 10,
        need_solve: 8,
        need_solve2: 9,
        exp: 25,
    },
    MissionDef {
        name: "L",
        area: 11,
        mine_size: 2000,
        storage_size: 900,
        enemy: [20, 21, 0, 0],
        set_solve: 11,
        need_solve: 9,
        need_solve2: 10,
        exp: 25,
    },
    MissionDef {
        name: "Z",
        area: 3,
        mine_size: 2000,
        storage_size: 900,
        enemy: [22, 23, 0, 0],
        set_solve: 12,
        need_solve: 10,
        need_solve2: 11,
        exp: 50,
    },
];

/// C `struct ai_preset` (`strategy.c:157-160`), one entry per
/// [`AI_PRESETS`] row - the initial `strategy_ppd` upgrade levels an AI
/// opponent spawns with (never raised further by the still-unported
/// `ai_main`, unlike a real player).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiPreset {
    pub name: &'static str,
    pub max_worker: i32,
    pub max_level: i32,
    pub trainspeed: i32,
    pub income: i32,
    pub endurance: i32,
    pub warcry: i32,
    pub speed: i32,
    pub eguards: i32,
    pub eguardlvl: i32,
}

/// C `struct ai_preset preset[64]` (`strategy.c:162-200`): only the first
/// 24 slots (index 0 = the unused empty-name placeholder, 1..=23 = the
/// real AI opponents [`MISSIONS`]' `enemy[]` fields index into) are
/// initialized in C; the remaining 40 declared slots are implicit
/// all-zero padding never referenced by any mission and are not carried
/// here.
pub const AI_PRESETS: [AiPreset; 24] = [
    AiPreset {
        name: "",
        max_worker: 0,
        max_level: 0,
        trainspeed: 0,
        income: 0,
        endurance: 0,
        warcry: 0,
        speed: 0,
        eguards: 0,
        eguardlvl: 0,
    },
    AiPreset {
        name: "Zakath",
        max_worker: 4,
        max_level: 60,
        trainspeed: 1,
        income: 0,
        endurance: 0,
        warcry: 0,
        speed: 0,
        eguards: 1,
        eguardlvl: 55,
    },
    AiPreset {
        name: "Mazian",
        max_worker: 4,
        max_level: 60,
        trainspeed: 1,
        income: 0,
        endurance: 0,
        warcry: 0,
        speed: 0,
        eguards: 1,
        eguardlvl: 55,
    },
    AiPreset {
        name: "Durnroth",
        max_worker: 4,
        max_level: 60,
        trainspeed: 1,
        income: 0,
        endurance: 0,
        warcry: 5,
        speed: 0,
        eguards: 1,
        eguardlvl: 55,
    },
    AiPreset {
        name: "Saphira",
        max_worker: 8,
        max_level: 60,
        trainspeed: 1,
        income: 0,
        endurance: 0,
        warcry: 15,
        speed: 5,
        eguards: 1,
        eguardlvl: 65,
    },
    AiPreset {
        name: "Cleran",
        max_worker: 4,
        max_level: 60,
        trainspeed: 1,
        income: 0,
        endurance: 0,
        warcry: 5,
        speed: 5,
        eguards: 1,
        eguardlvl: 55,
    },
    AiPreset {
        name: "Dagdar",
        max_worker: 4,
        max_level: 65,
        trainspeed: 1,
        income: 0,
        endurance: 5,
        warcry: 15,
        speed: 5,
        eguards: 1,
        eguardlvl: 65,
    },
    AiPreset {
        name: "Karkarath",
        max_worker: 4,
        max_level: 65,
        trainspeed: 1,
        income: 0,
        endurance: 5,
        warcry: 15,
        speed: 5,
        eguards: 1,
        eguardlvl: 65,
    },
    AiPreset {
        name: "Vashini",
        max_worker: 8,
        max_level: 65,
        trainspeed: 2,
        income: 0,
        endurance: 10,
        warcry: 25,
        speed: 5,
        eguards: 1,
        eguardlvl: 65,
    },
    AiPreset {
        name: "Kurbatz",
        max_worker: 12,
        max_level: 70,
        trainspeed: 2,
        income: 0,
        endurance: 25,
        warcry: 40,
        speed: 15,
        eguards: 1,
        eguardlvl: 70,
    },
    AiPreset {
        name: "Kalim",
        max_worker: 6,
        max_level: 70,
        trainspeed: 2,
        income: 0,
        endurance: 10,
        warcry: 25,
        speed: 5,
        eguards: 1,
        eguardlvl: 65,
    },
    AiPreset {
        name: "Sumpfbatz",
        max_worker: 6,
        max_level: 70,
        trainspeed: 2,
        income: 0,
        endurance: 25,
        warcry: 25,
        speed: 15,
        eguards: 1,
        eguardlvl: 70,
    },
    AiPreset {
        name: "Umfrag",
        max_worker: 6,
        max_level: 70,
        trainspeed: 2,
        income: 0,
        endurance: 25,
        warcry: 40,
        speed: 15,
        eguards: 1,
        eguardlvl: 70,
    },
    AiPreset {
        name: "Sickan",
        max_worker: 8,
        max_level: 75,
        trainspeed: 3,
        income: 0,
        endurance: 30,
        warcry: 45,
        speed: 20,
        eguards: 1,
        eguardlvl: 75,
    },
    AiPreset {
        name: "Logasi",
        max_worker: 8,
        max_level: 75,
        trainspeed: 3,
        income: 0,
        endurance: 30,
        warcry: 45,
        speed: 20,
        eguards: 1,
        eguardlvl: 75,
    },
    AiPreset {
        name: "Sumso",
        max_worker: 20,
        max_level: 80,
        trainspeed: 4,
        income: 0,
        endurance: 35,
        warcry: 60,
        speed: 30,
        eguards: 1,
        eguardlvl: 90,
    },
    AiPreset {
        name: "Karka",
        max_worker: 4,
        max_level: 85,
        trainspeed: 4,
        income: 0,
        endurance: 40,
        warcry: 65,
        speed: 35,
        eguards: 1,
        eguardlvl: 95,
    },
    AiPreset {
        name: "Rungan",
        max_worker: 12,
        max_level: 85,
        trainspeed: 4,
        income: 0,
        endurance: 40,
        warcry: 65,
        speed: 35,
        eguards: 1,
        eguardlvl: 90,
    },
    AiPreset {
        name: "Kirlo",
        max_worker: 12,
        max_level: 85,
        trainspeed: 4,
        income: 0,
        endurance: 40,
        warcry: 65,
        speed: 35,
        eguards: 1,
        eguardlvl: 90,
    },
    AiPreset {
        name: "Surgao",
        max_worker: 12,
        max_level: 85,
        trainspeed: 4,
        income: 0,
        endurance: 40,
        warcry: 65,
        speed: 35,
        eguards: 1,
        eguardlvl: 90,
    },
    AiPreset {
        name: "Huwa",
        max_worker: 12,
        max_level: 90,
        trainspeed: 5,
        income: 0,
        endurance: 45,
        warcry: 70,
        speed: 45,
        eguards: 1,
        eguardlvl: 95,
    },
    AiPreset {
        name: "Losaki",
        max_worker: 12,
        max_level: 90,
        trainspeed: 5,
        income: 0,
        endurance: 45,
        warcry: 70,
        speed: 45,
        eguards: 1,
        eguardlvl: 95,
    },
    AiPreset {
        name: "Death",
        max_worker: 16,
        max_level: 115,
        trainspeed: 8,
        income: 20,
        endurance: 115,
        warcry: 115,
        speed: 115,
        eguards: 1,
        eguardlvl: 115,
    },
    AiPreset {
        name: "Despair",
        max_worker: 16,
        max_level: 115,
        trainspeed: 8,
        income: 20,
        endurance: 115,
        warcry: 115,
        speed: 115,
        eguards: 1,
        eguardlvl: 115,
    },
];

impl AiPreset {
    /// C `preset[...].ppd`'s aggregate-initializer literal
    /// (`strategy.c:162-200`): the nine upgrade-level fields
    /// [`World::ai_init`] stamps a fresh AI opponent's `struct
    /// ai_data::ppd` with (`:2289`). Every other [`StrategyPpd`] field
    /// (`exp`/`won_cnt`/etc.) stays at its zero [`Default`], matching
    /// C's own partial-initializer semantics (fields not named in the
    /// `{ ... }` literal are implicitly zeroed).
    pub fn to_strategy_ppd(&self) -> StrategyPpd {
        StrategyPpd {
            max_worker: self.max_worker,
            max_level: self.max_level,
            trainspeed: self.trainspeed,
            income: self.income,
            endurance: self.endurance,
            warcry: self.warcry,
            speed: self.speed,
            eguards: self.eguards,
            eguardlvl: self.eguardlvl,
            ..StrategyPpd::default()
        }
    }
}

/// C `str_exp_cost(struct strategy_ppd *ppd, int nr)` (`strategy.c:3041-
/// 3095`): the strategy-exp price to raise upgrade slot `nr` (1..=8) by
/// one increment, or `0` once that slot is already at its cap (`nr`
/// outside `1..=8` also returns `0`, C's `default:` branch).
pub fn str_exp_cost(ppd: &StrategyPpd, nr: i32) -> i32 {
    match nr {
        1 => {
            if ppd.income < 20 {
                25
            } else {
                0
            }
        }
        2 => {
            if ppd.max_level < 115 {
                4
            } else {
                0
            }
        }
        3 => {
            if ppd.max_worker < 16 {
                10
            } else {
                0
            }
        }
        4 => {
            if ppd.trainspeed < 8 {
                4
            } else {
                0
            }
        }
        5 => {
            if ppd.warcry < 115 {
                4
            } else {
                0
            }
        }
        6 => {
            if ppd.endurance < 115 {
                4
            } else {
                0
            }
        }
        7 => {
            if ppd.speed < 115 {
                6
            } else {
                0
            }
        }
        8 => {
            if ppd.eguardlvl < 115 {
                3
            } else {
                0
            }
        }
        _ => 0,
    }
}

/// C `str_increment(struct strategy_ppd *ppd, int nr)` (`strategy.c:3097-
/// 3150`): how much slot `nr` goes up by if raised right now (`0` if
/// [`str_exp_cost`] says it's already capped).
pub fn str_increment(ppd: &StrategyPpd, nr: i32) -> i32 {
    if str_exp_cost(ppd, nr) == 0 {
        return 0;
    }
    match nr {
        1 => 1,
        2 => 2,
        3 => 1,
        4 => 1,
        5 => 5,
        6 => 5,
        7 => 5,
        8 => 1,
        _ => 0,
    }
}

/// Outcome of [`str_raise`], for the caller to render C's exact
/// `log_char` text (`strategy.c:3156`/`3162`/`3196`) since `World`/
/// `StrategyPpd` have no character-log sink of their own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyRaiseOutcome {
    /// "You cannot raise this value any higher." - slot `nr` is already
    /// capped (or `nr` is out of the `1..=8` range).
    CannotRaiseHigher,
    /// "You cannot afford to raise this value." - `ppd.exp < cost`.
    CannotAfford { cost: i32 },
    /// "Done." - `ppd` has already been updated in place.
    Raised,
}

/// C `str_raise(int cn, struct strategy_ppd *ppd, int nr)` (`strategy.c:
/// 3152-3198`), minus the `cn`-addressed `log_char` calls (the caller
/// renders [`StrategyRaiseOutcome`] into text instead) and minus C's
/// unreachable `default:` "Please report bug #4371g" branch (`nr` is
/// already guaranteed `1..=8` by this point in C - `str_exp_cost` would
/// have returned `0` and short-circuited otherwise for any other `nr`,
/// so that branch can never actually run; omitted here rather than
/// carried as dead code).
pub fn str_raise(ppd: &mut StrategyPpd, nr: i32) -> StrategyRaiseOutcome {
    let cost = str_exp_cost(ppd, nr);
    if cost == 0 {
        return StrategyRaiseOutcome::CannotRaiseHigher;
    }
    if cost > ppd.exp {
        return StrategyRaiseOutcome::CannotAfford { cost };
    }

    let inc = str_increment(ppd, nr);
    match nr {
        1 => ppd.income = (ppd.income + inc).min(20),
        2 => ppd.max_level = (ppd.max_level + inc).min(115),
        3 => ppd.max_worker = (ppd.max_worker + inc).min(24),
        4 => ppd.trainspeed = (ppd.trainspeed + inc).min(8),
        5 => ppd.warcry = (ppd.warcry + inc).min(115),
        6 => ppd.endurance = (ppd.endurance + inc).min(115),
        7 => ppd.speed = (ppd.speed + inc).min(115),
        8 => ppd.eguardlvl = (ppd.eguardlvl + inc).min(115),
        _ => unreachable!("str_exp_cost already returned 0 for nr outside 1..=8"),
    }
    ppd.exp -= cost;
    StrategyRaiseOutcome::Raised
}

/// C's `0` drdata sentinel: no owner (a free player slot, or a
/// depot/storage/mine that's never been claimed).
pub const STR_OWNER_NONE: u32 = 0;
/// C's `0xfffff000` drdata sentinel: an AI-designated spawner slot with
/// no `ai_init` done yet (`init_mission`'s `>= 0xfffff000` check, and
/// `remove_party`'s reset target for a losing AI party).
pub const STR_OWNER_AI_UNASSIGNED: u32 = 0xfffff000;
/// C's `0xfffff001 + enemy[]` base for a fully-assigned AI owner code
/// (`init_mission`, `strategy.c:361`).
pub const STR_OWNER_AI_BASE: u32 = 0xfffff001;

/// C `*(unsigned int *)(it[in].drdata + 0)`: the 4-byte little-endian
/// "owner code" every `IDR_STR_SPAWNER`/`IDR_STR_STORAGE`/`IDR_STR_MINE`/
/// `IDR_STR_DEPOT` item keys its ownership by - either `0` (free), a
/// player's [`Character::serial`], or an
/// [`STR_OWNER_AI_UNASSIGNED`]/[`STR_OWNER_AI_BASE`]-range AI code.
pub fn str_item_owner(item: &Item) -> u32 {
    item.driver_data
        .get(0..4)
        .map(|bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        .unwrap_or(0)
}

/// Writer half of [`str_item_owner`].
pub fn set_str_item_owner(item: &mut Item, owner: u32) {
    if item.driver_data.len() < 4 {
        item.driver_data.resize(4, 0);
    }
    item.driver_data[0..4].copy_from_slice(&owner.to_le_bytes());
}

/// C `*(unsigned int *)(it[in].drdata + 4)`: the 4-byte little-endian
/// gold amount held by a `IDR_STR_STORAGE`/`IDR_STR_MINE`/`IDR_STR_DEPOT`
/// item.
pub fn str_item_gold(item: &Item) -> u32 {
    item.driver_data
        .get(4..8)
        .map(|bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        .unwrap_or(0)
}

/// Writer half of [`str_item_gold`].
pub fn set_str_item_gold(item: &mut Item, gold: u32) {
    if item.driver_data.len() < 8 {
        item.driver_data.resize(8, 0);
    }
    item.driver_data[4..8].copy_from_slice(&gold.to_le_bytes());
}

/// Outcome of [`apply_strategy_mission_win`], for the caller to render
/// C's exact `log_char` text (`reward_winner`, `strategy.c:436-448`)
/// since `StrategyPpd` has no character-log sink of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyWinOutcome {
    /// "Please report bug #443f" - `ppd.current_mission` was out of
    /// [`MISSIONS`]' range.
    BadMissionIndex,
    /// `mission[n].exp` was `0` (the "J 2P" co-op mission never awards
    /// anything) - no reward text beyond the caller's own unconditional
    /// "Congratulations, you won!"; `ppd.current_mission` is still reset
    /// to `0`.
    NoReward,
    /// Reward applied; `ppd` already updated.
    Rewarded { exp: i32 },
}

/// C `reward_winner`'s per-player `ppd` mutation half (`strategy.c:428-
/// 454`), minus the `getfirst_char`/`ch[m].ID == code` character lookup
/// (`World::str_reward_winner` does that instead, since `World` can't
/// reach session-owned `PlayerRuntime`) and minus the unconditional
/// "Congratulations, you won!" `log_char`, which the caller renders
/// alongside this outcome.
pub fn apply_strategy_mission_win(ppd: &mut StrategyPpd, mission_index: i32) -> StrategyWinOutcome {
    let Ok(idx) = usize::try_from(mission_index) else {
        return StrategyWinOutcome::BadMissionIndex;
    };
    let Some(mission) = MISSIONS.get(idx) else {
        return StrategyWinOutcome::BadMissionIndex;
    };

    let outcome = if mission.exp != 0 {
        ppd.won_cnt += 1;
        ppd.exp += mission.exp;
        ppd.boss_exp += mission.exp;
        ppd.eguards += 1;
        ppd.increment_solve_count(mission.set_solve as usize);
        StrategyWinOutcome::Rewarded { exp: mission.exp }
    } else {
        StrategyWinOutcome::NoReward
    };
    ppd.current_mission = 0;
    outcome
}

/// `World::str_reward_winner`'s queued event: `ugaris-server` applies
/// [`apply_strategy_mission_win`] against the real
/// `PlayerRuntime::strategy` and renders the resulting text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrategyRewardEvent {
    pub character_id: CharacterId,
}

impl World {
    /// C `init_areas()` (`strategy.c:241-269`), lazily triggered the same
    /// way `world::pents::ensure_pentagram_system_initialized` is - C's
    /// own trigger site is `str_ticker`'s `if (!area_init) { init_areas();
    /// area_init = 1; }` guard (`:465-468`, `str_ticker` itself not yet
    /// ported, see this module's doc comment), so calling this lazily on
    /// first use is equivalent to C's "first ticker tick" timing.
    ///
    /// Skips the trailing `xlog(...)` summary loop (`:264-268`) - pure
    /// logging, no observable state change.
    pub fn ensure_strategy_areas_initialized(&mut self) {
        if self.strategy_areas.initialized {
            return;
        }
        self.strategy_areas.initialized = true;
        self.strategy_areas.areas = vec![StrArea::default(); MAX_STR_AREA];

        // C iterates `for (n = 1; n < MAXITEM; n++)` in ascending
        // item-index order, which affects the `drdata[10]` spawn-slot
        // number each `IDR_STR_SPAWNER` item is assigned and the resulting
        // `spawn`/`item` push order. `self.items` is a `HashMap` with no
        // such ordering guarantee, so sort explicitly to match C's
        // deterministic discovery order.
        let mut item_ids: Vec<ItemId> = self
            .items
            .iter()
            .filter(|(_, item)| !item.flags.is_empty())
            .map(|(id, _)| *id)
            .collect();
        item_ids.sort_by_key(|id| id.0);

        for item_id in item_ids {
            let Some(item) = self.items.get(&item_id) else {
                continue;
            };
            // C `slot = it[n].drdata[8];` (a single byte, so always
            // 0..=255) indexes `area[MAX_STR_AREA]` with no bounds check
            // in C; real zone data always keeps it inside 0..16, but skip
            // rather than panic if it somehow doesn't.
            let slot = *item.driver_data.get(8).unwrap_or(&0) as usize;
            if slot >= MAX_STR_AREA {
                continue;
            }
            let driver = item.driver;

            match driver {
                IDR_STR_SPAWNER => {
                    // C's `switch` has no `break` after this case: it sets
                    // `drdata[10]` and pushes to `spawn[]`, then falls
                    // through into the `IDR_STR_STORAGE`/`IDR_STR_MINE`/
                    // `IDR_STR_DEPOT` case body below, so the spawner item
                    // also lands in `item[]` and sets `used`.
                    let spawn_slot_number = self.strategy_areas.areas[slot].spawn.len() as u8;
                    if let Some(item) = self.items.get_mut(&item_id) {
                        if item.driver_data.len() <= 10 {
                            item.driver_data.resize(11, 0);
                        }
                        item.driver_data[10] = spawn_slot_number;
                    }
                    let area = &mut self.strategy_areas.areas[slot];
                    area.spawn.push(item_id);
                    area.item.push(item_id);
                    area.used = true;
                }
                IDR_STR_STORAGE | IDR_STR_MINE | IDR_STR_DEPOT => {
                    let area = &mut self.strategy_areas.areas[slot];
                    area.item.push(item_id);
                    area.used = true;
                }
                _ => {}
            }
        }
    }

    /// C `spawner2storage(int in)` (`strategy.c:380`): the storage item
    /// always sits on the map tile directly north (`y - 1`) of its
    /// spawner - a fixed zone-layout convention, not runtime bookkeeping.
    pub fn str_spawner_storage_item(&self, spawner_id: ItemId) -> Option<ItemId> {
        let spawner = self.items.get(&spawner_id)?;
        let y = usize::from(spawner.y).checked_sub(1)?;
        let tile = self.map.tile(usize::from(spawner.x), y)?;
        (tile.item != 0).then_some(ItemId(tile.item))
    }

    /// C `did_party_lose(int spawn)` (`strategy.c:382-413`).
    ///
    /// C's `dat->order == OR_ETERNALGUARD` skip needs `DRD_STRATEGYDRIVER`
    /// (`struct strategy_data`, the still-unported worker character
    /// driver's per-NPC order state) - no code path can create such a
    /// character without the still-unported `strategy_driver`/
    /// `assign_guards`/`add_etguard`, so every live character currently
    /// satisfies C's "not an eternal guard" default and is scanned here,
    /// matching C's actual observable behavior today; revisit once
    /// eternal guards can exist.
    pub fn str_did_party_lose(&self, spawn: ItemId) -> bool {
        let Some(spawner) = self.items.get(&spawn) else {
            return true;
        };
        let code = str_item_owner(spawner);
        let mut lost = true;
        let mut no_player = code < STR_OWNER_AI_UNASSIGNED;

        if let Some(storage_id) = self.str_spawner_storage_item(spawn) {
            if let Some(storage) = self.items.get(&storage_id) {
                if str_item_gold(storage) >= NPCPRICE as u32 {
                    lost = false;
                }
            }
        }

        // C: `ch[m].group`/`.ID` are plain `int`s that can theoretically
        // hold any `code` value; the Rust `Character::group` field is
        // narrowed to `u16` (see its own doc comment), so an
        // AI-range `code` (>= 0x1000) can never match a real character's
        // `group` here - harmless in practice, since real spawned workers
        // are always given small `group` ids at spawn time by the
        // still-unported `strategy_driver`.
        for character in self.characters.values() {
            if u32::from(character.group) == code {
                lost = false;
            }
            if character.serial == code {
                no_player = false;
            }
        }

        if no_player {
            lost = true;
        }
        lost
    }

    /// C `remove_party(int code, char *msg)` (`strategy.c:271-333`):
    /// forcibly ends a strategy party - destroys every non-player
    /// character grouped under `code` (except the hardcoded "Cinciac"
    /// boss NPC safety check), teleports the actual player identified by
    /// `code` (`ch[cn].ID`, i.e. [`Character::serial`]) to one of 4
    /// hardcoded fallback tiles with an optional message, then frees
    /// every spawner/depot/storage item in the one battleground slot that
    /// had a spawner owned by `code`. Returns whether any such slot was
    /// found (C's own return value, used by the still-unported
    /// "surrender" command to report "You are not doing any mission." on
    /// `false`).
    pub fn str_remove_party(&mut self, code: u32, message: Option<&str>) -> bool {
        let character_ids: Vec<CharacterId> = self.characters.keys().copied().collect();
        for character_id in character_ids {
            let Some(character) = self.characters.get(&character_id) else {
                continue;
            };
            let group = u32::from(character.group);
            let serial = character.serial;
            let is_player = character.flags.contains(CharacterFlags::PLAYER);
            let name_is_cinciac = character.name == "Cinciac";

            let mut destroyed = false;
            if group == code {
                if is_player {
                    // C `elog("panic: about to destroy player %s!", ...)`:
                    // never expected to actually happen (real players are
                    // never given a strategy party `group`) - no `elog`
                    // sink exists in this port, so this stays a silent
                    // no-op rather than ever destroying a real player.
                } else if !name_is_cinciac {
                    self.remove_character(character_id);
                    destroyed = true;
                }
            }

            if !destroyed && serial == code {
                if !self.teleport_char_driver(character_id, 15, 15)
                    && !self.teleport_char_driver(character_id, 20, 15)
                    && !self.teleport_char_driver(character_id, 15, 20)
                {
                    self.teleport_char_driver(character_id, 20, 20);
                }
                if let Some(message) = message {
                    self.queue_system_text(character_id, message.to_string());
                }
            }
        }

        self.ensure_strategy_areas_initialized();
        let Some(area_index) = self.strategy_areas.areas.iter().position(|area| {
            area.spawn.iter().any(|&item_id| {
                self.items
                    .get(&item_id)
                    .is_some_and(|item| str_item_owner(item) == code)
            })
        }) else {
            return false;
        };

        let item_ids = self.strategy_areas.areas[area_index].item.clone();
        for item_id in item_ids {
            let Some(item) = self.items.get_mut(&item_id) else {
                continue;
            };
            if str_item_owner(item) != code {
                continue;
            }
            let slot = *item.driver_data.get(8).unwrap_or(&0);
            match item.driver {
                IDR_STR_SPAWNER => {
                    let reset_to = if code < STR_OWNER_AI_UNASSIGNED {
                        STR_OWNER_NONE
                    } else {
                        STR_OWNER_AI_UNASSIGNED
                    };
                    set_str_item_owner(item, reset_to);
                    item.name = format!("Spawner ({slot})");
                }
                IDR_STR_DEPOT => {
                    set_str_item_owner(item, STR_OWNER_NONE);
                    item.name = format!("Depot ({slot})");
                }
                IDR_STR_STORAGE => {
                    set_str_item_owner(item, STR_OWNER_NONE);
                    item.name = format!("Storage ({slot})");
                }
                _ => {}
            }
        }

        true
    }

    /// C `close_area(int n)` (`strategy.c:417-426`): force-removes every
    /// still-owned party from an area's spawners (cleanup once
    /// [`Self::str_ticker`] decides the slot's contest is over).
    pub fn str_close_area(&mut self, area_index: usize) {
        let Some(area) = self.strategy_areas.areas.get(area_index) else {
            return;
        };
        let spawn_ids = area.spawn.clone();
        for item_id in spawn_ids {
            let Some(item) = self.items.get(&item_id) else {
                continue;
            };
            let owner = str_item_owner(item);
            if owner != STR_OWNER_NONE && owner != STR_OWNER_AI_UNASSIGNED {
                self.str_remove_party(owner, None);
            }
        }
    }

    /// C `reward_winner(int code)`'s character-lookup half (`strategy.c:
    /// 428-433`): finds the live player whose [`Character::serial`]
    /// (`ch[m].ID`) matches `code` and queues a [`StrategyRewardEvent`]
    /// for `ugaris-server` to apply against the real
    /// `PlayerRuntime::strategy` via [`apply_strategy_mission_win`] - see
    /// [`Self::drain_pending_strategy_rewards`].
    pub fn str_reward_winner(&mut self, code: u32) {
        if let Some(character) = self.characters.values().find(|c| c.serial == code) {
            self.pending_strategy_rewards.push(StrategyRewardEvent {
                character_id: character.id,
            });
        }
    }

    /// Drains the queue [`Self::str_reward_winner`] fills.
    pub fn drain_pending_strategy_rewards(&mut self) -> Vec<StrategyRewardEvent> {
        self.pending_strategy_rewards.drain(..).collect()
    }

    /// C `init_mission(int n)` (`strategy.c:337-379`): resets a mission
    /// area's depot/storage/mine ownership and starting gold, and assigns
    /// each pre-designated-AI spawner slot an AI preset index from
    /// `mission[n].enemy[]` (or frees a player-designated slot to `0`).
    /// No live caller yet - C's own only caller is `special_driver`'s
    /// still-unported "go" mission-join command (see this module's doc
    /// comment).
    pub fn str_init_mission(&mut self, mission_index: usize) -> bool {
        let Some(mission) = MISSIONS.get(mission_index).copied() else {
            return false;
        };
        self.ensure_strategy_areas_initialized();
        let area_index = mission.area as usize;
        let Some(item_ids) = self
            .strategy_areas
            .areas
            .get(area_index)
            .map(|area| area.item.clone())
        else {
            return false;
        };

        let mut enemy_slot = 0usize;
        for item_id in item_ids {
            let Some(item) = self.items.get_mut(&item_id) else {
                continue;
            };
            let slot = *item.driver_data.get(8).unwrap_or(&0);
            match item.driver {
                IDR_STR_DEPOT => {
                    set_str_item_owner(item, STR_OWNER_NONE);
                    set_str_item_gold(item, 0);
                    item.name = format!("Depot ({slot})");
                }
                IDR_STR_STORAGE => {
                    set_str_item_owner(item, STR_OWNER_NONE);
                    set_str_item_gold(item, mission.storage_size as u32);
                    if item.driver_data.len() <= 9 {
                        item.driver_data.resize(10, 0);
                    }
                    item.driver_data[9] = 0;
                    item.name = format!("Storage ({slot})");
                }
                IDR_STR_MINE => {
                    set_str_item_owner(item, STR_OWNER_NONE);
                    set_str_item_gold(item, mission.mine_size as u32);
                    item.name = format!("Mine ({slot})");
                }
                IDR_STR_SPAWNER => {
                    if str_item_owner(item) >= STR_OWNER_AI_UNASSIGNED {
                        let enemy = mission.enemy.get(enemy_slot).copied().unwrap_or(0);
                        enemy_slot += 1;
                        set_str_item_owner(item, STR_OWNER_AI_BASE + enemy as u32);
                        if item.driver_data.len() <= 9 {
                            item.driver_data.resize(10, 0);
                        }
                        item.driver_data[9] = 0;
                    } else {
                        set_str_item_owner(item, STR_OWNER_NONE);
                    }
                    item.name = format!("Spawner ({slot})");
                }
                _ => {}
            }
        }

        if let Some(area) = self.strategy_areas.areas.get_mut(area_index) {
            area.busy = true;
        }
        true
    }

    /// C `str_ticker(int in, int cn)` (`strategy.c:456-506`): the
    /// per-tick mission-lifecycle body - scans every used battleground
    /// slot for lost parties (removing them), detects a lone-player win
    /// (rewarding and closing the slot) or an all-AI sweep (closing with
    /// no reward), and tracks each slot's `busy` flag. The
    /// `call_item(...)` self-reschedule (`:462`) and the `if (cn) return;`
    /// early-out (`:459-461`) are handled by the caller
    /// (`item_driver::str_ticker_driver` gates on `character.id.0 == 0`;
    /// `ugaris-server`'s `StrTicker` outcome arm reschedules), so this
    /// method is the unconditional per-slot body only.
    pub fn str_ticker(&mut self) {
        self.ensure_strategy_areas_initialized();

        for area_index in 0..self.strategy_areas.areas.len() {
            if !self.strategy_areas.areas[area_index].used {
                continue;
            }

            let spawn_ids = self.strategy_areas.areas[area_index].spawn.clone();
            let mut player_count = 0;
            let mut ai_count = 0;
            let mut winner = STR_OWNER_NONE;

            for item_id in spawn_ids {
                let Some(item) = self.items.get(&item_id) else {
                    continue;
                };
                let owner = str_item_owner(item);
                if owner == STR_OWNER_NONE || owner == STR_OWNER_AI_UNASSIGNED {
                    continue;
                }

                if self.str_did_party_lose(item_id) {
                    self.str_remove_party(owner, Some("You lose. Better luck next time!"));
                }

                // C re-reads `it[in].drdata` fresh here rather than
                // reusing the `code` local it read before the
                // `did_party_lose`/`remove_party` calls - if the party was
                // just removed above, this sees the *post-reset* owner
                // value (`0` for a player slot, `0xfffff000` for an AI
                // slot), not the original owner. Preserved verbatim: a
                // just-lost player slot still counts toward
                // `player_count` (with `winner` becoming `0`, so the
                // later `reward_winner` call below is skipped), and a
                // just-lost AI slot still counts toward `ai_count`.
                let Some(item) = self.items.get(&item_id) else {
                    continue;
                };
                let current_owner = str_item_owner(item);
                if current_owner < STR_OWNER_AI_UNASSIGNED {
                    player_count += 1;
                    winner = current_owner;
                } else {
                    ai_count += 1;
                }
            }

            if player_count == 1 && ai_count == 0 {
                if winner != STR_OWNER_NONE {
                    self.str_reward_winner(winner);
                }
                self.str_close_area(area_index);
            } else if ai_count > 0 && player_count == 0 {
                self.str_close_area(area_index);
            }

            let area = &mut self.strategy_areas.areas[area_index];
            if ai_count + player_count > 0 {
                area.busy = true;
            } else if area.busy {
                area.busy = false;
            }
        }
    }

    /// C `queue_validate(int ar)` (`strategy.c:3200-3216`): drops any
    /// mission-entry queue slot whose character has since logged off
    /// (C's `!ch[cn].flags`) or whose `cn` array slot got reused by a
    /// different character (`ch[cn].ID != area[ar].q_playerID[n]`) - both
    /// collapse into one existence check here, since `CharacterId` is
    /// never reused across a character's lifetime (same simplification
    /// as `ArenaContender`'s own doc comment), then compacts the
    /// remaining entries to the front of the queue, matching C's own
    /// shuffle-down loop exactly.
    pub fn queue_validate(&mut self, area_index: usize) {
        self.ensure_strategy_areas_initialized();
        let Some(area) = self.strategy_areas.areas.get_mut(area_index) else {
            return;
        };
        for n in 0..MAXQUEUE {
            if let Some(character_id) = area.q_player_cn[n] {
                if !self.characters.contains_key(&character_id) {
                    area.q_player_cn[n] = None;
                    area.q_player_id[n] = 0;
                }
            }
        }

        let mut m = 0;
        for n in 0..MAXQUEUE {
            area.q_player_cn[m] = area.q_player_cn[n];
            area.q_player_id[m] = area.q_player_id[n];
            if area.q_player_cn[m].is_some() {
                m += 1;
            }
        }
        for slot in area.q_player_cn.iter_mut().skip(m) {
            *slot = None;
        }
        for slot in area.q_player_id.iter_mut().skip(m) {
            *slot = 0;
        }
    }

    /// C `queue_remove(int cn)` (`strategy.c:3220-3230`): removes a
    /// character from every battleground slot's mission-entry queue
    /// (C scans by `ch[cn].ID`; `CharacterId` identity is equivalent and
    /// simpler, per [`Self::queue_validate`]'s doc comment).
    pub fn queue_remove(&mut self, character_id: CharacterId) {
        self.ensure_strategy_areas_initialized();
        for area in &mut self.strategy_areas.areas {
            for n in 0..MAXQUEUE {
                if area.q_player_cn[n] == Some(character_id) {
                    area.q_player_cn[n] = None;
                    area.q_player_id[n] = 0;
                }
            }
        }
    }

    /// C `queue_mission(int cn, int ar)` (`strategy.c:3232-3253`): enters
    /// a character into an area's mission queue, unless already present;
    /// otherwise removes any stale entry it may hold elsewhere first
    /// (C's own `queue_remove(cn)` call), then appends to the first free
    /// slot. A full queue (all 4 slots occupied) silently drops the
    /// request, matching C's own no-op fallthrough.
    pub fn queue_mission(&mut self, character_id: CharacterId, area_index: usize) {
        self.queue_validate(area_index);

        let Some(area) = self.strategy_areas.areas.get(area_index) else {
            return;
        };
        if area.q_player_cn.contains(&Some(character_id)) {
            return;
        }

        self.queue_remove(character_id);

        let serial = self
            .characters
            .get(&character_id)
            .map(|c| c.serial)
            .unwrap_or(0);
        let Some(area) = self.strategy_areas.areas.get_mut(area_index) else {
            return;
        };
        for n in 0..MAXQUEUE {
            if area.q_player_cn[n].is_none() {
                area.q_player_cn[n] = Some(character_id);
                area.q_player_id[n] = serial;
                return;
            }
        }
    }

    /// C `queue_check(int cn, int ar)` (`strategy.c:3255-3263`): whether
    /// `character_id` is free to enter the mission (queue empty, or it
    /// already occupies the head slot).
    pub fn queue_check(&mut self, character_id: CharacterId, area_index: usize) -> bool {
        self.queue_validate(area_index);
        let Some(area) = self.strategy_areas.areas.get(area_index) else {
            return true;
        };
        !matches!(area.q_player_cn[0], Some(head) if head != character_id)
    }

    /// C `show_queue(int cn, int ar)` (`strategy.c:3265-3276`): sends the
    /// "Queue:" header (C logs this line *before* validating - order
    /// preserved verbatim) followed by one "`n: name`" line per occupied
    /// slot, to `character_id` via [`Self::queue_system_text`].
    pub fn show_queue(&mut self, character_id: CharacterId, area_index: usize) {
        self.queue_system_text(character_id, "Queue:".to_string());
        self.queue_validate(area_index);
        let Some(area) = self.strategy_areas.areas.get(area_index) else {
            return;
        };
        let entries: Vec<(usize, CharacterId)> = area
            .q_player_cn
            .iter()
            .enumerate()
            .filter_map(|(n, c)| c.map(|cid| (n, cid)))
            .collect();
        for (n, cid) in entries {
            let name = self
                .characters
                .get(&cid)
                .map(|c| c.name.clone())
                .unwrap_or_default();
            self.queue_system_text(character_id, format!("{}: {name}", n + 1));
        }
    }
}
