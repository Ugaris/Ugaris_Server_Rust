//! Areas 23/24 strategy minigame `special_driver` player-facing `#`/`/`
//! command table (`src/area/23_24/strategy.c:3278-3626`, `CDR_STRATEGY_
//! PARSER = 79`) - the `#jp`/`#list`/`#info`/`#raise`/`#reset`/`#mission`/
//! `#enter`/`#surrender`/`#queue` commands a player uses to interact with
//! the minigame once `strategy_boss` (Cinciac's still-unported dialogue
//! driver) has raised `StrategyPpd::boss_stage` to `9`. See
//! `crate::world::strategy`'s module doc comment for the C source
//! cross-reference and the rest of this subsystem's port status.
//!
//! `#eguard` is deliberately NOT ported yet: `create_eguard` spawns a
//! fresh "strategy_npc" character running the still-unported
//! `DRD_STRATEGYDRIVER` worker AI, which needs `ZoneLoader` (not
//! reachable from `World` alone) - same split precedent as `#nspawn`/
//! `#thrall` in `world::lq_admin`. Left as a documented gap in
//! `PORTING_TODO.md`; a player typing `#eguard` today falls through this
//! table unmatched and the text is treated as ordinary chat, exactly like
//! any other genuinely-unrecognized command word (not a regression from
//! today's fully-unported baseline).
//!
//! Every handler takes `ppd: &mut StrategyPpd` explicitly rather than
//! reading it from `self` - `World` cannot reach session-owned
//! `PlayerRuntime::strategy` directly (same split as [`crate::world::
//! str_raise`]/[`crate::world::apply_strategy_mission_win`]); `ugaris-
//! server` looks the real `ppd` up via `ServerRuntime::
//! player_for_character_mut` before calling
//! [`World::apply_strategy_special_command`].

use super::lq_admin::{cmd_word_matches, legacy_atoi, ArgReader};
use super::*;
use crate::player::StrategyPpd;

/// C `#define MAXJUMP 256` (`strategy.c:2988`): capacity of the
/// [`StrategyJumpPointRegistry`].
pub const MAXJUMP: usize = 256;

/// C `struct jumppoint { int in, x, y; }` (`strategy.c:2989-2992`), one
/// entry per registered `IDR_STR_DEPOT`/`IDR_STR_STORAGE` item.
#[derive(Debug, Clone, Copy)]
pub struct StrategyJumpPoint {
    pub item_id: ItemId,
    pub x: u16,
    pub y: u16,
}

impl Default for StrategyJumpPoint {
    /// Index `0`'s unused placeholder (C never populates `jp[0]`, since
    /// `max_jp` starts at `1`).
    fn default() -> Self {
        StrategyJumpPoint {
            item_id: ItemId(0),
            x: 0,
            y: 0,
        }
    }
}

/// C's file-static `struct jumppoint jp[MAXJUMP]`/`int special_init,
/// max_jp` (`strategy.c:2994-2995`). Index `0` is always an unused
/// placeholder (`points.len()` mirrors C's `max_jp`, starting at `1` once
/// initialized), matching every `1..max_jp` loop bound in the C source.
#[derive(Debug, Clone, Default)]
pub struct StrategyJumpPointRegistry {
    pub points: Vec<StrategyJumpPoint>,
    initialized: bool,
}

impl World {
    /// C `special_driver`'s lazy `if (!special_init) { ... }` block
    /// (`strategy.c:3291-3312`): discovers every `IDR_STR_DEPOT`/
    /// `IDR_STR_STORAGE` item across *all* areas (not scoped to one
    /// battleground slot, unlike [`Self::ensure_strategy_areas_
    /// initialized`]) in ascending item-id order (ditto rationale as
    /// that method: `self.items` is an unordered `HashMap`) and stamps
    /// each with its jump-point number via `item.description`. Skips
    /// C's trailing `elog("too many jump points")` call (pure logging).
    pub fn ensure_strategy_jump_points_initialized(&mut self) {
        if self.strategy_jump_points.initialized {
            return;
        }
        self.strategy_jump_points.initialized = true;
        self.strategy_jump_points.points = vec![StrategyJumpPoint::default()];

        let mut item_ids: Vec<ItemId> = self
            .items
            .iter()
            .filter(|(_, item)| {
                !item.flags.is_empty()
                    && (item.driver == IDR_STR_DEPOT || item.driver == IDR_STR_STORAGE)
            })
            .map(|(id, _)| *id)
            .collect();
        item_ids.sort_by_key(|id| id.0);

        for item_id in item_ids {
            if self.strategy_jump_points.points.len() >= MAXJUMP {
                break;
            }
            let Some(item) = self.items.get(&item_id) else {
                continue;
            };
            let index = self.strategy_jump_points.points.len();
            let (x, y, driver) = (item.x, item.y, item.driver);
            self.strategy_jump_points
                .points
                .push(StrategyJumpPoint { item_id, x, y });
            if let Some(item) = self.items.get_mut(&item_id) {
                item.description = if driver == IDR_STR_DEPOT {
                    format!("JP {index}. A depot is used to store Platinum temporarily.")
                } else {
                    format!("JP {index}. The storage contains all Platinum collected so far.")
                };
            }
        }
    }

