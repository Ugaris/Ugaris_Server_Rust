use super::*;

#[test]
pub(crate) fn goto_command_requires_lqmaster_permission() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    assert!(world.spawn_character(
        login_character(character_id, &login_block("Ralph"), 1, 10, 10),
        10,
        10
    ));
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/goto 50 60",
        1
    )
    .is_none());
    assert_eq!(world.characters.get(&character_id).unwrap().x, 10);

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/goto 50 60",
        1
    )
    .is_some());
}

#[test]
pub(crate) fn goto_command_numeric_coordinates_teleport_same_area() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(character, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/goto 50 60", 1)
            .expect("god goto command should be recognized");
    assert!(result.messages.is_empty());
    assert_eq!(result.mirror_changed, None);
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (50, 60));
}

#[test]
pub(crate) fn goto_command_named_location_normalizes_to_same_area() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(character, 10, 10));
    let mut runtime = ServerRuntime::default();

    // "fort" is gl[]'s (126,179,1); area_id 1 matches the caller's current
    // area so C's `if (a == areaID && !m) a = 0;` normalizes this to a
    // plain same-area teleport (not a `change_area` handoff).
    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/goto fort", 1)
            .expect("god goto command should be recognized");
    assert!(result.messages.is_empty());
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (126, 179));
}

#[test]
pub(crate) fn goto_command_named_location_cross_area_requests_transfer() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(character, 10, 10));
    let mut runtime = ServerRuntime::default();

    // "aston" is gl[]'s (167,188,3); the caller is in area 1, so C would
    // call `change_area(cn, 3, 167, 188)` - the command layer defers to
    // the `main.rs` call site's `attempt_cross_area_transfer` via the
    // `cross_area_transfer` field, matching every other cross-area
    // teleport site in this codebase.
    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/goto aston", 1)
            .expect("god goto command should be recognized");
    assert!(result.messages.is_empty());
    assert_eq!(result.cross_area_transfer, Some((3, 167, 188)));
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (10, 10));
}

#[test]
pub(crate) fn goto_command_non_god_lqmaster_ignores_cross_area_and_uses_local_coords() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 10, 10);
    character.flags.insert(CharacterFlags::EVENTMASTER);
    assert!(world.spawn_character(character, 10, 10));
    let mut runtime = ServerRuntime::default();

    // C `if (!(ch[cn].flags & CF_GOD)) a = 0;` forces non-GOD `is_lqmaster`
    // callers (here: `CF_EVENTMASTER`) to always land locally, using the
    // resolved x/y but ignoring the resolved area entirely - even though
    // "aston" nominally lives in a different area.
    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/goto aston", 1)
            .expect("eventmaster goto command should be recognized");
    assert!(result.messages.is_empty());
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (167, 188));
}

#[test]
pub(crate) fn goto_command_looks_up_online_character_by_name() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let target_id = CharacterId(2);
    assert!(world.spawn_character(
        login_character(target_id, &login_block("Lisa"), 1, 77, 88),
        77,
        88
    ));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/goto Lisa", 1)
            .expect("god goto command should be recognized");
    assert!(result.messages.is_empty());
    let caller = world.characters.get(&caller_id).unwrap();
    // Target's own tile is occupied, so `drop_char`'s neighbor fallback
    // (matching C's own `teleport_char_driver`) lands the caller on an
    // adjacent tile rather than exactly on top of the target.
    let dx = i32::from(caller.x) - 77;
    let dy = i32::from(caller.y) - 88;
    assert!(dx.abs() <= 1 && dy.abs() <= 1 && (dx, dy) != (0, 0));
}

#[test]
pub(crate) fn goto_command_direction_shorthand_offsets_from_caller_position() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 100, 100);
    character.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(character, 100, 100));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/goto 10 n", 1)
            .expect("god goto command should be recognized");
    assert!(result.messages.is_empty());
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (90, 90));
}

#[test]
pub(crate) fn goto_command_mirror_argument_always_forces_cross_area_handoff() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(character, 10, 10));
    let mut runtime = ServerRuntime::default();

    // C sets `ch[cn].mirror = m` unconditionally and forces `a = areaID`
    // when it was still 0, which then *fails* the `a == areaID && !m`
    // same-area normalization (because `m != 0`) - so requesting a mirror
    // always routes through `change_area`, even when the area number is
    // literally the caller's own current area. Copied as-is (a real C
    // quirk, not a Rust bug): the mirror still gets set and the command
    // layer still requests a cross-area transfer via `cross_area_transfer`
    // even though the target area number equals the caller's own.
    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/goto 50 60 0 3", 1)
            .expect("god goto command should be recognized");
    assert!(result.messages.is_empty());
    assert_eq!(result.cross_area_transfer, Some((1, 50, 60)));
    assert_eq!(result.mirror_changed, Some(3));
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (10, 10));
}

