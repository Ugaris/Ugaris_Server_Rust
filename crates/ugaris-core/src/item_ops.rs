use serde::{Deserialize, Serialize};

use crate::{
    entity::{Character, CharacterFlags, Item, ItemFlags, INVENTORY_SIZE},
    legacy::{INVENTORY_LAST_SPELLS, INVENTORY_START_INVENTORY, INVENTORY_START_SPELLS},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i8)]
pub enum GiveItemResult {
    Full = -1,
    Failed = 0,
    Ok = 1,
    Dropped = 2,
    Money = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GiveItemFlags(u8);

impl GiveItemFlags {
    pub const LOG: Self = Self(1 << 0);
    pub const ALLOW_DROP: Self = Self(1 << 1);
    pub const NONE: Self = Self(0);

    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

pub fn has_inventory_space(character: &Character) -> bool {
    character.cursor_item.is_none() || count_free_inventory_slots(character) > 0
}

pub fn count_free_inventory_slots(character: &Character) -> usize {
    character
        .inventory
        .iter()
        .skip(INVENTORY_START_INVENTORY)
        .filter(|slot| slot.is_none())
        .count()
}

pub fn can_use_inventory_slot(slot: usize) -> bool {
    slot < INVENTORY_SIZE && !(INVENTORY_START_SPELLS..=INVENTORY_LAST_SPELLS).contains(&slot)
}

pub fn give_item_to_character(
    character: &mut Character,
    item: &mut Item,
    flags: GiveItemFlags,
) -> GiveItemResult {
    if !character.flags.contains(CharacterFlags::USED) || !item.flags.contains(ItemFlags::USED) {
        return GiveItemResult::Failed;
    }

    if item.flags.contains(ItemFlags::MONEY) {
        character.gold = character.gold.saturating_add(item.value);
        item.flags.remove(ItemFlags::USED);
        item.carried_by = None;
        return GiveItemResult::Money;
    }

    if let Some(slot) = character
        .inventory
        .iter_mut()
        .skip(INVENTORY_START_INVENTORY)
        .find(|slot| slot.is_none())
    {
        *slot = Some(item.id);
        item.carried_by = Some(character.id);
        character.flags.insert(CharacterFlags::ITEMS);
        return GiveItemResult::Ok;
    }

    if character.cursor_item.is_none() {
        character.cursor_item = Some(item.id);
        item.carried_by = Some(character.id);
        character.flags.insert(CharacterFlags::ITEMS);
        return GiveItemResult::Ok;
    }

    if !flags.contains(GiveItemFlags::ALLOW_DROP) {
        return GiveItemResult::Full;
    }

    if item.flags.contains(ItemFlags::NODROP) {
        item.flags.remove(ItemFlags::USED);
        return GiveItemResult::Failed;
    }

    item.carried_by = None;
    item.x = character.x;
    item.y = character.y;
    GiveItemResult::Dropped
}

pub fn consume_item(character: &mut Character, item: &mut Item) -> bool {
    if character.cursor_item == Some(item.id) {
        character.cursor_item = None;
    }
    for slot in &mut character.inventory {
        if *slot == Some(item.id) {
            *slot = None;
        }
    }
    item.carried_by = None;
    item.flags.remove(ItemFlags::USED);
    character.flags.insert(CharacterFlags::ITEMS);
    true
}

pub fn remove_item_from_character(character: &mut Character, item: &mut Item) -> bool {
    if item.carried_by != Some(character.id) {
        return false;
    }

    if character.cursor_item == Some(item.id) {
        character.cursor_item = None;
        character.flags.insert(CharacterFlags::ITEMS);
        item.carried_by = None;
        return true;
    }

    if let Some(slot) = character
        .inventory
        .iter_mut()
        .find(|slot| **slot == Some(item.id))
    {
        *slot = None;
    }

    character.flags.insert(CharacterFlags::ITEMS);
    item.carried_by = None;
    true
}

pub fn replace_item_in_character(
    character: &mut Character,
    old_item: &mut Item,
    new_item: &mut Item,
) -> bool {
    if old_item.carried_by != Some(character.id) {
        return false;
    }

    if character.cursor_item == Some(old_item.id) {
        character.cursor_item = Some(new_item.id);
        character.flags.insert(CharacterFlags::ITEMS);
        old_item.carried_by = None;
        new_item.carried_by = Some(character.id);
        return true;
    }

    let Some(slot) = character
        .inventory
        .iter_mut()
        .find(|slot| **slot == Some(old_item.id))
    else {
        return false;
    };

    *slot = Some(new_item.id);
    character.flags.insert(CharacterFlags::ITEMS);
    old_item.carried_by = None;
    new_item.carried_by = Some(character.id);
    true
}

#[cfg(test)]
mod tests {
    use crate::{
        entity::{CharacterValue, SpeedMode, MAX_MODIFIERS},
        ids::{CharacterId, ItemId},
    };