    /// C's repeated `if (ppd->boss_stage < 9) { log_char(...); return 2;
    /// }` gate guarding every command in this table except `#reset`/
    /// `#queue`. Returns whether the gate fired (caller should stop).
    fn strategy_needs_boss(&mut self, character_id: CharacterId, ppd: &StrategyPpd) -> bool {
        if ppd.boss_stage < 9 {
            self.queue_system_text(
                character_id,
                "You have to talk to Cinciac first.".to_string(),
            );
            true
        } else {
            false
        }
    }

    /// C `#jp <nr>` (`strategy.c:3316-3336`).
    fn strategy_cmd_jp(&mut self, character_id: CharacterId, ppd: &StrategyPpd, val: i32) {
        if self.strategy_needs_boss(character_id, ppd) {
            return;
        }
        self.ensure_strategy_jump_points_initialized();
        let max_jp = self.strategy_jump_points.points.len() as i32;
        if val < 1 || val >= max_jp {
            self.queue_system_text(character_id, "Jump point out of bounds.".to_string());
            return;
        }
        let point = self.strategy_jump_points.points[val as usize];
        let serial = self
            .characters
            .get(&character_id)
            .map(|c| c.serial)
            .unwrap_or(0);
        let owner = self
            .items
            .get(&point.item_id)
            .map(str_item_owner)
            .unwrap_or(0);
        if owner != serial {
            self.queue_system_text(
                character_id,
                "You can only jump to points you control.".to_string(),
            );
            return;
        }
        self.teleport_char_driver(character_id, point.x, point.y);
    }

    /// C `#list` (`strategy.c:3338-3360`).
    fn strategy_cmd_list(&mut self, character_id: CharacterId, ppd: &StrategyPpd) {
        if self.strategy_needs_boss(character_id, ppd) {
            return;
        }
        self.ensure_strategy_jump_points_initialized();
        let serial = self
            .characters
            .get(&character_id)
            .map(|c| c.serial)
            .unwrap_or(0);
        let max_jp = self.strategy_jump_points.points.len();
        let mut flag = false;
        for val in 1..max_jp {
            let point = self.strategy_jump_points.points[val];
            let Some(item) = self.items.get(&point.item_id) else {
                continue;
            };
            if str_item_owner(item) != serial {
                continue;
            }
            let name = item.name.clone();
            let gold = str_item_gold(item);
            let mut bytes = format!("JP {val}: ").into_bytes();
            bytes.push(0x03);
            bytes.extend_from_slice(name.as_bytes());
            bytes.push(b' ');
            bytes.push(0x10);
            bytes.extend_from_slice(format!("{gold} Platinum.").as_bytes());
            self.queue_system_text_bytes(character_id, bytes);
            flag = true;
        }
        if flag {
            self.queue_system_text(character_id, "Use /jp <nr> to teleport.".to_string());
        }
    }

