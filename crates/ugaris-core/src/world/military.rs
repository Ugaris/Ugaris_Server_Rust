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

use super::*;

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
/// mission-offer statistic) has no Rust equivalent yet - `military_master_
/// data` itself is still unported (see this module's doc comment) - so
/// that counter is simply not incremented anywhere.
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
    /// needed for plain system text). Skips `dat->storage_data.quests_
    /// solved/pts_given/exp_given[difficulty]` (the NPC-scoped statistics -
    /// no Rust `military_master_data` equivalent yet) and the wealth-
    /// achievement ladder the real `give_money` also updates (that needs
    /// `add_gold_earned`'s DB-backed first-unlock announce, which lives in
    /// the server crate - wire it at the same time a real Military Master
    /// NPC driver call site lands).
    pub fn complete_mission(
        &mut self,
        character_id: CharacterId,
        player: &mut PlayerRuntime,
        area_id: u32,
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
