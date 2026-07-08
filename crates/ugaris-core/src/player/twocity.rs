use super::*;

impl PlayerRuntime {
    pub fn ensure_twocity_goodtile_with<F>(&mut self, mut roll_color: F) -> [u8; 5]
    where
        F: FnMut() -> u8,
    {
        if self.twocity_goodtile[0] == 0 {
            for color in &mut self.twocity_goodtile {
                *color = roll_color().clamp(1, 6);
            }
        }
        self.twocity_goodtile
    }

    pub fn set_twocity_thief_state(&mut self, state: i32) {
        self.twocity_ppd.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        write_i32(&mut self.twocity_ppd, TWOCITY_PPD_THIEF_STATE_OFFSET, state);
    }

    pub fn twocity_thief_state(&self) -> i32 {
        if self.twocity_ppd.len() < LEGACY_TWOCITY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.twocity_ppd, TWOCITY_PPD_THIEF_STATE_OFFSET)
    }

    pub fn twocity_thief_killed(&self, index: usize) -> i32 {
        if index >= 6 || self.twocity_ppd.len() < LEGACY_TWOCITY_PPD_SIZE {
            return 0;
        }
        read_i32(
            &self.twocity_ppd,
            TWOCITY_PPD_THIEF_KILLED_OFFSET + index * 4,
        )
    }

    pub fn twocity_sanwyn_state(&self) -> i32 {
        if self.twocity_ppd.len() < LEGACY_TWOCITY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.twocity_ppd, TWOCITY_PPD_SANWYN_STATE_OFFSET)
    }

    pub fn set_twocity_sanwyn_state(&mut self, state: i32) {
        self.twocity_ppd.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        write_i32(
            &mut self.twocity_ppd,
            TWOCITY_PPD_SANWYN_STATE_OFFSET,
            state,
        );
    }

    /// C `struct twocity_ppd::sanwyn_bits` (`common/two_ppd.h:22`): a
    /// 3-bit mask of which of the three incriminating palace notes have
    /// already been turned in to Sanwyn (`1`/`2`/`4`).
    pub fn twocity_sanwyn_bits(&self) -> i32 {
        if self.twocity_ppd.len() < LEGACY_TWOCITY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.twocity_ppd, TWOCITY_PPD_SANWYN_BITS_OFFSET)
    }

    pub fn set_twocity_sanwyn_bits(&mut self, bits: i32) {
        self.twocity_ppd.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        write_i32(&mut self.twocity_ppd, TWOCITY_PPD_SANWYN_BITS_OFFSET, bits);
    }

    pub fn twocity_skelly_state(&self) -> i32 {
        if self.twocity_ppd.len() < LEGACY_TWOCITY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.twocity_ppd, TWOCITY_PPD_SKELLY_STATE_OFFSET)
    }

    pub fn set_twocity_skelly_state(&mut self, state: i32) {
        self.twocity_ppd.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        write_i32(
            &mut self.twocity_ppd,
            TWOCITY_PPD_SKELLY_STATE_OFFSET,
            state,
        );
    }

    pub fn twocity_alchemist_state(&self) -> i32 {
        if self.twocity_ppd.len() < LEGACY_TWOCITY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.twocity_ppd, TWOCITY_PPD_ALCHEMIST_STATE_OFFSET)
    }

    pub fn set_twocity_alchemist_state(&mut self, state: i32) {
        self.twocity_ppd.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        write_i32(
            &mut self.twocity_ppd,
            TWOCITY_PPD_ALCHEMIST_STATE_OFFSET,
            state,
        );
    }

    /// Snapshot of the `twocity_ppd` fields consumed by
    /// `questlog_init_twocity` (`src/system/questlog.c:1470-1546`), for
    /// `crate::quest::init_twocity_quests`.
    pub fn twocity_quest_state(&self) -> crate::quest::TwocityQuestState {
        crate::quest::TwocityQuestState {
            thief_state: self.twocity_thief_state(),
            sanwyn_state: self.twocity_sanwyn_state(),
            skelly_state: self.twocity_skelly_state(),
            alchemist_state: self.twocity_alchemist_state(),
        }
    }

    pub fn mark_twocity_burndown_kill(&mut self) -> bool {
        self.twocity_ppd.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        let thief_state = read_i32(&self.twocity_ppd, TWOCITY_PPD_THIEF_STATE_OFFSET);
        if thief_state != 13 && thief_state != 14 {
            return false;
        }
        write_i32(&mut self.twocity_ppd, TWOCITY_PPD_THIEF_STATE_OFFSET, 14);
        let killed = read_i32(&self.twocity_ppd, TWOCITY_PPD_THIEF_KILLED_OFFSET);
        write_i32(
            &mut self.twocity_ppd,
            TWOCITY_PPD_THIEF_KILLED_OFFSET,
            killed.saturating_add(1),
        );
        true
    }

    pub fn encode_legacy_twocity_ppd(&self) -> Vec<u8> {
        let mut bytes = self.twocity_ppd.clone();
        bytes.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        for (index, color) in self.twocity_goodtile.iter().copied().enumerate() {
            write_i32(
                &mut bytes,
                TWOCITY_PPD_GOODTILE_OFFSET + index * 4,
                color as i32,
            );
        }
        write_i32(
            &mut bytes,
            TWOCITY_PPD_SOLVED_LIBRARY_OFFSET,
            i32::from(self.twocity_solved_library),
        );
        bytes
    }

    pub fn decode_legacy_twocity_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_TWOCITY_PPD_SIZE {
            return false;
        }
        self.twocity_ppd = bytes[..LEGACY_TWOCITY_PPD_SIZE].to_vec();
        for index in 0..self.twocity_goodtile.len() {
            let color = read_i32(bytes, TWOCITY_PPD_GOODTILE_OFFSET + index * 4);
            self.twocity_goodtile[index] = if (0..=u8::MAX as i32).contains(&color) {
                color as u8
            } else {
                0
            };
        }
        self.twocity_solved_library = read_i32(bytes, TWOCITY_PPD_SOLVED_LIBRARY_OFFSET) != 0;
        true
    }
}
