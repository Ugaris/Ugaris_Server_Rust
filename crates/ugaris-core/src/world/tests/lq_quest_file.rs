use super::*;

fn god(world: &mut World, id: u32, x: u16, y: u16) -> CharacterId {
    let character_id = CharacterId(id);
    let mut spawned = character(id);
    spawned.flags = CharacterFlags::USED | CharacterFlags::GOD;
    spawned.x = x;
    spawned.y = y;
    world.characters.insert(character_id, spawned);
    character_id
}

fn plain_player(world: &mut World, id: u32) -> CharacterId {
    let character_id = CharacterId(id);
    world.characters.insert(character_id, character(id));
    character_id
}

fn error_text(world: &mut World) -> String {
    let mut bytes = world.drain_pending_system_text_bytes();
    assert_eq!(bytes.len(), 1, "expected exactly one queued error message");
    let message = bytes.remove(0).message;
    String::from_utf8_lossy(&message[crate::text::COL_LIGHT_RED.len()..]).into_owned()
}

fn door(world: &mut World, id: u32, nick: &str, x: u16, y: u16) {
    let mut door = item(id, ItemFlags::USED | ItemFlags::USE);
    door.driver = IDR_DOOR;
    door.name = nick.to_string();
    door.driver_data = vec![0; 11];
    door.driver_data[10] = 1;
    door.x = x;
    door.y = y;
    world.add_item(door);
}

// ---- try_dispatch_lq_quest_file gate/parsing ----

#[test]
fn not_matched_outside_area_20_or_35() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    assert!(matches!(
        world.try_dispatch_lq_quest_file(caller, 1, "#questsave foo"),
        LqQuestFileDispatch::NotMatched
    ));
}

#[test]
fn not_matched_without_god_or_lqmaster() {
    let mut world = World::default();
    let caller = plain_player(&mut world, 1);
    assert!(matches!(
        world.try_dispatch_lq_quest_file(caller, 20, "#questsave foo"),
        LqQuestFileDispatch::NotMatched
    ));
}

#[test]
fn not_matched_for_unrelated_words() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    assert!(matches!(
        world.try_dispatch_lq_quest_file(caller, 20, "#questlevel 1 10"),
        LqQuestFileDispatch::NotMatched
    ));
    assert!(matches!(
        world.try_dispatch_lq_quest_file(caller, 20, "#questentrance"),
        LqQuestFileDispatch::NotMatched
    ));
}

#[test]
fn questsave_missing_name_reports_usage() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    assert!(matches!(
        world.try_dispatch_lq_quest_file(caller, 20, "#questsave"),
        LqQuestFileDispatch::Rejected
    ));
    assert_eq!(
        error_text(&mut world),
        "Missing name. Usage is: /questsave <name> [password]."
    );
}

#[test]
fn questdelete_missing_name_reports_the_questdel_usage_string() {
    // C quirk: `cmd_questdel`'s own `usage` local says "/questdel", not
    // "/questdelete", even though the dispatch keyword is "questdelete".
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    assert!(matches!(
        world.try_dispatch_lq_quest_file(caller, 20, "#questdelete"),
        LqQuestFileDispatch::Rejected
    ));
    assert_eq!(
        error_text(&mut world),
        "Missing name. Usage is: /questdel <name> [password]."
    );
}

#[test]
fn questload_missing_name_reports_usage() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    assert!(matches!(
        world.try_dispatch_lq_quest_file(caller, 20, "#questload"),
        LqQuestFileDispatch::Rejected
    ));
    assert_eq!(
        error_text(&mut world),
        "Missing name. Usage is: /questload <name> [password]."
    );
}

#[test]
fn rejects_illegal_character_in_name() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    assert!(matches!(
        world.try_dispatch_lq_quest_file(caller, 20, "#questsave my1quest"),
        LqQuestFileDispatch::Rejected
    ));
    assert_eq!(error_text(&mut world), "Name contains illegal character 1.");
}

#[test]
fn rejects_trailing_garbage() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    assert!(matches!(
        world.try_dispatch_lq_quest_file(caller, 20, "#questsave foo bar baz"),
        LqQuestFileDispatch::Rejected
    ));
    assert_eq!(
        error_text(&mut world),
        "Trailing garbage. Usage is: /questsave <name> [password]."
    );
}

#[test]
fn questsave_parses_name_and_password() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    match world.try_dispatch_lq_quest_file(caller, 35, "#questsave myquest secret") {
        LqQuestFileDispatch::Save { name, password } => {
            assert_eq!(name, "myquest");
            assert_eq!(password, "secret");
        }
        _ => panic!("expected Save"),
    }
}

#[test]
fn questsave_defaults_password_to_empty() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    match world.try_dispatch_lq_quest_file(caller, 20, "#questsave myquest") {
        LqQuestFileDispatch::Save { name, password } => {
            assert_eq!(name, "myquest");
            assert_eq!(password, "");
        }
        _ => panic!("expected Save"),
    }
}

#[test]
fn questdelete_parses_name_and_password() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    match world.try_dispatch_lq_quest_file(caller, 20, "#questdelete myquest secret") {
        LqQuestFileDispatch::Delete { name, password } => {
            assert_eq!(name, "myquest");
            assert_eq!(password, "secret");
        }
        _ => panic!("expected Delete"),
    }
}

