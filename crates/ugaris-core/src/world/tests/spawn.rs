use super::*;

#[test]
fn chestspawn_spawn_result_marks_active_and_schedules_poll() {
    let mut world = World::default();
    let mut spawner = item(8, ItemFlags::USE);
    spawner.driver = IDR_CHESTSPAWN;
    spawner.sprite = 1234;
    spawner.x = 10;
    spawner.y = 10;
    spawner.driver_data = vec![0, 0, 0, 0, 0, 0, 0, 0];
    world.items.insert(spawner.id, spawner);

    assert!(world.apply_chestspawn_spawn_result(ItemId(8), CharacterId(44), 0));
    let spawner = &world.items[&ItemId(8)];
    assert_eq!(spawner.sprite, 1235);
    assert_eq!(spawner.driver_data[1], 1);
    assert_eq!(&spawner.driver_data[2..4], &44_u16.to_le_bytes());
    assert_eq!(world.process_due_timers(2), Vec::<ItemDriverOutcome>::new());
    world.tick.0 = TICKS_PER_SECOND * 10;
    let outcomes = world.process_due_timers(2);
    assert_eq!(outcomes.len(), 1);
}

#[test]
fn world_spawns_and_removes_character_on_map() {
    let mut world = World::default();

    assert!(world.spawn_character(character(1), 10, 10));
    assert_eq!(world.map.tile(10, 10).unwrap().character, 1);
    assert!(!world.spawn_character(character(1), 11, 10));

    let removed = world.remove_character(CharacterId(1)).unwrap();
    assert_eq!(removed.id, CharacterId(1));
    assert_eq!(world.map.tile(10, 10).unwrap().character, 0);
}

#[test]
fn world_applies_clanspawn_exit_busy_target_feedback_outcome() {
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
    let mut blocker_id = 2;
    for y in 12..=14 {
        for x in 11..=13 {
            assert!(world.spawn_character(character(blocker_id), x, y));
            blocker_id += 1;
        }
    }

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
        ItemDriverOutcome::ClanSpawnExitBusy {
            item_id: ItemId(8),
            character_id: CharacterId(1),
        }
    );
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((character.x, character.y), (10, 10));
}
