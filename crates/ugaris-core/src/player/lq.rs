//! Area 20 (Live Quest) per-player state: C `struct lq_plr_data::mark[]`
//! (`src/area/20/lq.c:186-192`, `DRD_LQ_PLR_DATA`).

use crate::world::MAXLQMARK;

use super::PlayerRuntime;

impl PlayerRuntime {
    /// C `pdat->mark[markID]` read. `mark_id == 0` (never set by any C
    /// code path - `hurt_markID`/`kill_markID` are only ever compared
    /// with `> 0`) always reads `false`.
    pub fn lq_mark(&self, mark_id: u32) -> bool {
        let index = mark_id as usize;
        (1..MAXLQMARK).contains(&index) && self.lq_marks[index]
    }

    /// C `pdat->mark[markID] = 1`. Out-of-range `mark_id` (`0` or `>=
    /// MAXLQMARK`) is silently ignored, matching every C call site's own
    /// `hurt_markID > 0 && hurt_markID < MAXLQMARK` guard.
    pub fn set_lq_mark(&mut self, mark_id: u32) {
        let index = mark_id as usize;
        if (1..MAXLQMARK).contains(&index) {
            self.lq_marks[index] = true;
        }
    }
}
