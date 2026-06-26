pub const VERSION: u32 = 0x030303;
pub const TICKS_PER_SECOND: u64 = 24;
pub const MAX_MAP: usize = 256;
pub const TOTAL_MAX_CHARS: usize = 4096;
pub const GROUP_ID_OFFSET: u32 = 0x10000;
pub const MAX_AREA_NAME: usize = 80;
pub const MAX_PASSWORD: usize = 16;
pub const MAX_EMAIL: usize = 80;
pub const POWER_SCALE: i32 = 1000;
pub const DIST_MAX: usize = 40;
pub const DIST_OLD: usize = 25;
pub const SAY_DIST: usize = 25;

pub const INVENTORY_SIZE: usize = 110;
pub const INVENTORY_START_WORN: usize = 0;
pub const INVENTORY_LAST_WORN: usize = 11;
pub const INVENTORY_START_SPELLS: usize = 12;
pub const INVENTORY_LAST_SPELLS: usize = 29;
pub const INVENTORY_START_INVENTORY: usize = 30;
pub const INVENTORY_LAST_INVENTORY: usize = INVENTORY_SIZE - 1;

pub const MAX_FIELD: usize = 512;

pub mod action {
    pub const IDLE: u16 = 0;
    pub const WALK: u16 = 1;
    pub const TAKE: u16 = 2;
    pub const DROP: u16 = 3;
    pub const ATTACK1: u16 = 4;
    pub const ATTACK2: u16 = 5;
    pub const ATTACK3: u16 = 6;
    pub const USE: u16 = 7;
    pub const FIREBALL1: u16 = 10;
    pub const FIREBALL2: u16 = 11;
    pub const BALL1: u16 = 12;
    pub const BALL2: u16 = 13;
    pub const MAGICSHIELD: u16 = 14;
    pub const FLASH: u16 = 15;
    pub const BLESS_SELF: u16 = 16;
    pub const BLESS1: u16 = 17;
    pub const BLESS2: u16 = 18;
    pub const HEAL_SELF: u16 = 19;
    pub const HEAL1: u16 = 20;
    pub const HEAL2: u16 = 21;
    pub const FREEZE: u16 = 22;
    pub const WARCRY: u16 = 23;
    pub const GIVE: u16 = 24;
    pub const EARTHRAIN: u16 = 25;
    pub const EARTHMUD: u16 = 26;
    pub const PULSE: u16 = 27;
    pub const FIRERING: u16 = 28;
    pub const DIE: u16 = 50;
}

pub mod profession {
    pub const ATHLETE: usize = 0;
    pub const ALCHEMIST: usize = 1;
    pub const MINER: usize = 2;
    pub const ASSASSIN: usize = 3;
    pub const THIEF: usize = 4;
    pub const LIGHT: usize = 5;
    pub const DARK: usize = 6;
    pub const TRADER: usize = 7;
    pub const MERCENARY: usize = 8;
    pub const CLAN: usize = 9;
    pub const HERBALIST: usize = 10;
    pub const DEMON: usize = 11;
    pub const MAX: usize = 20;
}

pub mod worn_slot {
    pub const NECK: usize = 0;
    pub const HEAD: usize = 1;
    pub const CLOAK: usize = 2;
    pub const ARMS: usize = 3;
    pub const BODY: usize = 4;
    pub const BELT: usize = 5;
    pub const RIGHT_HAND: usize = 6;
    pub const LEGS: usize = 7;
    pub const LEFT_HAND: usize = 8;
    pub const RIGHT_RING: usize = 9;
    pub const FEET: usize = 10;
    pub const LEFT_RING: usize = 11;
}
