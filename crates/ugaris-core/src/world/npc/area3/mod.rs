//! Area 3 NPCs, one file per NPC.

pub mod astro1;
pub mod astro2;
pub mod carlos;
pub mod clara;
pub mod kassim;
pub mod kelly;
pub mod seymour;
pub mod sir_jones;
pub mod thomas;

#[allow(unused_imports)]
pub use astro1::*;
#[allow(unused_imports)]
pub use astro2::*;
#[allow(unused_imports)]
pub use carlos::*;
#[allow(unused_imports)]
pub use clara::*;
#[allow(unused_imports)]
pub use kassim::*;
#[allow(unused_imports)]
pub use kelly::*;
#[allow(unused_imports)]
pub use seymour::*;
#[allow(unused_imports)]
pub use sir_jones::*;
#[allow(unused_imports)]
pub use thomas::*;

use crate::character_driver::TextQaEntry;

/// C `struct qa qa[]` from `src/area/3/area3.c:106-204` - the small-talk
/// table `analyse_text_driver`'s own local copy in this file feeds every
/// `area3.c` NPC driver that calls it (`world::thomas`, `world::
/// sir_jones`, `world::astro2`, `world::seymour`, `world::kelly`, `world::
/// carlos`, and eventually `kassim_driver`/`supermax_driver`), not just
/// one - same "one shared file-local table, many drivers" shape as
/// `world::npc::area1::gwendylon::GWENDYLON_QA`.
///
/// The first 13 entries (`area3.c:106-118`, canned greetings plus
/// `repeat`/`restart`/`aye`/`nay`) plus the `shortcut to caligar`(7) entry
/// (`area3.c:121`, `world::kelly`'s own `case 7` god-only fast-forward) and
/// the `explain`(9) entry (`area3.c:122`, `world::kassim`'s own `case 9`
/// service explanation) are ported so far. The remaining C entries
/// (`area3.c:119-204`: `list`(5)/`money`(6), plus the ~86-entry `raise`/
/// `lower` skill block keyed on `V_*` skill-id offsets) are needed only by
/// `supermax_driver` - add them when that driver is ported, following this
/// table's own doc precedent rather than guessing the `V_*` mapping ahead
/// of time. The `engrave: `(8) row (`area3.c:123`) is genuinely dead code
/// in C - `kassim_driver`'s `NT_TEXT` branch special-cases `strcasestr`
/// for `"engrave:"` *before* ever calling `analyse_text_driver`
/// (`area3.c:479-499`), so `analyse_text_qa` never sees that literal text
/// - not ported (see `world::kassim`'s own module doc comment).
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
    TextQaEntry {
        words: &["shortcut", "to", "caligar"],
        answer: None,
        answer_code: 7,
    },
    TextQaEntry {
        words: &["explain"],
        answer: None,
        answer_code: 9,
    },
];
