//! Registry-edge dispatch: `CharacterDriverKind` and the legacy `ch_driver`
//! entry points.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterDriverKind {
    SimpleBaddy,
    Macro,
    SwampClara,
    SwampMonster,
    PalaceIslena,
    Trader,
    LqNpc,
    Janitor,
    TeufelDemon,
    TeufelGambler,
    TeufelQuest,
    TeufelRat,
    CaligarSkelly,
    Lab2Undead,
}
impl CharacterDriverKind {
    pub fn from_legacy_id(driver: u16) -> Option<Self> {
        match driver {
            CDR_SIMPLEBADDY => Some(Self::SimpleBaddy),
            CDR_MACRO => Some(Self::Macro),
            CDR_SWAMPCLARA => Some(Self::SwampClara),
            CDR_SWAMPMONSTER => Some(Self::SwampMonster),
            CDR_PALACEISLENA => Some(Self::PalaceIslena),
            CDR_TRADER => Some(Self::Trader),
            CDR_LQNPC => Some(Self::LqNpc),
            CDR_JANITOR => Some(Self::Janitor),
            CDR_TEUFELDEMON => Some(Self::TeufelDemon),
            CDR_TEUFELGAMBLER => Some(Self::TeufelGambler),
            CDR_TEUFELQUEST => Some(Self::TeufelQuest),
            CDR_TEUFELRAT => Some(Self::TeufelRat),
            CDR_CALIGARSKELLY => Some(Self::CaligarSkelly),
            CDR_LAB2UNDEAD => Some(Self::Lab2Undead),
            _ => None,
        }
    }

    pub fn legacy_id(self) -> u16 {
        match self {
            Self::SimpleBaddy => CDR_SIMPLEBADDY,
            Self::Macro => CDR_MACRO,
            Self::SwampClara => CDR_SWAMPCLARA,
            Self::SwampMonster => CDR_SWAMPMONSTER,
            Self::PalaceIslena => CDR_PALACEISLENA,
            Self::Trader => CDR_TRADER,
            Self::LqNpc => CDR_LQNPC,
            Self::Janitor => CDR_JANITOR,
            Self::TeufelDemon => CDR_TEUFELDEMON,
            Self::TeufelGambler => CDR_TEUFELGAMBLER,
            Self::TeufelQuest => CDR_TEUFELQUEST,
            Self::TeufelRat => CDR_TEUFELRAT,
            Self::CaligarSkelly => CDR_CALIGARSKELLY,
            Self::Lab2Undead => CDR_LAB2UNDEAD,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterDriverCall {
    Tick { ret: i32, last_action: i32 },
    Died { killer_character_id: u32 },
    Respawn,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterDriverOutcome {
    /// `simple_baddy_dead`: earth demons create earth/rain retaliation effects
    /// at the killer position when the dead NPC can see the killer.
    SimpleBaddyDeath { killer_character_id: u32 },
    /// Legacy handler returned `1`; behavior is intentionally deferred to a
    /// future typed implementation for this concrete driver.
    HandledStub {
        kind: CharacterDriverKind,
        call: CharacterDriverCall,
    },
    /// Legacy module probing would continue and eventually return `0`.
    Unsupported {
        driver: u16,
        call: CharacterDriverCall,
    },
}
impl CharacterDriverOutcome {
    pub fn legacy_return_code(self) -> i32 {
        match self {
            Self::SimpleBaddyDeath { .. } => 1,
            Self::HandledStub { .. } => 1,
            Self::Unsupported { .. } => 0,
        }
    }
}
pub fn execute_character_driver(driver: u16, ret: i32, last_action: i32) -> CharacterDriverOutcome {
    let call = CharacterDriverCall::Tick { ret, last_action };
    dispatch_known_character_driver(driver, call)
}
pub fn execute_character_died_driver(
    driver: u16,
    killer_character_id: u32,
) -> CharacterDriverOutcome {
    let call = CharacterDriverCall::Died {
        killer_character_id,
    };
    dispatch_known_character_driver(driver, call)
}
pub fn execute_character_respawn_driver(driver: u16) -> CharacterDriverOutcome {
    dispatch_known_character_driver(driver, CharacterDriverCall::Respawn)
}
fn dispatch_known_character_driver(
    driver: u16,
    call: CharacterDriverCall,
) -> CharacterDriverOutcome {
    if driver == CDR_SIMPLEBADDY {
        if let CharacterDriverCall::Died {
            killer_character_id,
        } = call
        {
            return CharacterDriverOutcome::SimpleBaddyDeath {
                killer_character_id,
            };
        }
    }

    match CharacterDriverKind::from_legacy_id(driver) {
        Some(kind) => CharacterDriverOutcome::HandledStub { kind, call },
        None => CharacterDriverOutcome::Unsupported { driver, call },
    }
}
