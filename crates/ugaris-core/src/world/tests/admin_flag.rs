use super::*;

fn spawn_character(world: &mut World, id: u32, name: &str) -> CharacterId {
    let character_id = CharacterId(id);
    let mut spawned = character(id);
    spawned.name = name.to_string();
    world.characters.insert(character_id, spawned);
    character_id
}

#[test]
fn online_target_toggles_immediately_and_names_the_flag() {
    let mut world = World::default();
    let caller_id = spawn_character(&mut world, 1, "Caller");
    spawn_character(&mut world, 2, "Godmode");

    let messages = world.apply_cmd_flag_command(caller_id, "Godmode", CharacterFlags::GOD, "god");

    assert_eq!(messages, vec!["Set Godmode god to on.".to_string()]);
    assert!(world.characters[&CharacterId(2)]
        .flags
        .contains(CharacterFlags::GOD));
    assert!(world.drain_pending_admin_flag_toggles().is_empty());
}

#[test]
fn online_target_toggling_off_reports_off() {
    let mut world = World::default();
    let caller_id = spawn_character(&mut world, 1, "Caller");
    let target_id = spawn_character(&mut world, 2, "Godmode");
    world
        .characters
        .get_mut(&target_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::HARDCORE);

    let messages =
        world.apply_cmd_flag_command(caller_id, "Godmode", CharacterFlags::HARDCORE, "hardcore");

    assert_eq!(messages, vec!["Set Godmode hardcore to off.".to_string()]);
}

#[test]
fn online_scan_matches_case_insensitively_and_ignores_no_cf_player_filter() {
    // C's `getfirst_char`/`getnext_char` walk has no `CF_PLAYER` check
    // (unlike `find_online_player_by_name`), so an NPC-flagged loaded
    // character still matches.
    let mut world = World::default();
    let caller_id = spawn_character(&mut world, 1, "Caller");
    spawn_character(&mut world, 2, "SomeNpc");

    let messages =
        world.apply_cmd_flag_command(caller_id, "somenpc", CharacterFlags::STAFF, "staff");

    assert_eq!(messages, vec!["Set SomeNpc staff to on.".to_string()]);
}

#[test]
fn invalid_shape_name_is_rejected_immediately_without_queuing() {
    let mut world = World::default();
    let caller_id = spawn_character(&mut world, 1, "Caller");

    let messages = world.apply_cmd_flag_command(caller_id, "a1", CharacterFlags::GOD, "god");

    assert_eq!(
        messages,
        vec!["Sorry, no player by the name a1.".to_string()]
    );
    assert!(world.drain_pending_admin_flag_toggles().is_empty());
}

#[test]
fn empty_name_is_rejected_immediately() {
    let mut world = World::default();
    let caller_id = spawn_character(&mut world, 1, "Caller");

    let messages = world.apply_cmd_flag_command(caller_id, "", CharacterFlags::GOD, "god");

    assert_eq!(messages, vec!["Sorry, no player by the name .".to_string()]);
}

#[test]
fn validly_shaped_unmatched_name_is_queued_with_no_immediate_message() {
    let mut world = World::default();
    let caller_id = spawn_character(&mut world, 1, "Caller");

    let messages =
        world.apply_cmd_flag_command(caller_id, "Nobodyhome", CharacterFlags::LQMASTER, "qmaster");

    assert!(messages.is_empty());
    let queued = world.drain_pending_admin_flag_toggles();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, caller_id);
    assert_eq!(queued[0].target_name, "Nobodyhome");
    assert_eq!(queued[0].flag, CharacterFlags::LQMASTER);
}
