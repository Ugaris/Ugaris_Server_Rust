//! Area 33 (`src/area/33/tunnel.c`) reward math and creeper-dungeon
//! instance generation. C `give_reward` (`:527-601`), the
//! `IDR_TUNNELDOOR` exit-pillar payout called from `tunneldoor`'s
//! `DOOR_EXIT_EXP`/`DOOR_EXIT_MILITARY` branches (`:630-636`,
//! `item_driver::area33_tunnel::tunneldoor_driver`).
//!
//! `give_reward` reads/writes both `tunnel_ppd::used[]` and
//! `gorwin_ppd::tunnel_level` - `PlayerRuntime`-only state `World` cannot
//! see - so this follows the same snapshot-in/events-out split already
//! established by `world::npc::area33::gorwin` (`GorwinPlayerFacts`/
//! `GorwinOutcomeEvent`): [`TunnelRewardFacts`] is the caller's
//! (`ugaris-server`) pre-fetched snapshot, [`TunnelRewardOutcome`] is what
//! the caller still needs to write back to `PlayerRuntime` plus the
//! player-facing message lines. The `give_exp`/`give_military_pts_no_npc`
//! payout itself *is* applied directly here, since both are plain `World`
//! methods.
//!
//! [`World::plan_tunnel_entry`] ports `tunneldoor`'s `DOOR_ENTRY`/
//! `DOOR_CONTINUE` branches (`:638-734`) plus their `find_unused_sector`/
//! `handle_block_marker`/`handle_creeper_marker`/`update_exit_door`
//! helpers (`:451-525`): the "walk into the entrance/next-level column,
//! land in a freshly regenerated 31x127 creeper-dungeon instance" flow.
//! Unlike `create_maze` (`crate::dungeon_maze`), this isn't a from-scratch
//! grid generator - the instance's corridors are static, pre-built map
//! geometry (14 replicated `31x127` sectors baked into `zones/33/
//! tunnel.map`/`tunnel.itm`); C's `RANDOM()` calls only pick *which*
//! pre-placed `BLOCK_MARKER_1`/`BLOCK_MARKER_2` alternative opens (vs.
//! becomes a wall) and *which* `CREEPER_MARKER_1/2/3` spawns a creeper
//! (`CREEPER_MARKER_ALL` always does). `build_fighter` (`:325-449`) is
//! ported as [`tunnel_build_fighter_stat_values`] (the pure per-skill
//! stat formula - deliberately **not** shared with
//! `world::npc::area32::mission_start::build_fighter_stat_values`, since
//! tunnel.c's own switch groups skills differently, e.g. `V_HP` alone at
//! `diff-15` vs `V_ENDURANCE`/`V_MANA` at `diff-30`, where missions.c
//! groups all three together at `diff-15` - a real, deliberate C
//! difference between the two `build_fighter`s, not a typo) plus
//! [`TunnelCreeperSpawnSpec`] (the actual `create_char`/equip-item
//! instantiation needs `ZoneLoader`, so - mirroring `world::npc::area32::
//! mission_start`'s own `FighterSpawnSpec`/`spawn_mission_fighter` split -
//! happens in `ugaris-server`'s `dispatch_tunnel_enter_outcome::
//! spawn_tunnel_creeper`).

use super::*;
use crate::item_driver::{
    skill_raise_cost_factor, IDR_TUNNELDOOR, IID_TUNNELDOOR1, IID_TUNNELDOOR2, IID_TUNNELENEMY1,
    IID_TUNNELENEMY2, IID_TUNNELENEMY3, IID_TUNNELENEMYALL,
};
use crate::player::{find_next_available_tunnel_level, MAX_TUNNEL_USES, MIN_TUNNEL_LEVEL};

/// C `enum TunnelDoorType` (`src/area/33/tunnel.h:16`), duplicated from
/// `item_driver::area33_tunnel` (that module's consts are private to
/// `item_driver`, and pulling in the whole `item_driver` module here for
/// a handful of bytes isn't worth it).
const DOOR_ENTRY: u8 = 0;
const DOOR_EXIT_EXP: u8 = 2;
const DOOR_EXIT_MILITARY: u8 = 3;

