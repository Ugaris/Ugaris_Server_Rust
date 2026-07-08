use super::*;

#[test]
fn initialization_scans_all_pentagram_items_and_sets_level_bounds() {
    let mut world = World::default();
    let mut p1 = item(1, ItemFlags::USED);
    p1.driver = IDR_PENT;
    p1.driver_data = vec![3, 0, 0, 0, 0]; // level 3, color unset
    world.add_item(p1);
    let mut p2 = item(2, ItemFlags::USED);
    p2.driver = IDR_PENT;
    p2.driver_data = vec![3, 0, 2, 0, 0]; // level 3, color already 2
    world.add_item(p2);
    let mut p3 = item(3, ItemFlags::USED);
    p3.driver = IDR_PENT;
    p3.driver_data = vec![5, 0, 1, 0, 0]; // level 5
    world.add_item(p3);

    world.ensure_pentagram_system_initialized();

    assert_eq!(world.pentagram_quest.total_pentagrams, 3);
    // C `min_level--; max_level--;` after the 1-indexed scan loop - ported
    // verbatim, so the observable bounds are one less than the raw levels.
    assert_eq!(world.pentagram_quest.min_level, 2);
    assert_eq!(world.pentagram_quest.max_level, 4);
    assert_eq!(world.pentagram_quest.area_pentagram_counts[3], 2);
    assert_eq!(world.pentagram_quest.area_pentagram_counts[5], 1);
    assert!(world.pentagram_quest.initialized);

    let p1_after = world.items.get(&ItemId(1)).unwrap();
    assert!((1..=3).contains(&(p1_after.driver_data[2] as i32)));
    let p2_after = world.items.get(&ItemId(2)).unwrap();
    assert_eq!(p2_after.driver_data[2], 2);

    // Lazily re-running init is a no-op (matches C's `if (!init_done)` guard).
    let snapshot = world.pentagram_quest.total_pentagrams;
    world.ensure_pentagram_system_initialized();
    assert_eq!(world.pentagram_quest.total_pentagrams, snapshot);
}

#[test]
fn activation_marks_area_complete_when_last_pentagram_at_level_activates() {
    let mut world = World::default();
    world.pentagram_quest.initialized = true;
    // High enough that this single activation doesn't also cross the
    // *solve* threshold (covered separately below) - this test is only
    // about the per-level *area-completion* serial bump.
    world.pentagram_quest.required_activations = 100;
    world.pentagram_quest.area_pentagram_counts[3] = 1;

    let mut pent = item(7, ItemFlags::USED | ItemFlags::USE);
    pent.driver = IDR_PENT;
    pent.driver_data = vec![3, 0, 4, 0, 0];
    world.add_item(pent);

    world.apply_pentagram_activate(ItemId(7), CharacterId(1));

    assert_eq!(world.pentagram_quest.area_activated_counts[3], 1);
    // Only one pentagram at level 3 => the area is complete on this single
    // activation => the area serial wraps from 255 to 1.
    assert_eq!(world.pentagram_quest.area_serials[3], 1);
    let item_after = world.items.get(&ItemId(7)).unwrap();
    // C stores the *pre-bump* area serial into the item.
    assert_eq!(item_after.driver_data[4], 255);
    assert_eq!(world.pentagram_quest.active_pentagrams, 1);

    let events = world.drain_pending_pentagram_activations();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].level, 3);
    assert!(!events[0].is_quest_solved);
}

