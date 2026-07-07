use super::*;

// C `command.c:9058-9066`/`3194-3218` (`/fixit`) and `command.c:9067-
// 9075`/`3221-3251` (`/questfix`).

#[test]
pub(crate) fn fixit_and_questfix_are_god_only() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    for command in ["/fixit Target", "/questfix Target"] {
        assert!(
            apply_admin_character_command(&mut world, &mut runtime, target_id, command, 1)
                .is_none(),
            "{command} should be GOD-gated"
        );
    }
}

#[test]
pub(crate) fn fixit_reports_no_one_by_that_name_when_target_is_offline() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let missing =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/fixit Nobody", 1)
            .expect("god fixit should be recognized");
    assert_eq!(
        missing.messages,
        vec!["Sorry, no one by the name Nobody around."]
    );
}

#[test]
pub(crate) fn fixit_wipes_and_reinitializes_the_targets_own_quest_log_with_no_confirmation() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    {
        let target_player = runtime.player_for_character_mut(target_id).unwrap();
        // Simulate a corrupted/stale quest log: already "initialized"
        // (sentinel set) but with a bogus entry that a fresh derive
        // would never produce.
        target_player.quest_log.mark_init_complete();
        target_player.quest_log.set_raw(0, 63, 3);
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/fixit Target", 1)
            .expect("god fixit should be recognized");
    // C sends no confirmation message to the caller at all.
    assert!(result.messages.is_empty());

    let target_player = runtime.player_for_character(target_id).unwrap();
    // The bogus entry is gone (wiped, then re-derived from scratch) and
    // the log is freshly marked complete again (re-init actually ran).
    assert_ne!(target_player.quest_log.entries()[0].done, 63);
    assert!(target_player.quest_log.is_init_complete());
}

#[test]
pub(crate) fn questfix_reports_no_one_by_that_name_when_target_is_offline() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let missing =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/questfix Nobody", 1)
            .expect("god questfix should be recognized");
    assert_eq!(
        missing.messages,
        vec!["Sorry, no one by the name Nobody around."]
    );
}

#[test]
pub(crate) fn questfix_clears_the_callers_own_sentinel_and_leaves_the_named_targets_log_untouched()
{
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    // Give the calling GOD their own connected PlayerRuntime too (C's
    // real bug operates on `cn`, the caller, not the named target `co`).
    let mut god_player = PlayerRuntime::connected(90, 0);
    god_player.character_id = Some(god_id);
    god_player.quest_log.mark_init_complete();
    runtime.players.insert(90, god_player);
    {
        let target_player = runtime.player_for_character_mut(target_id).unwrap();
        target_player.quest_log.mark_init_complete();
        target_player.quest_log.set_raw(0, 5, 3);
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/questfix Target", 1)
            .expect("god questfix should be recognized");
    assert!(result.messages.is_empty());

    // The caller's own sentinel was cleared (marked for full re-derive on
    // next login) even though the command targeted "Target".
    assert!(!runtime
        .player_for_character(god_id)
        .unwrap()
        .quest_log
        .is_init_complete());
    // The named target's quest log is completely untouched - C's bug
    // means `questlog_init(co)` is a no-op since `co`'s sentinel was
    // already set.
    let target_player = runtime.player_for_character(target_id).unwrap();
    assert!(target_player.quest_log.is_init_complete());
    assert_eq!(target_player.quest_log.entries()[0].done, 5);
}

// C `/clearppd <ppdname> [player]` (`command.c:10144-10146` dispatch,
// `CF_GOD | CF_STAFF`-gated; `cmd_clearppd`, `command.c:4214-4288`).

#[test]
pub(crate) fn clearppd_requires_god_or_staff() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        target_id,
        "/clearppd keyring",
        1
    )
    .is_none());
}

/// Registers a connected `PlayerRuntime` for `character_id` on a fresh
/// session, so self-target `/clearppd` calls (whose caller is also the
/// target) have somewhere to read/write PPD fields.
pub(crate) fn insert_runtime_for(
    runtime: &mut ServerRuntime,
    session_id: u64,
    character_id: CharacterId,
) {
    let mut player = PlayerRuntime::connected(session_id, 0);
    player.character_id = Some(character_id);
    runtime.players.insert(session_id, player);
}

#[test]
pub(crate) fn clearppd_staff_without_god_is_accepted() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    insert_runtime_for(&mut runtime, 90, god_id);
    // Demote the caller to STAFF-only, matching C's `CF_GOD | CF_STAFF`
    // gate accepting either flag.
    {
        let god = world.characters.get_mut(&god_id).unwrap();
        god.flags.remove(CharacterFlags::GOD);
        god.flags.insert(CharacterFlags::STAFF);
    }
    let _ = target_id;

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/clearppd keyring", 1)
            .expect("STAFF-only caller should still be recognized");
    assert_eq!(result.messages, vec!["No keyring PPD found for Godmode."]);
}