/// C `enum BlockMarkerId`/`enum CreeperId` (`src/area/33/tunnel.h:18-25`):
/// the map-marker item template ids `tunneldoor`'s instance scan switches
/// on (`it[in2].ID`, i.e. [`crate::entity::Item::template_id`]).
const BLOCK_MARKER_1: u32 = IID_TUNNELDOOR1;
const BLOCK_MARKER_2: u32 = IID_TUNNELDOOR2;
const CREEPER_MARKER_1: u32 = IID_TUNNELENEMY1;
const CREEPER_MARKER_2: u32 = IID_TUNNELENEMY2;
const CREEPER_MARKER_3: u32 = IID_TUNNELENEMY3;
const CREEPER_MARKER_ALL: u32 = IID_TUNNELENEMYALL;

/// C `#define CREEPER_TAB_SIZE 191` (`tunnel.h:31`).
const CREEPER_TAB_SIZE: usize = 191;

/// C's blocked-visual wall sprite (`tunnel.c:465`/`:712`/`open_door`'s own
/// `59791` isn't reused there, but every other closed-cell sprite in this
/// file is this literal).
const TUNNEL_WALL_SPRITE: u32 = 59791;

/// C `static int creeper_tab[CREEPER_TAB_SIZE]` (`tunnel.c:85-94`):
/// per-level creeper difficulty, indexed by `level - MIN_TUNNEL_LEVEL`
/// (so `creeper_tab[0]` is level 10's difficulty, `creeper_tab[190]` is
/// level 200's).
#[rustfmt::skip]
const CREEPER_TAB: [i32; CREEPER_TAB_SIZE] = [
    13,  15,  16,  18,  19,  20,  22,  23,  25,  26,  28,  29,  30,  31,  33,  34,  36,  37,  39,  40,  41,  42,
    44,  45,  46,  48,  49,  50,  51,  53,  54,  55,  57,  58,  59,  60,  61,  62,  64,  65,  66,  68,  69,  70,
    71,  72,  73,  75,  76,  77,  79,  80,  81,  82,  83,  84,  86,  87,  88,  89,  90,  91,  92,  94,  95,  96,
    97,  99,  100, 101, 102, 103, 104, 105, 107, 108, 109, 110, 111, 112, 113, 115, 116, 117, 118, 120, 121, 122,
    123, 124, 125, 126, 127, 129, 130, 131, 132, 133, 134, 135, 137, 138, 139, 140, 141, 142, 143, 145, 146, 147,
    148, 149, 150, 151, 152, 154, 155, 156, 157, 158, 160, 161, 162, 163, 164, 165, 166, 168, 169, 170, 171, 172,
    173, 174, 175, 177, 178, 179, 180, 181, 182, 183, 184, 186, 187, 188, 189, 190, 191, 192, 193, 195, 196, 197,
    198, 199, 200, 201, 202, 204, 205, 206, 207, 208, 210, 211, 212, 213, 214, 215, 216, 217, 219, 220, 221, 222,
    223, 224, 225, 226, 228, 229, 230, 231, 232, 233, 234, 235, 237, 238, 239,
];

/// `creeper_tab[level - MIN_TUNNEL_LEVEL]` (`tunnel.c:472`), clamped to
/// the table's own bounds - every real call site already keeps `level`
/// within `MIN_TUNNEL_LEVEL..=MAX_TUNNEL_LEVEL` (`CREEPER_TAB_SIZE` is
/// exactly that range's width), so the clamp is a defensive fallback
/// only, never a real C behavior difference.
fn creeper_difficulty(level: i32) -> i32 {
    let idx = (level - MIN_TUNNEL_LEVEL).clamp(0, CREEPER_TAB_SIZE as i32 - 1) as usize;
    CREEPER_TAB[idx]
}

