//! Area 26 NPCs, one file per NPC.

pub mod smugglecom;

#[allow(unused_imports)]
pub use smugglecom::*;

use crate::character_driver::TextQaEntry;

/// C `struct qa qa[]` from `src/area/26/staffer.c:90-101` - the small-talk
/// table `analyse_text_driver`'s own local copy in this file feeds every
/// `staffer.c` NPC driver that calls it (`world::npc::area26::smugglecom`,
/// and eventually `rouven_driver` once ported), not just one - same "one
/// shared file-local table, many drivers" shape as `world::npc::area3::
/// AREA3_QA`.
///
/// Unlike `AREA3_QA`, this file's own `answer_code`s diverge from area 3's:
/// `2` is still "repeat"/"restart" (reset the dialogue state back to its
/// greeting range), but `3` is `staffer.c`'s own "reset me" god-only bits/
/// state wipe (area 3's `qa[]` never defines a `3`; it uses `3`/`4` for
/// "aye"/"nay" instead - a different table entirely, not shared with this
/// one).
pub const AREA26_QA: &[TextQaEntry] = &[
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
