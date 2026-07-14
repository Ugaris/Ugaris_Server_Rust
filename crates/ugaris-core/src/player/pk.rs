use super::*;

impl PlayerRuntime {
    pub fn encode_legacy_pk_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_PK_PPD_SIZE];
        write_i32(
            &mut bytes,
            PK_PPD_KILLS_OFFSET,
            self.pk_kills.min(i32::MAX as u32) as i32,
        );
        write_i32(
            &mut bytes,
            PK_PPD_DEATHS_OFFSET,
            self.pk_deaths.min(i32::MAX as u32) as i32,
        );
        write_i32(
            &mut bytes,
            PK_PPD_LAST_KILL_OFFSET,
            self.pk_last_kill.min(i32::MAX as u32) as i32,
        );
        write_i32(
            &mut bytes,
            PK_PPD_LAST_DEATH_OFFSET,
            self.pk_last_death.min(i32::MAX as u32) as i32,
        );
        for (index, character_id) in self
            .pk_hate
            .iter()
            .copied()
            .take(PK_HATE_MAX_ENTRIES)
            .enumerate()
        {
            write_i32(
                &mut bytes,
                PK_PPD_HATE_OFFSET + index * 4,
                character_id.min(i32::MAX as u32) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_pk_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_PK_PPD_SIZE {
            return false;
        }

        self.pk_kills = read_i32(bytes, PK_PPD_KILLS_OFFSET).max(0) as u32;
        self.pk_deaths = read_i32(bytes, PK_PPD_DEATHS_OFFSET).max(0) as u32;
        self.pk_last_kill = read_i32(bytes, PK_PPD_LAST_KILL_OFFSET).max(0) as u32;
        self.pk_last_death = read_i32(bytes, PK_PPD_LAST_DEATH_OFFSET).max(0) as u32;
        self.pk_hate.clear();
        for index in 0..PK_HATE_MAX_ENTRIES {
            let character_id = read_i32(bytes, PK_PPD_HATE_OFFSET + index * 4);
            self.pk_hate.push(character_id.max(0) as u32);
        }
        Self::trim_pk_hate_slots(&mut self.pk_hate);
        true
    }

    pub(crate) fn trim_pk_hate_slots(slots: &mut Vec<u32>) {
        while slots.last().copied() == Some(0) {
            slots.pop();
        }
    }

    pub fn has_any_pk_hate(&self) -> bool {
        self.pk_hate.iter().any(|hate_id| *hate_id != 0)
    }

    pub fn active_pk_hate_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.pk_hate.iter().copied().filter(|hate_id| *hate_id != 0)
    }

    pub fn has_pk_hate_for(&self, character_id: u32) -> bool {
        character_id != 0 && self.pk_hate.contains(&character_id)
    }

    pub fn add_pk_hate(&mut self, character_id: u32) -> bool {
        if character_id == 0 {
            return false;
        }

        let mut slots = [0_u32; PK_HATE_MAX_ENTRIES];
        for (index, hate_id) in self
            .pk_hate
            .iter()
            .copied()
            .take(PK_HATE_MAX_ENTRIES)
            .enumerate()
        {
            slots[index] = hate_id;
        }

        let position = (0..PK_HATE_MAX_ENTRIES - 1).find(|index| slots[*index] == character_id);
        let newly_added = position.is_none();
        let shift_count = position.unwrap_or(PK_HATE_MAX_ENTRIES - 1);
        for index in (1..=shift_count).rev() {
            slots[index] = slots[index - 1];
        }
        slots[0] = character_id;

        self.pk_hate = slots.to_vec();
        Self::trim_pk_hate_slots(&mut self.pk_hate);
        newly_added
    }

    pub fn add_pk_hate_from_hit(
        &mut self,
        character: &mut Character,
        attacker_character_id: u32,
    ) -> bool {
        let newly_added = self.add_pk_hate(attacker_character_id);
        if attacker_character_id != 0 {
            character.flags.remove(CharacterFlags::LAG);
        }
        newly_added
    }

    pub fn add_pk_kill(&mut self, realtime_seconds: u64) {
        self.pk_kills = self.pk_kills.saturating_add(1);
        self.pk_last_kill = realtime_seconds.min(i32::MAX as u64) as u32;
    }

    /// C `add_pk_steal` (`src/system/tool.c:894-908`), called from `/steal`
    /// (`cmd_steal`, `prof.c:226`) after a successful theft. A genuine C
    /// quirk: unlike [`Self::add_pk_kill`] this does **not** increment
    /// `pk_kills` - it only bumps `pk_last_kill` (`ppd->last_kill =
    /// realtime;`), reusing the kill timestamp field for steal events too.
    /// C gates the whole thing on `ch[cn].flags & (CF_PLAYER|CF_PK)`
    /// inside `add_pk_steal` itself; callers here are expected to check
    /// that on the `Character` first (matching the `add_pk_kill`/
    /// `add_pk_death` convention of gating at the call site, see
    /// `ugaris-server`'s `world_events.rs`).
    pub fn add_pk_steal(&mut self, realtime_seconds: u64) {
        self.pk_last_kill = realtime_seconds.min(i32::MAX as u64) as u32;
    }

    pub fn add_pk_death(&mut self, realtime_seconds: u64) {
        self.pk_deaths = self.pk_deaths.saturating_add(1);
        self.pk_last_death = realtime_seconds.min(i32::MAX as u64) as u32;
    }

    pub fn remove_pk_hate(&mut self, character_id: u32) -> bool {
        let Some(position) = self
            .pk_hate
            .iter()
            .position(|hate_id| *hate_id == character_id)
        else {
            return false;
        };
        self.pk_hate[position] = 0;
        Self::trim_pk_hate_slots(&mut self.pk_hate);
        true
    }
}