/// C `DOOR_RANGE`/`DOOR_DEPTH` (`src/area/33/tunnel.h:31-32`), duplicated
/// from `item_driver::area33_tunnel` for the same reason as
/// [`DOOR_EXIT_EXP`] - `check_area_clear` needs `self.map`/
/// `self.characters`, which only `World` (this module) has access to.
const DOOR_RANGE: u16 = 4;
const DOOR_DEPTH: u16 = 20;

/// Snapshot of the `PlayerRuntime` fields C `give_reward` reads, matching
/// `world::npc::area33::gorwin::GorwinPlayerFacts`'s shape.
#[derive(Debug, Clone)]
pub struct TunnelRewardFacts {
    /// `gorwin_ppd::tunnel_level` (`PlayerRuntime::gorwin_tunnel_level`).
    pub reward_level: i32,
    /// `tunnel_ppd::used[]` (`PlayerRuntime::tunnel_used`), indexed by
    /// level directly, same shape as `GorwinPlayerFacts::tunnel_used`.
    pub tunnel_used: Vec<u8>,
}

impl TunnelRewardFacts {
    fn used_at(&self, level: i32) -> u8 {
        if level < 0 {
            return 0;
        }
        self.tunnel_used.get(level as usize).copied().unwrap_or(0)
    }
}

/// What [`World::apply_tunnel_reward`] could not apply directly, for
/// `ugaris-server` to finish (`PlayerRuntime` writes, player-facing
/// feedback delivery, and the `achievement_add_tunnel_level` DB/unlock
/// wiring `World` has no access to).
#[derive(Debug, Clone, Default)]
pub struct TunnelRewardOutcome {
    /// C `log_char(cn, LOG_SYSTEM, 0, ...)` lines, in call order. Color
    /// markers (`COL_HEADING`/`COL_YELLOW`/`COL_RESET`) around "Tunnel
    /// Mastery!"/the promoted level number are dropped - this outcome's
    /// plain `String` messages have no raw-byte counterpart to carry them
    /// in, matching the `dispatch_minewall_outcome`/`dispatch_lab_outcome`
    /// precedent for plain-text item-use feedback (documented deviation,
    /// same family as `WorldAreaText.message`'s).
    pub messages: Vec<String>,
    /// `PlayerRuntime::set_tunnel_used(level, value)` to apply, if the
    /// reward was actually granted (`used[reward_level] < MAX_TUNNEL_USES`
    /// on entry).
    pub new_used_count: Option<(i32, u8)>,
    /// `PlayerRuntime::set_gorwin_tunnel_level(next)` to apply, on either
    /// of C's two auto-promote branches (all-uses-exhausted-just-now, or
    /// already-maxed-on-entry).
    pub promote_gorwin_to: Option<i32>,
    /// Whether `achievement_add_tunnel_level(cn)` should fire (only when a
    /// reward was actually granted).
    pub award_achievement: bool,
}

