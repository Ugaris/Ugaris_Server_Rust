//! Static character-driver registry boundary for legacy `ch_driver` dispatch.
//!
//! The C server dynamically probes module libraries. The Rust rewrite keeps the
//! same numeric compatibility at the registry edge while routing known drivers
//! to typed outcomes that can be filled in incrementally.

pub const CDT_DRIVER: u16 = 0;
pub const CDT_ITEM: u16 = 1;
pub const CDT_DEAD: u16 = 2;
pub const CDT_RESPAWN: u16 = 3;
pub const CDT_SPECIAL: u16 = 4;

pub const CDR_MACRO: u16 = 37;
pub const CDR_TRADER: u16 = 72;
pub const CDR_JANITOR: u16 = 85;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterDriverKind {
    Macro,
    Trader,
    Janitor,
}

impl CharacterDriverKind {
    pub fn from_legacy_id(driver: u16) -> Option<Self> {
        match driver {
            CDR_MACRO => Some(Self::Macro),
            CDR_TRADER => Some(Self::Trader),
            CDR_JANITOR => Some(Self::Janitor),
            _ => None,
        }
    }

    pub fn legacy_id(self) -> u16 {
        match self {
            Self::Macro => CDR_MACRO,
            Self::Trader => CDR_TRADER,
            Self::Janitor => CDR_JANITOR,
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
    match CharacterDriverKind::from_legacy_id(driver) {
        Some(kind) => CharacterDriverOutcome::HandledStub { kind, call },
        None => CharacterDriverOutcome::Unsupported { driver, call },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_dispatch_type_constants_match_c_libload() {
        assert_eq!(CDT_DRIVER, 0);
        assert_eq!(CDT_ITEM, 1);
        assert_eq!(CDT_DEAD, 2);
        assert_eq!(CDT_RESPAWN, 3);
        assert_eq!(CDT_SPECIAL, 4);
    }

    #[test]
    fn base_character_driver_ids_match_c_drvlib() {
        assert_eq!(CDR_MACRO, 37);
        assert_eq!(CDR_TRADER, 72);
        assert_eq!(CDR_JANITOR, 85);
        assert_eq!(CharacterDriverKind::Macro.legacy_id(), CDR_MACRO);
        assert_eq!(CharacterDriverKind::Trader.legacy_id(), CDR_TRADER);
        assert_eq!(CharacterDriverKind::Janitor.legacy_id(), CDR_JANITOR);
    }

    #[test]
    fn known_base_tick_drivers_are_handled_like_c_ch_driver() {
        for (driver, kind) in [
            (CDR_MACRO, CharacterDriverKind::Macro),
            (CDR_TRADER, CharacterDriverKind::Trader),
            (CDR_JANITOR, CharacterDriverKind::Janitor),
        ] {
            let outcome = execute_character_driver(driver, 7, 11);
            assert_eq!(
                outcome,
                CharacterDriverOutcome::HandledStub {
                    kind,
                    call: CharacterDriverCall::Tick {
                        ret: 7,
                        last_action: 11,
                    },
                }
            );
            assert_eq!(outcome.legacy_return_code(), 1);
        }
    }

    #[test]
    fn known_base_death_and_respawn_drivers_are_handled_like_c() {
        let died = execute_character_died_driver(CDR_JANITOR, 123);
        assert_eq!(
            died,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::Janitor,
                call: CharacterDriverCall::Died {
                    killer_character_id: 123,
                },
            }
        );
        assert_eq!(died.legacy_return_code(), 1);

        let respawn = execute_character_respawn_driver(CDR_TRADER);
        assert_eq!(
            respawn,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::Trader,
                call: CharacterDriverCall::Respawn,
            }
        );
        assert_eq!(respawn.legacy_return_code(), 1);
    }

    #[test]
    fn unknown_character_driver_returns_legacy_zero() {
        let outcome = execute_character_driver(999, 0, 0);
        assert_eq!(
            outcome,
            CharacterDriverOutcome::Unsupported {
                driver: 999,
                call: CharacterDriverCall::Tick {
                    ret: 0,
                    last_action: 0,
                },
            }
        );
        assert_eq!(outcome.legacy_return_code(), 0);
    }
}