#[test]
pub(crate) fn clearppd_with_no_arguments_shows_usage() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/clearppd", 1)
        .expect("god clearppd should be recognized");
    assert_eq!(
        result.messages,
        vec![
            "Usage: #clearppd <ppdname> [player]",
            "Available PPDs: keyring, questlog, alias"
        ]
    );
}

#[test]
pub(crate) fn clearppd_rejects_unknown_ppd_name() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/clearppd bogus", 1)
            .expect("god clearppd should be recognized");
    assert_eq!(
        result.messages,
        vec![
            "Unknown PPD: bogus",
            "Available PPDs: keyring, questlog, alias"
        ]
    );
}

#[test]
pub(crate) fn clearppd_reports_player_not_found_with_its_own_distinct_message() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/clearppd keyring Nobody",
        1,
    )
    .expect("god clearppd should be recognized");
    // Deliberately NOT "Sorry, no one by the name %s around." - C's
    // `cmd_clearppd` uses its own distinct wording.
    assert_eq!(result.messages, vec!["Player 'Nobody' not found."]);
}

#[test]
pub(crate) fn clearppd_keyring_reports_not_found_when_already_empty_and_clears_when_populated() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    insert_runtime_for(&mut runtime, 90, god_id);

    // Empty keyring (default) -> "No ... PPD found".
    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/clearppd keyring", 1)
            .expect("god clearppd should be recognized");
    assert_eq!(result.messages, vec!["No keyring PPD found for Godmode."]);

    // Populate it, then clear for real.
    {
        let god_player = runtime.player_for_character_mut(god_id).unwrap();
        god_player.keyring.push(ugaris_core::player::KeyringEntry {
            template_id: 1,
            name: "Test Key".to_string(),
            description: String::new(),
            sprite: 0,
            flags: 0,
            value: 0,
            driver: 0,
            driver_data: Vec::new(),
            expire_serial: 0,
        });
    }
    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/clearppd keyring", 1)
            .expect("god clearppd should be recognized");
    assert_eq!(result.messages, vec!["Cleared keyring PPD for Godmode."]);
    assert!(runtime
        .player_for_character(god_id)
        .unwrap()
        .keyring
        .is_empty());
}

#[test]
pub(crate) fn clearppd_targets_a_named_player_and_notifies_both_sides() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    {
        let target_player = runtime.player_for_character_mut(target_id).unwrap();
        target_player
            .aliases
            .push(ugaris_core::player::CommandAlias {
                from: "gg".to_string(),
                to: "grin".to_string(),
            });
    }

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/clearppd alias Target",
        1,
    )
    .expect("god clearppd should be recognized");
    assert_eq!(result.messages, vec!["Cleared alias PPD for Target."]);
    assert_eq!(
        result.other_messages,
        vec![(
            target_id,
            "Your alias data has been cleared by Godmode.".to_string()
        )]
    );
    assert!(runtime
        .player_for_character(target_id)
        .unwrap()
        .aliases
        .is_empty());
}

#[test]
pub(crate) fn clearppd_self_target_sends_no_other_message() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    insert_runtime_for(&mut runtime, 90, god_id);

    {
        let god_player = runtime.player_for_character_mut(god_id).unwrap();
        god_player.aliases.push(ugaris_core::player::CommandAlias {
            from: "gg".to_string(),
            to: "grin".to_string(),
        });
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/clearppd alias", 1)
            .expect("god clearppd should be recognized");
    assert_eq!(result.messages, vec!["Cleared alias PPD for Godmode."]);
    assert!(result.other_messages.is_empty());
}

#[test]
pub(crate) fn clearppd_questlog_clears_and_reports_success() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    insert_runtime_for(&mut runtime, 90, god_id);

    {
        let god_player = runtime.player_for_character_mut(god_id).unwrap();
        god_player.quest_log.set_raw(0, 1, 1);
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/clearppd questlog", 1)
            .expect("god clearppd should be recognized");
    assert_eq!(result.messages, vec!["Cleared questlog PPD for Godmode."]);
    assert!(runtime
        .player_for_character(god_id)
        .unwrap()
        .quest_log
        .is_empty());
}

#[test]
pub(crate) fn clearppd_only_matches_online_player_flagged_characters() {
    // A non-CF_PLAYER character sharing the target name must not match
    // (C's search loop skips any `co` without `CF_PLAYER`).
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    {
        let target = world.characters.get_mut(&target_id).unwrap();
        target.flags.remove(CharacterFlags::PLAYER);
    }

    let result = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/clearppd keyring Target",
        1,
    )
    .expect("god clearppd should be recognized");
    assert_eq!(result.messages, vec!["Player 'Target' not found."]);
}
