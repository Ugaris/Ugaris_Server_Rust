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
//! REMAINING (tracked in `PORTING_TODO.md`, left `[~]` on purpose): the
//! `struct str_area area[MAX_STR_AREA]` runtime registry + `init_areas`/
//! `str_ticker` (needs a `World`-level scan of every `IDR_STR_*` item on
//! load, mirroring `world::pents`'s slot bookkeeping), the worker
//! character driver (`strategy_driver`, order assignment via NPC speech,
//! `setname`/`restplace`), the `mine`/`storage`/`depot`/`spawner`/
//! `nosnow` item drivers (currently dispatched as C-parity no-ops, see
//! `item_driver::dispatch`), the full AI-opponent driver (`ai_init`/
//! `ai_main`, `:2277-2994`), the mission queue (`queue_*`, `:3200-3276`),
//! and the boss NPC dialogue driver (`strategy_boss`, `:1414-1616`, plus
//! `special_driver`'s player-facing `#`-style commands, `:3278-3632`).

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
