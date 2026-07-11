//! Area 25 (the Warped World): `src/area/25/warped.c`.
//!
//! Item drivers (`IDR_WARPTELEPORT`/`IDR_WARPBONUS`/`IDR_WARPKEYSPAWN`/
//! `IDR_WARPKEYDOOR`/`IDR_WARPTRIALDOOR`) were already ported (see
//! `crates/ugaris-core/src/item_driver/area25_warped.rs`). This module adds
//! the two character drivers: `warpmaster` (`CDR_WARPMASTER`, the key-for-
//! stone trading NPC) and `warpfighter` (`CDR_WARPFIGHTER`, the hired
//! trial-room opponent `warptrialdoor_driver` spawns).

pub mod warpfighter;
pub mod warpmaster;

#[allow(unused_imports)]
pub use warpfighter::*;
#[allow(unused_imports)]
pub use warpmaster::*;

use crate::character_driver::TextQaEntry;

/// C `struct qa qa[]` (`warped.c:90-127`), shared by `warpmaster`'s
/// `analyse_text_driver` call (`warped.c:1044`). The final row (`"reset"`)
/// has a `NULL` answer and reports `answer_code: 2` back to the caller,
/// which resets the speaking player's `warped_ppd` progress
/// (`warpmaster`'s own `NT_TEXT` branch, `warped.c:1045-1052`).
///
/// The three long entries embed the legacy `COL_LIGHT_BLUE`/`COL_RESET`
/// byte markers verbatim (`\u{E0C4}`/`\u{E0C0}`, matching
/// `crate::text::COL_STR_LIGHT_BLUE`/`COL_STR_RESET`) per the "Area-text
/// color markers" P0.5 task.
pub const AREA25_QA: &[TextQaEntry] = &[
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
        words: &["warped", "world"],
        answer: Some(
            "This world has been created by Rodney, the Mighty Mage. Well, actually, Ishtar \
             created it while he was trying out designs for his Labyrinth, but Rodney added the \
             final touches. He tried to create a Labyrinth, just like Ishtar. Anyway. Do you \
             want to buy some \u{E0C4}keys\u{E0C0} and \u{E0C4}explore\u{E0C0} it?",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["keys"],
        answer: Some(
            "You need keys to open the various doors here. Each key will only work once, so \
             you'll need plenty of them. I'll trade one key for an earth \u{E0C4}stone\u{E0C0}, \
             two keys for a fire stone, three keys for an ice stone and four keys for a hell \
             stone. Just hand me the stones if you want to trade.",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["stone"],
        answer: Some(
            "I need the stones to power the \u{E0C4}Warped World\u{E0C0}. Rodney created a \
             device that will draw power from them.",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["explore"],
        answer: Some(
            "It might be worth the trouble. Adventurers report that \u{E0C4}dangers\u{E0C0} and \
             rewards are to be found inside. Oh, and one word of warning: Don't venture into the \
             blue area before you're level 70.",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["dangers"],
        answer: Some(
            "It is said that enemies hide behind red doors. I've also heard that people can get \
             stuck, with no way to progress. Should that happen to you, ask me to \
             \u{E0C4}reset\u{E0C0} your current points. You will not lose the level reached.",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["reset"],
        answer: None,
        answer_code: 2,
    },
];
