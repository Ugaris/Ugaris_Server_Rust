//! Area 17 (Two-City/Exkordon) NPCs, one file per NPC.

pub mod alchemist;
pub mod barkeeper;
pub mod guard;
mod guard_messages;
pub mod sanwyn;
pub mod servant;
pub mod thiefguard;
pub mod two_skelly;

#[allow(unused_imports)]
pub use alchemist::*;
#[allow(unused_imports)]
pub use barkeeper::*;
#[allow(unused_imports)]
pub use guard::*;
#[allow(unused_imports)]
pub use sanwyn::*;
#[allow(unused_imports)]
pub use servant::*;
#[allow(unused_imports)]
pub use thiefguard::*;
#[allow(unused_imports)]
pub use two_skelly::*;

use crate::character_driver::{TextQaEntry, NTID_TWOCITY, NT_NPC};
use crate::ids::CharacterId;
use crate::legacy::MAX_MAP;
use crate::world::World;

/// C `#define LS_CLEAN 0` (`two.c:271`): no fine owed, hasn't killed the
/// governor's double.
pub const LS_CLEAN: i32 = 0;
/// C `#define LS_FINE 1` (`two.c:272`): owes an accumulated fine
/// (`twocity_ppd::legal_fine`).
pub const LS_FINE: i32 = 1;
/// C `#define LS_DEAD 2` (`two.c:273`): has killed the governor's double.
pub const LS_DEAD: i32 = 2;

/// C `#define CS_ENEMY 0` (`two.c:275`).
pub const CS_ENEMY: i32 = 0;
/// C `#define CS_GUEST 1` (`two.c:276`): has bought a guest pass.
pub const CS_GUEST: i32 = 1;
/// C `#define CS_CITIZEN 2` (`two.c:277`).
pub const CS_CITIZEN: i32 = 2;
/// C `#define CS_HONOR 3` (`two.c:278`).
pub const CS_HONOR: i32 = 3;

