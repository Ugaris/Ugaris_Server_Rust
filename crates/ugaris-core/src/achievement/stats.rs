//! Stat-driven award logic: the `add_*`/`check_*` family and threshold fixups.

use super::*;

/// C `achievement_get_stat_progress` (`achievement.c:398-530`): the
/// current-progress value used for progress-bar display, derived from
/// `AchievementStats` rather than the per-achievement `progress` field
/// (which C keeps separately via `achievement_add_progress` and which this
/// module's stat-update functions below don't call - matching the C
/// module, where the `add_*` family calls `achievement_award` directly
/// once a threshold is crossed rather than `achievement_add_progress`).
pub fn get_stat_progress(stats: &AchievementStats, ty: AchievementType) -> u32 {
    use AchievementType::*;
    match ty {
        GreenThumb | BotanyEnthusiast | NaturesFriend | Herbalist | MasterHerbalist => {
            stats.flowers_picked
        }
        MushroomHunter | FungusFinder | SporeSeeker | MushroomMaster | Mycologist => {
            stats.mushrooms_picked
        }
        BerryPicker | FruitForager | BerryEnthusiast | HarvestHero | MasterGatherer => {
            stats.berries_picked
        }
        Alchemist | JourneymanBrewer | ArcaneAlchemist | GrandmasterBrewer | PotionMaster
        | LegendaryBrewer => stats.potions_brewed,
        FiendFighter | Demonbane | DreadDestroyer | DemonicExterminator => {
            stats.demons_defeated.min(u32::MAX as u64) as u32
        }
        FullOfSolves | RuneMaster | GrandmasterPentagram => stats.pents_solved,
        Looter | TreasureHunter | TreasureMaster | LegendaryLooter => stats.chests_opened,
        EarthRocks => stats.earth_stones,
        FireRocks => stats.fire_stones,
        IceRocks => stats.ice_stones,
        Recruit | Soldier | MilitaryVeteran | Commander | General | WarLegend => {
            stats.military_missions
        }
        TunnelExplorer | TunnelRunner | TunnelVeteran => stats.tunnel_levels,
        SilverNovice | SilverCollector | SilverHoarder | SilverBaron | SilverTycoon
        | SilverMagnate | SilverLegend => stats.silver_mined.min(u32::MAX as u64) as u32,
        GoldNovice | GoldCollector | GoldHoarder | GoldBaron | GoldTycoon | GoldMagnate
        | GoldLegend => stats.gold_mined.min(u32::MAX as u64) as u32,
        CoinCollector | WealthyAdventurer | RichNoble | Millionaire => {
            stats.gold_earned.min(u32::MAX as u64) as u32
        }
        DedicatedPlayer | VeteranPlayer | UgarisLifer => stats.play_time_minutes,
        Regular | Committed | Devoted => stats.login_streak,
        // Demon lord achievements are tracked separately, not in AchievementStats.
        SlayerOfDemonLords => 0,
        // All other achievements are instant/one-time, no progress tracking.
        _ => 0,
    }
}

/// Helper: push `ty` into `out` if `data.award(...)` newly unlocked it.
fn push_if_awarded(
    out: &mut Vec<AchievementType>,
    data: &mut AccountAchievements,
    ty: AchievementType,
    achieved_by: &str,
    now: i64,
) {
    if data.award(ty, achieved_by, now) {
        out.push(ty);
    }
}

