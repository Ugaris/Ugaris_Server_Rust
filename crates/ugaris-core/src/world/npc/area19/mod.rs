//! Area 19 (Nomad Plains): `src/area/19/nomad.c`.

pub mod madhermit;
pub mod nomad;
pub mod nomad_bet;
pub mod nomad_dialogue;
pub mod nomad_give;
pub mod nomad_text;

#[allow(unused_imports)]
pub use madhermit::*;
#[allow(unused_imports)]
pub use nomad::*;
#[allow(unused_imports)]
pub use nomad_bet::*;
#[allow(unused_imports)]
pub use nomad_dialogue::*;
#[allow(unused_imports)]
pub use nomad_give::*;
#[allow(unused_imports)]
pub use nomad_text::*;

use crate::character_driver::TextQaEntry;

/// C `#define TM_TRIBE1 1` (`src/common/nomad_ppd.h:6`): the Vana Kiru
/// tribe (Kalanur's tribe, the only one any driver in this file actually
/// grants or checks).
pub const TM_TRIBE1: i32 = 1;
/// C `#define TM_TRIBE2 2` (`src/common/nomad_ppd.h:7`): never
/// granted/checked by any driver in `nomad.c`.
#[allow(dead_code)]
pub const TM_TRIBE2: i32 = 2;
/// C `#define TM_TRIBE3 4` (`src/common/nomad_ppd.h:8`): never
/// granted/checked by any driver in `nomad.c`.
#[allow(dead_code)]
pub const TM_TRIBE3: i32 = 4;

/// C `struct qa qa[]` (`nomad.c:87-104`), shared by every `nomad_N`
/// persona via `analyse_text_driver` (`nomad.c:113-211`). Rows with a
/// `NULL` answer report `answer_code` back to the caller: `2` ("repeat")
/// resets the speaking persona's own `nomad_state[nr]`; `3`/`4`/`5`
/// ("cheap"/"mediocre"/"good dice") and `6` ("golden statue") are
/// interpreted by `nomad_2_text`/`nomad_6_text` respectively (only the
/// dice-seller/statue-seller personas act on them - every other persona
/// silently ignores a match, matching C's own `switch (dat->nr)` with no
/// default case).
pub const NOMAD_QA: &[TextQaEntry] = &[
    TextQaEntry {
        words: &["how", "are", "you"],
        answer: Some("I'm fine!"),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hello"],
        answer: Some("Sul vana ley, %s."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hi"],
        answer: Some("Sul vana ley, %s."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["greetings"],
        answer: Some("Sul vana ley, %s."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["hail"],
        answer: Some("Sul vana ley, %s."),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["whats", "up"],
        answer: Some("Everything that isn't nailed down."),
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
        words: &["llakal", "sla"],
        answer: Some(
            "Llakal Sla is a game played with three dice. The opponents agree on a bet, and the \
             one throwing the higher number wins. If thou wishest to play, say: 'bet <amount>', \
             where amount is in ounces of salt.",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["repeat"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["cheap", "dice"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["mediocre", "dice"],
        answer: None,
        answer_code: 4,
    },
    TextQaEntry {
        words: &["good", "dice"],
        answer: None,
        answer_code: 5,
    },
    TextQaEntry {
        words: &["golden", "statue"],
        answer: None,
        answer_code: 6,
    },
];
