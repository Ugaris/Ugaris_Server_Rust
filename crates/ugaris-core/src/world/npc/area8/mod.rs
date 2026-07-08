//! Area 8 NPCs, one file per NPC.

pub mod fdemon_army;
pub mod fdemon_army_combat;
pub mod fdemon_army_emote;
pub mod fdemon_army_movement;
pub mod fdemon_boss;
pub mod fdemon_demon;

#[allow(unused_imports)]
pub use fdemon_army::*;
#[allow(unused_imports)]
pub use fdemon_army_emote::*;
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

/// C `#define QA_YES 20` .. `#define QA_COWARD 49` (`fdemon.c:92-121`):
/// answer codes for [`FDEMON_ARMY_EMOTE_QA`], interpreted by
/// [`crate::world::World::fdemon_army_got_emote`] (C `got_emote`).
pub const QA_YES: i32 = 20;
pub const QA_NO: i32 = 21;
pub const QA_NICEDAY: i32 = 22;
pub const QA_THANKS: i32 = 23;
pub const QA_YOUSTINK: i32 = 24;
pub const QA_GOAWAY: i32 = 25;
pub const QA_DOSOMETHING: i32 = 26;
pub const QA_FUNNYFACE: i32 = 27;
pub const QA_LIKESMILE: i32 = 28;
pub const QA_TURNBACK: i32 = 29;
pub const QA_GREATEST: i32 = 30;
pub const QA_GREATSOLDIER: i32 = 31;
pub const QA_AFRAID: i32 = 32;
pub const QA_WHYMEAN: i32 = 33;
pub const QA_SMELLRATLING: i32 = 34;
pub const QA_WHATSUP: i32 = 35;
pub const QA_SHUTUP: i32 = 36;
pub const QA_DOSOON: i32 = 37;
pub const QA_STOPBOTHER: i32 = 38;
pub const QA_NOTFIGHT: i32 = 39;
pub const QA_BEQUIET: i32 = 40;
pub const QA_ISTHATSO: i32 = 41;
pub const QA_NOTTHATBAD: i32 = 42;
pub const QA_YOUAFRAID: i32 = 43;
pub const QA_TOUGHFELLOW: i32 = 44;
pub const QA_QUIETBIGMOUTH: i32 = 45;
pub const QA_ONEDAY: i32 = 46;
pub const QA_DONTTHINKSO: i32 = 47;
pub const QA_NONEED: i32 = 48;
pub const QA_COWARD: i32 = 49;

