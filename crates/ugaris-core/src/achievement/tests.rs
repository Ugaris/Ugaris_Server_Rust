use super::*;

const NOW: i64 = 1_700_000_000;

#[test]
fn defs_table_has_one_entry_per_achievement_type_in_index_order() {
    assert_eq!(ACHIEVEMENT_DEFS.len(), ACHIEVEMENT_TYPE_COUNT);
    assert_eq!(AchievementType::ALL.len(), ACHIEVEMENT_TYPE_COUNT);
    for (idx, def) in ACHIEVEMENT_DEFS.iter().enumerate() {
        assert_eq!(def.ty as usize, idx, "def at index {idx} has mismatched ty");
        assert_eq!(AchievementType::ALL[idx] as usize, idx);
        assert!(!def.steam_id.is_empty());
        assert!(!def.name.is_empty());
        assert!(!def.description.is_empty());
        assert!(!def.hidden, "no C table entry is hidden today");
    }
}

#[test]
fn defs_table_spot_checks_match_c_source_digit_for_digit() {
    let d = achievement_def(AchievementType::Demonbane);
    assert_eq!(d.steam_id, "DEMONBANE");
    assert_eq!(d.name, "Demonbane");
    assert_eq!(d.description, "Defeat 2,500 demons");
    assert_eq!(d.category, AchCategory::Combat);
    assert_eq!(d.target, 2500);

    let d = achievement_def(AchievementType::DemonicExterminator);
    assert_eq!(d.target, 250_000);

    let d = achievement_def(AchievementType::SilverLegend);
    assert_eq!(d.target, 50_000_000);

    let d = achievement_def(AchievementType::DedicatedPlayer);
    assert_eq!(d.target, 1440);
    assert_eq!(d.description, "Play for 24 hours total");

    let d = achievement_def(AchievementType::MasterHerbalistProf);
    assert_eq!(d.steam_id, "MASTER_HERBALIST_PROF");
    assert_eq!(d.name, "Master Herbalist (Prof)");

    let d = achievement_def(AchievementType::FiveInARow);
    assert_eq!(d.steam_id, "FIVE_IN_A_ROW");
    assert_eq!(d.name, "5 in a Row");

    let d = achievement_def(AchievementType::Devoted);
    assert_eq!(d.target, 100);
    assert_eq!(d.category, AchCategory::Special);
}

#[test]
fn area_to_pent_index_matches_c_switch() {
    assert_eq!(area_to_pent_index(4), Some(PentArea::Earth));
    assert_eq!(area_to_pent_index(7), Some(PentArea::Fire));
    assert_eq!(area_to_pent_index(9), Some(PentArea::Ice));
    assert_eq!(area_to_pent_index(34), Some(PentArea::Hell));
    assert_eq!(area_to_pent_index(1), None);
    assert_eq!(area_to_pent_index(0), None);
}

#[test]
fn award_unlocks_once_and_sets_progress_and_target() {
    let mut data = AccountAchievements::default();
    assert!(!data.is_unlocked(AchievementType::FirstBlood));
    let newly = data.award(AchievementType::FirstBlood, "Hero", NOW);
    assert!(newly);
    assert!(data.is_unlocked(AchievementType::FirstBlood));
    assert_eq!(data.get_progress(AchievementType::FirstBlood), 1); // target 0 -> progress 1
    assert_eq!(
        data.achievements[AchievementType::FirstBlood as usize].achieved_by,
        "Hero"
    );
    assert_eq!(
        data.achievements[AchievementType::FirstBlood as usize].timestamp,
        NOW
    );

    // Second award is a no-op (already unlocked).
    let newly_again = data.award(AchievementType::FirstBlood, "Someone Else", NOW + 1);
    assert!(!newly_again);
    assert_eq!(
        data.achievements[AchievementType::FirstBlood as usize].achieved_by,
        "Hero"
    );
    assert_eq!(
        data.achievements[AchievementType::FirstBlood as usize].timestamp,
        NOW
    );
}

