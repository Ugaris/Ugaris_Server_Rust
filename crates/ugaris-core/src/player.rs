use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{
    entity::{Character, CharacterFlags, CharacterValue, Item},
    ids::CharacterId,
    legacy::DIST_OLD,
    tell::TellData,
};

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
pub const TREASURE_CHEST_PPD_ENTRIES: usize = 200;
pub const LEGACY_TREASURE_CHEST_PPD_SIZE: usize = TREASURE_CHEST_PPD_ENTRIES * 4;
pub const LEGACY_TRANSPORT_PPD_SIZE: usize = 8;
pub const RANDCHEST_MAX_ENTRIES: usize = 100;
pub const LEGACY_RANDCHEST_PPD_SIZE: usize = RANDCHEST_MAX_ENTRIES * 4 * 2;
pub const ORBSPAWN_MAX_ENTRIES: usize = 100;
pub const LEGACY_ORBSPAWN_PPD_SIZE: usize = ORBSPAWN_MAX_ENTRIES * 4 * 2;
pub const FLOWER_MAX_ENTRIES: usize = 100;
pub const LEGACY_FLOWER_PPD_SIZE: usize = FLOWER_MAX_ENTRIES * 4 * 2;
pub const DEMONSHRINE_MAX_ENTRIES: usize = 100;
pub const LEGACY_DEMONSHRINE_PPD_SIZE: usize = DEMONSHRINE_MAX_ENTRIES * 4;
pub const TREASURE_DIG_PPD_ENTRIES: usize = 5;
pub const LEGACY_TREASURE_DIG_PPD_SIZE: usize = TREASURE_DIG_PPD_ENTRIES * 4;
pub const LEGACY_MISC_PPD_SIZE: usize = 36;
pub const LEGACY_AREA3_PPD_SIZE: usize = 17 * 4;
pub const LEGACY_LOSTCON_PPD_SIZE: usize = 19 * 4;
pub const RUNE_USED_WORDS: usize = 1024 / 32;
pub const RUNE_SPECIAL_EXEC_COUNT: usize = 25;
pub const LEGACY_RUNE_PPD_SIZE: usize = RUNE_USED_WORDS * 4 + RUNE_SPECIAL_EXEC_COUNT * 4;
pub const PK_HATE_MAX_ENTRIES: usize = 50;
pub const LEGACY_PK_PPD_SIZE: usize = 4 * 4 + PK_HATE_MAX_ENTRIES * 4;
pub const IGNORE_MAX_ENTRIES: usize = 100;
pub const LEGACY_IGNORE_PPD_SIZE: usize = IGNORE_MAX_ENTRIES * 4;
pub const SWEAR_SENTENCE_COUNT: usize = 10;
pub const SWEAR_SENTENCE_LEN: usize = 80;
pub const LEGACY_SWEAR_PPD_SIZE: usize =
    10 * 4 + 4 + SWEAR_SENTENCE_COUNT * SWEAR_SENTENCE_LEN + 10 * 4 + 10 * 4 + 4 + 4;
pub const ALIAS_MAX_ENTRIES: usize = 32;
pub const ALIAS_FROM_LEN: usize = 8;
pub const ALIAS_TO_LEN: usize = 56;
pub const LEGACY_ALIAS_PPD_SIZE: usize = ALIAS_MAX_ENTRIES * (ALIAS_FROM_LEN + ALIAS_TO_LEN);
pub const PERSISTENT_PLAYER_DATA: u32 = 1 << 31;
pub const PERSISTENT_SUBSCRIBER_DATA: u32 = 1 << 30;
pub const DEV_ID_DB: u32 = 1;
pub const DEV_ID_ED: u32 = 59;
pub const DRD_JUNK_PPD: u32 = make_drd(DEV_ID_DB, 114 | PERSISTENT_PLAYER_DATA);
pub const DRD_AREA3_PPD: u32 = make_drd(DEV_ID_DB, 40 | PERSISTENT_PLAYER_DATA);
pub const DRD_TREASURE_CHEST_PPD: u32 = make_drd(DEV_ID_DB, 17 | PERSISTENT_PLAYER_DATA);
pub const DRD_TRANSPORT_PPD: u32 = make_drd(DEV_ID_DB, 44 | PERSISTENT_PLAYER_DATA);
pub const DRD_PK_PPD: u32 = make_drd(DEV_ID_DB, 47 | PERSISTENT_PLAYER_DATA);
pub const TRANSPORT_MAJOR_CITIES_MASK: u64 = 0x03E0_0205;
pub const TRANSPORT_ALL_TELEPORTS_MASK: u64 = 0x03F3_F7FF;
pub const TRANSPORT_EARTH_UNDERGROUND_MASK: u64 = 0x01F8;
pub const DRD_RANDCHEST_PPD: u32 = make_drd(DEV_ID_DB, 63 | PERSISTENT_PLAYER_DATA);
pub const DRD_DEMONSHRINE_PPD: u32 = make_drd(DEV_ID_DB, 68 | PERSISTENT_PLAYER_DATA);
pub const DRD_ORBSPAWN_PPD: u32 = make_drd(DEV_ID_DB, 105 | PERSISTENT_PLAYER_DATA);
pub const DRD_LOSTCON_PPD: u32 = make_drd(DEV_ID_DB, 91 | PERSISTENT_PLAYER_DATA);
pub const DRD_FLOWER_PPD: u32 = make_drd(DEV_ID_DB, 62 | PERSISTENT_PLAYER_DATA);
pub const DRD_MISC_PPD: u32 = make_drd(DEV_ID_DB, 113 | PERSISTENT_PLAYER_DATA);
pub const DRD_ALIAS_PPD: u32 = make_drd(DEV_ID_DB, 80 | PERSISTENT_PLAYER_DATA);
pub const DRD_IGNORE_PPD: u32 = make_drd(DEV_ID_DB, 100 | PERSISTENT_PLAYER_DATA);
pub const DRD_SWEAR_PPD: u32 = make_drd(DEV_ID_DB, 109 | PERSISTENT_PLAYER_DATA);
pub const DRD_TREASURE_DIG_PPD: u32 = make_drd(DEV_ID_ED, 5 | PERSISTENT_PLAYER_DATA);
pub const DRD_KEYRING_PPD: u32 = make_drd(DEV_ID_ED, 7 | PERSISTENT_PLAYER_DATA);
pub const DRD_RUNE_PPD: u32 = make_drd(DEV_ID_DB, 108 | PERSISTENT_PLAYER_DATA);
pub const SPECIAL_SHRINE_HCSC_CUTOFF_SECONDS: u64 = 1_411_941_600;
pub const SPECIAL_SHRINE_CONFIRM_WINDOW_SECONDS: u64 = 10;

