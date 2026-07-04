//! Steam achievement mod packets (`SV_MOD3`, subtype range `0x30-0x3F`).
//!
//! Byte layouts copied from the sibling `Ugaris_Protocol` repo's
//! `include/ugaris/protocol/mod_achievements.h` (the legacy `mod_packet.h`
//! header this codebase's own `ugaris-protocol` crate stands in for; that
//! header isn't part of the `Ugaris_Server` C source tree, but the C server
//! (`src/module/achievements/achievement.c:1291-1415`) builds these exact
//! structs and sends them with `psend`). All fields are little-endian,
//! `#pragma pack(push, 1)` (no padding).

use bytes::{BufMut, BytesMut};

use crate::packet::SV_MOD3;

/// `mod_achievements.h`'s `SV_AchievementSubtype` enum.
pub const SV_ACH_UNLOCK: u8 = 0x30;
pub const SV_ACH_PROGRESS: u8 = 0x31;
pub const SV_ACH_SYNC: u8 = 0x32;
pub const SV_ACH_STATS: u8 = 0x33;

/// `mod_achievements.h`'s `ACHIEVEMENT_MAX_STEAM_ID`: max Steam achievement
/// ID string length (including the C null terminator budget).
pub const ACHIEVEMENT_MAX_STEAM_ID: usize = 40;
/// `mod_achievements.h`'s `ACHIEVEMENT_MAX_PER_SYNC`: max achievement
/// entries batched into a single `SV_ACH_SYNC` packet.
pub const ACHIEVEMENT_MAX_PER_SYNC: usize = 16;

/// C `struct sv_ach_unlock_packet` (51 bytes total, `len` field = 49):
/// `type(1) + len(1) + subtype(1) + achievement_id(1) + category(1) +
/// steam_api_name(40) + timestamp(4) + show_notification(1) + reserved(1)`.
pub const SV_ACH_UNLOCK_SIZE: usize = 51;
/// C `struct ach_sync_entry` (56 bytes): `achievement_id(1) + category(1) +
/// unlocked(1) + has_progress(1) + steam_api_name(40) + timestamp(4) +
/// progress_current(4) + progress_target(4)`.
pub const ACH_SYNC_ENTRY_SIZE: usize = 56;
/// C `struct sv_ach_sync_packet` header (5 bytes): `type(1) + len(1) +
/// subtype(1) + count(1) + is_final(1)`.
pub const SV_ACH_SYNC_HEADER_SIZE: usize = 5;

fn write_fixed_c_string(dst: &mut [u8], value: &str) {
    dst.fill(0);
    let bytes = value.as_bytes();
    let len = bytes.len().min(dst.len().saturating_sub(1));
    dst[..len].copy_from_slice(&bytes[..len]);
}

/// C `achievement_send_to_client` (`achievement.c:1291-1324`): builds a
/// single `SV_ACH_UNLOCK` packet for one newly-unlocked achievement.
/// `timestamp` is the C `(uint32_t)ach->timestamp` truncation of the
/// `time_t` unlock time.
pub fn ach_unlock(
    achievement_id: u8,
    category: u8,
    steam_api_name: &str,
    timestamp: u32,
    show_notification: bool,
) -> [u8; SV_ACH_UNLOCK_SIZE] {
    let mut out = [0u8; SV_ACH_UNLOCK_SIZE];
    out[0] = SV_MOD3;
    out[1] = (SV_ACH_UNLOCK_SIZE - 2) as u8; // PACKET_LEN: sizeof(pkt) - 2
    out[2] = SV_ACH_UNLOCK;
    out[3] = achievement_id;
    out[4] = category;
    write_fixed_c_string(&mut out[5..5 + ACHIEVEMENT_MAX_STEAM_ID], steam_api_name);
    let ts_off = 5 + ACHIEVEMENT_MAX_STEAM_ID;
    out[ts_off..ts_off + 4].copy_from_slice(&timestamp.to_le_bytes());
    out[ts_off + 4] = show_notification as u8;
    // out[ts_off + 5] (`reserved`) stays 0.
    out
}

/// C `struct ach_sync_entry` (`achievement.c:1355-1376`): one achievement's
/// state in a bulk `SV_ACH_SYNC` batch.
#[derive(Debug, Clone)]
pub struct AchSyncEntry {
    pub achievement_id: u8,
    pub category: u8,
    pub unlocked: bool,
    pub has_progress: bool,
    pub steam_api_name: String,
    pub timestamp: u32,
    pub progress_current: u32,
    pub progress_target: u32,
}

impl AchSyncEntry {
    fn write(&self, out: &mut [u8]) {
        out[0] = self.achievement_id;
        out[1] = self.category;
        out[2] = self.unlocked as u8;
        out[3] = self.has_progress as u8;
        write_fixed_c_string(
            &mut out[4..4 + ACHIEVEMENT_MAX_STEAM_ID],
            &self.steam_api_name,
        );
        let ts_off = 4 + ACHIEVEMENT_MAX_STEAM_ID;
        out[ts_off..ts_off + 4].copy_from_slice(&self.timestamp.to_le_bytes());
        out[ts_off + 4..ts_off + 8].copy_from_slice(&self.progress_current.to_le_bytes());
        out[ts_off + 8..ts_off + 12].copy_from_slice(&self.progress_target.to_le_bytes());
    }
}

