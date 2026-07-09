//! `World`-level resolution of `IDR_LAB3_SPECIAL`'s teleport-door branch
//! (`lab3_special_driver`'s raw `Lab3TeleportDoor`/`Lab3TeleportDoorLocked`
//! outcomes are pure-driver-only tested in
//! `item_driver::tests::area22_lab`; this file exercises the full
//! `World::execute_item_driver_request` pipeline, i.e. `World::
//! apply_lab3_teleport_door`'s actual mutation).

use super::*;
use crate::item_driver::IDR_LAB3_SPECIAL;

const WN_LHAND: usize = 8;

fn lab3_door(id: u32, dx: i8, dy: i8, password_protected: bool) -> Item {
    let mut door = item(id, ItemFlags::USED | ItemFlags::USE);
    door.driver = IDR_LAB3_SPECIAL;
    door.driver_data = vec![1, dx as u8, dy as u8, u8::from(password_protected)];
    door
}

fn lab3_door_request(item_id: u32, character_id: u32) -> ItemDriverRequest {
    ItemDriverRequest::Driver {
        driver: IDR_LAB3_SPECIAL,
        item_id: ItemId(item_id),
        character_id: CharacterId(character_id),
        spec: 0,
    }
}

#[test]
fn teleport_door_moves_character_when_dry() {
    let mut world = World::default();
    let mut player = character(1);
    player.x = 10;
    player.y = 10;
    world.add_character(player);
    world.add_item(lab3_door(7, 3, -2, false));

    let outcome = world.execute_item_driver_request(lab3_door_request(7, 1), 22);

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab3TeleportDoor {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            dx: 3,
            dy: -2,
            password_protected: false,
            extinguished_count: 0,
        }
    );
    let moved = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((moved.x, moved.y), (13, 8));
}

#[test]
fn teleport_door_reports_busy_when_target_is_the_current_tile() {
    let mut world = World::default();
    let mut player = character(1);
    player.x = 10;
    player.y = 10;
    world.add_character(player);
    // dx=0, dy=0: `teleport_char_driver`'s own "already within Manhattan
    // distance 1" no-op path (`lab3.c:917-920`'s "crowd behind the door"
    // failure branch).
    world.add_item(lab3_door(7, 0, 0, false));

    let outcome = world.execute_item_driver_request(lab3_door_request(7, 1), 22);

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab3TeleportDoorBusy {
            character_id: CharacterId(1),
        }
    );
    let unmoved = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((unmoved.x, unmoved.y), (10, 10));
}

#[test]
fn teleport_door_locked_without_guard_talkstep_context() {
    let mut world = World::default();
    let mut player = character(1);
    player.x = 10;
    player.y = 10;
    world.add_character(player);
    world.add_item(lab3_door(7, 1, 0, true));

    // Plain `execute_item_driver_request` uses `ItemDriverContext::
    // default()`, i.e. `lab3_guard_talkstep: None` -> treated as `0`,
    // matching a freshly-allocated `struct lab_ppd`.
    let outcome = world.execute_item_driver_request(lab3_door_request(7, 1), 22);

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab3TeleportDoorLocked {
            character_id: CharacterId(1),
        }
    );
    let unmoved = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((unmoved.x, unmoved.y), (10, 10));
}

#[test]
fn teleport_door_queues_lab_exit_reward_when_password_protected_and_opened() {
    let mut world = World::default();
    let mut player = character(1);
    player.x = 10;
    player.y = 10;
    world.add_character(player);
    world.add_item(lab3_door(7, 2, 0, true));

    let outcome = world.execute_item_driver_request_with_context(
        lab3_door_request(7, 1),
        22,
        &ItemDriverContext {
            lab3_guard_talkstep: Some(20),
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab3TeleportDoor {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            dx: 2,
            dy: 0,
            password_protected: true,
            extinguished_count: 0,
        }
    );
    let requests = world.drain_pending_lab_exit_spawns();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].killer_id, CharacterId(1));
    assert_eq!(requests[0].level, 25);
}

