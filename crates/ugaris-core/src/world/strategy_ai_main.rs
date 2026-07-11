//! Assembles the individually-ported `ai_init`/`ai_main` pieces
//! (`crate::world::strategy_ai`/`strategy_ai_tasks`, 21 prior slices - see
//! that module's own doc comment for the full porting history) into the
//! real per-tick call C's own `ai_main(int in, unsigned int code)`
//! (`strategy.c:2449-2971`) makes. This is the assembly step
//! `PORTING_TODO.md`'s "Areas 23/24" task REMAINING note calls out as the
//! last missing piece of the whole AI-opponent driver.
//!
//! Reproduces C's exact per-tick order: `ai_init` (only if this `code`
//! hasn't been seen before, C's own `if (!ad->ai_init)` gate - a missing
//! [`World::ai_parties`] entry *is* that gate) -> update npc list ->
//! update guard list -> update nag guard -> per-place worker/eguard-count
//! refresh -> per-place platin/owned/threat refresh -> update free npc
//! count -> (maybe) plan one new worker -> assign tasks to workers ->
//! (non-panic only:) threat-list/worklevel tick -> (maybe) plan one new
//! eternal guard -> nag attack -> commit `ragnarok`/`nogoldleft` ->
//! dispatch tasks to every live worker.
//!
//! Two documented simplifications, both already flagged by
//! [`AiWorkerSpawnPlan`]/[`AiEguardSpawnPlan`]'s own doc comments as
//! needing a `ZoneLoader`-backed caller this pure `World` method can't
//! provide:
//! - C's "create new workers"/"place eternal guards" blocks are each a
//!   loop that can spawn several NPCs in one `ai_main` call if there's
//!   enough gold/eligible places; [`World::ai_main`] plans **at most one
//!   of each** per call instead. `ai_main` fires roughly once a second
//!   (`IDR_STR_SPAWNER`'s own reschedule interval), so a party under its
//!   worker/eguard caps with spare gold still converges to the same
//!   steady state as C, just spread across a few more ticks instead of
//!   bursting in one - a real, minor timing difference, not an
//!   unreachable outcome. `ugaris-server` is expected to build the actual
//!   character from a returned plan and call [`World::
//!   register_ai_worker`]/[`World::register_ai_eguard`] back before the
//!   *next* `ai_main` call for this party - the same "plan now, register
//!   once the caller actually creates the character" split every other
//!   spawn plan in this subsystem already uses (`AiWorkerSpawnPlan`'s own
//!   doc comment).
//! - Wiring a live `IDR_STR_SPAWNER` `cn == 0` timer tick to actually call
//!   this method (`spawner`'s own `strategy.c:1319-1356` ambient/AI-init
//!   branch) remains a separate, still-unported gap - see
//!   `item_driver::area23_24`'s module doc comment and the "Areas 23/24"
//!   task's own REMAINING note in `PORTING_TODO.md`. This method is fully
//!   exercised by direct unit tests in the meantime, the same "ported but
//!   not yet spawnable" precedent every other piece of this subsystem
//!   already established.

use super::*;

/// What [`World::ai_main`] wants `ugaris-server` to do before the next
/// call for this same AI party - see this module's doc comment for why
/// at most one of each is ever returned.
#[derive(Debug, Default)]
pub struct AiMainOutcome {
    pub worker_plan: Option<AiWorkerSpawnPlan>,
    /// The place index [`World::ai_plan_eguard_spawn`] resolved this plan
    /// from - [`World::register_ai_eguard`] needs it back (C's own
    /// `create_eguard(n)` call already has `n`, the place index, from the
    /// very loop that found it eligible).
    pub eguard_plan: Option<(usize, AiEguardSpawnPlan)>,
}

