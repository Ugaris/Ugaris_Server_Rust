use super::*;

#[test]
fn lq_ticker_discovers_legacy_doors_once_and_writes_key_id() {
    let mut world = World::default();
    world.add_character(character(0));

    let mut ticker = item(7, ItemFlags::USED | ItemFlags::USE);
    ticker.driver = crate::item_driver::IDR_LQ_TICKER;
    world.add_item(ticker);

    let mut door = item(10, ItemFlags::USED | ItemFlags::USE);
    door.driver = IDR_DOOR;
    door.name = "north-gate".to_string();
    door.driver_data = vec![0; 11];
    door.driver_data[10] = 1;
    world.add_item(door);

    let mut ordinary_door = item(11, ItemFlags::USED | ItemFlags::USE);
    ordinary_door.driver = IDR_DOOR;
    ordinary_door.driver_data = vec![0; 11];
    world.add_item(ordinary_door);

    let outcome = world.execute_item_driver_timer_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_LQ_TICKER,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        20,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert!(matches!(outcome, ItemDriverOutcome::LqTicker { .. }));
    assert!(world.lq_doors_initialized);
    assert_eq!(world.lq_doors.len(), 1);
    assert_eq!(world.lq_doors[0].slot, 1);
    assert_eq!(world.lq_doors[0].item_id, ItemId(10));
    assert_eq!(world.lq_doors[0].nick, "north-gate");
    assert_eq!(&world.items[&ItemId(10)].driver_data[1..5], &[0, 0, 0, 5]);
    assert_eq!(&world.items[&ItemId(11)].driver_data[1..5], &[0, 0, 0, 0]);

    world.items.get_mut(&ItemId(10)).unwrap().driver_data[1] = 77;
    let _ = world.execute_item_driver_timer_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_LQ_TICKER,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        20,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );
    assert_eq!(world.lq_doors.len(), 1);
    assert_eq!(world.items[&ItemId(10)].driver_data[1], 77);
}

#[test]
fn lq_ticker_queues_due_npc_respawns_and_clears_schedule() {
    let mut world = World {
        tick: Tick(200),
        ..World::default()
    };
    world.add_character(character(0));

    let mut ticker = item(7, ItemFlags::USED | ItemFlags::USE);
    ticker.driver = crate::item_driver::IDR_LQ_TICKER;
    world.add_item(ticker);

    assert!(world.configure_lq_npc(LqNpcState {
        slot: 3,
        basename: "guard_base".to_string(),
        x: 45,
        y: 67,
        dir: Direction::Down as u8,
        level: 42,
        mode: b'a',
        respawn_seconds: 30,
        name: "Gate Guard".to_string(),
        description: "A stern live quest guard.".to_string(),
        nick: ["guard".to_string(), "gate".to_string()],
        character_id: None,
        character_serial: 0,
        sprite: 0,
        greeting: String::new(),
        trigger: [
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new()
        ],
        reply: [
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new()
        ],
        want_key_id: 0,
        reward_item: LqItemSpec::default(),
        reward_mark_id: 0,
        kill_mark_id: 0,
        hurt_mark_id: 0,
        carry_item: LqItemSpec::default(),
        carry_gold: 0,
    }));
    assert!(world.schedule_lq_npc_respawn(3, 199));
    assert!(!world.schedule_lq_npc_respawn(0, 199));
    assert!(!world.schedule_lq_npc_respawn(MAX_LQ_NPCS, 199));

    let outcome = world.execute_item_driver_timer_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_LQ_TICKER,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        20,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert!(matches!(outcome, ItemDriverOutcome::LqTicker { .. }));
    assert!(world.lq_npc_respawns.is_empty());
    assert_eq!(
        world.drain_pending_lq_npc_spawns(),
        vec![LqNpcSpawnRequest {
            slot: 3,
            basename: "guard_base".to_string(),
            x: 45,
            y: 67,
            dir: Direction::Down as u8,
            level: 42,
            mode: b'a',
            name: "Gate Guard".to_string(),
            description: "A stern live quest guard.".to_string(),
            nick: ["guard".to_string(), "gate".to_string()],
            sprite: 0,
            greeting: String::new(),
            trigger: [
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new()
            ],
            reply: [
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new()
            ],
            want_key_id: 0,
            reward_item: LqItemSpec::default(),
            reward_mark_id: 0,
            kill_mark_id: 0,
            hurt_mark_id: 0,
            carry_item: LqItemSpec::default(),
            carry_gold: 0,
        }]
    );
}

#[test]
fn lq_ticker_keeps_future_npc_respawns_pending() {
    let mut world = World {
        tick: Tick(200),
        ..World::default()
    };
    world.add_character(character(0));

    let mut ticker = item(7, ItemFlags::USED | ItemFlags::USE);
    ticker.driver = crate::item_driver::IDR_LQ_TICKER;
    world.add_item(ticker);
    assert!(world.configure_lq_npc(LqNpcState {
        slot: 2,
        basename: "rat".to_string(),
        x: 1,
        y: 2,
        dir: Direction::Left as u8,
        level: 1,
        mode: b'n',
        respawn_seconds: 10,
        name: String::new(),
        description: String::new(),
        nick: [String::new(), String::new()],
        character_id: None,
        character_serial: 0,
        sprite: 0,
        greeting: String::new(),
        trigger: [
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new()
        ],
        reply: [
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new()
        ],
        want_key_id: 0,
        reward_item: LqItemSpec::default(),
        reward_mark_id: 0,
        kill_mark_id: 0,
        hurt_mark_id: 0,
        carry_item: LqItemSpec::default(),
        carry_gold: 0,
    }));
    assert!(world.schedule_lq_npc_respawn(2, 201));

    let _ = world.execute_item_driver_timer_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_LQ_TICKER,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        20,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(world.lq_npc_respawns, vec![(2, 201)]);
    assert!(world.drain_pending_lq_npc_spawns().is_empty());
}

#[test]
fn lq_spawn_result_records_live_character_identity() {
    let mut world = World::default();
    assert!(world.configure_lq_npc(LqNpcState {
        slot: 4,
        basename: "guard".to_string(),
        x: 10,
        y: 11,
        dir: Direction::RightDown as u8,
        level: 12,
        mode: b'f',
        respawn_seconds: 30,
        name: String::new(),
        description: String::new(),
        nick: [String::new(), String::new()],
        character_id: None,
        character_serial: 0,
        sprite: 0,
        greeting: String::new(),
        trigger: [
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new()
        ],
        reply: [
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new()
        ],
        want_key_id: 0,
        reward_item: LqItemSpec::default(),
        reward_mark_id: 0,
        kill_mark_id: 0,
        hurt_mark_id: 0,
        carry_item: LqItemSpec::default(),
        carry_gold: 0,
    }));

    assert!(world.apply_lq_npc_spawn_result(4, CharacterId(77), 12345));
    let npc = world.lq_npcs.iter().find(|npc| npc.slot == 4).unwrap();
    assert_eq!(npc.character_id, Some(CharacterId(77)));
    assert_eq!(npc.character_serial, 12345);
    assert!(!world.apply_lq_npc_spawn_result(5, CharacterId(78), 1));
}
