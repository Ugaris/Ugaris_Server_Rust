use super::*;
use crate::item_driver::IDR_BONEHOLDER;

#[test]
fn update_bone_holder_sprite_matches_c_empty_and_filled_cases() {
    let mut world = World::default();
    let mut holder = item(1, ItemFlags::USED);
    assert!(world.map.set_item_map(&mut holder, 10, 10));
    holder.driver_data = vec![0, 0];
    world.add_item(holder);

    world.update_bone_holder_sprite(ItemId(1));
    assert_eq!(world.items[&ItemId(1)].sprite, 13103);
    assert_eq!(world.map.tile(10, 10).unwrap().foreground_sprite, 0);

    world.items.get_mut(&ItemId(1)).unwrap().driver_data[1] = 2; // activation stand
    world.update_bone_holder_sprite(ItemId(1));
    assert_eq!(world.items[&ItemId(1)].sprite, 13104);
    assert_eq!(world.map.tile(10, 10).unwrap().foreground_sprite, 0);

    world.items.get_mut(&ItemId(1)).unwrap().driver_data[0] = 3; // rune 3 held
    world.update_bone_holder_sprite(ItemId(1));
    assert_eq!(world.items[&ItemId(1)].sprite, 13107);
    assert_eq!(world.map.tile(10, 10).unwrap().foreground_sprite, 13104);
}

#[test]
fn scan_and_clear_bone_holder_runes_builds_digits_in_stand_order_and_gates_by_owner() {
    let mut world = World::default();
    let mut activation = item(4, ItemFlags::USED);
    assert!(world.map.set_item_map(&mut activation, 13, 10));
    world.add_item(activation);

    let mut holder_a = item(1, ItemFlags::USED);
    assert!(world.map.set_item_map(&mut holder_a, 10, 10));
    holder_a.driver_data = vec![7, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0];
    world.add_item(holder_a);

    let mut holder_b = item(2, ItemFlags::USED);
    assert!(world.map.set_item_map(&mut holder_b, 11, 10));
    holder_b.driver_data = vec![5, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0];
    world.add_item(holder_b);

    // Owned by a different character - must not contribute a digit and
    // must be left untouched (C only clears stands whose owner matches).
    let mut holder_c = item(3, ItemFlags::USED);
    assert!(world.map.set_item_map(&mut holder_c, 12, 10));
    holder_c.driver_data = vec![9, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0];
    world.add_item(holder_c);

    let (nr, cleared) = world.scan_and_clear_bone_holder_runes(ItemId(4), CharacterId(1));

    assert_eq!(nr, 75);
    assert_eq!(cleared, [Some((ItemId(1), 7)), Some((ItemId(2), 5)), None]);
    assert_eq!(world.items[&ItemId(1)].driver_data[0], 0);
    assert_eq!(world.items[&ItemId(2)].driver_data[0], 0);
    assert_eq!(world.items[&ItemId(3)].driver_data[0], 9);
    assert_eq!(world.items[&ItemId(1)].sprite, 13103);
}

#[test]
fn scan_and_clear_bone_holder_runes_reports_zero_when_nothing_matches() {
    let mut world = World::default();
    let mut activation = item(4, ItemFlags::USED);
    assert!(world.map.set_item_map(&mut activation, 13, 10));
    world.add_item(activation);

    let (nr, cleared) = world.scan_and_clear_bone_holder_runes(ItemId(4), CharacterId(1));
    assert_eq!(nr, 0);
    assert_eq!(cleared, [None, None, None]);
}

fn empty_special_exec() -> [i32; crate::player::RUNE_SPECIAL_EXEC_COUNT] {
    [0; crate::player::RUNE_SPECIAL_EXEC_COUNT]
}

#[test]
fn exec_rune_single_digit_teleports_without_setting_the_flag() {
    let mut world = World::default();
    let mut player = character(1);
    player.x = 0;
    player.y = 0;
    world.add_character(player);

    let flag = world.exec_rune(CharacterId(1), 1, &empty_special_exec(), false, 18);

    assert!(!flag);
    assert_eq!(
        (
            world.characters[&CharacterId(1)].x,
            world.characters[&CharacterId(1)].y
        ),
        (90, 25)
    );
}

