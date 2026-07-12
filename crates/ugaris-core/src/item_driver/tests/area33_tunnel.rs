use super::*;

fn tunnel_door(id: u32, door_type: u8) -> Item {
    let mut door = item(id, ItemFlags::USED | ItemFlags::USE, 0, IDR_TUNNELDOOR);
    door.driver_data = vec![door_type];
    door
}

fn request(item_id: u32, character_id: u32) -> ItemDriverRequest {
    ItemDriverRequest::Driver {
        driver: IDR_TUNNELDOOR,
        item_id: ItemId(item_id),
        character_id: CharacterId(character_id),
        spec: 0,
    }
}

// C `tunneldoor`'s `if (!cn) return;` automatic-call guard (`tunnel.c:
// 608-610`).
#[test]
fn tunneldoor_driver_is_a_noop_on_automatic_call() {
    let mut character = character(0);
    let mut door = tunnel_door(1, 2);
    assert_eq!(
        execute_item_driver(&mut character, &mut door, request(1, 0), 33, false),
        ItemDriverOutcome::Noop
    );
}

// C `tunneldoor`'s `DOOR_EXIT_EXP`/`DOOR_EXIT_MILITARY` branches
// (`tunnel.c:630-636`) produce the reward outcome.
#[test]
fn tunneldoor_driver_exit_pillars_produce_reward_outcome() {
    let mut character = character(1);
    for door_type in [2u8, 3u8] {
        let mut door = tunnel_door(1, door_type);
        assert_eq!(
            execute_item_driver(&mut character, &mut door, request(1, 1), 33, false),
            ItemDriverOutcome::TunnelDoorExitReward {
                item_id: ItemId(1),
                character_id: CharacterId(1),
                door_type,
            }
        );
    }
}

// `DOOR_ENTRY`/`DOOR_CONTINUE` (the not-yet-ported creeper-dungeon
// generator) fall through to `Unsupported` - a documented gap, not a
// regression (this driver was entirely undispatched before this port).
#[test]
fn tunneldoor_driver_entry_and_continue_are_unsupported() {
    let mut character = character(1);
    for door_type in [0u8, 1u8] {
        let mut door = tunnel_door(1, door_type);
        assert_eq!(
            execute_item_driver(&mut character, &mut door, request(1, 1), 33, false),
            ItemDriverOutcome::Unsupported {
                driver: IDR_TUNNELDOOR,
                item_id: ItemId(1),
                character_id: CharacterId(1),
            }
        );
    }
}

// `IDR_TUNNELDOOR`'s area-33 gate (`legacy_libload_required_area`).
#[test]
fn tunneldoor_driver_is_blocked_outside_area_33() {
    let mut character = character(1);
    let mut door = tunnel_door(1, 2);
    assert_eq!(
        execute_item_driver(&mut character, &mut door, request(1, 1), 1, false),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_TUNNELDOOR,
            item_id: ItemId(1),
            character_id: CharacterId(1),
            required_area: 33,
        }
    );
}
