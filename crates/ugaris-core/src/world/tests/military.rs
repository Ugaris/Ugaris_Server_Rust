use super::*;
use crate::character_driver::{
    parse_military_advisor_driver_args, parse_military_master_driver_args,
    MilitaryAdvisorDriverData, MilitaryMasterDriverData, CDR_MILITARY_ADVISOR, CDR_MILITARY_MASTER,
};
use crate::clan::CLAN_BONUS_MILITARY_ADVISOR;

// C `get_army_rank_int` (`tool.c:2023-2035`): `cbrt(military_pts)` clamped
// to `[0, MAX_ARMY_RANK]`.
#[test]
fn army_rank_for_points_matches_cube_root_thresholds() {
    assert_eq!(army_rank_for_points(0), 0);
    assert_eq!(army_rank_for_points(-5), 0);
    assert_eq!(army_rank_for_points(7), 1);
    assert_eq!(army_rank_for_points(8), 2);
    assert_eq!(army_rank_for_points(999), 9);
    assert_eq!(army_rank_for_points(1000), 10);
    assert_eq!(army_rank_for_points(64_000), 40);
    // Past the max-rank cube (41^3 = 68921), the raw cube root exceeds
    // MAX_ARMY_RANK; C's `set_army_rank` clamps via `min(MAX_ARMY_RANK,
    // rank)`, so the effective rank stays capped at 40, never higher.
    assert_eq!(army_rank_for_points(68_921), MAX_ARMY_RANK);
    assert_eq!(army_rank_for_points(1_000_000), MAX_ARMY_RANK);
}

// C `tool.c:1868-1907`'s `rankname[]` table, spot-checked letter for
// letter at both ends and a couple of interior entries.
#[test]
fn army_rank_name_matches_legacy_table() {
    assert_eq!(army_rank_name(0), "nobody");
    assert_eq!(army_rank_name(1), "Private");
    assert_eq!(army_rank_name(10), "Second Lieutenant");
    assert_eq!(army_rank_name(20), "Field Marshal");
    assert_eq!(army_rank_name(40), "Avatar of Astonia");
    // Out-of-range ranks clamp instead of panicking (defensive; C's own
    // `rankname[min(MAX_ARMY_RANK, ppd->army_rank)]` never overshoots
    // since `army_rank` itself is always clamped by `set_army_rank`).
    assert_eq!(army_rank_name(999), "Avatar of Astonia");
}

#[test]
fn give_military_pts_adds_points_and_exp_without_promotion_below_threshold() {
    let mut world = World::default();
    let player = character(1);
    assert!(world.spawn_character(player, 10, 10));

    let award = world.give_military_pts(CharacterId(1), 0, 1, 3);

    assert!(!award.promoted());
    assert_eq!(award.old_rank, 0);
    assert_eq!(award.new_rank, 0);
    let character = &world.characters[&CharacterId(1)];
    assert_eq!(character.military_points, 0);
    assert_eq!(character.military_normal_exp, 1);
    assert_eq!(character.exp, 1);
    assert!(world.drain_pending_system_texts().is_empty());
    assert!(world.drain_pending_channel_broadcasts().is_empty());
}

// C `give_military_pts_no_npc` (`tool.c:3279-3306`): crossing a rank
// threshold queues the "You've been promoted..." system text.
#[test]
fn give_military_pts_promotes_and_queues_feedback_text() {
    let mut world = World::default();
    let player = character(1);
    assert!(world.spawn_character(player, 10, 10));

    let award = world.give_military_pts(CharacterId(1), 8, 0, 3);

    assert!(award.promoted());
    assert_eq!(award.old_rank, 0);
    assert_eq!(award.new_rank, 2);
    let feedback = world.drain_pending_system_texts();
    assert_eq!(feedback.len(), 1);
    assert_eq!(
        feedback[0].message,
        "You've been promoted to Private First Class. Congratulations, Character!"
    );
    // Rank 2 is below the Sergeant Major (index 9) announce threshold, so
    // no server-wide broadcast is queued.
    assert!(world.drain_pending_channel_broadcasts().is_empty());
}

// C: `if (get_army_rank_int(co) > 9)` gates the server-wide "Grats: NAME
// is a X now!" channel-6 broadcast (`tool.c:3273-3275`).
#[test]
fn give_military_pts_above_rank_nine_also_broadcasts_server_wide() {
    let mut world = World::default();
    let player = character(1);
    assert!(world.spawn_character(player, 10, 10));

    let award = world.give_military_pts(CharacterId(1), 1000, 0, 3);

    assert!(award.promoted());
    assert_eq!(award.new_rank, 10);
    let broadcasts = world.drain_pending_channel_broadcasts();
    assert_eq!(broadcasts.len(), 1);
    assert_eq!(broadcasts[0].channel, 6);
    let mut expected = b"0000000000".to_vec();
    expected.extend_from_slice(crate::text::COL_CHAT_GRATS);
    expected.extend_from_slice(b"Grats: Character is a Second Lieutenant now!");
    assert_eq!(broadcasts[0].message_bytes, expected);
}

// C `give_military_pts_no_npc`: `pts` gets the hardcore military bonus
// multiplier (`hardcore_military_exp_bonus`), distinct from the normal
// exp hardcore bonus that `give_exp` applies to `exps` internally.
#[test]
fn give_military_pts_applies_hardcore_bonus_only_to_points_not_recorded_exp() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::HARDCORE);
    assert!(world.spawn_character(player, 10, 10));
    world.settings.hardcore_military_exp_bonus = 2.0;

    world.give_military_pts(CharacterId(1), 10, 5, 3);

    let character = &world.characters[&CharacterId(1)];
    assert_eq!(character.military_points, 20, "10 pts * 2.0 hardcore bonus");
    // C: `ppd->normal_exp += exps` uses the raw argument, not whatever
    // `give_exp` internally scaled the real exp total by.
    assert_eq!(character.military_normal_exp, 5);
}

#[test]
fn give_military_pts_on_unknown_character_is_a_no_op() {
    let mut world = World::default();
    let award = world.give_military_pts(CharacterId(99), 100, 5, 3);
    assert_eq!(award, MilitaryPointsAward::default());
    assert!(world.drain_pending_system_texts().is_empty());
}

// C `specific_mission_price` (`military.c:392-467`): difficulty and type
// multipliers, plus the per-difficulty minimum price floor.
#[test]
fn specific_mission_price_applies_difficulty_and_type_multipliers() {
    // level 100: base_price = 100*100/10 + 100*5 = 1000+500 = 1500.
    // level_scaling = min(1.0, 100/100) = 1.0, clamped to max(0.5, 1.0) = 1.0.
    // factor = (1.0 - (1.0-1.0)*0.5) = 1.0.
    assert_eq!(specific_mission_price(100, 2, 1), 1500); // hard, demon: 1500*1.0*1.0
    assert_eq!(specific_mission_price(100, 0, 1), 600); // easy: 1500*0.4 = 600
                                                        // insane: 1500*1.8 = 2700, but the insane floor is 3000, which wins.
    assert_eq!(specific_mission_price(100, 4, 1), 3000);
    assert_eq!(specific_mission_price(100, 2, 2), 1650); // ratling: 1500*1.1
    assert_eq!(specific_mission_price(100, 2, 3), 1800); // silver: 1500*1.2
}

#[test]
fn specific_mission_price_never_drops_below_the_difficulty_floor() {
    // Very low level -> tiny base_price, floor takes over for every
    // difficulty (200/400/800/1500/3000).
    assert_eq!(specific_mission_price(1, 0, 1), 200);
    assert_eq!(specific_mission_price(1, 1, 1), 400);
    assert_eq!(specific_mission_price(1, 2, 1), 800);
    assert_eq!(specific_mission_price(1, 3, 1), 1500);
    assert_eq!(specific_mission_price(1, 4, 1), 3000);
}

// C `get_level_experience_cap` (`military.c:580-609`).
#[test]
fn level_experience_cap_clamps_at_both_ends_and_boundaries() {
    assert_eq!(get_level_experience_cap(0), 1000);
    assert_eq!(get_level_experience_cap(-5), 1000);
    assert_eq!(get_level_experience_cap(200), 100_000);
    assert_eq!(get_level_experience_cap(500), 100_000);
    // level 100: current=100^4=100_000_000, next=101^4=104_060_401,
    // diff=4_060_401, cap = diff*15/100 = 609_060 (integer truncation).
    assert_eq!(get_level_experience_cap(100), 609_060);
    // level 1: current=1, next=16, diff=15, 15*15/100=2 -> below the 1000
    // floor, clamps up.
    assert_eq!(get_level_experience_cap(1), 1000);
}

// C `get_minimum_expected_rank`/`get_maximum_reasonable_rank`
// (`military.c:618-681`): spot-check every threshold boundary.
#[test]
fn minimum_and_maximum_expected_rank_thresholds() {
    assert_eq!(get_minimum_expected_rank(15), 0);
    assert_eq!(get_minimum_expected_rank(16), 2);
    assert_eq!(get_minimum_expected_rank(100), 16);
    assert_eq!(get_minimum_expected_rank(151), 22);

    assert_eq!(get_maximum_reasonable_rank(15), 3);
    assert_eq!(get_maximum_reasonable_rank(16), 6);
    assert_eq!(get_maximum_reasonable_rank(100), 20);
    assert_eq!(get_maximum_reasonable_rank(151), MAX_ARMY_RANK);
}

// C `get_expected_level_for_rank` (`military.c:690-725`).
#[test]
fn expected_level_for_rank_matches_the_piecewise_table() {
    assert_eq!(get_expected_level_for_rank(0), 7);
    assert_eq!(get_expected_level_for_rank(5), 15 + 5 * 3);
    assert_eq!(get_expected_level_for_rank(8), 30 + 3 * 5);
    assert_eq!(get_expected_level_for_rank(10), 45 + 2 * 5);
    assert_eq!(get_expected_level_for_rank(20), 55 + 10 * 5);
    assert_eq!(get_expected_level_for_rank(24), 105 + 4 * 5);
    assert_eq!(get_expected_level_for_rank(30), 125 + 6 * 5);
    assert_eq!(get_expected_level_for_rank(35), 155 + 5 * 6);
    assert_eq!(get_expected_level_for_rank(40), 185 + 5 * 3);
    assert_eq!(get_expected_level_for_rank(41), 200);
}

// C `get_enhanced_level_scaling_factor` (`military.c:734-757`).
#[test]
fn enhanced_level_scaling_factor_rewards_perfect_progression() {
    // rank 10's expected level is 55; level 55 is a perfect match.
    assert_eq!(get_enhanced_level_scaling_factor(55, 10), 1.5);
    // 10 levels off (45) still within min/max rank band for level 45
    // (min 6, max 12) -> "good" tier.
    assert_eq!(get_enhanced_level_scaling_factor(45, 10), 1.25);
    // Rank far outside the level's reasonable band always falls back to
    // neutral, regardless of level_diff.
    assert_eq!(get_enhanced_level_scaling_factor(55, 0), 1.0);
}

// C `calculate_mission_exp` (`military.c:767-785`).
#[test]
fn calculate_mission_exp_matches_hand_computed_values() {
    // military_pts=0 -> cbrt=0, rank=0; level 100's min/max rank band is
    // [16,20], rank 0 is outside it -> neutral 1.0 scaling.
    // base_exp = 1 * (0+5)^4 / 16 = 625/16 = 39 (truncated).
    assert_eq!(calculate_mission_exp(0, 1, 100), 39);

    // military_pts=1000 -> cbrt=10 exactly; rank 10's expected level (55)
    // exactly matches player level 55, and rank 10 is within level 55's
    // min/max band [8,16] -> 1.5x bonus.
    // base_exp = 4 * (10+5)^4 / 16 = 4*50625/16 = 12656 (truncated),
    // scaled = 12656 * 1.5 = 18984.
    assert_eq!(calculate_mission_exp(1000, 4, 55), 18984);
}

#[test]
fn calculate_mission_exp_never_returns_below_one_or_above_the_level_cap() {
    // Absurdly high difficulty_pts/military_pts must still clamp to the
    // level cap, and a pathological negative-ish result still floors at 1.
    let capped = calculate_mission_exp(1_000_000, 25, 10);
    assert_eq!(capped, get_level_experience_cap(10));
    assert!(calculate_mission_exp(0, 0, 50) >= 1);
}

