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
//! Not ported here (tracked as a `PORTING_TODO.md` "Area 4" REMAINING
//! gap): demon spawning/`CDR_PENTER`/`CDR_TESTER` (so `spawn_count`/
//! `spawn_demons_at_pentagram` calls in C's `handle_pentagram_interaction`
//! have no Rust equivalent yet - pentagrams activate/deactivate/solve but
//! never spawn guardian demons), the `pentagram_record` DB load/save
//! (`load_pentagram_record`/`save_pentagram_record_scheduled` - in-memory
//! only, resets on restart, same as `World::arena_toplist`), and the
//! macro-daemon challenge-room `saved_pent_*` restore
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
    /// `check_for_quest_completion` (`pents.c:1456-1461`, `538-552`),
    /// minus the demon-spawn tail (not ported, see module doc comment).
    /// Queues a [`PentagramActivationEvent`] for `ugaris-server` to apply
    /// the per-player reward bookkeeping.
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
    }

    /// C `handle_pentagram_interaction`'s timer branch (`pents.c:1462-
    /// 1474`), minus the demon-spawn tail (not ported, see module doc
    /// comment).
    pub(crate) fn apply_pentagram_timer(&mut self, item_id: ItemId, status: i32, area_status: i32) {
        self.ensure_pentagram_system_initialized();
        if status == 0 {
            return;
        }
        let level = self
            .items
            .get(&item_id)
            .map(|item| item.driver_data.first().copied().unwrap_or(0) as usize)
            .unwrap_or(0);
        let current_area_serial = self
            .pentagram_quest
            .area_serials
            .get(level)
            .copied()
            .unwrap_or(255);
        if status != self.pentagram_quest.solve_serial || area_status != current_area_serial {
            self.deactivate_pentagram_item(item_id);
        }
    }

    /// Drains [`PentagramActivationEvent`]s queued by
    /// [`World::apply_pentagram_activate`] for `ugaris-server` to apply.
    pub fn drain_pending_pentagram_activations(&mut self) -> Vec<PentagramActivationEvent> {
        std::mem::take(&mut self.pending_pentagram_activations)
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
