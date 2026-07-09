//! `CDR_LQPARSER`'s `#questend`/`#xinfo` pair (`lq.c:1314-1345`,
//! `2638-2642`, `2695-2701`) - split out of `world::lq_admin` (already
//! near the ~2,000-line hard cap; see that module's doc comment for the
//! rest of the `CDR_LQPARSER` admin command table) because both commands
//! need to read/clear *`PlayerRuntime::lq_marks`*, which `World` has no
//! access to (only `ugaris-server`'s session map does) - same split
//! rationale as [`super::LqNspawnDispatch`]/[`super::LqThrallDispatch`],
//! except neither command here ever fails argument validation (C's
//! `cmd_questend`/the inline `xinfo` block never call `get_str`/
//! `check_anything` on `ptr` at all), so the pure-`World` half only needs
//! to answer "does this command/permission/area match", not produce a
//! `Rejected` case.
//!
//! - `#questend` (`cmd_questend`, `lq.c:1314-1345`): for every *online*
//!   `CF_PLAYER` character, sums `lq_data.reward[n]` across every set
//!   mark (clearing them all), then grants `level_value(level) /
//!   (level/10+1) / 100.0 * min(100, sum)` exp if `sum` was nonzero.
//!   [`World::apply_lq_questend_reward`] is the pure-`World` half of the
//!   per-player body (the exp math plus `give_exp`/feedback text, given
//!   an already-summed `sum` the caller computed from that player's own
//!   `PlayerRuntime::lq_marks`); [`World::lq_admin_wants_questend`] is
//!   the top-level gate `ugaris-server` checks before iterating
//!   `ServerRuntime::players` itself.
//! - `#xinfo` (inlined in `special_driver`, `lq.c:2695-2701`): lists every
//!   mark the *caller's own* `PlayerRuntime::lq_marks` has set right now.
//!   [`World::report_lq_xinfo`] formats the reply given the caller's own
//!   marks array; [`World::lq_admin_wants_xinfo`] is the matching gate.

use super::lq_admin::{cmd_word_matches, ArgReader};
use super::*;

impl World {
    /// Shared gate for both commands in this module: area 20/35, `#`/`/`
    /// prefix, `CF_GOD`/`CF_LQMASTER` flag - the same checks
    /// `World::apply_lq_admin_command` runs for its own table
    /// (`lq.c:2514-2521`).
    fn lq_quest_admin_word(
        &self,
        character_id: CharacterId,
        area_id: u16,
        command: &str,
    ) -> Option<String> {
        if area_id != 20 && area_id != 35 {
            return None;
        }
        let trimmed = command.trim_start();
        let rest = trimmed
            .strip_prefix('#')
            .or_else(|| trimmed.strip_prefix('/'))?;
        let mut reader = ArgReader::new(rest);
        let word = reader.take_str()?;
        let flags = self.characters.get(&character_id)?.flags;
        if !flags.intersects(CharacterFlags::GOD | CharacterFlags::LQMASTER) {
            return None;
        }
        Some(word)
    }

    /// C `cmdcmp(ptr, "questend", 8)` gate (`lq.c:2638`) - `true` means
    /// the caller should sum/clear every online player's
    /// `PlayerRuntime::lq_marks` and call [`Self::apply_lq_questend_reward`]
    /// per player, then queue the "Rewarded N players." summary itself
    /// via [`Self::queue_system_text`].
    pub fn lq_admin_wants_questend(
        &self,
        character_id: CharacterId,
        area_id: u16,
        command: &str,
    ) -> bool {
        self.lq_quest_admin_word(character_id, area_id, command)
            .is_some_and(|word| cmd_word_matches(&word, "questend", 8))
    }

    /// C `cmd_questend`'s per-player body (`lq.c:1325-1338`), given `sum`
    /// = the caller's own sum of `lq_data.reward[n]` across every mark
    /// that was set on `target_id`'s `PlayerRuntime::lq_marks` (marks
    /// already cleared by the caller, matching C's `pdat->mark[n] = 0`
    /// inside the same summing loop). Returns whether a reward was
    /// granted (C's implicit `if (sum)` gate) so the caller can tally
    /// `cnt` for the final "Rewarded N players." message.
    pub fn apply_lq_questend_reward(&mut self, target_id: CharacterId, sum: i32) -> bool {
        if sum == 0 {
            return false;
        }
        let Some(character) = self.characters.get(&target_id) else {
            return false;
        };
        let level = character.level;
        let base = level_value(level) / (level / 10 + 1);
        let capped_sum = f64::from(sum.min(100));
        let val = (f64::from(base) / 100.0 * capped_sum) as i64;
        let area_id = u32::from(self.area_id);
        self.give_exp(target_id, val, area_id);
        self.queue_system_text(
            target_id,
            "You have been rewarded for your participation in this quest.",
        );
        true
    }

    /// C `cmdcmp(ptr, "xinfo", 2)` gate (`lq.c:2695`) - `true` means the
    /// caller should call [`Self::report_lq_xinfo`] with the caller's own
    /// `PlayerRuntime::lq_marks`.
    pub fn lq_admin_wants_xinfo(
        &self,
        character_id: CharacterId,
        area_id: u16,
        command: &str,
    ) -> bool {
        self.lq_quest_admin_word(character_id, area_id, command)
            .is_some_and(|word| cmd_word_matches(&word, "xinfo", 2))
    }

    /// C `xinfo`'s inline loop (`lq.c:2696-2700`): one "I have mark N"
    /// line per set mark, `1..MAXLQMARK` (index `0` is never used, same
    /// convention as everywhere else in this module).
    pub fn report_lq_xinfo(&mut self, character_id: CharacterId, marks: &[bool; MAXLQMARK]) {
        for (n, &set) in marks.iter().enumerate().skip(1) {
            if set {
                self.queue_system_text(character_id, format!("I have mark {n}"));
            }
        }
    }
}