// C `generate_single_demon_mission` (`military.c:795-839`): always
// available, opt2 caps at 118 (+1/+2 for higher difficulties).
#[test]
fn generate_single_demon_mission_covers_every_difficulty() {
    let mut seed = 42u32;
    for difficulty in 0..5 {
        let mission = generate_single_demon_mission(50, 0, difficulty, &mut seed);
        assert_eq!(mission.mission_type, MISSION_TYPE_DEMON);
        assert!(!mission.is_empty());
        assert!(mission.opt1 > 0);
        assert!(mission.opt2 <= 118);
        assert!(mission.exp >= 1);
    }
    // Impossible/insane raise the target level by 1/2 (still capped at 118).
    let mut seed2 = 1u32;
    let impossible = generate_single_demon_mission(118, 0, 3, &mut seed2);
    assert_eq!(impossible.opt2, 118);
    let normal_at_low_level = generate_single_demon_mission(10, 0, 3, &mut seed2);
    assert_eq!(normal_at_low_level.opt2, 11);
}

#[test]
fn generate_single_demon_mission_point_values_match_difficulty_table() {
    let mut seed = 7u32;
    assert_eq!(generate_single_demon_mission(50, 0, 0, &mut seed).pts, 1);
    assert_eq!(generate_single_demon_mission(50, 0, 1, &mut seed).pts, 2);
    assert_eq!(generate_single_demon_mission(50, 0, 2, &mut seed).pts, 4);
    assert_eq!(generate_single_demon_mission(50, 0, 3, &mut seed).pts, 10);
    assert_eq!(generate_single_demon_mission(50, 0, 4, &mut seed).pts, 25);
}

// C `generate_single_ratling_mission` (`military.c:865-921`): only odd
// levels 9..=39 (post difficulty-adjustment) yield a real mission.
#[test]
fn generate_single_ratling_mission_rejects_out_of_range_levels() {
    let mut seed = 3u32;
    assert!(generate_single_ratling_mission(8, 0, 0, &mut seed).is_empty()); // below 9
    assert!(generate_single_ratling_mission(40, 0, 0, &mut seed).is_empty()); // above 39
    assert!(generate_single_ratling_mission(10, 0, 0, &mut seed).is_empty()); // even level
    let valid = generate_single_ratling_mission(9, 0, 0, &mut seed);
    assert!(!valid.is_empty());
    assert_eq!(valid.mission_type, MISSION_TYPE_RATLING);
    assert_eq!(valid.opt2, 9);
}

#[test]
fn generate_single_ratling_mission_difficulty_shifts_the_target_level() {
    let mut seed = 9u32;
    // difficulty 3 (impossible) adds (3-2)=1 to the level: 36 -> 37, an
    // odd level within [9,39], so this is a valid mission at the shifted
    // target level.
    let shifted = generate_single_ratling_mission(36, 0, 3, &mut seed);
    assert!(!shifted.is_empty());
    assert_eq!(shifted.opt2, 37);
    assert_eq!(shifted.pts, 10);
}

// C `generate_single_silver_mission` (`military.c:951-1007`): only level
// 12+ (post difficulty-adjustment) yields a real mission; opt1 scales with
// the unclamped cube-root military rank.
#[test]
fn generate_single_silver_mission_rejects_below_level_twelve() {
    let mut seed = 5u32;
    assert!(generate_single_silver_mission(11, 0, 0, &mut seed).is_empty());
    let valid = generate_single_silver_mission(12, 0, 0, &mut seed);
    assert!(!valid.is_empty());
    assert_eq!(valid.mission_type, MISSION_TYPE_SILVER);
    assert_eq!(valid.opt2, 0);
    assert_eq!(valid.pts, 1);
}

#[test]
fn generate_single_silver_mission_scales_opt1_with_military_rank() {
    let mut seed_low = 11u32;
    let mut seed_high = 11u32;
    // Rank 0 (military_pts=0): opt1 = 10 + 0 + RANDOM(31), so it's always
    // in [10, 40]. Rank 10 (military_pts=1000): opt1 = 10 + 80 +
    // RANDOM(81), so it's always in [90, 170] - strictly higher than any
    // rank-0 roll, regardless of the individual RNG draws.
    let low_rank = generate_single_silver_mission(12, 0, 0, &mut seed_low);
    let high_rank = generate_single_silver_mission(12, 1000, 0, &mut seed_high);
    assert!((10..=40).contains(&low_rank.opt1));
    assert!((90..=170).contains(&high_rank.opt1));
    assert!(high_rank.opt1 > low_rank.opt1);
}

// C `check_military_solve`'s pent-demon class guard (`death.c:310-316`).
#[test]
fn is_pent_demon_mission_class_matches_every_disjoint_c_range() {
    // Normal pent demon ranges.
    assert!(is_pent_demon_mission_class(52));
    assert!(is_pent_demon_mission_class(84));
    assert!(!is_pent_demon_mission_class(85)); // sewer ratling range starts here
    assert!(is_pent_demon_mission_class(107));
    assert!(is_pent_demon_mission_class(170));
    assert!(is_pent_demon_mission_class(388));
    assert!(is_pent_demon_mission_class(403));
    assert!(!is_pent_demon_mission_class(404)); // demon lord range, not a mission target
                                                // Elite/lesser demon palette-swap ranges.
    assert!(is_pent_demon_mission_class(ELITE_DEMON_CLASS_BASE));
    assert!(is_pent_demon_mission_class(ELITE_DEMON_CLASS_BASE + 47));
    assert!(!is_pent_demon_mission_class(ELITE_DEMON_CLASS_BASE + 48));
    assert!(is_pent_demon_mission_class(LESSER_DEMON_CLASS_BASE));
    assert!(is_pent_demon_mission_class(LESSER_DEMON_CLASS_BASE + 47));
    assert!(!is_pent_demon_mission_class(LESSER_DEMON_CLASS_BASE + 48));
    // Well outside any range.
    assert!(!is_pent_demon_mission_class(0));
    assert!(!is_pent_demon_mission_class(1000));
}

// C `check_military_solve`'s sewer-ratling class guard (`death.c:358`).
#[test]
fn is_sewer_ratling_mission_class_matches_the_c_range() {
    assert!(!is_sewer_ratling_mission_class(84));
    assert!(is_sewer_ratling_mission_class(85));
    assert!(is_sewer_ratling_mission_class(100));
    assert!(!is_sewer_ratling_mission_class(101));
}

// C `get_demon_mission_value` (`death.c:281-288`): elite demons count for
// 10, everything else (including lesser demons and normal pents) for 1.
#[test]
fn get_demon_mission_value_matches_c_elite_vs_everything_else() {
    assert_eq!(get_demon_mission_value(ELITE_DEMON_CLASS_BASE), 10);
    assert_eq!(get_demon_mission_value(ELITE_DEMON_CLASS_BASE + 47), 10);
    assert_eq!(get_demon_mission_value(ELITE_DEMON_CLASS_BASE + 48), 1);
    assert_eq!(get_demon_mission_value(LESSER_DEMON_CLASS_BASE), 1);
    assert_eq!(get_demon_mission_value(52), 1);
}

// C `generate_demon_mission(level, ppd)` (`military.c:847-861`): fills
// every one of the 5 offer slots with a demon mission at the matching
// difficulty.
#[test]
fn generate_demon_mission_fills_all_five_difficulty_slots() {
    let mut seed = 42u32;
    let missions = generate_demon_mission(20, 0, &mut seed);
    for mission in missions {
        assert_eq!(mission.mission_type, MISSION_TYPE_DEMON);
        assert!(!mission.is_empty());
    }
}

// C `generate_sewer_mission(level, ppd)` (`military.c:930-948`): a level
// far outside the 9..=39 odd-level window can never produce a valid
// ratling mission regardless of which difficulty `RANDOM(5)` picks, so
// every draw across many seeds must come back `None`.
#[test]
fn generate_sewer_mission_returns_none_when_level_never_qualifies() {
    for seed_value in 0..50u32 {
        let mut seed = seed_value;
        assert!(generate_sewer_mission(2, 0, &mut seed).is_none());
    }
}

// A level inside the valid odd-level window always yields Some(...) for
// at least difficulty 0..=2 (which don't shift the target level at all).
#[test]
fn generate_sewer_mission_returns_some_within_valid_level_window() {
    let mut seed = 7u32;
    let (difficulty, mission) = generate_sewer_mission(21, 0, &mut seed).expect("valid pick");
    assert!(difficulty < 5);
    assert_eq!(mission.mission_type, MISSION_TYPE_RATLING);
}

// C `generate_mine_mission(level, ppd)` (`military.c:1016-1034`): same
// random-slot shape, gated on level >= 12 (post difficulty-adjustment).
#[test]
fn generate_mine_mission_returns_none_below_level_twelve() {
    for seed_value in 0..50u32 {
        let mut seed = seed_value;
        assert!(generate_mine_mission(5, 0, &mut seed).is_none());
    }
}

#[test]
fn generate_mine_mission_returns_some_above_level_twelve() {
    let mut seed = 3u32;
    let (difficulty, mission) = generate_mine_mission(30, 0, &mut seed).expect("valid pick");
    assert!(difficulty < 5);
    assert_eq!(mission.mission_type, MISSION_TYPE_SILVER);
}

// C `generate_mission_with_preference(cn, ppd, preferred_type)`
// (`military.c:1036-1131`): with `preferred_type == 0` (no preference,
// C's `default:` branch) at a level too low for ratling/silver missions,
// only the always-available demon missions are guaranteed - the "small
// chance of other mission types" branch can only ever add a ratling or
// silver mission (both level-gated below the level-7 floor here), so
// every non-preferred-type slot must still be a demon mission.
#[test]
fn generate_mission_with_preference_no_preference_keeps_demon_missions_at_low_level() {
    let mut seed = 123u32;
    let missions = generate_mission_with_preference(5, 0, 0, -1, &mut seed);
    for mission in missions {
        assert_eq!(mission.mission_type, MISSION_TYPE_DEMON);
    }
}

// `preferred_type == 2` at a valid odd ratling level replaces slot 0 with
// a ratling mission (C's own `ppd->mis[0] = mission` when the level
// qualifies) in addition to whatever slots the 3 extra `generate_sewer_
// mission` random draws hit.
#[test]
fn generate_mission_with_preference_ratling_preference_overwrites_slot_zero() {
    let mut seed = 55u32;
    let missions = generate_mission_with_preference(21, 0, 2, -1, &mut seed);
    assert_eq!(missions[0].mission_type, MISSION_TYPE_RATLING);
}

// `preferred_type == 3` at level 12+ replaces slot 0 with a silver
// mission the same way.
#[test]
fn generate_mission_with_preference_silver_preference_overwrites_slot_zero() {
    let mut seed = 77u32;
    let missions = generate_mission_with_preference(12, 0, 3, -1, &mut seed);
    assert_eq!(missions[0].mission_type, MISSION_TYPE_SILVER);
}

// `mission_difficulty_preference` in `0..=4` overrides whatever slot the
// rest of the function already picked, replacing it with a mission of the
// preferred type/difficulty combo (here type 1 = demon at difficulty 3,
// C's `ppd->mis[diff] = mission` after the main preference switch).
#[test]
fn generate_mission_with_preference_difficulty_preference_overrides_slot() {
    let mut seed = 9u32;
    let missions = generate_mission_with_preference(20, 0, 1, 3, &mut seed);
    assert_eq!(missions[3].mission_type, MISSION_TYPE_DEMON);
    // Impossible-difficulty demon missions are worth 10 pts (see
    // `generate_single_demon_mission`'s own difficulty=3 branch).
    assert_eq!(missions[3].pts, 10);
}

// C `generate_mission(cn, ppd)` (`military.c:1137-1139`): identical to
// `generate_mission_with_preference(cn, ppd, 0)`.
#[test]
fn generate_mission_matches_no_preference_wrapper() {
    let mut seed_a = 4u32;
    let mut seed_b = 4u32;
    let via_wrapper = generate_mission(20, 0, -1, &mut seed_a);
    let via_direct = generate_mission_with_preference(20, 0, 0, -1, &mut seed_b);
    assert_eq!(via_wrapper, via_direct);
}

// C `check_military_solve`'s progress-message display gate
// (`death.c:339-341` / `:369-370`): only echo every 5th/10th kill once
// the remaining count reaches double digits, but always echo below 10.
#[test]
fn military_mission_progress_message_should_display_matches_c_threshold() {
    assert!(military_mission_progress_message_should_display(9));
    assert!(military_mission_progress_message_should_display(1));
    assert!(!military_mission_progress_message_should_display(11));
    assert!(military_mission_progress_message_should_display(15)); // <100 and %5==0
    assert!(!military_mission_progress_message_should_display(17));
    assert!(military_mission_progress_message_should_display(20)); // %10==0
    assert!(military_mission_progress_message_should_display(110)); // >=100, %10==0
    assert!(!military_mission_progress_message_should_display(115)); // >=100, not %10==0
}