#[test]
fn award_target_based_sets_progress_to_target() {
    let mut data = AccountAchievements::default();
    data.award(AchievementType::FiendFighter, "Hero", NOW);
    assert_eq!(data.get_progress(AchievementType::FiendFighter), 100);
}

#[test]
fn add_progress_unlocks_only_when_target_reached() {
    let mut data = AccountAchievements::default();
    let def = achievement_def(AchievementType::FiendFighter);
    assert_eq!(def.target, 100);

    assert!(!data.add_progress(AchievementType::FiendFighter, 50, "Hero", NOW));
    assert!(!data.is_unlocked(AchievementType::FiendFighter));
    assert_eq!(data.get_progress(AchievementType::FiendFighter), 50);

    assert!(data.add_progress(AchievementType::FiendFighter, 50, "Hero", NOW));
    assert!(data.is_unlocked(AchievementType::FiendFighter));

    // Further progress calls are no-ops once unlocked.
    assert!(!data.add_progress(AchievementType::FiendFighter, 1000, "Hero", NOW));
    assert_eq!(data.get_progress(AchievementType::FiendFighter), 100);
}

#[test]
fn add_progress_on_instant_achievement_is_a_no_op() {
    // FirstBlood has target 0 (instant); add_progress never awards it,
    // matching C's `if (def->target == 0) return;` early-out.
    let mut data = AccountAchievements::default();
    assert!(!data.add_progress(AchievementType::FirstBlood, 1, "Hero", NOW));
    assert!(!data.is_unlocked(AchievementType::FirstBlood));
}

#[test]
fn get_stat_progress_covers_every_stat_category() {
    let mut stats = AchievementStats::default();
    stats.flowers_picked = 5;
    stats.mushrooms_picked = 6;
    stats.berries_picked = 7;
    stats.potions_brewed = 8;
    stats.demons_defeated = 9;
    stats.pents_solved = 10;
    stats.chests_opened = 11;
    stats.earth_stones = 12;
    stats.fire_stones = 13;
    stats.ice_stones = 14;
    stats.military_missions = 15;
    stats.tunnel_levels = 16;
    stats.silver_mined = 17;
    stats.gold_mined = 18;
    stats.gold_earned = 19;
    stats.play_time_minutes = 20;
    stats.login_streak = 21;

    assert_eq!(get_stat_progress(&stats, AchievementType::GreenThumb), 5);
    assert_eq!(
        get_stat_progress(&stats, AchievementType::MushroomHunter),
        6
    );
    assert_eq!(get_stat_progress(&stats, AchievementType::BerryPicker), 7);
    assert_eq!(get_stat_progress(&stats, AchievementType::Alchemist), 8);
    assert_eq!(get_stat_progress(&stats, AchievementType::FiendFighter), 9);
    assert_eq!(get_stat_progress(&stats, AchievementType::FullOfSolves), 10);
    assert_eq!(get_stat_progress(&stats, AchievementType::Looter), 11);
    assert_eq!(get_stat_progress(&stats, AchievementType::EarthRocks), 12);
    assert_eq!(get_stat_progress(&stats, AchievementType::FireRocks), 13);
    assert_eq!(get_stat_progress(&stats, AchievementType::IceRocks), 14);
    assert_eq!(get_stat_progress(&stats, AchievementType::Recruit), 15);
    assert_eq!(
        get_stat_progress(&stats, AchievementType::TunnelExplorer),
        16
    );
    assert_eq!(get_stat_progress(&stats, AchievementType::SilverNovice), 17);
    assert_eq!(get_stat_progress(&stats, AchievementType::GoldNovice), 18);
    assert_eq!(
        get_stat_progress(&stats, AchievementType::CoinCollector),
        19
    );
    assert_eq!(
        get_stat_progress(&stats, AchievementType::DedicatedPlayer),
        20
    );
    assert_eq!(get_stat_progress(&stats, AchievementType::Regular), 21);

    // Demon lords are tracked separately, not via AchievementStats.
    assert_eq!(
        get_stat_progress(&stats, AchievementType::SlayerOfDemonLords),
        0
    );
    // Instant achievements have no stat-driven progress.
    assert_eq!(get_stat_progress(&stats, AchievementType::FirstBlood), 0);
}

