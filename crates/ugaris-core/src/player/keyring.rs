use super::*;

impl PlayerRuntime {
    pub fn encode_legacy_keyring_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_KEYRING_PPD_SIZE];
        let count = self.keyring.len().min(KEYRING_MAX_KEYS);
        write_i32(&mut bytes, KEYRING_PPD_COUNT_OFFSET, count as i32);

        for (index, key) in self.keyring.iter().take(KEYRING_MAX_KEYS).enumerate() {
            write_u32(
                &mut bytes,
                KEYRING_PPD_KEYS_OFFSET + index * 4,
                key.template_id,
            );
            write_c_string(
                &mut bytes,
                KEYRING_PPD_NAMES_OFFSET + index * KEYRING_KEY_NAME_LEN,
                KEYRING_KEY_NAME_LEN,
                &key.name,
            );
            write_c_string(
                &mut bytes,
                KEYRING_PPD_DESCS_OFFSET + index * KEYRING_KEY_DESC_LEN,
                KEYRING_KEY_DESC_LEN,
                &key.description,
            );
            write_i32(
                &mut bytes,
                KEYRING_PPD_SPRITES_OFFSET + index * 4,
                key.sprite,
            );
            write_u64(&mut bytes, KEYRING_PPD_FLAGS_OFFSET + index * 8, key.flags);
            write_u32(&mut bytes, KEYRING_PPD_VALUES_OFFSET + index * 4, key.value);
            write_u16(
                &mut bytes,
                KEYRING_PPD_DRIVERS_OFFSET + index * 2,
                key.driver,
            );

            let drdata_offset = KEYRING_PPD_DRDATA_OFFSET + index * KEYRING_KEY_DRDATA_LEN;
            let drdata_len = key.driver_data.len().min(KEYRING_KEY_DRDATA_LEN);
            bytes[drdata_offset..drdata_offset + drdata_len]
                .copy_from_slice(&key.driver_data[..drdata_len]);
            bytes[KEYRING_PPD_EXPIRE_OFFSET + index] = key.expire_serial as u8;
        }

        write_i32(
            &mut bytes,
            KEYRING_PPD_AUTO_ADD_OFFSET,
            i32::from(self.keyring_auto_add),
        );
        bytes
    }

    pub fn decode_legacy_keyring_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_KEYRING_PPD_SIZE {
            return false;
        }

        let raw_count = read_i32(bytes, KEYRING_PPD_COUNT_OFFSET);
        let count = raw_count.clamp(0, KEYRING_MAX_KEYS as i32) as usize;
        let mut keyring = Vec::with_capacity(count);
        for index in 0..count {
            let driver_data_offset = KEYRING_PPD_DRDATA_OFFSET + index * KEYRING_KEY_DRDATA_LEN;
            keyring.push(KeyringEntry {
                template_id: read_u32(bytes, KEYRING_PPD_KEYS_OFFSET + index * 4),
                name: read_c_string(
                    bytes,
                    KEYRING_PPD_NAMES_OFFSET + index * KEYRING_KEY_NAME_LEN,
                    KEYRING_KEY_NAME_LEN,
                ),
                description: read_c_string(
                    bytes,
                    KEYRING_PPD_DESCS_OFFSET + index * KEYRING_KEY_DESC_LEN,
                    KEYRING_KEY_DESC_LEN,
                ),
                sprite: read_i32(bytes, KEYRING_PPD_SPRITES_OFFSET + index * 4),
                flags: read_u64(bytes, KEYRING_PPD_FLAGS_OFFSET + index * 8),
                value: read_u32(bytes, KEYRING_PPD_VALUES_OFFSET + index * 4),
                driver: read_u16(bytes, KEYRING_PPD_DRIVERS_OFFSET + index * 2),
                driver_data: bytes[driver_data_offset..driver_data_offset + KEYRING_KEY_DRDATA_LEN]
                    .to_vec(),
                expire_serial: u32::from(bytes[KEYRING_PPD_EXPIRE_OFFSET + index]),
            });
        }

        self.keyring = keyring;
        self.keyring_auto_add = read_i32(bytes, KEYRING_PPD_AUTO_ADD_OFFSET) != 0;
        true
    }

    pub fn add_keyring_key(
        &mut self,
        template_id: u32,
        name: impl Into<String>,
    ) -> KeyringAddResult {
        self.add_keyring_entry(KeyringEntry {
            template_id,
            name: name.into(),
            description: String::new(),
            sprite: 0,
            flags: 0,
            value: 0,
            driver: 0,
            driver_data: Vec::new(),
            expire_serial: 0,
        })
    }

    pub fn add_keyring_item(&mut self, item: &Item) -> KeyringAddResult {
        let driver_data_len = item.driver_data.len().min(KEYRING_KEY_DRDATA_LEN);
        self.add_keyring_entry(KeyringEntry {
            template_id: item.template_id,
            name: item.name.clone(),
            description: item.description.clone(),
            sprite: item.sprite,
            flags: item.flags.bits(),
            value: item.value,
            driver: item.driver,
            driver_data: item.driver_data[..driver_data_len].to_vec(),
            expire_serial: item.serial,
        })
    }

    pub fn add_keyring_entry(&mut self, entry: KeyringEntry) -> KeyringAddResult {
        if self
            .keyring
            .iter()
            .any(|key| key.template_id == entry.template_id)
        {
            return KeyringAddResult::Duplicate;
        }
        if self.keyring.len() >= KEYRING_MAX_KEYS {
            return KeyringAddResult::Full;
        }
        self.keyring.push(entry);
        KeyringAddResult::Added
    }

    pub fn keyring_auto_add(&self) -> bool {
        self.keyring_auto_add
    }

    pub fn set_keyring_auto_add(&mut self, enabled: bool) {
        self.keyring_auto_add = enabled;
    }

    pub fn keyring_key_name(&self, template_id: u32) -> Option<&str> {
        self.keyring
            .iter()
            .find(|key| key.template_id == template_id)
            .map(|key| key.name.as_str())
    }

    pub fn remove_keyring_key_at(&mut self, index: usize) -> Option<KeyringEntry> {
        if index >= self.keyring.len() {
            return None;
        }
        Some(self.keyring.remove(index))
    }

    pub fn keyring_display_lines(&self) -> Vec<String> {
        if self.keyring.is_empty() {
            return vec!["Your keyring is empty.".to_string()];
        }

        let mut lines = Vec::with_capacity(self.keyring.len() + 3);
        lines.push(format!(
            "=== Keyring ({}/{KEYRING_MAX_KEYS} keys) ===",
            self.keyring.len()
        ));
        for (index, key) in self.keyring.iter().enumerate() {
            if key.name.is_empty() {
                lines.push(format!(
                    " {}. Unknown Key (ID: {})",
                    index + 1,
                    key.template_id
                ));
            } else {
                lines.push(format!(" {}. {}", index + 1, key.name));
            }
        }
        lines.push("Use a key on the keyring to add it.".to_string());
        lines.push("Type '#keyring remove <number>' to remove a key.".to_string());
        lines.push("Type '#keyring addall' to add all keys from inventory.".to_string());
        lines
    }
}