#[test]
pub(crate) fn jump_command_requires_staff_or_god() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    assert!(world.spawn_character(
        login_character(character_id, &login_block("Ralph"), 1, 10, 10),
        10,
        10
    ));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/jump fort", 1)
            .is_none()
    );
}

#[test]
pub(crate) fn jump_command_refuses_while_busy() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 10, 10);
    character.flags.insert(CharacterFlags::STAFF);
    character.action = 1;
    assert!(world.spawn_character(character, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/jump fort", 1)
            .expect("staff jump command should be recognized");
    assert_eq!(result.messages, vec!["Pant, pant. Too tired.".to_string()]);
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (10, 10));
}

#[test]
pub(crate) fn jump_command_moves_staff_to_gotolist_entry_in_same_area() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 10, 10);
    character.flags.insert(CharacterFlags::STAFF);
    assert!(world.spawn_character(character, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/jump fort", 1)
            .expect("staff jump command should be recognized");
    assert!(result.messages.is_empty());
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (126, 179));
}

#[test]
pub(crate) fn jump_command_cross_area_is_not_restricted_to_god() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 10, 10);
    character.flags.insert(CharacterFlags::STAFF);
    assert!(world.spawn_character(character, 10, 10));
    let mut runtime = ServerRuntime::default();

    // Unlike `/goto`, C's `/jump` has no `CF_GOD`-only restriction on the
    // cross-area branch - a plain `CF_STAFF` caller jumping to a
    // different-area `gl[]` entry ("aston" is area 3) still reaches
    // `change_area` in C, so it requests the same `cross_area_transfer`
    // handoff here.
    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/jump aston", 1)
            .expect("staff jump command should be recognized");
    assert!(result.messages.is_empty());
    assert_eq!(result.cross_area_transfer, Some((3, 167, 188)));
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!((character.x, character.y), (10, 10));
}

#[test]
pub(crate) fn jump_command_unknown_location_reports_hu() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 10, 10);
    character.flags.insert(CharacterFlags::STAFF);
    assert!(world.spawn_character(character, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/jump nowhere", 1)
            .expect("staff jump command should be recognized");
    assert_eq!(result.messages, vec!["hu?".to_string()]);
}

#[test]
pub(crate) fn gotolist_command_is_god_only_and_lists_every_shortcut() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    assert!(world.spawn_character(
        login_character(character_id, &login_block("Ralph"), 1, 10, 10),
        10,
        10
    ));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/gotolist", 1)
            .is_none()
    );

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/gotolist", 1)
            .expect("god gotolist command should be recognized");
    assert_eq!(result.messages[0], "Available /goto locations:");
    assert!(result
        .messages
        .contains(&"aston (x:167, y:188, area:3)".to_string()));
    assert!(result
        .messages
        .contains(&"teufelearthgambler (x:248, y:238, area:34)".to_string()));
    assert_eq!(result.messages.len(), 1 + 79);
}

#[test]
pub(crate) fn gotosearch_command_is_case_sensitive_like_c_strstr() {
    let mut world = goto_test_world();
    let character_id = CharacterId(1);
    let mut character = login_character(character_id, &login_block("Ralph"), 1, 10, 10);
    character.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(character, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/gotosearch teufel",
        1,
    )
    .expect("god gotosearch command should be recognized");
    assert_eq!(result.messages[0], "Matching /goto locations:");
    assert!(result
        .messages
        .contains(&"teufelicegambler (x:84, y:186, area:34)".to_string()));
    assert!(result
        .messages
        .contains(&"Found 5 matching locations.".to_string()));

    // C `strstr`, not `strcasestr` - an uppercase term matches nothing.
    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/gotosearch TEUFEL",
        1,
    )
    .expect("god gotosearch command should be recognized");
    assert_eq!(
        result.messages,
        vec![
            "Matching /goto locations:".to_string(),
            "No matching locations found.".to_string()
        ]
    );
}

#[test]
pub(crate) fn summon_command_requires_god() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    assert!(world.spawn_character(
        login_character(caller_id, &login_block("Ralph"), 1, 10, 10),
        10,
        10
    ));
    let target_id = CharacterId(2);
    assert!(world.spawn_character(
        login_character(target_id, &login_block("Lydia"), 1, 90, 90),
        90,
        90
    ));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/summon Lydia", 1)
            .is_none()
    );
    assert_eq!(world.characters.get(&target_id).unwrap().x, 90);
}