impl World {
    /// C `ai_main(int in, unsigned int code)` (`strategy.c:2449-2971`) -
    /// see this module's doc comment for the exact call order and the two
    /// spawn-plan simplifications.
    pub fn ai_main(&mut self, spawner_item: ItemId, code: u32) -> AiMainOutcome {
        if !self.ai_parties.contains_key(&code) {
            let Some(fresh) = self.ai_init(spawner_item, code) else {
                return AiMainOutcome::default();
            };
            self.ai_parties.insert(code, fresh);
        }
        let Some(mut ad) = self.ai_parties.remove(&code) else {
            return AiMainOutcome::default();
        };

        // "update npc list" / "update guard list" / "update nag guard"
        // (`:2461-2500`).
        self.ai_update_npc_list(&mut ad);
        ad.update_guard_list();
        ad.update_nag_guard(self.tick.0 as i64);

        // Per-place worker/eguard-count refresh, then platin/owned/threat
        // refresh (`:2505-2630`) - `ad.pdist = ad.pdist.min(mindist)` is
        // already applied inside `ai_refresh_places` itself.
        ad.update_place_worker_and_eguard_counts();
        let refresh = self.ai_refresh_places(&mut ad, code);

        // "update free npc count" (`:2632-2642`).
        ad.update_free_npc_count();

        let mut outcome = AiMainOutcome::default();

        // "create new workers" (`:2644-2672`) - plans at most one; see
        // this module's doc comment.
        if self.ai_wants_more_workers(&ad) {
            outcome.worker_plan = self.ai_plan_worker_spawn(spawner_item, &ad, code);
        }

        // "assign tasks to workers" (`:2674-2796`) - the panic/non-panic
        // branch is already selected internally on `ad.panic`.
        ad.assign_tasks_to_workers(refresh.mindist);

        if !ad.panic {
            // "find places with too little workers" / threat-list
            // maintenance / worklevel adjust (`:2798-2916`).
            let ragnarok = self.ai_threat_and_worklevel_tick(
                &mut ad,
                self.tick.0 as i64,
                refresh.mindist,
                refresh.ragnarok,
            );

            // "place eternal guards" (`:2892-2911`) - plans at most one;
            // see this module's doc comment.
            if self.ai_wants_more_eguards(&ad) {
                if let Some(&place) = self.ai_eguard_spawn_candidates(&ad).first() {
                    if let Some(plan) = self.ai_plan_eguard_spawn(&ad, place, code) {
                        outcome.eguard_plan = Some((place, plan));
                    }
                }
            }

            // `nag_attack()` (`:2911`).
            self.ai_nag_attack(&mut ad);

            // `ad->ragnarok = ragnarok; ad->nogoldleft = nogoldleft;`
            // (`:2926-2927`) - only committed on the non-panic path,
            // matching C's own control flow exactly (both locals are
            // never touched at all in the panic branch).
            ad.ragnarok = ragnarok;
            ad.nogoldleft = refresh.nogoldleft;
        }

        // "make NPCs do their jobs" (`:2932-2972`).
        self.ai_dispatch_tasks(&mut ad);

        self.ai_parties.insert(code, ad);
        outcome
    }

    /// C's "add new npc to list" tail of the "create new workers" loop
    /// (`strategy.c:2661-2669`), called once `ugaris-server` has actually
    /// built the character [`AiMainOutcome::worker_plan`] asked for. A
    /// no-op if `code` doesn't (or no longer) resolve to a live
    /// [`AiData`] - the party could have been removed (`str_remove_party`)
    /// between planning and this call.
    pub fn register_ai_worker(&mut self, code: u32, cn: CharacterId, x: u16, y: u16) {
        if let Some(ad) = self.ai_parties.get_mut(&code) {
            ad.register_new_worker(cn, x, y);
        }
    }

    /// C's "add new npc to list" tail of the "place eternal guards" block
    /// (`strategy.c:2899-2916`), called once `ugaris-server` has actually
    /// built the character [`AiMainOutcome::eguard_plan`] asked for. See
    /// [`Self::register_ai_worker`]'s own doc comment for the same
    /// "party may already be gone" no-op case.
    pub fn register_ai_eguard(&mut self, code: u32, cn: CharacterId, x: u16, y: u16, place: usize) {
        if let Some(ad) = self.ai_parties.get_mut(&code) {
            ad.register_new_eguard(cn, x, y, place);
        }
    }
}
