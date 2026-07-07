use serde::{Deserialize, Serialize};

use crate::text::expand_color_sentinels;

pub const LOG_SYSTEM: u8 = 0;
pub const LOG_TALK: u8 = 1;
pub const LOG_SHOUT: u8 = 2;
pub const LOG_INFO: u8 = 3;

pub const LOG_BUFFER_SIZE: usize = 1024;
pub const LOG_TEXT_CAPACITY: usize = 1020;
pub const LEGACY_COLOR_MARKER: u8 = 0xb0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum LogType {
    System = LOG_SYSTEM,
    Talk = LOG_TALK,
    Shout = LOG_SHOUT,
    Info = LOG_INFO,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogMessage {
    pub log_type: LogType,
    pub dat1: u32,
    pub bytes: Vec<u8>,
}

impl LogMessage {
    pub fn new(log_type: LogType, dat1: u32, bytes: impl AsRef<[u8]>) -> Self {
        Self {
            log_type,
            dat1,
            bytes: sanitize_log_bytes(bytes.as_ref()),
        }
    }
}

pub fn sanitize_log_bytes(input: &[u8]) -> Vec<u8> {
    input
        .iter()
        .take(LOG_TEXT_CAPACITY)
        .map(|byte| {
            if byte.is_ascii_graphic() || *byte == b' ' || *byte == LEGACY_COLOR_MARKER {
                *byte
            } else {
                b' '
            }
        })
        .collect()
}

pub fn append_scrollback(
    scrollback: &mut Vec<u8>,
    scrollpos: &mut usize,
    message: &[u8],
    max_scrollback: usize,
) {
    if message.first() == Some(&b'#') || max_scrollback == 0 {
        return;
    }

    let mut bytes = Vec::with_capacity(message.len() + 1);
    bytes.extend_from_slice(message);
    bytes.push(0);

    if scrollback.len() < max_scrollback {
        scrollback.resize(max_scrollback, 0);
    }

    for byte in bytes {
        scrollback[*scrollpos % max_scrollback] = byte;
        *scrollpos = (*scrollpos + 1) % max_scrollback;
    }
}

/// Expands any [`crate::text::COL_STR_RESET`]-family sentinels in `message`
/// (see that constant's doc comment) and sanitizes the resulting bytes.
/// `message` is formatted first as a plain `&str` (sentinels round-trip
/// through `format!`/`Display` fine, since they're ordinary - if unusual -
/// `char`s), then expanded to raw legacy marker bytes right before
/// sanitizing, matching every other message helper's byte-boundary.
fn sanitize_formatted(text: &str) -> Vec<u8> {
    sanitize_log_bytes(&expand_color_sentinels(text))
}

pub fn say_message(name: &str, message: &str) -> Vec<u8> {
    sanitize_formatted(&format!("{name} says: \"{message}\""))
}

pub fn shout_message(name: &str, message: &str) -> Option<Vec<u8>> {
    (!message.contains('"')).then(|| sanitize_formatted(&format!("{name} shouts: \"{message}\"")))
}

pub fn holler_message(name: &str, message: &str) -> Option<Vec<u8>> {
    (!message.contains('"')).then(|| sanitize_formatted(&format!("{name} hollers: \"{message}\"")))
}

pub fn emote_message(name: &str, message: &str) -> Option<Vec<u8>> {
    (!message.contains('"')).then(|| sanitize_formatted(&format!("{name} {message}.")))
}

pub fn whisper_message(name: &str, message: &str) -> Option<Vec<u8>> {
    (!message.contains('"')).then(|| sanitize_formatted(&format!("{name} whispers: \"{message}\"")))
}

/// C `murmur` (`src/system/talk.c:315`): `"%s murmurs: \"%s\""`, same
/// quote-rejection as `whisper`/`emote`/`quiet_say` (`strchr(buf, '"')`).
pub fn murmur_message(name: &str, message: &str) -> Option<Vec<u8>> {
    (!message.contains('"')).then(|| sanitize_formatted(&format!("{name} murmurs: \"{message}\"")))
}

/// C `quiet_say` (`src/system/talk.c:271`): identical wire text to `say`
/// (`"%s says: \"%s\""`), but rejects text containing a `"` (`say` itself
/// has that check commented out - see `say_message`).
pub fn quiet_say_message(name: &str, message: &str) -> Option<Vec<u8>> {
    (!message.contains('"')).then(|| sanitize_formatted(&format!("{name} says: \"{message}\"")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_constants_match_talk_header() {
        assert_eq!(LogType::System as u8, 0);
        assert_eq!(LogType::Talk as u8, 1);
        assert_eq!(LogType::Shout as u8, 2);
        assert_eq!(LogType::Info as u8, 3);
    }

    #[test]
    fn sanitizer_preserves_color_marker_and_replaces_control_bytes() {
        assert_eq!(
            sanitize_log_bytes(&[b'A', 0, LEGACY_COLOR_MARKER]),
            vec![b'A', b' ', LEGACY_COLOR_MARKER]
        );
    }

    #[test]
    fn scrollback_skips_admin_hash_messages() {
        let mut scrollback = Vec::new();
        let mut pos = 0;
        append_scrollback(&mut scrollback, &mut pos, b"#hidden", 16);
        assert_eq!(pos, 0);
        append_scrollback(&mut scrollback, &mut pos, b"hello", 16);
        assert_eq!(pos, 6);
        assert_eq!(&scrollback[..6], b"hello\0");
    }

    #[test]
    fn talk_message_helpers_match_c_format_strings() {
        assert_eq!(say_message("Bob", "Hi"), b"Bob says: \"Hi\"".to_vec());
        assert_eq!(
            shout_message("Bob", "Hi").unwrap(),
            b"Bob shouts: \"Hi\"".to_vec()
        );
        assert_eq!(
            holler_message("Bob", "Hi").unwrap(),
            b"Bob hollers: \"Hi\"".to_vec()
        );
        assert_eq!(
            emote_message("Bob", "waves").unwrap(),
            b"Bob waves.".to_vec()
        );
        assert_eq!(
            whisper_message("Bob", "Hi").unwrap(),
            b"Bob whispers: \"Hi\"".to_vec()
        );
        assert!(shout_message("Bob", "bad\"quote").is_none());
    }

    #[test]
    fn quiet_say_message_expands_color_sentinels_to_legacy_marker_bytes() {
        use crate::text::{COL_STR_LIGHT_BLUE, COL_STR_RESET};

        let message = format!("dost thou want me to {COL_STR_LIGHT_BLUE}repeat{COL_STR_RESET} it?");
        assert_eq!(
            quiet_say_message("Bob", &message).unwrap(),
            b"Bob says: \"dost thou want me to \xb0c4repeat\xb0c0 it?\"".to_vec()
        );
    }

    #[test]
    fn murmur_and_quiet_say_match_c_format_and_reject_quotes() {
        assert_eq!(
            murmur_message("Bob", "psst").unwrap(),
            b"Bob murmurs: \"psst\"".to_vec()
        );
        assert!(murmur_message("Bob", "bad\"quote").is_none());
        assert_eq!(
            quiet_say_message("Bob", "hi").unwrap(),
            b"Bob says: \"hi\"".to_vec()
        );
        assert!(quiet_say_message("Bob", "bad\"quote").is_none());
    }
}