#[test]
fn reaching_required_activations_resets_and_queues_solved_event() {
    let mut world = World::default();
    world.pentagram_quest.initialized = true;
    world.pentagram_quest.total_pentagrams = 5;
    world.pentagram_quest.active_pentagrams = 0;
    world.pentagram_quest.required_activations = 1;
    // A large per-level pentagram count avoids the area-completion
    // wraparound interfering with this test's focus (the *solve*
    // threshold, not the area-completion one covered above).
    world.pentagram_quest.area_pentagram_counts[3] = 5;
    world.pentagram_quest.area_serials[3] = 200;

    let mut pent = item(7, ItemFlags::USED | ItemFlags::USE);
    pent.driver = IDR_PENT;
    pent.driver_data = vec![3, 0, 4, 0, 0];
    world.add_item(pent);

    world.apply_pentagram_activate(ItemId(7), CharacterId(1));

    assert_eq!(world.pentagram_quest.active_pentagrams, 0);
    assert_eq!(world.pentagram_quest.solve_serial, 1);
    assert!(world.pentagram_quest.last_solve_tick.is_some());

    let events = world.drain_pending_pentagram_activations();
    assert_eq!(events.len(), 1);
    assert!(events[0].is_quest_solved);
    // The event snapshots active_pentagrams *before* the post-solve reset.
    assert_eq!(events[0].active_pentagrams, 1);
}

#[test]
fn timer_deactivates_pentagram_from_a_superseded_solve_round() {
    let mut world = World::default();
    world.pentagram_quest.initialized = true;
    world.pentagram_quest.solve_serial = 5;
    world.pentagram_quest.area_serials[3] = 9;
    world.pentagram_quest.area_activated_counts[3] = 1;

    let mut pent = item(7, ItemFlags::USED | ItemFlags::USE);
    pent.driver = IDR_PENT;
    pent.driver_data = vec![3, 2, 4, 0, 9]; // status=2, stale vs solve_serial=5
    pent.sprite = 1000;
    world.add_item(pent);

    world.apply_pentagram_timer(ItemId(7), 2, 9);

    let after = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(after.driver_data[1], 0);
    assert_eq!(after.driver_data[4], 0);
    assert_eq!(after.sprite, 996); // 1000 - color(4)
    assert_eq!(world.pentagram_quest.area_activated_counts[3], 0);
}

#[test]
fn timer_keeps_pentagram_active_when_still_current_round() {
    let mut world = World::default();
    world.pentagram_quest.initialized = true;
    world.pentagram_quest.solve_serial = 5;
    world.pentagram_quest.area_serials[3] = 9;

    let mut pent = item(7, ItemFlags::USED | ItemFlags::USE);
    pent.driver = IDR_PENT;
    pent.driver_data = vec![3, 5, 4, 0, 9]; // status/area_status both current
    pent.sprite = 1000;
    world.add_item(pent);

    world.apply_pentagram_timer(ItemId(7), 5, 9);

    let after = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(after.driver_data[1], 5);
    assert_eq!(after.sprite, 1000);
}

#[test]
fn timer_is_a_noop_for_an_inactive_pentagram() {
    let mut world = World::default();
    world.pentagram_quest.initialized = true;

    let mut pent = item(7, ItemFlags::USED | ItemFlags::USE);
    pent.driver = IDR_PENT;
    pent.driver_data = vec![3, 0, 4, 0, 0];
    pent.sprite = 1000;
    world.add_item(pent);

    world.apply_pentagram_timer(ItemId(7), 0, 0);

    let after = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(after.sprite, 1000);
}

#[test]
fn reset_pentagram_colors_reassigns_color_for_combo_items_only() {
    let mut world = World::default();
    let mut pent = item(7, ItemFlags::USED);
    pent.driver = IDR_PENT;
    pent.driver_data = vec![3, 1, 2, 0, 0]; // active (status != 0), color 2
    pent.sprite = 1000;
    world.add_item(pent);

    world.reset_pentagram_colors(&[7, 0, 0, 0, 0, 0], true);

    let after = world.items.get(&ItemId(7)).unwrap();
    let new_color = after.driver_data[2] as i32;
    assert!((1..=3).contains(&new_color));
    assert_eq!(after.sprite, 1000 - 2 + new_color);
}

#[test]
fn reset_pentagram_colors_noop_without_a_combo() {
    let mut world = World::default();
    let mut pent = item(7, ItemFlags::USED);
    pent.driver = IDR_PENT;
    pent.driver_data = vec![3, 0, 2, 0, 0];
    world.add_item(pent);

    world.reset_pentagram_colors(&[7, 0, 0, 0, 0, 0], false);

    let after = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(after.driver_data[2], 2);
}

