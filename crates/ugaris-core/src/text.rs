pub const COLOR_MARKER: &[u8] = b"\xb0";

pub const COL_RESET: &[u8] = b"\xb0c0";
pub const COL_WHITE: &[u8] = COL_RESET;
pub const COL_DARK_GRAY: &[u8] = b"\xb0c1";
pub const COL_LIGHT_GREEN: &[u8] = b"\xb0c2";
pub const COL_LIGHT_RED: &[u8] = b"\xb0c3";
pub const COL_LIGHT_BLUE: &[u8] = b"\xb0c4";
pub const COL_LINK: &[u8] = COL_LIGHT_BLUE;
pub const COL_ORANGE: &[u8] = b"\xb0c5";
pub const COL_YELLOW: &[u8] = b"\xb0c6";
pub const COL_VIOLET: &[u8] = b"\xb0c7";
pub const COL_LIGHT_VIOLET: &[u8] = b"\xb0c8";
pub const COL_TAN: &[u8] = b"\xb0c9";
pub const COL_KHAKI: &[u8] = COL_TAN;
pub const COL_MAUVE: &[u8] = b"\xb0c10";
pub const COL_CYAN: &[u8] = b"\xb0c11";
pub const COL_PEACH: &[u8] = b"\xb0c12";
pub const COL_PINK: &[u8] = b"\xb0c13";
pub const COL_AQUA: &[u8] = b"\xb0c14";
pub const COL_LIME: &[u8] = b"\xb0c15";
pub const COL_PURPLE: &[u8] = b"\xb0c16";
pub const COL_HIDDEN_LINK: &[u8] = b"\xb0c17";
pub const COL_LINK_RESET: &[u8] = b"\xb0c18";

pub const COL_KEYWORD: &[u8] = COL_LIGHT_BLUE;
pub const COL_ERROR: &[u8] = COL_LIGHT_RED;
pub const COL_ANNOUNCE: &[u8] = COL_LIGHT_RED;
pub const COL_SUCCESS: &[u8] = COL_LIGHT_GREEN;
pub const COL_CHAT: &[u8] = COL_LIGHT_GREEN;
pub const COL_HEADING: &[u8] = COL_ORANGE;
pub const COL_TELL: &[u8] = COL_YELLOW;
pub const COL_STAFF: &[u8] = COL_VIOLET;
pub const COL_GOD: &[u8] = COL_LIGHT_VIOLET;
pub const COL_CHAT_AUCTION: &[u8] = COL_TAN;
pub const COL_CHAT_GRATS: &[u8] = COL_MAUVE;
pub const COL_CHAT_MIRROR: &[u8] = COL_CYAN;
pub const COL_CHAT_INFO: &[u8] = COL_PEACH;
pub const COL_CHAT_AREA: &[u8] = COL_PINK;
pub const COL_CHAT_GAMES: &[u8] = COL_AQUA;
pub const COL_CHAT_CLAN: &[u8] = COL_LIME;
pub const COL_CHAT_CLAN_INT: &[u8] = COL_PURPLE;

pub const ITEMDESC_MARKER: &[u8] = b"\xb0\xb0\xb0";

pub fn runtime_color(color: u8) -> Vec<u8> {
    let mut out = Vec::with_capacity(5);
    out.push(0xb0);
    out.extend_from_slice(format!("c{color}").as_bytes());
    out
}

