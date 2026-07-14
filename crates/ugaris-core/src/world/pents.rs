//! Pentagram quest system-wide state (`src/area/4/pents.c`'s file-static
//! globals: `init_done`, `solve_serial`, `total_pentagrams`/
//! `active_pentagrams`/`required_activations`, `power_levels[MAXLEVEL]`,
//! `area_pentagram_counts`/`area_activated_counts`/`area_serials`,
//! `training_power`, `last_solve_time`, `pentagram_record*`). One instance
//! per `World` matches C's one-area-process-per-file-static architecture
//! (see `World::area_id`'s doc comment).
//!
//! Everything here is *system-wide* (no per-player data): item mutation
//! (`activate_pentagram`/`deactivate_pentagram`) and the solve-threshold
//! bookkeeping (`check_for_quest_completion`/`complete_pentagram_quest`'s
//! `update_power_levels` half). The *per-player* half of C's pipeline
//! (`add_pentagram_to_player`/`distribute_rewards_to_player`, which read
//! and mutate `struct pentagram_player_data` - `PlayerRuntime::
//! pentagram_debug` in this port) cannot live here: `World` has no access
//! to the session-owned `PlayerRuntime` (same architectural constraint as
//! `pending_lostcon_hurt_events`, see that field's doc comment in
//! `world/mod.rs`). Every activation is queued as a
//! [`PentagramActivationEvent`] for `ugaris-server`'s `pents` module to
//! drain and apply the per-player half using the pure helpers in
//! `ugaris_core::pentagram`.
//!
//! Demon spawning (`spawn_demons_at_pentagram`/`enhance_elite_demon`/
//! `adjust_lesser_demon`/`enhance_demon_character`/`update_demon_
//! profession`/`handle_demon_death`) *is* ported here: `World` plans each
//! spawn (slot bookkeeping in the pentagram item's `driver_data[6..]`,
//! demon-type roll, template name) and queues a
//! [`PentagramDemonSpawnRequest`] for `ugaris-server` to actually
//! instantiate from a `penterN` zone template (needs `ZoneLoader`, which
//! `World` doesn't have - same split as the edemon/fdemon gate spawns in
//! `world/spawn.rs`), then calls back [`World::finish_pentagram_demon_
//! spawn`]/[`World::apply_pentagram_spawn_result`] to apply the elite/
//! lesser stat mutation and record the spawned demon's slot. Spawned
//! demons get `CDR_PENTER` (`character_driver::CDR_PENTER`), whose own
//! per-tick AI is the `CDR_SIMPLEBADDY` self-defense/idle-wander driver
//! reused wholesale (see the `character.driver == CDR_SIMPLEBADDY` gates
//! widened alongside `CDR_DUNGEONFIGHTER` in `world/npc_fight.rs`/
//! `world/npc_idle.rs`, and `zone.rs`'s `CDR_PENTER` branch).
//!
//! `pentagram_record`/`pentagram_record_holder`'s restart-persistence
//! (C `load_pentagram_record`/`save_pentagram_record_scheduled`,
//! `src/system/database/database_pent_record.c`) lives in
//! `ugaris-db`'s `PgPentagramRecordRepository` plus `ugaris-server`'s
//! `pents::save_pentagram_record_if_dirty` - `World` itself has no
//! database access (same split as every other DB-backed registry).
//!
//! Not ported here (tracked as a `PORTING_TODO.md` "Area 4" REMAINING
//! gap): the `CDR_TESTER` QA-bot character driver (`pentagram_tester_
//! driver`, a test-only helper bot - lowest priority, not player-facing)
//! and the macro-daemon challenge-room `saved_pent_*` restore
//! (`crate::macro_daemon::macro_save_pentagram_progress`'s own doc
//! comment already documents this as a no-op pending this task).

use super::*;

/// C `#define MAXLEVEL 56`.
pub const PENT_MAX_LEVEL: usize = 56;

/// C `struct pent_debug_data`-adjacent file statics tracking the shared
/// pentagram-quest solve state for this area. See the module doc comment
/// for what is/isn't ported.
#[derive(Debug, Clone)]
pub struct PentagramQuestState {
    pub initialized: bool,
    pub total_pentagrams: i32,
    pub active_pentagrams: i32,
    pub required_activations: i32,
    /// C `static int solve_serial = 255`.
    pub solve_serial: i32,
    pub min_level: i32,
    pub max_level: i32,
    /// C `static int power_levels[MAXLEVEL]`. Indexed by pentagram level
    /// (`it[item_id].drdata[0]`, `0..PENT_MAX_LEVEL`) when raised by a
    /// solve (`update_power_levels`); C also indexes it by a *different*
    /// space (demon-class index) on a demon-lord death
    /// (`handle_demon_death`) - that consumer isn't ported (demon
    /// spawning isn't ported), so only the solve-side writer exists here.
    pub power_levels: [i32; PENT_MAX_LEVEL],
    /// C `static int area_pentagram_counts[MAXLEVEL + 1]`.
    pub area_pentagram_counts: [i32; PENT_MAX_LEVEL + 1],
    /// C `static int area_activated_counts[MAXLEVEL + 1]`.
    pub area_activated_counts: [i32; PENT_MAX_LEVEL + 1],
    /// C `static int area_serials[MAXLEVEL + 1] = {255, ...}`.
    pub area_serials: [i32; PENT_MAX_LEVEL + 1],
    pub training_power: i32,
    /// C `static int last_solve_time = 0` (a `ticker` snapshot), read by
    /// `handle_demon_lord_door`'s access-time gate. Wired into
    /// [`ItemDriverContext::pent_last_solve_tick`] by
    /// `World::execute_item_driver_request_with_context`.
    pub last_solve_tick: Option<u32>,
    /// C `static int pentagram_record = 0`/`pentagram_record_ID`. Unlike
    /// C (which compares `ch[player_id].ID`, a persistent save-file
    /// identity this Rust port has no equivalent handle for outside
    /// `PlayerRuntime`), record ownership is tracked by character name
    /// (matching `World::arena_toplist`'s own name-keyed precedent) - see
    /// [`Self::pentagram_record_holder`].
    pub pentagram_record: i32,
    /// C `static char pentagram_record_holder[40] = {"Nobody"}`.
    pub pentagram_record_holder: String,
}

