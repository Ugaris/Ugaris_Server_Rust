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

fn mean_door(id: u32, x: u16, y: u16) -> Item {
    let mut door = item(id, ItemFlags::USED | ItemFlags::USE, 0, IDR_TUNNELDOOR2);
    door.x = x;
    door.y = y;
    door
}

fn mean_door_request(item_id: u32, character_id: u32) -> ItemDriverRequest {
    ItemDriverRequest::Driver {
        driver: IDR_TUNNELDOOR2,
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

// C `mean_door`'s automatic-call branch (`tunnel.c:737-742`): always
// reschedules itself and opens when
// `ItemDriverContext::tunnel_door_area_clear` says the room is clear.
#[test]
fn mean_door_driver_automatic_call_opens_when_area_clear() {
    let mut timer = character(0);
    let mut door = mean_door(1, 40, 60);
    let context = ItemDriverContext {
        timer_call: true,
        tunnel_door_area_clear: Some(true),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer,
            &mut door,
            mean_door_request(1, 0),
            33,
            false,
            &context,
        ),
        ItemDriverOutcome::TunnelDoorAreaCheck {
            item_id: ItemId(1),
            x: 40,
            y: 60,
            opened: true,
            schedule_after_ticks: TICKS_PER_SECOND,
        }
    );
}

// Same automatic-call branch, area not clear - still reschedules, but
// doesn't open.
#[test]
fn mean_door_driver_automatic_call_stays_closed_when_area_not_clear() {
    let mut timer = character(0);
    let mut door = mean_door(1, 40, 60);
    let context = ItemDriverContext {
        timer_call: true,
        tunnel_door_area_clear: Some(false),
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer,
            &mut door,
            mean_door_request(1, 0),
            33,
            false,
            &context,
        ),
        ItemDriverOutcome::TunnelDoorAreaCheck {
            item_id: ItemId(1),
            x: 40,
            y: 60,
            opened: false,
            schedule_after_ticks: TICKS_PER_SECOND,
        }
    );
}

// A missing `tunnel_door_area_clear` (not computed by the caller) is
// treated as "not clear", never opening the door by mistake.
#[test]
fn mean_door_driver_automatic_call_treats_missing_area_clear_as_not_clear() {
    let mut timer = character(0);
    let mut door = mean_door(1, 40, 60);
    let context = ItemDriverContext {
        timer_call: true,
        tunnel_door_area_clear: None,
        ..ItemDriverContext::default()
    };
    assert_eq!(
        execute_item_driver_with_context(
            &mut timer,
            &mut door,
            mean_door_request(1, 0),
            33,
            false,
            &context,
        ),
        ItemDriverOutcome::TunnelDoorAreaCheck {
            item_id: ItemId(1),
            x: 40,
            y: 60,
            opened: false,
            schedule_after_ticks: TICKS_PER_SECOND,
        }
    );
}

// C `mean_door`'s player-interaction branch (`tunnel.c:744-747`,
// literally commented "not implemented in the original code") - the
// door only reacts to its own periodic timer, never to a player.
#[test]
fn mean_door_driver_player_interaction_is_a_flavor_line_only() {
    let mut player = character(1);
    let mut door = mean_door(1, 40, 60);
    assert_eq!(
        execute_item_driver(&mut player, &mut door, mean_door_request(1, 1), 33, false),
        ItemDriverOutcome::TunnelDoorFlavor {
            item_id: ItemId(1),
            character_id: CharacterId(1),
        }
    );
}

// `IDR_TUNNELDOOR2`'s area-33 gate (`legacy_libload_required_area`).
#[test]
fn mean_door_driver_is_blocked_outside_area_33() {
    let mut player = character(1);
    let mut door = mean_door(1, 40, 60);
    assert_eq!(
        execute_item_driver(&mut player, &mut door, mean_door_request(1, 1), 1, false),
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_TUNNELDOOR2,
            item_id: ItemId(1),
            character_id: CharacterId(1),
            required_area: 33,
        }
    );
}
