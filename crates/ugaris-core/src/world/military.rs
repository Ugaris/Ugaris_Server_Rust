//! Imperial Army rank thresholds - C `src/module/military.c` +
//! `src/system/tool.c`'s shared point-award/promotion helper
//! (`give_military_pts`/`give_military_pts_no_npc`, `tool.c:3249-3306`) -
//! plus the pure mission-generation math (pricing, exp-reward scaling,
//! and the three per-type single-mission generators) from
//! `src/module/military.c:342-1027`.
//!
//! This second slice ports every *pure* function military.c uses to build
//! a mission offer: [`SingleMission`] (`struct single_mission`),
//! [`specific_mission_price`] (the paid-advisor-recommendation price
//! formula), the five level/rank scaling helpers behind
//! [`calculate_mission_exp`] (`get_level_experience_cap`/
//! `get_minimum_expected_rank`/`get_maximum_reasonable_rank`/
//! `get_expected_level_for_rank`/`get_enhanced_level_scaling_factor`),
//! and [`generate_single_demon_mission`]/[`generate_single_ratling_mission`]/
//! [`generate_single_silver_mission`] (the per-difficulty mission-instance
//! generators for each of the 3 mission types). None of this is wired to
//! any character/NPC state yet - it's exercised purely by value in/value
//! out, same as this file's first slice.
//!
//! This third slice ports the `military_ppd` per-character save data's
//! mission-progress fields (`PlayerRuntime::military_ppd`, see
//! `player.rs`'s `LEGACY_MILITARY_PPD_SIZE`/`military_mission`/
//! `military_took_mission`/`military_solved_mission`) and, on top of
//! that, `check_military_solve` (`death.c:290-383`, the kill-progress
//! decrement) as `PlayerRuntime::check_military_solve` - the pure state
//! mutation plus [`MilitaryMissionProgress`] outcome, this file's
//! [`get_demon_mission_value`]/[`is_pent_demon_mission_class`]/
//! [`is_sewer_ratling_mission_class`]/
//! [`military_mission_progress_message_should_display`] helpers, and the
//! `[World::kill_character_followup]` call site
//! (`MilitaryMissionKillCheck`, queued right where `FirstKillCheck` is,
//! drained and applied by `ugaris-server`'s
//! `apply_military_mission_kill_check`, `crates/ugaris-server/src/
//! military.rs`, sending the exact `COL_DARK_GRAY "Mission kill, %d to
//! go."` / `"Elite demon slain! ..."` / `"You solved your mission. ..."`
//! `log_char` text). `military_ppd`'s remaining fields at that point
//! (advisor state, `mission_yday`/`took_yday`/`solved_yday`, `recommend`,
//! mission type/difficulty preference, temp mission selection,
//! `reroll_yday`) still round-tripped as opaque bytes.
//!
//! This fourth slice ports the ppd-populating mission-offer wrappers on
//! top of the previous slices' per-instance generators:
//! [`generate_demon_mission`]/[`generate_sewer_mission`]/
//! [`generate_mine_mission`]/[`generate_mission_with_preference`]/
//! [`generate_mission`] (`military.c:847-1139`), all pure functions over
//! an already rank-cubed-floored `military_pts` and a raw level
//! (internally `max(7)`-floored, matching C). `player.rs` gained the 3
//! remaining ppd accessors these need (`mission_type_preference`/
//! `mission_difficulty_preference`/`mission_yday`) plus
//! `PlayerRuntime::apply_mission_offer`, the ppd-mutating wrapper that
//! actually writes the generated offer table into `mis[]` and stamps
//! `mission_type_preference`/`mission_yday`.
//!
//! This fifth slice ports `accept_mission`/`complete_mission`
//! (`military.c:1300-1436`), the remaining ppd-mutating state
//! transitions: [`crate::PlayerRuntime::accept_mission`] (pure ppd
//! mutation, [`AcceptMissionOutcome`]) and [`World::complete_mission`]
//! (ppd + `Character` mutation - exp/gold/military-points award, rank
//! promotion, [`CompleteMissionResult`]/[`CompletedMission`]). `player.rs`
//! gained the 3 remaining ppd accessors these need (`current_pts`/
//! `took_yday`/`solved_yday`).
//!
//! REMAINING (unported, needs the above): resolving the rank-cubed-
//! floored `military_pts`/level-7-floored level/current `yday` from
//! `Character`/`World` and actually calling `apply_mission_offer`/
//! `accept_mission`/`complete_mission` from a real driver (no real call
//! site yet for any of the three - needs the Military Master/Advisor NPC
//! drivers); those drivers themselves and their `qa[]` dialogue table
//! (`analyse_text_driver`), storage state machines (`process_master_
//! storage`/`process_advisor_storage`) and the `dat->storage_data`
//! quests-given/quests-solved/pts-given/exp-given per-difficulty counters
//! they own (no Rust `military_master_data` equivalent yet);
//! `handle_specific_mission_request` (the paid-advisor-recommendation
//! flow, `military.c:481-580`); `military_ppd`'s advisor-state/
//! `recommend`/temp-mission-selection/`reroll_yday` fields (still opaque
//! bytes, no accessors yet); the wealth-achievement ladder `give_money`
//! also updates on `complete_mission`'s mercenary gold bonus (needs the
//! DB-backed first-unlock announce, which lives in the server crate -
//! wire `ugaris_core::achievement::add_gold_earned` at the same time a
//! real driver call site lands); and the `SV_QUEST_EXT` mod-packet
//! (`mod_send_questlog_ext`, `common/mod_packet.c:351-397`) that shows the
//! active mission in the client's quest log (the `sendquestlog` calls
//! inside `check_military_solve`/`complete_mission` themselves are
//! consequently also not reproduced yet - a cosmetic gap only, since the
//! mission-progress state itself is already correct).
//!
//! This sixth slice ports `CDR_MILITARY_MASTER`'s own driver
//! (`military_master_driver`, `military.c:2108-2206`), the first real
//! call site for every function the previous slices left dangling:
//! [`crate::character_driver::MILITARY_QA`] (the 44-row `qa[]` table,
//! shared with the still-unported Advisor driver), `analyse_text_driver`
//! (reused as [`crate::character_driver::analyse_text_qa`], same as every
//! other qa-table NPC), [`World::handle_mission_request`] (C
//! `handle_mission_request`, `military.c:1842-1896` - the "mission"
//! keyword handler, newly ported here since nothing else needed it
//! before), [`describe_mission_text`]/[`display_mission_text`]/
//! [`offer_missions_text`] (C `describe_mission`/`display_mission`/
//! `offer_missions`, `military.c:1194-1246` - the mission-rendering text,
//! newly ported here too), and finally real call sites for
//! [`crate::PlayerRuntime::greet_player`], [`crate::PlayerRuntime::
//! accept_mission`], [`World::complete_mission`], and [`World::
//! mission_reroll`].
//!
//! Like `world/bank.rs`, `World` cannot reach `PlayerRuntime` (where
//! `military_ppd` and every mission-progress field actually live), so
//! essentially the entire message-handling body is deferred as a
//! [`MilitaryMasterEvent`] and applied by `ugaris-server`'s
//! `apply_military_master_events` (mirroring `apply_bank_events`'s
//! shape) - this is a wider deferral than bank's (which only deferred the
//! persistent-balance mutation) because nearly every branch of
//! `military_master_driver` touches `military_ppd`. `NT_CHAR` is ported
//! as the same periodic nearby-player-scan simplification `world/bank.rs`/
//! `world/merchant.rs` already established, queuing a `NearbyPlayer`
//! event for every player in range every tick rather than reacting to a
//! one-shot notify-area broadcast - safe here because `greet_player`'s own
//! `master_state` gate (and `complete_mission`'s own `solved_mission`
//! gate) already make repeated per-tick delivery a no-op once handled,
//! matching C's own steady-state behavior (C's `military_master_driver`
//! runs the identical `process_clan_recommendation`/`process_advisor_
//! recommendation`/`greet_player`/`complete_mission` sequence on every
//! incoming `NT_CHAR` message with no additional throttling of its own).
//!
//! Deliberately out of scope for this slice (documented here, not
//! silently dropped - see the "Military ranks" task in
//! `PORTING_TODO.md`):
//! - `process_clan_recommendation`/`process_advisor_recommendation` (need
//!   `military_master_data.storage_data.clan_pts[]`/`ppd->recommend` -
//!   the NPC-scoped clan-points ledger has no Rust storage-blob
//!   equivalent yet, same architectural gap the Arena rankings task's
//!   REMAINING note flags).
//! - The admin-only qa codes 18-21 (`info`/`reset`/`raise`/`promote`) -
//!   `info` additionally needs the same unported storage-blob counters;
//!   `/milinfo`/`/milpoints`/`/milstats` already cover this NPC's
//!   admin-facing needs via other means.
//! - `update_clan_points`/`process_master_storage` (the periodic
//!   `military_master_data.storage_data` persistence tick) - no Rust
//!   `military_master_data` equivalent exists (see above).
//! - The Military Advisor NPC (`CDR_MILITARY_ADVISOR`) entirely -
//!   `handle_specific_mission_request`/`offer_favor`/`process_favor_
//!   payment`/`handle_advisor_message`/`military_advisor_driver` and the
//!   advisor-recommendation qa codes 30-44 remain unported.
//! - `complete_mission`'s own reward text still goes through
//!   [`World::queue_system_text`]/[`World::queue_system_text_bytes`]
//!   rather than [`World::npc_quiet_say`] from the Master NPC (a
//!   pre-existing simplification from the fifth slice, documented on
//!   that function itself, not tightened here to avoid touching its
//!   already-tested behavior).
//!
//! This eighth slice closes the first bullet above:
//! [`World::process_clan_recommendation`]/[`World::update_clan_points`]
//! (`military.c:1654-1674,1815-1832`) plus the in-memory-only
//! [`MilitaryMasterStorage`]/[`MilitaryMasterStorageRegistry`] data model
//! they need (`struct military_master_storage`'s `clan_pts[MAXCLAN]` and
//! the 4 quest counters, which have no other call site yet), and
//! [`crate::character_driver::MilitaryMasterDriverData`] gained the two
//! `dat`-scoped runtime fields (`last_clan_update`/`last_recom`) both
//! functions need. Wired into a real call site: `ugaris-server`'s
//! `apply_military_master_nearby_player` now calls `process_clan_
//! recommendation` immediately before `process_advisor_recommendation`,
//! matching C's own call order exactly, and
//! [`World::process_military_master_actions`] now calls `update_clan_
//! points` once per NPC per tick (a new `now: i64` parameter, mirroring
//! `process_clanmaster_actions`' own shape).
//!
//! Still out of scope (documented on [`MilitaryMasterStorageRegistry`]'s
//! own doc comment): DB persistence for the registry (in-memory only,
//! resets on restart), the admin-only qa codes 19-21 (`reset`/`raise`/
//! `promote` don't touch storage at all and remain unported only because
//! no one has ported the admin-facing qa codes yet).
//!
//! This ninth slice closes the Advisor driver's own `struct cost_data`
//! sales-economy gap: [`CostData`]/[`MilitaryAdvisorStorage`]/
//! [`MilitaryAdvisorStorageRegistry`] (mirroring [`MilitaryMasterStorage`]/
//! [`MilitaryMasterStorageRegistry`]'s shape - in-memory only, no DB
//! persistence yet), `add_cost` wired into [`World::process_favor_
//! payment`], and the Advisor's own admin-only qa code 18 (`info`,
//! `military.c:2525-2538`) as [`MilitaryAdvisorEvent::Info`], applied by
//! `ugaris-server`'s `apply_military_advisor_info` (mirrors
//! `apply_military_master_info`'s shape). The `amount[20]`/`date[20]`
//! rolling sale-history window and `created` timestamp are not ported -
//! see [`CostData`]'s doc comment for why (they only ever fed the
//! never-called `calc_cost`). `update_advisor_storage`/`process_advisor_
//! storage` (the periodic async-DB-blob persistence state machine) remain
//! unported for the same reason [`MilitaryMasterStorageRegistry`]'s own
//! `process_master_storage` was never ported: the in-memory registry
//! supersedes the state machine entirely.

use super::*;
use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, MILITARY_QA};

/// C `military.h:12`'s `MAX_ARMY_RANK`.
pub const MAX_ARMY_RANK: i32 = 40;

/// C `tool.c:1868-1907`'s static `rankname[]` table (index 0..=40, letter
/// for letter).
pub const ARMY_RANK_NAMES: [&str; 41] = [
    "nobody",
    "Private",
    "Private First Class",
    "Lance Corporal",
    "Corporal",
    "Sergeant",
    "Staff Sergeant",
    "Master Sergeant",
    "First Sergeant",
    "Sergeant Major",
    "Second Lieutenant",
    "First Lieutenant",
    "Captain",
    "Major",
    "Lieutenant Colonel",
    "Colonel",
    "Brigadier General",
    "Major General",
    "Lieutenant General",
    "General",
    "Field Marshal",
    "Knight of Astonia",
    "Baron of Astonia",
    "Earl of Astonia",
    "Warlord of Astonia",
    "Duke of Astonia",
    "Archduke of Astonia",
    "Prince of Astonia",
    "High Prince of Astonia",
    "Royal Guardian",
    "Slayer of Demons",
    "Astonian Champion",
    "Defender of the Realm",
    "Sword of Astonia",
    "Shield of the Kingdom",
    "Legendary Warrior",
    "Immortal Guardian",
    "Hero of Ages",
    "Mythic Protector",
    "Eternal Champion",
    "Avatar of Astonia",
];

/// C `get_army_rank_int`/`set_army_rank` (`tool.c:2011-2035`): the current
/// rank is `cbrt(military_pts)`, clamped to `[0, MAX_ARMY_RANK]`.
///
/// C persists this as a separate `DRD_RANK_PPD` field, only ever written
/// by [`World::give_military_pts`]'s two C forms (`give_military_pts`/
/// `give_military_pts_no_npc`, `tool.c:3249-3306` - the *only* two
/// `set_army_rank` call sites in the entire C tree, both computing
/// exactly this formula). Since nothing else ever desyncs the persisted
/// value from the formula, rank is derived on the fly from
/// `Character.military_points` here instead of adding a second persisted
/// field - behaviorally identical for every real call site.
///
/// One narrow C quirk this intentionally does NOT reproduce: C's
/// `rank < (MAX_ARMY_RANK + 1)` promotion guard means a single point
/// grant large enough to jump the raw cube root past 41 in one step
/// would leave C's persisted rank frozen below what the formula says -
/// clearly an off-by-one accident (the guard was evidently meant to just
/// cap at `MAX_ARMY_RANK`, which `set_army_rank`'s own `min(...)` already
/// does), not intended design, and unreachable in practice short of a
/// deliberately huge single admin `/milexp` grant.
pub fn army_rank_for_points(military_points: i32) -> i32 {
    if military_points <= 0 {
        return 0;
    }
    let raw_rank = f64::from(military_points).cbrt() as i32;
    raw_rank.clamp(0, MAX_ARMY_RANK)
}

/// C `get_army_rank_string` (`tool.c:2037-2045`).
pub fn army_rank_name(rank: i32) -> &'static str {
    ARMY_RANK_NAMES[rank.clamp(0, MAX_ARMY_RANK) as usize]
}

