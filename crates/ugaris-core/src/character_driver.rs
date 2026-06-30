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

pub const CDR_SIMPLEBADDY: u16 = 7;
pub const CDR_MACRO: u16 = 37;
pub const CDR_TRADER: u16 = 72;
pub const CDR_JANITOR: u16 = 85;

pub const DRD_SIMPLEBADDYDRIVER: u32 = 0x0100_0013;

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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CharacterDriverMessage {
    pub message_type: i32,
    pub dat1: i32,
    pub dat2: i32,
    pub dat3: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CharacterDriverState {
    SimpleBaddy(SimpleBaddyDriverData),
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
    let mut bless_friend = None;

    let messages = std::mem::take(&mut character.driver_messages);
    for message in messages {
        if message.message_type == NT_CHAR && helper != 0 && message.dat1 > 0 {
            bless_friend = Some(crate::ids::CharacterId(message.dat1 as u32));
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
            && message.dat3 > 0
        {
            outcomes.push(SimpleBaddyMessageOutcome::AddEnemy {
                caller_id: CharacterId(message.dat2 as u32),
                target_id: CharacterId(message.dat3 as u32),
            });
        }
    }

    if let Some(target_id) = bless_friend {
        outcomes.push(SimpleBaddyMessageOutcome::BlessFriend { target_id });
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
        .find(|enemy| enemy.target_id == target_id)
    {
        enemy.priority = enemy.priority.max(priority);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterDriverKind {
    SimpleBaddy,
    Macro,
    Trader,
    Janitor,
}

impl CharacterDriverKind {
    pub fn from_legacy_id(driver: u16) -> Option<Self> {
        match driver {
            CDR_SIMPLEBADDY => Some(Self::SimpleBaddy),
            CDR_MACRO => Some(Self::Macro),
            CDR_TRADER => Some(Self::Trader),
            CDR_JANITOR => Some(Self::Janitor),
            _ => None,
        }
    }

    pub fn legacy_id(self) -> u16 {
        match self {
            Self::SimpleBaddy => CDR_SIMPLEBADDY,
            Self::Macro => CDR_MACRO,
            Self::Trader => CDR_TRADER,
            Self::Janitor => CDR_JANITOR,
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
        assert_eq!(CDR_SIMPLEBADDY, 7);
        assert_eq!(CDR_MACRO, 37);
        assert_eq!(CDR_TRADER, 72);
        assert_eq!(CDR_JANITOR, 85);
        assert_eq!(DRD_SIMPLEBADDYDRIVER, 0x0100_0013);
        assert_eq!(
            CharacterDriverKind::SimpleBaddy.legacy_id(),
            CDR_SIMPLEBADDY
        );
        assert_eq!(CharacterDriverKind::Macro.legacy_id(), CDR_MACRO);
        assert_eq!(CharacterDriverKind::Trader.legacy_id(), CDR_TRADER);
        assert_eq!(CharacterDriverKind::Janitor.legacy_id(), CDR_JANITOR);
    }

    #[test]
    fn known_base_tick_drivers_are_handled_like_c_ch_driver() {
        for (driver, kind) in [
            (CDR_SIMPLEBADDY, CharacterDriverKind::SimpleBaddy),
            (CDR_MACRO, CharacterDriverKind::Macro),
            (CDR_TRADER, CharacterDriverKind::Trader),
            (CDR_JANITOR, CharacterDriverKind::Janitor),
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
            CharacterDriverOutcome::HandledStub {
                kind: CharacterDriverKind::SimpleBaddy,
                call: CharacterDriverCall::Died {
                    killer_character_id: 123,
                },
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
    fn simple_baddy_char_message_selects_last_helper_bless_target() {
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
            vec![SimpleBaddyMessageOutcome::BlessFriend {
                target_id: crate::ids::CharacterId(3),
            }]
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
                SimpleBaddyMessageOutcome::BlessFriend {
                    target_id: crate::ids::CharacterId(2),
                },
            ]
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

    fn test_character() -> Character {
        Character {
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
