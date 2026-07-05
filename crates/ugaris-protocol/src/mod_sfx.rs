//! Special-effects (SFX) mod packets (`SV_MOD2`, `SV_VIS_SFX` subtype).
//!
//! Byte layout copied from the sibling `Ugaris_Protocol` repo's
//! `include/ugaris/protocol/mod_sfx.h` (the legacy `mod_packet.h` header
//! this codebase's own `ugaris-protocol` crate stands in for; the C server
//! call sites are `src/common/mod_packet.c`'s `mod_send_sfx`/
//! `mod_broadcast_sfx`/`mod_send_screen_sfx`/`mod_broadcast_screen_sfx`,
//! consumed today by `src/module/weather/weather_client.c`'s
//! `broadcast_weather_thunder_effect`). All fields are little-endian,
//! `#pragma pack(push, 1)` (no padding).

use crate::packet::SV_MOD2;

/// `mod_weather.h`'s `SV_VisualSubtype` enum entry for SFX packets (the
/// other entry, `SV_VIS_WEATHER = 0x01`, belongs to `mod_weather.rs`).
pub const SV_VIS_SFX: u8 = 0x02;

/// `mod_sfx.h`'s `ModSFXType` enum. Only `SFX_LIGHTNING_STRIKE` and
/// `SFX_SCREEN_FLASH` have a real C call site today
/// (`weather_client.c`'s `broadcast_weather_thunder_effect`) - the rest
/// are ported for header parity ahead of any future SFX feature.
pub const SFX_NONE: u8 = 0;
pub const SFX_SCREEN_FLASH: u8 = 1;
pub const SFX_SCREEN_SHAKE: u8 = 2;
pub const SFX_SCREEN_TINT: u8 = 3;
pub const SFX_EXPLOSION: u8 = 10;
pub const SFX_IMPACT: u8 = 11;
pub const SFX_SPARKLE: u8 = 12;
pub const SFX_SMOKE: u8 = 13;
pub const SFX_LIGHTNING_STRIKE: u8 = 20;
pub const SFX_DUST_CLOUD: u8 = 21;
pub const SFX_WATER_SPLASH: u8 = 22;

/// `mod_sfx.h`'s `SFX_POS_SCREEN`: use for `x`/`y` to indicate a
/// screen-wide effect rather than a world position.
pub const SFX_POS_SCREEN: u16 = 0xFFFF;

/// `mod_sfx.h`'s named RGB565 colors. Only `SFX_COLOR_DEFAULT` (use the
/// effect's own default color) and `SFX_COLOR_WHITE` have a real call
/// site today.
pub const SFX_COLOR_DEFAULT: u16 = 0x0000;
pub const SFX_COLOR_WHITE: u16 = 0xFFFF;
pub const SFX_COLOR_RED: u16 = 0xF800;
pub const SFX_COLOR_GREEN: u16 = 0x07E0;
pub const SFX_COLOR_BLUE: u16 = 0x001F;
pub const SFX_COLOR_YELLOW: u16 = 0xFFE0;
pub const SFX_COLOR_ORANGE: u16 = 0xFC00;
pub const SFX_COLOR_CYAN: u16 = 0x07FF;
pub const SFX_COLOR_PURPLE: u16 = 0xF81F;

/// `mod_sfx.h`'s `struct sv_sfx_packet` size (12 bytes: `type(1) + len(1)
/// + subtype(1) + sfx_type(1) + x(2) + y(2) + intensity(1) + duration(1)
/// + color(2)`).
pub const SV_SFX_PACKET_SIZE: usize = 12;

/// C `mod_send_sfx`/`mod_broadcast_sfx`/`struct sv_sfx_packet`: builds the
/// 12-byte `SV_MOD2`/`SV_VIS_SFX` packet. `len` is `PACKET_LEN` =
/// `sizeof(pkt) - 2` = 10, matching every other mod-packet builder in
/// this crate (`mod_weather.rs`'s `sv_weather_packet`).
pub fn sv_sfx_packet(
    sfx_type: u8,
    x: u16,
    y: u16,
    intensity: u8,
    duration: u8,
    color: u16,
) -> [u8; SV_SFX_PACKET_SIZE] {
    let mut packet = [0u8; SV_SFX_PACKET_SIZE];
    packet[0] = SV_MOD2;
    packet[1] = (SV_SFX_PACKET_SIZE - 2) as u8;
    packet[2] = SV_VIS_SFX;
    packet[3] = sfx_type;
    packet[4..6].copy_from_slice(&x.to_le_bytes());
    packet[6..8].copy_from_slice(&y.to_le_bytes());
    packet[8] = intensity;
    packet[9] = duration;
    packet[10..12].copy_from_slice(&color.to_le_bytes());
    packet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sv_sfx_packet_matches_legacy_wire_layout() {
        let packet = sv_sfx_packet(SFX_LIGHTNING_STRIKE, 100, 200, 255, 8, SFX_COLOR_DEFAULT);
        assert_eq!(packet.len(), SV_SFX_PACKET_SIZE);
        assert_eq!(packet[0], SV_MOD2);
        assert_eq!(packet[1], 10);
        assert_eq!(packet[2], SV_VIS_SFX);
        assert_eq!(packet[3], SFX_LIGHTNING_STRIKE);
        assert_eq!(u16::from_le_bytes([packet[4], packet[5]]), 100);
        assert_eq!(u16::from_le_bytes([packet[6], packet[7]]), 200);
        assert_eq!(packet[8], 255);
        assert_eq!(packet[9], 8);
        assert_eq!(u16::from_le_bytes([packet[10], packet[11]]), 0);
    }

    #[test]
    fn sv_sfx_packet_screen_wide_uses_pos_screen_sentinel() {
        let packet = sv_sfx_packet(
            SFX_SCREEN_FLASH,
            SFX_POS_SCREEN,
            SFX_POS_SCREEN,
            200,
            0,
            SFX_COLOR_WHITE,
        );
        assert_eq!(u16::from_le_bytes([packet[4], packet[5]]), SFX_POS_SCREEN);
        assert_eq!(u16::from_le_bytes([packet[6], packet[7]]), SFX_POS_SCREEN);
        assert_eq!(
            u16::from_le_bytes([packet[10], packet[11]]),
            SFX_COLOR_WHITE
        );
    }
}