/// Outcome of [`crate::PlayerRuntime::greet_player`] (C `greet_player`,
/// `military.c:1764-1798`), mirroring every distinct `say()` branch (plus
/// the silent no-op ones).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GreetPlayerOutcome {
    /// C: `ppd->master_state != 0` (after the stale-`10` reset) - already
    /// greeted this visit, no text.
    AlreadyGreeted,
    /// C: an advisor's specific-mission recommendation already rendered
    /// the greeting text this visit (`process_advisor_recommendation`,
    /// still unported) - no additional text here, just the `master_state
    /// = 2` stamp.
    AdvisorRecommendationAlreadyShown,
    /// C: `ppd->took_mission` nonzero -> "Ah, hello %s. Any luck with
    /// your mission? Or would you like to hear it again? Or have you
    /// failed to complete it?".
    HasActiveMission,
    /// C: `ppd->solved_yday == yday + 1` -> "I don't have another
    /// mission for you today, %s.".
    AlreadyCompletedToday,
    /// C: `get_army_rank_int(co)` nonzero -> "Hello, %s. I might have a
    /// mission for you. If you don't like the available missions, you
    /// can reroll for 200 gold.".
    HasRank,
    /// C: none of the above -> "Greetings, %s.".
    NewPlayer,
}

/// Outcome of [`World::give_military_pts`]: lets callers observe whether a
/// promotion happened without re-deriving the rank themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MilitaryPointsAward {
    pub old_rank: i32,
    pub new_rank: i32,
}

impl MilitaryPointsAward {
    pub fn promoted(&self) -> bool {
        self.new_rank > self.old_rank
    }
}

impl World {
    /// C `give_military_pts_no_npc(co, pts, exps)` (`tool.c:3279-3306`):
    /// awards `exps` via the shared [`World::give_exp`] (already applies
    /// the hardcore normal-exp bonus + `exp_modifier`), records the raw
    /// `exps` onto `Character.military_normal_exp` (C's `ppd->normal_exp
    /// += exps`, independent of whatever bonus `give_exp` applied to the
    /// real exp total), applies the *military*-specific hardcore bonus
    /// (`hardcore_military_exp_bonus`, distinct from the normal-exp one)
    /// to `pts` before adding it to `Character.military_points`, and - if
    /// the resulting rank increased - queues the "You've been promoted to
    /// X. Congratulations, NAME!" system-text feedback plus, for ranks
    /// above Sergeant Major (index 9), the server-wide "Grats: NAME is a
    /// X now!" channel-6 broadcast, matching C exactly.
    ///
    /// This is the "no_npc" C variant - the only one with a live Rust
    /// call site today (`/milexp` and the Area 25 `warpbonus_driver`
    /// reward). C's other form, `give_military_pts` (says the promotion
    /// line to a specific NPC-driven `cn` via `say` instead of straight to
    /// the target), has no C call site outside the still-unported
    /// mission-advisor driver; port it alongside that driver, reusing
    /// this function's point/rank math.
    pub fn give_military_pts(
        &mut self,
        character_id: CharacterId,
        pts: i32,
        exps: i32,
        area_id: u32,
    ) -> MilitaryPointsAward {
        let Some(character) = self.characters.get(&character_id) else {
            return MilitaryPointsAward::default();
        };
        let is_hardcore = character.flags.contains(CharacterFlags::HARDCORE);
        let old_rank = army_rank_for_points(character.military_points);

        self.give_exp(character_id, i64::from(exps), area_id);

        let Some(character) = self.characters.get_mut(&character_id) else {
            return MilitaryPointsAward::default();
        };
        character.military_normal_exp = character.military_normal_exp.saturating_add(exps);

        let mut awarded_pts = pts;
        if is_hardcore {
            awarded_pts = (f64::from(pts) * self.settings.hardcore_military_exp_bonus) as i32;
        }
        character.military_points = character.military_points.saturating_add(awarded_pts);
        character.flags.insert(CharacterFlags::UPDATE);
        let name = character.name.clone();
        let new_rank = army_rank_for_points(character.military_points);

        if new_rank > old_rank {
            self.queue_system_text(
                character_id,
                format!(
                    "You've been promoted to {}. Congratulations, {}!",
                    army_rank_name(new_rank),
                    name
                ),
            );
            if new_rank > 9 {
                let mut broadcast = b"0000000000".to_vec();
                broadcast.extend_from_slice(crate::text::COL_CHAT_GRATS);
                broadcast.extend_from_slice(
                    format!("Grats: {name} is a {} now!", army_rank_name(new_rank)).as_bytes(),
                );
                self.queue_channel_broadcast(6, broadcast);
            }
        }

        MilitaryPointsAward { old_rank, new_rank }
    }
}

/// C `military.h:19`'s `MAX_MISSION_EXP_PERCENTAGE`.
const MAX_MISSION_EXP_PERCENTAGE: i64 = 15;

/// Mission type discriminants (`military.c`'s own comments on
/// `struct single_mission::type`: "1: Pent mission; 2: Ratling mission; 3:
/// Silver mission").
pub const MISSION_TYPE_DEMON: i32 = 1;
pub const MISSION_TYPE_RATLING: i32 = 2;
pub const MISSION_TYPE_SILVER: i32 = 3;

/// C `military.h:21-26`'s `struct single_mission`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SingleMission {
    pub mission_type: i32,
    pub opt1: i32,
    pub opt2: i32,
    pub pts: i32,
    pub exp: i32,
}

impl SingleMission {
    /// C's own `mission.type == 0` "no mission" convention (used by every
    /// caller of the per-type generators below to detect an
    /// unavailable/level-gated mission).
    pub fn is_empty(&self) -> bool {
        self.mission_type == 0
    }
}

/// One draw from C's `RANDOM(a)` macro (`#define RANDOM(a) (rand() % (a))`,
/// `server.h:30`), seeded so callers can get deterministic results in
/// tests. `below` must be positive; C's own callers never pass 0 here.
fn mission_random(seed: &mut u32, below: i32) -> i32 {
    legacy_random_below_from_seed(seed, below.max(1) as u32) as i32
}

/// C `calculate_advisor_index(storage_id)` (`military.c:2231-2244`):
/// maps an Advisor NPC's `storage_ID` (a compact-but-non-contiguous
/// numbering scheme - IDs below 27 count from 7, IDs 27 and above skip a
/// 4-wide gap and count from 31) to a `0..MAXADVISOR` (20) slot index
/// into `military_ppd::advisor_last[]`. Out-of-range results (either
/// branch going negative or `>= MAXADVISOR`) fall back to slot `0`,
/// matching C's own `if (idx < 0 || idx >= MAXADVISOR) idx = 0;` exactly.
pub fn calculate_advisor_index(storage_id: i32) -> usize {
    let idx = if storage_id < 27 {
        storage_id - 7
    } else {
        storage_id - 31 + 3
    };
    if !(0..MILITARY_PPD_MAXADVISOR_I32).contains(&idx) {
        0
    } else {
        idx as usize
    }
}

/// `MAXADVISOR` (`military.h:17`) as `i32`, for [`calculate_advisor_index`]'s
/// range check (the accessor-side constant, [`crate::player::
/// MILITARY_PPD_MAXADVISOR`], is a private-module `usize`).
const MILITARY_PPD_MAXADVISOR_I32: i32 = 20;

/// C `advisor_price(level)` (`military.c:2288-2299`): the base gold price
/// (100 = 1G) an Advisor NPC's "favor" costs before the size multiplier
/// ([`offer_favor_cost`]) is applied, banded by player level.
pub fn advisor_price(level: i32) -> i32 {
    if level < 25 {
        400
    } else if level < 45 {
        800
    } else if level < 65 {
        1200
    } else if level < 85 {
        1500
    } else {
        2000
    }
}

/// C `offer_favor`'s cost calculation (`military.c:2318-2372`): the 5
/// favor sizes (small/medium/big/huge/vast, `favor_size` `0..=4`) each
/// apply a multiplier to [`advisor_price`]'s level-banded base price.
/// Returns `None` for an invalid `favor_size` (C's own `default: return
/// 0;` bail-out).
pub fn offer_favor_cost(level: i32, favor_size: i32) -> Option<i32> {
    let multiplier = match favor_size {
        0 => 1.0,
        1 => 3.0,
        2 => 10.0,
        3 => 20.0,
        4 => 35.0,
        _ => return None,
    };
    Some((f64::from(advisor_price(level)) * multiplier) as i32)
}

/// C `specific_mission_price(level, difficulty, mission_type)`
/// (`military.c:392-467`): the gold price an Advisor NPC quotes for a
/// specific paid mission recommendation.
pub fn specific_mission_price(level: i32, difficulty: i32, mission_type: i32) -> i32 {
    let base_price = (level * level) / 10 + level * 5;

    let difficulty_multiplier: f64 = match difficulty {
        0 => 0.4,
        1 => 0.8,
        2 => 1.0,
        3 => 1.5,
        4 => 1.8,
        _ => 1.0,
    };

    let type_multiplier: f64 = match mission_type {
        1 => 1.0,
        2 => 1.1,
        3 => 1.2,
        _ => 1.0,
    };

    let mut level_scaling = (100.0 / f64::from(level)).min(1.0);
    level_scaling = level_scaling.max(0.5);

    let price = (f64::from(base_price)
        * difficulty_multiplier
        * type_multiplier
        * (1.0 - (1.0 - level_scaling) * 0.5)) as i32;

    let min_price = match difficulty {
        0 => 200,
        1 => 400,
        2 => 800,
        3 => 1500,
        4 => 3000,
        _ => 200,
    };

    price.max(min_price)
}

/// C `offer_favor`'s favor-size name table (`military.c:2373-2378`
/// switch, used both in the offer text and [`ProcessFavorPaymentOutcome`]
/// rendering).
pub fn favor_size_name(favor_size: i32) -> &'static str {
    match favor_size {
        0 => "small",
        1 => "medium",
        2 => "big",
        3 => "huge",
        _ => "vast",
    }
}

/// C `handle_advisor_message`'s admin-only "info" qa code's own
/// `static char *fav_name[5]` (`military.c:2532`). Deliberately **not**
/// the same strings as [`favor_size_name`] - C itself uses "normal" here
/// for index 1 where `offer_favor`'s own switch says "medium", a genuine
/// (if seemingly accidental) inconsistency between the two tables,
/// reproduced verbatim rather than "fixed" to match.
pub const ADVISOR_INFO_FAVOR_NAMES: [&str; 5] = ["small", "normal", "big", "huge", "vast"];

/// C `handle_specific_mission_request`/`process_favor_payment`'s
/// mission-type name table (`military.c:521-533`, `2429-2440`).
pub fn mission_type_name(mission_type: i32) -> &'static str {
    match mission_type {
        1 => "demon-slaying",
        2 => "ratling-hunting",
        3 => "silver-mining",
        _ => "unknown",
    }
}

/// C `adv_introduction` (`military.c:2262-2281`): the Advisor NPC's
/// initial greeting, varying by `dat->storage_ID % 4`.
pub fn adv_introduction_text(storage_id: i32, player_name: &str) -> String {
    let template = match storage_id.rem_euclid(4) {
        0 => {
            "I could do you a favor, {name}, I could mention your name to the military governor \
             of Aston. I'm sure that'd help you get that promotion early!"
        }
        1 => {
            "Say, {name}, would you like to speed up your way up the rank ladder? I could speak \
             to the military governor of Aston if you want me to do you that favor."
        }
        2 => {
            "Not getting promoted as fast as you want, {name}? I could do you the favor of \
             talking to the military governor of Aston about you."
        }
        _ => "Need a favor, {name}?",
    };
    template.replace("{name}", player_name)
}

/// C `adv_favor_desc` (`military.c:2296-2308`): the two-line "favor
/// sizes"/"specific missions" explanation, sent as two separate
/// `quiet_say` calls.
pub fn adv_favor_desc_lines() -> [&'static str; 2] {
    [
        "My favors come in five sizes, small, medium, big, huge and vast.",
        "I can also recommend you for specific missions. Just tell me the difficulty and type \
         like easy demon or insane mining.",
    ]
}

/// [`World::offer_favor`]'s outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfferFavorOutcome {
    /// C: `ppd->advisor_last[idx] == yday + 1` -> "Mentioning your name
    /// twice a day won't accomplish much, %s.".
    AlreadyUsedToday,
    /// C's own `default: return 0;` bail-out for an out-of-range
    /// `favor_size` - unreachable via [`crate::character_driver::
    /// MILITARY_QA`]'s fixed qa-code mapping, ported defensively anyway.
    InvalidFavorSize,
    /// The offer was made: `ppd->advisor_cost`/`advisor_state`/
    /// `advisor_storage_nr` were stamped. `cost` is in gold cents (100 =
    /// 1G).
    Offered { favor_size: i32, cost: i32 },
}

/// [`World::handle_specific_mission_request`]'s outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecificMissionRequestOutcome {
    /// C: `ppd->advisor_last[idx] == yday + 1` -> "I've already used my
    /// influence for you today, %s. Come back tomorrow.".
    AlreadyUsedToday,
    /// C: `mission_type < 1 || mission_type > 3` -> "I don't know about
    /// that type of mission, %s.".
    InvalidMissionType,
    /// C: `difficulty < 0 || difficulty > 4` -> "I don't know that
    /// difficulty level, %s.".
    InvalidDifficulty,
    /// C: ratling missions need `level` odd in `9..=39` -> "Ratling
    /// missions are only available at odd levels between 9 and 39, %s.".
    RatlingLevelGate,
    /// C: silver missions need `level >= 12` -> "Silver missions are
    /// only available at level 12 and above, %s.".
    SilverLevelGate,
    /// The offer was made: `ppd->advisor_cost`/`advisor_state`/
    /// `advisor_storage_nr`/`temp_mission_type`/`temp_mission_difficulty`
    /// were stamped. `already_completed_today`/`has_active_mission` carry
    /// the two non-terminal warnings C emits *before* the offer text
    /// (both can be true simultaneously, matching C's `if`/`if` - not
    /// `if`/`else if` - chain).
    Offered {
        difficulty: i32,
        mission_type: i32,
        cost: i32,
        already_completed_today: bool,
        has_active_mission: bool,
    },
}

/// [`World::process_favor_payment`]'s outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessFavorPaymentOutcome {
    /// C: `ppd->current_advisor != dat->storage_ID || ppd->advisor_state
    /// != 2` -> "Pay for what? We haven't agreed on anything yet.",
    /// resets `advisor_state = 1`.
    NothingAgreed,
    /// C: `ch[co].gold < ppd->advisor_cost` -> "Alas, you do not have
    /// enough money.".
    InsufficientGold,
    /// C: `ppd->temp_mission_type > 0` branch - a specific mission
    /// recommendation was arranged; `mission_type_preference`/
    /// `mission_difficulty_preference` were stamped and the temp fields
    /// cleared.
    SpecificMissionArranged { mission_type: i32, difficulty: i32 },
    /// C's `else` branch - a plain favor was arranged;
    /// `ppd->current_pts` gained `2 + favor_size * 2`.
    FavorArranged { favor_size: i32 },
}

impl World {
    /// C `offer_favor(cn, co, ppd, idx, favor_size)` (`military.c:2339-
    /// 2382`). The sales-economy `struct cost_data` bookkeeping
    /// `process_favor_payment` records on acceptance is deliberately out
    /// of scope (no Rust storage-blob equivalent yet - see this module's
    /// doc comment); this only stamps the payment-confirmation state.
    pub fn offer_favor(
        &self,
        character_id: CharacterId,
        player: &mut PlayerRuntime,
        idx: usize,
        favor_size: i32,
        yday: i32,
    ) -> OfferFavorOutcome {
        if player.military_advisor_last(idx) == yday + 1 {
            return OfferFavorOutcome::AlreadyUsedToday;
        }
        let Some(character) = self.characters.get(&character_id) else {
            return OfferFavorOutcome::InvalidFavorSize;
        };
        let Some(cost) = offer_favor_cost(character.level as i32, favor_size) else {
            return OfferFavorOutcome::InvalidFavorSize;
        };
        player.set_advisor_cost(cost);
        player.set_advisor_state(2);
        player.set_advisor_storage_nr(favor_size);
        OfferFavorOutcome::Offered { favor_size, cost }
    }

