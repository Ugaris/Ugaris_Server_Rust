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

    /// C `struct twocity_ppd::thief_killed[6]` element write, used by
    /// `robber_dead`'s `ppd->thief_killed[N]++` (`two.c:2211-2247`) via
    /// `crates/ugaris-server/src/world_events/death_hooks.rs`'s
    /// `apply_two_robber_death_from_hurt_event`.
    pub fn set_twocity_thief_killed(&mut self, index: usize, value: i32) {
        if index >= 6 {
            return;
        }
        self.twocity_ppd.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        write_i32(
            &mut self.twocity_ppd,
            TWOCITY_PPD_THIEF_KILLED_OFFSET + index * 4,
            value,
        );
    }

    /// C `struct twocity_ppd::thief_last_seen`, read by `thiefmaster`'s
    /// `thief_state == 9` waiting nag (`two.c:1850`).
    pub fn twocity_thief_last_seen(&self) -> i32 {
        if self.twocity_ppd.len() < LEGACY_TWOCITY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.twocity_ppd, TWOCITY_PPD_THIEF_LAST_SEEN_OFFSET)
    }

    pub fn set_twocity_thief_last_seen(&mut self, realtime: i32) {
        self.twocity_ppd.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        write_i32(
            &mut self.twocity_ppd,
            TWOCITY_PPD_THIEF_LAST_SEEN_OFFSET,
            realtime,
        );
    }

    /// C `struct twocity_ppd::thief_bits`, `thiefmaster`'s lockpick-chain
    /// completion mask (see `TWOCITY_PPD_THIEF_BITS_OFFSET`'s own doc
    /// comment for why it's write-only).
    pub fn twocity_thief_bits(&self) -> i32 {
        if self.twocity_ppd.len() < LEGACY_TWOCITY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.twocity_ppd, TWOCITY_PPD_THIEF_BITS_OFFSET)
    }

    pub fn set_twocity_thief_bits(&mut self, bits: i32) {
        self.twocity_ppd.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        write_i32(&mut self.twocity_ppd, TWOCITY_PPD_THIEF_BITS_OFFSET, bits);
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

    /// C `struct twocity_ppd::legal_status` (`LS_CLEAN`/`LS_FINE`/
    /// `LS_DEAD`, see `crate::world::npc::area17::{LS_CLEAN, LS_FINE,
    /// LS_DEAD}`).
    pub fn twocity_legal_status(&self) -> i32 {
        if self.twocity_ppd.len() < LEGACY_TWOCITY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.twocity_ppd, TWOCITY_PPD_LEGAL_STATUS_OFFSET)
    }

    pub fn set_twocity_legal_status(&mut self, status: i32) {
        self.twocity_ppd.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        write_i32(
            &mut self.twocity_ppd,
            TWOCITY_PPD_LEGAL_STATUS_OFFSET,
            status,
        );
    }

    /// C `struct twocity_ppd::legal_fine`.
    pub fn twocity_legal_fine(&self) -> i32 {
        if self.twocity_ppd.len() < LEGACY_TWOCITY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.twocity_ppd, TWOCITY_PPD_LEGAL_FINE_OFFSET)
    }

    pub fn set_twocity_legal_fine(&mut self, fine: i32) {
        self.twocity_ppd.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        write_i32(&mut self.twocity_ppd, TWOCITY_PPD_LEGAL_FINE_OFFSET, fine);
    }

    /// C `struct twocity_ppd::citizen_status` (`CS_ENEMY`/`CS_GUEST`/
    /// `CS_CITIZEN`/`CS_HONOR`, see `crate::world::npc::area17::{CS_ENEMY,
    /// CS_GUEST, CS_CITIZEN, CS_HONOR}`).
    pub fn twocity_citizen_status(&self) -> i32 {
        if self.twocity_ppd.len() < LEGACY_TWOCITY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.twocity_ppd, TWOCITY_PPD_CITIZEN_STATUS_OFFSET)
    }

    pub fn set_twocity_citizen_status(&mut self, status: i32) {
        self.twocity_ppd.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        write_i32(
            &mut self.twocity_ppd,
            TWOCITY_PPD_CITIZEN_STATUS_OFFSET,
            status,
        );
    }

    /// C `struct twocity_ppd::barkeeper_state`.
    pub fn twocity_barkeeper_state(&self) -> i32 {
        if self.twocity_ppd.len() < LEGACY_TWOCITY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.twocity_ppd, TWOCITY_PPD_BARKEEPER_STATE_OFFSET)
    }

    pub fn set_twocity_barkeeper_state(&mut self, state: i32) {
        self.twocity_ppd.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        write_i32(
            &mut self.twocity_ppd,
            TWOCITY_PPD_BARKEEPER_STATE_OFFSET,
            state,
        );
    }

    /// C `struct twocity_ppd::barkeeper_last` (wall-clock `realtime`
    /// seconds).
    pub fn twocity_barkeeper_last(&self) -> i32 {
        if self.twocity_ppd.len() < LEGACY_TWOCITY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.twocity_ppd, TWOCITY_PPD_BARKEEPER_LAST_OFFSET)
    }

    pub fn set_twocity_barkeeper_last(&mut self, realtime: i32) {
        self.twocity_ppd.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        write_i32(
            &mut self.twocity_ppd,
            TWOCITY_PPD_BARKEEPER_LAST_OFFSET,
            realtime,
        );
    }

    /// C `struct twocity_ppd::current_guard`.
    pub fn twocity_current_guard(&self) -> i32 {
        if self.twocity_ppd.len() < LEGACY_TWOCITY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.twocity_ppd, TWOCITY_PPD_CURRENT_GUARD_OFFSET)
    }

    pub fn set_twocity_current_guard(&mut self, guard_id: i32) {
        self.twocity_ppd.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        write_i32(
            &mut self.twocity_ppd,
            TWOCITY_PPD_CURRENT_GUARD_OFFSET,
            guard_id,
        );
    }

    /// C `struct twocity_ppd::current_guard_time`.
    pub fn twocity_current_guard_time(&self) -> i32 {
        if self.twocity_ppd.len() < LEGACY_TWOCITY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.twocity_ppd, TWOCITY_PPD_CURRENT_GUARD_TIME_OFFSET)
    }

    pub fn set_twocity_current_guard_time(&mut self, realtime: i32) {
        self.twocity_ppd.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        write_i32(
            &mut self.twocity_ppd,
            TWOCITY_PPD_CURRENT_GUARD_TIME_OFFSET,
            realtime,
        );
    }

    /// C `struct twocity_ppd::last_attack`.
    pub fn twocity_last_attack(&self) -> i32 {
        if self.twocity_ppd.len() < LEGACY_TWOCITY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.twocity_ppd, TWOCITY_PPD_LAST_ATTACK_OFFSET)
    }

    pub fn set_twocity_last_attack(&mut self, realtime: i32) {
        self.twocity_ppd.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        write_i32(
            &mut self.twocity_ppd,
            TWOCITY_PPD_LAST_ATTACK_OFFSET,
            realtime,
        );
    }

    /// C `struct twocity_ppd::guard_intro`.
    pub fn twocity_guard_intro(&self) -> i32 {
        if self.twocity_ppd.len() < LEGACY_TWOCITY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.twocity_ppd, TWOCITY_PPD_GUARD_INTRO_OFFSET)
    }

    pub fn set_twocity_guard_intro(&mut self, value: i32) {
        self.twocity_ppd.resize(LEGACY_TWOCITY_PPD_SIZE, 0);
        write_i32(&mut self.twocity_ppd, TWOCITY_PPD_GUARD_INTRO_OFFSET, value);
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