impl World {
    /// C `give_reward` (`src/area/33/tunnel.c:527-601`). `door_type` is
    /// the raw `it[in].drdata[0]` (`DOOR_EXIT_EXP` or `DOOR_EXIT_MILITARY`
    /// - any other value is a no-op, matching C's `if/else if` with no
    /// `else` branch).
    pub fn apply_tunnel_reward(
        &mut self,
        character_id: CharacterId,
        facts: &TunnelRewardFacts,
        door_type: u8,
        area_id: u32,
    ) -> TunnelRewardOutcome {
        let mut outcome = TunnelRewardOutcome::default();
        let reward_level = facts.reward_level;
        let char_level = self
            .characters
            .get(&character_id)
            .map(|character| character.level as i32)
            .unwrap_or(0);
        let used_before = facts.used_at(reward_level);

        if used_before < MAX_TUNNEL_USES {
            // C `ppd->used[reward_level]++;` (`:540`) - the reward-value
            // formulas below read the *post*-increment count.
            let used_after = used_before + 1;
            outcome.new_used_count = Some((reward_level, used_after));

            if door_type == DOOR_EXIT_EXP {
                // C `value = level_value(reward_level) /
                // tunnel_exp_base_value_divider / (ppd->used[reward_level]
                // + 9);` (`:543`) - fully double-precision until the
                // final `(int)value` assignment.
                let divider = self.settings.tunnel_exp_base_value_divider;
                let raw = f64::from(level_value(reward_level.max(0) as u32)) / divider;
                let value = (raw / (f64::from(used_after) + 9.0)) as i64;
                outcome
                    .messages
                    .push("You have been given experience.".to_string());
                self.give_exp(character_id, value, area_id);
            } else {
                // C `value = (tunnel_mill_exp_base_value + (reward_level *
                // reward_level / 10)) / (ppd->used[reward_level] + 9);`
                // (`:550`) - all-integer.
                let base = self.settings.tunnel_mill_exp_base_value;
                let value =
                    (base + (reward_level * reward_level / 10)) / (i32::from(used_after) + 9);
                outcome
                    .messages
                    .push("You have been given military rank.".to_string());
                self.give_military_pts(character_id, value, 1, area_id);
            }
            outcome.award_achievement = true;

            // C `if (ppd->used[reward_level] >= MAX_TUNNEL_USES) { ... }
            // else { ... }` (`:560-586`).
            if used_after >= MAX_TUNNEL_USES {
                outcome.messages.push(format!(
                    "Tunnel Mastery! Thou hast conquered all {MAX_TUNNEL_USES} challenges at level {reward_level}."
                ));
                match find_next_available_tunnel_level(&facts.tunnel_used, reward_level, char_level)
                {
                    Some(next) => {
                        outcome.promote_gorwin_to = Some(next);
                        outcome.messages.push(format!(
                            "Gorwin has advanced thy tunnel level to {next}. Onward and upward!"
                        ));
                    }
                    None => {
                        outcome.messages.push(
                            "There are no more tunnel levels available to thee. Thou art a true master of the depths!"
                                .to_string(),
                        );
                    }
                }
            } else {
                let remaining = MAX_TUNNEL_USES - used_after;
                outcome.messages.push(format!(
                    "Completions at level {reward_level}: {used_after}/{MAX_TUNNEL_USES} ({remaining} remaining)."
                ));
            }
        } else {
            // C `else { log_char(...); int next_level = ...; if
            // (next_level) { ... } }` (`:587-599`).
            outcome.messages.push(format!(
                "You have used all {MAX_TUNNEL_USES} completions at level {reward_level}. No reward given."
            ));
            if let Some(next) =
                find_next_available_tunnel_level(&facts.tunnel_used, reward_level, char_level)
            {
                outcome.promote_gorwin_to = Some(next);
                outcome.messages.push(format!(
                    "Gorwin has advanced thy tunnel level to {next}. Speak with him for details."
                ));
            }
        }

        outcome
    }

    /// C `check_area_clear(in)` (`src/area/33/tunnel.c:750-762`): scans the
    /// `DOOR_RANGE`-wide, `DOOR_DEPTH`-deep rectangle in front of a
    /// `IDR_TUNNELDOOR2` "mean door" (`x` ± `DOOR_RANGE`, `y+1` through
    /// `y+DOOR_DEPTH-1`) for any non-player character. Out-of-bounds tiles
    /// are skipped (C's raw `map[x+y*MAXMAP]` indexing has no such bounds
    /// check, but every real door placement keeps this rectangle on-map).
    pub(crate) fn tunnel_mean_door_area_clear(&self, x: u16, y: u16) -> bool {
        let x_start = x.saturating_sub(DOOR_RANGE);
        let x_end = x.saturating_add(DOOR_RANGE);
        let y_start = y.saturating_add(1);
        let y_end = y.saturating_add(DOOR_DEPTH);
        for ty in y_start..y_end {
            for tx in x_start..=x_end {
                let Some(tile) = self.map.tile(usize::from(tx), usize::from(ty)) else {
                    continue;
                };
                if tile.character == 0 {
                    continue;
                }
                let occupant_is_player = self
                    .characters
                    .get(&CharacterId(u32::from(tile.character)))
                    .is_some_and(|character| character.flags.contains(CharacterFlags::PLAYER));
                if !occupant_is_player {
                    return false;
                }
            }
        }
        true
    }

