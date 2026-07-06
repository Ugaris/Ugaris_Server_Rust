use super::*;

pub(crate) const EDEMON_GATE_MODE0_POSITIONS: [(u16, u16); 7] = [
    (62, 157),
    (62, 164),
    (62, 174),
    (62, 184),
    (62, 191),
    (56, 174),
    (67, 174),
];

pub(crate) const EDEMON_GATE_MODE1_SLOT_BASE: usize = 404;

impl World {
    pub fn add_character(&mut self, mut character: Character) {
        // Backfill the independent `DRD_FIGHTDRIVER` slot
        // (`Character::fight_driver`) from a `SimpleBaddyDriverData` that
        // was constructed directly (bypassing
        // `apply_simple_baddy_create_message`, the normal zone-load path
        // that already seeds it) - e.g. hand-built test fixtures, or a
        // pre-migration DB save deserialized with `fight_driver: None`.
        // Mirrors C's `fight_driver_set_dist` always being able to derive
        // the same distances from `simple_baddy`'s own copy.
        if character.fight_driver.is_none() {
            if let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_ref() {
                character.fight_driver = Some(FightDriverData {
                    enemies: data.enemies.clone(),
                    start_dist: data.startdist,
                    stop_dist: data.stopdist,
                    char_dist: data.chardist,
                    home_x: data.home_x,
                    home_y: data.home_y,
                    last_hit: data.last_hit,
                });
            }
        }
        if self
            .map
            .tile(usize::from(character.x), usize::from(character.y))
            .is_some_and(|tile| tile.character == character.id.0 as u16)
        {
            add_character_light(&mut self.map, &character);
            self.mark_character_light_area(&character);
        }
        self.characters.insert(character.id, character);
    }

    pub fn spawn_character(&mut self, mut character: Character, x: usize, y: usize) -> bool {
        if self.characters.contains_key(&character.id) {
            return false;
        }
        if !self.map.drop_char(&mut character, x, y) {
            return false;
        }
        self.add_character(character);
        true
    }

    pub fn spawn_character_from_item_drop(
        &mut self,
        mut character: Character,
        item_id: ItemId,
    ) -> Option<(u16, u16)> {
        if self.characters.contains_key(&character.id) {
            return None;
        }
        let item = self.items.get(&item_id)?.clone();
        if !self.map.drop_char_from_item(&mut character, &item) {
            return None;
        }
        let placed = (character.x, character.y);
        self.add_character(character);
        Some(placed)
    }

    pub fn apply_edemon_gate_spawn_result(
        &mut self,
        item_id: ItemId,
        slot: usize,
        character_id: CharacterId,
        serial: u32,
    ) -> bool {
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        let mode = item.driver_data.first().copied().unwrap_or_default();
        let offset = edemon_gate_slot_offset(mode, slot);
        item.driver_data.resize(offset + 4, 0);
        let character_id = character_id.0 as u16;
        let serial = serial as u16;
        item.driver_data[offset..offset + 2].copy_from_slice(&character_id.to_le_bytes());
        item.driver_data[offset + 2..offset + 4].copy_from_slice(&serial.to_le_bytes());
        true
    }

    pub fn apply_chestspawn_spawn_result(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        _serial: u32,
    ) -> bool {
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        item.driver_data.resize(8, 0);
        if item.driver_data[1] != 0 {
            return false;
        }
        item.sprite += 1;
        item.driver_data[1] = 1;
        item.driver_data[2..4].copy_from_slice(&(character_id.0 as u16).to_le_bytes());
        item.driver_data[6..8].copy_from_slice(&0_u16.to_le_bytes());
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        self.mark_dirty_sector(x, y);
        self.schedule_item_driver_timer(item_id, CharacterId(0), TICKS_PER_SECOND * 10);
        true
    }

    pub fn apply_swampspawn_spawn_result(
        &mut self,
        item_id: ItemId,
        character_id: CharacterId,
        serial: u32,
    ) -> bool {
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        item.driver_data.resize(20, 0);
        item.driver_data[4..8].copy_from_slice(&character_id.0.to_le_bytes());
        item.driver_data[8..12].copy_from_slice(&serial.to_le_bytes());
        item.driver_data[12..16].copy_from_slice(&(self.tick.0 as u32).to_le_bytes());
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        self.mark_dirty_sector(x, y);
        true
    }

    pub(crate) fn chestspawn_spawn_alive(&self, character_id: CharacterId) -> bool {
        character_id.0 != 0
            && self
                .characters
                .get(&character_id)
                .is_some_and(|character| !character.flags.contains(CharacterFlags::DEAD))
    }

    pub(crate) fn reset_chestspawn_item(&mut self, item_id: ItemId) -> bool {
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        item.driver_data.resize(8, 0);
        if item.driver_data[1] == 0 {
            return false;
        }
        item.sprite -= 1;
        item.driver_data[1] = 0;
        let x = usize::from(item.x);
        let y = usize::from(item.y);
        self.mark_dirty_sector(x, y);
        true
    }

