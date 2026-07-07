use super::*;

#[test]
pub(crate) fn god_tick_tuning_commands_match_legacy_ranges_and_feedback() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();
    let ticks = TICKS_PER_SECOND as i32;

    let decay = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setdecaytime 1440tail",
        1,
    )
    .expect("god setdecaytime should be recognized");
    assert_eq!(runtime.item_decay_time, 1440);
    assert_eq!(
        decay.messages,
        vec!["Item decay time changed from 7200 to 1440 ticks (5 to 1 minutes)"]
    );

    let player_body = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setplayerbodytime 7200",
        1,
    )
    .expect("god setplayerbodytime should be recognized");
    assert_eq!(runtime.player_body_decay_time, 7200);
    assert_eq!(
        player_body.messages,
        vec!["Player body decay time changed from 43200 to 7200 ticks (30 to 5 minutes)"]
    );

    let npc_body =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setnpcbodytime 720", 1)
            .expect("god setnpcbodytime should be recognized");
    assert_eq!(runtime.npc_body_decay_time, 720);
    assert_eq!(
        npc_body.messages,
        vec!["NPC body decay time changed from 2880 to 720 ticks (2 to 0 minutes)"]
    );

    let area32_body = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setnpcbodytimearea32 7200",
        1,
    )
    .expect("god setnpcbodytimearea32 should be recognized");
    assert_eq!(runtime.npc_body_decay_time_area32, 7200);
    assert_eq!(
        area32_body.messages,
        vec!["NPC body decay time for area 32 changed from 21600 to 7200 ticks (15 to 5 minutes)"]
    );

    let respawn =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setrespawntime 720", 1)
            .expect("god setrespawntime should be recognized");
    assert_eq!(runtime.npc_respawn_timer, 720);
    assert_eq!(
        respawn.messages,
        vec!["NPC respawn time changed from 2880 to 720 ticks (2 to 0 minutes)"]
    );

    let sewer_respawn = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setsewerrespawntime 3600tail",
        1,
    )
    .expect("god setsewerrespawntime should be recognized");
    assert_eq!(runtime.sewer_item_respawn_time, 3600);
    assert_eq!(
        sewer_respawn.messages,
        vec!["Sewer item respawn time changed from 86400 to 3600 seconds (24 to 1 hours)"]
    );

    let lagout =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setlagouttime 1440", 1)
            .expect("god setlagouttime should be recognized");
    assert_eq!(runtime.lagout_time, 1440);
    assert_eq!(
        lagout.messages,
        vec!["Lagout time changed from 7200 to 1440 ticks (5 to 1 minutes)"]
    );

    let regen =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setregentime 12", 1)
            .expect("god setregentime should be recognized");
    assert_eq!(runtime.regen_time, 12);
    assert_eq!(
        regen.messages,
        vec!["Regeneration time changed from 96 to 12 ticks"]
    );

    let invalid =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setrespawntime 719", 1)
            .expect("invalid setrespawntime should still be handled");
    assert_eq!(runtime.npc_respawn_timer, 720);
    assert_eq!(
        invalid.messages,
        vec![format!(
            "Invalid value. Please specify a time between {} and {} ticks (0.5-10 minutes)",
            30 * ticks,
            10 * 60 * ticks
        )]
    );

    let invalid_sewer = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setsewerrespawntime 3599",
        1,
    )
    .expect("invalid setsewerrespawntime should still be handled");
    assert_eq!(runtime.sewer_item_respawn_time, 3600);
    assert_eq!(
        invalid_sewer.messages,
        vec!["Invalid value. Please specify a time between 3600 and 604800 seconds (1 hour to 7 days)"]
    );
}

