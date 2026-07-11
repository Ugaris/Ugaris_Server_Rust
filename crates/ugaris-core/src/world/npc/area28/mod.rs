//! Area 28 NPCs, one file per NPC.

pub mod aristocrat;
pub mod yoatin;

#[allow(unused_imports)]
pub use aristocrat::*;
#[allow(unused_imports)]
pub use yoatin::*;

use crate::character_driver::TextQaEntry;

/// C `struct qa qa[]` from `src/area/28/brannington_forest.c:81-92` - the
/// small-talk table `analyse_text_driver`'s own local copy in this file
/// feeds every `brannington_forest.c` NPC driver that calls it
/// (`world::npc::area28::aristocrat` and `world::npc::area28::yoatin`), not
/// just one - same "one shared file-local table, many drivers" shape as
/// `world::npc::area26::AREA26_QA`.
///
/// Byte-for-byte identical to `AREA26_QA`: `2` is "repeat"/"restart" (reset
/// the dialogue state back to its greeting), `3` is this file's own "reset
/// me" god-only state wipe (which, unlike `AREA26_QA`'s smugglecom/rouven
/// consumers, `aristocrat_driver`/`yoatin_driver` both pair with a visible
/// `say(cn, "reset done")` line - see their own doc comments).
pub const AREA28_QA: &[TextQaEntry] = &[
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
