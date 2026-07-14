//! Per-account achievement storage structs and their serde support.

use super::*;

/// C `struct Achievement` (`achievement.h:218-223`).
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct Achievement {
    /// Unix timestamp when earned; `0` = not achieved.
    pub timestamp: i64,
    pub progress: u32,
    pub target: u32,
    /// Character name who earned it (C `char achieved_by[40]`).
    pub achieved_by: String,
}

/// C `struct AccountAchievements` (`achievement.h:226-229`): per-subscriber
/// (account-wide in C; left per-character here pending the PPD/DB wiring
/// task noted in the module doc comment above) achievement storage.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AccountAchievements {
    pub version: u32,
    #[serde(with = "achievement_array_serde")]
    pub achievements: [Achievement; MAX_ACHIEVEMENTS],
}

/// `serde` support for the fixed-size 128-entry `Achievement` array.
/// `#[derive(Serialize, Deserialize)]` only covers array lengths 0..=32
/// out of the box (`Achievement` isn't `Copy`, so the const-generic array
/// impl serde otherwise offers doesn't apply here either); this goes
/// through a `Vec` on the wire and rebuilds the fixed array on load, padding
/// short/legacy data with `Achievement::default()` rather than erroring.
mod achievement_array_serde {
    use super::{Achievement, MAX_ACHIEVEMENTS};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        value: &[Achievement; MAX_ACHIEVEMENTS],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value.as_slice().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<[Achievement; MAX_ACHIEVEMENTS], D::Error> {
        let vec = Vec::<Achievement>::deserialize(deserializer)?;
        let mut out: [Achievement; MAX_ACHIEVEMENTS] =
            std::array::from_fn(|_| Achievement::default());
        for (slot, value) in out.iter_mut().zip(vec) {
            *slot = value;
        }
        Ok(out)
    }
}

impl Default for AccountAchievements {
    fn default() -> Self {
        AccountAchievements {
            version: 0,
            achievements: std::array::from_fn(|_| Achievement::default()),
        }
    }
}

impl AccountAchievements {
    /// C `achievement_is_unlocked` (`achievement.c:567-576`).
    pub fn is_unlocked(&self, ty: AchievementType) -> bool {
        self.achievements[ty as usize].timestamp != 0
    }

    /// C `achievement_get_progress` (`achievement.c:671-680`).
    pub fn get_progress(&self, ty: AchievementType) -> u32 {
        self.achievements[ty as usize].progress
    }

    /// C `achievement_award` (`achievement.c:578-632`), minus the
    /// Steam-sync/DB-first-unlock/chat-announce/log side effects the C
    /// version performs inline (those need `World`/networking access this
    /// leaf module doesn't have; the caller should perform them when this
    /// returns `true`). Returns `true` if this call newly unlocked the
    /// achievement (`false` if it was already unlocked).
    pub fn award(&mut self, ty: AchievementType, achieved_by: &str, now: i64) -> bool {
        let def = achievement_def(ty);
        let ach = &mut self.achievements[ty as usize];
        if ach.timestamp != 0 {
            return false;
        }
        ach.timestamp = now;
        ach.progress = if def.target > 0 { def.target } else { 1 };
        ach.target = def.target;
        ach.achieved_by = achieved_by.chars().take(39).collect();
        true
    }

    /// C `achievement_add_progress` (`achievement.c:634-669`). Returns
    /// `true` if this call's progress crossed the target and newly
    /// unlocked the achievement.
    pub fn add_progress(
        &mut self,
        ty: AchievementType,
        amount: u32,
        achieved_by: &str,
        now: i64,
    ) -> bool {
        let def = achievement_def(ty);
        {
            let ach = &mut self.achievements[ty as usize];
            if ach.timestamp != 0 {
                return false;
            }
            if def.target == 0 {
                return false;
            }
            ach.progress = ach.progress.saturating_add(amount);
            ach.target = def.target;
        }
        if self.achievements[ty as usize].progress >= def.target {
            self.award(ty, achieved_by, now)
        } else {
            false
        }
    }
}

/// C `struct AchievementStats` (`achievement.h:232-276`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct AchievementStats {
    pub flowers_picked: u32,
    pub mushrooms_picked: u32,
    pub berries_picked: u32,
    pub potions_brewed: u32,

    pub demons_defeated: u64,
    pub demons_per_area: [u64; PENT_AREA_COUNT],
    pub enemies_killed: u32,
    pub pvp_kills: u32,

    pub pents_solved: u32,
    pub pents_per_area: [u32; PENT_AREA_COUNT],
    pub lucky_pents_hit: u32,

    pub chests_opened: u32,
    pub earth_stones: u32,
    pub fire_stones: u32,
    pub ice_stones: u32,

    pub military_missions: u32,
    pub tunnel_levels: u32,

    pub silver_mined: u64,
    pub gold_mined: u64,

    pub gold_earned: u64,

    pub play_time_minutes: u32,

    pub login_streak: u32,
    pub last_login_day: u32,
}
