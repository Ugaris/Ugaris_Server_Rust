use serde::{Deserialize, Serialize};

pub const MAX_QUESTS: usize = 100;
pub const QF_OPEN: u8 = 1;
pub const QF_DONE: u8 = 2;
pub const QLF_REPEATABLE: u8 = 1;

pub const QLOG_LYDIA: usize = 0;
pub const QLOG_GWENDY_FIRST_SKULL: usize = 1;
pub const QLOG_GWENDY_SECOND_SKULL: usize = 2;
pub const QLOG_GWENDY_THIRD_SKULL: usize = 3;
pub const QLOG_GWENDY_FOUL_MAGICIAN: usize = 4;
pub const QLOG_NOOK: usize = 6;
pub const QLOG_JESSICA_ROBBER_NOTE: usize = 79;
pub const QLOG_JIU: usize = 80;
pub const QLOG_BRITHILDIE: usize = 81;
pub const QLOG_HERMIT_QUEST1: usize = 82;
pub const QLOG_HERMIT_QUEST2: usize = 83;
pub const QLOG_JESSICA_KILL: usize = 84;

const QUESTLOG_FLAGS: [u8; MAX_QUESTS] = {
    let mut flags = [0; MAX_QUESTS];
    flags[0] = QLF_REPEATABLE;
    flags[1] = QLF_REPEATABLE;
    flags[2] = QLF_REPEATABLE;
    flags[3] = QLF_REPEATABLE;
    flags[4] = QLF_REPEATABLE;
    flags[5] = QLF_REPEATABLE;
    flags[7] = QLF_REPEATABLE;
    flags[8] = QLF_REPEATABLE;
    flags[9] = QLF_REPEATABLE;
    flags[12] = QLF_REPEATABLE;
    flags[13] = QLF_REPEATABLE;
    flags[16] = QLF_REPEATABLE;
    flags[20] = QLF_REPEATABLE;
    flags[30] = QLF_REPEATABLE;
    flags[31] = QLF_REPEATABLE;
    flags[35] = QLF_REPEATABLE;
    flags[37] = QLF_REPEATABLE;
    flags[38] = QLF_REPEATABLE;
    flags[39] = QLF_REPEATABLE;
    flags[40] = QLF_REPEATABLE;
    flags[41] = QLF_REPEATABLE;
    flags[42] = QLF_REPEATABLE;
    flags[43] = QLF_REPEATABLE;
    flags[44] = QLF_REPEATABLE;
    flags[45] = QLF_REPEATABLE;
    flags[79] = QLF_REPEATABLE;
    flags[83] = QLF_REPEATABLE;
    flags[84] = QLF_REPEATABLE;
    flags
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestReopenResult {
    Reopened,
    CannotOpenAgain,
    CannotOpenNow,
    InvalidQuest,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestEntry {
    pub done: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestLog {
    quests: Vec<QuestEntry>,
}

impl Default for QuestLog {
    fn default() -> Self {
        Self {
            quests: vec![QuestEntry::default(); MAX_QUESTS],
        }
    }
}

impl QuestLog {
    pub fn entries(&self) -> &[QuestEntry] {
        &self.quests
    }

    pub fn open(&mut self, quest: usize) {
        if let Some(entry) = self.quests.get_mut(quest) {
            entry.flags |= QF_OPEN;
        }
    }

    pub fn close(&mut self, quest: usize) {
        if let Some(entry) = self.quests.get_mut(quest) {
            entry.flags &= !QF_OPEN;
        }
    }

    pub fn mark_done(&mut self, quest: usize) {
        if let Some(entry) = self.quests.get_mut(quest) {
            entry.flags = (entry.flags | QF_DONE) & !QF_OPEN;
            entry.done = entry.done.saturating_add(1).min(0x3f);
        }
    }

    pub fn reopen(&mut self, quest: usize) {
        if let Some(entry) = self.quests.get_mut(quest) {
            entry.flags |= QF_OPEN;
            entry.flags &= !QF_DONE;
        }
    }

    pub fn try_reopen_legacy(&mut self, quest: usize) -> QuestReopenResult {
        let Some(entry) = self.quests.get_mut(quest) else {
            return QuestReopenResult::InvalidQuest;
        };
        if entry.done > 9 || (QUESTLOG_FLAGS[quest] & QLF_REPEATABLE) == 0 {
            return QuestReopenResult::CannotOpenAgain;
        }
        if (entry.flags & QF_DONE) == 0 {
            return QuestReopenResult::CannotOpenNow;
        }

        entry.flags = (entry.flags | QF_OPEN) & !QF_DONE;
        QuestReopenResult::Reopened
    }

    pub fn is_done(&self, quest: usize) -> bool {
        self.quests
            .get(quest)
            .is_some_and(|entry| (entry.flags & QF_DONE) != 0)
    }

    pub fn count(&self, quest: usize) -> u8 {
        self.quests.get(quest).map_or(0, |entry| entry.done)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quest_constants_match_c_header() {
        assert_eq!(MAX_QUESTS, 100);
        assert_eq!(QF_OPEN, 1);
        assert_eq!(QF_DONE, 2);
        assert_eq!(QLF_REPEATABLE, 1);
        assert_eq!(QLOG_JESSICA_KILL, 84);
    }

    #[test]
    fn quest_done_count_is_six_bit_like_c_bitfield() {
        let mut log = QuestLog::default();
        for _ in 0..70 {
            log.mark_done(QLOG_LYDIA);
        }
        assert_eq!(log.count(QLOG_LYDIA), 0x3f);
        assert!(log.is_done(QLOG_LYDIA));
    }

    #[test]
    fn entries_expose_fixed_legacy_quest_count() {
        let log = QuestLog::default();

        assert_eq!(log.entries().len(), MAX_QUESTS);
    }

    #[test]
    fn reopen_legacy_allows_done_repeatable_quests() {
        let mut log = QuestLog::default();
        log.mark_done(QLOG_LYDIA);

        assert_eq!(
            log.try_reopen_legacy(QLOG_LYDIA),
            QuestReopenResult::Reopened
        );
        let entry = log.entries()[QLOG_LYDIA];
        assert_eq!(entry.done, 1);
        assert_eq!(entry.flags, QF_OPEN);
    }

    #[test]
    fn reopen_legacy_rejects_non_repeatable_and_not_done_quests() {
        let mut log = QuestLog::default();
        assert_eq!(
            log.try_reopen_legacy(QLOG_NOOK),
            QuestReopenResult::CannotOpenAgain
        );
        assert_eq!(
            log.try_reopen_legacy(QLOG_GWENDY_FIRST_SKULL),
            QuestReopenResult::CannotOpenNow
        );
    }

    #[test]
    fn reopen_legacy_rejects_after_ten_completions_like_c() {
        let mut log = QuestLog::default();
        for _ in 0..10 {
            log.mark_done(QLOG_LYDIA);
        }

        assert_eq!(
            log.try_reopen_legacy(QLOG_LYDIA),
            QuestReopenResult::CannotOpenAgain
        );
    }
}