/// C `achievement_add_flowers` (`achievement.c:686-710`).
pub fn add_flowers(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    count: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.flowers_picked = stats.flowers_picked.saturating_add(count);
    let mut out = Vec::new();
    if stats.flowers_picked >= 10 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::GreenThumb,
            achieved_by,
            now,
        );
    }
    if stats.flowers_picked >= 50 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::BotanyEnthusiast,
            achieved_by,
            now,
        );
    }
    if stats.flowers_picked >= 200 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::NaturesFriend,
            achieved_by,
            now,
        );
    }
    if stats.flowers_picked >= 500 {
        push_if_awarded(&mut out, data, AchievementType::Herbalist, achieved_by, now);
    }
    if stats.flowers_picked >= 1000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::MasterHerbalist,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_mushrooms` (`achievement.c:712-736`).
pub fn add_mushrooms(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    count: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.mushrooms_picked = stats.mushrooms_picked.saturating_add(count);
    let mut out = Vec::new();
    if stats.mushrooms_picked >= 10 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::MushroomHunter,
            achieved_by,
            now,
        );
    }
    if stats.mushrooms_picked >= 50 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::FungusFinder,
            achieved_by,
            now,
        );
    }
    if stats.mushrooms_picked >= 200 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::SporeSeeker,
            achieved_by,
            now,
        );
    }
    if stats.mushrooms_picked >= 500 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::MushroomMaster,
            achieved_by,
            now,
        );
    }
    if stats.mushrooms_picked >= 1000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::Mycologist,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_berries` (`achievement.c:738-762`).
pub fn add_berries(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    count: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.berries_picked = stats.berries_picked.saturating_add(count);
    let mut out = Vec::new();
    if stats.berries_picked >= 10 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::BerryPicker,
            achieved_by,
            now,
        );
    }
    if stats.berries_picked >= 50 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::FruitForager,
            achieved_by,
            now,
        );
    }
    if stats.berries_picked >= 200 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::BerryEnthusiast,
            achieved_by,
            now,
        );
    }
    if stats.berries_picked >= 500 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::HarvestHero,
            achieved_by,
            now,
        );
    }
    if stats.berries_picked >= 1000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::MasterGatherer,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_potions` (`achievement.c:764-791`).
pub fn add_potions(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    count: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.potions_brewed = stats.potions_brewed.saturating_add(count);
    let mut out = Vec::new();
    if stats.potions_brewed >= 10 {
        push_if_awarded(&mut out, data, AchievementType::Alchemist, achieved_by, now);
    }
    if stats.potions_brewed >= 50 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::JourneymanBrewer,
            achieved_by,
            now,
        );
    }
    if stats.potions_brewed >= 100 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::ArcaneAlchemist,
            achieved_by,
            now,
        );
    }
    if stats.potions_brewed >= 200 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::GrandmasterBrewer,
            achieved_by,
            now,
        );
    }
    if stats.potions_brewed >= 500 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::PotionMaster,
            achieved_by,
            now,
        );
    }
    if stats.potions_brewed >= 1000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::LegendaryBrewer,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_demons` (`achievement.c:793-819`).
pub fn add_demons(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    area_id: i32,
    count: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    if let Some(idx) = area_to_pent_index(area_id) {
        stats.demons_per_area[idx as usize] =
            stats.demons_per_area[idx as usize].saturating_add(count as u64);
    }
    stats.demons_defeated = stats.demons_defeated.saturating_add(count as u64);
    let mut out = Vec::new();
    if stats.demons_defeated >= 100 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::FiendFighter,
            achieved_by,
            now,
        );
    }
    if stats.demons_defeated >= 2500 {
        push_if_awarded(&mut out, data, AchievementType::Demonbane, achieved_by, now);
    }
    if stats.demons_defeated >= 15000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::DreadDestroyer,
            achieved_by,
            now,
        );
    }
    if stats.demons_defeated >= 250000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::DemonicExterminator,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_pents` (`achievement.c:821-863`).
pub fn add_pents(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    area_id: i32,
    count: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    let mut out = Vec::new();
    if let Some(idx) = area_to_pent_index(area_id) {
        stats.pents_per_area[idx as usize] =
            stats.pents_per_area[idx as usize].saturating_add(count);
        let area_ach = match idx {
            PentArea::Earth => AchievementType::EarthboundNovice,
            PentArea::Fire => AchievementType::FlameInitiate,
            PentArea::Ice => AchievementType::FightingTheFrost,
            PentArea::Hell => AchievementType::ThroughGatesOfHell,
        };
        push_if_awarded(&mut out, data, area_ach, achieved_by, now);
    }

    stats.pents_solved = stats.pents_solved.saturating_add(count);
    if stats.pents_solved >= 1 {
        push_if_awarded(&mut out, data, AchievementType::Solved, achieved_by, now);
    }
    if stats.pents_solved >= 20 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::FullOfSolves,
            achieved_by,
            now,
        );
    }
    if stats.pents_solved >= 100 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::RuneMaster,
            achieved_by,
            now,
        );
    }
    if stats.pents_solved >= 500 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::GrandmasterPentagram,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_chests` (`achievement.c:865-886`).