fn demon_mission(pts: i32, exp: i32) -> SingleMission {
    SingleMission {
        mission_type: MISSION_TYPE_DEMON,
        opt1: 5,
        opt2: 10,
        pts,
        exp,
    }
}

// C `accept_mission` (`military.c:1300-1341`): `ppd->took_mission != 0`
// always wins first, regardless of every other gate.
#[test]
fn accept_mission_rejects_when_already_has_mission() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_took_mission(1);

    assert_eq!(
        player.accept_mission(0, 100),
        AcceptMissionOutcome::AlreadyHasMission
    );
}

// C: `ppd->solved_yday == yday + 1` -> "I don't have another mission for
// you today".
#[test]
fn accept_mission_rejects_when_already_completed_today() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_solved_yday(101);

    assert_eq!(
        player.accept_mission(0, 100),
        AcceptMissionOutcome::AlreadyCompletedToday
    );
}

// C: `ppd->mission_yday != yday + 1` -> "I haven't offered you that kind
// of mission today".
#[test]
fn accept_mission_rejects_when_missions_not_offered_today() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_mission_yday(50);

    assert_eq!(
        player.accept_mission(0, 100),
        AcceptMissionOutcome::MissionsNotOfferedToday
    );
}

// C: non-advisor mission whose `pts` cost exceeds `current_pts` ->
// "I have not offered you that kind of mission" (difficulty 0 is always
// free regardless of points, matching C's `difficulty > 0` guard).
#[test]
fn accept_mission_rejects_insufficient_points_above_difficulty_zero() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_mission_yday(101);
    player.set_military_current_pts(5);
    player.set_military_mission(1, demon_mission(10, 500));

    assert_eq!(
        player.accept_mission(1, 100),
        AcceptMissionOutcome::InsufficientPoints
    );
}

// C `display_mission`'s own guard: `mis[difficulty].type == 0` ->
// "that mission is not available".
#[test]
fn accept_mission_rejects_unavailable_empty_slot() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_mission_yday(101);

    assert_eq!(
        player.accept_mission(0, 100),
        AcceptMissionOutcome::MissionUnavailable
    );
}

// Successful acceptance at difficulty 0 never costs points (C's
// `difficulty > 0` guard on the deduction), but still stamps
// `took_mission`/`took_yday` and clears the mission preferences.
#[test]
fn accept_mission_accepts_difficulty_zero_without_spending_points() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_mission_yday(101);
    player.set_military_current_pts(0);
    let mission = demon_mission(0, 200);
    player.set_military_mission(0, mission);
    player.set_mission_type_preference(1);
    player.set_mission_difficulty_preference(2);

    let outcome = player.accept_mission(0, 100);

    assert_eq!(outcome, AcceptMissionOutcome::Accepted(mission));
    assert_eq!(player.military_took_mission(), 1);
    assert_eq!(player.military_took_yday(), 101);
    assert_eq!(player.military_current_pts(), 0);
    assert_eq!(player.mission_type_preference(), 0);
    assert_eq!(player.mission_difficulty_preference(), -1);
}

// Successful acceptance above difficulty 0 deducts the mission's `pts`
// cost from `current_pts` (C's `ppd->current_pts -= ppd->mis[difficulty].
// pts`).
#[test]
fn accept_mission_deducts_points_above_difficulty_zero() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_mission_yday(101);
    player.set_military_current_pts(50);
    player.set_military_mission(2, demon_mission(20, 400));

    let outcome = player.accept_mission(2, 100);

    assert_eq!(
        outcome,
        AcceptMissionOutcome::Accepted(demon_mission(20, 400))
    );
    assert_eq!(player.military_current_pts(), 30);
    assert_eq!(player.military_took_mission(), 3);
}

// An advisor-recommended mission (`mission_type_preference > 0` matching
// `mission_difficulty_preference`) skips the points check and the points
// deduction entirely - C's own comment: "player already paid gold".
#[test]
fn accept_mission_advisor_mission_skips_points_check_and_deduction() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_mission_yday(101);
    player.set_military_current_pts(0);
    player.set_military_mission(3, demon_mission(999, 400));
    player.set_mission_type_preference(1);
    player.set_mission_difficulty_preference(3);

    let outcome = player.accept_mission(3, 100);

    assert_eq!(
        outcome,
        AcceptMissionOutcome::Accepted(demon_mission(999, 400))
    );
    assert_eq!(player.military_current_pts(), 0);
    assert_eq!(player.military_took_mission(), 4);
}

// C `complete_mission`: `if (!ppd->solved_mission) return 0;` - untouched
// no-op.
#[test]
fn complete_mission_no_active_mission_is_a_no_op() {
    let mut world = World::default();
    let player_char = character(1);
    world.add_character(player_char);
    let mut player = PlayerRuntime::connected(1, 0);

    let result = world.complete_mission(CharacterId(1), &mut player, 0, CharacterId(999));

    assert_eq!(result, CompleteMissionResult::NoActiveMission);
    assert!(world.drain_pending_system_texts().is_empty());
}

// Non-mercenary completion: exp via `give_exp`, `pts + pts/2` added to
// `military_points`, no gold bonus, "Well done" feedback queued. `exp`
// stays below `level2exp(2)` (16) so `check_levelup` doesn't add its own
// "gained level" feedback text to the queue this test inspects, and any
// positive `military_pts_awarded` inherently crosses rank 0 (C's
// `cbrt(1) == 1`), so this asserts the promotion that formula implies
// rather than "no promotion".
#[test]
fn complete_mission_awards_exp_and_points_for_non_mercenary() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_took_mission(1); // difficulty 0
    player.set_military_took_yday(50);
    player.set_military_solved_mission(true);
    player.set_military_mission(0, demon_mission(10, 10));

    let result = world.complete_mission(CharacterId(1), &mut player, 0, CharacterId(999));

    let CompleteMissionResult::Completed(outcome) = result else {
        panic!("expected Completed, got {result:?}");
    };
    assert_eq!(outcome.difficulty, 0);
    assert_eq!(outcome.exp_awarded, 10);
    assert_eq!(outcome.military_pts_awarded, 15); // 10 + 10/2
    assert_eq!(outcome.gold_awarded, 0);
    assert_eq!(outcome.promoted_to, Some(2)); // cbrt(15) = 2.46 -> 2

    let character = &world.characters[&CharacterId(1)];
    assert_eq!(character.exp, 10);
    assert_eq!(character.military_normal_exp, 10);
    assert_eq!(character.military_points, 15);
    assert_eq!(character.gold, 0);

    assert!(!player.military_solved_mission());
    assert_eq!(player.military_took_mission(), 0);
    assert_eq!(player.military_took_yday(), 0);
    assert_eq!(player.military_solved_yday(), 50);

    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|t| t.message.contains("Well done")));
    assert!(texts
        .iter()
        .any(|t| t.message.contains("You've been promoted")));
}

// Mercenary completion: gold bonus (`exp / 5`), and the mercenary
// points-bonus formula (`pts + pts/2 + pts*prof*3/100 + 1`).
#[test]
fn complete_mission_awards_gold_bonus_for_mercenary() {
    let mut world = World::default();
    let mut merc = character(1);
    merc.professions[profession::MERCENARY] = 10;
    world.add_character(merc);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_took_mission(1);
    player.set_military_solved_mission(true);
    player.set_military_mission(0, demon_mission(100, 10));

    let result = world.complete_mission(CharacterId(1), &mut player, 0, CharacterId(999));

    let CompleteMissionResult::Completed(outcome) = result else {
        panic!("expected Completed, got {result:?}");
    };
    assert_eq!(outcome.gold_awarded, 2); // 10 / 5
                                         // 100 + 100/2 + 100*10*3/100 + 1 = 100+50+30+1 = 181
    assert_eq!(outcome.military_pts_awarded, 181);

    let character = &world.characters[&CharacterId(1)];
    assert_eq!(character.gold, 2);
    assert_eq!(character.military_points, 181);
    assert!(character.flags.contains(CharacterFlags::ITEMS));

    let texts = world.drain_pending_system_texts();
    let text_bytes = world.drain_pending_system_text_bytes();
    assert_eq!(text_bytes.len(), 1);
    assert!(texts.iter().any(|t| t.message.contains("Well done")));
}

// Crossing a rank threshold queues the promotion feedback text, same
// wording as `World::give_military_pts`; the server-wide "Grats:"
// broadcast only fires above rank 9 (C's `get_army_rank_int(co) > 9`
// guard).
#[test]
fn complete_mission_promotes_and_queues_feedback() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_took_mission(1);
    player.set_military_solved_mission(true);
    // pts + pts/2 = 15 -> military_points = 15 -> cbrt(15) = rank 2.
    player.set_military_mission(0, demon_mission(10, 0));

    let result = world.complete_mission(CharacterId(1), &mut player, 0, CharacterId(999));

    let CompleteMissionResult::Completed(outcome) = result else {
        panic!("expected Completed, got {result:?}");
    };
    assert_eq!(outcome.promoted_to, Some(2));

    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|t| t.message.contains("You've been promoted")));
    assert!(world.drain_pending_channel_broadcasts().is_empty());
}

// C `calculate_advisor_index` (`military.c:2239-2249`): two disjoint
// linear bands (below/above `storage_id` 27) both mapping onto
// `0..MAXADVISOR`, with out-of-range results falling back to slot 0.
#[test]
fn calculate_advisor_index_matches_c_bands() {
    // Below-27 band: idx = storage_id - 7.
    assert_eq!(calculate_advisor_index(7), 0);
    assert_eq!(calculate_advisor_index(26), 19);
    // storage_id 6 -> idx -1 -> out of range -> falls back to 0.
    assert_eq!(calculate_advisor_index(6), 0);
    // storage_id 0 -> idx -7 -> out of range -> falls back to 0.
    assert_eq!(calculate_advisor_index(0), 0);

    // At-or-above-27 band: idx = storage_id - 31 + 3 = storage_id - 28.
    assert_eq!(calculate_advisor_index(28), 0);
    assert_eq!(calculate_advisor_index(47), 19);
    // storage_id 27 -> idx -1 -> out of range -> falls back to 0.
    assert_eq!(calculate_advisor_index(27), 0);
    // storage_id 48 -> idx 20 -> out of range (>= MAXADVISOR) -> 0.
    assert_eq!(calculate_advisor_index(48), 0);
}

// C `advisor_price(level)` (`military.c:2288-2299`): 5 flat level bands.
#[test]
fn advisor_price_matches_c_level_bands() {
    assert_eq!(advisor_price(1), 400);
    assert_eq!(advisor_price(24), 400);
    assert_eq!(advisor_price(25), 800);
    assert_eq!(advisor_price(44), 800);
    assert_eq!(advisor_price(45), 1200);
    assert_eq!(advisor_price(64), 1200);
    assert_eq!(advisor_price(65), 1500);
    assert_eq!(advisor_price(84), 1500);
    assert_eq!(advisor_price(85), 2000);
    assert_eq!(advisor_price(200), 2000);
}

// C `offer_favor`'s 5 favor-size multipliers (`military.c:2318-2372`),
// applied on top of `advisor_price`.
#[test]
fn offer_favor_cost_applies_size_multiplier_over_advisor_price() {
    // level 1 -> advisor_price == 400.
    assert_eq!(offer_favor_cost(1, 0), Some(400)); // small: x1
    assert_eq!(offer_favor_cost(1, 1), Some(1200)); // medium: x3
    assert_eq!(offer_favor_cost(1, 2), Some(4000)); // big: x10
    assert_eq!(offer_favor_cost(1, 3), Some(8000)); // huge: x20
    assert_eq!(offer_favor_cost(1, 4), Some(14000)); // vast: x35
    assert_eq!(offer_favor_cost(1, 5), None); // invalid size -> C's `return 0`
}

// C `greet_player` (`military.c:1764-1798`): fresh player, never greeted.
#[test]
fn greet_player_new_player_sets_state_one() {
    let mut player = PlayerRuntime::connected(1, 0);
    let outcome = player.greet_player(false, 100);
    assert_eq!(outcome, GreetPlayerOutcome::NewPlayer);
    assert_eq!(player.master_state(), 1);
}

// C: `ppd->master_state != 0` (after the stale-10 reset, checked before
// any of the other branches) -> silent no-op.
#[test]
fn greet_player_already_greeted_is_silent_no_op() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_master_state(2);
    let outcome = player.greet_player(true, 100);
    assert_eq!(outcome, GreetPlayerOutcome::AlreadyGreeted);
    assert_eq!(player.master_state(), 2);
}

