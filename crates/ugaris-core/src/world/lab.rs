//! `create_lab_exit` (`src/system/lab.c:137-158`): the reward-drop half of
//! the "kill a lab master, get a `labexit` gate, use it to solve the
//! level and warp to Aston" loop shared verbatim by all five
//! `src/area/22/lab*.c` files (`lab1.c:403`, `lab2.c:277`, `lab3.c:961`,
//! `lab4.c:274`, `lab5.c:451`, all `create_lab_exit(co, <level>)` calls
//! from the area's own master-kill death hook). `create_lab_exit` itself
//! needs `create_item`/`drop_item_extended` against a real
//! `ZoneLoader`-backed template, which `World` alone can't do (same
//! architectural gap as every other "spawn a fresh item" reward - see
//! `world::xmas`'s `grant_xmas_tree_gift` for the established
//! `ugaris-server`-side pattern this port reuses), so this module is only
//! the `World`-side queue: [`World::queue_lab_exit_spawn`] (called from
//! each area's own master-kill death hook, C's `if (co && (ch[co].flags &
//! CF_PLAYER))` guard) and [`World::drain_pending_lab_exit_spawns`]
//! (drained by `ugaris-server`'s `lab::create_lab_exit`, C's own function
//! of the same name, from `tick_sync::sync_phase`).
use super::*;

/// C `create_lab_exit(cn, level)`'s deferred request: `killer_id` is C's
/// `cn` parameter (the player who gets the reward gate dropped at their
/// feet), `level` is the lab-level tag the gate itself carries
/// (`it[in].drdata[4]`), read back by [`ItemDriverOutcome::LabExitUse`]
/// on use to call `set_solved_lab`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabExitSpawnRequest {
    pub killer_id: CharacterId,
    pub level: u8,
}

impl World {
    /// Queues a `create_lab_exit(co, level)` reward drop - see the module
    /// doc comment for why the actual item creation is deferred to
    /// `ugaris-server`.
    pub fn queue_lab_exit_spawn(&mut self, killer_id: CharacterId, level: u8) {
        self.pending_lab_exit_spawns
            .push(LabExitSpawnRequest { killer_id, level });
    }

    /// Drains every `create_lab_exit` request queued this tick.
    pub fn drain_pending_lab_exit_spawns(&mut self) -> Vec<LabExitSpawnRequest> {
        self.pending_lab_exit_spawns.drain(..).collect()
    }
}
