//! Weather system mod packets (`SV_MOD2`, `SV_VIS_WEATHER` subtype).
//!
//! Byte layout copied from the sibling `Ugaris_Protocol` repo's
//! `include/ugaris/protocol/mod_weather.h` (the legacy `mod_packet.h`
//! header this codebase's own `ugaris-protocol` crate stands in for; that
//! header isn't part of the `Ugaris_Server` C source tree, but the C
//! server (`src/module/weather/weather_client.c`'s `mod_send_weather`)
//! builds this exact struct and sends it with `psend`). All fields are
//! little-endian, `#pragma pack(push, 1)` (no padding).

use crate::packet::SV_MOD2;

/// `mod_weather.h`'s `SV_VisualSubtype` enum entry used by the weather
/// system (the other entry, `SV_VIS_SFX = 0x02`, belongs to the
/// lightning-strike/screen-flash effects, unported - see `PORTING_TODO.md`).
pub const SV_VIS_WEATHER: u8 = 0x01;

/// `mod_weather.h`'s `MOD_WEATHER_EFFECT_*` bitfield (client-visible bits
/// only; the C header's higher server-internal bits `0x40`/`0x80`
/// (`WEATHER_EFFECT_LIGHTNING`/`WEATHER_EFFECT_COMBAT`/
/// `WEATHER_EFFECT_ELEMENTAL` from `weather.h`) are never sent to the
/// client and have no representation here).
pub const MOD_WEATHER_EFFECT_SLOW: u8 = 0x01;
pub const MOD_WEATHER_EFFECT_BLIND: u8 = 0x02;
pub const MOD_WEATHER_EFFECT_DAMAGE: u8 = 0x04;
pub const MOD_WEATHER_EFFECT_SLIP: u8 = 0x08;
pub const MOD_WEATHER_EFFECT_SKILL: u8 = 0x10;
/// Set per-player by `broadcast_weather_packet` (`weather_client.c:113`)
/// for indoor players: "tells the client to suppress visual/audio
/// effects" while still delivering the current weather state for UI.
pub const MOD_WEATHER_EFFECT_INDOOR: u8 = 0x20;

/// `mod_weather.h`'s `struct sv_weather_packet` size (8 bytes: `type(1) +
/// len(1) + subtype(1) + weather(1) + intensity(1) + transition(1) +
/// day_night(1) + effects(1)`).
pub const SV_WEATHER_PACKET_SIZE: usize = 8;

/// C `mod_send_weather`/`struct sv_weather_packet`: builds the 8-byte
/// `SV_MOD2`/`SV_VIS_WEATHER` packet. `len` is `PACKET_LEN` =
/// `sizeof(pkt) - 2` = 6, matching every other mod-packet builder in this
/// crate (`mod_achievements.rs`'s `ach_unlock`).
pub fn sv_weather_packet(
    weather: u8,
    intensity: u8,
    transition: u8,
    day_night: u8,
    effects: u8,
) -> [u8; SV_WEATHER_PACKET_SIZE] {
    [
        SV_MOD2,
        (SV_WEATHER_PACKET_SIZE - 2) as u8,
        SV_VIS_WEATHER,
        weather,
        intensity,
        transition,
        day_night,
        effects,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sv_weather_packet_matches_legacy_wire_layout() {
        let packet = sv_weather_packet(2, 3, 128, 64, 0x0F);
        assert_eq!(packet.len(), SV_WEATHER_PACKET_SIZE);
        assert_eq!(packet[0], SV_MOD2);
        assert_eq!(packet[1], 6);
        assert_eq!(packet[2], SV_VIS_WEATHER);
        assert_eq!(packet[3], 2);
        assert_eq!(packet[4], 3);
        assert_eq!(packet[5], 128);
        assert_eq!(packet[6], 64);
        assert_eq!(packet[7], 0x0F);
    }
}