#[test]
pub(crate) fn summon_command_teleports_named_character_next_to_caller() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let target_id = CharacterId(2);
    assert!(world.spawn_character(
        login_character(target_id, &login_block("Lydia"), 1, 90, 90),
        90,
        90
    ));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/summon lydia", 1)
            .expect("god summon command should be recognized");
    assert!(result.messages.is_empty());
    let target = world.characters.get(&target_id).unwrap();
    assert!((i32::from(target.x) - 10).abs() + (i32::from(target.y) - 10).abs() < 2);
}

#[test]
pub(crate) fn summon_command_unknown_name_is_a_silent_no_op() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/summon Nobody", 1)
            .expect("god summon command should be recognized");
    assert!(result.messages.is_empty());
}

#[test]
pub(crate) fn kick_command_requires_staff_or_god() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    assert!(world.spawn_character(
        login_character(caller_id, &login_block("Ralph"), 1, 10, 10),
        10,
        10
    ));
    let target_id = CharacterId(2);
    assert!(world.spawn_character(
        login_character(target_id, &login_block("Lydia"), 1, 90, 90),
        90,
        90
    ));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/kick Lydia", 1)
            .is_none()
    );
    assert!(world.characters.contains_key(&target_id));
}

#[test]
pub(crate) fn kick_command_signals_target_teardown_for_staff() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::STAFF);
    assert!(world.spawn_character(caller, 10, 10));
    let target_id = CharacterId(2);
    assert!(world.spawn_character(
        login_character(target_id, &login_block("Lydia"), 1, 90, 90),
        90,
        90
    ));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/kick lydia", 1)
            .expect("staff kick command should be recognized");
    assert_eq!(result.messages, vec!["Kicked lydia.".to_string()]);
    assert_eq!(result.kick_target, Some(target_id));
    // Command dispatch only signals the teardown; the actual save/
    // despawn/disconnect happens at the async call site in main.rs, so
    // the character is still present here.
    assert!(world.characters.contains_key(&target_id));
}

#[test]
pub(crate) fn kick_command_ignores_npcs_by_name() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let npc_id = CharacterId(2);
    let mut npc = login_character(npc_id, &login_block("Goblin"), 1, 90, 90);
    npc.flags.remove(CharacterFlags::PLAYER);
    npc.flags.insert(CharacterFlags::ALIVE);
    assert!(world.spawn_character(npc, 90, 90));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/kick goblin", 1)
            .expect("god kick command should be recognized");
    assert_eq!(
        result.messages,
        vec!["No player by the name goblin.".to_string()]
    );
    assert_eq!(result.kick_target, None);
}

#[test]
pub(crate) fn kick_command_unknown_name_reports_not_found() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/kick Nobody", 1)
            .expect("god kick command should be recognized");
    assert_eq!(
        result.messages,
        vec!["No player by the name Nobody.".to_string()]
    );
}

#[test]
pub(crate) fn summonall_command_requires_god() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    assert!(world.spawn_character(
        login_character(caller_id, &login_block("Ralph"), 1, 10, 10),
        10,
        10
    ));
    let other_id = CharacterId(2);
    assert!(world.spawn_character(
        login_character(other_id, &login_block("Lydia"), 1, 90, 90),
        90,
        90
    ));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/summonall", 1)
            .is_none()
    );
    assert_eq!(world.characters.get(&other_id).unwrap().x, 90);
}

#[test]
pub(crate) fn summonall_command_teleports_every_player_next_to_caller() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let other_a = CharacterId(2);
    assert!(world.spawn_character(
        login_character(other_a, &login_block("Lydia"), 1, 90, 90),
        90,
        90
    ));
    let other_b = CharacterId(3);
    assert!(world.spawn_character(
        login_character(other_b, &login_block("Gwendylon"), 1, 200, 200),
        200,
        200
    ));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/summonall", 1)
            .expect("god summonall command should be recognized");
    assert!(result.messages.is_empty());
    for id in [other_a, other_b] {
        let character = world.characters.get(&id).unwrap();
        assert!((i32::from(character.x) - 10).abs() + (i32::from(character.y) - 10).abs() < 2);
    }
    // The caller themselves stays put (`teleport_char_driver` is a no-op
    // under Manhattan distance 2, and the caller is already at (10,10)).
    assert_eq!(world.characters.get(&caller_id).unwrap().x, 10);
}

