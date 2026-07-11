use super::*;

impl PlayerRuntime {
    pub fn encode_legacy_staffer_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_STAFFER_PPD_SIZE];
        let len = self.staffer_ppd.len().min(LEGACY_STAFFER_PPD_SIZE);
        bytes[..len].copy_from_slice(&self.staffer_ppd[..len]);
        bytes
    }

    pub fn decode_legacy_staffer_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_STAFFER_PPD_SIZE {
            return false;
        }
        self.staffer_ppd = bytes[..LEGACY_STAFFER_PPD_SIZE].to_vec();
        true
    }

    pub fn forestbran_done(&self) -> u8 {
        if self.staffer_ppd.len() < LEGACY_STAFFER_PPD_SIZE {
            return 0;
        }
        read_i32(&self.staffer_ppd, STAFFER_PPD_FORESTBRAN_DONE_OFFSET).clamp(0, 5) as u8
    }

    pub fn set_forestbran_done(&mut self, dig_index: u8) -> Option<u8> {
        if dig_index >= TREASURE_DIG_PPD_ENTRIES as u8 {
            return None;
        }
        let done = dig_index + 1;
        if self.staffer_ppd.len() < LEGACY_STAFFER_PPD_SIZE {
            self.staffer_ppd.resize(LEGACY_STAFFER_PPD_SIZE, 0);
        }
        write_i32(
            &mut self.staffer_ppd,
            STAFFER_PPD_FORESTBRAN_DONE_OFFSET,
            i32::from(done),
        );
        Some(done)
    }

    /// C `ppd->forestbran_done = 0` (`brannington.c:1424`, the god-only
    /// "reset me" branch of `forest_brannington_driver`) - unlike
    /// [`Self::set_forestbran_done`], this writes a raw `0`, not `dig_index
    /// + 1`.
    pub fn clear_forestbran_done(&mut self) {
        if self.staffer_ppd.len() < LEGACY_STAFFER_PPD_SIZE {
            self.staffer_ppd.resize(LEGACY_STAFFER_PPD_SIZE, 0);
        }
        write_i32(&mut self.staffer_ppd, STAFFER_PPD_FORESTBRAN_DONE_OFFSET, 0);
    }

    pub fn mark_staffer_animation_book_seen(&mut self) -> bool {
        if self.staffer_ppd.len() < LEGACY_STAFFER_PPD_SIZE {
            self.staffer_ppd.resize(LEGACY_STAFFER_PPD_SIZE, 0);
        }
        let state = read_i32(&self.staffer_ppd, STAFFER_PPD_SHANRA_STATE_OFFSET);
        if state >= 3 {
            return false;
        }
        write_i32(&mut self.staffer_ppd, STAFFER_PPD_SHANRA_STATE_OFFSET, 3);
        true
    }

    pub(crate) fn read_staffer_i32(&self, offset: usize) -> i32 {
        if self.staffer_ppd.len() < LEGACY_STAFFER_PPD_SIZE {
            return 0;
        }
        read_i32(&self.staffer_ppd, offset)
    }

    pub(crate) fn write_staffer_i32(&mut self, offset: usize, value: i32) {
        if self.staffer_ppd.len() < LEGACY_STAFFER_PPD_SIZE {
            self.staffer_ppd.resize(LEGACY_STAFFER_PPD_SIZE, 0);
        }
        write_i32(&mut self.staffer_ppd, offset, value);
    }

    pub fn staffer_smugglecom_state(&self) -> i32 {
        self.read_staffer_i32(STAFFER_PPD_SMUGGLECOM_STATE_OFFSET)
    }

    pub fn set_staffer_smugglecom_state(&mut self, state: i32) {
        self.write_staffer_i32(STAFFER_PPD_SMUGGLECOM_STATE_OFFSET, state);
    }

    /// C `struct staffer_ppd::smugglecom_bits` (`src/common/staffer_ppd.h:15`),
    /// cleared by `questlog_reopen_q35` when reopening at state `5`
    /// (`src/system/questlog.c:495-509`).
    pub fn staffer_smugglecom_bits(&self) -> i32 {
        self.read_staffer_i32(STAFFER_PPD_SMUGGLECOM_BITS_OFFSET)
    }

    pub fn set_staffer_smugglecom_bits(&mut self, bits: i32) {
        self.write_staffer_i32(STAFFER_PPD_SMUGGLECOM_BITS_OFFSET, bits);
    }

    pub fn staffer_carlos_state(&self) -> i32 {
        self.read_staffer_i32(STAFFER_PPD_CARLOS_STATE_OFFSET)
    }

    pub fn set_staffer_carlos_state(&mut self, state: i32) {
        self.write_staffer_i32(STAFFER_PPD_CARLOS_STATE_OFFSET, state);
    }

    /// C `struct staffer_ppd::carlos2_state` (`src/common/staffer_ppd.h:43`)
    /// - `carlos_driver`'s Imperial Vault ritual quest state, separate from
    /// the dragon-staff quest's `carlos_state` above.
    pub fn staffer_carlos2_state(&self) -> i32 {
        self.read_staffer_i32(STAFFER_PPD_CARLOS2_STATE_OFFSET)
    }

    pub fn set_staffer_carlos2_state(&mut self, state: i32) {
        self.write_staffer_i32(STAFFER_PPD_CARLOS2_STATE_OFFSET, state);
    }

    pub fn staffer_countbran_state(&self) -> i32 {
        self.read_staffer_i32(STAFFER_PPD_COUNTBRAN_STATE_OFFSET)
    }

    pub fn set_staffer_countbran_state(&mut self, state: i32) {
        self.write_staffer_i32(STAFFER_PPD_COUNTBRAN_STATE_OFFSET, state);
    }

    pub fn staffer_countbran_bits(&self) -> i32 {
        self.read_staffer_i32(STAFFER_PPD_COUNTBRAN_BITS_OFFSET)
    }

    pub fn set_staffer_countbran_bits(&mut self, bits: i32) {
        self.write_staffer_i32(STAFFER_PPD_COUNTBRAN_BITS_OFFSET, bits);
    }

    /// C `struct staffer_ppd::countessabran_state` (`src/common/
    /// staffer_ppd.h:21`), consumed by `world::npc::area29::countessabran`.
    pub fn staffer_countessabran_state(&self) -> i32 {
        self.read_staffer_i32(STAFFER_PPD_COUNTESSABRAN_STATE_OFFSET)
    }

    pub fn set_staffer_countessabran_state(&mut self, state: i32) {
        self.write_staffer_i32(STAFFER_PPD_COUNTESSABRAN_STATE_OFFSET, state);
    }

    /// C `struct staffer_ppd::daughterbran_state` (`src/common/
    /// staffer_ppd.h:22`), consumed by `world::npc::area29::daughterbran`.
    pub fn staffer_daughterbran_state(&self) -> i32 {
        self.read_staffer_i32(STAFFER_PPD_DAUGHTERBRAN_STATE_OFFSET)
    }

    pub fn set_staffer_daughterbran_state(&mut self, state: i32) {
        self.write_staffer_i32(STAFFER_PPD_DAUGHTERBRAN_STATE_OFFSET, state);
    }

    pub fn staffer_spiritbran_state(&self) -> i32 {
        self.read_staffer_i32(STAFFER_PPD_SPIRITBRAN_STATE_OFFSET)
    }

    pub fn set_staffer_spiritbran_state(&mut self, state: i32) {
        self.write_staffer_i32(STAFFER_PPD_SPIRITBRAN_STATE_OFFSET, state);
    }

    pub fn staffer_brennethbran_state(&self) -> i32 {
        self.read_staffer_i32(STAFFER_PPD_BRENNETHBRAN_STATE_OFFSET)
    }

    pub fn set_staffer_brennethbran_state(&mut self, state: i32) {
        self.write_staffer_i32(STAFFER_PPD_BRENNETHBRAN_STATE_OFFSET, state);
    }

    /// C `struct staffer_ppd::forestbran_state` (`src/common/staffer_ppd.h:
    /// 26`), consumed by `world::npc::area29::forestbran`. Separate from
    /// the neighboring `forestbran_done` field above.
    pub fn staffer_forestbran_state(&self) -> i32 {
        self.read_staffer_i32(STAFFER_PPD_FORESTBRAN_STATE_OFFSET)
    }

    pub fn set_staffer_forestbran_state(&mut self, state: i32) {
        self.write_staffer_i32(STAFFER_PPD_FORESTBRAN_STATE_OFFSET, state);
    }

    pub fn staffer_broklin_state(&self) -> i32 {
        self.read_staffer_i32(STAFFER_PPD_BROKLIN_STATE_OFFSET)
    }

    pub fn set_staffer_broklin_state(&mut self, state: i32) {
        self.write_staffer_i32(STAFFER_PPD_BROKLIN_STATE_OFFSET, state);
    }

    pub fn staffer_aristocrat_state(&self) -> i32 {
        self.read_staffer_i32(STAFFER_PPD_ARISTOCRAT_STATE_OFFSET)
    }

    pub fn set_staffer_aristocrat_state(&mut self, state: i32) {
        self.write_staffer_i32(STAFFER_PPD_ARISTOCRAT_STATE_OFFSET, state);
    }

    pub fn staffer_yoatin_state(&self) -> i32 {
        self.read_staffer_i32(STAFFER_PPD_YOATIN_STATE_OFFSET)
    }

    pub fn set_staffer_yoatin_state(&mut self, state: i32) {
        self.write_staffer_i32(STAFFER_PPD_YOATIN_STATE_OFFSET, state);
    }

    /// C `struct staffer_ppd::centinel_count` (`src/common/staffer_ppd.h:35`)
    /// - the sentinel kill counter consumed by `centinel_dead`
    /// (`src/area/29/brannington.c:2725-2758`).
    pub fn staffer_centinel_count(&self) -> i32 {
        self.read_staffer_i32(STAFFER_PPD_CENTINEL_COUNT_OFFSET)
    }

    pub fn set_staffer_centinel_count(&mut self, count: i32) {
        self.write_staffer_i32(STAFFER_PPD_CENTINEL_COUNT_OFFSET, count);
    }

    pub fn staffer_dwarfchief_state(&self) -> i32 {
        self.read_staffer_i32(STAFFER_PPD_DWARFCHIEF_STATE_OFFSET)
    }

    pub fn set_staffer_dwarfchief_state(&mut self, state: i32) {
        self.write_staffer_i32(STAFFER_PPD_DWARFCHIEF_STATE_OFFSET, state);
    }

    pub fn staffer_dwarfshaman_state(&self) -> i32 {
        self.read_staffer_i32(STAFFER_PPD_DWARFSHAMAN_STATE_OFFSET)
    }

    pub fn set_staffer_dwarfshaman_state(&mut self, state: i32) {
        self.write_staffer_i32(STAFFER_PPD_DWARFSHAMAN_STATE_OFFSET, state);
    }

    /// C `struct staffer_ppd::rouven_state` (`src/common/staffer_ppd.h:44`)
    /// - `rouven_driver`'s Imperial Vault guard quest state, also read by
    /// `vault_skull` (`IDR_STAFFER` `drdata[0]==4`) via
    /// `ItemDriverContext::rouven_state`.
    pub fn staffer_rouven_state(&self) -> i32 {
        self.read_staffer_i32(STAFFER_PPD_ROUVEN_STATE_OFFSET)
    }

    pub fn set_staffer_rouven_state(&mut self, state: i32) {
        self.write_staffer_i32(STAFFER_PPD_ROUVEN_STATE_OFFSET, state);
    }

    /// Snapshot of the `staffer_ppd` fields consumed by
    /// `questlog_init_staff` (`src/system/questlog.c:1203-1394`), for
    /// `crate::quest::init_staff_quests`.
    pub fn staff_quest_state(&self) -> crate::quest::StaffQuestState {
        crate::quest::StaffQuestState {
            carlos_state: self.staffer_carlos_state(),
            smugglecom_state: self.staffer_smugglecom_state(),
            aristocrat_state: self.staffer_aristocrat_state(),
            yoatin_state: self.staffer_yoatin_state(),
            countbran_state: self.staffer_countbran_state(),
            countbran_bits: self.staffer_countbran_bits(),
            brennethbran_state: self.staffer_brennethbran_state(),
            spiritbran_state: self.staffer_spiritbran_state(),
            broklin_state: self.staffer_broklin_state(),
            dwarfchief_state: self.staffer_dwarfchief_state(),
            dwarfshaman_state: self.staffer_dwarfshaman_state(),
        }
    }
}