    /// C `#info` (`strategy.c:3399-3445`).
    fn strategy_cmd_info(&mut self, character_id: CharacterId, ppd: &StrategyPpd) {
        if self.strategy_needs_boss(character_id, ppd) {
            return;
        }

        let mut header = vec![0x01u8];
        header.extend_from_slice(b"Name ");
        header.push(0x08);
        header.extend_from_slice(b"Value ");
        header.push(0x0D);
        header.extend_from_slice(b"Exp Cost");
        header.push(0x11);
        header.extend_from_slice(b"Increment");
        self.queue_system_text_bytes(character_id, header);

        let numbered: [(i32, &str, i32); 7] = [
            (1, "Base Income: ", ppd.income),
            (2, "Max Level: ", ppd.max_level),
            (3, "Max Worker: ", ppd.max_worker),
            (4, "Training Speed: ", ppd.trainspeed),
            (5, "Warcry Bonus: ", ppd.warcry),
            (6, "Endurance: ", ppd.endurance),
            (7, "Speed Bonus: ", ppd.speed),
        ];
        for (nr, label, value) in numbered {
            let cost = str_exp_cost(ppd, nr);
            let inc = str_increment(ppd, nr);
            let mut row = format!("{nr} ").into_bytes();
            row.push(0x01);
            row.extend_from_slice(label.as_bytes());
            row.push(0x09);
            row.extend_from_slice(format!("{value} ").as_bytes());
            row.push(0x0E);
            row.extend_from_slice(format!("{cost} ").as_bytes());
            row.push(0x12);
            row.extend_from_slice(format!("{inc}").as_bytes());
            self.queue_system_text_bytes(character_id, row);
        }

        {
            let mut row = b"- ".to_vec();
            row.push(0x01);
            row.extend_from_slice(b"Extra Guards: ");
            row.push(0x09);
            row.extend_from_slice(format!("{} ", ppd.eguards).as_bytes());
            row.push(0x0E);
            row.extend_from_slice(b"- ");
            row.push(0x12);
            row.extend_from_slice(b"-");
            self.queue_system_text_bytes(character_id, row);
        }

        {
            let cost = str_exp_cost(ppd, 8);
            let inc = str_increment(ppd, 8);
            let mut row = b"8 ".to_vec();
            row.push(0x01);
            row.extend_from_slice(b"Guard Level: ");
            row.push(0x09);
            row.extend_from_slice(format!("{} ", ppd.eguardlvl).as_bytes());
            row.push(0x0E);
            row.extend_from_slice(format!("{cost} ").as_bytes());
            row.push(0x12);
            row.extend_from_slice(format!("{inc}").as_bytes());
            self.queue_system_text_bytes(character_id, row);
        }

        for (label, value) in [
            ("Missions: ", ppd.mis_cnt),
            ("Victories: ", ppd.won_cnt),
            ("Experience: ", ppd.exp),
        ] {
            let mut row = b"- ".to_vec();
            row.push(0x01);
            row.extend_from_slice(label.as_bytes());
            row.push(0x09);
            row.extend_from_slice(format!("{value} ").as_bytes());
            row.push(0x0E);
            row.extend_from_slice(b"-");
            row.push(0x12);
            row.extend_from_slice(b"-");
            self.queue_system_text_bytes(character_id, row);
        }
    }

    /// C `#raise <nr>` (`strategy.c:3447-3462`), minus `str_raise`'s own
    /// `log_char` calls (already the case for the ported [`str_raise`] -
    /// this just renders its [`StrategyRaiseOutcome`] into text).
    fn strategy_cmd_raise(&mut self, character_id: CharacterId, ppd: &mut StrategyPpd, n: i32) {
        if self.strategy_needs_boss(character_id, ppd) {
            return;
        }
        if !(1..=9).contains(&n) {
            self.queue_system_text(character_id, "Number is out of bounds.".to_string());
            return;
        }
        match str_raise(ppd, n) {
            StrategyRaiseOutcome::CannotRaiseHigher => {
                self.queue_system_text(
                    character_id,
                    "You cannot raise this value any higher.".to_string(),
                );
            }
            StrategyRaiseOutcome::CannotAfford { .. } => {
                self.queue_system_text(
                    character_id,
                    "You cannot afford to raise this value.".to_string(),
                );
            }
            StrategyRaiseOutcome::Raised => {
                self.queue_system_text(character_id, "Done.".to_string());
            }
        }
    }

    /// C's god-only `#reset` (`strategy.c:3464-3468`): no boss-stage gate,
    /// no feedback text.
    fn strategy_cmd_reset(&mut self, ppd: &mut StrategyPpd) {
        ppd.init_done = 0;
    }

