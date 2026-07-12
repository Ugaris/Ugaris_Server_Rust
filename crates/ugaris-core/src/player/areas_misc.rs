use super::*;

impl PlayerRuntime {
    pub fn saltmine_ladder_ready(&self, ladder_index: u8, realtime_seconds: u64) -> bool {
        let Some(last_used) = self
            .saltmine_ladder_last_seconds
            .get(usize::from(ladder_index))
        else {
            return false;
        };
        *last_used == 0 || last_used.saturating_add(60 * 60 * 24) <= realtime_seconds
    }

    pub fn mark_saltmine_ladder_used(&mut self, ladder_index: u8, realtime_seconds: u64) -> bool {
        let Some(last_used) = self
            .saltmine_ladder_last_seconds
            .get_mut(usize::from(ladder_index))
        else {
            return false;
        };
        *last_used = realtime_seconds;
        true
    }

    pub fn encode_legacy_saltmine_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_SALTMINE_PPD_SIZE];
        bytes[0] = LEGACY_SALTMINE_PPD_VERSION;
        for (idx, seconds) in self.saltmine_ladder_last_seconds.iter().enumerate() {
            let value = (*seconds).min(i32::MAX as u64) as i32;
            write_i32(&mut bytes, 4 + idx * 4, value);
        }
        write_i32(
            &mut bytes,
            4 + SALTMINE_LADDER_COUNT * 4,
            self.saltmine_pending_salt.min(i32::MAX as u32) as i32,
        );
        bytes
    }

    pub fn decode_legacy_saltmine_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_SALTMINE_PPD_SIZE {
            return false;
        }
        if bytes[0] != LEGACY_SALTMINE_PPD_VERSION {
            self.saltmine_ladder_last_seconds = [0; SALTMINE_LADDER_COUNT];
            self.saltmine_pending_salt = 0;
            return true;
        }
        for idx in 0..SALTMINE_LADDER_COUNT {
            self.saltmine_ladder_last_seconds[idx] = read_i32(bytes, 4 + idx * 4).max(0) as u64;
        }
        self.saltmine_pending_salt = read_i32(bytes, 4 + SALTMINE_LADDER_COUNT * 4).max(0) as u32;
        true
    }

    pub fn ensure_rune_special_execs<F>(&mut self, mut random_below: F)
    where
        F: FnMut(u32) -> u32,
    {
        if self.rune_special_exec[0] != 0 {
            return;
        }

        const BADLIST: [i32; 15] = [555, 55, 5, 666, 66, 6, 777, 77, 7, 888, 88, 8, 999, 99, 9];
        for level in 5..10 {
            for offset in 0..5 {
                loop {
                    let value = random_below(level * 111) as i32;
                    if value < 100 || BADLIST.contains(&value) {
                        continue;
                    }
                    let base = (level - 5) as usize * 5;
                    if self.rune_special_exec[base..base + offset as usize].contains(&value) {
                        continue;
                    }
                    let digits = format!("{value:03}");
                    let level_digit = char::from_digit(level, 10).unwrap();
                    if digits.chars().any(|ch| ch == '0' || ch > level_digit) {
                        continue;
                    }
                    if !digits.chars().any(|ch| ch == level_digit) {
                        continue;
                    }
                    self.rune_special_exec[base + offset as usize] = value;
                    break;
                }
            }
        }
    }

    /// C `rune_check(cn, nr, ppd)` (`src/area/18/bones.c:285-299`): tests
    /// (without setting) the bit for combination `nr` in the 1024-bit
    /// `used[]` bitfield.
    pub fn rune_check(&self, nr: i32) -> RuneCheckResult {
        if !(0..MAXRUNE).contains(&nr) {
            return RuneCheckResult::OutOfRange;
        }
        let word = (nr / 32) as usize;
        let bit = 1u32 << (nr & 31);
        if self.rune_used_words[word] & bit != 0 {
            RuneCheckResult::AlreadyUsed
        } else {
            RuneCheckResult::Ok
        }
    }

    /// C `rune_set(nr, ppd)` (`src/area/18/bones.c:301-313`): marks
    /// combination `nr` as executed. Out-of-range `nr` is a silent no-op
    /// (C only `elog`s a server-side bug, never player-facing).
    pub fn rune_set(&mut self, nr: i32) {
        if !(0..MAXRUNE).contains(&nr) {
            return;
        }
        let word = (nr / 32) as usize;
        let bit = 1u32 << (nr & 31);
        self.rune_used_words[word] |= bit;
    }

    pub fn bone_hint<F>(&mut self, level: u8, nr: u8, pos: u8, random_below: F) -> BoneHintResult
    where
        F: FnMut(u32) -> u32,
    {
        self.ensure_rune_special_execs(random_below);
        let index = usize::from(level.saturating_sub(5)) * 5 + usize::from(nr);
        let value = self
            .rune_special_exec
            .get(index)
            .copied()
            .unwrap_or_default();
        let digits = value.to_string();
        let digit = digits
            .as_bytes()
            .get(usize::from(pos))
            .copied()
            .unwrap_or(b'0');
        let result = digit.saturating_sub(b'0');
        const RUNE_NAMES: [&str; 10] = [
            "none", "Ansuz", "Berkano", "Dagaz", "Ehwaz", "Fehu", "Hagalaz", "Isa", "Ingwaz",
            "Raidho",
        ];
        const POS_NAMES: [&str; 3] = ["first", "second", "third"];
        let Some(rune) = RUNE_NAMES.get(usize::from(result)).copied() else {
            return BoneHintResult::Bug {
                level,
                nr,
                pos,
                value,
            };
        };
        let Some(position) = POS_NAMES.get(usize::from(pos)).copied() else {
            return BoneHintResult::Bug {
                level,
                nr,
                pos,
                value,
            };
        };
        BoneHintResult::Hint {
            page: u16::from(level) * 10 + u16::from(nr),
            rune,
            position,
        }
    }

    pub fn encode_legacy_rune_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_RUNE_PPD_SIZE];
        for (index, word) in self.rune_used_words.iter().copied().enumerate() {
            write_u32(&mut bytes, index * 4, word);
        }
        for (index, value) in self.rune_special_exec.iter().copied().enumerate() {
            write_i32(&mut bytes, RUNE_PPD_SPECIAL_EXEC_OFFSET + index * 4, value);
        }
        bytes
    }

    pub fn decode_legacy_rune_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_RUNE_PPD_SIZE {
            return false;
        }
        for index in 0..RUNE_USED_WORDS {
            self.rune_used_words[index] = read_u32(bytes, index * 4);
        }
        for index in 0..RUNE_SPECIAL_EXEC_COUNT {
            self.rune_special_exec[index] =
                read_i32(bytes, RUNE_PPD_SPECIAL_EXEC_OFFSET + index * 4);
        }
        true
    }

    pub fn encode_legacy_warp_ppd(&self) -> Vec<u8> {
        let mut bytes = self.warp_ppd.clone();
        bytes.resize(LEGACY_WARP_PPD_SIZE, 0);
        write_i32(&mut bytes, WARP_PPD_BASE_OFFSET, self.warp_base);
        write_i32(&mut bytes, WARP_PPD_POINTS_OFFSET, self.warp_points);
        for index in 0..WARP_BONUS_COUNT {
            write_i32(
                &mut bytes,
                WARP_PPD_BONUS_ID_OFFSET + index * 4,
                self.warp_bonus_ids.get(index).copied().unwrap_or_default(),
            );
        }
        for index in 0..WARP_BONUS_COUNT {
            write_i32(
                &mut bytes,
                WARP_PPD_BONUS_LAST_USED_OFFSET + index * 4,
                self.warp_bonus_last_used
                    .get(index)
                    .copied()
                    .unwrap_or_default(),
            );
        }
        write_i32(&mut bytes, WARP_PPD_NOSTEPEXP_OFFSET, self.warp_nostepexp);
        bytes
    }

    pub fn decode_legacy_warp_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_WARP_PPD_SIZE {
            return false;
        }
        self.warp_ppd = bytes[..LEGACY_WARP_PPD_SIZE].to_vec();
        self.warp_base = read_i32(&self.warp_ppd, WARP_PPD_BASE_OFFSET);
        self.warp_points = read_i32(&self.warp_ppd, WARP_PPD_POINTS_OFFSET);
        self.warp_bonus_ids.resize(WARP_BONUS_COUNT, 0);
        self.warp_bonus_last_used.resize(WARP_BONUS_COUNT, 0);
        for index in 0..WARP_BONUS_COUNT {
            self.warp_bonus_ids[index] =
                read_i32(&self.warp_ppd, WARP_PPD_BONUS_ID_OFFSET + index * 4);
            self.warp_bonus_last_used[index] =
                read_i32(&self.warp_ppd, WARP_PPD_BONUS_LAST_USED_OFFSET + index * 4);
        }
        self.warp_nostepexp = read_i32(&self.warp_ppd, WARP_PPD_NOSTEPEXP_OFFSET);
        true
    }

    pub fn encode_legacy_gate_ppd(&self) -> Vec<u8> {
        let mut bytes = self.gate_ppd.clone();
        bytes.resize(LEGACY_GATE_PPD_SIZE, 0);
        write_i32(
            &mut bytes,
            GATE_PPD_WELCOME_STATE_OFFSET,
            self.gate_welcome_state,
        );
        write_i32(
            &mut bytes,
            GATE_PPD_TARGET_CLASS_OFFSET,
            self.gate_target_class,
        );
        write_i32(&mut bytes, GATE_PPD_STEP_OFFSET, self.gate_step);
        bytes
    }

    pub fn decode_legacy_gate_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_GATE_PPD_SIZE {
            return false;
        }
        self.gate_ppd = bytes[..LEGACY_GATE_PPD_SIZE].to_vec();
        self.gate_welcome_state = read_i32(&self.gate_ppd, GATE_PPD_WELCOME_STATE_OFFSET);
        self.gate_target_class = read_i32(&self.gate_ppd, GATE_PPD_TARGET_CLASS_OFFSET);
        self.gate_step = read_i32(&self.gate_ppd, GATE_PPD_STEP_OFFSET);
        true
    }

    pub fn encode_legacy_nomad_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_NOMAD_PPD_SIZE];
        let copy_len = self.nomad_ppd.len().min(LEGACY_NOMAD_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.nomad_ppd[..copy_len]);
        bytes
    }

    pub fn decode_legacy_nomad_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_NOMAD_PPD_SIZE {
            return false;
        }
        self.nomad_ppd = bytes[..LEGACY_NOMAD_PPD_SIZE].to_vec();
        true
    }

    /// C `nomad_state[MAXNOMAD]` element read (`src/common/nomad_ppd.h:10`).
    pub fn nomad_state(&self, index: usize) -> i32 {
        if index >= NOMAD_PPD_MAXNOMAD || self.nomad_ppd.len() < LEGACY_NOMAD_PPD_SIZE {
            return 0;
        }
        read_i32(&self.nomad_ppd, NOMAD_PPD_STATE_OFFSET + index * 4)
    }

    pub fn set_nomad_state(&mut self, index: usize, value: i32) {
        if index >= NOMAD_PPD_MAXNOMAD {
            return;
        }
        if self.nomad_ppd.len() < LEGACY_NOMAD_PPD_SIZE {
            self.nomad_ppd.resize(LEGACY_NOMAD_PPD_SIZE, 0);
        }
        write_i32(
            &mut self.nomad_ppd,
            NOMAD_PPD_STATE_OFFSET + index * 4,
            value,
        );
    }

    /// C `nomad_win[MAXNOMAD]` element read (`src/common/nomad_ppd.h:11`).
    pub fn nomad_win(&self, index: usize) -> i32 {
        if index >= NOMAD_PPD_MAXNOMAD || self.nomad_ppd.len() < LEGACY_NOMAD_PPD_SIZE {
            return 0;
        }
        read_i32(&self.nomad_ppd, NOMAD_PPD_WIN_OFFSET + index * 4)
    }

    pub fn set_nomad_win(&mut self, index: usize, value: i32) {
        if index >= NOMAD_PPD_MAXNOMAD {
            return;
        }
        if self.nomad_ppd.len() < LEGACY_NOMAD_PPD_SIZE {
            self.nomad_ppd.resize(LEGACY_NOMAD_PPD_SIZE, 0);
        }
        write_i32(&mut self.nomad_ppd, NOMAD_PPD_WIN_OFFSET + index * 4, value);
    }

    /// C `open_roll1`/`open_roll2`/`open_roll3` (`src/common/nomad_ppd.h:
    /// 11`): the last high dice roll left "on the table" when a player
    /// walked away mid-game (`nomad.c:866-872`).
    pub fn nomad_open_roll(&self) -> (i32, i32, i32) {
        if self.nomad_ppd.len() < LEGACY_NOMAD_PPD_SIZE {
            return (0, 0, 0);
        }
        (
            read_i32(&self.nomad_ppd, NOMAD_PPD_OPEN_ROLL1_OFFSET),
            read_i32(&self.nomad_ppd, NOMAD_PPD_OPEN_ROLL2_OFFSET),
            read_i32(&self.nomad_ppd, NOMAD_PPD_OPEN_ROLL3_OFFSET),
        )
    }

    /// C `open_bet` (`src/common/nomad_ppd.h:11`).
    pub fn nomad_open_bet(&self) -> i32 {
        if self.nomad_ppd.len() < LEGACY_NOMAD_PPD_SIZE {
            return 0;
        }
        read_i32(&self.nomad_ppd, NOMAD_PPD_OPEN_BET_OFFSET)
    }

    /// Writes `open_bet`/`open_roll1`/`open_roll2`/`open_roll3` together,
    /// matching every C call site that touches one also having the other
    /// three in scope (`nomad.c:845-848`/`:868-871`/`:885-889`).
    pub fn set_nomad_open_bet(&mut self, bet: i32, roll1: i32, roll2: i32, roll3: i32) {
        if self.nomad_ppd.len() < LEGACY_NOMAD_PPD_SIZE {
            self.nomad_ppd.resize(LEGACY_NOMAD_PPD_SIZE, 0);
        }
        write_i32(&mut self.nomad_ppd, NOMAD_PPD_OPEN_BET_OFFSET, bet);
        write_i32(&mut self.nomad_ppd, NOMAD_PPD_OPEN_ROLL1_OFFSET, roll1);
        write_i32(&mut self.nomad_ppd, NOMAD_PPD_OPEN_ROLL2_OFFSET, roll2);
        write_i32(&mut self.nomad_ppd, NOMAD_PPD_OPEN_ROLL3_OFFSET, roll3);
    }

    /// C `tribe_member` (`src/common/nomad_ppd.h:12`): a `TM_TRIBE1`/`2`/
    /// `3` bitmask.
    pub fn nomad_tribe_member(&self) -> i32 {
        if self.nomad_ppd.len() < LEGACY_NOMAD_PPD_SIZE {
            return 0;
        }
        read_i32(&self.nomad_ppd, NOMAD_PPD_TRIBE_MEMBER_OFFSET)
    }

    /// C `ppd->tribe_member |= flag` (`nomad.c:705`, the only writer).
    pub fn add_nomad_tribe_member(&mut self, flag: i32) {
        if self.nomad_ppd.len() < LEGACY_NOMAD_PPD_SIZE {
            self.nomad_ppd.resize(LEGACY_NOMAD_PPD_SIZE, 0);
        }
        let current = read_i32(&self.nomad_ppd, NOMAD_PPD_TRIBE_MEMBER_OFFSET);
        write_i32(
            &mut self.nomad_ppd,
            NOMAD_PPD_TRIBE_MEMBER_OFFSET,
            current | flag,
        );
    }

    /// Snapshot of the `nomad_state[]` array consumed by
    /// `questlog_init_nomad` (`src/system/questlog.c:1571-1607`), for
    /// `crate::quest::init_nomad_quests`.
    pub fn nomad_quest_state(&self) -> crate::quest::NomadQuestState {
        let mut nomad_state = [0i32; NOMAD_PPD_MAXNOMAD];
        for (index, slot) in nomad_state.iter_mut().enumerate() {
            *slot = self.nomad_state(index);
        }
        crate::quest::NomadQuestState { nomad_state }
    }

    pub fn encode_legacy_caligar_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_CALIGAR_PPD_SIZE];
        let copy_len = self.caligar_ppd.len().min(LEGACY_CALIGAR_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.caligar_ppd[..copy_len]);
        bytes
    }

    pub fn decode_legacy_caligar_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_CALIGAR_PPD_SIZE {
            return false;
        }
        self.caligar_ppd = bytes[..LEGACY_CALIGAR_PPD_SIZE].to_vec();
        true
    }

    pub fn encode_legacy_arkhata_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_ARKHATA_PPD_SIZE];
        let copy_len = self.arkhata_ppd.len().min(LEGACY_ARKHATA_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.arkhata_ppd[..copy_len]);
        bytes
    }

    pub fn decode_legacy_arkhata_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_ARKHATA_PPD_SIZE {
            return false;
        }
        self.arkhata_ppd = bytes[..LEGACY_ARKHATA_PPD_SIZE].to_vec();
        true
    }

    pub fn encode_legacy_farmy_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_FARMY_PPD_SIZE];
        let len = self.farmy_ppd.len().min(LEGACY_FARMY_PPD_SIZE);
        bytes[..len].copy_from_slice(&self.farmy_ppd[..len]);
        bytes
    }

    pub fn decode_legacy_farmy_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_FARMY_PPD_SIZE {
            return false;
        }
        self.farmy_ppd = bytes[..LEGACY_FARMY_PPD_SIZE].to_vec();
        true
    }

    pub fn farmy_boss_stage(&self) -> i32 {
        if self.farmy_ppd.len() < LEGACY_FARMY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.farmy_ppd, FARMY_PPD_BOSS_STAGE_OFFSET)
    }

    fn ensure_farmy_ppd_sized(&mut self) {
        if self.farmy_ppd.len() < LEGACY_FARMY_PPD_SIZE {
            self.farmy_ppd.resize(LEGACY_FARMY_PPD_SIZE, 0);
        }
    }

    /// Unconditional `ppd->boss_stage = N`/`ppd->boss_stage++` write, used
    /// by `fdemon_boss`'s dialogue-chain state machine (unlike
    /// [`Self::advance_farmy_blood_stage`]/[`Self::advance_farmy_lava_stage`]/
    /// [`Self::advance_farmy_golem_kill_stage`], which are each gated on a
    /// specific incoming stage range).
    pub fn set_farmy_boss_stage(&mut self, stage: i32) {
        self.ensure_farmy_ppd_sized();
        write_i32(&mut self.farmy_ppd, FARMY_PPD_BOSS_STAGE_OFFSET, stage);
    }

    pub fn farmy_boss_timer(&self) -> i32 {
        if self.farmy_ppd.len() < LEGACY_FARMY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.farmy_ppd, FARMY_PPD_BOSS_TIMER_OFFSET)
    }

    pub fn set_farmy_boss_timer(&mut self, value: i32) {
        self.ensure_farmy_ppd_sized();
        write_i32(&mut self.farmy_ppd, FARMY_PPD_BOSS_TIMER_OFFSET, value);
    }

    pub fn farmy_boss_counter(&self) -> i32 {
        if self.farmy_ppd.len() < LEGACY_FARMY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.farmy_ppd, FARMY_PPD_BOSS_COUNTER_OFFSET)
    }

    pub fn set_farmy_boss_counter(&mut self, value: i32) {
        self.ensure_farmy_ppd_sized();
        write_i32(&mut self.farmy_ppd, FARMY_PPD_BOSS_COUNTER_OFFSET, value);
    }

    pub fn farmy_boss_reported(&self) -> i32 {
        if self.farmy_ppd.len() < LEGACY_FARMY_PPD_SIZE {
            return 0;
        }
        read_i32(&self.farmy_ppd, FARMY_PPD_BOSS_REPORTED_OFFSET)
    }

    pub fn set_farmy_boss_reported(&mut self, value: i32) {
        self.ensure_farmy_ppd_sized();
        write_i32(&mut self.farmy_ppd, FARMY_PPD_BOSS_REPORTED_OFFSET, value);
    }

    /// Byte offset of `field` within `struct farmy_ppd::soldier[slot]`
    /// (`src/area/8/fdemon.c:364`). `slot` is clamped to
    /// `0..CDR_FDEMON_ARMY::MAXSOLDIER` by every caller below (out-of-range
    /// reads return `0`, out-of-range writes are a documented no-op) so a
    /// stray slot index can never corrupt `boss_counter`/`boss_reported`,
    /// which sit right after the soldier array.
    fn farmy_soldier_field_offset(slot: usize, field: usize) -> usize {
        FARMY_SOLDIER_ARRAY_OFFSET + slot * FARMY_SOLDIER_STRIDE + field
    }

    fn farmy_soldier_field(&self, slot: usize, field: usize) -> i32 {
        if slot >= crate::world::npc::area8::fdemon_army::MAXSOLDIER
            || self.farmy_ppd.len() < LEGACY_FARMY_PPD_SIZE
        {
            return 0;
        }
        read_i32(
            &self.farmy_ppd,
            Self::farmy_soldier_field_offset(slot, field),
        )
    }

    fn set_farmy_soldier_field(&mut self, slot: usize, field: usize, value: i32) {
        if slot >= crate::world::npc::area8::fdemon_army::MAXSOLDIER {
            return;
        }
        self.ensure_farmy_ppd_sized();
        write_i32(
            &mut self.farmy_ppd,
            Self::farmy_soldier_field_offset(slot, field),
            value,
        );
    }

    /// C `struct soldier::type` (`0`=empty, `1`=warrior, `2`=mage;
    /// `src/area/8/fdemon.c:347`).
    pub fn farmy_soldier_type(&self, slot: usize) -> i32 {
        self.farmy_soldier_field(slot, FARMY_SOLDIER_TYPE_FIELD)
    }

    pub fn set_farmy_soldier_type(&mut self, slot: usize, value: i32) {
        self.set_farmy_soldier_field(slot, FARMY_SOLDIER_TYPE_FIELD, value);
    }

    /// C `struct soldier::rank` (army rank, `fdemon.c:349`).
    pub fn farmy_soldier_rank(&self, slot: usize) -> i32 {
        self.farmy_soldier_field(slot, FARMY_SOLDIER_RANK_FIELD)
    }

    pub fn set_farmy_soldier_rank(&mut self, slot: usize, value: i32) {
        self.set_farmy_soldier_field(slot, FARMY_SOLDIER_RANK_FIELD, value);
    }

    /// C `struct soldier::base` (strength base, `43 + rank * 4`;
    /// `fdemon.c:350,408`).
    pub fn farmy_soldier_base(&self, slot: usize) -> i32 {
        self.farmy_soldier_field(slot, FARMY_SOLDIER_BASE_FIELD)
    }

    pub fn set_farmy_soldier_base(&mut self, slot: usize, value: i32) {
        self.set_farmy_soldier_field(slot, FARMY_SOLDIER_BASE_FIELD, value);
    }

    /// C `struct soldier::profile` (index into `profile[]`, `fdemon.c:351`).
    pub fn farmy_soldier_profile(&self, slot: usize) -> i32 {
        self.farmy_soldier_field(slot, FARMY_SOLDIER_PROFILE_FIELD)
    }

    pub fn set_farmy_soldier_profile(&mut self, slot: usize, value: i32) {
        self.set_farmy_soldier_field(slot, FARMY_SOLDIER_PROFILE_FIELD, value);
    }

    /// C `struct soldier::exp` (`fdemon.c:353`).
    pub fn farmy_soldier_exp(&self, slot: usize) -> i32 {
        self.farmy_soldier_field(slot, FARMY_SOLDIER_EXP_FIELD)
    }

    pub fn set_farmy_soldier_exp(&mut self, slot: usize, value: i32) {
        self.set_farmy_soldier_field(slot, FARMY_SOLDIER_EXP_FIELD, value);
    }

    /// C `struct soldier::cn` (live character id of the spawned soldier, `0`
    /// when not currently spawned; `fdemon.c:354`).
    pub fn farmy_soldier_cn(&self, slot: usize) -> i32 {
        self.farmy_soldier_field(slot, FARMY_SOLDIER_CN_FIELD)
    }

    pub fn set_farmy_soldier_cn(&mut self, slot: usize, value: i32) {
        self.set_farmy_soldier_field(slot, FARMY_SOLDIER_CN_FIELD, value);
    }

    /// C `struct soldier::serial` (`0` when not currently spawned, guards
    /// against a stale `cn` being reused by an unrelated character;
    /// `fdemon.c:355`).
    pub fn farmy_soldier_serial(&self, slot: usize) -> i32 {
        self.farmy_soldier_field(slot, FARMY_SOLDIER_SERIAL_FIELD)
    }

    pub fn set_farmy_soldier_serial(&mut self, slot: usize, value: i32) {
        self.set_farmy_soldier_field(slot, FARMY_SOLDIER_SERIAL_FIELD, value);
    }

    /// C `struct soldier::emote` (`fdemon.c:324-344,358`): the soldier's
    /// personality/chat-engine state, persisted across a recruit/drop/
    /// re-recruit cycle (C `take_soldiers`/`drop_soldiers` copy `dat->
    /// emote` to/from `ppd->soldier[n].emote`, `fdemon.c:559-563,608-612`).
    /// Out-of-range slots return a default (`0`) emote, matching every
    /// other `farmy_soldier_*` accessor.
    pub fn farmy_soldier_emote(
        &self,
        slot: usize,
    ) -> crate::world::npc::area8::fdemon_army_emote::SoldierEmote {
        use crate::world::npc::area8::fdemon_army_emote::SoldierEmote;
        SoldierEmote {
            cuddly: self.farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_CUDDLY_FIELD),
            lonely: self.farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_LONELY_FIELD),
            angst: self.farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_ANGST_FIELD),
            fear: self.farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_FEAR_FIELD),
            bore: self.farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_BORE_FIELD),
            boredom: self.farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_BOREDOM_FIELD),
            bigmouth: self.farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_BIGMOUTH_FIELD),
            praise: self.farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_PRAISE_FIELD),
            likes: std::array::from_fn(|index| {
                self.farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_LIKES_FIELD + index * 4)
            }),
            talked: std::array::from_fn(|index| {
                self.farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_TALKED_FIELD + index * 4)
            }),
            answer_timer: i64::from(
                self.farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_ANSWER_TIMER_FIELD),
            ),
            answer_cn: self.farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_ANSWER_CN_FIELD),
            answer_type: self.farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_ANSWER_TYPE_FIELD),
            last_emote: i64::from(
                self.farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_LAST_EMOTE_FIELD),
            ),
        }
    }

    pub fn set_farmy_soldier_emote(
        &mut self,
        slot: usize,
        emote: &crate::world::npc::area8::fdemon_army_emote::SoldierEmote,
    ) {
        self.set_farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_CUDDLY_FIELD, emote.cuddly);
        self.set_farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_LONELY_FIELD, emote.lonely);
        self.set_farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_ANGST_FIELD, emote.angst);
        self.set_farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_FEAR_FIELD, emote.fear);
        self.set_farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_BORE_FIELD, emote.bore);
        self.set_farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_BOREDOM_FIELD, emote.boredom);
        self.set_farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_BIGMOUTH_FIELD, emote.bigmouth);
        self.set_farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_PRAISE_FIELD, emote.praise);
        for (index, value) in emote.likes.iter().enumerate() {
            self.set_farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_LIKES_FIELD + index * 4, *value);
        }
        for (index, value) in emote.talked.iter().enumerate() {
            self.set_farmy_soldier_field(
                slot,
                FARMY_SOLDIER_EMOTE_TALKED_FIELD + index * 4,
                *value,
            );
        }
        self.set_farmy_soldier_field(
            slot,
            FARMY_SOLDIER_EMOTE_ANSWER_TIMER_FIELD,
            emote.answer_timer as i32,
        );
        self.set_farmy_soldier_field(slot, FARMY_SOLDIER_EMOTE_ANSWER_CN_FIELD, emote.answer_cn);
        self.set_farmy_soldier_field(
            slot,
            FARMY_SOLDIER_EMOTE_ANSWER_TYPE_FIELD,
            emote.answer_type,
        );
        self.set_farmy_soldier_field(
            slot,
            FARMY_SOLDIER_EMOTE_LAST_EMOTE_FIELD,
            emote.last_emote as i32,
        );
    }

    pub fn advance_farmy_blood_stage(&mut self) -> bool {
        let stage = self.farmy_boss_stage();
        if !(19..=20).contains(&stage) {
            return false;
        }
        if self.farmy_ppd.len() < LEGACY_FARMY_PPD_SIZE {
            self.farmy_ppd.resize(LEGACY_FARMY_PPD_SIZE, 0);
        }
        write_i32(&mut self.farmy_ppd, FARMY_PPD_BOSS_STAGE_OFFSET, 21);
        true
    }

    pub fn advance_farmy_lava_stage(&mut self) -> bool {
        let stage = self.farmy_boss_stage();
        if !(22..=23).contains(&stage) {
            return false;
        }
        if self.farmy_ppd.len() < LEGACY_FARMY_PPD_SIZE {
            self.farmy_ppd.resize(LEGACY_FARMY_PPD_SIZE, 0);
        }
        write_i32(&mut self.farmy_ppd, FARMY_PPD_BOSS_STAGE_OFFSET, 24);
        true
    }

    /// C `fdemon_demon_dead`'s `ppd->boss_stage >= 16 && ppd->boss_stage <=
    /// 17` branch (`fdemon.c:2875-2878`): slaying a "Fire Golem"
    /// (`sprite==190`) advances the boss-mission stage to `18` and logs
    /// "Well done. Now go back to the Commander." REMAINING (Area 8 task):
    /// C also credits this to the killer's platoon *leader* when the
    /// killer is a recruited `CDR_FDEMON_ARMY` soldier
    /// (`dat->platoon[MAXSOLDIER]`) rather than the killer itself - not
    /// reachable yet since soldier recruitment isn't ported, so this is
    /// only ever called with the actual player killer's own state.
    pub fn advance_farmy_golem_kill_stage(&mut self) -> bool {
        let stage = self.farmy_boss_stage();
        if !(16..=17).contains(&stage) {
            return false;
        }
        if self.farmy_ppd.len() < LEGACY_FARMY_PPD_SIZE {
            self.farmy_ppd.resize(LEGACY_FARMY_PPD_SIZE, 0);
        }
        write_i32(&mut self.farmy_ppd, FARMY_PPD_BOSS_STAGE_OFFSET, 18);
        true
    }

    pub fn encode_legacy_teufelrat_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_TEUFELRAT_PPD_SIZE];
        write_i32(
            &mut bytes,
            TEUFELRAT_PPD_KILLS_OFFSET,
            self.teufel_rat_kills.min(i32::MAX as u32) as i32,
        );
        write_i32(
            &mut bytes,
            TEUFELRAT_PPD_SCORE_OFFSET,
            self.teufel_rat_score.min(i32::MAX as u32) as i32,
        );
        bytes
    }

    pub fn decode_legacy_teufelrat_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_TEUFELRAT_PPD_SIZE {
            return false;
        }
        self.teufel_rat_kills = read_i32(bytes, TEUFELRAT_PPD_KILLS_OFFSET).max(0) as u32;
        self.teufel_rat_score = read_i32(bytes, TEUFELRAT_PPD_SCORE_OFFSET).max(0) as u32;
        true
    }

    pub fn add_teufel_rat_kill(&mut self, rat_level: u32, reduced_score: bool) -> (u32, u32) {
        let score = if reduced_score {
            1
        } else {
            rat_level.saturating_mul(rat_level) / 100
        };
        self.teufel_rat_kills = self.teufel_rat_kills.saturating_add(1);
        self.teufel_rat_score = self.teufel_rat_score.saturating_add(score);
        (self.teufel_rat_kills, self.teufel_rat_score)
    }

    /// The `PlayerRuntime` half of `turn_seyan`'s ~22 `del_data` calls
    /// (`src/system/tool.c:4331-4353`; the character-only half is
    /// `World::apply_turn_seyan`). 17 of the cleared ids have dedicated
    /// typed fields here - reset each to its empty/default state so
    /// `encode_legacy_ppd_blob` naturally omits the block on next save,
    /// exactly like a character that never touched that system
    /// (`DRD_QUESTLOG_PPD` resets `quest_log` to its default, which
    /// re-triggers `init_questlog`'s "not yet initialized" sentinel on
    /// next load - matching C's del+re-`questlog_init` behavior). The
    /// remaining 3 non-depot ids (`DRD_RANK_PPD`, `DRD_SIDESTORY_PPD`,
    /// `DRD_STRATEGY_PPD`) have no Rust representation at all, so they're
    /// stripped straight out of the raw `ppd_blob` via `strip_ppd_blocks`
    /// (the same byte-level mechanism that already round-trips every
    /// other still-unmodeled id). `DRD_TUNNEL_PPD` graduated from that
    /// stripped-raw list to a real `self.tunnel_ppd.clear()` once
    /// `tunnel_ppd` gained a typed Rust representation (matching C's
    /// `del_data(cn, DRD_TUNNEL_PPD)`, `tool.c:4362` - note `DRD_GORWIN_PPD`
    /// is NOT deleted by `turn_seyan` in C, so `gorwin_ppd` is left alone
    /// here too). `DRD_MILITARY_PPD`
    /// graduated from that stripped-raw list to a real
    /// `self.military_ppd.clear()` once `military_ppd` gained a typed
    /// Rust representation, matching how `first_kill_ppd`/`arena_ppd`
    /// made the same transition earlier. `DRD_DEPOT_PPD`'s "clear
    /// `IF_QUEST` flags from the 80 depot item slots" (`tool.c:4379-4387`
    /// - actually a full slot wipe, `ppd->itm[n].flags = 0`, not just
    /// stripping one flag off a kept item) is ported below now that
    /// `depot` has a typed Rust representation.
    pub fn clear_turn_seyan_ppd(&mut self) {
        for slot in self.depot.iter_mut() {
            if slot
                .as_ref()
                .is_some_and(|item| item.flags.contains(ItemFlags::QUEST))
            {
                *slot = None;
            }
        }
        self.chest_last_access_seconds.clear();
        self.area3_ppd.clear();
        self.area1_ppd.clear();
        self.nomad_ppd.clear();
        self.random_shrine_used_words = [0; RANDOMSHRINE_USED_WORDS];
        self.random_shrine_continuity = 0;
        self.flowers.clear();
        self.random_chests.clear();
        self.demonshrines.clear();
        self.farmy_ppd.clear();
        self.twocity_ppd.clear();
        self.twocity_goodtile = [0; 5];
        self.twocity_solved_library = false;
        self.orb_spawns.clear();
        self.rune_used_words = [0; RUNE_USED_WORDS];
        self.rune_special_exec = [0; RUNE_SPECIAL_EXEC_COUNT];
        self.lab_solved_bits = 0;
        self.lab_ppd.clear();
        self.rat_chests.clear();
        self.rat_chest_treasure_x = 0;
        self.rat_chest_treasure_y = 0;
        self.rat_chest_last_treasure_seconds = 0;
        self.staffer_ppd.clear();
        self.arkhata_ppd.clear();
        self.quest_log = QuestLog::default();
        self.first_kill_ppd.clear();
        self.arena_ppd.clear();
        self.military_ppd.clear();
        self.tunnel_ppd.clear();

        self.ppd_blob = strip_ppd_blocks(
            &self.ppd_blob,
            &[DRD_RANK_PPD, DRD_SIDESTORY_PPD, DRD_STRATEGY_PPD],
        );
    }

    pub fn arkhata_clerk_state(&self) -> i32 {
        if self.arkhata_ppd.len() < LEGACY_ARKHATA_PPD_SIZE {
            return 0;
        }
        read_i32(&self.arkhata_ppd, ARKHATA_PPD_CLERK_STATE_OFFSET)
    }

    /// C `struct arkhata_ppd::rammy_state` (`src/area/37/arkhata.h:5`),
    /// read-only - see [`ARKHATA_PPD_RAMMY_STATE_OFFSET`]'s doc comment.
    pub fn arkhata_rammy_state(&self) -> i32 {
        if self.arkhata_ppd.len() < LEGACY_ARKHATA_PPD_SIZE {
            return 0;
        }
        read_i32(&self.arkhata_ppd, ARKHATA_PPD_RAMMY_STATE_OFFSET)
    }

    pub fn arkhata_clerk_time_seconds(&self) -> i32 {
        if self.arkhata_ppd.len() < LEGACY_ARKHATA_PPD_SIZE {
            return 0;
        }
        read_i32(&self.arkhata_ppd, ARKHATA_PPD_CLERK_TIME_OFFSET)
    }

    pub fn set_arkhata_clerk_timer(&mut self, state: i32, realtime_seconds: i32) {
        if self.arkhata_ppd.len() < LEGACY_ARKHATA_PPD_SIZE {
            self.arkhata_ppd.resize(LEGACY_ARKHATA_PPD_SIZE, 0);
        }
        write_i32(&mut self.arkhata_ppd, ARKHATA_PPD_CLERK_STATE_OFFSET, state);
        write_i32(
            &mut self.arkhata_ppd,
            ARKHATA_PPD_CLERK_TIME_OFFSET,
            realtime_seconds,
        );
    }

    /// C `struct arkhata_ppd::monk_state` (`src/area/37/arkhata.h:9`) -
    /// see [`ARKHATA_PPD_MONK_STATE_OFFSET`]'s doc comment.
    pub fn arkhata_monk_state(&self) -> i32 {
        if self.arkhata_ppd.len() < LEGACY_ARKHATA_PPD_SIZE {
            return 0;
        }
        read_i32(&self.arkhata_ppd, ARKHATA_PPD_MONK_STATE_OFFSET)
    }

    /// C `ppd->monk_state = 20;` (`bookeater_dead`, `src/area/37/
    /// arkhata.c:4350`) plus `arkhatamonk_driver`'s own writes - see
    /// [`ARKHATA_PPD_MONK_STATE_OFFSET`]'s doc comment.
    pub fn set_arkhata_monk_state(&mut self, state: i32) {
        if self.arkhata_ppd.len() < LEGACY_ARKHATA_PPD_SIZE {
            self.arkhata_ppd.resize(LEGACY_ARKHATA_PPD_SIZE, 0);
        }
        write_i32(&mut self.arkhata_ppd, ARKHATA_PPD_MONK_STATE_OFFSET, state);
    }

    /// C `struct arkhata_ppd::monk_bits` (`src/area/37/arkhata.h:10`) -
    /// see [`ARKHATA_PPD_MONK_BITS_OFFSET`]'s doc comment.
    pub fn arkhata_monk_bits(&self) -> i32 {
        if self.arkhata_ppd.len() < LEGACY_ARKHATA_PPD_SIZE {
            return 0;
        }
        read_i32(&self.arkhata_ppd, ARKHATA_PPD_MONK_BITS_OFFSET)
    }

    /// C `ppd->monk_bits |= N;` (`arkhatamonk_driver`, `src/area/37/
    /// arkhata.c:1995,2007,2020`) - see [`ARKHATA_PPD_MONK_BITS_OFFSET`]'s
    /// doc comment.
    pub fn set_arkhata_monk_bits(&mut self, bits: i32) {
        if self.arkhata_ppd.len() < LEGACY_ARKHATA_PPD_SIZE {
            self.arkhata_ppd.resize(LEGACY_ARKHATA_PPD_SIZE, 0);
        }
        write_i32(&mut self.arkhata_ppd, ARKHATA_PPD_MONK_BITS_OFFSET, bits);
    }

    /// C `struct arkhata_ppd::ramin_state` (`src/area/37/arkhata.h:7`) -
    /// see [`ARKHATA_PPD_RAMIN_STATE_OFFSET`]'s doc comment.
    pub fn arkhata_ramin_state(&self) -> i32 {
        if self.arkhata_ppd.len() < LEGACY_ARKHATA_PPD_SIZE {
            return 0;
        }
        read_i32(&self.arkhata_ppd, ARKHATA_PPD_RAMIN_STATE_OFFSET)
    }

    /// C `ppd->ramin_state = 7;` (`arkhataskelly_dead`, `src/area/37/
    /// arkhata.c:1642`): the only writer of this field ported so far -
    /// see [`ARKHATA_PPD_RAMIN_STATE_OFFSET`]'s doc comment.
    pub fn set_arkhata_ramin_state(&mut self, state: i32) {
        if self.arkhata_ppd.len() < LEGACY_ARKHATA_PPD_SIZE {
            self.arkhata_ppd.resize(LEGACY_ARKHATA_PPD_SIZE, 0);
        }
        write_i32(&mut self.arkhata_ppd, ARKHATA_PPD_RAMIN_STATE_OFFSET, state);
    }

    /// C `struct arkhata_ppd::fiona_state` (`src/area/37/arkhata.h:8`) -
    /// see [`ARKHATA_PPD_FIONA_STATE_OFFSET`]'s doc comment.
    pub fn arkhata_fiona_state(&self) -> i32 {
        if self.arkhata_ppd.len() < LEGACY_ARKHATA_PPD_SIZE {
            return 0;
        }
        read_i32(&self.arkhata_ppd, ARKHATA_PPD_FIONA_STATE_OFFSET)
    }

    /// C `struct arkhata_ppd::fiona_state` write half - see
    /// [`ARKHATA_PPD_FIONA_STATE_OFFSET`]'s doc comment. `fiona_driver`
    /// (`world::npc::area37::fiona`) is the only writer.
    pub fn set_arkhata_fiona_state(&mut self, state: i32) {
        if self.arkhata_ppd.len() < LEGACY_ARKHATA_PPD_SIZE {
            self.arkhata_ppd.resize(LEGACY_ARKHATA_PPD_SIZE, 0);
        }
        write_i32(&mut self.arkhata_ppd, ARKHATA_PPD_FIONA_STATE_OFFSET, state);
    }

    /// C `struct arkhata_ppd::rammy_state` write half - see
    /// [`ARKHATA_PPD_RAMMY_STATE_OFFSET`]'s doc comment. `rammy_driver`
    /// (`world::npc::area37::rammy`) is the only writer.
    pub fn set_arkhata_rammy_state(&mut self, state: i32) {
        if self.arkhata_ppd.len() < LEGACY_ARKHATA_PPD_SIZE {
            self.arkhata_ppd.resize(LEGACY_ARKHATA_PPD_SIZE, 0);
        }
        write_i32(&mut self.arkhata_ppd, ARKHATA_PPD_RAMMY_STATE_OFFSET, state);
    }

    /// C `struct arkhata_ppd::jaz_state` (`src/area/37/arkhata.h:6`) -
    /// see [`ARKHATA_PPD_JAZ_STATE_OFFSET`]'s doc comment.
    pub fn arkhata_jaz_state(&self) -> i32 {
        if self.arkhata_ppd.len() < LEGACY_ARKHATA_PPD_SIZE {
            return 0;
        }
        read_i32(&self.arkhata_ppd, ARKHATA_PPD_JAZ_STATE_OFFSET)
    }

    /// C `struct arkhata_ppd::jaz_state` write half - see
    /// [`ARKHATA_PPD_JAZ_STATE_OFFSET`]'s doc comment. `jaz_driver`
    /// (`world::npc::area37::jaz`) is the only writer.
    pub fn set_arkhata_jaz_state(&mut self, state: i32) {
        if self.arkhata_ppd.len() < LEGACY_ARKHATA_PPD_SIZE {
            self.arkhata_ppd.resize(LEGACY_ARKHATA_PPD_SIZE, 0);
        }
        write_i32(&mut self.arkhata_ppd, ARKHATA_PPD_JAZ_STATE_OFFSET, state);
    }

    /// C `struct arkhata_ppd::letter_bits` - see
    /// [`ARKHATA_PPD_LETTER_BITS_OFFSET`]'s doc comment.
    pub fn arkhata_letter_bits(&self) -> i32 {
        if self.arkhata_ppd.len() < LEGACY_ARKHATA_PPD_SIZE {
            return 0;
        }
        read_i32(&self.arkhata_ppd, ARKHATA_PPD_LETTER_BITS_OFFSET)
    }

    /// C `ppd->letter_bits |= bit;`/`ppd2->letter_bits |= bit;` - written
    /// by `ramin_driver`/`captain_driver` (still unported, bits `2`/`8`)
    /// and `count_brannington_driver`'s cross-area `IID_ARKHATA_LETTER3`
    /// turn-in (`world::npc::area29::countbran`, bit `4`).
    pub fn set_arkhata_letter_bits(&mut self, bits: i32) {
        if self.arkhata_ppd.len() < LEGACY_ARKHATA_PPD_SIZE {
            self.arkhata_ppd.resize(LEGACY_ARKHATA_PPD_SIZE, 0);
        }
        write_i32(&mut self.arkhata_ppd, ARKHATA_PPD_LETTER_BITS_OFFSET, bits);
    }

    fn ensure_caligar_ppd_sized(&mut self) {
        if self.caligar_ppd.len() < LEGACY_CALIGAR_PPD_SIZE {
            self.caligar_ppd.resize(LEGACY_CALIGAR_PPD_SIZE, 0);
        }
    }

    /// C `struct caligar_ppd::guard_state` (`caligar.c:217`).
    pub fn caligar_guard_state(&self) -> i32 {
        if self.caligar_ppd.len() < LEGACY_CALIGAR_PPD_SIZE {
            return 0;
        }
        read_i32(&self.caligar_ppd, CALIGAR_PPD_GUARD_STATE_OFFSET)
    }

    /// C `struct caligar_ppd::guard_last_talk` (`caligar.c:218`).
    pub fn caligar_guard_last_talk(&self) -> i32 {
        if self.caligar_ppd.len() < LEGACY_CALIGAR_PPD_SIZE {
            return 0;
        }
        read_i32(&self.caligar_ppd, CALIGAR_PPD_GUARD_LAST_TALK_OFFSET)
    }

    /// C `ppd->guard_state++; ppd->guard_last_talk = realtime;`
    /// (`guard_driver`, `caligar.c:294-334`).
    pub fn set_caligar_guard_talk(&mut self, state: i32, realtime_seconds: i32) {
        self.ensure_caligar_ppd_sized();
        write_i32(&mut self.caligar_ppd, CALIGAR_PPD_GUARD_STATE_OFFSET, state);
        write_i32(
            &mut self.caligar_ppd,
            CALIGAR_PPD_GUARD_LAST_TALK_OFFSET,
            realtime_seconds,
        );
    }

    /// C `case 2:` (`analyse_text_driver` code `2`, "repeat"/"restart"):
    /// `ppd->guard_state == 3` resets back to `0` (`caligar.c:372-378`).
    pub fn reset_caligar_guard_if_state_three(&mut self) {
        if self.caligar_guard_state() != 3 {
            return;
        }
        self.set_caligar_guard_talk(0, 0);
    }

    /// C `case 5: if (realtime - ppd->guard_last_talk > 600)
    /// ppd->guard_state = 0;` (`caligar.c:336-338`) - only `guard_state`
    /// is written, `guard_last_talk` is left untouched.
    pub fn reset_caligar_guard_state_timeout(&mut self) {
        self.ensure_caligar_ppd_sized();
        write_i32(&mut self.caligar_ppd, CALIGAR_PPD_GUARD_STATE_OFFSET, 0);
    }

    /// C `struct caligar_ppd::guard2_last_talk` (`caligar.c:230`).
    pub fn caligar_guard2_last_talk(&self) -> i32 {
        if self.caligar_ppd.len() < LEGACY_CALIGAR_PPD_SIZE {
            return 0;
        }
        read_i32(&self.caligar_ppd, CALIGAR_PPD_GUARD2_LAST_TALK_OFFSET)
    }

    /// C `ppd->guard2_last_talk = realtime;` (`guard2_driver`,
    /// `caligar.c:438`).
    pub fn set_caligar_guard2_last_talk(&mut self, realtime_seconds: i32) {
        self.ensure_caligar_ppd_sized();
        write_i32(
            &mut self.caligar_ppd,
            CALIGAR_PPD_GUARD2_LAST_TALK_OFFSET,
            realtime_seconds,
        );
    }

    /// C `struct caligar_ppd::glori_state` (`caligar.c:219`).
    pub fn caligar_glori_state(&self) -> i32 {
        if self.caligar_ppd.len() < LEGACY_CALIGAR_PPD_SIZE {
            return 0;
        }
        read_i32(&self.caligar_ppd, CALIGAR_PPD_GLORI_STATE_OFFSET)
    }

    /// C `struct caligar_ppd::glori_last_talk` (`caligar.c:220`).
    pub fn caligar_glori_last_talk(&self) -> i32 {
        if self.caligar_ppd.len() < LEGACY_CALIGAR_PPD_SIZE {
            return 0;
        }
        read_i32(&self.caligar_ppd, CALIGAR_PPD_GLORI_LAST_TALK_OFFSET)
    }

    /// C `ppd->glori_state++; ppd->glori_last_talk = realtime;`
    /// (`glori_driver`, `caligar.c:567-709`).
    pub fn set_caligar_glori_talk(&mut self, state: i32, realtime_seconds: i32) {
        self.ensure_caligar_ppd_sized();
        write_i32(&mut self.caligar_ppd, CALIGAR_PPD_GLORI_STATE_OFFSET, state);
        write_i32(
            &mut self.caligar_ppd,
            CALIGAR_PPD_GLORI_LAST_TALK_OFFSET,
            realtime_seconds,
        );
    }

    /// C `ppd->watch_flag` read for `glori_driver`'s `case 5:` gate
    /// (`caligar.c:605`, `ppd->watch_flag < (1|2|4)`).
    pub fn caligar_watch_flag(&self) -> i32 {
        if self.caligar_ppd.len() < LEGACY_CALIGAR_PPD_SIZE {
            return 0;
        }
        read_i32(&self.caligar_ppd, CALIGAR_PPD_WATCH_FLAG_OFFSET)
    }

    /// C `analyse_text_driver` code `2` resetting `glori_state` back to
    /// the start of whichever mini-block is in progress (`caligar.c:759-
    /// 782`): `1..=5 -> 1`, `8..=10 -> 8`, `11..=12 -> 11`, `14..=16 ->
    /// 14`, `17..=18 -> 17`.
    pub fn reset_caligar_glori_to_mini_block_start(&mut self) {
        let state = self.caligar_glori_state();
        let reset_to = match state {
            1..=5 => Some(1),
            8..=10 => Some(8),
            11..=12 => Some(11),
            14..=16 => Some(14),
            17..=18 => Some(17),
            _ => None,
        };
        if let Some(reset_to) = reset_to {
            self.set_caligar_glori_talk(reset_to, 0);
        }
    }

    /// C `struct caligar_ppd::arquin_state` (`caligar.c:223`).
    pub fn caligar_arquin_state(&self) -> i32 {
        if self.caligar_ppd.len() < LEGACY_CALIGAR_PPD_SIZE {
            return 0;
        }
        read_i32(&self.caligar_ppd, CALIGAR_PPD_ARQUIN_STATE_OFFSET)
    }

    /// C `struct caligar_ppd::arquin_last_talk` (`caligar.c:224`).
    pub fn caligar_arquin_last_talk(&self) -> i32 {
        if self.caligar_ppd.len() < LEGACY_CALIGAR_PPD_SIZE {
            return 0;
        }
        read_i32(&self.caligar_ppd, CALIGAR_PPD_ARQUIN_LAST_TALK_OFFSET)
    }

    /// C `ppd->arquin_state++; ppd->arquin_last_talk = realtime;`
    /// (`arquin_driver`, `caligar.c:853-897`).
    pub fn set_caligar_arquin_talk(&mut self, state: i32, realtime_seconds: i32) {
        self.ensure_caligar_ppd_sized();
        write_i32(
            &mut self.caligar_ppd,
            CALIGAR_PPD_ARQUIN_STATE_OFFSET,
            state,
        );
        write_i32(
            &mut self.caligar_ppd,
            CALIGAR_PPD_ARQUIN_LAST_TALK_OFFSET,
            realtime_seconds,
        );
    }

    /// C `analyse_text_driver` code `2` resetting `arquin_state`
    /// (`caligar.c:935-942`): `1..=3 -> 1`, `4..=6 -> 4`.
    pub fn reset_caligar_arquin_to_mini_block_start(&mut self) {
        let state = self.caligar_arquin_state();
        let reset_to = match state {
            1..=3 => Some(1),
            4..=6 => Some(4),
            _ => None,
        };
        if let Some(reset_to) = reset_to {
            self.set_caligar_arquin_talk(reset_to, 0);
        }
    }

    /// C `struct caligar_ppd::smith_state` (`caligar.c:225`).
    pub fn caligar_smith_state(&self) -> i32 {
        if self.caligar_ppd.len() < LEGACY_CALIGAR_PPD_SIZE {
            return 0;
        }
        read_i32(&self.caligar_ppd, CALIGAR_PPD_SMITH_STATE_OFFSET)
    }

    /// C `struct caligar_ppd::smith_last_talk` (`caligar.c:226`).
    pub fn caligar_smith_last_talk(&self) -> i32 {
        if self.caligar_ppd.len() < LEGACY_CALIGAR_PPD_SIZE {
            return 0;
        }
        read_i32(&self.caligar_ppd, CALIGAR_PPD_SMITH_LAST_TALK_OFFSET)
    }

    /// C `ppd->smith_state++; ppd->smith_last_talk = realtime;`
    /// (`smith_driver`, `caligar.c:1015-1076`).
    pub fn set_caligar_smith_talk(&mut self, state: i32, realtime_seconds: i32) {
        self.ensure_caligar_ppd_sized();
        write_i32(&mut self.caligar_ppd, CALIGAR_PPD_SMITH_STATE_OFFSET, state);
        write_i32(
            &mut self.caligar_ppd,
            CALIGAR_PPD_SMITH_LAST_TALK_OFFSET,
            realtime_seconds,
        );
    }

    /// C `analyse_text_driver` code `2` resetting `smith_state`
    /// (`caligar.c:1114-1121`): `1..=2 -> 1`, `3..=8 -> 3`.
    pub fn reset_caligar_smith_to_mini_block_start(&mut self) {
        let state = self.caligar_smith_state();
        let reset_to = match state {
            1..=2 => Some(1),
            3..=8 => Some(3),
            _ => None,
        };
        if let Some(reset_to) = reset_to {
            self.set_caligar_smith_talk(reset_to, 0);
        }
    }

    /// C `struct caligar_ppd::homden_state` (`caligar.c:227`).
    pub fn caligar_homden_state(&self) -> i32 {
        if self.caligar_ppd.len() < LEGACY_CALIGAR_PPD_SIZE {
            return 0;
        }
        read_i32(&self.caligar_ppd, CALIGAR_PPD_HOMDEN_STATE_OFFSET)
    }

    /// C `struct caligar_ppd::homden_last_talk` (`caligar.c:228`).
    pub fn caligar_homden_last_talk(&self) -> i32 {
        if self.caligar_ppd.len() < LEGACY_CALIGAR_PPD_SIZE {
            return 0;
        }
        read_i32(&self.caligar_ppd, CALIGAR_PPD_HOMDEN_LAST_TALK_OFFSET)
    }

    /// C `ppd->homden_state++; ppd->homden_last_talk = realtime;`
    /// (`homden_driver`, `caligar.c:1242-1319`).
    pub fn set_caligar_homden_talk(&mut self, state: i32, realtime_seconds: i32) {
        self.ensure_caligar_ppd_sized();
        write_i32(
            &mut self.caligar_ppd,
            CALIGAR_PPD_HOMDEN_STATE_OFFSET,
            state,
        );
        write_i32(
            &mut self.caligar_ppd,
            CALIGAR_PPD_HOMDEN_LAST_TALK_OFFSET,
            realtime_seconds,
        );
    }

    /// C `analyse_text_driver` code `2` resetting `homden_state`
    /// (`caligar.c:1366-1373`): `2..=4 -> 2`, `6..=11 -> 6`.
    pub fn reset_caligar_homden_to_mini_block_start(&mut self) {
        let state = self.caligar_homden_state();
        let reset_to = match state {
            2..=4 => Some(2),
            6..=11 => Some(6),
            _ => None,
        };
        if let Some(reset_to) = reset_to {
            self.set_caligar_homden_talk(reset_to, 0);
        }
    }

    pub fn observe_caligar_training(&mut self, lesson: u8) -> Option<bool> {
        let bit = match lesson {
            1 => 1,
            2 => 4,
            3 => 2,
            _ => return None,
        };
        if self.caligar_ppd.len() < LEGACY_CALIGAR_PPD_SIZE {
            self.caligar_ppd.resize(LEGACY_CALIGAR_PPD_SIZE, 0);
        }
        let watch_flag = read_i32(&self.caligar_ppd, CALIGAR_PPD_WATCH_FLAG_OFFSET);
        let was_new = watch_flag & bit == 0;
        write_i32(
            &mut self.caligar_ppd,
            CALIGAR_PPD_WATCH_FLAG_OFFSET,
            watch_flag | bit,
        );
        Some(was_new)
    }

    pub fn caligar_skelly_door_unlocked(&self, door_index: u8) -> bool {
        let idx = usize::from(door_index);
        idx < CALIGAR_PPD_DOOR_FLAG_COUNT
            && self.caligar_ppd.len() >= LEGACY_CALIGAR_PPD_SIZE
            && self.caligar_ppd[CALIGAR_PPD_DOOR_FLAG_OFFSET + idx] & 0x07 == 0x07
    }

    pub fn mark_caligar_skelly_death(
        &mut self,
        home_x: u16,
        home_y: u16,
    ) -> CaligarSkellyDeathResult {
        let (door_index, lock_number) = match (home_x, home_y) {
            (103, 224) => (0, 0),
            (103, 211) => (0, 1),
            (103, 198) => (0, 2),
            (145, 225) => (1, 0),
            (145, 212) => (1, 1),
            (145, 186) => (1, 2),
            (226 | 227, 158) => (2, 0),
            (226 | 227, 145) => (2, 1),
            (226 | 227, 132) => (2, 2),
            _ => {
                return CaligarSkellyDeathResult::Unmapped {
                    x: home_x,
                    y: home_y,
                };
            }
        };

        if self.caligar_ppd.len() < LEGACY_CALIGAR_PPD_SIZE {
            self.caligar_ppd.resize(LEGACY_CALIGAR_PPD_SIZE, 0);
        }

        let bit = 1u8 << lock_number;
        let offset = CALIGAR_PPD_DOOR_FLAG_OFFSET + door_index;
        if self.caligar_ppd[offset] & bit != 0 {
            return CaligarSkellyDeathResult::AlreadyUnlocked {
                door_index: door_index as u8,
                bit,
            };
        }

        self.caligar_ppd[offset] |= bit;
        if self.caligar_ppd[offset] & 0x07 == 0x07 {
            CaligarSkellyDeathResult::FullyUnlocked {
                door_index: door_index as u8,
                bit,
            }
        } else {
            CaligarSkellyDeathResult::PartiallyUnlocked {
                door_index: door_index as u8,
                bit,
            }
        }
    }
}