pub const fn make_drd(dev_id: u32, nr: u32) -> u32 {
    (dev_id << 24) | nr
}

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
const RANDCHEST_PPD_IDS_OFFSET: usize = 0;
const RANDCHEST_PPD_LAST_USED_OFFSET: usize = RANDCHEST_PPD_IDS_OFFSET + RANDCHEST_MAX_ENTRIES * 4;
const ORBSPAWN_PPD_IDS_OFFSET: usize = 0;
const ORBSPAWN_PPD_LAST_USED_OFFSET: usize = ORBSPAWN_PPD_IDS_OFFSET + ORBSPAWN_MAX_ENTRIES * 4;
const FLOWER_PPD_IDS_OFFSET: usize = 0;
const FLOWER_PPD_LAST_USED_OFFSET: usize = FLOWER_PPD_IDS_OFFSET + FLOWER_MAX_ENTRIES * 4;
const AREA3_PPD_KELLY_FOUND1_OFFSET: usize = 3 * 4;
const AREA3_PPD_KELLY_FOUND2_OFFSET: usize = 4 * 4;
const AREA3_PPD_KELLY_FOUND3_OFFSET: usize = 5 * 4;
const MISC_PPD_TREEDONE_OFFSET: usize = 24;
const MISC_PPD_GIFT_YEAR_OFFSET: usize = 32;
const LOSTCON_PPD_MAXLAG_OFFSET: usize = 17 * 4;
const PK_PPD_KILLS_OFFSET: usize = 0;
const PK_PPD_DEATHS_OFFSET: usize = 4;
const PK_PPD_LAST_KILL_OFFSET: usize = 8;
const PK_PPD_LAST_DEATH_OFFSET: usize = 12;
const PK_PPD_HATE_OFFSET: usize = 16;
const RUNE_PPD_SPECIAL_EXEC_OFFSET: usize = RUNE_USED_WORDS * 4;
const SWEAR_PPD_BANNED_TILL_OFFSET: usize = LEGACY_SWEAR_PPD_SIZE - 4;

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
pub struct OrbSpawnAccess {
    pub location_id: u32,
    pub last_used_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowerAccess {
    pub location_id: u32,
    pub last_used_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyringAddResult {
    Added,
    Duplicate,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpecialShrineResult {
    NothingHere,
    ConfirmRequired,
    HardcoreRemoved,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DemonShrineResult {
    Learned { exp_added: u32 },
    AlreadyKnown,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum XmasTreeResult {
    Dormant,
    AlreadyGranted,
    NeedsHolidayTreat,
    GiftGranted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoneHintResult {
    Hint {
        page: u16,
        rune: &'static str,
        position: &'static str,
    },
    Bug {
        level: u8,
        nr: u8,
        pos: u8,
        value: i32,
    },
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AchievementState {
    pub chests_opened: u32,
    pub looter: bool,
    pub treasure_hunter: bool,
    pub treasure_master: bool,
    pub legendary_looter: bool,
    pub gold_looter: bool,
    #[serde(default)]
    pub traveller_of_astonia: bool,
    #[serde(default)]
    pub explorer_of_astonia: bool,
    #[serde(default)]
    pub underground_explorer: bool,
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
    #[serde(default)]
    pub ppd_blob: Vec<u8>,
    #[serde(default)]
    pub subscriber_blob: Vec<u8>,
    pub chest_last_access_seconds: HashMap<u8, u64>,
    pub keyring: Vec<KeyringEntry>,
    pub random_chests: Vec<RandomChestAccess>,
    #[serde(default)]
    pub orb_spawns: Vec<OrbSpawnAccess>,
    #[serde(default)]
    pub flowers: Vec<FlowerAccess>,
    #[serde(default)]
    pub demonshrines: Vec<u32>,
    #[serde(default)]
    pub treasure_dig_last_seconds: [u64; TREASURE_DIG_PPD_ENTRIES],
    #[serde(default)]
    pub misc_ppd: Vec<u8>,
    #[serde(default)]
    pub area3_ppd: Vec<u8>,
    #[serde(default)]
    pub pk_kills: u32,
    #[serde(default)]
    pub pk_deaths: u32,
    #[serde(default)]
    pub pk_last_kill: u32,
    #[serde(default)]
    pub pk_last_death: u32,
    #[serde(default)]
    pub pk_hate: Vec<u32>,
    pub achievements: AchievementState,
    #[serde(default)]
    pub keyring_auto_add: bool,
    #[serde(default)]
    pub current_section_id: u16,
    #[serde(default)]
    pub special_shrine_hcsc_last_touch_seconds: u64,
    #[serde(default)]
    pub transport_seen: u64,
    #[serde(default)]
    pub current_mirror_id: u16,
    #[serde(default)]
    pub max_lag_seconds: u8,
    #[serde(default)]
    pub shutup_until_seconds: u64,
    #[serde(default)]
    pub swear_ppd: Vec<u8>,
    #[serde(default)]
    pub tell_data: TellData,
    #[serde(default)]
    pub ignored_characters: Vec<u32>,
    #[serde(default)]
    pub chat_channels: u32,
    #[serde(default)]
    pub rune_used_words: [u32; RUNE_USED_WORDS],
    #[serde(default)]
    pub rune_special_exec: [i32; RUNE_SPECIAL_EXEC_COUNT],
    #[serde(default)]
    pub aliases: Vec<CommandAlias>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandAlias {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgnoreToggleResult {
    Added,
    Removed,
    Full,
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
            ppd_blob: Vec::new(),
            subscriber_blob: Vec::new(),
            chest_last_access_seconds: HashMap::new(),
            keyring: Vec::new(),
            random_chests: Vec::new(),
            orb_spawns: Vec::new(),
            flowers: Vec::new(),
            demonshrines: Vec::new(),
            treasure_dig_last_seconds: [0; TREASURE_DIG_PPD_ENTRIES],
            misc_ppd: Vec::new(),
            area3_ppd: Vec::new(),
            pk_kills: 0,
            pk_deaths: 0,
            pk_last_kill: 0,
            pk_last_death: 0,
            pk_hate: Vec::new(),
            achievements: AchievementState::default(),
            keyring_auto_add: false,
            current_section_id: 0,
            special_shrine_hcsc_last_touch_seconds: 0,
            transport_seen: 0,
            current_mirror_id: 0,
            max_lag_seconds: 0,
            shutup_until_seconds: 0,
            swear_ppd: Vec::new(),
            tell_data: TellData::default(),
            ignored_characters: Vec::new(),
            chat_channels: 0,
            rune_used_words: [0; RUNE_USED_WORDS],
            rune_special_exec: [0; RUNE_SPECIAL_EXEC_COUNT],
            aliases: Vec::new(),
        }
    }

    pub fn encode_legacy_alias_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_ALIAS_PPD_SIZE];
        for (index, alias) in self.aliases.iter().take(ALIAS_MAX_ENTRIES).enumerate() {
            let offset = index * (ALIAS_FROM_LEN + ALIAS_TO_LEN);
            write_c_string(&mut bytes, offset, ALIAS_FROM_LEN, &alias.from);
            write_c_string(&mut bytes, offset + ALIAS_FROM_LEN, ALIAS_TO_LEN, &alias.to);
        }
        bytes
    }

    pub fn decode_legacy_alias_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_ALIAS_PPD_SIZE {
            return false;
        }
        self.aliases.clear();
        for index in 0..ALIAS_MAX_ENTRIES {
            let offset = index * (ALIAS_FROM_LEN + ALIAS_TO_LEN);
            let from = read_c_string(bytes, offset, ALIAS_FROM_LEN);
            if from.is_empty() {
                continue;
            }
            let to = read_c_string(bytes, offset + ALIAS_FROM_LEN, ALIAS_TO_LEN);
            self.aliases.push(CommandAlias { from, to });
        }
        true
    }

    pub fn expand_aliases(&self, source: &str) -> String {
        fn alias_stop(ch: char) -> bool {
            ch.is_whitespace() || (ch.is_ascii_punctuation() && ch != '\'')
        }

        let mut out = String::new();
        let mut token = String::new();
        for ch in source.chars() {
            if alias_stop(ch) {
                if token.is_empty() {
                    out.push(ch);
                    continue;
                }
                if let Some(alias) = self
                    .aliases
                    .iter()
                    .find(|alias| alias.from.eq_ignore_ascii_case(&token))
                {
                    out.push_str(&alias.to);
                } else {
                    out.push_str(&token);
                }
                token.clear();
                out.push(ch);
            } else {
                token.push(ch);
            }
            if out.len() > 198 {
                out.truncate(199);
                return out;
            }
        }
        if !token.is_empty() {
            if let Some(alias) = self
                .aliases
                .iter()
                .find(|alias| alias.from.eq_ignore_ascii_case(&token))
            {
                out.push_str(&alias.to);
            } else {
                out.push_str(&token);
            }
        }
        if out.len() > 199 {
            out.truncate(199);
        }
        out
    }

    pub fn ignores_character(&self, character_id: u32) -> bool {
        character_id != 0 && self.ignored_characters.contains(&character_id)
    }

    pub fn toggle_ignored_character(&mut self, character_id: u32) -> IgnoreToggleResult {
        if character_id == 0 {
            return IgnoreToggleResult::Full;
        }
        if let Some(index) = self
            .ignored_characters
            .iter()
            .position(|ignored| *ignored == character_id)
        {
            self.ignored_characters.remove(index);
            return IgnoreToggleResult::Removed;
        }
        if self.ignored_characters.len() >= IGNORE_MAX_ENTRIES {
            return IgnoreToggleResult::Full;
        }
        self.ignored_characters.push(character_id);
        IgnoreToggleResult::Added
    }

    pub fn clear_ignored_characters(&mut self) {
        self.ignored_characters.clear();
    }

    pub fn encode_legacy_ignore_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_IGNORE_PPD_SIZE];
        for (index, character_id) in self
            .ignored_characters
            .iter()
            .copied()
            .take(IGNORE_MAX_ENTRIES)
            .enumerate()
        {
            write_i32(
                &mut bytes,
                index * 4,
                character_id.min(i32::MAX as u32) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_ignore_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_IGNORE_PPD_SIZE {
            return false;
        }
        self.ignored_characters.clear();
        for index in 0..IGNORE_MAX_ENTRIES {
            let character_id = read_i32(bytes, index * 4);
            if character_id > 0 {
                self.ignored_characters.push(character_id as u32);
            }
        }
        true
    }

    pub fn ensure_rune_special_execs<F>(&mut self, mut random_below: F)
    where
        F: FnMut(u32) -> u32,
    {
        if self.rune_special_exec[0] != 0 {
            return;
        }

        const BADLIST: [i32; 15] = [555, 55, 5, 666, 66, 6, 777, 77, 7, 888, 88, 8, 999, 99, 9];
        for level in 5..10 {
            for offset in 0..5 {
                loop {
                    let value = random_below(level * 111) as i32;
                    if value < 100 || BADLIST.contains(&value) {
                        continue;
                    }
                    let base = (level - 5) as usize * 5;
                    if self.rune_special_exec[base..base + offset as usize].contains(&value) {
                        continue;
                    }
                    let digits = format!("{value:03}");
                    let level_digit = char::from_digit(level, 10).unwrap();
                    if digits.chars().any(|ch| ch == '0' || ch > level_digit) {
                        continue;
                    }
                    if !digits.chars().any(|ch| ch == level_digit) {
                        continue;
                    }
                    self.rune_special_exec[base + offset as usize] = value;
                    break;
                }
            }
        }
    }

    pub fn bone_hint<F>(&mut self, level: u8, nr: u8, pos: u8, random_below: F) -> BoneHintResult
    where
        F: FnMut(u32) -> u32,
    {
        self.ensure_rune_special_execs(random_below);
        let index = usize::from(level.saturating_sub(5)) * 5 + usize::from(nr);
        let value = self
            .rune_special_exec
            .get(index)
            .copied()
            .unwrap_or_default();
        let digits = value.to_string();
        let digit = digits
            .as_bytes()
            .get(usize::from(pos))
            .copied()
            .unwrap_or(b'0');
        let result = digit.saturating_sub(b'0');
        const RUNE_NAMES: [&str; 10] = [
            "none", "Ansuz", "Berkano", "Dagaz", "Ehwaz", "Fehu", "Hagalaz", "Isa", "Ingwaz",
            "Raidho",
        ];
        const POS_NAMES: [&str; 3] = ["first", "second", "third"];
        let Some(rune) = RUNE_NAMES.get(usize::from(result)).copied() else {
            return BoneHintResult::Bug {
                level,
                nr,
                pos,
                value,
            };
        };
        let Some(position) = POS_NAMES.get(usize::from(pos)).copied() else {
            return BoneHintResult::Bug {
                level,
                nr,
                pos,
                value,
            };
        };
        BoneHintResult::Hint {
            page: u16::from(level) * 10 + u16::from(nr),
            rune,
            position,
        }
    }

    pub fn encode_legacy_rune_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_RUNE_PPD_SIZE];
        for (index, word) in self.rune_used_words.iter().copied().enumerate() {
            write_u32(&mut bytes, index * 4, word);
        }
        for (index, value) in self.rune_special_exec.iter().copied().enumerate() {
            write_i32(&mut bytes, RUNE_PPD_SPECIAL_EXEC_OFFSET + index * 4, value);
        }
        bytes
    }

    pub fn decode_legacy_rune_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_RUNE_PPD_SIZE {
            return false;
        }
        for index in 0..RUNE_USED_WORDS {
            self.rune_used_words[index] = read_u32(bytes, index * 4);
        }
        for index in 0..RUNE_SPECIAL_EXEC_COUNT {
            self.rune_special_exec[index] =
                read_i32(bytes, RUNE_PPD_SPECIAL_EXEC_OFFSET + index * 4);
        }
        true
    }

    pub fn set_max_lag_seconds(&mut self, seconds: u8) {
        self.max_lag_seconds = seconds;
    }

    pub fn encode_legacy_lostcon_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_LOSTCON_PPD_SIZE];
        write_i32(
            &mut bytes,
            LOSTCON_PPD_MAXLAG_OFFSET,
            i32::from(self.max_lag_seconds),
        );
        bytes
    }

    pub fn decode_legacy_lostcon_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_LOSTCON_PPD_SIZE {
            return false;
        }
        self.max_lag_seconds =
            read_i32(bytes, LOSTCON_PPD_MAXLAG_OFFSET).clamp(0, i32::from(u8::MAX)) as u8;
        true
    }

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

    fn update_transport_achievement_markers(&mut self) {
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
            if character_id > 0 {
                self.pk_hate.push(character_id as u32);
            }
        }
        true
    }

    pub fn has_pk_hate_for(&self, character_id: u32) -> bool {
        character_id != 0 && self.pk_hate.iter().any(|hate_id| *hate_id == character_id)
    }

    pub fn add_pk_hate(&mut self, character_id: u32) -> bool {
        if character_id == 0 {
            return false;
        }

        let newly_added = if let Some(position) = self
            .pk_hate
            .iter()
            .position(|hate_id| *hate_id == character_id)
        {
            self.pk_hate.remove(position);
            false
        } else {
            true
        };

        self.pk_hate.insert(0, character_id);
        self.pk_hate.truncate(PK_HATE_MAX_ENTRIES);
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
        self.pk_hate.remove(position);
        true
    }

    pub fn touch_special_shrine(
        &mut self,
        character: &mut Character,
        kind: u8,
        realtime_seconds: u64,
    ) -> SpecialShrineResult {
        if kind != 0x0A {
            return SpecialShrineResult::Unsupported;
        }
        if !character.flags.contains(CharacterFlags::HARDCORE)
            || character.creation_time > SPECIAL_SHRINE_HCSC_CUTOFF_SECONDS
        {
            return SpecialShrineResult::NothingHere;
        }
        if self.special_shrine_hcsc_last_touch_seconds == 0
            || realtime_seconds.saturating_sub(self.special_shrine_hcsc_last_touch_seconds)
                > SPECIAL_SHRINE_CONFIRM_WINDOW_SECONDS
        {
            self.special_shrine_hcsc_last_touch_seconds = realtime_seconds;
            return SpecialShrineResult::ConfirmRequired;
        }

        character.flags.remove(CharacterFlags::HARDCORE);
        self.special_shrine_hcsc_last_touch_seconds = 0;
        SpecialShrineResult::HardcoreRemoved
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

    pub fn encode_legacy_treasure_chest_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_TREASURE_CHEST_PPD_SIZE];
        for (&treasure_index, &last_access_seconds) in &self.chest_last_access_seconds {
            let index = usize::from(treasure_index);
            if index >= TREASURE_CHEST_PPD_ENTRIES {
                continue;
            }
            write_i32(
                &mut bytes,
                index * 4,
                last_access_seconds.min(i32::MAX as u64) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_treasure_chest_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_TREASURE_CHEST_PPD_SIZE {
            return false;
        }

        self.chest_last_access_seconds.clear();
        for index in 0..TREASURE_CHEST_PPD_ENTRIES {
            let last_access_seconds = read_i32(bytes, index * 4);
            if last_access_seconds > 0 {
                self.chest_last_access_seconds
                    .insert(index as u8, last_access_seconds as u64);
            }
        }
        true
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

    pub fn encode_legacy_randchest_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_RANDCHEST_PPD_SIZE];
        for (index, entry) in self
            .random_chests
            .iter()
            .take(RANDCHEST_MAX_ENTRIES)
            .enumerate()
        {
            write_i32(
                &mut bytes,
                RANDCHEST_PPD_IDS_OFFSET + index * 4,
                entry.location_id.min(i32::MAX as u32) as i32,
            );
            write_i32(
                &mut bytes,
                RANDCHEST_PPD_LAST_USED_OFFSET + index * 4,
                entry.last_used_seconds.min(i32::MAX as u64) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_randchest_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_RANDCHEST_PPD_SIZE {
            return false;
        }

        self.random_chests.clear();
        for index in 0..RANDCHEST_MAX_ENTRIES {
            let location_id = read_i32(bytes, RANDCHEST_PPD_IDS_OFFSET + index * 4);
            let last_used_seconds = read_i32(bytes, RANDCHEST_PPD_LAST_USED_OFFSET + index * 4);
            if location_id > 0 && last_used_seconds > 0 {
                self.random_chests.push(RandomChestAccess {
                    location_id: location_id as u32,
                    last_used_seconds: last_used_seconds as u64,
                });
            }
        }
        true
    }

    pub fn encode_legacy_orbspawn_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_ORBSPAWN_PPD_SIZE];
        for (index, entry) in self
            .orb_spawns
            .iter()
            .take(ORBSPAWN_MAX_ENTRIES)
            .enumerate()
        {
            write_i32(
                &mut bytes,
                ORBSPAWN_PPD_IDS_OFFSET + index * 4,
                entry.location_id.min(i32::MAX as u32) as i32,
            );
            write_i32(
                &mut bytes,
                ORBSPAWN_PPD_LAST_USED_OFFSET + index * 4,
                entry.last_used_seconds.min(i32::MAX as u64) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_orbspawn_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_ORBSPAWN_PPD_SIZE {
            return false;
        }

        self.orb_spawns.clear();
        for index in 0..ORBSPAWN_MAX_ENTRIES {
            let location_id = read_i32(bytes, ORBSPAWN_PPD_IDS_OFFSET + index * 4);
            let last_used_seconds = read_i32(bytes, ORBSPAWN_PPD_LAST_USED_OFFSET + index * 4);
            if location_id > 0 && last_used_seconds > 0 {
                self.orb_spawns.push(OrbSpawnAccess {
                    location_id: location_id as u32,
                    last_used_seconds: last_used_seconds as u64,
                });
            }
        }
        true
    }

    pub fn encode_legacy_demonshrine_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_DEMONSHRINE_PPD_SIZE];
        for (index, location_id) in self
            .demonshrines
            .iter()
            .copied()
            .take(DEMONSHRINE_MAX_ENTRIES)
            .enumerate()
        {
            write_i32(
                &mut bytes,
                index * 4,
                location_id.min(i32::MAX as u32) as i32,
            );
        }
        bytes
    }

    pub fn encode_legacy_misc_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_MISC_PPD_SIZE];
        let copy_len = self.misc_ppd.len().min(LEGACY_MISC_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.misc_ppd[..copy_len]);
        bytes
    }

    pub fn decode_legacy_misc_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_MISC_PPD_SIZE {
            return false;
        }

        self.misc_ppd = bytes[..LEGACY_MISC_PPD_SIZE].to_vec();
        true
    }

    pub fn decode_legacy_demonshrine_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_DEMONSHRINE_PPD_SIZE {
            return false;
        }

        self.demonshrines.clear();
        for index in 0..DEMONSHRINE_MAX_ENTRIES {
            let location_id = read_i32(bytes, index * 4);
            if location_id > 0 {
                self.demonshrines.push(location_id as u32);
            }
        }
        true
    }

    pub fn treasure_dig_last_seconds(&self, dig_index: u8) -> u64 {
        self.treasure_dig_last_seconds
            .get(usize::from(dig_index))
            .copied()
            .unwrap_or_default()
    }

    pub fn mark_treasure_dig(&mut self, dig_index: u8, realtime_seconds: u64) -> bool {
        let Some(last_dig) = self
            .treasure_dig_last_seconds
            .get_mut(usize::from(dig_index))
        else {
            return false;
        };
        *last_dig = realtime_seconds;
        true
    }

    pub fn encode_legacy_treasure_dig_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_TREASURE_DIG_PPD_SIZE];
        for (index, last_dig_seconds) in self.treasure_dig_last_seconds.iter().copied().enumerate()
        {
            write_i32(
                &mut bytes,
                index * 4,
                last_dig_seconds.min(i32::MAX as u64) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_treasure_dig_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_TREASURE_DIG_PPD_SIZE {
            return false;
        }
        for index in 0..TREASURE_DIG_PPD_ENTRIES {
            self.treasure_dig_last_seconds[index] = read_i32(bytes, index * 4).max(0) as u64;
        }
        true
    }

    pub fn flower_last_used_seconds(&self, location_id: u32) -> Option<u64> {
        self.flowers
            .iter()
            .find(|entry| entry.location_id == location_id)
            .map(|entry| entry.last_used_seconds)
    }

    pub fn mark_flower_used(&mut self, location_id: u32, realtime_seconds: u64) {
        if let Some(entry) = self
            .flowers
            .iter_mut()
            .find(|entry| entry.location_id == location_id)
        {
            entry.last_used_seconds = realtime_seconds;
            return;
        }

        if self.flowers.len() < FLOWER_MAX_ENTRIES {
            self.flowers.push(FlowerAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            });
            return;
        }

        if let Some(oldest) = self
            .flowers
            .iter_mut()
            .min_by_key(|entry| entry.last_used_seconds)
        {
            *oldest = FlowerAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            };
        }
    }

    pub fn encode_legacy_flower_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_FLOWER_PPD_SIZE];
        for (index, entry) in self.flowers.iter().take(FLOWER_MAX_ENTRIES).enumerate() {
            write_i32(
                &mut bytes,
                FLOWER_PPD_IDS_OFFSET + index * 4,
                entry.location_id.min(i32::MAX as u32) as i32,
            );
            write_i32(
                &mut bytes,
                FLOWER_PPD_LAST_USED_OFFSET + index * 4,
                entry.last_used_seconds.min(i32::MAX as u64) as i32,
            );
        }
        bytes
    }

    pub fn decode_legacy_flower_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_FLOWER_PPD_SIZE {
            return false;
        }
        self.flowers.clear();
        for index in 0..FLOWER_MAX_ENTRIES {
            let location_id = read_i32(bytes, FLOWER_PPD_IDS_OFFSET + index * 4);
            let last_used = read_i32(bytes, FLOWER_PPD_LAST_USED_OFFSET + index * 4);
            if location_id > 0 || last_used > 0 {
                self.flowers.push(FlowerAccess {
                    location_id: location_id.max(0) as u32,
                    last_used_seconds: last_used.max(0) as u64,
                });
            }
        }
        true
    }

    pub fn encode_legacy_area3_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_AREA3_PPD_SIZE];
        let copy_len = self.area3_ppd.len().min(LEGACY_AREA3_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.area3_ppd[..copy_len]);
        bytes
    }

    pub fn decode_legacy_area3_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_AREA3_PPD_SIZE {
            return false;
        }
        self.area3_ppd = bytes[..LEGACY_AREA3_PPD_SIZE].to_vec();
        true
    }

    pub fn encode_legacy_swear_ppd(&self) -> Vec<u8> {
        let mut bytes = vec![0; LEGACY_SWEAR_PPD_SIZE];
        let copy_len = self.swear_ppd.len().min(LEGACY_SWEAR_PPD_SIZE);
        bytes[..copy_len].copy_from_slice(&self.swear_ppd[..copy_len]);
        write_i32(
            &mut bytes,
            SWEAR_PPD_BANNED_TILL_OFFSET,
            self.shutup_until_seconds.min(i32::MAX as u64) as i32,
        );
        bytes
    }

    pub fn decode_legacy_swear_ppd(&mut self, bytes: &[u8]) -> bool {
        if bytes.len() < LEGACY_SWEAR_PPD_SIZE {
            return false;
        }
        self.swear_ppd = bytes[..LEGACY_SWEAR_PPD_SIZE].to_vec();
        self.shutup_until_seconds = read_i32(bytes, SWEAR_PPD_BANNED_TILL_OFFSET).max(0) as u64;
        true
    }

    pub fn decode_legacy_ppd_blob(&mut self, bytes: &[u8]) -> bool {
        for block in LegacyPpdBlocks::parse(bytes) {
            let Some(block) = block else {
                return false;
            };
            match block.id {
                DRD_KEYRING_PPD => {
                    if !self.decode_legacy_keyring_ppd(block.data) {
                        return false;
                    }
                }
                DRD_TREASURE_CHEST_PPD => {
                    if !self.decode_legacy_treasure_chest_ppd(block.data) {
                        return false;
                    }
                }
                DRD_TRANSPORT_PPD => {
                    if !self.decode_legacy_transport_ppd(block.data) {
                        return false;
                    }
                }
                DRD_PK_PPD => {
                    if !self.decode_legacy_pk_ppd(block.data) {
                        return false;
                    }
                }
                DRD_RANDCHEST_PPD => {
                    if !self.decode_legacy_randchest_ppd(block.data) {
                        return false;
                    }
                }
                DRD_DEMONSHRINE_PPD => {
                    if !self.decode_legacy_demonshrine_ppd(block.data) {
                        return false;
                    }
                }
                DRD_ORBSPAWN_PPD => {
                    if !self.decode_legacy_orbspawn_ppd(block.data) {
                        return false;
                    }
                }
                DRD_LOSTCON_PPD => {
                    if !self.decode_legacy_lostcon_ppd(block.data) {
                        return false;
                    }
                }
                DRD_FLOWER_PPD => {
                    if !self.decode_legacy_flower_ppd(block.data) {
                        return false;
                    }
                }
                DRD_AREA3_PPD => {
                    if !self.decode_legacy_area3_ppd(block.data) {
                        return false;
                    }
                }
                DRD_TREASURE_DIG_PPD => {
                    if !self.decode_legacy_treasure_dig_ppd(block.data) {
                        return false;
                    }
                }
                DRD_MISC_PPD => {
                    if !self.decode_legacy_misc_ppd(block.data) {
                        return false;
                    }
                }
                DRD_RUNE_PPD => {
                    if !self.decode_legacy_rune_ppd(block.data) {
                        return false;
                    }
                }
                DRD_ALIAS_PPD => {
                    if !self.decode_legacy_alias_ppd(block.data) {
                        return false;
                    }
                }
                DRD_IGNORE_PPD => {
                    if !self.decode_legacy_ignore_ppd(block.data) {
                        return false;
                    }
                }
                DRD_SWEAR_PPD => {
                    if !self.decode_legacy_swear_ppd(block.data) {
                        return false;
                    }
                }
                _ => {}
            }
        }
        true
    }

    pub fn encode_legacy_ppd_blob(&self, existing: &[u8]) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(existing.len().max(LEGACY_KEYRING_PPD_SIZE + 8));
        let mut had_keyring = false;
        let mut had_treasure_chest = false;
        let mut had_transport = false;
        let mut had_pk = false;
        let mut had_randchest = false;
        let mut had_demonshrine = false;
        let mut had_orbspawn = false;
        let mut had_lostcon = false;
        let mut had_flower = false;
        let mut had_area3 = false;
        let mut had_treasure_dig = false;
        let mut had_misc = false;
        let mut had_rune = false;
        let mut had_alias = false;
        let mut had_ignore = false;
        let mut had_swear = false;
        let mut existing_was_valid = true;

        for block in LegacyPpdBlocks::parse(existing) {
            let Some(block) = block else {
                existing_was_valid = false;
                break;
            };
            if block.id == DRD_JUNK_PPD {
                continue;
            }
            if block.id == DRD_KEYRING_PPD {
                had_keyring = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_KEYRING_PPD,
                    &self.encode_legacy_keyring_ppd(),
                );
            } else if block.id == DRD_TREASURE_CHEST_PPD {
                had_treasure_chest = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_TREASURE_CHEST_PPD,
                    &self.encode_legacy_treasure_chest_ppd(),
                );
            } else if block.id == DRD_TRANSPORT_PPD {
                had_transport = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_TRANSPORT_PPD,
                    &self.encode_legacy_transport_ppd(),
                );
            } else if block.id == DRD_PK_PPD {
                had_pk = true;
                write_ppd_block(&mut encoded, DRD_PK_PPD, &self.encode_legacy_pk_ppd());
            } else if block.id == DRD_RANDCHEST_PPD {
                had_randchest = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_RANDCHEST_PPD,
                    &self.encode_legacy_randchest_ppd(),
                );
            } else if block.id == DRD_DEMONSHRINE_PPD {
                had_demonshrine = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_DEMONSHRINE_PPD,
                    &self.encode_legacy_demonshrine_ppd(),
                );
            } else if block.id == DRD_ORBSPAWN_PPD {
                had_orbspawn = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_ORBSPAWN_PPD,
                    &self.encode_legacy_orbspawn_ppd(),
                );
            } else if block.id == DRD_LOSTCON_PPD {
                had_lostcon = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_LOSTCON_PPD,
                    &self.encode_legacy_lostcon_ppd(),
                );
            } else if block.id == DRD_FLOWER_PPD {
                had_flower = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_FLOWER_PPD,
                    &self.encode_legacy_flower_ppd(),
                );
            } else if block.id == DRD_AREA3_PPD {
                had_area3 = true;
                write_ppd_block(&mut encoded, DRD_AREA3_PPD, &self.encode_legacy_area3_ppd());
            } else if block.id == DRD_TREASURE_DIG_PPD {
                had_treasure_dig = true;
                write_ppd_block(
                    &mut encoded,
                    DRD_TREASURE_DIG_PPD,
                    &self.encode_legacy_treasure_dig_ppd(),
                );
            } else if block.id == DRD_MISC_PPD {
                had_misc = true;
                write_ppd_block(&mut encoded, DRD_MISC_PPD, &self.encode_legacy_misc_ppd());
            } else if block.id == DRD_RUNE_PPD {
                had_rune = true;
                write_ppd_block(&mut encoded, DRD_RUNE_PPD, &self.encode_legacy_rune_ppd());
            } else if block.id == DRD_ALIAS_PPD {
                had_alias = true;
                if !self.aliases.is_empty() {
                    write_ppd_block(&mut encoded, DRD_ALIAS_PPD, &self.encode_legacy_alias_ppd());
                }
            } else if block.id == DRD_IGNORE_PPD {
                had_ignore = true;
                if !self.ignored_characters.is_empty() {
                    write_ppd_block(
                        &mut encoded,
                        DRD_IGNORE_PPD,
                        &self.encode_legacy_ignore_ppd(),
                    );
                }
            } else if block.id == DRD_SWEAR_PPD {
                had_swear = true;
                if !self.swear_ppd.is_empty() || self.shutup_until_seconds != 0 {
                    write_ppd_block(&mut encoded, DRD_SWEAR_PPD, &self.encode_legacy_swear_ppd());
                }
            } else {
                write_ppd_block(&mut encoded, block.id, block.data);
            }
        }

        if !had_keyring && (existing_was_valid || existing.is_empty()) {
            if !self.keyring.is_empty() || self.keyring_auto_add {
                write_ppd_block(
                    &mut encoded,
                    DRD_KEYRING_PPD,
                    &self.encode_legacy_keyring_ppd(),
                );
            }
        }
        if !had_treasure_chest && (existing_was_valid || existing.is_empty()) {
            if !self.chest_last_access_seconds.is_empty() {
                write_ppd_block(
                    &mut encoded,
                    DRD_TREASURE_CHEST_PPD,
                    &self.encode_legacy_treasure_chest_ppd(),
                );
            }
        }
        if !had_transport && (existing_was_valid || existing.is_empty()) && self.transport_seen != 0
        {
            write_ppd_block(
                &mut encoded,
                DRD_TRANSPORT_PPD,
                &self.encode_legacy_transport_ppd(),
            );
        }
        if !had_pk && (existing_was_valid || existing.is_empty()) {
            if self.pk_kills != 0
                || self.pk_deaths != 0
                || self.pk_last_kill != 0
                || self.pk_last_death != 0
                || !self.pk_hate.is_empty()
            {
                write_ppd_block(&mut encoded, DRD_PK_PPD, &self.encode_legacy_pk_ppd());
            }
        }
        if !had_randchest && (existing_was_valid || existing.is_empty()) {
            if !self.random_chests.is_empty() {
                write_ppd_block(
                    &mut encoded,
                    DRD_RANDCHEST_PPD,
                    &self.encode_legacy_randchest_ppd(),
                );
            }
        }
        if !had_demonshrine && (existing_was_valid || existing.is_empty()) {
            if !self.demonshrines.is_empty() {
                write_ppd_block(
                    &mut encoded,
                    DRD_DEMONSHRINE_PPD,
                    &self.encode_legacy_demonshrine_ppd(),
                );
            }
        }
        if !had_orbspawn && (existing_was_valid || existing.is_empty()) {
            if !self.orb_spawns.is_empty() {
                write_ppd_block(
                    &mut encoded,
                    DRD_ORBSPAWN_PPD,
                    &self.encode_legacy_orbspawn_ppd(),
                );
            }
        }
        if !had_lostcon && (existing_was_valid || existing.is_empty()) && self.max_lag_seconds != 0
        {
            write_ppd_block(
                &mut encoded,
                DRD_LOSTCON_PPD,
                &self.encode_legacy_lostcon_ppd(),
            );
        }
        if !had_flower && (existing_was_valid || existing.is_empty()) && !self.flowers.is_empty() {
            write_ppd_block(
                &mut encoded,
                DRD_FLOWER_PPD,
                &self.encode_legacy_flower_ppd(),
            );
        }
        if !had_area3 && (existing_was_valid || existing.is_empty()) && !self.area3_ppd.is_empty() {
            write_ppd_block(&mut encoded, DRD_AREA3_PPD, &self.encode_legacy_area3_ppd());
        }
        if !had_treasure_dig && (existing_was_valid || existing.is_empty()) {
            if self
                .treasure_dig_last_seconds
                .iter()
                .any(|seconds| *seconds != 0)
            {
                write_ppd_block(
                    &mut encoded,
                    DRD_TREASURE_DIG_PPD,
                    &self.encode_legacy_treasure_dig_ppd(),
                );
            }
        }
        if !had_misc && (existing_was_valid || existing.is_empty()) && !self.misc_ppd.is_empty() {
            write_ppd_block(&mut encoded, DRD_MISC_PPD, &self.encode_legacy_misc_ppd());
        }
        if !had_rune && (existing_was_valid || existing.is_empty()) {
            if self.rune_used_words.iter().any(|word| *word != 0)
                || self.rune_special_exec.iter().any(|value| *value != 0)
            {
                write_ppd_block(&mut encoded, DRD_RUNE_PPD, &self.encode_legacy_rune_ppd());
            }
        }
        if !had_alias && (existing_was_valid || existing.is_empty()) && !self.aliases.is_empty() {
            write_ppd_block(&mut encoded, DRD_ALIAS_PPD, &self.encode_legacy_alias_ppd());
        }
        if !had_ignore
            && (existing_was_valid || existing.is_empty())
            && !self.ignored_characters.is_empty()
        {
            write_ppd_block(
                &mut encoded,
                DRD_IGNORE_PPD,
                &self.encode_legacy_ignore_ppd(),
            );
        }
        if !had_swear
            && (existing_was_valid || existing.is_empty())
            && self.shutup_until_seconds != 0
        {
            write_ppd_block(&mut encoded, DRD_SWEAR_PPD, &self.encode_legacy_swear_ppd());
        }

        encoded
    }

    pub fn touch_xmas_tree(
        &mut self,
        area_id: u16,
        event_year: i32,
        is_xmas: bool,
        has_holiday_treat: bool,
    ) -> XmasTreeResult {
        if !is_xmas {
            return XmasTreeResult::Dormant;
        }
        if self.misc_ppd.len() < LEGACY_MISC_PPD_SIZE {
            self.misc_ppd.resize(LEGACY_MISC_PPD_SIZE, 0);
        }
        if read_i32(&self.misc_ppd, MISC_PPD_GIFT_YEAR_OFFSET) != event_year {
            for byte in &mut self.misc_ppd[MISC_PPD_TREEDONE_OFFSET..MISC_PPD_TREEDONE_OFFSET + 8] {
                *byte = 0;
            }
            write_i32(&mut self.misc_ppd, MISC_PPD_GIFT_YEAR_OFFSET, event_year);
        }

        let idx = usize::from(area_id / 8);
        let bit = 1u8 << (area_id % 8);
        if idx >= 8 || self.misc_ppd[MISC_PPD_TREEDONE_OFFSET + idx] & bit != 0 {
            return XmasTreeResult::AlreadyGranted;
        }
        if !has_holiday_treat {
            return XmasTreeResult::NeedsHolidayTreat;
        }

        self.misc_ppd[MISC_PPD_TREEDONE_OFFSET + idx] |= bit;
        XmasTreeResult::GiftGranted
    }

    pub fn memorize_park_shrine(&mut self, shrine: u8) -> Option<bool> {
        let offset = match shrine {
            1 => AREA3_PPD_KELLY_FOUND1_OFFSET,
            2 => AREA3_PPD_KELLY_FOUND2_OFFSET,
            3 => AREA3_PPD_KELLY_FOUND3_OFFSET,
            _ => return None,
        };
        if self.area3_ppd.len() < LEGACY_AREA3_PPD_SIZE {
            self.area3_ppd.resize(LEGACY_AREA3_PPD_SIZE, 0);
        }
        let was_new = read_i32(&self.area3_ppd, offset) == 0;
        write_i32(&mut self.area3_ppd, offset, 1);
        Some(was_new)
    }

    pub fn unmark_xmas_tree(&mut self, area_id: u16) {
        if self.misc_ppd.len() < LEGACY_MISC_PPD_SIZE {
            return;
        }
        let idx = usize::from(area_id / 8);
        if idx < 8 {
            self.misc_ppd[MISC_PPD_TREEDONE_OFFSET + idx] &= !(1u8 << (area_id % 8));
        }
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

    pub fn touch_demonshrine(
        &mut self,
        character: &mut Character,
        location_id: u32,
    ) -> DemonShrineResult {
        if self.demonshrines.iter().any(|&id| id == location_id) {
            return DemonShrineResult::AlreadyKnown;
        }
        if self.demonshrines.len() >= DEMONSHRINE_MAX_ENTRIES {
            return DemonShrineResult::Full;
        }

        self.demonshrines.push(location_id);
        let demon_index = CharacterValue::Demon as usize;
        let demon_value = character
            .values
            .get_mut(1)
            .and_then(|values| values.get_mut(demon_index));
        let new_demon = if let Some(value) = demon_value {
            *value = value.saturating_add(1);
            u32::from((*value).max(0) as u16)
        } else {
            0
        };
        let exp_added =
            (250_u32.saturating_add(new_demon.saturating_mul(100))).min(character.exp / 25);
        character.exp = character.exp.saturating_add(exp_added);
        character
            .flags
            .insert(CharacterFlags::UPDATE | CharacterFlags::ITEMS);
        DemonShrineResult::Learned { exp_added }
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

    pub fn orb_spawn_last_used_seconds(&self, location_id: u32) -> Option<u64> {
        self.orb_spawns
            .iter()
            .find(|entry| entry.location_id == location_id)
            .map(|entry| entry.last_used_seconds)
    }

    pub fn mark_orb_spawn_used(&mut self, location_id: u32, realtime_seconds: u64) {
        if let Some(entry) = self
            .orb_spawns
            .iter_mut()
            .find(|entry| entry.location_id == location_id)
        {
            entry.last_used_seconds = realtime_seconds;
            return;
        }
        if self.orb_spawns.len() < ORBSPAWN_MAX_ENTRIES {
            self.orb_spawns.push(OrbSpawnAccess {
                location_id,
                last_used_seconds: realtime_seconds,
            });
            return;
        }
        if let Some(oldest) = self
            .orb_spawns
            .iter_mut()
            .min_by_key(|entry| entry.last_used_seconds)
        {
            *oldest = OrbSpawnAccess {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LegacyPpdBlock<'a> {
    id: u32,
    data: &'a [u8],
}

struct LegacyPpdBlocks<'a> {
    bytes: &'a [u8],
    offset: usize,
    failed: bool,
}

impl<'a> LegacyPpdBlocks<'a> {
    fn parse(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            offset: 0,
            failed: false,
        }
    }
}

impl<'a> Iterator for LegacyPpdBlocks<'a> {
    type Item = Option<LegacyPpdBlock<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed || self.offset == self.bytes.len() {
            return None;
        }
        if self.bytes.len().saturating_sub(self.offset) < 8 {
            self.failed = true;
            return Some(None);
        }

        let id = read_u32(self.bytes, self.offset);
        let size = read_u32(self.bytes, self.offset + 4) as usize;
        self.offset += 8;
        if self.bytes.len().saturating_sub(self.offset) < size {
            self.failed = true;
            return Some(None);
        }

        let data = &self.bytes[self.offset..self.offset + size];
        self.offset += size;
        Some(Some(LegacyPpdBlock { id, data }))
    }
}

fn write_ppd_block(bytes: &mut Vec<u8>, id: u32, data: &[u8]) {
    bytes.extend_from_slice(&id.to_le_bytes());
    bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(data);
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
        entity::{Character, CharacterFlags, ItemFlags, MAX_MODIFIERS},
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
        assert_eq!(DRD_JUNK_PPD, 0x8100_0072);
        assert_eq!(DRD_TREASURE_CHEST_PPD, 0x8100_0011);
        assert_eq!(DRD_RANDCHEST_PPD, 0x8100_003f);
        assert_eq!(DRD_DEMONSHRINE_PPD, 0x8100_0044);
        assert_eq!(DRD_MISC_PPD, 0x8100_0071);
        assert_eq!(DRD_ALIAS_PPD, 0x8100_0050);
        assert_eq!(DRD_IGNORE_PPD, 0x8100_0064);
        assert_eq!(DRD_SWEAR_PPD, 0x8100_006d);
        assert_eq!(DRD_KEYRING_PPD, 0xbb00_0007);
        assert_eq!(LEGACY_TREASURE_CHEST_PPD_SIZE, 800);
        assert_eq!(LEGACY_RANDCHEST_PPD_SIZE, 800);
        assert_eq!(LEGACY_DEMONSHRINE_PPD_SIZE, 400);
        assert_eq!(LEGACY_MISC_PPD_SIZE, 36);
        assert_eq!(LEGACY_IGNORE_PPD_SIZE, 400);
        assert_eq!(LEGACY_SWEAR_PPD_SIZE, 932);
    }

    #[test]
    fn swear_ppd_codec_preserves_counters_and_maps_banned_till() {
        let mut bytes = vec![0; LEGACY_SWEAR_PPD_SIZE];
        write_i32(&mut bytes, 0, 11);
        write_i32(&mut bytes, 40, 22);
        bytes[44..49].copy_from_slice(b"hello");
        write_i32(&mut bytes, SWEAR_PPD_BANNED_TILL_OFFSET, 1234);

        let mut player = PlayerRuntime::connected(1, 0);
        assert!(player.decode_legacy_swear_ppd(&bytes));
        assert_eq!(player.shutup_until_seconds, 1234);

        player.shutup_until_seconds = 5678;
        let encoded = player.encode_legacy_swear_ppd();
        assert_eq!(encoded.len(), LEGACY_SWEAR_PPD_SIZE);
        assert_eq!(read_i32(&encoded, 0), 11);
        assert_eq!(read_i32(&encoded, 40), 22);
        assert_eq!(&encoded[44..49], b"hello");
        assert_eq!(read_i32(&encoded, SWEAR_PPD_BANNED_TILL_OFFSET), 5678);
    }

    #[test]
    fn swear_ppd_outer_blob_replaces_appends_and_removes_empty_state() {
        let mut existing = Vec::new();
        let mut old_swear = vec![0; LEGACY_SWEAR_PPD_SIZE];
        write_i32(&mut old_swear, 0, 77);
        write_ppd_block(&mut existing, DRD_SWEAR_PPD, &old_swear);
        write_ppd_block(&mut existing, 0x5566_7788, &[3]);

        let mut player = PlayerRuntime::connected(1, 0);
        player.swear_ppd = old_swear;
        player.shutup_until_seconds = 600;
        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), DRD_SWEAR_PPD);
        assert_eq!(read_i32(&encoded, 8), 77);
        assert_eq!(read_i32(&encoded, 8 + SWEAR_PPD_BANNED_TILL_OFFSET), 600);
        assert_eq!(read_u32(&encoded, 8 + LEGACY_SWEAR_PPD_SIZE), 0x5566_7788);

        let mut appended = PlayerRuntime::connected(2, 0);
        appended.shutup_until_seconds = 700;
        let encoded = appended.encode_legacy_ppd_blob(&[]);
        assert_eq!(read_u32(&encoded, 0), DRD_SWEAR_PPD);
        assert_eq!(read_i32(&encoded, 8 + SWEAR_PPD_BANNED_TILL_OFFSET), 700);

        let empty = PlayerRuntime::connected(3, 0);
        let encoded = empty.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), 0x5566_7788);
        assert!(!encoded
            .windows(4)
            .any(|window| window == DRD_SWEAR_PPD.to_le_bytes()));
    }

    #[test]
    fn ignore_ppd_codec_matches_legacy_fixed_array() {
        let mut player = PlayerRuntime::connected(1, 0);
        assert_eq!(
            player.toggle_ignored_character(42),
            IgnoreToggleResult::Added
        );
        assert_eq!(
            player.toggle_ignored_character(99),
            IgnoreToggleResult::Added
        );
        assert!(player.ignores_character(42));

        let bytes = player.encode_legacy_ignore_ppd();
        assert_eq!(bytes.len(), LEGACY_IGNORE_PPD_SIZE);
        assert_eq!(read_i32(&bytes, 0), 42);
        assert_eq!(read_i32(&bytes, 4), 99);
        assert_eq!(read_i32(&bytes, 8), 0);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ignore_ppd(&bytes));
        assert_eq!(decoded.ignored_characters, vec![42, 99]);
        assert_eq!(
            decoded.toggle_ignored_character(42),
            IgnoreToggleResult::Removed
        );
        assert!(!decoded.ignores_character(42));
    }

    #[test]
    fn ignore_ppd_outer_blob_replaces_and_removes_empty_lists() {
        let mut existing = Vec::new();
        write_ppd_block(&mut existing, DRD_IGNORE_PPD, &[1; LEGACY_IGNORE_PPD_SIZE]);
        write_ppd_block(&mut existing, 0x8765_4321, &[7]);

        let mut player = PlayerRuntime::connected(1, 0);
        player.ignored_characters.push(123);
        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), DRD_IGNORE_PPD);
        assert_eq!(read_i32(&encoded, 8), 123);
        assert_eq!(read_u32(&encoded, 8 + LEGACY_IGNORE_PPD_SIZE), 0x8765_4321);

        player.clear_ignored_characters();
        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), 0x8765_4321);
        assert!(!encoded
            .windows(4)
            .any(|window| window == DRD_IGNORE_PPD.to_le_bytes()));
    }

    #[test]
    fn alias_ppd_codec_matches_legacy_fixed_arrays() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.aliases.push(CommandAlias {
            from: "tyvm123".to_string(),
            to: "Thank you very much for everything".to_string(),
        });

        let bytes = player.encode_legacy_alias_ppd();
        assert_eq!(bytes.len(), LEGACY_ALIAS_PPD_SIZE);
        assert_eq!(&bytes[..8], b"tyvm123\0");
        assert_eq!(&bytes[8..42], b"Thank you very much for everything");
        assert_eq!(bytes[42], 0);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_alias_ppd(&bytes));
        assert_eq!(decoded.aliases, player.aliases);
    }

    #[test]
    fn alias_ppd_outer_blob_replaces_and_removes_empty_aliases() {
        let mut existing = Vec::new();
        write_ppd_block(&mut existing, DRD_ALIAS_PPD, &[1; LEGACY_ALIAS_PPD_SIZE]);
        write_ppd_block(&mut existing, 0x1234_5678, &[9]);

        let mut player = PlayerRuntime::connected(1, 0);
        player.aliases.push(CommandAlias {
            from: "ty".to_string(),
            to: "Thank you!".to_string(),
        });
        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), DRD_ALIAS_PPD);
        assert_eq!(&encoded[8..11], b"ty\0");
        assert_eq!(read_u32(&encoded, 8 + LEGACY_ALIAS_PPD_SIZE), 0x1234_5678);

        player.aliases.clear();
        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), 0x1234_5678);
        assert!(!encoded
            .windows(4)
            .any(|window| window == DRD_ALIAS_PPD.to_le_bytes()));
    }

    #[test]
    fn alias_expansion_matches_legacy_word_boundaries() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.aliases.push(CommandAlias {
            from: "ty".to_string(),
            to: "Thank you".to_string(),
        });
        player.aliases.push(CommandAlias {
            from: "don't".to_string(),
            to: "do not".to_string(),
        });

        assert_eq!(player.expand_aliases("ty!"), "Thank you!");
        assert_eq!(player.expand_aliases("pretty ty"), "pretty Thank you");
        assert_eq!(player.expand_aliases("don't stop"), "do not stop");
    }

    #[test]
    fn special_shrine_requires_confirmation_then_removes_hardcore() {
        let mut player = PlayerRuntime::connected(7, 11);
        let mut character = character(3);
        character.flags.insert(CharacterFlags::HARDCORE);
        character.creation_time = SPECIAL_SHRINE_HCSC_CUTOFF_SECONDS;

        assert_eq!(
            player.touch_special_shrine(&mut character, 0x0A, 100),
            SpecialShrineResult::ConfirmRequired,
        );
        assert!(character.flags.contains(CharacterFlags::HARDCORE));
        assert_eq!(
            player.touch_special_shrine(&mut character, 0x0A, 109),
            SpecialShrineResult::HardcoreRemoved,
        );
        assert!(!character.flags.contains(CharacterFlags::HARDCORE));
    }

    #[test]
    fn special_shrine_blocks_non_hardcore_and_new_hardcore() {
        let mut player = PlayerRuntime::connected(7, 11);
        let mut softcore = character(3);
        assert_eq!(
            player.touch_special_shrine(&mut softcore, 0x0A, 100),
            SpecialShrineResult::NothingHere,
        );

        let mut new_hardcore = character(4);
        new_hardcore.flags.insert(CharacterFlags::HARDCORE);
        new_hardcore.creation_time = SPECIAL_SHRINE_HCSC_CUTOFF_SECONDS + 1;
        assert_eq!(
            player.touch_special_shrine(&mut new_hardcore, 0x0A, 100),
            SpecialShrineResult::NothingHere,
        );
        assert!(new_hardcore.flags.contains(CharacterFlags::HARDCORE));
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
    fn treasure_chest_ppd_codec_matches_legacy_c_layout() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.mark_chest_access(0, 1234);
        player.mark_chest_access(63, 86_400);
        player.mark_chest_access(199, i32::MAX as u64 + 99);

        let bytes = player.encode_legacy_treasure_chest_ppd();
        assert_eq!(bytes.len(), LEGACY_TREASURE_CHEST_PPD_SIZE);
        assert_eq!(read_i32(&bytes, 0), 1234);
        assert_eq!(read_i32(&bytes, 63 * 4), 86_400);
        assert_eq!(read_i32(&bytes, 199 * 4), i32::MAX);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_treasure_chest_ppd(&bytes));
        assert_eq!(decoded.chest_last_access_seconds(0), 1234);
        assert_eq!(decoded.chest_last_access_seconds(63), 86_400);
        assert_eq!(decoded.chest_last_access_seconds(199), i32::MAX as u64);
        assert_eq!(decoded.chest_last_access_seconds(1), 0);
    }

    #[test]
    fn treasure_dig_ppd_codec_matches_legacy_c_layout() {
        let mut player = PlayerRuntime::connected(1, 0);
        assert!(player.mark_treasure_dig(0, 1234));
        assert!(player.mark_treasure_dig(4, i32::MAX as u64 + 99));

        let bytes = player.encode_legacy_treasure_dig_ppd();
        assert_eq!(bytes.len(), LEGACY_TREASURE_DIG_PPD_SIZE);
        assert_eq!(read_i32(&bytes, 0), 1234);
        assert_eq!(read_i32(&bytes, 4), 0);
        assert_eq!(read_i32(&bytes, 4 * 4), i32::MAX);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_treasure_dig_ppd(&bytes));
        assert_eq!(decoded.treasure_dig_last_seconds(0), 1234);
        assert_eq!(decoded.treasure_dig_last_seconds(1), 0);
        assert_eq!(decoded.treasure_dig_last_seconds(4), i32::MAX as u64);
    }

    #[test]
    fn randchest_ppd_codec_matches_legacy_c_layout() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.mark_random_chest_used(0x0001_0506, 1234);
        player.mark_random_chest_used(0x0001_0708, i32::MAX as u64 + 99);

        let bytes = player.encode_legacy_randchest_ppd();
        assert_eq!(bytes.len(), LEGACY_RANDCHEST_PPD_SIZE);
        assert_eq!(read_i32(&bytes, 0), 0x0001_0506);
        assert_eq!(read_i32(&bytes, 4), 0x0001_0708);
        assert_eq!(read_i32(&bytes, RANDCHEST_PPD_LAST_USED_OFFSET), 1234);
        assert_eq!(
            read_i32(&bytes, RANDCHEST_PPD_LAST_USED_OFFSET + 4),
            i32::MAX
        );

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_randchest_ppd(&bytes));
        assert_eq!(
            decoded.random_chest_last_used_seconds(0x0001_0506),
            Some(1234)
        );
        assert_eq!(
            decoded.random_chest_last_used_seconds(0x0001_0708),
            Some(i32::MAX as u64)
        );
        assert_eq!(decoded.random_chest_last_used_seconds(0x0001_090a), None);
    }

    #[test]
    fn orbspawn_ppd_codec_matches_legacy_c_layout() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.mark_orb_spawn_used(0x0001_0506, 1234);
        player.mark_orb_spawn_used(0x0001_0708, i32::MAX as u64 + 99);

        let bytes = player.encode_legacy_orbspawn_ppd();
        assert_eq!(bytes.len(), LEGACY_ORBSPAWN_PPD_SIZE);
        assert_eq!(read_i32(&bytes, 0), 0x0001_0506);
        assert_eq!(read_i32(&bytes, 4), 0x0001_0708);
        assert_eq!(read_i32(&bytes, ORBSPAWN_PPD_LAST_USED_OFFSET), 1234);
        assert_eq!(
            read_i32(&bytes, ORBSPAWN_PPD_LAST_USED_OFFSET + 4),
            i32::MAX
        );

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_orbspawn_ppd(&bytes));
        assert_eq!(decoded.orb_spawn_last_used_seconds(0x0001_0506), Some(1234));
        assert_eq!(
            decoded.orb_spawn_last_used_seconds(0x0001_0708),
            Some(i32::MAX as u64)
        );
        assert_eq!(decoded.orb_spawn_last_used_seconds(0x0001_090a), None);
    }

    #[test]
    fn keyring_ppd_blob_round_trips_with_legacy_block_framing() {
        let unknown_id = make_drd(DEV_ID_DB, 22 | PERSISTENT_PLAYER_DATA);
        let mut existing = Vec::new();
        write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
        write_ppd_block(&mut existing, DRD_JUNK_PPD, &[9, 9, 9]);

        let mut player = PlayerRuntime::connected(1, 0);
        player.set_keyring_auto_add(true);
        assert_eq!(
            player.add_keyring_key(0x1122_3344, "Copper Key"),
            KeyringAddResult::Added
        );

        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), unknown_id);
        assert_eq!(read_u32(&encoded, 4), 4);
        assert_eq!(&encoded[8..12], &[1, 2, 3, 4]);
        assert_eq!(read_u32(&encoded, 12), DRD_KEYRING_PPD);
        assert_eq!(read_u32(&encoded, 16), LEGACY_KEYRING_PPD_SIZE as u32);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert!(decoded.keyring_auto_add());
        assert_eq!(decoded.keyring_key_name(0x1122_3344), Some("Copper Key"));
        assert!(!encoded
            .windows(4)
            .any(|window| window == DRD_JUNK_PPD.to_le_bytes()));
    }

    #[test]
    fn treasure_chest_ppd_blob_round_trips_with_legacy_block_framing() {
        let unknown_id = make_drd(DEV_ID_DB, 222 | PERSISTENT_PLAYER_DATA);
        let mut existing = Vec::new();
        write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
        write_ppd_block(
            &mut existing,
            DRD_TREASURE_CHEST_PPD,
            &[0; LEGACY_TREASURE_CHEST_PPD_SIZE],
        );

        let mut player = PlayerRuntime::connected(1, 0);
        player.mark_chest_access(17, 777);

        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), unknown_id);
        assert_eq!(read_u32(&encoded, 12), DRD_TREASURE_CHEST_PPD);
        assert_eq!(
            read_u32(&encoded, 16),
            LEGACY_TREASURE_CHEST_PPD_SIZE as u32
        );
        assert_eq!(read_i32(&encoded, 20 + 17 * 4), 777);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(decoded.chest_last_access_seconds(17), 777);
    }

    #[test]
    fn ppd_blob_appends_treasure_chests_without_existing_block() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.mark_chest_access(5, 55);

        let encoded = player.encode_legacy_ppd_blob(&[]);
        assert_eq!(read_u32(&encoded, 0), DRD_TREASURE_CHEST_PPD);
        assert_eq!(read_u32(&encoded, 4), LEGACY_TREASURE_CHEST_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 8 + 5 * 4), 55);
    }

    #[test]
    fn randchest_ppd_blob_round_trips_with_legacy_block_framing() {
        let unknown_id = make_drd(DEV_ID_DB, 22 | PERSISTENT_PLAYER_DATA);
        let mut existing_randchest = vec![0; LEGACY_RANDCHEST_PPD_SIZE];
        write_i32(
            &mut existing_randchest,
            RANDCHEST_PPD_IDS_OFFSET,
            0x0001_0203,
        );
        write_i32(&mut existing_randchest, RANDCHEST_PPD_LAST_USED_OFFSET, 44);

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
        write_ppd_block(&mut existing, DRD_RANDCHEST_PPD, &existing_randchest);

        let mut player = PlayerRuntime::connected(1, 0);
        player.mark_random_chest_used(0x0001_0506, 777);

        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), unknown_id);
        assert_eq!(read_u32(&encoded, 12), DRD_RANDCHEST_PPD);
        assert_eq!(read_u32(&encoded, 16), LEGACY_RANDCHEST_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 20), 0x0001_0506);
        assert_eq!(read_i32(&encoded, 20 + RANDCHEST_PPD_LAST_USED_OFFSET), 777);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(
            decoded.random_chest_last_used_seconds(0x0001_0506),
            Some(777)
        );
        assert_eq!(decoded.random_chest_last_used_seconds(0x0001_0203), None);
    }

    #[test]
    fn ppd_blob_appends_randchests_without_existing_block() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.mark_random_chest_used(0x0001_0203, 55);

        let encoded = player.encode_legacy_ppd_blob(&[]);
        assert_eq!(read_u32(&encoded, 0), DRD_RANDCHEST_PPD);
        assert_eq!(read_u32(&encoded, 4), LEGACY_RANDCHEST_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 8), 0x0001_0203);
        assert_eq!(read_i32(&encoded, 8 + RANDCHEST_PPD_LAST_USED_OFFSET), 55);
    }

    #[test]
    fn transport_ppd_codec_matches_legacy_seen_mask_layout() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.transport_seen = 0x0102_0304_0506_0708;

        let encoded = player.encode_legacy_transport_ppd();
        assert_eq!(encoded, 0x0102_0304_0506_0708_u64.to_le_bytes());

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_transport_ppd(&encoded));
        assert_eq!(decoded.transport_seen, 0x0102_0304_0506_0708);
        assert!(!decoded.decode_legacy_transport_ppd(&encoded[..7]));
    }

    #[test]
    fn lostcon_ppd_codec_matches_legacy_c_layout() {
        assert_eq!(LOSTCON_PPD_MAXLAG_OFFSET + 8, LEGACY_LOSTCON_PPD_SIZE);

        let mut player = PlayerRuntime::connected(1, 0);
        player.set_max_lag_seconds(17);

        let encoded = player.encode_legacy_lostcon_ppd();
        assert_eq!(encoded.len(), LEGACY_LOSTCON_PPD_SIZE);
        assert_eq!(read_i32(&encoded, 0), 0);
        assert_eq!(read_i32(&encoded, LOSTCON_PPD_MAXLAG_OFFSET), 17);
        assert_eq!(read_i32(&encoded, LOSTCON_PPD_MAXLAG_OFFSET + 4), 0);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_lostcon_ppd(&encoded));
        assert_eq!(decoded.max_lag_seconds, 17);
        assert!(!decoded.decode_legacy_lostcon_ppd(&encoded[..LEGACY_LOSTCON_PPD_SIZE - 1]));
    }

    #[test]
    fn pk_ppd_codec_matches_legacy_c_layout() {
        assert_eq!(
            PK_PPD_HATE_OFFSET + PK_HATE_MAX_ENTRIES * 4,
            LEGACY_PK_PPD_SIZE
        );

        let mut player = PlayerRuntime::connected(1, 0);
        player.pk_kills = 3;
        player.pk_deaths = 4;
        player.pk_last_kill = 0x1122_3344;
        player.pk_last_death = i32::MAX as u32 + 99;
        assert!(player.add_pk_hate(1001));
        assert!(player.add_pk_hate(1002));
        assert!(!player.add_pk_hate(1002));

        let encoded = player.encode_legacy_pk_ppd();
        assert_eq!(encoded.len(), LEGACY_PK_PPD_SIZE);
        assert_eq!(read_i32(&encoded, PK_PPD_KILLS_OFFSET), 3);
        assert_eq!(read_i32(&encoded, PK_PPD_DEATHS_OFFSET), 4);
        assert_eq!(read_i32(&encoded, PK_PPD_LAST_KILL_OFFSET), 0x1122_3344);
        assert_eq!(read_i32(&encoded, PK_PPD_LAST_DEATH_OFFSET), i32::MAX);
        assert_eq!(read_i32(&encoded, PK_PPD_HATE_OFFSET), 1002);
        assert_eq!(read_i32(&encoded, PK_PPD_HATE_OFFSET + 4), 1001);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_pk_ppd(&encoded));
        assert_eq!(decoded.pk_kills, 3);
        assert_eq!(decoded.pk_deaths, 4);
        assert_eq!(decoded.pk_last_kill, 0x1122_3344);
        assert_eq!(decoded.pk_last_death, i32::MAX as u32);
        assert_eq!(decoded.pk_hate, vec![1002, 1001]);
        assert!(decoded.has_pk_hate_for(1001));
        assert!(!decoded.has_pk_hate_for(1003));
        assert!(!decoded.decode_legacy_pk_ppd(&encoded[..LEGACY_PK_PPD_SIZE - 1]));
    }

    #[test]
    fn pk_hate_helpers_preserve_legacy_front_priority_and_eviction() {
        let mut player = PlayerRuntime::connected(1, 0);
        assert!(!player.add_pk_hate(0));
        assert!(player.add_pk_hate(10));
        assert!(player.add_pk_hate(20));
        assert!(player.add_pk_hate(30));
        assert_eq!(player.pk_hate, vec![30, 20, 10]);

        assert!(!player.add_pk_hate(10));
        assert_eq!(player.pk_hate, vec![10, 30, 20]);

        assert!(player.remove_pk_hate(30));
        assert_eq!(player.pk_hate, vec![10, 20]);
        assert!(!player.remove_pk_hate(30));

        for id in 100..(100 + PK_HATE_MAX_ENTRIES as u32 + 5) {
            player.add_pk_hate(id);
        }
        assert_eq!(player.pk_hate.len(), PK_HATE_MAX_ENTRIES);
        assert_eq!(player.pk_hate[0], 154);
        assert_eq!(player.pk_hate[PK_HATE_MAX_ENTRIES - 1], 105);
        assert!(!player.has_pk_hate_for(104));
    }

    #[test]
    fn pk_hate_hit_helper_clears_legacy_lag_flag() {
        let mut player = PlayerRuntime::connected(1, 0);
        let mut character = character(1);
        character.flags.insert(CharacterFlags::LAG);

        assert!(player.add_pk_hate_from_hit(&mut character, 20));
        assert_eq!(player.pk_hate, vec![20]);
        assert!(!character.flags.contains(CharacterFlags::LAG));

        character.flags.insert(CharacterFlags::LAG);
        assert!(!player.add_pk_hate_from_hit(&mut character, 20));
        assert_eq!(player.pk_hate, vec![20]);
        assert!(!character.flags.contains(CharacterFlags::LAG));

        character.flags.insert(CharacterFlags::LAG);
        assert!(!player.add_pk_hate_from_hit(&mut character, 0));
        assert!(character.flags.contains(CharacterFlags::LAG));
    }

    #[test]
    fn pk_ppd_blob_round_trips_with_legacy_block_framing() {
        let unknown_id = make_drd(DEV_ID_DB, 22 | PERSISTENT_PLAYER_DATA);
        let mut existing_pk = vec![0; LEGACY_PK_PPD_SIZE];
        write_i32(&mut existing_pk, PK_PPD_KILLS_OFFSET, 1);
        write_i32(&mut existing_pk, PK_PPD_HATE_OFFSET, 999);

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
        write_ppd_block(&mut existing, DRD_PK_PPD, &existing_pk);

        let mut player = PlayerRuntime::connected(1, 0);
        player.pk_deaths = 2;
        assert!(player.add_pk_hate(1234));

        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), unknown_id);
        assert_eq!(read_u32(&encoded, 12), DRD_PK_PPD);
        assert_eq!(read_u32(&encoded, 16), LEGACY_PK_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 20 + PK_PPD_KILLS_OFFSET), 0);
        assert_eq!(read_i32(&encoded, 20 + PK_PPD_DEATHS_OFFSET), 2);
        assert_eq!(read_i32(&encoded, 20 + PK_PPD_HATE_OFFSET), 1234);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(decoded.pk_deaths, 2);
        assert_eq!(decoded.pk_hate, vec![1234]);
    }

    #[test]
    fn ppd_blob_appends_pk_without_existing_block() {
        let mut player = PlayerRuntime::connected(1, 0);
        assert!(player.add_pk_hate(777));

        let encoded = player.encode_legacy_ppd_blob(&[]);
        assert_eq!(read_u32(&encoded, 0), DRD_PK_PPD);
        assert_eq!(read_u32(&encoded, 4), LEGACY_PK_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 8 + PK_PPD_HATE_OFFSET), 777);
    }

    #[test]
    fn transport_ppd_blob_round_trips_with_legacy_block_framing() {
        let unknown_id = make_drd(DEV_ID_DB, 22 | PERSISTENT_PLAYER_DATA);
        let mut existing_transport = vec![0; LEGACY_TRANSPORT_PPD_SIZE];
        write_u64(&mut existing_transport, 0, 0x0000_0000_0000_0004);

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
        write_ppd_block(&mut existing, DRD_TRANSPORT_PPD, &existing_transport);

        let mut player = PlayerRuntime::connected(1, 0);
        player.transport_seen = 0x0000_0000_0000_0021;

        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), unknown_id);
        assert_eq!(read_u32(&encoded, 12), DRD_TRANSPORT_PPD);
        assert_eq!(read_u32(&encoded, 16), LEGACY_TRANSPORT_PPD_SIZE as u32);
        assert_eq!(read_u64(&encoded, 20), 0x0000_0000_0000_0021);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(decoded.transport_seen, 0x0000_0000_0000_0021);
    }

    #[test]
    fn ppd_blob_appends_transport_without_existing_block() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.touch_transport(5);

        let encoded = player.encode_legacy_ppd_blob(&[]);
        assert_eq!(read_u32(&encoded, 0), DRD_TRANSPORT_PPD);
        assert_eq!(read_u32(&encoded, 4), LEGACY_TRANSPORT_PPD_SIZE as u32);
        assert_eq!(read_u64(&encoded, 8), 1_u64 << 5);
    }

    #[test]
    fn lostcon_ppd_blob_round_trips_with_legacy_block_framing() {
        let unknown_id = make_drd(DEV_ID_DB, 22 | PERSISTENT_PLAYER_DATA);
        let mut existing_lostcon = vec![0; LEGACY_LOSTCON_PPD_SIZE];
        write_i32(&mut existing_lostcon, LOSTCON_PPD_MAXLAG_OFFSET, 9);

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
        write_ppd_block(&mut existing, DRD_LOSTCON_PPD, &existing_lostcon);

        let mut player = PlayerRuntime::connected(1, 0);
        player.set_max_lag_seconds(19);

        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), unknown_id);
        assert_eq!(read_u32(&encoded, 12), DRD_LOSTCON_PPD);
        assert_eq!(read_u32(&encoded, 16), LEGACY_LOSTCON_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 20 + LOSTCON_PPD_MAXLAG_OFFSET), 19);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(decoded.max_lag_seconds, 19);
    }

    #[test]
    fn ppd_blob_appends_lostcon_without_existing_block() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.set_max_lag_seconds(20);

        let encoded = player.encode_legacy_ppd_blob(&[]);
        assert_eq!(read_u32(&encoded, 0), DRD_LOSTCON_PPD);
        assert_eq!(read_u32(&encoded, 4), LEGACY_LOSTCON_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 8 + LOSTCON_PPD_MAXLAG_OFFSET), 20);
    }

    #[test]
    fn transport_discovery_marks_legacy_exploration_achievement_thresholds() {
        let mut player = PlayerRuntime::connected(1, 0);
        for point in [0, 2, 9, 21, 22, 23, 24] {
            assert!(player.touch_transport(point));
        }
        assert!(!player.achievements.traveller_of_astonia);

        assert!(player.touch_transport(25));
        assert!(player.achievements.traveller_of_astonia);

        let mut underground = PlayerRuntime::connected(2, 0);
        for point in 3..=7 {
            assert!(underground.touch_transport(point));
        }
        assert!(!underground.achievements.underground_explorer);
        assert!(underground.touch_transport(8));
        assert!(underground.achievements.underground_explorer);

        let mut explorer = PlayerRuntime::connected(3, 0);
        for point in 0..=25 {
            if ![11, 18, 19].contains(&point) {
                assert!(explorer.touch_transport(point));
            }
        }
        assert!(explorer.achievements.explorer_of_astonia);
        assert_eq!(explorer.transport_seen & !TRANSPORT_ALL_TELEPORTS_MASK, 0);
    }

    #[test]
    fn orbspawn_ppd_blob_round_trips_with_legacy_block_framing() {
        let unknown_id = make_drd(DEV_ID_DB, 22 | PERSISTENT_PLAYER_DATA);
        let mut existing_orbspawn = vec![0; LEGACY_ORBSPAWN_PPD_SIZE];
        write_i32(&mut existing_orbspawn, ORBSPAWN_PPD_IDS_OFFSET, 0x0001_0203);
        write_i32(&mut existing_orbspawn, ORBSPAWN_PPD_LAST_USED_OFFSET, 44);

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
        write_ppd_block(&mut existing, DRD_ORBSPAWN_PPD, &existing_orbspawn);

        let mut player = PlayerRuntime::connected(1, 0);
        player.mark_orb_spawn_used(0x0001_0506, 777);

        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), unknown_id);
        assert_eq!(read_u32(&encoded, 12), DRD_ORBSPAWN_PPD);
        assert_eq!(read_u32(&encoded, 16), LEGACY_ORBSPAWN_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 20), 0x0001_0506);
        assert_eq!(read_i32(&encoded, 20 + ORBSPAWN_PPD_LAST_USED_OFFSET), 777);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(decoded.orb_spawn_last_used_seconds(0x0001_0506), Some(777));
        assert_eq!(decoded.orb_spawn_last_used_seconds(0x0001_0203), None);
    }

    #[test]
    fn ppd_blob_appends_orbspawns_without_existing_block() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.mark_orb_spawn_used(0x0001_0203, 55);

        let encoded = player.encode_legacy_ppd_blob(&[]);
        assert_eq!(read_u32(&encoded, 0), DRD_ORBSPAWN_PPD);
        assert_eq!(read_u32(&encoded, 4), LEGACY_ORBSPAWN_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 8), 0x0001_0203);
        assert_eq!(read_i32(&encoded, 8 + ORBSPAWN_PPD_LAST_USED_OFFSET), 55);
    }

    #[test]
    fn demonshrine_touch_updates_value_exp_and_blocks_repeats() {
        let mut player = PlayerRuntime::connected(1, 0);
        let mut character = character(3);
        character.exp = 10_000;

        assert_eq!(
            player.touch_demonshrine(&mut character, 0x0001_0203),
            DemonShrineResult::Learned { exp_added: 350 }
        );
        assert_eq!(character.values[1][CharacterValue::Demon as usize], 1);
        assert_eq!(character.exp, 10_350);
        assert!(character.flags.contains(CharacterFlags::UPDATE));
        assert!(character.flags.contains(CharacterFlags::ITEMS));
        assert_eq!(
            player.touch_demonshrine(&mut character, 0x0001_0203),
            DemonShrineResult::AlreadyKnown
        );
    }

    #[test]
    fn demonshrine_ppd_blob_round_trips_with_legacy_block_framing() {
        let unknown_id = make_drd(DEV_ID_DB, 22 | PERSISTENT_PLAYER_DATA);
        let mut existing_demonshrine = vec![0; LEGACY_DEMONSHRINE_PPD_SIZE];
        write_i32(&mut existing_demonshrine, 0, 0x0001_0203);

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, unknown_id, &[1, 2, 3, 4]);
        write_ppd_block(&mut existing, DRD_DEMONSHRINE_PPD, &existing_demonshrine);

        let mut player = PlayerRuntime::connected(1, 0);
        player.demonshrines.push(0x0001_0506);

        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), unknown_id);
        assert_eq!(read_u32(&encoded, 12), DRD_DEMONSHRINE_PPD);
        assert_eq!(read_u32(&encoded, 16), LEGACY_DEMONSHRINE_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 20), 0x0001_0506);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(decoded.demonshrines, vec![0x0001_0506]);
    }

    #[test]
    fn xmas_tree_touch_resets_by_event_year_and_blocks_repeats() {
        let mut player = PlayerRuntime::connected(1, 0);

        assert_eq!(
            player.touch_xmas_tree(1, 2025, false, true),
            XmasTreeResult::Dormant
        );
        assert_eq!(
            player.touch_xmas_tree(1, 2025, true, false),
            XmasTreeResult::NeedsHolidayTreat
        );
        assert_eq!(
            player.touch_xmas_tree(1, 2025, true, true),
            XmasTreeResult::GiftGranted
        );
        assert_eq!(
            player.touch_xmas_tree(1, 2025, true, true),
            XmasTreeResult::AlreadyGranted
        );
        assert_eq!(
            player.touch_xmas_tree(1, 2026, true, true),
            XmasTreeResult::GiftGranted
        );
        assert_eq!(read_i32(&player.misc_ppd, MISC_PPD_GIFT_YEAR_OFFSET), 2026);
        assert_eq!(player.misc_ppd[MISC_PPD_TREEDONE_OFFSET], 0b0000_0010);
    }

    #[test]
    fn misc_ppd_blob_preserves_non_tree_legacy_fields() {
        let mut existing_misc = vec![0; LEGACY_MISC_PPD_SIZE];
        write_i32(&mut existing_misc, 0, 123);
        write_i32(&mut existing_misc, 20, 456);
        write_i32(&mut existing_misc, MISC_PPD_GIFT_YEAR_OFFSET, 2024);

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, DRD_MISC_PPD, &existing_misc);

        let mut player = PlayerRuntime::connected(1, 0);
        assert!(player.decode_legacy_ppd_blob(&existing));
        assert_eq!(
            player.touch_xmas_tree(2, 2025, true, true),
            XmasTreeResult::GiftGranted
        );

        let encoded = player.encode_legacy_ppd_blob(&existing);
        assert_eq!(read_u32(&encoded, 0), DRD_MISC_PPD);
        assert_eq!(read_u32(&encoded, 4), LEGACY_MISC_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 8), 123);
        assert_eq!(read_i32(&encoded, 28), 456);
        assert_eq!(encoded[8 + MISC_PPD_TREEDONE_OFFSET], 0b0000_0100);
        assert_eq!(read_i32(&encoded, 8 + MISC_PPD_GIFT_YEAR_OFFSET), 2025);
    }

    #[test]
    fn flower_ppd_codec_matches_legacy_fixed_arrays() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.mark_flower_used(0x001f_2030, 1234);
        player.mark_flower_used(0x001f_2031, 5678);

        let encoded = player.encode_legacy_flower_ppd();

        assert_eq!(encoded.len(), LEGACY_FLOWER_PPD_SIZE);
        assert_eq!(read_i32(&encoded, FLOWER_PPD_IDS_OFFSET), 0x001f_2030);
        assert_eq!(read_i32(&encoded, FLOWER_PPD_IDS_OFFSET + 4), 0x001f_2031);
        assert_eq!(read_i32(&encoded, FLOWER_PPD_LAST_USED_OFFSET), 1234);
        assert_eq!(read_i32(&encoded, FLOWER_PPD_LAST_USED_OFFSET + 4), 5678);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_flower_ppd(&encoded));
        assert_eq!(decoded.flower_last_used_seconds(0x001f_2030), Some(1234));
        assert_eq!(decoded.flower_last_used_seconds(0x001f_2031), Some(5678));
    }

    #[test]
    fn flower_ppd_blob_replaces_and_appends_legacy_block() {
        let mut existing = Vec::new();
        write_ppd_block(&mut existing, 0x1122_3344, &[1, 2, 3]);

        let mut player = PlayerRuntime::connected(1, 0);
        player.mark_flower_used(7, 99);
        let encoded = player.encode_legacy_ppd_blob(&existing);

        assert_eq!(read_u32(&encoded, 0), 0x1122_3344);
        assert_eq!(read_u32(&encoded, 11), DRD_FLOWER_PPD);
        assert_eq!(read_u32(&encoded, 15), LEGACY_FLOWER_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 19), 7);
        assert_eq!(read_i32(&encoded, 19 + FLOWER_PPD_LAST_USED_OFFSET), 99);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(decoded.flower_last_used_seconds(7), Some(99));
    }

    #[test]
    fn rune_special_exec_generation_matches_legacy_constraints() {
        let mut player = PlayerRuntime::connected(1, 0);
        let mut seed = 0_u32;
        player.ensure_rune_special_execs(|limit| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            seed % limit
        });

        for level in 5..10_u32 {
            let base = (level - 5) as usize * 5;
            let mut seen = Vec::new();
            for value in player.rune_special_exec[base..base + 5].iter().copied() {
                assert!(value >= 100);
                assert!(
                    ![555, 55, 5, 666, 66, 6, 777, 77, 7, 888, 88, 8, 999, 99, 9].contains(&value)
                );
                let digits = format!("{value:03}");
                assert!(digits
                    .chars()
                    .all(|ch| ch != '0' && ch <= char::from_digit(level, 10).unwrap()));
                assert!(digits
                    .chars()
                    .any(|ch| ch == char::from_digit(level, 10).unwrap()));
                assert!(!seen.contains(&value));
                seen.push(value);
            }
        }
    }

    #[test]
    fn bone_hint_uses_generated_special_exec_digit() {
        let mut player = PlayerRuntime::connected(1, 0);
        player.rune_special_exec[0] = 511;
        player.rune_special_exec[(7 - 5) * 5 + 2] = 731;

        assert_eq!(
            player.bone_hint(7, 2, 1, |_| 0),
            BoneHintResult::Hint {
                page: 72,
                rune: "Dagaz",
                position: "second",
            }
        );
    }

    #[test]
    fn rune_ppd_blob_replaces_and_appends_legacy_block() {
        let mut existing_rune = vec![0; LEGACY_RUNE_PPD_SIZE];
        write_u32(&mut existing_rune, 0, 0x8000_0001);
        write_i32(&mut existing_rune, RUNE_PPD_SPECIAL_EXEC_OFFSET, 555);

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, 0x1122_3344, &[1, 2, 3]);
        write_ppd_block(&mut existing, DRD_RUNE_PPD, &existing_rune);

        let mut player = PlayerRuntime::connected(1, 0);
        player.rune_used_words[0] = 0x8000_0002;
        player.rune_special_exec[0] = 654;
        let encoded = player.encode_legacy_ppd_blob(&existing);

        assert_eq!(read_u32(&encoded, 0), 0x1122_3344);
        assert_eq!(read_u32(&encoded, 11), DRD_RUNE_PPD);
        assert_eq!(read_u32(&encoded, 15), LEGACY_RUNE_PPD_SIZE as u32);
        assert_eq!(read_u32(&encoded, 19), 0x8000_0002);
        assert_eq!(read_i32(&encoded, 19 + RUNE_PPD_SPECIAL_EXEC_OFFSET), 654);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(decoded.rune_used_words[0], 0x8000_0002);
        assert_eq!(decoded.rune_special_exec[0], 654);
    }

    #[test]
    fn area3_ppd_tracks_park_shrine_memorization() {
        let mut player = PlayerRuntime::connected(1, 0);

        assert_eq!(
            DRD_AREA3_PPD,
            make_drd(DEV_ID_DB, 40 | PERSISTENT_PLAYER_DATA)
        );
        assert_eq!(player.memorize_park_shrine(2), Some(true));
        assert_eq!(player.memorize_park_shrine(2), Some(false));
        assert_eq!(player.memorize_park_shrine(4), None);

        let encoded = player.encode_legacy_area3_ppd();
        assert_eq!(encoded.len(), LEGACY_AREA3_PPD_SIZE);
        assert_eq!(read_i32(&encoded, AREA3_PPD_KELLY_FOUND1_OFFSET), 0);
        assert_eq!(read_i32(&encoded, AREA3_PPD_KELLY_FOUND2_OFFSET), 1);
        assert_eq!(read_i32(&encoded, AREA3_PPD_KELLY_FOUND3_OFFSET), 0);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_area3_ppd(&encoded));
        assert_eq!(decoded.memorize_park_shrine(2), Some(false));
        assert_eq!(decoded.memorize_park_shrine(3), Some(true));
    }

    #[test]
    fn area3_ppd_blob_replaces_and_appends_legacy_block() {
        let mut existing_area3 = vec![0; LEGACY_AREA3_PPD_SIZE];
        write_i32(&mut existing_area3, AREA3_PPD_KELLY_FOUND1_OFFSET, 1);

        let mut existing = Vec::new();
        write_ppd_block(&mut existing, 0x1122_3344, &[1, 2, 3]);
        write_ppd_block(&mut existing, DRD_AREA3_PPD, &existing_area3);

        let mut player = PlayerRuntime::connected(1, 0);
        assert_eq!(player.memorize_park_shrine(3), Some(true));
        let encoded = player.encode_legacy_ppd_blob(&existing);

        assert_eq!(read_u32(&encoded, 0), 0x1122_3344);
        assert_eq!(read_u32(&encoded, 11), DRD_AREA3_PPD);
        assert_eq!(read_u32(&encoded, 15), LEGACY_AREA3_PPD_SIZE as u32);
        assert_eq!(read_i32(&encoded, 19 + AREA3_PPD_KELLY_FOUND1_OFFSET), 0);
        assert_eq!(read_i32(&encoded, 19 + AREA3_PPD_KELLY_FOUND3_OFFSET), 1);

        let mut decoded = PlayerRuntime::connected(2, 0);
        assert!(decoded.decode_legacy_ppd_blob(&encoded));
        assert_eq!(decoded.memorize_park_shrine(3), Some(false));

        let appended = player.encode_legacy_ppd_blob(&[]);
        assert_eq!(read_u32(&appended, 0), DRD_AREA3_PPD);
    }

    #[test]
    fn malformed_ppd_blob_is_rejected() {
        let mut player = PlayerRuntime::connected(1, 0);
        let mut malformed = Vec::new();
        malformed.extend_from_slice(&DRD_KEYRING_PPD.to_le_bytes());
        malformed.extend_from_slice(&(LEGACY_KEYRING_PPD_SIZE as u32).to_le_bytes());
        malformed.extend_from_slice(&[0; 7]);

        assert!(!player.decode_legacy_ppd_blob(&malformed));
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

    fn character(id: u32) -> Character {
        Character {
            id: CharacterId(id),
            name: "Character".into(),
            description: String::new(),
            flags: CharacterFlags::USED,
            sprite: 0,
            c1: 0,
            c2: 0,
            c3: 0,
            driver: 0,
            group: 0,
            clan: 0,
            clan_rank: 0,
            clan_serial: 0,
            speed_mode: crate::entity::SpeedMode::Normal,
            x: 0,
            y: 0,
            rest_area: 0,
            rest_x: 0,
            rest_y: 0,
            tox: 0,
            toy: 0,
            dir: 0,
            action: 0,
            duration: 0,
            step: 0,
            act1: 0,
            act2: 0,
            hp: 0,
            mana: 0,
            endurance: 0,
            lifeshield: 0,
            level: 1,
            exp: 0,
            exp_used: 0,
            gold: 0,
            creation_time: 0,
            saves: 0,
            deaths: 0,
            regen_ticker: 0,
            cursor_item: None,
            current_container: None,
            values: Character::empty_values(),
            professions: Character::empty_professions(),
            inventory: Character::empty_inventory(),
            driver_state: None,
            driver_messages: Vec::new(),
        }
    }
}