#[test]
pub(crate) fn tick_tuning_commands_are_god_only_and_preserve_minimum_lengths() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    world.add_character(login_character(
        character_id,
        &login_block("Tester"),
        1,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setdecaytime 1440",
        1,
    )
    .is_none());
    assert_eq!(
        runtime.item_decay_time,
        GameSettings::default().item_decay_time
    );
    assert_eq!(
        runtime.sewer_item_respawn_time,
        GameSettings::default().sewer_item_respawn_time
    );

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
        "/setdecaytim 1440",
        1,
    )
    .is_none());
    assert_eq!(
        runtime.item_decay_time,
        GameSettings::default().item_decay_time
    );

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setnpcbodytim 720",
        1,
    )
    .is_none());
    assert_eq!(
        runtime.npc_body_decay_time,
        GameSettings::default().npc_body_decay_time
    );

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setsewerrespawntim 3600",
        1,
    )
    .is_none());
    assert_eq!(
        runtime.sewer_item_respawn_time,
        GameSettings::default().sewer_item_respawn_time
    );
}

#[test]
pub(crate) fn god_communication_tuning_commands_match_legacy_ranges_and_feedback() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    let holler =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/sethollerdist 75tail", 1)
            .expect("god sethollerdist should be recognized");
    assert_eq!(runtime.holler_dist, 75);
    assert_eq!(
        holler.messages,
        vec!["Holler distance changed from 75 to 75 tiles"]
    );

    let quiet =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setquietsaydist 4", 1)
            .expect("god setquietsaydist should be recognized");
    assert_eq!(runtime.quietsay_dist, 4);
    assert_eq!(
        quiet.messages,
        vec!["Quiet say distance changed from 8 to 4 tiles"]
    );

    let shout_cost =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setshoutcost 2000", 1)
            .expect("god setshoutcost should be recognized");
    assert_eq!(runtime.shout_cost, 2000);
    assert_eq!(
        shout_cost.messages,
        vec!["Shout cost changed from 6 to 2 endurance points"]
    );

    let invalid_whisper =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setwhisperdist 0", 1)
            .expect("invalid setwhisperdist should still be handled");
    assert_eq!(runtime.whisper_dist, GameSettings::default().whisper_dist);
    assert_eq!(
        invalid_whisper.messages,
        vec!["Invalid value. Please specify a distance between 1 and 12 tiles"]
    );
}

#[test]
pub(crate) fn communication_tuning_commands_are_god_only_and_preserve_minimum_lengths() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    world.add_character(login_character(
        character_id,
        &login_block("Tester"),
        1,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();

    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setshoutdist 50",
        1,
    )
    .is_none());
    assert_eq!(runtime.shout_dist, GameSettings::default().shout_dist);

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
        "/setshoutdis 50",
        1,
    )
    .is_none());
    assert_eq!(runtime.shout_dist, GameSettings::default().shout_dist);
}

#[test]
pub(crate) fn god_game_settings_int_tuning_commands_match_legacy_ranges_and_feedback() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    let lots =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setsplots 8000", 1)
            .expect("god setsplots should be recognized");
    assert_eq!(world.settings.sp_lots, 8000);
    assert_eq!(
        lots.messages,
        vec!["Special item probability 'lots' category changed from 5000 to 8000"]
    );

    let invalid_lots =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setsplots 999", 1)
            .expect("invalid setsplots should still be handled");
    assert_eq!(world.settings.sp_lots, 8000);
    assert_eq!(
        invalid_lots.messages,
        vec!["Invalid value. Please specify a value between 1000 and 10000"]
    );

    let dungeon =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setdungeontime 43200", 1)
            .expect("god setdungeontime should be recognized");
    assert_eq!(world.settings.dungeon_time, 43200);
    assert_eq!(
        dungeon.messages,
        vec!["Dungeon time limit changed from 86400 to 43200 ticks (60 to 30 minutes)"]
    );

    let invalid_dungeon =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setdungeontime 100", 1)
            .expect("invalid setdungeontime should still be handled");
    assert_eq!(world.settings.dungeon_time, 43200);
    assert_eq!(
        invalid_dungeon.messages,
        vec![
            "Invalid value. Please specify a time between 43200 and 172800 ticks (30-120 minutes)"
        ]
    );

    let jewel =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setmaxjewelcount 5", 1)
            .expect("god setmaxjewelcount should be recognized");
    assert_eq!(world.settings.max_jewel_count, 5);
    assert_eq!(
        jewel.messages,
        vec!["Maximum jewel count changed from 2 to 5"]
    );

    let drop_low =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setdropproblow 500", 1)
            .expect("god setdropproblow should be recognized");
    assert_eq!(world.settings.drop_prob_low_level, 500);
    assert_eq!(
        drop_low.messages,
        vec!["Drop probability (low level) changed from 1700 to 500"]
    );
}

