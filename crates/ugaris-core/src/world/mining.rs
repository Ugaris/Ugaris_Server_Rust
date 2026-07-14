//! Area 12 (`src/area/12/mine.c`) diggable-wall reward cascade.
//!
//! Ports the pure (no `ZoneLoader` needed) parts of C's
//! `handle_mining_result` (`mine.c:222-279`), fired from
//! `process_wall_digging` once a wall reaches `drdata[3] == 8` (already
//! wired as `ItemDriverOutcome::MineWallDig { opened: true, .. }`, see
//! `item_driver::area12_mine::minewall_driver`): the weighted event roll
//! itself ([`World::roll_mining_event`]), the silver/gold amount rolls
//! (`handle_silver_find`/`handle_gold_find`), the rare-vs-normal golem
//! roll (`handle_golem_spawn`), the golem loot-drop amount roll
//! (`calculate_drop_amount`), and the full cave-in endurance-loss
//! mechanic (`handle_cave_in`, entirely self-contained since it only
//! mutates `Character::endurance`).
//!
//! The orb roll (`handle_orb_find`) and the artifact-relic table
//! (`handle_artifact_find`) are not ported yet (see `PORTING_TODO.md`'s
//! Area 12 entry - REMAINING note); both are rare branches (5 and 200 out
//! of the 100,000-wide roll respectively) and need the same `ZoneLoader`/
//! achievement-repository access described below.
//!
//! Everything that needs `ZoneLoader` (instantiating "silver"/"gold"/
//! golem-template items and characters) or `PlayerRuntime`/achievement
//! wiring (military-mission silver tracking, the mined-amount achievement
//! ladders) lives in `ugaris-server`'s `mine.rs`, mirroring the chest
//! family's core/server split (`world/loot.rs` decides, `ugaris-server/
//! src/chests.rs` instantiates).

use super::*;
use crate::item_driver::drdata;

/// C `handle_mining_result`'s weighted-event roll (`mine.c:229-275`),
/// decided from `GameSettings` tunables and `World::legacy_random_seed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MiningEvent {
    Silver,
    Gold,
    Golem,
    Orb,
    CaveIn,
    Artifact,
    /// C's fall-through (commented-out `"You didn't find anything of
    /// value this time."` - too spammy, left disabled in C itself).
    Nothing,
}

/// Outcome of [`World::apply_mining_cave_in`] (C `handle_cave_in`,
/// `mine.c:362-403`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaveInResult {
    /// C: the miner's `P_MINER`-scaled avoid-chance roll succeeded
    /// ("Your mining expertise helped you avoid a cave-in!").
    Avoided,
    /// The wall collapsed; `endurance_loss_units` is already applied to
    /// `Character::endurance` by the time this is returned (already
    /// divided by `POWERSCALE`, ready to format into the C message
    /// text). `unreduced_loss_units` is `Some` only when `P_ATHLETE > 0`
    /// - C's athlete-reduction message needs both the actual loss and a
    ///   reverse-divided "instead of" value computed from the *already*
    ///   `min(_, endurance)`-clamped loss (a real C quirk: if the clamp
    ///   kicked in, the "instead of" number is derived from the clamped
    ///   value, not the pre-clamp one). `now_exhausted` mirrors C's
    ///   trailing `endurance < POWERSCALE` warning.
    Collapsed {
        endurance_loss_units: i32,
        unreduced_loss_units: Option<i32>,
        now_exhausted: bool,
    },
}

/// Pure classification half of [`World::roll_mining_event`] (C
/// `handle_mining_result`, `mine.c:229-275`): given an already-drawn
/// `RANDOM(100000)` roll, walks the six event bands, each base chance
/// separately scaled by its own `GameSettings` multiplier and truncated
/// to an integer *before* accumulating - matching C's `cumulative_chance
/// += (int)(base * mult);` exactly (not `(int)(sum * mult)`).
pub(crate) fn classify_mining_roll(roll: i64, s: &GameSettings) -> MiningEvent {
    let mut cumulative: i64 = 0;

    cumulative += (f64::from(s.mining_silver_chance_base) * s.mining_silver_gold_multiplier) as i64;
    if roll < cumulative {
        return MiningEvent::Silver;
    }
    cumulative += (f64::from(s.mining_gold_chance_base) * s.mining_silver_gold_multiplier) as i64;
    if roll < cumulative {
        return MiningEvent::Gold;
    }
    cumulative += (f64::from(s.mining_golem_chance_base) * s.mining_golem_event_multiplier) as i64;
    if roll < cumulative {
        return MiningEvent::Golem;
    }
    cumulative += i64::from(s.mining_orb_chance_base);
    if roll < cumulative {
        return MiningEvent::Orb;
    }
    cumulative += (f64::from(s.mining_cavein_chance_base) * s.mining_cavein_multiplier) as i64;
    if roll < cumulative {
        return MiningEvent::CaveIn;
    }
    cumulative += (f64::from(s.mining_artifact_chance_base) * s.mining_artifact_multiplier) as i64;
    if roll < cumulative {
        return MiningEvent::Artifact;
    }
    MiningEvent::Nothing
}

impl World {
    /// Wrapper around the crate-private legacy LCG so `ugaris-server`
    /// glue for this system (e.g. `give_mine_item`'s 2%-chance-to-place-
    /// on-cursor roll, or the golem HP/loot-drop rolls) can keep drawing
    /// from the same canonical `World::legacy_random_seed` stream instead
    /// of a derived per-call hash (the pattern used by the chest family,
    /// which has no such shared-stream expectation in C).
    pub fn roll_legacy_random(&mut self, below: u32) -> u32 {
        legacy_random_below_from_seed(&mut self.legacy_random_seed, below)
    }

