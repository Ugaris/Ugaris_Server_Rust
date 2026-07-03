use super::*;

#[test]
fn dungeon_teleport_decodes_legacy_target_and_requires_player() {
    let mut actor = character(1);
    actor.flags |= CharacterFlags::PLAYER;
    let mut tele = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DUNGEONTELE);
    tele.driver_data = vec![44, 1, 55, 1, 13, 0];

    let outcome = execute_item_driver(
        &mut actor,
        &mut tele,
        ItemDriverRequest::Driver {
            driver: IDR_DUNGEONTELE,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        13,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::DungeonTeleport {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            x: 300,
            y: 311,
            clan_number: 13,
        }
    );

    actor.flags.remove(CharacterFlags::PLAYER);
    let outcome = execute_item_driver(
        &mut actor,
        &mut tele,
        ItemDriverRequest::Driver {
            driver: IDR_DUNGEONTELE,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        13,
        false,
    );
    assert_eq!(outcome, ItemDriverOutcome::Noop);
}

#[test]
fn dungeon_fake_and_key_port_area13_dispatch_boundary() {
    let mut actor = character(1);
    let mut fake = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DUNGEONFAKE);
    fake.driver_data = vec![21, 0];

    let outcome = execute_item_driver(
        &mut actor,
        &mut fake,
        ItemDriverRequest::Driver {
            driver: IDR_DUNGEONFAKE,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        13,
        false,
    );
    assert_eq!(
        outcome,
        ItemDriverOutcome::DungeonFake {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            clan_number: 21,
        }
    );

    let mut key = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_DUNGEONKEY);
    key.driver_data = vec![1, 21, 0, 0, 0x44, 0x33, 0x22, 0x11];
    let outcome = execute_item_driver(
        &mut actor,
        &mut key,
        ItemDriverRequest::Driver {
            driver: IDR_DUNGEONKEY,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        13,
        false,
    );
    assert_eq!(
        outcome,
        ItemDriverOutcome::DungeonKey {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            template: "maze_key1",
            key_id: 0x1122_3344,
            clan_number: 21,
            first_taken: true,
        }
    );
    assert_eq!(key.driver_data[2], 1);

    actor.cursor_item = Some(ItemId(99));
    let blocked = execute_item_driver(
        &mut actor,
        &mut key,
        ItemDriverRequest::Driver {
            driver: IDR_DUNGEONKEY,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        13,
        false,
    );
    assert_eq!(
        blocked,
        ItemDriverOutcome::DungeonKeyCursorOccupied {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
}

#[test]
fn dungeon_door_ports_key_and_defender_gates() {
    let mut actor = character(1);
    let mut door = item(8, ItemFlags::USED | ItemFlags::USE, 0, IDR_DUNGEONDOOR);
    door.x = 164;
    door.y = 83;
    door.driver_data = vec![
        0x44, 0x33, 0x22, 0x11, 0x88, 0x77, 0x66, 0x55, 9, 0, 0, 0, 0,
    ];
    let request = ItemDriverRequest::Driver {
        driver: IDR_DUNGEONDOOR,
        item_id: ItemId(8),
        character_id: CharacterId(1),
        spec: 0,
    };

    let missing = execute_item_driver_with_context(
        &mut actor,
        &mut door,
        request,
        13,
        false,
        &ItemDriverContext::default(),
    );
    assert_eq!(
        missing,
        ItemDriverOutcome::DungeonDoorMissingKeys {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            missing: 2,
            both_required: true,
        }
    );

    let too_many = execute_item_driver_with_context(
        &mut actor,
        &mut door,
        request,
        13,
        false,
        &ItemDriverContext {
            has_dungeon_door_key1: true,
            has_dungeon_door_key2: true,
            dungeon_defender_count: Some(21),
            ..ItemDriverContext::default()
        },
    );
    assert_eq!(
        too_many,
        ItemDriverOutcome::DungeonDoorTooManyDefenders {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            alive: 21,
            max_allowed: 20,
        }
    );

    let solved = execute_item_driver_with_context(
        &mut actor,
        &mut door,
        request,
        13,
        false,
        &ItemDriverContext {
            has_dungeon_door_key1: true,
            has_dungeon_door_key2: true,
            dungeon_defender_count: Some(20),
            ..ItemDriverContext::default()
        },
    );
    assert_eq!(
        solved,
        ItemDriverOutcome::DungeonDoorSolved {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            clan_number: 9,
            catacomb: 5,
            first_solve: true,
        }
    );
    assert_eq!(drdata_u32(&door, 0), 0);
    assert_eq!(drdata_u32(&door, 4), 0);
    assert_eq!(door.driver_data[12], 1);
}

#[test]
fn dungeon_driver_ids_are_area13_guarded_like_legacy_libload() {
    let mut actor = character(1);
    actor.flags |= CharacterFlags::PLAYER;
    let mut tele = item(7, ItemFlags::USED | ItemFlags::USE, 0, IDR_DUNGEONTELE);

    let outcome = execute_item_driver(
        &mut actor,
        &mut tele,
        ItemDriverRequest::Driver {
            driver: IDR_DUNGEONTELE,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
        false,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::LibloadAreaBlocked {
            driver: IDR_DUNGEONTELE,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            required_area: 13,
        }
    );
}