/// C `CDR_FDEMON_ARMY`'s `qa[]` rows with `needs_name: 1` (`fdemon.c:139-
/// 183`): the recruited-soldier emote-reaction small talk, matched by
/// [`crate::character_driver::analyse_text_qa_needs_name`] only when the
/// speaker also addressed the soldier by its own name in the same
/// sentence. Digit-for-digit, including `fdemon.c:162`'s `"and you small
/// like a ratling"` (not "smell" - a pre-existing C typo, kept verbatim).
pub const FDEMON_ARMY_EMOTE_QA: &[TextQaEntry] = &[
    TextQaEntry {
        words: &["yes"],
        answer: None,
        answer_code: QA_YES,
    },
    TextQaEntry {
        words: &["yes", "they", "are"],
        answer: None,
        answer_code: QA_YES,
    },
    TextQaEntry {
        words: &["yes", "it", "is"],
        answer: None,
        answer_code: QA_YES,
    },
    TextQaEntry {
        words: &["sure", "they", "are"],
        answer: None,
        answer_code: QA_YES,
    },
    TextQaEntry {
        words: &["sure", "it", "is"],
        answer: None,
        answer_code: QA_YES,
    },
    TextQaEntry {
        words: &["no"],
        answer: None,
        answer_code: QA_NO,
    },
    TextQaEntry {
        words: &["no", "they", "are", "not"],
        answer: None,
        answer_code: QA_NO,
    },
    TextQaEntry {
        words: &["no", "it", "is", "not"],
        answer: None,
        answer_code: QA_NO,
    },
    TextQaEntry {
        words: &["oh", "what", "a", "nice", "day", "it", "is", "isn't", "it"],
        answer: None,
        answer_code: QA_NICEDAY,
    },
    TextQaEntry {
        words: &["the", "nights", "here", "are", "scary", "aren't", "they"],
        answer: None,
        answer_code: QA_NICEDAY,
    },
    TextQaEntry {
        words: &["thanks"],
        answer: None,
        answer_code: QA_THANKS,
    },
    TextQaEntry {
        words: &["thank", "you"],
        answer: None,
        answer_code: QA_THANKS,
    },
    TextQaEntry {
        words: &["why", "thank", "you"],
        answer: None,
        answer_code: QA_THANKS,
    },
    TextQaEntry {
        words: &["why", "thanks"],
        answer: None,
        answer_code: QA_THANKS,
    },
    TextQaEntry {
        words: &["you", "stink"],
        answer: None,
        answer_code: QA_YOUSTINK,
    },
    TextQaEntry {
        words: &["why", "don't", "you", "go", "away"],
        answer: None,
        answer_code: QA_GOAWAY,
    },
    TextQaEntry {
        words: &["oh", "come", "on", "do", "something"],
        answer: None,
        answer_code: QA_DOSOMETHING,
    },
    TextQaEntry {
        words: &["you", "have", "a", "funny", "face"],
        answer: None,
        answer_code: QA_FUNNYFACE,
    },
    TextQaEntry {
        words: &["i", "like", "the", "way", "you", "smile"],
        answer: None,
        answer_code: QA_LIKESMILE,
    },
    TextQaEntry {
        words: &[
            "shouldn't",
            "we",
            "turn",
            "back",
            "what",
            "do",
            "you",
            "think",
        ],
        answer: None,
        answer_code: QA_TURNBACK,
    },
    TextQaEntry {
        words: &["ha", "i'm", "the", "greatest", "right"],
        answer: None,
        answer_code: QA_GREATEST,
    },
    TextQaEntry {
        words: &["i'm", "becoming", "a", "great", "soldier"],
        answer: None,
        answer_code: QA_GREATSOLDIER,
    },
    TextQaEntry {
        words: &["i'm", "afraid"],
        answer: None,
        answer_code: QA_AFRAID,
    },
    TextQaEntry {
        words: &["why", "are", "you", "so", "mean", "to", "me"],
        answer: None,
        answer_code: QA_WHYMEAN,
    },
    TextQaEntry {
        words: &["and", "you", "small", "like", "a", "ratling"],
        answer: None,
        answer_code: QA_SMELLRATLING,
    },
    TextQaEntry {
        words: &["what's", "up", "with", "you"],
        answer: None,
        answer_code: QA_WHATSUP,
    },
    TextQaEntry {
        words: &["shut", "up"],
        answer: None,
        answer_code: QA_SHUTUP,
    },
    TextQaEntry {
        words: &["yeah", "i", "hope", "we'll", "do", "something", "soon"],
        answer: None,
        answer_code: QA_DOSOON,
    },
    TextQaEntry {
        words: &["stop", "bothering", "me"],
        answer: None,
        answer_code: QA_STOPBOTHER,
    },
    TextQaEntry {
        words: &["i'm", "bored", "too", "let's", "not", "fight"],
        answer: None,
        answer_code: QA_NOTFIGHT,
    },
    TextQaEntry {
        words: &["oh", "boy", "please", "be", "quiet"],
        answer: None,
        answer_code: QA_BEQUIET,
    },
    TextQaEntry {
        words: &["is", "that", "so"],
        answer: None,
        answer_code: QA_ISTHATSO,
    },
    TextQaEntry {
        words: &["it's", "not", "that", "bad"],
        answer: None,
        answer_code: QA_NOTTHATBAD,
    },
    TextQaEntry {
        words: &["are", "you", "afraid"],
        answer: None,
        answer_code: QA_YOUAFRAID,
    },
    TextQaEntry {
        words: &["you're", "a", "tough", "fellow"],
        answer: None,
        answer_code: QA_TOUGHFELLOW,
    },
    TextQaEntry {
        words: &["be", "quiet", "you", "bigmouth"],
        answer: None,
        answer_code: QA_QUIETBIGMOUTH,
    },
    TextQaEntry {
        words: &["one", "day", "you'll", "be", "a", "great", "soldier"],
        answer: None,
        answer_code: QA_ONEDAY,
    },
    TextQaEntry {
        words: &["i", "don't", "think", "so"],
        answer: None,
        answer_code: QA_DONTTHINKSO,
    },
    TextQaEntry {
        words: &["there's", "no", "need", "to", "be", "afraid"],
        answer: None,
        answer_code: QA_NONEED,
    },
    TextQaEntry {
        words: &["shut", "up", "you", "you", "coward"],
        answer: None,
        answer_code: QA_COWARD,
    },
];
