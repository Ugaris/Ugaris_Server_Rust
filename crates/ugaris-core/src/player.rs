use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{entity::Item, ids::CharacterId, legacy::DIST_OLD};

pub const MAX_PLAYERS: usize = 512;
pub const OUTPUT_BUFFER_SIZE: usize = 16_384 * 2;
pub const MAX_SCROLLBACK: usize = 8192;
pub const MAX_PLAYER_EFFECTS: usize = 64;
pub const COMMAND_QUEUE_SIZE: usize = 16;
pub const KEYRING_MAX_KEYS: usize = 100;
pub const KEYRING_KEY_NAME_LEN: usize = 40;
pub const KEYRING_KEY_DESC_LEN: usize = 80;
pub const KEYRING_KEY_DRDATA_LEN: usize = 16;
pub const LEGACY_KEYRING_PPD_SIZE: usize = 15_912;
pub const RANDCHEST_MAX_ENTRIES: usize = 100;

const KEYRING_PPD_COUNT_OFFSET: usize = 0;
const KEYRING_PPD_KEYS_OFFSET: usize = 4;
const KEYRING_PPD_NAMES_OFFSET: usize = KEYRING_PPD_KEYS_OFFSET + KEYRING_MAX_KEYS * 4;
const KEYRING_PPD_DESCS_OFFSET: usize =
    KEYRING_PPD_NAMES_OFFSET + KEYRING_MAX_KEYS * KEYRING_KEY_NAME_LEN;
const KEYRING_PPD_SPRITES_OFFSET: usize =
    KEYRING_PPD_DESCS_OFFSET + KEYRING_MAX_KEYS * KEYRING_KEY_DESC_LEN;
const KEYRING_PPD_FLAGS_OFFSET: usize = KEYRING_PPD_SPRITES_OFFSET + KEYRING_MAX_KEYS * 4 + 4;
const KEYRING_PPD_VALUES_OFFSET: usize = KEYRING_PPD_FLAGS_OFFSET + KEYRING_MAX_KEYS * 8;
const KEYRING_PPD_DRIVERS_OFFSET: usize = KEYRING_PPD_VALUES_OFFSET + KEYRING_MAX_KEYS * 4;
const KEYRING_PPD_DRDATA_OFFSET: usize = KEYRING_PPD_DRIVERS_OFFSET + KEYRING_MAX_KEYS * 2;
const KEYRING_PPD_EXPIRE_OFFSET: usize =
    KEYRING_PPD_DRDATA_OFFSET + KEYRING_MAX_KEYS * KEYRING_KEY_DRDATA_LEN;
const KEYRING_PPD_AUTO_ADD_OFFSET: usize = KEYRING_PPD_EXPIRE_OFFSET + KEYRING_MAX_KEYS;

