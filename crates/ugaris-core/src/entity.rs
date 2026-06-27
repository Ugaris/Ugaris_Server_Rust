use bitflags::bitflags;
use serde::{Deserialize, Serialize};

use crate::character_driver::{CharacterDriverMessage, CharacterDriverState};
use crate::ids::{CharacterId, ItemId};

pub const CHARACTER_NAME_SIZE: usize = 40;
pub const ITEM_NAME_SIZE: usize = 40;
pub const ITEM_DESCRIPTION_SIZE: usize = 80;
pub const CHARACTER_DESCRIPTION_SIZE: usize = 160;
pub const INVENTORY_SIZE: usize = 110;
pub const MAX_MODIFIERS: usize = 5;
pub const POWERSCALE: i32 = 1000;
pub const CHARACTER_VALUE_COUNT: usize = 43;
pub const PROFESSION_COUNT: usize = 20;

pub const CHARACTER_VALUE_NAMES: [&str; CHARACTER_VALUE_COUNT] = [
    "Hitpoints",
    "Endurance",
    "Mana",
    "Wisdom",
    "Intuition",
    "Agility",
    "Strength",
    "Armor Value",
    "Weapon Value",
    "Light",
    "Speed",
    "Pulse",
    "Dagger",
    "Hand to Hand",
    "Staff",
    "Sword",
    "Two-Handed",
    "Armor Skill",
    "Attack",
    "Parry",
    "War Cry",
    "Tactics",
    "Surround Hit",
    "Body Control",
    "Speed Skill",
    "Bartering",
    "Perception",
    "Stealth",
    "Bless",
    "Heal",
    "Freeze",
    "Magic Shield",
    "Lightning",
    "Fire",
    "Empty",
    "Regenerate",
    "Meditate",
    "Immunity",
    "Ancient Power",
    "Duration",
    "Rage",
    "Cold Resistance",
    "Profession",
];

pub const PROFESSION_NAMES: [&str; 12] = [
    "Athlete",
    "Alchemist",
    "Miner",
    "Assassin",
    "Thief",
    "Light Warrior",
    "Dark Warrior",
    "Master Trader",
    "Mercenary",
    "Clan Warrior",
    "Herbalist",
    "Demon",
];

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct CharacterFlags: u64 {
        const USED = 1 << 0;
        const IMMORTAL = 1 << 1;
        const GOD = 1 << 2;
        const PLAYER = 1 << 3;
        const STAFF = 1 << 4;
        const INVISIBLE = 1 << 5;
        const SHUTUP = 1 << 6;
        const KICKED = 1 << 7;
        const UPDATE = 1 << 8;
        const RESERVED0 = 1 << 9;
        const RESERVED1 = 1 << 10;
        const DEAD = 1 << 11;
        const ITEMS = 1 << 12;
        const RESPAWN = 1 << 13;
        const MALE = 1 << 14;
        const FEMALE = 1 << 15;
        const WARRIOR = 1 << 16;
        const MAGE = 1 << 17;
        const ARCH = 1 << 18;
        const RESERVED2 = 1 << 19;
        const NOATTACK = 1 << 20;
        const HASNAME = 1 << 21;
        const QUESTITEM = 1 << 22;
        const INFRARED = 1 << 23;
        const PK = 1 << 24;
        const ITEMDEATH = 1 << 25;
        const NODEATH = 1 << 26;
        const NOBODY = 1 << 27;
        const EDEMON = 1 << 28;
        const FDEMON = 1 << 29;
        const IDEMON = 1 << 30;
        const NOGIVE = 1 << 31;
        const PLAYERLIKE = 1 << 32;
        const RESERVED3 = 1 << 33;
        const PAID = 1 << 34;
        const PROF = 1 << 35;
        const ALIVE = 1 << 36;
        const DEMON = 1 << 37;
        const UNDEAD = 1 << 38;
        const HARDKILL = 1 << 39;
        const NOBLESS = 1 << 40;
        const AREACHANGE = 1 << 41;
        const LAG = 1 << 42;
        const RESERVED4 = 1 << 43;
        const THIEFMODE = 1 << 44;
        const NOTELL = 1 << 45;
        const INFRAVISION = 1 << 46;
        const NOMAGIC = 1 << 47;
        const NONOMAGIC = 1 << 48;
        const OXYGEN = 1 << 49;
        const NOPLRATT = 1 << 50;
        const ALLOWSWAP = 1 << 51;
        const LQMASTER = 1 << 52;
        const HARDCORE = 1 << 53;
        const NONOTIFY = 1 << 54;
        const SMALLUPDATE = 1 << 55;
        const NOWHO = 1 << 56;
        const WON = 1 << 57;
        const NOEXP = 1 << 58;
        const DEVELOPER = 1 << 59;
        const EVENTMASTER = 1 << 60;
        const XRAY = 1 << 61;
        const NOLEVEL = 1 << 62;
        const SPY = 1 << 63;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ItemFlags: u64 {
        const USED = 1 << 0;
        const MOVEBLOCK = 1 << 1;
        const SIGHTBLOCK = 1 << 2;
        const TAKE = 1 << 3;
        const USE = 1 << 4;
        const WNHEAD = 1 << 5;
        const WNNECK = 1 << 6;
        const WNBODY = 1 << 7;
        const WNARMS = 1 << 8;
        const WNBELT = 1 << 9;
        const WNLEGS = 1 << 10;
        const WNFEET = 1 << 11;
        const WNLHAND = 1 << 12;
        const WNRHAND = 1 << 13;
        const WNCLOAK = 1 << 14;
        const WNLRING = 1 << 15;
        const WNRRING = 1 << 16;
        const WNTWOHANDED = 1 << 17;
        const AXE = 1 << 18;
        const DAGGER = 1 << 19;
        const HAND = 1 << 20;
        const SHIELD = 1 << 21;
        const STAFF = 1 << 22;
        const SWORD = 1 << 23;
        const TWOHAND = 1 << 24;
        const DOOR = 1 << 25;
        const QUEST = 1 << 26;
        const SOUNDBLOCK = 1 << 27;
        const STEPACTION = 1 << 28;
        const MONEY = 1 << 29;
        const NODECAY = 1 << 30;
        const FRONTWALL = 1 << 31;
        const DEPOT = 1 << 32;
        const NODEPOT = 1 << 33;
        const NODROP = 1 << 34;
        const NOJUNK = 1 << 35;
        const PLAYERBODY = 1 << 36;
        const BONDTAKE = 1 << 37;
        const BONDWEAR = 1 << 38;
        const LABITEM = 1 << 39;
        const VOID = 1 << 40;
        const NOENHANCE = 1 << 41;
        const BEYONDBOUNDS = 1 << 42;
        const BEYONDMAXMOD = 1 << 43;
        const ENGRAVED = 1 << 44;
        const GIVEN_ITEM = 1 << 45;
        const FORCEUPDATE = 1 << 46;
    }
}

