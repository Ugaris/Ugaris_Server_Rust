use bytes::{Buf, Bytes, BytesMut};
use thiserror::Error;

pub const CL_NOP: u8 = 1;
pub const CL_MOVE: u8 = 2;
pub const CL_SWAP: u8 = 3;
pub const CL_TAKE: u8 = 4;
pub const CL_DROP: u8 = 5;
pub const CL_KILL: u8 = 6;
pub const CL_CONTAINER: u8 = 7;
pub const CL_TEXT: u8 = 8;
pub const CL_USE: u8 = 9;
pub const CL_BLESS: u8 = 10;
pub const CL_FIREBALL: u8 = 11;
pub const CL_HEAL: u8 = 12;
pub const CL_MAGICSHIELD: u8 = 13;
pub const CL_FREEZE: u8 = 14;
pub const CL_RAISE: u8 = 15;
pub const CL_USE_INV: u8 = 16;
pub const CL_FLASH: u8 = 17;
pub const CL_BALL: u8 = 18;
pub const CL_WARCRY: u8 = 19;
pub const CL_LOOK_CONTAINER: u8 = 20;
pub const CL_LOOK_MAP: u8 = 21;
pub const CL_LOOK_INV: u8 = 22;
pub const CL_LOOK_CHAR: u8 = 23;
pub const CL_LOOK_ITEM: u8 = 24;
pub const CL_GIVE: u8 = 25;
pub const CL_SPEED: u8 = 26;
pub const CL_STOP: u8 = 27;
pub const CL_TAKE_GOLD: u8 = 28;
pub const CL_DROP_GOLD: u8 = 29;
pub const CL_JUNK_ITEM: u8 = 30;
pub const CL_CLIENTINFO: u8 = 31;
pub const CL_FIGHTMODE: u8 = 32;
pub const CL_TICKER: u8 = 33;
pub const CL_CONTAINER_FAST: u8 = 34;
pub const CL_FASTSELL: u8 = 35;
pub const CL_LOG: u8 = 36;
pub const CL_TELEPORT: u8 = 37;
pub const CL_PULSE: u8 = 38;
pub const CL_PING: u8 = 39;
pub const CL_GETQUESTLOG: u8 = 40;
pub const CL_REOPENQUEST: u8 = 41;
pub const CL_WALK_DIR: u8 = 42;
pub const CL_MOD1: u8 = 58;
pub const CL_MOD2: u8 = 59;
pub const CL_MOD3: u8 = 60;
pub const CL_MOD4: u8 = 61;
pub const CL_MOD5: u8 = 62;

