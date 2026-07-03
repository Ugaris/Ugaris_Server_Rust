use super::*;

#[test]
fn world_edemon_tube_overload_teleports_visible_nearby_players() {
    let mut world = World::default();
    world.add_character(character(0));
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    assert!(world.spawn_character(player, 12, 10));

    let mut tube = item(7, ItemFlags::USED | ItemFlags::USE);
    tube.driver = IDR_EDEMONTUBE;
    tube.driver_data = vec![4, 0, 0, 0, 0, 0];
    assert!(world.map.set_item_map(&mut tube, 10, 10));
    world.add_item(tube);

    let mut loader = item(8, ItemFlags::USED | ItemFlags::USE);
    loader.driver = IDR_EDEMONLOADER;
    loader.driver_data = vec![4, 251, 0];
    loader.x = 20;
    loader.y = 20;
    world.add_item(loader);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_EDEMONTUBE,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        6,
    );

    assert!(matches!(outcome, ItemDriverOutcome::EdemonTubePulse { .. }));
    let player = &world.characters[&CharacterId(1)];
    assert_eq!((player.x, player.y), (20, 21));
    assert_eq!(
        world.drain_pending_system_texts(),
        vec![WorldSystemText {
            character_id: CharacterId(1),
            message: "The strange tube teleported you.".to_string(),
        }]
    );
    assert_eq!(world.timers.used_timers(), 1);
}

#[test]
fn world_executes_same_area_teleport_driver_outcome() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    character.action = action::USE;
    character.duration = 3;
    world.map.tile_mut(10, 10).unwrap().character = 1;
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::TMOVEBLOCK);
    world.add_character(character);
    let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
    item.driver = crate::item_driver::IDR_TELEPORT;
    item.driver_data = vec![30, 0, 40, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
    world.add_item(item);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_TELEPORT,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(outcome, ItemDriverOutcome::Teleport { .. }));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((character.x, character.y), (30, 40));
    assert_eq!(character.action, 0);
    assert_eq!(world.map.tile(10, 10).unwrap().character, 0);
    assert_eq!(world.map.tile(30, 40).unwrap().character, 1);
}

#[test]
fn world_executes_same_area_recall_and_consumes_scroll() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    character.rest_area = 1;
    character.rest_x = 30;
    character.rest_y = 40;
    character.cursor_item = Some(ItemId(7));
    world.map.tile_mut(10, 10).unwrap().character = 1;
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::TMOVEBLOCK);
    world.add_character(character);
    let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
    item.driver = crate::item_driver::IDR_RECALL;
    item.carried_by = Some(CharacterId(1));
    item.driver_data = vec![10];
    world.add_item(item);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_RECALL,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(outcome, ItemDriverOutcome::Recall { .. }));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((character.x, character.y), (30, 40));
    assert_eq!(character.cursor_item, None);
    assert_eq!(world.map.tile(10, 10).unwrap().character, 0);
    assert_eq!(world.map.tile(30, 40).unwrap().character, 1);
    assert!(!world
        .items
        .get(&ItemId(7))
        .unwrap()
        .flags
        .contains(ItemFlags::USED));
}

#[test]
fn world_executes_same_area_city_recall_and_decrements_stack() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    character.inventory[30] = Some(ItemId(7));
    world.map.tile_mut(10, 10).unwrap().character = 1;
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::TMOVEBLOCK);
    world.add_character(character);
    let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
    item.driver = crate::item_driver::IDR_CITY_RECALL;
    item.carried_by = Some(CharacterId(1));
    item.driver_data = vec![0, 3];
    world.add_item(item);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_CITY_RECALL,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(outcome, ItemDriverOutcome::CityRecall { .. }));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((character.x, character.y), (126, 179));
    assert_eq!(character.inventory[30], Some(ItemId(7)));
    assert_eq!(world.items.get(&ItemId(7)).unwrap().driver_data[1], 2);
    assert_eq!(world.map.tile(10, 10).unwrap().character, 0);
    assert_eq!(world.map.tile(126, 179).unwrap().character, 1);
}

#[test]
fn world_consumes_final_city_recall_before_cross_area_handoff() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    character.cursor_item = Some(ItemId(7));
    world.map.tile_mut(10, 10).unwrap().character = 1;
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::TMOVEBLOCK);
    world.add_character(character);
    let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
    item.driver = crate::item_driver::IDR_CITY_RECALL;
    item.carried_by = Some(CharacterId(1));
    item.driver_data = vec![1, 1];
    world.add_item(item);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_CITY_RECALL,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::CityRecall {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            x: 167,
            y: 188,
            area_id: 3,
        }
    );
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((character.x, character.y), (10, 10));
    assert_eq!(character.cursor_item, None);
    assert!(!world
        .items
        .get(&ItemId(7))
        .unwrap()
        .flags
        .contains(ItemFlags::USED));
}