    /// C `find_unused_sector` (`src/area/33/tunnel.c:488-512`): scans the
    /// 7x2 grid of `31x127`-tile creeper-dungeon instance sectors (`xoff`
    /// in `{1,32,...,187}` step `31`, `yoff` in `{1,128}` step `127` - C's
    /// own loop bounds, kept literal rather than hardcoded so the `(218,
    /// 128)` skip - dead code today, since `xoff` never actually reaches
    /// `218` before its `<210` loop guard fails - stays visibly
    /// C-faithful) for the first one with no *other* player character
    /// standing in it (`exclude` is the entering player, matching C's
    /// `co != cn`).
    pub(crate) fn find_unused_tunnel_sector(&self, exclude: CharacterId) -> Option<(u16, u16)> {
        let mut xoff = 1u16;
        while xoff < 210 {
            let mut yoff = 1u16;
            while yoff < 255 {
                if !(xoff == 218 && yoff == 128) {
                    let mut used = false;
                    'scan: for x in (1 + xoff)..(32 + xoff) {
                        for y in (1 + yoff)..(128 + yoff) {
                            let Some(tile) = self.map.tile(usize::from(x), usize::from(y)) else {
                                continue;
                            };
                            if tile.character == 0 {
                                continue;
                            }
                            let occupant = CharacterId(u32::from(tile.character));
                            if occupant != exclude
                                && self.characters.get(&occupant).is_some_and(|character| {
                                    character.flags.contains(CharacterFlags::PLAYER)
                                })
                            {
                                used = true;
                                break 'scan;
                            }
                        }
                    }
                    if !used {
                        return Some((xoff, yoff));
                    }
                }
                yoff += 127;
            }
            xoff += 31;
        }
        None
    }

    /// C `tunneldoor`'s `DOOR_ENTRY`/`DOOR_CONTINUE` map-regeneration
    /// pass (`src/area/33/tunnel.c:667-733`), run after `ugaris-server`'s
    /// `dispatch_tunnel_enter_outcome` has already computed `clevel`
    /// (`ppd->clevel`, C's own `:640`/`:658-664`) and persisted it.
    /// `used_at_clevel` is `ppd->used[ppd->clevel]` (`PlayerRuntime`-only
    /// state `World` cannot see), needed for [`Self::
    /// update_tunnel_exit_door_text`]'s "Column N, used M times" pillar
    /// label. Returns `None` when every instance sector is occupied by
    /// another player (C's "All tunnels are busy" refusal, `:668`) -
    /// note C already mutated `ppd->clevel` *before* this check, which
    /// `dispatch_tunnel_enter_outcome` mirrors by persisting `clevel`
    /// before calling this.
    pub fn plan_tunnel_entry(
        &mut self,
        character_id: CharacterId,
        clevel: i32,
        door_type: u8,
        used_at_clevel: u8,
    ) -> Option<TunnelEntryPlan> {
        let (xoff, yoff) = self.find_unused_tunnel_sector(character_id)?;

        // C `srand(ch[cn].ID * ppd->clevel);` (`:673`) - a fresh, fully
        // reseeded local sequence (not `self.legacy_random_seed`), same
        // "seeded-LCG shape, not a bit-exact libc PRNG port" precedent as
        // `crate::dungeon_maze`.
        let serial = self.characters.get(&character_id)?.serial;
        let mut seed = serial.wrapping_mul(clevel as u32);
        let b1 = legacy_random_below_from_seed(&mut seed, 3);
        let b2 = legacy_random_below_from_seed(&mut seed, 2);
        let c = legacy_random_below_from_seed(&mut seed, 3);

        let diff = creeper_difficulty(clevel);
        let mut b1_count = 0u32;
        let mut b2_count = 0u32;
        let mut creepers = Vec::new();

        for x in (1 + xoff)..(32 + xoff) {
            for y in (1 + yoff)..(128 + yoff) {
                let (ux, uy) = (usize::from(x), usize::from(y));

                // C: `if ((co = map[m].ch) && !(ch[co].flags &
                // CF_PLAYER)) remove_destroy_char(co);` (`:686-688`).
                let occupant = self.map.tile(ux, uy).map_or(0, |tile| tile.character);
                if occupant != 0 {
                    let occupant_id = CharacterId(u32::from(occupant));
                    let is_player = self
                        .characters
                        .get(&occupant_id)
                        .is_some_and(|character| character.flags.contains(CharacterFlags::PLAYER));
                    if !is_player {
                        self.remove_and_destroy_tunnel_character(occupant_id);
                    }
                }

                let item_id = self
                    .map
                    .tile(ux, uy)
                    .and_then(|tile| (tile.item != 0).then_some(ItemId(tile.item)));
                if let Some(item_id) = item_id {
                    if let Some(item) = self.items.get(&item_id) {
                        let template_id = item.template_id;
                        let driver = item.driver;
                        let door_drdata0 = item.driver_data.first().copied().unwrap_or(0);

                        if template_id == BLOCK_MARKER_1 {
                            self.resolve_tunnel_block_marker(item_id, x, y, b1_count == b1);
                            b1_count += 1;
                        } else if template_id == BLOCK_MARKER_2 {
                            self.resolve_tunnel_block_marker(item_id, x, y, b2_count == b2);
                            b2_count += 1;
                        } else if matches!(
                            template_id,
                            CREEPER_MARKER_1
                                | CREEPER_MARKER_2
                                | CREEPER_MARKER_3
                                | CREEPER_MARKER_ALL
                        ) {
                            // C `(marker_id == CREEPER_MARKER_ALL) ||
                            // (marker_id - CREEPER_MARKER_1 ==
                            // chosen_creeper)` (`:471`).
                            let chosen = template_id == CREEPER_MARKER_ALL
                                || template_id.wrapping_sub(CREEPER_MARKER_1) == c;
                            if chosen {
                                creepers.push(TunnelCreeperSpawnSpec {
                                    x,
                                    y,
                                    diff,
                                    level: clevel,
                                });
                            }
                            self.clear_tunnel_marker(item_id, x, y);
                        } else if driver == IDR_TUNNELDOOR
                            && matches!(door_drdata0, DOOR_EXIT_EXP | DOOR_EXIT_MILITARY)
                        {
                            self.update_tunnel_exit_door_text(item_id, clevel, used_at_clevel);
                        } else if driver == IDR_TUNNELDOOR2 {
                            self.block_tunnel_mean_door(item_id, x, y);
                        }
                    }
                }

                // C: `if (map[m].ch && map[m].fsprite != 0) update_map_
                // cell(m, 0, 0);` (`:719-722`).
                if let Some(tile) = self.map.tile_mut(ux, uy) {
                    if tile.character != 0 && tile.foreground_sprite != 0 {
                        tile.foreground_sprite = 0;
                        tile.flags
                            .remove(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK);
                        self.mark_dirty_sector(ux, uy);
                    }
                }
            }
        }

        // C `if (door_type == DOOR_ENTRY) ch[cn].hp = value[0][V_HP] *
        // POWERSCALE; else ch[cn].hp = min(value[0][V_HP] * POWERSCALE,
        // hp + value[1][V_HP] * POWERSCALE / 2);` (`:727-731`).
        if let Some(character) = self.characters.get_mut(&character_id) {
            let max_hp = i32::from(character.values[0][CharacterValue::Hp as usize]) * POWERSCALE;
            character.hp = if door_type == DOOR_ENTRY {
                max_hp
            } else {
                let regen =
                    i32::from(character.values[1][CharacterValue::Hp as usize]) * POWERSCALE / 2;
                max_hp.min(character.hp + regen)
            };
        }

        // C `teleport_char_driver(cn, 16 + xoff, 123 + yoff);` (`:733`).
        self.teleport_char_driver(character_id, 16 + xoff, 123 + yoff);

        Some(TunnelEntryPlan { creepers })
    }

    /// C `handle_block_marker` (`src/area/33/tunnel.c:460-467`): hides the
    /// marker item itself (`it[in2].sprite = 0`) and either opens the
    /// cell (`open == true`, the marker matching this instance's chosen
    /// "real" path) or turns it into a solid wall (`TUNNEL_WALL_SPRITE`,
    /// `MF_MOVEBLOCK`/`MF_SIGHTBLOCK` ported as `TMOVEBLOCK`/
    /// `TSIGHTBLOCK` - see [`Self::block_tunnel_mean_door`]'s doc comment
    /// for why).
    fn resolve_tunnel_block_marker(&mut self, item_id: ItemId, x: u16, y: u16, open: bool) {
        if let Some(item) = self.items.get_mut(&item_id) {
            item.sprite = 0;
        }
        let (ux, uy) = (usize::from(x), usize::from(y));
        if let Some(tile) = self.map.tile_mut(ux, uy) {
            if open {
                tile.foreground_sprite = 0;
                tile.flags
                    .remove(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK);
            } else {
                tile.foreground_sprite = TUNNEL_WALL_SPRITE;
                tile.flags
                    .insert(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK);
            }
        }
        self.mark_dirty_sector(ux, uy);
    }

    /// C `handle_creeper_marker`'s own unconditional trailing
    /// `update_map_cell(m, 0, 0)` (`tunnel.c:480`): every creeper marker
    /// cell ends up open/unblocked regardless of whether it actually
    /// spawned a creeper (creeper markers never block movement - only
    /// [`Self::resolve_tunnel_block_marker`]'s wall cells do).
    fn clear_tunnel_marker(&mut self, item_id: ItemId, x: u16, y: u16) {
        if let Some(item) = self.items.get_mut(&item_id) {
            item.sprite = 0;
        }
        let (ux, uy) = (usize::from(x), usize::from(y));
        if let Some(tile) = self.map.tile_mut(ux, uy) {
            tile.foreground_sprite = 0;
            tile.flags
                .remove(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK);
        }
        self.mark_dirty_sector(ux, uy);
    }

    /// C `update_exit_door` (`src/area/33/tunnel.c:484-486`): relabels an
    /// `IDR_TUNNELDOOR` exit pillar encountered mid-scan with the
    /// player's completion count at the level they're about to play.
    fn update_tunnel_exit_door_text(&mut self, item_id: ItemId, level: i32, uses: u8) {
        if let Some(item) = self.items.get_mut(&item_id) {
            item.name = format!("Column {level}, used {uses} times");
        }
    }

    /// C's `default:` scan-loop arm for an `IDR_TUNNELDOOR2` "mean door"
    /// encountered mid-scan (`tunnel.c:710-713`): always reset back to
    /// closed (hidden sprite item-side, solid wall map-side) whenever the
    /// instance regenerates, regardless of whatever open/closed state
    /// [`Self::tunnel_mean_door_area_clear`]'s own periodic check had
    /// last left it in. Uses `TMOVEBLOCK`/`TSIGHTBLOCK` rather than the
    /// literal C `MF_MOVEBLOCK`/`MF_SIGHTBLOCK` bits, matching this same
    /// door family's already-ported `TunnelDoorAreaCheck` open-door arm
    /// (`world::item_outcomes`) for consistency within this one feature -
    /// a documented, deliberate deviation (both bit-pairs are load-
    /// bearing for `path::pathfinder`'s blocking checks either way).
    fn block_tunnel_mean_door(&mut self, item_id: ItemId, x: u16, y: u16) {
        if let Some(item) = self.items.get_mut(&item_id) {
            item.sprite = 0;
        }
        let (ux, uy) = (usize::from(x), usize::from(y));
        if let Some(tile) = self.map.tile_mut(ux, uy) {
            tile.foreground_sprite = TUNNEL_WALL_SPRITE;
            tile.flags
                .insert(MapFlags::TMOVEBLOCK | MapFlags::TSIGHTBLOCK);
        }
        self.mark_dirty_sector(ux, uy);
    }

    /// C `remove_destroy_char(co)` (`tunnel.c:687`), same shape as
    /// `world::npc::area32::mission_start`'s own private
    /// `remove_and_destroy_character` (module-private there, so
    /// duplicated here rather than reused - same precedent as this file's
    /// other small duplicated helpers, see the module doc comment).
    fn remove_and_destroy_tunnel_character(&mut self, character_id: CharacterId) {
        let carried: Vec<ItemId> = self
            .characters
            .get(&character_id)
            .map(|character| {
                character
                    .inventory
                    .iter()
                    .flatten()
                    .copied()
                    .chain(character.cursor_item)
                    .collect()
            })
            .unwrap_or_default();
        for item_id in carried {
            self.destroy_item(item_id);
        }
        self.remove_character(character_id);
    }
}

