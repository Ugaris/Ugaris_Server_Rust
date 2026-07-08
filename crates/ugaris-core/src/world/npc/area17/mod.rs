//! Area 17 (Two-City/Exkordon) NPCs, one file per NPC.

pub mod alchemist;
pub mod barkeeper;
pub mod sanwyn;
pub mod two_skelly;

#[allow(unused_imports)]
pub use alchemist::*;
#[allow(unused_imports)]
pub use barkeeper::*;
#[allow(unused_imports)]
pub use sanwyn::*;
#[allow(unused_imports)]
pub use two_skelly::*;

use crate::character_driver::TextQaEntry;

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
/// `thiefguard`/`thiefmaster`/`sanwyn`/`skelly`/`alchemist`). The first 8
/// rows (through `repeat`/answer_code 2) plus `buy pass`/answer_code 13
/// (`world::npc::area17::barkeeper`) are ported so far; the remaining
/// `guest`/`citizen`/`honor`/`enemy`/`chat`/`bribe`/`threaten`/
/// `pay bribe`/`pay`/`status`/`pay a fee`/`i am done` rows belong to NPCs
/// not yet ported - add them here (never duplicate the table) when that
/// work happens, same "one shared file-local table, many drivers"
/// precedent as `world::npc::area16::FOREST_QA`/`world::npc::area3::
/// AREA3_QA`.
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
];