    /// C `handle_specific_mission_request(cn, co, ppd, dat, idx,
    /// difficulty, mission_type)` (`military.c:481-566`).
    pub fn handle_specific_mission_request(
        &self,
        character_id: CharacterId,
        player: &mut PlayerRuntime,
        idx: usize,
        difficulty: i32,
        mission_type: i32,
        yday: i32,
    ) -> SpecificMissionRequestOutcome {
        if player.military_advisor_last(idx) == yday + 1 {
            return SpecificMissionRequestOutcome::AlreadyUsedToday;
        }
        if !(1..=3).contains(&mission_type) {
            return SpecificMissionRequestOutcome::InvalidMissionType;
        }
        if !(0..=4).contains(&difficulty) {
            return SpecificMissionRequestOutcome::InvalidDifficulty;
        }

        let Some(character) = self.characters.get(&character_id) else {
            return SpecificMissionRequestOutcome::InvalidMissionType;
        };
        let level = character.level as i32;

        if mission_type == 2 && (level < 9 || level > 39 || level % 2 == 0) {
            return SpecificMissionRequestOutcome::RatlingLevelGate;
        }
        if mission_type == 3 && level < 12 {
            return SpecificMissionRequestOutcome::SilverLevelGate;
        }

        let already_completed_today = player.military_solved_yday() == yday + 1;
        let has_active_mission = player.military_took_mission() != 0;
        let cost = specific_mission_price(level, difficulty, mission_type);

        player.set_advisor_cost(cost);
        player.set_advisor_state(2);
        player.set_advisor_storage_nr(difficulty);
        player.set_temp_mission_type(mission_type);
        player.set_temp_mission_difficulty(difficulty);

        SpecificMissionRequestOutcome::Offered {
            difficulty,
            mission_type,
            cost,
            already_completed_today,
            has_active_mission,
        }
    }

    /// C `process_favor_payment(cn, co, ppd, dat, idx)` (`military.c:
    /// 2402-2474`). `add_cost(ppd->advisor_cost, dat->storage_data +
    /// ppd->advisor_storage_nr)` is ported via [`MilitaryAdvisorStorageRegistry::
    /// add_cost`]; `update_advisor_storage`'s `storage_state`
    /// bump (the C-only async-DB-blob state machine kickoff) is not
    /// reproduced since the in-memory registry has no such state machine
    /// to kick - see this module's doc comment.
    pub fn process_favor_payment(
        &mut self,
        character_id: CharacterId,
        player: &mut PlayerRuntime,
        idx: usize,
        storage_id: i32,
        yday: i32,
    ) -> ProcessFavorPaymentOutcome {
        if player.current_advisor() != storage_id || player.advisor_state() != 2 {
            player.set_advisor_state(1);
            return ProcessFavorPaymentOutcome::NothingAgreed;
        }

        let Some(character) = self.characters.get_mut(&character_id) else {
            player.set_advisor_state(1);
            return ProcessFavorPaymentOutcome::NothingAgreed;
        };
        let advisor_cost = player.advisor_cost().max(0) as u32;
        if character.gold < advisor_cost {
            return ProcessFavorPaymentOutcome::InsufficientGold;
        }
        character.gold -= advisor_cost;
        character.flags.insert(CharacterFlags::ITEMS);

        self.military_advisor_storage.add_cost(
            storage_id,
            player.advisor_storage_nr().clamp(0, 4) as usize,
            advisor_cost as i32,
        );

        let outcome = if player.temp_mission_type() > 0 {
            let mission_type = player.temp_mission_type();
            let difficulty = player.temp_mission_difficulty();
            player.set_mission_type_preference(mission_type);
            player.set_mission_difficulty_preference(difficulty);
            player.set_temp_mission_type(0);
            player.set_temp_mission_difficulty(-1);
            ProcessFavorPaymentOutcome::SpecificMissionArranged {
                mission_type,
                difficulty,
            }
        } else {
            let favor_size = player.advisor_storage_nr();
            player.set_military_current_pts(player.military_current_pts() + 2 + favor_size * 2);
            ProcessFavorPaymentOutcome::FavorArranged { favor_size }
        };

        player.set_advisor_state(1);
        player.set_military_advisor_last(idx, yday + 1);

        outcome
    }
}

/// C `get_level_experience_cap(player_level)` (`military.c:580-609`): caps
/// a mission's exp reward at 15% of the exp needed to reach the next
/// level, itself clamped to `[1000, 1_000_000]`.
pub fn get_level_experience_cap(level: i32) -> i32 {
    if level <= 0 {
        return 1000;
    }
    if level >= 200 {
        return 100_000;
    }
    let current = i64::from(level2exp(level as u32));
    let next = i64::from(level2exp((level + 1) as u32));
    let exp_to_next_level = next - current;
    let mut cap = (exp_to_next_level * MAX_MISSION_EXP_PERCENTAGE / 100) as i32;
    if cap < 1000 {
        cap = 1000;
    }
    if cap > 1_000_000 {
        cap = 1_000_000;
    }
    cap
}

/// C `get_minimum_expected_rank(player_level)` (`military.c:618-645`).
pub fn get_minimum_expected_rank(level: i32) -> i32 {
    if level <= 15 {
        0
    } else if level <= 25 {
        2
    } else if level <= 35 {
        4
    } else if level <= 50 {
        6
    } else if level <= 65 {
        8
    } else if level <= 80 {
        12
    } else if level <= 100 {
        16
    } else if level <= 150 {
        20
    } else {
        22
    }
}

/// C `get_maximum_reasonable_rank(player_level)` (`military.c:654-681`).
pub fn get_maximum_reasonable_rank(level: i32) -> i32 {
    if level <= 15 {
        3
    } else if level <= 25 {
        6
    } else if level <= 35 {
        9
    } else if level <= 50 {
        12
    } else if level <= 65 {
        16
    } else if level <= 80 {
        18
    } else if level <= 100 {
        20
    } else if level <= 150 {
        30
    } else {
        MAX_ARMY_RANK
    }
}

/// C `get_expected_level_for_rank(rank)` (`military.c:690-725`).
pub fn get_expected_level_for_rank(rank: i32) -> i32 {
    if rank <= 0 {
        7
    } else if rank <= 5 {
        15 + rank * 3
    } else if rank <= 8 {
        30 + (rank - 5) * 5
    } else if rank <= 10 {
        45 + (rank - 8) * 5
    } else if rank <= 20 {
        55 + (rank - 10) * 5
    } else if rank <= 24 {
        105 + (rank - 20) * 5
    } else if rank <= 30 {
        125 + (rank - 24) * 5
    } else if rank <= 35 {
        155 + (rank - 30) * 6
    } else if rank <= 40 {
        185 + (rank - 35) * 3
    } else {
        200
    }
}

/// C `get_enhanced_level_scaling_factor(player_level, military_rank)`
/// (`military.c:734-757`): rewards a player whose level matches their
/// military rank's expected level band, and is neutral (`1.0`) otherwise -
/// including when the rank itself is outside the level's reasonable
/// min/max band (C's own fallback `return 1.0;`).
pub fn get_enhanced_level_scaling_factor(level: i32, military_rank: i32) -> f64 {
    let expected_level = get_expected_level_for_rank(military_rank);
    let min_rank = get_minimum_expected_rank(level);
    let max_rank = get_maximum_reasonable_rank(level);

    if military_rank >= min_rank && military_rank <= max_rank {
        let level_diff = (level - expected_level).abs();
        if level_diff <= 5 {
            1.5
        } else if level_diff <= 10 {
            1.25
        } else if level_diff <= 20 {
            1.1
        } else {
            1.0
        }
    } else {
        1.0
    }
}

/// C `calculate_mission_exp(military_pts, difficulty, player_level)`
/// (`military.c:767-785`): the level-scaled, level-capped exp reward for
/// a mission worth `difficulty_pts` military points. Note `military_rank`
/// here is `cbrt(military_pts)` truncated to `int` *without* clamping to
/// `MAX_ARMY_RANK` - unlike [`army_rank_for_points`], matching C exactly
/// (this is a distinct local variable in the original function, not a
/// call to `get_army_rank_int`).
pub fn calculate_mission_exp(military_pts: i32, difficulty_pts: i32, level: i32) -> i32 {
    let cbrt_val = f64::from(military_pts).cbrt();
    let military_rank = cbrt_val as i32;
    let base_exp = (f64::from(difficulty_pts) * (cbrt_val + 5.0).powi(4) / 16.0) as i32;
    let level_scaling = get_enhanced_level_scaling_factor(level, military_rank);
    let scaled_exp = (f64::from(base_exp) * level_scaling) as i32;
    let level_cap = get_level_experience_cap(level);
    let final_exp = scaled_exp.min(level_cap);
    final_exp.max(1)
}

/// C `generate_single_demon_mission(level, military_pts, difficulty)`
/// (`military.c:795-839`): a demon-slaying mission at the Pentagram Quest
/// (mission type 1), always available regardless of level.
pub fn generate_single_demon_mission(
    level: i32,
    military_pts: i32,
    difficulty: i32,
    rng_seed: &mut u32,
) -> SingleMission {
    let (opt1, opt2, pts) = match difficulty {
        0 => (1 + mission_random(rng_seed, 10), level.min(118), 1),
        1 => (5 + mission_random(rng_seed, 16), level.min(118), 2),
        2 => (25 + mission_random(rng_seed, 76), level.min(118), 4),
        3 => (
            200 + mission_random(rng_seed, 301),
            (level + 1).min(118),
            10,
        ),
        4 => (
            500 + mission_random(rng_seed, 1501),
            (level + 2).min(118),
            25,
        ),
        // C's own `default:` fallback (unreachable with the driver's own
        // 0..=4 difficulty loop, kept for parity).
        _ => (1 + mission_random(rng_seed, 10), level.min(118), 1),
    };
    SingleMission {
        mission_type: MISSION_TYPE_DEMON,
        opt1,
        opt2,
        pts,
        exp: calculate_mission_exp(military_pts, pts, level),
    }
}

/// C `generate_single_ratling_mission(level, military_pts, difficulty)`
/// (`military.c:865-921`): a ratling-slaying mission in the Sewers
/// (mission type 2), only available for odd levels 9..=39 (higher
/// difficulties raise the effective target level, which can push it back
/// out of range - matching C's own `adjusted_level` re-check). Returns an
/// empty (`type == 0`) mission when the level requirement isn't met.
pub fn generate_single_ratling_mission(
    level: i32,
    military_pts: i32,
    difficulty: i32,
    rng_seed: &mut u32,
) -> SingleMission {
    let mut adjusted_level = level;
    if difficulty >= 3 {
        adjusted_level += difficulty - 2;
    }

    if adjusted_level < 9 || adjusted_level > 39 || adjusted_level & 1 == 0 {
        return SingleMission::default();
    }

    let (opt1, pts) = match difficulty {
        0 => (1 + mission_random(rng_seed, 4), 1),
        1 => (5 + mission_random(rng_seed, 6), 2),
        2 => (25 + mission_random(rng_seed, 26), 4),
        3 => (100 + mission_random(rng_seed, 201), 10),
        4 => (200 + mission_random(rng_seed, 501), 25),
        _ => (1 + mission_random(rng_seed, 4), 1),
    };

    SingleMission {
        mission_type: MISSION_TYPE_RATLING,
        opt1,
        opt2: adjusted_level,
        pts,
        exp: calculate_mission_exp(military_pts, pts, level),
    }
}

/// C `generate_single_silver_mission(level, military_pts, difficulty)`
/// (`military.c:951-1007`): a silver-finding mission in the Mine (mission
/// type 3), only available at level 12+ (again re-checked against the
/// difficulty-adjusted level). The silver quantity scales with the
/// player's *unclamped* cube-root military rank, same quirk as
/// [`calculate_mission_exp`]'s own `military_rank`.
pub fn generate_single_silver_mission(
    level: i32,
    military_pts: i32,
    difficulty: i32,
    rng_seed: &mut u32,
) -> SingleMission {
    let mut adjusted_level = level;
    if difficulty >= 3 {
        adjusted_level += difficulty - 2;
    }

    if adjusted_level < 12 {
        return SingleMission::default();
    }

    let rank = f64::from(military_pts).cbrt() as i32;

    let (opt1, pts) = match difficulty {
        0 => (10 + rank * 8 + mission_random(rng_seed, 31 + rank * 5), 1),
        1 => (50 + rank * 20 + mission_random(rng_seed, 51 + rank * 10), 2),
        2 => (
            250 + rank * 60 + mission_random(rng_seed, 251 + rank * 40),
            4,
        ),
        3 => (
            1000 + rank * 200 + mission_random(rng_seed, 1001 + rank * 150),
            10,
        ),
        4 => (
            2000 + rank * 500 + mission_random(rng_seed, 3001 + rank * 600),
            25,
        ),
        _ => (10 + rank * 8 + mission_random(rng_seed, 31 + rank * 5), 1),
    };

    SingleMission {
        mission_type: MISSION_TYPE_SILVER,
        opt1,
        opt2: 0,
        pts,
        exp: calculate_mission_exp(military_pts, pts, level),
    }
}

/// C `generate_demon_mission(level, ppd)` (`military.c:847-861`): fills
/// all 5 offer slots with demon missions, one per difficulty.
pub fn generate_demon_mission(
    level: i32,
    military_pts: i32,
    rng_seed: &mut u32,
) -> [SingleMission; 5] {
    let mut missions = [SingleMission::default(); 5];
    for (difficulty, slot) in missions.iter_mut().enumerate() {
        *slot = generate_single_demon_mission(level, military_pts, difficulty as i32, rng_seed);
    }
    missions
}

/// C `generate_sewer_mission(level, ppd)` (`military.c:930-948`): picks
/// one random difficulty slot (`RANDOM(5)`) and overwrites it with a
/// ratling mission - but only if the level requirement is met (C's own
/// `if (mission.type != 0) ppd->mis[difficulty] = mission;`, mirrored
/// here by returning `None` instead of a slot index/mission pair when the
/// pick is empty).
pub fn generate_sewer_mission(
    level: i32,
    military_pts: i32,
    rng_seed: &mut u32,
) -> Option<(usize, SingleMission)> {
    let difficulty = mission_random(rng_seed, 5) as usize;
    let mission = generate_single_ratling_mission(level, military_pts, difficulty as i32, rng_seed);
    if mission.is_empty() {
        None
    } else {
        Some((difficulty, mission))
    }
}

/// C `generate_mine_mission(level, ppd)` (`military.c:1016-1034`): same
/// random-slot-overwrite shape as [`generate_sewer_mission`], for silver
/// missions.
pub fn generate_mine_mission(
    level: i32,
    military_pts: i32,
    rng_seed: &mut u32,
) -> Option<(usize, SingleMission)> {
    let difficulty = mission_random(rng_seed, 5) as usize;
    let mission = generate_single_silver_mission(level, military_pts, difficulty as i32, rng_seed);
    if mission.is_empty() {
        None
    } else {
        Some((difficulty, mission))
    }
}

