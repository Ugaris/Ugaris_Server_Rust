use super::*;

impl World {
    /// C `cmd_questlevel` (`lq.c:2393-2428`).
    pub(super) fn lq_admin_cmd_questlevel(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/questlevel <min:int> <max:int>";
        let mut reader = ArgReader::new(args);
        let Some(min_level) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing mini. Usage is: {USAGE}."));
            return;
        };
        let Some(max_level) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing maxi. Usage is: {USAGE}."));
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        if !(1..=200).contains(&min_level) {
            self.queue_lq_error(character_id, "Min Level is out of bounds (1 to 200).");
            return;
        }
        if !(1..=200).contains(&max_level) {
            self.queue_lq_error(character_id, "Max Level is out of bounds (1 to 200).");
            return;
        }
        if min_level > max_level {
            self.queue_lq_error(character_id, "Min Level cannot be greater than Max Level.");
            return;
        }
        self.lq_data.min_level = min_level as u32;
        self.lq_data.max_level = max_level as u32;
        self.queue_system_text(
            character_id,
            format!("Set min level to {min_level} and max level to {max_level}."),
        );
    }

    /// C `cmd_questreward` (`lq.c:2357-2391`).
    pub(super) fn lq_admin_cmd_questreward(&mut self, character_id: CharacterId, args: &str) {
        const USAGE: &str = "/questreward <nr:int> <amount:int> <desc:str>";
        let mut reader = ArgReader::new(args);
        let Some(nr) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing nr. Usage is: {USAGE}."));
            return;
        };
        let Some(amount) = reader.take_int() else {
            self.queue_lq_error(character_id, format!("Missing amount. Usage is: {USAGE}."));
            return;
        };
        let Some(mut desc) = reader.take_str() else {
            self.queue_lq_error(
                character_id,
                format!("Missing description. Usage is: {USAGE}."),
            );
            return;
        };
        if reader.has_trailing_garbage() {
            self.queue_lq_error(
                character_id,
                format!("Trailing garbage. Usage is: {USAGE}."),
            );
            return;
        }
        if nr < 1 || nr >= MAXLQMARK as i64 {
            self.queue_lq_error(
                character_id,
                format!("Nr is out of bounds (1-{})", MAXLQMARK - 1),
            );
            return;
        }
        if !(1..=100).contains(&amount) {
            self.queue_lq_error(
                character_id,
                "Amount is out of bounds. It must be in the range 1..100. (Percentage of maximum allowed reward).",
            );
            return;
        }
        // C `char desc[sizeof(lq_data.reward_desc[0])]` = `char[80]`.
        desc.truncate(79);
        let idx = nr as usize;
        self.lq_data.reward[idx] = amount as i32;
        self.lq_data.reward_desc[idx] = desc.clone();
        self.queue_system_text(
            character_id,
            format!("Set reward for mark {desc} ({nr}) to {amount} exp."),
        );
    }

    /// C `cmd_questshow` (`lq.c:2430-2441`). Like `#doorlist`, C never
    /// validates `ptr` here - any extra text after `/questshow` is
    /// silently ignored.
    pub(super) fn lq_admin_cmd_questshow(&mut self, character_id: CharacterId, _args: &str) {
        self.queue_system_text(
            character_id,
            format!("Min level: {}", self.lq_data.min_level),
        );
        self.queue_system_text(
            character_id,
            format!("Max level: {}", self.lq_data.max_level),
        );
        for n in 1..MAXLQMARK {
            if self.lq_data.reward[n] != 0 {
                self.queue_system_text(
                    character_id,
                    format!(
                        "Reward for mark {} ({}) is {} exp.",
                        self.lq_data.reward_desc[n], n, self.lq_data.reward[n]
                    ),
                );
            }
        }
    }

    /// C `cmd_questentrance` (`lq.c:2336-2341`). Like `#questshow`, C
    /// never validates `ptr`.
    pub(super) fn lq_admin_cmd_questentrance(&mut self, character_id: CharacterId, _args: &str) {
        let Some(character) = self.characters.get(&character_id) else {
            return;
        };
        self.lq_data.entrance_x = character.x;
        self.lq_data.entrance_y = character.y;
        self.queue_system_text(character_id, "Set quest entrance.");
    }

    /// C `cmd_queststart` (`lq.c:2343-2355`). Unlike most of this table,
    /// C's own "not ready yet" replies here have no `COL_LIGHT_RED` prefix.
    pub(super) fn lq_admin_cmd_queststart(&mut self, character_id: CharacterId, _args: &str) {
        if self.lq_data.min_level == 0 || self.lq_data.max_level == 0 {
            self.queue_system_text(
                character_id,
                "You have to set min/max levels first (/questlevel)",
            );
            return;
        }
        if self.lq_data.entrance_x == 0 || self.lq_data.entrance_y == 0 {
            self.queue_system_text(
                character_id,
                "You have to set entrance position first (/questentrance)",
            );
            return;
        }
        self.lq_data.open = true;
        self.queue_system_text(character_id, "Quest starts...");
    }

    /// C `remove_item_map(n); drop_item(n, x, y)` sequence used only by
    /// `#questreset`'s player-body branch (`lq.c:2299-2306`): unlike
    /// [`World::drop_body_item`] (used for a freshly-created body that was
    /// never registered on the map), this item is already placed on the
    /// map, so its old tile registration and light must be cleared first.
    pub(super) fn lq_reset_drop_body_item(&mut self, item_id: ItemId) -> bool {
        let Some(mut item) = self.items.remove(&item_id) else {
            return false;
        };
        remove_item_light(&mut self.map, &item);
        self.map.remove_item_map(&mut item);
        for (x, y) in QUESTRESET_FALLBACK_POSITIONS {
            if self
                .map
                .drop_item(&mut item, usize::from(x), usize::from(y))
            {
                add_item_light(&mut self.map, &item);
                self.mark_item_light_area(&item);
                let (ix, iy) = (usize::from(item.x), usize::from(item.y));
                self.items.insert(item_id, item);
                self.mark_dirty_sector(ix, iy);
                return true;
            }
        }
        // C leaves the item off the map entirely if every fallback spot is
        // blocked (`remove_item_map` already zeroed `it[n].x/y`, and
        // `drop_item` never restores them on failure) - the item still
        // exists, just landless, matching that exactly.
        self.items.insert(item_id, item);
        false
    }

    /// C `cmd_questreset` (`lq.c:2278-2321`): wipes the Live Quest area
    /// back to a blank slate - removes every live `CDR_LQNPC` character,
    /// evicts every player to one of 7 fallback positions, clears every
    /// `IF_PLAYERBODY`/`IF_TAKE` item off the map, and resets every quest
    /// registry (`lq_npcs`/`lq_npc_respawns`/`lq_doors`+
    /// `lq_doors_initialized`/`lq_data`). Like `#questshow`, C never
    /// validates `ptr`. Note this also evicts the issuing admin
    /// themselves if they are a player standing in the reset area -
    /// matching C's unconditional `ch[n].flags & CF_PLAYER` scan.
    pub(super) fn lq_admin_cmd_questreset(&mut self, character_id: CharacterId, _args: &str) {
        let character_ids: Vec<CharacterId> = self.characters.keys().copied().collect();
        for id in character_ids {
            let Some((driver, is_player)) = self.characters.get(&id).map(|character| {
                (
                    character.driver,
                    character.flags.contains(CharacterFlags::PLAYER),
                )
            }) else {
                continue;
            };
            if driver == CDR_LQNPC {
                self.remove_character(id);
            } else if is_player {
                let moved = QUESTRESET_FALLBACK_POSITIONS
                    .iter()
                    .any(|&(x, y)| self.teleport_char_driver(id, x, y));
                if !moved {
                    self.queue_lq_error(
                        character_id,
                        "Could not remove all players, please try again soon.",
                    );
                }
            }
        }

        let item_ids: Vec<ItemId> = self
            .items
            .iter()
            .filter(|(_, item)| item.x != 0)
            .map(|(item_id, _)| *item_id)
            .collect();
        for item_id in item_ids {
            let Some((is_body, is_take)) = self.items.get(&item_id).map(|item| {
                (
                    item.flags.contains(ItemFlags::PLAYERBODY),
                    item.flags.contains(ItemFlags::TAKE),
                )
            }) else {
                continue;
            };
            if is_body {
                if self.lq_reset_drop_body_item(item_id) {
                    self.set_item_expire(item_id, self.settings.item_decay_time.max(1) as u64);
                } else {
                    self.queue_lq_error(
                        character_id,
                        "Could not remove all player bodies, please try again soon.",
                    );
                }
            } else if is_take {
                self.destroy_item(item_id);
            }
        }

        self.lq_npcs.clear();
        self.lq_npc_respawns.clear();
        self.lq_doors.clear();
        self.lq_doors_initialized = false;
        self.lq_data = LqData::default();

        self.queue_system_text(character_id, "Done.");
    }
}

/// C's 7-position fallback spot list used by both the player-teleport and
/// item-drop halves of `#questreset` (`lq.c:2285-2288`/`2300-2302`).
const QUESTRESET_FALLBACK_POSITIONS: [(u16, u16); 7] = [
    (240, 240),
    (235, 240),
    (240, 235),
    (235, 235),
    (245, 240),
    (240, 245),
    (245, 245),
];
