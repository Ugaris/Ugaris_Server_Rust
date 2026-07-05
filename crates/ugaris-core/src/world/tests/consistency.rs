use super::*;

#[test]
fn all_zero_when_nothing_is_linked_wrong() {
    let mut world = World::default();
    let mut player = character(1);
    let mut carried = item(10, ItemFlags::USED);
    carried.carried_by = Some(CharacterId(1));
    player.inventory[30] = Some(ItemId(10));
    world.add_character(player);
    world.add_item(carried);

    let mut ground = item(11, ItemFlags::USED);
    assert!(world.map.set_item_map(&mut ground, 5, 5));
    world.add_item(ground);

    let mut container = item(12, ItemFlags::USED);
    container.content_id = 1;
    assert!(world.map.set_item_map(&mut container, 7, 7));
    world.add_item(container);
    let mut contained = item(13, ItemFlags::USED);
    contained.contained_in = Some(ItemId(12));
    world.add_item(contained);

    let report = world.consistency_check();
    assert_eq!(report, ConsistencyReport::default());

    // Nothing was mutated by a clean sweep.
    assert_eq!(
        world.items.get(&ItemId(10)).unwrap().carried_by,
        Some(CharacterId(1))
    );
    assert_eq!(world.map.tile(5, 5).unwrap().item, 11);
    assert_eq!(
        world.items.get(&ItemId(13)).unwrap().contained_in,
        Some(ItemId(12))
    );
}

#[test]
fn item_with_no_link_at_all_is_removed() {
    let mut world = World::default();
    world.add_item(item(20, ItemFlags::USED));

    let report = world.consistency_check();
    assert_eq!(report.item_errors, 1);
    assert!(world.items.get(&ItemId(20)).is_none());
}

#[test]
fn void_items_are_ignored_by_the_item_scan() {
    let mut world = World::default();
    world.add_item(item(21, ItemFlags::USED | ItemFlags::VOID));

    let report = world.consistency_check();
    assert_eq!(report.item_errors, 0);
    assert!(world.items.contains_key(&ItemId(21)));
}

#[test]
fn item_carried_by_missing_character_is_fixed() {
    let mut world = World::default();
    let mut dangling = item(30, ItemFlags::USED);
    dangling.carried_by = Some(CharacterId(99));
    world.add_item(dangling);

    let report = world.consistency_check();
    assert_eq!(report.item_errors, 1);
    let fixed = world.items.get(&ItemId(30)).unwrap();
    assert_eq!(fixed.carried_by, None);
}

#[test]
fn item_carried_by_character_with_no_back_link_is_fixed() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut dangling = item(31, ItemFlags::USED);
    dangling.carried_by = Some(CharacterId(1));
    world.add_item(dangling);

    let report = world.consistency_check();
    assert_eq!(report.item_errors, 1);
    assert_eq!(world.items.get(&ItemId(31)).unwrap().carried_by, None);
}

#[test]
fn item_carried_via_cursor_slot_is_recognized_as_linked() {
    let mut world = World::default();
    let mut player = character(1);
    player.cursor_item = Some(ItemId(32));
    world.add_character(player);
    let mut carried = item(32, ItemFlags::USED);
    carried.carried_by = Some(CharacterId(1));
    world.add_item(carried);

    let report = world.consistency_check();
    assert_eq!(report, ConsistencyReport::default());
    assert_eq!(
        world.items.get(&ItemId(32)).unwrap().carried_by,
        Some(CharacterId(1))
    );
}

#[test]
fn item_on_ground_with_no_tile_back_link_is_fixed() {
    let mut world = World::default();
    let mut ghost = item(40, ItemFlags::USED);
    ghost.x = 5;
    ghost.y = 5;
    world.add_item(ghost);

    let report = world.consistency_check();
    assert_eq!(report.item_errors, 1);
    let fixed = world.items.get(&ItemId(40)).unwrap();
    assert_eq!(fixed.x, 0);
    assert_eq!(fixed.y, 0);
}

#[test]
fn item_contained_in_missing_container_is_fixed() {
    let mut world = World::default();
    let mut orphan = item(50, ItemFlags::USED);
    orphan.contained_in = Some(ItemId(999));
    world.add_item(orphan);

    let report = world.consistency_check();
    assert_eq!(report.item_errors, 1);
    assert_eq!(world.items.get(&ItemId(50)).unwrap().contained_in, None);
}