impl Default for PentagramQuestState {
    fn default() -> Self {
        Self {
            initialized: false,
            total_pentagrams: 0,
            active_pentagrams: 0,
            // C `static int ... required_activations = 12` (solve_base's
            // default value, before the first `calculate_required_
            // pentagrams` call).
            required_activations: 12,
            solve_serial: 255,
            min_level: 0,
            max_level: 0,
            power_levels: [0; PENT_MAX_LEVEL],
            area_pentagram_counts: [0; PENT_MAX_LEVEL + 1],
            area_activated_counts: [0; PENT_MAX_LEVEL + 1],
            area_serials: [255; PENT_MAX_LEVEL + 1],
            training_power: 0,
            last_solve_tick: None,
            pentagram_record: 0,
            pentagram_record_holder: "Nobody".to_string(),
        }
    }
}

/// C globals `solve_base`/`solve_multiplier`/`solve_random_multiplier`
/// (`pents.c:75-77`) - plain mutable ints in C with no `get_*` tuning
/// wrapper (unlike the `GameSettings`-backed constants this file also
/// uses), so ported as plain constants rather than `GameSettings` fields.
const SOLVE_BASE: i32 = 12;
const SOLVE_MULTIPLIER: i32 = 4;
const SOLVE_RANDOM_MULTIPLIER: i32 = 7;

/// C `int elite_demon_spawn_chance = 100;` (`pents.c:70`, `1 in 100`).
const ELITE_DEMON_SPAWN_CHANCE: u32 = 100;
/// C `int lesser_demon_spawn_chance = 10;` (`pents.c:71`, `1 in 10`).
const LESSER_DEMON_SPAWN_CHANCE: u32 = 10;
/// C `float elite_demon_stat_multiplier = 1.2;` (`pents.c:72`).
const ELITE_DEMON_STAT_MULTIPLIER: f64 = 1.2;
/// C `float lesser_demon_stat_multiplier = 0.8;` (`pents.c:73`).
const LESSER_DEMON_STAT_MULTIPLIER: f64 = 0.8;

/// C `LESSER_DEMON_CLASS_BASE`/`ELITE_DEMON_CLASS_BASE`
/// (`pents.h:24-25`), re-exported from the one place this codebase
/// already defines them (`world::npc::area32::military::missions`, which
/// needs the same constants for its own mission-kill-counting class
/// ranges).
pub use crate::world::npc::area32::military::{ELITE_DEMON_CLASS_BASE, LESSER_DEMON_CLASS_BASE};

/// C `static const char *elite_demon_names[]` (`pents.c:98-119`): unique
/// Babylonian demonic names for elite demons.
const ELITE_DEMON_NAMES: [&str; 20] = [
    "Pazuzu",
    "Lamashtu",
    "Namtar",
    "Utukku",
    "Asag",
    "Humbaba",
    "Kingu",
    "Mushussu",
    "Gallu",
    "Alal",
    "Nergal",
    "Ereshkigal",
    "Apkallu",
    "Edimmu",
    "Sebitti",
    "Irkalla",
    "Asakku",
    "Rabisu",
    "Labartu",
    "Lilitu",
];

/// C `demon_type` local (`pents.c:1007`, `spawn_demons_at_pentagram`):
/// `2` = elite, `0` = lesser, `1` = normal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemonType {
    Lesser,
    Normal,
    Elite,
}

/// One planned demon spawn slot queued for `ugaris-server` to instantiate
/// from a `penterN` zone template - see the module doc comment for why
/// `World` can't do this itself (needs `ZoneLoader`).
#[derive(Debug, Clone)]
pub struct PentagramDemonSpawnRequest {
    pub item_id: ItemId,
    /// C `spawn_count`: how many demons `spawn_demons_at_pentagram` should
    /// try to place (bounded by [`World::pentagram_max_spawns`] and empty/
    /// stale slots - see [`World::pentagram_spawn_slot_is_stale`]).
    pub spawn_count: i32,
    pub level: i32,
}

/// C `P_DEMON` (`PROFESSION_NAMES[11] == "Demon"`).
pub(crate) const P_DEMON: usize = 11;

/// Facts about a pentagram activation queued for `ugaris-server` to apply
/// the per-player half of C's pipeline - see the module doc comment for
/// why `World` can't do this itself.
#[derive(Debug, Clone, Copy)]
pub struct PentagramActivationEvent {
    pub item_id: ItemId,
    pub character_id: CharacterId,
    pub level: i32,
    pub color: i32,
    /// C `number = it[item_id].drdata[3]` (the pentagram's per-level
    /// sequence id, assigned once during system init).
    pub number: i32,
    pub is_quest_solved: bool,
    /// `active_pentagrams` *before* the post-solve reset (matches C
    /// calling `add_pentagram_to_player` - which logs this count - before
    /// `check_for_quest_completion` zeroes it).
    pub active_pentagrams: i32,
    pub total_pentagrams: i32,
}

