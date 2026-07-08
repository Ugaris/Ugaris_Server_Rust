//! Area 17 (Two-City/Exkordon) NPCs, one file per NPC.

pub mod alchemist;
pub mod two_skelly;

#[allow(unused_imports)]
pub use alchemist::*;
#[allow(unused_imports)]
pub use two_skelly::*;

use crate::character_driver::TextQaEntry;

/// C `struct qa qa[]` from `src/area/17/two.c:92-112` - the shared
/// small-talk/command table `analyse_text_driver` matches against for
/// every Two-City NPC in this file (`guard_driver`/`barkeeper`/`servant`/
/// `thiefguard`/`thiefmaster`/`sanwyn`/`skelly`/`alchemist`). Only the
/// entries `world::npc::area17::two_skelly`/`world::npc::area17::
/// alchemist` actually need are ported so far (the first 8 rows, through
/// `repeat`/answer_code 2); the remaining `guest`/`citizen`/`honor`/
/// `enemy`/`chat`/`bribe`/`threaten`/`pay bribe`/`pay`/`buy pass`/
/// `status`/`pay a fee`/`i am done` rows belong to NPCs not yet ported -
/// add them here (never duplicate the table) when that work happens, same
/// "one shared file-local table, many drivers" precedent as `world::npc::
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
];
