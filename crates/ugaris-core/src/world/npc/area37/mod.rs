//! Area 37 (Arkhata) NPCs, one file per NPC.
//!
//! `src/area/37/arkhata.c` (4,764 lines, 23 character drivers) is only
//! partially ported so far - see `PORTING_TODO.md`/`PORTING_LEDGER.md`
//! for the remaining `ramin`/`arkhatamonk`/`captain`/`judge`/
//! `fortressguard`/`jada`/`potmaker`/`hunter`/`thaipan`/`clerk`/
//! `trainer`/`kidnappee`/`krenach` drivers, most of which read/write the
//! shared `struct arkhata_ppd` quest-state blob (`PlayerRuntime::
//! arkhata_ppd`, already scaffolded in `crate::player::areas_misc` for
//! other areas' cross-area reads).
//! `CDR_MADHERMIT` (`src/area/37/arkhata.c::madhermit_driver`, `:4494-
//! 4552`) needs no work here at all - it is byte-for-byte identical to
//! the already-ported Nomad Plains hermit (`world::npc::area19::
//! madhermit`) and dispatches off the same shared `CDR_MADHERMIT`
//! driver id, so `World::process_madhermit_actions` already covers any
//! `CDR_MADHERMIT` character regardless of which area loaded it.

pub mod bridgeguard;
pub mod fiona;
pub mod gladiator;
pub mod jaz;
pub mod nop;
pub mod rammy;

#[allow(unused_imports)]
pub use bridgeguard::*;
#[allow(unused_imports)]
pub use fiona::*;
#[allow(unused_imports)]
pub use gladiator::*;
#[allow(unused_imports)]
pub use jaz::*;
#[allow(unused_imports)]
pub use nop::*;
#[allow(unused_imports)]
pub use rammy::*;

use crate::character_driver::TextQaEntry;

