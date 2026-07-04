use super::*;

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
