//! `World`-level methods for the Areas 23/24 AI-opponent driver
//! (`ai_init`/`ai_main`, `src/area/23_24/strategy.c:2269-2994`) - split out
//! of `crate::world::strategy_ai` (which keeps the pure `AiData`/`AiPlace`/
//! `AiNpc` types, their own inherent-`impl` bookkeeping methods, and the
//! full module-level porting-status doc comment) once that file crossed
//! ~1,900 lines, per the split-before-next-slice note left there. Every
//! `World` method that operates on a live [`AiData`] - the seven `task_*`
//! order-resolution functions, the place-graph navigation primitives, the
//! defense-allocation/nag-attack logic, `ai_init`'s place-graph
//! construction, and `ai_refresh_places`'s per-tick threat scan - lives
//! here. See `crate::world::strategy_ai`'s module doc comment for the
//! full porting history and the REMAINING list (worker spawning, the
//! "place eternal guards" tail, the final per-npc dispatch switch, and
//! wiring a live `ai_main` call site).

use super::*;
use crate::character_driver::CDR_STRATEGY;
use crate::path::pathfinder;

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

    /// C `ai_main`'s final "make NPCs do their jobs" `switch` (`strategy.c:
    /// 2932-2972`): dispatch every roster NPC to the `task_*` function
    /// matching its current [`AiTask`], then sync the raw `order`/`or1`/
    /// `or2` triple that call just wrote back onto the live worker
    /// character's own [`StrategyWorkerDriverData::order`] (C's `dat->order
    /// = ad->an[n].order` etc., auto-vivifying the driver state exactly
    /// like C's `set_data` and [`World::ai_task_idle`] above, for the same
    /// reason: a live AI-controlled worker's driver state is not
    /// guaranteed to have been touched by a real tick yet the first time
    /// this runs). [`AiTask::EGuard`]'s nested `if` (train if idle at
    /// storage and the economy can afford it, else idle; guard if it has
    /// an actual assigned target) and [`AiTask::Ignore`]'s no-op
    /// (eternal guards keep whatever order they were created with) are
    /// both kept verbatim.
    pub fn ai_dispatch_tasks(&mut self, ad: &mut AiData) {
        for n in 0..ad.npcs.len() {
            match ad.npcs[n].task {
                AiTask::Idle => self.ai_task_idle(ad, n),
                AiTask::Mine => self.ai_task_mine(ad, n),
                AiTask::Transfer => self.ai_task_transfer(ad, n),
                AiTask::Fight => self.ai_task_fight(ad, n),
                AiTask::EGuard => {
                    if ad.npcs[n].target == 0 {
                        let level = ad.npcs[n]
                            .cn
                            .and_then(|cn| self.characters.get(&cn))
                            .map(|c| c.level as i32)
                            .unwrap_or(0);
                        let storage_platin = ad.places.first().map(|p| p.platin).unwrap_or(0);
                        if level < ad.ppd.max_level
                            && (storage_platin > NPCPRICE * 2
                                || ad.free_workers != 0
                                || ad.npcs[n].platin > ad.ppd.trainspeed * TRAINMULTI * 2
                                || ad.npc_cnt >= ad.ppd.max_worker)
                        {
                            self.ai_task_train(ad, n);
                        } else {
                            self.ai_task_idle(ad, n);
                        }
                    } else {
                        self.ai_task_guard(ad, n);
                    }
                }
                AiTask::Ignore => {}
                AiTask::Take => self.ai_task_take(ad, n),
            }

            let Some(worker_id) = ad.npcs[n].cn else {
                continue;
            };
            let order =
                raw_to_strategy_worker_order(ad.npcs[n].order, ad.npcs[n].or1, ad.npcs[n].or2);
            if let Some(character) = self.characters.get_mut(&worker_id) {
                match character.driver_state.get_or_insert_with(|| {
                    CharacterDriverState::StrategyWorker(StrategyWorkerDriverData::default())
                }) {
                    CharacterDriverState::StrategyWorker(data) => data.order = order,
                    other => {
                        *other = CharacterDriverState::StrategyWorker(StrategyWorkerDriverData {
                            order,
                            ..Default::default()
                        })
                    }
                }
            }
        }
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
            mindist,
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

/// Everything [`World::ai_plan_worker_spawn`] needs to hand off to
/// `ugaris-server` for the actual `ZoneLoader`-needing character creation
/// (C `spawner_sub`'s own `create_char`/`item_drop_char` tail,
/// `strategy.c:1259-1286`) - same split precedent as
/// `StrategySpawnerSpawnPlan`/`World::try_dispatch_strategy_spawner_use`
/// (the player-triggered spawner), just fed from an [`AiPreset`] instead
/// of a player's own name/[`StrategyPpd`].
pub struct AiWorkerSpawnPlan {
    pub spawner_id: ItemId,
    /// C `group` (`ai_main`'s own `code` parameter, forwarded to
    /// `spawner_sub` unchanged) - narrowed to `u16` to match
    /// [`Character::group`]'s own field type. This is the same
    /// pre-existing, documented gap [`World::ai_init`]'s own doc comment
    /// already flags: a real [`STR_OWNER_AI_BASE`]-range `code` can never
    /// actually round-trip through a `u16` field, so a worker spawned via
    /// this plan can never be rediscovered by `ai_init`'s own roster
    /// scan - not a new gap introduced here, just the same one appearing
    /// on the spawning side too.
    pub group: u16,
    /// C `name` (`preset[code - STR_OWNER_AI_BASE].name`, truncated to
    /// 20 chars by `spawner_sub`'s own `strncpy(dat->name, name, 19)`,
    /// same truncation [`crate::world::StrategySpawnerSpawnPlan::
    /// owner_name`] already applies for the player-triggered spawner).
    pub owner_name: String,
    pub warcry: i32,
    pub endurance: i32,
    pub speed: i32,
    pub trainspeed: i32,
    pub max_level: i32,
    pub npc_color: i32,
}

impl World {
    /// C `ai_main`'s "create new workers" `while` loop condition
    /// (`strategy.c:2644-2645`): should the AI attempt to spawn (at
    /// least) one more worker this tick? Reads [`AiData::storage_item`]'s
    /// live gold total directly (C's own `*(unsigned int
    /// *)(it[ad->storage_in].drdata + 4)`) rather than a cached copy,
    /// same "read the live item, not a stale snapshot" precedent as
    /// every other `str_item_gold` caller in this file.
    pub fn ai_wants_more_workers(&self, ad: &AiData) -> bool {
        let Some(storage) = self.items.get(&ad.storage_item) else {
            return false;
        };
        let gold = str_item_gold(storage);
        let cap = ad.ppd.max_worker.min(16 + (gold / 500) as i32);
        (ad.panic || ad.free_workers == 0) && ad.npc_cnt < cap
    }

    /// One iteration of C `ai_main`'s "create new workers" loop body, up
    /// to (not including) `spawner_sub`'s own character-creation tail
    /// (`strategy.c:2646-2660` minus the `create_char`/`item_drop_char`
    /// call itself - `ugaris-server` builds the actual character from the
    /// returned plan, same split as
    /// [`World::try_dispatch_strategy_spawner_use`]). Deducts `NPCPRICE`
    /// from storage gold *unconditionally* once eligible, before the
    /// caller ever attempts to build a character - the same
    /// spend-before-creation-attempt quirk `spawner_sub` itself has for
    /// the player-triggered spawner (see
    /// [`World::try_dispatch_strategy_spawner_use`]'s own doc comment) -
    /// the caller must NOT refund it if character creation subsequently
    /// fails; C's own `ai_main` simply `break`s the "create new workers"
    /// loop in that case (`spawner_sub` returning `0`), with no
    /// player-facing message since this is AI-side, not a player action.
    ///
    /// Returns `None` (C's own `else { break; }`, or `spawner_sub`'s own
    /// unreachable-in-practice `code` guard) if there isn't `NPCPRICE`
    /// gold available, or `code` doesn't resolve to a real
    /// [`AI_PRESETS`] row - either way, the caller should stop attempting
    /// to spawn more workers this tick, matching C's `break`.
    pub fn ai_plan_worker_spawn(
        &mut self,
        spawner_id: ItemId,
        ad: &AiData,
        code: u32,
    ) -> Option<AiWorkerSpawnPlan> {
        let preset_index = code.checked_sub(STR_OWNER_AI_BASE)? as usize;
        let preset = AI_PRESETS.get(preset_index)?;
        if str_item_gold(self.items.get(&ad.storage_item)?) < NPCPRICE as u32 {
            return None;
        }
        if let Some(storage) = self.items.get_mut(&ad.storage_item) {
            let new_gold = str_item_gold(storage).saturating_sub(NPCPRICE as u32);
            set_str_item_gold(storage, new_gold);
        }
        // C `strncpy(dat->name, name, 19); dat->name[19] = 0;` - 19
        // visible characters, same truncation as
        // `World::try_dispatch_strategy_spawner_use`'s own `owner_name`.
        let owner_name: String = preset.name.chars().take(19).collect();
        Some(AiWorkerSpawnPlan {
            spawner_id,
            group: code as u16,
            owner_name,
            warcry: ad.ppd.warcry,
            endurance: ad.ppd.endurance,
            speed: ad.ppd.speed,
            trainspeed: ad.ppd.trainspeed,
            max_level: ad.ppd.max_level,
            npc_color: ad.ppd.npc_color,
        })
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

/// The other direction: C's `dat->order = ad->an[n].order; dat->or1 =
/// ad->an[n].or1; dat->or2 = ad->an[n].or2;` write-back at the end of
/// `ai_main`'s per-npc dispatch loop (`strategy.c:2967-2971`), needed by
/// [`World::ai_dispatch_tasks`] to sync a `task_*` function's raw output
/// back onto the live worker's typed [`StrategyWorkerOrder`]. An
/// unrecognized raw order code (never produced by any `task_*` function
/// or `create_eguard`, so unreachable in practice) falls back to
/// [`StrategyWorkerOrder::None`], same "unknown raw state coerces to the
/// default" precedent used elsewhere in this port.
fn raw_to_strategy_worker_order(order: i32, or1: i32, or2: i32) -> StrategyWorkerOrder {
    if order == OR_MINE {
        StrategyWorkerOrder::Mine {
            mine_item: ItemId(or1 as u32),
            depot_item: ItemId(or2 as u32),
        }
    } else if order == OR_FOLLOW {
        StrategyWorkerOrder::Follow {
            leader: CharacterId(or1 as u32),
        }
    } else if order == OR_GUARD {
        StrategyWorkerOrder::Guard {
            x: or1 as u16,
            y: or2 as u16,
        }
    } else if order == OR_FIGHTER {
        StrategyWorkerOrder::Fighter {
            leader: CharacterId(or1 as u32),
        }
    } else if order == OR_TAKE {
        StrategyWorkerOrder::Take {
            depot_item: ItemId(or1 as u32),
            leader: CharacterId(or2 as u32),
        }
    } else if order == OR_TRANSFER {
        StrategyWorkerOrder::Transfer {
            from_item: ItemId(or1 as u32),
            to_item: ItemId(or2 as u32),
        }
    } else if order == OR_TRAIN {
        StrategyWorkerOrder::Train {
            storage_item: ItemId(or1 as u32),
        }
    } else if order == OR_ETERNALGUARD {
        StrategyWorkerOrder::EternalGuard {
            x: or1 as u16,
            y: or2 as u16,
        }
    } else {
        StrategyWorkerOrder::None
    }
}