impl World {
    /// C `initialize_pentagram_system` (`pents.c:304-374`), minus the
    /// DB-backed `load_pentagram_record` call (not ported, see the module
    /// doc comment). Lazily triggered on first pentagram interaction,
    /// matching C's `if (!init_done) { ...; init_done = 1; }` guard at the
    /// top of `handle_pentagram_interaction`.
    pub(crate) fn ensure_pentagram_system_initialized(&mut self) {
        if self.pentagram_quest.initialized {
            return;
        }
        self.pentagram_quest.initialized = true;

        let mut level_counts = [0i32; PENT_MAX_LEVEL + 1];
        let mut total = 0;
        let mut active = 0;
        let mut area_counts = [0i32; PENT_MAX_LEVEL + 1];
        let mut area_activated = [0i32; PENT_MAX_LEVEL + 1];
        let solve_serial = self.pentagram_quest.solve_serial;
        let area_serials = self.pentagram_quest.area_serials;
        let mut min_level = 0;
        let mut max_level = 0;

        let item_ids: Vec<ItemId> = self
            .items
            .iter()
            .filter(|(_, item)| item.driver == IDR_PENT && !item.flags.is_empty())
            .map(|(id, _)| *id)
            .collect();

        for item_id in item_ids {
            let Some(item) = self.items.get(&item_id) else {
                continue;
            };
            let level = item.driver_data.first().copied().unwrap_or(0) as usize;
            if level > PENT_MAX_LEVEL {
                continue;
            }
            level_counts[level] += 1;
            let number = level_counts[level];
            let mut color = item.driver_data.get(2).copied().unwrap_or(0) as i32;
            if color == 0 {
                color = 1 + legacy_random_below_from_seed(&mut self.legacy_random_seed, 3) as i32;
            }
            let status = item.driver_data.get(1).copied().unwrap_or(0) as i32;
            let area_status = item.driver_data.get(4).copied().unwrap_or(0) as i32;

            if let Some(item) = self.items.get_mut(&item_id) {
                if item.driver_data.len() <= 3 {
                    item.driver_data.resize(4, 0);
                }
                item.driver_data[3] = number as u8;
                item.driver_data[2] = color as u8;
            }

            total += 1;
            if status != 0 && status == solve_serial {
                active += 1;
            }
            area_counts[level] += 1;
            if status != 0 && area_status == area_serials[level] {
                area_activated[level] += 1;
            }
        }

        for (level, count) in level_counts.iter().enumerate().skip(1) {
            if *count > 0 {
                if min_level == 0 {
                    min_level = level as i32;
                }
                max_level = level as i32;
            }
        }
        // C `min_level--; max_level--;` after the 1-indexed scan loop.
        min_level -= 1;
        max_level -= 1;

        self.pentagram_quest.total_pentagrams = total;
        self.pentagram_quest.active_pentagrams = active;
        self.pentagram_quest.area_pentagram_counts = area_counts;
        self.pentagram_quest.area_activated_counts = area_activated;
        self.pentagram_quest.min_level = min_level;
        self.pentagram_quest.max_level = max_level;

        self.calculate_required_pentagrams();
    }

    /// C `count_active_players_in_area` (`pents.c:383-408`).
    fn count_active_players_in_area(&self) -> i32 {
        if self.area_id == 25 {
            self.characters
                .values()
                .filter(|c| c.flags.contains(CharacterFlags::PLAYER) && c.x < 108)
                .count() as i32
        } else if self.area_id == 34 {
            self.characters
                .values()
                .filter(|c| {
                    c.flags.contains(CharacterFlags::PLAYER)
                        && (129..=238).contains(&c.x)
                        && c.y < 188
                })
                .count() as i32
        } else {
            // C uses the global `online` player count here; `World` has
            // no such counter (see `PentagramQuestState`'s module doc
            // comment), so this counts currently-instantiated `CF_PLAYER`
            // characters as the closest available proxy.
            self.characters
                .values()
                .filter(|c| c.flags.contains(CharacterFlags::PLAYER))
                .count() as i32
        }
    }

    /// C `calculate_required_pentagrams` (`pents.c:416-432`).
    pub(crate) fn calculate_required_pentagrams(&mut self) {
        let player_count = self.count_active_players_in_area();
        let mut required = SOLVE_BASE
            + (player_count + 1) * SOLVE_MULTIPLIER
            + legacy_random_below_from_seed(
                &mut self.legacy_random_seed,
                ((player_count + 1) * SOLVE_RANDOM_MULTIPLIER).max(0) as u32,
            ) as i32;

        let max_allowed = self.pentagram_quest.total_pentagrams
            - self.pentagram_quest.total_pentagrams / self.settings.get_solve_max_divisor().max(1);
        if required > max_allowed {
            required = max_allowed;
        }

        if self.area_id == 21 {
            required = 12;
        }

        self.pentagram_quest.required_activations = required;
    }

