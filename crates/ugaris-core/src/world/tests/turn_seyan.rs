use super::*;
use crate::entity::CHARACTER_VALUE_COUNT;
use crate::legacy::{INVENTORY_LAST_SPELLS, INVENTORY_SIZE, INVENTORY_START_SPELLS};

fn seyan_base_values(hp: i16) -> Vec<i16> {
    let mut values = vec![0i16; CHARACTER_VALUE_COUNT];
    values[CharacterValue::Hp as usize] = hp;
    values
}

#[test]
fn apply_turn_seyan_resets_stats_exp_level_and_professions() {
    let mut world = World::default();
    let mut victim = character(2);
    victim.level = 40;
    victim.exp = 500_000;
    victim.exp_used = 400_000;
    victim.lifeshield = 5;
    victim.professions[0] = 3;
    victim.professions[1] = 7;
    assert!(world.spawn_character(victim, 10, 10));

    let base_values = seyan_base_values(10);
    assert!(world.apply_turn_seyan(CharacterId(2), &base_values));

    let victim = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(victim.level, 1);
    assert_eq!(victim.exp, 0);
    assert_eq!(victim.exp_used, 0);
    assert_eq!(victim.lifeshield, 0);
    assert!(victim.professions.iter().all(|profession| *profession == 0));
    assert_eq!(victim.values[1][CharacterValue::Hp as usize], 10);
}

#[test]
fn apply_turn_seyan_sets_mage_warrior_items_flags() {
    let mut world = World::default();
    let victim = character(2);
    assert!(world.spawn_character(victim, 10, 10));

    let base_values = seyan_base_values(10);
    assert!(world.apply_turn_seyan(CharacterId(2), &base_values));

    let victim = world.characters.get(&CharacterId(2)).unwrap();
    assert!(victim.flags.contains(CharacterFlags::MAGE));
    assert!(victim.flags.contains(CharacterFlags::WARRIOR));
    assert!(victim.flags.contains(CharacterFlags::ITEMS));
}

#[test]
fn apply_turn_seyan_moves_worn_item_into_first_free_inventory_slot() {
    let mut world = World::default();
    let mut victim = character(2);
    victim.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(50));
    world.items.insert(ItemId(50), item(50, ItemFlags::empty()));
    assert!(world.spawn_character(victim, 10, 10));

    let base_values = seyan_base_values(10);
    assert!(world.apply_turn_seyan(CharacterId(2), &base_values));

    let victim = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(victim.inventory[worn_slot::RIGHT_HAND], None);
    assert_eq!(victim.inventory[30], Some(ItemId(50)));
    assert!(world.items.contains_key(&ItemId(50)));
}

#[test]
fn apply_turn_seyan_destroys_worn_item_when_inventory_is_completely_full() {
    let mut world = World::default();
    let mut victim = character(2);
    victim.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(50));
    world.items.insert(ItemId(50), item(50, ItemFlags::empty()));
    for slot in 30..INVENTORY_SIZE {
        let item_id = ItemId(1000 + slot as u32);
        victim.inventory[slot] = Some(item_id);
        world
            .items
            .insert(item_id, item(item_id.0, ItemFlags::empty()));
    }
    assert!(world.spawn_character(victim, 10, 10));

    let base_values = seyan_base_values(10);
    assert!(world.apply_turn_seyan(CharacterId(2), &base_values));

    let victim = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(victim.inventory[worn_slot::RIGHT_HAND], None);
    // No free slot existed, so C's `free_item(in)` destroyed it instead of
    // moving it.
    assert!(!world.items.contains_key(&ItemId(50)));
}

#[test]
fn apply_turn_seyan_destroys_spell_slot_items() {
    let mut world = World::default();
    let mut victim = character(2);
    victim.inventory[INVENTORY_START_SPELLS] = Some(ItemId(60));
    victim.inventory[INVENTORY_LAST_SPELLS] = Some(ItemId(61));
    world.items.insert(ItemId(60), item(60, ItemFlags::empty()));
    world.items.insert(ItemId(61), item(61, ItemFlags::empty()));
    assert!(world.spawn_character(victim, 10, 10));

    let base_values = seyan_base_values(10);
    assert!(world.apply_turn_seyan(CharacterId(2), &base_values));

    let victim = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(victim.inventory[INVENTORY_START_SPELLS], None);
    assert_eq!(victim.inventory[INVENTORY_LAST_SPELLS], None);
    assert!(!world.items.contains_key(&ItemId(60)));
    assert!(!world.items.contains_key(&ItemId(61)));
}

#[test]
fn apply_turn_seyan_strips_quest_items_from_remaining_inventory() {
    let mut world = World::default();
    let mut victim = character(2);
    victim.inventory[30] = Some(ItemId(70));
    victim.inventory[31] = Some(ItemId(71));
    world.items.insert(ItemId(70), item(70, ItemFlags::QUEST));
    world.items.insert(ItemId(71), item(71, ItemFlags::empty()));
    assert!(world.spawn_character(victim, 10, 10));

    let base_values = seyan_base_values(10);
    assert!(world.apply_turn_seyan(CharacterId(2), &base_values));

    let victim = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(victim.inventory[30], None);
    assert_eq!(victim.inventory[31], Some(ItemId(71)));
    assert!(!world.items.contains_key(&ItemId(70)));
    assert!(world.items.contains_key(&ItemId(71)));
}

#[test]
fn apply_turn_seyan_clamps_hp_endurance_mana_to_new_recomputed_max() {
    let mut world = World::default();
    let mut victim = character(2);
    victim.flags |= CharacterFlags::USED;
    victim.hp = 200_000;
    victim.endurance = 200_000;
    victim.mana = 200_000;
    victim.values[0][CharacterValue::Hp as usize] = 200;
    assert!(world.spawn_character(victim, 10, 10));

    // A fresh level-1 base HP of 10 recomputes to a much smaller max than
    // the old (pre-reroll) character's 200_000 pool, so
    // `update_character`'s clamp (C's `update_char`) brings it back down.
    let base_values = seyan_base_values(10);
    assert!(world.apply_turn_seyan(CharacterId(2), &base_values));

    let victim = world.characters.get(&CharacterId(2)).unwrap();
    assert!(victim.hp < 200_000);
    assert!(victim.endurance < 200_000);
    assert!(victim.mana < 200_000);
}

#[test]
fn apply_turn_seyan_returns_false_for_missing_character() {
    let mut world = World::default();
    let base_values = seyan_base_values(10);
    assert!(!world.apply_turn_seyan(CharacterId(99), &base_values));
}

#[test]
fn apply_turn_seyan_returns_false_for_mismatched_base_value_length() {
    let mut world = World::default();
    let victim = character(2);
    assert!(world.spawn_character(victim, 10, 10));

    let short_base_values = vec![0i16; 3];
    assert!(!world.apply_turn_seyan(CharacterId(2), &short_base_values));
}
