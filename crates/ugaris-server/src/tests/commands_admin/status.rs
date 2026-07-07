use super::*;

#[test]
pub(crate) fn status_command_shows_represented_lostcon_and_account_state() {
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    character
        .flags
        .insert(CharacterFlags::PAID | CharacterFlags::NOBLESS);
    character.values[1][CharacterValue::Bless as usize] = 10;
    character.values[1][CharacterValue::Pulse as usize] = 8;
    character.values[1][CharacterValue::Fireball as usize] = 5;
    let mut player = PlayerRuntime::connected(1, 0);
    player.set_max_lag_seconds(12);
    player.autoturn_enabled = true;

    let result = apply_status_command(&character, &player, "/status")
        .expect("status command should be recognized");

    assert_eq!(result.messages[0], "Lag Control Settings:");
    assert!(result
        .messages
        .contains(&"Max. Lag [/MAXLAG]: 12 sec.".to_string()));
    assert!(result
        .messages
        .contains(&"Don't use Bless [/NOBLESS]: Off.".to_string()));
    assert!(result
        .messages
        .contains(&"Don't use Fireball [/NOFIREBALL]: Off.".to_string()));
    assert!(result
        .messages
        .contains(&"Automatic Pulse [/AUTOPULSE]: Off.".to_string()));
    assert!(result
        .messages
        .contains(&"Automatic Turning [/AUTOTURN]: On.".to_string()));
    assert!(result
        .messages
        .contains(&"Allow others to bless me [/ALLOWBLESS]: No.".to_string()));
    assert!(result.messages.contains(&"Account Status:".to_string()));
    assert!(result.messages.contains(&"Paid Account".to_string()));
}

#[test]
pub(crate) fn status_command_reflects_enabled_lag_control_toggles() {
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    character.values[1][CharacterValue::Bless as usize] = 10;
    let mut player = PlayerRuntime::connected(1, 0);
    player.no_bless = true;
    player.no_life = true;
    player.no_move = true;
    player.autobless_enabled = true;

    let result = apply_status_command(&character, &player, "/status")
        .expect("status command should be recognized");

    assert!(result
        .messages
        .contains(&"Don't use Bless [/NOBLESS]: On.".to_string()));
    assert!(result
        .messages
        .contains(&"Don't use Healing Potions [/NOLIFE]: On.".to_string()));
    assert!(result
        .messages
        .contains(&"Don't Move [/NOMOVE]: On.".to_string()));
    assert!(result
        .messages
        .contains(&"Automatic Re-Bless [/AUTOBLESS]: On.".to_string()));
}

#[test]
pub(crate) fn status_command_preserves_cmdcmp_prefix_shape() {
    let character = login_character(CharacterId(7), &login_block("Tester"), 1, 10, 10);
    let player = PlayerRuntime::connected(1, 0);

    assert!(apply_status_command(&character, &player, "/s").is_some());
    assert!(apply_status_command(&character, &player, "/stat").is_some());
    assert!(apply_status_command(&character, &player, "/statusx").is_none());
}

#[test]
pub(crate) fn setpentcount_setpentstatus_setpentbonus_mutate_the_named_targets_data() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let count = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setpentcount Target 3",
        1,
    )
    .expect("god setpentcount should be recognized");
    assert_eq!(count.messages, vec!["Set pent_cnt for Target: 0 -> 3"]);
    assert_eq!(
        runtime
            .player_for_character(target_id)
            .unwrap()
            .pentagram_debug
            .pent_cnt,
        3
    );

    let status = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setpentstatus Target 1",
        1,
    )
    .expect("god setpentstatus should be recognized");
    assert_eq!(status.messages, vec!["Set pent status for Target: 0 -> 1"]);
    assert_eq!(
        runtime
            .player_for_character(target_id)
            .unwrap()
            .pentagram_debug
            .status,
        1
    );

    let bonus = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setpentbonus Target -50",
        1,
    )
    .expect("god setpentbonus should be recognized");
    assert_eq!(bonus.messages, vec!["Set pent bonus for Target: 0 -> -50"]);
    assert_eq!(
        runtime
            .player_for_character(target_id)
            .unwrap()
            .pentagram_debug
            .bonus,
        -50
    );
}