/// C `generate_mission_with_preference(cn, ppd, preferred_type)`
/// (`military.c:1036-1131`)'s pure mission-table-building half: given the
/// already rank-cubed-floored `military_pts` and the level (C clamps to a
/// minimum of 7 itself before calling this - matched here too so callers
/// can pass a raw character level), builds the 5-slot offer table.
/// `mission_difficulty_preference` is `ppd->mission_difficulty_preference`
/// (`-1`/anything outside `0..=4` means "no preference", matching C's own
/// `>= 0 && < 5` guard). Does not touch `ppd->mission_type_preference` /
/// `ppd->mission_yday` - see [`crate::PlayerRuntime::apply_mission_offer`]
/// for the ppd-mutating wrapper that also stamps those.
pub fn generate_mission_with_preference(
    level: i32,
    military_pts: i32,
    preferred_type: i32,
    mission_difficulty_preference: i32,
    rng_seed: &mut u32,
) -> [SingleMission; 5] {
    let level = level.max(7);
    let mut missions = generate_demon_mission(level, military_pts, rng_seed);

    match preferred_type {
        2 => {
            if (9..=39).contains(&level) && level % 2 == 1 {
                let mission = generate_single_ratling_mission(level, military_pts, 0, rng_seed);
                if !mission.is_empty() {
                    missions[0] = mission;
                }
            }
            for _ in 0..3 {
                if let Some((difficulty, mission)) =
                    generate_sewer_mission(level, military_pts, rng_seed)
                {
                    missions[difficulty] = mission;
                }
            }
        }
        3 => {
            if level >= 12 {
                let mission = generate_single_silver_mission(level, military_pts, 0, rng_seed);
                if !mission.is_empty() {
                    missions[0] = mission;
                }
            }
            for _ in 0..3 {
                if let Some((difficulty, mission)) =
                    generate_mine_mission(level, military_pts, rng_seed)
                {
                    missions[difficulty] = mission;
                }
            }
        }
        _ => {
            if mission_random(rng_seed, 3) == 0 {
                if let Some((difficulty, mission)) =
                    generate_sewer_mission(level, military_pts, rng_seed)
                {
                    missions[difficulty] = mission;
                }
            }
            if let Some((difficulty, mission)) =
                generate_mine_mission(level, military_pts, rng_seed)
            {
                missions[difficulty] = mission;
            }
        }
    }

    if (0..5).contains(&mission_difficulty_preference) {
        let diff = mission_difficulty_preference;
        let mission = match preferred_type {
            1 => generate_single_demon_mission(level, military_pts, diff, rng_seed),
            2 => generate_single_ratling_mission(level, military_pts, diff, rng_seed),
            3 => generate_single_silver_mission(level, military_pts, diff, rng_seed),
            _ => SingleMission::default(),
        };
        if !mission.is_empty() {
            missions[diff as usize] = mission;
        }
    }

    missions
}

/// C `generate_mission(cn, ppd)` (`military.c:1137-1139`): the
/// backwards-compatible no-preference entry point, `preferred_type = 0`.
pub fn generate_mission(
    level: i32,
    military_pts: i32,
    mission_difficulty_preference: i32,
    rng_seed: &mut u32,
) -> [SingleMission; 5] {
    generate_mission_with_preference(
        level,
        military_pts,
        0,
        mission_difficulty_preference,
        rng_seed,
    )
}

/// C `death.h:21`/`pents.h:24`'s `LESSER_DEMON_CLASS_BASE`.
pub const LESSER_DEMON_CLASS_BASE: i32 = 600;
/// C `death.h:26`/`pents.h:25`'s `ELITE_DEMON_CLASS_BASE`.
pub const ELITE_DEMON_CLASS_BASE: i32 = 700;

/// C `check_military_solve`'s pent-demon class guard (`death.c:310-316`):
/// normal pent demons (three disjoint `ch.class` ranges left over from
/// incremental area content additions), plus the elite/lesser demon
/// palette-swap ranges (`ELITE_DEMON_CLASS_BASE`/`LESSER_DEMON_CLASS_BASE`,
/// each +48 wide).
pub fn is_pent_demon_mission_class(class: i32) -> bool {
    matches!(class, 52..=84 | 107..=170 | 388..=403)
        || (ELITE_DEMON_CLASS_BASE..ELITE_DEMON_CLASS_BASE + 48).contains(&class)
        || (LESSER_DEMON_CLASS_BASE..LESSER_DEMON_CLASS_BASE + 48).contains(&class)
}

/// C `check_military_solve`'s sewer-ratling class guard (`death.c:358`).
pub fn is_sewer_ratling_mission_class(class: i32) -> bool {
    (85..=100).contains(&class)
}

/// C `get_demon_mission_value(character_id)` (`src/system/death.c:281-288`,
/// identically duplicated at `src/area/4/pents.c:255-262`): elite demons
/// count for 10 mission kills each (`ELITE_DEMON_CLASS_BASE` +0..48
/// range), everything else - including lesser demons - for 1. `character_
/// id` in C is only ever used to read `ch[character_id].class`, so this
/// takes the class directly.
pub fn get_demon_mission_value(victim_class: i32) -> i32 {
    if (ELITE_DEMON_CLASS_BASE..ELITE_DEMON_CLASS_BASE + 48).contains(&victim_class) {
        10
    } else {
        1
    }
}

/// C `check_military_solve`'s progress-message display gate
/// (`death.c:339-341` demon / `:369-370` ratling, identical condition
/// both places): given the mission's new (already decremented, still
/// nonzero) `opt1` remaining count, whether C bothers to `log_char` a
/// "N to go" update this kill (large remaining counts only echo every
/// 5th/10th kill to avoid log spam).
pub fn military_mission_progress_message_should_display(remaining: i32) -> bool {
    remaining < 10 || (remaining < 100 && remaining % 5 == 0) || remaining % 10 == 0
}

/// Outcome of [`crate::PlayerRuntime::check_military_solve`], mirroring C
/// `check_military_solve`'s three observable branches (`death.c:290-383`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MilitaryMissionProgress {
    /// No active unsolved mission, or the kill didn't match its type/
    /// class/level target - `check_military_solve` is a silent no-op in
    /// C for all of these (no `else` branch on the outer `if`, and the
    /// `switch`'s default falls through to nothing).
    NoMatch,
    /// The mission's remaining count (`mis[nr].opt1`) was decremented and
    /// is still above zero. `remaining` is the new count; `elite_count`
    /// is C's `count_value` (`get_demon_mission_value`'s result, only
    /// ever >1 for elite demons - ratling missions always decrement by
    /// exactly 1).
    Progress { remaining: i32, elite_count: i32 },
    /// The mission's remaining count reached zero this kill -
    /// `solved_mission` just flipped from false to true.
    Solved,
}

/// Outcome of [`crate::PlayerRuntime::accept_mission`] (C `accept_mission`,
/// `military.c:1300-1341`). Mirrors every distinct `say()` branch;
/// `dat->storage_data.quests_given[difficulty]++` (the NPC-scoped
/// mission-offer statistic) needs a `master_id`/`World` this
/// `PlayerRuntime` method has no access to - callers should invoke
/// [`World::record_mission_offered`] themselves on
/// [`AcceptMissionOutcome::Accepted`], matching C calling it
/// unconditionally at the very end of `accept_mission`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptMissionOutcome {
    /// C: `ppd->took_mission` already nonzero -> "You already have a
    /// mission, %s. Would you like to hear it again?".
    AlreadyHasMission,
    /// C: `ppd->solved_yday == yday + 1` -> "I don't have another mission
    /// for you today, %s.".
    AlreadyCompletedToday,
    /// C: `ppd->mission_yday != yday + 1` -> "I haven't offered you that
    /// kind of mission today, %s.".
    MissionsNotOfferedToday,
    /// C: not an advisor-paid mission and its points cost exceeds
    /// `current_pts` -> "I have not offered you that kind of mission,
    /// %s.".
    InsufficientPoints,
    /// C `display_mission`'s own guard (`difficulty` out of `0..5` or
    /// `mis[difficulty].type == 0`) -> "I'm sorry, %s, but that mission is
    /// not available.".
    MissionUnavailable,
    /// Accepted; carries the mission just committed to (`mis[difficulty]`,
    /// unchanged in value by acceptance).
    Accepted(SingleMission),
}

/// Outcome of [`World::complete_mission`] (C `complete_mission`,
/// `military.c:1362-1436`)'s successful branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompletedMission {
    pub difficulty: usize,
    pub exp_awarded: i32,
    pub military_pts_awarded: i32,
    /// Mercenary-only bonus gold (`ppd->mis[difficulty].exp / 5`), 0 for
    /// every other profession.
    pub gold_awarded: i32,
    /// `Some(new_rank)` if this completion crossed an Imperial Army rank
    /// threshold (C's `rank > get_army_rank_int(co)` guard).
    pub promoted_to: Option<i32>,
}

/// Outcome of [`World::complete_mission`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompleteMissionResult {
    /// C: `if (!ppd->solved_mission) return 0;` - nothing to complete, no
    /// mutation happened.
    NoActiveMission,
    Completed(CompletedMission),
}

impl World {
    /// C `complete_mission(cn, co, ppd, dat)` (`military.c:1362-1436`)'s
    /// full ppd + character mutation: awards the mission's exp via
    /// [`World::give_exp`] (`ppd->normal_exp` bookkeeping matches
    /// `Character.military_normal_exp`, same field [`World::give_military_
    /// pts`] uses), the mercenary bonus gold/points formula
    /// (`ch[co].prof[P_MERCENARY]`, `legacy::profession::MERCENARY`), the
    /// raw `military_pts` add (deliberately *not* routed through
    /// [`World::give_military_pts`] - unlike that function's `_no_npc`
    /// form, C's own `complete_mission` never applies
    /// `hardcore_military_exp_bonus` to `pts`, and the exp was already
    /// awarded above, so reusing it would double-grant exp and misapply
    /// the hardcore bonus), and the identical rank-promotion
    /// message/broadcast pattern. Queues the "Well done, %s. You've solved
    /// your mission!" and (mercenary-only) gold-received text via
    /// [`World::queue_system_text`]/[`World::queue_system_text_bytes`],
    /// matching `check_military_solve`'s own wiring pattern (no NPC driver
    /// needed for plain system text). Also bumps `dat->storage_data.
    /// quests_solved/pts_given/exp_given[difficulty]` on the Military
    /// Master NPC identified by `master_id` (`military.c:1382,1407,1411`) -
    /// a no-op if `master_id` isn't a live `CDR_MILITARY_MASTER` NPC.
    /// Skips the wealth-achievement ladder the real `give_money` also
    /// updates (that needs `add_gold_earned`'s DB-backed first-unlock
    /// announce, which lives in the server crate - wire it at the same
    /// time a real Military Master NPC driver call site lands).
    pub fn complete_mission(
        &mut self,
        character_id: CharacterId,
        player: &mut PlayerRuntime,
        area_id: u32,
        master_id: CharacterId,
    ) -> CompleteMissionResult {
        if !player.military_solved_mission() {
            return CompleteMissionResult::NoActiveMission;
        }
        player.set_military_solved_mission(false);

        let took_yday = player.military_took_yday();
        player.set_military_solved_yday(took_yday);
        player.set_military_took_yday(0);

        let took_mission = player.military_took_mission();
        let difficulty = (took_mission - 1).clamp(0, 4) as usize;
        player.set_military_took_mission(0);

        let mission = player.military_mission(difficulty);

        self.give_exp(character_id, i64::from(mission.exp), area_id);

        let Some(character) = self.characters.get_mut(&character_id) else {
            return CompleteMissionResult::Completed(CompletedMission {
                difficulty,
                exp_awarded: mission.exp,
                ..Default::default()
            });
        };
        character.military_normal_exp = character.military_normal_exp.saturating_add(mission.exp);

        let mercenary_level = i32::from(character.professions[profession::MERCENARY]);
        let mut gold_awarded = 0;
        let pts = if mercenary_level > 0 {
            gold_awarded = mission.exp / 5;
            character.gold = character.gold.saturating_add(gold_awarded as u32);
            character.flags.insert(CharacterFlags::ITEMS);
            mission.pts + mission.pts / 2 + mission.pts * mercenary_level * 3 / 100 + 1
        } else {
            mission.pts + mission.pts / 2
        };

        let old_rank = army_rank_for_points(character.military_points);
        character.military_points = character.military_points.saturating_add(pts);
        character.flags.insert(CharacterFlags::UPDATE);
        let new_rank = army_rank_for_points(character.military_points);
        let name = character.name.clone();

        // C `dat->storage_data.quests_solved[difficulty]++;` /
        // `pts_given[difficulty] += ppd->mis[difficulty].pts;` /
        // `exp_given[difficulty] += ppd->mis[difficulty].exp;`
        // (`military.c:1382,1407,1411`) - a no-op if `master_id` isn't a
        // live `CDR_MILITARY_MASTER` NPC.
        if let Some(CharacterDriverState::MilitaryMaster(data)) = self
            .characters
            .get(&master_id)
            .and_then(|c| c.driver_state.as_ref())
        {
            let storage_id = data.storage_id;
            self.military_master_storage.add_completed_mission_stats(
                storage_id,
                difficulty,
                mission.exp,
                mission.pts,
            );
        }

        if gold_awarded > 0 {
            let gold_str = if gold_awarded < 100 {
                format!("{gold_awarded}s")
            } else {
                format!("{:.2}G", f64::from(gold_awarded) / 100.0)
            };
            let mut message = Vec::with_capacity(64);
            message.extend_from_slice(b"You received");
            message.extend_from_slice(crate::text::COL_YELLOW);
            message.push(b' ');
            message.extend_from_slice(gold_str.as_bytes());
            message.extend_from_slice(crate::text::COL_RESET);
            message.extend_from_slice(b". It has been placed in your gold pouch.");
            self.queue_system_text_bytes(character_id, message);
        }
        self.queue_system_text(
            character_id,
            format!("Well done, {name}. You've solved your mission!"),
        );

        let promoted_to = if new_rank > old_rank {
            self.queue_system_text(
                character_id,
                format!(
                    "You've been promoted to {}. Congratulations, {name}!",
                    army_rank_name(new_rank)
                ),
            );
            if new_rank > 9 {
                let mut broadcast = b"0000000000".to_vec();
                broadcast.extend_from_slice(crate::text::COL_CHAT_GRATS);
                broadcast.extend_from_slice(
                    format!("Grats: {name} is a {} now!", army_rank_name(new_rank)).as_bytes(),
                );
                self.queue_channel_broadcast(6, broadcast);
            }
            Some(new_rank)
        } else {
            None
        };

        CompleteMissionResult::Completed(CompletedMission {
            difficulty,
            exp_awarded: mission.exp,
            military_pts_awarded: pts,
            gold_awarded,
            promoted_to,
        })
    }
}

/// Outcome of [`World::mission_reroll`] (C `handle_mission_reroll`,
/// `military.c:1889-1936`), mirroring every distinct `say()` branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionRerollOutcome {
    /// C: `ppd->reroll_yday == yday + 1` -> "I've already offered you a
    /// different set of missions today, %s. Come back tomorrow if you
    /// want more options.".
    AlreadyRerolledToday,
    /// C: `ppd->took_mission` nonzero -> "You already accepted a mission,
    /// %s. You must either complete it or report your failure before
    /// requesting new missions.".
    HasActiveMission,
    /// C: `ch[co].gold < 20000` -> "Generating new mission plans costs
    /// 200 gold, %s, which you don't seem to have.".
    InsufficientGold,
    /// C: `ppd->master_state != 10` (not yet confirmed) -> "I can prepare
    /// a different set of missions for you, %s, but it will cost 200
    /// gold. Say reroll again to confirm.", stamps `master_state = 10`.
    ConfirmationRequested,
    /// Confirmed; 200 gold spent and a fresh 5-slot offer table
    /// generated (now in `ppd->mis[]`), matching C's "Very well, %s.
    /// Here are your new mission options:" plus its `offer_missions`
    /// call - callers should read the mission table back via
    /// [`crate::PlayerRuntime::military_mission`] to render it, same as
    /// every other offer-table consumer in this module.
    Rerolled,
}

