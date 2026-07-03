//! Static character-driver registry boundary for legacy `ch_driver` dispatch.
//!
//! The C server dynamically probes module libraries. The Rust rewrite keeps the
//! same numeric compatibility at the registry edge while routing known drivers
//! to typed outcomes that can be filled in incrementally.

use crate::{
    entity::{Character, CharacterFlags, CharacterValue, Item, INVENTORY_SIZE, POWERSCALE},
    ids::{CharacterId, ItemId},
    item_driver::IDR_POTION,
};

pub const CDT_DRIVER: u16 = 0;
pub const CDT_ITEM: u16 = 1;
pub const CDT_DEAD: u16 = 2;
pub const CDT_RESPAWN: u16 = 3;
pub const CDT_SPECIAL: u16 = 4;

pub const CDR_LOSTCON: u16 = 5;
pub const CDR_MERCHANT: u16 = 6;
pub const CDR_SIMPLEBADDY: u16 = 7;
pub const CDR_MACRO: u16 = 37;
pub const CDR_SWAMPCLARA: u16 = 54;
pub const CDR_SWAMPMONSTER: u16 = 56;
pub const CDR_PALACEISLENA: u16 = 57;
pub const CDR_TWOSKELLY: u16 = 70;
pub const CDR_TRADER: u16 = 72;
pub const CDR_LQNPC: u16 = 74;
pub const CDR_JANITOR: u16 = 85;
pub const CDR_TEUFELDEMON: u16 = 114;
pub const CDR_TEUFELGAMBLER: u16 = 115;
pub const CDR_TEUFELQUEST: u16 = 116;
pub const CDR_TEUFELRAT: u16 = 117;
pub const CDR_CALIGARSKELLY: u16 = 124;
pub const CDR_LAB2UNDEAD: u16 = 198;

pub const DRD_SIMPLEBADDYDRIVER: u32 = 0x0100_0013;
pub const DRD_CLARADRIVER: u32 = 0x0100_0059;
pub const DRD_SKELLYDRIVER: u32 = 0x0100_006a;
pub const DRD_LAB2_UNDEAD: u32 = 0x0200_0001;

pub const NT_CHAR: i32 = 1;
pub const NT_ITEM: i32 = 2;
pub const NT_GOTHIT: i32 = 3;
pub const NT_DIDHIT: i32 = 4;
pub const NT_SEEHIT: i32 = 5;
pub const NT_DEAD: i32 = 6;
pub const NT_SPELL: i32 = 7;
pub const NT_GIVE: i32 = 8;
pub const NT_CREATE: i32 = 9;
pub const NT_TEXT: i32 = 200;
pub const NT_NPC: i32 = 300;

pub const NTID_MERCHANT: i32 = 1;
pub const NTID_TERION: i32 = 2;
pub const NTID_ASTURIN: i32 = 3;
pub const NTID_GATEKEEPER: i32 = 4;
pub const NTID_DIDSAY: i32 = 5;
pub const NTID_TUTORIAL: i32 = 6;
pub const NTID_PALACE_ALERT: i32 = 7;
pub const NTID_ARENA: i32 = 8;
pub const NTID_DUNGEON: i32 = 9;
pub const NTID_TWOCITY: i32 = 10;
pub const NTID_TWOCITY_PICK: i32 = 11;
pub const NTID_DICE: i32 = 12;
pub const NTID_LABGNOMETORCH: i32 = 13;
pub const NTID_LAB2_DEAMONCHECK: i32 = 14;
pub const NTID_SALTMINE_USEITEM: i32 = 15;
pub const NTID_GLADIATOR: i32 = 16;
pub const NTID_FDEMON: i32 = 17;

pub const FDEMON_MSG_WAYPOINT: i32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CharacterDriverMessage {
    pub message_type: i32,
    pub dat1: i32,
    pub dat2: i32,
    pub dat3: i32,
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CharacterDriverState {
    SimpleBaddy(SimpleBaddyDriverData),
    Clara(ClaraDriverData),
    TwoSkelly(TwoSkellyDriverData),
    Lab2Undead(Lab2UndeadDriverData),
    Merchant(MerchantDriverData),
    Lostcon(LostconDriverData),
}

/// C `struct lostcon_driver_data` (`src/module/lostcon.c`): the linger-timer
/// half of the `CDR_LOSTCON` driver. `deadline` is the absolute tick
/// (mirroring C's `dat->timeout = ticker + lagout_time`) at which the
/// character is saved and despawned if still unclaimed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LostconDriverData {
    pub deadline: u64,
}

/// C `struct merchant_driver_data` from `src/module/merchants/merchant.c`
/// plus the driver memory used for greeting throttling.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MerchantDriverData {
    pub dir: i32,
    pub dayx: i32,
    pub dayy: i32,
    pub daydir: i32,
    pub nightx: i32,
    pub nighty: i32,
    pub nightdir: i32,
    pub doorx: i32,
    pub doory: i32,
    pub storefx: i32,
    pub storefy: i32,
    pub storetx: i32,
    pub storety: i32,
    pub open: i32,
    pub close: i32,
    pub ignore: i32,
    pub special: i32,
    pub pricemulti: i32,
    /// Characters already greeted, mirroring C `mem_add_driver(cn, co, 7)`.
    #[serde(default)]
    pub greeted: Vec<u32>,
    #[serde(default)]
    pub last_talk: u64,
    #[serde(default)]
    pub last_special_add: u64,
    #[serde(default)]
    pub memory_clear_tick: u64,
    #[serde(default)]
    pub store_created: bool,
}

