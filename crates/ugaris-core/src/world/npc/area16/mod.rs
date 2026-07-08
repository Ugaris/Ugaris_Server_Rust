//! Area 16 (the forest) NPCs, one file per NPC.

pub mod hermit;
pub mod imp;
pub mod william;

#[allow(unused_imports)]
pub use hermit::*;
#[allow(unused_imports)]
pub use imp::*;
#[allow(unused_imports)]
pub use william::*;

use crate::character_driver::TextQaEntry;

/// C `struct qa qa[]` from `src/area/16/forest.c:89-97` - the small-talk
/// table `analyse_text_driver`'s own local copy in this file feeds
/// `world::npc::area16::william`/`world::npc::area16::hermit` (`imp_
/// driver` never calls `analyse_text_driver` at all - it has no `NT_TEXT`
/// handling). A smaller, *distinct* array from `world::npc::area3::
/// AREA3_QA`/`world::npc::area3::clara::CLARA_QA`: every `area/*.c` file
/// defines its own file-local `qa[]`/`analyse_text_driver` copy - same
/// "one shared file-local table, many drivers" shape as those two.
pub const FOREST_QA: &[TextQaEntry] = &[
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
        words: &["imp"],
        answer: Some(
            "A nice little guy. He's got a peculiar sense of humor, but he's very helpful.",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["repeat"],
        answer: None,
        answer_code: 2,
    },
];