    /// C `handle_mining_result`'s cumulative-chance roll
    /// (`mine.c:229-275`): `RANDOM(100000)` against six event bands. See
    /// [`classify_mining_roll`] for the pure classification (split out so
    /// it can be tested with explicit roll values, without needing
    /// pre-computed LCG seeds).
    pub fn roll_mining_event(&mut self) -> MiningEvent {
        let roll = i64::from(self.roll_legacy_random(100_000));
        classify_mining_roll(roll, &self.settings)
    }

    /// C `handle_silver_find`'s amount roll (`mine.c:290-296`, minus the
    /// `give_mine_item`/`check_military_silver`/achievement tail, which
    /// need `ZoneLoader`/`PlayerRuntime` - see `ugaris-server::mine`).
    /// Note C has no `amount > 0` guard here (unlike the gold variant
    /// below) - silver is always granted, even a zero amount, matching
    /// C's unconditional `give_mine_item`/`check_military_silver`/
    /// `achievement_add_silver_mined` calls.
    pub fn roll_mining_silver_amount(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
    ) -> Option<i32> {
        self.roll_mining_stack_amount(item_id, 0, character_id)
    }

    /// C `handle_gold_find`'s amount roll (`mine.c:304-309`, same split
    /// as [`Self::roll_mining_silver_amount`]). Callers must replicate
    /// C's `if (amount > 0)` guard before granting/tracking.
    pub fn roll_mining_gold_amount(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
    ) -> Option<i32> {
        self.roll_mining_stack_amount(item_id, 1, character_id)
    }

    fn roll_mining_stack_amount(
        &mut self,
        item_id: ItemId,
        drdata_index: usize,
        character_id: CharacterId,
    ) -> Option<i32> {
        let base = i32::from(drdata(self.items.get(&item_id)?, drdata_index));
        let miner = i32::from(
            self.characters
                .get(&character_id)?
                .professions
                .get(profession::MINER)
                .copied()
                .unwrap_or_default()
                .max(0),
        );
        let span = (base.saturating_mul(2).saturating_add(1)).max(1) as u32;
        let mut amount = self.roll_legacy_random(span) as i32 + base;
        if miner != 0 {
            amount += amount * miner / 10;
        }
        Some(amount)
    }

    /// C `handle_golem_spawn`'s rare-vs-normal roll (`mine.c:320-326`):
    /// `RANDOM(get_rare_golem_chance()) == 0`.
    pub fn roll_mining_golem_rare(&mut self) -> bool {
        self.roll_legacy_random(self.settings.rare_golem_chance.max(1) as u32) == 0
    }

    /// C `calculate_drop_amount(level, is_rare)` (`mine.c:559-569`): the
    /// golem loot-drop quantity roll shared by `spawn_normal_golem`/
    /// `spawn_rare_golem`.
    pub fn calculate_golem_drop_amount(&mut self, level: i32, is_rare: bool) -> i32 {
        let base_min =
            (level / self.settings.level_divisor.max(1)) * self.settings.base_drop_multiplier;
        let base_max = base_min + self.settings.base_drop_multiplier;
        let span = (base_max - base_min + 1).max(1) as u32;
        let mut amount = base_min + self.roll_legacy_random(span) as i32;
        if is_rare {
            amount = (amount as f32 * self.settings.rare_drop_multiplier) as i32;
        }
        amount
    }

    /// C `handle_cave_in` (`mine.c:362-403`): rolls the miner's avoid
    /// chance, then (if not avoided) the wall-tier-scaled endurance loss,
    /// applying it directly to `Character::endurance`. Returns `None`
    /// only if `item_id`/`character_id` don't resolve (defensive; C's
    /// array indexing can't fail this way).
    pub fn apply_mining_cave_in(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
    ) -> Option<CaveInResult> {
        let miner = i32::from(
            self.characters
                .get(&character_id)?
                .professions
                .get(profession::MINER)
                .copied()
                .unwrap_or_default()
                .max(0),
        );
        if miner > 0 {
            let avoid_chance = miner * 2;
            let roll = self.roll_legacy_random(100) as i32;
            if roll < avoid_chance {
                return Some(CaveInResult::Avoided);
            }
        }

        let mine_level = i32::from(drdata(self.items.get(&item_id)?, 2)) * 10;
        let base_loss = (mine_level / 2) * POWERSCALE;
        let random_factor = (self.roll_legacy_random(50) as i32 + 75) as f32 / 100.0;
        let mut endurance_loss = (base_loss as f32 * random_factor) as i32;

        let athlete = i32::from(
            self.characters
                .get(&character_id)?
                .professions
                .get(profession::ATHLETE)
                .copied()
                .unwrap_or_default()
                .max(0),
        );
        let reduction_factor = if athlete > 0 {
            Some(1.0_f32 - (athlete as f32) * 0.02)
        } else {
            None
        };
        if let Some(factor) = reduction_factor {
            endurance_loss = (endurance_loss as f32 * factor) as i32;
        }

        let current_endurance = self.characters.get(&character_id)?.endurance;
        endurance_loss = endurance_loss.min(current_endurance);

        let character = self.characters.get_mut(&character_id)?;
        character.endurance -= endurance_loss;
        let now_exhausted = character.endurance < POWERSCALE;

        let unreduced_loss_units =
            reduction_factor.map(|factor| ((endurance_loss as f32 / factor) as i32) / POWERSCALE);

        Some(CaveInResult::Collapsed {
            endurance_loss_units: endurance_loss / POWERSCALE,
            unreduced_loss_units,
            now_exhausted,
        })
    }
}