#[test]
fn world_executes_teufel_arena_exit_same_area_teleport() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.values[0][CharacterValue::Hp as usize] = 100;
    actor.hp = 100 * POWERSCALE;
    world.spawn_character(actor, 150, 220);
    let mut exit = item(8, ItemFlags::USED | ItemFlags::USE);
    exit.driver = crate::item_driver::IDR_TEUFELARENAEXIT;
    world.add_item(exit);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_TEUFELARENAEXIT,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        34,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::TeufelArenaExit {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            x: 206,
            y: 231,
        }
    );
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((actor.x, actor.y), (206, 231));
}

#[test]
fn world_applies_player_teleport_as_facing_item_use() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    character.dir = Direction::Right as u8;
    world.add_character(character);
    let mut item = item(7, ItemFlags::USED | ItemFlags::USE);
    assert!(world.map.set_item_map(&mut item, 11, 10));
    world.add_item(item);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Teleport,
        arg1: 5,
        arg2: 0,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::USE);
    assert_eq!((character.act1, character.act2), (7, 6));
    assert_eq!(character.dir, Direction::Right as u8);
}

#[test]
fn staffer2_animation_book_teleports_without_granting_core_exp() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.level = 60;
    assert!(world.spawn_character(player, 10, 10));
    let mut book = item(8, ItemFlags::USED | ItemFlags::USE);
    book.driver = IDR_STAFFER2;
    book.driver_data = vec![6];
    world.add_item(book);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_STAFFER2,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        29,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::StafferAnimationBook {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            exp_added: 177_168,
        }
    );
    let player = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((player.x, player.y), (25, 114));
    assert_eq!(player.exp, 0);
}

#[test]
fn world_applies_clanspawn_exit_same_area_rest_teleport() {
    let mut world = World::default();
    let mut exit = item(8, ItemFlags::USED | ItemFlags::USE);
    exit.driver = crate::item_driver::IDR_CLANSPAWNEXIT;
    assert!(world.map.set_item_map(&mut exit, 10, 10));
    world.add_item(exit);
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.rest_area = 30;
    player.rest_x = 12;
    player.rest_y = 13;
    assert!(world.spawn_character(player, 10, 10));

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_CLANSPAWNEXIT,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        30,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::ClanSpawnExit {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            area_id: 30,
            x: 12,
            y: 13,
        }
    );
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((character.x, character.y), (12, 13));
    assert_eq!(world.map.tile(10, 10).unwrap().character, 0);
    assert_eq!(world.map.tile(12, 13).unwrap().character, 1);
}

#[test]
fn world_warpteleport_spheres_teleports_and_consumes_all_spheres() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    actor.cursor_item = Some(ItemId(9));
    actor.inventory[30] = Some(ItemId(10));
    actor.inventory[31] = Some(ItemId(11));
    assert!(world.spawn_character(actor, 10, 10));

    let mut portal = item(7, ItemFlags::USED | ItemFlags::USE);
    portal.driver = crate::item_driver::IDR_WARPTELEPORT;
    portal.driver_data = vec![2];
    world.add_item(portal);

    let mut cursor_sphere = item(9, ItemFlags::USED);
    cursor_sphere.template_id = IID_AREA25_TELEKEY;
    cursor_sphere.carried_by = Some(CharacterId(1));
    cursor_sphere.driver_data = vec![3];
    world.add_item(cursor_sphere);

    let mut inventory_sphere = item(10, ItemFlags::USED);
    inventory_sphere.template_id = IID_AREA25_TELEKEY;
    inventory_sphere.carried_by = Some(CharacterId(1));
    inventory_sphere.driver_data = vec![1];
    world.add_item(inventory_sphere);

    let mut other_item = item(11, ItemFlags::USED);
    other_item.template_id = 123;
    other_item.carried_by = Some(CharacterId(1));
    world.add_item(other_item);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_WARPTELEPORT,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        25,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::WarpTeleportSpheres { .. }
    ));
    let actor = &world.characters[&CharacterId(1)];
    assert_eq!((actor.x, actor.y), (251, 41));
    assert_eq!(actor.cursor_item, None);
    assert_eq!(actor.inventory[30], None);
    assert_eq!(actor.inventory[31], Some(ItemId(11)));
    assert!(!world.items.contains_key(&ItemId(9)));
    assert!(!world.items.contains_key(&ItemId(10)));
    assert!(world.items.contains_key(&ItemId(11)));
}

#[test]
fn world_warpteleport_plain_busy_preserves_legacy_feedback_boundary() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.flags.insert(CharacterFlags::PLAYER);
    assert!(world.spawn_character(actor, 10, 10));
    let blocker = character(2);
    assert!(world.spawn_character(blocker, 242, 252));

    let mut portal = item(7, ItemFlags::USED | ItemFlags::USE);
    portal.driver = crate::item_driver::IDR_WARPTELEPORT;
    portal.driver_data = vec![0, 1];
    world.add_item(portal);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_WARPTELEPORT,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        25,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::WarpTeleportBusy {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
    let actor = &world.characters[&CharacterId(1)];
    assert_eq!((actor.x, actor.y), (10, 10));
}