#[test]
pub(crate) fn summonall_command_does_not_teleport_npcs() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let npc_id = CharacterId(2);
    let mut npc = login_character(npc_id, &login_block("Goblin"), 1, 90, 90);
    npc.flags.remove(CharacterFlags::PLAYER);
    npc.flags.insert(CharacterFlags::ALIVE);
    assert!(world.spawn_character(npc, 90, 90));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/summonall", 1)
            .expect("god summonall command should be recognized");
    assert!(result.messages.is_empty());
    assert_eq!(world.characters.get(&npc_id).unwrap().x, 90);
}

#[test]
pub(crate) fn office_command_requires_god() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    assert!(world.spawn_character(
        login_character(caller_id, &login_block("Ralph"), 3, 10, 10),
        10,
        10
    ));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/office", 3).is_none()
    );
    assert_eq!(world.characters.get(&caller_id).unwrap().x, 10);
}

#[test]
pub(crate) fn office_command_teleports_within_aston() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(&mut world, &mut runtime, caller_id, "/office", 3)
        .expect("god office command should be recognized");
    assert!(result.messages.is_empty());
    let character = world.characters.get(&caller_id).unwrap();
    assert_eq!((character.x, character.y), (11, 195));
}

#[test]
pub(crate) fn office_command_from_another_area_requests_cross_area_transfer() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 1, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(&mut world, &mut runtime, caller_id, "/office", 1)
        .expect("god office command should be recognized");
    // C `change_area(cn, 3, 11, 195)` - the command layer defers to the
    // `main.rs` call site's `attempt_cross_area_transfer`, which reports
    // "Nothing happens - target area server is down." only if the
    // transfer itself fails (no DB/repositories configured in this unit
    // test's harness).
    assert!(result.messages.is_empty());
    assert_eq!(result.cross_area_transfer, Some((3, 11, 195)));
    // Position is unaffected until the call site's transfer succeeds.
    let character = world.characters.get(&caller_id).unwrap();
    assert_eq!((character.x, character.y), (10, 10));
}

#[test]
pub(crate) fn office_command_abbreviation_is_not_recognized() {
    // C `cmdcmp(ptr, "office", 6)` requires the full six-letter word;
    // there is no shorter valid abbreviation.
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/offic", 3).is_none()
    );
    assert_eq!(world.characters.get(&caller_id).unwrap().x, 10);
}

// C `/jail`/`/unjail <name>` (`command.c:8839-8882`), `CF_STAFF|CF_GOD`-
// gated, full-word only. Both commands defer to `World::
// queue_jail_lookup`/`apply_jail_events` for the actual DB round trip and
// online-scan/mutation (see `world/jail.rs`'s tests for that half), so
// these dispatch-level tests only cover permission gating, exact-word
// matching, and that a valid-looking name is queued rather than answered
// immediately.

#[test]
pub(crate) fn jail_command_requires_staff_or_god() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    assert!(world.spawn_character(
        login_character(caller_id, &login_block("Ralph"), 3, 10, 10),
        10,
        10
    ));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/jail Baddie", 3)
            .is_none()
    );
    assert!(world.drain_pending_jail_lookups().is_empty());
}

#[test]
pub(crate) fn jail_command_with_an_invalid_name_is_rejected_immediately() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    // C `lookup_name`'s `strlen(name) < 2` gate (`lookup.c:57-59`).
    let result = apply_admin_character_command(&mut world, &mut runtime, caller_id, "/jail A", 3)
        .expect("god jail command should be recognized");
    assert!(result.messages.is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].character_id, caller_id);
    assert_eq!(texts[0].message, "No character by the name A.");
    assert!(world.drain_pending_jail_lookups().is_empty());
}

#[test]
pub(crate) fn jail_command_abbreviation_is_not_recognized() {
    // C `cmdcmp(ptr, "jail", 4)` requires the full four-letter word.
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/jai Baddie", 3)
            .is_none()
    );
}

#[test]
pub(crate) fn unjail_command_abbreviation_is_not_recognized() {
    // C `cmdcmp(ptr, "unjail", 6)` requires the full six-letter word.
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/unjai Baddie", 3)
            .is_none()
    );
}

// C `/rmdeath <name>` (`command.c:8884-8903` dispatch -> `cmd_removedeath`,
// `command.c:2006-2019`), `CF_GOD`-gated, full-word only.

#[test]
pub(crate) fn rmdeath_command_requires_god() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::STAFF);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/rmdeath Baddie",
        3
    )
    .is_none());
    assert!(world.drain_pending_rmdeath_lookups().is_empty());
}

