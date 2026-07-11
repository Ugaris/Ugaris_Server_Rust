//! Area 31 (Warr Mines / Grimroot) NPCs, one file per NPC.

pub mod dwarfchief;
pub mod dwarfshaman;
pub mod dwarfsmith;
pub mod lostdwarf;

#[allow(unused_imports)]
pub use dwarfchief::*;
#[allow(unused_imports)]
pub use dwarfshaman::*;
#[allow(unused_imports)]
pub use dwarfsmith::*;
#[allow(unused_imports)]
pub use lostdwarf::*;

use crate::character_driver::TextQaEntry;

/// C `struct qa qa[]` from `src/area/31/warrmines.c:76-87` - the small-talk
/// table `analyse_text_driver`'s own local copy in this file feeds every
/// `warrmines.c` NPC driver that calls it, same "one shared file-local
/// table, many drivers" shape as `world::npc::area29::AREA29_QA`. Unlike
/// `AREA29_QA`, this table has no extra area-specific trade codes (4/5) -
/// it is the plain 12-entry base table.
pub const AREA31_QA: &[TextQaEntry] = &[
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
        words: &["restart"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["please", "repeat"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["please", "restart"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["reset", "me"],
        answer: None,
        answer_code: 3,
    },
];