// C: a stale `master_state == 10` (interrupted reroll confirmation from a
// previous visit) is reset to 0 first, then falls through to the rest of
// the function afresh - NOT treated as "already greeted".
#[test]
fn greet_player_resets_stale_reroll_confirmation_state_and_falls_through() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_master_state(10);
    let outcome = player.greet_player(true, 100);
    assert_eq!(outcome, GreetPlayerOutcome::HasRank);
    assert_eq!(player.master_state(), 2);
}

#[test]
fn greet_player_has_active_mission() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_took_mission(1);
    let outcome = player.greet_player(true, 100);
    assert_eq!(outcome, GreetPlayerOutcome::HasActiveMission);
    assert_eq!(player.master_state(), 2);
}

#[test]
fn greet_player_already_completed_today() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_solved_yday(101);
    let outcome = player.greet_player(true, 100);
    assert_eq!(outcome, GreetPlayerOutcome::AlreadyCompletedToday);
    assert_eq!(player.master_state(), 2);
}

#[test]
fn greet_player_has_rank() {
    let mut player = PlayerRuntime::connected(1, 0);
    let outcome = player.greet_player(true, 100);
    assert_eq!(outcome, GreetPlayerOutcome::HasRank);
    assert_eq!(player.master_state(), 2);
}

// C: an advisor's specific-mission recommendation already rendered the
// greeting text this visit - takes priority over every other branch
// (checked right after the `master_state != 0` guard).
#[test]
fn greet_player_advisor_recommendation_already_shown_takes_priority() {
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_recommend(101);
    player.set_mission_type_preference(2);
    player.set_mission_difficulty_preference(3);
    // Would otherwise be `HasActiveMission`/`HasRank` - the advisor
    // branch must win.
    player.set_military_took_mission(1);
    let outcome = player.greet_player(true, 100);
    assert_eq!(
        outcome,
        GreetPlayerOutcome::AdvisorRecommendationAlreadyShown
    );
    assert_eq!(player.master_state(), 2);
}

// C `handle_mission_reroll` (`military.c:1889-1936`): already used today.
#[test]
fn mission_reroll_already_rerolled_today_is_a_no_op() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_reroll_yday(101);
    let mut rng = 42u32;

    let outcome = world.mission_reroll(CharacterId(1), &mut player, 100, &mut rng);

    assert_eq!(outcome, MissionRerollOutcome::AlreadyRerolledToday);
}

#[test]
fn mission_reroll_blocked_by_active_mission() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_took_mission(1);
    let mut rng = 42u32;

    let outcome = world.mission_reroll(CharacterId(1), &mut player, 100, &mut rng);

    assert_eq!(outcome, MissionRerollOutcome::HasActiveMission);
}

#[test]
fn mission_reroll_blocked_by_insufficient_gold() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.gold = 100;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    let mut rng = 42u32;

    let outcome = world.mission_reroll(CharacterId(1), &mut player, 100, &mut rng);

    assert_eq!(outcome, MissionRerollOutcome::InsufficientGold);
    assert_eq!(world.characters[&CharacterId(1)].gold, 100);
}

// First `reroll` says: sets `master_state = 10` and asks for confirmation
// without spending any gold yet.
#[test]
fn mission_reroll_first_call_requests_confirmation_without_spending_gold() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.gold = 20_000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    let mut rng = 42u32;

    let outcome = world.mission_reroll(CharacterId(1), &mut player, 100, &mut rng);

    assert_eq!(outcome, MissionRerollOutcome::ConfirmationRequested);
    assert_eq!(player.master_state(), 10);
    assert_eq!(world.characters[&CharacterId(1)].gold, 20_000);
}

// Second `reroll` (with `master_state` already `10`) confirms: spends the
// 200 gold, stamps `reroll_yday`/resets `mission_yday`, generates a fresh
// offer table, and returns to `master_state = 2`.
#[test]
fn mission_reroll_confirmed_spends_gold_and_generates_new_missions() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.gold = 20_000;
    character_data.level = 20;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_master_state(10);
    let mut rng = 42u32;

    let outcome = world.mission_reroll(CharacterId(1), &mut player, 100, &mut rng);

    assert_eq!(outcome, MissionRerollOutcome::Rerolled);
    assert_eq!(world.characters[&CharacterId(1)].gold, 0);
    assert!(world.characters[&CharacterId(1)]
        .flags
        .contains(CharacterFlags::ITEMS));
    assert_eq!(player.military_reroll_yday(), 101);
    assert_eq!(player.master_state(), 2);
    // A fresh 5-slot offer table was generated (matches `generate_
    // mission`'s baseline "no preference" shape: `generate_demon_mission`
    // fills every slot, then the default branch's unconditional
    // `generate_mine_mission` call may overwrite one slot with a silver
    // mission at this level - every slot is still non-empty either way).
    for idx in 0..5 {
        assert_ne!(player.military_mission(idx).mission_type, 0);
    }
}

// The rank-cubed `military_pts` floor-up (`generate_mission_with_
// preference`'s "Adjust military exp for rank if the player gained a
// rank elsewhere" comment) is applied here at the `mission_reroll` call
// site, matching that comment's intent exactly.
#[test]
fn mission_reroll_floors_military_pts_to_rank_cubed() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.gold = 20_000;
    character_data.level = 20;
    character_data.military_points = 1000; // rank 10 (cbrt(1000) = 10)
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_master_state(10);
    player.set_military_pts(5); // stale, far below rank^3 = 1000
    let mut rng = 42u32;

    world.mission_reroll(CharacterId(1), &mut player, 100, &mut rng);

    assert_eq!(player.military_pts(), 1000);
}

//-----------------------
// CDR_MILITARY_MASTER driver (military.c:2108-2206).

fn master_npc(id: u32) -> Character {
    let mut master = character(id);
    master.name = "Seymour".into();
    master.driver = CDR_MILITARY_MASTER;
    master
}

fn recruit(id: u32) -> Character {
    let mut recruit = character(id);
    recruit.flags |= CharacterFlags::PLAYER;
    recruit
}

// C `military_master_parse` (`military.c:1634-1644`): the only zone-file
// arg this driver reads is `storage=N;`.
#[test]
fn military_master_driver_args_parse_storage_field() {
    let data = parse_military_master_driver_args("storage=42;");
    assert_eq!(data.storage_id, 42);
}

#[test]
fn military_master_driver_args_default_when_absent() {
    let data = parse_military_master_driver_args("");
    assert_eq!(data.storage_id, 0);
}

// C `diff_name[difficulty]`/`get_colored_difficulty_name`'s clamp
// (`military.c:339,1350-1361`).
#[test]
fn mission_difficulty_name_matches_legacy_table_and_clamps_out_of_range() {
    assert_eq!(mission_difficulty_name(0), "easy");
    assert_eq!(mission_difficulty_name(1), "normal");
    assert_eq!(mission_difficulty_name(2), "hard");
    assert_eq!(mission_difficulty_name(3), "impossible");
    assert_eq!(mission_difficulty_name(4), "insane");
    assert_eq!(mission_difficulty_name(99), "easy");
}

// C `describe_mission` (`military.c:1194-1220`).
#[test]
fn describe_mission_text_renders_each_mission_type() {
    let demon = SingleMission {
        mission_type: MISSION_TYPE_DEMON,
        opt1: 3,
        opt2: 10,
        pts: 5,
        exp: 100,
    };
    assert_eq!(
        describe_mission_text(&demon, 0, "Godmode").unwrap(),
        "I have an easy mission for you, Godmode. It is to slay 3 level 10 demons in the \
         Pentagram Quest."
    );

    let ratling = SingleMission {
        mission_type: MISSION_TYPE_RATLING,
        opt1: 4,
        opt2: 12,
        pts: 5,
        exp: 100,
    };
    assert_eq!(
        describe_mission_text(&ratling, 2, "Godmode").unwrap(),
        "I have an hard mission for you, Godmode. It is to slay 4 level 12 ratlings in the \
         Sewers."
    );

    let silver = SingleMission {
        mission_type: MISSION_TYPE_SILVER,
        opt1: 50,
        opt2: 0,
        pts: 5,
        exp: 100,
    };
    assert_eq!(
        describe_mission_text(&silver, 4, "Godmode").unwrap(),
        "I have an insane mission for you, Godmode. It is to find 50 units of silver in the \
         Mine."
    );

    assert!(describe_mission_text(&SingleMission::default(), 0, "Godmode").is_none());
}

// C `display_mission` (`military.c:1261-1288`).
#[test]
fn display_mission_text_renders_each_mission_type() {
    let demon = SingleMission {
        mission_type: MISSION_TYPE_DEMON,
        opt1: 3,
        opt2: 10,
        ..Default::default()
    };
    assert_eq!(
        display_mission_text(&demon).unwrap(),
        "Your mission is to slay 3 level 10 demons in the Pentagram Quest."
    );

    let silver = SingleMission {
        mission_type: MISSION_TYPE_SILVER,
        opt1: 50,
        ..Default::default()
    };
    assert_eq!(
        display_mission_text(&silver).unwrap(),
        "Your mission is to find 50 units of silver in the Mine."
    );

    assert!(display_mission_text(&SingleMission::default()).is_none());
}

// C `offer_missions` (`military.c:1231-1246`): skips missions the player
// can't afford (`pts > 1 && pts > current_pts`), falling back to the "no
// suitable missions" line if none qualified.
#[test]
fn offer_missions_text_skips_unaffordable_missions() {
    let missions = [
        SingleMission {
            mission_type: MISSION_TYPE_DEMON,
            opt1: 1,
            opt2: 5,
            pts: 1,
            exp: 10,
        },
        SingleMission {
            mission_type: MISSION_TYPE_DEMON,
            opt1: 2,
            opt2: 6,
            pts: 500,
            exp: 20,
        },
        SingleMission::default(),
        SingleMission::default(),
        SingleMission::default(),
    ];

    let lines = offer_missions_text(&missions, 10, "Godmode");
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("slay 1 level 5 demons"));
}

#[test]
fn offer_missions_text_falls_back_when_nothing_affordable() {
    let missions: [SingleMission; 5] = std::array::from_fn(|_| SingleMission::default());
    let lines = offer_missions_text(&missions, 0, "Godmode");
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("I don't have any suitable missions"));
}

// C `handle_mission_request` (`military.c:1842-1896`): already has a
// mission.
#[test]
fn handle_mission_request_blocked_by_active_mission() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_took_mission(1);
    let mut rng = 42u32;

    let outcome =
        world.handle_mission_request(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    assert_eq!(outcome, MissionRequestOutcome::AlreadyHasMission);
}

#[test]
fn handle_mission_request_blocked_by_completed_today() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_solved_yday(101);
    let mut rng = 42u32;

    let outcome =
        world.handle_mission_request(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    assert_eq!(outcome, MissionRequestOutcome::AlreadyCompletedToday);
}

// C: `!get_army_rank_int(co)` -> not enrolled in the army yet.
#[test]
fn handle_mission_request_rejects_unenrolled_player() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    let mut rng = 42u32;

    let outcome =
        world.handle_mission_request(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    assert_eq!(outcome, MissionRequestOutcome::NotEnrolled);
}

#[test]
fn handle_mission_request_generates_and_offers_missions_for_enrolled_player() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 20;
    character_data.military_points = 1000; // rank 10, enrolled.
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    let mut rng = 42u32;

    let outcome =
        world.handle_mission_request(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    let MissionRequestOutcome::Offered(lines) = outcome else {
        panic!("expected Offered, got {outcome:?}");
    };
    // Reroll-footer line always appended last.
    assert!(lines.last().unwrap().contains("saying reroll for 200 gold"));
    assert_eq!(player.mission_yday(), 101);
    // A fresh 5-slot offer table was generated (matches `mission_reroll`'s
    // own equivalent assertion).
    for idx in 0..5 {
        assert_ne!(player.military_mission(idx).mission_type, 0);
    }
}

// C: re-requesting the same day's already-generated offer table doesn't
// regenerate it (`ppd->mission_yday == yday + 1` guard) - still renders
// the listing from the existing table.
#[test]
fn handle_mission_request_reuses_todays_offer_table() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 20;
    character_data.military_points = 1000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_mission_yday(101);
    player.set_military_mission(
        0,
        SingleMission {
            mission_type: MISSION_TYPE_DEMON,
            opt1: 7,
            opt2: 9,
            pts: 1,
            exp: 10,
        },
    );
    let mut rng = 42u32;

    let outcome =
        world.handle_mission_request(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    let MissionRequestOutcome::Offered(lines) = outcome else {
        panic!("expected Offered, got {outcome:?}");
    };
    assert!(lines
        .iter()
        .any(|line| line.contains("slay 7 level 9 demons")));
}

