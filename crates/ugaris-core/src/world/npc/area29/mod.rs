//! Area 29 NPCs, one file per NPC.

pub mod spiritbran;

#[allow(unused_imports)]
pub use spiritbran::*;

use crate::character_driver::TextQaEntry;

/// C `struct qa qa[]` from `src/area/29/brannington.c:86-99` - the
/// small-talk table `analyse_text_driver`'s own local copy in this file
/// feeds every `brannington.c` NPC driver that calls it (not just
/// `world::npc::area29::spiritbran`), same "one shared file-local table,
/// many drivers" shape as `world::npc::area26::AREA26_QA`/
/// `world::npc::area28::AREA28_QA`.
///
/// Unlike `AREA28_QA`, this table carries two extra area-29-only entries
/// (`4` "thousand gold", `5` "five thousand silver") consumed by
/// `broklin_driver`'s permanent gold<->silver trade service, not yet
/// ported.
pub const AREA29_QA: &[TextQaEntry] = &[
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
    TextQaEntry {
        words: &["thousand", "gold"],
        answer: None,
        answer_code: 4,
    },
    TextQaEntry {
        words: &["five", "thousand", "silver"],
        answer: None,
        answer_code: 5,
    },
];