pub fn add_chests(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    count: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.chests_opened = stats.chests_opened.saturating_add(count);
    let mut out = Vec::new();
    if stats.chests_opened >= 10 {
        push_if_awarded(&mut out, data, AchievementType::Looter, achieved_by, now);
    }
    if stats.chests_opened >= 50 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::TreasureHunter,
            achieved_by,
            now,
        );
    }
    if stats.chests_opened >= 100 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::TreasureMaster,
            achieved_by,
            now,
        );
    }
    if stats.chests_opened >= 500 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::LegendaryLooter,
            achieved_by,
            now,
        );
    }
    out
}

/// Legacy stone-type indices used by C `achievement_add_stones`'s
/// `switch (stone_type)` (`achievement.c:894-913`): `0` = Earth, `1` =
/// Fire, `2` = Ice.
pub const STONE_TYPE_EARTH: i32 = 0;
pub const STONE_TYPE_FIRE: i32 = 1;
pub const STONE_TYPE_ICE: i32 = 2;

/// C `achievement_add_stones` (`achievement.c:888-914`).
pub fn add_stones(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    stone_type: i32,
    count: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    let mut out = Vec::new();
    match stone_type {
        STONE_TYPE_EARTH => {
            stats.earth_stones = stats.earth_stones.saturating_add(count);
            if stats.earth_stones >= 50 {
                push_if_awarded(
                    &mut out,
                    data,
                    AchievementType::EarthRocks,
                    achieved_by,
                    now,
                );
            }
        }
        STONE_TYPE_FIRE => {
            stats.fire_stones = stats.fire_stones.saturating_add(count);
            if stats.fire_stones >= 100 {
                push_if_awarded(&mut out, data, AchievementType::FireRocks, achieved_by, now);
            }
        }
        STONE_TYPE_ICE => {
            stats.ice_stones = stats.ice_stones.saturating_add(count);
            if stats.ice_stones >= 1000 {
                push_if_awarded(&mut out, data, AchievementType::IceRocks, achieved_by, now);
            }
        }
        _ => {}
    }
    out
}

/// C `achievement_add_enemy_killed` (`achievement.c:916-928`).
pub fn add_enemy_killed(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.enemies_killed = stats.enemies_killed.saturating_add(1);
    let mut out = Vec::new();
    if stats.enemies_killed == 1 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::FirstBlood,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_pvp_kill` (`achievement.c:930-941`).
pub fn add_pvp_kill(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.pvp_kills = stats.pvp_kills.saturating_add(1);
    let mut out = Vec::new();
    if stats.pvp_kills >= 1 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::ArenaCombatant,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_military_mission` (`achievement.c:943-970`).
pub fn add_military_mission(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.military_missions = stats.military_missions.saturating_add(1);
    let mut out = Vec::new();
    if stats.military_missions >= 10 {
        push_if_awarded(&mut out, data, AchievementType::Recruit, achieved_by, now);
    }
    if stats.military_missions >= 25 {
        push_if_awarded(&mut out, data, AchievementType::Soldier, achieved_by, now);
    }
    if stats.military_missions >= 100 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::MilitaryVeteran,
            achieved_by,
            now,
        );
    }
    if stats.military_missions >= 250 {
        push_if_awarded(&mut out, data, AchievementType::Commander, achieved_by, now);
    }
    if stats.military_missions >= 500 {
        push_if_awarded(&mut out, data, AchievementType::General, achieved_by, now);
    }
    if stats.military_missions >= 1000 {
        push_if_awarded(&mut out, data, AchievementType::WarLegend, achieved_by, now);
    }
    out
}