#[test]
fn item_contained_in_non_container_item_is_fixed() {
    let mut world = World::default();
    let mut not_a_container = item(51, ItemFlags::USED); // content_id == 0: not a container
    assert!(world.map.set_item_map(&mut not_a_container, 7, 7));
    world.add_item(not_a_container);
    let mut contained = item(52, ItemFlags::USED);
    contained.contained_in = Some(ItemId(51));
    world.add_item(contained);

    let report = world.consistency_check();
    assert_eq!(report.item_errors, 1);
    assert_eq!(world.items.get(&ItemId(52)).unwrap().contained_in, None);
}

#[test]
fn map_tile_referencing_missing_item_is_cleared() {
    let mut world = World::default();
    world.map.tile_mut(5, 5).unwrap().item = 60;

    let report = world.consistency_check();
    assert_eq!(report.map_errors, 1);
    assert_eq!(world.map.tile(5, 5).unwrap().item, 0);
}

#[test]
fn map_tile_referencing_carried_item_is_cleared() {
    let mut world = World::default();
    world.add_character(character(1));
    let mut carried = item(61, ItemFlags::USED);
    carried.carried_by = Some(CharacterId(1));
    world.add_item(carried);
    world.map.tile_mut(5, 5).unwrap().item = 61;

    let report = world.consistency_check();
    assert_eq!(report.map_errors, 1);
    assert_eq!(world.map.tile(5, 5).unwrap().item, 0);
}

#[test]
fn map_tile_with_mismatched_item_coordinates_is_cleared() {
    let mut world = World::default();
    let mut wrong_spot = item(62, ItemFlags::USED);
    wrong_spot.x = 6;
    wrong_spot.y = 6;
    world.add_item(wrong_spot);
    // Tile at (5,5) claims item 62, but the item itself thinks it's at (6,6).
    world.map.tile_mut(5, 5).unwrap().item = 62;

    let report = world.consistency_check();
    assert_eq!(report.map_errors, 1);
    assert_eq!(world.map.tile(5, 5).unwrap().item, 0);
}

#[test]
fn item_referenced_from_two_tiles_is_deduplicated() {
    let mut world = World::default();
    let mut ground = item(63, ItemFlags::USED);
    ground.x = 5;
    ground.y = 5;
    world.add_item(ground);
    world.map.tile_mut(5, 5).unwrap().item = 63;
    // A second, unrelated tile also (wrongly) claims the same item id.
    world.map.tile_mut(6, 6).unwrap().item = 63;

    let report = world.consistency_check();
    // The legitimate tile (5,5) survives; only the duplicate at (6,6) is
    // counted as an error and cleared.
    assert_eq!(report.map_errors, 1);
    assert_eq!(world.map.tile(5, 5).unwrap().item, 63);
    assert_eq!(world.map.tile(6, 6).unwrap().item, 0);
}

#[test]
fn character_inventory_slot_referencing_missing_item_is_cleared() {
    let mut world = World::default();
    let mut player = character(1);
    player.inventory[30] = Some(ItemId(70));
    world.add_character(player);

    let report = world.consistency_check();
    assert_eq!(report.char_errors, 1);
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().inventory[30],
        None
    );
}

#[test]
fn character_inventory_slot_with_item_claiming_a_different_carrier_is_cleared() {
    let mut world = World::default();
    let mut player = character(1);
    player.inventory[30] = Some(ItemId(71));
    world.add_character(player);
    let mut item71 = item(71, ItemFlags::USED);
    item71.carried_by = Some(CharacterId(2)); // claims a different owner
    world.add_item(item71);

    let report = world.consistency_check();
    assert_eq!(report.char_errors, 1);
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().inventory[30],
        None
    );
    // The item's own dangling `carried_by` is a separate, still-standing
    // bug that only the items pass (run earlier) can catch; character 2
    // does not exist, so that pass already cleared it.
    assert_eq!(world.items.get(&ItemId(71)).unwrap().carried_by, None);
}

#[test]
fn character_inventory_item_with_stray_position_is_fixed_in_place() {
    let mut world = World::default();
    let mut player = character(1);
    player.inventory[30] = Some(ItemId(72));
    world.add_character(player);
    let mut item72 = item(72, ItemFlags::USED);
    item72.carried_by = Some(CharacterId(1));
    item72.x = 9;
    item72.y = 9;
    world.add_item(item72);

    let report = world.consistency_check();
    assert_eq!(report.char_errors, 1);
    // The slot itself is left alone; only the item's stray position is
    // cleared, matching C's per-check `continue` semantics.
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().inventory[30],
        Some(ItemId(72))
    );
    let fixed = world.items.get(&ItemId(72)).unwrap();
    assert_eq!(fixed.x, 0);
    assert_eq!(fixed.y, 0);
}

