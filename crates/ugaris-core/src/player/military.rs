use super::*;

impl PlayerRuntime {
    pub fn encode_legacy_military_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_MILITARY_PPD_SIZE];
        let copy_len = self.military_ppd.len().min(LEGACY_MILITARY_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.military_ppd[..copy_len]);
        bytes
    }

    pub fn decode_legacy_military_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_MILITARY_PPD_SIZE {
            return false;
        }
        self.military_ppd = bytes[..LEGACY_MILITARY_PPD_SIZE].to_vec();
        true
    }

    pub(crate) fn read_military_i32(&self, offset: usize) -> i32 {
        if self.military_ppd.len() < LEGACY_MILITARY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.military_ppd, offset)
    }

    pub(crate) fn write_military_i32(&mut self, offset: usize, value: i32) {
        if self.military_ppd.len() < LEGACY_MILITARY_PPD_SIZE {
            self.military_ppd.resize(LEGACY_MILITARY_PPD_SIZE, 0);
        }
        write_i32(&mut self.military_ppd, offset, value);
    }

    pub(crate) fn military_mission_offset(idx: usize) -> usize {
        MILITARY_PPD_MIS_BASE_OFFSET
            + idx.min(MILITARY_PPD_MISSION_COUNT - 1) * MILITARY_PPD_MIS_ENTRY_SIZE
    }

    /// C `military_ppd::mis[idx]` (`military.h:43`), one of the 5 offered
    /// missions (easy/normal/hard/impossible/insane, index 0..=4).
    pub fn military_mission(&self, idx: usize) -> crate::world::SingleMission {
        let base = Self::military_mission_offset(idx);
        crate::world::SingleMission {
            mission_type: self.read_military_i32(base),
            opt1: self.read_military_i32(base + 4),
            opt2: self.read_military_i32(base + 8),
            pts: self.read_military_i32(base + 12),
            exp: self.read_military_i32(base + 16),
        }
    }

    pub fn set_military_mission(&mut self, idx: usize, mission: crate::world::SingleMission) {
        let base = Self::military_mission_offset(idx);
        self.write_military_i32(base, mission.mission_type);
        self.write_military_i32(base + 4, mission.opt1);
        self.write_military_i32(base + 8, mission.opt2);
        self.write_military_i32(base + 12, mission.pts);
        self.write_military_i32(base + 16, mission.exp);
    }

    pub(crate) fn set_military_mission_opt1(&mut self, idx: usize, opt1: i32) {
        let base = Self::military_mission_offset(idx) + 4;
        self.write_military_i32(base, opt1);
    }

    /// C `military_ppd::took_mission` (`military.h:45`): 0 = no active
    /// mission, else `1 + difficulty` (the offered mission's index this
    /// player accepted).
    pub fn military_took_mission(&self) -> i32 {
        self.read_military_i32(MILITARY_PPD_TOOK_MISSION_OFFSET)
    }

    pub fn set_military_took_mission(&mut self, value: i32) {
        self.write_military_i32(MILITARY_PPD_TOOK_MISSION_OFFSET, value);
    }

    /// C `military_ppd::took_yday` (`military.h:46`): day of the year
    /// (`yday + 1`) this player accepted the currently-active mission.
    pub fn military_took_yday(&self) -> i32 {
        self.read_military_i32(MILITARY_PPD_TOOK_YDAY_OFFSET)
    }

    pub fn set_military_took_yday(&mut self, value: i32) {
        self.write_military_i32(MILITARY_PPD_TOOK_YDAY_OFFSET, value);
    }

    /// C `military_ppd::solved_mission` (`military.h:47`).
    pub fn military_solved_mission(&self) -> bool {
        self.read_military_i32(MILITARY_PPD_SOLVED_MISSION_OFFSET) != 0
    }

    pub fn set_military_solved_mission(&mut self, value: bool) {
        self.write_military_i32(MILITARY_PPD_SOLVED_MISSION_OFFSET, i32::from(value));
    }

    /// C `military_ppd::solved_yday` (`military.h:48`): day of the year
    /// this player last solved a mission (`took_yday` at solve time),
    /// used by `accept_mission`'s "already completed today" gate.
    pub fn military_solved_yday(&self) -> i32 {
        self.read_military_i32(MILITARY_PPD_SOLVED_YDAY_OFFSET)
    }

    pub fn set_military_solved_yday(&mut self, value: i32) {
        self.write_military_i32(MILITARY_PPD_SOLVED_YDAY_OFFSET, value);
    }

    /// C `military_ppd::current_pts` (`military.h:29`): the "unused
    /// 'recommendation' pts" balance `accept_mission` spends on paying for
    /// a non-advisor mission (C's own comment calls it unused, but
    /// `accept_mission` reads/writes it regardless - matching that
    /// verbatim, not the stale comment).
    pub fn military_current_pts(&self) -> i32 {
        self.read_military_i32(MILITARY_PPD_CURRENT_PTS_OFFSET)
    }

    pub fn set_military_current_pts(&mut self, value: i32) {
        self.write_military_i32(MILITARY_PPD_CURRENT_PTS_OFFSET, value);
    }

    /// C `military_ppd::mission_type_preference` (`military.h:50`): 0 = no
    /// preference, 1/2/3 = demon/ratling/silver.
    pub fn mission_type_preference(&self) -> i32 {
        self.read_military_i32(MILITARY_PPD_MISSION_TYPE_PREFERENCE_OFFSET)
    }

    pub fn set_mission_type_preference(&mut self, value: i32) {
        self.write_military_i32(MILITARY_PPD_MISSION_TYPE_PREFERENCE_OFFSET, value);
    }

    /// C `military_ppd::mission_difficulty_preference` (`military.h:51`):
    /// `0..=4` (easy..insane) or anything else for "no preference" (C's
    /// own struct doesn't reserve a dedicated sentinel; the default value
    /// after a fresh `set_data` zero-init is `0`, matching easy - callers
    /// that want "unset" must explicitly write a negative value, same as
    /// C's own callers never do differently).
    pub fn mission_difficulty_preference(&self) -> i32 {
        self.read_military_i32(MILITARY_PPD_MISSION_DIFFICULTY_PREFERENCE_OFFSET)
    }

    pub fn set_mission_difficulty_preference(&mut self, value: i32) {
        self.write_military_i32(MILITARY_PPD_MISSION_DIFFICULTY_PREFERENCE_OFFSET, value);
    }

    /// C `military_ppd::mission_yday` (`military.h:41`): day of the year
    /// (`yday + 1`, i.e. tomorrow) the current mission offer table
    /// (`mis[5]`) was generated - used elsewhere in C to decide whether
    /// to regenerate a stale offer (not itself ported yet).
    pub fn mission_yday(&self) -> i32 {
        self.read_military_i32(MILITARY_PPD_MISSION_YDAY_OFFSET)
    }

    pub fn set_mission_yday(&mut self, value: i32) {
        self.write_military_i32(MILITARY_PPD_MISSION_YDAY_OFFSET, value);
    }

    /// C `military_ppd::advisor_last[idx]` (`military.h:37`,
    /// `MAXADVISOR` = [`MILITARY_PPD_MAXADVISOR`] = 20): day-of-year each
    /// advisor slot was last consulted, used by the (still-unported)
    /// advisor-recommendation cooldown check. Index is clamped to the
    /// valid range like every other slot accessor in this file.
    pub fn military_advisor_last(&self, idx: usize) -> i32 {
        let idx = idx.min(MILITARY_PPD_MAXADVISOR - 1);
        self.read_military_i32(MILITARY_PPD_ADVISOR_LAST_BASE_OFFSET + idx * 4)
    }

    pub fn set_military_advisor_last(&mut self, idx: usize, value: i32) {
        let idx = idx.min(MILITARY_PPD_MAXADVISOR - 1);
        self.write_military_i32(MILITARY_PPD_ADVISOR_LAST_BASE_OFFSET + idx * 4, value);
    }

    /// C `military_ppd::reroll_yday` (`military.h:59`): day of the year
    /// the player last used the (still-unported) mission-reroll option.
    pub fn military_reroll_yday(&self) -> i32 {
        self.read_military_i32(MILITARY_PPD_REROLL_YDAY_OFFSET)
    }

    pub fn set_military_reroll_yday(&mut self, value: i32) {
        self.write_military_i32(MILITARY_PPD_REROLL_YDAY_OFFSET, value);
    }

    /// C `military_ppd::master_state` (`military.h:29`): the Military
    /// Master driver's own dialogue-state machine (`greet_player`'s
    /// `0`=fresh/`1`=greeted-once/`2`=ready-for-mission-talk states plus
    /// `handle_mission_reroll`'s `10`=awaiting-confirmation state).
    pub fn master_state(&self) -> i32 {
        self.read_military_i32(MILITARY_PPD_MASTER_STATE_OFFSET)
    }

    pub fn set_master_state(&mut self, value: i32) {
        self.write_military_i32(MILITARY_PPD_MASTER_STATE_OFFSET, value);
    }

    /// C `military_ppd::current_advisor` (`military.h:31`, "re-using
    /// storage ID" per its own comment): the advisor NPC's `storage_ID`
    /// most recently talked to.
    pub fn current_advisor(&self) -> i32 {
        self.read_military_i32(MILITARY_PPD_CURRENT_ADVISOR_OFFSET)
    }

    pub fn set_current_advisor(&mut self, value: i32) {
        self.write_military_i32(MILITARY_PPD_CURRENT_ADVISOR_OFFSET, value);
    }

    /// C `military_ppd::advisor_state` (`military.h:32`): the Military
    /// Advisor driver's own dialogue-state machine (`offer_favor`'s
    /// `2`=awaiting-payment state).
    pub fn advisor_state(&self) -> i32 {
        self.read_military_i32(MILITARY_PPD_ADVISOR_STATE_OFFSET)
    }

    pub fn set_advisor_state(&mut self, value: i32) {
        self.write_military_i32(MILITARY_PPD_ADVISOR_STATE_OFFSET, value);
    }

    /// C `military_ppd::advisor_cost` (`military.h:33`): the gold cost
    /// (100 = 1G) of the favor/specific-mission request currently
    /// awaiting payment.
    pub fn advisor_cost(&self) -> i32 {
        self.read_military_i32(MILITARY_PPD_ADVISOR_COST_OFFSET)
    }

    pub fn set_advisor_cost(&mut self, value: i32) {
        self.write_military_i32(MILITARY_PPD_ADVISOR_COST_OFFSET, value);
    }

    /// C `military_ppd::advisor_storage_nr` (`military.h:34`): the favor
    /// size (`0..=4`, small/medium/big/huge/vast) currently awaiting
    /// payment.
    pub fn advisor_storage_nr(&self) -> i32 {
        self.read_military_i32(MILITARY_PPD_ADVISOR_STORAGE_NR_OFFSET)
    }

    pub fn set_advisor_storage_nr(&mut self, value: i32) {
        self.write_military_i32(MILITARY_PPD_ADVISOR_STORAGE_NR_OFFSET, value);
    }

    /// C `military_ppd::military_pts` (`military.h:39`): "exp gained
    /// towards ranks" per its own comment - the rank-cubed-floored value
    /// `generate_mission_with_preference` feeds into every mission
    /// generator as their shared difficulty-scaling input (distinct from
    /// `Character.military_points`, the real promotion-rank score).
    pub fn military_pts(&self) -> i32 {
        self.read_military_i32(MILITARY_PPD_MILITARY_PTS_OFFSET)
    }

    pub fn set_military_pts(&mut self, value: i32) {
        self.write_military_i32(MILITARY_PPD_MILITARY_PTS_OFFSET, value);
    }

    /// C `military_ppd::normal_exp` (`military.h:40`): exp given out.
    pub fn military_normal_exp_ppd(&self) -> i32 {
        self.read_military_i32(MILITARY_PPD_NORMAL_EXP_OFFSET)
    }

    pub fn set_military_normal_exp_ppd(&mut self, value: i32) {
        self.write_military_i32(MILITARY_PPD_NORMAL_EXP_OFFSET, value);
    }

    /// C `military_ppd::recommend` (`military.h:53`): "to remember if we
    /// mentioned a recommendation already" per its own comment - stamped
    /// `yday + 1` by `process_advisor_recommendation`/`process_clan_
    /// recommendation`.
    pub fn military_recommend(&self) -> i32 {
        self.read_military_i32(MILITARY_PPD_RECOMMEND_OFFSET)
    }

    pub fn set_military_recommend(&mut self, value: i32) {
        self.write_military_i32(MILITARY_PPD_RECOMMEND_OFFSET, value);
    }

    /// C `military_ppd::temp_mission_type` (`military.h:56`): "New
    /// temporary fields for mission selection before payment" per the
    /// struct's own comment.
    pub fn temp_mission_type(&self) -> i32 {
        self.read_military_i32(MILITARY_PPD_TEMP_MISSION_TYPE_OFFSET)
    }

    pub fn set_temp_mission_type(&mut self, value: i32) {
        self.write_military_i32(MILITARY_PPD_TEMP_MISSION_TYPE_OFFSET, value);
    }

    /// C `military_ppd::temp_mission_difficulty` (`military.h:57`).
    pub fn temp_mission_difficulty(&self) -> i32 {
        self.read_military_i32(MILITARY_PPD_TEMP_MISSION_DIFFICULTY_OFFSET)
    }

    pub fn set_temp_mission_difficulty(&mut self, value: i32) {
        self.write_military_i32(MILITARY_PPD_TEMP_MISSION_DIFFICULTY_OFFSET, value);
    }

    /// C `generate_mission_with_preference(cn, ppd, preferred_type)`
    /// (`military.c:1036-1131`)'s ppd-mutating half: builds the 5-slot
    /// mission offer table via [`crate::world::generate_mission_with_
    /// preference`] (reading this ppd's own stored `mission_difficulty_
    /// preference`), writes every slot into `mis[]`, and stamps `mission_
    /// type_preference`/`mission_yday` (`yday + 1`).
    ///
    /// C computes `military_pts` (the rank-cubed floor against `get_army_
    /// rank_int(cn)`) and `level` (`ch[cn].level`, floored to 7) itself
    /// before calling this - callers here must resolve those from
    /// `Character` (unreachable from this session-only struct, see this
    /// file's module doc) and pass them in; `level`'s own `max(7)` floor
    /// is still applied internally, matching C exactly. `yday` is C's
    /// global `yday` (`World.date.yday`). REMAINING: no Rust call site
    /// yet invokes this - it needs the Military Master/Advisor NPC driver
    /// (unported, see `PORTING_TODO.md`'s "Military ranks" entry).
    pub fn apply_mission_offer(
        &mut self,
        level: i32,
        military_pts: i32,
        preferred_type: i32,
        yday: i32,
        rng_seed: &mut u32,
    ) {
        let missions = crate::world::generate_mission_with_preference(
            level,
            military_pts,
            preferred_type,
            self.mission_difficulty_preference(),
            rng_seed,
        );
        for (idx, mission) in missions.into_iter().enumerate() {
            self.set_military_mission(idx, mission);
        }
        self.set_mission_type_preference(preferred_type);
        self.set_mission_yday(yday + 1);
    }

    /// C `check_military_solve(cn, co)` (`src/system/death.c:290-383`):
    /// fired from `kill_char` whenever a player (`cn`, `self` here) kills
    /// anything (`co`), decrementing the active mission's remaining-kill
    /// count (`mis[nr].opt1`) if the victim's class/level matches the
    /// mission's type/target, and flipping `solved_mission` once it
    /// reaches 0. `victim_class`/`victim_level` are C's `ch[co].class`/
    /// `ch[co].level`. A no-op ([`crate::world::MilitaryMissionProgress::
    /// NoMatch`]) if there's no active unsolved mission (`!ppd->took_
    /// mission || ppd->solved_mission`) or the victim doesn't match the
    /// active mission's type/class/level target - C's outer `if` plus
    /// `switch` both have no `else`/`default` branch, so every mismatch
    /// silently does nothing.
    pub fn check_military_solve(
        &mut self,
        victim_class: i32,
        victim_level: i32,
    ) -> crate::world::MilitaryMissionProgress {
        use crate::world::{
            get_demon_mission_value, is_pent_demon_mission_class, is_sewer_ratling_mission_class,
            MilitaryMissionProgress, MISSION_TYPE_DEMON, MISSION_TYPE_RATLING,
        };

        let took_mission = self.military_took_mission();
        if took_mission == 0 || self.military_solved_mission() {
            return MilitaryMissionProgress::NoMatch;
        }
        let nr = (took_mission - 1) as usize;
        if nr >= MILITARY_PPD_MISSION_COUNT {
            return MilitaryMissionProgress::NoMatch;
        }
        let mission = self.military_mission(nr);

        let level_matches = |target_level: i32| {
            victim_level == target_level
                || victim_level == target_level - 1
                || victim_level == target_level + 1
        };

        let elite_count = match mission.mission_type {
            MISSION_TYPE_DEMON => {
                if !is_pent_demon_mission_class(victim_class) || !level_matches(mission.opt2) {
                    return MilitaryMissionProgress::NoMatch;
                }
                get_demon_mission_value(victim_class)
            }
            MISSION_TYPE_RATLING => {
                if !is_sewer_ratling_mission_class(victim_class) || !level_matches(mission.opt2) {
                    return MilitaryMissionProgress::NoMatch;
                }
                1
            }
            _ => return MilitaryMissionProgress::NoMatch,
        };

        let remaining = (mission.opt1 - elite_count).max(0);
        self.set_military_mission_opt1(nr, remaining);
        if remaining == 0 {
            self.set_military_solved_mission(true);
            MilitaryMissionProgress::Solved
        } else {
            MilitaryMissionProgress::Progress {
                remaining,
                elite_count,
            }
        }
    }

    /// C `check_military_silver(cn, amount)` (`src/area/12/mine.c:102-
    /// 134`): fired from the mine-wall reward cascade whenever a silver
    /// or gold find is granted (gold counts double, see the caller),
    /// decrementing the active mission's remaining silver requirement
    /// (`mis[nr].opt1`) if it's a silver-type mission, flipping
    /// `solved_mission` once the requirement is met. Unlike
    /// [`Self::check_military_solve`], C calls `sendquestlog`
    /// unconditionally whenever there's *any* active unsolved mission
    /// (even a non-silver one) - see [`MilitaryMissionSilverProgress::
    /// NotSilverMission`]'s doc comment; callers should resend the
    /// questlog display for every non-`NoMission` outcome.
    pub fn check_military_silver(
        &mut self,
        amount: i32,
    ) -> crate::world::MilitaryMissionSilverProgress {
        use crate::world::{MilitaryMissionSilverProgress, MISSION_TYPE_SILVER};

        let took_mission = self.military_took_mission();
        if took_mission == 0 || self.military_solved_mission() {
            return MilitaryMissionSilverProgress::NoMission;
        }
        let nr = (took_mission - 1) as usize;
        if nr >= MILITARY_PPD_MISSION_COUNT {
            return MilitaryMissionSilverProgress::NoMission;
        }
        let mission = self.military_mission(nr);
        if mission.mission_type != MISSION_TYPE_SILVER {
            return MilitaryMissionSilverProgress::NotSilverMission;
        }

        if amount < mission.opt1 {
            let remaining = mission.opt1 - amount;
            self.set_military_mission_opt1(nr, remaining);
            MilitaryMissionSilverProgress::Progress { remaining }
        } else {
            self.set_military_solved_mission(true);
            self.set_military_mission_opt1(nr, 0);
            MilitaryMissionSilverProgress::Solved
        }
    }

    /// C `accept_mission(cn, co, difficulty, ppd, dat)`
    /// (`military.c:1300-1341`)'s ppd-mutating half. `difficulty` is
    /// `0..=4` (C's only call sites, `military.c:1996-2015`, always pass a
    /// literal `0`-`4` - this trusts the caller the same way
    /// `military_mission`/`check_military_solve` already do). `yday` is
    /// C's global `yday` (`World.date.yday`). Skips `dat->storage_data.
    /// quests_given[difficulty]++` (the NPC-scoped mission-offer counter -
    /// this method has no `World`/`master_id` access; the caller should
    /// invoke `crate::world::World::record_mission_offered` itself on
    /// `Accepted`, see that function's doc comment) and the `say()` text
    /// itself - the caller renders
    /// `crate::world::AcceptMissionOutcome` into the exact wording once
    /// the Military Master NPC driver lands (see `crate::world::military`'s
    /// module doc).
    pub fn accept_mission(
        &mut self,
        difficulty: usize,
        yday: i32,
    ) -> crate::world::AcceptMissionOutcome {
        use crate::world::AcceptMissionOutcome;

        if self.military_took_mission() != 0 {
            return AcceptMissionOutcome::AlreadyHasMission;
        }
        if self.military_solved_yday() == yday + 1 {
            return AcceptMissionOutcome::AlreadyCompletedToday;
        }
        if self.mission_yday() != yday + 1 {
            return AcceptMissionOutcome::MissionsNotOfferedToday;
        }

        let is_advisor_mission = self.mission_type_preference() > 0
            && self.mission_difficulty_preference() == difficulty as i32;
        if difficulty >= MILITARY_PPD_MISSION_COUNT {
            return AcceptMissionOutcome::MissionUnavailable;
        }
        let mission = self.military_mission(difficulty);
        if !is_advisor_mission && difficulty > 0 && mission.pts > self.military_current_pts() {
            return AcceptMissionOutcome::InsufficientPoints;
        }
        if mission.is_empty() {
            return AcceptMissionOutcome::MissionUnavailable;
        }

        self.set_military_took_mission(difficulty as i32 + 1);
        self.set_military_took_yday(yday + 1);
        if difficulty > 0 && !is_advisor_mission {
            self.set_military_current_pts(self.military_current_pts() - mission.pts);
        }
        self.set_mission_type_preference(0);
        self.set_mission_difficulty_preference(-1);

        AcceptMissionOutcome::Accepted(mission)
    }

    /// C `greet_player(cn, co, ppd)` (`military.c:1764-1798`): the
    /// Military Master driver's own `NT_CHAR`-triggered dialogue-state
    /// machine, deciding what to say (if anything) the first time a
    /// player comes into view this visit. `has_army_rank` is C's
    /// `get_army_rank_int(co)` (nonzero = enrolled). `yday` is C's global
    /// `yday` (`World.date.yday`). The stale confirmation state (`10`,
    /// left behind by an interrupted `handle_mission_reroll` from a
    /// previous visit) is reset to `0` first, exactly like C - note this
    /// means a stale `10` always falls through to the rest of the
    /// function afresh (C's guard is `if (ppd->master_state != 0) return;`,
    /// checked *after* the reset, not `else`).
    pub fn greet_player(
        &mut self,
        has_army_rank: bool,
        yday: i32,
    ) -> crate::world::GreetPlayerOutcome {
        use crate::world::GreetPlayerOutcome;

        if self.master_state() == 10 {
            self.set_master_state(0);
        }
        if self.master_state() != 0 {
            return GreetPlayerOutcome::AlreadyGreeted;
        }
        if self.military_recommend() == yday + 1
            && self.mission_type_preference() > 0
            && self.mission_difficulty_preference() >= 0
        {
            self.set_master_state(2);
            return GreetPlayerOutcome::AdvisorRecommendationAlreadyShown;
        }

        if self.military_took_mission() != 0 {
            self.set_master_state(2);
            GreetPlayerOutcome::HasActiveMission
        } else if self.military_solved_yday() == yday + 1 {
            self.set_master_state(2);
            GreetPlayerOutcome::AlreadyCompletedToday
        } else if has_army_rank {
            self.set_master_state(2);
            GreetPlayerOutcome::HasRank
        } else {
            self.set_master_state(1);
            GreetPlayerOutcome::NewPlayer
        }
    }
}
