use super::*;

impl PlayerRuntime {
    /// C `struct quest quest[MAXQUEST]` (`src/system/questlog.h:36-39`):
    /// one byte per quest, `done` packed into the low 6 bits and `flags`
    /// into the high 2 bits (x86 GCC allocates bitfields LSB-first, so
    /// `done` - declared first - occupies bits 0-5).
    pub fn encode_legacy_questlog_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0u8; LEGACY_QUESTLOG_PPD_SIZE];
        for (index, entry) in self.quest_log.entries().iter().enumerate() {
            bytes[index] = (entry.done & 0x3f) | ((entry.flags & 0x3) << 6);
        }
        bytes
    }

    pub fn decode_legacy_questlog_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_QUESTLOG_PPD_SIZE {
            return false;
        }
        for (index, byte) in bytes[..LEGACY_QUESTLOG_PPD_SIZE].iter().enumerate() {
            self.quest_log
                .set_raw(index, byte & 0x3f, (byte >> 6) & 0x3);
        }
        true
    }

    /// C `questlog_init` (`src/system/questlog.c:1610-1626`): the
    /// top-level dispatcher that lazily seeds every area's questlog
    /// entries the first time a character is loaded, guarded by the
    /// `quest[MAXQUEST - 1].done == 55` sentinel so it only ever runs
    /// once per character. Builds the plain snapshot structs each
    /// `init_*_quests` sub-function needs from the typed PPD accessors
    /// (this method has full `PlayerRuntime` access, unlike the leaf
    /// `quest` module functions it calls).
    pub fn init_questlog(&mut self) {
        if self.quest_log.is_init_complete() {
            return;
        }

        let area1 = self.area1_quest_state();
        let area3 = self.area3_quest_state();
        let staff = self.staff_quest_state();
        let twocity = self.twocity_quest_state();
        let nomad = self.nomad_quest_state();

        crate::quest::init_area1_quests(&mut self.quest_log, &area1);
        crate::quest::init_area3_quests(&mut self.quest_log, &area3);
        crate::quest::init_staff_quests(&mut self.quest_log, &staff);
        crate::quest::init_twocity_quests(&mut self.quest_log, &twocity);
        crate::quest::init_nomad_quests(&mut self.quest_log, &nomad);

        self.quest_log.mark_init_complete();
    }

    /// C `questlog_reopen(cn, qnr)` (`src/system/questlog.c:613-826`): the
    /// live `CL_REOPENQUEST` handler dispatch. Runs the generic
    /// preconditions (`QuestLog::reopen_precheck`), then the per-quest
    /// `questlog_reopen_qN` switch - each arm's area-PPD side effect plus
    /// the "cannot re-open more than one quest from a series"
    /// exclusivity check against sibling quest numbers' `QF_OPEN` flags -
    /// before finally opening the quest (`quest[qnr].flags = QF_OPEN`)
    /// only when the switch's `ret` stayed truthy, exactly matching C
    /// (including the switch's unimplemented/dead-code cases that force
    /// `ret = 0` and silently fail to reopen despite passing every
    /// precondition, and `case 36`'s missing `break;` that falls through
    /// into `case 37`'s helper call).
    pub fn reopen_quest_legacy(&mut self, qnr: usize) -> crate::quest::QuestReopenResult {
        use crate::quest::QuestReopenResult;

        if let Err(result) = self.quest_log.reopen_precheck(qnr) {
            return result;
        }

        match self.reopen_dispatch(qnr) {
            ReopenOutcome::Open => {
                self.quest_log.open(qnr);
                QuestReopenResult::Reopened
            }
            ReopenOutcome::SeriesConflict => QuestReopenResult::SeriesConflict,
            ReopenOutcome::NoEffect => QuestReopenResult::NoEffect,
        }
    }

    /// The `switch (qnr)` body of C `questlog_reopen`
    /// (`src/system/questlog.c:637-817`), with no precondition gating -
    /// split out from `reopen_quest_legacy` purely so tests can exercise
    /// individual arms (including ones that are unreachable in practice
    /// because `QuestLog::reopen_precheck` rejects their quest number
    /// first, like the dead `case 6`/`case 22`/`case 36` arms below,
    /// whose table row has no `QLF_REPEATABLE`/`QLF_XREPEAT` flags at
    /// all) directly.
    pub(crate) fn reopen_dispatch(&mut self, qnr: usize) -> ReopenOutcome {
        match qnr {
            0 => {
                // `questlog_reopen_q0` (`questlog.c:342-351`).
                self.set_area1_james_state(0);
                self.set_area1_lydia_state(0);
                ReopenOutcome::Open
            }
            1 => self.reopen_gwendy(crate::quest::GWENDYLON_STATE_ENTRY),
            2 => self.reopen_gwendy(crate::quest::GWENDYLON_STATE_FIRST_SKULL_DONE),
            3 => self.reopen_gwendy(crate::quest::GWENDYLON_STATE_SECOND_SKULL_DONE),
            4 => self.reopen_gwendy(crate::quest::GWENDYLON_STATE_THIRD_SKULL_DONE),
            5 => {
                // `questlog_reopen_q5` (`questlog.c:369-377`).
                self.set_area1_yoakin_state(0);
                ReopenOutcome::Open
            }
            7 => self.reopen_guiwynn(0),
            8 => self.reopen_guiwynn(6),
            9 => {
                // `questlog_reopen_q9` (`questlog.c:404-412`).
                self.set_area1_logain_state(0);
                ReopenOutcome::Open
            }
            12 => self.reopen_seymour(),
            13 => {
                // `questlog_reopen_q13` (`questlog.c:427-433`).
                self.set_area3_kelly_state(0);
                ReopenOutcome::Open
            }
            16 => {
                // `questlog_reopen_q16` (`questlog.c:435-441`).
                self.set_area3_astro2_state(0);
                ReopenOutcome::Open
            }
            20 => {
                // `questlog_reopen_q20` (`questlog.c:456-462`).
                self.set_staffer_carlos_state(0);
                ReopenOutcome::Open
            }
            22 => self.reopen_william(),
            30 => {
                // `questlog_reopen_q30` (`questlog.c:479-485`).
                self.set_twocity_skelly_state(0);
                ReopenOutcome::Open
            }
            31 => {
                // `questlog_reopen_q31` (`questlog.c:487-493`).
                self.set_twocity_alchemist_state(0);
                ReopenOutcome::Open
            }
            35 => self.reopen_smugglecom(0),
            // C `questlog_reopen`'s `case 36` has no `break;`
            // (`questlog.c:746-750`), so it falls through into `case
            // 37`'s `questlog_reopen_q35(cn, 7, quest)` instead of doing
            // nothing - faithfully reproduced rather than "fixed".
            36 | 37 => self.reopen_smugglecom(7),
            38 => {
                // `questlog_reopen_q38` (`questlog.c:511-517`).
                self.set_staffer_aristocrat_state(0);
                ReopenOutcome::Open
            }
            39 => {
                // `questlog_reopen_q39` (`questlog.c:519-525`).
                self.set_staffer_yoatin_state(0);
                ReopenOutcome::Open
            }
            40 => {
                // `questlog_reopen_q40` (`questlog.c:527-534`).
                self.set_staffer_countbran_state(0);
                let bits = self.staffer_countbran_bits();
                self.set_staffer_countbran_bits(bits & !(1 | 2 | 4));
                ReopenOutcome::Open
            }
            41 => self.reopen_brennethbran(0),
            42 => self.reopen_brennethbran(5),
            43 => self.reopen_brennethbran(9),
            44 => {
                // `questlog_reopen_q44` (`questlog.c:548-554`).
                self.set_staffer_spiritbran_state(0);
                ReopenOutcome::Open
            }
            45 => self.reopen_broklin(0),
            crate::quest::QLOG_JESSICA_ROBBER_NOTE => self.reopen_jessica_note(),
            crate::quest::QLOG_HERMIT_QUEST2 => {
                // `questlog_reopen_q83` (`questlog.c:586-594`).
                self.set_area1_camhermit_state(crate::quest::CAMHERMIT_STATE_QUEST2_1);
                ReopenOutcome::Open
            }
            crate::quest::QLOG_JESSICA_KILL => self.reopen_jessica_kill(),
            // Every other `case` in the C switch either has no arm at
            // all (falls to the implicit `switch` default) or explicitly
            // sets `ret = 0` with no helper call (cases 6, 10, 11, 14,
            // 15, 17-19, 21, 23-29, 32-34, 46-54, 80, 81) - a silent
            // no-op in C.
            _ => ReopenOutcome::NoEffect,
        }
    }

    /// `questlog_reopen_q1` (`src/system/questlog.c:353-367`): shared by
    /// reopen cases 1-4 (Gwendylon's skull-hunt series).
    pub(crate) fn reopen_gwendy(&mut self, state: i32) -> ReopenOutcome {
        if self
            .quest_log
            .is_open(crate::quest::QLOG_GWENDY_FIRST_SKULL)
            || self
                .quest_log
                .is_open(crate::quest::QLOG_GWENDY_SECOND_SKULL)
            || self
                .quest_log
                .is_open(crate::quest::QLOG_GWENDY_THIRD_SKULL)
            || self
                .quest_log
                .is_open(crate::quest::QLOG_GWENDY_FOUL_MAGICIAN)
        {
            return ReopenOutcome::SeriesConflict;
        }
        self.set_area1_gwendy_state(state);
        ReopenOutcome::Open
    }

    /// `questlog_reopen_q7` (`src/system/questlog.c:389-402`): shared by
    /// reopen cases 7-8 (Guiwynn's "Mages Gone Berserk" series).
    pub(crate) fn reopen_guiwynn(&mut self, state: i32) -> ReopenOutcome {
        if self.quest_log.is_open(7) || self.quest_log.is_open(8) {
            return ReopenOutcome::SeriesConflict;
        }
        self.set_area1_guiwynn_state(state);
        ReopenOutcome::Open
    }

    /// `questlog_reopen_q10` (`src/system/questlog.c:414-425`): only ever
    /// reached via reopen case 12, always with `state = 12`.
    pub(crate) fn reopen_seymour(&mut self) -> ReopenOutcome {
        if self.quest_log.is_open(10) || self.quest_log.is_open(11) || self.quest_log.is_open(12) {
            return ReopenOutcome::SeriesConflict;
        }
        self.set_area3_seymour_state(12);
        ReopenOutcome::Open
    }

    /// `questlog_reopen_q22` (`src/system/questlog.c:464-477`).
    pub(crate) fn reopen_william(&mut self) -> ReopenOutcome {
        if self.quest_log.is_open(22) || self.quest_log.is_open(23) {
            return ReopenOutcome::SeriesConflict;
        }
        self.set_area3_william_state(0);
        self.set_area3_imp_state(0);
        self.set_area3_imp_kills(0);
        ReopenOutcome::Open
    }

    /// `questlog_reopen_q35` (`src/system/questlog.c:495-509`): shared by
    /// reopen cases 35, 36 (via the case-36 fallthrough), and 37.
    pub(crate) fn reopen_smugglecom(&mut self, state: i32) -> ReopenOutcome {
        if self.quest_log.is_open(35) || self.quest_log.is_open(36) || self.quest_log.is_open(37) {
            return ReopenOutcome::SeriesConflict;
        }
        if state == 5 {
            self.set_staffer_smugglecom_bits(0);
        }
        self.set_staffer_smugglecom_state(state);
        ReopenOutcome::Open
    }

    /// `questlog_reopen_q41` (`src/system/questlog.c:536-546`): shared by
    /// reopen cases 41-43.
    pub(crate) fn reopen_brennethbran(&mut self, state: i32) -> ReopenOutcome {
        if self.quest_log.is_open(41) || self.quest_log.is_open(42) || self.quest_log.is_open(43) {
            return ReopenOutcome::SeriesConflict;
        }
        self.set_staffer_brennethbran_state(state);
        ReopenOutcome::Open
    }

    /// `questlog_reopen_q45` (`src/system/questlog.c:556-566`): only ever
    /// reached via reopen case 45, always with `state = 0`.
    pub(crate) fn reopen_broklin(&mut self, state: i32) -> ReopenOutcome {
        if self.quest_log.is_open(45) || self.quest_log.is_open(46) {
            return ReopenOutcome::SeriesConflict;
        }
        self.set_staffer_broklin_state(state);
        ReopenOutcome::Open
    }

    /// `questlog_reopen_q79` (`src/system/questlog.c:569-583`): Jessica's
    /// "collect the robber's note" reopen.
    pub(crate) fn reopen_jessica_note(&mut self) -> ReopenOutcome {
        if self.quest_log.is_open(crate::quest::QLOG_JESSICA_KILL) {
            return ReopenOutcome::SeriesConflict;
        }
        self.set_area1_jessica_state(crate::quest::JESSICA_STATE_QUEST1_GIVE_1);
        ReopenOutcome::Open
    }

    /// `questlog_reopen_q84` (`src/system/questlog.c:597-611`): Jessica's
    /// "kill the robber boss" reopen.
    pub(crate) fn reopen_jessica_kill(&mut self) -> ReopenOutcome {
        if self
            .quest_log
            .is_open(crate::quest::QLOG_JESSICA_ROBBER_NOTE)
        {
            return ReopenOutcome::SeriesConflict;
        }
        self.set_area1_jessica_state(crate::quest::JESSICA_STATE_QUEST2_GIVE_1);
        ReopenOutcome::Open
    }
}
