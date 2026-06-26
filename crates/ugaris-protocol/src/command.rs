use thiserror::Error;

use crate::client::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientAction {
    Nop,
    Move {
        x: u16,
        y: u16,
    },
    Swap {
        slot: u8,
    },
    Take {
        x: u16,
        y: u16,
    },
    Drop {
        x: u16,
        y: u16,
    },
    Kill {
        character: u16,
    },
    Text(Vec<u8>),
    Log(Vec<u8>),
    UseMap {
        x: u16,
        y: u16,
    },
    CharacterSpell {
        spell: SpellAction,
        character: u16,
    },
    MapSpell {
        spell: SpellAction,
        x: u16,
        y: u16,
    },
    SelfSpell {
        spell: SpellAction,
    },
    Container {
        slot: u8,
        fast: bool,
    },
    LookContainer {
        slot: u8,
    },
    Raise {
        value: u16,
    },
    UseInventory {
        slot: u8,
    },
    FastSell {
        slot: u8,
    },
    LookMap {
        x: u16,
        y: u16,
    },
    LookInventory {
        slot: u8,
    },
    LookItem {
        x: u16,
        y: u16,
    },
    LookCharacter {
        character: u16,
    },
    Give {
        character: u16,
    },
    Speed {
        mode: u8,
    },
    FightMode {
        mode: u8,
    },
    Stop,
    TakeGold {
        amount: u32,
    },
    DropGold,
    JunkItem,
    ClientInfo(Vec<u8>),
    Ticker {
        tick: u32,
    },
    Teleport {
        teleport: u8,
        mirror: u8,
    },
    Ping {
        value: u32,
    },
    GetQuestLog,
    ReopenQuest {
        quest: u8,
    },
    WalkDir {
        direction: u8,
    },
    ModPacket {
        packet_type: u8,
        subtype: u8,
        bytes: Vec<u8>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellAction {
    Bless,
    Fireball,
    Heal,
    MagicShield,
    Freeze,
    Flash,
    Ball,
    Warcry,
    Pulse,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CommandParseError {
    #[error("command payload is too short for {command}: expected {expected}, got {actual}")]
    TooShort {
        command: u8,
        expected: usize,
        actual: usize,
    },
    #[error("unknown client command {0}")]
    UnknownCommand(u8),
}

impl TryFrom<&ClientCommand> for ClientAction {
    type Error = CommandParseError;

    fn try_from(command: &ClientCommand) -> Result<Self, Self::Error> {
        parse_action(&command.bytes)
    }
}

pub fn parse_action(bytes: &[u8]) -> Result<ClientAction, CommandParseError> {
    let command = *bytes.first().ok_or(CommandParseError::TooShort {
        command: 0,
        expected: 1,
        actual: 0,
    })?;

    match command {
        CL_NOP => Ok(ClientAction::Nop),
        CL_MOVE => Ok(ClientAction::Move {
            x: u16_at(bytes, 1)?,
            y: u16_at(bytes, 3)?,
        }),
        CL_SWAP => Ok(ClientAction::Swap {
            slot: u8_at(bytes, 1)?,
        }),
        CL_TAKE => Ok(ClientAction::Take {
            x: u16_at(bytes, 1)?,
            y: u16_at(bytes, 3)?,
        }),
        CL_DROP => Ok(ClientAction::Drop {
            x: u16_at(bytes, 1)?,
            y: u16_at(bytes, 3)?,
        }),
        CL_KILL => Ok(ClientAction::Kill {
            character: u16_at(bytes, 1)?,
        }),
        CL_TEXT => Ok(ClientAction::Text(var_payload(bytes, command)?)),
        CL_LOG => Ok(ClientAction::Log(var_payload(bytes, command)?)),
        CL_USE => Ok(ClientAction::UseMap {
            x: u16_at(bytes, 1)?,
            y: u16_at(bytes, 3)?,
        }),
        CL_BLESS => Ok(ClientAction::CharacterSpell {
            spell: SpellAction::Bless,
            character: u16_at(bytes, 1)?,
        }),
        CL_HEAL => Ok(ClientAction::CharacterSpell {
            spell: SpellAction::Heal,
            character: u16_at(bytes, 1)?,
        }),
        CL_FIREBALL => Ok(ClientAction::MapSpell {
            spell: SpellAction::Fireball,
            x: u16_at(bytes, 1)?,
            y: u16_at(bytes, 3)?,
        }),
        CL_BALL => Ok(ClientAction::MapSpell {
            spell: SpellAction::Ball,
            x: u16_at(bytes, 1)?,
            y: u16_at(bytes, 3)?,
        }),
        CL_MAGICSHIELD => Ok(ClientAction::SelfSpell {
            spell: SpellAction::MagicShield,
        }),
        CL_FLASH => Ok(ClientAction::SelfSpell {
            spell: SpellAction::Flash,
        }),
        CL_WARCRY => Ok(ClientAction::SelfSpell {
            spell: SpellAction::Warcry,
        }),
        CL_FREEZE => Ok(ClientAction::SelfSpell {
            spell: SpellAction::Freeze,
        }),
        CL_PULSE => Ok(ClientAction::SelfSpell {
            spell: SpellAction::Pulse,
        }),
        CL_CONTAINER => Ok(ClientAction::Container {
            slot: u8_at(bytes, 1)?,
            fast: false,
        }),
        CL_CONTAINER_FAST => Ok(ClientAction::Container {
            slot: u8_at(bytes, 1)?,
            fast: true,
        }),
        CL_LOOK_CONTAINER => Ok(ClientAction::LookContainer {
            slot: u8_at(bytes, 1)?,
        }),
        CL_RAISE => Ok(ClientAction::Raise {
            value: u16_at(bytes, 1)?,
        }),
        CL_USE_INV => Ok(ClientAction::UseInventory {
            slot: u8_at(bytes, 1)?,
        }),
        CL_FASTSELL => Ok(ClientAction::FastSell {
            slot: u8_at(bytes, 1)?,
        }),
        CL_LOOK_MAP => Ok(ClientAction::LookMap {
            x: u16_at(bytes, 1)?,
            y: u16_at(bytes, 3)?,
        }),
        CL_LOOK_INV => Ok(ClientAction::LookInventory {
            slot: u8_at(bytes, 1)?,
        }),
        CL_LOOK_ITEM => Ok(ClientAction::LookItem {
            x: u16_at(bytes, 1)?,
            y: u16_at(bytes, 3)?,
        }),
        CL_LOOK_CHAR => Ok(ClientAction::LookCharacter {
            character: u16_at(bytes, 1)?,
        }),
        CL_GIVE => Ok(ClientAction::Give {
            character: u16_at(bytes, 1)?,
        }),
        CL_SPEED => Ok(ClientAction::Speed {
            mode: u8_at(bytes, 1)?,
        }),
        CL_FIGHTMODE => Ok(ClientAction::FightMode {
            mode: u8_at(bytes, 1)?,
        }),
        CL_STOP => Ok(ClientAction::Stop),
        CL_TAKE_GOLD => Ok(ClientAction::TakeGold {
            amount: u32_at(bytes, 1)?,
        }),
        CL_DROP_GOLD => Ok(ClientAction::DropGold),
        CL_JUNK_ITEM => Ok(ClientAction::JunkItem),
        CL_CLIENTINFO => Ok(ClientAction::ClientInfo(bytes[1..].to_vec())),
        CL_TICKER => Ok(ClientAction::Ticker {
            tick: u32_at(bytes, 1)?,
        }),
        CL_TELEPORT => Ok(ClientAction::Teleport {
            teleport: u8_at(bytes, 1)?,
            mirror: u8_at(bytes, 2)?,
        }),
        CL_PING => Ok(ClientAction::Ping {
            value: u32_at(bytes, 1)?,
        }),
        CL_GETQUESTLOG => Ok(ClientAction::GetQuestLog),
        CL_REOPENQUEST => Ok(ClientAction::ReopenQuest {
            quest: u8_at(bytes, 1)?,
        }),
        CL_WALK_DIR => Ok(ClientAction::WalkDir {
            direction: u8_at(bytes, 1)?,
        }),
        CL_MOD1 | CL_MOD2 | CL_MOD3 | CL_MOD4 | CL_MOD5 => Ok(ClientAction::ModPacket {
            packet_type: command,
            subtype: u8_at(bytes, 2)?,
            bytes: bytes.to_vec(),
        }),
        other => Err(CommandParseError::UnknownCommand(other)),
    }
}

fn var_payload(bytes: &[u8], command: u8) -> Result<Vec<u8>, CommandParseError> {
    let len = u8_at(bytes, 1)? as usize;
    let end = 2 + len;
    if bytes.len() < end {
        return Err(CommandParseError::TooShort {
            command,
            expected: end,
            actual: bytes.len(),
        });
    }
    Ok(bytes[2..end].to_vec())
}

fn u8_at(bytes: &[u8], offset: usize) -> Result<u8, CommandParseError> {
    bytes
        .get(offset)
        .copied()
        .ok_or(CommandParseError::TooShort {
            command: bytes.first().copied().unwrap_or_default(),
            expected: offset + 1,
            actual: bytes.len(),
        })
}

fn u16_at(bytes: &[u8], offset: usize) -> Result<u16, CommandParseError> {
    if bytes.len() < offset + 2 {
        return Err(CommandParseError::TooShort {
            command: bytes.first().copied().unwrap_or_default(),
            expected: offset + 2,
            actual: bytes.len(),
        });
    }
    Ok(u16::from_le_bytes([bytes[offset], bytes[offset + 1]]))
}

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, CommandParseError> {
    if bytes.len() < offset + 4 {
        return Err(CommandParseError::TooShort {
            command: bytes.first().copied().unwrap_or_default(),
            expected: offset + 4,
            actual: bytes.len(),
        });
    }
    Ok(u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ]))
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::*;
    use crate::client::ClientCommandKind;

    #[test]
    fn parses_move_like_c_casts() {
        let command = ClientCommand {
            kind: ClientCommandKind(CL_MOVE),
            bytes: Bytes::from_static(&[CL_MOVE, 0x34, 0x12, 0x78, 0x56]),
        };
        assert_eq!(
            ClientAction::try_from(&command).unwrap(),
            ClientAction::Move {
                x: 0x1234,
                y: 0x5678
            }
        );
    }

    #[test]
    fn parses_text_as_raw_bytes() {
        let command = ClientCommand {
            kind: ClientCommandKind(CL_TEXT),
            bytes: Bytes::from_static(&[CL_TEXT, 4, 0xb0, b'c', b'3', b'!']),
        };
        assert_eq!(
            ClientAction::try_from(&command).unwrap(),
            ClientAction::Text(vec![0xb0, b'c', b'3', b'!'])
        );
    }

    #[test]
    fn parses_gold_amount_little_endian() {
        assert_eq!(
            parse_action(&[CL_TAKE_GOLD, 1, 2, 3, 4]).unwrap(),
            ClientAction::TakeGold { amount: 0x04030201 }
        );
    }
}