/// C `achievement_add_tunnel_level` (`achievement.c:972-994`).
pub fn add_tunnel_level(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.tunnel_levels = stats.tunnel_levels.saturating_add(1);
    let mut out = Vec::new();
    if stats.tunnel_levels >= 10 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::TunnelExplorer,
            achieved_by,
            now,
        );
    }
    if stats.tunnel_levels >= 25 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::TunnelRunner,
            achieved_by,
            now,
        );
    }
    if stats.tunnel_levels >= 50 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::TunnelVeteran,
            achieved_by,
            now,
        );
    }
    // C comment: "Award Tunnel Rat achievement after completing 100 tunnel sections".
    if stats.tunnel_levels >= 100 {
        push_if_awarded(&mut out, data, AchievementType::TunnelRat, achieved_by, now);
    }
    out
}

/// C `achievement_add_silver_mined` (`achievement.c:996-1026`).
pub fn add_silver_mined(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    amount: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.silver_mined = stats.silver_mined.saturating_add(amount as u64);
    let mut out = Vec::new();
    if stats.silver_mined >= 100 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::SilverNovice,
            achieved_by,
            now,
        );
    }
    if stats.silver_mined >= 1000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::SilverCollector,
            achieved_by,
            now,
        );
    }
    if stats.silver_mined >= 10000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::SilverHoarder,
            achieved_by,
            now,
        );
    }
    if stats.silver_mined >= 100000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::SilverBaron,
            achieved_by,
            now,
        );
    }
    if stats.silver_mined >= 1000000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::SilverTycoon,
            achieved_by,
            now,
        );
    }
    if stats.silver_mined >= 10000000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::SilverMagnate,
            achieved_by,
            now,
        );
    }
    if stats.silver_mined >= 50000000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::SilverLegend,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_gold_mined` (`achievement.c:1028-1058`).
pub fn add_gold_mined(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    amount: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.gold_mined = stats.gold_mined.saturating_add(amount as u64);
    let mut out = Vec::new();
    if stats.gold_mined >= 50 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::GoldNovice,
            achieved_by,
            now,
        );
    }
    if stats.gold_mined >= 500 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::GoldCollector,
            achieved_by,
            now,
        );
    }
    if stats.gold_mined >= 5000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::GoldHoarder,
            achieved_by,
            now,
        );
    }
    if stats.gold_mined >= 50000 {
        push_if_awarded(&mut out, data, AchievementType::GoldBaron, achieved_by, now);
    }
    if stats.gold_mined >= 500000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::GoldTycoon,
            achieved_by,
            now,
        );
    }
    if stats.gold_mined >= 5000000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::GoldMagnate,
            achieved_by,
            now,
        );
    }
    if stats.gold_mined >= 50000000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::GoldLegend,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_gold_earned` (`achievement.c:1060-1081`).
pub fn add_gold_earned(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    amount: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.gold_earned = stats.gold_earned.saturating_add(amount as u64);
    let mut out = Vec::new();
    if stats.gold_earned >= 10000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::CoinCollector,
            achieved_by,
            now,
        );
    }
    if stats.gold_earned >= 100000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::WealthyAdventurer,
            achieved_by,
            now,
        );
    }
    if stats.gold_earned >= 1000000 {
        push_if_awarded(&mut out, data, AchievementType::RichNoble, achieved_by, now);
    }
    if stats.gold_earned >= 10000000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::Millionaire,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_add_play_time` (`achievement.c:1083-1101`).
pub fn add_play_time(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    minutes: u32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    stats.play_time_minutes = stats.play_time_minutes.saturating_add(minutes);
    let mut out = Vec::new();
    if stats.play_time_minutes >= 1440 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::DedicatedPlayer,
            achieved_by,
            now,
        );
    }
    if stats.play_time_minutes >= 6000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::VeteranPlayer,
            achieved_by,
            now,
        );
    }
    if stats.play_time_minutes >= 30000 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::UgarisLifer,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_check_login_streak` (`achievement.c:1103-1139`). `now`