#[test]
fn teleport_door_arriving_underwater_extinguishes_lit_torches_without_oxygen() {
    let mut world = World::default();
    let mut player = character(1);
    player.x = 10;
    player.y = 10;
    player.inventory[30] = Some(ItemId(20));
    player.inventory[WN_LHAND] = Some(ItemId(21));
    world.add_character(player);
    world.add_item(lab3_door(7, 0, 2, false));

    let mut inventory_torch = item(20, ItemFlags::USED | ItemFlags::USE);
    inventory_torch.driver = IDR_TORCH;
    inventory_torch.driver_data = vec![1, 55];
    inventory_torch.carried_by = Some(CharacterId(1));
    world.add_item(inventory_torch);

    let mut lhand_torch = item(21, ItemFlags::USED | ItemFlags::USE);
    lhand_torch.driver = IDR_TORCH;
    lhand_torch.driver_data = vec![1, 40];
    lhand_torch.carried_by = Some(CharacterId(1));
    world.add_item(lhand_torch);

    world
        .map
        .tile_mut(10, 12)
        .unwrap()
        .flags
        .insert(MapFlags::UNDERWATER);

    let outcome = world.execute_item_driver_request(lab3_door_request(7, 1), 22);

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab3TeleportDoor {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            dx: 0,
            dy: 2,
            password_protected: false,
            extinguished_count: 2,
        }
    );
    assert_eq!(world.items.get(&ItemId(20)).unwrap().driver_data[0], 0);
    assert_eq!(world.items.get(&ItemId(21)).unwrap().driver_data[0], 0);
    // No `CF_OXYGEN`: the bubble/talk flavor fires too.
    assert_eq!(world.pending_area_texts.len(), 1);
    assert!(world.pending_area_texts[0].message.contains("Hrgblub."));
}

#[test]
fn teleport_door_arriving_underwater_with_oxygen_skips_bubble_flavor_but_still_extinguishes() {
    let mut world = World::default();
    let mut player = character(1);
    player.x = 10;
    player.y = 10;
    player.flags.insert(CharacterFlags::OXYGEN);
    player.inventory[30] = Some(ItemId(20));
    world.add_character(player);
    world.add_item(lab3_door(7, 0, 2, false));

    let mut inventory_torch = item(20, ItemFlags::USED | ItemFlags::USE);
    inventory_torch.driver = IDR_TORCH;
    inventory_torch.driver_data = vec![1, 55];
    inventory_torch.carried_by = Some(CharacterId(1));
    world.add_item(inventory_torch);

    world
        .map
        .tile_mut(10, 12)
        .unwrap()
        .flags
        .insert(MapFlags::UNDERWATER);

    let outcome = world.execute_item_driver_request(lab3_door_request(7, 1), 22);

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab3TeleportDoor {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            dx: 0,
            dy: 2,
            password_protected: false,
            extinguished_count: 1,
        }
    );
    assert_eq!(world.items.get(&ItemId(20)).unwrap().driver_data[0], 0);
    assert!(world.pending_area_texts.is_empty());
}

#[test]
fn teleport_door_arriving_on_dry_land_leaves_torches_alone() {
    let mut world = World::default();
    let mut player = character(1);
    player.x = 10;
    player.y = 10;
    player.inventory[30] = Some(ItemId(20));
    world.add_character(player);
    world.add_item(lab3_door(7, 0, 2, false));

    let mut inventory_torch = item(20, ItemFlags::USED | ItemFlags::USE);
    inventory_torch.driver = IDR_TORCH;
    inventory_torch.driver_data = vec![1, 55];
    inventory_torch.carried_by = Some(CharacterId(1));
    world.add_item(inventory_torch);

    let outcome = world.execute_item_driver_request(lab3_door_request(7, 1), 22);

    assert_eq!(
        outcome,
        ItemDriverOutcome::Lab3TeleportDoor {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            dx: 0,
            dy: 2,
            password_protected: false,
            extinguished_count: 0,
        }
    );
    assert_eq!(world.items.get(&ItemId(20)).unwrap().driver_data[0], 1);
}