/// C `struct qa qa[]` from `src/area/17/two.c:92-112` - the shared
/// small-talk/command table `analyse_text_driver` matches against for
/// every Two-City NPC in this file (`guard_driver`/`barkeeper`/`servant`/
/// `thiefguard`/`thiefmaster`/`sanwyn`/`skelly`/`alchemist`). Every row
/// except `i am done`/answer_code 16 (belonging to the still-unported
/// `thiefmaster`) is ported so far - `status`/answer_code 14 is ported
/// (row present below) even though it is genuinely dead code in C itself:
/// grepping every `switch (didsay)`/`switch (analyse_text_driver(...))`
/// block in `two.c` confirms no driver's `NT_TEXT` switch has a `case 14`
/// arm at all (not even `thiefmaster`, whose own switch only handles `2`/
/// `16`), so saying "status" to any Two-City NPC has always been a silent
/// no-op beyond `didsay`'s usual talkdir/current_victim bookkeeping - kept
/// in the table for parity, not silently dropped. Add the rest here
/// (never duplicate the table) when `thiefmaster` is ported, same "one
/// shared file-local table, many drivers" precedent as `world::npc::
/// area16::FOREST_QA`/`world::npc::area3::AREA3_QA`.
pub const TWOCITY_QA: &[TextQaEntry] = &[
    TextQaEntry {
        words: &["how", "are", "you"],
        answer: Some("I'm fine!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hello"],
        answer: Some("Hello, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hi"],
        answer: Some("Hi, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["greetings"],
        answer: Some("Greetings, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hail"],
        answer: Some("And hail to you, %s!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["what's", "up"],
        answer: Some("Everything that isn't nailed down."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["what", "is", "up"],
        answer: Some("Everything that isn't nailed down."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["repeat"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["buy", "pass"],
        answer: None,
        answer_code: 13,
    },
    TextQaEntry {
        words: &["status"],
        answer: None,
        answer_code: 14,
    },
    TextQaEntry {
        words: &["pay", "a", "fee"],
        answer: None,
        answer_code: 15,
    },
    TextQaEntry {
        words: &["pay"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["guest"],
        answer: None,
        answer_code: 5,
    },
    TextQaEntry {
        words: &["citizen"],
        answer: None,
        answer_code: 6,
    },
    TextQaEntry {
        words: &["honor"],
        answer: None,
        answer_code: 7,
    },
    TextQaEntry {
        words: &["enemy"],
        answer: None,
        answer_code: 12,
    },
    TextQaEntry {
        words: &["chat"],
        answer: None,
        answer_code: 8,
    },
    TextQaEntry {
        words: &["bribe"],
        answer: None,
        answer_code: 9,
    },
    TextQaEntry {
        words: &["threaten"],
        answer: None,
        answer_code: 10,
    },
    TextQaEntry {
        words: &["pay", "bribe"],
        answer: None,
        answer_code: 11,
    },
];

/// C `struct places places[]` (`two.c:245-257`): fixed bounding boxes
/// gating how strict a citizen's standing must be to enter, checked in
/// declaration order (`illegal_place` returns the *first* match, so more
/// specific boxes must come before the catch-all "whole city" box).
struct Place {
    fx: u16,
    fy: u16,
    tx: u16,
    ty: u16,
    level: i32,
}

const PLACES: &[Place] = &[
    // palace
    Place {
        fx: 1,
        fy: 3,
        tx: 15,
        ty: 15,
        level: 4,
    },
    Place {
        fx: 15,
        fy: 7,
        tx: 21,
        ty: 15,
        level: 4,
    },
    Place {
        fx: 1,
        fy: 1,
        tx: 59,
        ty: 37,
        level: 3,
    },
    // shop
    Place {
        fx: 56,
        fy: 57,
        tx: 63,
        ty: 63,
        level: 4,
    },
    // servant 5 house
    Place {
        fx: 73,
        fy: 34,
        tx: 79,
        ty: 46,
        level: 4,
    },
    // whole city
    Place {
        fx: 1,
        fy: 1,
        tx: 255,
        ty: 100,
        level: 1,
    },
];

/// C `illegal_place(x, y)` (`two.c:259-269`): the minimum `citizen_status`
/// required to be at `(x, y)` without triggering the city guard, or `0`
/// if the tile is outside every restricted box.
pub fn illegal_place(x: u16, y: u16) -> i32 {
    for place in PLACES {
        if place.fx <= x && place.fy <= y && place.tx >= x && place.ty >= y {
            return place.level;
        }
    }
    0
}

impl World {
    /// C `call_guard(cn, co)` (`two.c:219-237`): scans every live
    /// character for the nearest higher-level same-`group` character and
    /// pushes it an `NT_NPC`/`NTID_TWOCITY` alert. Shared by
    /// `guard_driver`/`servant_driver` (both ported, `two.c:1050-1053`/
    /// `:1188`/`:1194`/`:1213`/`:1300`) and `servant_dead`'s death hook
    /// (`ugaris-server::world_events::death_hooks::
    /// apply_two_servant_death_from_hurt_event`, hence `pub` rather than
    /// `pub(crate)`) - kept here, not in `guard.rs`/`servant.rs`, for
    /// that reason.
    ///
    /// Reproduces a real C quirk digit-for-digit: the alert's packed
    /// `dat3` coordinate mixes `caller.x` with `target.y` (`ch[cn].x +
    /// ch[co].y * MAXMAP`), not a matched `(x, y)` pair from either
    /// character alone. Not "fixed" - the receiving `guard_driver` reads
    /// it back with the same mismatched split (`msg->dat3 % MAXMAP`/`/
    /// MAXMAP`), so the two sides agree with each other even though
    /// neither matches a real map position.
    pub fn two_city_call_guard(&mut self, caller_id: CharacterId, target_id: CharacterId) {
        let Some(caller) = self.characters.get(&caller_id).cloned() else {
            return;
        };
        let Some(target) = self.characters.get(&target_id).cloned() else {
            return;
        };
        let level = caller.level.max(target.level);

        let mut best_id: Option<CharacterId> = None;
        let mut best_dist: i64 = 9999;
        for candidate in self.characters.values() {
            if candidate.id == caller_id
                || candidate.group != caller.group
                || candidate.level <= level
            {
                continue;
            }
            let dist = (i64::from(caller.x) - i64::from(candidate.x)).abs()
                + (i64::from(caller.y) - i64::from(candidate.y)).abs()
                + (i64::from(candidate.level) - i64::from(level)).abs();
            if dist < best_dist {
                best_dist = dist;
                best_id = Some(candidate.id);
            }
        }

        let Some(best_id) = best_id else {
            return;
        };
        if let Some(best) = self.characters.get_mut(&best_id) {
            best.push_driver_message(
                NT_NPC,
                NTID_TWOCITY,
                target.level as i32,
                i32::from(caller.x) + i32::from(target.y) * MAX_MAP as i32,
            );
        }
    }
}
