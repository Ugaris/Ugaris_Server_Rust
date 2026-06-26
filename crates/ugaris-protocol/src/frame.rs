use bytes::{BufMut, BytesMut};
use thiserror::Error;

pub const MAX_LEGACY_TICK_PAYLOAD: usize = 0x3ffe;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FrameError {
    #[error("legacy tick payload exceeds 16 KiB compatibility limit: {0}")]
    PayloadTooLarge(usize),
}

pub fn encode_tick_frame(payload: &[u8]) -> Result<BytesMut, FrameError> {
    if payload.len() > MAX_LEGACY_TICK_PAYLOAD {
        return Err(FrameError::PayloadTooLarge(payload.len()));
    }

    let mut out = BytesMut::with_capacity(payload.len() + 2);
    if payload.len() > 63 {
        out.put_u8((payload.len() >> 8) as u8);
        out.put_u8((payload.len() & 0xff) as u8);
    } else {
        out.put_u8((payload.len() as u8) | 0x40);
    }
    out.extend_from_slice(payload);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_short_payload_like_c_server() {
        let frame = encode_tick_frame(&[9, 1, 2]).unwrap();
        assert_eq!(&frame[..], &[0x43, 9, 1, 2]);
    }

    #[test]
    fn encodes_empty_payload_like_c_pflush() {
        let frame = encode_tick_frame(&[]).unwrap();
        assert_eq!(&frame[..], &[0x40]);
    }

    #[test]
    fn encodes_long_payload_big_endian() {
        let payload = vec![0xaa; 64];
        let frame = encode_tick_frame(&payload).unwrap();
        assert_eq!(frame[0], 0);
        assert_eq!(frame[1], 64);
        assert_eq!(&frame[2..], &payload[..]);
    }
}