#[test]
fn exec_rune_double_digit_grants_bonus_exp_and_sets_the_flag() {
    let mut world = World::default();
    let player = character(1);
    world.add_character(player);

    let flag = world.exec_rune(CharacterId(1), 11, &empty_special_exec(), false, 18);

    assert!(flag);
    assert!(world.characters[&CharacterId(1)].exp > 0);
    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message == "You gained experience."));
}

#[test]
fn exec_rune_triple_digit_laughs_and_teleports_without_setting_the_flag() {
    let mut world = World::default();
    let player = character(1);
    world.add_character(player);

    let flag = world.exec_rune(CharacterId(1), 111, &empty_special_exec(), false, 18);

    assert!(!flag);
    assert_eq!(
        (
            world.characters[&CharacterId(1)].x,
            world.characters[&CharacterId(1)].y
        ),
        (72, 52)
    );
    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("sinister laugh")));
}

#[test]
fn exec_rune_literal_skill_combo_sets_flag_even_when_raise_fails() {
    let mut world = World::default();
    let mut player = character(1);
    // Endurance is already at its per-profile cap (0/0), so
    // `raise_value_exp` fails - C still sets `flag = 1` unconditionally.
    player.values = Character::empty_values();
    world.add_character(player);

    let flag = world.exec_rune(CharacterId(1), 212, &empty_special_exec(), false, 18);

    assert!(flag);
    let texts = world.drain_pending_system_texts();
    assert!(!texts
        .iter()
        .any(|text| text.message == "You gained endurance."));
}

#[test]
fn exec_rune_special_exec_table_raises_warrior_or_mage_skill() {
    let mut special_exec = empty_special_exec();
    special_exec[0] = 512; // arbitrary combination mapped to slot 0

    let mut world = World::default();
    let mut warrior = character(1);
    warrior.flags.insert(CharacterFlags::WARRIOR);
    warrior.values[1][CharacterValue::Attack as usize] = 5;
    warrior.values[0][CharacterValue::Attack as usize] = 50;
    world.add_character(warrior);

    let flag = world.exec_rune(CharacterId(1), 512, &special_exec, false, 18);

    assert!(flag);
    assert_eq!(
        world.characters[&CharacterId(1)].values[1][CharacterValue::Attack as usize],
        6
    );
    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message == "You gained Attack."));
}

#[test]
fn exec_rune_special_exec_bonus_area_teleport_needs_last_holder() {
    let mut special_exec = empty_special_exec();
    special_exec[20] = 512;

    let mut world = World::default();
    let player = character(1);
    world.add_character(player);

    let flag = world.exec_rune(CharacterId(1), 512, &special_exec, false, 18);
    assert!(!flag);
    assert_ne!(
        (
            world.characters[&CharacterId(1)].x,
            world.characters[&CharacterId(1)].y
        ),
        (14, 213)
    );
    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("does not work here")));

    let flag = world.exec_rune(CharacterId(1), 512, &special_exec, true, 18);
    assert!(!flag);
    assert_eq!(
        (
            world.characters[&CharacterId(1)].x,
            world.characters[&CharacterId(1)].y
        ),
        (14, 213)
    );
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| text.message == "Uh-oh"));
}

#[test]
fn exec_rune_unmatched_combination_reports_nothing_happened() {
    let mut world = World::default();
    let player = character(1);
    world.add_character(player);

    let flag = world.exec_rune(CharacterId(1), 42, &empty_special_exec(), false, 18);
    assert!(!flag);
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| text.message == "Nothing happened."));
}

#[test]
fn boneholder_activate_outcome_resolves_to_combination_and_cleared_stands() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    world.add_character(player);

    let mut activation = item(4, ItemFlags::USED | ItemFlags::USE);
    activation.driver = IDR_BONEHOLDER;
    activation.driver_data = vec![0, 2];
    assert!(world.map.set_item_map(&mut activation, 13, 10));
    world.add_item(activation);

    let mut holder = item(3, ItemFlags::USED);
    assert!(world.map.set_item_map(&mut holder, 12, 10));
    holder.driver_data = vec![4, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0];
    world.add_item(holder);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_BONEHOLDER,
            item_id: ItemId(4),
            character_id: CharacterId(1),
            spec: 0,
        },
        18,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::BoneHolderActivateResolved {
            nr: 4,
            last_holder: false,
            ..
        }
    ));
    assert_eq!(world.items[&ItemId(3)].driver_data[0], 0);
}
