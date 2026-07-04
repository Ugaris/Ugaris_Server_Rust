//! Imperial Army rank thresholds - C `src/module/military.c` +
//! `src/system/tool.c`'s shared point-award/promotion helper
//! (`give_military_pts`/`give_military_pts_no_npc`, `tool.c:3249-3306`).
//!
//! Only the rank table and the point-award/promotion helper are ported
//! here (the "Military ranks" P3 task in `PORTING_TODO.md`'s first
//! slice). The mission PPD (`military_ppd`/`mission_ppd.h`), the advisor
//! NPC driver, and the governor mission flow (`check_military_solve`,
//! referenced from the death path) remain unported.

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
