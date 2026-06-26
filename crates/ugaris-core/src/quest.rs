use serde::{Deserialize, Serialize};

pub const MAX_QUESTS: usize = 100;
pub const QF_OPEN: u8 = 1;
pub const QF_DONE: u8 = 2;

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
}