#[test]
fn character_cursor_item_is_checked_like_an_inventory_slot() {
    let mut world = World::default();
    let mut player = character(1);
    player.cursor_item = Some(ItemId(73));
    world.add_character(player);

    let report = world.consistency_check();
    assert_eq!(report.char_errors, 1);
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().cursor_item,
        None
    );
}

#[test]
fn item_carried_by_two_characters_is_deduplicated() {
    let mut world = World::default();
    let mut alice = character(1);
    alice.inventory[30] = Some(ItemId(74));
    world.add_character(alice);
    let mut bob = character(2);
    bob.inventory[30] = Some(ItemId(74));
    world.add_character(bob);
    let mut item74 = item(74, ItemFlags::USED);
    item74.carried_by = Some(CharacterId(1));
    world.add_item(item74);

    let report = world.consistency_check();
    // Character 1 legitimately links back; character 2's stray copy is
    // the one counted and cleared.
    assert_eq!(report.char_errors, 1);
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().inventory[30],
        Some(ItemId(74))
    );
    assert_eq!(
        world.characters.get(&CharacterId(2)).unwrap().inventory[30],
        None
    );
}

#[test]
fn contained_item_with_stray_position_is_fixed() {
    // Item 81 legitimately sits on the map at (3,3) *and* still carries a
    // stale `contained_in` link left over from before it was picked out
    // of container 80 - the items pass alone can't see this (its x/y
    // branch validates cleanly against the map tile, matching C's
    // `else if (it[in].x)` priority which never inspects `contained` at
    // all once `x` wins), but the map pass *does* check `it[in].contained`
    // (`consistency.c:216-222`) and clears the tile's back-link, and the
    // containers pass separately notices the item's own stray `x` and
    // clears that - two independent fixes triggered by the one stale
    // field, exactly matching C's incremental (not fully atomic)
    // self-healing across the four checks.
    let mut world = World::default();
    let mut container = item(80, ItemFlags::USED);
    container.content_id = 1;
    assert!(world.map.set_item_map(&mut container, 7, 7));
    world.add_item(container);

    let mut contained = item(81, ItemFlags::USED);
    contained.contained_in = Some(ItemId(80));
    assert!(world.map.set_item_map(&mut contained, 3, 3));
    // `set_item_map` clears `contained_in`; restore it to simulate the
    // stale leftover this test targets.
    contained.contained_in = Some(ItemId(80));
    world.add_item(contained);

    let report = world.consistency_check();
    assert_eq!(report.map_errors, 1);
    assert_eq!(report.container_errors, 1);
    assert_eq!(world.map.tile(3, 3).unwrap().item, 0);
    let fixed = world.items.get(&ItemId(81)).unwrap();
    assert_eq!(fixed.x, 0);
    assert_eq!(fixed.y, 0);
}

#[test]
fn contained_item_that_is_also_carried_is_fixed() {
    // Item 83 is legitimately carried by character 1 (a valid inventory
    // back-link, so the items pass sees a clean `carried_by` link and
    // raises no error) but still carries a stale `contained_in` link from
    // container 82. The characters pass checks its carried items in
    // order - carried-by, then position, *then* `contained_in`
    // (`consistency.c:344-352`) - and clears the stale container link
    // before the containers pass ever gets a chance to see it (an item
    // legitimately carried is never a candidate the containers pass
    // iterates in the first place, since by then `contained_in` is
    // already `None`).
    let mut world = World::default();
    let mut container = item(82, ItemFlags::USED);
    container.content_id = 1;
    assert!(world.map.set_item_map(&mut container, 7, 7));
    world.add_item(container);

    let mut player = character(1);
    player.inventory[30] = Some(ItemId(83));
    world.add_character(player);

    let mut contained = item(83, ItemFlags::USED);
    contained.contained_in = Some(ItemId(82));
    contained.carried_by = Some(CharacterId(1));
    world.add_item(contained);

    let report = world.consistency_check();
    assert_eq!(report.item_errors, 0);
    assert_eq!(report.char_errors, 1);
    assert_eq!(report.container_errors, 0);
    let fixed = world.items.get(&ItemId(83)).unwrap();
    assert_eq!(fixed.carried_by, Some(CharacterId(1)));
    assert_eq!(fixed.contained_in, None);
}
