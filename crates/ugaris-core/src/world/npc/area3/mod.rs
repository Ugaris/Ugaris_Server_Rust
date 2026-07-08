//! Area 3 NPCs, one file per NPC.

pub mod astro1;
pub mod clara;
pub mod sir_jones;
pub mod thomas;

#[allow(unused_imports)]
pub use astro1::*;
#[allow(unused_imports)]
pub use clara::*;
#[allow(unused_imports)]
pub use sir_jones::*;
#[allow(unused_imports)]
pub use thomas::*;

use crate::character_driver::TextQaEntry;

/// C `struct qa qa[]` from `src/area/3/area3.c:106-204` - the small-talk
/// table `analyse_text_driver`'s own local copy in this file feeds every
/// `area3.c` NPC driver that calls it (`world::thomas`, `world::
/// sir_jones`, and eventually `kassim_driver`/`supermax_driver`/
/// `carlos_driver`/`astro2_driver`/`seymour_driver`/`kelly_driver`), not
/// just one - same "one shared file-local table, many drivers" shape as
/// `world::npc::area1::gwendylon::GWENDYLON_QA`.
///
/// Only the first 13 entries (`area3.c:106-118`, canned greetings plus
/// `repeat`/`restart`/`aye`/`nay`) are ported so far - the only ones
/// `world::thomas`/`world::sir_jones` interpret. The remaining C entries
/// (`area3.c:119-204`: `list`(5)/`money`(6)/`shortcut to caligar`(7)/
/// `explain`(9)/`engrave: `(8), plus the ~86-entry `raise`/`lower` skill
/// block keyed on `V_*` skill-id offsets) are needed only by
/// `kassim_driver`/`supermax_driver` - add them when those drivers are
/// ported, following this table's own doc precedent rather than
/// guessing the `V_*` mapping ahead of time.
pub const AREA3_QA: &[TextQaEntry] = &[
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
        words: &["aye"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["nay"],
        answer: None,
        answer_code: 4,
    },
];