#[test]
fn get_stat_progress_caps_u64_counters_at_u32_max() {
    let mut stats = AchievementStats::default();
    stats.demons_defeated = u64::MAX;
    stats.silver_mined = u64::MAX;
    stats.gold_mined = u64::MAX;
    stats.gold_earned = u64::MAX;
    assert_eq!(
        get_stat_progress(&stats, AchievementType::Demonbane),
        u32::MAX
    );
    assert_eq!(
        get_stat_progress(&stats, AchievementType::SilverLegend),
        u32::MAX
    );
    assert_eq!(
        get_stat_progress(&stats, AchievementType::GoldLegend),
        u32::MAX
    );
    assert_eq!(
        get_stat_progress(&stats, AchievementType::Millionaire),
        u32::MAX
    );
}

#[test]
fn add_flowers_full_ladder() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();

    let unlocked = add_flowers(&mut data, &mut stats, 9, "Hero", NOW);
    assert!(unlocked.is_empty());

    let unlocked = add_flowers(&mut data, &mut stats, 1, "Hero", NOW); // 10 total
    assert_eq!(unlocked, vec![AchievementType::GreenThumb]);

    let unlocked = add_flowers(&mut data, &mut stats, 40, "Hero", NOW); // 50
    assert_eq!(unlocked, vec![AchievementType::BotanyEnthusiast]);

    let unlocked = add_flowers(&mut data, &mut stats, 150, "Hero", NOW); // 200
    assert_eq!(unlocked, vec![AchievementType::NaturesFriend]);

    let unlocked = add_flowers(&mut data, &mut stats, 300, "Hero", NOW); // 500
    assert_eq!(unlocked, vec![AchievementType::Herbalist]);

    let unlocked = add_flowers(&mut data, &mut stats, 500, "Hero", NOW); // 1000
    assert_eq!(unlocked, vec![AchievementType::MasterHerbalist]);

    assert_eq!(stats.flowers_picked, 1000);
    for ty in [
        AchievementType::GreenThumb,
        AchievementType::BotanyEnthusiast,
        AchievementType::NaturesFriend,
        AchievementType::Herbalist,
        AchievementType::MasterHerbalist,
    ] {
        assert!(data.is_unlocked(ty));
    }
}

#[test]
fn add_mushrooms_reaches_top_tier() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    add_mushrooms(&mut data, &mut stats, 1000, "Hero", NOW);
    assert_eq!(stats.mushrooms_picked, 1000);
    for ty in [
        AchievementType::MushroomHunter,
        AchievementType::FungusFinder,
        AchievementType::SporeSeeker,
        AchievementType::MushroomMaster,
        AchievementType::Mycologist,
    ] {
        assert!(data.is_unlocked(ty));
    }
}

#[test]
fn add_berries_reaches_top_tier() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    add_berries(&mut data, &mut stats, 1000, "Hero", NOW);
    assert!(data.is_unlocked(AchievementType::MasterGatherer));
}

#[test]
fn add_potions_full_ladder_including_legendary_brewer() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    let unlocked = add_potions(&mut data, &mut stats, 1000, "Hero", NOW);
    assert_eq!(
        unlocked,
        vec![
            AchievementType::Alchemist,
            AchievementType::JourneymanBrewer,
            AchievementType::ArcaneAlchemist,
            AchievementType::GrandmasterBrewer,
            AchievementType::PotionMaster,
            AchievementType::LegendaryBrewer,
        ]
    );
}