#[test]
pub(crate) fn god_game_settings_float_tuning_commands_match_legacy_ranges_and_feedback() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    let divider = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/settunnelexpdivider 4.5",
        1,
    )
    .expect("god settunnelexpdivider should be recognized");
    assert_eq!(world.settings.tunnel_exp_base_value_divider, 4.5);
    assert_eq!(
        divider.messages,
        vec!["Tunnel experience base value divider changed from 5.00 to 4.50"]
    );

    let invalid_divider = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/settunnelexpdivider 0.5",
        1,
    )
    .expect("invalid settunnelexpdivider should still be handled");
    assert_eq!(world.settings.tunnel_exp_base_value_divider, 4.5);
    assert_eq!(
        invalid_divider.messages,
        vec!["Invalid value. Please specify a value between 1.0 and 10.0"]
    );

    let solve =
        apply_admin_character_command(&mut world, &mut runtime, god_id, "/setexpsolve 1.5", 1)
            .expect("god setexpsolve should be recognized");
    assert_eq!(world.settings.exp_solve_multiplier, 1.5);
    assert_eq!(
        solve.messages,
        vec!["Experience solve multiplier changed from 0.66 to 1.50"]
    );

    let reflection = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setclanreflection 0.5",
        1,
    )
    .expect("god setclanreflection should be recognized");
    assert_eq!(world.settings.exp_clan_reflection_multiplier, 0.5);
    assert_eq!(
        reflection.messages,
        vec!["Clan reflection multiplier changed from 0.70 to 0.50"]
    );

    let rare_mult = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setraredropmultiplier 2.0",
        1,
    )
    .expect("god setraredropmultiplier should be recognized");
    assert_eq!(world.settings.rare_drop_multiplier, 2.0);
    assert_eq!(
        rare_mult.messages,
        vec!["Rare drop multiplier changed from 1.20 to 2.00"]
    );

    let invalid_rare_mult = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setraredropmultiplier 5.0",
        1,
    )
    .expect("invalid setraredropmultiplier should still be handled");
    assert_eq!(world.settings.rare_drop_multiplier, 2.0);
    assert_eq!(
        invalid_rare_mult.messages,
        vec!["Invalid value. Please specify a value between 1.0 and 3.0"]
    );
}