/// is a Unix timestamp (seconds); C computes `current_day = now / 86400`.
pub fn check_login_streak(
    data: &mut AccountAchievements,
    stats: &mut AchievementStats,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    let current_day = (now / 86400) as u32;

    if stats.last_login_day == 0 {
        stats.login_streak = 1;
        stats.last_login_day = current_day;
    } else if current_day == stats.last_login_day {
        // Already logged in today, do nothing.
    } else if current_day == stats.last_login_day + 1 {
        stats.login_streak = stats.login_streak.saturating_add(1);
        stats.last_login_day = current_day;
    } else {
        stats.login_streak = 1;
        stats.last_login_day = current_day;
    }

    let mut out = Vec::new();
    if stats.login_streak >= 7 {
        push_if_awarded(&mut out, data, AchievementType::Regular, achieved_by, now);
    }
    if stats.login_streak >= 30 {
        push_if_awarded(&mut out, data, AchievementType::Committed, achieved_by, now);
    }
    if stats.login_streak >= 100 {
        push_if_awarded(&mut out, data, AchievementType::Devoted, achieved_by, now);
    }
    out
}

/// C `achievement_check_level` (`achievement.c:1145-1176`).
pub fn check_level(
    data: &mut AccountAchievements,
    level: i32,
    is_hardcore: bool,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    let mut out = Vec::new();
    if level >= 10 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::RisingBeginner,
            achieved_by,
            now,
        );
    }
    if level >= 20 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::ExperiencedHero,
            achieved_by,
            now,
        );
    }
    if level >= 50 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::UgarisVeteran,
            achieved_by,
            now,
        );
        if is_hardcore {
            push_if_awarded(
                &mut out,
                data,
                AchievementType::HardcoreHero,
                achieved_by,
                now,
            );
        }
    }
    if level >= 75 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::LegendaryAdventurer,
            achieved_by,
            now,
        );
    }
    if level >= 100 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::DemonSlayer,
            achieved_by,
            now,
        );
        if is_hardcore {
            push_if_awarded(
                &mut out,
                data,
                AchievementType::HardcoreLegend,
                achieved_by,
                now,
            );
        }
    }
    if level >= 150 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::MasterOfHell,
            achieved_by,
            now,
        );
    }
    if level >= 200 {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::MasterOfUgaris,
            achieved_by,
            now,
        );
    }
    out
}

/// Legacy weapon-skill value range (`V_DAGGER` .. `V_TWOHAND`,
/// `src/server.h:322-326`: dagger/hand-to-hand/staff/sword/two-hand).
pub const V_DAGGER: i32 = 12;
pub const V_TWOHAND: i32 = 16;
/// `src/server.h:328-329`.
pub const V_ATTACK: i32 = 18;
pub const V_PARRY: i32 = 19;
/// `src/server.h:342-344`: `V_FLASH` (Lightning) and `V_FIRE` (alias of
/// `V_FIREBALL`, already `crate::entity::V_FIREBALL`).
pub const V_FLASH: i32 = 32;
pub const V_FIRE: i32 = crate::entity::V_FIREBALL;

/// C `achievement_check_skill` (`achievement.c:1178-1218`).
pub fn check_skill(
    data: &mut AccountAchievements,
    skill_type: i32,
    skill_level: i32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    let mut out = Vec::new();
    if (V_DAGGER..=V_TWOHAND).contains(&skill_type) {
        if skill_level >= 10 {
            push_if_awarded(
                &mut out,
                data,
                AchievementType::WeaponNovice,
                achieved_by,
                now,
            );
        }
        if skill_level >= 110 {
            push_if_awarded(
                &mut out,
                data,
                AchievementType::MasterOfArms,
                achieved_by,
                now,
            );
        }
    }
    if skill_type == V_FIRE || skill_type == V_FLASH {
        if skill_level >= 10 {
            push_if_awarded(
                &mut out,
                data,
                AchievementType::ApprenticeMagic,
                achieved_by,
                now,
            );
        }
        if skill_level >= 50 {
            push_if_awarded(
                &mut out,
                data,
                AchievementType::IntermediateMagic,
                achieved_by,
                now,
            );
        }
        if skill_level >= 110 {
            push_if_awarded(
                &mut out,
                data,
                AchievementType::MasterOfMagic,
                achieved_by,
                now,
            );
        }
    }
    if skill_type == V_ATTACK || skill_type == V_PARRY {
        if skill_level >= 10 {
            push_if_awarded(
                &mut out,
                data,
                AchievementType::ApprenticeFighting,
                achieved_by,
                now,
            );
        }
        if skill_level >= 50 {
            push_if_awarded(
                &mut out,
                data,
                AchievementType::IntermediateFighting,
                achieved_by,
                now,
            );
        }
        if skill_level >= 110 {
            push_if_awarded(
                &mut out,
                data,
                AchievementType::MasterOfFighting,
                achieved_by,
                now,
            );
        }
    }
    out
}

