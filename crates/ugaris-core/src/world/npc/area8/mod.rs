//! Area 8 NPCs, one file per NPC.

pub mod fdemon_army;
pub mod fdemon_boss;
pub mod fdemon_demon;

#[allow(unused_imports)]
pub use fdemon_army::*;
#[allow(unused_imports)]
pub use fdemon_boss::*;
#[allow(unused_imports)]
pub use fdemon_demon::*;

use crate::character_driver::TextQaEntry;

/// C `src/area/8/fdemon.c::qa[]` (`:123-183`), the shared `analyse_text_
/// driver` small-talk table used by both `fdemon_boss`'s `NT_TEXT`
/// "repeat" detection and (once ported) `CDR_FDEMON_ARMY`'s soldier
/// small-talk. Only the first 15 rows (C's `needs_name: 0` rows, i.e.
/// matched regardless of whether the speaker also said the NPC's own
/// name) are ported here - `fdemon_boss` only ever inspects `answer_code
/// == 8` ("repeat"), plus `answer_code == 1` ("what's your name"/"who are
/// you") which reproduces `analyse_text_driver`'s own built-in "I'm %s."
/// fallback (`fdemon.c:285-289`, the `case 1: say(...); default: return
/// ...;` fallthrough - ported as an explicit caller-side branch, same
/// convention `character_driver::analyse_text_qa`'s own doc comment
/// establishes for every `Matched` code). The remaining ~35 rows (`needs_
/// name: 1` - "yes"/"no"/"thanks"/"you stink"/etc., only matched when a
/// soldier's own name is also spoken, C's `if (qa[q].needs_name && !name)
/// continue;` gate) are the recruited-soldier emote-reaction small talk
/// (`QA_YES`..`QA_COWARD`) and are meaningless without `CDR_FDEMON_ARMY`'s
/// soldier companions to address by name - deferred to that task, along
/// with the `needs_name` field itself (not yet a concept `character_
/// driver::TextQaEntry`/`analyse_text_qa` support).
pub const FDEMON_QA: &[TextQaEntry] = &[
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
        words: &["what's", "your", "name"],
        answer: None,
        answer_code: 1,
    },
    TextQaEntry {
        words: &["what", "is", "your", "name"],
        answer: None,
        answer_code: 1,
    },
    TextQaEntry {
        words: &["who", "are", "you"],
        answer: None,
        answer_code: 1,
    },
    TextQaEntry {
        words: &["follow"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["back"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["retreat"],
        answer: None,
        answer_code: 4,
    },
    TextQaEntry {
        words: &["front"],
        answer: None,
        answer_code: 5,
    },
    TextQaEntry {
        words: &["behind"],
        answer: None,
        answer_code: 6,
    },
    TextQaEntry {
        words: &["emote"],
        answer: None,
        answer_code: 7,
    },
    TextQaEntry {
        words: &["repeat"],
        answer: None,
        answer_code: 8,
    },
];