#[test]
fn activation_always_queues_a_demon_spawn_request() {
    let mut world = World::default();
    world.pentagram_quest.initialized = true;
    world.pentagram_quest.required_activations = 100;
    world.pentagram_quest.area_pentagram_counts[3] = 5;

    let mut pent = item(7, ItemFlags::USED | ItemFlags::USE);
    pent.driver = IDR_PENT;
    pent.driver_data = vec![3, 0, 4, 0, 0];
    world.add_item(pent);

    world.apply_pentagram_activate(ItemId(7), CharacterId(1));

    let spawns = world.drain_pending_pentagram_demon_spawns();
    assert_eq!(spawns.len(), 1);
    assert_eq!(spawns[0].item_id, ItemId(7));
    assert_eq!(spawns[0].level, 3);
    // C `get_activation_spawn_count()` default is 3.
    assert_eq!(spawns[0].spawn_count, 3);
}

#[test]
fn timer_reset_queues_a_demon_spawn_request() {
    let mut world = World::default();
    world.pentagram_quest.initialized = true;
    world.pentagram_quest.solve_serial = 5;
    world.pentagram_quest.area_serials[3] = 9;
    world.pentagram_quest.area_activated_counts[3] = 1;

    let mut pent = item(7, ItemFlags::USED | ItemFlags::USE);
    pent.driver = IDR_PENT;
    pent.driver_data = vec![3, 2, 4, 0, 9];
    world.add_item(pent);

    world.apply_pentagram_timer(ItemId(7), 2, 9);

    let spawns = world.drain_pending_pentagram_demon_spawns();
    assert_eq!(spawns.len(), 1);
    assert_eq!(spawns[0].spawn_count, 3);
}

#[test]
fn timer_inactive_pentagram_never_queues_more_than_one_demon() {
    let mut world = World::default();
    world.pentagram_quest.initialized = true;

    let mut pent = item(7, ItemFlags::USED | ItemFlags::USE);
    pent.driver = IDR_PENT;
    pent.driver_data = vec![3, 0, 4, 0, 0];
    world.add_item(pent);

    // Run the timer many times; whenever it queues a spawn it must be
    // exactly 1 demon (C's occasional single-demon roll), never the
    // activation-count's 3.
    for _ in 0..200 {
        world.apply_pentagram_timer(ItemId(7), 0, 0);
    }
    let spawns = world.drain_pending_pentagram_demon_spawns();
    assert!(spawns.iter().all(|request| request.spawn_count == 1));
}

#[test]
fn pentagram_max_spawns_uses_level_threshold() {
    let world = World::default();
    // C default `spawn_count_level_threshold = 16`, `max_spawn_low_level =
    // 3`, `max_spawn_high_level = 2`.
    assert_eq!(world.pentagram_max_spawns(5), 3);
    assert_eq!(world.pentagram_max_spawns(16), 2);
    assert_eq!(world.pentagram_max_spawns(30), 2);
}

#[test]
fn pentagram_spawn_slot_is_stale_for_empty_dead_and_mismatched_serial() {
    let mut world = World::default();
    let mut pent = item(7, ItemFlags::USED);
    pent.driver = IDR_PENT;
    pent.driver_data = vec![0; 10];
    world.add_item(pent);

    // Empty slot (character_id == 0).
    assert!(world.pentagram_spawn_slot_is_stale(ItemId(7), 0));

    // Slot references a character that no longer exists.
    if let Some(item) = world.items.get_mut(&ItemId(7)) {
        item.driver_data[6..8].copy_from_slice(&99u16.to_le_bytes());
        item.driver_data[8..10].copy_from_slice(&42u16.to_le_bytes());
    }
    assert!(world.pentagram_spawn_slot_is_stale(ItemId(7), 0));

    // Slot references a live character with a matching serial - not stale.
    let mut demon = character(99);
    demon.serial = 42;
    world.add_character(demon);
    assert!(!world.pentagram_spawn_slot_is_stale(ItemId(7), 0));

    // Serial mismatch (old demon died and the slot number got reused) -
    // stale again.
    if let Some(item) = world.items.get_mut(&ItemId(7)) {
        item.driver_data[8..10].copy_from_slice(&7u16.to_le_bytes());
    }
    assert!(world.pentagram_spawn_slot_is_stale(ItemId(7), 0));
}