    pub fn apply_fdemon_gate_spawn_result(
        &mut self,
        item_id: ItemId,
        slot: usize,
        character_id: CharacterId,
        serial: u32,
    ) -> bool {
        let Some(item) = self.items.get_mut(&item_id) else {
            return false;
        };
        if slot >= 3 {
            return false;
        }
        let offset = fdemon_gate_slot_offset(slot);
        item.driver_data.resize(offset + 4, 0);
        let character_id = character_id.0 as u16;
        let serial = serial as u16;
        item.driver_data[offset..offset + 2].copy_from_slice(&character_id.to_le_bytes());
        item.driver_data[offset + 2..offset + 4].copy_from_slice(&serial.to_le_bytes());
        true
    }

    pub(crate) fn edemon_gate_spawn_context(
        &self,
        item_id: ItemId,
    ) -> Option<EdemonGateSpawnContext> {
        let item = self.items.get(&item_id)?;
        match item.driver_data.first().copied().unwrap_or_default() {
            0 => EDEMON_GATE_MODE0_POSITIONS
                .iter()
                .copied()
                .enumerate()
                .find_map(|(slot, (x, y))| {
                    self.edemon_gate_slot_is_stale(item, 0, slot)
                        .then_some(EdemonGateSpawnContext { slot, x, y })
                }),
            1 => {
                let mut positions = self
                    .items
                    .values()
                    .filter(|candidate| {
                        candidate.driver == IDR_EDEMONLIGHT
                            && candidate.driver_data.first() == Some(&4)
                    })
                    .map(|candidate| (candidate.id, candidate.x, candidate.y))
                    .collect::<Vec<_>>();
                positions.sort_by_key(|(id, _, _)| id.0);
                positions
                    .into_iter()
                    .take(100)
                    .enumerate()
                    .find_map(|(slot, (_, x, y))| {
                        self.edemon_gate_slot_is_stale(item, 1, slot)
                            .then_some(EdemonGateSpawnContext { slot, x, y })
                    })
            }
            _ => None,
        }
    }

    pub(crate) fn edemon_gate_slot_is_stale(&self, item: &Item, mode: u8, slot: usize) -> bool {
        let offset = edemon_gate_slot_offset(mode, slot);
        let Some(bytes) = item.driver_data.get(offset..offset + 4) else {
            return true;
        };
        let character_id = u16::from_le_bytes([bytes[0], bytes[1]]);
        let serial = u16::from_le_bytes([bytes[2], bytes[3]]);
        if character_id == 0 {
            return true;
        }
        self.characters
            .get(&CharacterId(u32::from(character_id)))
            .is_none_or(|character| {
                !character.flags.contains(CharacterFlags::USED)
                    || character.flags.contains(CharacterFlags::DEAD)
                    || character.serial as u16 != serial
            })
    }

    pub(crate) fn fdemon_gate_spawn_context(
        &self,
        item_id: ItemId,
    ) -> Option<FdemonGateSpawnContext> {
        let item = self.items.get(&item_id)?;
        (0..3).find_map(|slot| {
            self.fdemon_gate_slot_is_stale(item, slot)
                .then_some(FdemonGateSpawnContext {
                    slot,
                    x: item.x,
                    y: item.y,
                })
        })
    }

    pub(crate) fn fdemon_gate_slot_is_stale(&self, item: &Item, slot: usize) -> bool {
        let offset = fdemon_gate_slot_offset(slot);
        let Some(bytes) = item.driver_data.get(offset..offset + 4) else {
            return true;
        };
        let character_id = u16::from_le_bytes([bytes[0], bytes[1]]);
        let serial = u16::from_le_bytes([bytes[2], bytes[3]]);
        if character_id == 0 {
            return true;
        }
        self.characters
            .get(&CharacterId(u32::from(character_id)))
            .is_none_or(|character| {
                !character.flags.contains(CharacterFlags::USED)
                    || character.flags.contains(CharacterFlags::DEAD)
                    || character.serial as u16 != serial
            })
    }

    pub fn remove_character(&mut self, character_id: CharacterId) -> Option<Character> {
        let mut character = self.characters.remove(&character_id)?;
        let old_x = usize::from(character.x);
        let old_y = usize::from(character.y);
        remove_character_light(&mut self.map, &character);
        self.mark_character_light_area(&character);
        self.map.remove_char(&mut character);
        self.mark_dirty_sector(old_x, old_y);
        Some(character)
    }
}

pub(crate) fn edemon_gate_slot_offset(mode: u8, slot: usize) -> usize {
    match mode {
        0 => 4 + slot * 4,
        1 => EDEMON_GATE_MODE1_SLOT_BASE + slot * 4,
        _ => 4 + slot * 4,
    }
}

pub(crate) fn fdemon_gate_slot_offset(slot: usize) -> usize {
    4 + slot * 4
}
