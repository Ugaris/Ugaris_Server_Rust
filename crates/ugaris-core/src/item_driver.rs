use serde::{Deserialize, Serialize};

use crate::{
    do_action::ItemUseRequest,
    entity::{
        Character, CharacterFlags, CharacterValue, Item, ItemFlags, CHARACTER_VALUE_COUNT,
        MAX_MODIFIERS, POWERSCALE,
    },
    ids::{CharacterId, ItemId},
    item_ops::consume_item,
    legacy::{action, MAX_MAP},
    text::{COL_DARK_GRAY, COL_LIGHT_GREEN, COL_RESET},
    tick::TICKS_PER_SECOND,
};

pub const IDR_POTION: u16 = 1;
pub const IDR_DOOR: u16 = 2;
pub const IDR_BALLTRAP: u16 = 3;
pub const IDR_CHEST: u16 = 5;
pub const IDR_USETRAP: u16 = 6;
pub const IDR_PALACEGATE: u16 = 9;
pub const IDR_TELEPORT: u16 = 10;
pub const IDR_NIGHTLIGHT: u16 = 11;
pub const IDR_TORCH: u16 = 12;
pub const IDR_RECALL: u16 = 13;
pub const IDR_SHRINE: u16 = 14;
pub const IDR_FIREBALL: u16 = 15;
pub const IDR_BOOK: u16 = 16;
pub const BOOK_NOOK_JOKES: u8 = 48;
pub const IDR_ONOFFLIGHT: u16 = 17;
pub const IDR_TRANSPORT: u16 = 18;
pub const IDR_STATSCROLL: u16 = 19;
pub const IDR_CLANSPAWN: u16 = 20;
pub const IDR_CLANJEWEL: u16 = 21;
pub const IDR_CLANVAULT: u16 = 22;
pub const IDR_PARKSHRINE: u16 = 23;
pub const IDR_FLAMETHROW: u16 = 24;
pub const IDR_STEPTRAP: u16 = 25;
pub const IDR_SPIKETRAP: u16 = 26;
pub const IDR_CHESTSPAWN: u16 = 27;
pub const IDR_EXTINGUISH: u16 = 28;
pub const IDR_ASSEMBLE: u16 = 29;
pub const IDR_PENT: u16 = 30;
pub const IDR_TELE_DOOR: u16 = 31;
pub const IDR_FLASK: u16 = 32;
pub const IDR_FLOWER: u16 = 33;
pub const IDR_RANDCHEST: u16 = 34;
pub const IDR_DEMONSHRINE: u16 = 35;
pub const IDR_EDEMONBALL: u16 = 36;
pub const IDR_EDEMONSWITCH: u16 = 37;
pub const IDR_EDEMONGATE: u16 = 38;
pub const IDR_EDEMONLOADER: u16 = 39;
pub const IDR_EDEMONLIGHT: u16 = 40;
pub const IDR_EDEMONDOOR: u16 = 41;
pub const IDR_EDEMONBLOCK: u16 = 42;
pub const IDR_EDEMONTUBE: u16 = 43;
pub const IDR_FDEMONLIGHT: u16 = 44;
pub const IDR_FDEMONLOADER: u16 = 45;
pub const IDR_FDEMONCANNON: u16 = 46;
pub const IDR_FDEMONGATE: u16 = 47;
pub const IDR_FDEMONWAYPOINT: u16 = 48;
pub const IDR_FDEMONFARM: u16 = 49;
pub const IDR_FDEMONBLOOD: u16 = 50;
pub const IDR_FDEMONLAVA: u16 = 51;
pub const IDR_MELTINGKEY: u16 = 52;
pub const IDR_ITEMSPAWN: u16 = 53;
pub const IDR_WARMFIRE: u16 = 54;
pub const IDR_BACKTOFIRE: u16 = 55;
pub const IDR_PALACEBOMB: u16 = 56;
pub const IDR_PALACECAP: u16 = 57;
pub const IDR_FREAKDOOR: u16 = 58;
pub const IDR_PALACEKEY: u16 = 59;
pub const IDR_MINEWALL: u16 = 60;
pub const IDR_ENHANCE: u16 = 61;
pub const IDR_MINEDOOR: u16 = 62;
pub const IDR_TOPLIST: u16 = 63;
pub const IDR_FOOD: u16 = 64;
pub const IDR_DUNGEONTELE: u16 = 65;
pub const IDR_DUNGEONFAKE: u16 = 66;
pub const IDR_DUNGEONDOOR: u16 = 67;
pub const IDR_DUNGEONKEY: u16 = 68;
pub const IDR_RANDOMSHRINE: u16 = 69;
pub const IDR_TRAPDOOR: u16 = 70;
pub const IDR_JUNKPILE: u16 = 71;
pub const IDR_GASTRAP: u16 = 72;
pub const IDR_SWAMPARM: u16 = 73;
pub const IDR_SWAMPWHISP: u16 = 74;
pub const IDR_SWAMPSPAWN: u16 = 75;
pub const IDR_PALACEDOOR: u16 = 76;
pub const IDR_FORESTSPADE: u16 = 77;
pub const IDR_FORESTCHEST: u16 = 78;
pub const IDR_PICKDOOR: u16 = 79;
pub const IDR_PICKCHEST: u16 = 80;
pub const IDR_PENTBOSSDOOR: u16 = 81;
pub const IDR_BURNDOWN: u16 = 82;
pub const IDR_ENCHANTITEM: u16 = 83;
pub const IDR_ORBSPAWN: u16 = 84;
pub const IDR_BOOKCASE: u16 = 85;
pub const IDR_COLORTILE: u16 = 86;
pub const IDR_SKELRAISE: u16 = 87;
pub const IDR_SPECIAL_POTION: u16 = 88;
pub const IDR_BONEBRIDGE: u16 = 89;
pub const IDR_BONELADDER: u16 = 90;
pub const IDR_BONEHOLDER: u16 = 91;
pub const IDR_BONEWALL: u16 = 92;
pub const IDR_INFINITE_CHEST: u16 = 93;
pub const IDR_BONEHINT: u16 = 94;
pub const IDR_NOMADDICE: u16 = 95;
pub const IDR_NOMADSTACK: u16 = 96;
pub const IDR_LFREDUCT: u16 = 97;
pub const IDR_LQ_DOOR: u16 = 98;
pub const IDR_LQ_CHEST: u16 = 99;
pub const IDR_LQ_KEY: u16 = 100;
pub const IDR_LABENTRANCE: u16 = 101;
pub const IDR_LABEXIT: u16 = 102;
pub const IDR_LQ_TICKER: u16 = 103;
pub const IDR_LQ_ENTRANCE: u16 = 104;
pub const IDR_STR_MINE: u16 = 105;
pub const IDR_STR_STORAGE: u16 = 106;
pub const IDR_STR_SPAWNER: u16 = 107;
pub const IDR_STR_DEPOT: u16 = 108;
pub const IDR_STR_TICKER: u16 = 109;
pub const IDR_NOSNOW: u16 = 110;
pub const IDR_RATCHEST: u16 = 111;
pub const IDR_WARPTELEPORT: u16 = 112;
pub const IDR_WARPTRIALDOOR: u16 = 113;
pub const IDR_WARPBONUS: u16 = 114;
pub const IDR_WARPKEYSPAWN: u16 = 115;
pub const IDR_WARPKEYDOOR: u16 = 116;
pub const IDR_TOYLIGHT: u16 = 117;
pub const IDR_SHRIKEAMULET: u16 = 118;
pub const IDR_SHRIKE: u16 = 119;
pub const IDR_WEREWOLF: u16 = 120;
pub const IDR_STAFFER: u16 = 121;
pub const IDR_STAFFER2: u16 = 122;
pub const IDR_BRANNINGTONFOREST: u16 = 123;
pub const IDR_CLANSPAWNEXIT: u16 = 124;
pub const IDR_MINEKEYDOOR: u16 = 125;
pub const IDR_MINEGATEWAYKEY: u16 = 126;
pub const IDR_MINEGATEWAY: u16 = 127;
pub const IDR_OXYPOTION: u16 = 128;
pub const IDR_PICKBERRY: u16 = 129;
pub const IDR_LIZARDFLOWER: u16 = 130;
pub const IDR_MISSIONCHEST: u16 = 131;
pub const IDR_DECAYITEM: u16 = 132;
pub const IDR_BEYONDPOTION: u16 = 133;
pub const IDR_TUNNELDOOR: u16 = 134;
pub const IDR_TUNNELDOOR2: u16 = 135;
pub const IDR_DEMONCHIP: u16 = 136;
pub const IDR_TEUFELDOOR: u16 = 137;
pub const IDR_ISLENADOOR: u16 = 138;
pub const IDR_TEUFELARENA: u16 = 139;
pub const IDR_TEUFELRATNEST: u16 = 140;
pub const IDR_TEUFELARENAEXIT: u16 = 141;
pub const IDR_XMASTREE: u16 = 142;
pub const IDR_XMASMAKER: u16 = 143;
pub const IDR_CALIGAR: u16 = 144;
pub const IDR_CALIGARFLAME: u16 = 145;
pub const IDR_ARKHATA: u16 = 146;
pub const IDR_SPECIAL_SHRINE: u16 = 147;
pub const IDR_ACCOUNT_DEPOT: u16 = 148;
pub const IDR_ANTIENCHANTITEM: u16 = 160;
pub const IDR_SPECIALANTIENCHANTITEM: u16 = 161;
pub const IDR_CITY_RECALL: u16 = 159;
pub const IDR_ANTIORBSPAWN: u16 = 162;
pub const IDR_DOUBLE_DOOR: u16 = 187;
pub const IDR_SALTMINE_ITEM: u16 = 188;
pub const IDR_LAB5_ITEM: u16 = 190;
pub const IDR_LAB4_ITEM: u16 = 191;
pub const IDR_LAB3_SPECIAL: u16 = 192;
pub const IDR_LAB3_PLANT: u16 = 193;
pub const IDR_LAB2_REGENERATE: u16 = 194;
pub const IDR_LAB2_STEPACTION: u16 = 195;
pub const IDR_LAB2_WATER: u16 = 196;
pub const IDR_LAB2_GRAVE: u16 = 197;
pub const IDR_DEATHFIBRIN: u16 = 198;
pub const IDR_LABTORCH: u16 = 199;
pub const IDR_KEY_RING: u16 = 200;
pub const IDR_SKELETON_KEY: u16 = 201;

pub const CLANJEWEL_CHECK_INTERVAL_TICKS: u64 = TICKS_PER_SECOND * 30;
pub const CLANJEWEL_LIFETIME_SECONDS: u32 = 60 * 60;
pub const IID_ALCHEMY_INGREDIENT: u32 = (0x01 << 24) | 0x000043;
pub const IID_AREA18_BONE: u32 = (0x01 << 24) | 0x000077;
pub const IID_SKELETON_KEY: u32 = (59 << 24) | 0x000003;
pub const IID_AREA2_ZOMBIESKULL1: u32 = (0x01 << 24) | 0x000025;
pub const IID_AREA2_ZOMBIESKULL2: u32 = (0x01 << 24) | 0x000026;
pub const IID_AREA2_ZOMBIESKULL3: u32 = (0x01 << 24) | 0x000027;
pub const IID_AREA11_PALACEKEY: u32 = (0x01 << 24) | 0x000050;
pub const IID_AREA11_PALACEKEYPART: u32 = (0x01 << 24) | 0x000051;
pub const IID_AREA6_YELLOWCRYSTAL: u32 = (0x01 << 24) | 0x000049;
pub const IID_AREA17_LIBRARYKEY: u32 = (0x01 << 24) | 0x00006F;
pub const IID_AREA17_BLOODBOWL: u32 = (0x01 << 24) | 0x000071;
pub const IID_AREA17_LOCKPICK: u32 = (0x01 << 24) | 0x000062;
pub const IID_CALIGAR_PALACE_KEY_PART: u32 = (0x01 << 24) | 0x0000B3;
const V_LIGHT: i16 = 9;
const LIGHT_TIMER_TICKS: u64 = TICKS_PER_SECOND * 30;
pub const OUTCOME_ITEM_NAME_BYTES: usize = 32;
pub const LEGACY_TRANSPORT_POINT_COUNT: u8 = 26;
pub const LEGACY_TRANSPORT_CLAN_EXIT: u8 = 255;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoorKeyAccess {
    pub key_id: u32,
    pub name: String,
    pub source: DoorKeySource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DoorKeySource {
    Carried,
    Keyring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssembleTemplate {
    SunAmulet12,
    SunAmulet13,
    SunAmulet23,
    SunAmulet123,
    WarrBluekey12,
    WarrBluekey13,
    WarrBluekey23,
    WarrBluekey123,
    WarrGreenkey12,
    WarrGreenkey13,
    WarrGreenkey23,
    WarrGreenkey123,
    WarrRedkey12,
    WarrRedkey13,
    WarrRedkey23,
    WarrRedkey123,
}

impl AssembleTemplate {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SunAmulet12 => "sun_amulet12",
            Self::SunAmulet13 => "sun_amulet13",
            Self::SunAmulet23 => "sun_amulet23",
            Self::SunAmulet123 => "sun_amulet123",
            Self::WarrBluekey12 => "warr_bluekey12",
            Self::WarrBluekey13 => "warr_bluekey13",
            Self::WarrBluekey23 => "warr_bluekey23",
            Self::WarrBluekey123 => "warr_bluekey123",
            Self::WarrGreenkey12 => "warr_greenkey12",
            Self::WarrGreenkey13 => "warr_greenkey13",
            Self::WarrGreenkey23 => "warr_greenkey23",
            Self::WarrGreenkey123 => "warr_greenkey123",
            Self::WarrRedkey12 => "warr_redkey12",
            Self::WarrRedkey13 => "warr_redkey13",
            Self::WarrRedkey23 => "warr_redkey23",
            Self::WarrRedkey123 => "warr_redkey123",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InfiniteChestTemplate {
    Rune1,
    Rune2,
    Rune3,
    Rune4,
    Rune5,
    Rune6,
    Rune7,
    Rune8,
    Rune9,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForestSpadeFind {
    ForestNote1,
    BranningtonTreasure { dig_index: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PickChestTemplate {
    PalaceNote1,
    PalaceNote2,
    PalaceNote3,
    MerchantNote1,
}

impl PickChestTemplate {
    pub fn from_kind(kind: u8) -> Option<Self> {
        match kind {
            0 => Some(Self::PalaceNote1),
            1 => Some(Self::PalaceNote2),
            2 => Some(Self::PalaceNote3),
            3 => Some(Self::MerchantNote1),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::PalaceNote1 => "palace_note1",
            Self::PalaceNote2 => "palace_note2",
            Self::PalaceNote3 => "palace_note3",
            Self::MerchantNote1 => "merchant_note1",
        }
    }
}

impl InfiniteChestTemplate {
    pub fn from_kind(kind: u8) -> Option<Self> {
        match kind {
            1 => Some(Self::Rune1),
            2 => Some(Self::Rune2),
            3 => Some(Self::Rune3),
            4 => Some(Self::Rune4),
            5 => Some(Self::Rune5),
            6 => Some(Self::Rune6),
            7 => Some(Self::Rune7),
            8 => Some(Self::Rune8),
            9 => Some(Self::Rune9),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rune1 => "rune1",
            Self::Rune2 => "rune2",
            Self::Rune3 => "rune3",
            Self::Rune4 => "rune4",
            Self::Rune5 => "rune5",
            Self::Rune6 => "rune6",
            Self::Rune7 => "rune7",
            Self::Rune8 => "rune8",
            Self::Rune9 => "rune9",
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemDriverContext {
    pub door_key: Option<DoorKeyAccess>,
    pub cursor_template_id: Option<u32>,
    pub cursor_driver: Option<u16>,
    pub cursor_sprite: Option<i32>,
    pub cursor_drdata0: Option<u8>,
    pub timer_call: bool,
    pub daylight: u8,
    pub hour: u8,
    pub fullmoon: bool,
    pub newmoon: bool,
    pub solstice: bool,
    pub equinox: bool,
    pub character_underwater: bool,
    pub current_tick: u32,
    pub edemon_section_power: Option<u8>,
    pub fdemon_loader_power: Option<u16>,
    pub bone_hint_nr: Option<u8>,
    pub bone_hint_pos: Option<u8>,
    pub has_area17_library_key: bool,
    pub has_area17_lockpick: bool,
    pub has_dungeon_door_key1: bool,
    pub has_dungeon_door_key2: bool,
    pub dungeon_defender_count: Option<u16>,
    pub pent_last_solve_tick: Option<u32>,
    pub pent_demon_lord_access_seconds: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UseItemError {
    IllegalCharacter,
    IllegalItem,
    Dead,
    AccessDenied,
    AccountDepotUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemDriverRequest {
    Driver {
        driver: u16,
        item_id: ItemId,
        character_id: CharacterId,
        spec: i32,
    },
    AccountDepot {
        item_id: ItemId,
        character_id: CharacterId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UseItemOutcome {
    OpenContainer { item_id: ItemId },
    OpenDepot { item_id: ItemId },
    OpenAccountDepot { item_id: ItemId },
    Dispatch(ItemDriverRequest),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FdemonLoaderBlockReason {
    CrystalAlreadyPresent,
    CrystalStuck,
    NeedsCrystal,
    WrongCrystal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdemonLoaderBlockReason {
    CrystalAlreadyPresent,
    CrystalStuck,
    NeedsCrystal,
    WrongCrystal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FdemonBloodBlockReason {
    BareHands,
    WrongItem,
    ContainerFull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FdemonCrystalTemplate {
    Small,
    Medium,
    Large,
    Huge,
    Giant,
}

impl FdemonCrystalTemplate {
    pub fn from_farm_size(size: u8) -> Self {
        if size >= 48 {
            Self::Giant
        } else if size >= 40 {
            Self::Huge
        } else if size >= 32 {
            Self::Large
        } else if size >= 24 {
            Self::Medium
        } else {
            Self::Small
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Small => "fdemon_crystal1",
            Self::Medium => "fdemon_crystal2",
            Self::Large => "fdemon_crystal3",
            Self::Huge => "fdemon_crystal4",
            Self::Giant => "fdemon_crystal5",
        }
    }

    pub fn legacy_number(self) -> u8 {
        match self {
            Self::Small => 1,
            Self::Medium => 2,
            Self::Large => 3,
            Self::Huge => 4,
            Self::Giant => 5,
        }
    }

    pub fn foreground_sprite(self) -> u32 {
        match self {
            Self::Small => 59020,
            Self::Medium => 59040,
            Self::Large => 59041,
            Self::Huge => 59042,
            Self::Giant => 59043,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemDriverOutcome {
    LookItem {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PotionDrunk {
        item_id: ItemId,
        character_id: CharacterId,
        hp_added: i32,
        mana_added: i32,
        endurance_added: i32,
    },
    FoodEaten {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
    },
    LollipopLicked {
        item_id: ItemId,
        character_id: CharacterId,
        exp_added: u32,
        lick_count: u8,
    },
    LollipopMemories {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ChristmasPopInspected {
        item_id: ItemId,
        character_id: CharacterId,
    },
    Teleport {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
        area_id: u16,
        stop_driver: bool,
        quiet: bool,
    },
    TeleportDoor {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
    },
    Recall {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
        area_id: u16,
    },
    CityRecall {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
        area_id: u16,
    },
    DungeonTeleport {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
        clan_number: u16,
    },
    DungeonFake {
        item_id: ItemId,
        character_id: CharacterId,
        clan_number: u16,
    },
    DungeonKey {
        item_id: ItemId,
        character_id: CharacterId,
        template: &'static str,
        key_id: u32,
        clan_number: u8,
        first_taken: bool,
    },
    DungeonKeyCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    DungeonDoorMissingKeys {
        item_id: ItemId,
        character_id: CharacterId,
        missing: u8,
        both_required: bool,
    },
    DungeonDoorTooManyDefenders {
        item_id: ItemId,
        character_id: CharacterId,
        alive: u16,
        max_allowed: u16,
    },
    DungeonDoorSolved {
        item_id: ItemId,
        character_id: CharacterId,
        clan_number: u32,
        catacomb: u8,
        first_solve: bool,
    },
    DoorToggle {
        item_id: ItemId,
        character_id: CharacterId,
    },
    KeyedDoorToggle {
        item_id: ItemId,
        character_id: CharacterId,
        key_id: u32,
        source: DoorKeySource,
        locking: bool,
    },
    DoubleDoorToggle {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PickDoorToggle {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PickDoorLocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    StafferSpecDoorToggle {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
    },
    FreakDoorUse {
        item_id: ItemId,
        character_id: CharacterId,
        link_group: u8,
        one_way: bool,
        recursion_guard: bool,
        cached_partner_id: Option<ItemId>,
        no_target: bool,
    },
    StafferSpecDoorLocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BallTrapProjectile {
        item_id: ItemId,
        character_id: CharacterId,
        start_x: u16,
        start_y: u16,
        target_x: u16,
        target_y: u16,
        power: u8,
    },
    FireballMachineProjectile {
        item_id: ItemId,
        character_id: CharacterId,
        start_x: u16,
        start_y: u16,
        target_x: u16,
        target_y: u16,
        power: u8,
        schedule_after_ticks: Option<u64>,
    },
    EdemonBallProjectile {
        item_id: ItemId,
        character_id: CharacterId,
        start_x: u16,
        start_y: u16,
        target_x: u16,
        target_y: u16,
        strength: i32,
        base_sprite: i32,
        schedule_after_ticks: u64,
    },
    CaligarGunProjectile {
        item_id: ItemId,
        character_id: CharacterId,
        direction: u8,
        schedule_after_ticks: u64,
    },
    FlameThrowerPulse {
        item_id: ItemId,
        character_id: CharacterId,
        direction: u8,
        schedule_after_ticks: u64,
    },
    FlameThrowerExtinguished {
        item_id: ItemId,
        character_id: CharacterId,
        schedule_after_ticks: Option<u64>,
    },
    SpikeTrapTriggered {
        item_id: ItemId,
        character_id: CharacterId,
        damage: i32,
        reset_after_ticks: u64,
    },
    SpikeTrapReset {
        item_id: ItemId,
    },
    Extinguish {
        item_id: ItemId,
        character_id: CharacterId,
        extinguished: bool,
    },
    TriggerMapItem {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
        target_character_id: CharacterId,
        delay_ticks: u64,
    },
    StepTrapDiscoverTarget {
        item_id: ItemId,
    },
    ChestTreasure {
        item_id: ItemId,
        character_id: CharacterId,
        treasure_index: u8,
    },
    RandomChest {
        item_id: ItemId,
        character_id: CharacterId,
    },
    InfiniteChest {
        item_id: ItemId,
        character_id: CharacterId,
        template: InfiniteChestTemplate,
        key_name: Option<[u8; OUTCOME_ITEM_NAME_BYTES]>,
    },
    InfiniteChestCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    InfiniteChestKeyRequired {
        item_id: ItemId,
        character_id: CharacterId,
    },
    InfiniteChestUnknown {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ForestSpadeFind {
        item_id: ItemId,
        character_id: CharacterId,
        find: ForestSpadeFind,
    },
    ForestSpadeCollapse {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
    },
    ForestSpadeNothing {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ForestSpadeCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PickChest {
        item_id: ItemId,
        character_id: CharacterId,
        template: PickChestTemplate,
    },
    PickChestCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PickChestLocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PickChestBug {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PentBossDoor {
        item_id: ItemId,
        character_id: CharacterId,
        x: u16,
        y: u16,
    },
    PentBossDoorLocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PentBossDoorBusy {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ColorTile {
        item_id: ItemId,
        character_id: CharacterId,
        row: u8,
        color: u8,
    },
    BurndownTouch {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BurndownTooHot {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BurndownAlreadyBurned {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BurndownIgnite {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BurndownTimerTick {
        item_id: ItemId,
    },
    KeyringShow {
        item_id: ItemId,
        character_id: CharacterId,
    },
    KeyringAddCursorItem {
        item_id: ItemId,
        character_id: CharacterId,
        key_item_id: ItemId,
    },
    LightChanged {
        item_id: ItemId,
        character_id: CharacterId,
        schedule_after_ticks: Option<u64>,
    },
    FdemonLoaderChanged {
        item_id: ItemId,
        character_id: CharacterId,
        consumed_cursor_item_id: Option<ItemId>,
        ground_overlay_sprite: u32,
        sound_type: Option<u32>,
        schedule_after_ticks: Option<u64>,
    },
    FdemonLoaderBlocked {
        item_id: ItemId,
        character_id: CharacterId,
        reason: FdemonLoaderBlockReason,
    },
    EdemonLoaderChanged {
        item_id: ItemId,
        character_id: CharacterId,
        consumed_cursor_item_id: Option<ItemId>,
        ground_overlay_sprite: u32,
        sound_type: Option<u32>,
        schedule_after_ticks: Option<u64>,
    },
    EdemonLoaderBlocked {
        item_id: ItemId,
        character_id: CharacterId,
        reason: EdemonLoaderBlockReason,
    },
    FdemonFarmChanged {
        item_id: ItemId,
        character_id: CharacterId,
        foreground_sprite: u32,
        schedule_after_ticks: Option<u64>,
    },
    FdemonFarmHarvest {
        item_id: ItemId,
        character_id: CharacterId,
        template: FdemonCrystalTemplate,
        foreground_sprite: u32,
    },
    FdemonFarmCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    FdemonFarmNotReady {
        item_id: ItemId,
        character_id: CharacterId,
        current: u8,
        required: u8,
    },
    FdemonFarmBug {
        item_id: ItemId,
        character_id: CharacterId,
        crystal_number: u8,
    },
    FdemonBloodBlocked {
        item_id: ItemId,
        character_id: CharacterId,
        reason: FdemonBloodBlockReason,
    },
    FdemonBloodDestroyedFlask {
        item_id: ItemId,
        character_id: CharacterId,
        flask_item_id: ItemId,
    },
    FdemonBloodFilled {
        item_id: ItemId,
        character_id: CharacterId,
        container_item_id: ItemId,
        amount: u8,
    },
    EdemonSwitchStuck {
        item_id: ItemId,
        character_id: CharacterId,
    },
    OnOffLightChanged {
        item_id: ItemId,
        character_id: CharacterId,
        now_on: bool,
        remaining_off: Option<i32>,
        gates_opened: bool,
    },
    PalaceGateTick {
        item_id: ItemId,
        opened: bool,
        closed: bool,
        blocked: bool,
    },
    TorchExtinguishedUnderwater {
        item_id: ItemId,
        character_id: CharacterId,
        schedule_after_ticks: u64,
    },
    TorchExpired {
        item_id: ItemId,
        character_id: CharacterId,
        item_name: [u8; OUTCOME_ITEM_NAME_BYTES],
    },
    ClanJewelRescheduled {
        item_id: ItemId,
        schedule_after_ticks: u64,
    },
    ClanJewelExpired {
        item_id: ItemId,
        character_id: Option<CharacterId>,
        item_name: [u8; OUTCOME_ITEM_NAME_BYTES],
    },
    DecayItemToggled {
        item_id: ItemId,
        character_id: CharacterId,
        active: bool,
        schedule_after_ticks: Option<u64>,
    },
    DecayItemExpired {
        item_id: ItemId,
        character_id: CharacterId,
        item_name: [u8; OUTCOME_ITEM_NAME_BYTES],
    },
    LabExitAnimating {
        item_id: ItemId,
        sprite: i32,
        frame: u32,
        schedule_after_ticks: u64,
    },
    LabExitExpired {
        item_id: ItemId,
    },
    LabExitUse {
        item_id: ItemId,
        character_id: CharacterId,
        lab_nr: u8,
        frame: u32,
        target_area: u16,
        target_x: u16,
        target_y: u16,
    },
    LabExitWrongOwner {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BeyondPotion {
        item_id: ItemId,
        character_id: CharacterId,
        duration_minutes: u8,
        modifier_index: [i16; MAX_MODIFIERS],
        modifier_value: [i16; MAX_MODIFIERS],
        beyond_max_mod: bool,
    },
    AlchemyFlaskPotion {
        item_id: ItemId,
        character_id: CharacterId,
        duration_minutes: u8,
        modifier_index: [i16; MAX_MODIFIERS],
        modifier_value: [i16; MAX_MODIFIERS],
    },
    TorchExtractOrb {
        item_id: ItemId,
        character_id: CharacterId,
        modifier_slot: usize,
        modifier: i16,
    },
    StatScrollUsed {
        item_id: ItemId,
        character_id: CharacterId,
        value: u8,
        raised: u8,
        exp_cost: u32,
    },
    AssembleItem {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        template: AssembleTemplate,
    },
    AssembleNeedsCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    AssembleDoesNotFit {
        item_id: ItemId,
        character_id: CharacterId,
    },
    AssembleUnknownItem {
        item_id: ItemId,
        character_id: CharacterId,
    },
    EnchantCursorItem {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        modifier: i16,
        amount: i16,
    },
    AntiEnchantCursorItem {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        modifier: i16,
        amount: i16,
        extract_orb: bool,
    },
    OrbSpawn {
        item_id: ItemId,
        character_id: CharacterId,
        anti: bool,
        special: bool,
    },
    NomadStack {
        item_id: ItemId,
        character_id: CharacterId,
    },
    TransportOpen {
        item_id: ItemId,
        character_id: CharacterId,
        point: u8,
    },
    TransportTravel {
        item_id: ItemId,
        character_id: CharacterId,
        spec: i32,
    },
    TransportInvalid {
        item_id: ItemId,
        character_id: CharacterId,
        point: u8,
    },
    ArenaToplist {
        item_id: ItemId,
        character_id: CharacterId,
    },
    SpecialPotionDrunk {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
        hp_delta: i32,
        mana_delta: i32,
        endurance_delta: i32,
    },
    SpecialPotionAntidote {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
        poison_removed: bool,
    },
    SpecialPotionInfravision {
        item_id: ItemId,
        character_id: CharacterId,
        installed: bool,
    },
    SpecialPotionSecurity {
        item_id: ItemId,
        character_id: CharacterId,
        used: bool,
    },
    SpecialPotionProfessionReset {
        item_id: ItemId,
        character_id: CharacterId,
        used: bool,
        professions_reset: u16,
        profession_points_lowered: u16,
        exp_refunded: u32,
    },
    SpecialPotionBug {
        item_id: ItemId,
        character_id: CharacterId,
    },
    SpecialShrine {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
    },
    DemonShrine {
        item_id: ItemId,
        character_id: CharacterId,
        location_id: u32,
    },
    ZombieShrine {
        item_id: ItemId,
        character_id: CharacterId,
        shrine_type: u8,
    },
    ZombieShrineNeedsOffering {
        item_id: ItemId,
        character_id: CharacterId,
        shrine_type: u8,
    },
    XmasMaker {
        item_id: ItemId,
        character_id: CharacterId,
    },
    XmasTree {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PalaceKeySplit {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_part_sprite: i32,
        carried_part_sprite: i32,
    },
    PalaceKeyCombine {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        result_sprite: i32,
        final_key: bool,
    },
    PalaceKeyNeedsCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PalaceKeyDoesNotFit {
        item_id: ItemId,
        character_id: CharacterId,
    },
    EnchantNeedsCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ShrikeAmuletAssemble {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        combined_bits: u8,
    },
    ShrikeAmuletNeedsCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ShrikeAmuletDoesNotFit {
        item_id: ItemId,
        character_id: CharacterId,
    },
    MineGatewayKeyAssemble {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        combined_bits: u8,
    },
    MineGatewayKeyNeedsCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    MineGatewayKeyDoesNotFit {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ArkhataKeyAssemble {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        result_template_id: u32,
        result_sprite: i32,
        final_key: bool,
    },
    ArkhataKeyNeedsCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ArkhataKeyDoesNotFit {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ArkhataPool {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
    },
    ArkhataPoolNeedsCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    ArkhataPoolWrongCursor {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
    },
    ArkhataStopwatch {
        item_id: ItemId,
        character_id: CharacterId,
        schedule_after_ticks: u32,
    },
    BlockedByRequirements {
        item_id: ItemId,
        character_id: CharacterId,
    },
    EmptyPotionTemplateNeeded {
        item_id: ItemId,
        character_id: CharacterId,
        empty_kind: u8,
    },
    BlockedByArea {
        item_id: ItemId,
        character_id: CharacterId,
    },
    LibloadAreaBlocked {
        driver: u16,
        item_id: ItemId,
        character_id: CharacterId,
        required_area: u16,
    },
    OxygenPotion {
        item_id: ItemId,
        character_id: CharacterId,
        installed: bool,
    },
    PickBerry {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
        location_id: u32,
    },
    PickBerryCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    PickAlchemyFlower {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
        location_id: u32,
    },
    PickAlchemyFlowerCursorOccupied {
        item_id: ItemId,
        character_id: CharacterId,
    },
    NomadDice {
        item_id: ItemId,
        character_id: CharacterId,
        luck: u8,
    },
    FlaskIngredientAdded {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        ingredient_kind: u8,
    },
    FlaskWrongCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    FlaskFull {
        item_id: ItemId,
        character_id: CharacterId,
    },
    FlaskFinishedNoMoreIngredients {
        item_id: ItemId,
        character_id: CharacterId,
    },
    FlaskEmptyShaken {
        item_id: ItemId,
        character_id: CharacterId,
    },
    FlaskIngredientBug {
        item_id: ItemId,
        character_id: CharacterId,
    },
    FlaskMixed {
        item_id: ItemId,
        character_id: CharacterId,
        ingredient_counts: [u8; 29],
    },
    FlaskRuined {
        item_id: ItemId,
        character_id: CharacterId,
        ingredient_counts: [u8; 29],
    },
    LizardFlowerMixed {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        combined_bits: u8,
        complete: bool,
        bottle_message: bool,
    },
    LizardFlowerNeedsCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    LizardFlowerDoesNotFit {
        item_id: ItemId,
        character_id: CharacterId,
    },
    Lab3YellowBerry {
        item_id: ItemId,
        character_id: CharacterId,
        duration_ticks: u64,
        installed: bool,
    },
    Lab3WhiteBerry {
        item_id: ItemId,
        character_id: CharacterId,
        light_power: i16,
        started_emit: bool,
        installed: bool,
    },
    Lab3WhiteBerryLightTick {
        item_id: ItemId,
        destroyed: bool,
    },
    Lab3BrownBerry {
        item_id: ItemId,
        character_id: CharacterId,
        duration_ticks: u64,
        installed: bool,
    },
    ParkShrine {
        item_id: ItemId,
        character_id: CharacterId,
        shrine: u8,
    },
    ParkShrineBug {
        item_id: ItemId,
        character_id: CharacterId,
        shrine: u8,
    },
    CaligarTraining {
        item_id: ItemId,
        character_id: CharacterId,
        lesson: u8,
    },
    CaligarWeightMove {
        item_id: ItemId,
        character_id: CharacterId,
    },
    CaligarWeightTimer {
        item_id: ItemId,
    },
    CaligarWeightBlocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    CaligarWeightDoor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    CaligarWeightDoorLocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    CaligarWeightDoorBusy {
        item_id: ItemId,
        character_id: CharacterId,
    },
    CaligarSkellyDoor {
        item_id: ItemId,
        character_id: CharacterId,
        door_index: u8,
    },
    CaligarSkellyDoorLocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    CaligarSkellyDoorBusy {
        item_id: ItemId,
        character_id: CharacterId,
    },
    CaligarKeyAssemble {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        result_sprite: i32,
        final_key: bool,
    },
    CaligarKeyNeedsCursor {
        item_id: ItemId,
        character_id: CharacterId,
    },
    CaligarKeyDoesNotFit {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BookText {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
        demon_value: i32,
    },
    BookcaseText {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
    },
    SkelRaiseDust {
        item_id: ItemId,
        character_id: CharacterId,
    },
    SkelRaiseTouch {
        item_id: ItemId,
        character_id: CharacterId,
    },
    SkelRaiseRaise {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
        template: &'static str,
    },
    SkelRaiseTimer {
        item_id: ItemId,
    },
    BookcaseLocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BoneHint {
        item_id: ItemId,
        character_id: CharacterId,
        level: u8,
        nr: u8,
        pos: u8,
    },
    StafferBookText {
        item_id: ItemId,
        character_id: CharacterId,
        page: u8,
    },
    StafferAnimationBook {
        item_id: ItemId,
        character_id: CharacterId,
        exp_added: u32,
    },
    StafferMineDig {
        item_id: ItemId,
        character_id: CharacterId,
    },
    StafferMineTimer {
        item_id: ItemId,
    },
    StafferMineExhausted {
        item_id: ItemId,
        character_id: CharacterId,
    },
    StafferBlockMove {
        item_id: ItemId,
        character_id: CharacterId,
    },
    StafferBlockTimer {
        item_id: ItemId,
    },
    StafferBlockBlocked {
        item_id: ItemId,
        character_id: CharacterId,
    },
    BoneBridgePlace {
        item_id: ItemId,
        character_id: CharacterId,
        cursor_item_id: ItemId,
    },
    BoneBridgeTimerTick {
        item_id: ItemId,
    },
    AccountDepotOpened {
        item_id: ItemId,
        character_id: CharacterId,
    },
    IdentityTag {
        driver: u16,
        item_id: ItemId,
        character_id: CharacterId,
    },
    Noop,
    Unsupported {
        driver: u16,
        item_id: ItemId,
        character_id: CharacterId,
    },
}

pub fn legacy_item_driver_return_code(driver: Option<u16>, outcome: &ItemDriverOutcome) -> i32 {
    match outcome {
        ItemDriverOutcome::DoorToggle { .. }
        | ItemDriverOutcome::KeyedDoorToggle { .. }
        | ItemDriverOutcome::DoubleDoorToggle { .. }
        | ItemDriverOutcome::PickDoorToggle { .. }
        | ItemDriverOutcome::StafferSpecDoorToggle { .. } => 1,
        ItemDriverOutcome::StafferSpecDoorLocked { .. }
        | ItemDriverOutcome::PickDoorLocked { .. }
        | ItemDriverOutcome::CaligarWeightDoorLocked { .. }
        | ItemDriverOutcome::CaligarSkellyDoorLocked { .. }
        | ItemDriverOutcome::PentBossDoorLocked { .. }
        | ItemDriverOutcome::PentBossDoorBusy { .. } => 2,
        ItemDriverOutcome::Noop
            if matches!(
                driver,
                Some(IDR_DOOR) | Some(IDR_DOUBLE_DOOR) | Some(IDR_STAFFER2) | Some(IDR_CALIGAR)
            ) =>
        {
            2
        }
        ItemDriverOutcome::IdentityTag { .. } => 1,
        ItemDriverOutcome::Noop | ItemDriverOutcome::Unsupported { .. } => 0,
        _ => 1,
    }
}

pub fn use_item(
    character: &mut Character,
    item: &Item,
    request: ItemUseRequest,
    account_depot_available: bool,
) -> Result<UseItemOutcome, UseItemError> {
    if character.id != request.character_id {
        return Err(UseItemError::IllegalCharacter);
    }
    if item.id != request.item_id {
        return Err(UseItemError::IllegalItem);
    }
    if character.flags.contains(CharacterFlags::DEAD) {
        return Err(UseItemError::Dead);
    }

    if item.driver == IDR_ACCOUNT_DEPOT {
        if !account_depot_available {
            return Err(UseItemError::AccountDepotUnavailable);
        }
        character.current_container = Some(item.id);
        return Ok(UseItemOutcome::OpenAccountDepot { item_id: item.id });
    }

    if item.content_id != 0 {
        character.current_container = Some(item.id);
        return Ok(UseItemOutcome::OpenContainer { item_id: item.id });
    }

    if item.flags.contains(ItemFlags::DEPOT) {
        character.current_container = Some(item.id);
        return Ok(UseItemOutcome::OpenDepot { item_id: item.id });
    }

    Ok(UseItemOutcome::Dispatch(ItemDriverRequest::Driver {
        driver: item.driver,
        item_id: item.id,
        character_id: character.id,
        spec: request.spec,
    }))
}

pub fn execute_item_driver(
    character: &mut Character,
    item: &mut Item,
    request: ItemDriverRequest,
    area_id: u16,
    in_arena: bool,
) -> ItemDriverOutcome {
    execute_item_driver_with_context(
        character,
        item,
        request,
        area_id,
        in_arena,
        &ItemDriverContext::default(),
    )
}

pub fn execute_item_driver_with_context(
    character: &mut Character,
    item: &mut Item,
    request: ItemDriverRequest,
    area_id: u16,
    in_arena: bool,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    match request {
        ItemDriverRequest::Driver {
            driver,
            item_id,
            character_id,
            spec,
        } => {
            if character.id != character_id || item.id != item_id {
                return ItemDriverOutcome::Noop;
            }
            if let Some(required_area) = legacy_libload_required_area(driver) {
                if area_id != required_area {
                    return ItemDriverOutcome::LibloadAreaBlocked {
                        driver,
                        item_id,
                        character_id,
                        required_area,
                    };
                }
            }
            if driver >= 1000 {
                return ItemDriverOutcome::IdentityTag {
                    driver,
                    item_id,
                    character_id,
                };
            }

            match driver {
                0 => ItemDriverOutcome::LookItem {
                    item_id,
                    character_id,
                },
                IDR_POTION => potion_driver(character, item, area_id, in_arena),
                IDR_DOOR => door_driver(character, item, context),
                IDR_BALLTRAP => balltrap_driver(character, item),
                IDR_BONEBRIDGE => bonebridge_driver(character, item, context),
                IDR_BONELADDER => boneladder_driver(character, item),
                IDR_BONEHINT => bonehint_driver(character, item, context),
                IDR_FIREBALL => fireball_machine_driver(character, item, context),
                IDR_EDEMONBALL => edemonball_driver(character, item, context),
                IDR_EDEMONSWITCH => edemon_switch_driver(character, item, context),
                IDR_EDEMONLOADER => edemon_loader_driver(character, item, context),
                IDR_EDEMONLIGHT => edemon_light_driver(character, item, context),
                IDR_FDEMONLIGHT => fdemon_light_driver(character, item, context),
                IDR_FDEMONLOADER => fdemon_loader_driver(character, item, context),
                IDR_FDEMONFARM => fdemon_farm_driver(character, item, context),
                IDR_FDEMONBLOOD => fdemon_blood_driver(character, item, context),
                IDR_FLAMETHROW => flamethrow_driver(character, item, context),
                IDR_USETRAP => usetrap_driver(character, item),
                IDR_STEPTRAP => steptrap_driver(character, item, context),
                IDR_PALACEGATE => palace_gate_driver(character, item, context),
                IDR_SPIKETRAP => spiketrap_driver(character, item, context),
                IDR_EXTINGUISH => extinguish_driver(character, item),
                IDR_CHEST => chest_driver(character, item),
                IDR_RANDCHEST => randchest_driver(character, item),
                IDR_FORESTSPADE => forest_spade_driver(character, item, area_id),
                IDR_PICKDOOR => pick_door_driver(character, item, context),
                IDR_PICKCHEST => pick_chest_driver(character, item, context),
                IDR_PENTBOSSDOOR => pent_boss_door_driver(character, item, context),
                IDR_BURNDOWN => burndown_driver(character, item, context),
                IDR_COLORTILE => colortile_driver(character, item),
                IDR_SKELRAISE => skelraise_driver(character, item, context),
                IDR_SHRINE => zombie_shrine_driver(character, item, context),
                IDR_PARKSHRINE => parkshrine_driver(character, item),
                IDR_BOOK => book_driver(character, item),
                IDR_BOOKCASE => bookcase_driver(character, item, context),
                IDR_DEMONSHRINE => demonshrine_driver(character, item, area_id),
                IDR_PALACEKEY => palace_key_driver(character, item, context),
                IDR_INFINITE_CHEST => infinite_chest_driver(character, item, context),
                IDR_RECALL => recall_driver(character, item, area_id, in_arena),
                IDR_TRANSPORT => transport_driver(character, item, spec),
                IDR_STATSCROLL => stat_scroll_driver(character, item),
                IDR_CLANJEWEL => clanjewel_driver(character, item, context),
                IDR_ASSEMBLE => assemble_driver(character, item, context),
                IDR_CITY_RECALL => city_recall_driver(character, item, area_id, in_arena),
                IDR_DUNGEONTELE => dungeon_teleport_driver(character, item),
                IDR_DUNGEONFAKE => dungeon_fake_driver(character, item),
                IDR_DUNGEONDOOR => dungeon_door_driver(character, item, context),
                IDR_DUNGEONKEY => dungeon_key_driver(character, item),
                IDR_FLASK => flask_driver(character, item, context, area_id, in_arena),
                IDR_DOUBLE_DOOR => double_door_driver(character, item),
                IDR_TELE_DOOR => teleport_door_driver(character, item),
                IDR_TELEPORT => teleport_driver(character, item),
                IDR_ONOFFLIGHT => onofflight_driver(character, item, context),
                IDR_NIGHTLIGHT => nightlight_driver(character, item, context),
                IDR_TORCH => torch_driver(character, item, context),
                IDR_FOOD => food_driver(character, item),
                IDR_TOPLIST => toplist_driver(character, item),
                IDR_ENCHANTITEM => enchant_driver(character, item),
                IDR_ANTIENCHANTITEM => anti_enchant_driver(character, item, false),
                IDR_SPECIALANTIENCHANTITEM => anti_enchant_driver(character, item, true),
                IDR_ORBSPAWN => orbspawn_driver(character, item, false),
                IDR_ANTIORBSPAWN => orbspawn_driver(character, item, true),
                IDR_SPECIAL_POTION => {
                    special_potion_driver(character, item, area_id, in_arena, context.current_tick)
                }
                IDR_SPECIAL_SHRINE => special_shrine_driver(character, item),
                IDR_NOMADDICE => nomad_dice_driver(character, item),
                IDR_NOMADSTACK => nomad_stack_driver(character, item),
                IDR_DEMONCHIP => nomad_stack_driver(character, item),
                IDR_STAFFER2 => staffer2_driver(character, item),
                IDR_SHRIKEAMULET => shrike_amulet_driver(character, item, context),
                IDR_MINEGATEWAYKEY => mine_gateway_key_driver(character, item, context),
                IDR_TOYLIGHT => toylight_driver(character, item, context),
                IDR_DECAYITEM => decaying_item_driver(character, item, context),
                IDR_OXYPOTION => oxy_potion_driver(character, item, area_id),
                IDR_FLOWER => alchemy_flower_driver(character, item, area_id),
                IDR_PICKBERRY => pick_berry_driver(character, item, area_id),
                IDR_LIZARDFLOWER => lizard_flower_driver(character, item, context, area_id),
                IDR_LAB3_PLANT => lab3_plant_driver(character, item, context),
                IDR_LABEXIT => labexit_driver(character, item, context),
                IDR_BEYONDPOTION => beyond_potion_driver(character, item, area_id, in_arena),
                IDR_XMASTREE => xmastree_driver(character, item),
                IDR_XMASMAKER => xmasmaker_driver(character, item),
                IDR_CALIGAR => caligar_driver(character, item, context),
                IDR_ARKHATA => arkhata_driver(character, item, context),
                IDR_CALIGARFLAME => flamethrow_driver(character, item, context),
                IDR_FREAKDOOR => freakdoor_driver(character, item),
                IDR_KEY_RING => keyring_driver(character, item),
                _ => ItemDriverOutcome::Unsupported {
                    driver,
                    item_id,
                    character_id,
                },
            }
        }
        ItemDriverRequest::AccountDepot {
            item_id,
            character_id,
        } => ItemDriverOutcome::AccountDepotOpened {
            item_id,
            character_id,
        },
    }
}

fn legacy_libload_required_area(driver: u16) -> Option<u16> {
    match driver {
        IDR_BONEBRIDGE | IDR_BONELADDER | IDR_BONEHINT => Some(18),
        IDR_NOMADDICE => Some(19),
        IDR_PENT | IDR_PENTBOSSDOOR => Some(4),
        IDR_PICKDOOR | IDR_PICKCHEST | IDR_BURNDOWN | IDR_COLORTILE | IDR_SKELRAISE => Some(17),
        IDR_STAFFER2 => Some(29),
        IDR_OXYPOTION | IDR_LIZARDFLOWER => Some(31),
        IDR_CALIGAR => Some(36),
        IDR_ARKHATA => Some(37),
        IDR_DUNGEONTELE | IDR_DUNGEONFAKE | IDR_DUNGEONDOOR | IDR_DUNGEONKEY => Some(13),
        IDR_FDEMONLIGHT | IDR_FDEMONLOADER | IDR_FDEMONFARM | IDR_FDEMONBLOOD => Some(8),
        _ => None,
    }
}

fn clanjewel_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if !context.timer_call || character.id.0 != 0 {
        return ItemDriverOutcome::Noop;
    }

    let current_seconds = context.current_tick / TICKS_PER_SECOND as u32;
    let mut creation_time = drdata_u32(item, 0);
    if creation_time == 0 {
        creation_time = current_seconds;
        set_drdata_u32(item, 0, creation_time);
    }

    if current_seconds > creation_time.saturating_add(CLANJEWEL_LIFETIME_SECONDS) {
        return ItemDriverOutcome::ClanJewelExpired {
            item_id: item.id,
            character_id: item.carried_by,
            item_name: outcome_item_name(&item.name),
        };
    }

    ItemDriverOutcome::ClanJewelRescheduled {
        item_id: item.id,
        schedule_after_ticks: CLANJEWEL_CHECK_INTERVAL_TICKS,
    }
}

fn balltrap_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 || character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }

    let dx = i16::from(drdata(item, 0)) - 128;
    let dy = i16::from(drdata(item, 1)) - 128;
    let dxs = dx.signum();
    let dys = dy.signum();
    let item_x = i32::from(item.x);
    let item_y = i32::from(item.y);

    ItemDriverOutcome::BallTrapProjectile {
        item_id: item.id,
        character_id: character.id,
        start_x: clamp_legacy_coordinate(item_x + i32::from(dxs)),
        start_y: clamp_legacy_coordinate(item_y + i32::from(dys)),
        target_x: clamp_legacy_coordinate(item_x + i32::from(dx)),
        target_y: clamp_legacy_coordinate(item_y + i32::from(dy)),
        power: drdata(item, 2),
    }
}

fn parkshrine_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let shrine = drdata(item, 0);
    if !(1..=3).contains(&shrine) {
        return ItemDriverOutcome::ParkShrineBug {
            item_id: item.id,
            character_id: character.id,
            shrine,
        };
    }

    ItemDriverOutcome::ParkShrine {
        item_id: item.id,
        character_id: character.id,
        shrine,
    }
}

fn caligar_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    match drdata(item, 0) {
        1 => caligar_training_driver(character, item),
        2 | 4 => caligar_weight_driver(character, item),
        3 => caligar_weight_door_driver(character, item),
        5..=9 => caligar_gun_driver(character, item),
        10 => caligar_key_assembly_driver(character, item, context),
        11 => extinguish_driver(character, item),
        12 => caligar_skelly_door_driver(character, item),
        _ => ItemDriverOutcome::Unsupported {
            driver: IDR_CALIGAR,
            item_id: item.id,
            character_id: character.id,
        },
    }
}

fn caligar_skelly_door_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::CaligarSkellyDoor {
        item_id: item.id,
        character_id: character.id,
        door_index: drdata(item, 1),
    }
}

fn caligar_key_assembly_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::CaligarKeyNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    };
    if context.cursor_template_id != Some(IID_CALIGAR_PALACE_KEY_PART) {
        return ItemDriverOutcome::CaligarKeyNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let sp1 = item.sprite.min(context.cursor_sprite.unwrap_or_default());
    let sp2 = item.sprite.max(context.cursor_sprite.unwrap_or_default());
    let result = match (sp1, sp2) {
        (13414, 13415) => Some((13421, false)),
        (13415, 13416) => Some((13420, false)),
        (13414, 13420) | (13416, 13421) => Some((0, true)),
        _ => None,
    };

    let Some((result_sprite, final_key)) = result else {
        return ItemDriverOutcome::CaligarKeyDoesNotFit {
            item_id: item.id,
            character_id: character.id,
        };
    };

    ItemDriverOutcome::CaligarKeyAssemble {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        result_sprite,
        final_key,
    }
}

fn caligar_training_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 || !character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }

    match drdata(item, 1) {
        1..=3 => ItemDriverOutcome::CaligarTraining {
            item_id: item.id,
            character_id: character.id,
            lesson: drdata(item, 1),
        },
        _ => ItemDriverOutcome::Noop,
    }
}

fn caligar_weight_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::CaligarWeightTimer { item_id: item.id };
    }

    ItemDriverOutcome::CaligarWeightMove {
        item_id: item.id,
        character_id: character.id,
    }
}

fn caligar_weight_door_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::CaligarWeightDoor {
        item_id: item.id,
        character_id: character.id,
    }
}

fn caligar_gun_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    ItemDriverOutcome::CaligarGunProjectile {
        item_id: item.id,
        character_id: character.id,
        direction: drdata(item, 0) - 4,
        schedule_after_ticks: 12,
    }
}

fn arkhata_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    match drdata(item, 0) {
        0 => arkhata_pool_driver(character, item, context),
        1 => arkhata_stopwatch_driver(character, item),
        2 => arkhata_key_assemble_driver(character, item, context),
        _ => ItemDriverOutcome::Unsupported {
            driver: IDR_ARKHATA,
            item_id: item.id,
            character_id: character.id,
        },
    }
}

fn arkhata_stopwatch_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 != 0 {
        return ItemDriverOutcome::Noop;
    }

    let Some(character_id) = item.carried_by.filter(|id| id.0 != 0) else {
        return ItemDriverOutcome::Noop;
    };

    ItemDriverOutcome::ArkhataStopwatch {
        item_id: item.id,
        character_id,
        schedule_after_ticks: 10,
    }
}

const IID_ARKHATA_SCROLL1: u32 = make_item_id(DEV_ID_DB, 0x0000C2);

fn arkhata_pool_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::ArkhataPoolNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    };
    if context.cursor_template_id != Some(IID_ARKHATA_SCROLL1) {
        return ItemDriverOutcome::ArkhataPoolWrongCursor {
            item_id: item.id,
            character_id: character.id,
            cursor_item_id,
        };
    }

    ItemDriverOutcome::ArkhataPool {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
    }
}

const IID_ARKHATA_AKEY1: u32 = make_item_id(DEV_ID_DB, 0x0000CA);
const IID_ARKHATA_AKEY2: u32 = make_item_id(DEV_ID_DB, 0x0000CB);
const IID_ARKHATA_AKEY3: u32 = make_item_id(DEV_ID_DB, 0x0000CC);
const IID_ARKHATA_AKEY12: u32 = make_item_id(DEV_ID_DB, 0x0000CD);
const IID_ARKHATA_AKEY23: u32 = make_item_id(DEV_ID_DB, 0x0000CE);
const IID_ARKHATA_AKEY: u32 = make_item_id(0x3B, 0x000089);

fn arkhata_key_assemble_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::ArkhataKeyNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    };
    let Some(cursor_template_id) = context.cursor_template_id else {
        return ItemDriverOutcome::ArkhataKeyDoesNotFit {
            item_id: item.id,
            character_id: character.id,
        };
    };

    let result = match (item.template_id, cursor_template_id) {
        (IID_ARKHATA_AKEY1, IID_ARKHATA_AKEY2) | (IID_ARKHATA_AKEY2, IID_ARKHATA_AKEY1) => {
            Some((IID_ARKHATA_AKEY12, 13421, false))
        }
        (IID_ARKHATA_AKEY2, IID_ARKHATA_AKEY3) | (IID_ARKHATA_AKEY3, IID_ARKHATA_AKEY2) => {
            Some((IID_ARKHATA_AKEY23, 13420, false))
        }
        (IID_ARKHATA_AKEY1, IID_ARKHATA_AKEY23)
        | (IID_ARKHATA_AKEY23, IID_ARKHATA_AKEY1)
        | (IID_ARKHATA_AKEY12, IID_ARKHATA_AKEY3)
        | (IID_ARKHATA_AKEY3, IID_ARKHATA_AKEY12) => Some((IID_ARKHATA_AKEY, 13413, true)),
        _ => None,
    };

    let Some((result_template_id, result_sprite, final_key)) = result else {
        return ItemDriverOutcome::ArkhataKeyDoesNotFit {
            item_id: item.id,
            character_id: character.id,
        };
    };

    ItemDriverOutcome::ArkhataKeyAssemble {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        result_template_id,
        result_sprite,
        final_key,
    }
}

fn bonebridge_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call {
        if drdata(item, 1) == 0 {
            return ItemDriverOutcome::Noop;
        }
        return ItemDriverOutcome::BoneBridgeTimerTick { item_id: item.id };
    }

    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    if drdata(item, 0) != 0 && drdata(item, 1) == 0 {
        // Adding/removing bones from a partial carried bridge depends on creating
        // the generic "bone" template and is applied as a later area-18 slice.
        return ItemDriverOutcome::Noop;
    }

    let Some(cursor_item_id) = character.cursor_item else {
        return ItemDriverOutcome::Noop;
    };
    if context.cursor_template_id != Some(IID_AREA18_BONE) || context.cursor_drdata0 != Some(5) {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::BoneBridgePlace {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
    }
}

fn bonehint_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    if drdata(item, 1) == 0 {
        let Some(nr) = context.bone_hint_nr else {
            return ItemDriverOutcome::Noop;
        };
        let Some(pos) = context.bone_hint_pos else {
            return ItemDriverOutcome::Noop;
        };
        set_drdata(item, 1, 1);
        set_drdata(item, 2, nr.min(4));
        set_drdata(item, 3, pos.min(2));
    }

    ItemDriverOutcome::BoneHint {
        item_id: item.id,
        character_id: character.id,
        level: drdata(item, 0),
        nr: drdata(item, 2),
        pos: drdata(item, 3),
    }
}

fn boneladder_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let (dx, dy) = if drdata(item, 0) != 0 {
        (-4, -3)
    } else {
        (4, 3)
    };
    let x = (i32::from(item.x) + dx).max(0) as u16;
    let y = (i32::from(item.y) + dy).max(0) as u16;

    ItemDriverOutcome::Teleport {
        item_id: item.id,
        character_id: character.id,
        x,
        y,
        area_id: 0,
        stop_driver: false,
        quiet: false,
    }
}

fn staffer2_driver(character: &mut Character, item: &mut Item) -> ItemDriverOutcome {
    match drdata(item, 0) {
        1 => {
            if character.id.0 == 0 || !character.flags.contains(CharacterFlags::PLAYER) {
                return ItemDriverOutcome::Noop;
            }
            staffer_book_driver(character, item)
        }
        2 => staffer_mine_driver(character, item),
        3 => staffer_block_driver(character, item),
        4 | 5 => staffer_spec_door_driver(character, item),
        6 => {
            if character.id.0 == 0 || !character.flags.contains(CharacterFlags::PLAYER) {
                return ItemDriverOutcome::Noop;
            }
            let exp_added =
                (legacy_level_value(60) / 5).min(legacy_level_value(character.level) / 4);
            ItemDriverOutcome::StafferAnimationBook {
                item_id: item.id,
                character_id: character.id,
                exp_added,
            }
        }
        _ => ItemDriverOutcome::Unsupported {
            driver: IDR_STAFFER2,
            item_id: item.id,
            character_id: character.id,
        },
    }
}

fn staffer_spec_door_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if item.x == 0 {
        return ItemDriverOutcome::Noop;
    }
    ItemDriverOutcome::StafferSpecDoorToggle {
        item_id: item.id,
        character_id: character.id,
        kind: drdata(item, 0),
    }
}

fn freakdoor_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    ItemDriverOutcome::FreakDoorUse {
        item_id: item.id,
        character_id: character.id,
        link_group: drdata(item, 8),
        one_way: drdata(item, 14) != 0,
        recursion_guard: drdata(item, 9) != 0,
        cached_partner_id: match drdata_u32(item, 10) {
            0 => None,
            id => Some(ItemId(id)),
        },
        no_target: drdata(item, 15) != 0,
    }
}

fn staffer_mine_driver(character: &mut Character, item: &mut Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::StafferMineTimer { item_id: item.id };
    }
    if !character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }

    if drdata(item, 3) < 9 {
        if character.endurance < POWERSCALE {
            return ItemDriverOutcome::StafferMineExhausted {
                item_id: item.id,
                character_id: character.id,
            };
        }
        let miner = character.professions.get(2).copied().unwrap_or_default();
        let cost = POWERSCALE / 4 - (i32::from(miner) * POWERSCALE / (4 * 25));
        character.endurance = character.endurance.saturating_sub(cost.max(0));
        set_drdata(item, 3, drdata(item, 3).saturating_add(1));
        set_drdata(item, 5, 0);
        item.sprite += 1;
    }

    ItemDriverOutcome::StafferMineDig {
        item_id: item.id,
        character_id: character.id,
    }
}

fn staffer_block_driver(character: &Character, item: &mut Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::StafferBlockTimer { item_id: item.id };
    }
    if !character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }
    ItemDriverOutcome::StafferBlockMove {
        item_id: item.id,
        character_id: character.id,
    }
}

fn staffer_book_driver(character: &Character, item: &mut Item) -> ItemDriverOutcome {
    if drdata_u32(item, 4) != character.id.0 {
        set_drdata(item, 1, 0);
        set_drdata_u32(item, 4, character.id.0);
    }

    let page = drdata(item, 1);
    if page > 4 {
        return ItemDriverOutcome::Noop;
    }

    if page == 4 {
        set_drdata(item, 1, 0);
    } else {
        set_drdata(item, 1, page + 1);
    }

    ItemDriverOutcome::StafferBookText {
        item_id: item.id,
        character_id: character.id,
        page,
    }
}

pub fn staffer_book_text(page: u8) -> Option<&'static str> {
    match page {
        0 => Some("The training of these thieves into skilled mages has been succesful. They can now create Golems, and summon the old enemies of Aston, the Grolms. I will not teach them how to create and control Undead though, lest they use them against me... Also, to this end, I have enlisted the help of an assassin by the name of Brenneth. I hope he will not disappoint me..."),
        1 => Some("My golems have dug their way into the Brannington Crypt. I have taken their Holy Relic, and turned it into my weapon to make undead of the Brannington Ancestors. They shall serve as my army and take over Brannington town. All serve as zombies and skeletons, however, there is one spirit who managed to escape my grasp. I will have to find ways to control it... Also, Brenneth was attacked by a grolm and is suffering from loss of memory... He is in one of the thief mage houses right now... Fortunately, they don't know who he is..."),
        2 => Some("Brenneth got rescued by a group of traveling adventurers while the thief mage who had him captured was creating more golems... Luckily, Brenneth doesn't recall anything of what he is supposed to do, and it doesn't look like he'll get his memory back... ever..."),
        3 => Some("The spirit seems uncontrollable... I will have to become stronger to control it, which means I have to train... And that takes time, time which I'd rather not waste... I have also seen the face of a new enemy... This enemy has killed my thief mages, and surely must be coming for me next... He ruined my plans to open the crypt doors with the jewelry the thief mages had managed to steal... They should have been faster in returning it to me... fools..."),
        4 => Some("I can hear my enemy coming for me... I shall kill and make of my enemy a commander in my army of undead... Now, I will fight and show my power!"),
        _ => None,
    }
}

pub fn staffer_book_continue_text(page: u8) -> Option<&'static str> {
    match page {
        0..=3 => Some("USE again to continue."),
        4 => Some("USE to start over."),
        _ => None,
    }
}

fn fireball_machine_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    let dx = i16::from(drdata(item, 0)) - 128;
    let dy = i16::from(drdata(item, 1)) - 128;
    let dxs = dx.signum();
    let dys = dy.signum();
    let item_x = i32::from(item.x);
    let item_y = i32::from(item.y);
    let frequency = u64::from(drdata(item, 3));

    ItemDriverOutcome::FireballMachineProjectile {
        item_id: item.id,
        character_id: character.id,
        start_x: clamp_legacy_coordinate(item_x + i32::from(dxs)),
        start_y: clamp_legacy_coordinate(item_y + i32::from(dys)),
        target_x: clamp_legacy_coordinate(item_x + i32::from(dx)),
        target_y: clamp_legacy_coordinate(item_y + i32::from(dy)),
        power: drdata(item, 2),
        schedule_after_ticks: (context.timer_call && frequency != 0).then_some(frequency),
    }
}

fn oxy_potion_driver(character: &Character, item: &Item, area_id: u16) -> ItemDriverOutcome {
    if area_id != 31 {
        return ItemDriverOutcome::BlockedByArea {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::OxygenPotion {
        item_id: item.id,
        character_id: character.id,
        installed: false,
    }
}

fn pick_berry_driver(character: &Character, item: &Item, area_id: u16) -> ItemDriverOutcome {
    if area_id != 31 {
        return ItemDriverOutcome::BlockedByArea {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    if character.cursor_item.is_some() {
        return ItemDriverOutcome::PickBerryCursorOccupied {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::PickBerry {
        item_id: item.id,
        character_id: character.id,
        kind: drdata(item, 0),
        location_id: u32::from(item.x) + (u32::from(item.y) << 8) + (u32::from(area_id) << 16),
    }
}

fn alchemy_flower_driver(character: &Character, item: &Item, area_id: u16) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    if character.cursor_item.is_some() {
        return ItemDriverOutcome::PickAlchemyFlowerCursorOccupied {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::PickAlchemyFlower {
        item_id: item.id,
        character_id: character.id,
        kind: item.driver_data.first().copied().unwrap_or_default(),
        location_id: u32::from(item.x) + (u32::from(item.y) << 8) + (u32::from(area_id) << 16),
    }
}

fn nomad_dice_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::NomadDice {
        item_id: item.id,
        character_id: character.id,
        luck: drdata(item, 0),
    }
}

pub fn legacy_lucky_die_from_rolls(sides: u8, luck: u8, rolls: impl IntoIterator<Item = u8>) -> u8 {
    let needed = usize::from(luck) + 1;
    rolls
        .into_iter()
        .take(needed)
        .map(|roll| roll.clamp(1, sides.max(1)))
        .max()
        .unwrap_or(1)
}

pub fn legacy_nomad_dice_total<const ROLLS_PER_DIE: usize>(
    luck: u8,
    rolls: [[u8; ROLLS_PER_DIE]; 3],
) -> u8 {
    rolls
        .into_iter()
        .map(|die_rolls| legacy_lucky_die_from_rolls(6, luck, die_rolls))
        .sum()
}

const P_ALCHEMIST: usize = 1;

fn alchemist_profession(character: &Character) -> i16 {
    character
        .professions
        .get(P_ALCHEMIST)
        .copied()
        .unwrap_or_default()
}

fn flask_power(item: &Item, character: &Character, context: &ItemDriverContext) -> i32 {
    let powers = [
        drdata(item, 18),
        drdata(item, 19),
        drdata(item, 20),
        drdata(item, 21),
        drdata(item, 22),
        drdata(item, 23),
        drdata(item, 24),
        drdata(item, 25),
        drdata(item, 26),
    ];
    let count: u8 = powers.iter().copied().sum();
    let stone = drdata(item, 31) + drdata(item, 32) + drdata(item, 33) + drdata(item, 34);
    let alchemist = alchemist_profession(character);

    const PAIR_POWER: [(i32, i32, i32); 8] = [
        (16, 12, 10),
        (24, 20, 16),
        (32, 26, 20),
        (40, 32, 24),
        (48, 38, 28),
        (56, 44, 32),
        (64, 50, 36),
        (72, 56, 40),
    ];
    if count == 2 {
        for pair in 0..8 {
            if powers[pair] == 1 && powers[pair + 1] == 1 {
                let (best, mid, low) = PAIR_POWER[pair];
                if context.solstice || (context.fullmoon && alchemist >= 30) || alchemist >= 50 {
                    return best;
                }
                if context.equinox || (context.fullmoon && alchemist >= 20) || alchemist >= 40 {
                    return mid;
                }
                if context.fullmoon || stone != 0 || alchemist >= 10 {
                    return low;
                }
            }
        }
    }

    let good = if context.solstice || (context.fullmoon && alchemist >= 30) || alchemist >= 50 {
        8
    } else if context.equinox || (context.fullmoon && alchemist >= 20) || alchemist >= 40 {
        4
    } else if context.fullmoon || alchemist >= 10 {
        2
    } else if context.hour == 12 {
        1
    } else {
        0
    };
    let bad = if context.newmoon {
        2
    } else if context.hour == 0 {
        1
    } else {
        0
    };

    for (idx, present) in powers.iter().enumerate().rev() {
        if *present != 0 {
            let base = match idx {
                8 => 36,
                7 => 32,
                6 => 28,
                5 => 24,
                4 => 20,
                3 => 16,
                2 => 12,
                1 => 8,
                _ => 6,
            };
            return if idx <= 1 {
                (base + good - bad).max(2)
            } else {
                base + good - bad
            };
        }
    }

    -1
}

fn flask_duration(item: &Item) -> Option<(u8, f64)> {
    if drdata(item, 27) != 0 {
        Some((60, 1.75))
    } else if drdata(item, 30) != 0 {
        Some((30, 1.5))
    } else if drdata(item, 29) != 0 {
        Some((20, 1.25))
    } else if drdata(item, 28) != 0 {
        Some((10, 1.0))
    } else {
        None
    }
}

fn flask_ingredient_counts(item: &Item) -> [u8; 29] {
    let mut counts = [0; 29];
    for (idx, count) in counts.iter_mut().enumerate() {
        *count = drdata(item, idx + 11);
    }
    counts
}

fn c_div(power: i32, divi: f64, divisor: f64) -> i16 {
    (f64::from(power) / divi / divisor) as i16
}

fn c_scaled(power: i32, amount: u8, divi: f64, count: u8, divisor: f64) -> i16 {
    (f64::from(power) * f64::from(amount) / divi / f64::from(count) / divisor) as i16
}

fn flask_skill_mix(
    item: &Item,
    character: &Character,
    context: &ItemDriverContext,
) -> Option<([i16; MAX_MODIFIERS], [i16; MAX_MODIFIERS], u8, i32, u8)> {
    let mut power = flask_power(item, character, context);
    let (duration, divi) = flask_duration(item)?;
    if power <= 0 {
        return None;
    }

    let mut wis = drdata(item, 11);
    let mut inu = drdata(item, 12);
    let mut agi = drdata(item, 13);
    let mut strn = drdata(item, 14);
    let mut lfe = drdata(item, 15);
    let mut spr = drdata(item, 16);
    let mut end = drdata(item, 17);
    let count = wis + inu + agi + strn + lfe + spr + end;
    let fire = drdata(item, 31);
    let ice = drdata(item, 32);
    let hell = drdata(item, 34);

    power += i32::from(fire) * 4 + i32::from(ice) * 8 + i32::from(hell) * 12;
    let alchemist = alchemist_profession(character);
    for threshold in [20, 30, 40, 50] {
        if alchemist >= threshold {
            power += 4;
        }
    }

    let c_empty_modifier_index = || {
        let mut idx = [0; MAX_MODIFIERS];
        idx[0] = -1;
        idx[1] = -1;
        idx[2] = -1;
        idx
    };
    let single = |skill: CharacterValue, divisor: f64, value: i32| {
        let mut idx = c_empty_modifier_index();
        let mut val = [0; MAX_MODIFIERS];
        idx[0] = skill as i16;
        val[0] = c_div(power, divi, divisor);
        (idx, val, value)
    };
    let double = |a: CharacterValue, b: CharacterValue, divisor: f64, value: i32| {
        let mut idx = c_empty_modifier_index();
        let mut val = [0; MAX_MODIFIERS];
        idx[0] = a as i16;
        idx[1] = b as i16;
        val[0] = c_div(power, divi, divisor);
        val[1] = c_div(power, divi, divisor);
        (idx, val, value)
    };
    let triple =
        |a: CharacterValue, b: CharacterValue, c: CharacterValue, divisor: f64, value: i32| {
            let mut idx = c_empty_modifier_index();
            let mut val = [0; MAX_MODIFIERS];
            idx[0] = a as i16;
            idx[1] = b as i16;
            idx[2] = c as i16;
            val[0] = c_div(power, divi, divisor);
            val[1] = c_div(power, divi, divisor);
            val[2] = c_div(power, divi, divisor);
            (idx, val, value)
        };

    let (modifier_index, modifier_value, value_factor) =
        if count == 5 && wis == 1 && inu == 1 && agi == 2 && strn == 1 {
            triple(
                CharacterValue::Sword,
                CharacterValue::Attack,
                CharacterValue::Parry,
                4.0,
                10,
            )
        } else if count == 5 && wis == 1 && inu == 1 && agi == 1 && strn == 2 {
            triple(
                CharacterValue::TwoHand,
                CharacterValue::Attack,
                CharacterValue::Parry,
                4.0,
                10,
            )
        } else if count == 5 && agi == 1 && strn == 2 && lfe == 1 && spr == 1 {
            triple(
                CharacterValue::Attack,
                CharacterValue::Parry,
                CharacterValue::Immunity,
                4.0,
                10,
            )
        } else if count == 5 && inu == 1 && strn == 1 && lfe == 2 && spr == 1 {
            triple(
                CharacterValue::Flash,
                CharacterValue::MagicShield,
                CharacterValue::Immunity,
                4.0,
                10,
            )
        } else if count == 5 && strn == 2 && lfe == 2 && spr == 1 {
            triple(
                CharacterValue::Fireball,
                CharacterValue::MagicShield,
                CharacterValue::Immunity,
                4.0,
                10,
            )
        } else if count == 5 && inu == 1 && lfe == 2 && spr == 1 && end == 1 {
            triple(
                CharacterValue::Flash,
                CharacterValue::MagicShield,
                CharacterValue::Pulse,
                4.0,
                10,
            )
        } else if count == 5 && strn == 1 && lfe == 2 && spr == 1 && end == 1 {
            triple(
                CharacterValue::Fireball,
                CharacterValue::MagicShield,
                CharacterValue::Pulse,
                4.0,
                10,
            )
        } else if count == 5 && inu == 1 && strn == 2 && spr == 1 && end == 1 {
            triple(
                CharacterValue::Flash,
                CharacterValue::Immunity,
                CharacterValue::Pulse,
                4.0,
                10,
            )
        } else if count == 5 && strn == 3 && spr == 1 && end == 1 {
            triple(
                CharacterValue::Fireball,
                CharacterValue::Immunity,
                CharacterValue::Pulse,
                4.0,
                10,
            )
        } else if count == 4 && wis == 1 && inu == 1 && agi == 1 && strn == 1 {
            double(CharacterValue::Attack, CharacterValue::Parry, 3.0, 8)
        } else if count == 4 && inu == 1 && strn == 1 && lfe == 1 && spr == 1 {
            double(CharacterValue::Flash, CharacterValue::Immunity, 3.0, 8)
        } else if count == 4 && strn == 2 && lfe == 1 && spr == 1 {
            double(CharacterValue::Fireball, CharacterValue::Immunity, 3.0, 8)
        } else if count == 4 && strn == 1 && lfe == 2 && spr == 1 {
            double(
                CharacterValue::MagicShield,
                CharacterValue::Immunity,
                3.0,
                10,
            )
        } else if count == 4 && agi == 1 && end == 1 && lfe == 1 && spr == 1 {
            double(CharacterValue::Dagger, CharacterValue::Flash, 3.0, 8)
        } else if count == 4 && agi == 1 && strn == 1 && end == 1 && spr == 1 {
            double(CharacterValue::Dagger, CharacterValue::Fireball, 3.0, 8)
        } else if count == 4 && agi == 1 && strn == 1 && lfe == 1 && spr == 1 {
            double(CharacterValue::Staff, CharacterValue::Flash, 3.0, 8)
        } else if count == 4 && agi == 1 && strn == 2 && spr == 1 {
            double(CharacterValue::Staff, CharacterValue::Fireball, 3.0, 8)
        } else if count == 3 && strn == 2 && end == 1 {
            single(CharacterValue::Pulse, 2.0, 3)
        } else if count == 3 && agi == 2 && end == 1 {
            single(CharacterValue::Dagger, 2.0, 3)
        } else if count == 3 && agi == 1 && strn == 1 && end == 1 {
            single(CharacterValue::Staff, 2.0, 3)
        } else if count == 3 && agi == 2 && strn == 1 {
            single(CharacterValue::Sword, 2.0, 3)
        } else if count == 3 && agi == 1 && strn == 2 {
            single(CharacterValue::TwoHand, 2.0, 3)
        } else if count == 3 && inu == 1 && agi == 1 && strn == 1 {
            single(CharacterValue::Attack, 2.0, 3)
        } else if count == 3 && wis == 1 && agi == 1 && strn == 1 {
            single(CharacterValue::Parry, 2.0, 3)
        } else if count == 3 && inu == 2 && end == 1 {
            single(CharacterValue::Percept, 2.0, 3)
        } else if count == 3 && inu == 2 && agi == 1 {
            single(CharacterValue::Stealth, 2.0, 3)
        } else if count == 3 && agi == 2 && lfe == 1 {
            single(CharacterValue::BodyControl, 2.0, 3)
        } else if count == 3 && agi == 1 && end == 1 && spr == 1 {
            single(CharacterValue::Freeze, 2.0, 3)
        } else if count == 3 && lfe == 2 && spr == 1 {
            single(CharacterValue::MagicShield, 2.0, 3)
        } else if count == 3 && inu == 1 && lfe == 1 && spr == 1 {
            single(CharacterValue::Flash, 2.0, 3)
        } else if count == 3 && strn == 1 && lfe == 1 && spr == 1 {
            single(CharacterValue::Fireball, 2.0, 3)
        } else if count == 3 && strn == 2 && spr == 1 {
            single(CharacterValue::Immunity, 2.0, 3)
        } else if count == 3 && agi == 1 && strn == 1 && lfe == 1 {
            single(CharacterValue::Hand, 2.0, 3)
        } else if count == 3 && inu == 1 && strn == 1 && end == 1 {
            single(CharacterValue::Warcry, 2.0, 3)
        } else if count == 3 && wis == 1 && inu == 1 && agi == 1 {
            single(CharacterValue::Tactics, 2.0, 3)
        } else if count == 3 && inu == 1 && agi == 2 {
            single(CharacterValue::Surround, 2.0, 3)
        } else if count == 3 && wis == 1 && inu == 2 {
            single(CharacterValue::Barter, 2.0, 3)
        } else if count == 3 && wis == 1 && inu == 1 && spr == 1 {
            single(CharacterValue::Bless, 2.0, 3)
        } else if count == 3 && wis == 1 && inu == 1 && lfe == 1 {
            single(CharacterValue::Heal, 2.0, 3)
        } else if count == 3 && lfe == 1 && spr == 2 {
            single(CharacterValue::Duration, 2.0, 3)
        } else if count == 3 && strn == 2 && lfe == 1 {
            single(CharacterValue::Rage, 2.0, 3)
        } else if count != 0 {
            let mut idx = c_empty_modifier_index();
            let mut val = [0; MAX_MODIFIERS];
            for slot in 0..3 {
                if wis != 0 {
                    idx[slot] = CharacterValue::Wisdom as i16;
                    val[slot] = c_scaled(power, wis, divi, count, 4.0);
                    wis = 0;
                } else if inu != 0 {
                    idx[slot] = CharacterValue::Intelligence as i16;
                    val[slot] = c_scaled(power, inu, divi, count, 4.0);
                    inu = 0;
                } else if agi != 0 {
                    idx[slot] = CharacterValue::Agility as i16;
                    val[slot] = c_scaled(power, agi, divi, count, 4.0);
                    agi = 0;
                } else if strn != 0 {
                    idx[slot] = CharacterValue::Strength as i16;
                    val[slot] = c_scaled(power, strn, divi, count, 4.0);
                    strn = 0;
                } else if lfe != 0 {
                    idx[slot] = CharacterValue::Hp as i16;
                    val[slot] = c_scaled(power, lfe, divi, count, 2.0);
                    lfe = 0;
                } else if spr != 0 {
                    idx[slot] = CharacterValue::Mana as i16;
                    val[slot] = c_scaled(power, spr, divi, count, 2.0);
                    spr = 0;
                } else if end != 0 {
                    idx[slot] = CharacterValue::Endurance as i16;
                    val[slot] = c_scaled(power, end, divi, count, 1.0);
                    end = 0;
                }
            }
            (idx, val, 1)
        } else {
            return None;
        };

    if !modifier_value.iter().any(|value| *value != 0) {
        return None;
    }

    let value = value_factor * power * 13 + 50;
    let needs_class = if fire != 0 || ice != 0 || hell != 0 {
        8
    } else {
        0
    };
    Some((modifier_index, modifier_value, duration, value, needs_class))
}

fn finish_flask_mix(
    item: &mut Item,
    character: &Character,
    context: &ItemDriverContext,
) -> Option<()> {
    let (modifier_index, modifier_value, duration, value, needs_class) =
        flask_skill_mix(item, character, context)?;
    item.modifier_index = modifier_index;
    item.modifier_value = modifier_value;
    set_drdata(item, 2, 1);
    set_drdata(item, 3, duration);
    item.value = value.max(0) as u32;
    item.needs_class = needs_class;
    set_flask_magical_state(item);
    Some(())
}

pub fn reset_flask_empty_state(item: &mut Item) {
    let size = drdata(item, 0);
    item.name = "Empty Potion".to_string();
    match size {
        1 => {
            item.sprite = 10290;
            item.description = "A small flask made of glass.".to_string();
        }
        2 => {
            item.sprite = 10294;
            item.description = "A flask made of glass.".to_string();
        }
        3 => {
            item.sprite = 10302;
            item.description = "A big flask made of glass.".to_string();
        }
        _ => {}
    }
    item.driver_data.clear();
    item.driver_data.push(size);
    item.modifier_index = [0; MAX_MODIFIERS];
    item.modifier_value = [0; MAX_MODIFIERS];
    item.value = 10;
    item.needs_class = 0;
}

fn set_flask_magical_state(item: &mut Item) {
    item.name = "Magical Potion".to_string();
    match drdata(item, 0) {
        1 => {
            item.sprite = 50213;
            item.description = "A small flask containing a magical liquid.".to_string();
        }
        2 => {
            item.sprite = 50214;
            item.description = "A flask containing a magical liquid.".to_string();
        }
        3 => {
            item.sprite = 50253;
            item.description = "A big flask containing a magical liquid.".to_string();
        }
        _ => {}
    }
}

fn flask_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
    area_id: u16,
    in_arena: bool,
) -> ItemDriverOutcome {
    if character.id.0 == 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }
    if area_id == 34 && in_arena {
        return ItemDriverOutcome::BlockedByArea {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let size = drdata(item, 0);
    let used = drdata(item, 1);
    let shaken = drdata(item, 2) != 0;

    if shaken && character.cursor_item.is_some() {
        return ItemDriverOutcome::FlaskFinishedNoMoreIngredients {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let Some(cursor_item_id) = character.cursor_item else {
        if shaken {
            if !check_item_requirements(character, item) {
                return ItemDriverOutcome::BlockedByRequirements {
                    item_id: item.id,
                    character_id: character.id,
                };
            }
            return ItemDriverOutcome::AlchemyFlaskPotion {
                item_id: item.id,
                character_id: character.id,
                duration_minutes: drdata(item, 3),
                modifier_index: item.modifier_index,
                modifier_value: item.modifier_value,
            };
        }
        if used != 0 {
            let ingredient_counts = flask_ingredient_counts(item);
            if finish_flask_mix(item, character, context).is_some() {
                return ItemDriverOutcome::FlaskMixed {
                    item_id: item.id,
                    character_id: character.id,
                    ingredient_counts,
                };
            }
            reset_flask_empty_state(item);
            return ItemDriverOutcome::FlaskRuined {
                item_id: item.id,
                character_id: character.id,
                ingredient_counts,
            };
        }
        return ItemDriverOutcome::FlaskEmptyShaken {
            item_id: item.id,
            character_id: character.id,
        };
    };

    if context.cursor_template_id != Some(IID_ALCHEMY_INGREDIENT) {
        return ItemDriverOutcome::FlaskWrongCursor {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if used >= size.saturating_mul(3) {
        return ItemDriverOutcome::FlaskFull {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let ingredient_kind = context.cursor_drdata0.unwrap_or_default();
    if !(1..=29).contains(&ingredient_kind) {
        return ItemDriverOutcome::FlaskIngredientBug {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::FlaskIngredientAdded {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        ingredient_kind,
    }
}

fn lizard_flower_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
    area_id: u16,
) -> ItemDriverOutcome {
    if area_id != 31 {
        return ItemDriverOutcome::BlockedByArea {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if character.id.0 == 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::LizardFlowerNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    };

    if context.cursor_driver != Some(IDR_LIZARDFLOWER) {
        return ItemDriverOutcome::LizardFlowerDoesNotFit {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let combined_bits = drdata(item, 0) | context.cursor_drdata0.unwrap_or_default();
    ItemDriverOutcome::LizardFlowerMixed {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        combined_bits,
        complete: combined_bits == 7,
        bottle_message: item.sprite != 11189 && context.cursor_sprite != Some(11189),
    }
}

fn lab3_plant_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call && character.id.0 == 0 && drdata(item, 0) == 10 {
        return ItemDriverOutcome::Lab3WhiteBerryLightTick {
            item_id: item.id,
            destroyed: false,
        };
    }

    if character.id.0 == 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    match drdata(item, 0) {
        5 => {
            const OXYGEN_SECONDS: [u64; 5] = [3, 8, 10, 12, 15];
            let freshness = usize::from(drdata(item, 2).min(4));
            let count = u64::from(drdata(item, 1));
            ItemDriverOutcome::Lab3YellowBerry {
                item_id: item.id,
                character_id: character.id,
                duration_ticks: OXYGEN_SECONDS[freshness] * count * TICKS_PER_SECOND,
                installed: false,
            }
        }
        6 => {
            const LIGHT_POWER: [i16; 5] = [10, 30, 40, 45, 50];
            let freshness = usize::from(drdata(item, 2).min(4));
            let count = i16::from(drdata(item, 1));
            ItemDriverOutcome::Lab3WhiteBerry {
                item_id: item.id,
                character_id: character.id,
                light_power: LIGHT_POWER[freshness].saturating_mul(count),
                started_emit: false,
                installed: false,
            }
        }
        11 => ItemDriverOutcome::Lab3BrownBerry {
            item_id: item.id,
            character_id: character.id,
            duration_ticks: 10 * TICKS_PER_SECOND,
            installed: false,
        },
        _ => ItemDriverOutcome::Noop,
    }
}

pub fn book_text_lines(kind: u8) -> &'static [&'static str] {
    match kind {
        0 => &[
            "The magical properties of these skulls are astonishing. They are the artifacts the various shrines of the ancients accept, they can also be used to animate skeletons.",
            "After I told Moakin about them he used some magical stones to enhance the skulls and he created a small army of skeletons. Too bad his hunger for power made him go away without sharing his secrets with me.",
            "I wonder what became of him, and his puppet, Dorugin. But I digress. Now that Moakin has left, I will have to find out how to control the undead created with these skulls.",
            "My experiments have been successful in raising skeletons and zombies. A single plain skull can be used to create about a dozen of them. I wonder what one of the rare silver skulls would do.",
            "But I still have to control my creations. How Moakin managed to do that escapes me. I have tried various potions on those fools in Cameron to understand how to control a mind, but to no avail.",
            "It seems alchemy is worthless when it comes to control. I will have to resort to magical jewels. But those are hard to find. Maybe one of the shrines will produce some?",
        ],
        1 => &[
            "Healing Potions. Mana Potions. Torches. Small magical effects. Plain skulls are worthless. Must try the silver ones.",
            "Ahh. These zombies are dangerous. But they shall not stop me, Loisan. No luck with silver skulls either.",
            "Must try to get golden ones. But the danger...",
            "I am dying. So close. Oh, how cruel.",
        ],
        2 => &[
            "Day 122, year 48, morning by outside time. Personal diary of Ioslan of the Cerasa.",
            "We had to retreat further into the tunnels. The enemy is sending a new type of monster. Our creatures fight valiantly, but they cannot withstand them for long. We will have to flee, or we will perish.",
            "Armenicon has created more powerful creatures, but they fail to recognize us. Therefore, Armenicon added keywords to them, which will stun them for a short time, allowing us to flee from them.",
            "Once they are released, we will leave this part of the tunnel system and hope our enemy will invade and die. Today it is my turn to sneak through the tunnels and collect the skulls of our creatures.",
            "The enemy still has not learned the value of them. I just hope I will survive to flee with my kin.",
        ],
        3 => &[
            "Specimen 33. Prototype 4. Keyword: Nazimah.",
            "I will send this creature to guard the huge cavern. We cannot prevent the enemy from taking our storage room, but we can make him pay dearly for it.",
        ],
        4 => &[
            "Specimen 35. Prototype 4. Keyword: Argatoth.",
            "Another guard for our storage room.",
        ],
        5 => &[
            "Day 122, year 48, evening by outside time. Personal diary of Armenicon of the Cerasa.",
            "Ioslan has not returned. We cannot tell if he managed to recharge the spawners or not. We must flee immediately. The enemy will attack very soon.",
        ],
        6 => &[
            "Specimen 34. Prototype 4. Keyword: Lorganoth.",
            "Good. Prototype 4 is very difficult to create, but extremely powerful. This creature is to guard the storage room.",
        ],
        7 => &[
            "Specimen 36. Prototype 4. Keyword: Markanoth.",
            "The last prototype 4 for the storage room. These creatures are deadly.",
        ],
        8 => &[
            "There are two kinds of vampires. One is known under varying names, such as 'Vampire', 'Lesser Vampire', 'Dracul' or 'Necrifah'.",
            "Of the other kind, only a few sources report. They are called 'Vampire Lords' or 'Methusalah'.",
            "Killing a Lesser Vampire is as simple as penetrating it with a sword, or frying it with magic. They possess the abilities of the human they were once, but not much more.",
            "But killing a Vampire Lord on the other hand is very difficult, since each of them only has one weakness. Discovering that weakness is of utmost importance.",
            "Even if the weakness is known, it will still be a hard battle, as Vampire Lords are extremely old and powerful.",
        ],
        9 => &[
            "In a vision, I saw a sun shine in the darkness, and I saw fear in the eyes of the Lord.",
            "But then the sun was shattered, and parts of it fell into the dark. The Lord took them, and hid them in His lair.",
            "Then I saw Him leave His crypt, and come for me.",
        ],
        10 => &["One among many, one pointing sideways, part you shall find there. Cross I shall be with thee, shouldst thou fail."],
        11 => &[
            "'And,' said the wise, 'If ye are burning, my pupil, what shall ye do?'",
            "'Extinguish the flames, master?'",
        ],
        12 => &[
            "Take heed, and go no further! This way leads to the Vampire Lord!",
            "It is said that one strike with the right dagger will kill the Lord. But alas, many have tried, but no one found the right dagger.",
        ],
        20 => &[
            "Day 91, year 97, evening by outside time. Personal diary of Avaisor of the Isara.",
            "The struggle seems hopeless now. We're trapped in these caverns by our own defense systems. We can no longer control them as the key was lost when Daoslan was slain by demons in the southern part of the natural caverns.",
            "Our desperate attempts to raise demons for our defense have failed so far. Some of the research labs had to be closed since the demons in them could no longer be controlled.",
        ],
        21 => &[
            "Day 58, year 97, memo on the state of War by Seraios of the Isara",
            "Only one adversary remains after the glorious defeat of Keriaos. But it is a dangerous one. Islena has persuaded four of our enemies to join forces with her, and she will gather all her allies in the north to form an army capable of destroying us.",
            "We must make our move first and attack before she is ready. I advise that we attack Islena's headquarter with...",
            "(The remaining pages are burned.)",
        ],
        22 => &[
            "Day 84, year 97, evening by outside time. Personal diary of Delasar of the Isara.",
            "I have established two outposts beyond our defense line to the north. They will allow me to study the demons as they are attacking our defense systems. I might be able to find other means to protect us this way.",
            "Going there is dangerous, and I might not make it back with my knowledge. I will keep other diaries there, so that my clan will be able to use my findings even after my death.",
            "I have asked Avaisor to turn off the defense systems in an hour so that I can reach my outpost. Fate, let me survive!",
        ],
        23 => &[
            "Day 55, year 97, evening by outside time. Personal diary of Isranor of the Cerasa.",
            "Our glorious leader, Carisar, has joined forces with Islena of the Ilasner. Our talks with our direct enemy the Isara have failed. Too much blood was shed already and neither they nor we could overcome the hatred. But still, I was impressed by Ishtar, their leader.",
            "In spite of our alliance with the Ilasner, our position here is quite hopeless. The Isara will soon be forced to attack with all their might. Our defenses cannot withstand them for long. We will abandon this position within the next few days.",
            "Fortunately, we made good progress with our demonic research project. We will not suffer much from the demons that escaped during the early stages of our research. And we can hope that they will delay the Isara's pursuit.",
            "We will open the demon-gate before we leave. The steady flood of demons coming from it will give us the time we need and hinder the Isara.",
        ],
        24 => &[
            "Day 155, year 103. Personal diary of Kamaleon of the Isara.",
            "In our pursuit of Islena's forces we have finally reached one of her former settlements. The long march north, first through that fiery maze full of lava and unmanned yet dangerous defense stations and now through these icy caverns has tired us beyond measure.",
            "So many friends lost, so many deaths. And yet we must press on after only a few days rest, lest we give Islena time to counter-attack and crush us while we are defenseless. I wonder how far this pursuit will take us. We have come so far north in these caverns, we must be below the sea already.",
            "But we will not stop, it would mean death. Mind, be tranquil, this will end.",
        ],
        25 => &[
            "Day 158, year 103. Personal diary of Ileanor, Lieutenant of the Isara.",
            "The three days rest we have given our men are all the time we can spare. Not all wounds are healed, and the men are still tired, but delaying further would leave us open to a counter-attack. I wonder what the Ilasner are up to. It is not like them to give up this much ground without any resistance.",
            "Tomorrow at dawn, well, tomorrow when we wake up, we will move on. We still have some wood to build fires to break the ice demon's spell, and the morale is as well as can be expected under these circumstances. I am greatly worried, though. We haven't seen the surface for years, and all the explosions we heard a few weeks ago mean the war is raging there as savage as it is here.",
        ],
        26 => &[
            "Day 145, year 103. Personal diary of Cari-Maar of the Ilasner.",
            "Today we were ordered to retreat. The defense stations and the fire demons have delayed our pursuers long enough. Rumor has it that Islena and the main force have established a defensive position further to the north.",
            "But our scouts report that those cursed Isara have managed to bring half their forces through alive. We are vastly outnumbered. We can only hope that the fortified positions will give us enough advantage to make up for our lack in numbers.",
        ],
        27 => &[
            "Contrary to my original belief, the swamp beasts possess no intelligence. The buildings they inhabitate must have been built by a now extinct people. I assume that the three stone circles have been built by the same people.",
            "Some pages later: I have discovered old drawings, showing humans fighting against swamp beasts. In the first pictures the humans flee from a huge beast. A bit further down, one of the drawings shows a human warrior standing in the center of a stone circle, holding a weapon in his hand. Strangely, it shows the sun being exactly below the warrior and the ground. The warrior seems to be waiting, and looking at the sun. In the next drawing, he is still standing in the stone circle, but now he is killing a small swamp beast. His weapon seems to be glowing.",
        ],
        28 => &[
            "Day 172, year 103. Personal diary of Cari-Maar of the Ilasner.",
            "Today we finished raising the demon lord for the trap we've built for the Isara. Let them come now, they are doomed.",
        ],
        29 => &[
            "Day 175, year 103. Personal diary of Ileanor, Lieutenant of the Isara.",
            "Dead. All dead. Only Ishtar and I survived the storm of demon lords the Ilasner raised. We could flee, but we are locked into these rooms. The demon lords cannot enter, but they have begun to invoke the icy cold. We will freeze to death.",
            "Day 177, year 103. Personal diary of Ileanor, Lieutenant of the Isara.",
            "The cold is creeping into my bones. Ishtar has kept us alive so far, but now he is exhausted and cannot sustain the heating spell. I think the whole palace is frozen.",
            "Why, oh why did we have to fight this war? The world was so beautiful, and so were we. But now, all that remains is blood and tears. If anyone survives this folly, let our fate teach you not to repeat our mistakes!",
        ],
        30 => &[
            "Day 175, year 103. Personal diary of Islena.",
            "The cursed Isara are caught in our trap. Now they will die, all of them will die.",
            "Day 176, year 103. Personal diary of Islena.",
            "It seems some of them got away. The demon lords are out of control and trying to freeze them to death. I can feel the cold even here, in my rooms.",
            "Day 177, year 103. Personal diary of Islena.",
            "The cold is slowly killing all of us. All attempts to control the demon lords have failed. Now all of us must die. But I shall die happily if I can take Ishtar with me into the cold.",
        ],
        31 => &[
            "Personal Diary of Korzam, Magical Advisor of Scarcewind.",
            "The line above has been nearly scratched out, and replaced by:",
            "Personal Diary of Korzam, Governor of Exkordon.",
            "Scarcewind, the fool, is still loyal to Aston. He does not understand that the only way for our city to prosper is to cut our ties to that rotten empire. What good is an advisor, if no one listens to him?",
            "To get my mind on other things, I have gone north, into the barren lands below the mountains, hunting rumors. It is said that huge towers are build on those plains, and in those mountains. Towers built by powerful wizards of the old age. Whoever started these rumors has his history wrong, that is for sure. There was no old age. Before us were the ancients. They destroyed each other, and the world, in their foolish war. After them came we, and Ishtar and his notions of godhood and the empire.",
            "But if these towers are really there, and if they are as magical as the rumors say, who built them? Who else but the ancients! There was no one else who could have built them. And if the ancients are the makers, those towers are old and must have survived the destructions of the war. I want to see what kind of magic can make buildings survive what has shattered the earth.",
            "You skip several pages containing a description of the voyage to the towers.",
            "I have forced my way into one of the towers. Magical they are, for sure, and guarded by the living dead. Fighting my way inside nearly exhausted me, and all I could do was grab some parchments and a small bag and flee, before those undead came back in greater numbers.",
            "The book is written in the language of the ancients. Unfortunately, I can barely understand some words. The bag contained polished pieces of bone, each bearing a rune. I will return to Exkordon now, and study them at my leisure.",
            "I found some pictures in the book, showing how to arrange the runes. I wonder what will happen...",
            "You notice a change in the writing. It is the same hand, but the letters are bigger, and more forcefully written.",
            "That does it. Scarewind is a weak fool. I shall kill him, and take Exkordons fate into my own hands.",
            "Easy, almost too easy it was. I am now Governor of Exkordon. Scarcewind died like the fool he was in life. 'How can you do that? Why? I trusted you!' What a fool. I invited him into my house, told him about an important discovery I made. He came, and left his guards outside. And so he died. When his guards came looking for him, I lured them into my cellars, and disposed of them. They are no match for the ancient's magic.",
            "Here, the writing changes back to the style used in the beginning.",
            "What have I done? What came over me? And why are the dead rising, and walking my halls? They are dead! Dead! I killed them!",
        ],
        32 => &["Once leads on, twice is rewarding, three times is dangerous."],
        33 => &["Two Berkano flanking an Ansuz will give thee Endurance."],
        34 => &["Berkano, Dagaz, Ansuz is healthy."],
        35 => &["Ansuz and Dagaz twice - good for Mages."],
        36 => &["Ansuz, Ehwaz, Dagaz - better defense for the Warrior."],
        37 => &["Ehwaz twice followed by Berkano - better defense for the Mage."],
        38 => &["Berkano, Ehwaz, Ansuz will decrease magic damage."],
        39 => &[
            "Day 12, year 45. Personal diary of Sluiran of the Caremar.",
            "The battles raging outside are closer to our hiding place. We must find some means to defend ourselves. I have started to study the forbidden art of necromancy, based on the rune magic. The undead shall fight where the living cannot.",
            "You skip some pages.",
            "Day 37, year 47. Personal diary of Sluiran of the Caremar.",
            "The towers have fallen, but the undead have held our halls against the first wave of attackers. I have many, many bodies for my work now. More and more undead shall defend us. We might survive, after all.",
        ],
        40 => &[
            "Day 213, year 61. Personal diary of Sluiran of the Caremar.",
            "We have been attacked by demons again, and we are running out of dead bodies to raise in our defense. We can no longer reach those in the outer halls. It will not take long before they take our last defenses. But they shall not gain any profit by this. I shall cast a spell that will raise all dead in these halls over and over again. So we will continue the fight, even after we are dead.",
        ],
        41 => &[
            "My dear Sarkilar,",
            "thine shall be the land from rotten Exkordon to the icy shores Valkyries. It is ripe, ready for thee to take it. The magic of the Kir should give thee sufficient strength. Take as many of the young monks as thou canst, and cloud their mind, as I taught thee. Once thy force is strong enough, take the land which is promised thee.",
            "Islena",
        ],
        42 => &["My wounds are too much to bear and I fear that I will not survive. I have found none of the parts of the Talisman of the Moon, nor the location of the Moon Pool in which to enchant it. I have failed to find a way to lift the curse off my old friend, and I am sorry."],
        43 => &["Thou canst comprehend the intricate handwriting fully, something about an incantation of transportation. It sounds like folly and you choose not to decipher more of the scribbles."],
        44 => &[
            "It is a long list of names, the masters and teachers of the mages order. With deep respect, the great past masters of the mages order, are here honoured.",
            "Wijn, the old one. Gree-Dli, master of summoning spells, Leerea, the empat, Djurna bridgecaller, Friize the recluse, Loisan creator. ",
            "At the bottom of the following page you find a list of the current teachers of the mages order: Bretl, Anna-Sofia, Leaner, Crem, Guiwynn.",
            "It appears that someone has attempted to scratch away the final name from the parchment.",
        ],
        45 => &[
            "Sacred potions",
            "There are rumors saying a potion can be created, which holds the insignia of the very Ishtar himself. Bestowing his blessing upon the user. Imagine! The potency of such a liquid! Some of the ingredients are obvious.",
            "Sulphur for preserving power in bottled form. Some kind of transformative agent must be added, how else can mere mortals consume the insignia without being entirely overcome by it? Madness it is to directly consume such an element. And madness will be the curse upon those who attempt it.",
            "Here art no choice but to explore by testing the potions out. To balance the splendor of Ishtar's insignia it must contain a liquid harmful to humans. A poison or venom most likely. Possibly from a mushroom.",
            "My first attempt ready now. The coloration looks promising, and the volunteers are ready. This will either be a splendid achievement worthy a record in the great library of Exkordon, or a good reason for me to go under ground.",
        ],
        46 => &["This is an arena. Death on the sand incurs none of the usual penalties of death. Thou shall not loose saves, experience, equipment or gold"],
        100 => &[
            "The pages are badly burned. You can only read: All those heros who tried to kill my brother died through his hands. To keep these young hotheads away, I summoned a demon to guard the entrance and ordered him to let no one pass but me. He is a bit short-sighted, but...",
            "My brother must be killed, or the horror will never stop. He is my brother, but he must die for his misdeeds...",
            "The last fight with the undeads was hard. But even though I am bleeding from many wounds, today is the day I will kill my brother. I will take the amulet and go into the family vault and face him now!",
        ],
        101 => &["Most of the page is burned, but you can read: To prevent holy water from hurting him, and his minions, my brother created a anti-magic zone which dispells all holy effects and all magic. But I have found a way to break this spell. I created an amulet to hold the counter-spell..."],
        _ => &[],
    }
}

pub fn book_text_line_bytes(kind: u8) -> Vec<Vec<u8>> {
    book_text_line_bytes_for_reader(kind, 0)
}

pub fn book_text_line_bytes_for_reader(kind: u8, demon_value: i32) -> Vec<Vec<u8>> {
    book_text_line_bytes_for_reader_id(kind, demon_value, 0)
}

pub fn book_text_line_bytes_for_reader_id(
    kind: u8,
    demon_value: i32,
    reader_id: u32,
) -> Vec<Vec<u8>> {
    match kind {
        13..=17 => demon_book_line_bytes(kind, reader_id),
        18 => edemon_sign_line_bytes(demon_value, &["Defense Systems Control Room"]),
        19 => edemon_sign_line_bytes(
            demon_value,
            &["Research Laboratorium", "Caution, live demons!"],
        ),
        31 => vec![
            plain_book_line_bytes("Personal Diary of Korzam, Magical Advisor of Scarcewind."),
            dark_gray_book_line_bytes("The line above has been nearly scratched out, and replaced by:", false),
            plain_book_line_bytes("Personal Diary of Korzam, Governor of Exkordon."),
            plain_book_line_bytes("Scarcewind, the fool, is still loyal to Aston. He does not understand that the only way for our city to prosper is to cut our ties to that rotten empire. What good is an advisor, if no one listens to him?"),
            plain_book_line_bytes("To get my mind on other things, I have gone north, into the barren lands below the mountains, hunting rumors. It is said that huge towers are build on those plains, and in those mountains. Towers built by powerful wizards of the old age. Whoever started these rumors has his history wrong, that is for sure. There was no old age. Before us were the ancients. They destroyed each other, and the world, in their foolish war. After them came we, and Ishtar and his notions of godhood and the empire."),
            plain_book_line_bytes("But if these towers are really there, and if they are as magical as the rumors say, who built them? Who else but the ancients! There was no one else who could have built them. And if the ancients are the makers, those towers are old and must have survived the destructions of the war. I want to see what kind of magic can make buildings survive what has shattered the earth."),
            dark_gray_book_line_bytes("You skip several pages containing a description of the voyage to the towers.", false),
            plain_book_line_bytes("I have forced my way into one of the towers. Magical they are, for sure, and guarded by the living dead. Fighting my way inside nearly exhausted me, and all I could do was grab some parchments and a small bag and flee, before those undead came back in greater numbers."),
            plain_book_line_bytes("The book is written in the language of the ancients. Unfortunately, I can barely understand some words. The bag contained polished pieces of bone, each bearing a rune. I will return to Exkordon now, and study them at my leisure."),
            plain_book_line_bytes("I found some pictures in the book, showing how to arrange the runes. I wonder what will happen..."),
            dark_gray_book_line_bytes("You notice a change in the writing. It is the same hand, but the letters are bigger, and more forcefully written.", false),
            plain_book_line_bytes("That does it. Scarewind is a weak fool. I shall kill him, and take Exkordons fate into my own hands."),
            plain_book_line_bytes("Easy, almost too easy it was. I am now Governor of Exkordon. Scarcewind died like the fool he was in life. 'How can you do that? Why? I trusted you!' What a fool. I invited him into my house, told him about an important discovery I made. He came, and left his guards outside. And so he died. When his guards came looking for him, I lured them into my cellars, and disposed of them. They are no match for the ancient's magic."),
            dark_gray_book_line_bytes("Here, the writing changes back to the style used in the beginning.", false),
            plain_book_line_bytes("What have I done? What came over me? And why are the dead rising, and walking my halls? They are dead! Dead! I killed them!"),
        ],
        39 => vec![
            plain_book_line_bytes("Day 12, year 45. Personal diary of Sluiran of the Caremar."),
            plain_book_line_bytes("The battles raging outside are closer to our hiding place. We must find some means to defend ourselves. I have started to study the forbidden art of necromancy, based on the rune magic. The undead shall fight where the living cannot."),
            dark_gray_book_line_bytes("You skip some pages.", false),
            plain_book_line_bytes("Day 37, year 47. Personal diary of Sluiran of the Caremar."),
            plain_book_line_bytes("The towers have fallen, but the undead have held our halls against the first wave of attackers. I have many, many bodies for my work now. More and more undead shall defend us. We might survive, after all."),
        ],
        43 => vec![dark_gray_book_line_bytes("Thou canst comprehend the intricate handwriting fully, something about an incantation of transportation. It sounds like folly and you choose not to decipher more of the scribbles.", false)],
        44 => vec![
            plain_book_line_bytes("It is a long list of names, the masters and teachers of the mages order. With deep respect, the great past masters of the mages order, are here honoured."),
            plain_book_line_bytes("Wijn, the old one. Gree-Dli, master of summoning spells, Leerea, the empat, Djurna bridgecaller, Friize the recluse, Loisan creator. "),
            dark_gray_book_line_bytes("At the bottom of the following page you find a list of the current teachers of the mages order: ", true),
            dark_gray_book_line_bytes("It appears that someone has attempted to scratch away the final name from the parchment.", false),
        ],
        _ => book_text_lines(kind)
            .iter()
            .map(|line| plain_book_line_bytes(line))
            .collect(),
    }
}

fn demon_book_line_bytes(kind: u8, reader_id: u32) -> Vec<Vec<u8>> {
    let ritual = demonspeak(reader_id, u32::from(kind - 13));
    let line = match kind {
        13 => format!(
            "I have seen in written in fiery letters upon the sky: Those who have the knowledge can invoke protection against demonic might by uttering the words: '{ritual}'"
        ),
        14 => format!(
            "Those who need better protection against earth demons, those who have the knowledge, use these words: '{ritual}'"
        ),
        15..=17 => format!("'{ritual}' will give thee even better protection."),
        _ => return Vec::new(),
    };
    vec![plain_book_line_bytes(&line)]
}

fn demonspeak(reader_id: u32, nr: u32) -> String {
    const SYLLABLES: [&str; 10] = [
        "shir", "ka", "dor", "lagh", "kir", "dul", "arl", "sli", "dlu", "usga",
    ];
    const LEADS: [&str; 5] = ["ki", "do", "sa", "mi", "ru"];

    let mut val = id_rand(reader_id, nr);
    let v1 = (val % SYLLABLES.len() as u32) as usize;
    val >>= 4;
    let v2 = (val % SYLLABLES.len() as u32) as usize;
    val >>= 3;
    let v3 = (val % SYLLABLES.len() as u32) as usize;
    val >>= 5;
    let v4 = (val % SYLLABLES.len() as u32) as usize;
    let lead = LEADS.get(nr as usize).copied().unwrap_or(LEADS[0]);

    format!(
        "{}{} {}{}{}",
        SYLLABLES[v1], SYLLABLES[v2], lead, SYLLABLES[v3], SYLLABLES[v4]
    )
}

fn id_rand(base: u32, step: u32) -> u32 {
    const VALUES: [u32; 16] = [
        0x12345678, 0x87654321, 0x17263524, 0xabef53ac, 0xbd341ace, 0x1045fe45, 0xea6deb2a,
        0x1d40fb4a, 0x1a83be1d, 0x1d441eff, 0x1a15e63f, 0x192502de, 0x90ae3ce2, 0x1de94be3,
        0x1e358f3b, 0xa1e3ff56,
    ];
    let mut ret = base
        .wrapping_add(step)
        .wrapping_add(base.wrapping_mul(step));
    for _ in 0..4 {
        ret ^= VALUES[(ret % VALUES.len() as u32) as usize];
    }
    ret
}

fn edemon_sign_line_bytes(demon_value: i32, readable_lines: &[&str]) -> Vec<Vec<u8>> {
    if demon_value < 1 {
        return vec![plain_book_line_bytes(
            "It's written in strange letters you cannot read.",
        )];
    }
    if demon_value < 2 {
        return vec![plain_book_line_bytes(
            "You recognice some of the letters used in this sign from your studies of the ancient knowledge, but you cannot tell what the sign means.",
        )];
    }
    readable_lines
        .iter()
        .map(|line| plain_book_line_bytes(line))
        .collect()
}

pub fn book_nook_joke_line_bytes(roll: u32) -> Vec<Vec<u8>> {
    let lines: &[&str] = match roll % 5 {
        0 => &[
            "What did the fisherman say to the card magician?",
            "Pick a cod, any cod!",
        ],
        1 => &[
            "Who can shave 25 times a day and still have a beard?",
            "A barber.",
        ],
        2 => &[
            "Did you hear about the fire at the circus?",
            "It was in tents.",
        ],
        3 => &[
            "What did the rude prism say to the light beam that smacked into him?",
            "Get bent!",
        ],
        _ => &["What bone will a dog never eat?", "A trombone."],
    };
    lines
        .iter()
        .map(|line| plain_book_line_bytes(line))
        .collect()
}

pub fn book_special_effect(kind: u8) -> Option<u32> {
    match kind {
        22 => Some(50287),
        23 => Some(50305),
        _ => None,
    }
}

fn plain_book_line_bytes(line: &str) -> Vec<u8> {
    line.as_bytes().to_vec()
}

fn dark_gray_book_line_bytes(line: &str, reset_before_current_teachers: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(line.len() + 32);
    out.extend_from_slice(COL_DARK_GRAY);
    out.extend_from_slice(line.as_bytes());
    if reset_before_current_teachers {
        out.extend_from_slice(COL_RESET);
        out.extend_from_slice(b"Bretl, Anna-Sofia, Leaner, Crem, Guiwynn.");
    }
    out
}

fn book_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::BookText {
        item_id: item.id,
        character_id: character.id,
        kind: drdata(item, 0),
        demon_value: i32::from(character.values[1][CharacterValue::Demon as usize]),
    }
}

const TWOCITY_COLORS: [&str; 7] = ["null", "Red", "Green", "Blue", "Yellow", "Black", "White"];

const BOOKCASE_RANDOM_TITLES: [&str; 26] = [
    "Tales of Two Towns by Karl Dicker",
    "The Art of Warfare by Hun Yu",
    "Chris Maas visits Carol by Karl Dicker",
    "Secrets of Adygalah Alchemy by Leonarda",
    "The rise and fall of the Seyan Empire by Takitus",
    "History of Ancient Astonia by Chiasmaphora",
    "Treatise on the Mastery of Mana by Mage Niuma",
    "The Song of the Warrior by Sir Regis Le Voleir",
    "The Book of Ishtar, Anonymous",
    "Concessions to Fear by Kentindher",
    "Poems of War and Homecoming by Melthold of Anten",
    "Memoires of a Lady-in-Waiting by Dame Sakanor",
    "Comprehension and Expression by Master Getsades",
    "Great Astonian Thinkers by Master Riotan",
    "A Portrait of the Seyan'Du as A Young Mage by Esjamocey",
    "Critique of Pure Courage by Imanel Dique",
    "Collected Essays by Lindmar the Elder",
    "The Reforming of Curves by Master Elyosod",
    "Advanced Agility in Forty-two Steps by Seyan'Du Bartoshi",
    "The Oath by Sheney",
    "The Strife for Light by Father Ignato",
    "The Aston Years by Lord Ironborn",
    "Luctim - Superstition or Reality? by Mintu the Enlightened",
    "I Have, Alas by Goytila",
    "A Midwinter Day's Wake by Pearshaks",
    "Fama Fraternitatis by Valentin Andreae",
];

pub fn bookcase_text_line_bytes(
    kind: u8,
    random_index: u8,
    color: u8,
    solved_library: bool,
) -> Vec<u8> {
    let standard = "After reading the title you put the book back.";
    let color = TWOCITY_COLORS
        .get(usize::from(color))
        .copied()
        .unwrap_or(TWOCITY_COLORS[0]);
    let (name, text) = match kind {
        0 => {
            let idx = usize::from(random_index % BOOKCASE_RANDOM_TITLES.len() as u8);
            let text = if idx == 3 {
                "One recipe most mages will find useful uses Adygalah, Bhalkissa and Firuba, plus one berry and one or two mushrooms."
            } else {
                standard
            };
            (BOOKCASE_RANDOM_TITLES[idx].to_string(), text)
        }
        1 => {
            let text = if solved_library {
                standard
            } else {
                "You read the book and absorb the knowledge contained therein."
            };
            ("The Knowledge of Ages by Ishtar".to_string(), text)
        }
        2 => (format!("How to Raise {color} Orchids by Klark"), standard),
        3 => (
            format!("A {color} Day in the Life of a Warrior by C. O. Nan"),
            standard,
        ),
        4 => (
            format!("Dancing in Ten Easy Lessons by James {color}"),
            standard,
        ),
        5 => (
            format!("Help! I Have Been Visited by Little {color} Man! by Meier"),
            standard,
        ),
        6 => (
            format!("The Day the World turned {color} by Casaldra"),
            standard,
        ),
        _ => (
            "Lady Manners' Guide to Decent Behaviour".to_string(),
            standard,
        ),
    };

    let mut out = Vec::new();
    out.extend_from_slice(COL_LIGHT_GREEN);
    out.extend_from_slice(name.as_bytes());
    out.extend_from_slice(b".");
    out.extend_from_slice(COL_RESET);
    out.extend_from_slice(b" ");
    out.extend_from_slice(text.as_bytes());
    out
}

pub fn bookcase_locked_text_lines() -> [&'static str; 2] {
    [
        "The bookcase is locked and you do not have the right key.",
        "There is a note attached to the lock: A statue stole the key and vanished with it in the northern part of the library.",
    ]
}

fn bookcase_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    if !character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }

    let kind = drdata(item, 0);
    if kind == 1 && !context.has_area17_library_key {
        return ItemDriverOutcome::BookcaseLocked {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::BookcaseText {
        item_id: item.id,
        character_id: character.id,
        kind,
    }
}

fn edemonball_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if !context.timer_call || character.id.0 != 0 {
        return ItemDriverOutcome::Noop;
    }

    let item_x = i32::from(item.x);
    let item_y = i32::from(item.y);
    let strength = i32::from(drdata(item, 2));
    let base_sprite = i32::from(drdata(item, 1));
    let shot = match drdata(item, 3) {
        0 => Some((item_x, item_y + 1, item_x, item_y + 10)),
        1 => Some((item_x, item_y + 1, item_x + 1, item_y + 10)),
        2 => Some((item_x, item_y + 1, item_x - 1, item_y + 10)),
        3 => Some((item_x, item_y - 1, item_x, item_y - 10)),
        4 => Some((item_x, item_y - 1, item_x + 1, item_y - 10)),
        5 => Some((item_x, item_y - 1, item_x - 1, item_y - 10)),
        6 => Some((item_x + 1, item_y, item_x + 10, item_y)),
        7 => Some((item_x + 1, item_y, item_x + 10, item_y + 1)),
        8 => Some((item_x + 1, item_y, item_x + 10, item_y - 1)),
        9 => Some((item_x - 1, item_y, item_x - 10, item_y)),
        10 => Some((item_x - 1, item_y, item_x - 10, item_y + 1)),
        11 => Some((item_x - 1, item_y, item_x - 10, item_y - 1)),
        _ => None,
    };

    let Some((start_x, start_y, target_x, target_y)) = shot else {
        set_drdata(item, 3, 0);
        return ItemDriverOutcome::Noop;
    };
    set_drdata(item, 3, drdata(item, 3).saturating_add(1));

    ItemDriverOutcome::EdemonBallProjectile {
        item_id: item.id,
        character_id: character.id,
        start_x: clamp_legacy_coordinate(start_x),
        start_y: clamp_legacy_coordinate(start_y),
        target_x: clamp_legacy_coordinate(target_x),
        target_y: clamp_legacy_coordinate(target_y),
        strength,
        base_sprite,
        schedule_after_ticks: TICKS_PER_SECOND * 16,
    }
}

fn flamethrow_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if !context.timer_call || character.id.0 != 0 {
        return ItemDriverOutcome::Noop;
    }

    let fire = drdata(item, 0);
    if fire != 0 {
        set_drdata(item, 0, fire.saturating_sub(1));
        if drdata(item, 2) == 0 {
            item.sprite += 1;
            set_drdata(item, 2, 1);
            item.modifier_index[4] = V_LIGHT;
            item.modifier_value[4] = 250;
        }
        return ItemDriverOutcome::FlameThrowerPulse {
            item_id: item.id,
            character_id: character.id,
            direction: drdata(item, 1),
            schedule_after_ticks: 1,
        };
    }

    item.sprite -= 1;
    set_drdata(item, 0, TICKS_PER_SECOND as u8);
    set_drdata(item, 2, 0);
    item.modifier_index[4] = 0;
    item.modifier_value[4] = 0;
    let delay_seconds = drdata(item, 3);

    ItemDriverOutcome::FlameThrowerExtinguished {
        item_id: item.id,
        character_id: character.id,
        schedule_after_ticks: (delay_seconds != 0)
            .then_some(TICKS_PER_SECOND.saturating_mul(u64::from(delay_seconds))),
    }
}

fn extinguish_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::Extinguish {
        item_id: item.id,
        character_id: character.id,
        extinguished: false,
    }
}

fn spiketrap_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call || character.id.0 == 0 {
        if drdata(item, 0) != 0 {
            item.sprite -= 1;
            set_drdata(item, 0, 0);
            return ItemDriverOutcome::SpikeTrapReset { item_id: item.id };
        }
        return ItemDriverOutcome::Noop;
    }

    if drdata(item, 0) != 0 {
        return ItemDriverOutcome::Noop;
    }

    item.sprite += 1;
    set_drdata(item, 0, 1);
    ItemDriverOutcome::SpikeTrapTriggered {
        item_id: item.id,
        character_id: character.id,
        damage: i32::from(drdata(item, 1)) * crate::entity::POWERSCALE,
        reset_after_ticks: TICKS_PER_SECOND,
    }
}

fn usetrap_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::TriggerMapItem {
        item_id: item.id,
        character_id: character.id,
        x: u16::from(drdata(item, 0)),
        y: u16::from(drdata(item, 1)),
        target_character_id: character.id,
        delay_ticks: TICKS_PER_SECOND / 2,
    }
}

fn steptrap_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call || character.id.0 == 0 {
        if drdata(item, 0) == 0 {
            return ItemDriverOutcome::StepTrapDiscoverTarget { item_id: item.id };
        }
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::TriggerMapItem {
        item_id: item.id,
        character_id: character.id,
        x: u16::from(drdata(item, 0)),
        y: u16::from(drdata(item, 1)),
        target_character_id: CharacterId(0),
        delay_ticks: 1,
    }
}

fn orbspawn_driver(character: &Character, item: &Item, anti: bool) -> ItemDriverOutcome {
    if character.cursor_item.is_some()
        || character.level < u32::from(item.min_level)
        || !character.flags.contains(CharacterFlags::PAID)
    {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::OrbSpawn {
        item_id: item.id,
        character_id: character.id,
        anti,
        special: anti && drdata(item, 0) == 1,
    }
}

fn nomad_stack_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::NomadStack {
        item_id: item.id,
        character_id: character.id,
    }
}

fn transport_driver(character: &Character, item: &Item, spec: i32) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    if spec != 0 {
        return ItemDriverOutcome::TransportTravel {
            item_id: item.id,
            character_id: character.id,
            spec,
        };
    }

    let point = drdata(item, 0);
    if point != LEGACY_TRANSPORT_CLAN_EXIT && point >= LEGACY_TRANSPORT_POINT_COUNT {
        return ItemDriverOutcome::TransportInvalid {
            item_id: item.id,
            character_id: character.id,
            point,
        };
    }

    ItemDriverOutcome::TransportOpen {
        item_id: item.id,
        character_id: character.id,
        point,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArenaToplistEntry {
    pub name: String,
    pub score: i32,
}

pub fn arena_toplist_lines(
    entries: &[ArenaToplistEntry],
    score: i32,
    wins: i32,
    losses: i32,
    fights: i32,
) -> Vec<String> {
    let player_score = if fights == 0 { -2000 } else { score };
    let mut lines = Vec::new();

    for (index, entry) in entries.iter().take(10).enumerate() {
        if entry.name.is_empty() {
            break;
        }
        lines.push(format!("{}: {} {}", index + 1, entry.name, entry.score));
    }

    let mut rank_index = 10usize;
    while rank_index < entries.len().min(100) {
        let entry = &entries[rank_index];
        if entry.name.is_empty() || entry.score < player_score {
            break;
        }
        rank_index += 1;
    }

    let start = rank_index.saturating_sub(5).max(10);
    let end = (rank_index + 5).min(entries.len()).min(100);
    for index in start..end {
        let entry = &entries[index];
        if entry.name.is_empty() {
            break;
        }
        lines.push(format!("{}: {} {}", index + 1, entry.name, entry.score));
    }

    lines.push(format!(
        "Your score is {player_score}, you have won {wins} fights and lost {losses} fights."
    ));
    lines
}

fn toplist_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::ArenaToplist {
        item_id: item.id,
        character_id: character.id,
    }
}

fn xmasmaker_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0
        || !character
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
    {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::XmasMaker {
        item_id: item.id,
        character_id: character.id,
    }
}

fn zombie_shrine_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let shrine_type = drdata(item, 0);
    let required_skull = match shrine_type {
        0 => IID_AREA2_ZOMBIESKULL1,
        1 => IID_AREA2_ZOMBIESKULL2,
        _ => IID_AREA2_ZOMBIESKULL3,
    };
    if context.cursor_template_id != Some(required_skull) {
        return ItemDriverOutcome::ZombieShrineNeedsOffering {
            item_id: item.id,
            character_id: character.id,
            shrine_type,
        };
    }

    ItemDriverOutcome::ZombieShrine {
        item_id: item.id,
        character_id: character.id,
        shrine_type,
    }
}

fn xmastree_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::XmasTree {
        item_id: item.id,
        character_id: character.id,
    }
}

fn special_shrine_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::SpecialShrine {
        item_id: item.id,
        character_id: character.id,
        kind: drdata(item, 0),
    }
}

fn demonshrine_driver(character: &Character, item: &Item, area_id: u16) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    if character.level < u32::from(item.min_level) {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::DemonShrine {
        item_id: item.id,
        character_id: character.id,
        location_id: u32::from(item.x) + (u32::from(item.y) << 8) + (u32::from(area_id) << 16),
    }
}

const PALACE_KEY_COMBINATIONS: &[(i32, i32, i32)] = &[
    (51015, 51016, 51021),
    (51015, 51017, 51027),
    (51015, 51022, 51023),
    (51015, 51024, 51026),
    (51015, 51025, 51027),
    (51015, 51029, 51032),
    (51015, 51030, 51033),
    (51015, 51034, 51031),
    (51015, 51036, 51038),
    (51015, 51039, 51014),
    (51015, 51040, 51037),
    (51016, 51018, 51022),
    (51016, 51025, 51024),
    (51016, 51027, 51041),
    (51016, 51028, 51026),
    (51016, 51030, 51034),
    (51016, 51032, 51042),
    (51016, 51033, 51031),
    (51016, 51037, 51014),
    (51016, 51038, 51043),
    (51016, 51040, 51039),
    (51017, 51018, 51025),
    (51017, 51019, 51029),
    (51017, 51021, 51041),
    (51017, 51022, 51024),
    (51017, 51023, 51026),
    (51017, 51035, 51036),
    (51017, 51022, 51024),
    (51018, 51021, 51023),
    (51018, 51027, 51028),
    (51018, 51029, 51030),
    (51018, 51032, 51033),
    (51018, 51036, 51040),
    (51018, 51038, 51037),
    (51018, 51041, 51026),
    (51018, 51042, 51031),
    (51018, 51043, 51014),
    (51019, 51020, 51035),
    (51019, 51024, 51034),
    (51019, 51025, 51030),
    (51019, 51026, 51031),
    (51019, 51027, 51032),
    (51019, 51028, 51033),
    (51019, 51041, 51042),
    (51020, 51029, 51036),
    (51020, 51030, 51040),
    (51020, 51031, 51014),
    (51020, 51032, 51038),
    (51020, 51033, 51037),
    (51020, 51034, 51039),
    (51021, 51025, 51026),
    (51021, 51030, 51031),
    (51021, 51036, 51043),
    (51021, 51040, 51014),
    (51022, 51027, 51026),
    (51022, 51029, 51034),
    (51022, 51032, 51031),
    (51022, 51036, 51039),
    (51022, 51038, 51014),
    (51023, 51029, 51031),
    (51023, 51036, 51014),
    (51024, 51035, 51039),
    (51025, 51035, 51040),
    (51026, 51035, 51014),
    (51027, 51035, 51038),
    (51028, 51035, 51037),
    (51035, 51041, 51037),
];

fn palace_key_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        if let Some(&(part1, part2, _)) = PALACE_KEY_COMBINATIONS
            .iter()
            .find(|(_, _, result)| *result == item.sprite)
        {
            return ItemDriverOutcome::PalaceKeySplit {
                item_id: item.id,
                character_id: character.id,
                cursor_part_sprite: part1,
                carried_part_sprite: part2,
            };
        }
        return ItemDriverOutcome::PalaceKeyNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    };

    if context.cursor_template_id != Some(IID_AREA11_PALACEKEYPART) {
        return ItemDriverOutcome::PalaceKeyDoesNotFit {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let cursor_sprite = context.cursor_sprite.unwrap_or_default();
    let Some(&(_, _, result_sprite)) =
        PALACE_KEY_COMBINATIONS.iter().find(|&&(part1, part2, _)| {
            (item.sprite == part1 && cursor_sprite == part2)
                || (cursor_sprite == part1 && item.sprite == part2)
        })
    else {
        return ItemDriverOutcome::PalaceKeyDoesNotFit {
            item_id: item.id,
            character_id: character.id,
        };
    };

    ItemDriverOutcome::PalaceKeyCombine {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        result_sprite,
        final_key: result_sprite == 51014,
    }
}

fn double_door_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if item.x == 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::DoubleDoorToggle {
        item_id: item.id,
        character_id: character.id,
    }
}

fn chest_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.cursor_item.is_some() {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::ChestTreasure {
        item_id: item.id,
        character_id: character.id,
        treasure_index: drdata(item, 0),
    }
}

fn randchest_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    ItemDriverOutcome::RandomChest {
        item_id: item.id,
        character_id: character.id,
    }
}

fn forest_spade_driver(character: &Character, item: &Item, area_id: u16) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }
    if character.cursor_item.is_some() {
        return ItemDriverOutcome::ForestSpadeCursorOccupied {
            item_id: item.id,
            character_id: character.id,
        };
    }

    match (area_id, character.x, character.y) {
        (16, 205, 234) => ItemDriverOutcome::ForestSpadeFind {
            item_id: item.id,
            character_id: character.id,
            find: ForestSpadeFind::ForestNote1,
        },
        (16, 130, 219) => ItemDriverOutcome::ForestSpadeCollapse {
            item_id: item.id,
            character_id: character.id,
            x: 44,
            y: 231,
        },
        (1, 93, 36) => ItemDriverOutcome::ForestSpadeCollapse {
            item_id: item.id,
            character_id: character.id,
            x: 106,
            y: 211,
        },
        (29, 83, 127) => forest_spade_treasure(item.id, character.id, 0),
        (29, 94, 222) => forest_spade_treasure(item.id, character.id, 1),
        (29, 214, 136) => forest_spade_treasure(item.id, character.id, 2),
        (29, 185, 22) => forest_spade_treasure(item.id, character.id, 3),
        (29, 165, 79) => forest_spade_treasure(item.id, character.id, 4),
        _ => ItemDriverOutcome::ForestSpadeNothing {
            item_id: item.id,
            character_id: character.id,
        },
    }
}

fn forest_spade_treasure(
    item_id: ItemId,
    character_id: CharacterId,
    dig_index: u8,
) -> ItemDriverOutcome {
    ItemDriverOutcome::ForestSpadeFind {
        item_id,
        character_id,
        find: ForestSpadeFind::BranningtonTreasure { dig_index },
    }
}

fn pick_chest_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    if character.cursor_item.is_some() {
        return ItemDriverOutcome::PickChestCursorOccupied {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if !context.has_area17_lockpick {
        return ItemDriverOutcome::PickChestLocked {
            item_id: item.id,
            character_id: character.id,
        };
    }
    let Some(template) = PickChestTemplate::from_kind(drdata(item, 0)) else {
        return ItemDriverOutcome::PickChestBug {
            item_id: item.id,
            character_id: character.id,
        };
    };

    ItemDriverOutcome::PickChest {
        item_id: item.id,
        character_id: character.id,
        template,
    }
}

fn pick_door_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call {
        if drdata(item, 0) == 0 {
            return ItemDriverOutcome::Noop;
        }
        return ItemDriverOutcome::PickDoorToggle {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if drdata(item, 0) != 0 {
        return ItemDriverOutcome::Noop;
    }
    if character.flags.contains(CharacterFlags::PLAYER) && !context.has_area17_lockpick {
        return ItemDriverOutcome::PickDoorLocked {
            item_id: item.id,
            character_id: character.id,
        };
    }
    ItemDriverOutcome::PickDoorToggle {
        item_id: item.id,
        character_id: character.id,
    }
}

fn pent_boss_door_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let offset_x = i32::from(character.x) - i32::from(item.x);
    let offset_y = i32::from(character.y) - i32::from(item.y);

    let access_ticks = context
        .pent_demon_lord_access_seconds
        .unwrap_or(120)
        .saturating_mul(TICKS_PER_SECOND as u32);
    let recently_solved = context
        .pent_last_solve_tick
        .is_some_and(|last| context.current_tick.saturating_sub(last) <= access_ticks);
    if !recently_solved && (offset_x > 0 || offset_y > 0) {
        return ItemDriverOutcome::PentBossDoorLocked {
            item_id: item.id,
            character_id: character.id,
        };
    }

    if offset_x != 0 && offset_y != 0 {
        return ItemDriverOutcome::Noop;
    }

    let target_x = i32::from(item.x) - offset_x;
    let target_y = i32::from(item.y) - offset_y;
    if target_x < 1
        || target_x > MAX_MAP as i32 - 2
        || target_y < 1
        || target_y > MAX_MAP as i32 - 2
    {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::PentBossDoor {
        item_id: item.id,
        character_id: character.id,
        x: target_x as u16,
        y: target_y as u16,
    }
}

fn burndown_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    let burn_state = drdata(item, 0);
    if context.timer_call || character.id.0 == 0 {
        if burn_state == 0 {
            return ItemDriverOutcome::Noop;
        }
        return ItemDriverOutcome::BurndownTimerTick { item_id: item.id };
    }

    if burn_state > 15 {
        return ItemDriverOutcome::BurndownTooHot {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if burn_state != 0 {
        return ItemDriverOutcome::BurndownAlreadyBurned {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if context.cursor_driver != Some(IDR_TORCH) || context.cursor_drdata0.unwrap_or_default() == 0 {
        return ItemDriverOutcome::BurndownTouch {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::BurndownIgnite {
        item_id: item.id,
        character_id: character.id,
    }
}

fn colortile_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 || !character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::ColorTile {
        item_id: item.id,
        character_id: character.id,
        row: drdata(item, 0),
        color: drdata(item, 1),
    }
}

fn skelraise_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call || character.id.0 == 0 {
        return ItemDriverOutcome::SkelRaiseTimer { item_id: item.id };
    }

    if drdata(item, 2) != 0 {
        return ItemDriverOutcome::SkelRaiseTouch {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::SkelRaiseDust {
            item_id: item.id,
            character_id: character.id,
        };
    };
    if context.cursor_template_id != Some(IID_AREA17_BLOODBOWL) {
        return ItemDriverOutcome::SkelRaiseDust {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let template = match drdata(item, 0) {
        0 => "raised_skeleton_green",
        1 => "raised_skeleton_red",
        2 => "raised_skeleton_green_key",
        3 => "raised_skeleton_red_key",
        4 => "raised_skeleton_nolight",
        5 => "quest_skeleton",
        _ => {
            return ItemDriverOutcome::SkelRaiseDust {
                item_id: item.id,
                character_id: character.id,
            }
        }
    };

    ItemDriverOutcome::SkelRaiseRaise {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        template,
    }
}

fn infinite_chest_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    if character.cursor_item.is_some() {
        return ItemDriverOutcome::InfiniteChestCursorOccupied {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let required_key_id = drdata_u32(item, 1);
    let key_name = if required_key_id == 0 {
        None
    } else {
        match context
            .door_key
            .as_ref()
            .filter(|key| key.key_id == required_key_id)
        {
            Some(key) => Some(outcome_item_name(&key.name)),
            None => {
                return ItemDriverOutcome::InfiniteChestKeyRequired {
                    item_id: item.id,
                    character_id: character.id,
                };
            }
        }
    };

    let Some(template) = InfiniteChestTemplate::from_kind(drdata(item, 0)) else {
        return ItemDriverOutcome::InfiniteChestUnknown {
            item_id: item.id,
            character_id: character.id,
        };
    };

    ItemDriverOutcome::InfiniteChest {
        item_id: item.id,
        character_id: character.id,
        template,
        key_name,
    }
}

fn keyring_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    match character.cursor_item {
        Some(key_item_id) => ItemDriverOutcome::KeyringAddCursorItem {
            item_id: item.id,
            character_id: character.id,
            key_item_id,
        },
        None => ItemDriverOutcome::KeyringShow {
            item_id: item.id,
            character_id: character.id,
        },
    }
}

fn enchant_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::EnchantNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    };

    ItemDriverOutcome::EnchantCursorItem {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        modifier: i16::from(drdata(item, 0)),
        amount: i16::from(drdata(item, 1)),
    }
}

fn anti_enchant_driver(character: &Character, item: &Item, extract_orb: bool) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::EnchantNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    };

    ItemDriverOutcome::AntiEnchantCursorItem {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        modifier: i16::from(drdata(item, 0)),
        amount: i16::from(drdata(item, 1)),
        extract_orb,
    }
}

fn shrike_amulet_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }
    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::ShrikeAmuletNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    };
    let own_bits = drdata(item, 0);
    let cursor_bits = context.cursor_drdata0.unwrap_or(0);
    if context.cursor_driver != Some(IDR_SHRIKEAMULET) || (own_bits & cursor_bits) != 0 {
        return ItemDriverOutcome::ShrikeAmuletDoesNotFit {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::ShrikeAmuletAssemble {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        combined_bits: own_bits | cursor_bits,
    }
}

fn mine_gateway_key_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::MineGatewayKeyNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    };
    if context.cursor_driver != Some(IDR_MINEGATEWAYKEY) {
        return ItemDriverOutcome::MineGatewayKeyDoesNotFit {
            item_id: item.id,
            character_id: character.id,
        };
    }
    let combined_bits = drdata(item, 0) | context.cursor_drdata0.unwrap_or(0);
    ItemDriverOutcome::MineGatewayKeyAssemble {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        combined_bits,
    }
}

const DEV_ID_DB: u32 = 0x01;
const DEV_ID_WARR: u32 = 0x06;

const fn make_item_id(dev_id: u32, nr: u32) -> u32 {
    (dev_id << 24) | nr
}

const IID_AREA2_SUN1: u32 = make_item_id(DEV_ID_DB, 0x00003A);
const IID_AREA8_REDCRYSTAL: u32 = make_item_id(DEV_ID_DB, 0x00004A);
const IID_AREA8_BLOOD: u32 = make_item_id(DEV_ID_DB, 0x00004B);
const IID_AREA2_SUN2: u32 = make_item_id(DEV_ID_DB, 0x00003B);
const IID_AREA2_SUN3: u32 = make_item_id(DEV_ID_DB, 0x00003C);
const IID_AREA2_SUN12: u32 = make_item_id(DEV_ID_DB, 0x00003D);
const IID_AREA2_SUN13: u32 = make_item_id(DEV_ID_DB, 0x00003E);
const IID_AREA2_SUN23: u32 = make_item_id(DEV_ID_DB, 0x00003F);

const IID_STAFF_BLUEKEY1: u32 = make_item_id(DEV_ID_WARR, 0x00000A);
const IID_STAFF_BLUEKEY2: u32 = make_item_id(DEV_ID_WARR, 0x00000B);
const IID_STAFF_BLUEKEY3: u32 = make_item_id(DEV_ID_WARR, 0x00000C);
const IID_STAFF_BLUEKEY12: u32 = make_item_id(DEV_ID_WARR, 0x00000D);
const IID_STAFF_BLUEKEY13: u32 = make_item_id(DEV_ID_WARR, 0x00000E);
const IID_STAFF_BLUEKEY23: u32 = make_item_id(DEV_ID_WARR, 0x00000F);

const IID_STAFF_GREENKEY1: u32 = make_item_id(DEV_ID_WARR, 0x000011);
const IID_STAFF_GREENKEY2: u32 = make_item_id(DEV_ID_WARR, 0x000012);
const IID_STAFF_GREENKEY3: u32 = make_item_id(DEV_ID_WARR, 0x000013);
const IID_STAFF_GREENKEY12: u32 = make_item_id(DEV_ID_WARR, 0x000014);
const IID_STAFF_GREENKEY13: u32 = make_item_id(DEV_ID_WARR, 0x000015);
const IID_STAFF_GREENKEY23: u32 = make_item_id(DEV_ID_WARR, 0x000016);

const IID_STAFF_REDKEY1: u32 = make_item_id(DEV_ID_WARR, 0x000018);
const IID_STAFF_REDKEY2: u32 = make_item_id(DEV_ID_WARR, 0x000019);
const IID_STAFF_REDKEY3: u32 = make_item_id(DEV_ID_WARR, 0x00001A);
const IID_STAFF_REDKEY12: u32 = make_item_id(DEV_ID_WARR, 0x00001B);
const IID_STAFF_REDKEY13: u32 = make_item_id(DEV_ID_WARR, 0x00001C);
const IID_STAFF_REDKEY23: u32 = make_item_id(DEV_ID_WARR, 0x00001D);

fn assemble_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }
    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::AssembleNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    };

    if !is_assemblable_primary(item.template_id) {
        return ItemDriverOutcome::AssembleUnknownItem {
            item_id: item.id,
            character_id: character.id,
        };
    };
    let Some(template) = assemble_template(item.template_id, context.cursor_template_id) else {
        return ItemDriverOutcome::AssembleDoesNotFit {
            item_id: item.id,
            character_id: character.id,
        };
    };

    ItemDriverOutcome::AssembleItem {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        template,
    }
}

fn is_assemblable_primary(primary_id: u32) -> bool {
    matches!(
        primary_id,
        IID_AREA2_SUN1
            | IID_AREA2_SUN2
            | IID_AREA2_SUN3
            | IID_AREA2_SUN12
            | IID_AREA2_SUN13
            | IID_AREA2_SUN23
            | IID_STAFF_BLUEKEY1
            | IID_STAFF_BLUEKEY2
            | IID_STAFF_BLUEKEY3
            | IID_STAFF_BLUEKEY12
            | IID_STAFF_BLUEKEY13
            | IID_STAFF_BLUEKEY23
            | IID_STAFF_GREENKEY1
            | IID_STAFF_GREENKEY2
            | IID_STAFF_GREENKEY3
            | IID_STAFF_GREENKEY12
            | IID_STAFF_GREENKEY13
            | IID_STAFF_GREENKEY23
            | IID_STAFF_REDKEY1
            | IID_STAFF_REDKEY2
            | IID_STAFF_REDKEY3
            | IID_STAFF_REDKEY12
            | IID_STAFF_REDKEY13
            | IID_STAFF_REDKEY23
    )
}

pub fn assemble_template(primary_id: u32, cursor_id: Option<u32>) -> Option<AssembleTemplate> {
    let cursor_id = cursor_id?;
    match primary_id {
        IID_AREA2_SUN1 => match cursor_id {
            IID_AREA2_SUN2 => Some(AssembleTemplate::SunAmulet12),
            IID_AREA2_SUN3 => Some(AssembleTemplate::SunAmulet13),
            IID_AREA2_SUN23 => Some(AssembleTemplate::SunAmulet123),
            _ => None,
        },
        IID_AREA2_SUN2 => match cursor_id {
            IID_AREA2_SUN1 => Some(AssembleTemplate::SunAmulet12),
            IID_AREA2_SUN3 => Some(AssembleTemplate::SunAmulet23),
            IID_AREA2_SUN13 => Some(AssembleTemplate::SunAmulet123),
            _ => None,
        },
        IID_AREA2_SUN3 => match cursor_id {
            IID_AREA2_SUN1 => Some(AssembleTemplate::SunAmulet13),
            IID_AREA2_SUN2 => Some(AssembleTemplate::SunAmulet23),
            IID_AREA2_SUN12 => Some(AssembleTemplate::SunAmulet123),
            _ => None,
        },
        IID_AREA2_SUN12 => (cursor_id == IID_AREA2_SUN3).then_some(AssembleTemplate::SunAmulet123),
        IID_AREA2_SUN13 => (cursor_id == IID_AREA2_SUN2).then_some(AssembleTemplate::SunAmulet123),
        IID_AREA2_SUN23 => (cursor_id == IID_AREA2_SUN1).then_some(AssembleTemplate::SunAmulet123),

        IID_STAFF_BLUEKEY1 => match cursor_id {
            IID_STAFF_BLUEKEY2 => Some(AssembleTemplate::WarrBluekey12),
            IID_STAFF_BLUEKEY3 => Some(AssembleTemplate::WarrBluekey13),
            IID_STAFF_BLUEKEY23 => Some(AssembleTemplate::WarrBluekey123),
            _ => None,
        },
        IID_STAFF_BLUEKEY2 => match cursor_id {
            IID_STAFF_BLUEKEY1 => Some(AssembleTemplate::WarrBluekey12),
            IID_STAFF_BLUEKEY3 => Some(AssembleTemplate::WarrBluekey23),
            IID_STAFF_BLUEKEY13 => Some(AssembleTemplate::WarrBluekey123),
            _ => None,
        },
        IID_STAFF_BLUEKEY3 => match cursor_id {
            IID_STAFF_BLUEKEY1 => Some(AssembleTemplate::WarrBluekey13),
            IID_STAFF_BLUEKEY2 => Some(AssembleTemplate::WarrBluekey23),
            IID_STAFF_BLUEKEY12 => Some(AssembleTemplate::WarrBluekey123),
            _ => None,
        },
        IID_STAFF_BLUEKEY12 => {
            (cursor_id == IID_STAFF_BLUEKEY3).then_some(AssembleTemplate::WarrBluekey123)
        }
        IID_STAFF_BLUEKEY13 => {
            (cursor_id == IID_STAFF_BLUEKEY2).then_some(AssembleTemplate::WarrBluekey123)
        }
        IID_STAFF_BLUEKEY23 => {
            (cursor_id == IID_STAFF_BLUEKEY1).then_some(AssembleTemplate::WarrBluekey123)
        }

        IID_STAFF_GREENKEY1 => match cursor_id {
            IID_STAFF_GREENKEY2 => Some(AssembleTemplate::WarrGreenkey12),
            IID_STAFF_GREENKEY3 => Some(AssembleTemplate::WarrGreenkey13),
            IID_STAFF_GREENKEY23 => Some(AssembleTemplate::WarrGreenkey123),
            _ => None,
        },
        IID_STAFF_GREENKEY2 => match cursor_id {
            IID_STAFF_GREENKEY1 => Some(AssembleTemplate::WarrGreenkey12),
            IID_STAFF_GREENKEY3 => Some(AssembleTemplate::WarrGreenkey23),
            IID_STAFF_GREENKEY13 => Some(AssembleTemplate::WarrGreenkey123),
            _ => None,
        },
        IID_STAFF_GREENKEY3 => match cursor_id {
            IID_STAFF_GREENKEY1 => Some(AssembleTemplate::WarrGreenkey13),
            IID_STAFF_GREENKEY2 => Some(AssembleTemplate::WarrGreenkey23),
            IID_STAFF_GREENKEY12 => Some(AssembleTemplate::WarrGreenkey123),
            _ => None,
        },
        IID_STAFF_GREENKEY12 => {
            (cursor_id == IID_STAFF_GREENKEY3).then_some(AssembleTemplate::WarrGreenkey123)
        }
        IID_STAFF_GREENKEY13 => {
            (cursor_id == IID_STAFF_GREENKEY2).then_some(AssembleTemplate::WarrGreenkey123)
        }
        IID_STAFF_GREENKEY23 => {
            (cursor_id == IID_STAFF_GREENKEY1).then_some(AssembleTemplate::WarrGreenkey123)
        }

        IID_STAFF_REDKEY1 => match cursor_id {
            IID_STAFF_REDKEY2 => Some(AssembleTemplate::WarrRedkey12),
            IID_STAFF_REDKEY3 => Some(AssembleTemplate::WarrRedkey13),
            IID_STAFF_REDKEY23 => Some(AssembleTemplate::WarrRedkey123),
            _ => None,
        },
        IID_STAFF_REDKEY2 => match cursor_id {
            IID_STAFF_REDKEY1 => Some(AssembleTemplate::WarrRedkey12),
            IID_STAFF_REDKEY3 => Some(AssembleTemplate::WarrRedkey23),
            IID_STAFF_REDKEY13 => Some(AssembleTemplate::WarrRedkey123),
            _ => None,
        },
        IID_STAFF_REDKEY3 => match cursor_id {
            IID_STAFF_REDKEY1 => Some(AssembleTemplate::WarrRedkey13),
            IID_STAFF_REDKEY2 => Some(AssembleTemplate::WarrRedkey23),
            IID_STAFF_REDKEY12 => Some(AssembleTemplate::WarrRedkey123),
            _ => None,
        },
        IID_STAFF_REDKEY12 => {
            (cursor_id == IID_STAFF_REDKEY3).then_some(AssembleTemplate::WarrRedkey123)
        }
        IID_STAFF_REDKEY13 => {
            (cursor_id == IID_STAFF_REDKEY2).then_some(AssembleTemplate::WarrRedkey123)
        }
        IID_STAFF_REDKEY23 => {
            (cursor_id == IID_STAFF_REDKEY1).then_some(AssembleTemplate::WarrRedkey123)
        }
        _ => None,
    }
}

fn stat_scroll_driver(character: &mut Character, item: &mut Item) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }
    if character.flags.contains(CharacterFlags::NOEXP) {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let value = usize::from(drdata(item, 0));
    let requested = drdata(item, 1);
    if requested == 0 || value >= CHARACTER_VALUE_COUNT || bare_value(character, value) <= 0 {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let mut raised = 0_u8;
    let mut exp_cost = 0_u32;
    for _ in 0..requested {
        let Some(cost) = raise_value_exp(character, value) else {
            break;
        };
        raised = raised.saturating_add(1);
        exp_cost = exp_cost.saturating_add(cost);
    }

    if raised == 0 {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }

    consume_item(character, item);
    ItemDriverOutcome::StatScrollUsed {
        item_id: item.id,
        character_id: character.id,
        value: value as u8,
        raised,
        exp_cost,
    }
}

fn raise_value_exp(character: &mut Character, value: usize) -> Option<u32> {
    if value >= CHARACTER_VALUE_COUNT || skill_raise_cost_factor(value) == 0 {
        return None;
    }
    let current = bare_value(character, value);
    if current <= 0 || current >= skillmax(character) {
        return None;
    }
    if value == CharacterValue::Profession as usize && current > 99 {
        return None;
    }

    let seyan = character.flags.contains(CharacterFlags::WARRIOR)
        && character.flags.contains(CharacterFlags::MAGE);
    let cost = raise_cost(value, current, seyan);
    character.exp_used = character.exp_used.saturating_add(cost);
    character.exp = character.exp.saturating_add(cost);
    character.values[1][value] = character.values[1][value].saturating_add(1);
    if character.values[0][value] < character.values[1][value] {
        character.values[0][value] = character.values[1][value];
    }
    Some(cost)
}

fn lower_value(character: &mut Character, value: usize) -> Option<u32> {
    if character.flags.contains(CharacterFlags::NOEXP)
        || value >= CHARACTER_VALUE_COUNT
        || skill_raise_cost_factor(value) == 0
    {
        return None;
    }
    let current = bare_value(character, value);
    if i32::from(current) <= skill_start(value) {
        return None;
    }

    let seyan = character.flags.contains(CharacterFlags::WARRIOR)
        && character.flags.contains(CharacterFlags::MAGE);
    let lowered = current.saturating_sub(1);
    character.values[1][value] = lowered;
    let cost = raise_cost(value, lowered, seyan);
    character.exp_used = character.exp_used.saturating_sub(cost);
    character.flags.insert(CharacterFlags::UPDATE);
    Some(cost)
}

fn bare_value(character: &Character, value: usize) -> i16 {
    character
        .values
        .get(1)
        .and_then(|values| values.get(value))
        .copied()
        .unwrap_or_default()
}

fn skillmax(character: &Character) -> i16 {
    if !character.flags.contains(CharacterFlags::ARCH) {
        return 50;
    }
    if character.flags.contains(CharacterFlags::WARRIOR)
        && character.flags.contains(CharacterFlags::MAGE)
    {
        110
    } else {
        125
    }
}

fn raise_cost(value: usize, current: i16, seyan: bool) -> u32 {
    let nr = i32::from(current) - skill_start(value) + 1 + 5;
    let cost = nr * nr * nr * i32::from(skill_raise_cost_factor(value));
    let cost = if seyan { cost * 4 / 30 } else { cost / 10 };
    cost.max(1) as u32
}

fn skill_start(value: usize) -> i32 {
    match value {
        0..=6 => 10,
        42 => -1,
        11..=41 => 1,
        _ => -1,
    }
}

fn skill_raise_cost_factor(value: usize) -> i16 {
    match value {
        0..=2 | 42 => 3,
        3..=6 => 2,
        11..=37 | 39 | 40 => 1,
        _ => 0,
    }
}

fn teleport_door_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if item.x == 0 || item.y == 0 {
        return ItemDriverOutcome::Noop;
    }

    let dx = i32::from(character.x) - i32::from(item.x);
    let dy = i32::from(character.y) - i32::from(item.y);
    if (dx != 0 && dy != 0) || (dx == 0 && dy == 0) {
        return ItemDriverOutcome::Noop;
    }

    match drdata(item, 0) {
        1 if dx == 1 => return ItemDriverOutcome::Noop,
        2 if dx == -1 => return ItemDriverOutcome::Noop,
        3 if dy == 1 => return ItemDriverOutcome::Noop,
        4 if dy == -1 => return ItemDriverOutcome::Noop,
        _ => {}
    }

    let max_level = drdata(item, 1);
    if max_level != 0 && character.level > u32::from(max_level) {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let target_x = i32::from(item.x) - dx;
    let target_y = i32::from(item.y) - dy;
    if target_x < 1
        || target_y < 1
        || target_x > i32::from(u16::MAX)
        || target_y > i32::from(u16::MAX)
    {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::TeleportDoor {
        item_id: item.id,
        character_id: character.id,
        x: target_x as u16,
        y: target_y as u16,
    }
}

fn door_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if item.x == 0 {
        return ItemDriverOutcome::Noop;
    }

    if context.timer_call {
        item.driver_data.resize(40, 0);
        if item.driver_data[39] != 0 {
            item.driver_data[39] -= 1;
        }
        if drdata(item, 0) == 0 || item.driver_data[39] != 0 || drdata(item, 5) != 0 {
            return ItemDriverOutcome::Noop;
        }
    }

    let required_key_id = door_required_key_id(item);
    if !context.timer_call && required_key_id != 0 {
        if let Some(key) = context
            .door_key
            .as_ref()
            .filter(|key| key.key_id == required_key_id || key.key_id == IID_SKELETON_KEY)
        {
            return ItemDriverOutcome::KeyedDoorToggle {
                item_id: item.id,
                character_id: character.id,
                key_id: key.key_id,
                source: key.source,
                locking: drdata(item, 0) != 0,
            };
        }
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::DoorToggle {
        item_id: item.id,
        character_id: character.id,
    }
}

fn door_required_key_id(item: &Item) -> u32 {
    u32::from(drdata(item, 1))
        | (u32::from(drdata(item, 2)) << 8)
        | (u32::from(drdata(item, 3)) << 16)
        | (u32::from(drdata(item, 4)) << 24)
}

fn recall_driver(
    character: &Character,
    item: &Item,
    area_id: u16,
    in_arena: bool,
) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }
    if character.action == action::DIE {
        return ItemDriverOutcome::Noop;
    }
    if character.level > u32::from(drdata(item, 0)) {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if area_id == 34 && in_arena {
        return ItemDriverOutcome::BlockedByArea {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::Recall {
        item_id: item.id,
        character_id: character.id,
        x: character.rest_x,
        y: character.rest_y,
        area_id: character.rest_area,
    }
}

fn city_recall_driver(
    character: &Character,
    item: &Item,
    area_id: u16,
    in_arena: bool,
) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }
    if character.action == action::DIE {
        return ItemDriverOutcome::Noop;
    }
    if area_id == 34 && in_arena {
        return ItemDriverOutcome::BlockedByArea {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let Some((x, y, area_id)) = city_recall_destination(drdata(item, 0)) else {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    };

    ItemDriverOutcome::CityRecall {
        item_id: item.id,
        character_id: character.id,
        x,
        y,
        area_id,
    }
}

fn city_recall_destination(scroll_type: u8) -> Option<(u16, u16, u16)> {
    Some(match scroll_type {
        0 => (126, 179, 1),
        1 => (167, 188, 3),
        2 => (229, 94, 3),
        3 => (236, 176, 3),
        4 => (41, 250, 14),
        5 => (231, 242, 12),
        6 => (67, 108, 17),
        7 => (203, 227, 29),
        8 => (226, 164, 29),
        9 => (27, 14, 37),
        10 => (120, 120, 36),
        11 => (210, 247, 31),
        12 => (224, 248, 34),
        _ => return None,
    })
}

fn dungeon_teleport_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if !character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::DungeonTeleport {
        item_id: item.id,
        character_id: character.id,
        x: drdata_u16(item, 0),
        y: drdata_u16(item, 2),
        clan_number: drdata_u16(item, 4),
    }
}

fn dungeon_fake_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    ItemDriverOutcome::DungeonFake {
        item_id: item.id,
        character_id: character.id,
        clan_number: drdata_u16(item, 0),
    }
}

fn dungeon_key_driver(character: &Character, item: &mut Item) -> ItemDriverOutcome {
    if character.cursor_item.is_some() {
        return ItemDriverOutcome::DungeonKeyCursorOccupied {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let first_taken = drdata(item, 2) == 0;
    if first_taken {
        set_drdata(item, 2, 1);
    }

    ItemDriverOutcome::DungeonKey {
        item_id: item.id,
        character_id: character.id,
        template: if drdata(item, 0) == 1 {
            "maze_key1"
        } else {
            "maze_key2"
        },
        key_id: drdata_u32(item, 4),
        clan_number: drdata(item, 1),
        first_taken,
    }
}

fn dungeon_door_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let key1 = drdata_u32(item, 0);
    let key2 = drdata_u32(item, 4);
    let mut missing = 0;
    if key1 != 0 && !context.has_dungeon_door_key1 {
        missing += 1;
    }
    if key2 != 0 && !context.has_dungeon_door_key2 {
        missing += 1;
    }
    if missing != 0 {
        return ItemDriverOutcome::DungeonDoorMissingKeys {
            item_id: item.id,
            character_id: character.id,
            missing,
            both_required: key1 != 0 && key2 != 0,
        };
    }

    let alive = context.dungeon_defender_count.unwrap_or(0);
    if alive > 20 {
        return ItemDriverOutcome::DungeonDoorTooManyDefenders {
            item_id: item.id,
            character_id: character.id,
            alive,
            max_allowed: 20,
        };
    }

    let catacomb = (((u32::from(item.x).saturating_sub(2)) / 81)
        + ((u32::from(item.y).saturating_sub(2)) / 81) * 3) as u8;
    let first_solve = drdata(item, 12) == 0;
    if first_solve {
        set_drdata_u32(item, 0, 0);
        set_drdata_u32(item, 4, 0);
        set_drdata(item, 12, 1);
    }

    ItemDriverOutcome::DungeonDoorSolved {
        item_id: item.id,
        character_id: character.id,
        clan_number: drdata_u32(item, 8),
        catacomb,
        first_solve,
    }
}

fn teleport_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    let target_x = drdata_u16(item, 0);
    let target_y = drdata_u16(item, 2);
    let target_area = drdata_u16(item, 4);
    let arch_only = drdata(item, 10) != 0;
    let brannington_arch_gate = drdata(item, 11) != 0;
    let stop_driver = drdata(item, 12) != 0;
    let quiet = drdata(item, 6) != 0;

    if brannington_arch_gate || (arch_only && !character.flags.contains(CharacterFlags::ARCH)) {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if item.max_level != 0 && character.level > u32::from(item.max_level) {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if character.level < u32::from(item.min_level) {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if target_x < 1 || target_y < 1 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::Teleport {
        item_id: item.id,
        character_id: character.id,
        x: target_x,
        y: target_y,
        area_id: target_area,
        stop_driver,
        quiet,
    }
}

fn drdata_u16(item: &Item, idx: usize) -> u16 {
    let lo = u16::from(drdata(item, idx));
    let hi = u16::from(drdata(item, idx + 1));
    lo | (hi << 8)
}

fn drdata_u32(item: &Item, idx: usize) -> u32 {
    u32::from_le_bytes([
        drdata(item, idx),
        drdata(item, idx + 1),
        drdata(item, idx + 2),
        drdata(item, idx + 3),
    ])
}

fn set_drdata_u16(item: &mut Item, idx: usize, value: u16) {
    set_drdata(item, idx, value as u8);
    set_drdata(item, idx + 1, (value >> 8) as u8);
}

fn set_drdata_u32(item: &mut Item, idx: usize, value: u32) {
    for (offset, byte) in value.to_le_bytes().into_iter().enumerate() {
        set_drdata(item, idx + offset, byte);
    }
}

fn labexit_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call && character.id.0 == 0 {
        let frame = drdata_u32(item, 8);
        if frame < 24 {
            item.sprite = 1060 + (frame % 24) as i32;
        } else if frame < 240 {
            item.sprite = 1060 + (frame % 24) as i32 + 24;
        } else if frame < 240 + 24 {
            item.sprite = 1060 + (frame % 24) as i32 + 48;
        } else {
            return ItemDriverOutcome::LabExitExpired { item_id: item.id };
        }

        let next_frame = frame.saturating_add(1);
        set_drdata_u32(item, 8, next_frame);
        return ItemDriverOutcome::LabExitAnimating {
            item_id: item.id,
            sprite: item.sprite,
            frame: next_frame,
            schedule_after_ticks: 2,
        };
    }

    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let owner_id = drdata_u32(item, 0);
    if character.id.0 != owner_id {
        return ItemDriverOutcome::LabExitWrongOwner {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let frame = drdata_u32(item, 8);
    let close_frame = 240 - 24 + (frame % 24);
    set_drdata_u32(item, 8, close_frame);

    ItemDriverOutcome::LabExitUse {
        item_id: item.id,
        character_id: character.id,
        lab_nr: drdata(item, 4),
        frame: close_frame,
        target_area: 3,
        target_x: 183,
        target_y: 199,
    }
}

fn toylight_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call {
        return ItemDriverOutcome::Noop;
    }

    item.driver_data.resize(2, 0);
    if item.driver_data[0] != 0 {
        item.driver_data[0] = 0;
        item.modifier_value[0] = 0;
        item.sprite -= 1;
    } else {
        let light = i16::from(item.driver_data[1]);
        item.driver_data[0] = 1;
        item.modifier_index[0] = V_LIGHT;
        item.modifier_value[0] = light;
        item.sprite += 1;
    }

    ItemDriverOutcome::LightChanged {
        item_id: item.id,
        character_id: character.id,
        schedule_after_ticks: None,
    }
}

fn nightlight_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if !context.timer_call {
        return ItemDriverOutcome::Noop;
    }

    item.driver_data.resize(2, 0);
    let was_on = item.driver_data[0] != 0;
    if was_on && context.daylight > 80 {
        item.driver_data[0] = 0;
        item.modifier_value[0] = 0;
        item.sprite -= 1;
    } else if !was_on && context.daylight < 80 {
        let light = i16::from(item.driver_data[1]);
        item.driver_data[0] = 1;
        item.modifier_index[0] = V_LIGHT;
        item.modifier_value[0] = light;
        item.sprite += 1;
    }

    ItemDriverOutcome::LightChanged {
        item_id: item.id,
        character_id: character.id,
        schedule_after_ticks: Some(LIGHT_TIMER_TICKS),
    }
}

fn onofflight_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    item.driver_data.resize(7, 0);

    if context.timer_call && character.id.0 == 0 {
        if item.driver_data[0] == 0 {
            return ItemDriverOutcome::Noop;
        }
        if item.driver_data[6] == 0 {
            item.driver_data[6] = 1;
            return ItemDriverOutcome::Noop;
        }
    }

    let now_on = if item.driver_data[0] != 0 {
        item.driver_data[0] = 0;
        item.modifier_value[0] = 0;
        item.sprite -= 1;
        false
    } else {
        let light = i16::from(item.driver_data[1]);
        item.driver_data[0] = 1;
        item.modifier_index[0] = V_LIGHT;
        item.modifier_value[0] = light;
        item.sprite += 1;
        true
    };

    ItemDriverOutcome::OnOffLightChanged {
        item_id: item.id,
        character_id: character.id,
        now_on,
        remaining_off: None,
        gates_opened: false,
    }
}

const EDEMON_SWITCH_COOLDOWN_TICKS: u64 = TICKS_PER_SECOND * 60 * 5;

fn edemon_switch_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    item.driver_data.resize(5, 0);
    let fire = item.driver_data[0] != 0;
    let pause_until = u32::from_le_bytes([
        item.driver_data[1],
        item.driver_data[2],
        item.driver_data[3],
        item.driver_data[4],
    ]);

    if context.timer_call || character.id.0 == 0 {
        if fire || context.current_tick <= pause_until {
            return ItemDriverOutcome::Noop;
        }
        item.driver_data[0] = 1;
        item.sprite -= 1;
        item.modifier_index[0] = V_LIGHT;
        item.modifier_value[0] = 64;
        return ItemDriverOutcome::LightChanged {
            item_id: item.id,
            character_id: character.id,
            schedule_after_ticks: None,
        };
    }

    if !fire {
        return ItemDriverOutcome::EdemonSwitchStuck {
            item_id: item.id,
            character_id: character.id,
        };
    }

    item.driver_data[0] = 0;
    let pause_until = context
        .current_tick
        .wrapping_add(EDEMON_SWITCH_COOLDOWN_TICKS as u32);
    item.driver_data[1..5].copy_from_slice(&pause_until.to_le_bytes());
    item.sprite += 1;
    item.modifier_value[0] = 0;

    ItemDriverOutcome::LightChanged {
        item_id: item.id,
        character_id: character.id,
        schedule_after_ticks: Some(EDEMON_SWITCH_COOLDOWN_TICKS + 1),
    }
}

fn edemon_light_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 != 0 || !context.timer_call {
        return ItemDriverOutcome::Noop;
    }

    let power = context.edemon_section_power.unwrap_or_default();
    let (light, sprite) = if power != 0 && power < 249 {
        (200, 14191)
    } else {
        (0, 14189)
    };

    item.modifier_index[0] = V_LIGHT;
    item.modifier_value[0] = light;
    item.sprite = sprite;

    ItemDriverOutcome::LightChanged {
        item_id: item.id,
        character_id: character.id,
        schedule_after_ticks: Some(TICKS_PER_SECOND),
    }
}

fn edemon_loader_driver(
    character: &mut Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    item.driver_data.resize(3, 0);

    let mut power = item.driver_data[1];
    let mut animation = item.driver_data[2];
    let mut consumed_cursor_item_id = None;
    let mut sound_type = None;

    if context.timer_call || character.id.0 == 0 {
        power = power.saturating_sub((power != 0) as u8);
        animation = animation.saturating_sub((animation != 0) as u8);
    } else {
        if power != 0 {
            return ItemDriverOutcome::EdemonLoaderBlocked {
                item_id: item.id,
                character_id: character.id,
                reason: if character.cursor_item.is_some() {
                    EdemonLoaderBlockReason::CrystalAlreadyPresent
                } else {
                    EdemonLoaderBlockReason::CrystalStuck
                },
            };
        }
        let Some(cursor_item_id) = character.cursor_item else {
            return ItemDriverOutcome::EdemonLoaderBlocked {
                item_id: item.id,
                character_id: character.id,
                reason: EdemonLoaderBlockReason::NeedsCrystal,
            };
        };
        if context.cursor_template_id != Some(IID_AREA6_YELLOWCRYSTAL) {
            return ItemDriverOutcome::EdemonLoaderBlocked {
                item_id: item.id,
                character_id: character.id,
                reason: EdemonLoaderBlockReason::WrongCrystal,
            };
        }

        power = context.cursor_drdata0.unwrap_or_default();
        animation = 7;
        character.cursor_item = None;
        character.flags.insert(CharacterFlags::ITEMS);
        consumed_cursor_item_id = Some(cursor_item_id);
        sound_type = Some(41);
    }

    item.driver_data[1] = power;
    item.driver_data[2] = animation;

    let overlay = if animation != 0 {
        14247u32.saturating_sub(u32::from(animation))
    } else if power != 0 {
        14248
    } else {
        14240
    };

    let old_sprite = item.sprite;
    item.sprite = if power != 0 {
        14262 - (i32::from(power) / 43)
    } else {
        14234
    };
    if old_sprite != 14234 && item.sprite == 14234 {
        sound_type = Some(43);
    }

    ItemDriverOutcome::EdemonLoaderChanged {
        item_id: item.id,
        character_id: character.id,
        consumed_cursor_item_id,
        ground_overlay_sprite: overlay,
        sound_type,
        schedule_after_ticks: (context.timer_call || character.id.0 == 0)
            .then_some(TICKS_PER_SECOND),
    }
}

fn fdemon_light_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 != 0 || !context.timer_call {
        return ItemDriverOutcome::Noop;
    }

    let Some(power) = context.fdemon_loader_power else {
        return ItemDriverOutcome::Noop;
    };
    let (light, sprite) = if power != 0 { (200, 14192) } else { (0, 14189) };

    item.modifier_index[0] = V_LIGHT;
    item.modifier_value[0] = light;
    item.sprite = sprite;

    ItemDriverOutcome::LightChanged {
        item_id: item.id,
        character_id: character.id,
        schedule_after_ticks: Some(TICKS_PER_SECOND),
    }
}

fn fdemon_loader_driver(
    character: &mut Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    item.driver_data.resize(7, 0);

    let mut power = drdata_u16(item, 1);
    let mut animation = item.driver_data[3];
    let mut next_power = drdata_u16(item, 4);
    let mut consumed_cursor_item_id = None;
    let mut sound_type = None;

    if context.timer_call || character.id.0 == 0 {
        if animation != 0 {
            animation = animation.saturating_sub(1);
            if animation == 0 {
                power = next_power;
            }
        }
        if power != 0 {
            power = power.saturating_sub(1);
        }
    } else {
        if power != 0 || animation != 0 {
            if character.flags.contains(CharacterFlags::FDEMON) {
                power = 0;
                animation = 0;
                next_power = 0;
            } else if character.cursor_item.is_some() {
                return ItemDriverOutcome::FdemonLoaderBlocked {
                    item_id: item.id,
                    character_id: character.id,
                    reason: FdemonLoaderBlockReason::CrystalAlreadyPresent,
                };
            } else {
                return ItemDriverOutcome::FdemonLoaderBlocked {
                    item_id: item.id,
                    character_id: character.id,
                    reason: FdemonLoaderBlockReason::CrystalStuck,
                };
            }
        } else {
            let Some(cursor_item_id) = character.cursor_item else {
                return ItemDriverOutcome::FdemonLoaderBlocked {
                    item_id: item.id,
                    character_id: character.id,
                    reason: FdemonLoaderBlockReason::NeedsCrystal,
                };
            };
            if context.cursor_template_id != Some(IID_AREA8_REDCRYSTAL) {
                return ItemDriverOutcome::FdemonLoaderBlocked {
                    item_id: item.id,
                    character_id: character.id,
                    reason: FdemonLoaderBlockReason::WrongCrystal,
                };
            }

            next_power = u16::from(context.cursor_drdata0.unwrap_or_default()).saturating_mul(100);
            animation = 7;
            character.cursor_item = None;
            character.flags.insert(CharacterFlags::ITEMS);
            consumed_cursor_item_id = Some(cursor_item_id);
            sound_type = Some(41);
        }
    }

    if animation == 0 {
        next_power = power;
    }
    set_drdata_u16(item, 4, next_power);
    item.driver_data[3] = animation;
    set_drdata_u16(item, 1, power);

    let overlay = if animation != 0 {
        59028u32.saturating_sub(u32::from(animation))
    } else if next_power != 0 {
        59029
    } else {
        59021
    };

    let old_sprite = item.sprite;
    item.sprite = if next_power != 0 {
        59030 + 9 - (i32::from(next_power.min(2880)) / 320)
    } else {
        14234
    };
    if old_sprite != 14234 && item.sprite == 14234 {
        sound_type = Some(43);
    }

    ItemDriverOutcome::FdemonLoaderChanged {
        item_id: item.id,
        character_id: character.id,
        consumed_cursor_item_id,
        ground_overlay_sprite: overlay,
        sound_type,
        schedule_after_ticks: (context.timer_call || character.id.0 == 0)
            .then_some(TICKS_PER_SECOND),
    }
}

fn fdemon_farm_driver(
    character: &mut Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    item.driver_data.resize(3, 0);

    let step = item.driver_data[0];
    let size = item.driver_data[1];
    let mut strength = item.driver_data[2];

    let ready_template = if strength < size {
        strength = strength.wrapping_add(step);
        None
    } else {
        Some(FdemonCrystalTemplate::from_farm_size(size))
    };

    if !context.timer_call && character.id.0 != 0 {
        if character.cursor_item.is_some() {
            return ItemDriverOutcome::FdemonFarmCursorOccupied {
                item_id: item.id,
                character_id: character.id,
            };
        }
        let Some(template) = ready_template else {
            item.driver_data[2] = strength;
            return ItemDriverOutcome::FdemonFarmNotReady {
                item_id: item.id,
                character_id: character.id,
                current: strength,
                required: size,
            };
        };

        strength = 0;
        item.driver_data[2] = strength;
        return ItemDriverOutcome::FdemonFarmHarvest {
            item_id: item.id,
            character_id: character.id,
            template,
            foreground_sprite: 0,
        };
    }

    item.driver_data[2] = strength;
    ItemDriverOutcome::FdemonFarmChanged {
        item_id: item.id,
        character_id: character.id,
        foreground_sprite: ready_template.map_or(0, FdemonCrystalTemplate::foreground_sprite),
        schedule_after_ticks: Some(TICKS_PER_SECOND * 2),
    }
}

fn fdemon_blood_driver(
    character: &mut Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 || context.timer_call {
        return ItemDriverOutcome::Noop;
    }

    let Some(cursor_item_id) = character.cursor_item else {
        return ItemDriverOutcome::FdemonBloodBlocked {
            item_id: item.id,
            character_id: character.id,
            reason: FdemonBloodBlockReason::BareHands,
        };
    };

    if context.cursor_driver == Some(IDR_FLASK) {
        character.cursor_item = None;
        character.flags.insert(CharacterFlags::ITEMS);
        item.sprite = 14348;
        return ItemDriverOutcome::FdemonBloodDestroyedFlask {
            item_id: item.id,
            character_id: character.id,
            flask_item_id: cursor_item_id,
        };
    }

    if context.cursor_template_id != Some(IID_AREA8_BLOOD) {
        return ItemDriverOutcome::FdemonBloodBlocked {
            item_id: item.id,
            character_id: character.id,
            reason: FdemonBloodBlockReason::WrongItem,
        };
    }

    let amount = context.cursor_drdata0.unwrap_or_default();
    if amount > 2 {
        return ItemDriverOutcome::FdemonBloodBlocked {
            item_id: item.id,
            character_id: character.id,
            reason: FdemonBloodBlockReason::ContainerFull,
        };
    }

    let amount = amount.saturating_add(1);
    character.flags.insert(CharacterFlags::ITEMS);
    ItemDriverOutcome::FdemonBloodFilled {
        item_id: item.id,
        character_id: character.id,
        container_item_id: cursor_item_id,
        amount,
    }
}

fn torch_driver(
    character: &mut Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    item.driver_data.resize(4, 0);

    if context.timer_call {
        mark_special_modified_torch(item);
        if item.driver_data[0] == 0 {
            return ItemDriverOutcome::Noop;
        }
        if context.character_underwater {
            extinguish_torch(item);
            character.flags.insert(CharacterFlags::ITEMS);
            return ItemDriverOutcome::TorchExtinguishedUnderwater {
                item_id: item.id,
                character_id: character.id,
                schedule_after_ticks: LIGHT_TIMER_TICKS,
            };
        }

        item.driver_data[1] = item.driver_data[1].saturating_add(1);
        if item.driver_data[1] > item.driver_data[2] {
            return ItemDriverOutcome::TorchExpired {
                item_id: item.id,
                character_id: character.id,
                item_name: outcome_item_name(&item.name),
            };
        }
        set_torch_light(item);
        character.flags.insert(CharacterFlags::ITEMS);
        return ItemDriverOutcome::LightChanged {
            item_id: item.id,
            character_id: character.id,
            schedule_after_ticks: Some(LIGHT_TIMER_TICKS),
        };
    }

    if item.x != 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    if let Some((modifier_slot, modifier)) = torch_extractable_modifier(item) {
        return ItemDriverOutcome::TorchExtractOrb {
            item_id: item.id,
            character_id: character.id,
            modifier_slot,
            modifier,
        };
    }

    if item.driver_data[0] != 0 {
        extinguish_torch(item);
    } else {
        if context.character_underwater {
            return ItemDriverOutcome::BlockedByRequirements {
                item_id: item.id,
                character_id: character.id,
            };
        }
        item.driver_data[0] = 1;
        set_torch_light(item);
        item.sprite -= 1;
        item.flags.insert(ItemFlags::NODECAY);
    }
    character.flags.insert(CharacterFlags::ITEMS);

    ItemDriverOutcome::LightChanged {
        item_id: item.id,
        character_id: character.id,
        schedule_after_ticks: (item.driver_data[0] != 0).then_some(LIGHT_TIMER_TICKS),
    }
}

fn mark_special_modified_torch(item: &mut Item) {
    if item.min_level == 200 {
        return;
    }
    if item
        .modifier_index
        .iter()
        .zip(item.modifier_value.iter())
        .any(|(&index, &value)| index != V_LIGHT && index >= 0 && value > 0)
    {
        item.min_level = 200;
    }
}

fn torch_extractable_modifier(item: &Item) -> Option<(usize, i16)> {
    item.modifier_index
        .iter()
        .zip(item.modifier_value.iter())
        .enumerate()
        .find_map(|(slot, (&index, &value))| {
            (index != V_LIGHT && index >= 0 && value > 0).then_some((slot, index))
        })
}

fn extinguish_torch(item: &mut Item) {
    item.driver_data[0] = 0;
    item.modifier_value[0] = 0;
    item.sprite += 1;
    item.flags.remove(ItemFlags::NODECAY);
}

fn set_torch_light(item: &mut Item) {
    let burn = i32::from(item.driver_data[1]);
    let max_burn = i32::from(item.driver_data[2]);
    let base = i32::from(item.driver_data[3]);
    let light = base.min(base * max_burn / (burn + 1) / 2);
    item.modifier_index[0] = V_LIGHT;
    item.modifier_value[0] = light as i16;
}

pub fn outcome_item_name(name: &str) -> [u8; OUTCOME_ITEM_NAME_BYTES] {
    let mut bytes = [0; OUTCOME_ITEM_NAME_BYTES];
    let source = name.as_bytes();
    let len = source.len().min(OUTCOME_ITEM_NAME_BYTES);
    bytes[..len].copy_from_slice(&source[..len]);
    bytes
}

fn palace_gate_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if !context.timer_call || character.id.0 != 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::PalaceGateTick {
        item_id: item.id,
        opened: false,
        closed: false,
        blocked: false,
    }
}

fn food_driver(character: &mut Character, item: &mut Item) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    let kind = drdata(item, 0);
    if kind == 2 {
        return lollipop_driver(character, item);
    }
    if kind == 3 {
        return ItemDriverOutcome::ChristmasPopInspected {
            item_id: item.id,
            character_id: character.id,
        };
    }

    consume_item(character, item);
    ItemDriverOutcome::FoodEaten {
        item_id: item.id,
        character_id: character.id,
        kind,
    }
}

fn lollipop_driver(character: &mut Character, item: &mut Item) -> ItemDriverOutcome {
    let licks = drdata(item, 1);
    if licks == 8 {
        return ItemDriverOutcome::LollipopMemories {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let next_licks = licks.saturating_add(1);
    set_drdata(item, 1, next_licks);
    item.sprite += 1;

    let exp_added = lollipop_exp(character.level);
    character.exp = character.exp.saturating_add(exp_added);

    if next_licks == 1 {
        item.description = "A sweet lollipop. Well, it's already used.".to_string();
    } else if next_licks == 8 {
        item.description = "A lollipop stick.".to_string();
    }

    ItemDriverOutcome::LollipopLicked {
        item_id: item.id,
        character_id: character.id,
        exp_added,
        lick_count: next_licks,
    }
}

fn lollipop_exp(level: u32) -> u32 {
    legacy_level_value(level).saturating_div(750).max(5)
}

fn legacy_level_value(level: u32) -> u32 {
    let level = u64::from(level);
    let next = level.saturating_add(1);
    next.saturating_pow(4)
        .saturating_sub(level.saturating_pow(4))
        .min(u64::from(u32::MAX)) as u32
}

fn special_potion_driver(
    character: &mut Character,
    item: &mut Item,
    area_id: u16,
    in_arena: bool,
    current_tick: u32,
) -> ItemDriverOutcome {
    if character.id.0 == 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }
    if item.min_level != 0 && character.level < u32::from(item.min_level) {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if item.max_level != 0 && character.level > u32::from(item.max_level) {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if area_id == 33 || (area_id == 34 && in_arena) {
        return ItemDriverOutcome::BlockedByArea {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let kind = drdata(item, 0);
    let max_hp = character
        .values
        .first()
        .and_then(|values| values.get(CharacterValue::Hp as usize))
        .copied()
        .unwrap_or(0)
        .max(0) as i32
        * POWERSCALE;
    let old_hp = character.hp;
    let old_mana = character.mana;
    let old_endurance = character.endurance;

    match kind {
        0..=4 => {
            consume_item(character, item);
            return ItemDriverOutcome::SpecialPotionAntidote {
                item_id: item.id,
                character_id: character.id,
                kind,
                poison_removed: false,
            };
        }
        5 => {
            if character.saves < 10 && !character.flags.contains(CharacterFlags::HARDCORE) {
                character.saves += 1;
                consume_item(character, item);
                return ItemDriverOutcome::SpecialPotionSecurity {
                    item_id: item.id,
                    character_id: character.id,
                    used: true,
                };
            }
            return ItemDriverOutcome::SpecialPotionSecurity {
                item_id: item.id,
                character_id: character.id,
                used: false,
            };
        }
        6 => {
            return ItemDriverOutcome::SpecialPotionInfravision {
                item_id: item.id,
                character_id: character.id,
                installed: false,
            };
        }
        7 => {
            if character.exp < character.exp_used {
                return ItemDriverOutcome::SpecialPotionProfessionReset {
                    item_id: item.id,
                    character_id: character.id,
                    used: false,
                    professions_reset: 0,
                    profession_points_lowered: 0,
                    exp_refunded: 0,
                };
            }

            let professions_reset = character
                .professions
                .iter()
                .fold(0_u16, |sum, &value| sum.saturating_add(value.max(0) as u16));
            if professions_reset == 0 {
                return ItemDriverOutcome::SpecialPotionProfessionReset {
                    item_id: item.id,
                    character_id: character.id,
                    used: false,
                    professions_reset: 0,
                    profession_points_lowered: 0,
                    exp_refunded: 0,
                };
            }

            for profession in &mut character.professions {
                *profession = 0;
            }
            let old_exp_used = character.exp_used;
            let mut profession_points_lowered = 0_u16;
            for _ in 0..(professions_reset / 3) {
                if lower_value(character, CharacterValue::Profession as usize).is_some() {
                    profession_points_lowered = profession_points_lowered.saturating_add(1);
                }
            }
            let exp_refunded = old_exp_used.saturating_sub(character.exp_used);
            character.exp = character.exp.saturating_sub(exp_refunded);
            character
                .flags
                .insert(CharacterFlags::PROF | CharacterFlags::UPDATE);
            consume_item(character, item);
            return ItemDriverOutcome::SpecialPotionProfessionReset {
                item_id: item.id,
                character_id: character.id,
                used: true,
                professions_reset,
                profession_points_lowered,
                exp_refunded,
            };
        }
        8 => {
            character.hp = (character.hp - 10 * POWERSCALE).max(1);
            character.endurance = (character.endurance - 10 * POWERSCALE).max(0);
            character.mana = (character.mana - 10 * POWERSCALE).max(0);
            character.regen_ticker = current_tick;
        }
        9 => {
            character.hp = (character.hp - 10 * POWERSCALE).max(1);
            character.regen_ticker = current_tick;
        }
        10 => {
            character.mana = (character.mana - 10 * POWERSCALE).max(0);
            character.regen_ticker = current_tick;
        }
        11 => {
            character.hp = (character.hp - 10 * POWERSCALE).max(1);
            character.endurance = (character.endurance - 10 * POWERSCALE).max(0);
            character.mana = (character.mana - 10 * POWERSCALE).max(0);
            character.regen_ticker = current_tick;
        }
        12 => {
            if area_id != 33 {
                character.hp = (character.hp + 3 * POWERSCALE).min(max_hp);
            }
        }
        13 => {
            if area_id != 33 {
                character.hp = (character.hp + 4 * POWERSCALE).min(max_hp);
            }
        }
        14 => {
            if area_id != 33 {
                character.hp = (character.hp + 5 * POWERSCALE).min(max_hp);
            }
        }
        15 => {
            character.endurance = (character.endurance - 10 * POWERSCALE).max(0);
            character.regen_ticker = current_tick;
        }
        _ => {
            return ItemDriverOutcome::SpecialPotionBug {
                item_id: item.id,
                character_id: character.id,
            };
        }
    }

    consume_item(character, item);
    character.flags.insert(CharacterFlags::UPDATE);
    ItemDriverOutcome::SpecialPotionDrunk {
        item_id: item.id,
        character_id: character.id,
        kind,
        hp_delta: character.hp - old_hp,
        mana_delta: character.mana - old_mana,
        endurance_delta: character.endurance - old_endurance,
    }
}

fn decaying_item_driver(
    character: &mut Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    item.driver_data.resize(7, 0);

    if context.timer_call {
        if item.driver_data[0] == 0 {
            return ItemDriverOutcome::Noop;
        }

        let age = drdata_u16(item, 3).saturating_add(1);
        set_drdata_u16(item, 3, age);
        if age > drdata_u16(item, 5) {
            return ItemDriverOutcome::DecayItemExpired {
                item_id: item.id,
                character_id: item.carried_by.unwrap_or(character.id),
                item_name: outcome_item_name(&item.name),
            };
        }

        return ItemDriverOutcome::DecayItemToggled {
            item_id: item.id,
            character_id: item.carried_by.unwrap_or(character.id),
            active: true,
            schedule_after_ticks: Some(TICKS_PER_SECOND * 2),
        };
    }

    if item.x != 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    let activating = item.driver_data[0] == 0;
    item.driver_data[0] = u8::from(activating);
    let target_value = i16::from(if activating {
        item.driver_data[2]
    } else {
        item.driver_data[1]
    });
    for value in &mut item.modifier_value {
        if *value != 0 {
            *value = target_value;
        }
    }
    if activating {
        item.sprite += 1;
    } else {
        item.sprite -= 1;
    }
    character.flags.insert(CharacterFlags::ITEMS);

    ItemDriverOutcome::DecayItemToggled {
        item_id: item.id,
        character_id: character.id,
        active: activating,
        schedule_after_ticks: activating.then_some(TICKS_PER_SECOND * 2),
    }
}

fn potion_driver(
    character: &mut Character,
    item: &mut Item,
    area_id: u16,
    in_arena: bool,
) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }
    if area_id == 33 || (area_id == 34 && in_arena) {
        return ItemDriverOutcome::BlockedByArea {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let empty_kind = drdata(item, 0);
    if empty_kind != 0 {
        return ItemDriverOutcome::EmptyPotionTemplateNeeded {
            item_id: item.id,
            character_id: character.id,
            empty_kind,
        };
    }

    let old_hp = character.hp;
    let old_mana = character.mana;
    let old_endurance = character.endurance;
    character.hp = capped_resource(
        character.hp,
        drdata(item, 1),
        max_value(character, CharacterValue::Hp),
    );
    character.mana = capped_resource(
        character.mana,
        drdata(item, 2),
        max_value(character, CharacterValue::Mana),
    );
    character.endurance = capped_resource(
        character.endurance,
        drdata(item, 3),
        max_value(character, CharacterValue::Endurance),
    );
    consume_item(character, item);

    ItemDriverOutcome::PotionDrunk {
        item_id: item.id,
        character_id: character.id,
        hp_added: character.hp - old_hp,
        mana_added: character.mana - old_mana,
        endurance_added: character.endurance - old_endurance,
    }
}

fn beyond_potion_driver(
    character: &Character,
    item: &Item,
    area_id: u16,
    in_arena: bool,
) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }
    if !check_item_requirements(character, item) {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if area_id == 34 && in_arena {
        return ItemDriverOutcome::BlockedByArea {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::BeyondPotion {
        item_id: item.id,
        character_id: character.id,
        duration_minutes: drdata(item, 0),
        modifier_index: item.modifier_index,
        modifier_value: item.modifier_value,
        beyond_max_mod: item.flags.contains(ItemFlags::BEYONDMAXMOD),
    }
}

fn check_item_requirements(character: &Character, item: &Item) -> bool {
    if character.level < u32::from(item.min_level) {
        return false;
    }
    if item.max_level != 0 && character.level > u32::from(item.max_level) {
        return false;
    }
    if item.needs_class & 1 != 0 && !character.flags.contains(CharacterFlags::WARRIOR) {
        return false;
    }
    if item.needs_class & 2 != 0 && !character.flags.contains(CharacterFlags::MAGE) {
        return false;
    }
    if item.needs_class & 4 != 0
        && !(character.flags.contains(CharacterFlags::WARRIOR)
            && character.flags.contains(CharacterFlags::MAGE))
    {
        return false;
    }
    if item.needs_class & 8 != 0 && !character.flags.contains(CharacterFlags::ARCH) {
        return false;
    }

    item.modifier_index
        .iter()
        .zip(item.modifier_value.iter())
        .all(|(&index, &required)| {
            if index >= 0 || required <= 0 {
                return true;
            }
            let value = (-index) as usize;
            character
                .values
                .get(1)
                .and_then(|values| values.get(value))
                .copied()
                .unwrap_or_default()
                >= required
        })
}

fn capped_resource(current: i32, added_units: u8, max_units: i32) -> i32 {
    (current + i32::from(added_units) * POWERSCALE).min(max_units * POWERSCALE)
}

fn max_value(character: &Character, value: CharacterValue) -> i32 {
    character
        .values
        .first()
        .and_then(|values| values.get(value as usize))
        .copied()
        .unwrap_or_default() as i32
}

fn drdata(item: &Item, idx: usize) -> u8 {
    item.driver_data.get(idx).copied().unwrap_or_default()
}

fn set_drdata(item: &mut Item, idx: usize, value: u8) {
    if item.driver_data.len() <= idx {
        item.driver_data.resize(idx + 1, 0);
    }
    item.driver_data[idx] = value;
}

fn clamp_legacy_coordinate(value: i32) -> u16 {
    value.clamp(0, i32::from(u16::MAX)) as u16
}

#[cfg(test)]
mod tests {
    use crate::{
        entity::{Character, Item, ItemFlags, SpeedMode, MAX_MODIFIERS},
        ids::{CharacterId, ItemId},
    };

    use super::*;

    #[test]
    fn item_driver_constants_cover_legacy_drvlib_surface() {
        assert_eq!(IDR_CLANSPAWN, 20);
        assert_eq!(IDR_CLANVAULT, 22);
        assert_eq!(IDR_CHESTSPAWN, 27);
        assert_eq!(IDR_PENT, 30);
        assert_eq!(IDR_EDEMONSWITCH, 37);
        assert_eq!(IDR_EDEMONTUBE, 43);
        assert_eq!(IDR_FDEMONLIGHT, 44);
        assert_eq!(IDR_FDEMONLAVA, 51);
        assert_eq!(IDR_ITEMSPAWN, 53);
        assert_eq!(IDR_WARMFIRE, 54);
        assert_eq!(IDR_FREAKDOOR, 58);
        assert_eq!(IDR_MINEWALL, 60);
        assert_eq!(IDR_TOPLIST, 63);
        assert_eq!(IDR_DUNGEONTELE, 65);
        assert_eq!(IDR_SWAMPSPAWN, 75);
        assert_eq!(IDR_PALACEDOOR, 76);
        assert_eq!(IDR_BONEHOLDER, 91);
        assert_eq!(IDR_LFREDUCT, 97);
        assert_eq!(IDR_LQ_KEY, 100);
        assert_eq!(IDR_STR_DEPOT, 108);
        assert_eq!(IDR_WARPKEYDOOR, 116);
        assert_eq!(IDR_STAFFER, 121);
        assert_eq!(IDR_MINEGATEWAY, 127);
        assert_eq!(IDR_TEUFELARENAEXIT, 141);
        assert_eq!(IDR_SALTMINE_ITEM, 188);
        assert_eq!(IDR_LAB5_ITEM, 190);
        assert_eq!(IDR_LABTORCH, 199);
        assert_eq!(IDR_SKELETON_KEY, 201);
    }

    #[test]
    fn dungeon_teleport_decodes_legacy_target_and_requires_player() {
        let mut actor = character(1);
        actor.flags |= CharacterFlags::PLAYER;
        let mut tele = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DUNGEONTELE);
        tele.driver_data = vec![44, 1, 55, 1, 13, 0];

        let outcome = execute_item_driver(
            &mut actor,
            &mut tele,
            ItemDriverRequest::Driver {
                driver: IDR_DUNGEONTELE,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            13,
            false,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::DungeonTeleport {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                x: 300,
                y: 311,
                clan_number: 13,
            }
        );

        actor.flags.remove(CharacterFlags::PLAYER);
        let outcome = execute_item_driver(
            &mut actor,
            &mut tele,
            ItemDriverRequest::Driver {
                driver: IDR_DUNGEONTELE,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            13,
            false,
        );
        assert_eq!(outcome, ItemDriverOutcome::Noop);
    }

    #[test]
    fn dungeon_fake_and_key_port_area13_dispatch_boundary() {
        let mut actor = character(1);
        let mut fake = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DUNGEONFAKE);
        fake.driver_data = vec![21, 0];

        let outcome = execute_item_driver(
            &mut actor,
            &mut fake,
            ItemDriverRequest::Driver {
                driver: IDR_DUNGEONFAKE,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            13,
            false,
        );
        assert_eq!(
            outcome,
            ItemDriverOutcome::DungeonFake {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                clan_number: 21,
            }
        );

        let mut key = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_DUNGEONKEY);
        key.driver_data = vec![1, 21, 0, 0, 0x44, 0x33, 0x22, 0x11];
        let outcome = execute_item_driver(
            &mut actor,
            &mut key,
            ItemDriverRequest::Driver {
                driver: IDR_DUNGEONKEY,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            13,
            false,
        );
        assert_eq!(
            outcome,
            ItemDriverOutcome::DungeonKey {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                template: "maze_key1",
                key_id: 0x1122_3344,
                clan_number: 21,
                first_taken: true,
            }
        );
        assert_eq!(key.driver_data[2], 1);

        actor.cursor_item = Some(ItemId(99));
        let blocked = execute_item_driver(
            &mut actor,
            &mut key,
            ItemDriverRequest::Driver {
                driver: IDR_DUNGEONKEY,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            13,
            false,
        );
        assert_eq!(
            blocked,
            ItemDriverOutcome::DungeonKeyCursorOccupied {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn dungeon_door_ports_key_and_defender_gates() {
        let mut actor = character(1);
        let mut door = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_DUNGEONDOOR);
        door.x = 164;
        door.y = 83;
        door.driver_data = vec![
            0x44, 0x33, 0x22, 0x11, 0x88, 0x77, 0x66, 0x55, 9, 0, 0, 0, 0,
        ];
        let request = ItemDriverRequest::Driver {
            driver: IDR_DUNGEONDOOR,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        let missing = execute_item_driver_with_context(
            &mut actor,
            &mut door,
            request,
            13,
            false,
            &ItemDriverContext::default(),
        );
        assert_eq!(
            missing,
            ItemDriverOutcome::DungeonDoorMissingKeys {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                missing: 2,
                both_required: true,
            }
        );

        let too_many = execute_item_driver_with_context(
            &mut actor,
            &mut door,
            request,
            13,
            false,
            &ItemDriverContext {
                has_dungeon_door_key1: true,
                has_dungeon_door_key2: true,
                dungeon_defender_count: Some(21),
                ..ItemDriverContext::default()
            },
        );
        assert_eq!(
            too_many,
            ItemDriverOutcome::DungeonDoorTooManyDefenders {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                alive: 21,
                max_allowed: 20,
            }
        );

        let solved = execute_item_driver_with_context(
            &mut actor,
            &mut door,
            request,
            13,
            false,
            &ItemDriverContext {
                has_dungeon_door_key1: true,
                has_dungeon_door_key2: true,
                dungeon_defender_count: Some(20),
                ..ItemDriverContext::default()
            },
        );
        assert_eq!(
            solved,
            ItemDriverOutcome::DungeonDoorSolved {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                clan_number: 9,
                catacomb: 5,
                first_solve: true,
            }
        );
        assert_eq!(drdata_u32(&door, 0), 0);
        assert_eq!(drdata_u32(&door, 4), 0);
        assert_eq!(door.driver_data[12], 1);
    }

    #[test]
    fn dungeon_driver_ids_are_area13_guarded_like_legacy_libload() {
        let mut actor = character(1);
        actor.flags |= CharacterFlags::PLAYER;
        let mut tele = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DUNGEONTELE);

        let outcome = execute_item_driver(
            &mut actor,
            &mut tele,
            ItemDriverRequest::Driver {
                driver: IDR_DUNGEONTELE,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::LibloadAreaBlocked {
                driver: IDR_DUNGEONTELE,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                required_area: 13,
            }
        );
    }

    #[test]
    fn use_item_opens_container_before_driver_dispatch() {
        let mut character = character(1);
        let mut item = item(7, ItemFlags::USED | ItemFlags::USE, 22, IDR_POTION);

        let outcome = use_item(&mut character, &item, request(1, 7, 0), false).unwrap();

        assert_eq!(
            outcome,
            UseItemOutcome::OpenContainer { item_id: ItemId(7) }
        );
        assert_eq!(character.current_container, Some(ItemId(7)));

        item.content_id = 0;
        let outcome = use_item(&mut character, &item, request(1, 7, 5), false).unwrap();
        assert_eq!(
            outcome,
            UseItemOutcome::Dispatch(ItemDriverRequest::Driver {
                driver: IDR_POTION,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 5,
            })
        );
    }

    #[test]
    fn use_item_opens_depot_and_account_depot_like_legacy_order() {
        let mut character = character(1);
        let depot = item(7, ItemFlags::USED | ItemFlags::USE | ItemFlags::DEPOT, 0, 0);
        let outcome = use_item(&mut character, &depot, request(1, 7, 0), false).unwrap();
        assert_eq!(outcome, UseItemOutcome::OpenDepot { item_id: ItemId(7) });

        let account_depot = item(
            8,
            ItemFlags::USED | ItemFlags::USE | ItemFlags::DEPOT,
            0,
            IDR_ACCOUNT_DEPOT,
        );
        assert_eq!(
            use_item(&mut character, &account_depot, request(1, 8, 0), false),
            Err(UseItemError::AccountDepotUnavailable)
        );
        assert_eq!(
            use_item(&mut character, &account_depot, request(1, 8, 0), true).unwrap(),
            UseItemOutcome::OpenAccountDepot { item_id: ItemId(8) }
        );
        assert_eq!(character.current_container, Some(ItemId(8)));
    }

    #[test]
    fn account_depot_driver_request_is_supported_for_non_use_paths() {
        let mut character = character(1);
        let mut depot = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_ACCOUNT_DEPOT);

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut depot,
                ItemDriverRequest::AccountDepot {
                    item_id: ItemId(8),
                    character_id: CharacterId(1),
                },
                1,
                false,
                &ItemDriverContext::default(),
            ),
            ItemDriverOutcome::AccountDepotOpened {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn identity_tag_item_drivers_are_handled_noops_like_legacy_libload() {
        let mut character = character(1);
        let mut tagged = item(8, ItemFlags::USED | ItemFlags::USE, 0, 1000);
        let request = ItemDriverRequest::Driver {
            driver: 1000,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        let outcome = execute_item_driver(&mut character, &mut tagged, request, 1, false);

        assert_eq!(
            outcome,
            ItemDriverOutcome::IdentityTag {
                driver: 1000,
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
        assert_eq!(legacy_item_driver_return_code(Some(1000), &outcome), 1);
    }

    #[test]
    fn clanjewel_driver_initializes_creation_time_and_reschedules_timer() {
        let mut timer_character = character(0);
        let mut jewel = item(8, ItemFlags::USED, 0, IDR_CLANJEWEL);

        let outcome = execute_item_driver_with_context(
            &mut timer_character,
            &mut jewel,
            ItemDriverRequest::Driver {
                driver: IDR_CLANJEWEL,
                item_id: ItemId(8),
                character_id: CharacterId(0),
                spec: 0,
            },
            30,
            false,
            &ItemDriverContext {
                timer_call: true,
                current_tick: 123 * TICKS_PER_SECOND as u32,
                ..ItemDriverContext::default()
            },
        );

        assert_eq!(drdata_u32(&jewel, 0), 123);
        assert_eq!(
            outcome,
            ItemDriverOutcome::ClanJewelRescheduled {
                item_id: ItemId(8),
                schedule_after_ticks: CLANJEWEL_CHECK_INTERVAL_TICKS,
            }
        );
    }

    #[test]
    fn clanjewel_driver_expires_after_one_hour_timer_lifetime() {
        let mut timer_character = character(0);
        let mut jewel = item(8, ItemFlags::USED, 0, IDR_CLANJEWEL);
        jewel.name = "Clan Jewel".into();
        jewel.carried_by = Some(CharacterId(42));
        set_drdata_u32(&mut jewel, 0, 100);

        let outcome = execute_item_driver_with_context(
            &mut timer_character,
            &mut jewel,
            ItemDriverRequest::Driver {
                driver: IDR_CLANJEWEL,
                item_id: ItemId(8),
                character_id: CharacterId(0),
                spec: 0,
            },
            30,
            false,
            &ItemDriverContext {
                timer_call: true,
                current_tick: (100 + CLANJEWEL_LIFETIME_SECONDS + 1) * TICKS_PER_SECOND as u32,
                ..ItemDriverContext::default()
            },
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::ClanJewelExpired {
                item_id: ItemId(8),
                character_id: Some(CharacterId(42)),
                item_name: outcome_item_name("Clan Jewel"),
            }
        );
    }

    #[test]
    fn clanjewel_driver_ignores_direct_character_use() {
        let mut character = character(1);
        let mut jewel = item(8, ItemFlags::USED, 0, IDR_CLANJEWEL);

        let outcome = execute_item_driver_with_context(
            &mut character,
            &mut jewel,
            ItemDriverRequest::Driver {
                driver: IDR_CLANJEWEL,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                spec: 0,
            },
            30,
            false,
            &ItemDriverContext::default(),
        );

        assert_eq!(outcome, ItemDriverOutcome::Noop);
        assert_eq!(drdata_u32(&jewel, 0), 0);
    }

    #[test]
    fn oxy_potion_driver_requires_area31_and_carried_item() {
        let mut character = character(1);
        let mut potion = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_OXYPOTION);
        let request = ItemDriverRequest::Driver {
            driver: IDR_OXYPOTION,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut potion, request, 30, false),
            ItemDriverOutcome::LibloadAreaBlocked {
                driver: IDR_OXYPOTION,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                required_area: 31,
            }
        );
        assert_eq!(
            execute_item_driver(&mut character, &mut potion, request, 31, false),
            ItemDriverOutcome::Noop
        );

        potion.carried_by = Some(CharacterId(1));
        character.inventory[30] = Some(ItemId(8));
        assert_eq!(
            execute_item_driver(&mut character, &mut potion, request, 31, false),
            ItemDriverOutcome::OxygenPotion {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                installed: false,
            }
        );
    }

    #[test]
    fn pick_berry_driver_ports_area31_boundary_and_location_id() {
        let mut actor = character(1);
        let mut berry = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_PICKBERRY);
        berry.x = 12;
        berry.y = 34;
        berry.driver_data = vec![3];
        let request = ItemDriverRequest::Driver {
            driver: IDR_PICKBERRY,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(IDR_PICKBERRY, 129);
        assert_eq!(
            execute_item_driver(&mut actor, &mut berry, request, 30, false),
            ItemDriverOutcome::BlockedByArea {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );

        actor.cursor_item = Some(ItemId(99));
        assert_eq!(
            execute_item_driver(&mut actor, &mut berry, request, 31, false),
            ItemDriverOutcome::PickBerryCursorOccupied {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );

        actor.cursor_item = None;
        assert_eq!(
            execute_item_driver(&mut actor, &mut berry, request, 31, false),
            ItemDriverOutcome::PickBerry {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                kind: 3,
                location_id: 12 + (34 << 8) + (31 << 16),
            }
        );

        let mut timer_character = character(0);
        assert_eq!(
            execute_item_driver(&mut timer_character, &mut berry, request, 31, false),
            ItemDriverOutcome::Noop
        );
    }

    #[test]
    fn parkshrine_driver_ports_area2_memorize_boundary() {
        let mut actor = character(1);
        let mut shrine = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_PARKSHRINE);
        shrine.driver_data = vec![2];
        let request = ItemDriverRequest::Driver {
            driver: IDR_PARKSHRINE,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(IDR_PARKSHRINE, 23);
        assert_eq!(
            execute_item_driver(&mut actor, &mut shrine, request, 2, false),
            ItemDriverOutcome::ParkShrine {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                shrine: 2,
            }
        );

        shrine.driver_data = vec![4];
        assert_eq!(
            execute_item_driver(&mut actor, &mut shrine, request, 2, false),
            ItemDriverOutcome::ParkShrineBug {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                shrine: 4,
            }
        );

        let mut timer_character = character(0);
        assert_eq!(
            execute_item_driver(&mut timer_character, &mut shrine, request, 2, false),
            ItemDriverOutcome::Noop
        );
    }

    #[test]
    fn caligar_training_driver_ports_watch_lesson_boundary() {
        let mut actor = character(1);
        actor.flags.insert(CharacterFlags::PLAYER);
        let mut training = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_CALIGAR);
        training.driver_data = vec![1, 2];
        let request = ItemDriverRequest::Driver {
            driver: IDR_CALIGAR,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(IDR_CALIGAR, 144);
        assert_eq!(
            execute_item_driver(&mut actor, &mut training, request, 36, false),
            ItemDriverOutcome::CaligarTraining {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                lesson: 2,
            }
        );

        training.driver_data = vec![1, 9];
        assert_eq!(
            execute_item_driver(&mut actor, &mut training, request, 36, false),
            ItemDriverOutcome::Noop
        );

        training.driver_data = vec![2, 1];
        assert_eq!(
            execute_item_driver(&mut actor, &mut training, request, 36, false),
            ItemDriverOutcome::CaligarWeightMove {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );

        training.driver_data = vec![3, 0];
        assert_eq!(
            execute_item_driver(&mut actor, &mut training, request, 36, false),
            ItemDriverOutcome::CaligarWeightDoor {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
        assert_eq!(
            legacy_item_driver_return_code(
                Some(IDR_CALIGAR),
                &ItemDriverOutcome::CaligarWeightDoorLocked {
                    item_id: ItemId(8),
                    character_id: CharacterId(1),
                },
            ),
            2
        );

        training.driver_data = vec![5, 0];
        assert_eq!(
            execute_item_driver(&mut actor, &mut training, request, 36, false),
            ItemDriverOutcome::CaligarGunProjectile {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                direction: 1,
                schedule_after_ticks: 12,
            }
        );

        training.driver_data = vec![9, 0];
        assert_eq!(
            execute_item_driver(&mut actor, &mut training, request, 36, false),
            ItemDriverOutcome::CaligarGunProjectile {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                direction: 5,
                schedule_after_ticks: 12,
            }
        );

        training.driver_data = vec![11, 0];
        assert_eq!(
            execute_item_driver(&mut actor, &mut training, request, 36, false),
            ItemDriverOutcome::Extinguish {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                extinguished: false,
            }
        );

        training.driver_data = vec![12, 3];
        assert_eq!(
            execute_item_driver(&mut actor, &mut training, request, 36, false),
            ItemDriverOutcome::CaligarSkellyDoor {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                door_index: 3,
            }
        );
        assert_eq!(
            legacy_item_driver_return_code(
                Some(IDR_CALIGAR),
                &ItemDriverOutcome::CaligarSkellyDoorLocked {
                    item_id: ItemId(8),
                    character_id: CharacterId(1),
                },
            ),
            2
        );

        let mut timer_character = character(0);
        assert_eq!(
            execute_item_driver(&mut timer_character, &mut training, request, 36, false),
            ItemDriverOutcome::Noop
        );

        training.driver_data = vec![2, 1];
        let timer_request = ItemDriverRequest::Driver {
            driver: IDR_CALIGAR,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        };
        assert_eq!(
            execute_item_driver(
                &mut timer_character,
                &mut training,
                timer_request,
                36,
                false
            ),
            ItemDriverOutcome::CaligarWeightTimer { item_id: ItemId(8) }
        );
    }

    #[test]
    fn caligar_key_assembly_ports_piece_matrix() {
        let mut actor = character(1);
        actor.cursor_item = Some(ItemId(9));
        let mut key = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_CALIGAR);
        key.driver_data = vec![10];
        key.sprite = 13414;
        let request = ItemDriverRequest::Driver {
            driver: IDR_CALIGAR,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };
        let mut context = ItemDriverContext {
            cursor_template_id: Some(IID_CALIGAR_PALACE_KEY_PART),
            cursor_sprite: Some(13415),
            ..ItemDriverContext::default()
        };

        assert_eq!(IID_CALIGAR_PALACE_KEY_PART, 0x0100_00B3);
        assert_eq!(
            execute_item_driver_with_context(&mut actor, &mut key, request, 36, false, &context),
            ItemDriverOutcome::CaligarKeyAssemble {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                cursor_item_id: ItemId(9),
                result_sprite: 13421,
                final_key: false,
            }
        );

        key.sprite = 13420;
        context.cursor_sprite = Some(13414);
        assert_eq!(
            execute_item_driver_with_context(&mut actor, &mut key, request, 36, false, &context),
            ItemDriverOutcome::CaligarKeyAssemble {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                cursor_item_id: ItemId(9),
                result_sprite: 0,
                final_key: true,
            }
        );

        context.cursor_template_id = Some(IID_AREA11_PALACEKEYPART);
        assert_eq!(
            execute_item_driver_with_context(&mut actor, &mut key, request, 36, false, &context),
            ItemDriverOutcome::CaligarKeyNeedsCursor {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );

        context.cursor_template_id = Some(IID_CALIGAR_PALACE_KEY_PART);
        context.cursor_sprite = Some(13416);
        assert_eq!(
            execute_item_driver_with_context(&mut actor, &mut key, request, 36, false, &context),
            ItemDriverOutcome::CaligarKeyDoesNotFit {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn alchemy_flower_driver_ports_location_and_cursor_gate() {
        let mut actor = character(1);
        let mut flower = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLOWER);
        flower.x = 20;
        flower.y = 40;
        flower.driver_data = vec![17];
        let request = ItemDriverRequest::Driver {
            driver: IDR_FLOWER,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(IDR_FLASK, 32);
        assert_eq!(IDR_FLOWER, 33);
        assert_eq!(
            execute_item_driver(&mut actor, &mut flower, request, 7, false),
            ItemDriverOutcome::PickAlchemyFlower {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                kind: 17,
                location_id: 20 + (40 << 8) + (7 << 16),
            }
        );

        actor.cursor_item = Some(ItemId(99));
        assert_eq!(
            execute_item_driver(&mut actor, &mut flower, request, 7, false),
            ItemDriverOutcome::PickAlchemyFlowerCursorOccupied {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );

        let mut timer_character = character(0);
        assert_eq!(
            execute_item_driver(&mut timer_character, &mut flower, request, 7, false),
            ItemDriverOutcome::Noop
        );
    }

    #[test]
    fn flask_driver_ports_ingredient_gates_and_add_outcome() {
        let mut actor = character(1);
        actor.cursor_item = Some(ItemId(9));
        let mut flask = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLASK);
        flask.carried_by = Some(CharacterId(1));
        flask.driver_data = vec![2, 1, 0];
        let request = ItemDriverRequest::Driver {
            driver: IDR_FLASK,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(IID_ALCHEMY_INGREDIENT, (1 << 24) | 0x43);
        assert_eq!(
            execute_item_driver_with_context(
                &mut actor,
                &mut flask,
                request,
                1,
                false,
                &ItemDriverContext::default(),
            ),
            ItemDriverOutcome::FlaskWrongCursor {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );

        let context = ItemDriverContext {
            cursor_template_id: Some(IID_ALCHEMY_INGREDIENT),
            cursor_drdata0: Some(7),
            ..ItemDriverContext::default()
        };
        assert_eq!(
            execute_item_driver_with_context(&mut actor, &mut flask, request, 1, false, &context),
            ItemDriverOutcome::FlaskIngredientAdded {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                cursor_item_id: ItemId(9),
                ingredient_kind: 7,
            }
        );

        flask.driver_data[1] = 6;
        assert_eq!(
            execute_item_driver_with_context(&mut actor, &mut flask, request, 1, false, &context),
            ItemDriverOutcome::FlaskFull {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn flask_driver_ports_empty_shake_teufelheim_arena_and_finished_blocks() {
        let mut actor = character(1);
        let mut flask = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLASK);
        flask.carried_by = Some(CharacterId(1));
        flask.driver_data = vec![1, 0, 0];
        let request = ItemDriverRequest::Driver {
            driver: IDR_FLASK,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut actor, &mut flask, request, 1, true),
            ItemDriverOutcome::FlaskEmptyShaken {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
        assert_eq!(
            execute_item_driver(&mut actor, &mut flask, request, 34, true),
            ItemDriverOutcome::BlockedByArea {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
        assert_eq!(
            execute_item_driver(&mut actor, &mut flask, request, 1, false),
            ItemDriverOutcome::FlaskEmptyShaken {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );

        actor.cursor_item = Some(ItemId(9));
        flask.driver_data[2] = 1;
        assert_eq!(
            execute_item_driver(&mut actor, &mut flask, request, 1, false),
            ItemDriverOutcome::FlaskFinishedNoMoreIngredients {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn flask_driver_ports_finished_potion_use_boundary() {
        let mut actor = character(1);
        actor.level = 10;
        actor.flags.insert(CharacterFlags::WARRIOR);
        let mut flask = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLASK);
        flask.carried_by = Some(CharacterId(1));
        flask.min_level = 10;
        flask.driver_data = vec![2, 3, 1, 20];
        flask.modifier_index = [CharacterValue::Strength as i16, 0, 0, 0, 0];
        flask.modifier_value = [7, 0, 0, 0, 0];
        let request = ItemDriverRequest::Driver {
            driver: IDR_FLASK,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut actor, &mut flask, request, 1, false),
            ItemDriverOutcome::AlchemyFlaskPotion {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                duration_minutes: 20,
                modifier_index: [CharacterValue::Strength as i16, 0, 0, 0, 0],
                modifier_value: [7, 0, 0, 0, 0],
            }
        );

        actor.level = 9;
        assert_eq!(
            execute_item_driver(&mut actor, &mut flask, request, 1, false),
            ItemDriverOutcome::BlockedByRequirements {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn flask_driver_ports_successful_shake_recipe_mix() {
        let mut actor = character(1);
        let mut flask = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLASK);
        flask.carried_by = Some(CharacterId(1));
        flask.driver_data = vec![
            2, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1,
        ];
        flask.driver_data[12] = 1;
        flask.driver_data[13] = 1;
        flask.driver_data[14] = 1;
        let request = ItemDriverRequest::Driver {
            driver: IDR_FLASK,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut actor, &mut flask, request, 1, false),
            ItemDriverOutcome::FlaskMixed {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                ingredient_counts: {
                    let mut counts = [0; 29];
                    counts[1] = 1;
                    counts[2] = 1;
                    counts[3] = 1;
                    counts[7] = 1;
                    counts[8] = 1;
                    counts[17] = 1;
                    counts
                },
            }
        );
        assert_eq!(flask.driver_data[2], 1);
        assert_eq!(flask.driver_data[3], 10);
        assert_eq!(flask.modifier_index[0], CharacterValue::Attack as i16);
        assert_eq!(flask.modifier_value[0], 3);
        assert_eq!(flask.value, 3 * 7 * 13 + 50);
        assert_eq!(flask.needs_class, 0);
        assert_eq!(flask.name, "Magical Potion");
        assert_eq!(flask.sprite, 50214);
        assert_eq!(flask.description, "A flask containing a magical liquid.");
    }

    #[test]
    fn flask_driver_ports_c_empty_modifier_slots_for_smaller_recipes() {
        let mut actor = character(1);
        let request = ItemDriverRequest::Driver {
            driver: IDR_FLASK,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        let mut double_recipe = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLASK);
        double_recipe.carried_by = Some(CharacterId(1));
        double_recipe.driver_data.resize(31, 0);
        double_recipe.driver_data[0] = 2;
        double_recipe.driver_data[1] = 4;
        double_recipe.driver_data[11] = 1;
        double_recipe.driver_data[12] = 1;
        double_recipe.driver_data[13] = 1;
        double_recipe.driver_data[14] = 1;
        double_recipe.driver_data[18] = 1;
        double_recipe.driver_data[28] = 1;

        assert!(matches!(
            execute_item_driver(&mut actor, &mut double_recipe, request, 1, false),
            ItemDriverOutcome::FlaskMixed { .. }
        ));
        assert_eq!(
            double_recipe.modifier_index[0..3],
            [
                CharacterValue::Attack as i16,
                CharacterValue::Parry as i16,
                -1,
            ]
        );
        assert_eq!(double_recipe.modifier_value[0..3], [1, 1, 0]);

        let mut single_recipe = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLASK);
        single_recipe.carried_by = Some(CharacterId(1));
        single_recipe.driver_data.resize(29, 0);
        single_recipe.driver_data[0] = 1;
        single_recipe.driver_data[1] = 3;
        single_recipe.driver_data[14] = 2;
        single_recipe.driver_data[17] = 1;
        single_recipe.driver_data[18] = 1;
        single_recipe.driver_data[28] = 1;

        assert!(matches!(
            execute_item_driver(&mut actor, &mut single_recipe, request, 1, false),
            ItemDriverOutcome::FlaskMixed { .. }
        ));
        assert_eq!(
            single_recipe.modifier_index[0..3],
            [CharacterValue::Pulse as i16, -1, -1]
        );
        assert_eq!(single_recipe.modifier_value[0..3], [2, 0, 0]);
    }

    #[test]
    fn flask_driver_ports_failed_shake_reset_to_empty_bottle() {
        let mut actor = character(1);
        let mut flask = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLASK);
        flask.carried_by = Some(CharacterId(1));
        flask.name = "Unfinished Potion".to_string();
        flask.description = "A flask containing some strange liquid.".to_string();
        flask.sprite = 50209;
        flask.value = 123;
        flask.needs_class = 8;
        flask.driver_data.resize(35, 0);
        flask.driver_data[0] = 2;
        flask.driver_data[1] = 1;
        flask.driver_data[11] = 1;
        flask.modifier_index[0] = CharacterValue::Wisdom as i16;
        flask.modifier_value[0] = 1;
        let request = ItemDriverRequest::Driver {
            driver: IDR_FLASK,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut actor, &mut flask, request, 1, false),
            ItemDriverOutcome::FlaskRuined {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                ingredient_counts: {
                    let mut counts = [0; 29];
                    counts[0] = 1;
                    counts
                },
            }
        );
        assert_eq!(flask.name, "Empty Potion");
        assert_eq!(flask.sprite, 10294);
        assert_eq!(flask.description, "A flask made of glass.");
        assert_eq!(flask.driver_data, vec![2]);
        assert_eq!(flask.modifier_index, [0; MAX_MODIFIERS]);
        assert_eq!(flask.modifier_value, [0; MAX_MODIFIERS]);
        assert_eq!(flask.value, 10);
        assert_eq!(flask.needs_class, 0);
    }

    #[test]
    fn flask_driver_ports_fallback_attribute_mix_and_stone_class() {
        let mut actor = character(1);
        actor.professions[P_ALCHEMIST] = 20;
        let mut flask = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLASK);
        flask.carried_by = Some(CharacterId(1));
        flask.driver_data.resize(35, 0);
        flask.driver_data[0] = 2;
        flask.driver_data[1] = 3;
        flask.driver_data[11] = 2;
        flask.driver_data[15] = 1;
        flask.driver_data[18] = 1;
        flask.driver_data[28] = 1;
        flask.driver_data[31] = 1;
        let request = ItemDriverRequest::Driver {
            driver: IDR_FLASK,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        execute_item_driver(&mut actor, &mut flask, request, 1, false);

        assert_eq!(flask.modifier_index[0], CharacterValue::Wisdom as i16);
        assert_eq!(flask.modifier_value[0], 2);
        assert_eq!(flask.modifier_index[1], CharacterValue::Hp as i16);
        assert_eq!(flask.modifier_value[1], 2);
        assert_eq!(flask.value, 15 * 13 + 50);
        assert_eq!(flask.needs_class, 8);
    }

    #[test]
    fn flask_power_uses_legacy_time_and_alchemist_thresholds() {
        let mut actor = character(1);
        actor.professions[P_ALCHEMIST] = 50;
        let mut flask = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLASK);
        flask.carried_by = Some(CharacterId(1));
        flask.driver_data.resize(29, 0);
        flask.driver_data[0] = 2;
        flask.driver_data[1] = 3;
        flask.driver_data[12] = 1;
        flask.driver_data[13] = 1;
        flask.driver_data[14] = 1;
        flask.driver_data[25] = 1;
        flask.driver_data[26] = 1;
        flask.driver_data[28] = 1;
        let context = ItemDriverContext {
            fullmoon: true,
            ..ItemDriverContext::default()
        };
        let request = ItemDriverRequest::Driver {
            driver: IDR_FLASK,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        execute_item_driver_with_context(&mut actor, &mut flask, request, 1, false, &context);

        assert_eq!(flask.modifier_value[0], 44);
        assert_eq!(flask.value, 3 * 88 * 13 + 50);
    }

    #[test]
    fn lizard_flower_mixer_requires_cursor_flower_and_combines_bits() {
        let mut actor = character(1);
        actor.cursor_item = Some(ItemId(9));
        let mut flower = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_LIZARDFLOWER);
        flower.carried_by = Some(CharacterId(1));
        flower.driver_data = vec![1];
        flower.sprite = 11190;
        let request = ItemDriverRequest::Driver {
            driver: IDR_LIZARDFLOWER,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut actor,
                &mut flower,
                request,
                30,
                false,
                &ItemDriverContext::default(),
            ),
            ItemDriverOutcome::LibloadAreaBlocked {
                driver: IDR_LIZARDFLOWER,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                required_area: 31,
            }
        );

        assert_eq!(
            execute_item_driver_with_context(
                &mut actor,
                &mut flower,
                request,
                31,
                false,
                &ItemDriverContext::default(),
            ),
            ItemDriverOutcome::LizardFlowerDoesNotFit {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );

        let context = ItemDriverContext {
            cursor_driver: Some(IDR_LIZARDFLOWER),
            cursor_sprite: Some(11191),
            cursor_drdata0: Some(6),
            ..ItemDriverContext::default()
        };
        assert_eq!(
            execute_item_driver_with_context(&mut actor, &mut flower, request, 31, false, &context,),
            ItemDriverOutcome::LizardFlowerMixed {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                cursor_item_id: ItemId(9),
                combined_bits: 7,
                complete: true,
                bottle_message: true,
            }
        );

        actor.cursor_item = None;
        assert_eq!(
            execute_item_driver_with_context(&mut actor, &mut flower, request, 31, false, &context,),
            ItemDriverOutcome::LizardFlowerNeedsCursor {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn lab3_berry_driver_decodes_yellow_white_and_brown() {
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(8));
        let request = ItemDriverRequest::Driver {
            driver: IDR_LAB3_PLANT,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        let mut yellow = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_LAB3_PLANT);
        yellow.carried_by = Some(CharacterId(1));
        yellow.driver_data = vec![5, 3, 4];
        assert_eq!(
            execute_item_driver(&mut character, &mut yellow, request, 22, false),
            ItemDriverOutcome::Lab3YellowBerry {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                duration_ticks: 45 * TICKS_PER_SECOND,
                installed: false,
            }
        );

        let mut brown = yellow.clone();
        brown.driver_data = vec![11];
        assert_eq!(
            execute_item_driver(&mut character, &mut brown, request, 22, false),
            ItemDriverOutcome::Lab3BrownBerry {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                duration_ticks: 10 * TICKS_PER_SECOND,
                installed: false,
            }
        );

        let mut white = yellow;
        white.driver_data = vec![6, 1, 2];
        assert_eq!(
            execute_item_driver(&mut character, &mut white, request, 22, false),
            ItemDriverOutcome::Lab3WhiteBerry {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                light_power: 40,
                started_emit: false,
                installed: false,
            }
        );

        let mut light = white.clone();
        light.driver_data = vec![10];
        let mut timer_character = character.clone();
        timer_character.id = CharacterId(0);
        let timer_request = ItemDriverRequest::Driver {
            driver: IDR_LAB3_PLANT,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        };
        assert_eq!(
            execute_item_driver_with_context(
                &mut timer_character,
                &mut light,
                timer_request,
                22,
                false,
                &ItemDriverContext {
                    timer_call: true,
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::Lab3WhiteBerryLightTick {
                item_id: ItemId(8),
                destroyed: false,
            }
        );
    }

    #[test]
    fn book_driver_returns_legacy_text_kind() {
        let mut character = character(1);
        let mut book = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BOOK);
        set_drdata(&mut book, 0, 8);
        let request = ItemDriverRequest::Driver {
            driver: IDR_BOOK,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(IDR_BOOK, 16);
        assert_eq!(
            execute_item_driver(&mut character, &mut book, request, 2, false),
            ItemDriverOutcome::BookText {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                kind: 8,
                demon_value: 0,
            }
        );
        assert_eq!(
            book_text_lines(8)[0],
            "There are two kinds of vampires. One is known under varying names, such as 'Vampire', 'Lesser Vampire', 'Dracul' or 'Necrifah'."
        );
    }

    #[test]
    fn libload_area_guards_block_outside_legacy_area() {
        let mut character = character(1);
        let mut bridge = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BONEBRIDGE);
        let request = ItemDriverRequest::Driver {
            driver: IDR_BONEBRIDGE,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(IDR_BONEBRIDGE, 89);
        assert_eq!(IDR_BONEHINT, 94);
        assert_eq!(IDR_NOMADDICE, 95);
        assert_eq!(IDR_STAFFER2, 122);
        assert_eq!(IDR_CALIGAR, 144);
        assert_eq!(IDR_CALIGARFLAME, 145);
        assert_eq!(IDR_ARKHATA, 146);
        assert_eq!(
            execute_item_driver(&mut character, &mut bridge, request, 1, false),
            ItemDriverOutcome::LibloadAreaBlocked {
                driver: IDR_BONEBRIDGE,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                required_area: 18,
            }
        );
    }

    #[test]
    fn arkhata_key_assemble_ports_legacy_combinations() {
        let mut character = character(1);
        character.cursor_item = Some(ItemId(9));
        let mut key = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_ARKHATA);
        key.template_id = IID_ARKHATA_AKEY12;
        key.carried_by = Some(CharacterId(1));
        set_drdata(&mut key, 0, 2);
        let request = ItemDriverRequest::Driver {
            driver: IDR_ARKHATA,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(IID_ARKHATA_AKEY1, 0x0100_00CA);
        assert_eq!(IID_ARKHATA_AKEY, 0x3B00_0089);
        assert_eq!(
            execute_item_driver(&mut character, &mut key, request, 37, false),
            ItemDriverOutcome::ArkhataKeyDoesNotFit {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );

        let mut context = ItemDriverContext {
            cursor_template_id: Some(IID_ARKHATA_AKEY3),
            ..Default::default()
        };
        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut key,
                request,
                37,
                false,
                &context
            ),
            ItemDriverOutcome::ArkhataKeyAssemble {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                cursor_item_id: ItemId(9),
                result_template_id: IID_ARKHATA_AKEY,
                result_sprite: 13413,
                final_key: true,
            }
        );

        key.template_id = IID_ARKHATA_AKEY1;
        context.cursor_template_id = Some(IID_ARKHATA_AKEY2);
        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut key,
                request,
                37,
                false,
                &context
            ),
            ItemDriverOutcome::ArkhataKeyAssemble {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                cursor_item_id: ItemId(9),
                result_template_id: IID_ARKHATA_AKEY12,
                result_sprite: 13421,
                final_key: false,
            }
        );
    }

    #[test]
    fn arkhata_pool_dispatch_ports_cursor_gates() {
        let mut character = character(1);
        let mut pool = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_ARKHATA);
        set_drdata(&mut pool, 0, 0);
        let request = ItemDriverRequest::Driver {
            driver: IDR_ARKHATA,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(IID_ARKHATA_SCROLL1, 0x0100_00C2);
        assert_eq!(
            execute_item_driver(&mut character, &mut pool, request, 37, false),
            ItemDriverOutcome::ArkhataPoolNeedsCursor {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );

        character.cursor_item = Some(ItemId(9));
        let wrong_context = ItemDriverContext {
            cursor_template_id: Some(0x0100_00C3),
            ..Default::default()
        };
        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut pool,
                request,
                37,
                false,
                &wrong_context
            ),
            ItemDriverOutcome::ArkhataPoolWrongCursor {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                cursor_item_id: ItemId(9),
            }
        );

        let scroll_context = ItemDriverContext {
            cursor_template_id: Some(IID_ARKHATA_SCROLL1),
            ..Default::default()
        };
        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut pool,
                request,
                37,
                false,
                &scroll_context
            ),
            ItemDriverOutcome::ArkhataPool {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                cursor_item_id: ItemId(9),
            }
        );
    }

    #[test]
    fn arkhata_stopwatch_dispatch_is_timer_only_and_reschedules() {
        let mut character = character(0);
        let mut stopwatch = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_ARKHATA);
        stopwatch.carried_by = Some(CharacterId(7));
        set_drdata(&mut stopwatch, 0, 1);
        let request = ItemDriverRequest::Driver {
            driver: IDR_ARKHATA,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut stopwatch, request, 37, false),
            ItemDriverOutcome::ArkhataStopwatch {
                item_id: ItemId(8),
                character_id: CharacterId(7),
                schedule_after_ticks: 10,
            }
        );

        character.id = CharacterId(7);
        assert_eq!(
            execute_item_driver(&mut character, &mut stopwatch, request, 37, false),
            ItemDriverOutcome::Noop
        );

        character.id = CharacterId(0);
        stopwatch.carried_by = None;
        assert_eq!(
            execute_item_driver(&mut character, &mut stopwatch, request, 37, false),
            ItemDriverOutcome::Noop
        );
    }

    #[test]
    fn nomad_dice_driver_requires_carried_item_and_reports_luck() {
        let mut character = character(1);
        let mut dice = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_NOMADDICE);
        set_drdata(&mut dice, 0, 2);
        let request = ItemDriverRequest::Driver {
            driver: IDR_NOMADDICE,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut dice, request, 19, false),
            ItemDriverOutcome::Noop
        );

        dice.carried_by = Some(CharacterId(1));
        assert_eq!(
            execute_item_driver(&mut character, &mut dice, request, 19, false),
            ItemDriverOutcome::NomadDice {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                luck: 2,
            }
        );

        assert_eq!(
            execute_item_driver(&mut character, &mut dice, request, 1, false),
            ItemDriverOutcome::LibloadAreaBlocked {
                driver: IDR_NOMADDICE,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                required_area: 19,
            }
        );
    }

    #[test]
    fn legacy_lucky_die_uses_best_of_luck_plus_one_rolls() {
        assert_eq!(legacy_lucky_die_from_rolls(6, 0, [2, 6, 6]), 2);
        assert_eq!(legacy_lucky_die_from_rolls(6, 2, [2, 5, 3, 6]), 5);
        assert_eq!(legacy_lucky_die_from_rolls(6, 1, [0, 9]), 6);
        assert_eq!(
            legacy_nomad_dice_total(
                1,
                [
                    [1, 6, 1, 1, 1, 1, 1, 1],
                    [2, 4, 1, 1, 1, 1, 1, 1],
                    [3, 1, 1, 1, 1, 1, 1, 1]
                ]
            ),
            13
        );
    }

    #[test]
    fn libload_area_guards_fall_through_inside_legacy_area() {
        let mut character = character(1);
        let mut bridge = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BONEBRIDGE);
        let request = ItemDriverRequest::Driver {
            driver: IDR_BONEBRIDGE,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut bridge, request, 18, false),
            ItemDriverOutcome::Noop
        );
    }

    #[test]
    fn bonebridge_driver_requires_full_area18_bone_cursor_and_ports_timer_boundary() {
        let mut actor = character(1);
        actor.cursor_item = Some(ItemId(9));
        let mut bridge = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BONEBRIDGE);
        let request = ItemDriverRequest::Driver {
            driver: IDR_BONEBRIDGE,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut actor,
                &mut bridge,
                request,
                18,
                false,
                &ItemDriverContext {
                    cursor_template_id: Some(IID_AREA18_BONE),
                    cursor_drdata0: Some(4),
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::Noop
        );

        assert_eq!(
            execute_item_driver_with_context(
                &mut actor,
                &mut bridge,
                request,
                18,
                false,
                &ItemDriverContext {
                    cursor_template_id: Some(IID_AREA18_BONE),
                    cursor_drdata0: Some(5),
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::BoneBridgePlace {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                cursor_item_id: ItemId(9),
            }
        );

        bridge.driver_data = vec![0, 1];
        let mut timer_character = character(0);
        let timer_request = ItemDriverRequest::Driver {
            driver: IDR_BONEBRIDGE,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        };
        assert_eq!(
            execute_item_driver_with_context(
                &mut timer_character,
                &mut bridge,
                timer_request,
                18,
                false,
                &ItemDriverContext {
                    timer_call: true,
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::BoneBridgeTimerTick { item_id: ItemId(8) }
        );
    }

    #[test]
    fn book_driver_ignores_timer_style_zero_character_calls() {
        let mut character = character(0);
        let mut book = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BOOK);
        let request = ItemDriverRequest::Driver {
            driver: IDR_BOOK,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut book, request, 2, false),
            ItemDriverOutcome::Noop
        );
    }

    #[test]
    fn bonehint_driver_initializes_carried_diary_hint() {
        let mut character = character(1);
        let mut diary = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BONEHINT);
        diary.carried_by = Some(character.id);
        set_drdata(&mut diary, 0, 7);
        let request = ItemDriverRequest::Driver {
            driver: IDR_BONEHINT,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(IDR_BONEHINT, 94);
        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut diary,
                request,
                18,
                false,
                &ItemDriverContext {
                    bone_hint_nr: Some(3),
                    bone_hint_pos: Some(2),
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::BoneHint {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                level: 7,
                nr: 3,
                pos: 2,
            }
        );
        assert_eq!(drdata(&diary, 1), 1);
        assert_eq!(drdata(&diary, 2), 3);
        assert_eq!(drdata(&diary, 3), 2);
    }

    #[test]
    fn bonehint_driver_requires_carried_item() {
        let mut character = character(1);
        let mut diary = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BONEHINT);
        let request = ItemDriverRequest::Driver {
            driver: IDR_BONEHINT,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut diary, request, 18, false),
            ItemDriverOutcome::Noop
        );
    }

    #[test]
    fn boneladder_driver_ports_paired_ladder_offsets() {
        let mut character = character(1);
        let mut ladder = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BONELADDER);
        ladder.x = 100;
        ladder.y = 80;
        let request = ItemDriverRequest::Driver {
            driver: IDR_BONELADDER,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(IDR_BONELADDER, 90);
        assert_eq!(
            execute_item_driver(&mut character, &mut ladder, request, 18, false),
            ItemDriverOutcome::Teleport {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                x: 104,
                y: 83,
                area_id: 0,
                stop_driver: false,
                quiet: false,
            }
        );

        set_drdata(&mut ladder, 0, 1);
        assert_eq!(
            execute_item_driver(&mut character, &mut ladder, request, 18, false),
            ItemDriverOutcome::Teleport {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                x: 96,
                y: 77,
                area_id: 0,
                stop_driver: false,
                quiet: false,
            }
        );
    }

    #[test]
    fn boneladder_driver_preserves_area18_libload_guard() {
        let mut character = character(1);
        let mut ladder = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BONELADDER);
        let request = ItemDriverRequest::Driver {
            driver: IDR_BONELADDER,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut ladder, request, 1, false),
            ItemDriverOutcome::LibloadAreaBlocked {
                driver: IDR_BONELADDER,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                required_area: 18,
            }
        );
    }

    #[test]
    fn staffer2_animation_book_reports_legacy_exp_for_runtime_ppd_gate() {
        let mut reader = character(1);
        reader.flags.insert(CharacterFlags::PLAYER);
        reader.level = 60;
        let mut book = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_STAFFER2);
        set_drdata(&mut book, 0, 6);
        let request = ItemDriverRequest::Driver {
            driver: IDR_STAFFER2,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(IDR_STAFFER2, 122);
        assert_eq!(legacy_level_value(60), 885_841);
        assert_eq!(
            execute_item_driver(&mut reader, &mut book, request, 29, false),
            ItemDriverOutcome::StafferAnimationBook {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                exp_added: 177_168,
            }
        );
        assert_eq!(reader.exp, 0);
    }

    #[test]
    fn staffer2_animation_book_requires_area29_and_player_character() {
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        let mut book = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_STAFFER2);
        set_drdata(&mut book, 0, 6);
        let request = ItemDriverRequest::Driver {
            driver: IDR_STAFFER2,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut player, &mut book, request, 1, false),
            ItemDriverOutcome::LibloadAreaBlocked {
                driver: IDR_STAFFER2,
                item_id: ItemId(8),
                character_id: CharacterId(1),
                required_area: 29,
            }
        );

        let mut npc = character(2);
        let npc_request = ItemDriverRequest::Driver {
            driver: IDR_STAFFER2,
            item_id: ItemId(8),
            character_id: CharacterId(2),
            spec: 0,
        };
        assert_eq!(
            execute_item_driver(&mut npc, &mut book, npc_request, 29, false),
            ItemDriverOutcome::Noop
        );
        assert_eq!(npc.exp, 0);
    }

    #[test]
    fn staffer2_book_cycles_pages_and_resets_per_reader() {
        let mut reader = character(1);
        reader.flags.insert(CharacterFlags::PLAYER);
        let mut book = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_STAFFER2);
        set_drdata(&mut book, 0, 1);
        let request = ItemDriverRequest::Driver {
            driver: IDR_STAFFER2,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut reader, &mut book, request, 29, false),
            ItemDriverOutcome::StafferBookText {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                page: 0,
            }
        );
        assert_eq!(drdata(&book, 1), 1);
        assert_eq!(drdata_u32(&book, 4), 1);

        for expected_page in 1..=4 {
            assert_eq!(
                execute_item_driver(&mut reader, &mut book, request, 29, false),
                ItemDriverOutcome::StafferBookText {
                    item_id: ItemId(8),
                    character_id: CharacterId(1),
                    page: expected_page,
                }
            );
        }
        assert_eq!(drdata(&book, 1), 0);

        let mut other = character(2);
        other.flags.insert(CharacterFlags::PLAYER);
        let other_request = ItemDriverRequest::Driver {
            driver: IDR_STAFFER2,
            item_id: ItemId(8),
            character_id: CharacterId(2),
            spec: 0,
        };
        assert_eq!(
            execute_item_driver(&mut other, &mut book, other_request, 29, false),
            ItemDriverOutcome::StafferBookText {
                item_id: ItemId(8),
                character_id: CharacterId(2),
                page: 0,
            }
        );
        assert_eq!(drdata_u32(&book, 4), 2);
    }

    #[test]
    fn staffer_book_text_preserves_legacy_prompts() {
        assert_eq!(
            staffer_book_text(0).unwrap(),
            "The training of these thieves into skilled mages has been succesful. They can now create Golems, and summon the old enemies of Aston, the Grolms. I will not teach them how to create and control Undead though, lest they use them against me... Also, to this end, I have enlisted the help of an assassin by the name of Brenneth. I hope he will not disappoint me..."
        );
        assert_eq!(
            staffer_book_continue_text(3),
            Some("USE again to continue.")
        );
        assert_eq!(staffer_book_continue_text(4), Some("USE to start over."));
        assert_eq!(staffer_book_text(5), None);
    }

    #[test]
    fn staffer2_mine_dig_ports_endurance_and_stage_progression() {
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.endurance = POWERSCALE * 2;
        player.professions[2] = 25;
        let mut mine = item(
            8,
            ItemFlags::USED | ItemFlags::USE | ItemFlags::SIGHTBLOCK,
            0,
            IDR_STAFFER2,
        );
        mine.sprite = 15070;
        set_drdata(&mut mine, 0, 2);
        let request = ItemDriverRequest::Driver {
            driver: IDR_STAFFER2,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut player, &mut mine, request, 29, false),
            ItemDriverOutcome::StafferMineDig {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
        assert_eq!(player.endurance, POWERSCALE * 2);
        assert_eq!(drdata(&mine, 3), 1);
        assert_eq!(mine.sprite, 15071);
    }

    #[test]
    fn staffer2_mine_dig_blocks_exhausted_players() {
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        player.endurance = POWERSCALE - 1;
        let mut mine = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_STAFFER2);
        set_drdata(&mut mine, 0, 2);
        let request = ItemDriverRequest::Driver {
            driver: IDR_STAFFER2,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut player, &mut mine, request, 29, false),
            ItemDriverOutcome::StafferMineExhausted {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );
        assert_eq!(drdata(&mine, 3), 0);
    }

    #[test]
    fn staffer2_block_dispatches_player_and_timer_paths() {
        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        let mut block = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_STAFFER2);
        set_drdata(&mut block, 0, 3);
        let request = ItemDriverRequest::Driver {
            driver: IDR_STAFFER2,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut player, &mut block, request, 29, false),
            ItemDriverOutcome::StafferBlockMove {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );

        let mut timer = character(0);
        let timer_request = ItemDriverRequest::Driver {
            driver: IDR_STAFFER2,
            item_id: ItemId(8),
            character_id: CharacterId(0),
            spec: 0,
        };
        assert_eq!(
            execute_item_driver(&mut timer, &mut block, timer_request, 29, false),
            ItemDriverOutcome::StafferBlockTimer { item_id: ItemId(8) }
        );
    }

    #[test]
    fn staffer2_special_door_subtypes_dispatch_to_typed_outcome() {
        let mut character = character(1);
        character.flags.insert(CharacterFlags::PLAYER);
        let request = ItemDriverRequest::Driver {
            driver: IDR_STAFFER2,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };

        for subtype in 4..=5 {
            let mut item = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_STAFFER2);
            set_drdata(&mut item, 0, subtype);
            item.x = 10;
            item.y = 10;
            assert_eq!(
                execute_item_driver(&mut character, &mut item, request, 29, false),
                ItemDriverOutcome::StafferSpecDoorToggle {
                    item_id: ItemId(8),
                    character_id: CharacterId(1),
                    kind: subtype,
                }
            );
        }
    }

    #[test]
    fn book_text_lines_include_static_later_legacy_books() {
        assert_eq!(
            book_text_lines(24)[0],
            "Day 155, year 103. Personal diary of Kamaleon of the Isara."
        );
        assert_eq!(
            book_text_lines(30)[5],
            "The cold is slowly killing all of us. All attempts to control the demon lords have failed. Now all of us must die. But I shall die happily if I can take Ishtar with me into the cold."
        );
        assert_eq!(
            book_text_lines(38),
            &["Berkano, Ehwaz, Ansuz will decrease magic damage."][..]
        );
        assert_eq!(
            book_text_lines(100)[2],
            "The last fight with the undeads was hard. But even though I am bleeding from many wounds, today is the day I will kill my brother. I will take the amulet and go into the family vault and face him now!"
        );
    }

    #[test]
    fn book_text_lines_include_raw_color_marker_book_cases() {
        assert_eq!(
            book_text_lines(31)[0],
            "Personal Diary of Korzam, Magical Advisor of Scarcewind."
        );
        assert_eq!(book_text_lines(39)[2], "You skip some pages.");
        assert_eq!(
            book_text_lines(43),
            &["Thou canst comprehend the intricate handwriting fully, something about an incantation of transportation. It sounds like folly and you choose not to decipher more of the scribbles."][..]
        );
        assert_eq!(
            book_text_lines(44)[2],
            "At the bottom of the following page you find a list of the current teachers of the mages order: Bretl, Anna-Sofia, Leaner, Crem, Guiwynn."
        );

        let runes = book_text_line_bytes(31);
        assert_eq!(&runes[1][..3], crate::text::COL_DARK_GRAY);
        assert!(runes[1].ends_with(b"replaced by:"));

        let mad_mages = book_text_line_bytes(44);
        assert_eq!(&mad_mages[2][..3], crate::text::COL_DARK_GRAY);
        assert!(mad_mages[2]
            .windows(3)
            .any(|bytes| bytes == crate::text::COL_RESET));
        assert!(mad_mages[2].ends_with(b"Bretl, Anna-Sofia, Leaner, Crem, Guiwynn."));
    }

    #[test]
    fn bookcase_driver_ports_locked_and_text_boundaries() {
        let mut actor = character(1);
        actor.flags.insert(CharacterFlags::PLAYER);
        let request = ItemDriverRequest::Driver {
            driver: IDR_BOOKCASE,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        };
        let mut bookcase = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_BOOKCASE);
        bookcase.driver_data = vec![1];

        assert_eq!(
            execute_item_driver_with_context(
                &mut actor,
                &mut bookcase,
                request,
                17,
                false,
                &ItemDriverContext::default(),
            ),
            ItemDriverOutcome::BookcaseLocked {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );

        let context = ItemDriverContext {
            has_area17_library_key: true,
            ..ItemDriverContext::default()
        };
        assert_eq!(
            execute_item_driver_with_context(
                &mut actor,
                &mut bookcase,
                request,
                17,
                false,
                &context
            ),
            ItemDriverOutcome::BookcaseText {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                kind: 1,
            }
        );
    }

    #[test]
    fn bookcase_text_matches_legacy_title_format() {
        let locked = bookcase_locked_text_lines();
        assert_eq!(
            locked[0],
            "The bookcase is locked and you do not have the right key."
        );
        assert_eq!(
            bookcase_text_line_bytes(3, 0, 2, false),
            [
                crate::text::COL_LIGHT_GREEN,
                b"A Green Day in the Life of a Warrior by C. O. Nan.",
                crate::text::COL_RESET,
                b" After reading the title you put the book back.",
            ]
            .concat()
        );
        assert_eq!(
            bookcase_text_line_bytes(0, 3, 1, false),
            [
                crate::text::COL_LIGHT_GREEN,
                b"Secrets of Adygalah Alchemy by Leonarda.",
                crate::text::COL_RESET,
                b" One recipe most mages will find useful uses Adygalah, Bhalkissa and Firuba, plus one berry and one or two mushrooms.",
            ]
            .concat()
        );
    }

    #[test]
    fn earth_demon_sign_books_use_reader_demon_knowledge() {
        assert_eq!(
            book_text_line_bytes_for_reader(18, 0),
            vec![b"It's written in strange letters you cannot read.".to_vec()]
        );
        assert_eq!(
            book_text_line_bytes_for_reader(19, 1),
            vec![b"You recognice some of the letters used in this sign from your studies of the ancient knowledge, but you cannot tell what the sign means.".to_vec()]
        );
        assert_eq!(
            book_text_line_bytes_for_reader(18, 2),
            vec![b"Defense Systems Control Room".to_vec()]
        );
        assert_eq!(
            book_text_line_bytes_for_reader(19, 2),
            vec![
                b"Research Laboratorium".to_vec(),
                b"Caution, live demons!".to_vec(),
            ]
        );
    }

    #[test]
    fn demon_books_generate_legacy_character_specific_ritual_words() {
        assert_eq!(demonspeak(6, 2), "shirsli sausgadul");
        assert_eq!(
            book_text_line_bytes_for_reader_id(15, 0, 6),
            vec![b"'shirsli sausgadul' will give thee even better protection.".to_vec()]
        );
        assert_eq!(
            book_text_line_bytes_for_reader_id(13, 0, 6),
            vec![b"I have seen in written in fiery letters upon the sky: Those who have the knowledge can invoke protection against demonic might by uttering the words: 'dorsli kilaghshir'".to_vec()]
        );
        assert_ne!(
            book_text_line_bytes_for_reader_id(13, 0, 6),
            book_text_line_bytes_for_reader_id(13, 0, 7)
        );
    }

    #[test]
    fn book_nook_joke_lines_match_legacy_random_cases() {
        assert_eq!(
            book_nook_joke_line_bytes(0),
            vec![
                b"What did the fisherman say to the card magician?".to_vec(),
                b"Pick a cod, any cod!".to_vec(),
            ]
        );
        assert_eq!(
            book_nook_joke_line_bytes(4),
            vec![
                b"What bone will a dog never eat?".to_vec(),
                b"A trombone.".to_vec(),
            ]
        );
        assert_eq!(book_nook_joke_line_bytes(9), book_nook_joke_line_bytes(4));
    }

    #[test]
    fn book_special_effects_match_legacy_earth_demon_diaries() {
        assert_eq!(book_special_effect(22), Some(50287));
        assert_eq!(book_special_effect(23), Some(50305));
        assert_eq!(book_special_effect(20), None);
        assert_eq!(book_special_effect(24), None);
    }

    #[test]
    fn legacy_item_driver_return_code_matches_c_driver_contract() {
        assert_eq!(
            legacy_item_driver_return_code(None, &ItemDriverOutcome::Noop),
            0
        );
        assert_eq!(
            legacy_item_driver_return_code(
                Some(IDR_POTION),
                &ItemDriverOutcome::Unsupported {
                    driver: IDR_POTION,
                    item_id: ItemId(7),
                    character_id: CharacterId(1),
                },
            ),
            0
        );
        assert_eq!(
            legacy_item_driver_return_code(Some(IDR_DOOR), &ItemDriverOutcome::Noop),
            2
        );
        assert_eq!(
            legacy_item_driver_return_code(
                Some(IDR_DOOR),
                &ItemDriverOutcome::DoorToggle {
                    item_id: ItemId(7),
                    character_id: CharacterId(1),
                },
            ),
            1
        );
        assert_eq!(
            legacy_item_driver_return_code(
                Some(IDR_DOUBLE_DOOR),
                &ItemDriverOutcome::DoubleDoorToggle {
                    item_id: ItemId(8),
                    character_id: CharacterId(1),
                },
            ),
            1
        );
        assert_eq!(
            legacy_item_driver_return_code(
                Some(IDR_CHEST),
                &ItemDriverOutcome::ChestTreasure {
                    item_id: ItemId(9),
                    character_id: CharacterId(1),
                    treasure_index: 3,
                },
            ),
            1
        );
        assert_eq!(
            legacy_item_driver_return_code(
                Some(IDR_BOOK),
                &ItemDriverOutcome::BookText {
                    item_id: ItemId(10),
                    character_id: CharacterId(1),
                    kind: 8,
                    demon_value: 0,
                },
            ),
            1
        );
    }

    #[test]
    fn nomad_stack_driver_dispatches_for_carried_items() {
        let mut character = character(1);
        let mut stack = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_NOMADSTACK);
        stack.carried_by = Some(character.id);
        character.inventory[30] = Some(stack.id);

        assert_eq!(
            execute_item_driver(
                &mut character,
                &mut stack,
                ItemDriverRequest::Driver {
                    driver: IDR_NOMADSTACK,
                    item_id: ItemId(7),
                    character_id: CharacterId(1),
                    spec: 0,
                },
                1,
                false,
            ),
            ItemDriverOutcome::NomadStack {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn transport_driver_opens_valid_points_and_rejects_invalid_points() {
        let mut character = character(1);
        let mut transport = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_TRANSPORT);
        let request = ItemDriverRequest::Driver {
            driver: IDR_TRANSPORT,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        transport.driver_data = vec![25];
        assert_eq!(
            execute_item_driver(&mut character, &mut transport, request, 1, false),
            ItemDriverOutcome::TransportOpen {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                point: 25,
            }
        );

        transport.driver_data = vec![26];
        assert_eq!(
            execute_item_driver(&mut character, &mut transport, request, 1, false),
            ItemDriverOutcome::TransportInvalid {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                point: 26,
            }
        );

        transport.driver_data = vec![LEGACY_TRANSPORT_CLAN_EXIT];
        assert!(matches!(
            execute_item_driver(&mut character, &mut transport, request, 1, false),
            ItemDriverOutcome::TransportOpen {
                point: LEGACY_TRANSPORT_CLAN_EXIT,
                ..
            }
        ));

        let travel_request = ItemDriverRequest::Driver {
            driver: IDR_TRANSPORT,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 22 + 5 * 256,
        };
        assert_eq!(
            execute_item_driver(&mut character, &mut transport, travel_request, 1, false),
            ItemDriverOutcome::TransportTravel {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 22 + 5 * 256,
            }
        );
    }

    #[test]
    fn toplist_driver_dispatches_for_players_only() {
        let mut character = character(1);
        let mut toplist = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_TOPLIST);
        let request = ItemDriverRequest::Driver {
            driver: IDR_TOPLIST,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut toplist, request, 1, false),
            ItemDriverOutcome::ArenaToplist {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        character.id = CharacterId(0);
        assert_eq!(
            execute_item_driver(&mut character, &mut toplist, request, 1, false),
            ItemDriverOutcome::Noop
        );
    }

    #[test]
    fn arena_toplist_lines_match_legacy_rank_window() {
        let entries: Vec<ArenaToplistEntry> = (0..20)
            .map(|index| ArenaToplistEntry {
                name: format!("Fighter{index}"),
                score: 2000 - index * 100,
            })
            .collect();

        let lines = arena_toplist_lines(&entries, 650, 3, 2, 5);

        assert_eq!(lines[0], "1: Fighter0 2000");
        assert_eq!(lines[9], "10: Fighter9 1100");
        assert_eq!(lines[10], "11: Fighter10 1000");
        assert_eq!(lines[14], "15: Fighter14 600");
        assert_eq!(
            lines.last().unwrap(),
            "Your score is 650, you have won 3 fights and lost 2 fights."
        );
    }

    #[test]
    fn arena_toplist_lines_use_legacy_newcomer_score() {
        let entries = vec![ArenaToplistEntry {
            name: "Champion".to_string(),
            score: 42,
        }];

        let lines = arena_toplist_lines(&entries, 500, 0, 0, 0);

        assert_eq!(lines[0], "1: Champion 42");
        assert_eq!(
            lines[1],
            "Your score is -2000, you have won 0 fights and lost 0 fights."
        );
    }

    #[test]
    fn demon_chip_driver_dispatches_to_stack_outcome() {
        let mut character = character(1);
        let mut stack = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DEMONCHIP);
        stack.carried_by = Some(character.id);
        character.inventory[30] = Some(stack.id);

        assert_eq!(
            execute_item_driver(
                &mut character,
                &mut stack,
                ItemDriverRequest::Driver {
                    driver: IDR_DEMONCHIP,
                    item_id: ItemId(7),
                    character_id: CharacterId(1),
                    spec: 0,
                },
                1,
                false,
            ),
            ItemDriverOutcome::NomadStack {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn xmasmaker_driver_only_dispatches_for_staff_or_god() {
        let mut character = character(1);
        let mut maker = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_XMASMAKER);

        let request = ItemDriverRequest::Driver {
            driver: IDR_XMASMAKER,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };
        assert_eq!(
            execute_item_driver(&mut character, &mut maker, request, 1, false),
            ItemDriverOutcome::Noop
        );

        character.flags.insert(CharacterFlags::STAFF);
        assert_eq!(
            execute_item_driver(&mut character, &mut maker, request, 1, false),
            ItemDriverOutcome::XmasMaker {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn zombie_shrine_requires_matching_skull_on_cursor() {
        let mut character = character(1);
        let mut shrine = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SHRINE);
        shrine.driver_data = vec![1];
        let request = ItemDriverRequest::Driver {
            driver: IDR_SHRINE,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut shrine,
                request,
                2,
                false,
                &ItemDriverContext::default(),
            ),
            ItemDriverOutcome::ZombieShrineNeedsOffering {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                shrine_type: 1,
            }
        );

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut shrine,
                request,
                2,
                false,
                &ItemDriverContext {
                    cursor_template_id: Some(IID_AREA2_ZOMBIESKULL2),
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::ZombieShrine {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                shrine_type: 1,
            }
        );
    }

    #[test]
    fn xmastree_driver_dispatches_for_character_use() {
        let mut character = character(1);
        let mut tree = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_XMASTREE);

        assert_eq!(
            execute_item_driver(
                &mut character,
                &mut tree,
                ItemDriverRequest::Driver {
                    driver: IDR_XMASTREE,
                    item_id: ItemId(7),
                    character_id: CharacterId(1),
                    spec: 0,
                },
                1,
                false,
            ),
            ItemDriverOutcome::XmasTree {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn execute_potion_driver_restores_resources_and_consumes_non_empty_potion() {
        let mut character = character(1);
        character.hp = 1_000;
        character.mana = 2_000;
        character.endurance = 3_000;
        character.values[0][CharacterValue::Hp as usize] = 10;
        character.values[0][CharacterValue::Mana as usize] = 10;
        character.values[0][CharacterValue::Endurance as usize] = 10;
        character.inventory[30] = Some(ItemId(7));
        let mut item = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_POTION);
        item.carried_by = Some(CharacterId(1));
        item.driver_data = vec![0, 20, 3, 4];

        let outcome = execute_item_driver(
            &mut character,
            &mut item,
            ItemDriverRequest::Driver {
                driver: IDR_POTION,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::PotionDrunk {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                hp_added: 9_000,
                mana_added: 3_000,
                endurance_added: 4_000,
            }
        );
        assert_eq!(
            (character.hp, character.mana, character.endurance),
            (10_000, 5_000, 7_000)
        );
        assert_eq!(character.inventory[30], None);
        assert!(!item.flags.contains(ItemFlags::USED));
    }

    #[test]
    fn execute_potion_driver_defers_empty_bottle_template_creation() {
        let mut character = character(1);
        character.values[0][CharacterValue::Hp as usize] = 10;
        let mut item = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_POTION);
        item.carried_by = Some(CharacterId(1));
        item.driver_data = vec![2, 5, 0, 0];

        let outcome = execute_item_driver(
            &mut character,
            &mut item,
            ItemDriverRequest::Driver {
                driver: IDR_POTION,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::EmptyPotionTemplateNeeded {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                empty_kind: 2,
            }
        );
        assert!(item.flags.contains(ItemFlags::USED));
        assert_eq!(character.hp, 0);
    }

    #[test]
    fn special_potion_type_7_resets_professions_and_lowers_profession_points() {
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(7));
        character.professions[0] = 2;
        character.professions[3] = 4;
        character.values[1][CharacterValue::Profession as usize] = 10;
        character.exp = 10_000;
        character.exp_used = 5_000;
        let mut potion = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPECIAL_POTION);
        potion.carried_by = Some(CharacterId(1));
        potion.driver_data = vec![7];

        let outcome = execute_item_driver(
            &mut character,
            &mut potion,
            ItemDriverRequest::Driver {
                driver: IDR_SPECIAL_POTION,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::SpecialPotionProfessionReset {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                used: true,
                professions_reset: 6,
                profession_points_lowered: 2,
                exp_refunded: 2_240,
            }
        );
        assert!(character.professions.iter().all(|&value| value == 0));
        assert_eq!(character.values[1][CharacterValue::Profession as usize], 8);
        assert_eq!((character.exp, character.exp_used), (7_760, 2_760));
        assert_eq!(character.inventory[30], None);
        assert!(!potion.flags.contains(ItemFlags::USED));
        assert!(character
            .flags
            .contains(CharacterFlags::PROF | CharacterFlags::UPDATE));
    }

    #[test]
    fn special_potion_type_7_blocks_without_professions() {
        let mut character = character(1);
        let mut potion = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPECIAL_POTION);
        potion.carried_by = Some(CharacterId(1));
        potion.driver_data = vec![7];

        let outcome = execute_item_driver(
            &mut character,
            &mut potion,
            ItemDriverRequest::Driver {
                driver: IDR_SPECIAL_POTION,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::SpecialPotionProfessionReset {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                used: false,
                professions_reset: 0,
                profession_points_lowered: 0,
                exp_refunded: 0,
            }
        );
        assert!(potion.flags.contains(ItemFlags::USED));
    }

    #[test]
    fn execute_food_driver_consumes_simple_food_and_ports_special_food() {
        let mut character = character(1);
        character.cursor_item = Some(ItemId(7));
        let mut food = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FOOD);
        food.carried_by = Some(CharacterId(1));
        food.driver_data = vec![1];

        let outcome = execute_item_driver(
            &mut character,
            &mut food,
            ItemDriverRequest::Driver {
                driver: IDR_FOOD,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::FoodEaten {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                kind: 1,
            }
        );
        assert_eq!(character.cursor_item, None);
        assert!(!food.flags.contains(ItemFlags::USED));

        let mut lollipop = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FOOD);
        lollipop.carried_by = Some(CharacterId(1));
        lollipop.driver_data = vec![2, 0];
        lollipop.sprite = 100;
        lollipop.description = "A sweet lollipop.".to_string();
        character.level = 10;
        character.exp = 7;
        assert_eq!(
            execute_item_driver(
                &mut character,
                &mut lollipop,
                ItemDriverRequest::Driver {
                    driver: IDR_FOOD,
                    item_id: ItemId(8),
                    character_id: CharacterId(1),
                    spec: 0,
                },
                1,
                false,
            ),
            ItemDriverOutcome::LollipopLicked {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                exp_added: 6,
                lick_count: 1,
            }
        );
        assert_eq!(lollipop.sprite, 101);
        assert_eq!(lollipop.driver_data[1], 1);
        assert_eq!(
            lollipop.description,
            "A sweet lollipop. Well, it's already used."
        );
        assert_eq!(character.exp, 13);
        assert!(lollipop.flags.contains(ItemFlags::USED));

        lollipop.driver_data[1] = 7;
        assert_eq!(
            execute_item_driver(
                &mut character,
                &mut lollipop,
                ItemDriverRequest::Driver {
                    driver: IDR_FOOD,
                    item_id: ItemId(8),
                    character_id: CharacterId(1),
                    spec: 0,
                },
                1,
                false,
            ),
            ItemDriverOutcome::LollipopLicked {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                exp_added: 6,
                lick_count: 8,
            }
        );
        assert_eq!(lollipop.driver_data[1], 8);
        assert_eq!(lollipop.description, "A lollipop stick.");

        assert_eq!(
            execute_item_driver(
                &mut character,
                &mut lollipop,
                ItemDriverRequest::Driver {
                    driver: IDR_FOOD,
                    item_id: ItemId(8),
                    character_id: CharacterId(1),
                    spec: 0,
                },
                1,
                false,
            ),
            ItemDriverOutcome::LollipopMemories {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );

        let mut xmaspop = item(9, ItemFlags::USED | ItemFlags::USE, 0, IDR_FOOD);
        xmaspop.carried_by = Some(CharacterId(1));
        xmaspop.driver_data = vec![3];
        assert_eq!(
            execute_item_driver(
                &mut character,
                &mut xmaspop,
                ItemDriverRequest::Driver {
                    driver: IDR_FOOD,
                    item_id: ItemId(9),
                    character_id: CharacterId(1),
                    spec: 0,
                },
                1,
                false,
            ),
            ItemDriverOutcome::ChristmasPopInspected {
                item_id: ItemId(9),
                character_id: CharacterId(1),
            }
        );
        assert!(xmaspop.flags.contains(ItemFlags::USED));
    }

    #[test]
    fn execute_decaying_item_toggles_carried_modifiers_and_schedules_timer() {
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(7));
        let mut decaying = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DECAYITEM);
        decaying.carried_by = Some(CharacterId(1));
        decaying.sprite = 100;
        decaying.modifier_value = [1, 0, 2, 0, 3];
        decaying.driver_data = vec![0, 4, 9, 0, 0, 2, 0];
        let request = ItemDriverRequest::Driver {
            driver: IDR_DECAYITEM,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut decaying, request, 1, false),
            ItemDriverOutcome::DecayItemToggled {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                active: true,
                schedule_after_ticks: Some(TICKS_PER_SECOND * 2),
            }
        );
        assert_eq!(decaying.driver_data[0], 1);
        assert_eq!(decaying.sprite, 101);
        assert_eq!(decaying.modifier_value, [9, 0, 9, 0, 9]);
        assert!(character.flags.contains(CharacterFlags::ITEMS));

        assert_eq!(
            execute_item_driver(&mut character, &mut decaying, request, 1, false),
            ItemDriverOutcome::DecayItemToggled {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                active: false,
                schedule_after_ticks: None,
            }
        );
        assert_eq!(decaying.driver_data[0], 0);
        assert_eq!(decaying.sprite, 100);
        assert_eq!(decaying.modifier_value, [4, 0, 4, 0, 4]);
    }

    #[test]
    fn execute_decaying_item_timer_ages_active_item_until_expiry() {
        let mut timer_character = character(0);
        let mut decaying = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DECAYITEM);
        decaying.name = "Vanishing Charm".into();
        decaying.carried_by = Some(CharacterId(1));
        decaying.driver_data = vec![1, 4, 9, 1, 0, 2, 0];
        let request = ItemDriverRequest::Driver {
            driver: IDR_DECAYITEM,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        };
        let context = ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut timer_character,
                &mut decaying,
                request,
                1,
                false,
                &context,
            ),
            ItemDriverOutcome::DecayItemToggled {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                active: true,
                schedule_after_ticks: Some(TICKS_PER_SECOND * 2),
            }
        );
        assert_eq!(decaying.driver_data[3], 2);
        assert_eq!(decaying.driver_data[4], 0);

        assert_eq!(
            execute_item_driver_with_context(
                &mut timer_character,
                &mut decaying,
                request,
                1,
                false,
                &context,
            ),
            ItemDriverOutcome::DecayItemExpired {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                item_name: outcome_item_name("Vanishing Charm"),
            }
        );
        assert_eq!(decaying.driver_data[3], 3);
        assert_eq!(decaying.driver_data[4], 0);

        decaying.driver_data[0] = 0;
        assert_eq!(
            execute_item_driver_with_context(
                &mut timer_character,
                &mut decaying,
                request,
                1,
                false,
                &context,
            ),
            ItemDriverOutcome::Noop
        );
    }

    #[test]
    fn execute_stat_scroll_raises_value_grants_exp_and_consumes_item() {
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(7));
        character.values[0][CharacterValue::Sword as usize] = 10;
        character.values[1][CharacterValue::Sword as usize] = 10;
        let mut scroll = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_STATSCROLL);
        scroll.carried_by = Some(CharacterId(1));
        scroll.driver_data = vec![CharacterValue::Sword as u8, 2];

        let outcome = execute_item_driver(
            &mut character,
            &mut scroll,
            ItemDriverRequest::Driver {
                driver: IDR_STATSCROLL,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::StatScrollUsed {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                value: CharacterValue::Sword as u8,
                raised: 2,
                exp_cost: 746,
            }
        );
        assert_eq!(character.values[1][CharacterValue::Sword as usize], 12);
        assert_eq!(character.values[0][CharacterValue::Sword as usize], 12);
        assert_eq!(character.exp, 746);
        assert_eq!(character.exp_used, 746);
        assert_eq!(character.inventory[30], None);
        assert!(!scroll.flags.contains(ItemFlags::USED));
    }

    #[test]
    fn execute_stat_scroll_blocks_unusable_cases_without_consuming() {
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(7));
        character.values[0][CharacterValue::Armor as usize] = 10;
        character.values[1][CharacterValue::Armor as usize] = 10;
        let mut scroll = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_STATSCROLL);
        scroll.carried_by = Some(CharacterId(1));
        scroll.driver_data = vec![CharacterValue::Armor as u8, 1];
        let request = ItemDriverRequest::Driver {
            driver: IDR_STATSCROLL,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut scroll, request, 1, false),
            ItemDriverOutcome::BlockedByRequirements {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
        assert_eq!(character.inventory[30], Some(ItemId(7)));
        assert!(scroll.flags.contains(ItemFlags::USED));

        scroll.driver_data = vec![CharacterValue::Sword as u8, 1];
        character.values[1][CharacterValue::Sword as usize] = 10;
        character.flags.insert(CharacterFlags::NOEXP);
        assert_eq!(
            execute_item_driver(&mut character, &mut scroll, request, 1, false),
            ItemDriverOutcome::BlockedByRequirements {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        character.flags.remove(CharacterFlags::NOEXP);
        scroll.carried_by = None;
        assert_eq!(
            execute_item_driver(&mut character, &mut scroll, request, 1, false),
            ItemDriverOutcome::Noop
        );
    }

    #[test]
    fn execute_door_driver_returns_toggle_or_key_block() {
        let mut character = character(1);
        let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DOOR);
        door.x = 10;
        door.y = 11;

        let request = ItemDriverRequest::Driver {
            driver: IDR_DOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut door, request, 1, false),
            ItemDriverOutcome::DoorToggle {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        door.driver_data = vec![0, 1, 0, 0, 0];
        assert_eq!(
            execute_item_driver(&mut character, &mut door, request, 1, false),
            ItemDriverOutcome::BlockedByRequirements {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        door.x = 0;
        door.driver_data.clear();
        assert_eq!(
            execute_item_driver(&mut character, &mut door, request, 1, false),
            ItemDriverOutcome::Noop
        );
    }

    #[test]
    fn execute_door_driver_accepts_key_context() {
        let mut character = character(1);
        let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DOOR);
        door.x = 10;
        door.y = 11;
        door.driver_data = vec![1, 0x44, 0x33, 0x22, 0x11];
        let request = ItemDriverRequest::Driver {
            driver: IDR_DOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };
        let context = ItemDriverContext {
            door_key: Some(DoorKeyAccess {
                key_id: 0x1122_3344,
                name: "Copper Key".to_string(),
                source: DoorKeySource::Keyring,
            }),
            cursor_template_id: None,
            ..ItemDriverContext::default()
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut door,
                request,
                1,
                false,
                &context,
            ),
            ItemDriverOutcome::KeyedDoorToggle {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                key_id: 0x1122_3344,
                source: DoorKeySource::Keyring,
                locking: true,
            }
        );
    }

    #[test]
    fn execute_double_door_driver_returns_typed_toggle() {
        let mut character = character(1);
        let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DOUBLE_DOOR);
        door.x = 10;
        door.y = 11;
        let request = ItemDriverRequest::Driver {
            driver: IDR_DOUBLE_DOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut door, request, 1, false),
            ItemDriverOutcome::DoubleDoorToggle {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        door.x = 0;
        assert_eq!(
            execute_item_driver(&mut character, &mut door, request, 1, false),
            ItemDriverOutcome::Noop
        );
    }

    #[test]
    fn execute_assemble_driver_maps_legacy_combinations() {
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(7));
        character.cursor_item = Some(ItemId(8));
        let mut item = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_ASSEMBLE);
        item.carried_by = Some(CharacterId(1));
        item.template_id = IID_AREA2_SUN1;
        let request = ItemDriverRequest::Driver {
            driver: IDR_ASSEMBLE,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };
        let context = ItemDriverContext {
            door_key: None,
            cursor_template_id: Some(IID_AREA2_SUN23),
            ..ItemDriverContext::default()
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut item,
                request,
                1,
                false,
                &context
            ),
            ItemDriverOutcome::AssembleItem {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                cursor_item_id: ItemId(8),
                template: AssembleTemplate::SunAmulet123,
            }
        );

        item.template_id = IID_STAFF_REDKEY2;
        let context = ItemDriverContext {
            door_key: None,
            cursor_template_id: Some(IID_STAFF_REDKEY13),
            ..ItemDriverContext::default()
        };
        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut item,
                request,
                1,
                false,
                &context
            ),
            ItemDriverOutcome::AssembleItem {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                cursor_item_id: ItemId(8),
                template: AssembleTemplate::WarrRedkey123,
            }
        );
    }

    #[test]
    fn execute_assemble_driver_reports_legacy_failures() {
        let mut character = character(1);
        let mut item = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_ASSEMBLE);
        item.carried_by = Some(CharacterId(1));
        item.template_id = IID_AREA2_SUN1;
        let request = ItemDriverRequest::Driver {
            driver: IDR_ASSEMBLE,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut item, request, 1, false),
            ItemDriverOutcome::AssembleNeedsCursor {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        character.cursor_item = Some(ItemId(8));
        assert_eq!(
            execute_item_driver(&mut character, &mut item, request, 1, false),
            ItemDriverOutcome::AssembleDoesNotFit {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        item.template_id = 0xDEAD_BEEF;
        assert_eq!(
            execute_item_driver(&mut character, &mut item, request, 1, false),
            ItemDriverOutcome::AssembleUnknownItem {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn execute_palace_key_driver_splits_and_combines_legacy_sprites() {
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(7));
        let mut key_part = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_PALACEKEY);
        key_part.carried_by = Some(CharacterId(1));
        key_part.template_id = IID_AREA11_PALACEKEYPART;
        key_part.sprite = 51021;
        let request = ItemDriverRequest::Driver {
            driver: IDR_PALACEKEY,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut key_part, request, 11, false),
            ItemDriverOutcome::PalaceKeySplit {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                cursor_part_sprite: 51015,
                carried_part_sprite: 51016,
            }
        );

        character.cursor_item = Some(ItemId(8));
        key_part.sprite = 51015;
        let context = ItemDriverContext {
            cursor_template_id: Some(IID_AREA11_PALACEKEYPART),
            cursor_sprite: Some(51039),
            ..ItemDriverContext::default()
        };
        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut key_part,
                request,
                11,
                false,
                &context,
            ),
            ItemDriverOutcome::PalaceKeyCombine {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                cursor_item_id: ItemId(8),
                result_sprite: 51014,
                final_key: true,
            }
        );
    }

    #[test]
    fn execute_palace_key_driver_reports_legacy_failures() {
        let mut character = character(1);
        let mut key_part = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_PALACEKEY);
        key_part.carried_by = Some(CharacterId(1));
        key_part.template_id = IID_AREA11_PALACEKEYPART;
        key_part.sprite = 51015;
        let request = ItemDriverRequest::Driver {
            driver: IDR_PALACEKEY,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut key_part, request, 11, false),
            ItemDriverOutcome::PalaceKeyNeedsCursor {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        character.cursor_item = Some(ItemId(8));
        assert_eq!(
            execute_item_driver(&mut character, &mut key_part, request, 11, false),
            ItemDriverOutcome::PalaceKeyDoesNotFit {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn execute_shrike_amulet_driver_combines_non_overlapping_parts() {
        let mut character = character(1);
        character.inventory[30] = Some(ItemId(7));
        character.cursor_item = Some(ItemId(8));
        let mut amulet = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SHRIKEAMULET);
        amulet.carried_by = Some(CharacterId(1));
        amulet.driver_data = vec![1];
        let request = ItemDriverRequest::Driver {
            driver: IDR_SHRIKEAMULET,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        let outcome = execute_item_driver_with_context(
            &mut character,
            &mut amulet,
            request,
            1,
            false,
            &ItemDriverContext {
                cursor_driver: Some(IDR_SHRIKEAMULET),
                cursor_drdata0: Some(2),
                ..ItemDriverContext::default()
            },
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::ShrikeAmuletAssemble {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                cursor_item_id: ItemId(8),
                combined_bits: 3,
            }
        );

        let outcome = execute_item_driver_with_context(
            &mut character,
            &mut amulet,
            request,
            1,
            false,
            &ItemDriverContext {
                cursor_driver: Some(IDR_SHRIKEAMULET),
                cursor_drdata0: Some(1),
                ..ItemDriverContext::default()
            },
        );
        assert_eq!(
            outcome,
            ItemDriverOutcome::ShrikeAmuletDoesNotFit {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn execute_mine_gateway_key_driver_combines_key_bits() {
        let mut character = character(1);
        character.cursor_item = Some(ItemId(8));
        let mut key = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_MINEGATEWAYKEY);
        key.driver_data = vec![1];
        let request = ItemDriverRequest::Driver {
            driver: IDR_MINEGATEWAYKEY,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut key,
                request,
                1,
                false,
                &ItemDriverContext {
                    cursor_driver: Some(IDR_MINEGATEWAYKEY),
                    cursor_drdata0: Some(14),
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::MineGatewayKeyAssemble {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                cursor_item_id: ItemId(8),
                combined_bits: 15,
            }
        );

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut key,
                request,
                1,
                false,
                &ItemDriverContext {
                    cursor_driver: Some(IDR_SHRIKEAMULET),
                    cursor_drdata0: Some(2),
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::MineGatewayKeyDoesNotFit {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn execute_chest_driver_returns_treasure_or_blocks() {
        let mut character = character(1);
        let mut chest = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_CHEST);
        chest.driver_data = vec![9];
        let request = ItemDriverRequest::Driver {
            driver: IDR_CHEST,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut chest, request, 1, false),
            ItemDriverOutcome::ChestTreasure {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                treasure_index: 9,
            }
        );

        character.cursor_item = Some(ItemId(99));
        assert_eq!(
            execute_item_driver(&mut character, &mut chest, request, 1, false),
            ItemDriverOutcome::BlockedByRequirements {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        character.cursor_item = None;
        chest.driver_data = vec![9, 1, 0, 0, 0];
        assert_eq!(
            execute_item_driver(&mut character, &mut chest, request, 1, false),
            ItemDriverOutcome::ChestTreasure {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                treasure_index: 9,
            }
        );
    }

    #[test]
    fn execute_randchest_driver_returns_runtime_outcome_even_with_cursor_item() {
        let mut character = character(1);
        character.cursor_item = Some(ItemId(99));
        let mut chest = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_RANDCHEST);
        let request = ItemDriverRequest::Driver {
            driver: IDR_RANDCHEST,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut chest, request, 1, false),
            ItemDriverOutcome::RandomChest {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn execute_infinite_chest_maps_rune_kind_to_template() {
        let mut character = character(1);
        let mut chest = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_INFINITE_CHEST);
        chest.driver_data = vec![4];

        let outcome = execute_item_driver(
            &mut character,
            &mut chest,
            ItemDriverRequest::Driver {
                driver: IDR_INFINITE_CHEST,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::InfiniteChest {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                template: InfiniteChestTemplate::Rune4,
                key_name: None,
            }
        );
    }

    #[test]
    fn execute_infinite_chest_requires_empty_cursor_before_key_checks() {
        let mut character = character(1);
        character.cursor_item = Some(ItemId(9));
        let mut chest = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_INFINITE_CHEST);
        chest.driver_data = vec![1, 0x44, 0x33, 0x22, 0x11];

        let outcome = execute_item_driver(
            &mut character,
            &mut chest,
            ItemDriverRequest::Driver {
                driver: IDR_INFINITE_CHEST,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::InfiniteChestCursorOccupied {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn execute_infinite_chest_requires_matching_key_when_configured() {
        let mut character = character(1);
        let mut chest = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_INFINITE_CHEST);
        chest.driver_data = vec![1, 0x44, 0x33, 0x22, 0x11];

        let missing = execute_item_driver(
            &mut character,
            &mut chest.clone(),
            ItemDriverRequest::Driver {
                driver: IDR_INFINITE_CHEST,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        );
        assert_eq!(
            missing,
            ItemDriverOutcome::InfiniteChestKeyRequired {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        let outcome = execute_item_driver_with_context(
            &mut character,
            &mut chest,
            ItemDriverRequest::Driver {
                driver: IDR_INFINITE_CHEST,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
            &ItemDriverContext {
                door_key: Some(DoorKeyAccess {
                    key_id: 0x1122_3344,
                    name: "Palace Key".to_string(),
                    source: DoorKeySource::Carried,
                }),
                ..ItemDriverContext::default()
            },
        );
        assert_eq!(
            outcome,
            ItemDriverOutcome::InfiniteChest {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                template: InfiniteChestTemplate::Rune1,
                key_name: Some(outcome_item_name("Palace Key")),
            }
        );
    }

    #[test]
    fn execute_infinite_chest_rejects_skeleton_key() {
        let mut character = character(1);
        let mut chest = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_INFINITE_CHEST);
        chest.driver_data = vec![1, 0x44, 0x33, 0x22, 0x11];

        let outcome = execute_item_driver_with_context(
            &mut character,
            &mut chest,
            ItemDriverRequest::Driver {
                driver: IDR_INFINITE_CHEST,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
            &ItemDriverContext {
                door_key: Some(DoorKeyAccess {
                    key_id: IID_SKELETON_KEY,
                    name: "Skeleton Key".to_string(),
                    source: DoorKeySource::Carried,
                }),
                ..ItemDriverContext::default()
            },
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::InfiniteChestKeyRequired {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn pick_chest_requires_lockpick_and_empty_cursor() {
        let mut character = character(1);
        let mut chest = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_PICKCHEST);
        chest.driver_data = vec![2];
        let request = ItemDriverRequest::Driver {
            driver: IDR_PICKCHEST,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut chest,
                request,
                17,
                false,
                &ItemDriverContext::default(),
            ),
            ItemDriverOutcome::PickChestLocked {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        character.cursor_item = Some(ItemId(9));
        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut chest,
                request,
                17,
                false,
                &ItemDriverContext {
                    has_area17_lockpick: true,
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::PickChestCursorOccupied {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn pick_chest_maps_legacy_note_kinds() {
        let mut character = character(1);
        let mut chest = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_PICKCHEST);
        let request = ItemDriverRequest::Driver {
            driver: IDR_PICKCHEST,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };
        let context = ItemDriverContext {
            has_area17_lockpick: true,
            ..ItemDriverContext::default()
        };

        for (kind, template) in [
            (0, PickChestTemplate::PalaceNote1),
            (1, PickChestTemplate::PalaceNote2),
            (2, PickChestTemplate::PalaceNote3),
            (3, PickChestTemplate::MerchantNote1),
        ] {
            chest.driver_data = vec![kind];
            assert_eq!(
                execute_item_driver_with_context(
                    &mut character,
                    &mut chest,
                    request,
                    17,
                    false,
                    &context,
                ),
                ItemDriverOutcome::PickChest {
                    item_id: ItemId(7),
                    character_id: CharacterId(1),
                    template,
                }
            );
        }

        chest.driver_data = vec![4];
        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut chest,
                request,
                17,
                false,
                &context,
            ),
            ItemDriverOutcome::PickChestBug {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn pick_door_requires_area17_lockpick_for_players() {
        let mut character = character(1);
        character.flags.insert(CharacterFlags::PLAYER);
        let mut door = item(
            7,
            ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK | ItemFlags::DOOR,
            0,
            IDR_PICKDOOR,
        );
        door.driver_data = vec![0];
        let request = ItemDriverRequest::Driver {
            driver: IDR_PICKDOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut door,
                request,
                17,
                false,
                &ItemDriverContext::default(),
            ),
            ItemDriverOutcome::PickDoorLocked {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut door,
                request,
                17,
                false,
                &ItemDriverContext {
                    has_area17_lockpick: true,
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::PickDoorToggle {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn pick_door_timer_only_closes_open_doors() {
        let mut timer = character(0);
        let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_PICKDOOR);
        let request = ItemDriverRequest::Driver {
            driver: IDR_PICKDOOR,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        };

        door.driver_data = vec![0];
        assert_eq!(
            execute_item_driver_with_context(
                &mut timer,
                &mut door,
                request,
                17,
                false,
                &ItemDriverContext {
                    timer_call: true,
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::Noop
        );

        door.driver_data = vec![1];
        assert_eq!(
            execute_item_driver_with_context(
                &mut timer,
                &mut door,
                request,
                17,
                false,
                &ItemDriverContext {
                    timer_call: true,
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::PickDoorToggle {
                item_id: ItemId(7),
                character_id: CharacterId(0),
            }
        );
    }

    #[test]
    fn pent_boss_door_preserves_legacy_access_and_position_checks() {
        let mut character = character(1);
        character.x = 11;
        character.y = 10;
        let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_PENTBOSSDOOR);
        door.x = 10;
        door.y = 10;
        let request = ItemDriverRequest::Driver {
            driver: IDR_PENTBOSSDOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut door,
                request,
                4,
                false,
                &ItemDriverContext::default(),
            ),
            ItemDriverOutcome::PentBossDoorLocked {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut door,
                request,
                4,
                false,
                &ItemDriverContext {
                    current_tick: 1_000,
                    pent_last_solve_tick: Some(900),
                    pent_demon_lord_access_seconds: Some(120),
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::PentBossDoor {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                x: 9,
                y: 10,
            }
        );

        character.y = 11;
        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut door,
                request,
                4,
                false,
                &ItemDriverContext {
                    current_tick: 1_000,
                    pent_last_solve_tick: Some(900),
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::Noop
        );
    }

    #[test]
    fn pent_drivers_keep_area4_libload_guard() {
        let mut character = character(1);
        let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_PENTBOSSDOOR);
        let request = ItemDriverRequest::Driver {
            driver: IDR_PENTBOSSDOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut door,
                request,
                1,
                false,
                &ItemDriverContext::default(),
            ),
            ItemDriverOutcome::LibloadAreaBlocked {
                driver: IDR_PENTBOSSDOOR,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                required_area: 4,
            }
        );
    }

    #[test]
    fn burndown_driver_ports_touch_ignite_and_timer_gates() {
        let mut actor = character(1);
        let mut barrel = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_BURNDOWN);
        let request = ItemDriverRequest::Driver {
            driver: IDR_BURNDOWN,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        barrel.driver_data = vec![0];
        assert_eq!(
            execute_item_driver_with_context(
                &mut actor,
                &mut barrel,
                request,
                17,
                false,
                &ItemDriverContext::default(),
            ),
            ItemDriverOutcome::BurndownTouch {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        assert_eq!(
            execute_item_driver_with_context(
                &mut actor,
                &mut barrel,
                request,
                17,
                false,
                &ItemDriverContext {
                    cursor_driver: Some(IDR_TORCH),
                    cursor_drdata0: Some(1),
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::BurndownIgnite {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        barrel.driver_data = vec![16];
        assert_eq!(
            execute_item_driver_with_context(
                &mut actor,
                &mut barrel,
                request,
                17,
                false,
                &ItemDriverContext::default(),
            ),
            ItemDriverOutcome::BurndownTooHot {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        barrel.driver_data = vec![1];
        assert_eq!(
            execute_item_driver_with_context(
                &mut actor,
                &mut barrel,
                request,
                17,
                false,
                &ItemDriverContext::default(),
            ),
            ItemDriverOutcome::BurndownAlreadyBurned {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        let mut timer = character(0);
        assert_eq!(
            execute_item_driver_with_context(
                &mut timer,
                &mut barrel,
                ItemDriverRequest::Driver {
                    driver: IDR_BURNDOWN,
                    item_id: ItemId(7),
                    character_id: CharacterId(0),
                    spec: 0,
                },
                17,
                false,
                &ItemDriverContext {
                    timer_call: true,
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::BurndownTimerTick { item_id: ItemId(7) }
        );
    }

    #[test]
    fn colortile_reports_legacy_row_and_color_for_players_only() {
        let mut character = character(1);
        character.flags.insert(CharacterFlags::PLAYER);
        let mut tile = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_COLORTILE);
        tile.driver_data = vec![3, 5];
        let request = ItemDriverRequest::Driver {
            driver: IDR_COLORTILE,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut tile,
                request,
                17,
                false,
                &ItemDriverContext::default(),
            ),
            ItemDriverOutcome::ColorTile {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                row: 3,
                color: 5,
            }
        );

        character.flags.remove(CharacterFlags::PLAYER);
        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut tile,
                request,
                17,
                false,
                &ItemDriverContext::default(),
            ),
            ItemDriverOutcome::Noop
        );
    }

    #[test]
    fn execute_keyring_driver_shows_or_requests_cursor_key_add() {
        let mut character = character(1);
        let mut keyring = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_KEY_RING);
        let request = ItemDriverRequest::Driver {
            driver: IDR_KEY_RING,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut keyring, request, 1, false),
            ItemDriverOutcome::KeyringShow {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        character.cursor_item = Some(ItemId(99));
        assert_eq!(
            execute_item_driver(&mut character, &mut keyring, request, 1, false),
            ItemDriverOutcome::KeyringAddCursorItem {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                key_item_id: ItemId(99),
            }
        );
    }

    #[test]
    fn execute_freakdoor_driver_returns_link_metadata() {
        let mut character = character(1);
        let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FREAKDOOR);
        door.driver_data = vec![0; 16];
        door.driver_data[8] = 42;
        door.driver_data[10..14].copy_from_slice(&99_u32.to_le_bytes());
        door.driver_data[14] = 1;
        door.driver_data[15] = 1;
        let request = ItemDriverRequest::Driver {
            driver: IDR_FREAKDOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut door, request, 10, false),
            ItemDriverOutcome::FreakDoorUse {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                link_group: 42,
                one_way: true,
                recursion_guard: false,
                cached_partner_id: Some(ItemId(99)),
                no_target: true,
            }
        );
    }

    #[test]
    fn execute_teleport_door_driver_moves_to_opposite_side() {
        let mut character = character(1);
        character.x = 9;
        character.y = 10;
        character.level = 5;
        let mut door = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_TELE_DOOR);
        door.x = 10;
        door.y = 10;
        door.driver_data = vec![0, 10];

        let request = ItemDriverRequest::Driver {
            driver: IDR_TELE_DOOR,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut door, request, 1, false),
            ItemDriverOutcome::TeleportDoor {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                x: 11,
                y: 10,
            }
        );

        door.driver_data[0] = 2;
        assert_eq!(
            execute_item_driver(&mut character, &mut door, request, 1, false),
            ItemDriverOutcome::Noop
        );

        door.driver_data = vec![0, 4];
        assert_eq!(
            execute_item_driver(&mut character, &mut door, request, 1, false),
            ItemDriverOutcome::BlockedByRequirements {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn execute_teleport_driver_decodes_target_and_checks_requirements() {
        let mut character = character(1);
        character.level = 10;
        let mut item = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_TELEPORT);
        item.min_level = 5;
        item.max_level = 20;
        item.driver_data = vec![44, 1, 88, 2, 3, 0, 1, 0, 0, 0, 0, 0, 1];

        let outcome = execute_item_driver(
            &mut character,
            &mut item,
            ItemDriverRequest::Driver {
                driver: IDR_TELEPORT,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::Teleport {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                x: 300,
                y: 600,
                area_id: 3,
                stop_driver: true,
                quiet: true,
            }
        );

        item.driver_data[10] = 1;
        assert_eq!(
            execute_item_driver(
                &mut character,
                &mut item,
                ItemDriverRequest::Driver {
                    driver: IDR_TELEPORT,
                    item_id: ItemId(7),
                    character_id: CharacterId(1),
                    spec: 0,
                },
                1,
                false,
            ),
            ItemDriverOutcome::BlockedByRequirements {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn execute_recall_driver_targets_character_rest_area_and_checks_level() {
        let mut character = character(1);
        character.level = 10;
        character.rest_area = 3;
        character.rest_x = 44;
        character.rest_y = 55;
        let mut item = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_RECALL);
        item.carried_by = Some(CharacterId(1));
        item.driver_data = vec![20];

        let request = ItemDriverRequest::Driver {
            driver: IDR_RECALL,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };
        assert_eq!(
            execute_item_driver(&mut character, &mut item, request, 1, false),
            ItemDriverOutcome::Recall {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                x: 44,
                y: 55,
                area_id: 3,
            }
        );

        item.driver_data = vec![9];
        assert_eq!(
            execute_item_driver(&mut character, &mut item, request, 1, false),
            ItemDriverOutcome::BlockedByRequirements {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn execute_city_recall_driver_maps_scroll_types_and_blocks_arena() {
        let mut character = character(1);
        character.level = 99;
        let mut item = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_CITY_RECALL);
        item.carried_by = Some(CharacterId(1));
        item.driver_data = vec![7, 3];

        let request = ItemDriverRequest::Driver {
            driver: IDR_CITY_RECALL,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut item, request, 1, false),
            ItemDriverOutcome::CityRecall {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                x: 203,
                y: 227,
                area_id: 29,
            }
        );

        assert_eq!(
            execute_item_driver(&mut character, &mut item, request, 34, true),
            ItemDriverOutcome::BlockedByArea {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        item.driver_data = vec![99, 3];
        assert_eq!(
            execute_item_driver(&mut character, &mut item, request, 1, false),
            ItemDriverOutcome::BlockedByRequirements {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn toylight_toggles_light_state_on_character_use() {
        let mut character = character(1);
        let mut light = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_TOYLIGHT);
        light.driver_data = vec![0, 12];
        let request = ItemDriverRequest::Driver {
            driver: IDR_TOYLIGHT,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut light, request, 1, false),
            ItemDriverOutcome::LightChanged {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                schedule_after_ticks: None,
            }
        );
        assert_eq!(light.driver_data[0], 1);
        assert_eq!(light.modifier_index[0], V_LIGHT);
        assert_eq!(light.modifier_value[0], 12);
        assert_eq!(light.sprite, 1);

        execute_item_driver(&mut character, &mut light, request, 1, false);
        assert_eq!(light.driver_data[0], 0);
        assert_eq!(light.modifier_value[0], 0);
        assert_eq!(light.sprite, 0);
    }

    #[test]
    fn nightlight_timer_follows_daylight_threshold_and_reschedules() {
        let mut character = character(1);
        let mut light = item(7, ItemFlags::USED, 0, IDR_NIGHTLIGHT);
        light.driver_data = vec![0, 9];
        let request = ItemDriverRequest::Driver {
            driver: IDR_NIGHTLIGHT,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };
        let mut context = ItemDriverContext {
            timer_call: true,
            daylight: 79,
            ..ItemDriverContext::default()
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut light,
                request,
                1,
                false,
                &context
            ),
            ItemDriverOutcome::LightChanged {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                schedule_after_ticks: Some(LIGHT_TIMER_TICKS),
            }
        );
        assert_eq!(light.driver_data[0], 1);
        assert_eq!(light.modifier_value[0], 9);
        assert_eq!(light.sprite, 1);

        context.daylight = 81;
        execute_item_driver_with_context(&mut character, &mut light, request, 1, false, &context);
        assert_eq!(light.driver_data[0], 0);
        assert_eq!(light.modifier_value[0], 0);
        assert_eq!(light.sprite, 0);
    }

    #[test]
    fn onofflight_timer_registers_and_use_toggles_light_state() {
        let mut timer_character = character(0);
        let mut light = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_ONOFFLIGHT);
        light.driver_data = vec![1, 15];
        light.modifier_index[0] = V_LIGHT;
        light.modifier_value[0] = 15;
        light.sprite = 101;
        let request = ItemDriverRequest::Driver {
            driver: IDR_ONOFFLIGHT,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut timer_character,
                &mut light,
                request,
                3,
                false,
                &ItemDriverContext {
                    timer_call: true,
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::Noop
        );
        assert_eq!(light.driver_data[6], 1);
        assert_eq!(light.driver_data[0], 1);
        assert_eq!(light.modifier_value[0], 15);
        assert_eq!(light.sprite, 101);

        let mut character = character(1);
        let request = ItemDriverRequest::Driver {
            driver: IDR_ONOFFLIGHT,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };
        assert_eq!(
            execute_item_driver(&mut character, &mut light, request, 3, false),
            ItemDriverOutcome::OnOffLightChanged {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                now_on: false,
                remaining_off: None,
                gates_opened: false,
            }
        );
        assert_eq!(light.driver_data[0], 0);
        assert_eq!(light.modifier_value[0], 0);
        assert_eq!(light.sprite, 100);

        execute_item_driver(&mut character, &mut light, request, 3, false);
        assert_eq!(light.driver_data[0], 1);
        assert_eq!(light.modifier_index[0], V_LIGHT);
        assert_eq!(light.modifier_value[0], 15);
        assert_eq!(light.sprite, 101);
    }

    #[test]
    fn edemon_switch_use_disables_fire_and_schedules_reenable() {
        let mut character = character(1);
        let mut lever = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_EDEMONSWITCH);
        lever.sprite = 100;
        lever.driver_data = vec![1, 0, 0, 0, 0];
        lever.modifier_index[0] = V_LIGHT;
        lever.modifier_value[0] = 64;
        let request = ItemDriverRequest::Driver {
            driver: IDR_EDEMONSWITCH,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut lever,
                request,
                6,
                false,
                &ItemDriverContext {
                    current_tick: 123,
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::LightChanged {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                schedule_after_ticks: Some(EDEMON_SWITCH_COOLDOWN_TICKS + 1),
            }
        );
        assert_eq!(lever.driver_data[0], 0);
        assert_eq!(
            u32::from_le_bytes(lever.driver_data[1..5].try_into().unwrap()),
            123 + EDEMON_SWITCH_COOLDOWN_TICKS as u32
        );
        assert_eq!(lever.modifier_value[0], 0);
        assert_eq!(lever.sprite, 101);
    }

    #[test]
    fn edemon_switch_timer_reenables_after_cooldown() {
        let mut timer_character = character(0);
        let mut lever = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_EDEMONSWITCH);
        lever.sprite = 101;
        lever.driver_data = vec![0, 10, 0, 0, 0];
        let request = ItemDriverRequest::Driver {
            driver: IDR_EDEMONSWITCH,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut timer_character,
                &mut lever,
                request,
                6,
                false,
                &ItemDriverContext {
                    timer_call: true,
                    current_tick: 10,
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::Noop
        );

        assert_eq!(
            execute_item_driver_with_context(
                &mut timer_character,
                &mut lever,
                request,
                6,
                false,
                &ItemDriverContext {
                    timer_call: true,
                    current_tick: 11,
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::LightChanged {
                item_id: ItemId(7),
                character_id: CharacterId(0),
                schedule_after_ticks: None,
            }
        );
        assert_eq!(lever.driver_data[0], 1);
        assert_eq!(lever.modifier_index[0], V_LIGHT);
        assert_eq!(lever.modifier_value[0], 64);
        assert_eq!(lever.sprite, 100);
    }

    #[test]
    fn edemon_switch_reports_stuck_while_fire_is_disabled() {
        let mut character = character(1);
        let mut lever = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_EDEMONSWITCH);
        lever.sprite = 100;
        lever.driver_data = vec![0, 0, 0, 0, 0];
        let request = ItemDriverRequest::Driver {
            driver: IDR_EDEMONSWITCH,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut lever, request, 6, false),
            ItemDriverOutcome::EdemonSwitchStuck {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
        assert_eq!(lever.sprite, 100);
    }

    #[test]
    fn edemon_light_timer_follows_section_power_and_reschedules() {
        let mut timer_character = character(0);
        let mut light = item(7, ItemFlags::USED, 0, IDR_EDEMONLIGHT);
        light.sprite = 14189;
        let request = ItemDriverRequest::Driver {
            driver: IDR_EDEMONLIGHT,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        };

        assert_eq!(IDR_EDEMONLIGHT, 40);
        assert_eq!(
            execute_item_driver_with_context(
                &mut timer_character,
                &mut light,
                request,
                6,
                false,
                &ItemDriverContext {
                    timer_call: true,
                    edemon_section_power: Some(42),
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::LightChanged {
                item_id: ItemId(7),
                character_id: CharacterId(0),
                schedule_after_ticks: Some(TICKS_PER_SECOND),
            }
        );
        assert_eq!(light.sprite, 14191);
        assert_eq!(light.modifier_index[0], V_LIGHT);
        assert_eq!(light.modifier_value[0], 200);

        execute_item_driver_with_context(
            &mut timer_character,
            &mut light,
            request,
            6,
            false,
            &ItemDriverContext {
                timer_call: true,
                edemon_section_power: Some(249),
                ..ItemDriverContext::default()
            },
        );
        assert_eq!(light.sprite, 14189);
        assert_eq!(light.modifier_value[0], 0);

        let mut user = character(1);
        assert_eq!(
            execute_item_driver_with_context(
                &mut user,
                &mut light,
                ItemDriverRequest::Driver {
                    driver: IDR_EDEMONLIGHT,
                    item_id: ItemId(7),
                    character_id: CharacterId(1),
                    spec: 0,
                },
                6,
                false,
                &ItemDriverContext::default(),
            ),
            ItemDriverOutcome::Noop
        );
    }

    #[test]
    fn fdemon_light_timer_follows_loader_power_and_reschedules() {
        let mut timer_character = character(0);
        let mut light = item(7, ItemFlags::USED, 0, IDR_FDEMONLIGHT);
        light.sprite = 14189;
        let request = ItemDriverRequest::Driver {
            driver: IDR_FDEMONLIGHT,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut timer_character,
                &mut light,
                request,
                8,
                false,
                &ItemDriverContext {
                    timer_call: true,
                    fdemon_loader_power: Some(300),
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::LightChanged {
                item_id: ItemId(7),
                character_id: CharacterId(0),
                schedule_after_ticks: Some(TICKS_PER_SECOND),
            }
        );
        assert_eq!(light.sprite, 14192);
        assert_eq!(light.modifier_index[0], V_LIGHT);
        assert_eq!(light.modifier_value[0], 200);

        execute_item_driver_with_context(
            &mut timer_character,
            &mut light,
            request,
            8,
            false,
            &ItemDriverContext {
                timer_call: true,
                fdemon_loader_power: Some(0),
                ..ItemDriverContext::default()
            },
        );
        assert_eq!(light.sprite, 14189);
        assert_eq!(light.modifier_value[0], 0);
    }

    #[test]
    fn fdemon_light_preserves_area8_libload_guard_and_player_noop() {
        let mut timer_character = character(0);
        let mut light = item(7, ItemFlags::USED, 0, IDR_FDEMONLIGHT);
        let request = ItemDriverRequest::Driver {
            driver: IDR_FDEMONLIGHT,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut timer_character,
                &mut light,
                request,
                6,
                false,
                &ItemDriverContext {
                    timer_call: true,
                    fdemon_loader_power: Some(300),
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::LibloadAreaBlocked {
                driver: IDR_FDEMONLIGHT,
                item_id: ItemId(7),
                character_id: CharacterId(0),
                required_area: 8,
            }
        );

        let mut user = character(1);
        assert_eq!(
            execute_item_driver_with_context(
                &mut user,
                &mut light,
                ItemDriverRequest::Driver {
                    driver: IDR_FDEMONLIGHT,
                    item_id: ItemId(7),
                    character_id: CharacterId(1),
                    spec: 0,
                },
                8,
                false,
                &ItemDriverContext::default(),
            ),
            ItemDriverOutcome::Noop
        );
    }

    #[test]
    fn fdemon_farm_timer_grows_and_exposes_crystal_overlay() {
        let mut timer_character = character(0);
        let mut farm = item(7, ItemFlags::USED, 0, IDR_FDEMONFARM);
        farm.driver_data = vec![5, 24, 20];
        let request = ItemDriverRequest::Driver {
            driver: IDR_FDEMONFARM,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut timer_character,
                &mut farm,
                request,
                8,
                false,
                &ItemDriverContext {
                    timer_call: true,
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::FdemonFarmChanged {
                item_id: ItemId(7),
                character_id: CharacterId(0),
                foreground_sprite: 0,
                schedule_after_ticks: Some(TICKS_PER_SECOND * 2),
            }
        );
        assert_eq!(farm.driver_data[2], 25);

        assert_eq!(
            execute_item_driver_with_context(
                &mut timer_character,
                &mut farm,
                request,
                8,
                false,
                &ItemDriverContext {
                    timer_call: true,
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::FdemonFarmChanged {
                item_id: ItemId(7),
                character_id: CharacterId(0),
                foreground_sprite: 59040,
                schedule_after_ticks: Some(TICKS_PER_SECOND * 2),
            }
        );
        assert_eq!(farm.driver_data[2], 25);
    }

    #[test]
    fn fdemon_farm_player_harvest_and_block_messages_are_typed() {
        let mut user = character(1);
        let mut farm = item(7, ItemFlags::USED, 0, IDR_FDEMONFARM);
        farm.driver_data = vec![5, 48, 48];
        let request = ItemDriverRequest::Driver {
            driver: IDR_FDEMONFARM,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut user, &mut farm, request, 8, false),
            ItemDriverOutcome::FdemonFarmHarvest {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                template: FdemonCrystalTemplate::Giant,
                foreground_sprite: 0,
            }
        );
        assert_eq!(farm.driver_data[2], 0);

        assert_eq!(
            execute_item_driver(&mut user, &mut farm, request, 8, false),
            ItemDriverOutcome::FdemonFarmNotReady {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                current: 5,
                required: 48,
            }
        );

        user.cursor_item = Some(ItemId(99));
        assert_eq!(
            execute_item_driver(&mut user, &mut farm, request, 8, false),
            ItemDriverOutcome::FdemonFarmCursorOccupied {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );
    }

    #[test]
    fn palace_gate_only_dispatches_for_zero_character_timer_calls() {
        let mut timer_character = character(0);
        let mut gate = item(7, ItemFlags::USED, 0, IDR_PALACEGATE);
        let request = ItemDriverRequest::Driver {
            driver: IDR_PALACEGATE,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut timer_character,
                &mut gate,
                request,
                3,
                false,
                &ItemDriverContext {
                    timer_call: true,
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::PalaceGateTick {
                item_id: ItemId(7),
                opened: false,
                closed: false,
                blocked: false,
            }
        );

        assert_eq!(
            execute_item_driver(&mut character(1), &mut gate, request, 3, false),
            ItemDriverOutcome::Noop
        );
    }

    #[test]
    fn torch_user_use_lights_and_extinguishes_carried_torch() {
        let mut character = character(1);
        let mut torch = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_TORCH);
        torch.carried_by = Some(CharacterId(1));
        torch.driver_data = vec![0, 0, 10, 20];
        let request = ItemDriverRequest::Driver {
            driver: IDR_TORCH,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        execute_item_driver(&mut character, &mut torch, request, 1, false);
        assert_eq!(torch.driver_data[0], 1);
        assert_eq!(torch.modifier_index[0], V_LIGHT);
        assert_eq!(torch.modifier_value[0], 20.min(20 * 10 / 1 / 2));
        assert_eq!(torch.sprite, -1);
        assert!(torch.flags.contains(ItemFlags::NODECAY));
        assert!(character.flags.contains(CharacterFlags::ITEMS));

        execute_item_driver(&mut character, &mut torch, request, 1, false);
        assert_eq!(torch.driver_data[0], 0);
        assert_eq!(torch.modifier_value[0], 0);
        assert_eq!(torch.sprite, 0);
        assert!(!torch.flags.contains(ItemFlags::NODECAY));
    }

    #[test]
    fn torch_user_use_extracts_non_light_modifier_before_toggling() {
        let mut character = character(1);
        let mut torch = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_TORCH);
        torch.carried_by = Some(CharacterId(1));
        torch.driver_data = vec![0, 0, 10, 20];
        torch.modifier_index[1] = CharacterValue::Speed as i16;
        torch.modifier_value[1] = 2;
        let request = ItemDriverRequest::Driver {
            driver: IDR_TORCH,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut torch, request, 1, false),
            ItemDriverOutcome::TorchExtractOrb {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                modifier_slot: 1,
                modifier: CharacterValue::Speed as i16,
            }
        );
        assert_eq!(torch.driver_data[0], 0);
        assert_eq!(torch.modifier_value[1], 2);
    }

    #[test]
    fn torch_timer_burns_down_marks_special_and_expires() {
        let mut character = character(1);
        let mut torch = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_TORCH);
        torch.carried_by = Some(CharacterId(1));
        torch.driver_data = vec![1, 1, 2, 20];
        torch.modifier_index[1] = CharacterValue::Speed as i16;
        torch.modifier_value[1] = 1;
        let request = ItemDriverRequest::Driver {
            driver: IDR_TORCH,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };
        let context = ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut torch,
                request,
                1,
                false,
                &context
            ),
            ItemDriverOutcome::LightChanged {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                schedule_after_ticks: Some(LIGHT_TIMER_TICKS),
            }
        );
        assert_eq!(torch.min_level, 200);
        assert_eq!(torch.driver_data[1], 2);
        assert_eq!(torch.modifier_value[0], 20.min(20 * 2 / 3 / 2));

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut torch,
                request,
                1,
                false,
                &context
            ),
            ItemDriverOutcome::TorchExpired {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                item_name: outcome_item_name("Item"),
            }
        );
    }

    #[test]
    fn edemon_loader_accepts_yellow_crystal_and_starts_animation() {
        let mut character = character(1);
        character.cursor_item = Some(ItemId(9));
        let mut loader = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_EDEMONLOADER);
        loader.driver_data = vec![2, 0, 0];
        let request = ItemDriverRequest::Driver {
            driver: IDR_EDEMONLOADER,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        let outcome = execute_item_driver_with_context(
            &mut character,
            &mut loader,
            request,
            6,
            false,
            &ItemDriverContext {
                cursor_template_id: Some(IID_AREA6_YELLOWCRYSTAL),
                cursor_drdata0: Some(86),
                ..ItemDriverContext::default()
            },
        );

        assert_eq!(character.cursor_item, None);
        assert!(character.flags.contains(CharacterFlags::ITEMS));
        assert_eq!(loader.driver_data, vec![2, 86, 7]);
        assert_eq!(loader.sprite, 14260);
        assert_eq!(
            outcome,
            ItemDriverOutcome::EdemonLoaderChanged {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                consumed_cursor_item_id: Some(ItemId(9)),
                ground_overlay_sprite: 14240,
                sound_type: Some(41),
                schedule_after_ticks: None,
            }
        );
    }

    #[test]
    fn edemon_loader_timer_decays_power_animation_and_schedules() {
        let mut timer_character = character(0);
        let mut loader = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_EDEMONLOADER);
        loader.sprite = 14235;
        loader.driver_data = vec![2, 1, 1];
        let request = ItemDriverRequest::Driver {
            driver: IDR_EDEMONLOADER,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut timer_character,
                &mut loader,
                request,
                6,
                false,
                &ItemDriverContext {
                    timer_call: true,
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::EdemonLoaderChanged {
                item_id: ItemId(7),
                character_id: CharacterId(0),
                consumed_cursor_item_id: None,
                ground_overlay_sprite: 14240,
                sound_type: Some(43),
                schedule_after_ticks: Some(TICKS_PER_SECOND),
            }
        );
        assert_eq!(loader.driver_data, vec![2, 0, 0]);
        assert_eq!(loader.sprite, 14234);
    }

    #[test]
    fn edemon_loader_blocks_missing_wrong_and_stuck_crystals() {
        let mut character = character(1);
        let mut loader = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_EDEMONLOADER);
        loader.driver_data = vec![2, 0, 0];
        let request = ItemDriverRequest::Driver {
            driver: IDR_EDEMONLOADER,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut loader, request, 6, false),
            ItemDriverOutcome::EdemonLoaderBlocked {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                reason: EdemonLoaderBlockReason::NeedsCrystal,
            }
        );

        character.cursor_item = Some(ItemId(9));
        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut loader,
                request,
                6,
                false,
                &ItemDriverContext {
                    cursor_template_id: Some(123),
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::EdemonLoaderBlocked {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                reason: EdemonLoaderBlockReason::WrongCrystal,
            }
        );

        loader.driver_data[1] = 4;
        assert_eq!(
            execute_item_driver(&mut character, &mut loader, request, 6, false),
            ItemDriverOutcome::EdemonLoaderBlocked {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                reason: EdemonLoaderBlockReason::CrystalAlreadyPresent,
            }
        );
        character.cursor_item = None;
        assert_eq!(
            execute_item_driver(&mut character, &mut loader, request, 6, false),
            ItemDriverOutcome::EdemonLoaderBlocked {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                reason: EdemonLoaderBlockReason::CrystalStuck,
            }
        );
    }

    #[test]
    fn fdemon_loader_accepts_red_crystal_and_starts_animation() {
        let mut character = character(1);
        character.cursor_item = Some(ItemId(9));
        let mut loader = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FDEMONLOADER);
        let request = ItemDriverRequest::Driver {
            driver: IDR_FDEMONLOADER,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        let outcome = execute_item_driver_with_context(
            &mut character,
            &mut loader,
            request,
            8,
            false,
            &ItemDriverContext {
                cursor_template_id: Some(IID_AREA8_REDCRYSTAL),
                cursor_drdata0: Some(12),
                ..ItemDriverContext::default()
            },
        );

        assert_eq!(character.cursor_item, None);
        assert!(character.flags.contains(CharacterFlags::ITEMS));
        assert_eq!(drdata_u16(&loader, 1), 0);
        assert_eq!(loader.driver_data[3], 7);
        assert_eq!(drdata_u16(&loader, 4), 1200);
        assert_eq!(loader.sprite, 59036);
        assert_eq!(
            outcome,
            ItemDriverOutcome::FdemonLoaderChanged {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                consumed_cursor_item_id: Some(ItemId(9)),
                ground_overlay_sprite: 59021,
                sound_type: Some(41),
                schedule_after_ticks: None,
            }
        );
    }

    #[test]
    fn fdemon_loader_timer_counts_animation_and_power() {
        let mut timer_character = character(0);
        let mut loader = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FDEMONLOADER);
        loader.driver_data = vec![1, 0, 0, 1, 2, 0, 0];
        loader.sprite = 59030;
        let request = ItemDriverRequest::Driver {
            driver: IDR_FDEMONLOADER,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        };

        let outcome = execute_item_driver_with_context(
            &mut timer_character,
            &mut loader,
            request,
            8,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        );

        assert_eq!(drdata_u16(&loader, 1), 1);
        assert_eq!(loader.driver_data[3], 0);
        assert_eq!(drdata_u16(&loader, 4), 1);
        assert_eq!(loader.sprite, 59039);
        assert_eq!(
            outcome,
            ItemDriverOutcome::FdemonLoaderChanged {
                item_id: ItemId(7),
                character_id: CharacterId(0),
                consumed_cursor_item_id: None,
                ground_overlay_sprite: 59029,
                sound_type: None,
                schedule_after_ticks: Some(TICKS_PER_SECOND),
            }
        );
    }

    #[test]
    fn fdemon_loader_blocks_wrong_or_missing_crystal() {
        let mut character = character(1);
        let mut loader = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FDEMONLOADER);
        let request = ItemDriverRequest::Driver {
            driver: IDR_FDEMONLOADER,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut loader, request, 8, false),
            ItemDriverOutcome::FdemonLoaderBlocked {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                reason: FdemonLoaderBlockReason::NeedsCrystal,
            }
        );

        character.cursor_item = Some(ItemId(9));
        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut loader,
                request,
                8,
                false,
                &ItemDriverContext {
                    cursor_template_id: Some(123),
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::FdemonLoaderBlocked {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                reason: FdemonLoaderBlockReason::WrongCrystal,
            }
        );
    }

    #[test]
    fn orbspawn_driver_returns_typed_spawn_for_paid_eligible_character() {
        let mut character = character(1);
        character.flags.insert(CharacterFlags::PAID);
        character.level = 10;
        let mut spawner = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_ORBSPAWN);
        spawner.min_level = 5;
        let request = ItemDriverRequest::Driver {
            driver: IDR_ORBSPAWN,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut spawner, request, 1, false),
            ItemDriverOutcome::OrbSpawn {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                anti: false,
                special: false,
            }
        );
    }

    #[test]
    fn anti_orbspawn_driver_blocks_unpaid_and_marks_special() {
        let mut character = character(1);
        let mut spawner = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_ANTIORBSPAWN);
        spawner.driver_data = vec![1];
        let request = ItemDriverRequest::Driver {
            driver: IDR_ANTIORBSPAWN,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut spawner, request, 1, false),
            ItemDriverOutcome::BlockedByRequirements {
                item_id: ItemId(7),
                character_id: CharacterId(1),
            }
        );

        character.flags.insert(CharacterFlags::PAID);
        assert_eq!(
            execute_item_driver(&mut character, &mut spawner, request, 1, false),
            ItemDriverOutcome::OrbSpawn {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                anti: true,
                special: true,
            }
        );
    }

    #[test]
    fn balltrap_non_player_launches_projectile_from_driver_data() {
        let mut character = character(1);
        let mut trap = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_BALLTRAP);
        trap.x = 100;
        trap.y = 100;
        trap.driver_data = vec![131, 126, 42];

        let outcome = execute_item_driver(
            &mut character,
            &mut trap,
            ItemDriverRequest::Driver {
                driver: IDR_BALLTRAP,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::BallTrapProjectile {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                start_x: 101,
                start_y: 99,
                target_x: 103,
                target_y: 98,
                power: 42,
            }
        );
    }

    #[test]
    fn balltrap_ignores_timer_and_player_triggers() {
        let mut trap = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_BALLTRAP);
        trap.x = 100;
        trap.y = 100;
        trap.driver_data = vec![131, 126, 42];
        let request = ItemDriverRequest::Driver {
            driver: IDR_BALLTRAP,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        let mut timer_character = character(0);
        assert_eq!(
            execute_item_driver(&mut timer_character, &mut trap, request, 1, false),
            ItemDriverOutcome::Noop
        );

        let mut player = character(1);
        player.flags.insert(CharacterFlags::PLAYER);
        assert_eq!(
            execute_item_driver(&mut player, &mut trap, request, 1, false),
            ItemDriverOutcome::Noop
        );
    }

    #[test]
    fn fireball_machine_decodes_projectile_and_timer_reschedule() {
        let mut timer_character = character(0);
        let mut machine = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FIREBALL);
        machine.x = 100;
        machine.y = 100;
        machine.driver_data = vec![131, 126, 42, 9];

        let outcome = execute_item_driver_with_context(
            &mut timer_character,
            &mut machine,
            ItemDriverRequest::Driver {
                driver: IDR_FIREBALL,
                item_id: ItemId(7),
                character_id: CharacterId(0),
                spec: 0,
            },
            2,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::FireballMachineProjectile {
                item_id: ItemId(7),
                character_id: CharacterId(0),
                start_x: 101,
                start_y: 99,
                target_x: 103,
                target_y: 98,
                power: 42,
                schedule_after_ticks: Some(9),
            }
        );

        let mut player = character(1);
        let outcome = execute_item_driver(
            &mut player,
            &mut machine,
            ItemDriverRequest::Driver {
                driver: IDR_FIREBALL,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            2,
            false,
        );
        assert!(matches!(
            outcome,
            ItemDriverOutcome::FireballMachineProjectile {
                schedule_after_ticks: None,
                ..
            }
        ));
    }

    #[test]
    fn flamethrower_timer_pulses_light_and_reschedules() {
        let mut character = character(0);
        let mut trap = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLAMETHROW);
        trap.driver_data = vec![2, 3, 0, 5];

        let outcome = execute_item_driver_with_context(
            &mut character,
            &mut trap,
            ItemDriverRequest::Driver {
                driver: IDR_FLAMETHROW,
                item_id: ItemId(7),
                character_id: CharacterId(0),
                spec: 0,
            },
            1,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        );

        assert_eq!(trap.driver_data[0], 1);
        assert_eq!(trap.driver_data[2], 1);
        assert_eq!(trap.sprite, 1);
        assert_eq!(trap.modifier_index[4], V_LIGHT);
        assert_eq!(trap.modifier_value[4], 250);
        assert_eq!(
            outcome,
            ItemDriverOutcome::FlameThrowerPulse {
                item_id: ItemId(7),
                character_id: CharacterId(0),
                direction: 3,
                schedule_after_ticks: 1,
            }
        );
    }

    #[test]
    fn flamethrower_timer_extinguishes_and_uses_interval() {
        let mut character = character(0);
        let mut trap = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FLAMETHROW);
        trap.sprite = 10;
        trap.modifier_index[4] = V_LIGHT;
        trap.modifier_value[4] = 250;
        trap.driver_data = vec![0, 3, 1, 5];

        let outcome = execute_item_driver_with_context(
            &mut character,
            &mut trap,
            ItemDriverRequest::Driver {
                driver: IDR_FLAMETHROW,
                item_id: ItemId(7),
                character_id: CharacterId(0),
                spec: 0,
            },
            1,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        );

        assert_eq!(trap.sprite, 9);
        assert_eq!(&trap.driver_data[..3], &[TICKS_PER_SECOND as u8, 3, 0]);
        assert_eq!(trap.modifier_index[4], 0);
        assert_eq!(trap.modifier_value[4], 0);
        assert_eq!(
            outcome,
            ItemDriverOutcome::FlameThrowerExtinguished {
                item_id: ItemId(7),
                character_id: CharacterId(0),
                schedule_after_ticks: Some(TICKS_PER_SECOND * 5),
            }
        );
    }

    #[test]
    fn caligar_flame_uses_legacy_flamethrower_timer_path() {
        let mut timer_character = character(0);
        let mut flame = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_CALIGARFLAME);
        flame.driver_data = vec![2, 5, 0, 4];

        let outcome = execute_item_driver_with_context(
            &mut timer_character,
            &mut flame,
            ItemDriverRequest::Driver {
                driver: IDR_CALIGARFLAME,
                item_id: ItemId(7),
                character_id: CharacterId(0),
                spec: 0,
            },
            36,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        );

        assert_eq!(flame.driver_data[0], 1);
        assert_eq!(flame.driver_data[2], 1);
        assert_eq!(flame.sprite, 1);
        assert_eq!(flame.modifier_index[4], V_LIGHT);
        assert_eq!(flame.modifier_value[4], 250);
        assert_eq!(
            outcome,
            ItemDriverOutcome::FlameThrowerPulse {
                item_id: ItemId(7),
                character_id: CharacterId(0),
                direction: 5,
                schedule_after_ticks: 1,
            }
        );

        let mut player = character(1);
        assert_eq!(
            execute_item_driver_with_context(
                &mut player,
                &mut flame,
                ItemDriverRequest::Driver {
                    driver: IDR_CALIGARFLAME,
                    item_id: ItemId(7),
                    character_id: CharacterId(1),
                    spec: 0,
                },
                36,
                false,
                &ItemDriverContext::default(),
            ),
            ItemDriverOutcome::Noop
        );
    }

    #[test]
    fn spiketrap_triggers_once_and_timer_resets() {
        let mut actor = character(1);
        let mut trap = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPIKETRAP);
        trap.driver_data = vec![0, 4];

        let outcome = execute_item_driver(
            &mut actor,
            &mut trap,
            ItemDriverRequest::Driver {
                driver: IDR_SPIKETRAP,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        );
        assert_eq!(trap.sprite, 1);
        assert_eq!(trap.driver_data[0], 1);
        assert_eq!(
            outcome,
            ItemDriverOutcome::SpikeTrapTriggered {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                damage: 4 * crate::entity::POWERSCALE,
                reset_after_ticks: TICKS_PER_SECOND,
            }
        );

        let mut timer_character = character(0);
        let outcome = execute_item_driver_with_context(
            &mut timer_character,
            &mut trap,
            ItemDriverRequest::Driver {
                driver: IDR_SPIKETRAP,
                item_id: ItemId(7),
                character_id: CharacterId(0),
                spec: 0,
            },
            1,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        );
        assert_eq!(trap.sprite, 0);
        assert_eq!(trap.driver_data[0], 0);
        assert_eq!(
            outcome,
            ItemDriverOutcome::SpikeTrapReset { item_id: ItemId(7) }
        );
    }

    #[test]
    fn usetrap_schedules_target_item_with_using_character() {
        let mut character = character(1);
        let mut trap = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_USETRAP);
        trap.driver_data = vec![20, 30];

        let outcome = execute_item_driver(
            &mut character,
            &mut trap,
            ItemDriverRequest::Driver {
                driver: IDR_USETRAP,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        );

        assert_eq!(
            outcome,
            ItemDriverOutcome::TriggerMapItem {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                x: 20,
                y: 30,
                target_character_id: CharacterId(1),
                delay_ticks: TICKS_PER_SECOND / 2,
            }
        );
    }

    #[test]
    fn steptrap_timer_discovers_target_and_character_trigger_calls_target_without_character() {
        let mut timer_character = character(0);
        let mut trap = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_STEPTRAP);

        let outcome = execute_item_driver_with_context(
            &mut timer_character,
            &mut trap,
            ItemDriverRequest::Driver {
                driver: IDR_STEPTRAP,
                item_id: ItemId(7),
                character_id: CharacterId(0),
                spec: 0,
            },
            1,
            false,
            &ItemDriverContext {
                timer_call: true,
                ..ItemDriverContext::default()
            },
        );
        assert_eq!(
            outcome,
            ItemDriverOutcome::StepTrapDiscoverTarget { item_id: ItemId(7) }
        );

        let mut character = character(1);
        trap.driver_data = vec![20, 30];
        let outcome = execute_item_driver(
            &mut character,
            &mut trap,
            ItemDriverRequest::Driver {
                driver: IDR_STEPTRAP,
                item_id: ItemId(7),
                character_id: CharacterId(1),
                spec: 0,
            },
            1,
            false,
        );
        assert_eq!(
            outcome,
            ItemDriverOutcome::TriggerMapItem {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                x: 20,
                y: 30,
                target_character_id: CharacterId(0),
                delay_ticks: 1,
            }
        );
    }

    #[test]
    fn special_potion_fun_drinks_mutate_resources_and_consume_item() {
        let mut character = character(3);
        character.level = 10;
        character.hp = 15 * POWERSCALE;
        character.mana = 12 * POWERSCALE;
        character.endurance = 11 * POWERSCALE;
        character.values[0][CharacterValue::Hp as usize] = 20;
        character.inventory[30] = Some(ItemId(7));
        let mut potion = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPECIAL_POTION);
        potion.carried_by = Some(character.id);
        potion.driver_data = vec![8];

        let outcome = execute_item_driver_with_context(
            &mut character,
            &mut potion,
            ItemDriverRequest::Driver {
                driver: IDR_SPECIAL_POTION,
                item_id: ItemId(7),
                character_id: CharacterId(3),
                spec: 0,
            },
            1,
            false,
            &ItemDriverContext {
                current_tick: 12_345,
                ..ItemDriverContext::default()
            },
        );

        assert_eq!(character.hp, 5 * POWERSCALE);
        assert_eq!(character.mana, 2 * POWERSCALE);
        assert_eq!(character.endurance, POWERSCALE);
        assert_eq!(character.regen_ticker, 12_345);
        assert_eq!(character.inventory[30], None);
        assert!(!potion.flags.contains(ItemFlags::USED));
        assert_eq!(
            outcome,
            ItemDriverOutcome::SpecialPotionDrunk {
                item_id: ItemId(7),
                character_id: CharacterId(3),
                kind: 8,
                hp_delta: -10 * POWERSCALE,
                mana_delta: -10 * POWERSCALE,
                endurance_delta: -10 * POWERSCALE,
            }
        );
    }

    #[test]
    fn special_potion_healing_caps_at_max_hp_and_area_blocks() {
        let mut character = character(3);
        character.level = 10;
        character.hp = 18 * POWERSCALE;
        character.values[0][CharacterValue::Hp as usize] = 20;
        character.inventory[30] = Some(ItemId(7));
        let mut potion = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPECIAL_POTION);
        potion.carried_by = Some(character.id);
        potion.driver_data = vec![14];
        let request = ItemDriverRequest::Driver {
            driver: IDR_SPECIAL_POTION,
            item_id: ItemId(7),
            character_id: CharacterId(3),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut potion, request, 1, false),
            ItemDriverOutcome::SpecialPotionDrunk {
                item_id: ItemId(7),
                character_id: CharacterId(3),
                kind: 14,
                hp_delta: 2 * POWERSCALE,
                mana_delta: 0,
                endurance_delta: 0,
            }
        );
        assert_eq!(character.hp, 20 * POWERSCALE);

        let mut blocked = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPECIAL_POTION);
        blocked.carried_by = Some(character.id);
        blocked.driver_data = vec![14];
        assert!(matches!(
            execute_item_driver(
                &mut character,
                &mut blocked,
                ItemDriverRequest::Driver {
                    driver: IDR_SPECIAL_POTION,
                    item_id: ItemId(8),
                    character_id: CharacterId(3),
                    spec: 0,
                },
                34,
                true,
            ),
            ItemDriverOutcome::BlockedByArea { .. }
        ));
    }

    #[test]
    fn special_potion_security_increments_saves_and_consumes_item() {
        let mut character = character(3);
        character.level = 10;
        character.saves = 9;
        character.inventory[30] = Some(ItemId(7));
        let mut potion = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPECIAL_POTION);
        potion.carried_by = Some(character.id);
        potion.driver_data = vec![5];

        let outcome = execute_item_driver(
            &mut character,
            &mut potion,
            ItemDriverRequest::Driver {
                driver: IDR_SPECIAL_POTION,
                item_id: ItemId(7),
                character_id: CharacterId(3),
                spec: 0,
            },
            1,
            false,
        );

        assert_eq!(character.saves, 10);
        assert_eq!(character.inventory[30], None);
        assert!(!potion.flags.contains(ItemFlags::USED));
        assert_eq!(
            outcome,
            ItemDriverOutcome::SpecialPotionSecurity {
                item_id: ItemId(7),
                character_id: CharacterId(3),
                used: true,
            }
        );
    }

    #[test]
    fn special_potion_security_blocks_hardcore_or_capped_saves() {
        let request = ItemDriverRequest::Driver {
            driver: IDR_SPECIAL_POTION,
            item_id: ItemId(7),
            character_id: CharacterId(3),
            spec: 0,
        };

        let mut capped = character(3);
        capped.level = 10;
        capped.saves = 10;
        capped.inventory[30] = Some(ItemId(7));
        let mut capped_potion = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPECIAL_POTION);
        capped_potion.carried_by = Some(capped.id);
        capped_potion.driver_data = vec![5];
        let capped_outcome =
            execute_item_driver(&mut capped, &mut capped_potion, request, 1, false);

        assert_eq!(capped.saves, 10);
        assert_eq!(capped.inventory[30], Some(ItemId(7)));
        assert_eq!(
            capped_outcome,
            ItemDriverOutcome::SpecialPotionSecurity {
                item_id: ItemId(7),
                character_id: CharacterId(3),
                used: false,
            }
        );

        let mut hardcore = character(3);
        hardcore.level = 10;
        hardcore.flags.insert(CharacterFlags::HARDCORE);
        hardcore.inventory[30] = Some(ItemId(7));
        let mut hardcore_potion = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPECIAL_POTION);
        hardcore_potion.carried_by = Some(hardcore.id);
        hardcore_potion.driver_data = vec![5];
        let hardcore_outcome =
            execute_item_driver(&mut hardcore, &mut hardcore_potion, request, 1, false);

        assert_eq!(hardcore.saves, 0);
        assert_eq!(hardcore.inventory[30], Some(ItemId(7)));
        assert_eq!(
            hardcore_outcome,
            ItemDriverOutcome::SpecialPotionSecurity {
                item_id: ItemId(7),
                character_id: CharacterId(3),
                used: false,
            }
        );
    }

    #[test]
    fn special_potion_unknown_kind_reports_legacy_bug_without_consuming() {
        let mut character = character(3);
        character.level = 10;
        character.inventory[30] = Some(ItemId(7));
        let mut potion = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPECIAL_POTION);
        potion.carried_by = Some(character.id);
        potion.driver_data = vec![99];

        let outcome = execute_item_driver(
            &mut character,
            &mut potion,
            ItemDriverRequest::Driver {
                driver: IDR_SPECIAL_POTION,
                item_id: ItemId(7),
                character_id: CharacterId(3),
                spec: 0,
            },
            1,
            false,
        );

        assert_eq!(character.inventory[30], Some(ItemId(7)));
        assert!(potion.flags.contains(ItemFlags::USED));
        assert_eq!(
            outcome,
            ItemDriverOutcome::SpecialPotionBug {
                item_id: ItemId(7),
                character_id: CharacterId(3),
            }
        );
    }

    #[test]
    fn beyond_potion_dispatch_copies_modifiers_and_duration() {
        let mut character = character(3);
        character.level = 12;
        character.flags.insert(CharacterFlags::WARRIOR);
        let mut potion = item(
            7,
            ItemFlags::USED | ItemFlags::USE | ItemFlags::BEYONDMAXMOD,
            0,
            IDR_BEYONDPOTION,
        );
        potion.carried_by = Some(character.id);
        potion.min_level = 10;
        potion.driver_data = vec![15];
        potion.modifier_index = [
            CharacterValue::Strength as i16,
            CharacterValue::Agility as i16,
            0,
            0,
            0,
        ];
        potion.modifier_value = [3, 4, 0, 0, 0];

        assert_eq!(
            execute_item_driver(
                &mut character,
                &mut potion,
                ItemDriverRequest::Driver {
                    driver: IDR_BEYONDPOTION,
                    item_id: ItemId(7),
                    character_id: CharacterId(3),
                    spec: 0,
                },
                1,
                false,
            ),
            ItemDriverOutcome::BeyondPotion {
                item_id: ItemId(7),
                character_id: CharacterId(3),
                duration_minutes: 15,
                modifier_index: [
                    CharacterValue::Strength as i16,
                    CharacterValue::Agility as i16,
                    0,
                    0,
                    0,
                ],
                modifier_value: [3, 4, 0, 0, 0],
                beyond_max_mod: true,
            }
        );
    }

    #[test]
    fn beyond_potion_blocks_failed_requirements_and_teufelheim_arena() {
        let mut character = character(3);
        character.level = 9;
        let mut potion = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_BEYONDPOTION);
        potion.carried_by = Some(character.id);
        potion.min_level = 10;
        let request = ItemDriverRequest::Driver {
            driver: IDR_BEYONDPOTION,
            item_id: ItemId(7),
            character_id: CharacterId(3),
            spec: 0,
        };

        assert!(matches!(
            execute_item_driver(&mut character, &mut potion, request, 1, false),
            ItemDriverOutcome::BlockedByRequirements { .. }
        ));

        character.level = 10;
        assert!(matches!(
            execute_item_driver(&mut character, &mut potion, request, 34, true),
            ItemDriverOutcome::BlockedByArea { .. }
        ));
    }

    #[test]
    fn special_shrine_dispatches_hc_to_sc_kind() {
        let mut character = character(3);
        let mut shrine = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SPECIAL_SHRINE);
        shrine.driver_data = vec![0x0A];

        assert_eq!(
            execute_item_driver(
                &mut character,
                &mut shrine,
                ItemDriverRequest::Driver {
                    driver: IDR_SPECIAL_SHRINE,
                    item_id: ItemId(7),
                    character_id: CharacterId(3),
                    spec: 0,
                },
                1,
                false,
            ),
            ItemDriverOutcome::SpecialShrine {
                item_id: ItemId(7),
                character_id: CharacterId(3),
                kind: 0x0A,
            }
        );
    }

    #[test]
    fn demonshrine_dispatches_location_and_level_gate() {
        let mut character = character(3);
        character.level = 9;
        let mut shrine = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DEMONSHRINE);
        shrine.min_level = 10;
        shrine.x = 12;
        shrine.y = 34;

        let request = ItemDriverRequest::Driver {
            driver: IDR_DEMONSHRINE,
            item_id: ItemId(7),
            character_id: CharacterId(3),
            spec: 0,
        };
        assert_eq!(
            execute_item_driver(&mut character, &mut shrine, request, 5, false),
            ItemDriverOutcome::BlockedByRequirements {
                item_id: ItemId(7),
                character_id: CharacterId(3),
            }
        );

        character.level = 10;
        assert_eq!(
            execute_item_driver(&mut character, &mut shrine, request, 5, false),
            ItemDriverOutcome::DemonShrine {
                item_id: ItemId(7),
                character_id: CharacterId(3),
                location_id: 12 + (34 << 8) + (5 << 16),
            }
        );
    }

    #[test]
    fn labexit_timer_animates_reschedules_and_expires() {
        let mut timer_character = character(0);
        let mut gate = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LABEXIT);
        set_drdata_u32(&mut gate, 8, 23);
        let timer_context = ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        };
        let request = ItemDriverRequest::Driver {
            driver: IDR_LABEXIT,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut timer_character,
                &mut gate,
                request,
                2,
                false,
                &timer_context,
            ),
            ItemDriverOutcome::LabExitAnimating {
                item_id: ItemId(7),
                sprite: 1083,
                frame: 24,
                schedule_after_ticks: 2,
            }
        );
        assert_eq!(drdata_u32(&gate, 8), 24);

        set_drdata_u32(&mut gate, 8, 264);
        assert_eq!(
            execute_item_driver_with_context(
                &mut timer_character,
                &mut gate,
                request,
                2,
                false,
                &timer_context,
            ),
            ItemDriverOutcome::LabExitExpired { item_id: ItemId(7) }
        );
    }

    #[test]
    fn labexit_use_requires_owner_and_returns_area_exit() {
        let mut character = character(42);
        let mut gate = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_LABEXIT);
        set_drdata_u32(&mut gate, 0, 41);
        set_drdata(&mut gate, 4, 9);
        set_drdata_u32(&mut gate, 8, 35);
        let request = ItemDriverRequest::Driver {
            driver: IDR_LABEXIT,
            item_id: ItemId(7),
            character_id: CharacterId(42),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut gate, request, 2, false),
            ItemDriverOutcome::LabExitWrongOwner {
                item_id: ItemId(7),
                character_id: CharacterId(42),
            }
        );

        set_drdata_u32(&mut gate, 0, 42);
        assert_eq!(
            execute_item_driver(&mut character, &mut gate, request, 2, false),
            ItemDriverOutcome::LabExitUse {
                item_id: ItemId(7),
                character_id: CharacterId(42),
                lab_nr: 9,
                frame: 227,
                target_area: 3,
                target_x: 183,
                target_y: 199,
            }
        );
        assert_eq!(drdata_u32(&gate, 8), 227);
    }

    #[test]
    fn forest_spade_classifies_note_collapse_and_treasure_locations() {
        let mut character = character(42);
        let mut spade = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FORESTSPADE);
        spade.carried_by = Some(CharacterId(42));
        let request = ItemDriverRequest::Driver {
            driver: IDR_FORESTSPADE,
            item_id: ItemId(7),
            character_id: CharacterId(42),
            spec: 0,
        };

        character.x = 205;
        character.y = 234;
        assert_eq!(
            execute_item_driver(&mut character, &mut spade, request, 16, false),
            ItemDriverOutcome::ForestSpadeFind {
                item_id: ItemId(7),
                character_id: CharacterId(42),
                find: ForestSpadeFind::ForestNote1,
            }
        );

        character.x = 93;
        character.y = 36;
        assert_eq!(
            execute_item_driver(&mut character, &mut spade, request, 1, false),
            ItemDriverOutcome::ForestSpadeCollapse {
                item_id: ItemId(7),
                character_id: CharacterId(42),
                x: 106,
                y: 211,
            }
        );

        character.x = 214;
        character.y = 136;
        assert_eq!(
            execute_item_driver(&mut character, &mut spade, request, 29, false),
            ItemDriverOutcome::ForestSpadeFind {
                item_id: ItemId(7),
                character_id: CharacterId(42),
                find: ForestSpadeFind::BranningtonTreasure { dig_index: 2 },
            }
        );
    }

    #[test]
    fn forest_spade_blocks_cursor_and_reports_empty_ground() {
        let mut character = character(42);
        character.cursor_item = Some(ItemId(9));
        let mut spade = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FORESTSPADE);
        spade.carried_by = Some(CharacterId(42));
        let request = ItemDriverRequest::Driver {
            driver: IDR_FORESTSPADE,
            item_id: ItemId(7),
            character_id: CharacterId(42),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut spade, request, 16, false),
            ItemDriverOutcome::ForestSpadeCursorOccupied {
                item_id: ItemId(7),
                character_id: CharacterId(42),
            }
        );

        character.cursor_item = None;
        assert_eq!(
            execute_item_driver(&mut character, &mut spade, request, 16, false),
            ItemDriverOutcome::ForestSpadeNothing {
                item_id: ItemId(7),
                character_id: CharacterId(42),
            }
        );
    }

    #[test]
    fn skelraise_dispatches_blood_bowl_and_dust_paths() {
        let mut character = character(42);
        character.flags.insert(CharacterFlags::PLAYER);
        let mut chair = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SKELRAISE);
        chair.driver_data = vec![2, 0, 0];
        let request = ItemDriverRequest::Driver {
            driver: IDR_SKELRAISE,
            item_id: ItemId(7),
            character_id: CharacterId(42),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut chair, request, 17, false),
            ItemDriverOutcome::SkelRaiseDust {
                item_id: ItemId(7),
                character_id: CharacterId(42),
            }
        );

        character.cursor_item = Some(ItemId(9));
        let outcome = execute_item_driver_with_context(
            &mut character,
            &mut chair,
            request,
            17,
            false,
            &ItemDriverContext {
                cursor_template_id: Some(IID_AREA17_BLOODBOWL),
                ..ItemDriverContext::default()
            },
        );
        assert_eq!(
            outcome,
            ItemDriverOutcome::SkelRaiseRaise {
                item_id: ItemId(7),
                character_id: CharacterId(42),
                cursor_item_id: ItemId(9),
                template: "raised_skeleton_green_key",
            }
        );
    }

    #[test]
    fn skelraise_active_chair_and_timer_paths_match_c_boundary() {
        let mut character = character(42);
        let mut chair = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_SKELRAISE);
        chair.driver_data = vec![1, 0, 1];
        let request = ItemDriverRequest::Driver {
            driver: IDR_SKELRAISE,
            item_id: ItemId(7),
            character_id: CharacterId(42),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver(&mut character, &mut chair, request, 17, false),
            ItemDriverOutcome::SkelRaiseTouch {
                item_id: ItemId(7),
                character_id: CharacterId(42),
            }
        );
        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut chair,
                request,
                17,
                false,
                &ItemDriverContext {
                    timer_call: true,
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::SkelRaiseTimer { item_id: ItemId(7) }
        );
    }

    #[test]
    fn fdemon_blood_blocks_bare_wrong_and_full_cursor_items() {
        let mut character = character(1);
        let mut blood = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FDEMONBLOOD);
        let request = ItemDriverRequest::Driver {
            driver: IDR_FDEMONBLOOD,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(IDR_FDEMONBLOOD, 50);
        assert_eq!(
            execute_item_driver(&mut character, &mut blood, request, 8, false),
            ItemDriverOutcome::FdemonBloodBlocked {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                reason: FdemonBloodBlockReason::BareHands,
            }
        );

        character.cursor_item = Some(ItemId(9));
        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut blood,
                request,
                8,
                false,
                &ItemDriverContext {
                    cursor_template_id: Some(0x0100_004A),
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::FdemonBloodBlocked {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                reason: FdemonBloodBlockReason::WrongItem,
            }
        );

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut blood,
                request,
                8,
                false,
                &ItemDriverContext {
                    cursor_template_id: Some(IID_AREA8_BLOOD),
                    cursor_drdata0: Some(3),
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::FdemonBloodBlocked {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                reason: FdemonBloodBlockReason::ContainerFull,
            }
        );
    }

    #[test]
    fn fdemon_blood_destroys_flasks_or_fills_blood_container() {
        let mut character = character(1);
        character.cursor_item = Some(ItemId(9));
        let mut blood = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_FDEMONBLOOD);
        let request = ItemDriverRequest::Driver {
            driver: IDR_FDEMONBLOOD,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        };

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut blood,
                request,
                8,
                false,
                &ItemDriverContext {
                    cursor_driver: Some(IDR_FLASK),
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::FdemonBloodDestroyedFlask {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                flask_item_id: ItemId(9),
            }
        );
        assert_eq!(character.cursor_item, None);
        assert!(character.flags.contains(CharacterFlags::ITEMS));
        assert_eq!(blood.sprite, 14348);

        character.cursor_item = Some(ItemId(10));
        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut blood,
                request,
                8,
                false,
                &ItemDriverContext {
                    cursor_template_id: Some(IID_AREA8_BLOOD),
                    cursor_drdata0: Some(2),
                    ..ItemDriverContext::default()
                },
            ),
            ItemDriverOutcome::FdemonBloodFilled {
                item_id: ItemId(7),
                character_id: CharacterId(1),
                container_item_id: ItemId(10),
                amount: 3,
            }
        );
        assert!(character.flags.contains(CharacterFlags::ITEMS));
    }

    fn request(character_id: u32, item_id: u32, spec: i32) -> ItemUseRequest {
        ItemUseRequest {
            character_id: CharacterId(character_id),
            item_id: ItemId(item_id),
            spec,
        }
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

    fn item(id: u32, flags: ItemFlags, content_id: u16, driver: u16) -> Item {
        Item {
            id: ItemId(id),
            name: "Item".into(),
            description: String::new(),
            flags,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [0; MAX_MODIFIERS],
            modifier_value: [0; MAX_MODIFIERS],
            x: 0,
            y: 0,
            carried_by: None,
            contained_in: None,
            content_id,
            driver,
            driver_data: Vec::new(),
            serial: 0,
        }
    }
}
