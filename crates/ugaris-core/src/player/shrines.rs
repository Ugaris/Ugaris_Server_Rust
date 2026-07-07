use super::*;

impl PlayerRuntime {
    pub fn touch_special_shrine(
        &mut self,
        character: &mut Character,
        kind: u8,
        realtime_seconds: u64,
    ) -> SpecialShrineResult {
        if kind != 0x0A {
            return SpecialShrineResult::Unsupported;
        }
        if !character.flags.contains(CharacterFlags::HARDCORE)
            || character.creation_time > SPECIAL_SHRINE_HCSC_CUTOFF_SECONDS
        {
            return SpecialShrineResult::NothingHere;
        }
        if self.special_shrine_hcsc_last_touch_seconds == 0
            || realtime_seconds.saturating_sub(self.special_shrine_hcsc_last_touch_seconds)
                > SPECIAL_SHRINE_CONFIRM_WINDOW_SECONDS
        {
            self.special_shrine_hcsc_last_touch_seconds = realtime_seconds;
            return SpecialShrineResult::ConfirmRequired;
        }

        character.flags.remove(CharacterFlags::HARDCORE);
        self.special_shrine_hcsc_last_touch_seconds = 0;
        SpecialShrineResult::HardcoreRemoved
    }

    pub fn encode_legacy_demonshrine_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_DEMONSHRINE_PPD_SIZE];
        for (index, location_id) in self
            .demonshrines
            .iter()
            .copied()
            .take(DEMONSHRINE_MAX_ENTRIES)
            .enumerate()
        {
            write_i32(
                &mut bytes,
                index * 4,
                location_id.min(i32::MAX as u32) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_demonshrine_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_DEMONSHRINE_PPD_SIZE {
            return false;
        }

        self.demonshrines.clear();
        for index in 0..DEMONSHRINE_MAX_ENTRIES {
            let location_id = read_i32(bytes, index * 4);
            if location_id > 0 {
                self.demonshrines.push(location_id as u32);
            }
        }
        true
    }

    pub fn encode_legacy_randomshrine_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_RANDOMSHRINE_PPD_SIZE];
        for (index, word) in self.random_shrine_used_words.iter().copied().enumerate() {
            write_u32(&mut bytes, index * 4, word);
        }
        bytes[RANDOMSHRINE_USED_WORDS * 4] = self.random_shrine_continuity;
        bytes
    }

    pub fn decode_legacy_randomshrine_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_RANDOMSHRINE_PPD_SIZE {
            return false;
        }
        for index in 0..RANDOMSHRINE_USED_WORDS {
            self.random_shrine_used_words[index] = read_u32(bytes, index * 4);
        }
        self.random_shrine_continuity = bytes[RANDOMSHRINE_USED_WORDS * 4];
        true
    }

    pub fn has_used_random_shrine(&self, shrine: u8) -> bool {
        let word = usize::from(shrine / 32);
        let bit = 1u32 << (shrine & 31);
        self.random_shrine_used_words[word] & bit != 0
    }

    pub fn mark_random_shrine_used(&mut self, shrine: u8) {
        let word = usize::from(shrine / 32);
        let bit = 1u32 << (shrine & 31);
        self.random_shrine_used_words[word] |= bit;
    }

    /// C `cmd_clearrd`'s per-bit clear (`command.c:1888-1932`): the
    /// counterpart to [`Self::mark_random_shrine_used`] that unsets a
    /// single shrine's "used" bit instead of setting it.
    pub fn clear_random_shrine_used(&mut self, shrine: u8) {
        let word = usize::from(shrine / 32);
        let bit = 1u32 << (shrine & 31);
        self.random_shrine_used_words[word] &= !bit;
    }

    pub fn memorize_park_shrine(&mut self, shrine: u8) -> Option<bool> {
        let offset = match shrine {
            1 => AREA3_PPD_KELLY_FOUND1_OFFSET,
            2 => AREA3_PPD_KELLY_FOUND2_OFFSET,
            3 => AREA3_PPD_KELLY_FOUND3_OFFSET,
            _ => return None,
        };
        if self.area3_ppd.len() < LEGACY_AREA3_PPD_SIZE {
            self.area3_ppd.resize(LEGACY_AREA3_PPD_SIZE, 0);
        }
        let was_new = read_i32(&self.area3_ppd, offset) == 0;
        write_i32(&mut self.area3_ppd, offset, 1);
        Some(was_new)
    }

    pub fn touch_demonshrine(
        &mut self,
        character: &mut Character,
        location_id: u32,
    ) -> DemonShrineResult {
        if self.demonshrines.iter().any(|&id| id == location_id) {
            return DemonShrineResult::AlreadyKnown;
        }
        if self.demonshrines.len() >= DEMONSHRINE_MAX_ENTRIES {
            return DemonShrineResult::Full;
        }

        self.demonshrines.push(location_id);
        let demon_index = CharacterValue::Demon as usize;
        let demon_value = character
            .values
            .get_mut(1)
            .and_then(|values| values.get_mut(demon_index));
        let new_demon = if let Some(value) = demon_value {
            *value = value.saturating_add(1);
            u32::from((*value).max(0) as u16)
        } else {
            0
        };
        let exp_added =
            (250_u32.saturating_add(new_demon.saturating_mul(100))).min(character.exp / 25);
        // C `demonshrine_driver` (`base.c:3231-3235`) also calls
        // `update_char(cn)` (Demon value changed) and `give_exp(cn, ...)`
        // after this point; this function only has `&mut Character`
        // (`PlayerData` is not `World`), so both are applied by the caller
        // (`World::give_exp`/`World::update_character`) using the returned
        // `exp_added`, matching the `ItemDriverOutcome::LollipopLicked`
        // pattern in `world/item_outcomes.rs`.
        character.flags.insert(CharacterFlags::ITEMS);
        DemonShrineResult::Learned { exp_added }
    }
}