/// Legacy profession type indices (`src/common/professor.c`'s `P_*`
/// constants, documented verbatim in `achievement_check_profession`'s C
/// comment, `achievement.c:1221-1223`).
pub const P_ATHLETE: i32 = 0;
pub const P_ALCHEMIST: i32 = 1;
pub const P_MINER: i32 = 2;
pub const P_ASSASSIN: i32 = 3;
pub const P_THIEF: i32 = 4;
pub const P_LIGHT: i32 = 5;
pub const P_DARK: i32 = 6;
pub const P_TRADER: i32 = 7;
pub const P_MERCENARY: i32 = 8;
pub const P_CLAN: i32 = 9;
pub const P_HERBALIST: i32 = 10;

/// C `achievement_check_profession` (`achievement.c:1220-1285`). Max
/// levels per profession are documented in the C comment
/// (`achievement.c:1225-1226`): Athlete=30, Alchemist=50, Miner=20,
/// Assassin=50, Thief=30, Light=30, Dark=30, Trader=20, Mercenary=20,
/// Clan=30, Herbalist=30.
pub fn check_profession(
    data: &mut AccountAchievements,
    prof_type: i32,
    prof_level: i32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    let mut out = Vec::new();
    let (threshold, ty) = match prof_type {
        P_ATHLETE => (30, AchievementType::MasterAthlete),
        P_ALCHEMIST => (50, AchievementType::MasterAlchemist),
        P_MINER => (20, AchievementType::MasterMiner),
        P_ASSASSIN => (50, AchievementType::MasterAssassin),
        P_THIEF => (30, AchievementType::MasterThief),
        P_LIGHT => (30, AchievementType::MasterLightWarrior),
        P_DARK => (30, AchievementType::MasterDarkWarrior),
        P_TRADER => (20, AchievementType::MasterTrader),
        P_MERCENARY => (20, AchievementType::MasterMercenary),
        P_CLAN => (30, AchievementType::MasterClanWarrior),
        P_HERBALIST => (30, AchievementType::MasterHerbalistProf),
        _ => return out,
    };
    if prof_level >= threshold {
        push_if_awarded(&mut out, data, ty, achieved_by, now);
    }
    out
}

/// Legacy area id for Aston (C `achievement_check_exploration`'s
/// `case 3:`, `achievement.c:1801`).
pub const AREA_ASTON: i32 = 3;

/// C `achievement_check_exploration` (`achievement.c:1794-1807`).
pub fn check_exploration(
    data: &mut AccountAchievements,
    area_id: i32,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    let mut out = Vec::new();
    if area_id == AREA_ASTON {
        push_if_awarded(
            &mut out,
            data,
            AchievementType::UgarisPathfinder,
            achieved_by,
            now,
        );
    }
    out
}

/// C `achievement_clear_all` (`achievement.c:1774-1788`).
pub fn clear_all(data: &mut AccountAchievements, stats: &mut AchievementStats) {
    *data = AccountAchievements::default();
    *stats = AchievementStats::default();
}