/// C `merchant_driver_parse` from `src/module/merchants/merchant.c`. The C
/// driver defaults opening hours to 6..23 before parsing.
pub fn parse_merchant_driver_args(args: &str) -> MerchantDriverData {
    let mut data = MerchantDriverData {
        open: 6,
        close: 23,
        ..MerchantDriverData::default()
    };
    let mut rest = args;
    while let Some((name, value, next)) = next_legacy_name_value(rest) {
        let parsed = value.parse::<i32>().unwrap_or(0);
        match name {
            "dir" => data.dir = parsed,
            "dayx" => data.dayx = parsed,
            "dayy" => data.dayy = parsed,
            "daydir" => data.daydir = parsed,
            "nightx" => data.nightx = parsed,
            "nighty" => data.nighty = parsed,
            "nightdir" => data.nightdir = parsed,
            "ignore" => data.ignore = parsed,
            "storefx" => data.storefx = parsed,
            "storefy" => data.storefy = parsed,
            "storetx" => data.storetx = parsed,
            "storety" => data.storety = parsed,
            "doorx" => data.doorx = parsed,
            "doory" => data.doory = parsed,
            "open" => data.open = parsed,
            "close" => data.close = parsed,
            "special" => data.special = parsed,
            "pricemulti" => data.pricemulti = parsed,
            _ => {}
        }
        rest = next;
    }
    data
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TwoSkellyDriverData {
    pub last_talk_tick: i32,
    pub current_victim: Option<CharacterId>,
    pub alive_tick: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClaraDriverData {
    pub last_talk_tick: i32,
    pub current_victim: Option<CharacterId>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Lab2UndeadDriverData {
    pub aggressive: i32,
    pub helper: i32,
    pub undead: i32,
    pub patrol: i32,
    pub pat: u8,
    pub patstep: u8,
    pub patx: [u8; 8],
    pub paty: [u8; 8],
    pub grave_item_id: Option<ItemId>,
    pub regenerate_item_id: Option<ItemId>,
    pub opened_by_character_id: Option<CharacterId>,
    pub opened_by_serial: u32,
    pub next_wait_tick: i32,
    #[serde(default)]
    pub enemies: Vec<SimpleBaddyEnemy>,
}

impl Default for Lab2UndeadDriverData {
    fn default() -> Self {
        Self {
            aggressive: 0,
            helper: 0,
            undead: 0,
            patrol: 0,
            pat: 0,
            patstep: 0,
            patx: [0; 8],
            paty: [0; 8],
            grave_item_id: None,
            regenerate_item_id: None,
            opened_by_character_id: None,
            opened_by_serial: 0,
            next_wait_tick: 0,
            enemies: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SimpleBaddyDriverData {
    pub startdist: i32,
    pub chardist: i32,
    pub stopdist: i32,
    #[serde(default)]
    pub home_x: u16,
    #[serde(default)]
    pub home_y: u16,
    pub aggressive: i32,
    pub helper: i32,
    pub scavenger: i32,
    pub dir: i32,
    pub dayx: i32,
    pub dayy: i32,
    pub daydir: i32,
    pub nightx: i32,
    pub nighty: i32,
    pub nightdir: i32,
    pub teleport: i32,
    pub helpid: i32,
    pub creation_time: i32,
    pub notsecure: i32,
    pub mindist: i32,
    pub lastfight: i32,
    #[serde(default)]
    pub last_hit: i32,
    #[serde(default)]
    pub pending_bless_friend: Option<CharacterId>,
    pub poison_power: i32,
    pub poison_chance: i32,
    pub poison_type: i32,
    pub drinkspecial: i32,
    pub drink_inventory_potions: i32,
    #[serde(default)]
    pub enemies: Vec<SimpleBaddyEnemy>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SimpleBaddyEnemy {
    pub target_id: CharacterId,
    pub priority: i32,
    pub last_seen_tick: i32,
    #[serde(default)]
    pub visible: bool,
    #[serde(default)]
    pub last_x: u16,
    #[serde(default)]
    pub last_y: u16,
}

impl Default for SimpleBaddyDriverData {
    fn default() -> Self {
        Self {
            startdist: 20,
            chardist: 0,
            stopdist: 40,
            home_x: 0,
            home_y: 0,
            aggressive: 0,
            helper: 0,
            scavenger: 0,
            dir: 3,
            dayx: 0,
            dayy: 0,
            daydir: 0,
            nightx: 0,
            nighty: 0,
            nightdir: 0,
            teleport: 0,
            helpid: 0,
            creation_time: 0,
            notsecure: 0,
            mindist: 0,
            lastfight: 0,
            last_hit: 0,
            pending_bless_friend: None,
            poison_power: 0,
            poison_chance: 0,
            poison_type: 0,
            drinkspecial: 0,
            drink_inventory_potions: 0,
            enemies: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownSimpleBaddyArgument {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimpleBaddyParseResult {
    pub data: SimpleBaddyDriverData,
    pub unknown: Vec<UnknownSimpleBaddyArgument>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimpleBaddyMessageOutcome {
    UseInventoryPotion {
        item_id: ItemId,
        reason: PotionUseReason,
    },
    BlessFriend {
        target_id: crate::ids::CharacterId,
    },
    PoisonHit {
        target_id: CharacterId,
        power: u16,
        poison_type: u16,
        chance: i32,
    },
    AddEnemy {
        caller_id: CharacterId,
        target_id: CharacterId,
    },
    RemoveEnemy {
        target_id: CharacterId,
    },
    StandardAggro {
        target_id: CharacterId,
        priority: i32,
        require_visible: bool,
        hurtme: bool,
    },
    StandardSeenHit {
        attacker_id: CharacterId,
        victim_id: CharacterId,
    },
    TextNotification {
        speaker_id: CharacterId,
        text_token: i32,
        text: Option<String>,
    },
    NoteHit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PotionUseReason {
    LowHp,
    LowMana,
}

pub fn parse_simple_baddy_driver_args(args: &str) -> SimpleBaddyParseResult {
    let mut data = SimpleBaddyDriverData::default();
    let mut unknown = Vec::new();
    let mut rest = args;

    while let Some((name, value, next)) = next_legacy_name_value(rest) {
        let parsed = value.parse::<i32>().unwrap_or(0);
        match name {
            "aggressive" => data.aggressive = parsed,
            "scavenger" => data.scavenger = parsed,
            "helper" => data.helper = parsed,
            "startdist" => data.startdist = parsed,
            "chardist" => data.chardist = parsed,
            "stopdist" => data.stopdist = parsed,
            "dir" => data.dir = parsed,
            "dayx" => data.dayx = parsed,
            "dayy" => data.dayy = parsed,
            "daydir" => data.daydir = parsed,
            "nightx" => data.nightx = parsed,
            "nighty" => data.nighty = parsed,
            "nightdir" => data.nightdir = parsed,
            "teleport" => data.teleport = parsed,
            "helpid" => data.helpid = parsed,
            "notsecure" => data.notsecure = parsed,
            "mindist" => data.mindist = parsed,
            "poisonpower" => data.poison_power = parsed,
            "poisontype" => data.poison_type = parsed,
            "poisonchance" => data.poison_chance = parsed,
            "drinkspecial" => data.drinkspecial = parsed,
            "drinkinvpots" => data.drink_inventory_potions = parsed,
            _ => unknown.push(UnknownSimpleBaddyArgument {
                name: name.to_string(),
                value: value.to_string(),
            }),
        }
        rest = next;
    }

    SimpleBaddyParseResult { data, unknown }
}

pub fn apply_simple_baddy_create_message(
    character: &mut Character,
    args: Option<&str>,
    current_tick: i32,
) -> Vec<UnknownSimpleBaddyArgument> {
    let mut data = match character.driver_state.take() {
        Some(CharacterDriverState::SimpleBaddy(data)) => data,
        Some(
            CharacterDriverState::Clara(_)
            | CharacterDriverState::TwoSkelly(_)
            | CharacterDriverState::Lab2Undead(_)
            | CharacterDriverState::Merchant(_)
            | CharacterDriverState::Lostcon(_),
        ) => SimpleBaddyDriverData::default(),
        None => SimpleBaddyDriverData::default(),
    };

    let unknown = if let Some(args) = args.filter(|args| !args.is_empty()) {
        let parsed = parse_simple_baddy_driver_args(args);
        data = parsed.data;
        parsed.unknown
    } else {
        Vec::new()
    };

    data.creation_time = current_tick;
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(data));
    character
        .driver_messages
        .retain(|message| message.message_type != NT_CREATE);

    if character.inventory.get(30).and_then(|slot| *slot).is_some()
        && character.flags.contains(CharacterFlags::NOBODY)
    {
        character.flags.remove(CharacterFlags::NOBODY);
        character.flags.insert(CharacterFlags::ITEMDEATH);
    }

    unknown
}

pub fn parse_lab2_undead_driver_args(
    args: &str,
) -> (Lab2UndeadDriverData, Vec<UnknownSimpleBaddyArgument>) {
    let mut data = Lab2UndeadDriverData::default();
    let mut unknown = Vec::new();
    let mut rest = args;

    while let Some((name, value, next)) = next_legacy_name_value(rest) {
        let parsed = value.parse::<i32>().unwrap_or(0);
        match name {
            "aggressive" => data.aggressive = parsed,
            "helper" => data.helper = parsed,
            "patrol" => data.patrol = parsed,
            "undead" => data.undead = parsed,
            _ => unknown.push(UnknownSimpleBaddyArgument {
                name: name.to_string(),
                value: value.to_string(),
            }),
        }
        rest = next;
    }

    (data, unknown)
}

pub fn apply_lab2_undead_create_message(
    character: &mut Character,
    args: Option<&str>,
) -> Vec<UnknownSimpleBaddyArgument> {
    let mut data = match character.driver_state.take() {
        Some(CharacterDriverState::Lab2Undead(data)) => data,
        _ => Lab2UndeadDriverData::default(),
    };

    let unknown = if let Some(args) = args.filter(|args| !args.is_empty()) {
        let parsed = parse_lab2_undead_driver_args(args);
        data = parsed.0;
        parsed.1
    } else {
        Vec::new()
    };

    apply_lab2_undead_patrol_defaults(&mut data);
    character.driver_state = Some(CharacterDriverState::Lab2Undead(data));
    character
        .driver_messages
        .retain(|message| message.message_type != NT_CREATE);
    unknown
}

fn apply_lab2_undead_patrol_defaults(data: &mut Lab2UndeadDriverData) {
    match data.patrol {
        1 => {
            data.patx = [168, 168, 204, 204, 0, 0, 0, 0];
            data.paty = [178, 218, 218, 178, 0, 0, 0, 0];
            data.patstep = 4;
            data.helper = 0;
        }
        2 => {
            data.patx = [171, 138, 138, 165, 167, 138, 138, 171];
            data.paty = [164, 164, 146, 146, 146, 146, 164, 164];
            data.patstep = 8;
            data.helper = 0;
        }
        _ => {}
    }
}

pub fn process_simple_baddy_messages(
    character: &mut Character,
    carried_items: &[Item],
) -> Vec<SimpleBaddyMessageOutcome> {
    let drink_inventory_potions = matches!(
        character.driver_state.as_ref(),
        Some(CharacterDriverState::SimpleBaddy(data)) if data.drink_inventory_potions != 0
    );
    let helper = match character.driver_state.as_ref() {
        Some(CharacterDriverState::SimpleBaddy(data)) => data.helper,
        _ => 0,
    };
    let aggressive = match character.driver_state.as_ref() {
        Some(CharacterDriverState::SimpleBaddy(data)) => data.aggressive,
        _ => 0,
    };
    let poison = match character.driver_state.as_ref() {
        Some(CharacterDriverState::SimpleBaddy(data)) if data.poison_power > 0 => Some((
            data.poison_power as u16,
            data.poison_type.max(0) as u16,
            data.poison_chance,
        )),
        _ => None,
    };
    let helpid = match character.driver_state.as_ref() {
        Some(CharacterDriverState::SimpleBaddy(data)) => data.helpid,
        _ => 0,
    };
    let mut outcomes = Vec::new();

    let messages = std::mem::take(&mut character.driver_messages);
    for message in messages {
        if message.message_type == NT_CHAR && helper != 0 && message.dat1 > 0 {
            outcomes.push(SimpleBaddyMessageOutcome::BlessFriend {
                target_id: crate::ids::CharacterId(message.dat1 as u32),
            });
        }

        if message.message_type == NT_CHAR && aggressive != 0 && message.dat1 > 0 {
            outcomes.push(SimpleBaddyMessageOutcome::StandardAggro {
                target_id: CharacterId(message.dat1 as u32),
                priority: 0,
                require_visible: true,
                hurtme: false,
            });
        }

        if message.message_type == NT_SEEHIT && helper != 0 && message.dat1 > 0 && message.dat2 > 0
        {
            outcomes.push(SimpleBaddyMessageOutcome::StandardSeenHit {
                attacker_id: CharacterId(message.dat1 as u32),
                victim_id: CharacterId(message.dat2 as u32),
            });
        }

        if message.message_type == NT_TEXT && message.dat3 > 0 {
            outcomes.push(SimpleBaddyMessageOutcome::TextNotification {
                speaker_id: CharacterId(message.dat3 as u32),
                text_token: message.dat2,
                text: message.text.clone(),
            });
        }

        if message.message_type == NT_GOTHIT && drink_inventory_potions {
            if let Some(item_id) = find_simple_baddy_inventory_potion(
                character,
                carried_items,
                CharacterValue::Hp,
                2,
                PotionUseReason::LowHp,
            ) {
                outcomes.push(SimpleBaddyMessageOutcome::UseInventoryPotion {
                    item_id,
                    reason: PotionUseReason::LowHp,
                });
            }

            if let Some(item_id) = find_simple_baddy_inventory_potion(
                character,
                carried_items,
                CharacterValue::Mana,
                4,
                PotionUseReason::LowMana,
            ) {
                outcomes.push(SimpleBaddyMessageOutcome::UseInventoryPotion {
                    item_id,
                    reason: PotionUseReason::LowMana,
                });
            }
        }

        if message.message_type == NT_GOTHIT {
            outcomes.push(SimpleBaddyMessageOutcome::NoteHit);
        }

        if message.message_type == NT_GOTHIT && message.dat1 > 0 {
            outcomes.push(SimpleBaddyMessageOutcome::StandardAggro {
                target_id: CharacterId(message.dat1 as u32),
                priority: 1,
                require_visible: false,
                hurtme: true,
            });
        }

        if message.message_type == NT_DIDHIT && message.dat1 > 0 && message.dat2 > 0 {
            if let Some((power, poison_type, chance)) = poison {
                outcomes.push(SimpleBaddyMessageOutcome::PoisonHit {
                    target_id: CharacterId(message.dat1 as u32),
                    power,
                    poison_type,
                    chance,
                });
            }
        }

        if message.message_type == NT_NPC
            && helpid != 0
            && message.dat1 == helpid
            && message.dat2 > 0
        {
            outcomes.push(SimpleBaddyMessageOutcome::AddEnemy {
                caller_id: CharacterId(message.dat2 as u32),
                target_id: CharacterId(message.dat3.max(0) as u32),
            });
        }

        if message.message_type == NT_DEAD && message.dat1 > 0 {
            outcomes.push(SimpleBaddyMessageOutcome::RemoveEnemy {
                target_id: CharacterId(message.dat1 as u32),
            });
        }
    }

    outcomes
}

pub fn add_simple_baddy_enemy(
    character: &mut Character,
    caller: &Character,
    target_id: CharacterId,
    current_tick: i32,
) -> bool {
    if caller.id == character.id || caller.group != character.group {
        return false;
    }

    add_simple_baddy_enemy_unchecked(character, target_id, 1, current_tick)
}

pub fn add_simple_baddy_enemy_unchecked(
    character: &mut Character,
    target_id: CharacterId,
    priority: i32,
    current_tick: i32,
) -> bool {
    let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() else {
        return false;
    };

    if let Some(enemy) = data
        .enemies
        .iter_mut()
        .take(9)
        .find(|enemy| enemy.target_id == target_id)
    {
        enemy.priority = priority;
        enemy.last_seen_tick = current_tick;
        return false;
    }

    let enemy = SimpleBaddyEnemy {
        target_id,
        priority,
        last_seen_tick: current_tick,
        visible: false,
        last_x: 0,
        last_y: 0,
    };
    if data.enemies.len() < 10 {
        data.enemies.push(enemy);
    } else {
        data.enemies[9] = enemy;
    }
    true
}

pub fn remove_simple_baddy_enemy(character: &mut Character, target_id: CharacterId) -> bool {
    let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state.as_mut() else {
        return false;
    };

    let previous_len = data.enemies.len();
    data.enemies.retain(|enemy| enemy.target_id != target_id);
    data.enemies.len() != previous_len
}

fn find_simple_baddy_inventory_potion(
    character: &Character,
    carried_items: &[Item],
    value: CharacterValue,
    divisor: i32,
    reason: PotionUseReason,
) -> Option<ItemId> {
    let max_value = character_value(character, value);
    if max_value == 0 {
        return None;
    }

    let current = match value {
        CharacterValue::Hp => character.hp,
        CharacterValue::Mana => character.mana,
        _ => return None,
    };
    if current >= max_value * POWERSCALE / divisor {
        return None;
    }

    character
        .inventory
        .get(30..INVENTORY_SIZE)
        .unwrap_or_default()
        .iter()
        .flatten()
        .find(|item_id| {
            carried_items
                .iter()
                .find(|item| item.id == **item_id)
                .is_some_and(|item| {
                    item.driver == IDR_POTION
                        && match reason {
                            PotionUseReason::LowHp => drdata(item, 1) != 0,
                            PotionUseReason::LowMana => drdata(item, 2) != 0,
                        }
                })
        })
        .copied()
}

fn character_value(character: &Character, value: CharacterValue) -> i32 {
    character
        .values
        .get(1)
        .and_then(|values| values.get(value as usize))
        .copied()
        .unwrap_or_default()
        .into()
}

fn drdata(item: &Item, index: usize) -> u8 {
    item.driver_data.get(index).copied().unwrap_or_default()
}

fn next_legacy_name_value(input: &str) -> Option<(&str, &str, &str)> {
    let input = input.trim_start_matches(char::is_whitespace);
    let name_len = input
        .bytes()
        .take(60)
        .take_while(|byte| byte.is_ascii_alphabetic())
        .count();
    if name_len == 0 {
        return None;
    }
    let name = &input[..name_len];
    let input = input[name_len..].trim_start_matches(char::is_whitespace);
    let input = input.strip_prefix('=')?;
    let input = input.trim_start_matches(char::is_whitespace);
    let value_len = input
        .bytes()
        .take(60)
        .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
        .count();
    let value = &input[..value_len];
    let input = input[value_len..].strip_prefix(';')?;
    Some((name, value, input.trim_start_matches(char::is_whitespace)))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaraDialogueContext<'a> {
    pub player_name: &'a str,
    pub clara_name: &'a str,
    pub army_rank: &'a str,
    pub kelly_state: i32,
    pub clara_state: i32,
    pub has_hardkill_item: bool,
    pub hardkill_ritual_progress: u8,
    pub questlog_21_count: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaraDialogueOutcome {
    pub clara_state: i32,
    pub text: Option<String>,
    pub open_questlog: Option<u16>,
    pub complete_questlog: Option<u16>,
    pub military_points: i32,
    pub military_exp: i32,
}

pub const EXP_AREA15_HARDKILL: i32 = 5_000;

pub fn clara_dialogue_step(context: ClaraDialogueContext<'_>) -> ClaraDialogueOutcome {
    let mut state = context.clara_state;
    let mut open_questlog = None;
    let mut complete_questlog = None;
    let mut military_points = 0;
    let mut military_exp = 0;
    let text = match state {
        0 => {
            state += 1;
            Some(format!(
                "Greetings, {}! I am {}, First Sergeant of the Seyan'Du and commander of this outpost.",
                context.player_name, context.clara_name
            ))
        }
        1 if context.kelly_state >= 15 => {
            state += 1;
            clara_dialogue_step_text_after_fallthrough(&mut state, context)
        }
        1 => None,
        2 => clara_dialogue_step_text_after_fallthrough(&mut state, context),
        3 => {
            state += 1;
            Some(
                "Under the current circumstances, I do not recommend sending reinforcements to secure the road. We cannot afford to bind our forces here. Now go back to Aston and deliver this report."
                    .to_string(),
            )
        }
        4 => {
            state += 1;
            Some(format!(
                "Afterwards come back here, I have more work for thee. That will be all, {}. Dismissed!",
                context.army_rank
            ))
        }
        5 if context.kelly_state >= 18 => {
            state += 1;
            open_questlog = Some(21);
            state += 1;
            Some(format!(
                "I have a difficult mission for thee, {}. The main reason we had to retreat to this camp was one huge swamp beast. It seemed to be immune to our attacks.",
                context.player_name
            ))
        }
        5 => None,
        6 => {
            open_questlog = Some(21);
            state += 1;
            Some(format!(
                "I have a difficult mission for thee, {}. The main reason we had to retreat to this camp was one huge swamp beast. It seemed to be immune to our attacks.",
                context.player_name
            ))
        }
        7 => {
            state += 1;
            Some(
                "I want thee to find a way to slay it. I have heard rumors about a man who used to live with the swamp beasts north-east of this camp. Mayhap he knows a way to injure this beast."
                    .to_string(),
            )
        }
        8 => {
            state += 1;
            Some(format!(
                "Dismissed, {}. And good luck. Thou wilt need it.",
                context.army_rank
            ))
        }
        9 if context.has_hardkill_item => {
            if context.questlog_21_count == 0 {
                military_points = 4;
                military_exp = EXP_AREA15_HARDKILL;
            }
            state += 1;
            clara_hardkill_report_text(&mut state, context)
        }
        9 => None,
        10 => clara_hardkill_report_text(&mut state, context),
        11 if context.has_hardkill_item && context.hardkill_ritual_progress >= 36 => {
            state += 1;
            state += 1;
            Some("Now that thou knowest how to kill that beast, please go and do it.".to_string())
        }
        11 => None,
        12 => {
            state += 1;
            Some("Now that thou knowest how to kill that beast, please go and do it.".to_string())
        }
        13 => None,
        14 => {
            complete_questlog = Some(21);
            if context.questlog_21_count == 1 {
                military_points = 8;
                military_exp = 1;
            }
            state += 1;
            Some(format!("Well done indeed, {}!", context.player_name))
        }
        15 => {
            state += 1;
            Some(format!(
                "The swamp will be safer now, but more dangers await thee on thy travels. May Ishtar be with thee, {}.",
                context.player_name
            ))
        }
        _ => None,
    };

    ClaraDialogueOutcome {
        clara_state: state,
        text,
        open_questlog,
        complete_questlog,
        military_points,
        military_exp,
    }
}

fn clara_dialogue_step_text_after_fallthrough(
    state: &mut i32,
    context: ClaraDialogueContext<'_>,
) -> Option<String> {
    *state += 1;
    Some(format!(
        "I assume thou hast been sent from Aston, {}, to report on our status. The road through the swamp is no longer secure and we have been under attack from beasts emerging from the swamp.",
        context.army_rank
    ))
}

fn clara_hardkill_report_text(
    state: &mut i32,
    context: ClaraDialogueContext<'_>,
) -> Option<String> {
    *state += 1;
    if context.has_hardkill_item && context.hardkill_ritual_progress < 36 {
        Some(format!(
            "So that is how one can kill them. Thou wilt need to find all three stone circles and perform the ritual in each one, then, {}.",
            context.player_name
        ))
    } else {
        Some("So that is how one can kill them.".to_string())
    }
}

pub fn clara_replay_state_after_text_analysis(clara_state: i32, didsay: i32) -> i32 {
    if didsay != 2 {
        return clara_state;
    }
    match clara_state {
        ..=5 => 0,
        6..=9 => 6,
        10..=11 => 10,
        12..=13 => 12,
        15..=16 => 15,
        _ => clara_state,
    }
}

pub fn clara_state_after_swamp_monster_death(
    clara_state: i32,
    killer_is_player: bool,
    monster_is_hardkill: bool,
) -> i32 {
    if killer_is_player && monster_is_hardkill && (12..=13).contains(&clara_state) {
        14
    } else {
        clara_state
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterDriverKind {
    SimpleBaddy,
    Macro,
    SwampClara,
    SwampMonster,
    PalaceIslena,
    TwoSkelly,
    Trader,
    LqNpc,
    Janitor,
    TeufelDemon,
    TeufelGambler,
    TeufelQuest,
    TeufelRat,
    CaligarSkelly,
    Lab2Undead,
}

impl CharacterDriverKind {
    pub fn from_legacy_id(driver: u16) -> Option<Self> {
        match driver {
            CDR_SIMPLEBADDY => Some(Self::SimpleBaddy),
            CDR_MACRO => Some(Self::Macro),
            CDR_SWAMPCLARA => Some(Self::SwampClara),
            CDR_SWAMPMONSTER => Some(Self::SwampMonster),
            CDR_PALACEISLENA => Some(Self::PalaceIslena),
            CDR_TWOSKELLY => Some(Self::TwoSkelly),
            CDR_TRADER => Some(Self::Trader),
            CDR_LQNPC => Some(Self::LqNpc),
            CDR_JANITOR => Some(Self::Janitor),
            CDR_TEUFELDEMON => Some(Self::TeufelDemon),
            CDR_TEUFELGAMBLER => Some(Self::TeufelGambler),
            CDR_TEUFELQUEST => Some(Self::TeufelQuest),
            CDR_TEUFELRAT => Some(Self::TeufelRat),
            CDR_CALIGARSKELLY => Some(Self::CaligarSkelly),
            CDR_LAB2UNDEAD => Some(Self::Lab2Undead),
            _ => None,
        }
    }

    pub fn legacy_id(self) -> u16 {
        match self {
            Self::SimpleBaddy => CDR_SIMPLEBADDY,
            Self::Macro => CDR_MACRO,
            Self::SwampClara => CDR_SWAMPCLARA,
            Self::SwampMonster => CDR_SWAMPMONSTER,
            Self::PalaceIslena => CDR_PALACEISLENA,
            Self::TwoSkelly => CDR_TWOSKELLY,
            Self::Trader => CDR_TRADER,
            Self::LqNpc => CDR_LQNPC,
            Self::Janitor => CDR_JANITOR,
            Self::TeufelDemon => CDR_TEUFELDEMON,
            Self::TeufelGambler => CDR_TEUFELGAMBLER,
            Self::TeufelQuest => CDR_TEUFELQUEST,
            Self::TeufelRat => CDR_TEUFELRAT,
            Self::CaligarSkelly => CDR_CALIGARSKELLY,
            Self::Lab2Undead => CDR_LAB2UNDEAD,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterDriverCall {
    Tick { ret: i32, last_action: i32 },
    Died { killer_character_id: u32 },
    Respawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterDriverOutcome {
    /// `simple_baddy_dead`: earth demons create earth/rain retaliation effects
    /// at the killer position when the dead NPC can see the killer.
    SimpleBaddyDeath { killer_character_id: u32 },
    /// Legacy handler returned `1`; behavior is intentionally deferred to a
    /// future typed implementation for this concrete driver.
    HandledStub {
        kind: CharacterDriverKind,
        call: CharacterDriverCall,
    },
    /// Legacy module probing would continue and eventually return `0`.
    Unsupported {
        driver: u16,
        call: CharacterDriverCall,
    },
}

impl CharacterDriverOutcome {
    pub fn legacy_return_code(self) -> i32 {
        match self {
            Self::SimpleBaddyDeath { .. } => 1,
            Self::HandledStub { .. } => 1,
            Self::Unsupported { .. } => 0,
        }
    }
}

pub fn execute_character_driver(driver: u16, ret: i32, last_action: i32) -> CharacterDriverOutcome {
    let call = CharacterDriverCall::Tick { ret, last_action };
    dispatch_known_character_driver(driver, call)
}

pub fn execute_character_died_driver(
    driver: u16,
    killer_character_id: u32,
) -> CharacterDriverOutcome {
    let call = CharacterDriverCall::Died {
        killer_character_id,
    };
    dispatch_known_character_driver(driver, call)
}

pub fn execute_character_respawn_driver(driver: u16) -> CharacterDriverOutcome {
    dispatch_known_character_driver(driver, CharacterDriverCall::Respawn)
}

fn dispatch_known_character_driver(
    driver: u16,
    call: CharacterDriverCall,
) -> CharacterDriverOutcome {
    if driver == CDR_SIMPLEBADDY {
        if let CharacterDriverCall::Died {
            killer_character_id,
        } = call
        {
            return CharacterDriverOutcome::SimpleBaddyDeath {
                killer_character_id,
            };
        }
    }

    match CharacterDriverKind::from_legacy_id(driver) {
        Some(kind) => CharacterDriverOutcome::HandledStub { kind, call },
        None => CharacterDriverOutcome::Unsupported { driver, call },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        entity::{ItemFlags, SpeedMode},
        ids::ItemId,
    };

    #[test]
    fn legacy_dispatch_type_constants_match_c_libload() {
        assert_eq!(CDT_DRIVER, 0);
        assert_eq!(CDT_ITEM, 1);
        assert_eq!(CDT_DEAD, 2);
        assert_eq!(CDT_RESPAWN, 3);
        assert_eq!(CDT_SPECIAL, 4);
    }

    #[test]
    fn notify_constants_match_c_notify_header() {
        assert_eq!(NT_CHAR, 1);
        assert_eq!(NT_ITEM, 2);
        assert_eq!(NT_GOTHIT, 3);
        assert_eq!(NT_DIDHIT, 4);
        assert_eq!(NT_SEEHIT, 5);
        assert_eq!(NT_DEAD, 6);
        assert_eq!(NT_SPELL, 7);
        assert_eq!(NT_GIVE, 8);
        assert_eq!(NT_CREATE, 9);
        assert_eq!(NT_TEXT, 200);
        assert_eq!(NT_NPC, 300);
        assert_eq!(NTID_MERCHANT, 1);
        assert_eq!(NTID_GLADIATOR, 16);
    }

    #[test]
    fn base_character_driver_ids_match_c_drvlib() {
        assert_eq!(CDR_LOSTCON, 5);
        assert_eq!(CDR_SIMPLEBADDY, 7);
        assert_eq!(CDR_MACRO, 37);
        assert_eq!(CDR_SWAMPCLARA, 54);
        assert_eq!(CDR_SWAMPMONSTER, 56);
        assert_eq!(CDR_PALACEISLENA, 57);
        assert_eq!(CDR_TWOSKELLY, 70);
        assert_eq!(CDR_TRADER, 72);
        assert_eq!(CDR_LQNPC, 74);
        assert_eq!(CDR_JANITOR, 85);
        assert_eq!(CDR_TEUFELDEMON, 114);
        assert_eq!(CDR_TEUFELGAMBLER, 115);
        assert_eq!(CDR_TEUFELQUEST, 116);
        assert_eq!(CDR_TEUFELRAT, 117);
        assert_eq!(CDR_CALIGARSKELLY, 124);
        assert_eq!(CDR_LAB2UNDEAD, 198);
        assert_eq!(DRD_SIMPLEBADDYDRIVER, 0x0100_0013);
        assert_eq!(
            CharacterDriverKind::SimpleBaddy.legacy_id(),
            CDR_SIMPLEBADDY
        );
        assert_eq!(CharacterDriverKind::Macro.legacy_id(), CDR_MACRO);
        assert_eq!(CharacterDriverKind::SwampClara.legacy_id(), CDR_SWAMPCLARA);
        assert_eq!(
            CharacterDriverKind::SwampMonster.legacy_id(),
            CDR_SWAMPMONSTER
        );
        assert_eq!(
            CharacterDriverKind::PalaceIslena.legacy_id(),
            CDR_PALACEISLENA
        );
        assert_eq!(CharacterDriverKind::TwoSkelly.legacy_id(), CDR_TWOSKELLY);
        assert_eq!(CharacterDriverKind::Trader.legacy_id(), CDR_TRADER);
        assert_eq!(CharacterDriverKind::LqNpc.legacy_id(), CDR_LQNPC);
        assert_eq!(CharacterDriverKind::Janitor.legacy_id(), CDR_JANITOR);
        assert_eq!(
            CharacterDriverKind::TeufelDemon.legacy_id(),
            CDR_TEUFELDEMON
        );
        assert_eq!(
            CharacterDriverKind::TeufelGambler.legacy_id(),
            CDR_TEUFELGAMBLER
        );
        assert_eq!(
            CharacterDriverKind::TeufelQuest.legacy_id(),
            CDR_TEUFELQUEST
        );
        assert_eq!(CharacterDriverKind::TeufelRat.legacy_id(), CDR_TEUFELRAT);
        assert_eq!(
            CharacterDriverKind::CaligarSkelly.legacy_id(),
            CDR_CALIGARSKELLY
        );
        assert_eq!(CharacterDriverKind::Lab2Undead.legacy_id(), CDR_LAB2UNDEAD);
        assert_eq!(DRD_CLARADRIVER, 0x0100_0059);
        assert_eq!(DRD_SKELLYDRIVER, 0x0100_006a);
        assert_eq!(DRD_LAB2_UNDEAD, 0x0200_0001);
    }

    #[test]
    fn two_skelly_driver_state_matches_legacy_runtime_data_shape() {
        let mut data = TwoSkellyDriverData::default();
        assert_eq!(data.last_talk_tick, 0);
        assert_eq!(data.current_victim, None);
        assert_eq!(data.alive_tick, 0);

        data.last_talk_tick = 111;
        data.current_victim = Some(CharacterId(12));
        data.alive_tick = 222;
        assert_eq!(
            CharacterDriverState::TwoSkelly(data),
            CharacterDriverState::TwoSkelly(TwoSkellyDriverData {
                last_talk_tick: 111,
                current_victim: Some(CharacterId(12)),
                alive_tick: 222,
            })
        );
    }

    #[test]
    fn clara_driver_state_matches_legacy_runtime_data_shape() {
        let mut data = ClaraDriverData::default();
        assert_eq!(data.last_talk_tick, 0);
        assert_eq!(data.current_victim, None);

        data.last_talk_tick = 1234;
        data.current_victim = Some(CharacterId(77));
        assert_eq!(
            CharacterDriverState::Clara(data),
            CharacterDriverState::Clara(ClaraDriverData {
                last_talk_tick: 1234,
                current_victim: Some(CharacterId(77)),
            })
        );
    }

    #[test]
    fn known_base_tick_drivers_are_handled_like_c_ch_driver() {
        for (driver, kind) in [
            (CDR_SIMPLEBADDY, CharacterDriverKind::SimpleBaddy),
            (CDR_MACRO, CharacterDriverKind::Macro),
            (CDR_SWAMPCLARA, CharacterDriverKind::SwampClara),
            (CDR_SWAMPMONSTER, CharacterDriverKind::SwampMonster),
            (CDR_PALACEISLENA, CharacterDriverKind::PalaceIslena),
            (CDR_TWOSKELLY, CharacterDriverKind::TwoSkelly),
            (CDR_TRADER, CharacterDriverKind::Trader),
            (CDR_LQNPC, CharacterDriverKind::LqNpc),
            (CDR_JANITOR, CharacterDriverKind::Janitor),
            (CDR_TEUFELDEMON, CharacterDriverKind::TeufelDemon),
            (CDR_TEUFELGAMBLER, CharacterDriverKind::TeufelGambler),
            (CDR_TEUFELQUEST, CharacterDriverKind::TeufelQuest),
            (CDR_TEUFELRAT, CharacterDriverKind::TeufelRat),
            (CDR_LAB2UNDEAD, CharacterDriverKind::Lab2Undead),
        ] {
            let outcome = execute_character_driver(driver, 7, 11);
            assert_eq!(
                outcome,
                CharacterDriverOutcome::HandledStub {
                    kind,
                    call: CharacterDriverCall::Tick {
                        ret: 7,
                        last_action: 11,
                    },
                }
            );
            assert_eq!(outcome.legacy_return_code(), 1);
        }
    }

    #[test]
    fn known_base_death_and_respawn_drivers_are_handled_like_c() {
        let simple_died = execute_character_died_driver(CDR_SIMPLEBADDY, 123);
        assert_eq!(
            simple_died,
            CharacterDriverOutcome::SimpleBaddyDeath {
                killer_character_id: 123,
            }
        );
        assert_eq!(simple_died.legacy_return_code(), 1);

        let died = execute_character_died_driver(CDR_JANITOR, 123);
        assert_eq!(
            died,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::Janitor,
                call: CharacterDriverCall::Died {
                    killer_character_id: 123,
                },
            }
        );
        assert_eq!(died.legacy_return_code(), 1);

        let islena_died = execute_character_died_driver(CDR_PALACEISLENA, 123);
        assert_eq!(
            islena_died,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::PalaceIslena,
                call: CharacterDriverCall::Died {
                    killer_character_id: 123,
                },
            }
        );
        assert_eq!(islena_died.legacy_return_code(), 1);

        let clara_died = execute_character_died_driver(CDR_SWAMPCLARA, 123);
        assert_eq!(
            clara_died,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::SwampClara,
                call: CharacterDriverCall::Died {
                    killer_character_id: 123,
                },
            }
        );
        assert_eq!(clara_died.legacy_return_code(), 1);

        let two_skelly_died = execute_character_died_driver(CDR_TWOSKELLY, 123);
        assert_eq!(
            two_skelly_died,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::TwoSkelly,
                call: CharacterDriverCall::Died {
                    killer_character_id: 123,
                },
            }
        );
        assert_eq!(two_skelly_died.legacy_return_code(), 1);

        let swamp_monster_died = execute_character_died_driver(CDR_SWAMPMONSTER, 123);
        assert_eq!(
            swamp_monster_died,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::SwampMonster,
                call: CharacterDriverCall::Died {
                    killer_character_id: 123,
                },
            }
        );
        assert_eq!(swamp_monster_died.legacy_return_code(), 1);

        let simple_respawn = execute_character_respawn_driver(CDR_SIMPLEBADDY);
        assert_eq!(
            simple_respawn,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::SimpleBaddy,
                call: CharacterDriverCall::Respawn,
            }
        );
        assert_eq!(simple_respawn.legacy_return_code(), 1);

        let respawn = execute_character_respawn_driver(CDR_TRADER);
        assert_eq!(
            respawn,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::Trader,
                call: CharacterDriverCall::Respawn,
            }
        );
        assert_eq!(respawn.legacy_return_code(), 1);

        let islena_respawn = execute_character_respawn_driver(CDR_PALACEISLENA);
        assert_eq!(
            islena_respawn,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::PalaceIslena,
                call: CharacterDriverCall::Respawn,
            }
        );
        assert_eq!(islena_respawn.legacy_return_code(), 1);

        let clara_respawn = execute_character_respawn_driver(CDR_SWAMPCLARA);
        assert_eq!(
            clara_respawn,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::SwampClara,
                call: CharacterDriverCall::Respawn,
            }
        );
        assert_eq!(clara_respawn.legacy_return_code(), 1);

        let two_skelly_respawn = execute_character_respawn_driver(CDR_TWOSKELLY);
        assert_eq!(
            two_skelly_respawn,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::TwoSkelly,
                call: CharacterDriverCall::Respawn,
            }
        );
        assert_eq!(two_skelly_respawn.legacy_return_code(), 1);

        let swamp_monster_respawn = execute_character_respawn_driver(CDR_SWAMPMONSTER);
        assert_eq!(
            swamp_monster_respawn,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::SwampMonster,
                call: CharacterDriverCall::Respawn,
            }
        );
        assert_eq!(swamp_monster_respawn.legacy_return_code(), 1);

        let lab2_undead_died = execute_character_died_driver(CDR_LAB2UNDEAD, 123);
        assert_eq!(
            lab2_undead_died,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::Lab2Undead,
                call: CharacterDriverCall::Died {
                    killer_character_id: 123,
                },
            }
        );
        assert_eq!(lab2_undead_died.legacy_return_code(), 1);

        let lab2_undead_respawn = execute_character_respawn_driver(CDR_LAB2UNDEAD);
        assert_eq!(
            lab2_undead_respawn,
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::Lab2Undead,
                call: CharacterDriverCall::Respawn,
            }
        );
        assert_eq!(lab2_undead_respawn.legacy_return_code(), 1);
    }

    #[test]
    fn unknown_character_driver_returns_legacy_zero() {
        let outcome = execute_character_driver(999, 0, 0);
        assert_eq!(
            outcome,
            CharacterDriverOutcome::Unsupported {
                driver: 999,
                call: CharacterDriverCall::Tick {
                    ret: 0,
                    last_action: 0,
                },
            }
        );
        assert_eq!(outcome.legacy_return_code(), 0);
    }

    #[test]
    fn simple_baddy_defaults_match_create_message_initialization() {
        let data = SimpleBaddyDriverData::default();
        assert_eq!(data.aggressive, 0);
        assert_eq!(data.helper, 0);
        assert_eq!(data.startdist, 20);
        assert_eq!(data.chardist, 0);
        assert_eq!(data.stopdist, 40);
        assert_eq!(data.scavenger, 0);
        assert_eq!(data.dir, 3);
        assert_eq!(data.last_hit, 0);
        assert_eq!(data.drink_inventory_potions, 0);
    }

    #[test]
    fn parses_simple_baddy_legacy_arg_string() {
        let parsed = parse_simple_baddy_driver_args(
            " aggressive = 1; helper=2; startdist=12; poisonpower=-4; poisontype=3; poisonchance=25; drinkinvpots=1; unknown=99;",
        );

        assert_eq!(parsed.data.aggressive, 1);
        assert_eq!(parsed.data.helper, 2);
        assert_eq!(parsed.data.startdist, 12);
        assert_eq!(parsed.data.poison_power, -4);
        assert_eq!(parsed.data.poison_type, 3);
        assert_eq!(parsed.data.poison_chance, 25);
        assert_eq!(parsed.data.drink_inventory_potions, 1);
        assert_eq!(
            parsed.unknown,
            vec![UnknownSimpleBaddyArgument {
                name: "unknown".to_string(),
                value: "99".to_string(),
            }]
        );
    }

    #[test]
    fn simple_baddy_arg_parser_stops_like_c_nextnv_on_malformed_pair() {
        let parsed = parse_simple_baddy_driver_args("aggressive=1; broken 7; helper=1;");

        assert_eq!(parsed.data.aggressive, 1);
        assert_eq!(parsed.data.helper, 0);
        assert!(parsed.unknown.is_empty());
    }

    #[test]
    fn simple_baddy_create_initializes_state_and_item_body_flags() {
        let mut character = test_character();
        character.flags.insert(CharacterFlags::NOBODY);
        character.inventory[30] = Some(ItemId(77));
        character.push_driver_message(NT_CREATE, 0, 0, 0);

        let unknown = apply_simple_baddy_create_message(
            &mut character,
            Some("aggressive=1; startdist=9; drinkinvpots=1; unknown=7;"),
            1234,
        );

        assert_eq!(
            unknown,
            vec![UnknownSimpleBaddyArgument {
                name: "unknown".to_string(),
                value: "7".to_string(),
            }]
        );
        assert!(!character.flags.contains(CharacterFlags::NOBODY));
        assert!(character.flags.contains(CharacterFlags::ITEMDEATH));
        assert!(character.driver_messages.is_empty());

        let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.aggressive, 1);
        assert_eq!(data.startdist, 9);
        assert_eq!(data.drink_inventory_potions, 1);
        assert_eq!(data.creation_time, 1234);
    }

    #[test]
    fn lab2_undead_create_parses_legacy_args_and_graveyard_patrol() {
        let mut character = test_character();
        character.push_driver_message(NT_CREATE, 0, 0, 0);

        let unknown = apply_lab2_undead_create_message(
            &mut character,
            Some("aggressive=1; helper=1; patrol=1; undead=1; strange=7;"),
        );

        assert_eq!(
            unknown,
            vec![UnknownSimpleBaddyArgument {
                name: "strange".to_string(),
                value: "7".to_string(),
            }]
        );
        assert!(character.driver_messages.is_empty());
        let Some(CharacterDriverState::Lab2Undead(data)) = character.driver_state else {
            panic!("lab2 undead state missing");
        };
        assert_eq!(data.aggressive, 1);
        assert_eq!(data.helper, 0);
        assert_eq!(data.undead, 1);
        assert_eq!(data.patrol, 1);
        assert_eq!(data.patstep, 4);
        assert_eq!(&data.patx[..4], &[168, 168, 204, 204]);
        assert_eq!(&data.paty[..4], &[178, 218, 218, 178]);
    }

    #[test]
    fn lab2_undead_crypt_patrol_matches_c_coordinate_table() {
        let mut character = test_character();

        apply_lab2_undead_create_message(&mut character, Some("helper=1; patrol=2;"));

        let Some(CharacterDriverState::Lab2Undead(data)) = character.driver_state else {
            panic!("lab2 undead state missing");
        };
        assert_eq!(data.helper, 0);
        assert_eq!(data.patstep, 8);
        assert_eq!(data.patx, [171, 138, 138, 165, 167, 138, 138, 171]);
        assert_eq!(data.paty, [164, 164, 146, 146, 146, 146, 164, 164]);
    }

    #[test]
    fn simple_baddy_gothit_uses_matching_inventory_potions_when_low() {
        let mut character = test_character();
        character.values[1][CharacterValue::Hp as usize] = 20;
        character.values[1][CharacterValue::Mana as usize] = 20;
        character.hp = 9 * POWERSCALE;
        character.mana = 4 * POWERSCALE;
        character.inventory[30] = Some(ItemId(30));
        character.inventory[31] = Some(ItemId(31));
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            drink_inventory_potions: 1,
            ..SimpleBaddyDriverData::default()
        }));
        character.push_driver_message(NT_GOTHIT, 0, 0, 0);

        let outcomes = process_simple_baddy_messages(
            &mut character,
            &[
                test_item(ItemId(30), IDR_POTION, &[0, 1, 0]),
                test_item(ItemId(31), IDR_POTION, &[0, 0, 1]),
            ],
        );

        assert_eq!(
            outcomes,
            vec![
                SimpleBaddyMessageOutcome::UseInventoryPotion {
                    item_id: ItemId(30),
                    reason: PotionUseReason::LowHp,
                },
                SimpleBaddyMessageOutcome::UseInventoryPotion {
                    item_id: ItemId(31),
                    reason: PotionUseReason::LowMana,
                },
                SimpleBaddyMessageOutcome::NoteHit,
            ]
        );
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_gothit_ignores_disabled_or_wrong_potions() {
        let mut character = test_character();
        character.values[1][CharacterValue::Hp as usize] = 20;
        character.hp = 9 * POWERSCALE;
        character.inventory[29] = Some(ItemId(29));
        character.inventory[30] = Some(ItemId(30));
        character.inventory[31] = Some(ItemId(31));
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            drink_inventory_potions: 1,
            ..SimpleBaddyDriverData::default()
        }));
        character.push_driver_message(NT_GOTHIT, 0, 0, 0);

        let outcomes = process_simple_baddy_messages(
            &mut character,
            &[
                test_item(ItemId(29), IDR_POTION, &[0, 1, 0]),
                test_item(ItemId(30), 999, &[0, 1, 0]),
                test_item(ItemId(31), IDR_POTION, &[0, 0, 1]),
            ],
        );

        assert_eq!(outcomes, vec![SimpleBaddyMessageOutcome::NoteHit]);
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_char_messages_emit_ordered_helper_bless_candidates() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            helper: 1,
            ..SimpleBaddyDriverData::default()
        }));
        character.push_driver_message(NT_CHAR, 2, 0, 0);
        character.push_driver_message(NT_CHAR, 3, 0, 0);

        let outcomes = process_simple_baddy_messages(&mut character, &[]);

        assert_eq!(
            outcomes,
            vec![
                SimpleBaddyMessageOutcome::BlessFriend {
                    target_id: crate::ids::CharacterId(2),
                },
                SimpleBaddyMessageOutcome::BlessFriend {
                    target_id: crate::ids::CharacterId(3),
                },
            ]
        );
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_char_message_ignores_bless_when_helper_disabled() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        character.push_driver_message(NT_CHAR, 2, 0, 0);

        assert!(process_simple_baddy_messages(&mut character, &[]).is_empty());
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_standard_messages_emit_aggro_outcomes() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            aggressive: 1,
            helper: 1,
            ..SimpleBaddyDriverData::default()
        }));
        character.push_driver_message(NT_CHAR, 2, 0, 0);
        character.push_driver_message(NT_SEEHIT, 3, 4, 0);
        character.push_driver_message(NT_GOTHIT, 5, 10, 0);

        let outcomes = process_simple_baddy_messages(&mut character, &[]);

        assert_eq!(
            outcomes,
            vec![
                SimpleBaddyMessageOutcome::BlessFriend {
                    target_id: crate::ids::CharacterId(2),
                },
                SimpleBaddyMessageOutcome::StandardAggro {
                    target_id: crate::ids::CharacterId(2),
                    priority: 0,
                    require_visible: true,
                    hurtme: false,
                },
                SimpleBaddyMessageOutcome::StandardSeenHit {
                    attacker_id: crate::ids::CharacterId(3),
                    victim_id: crate::ids::CharacterId(4),
                },
                SimpleBaddyMessageOutcome::NoteHit,
                SimpleBaddyMessageOutcome::StandardAggro {
                    target_id: crate::ids::CharacterId(5),
                    priority: 1,
                    require_visible: false,
                    hurtme: true,
                },
            ]
        );
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_text_message_preserves_tabunga_notification_boundary() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        character.push_driver_message(NT_TEXT, 0, 12345, 7);

        let outcomes = process_simple_baddy_messages(&mut character, &[]);

        assert_eq!(
            outcomes,
            vec![SimpleBaddyMessageOutcome::TextNotification {
                speaker_id: crate::ids::CharacterId(7),
                text_token: 12345,
                text: None,
            }]
        );
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_text_message_preserves_optional_text_payload() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        character.push_driver_text_message(crate::ids::CharacterId(7), "Tabunga please");

        let outcomes = process_simple_baddy_messages(&mut character, &[]);

        assert_eq!(
            outcomes,
            vec![SimpleBaddyMessageOutcome::TextNotification {
                speaker_id: crate::ids::CharacterId(7),
                text_token: 0,
                text: Some("Tabunga please".to_string()),
            }]
        );
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_didhit_emits_poison_hit_outcome() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            poison_power: 7,
            poison_type: 2,
            poison_chance: 35,
            ..SimpleBaddyDriverData::default()
        }));
        character.push_driver_message(NT_DIDHIT, 42, 3, 0);

        let outcomes = process_simple_baddy_messages(&mut character, &[]);

        assert_eq!(
            outcomes,
            vec![SimpleBaddyMessageOutcome::PoisonHit {
                target_id: crate::ids::CharacterId(42),
                power: 7,
                poison_type: 2,
                chance: 35,
            }]
        );
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_didhit_requires_power_target_and_damage() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            poison_power: 7,
            poison_type: 2,
            poison_chance: 100,
            ..SimpleBaddyDriverData::default()
        }));
        character.push_driver_message(NT_DIDHIT, 0, 3, 0);
        character.push_driver_message(NT_DIDHIT, 42, 0, 0);

        assert!(process_simple_baddy_messages(&mut character, &[]).is_empty());
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_npc_message_emits_helpid_enemy_outcome() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            helpid: NTID_GLADIATOR,
            ..SimpleBaddyDriverData::default()
        }));
        character.push_driver_message(NT_NPC, NTID_MERCHANT, 2, 99);
        character.push_driver_message(NT_NPC, NTID_GLADIATOR, 2, 99);

        let outcomes = process_simple_baddy_messages(&mut character, &[]);

        assert_eq!(
            outcomes,
            vec![SimpleBaddyMessageOutcome::AddEnemy {
                caller_id: crate::ids::CharacterId(2),
                target_id: crate::ids::CharacterId(99),
            }]
        );
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_npc_message_preserves_zero_target_like_c() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            helpid: NTID_GLADIATOR,
            ..SimpleBaddyDriverData::default()
        }));
        character.push_driver_message(NT_NPC, NTID_GLADIATOR, 2, 0);

        let outcomes = process_simple_baddy_messages(&mut character, &[]);

        assert_eq!(
            outcomes,
            vec![SimpleBaddyMessageOutcome::AddEnemy {
                caller_id: crate::ids::CharacterId(2),
                target_id: crate::ids::CharacterId(0),
            }]
        );
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn simple_baddy_dead_message_emits_remove_enemy_outcome() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        character.push_driver_message(NT_DEAD, 42, 7, 0);
        character.push_driver_message(NT_DEAD, 0, 7, 0);

        let outcomes = process_simple_baddy_messages(&mut character, &[]);

        assert_eq!(
            outcomes,
            vec![SimpleBaddyMessageOutcome::RemoveEnemy {
                target_id: crate::ids::CharacterId(42),
            }]
        );
        assert!(character.driver_messages.is_empty());
    }

    #[test]
    fn add_simple_baddy_enemy_requires_same_group_caller_and_updates_existing() {
        let mut character = test_character();
        character.group = 7;
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));
        let mut caller = test_character();
        caller.id = crate::ids::CharacterId(2);
        caller.group = 8;

        assert!(!add_simple_baddy_enemy(
            &mut character,
            &caller,
            crate::ids::CharacterId(99),
            10,
        ));

        caller.group = 7;
        assert!(add_simple_baddy_enemy(
            &mut character,
            &caller,
            crate::ids::CharacterId(99),
            10,
        ));
        assert!(!add_simple_baddy_enemy(
            &mut character,
            &caller,
            crate::ids::CharacterId(99),
            12,
        ));

        let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state else {
            panic!("simple baddy state missing");
        };
        assert_eq!(
            data.enemies,
            vec![SimpleBaddyEnemy {
                target_id: crate::ids::CharacterId(99),
                priority: 1,
                last_seen_tick: 12,
                visible: false,
                last_x: 0,
                last_y: 0,
            }]
        );
    }

    #[test]
    fn add_simple_baddy_enemy_keeps_legacy_ten_entry_table() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));

        for target in 10..22 {
            assert!(add_simple_baddy_enemy_unchecked(
                &mut character,
                crate::ids::CharacterId(target),
                0,
                target as i32,
            ));
        }

        let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.enemies.len(), 10);
        assert_eq!(data.enemies[0].target_id, crate::ids::CharacterId(10));
        assert_eq!(data.enemies[8].target_id, crate::ids::CharacterId(18));
        assert_eq!(data.enemies[9].target_id, crate::ids::CharacterId(21));
    }

    #[test]
    fn add_simple_baddy_enemy_matches_c_slot_nine_overflow_semantics() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));

        for target in 1..=10 {
            assert!(add_simple_baddy_enemy_unchecked(
                &mut character,
                crate::ids::CharacterId(target),
                0,
                target as i32,
            ));
        }

        assert!(add_simple_baddy_enemy_unchecked(
            &mut character,
            crate::ids::CharacterId(10),
            1,
            99,
        ));

        let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.enemies.len(), 10);
        assert_eq!(data.enemies[9].target_id, crate::ids::CharacterId(10));
        assert_eq!(data.enemies[9].priority, 1);
        assert_eq!(data.enemies[9].last_seen_tick, 99);
    }

    #[test]
    fn add_simple_baddy_enemy_overwrites_priority_like_c_hurtme_flag() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(
            SimpleBaddyDriverData::default(),
        ));

        assert!(add_simple_baddy_enemy_unchecked(
            &mut character,
            crate::ids::CharacterId(2),
            1,
            10,
        ));
        assert!(!add_simple_baddy_enemy_unchecked(
            &mut character,
            crate::ids::CharacterId(2),
            0,
            11,
        ));

        let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.enemies[0].priority, 0);
        assert_eq!(data.enemies[0].last_seen_tick, 11);
    }

    #[test]
    fn remove_simple_baddy_enemy_matches_fight_driver_remove_boundary() {
        let mut character = test_character();
        character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
            enemies: vec![
                SimpleBaddyEnemy {
                    target_id: crate::ids::CharacterId(2),
                    priority: 0,
                    last_seen_tick: 10,
                    visible: true,
                    last_x: 20,
                    last_y: 21,
                },
                SimpleBaddyEnemy {
                    target_id: crate::ids::CharacterId(3),
                    priority: 1,
                    last_seen_tick: 11,
                    visible: false,
                    last_x: 30,
                    last_y: 31,
                },
            ],
            ..SimpleBaddyDriverData::default()
        }));

        assert!(remove_simple_baddy_enemy(
            &mut character,
            crate::ids::CharacterId(2),
        ));
        assert!(!remove_simple_baddy_enemy(
            &mut character,
            crate::ids::CharacterId(99),
        ));

        let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state else {
            panic!("simple baddy state missing");
        };
        assert_eq!(data.enemies.len(), 1);
        assert_eq!(data.enemies[0].target_id, crate::ids::CharacterId(3));
    }

    #[test]
    fn remove_simple_baddy_enemy_ignores_non_simple_baddy_state() {
        let mut character = test_character();

        assert!(!remove_simple_baddy_enemy(
            &mut character,
            crate::ids::CharacterId(2),
        ));
    }

    #[test]
    fn clara_dialogue_ports_initial_report_state_machine() {
        let outcome = clara_dialogue_step(clara_context(0, 0));
        assert_eq!(outcome.clara_state, 1);
        assert_eq!(
            outcome.text.as_deref(),
            Some(
                "Greetings, Hero! I am Clara, First Sergeant of the Seyan'Du and commander of this outpost."
            )
        );

        let blocked = clara_dialogue_step(clara_context(1, 14));
        assert_eq!(blocked.clara_state, 1);
        assert_eq!(blocked.text, None);

        let report = clara_dialogue_step(clara_context(1, 15));
        assert_eq!(report.clara_state, 3);
        assert_eq!(
            report.text.as_deref(),
            Some(
                "I assume thou hast been sent from Aston, Private, to report on our status. The road through the swamp is no longer secure and we have been under attack from beasts emerging from the swamp."
            )
        );

        let dismissed = clara_dialogue_step(clara_context(4, 15));
        assert_eq!(dismissed.clara_state, 5);
        assert_eq!(
            dismissed.text.as_deref(),
            Some(
                "Afterwards come back here, I have more work for thee. That will be all, Private. Dismissed!"
            )
        );
    }

    #[test]
    fn clara_dialogue_ports_hardkill_quest_gates_and_rewards() {
        let blocked = clara_dialogue_step(clara_context(5, 17));
        assert_eq!(blocked.clara_state, 5);
        assert_eq!(blocked.text, None);

        let mission = clara_dialogue_step(clara_context(5, 18));
        assert_eq!(mission.clara_state, 7);
        assert_eq!(mission.open_questlog, Some(21));
        assert_eq!(
            mission.text.as_deref(),
            Some(
                "I have a difficult mission for thee, Hero. The main reason we had to retreat to this camp was one huge swamp beast. It seemed to be immune to our attacks."
            )
        );

        let no_hardkill = clara_dialogue_step(clara_context(9, 18));
        assert_eq!(no_hardkill.clara_state, 9);
        assert_eq!(no_hardkill.text, None);

        let mut context = clara_context(9, 18);
        context.has_hardkill_item = true;
        context.hardkill_ritual_progress = 24;
        let partial_ritual = clara_dialogue_step(context);
        assert_eq!(partial_ritual.clara_state, 11);
        assert_eq!(partial_ritual.military_points, 4);
        assert_eq!(partial_ritual.military_exp, EXP_AREA15_HARDKILL);
        assert_eq!(
            partial_ritual.text.as_deref(),
            Some(
                "So that is how one can kill them. Thou wilt need to find all three stone circles and perform the ritual in each one, then, Hero."
            )
        );

        let mut context = clara_context(11, 18);
        context.has_hardkill_item = true;
        context.hardkill_ritual_progress = 36;
        let ready_to_kill = clara_dialogue_step(context);
        assert_eq!(ready_to_kill.clara_state, 13);
        assert_eq!(
            ready_to_kill.text.as_deref(),
            Some("Now that thou knowest how to kill that beast, please go and do it.")
        );

        let mut context = clara_context(14, 18);
        context.questlog_21_count = 1;
        let done = clara_dialogue_step(context);
        assert_eq!(done.clara_state, 15);
        assert_eq!(done.complete_questlog, Some(21));
        assert_eq!(done.military_points, 8);
        assert_eq!(done.military_exp, 1);
        assert_eq!(done.text.as_deref(), Some("Well done indeed, Hero!"));
    }

    #[test]
    fn clara_replay_and_monster_death_match_c_state_boundaries() {
        assert_eq!(clara_replay_state_after_text_analysis(5, 2), 0);
        assert_eq!(clara_replay_state_after_text_analysis(9, 2), 6);
        assert_eq!(clara_replay_state_after_text_analysis(11, 2), 10);
        assert_eq!(clara_replay_state_after_text_analysis(13, 2), 12);
        assert_eq!(clara_replay_state_after_text_analysis(16, 2), 15);
        assert_eq!(clara_replay_state_after_text_analysis(14, 2), 14);
        assert_eq!(clara_replay_state_after_text_analysis(13, 1), 13);

        assert_eq!(clara_state_after_swamp_monster_death(12, true, true), 14);
        assert_eq!(clara_state_after_swamp_monster_death(13, true, true), 14);
        assert_eq!(clara_state_after_swamp_monster_death(11, true, true), 11);
        assert_eq!(clara_state_after_swamp_monster_death(12, false, true), 12);
        assert_eq!(clara_state_after_swamp_monster_death(12, true, false), 12);
    }

    fn test_character() -> Character {
        Character {
            merchant: None,
            template_key: String::new(),
            respawn_ticks: 0,
            id: crate::ids::CharacterId(1),
            serial: 1,
            name: "Rat".to_string(),
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
            staff_code: String::new(),
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
            military_points: 0,
            military_normal_exp: 0,
            gold: 0,
            karma: 0,
            creation_time: 0,
            saves: 0,
            got_saved: 0,
            deaths: 0,
            regen_ticker: 0,
            last_regen: 0,
            cursor_item: None,
            current_container: None,
            values: Character::empty_values(),
            professions: Character::empty_professions(),
            inventory: Character::empty_inventory(),
            driver_state: None,
            driver_messages: Vec::new(),
        }
    }

    fn clara_context(clara_state: i32, kelly_state: i32) -> ClaraDialogueContext<'static> {
        ClaraDialogueContext {
            player_name: "Hero",
            clara_name: "Clara",
            army_rank: "Private",
            kelly_state,
            clara_state,
            has_hardkill_item: false,
            hardkill_ritual_progress: 0,
            questlog_21_count: 0,
        }
    }

    fn test_item(id: ItemId, driver: u16, driver_data: &[u8]) -> Item {
        Item {
            id,
            name: String::new(),
            description: String::new(),
            flags: ItemFlags::USED,
            sprite: 0,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0,
            owner_id: 0,
            modifier_index: [0; crate::entity::MAX_MODIFIERS],
            modifier_value: [0; crate::entity::MAX_MODIFIERS],
            x: 0,
            y: 0,
            carried_by: None,
            contained_in: None,
            content_id: 0,
            driver,
            driver_data: driver_data.to_vec(),
            serial: 0,
        }
    }
}