#[test]
fn add_demons_tracks_per_area_and_thresholds() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();

    add_demons(&mut data, &mut stats, 4, 50, "Hero", NOW); // Earth pents area
    assert_eq!(stats.demons_per_area[PentArea::Earth as usize], 50);
    assert_eq!(stats.demons_defeated, 50);
    assert!(!data.is_unlocked(AchievementType::FiendFighter));

    // Non-pent area: no per-area bucket, but total still counts.
    let unlocked = add_demons(&mut data, &mut stats, 1, 50, "Hero", NOW);
    assert_eq!(stats.demons_defeated, 100);
    assert_eq!(unlocked, vec![AchievementType::FiendFighter]);
    assert_eq!(stats.demons_per_area[PentArea::Earth as usize], 50);

    add_demons(&mut data, &mut stats, 1, 2400, "Hero", NOW);
    assert!(data.is_unlocked(AchievementType::Demonbane));
    add_demons(&mut data, &mut stats, 1, 12500, "Hero", NOW);
    assert!(data.is_unlocked(AchievementType::DreadDestroyer));
    add_demons(&mut data, &mut stats, 1, 235000, "Hero", NOW);
    assert!(data.is_unlocked(AchievementType::DemonicExterminator));
    assert_eq!(stats.demons_defeated, 250000);
}

#[test]
fn add_pents_awards_area_specific_and_tier_achievements() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();

    let unlocked = add_pents(&mut data, &mut stats, 34, 1, "Hero", NOW); // Hell area
    assert_eq!(
        unlocked,
        vec![AchievementType::ThroughGatesOfHell, AchievementType::Solved]
    );
    assert_eq!(stats.pents_per_area[PentArea::Hell as usize], 1);
    assert_eq!(stats.pents_solved, 1);

    // Non-pent area still counts toward the tier ladder, just no area-specific award.
    let unlocked = add_pents(&mut data, &mut stats, 1, 19, "Hero", NOW); // total 20
    assert_eq!(unlocked, vec![AchievementType::FullOfSolves]);

    add_pents(&mut data, &mut stats, 1, 80, "Hero", NOW); // total 100
    assert!(data.is_unlocked(AchievementType::RuneMaster));
    add_pents(&mut data, &mut stats, 1, 400, "Hero", NOW); // total 500
    assert!(data.is_unlocked(AchievementType::GrandmasterPentagram));
}

#[test]
fn add_pents_earth_fire_ice_variants() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    add_pents(&mut data, &mut stats, 4, 1, "Hero", NOW);
    assert!(data.is_unlocked(AchievementType::EarthboundNovice));
    add_pents(&mut data, &mut stats, 7, 1, "Hero", NOW);
    assert!(data.is_unlocked(AchievementType::FlameInitiate));
    add_pents(&mut data, &mut stats, 9, 1, "Hero", NOW);
    assert!(data.is_unlocked(AchievementType::FightingTheFrost));
}

#[test]
fn add_chests_full_ladder() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    add_chests(&mut data, &mut stats, 500, "Hero", NOW);
    for ty in [
        AchievementType::Looter,
        AchievementType::TreasureHunter,
        AchievementType::TreasureMaster,
        AchievementType::LegendaryLooter,
    ] {
        assert!(data.is_unlocked(ty));
    }
}

#[test]
fn add_stones_per_type_thresholds() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();

    let unlocked = add_stones(&mut data, &mut stats, STONE_TYPE_EARTH, 49, "Hero", NOW);
    assert!(unlocked.is_empty());
    let unlocked = add_stones(&mut data, &mut stats, STONE_TYPE_EARTH, 1, "Hero", NOW);
    assert_eq!(unlocked, vec![AchievementType::EarthRocks]);

    let unlocked = add_stones(&mut data, &mut stats, STONE_TYPE_FIRE, 100, "Hero", NOW);
    assert_eq!(unlocked, vec![AchievementType::FireRocks]);

    let unlocked = add_stones(&mut data, &mut stats, STONE_TYPE_ICE, 1000, "Hero", NOW);
    assert_eq!(unlocked, vec![AchievementType::IceRocks]);

    // Unknown stone type is a documented no-op (matches C's switch default).
    let unlocked = add_stones(&mut data, &mut stats, 99, 1000, "Hero", NOW);
    assert!(unlocked.is_empty());
}