// C: a fresh advisor-recommended mission short-circuits the general
// offer listing.
#[test]
fn handle_mission_request_advisor_recommendation_short_circuits() {
    let mut world = World::default();
    let mut character_data = character(1);
    // Odd level so `generate_single_ratling_mission`'s `adjusted_level &
    // 1 == 0` rejection doesn't kick in for difficulty 1 (`adjusted_level
    // == level` below difficulty 3).
    character_data.level = 21;
    character_data.military_points = 1000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_mission_type_preference(MISSION_TYPE_RATLING);
    player.set_mission_difficulty_preference(1);
    let mut rng = 42u32;

    let outcome =
        world.handle_mission_request(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    let MissionRequestOutcome::AdvisorRecommendation {
        description,
        prompt,
    } = outcome
    else {
        panic!("expected AdvisorRecommendation, got {outcome:?}");
    };
    assert!(description.contains("ratlings in the Sewers"));
    assert!(prompt.contains("saying normal"));
}

// C `process_advisor_recommendation` (`military.c:1685-1755`): already
// processed today (`ppd->recommend == yday + 1`) is a total no-op.
#[test]
fn process_advisor_recommendation_skips_when_already_processed_today() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_recommend(101);
    let mut rng = 42u32;

    let outcome =
        world.process_advisor_recommendation(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    assert_eq!(outcome, AdvisorRecommendationOutcome::AlreadyProcessed);
    // Untouched - C's own guard returns before the trailing `ppd->recommend
    // = yday + 1` stamp too.
    assert_eq!(player.military_recommend(), 101);
}

// C: no specific-mission preference and no matching `advisor_last[n]` ->
// an empty `StandardRecommendations` list, but `recommend` is still
// stamped (C's own unconditional trailing assignment).
#[test]
fn process_advisor_recommendation_standard_branch_empty_when_nothing_matched() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    let mut rng = 42u32;

    let outcome =
        world.process_advisor_recommendation(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    assert_eq!(
        outcome,
        AdvisorRecommendationOutcome::StandardRecommendations(Vec::new())
    );
    assert_eq!(player.military_recommend(), 101);
}

// C: the standard branch reports every `advisor_last[n]` entry stamped
// today, by index.
#[test]
fn process_advisor_recommendation_standard_branch_reports_every_matching_advisor() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_advisor_last(0, 101);
    player.set_military_advisor_last(3, 101);
    player.set_military_advisor_last(5, 50); // Not today - excluded.
    let mut rng = 42u32;

    let outcome =
        world.process_advisor_recommendation(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    let AdvisorRecommendationOutcome::StandardRecommendations(lines) = outcome else {
        panic!("expected StandardRecommendations, got {outcome:?}");
    };
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("advisor 0"));
    assert!(lines[1].contains("advisor 3"));
}

// C: a specific-mission preference short-circuits into the paid-favor
// greeting, regenerating a fresh offer table for today
// (`mission_yday != yday + 1`), describing the preferred slot, and
// prompting "say <difficulty>" since nothing blocks acceptance.
#[test]
fn process_advisor_recommendation_specific_mission_regenerates_and_prompts_accept() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 21;
    character_data.military_points = 1000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_mission_type_preference(MISSION_TYPE_RATLING);
    player.set_mission_difficulty_preference(1);
    let mut rng = 42u32;

    let outcome =
        world.process_advisor_recommendation(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    let AdvisorRecommendationOutcome::SpecificMission {
        greeting,
        description,
        followup,
    } = outcome
    else {
        panic!("expected SpecificMission, got {outcome:?}");
    };
    assert!(greeting.contains("oddly specific request for normal ratling-hunting"));
    assert!(description.unwrap().contains("ratlings in the Sewers"));
    assert!(followup.contains("Say normal to accept this mission"));
    assert_eq!(player.mission_yday(), 101);
    assert_eq!(player.military_recommend(), 101);
}

// C: reuses today's already-generated offer table instead of
// regenerating (`mission_yday == yday + 1` guard) - still describes
// whatever is already sitting in the preferred slot.
#[test]
fn process_advisor_recommendation_specific_mission_reuses_todays_offer_table() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 21;
    character_data.military_points = 1000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_mission_type_preference(MISSION_TYPE_RATLING);
    player.set_mission_difficulty_preference(1);
    player.set_mission_yday(101);
    player.set_military_mission(
        1,
        SingleMission {
            mission_type: MISSION_TYPE_RATLING,
            opt1: 7,
            opt2: 9,
            pts: 1,
            exp: 10,
        },
    );
    let mut rng = 42u32;

    let outcome =
        world.process_advisor_recommendation(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    let AdvisorRecommendationOutcome::SpecificMission { description, .. } = outcome else {
        panic!("expected SpecificMission, got {outcome:?}");
    };
    assert!(description.unwrap().contains("slay 7 level 9 ratlings"));
}

// C: the already-completed-today follow-up line wins over the accept
// prompt.
#[test]
fn process_advisor_recommendation_specific_mission_already_completed_today_followup() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 21;
    character_data.military_points = 1000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_mission_type_preference(MISSION_TYPE_RATLING);
    player.set_mission_difficulty_preference(1);
    player.set_military_solved_yday(101);
    let mut rng = 42u32;

    let outcome =
        world.process_advisor_recommendation(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    let AdvisorRecommendationOutcome::SpecificMission { followup, .. } = outcome else {
        panic!("expected SpecificMission, got {outcome:?}");
    };
    assert!(followup.contains("you've already completed a mission today"));
}

// C: the active-mission-conflict follow-up line wins over the accept
// prompt when the player already took a (different) mission.
#[test]
fn process_advisor_recommendation_specific_mission_active_mission_conflict_followup() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 21;
    character_data.military_points = 1000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_mission_type_preference(MISSION_TYPE_RATLING);
    player.set_mission_difficulty_preference(1);
    player.set_military_took_mission(3);
    let mut rng = 42u32;

    let outcome =
        world.process_advisor_recommendation(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    let AdvisorRecommendationOutcome::SpecificMission { followup, .. } = outcome else {
        panic!("expected SpecificMission, got {outcome:?}");
    };
    assert!(followup.contains("you already have an active mission"));
}

// C: the difficulty-name ternary used in this function's own text falls
// through to "insane" for any preference other than 0-3 (unlike
// `mission_difficulty_name`'s out-of-range clamp to "easy") - exercised
// here via preference `4` ("insane" itself, the highest real difficulty)
// to also confirm the description embeds the demon-mission text (no
// type preference set, so C's `describe_mission` falls back on whatever
// was last generated - here nothing, so `None`).
#[test]
fn process_advisor_recommendation_difficulty_text_falls_through_to_insane() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 21;
    character_data.military_points = 1000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_mission_type_preference(MISSION_TYPE_RATLING);
    player.set_mission_difficulty_preference(4);
    let mut rng = 42u32;

    let outcome =
        world.process_advisor_recommendation(CharacterId(1), &mut player, 100, &mut rng, "Godmode");

    let AdvisorRecommendationOutcome::SpecificMission {
        greeting, followup, ..
    } = outcome
    else {
        panic!("expected SpecificMission, got {outcome:?}");
    };
    assert!(greeting.contains("oddly specific request for insane ratling-hunting"));
    assert!(followup.contains("Say insane to accept this mission"));
}

// C `military_master_driver`'s `NT_CHAR` branch (`military.c:2153-2177`),
// ported as a periodic nearby-player scan.
#[test]
fn military_master_greet_scan_queues_nearby_visible_player() {
    let mut world = World::default();
    world.map.tile_mut(12, 10).unwrap().light = 255;
    assert!(world.spawn_character(master_npc(1), 10, 10));
    let mut visitor = recruit(2);
    visitor.name = "Godmode".into();
    assert!(world.spawn_character(visitor, 12, 10));

    world.process_military_master_actions(0, 0);

    let events = world.drain_pending_military_master_events();
    assert!(events.contains(&MilitaryMasterEvent::NearbyPlayer {
        master_id: CharacterId(1),
        player_id: CharacterId(2),
    }));
}

#[test]
fn military_master_greet_scan_skips_out_of_range_player() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc(1), 10, 10));
    let visitor = recruit(2);
    assert!(world.spawn_character(visitor, 30, 30));

    world.process_military_master_actions(0, 0);

    assert!(world.drain_pending_military_master_events().is_empty());
}

#[test]
fn military_master_replies_to_small_talk_keyword_directly() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc(1), 10, 10));
    let mut visitor = recruit(2);
    visitor.name = "Godmode".into();
    assert!(world.spawn_character(visitor, 10, 10));

    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.push_driver_text_message(CharacterId(2), "hello");
    }
    world.process_military_master_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("Hello, Godmode!")));
    // The visitor is also in range of the periodic `NT_CHAR` greet scan
    // (same tile as the master) - only that event, no message-driven one,
    // should have been queued for a plain "hello".
    let events = world.drain_pending_military_master_events();
    assert!(events
        .iter()
        .all(|event| matches!(event, MilitaryMasterEvent::NearbyPlayer { .. })));
}

#[test]
fn military_master_whats_your_name_replies_with_own_name() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc(1), 10, 10));
    let visitor = recruit(2);
    assert!(world.spawn_character(visitor, 10, 10));

    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.push_driver_text_message(CharacterId(2), "what's your name");
    }
    world.process_military_master_actions(0, 0);

    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I'm Seymour.")));
}

#[test]
fn military_master_mission_keyword_queues_mission_request_event() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc(1), 10, 10));
    let visitor = recruit(2);
    assert!(world.spawn_character(visitor, 10, 10));

    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.push_driver_text_message(CharacterId(2), "mission");
    }
    world.process_military_master_actions(0, 0);

    let events = world.drain_pending_military_master_events();
    assert!(events.contains(&MilitaryMasterEvent::MissionRequest {
        master_id: CharacterId(1),
        player_id: CharacterId(2),
    }));
}

#[test]
fn military_master_difficulty_keywords_queue_accept_mission_events_with_correct_difficulty() {
    let cases = [
        ("easy", 0usize),
        ("normal", 1),
        ("hard", 2),
        ("impossible", 3),
        ("insane", 4),
    ];
    for (keyword, expected_difficulty) in cases {
        let mut world = World::default();
        assert!(world.spawn_character(master_npc(1), 10, 10));
        let visitor = recruit(2);
        assert!(world.spawn_character(visitor, 10, 10));

        if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
            master.push_driver_text_message(CharacterId(2), keyword);
        }
        world.process_military_master_actions(0, 0);

        let events = world.drain_pending_military_master_events();
        assert!(
            events.contains(&MilitaryMasterEvent::AcceptMission {
                master_id: CharacterId(1),
                player_id: CharacterId(2),
                difficulty: expected_difficulty,
            }),
            "keyword {keyword:?} expected difficulty {expected_difficulty}, got {events:?}"
        );
    }
}