    /// C `update_power_levels` (`pents.c:440-459`).
    fn update_power_levels(&mut self) {
        let min_level = self.pentagram_quest.min_level;
        let max_level = self.pentagram_quest.max_level;
        if min_level < 0 || max_level < min_level {
            return;
        }
        let increment = self.settings.get_power_increment();
        let cap = self.settings.get_max_power_level();
        let mut total_power = 0i64;
        for level in min_level..=max_level {
            let Some(slot) = usize::try_from(level).ok().filter(|&l| l < PENT_MAX_LEVEL) else {
                continue;
            };
            let value = (self.pentagram_quest.power_levels[slot] + increment).min(cap);
            self.pentagram_quest.power_levels[slot] = value;
            total_power += i64::from(value);
        }

        if self.area_id != 21 {
            let max_training = i64::from(self.settings.get_max_training_power());
            self.pentagram_quest.training_power = total_power.clamp(0, max_training.max(0)) as i32;
        }
    }

    /// C `activate_pentagram` (`pents.c:473-501`).
    fn activate_pentagram_item(&mut self, item_id: ItemId) {
        let Some(before) = self.items.get(&item_id).cloned() else {
            return;
        };
        let level = before.driver_data.first().copied().unwrap_or(0) as usize;
        let color = before.driver_data.get(2).copied().unwrap_or(0) as i32;
        let solve_serial = self.pentagram_quest.solve_serial;

        if let Some(item) = self.items.get_mut(&item_id) {
            if item.driver_data.len() <= 4 {
                item.driver_data.resize(5, 0);
            }
            item.driver_data[1] = solve_serial as u8;
            item.sprite += color;
            item.modifier_value[0] = 100;
        }
        self.refresh_item_light_after_mutation(&before, item_id);
        if let Some(item) = self.items.get(&item_id) {
            self.queue_sound_area(usize::from(item.x), usize::from(item.y), 42);
        }

        self.pentagram_quest.active_pentagrams += 1;

        if level < PENT_MAX_LEVEL + 1 {
            self.pentagram_quest.area_activated_counts[level] += 1;
            let area_serial = self.pentagram_quest.area_serials[level];
            if let Some(item) = self.items.get_mut(&item_id) {
                item.driver_data[4] = area_serial as u8;
            }
            if self.pentagram_quest.area_activated_counts[level]
                >= self.pentagram_quest.area_pentagram_counts[level]
            {
                let mut next_serial = self.pentagram_quest.area_serials[level] + 1;
                if next_serial > 255 {
                    next_serial = 1;
                }
                self.pentagram_quest.area_serials[level] = next_serial;
            }
        }
    }

    /// C `deactivate_pentagram` (`pents.c:510-528`).
    fn deactivate_pentagram_item(&mut self, item_id: ItemId) {
        let Some(before) = self.items.get(&item_id).cloned() else {
            return;
        };
        let level = before.driver_data.first().copied().unwrap_or(0) as usize;
        let color = before.driver_data.get(2).copied().unwrap_or(0) as i32;

        if let Some(item) = self.items.get_mut(&item_id) {
            if item.driver_data.len() <= 4 {
                item.driver_data.resize(5, 0);
            }
            item.driver_data[1] = 0;
            item.driver_data[4] = 0;
            item.sprite -= color;
            item.modifier_value[0] = 10;
        }
        self.refresh_item_light_after_mutation(&before, item_id);

        if level < PENT_MAX_LEVEL + 1 && self.pentagram_quest.area_activated_counts[level] > 0 {
            self.pentagram_quest.area_activated_counts[level] -= 1;
        }
    }

    /// C `handle_pentagram_interaction`'s player-activation branch plus
    /// `check_for_quest_completion` (`pents.c:1456-1461`, `538-552`).
    /// Queues a [`PentagramActivationEvent`] for `ugaris-server` to apply
    /// the per-player reward bookkeeping, and a
    /// [`PentagramDemonSpawnRequest`] for the unconditional `spawn_count =
    /// get_activation_spawn_count()` demon-spawn tail (`pents.c:1460`).
    pub(crate) fn apply_pentagram_activate(&mut self, item_id: ItemId, character_id: CharacterId) {
        self.ensure_pentagram_system_initialized();

        let level = self
            .items
            .get(&item_id)
            .map(|item| item.driver_data.first().copied().unwrap_or(0) as i32)
            .unwrap_or(0);
        let number = self
            .items
            .get(&item_id)
            .map(|item| item.driver_data.get(3).copied().unwrap_or(0) as i32)
            .unwrap_or(0);

        self.activate_pentagram_item(item_id);
        let color = self
            .items
            .get(&item_id)
            .map(|item| item.driver_data.get(2).copied().unwrap_or(0) as i32)
            .unwrap_or(0);

        let active_snapshot = self.pentagram_quest.active_pentagrams;
        let total_snapshot = self.pentagram_quest.total_pentagrams;
        let is_quest_solved = active_snapshot >= self.pentagram_quest.required_activations;

        if is_quest_solved {
            self.pentagram_quest.active_pentagrams = 0;
            let mut next_serial = self.pentagram_quest.solve_serial + 1;
            if next_serial > 255 {
                next_serial = 1;
            }
            self.pentagram_quest.solve_serial = next_serial;
            self.calculate_required_pentagrams();
            self.update_power_levels();
            self.pentagram_quest.last_solve_tick = Some(self.tick.0 as u32);
        }

        self.pending_pentagram_activations
            .push(PentagramActivationEvent {
                item_id,
                character_id,
                level,
                color,
                number,
                is_quest_solved,
                active_pentagrams: active_snapshot,
                total_pentagrams: total_snapshot,
            });

        let spawn_count = self.settings.get_activation_spawn_count();
        if spawn_count > 0 {
            self.pending_pentagram_demon_spawns
                .push(PentagramDemonSpawnRequest {
                    item_id,
                    spawn_count,
                    level,
                });
        }
    }

