mod loader;
mod parse;

pub use parse::*;

#[cfg(test)]
mod tests;

use crate::{
    character_driver::{
        apply_lab2_herald_create_message, apply_lab2_undead_create_message,
        apply_lab4_gnalb_create_message, apply_lab4_seyan_create_message,
        apply_lab5_daemon_create_message, apply_lab5_mage_create_message,
        apply_lab5_seyan_create_message, apply_labgnome_create_message,
        apply_simple_baddy_create_message, parse_arena_manager_driver_args,
        parse_clanclerk_driver_args, parse_clanmaster_driver_args, parse_clubmaster_driver_args,
        parse_nop_driver_args, ArenaFighterDriverData, ArenaMasterDriverData, AristocratDriverData,
        Astro2DriverData, BrennethBranDriverData, BrithildieDriverData, BroklinDriverData,
        CamhermitDriverData, CarlosDriverData, CharacterDriverState, ClaraDriverData,
        CountBranDriverData, CountessaBranDriverData, DaughterBranDriverData,
        DungeonmasterDriverData, DwarfChiefDriverData, DwarfShamanDriverData, DwarfSmithDriverData,
        FightDriverData, ForestBranDriverData, ForestHermitDriverData, ForestImpDriverData,
        ForestRangerDriverData, ForestWilliamDriverData, GateFightDriverData,
        GateWelcomeDriverData, GolemKeyholdDriverData, GorwinDriverData, GreeterDriverData,
        GrinnichDriverData, GuardBranDriverData, GwendylonDriverData, JanitorDriverData,
        JazDriverData, JessicaDriverData, JiuDriverData, KassimDriverData, KellyDriverData,
        LostDwarfDriverData, MissionGiverDriverData, NookDriverData, RammyDriverData,
        ReskinDriverData, RouvenDriverData, SeymourDriverData, ShanraDriverData,
        SirJonesDriverData, SmuggleComDriverData, SpiritBranDriverData, SuperiorDriverData,
        SupermaxDriverData, TerionDriverData, TeufelGambleDriverData, TeufelQuestDriverData,
        ThomasDriverData, TraderDriverData, TwoAlchemistDriverData, TwoBarkeeperDriverData,
        TwoSanwynDriverData, TwoSkellyDriverData, TwoThiefGuardDriverData,
        TwoThiefMasterDriverData, YoakinDriverData, YoatinDriverData, ARENA_FIGHTER_REST_POS,
        CDR_ARENAFIGHTER, CDR_ARENAMANAGER, CDR_ARENAMASTER, CDR_ARISTOCRAT, CDR_ARKHATAPRISON,
        CDR_ASTRO2, CDR_BRENNETHBRAN, CDR_BRITHILDIE, CDR_BROKLIN, CDR_CALIGARGUARD2,
        CDR_CALIGARSKELLY, CDR_CAMHERMIT, CDR_CARLOS, CDR_CENTINEL, CDR_CLANCLERK, CDR_CLANMASTER,
        CDR_CLUBMASTER, CDR_COUNTBRAN, CDR_COUNTESSABRAN, CDR_DAUGHTERBRAN, CDR_DUNGEONMASTER,
        CDR_DWARFCHIEF, CDR_DWARFSHAMAN, CDR_DWARFSMITH, CDR_FORESTBRAN, CDR_FORESTHERMIT,
        CDR_FORESTIMP, CDR_FORESTMONSTER, CDR_FORESTWILLIAM, CDR_FOREST_RANGER, CDR_GATE_FIGHT,
        CDR_GATE_WELCOME, CDR_GOLEMKEYHOLDER, CDR_GREETER, CDR_GRINNICH, CDR_GUARDBRAN,
        CDR_GWENDYLON, CDR_JANITOR, CDR_JAZ, CDR_JESSICA, CDR_JIU, CDR_KASSIM, CDR_KELLY,
        CDR_LAB2HERALD, CDR_LAB2UNDEAD, CDR_LAB4GNALB, CDR_LAB4SEYAN, CDR_LAB5DAEMON, CDR_LAB5MAGE,
        CDR_LAB5SEYAN, CDR_LABGNOMEDRIVER, CDR_LOSTDWARF, CDR_MISSIONGIVE, CDR_NOOK, CDR_NOP,
        CDR_RAMMY, CDR_RESKIN, CDR_ROUVEN, CDR_SEYMOUR, CDR_SHANRA, CDR_SHR_WEREWOLF,
        CDR_SIMPLEBADDY, CDR_SIRJONES, CDR_SMUGGLECOM, CDR_SPIRITBRAN, CDR_SUPERIOR, CDR_SUPERMAX,
        CDR_SWAMPCLARA, CDR_TERION, CDR_TEUFELDEMON, CDR_TEUFELGAMBLER, CDR_TEUFELQUEST,
        CDR_TEUFELRAT, CDR_THOMAS, CDR_TRADER, CDR_TUNNELER_GORWIN, CDR_TWOALCHEMIST,
        CDR_TWOBARKEEPER, CDR_TWOGUARD, CDR_TWOSANWYN, CDR_TWOSERVANT, CDR_TWOSKELLY,
        CDR_TWOTHIEFGUARD, CDR_TWOTHIEFMASTER, CDR_WHITEROBBERBOSS, CDR_YOAKIN, CDR_YOATIN,
        NT_CREATE,
    },
    entity::{
        Character, CharacterFlags, Item, ItemFlags, CHARACTER_VALUE_COUNT, INVENTORY_SIZE,
        MAX_MODIFIERS, POWERSCALE, PROFESSION_COUNT,
    },
    ids::{CharacterId, ItemId},
    item_driver::IDR_LAB2_REGENERATE,
    legacy::{INVENTORY_START_INVENTORY, INVENTORY_START_SPELLS},
    map::{MapFlags, MapTile},
    world::World,
};
use std::collections::HashMap;
use thiserror::Error;