#[test]
fn questload_parses_name_and_password() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    match world.try_dispatch_lq_quest_file(caller, 20, "#questload myquest secret") {
        LqQuestFileDispatch::Load { name, password } => {
            assert_eq!(name, "myquest");
            assert_eq!(password, "secret");
        }
        _ => panic!("expected Load"),
    }
}

#[test]
fn slash_prefix_is_also_accepted() {
    let mut world = World::default();
    let caller = god(&mut world, 1, 5, 5);
    assert!(matches!(
        world.try_dispatch_lq_quest_file(caller, 20, "/questsave myquest"),
        LqQuestFileDispatch::Save { .. }
    ));
}

// ---- lq_quest_snapshot / apply_lq_quest_snapshot ----

#[test]
fn snapshot_round_trips_data_and_npcs() {
    let mut world = World::default();
    world.lq_data.min_level = 5;
    world.lq_data.max_level = 50;
    world.lq_data.open = true;
    world.configure_lq_npc(LqNpcState {
        slot: 1,
        basename: "base".to_string(),
        x: 10,
        y: 20,
        dir: 0,
        level: 1,
        mode: b'a',
        respawn_seconds: 60,
        name: "Test NPC".to_string(),
        description: String::new(),
        nick: ["nicka".to_string(), "nickb".to_string()],
        character_id: None,
        character_serial: 0,
        sprite: 0,
        greeting: String::new(),
        trigger: Default::default(),
        reply: Default::default(),
        want_key_id: 0,
        reward_item: LqItemSpec::default(),
        reward_mark_id: 0,
        kill_mark_id: 0,
        hurt_mark_id: 0,
        carry_item: LqItemSpec::default(),
        carry_gold: 0,
    });

    let snapshot = world.lq_quest_snapshot();
    assert_eq!(snapshot.data.min_level, 5);
    assert_eq!(snapshot.npcs.len(), 1);
    assert_eq!(snapshot.npcs[0].name, "Test NPC");

    // Round-trip through JSON, exactly like the on-disk file would.
    let json = serde_json::to_string(&snapshot).unwrap();
    let restored: LqQuestSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, snapshot);

    let mut fresh_world = World::default();
    fresh_world.apply_lq_quest_snapshot(restored);
    assert_eq!(fresh_world.lq_data.min_level, 5);
    assert_eq!(fresh_world.lq_data.max_level, 50);
    // C `lq_data.open = 0;` after load - always cleared regardless of the
    // saved value.
    assert!(!fresh_world.lq_data.open);
    assert_eq!(fresh_world.lq_npcs.len(), 1);
    assert_eq!(fresh_world.lq_npcs[0].name, "Test NPC");
}

#[test]
fn apply_snapshot_restores_door_key_ids_by_slot_after_rescan() {
    let mut world = World::default();
    door(&mut world, 10, "north-gate", 12, 34);
    door(&mut world, 11, "south-gate", 56, 78);
    world.discover_lq_doors_once();
    // Lock the first discovered door directly (bypassing #doorlock, same
    // effect).
    let slot = world.lq_doors[0].slot;
    let item_id = world.lq_doors[0].item_id;
    world.lq_doors[0].key_id = 42;
    write_lq_door_key_id(world.items.get_mut(&item_id).unwrap(), 42);

    let snapshot = world.lq_quest_snapshot();
    assert_eq!(snapshot.doors.len(), 2);

    // Simulate a fresh server start: fresh World, same map doors, no
    // in-memory door state yet.
    let mut fresh_world = World::default();
    door(&mut fresh_world, 10, "north-gate", 12, 34);
    door(&mut fresh_world, 11, "south-gate", 56, 78);
    fresh_world.apply_lq_quest_snapshot(snapshot);

    assert!(fresh_world.lq_doors_initialized);
    let restored_door = fresh_world
        .lq_doors
        .iter()
        .find(|door| door.slot == slot)
        .unwrap();
    assert_eq!(restored_door.key_id, 42);
    // `MAKE_ITEMID(DEV_ID_LQ, 42)` little-endian: `key_id` (42) in the
    // bottom byte, `DEV_ID_LQ` (5) in the top byte.
    assert_eq!(
        &fresh_world.items[&ItemId(10)].driver_data[1..5],
        &[42, 0, 0, 5]
    );
    // The second door's key was never set, so it stays at 0.
    let other_door = fresh_world
        .lq_doors
        .iter()
        .find(|door| door.item_id == ItemId(11))
        .unwrap();
    assert_eq!(other_door.key_id, 0);
}

#[test]
fn apply_snapshot_clears_stale_lq_doors_before_rescanning() {
    let mut world = World::default();
    world.lq_doors_initialized = true;
    world.lq_doors.push(LqDoorState {
        slot: 1,
        item_id: ItemId(999),
        nick: "stale".to_string(),
        key_id: 7,
    });

    let snapshot = LqQuestSnapshot {
        data: LqData::default(),
        npcs: Vec::new(),
        doors: Vec::new(),
    };
    world.apply_lq_quest_snapshot(snapshot);
    assert!(world.lq_doors.is_empty());
}
