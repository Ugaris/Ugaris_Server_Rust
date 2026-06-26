use serde::{Deserialize, Serialize};

use crate::{
    do_action::ItemUseRequest,
    entity::{Character, CharacterFlags, CharacterValue, Item, ItemFlags, POWERSCALE},
    ids::{CharacterId, ItemId},
    item_ops::consume_item,
    legacy::action,
};

pub const IDR_POTION: u16 = 1;
pub const IDR_DOOR: u16 = 2;
pub const IDR_CHEST: u16 = 5;
pub const IDR_TELEPORT: u16 = 10;
pub const IDR_RECALL: u16 = 13;
pub const IDR_TELE_DOOR: u16 = 31;
pub const IDR_RANDCHEST: u16 = 34;
pub const IDR_FOOD: u16 = 64;
pub const IDR_ACCOUNT_DEPOT: u16 = 148;
pub const IDR_DOUBLE_DOOR: u16 = 187;

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
    DoorToggle {
        item_id: ItemId,
        character_id: CharacterId,
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
                IDR_DOOR => door_driver(character, item),
                IDR_CHEST => chest_driver(character, item),
                IDR_RANDCHEST => randchest_driver(character, item),
                IDR_RECALL => recall_driver(character, item, area_id, in_arena),
                IDR_TELE_DOOR => teleport_door_driver(character, item),
                IDR_TELEPORT => teleport_driver(character, item),
                IDR_FOOD => food_driver(character, item),
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

fn door_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if item.x == 0 {
        return ItemDriverOutcome::Noop;
    }

    if door_required_key_id(item) != 0 {
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