    /// C `#mission` (`strategy.c:3470-3513`).
    fn strategy_cmd_mission_list(&mut self, character_id: CharacterId, ppd: &StrategyPpd) {
        if self.strategy_needs_boss(character_id, ppd) {
            return;
        }
        self.ensure_strategy_areas_initialized();

        let mut header = crate::text::COL_LIGHT_RED.to_vec();
        header.extend_from_slice(b"Nr \x02Name \x05Busy \x07Solved \x0BEnemies ");
        self.queue_system_text_bytes(character_id, header);

        for (idx, mission) in MISSIONS.iter().enumerate() {
            if mission.need_solve != 0
                && ppd.solve_count(mission.need_solve as usize) == 0
                && ppd.solve_count(mission.need_solve2 as usize) == 0
            {
                continue;
            }
            if mission.set_solve != 0
                && i32::from(ppd.solve_count(mission.set_solve as usize)) >= MAXMISSIONTRY
            {
                continue;
            }
            let area_index = mission.area as usize;
            let has_spawn = self
                .strategy_areas
                .areas
                .get(area_index)
                .is_some_and(|area| !area.spawn.is_empty());
            if !has_spawn {
                continue;
            }

            self.queue_validate(area_index);
            let area = &self.strategy_areas.areas[area_index];
            let mut cnt = 0i32;
            let mut self_pos = 0i32;
            for n in 0..MAXQUEUE {
                if area.q_player_cn[n].is_some() {
                    cnt += 1;
                }
                if area.q_player_cn[n] == Some(character_id) {
                    self_pos = cnt;
                }
            }
            let busy = area.busy as i32 + cnt;
            let (slash_char, digit_char) = if self_pos != 0 {
                ('/', char::from(b'0' + self_pos as u8))
            } else {
                (' ', ' ')
            };
            let solve_val = if mission.set_solve != 0 {
                ppd.solve_count(mission.set_solve as usize)
            } else {
                0
            };
            let preset_name = |k: usize| -> &'static str {
                let preset_idx = mission.enemy.get(k).copied().unwrap_or(0) as usize;
                AI_PRESETS.get(preset_idx).map_or("", |p| p.name)
            };

            let mut row = crate::text::COL_LIGHT_GREEN.to_vec();
            row.extend_from_slice(
                format!(
                    " {} \x02{} \x06{}{}{} \x08{} \x0B{} {} {} {}",
                    idx + 1,
                    mission.name,
                    busy,
                    slash_char,
                    digit_char,
                    solve_val,
                    preset_name(0),
                    preset_name(1),
                    preset_name(2),
                    preset_name(3)
                )
                .as_bytes(),
            );
            self.queue_system_text_bytes(character_id, row);
        }

