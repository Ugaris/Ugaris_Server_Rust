use super::*;

impl PlayerRuntime {
    pub fn encode_legacy_area1_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_AREA1_PPD_SIZE];
        let copy_len = self.area1_ppd.len().min(LEGACY_AREA1_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.area1_ppd[..copy_len]);
        bytes
    }

    pub fn decode_legacy_area1_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_AREA1_PPD_SIZE {
            return false;
        }
        self.area1_ppd = bytes[..LEGACY_AREA1_PPD_SIZE].to_vec();
        true
    }

    pub(crate) fn read_area1_i32(&self, offset: usize) -> i32 {
        if self.area1_ppd.len() < LEGACY_AREA1_PPD_SIZE {
            return 0;
        }
        read_i32(&self.area1_ppd, offset)
    }

    pub(crate) fn write_area1_i32(&mut self, offset: usize, value: i32) {
        if self.area1_ppd.len() < LEGACY_AREA1_PPD_SIZE {
            self.area1_ppd.resize(LEGACY_AREA1_PPD_SIZE, 0);
        }
        write_i32(&mut self.area1_ppd, offset, value);
    }

    pub fn area1_yoakin_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_YOAKIN_STATE_OFFSET)
    }

    pub fn set_area1_yoakin_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_YOAKIN_STATE_OFFSET, state);
    }

    pub fn area1_gwendy_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_GWENDY_STATE_OFFSET)
    }

    pub fn set_area1_gwendy_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_GWENDY_STATE_OFFSET, state);
    }

    /// C `struct area1_ppd::james_state` (`src/area/1/area1.h:33`), reset by
    /// `questlog_reopen_q0` (`src/system/questlog.c:342-351`).
    pub fn area1_james_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_JAMES_STATE_OFFSET)
    }

    pub fn set_area1_james_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_JAMES_STATE_OFFSET, state);
    }

    pub fn area1_nook_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_NOOK_STATE_OFFSET)
    }

    pub fn set_area1_nook_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_NOOK_STATE_OFFSET, state);
    }

    pub fn area1_lydia_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_LYDIA_STATE_OFFSET)
    }

    pub fn set_area1_lydia_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_LYDIA_STATE_OFFSET, state);
    }

    pub fn area1_guiwynn_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_GUIWYNN_STATE_OFFSET)
    }

    pub fn set_area1_guiwynn_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_GUIWYNN_STATE_OFFSET, state);
    }

    pub fn area1_logain_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_LOGAIN_STATE_OFFSET)
    }

    pub fn set_area1_logain_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_LOGAIN_STATE_OFFSET, state);
    }

    pub fn area1_reskin_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_RESKIN_STATE_OFFSET)
    }

    pub fn set_area1_reskin_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_RESKIN_STATE_OFFSET, state);
    }

    pub fn area1_brithildie_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_BRITHILDIE_STATE_OFFSET)
    }

    pub fn set_area1_brithildie_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_BRITHILDIE_STATE_OFFSET, state);
    }

    pub fn area1_camhermit_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_CAMHERMIT_STATE_OFFSET)
    }

    pub fn set_area1_camhermit_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_CAMHERMIT_STATE_OFFSET, state);
    }

    pub fn area1_jessica_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_JESSICA_STATE_OFFSET)
    }

    pub fn set_area1_jessica_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_JESSICA_STATE_OFFSET, state);
    }

    // The remaining `area1_ppd` getters below have no gameplay driver in
    // Rust yet (no NPC state machine sets them); they exist solely to
    // back `cmd_showppd`'s `/showppd <name> area1` branch
    // (`src/system/command.c:275-336`), which reads every field of the
    // struct for a `CF_GOD` debug dump.
    pub fn area1_yoakin_seen_timer(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_YOAKIN_SEEN_TIMER_OFFSET)
    }

    pub fn set_area1_yoakin_seen_timer(&mut self, value: i32) {
        self.write_area1_i32(AREA1_PPD_YOAKIN_SEEN_TIMER_OFFSET, value);
    }

    pub fn area1_gwendy_seen_timer(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_GWENDY_SEEN_TIMER_OFFSET)
    }

    pub fn set_area1_gwendy_seen_timer(&mut self, value: i32) {
        self.write_area1_i32(AREA1_PPD_GWENDY_SEEN_TIMER_OFFSET, value);
    }

    pub fn area1_terion_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_TERION_STATE_OFFSET)
    }

    pub fn set_area1_terion_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_TERION_STATE_OFFSET, state);
    }

    pub fn area1_flags(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_FLAGS_OFFSET)
    }

    pub fn set_area1_flags(&mut self, flags: i32) {
        self.write_area1_i32(AREA1_PPD_FLAGS_OFFSET, flags);
    }

    pub fn area1_darkin_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_DARKIN_STATE_OFFSET)
    }

    pub fn set_area1_darkin_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_DARKIN_STATE_OFFSET, state);
    }

    pub fn area1_gerewin_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_GEREWIN_STATE_OFFSET)
    }

    pub fn set_area1_gerewin_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_GEREWIN_STATE_OFFSET, state);
    }

    pub fn area1_gerewin_seen_timer(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_GEREWIN_SEEN_TIMER_OFFSET)
    }

    pub fn set_area1_gerewin_seen_timer(&mut self, value: i32) {
        self.write_area1_i32(AREA1_PPD_GEREWIN_SEEN_TIMER_OFFSET, value);
    }

    pub fn area1_lydia_seen_timer(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_LYDIA_SEEN_TIMER_OFFSET)
    }

    pub fn set_area1_lydia_seen_timer(&mut self, value: i32) {
        self.write_area1_i32(AREA1_PPD_LYDIA_SEEN_TIMER_OFFSET, value);
    }

    pub fn area1_asturin_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_ASTURIN_STATE_OFFSET)
    }

    pub fn set_area1_asturin_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_ASTURIN_STATE_OFFSET, state);
    }

    pub fn area1_asturin_seen_timer(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_ASTURIN_SEEN_TIMER_OFFSET)
    }

    pub fn set_area1_asturin_seen_timer(&mut self, value: i32) {
        self.write_area1_i32(AREA1_PPD_ASTURIN_SEEN_TIMER_OFFSET, value);
    }

    pub fn area1_guiwynn_seen_timer(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_GUIWYNN_SEEN_TIMER_OFFSET)
    }

    pub fn set_area1_guiwynn_seen_timer(&mut self, value: i32) {
        self.write_area1_i32(AREA1_PPD_GUIWYNN_SEEN_TIMER_OFFSET, value);
    }

    pub fn area1_logain_seen_timer(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_LOGAIN_SEEN_TIMER_OFFSET)
    }

    pub fn set_area1_logain_seen_timer(&mut self, value: i32) {
        self.write_area1_i32(AREA1_PPD_LOGAIN_SEEN_TIMER_OFFSET, value);
    }

    pub fn area1_reskin_seen_timer(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_RESKIN_SEEN_TIMER_OFFSET)
    }

    pub fn set_area1_reskin_seen_timer(&mut self, value: i32) {
        self.write_area1_i32(AREA1_PPD_RESKIN_SEEN_TIMER_OFFSET, value);
    }

    pub fn area1_reskin_got_bits(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_RESKIN_GOT_BITS_OFFSET)
    }

    pub fn set_area1_reskin_got_bits(&mut self, value: i32) {
        self.write_area1_i32(AREA1_PPD_RESKIN_GOT_BITS_OFFSET, value);
    }

    pub fn area1_shrike_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_SHRIKE_STATE_OFFSET)
    }

    pub fn set_area1_shrike_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_SHRIKE_STATE_OFFSET, state);
    }

    pub fn area1_shrike_fails(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_SHRIKE_FAILS_OFFSET)
    }

    pub fn set_area1_shrike_fails(&mut self, value: i32) {
        self.write_area1_i32(AREA1_PPD_SHRIKE_FAILS_OFFSET, value);
    }

    pub fn area1_brithildie_seen_timer(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_BRITHILDIE_SEEN_TIMER_OFFSET)
    }

    pub fn set_area1_brithildie_seen_timer(&mut self, value: i32) {
        self.write_area1_i32(AREA1_PPD_BRITHILDIE_SEEN_TIMER_OFFSET, value);
    }

    pub fn area1_jiu_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_JIU_STATE_OFFSET)
    }

    pub fn set_area1_jiu_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_JIU_STATE_OFFSET, state);
    }

    pub fn area1_jiu_seen_timer(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_JIU_SEEN_TIMER_OFFSET)
    }

    pub fn set_area1_jiu_seen_timer(&mut self, value: i32) {
        self.write_area1_i32(AREA1_PPD_JIU_SEEN_TIMER_OFFSET, value);
    }

    pub fn area1_greeter_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_GREETER_STATE_OFFSET)
    }

    pub fn set_area1_greeter_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_GREETER_STATE_OFFSET, state);
    }

    pub fn area1_greeter_seen_timer(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_GREETER_SEEN_TIMER_OFFSET)
    }

    pub fn set_area1_greeter_seen_timer(&mut self, value: i32) {
        self.write_area1_i32(AREA1_PPD_GREETER_SEEN_TIMER_OFFSET, value);
    }

    pub fn area1_aclerk_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_ACLERK_STATE_OFFSET)
    }

    pub fn set_area1_aclerk_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_ACLERK_STATE_OFFSET, state);
    }

    pub fn area1_aclerk_seen_timer(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_ACLERK_SEEN_TIMER_OFFSET)
    }

    pub fn set_area1_aclerk_seen_timer(&mut self, value: i32) {
        self.write_area1_i32(AREA1_PPD_ACLERK_SEEN_TIMER_OFFSET, value);
    }

    pub fn area1_camhermit_seen_timer(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_CAMHERMIT_SEEN_TIMER_OFFSET)
    }

    pub fn set_area1_camhermit_seen_timer(&mut self, value: i32) {
        self.write_area1_i32(AREA1_PPD_CAMHERMIT_SEEN_TIMER_OFFSET, value);
    }

    pub fn area1_camhermit_kills(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_CAMHERMIT_KILLS_OFFSET)
    }

    pub fn set_area1_camhermit_kills(&mut self, value: i32) {
        self.write_area1_i32(AREA1_PPD_CAMHERMIT_KILLS_OFFSET, value);
    }

    pub fn area1_jessica_seen_timer(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_JESSICA_SEEN_TIMER_OFFSET)
    }

    pub fn set_area1_jessica_seen_timer(&mut self, value: i32) {
        self.write_area1_i32(AREA1_PPD_JESSICA_SEEN_TIMER_OFFSET, value);
    }

    pub fn area1_forest_ranger_state(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_FOREST_RANGER_STATE_OFFSET)
    }

    pub fn set_area1_forest_ranger_state(&mut self, state: i32) {
        self.write_area1_i32(AREA1_PPD_FOREST_RANGER_STATE_OFFSET, state);
    }

    pub fn area1_forest_ranger_seen_timer(&self) -> i32 {
        self.read_area1_i32(AREA1_PPD_FOREST_RANGER_SEEN_TIMER_OFFSET)
    }

    pub fn set_area1_forest_ranger_seen_timer(&mut self, value: i32) {
        self.write_area1_i32(AREA1_PPD_FOREST_RANGER_SEEN_TIMER_OFFSET, value);
    }

    /// Snapshot of the `area1_ppd` fields consumed by
    /// `questlog_init_area1` (`src/system/questlog.c:828-1039`), for
    /// `crate::quest::init_area1_quests`.
    pub fn area1_quest_state(&self) -> crate::quest::Area1QuestState {
        crate::quest::Area1QuestState {
            lydia_state: self.area1_lydia_state(),
            gwendy_state: self.area1_gwendy_state(),
            yoakin_state: self.area1_yoakin_state(),
            nook_state: self.area1_nook_state(),
            guiwynn_state: self.area1_guiwynn_state(),
            logain_state: self.area1_logain_state(),
            reskin_state: self.area1_reskin_state(),
            jessica_state: self.area1_jessica_state(),
            brithildie_state: self.area1_brithildie_state(),
            camhermit_state: self.area1_camhermit_state(),
        }
    }
}
