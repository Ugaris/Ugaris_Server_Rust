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
//!
//! This tenth slice closes the wealth-achievement ladder gap
//! [`World::complete_mission`]'s own doc comment flagged: C's
//! `complete_mission` pays its mercenary bonus gold through `give_money`
//! (`military.c:1391`), which (`tool.c:1475-1481`) also tracks
//! `achievement_add_gold_earned` whenever the payout is positive and the
//! character is a player - `complete_mission` itself only ports
//! `give_money`'s inlined gold-add/message half (achievement tracking
//! needs the DB-backed first-unlock announce, which lives in the server
//! crate). Wired at `ugaris-server`'s `apply_military_master_nearby_
//! player`, the one real call site: a `Completed` outcome with
//! `gold_awarded > 0` now calls the already-existing
//! `award_swap_money_converted_achievement` helper (same "silver amount,
//! `CF_PLAYER`-gated, `/100` integer division" shape `swap`'s `IF_MONEY`
//! branch already uses) with `gold_awarded` as the silver price.

mod military_advisor;
mod military_master;
mod missions;

pub use military_advisor::*;
pub use military_master::*;
pub use missions::*;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome};

use crate::text::{COL_STR_LIGHT_BLUE, COL_STR_RESET};

use crate::world::*;

// ---- legacy driver registry surface (moved from character_driver.rs) ----

#[allow(unused_imports)]
use crate::character_driver::*;