/// One creeper [`World::plan_tunnel_entry`] wants `ugaris-server`'s
/// `dispatch_tunnel_enter_outcome::spawn_tunnel_creeper` to instantiate
/// (C `build_fighter(x, y, creeper_tab[level - MIN_TUNNEL_LEVEL], level)`,
/// called from `handle_creeper_marker`, `tunnel.c:472`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TunnelCreeperSpawnSpec {
    pub x: u16,
    pub y: u16,
    /// `creeper_tab[level - MIN_TUNNEL_LEVEL]` (C's own `diff` parameter).
    pub diff: i32,
    /// The tunnel's `clevel` (C's own `level` parameter) - `build_fighter`
    /// sets `ch[cn].level` to this directly, *not* derived from `exp2level`.
    pub level: i32,
}

/// Return value of [`World::plan_tunnel_entry`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TunnelEntryPlan {
    pub creepers: Vec<TunnelCreeperSpawnSpec>,
}

/// C `build_fighter`'s per-skill stat formula (`tunnel.c:334-393`),
/// applied only where `markers[v] != 0` (the freshly-instantiated
/// `tunnel_creeper` template's own bare `value[1]`) **and**
/// [`skill_raise_cost_factor`]`(v) != 0` - C's own `for (n = 0; n < V_MAX;
/// n++) { if (!skill[n].cost || !ch[cn].value[1][n]) continue; ... }`
/// loop. Deliberately a separate function from `world::npc::area32::
/// mission_start::build_fighter_stat_values` - see this module's doc
/// comment for why the two C `build_fighter`s group skills differently.
pub fn tunnel_build_fighter_stat_values(markers: &[i16], diff: i32) -> Vec<i16> {
    markers
        .iter()
        .enumerate()
        .map(|(index, &marker)| {
            if marker == 0 || skill_raise_cost_factor(index) == 0 {
                marker
            } else {
                tunnel_build_fighter_stat_value(index, diff)
            }
        })
        .collect()
}

