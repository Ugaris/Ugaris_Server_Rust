use super::*;

impl PlayerRuntime {
    pub fn encode_legacy_lab_ppd(&self) -> Vec<u8> {
        let mut bytes = if self.lab_ppd.len() >= LEGACY_LAB_PPD_SIZE {
            self.lab_ppd.clone()
        } else {
            let mut bytes = vec![0; LEGACY_LAB_PPD_SIZE];
            let copy_len = self.lab_ppd.len().min(LEGACY_LAB_PPD_SIZE);
            bytes[..copy_len].copy_from_slice(&self.lab_ppd[..copy_len]);
            bytes
        };
        write_u64(&mut bytes, 0, self.lab_solved_bits);
        bytes
    }

    pub fn decode_legacy_lab_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < 8 {
            return false;
        }
        self.lab_ppd = bytes.to_vec();
        self.lab_solved_bits = read_u64(bytes, 0);
        true
    }

    pub fn ensure_legacy_lab2_described_graves(&mut self) -> [u8; 4] {
        self.ensure_legacy_lab2_described_graves_with_indices([0, 4, 8, 9])
    }

    pub fn ensure_legacy_lab2_described_graves_with_indices(
        &mut self,
        indices: [u8; 4],
    ) -> [u8; 4] {
        if self.lab_ppd.len() < LEGACY_LAB_PPD_SIZE {
            self.lab_ppd.resize(LEGACY_LAB_PPD_SIZE, 0);
        }
        if self.lab_ppd[LEGACY_LAB2_GRAVEVERSION_OFFSET] != LEGACY_LAB2_GRAVE_VERSION {
            self.lab_ppd[LEGACY_LAB2_GRAVEVERSION_OFFSET] = LEGACY_LAB2_GRAVE_VERSION;
            self.lab_ppd[LEGACY_LAB2_GRAVEINDEX_OFFSET..LEGACY_LAB2_GRAVEINDEX_OFFSET + 4]
                .copy_from_slice(&indices);
        }
        self.legacy_lab2_grave_indices()
    }

    pub fn legacy_lab2_grave_indices(&self) -> [u8; 4] {
        if self.lab_ppd.len() < LEGACY_LAB2_GRAVEINDEX_OFFSET + 4 {
            return [0, 0, 0, 0];
        }
        let mut indices = [0u8; 4];
        indices.copy_from_slice(
            &self.lab_ppd[LEGACY_LAB2_GRAVEINDEX_OFFSET..LEGACY_LAB2_GRAVEINDEX_OFFSET + 4],
        );
        indices
    }

    pub fn legacy_lab2_grave_clue_text(&mut self, book: u8) -> Option<String> {
        let indices = self.ensure_legacy_lab2_described_graves();
        let (slot, name) = match book {
            1 => (0, "Henry"),
            2 => (1, "Eldrick"),
            3 => (2, "John"),
            4 => (3, "Mariah"),
            _ => return None,
        };
        let description = LAB2_DESCRIBED_GRAVES
            .get(indices[slot] as usize)
            .map(|(_, description)| *description)
            .unwrap_or("%s is buried in an unknown grave.");
        Some(description.replace("%s", name))
    }

    pub fn legacy_lab2_special_grave_kind_at(&mut self, x: u16, y: u16) -> Option<u8> {
        let indices = self.ensure_legacy_lab2_described_graves();
        indices.into_iter().enumerate().find_map(|(slot, index)| {
            let ((grave_x, grave_y), _) = *LAB2_DESCRIBED_GRAVES.get(index as usize)?;
            (grave_x == x && grave_y == y).then_some(slot as u8 + 1)
        })
    }

    pub fn legacy_lab2_grave_cleared(&self, grave_number: usize) -> bool {
        let byte = grave_number / 8;
        let bit = grave_number % 8;
        self.lab2_grave_bits
            .get(byte)
            .is_some_and(|value| value & (1 << bit) != 0)
    }

    pub fn mark_legacy_lab2_grave_cleared(&mut self, grave_number: usize) -> bool {
        let byte = grave_number / 8;
        let bit = grave_number % 8;
        if byte >= LAB2_GRAVE_BITSET_BYTES {
            return false;
        }
        if self.lab2_grave_bits.len() <= byte {
            self.lab2_grave_bits.resize(byte + 1, 0);
        }
        let was_cleared = self.lab2_grave_bits[byte] & (1 << bit) != 0;
        self.lab2_grave_bits[byte] |= 1 << bit;
        !was_cleared
    }
}