#[test]
pub(crate) fn god_setjaillocation_and_setastonlocation_update_settings_like_c() {
    let mut world = World::default();
    let god_id = CharacterId(7);
    let mut god = login_character(god_id, &login_block("Godmode"), 1, 10, 10);
    god.flags.insert(CharacterFlags::GOD);
    world.add_character(god);
    let mut runtime = ServerRuntime::default();

    let jail = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setjaillocation 100 200 5",
        1,
    )
    .expect("god setjaillocation should be recognized");
    assert_eq!(
        (
            world.settings.jail_x,
            world.settings.jail_y,
            world.settings.jail_area
        ),
        (100, 200, 5)
    );
    assert_eq!(
        jail.messages,
        vec!["Jail location changed from 186,234 (area 3) to 100,200 (area 5)"]
    );

    let invalid_jail = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setjaillocation 0 200 5",
        1,
    )
    .expect("invalid setjaillocation should still be handled");
    assert_eq!(
        (
            world.settings.jail_x,
            world.settings.jail_y,
            world.settings.jail_area
        ),
        (100, 200, 5)
    );
    assert_eq!(
        invalid_jail.messages,
        vec!["Invalid coordinates or area. Format: /setjaillocation x y area"]
    );

    let aston = apply_admin_character_command(
        &mut world,
        &mut runtime,
        god_id,
        "/setastonlocation 50 60 7",
        1,
    )
    .expect("god setastonlocation should be recognized");
    assert_eq!(
        (
            world.settings.aston_x,
            world.settings.aston_y,
            world.settings.aston_area
        ),
        (50, 60, 7)
    );
    assert_eq!(
        aston.messages,
        vec!["Aston location changed from 133,203 (area 3) to 50,60 (area 7)"]
    );
}

#[test]
pub(crate) fn game_settings_tuning_commands_are_god_only_and_resolve_ambiguous_abbreviations_by_source_order(
) {
    let mut world = World::default();
    let character_id = CharacterId(7);
    world.add_character(login_character(
        character_id,
        &login_block("Tester"),
        1,
        10,
        10,
    ));
    let mut runtime = ServerRuntime::default();

    // Non-god caller: recognized-but-gated commands must return `None`.
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setsplots 8000",
        1,
    )
    .is_none());
    assert_eq!(world.settings.sp_lots, GameSettings::default().sp_lots);

    world
        .characters
        .get_mut(&character_id)
        .unwrap()
        .flags
        .insert(CharacterFlags::GOD);

    // `setpentmaxpower`'s C `cmdcmp` `minlen` is 15 - equal to the full
    // command's own length, i.e. no abbreviation is accepted at all.
    // Dropping the trailing "r" (14 characters) must not match anything
    // (C `cmdcmp` returns 0 when the matched length is short of `minlen`).
    assert!(apply_admin_character_command(
        &mut world,
        &mut runtime,
        character_id,
        "/setpentmaxpowe 5000",
        1,
    )
    .is_none());
    assert_eq!(
        world.settings.max_power_level,
        GameSettings::default().max_power_level
    );

    // "setmax" (6 chars) is a valid abbreviation-length prefix of
    // `setmaxjewelcount` (minlen 16, too short here), `setmaxsilvergolemtype`
    // (minlen 6, matches) and `setmaxclanbonus` (minlen 6, matches) - C's
    // first-declared-wins `if` chain resolves this to
    // `setmaxsilvergolemtype` (`command.c:7610`, declared before
    // `setmaxclanbonus` at `command.c:8008`), so the Rust port must too.
    let result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/setmax 15", 1)
            .expect("ambiguous setmax abbreviation should still resolve to a command");
    assert_eq!(world.settings.max_silver_golem_type, 15);
    assert_eq!(
        world.settings.max_clan_bonus_percent,
        GameSettings::default().max_clan_bonus_percent
    );
    assert_eq!(
        result.messages,
        vec!["Max silver golem type changed from 8 to 15"]
    );

    // Likewise "setpent" (7 chars) is too short for `setpentvismaxpents`
    // (minlen 18) and `setpentmaxpower` (minlen 15) but long enough for
    // `setpentvaluemultiplier` (minlen 6, declared next at
    // `command.c:7829`), which therefore wins.
    let pent_result =
        apply_admin_character_command(&mut world, &mut runtime, character_id, "/setpent 99", 1)
            .expect("ambiguous setpent abbreviation should still resolve to a command");
    assert_eq!(world.settings.pentagram_value_multiplier, 99);
    assert_eq!(
        world.settings.max_visible_pents,
        GameSettings::default().max_visible_pents
    );
    assert_eq!(
        pent_result.messages,
        vec!["Pentagram value multiplier changed from 50 to 99"]
    );
}