        self.queue_system_text(
            character_id,
            "Use /enter <nr> to start a mission. If that mission is busy your request will be queued."
                .to_string(),
        );
    }

    /// C `take_spawner(int in, int cn, struct strategy_ppd *ppd)`
    /// (`strategy.c:1296-1319`): claims a free spawner (and its paired
    /// storage item, [`Self::str_spawner_storage_item`]) for `character_id`,
    /// stamping both items' display names and caching the player's
    /// current income onto the storage item's `drdata[9]` (read by the
    /// still-unported `storage` item driver).
    fn strategy_take_spawner(
        &mut self,
        spawner_id: ItemId,
        character_id: CharacterId,
        ppd: &mut StrategyPpd,
    ) {
        let Some(character) = self.characters.get(&character_id) else {
            return;
        };
        if !character.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        let serial = character.serial;
        let name: String = character.name.chars().take(20).collect();

        let Some(spawner) = self.items.get(&spawner_id) else {
            return;
        };
        if str_item_owner(spawner) != STR_OWNER_NONE {
            return;
        }
        let Some(storage_id) = self.str_spawner_storage_item(spawner_id) else {
            self.queue_system_text(
                character_id,
                "Failed. Please report bug #25476g".to_string(),
            );
            return;
        };
        let spawner_slot = spawner.driver_data.get(8).copied().unwrap_or(0);
        let color = spawner.driver_data.get(10).copied().unwrap_or(0);

        if let Some(item) = self.items.get_mut(&spawner_id) {
            set_str_item_owner(item, serial);
            item.name = format!("{name}'s Spawner ({spawner_slot})");
        }
        if let Some(item) = self.items.get_mut(&storage_id) {
            set_str_item_owner(item, serial);
            // C: `it[in2].drdata[8]` - the *storage* item's own area-slot
            // byte, not the spawner's (they're always equal in real zone
            // data, but the C source reads each item's own byte).
            let storage_slot = item.driver_data.get(8).copied().unwrap_or(0);
            item.name = format!("{name}'s Storage ({storage_slot})");
            if item.driver_data.len() <= 9 {
                item.driver_data.resize(10, 0);
            }
            item.driver_data[9] = ppd.income.clamp(0, u8::MAX as i32) as u8;
        }
        self.queue_system_text(
            character_id,
            "You take control of this spawner. Use it again to create workers.".to_string(),
        );
        ppd.npc_color = i32::from(color);
    }

    /// C `#enter <nr>` (`strategy.c:3515-3592`).
    fn strategy_cmd_enter(&mut self, character_id: CharacterId, ppd: &mut StrategyPpd, n: i32) {
        if self.strategy_needs_boss(character_id, ppd) {
            return;
        }
        let out_of_bounds = |world: &mut World| {
            world.queue_system_text(character_id, "Mission number is out of bounds.".to_string());
        };
        let Some(idx) = usize::try_from(n - 1).ok() else {
            out_of_bounds(self);
            return;
        };
        let Some(mission) = MISSIONS.get(idx).copied() else {
            out_of_bounds(self);
            return;
        };
        if mission.need_solve != 0
            && ppd.solve_count(mission.need_solve as usize) == 0
            && ppd.solve_count(mission.need_solve2 as usize) == 0
        {
            out_of_bounds(self);
            return;
        }
        if mission.set_solve != 0
            && i32::from(ppd.solve_count(mission.set_solve as usize)) >= MAXMISSIONTRY
        {
            out_of_bounds(self);
            return;
        }
        self.ensure_strategy_areas_initialized();
        let area_index = mission.area as usize;
        let has_spawn = self
            .strategy_areas
            .areas
            .get(area_index)
            .is_some_and(|area| !area.spawn.is_empty());
        if !has_spawn {
            out_of_bounds(self);
            return;
        }

        let serial = self
            .characters
            .get(&character_id)
            .map(|c| c.serial)
            .unwrap_or(0);

        let mut item_ids: Vec<ItemId> = self
            .items
            .iter()
            .filter(|(_, item)| !item.flags.is_empty() && item.driver == IDR_STR_SPAWNER)
            .map(|(id, _)| *id)
            .collect();
        item_ids.sort_by_key(|id| id.0);

        let mut spawn: Option<ItemId> = None;
        let mut twoparty = 0;
        for item_id in item_ids {
            let Some(item) = self.items.get(&item_id) else {
                continue;
            };
            let slot = i32::from(item.driver_data.get(8).copied().unwrap_or(0));
            let owner = str_item_owner(item);
            if slot == mission.area {
                if owner == STR_OWNER_NONE {
                    spawn = Some(item_id);
                } else if owner < STR_OWNER_AI_UNASSIGNED {
                    twoparty += 1;
                }
            }
            if owner == serial {
                let (x, y) = (item.x, item.y);
                self.queue_remove(character_id);
                self.teleport_char_driver(character_id, x, y);
                self.queue_system_text(character_id, "Re-entering mission.".to_string());
                return;
            }
        }

        let Some(spawn_id) = spawn else {
            self.queue_mission(character_id, area_index);
            self.queue_system_text(
                character_id,
                "Mission area is busy. Request has been queued.".to_string(),
            );
            self.show_queue(character_id, area_index);
            return;
        };

        if !self.queue_check(character_id, area_index) {
            self.queue_system_text(
                character_id,
                "You are not the next one in the queue.".to_string(),
            );
            self.queue_mission(character_id, area_index);
            self.show_queue(character_id, area_index);
            return;
        }
        self.queue_remove(character_id);

        if twoparty == 0 {
            self.str_init_mission(idx);
        }

        ppd.mis_cnt += 1;
        ppd.current_mission = idx as i32;

        let Some(spawn_item) = self.items.get(&spawn_id) else {
            return;
        };
        let (x, y) = (spawn_item.x, spawn_item.y);
        self.teleport_char_driver(character_id, x, y);
        self.strategy_take_spawner(spawn_id, character_id, ppd);
    }

    /// C `#surrender` (`strategy.c:3593-3604`).
    fn strategy_cmd_surrender(&mut self, character_id: CharacterId, ppd: &StrategyPpd) {
        if self.strategy_needs_boss(character_id, ppd) {
            return;
        }
        let serial = self
            .characters
            .get(&character_id)
            .map(|c| c.serial)
            .unwrap_or(0);
        if !self.str_remove_party(serial, None) {
            self.queue_system_text(character_id, "You are not doing any mission.".to_string());
        }
    }

    /// C's god-only `#queue` debug dump (`strategy.c:3606-3622`).
    fn strategy_cmd_queue_debug(&mut self, character_id: CharacterId) {
        self.ensure_strategy_areas_initialized();
        let mut lines = Vec::new();
        for (area_index, area) in self.strategy_areas.areas.iter().enumerate() {
            for n in 0..MAXQUEUE {
                if let Some(cn) = area.q_player_cn[n] {
                    let id = area.q_player_id[n];
                    lines.push(format!(
                        "Area {area_index}, queue {n}, cn={}, ID={id}",
                        cn.0
                    ));
                }
            }
        }
        for line in lines {
            self.queue_system_text(character_id, line);
        }
    }

    /// C `special_driver(int nr, int cn, char *ptr)` (`strategy.c:3278-
    /// 3626`), minus the `nr != CDR_STRATEGY_PARSER`/`set_data` guards
    /// (`ppd` is already resolved by the caller - `ugaris-server` looks
    /// up the real `PlayerRuntime::strategy` since `World` can't reach
    /// session state) and minus the lazy jump-point-registry init's own
    /// `*ptr == '#' || *ptr == '/'` gate placement (hoisted to run
    /// unconditionally on every call here, matching C's own
    /// unconditional placement before that check). Returns whether a
    /// command matched (C's `return 2`); `false` (C's `return 1`) means
    /// the caller should treat `command` as ordinary chat text.
    pub fn apply_strategy_special_command(
        &mut self,
        character_id: CharacterId,
        area_id: u16,
        ppd: &mut StrategyPpd,
        command: &str,
    ) -> bool {
        if area_id != 23 && area_id != 24 {
            return false;
        }
        self.ensure_strategy_jump_points_initialized();

        let trimmed = command.trim_start();
        let Some(rest) = trimmed
            .strip_prefix('#')
            .or_else(|| trimmed.strip_prefix('/'))
        else {
            return false;
        };
        let mut reader = ArgReader::new(rest);
        let Some(word) = reader.take_str() else {
            return false;
        };
        let args = reader.remaining();
        let is_god = self
            .characters
            .get(&character_id)
            .is_some_and(|character| character.flags.contains(CharacterFlags::GOD));

        if cmd_word_matches(&word, "jp", 2) {
            let val = legacy_atoi(args) as i32;
            self.strategy_cmd_jp(character_id, ppd, val);
            return true;
        }
        if cmd_word_matches(&word, "list", 4) {
            self.strategy_cmd_list(character_id, ppd);
            return true;
        }
        // C: `#eguard` is next in the table here - deliberately not
        // ported, see this module's doc comment.
        if cmd_word_matches(&word, "info", 4) {
            self.strategy_cmd_info(character_id, ppd);
            return true;
        }
        if cmd_word_matches(&word, "raise", 4) {
            let n = legacy_atoi(args) as i32;
            self.strategy_cmd_raise(character_id, ppd, n);
            return true;
        }
        if is_god && cmd_word_matches(&word, "reset", 4) {
            self.strategy_cmd_reset(ppd);
            return true;
        }
        if cmd_word_matches(&word, "mission", 4) {
            self.strategy_cmd_mission_list(character_id, ppd);
            return true;
        }
        if cmd_word_matches(&word, "enter", 4) {
            let n = legacy_atoi(args) as i32;
            self.strategy_cmd_enter(character_id, ppd, n);
            return true;
        }
        if cmd_word_matches(&word, "surrender", 9) {
            self.strategy_cmd_surrender(character_id, ppd);
            return true;
        }
        if is_god && cmd_word_matches(&word, "queue", 5) {
            self.strategy_cmd_queue_debug(character_id);
            return true;
        }

        false
    }
}