impl ItemFlags {
    pub const WEAPON: Self = Self::from_bits_retain(
        Self::AXE.bits()
            | Self::DAGGER.bits()
            | Self::HAND.bits()
            | Self::STAFF.bits()
            | Self::SWORD.bits()
            | Self::TWOHAND.bits(),
    );

    pub const WEAR: Self = Self::from_bits_retain(
        Self::WNHEAD.bits()
            | Self::WNNECK.bits()
            | Self::WNBODY.bits()
            | Self::WNARMS.bits()
            | Self::WNBELT.bits()
            | Self::WNLEGS.bits()
            | Self::WNFEET.bits()
            | Self::WNLHAND.bits()
            | Self::WNRHAND.bits()
            | Self::WNCLOAK.bits()
            | Self::WNLRING.bits()
            | Self::WNRRING.bits(),
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum CharacterValue {
    Hp = 0,
    Endurance = 1,
    Mana = 2,
    Wisdom = 3,
    Intelligence = 4,
    Agility = 5,
    Strength = 6,
    Armor = 7,
    Weapon = 8,
    Light = 9,
    Speed = 10,
    Pulse = 11,
    Dagger = 12,
    Hand = 13,
    Staff = 14,
    Sword = 15,
    TwoHand = 16,
    ArmorSkill = 17,
    Attack = 18,
    Parry = 19,
    Warcry = 20,
    Tactics = 21,
    Surround = 22,
    BodyControl = 23,
    SpeedSkill = 24,
    Barter = 25,
    Percept = 26,
    Stealth = 27,
    Bless = 28,
    Heal = 29,
    Freeze = 30,
    MagicShield = 31,
    Flash = 32,
    Fireball = 33,
    Empty = 34,
    Regenerate = 35,
    Meditate = 36,
    Immunity = 37,
    Demon = 38,
    Duration = 39,
    Rage = 40,
    Cold = 41,
    Profession = 42,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum SpeedMode {
    Normal = 0,
    Fast = 1,
    Stealth = 2,
}

impl Default for SpeedMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: CharacterId,
    pub name: String,
    pub description: String,
    pub flags: CharacterFlags,
    pub sprite: i32,
    #[serde(default)]
    pub driver: u16,
    #[serde(default)]
    pub group: u16,
    pub speed_mode: SpeedMode,
    pub x: u16,
    pub y: u16,
    pub rest_area: u16,
    pub rest_x: u16,
    pub rest_y: u16,
    pub tox: u16,
    pub toy: u16,
    pub dir: u8,
    pub action: u16,
    pub duration: i32,
    pub step: i32,
    pub act1: i32,
    pub act2: i32,
    pub hp: i32,
    pub mana: i32,
    pub endurance: i32,
    pub lifeshield: i32,
    pub level: u32,
    pub exp: u32,
    pub exp_used: u32,
    pub gold: u32,
    #[serde(default)]
    pub creation_time: u64,
    #[serde(default)]
    pub saves: u8,
    #[serde(default)]
    pub deaths: u32,
    pub cursor_item: Option<ItemId>,
    pub current_container: Option<ItemId>,
    pub values: Vec<Vec<i16>>,
    pub professions: Vec<i16>,
    pub inventory: Vec<Option<ItemId>>,
    #[serde(default)]
    pub driver_state: Option<CharacterDriverState>,
    #[serde(default)]
    pub driver_messages: Vec<CharacterDriverMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: ItemId,
    pub name: String,
    pub description: String,
    pub flags: ItemFlags,
    pub sprite: i32,
    pub value: u32,
    pub min_level: u8,
    pub max_level: u8,
    pub needs_class: u8,
    #[serde(default)]
    pub template_id: u32,
    pub owner_id: i32,
    pub modifier_index: [i16; MAX_MODIFIERS],
    pub modifier_value: [i16; MAX_MODIFIERS],
    pub x: u16,
    pub y: u16,
    pub carried_by: Option<CharacterId>,
    pub contained_in: Option<ItemId>,
    pub content_id: u16,
    pub driver: u16,
    pub driver_data: Vec<u8>,
    pub serial: u32,
}

impl Character {
    pub fn empty_inventory() -> Vec<Option<ItemId>> {
        vec![None; INVENTORY_SIZE]
    }

    pub fn empty_values() -> Vec<Vec<i16>> {
        vec![vec![0; CHARACTER_VALUE_COUNT]; 2]
    }

    pub fn empty_professions() -> Vec<i16> {
        vec![0; PROFESSION_COUNT]
    }

    pub fn push_driver_message(&mut self, message_type: i32, dat1: i32, dat2: i32, dat3: i32) {
        self.driver_messages.push(CharacterDriverMessage {
            message_type,
            dat1,
            dat2,
            dat3,
        });
    }

    pub fn purge_driver_messages(&mut self) {
        self.driver_messages.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn character_flags_match_c_header_positions() {
        assert_eq!(CharacterFlags::PLAYER.bits(), 1_u64 << 3);
        assert_eq!(CharacterFlags::HARDCORE.bits(), 1_u64 << 53);
        assert_eq!(CharacterFlags::SPY.bits(), 1_u64 << 63);
    }

    #[test]
    fn item_flags_match_c_header_positions() {
        assert_eq!(ItemFlags::TAKE.bits(), 1_u64 << 3);
        assert_eq!(ItemFlags::DEPOT.bits(), 1_u64 << 32);
        assert_eq!(ItemFlags::FORCEUPDATE.bits(), 1_u64 << 46);
    }

    #[test]
    fn value_and_profession_names_match_character_json_tables() {
        assert_eq!(CHARACTER_VALUE_NAMES[0], "Hitpoints");
        assert_eq!(CHARACTER_VALUE_NAMES[33], "Fire");
        assert_eq!(CHARACTER_VALUE_NAMES[42], "Profession");
        assert_eq!(PROFESSION_NAMES[0], "Athlete");
        assert_eq!(PROFESSION_NAMES[11], "Demon");
    }

    #[test]
    fn legacy_character_snapshots_default_driver_runtime_fields() {
        let json = r#"{
            "id": 1,
            "name": "Rat",
            "description": "",
            "flags": "USED",
            "sprite": 1,
            "speed_mode": "Normal",
            "x": 10,
            "y": 11,
            "rest_area": 1,
            "rest_x": 10,
            "rest_y": 11,
            "tox": 0,
            "toy": 0,
            "dir": 0,
            "action": 0,
            "duration": 0,
            "step": 0,
            "act1": 0,
            "act2": 0,
            "hp": 1000,
            "mana": 0,
            "endurance": 0,
            "lifeshield": 0,
            "level": 1,
            "exp": 0,
            "exp_used": 0,
            "gold": 0,
            "cursor_item": null,
            "current_container": null,
            "values": [],
            "professions": [],
            "inventory": []
        }"#;

        let character: Character = serde_json::from_str(json).unwrap();

        assert!(character.driver_state.is_none());
        assert!(character.driver_messages.is_empty());
        assert_eq!(character.driver, 0);
    }

    #[test]
    fn driver_message_queue_preserves_legacy_payload_order() {
        let mut character = Character {
            id: CharacterId(1),
            name: String::new(),
            description: String::new(),
            flags: CharacterFlags::USED,
            sprite: 0,
            driver: 0,
            group: 0,
            speed_mode: SpeedMode::Normal,
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
            level: 0,
            exp: 0,
            exp_used: 0,
            gold: 0,
            creation_time: 0,
            saves: 0,
            deaths: 0,
            cursor_item: None,
            current_container: None,
            values: Character::empty_values(),
            professions: Character::empty_professions(),
            inventory: Character::empty_inventory(),
            driver_state: None,
            driver_messages: Vec::new(),
        };

        character.push_driver_message(crate::character_driver::NT_CREATE, 1, 2, 3);
        character.push_driver_message(crate::character_driver::NT_GOTHIT, 4, 5, 6);

        assert_eq!(character.driver_messages[0].message_type, 9);
        assert_eq!(character.driver_messages[1].dat1, 4);
        character.purge_driver_messages();
        assert!(character.driver_messages.is_empty());
    }
}