    /// C `handle_pentagram_interaction`'s timer branch (`pents.c:1462-
    /// 1474`), including the demon-spawn tail: a reset (solved/area-reset)
    /// pentagram spawns `get_activation_spawn_count()` demons, while an
    /// inactive one has a `1/get_random_spawn_chance()` chance to spawn a
    /// single demon each timer tick.
    pub(crate) fn apply_pentagram_timer(&mut self, item_id: ItemId, status: i32, area_status: i32) {
        self.ensure_pentagram_system_initialized();
        let level = self
            .items
            .get(&item_id)
            .map(|item| item.driver_data.first().copied().unwrap_or(0) as i32)
            .unwrap_or(0);

        let mut spawn_count = 0;
        if status != 0 {
            let current_area_serial = self
                .pentagram_quest
                .area_serials
                .get(level as usize)
                .copied()
                .unwrap_or(255);
            if status != self.pentagram_quest.solve_serial || area_status != current_area_serial {
                self.deactivate_pentagram_item(item_id);
                spawn_count = self.settings.get_activation_spawn_count();
            }
        } else {
            let chance = self.settings.get_random_spawn_chance().max(0) as u32;
            if legacy_random_below_from_seed(&mut self.legacy_random_seed, chance) == 0 {
                spawn_count = 1;
            }
        }

        if spawn_count > 0 {
            self.pending_pentagram_demon_spawns
                .push(PentagramDemonSpawnRequest {
                    item_id,
                    spawn_count,
                    level,
                });
        }
    }

    /// Drains [`PentagramActivationEvent`]s queued by
    /// [`World::apply_pentagram_activate`] for `ugaris-server` to apply.
    pub fn drain_pending_pentagram_activations(&mut self) -> Vec<PentagramActivationEvent> {
        std::mem::take(&mut self.pending_pentagram_activations)
    }

    /// Drains [`PentagramDemonSpawnRequest`]s queued by
    /// [`World::apply_pentagram_activate`]/[`World::apply_pentagram_timer`]
    /// for `ugaris-server` to instantiate from `penterN` zone templates.
    pub fn drain_pending_pentagram_demon_spawns(&mut self) -> Vec<PentagramDemonSpawnRequest> {
        std::mem::take(&mut self.pending_pentagram_demon_spawns)
    }

    /// Drains the `CharacterId`s queued by [`World::apply_penter_demon_death`]
    /// for `ugaris-server` to award `ACHIEVEMENT_DEMON_LORDS_DEMISE`.
    pub fn drain_pending_penter_demon_lords_demise_awards(&mut self) -> Vec<CharacterId> {
        std::mem::take(&mut self.pending_penter_demon_lords_demise_awards)
    }

    /// C `spawn_demons_at_pentagram`'s max-spawns lookup (`pents.c:1010-
    /// 1015`).
    pub fn pentagram_max_spawns(&self, level: i32) -> i32 {
        if level < self.settings.get_spawn_count_level_threshold() {
            self.settings.get_max_spawn_low_level()
        } else {
            self.settings.get_max_spawn_high_level()
        }
    }

    /// C `spawn_demons_at_pentagram`'s per-slot occupancy check
    /// (`pents.c:1022-1024`): `!character_id || !ch[character_id].flags ||
    /// (unsigned short)ch[character_id].serial != (unsigned short)serial`.
    /// Slot `index`'s 4 bytes live at `driver_data[6 + index*4 ..]`
    /// (`unsigned short character_id`, `unsigned short serial`).
    pub fn pentagram_spawn_slot_is_stale(&self, item_id: ItemId, index: usize) -> bool {
        let Some(item) = self.items.get(&item_id) else {
            return true;
        };
        let offset = 6 + index * 4;
        let Some(bytes) = item.driver_data.get(offset..offset + 4) else {
            return true;
        };
        let character_id = u16::from_le_bytes([bytes[0], bytes[1]]);
        let serial = u16::from_le_bytes([bytes[2], bytes[3]]);
        if character_id == 0 {
            return true;
        }
        match self.characters.get(&CharacterId(u32::from(character_id))) {
            None => true,
            Some(character) => character.flags.is_empty() || character.serial as u16 != serial,
        }
    }

    /// C `spawn_demons_at_pentagram`'s demon-type roll (`pents.c:1025-
    /// 1035`).
    pub fn roll_pentagram_demon_type(&mut self) -> DemonType {
        if legacy_random_below_from_seed(&mut self.legacy_random_seed, ELITE_DEMON_SPAWN_CHANCE)
            == 0
        {
            DemonType::Elite
        } else if legacy_random_below_from_seed(
            &mut self.legacy_random_seed,
            LESSER_DEMON_SPAWN_CHANCE,
        ) == 0
        {
            DemonType::Lesser
        } else {
            DemonType::Normal
        }
    }

    /// C `sprintf(npc_name, "penter%d", level * 2 + RANDOM(2));`
    /// (`pents.c:1038`).
    pub fn pentagram_demon_template_name(&mut self, level: i32) -> String {
        let extra = legacy_random_below_from_seed(&mut self.legacy_random_seed, 2) as i32;
        format!("penter{}", level * 2 + extra)
    }