fn tunnel_build_fighter_stat_value(index: usize, diff: i32) -> i16 {
    use crate::entity::CharacterValue as V;
    let val = if index == V::Hp as usize {
        (diff - 15).max(10)
    } else if index == V::Endurance as usize || index == V::Mana as usize {
        (diff - 30).max(10)
    } else if index == V::Wisdom as usize {
        (diff - 25).max(10)
    } else if index == V::Intelligence as usize
        || index == V::Agility as usize
        || index == V::Strength as usize
    {
        (diff - 5).max(10)
    } else if index == V::Hand as usize
        || index == V::Attack as usize
        || index == V::Parry as usize
        || index == V::Immunity as usize
    {
        diff.max(1)
    } else if index == V::ArmorSkill as usize {
        ((diff / 10) * 10).max(1)
    } else if index == V::Tactics as usize || index == V::SpeedSkill as usize {
        (diff - 5).max(1)
    } else if index == V::Warcry as usize {
        (diff - 15).max(1)
    } else if index == V::Surround as usize || index == V::BodyControl as usize {
        (diff - 20).max(1)
    } else if index == V::Percept as usize {
        (diff - 10).max(1)
    } else if index == V::Bless as usize
        || index == V::Fireball as usize
        || index == V::MagicShield as usize
    {
        (diff - 5).max(1)
    } else if index == V::Freeze as usize {
        diff.max(1)
    } else {
        (diff - 30).max(1)
    };
    val.min(250) as i16
}