    use super::*;

    fn character() -> Character {
        Character {
            merchant: None,
            template_key: String::new(),
            respawn_ticks: 0,
            id: CharacterId(1),
            serial: 1,
            name: "Tester".into(),
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
            x: 10,
            y: 10,
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
            driver_memory: crate::character_driver::DriverMemory::default(),
            class: 0,
            dungeonfighter: None,
        }
    }

    fn item(id: u32, flags: ItemFlags) -> Item {
        Item {
            id: ItemId(id),
            name: "Item".into(),
            description: String::new(),
            flags: flags | ItemFlags::USED,
            sprite: 0,
            value: 5,
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
            content_id: 0,
            driver: 0,
            driver_data: Vec::new(),
            serial: 0,
        }
    }

    #[test]
    fn result_codes_match_c_header() {
        assert_eq!(GiveItemResult::Ok as i8, 1);
        assert_eq!(GiveItemResult::Dropped as i8, 2);
        assert_eq!(GiveItemResult::Money as i8, 3);
        assert_eq!(GiveItemResult::Full as i8, -1);
        assert_eq!(GiveItemResult::Failed as i8, 0);
        assert_eq!(CharacterValue::Profession as u8, 42);
    }

    #[test]
    fn gives_item_to_inventory_slot_30_plus() {
        let mut character = character();
        let mut item = item(7, ItemFlags::empty());
        assert_eq!(
            give_item_to_character(&mut character, &mut item, GiveItemFlags::NONE),
            GiveItemResult::Ok
        );
        assert_eq!(
            character.inventory[INVENTORY_START_INVENTORY],
            Some(ItemId(7))
        );
        assert_eq!(item.carried_by, Some(CharacterId(1)));
    }

    #[test]
    fn money_item_adds_gold_and_consumes_item() {
        let mut character = character();
        let mut item = item(7, ItemFlags::MONEY);
        assert_eq!(
            give_item_to_character(&mut character, &mut item, GiveItemFlags::NONE),
            GiveItemResult::Money
        );
        assert_eq!(character.gold, 5);
        assert!(!item.flags.contains(ItemFlags::USED));
    }

    #[test]
    fn full_inventory_without_drop_keeps_item() {
        let mut character = character();
        character.cursor_item = Some(ItemId(99));
        for slot in character
            .inventory
            .iter_mut()
            .skip(INVENTORY_START_INVENTORY)
        {
            *slot = Some(ItemId(1));
        }
        let mut item = item(7, ItemFlags::empty());
        assert_eq!(
            give_item_to_character(&mut character, &mut item, GiveItemFlags::NONE),
            GiveItemResult::Full
        );
        assert!(item.flags.contains(ItemFlags::USED));
    }

    #[test]
    fn remove_item_from_character_handles_cursor_and_inventory() {
        let mut character = character();
        let mut cursor = item(8, ItemFlags::empty());
        cursor.carried_by = Some(character.id);
        character.cursor_item = Some(cursor.id);

        assert!(remove_item_from_character(&mut character, &mut cursor));
        assert_eq!(character.cursor_item, None);
        assert_eq!(cursor.carried_by, None);
        assert!(character.flags.contains(CharacterFlags::ITEMS));

        let mut inventory = item(9, ItemFlags::empty());
        inventory.carried_by = Some(character.id);
        character.inventory[INVENTORY_START_INVENTORY] = Some(inventory.id);

        assert!(remove_item_from_character(&mut character, &mut inventory));
        assert_eq!(character.inventory[INVENTORY_START_INVENTORY], None);
        assert_eq!(inventory.carried_by, None);
    }

    #[test]
    fn replace_item_in_character_updates_cursor_or_inventory_slot() {
        let mut character = character();
        let mut old_item = item(8, ItemFlags::empty());
        let mut new_item = item(9, ItemFlags::empty());
        old_item.carried_by = Some(character.id);
        character.cursor_item = Some(old_item.id);

        assert!(replace_item_in_character(
            &mut character,
            &mut old_item,
            &mut new_item
        ));
        assert_eq!(character.cursor_item, Some(new_item.id));
        assert_eq!(old_item.carried_by, None);
        assert_eq!(new_item.carried_by, Some(character.id));

        let mut old_item = item(10, ItemFlags::empty());
        let mut new_item = item(11, ItemFlags::empty());
        old_item.carried_by = Some(character.id);
        character.inventory[INVENTORY_START_INVENTORY + 1] = Some(old_item.id);

        assert!(replace_item_in_character(
            &mut character,
            &mut old_item,
            &mut new_item
        ));
        assert_eq!(
            character.inventory[INVENTORY_START_INVENTORY + 1],
            Some(new_item.id)
        );
    }
}