/// C `struct qa qa[]` from `src/module/military.c:89-164`, shared verbatim
/// by both `military_master_driver` and (once ported)
/// `military_advisor_driver`. Note `"help"`'s answer is the same
/// copy-pasted `"Sorry, I'm just a merchant, %s!"` line every other
/// `qa[]` table carries, even though neither NPC is a merchant -
/// preserved verbatim per the porting rule to copy quirks, not "fix"
/// them. `COL_LIGHT_BLUE`/`COL_RESET` markers around a few keywords in
/// C's own `say()` calls (not this table's `answer` strings, which carry
/// none) are dropped at the call sites that render them, same as
/// `BANK_QA`'s doc comment explains.
pub const MILITARY_QA: &[TextQaEntry] = &[
    TextQaEntry {
        words: &["how", "are", "you"],
        answer: Some("I'm fine!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hello"],
        answer: Some("Hello, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hi"],
        answer: Some("Hi, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["greetings"],
        answer: Some("Greetings, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hail"],
        answer: Some("And hail to you, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["help"],
        answer: Some("Sorry, I'm just a merchant, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["what's", "up"],
        answer: Some("Everything that isn't nailed down."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["what", "is", "up"],
        answer: Some("Everything that isn't nailed down."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["what's", "your", "name"],
        answer: None,
        answer_code: 1,
    },
    TextQaEntry {
        words: &["what", "is", "your", "name"],
        answer: None,
        answer_code: 1,
    },
    TextQaEntry {
        words: &["who", "are", "you"],
        answer: None,
        answer_code: 1,
    },
    TextQaEntry {
        words: &["repeat"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["favor"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["small"],
        answer: None,
        answer_code: 4,
    },
    TextQaEntry {
        words: &["medium"],
        answer: None,
        answer_code: 5,
    },
    TextQaEntry {
        words: &["big"],
        answer: None,
        answer_code: 6,
    },
    TextQaEntry {
        words: &["huge"],
        answer: None,
        answer_code: 7,
    },
    TextQaEntry {
        words: &["vast"],
        answer: None,
        answer_code: 8,
    },
    TextQaEntry {
        words: &["pay"],
        answer: None,
        answer_code: 9,
    },
    TextQaEntry {
        words: &["mission"],
        answer: None,
        answer_code: 10,
    },
    TextQaEntry {
        words: &["easy"],
        answer: None,
        answer_code: 11,
    },
    TextQaEntry {
        words: &["normal"],
        answer: None,
        answer_code: 12,
    },
    TextQaEntry {
        words: &["hard"],
        answer: None,
        answer_code: 13,
    },
    TextQaEntry {
        words: &["impossible"],
        answer: None,
        answer_code: 14,
    },
    TextQaEntry {
        words: &["insane"],
        answer: None,
        answer_code: 15,
    },
    TextQaEntry {
        words: &["failed"],
        answer: None,
        answer_code: 16,
    },
    TextQaEntry {
        words: &["hear"],
        answer: None,
        answer_code: 17,
    },
    TextQaEntry {
        words: &["info"],
        answer: None,
        answer_code: 18,
    },
    TextQaEntry {
        words: &["reset"],
        answer: None,
        answer_code: 19,
    },
    TextQaEntry {
        words: &["raise"],
        answer: None,
        answer_code: 20,
    },
    TextQaEntry {
        words: &["promote"],
        answer: None,
        answer_code: 21,
    },
    TextQaEntry {
        words: &["reroll"],
        answer: None,
        answer_code: 22,
    },
    TextQaEntry {
        words: &["decline"],
        answer: None,
        answer_code: 22,
    },
    TextQaEntry {
        words: &["new", "missions"],
        answer: None,
        answer_code: 22,
    },
    TextQaEntry {
        words: &["easy", "demon"],
        answer: None,
        answer_code: 30,
    },
    TextQaEntry {
        words: &["easy", "pentagram"],
        answer: None,
        answer_code: 30,
    },
    TextQaEntry {
        words: &["normal", "demon"],
        answer: None,
        answer_code: 31,
    },
    TextQaEntry {
        words: &["normal", "pentagram"],
        answer: None,
        answer_code: 31,
    },
    TextQaEntry {
        words: &["hard", "demon"],
        answer: None,
        answer_code: 32,
    },
    TextQaEntry {
        words: &["hard", "pentagram"],
        answer: None,
        answer_code: 32,
    },
    TextQaEntry {
        words: &["impossible", "demon"],
        answer: None,
        answer_code: 33,
    },
    TextQaEntry {
        words: &["impossible", "pentagram"],
        answer: None,
        answer_code: 33,
    },
    TextQaEntry {
        words: &["insane", "demon"],
        answer: None,
        answer_code: 34,
    },
    TextQaEntry {
        words: &["insane", "pentagram"],
        answer: None,
        answer_code: 34,
    },
    TextQaEntry {
        words: &["easy", "ratling"],
        answer: None,
        answer_code: 35,
    },
    TextQaEntry {
        words: &["easy", "rats"],
        answer: None,
        answer_code: 35,
    },
    TextQaEntry {
        words: &["normal", "ratling"],
        answer: None,
        answer_code: 36,
    },
    TextQaEntry {
        words: &["normal", "rats"],
        answer: None,
        answer_code: 36,
    },
    TextQaEntry {
        words: &["hard", "ratling"],
        answer: None,
        answer_code: 37,
    },
    TextQaEntry {
        words: &["hard", "rats"],
        answer: None,
        answer_code: 37,
    },
    TextQaEntry {
        words: &["impossible", "ratling"],
        answer: None,
        answer_code: 38,
    },
    TextQaEntry {
        words: &["impossible", "rats"],
        answer: None,
        answer_code: 38,
    },
    TextQaEntry {
        words: &["insane", "ratling"],
        answer: None,
        answer_code: 39,
    },
    TextQaEntry {
        words: &["insane", "rats"],
        answer: None,
        answer_code: 39,
    },
    TextQaEntry {
        words: &["easy", "silver"],
        answer: None,
        answer_code: 40,
    },
    TextQaEntry {
        words: &["easy", "mining"],
        answer: None,
        answer_code: 40,
    },
    TextQaEntry {
        words: &["normal", "silver"],
        answer: None,
        answer_code: 41,
    },
    TextQaEntry {
        words: &["normal", "mining"],
        answer: None,
        answer_code: 41,
    },
    TextQaEntry {
        words: &["hard", "silver"],
        answer: None,
        answer_code: 42,
    },
    TextQaEntry {
        words: &["hard", "mining"],
        answer: None,
        answer_code: 42,
    },
    TextQaEntry {
        words: &["impossible", "silver"],
        answer: None,
        answer_code: 43,
    },
    TextQaEntry {
        words: &["impossible", "mining"],
        answer: None,
        answer_code: 43,
    },
    TextQaEntry {
        words: &["insane", "silver"],
        answer: None,
        answer_code: 44,
    },
    TextQaEntry {
        words: &["insane", "mining"],
        answer: None,
        answer_code: 44,
    },
];
