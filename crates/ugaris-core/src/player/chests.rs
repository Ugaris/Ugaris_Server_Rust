use super::*;

impl PlayerRuntime {
    pub fn chest_last_access_seconds(&self, treasure_index: u8) -> u64 {
        self.chest_last_access_seconds
            .get(&treasure_index)
            .copied()
            .unwrap_or_default()
    }

    pub fn mark_chest_access(&mut self, treasure_index: u8, realtime_seconds: u64) {
        self.chest_last_access_seconds
            .insert(treasure_index, realtime_seconds);
    }

    pub fn encode_legacy_treasure_chest_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_TREASURE_CHEST_PPD_SIZE];
        for (&treasure_index, &last_access_seconds) in &self.chest_last_access_seconds {
            let index = usize::from(treasure_index);
            if index >= TREASURE_CHEST_PPD_ENTRIES {
                continue;
            }
            write_i32(
                &mut bytes,
                index * 4,
                last_access_seconds.min(i32::MAX as u64) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_treasure_chest_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_TREASURE_CHEST_PPD_SIZE {
            return false;
        }

        self.chest_last_access_seconds.clear();
        for index in 0..TREASURE_CHEST_PPD_ENTRIES {
            let last_access_seconds = read_i32(bytes, index * 4);
            if last_access_seconds > 0 {
                self.chest_last_access_seconds
                    .insert(index as u8, last_access_seconds as u64);
            }
        }
        true
    }

    pub fn encode_legacy_randchest_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_RANDCHEST_PPD_SIZE];
        for (index, entry) in self
            .random_chests
            .iter()
            .take(RANDCHEST_MAX_ENTRIES)
            .enumerate()
        {
            write_i32(
                &mut bytes,
                RANDCHEST_PPD_IDS_OFFSET + index * 4,
                entry.location_id.min(i32::MAX as u32) as i32,
            );
            write_i32(
                &mut bytes,
                RANDCHEST_PPD_LAST_USED_OFFSET + index * 4,
                entry.last_used_seconds.min(i32::MAX as u64) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_randchest_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_RANDCHEST_PPD_SIZE {
            return false;
        }

        self.random_chests.clear();
        for index in 0..RANDCHEST_MAX_ENTRIES {
            let location_id = read_i32(bytes, RANDCHEST_PPD_IDS_OFFSET + index * 4);
            let last_used_seconds = read_i32(bytes, RANDCHEST_PPD_LAST_USED_OFFSET + index * 4);
            if location_id > 0 && last_used_seconds > 0 {
                self.random_chests.push(RandomChestAccess {
                    location_id: location_id as u32,
                    last_used_seconds: last_used_seconds as u64,
                });
            }
        }
        true
    }

    pub fn encode_legacy_ratchest_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_RATCHEST_PPD_SIZE];
        for (index, entry) in self
            .rat_chests
            .iter()
            .take(RATCHEST_MAX_ENTRIES)
            .enumerate()
        {
            write_i32(
                &mut bytes,
                RATCHEST_PPD_IDS_OFFSET + index * 4,
                entry.location_id.min(i32::MAX as u32) as i32,
            );
            write_i32(
                &mut bytes,
                RATCHEST_PPD_LAST_USED_OFFSET + index * 4,
                entry.last_used_seconds.min(i32::MAX as u64) as i32,
            );
        }
        write_i32(
            &mut bytes,
            RATCHEST_PPD_TREASURE_X_OFFSET,
            i32::from(self.rat_chest_treasure_x),
        );
        write_i32(
            &mut bytes,
            RATCHEST_PPD_TREASURE_Y_OFFSET,
            i32::from(self.rat_chest_treasure_y),
        );
        write_i32(
            &mut bytes,
            RATCHEST_PPD_LAST_TREASURE_OFFSET,
            self.rat_chest_last_treasure_seconds.min(i32::MAX as u64) as i32,
        );
        bytes
    }

    pub fn decode_legacy_ratchest_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_RATCHEST_PPD_SIZE {
            return false;
        }

        self.rat_chests.clear();
        for index in 0..RATCHEST_MAX_ENTRIES {
            let location_id = read_i32(bytes, RATCHEST_PPD_IDS_OFFSET + index * 4);
            let last_used_seconds = read_i32(bytes, RATCHEST_PPD_LAST_USED_OFFSET + index * 4);
            if location_id > 0 && last_used_seconds > 0 {
                self.rat_chests.push(RatChestAccess {
                    location_id: location_id as u32,
                    last_used_seconds: last_used_seconds as u64,
                });
            }
        }
        self.rat_chest_treasure_x = read_i32(bytes, RATCHEST_PPD_TREASURE_X_OFFSET).max(0) as u16;
        self.rat_chest_treasure_y = read_i32(bytes, RATCHEST_PPD_TREASURE_Y_OFFSET).max(0) as u16;
        self.rat_chest_last_treasure_seconds =
            read_i32(bytes, RATCHEST_PPD_LAST_TREASURE_OFFSET).max(0) as u64;
        true
    }

    pub fn record_chest_opened(&mut self, treasure_index: u8) {
        self.achievements.chests_opened = self.achievements.chests_opened.saturating_add(1);
        if self.achievements.chests_opened >= 10 {
            self.achievements.looter = true;
        }
        if self.achievements.chests_opened >= 50 {
            self.achievements.treasure_hunter = true;
        }
        if self.achievements.chests_opened >= 100 {
            self.achievements.treasure_master = true;
        }
        if self.achievements.chests_opened >= 500 {
            self.achievements.legendary_looter = true;
        }
        if treasure_index == 63 {
            self.achievements.gold_looter = true;
        }
    }

    pub fn random_chest_last_used_seconds(&self, location_id: u32) -> Option<u64> {
        self.random_chests
            .iter()
            .find(|entry| entry.location_id == location_id)
            .map(|entry| entry.last_used_seconds)
    }

    pub fn mark_random_chest_used(&mut self, location_id: u32, realtime_seconds: u64) {
        if let Some(entry) = self
            .random_chests
            .iter_mut()
            .find(|entry| entry.location_id == location_id)
        {
            entry.last_used_seconds = realtime_seconds;
            return;
        }
        if self.random_chests.len() < RANDCHEST_MAX_ENTRIES {
            self.random_chests.push(RandomChestAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            });
            return;
        }
        if let Some(oldest) = self
            .random_chests
            .iter_mut()
            .min_by_key(|entry| entry.last_used_seconds)
        {
            *oldest = RandomChestAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            };
        }
    }

    pub fn rat_chest_last_used_seconds(&self, location_id: u32) -> Option<u64> {
        self.rat_chests
            .iter()
            .find(|entry| entry.location_id == location_id)
            .map(|entry| entry.last_used_seconds)
    }

    pub fn mark_rat_chest_used(&mut self, location_id: u32, realtime_seconds: u64) {
        if let Some(entry) = self
            .rat_chests
            .iter_mut()
            .find(|entry| entry.location_id == location_id)
        {
            entry.last_used_seconds = realtime_seconds;
            return;
        }
        if self.rat_chests.len() < RATCHEST_MAX_ENTRIES {
            self.rat_chests.push(RatChestAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            });
            return;
        }
        if let Some(oldest) = self
            .rat_chests
            .iter_mut()
            .min_by_key(|entry| entry.last_used_seconds)
        {
            *oldest = RatChestAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            };
        }
    }
}