#[test]
pub(crate) fn rmdeath_command_with_an_invalid_name_is_rejected_immediately() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    // C `lookup_name`'s `strlen(name) < 2` gate (`lookup.c:57-59`).
    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/rmdeath A", 3)
            .expect("god rmdeath command should be recognized");
    assert!(result.messages.is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].character_id, caller_id);
    assert_eq!(texts[0].message, "No character by the name A.");
    assert!(world.drain_pending_rmdeath_lookups().is_empty());
}

#[test]
pub(crate) fn rmdeath_command_abbreviation_is_not_recognized() {
    // C `cmdcmp(ptr, "rmdeath", 7)` requires the full seven-letter word.
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/rmdeat Baddie",
        3
    )
    .is_none());
}

// C `/rename <from> <to>` (`command.c:6517-6524` dispatch -> `cmd_rename`,
// `command.c:2657-2676`), `CF_GOD`-gated, full-word only.

#[test]
pub(crate) fn rename_command_requires_god() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::STAFF);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/rename Baddie Newname",
        3
    )
    .is_none());
    assert!(world.drain_pending_rename_lookups().is_empty());
}

#[test]
pub(crate) fn rename_command_queues_both_names() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/rename Baddie newname",
        3,
    )
    .expect("god rename command should be recognized");
    assert!(result.messages.is_empty());
    let queued = world.drain_pending_rename_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].requester_id, caller_id);
    assert_eq!(queued[0].from_name, "Baddie");
    assert_eq!(queued[0].to_name, "Newname");
}

#[test]
pub(crate) fn rename_command_with_an_illegal_to_name_is_rejected_immediately() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/rename Baddie ab", 3)
            .expect("god rename command should be recognized");
    assert!(result.messages.is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].character_id, caller_id);
    assert_eq!(texts[0].message, "Name too long or too short.");
    assert!(world.drain_pending_rename_lookups().is_empty());
}

#[test]
pub(crate) fn rename_command_abbreviation_is_not_recognized() {
    // C `cmdcmp(ptr, "rename", 6)` requires the full six-letter word.
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/renam Baddie Newname",
        3
    )
    .is_none());
}

// C `/lockname <name>`/`/unlockname <name>` (`command.c:6528-6543`
// dispatch -> `cmd_lockname`/`cmd_unlockname`, `command.c:2679-2701`),
// both `CF_GOD`-gated, full-word only.

#[test]
pub(crate) fn lockname_command_requires_god() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::STAFF);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/lockname Baddie",
        3
    )
    .is_none());
    assert!(world.drain_pending_lockname_lookups().is_empty());
}

#[test]
pub(crate) fn lockname_command_queues_the_lowercased_name() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/lockname Baddie", 3)
            .expect("god lockname command should be recognized");
    assert!(result.messages.is_empty());
    let queued = world.drain_pending_lockname_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].requester_id, caller_id);
    assert_eq!(queued[0].original_name, "Baddie");
    assert_eq!(queued[0].lookup_name, "baddie");
}

#[test]
pub(crate) fn unlockname_command_queues_the_lowercased_name() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/unlockname Baddie", 3)
            .expect("god unlockname command should be recognized");
    assert!(result.messages.is_empty());
    let queued = world.drain_pending_unlockname_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].original_name, "Baddie");
    assert_eq!(queued[0].lookup_name, "baddie");
}

#[test]
pub(crate) fn lockname_command_with_a_too_short_name_is_rejected_immediately() {
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    let result =
        apply_admin_character_command(&mut world, &mut runtime, caller_id, "/lockname ab", 3)
            .expect("god lockname command should be recognized");
    assert!(result.messages.is_empty());
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].character_id, caller_id);
    assert_eq!(texts[0].message, "Name too long or too short.");
    assert!(world.drain_pending_lockname_lookups().is_empty());
}

#[test]
pub(crate) fn lockname_command_abbreviation_is_not_recognized() {
    // C `cmdcmp(ptr, "lockname", 8)` requires the full eight-letter word.
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/lockna Baddie",
        3
    )
    .is_none());
}

#[test]
pub(crate) fn unlockname_command_abbreviation_is_not_recognized() {
    // C `cmdcmp(ptr, "unlockname", 10)` requires the full ten-letter word.
    let mut world = goto_test_world();
    let caller_id = CharacterId(1);
    let mut caller = login_character(caller_id, &login_block("Ralph"), 3, 10, 10);
    caller.flags.insert(CharacterFlags::GOD);
    assert!(world.spawn_character(caller, 10, 10));
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        caller_id,
        "/unlockna Baddie",
        3
    )
    .is_none());
}
