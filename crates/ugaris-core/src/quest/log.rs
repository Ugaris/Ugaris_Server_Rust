use super::*;

/// Result of `QuestLog::complete_legacy`, mirroring the values C's
/// `questlog_done` (`src/system/questlog.c:267-305`) uses to call
/// `give_exp`/`dlog`/`sendquestlog` - all of which stay in the caller
/// (`World`/`PlayerRuntime` live in different structures, so this leaf
/// module cannot call `World::give_exp` directly).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestCompletion {
    /// `quest[qnr].done` *after* the increment (C's `cnt + 1` in the dlog
    /// text, and the function's `int` return value).
    pub times_done: u8,
    /// The exp value C passes to `give_exp(cn, val)` (already scaled by
    /// prior completions and tapered by level).
    pub granted_exp: i64,
    /// C's nominal `questlog[qnr].exp` - the `dlog` line is only emitted
    /// when this is `> 0`.
    pub nominal_exp: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestReopenResult {
    /// C `questlog_reopen`'s `if (ret) { quest[qnr].flags = QF_OPEN; ...
    /// }` branch (`src/system/questlog.c:818-824`): the per-quest
    /// `questlog_reopen_qN` helper ran and left `ret` truthy.
    Reopened,
    /// The per-quest switch was reached (all generic preconditions
    /// passed) but its helper reported "cannot re-open more than one
    /// quest from a series" (e.g. `questlog_reopen_q1`,
    /// `src/system/questlog.c:359-363`) - C still calls `sendquestlog`
    /// afterwards, it just never sets `QF_OPEN`.
    SeriesConflict,
    /// The per-quest switch was reached but its `case` forces `ret = 0`
    /// with no helper call (either genuinely unimplemented, like case 6,
    /// or the helper call is dead/commented-out C, like case 18/19) - a
    /// silent no-op in C: no log message, `sendquestlog` still fires,
    /// the quest stays `QF_DONE`.
    NoEffect,
    /// C's generic `quest[qnr].done > 9` or "table flags are zero"
    /// precondition failure (`src/system/questlog.c:624-631`) -
    /// `log_char` shows "You cannot open this quest again." and
    /// `questlog_reopen` returns *before* `sendquestlog`.
    CannotOpenAgain,
    /// C's generic `!(quest[qnr].flags & QF_DONE)` precondition failure
    /// (`src/system/questlog.c:632-635`) - "You cannot open this quest at
    /// the moment.", also returns before `sendquestlog`.
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

    /// True when every quest slot is still at its untouched default
    /// (`done == 0 && flags == 0`) - i.e. this player's quest log has
    /// never recorded any progress. Rust keeps `QuestLog` as a plain,
    /// always-present field rather than C's lazily-allocated `del_data`
    /// block, so this is the equivalent of "the `DRD_QUESTLOG_PPD` block
    /// was never `set_data`'d" for callers (`/clearppd questlog`,
    /// `command.c:4258-4260`/`4275-4287`) that need to distinguish
    /// "there was nothing to clear" from "cleared".
    pub fn is_empty(&self) -> bool {
        self.quests
            .iter()
            .all(|entry| *entry == QuestEntry::default())
    }

    /// C `questlog_open(cn, qnr)` (`src/system/questlog.c:204-219`): sets
    /// `flags` to exactly `QF_OPEN`, discarding any prior `QF_DONE` bit
    /// (C assigns, it doesn't OR). The caller is responsible for the C
    /// side effect of resending the quest log packet.
    pub fn open(&mut self, quest: usize) {
        if let Some(entry) = self.quests.get_mut(quest) {
            entry.flags = QF_OPEN;
        }
    }

    /// C `questlog_close(cn, qnr)` (`src/system/questlog.c:221-238`): only
    /// transitions `QF_OPEN` -> `QF_DONE` when `flags` is *exactly*
    /// `QF_OPEN` (C's `if (quest[qnr].flags == QF_OPEN)`); any other state
    /// (closed, already done) is left untouched.
    pub fn close(&mut self, quest: usize) {
        if let Some(entry) = self.quests.get_mut(quest) {
            if entry.flags == QF_OPEN {
                entry.flags = QF_DONE;
            }
        }
    }

    /// C `questlog_done(cn, qnr)`'s bookkeeping half
    /// (`src/system/questlog.c:267-305`, minus the exp math and side
    /// effects - see `complete_legacy` for the full port). Kept as a
    /// simple flag/counter helper for callers that don't need the exp
    /// reward (e.g. test fixtures).
    pub fn mark_done(&mut self, quest: usize) {
        if let Some(entry) = self.quests.get_mut(quest) {
            entry.flags = (entry.flags | QF_DONE) & !QF_OPEN;
            entry.done = entry.done.saturating_add(1).min(0x3f);
        }
    }

    /// C `questlog_done(cn, qnr)` (`src/system/questlog.c:267-305`): full
    /// port including the exp reward computation. Returns `None` for an
    /// out-of-range quest number or one with no metadata row (C would read
    /// past the end of the 85-entry `questlog[]` table for indices
    /// `85..MAX_QUESTS`, which nothing in the ported tree ever does).
    ///
    /// The caller must still perform C's `give_exp(cn, val)` (using
    /// `QuestCompletion::granted_exp`), the `dlog` line (only when
    /// `nominal_exp > 0`), and `sendquestlog` resend - this leaf module
    /// has no access to `World`/`PlayerRuntime`.
    pub fn complete_legacy(
        &mut self,
        quest: usize,
        level: u32,
        level_value: u32,
    ) -> Option<QuestCompletion> {
        let meta = quest_meta(quest)?;
        let entry = self.quests.get_mut(quest)?;

        // C: `cnt = quest[qnr].done++;` (post-increment: `cnt` is the
        // count *before* this completion).
        let prior_completions = entry.done;
        entry.done = entry.done.saturating_add(1).min(0x3f);
        entry.flags = QF_DONE;

        let scaled = scale_exp(prior_completions, meta.exp);
        let granted_exp = taper_exp_by_level(level, level_value, scaled);

        Some(QuestCompletion {
            times_done: entry.done,
            granted_exp,
            nominal_exp: meta.exp,
        })
    }

    pub fn reopen(&mut self, quest: usize) {
        if let Some(entry) = self.quests.get_mut(quest) {
            entry.flags |= QF_OPEN;
            entry.flags &= !QF_DONE;
        }
    }

    /// C `questlog_reopen`'s generic preconditions
    /// (`src/system/questlog.c:617-635`), shared by `try_reopen_legacy`
    /// and `PlayerRuntime::reopen_quest_legacy`'s per-quest dispatch.
    /// Returns `Err` with the matching reject result, or `Ok(())` once
    /// every generic check has passed (mirrors reaching the C `switch`).
    pub(crate) fn reopen_precheck(&self, quest: usize) -> Result<(), QuestReopenResult> {
        let Some(entry) = self.quests.get(quest) else {
            return Err(QuestReopenResult::InvalidQuest);
        };
        if entry.done > 9 {
            return Err(QuestReopenResult::CannotOpenAgain);
        }
        // C: `if ((!questlog[qnr].flags & QLF_REPEATABLE))` - `!` binds
        // tighter than `&` in C, so this is actually `(!flags) &
        // QLF_REPEATABLE`, which is nonzero only when `flags == 0`. This
        // is a genuine operator-precedence bug: it really tests "the
        // table row has *no* flags at all" rather than "is missing the
        // REPEATABLE bit" specifically, so `QLF_XREPEAT`-only rows (25-28)
        // pass this check too, despite lacking `QLF_REPEATABLE`.
        if QUESTLOG_FLAGS[quest] == 0 {
            return Err(QuestReopenResult::CannotOpenAgain);
        }
        if (entry.flags & QF_DONE) == 0 {
            return Err(QuestReopenResult::CannotOpenNow);
        }
        Ok(())
    }

    pub fn try_reopen_legacy(&mut self, quest: usize) -> QuestReopenResult {
        if let Err(result) = self.reopen_precheck(quest) {
            return result;
        }

        let entry = self
            .quests
            .get_mut(quest)
            .expect("reopen_precheck already validated the index");
        entry.flags = (entry.flags | QF_OPEN) & !QF_DONE;
        QuestReopenResult::Reopened
    }

    /// C `quest[qnr].flags & QF_OPEN` read, used by the series-exclusivity
    /// checks in `questlog_reopen_qN` (e.g. `questlog_reopen_q1`,
    /// `src/system/questlog.c:359-363`).
    pub fn is_open(&self, quest: usize) -> bool {
        self.quests
            .get(quest)
            .is_some_and(|entry| (entry.flags & QF_OPEN) != 0)
    }

    pub fn is_done(&self, quest: usize) -> bool {
        self.quests
            .get(quest)
            .is_some_and(|entry| (entry.flags & QF_DONE) != 0)
    }

    pub fn count(&self, quest: usize) -> u8 {
        self.quests.get(quest).map_or(0, |entry| entry.done)
    }

    /// Raw mutable access to a quest entry, for the `questlog_init_*`
    /// ports below which manipulate `quest[qnr].done`/`.flags` directly,
    /// exactly like the C `struct quest *quest` array they read
    /// (`src/system/questlog.c:828-1607`).
    fn entry_mut(&mut self, quest: usize) -> Option<&mut QuestEntry> {
        self.quests.get_mut(quest)
    }

    /// C `questlog_init`'s (`src/system/questlog.c:1610-1626`)
    /// already-initialized sentinel: `if (quest[MAXQUEST - 1].done == 55)
    /// return;`.
    pub fn is_init_complete(&self) -> bool {
        self.quests.last().is_some_and(|entry| entry.done == 55)
    }

    /// C `questlog_init`'s final `quest[MAXQUEST - 1].done = 55;` marker,
    /// set after all five `questlog_init_*` sub-functions have run.
    pub fn mark_init_complete(&mut self) {
        if let Some(entry) = self.quests.last_mut() {
            entry.done = 55;
        }
    }

    /// C `/questfix`'s `quest[MAXQUEST - 1].done = 0;`
    /// (`src/system/command.c:3245`): clears the init-complete sentinel
    /// without touching any other entry, so the next
    /// [`PlayerRuntime::init_questlog`] call (typically the character's
    /// next login) fully re-derives every quest slot from its area PPD
    /// state again.
    pub fn clear_init_complete(&mut self) {
        if let Some(entry) = self.quests.last_mut() {
            entry.done = 0;
        }
    }

    /// Raw `done`/`flags` setter used by the PPD codec
    /// (`PlayerRuntime::decode_legacy_questlog_ppd`) to unpack the
    /// persisted `struct quest { unsigned char done:6; flags:2; }`
    /// bitfield byte per quest (`src/system/questlog.h:36-39`).
    pub fn set_raw(&mut self, quest: usize, done: u8, flags: u8) {
        if let Some(entry) = self.quests.get_mut(quest) {
            entry.done = done;
            entry.flags = flags;
        }
    }
}

/// C's repeated `if (!quest[qnr].done) { quest[qnr].done = 1; }
/// quest[qnr].flags = QF_DONE;` idiom used throughout `questlog_init_*`
/// (e.g. `src/system/questlog.c:836-839`): marks a quest done, seeding
/// `done` to `1` only the first time (never incrementing an existing
/// completion count).
pub(super) fn mark_init_done(quests: &mut QuestLog, quest: usize) {
    if let Some(entry) = quests.entry_mut(quest) {
        if entry.done == 0 {
            entry.done = 1;
        }
        entry.flags = QF_DONE;
    }
}

pub(super) fn set_flags(quests: &mut QuestLog, quest: usize, flags: u8) {
    if let Some(entry) = quests.entry_mut(quest) {
        entry.flags = flags;
    }
}