#[test]
fn add_enemy_killed_awards_first_blood_once() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    let unlocked = add_enemy_killed(&mut data, &mut stats, "Hero", NOW);
    assert_eq!(unlocked, vec![AchievementType::FirstBlood]);
    let unlocked = add_enemy_killed(&mut data, &mut stats, "Hero", NOW);
    assert!(unlocked.is_empty());
    assert_eq!(stats.enemies_killed, 2);
}

#[test]
fn add_pvp_kill_awards_arena_combatant() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    let unlocked = add_pvp_kill(&mut data, &mut stats, "Hero", NOW);
    assert_eq!(unlocked, vec![AchievementType::ArenaCombatant]);
}

#[test]
fn add_military_mission_full_ladder() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    for _ in 0..1000 {
        add_military_mission(&mut data, &mut stats, "Hero", NOW);
    }
    for ty in [
        AchievementType::Recruit,
        AchievementType::Soldier,
        AchievementType::MilitaryVeteran,
        AchievementType::Commander,
        AchievementType::General,
        AchievementType::WarLegend,
    ] {
        assert!(data.is_unlocked(ty));
    }
}

#[test]
fn add_tunnel_level_full_ladder_incl_tunnel_rat() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    for _ in 0..100 {
        add_tunnel_level(&mut data, &mut stats, "Hero", NOW);
    }
    for ty in [
        AchievementType::TunnelExplorer,
        AchievementType::TunnelRunner,
        AchievementType::TunnelVeteran,
        AchievementType::TunnelRat,
    ] {
        assert!(data.is_unlocked(ty));
    }
}

#[test]
fn add_silver_mined_full_ladder() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    add_silver_mined(&mut data, &mut stats, u32::MAX, "Hero", NOW);
    add_silver_mined(&mut data, &mut stats, u32::MAX, "Hero", NOW);
    assert!(stats.silver_mined >= 50_000_000);
    assert!(data.is_unlocked(AchievementType::SilverLegend));
}

#[test]
fn add_gold_mined_full_ladder() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    add_gold_mined(&mut data, &mut stats, u32::MAX, "Hero", NOW);
    assert!(data.is_unlocked(AchievementType::GoldLegend));
}

#[test]
fn add_gold_earned_full_ladder() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    let unlocked = add_gold_earned(&mut data, &mut stats, 10_000, "Hero", NOW);
    assert_eq!(unlocked, vec![AchievementType::CoinCollector]);
    add_gold_earned(&mut data, &mut stats, 90_000, "Hero", NOW); // 100,000
    assert!(data.is_unlocked(AchievementType::WealthyAdventurer));
    add_gold_earned(&mut data, &mut stats, 900_000, "Hero", NOW); // 1,000,000
    assert!(data.is_unlocked(AchievementType::RichNoble));
    add_gold_earned(&mut data, &mut stats, 9_000_000, "Hero", NOW); // 10,000,000
    assert!(data.is_unlocked(AchievementType::Millionaire));
}

#[test]
fn add_play_time_full_ladder() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    add_play_time(&mut data, &mut stats, 1440, "Hero", NOW);
    assert!(data.is_unlocked(AchievementType::DedicatedPlayer));
    add_play_time(&mut data, &mut stats, 6000 - 1440, "Hero", NOW);
    assert!(data.is_unlocked(AchievementType::VeteranPlayer));
    add_play_time(&mut data, &mut stats, 30000 - 6000, "Hero", NOW);
    assert!(data.is_unlocked(AchievementType::UgarisLifer));
}

#[test]
fn check_login_streak_first_login_sets_streak_one() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    let day0 = 100 * 86400;
    check_login_streak(&mut data, &mut stats, "Hero", day0);
    assert_eq!(stats.login_streak, 1);
    assert_eq!(stats.last_login_day, 100);
}

#[test]
fn check_login_streak_same_day_is_a_no_op() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    let day0 = 100 * 86400;
    check_login_streak(&mut data, &mut stats, "Hero", day0);
    check_login_streak(&mut data, &mut stats, "Hero", day0 + 3600);
    assert_eq!(stats.login_streak, 1);
}

