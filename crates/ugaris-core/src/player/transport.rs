use super::*;

impl PlayerRuntime {
    pub fn set_current_mirror(&mut self, mirror_id: u32) {
        self.current_mirror_id = mirror_id.min(u32::from(u16::MAX)) as u16;
    }

    pub fn touch_transport(&mut self, point: u8) -> bool {
        if point >= 64 {
            return false;
        }
        let bit = 1_u64 << point;
        let newly_seen = self.transport_seen & bit == 0;
        self.transport_seen |= bit;
        if newly_seen {
            self.update_transport_achievement_markers();
        }
        newly_seen
    }

    pub(crate) fn update_transport_achievement_markers(&mut self) {
        if (self.transport_seen & TRANSPORT_MAJOR_CITIES_MASK) == TRANSPORT_MAJOR_CITIES_MASK {
            self.achievements.traveller_of_astonia = true;
        }
        if (self.transport_seen & TRANSPORT_ALL_TELEPORTS_MASK) == TRANSPORT_ALL_TELEPORTS_MASK {
            self.achievements.explorer_of_astonia = true;
        }
        if (self.transport_seen & TRANSPORT_EARTH_UNDERGROUND_MASK)
            == TRANSPORT_EARTH_UNDERGROUND_MASK
        {
            self.achievements.underground_explorer = true;
        }
    }

    pub fn encode_legacy_transport_ppd(&self) -> Vec<u8> {
        self.transport_seen.to_le_bytes().to_vec()
    }

    pub fn decode_legacy_transport_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_TRANSPORT_PPD_SIZE {
            return false;
        }
        self.transport_seen = read_u64(bytes, 0);
        true
    }
}