#[test]
fn apply_pentagram_spawn_result_records_character_and_serial() {
    let mut world = World::default();
    let mut pent = item(7, ItemFlags::USED);
    pent.driver = IDR_PENT;
    world.add_item(pent);
    let mut demon = character(55);
    demon.serial = 4321;
    world.add_character(demon);

    assert!(world.apply_pentagram_spawn_result(ItemId(7), 1, CharacterId(55), 4321));

    let after = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(&after.driver_data[10..12], &55u16.to_le_bytes());
    assert_eq!(&after.driver_data[12..14], &4321u16.to_le_bytes());
    assert!(!world.pentagram_spawn_slot_is_stale(ItemId(7), 1));
}

#[test]
fn enhance_elite_demon_boosts_stats_renames_and_tints() {
    let mut world = World::default();
    let mut demon = character(1);
    // >= `skill_count` (3) values in the "default" candidate pool so the
    // `enhance_demon_character` tail can't fall back to scanning every
    // nonzero skill (which would non-deterministically also touch Hp,
    // since it's not itself in the default pool).
    demon.values[1][CharacterValue::Hp as usize] = 100;
    demon.values[1][CharacterValue::Attack as usize] = 20;
    demon.values[1][CharacterValue::Parry as usize] = 15;
    demon.values[1][CharacterValue::Sword as usize] = 10;
    demon.name = "Demon".to_string();
    world.add_character(demon);

    world.enhance_elite_demon(CharacterId(1));

    let after = world.characters.get(&CharacterId(1)).unwrap();
    // 100 * 1.2 = 120 - Hp isn't in the default enhancement pool, so it's
    // untouched by the `enhance_demon_character` tail.
    assert_eq!(after.values[1][CharacterValue::Hp as usize], 120);
    assert!(after.name.starts_with("Elite Demon \""));
    assert_eq!(after.c1, 153);
    assert!(after.description.contains("elite demon"));
    // `enhance_demon_character(character_id, 3, 3, 8, 0, true)` boosted 3
    // of {Attack, Parry, Sword} further past their own 1.2x multiplier.
    assert!(after.description.contains("Enhanced:"));
}

#[test]
fn adjust_lesser_demon_reduces_stats_and_renames() {
    let mut world = World::default();
    let mut demon = character(1);
    demon.values[1][CharacterValue::Hp as usize] = 100;
    demon.name = "Demon".to_string();
    world.add_character(demon);

    world.adjust_lesser_demon(CharacterId(1));

    let after = world.characters.get(&CharacterId(1)).unwrap();
    // 100 * 0.8 = 80.
    assert_eq!(after.values[1][CharacterValue::Hp as usize], 80);
    assert_eq!(after.name, "Lesser Demon");
    assert_eq!(after.c1, 187);
}

#[test]
fn finish_pentagram_demon_spawn_sets_full_power_and_nonotify() {
    let mut world = World::default();
    let mut demon = character(1);
    demon.values[0][CharacterValue::Hp as usize] = 10;
    demon.values[0][CharacterValue::Endurance as usize] = 5;
    demon.values[0][CharacterValue::Mana as usize] = 3;
    demon.values[1] = demon.values[0].clone();
    demon.class = 53;
    world.add_character(demon);

    world.finish_pentagram_demon_spawn(CharacterId(1), DemonType::Normal, 53);

    let after = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(after.hp, 10 * POWERSCALE);
    assert_eq!(after.endurance, 5 * POWERSCALE);
    assert_eq!(after.mana, 3 * POWERSCALE);
    assert_eq!(after.dir, Direction::RightDown as u8);
    assert!(after.flags.contains(CharacterFlags::NONOTIFY));
    // Normal demons keep their template class untouched.
    assert_eq!(after.class, 53);
}

