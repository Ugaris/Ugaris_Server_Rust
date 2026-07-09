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

    /// C `ppd->herald_talkstep` read (`lab2.c`'s `lab2_herald_driver`).
    pub fn legacy_lab2_herald_talkstep(&self) -> u8 {
        self.lab_ppd
            .get(LEGACY_LAB2_HERALD_TALKSTEP_OFFSET)
            .copied()
            .unwrap_or(0)
    }

    /// C `ppd->herald_talkstep = ...` write (`lab2.c`'s `lab2_herald_driver`).
    pub fn set_legacy_lab2_herald_talkstep(&mut self, value: u8) {
        if self.lab_ppd.len() <= LEGACY_LAB2_HERALD_TALKSTEP_OFFSET {
            self.lab_ppd
                .resize(LEGACY_LAB2_HERALD_TALKSTEP_OFFSET + 1, 0);
        }
        self.lab_ppd[LEGACY_LAB2_HERALD_TALKSTEP_OFFSET] = value;
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

    /// C `ppd->password1` (`src/system/lab.h`), an 8-byte nul-padded ASCII
    /// fragment. Trailing zero bytes are trimmed on read.
    pub fn legacy_lab3_password1(&self) -> Vec<u8> {
        legacy_lab3_password_field(&self.lab_ppd, LEGACY_LAB3_PASSWORD1_OFFSET)
    }

    /// C `sprintf(ppd->password1, password[ho])` write.
    pub fn set_legacy_lab3_password1(&mut self, value: &[u8]) {
        set_legacy_lab3_password_field(&mut self.lab_ppd, LEGACY_LAB3_PASSWORD1_OFFSET, value);
    }

    /// C `ppd->password2` (`src/system/lab.h`).
    pub fn legacy_lab3_password2(&self) -> Vec<u8> {
        legacy_lab3_password_field(&self.lab_ppd, LEGACY_LAB3_PASSWORD2_OFFSET)
    }

    /// C `sprintf(ppd->password2, password[ho + 1])` write.
    pub fn set_legacy_lab3_password2(&mut self, value: &[u8]) {
        set_legacy_lab3_password_field(&mut self.lab_ppd, LEGACY_LAB3_PASSWORD2_OFFSET, value);
    }

    /// C `sprintf(password, "%s%s", ppd->password1, ppd->password2)`
    /// (`lab3.c:261`): the guard's full expected password.
    pub fn legacy_lab3_full_password(&self) -> Vec<u8> {
        let mut password = self.legacy_lab3_password1();
        password.extend(self.legacy_lab3_password2());
        password
    }

    /// C `ppd->prisoner_talkstep` (`src/system/lab.h`).
    pub fn legacy_lab3_prisoner_talkstep(&self) -> u8 {
        self.lab_ppd
            .get(LEGACY_LAB3_PRISONER_TALKSTEP_OFFSET)
            .copied()
            .unwrap_or(0)
    }

    /// C `ppd->prisoner_talkstep = ...` write.
    pub fn set_legacy_lab3_prisoner_talkstep(&mut self, value: u8) {
        if self.lab_ppd.len() <= LEGACY_LAB3_PRISONER_TALKSTEP_OFFSET {
            self.lab_ppd
                .resize(LEGACY_LAB3_PRISONER_TALKSTEP_OFFSET + 1, 0);
        }
        self.lab_ppd[LEGACY_LAB3_PRISONER_TALKSTEP_OFFSET] = value;
    }

    /// C `ppd->guard_talkstep` (`src/system/lab.h`).
    pub fn legacy_lab3_guard_talkstep(&self) -> u8 {
        self.lab_ppd
            .get(LEGACY_LAB3_GUARD_TALKSTEP_OFFSET)
            .copied()
            .unwrap_or(0)
    }

    /// C `ppd->guard_talkstep = ...` write.
    pub fn set_legacy_lab3_guard_talkstep(&mut self, value: u8) {
        if self.lab_ppd.len() <= LEGACY_LAB3_GUARD_TALKSTEP_OFFSET {
            self.lab_ppd
                .resize(LEGACY_LAB3_GUARD_TALKSTEP_OFFSET + 1, 0);
        }
        self.lab_ppd[LEGACY_LAB3_GUARD_TALKSTEP_OFFSET] = value;
    }

    /// C `set_seyan_state` (`src/area/22/lab4.c:94-104`): recomputes
    /// `ppd->seyan4state` from the `seyan4got` bitfield (bit 0 = crown,
    /// bit 1 = szepter) after either bit changes. Writes both fields
    /// (`self.lab4_seyan_got` is assumed already updated by the caller).
    pub fn recompute_lab4_seyan_state(&mut self) {
        self.lab4_seyan_state = lab4_seyan_state_from_got(self.lab4_seyan_got);
    }
}

/// Pure half of C `set_seyan_state` (`src/area/22/lab4.c:94-104`), split
/// out so `world::npc::area22::lab4_seyan` can recompute the outcome
/// state without a `PlayerRuntime` handle (the driver only sees a
/// [`Lab4SeyanPlayerFacts`](crate::world::npc::area22::lab4_seyan::Lab4SeyanPlayerFacts)
/// snapshot).
pub fn lab4_seyan_state_from_got(got: u8) -> u8 {
    const GOT_CROWN: u8 = 1 << 0;
    const GOT_SZEPTER: u8 = 1 << 1;
    if got & GOT_CROWN != 0 && got & GOT_SZEPTER != 0 {
        30
    } else if got & GOT_CROWN != 0 {
        10
    } else if got & GOT_SZEPTER != 0 {
        20
    } else {
        0
    }
}

/// Pure half of C `set_seyan_state` (`src/area/22/lab5.c:263-271`), split
/// out so `world::npc::area22::lab5_seyan` can recompute the outcome
/// state without a `PlayerRuntime` handle, same precedent as
/// [`lab4_seyan_state_from_got`]. Unlike lab4's 2-bit crown/szepter
/// scheme, lab5 needs all three head bits set for the "done" state.
pub fn lab5_seyan_state_from_got(got: u8) -> u8 {
    const GOT_HEAD1: u8 = 1 << 0;
    const GOT_HEAD2: u8 = 1 << 1;
    const GOT_HEAD3: u8 = 1 << 2;
    if got & GOT_HEAD1 != 0 && got & GOT_HEAD2 != 0 && got & GOT_HEAD3 != 0 {
        20 // done
    } else if got != 0 {
        10 // some
    } else {
        0 // start
    }
}

/// Reads an 8-byte nul-padded ASCII field, trimming trailing zero bytes
/// (C's `char[8]` `sprintf`-written strings, always short enough to fit
/// with a nul terminator).
fn legacy_lab3_password_field(lab_ppd: &[u8], offset: usize) -> Vec<u8> {
    let end = (offset + LEGACY_LAB3_PASSWORD_FIELD_LEN).min(lab_ppd.len());
    if offset >= end {
        return Vec::new();
    }
    let field = &lab_ppd[offset..end];
    let len = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    field[..len].to_vec()
}

fn set_legacy_lab3_password_field(lab_ppd: &mut Vec<u8>, offset: usize, value: &[u8]) {
    let needed = offset + LEGACY_LAB3_PASSWORD_FIELD_LEN;
    if lab_ppd.len() < needed {
        lab_ppd.resize(needed, 0);
    }
    let copy_len = value.len().min(LEGACY_LAB3_PASSWORD_FIELD_LEN - 1);
    lab_ppd[offset..offset + LEGACY_LAB3_PASSWORD_FIELD_LEN].fill(0);
    lab_ppd[offset..offset + copy_len].copy_from_slice(&value[..copy_len]);
}