/// C `achievement_sync_all`'s per-batch send (`achievement.c:1379-1405`,
/// plus the `count == 0`/`total_sent > 0` empty-final-packet edge case at
/// `achievement.c:1406-1414`, reproduced by simply calling this with an
/// empty `entries` slice). `entries.len()` must be `<=
/// ACHIEVEMENT_MAX_PER_SYNC`; callers are responsible for batching.
pub fn ach_sync_batch(entries: &[AchSyncEntry], is_final: bool) -> BytesMut {
    let count = entries.len();
    let payload_len = SV_ACH_SYNC_HEADER_SIZE - 2 + count * ACH_SYNC_ENTRY_SIZE;
    let mut out = BytesMut::with_capacity(SV_ACH_SYNC_HEADER_SIZE + count * ACH_SYNC_ENTRY_SIZE);
    out.put_u8(SV_MOD3);
    out.put_u8(payload_len as u8);
    out.put_u8(SV_ACH_SYNC);
    out.put_u8(count as u8);
    out.put_u8(is_final as u8);
    for entry in entries {
        let mut buf = [0u8; ACH_SYNC_ENTRY_SIZE];
        entry.write(&mut buf);
        out.extend_from_slice(&buf);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ach_unlock_packet_matches_c_sv_ach_unlock_packet_layout() {
        let pkt = ach_unlock(5, 2, "DEMON_SLAYER", 0x1234_5678, true);
        assert_eq!(pkt.len(), 51);
        assert_eq!(pkt[0], 60); // SV_MOD3
        assert_eq!(pkt[1], 49); // PACKET_LEN
        assert_eq!(pkt[2], 0x30); // SV_ACH_UNLOCK
        assert_eq!(pkt[3], 5); // achievement_id
        assert_eq!(pkt[4], 2); // category
        let mut expected_name = [0u8; ACHIEVEMENT_MAX_STEAM_ID];
        expected_name[..12].copy_from_slice(b"DEMON_SLAYER");
        assert_eq!(&pkt[5..45], &expected_name[..]);
        assert_eq!(&pkt[45..49], &0x1234_5678u32.to_le_bytes());
        assert_eq!(pkt[49], 1); // show_notification
        assert_eq!(pkt[50], 0); // reserved
    }

    #[test]
    fn ach_unlock_truncates_overlong_steam_id() {
        let long_name = "A".repeat(60);
        let pkt = ach_unlock(0, 0, &long_name, 0, false);
        // 39 'A's + a null terminator byte fill the 40-byte field.
        assert_eq!(&pkt[5..44], &b"A".repeat(39)[..]);
        assert_eq!(pkt[44], 0);
    }

    #[test]
    fn ach_sync_batch_header_and_entry_sizes_match_c() {
        let entries = vec![AchSyncEntry {
            achievement_id: 1,
            category: 0,
            unlocked: true,
            has_progress: false,
            steam_api_name: "STARTED_UGARIS".to_string(),
            timestamp: 42,
            progress_current: 0,
            progress_target: 0,
        }];
        let batch = ach_sync_batch(&entries, true);
        assert_eq!(batch.len(), SV_ACH_SYNC_HEADER_SIZE + ACH_SYNC_ENTRY_SIZE);
        assert_eq!(batch[0], 60); // SV_MOD3
        assert_eq!(batch[1], (3 + ACH_SYNC_ENTRY_SIZE) as u8);
        assert_eq!(batch[2], 0x32); // SV_ACH_SYNC
        assert_eq!(batch[3], 1); // count
        assert_eq!(batch[4], 1); // is_final
        let entry = &batch[5..];
        assert_eq!(entry[0], 1); // achievement_id
        assert_eq!(entry[2], 1); // unlocked
        assert_eq!(entry[3], 0); // has_progress
        let mut expected_name = [0u8; ACHIEVEMENT_MAX_STEAM_ID];
        expected_name[..14].copy_from_slice(b"STARTED_UGARIS");
        assert_eq!(&entry[4..44], &expected_name[..]);
        let ts_off = 4 + ACHIEVEMENT_MAX_STEAM_ID;
        assert_eq!(&entry[ts_off..ts_off + 4], &42u32.to_le_bytes());
    }

    #[test]
    fn ach_sync_batch_empty_final_matches_c_bare_header() {
        let batch = ach_sync_batch(&[], true);
        assert_eq!(batch.len(), SV_ACH_SYNC_HEADER_SIZE);
        assert_eq!(batch[3], 0); // count
        assert_eq!(batch[4], 1); // is_final
    }
}