/// Sentinel stand-ins for [`COL_RESET`]/etc. usable inside a plain Rust
/// `&str` (the raw `0xb0` marker byte is not a valid standalone UTF-8 byte,
/// so it can never appear directly in a `&str`/`String` - this is why
/// dialogue built through `say`/`quiet_say`/etc., which round-trip through
/// `String`-typed `WorldAreaText`, has historically dropped the C source's
/// `COL_*` styling around keywords). These use Unicode Private-Use-Area
/// codepoints `U+E0C0..=U+E0D2` (never legal game text) mapped 1:1 to color
/// codes c0-c18; [`expand_color_sentinels`] expands them back to the
/// legacy `\xb0c<N>` byte sequence immediately before a message is
/// sanitized/enqueued (see `log_text::sanitize_log_bytes` callers), so the
/// raw byte only needs to exist right at the wire-serialization boundary.
pub const COL_STR_RESET: &str = "\u{E0C0}";
pub const COL_STR_WHITE: &str = COL_STR_RESET;
pub const COL_STR_DARK_GRAY: &str = "\u{E0C1}";
pub const COL_STR_LIGHT_GREEN: &str = "\u{E0C2}";
pub const COL_STR_LIGHT_RED: &str = "\u{E0C3}";
pub const COL_STR_LIGHT_BLUE: &str = "\u{E0C4}";
pub const COL_STR_LINK: &str = COL_STR_LIGHT_BLUE;
pub const COL_STR_ORANGE: &str = "\u{E0C5}";
pub const COL_STR_YELLOW: &str = "\u{E0C6}";
pub const COL_STR_VIOLET: &str = "\u{E0C7}";
pub const COL_STR_LIGHT_VIOLET: &str = "\u{E0C8}";
pub const COL_STR_TAN: &str = "\u{E0C9}";
pub const COL_STR_KHAKI: &str = COL_STR_TAN;
pub const COL_STR_MAUVE: &str = "\u{E0CA}";
pub const COL_STR_CYAN: &str = "\u{E0CB}";
pub const COL_STR_PEACH: &str = "\u{E0CC}";
pub const COL_STR_PINK: &str = "\u{E0CD}";
pub const COL_STR_AQUA: &str = "\u{E0CE}";
pub const COL_STR_LIME: &str = "\u{E0CF}";
pub const COL_STR_PURPLE: &str = "\u{E0D0}";
pub const COL_STR_HIDDEN_LINK: &str = "\u{E0D1}";
pub const COL_STR_LINK_RESET: &str = "\u{E0D2}";

pub const COL_STR_KEYWORD: &str = COL_STR_LIGHT_BLUE;
pub const COL_STR_ERROR: &str = COL_STR_LIGHT_RED;
pub const COL_STR_ANNOUNCE: &str = COL_STR_LIGHT_RED;
pub const COL_STR_SUCCESS: &str = COL_STR_LIGHT_GREEN;

const COL_STR_SENTINEL_BASE: u32 = 0xE0C0;
const COL_STR_SENTINEL_MAX: u32 = 0xE0D2;

/// Expands [`COL_STR_RESET`]-family sentinel codepoints in `s` back into
/// the raw legacy `\xb0c<N>` byte marker, byte-encoding everything else as
/// normal UTF-8. See the sentinel constants' doc comment for why this
/// indirection exists.
pub fn expand_color_sentinels(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut buf = [0u8; 4];
    for ch in s.chars() {
        let code = ch as u32;
        if (COL_STR_SENTINEL_BASE..=COL_STR_SENTINEL_MAX).contains(&code) {
            out.extend_from_slice(&runtime_color((code - COL_STR_SENTINEL_BASE) as u8));
        } else {
            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_marker_matches_legacy_protocol_byte() {
        assert_eq!(COLOR_MARKER, &[0xb0]);
        assert_eq!(COL_LIGHT_RED, &[0xb0, b'c', b'3']);
        assert_eq!(ITEMDESC_MARKER, &[0xb0, 0xb0, 0xb0]);
        assert_eq!(runtime_color(18), vec![0xb0, b'c', b'1', b'8']);
    }

    #[test]
    fn color_sentinels_expand_to_legacy_marker_bytes() {
        let text = format!("do you want me to {COL_STR_LIGHT_BLUE}repeat{COL_STR_RESET} it?");
        assert_eq!(
            expand_color_sentinels(&text),
            b"do you want me to \xb0c4repeat\xb0c0 it?".to_vec()
        );
    }

    #[test]
    fn color_sentinel_passthrough_leaves_plain_text_untouched() {
        assert_eq!(
            expand_color_sentinels("plain text, no markers"),
            b"plain text, no markers".to_vec()
        );
    }
}