#[test]
pub(crate) fn macrolist_formats_every_status_and_sorts_by_character_id() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);

    let names = ["Zed", "Amy", "Mel", "Ida"];
    let mut ids = Vec::new();
    for (offset, name) in names.iter().enumerate() {
        let id = CharacterId(20 + offset as u32);
        world.add_character(login_character(id, &login_block(name), 1, 11, 10));
        let mut player = PlayerRuntime::connected(100 + offset as u64, 0);
        player.character_id = Some(id);
        runtime.players.insert(100 + offset as u64, player);
        ids.push(id);
    }
    // ids[0]="Zed": default OK status.
    // ids[1]="Amy": in challenge room -> CHALLENGED (highest priority).
    // ids[2]="Mel": immune -> IMMUNE.
    // ids[3]="Ida": suspicion >= 50 -> SUSPICIOUS.
    runtime
        .player_for_character_mut(ids[1])
        .unwrap()
        .macro_ppd
        .in_challenge_room = true;
    {
        let ppd = &mut runtime.player_for_character_mut(ids[2]).unwrap().macro_ppd;
        ppd.immune_until = i64::MAX;
    }
    runtime
        .player_for_character_mut(ids[3])
        .unwrap()
        .macro_ppd
        .suspicion = 50;

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "/macrolist", 1)
        .expect("god macrolist should be recognized");
    assert_eq!(
        result.messages,
        vec![
            "=== Online Players - Macro Status ===".to_string(),
            "Name                 Karma  Susp  Pass/Fail  Status".to_string(),
            "---------------------------------------------------".to_string(),
            "Zed                      0     0     0/0     OK".to_string(),
            "Amy                      0     0     0/0     CHALLENGED".to_string(),
            "Mel                      0     0     0/0     IMMUNE".to_string(),
            "Ida                      0    50     0/0     SUSPICIOUS".to_string(),
            "---------------------------------------------------".to_string(),
            "Total: 4 players".to_string(),
        ]
    );
}

#[test]
pub(crate) fn acstatus_is_god_or_staff_only() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (_god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        target_id,
        "#acstatus Target",
        1
    )
    .is_none());
}

#[test]
pub(crate) fn acstatus_without_a_name_shows_usage() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result = apply_admin_character_command(&mut world, &mut runtime, god_id, "#acstatus", 1)
        .expect("god acstatus should be recognized");
    assert_eq!(result.messages, vec!["Usage: #acstatus <player>"]);
}

#[test]
pub(crate) fn acstatus_reports_not_found_online_for_an_unknown_name() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acstatus Nobody", 1)
            .expect("god acstatus should be recognized");
    assert_eq!(result.messages, vec!["Player 'Nobody' not found online."]);
}

#[test]
pub(crate) fn acstatus_reports_no_connection_data_when_target_has_no_anticheat_session() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, _target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    // `setup_god_and_online_target` registers the target's `PlayerRuntime`
    // with `anticheat_session_id: None` (the default) - matching C's
    // `!nr || !player[nr]` branch, just for a different underlying reason
    // (no anti-cheat session ever got created rather than no connection
    // at all).

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acstatus Target", 1)
            .expect("god acstatus should be recognized");
    assert_eq!(
        result.messages,
        vec!["Player 'Target' has no connection data."]
    );
    assert!(world.drain_pending_ac_status_lookups().is_empty());
}

#[test]
pub(crate) fn acstatus_queues_a_lookup_using_the_targets_anticheat_session_id() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let (god_id, target_id) = setup_god_and_online_target(&mut world, &mut runtime);
    for player in runtime.players.values_mut() {
        if player.character_id == Some(target_id) {
            player.anticheat_session_id = Some(1234);
        }
    }

    let result =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "#acstatus target", 1)
            .expect("god acstatus should be recognized");
    assert_eq!(result.messages, Vec::<String>::new());

    let queued = world.drain_pending_ac_status_lookups();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].caller_id, god_id);
    assert_eq!(queued[0].target_name, "Target");
    assert_eq!(queued[0].session_id, 1234);
}
