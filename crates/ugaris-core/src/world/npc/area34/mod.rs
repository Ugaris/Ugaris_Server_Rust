//! Area 34 (Teufelheim) NPCs, one file per NPC.
//!
//! `src/area/34/teufel.c` shares one `struct qa qa[]` table (`:138-239`)
//! and one `analyse_text_driver` (`:248-363`) across both
//! `teufelgambler_driver` and `teufelquest_driver` - [`TEUFEL_QA`] and
//! [`teufel_analyse_text`] below are the equivalent shared surface, kept
//! here (rather than duplicated per-NPC-file) so both `teufelgambler`
//! and `teufelquest` can reuse them, matching the `AREA3_QA`/
//! `TWOCITY_QA` precedent from `world::npc::area3`/`world::npc::area17`.

pub mod teufeldemon;
pub mod teufelgambler;
pub mod teufelquest;

#[allow(unused_imports)]
pub use teufeldemon::*;
#[allow(unused_imports)]
pub use teufelgambler::*;
#[allow(unused_imports)]
pub use teufelquest::*;

use crate::character_driver::{analyse_text_qa, TextAnalysisOutcome, TextQaEntry};
use crate::world::*;

/// C `is_demon(cn)` (`teufel.c:366-371`): the three demon-suit sprites
/// that let a player pass as one of Teufelheim's own (earth/fire/ice
/// demon costumes).
pub(crate) fn is_demon(sprite: i32) -> bool {
    matches!(sprite, 27 | 157 | 39)
}