#[test]
fn check_login_streak_consecutive_days_increment() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    let day0 = 100 * 86400;
    for i in 0..7 {
        check_login_streak(&mut data, &mut stats, "Hero", day0 + i * 86400);
    }
    assert_eq!(stats.login_streak, 7);
    assert!(data.is_unlocked(AchievementType::Regular));
}

#[test]
// `1 * 86400` kept for the readable `day0 + <n> * 86400` day series.
#[allow(clippy::identity_op)]
fn check_login_streak_gap_resets_to_one() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    let day0 = 100 * 86400;
    check_login_streak(&mut data, &mut stats, "Hero", day0);
    check_login_streak(&mut data, &mut stats, "Hero", day0 + 1 * 86400);
    assert_eq!(stats.login_streak, 2);
    // Skip a day (gap of 2 days instead of 1) -> streak resets.
    check_login_streak(&mut data, &mut stats, "Hero", day0 + 3 * 86400);
    assert_eq!(stats.login_streak, 1);
}

#[test]
fn check_login_streak_committed_and_devoted_thresholds() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    let day0 = 100 * 86400;
    for i in 0..100 {
        check_login_streak(&mut data, &mut stats, "Hero", day0 + i * 86400);
    }
    assert_eq!(stats.login_streak, 100);
    assert!(data.is_unlocked(AchievementType::Committed));
    assert!(data.is_unlocked(AchievementType::Devoted));
}

#[test]
fn check_level_standard_thresholds() {
    let mut data = AccountAchievements::default();
    for level in [10, 20, 50, 75, 100, 150, 200] {
        check_level(&mut data, level, false, "Hero", NOW);
    }
    for ty in [
        AchievementType::RisingBeginner,
        AchievementType::ExperiencedHero,
        AchievementType::UgarisVeteran,
        AchievementType::LegendaryAdventurer,
        AchievementType::DemonSlayer,
        AchievementType::MasterOfHell,
        AchievementType::MasterOfUgaris,
    ] {
        assert!(data.is_unlocked(ty));
    }
    assert!(!data.is_unlocked(AchievementType::HardcoreHero));
    assert!(!data.is_unlocked(AchievementType::HardcoreLegend));
}

#[test]
fn check_level_hardcore_variants_only_awarded_when_hardcore() {
    let mut data = AccountAchievements::default();
    let unlocked = check_level(&mut data, 50, true, "Hero", NOW);
    assert!(unlocked.contains(&AchievementType::UgarisVeteran));
    assert!(unlocked.contains(&AchievementType::HardcoreHero));

    let unlocked = check_level(&mut data, 100, true, "Hero", NOW);
    assert!(unlocked.contains(&AchievementType::HardcoreLegend));
}

#[test]
fn check_skill_weapon_range_covers_dagger_through_twohand() {
    let mut data = AccountAchievements::default();
    let unlocked = check_skill(&mut data, V_DAGGER, 10, "Hero", NOW);
    assert_eq!(unlocked, vec![AchievementType::WeaponNovice]);
    let unlocked = check_skill(&mut data, V_TWOHAND, 110, "Hero", NOW);
    assert_eq!(unlocked, vec![AchievementType::MasterOfArms]);
    // Out of range: V_ARMORSKILL (17) is not a weapon skill.
    let unlocked = check_skill(&mut data, 17, 110, "Hero", NOW);
    assert!(unlocked.is_empty());
}

#[test]
fn check_skill_magic_ladder() {
    let mut data = AccountAchievements::default();
    let unlocked = check_skill(&mut data, V_FIRE, 10, "Hero", NOW);
    assert_eq!(unlocked, vec![AchievementType::ApprenticeMagic]);
    let unlocked = check_skill(&mut data, V_FLASH, 50, "Hero", NOW);
    assert_eq!(unlocked, vec![AchievementType::IntermediateMagic]);
    let unlocked = check_skill(&mut data, V_FIRE, 110, "Hero", NOW);
    assert_eq!(unlocked, vec![AchievementType::MasterOfMagic]);
}