impl World {
    /// C `handle_mission_reroll(cn, co, ppd)` (`military.c:1889-1936`):
    /// the paid mission-reroll confirmation flow. `yday` is C's global
    /// `yday` (`World.date.yday`); `rng_seed` is caller-supplied, same as
    /// [`crate::PlayerRuntime::apply_mission_offer`] (no Rust call site
    /// yet resolves either - see this module's doc comment). Reproduces
    /// C's own rank-cubed `military_pts` floor-up (`generate_mission_
    /// with_preference`'s "Adjust military exp for rank if the player
    /// gained a rank elsewhere" comment) here at the call site, exactly
    /// like that comment describes, since `military_pts` isn't otherwise
    /// kept in sync with `Character.military_points` between mission
    /// generations.
    pub fn mission_reroll(
        &mut self,
        character_id: CharacterId,
        player: &mut PlayerRuntime,
        yday: i32,
        rng_seed: &mut u32,
    ) -> MissionRerollOutcome {
        if player.military_reroll_yday() == yday + 1 {
            return MissionRerollOutcome::AlreadyRerolledToday;
        }
        if player.military_took_mission() != 0 {
            return MissionRerollOutcome::HasActiveMission;
        }
        let Some(character) = self.characters.get(&character_id) else {
            return MissionRerollOutcome::InsufficientGold;
        };
        if character.gold < 20_000 {
            return MissionRerollOutcome::InsufficientGold;
        }
        if player.master_state() != 10 {
            player.set_master_state(10);
            return MissionRerollOutcome::ConfirmationRequested;
        }

        let (level, rank) = {
            let character = self
                .characters
                .get_mut(&character_id)
                .expect("checked above");
            character.gold -= 20_000;
            character.flags.insert(CharacterFlags::ITEMS);
            (
                character.level as i32,
                army_rank_for_points(character.military_points),
            )
        };

        let rank_cubed = rank.saturating_mul(rank).saturating_mul(rank);
        if rank_cubed > player.military_pts() {
            player.set_military_pts(rank_cubed);
        }

        player.set_military_reroll_yday(yday + 1);
        player.set_mission_yday(0);

        let preferred_type = player.mission_type_preference();
        let military_pts = player.military_pts();
        player.apply_mission_offer(level, military_pts, preferred_type, yday, rng_seed);

        player.set_master_state(2);

        MissionRerollOutcome::Rerolled
    }
}

/// C `military.c:2108-2206`'s `military_master_driver`'s `NT_CHAR`
/// distance gate (`char_dist(cn, co) > 10`).
const MILITARY_MASTER_GREET_DISTANCE: i32 = 10;
/// C `analyse_text_driver`'s own distance gate (`char_dist(cn, co) >
/// 12`), shared by every qa-table NPC's text handling.
const MILITARY_MASTER_TEXT_DISTANCE: i32 = 12;
/// C `DX_DOWN` (`common/direction.h:20`): the Military Master's fixed
/// resting facing (C's own `secure_move_driver(cn, ch[cn].tmpx,
/// ch[cn].tmpy, DX_DOWN, ret, lastact)`, `military.c:2201`).
const MILITARY_MASTER_REST_DIRECTION: u8 = 3;

/// C `static char *diff_name[5]` (`military.c:339`).
const MISSION_DIFFICULTY_NAMES: [&str; 5] = ["easy", "normal", "hard", "impossible", "insane"];

/// C `diff_name[difficulty]`/`get_colored_difficulty_name`'s own clamp
/// (`military.c:1350-1361` - out-of-range falls back to index `0`).
pub fn mission_difficulty_name(difficulty: usize) -> &'static str {
    MISSION_DIFFICULTY_NAMES
        .get(difficulty)
        .copied()
        .unwrap_or("easy")
}

/// C `describe_mission` (`military.c:1194-1220`): the offer-time
/// description ("I have an easy mission for you, NAME. ..."). `None` for
/// an empty mission slot (`mission->type == 0`) or an unrecognized type,
/// matching C's own guard/`default: return 0`.
pub fn describe_mission_text(
    mission: &SingleMission,
    difficulty: usize,
    player_name: &str,
) -> Option<String> {
    if mission.is_empty() {
        return None;
    }
    let diff = mission_difficulty_name(difficulty);
    match mission.mission_type {
        MISSION_TYPE_DEMON => Some(format!(
            "I have an {diff} mission for you, {player_name}. It is to slay {} level {} demons \
             in the Pentagram Quest.",
            mission.opt1, mission.opt2
        )),
        MISSION_TYPE_RATLING => Some(format!(
            "I have an {diff} mission for you, {player_name}. It is to slay {} level {} \
             ratlings in the Sewers.",
            mission.opt1, mission.opt2
        )),
        MISSION_TYPE_SILVER => Some(format!(
            "I have an {diff} mission for you, {player_name}. It is to find {} units of silver \
             in the Mine.",
            mission.opt1
        )),
        _ => None,
    }
}

/// C `display_mission` (`military.c:1261-1288`): the accept/hear-time
/// description ("Your mission is to..."). `None` for an unrecognized
/// type; callers should say the "that mission is not available" line on
/// `None`, matching C's own fallback (this should not happen in practice
/// for a mission slot that was already validated non-empty by the
/// caller).
pub fn display_mission_text(mission: &SingleMission) -> Option<String> {
    match mission.mission_type {
        MISSION_TYPE_DEMON => Some(format!(
            "Your mission is to slay {} level {} demons in the Pentagram Quest.",
            mission.opt1, mission.opt2
        )),
        MISSION_TYPE_RATLING => Some(format!(
            "Your mission is to slay {} level {} ratlings in the Sewers.",
            mission.opt1, mission.opt2
        )),
        MISSION_TYPE_SILVER => Some(format!(
            "Your mission is to find {} units of silver in the Mine.",
            mission.opt1
        )),
        _ => None,
    }
}

/// C `offer_missions` (`military.c:1231-1246`): describes every mission
/// slot the player can currently afford (`mis[d].pts <= 1 ||
/// mis[d].pts <= current_pts`), falling back to the "no suitable
/// missions" line if none qualified.
pub fn offer_missions_text(
    missions: &[SingleMission; 5],
    current_pts: i32,
    player_name: &str,
) -> Vec<String> {
    let mut lines = Vec::new();
    for (difficulty, mission) in missions.iter().enumerate() {
        if mission.pts > 1 && mission.pts > current_pts {
            continue;
        }
        if let Some(text) = describe_mission_text(mission, difficulty, player_name) {
            lines.push(text);
        }
    }
    if lines.is_empty() {
        lines.push(format!(
            "I'm sorry, {player_name}, but I don't have any suitable missions for you at the \
             moment."
        ));
    }
    lines
}

/// Outcome of [`World::handle_mission_request`] (C `handle_mission_request`,
/// `military.c:1842-1896`), mirroring every distinct `say()` branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissionRequestOutcome {
    /// C: `ppd->took_mission` nonzero -> "You already have a mission.
    /// Would you like to hear it again?" (this particular line has no
    /// `%s` player-name substitution, unlike almost every other branch in
    /// this file - matches C exactly).
    AlreadyHasMission,
    /// C: `ppd->solved_yday == yday + 1` -> "I don't have another mission
    /// for you today, %s.".
    AlreadyCompletedToday,
    /// C: `!get_army_rank_int(co)` -> "But you don't even belong to the
    /// army, %s. Talk to Seymour about enrollment.".
    NotEnrolled,
    /// C: a fresh advisor-recommended mission was generated and
    /// highlighted this call (`mission_type_preference > 0` and the
    /// preferred difficulty's freshly generated mission type matches it)
    /// - carries the mission description line plus the "accept by
    /// saying X" prompt line; C returns immediately here without the
    /// general `offer_missions` listing.
    AdvisorRecommendation { description: String, prompt: String },
    /// Normal offer: every line [`offer_missions_text`] produced, plus
    /// the reroll-footer line.
    Offered(Vec<String>),
}

impl World {
    /// C `handle_mission_request(cn, co, ppd)` (`military.c:1842-1896`):
    /// the "mission" keyword handler. Generates a fresh offer table via
    /// [`crate::PlayerRuntime::apply_mission_offer`] if none was
    /// generated today, reproducing the same rank-cubed `military_pts`
    /// floor-up [`World::mission_reroll`] already applies at its own call
    /// site (`generate_mission_with_preference`'s "Adjust military exp
    /// for rank" comment - the floor lives in the C *caller*, not the
    /// pure generator, so every caller must repeat it).
    pub fn handle_mission_request(
        &mut self,
        character_id: CharacterId,
        player: &mut PlayerRuntime,
        yday: i32,
        rng_seed: &mut u32,
        player_name: &str,
    ) -> MissionRequestOutcome {
        if player.military_took_mission() != 0 {
            return MissionRequestOutcome::AlreadyHasMission;
        }
        if player.military_solved_yday() == yday + 1 {
            return MissionRequestOutcome::AlreadyCompletedToday;
        }
        let Some(character) = self.characters.get(&character_id) else {
            return MissionRequestOutcome::NotEnrolled;
        };
        if army_rank_for_points(character.military_points) <= 0 {
            return MissionRequestOutcome::NotEnrolled;
        }

        if player.mission_yday() != yday + 1 {
            let rank = army_rank_for_points(character.military_points);
            let rank_cubed = rank.saturating_mul(rank).saturating_mul(rank);
            if rank_cubed > player.military_pts() {
                player.set_military_pts(rank_cubed);
            }
            let level = (character.level as i32).max(7);
            let preferred_type = player.mission_type_preference();
            let military_pts = player.military_pts();
            player.apply_mission_offer(level, military_pts, preferred_type, yday, rng_seed);

            if preferred_type > 0 {
                let diff_pref = player.mission_difficulty_preference();
                if (0..5).contains(&diff_pref) {
                    let mission = player.military_mission(diff_pref as usize);
                    if mission.mission_type == preferred_type {
                        let description =
                            describe_mission_text(&mission, diff_pref as usize, player_name)
                                .unwrap_or_default();
                        let prompt = format!(
                            "This mission was specifically requested by your advisor. You may \
                             accept it by saying {}.",
                            mission_difficulty_name(diff_pref as usize)
                        );
                        return MissionRequestOutcome::AdvisorRecommendation {
                            description,
                            prompt,
                        };
                    }
                }
            }
        }

        let missions: [SingleMission; 5] = std::array::from_fn(|i| player.military_mission(i));
        let mut lines = offer_missions_text(&missions, player.military_current_pts(), player_name);
        lines.push(
            "If you don't like these missions, you can request a new set by saying reroll for \
             200 gold. This can only be done once per day."
                .to_string(),
        );
        MissionRequestOutcome::Offered(lines)
    }
}

/// C `struct military_master_storage` (`military.c:346-352`): the
/// NPC-scoped counters `military_master_data` persists through the
/// generic `storage` table (`create_storage`/`read_storage`/
/// `update_storage`, `src/system/database/database_storage.c`) rather
/// than through per-character PPD or the world save. This is the first
/// consumer of that still-unported "storage-blob" concept - see
/// [`MilitaryMasterStorageRegistry`]'s doc comment for the scoped
/// in-memory-only approach this takes instead (no DB persistence yet).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MilitaryMasterStorage {
    /// `clan_pts[MAXCLAN]`: military points banked on behalf of each
    /// clan, fed by [`World::update_clan_points`] and spent by
    /// [`World::process_clan_recommendation`]. Index `0` ("no clan") is
    /// never read/written by either function (`get_char_clan` never
    /// returns `0`), matching C leaving `clan_pts[0]` permanently unused.
    clan_pts: [i32; crate::clan::MAX_CLAN],
    /// `quests_given[5]`: incremented once per difficulty every time a
    /// mission is offered (`military.c:1348`), fed by
    /// [`World::record_mission_offered`].
    quests_given: [i32; 5],
    /// `quests_solved[5]` (`military.c:1382`), fed by
    /// [`World::complete_mission`].
    quests_solved: [i32; 5],
    /// `exp_given[5]` (`military.c:1411`), fed by
    /// [`World::complete_mission`].
    exp_given: [i32; 5],
    /// `pts_given[5]` (`military.c:1407`), fed by
    /// [`World::complete_mission`].
    pts_given: [i32; 5],
}

impl Default for MilitaryMasterStorage {
    fn default() -> Self {
        Self {
            clan_pts: [0; crate::clan::MAX_CLAN],
            quests_given: [0; 5],
            quests_solved: [0; 5],
            exp_given: [0; 5],
            pts_given: [0; 5],
        }
    }
}

impl MilitaryMasterStorage {
    /// `dat->storage_data.clan_pts[clan_nr]`. Out-of-range clan numbers
    /// read as `0`, matching a fresh `struct military_master_storage`'s
    /// zero-initialized array (C itself never range-checks `clan_nr`
    /// beyond the caller already having a valid `get_char_clan` result).
    pub fn clan_pts(&self, clan_nr: u16) -> i32 {
        self.clan_pts.get(clan_nr as usize).copied().unwrap_or(0)
    }

    fn add_clan_pts(&mut self, clan_nr: u16, delta: i32) {
        if let Some(slot) = self.clan_pts.get_mut(clan_nr as usize) {
            *slot += delta;
        }
    }

    /// `dat->storage_data.quests_given[difficulty]`.
    pub fn quests_given(&self, difficulty: usize) -> i32 {
        self.quests_given.get(difficulty).copied().unwrap_or(0)
    }

    /// `dat->storage_data.quests_solved[difficulty]`.
    pub fn quests_solved(&self, difficulty: usize) -> i32 {
        self.quests_solved.get(difficulty).copied().unwrap_or(0)
    }

    /// `dat->storage_data.exp_given[difficulty]`.
    pub fn exp_given(&self, difficulty: usize) -> i32 {
        self.exp_given.get(difficulty).copied().unwrap_or(0)
    }

    /// `dat->storage_data.pts_given[difficulty]`.
    pub fn pts_given(&self, difficulty: usize) -> i32 {
        self.pts_given.get(difficulty).copied().unwrap_or(0)
    }

    /// `dat->storage_data.quests_given[difficulty]++` (`military.c:1348`).
    fn add_quests_given(&mut self, difficulty: usize) {
        if let Some(slot) = self.quests_given.get_mut(difficulty) {
            *slot += 1;
        }
    }

    /// `dat->storage_data.quests_solved[difficulty]++` (`military.c:1382`).
    fn add_quests_solved(&mut self, difficulty: usize) {
        if let Some(slot) = self.quests_solved.get_mut(difficulty) {
            *slot += 1;
        }
    }

    /// `dat->storage_data.exp_given[difficulty] += ppd->mis[difficulty].
    /// exp;` (`military.c:1411`).
    fn add_exp_given(&mut self, difficulty: usize, delta: i32) {
        if let Some(slot) = self.exp_given.get_mut(difficulty) {
            *slot += delta;
        }
    }