/// C `achievement_fix_all`'s stat-based re-check section
/// (`achievement.c:1511-1762`, the `if (stats) { ... }` block): re-derives
/// every stat-driven achievement from the *current* `AchievementStats`
/// totals, rather than a fresh delta, for `/achfix` use on players who
/// accrued progress before the achievement system existed. Deliberately
/// excludes the per-area demon/pentagram achievements (`EarthboundNovice`
/// etc. - not in the C function's checks either) and the level/
/// `Ladykiller`/exploration/profession checks, which the caller performs
/// separately via `check_level`/`check_profession`/`check_exploration`
/// and its own `CF_WON` check (mirroring `achievement_fix_all`'s own
/// structure, which calls those independently around this block).
/// Unlike the `add_*` family, every award here passes `show_congrats =
/// 0` in C (no chat text) - this function has no side effect beyond the
/// `AccountAchievements` mutation; the caller decides whether/how to
/// notify the client (`achievement_send_to_client`'s `SV_ACH_UNLOCK` is
/// unconditional in C regardless of `show_congrats`, so callers should
/// still send it for anything returned here).
pub fn fix_all_stat_thresholds(
    data: &mut AccountAchievements,
    stats: &AchievementStats,
    achieved_by: &str,
    now: i64,
) -> Vec<AchievementType> {
    let mut out = Vec::new();
    let mut award = |ty: AchievementType| {
        if data.award(ty, achieved_by, now) {
            out.push(ty);
        }
    };

    if stats.flowers_picked >= 10 {
        award(AchievementType::GreenThumb);
    }
    if stats.flowers_picked >= 50 {
        award(AchievementType::BotanyEnthusiast);
    }
    if stats.flowers_picked >= 200 {
        award(AchievementType::NaturesFriend);
    }
    if stats.flowers_picked >= 500 {
        award(AchievementType::Herbalist);
    }
    if stats.flowers_picked >= 1000 {
        award(AchievementType::MasterHerbalist);
    }

    if stats.mushrooms_picked >= 10 {
        award(AchievementType::MushroomHunter);
    }
    if stats.mushrooms_picked >= 50 {
        award(AchievementType::FungusFinder);
    }
    if stats.mushrooms_picked >= 200 {
        award(AchievementType::SporeSeeker);
    }
    if stats.mushrooms_picked >= 500 {
        award(AchievementType::MushroomMaster);
    }
    if stats.mushrooms_picked >= 1000 {
        award(AchievementType::Mycologist);
    }

    if stats.berries_picked >= 10 {
        award(AchievementType::BerryPicker);
    }
    if stats.berries_picked >= 50 {
        award(AchievementType::FruitForager);
    }
    if stats.berries_picked >= 200 {
        award(AchievementType::BerryEnthusiast);
    }
    if stats.berries_picked >= 500 {
        award(AchievementType::HarvestHero);
    }
    if stats.berries_picked >= 1000 {
        award(AchievementType::MasterGatherer);
    }

    if stats.potions_brewed >= 10 {
        award(AchievementType::Alchemist);
    }
    if stats.potions_brewed >= 50 {
        award(AchievementType::JourneymanBrewer);
    }
    if stats.potions_brewed >= 100 {
        award(AchievementType::ArcaneAlchemist);
    }
    if stats.potions_brewed >= 200 {
        award(AchievementType::GrandmasterBrewer);
    }
    if stats.potions_brewed >= 500 {
        award(AchievementType::PotionMaster);
    }
    if stats.potions_brewed >= 1000 {
        award(AchievementType::LegendaryBrewer);
    }

    if stats.demons_defeated >= 100 {
        award(AchievementType::FiendFighter);
    }
    if stats.demons_defeated >= 2500 {
        award(AchievementType::Demonbane);
    }
    if stats.demons_defeated >= 15000 {
        award(AchievementType::DreadDestroyer);
    }
    if stats.demons_defeated >= 250_000 {
        award(AchievementType::DemonicExterminator);
    }

    if stats.pents_solved >= 1 {
        award(AchievementType::Solved);
    }
    if stats.pents_solved >= 20 {
        award(AchievementType::FullOfSolves);
    }
    if stats.pents_solved >= 100 {
        award(AchievementType::RuneMaster);
    }
    if stats.pents_solved >= 500 {
        award(AchievementType::GrandmasterPentagram);
    }

    if stats.enemies_killed >= 1 {
        award(AchievementType::FirstBlood);
    }
    if stats.pvp_kills >= 1 {
        award(AchievementType::ArenaCombatant);
    }

    if stats.chests_opened >= 10 {
        award(AchievementType::Looter);
    }
    if stats.chests_opened >= 50 {
        award(AchievementType::TreasureHunter);
    }
    if stats.chests_opened >= 100 {
        award(AchievementType::TreasureMaster);
    }
    if stats.chests_opened >= 500 {
        award(AchievementType::LegendaryLooter);
    }

    if stats.earth_stones >= 50 {
        award(AchievementType::EarthRocks);
    }
    if stats.fire_stones >= 100 {
        award(AchievementType::FireRocks);
    }
    if stats.ice_stones >= 1000 {
        award(AchievementType::IceRocks);
    }

    if stats.military_missions >= 10 {
        award(AchievementType::Recruit);
    }
    if stats.military_missions >= 25 {
        award(AchievementType::Soldier);
    }
    if stats.military_missions >= 100 {
        award(AchievementType::MilitaryVeteran);
    }
    if stats.military_missions >= 250 {
        award(AchievementType::Commander);
    }
    if stats.military_missions >= 500 {
        award(AchievementType::General);
    }
    if stats.military_missions >= 1000 {
        award(AchievementType::WarLegend);
    }

    if stats.tunnel_levels >= 10 {
        award(AchievementType::TunnelExplorer);
    }
    if stats.tunnel_levels >= 25 {
        award(AchievementType::TunnelRunner);
    }
    if stats.tunnel_levels >= 50 {
        award(AchievementType::TunnelVeteran);
    }
    if stats.tunnel_levels >= 100 {
        award(AchievementType::TunnelRat);
    }

    if stats.gold_earned >= 10_000 {
        award(AchievementType::CoinCollector);
    }
    if stats.gold_earned >= 100_000 {
        award(AchievementType::WealthyAdventurer);
    }
    if stats.gold_earned >= 1_000_000 {
        award(AchievementType::RichNoble);
    }
    if stats.gold_earned >= 10_000_000 {
        award(AchievementType::Millionaire);
    }

    if stats.play_time_minutes >= 1440 {
        award(AchievementType::DedicatedPlayer);
    }
    if stats.play_time_minutes >= 6000 {
        award(AchievementType::VeteranPlayer);
    }
    if stats.play_time_minutes >= 30_000 {
        award(AchievementType::UgarisLifer);
    }

    if stats.login_streak >= 7 {
        award(AchievementType::Regular);
    }
    if stats.login_streak >= 30 {
        award(AchievementType::Committed);
    }
    if stats.login_streak >= 100 {
        award(AchievementType::Devoted);
    }

    if stats.silver_mined >= 100 {
        award(AchievementType::SilverNovice);
    }
    if stats.silver_mined >= 1_000 {
        award(AchievementType::SilverCollector);
    }
    if stats.silver_mined >= 10_000 {
        award(AchievementType::SilverHoarder);
    }
    if stats.silver_mined >= 100_000 {
        award(AchievementType::SilverBaron);
    }
    if stats.silver_mined >= 1_000_000 {
        award(AchievementType::SilverTycoon);
    }
    if stats.silver_mined >= 10_000_000 {
        award(AchievementType::SilverMagnate);
    }
    if stats.silver_mined >= 50_000_000 {
        award(AchievementType::SilverLegend);
    }

    if stats.gold_mined >= 50 {
        award(AchievementType::GoldNovice);
    }
    if stats.gold_mined >= 500 {
        award(AchievementType::GoldCollector);
    }
    if stats.gold_mined >= 5_000 {
        award(AchievementType::GoldHoarder);
    }
    if stats.gold_mined >= 50_000 {
        award(AchievementType::GoldBaron);
    }
    if stats.gold_mined >= 500_000 {
        award(AchievementType::GoldTycoon);
    }
    if stats.gold_mined >= 5_000_000 {
        award(AchievementType::GoldMagnate);
    }
    if stats.gold_mined >= 50_000_000 {
        award(AchievementType::GoldLegend);
    }

    out
}