#[test]
fn check_skill_fighting_ladder() {
    let mut data = AccountAchievements::default();
    let unlocked = check_skill(&mut data, V_ATTACK, 10, "Hero", NOW);
    assert_eq!(unlocked, vec![AchievementType::ApprenticeFighting]);
    let unlocked = check_skill(&mut data, V_PARRY, 50, "Hero", NOW);
    assert_eq!(unlocked, vec![AchievementType::IntermediateFighting]);
    let unlocked = check_skill(&mut data, V_ATTACK, 110, "Hero", NOW);
    assert_eq!(unlocked, vec![AchievementType::MasterOfFighting]);
}

#[test]
fn check_profession_every_branch() {
    let cases = [
        (P_ATHLETE, 30, AchievementType::MasterAthlete),
        (P_ALCHEMIST, 50, AchievementType::MasterAlchemist),
        (P_MINER, 20, AchievementType::MasterMiner),
        (P_ASSASSIN, 50, AchievementType::MasterAssassin),
        (P_THIEF, 30, AchievementType::MasterThief),
        (P_LIGHT, 30, AchievementType::MasterLightWarrior),
        (P_DARK, 30, AchievementType::MasterDarkWarrior),
        (P_TRADER, 20, AchievementType::MasterTrader),
        (P_MERCENARY, 20, AchievementType::MasterMercenary),
        (P_CLAN, 30, AchievementType::MasterClanWarrior),
        (P_HERBALIST, 30, AchievementType::MasterHerbalistProf),
    ];
    for (prof_type, threshold, ty) in cases {
        let mut data = AccountAchievements::default();
        let unlocked = check_profession(&mut data, prof_type, threshold - 1, "Hero", NOW);
        assert!(
            unlocked.is_empty(),
            "unexpected unlock below threshold for {ty:?}"
        );
        let unlocked = check_profession(&mut data, prof_type, threshold, "Hero", NOW);
        assert_eq!(unlocked, vec![ty]);
    }
}

#[test]
fn check_profession_unknown_type_is_a_no_op() {
    let mut data = AccountAchievements::default();
    let unlocked = check_profession(&mut data, 99, 1000, "Hero", NOW);
    assert!(unlocked.is_empty());
}

#[test]
fn check_exploration_awards_only_for_aston() {
    let mut data = AccountAchievements::default();
    let unlocked = check_exploration(&mut data, 1, "Hero", NOW);
    assert!(unlocked.is_empty());
    let unlocked = check_exploration(&mut data, AREA_ASTON, "Hero", NOW);
    assert_eq!(unlocked, vec![AchievementType::UgarisPathfinder]);
}

#[test]
fn clear_all_resets_both_data_and_stats() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    data.award(AchievementType::FirstBlood, "Hero", NOW);
    stats.flowers_picked = 42;
    clear_all(&mut data, &mut stats);
    assert!(!data.is_unlocked(AchievementType::FirstBlood));
    assert_eq!(stats.flowers_picked, 0);
}

#[test]
fn achieved_by_name_is_truncated_to_39_chars_matching_c_buffer() {
    let mut data = AccountAchievements::default();
    let long_name: String = "A".repeat(60);
    data.award(AchievementType::FirstBlood, &long_name, NOW);
    assert_eq!(
        data.achievements[AchievementType::FirstBlood as usize]
            .achieved_by
            .len(),
        39
    );
}

#[test]
fn account_achievements_json_roundtrip_preserves_all_128_slots() {
    let mut data = AccountAchievements::default();
    data.award(AchievementType::FirstBlood, "Hero", NOW);
    data.add_progress(AchievementType::DemonSlayer, 3, "Hero", NOW);
    let last = MAX_ACHIEVEMENTS - 1;
    data.achievements[last].progress = 7;

    let json = serde_json::to_string(&data).expect("serialize");
    let restored: AccountAchievements = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, data);
    assert!(restored.is_unlocked(AchievementType::FirstBlood));
    assert_eq!(restored.achievements[last].progress, 7);
}