#[test]
fn military_master_repeat_failed_hear_and_reroll_keywords_queue_matching_events() {
    let cases = [
        (
            "repeat",
            MilitaryMasterEvent::Repeat {
                master_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
        (
            "failed",
            MilitaryMasterEvent::Failed {
                master_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
        (
            "hear",
            MilitaryMasterEvent::Hear {
                master_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
        (
            "reroll",
            MilitaryMasterEvent::Reroll {
                master_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
        (
            "decline",
            MilitaryMasterEvent::Reroll {
                master_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
        (
            "new missions",
            MilitaryMasterEvent::Reroll {
                master_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
    ];
    for (keyword, expected_event) in cases {
        let mut world = World::default();
        assert!(world.spawn_character(master_npc(1), 10, 10));
        let visitor = recruit(2);
        assert!(world.spawn_character(visitor, 10, 10));

        if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
            master.push_driver_text_message(CharacterId(2), keyword);
        }
        world.process_military_master_actions(0, 0);

        let events = world.drain_pending_military_master_events();
        assert!(
            events.contains(&expected_event),
            "keyword {keyword:?} expected {expected_event:?}, got {events:?}"
        );
    }
}

// Advisor-only codes (favor/small/medium/big/huge/vast/pay) and
// advisor-recommendation combo codes (e.g. "easy demon") are matched by
// the shared qa table but not handled by the Master driver - matches C's
// own `default: return 0`. The admin-only codes (info/reset/raise/
// promote) are also matched here but require `CF_GOD` on the speaker
// (`military.c:2037-2089`'s shared guard) - a non-admin speaker gets the
// same silent no-op, exercised below with `recruit` (no `GOD` flag).
#[test]
fn military_master_ignores_advisor_and_non_admin_codes() {
    for keyword in [
        "favor",
        "small",
        "pay",
        "info",
        "reset",
        "raise",
        "promote",
        "easy demon",
    ] {
        let mut world = World::default();
        assert!(world.spawn_character(master_npc(1), 10, 10));
        let visitor = recruit(2);
        assert!(world.spawn_character(visitor, 10, 10));

        if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
            master.push_driver_text_message(CharacterId(2), keyword);
        }
        world.process_military_master_actions(0, 0);

        // The visitor is also in range of the periodic `NT_CHAR` greet
        // scan (same tile as the master) - only that event, never a
        // message-driven one, should have been queued for these
        // Master-ignored/non-admin codes.
        let events = world.drain_pending_military_master_events();
        assert!(
            events
                .iter()
                .all(|event| matches!(event, MilitaryMasterEvent::NearbyPlayer { .. })),
            "keyword {keyword:?} should not queue a message-driven event, got {events:?}"
        );
    }
}

// C `military.c:2037-2089`'s shared `if (!(ch[co].flags & CF_GOD)) break;`
// guard: a `CF_GOD`-flagged speaker's "info"/"reset"/"raise"/"promote"
// keywords each queue their matching admin-only event.
#[test]
fn military_master_admin_codes_queue_matching_events_for_god_speaker() {
    let cases = [
        (
            "info",
            MilitaryMasterEvent::Info {
                master_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
        (
            "reset",
            MilitaryMasterEvent::Reset {
                player_id: CharacterId(2),
            },
        ),
        (
            "raise",
            MilitaryMasterEvent::Raise {
                player_id: CharacterId(2),
            },
        ),
        (
            "promote",
            MilitaryMasterEvent::Promote {
                master_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
    ];
    for (keyword, expected_event) in cases {
        let mut world = World::default();
        assert!(world.spawn_character(master_npc(1), 10, 10));
        let mut admin = recruit(2);
        admin.flags |= CharacterFlags::GOD;
        assert!(world.spawn_character(admin, 10, 10));

        if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
            master.push_driver_text_message(CharacterId(2), keyword);
        }
        world.process_military_master_actions(0, 0);

        let events = world.drain_pending_military_master_events();
        assert!(
            events.contains(&expected_event),
            "keyword {keyword:?} expected {expected_event:?}, got {events:?}"
        );
    }
}

#[test]
fn military_master_ignores_text_from_speaker_out_of_range() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc(1), 10, 10));
    let visitor = recruit(2);
    assert!(world.spawn_character(visitor, 30, 30));

    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.push_driver_text_message(CharacterId(2), "mission");
    }
    world.process_military_master_actions(0, 0);

    assert!(world.drain_pending_military_master_events().is_empty());
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn military_master_given_item_is_destroyed_and_replies_junk() {
    let mut world = World::default();
    let mut master = master_npc(1);
    master.cursor_item = Some(ItemId(900));
    assert!(world.spawn_character(master, 10, 10));
    world.items.insert(ItemId(900), item(900, ItemFlags::TAKE));

    if let Some(master) = world.characters.get_mut(&CharacterId(1)) {
        master.push_driver_message(NT_GIVE, 2, 0, 0);
    }
    world.process_military_master_actions(0, 0);

    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
    assert!(!world.items.contains_key(&ItemId(900)));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("That's junk.")));
}

//-----------------------
// CDR_MILITARY_ADVISOR driver (military.c:2607-2699).

fn advisor_npc(id: u32, storage_id: i32) -> Character {
    let mut advisor = character(id);
    advisor.name = "Advisor".into();
    advisor.driver = CDR_MILITARY_ADVISOR;
    advisor.driver_state = Some(CharacterDriverState::MilitaryAdvisor(
        MilitaryAdvisorDriverData { storage_id },
    ));
    advisor
}

// C `military_advisor_parse` (`military.c:2221-2230`): the only
// zone-file arg this driver reads is `storage=N;`.
#[test]
fn military_advisor_driver_args_parse_storage_field() {
    let data = parse_military_advisor_driver_args("storage=42;");
    assert_eq!(data.storage_id, 42);
}

#[test]
fn military_advisor_driver_args_default_when_absent() {
    let data = parse_military_advisor_driver_args("");
    assert_eq!(data.storage_id, 0);
}

#[test]
fn advisor_storage_id_reads_driver_state() {
    let mut world = World::default();
    assert!(world.spawn_character(advisor_npc(1, 33), 10, 10));
    assert_eq!(world.advisor_storage_id(CharacterId(1)), 33);
}

#[test]
fn advisor_storage_id_defaults_to_zero_for_mismatched_driver_state() {
    let mut world = World::default();
    assert!(world.spawn_character(character(1), 10, 10));
    assert_eq!(world.advisor_storage_id(CharacterId(1)), 0);
}

// C `military.c:339`'s favor-size name table (`offer_favor`'s switch).
#[test]
fn favor_size_name_matches_c_table() {
    assert_eq!(favor_size_name(0), "small");
    assert_eq!(favor_size_name(1), "medium");
    assert_eq!(favor_size_name(2), "big");
    assert_eq!(favor_size_name(3), "huge");
    assert_eq!(favor_size_name(4), "vast");
    // Out-of-range falls back to "vast" (C's own trailing `: "vast"`
    // ternary chain default).
    assert_eq!(favor_size_name(999), "vast");
}

#[test]
fn mission_type_name_matches_c_table() {
    assert_eq!(mission_type_name(1), "demon-slaying");
    assert_eq!(mission_type_name(2), "ratling-hunting");
    assert_eq!(mission_type_name(3), "silver-mining");
    assert_eq!(mission_type_name(0), "unknown");
    assert_eq!(mission_type_name(999), "unknown");
}

// C `adv_introduction` (`military.c:2262-2281`): 4 rotating greetings
// keyed by `storage_ID % 4`.
#[test]
fn adv_introduction_text_rotates_by_storage_id_modulo_four() {
    assert!(adv_introduction_text(0, "Bob").contains("I could do you a favor, Bob"));
    assert!(adv_introduction_text(1, "Bob").contains("Say, Bob, would you like to speed up"));
    assert!(
        adv_introduction_text(2, "Bob").contains("Not getting promoted as fast as you want, Bob?")
    );
    assert!(adv_introduction_text(3, "Bob").contains("Need a favor, Bob?"));
    // Wraps around: storage_ID 4 behaves like 0, 7 like 3.
    assert_eq!(
        adv_introduction_text(4, "Bob"),
        adv_introduction_text(0, "Bob")
    );
    assert_eq!(
        adv_introduction_text(7, "Bob"),
        adv_introduction_text(3, "Bob")
    );
}

#[test]
fn adv_favor_desc_lines_matches_c_text() {
    let lines = adv_favor_desc_lines();
    assert_eq!(
        lines[0],
        "My favors come in five sizes, small, medium, big, huge and vast."
    );
    assert!(lines[1].contains("easy demon"));
    assert!(lines[1].contains("insane mining"));
}

// C `offer_favor` (`military.c:2339-2382`).
#[test]
fn offer_favor_already_used_today_is_a_no_op() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_advisor_last(5, 101);

    let outcome = world.offer_favor(CharacterId(1), &mut player, 5, 0, 100);

    assert_eq!(outcome, OfferFavorOutcome::AlreadyUsedToday);
}

#[test]
fn offer_favor_invalid_favor_size_is_a_no_op() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);

    let outcome = world.offer_favor(CharacterId(1), &mut player, 5, 99, 100);

    assert_eq!(outcome, OfferFavorOutcome::InvalidFavorSize);
}

#[test]
fn offer_favor_stamps_cost_state_and_storage_nr_matching_price_table() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 30;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);

    let outcome = world.offer_favor(CharacterId(1), &mut player, 5, 2, 100);

    // level 30 -> advisor_price = 800; favor_size 2 ("big") -> x10.
    assert_eq!(
        outcome,
        OfferFavorOutcome::Offered {
            favor_size: 2,
            cost: 8000
        }
    );
    assert_eq!(player.advisor_cost(), 8000);
    assert_eq!(player.advisor_state(), 2);
    assert_eq!(player.advisor_storage_nr(), 2);
    // `offer_favor` itself never stamps `advisor_last` (only
    // `process_favor_payment` does, on actual payment).
    assert_eq!(player.military_advisor_last(5), 0);
}

// C `handle_specific_mission_request` (`military.c:481-566`).
#[test]
fn handle_specific_mission_request_already_used_today_is_a_no_op() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_advisor_last(2, 101);

    let outcome = world.handle_specific_mission_request(CharacterId(1), &mut player, 2, 0, 1, 100);

    assert_eq!(outcome, SpecificMissionRequestOutcome::AlreadyUsedToday);
}

#[test]
fn handle_specific_mission_request_rejects_invalid_mission_type_and_difficulty() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(
        world.handle_specific_mission_request(CharacterId(1), &mut player, 0, 0, 0, 100),
        SpecificMissionRequestOutcome::InvalidMissionType
    );
    assert_eq!(
        world.handle_specific_mission_request(CharacterId(1), &mut player, 0, 0, 4, 100),
        SpecificMissionRequestOutcome::InvalidMissionType
    );
    assert_eq!(
        world.handle_specific_mission_request(CharacterId(1), &mut player, 0, -1, 1, 100),
        SpecificMissionRequestOutcome::InvalidDifficulty
    );
    assert_eq!(
        world.handle_specific_mission_request(CharacterId(1), &mut player, 0, 5, 1, 100),
        SpecificMissionRequestOutcome::InvalidDifficulty
    );
}

#[test]
fn handle_specific_mission_request_ratling_needs_odd_level_between_nine_and_thirty_nine() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 10; // even -> rejected
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(
        world.handle_specific_mission_request(CharacterId(1), &mut player, 0, 0, 2, 100),
        SpecificMissionRequestOutcome::RatlingLevelGate
    );
}

#[test]
fn handle_specific_mission_request_silver_needs_level_twelve_or_above() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 11;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(
        world.handle_specific_mission_request(CharacterId(1), &mut player, 0, 0, 3, 100),
        SpecificMissionRequestOutcome::SilverLevelGate
    );
}

#[test]
fn handle_specific_mission_request_offers_and_stamps_temp_preferences() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 20;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);

    let outcome = world.handle_specific_mission_request(CharacterId(1), &mut player, 0, 2, 1, 100);

    match outcome {
        SpecificMissionRequestOutcome::Offered {
            difficulty,
            mission_type,
            cost,
            already_completed_today,
            has_active_mission,
        } => {
            assert_eq!(difficulty, 2);
            assert_eq!(mission_type, 1);
            assert_eq!(cost, specific_mission_price(20, 2, 1));
            assert!(!already_completed_today);
            assert!(!has_active_mission);
        }
        other => panic!("expected Offered, got {other:?}"),
    }
    assert_eq!(player.advisor_state(), 2);
    assert_eq!(player.advisor_storage_nr(), 2);
    assert_eq!(player.temp_mission_type(), 1);
    assert_eq!(player.temp_mission_difficulty(), 2);
}

#[test]
fn handle_specific_mission_request_surfaces_already_completed_and_active_mission_warnings() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.level = 20;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_solved_yday(101);
    player.set_military_took_mission(1);

    let outcome = world.handle_specific_mission_request(CharacterId(1), &mut player, 0, 0, 1, 100);

    match outcome {
        SpecificMissionRequestOutcome::Offered {
            already_completed_today,
            has_active_mission,
            ..
        } => {
            assert!(already_completed_today);
            assert!(has_active_mission);
        }
        other => panic!("expected Offered, got {other:?}"),
    }
}

// C `process_favor_payment` (`military.c:2402-2474`).
#[test]
fn process_favor_payment_nothing_agreed_when_state_or_advisor_mismatches() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    // advisor_state defaults to 0, not 2.
    player.set_current_advisor(5);

    let outcome = world.process_favor_payment(CharacterId(1), &mut player, 0, 5, 100);

    assert_eq!(outcome, ProcessFavorPaymentOutcome::NothingAgreed);
    assert_eq!(player.advisor_state(), 1);
}

#[test]
fn process_favor_payment_insufficient_gold() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.gold = 50;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_current_advisor(5);
    player.set_advisor_state(2);
    player.set_advisor_cost(100);

    let outcome = world.process_favor_payment(CharacterId(1), &mut player, 0, 5, 100);

    assert_eq!(outcome, ProcessFavorPaymentOutcome::InsufficientGold);
    assert_eq!(world.characters[&CharacterId(1)].gold, 50);
}

