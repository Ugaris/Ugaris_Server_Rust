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