#[test]
fn account_achievements_deserializes_from_short_legacy_array() {
    // Simulate an older/short PPD blob: only the first few slots
    // present. Missing trailing slots must fall back to
    // `Achievement::default()` instead of erroring.
    let json = r#"{"version":1,"achievements":[
            {"timestamp":5,"progress":1,"target":1,"achieved_by":"Hero"}
        ]}"#;
    let restored: AccountAchievements = serde_json::from_str(json).expect("deserialize");
    assert_eq!(restored.achievements[0].timestamp, 5);
    assert_eq!(restored.achievements[1], Achievement::default());
    assert_eq!(
        restored.achievements[MAX_ACHIEVEMENTS - 1],
        Achievement::default()
    );
}

#[test]
fn achievement_stats_json_roundtrip() {
    let mut stats = AchievementStats::default();
    stats.flowers_picked = 10;
    stats.demons_per_area = [1, 2, 3, 4];
    stats.gold_earned = 123_456_789;

    let json = serde_json::to_string(&stats).expect("serialize");
    let restored: AchievementStats = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, stats);
}

#[test]
fn fix_all_stat_thresholds_awards_every_crossed_tier_from_current_totals() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    stats.flowers_picked = 1000;
    stats.gold_mined = 50_000_000;
    stats.enemies_killed = 1;

    let unlocked = fix_all_stat_thresholds(&mut data, &stats, "Hero", NOW);

    for ty in [
        AchievementType::GreenThumb,
        AchievementType::BotanyEnthusiast,
        AchievementType::NaturesFriend,
        AchievementType::Herbalist,
        AchievementType::MasterHerbalist,
        AchievementType::GoldNovice,
        AchievementType::GoldCollector,
        AchievementType::GoldHoarder,
        AchievementType::GoldBaron,
        AchievementType::GoldTycoon,
        AchievementType::GoldMagnate,
        AchievementType::GoldLegend,
        AchievementType::FirstBlood,
    ] {
        assert!(unlocked.contains(&ty), "expected {ty:?} to be unlocked");
        assert!(data.is_unlocked(ty));
    }
    // Nothing else should have been touched (e.g. mushrooms stayed 0).
    assert!(!data.is_unlocked(AchievementType::MushroomHunter));
}

#[test]
fn fix_all_stat_thresholds_is_a_noop_below_every_threshold() {
    let mut data = AccountAchievements::default();
    let stats = AchievementStats::default();
    let unlocked = fix_all_stat_thresholds(&mut data, &stats, "Hero", NOW);
    assert!(unlocked.is_empty());
    assert_eq!(data, AccountAchievements::default());
}

#[test]
fn fix_all_stat_thresholds_does_not_double_award_already_unlocked() {
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    stats.chests_opened = 10;
    data.award(AchievementType::Looter, "Hero", 1);

    let unlocked = fix_all_stat_thresholds(&mut data, &stats, "Hero", NOW);
    assert!(!unlocked.contains(&AchievementType::Looter));
    // Original award's `achieved_by`/timestamp must be untouched.
    assert_eq!(
        data.achievements[AchievementType::Looter as usize].timestamp,
        1
    );
}

#[test]
fn fix_all_stat_thresholds_excludes_per_area_pent_and_demon_achievements() {
    // C's `achievement_fix_all` only re-checks the *aggregate*
    // demon/pentagram thresholds, never the per-area ones
    // (`EarthboundNovice` etc.) - those stay gated behind
    // `achievement_check_exploration`-adjacent per-hit calls this
    // function deliberately doesn't replicate.
    let mut data = AccountAchievements::default();
    let mut stats = AchievementStats::default();
    stats.demons_per_area[PentArea::Earth as usize] = 999;
    stats.pents_per_area[PentArea::Earth as usize] = 999;
    let unlocked = fix_all_stat_thresholds(&mut data, &stats, "Hero", NOW);
    assert!(!unlocked.contains(&AchievementType::EarthboundNovice));
    assert!(!data.is_unlocked(AchievementType::EarthboundNovice));
}