#[test]
fn process_favor_payment_arranges_plain_favor_and_grants_points() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.gold = 10_000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_current_advisor(5);
    player.set_advisor_state(2);
    player.set_advisor_cost(1200);
    player.set_advisor_storage_nr(2); // "big" favor

    let outcome = world.process_favor_payment(CharacterId(1), &mut player, 7, 5, 100);

    assert_eq!(
        outcome,
        ProcessFavorPaymentOutcome::FavorArranged { favor_size: 2 }
    );
    assert_eq!(world.characters[&CharacterId(1)].gold, 8_800);
    assert_eq!(player.military_current_pts(), 2 + 2 * 2);
    assert_eq!(player.advisor_state(), 1);
    assert_eq!(player.military_advisor_last(7), 101);
    // C `add_cost(ppd->advisor_cost, dat->storage_data + ppd->
    // advisor_storage_nr)` (`military.c:2421`): storage_id 5, slot 2
    // ("big" favor) records the 1200 payment.
    assert_eq!(world.military_advisor_storage.earned(5, 2), 1200);
    assert_eq!(world.military_advisor_storage.sold(5, 2), 1);
    // Other slots/storage ids stay untouched.
    assert_eq!(world.military_advisor_storage.sold(5, 0), 0);
    assert_eq!(world.military_advisor_storage.sold(6, 2), 0);
}

#[test]
fn process_favor_payment_records_cost_across_multiple_sales() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.gold = 10_000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_current_advisor(9);
    player.set_advisor_state(2);
    player.set_advisor_cost(300);
    player.set_advisor_storage_nr(0); // "small" favor

    let _ = world.process_favor_payment(CharacterId(1), &mut player, 0, 9, 100);

    // A second sale of a different favor size on the same NPC.
    player.set_advisor_state(2);
    player.set_advisor_cost(700);
    player.set_advisor_storage_nr(0);
    let _ = world.process_favor_payment(CharacterId(1), &mut player, 0, 9, 100);

    assert_eq!(world.military_advisor_storage.earned(9, 0), 1000);
    assert_eq!(world.military_advisor_storage.sold(9, 0), 2);
}

#[test]
fn process_favor_payment_arranges_specific_mission_and_stamps_preferences() {
    let mut world = World::default();
    let mut character_data = character(1);
    character_data.gold = 10_000;
    world.add_character(character_data);
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_current_advisor(5);
    player.set_advisor_state(2);
    player.set_advisor_cost(500);
    player.set_temp_mission_type(2);
    player.set_temp_mission_difficulty(3);

    let outcome = world.process_favor_payment(CharacterId(1), &mut player, 7, 5, 100);

    assert_eq!(
        outcome,
        ProcessFavorPaymentOutcome::SpecificMissionArranged {
            mission_type: 2,
            difficulty: 3
        }
    );
    assert_eq!(player.mission_type_preference(), 2);
    assert_eq!(player.mission_difficulty_preference(), 3);
    assert_eq!(player.temp_mission_type(), 0);
    assert_eq!(player.temp_mission_difficulty(), -1);
    assert_eq!(player.military_advisor_last(7), 101);
    // Not a plain favor, so no `current_pts` were granted.
    assert_eq!(player.military_current_pts(), 0);
}

// Driver-level event generation (`military_advisor_driver`,
// `military.c:2607-2699`).
#[test]
fn military_advisor_greet_scan_queues_nearby_visible_player() {
    let mut world = World::default();
    assert!(world.spawn_character(advisor_npc(1, 10), 10, 10));
    assert!(world.spawn_character(recruit(2), 11, 10));

    world.process_military_advisor_actions(0);

    let events = world.drain_pending_military_advisor_events();
    assert!(events.contains(&MilitaryAdvisorEvent::NearbyPlayer {
        advisor_id: CharacterId(1),
        player_id: CharacterId(2),
    }));
}

#[test]
fn military_advisor_ignores_text_from_speaker_out_of_range() {
    let mut world = World::default();
    assert!(world.spawn_character(advisor_npc(1, 10), 10, 10));
    let visitor = recruit(2);
    assert!(world.spawn_character(visitor, 30, 30));

    if let Some(advisor) = world.characters.get_mut(&CharacterId(1)) {
        advisor.push_driver_text_message(CharacterId(2), "favor");
    }
    world.process_military_advisor_actions(0);

    assert!(world.drain_pending_military_advisor_events().is_empty());
}

#[test]
fn military_advisor_repeat_and_favor_keywords_queue_matching_events() {
    let mut world = World::default();
    assert!(world.spawn_character(advisor_npc(1, 10), 10, 10));
    assert!(world.spawn_character(recruit(2), 10, 10));

    for (keyword, expected) in [
        (
            "repeat",
            MilitaryAdvisorEvent::Repeat {
                advisor_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
        (
            "favor",
            MilitaryAdvisorEvent::FavorDesc {
                advisor_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
        (
            "small",
            MilitaryAdvisorEvent::Favor {
                advisor_id: CharacterId(1),
                player_id: CharacterId(2),
                favor_size: 0,
            },
        ),
        (
            "vast",
            MilitaryAdvisorEvent::Favor {
                advisor_id: CharacterId(1),
                player_id: CharacterId(2),
                favor_size: 4,
            },
        ),
        (
            "pay",
            MilitaryAdvisorEvent::Pay {
                advisor_id: CharacterId(1),
                player_id: CharacterId(2),
            },
        ),
    ] {
        if let Some(advisor) = world.characters.get_mut(&CharacterId(1)) {
            advisor.push_driver_text_message(CharacterId(2), keyword);
        }
        world.process_military_advisor_actions(0);
        let events = world.drain_pending_military_advisor_events();
        assert!(
            events.contains(&expected),
            "keyword {keyword:?} should queue {expected:?}, got {events:?}"
        );
    }
}

#[test]
fn military_advisor_specific_mission_keywords_queue_matching_events() {
    let mut world = World::default();
    assert!(world.spawn_character(advisor_npc(1, 10), 10, 10));
    assert!(world.spawn_character(recruit(2), 10, 10));

    for (keyword, difficulty, mission_type) in [
        ("easy demon", 0, 1),
        ("insane demon", 4, 1),
        ("easy ratling", 0, 2),
        ("insane ratling", 4, 2),
        ("easy silver", 0, 3),
        ("insane silver", 4, 3),
    ] {
        if let Some(advisor) = world.characters.get_mut(&CharacterId(1)) {
            advisor.push_driver_text_message(CharacterId(2), keyword);
        }
        world.process_military_advisor_actions(0);
        let events = world.drain_pending_military_advisor_events();
        assert!(
            events.contains(&MilitaryAdvisorEvent::SpecificMissionRequest {
                advisor_id: CharacterId(1),
                player_id: CharacterId(2),
                difficulty,
                mission_type,
            }),
            "keyword {keyword:?} should queue difficulty {difficulty}/type {mission_type}, got \
             {events:?}"
        );
    }
}

#[test]
fn military_advisor_master_only_and_admin_keywords_are_silently_ignored() {
    let mut world = World::default();
    assert!(world.spawn_character(advisor_npc(1, 10), 10, 10));
    assert!(world.spawn_character(recruit(2), 10, 10));

    for keyword in ["mission", "easy", "reroll", "info", "reset"] {
        if let Some(advisor) = world.characters.get_mut(&CharacterId(1)) {
            advisor.push_driver_text_message(CharacterId(2), keyword);
        }
        world.process_military_advisor_actions(0);
        let events = world.drain_pending_military_advisor_events();
        assert!(
            events
                .iter()
                .all(|event| matches!(event, MilitaryAdvisorEvent::NearbyPlayer { .. })),
            "keyword {keyword:?} should not queue a message-driven event, got {events:?}"
        );
    }
}

// C `military.c:2523-2525`'s `if (!(ch[co].flags & CF_GOD)) { break; }`
// guard on the admin-only "info" code: a `CF_GOD`-flagged speaker queues
// the matching event (unlike the non-admin `recruit` speaker exercised by
// `military_advisor_master_only_and_admin_keywords_are_silently_ignored`
// above).
#[test]
fn military_advisor_info_keyword_queues_event_for_god_speaker() {
    let mut world = World::default();
    assert!(world.spawn_character(advisor_npc(1, 10), 10, 10));
    let mut admin = recruit(2);
    admin.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(admin, 10, 10));

    if let Some(advisor) = world.characters.get_mut(&CharacterId(1)) {
        advisor.push_driver_text_message(CharacterId(2), "info");
    }
    world.process_military_advisor_actions(0);

    let events = world.drain_pending_military_advisor_events();
    assert!(events.contains(&MilitaryAdvisorEvent::Info {
        advisor_id: CharacterId(1),
        player_id: CharacterId(2),
    }));
}

#[test]
fn military_advisor_given_item_is_destroyed_and_replies_junk() {
    let mut world = World::default();
    let mut advisor = advisor_npc(1, 10);
    advisor.cursor_item = Some(ItemId(900));
    assert!(world.spawn_character(advisor, 10, 10));
    world.items.insert(ItemId(900), item(900, ItemFlags::TAKE));

    if let Some(advisor) = world.characters.get_mut(&CharacterId(1)) {
        advisor.push_driver_message(NT_GIVE, 2, 0, 0);
    }
    world.process_military_advisor_actions(0);

    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .cursor_item
        .is_none());
    assert!(!world.items.contains_key(&ItemId(900)));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("That's junk.")));
}

#[test]
fn military_advisor_movement_rests_facing_dx_right() {
    let mut world = World::default();
    assert!(world.spawn_character(advisor_npc(1, 10), 10, 10));

    world.process_military_advisor_actions(0);

    assert_eq!(world.characters[&CharacterId(1)].dir, 4);
}

//-----------------------
// Military Master NPC-scoped storage blob: `process_clan_recommendation`/
// `update_clan_points` (`military.c:1654-1674,1815-1832`).

fn master_npc_with_storage(id: u32, storage_id: i32) -> Character {
    let mut master = master_npc(id);
    master.driver_state = Some(CharacterDriverState::MilitaryMaster(
        MilitaryMasterDriverData {
            storage_id,
            ..Default::default()
        },
    ));
    master
}

// C `update_clan_points`'s own `dat->last_clan_update = realtime` on
// `NT_CREATE` (`military.c:2126`) has no Rust zone-parse-time equivalent,
// so a `0` timestamp lazily stamps to `now` on the first call without
// granting any bonus yet.
#[test]
fn update_clan_points_lazily_stamps_first_tick_without_granting_bonus() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));
    let cnr = world.clan_registry.found_clan("Iron Wolves", 0).unwrap();
    world
        .clan_registry
        .set_bonus_level(cnr, CLAN_BONUS_MILITARY_ADVISOR, 50)
        .unwrap();

    world.update_clan_points(CharacterId(1), 1_000);

    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 0);
    let Some(CharacterDriverState::MilitaryMaster(data)) =
        world.characters[&CharacterId(1)].driver_state.clone()
    else {
        panic!("expected MilitaryMaster driver state");
    };
    assert_eq!(data.last_clan_update, 1_000);
}

// C `update_clan_points`: `realtime - dat->last_clan_update <= 60` throttle
// - no change until more than 60 seconds have passed since the last real
// update.
#[test]
fn update_clan_points_throttles_updates_to_every_sixty_seconds() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));
    let cnr = world.clan_registry.found_clan("Iron Wolves", 0).unwrap();
    world
        .clan_registry
        .set_bonus_level(cnr, CLAN_BONUS_MILITARY_ADVISOR, 50)
        .unwrap();

    world.update_clan_points(CharacterId(1), 1_000); // lazy-init stamp only
    world.update_clan_points(CharacterId(1), 1_030); // only 30s later: no-op
    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 0);

    world.update_clan_points(CharacterId(1), 1_061); // 61s later: applies
    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 50 * 20);
    let Some(CharacterDriverState::MilitaryMaster(data)) =
        world.characters[&CharacterId(1)].driver_state.clone()
    else {
        panic!("expected MilitaryMaster driver state");
    };
    // C: `dat->last_clan_update += 60;` (not stamped to `now`).
    assert_eq!(data.last_clan_update, 1_060);

    // A second call still within the same 60s window is a no-op.
    world.update_clan_points(CharacterId(1), 1_090);
    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 50 * 20);
}

// C: `bonus = get_clan_bonus(n, 1) * 20; if (bonus > 0) ...` - a clan with
// no Military Advisor bonus level gets nothing.
#[test]
fn update_clan_points_skips_clans_with_no_military_advisor_bonus() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));
    let cnr = world.clan_registry.found_clan("Iron Wolves", 0).unwrap();

    world.update_clan_points(CharacterId(1), 1_000);
    world.update_clan_points(CharacterId(1), 1_061);

    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 0);
}

