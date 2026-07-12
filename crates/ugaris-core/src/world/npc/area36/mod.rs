//! Area 36 (Caligar) NPCs, one file per NPC.

pub mod caligar_guard;
pub mod caligar_guard2;

#[allow(unused_imports)]
pub use caligar_guard::*;
#[allow(unused_imports)]
pub use caligar_guard2::*;

use crate::character_driver::TextQaEntry;

/// C `struct qa qa[]` from `src/area/36/caligar.c:92-105` - the small-talk
/// table `analyse_text_driver`'s own local copy in this file feeds every
/// `caligar.c` NPC driver that calls it, same "one shared file-local
/// table, many drivers" shape as `world::npc::area29::AREA29_QA`.
///
/// Unlike `AREA29_QA`, this table has no "reset me" entry at all - codes
/// `3`/`4`/`5` are instead `smith_driver`'s own price-negotiation replies
/// ("yes okay" / "no not today" / "pay 10000g", `caligar.c:103-105`), used
/// by no other driver in this file.
pub const AREA36_QA: &[TextQaEntry] = &[
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
        words: &["yes", "okay"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["no", "not", "today"],
        answer: None,
        answer_code: 4,
    },
    TextQaEntry {
        words: &["pay", "10000g"],
        answer: None,
        answer_code: 5,
    },
];