pub const CLIENT_SURFACE_COUNT: usize = 32;
pub const CLIENT_SURFACE_SIZE: usize = 4;
pub const CLIENT_INFO_SIZE: usize = 24 + CLIENT_SURFACE_COUNT * CLIENT_SURFACE_SIZE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientCommandKind(pub u8);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientCommand {
    pub kind: ClientCommandKind,
    pub bytes: Bytes,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DecodeError {
    #[error("unknown client command {0}")]
    UnknownCommand(u8),
    #[error("unknown mod packet subtype {packet_type}:{subtype}")]
    UnknownModSubtype { packet_type: u8, subtype: u8 },
}

#[derive(Default)]
pub struct ClientCommandDecoder {
    input: BytesMut,
}

impl ClientCommandDecoder {
    pub fn push(&mut self, data: &[u8]) {
        self.input.extend_from_slice(data);
    }

    pub fn next_command(&mut self) -> Result<Option<ClientCommand>, DecodeError> {
        let Some(needed) = client_command_size(&self.input)? else {
            return Ok(None);
        };
        if self.input.len() < needed {
            return Ok(None);
        }

        let bytes = self.input.split_to(needed).freeze();
        Ok(Some(ClientCommand {
            kind: ClientCommandKind(bytes[0]),
            bytes,
        }))
    }
}

pub fn client_command_size(input: &[u8]) -> Result<Option<usize>, DecodeError> {
    if input.is_empty() {
        return Ok(None);
    }

    let cmd = input[0];
    let size = match cmd {
        CL_NOP => 1,
        CL_MOVE => 5,
        CL_SWAP => 2,
        CL_TAKE => 5,
        CL_DROP => 5,
        CL_KILL => 3,
        CL_TEXT | CL_LOG => {
            if input.len() < 2 {
                return Ok(None);
            }
            input[1] as usize + 2
        }
        CL_USE => 5,
        CL_BLESS | CL_HEAL => 3,
        CL_FIREBALL | CL_BALL => 5,
        CL_MAGICSHIELD | CL_FLASH | CL_WARCRY | CL_FREEZE | CL_PULSE => 1,
        CL_CONTAINER | CL_CONTAINER_FAST | CL_LOOK_CONTAINER => 2,
        CL_RAISE => 3,
        CL_USE_INV | CL_FASTSELL => 2,
        CL_LOOK_MAP => 5,
        CL_LOOK_INV => 2,
        CL_LOOK_ITEM => 5,
        CL_LOOK_CHAR | CL_GIVE => 3,
        CL_SPEED | CL_FIGHTMODE => 2,
        CL_STOP => 1,
        CL_WALK_DIR => 2,
        CL_TAKE_GOLD => 5,
        CL_DROP_GOLD | CL_JUNK_ITEM => 1,
        CL_CLIENTINFO => 1 + CLIENT_INFO_SIZE,
        CL_TICKER => 5,
        CL_TELEPORT => 3,
        CL_PING => 5,
        CL_GETQUESTLOG => 1,
        CL_REOPENQUEST => 2,
        CL_MOD1 | CL_MOD3 => {
            if input.len() < 3 {
                return Ok(None);
            }
            mod_packet_size(cmd, input[2])?
        }
        CL_MOD2 | CL_MOD4 | CL_MOD5 => return Err(DecodeError::UnknownCommand(cmd)),
        _ => return Err(DecodeError::UnknownCommand(cmd)),
    };

    Ok(Some(size))
}

fn mod_packet_size(packet_type: u8, subtype: u8) -> Result<usize, DecodeError> {
    let size = match (packet_type, subtype) {
        (CL_MOD1, 0x01) => 7,
        (CL_MOD1, 0x02) => 7,
        (CL_MOD1, 0x03) => 3,
        (CL_MOD1, 0x10) => 11,
        (CL_MOD1, 0x11) => 19,
        (CL_MOD1, 0x12) => 11,
        (CL_MOD1, 0x13) => 31,
        (CL_MOD1, 0x14) => 15,
        (CL_MOD3, _) => {
            return Err(DecodeError::UnknownModSubtype {
                packet_type,
                subtype,
            })
        }
        _ => {
            return Err(DecodeError::UnknownModSubtype {
                packet_type,
                subtype,
            })
        }
    };
    Ok(size)
}

pub fn read_u32_le(payload: &mut &[u8]) -> Option<u32> {
    if payload.len() < 4 {
        return None;
    }
    Some(payload.get_u32_le())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_match_legacy_fixed_commands() {
        assert_eq!(
            client_command_size(&[CL_MOVE, 0, 0, 0, 0]).unwrap(),
            Some(5)
        );
        assert_eq!(client_command_size(&[CL_MAGICSHIELD]).unwrap(), Some(1));
        assert_eq!(client_command_size(&[CL_CLIENTINFO]).unwrap(), Some(153));
    }

    #[test]
    fn text_size_uses_second_byte() {
        assert_eq!(client_command_size(&[CL_TEXT, 5, b'h']).unwrap(), Some(7));
    }

    #[test]
    fn decoder_waits_for_complete_command() {
        let mut decoder = ClientCommandDecoder::default();
        decoder.push(&[CL_MOVE, 1]);
        assert_eq!(decoder.next_command().unwrap(), None);
        decoder.push(&[2, 3, 4]);
        assert_eq!(decoder.next_command().unwrap().unwrap().bytes.len(), 5);
    }
}