/// C `struct qa qa[]` from `src/area/37/arkhata.c:115-169` - the shared
/// small-talk/skill-raise-request table `analyse_text_driver`'s own local
/// copy in this file feeds every `arkhata.c` NPC driver that calls it
/// (`rammy`/`jaz`/`fiona`/`ramin`/`arkhatamonk`/`captain`/`judge`/`jada`/
/// `potmaker`/`hunter`/`thaipan`/`clerk`/`trainer`/`kidnappee`/`krenach`/
/// `nop`), same "one shared file-local table, many drivers" shape as
/// `world::npc::area36::AREA36_QA`. Every row is ported now, including
/// the 40 `"raise <skill>"` rows (`answer_code = V_<SKILL> + 100`,
/// `V_*` values from `src/server.h:313-351`) even though the only driver
/// in this slice that consumes the table (`nop`) discards
/// `analyse_text_driver`'s return value entirely (C: `analyse_text_driver
/// (cn, msg->dat1, (char *)msg->dat2, co);` with no assignment,
/// `arkhata.c:1319`) - kept for parity with the other, still-unported
/// drivers that do switch on these codes, not silently dropped.
pub const ARKHATA_QA: &[TextQaEntry] = &[
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
        words: &["enter"],
        answer: None,
        answer_code: 5,
    },
    TextQaEntry {
        words: &["aye"],
        answer: None,
        answer_code: 6,
    },
    TextQaEntry {
        words: &["watch"],
        answer: None,
        answer_code: 7,
    },
    // `"raise <skill>"` rows: `answer_code = V_<SKILL> + 100`.
    TextQaEntry {
        words: &["raise", "wisdom"],
        answer: None,
        answer_code: 3 + 100, // V_WIS
    },
    TextQaEntry {
        words: &["raise", "intuition"],
        answer: None,
        answer_code: 4 + 100, // V_INT
    },
    TextQaEntry {
        words: &["raise", "agility"],
        answer: None,
        answer_code: 5 + 100, // V_AGI
    },
    TextQaEntry {
        words: &["raise", "strength"],
        answer: None,
        answer_code: 6 + 100, // V_STR
    },
    TextQaEntry {
        words: &["raise", "pulse"],
        answer: None,
        answer_code: 11 + 100, // V_PULSE
    },
    TextQaEntry {
        words: &["raise", "armor"],
        answer: None,
        answer_code: 17 + 100, // V_ARMORSKILL
    },
    TextQaEntry {
        words: &["raise", "armor", "skill"],
        answer: None,
        answer_code: 17 + 100, // V_ARMORSKILL
    },
    TextQaEntry {
        words: &["raise", "dagger"],
        answer: None,
        answer_code: 12 + 100, // V_DAGGER
    },
    TextQaEntry {
        words: &["raise", "hand"],
        answer: None,
        answer_code: 13 + 100, // V_HAND
    },
    TextQaEntry {
        words: &["raise", "hand", "to", "hand"],
        answer: None,
        answer_code: 13 + 100, // V_HAND
    },
    TextQaEntry {
        words: &["raise", "sword"],
        answer: None,
        answer_code: 15 + 100, // V_SWORD
    },
    TextQaEntry {
        words: &["raise", "staff"],
        answer: None,
        answer_code: 14 + 100, // V_STAFF
    },
    TextQaEntry {
        words: &["raise", "twohanded"],
        answer: None,
        answer_code: 16 + 100, // V_TWOHAND
    },
    TextQaEntry {
        words: &["raise", "two", "handed"],
        answer: None,
        answer_code: 16 + 100, // V_TWOHAND
    },
    TextQaEntry {
        words: &["raise", "two-handed"],
        answer: None,
        answer_code: 16 + 100, // V_TWOHAND
    },
    TextQaEntry {
        words: &["raise", "attack"],
        answer: None,
        answer_code: 18 + 100, // V_ATTACK
    },
    TextQaEntry {
        words: &["raise", "parry"],
        answer: None,
        answer_code: 19 + 100, // V_PARRY
    },
    TextQaEntry {
        words: &["raise", "warcry"],
        answer: None,
        answer_code: 20 + 100, // V_WARCRY
    },
    TextQaEntry {
        words: &["raise", "tactics"],
        answer: None,
        answer_code: 21 + 100, // V_TACTICS
    },
    TextQaEntry {
        words: &["raise", "surround"],
        answer: None,
        answer_code: 22 + 100, // V_SURROUND
    },
    TextQaEntry {
        words: &["raise", "surround", "hit"],
        answer: None,
        answer_code: 22 + 100, // V_SURROUND
    },
    TextQaEntry {
        words: &["raise", "body"],
        answer: None,
        answer_code: 23 + 100, // V_BODYCONTROL
    },
    TextQaEntry {
        words: &["raise", "body", "control"],
        answer: None,
        answer_code: 23 + 100, // V_BODYCONTROL
    },
    TextQaEntry {
        words: &["raise", "speed"],
        answer: None,
        answer_code: 24 + 100, // V_SPEEDSKILL
    },
    TextQaEntry {
        words: &["raise", "barter"],
        answer: None,
        answer_code: 25 + 100, // V_BARTER
    },
    TextQaEntry {
        words: &["raise", "bartering"],
        answer: None,
        answer_code: 25 + 100, // V_BARTER
    },
    TextQaEntry {
        words: &["raise", "perception"],
        answer: None,
        answer_code: 26 + 100, // V_PERCEPT
    },
    TextQaEntry {
        words: &["raise", "stealth"],
        answer: None,
        answer_code: 27 + 100, // V_STEALTH
    },
    TextQaEntry {
        words: &["raise", "bless"],
        answer: None,
        answer_code: 28 + 100, // V_BLESS
    },
    TextQaEntry {
        words: &["raise", "heal"],
        answer: None,
        answer_code: 29 + 100, // V_HEAL
    },
    TextQaEntry {
        words: &["raise", "freeze"],
        answer: None,
        answer_code: 30 + 100, // V_FREEZE
    },
    TextQaEntry {
        words: &["raise", "magic"],
        answer: None,
        answer_code: 31 + 100, // V_MAGICSHIELD
    },
    TextQaEntry {
        words: &["raise", "magic", "shield"],
        answer: None,
        answer_code: 31 + 100, // V_MAGICSHIELD
    },
    TextQaEntry {
        words: &["raise", "lightning"],
        answer: None,
        answer_code: 32 + 100, // V_FLASH
    },
    TextQaEntry {
        words: &["raise", "fire"],
        answer: None,
        answer_code: 33 + 100, // V_FIRE (== V_FIREBALL)
    },
    TextQaEntry {
        words: &["raise", "regenerate"],
        answer: None,
        answer_code: 35 + 100, // V_REGENERATE
    },
    TextQaEntry {
        words: &["raise", "meditate"],
        answer: None,
        answer_code: 36 + 100, // V_MEDITATE
    },
    TextQaEntry {
        words: &["raise", "immunity"],
        answer: None,
        answer_code: 37 + 100, // V_IMMUNITY
    },
    TextQaEntry {
        words: &["raise", "duration"],
        answer: None,
        answer_code: 39 + 100, // V_DURATION
    },
    TextQaEntry {
        words: &["raise", "rage"],
        answer: None,
        answer_code: 40 + 100, // V_RAGE
    },
];