    /// `dat->storage_data.pts_given[difficulty] += ppd->mis[difficulty].
    /// pts;` (`military.c:1407`) - note this is the mission's raw point
    /// *cost*, not the (larger, formula-adjusted) amount actually
    /// credited to the player's `military_pts` (`CompletedMission::
    /// military_pts_awarded`).
    fn add_pts_given(&mut self, difficulty: usize, delta: i32) {
        if let Some(slot) = self.pts_given.get_mut(difficulty) {
            *slot += delta;
        }
    }
}

/// A registry of [`MilitaryMasterStorage`] blobs keyed by `storage_id`
/// (the zone-file `storage=N;` arg every Military Master NPC is
/// configured with, `military.c:1634-1644` - see
/// [`crate::character_driver::MilitaryMasterDriverData`]), mirroring
/// [`crate::clan::ClanRegistry`]'s "one typed struct per consumer,
/// `Serialize`/`Deserialize` end to end" shape rather than C's own
/// generic byte-blob `storage` table
/// (`src/system/database/database_storage.c`'s `create_storage`/
/// `read_storage`/`update_storage`, id/version/blob with optimistic
/// concurrency) - the scoped recommendation researched in the "Military
/// ranks" task's own iteration-114 progress-log note: "a small
/// typed-struct-per-consumer table/repository in `ugaris-db` ... keyed
/// per storage id since these aren't singletons, not a generic
/// byte-blob framework".
///
/// Unlike [`crate::clan::ClanRegistry`], this registry is **not yet
/// wired to any DB repository** - it lives only in memory for the
/// lifetime of the process, resetting to empty (all-zero counters) on
/// every restart. This is a smaller regression than it sounds: C's own
/// per-clan `clan_pts` bonus feed only ever grows by `get_clan_bonus(n,
/// 1) * 20` every 60 seconds and is spent in 12000-point chunks, so a
/// restart merely delays the next recommendation rather than losing
/// meaningful player-facing state permanently. Closing this gap (a
/// `military_master_storage(storage_id integer primary key, storage_json
/// jsonb not null, updated_at)` table following `clan.rs`'s
/// `PgClanRegistryRepository` pattern, loaded at boot and periodically
/// saved when [`Self::dirty`]) is left for a future slice - see the
/// "Military ranks" task in `PORTING_TODO.md`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MilitaryMasterStorageRegistry {
    entries: std::collections::BTreeMap<i32, MilitaryMasterStorage>,
    #[serde(skip)]
    dirty: bool,
}

impl MilitaryMasterStorageRegistry {
    /// Read-only lookup; a `storage_id` with no entry yet reads as a
    /// fresh all-zero [`MilitaryMasterStorage`] (matching C's own
    /// zero-initialized `struct military_master_storage` before the
    /// first `create_storage` round trip completes) without allocating
    /// one.
    pub fn clan_pts(&self, storage_id: i32, clan_nr: u16) -> i32 {
        self.entries
            .get(&storage_id)
            .map(|storage| storage.clan_pts(clan_nr))
            .unwrap_or(0)
    }

    /// C `dat->storage_data.clan_pts[n] += bonus;`
    /// ([`World::update_clan_points`]) / `dat->storage_data.clan_pts[clan_
    /// nr] -= 12000;` ([`World::process_clan_recommendation`]). Creates
    /// the entry on first use, matching C's `create_storage` lazily
    /// bringing a fresh zeroed blob into existence.
    fn add_clan_pts(&mut self, storage_id: i32, clan_nr: u16, delta: i32) {
        self.entries
            .entry(storage_id)
            .or_default()
            .add_clan_pts(clan_nr, delta);
        self.dirty = true;
    }

    /// C `dat->storage_data.quests_given[difficulty]++` (`accept_mission`,
    /// `military.c:1348`). Creates the entry on first use, matching
    /// [`Self::add_clan_pts`]'s lazy-`create_storage` semantics.
    fn add_quests_given(&mut self, storage_id: i32, difficulty: usize) {
        self.entries
            .entry(storage_id)
            .or_default()
            .add_quests_given(difficulty);
        self.dirty = true;
    }

    /// C `dat->storage_data.quests_solved[difficulty]++` / `pts_given`/
    /// `exp_given[difficulty] += ...` (`complete_mission`,
    /// `military.c:1382,1407,1411`), all three bumped together since C's
    /// own `complete_mission` always updates them in the same call.
    fn add_completed_mission_stats(
        &mut self,
        storage_id: i32,
        difficulty: usize,
        exp: i32,
        pts: i32,
    ) {
        let entry = self.entries.entry(storage_id).or_default();
        entry.add_quests_solved(difficulty);
        entry.add_exp_given(difficulty, exp);
        entry.add_pts_given(difficulty, pts);
        self.dirty = true;
    }

    /// Read-only per-difficulty quest-stat lookup - `(given, solved,
    /// exp_given, pts_given)`, wired from [`World::record_mission_
    /// offered`] (quests_given) and [`World::complete_mission`] (the
    /// other three) since iteration 116.
    pub fn quest_stats(&self, storage_id: i32, difficulty: usize) -> (i32, i32, i32, i32) {
        match self.entries.get(&storage_id) {
            Some(storage) => (
                storage.quests_given(difficulty),
                storage.quests_solved(difficulty),
                storage.exp_given(difficulty),
                storage.pts_given(difficulty),
            ),
            None => (0, 0, 0, 0),
        }
    }

    /// Whether any entry has changed since the last [`Self::clear_dirty`],
    /// consulted by `ugaris-server`'s periodic save tick (see
    /// `crates/ugaris-db/src/military.rs`).
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// All `(storage_id, storage)` rows currently held, for the DB
    /// repository's per-row upsert on save (`crates/ugaris-db/src/
    /// military.rs`) - mirrors the table design this type's own doc
    /// comment describes (one row per `storage_id`, unlike
    /// [`crate::clan::ClanRegistry`]'s single-row-blob approach).
    pub fn iter(&self) -> impl Iterator<Item = (i32, &MilitaryMasterStorage)> {
        self.entries.iter().map(|(id, storage)| (*id, storage))
    }

    /// Rebuilds a registry from persisted `(storage_id, storage)` rows
    /// (the DB repository's load path) without marking it dirty - a
    /// freshly loaded registry has nothing new to save back until it is
    /// mutated again.
    pub fn from_rows(rows: impl IntoIterator<Item = (i32, MilitaryMasterStorage)>) -> Self {
        Self {
            entries: rows.into_iter().collect(),
            dirty: false,
        }
    }
}

/// C `struct cost_data` (`system/tool.h:94-101`): a single favor size's
/// market-driven sales counters. Only [`Self::earned`]/[`Self::sold`] are
/// ported - the rolling `amount[20]`/`date[20]` sale-history window and
/// the `created` creation timestamp exist in C purely to feed
/// `calc_cost`'s market-driven pricing formula (`tool.c:3187-3215`), and
/// `calc_cost` is never actually called anywhere in the C tree (checked
/// via `grep -rn calc_cost src/` - only its own declaration/definition
/// match), so those fields would be genuinely dead weight with no reader
/// anywhere, unlike `earned`/`sold` which the admin-only "info" qa code
/// (`military.c:2534-2537`) does read back.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CostData {
    /// `dat->earned` (`long long`): total gold (in copper/gold-cent
    /// units, matching `ch[].gold`) ever recorded via `add_cost`.
    earned: i64,
    /// `dat->sold` (`int`): number of `add_cost` calls ever recorded.
    sold: i32,
}

impl CostData {
    /// C `add_cost(cost, dat)`'s `dat->earned += cost; dat->sold++;`
    /// half (`tool.c:3219-3226`) - the `amount`/`date` ring-buffer shift
    /// is intentionally not reproduced, see this type's doc comment.
    fn add_cost(&mut self, cost: i32) {
        self.earned += i64::from(cost);
        self.sold += 1;
    }
}

/// C `struct military_advisor_data`'s `struct cost_data storage_data[5]`
/// (`military.c:374`): one [`CostData`] slot per favor size (small/
/// normal/big/huge/vast, indices `0..=4`) - the same 5-way index
/// `ppd->advisor_storage_nr` already uses for both plain favors
/// ([`World::offer_favor`]'s `favor_size`) and specific mission requests
/// ([`World::handle_specific_mission_request`]'s `difficulty`, reused as
/// the storage index verbatim by C's own `dat->storage_data +
/// ppd->advisor_storage_nr` in `process_favor_payment`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MilitaryAdvisorStorage {
    cost_data: [CostData; 5],
}

impl MilitaryAdvisorStorage {
    /// `dat->storage_data[n].earned`. Out-of-range `n` reads as `0`,
    /// matching a fresh zero-initialized slot.
    pub fn earned(&self, favor_size: usize) -> i64 {
        self.cost_data.get(favor_size).map_or(0, |d| d.earned)
    }

    /// `dat->storage_data[n].sold`.
    pub fn sold(&self, favor_size: usize) -> i32 {
        self.cost_data.get(favor_size).map_or(0, |d| d.sold)
    }

    /// C `add_cost(ppd->advisor_cost, dat->storage_data +
    /// ppd->advisor_storage_nr)` (`military.c:2421`).
    fn add_cost(&mut self, favor_size: usize, cost: i32) {
        if let Some(slot) = self.cost_data.get_mut(favor_size) {
            slot.add_cost(cost);
        }
    }
}

/// A registry of [`MilitaryAdvisorStorage`] blobs keyed by `storage_id`
/// (the zone-file `storage=N;` arg every Military Advisor NPC is
/// configured with - `crate::character_driver::MilitaryAdvisorDriverData`),
/// mirroring [`MilitaryMasterStorageRegistry`]'s own shape exactly (one
/// typed struct per consumer, in-memory only for now - see that type's
/// doc comment for the rationale). DB persistence for this registry
/// (a `military_advisor_storage(storage_id integer primary key,
/// storage_json jsonb, updated_at)` table following `crates/ugaris-db/
/// src/military.rs`'s `PgMilitaryMasterStorageRepository` pattern) is
/// left for a future slice - see the "Military ranks" task in
/// `PORTING_TODO.md`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct MilitaryAdvisorStorageRegistry {
    entries: std::collections::BTreeMap<i32, MilitaryAdvisorStorage>,
    #[serde(skip)]
    dirty: bool,
}

impl MilitaryAdvisorStorageRegistry {
    /// Read-only lookup; a `storage_id` with no entry yet reads as a
    /// fresh all-zero [`MilitaryAdvisorStorage`], matching C's own
    /// zero-initialized `struct military_advisor_data` before the first
    /// `create_storage` round trip completes.
    pub fn earned(&self, storage_id: i32, favor_size: usize) -> i64 {
        self.entries
            .get(&storage_id)
            .map_or(0, |storage| storage.earned(favor_size))
    }

    /// `dat->storage_data[n].sold`.
    pub fn sold(&self, storage_id: i32, favor_size: usize) -> i32 {
        self.entries
            .get(&storage_id)
            .map_or(0, |storage| storage.sold(favor_size))
    }

    /// C `add_cost(ppd->advisor_cost, dat->storage_data +
    /// ppd->advisor_storage_nr)` (`military.c:2421`), fed by
    /// [`World::process_favor_payment`]. Creates the entry on first use,
    /// matching C's `create_storage` lazily bringing a fresh zeroed blob
    /// into existence.
    fn add_cost(&mut self, storage_id: i32, favor_size: usize, cost: i32) {
        self.entries
            .entry(storage_id)
            .or_default()
            .add_cost(favor_size, cost);
        self.dirty = true;
    }

    /// Whether any entry has changed since the last [`Self::clear_dirty`].
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// All `(storage_id, storage)` rows currently held, for a future DB
    /// repository's per-row upsert on save (mirrors
    /// [`MilitaryMasterStorageRegistry::iter`]).
    pub fn iter(&self) -> impl Iterator<Item = (i32, &MilitaryAdvisorStorage)> {
        self.entries.iter().map(|(id, storage)| (*id, storage))
    }

    /// Rebuilds a registry from persisted `(storage_id, storage)` rows
    /// without marking it dirty (mirrors
    /// [`MilitaryMasterStorageRegistry::from_rows`]).
    pub fn from_rows(rows: impl IntoIterator<Item = (i32, MilitaryAdvisorStorage)>) -> Self {
        Self {
            entries: rows.into_iter().collect(),
            dirty: false,
        }
    }
}

impl World {
    /// C `process_clan_recommendation(cn, co, ppd, dat)`
    /// (`military.c:1654-1674`): grants `+5 military_current_pts` (C's
    /// `ppd->current_pts += 5`) and deducts 12000 clan points once a
    /// clan-member player is greeted, gated on the clan having banked
    /// more than 12000 points (fed by [`World::update_clan_points`]) and
    /// not being the same player already recommended (`dat->last_recom`)
    /// this NPC's lifetime. Called right before
    /// [`World::process_advisor_recommendation`] in C's own `NT_CHAR`
    /// handler (`military.c:2150-2153`), matching this function's own
    /// call site in `ugaris-server`'s `apply_military_master_nearby_
    /// player`.
    ///
    /// Reads through [`crate::clan::ClanRegistry::get_char_clan`], so a
    /// stale clan reference on the player is cleared as a side effect,
    /// exactly like every other `get_char_clan` call site. Non-clan-
    /// member players are a silent no-op, matching C's `!(clan_nr =
    /// get_char_clan(co))` early return.
    pub fn process_clan_recommendation(
        &mut self,
        master_id: CharacterId,
        player_id: CharacterId,
        player: &mut PlayerRuntime,
        player_name: &str,
    ) -> Option<String> {
        let clan_nr = {
            let character = self.characters.get_mut(&player_id)?;
            self.clan_registry.get_char_clan(character)?
        };

        let storage_id = match self
            .characters
            .get(&master_id)
            .and_then(|c| c.driver_state.as_ref())
        {
            Some(CharacterDriverState::MilitaryMaster(data)) => data.storage_id,
            _ => return None,
        };

        if self.military_master_storage.clan_pts(storage_id, clan_nr) <= 12000 {
            return None;
        }

        let already_recommended = match self
            .characters
            .get(&master_id)
            .and_then(|c| c.driver_state.as_ref())
        {
            Some(CharacterDriverState::MilitaryMaster(data)) => data.last_recom == player_id.0,
            _ => true,
        };
        if already_recommended {
            return None;
        }

        player.set_military_current_pts(player.military_current_pts() + 5);
        self.military_master_storage
            .add_clan_pts(storage_id, clan_nr, -12000);
        if let Some(CharacterDriverState::MilitaryMaster(data)) = self
            .characters
            .get_mut(&master_id)
            .and_then(|c| c.driver_state.as_mut())
        {
            data.last_recom = player_id.0;
        }

        Some(format!(
            "Be greeted, {player_name}. You've been recommended by your clan!"
        ))
    }

