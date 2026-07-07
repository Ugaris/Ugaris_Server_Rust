use super::*;

#[test]
pub(crate) fn laugh_command_is_god_only_and_queues_legacy_sound() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character.clone());

    assert!(apply_laugh_command(&mut world, character_id, "/laugh").is_none());
    assert!(world.drain_pending_sound_specials().is_empty());

    character.flags.insert(CharacterFlags::GOD);
    world.characters.insert(character_id, character);
    let result = apply_laugh_command(&mut world, character_id, "/laugh")
        .expect("god laugh command should be recognized");

    assert!(result.messages.is_empty());
    let sounds = world.drain_pending_sound_specials();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].character_id, character_id);
    assert_eq!(sounds[0].special.special_type, 13);
}

#[test]
pub(crate) fn saves_command_is_god_only_and_uses_legacy_prefix_parsing() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.saves = 3;
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/saves 12", 1)
            .is_none()
    );
    assert_eq!(world.characters.get(&character_id).unwrap().saves, 3);

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);
    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/save 12abc", 1)
            .expect("god saves command should be recognized by minlen 4 abbreviation");

    assert!(result.messages.is_empty());
    assert!(!result.inventory_changed);
    assert_eq!(world.characters.get(&character_id).unwrap().saves, 12);

    apply_admin_character_command(&mut world, &mut runtime, character_id, "/saves nope", 1)
        .expect("god saves command should be recognized");
    assert_eq!(world.characters.get(&character_id).unwrap().saves, 0);
}

#[test]
pub(crate) fn saveall_command_is_god_only_and_disambiguated_from_saves() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    world.add_character(character);
    let mut runtime = ServerRuntime::default();

    assert!(
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/saveall", 1)
            .is_none()
    );

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);

    // Exactly "/save" (minlen 4) is claimed by the `saves` stat setter,
    // matching C's `cmdcmp(ptr, "saves", 4)` appearing before `cmdcmp(ptr,
    // "saveall", 4)` in `command.c` - it must not trigger `save_all_requested`.
    let saves_result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/save 9", 1)
            .expect("god saves command should still win on exact minlen-4 abbreviation");
    assert!(!saves_result.save_all_requested);
    assert_eq!(world.characters.get(&character_id).unwrap().saves, 9);

    for command in ["/savea", "/saveal", "/saveall"] {
        let result =
            apply_admin_character_command(&mut world, &mut runtime, character_id, command, 1)
                .unwrap_or_else(|| panic!("{command} should be recognized as /saveall"));
        assert!(
            result.save_all_requested,
            "{command} should request save-all"
        );
        assert_eq!(
            result.messages,
            vec![
                "Forcing save of all players...".to_string(),
                "Player data saved".to_string(),
                "Forcing save of merchant inventories...".to_string(),
                "Merchant data saved".to_string(),
            ]
        );
    }
}

#[test]
pub(crate) fn backup_rotation_cursor_cycles_through_connected_players_deterministically() {
    let mut runtime = ServerRuntime::default();
    // No connected players yet.
    assert_eq!(runtime.next_backup_rotation_target(), None);

    let (commands, _rx) = mpsc::channel(16);
    runtime.connect(1, commands, 0);
    runtime.players.get_mut(&1).unwrap().character_id = Some(CharacterId(5));
    let (commands, _rx2) = mpsc::channel(16);
    runtime.connect(2, commands, 0);
    runtime.players.get_mut(&2).unwrap().character_id = Some(CharacterId(3));
    let (commands, _rx3) = mpsc::channel(16);
    runtime.connect(3, commands, 0);
    runtime.players.get_mut(&3).unwrap().character_id = None;

    // Sorted by CharacterId: 3, then 5. Session 3 has no character_id and
    // is skipped, matching C's `player[n] && cn` guard.
    assert_eq!(runtime.next_backup_rotation_target(), Some(CharacterId(3)));
    assert_eq!(runtime.next_backup_rotation_target(), Some(CharacterId(5)));
    // Cursor wraps back to the start once every connected player has been
    // visited once, matching C's `n = 1;` reset at the end of the scan.
    assert_eq!(runtime.next_backup_rotation_target(), Some(CharacterId(3)));
}
