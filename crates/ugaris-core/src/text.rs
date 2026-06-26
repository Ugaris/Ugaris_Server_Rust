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
}