    /// C `update_clan_points(dat)` (`military.c:1815-1832`): every 60
    /// seconds, feeds every clan's `get_clan_bonus(n, 1) * 20` ("Military
    /// Advisor" bonus level, [`crate::clan::CLAN_BONUS_MILITARY_ADVISOR`])
    /// into that clan's banked `clan_pts`. Called once per Military
    /// Master NPC per tick (`ugaris-server`'s
    /// `process_military_master_actions` call site), independent of any
    /// player message.
    ///
    /// C stamps `dat->last_clan_update = realtime` on the NPC's own
    /// `NT_CREATE` (`military.c:2126`), which Rust has no equivalent
    /// hook for at zone-parse time (see
    /// [`crate::character_driver::MilitaryMasterDriverData`]'s doc
    /// comment) - so a `last_clan_update == 0` reads as "just created"
    /// here and is lazily stamped to `now` without granting a bonus yet,
    /// reproducing the same "no bonus for the first 60 seconds after
    /// spawn" behavior.
    pub fn update_clan_points(&mut self, master_id: CharacterId, now: i64) {
        let Some(CharacterDriverState::MilitaryMaster(data)) = self
            .characters
            .get(&master_id)
            .and_then(|c| c.driver_state.as_ref())
        else {
            return;
        };
        let storage_id = data.storage_id;
        let last_clan_update = data.last_clan_update;

        if last_clan_update == 0 {
            if let Some(CharacterDriverState::MilitaryMaster(data)) = self
                .characters
                .get_mut(&master_id)
                .and_then(|c| c.driver_state.as_mut())
            {
                data.last_clan_update = now;
            }
            return;
        }
        if now - last_clan_update <= 60 {
            return;
        }

        for clan_nr in 1..crate::clan::MAX_CLAN as u16 {
            let bonus = self
                .clan_registry
                .bonus_level(clan_nr, crate::clan::CLAN_BONUS_MILITARY_ADVISOR)
                * 20;
            if bonus > 0 {
                self.military_master_storage
                    .add_clan_pts(storage_id, clan_nr, bonus);
            }
        }

        if let Some(CharacterDriverState::MilitaryMaster(data)) = self
            .characters
            .get_mut(&master_id)
            .and_then(|c| c.driver_state.as_mut())
        {
            data.last_clan_update += 60;
        }
    }

    /// C `dat->storage_data.quests_given[difficulty]++;`
    /// (`accept_mission`, `military.c:1348`) - the NPC-scoped mission-
    /// offer statistic `PlayerRuntime::accept_mission` itself
    /// deliberately skips (see that function's doc comment; it has no
    /// `master_id`/`World` access). Call once per successful
    /// [`AcceptMissionOutcome::Accepted`], mirroring C calling this
    /// unconditionally at the end of `accept_mission` (which itself only
    /// runs after every rejection branch has already returned). A no-op
    /// if `master_id` isn't a live `CDR_MILITARY_MASTER` NPC.
    pub fn record_mission_offered(&mut self, master_id: CharacterId, difficulty: usize) {
        if let Some(CharacterDriverState::MilitaryMaster(data)) = self
            .characters
            .get(&master_id)
            .and_then(|c| c.driver_state.as_ref())
        {
            let storage_id = data.storage_id;
            self.military_master_storage
                .add_quests_given(storage_id, difficulty);
        }
    }
}

/// C `process_advisor_recommendation`'s own difficulty-name ternary
/// (`military.c:1706-1710`), embedded in the greeting/accept-prompt
/// text: `pref == 0 ? "easy" : pref == 1 ? "normal" : pref == 2 ? "hard" :
/// pref == 3 ? "impossible" : "insane"`. Deliberately distinct from
/// [`mission_difficulty_name`]'s out-of-range fallback (`"easy"`, via
/// `get_colored_difficulty_name`'s own clamp) - this ternary instead
/// falls through to `"insane"` for anything other than `0..=3`, matching
/// C exactly (the caller already guards `mission_difficulty_preference
/// >= 0`, so only the upper side of the range can differ from
/// `mission_difficulty_name`).
fn advisor_recommendation_difficulty_text(preference: i32) -> &'static str {
    match preference {
        0 => "easy",
        1 => "normal",
        2 => "hard",
        3 => "impossible",
        _ => "insane",
    }
}

/// Outcome of [`World::process_advisor_recommendation`] (C
/// `process_advisor_recommendation`, `military.c:1685-1755`), mirroring
/// every distinct `say()` branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdvisorRecommendationOutcome {
    /// C: `ppd->recommend == yday + 1` -> already processed today, no
    /// text at all.
    AlreadyProcessed,
    /// C: `mission_type_preference > 0 && mission_difficulty_preference
    /// >= 0` branch - a paid-favor specific-mission recommendation.
    SpecificMission {
        /// The initial "Be greeted... oddly specific request..." line.
        greeting: String,
        /// [`describe_mission_text`]'s own text for the freshly
        /// (re)generated preferred slot - `None` if the mission slot
        /// ended up empty/unrecognized, matching C's own `describe_
        /// mission` silently returning `0` in that case (no text at
        /// all, not even a fallback line).
        description: Option<String>,
        /// The trailing conditional line: already-completed-today /
        /// active-mission-conflict / the "say X to accept" prompt.
        followup: String,
    },
    /// C: the `else` branch - one line per matching `advisor_last[n]`
    /// entry (`military.c:1748-1752`), possibly empty if none matched
    /// today, matching C's own unconditional loop (no "nothing to
    /// report" fallback line either).
    StandardRecommendations(Vec<String>),
}

impl World {
    /// C `process_advisor_recommendation(cn, co, ppd)`
    /// (`military.c:1685-1755`): the Military Master driver's per-visit
    /// paid-advisor-recommendation greeting, called just before
    /// [`crate::PlayerRuntime::greet_player`] in C's own `NT_CHAR`
    /// handler (`military.c:2150-2151`) - [`crate::PlayerRuntime::
    /// greet_player`]'s own `military_recommend() == yday + 1 &&
    /// mission_type_preference() > 0 && mission_difficulty_preference()
    /// >= 0` short-circuit (`GreetPlayerOutcome::
    /// AdvisorRecommendationAlreadyShown`) exists specifically to detect
    /// that this function already greeted the player this same call,
    /// matching C's own back-to-back `process_advisor_recommendation`/
    /// `greet_player` call order.
    ///
    /// Reuses [`handle_mission_request`]'s own rank-cubed `military_pts`
    /// floor / level-7 floor / [`crate::PlayerRuntime::
    /// apply_mission_offer`] pattern for the `mission_yday != yday + 1`
    /// regeneration branch (C's own `generate_mission_with_preference(co,
    /// ppd, ppd->mission_type_preference)` call, `military.c:1712-1714`,
    /// is the *full* ppd-mutating function - not the pure table-builder
    /// of the same name in this module - which does exactly that floor/
    /// clamp/stamp sequence internally).
    pub fn process_advisor_recommendation(
        &self,
        character_id: CharacterId,
        player: &mut PlayerRuntime,
        yday: i32,
        rng_seed: &mut u32,
        player_name: &str,
    ) -> AdvisorRecommendationOutcome {
        if player.military_recommend() == yday + 1 {
            return AdvisorRecommendationOutcome::AlreadyProcessed;
        }

        let outcome = if player.mission_type_preference() > 0
            && player.mission_difficulty_preference() >= 0
        {
            let preferred_type = player.mission_type_preference();
            let diff_pref = player.mission_difficulty_preference();
            let diff_text = advisor_recommendation_difficulty_text(diff_pref);
            let greeting = format!(
                "Be greeted, {player_name}. You have been recommended by my trusty advisor, with \
                 an oddly specific request for {diff_text} {}. Alas, thine wish be granted.",
                mission_type_name(preferred_type)
            );

            if player.mission_yday() != yday + 1 {
                if let Some(character) = self.characters.get(&character_id) {
                    let rank = army_rank_for_points(character.military_points);
                    let rank_cubed = rank.saturating_mul(rank).saturating_mul(rank);
                    if rank_cubed > player.military_pts() {
                        player.set_military_pts(rank_cubed);
                    }
                    let level = (character.level as i32).max(7);
                    let military_pts = player.military_pts();
                    player.apply_mission_offer(level, military_pts, preferred_type, yday, rng_seed);
                }
            }

            let diff_idx = diff_pref.max(0) as usize;
            let mission = player.military_mission(diff_idx);
            let description = describe_mission_text(&mission, diff_idx, player_name);

            let followup = if player.military_solved_yday() == yday + 1 {
                format!(
                    "However, you've already completed a mission today, {player_name}. Come \
                     back tomorrow and this mission will be waiting for you."
                )
            } else if player.military_took_mission() != 0 {
                format!(
                    "However, you already have an active mission, {player_name}. Complete or \
                     abandon it first, then come back to accept this one."
                )
            } else {
                format!("Say {diff_text} to accept this mission.")
            };

            AdvisorRecommendationOutcome::SpecificMission {
                greeting,
                description,
                followup,
            }
        } else {
            let mut lines = Vec::new();
            for n in 0..crate::player::MILITARY_PPD_MAXADVISOR {
                if player.military_advisor_last(n) == yday + 1 {
                    lines.push(format!(
                        "Be greeted, {player_name}. You have been recommended by my trusty \
                         advisor {n}"
                    ));
                }
            }
            AdvisorRecommendationOutcome::StandardRecommendations(lines)
        };

        player.set_military_recommend(yday + 1);
        outcome
    }
}