/// C `struct qa qa[]` (`teufel.c:138-239`), shared by
/// `teufelgambler_driver` and `teufelquest_driver`. Answer codes:
/// `1` = "what's your name" family (handled inline by
/// [`teufel_analyse_text`], matching C's own `case 1:` inside
/// `analyse_text_driver` rather than propagating to the caller);
/// `2`/`3`/`4` = "bet one/two/five" (Gambler only, wired in
/// [`teufelgambler`]); `5`/`6`/`7`/`8` = "give experience/military/money/godly"
/// (Quest driver, wired in [`teufelquest`]).
pub(crate) const TEUFEL_QA: &[TextQaEntry] = &[
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
        words: &["play"],
        answer: Some(
            "I only play for bronze chips. You can \u{E0C4}bet one\u{E0C0} or \u{E0C4}bet two\u{E0C0} or \u{E0C4}bet five\u{E0C0} of them. Then you'll roll three dice and depending on the \u{E0C4}results\u{E0C0} you'll win the most fantastic stuff possible!",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["results"],
        answer: Some(
            "The dice are twenty-sided and the numbers are added up. If you roll 3 to 20 you win and if you roll 43 to 60 you win, too. Want to hear about the \u{E0C4}prizes\u{E0C0}?",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["prizes"],
        answer: Some(
            "A 3 gets you a Cape of the Warrior. A 60 a Cape of the Mage. +7 if you bet one chip, +14 if you bet two, +21 if you bet all five. Want to hear \u{E0C4} more prizes\u{E0C0}?",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["more", "prizes"],
        answer: Some(
            "With a 4 or a 59 you'll win 100,000 gold (when betting 5 chips, 20,000 for 1 chip, 40,000 for 2 chips). And there are many, many more prizes...",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["play2"],
        answer: Some(
            "I only play for silver chips. You can \u{E0C4}bet one\u{E0C0} or \u{E0C4}bet two\u{E0C0} or \u{E0C4}bet five\u{E0C0} of them. Then you'll roll three dice and depending on the \u{E0C4}results2\u{E0C0} you'll win the most fantastic stuff possible!",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["results2"],
        answer: Some(
            "The dice are twenty-sided and the numbers are added up. If you roll 3 to 20 you win and if you roll 43 to 60 you win, too. Want to hear about the \u{E0C4}prizes2\u{E0C0}?",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["prizes2"],
        answer: Some(
            "A 3 gets you boots of the Warrior. A 60 boots of the Mage. +8 if you bet one chip, +15 if you bet two, +22 if you bet all five. Want to hear \u{E0C4} more prizes2\u{E0C0}?",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["more", "prizes2"],
        answer: Some(
            "With a 4 or a 59 you'll win 150,000 gold (when betting 5 chips, 30,000 for 1 chip, 60,000 for 2 chips). And there are many, many more prizes...",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["play3"],
        answer: Some(
            "I only play for gold chips. You can \u{E0C4}bet one\u{E0C0} or \u{E0C4}bet two\u{E0C0} or \u{E0C4}bet five\u{E0C0} of them. Then you'll roll three dice and depending on the \u{E0C4}results3\u{E0C0} you'll win the most fantastic stuff possible!",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["results3"],
        answer: Some(
            "The dice are twenty-sided and the numbers are added up. If you roll 3 to 20 you win and if you roll 43 to 60 you win, too. Want to hear about the \u{E0C4}prizes3\u{E0C0}?",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["prizes3"],
        answer: Some(
            "A 3 gets you a helmet of the Warrior. A 60 a hat of the Mage. +9 if you bet one chip, +16 if you bet two, +23 if you bet all five. Want to hear \u{E0C4} more prizes3\u{E0C0}?",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["more", "prizes3"],
        answer: Some(
            "With a 4 or a 59 you'll win 200,000 gold (when betting 5 chips, 40,000 for 1 chip, 80,000 for 2 chips). And there are many, many more prizes...",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["bet", "one"],
        answer: None,
        answer_code: 2,
    },
    TextQaEntry {
        words: &["bet", "two"],
        answer: None,
        answer_code: 3,
    },
    TextQaEntry {
        words: &["bet", "five"],
        answer: None,
        answer_code: 4,
    },
    TextQaEntry {
        words: &["repeat"],
        answer: Some(
            "Hello, %s! We have a slight rat problem in the caverns to the north. There's a nice \u{E0C4}reward\u{E0C0} for killing some rats.",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["reward"],
        answer: Some(
            "Yeah. You go kill some rats. The more and bigger the rats you kill, the more points you get in my book. The more points you have, the better the rewards you get. You know, \u{E0C4}experience\u{E0C0}, \u{E0C4}military\u{E0C0} knowledge or just plain \u{E0C4}money\u{E0C0} if that's what you want.",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["experience"],
        answer: Some(
            "Exactly. Experience. The fire-is-hot-so-don't-touch-it kind of experience. \u{E0C4}Give experience\u{E0C0} will exchange your points for experience.",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["military"],
        answer: Some(
            "That's right. Everything your drill sergeant told you and you can't remember. \u{E0C4}Give military\u{E0C0} will exchange your points for military knowledge.",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["money"],
        answer: Some(
            "You know, them greenbacks. Oh, wait. Wrong dimension. Money... Ah, right. Round, flat and shiny... Coins! That's it. \u{E0C4}Give money\u{E0C0} will exchange your points for greenbacks. Err, gold coins.",
        ),
        answer_code: 0,
    },
    TextQaEntry {
        words: &["give", "experience"],
        answer: None,
        answer_code: 5,
    },
    TextQaEntry {
        words: &["give", "military"],
        answer: None,
        answer_code: 6,
    },
    TextQaEntry {
        words: &["give", "money"],
        answer: None,
        answer_code: 7,
    },
    TextQaEntry {
        words: &["give", "godly"],
        answer: None,
        answer_code: 8,
    },
];

/// Outcome of [`teufel_analyse_text`], distinguishing C's early `return 0`
/// (guard clauses failed, no reply, no facing-turn) from every other case
/// (C always returns a truthy value in every other path - see the
/// function's own doc comment for why).
pub(crate) enum TeufelTextOutcome {
    /// C's `analyse_text_driver` returned `0`: message ignored entirely
    /// (not a player, too far, or not visible).
    Filtered,
    /// C's `analyse_text_driver` returned non-zero (matched, unmatched
    /// with words present, or an empty word list) - carries the
    /// underlying qa-table result for the caller to interpret.
    Recognized(TextAnalysisOutcome),
}

/// C `analyse_text_driver`'s shared guard clauses (`teufel.c:263-273`)
/// plus qa-table dispatch (`teufel.c:325-360`), reused by both
/// `teufelgambler_driver` and `teufelquest_driver`.
///
/// Deviation (documented, not silent): C's function *always* returns a
/// truthy value once the guard clauses pass - whether a qa row matched
/// (`return n` for codes `2..=8`), the "what's your name" row matched
/// (falls through to the unconditional trailing `return 1;` after its
/// inline `quiet_say`), or nothing matched at all, or the word list was
/// even empty (`if (w) { ... }` simply does nothing, still falls through
/// to `return 1;`). Every one of those non-zero outcomes makes the caller
/// enter its `if (n) { talkdir = ...; switch (n) { ... } }` block, so a
/// caller must treat [`TextAnalysisOutcome::NoMatch`] here the same as any
/// other [`TeufelTextOutcome::Recognized`] variant for the "set talkdir"
/// side effect, even though nothing was said.
pub(crate) fn teufel_analyse_text(
    world: &World,
    npc: &Character,
    speaker: &Character,
    text: &str,
) -> TeufelTextOutcome {
    if npc.id == speaker.id
        || !speaker.flags.contains(CharacterFlags::PLAYER)
        || char_dist(npc, speaker) > 12
        || !char_see_char(npc, speaker, &world.map, world.date.daylight)
    {
        return TeufelTextOutcome::Filtered;
    }
    TeufelTextOutcome::Recognized(analyse_text_qa(text, &npc.name, &speaker.name, TEUFEL_QA))
}
