use serde::{Deserialize, Serialize};

use crate::{
    do_action::ItemUseRequest,
    entity::{
        Character, CharacterFlags, CharacterValue, Item, ItemFlags, CHARACTER_VALUE_COUNT,
        MAX_MODIFIERS, POWERSCALE,
    },
    ids::{CharacterId, ItemId},
    item_ops::consume_item,
    legacy::action,
    text::{COL_DARK_GRAY, COL_RESET},
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
pub const IDR_FLAMETHROW: u16 = 24;
pub const IDR_STEPTRAP: u16 = 25;
pub const IDR_SPIKETRAP: u16 = 26;
pub const IDR_EXTINGUISH: u16 = 28;
pub const IDR_ASSEMBLE: u16 = 29;
pub const IDR_TELE_DOOR: u16 = 31;
pub const IDR_RANDCHEST: u16 = 34;
pub const IDR_DEMONSHRINE: u16 = 35;
pub const IDR_EDEMONBALL: u16 = 36;
pub const IDR_PALACEKEY: u16 = 59;
pub const IDR_FORESTSPADE: u16 = 77;
pub const IDR_SHRIKEAMULET: u16 = 118;
pub const IDR_MINEGATEWAYKEY: u16 = 126;
pub const IDR_INFINITE_CHEST: u16 = 93;
pub const IDR_FOOD: u16 = 64;
pub const IDR_ENCHANTITEM: u16 = 83;
pub const IDR_ORBSPAWN: u16 = 84;
pub const IDR_SPECIAL_POTION: u16 = 88;
pub const IDR_NOMADSTACK: u16 = 96;
pub const IDR_LABEXIT: u16 = 102;
pub const IDR_TOYLIGHT: u16 = 117;
pub const IDR_DECAYITEM: u16 = 132;
pub const IDR_OXYPOTION: u16 = 128;
pub const IDR_PICKBERRY: u16 = 129;
pub const IDR_LIZARDFLOWER: u16 = 130;
pub const IDR_BEYONDPOTION: u16 = 133;
pub const IDR_DEMONCHIP: u16 = 136;
pub const IDR_XMASTREE: u16 = 142;
pub const IDR_XMASMAKER: u16 = 143;
pub const IDR_SPECIAL_SHRINE: u16 = 147;
pub const IDR_ACCOUNT_DEPOT: u16 = 148;
pub const IDR_ANTIENCHANTITEM: u16 = 160;
pub const IDR_SPECIALANTIENCHANTITEM: u16 = 161;
pub const IDR_CITY_RECALL: u16 = 159;
pub const IDR_ANTIORBSPAWN: u16 = 162;
pub const IDR_DOUBLE_DOOR: u16 = 187;
pub const IDR_LAB3_PLANT: u16 = 193;
pub const IDR_KEY_RING: u16 = 200;
pub const IID_SKELETON_KEY: u32 = (59 << 24) | 0x000003;
pub const IID_AREA2_ZOMBIESKULL1: u32 = (0x01 << 24) | 0x000025;
pub const IID_AREA2_ZOMBIESKULL2: u32 = (0x01 << 24) | 0x000026;
pub const IID_AREA2_ZOMBIESKULL3: u32 = (0x01 << 24) | 0x000027;
pub const IID_AREA11_PALACEKEY: u32 = (0x01 << 24) | 0x000050;
pub const IID_AREA11_PALACEKEYPART: u32 = (0x01 << 24) | 0x000051;
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
    pub character_underwater: bool,
    pub current_tick: u32,
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
    BookText {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
        demon_value: i32,
    },
    AccountDepotOpened {
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
        | ItemDriverOutcome::DoubleDoorToggle { .. } => 1,
        ItemDriverOutcome::Noop if matches!(driver, Some(IDR_DOOR) | Some(IDR_DOUBLE_DOOR)) => 2,
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
            match driver {
                0 => ItemDriverOutcome::LookItem {
                    item_id,
                    character_id,
                },
                IDR_POTION => potion_driver(character, item, area_id, in_arena),
                IDR_DOOR => door_driver(character, item, context),
                IDR_BALLTRAP => balltrap_driver(character, item),
                IDR_FIREBALL => fireball_machine_driver(character, item, context),
                IDR_EDEMONBALL => edemonball_driver(character, item, context),
                IDR_FLAMETHROW => flamethrow_driver(character, item, context),
                IDR_USETRAP => usetrap_driver(character, item),
                IDR_STEPTRAP => steptrap_driver(character, item, context),
                IDR_PALACEGATE => palace_gate_driver(character, item, context),
                IDR_SPIKETRAP => spiketrap_driver(character, item, context),
                IDR_EXTINGUISH => extinguish_driver(character, item),
                IDR_CHEST => chest_driver(character, item),
                IDR_RANDCHEST => randchest_driver(character, item),
                IDR_FORESTSPADE => forest_spade_driver(character, item, area_id),
                IDR_SHRINE => zombie_shrine_driver(character, item, context),
                IDR_BOOK => book_driver(character, item),
                IDR_DEMONSHRINE => demonshrine_driver(character, item, area_id),
                IDR_PALACEKEY => palace_key_driver(character, item, context),
                IDR_INFINITE_CHEST => infinite_chest_driver(character, item, context),
                IDR_RECALL => recall_driver(character, item, area_id, in_arena),
                IDR_TRANSPORT => transport_driver(character, item, spec),
                IDR_STATSCROLL => stat_scroll_driver(character, item),
                IDR_ASSEMBLE => assemble_driver(character, item, context),
                IDR_CITY_RECALL => city_recall_driver(character, item, area_id, in_arena),
                IDR_DOUBLE_DOOR => double_door_driver(character, item),
                IDR_TELE_DOOR => teleport_door_driver(character, item),
                IDR_TELEPORT => teleport_driver(character, item),
                IDR_ONOFFLIGHT => onofflight_driver(character, item, context),
                IDR_NIGHTLIGHT => nightlight_driver(character, item, context),
                IDR_TORCH => torch_driver(character, item, context),
                IDR_FOOD => food_driver(character, item),
                IDR_ENCHANTITEM => enchant_driver(character, item),
                IDR_ANTIENCHANTITEM => anti_enchant_driver(character, item, false),
                IDR_SPECIALANTIENCHANTITEM => anti_enchant_driver(character, item, true),
                IDR_ORBSPAWN => orbspawn_driver(character, item, false),
                IDR_ANTIORBSPAWN => orbspawn_driver(character, item, true),
                IDR_SPECIAL_POTION => {
                    special_potion_driver(character, item, area_id, in_arena, context.current_tick)
                }
                IDR_SPECIAL_SHRINE => special_shrine_driver(character, item),
                IDR_NOMADSTACK => nomad_stack_driver(character, item),
                IDR_DEMONCHIP => nomad_stack_driver(character, item),
                IDR_SHRIKEAMULET => shrike_amulet_driver(character, item, context),
                IDR_MINEGATEWAYKEY => mine_gateway_key_driver(character, item, context),
                IDR_TOYLIGHT => toylight_driver(character, item, context),
                IDR_DECAYITEM => decaying_item_driver(character, item, context),
                IDR_OXYPOTION => oxy_potion_driver(character, item, area_id),
                IDR_PICKBERRY => pick_berry_driver(character, item, area_id),
                IDR_LIZARDFLOWER => lizard_flower_driver(character, item, context, area_id),
                IDR_LAB3_PLANT => lab3_plant_driver(character, item, context),
                IDR_LABEXIT => labexit_driver(character, item, context),
                IDR_BEYONDPOTION => beyond_potion_driver(character, item, area_id, in_arena),
                IDR_XMASTREE => xmastree_driver(character, item),
                IDR_XMASMAKER => xmasmaker_driver(character, item),
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
            ItemDriverOutcome::BlockedByArea {
                item_id: ItemId(8),
                character_id: CharacterId(1),
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
    fn lizard_flower_mixer_requires_cursor_flower_and_combines_bits() {
        let mut character = character(1);
        character.cursor_item = Some(ItemId(9));
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
                &mut character,
                &mut flower,
                request,
                30,
                false,
                &ItemDriverContext::default(),
            ),
            ItemDriverOutcome::BlockedByArea {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            }
        );

        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
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
            execute_item_driver_with_context(
                &mut character,
                &mut flower,
                request,
                31,
                false,
                &context,
            ),
            ItemDriverOutcome::LizardFlowerMixed {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                cursor_item_id: ItemId(9),
                combined_bits: 7,
                complete: true,
                bottle_message: true,
            }
        );

        character.cursor_item = None;
        assert_eq!(
            execute_item_driver_with_context(
                &mut character,
                &mut flower,
                request,
                31,
                false,
                &context,
            ),
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
