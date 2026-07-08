//! Area 3 NPCs, one file per NPC.

pub mod astro1;
pub mod astro2;
pub mod carlos;
pub mod clara;
pub mod kassim;
pub mod kelly;
pub mod seymour;
pub mod sir_jones;
pub mod supermax;
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
pub use supermax::*;
#[allow(unused_imports)]
pub use thomas::*;

use crate::character_driver::TextQaEntry;
use crate::entity::CharacterValue;

/// C `struct qa qa[]` from `src/area/3/area3.c:106-204` - the small-talk
/// table `analyse_text_driver`'s own local copy in this file feeds every
/// `area3.c` NPC driver that calls it (`world::thomas`, `world::
/// sir_jones`, `world::astro2`, `world::seymour`, `world::kelly`, `world::
/// carlos`, `world::kassim`, and `world::supermax`), not just one - same
/// "one shared file-local table, many drivers" shape as `world::npc::
/// area1::gwendylon::GWENDYLON_QA`.
///
/// Every entry is now ported: the first 13 (`area3.c:106-118`, canned
/// greetings plus `repeat`/`restart`/`aye`/`nay`), `shortcut to
/// caligar`(7) (`area3.c:121`, `world::kelly`'s own `case 7` god-only
/// fast-forward), `explain`(9) (`area3.c:122`, `world::kassim`'s own
/// `case 9` service explanation), and - the last remaining gap -
/// `list`(5)/`money`(6) plus the 80-row `raise`/`lower` skill block
/// (`area3.c:119-204`), all consumed only by `world::supermax`. `answer_
/// code` for each raise/lower row is `V_* + 100`/`V_* + 200` exactly as
/// C encodes it (`skl = didsay - 100`/`didsay - 200` in `supermax_
/// driver`); `CharacterValue`'s numeric values match every `V_*` index in
/// `src/system/skill.h` one for one, so `CharacterValue::X as i32 + 100`
/// reproduces the C constant without a separate lookup table. The
/// `engrave: `(8) row (`area3.c:123`) is genuinely dead code in C -
/// `kassim_driver`'s `NT_TEXT` branch special-cases `strcasestr` for
/// `"engrave:"` *before* ever calling `analyse_text_driver`
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
    // C `area3.c:119-120`: `world::supermax`'s own service-list/gold-
    // spent-so-far commands.
    TextQaEntry {
        words: &["list"],
        answer: None,
        answer_code: 5,
    },
    TextQaEntry {
        words: &["money"],
        answer: None,
        answer_code: 6,
    },
    // C `area3.c:125-204`: the 80-row `raise <skill>`/`lower <skill>`
    // block, keyed on `V_* + 100`/`V_* + 200` (`world::supermax`'s
    // `supermax_raise`/`supermax_lower`). Word patterns and skill order
    // copied verbatim.
    TextQaEntry {
        words: &["raise", "wisdom"],
        answer: None,
        answer_code: CharacterValue::Wisdom as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "wisdom"],
        answer: None,
        answer_code: CharacterValue::Wisdom as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "intuition"],
        answer: None,
        answer_code: CharacterValue::Intelligence as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "intuition"],
        answer: None,
        answer_code: CharacterValue::Intelligence as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "agility"],
        answer: None,
        answer_code: CharacterValue::Agility as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "agility"],
        answer: None,
        answer_code: CharacterValue::Agility as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "strength"],
        answer: None,
        answer_code: CharacterValue::Strength as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "strength"],
        answer: None,
        answer_code: CharacterValue::Strength as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "pulse"],
        answer: None,
        answer_code: CharacterValue::Pulse as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "pulse"],
        answer: None,
        answer_code: CharacterValue::Pulse as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "armor"],
        answer: None,
        answer_code: CharacterValue::ArmorSkill as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "armor"],
        answer: None,
        answer_code: CharacterValue::ArmorSkill as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "armor", "skill"],
        answer: None,
        answer_code: CharacterValue::ArmorSkill as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "armor", "skill"],
        answer: None,
        answer_code: CharacterValue::ArmorSkill as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "dagger"],
        answer: None,
        answer_code: CharacterValue::Dagger as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "dagger"],
        answer: None,
        answer_code: CharacterValue::Dagger as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "hand"],
        answer: None,
        answer_code: CharacterValue::Hand as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "hand"],
        answer: None,
        answer_code: CharacterValue::Hand as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "hand", "to", "hand"],
        answer: None,
        answer_code: CharacterValue::Hand as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "hand", "to", "hand"],
        answer: None,
        answer_code: CharacterValue::Hand as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "sword"],
        answer: None,
        answer_code: CharacterValue::Sword as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "sword"],
        answer: None,
        answer_code: CharacterValue::Sword as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "staff"],
        answer: None,
        answer_code: CharacterValue::Staff as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "staff"],
        answer: None,
        answer_code: CharacterValue::Staff as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "twohanded"],
        answer: None,
        answer_code: CharacterValue::TwoHand as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "twohanded"],
        answer: None,
        answer_code: CharacterValue::TwoHand as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "two", "handed"],
        answer: None,
        answer_code: CharacterValue::TwoHand as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "two", "handed"],
        answer: None,
        answer_code: CharacterValue::TwoHand as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "two-handed"],
        answer: None,
        answer_code: CharacterValue::TwoHand as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "two-handed"],
        answer: None,
        answer_code: CharacterValue::TwoHand as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "attack"],
        answer: None,
        answer_code: CharacterValue::Attack as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "attack"],
        answer: None,
        answer_code: CharacterValue::Attack as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "parry"],
        answer: None,
        answer_code: CharacterValue::Parry as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "parry"],
        answer: None,
        answer_code: CharacterValue::Parry as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "warcry"],
        answer: None,
        answer_code: CharacterValue::Warcry as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "warcry"],
        answer: None,
        answer_code: CharacterValue::Warcry as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "tactics"],
        answer: None,
        answer_code: CharacterValue::Tactics as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "tactics"],
        answer: None,
        answer_code: CharacterValue::Tactics as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "surround"],
        answer: None,
        answer_code: CharacterValue::Surround as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "surround"],
        answer: None,
        answer_code: CharacterValue::Surround as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "surround", "hit"],
        answer: None,
        answer_code: CharacterValue::Surround as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "surround", "hit"],
        answer: None,
        answer_code: CharacterValue::Surround as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "body"],
        answer: None,
        answer_code: CharacterValue::BodyControl as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "body"],
        answer: None,
        answer_code: CharacterValue::BodyControl as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "body", "control"],
        answer: None,
        answer_code: CharacterValue::BodyControl as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "body", "control"],
        answer: None,
        answer_code: CharacterValue::BodyControl as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "speed"],
        answer: None,
        answer_code: CharacterValue::SpeedSkill as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "speed"],
        answer: None,
        answer_code: CharacterValue::SpeedSkill as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "barter"],
        answer: None,
        answer_code: CharacterValue::Barter as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "barter"],
        answer: None,
        answer_code: CharacterValue::Barter as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "bartering"],
        answer: None,
        answer_code: CharacterValue::Barter as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "bartering"],
        answer: None,
        answer_code: CharacterValue::Barter as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "perception"],
        answer: None,
        answer_code: CharacterValue::Percept as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "perception"],
        answer: None,
        answer_code: CharacterValue::Percept as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "stealth"],
        answer: None,
        answer_code: CharacterValue::Stealth as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "stealth"],
        answer: None,
        answer_code: CharacterValue::Stealth as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "bless"],
        answer: None,
        answer_code: CharacterValue::Bless as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "bless"],
        answer: None,
        answer_code: CharacterValue::Bless as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "heal"],
        answer: None,
        answer_code: CharacterValue::Heal as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "heal"],
        answer: None,
        answer_code: CharacterValue::Heal as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "freeze"],
        answer: None,
        answer_code: CharacterValue::Freeze as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "freeze"],
        answer: None,
        answer_code: CharacterValue::Freeze as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "magic"],
        answer: None,
        answer_code: CharacterValue::MagicShield as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "magic"],
        answer: None,
        answer_code: CharacterValue::MagicShield as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "magic", "shield"],
        answer: None,
        answer_code: CharacterValue::MagicShield as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "magic", "shield"],
        answer: None,
        answer_code: CharacterValue::MagicShield as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "lightning"],
        answer: None,
        answer_code: CharacterValue::Flash as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "lightning"],
        answer: None,
        answer_code: CharacterValue::Flash as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "fire"],
        answer: None,
        answer_code: CharacterValue::Fireball as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "fire"],
        answer: None,
        answer_code: CharacterValue::Fireball as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "regenerate"],
        answer: None,
        answer_code: CharacterValue::Regenerate as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "regenerate"],
        answer: None,
        answer_code: CharacterValue::Regenerate as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "meditate"],
        answer: None,
        answer_code: CharacterValue::Meditate as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "meditate"],
        answer: None,
        answer_code: CharacterValue::Meditate as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "immunity"],
        answer: None,
        answer_code: CharacterValue::Immunity as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "immunity"],
        answer: None,
        answer_code: CharacterValue::Immunity as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "duration"],
        answer: None,
        answer_code: CharacterValue::Duration as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "duration"],
        answer: None,
        answer_code: CharacterValue::Duration as i32 + 200,
    },
    TextQaEntry {
        words: &["raise", "rage"],
        answer: None,
        answer_code: CharacterValue::Rage as i32 + 100,
    },
    TextQaEntry {
        words: &["lower", "rage"],
        answer: None,
        answer_code: CharacterValue::Rage as i32 + 200,
    },
];