    /// C `process_demon_messages`'s JSON loot-table id pick (`pents.c:
    /// 1560-1583`).
    pub fn pentagram_demon_loot_table_id(level: u32, is_elite: bool) -> &'static str {
        if level <= 38 {
            if is_elite {
                "pent_demon_low_elite"
            } else {
                "pent_demon_low"
            }
        } else if level <= 70 {
            if is_elite {
                "pent_demon_mid_elite"
            } else {
                "pent_demon_mid"
            }
        } else if is_elite {
            "pent_demon_high_elite"
        } else {
            "pent_demon_high"
        }
    }

    /// C `enhance_elite_demon` (`pents.c:938-967`).
    pub fn enhance_elite_demon(&mut self, character_id: CharacterId) {
        if let Some(character) = self.characters.get_mut(&character_id) {
            for stat in character.values[1].iter_mut() {
                if *stat > 0 {
                    *stat = (f64::from(*stat) * ELITE_DEMON_STAT_MULTIPLIER) as i16;
                }
            }
        }

        let name_index = legacy_random_below_from_seed(
            &mut self.legacy_random_seed,
            ELITE_DEMON_NAMES.len() as u32,
        ) as usize;
        let demon_name = ELITE_DEMON_NAMES[name_index];

        if let Some(character) = self.characters.get_mut(&character_id) {
            character.name = format!("Elite Demon \"{demon_name}\"");
            // C `ch[character_id].c1 = 153;` (reddish tint).
            character.c1 = 153;
            character.description =
                format!("A powerful elite demon bearing the ancient Babylonian name {demon_name}.");
        }

        self.enhance_demon_character(character_id, 3, 3, 8, 0, true);
    }

    /// C `adjust_lesser_demon` (`pents.c:976-992`).
    pub fn adjust_lesser_demon(&mut self, character_id: CharacterId) {
        if let Some(character) = self.characters.get_mut(&character_id) {
            for stat in character.values[1].iter_mut() {
                if *stat > 0 {
                    *stat = (f64::from(*stat) * LESSER_DEMON_STAT_MULTIPLIER) as i16;
                }
            }
            character.name = format!("Lesser {}", character.name);
            // C `ch[character_id].c1 = 187;` (grayish tint).
            character.c1 = 187;
        }
    }

    /// C `enhance_demon_character` (`pents.c:1135-1356`): boosts
    /// `skill_count` randomly-selected skills (from a `priority_type`-
    /// dependent candidate pool, falling back to every nonzero skill when
    /// the pool is too small) by `min_boost..=max_boost` percent each,
    /// optionally appending an `"Enhanced: Name +N, ..."` line to the
    /// character's description.
    fn enhance_demon_character(
        &mut self,
        character_id: CharacterId,
        skill_count: i32,
        min_boost: i32,
        max_boost: i32,
        priority_type: i32,
        update_desc: bool,
    ) {
        use crate::entity::CharacterValue as CV;
        const MAX_SKILL_POOL: usize = 15;

        let skill_count = if skill_count <= 0 {
            3
        } else {
            skill_count.min(MAX_SKILL_POOL as i32)
        } as usize;
        let min_boost = if min_boost <= 0 { 10 } else { min_boost };
        let max_boost = if max_boost <= min_boost {
            min_boost + 20
        } else {
            max_boost
        };
        let priority_type = if (0..=3).contains(&priority_type) {
            priority_type
        } else {
            0
        };

        let Some(character) = self.characters.get(&character_id) else {
            return;
        };
        let values = character.values[1].clone();
        let has = |v: CV| values.get(v as usize).copied().unwrap_or(0) > 0;

        let mut pool: Vec<usize> = Vec::with_capacity(MAX_SKILL_POOL);
        match priority_type {
            1 => {
                for v in [
                    CV::Attack,
                    CV::Sword,
                    CV::Hand,
                    CV::Dagger,
                    CV::TwoHand,
                    CV::Tactics,
                    CV::Surround,
                    CV::Warcry,
                    CV::Rage,
                ] {
                    if has(v) {
                        pool.push(v as usize);
                    }
                }
            }
            2 => {
                for v in [CV::Parry, CV::Immunity, CV::Armor, CV::ArmorSkill, CV::Hp] {
                    if has(v) {
                        pool.push(v as usize);
                    }
                }
            }
            3 => {
                // C pushes `V_FIREBALL` and its alias `V_FIRE` as two
                // separate entries (`pents.c:1222-1223`) - a genuine
                // duplicate that doubles Fireball's selection weight.
                for v in [
                    CV::Mana,
                    CV::MagicShield,
                    CV::Flash,
                    CV::Fireball,
                    CV::Fireball,
                    CV::Freeze,
                    CV::Bless,
                    CV::Heal,
                    CV::Duration,
                ] {
                    if has(v) {
                        pool.push(v as usize);
                    }
                }
            }
            _ => {
                for v in [
                    CV::Attack,
                    CV::Parry,
                    CV::Sword,
                    CV::Hand,
                    CV::Freeze,
                    CV::Flash,
                    CV::Fireball,
                    CV::Immunity,
                    CV::Tactics,
                    CV::Warcry,
                    CV::Bless,
                    CV::MagicShield,
                    CV::Dagger,
                    CV::Staff,
                    CV::Duration,
                ] {
                    if has(v) && pool.len() < MAX_SKILL_POOL {
                        pool.push(v as usize);
                    }
                }
            }
        }

        if pool.len() < skill_count {
            pool.clear();
            for (index, &value) in values.iter().enumerate() {
                if value > 0 {
                    pool.push(index);
                    if pool.len() >= MAX_SKILL_POOL {
                        break;
                    }
                }
            }
        }
        let skill_count = skill_count.min(pool.len());
        if skill_count == 0 {
            return;
        }

        let mut boosted = [false; CHARACTER_VALUE_COUNT];
        let mut skill_desc = String::new();
        if update_desc {
            skill_desc.push_str("Enhanced: ");
        }

        for i in 0..skill_count {
            let mut skill_index = pool[0];
            for tries in 0..=20 {
                let selected =
                    legacy_random_below_from_seed(&mut self.legacy_random_seed, pool.len() as u32)
                        as usize;
                skill_index = pool[selected];
                if tries >= 20 || !boosted[skill_index] {
                    break;
                }
            }
            boosted[skill_index] = true;

            let range = legacy_random_below_from_seed(
                &mut self.legacy_random_seed,
                (max_boost - min_boost + 1).max(0) as u32,
            ) as i32;

            let Some(character) = self.characters.get_mut(&character_id) else {
                return;
            };
            let base = i32::from(character.values[1][skill_index]);
            let mut boost_amount = base * (min_boost + range) / 100;
            if boost_amount < 1 {
                boost_amount = 1;
            }
            character.values[1][skill_index] =
                (base + boost_amount).clamp(i16::MIN as i32, i16::MAX as i32) as i16;

            if update_desc {
                if i > 0 {
                    skill_desc.push_str(", ");
                }
                skill_desc.push_str(&format!(
                    "{} +{boost_amount}",
                    crate::entity::CHARACTER_VALUE_NAMES[skill_index]
                ));
            }
        }

        if update_desc && !skill_desc.is_empty() {
            if let Some(character) = self.characters.get_mut(&character_id) {
                let old_desc = character.description.clone();
                character.description = if old_desc.is_empty() {
                    skill_desc
                } else if let Some(pos) = old_desc.find("Enhanced:") {
                    format!("{}{skill_desc}", &old_desc[..pos])
                } else {
                    format!("{old_desc}\n{skill_desc}")
                };
            }
        }
    }

    /// Finishes a demon spawn after `ugaris-server` placed the
    /// `penterN`-template character at the pentagram (`pents.c:1050-
    /// 1080`): elite/lesser stat mutation and class reassignment,
    /// `update_char`, self-bless if the template carries `V_BLESS`, full
    /// power hp/endurance/mana/lifeshield, facing, and `CF_NONOTIFY`.
    pub fn finish_pentagram_demon_spawn(
        &mut self,
        character_id: CharacterId,
        demon_type: DemonType,
        original_class: i32,
    ) {
        match demon_type {
            DemonType::Elite => {
                self.enhance_elite_demon(character_id);
                if let Some(character) = self.characters.get_mut(&character_id) {
                    character.class = ELITE_DEMON_CLASS_BASE + original_class % 48;
                }
            }
            DemonType::Lesser => {
                self.adjust_lesser_demon(character_id);
                if let Some(character) = self.characters.get_mut(&character_id) {
                    character.class = LESSER_DEMON_CLASS_BASE + original_class % 48;
                }
            }
            DemonType::Normal => {}
        }

        self.update_character(character_id);

        let bless_strength = self
            .characters
            .get(&character_id)
            .map(|character| i32::from(character.values[0][CharacterValue::Bless as usize]))
            .unwrap_or(0);
        if bless_strength > 0 {
            self.install_bless_spell(character_id, bless_strength, BLESS_DURATION);
        }

        if let Some(character) = self.characters.get_mut(&character_id) {
            character.hp = i32::from(character.values[0][CharacterValue::Hp as usize]) * POWERSCALE;
            character.endurance =
                i32::from(character.values[0][CharacterValue::Endurance as usize]) * POWERSCALE;
            character.mana =
                i32::from(character.values[0][CharacterValue::Mana as usize]) * POWERSCALE;
            character.lifeshield =
                i32::from(character.values[0][CharacterValue::MagicShield as usize]) * POWERSCALE;
            character.dir = Direction::RightDown as u8;
            character.flags.insert(CharacterFlags::NONOTIFY);
        }
    }

    /// C `spawn_demons_at_pentagram`'s per-slot bookkeeping write
    /// (`pents.c:1082-1083`): `*(unsigned short *)(it[item_id].drdata + 6
    /// + index * 4) = character_id; *(unsigned short *)(it[item_id].drdata
    /// + 8 + index * 4) = ch[character_id].serial;`.
    pub fn apply_pentagram_spawn_result(
        &mut self,
        item_id: ItemId,
        index: usize,
        character_id: CharacterId,
        serial: u32,
    ) -> bool {
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        let offset = 6 + index * 4;
        if item.driver_data.len() < offset + 4 {
            item.driver_data.resize(offset + 4, 0);
        }
        item.driver_data[offset..offset + 2]
            .copy_from_slice(&(character_id.0 as u16).to_le_bytes());
        item.driver_data[offset + 2..offset + 4].copy_from_slice(&(serial as u16).to_le_bytes());
        true
    }

    /// C `update_demon_profession` (`pents.c:1102-1119`).
    pub fn update_demon_profession(&mut self, character_id: CharacterId) {
        let training_power = self.pentagram_quest.training_power;
        let mut changed = false;
        if let Some(character) = self.characters.get_mut(&character_id) {
            if character.deaths == 0 {
                character.deaths = character
                    .professions
                    .get(P_DEMON)
                    .copied()
                    .unwrap_or(0)
                    .max(0) as u32;
            }
            let offset: i64 = if (258..=305).contains(&character.class) {
                52_000
            } else {
                20_000
            };
            let prof_value = (i64::from(character.deaths)
                * (i64::from(training_power.max(0)) + offset)
                / 64_000) as i32;
            let prof_value = prof_value.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            if character.professions.len() <= P_DEMON {
                character.professions.resize(P_DEMON + 1, 0);
            }
            if character.professions[P_DEMON] != prof_value {
                character.professions[P_DEMON] = prof_value;
                changed = true;
            }
        }
        if changed {
            self.update_character(character_id);
        }
    }

    /// C `notify_power_change` (`pents.c:1415-1424`).
    fn notify_power_change(&mut self, player_id: CharacterId, power_loss: i32) {
        let training_power = self.pentagram_quest.training_power;
        let message = if training_power >= 0 {
            format!(
                "Training area power setting down to {:.2}%, loss of {:.2}%.",
                100.0 / 32000.0 * f64::from(training_power),
                100.0 / 32000.0 * f64::from(power_loss),
            )
        } else {
            format!(
                "Training area power setting down to 0.00%, {:.2}% underpowered, loss of {:.2}%.",
                -100.0 / 32000.0 * f64::from(training_power),
                100.0 / 32000.0 * f64::from(power_loss),
            )
        };
        self.queue_system_text(player_id, message);
    }

    /// C `handle_demon_death` (`pents.c:1366-1405`), dispatched from
    /// `ch_died_driver`'s `CDR_PENTER` case (`pents.c:1889-1893`) - called
    /// by [`World::kill_character_followup`] for every `CDR_PENTER`
    /// death. Elite/lesser custom demon types never reduce power; only
    /// the demon-lord class ranges `258..=305`/`404..=411` do (neither of
    /// which any `zones/4/pents.chr` template currently uses - see the
    /// module doc comment - so this is a documented no-op for Area 4's
    /// own demons today, exactly matching C).
    pub(crate) fn apply_penter_demon_death(
        &mut self,
        character_id: CharacterId,
        killer_id: Option<CharacterId>,
    ) {
        let Some(demon_class) = self.characters.get(&character_id).map(|c| c.class) else {
            return;
        };

        if (LESSER_DEMON_CLASS_BASE..LESSER_DEMON_CLASS_BASE + 48).contains(&demon_class)
            || (ELITE_DEMON_CLASS_BASE..ELITE_DEMON_CLASS_BASE + 48).contains(&demon_class)
        {
            return;
        }
        if !((258..=305).contains(&demon_class) || (404..=411).contains(&demon_class)) {
            return;
        }
        let class_index = if (258..=305).contains(&demon_class) {
            (demon_class - 258) as usize
        } else {
            (demon_class - 404 + 48) as usize
        };
        let Some(&current_power) = self.pentagram_quest.power_levels.get(class_index) else {
            return;
        };

        let power_reduction = self.settings.get_demon_power_deduction();
        let power_loss = current_power + power_reduction;
        self.pentagram_quest.power_levels[class_index] = -power_reduction;
        self.pentagram_quest.training_power -= power_loss;

        if let Some(killer_id) = killer_id {
            self.notify_power_change(killer_id, power_loss);
            if self
                .characters
                .get(&killer_id)
                .is_some_and(|killer| killer.flags.contains(CharacterFlags::PLAYER))
            {
                self.pending_penter_demon_lords_demise_awards
                    .push(killer_id);
            }
        }
    }

    /// C `reset_pentagram_colors` (`pents.c:682-696`): only runs when the
    /// player had a five-of-a-kind combo (`player_data->status != 0`),
    /// randomly reassigning the five involved pentagrams' colors. Item
    /// mutation belongs to `World`, unlike the rest of `distribute_
    /// rewards_to_player` (see module doc comment), so `ugaris-server`
    /// calls this directly with the player's `pent_it`/`pent_color`
    /// arrays before resetting its own `PentagramDebugData`.
    pub fn reset_pentagram_colors(&mut self, pent_it: &[i32; 6], had_combo: bool) {
        if !had_combo {
            return;
        }
        for &raw_item_id in pent_it.iter().take(5) {
            if raw_item_id <= 0 {
                continue;
            }
            let item_id = ItemId(raw_item_id as u32);
            let Some(before) = self.items.get(&item_id).cloned() else {
                continue;
            };
            let old_color = before.driver_data.get(2).copied().unwrap_or(0) as i32;
            let new_color =
                1 + legacy_random_below_from_seed(&mut self.legacy_random_seed, 3) as i32;
            let is_active = before.driver_data.get(1).copied().unwrap_or(0) != 0;
            if let Some(item) = self.items.get_mut(&item_id) {
                if item.driver_data.len() <= 2 {
                    item.driver_data.resize(3, 0);
                }
                item.driver_data[2] = new_color as u8;
                if is_active {
                    item.sprite -= old_color - new_color;
                }
            }
        }
    }
}