// C: every founded clan is updated independently in the same tick.
#[test]
fn update_clan_points_updates_every_clan_independently() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));
    let alpha = world.clan_registry.found_clan("Alpha", 0).unwrap();
    let beta = world.clan_registry.found_clan("Beta", 0).unwrap();
    world
        .clan_registry
        .set_bonus_level(alpha, CLAN_BONUS_MILITARY_ADVISOR, 10)
        .unwrap();
    world
        .clan_registry
        .set_bonus_level(beta, CLAN_BONUS_MILITARY_ADVISOR, 30)
        .unwrap();

    world.update_clan_points(CharacterId(1), 1_000);
    world.update_clan_points(CharacterId(1), 1_061);

    assert_eq!(world.military_master_storage.clan_pts(7, alpha), 10 * 20);
    assert_eq!(world.military_master_storage.clan_pts(7, beta), 30 * 20);
}

// Two Military Master NPCs (distinct `storage_id`s) accrue independent
// clan-point pools, matching each NPC's own `struct military_master_data`.
#[test]
fn update_clan_points_keeps_separate_npcs_storage_independent() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));
    assert!(world.spawn_character(master_npc_with_storage(2, 9), 12, 10));
    let cnr = world.clan_registry.found_clan("Iron Wolves", 0).unwrap();
    world
        .clan_registry
        .set_bonus_level(cnr, CLAN_BONUS_MILITARY_ADVISOR, 50)
        .unwrap();

    world.update_clan_points(CharacterId(1), 1_000);
    world.update_clan_points(CharacterId(1), 1_061);
    // NPC 2 never ticked past its own lazy-init stamp.
    world.update_clan_points(CharacterId(2), 2_000);

    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 50 * 20);
    assert_eq!(world.military_master_storage.clan_pts(9, cnr), 0);
}

fn clan_member(id: u32, world: &mut World, cnr: u16) -> Character {
    let mut player = recruit(id);
    world.clan_registry.add_member(&mut player, cnr).unwrap();
    player
}

// C `process_clan_recommendation` (`military.c:1654-1674`): grants
// `ppd->current_pts += 5` and deducts 12000 from the clan's banked
// points once the clan has banked more than 12000.
#[test]
fn process_clan_recommendation_grants_points_and_deducts_clan_pool_above_threshold() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));
    let cnr = world.clan_registry.found_clan("Iron Wolves", 0).unwrap();
    world
        .clan_registry
        .set_bonus_level(cnr, CLAN_BONUS_MILITARY_ADVISOR, 700)
        .unwrap(); // 700 * 20 = 14000 > 12000 in a single tick
    world.update_clan_points(CharacterId(1), 1_000);
    world.update_clan_points(CharacterId(1), 1_061);
    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 14_000);

    let mut player_char = clan_member(2, &mut world, cnr);
    player_char.name = "Godmode".into();
    assert!(world.spawn_character(player_char, 10, 10));
    let mut player = PlayerRuntime::connected(2, 0);

    let greeting =
        world.process_clan_recommendation(CharacterId(1), CharacterId(2), &mut player, "Godmode");

    assert_eq!(
        greeting.as_deref(),
        Some("Be greeted, Godmode. You've been recommended by your clan!")
    );
    assert_eq!(player.military_current_pts(), 5);
    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 2_000);
}

// C: `dat->storage_data.clan_pts[clan_nr] > 12000` - exactly at (or
// below) the threshold is not enough.
#[test]
fn process_clan_recommendation_is_a_no_op_at_or_below_threshold() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));
    let cnr = world.clan_registry.found_clan("Iron Wolves", 0).unwrap();
    world
        .clan_registry
        .set_bonus_level(cnr, CLAN_BONUS_MILITARY_ADVISOR, 600)
        .unwrap(); // 600 * 20 = 12000, exactly at the threshold
    world.update_clan_points(CharacterId(1), 1_000);
    world.update_clan_points(CharacterId(1), 1_061);
    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 12_000);

    let player_char = clan_member(2, &mut world, cnr);
    assert!(world.spawn_character(player_char, 10, 10));
    let mut player = PlayerRuntime::connected(2, 0);

    let greeting =
        world.process_clan_recommendation(CharacterId(1), CharacterId(2), &mut player, "Godmode");

    assert_eq!(greeting, None);
    assert_eq!(player.military_current_pts(), 0);
    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 12_000);
}

// C: `!(clan_nr = get_char_clan(co))` - a non-clan-member player is a
// silent no-op regardless of the clan pool.
#[test]
fn process_clan_recommendation_is_a_no_op_for_non_clan_members() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));
    let cnr = world.clan_registry.found_clan("Iron Wolves", 0).unwrap();
    world
        .clan_registry
        .set_bonus_level(cnr, CLAN_BONUS_MILITARY_ADVISOR, 700)
        .unwrap();
    world.update_clan_points(CharacterId(1), 1_000);
    world.update_clan_points(CharacterId(1), 1_061);

    let player_char = recruit(2); // no clan membership
    assert!(world.spawn_character(player_char, 10, 10));
    let mut player = PlayerRuntime::connected(2, 0);

    let greeting =
        world.process_clan_recommendation(CharacterId(1), CharacterId(2), &mut player, "Godmode");

    assert_eq!(greeting, None);
    assert_eq!(player.military_current_pts(), 0);
    assert_eq!(world.military_master_storage.clan_pts(7, cnr), 14_000);
}

// C: `dat->last_recom != ch[co].ID` - the same player is only ever
// recommended once per NPC lifetime, even if the clan pool refills above
// threshold again.
#[test]
fn process_clan_recommendation_does_not_repeat_for_the_same_player() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));
    let cnr = world.clan_registry.found_clan("Iron Wolves", 0).unwrap();
    world
        .clan_registry
        .set_bonus_level(cnr, CLAN_BONUS_MILITARY_ADVISOR, 700)
        .unwrap();
    world.update_clan_points(CharacterId(1), 1_000);
    world.update_clan_points(CharacterId(1), 1_061);

    let player_char = clan_member(2, &mut world, cnr);
    assert!(world.spawn_character(player_char, 10, 10));
    let mut player = PlayerRuntime::connected(2, 0);

    let first =
        world.process_clan_recommendation(CharacterId(1), CharacterId(2), &mut player, "Godmode");
    assert!(first.is_some());
    assert_eq!(player.military_current_pts(), 5);

    // Refill the pool above threshold again, then greet the same player.
    world.update_clan_points(CharacterId(1), 1_121);
    let second =
        world.process_clan_recommendation(CharacterId(1), CharacterId(2), &mut player, "Godmode");

    assert_eq!(second, None);
    assert_eq!(player.military_current_pts(), 5); // unchanged
}

// A different player at the same NPC can still be recommended after
// another player already was.
#[test]
fn process_clan_recommendation_allows_a_different_player_after_another_was_recommended() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));
    let cnr = world.clan_registry.found_clan("Iron Wolves", 0).unwrap();
    world
        .clan_registry
        .set_bonus_level(cnr, CLAN_BONUS_MILITARY_ADVISOR, 1300)
        .unwrap(); // 1300 * 20 = 26000: enough for two 12000 deductions
    world.update_clan_points(CharacterId(1), 1_000);
    world.update_clan_points(CharacterId(1), 1_061);

    let first_char = clan_member(2, &mut world, cnr);
    assert!(world.spawn_character(first_char, 10, 10));
    let mut first_player = PlayerRuntime::connected(2, 0);
    let first_outcome = world.process_clan_recommendation(
        CharacterId(1),
        CharacterId(2),
        &mut first_player,
        "Alice",
    );
    assert!(first_outcome.is_some());

    let second_char = clan_member(3, &mut world, cnr);
    assert!(world.spawn_character(second_char, 11, 10));
    let mut second_player = PlayerRuntime::connected(3, 0);
    let second_outcome = world.process_clan_recommendation(
        CharacterId(1),
        CharacterId(3),
        &mut second_player,
        "Bob",
    );

    assert!(second_outcome.is_some());
    assert_eq!(second_player.military_current_pts(), 5);
}

//-----------------------
// Military Master NPC-scoped quest statistics: `World::record_mission_
// offered` (`accept_mission`'s `quests_given[difficulty]++`,
// `military.c:1348`) and `World::complete_mission`'s `quests_solved`/
// `pts_given`/`exp_given[difficulty]` bumps (`military.c:1382,1407,1411`).

// C: `dat->storage_data.quests_given[difficulty]++;` - called once per
// successful mission acceptance, keyed by the accepting NPC's own
// `storage_id`.
#[test]
fn record_mission_offered_increments_quests_given_for_its_difficulty() {
    let mut world = World::default();
    assert!(world.spawn_character(master_npc_with_storage(1, 7), 10, 10));

    world.record_mission_offered(CharacterId(1), 2);
    world.record_mission_offered(CharacterId(1), 2);
    world.record_mission_offered(CharacterId(1), 0);

    assert_eq!(
        world.military_master_storage.quest_stats(7, 2),
        (2, 0, 0, 0)
    );
    assert_eq!(
        world.military_master_storage.quest_stats(7, 0),
        (1, 0, 0, 0)
    );
}

// A `master_id` with no live `CDR_MILITARY_MASTER` driver state is a
// silent no-op (mirrors every other storage-scoped `World` method's own
// guard in this module).
#[test]
fn record_mission_offered_is_a_no_op_for_a_non_master_character() {
    let mut world = World::default();
    world.add_character(character(1));

    world.record_mission_offered(CharacterId(1), 0);

    assert_eq!(
        world.military_master_storage.quest_stats(0, 0),
        (0, 0, 0, 0)
    );
}

// C `complete_mission`: `quests_solved[difficulty]++`, `pts_given[
// difficulty] += mis[difficulty].pts` (the mission's raw point *cost*,
// not the larger formula-adjusted `military_pts_awarded`), `exp_given[
// difficulty] += mis[difficulty].exp`.
#[test]
fn complete_mission_records_quest_stats_on_its_master_npc() {
    let mut world = World::default();
    world.add_character(character(1));
    assert!(world.spawn_character(master_npc_with_storage(2, 9), 10, 10));
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_took_mission(3); // difficulty 2
    player.set_military_solved_mission(true);
    player.set_military_mission(2, demon_mission(20, 40));

    let result = world.complete_mission(CharacterId(1), &mut player, 0, CharacterId(2));

    let CompleteMissionResult::Completed(outcome) = result else {
        panic!("expected Completed, got {result:?}");
    };
    assert_eq!(outcome.difficulty, 2);

    // (quests_given, quests_solved, exp_given, pts_given)
    assert_eq!(
        world.military_master_storage.quest_stats(9, 2),
        (0, 1, 40, 20)
    );
}

// A second completion at a different difficulty accumulates
// independently, and the counters are keyed per-`storage_id` (a
// different Master NPC's own blob stays untouched).
#[test]
fn complete_mission_accumulates_stats_across_difficulties_and_keeps_npcs_independent() {
    let mut world = World::default();
    world.add_character(character(1));
    assert!(world.spawn_character(master_npc_with_storage(2, 9), 10, 10));
    assert!(world.spawn_character(master_npc_with_storage(3, 40), 11, 10));

    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_took_mission(1); // difficulty 0
    player.set_military_solved_mission(true);
    player.set_military_mission(0, demon_mission(10, 10));
    let _ = world.complete_mission(CharacterId(1), &mut player, 0, CharacterId(2));

    player.set_military_took_mission(2); // difficulty 1
    player.set_military_solved_mission(true);
    player.set_military_mission(1, demon_mission(5, 8));
    let _ = world.complete_mission(CharacterId(1), &mut player, 0, CharacterId(2));

    assert_eq!(
        world.military_master_storage.quest_stats(9, 0),
        (0, 1, 10, 10)
    );
    assert_eq!(
        world.military_master_storage.quest_stats(9, 1),
        (0, 1, 8, 5)
    );
    // The other Master NPC's storage_id (40) was never touched.
    assert_eq!(
        world.military_master_storage.quest_stats(40, 0),
        (0, 0, 0, 0)
    );
}

// A `master_id` with no live `CDR_MILITARY_MASTER` driver state is a
// silent no-op for the stats bump - `complete_mission`'s own character/
// exp/points mutation still applies normally.
#[test]
fn complete_mission_stats_are_a_no_op_for_a_non_master_character() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_military_took_mission(1);
    player.set_military_solved_mission(true);
    player.set_military_mission(0, demon_mission(10, 10));

    let result = world.complete_mission(CharacterId(1), &mut player, 0, CharacterId(999));

    let CompleteMissionResult::Completed(outcome) = result else {
        panic!("expected Completed, got {result:?}");
    };
    assert_eq!(outcome.exp_awarded, 10);
    assert_eq!(
        world.military_master_storage.quest_stats(0, 0),
        (0, 0, 0, 0)
    );
}
