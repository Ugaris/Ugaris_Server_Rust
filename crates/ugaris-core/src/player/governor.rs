//! Area 32 governor "Mister Jones" mission-giver persistent player state.
//!
//! Mirrors C `struct mission_ppd`/`struct single_mission`
//! (`src/common/mission_ppd.h`, backing `DRD_MISSION_PPD`), the per-player
//! job-board profile for `src/area/32/missions.c::mission_giver_driver`
//! (`CDR_MISSIONGIVE`). See `crate::world::npc::area32::governor`'s module
//! doc comment for the ported/remaining slice breakdown.

use serde::{Deserialize, Serialize};

/// C `struct single_mission` (`mission_ppd.h:5-9`): one of the three
/// Alpha/Beta/Gamma job-board slots.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SingleMission {
    /// C `int type` (`MISS_TYPE_KILL` = 1; `0` = slot empty/unrolled).
    pub mission_type: i32,
    /// C `int mdidx`: index into `mdtab[]` (see
    /// [`crate::world::npc::area32::governor::MISSION_TEMPLATES`]).
    pub mdidx: i32,
    /// C `int difficulty`.
    pub difficulty: i32,
}

/// C `struct mission_ppd` (`mission_ppd.h:11-34`, `DRD_MISSION_PPD`).
///
/// `md_idx`/`kill_easy`/`kill_normal`/`kill_hard`/`kill_boss`/`find_item`/
/// `mcnt` back C `start_mission`/`build_fighter`/`missionchest_driver`/
/// `mission_status`/`mission_done` - the instance-dungeon spawn/kill-
/// tracking machinery this port has not ported yet (see the module doc
/// comment on `crate::world::npc::area32::governor` for the precise gap);
/// they exist here already, zero-initialized, so that slice can be added
/// later as a pure additive change with no further `PlayerRuntime` field
/// churn. `statowed`/`statcnt`/`stat` back the now-ported custom stat
/// potion (`CTPOT` reward) flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MissionPpd {
    /// C `int missiongive_state`: `0` greet, `1` about to offer jobs, `2`
    /// waiting for the player.
    pub missiongive_state: i32,
    /// C `int lastseenmissiongiver`: wall-clock `realtime` seconds; resets
    /// `missiongive_state` to `0` once more than 30s stale.
    pub lastseenmissiongiver: u64,
    /// C `int active`: `1`-based index+1 of the currently accepted job
    /// (`0` = none).
    pub active: i32,
    /// C `int solved`: `1`-based index+1 of the job solved and awaiting
    /// reward collection (`0` = none).
    pub solved: i32,
    /// C `int points`: the "brownie points" reward-shop currency.
    pub points: i32,
    /// C `int mcnt`: accepted-missions counter, seeds `start_mission`'s
    /// per-instance key item IDs. Not yet incremented (`start_mission` is
    /// unported).
    pub mcnt: i32,
    /// C `int dif_kill`: rolling kill-mission difficulty baseline,
    /// player-adjustable via "increase"/"decrease", clamped to
    /// `[0, MAXDIFF]` (`MAXDIFF` = 1000,
    /// [`crate::world::npc::area32::governor::MAX_DIFFICULTY`]).
    pub dif_kill: i32,
    /// C `struct single_mission sm[3]`: the three offered Alpha/Beta/Gamma
    /// jobs.
    pub sm: [SingleMission; 3],
    /// C `int md_idx`: the active job's `mdtab[]` index. Not yet written
    /// (`start_mission` is unported).
    pub md_idx: i32,
    /// C `int kill_easy[2]` (`[done, total]`). Not yet written.
    pub kill_easy: [i32; 2],
    /// C `int kill_normal[2]` (`[done, total]`). Not yet written.
    pub kill_normal: [i32; 2],
    /// C `int kill_hard[2]` (`[done, total]`). Not yet written.
    pub kill_hard: [i32; 2],
    /// C `int kill_boss[2]` (`[done, total]`). Not yet written.
    pub kill_boss: [i32; 2],
    /// C `int find_item[2]` (`[done, total]`). Not yet written.
    pub find_item: [i32; 2],
    /// C `int statowed`: custom stat potions purchased but not yet
    /// configured. Incremented by the `CTPOT` reward give-flow, decremented
    /// back to `0` once the potion is successfully handed over (see
    /// `crate::world::npc::area32::governor`'s module doc comment).
    pub statowed: i32,
    /// C `int statcnt`: how many stats (1-3) the in-progress custom
    /// potion should hold.
    pub statcnt: i32,
    /// C `int stat[3]`: the `V_*` skill indices chosen so far for the
    /// in-progress custom potion.
    pub stat: [i32; 3],
}