const LEGACY_DRIVER_DATA_SIZE: usize = 40;

const LEGACY_DIR_RIGHTDOWN: u8 = 2;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ZoneError {
    #[error("line {line}: {message}")]
    Syntax { line: usize, message: String },
    #[error("unknown item template `{0}`")]
    UnknownItem(String),
    #[error("unknown character template `{0}`")]
    UnknownCharacter(String),
    #[error("map coordinate ({x},{y}) is outside the current map")]
    MapOutOfBounds { x: usize, y: usize },
    #[error("character inventory is full")]
    InventoryFull,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneRecord {
    pub key: String,
    pub fields: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemTemplate {
    pub key: String,
    pub name: String,
    pub description: String,
    pub flags: ItemFlags,
    pub sprite: i32,
    pub value: u32,
    pub min_level: u8,
    pub max_level: u8,
    pub needs_class: u8,
    pub template_id: u32,
    pub modifier_index: [i16; MAX_MODIFIERS],
    pub modifier_value: [i16; MAX_MODIFIERS],
    pub driver: u16,
    pub driver_data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterTemplate {
    pub key: String,
    pub name: String,
    pub description: String,
    pub flags: CharacterFlags,
    pub sprite: i32,
    pub sound: i32,
    pub gold: u32,
    pub driver: u16,
    pub group: i32,
    pub class: i32,
    pub respawn_seconds: Option<u32>,
    pub base_values: Vec<i16>,
    pub professions: Vec<i16>,
    pub inventory: Vec<Option<String>>,
    pub args: String,
    pub level_override: Option<u32>,
    pub loot_table: String,
    pub loot_table_death: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapDirective {
    Origin { x: usize, y: usize },
    Field { x: usize, y: usize },
    From { x: usize, y: usize },
    To { x: usize, y: usize },
    GroundSprite(u32),
    ForegroundSprite(u32),
    Character(String),
    Item(String),
    Flag(MapFlags),
}

#[derive(Debug, Default)]
pub struct ZoneLoader {
    pub item_templates: HashMap<String, ItemTemplate>,
    pub character_templates: HashMap<String, CharacterTemplate>,
    next_item_id: u32,
    next_character_id: u32,
    next_serial: u32,
}
