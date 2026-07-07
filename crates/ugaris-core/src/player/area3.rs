use super::*;

impl PlayerRuntime {
    pub fn encode_legacy_area3_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_AREA3_PPD_SIZE];
        let copy_len = self.area3_ppd.len().min(LEGACY_AREA3_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.area3_ppd[..copy_len]);
        bytes
    }

    pub fn decode_legacy_area3_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_AREA3_PPD_SIZE {
            return false;
        }
        self.area3_ppd = bytes[..LEGACY_AREA3_PPD_SIZE].to_vec();
        true
    }

    pub fn area3_imp_flags(&self) -> u32 {
        if self.area3_ppd.len() < LEGACY_AREA3_PPD_SIZE {
            return 0;
        }
        read_i32(&self.area3_ppd, AREA3_PPD_IMP_FLAGS_OFFSET).max(0) as u32
    }

    pub fn area3_kelly_state(&self) -> i32 {
        self.read_area3_i32(AREA3_PPD_KELLY_STATE_OFFSET)
    }

    pub fn set_area3_kelly_state(&mut self, state: i32) {
        self.write_area3_i32(AREA3_PPD_KELLY_STATE_OFFSET, state);
    }

    pub fn area3_clara_state(&self) -> i32 {
        self.read_area3_i32(AREA3_PPD_CLARA_STATE_OFFSET)
    }

    pub fn set_area3_clara_state(&mut self, state: i32) {
        self.write_area3_i32(AREA3_PPD_CLARA_STATE_OFFSET, state);
    }

    pub fn area3_seymour_state(&self) -> i32 {
        self.read_area3_i32(AREA3_PPD_SEYMOUR_STATE_OFFSET)
    }

    pub fn set_area3_seymour_state(&mut self, state: i32) {
        self.write_area3_i32(AREA3_PPD_SEYMOUR_STATE_OFFSET, state);
    }

    pub fn area3_astro2_state(&self) -> i32 {
        self.read_area3_i32(AREA3_PPD_ASTRO2_STATE_OFFSET)
    }

    pub fn set_area3_astro2_state(&mut self, state: i32) {
        self.write_area3_i32(AREA3_PPD_ASTRO2_STATE_OFFSET, state);
    }

    pub fn area3_crypt_state(&self) -> i32 {
        self.read_area3_i32(AREA3_PPD_CRYPT_STATE_OFFSET)
    }

    pub fn set_area3_crypt_state(&mut self, state: i32) {
        self.write_area3_i32(AREA3_PPD_CRYPT_STATE_OFFSET, state);
    }

    pub fn area3_william_state(&self) -> i32 {
        self.read_area3_i32(AREA3_PPD_WILLIAM_STATE_OFFSET)
    }

    pub fn set_area3_william_state(&mut self, state: i32) {
        self.write_area3_i32(AREA3_PPD_WILLIAM_STATE_OFFSET, state);
    }

    /// C `struct area3_ppd::imp_state` (`src/area/3/area3.h:29`), reset by
    /// `questlog_reopen_q22` (`src/system/questlog.c:464-477`).
    pub fn area3_imp_state(&self) -> i32 {
        self.read_area3_i32(AREA3_PPD_IMP_STATE_OFFSET)
    }

    pub fn set_area3_imp_state(&mut self, state: i32) {
        self.write_area3_i32(AREA3_PPD_IMP_STATE_OFFSET, state);
    }

    /// C `struct area3_ppd::imp_kills` (`src/area/3/area3.h:30`), reset by
    /// `questlog_reopen_q22` (`src/system/questlog.c:464-477`).
    pub fn area3_imp_kills(&self) -> i32 {
        self.read_area3_i32(AREA3_PPD_IMP_KILLS_OFFSET)
    }

    pub fn set_area3_imp_kills(&mut self, kills: i32) {
        self.write_area3_i32(AREA3_PPD_IMP_KILLS_OFFSET, kills);
    }

    pub fn area3_hermit_state(&self) -> i32 {
        self.read_area3_i32(AREA3_PPD_HERMIT_STATE_OFFSET)
    }

    pub fn set_area3_hermit_state(&mut self, state: i32) {
        self.write_area3_i32(AREA3_PPD_HERMIT_STATE_OFFSET, state);
    }

    /// Backs `cmd_showppd`'s `/showppd <name> area3` branch
    /// (`src/system/command.c:339-346`); no gameplay driver reads/writes
    /// this yet.
    pub fn area3_kassim_state(&self) -> i32 {
        self.read_area3_i32(AREA3_PPD_KASSIM_STATE_OFFSET)
    }

    pub fn set_area3_kassim_state(&mut self, state: i32) {
        self.write_area3_i32(AREA3_PPD_KASSIM_STATE_OFFSET, state);
    }

    /// Snapshot of the `area3_ppd` fields consumed by
    /// `questlog_init_area3` (`src/system/questlog.c:1040-1203`), for
    /// `crate::quest::init_area3_quests`.
    pub fn area3_quest_state(&self) -> crate::quest::Area3QuestState {
        crate::quest::Area3QuestState {
            seymour_state: self.area3_seymour_state(),
            kelly_state: self.area3_kelly_state(),
            astro2_state: self.area3_astro2_state(),
            crypt_state: self.area3_crypt_state(),
            clara_state: self.area3_clara_state(),
            william_state: self.area3_william_state(),
            hermit_state: self.area3_hermit_state(),
        }
    }

    pub(crate) fn read_area3_i32(&self, offset: usize) -> i32 {
        if self.area3_ppd.len() < LEGACY_AREA3_PPD_SIZE {
            return 0;
        }
        read_i32(&self.area3_ppd, offset)
    }

    pub(crate) fn write_area3_i32(&mut self, offset: usize, value: i32) {
        if self.area3_ppd.len() < LEGACY_AREA3_PPD_SIZE {
            self.area3_ppd.resize(LEGACY_AREA3_PPD_SIZE, 0);
        }
        write_i32(&mut self.area3_ppd, offset, value);
    }

    pub fn mark_area3_imp_flag(&mut self, mask: u32) -> bool {
        if self.area3_ppd.len() < LEGACY_AREA3_PPD_SIZE {
            self.area3_ppd.resize(LEGACY_AREA3_PPD_SIZE, 0);
        }
        let current = self.area3_imp_flags();
        if current & mask != 0 {
            return false;
        }
        write_i32(
            &mut self.area3_ppd,
            AREA3_PPD_IMP_FLAGS_OFFSET,
            (current | mask) as i32,
        );
        true
    }
}