/// A `military_master_driver` outcome that needs `PlayerRuntime`'s
/// `military_ppd` (owned by `ugaris-server`'s session layer, outside
/// `World`'s visibility) to finish applying - see this module's sixth-
/// slice doc comment for why nearly every branch ends up here, unlike
/// `world/bank.rs`'s narrower `BankEvent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MilitaryMasterEvent {
    /// C `military_master_driver`'s `NT_CHAR` branch (`military.c:
    /// 2153-2177`, minus the still-unported `process_clan_recommendation`/
    /// `process_advisor_recommendation` calls - see this module's doc
    /// comment): greet, the `master_state == 1` rank-follow-up check,
    /// and `complete_mission`.
    NearbyPlayer {
        master_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa code 2 ("repeat"): `ppd->master_state = 0;`, no text.
    Repeat {
        master_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa code 10 ("mission"): [`World::handle_mission_request`].
    MissionRequest {
        master_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa codes 11-15 ("easy".."insane"): [`crate::PlayerRuntime::
    /// accept_mission`]. `difficulty` is `0..=4`.
    AcceptMission {
        master_id: CharacterId,
        player_id: CharacterId,
        difficulty: usize,
    },
    /// qa code 16 ("failed"): abandon the active mission.
    Failed {
        master_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa code 17 ("hear"): repeat the active mission's description.
    Hear {
        master_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa codes 22/"decline"/"new missions": [`World::mission_reroll`].
    Reroll {
        master_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa code 18 ("info", admin-only, `military.c:2037-2059`): dumps the
    /// speaker's own `military_pts`/`normal_exp` plus the master NPC's
    /// storage-scoped clan points and per-difficulty quest statistics via
    /// consecutive `say()` lines. Only queued when the speaker has
    /// `CF_GOD` (`ugaris_core::character::CharacterFlags::GOD`), matching
    /// C's own guard - a non-admin typing "info" gets silent no-op
    /// handling here exactly like C's `break;` with no output.
    Info {
        master_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa code 19 ("reset", admin-only, `military.c:2068-2075`):
    /// `ppd->solved_yday = ppd->mission_yday = 0`, no text. Admin-gated
    /// the same way as [`Self::Info`].
    Reset { player_id: CharacterId },
    /// qa code 20 ("raise", admin-only, `military.c:2076-2082`):
    /// `ppd->military_pts += 1000`, no text. Admin-gated the same way as
    /// [`Self::Info`].
    Raise { player_id: CharacterId },
    /// qa code 21 ("promote", admin-only, `military.c:2083-2089`):
    /// `give_military_pts(cn, co, 100, 1)`. Admin-gated the same way as
    /// [`Self::Info`].
    Promote {
        master_id: CharacterId,
        player_id: CharacterId,
    },
}

impl World {
    pub fn drain_pending_military_master_events(&mut self) -> Vec<MilitaryMasterEvent> {
        self.pending_military_master_events.drain(..).collect()
    }

    /// C `military_master_driver`'s `NT_TEXT`/`NT_GIVE` message loop
    /// (`military.c:2178-2198`). `NT_CHAR` is handled separately by
    /// [`Self::greet_nearby_military_master_players`] (see this module's
    /// doc comment).
    fn process_military_master_messages(&mut self, master_id: CharacterId) {
        let Some(master) = self.characters.get(&master_id).cloned() else {
            return;
        };
        let messages = {
            let Some(master_mut) = self.characters.get_mut(&master_id) else {
                return;
            };
            std::mem::take(&mut master_mut.driver_messages)
        };

        let mut destroy_cursor = false;
        let mut replies: Vec<String> = Vec::new();
        let mut events: Vec<MilitaryMasterEvent> = Vec::new();

        for message in messages {
            match message.message_type {
                NT_TEXT => {
                    let speaker_id = CharacterId(message.dat3 as u32);
                    if speaker_id == master_id {
                        continue;
                    }
                    let Some(text) = message.text.as_deref() else {
                        continue;
                    };
                    let Some(speaker) = self.characters.get(&speaker_id) else {
                        continue;
                    };
                    if !speaker.flags.contains(CharacterFlags::PLAYER) {
                        continue;
                    }
                    if char_dist(&master, speaker) > MILITARY_MASTER_TEXT_DISTANCE {
                        continue;
                    }
                    if !char_see_char(&master, speaker, &self.map, self.date.daylight) {
                        continue;
                    }
                    let speaker_name = speaker.name.clone();

                    match analyse_text_qa(text, &master.name, &speaker_name, MILITARY_QA) {
                        TextAnalysisOutcome::Said(reply) => replies.push(reply),
                        // C: `answer_code == 1` -> `quiet_say(cn, "I'm
                        // %s.", ch[cn].name)`.
                        TextAnalysisOutcome::Matched(1) => {
                            replies.push(format!("I'm {}.", master.name));
                        }
                        TextAnalysisOutcome::Matched(2) => {
                            events.push(MilitaryMasterEvent::Repeat {
                                master_id,
                                player_id: speaker_id,
                            });
                        }
                        TextAnalysisOutcome::Matched(10) => {
                            events.push(MilitaryMasterEvent::MissionRequest {
                                master_id,
                                player_id: speaker_id,
                            });
                        }
                        TextAnalysisOutcome::Matched(code @ 11..=15) => {
                            events.push(MilitaryMasterEvent::AcceptMission {
                                master_id,
                                player_id: speaker_id,
                                difficulty: (code - 11) as usize,
                            });
                        }
                        TextAnalysisOutcome::Matched(16) => {
                            events.push(MilitaryMasterEvent::Failed {
                                master_id,
                                player_id: speaker_id,
                            });
                        }
                        TextAnalysisOutcome::Matched(17) => {
                            events.push(MilitaryMasterEvent::Hear {
                                master_id,
                                player_id: speaker_id,
                            });
                        }
                        TextAnalysisOutcome::Matched(22) => {
                            events.push(MilitaryMasterEvent::Reroll {
                                master_id,
                                player_id: speaker_id,
                            });
                        }
                        // C: `if (!(ch[co].flags & CF_GOD)) { break; }` -
                        // every admin-only code 18-21 guards identically,
                        // silently dropping the message with no output
                        // for a non-admin speaker (matching C's `break`
                        // out of the `switch`, still `return 1` overall).
                        TextAnalysisOutcome::Matched(code @ 18..=21)
                            if speaker.flags.contains(CharacterFlags::GOD) =>
                        {
                            match code {
                                18 => events.push(MilitaryMasterEvent::Info {
                                    master_id,
                                    player_id: speaker_id,
                                }),
                                19 => events.push(MilitaryMasterEvent::Reset {
                                    player_id: speaker_id,
                                }),
                                20 => events.push(MilitaryMasterEvent::Raise {
                                    player_id: speaker_id,
                                }),
                                21 => events.push(MilitaryMasterEvent::Promote {
                                    master_id,
                                    player_id: speaker_id,
                                }),
                                _ => unreachable!(),
                            }
                        }
                        // Advisor-only codes (3-9, 30-44) and any
                        // unmatched text: no handling, matches C's own
                        // `default: return 0`.
                        TextAnalysisOutcome::Matched(_) | TextAnalysisOutcome::NoMatch => {}
                    }
                }
                NT_GIVE => {
                    destroy_cursor = true;
                    replies.push("That's junk.".to_string());
                }
                _ => {}
            }
        }

        if destroy_cursor {
            let cursor = self
                .characters
                .get_mut(&master_id)
                .and_then(|master| master.cursor_item.take());
            if let Some(item_id) = cursor {
                self.destroy_item(item_id);
            }
        }

        for reply in replies {
            self.npc_quiet_say(master_id, &reply);
        }

        self.pending_military_master_events.extend(events);
    }

    /// C `military_master_driver`'s `NT_CHAR` greeting branch
    /// (`military.c:2153-2177`), ported as a periodic nearby-player scan
    /// (see this module's doc comment for why).
    fn greet_nearby_military_master_players(&mut self, master_id: CharacterId) {
        let Some(master) = self.characters.get(&master_id).cloned() else {
            return;
        };

        let mut nearby: Vec<CharacterId> = Vec::new();
        for character in self.characters.values() {
            if character.id == master_id || !character.flags.contains(CharacterFlags::PLAYER) {
                continue;
            }
            if char_dist(&master, character) > MILITARY_MASTER_GREET_DISTANCE {
                continue;
            }
            if !char_see_char(&master, character, &self.map, self.date.daylight) {
                continue;
            }
            nearby.push(character.id);
        }

        self.pending_military_master_events
            .extend(
                nearby
                    .into_iter()
                    .map(|player_id| MilitaryMasterEvent::NearbyPlayer {
                        master_id,
                        player_id,
                    }),
            );
    }

    /// C `military_master_driver`'s movement section (`military.c:
    /// 2200-2204`): stationary NPC returning to its `rest_x`/`rest_y`
    /// spawn tile, facing `DX_DOWN`. Unlike `world/bank.rs`'s day/night
    /// shop positions, C's own `struct military_master_data` has no
    /// movement fields at all, so this is always the "no configured
    /// position" fallback.
    fn process_military_master_tick_action(&mut self, master_id: CharacterId, area_id: u16) {
        let Some(master) = self.characters.get(&master_id).cloned() else {
            return;
        };
        if self.setup_walk_toward(
            master_id,
            usize::from(master.rest_x),
            usize::from(master.rest_y),
            0,
            area_id,
            false,
        ) {
            return;
        }
        if master.dir != MILITARY_MASTER_REST_DIRECTION {
            if let Some(master_mut) = self.characters.get_mut(&master_id) {
                let _ = turn(master_mut, MILITARY_MASTER_REST_DIRECTION);
            }
        }
    }

    /// Military Master NPC tick: process messages, greet/complete-
    /// mission scan, and the movement fallback. Ports the per-tick body
    /// of C `military_master_driver` (minus the deferred clan/advisor
    /// recommendation and storage-blob persistence - see this module's
    /// doc comment).
    pub fn process_military_master_actions(&mut self, area_id: u16, now: i64) {
        let master_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_MILITARY_MASTER
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        for master_id in master_ids {
            self.process_military_master_messages(master_id);
            self.greet_nearby_military_master_players(master_id);
            // C `update_clan_points(dat)` (`military.c:2195`): once per
            // NPC per tick, independent of any player message.
            self.update_clan_points(master_id, now);
            self.process_military_master_tick_action(master_id, area_id);
        }
    }
}

/// This seventh slice ports `CDR_MILITARY_ADVISOR`'s own driver
/// (`military_advisor_driver`, `military.c:2607-2699`), the paid
/// mission-recommendation NPC the previous slice's doc comment listed as
/// entirely unported: [`World::offer_favor`]/[`World::
/// handle_specific_mission_request`]/[`World::process_favor_payment`]
/// (the ppd-mutating halves of `offer_favor`/`handle_specific_mission_
/// request`/`process_favor_payment`), [`adv_introduction_text`]/
/// [`adv_favor_desc_lines`] (the dialogue-rendering halves of
/// `adv_introduction`/`adv_favor_desc`), and finally the driver itself:
/// [`MilitaryAdvisorEvent`]/[`World::process_military_advisor_actions`],
/// mirroring [`MilitaryMasterEvent`]/`process_military_master_actions`'s
/// shape exactly (same `World`/`PlayerRuntime` split, same periodic
/// nearby-player-scan simplification for `NT_CHAR`, same shared
/// [`crate::character_driver::MILITARY_QA`] table via
/// `analyse_text_qa`). Applied by `ugaris-server`'s
/// `apply_military_advisor_events`.
///
/// Deliberately out of scope for this slice (documented here, not
/// silently dropped - see the "Military ranks" task in
/// `PORTING_TODO.md`); closed in a later ("ninth") slice - see this
/// module's doc comment:
/// - The admin-only qa code 18 (`info`, `military.c:2525-2538`) - needed
///   the still-unported `dat->storage_data` sales-economy counters.
/// - `add_cost`'s sales-economy bookkeeping inside `process_favor_
///   payment`.
///
/// Still out of scope: `update_advisor_storage`/`process_advisor_storage`
/// (the periodic async-DB-blob persistence state machine, superseded by
/// the in-memory registry the same way `MilitaryMasterStorageRegistry`
/// supersedes `process_master_storage` - see [`CostData`]'s doc comment).
impl World {
    /// Reads the Advisor NPC's `storage_ID` (`military.c:369-375`'s
    /// `struct military_advisor_data`, stored via zone-file `storage=N;`
    /// parsing at spawn time - see `crate::zone`) out of its
    /// `driver_state`, defaulting to `0` for a not-yet-initialized or
    /// mismatched driver state (shouldn't happen for a real
    /// `CDR_MILITARY_ADVISOR` character, but mirrors every other
    /// `driver_state`-reading helper's defensive fallback in this
    /// codebase).
    pub fn advisor_storage_id(&self, advisor_id: CharacterId) -> i32 {
        self.characters
            .get(&advisor_id)
            .and_then(|character| character.driver_state.clone())
            .map(|state| match state {
                CharacterDriverState::MilitaryAdvisor(data) => data.storage_id,
                _ => 0,
            })
            .unwrap_or(0)
    }

    pub fn drain_pending_military_advisor_events(&mut self) -> Vec<MilitaryAdvisorEvent> {
        self.pending_military_advisor_events.drain(..).collect()
    }

    /// C `military_advisor_driver`'s `NT_TEXT`/`NT_GIVE` message loop
    /// (`military.c:2678-2691`), via `handle_advisor_message`
    /// (`military.c:2481-2605`). `NT_CHAR` is handled separately by
    /// [`Self::greet_nearby_military_advisor_players`] (matching
    /// `process_military_master_messages`'s split).
    fn process_military_advisor_messages(&mut self, advisor_id: CharacterId) {
        let Some(advisor) = self.characters.get(&advisor_id).cloned() else {
            return;
        };
        let messages = {
            let Some(advisor_mut) = self.characters.get_mut(&advisor_id) else {
                return;
            };
            std::mem::take(&mut advisor_mut.driver_messages)
        };

        let mut destroy_cursor = false;
        let mut replies: Vec<String> = Vec::new();
        let mut events: Vec<MilitaryAdvisorEvent> = Vec::new();

        for message in messages {
            match message.message_type {
                NT_TEXT => {
                    let speaker_id = CharacterId(message.dat3 as u32);
                    if speaker_id == advisor_id {
                        continue;
                    }
                    let Some(text) = message.text.as_deref() else {
                        continue;
                    };
                    let Some(speaker) = self.characters.get(&speaker_id) else {
                        continue;
                    };
                    if !speaker.flags.contains(CharacterFlags::PLAYER) {
                        continue;
                    }
                    if char_dist(&advisor, speaker) > MILITARY_MASTER_TEXT_DISTANCE {
                        continue;
                    }
                    if !char_see_char(&advisor, speaker, &self.map, self.date.daylight) {
                        continue;
                    }
                    let speaker_name = speaker.name.clone();

                    match analyse_text_qa(text, &advisor.name, &speaker_name, MILITARY_QA) {
                        TextAnalysisOutcome::Said(reply) => replies.push(reply),
                        // C: `answer_code == 1` -> `quiet_say(cn, "I'm
                        // %s.", ch[cn].name)`.
                        TextAnalysisOutcome::Matched(1) => {
                            replies.push(format!("I'm {}.", advisor.name));
                        }
                        TextAnalysisOutcome::Matched(2) => {
                            events.push(MilitaryAdvisorEvent::Repeat {
                                advisor_id,
                                player_id: speaker_id,
                            });
                        }
                        TextAnalysisOutcome::Matched(3) => {
                            events.push(MilitaryAdvisorEvent::FavorDesc {
                                advisor_id,
                                player_id: speaker_id,
                            });
                        }
                        TextAnalysisOutcome::Matched(code @ 4..=8) => {
                            events.push(MilitaryAdvisorEvent::Favor {
                                advisor_id,
                                player_id: speaker_id,
                                favor_size: (code - 4),
                            });
                        }
                        TextAnalysisOutcome::Matched(9) => {
                            events.push(MilitaryAdvisorEvent::Pay {
                                advisor_id,
                                player_id: speaker_id,
                            });
                        }
                        TextAnalysisOutcome::Matched(code @ 30..=44) => {
                            let offset = code - 30;
                            let mission_type = 1 + offset / 5;
                            let difficulty = offset % 5;
                            events.push(MilitaryAdvisorEvent::SpecificMissionRequest {
                                advisor_id,
                                player_id: speaker_id,
                                difficulty,
                                mission_type,
                            });
                        }
                        // C: `if (!(ch[co].flags & CF_GOD)) { break; }`
                        // (`military.c:2523-2525`), same admin-only guard
                        // shape as the Master driver's codes 18-21.
                        TextAnalysisOutcome::Matched(18)
                            if speaker.flags.contains(CharacterFlags::GOD) =>
                        {
                            events.push(MilitaryAdvisorEvent::Info {
                                advisor_id,
                                player_id: speaker_id,
                            });
                        }
                        // Master-only codes (10-17, 19-22), a non-admin
                        // speaker's "info" (18), and any unmatched text:
                        // no handling, matches C's own `default: return
                        // 0;` / admin-gate `break;`.
                        TextAnalysisOutcome::Matched(_) | TextAnalysisOutcome::NoMatch => {}
                    }
                }
                NT_GIVE => {
                    destroy_cursor = true;
                    replies.push("That's junk.".to_string());
                }
                _ => {}
            }
        }

        if destroy_cursor {
            let cursor = self
                .characters
                .get_mut(&advisor_id)
                .and_then(|advisor| advisor.cursor_item.take());
            if let Some(item_id) = cursor {
                self.destroy_item(item_id);
            }
        }

        for reply in replies {
            self.npc_quiet_say(advisor_id, &reply);
        }

        self.pending_military_advisor_events.extend(events);
    }

    /// C `military_advisor_driver`'s `NT_CHAR` greeting branch
    /// (`military.c:2639-2661`), ported as the same periodic
    /// nearby-player-scan simplification
    /// [`Self::greet_nearby_military_master_players`] already
    /// established.
    fn greet_nearby_military_advisor_players(&mut self, advisor_id: CharacterId) {
        let Some(advisor) = self.characters.get(&advisor_id).cloned() else {
            return;
        };

        let mut nearby: Vec<CharacterId> = Vec::new();
        for character in self.characters.values() {
            if character.id == advisor_id || !character.flags.contains(CharacterFlags::PLAYER) {
                continue;
            }
            if char_dist(&advisor, character) > MILITARY_MASTER_GREET_DISTANCE {
                continue;
            }
            if !char_see_char(&advisor, character, &self.map, self.date.daylight) {
                continue;
            }
            nearby.push(character.id);
        }

        self.pending_military_advisor_events
            .extend(
                nearby
                    .into_iter()
                    .map(|player_id| MilitaryAdvisorEvent::NearbyPlayer {
                        advisor_id,
                        player_id,
                    }),
            );
    }

    /// C `military_advisor_driver`'s movement section (`military.c:
    /// 2696-2699`): stationary NPC returning to its `rest_x`/`rest_y`
    /// spawn tile, facing `DX_RIGHT` - unlike the Military Master's
    /// `DX_DOWN`, a genuine (if arbitrary) C difference between the two
    /// drivers, preserved verbatim.
    fn process_military_advisor_tick_action(&mut self, advisor_id: CharacterId, area_id: u16) {
        let Some(advisor) = self.characters.get(&advisor_id).cloned() else {
            return;
        };
        if self.setup_walk_toward(
            advisor_id,
            usize::from(advisor.rest_x),
            usize::from(advisor.rest_y),
            0,
            area_id,
            false,
        ) {
            return;
        }
        if advisor.dir != MILITARY_ADVISOR_REST_DIRECTION {
            if let Some(advisor_mut) = self.characters.get_mut(&advisor_id) {
                let _ = turn(advisor_mut, MILITARY_ADVISOR_REST_DIRECTION);
            }
        }
    }

    /// Military Advisor NPC tick: process messages, greet scan, and the
    /// movement fallback. Ports the per-tick body of C
    /// `military_advisor_driver` (minus the still-unported storage-blob
    /// async persistence state machine - see this module's doc comment).
    pub fn process_military_advisor_actions(&mut self, area_id: u16) {
        let advisor_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_MILITARY_ADVISOR
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        for advisor_id in advisor_ids {
            self.process_military_advisor_messages(advisor_id);
            self.greet_nearby_military_advisor_players(advisor_id);
            self.process_military_advisor_tick_action(advisor_id, area_id);
        }
    }
}

/// C `DX_RIGHT` (`common/direction.h:21`): the Military Advisor's fixed
/// resting facing (C's own `secure_move_driver(cn, ch[cn].tmpx,
/// ch[cn].tmpy, DX_RIGHT, ret, lastact)`, `military.c:2698`).
const MILITARY_ADVISOR_REST_DIRECTION: u8 = 4;

/// A `military_advisor_driver` outcome that needs `PlayerRuntime`'s
/// `military_ppd` to finish applying - see this module's seventh-slice
/// doc comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MilitaryAdvisorEvent {
    /// C `military_advisor_driver`'s `NT_CHAR` branch (`military.c:
    /// 2639-2661`): the initial-greeting/`current_advisor` stamp.
    NearbyPlayer {
        advisor_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa code 2 ("repeat"): `ppd->advisor_state = 0;`, no text.
    Repeat {
        advisor_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa code 3 ("favor"): [`adv_favor_desc_lines`], gated on
    /// `advisor_last[idx]`.
    FavorDesc {
        advisor_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa codes 4-8 ("small".."vast"): [`World::offer_favor`].
    /// `favor_size` is `0..=4`.
    Favor {
        advisor_id: CharacterId,
        player_id: CharacterId,
        favor_size: i32,
    },
    /// qa code 9 ("pay"): [`World::process_favor_payment`].
    Pay {
        advisor_id: CharacterId,
        player_id: CharacterId,
    },
    /// qa codes 30-44 (e.g. "easy demon".."insane silver"):
    /// [`World::handle_specific_mission_request`]. `difficulty` is
    /// `0..=4`, `mission_type` is `1..=3`.
    SpecificMissionRequest {
        advisor_id: CharacterId,
        player_id: CharacterId,
        difficulty: i32,
        mission_type: i32,
    },
    /// Admin-only qa code 18 ("info", `military.c:2525-2538`): shows the
    /// speaker each favor size's sales stats (only nonzero-`sold` sizes),
    /// via [`MilitaryAdvisorStorageRegistry`].
    Info {
        advisor_id: CharacterId,
        player_id: CharacterId,
    },
}