#[test]
fn finish_pentagram_demon_spawn_elite_reassigns_class_base() {
    let mut world = World::default();
    let mut demon = character(1);
    demon.class = 53;
    world.add_character(demon);

    world.finish_pentagram_demon_spawn(CharacterId(1), DemonType::Elite, 53);

    let after = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(after.class, ELITE_DEMON_CLASS_BASE + 53 % 48);
}

#[test]
fn finish_pentagram_demon_spawn_lesser_reassigns_class_base() {
    let mut world = World::default();
    let mut demon = character(1);
    demon.class = 53;
    world.add_character(demon);

    world.finish_pentagram_demon_spawn(CharacterId(1), DemonType::Lesser, 53);

    let after = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(after.class, LESSER_DEMON_CLASS_BASE + 53 % 48);
}

#[test]
fn update_demon_profession_seeds_from_existing_and_scales_with_training_power() {
    let mut world = World::default();
    world.pentagram_quest.training_power = 12_000;
    let mut demon = character(1);
    demon.deaths = 0;
    demon.professions[P_DEMON] = 640; // seeds `deaths` on the first call.
    demon.class = 53; // outside the 258..=305 demon-lord range.
    world.add_character(demon);

    world.update_demon_profession(CharacterId(1));

    let after = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(after.deaths, 640);
    // 640 * (12000 + 20000) / 64000 = 320.
    assert_eq!(after.professions[P_DEMON], 320);
}

#[test]
fn update_demon_profession_uses_higher_offset_for_demon_lord_classes() {
    let mut world = World::default();
    world.pentagram_quest.training_power = 12_000;
    let mut demon = character(1);
    demon.deaths = 640;
    demon.class = 260; // inside 258..=305.
    world.add_character(demon);

    world.update_demon_profession(CharacterId(1));

    let after = world.characters.get(&CharacterId(1)).unwrap();
    // 640 * (12000 + 52000) / 64000 = 640.
    assert_eq!(after.professions[P_DEMON], 640);
}

#[test]
fn penter_demon_death_reduces_power_only_for_demon_lord_classes() {
    let mut world = World::default();
    world.pentagram_quest.power_levels[2] = 100; // class_index for 260.
    let mut demon = character(1);
    demon.class = 260; // 260 - 258 = index 2.
    demon.driver = crate::character_driver::CDR_PENTER;
    world.add_character(demon);
    let mut killer = character(2);
    killer.flags = CharacterFlags::USED | CharacterFlags::PLAYER;
    world.add_character(killer);

    world.apply_penter_demon_death(CharacterId(1), Some(CharacterId(2)));

    // power_loss = 100 + 750 (default demon_power_deduction) = 850.
    assert_eq!(world.pentagram_quest.power_levels[2], -750);
    assert_eq!(world.pentagram_quest.training_power, -850);
    let awards = world.drain_pending_penter_demon_lords_demise_awards();
    assert_eq!(awards, vec![CharacterId(2)]);
}

#[test]
fn penter_demon_death_is_a_noop_for_elite_and_lesser_and_ordinary_classes() {
    let mut world = World::default();
    world.pentagram_quest.training_power = 500;

    for class in [
        ELITE_DEMON_CLASS_BASE + 5,
        LESSER_DEMON_CLASS_BASE + 5,
        53, // ordinary area-4 penterN class, outside both ranges.
    ] {
        let mut demon = character(1);
        demon.class = class;
        world.add_character(demon);

        world.apply_penter_demon_death(CharacterId(1), Some(CharacterId(2)));

        assert_eq!(world.pentagram_quest.training_power, 500);
        assert!(world
            .drain_pending_penter_demon_lords_demise_awards()
            .is_empty());
    }
}

#[test]
fn penter_demon_death_without_a_killer_still_reduces_power() {
    let mut world = World::default();
    world.pentagram_quest.power_levels[0] = 0;
    let mut demon = character(1);
    demon.class = 258; // class_index 0.
    world.add_character(demon);

    world.apply_penter_demon_death(CharacterId(1), None);

    assert_eq!(world.pentagram_quest.training_power, -750);
    assert!(world
        .drain_pending_penter_demon_lords_demise_awards()
        .is_empty());
}
