use serde::{Deserialize, Serialize};

use crate::{
    do_action::ItemUseRequest,
    entity::{
        Character, CharacterFlags, CharacterValue, Item, ItemFlags, CHARACTER_VALUE_COUNT,
        POWERSCALE,
    },
    ids::{CharacterId, ItemId},
    item_ops::consume_item,
    legacy::action,
};

pub const IDR_POTION: u16 = 1;
pub const IDR_DOOR: u16 = 2;
pub const IDR_CHEST: u16 = 5;
pub const IDR_TELEPORT: u16 = 10;
pub const IDR_RECALL: u16 = 13;
pub const IDR_STATSCROLL: u16 = 19;
pub const IDR_ASSEMBLE: u16 = 29;
pub const IDR_TELE_DOOR: u16 = 31;
pub const IDR_RANDCHEST: u16 = 34;
pub const IDR_FOOD: u16 = 64;
pub const IDR_ACCOUNT_DEPOT: u16 = 148;
pub const IDR_CITY_RECALL: u16 = 159;
pub const IDR_DOUBLE_DOOR: u16 = 187;
pub const IDR_KEY_RING: u16 = 200;
pub const IID_SKELETON_KEY: u32 = (59 << 24) | 0x000003;

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

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemDriverContext {
    pub door_key: Option<DoorKeyAccess>,
    pub cursor_template_id: Option<u32>,
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
    ChestTreasure {
        item_id: ItemId,
        character_id: CharacterId,
        treasure_index: u8,
    },
    RandomChest {
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
    Noop,
    Unsupported {
        driver: u16,
        item_id: ItemId,
        character_id: CharacterId,
    },
    UnsupportedSpecialFood {
        item_id: ItemId,
        character_id: CharacterId,
        kind: u8,
    },
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
        return Ok(UseItemOutcome::Dispatch(ItemDriverRequest::AccountDepot {
            item_id: item.id,
            character_id: character.id,
        }));
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
            ..
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
                IDR_CHEST => chest_driver(character, item),
                IDR_RANDCHEST => randchest_driver(character, item),
                IDR_RECALL => recall_driver(character, item, area_id, in_arena),
                IDR_STATSCROLL => stat_scroll_driver(character, item),
                IDR_ASSEMBLE => assemble_driver(character, item, context),
                IDR_CITY_RECALL => city_recall_driver(character, item, area_id, in_arena),
                IDR_TELE_DOOR => teleport_door_driver(character, item),
                IDR_TELEPORT => teleport_driver(character, item),
                IDR_FOOD => food_driver(character, item),
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
        } => ItemDriverOutcome::Unsupported {
            driver: IDR_ACCOUNT_DEPOT,
            item_id,
            character_id,
        },
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
        11..=42 => 1,
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
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if item.x == 0 {
        return ItemDriverOutcome::Noop;
    }

    let required_key_id = door_required_key_id(item);
    if required_key_id != 0 {
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

fn food_driver(character: &mut Character, item: &mut Item) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    let kind = drdata(item, 0);
    if kind == 2 || kind == 3 {
        return ItemDriverOutcome::UnsupportedSpecialFood {
            item_id: item.id,
            character_id: character.id,
            kind,
        };
    }

    consume_item(character, item);
    ItemDriverOutcome::FoodEaten {
        item_id: item.id,
        character_id: character.id,
        kind,
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
            UseItemOutcome::Dispatch(ItemDriverRequest::AccountDepot {
                item_id: ItemId(8),
                character_id: CharacterId(1),
            })
        );
        assert_eq!(character.current_container, Some(ItemId(8)));
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
    fn execute_food_driver_consumes_simple_food_and_defers_special_food() {
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

        character.cursor_item = Some(ItemId(8));
        let mut special = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_FOOD);
        special.carried_by = Some(CharacterId(1));
        special.driver_data = vec![3];
        assert_eq!(
            execute_item_driver(
                &mut character,
                &mut special,
                ItemDriverRequest::Driver {
                    driver: IDR_FOOD,
                    item_id: ItemId(8),
                    character_id: CharacterId(1),
                    spec: 0,
                },
                1,
                false,
            ),
            ItemDriverOutcome::UnsupportedSpecialFood {
                item_id: ItemId(8),
                character_id: CharacterId(1),
                kind: 3,
            }
        );
        assert!(special.flags.contains(ItemFlags::USED));
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
            deaths: 0,
            cursor_item: None,
            current_container: None,
            values: Character::empty_values(),
            professions: Character::empty_professions(),
            inventory: Character::empty_inventory(),
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