pub const DEFERRED_ACHIEVEMENTS: u32 = 1 << 0;
pub const DEFERRED_MOTD: u32 = 1 << 1;
pub const DEFERRED_AUCTION: u32 = 1 << 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PlayerConnectionState {
    Connect = 1,
    Normal = 2,
    Exit = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PlayerActionCode {
    Idle = 0,
    Move = 1,
    Take = 2,
    Drop = 3,
    Kill = 4,
    Use = 5,
    Bless = 6,
    Heal = 7,
    Freeze = 8,
    Fireball = 9,
    Ball = 10,
    MagicShield = 11,
    Flash = 12,
    Warcry = 13,
    LookMap = 14,
    Give = 15,
    FireballCharacter = 16,
    BallCharacter = 17,
    Teleport = 18,
    Pulse = 19,
    WalkDir = 20,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedAction {
    pub action: PlayerActionCode,
    pub arg1: i32,
    pub arg2: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyringEntry {
    pub template_id: u32,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub sprite: i32,
    #[serde(default)]
    pub flags: u64,
    #[serde(default)]
    pub value: u32,
    #[serde(default)]
    pub driver: u16,
    #[serde(default)]
    pub driver_data: Vec<u8>,
    #[serde(default)]
    pub expire_serial: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomChestAccess {
    pub location_id: u32,
    pub last_used_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyringAddResult {
    Added,
    Duplicate,
    Full,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AchievementState {
    pub chests_opened: u32,
    pub looter: bool,
    pub treasure_hunter: bool,
    pub treasure_master: bool,
    pub legendary_looter: bool,
    pub gold_looter: bool,
}

impl Default for QueuedAction {
    fn default() -> Self {
        Self {
            action: PlayerActionCode::Idle,
            arg1: 0,
            arg2: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRuntime {
    pub session_id: u64,
    pub state: PlayerConnectionState,
    pub client_version: u8,
    pub view_distance: usize,
    pub last_command_tick: u64,
    pub character_id: Option<CharacterId>,
    pub character_number: u32,
    pub command: Vec<u8>,
    pub action: QueuedAction,
    pub queue: VecDeque<QueuedAction>,
    pub client_ticker: u32,
    pub next_fightback_character: Option<CharacterId>,
    pub next_fightback_serial: u32,
    pub next_fightback_tick: u64,
    pub nofight_timer: u64,
    pub login_tick: u64,
    pub deferred_init: u32,
    pub scrollback: Vec<u8>,
    pub chest_last_access_seconds: HashMap<u8, u64>,
    pub keyring: Vec<KeyringEntry>,
    pub random_chests: Vec<RandomChestAccess>,
    pub achievements: AchievementState,
    #[serde(default)]
    pub keyring_auto_add: bool,
}

impl PlayerRuntime {
    pub fn connected(session_id: u64, current_tick: u64) -> Self {
        Self {
            session_id,
            state: PlayerConnectionState::Connect,
            client_version: 0,
            view_distance: DIST_OLD,
            last_command_tick: current_tick,
            character_id: None,
            character_number: 0,
            command: Vec::new(),
            action: QueuedAction::default(),
            queue: VecDeque::with_capacity(COMMAND_QUEUE_SIZE),
            client_ticker: 0,
            next_fightback_character: None,
            next_fightback_serial: 0,
            next_fightback_tick: 0,
            nofight_timer: 0,
            login_tick: current_tick,
            deferred_init: 0,
            scrollback: Vec::with_capacity(MAX_SCROLLBACK),
            chest_last_access_seconds: HashMap::new(),
            keyring: Vec::new(),
            random_chests: Vec::new(),
            achievements: AchievementState::default(),
            keyring_auto_add: false,
        }
    }

    pub fn chest_last_access_seconds(&self, treasure_index: u8) -> u64 {
        self.chest_last_access_seconds
            .get(&treasure_index)
            .copied()
            .unwrap_or_default()
    }

    pub fn mark_chest_access(&mut self, treasure_index: u8, realtime_seconds: u64) {
        self.chest_last_access_seconds
            .insert(treasure_index, realtime_seconds);
    }

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

    pub fn record_chest_opened(&mut self, treasure_index: u8) {
        self.achievements.chests_opened = self.achievements.chests_opened.saturating_add(1);
        if self.achievements.chests_opened >= 10 {
            self.achievements.looter = true;
        }
        if self.achievements.chests_opened >= 50 {
            self.achievements.treasure_hunter = true;
        }
        if self.achievements.chests_opened >= 100 {
            self.achievements.treasure_master = true;
        }
        if self.achievements.chests_opened >= 500 {
            self.achievements.legendary_looter = true;
        }
        if treasure_index == 63 {
            self.achievements.gold_looter = true;
        }
    }

    pub fn random_chest_last_used_seconds(&self, location_id: u32) -> Option<u64> {
        self.random_chests
            .iter()
            .find(|entry| entry.location_id == location_id)
            .map(|entry| entry.last_used_seconds)
    }

    pub fn mark_random_chest_used(&mut self, location_id: u32, realtime_seconds: u64) {
        if let Some(entry) = self
            .random_chests
            .iter_mut()
            .find(|entry| entry.location_id == location_id)
        {
            entry.last_used_seconds = realtime_seconds;
            return;
        }
        if self.random_chests.len() < RANDCHEST_MAX_ENTRIES {
            self.random_chests.push(RandomChestAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            });
            return;
        }
        if let Some(oldest) = self
            .random_chests
            .iter_mut()
            .min_by_key(|entry| entry.last_used_seconds)
        {
            *oldest = RandomChestAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            };
        }
    }

    pub fn mark_login_parsed(&mut self, client_version: Option<u8>, current_tick: u64) {
        self.client_version = client_version.unwrap_or_default();
        self.view_distance = if self.client_version >= 3 {
            40
        } else {
            DIST_OLD
        };
        self.login_tick = current_tick;
    }

    pub fn set_pending_action(&mut self, action: QueuedAction) {
        self.action = action;
    }

    pub fn push_queued_action(&mut self, action: QueuedAction) {
        if self.queue.len() == COMMAND_QUEUE_SIZE {
            self.queue.pop_front();
        }
        self.queue.push_back(action);
    }

    pub fn driver_stop(&mut self, current_tick: u64, nofight: bool) {
        self.queue.clear();
        self.action = QueuedAction::default();
        self.next_fightback_character = None;
        self.next_fightback_serial = 0;
        self.next_fightback_tick = 0;
        if nofight {
            self.nofight_timer = current_tick;
        }
    }

    pub fn driver_halt(&mut self) {
        self.action = QueuedAction::default();
        self.next_fightback_character = None;
        self.next_fightback_serial = 0;
        self.next_fightback_tick = 0;
    }

    pub fn driver_move(&mut self, x: i32, y: i32) {
        self.action = QueuedAction {
            action: PlayerActionCode::Move,
            arg1: x,
            arg2: y,
        };
    }

    pub fn driver_take(&mut self, item: i32, serial: u32) {
        self.action = QueuedAction {
            action: PlayerActionCode::Take,
            arg1: item,
            arg2: serial as i32,
        };
    }

    pub fn driver_drop(&mut self, x: i32, y: i32) {
        self.action = QueuedAction {
            action: PlayerActionCode::Drop,
            arg1: x,
            arg2: y,
        };
    }

    pub fn driver_use(&mut self, item: i32, serial: u32) {
        self.action = QueuedAction {
            action: PlayerActionCode::Use,
            arg1: item,
            arg2: serial as i32,
        };
    }

    pub fn driver_teleport(&mut self, teleport: i32) {
        self.action = QueuedAction {
            action: PlayerActionCode::Teleport,
            arg1: teleport,
            arg2: 0,
        };
    }

    pub fn driver_kill(&mut self, character: CharacterId, serial: u32) {
        self.action = QueuedAction {
            action: PlayerActionCode::Kill,
            arg1: character.0 as i32,
            arg2: serial as i32,
        };
    }

    pub fn driver_give(&mut self, character: CharacterId, serial: u32) {
        self.action = QueuedAction {
            action: PlayerActionCode::Give,
            arg1: character.0 as i32,
            arg2: serial as i32,
        };
    }

    pub fn driver_charspell(
        &mut self,
        spell: PlayerActionCode,
        character: CharacterId,
        serial: u32,
    ) {
        self.insert_driver_queue(QueuedAction {
            action: spell,
            arg1: character.0 as i32,
            arg2: serial as i32,
        });
    }

    pub fn driver_mapspell(&mut self, spell: PlayerActionCode, x: i32, y: i32) {
        self.insert_driver_queue(QueuedAction {
            action: spell,
            arg1: x,
            arg2: y,
        });
    }

    pub fn driver_selfspell(&mut self, spell: PlayerActionCode) {
        self.insert_driver_queue(QueuedAction {
            action: spell,
            arg1: 0,
            arg2: 0,
        });
    }

    fn insert_driver_queue(&mut self, action: QueuedAction) {
        if self.queue.len() == COMMAND_QUEUE_SIZE {
            if let Some(back) = self.queue.back_mut() {
                *back = action;
            }
        } else {
            self.queue.push_back(action);
        }
    }
}

fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64(bytes: &mut [u8], offset: usize, value: u64) {
    bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn write_c_string(bytes: &mut [u8], offset: usize, len: usize, value: &str) {
    let max_len = len.saturating_sub(1);
    let value_bytes = value.as_bytes();
    let copy_len = value_bytes.len().min(max_len);
    bytes[offset..offset + copy_len].copy_from_slice(&value_bytes[..copy_len]);
}

fn read_i32(bytes: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}

fn read_c_string(bytes: &[u8], offset: usize, len: usize) -> String {
    let raw = &bytes[offset..offset + len];
    let end = raw.iter().position(|byte| *byte == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use crate::{
        entity::{ItemFlags, MAX_MODIFIERS},
        ids::ItemId,
    };

    use super::*;

    #[test]
    fn player_constants_match_c_header() {
        assert_eq!(MAX_PLAYERS, 512);
        assert_eq!(PlayerConnectionState::Connect as u8, 1);
        assert_eq!(PlayerConnectionState::Normal as u8, 2);
        assert_eq!(PlayerConnectionState::Exit as u8, 3);
        assert_eq!(PlayerActionCode::WalkDir as u8, 20);
        assert_eq!(MAX_PLAYER_EFFECTS, 64);
    }

    #[test]
    fn command_queue_keeps_legacy_capacity() {
        let mut player = PlayerRuntime::connected(1, 0);
        for n in 0..20 {
            player.push_queued_action(QueuedAction {
                action: PlayerActionCode::Move,
                arg1: n,
                arg2: 0,
            });
        }
        assert_eq!(player.queue.len(), COMMAND_QUEUE_SIZE);
        assert_eq!(player.queue.front().unwrap().arg1, 4);
    }

    #[test]
    fn keyring_tracks_legacy_key_ids_with_duplicate_and_capacity_rules() {
        let mut player = PlayerRuntime::connected(1, 0);

        assert_eq!(
            player.add_keyring_key(0x1122_3344, "Copper Key"),
            KeyringAddResult::Added
        );
        assert_eq!(player.keyring_key_name(0x1122_3344), Some("Copper Key"));
        assert_eq!(
            player.add_keyring_key(0x1122_3344, "Duplicate"),
            KeyringAddResult::Duplicate
        );

        for index in 1..KEYRING_MAX_KEYS {
            assert_eq!(
                player.add_keyring_key(index as u32, format!("Key {index}")),
                KeyringAddResult::Added
            );
        }
        assert_eq!(
            player.add_keyring_key(0x5566_7788, "Overflow"),
            KeyringAddResult::Full
        );
    }

    #[test]
    fn keyring_item_storage_keeps_legacy_recreation_metadata() {
        let mut player = PlayerRuntime::connected(1, 0);
        let item = Item {
            id: ItemId(7),
            name: "Copper Key".into(),
            description: "Opens a copper lock".into(),
            flags: ItemFlags::USED | ItemFlags::TAKE | ItemFlags::QUEST,
            sprite: 1234,
            value: 55,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0x1122_3344,
            owner_id: 0,
            modifier_index: [0; MAX_MODIFIERS],
            modifier_value: [0; MAX_MODIFIERS],
            x: 0,
            y: 0,
            carried_by: None,
            contained_in: None,
            content_id: 0,
            driver: 77,
            driver_data: (0..32).collect(),
            serial: 9,
        };

        assert_eq!(player.add_keyring_item(&item), KeyringAddResult::Added);

        let stored = &player.keyring[0];
        assert_eq!(stored.template_id, 0x1122_3344);
        assert_eq!(stored.name, "Copper Key");
        assert_eq!(stored.description, "Opens a copper lock");
        assert_eq!(stored.sprite, 1234);
        assert_eq!(stored.flags, item.flags.bits());
        assert_eq!(stored.value, 55);
        assert_eq!(stored.driver, 77);
        assert_eq!(stored.driver_data, (0..16).collect::<Vec<_>>());
        assert_eq!(stored.expire_serial, 9);
    }

    #[test]
    fn keyring_auto_add_setting_round_trips() {
        let mut player = PlayerRuntime::connected(1, 0);
        assert!(!player.keyring_auto_add());
        player.set_keyring_auto_add(true);
        assert!(player.keyring_auto_add());
    }

    #[test]
    fn keyring_ppd_codec_matches_legacy_c_layout() {
        assert_eq!(KEYRING_PPD_FLAGS_OFFSET % 8, 0);
        assert_eq!(KEYRING_PPD_AUTO_ADD_OFFSET + 4, LEGACY_KEYRING_PPD_SIZE);

        let mut player = PlayerRuntime::connected(1, 0);
        player.set_keyring_auto_add(true);
        assert_eq!(
            player.add_keyring_entry(KeyringEntry {
                template_id: 0x1122_3344,
                name: "A name that is deliberately longer than forty bytes".to_string(),
                description: "Opens a door and has a long legacy description".to_string(),
                sprite: -123,
                flags: 0x0102_0304_0506_0708,
                value: 99,
                driver: 77,
                driver_data: (0..32).collect(),
                expire_serial: 0x1234,
            }),
            KeyringAddResult::Added
        );

        let bytes = player.encode_legacy_keyring_ppd();
        assert_eq!(bytes.len(), LEGACY_KEYRING_PPD_SIZE);
        assert_eq!(read_i32(&bytes, KEYRING_PPD_COUNT_OFFSET), 1);
        assert_eq!(read_u32(&bytes, KEYRING_PPD_KEYS_OFFSET), 0x1122_3344);
        assert_eq!(
            bytes[KEYRING_PPD_NAMES_OFFSET + KEYRING_KEY_NAME_LEN - 1],
            0
        );
        assert_eq!(read_i32(&bytes, KEYRING_PPD_AUTO_ADD_OFFSET), 1);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_keyring_ppd(&bytes));
        assert!(decoded.keyring_auto_add());
        assert_eq!(decoded.keyring.len(), 1);
        assert_eq!(decoded.keyring[0].template_id, 0x1122_3344);
        assert_eq!(
            decoded.keyring[0].name,
            "A name that is deliberately longer than"
        );
        assert_eq!(decoded.keyring[0].sprite, -123);
        assert_eq!(decoded.keyring[0].flags, 0x0102_0304_0506_0708);
        assert_eq!(decoded.keyring[0].driver_data, (0..16).collect::<Vec<_>>());
        assert_eq!(decoded.keyring[0].expire_serial, 0x34);
    }

    #[test]
    fn keyring_display_lines_match_legacy_shape_and_remove_by_position() {
        let mut player = PlayerRuntime::connected(1, 0);

        assert_eq!(
            player.keyring_display_lines(),
            vec!["Your keyring is empty."]
        );
        assert_eq!(
            player.add_keyring_key(0x1122_3344, "Copper Key"),
            KeyringAddResult::Added
        );
        assert_eq!(
            player.add_keyring_key(0x5566_7788, "Silver Key"),
            KeyringAddResult::Added
        );

        assert_eq!(
            player.keyring_display_lines(),
            vec![
                "=== Keyring (2/100 keys) ===",
                " 1. Copper Key",
                " 2. Silver Key",
                "Use a key on the keyring to add it.",
                "Type '#keyring remove <number>' to remove a key.",
                "Type '#keyring addall' to add all keys from inventory.",
            ]
        );
        assert_eq!(
            player.remove_keyring_key_at(0).map(|key| key.name),
            Some("Copper Key".to_string())
        );
        assert_eq!(player.keyring_key_name(0x1122_3344), None);
        assert_eq!(player.keyring_key_name(0x5566_7788), Some("Silver Key"));
        assert_eq!(player.remove_keyring_key_at(99), None);
    }

    #[test]
    fn chest_achievement_state_tracks_legacy_threshold_hooks() {
        let mut player = PlayerRuntime::connected(1, 0);

        for _ in 0..9 {
            player.record_chest_opened(1);
        }
        assert_eq!(player.achievements.chests_opened, 9);
        assert!(!player.achievements.looter);

        player.record_chest_opened(1);
        assert!(player.achievements.looter);
        assert!(!player.achievements.treasure_hunter);

        for _ in 10..50 {
            player.record_chest_opened(1);
        }
        assert!(player.achievements.treasure_hunter);
        assert!(!player.achievements.treasure_master);

        for _ in 50..100 {
            player.record_chest_opened(1);
        }
        assert!(player.achievements.treasure_master);
        assert!(!player.achievements.legendary_looter);

        for _ in 100..500 {
            player.record_chest_opened(1);
        }
        assert!(player.achievements.legendary_looter);

        player.record_chest_opened(63);
        assert!(player.achievements.gold_looter);
    }

    #[test]
    fn random_chest_access_tracks_hundred_recent_locations() {
        let mut player = PlayerRuntime::connected(1, 0);

        player.mark_random_chest_used(7, 100);
        assert_eq!(player.random_chest_last_used_seconds(7), Some(100));
        player.mark_random_chest_used(7, 200);
        assert_eq!(player.random_chest_last_used_seconds(7), Some(200));

        for index in 1..RANDCHEST_MAX_ENTRIES {
            player.mark_random_chest_used(1_000 + index as u32, index as u64);
        }
        assert_eq!(player.random_chests.len(), RANDCHEST_MAX_ENTRIES);
        player.mark_random_chest_used(9_999, 300);
        assert_eq!(player.random_chests.len(), RANDCHEST_MAX_ENTRIES);
        assert_eq!(player.random_chest_last_used_seconds(9_999), Some(300));
    }

    #[test]
    fn driver_stop_clears_action_queue_and_fightback_state() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.driver_move(10, 11);
        player.driver_selfspell(PlayerActionCode::Bless);
        player.next_fightback_character = Some(CharacterId(2));
        player.next_fightback_serial = 44;
        player.next_fightback_tick = 55;

        player.driver_stop(99, true);

        assert_eq!(player.action.action, PlayerActionCode::Idle);
        assert!(player.queue.is_empty());
        assert_eq!(player.next_fightback_character, None);
        assert_eq!(player.next_fightback_serial, 0);
        assert_eq!(player.next_fightback_tick, 0);
        assert_eq!(player.nofight_timer, 99);
    }

    #[test]
    fn driver_setters_match_c_action_payloads() {
        let mut player = PlayerRuntime::connected(1, 0);

        player.driver_take(7, 1234);
        assert_eq!(player.action.action, PlayerActionCode::Take);
        assert_eq!((player.action.arg1, player.action.arg2), (7, 1234));

        player.driver_kill(CharacterId(9), 4321);
        assert_eq!(player.action.action, PlayerActionCode::Kill);
        assert_eq!((player.action.arg1, player.action.arg2), (9, 4321));

        player.driver_drop(12, 13);
        assert_eq!(player.action.action, PlayerActionCode::Drop);
        assert_eq!((player.action.arg1, player.action.arg2), (12, 13));
    }

    #[test]
    fn driver_spell_queue_overwrites_last_slot_when_full() {
        let mut player = PlayerRuntime::connected(1, 0);
        for n in 0..COMMAND_QUEUE_SIZE {
            player.driver_mapspell(PlayerActionCode::Fireball, n as i32, 0);
        }

        player.driver_selfspell(PlayerActionCode::Bless);

        assert_eq!(player.queue.len(), COMMAND_QUEUE_SIZE);
        assert_eq!(player.queue.front().unwrap().arg1, 0);
        assert_eq!(player.queue.back().unwrap().action, PlayerActionCode::Bless);
    }
}
