use super::*;
use ugaris_core::character_driver::{
    ArenaFighterDriverData, ArenaMasterDriverData, CDR_ARENAFIGHTER, CDR_ARENAMASTER,
    CDR_LAMPGHOST, MS_FIGHT,
};
use ugaris_core::world::LegacyHurtOutcome;

#[test]
fn random_shrine_kindness_clears_pk_and_marks_ppd() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut character = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);
    character.flags.insert(CharacterFlags::PK);

    let result = apply_random_shrine_kindness(&mut player, &mut character, 40);

    assert_eq!(result, RandomShrineKindnessApplyResult::Used);
    assert!(!character.flags.contains(CharacterFlags::PK));
    assert!(character.flags.contains(CharacterFlags::UPDATE));
    assert!(player.has_used_random_shrine(40));
}

#[test]
fn pk_hate_command_adds_online_player_and_clears_lag() {
    let mut attacker = login_character(CharacterId(7), &login_block("Attacker"), 1, 10, 10);
    attacker
        .flags
        .insert(CharacterFlags::PK | CharacterFlags::LAG);
    attacker.level = 12;
    let mut target = login_character(CharacterId(8), &login_block("Target"), 1, 11, 10);
    target.flags.insert(CharacterFlags::PK);
    target.level = 10;
    let mut world = World::default();
    world.add_character(attacker);
    world.add_character(target);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(7));

    let result = apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/hate target", 0)
        .expect("hate command should be recognized");

    assert!(result.messages.is_empty());
    assert_eq!(result.name_refresh, vec![CharacterId(7), CharacterId(8)]);
    assert!(player.has_pk_hate_for(8));
    assert!(!world
        .characters
        .get(&CharacterId(7))
        .unwrap()
        .flags
        .contains(CharacterFlags::LAG));
}

#[test]
fn pk_hate_command_list_and_remove_match_legacy_feedback() {
    let mut attacker = login_character(CharacterId(7), &login_block("Attacker"), 1, 10, 10);
    attacker.flags.insert(CharacterFlags::PK);
    let mut target = login_character(CharacterId(8), &login_block("Target"), 1, 11, 10);
    target.flags.insert(CharacterFlags::PK);
    let mut world = World::default();
    world.add_character(attacker);
    world.add_character(target);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(7));
    assert!(player.add_pk_hate(8));

    let listed = apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/listhate", 0)
        .expect("listhate command should be recognized");
    let removed =
        apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/nohate target", 0)
            .expect("nohate command should be recognized");
    let empty = apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/listhate", 0)
        .expect("listhate command should be recognized");

    assert_eq!(listed.messages, vec!["Hate: Target"]);
    assert_eq!(removed.messages, vec!["Removed Target from hate list"]);
    assert_eq!(removed.name_refresh, vec![CharacterId(7), CharacterId(8)]);
    assert_eq!(empty.messages, vec!["List is empty."]);
    assert!(!player.has_pk_hate_for(8));
}

#[test]
fn pk_nohate_numeric_id_uses_legacy_del_hate_id_feedback() {
    let mut attacker = login_character(CharacterId(7), &login_block("Attacker"), 1, 10, 10);
    attacker.flags.insert(CharacterFlags::PK);
    let mut world = World::default();
    world.add_character(attacker);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(7));
    assert!(player.add_pk_hate(1234));

    let removed = apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/nohate 1234", 0)
        .expect("nohate command should be recognized");

    assert_eq!(removed.messages, vec!["Removed from hate list"]);
    assert_eq!(removed.name_refresh, vec![CharacterId(7)]);
    assert!(!player.has_pk_hate_for(1234));
}

#[test]
fn pk_hate_commands_accept_legacy_abbreviations() {
    let mut attacker = login_character(CharacterId(7), &login_block("Attacker"), 1, 10, 10);
    attacker
        .flags
        .insert(CharacterFlags::PK | CharacterFlags::LAG);
    attacker.level = 12;
    let mut target = login_character(CharacterId(8), &login_block("Target"), 1, 11, 10);
    target.flags.insert(CharacterFlags::PK);
    target.level = 10;
    let mut world = World::default();
    world.add_character(attacker);
    world.add_character(target);
    let mut player = PlayerRuntime::connected(1, 0);

    let added = apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/hat target", 0)
        .expect("abbreviated hate command should be recognized");
    let listed = apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/li", 0)
        .expect("abbreviated listhate command should be recognized");
    let removed = apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/noh target", 0)
        .expect("abbreviated nohate command should be recognized");

    assert!(added.messages.is_empty());
    assert_eq!(listed.messages, vec!["Hate: Target"]);
    assert_eq!(removed.messages, vec!["Removed Target from hate list"]);
    assert!(!player.has_pk_hate_for(8));
    assert!(
        apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/ha target", 0).is_none()
    );
}

#[test]
fn pk_clearhate_is_silent_and_only_mutates_pk_characters() {
    let mut pk_character = login_character(CharacterId(7), &login_block("Pk"), 1, 10, 10);
    pk_character.flags.insert(CharacterFlags::PK);
    let non_pk_character = login_character(CharacterId(8), &login_block("NonPk"), 1, 11, 10);
    let mut world = World::default();
    world.add_character(pk_character);
    world.add_character(non_pk_character);

    let mut player = PlayerRuntime::connected(1, 0);
    assert!(player.add_pk_hate(100));
    let pk_clear = apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/clearhate", 0)
        .expect("clearhate command should be recognized");

    assert!(pk_clear.messages.is_empty());
    assert!(player.pk_hate.is_empty());

    assert!(player.add_pk_hate(100));
    let non_pk_clear =
        apply_pk_hate_command(&mut world, &mut player, CharacterId(8), "/clearhate", 0)
            .expect("clearhate command should be recognized");

    assert!(non_pk_clear.messages.is_empty());
    assert!(player.has_pk_hate_for(100));
}

#[test]
fn pk_hate_command_uses_legacy_front_priority_on_duplicate_add() {
    let mut attacker = login_character(CharacterId(7), &login_block("Attacker"), 1, 10, 10);
    attacker.flags.insert(CharacterFlags::PK);
    attacker.level = 12;
    let mut first = login_character(CharacterId(8), &login_block("First"), 1, 11, 10);
    first.flags.insert(CharacterFlags::PK);
    first.level = 10;
    let mut second = login_character(CharacterId(9), &login_block("Second"), 1, 12, 10);
    second.flags.insert(CharacterFlags::PK);
    second.level = 10;
    let mut world = World::default();
    world.add_character(attacker);
    world.add_character(first);
    world.add_character(second);
    let mut player = PlayerRuntime::connected(1, 0);

    apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/hate first", 0)
        .expect("hate command should be recognized");
    apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/hate second", 0)
        .expect("hate command should be recognized");
    let refreshed =
        apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/hate first", 0)
            .expect("hate command should be recognized");

    assert_eq!(player.pk_hate, vec![8, 9]);
    assert_eq!(refreshed.name_refresh, vec![CharacterId(7), CharacterId(8)]);
}

#[test]
fn pk_hate_command_clear_requires_pk_and_clears_runtime_list() {
    let mut character = login_character(CharacterId(7), &login_block("Attacker"), 1, 10, 10);
    character.flags.remove(CharacterFlags::PK);
    let mut world = World::default();
    world.add_character(character);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(7));
    assert!(player.add_pk_hate(8));

    let not_pk = apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/clearhate", 0)
        .expect("clearhate command should be recognized");
    assert!(not_pk.messages.is_empty());
    assert!(player.has_pk_hate_for(8));

    world
        .characters
        .get_mut(&CharacterId(7))
        .unwrap()
        .flags
        .insert(CharacterFlags::PK);
    let cleared = apply_pk_hate_command(&mut world, &mut player, CharacterId(7), "/clearhate", 0)
        .expect("clearhate command should be recognized");
    assert!(cleared.messages.is_empty());
    assert!(player.pk_hate.is_empty());
}

#[test]
fn pk_playerkiller_command_requires_level_and_paid_before_confirmation() {
    let mut character = login_character(CharacterId(77), &login_block("Tester"), 1, 10, 10);
    character.level = 9;
    let mut world = World::default();
    world.add_character(character);
    let mut player = PlayerRuntime::connected(1, 0);

    let low_level =
        apply_pk_hate_command(&mut world, &mut player, CharacterId(77), "/playerkiller", 0)
            .expect("playerkiller command should be recognized");
    assert_eq!(
        low_level.messages,
        vec![
            "Sorry, you may not become a player killer before reaching level 10.",
            "PK is off."
        ]
    );

    let character = world.characters.get_mut(&CharacterId(77)).unwrap();
    character.level = 10;
    let unpaid =
        apply_pk_hate_command(&mut world, &mut player, CharacterId(77), "/playerkiller", 0)
            .expect("playerkiller command should be recognized");
    assert_eq!(
        unpaid.messages,
        vec![
            "Sorry, only paying players may become player killers.",
            "PK is off."
        ]
    );

    world
        .characters
        .get_mut(&CharacterId(77))
        .unwrap()
        .flags
        .insert(CharacterFlags::PAID);
    let confirm =
        apply_pk_hate_command(&mut world, &mut player, CharacterId(77), "/playerkiller", 0)
            .expect("playerkiller command should be recognized");
    assert_eq!(confirm.messages.len(), 2);
    assert!(confirm.messages[0].contains("Type: '/iwilldie 77' to confirm."));
    assert_eq!(confirm.messages[1], "PK is off.");
}

#[test]
fn pk_iwilldie_command_toggles_pk_and_clears_ppd_like_state() {
    let mut character = login_character(CharacterId(77), &login_block("Tester"), 1, 10, 10);
    character.level = 10;
    character.flags.insert(CharacterFlags::PAID);
    let mut world = World::default();
    world.add_character(character);
    let mut player = PlayerRuntime::connected(1, 0);
    player.pk_kills = 3;
    player.pk_deaths = 2;
    player.pk_last_kill = 123;
    player.pk_last_death = 456;
    assert!(player.add_pk_hate(999));

    let wrong = apply_pk_hate_command(&mut world, &mut player, CharacterId(77), "/iwilldie 76", 0)
        .expect("iwilldie command should be recognized");
    assert_eq!(
        wrong.messages,
        vec!["Please type: '/playerkiller' first.", "PK is off."]
    );

    let joined = apply_pk_hate_command(
        &mut world,
        &mut player,
        CharacterId(77),
        "/iwilldie 77abc",
        0,
    )
    .expect("iwilldie command should be recognized");
    assert_eq!(joined.messages, vec!["PK is on."]);
    assert!(world
        .characters
        .get(&CharacterId(77))
        .unwrap()
        .flags
        .contains(CharacterFlags::PK));
    assert_eq!(player.pk_kills, 0);
    assert_eq!(player.pk_deaths, 0);
    assert_eq!(player.pk_last_kill, 0);
    assert_eq!(player.pk_last_death, 0);
    assert!(player.pk_hate.is_empty());
}

#[test]
fn pk_playerkiller_leave_respects_tired_and_kill_cooldown() {
    let mut character = login_character(CharacterId(77), &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::PK);
    character.regen_ticker = 10;
    let mut world = World::default();
    world.tick.0 = 20;
    world.add_character(character);
    let mut player = PlayerRuntime::connected(1, 0);

    let tired = apply_pk_hate_command(&mut world, &mut player, CharacterId(77), "/playerkiller", 0)
        .expect("playerkiller command should be recognized");
    assert_eq!(tired.messages, vec!["Pant, pant. Too tired.", "PK is on."]);

    world.tick.0 = TICKS_PER_SECOND * 4;
    player.pk_last_kill = 60 * 60 * 24 * 27;
    let blocked = apply_pk_hate_command(
        &mut world,
        &mut player,
        CharacterId(77),
        "/playerkiller",
        60 * 60 * 24 * 27,
    )
    .expect("playerkiller command should be recognized");
    assert_eq!(
        blocked.messages,
        vec![
            "You have killed 0.00 days ago, you need to wait 28.00 more days.",
            "PK is on."
        ]
    );

    let left = apply_pk_hate_command(
        &mut world,
        &mut player,
        CharacterId(77),
        "/playerkiller",
        60 * 60 * 24 * 56,
    )
    .expect("playerkiller command should be recognized");
    assert_eq!(left.messages, vec!["PK is off."]);
    assert!(!world
        .characters
        .get(&CharacterId(77))
        .unwrap()
        .flags
        .contains(CharacterFlags::PK));
}

#[test]
fn hurt_events_add_pk_hate_and_clear_lag_for_valid_player_hit() {
    let mut world = World::default();
    let mut target = login_character(CharacterId(1), &login_block("Target"), 1, 10, 10);
    target
        .flags
        .insert(CharacterFlags::PK | CharacterFlags::LAG);
    target.level = 10;
    let mut attacker = login_character(CharacterId(2), &login_block("Attacker"), 1, 11, 10);
    attacker.flags.insert(CharacterFlags::PK);
    attacker.level = 12;
    world.add_character(target);
    world.add_character(attacker);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    runtime.players.insert(1, player);

    world.apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), 0, 1, 0, 0);

    assert_eq!(
        apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 123, &ZoneLoader::new()),
        1
    );
    assert!(runtime
        .player_for_character(CharacterId(1))
        .unwrap()
        .has_pk_hate_for(2));
    assert!(!world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .flags
        .contains(CharacterFlags::LAG));
}

#[test]
fn lethal_teufel_rat_hurt_updates_legacy_rat_ppd_score() {
    let mut world = World::default();
    let mut rat = login_character(CharacterId(1), &login_block("Rat"), 34, 10, 10);
    rat.flags.remove(CharacterFlags::PLAYER);
    rat.driver = CDR_TEUFELRAT;
    rat.level = 80;
    rat.hp = POWERSCALE;
    let mut killer = login_character(CharacterId(2), &login_block("Killer"), 34, 11, 10);
    killer.flags.insert(CharacterFlags::LAG);
    world.add_character(rat);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    runtime.players.insert(1, player);

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    assert_eq!(
        apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new()),
        0
    );

    let player = runtime.player_for_character(CharacterId(2)).unwrap();
    assert_eq!(player.teufel_rat_kills, 1);
    assert_eq!(player.teufel_rat_score, 1);
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| text.message == "#90 1 Rat Kills"));
    assert!(texts.iter().any(|text| text.message == "#80 1 Rat Points"));
}

#[test]
fn lethal_caligar_skelly_hurt_marks_killer_door_lock_ppd() {
    let mut world = World::default();
    let mut skelly = login_character(CharacterId(1), &login_block("Skelly"), 36, 10, 10);
    skelly.flags.remove(CharacterFlags::PLAYER);
    skelly.driver = CDR_CALIGARSKELLY;
    skelly.rest_x = 103;
    skelly.rest_y = 224;
    skelly.hp = POWERSCALE;
    let killer = login_character(CharacterId(2), &login_block("Killer"), 36, 11, 10);
    world.add_character(skelly);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    runtime.players.insert(1, player);

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    assert_eq!(
        apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new()),
        0
    );

    assert!(!runtime
        .player_for_character(CharacterId(2))
        .unwrap()
        .caligar_skelly_door_unlocked(0));
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| {
        text.character_id == CharacterId(2)
            && text.message
                == "You hear a faint sound in the distance, as if a lock was partially opened."
    }));
}

#[test]
fn lethal_caligar_skelly_hurt_reports_completed_and_repeated_locks() {
    let mut world = World::default();
    let mut killer = login_character(CharacterId(2), &login_block("Killer"), 36, 11, 10);
    killer.hp = POWERSCALE * 10;
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    player.mark_caligar_skelly_death(103, 224);
    player.mark_caligar_skelly_death(103, 211);
    runtime.players.insert(1, player);

    for (id, y) in [(1, 198), (3, 198)] {
        let mut skelly = login_character(CharacterId(id), &login_block("Skelly"), 36, 10, 10);
        skelly.flags.remove(CharacterFlags::PLAYER);
        skelly.driver = CDR_CALIGARSKELLY;
        skelly.rest_x = 103;
        skelly.rest_y = y;
        skelly.hp = POWERSCALE;
        world.add_character(skelly);
        world.apply_legacy_hurt(
            CharacterId(id),
            Some(CharacterId(2)),
            POWERSCALE * 2,
            1,
            0,
            0,
        );
        apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());
    }

    assert!(runtime
        .player_for_character(CharacterId(2))
        .unwrap()
        .caligar_skelly_door_unlocked(0));
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| {
        text.message == "You hear a \"click\" in the distance, as if a lock had opened."
    }));
    assert!(texts.iter().any(|text| {
        text.message
            == "You expect to hear a click, but nothing happens. Maybe you've been here before?"
    }));
}

#[test]
fn lethal_riverbeast_hurt_advances_jiu_state_to_beast_killed() {
    let mut world = World::default();
    let mut riverbeast = login_character(CharacterId(1), &login_block("Riverbeast"), 1, 10, 10);
    riverbeast.flags.remove(CharacterFlags::PLAYER);
    riverbeast.driver = CDR_RIVERBEAST;
    riverbeast.hp = POWERSCALE;
    let killer = login_character(CharacterId(2), &login_block("Killer"), 1, 11, 10);
    world.add_character(riverbeast);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    // C `JIU_STATE_WAIT_FOR_KILL 2` (`npc_states.h:78`).
    player.set_area1_jiu_state(2);
    runtime.players.insert(1, player);

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    // C `JIU_STATE_BEAST_KILLED 3` (`npc_states.h:79`).
    assert_eq!(
        runtime
            .player_for_character(CharacterId(2))
            .unwrap()
            .area1_jiu_state(),
        3
    );
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| {
        text.character_id == CharacterId(2)
            && text.message == "Well done. Jiu will be proud of thee!"
    }));
}

#[test]
fn lethal_riverbeast_hurt_ignores_players_not_awaiting_the_kill() {
    let mut world = World::default();
    let mut riverbeast = login_character(CharacterId(1), &login_block("Riverbeast"), 1, 10, 10);
    riverbeast.flags.remove(CharacterFlags::PLAYER);
    riverbeast.driver = CDR_RIVERBEAST;
    riverbeast.hp = POWERSCALE;
    let killer = login_character(CharacterId(2), &login_block("Killer"), 1, 11, 10);
    world.add_character(riverbeast);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    // C `JIU_STATE_ENTRY 0` (`npc_states.h:76`) - hasn't taken the quest.
    player.set_area1_jiu_state(0);
    runtime.players.insert(1, player);

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    assert_eq!(
        runtime
            .player_for_character(CharacterId(2))
            .unwrap()
            .area1_jiu_state(),
        0
    );
    assert!(world.drain_pending_system_texts().is_empty());
}

/// C `ch_died_driver`/`CDR_LAMPGHOST` -> `lampghost_dead` (`area3.c:2741-
/// 2752`): the dying lampghost's claimed lamp is released so another
/// lampghost (or the same one on respawn) can pick it up.
#[test]
fn lethal_lampghost_hurt_releases_its_lamp_claim() {
    let mut world = World::default();
    let mut lampghost = login_character(CharacterId(1), &login_block("Lampghost"), 1, 10, 10);
    lampghost.flags.remove(CharacterFlags::PLAYER);
    lampghost.driver = CDR_LAMPGHOST;
    lampghost.hp = POWERSCALE;
    let killer = login_character(CharacterId(2), &login_block("Killer"), 1, 11, 10);
    world.add_character(lampghost);
    world.add_character(killer);
    world
        .area3_lamp_claims
        .insert(ItemId(9), (CharacterId(1), 5));
    // An unrelated claim by a different lampghost must survive untouched.
    world
        .area3_lamp_claims
        .insert(ItemId(10), (CharacterId(3), 2));

    let mut runtime = ServerRuntime::default();
    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    assert!(!world.area3_lamp_claims.contains_key(&ItemId(9)));
    assert_eq!(
        world.area3_lamp_claims.get(&ItemId(10)),
        Some(&(CharacterId(3), 2))
    );
}

#[test]
fn lethal_bredel_hurt_advances_jessica_state_to_quest2_finish() {
    let mut world = World::default();
    let mut bredel = login_character(CharacterId(1), &login_block("Bredel"), 1, 10, 10);
    bredel.flags.remove(CharacterFlags::PLAYER);
    bredel.driver = CDR_BREDEL;
    bredel.hp = POWERSCALE;
    let killer = login_character(CharacterId(2), &login_block("Killer"), 1, 11, 10);
    world.add_character(bredel);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    // C `JESSICA_STATE_QUEST2_DO 10` (`npc_states.h:94`).
    player.set_area1_jessica_state(10);
    runtime.players.insert(1, player);

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    // C `JESSICA_STATE_QUEST2_FINISH 11` (`npc_states.h:95`).
    assert_eq!(
        runtime
            .player_for_character(CharacterId(2))
            .unwrap()
            .area1_jessica_state(),
        11
    );
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| {
        text.character_id == CharacterId(2)
            && text.message
                == "The local robber leader has been killed by thine hands. Congratulations!"
    }));
}

#[test]
fn lethal_bigbadspider_hurt_completes_brithildie_quest_and_advances_state() {
    let mut world = World::default();
    let mut bigbadspider = login_character(CharacterId(1), &login_block("Spider"), 1, 10, 10);
    bigbadspider.flags.remove(CharacterFlags::PLAYER);
    bigbadspider.driver = CDR_BIGBADSPIDER;
    bigbadspider.hp = POWERSCALE;
    let killer = login_character(CharacterId(2), &login_block("Killer"), 1, 11, 10);
    world.add_character(bigbadspider);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    // C `BRITHILDIE_STATE_NOMORETALES_QOPEN 20` (`npc_states.h:71`).
    player.set_area1_brithildie_state(20);
    runtime.players.insert(1, player);

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    // C `BRITHILDIE_STATE_NOMORETALES_QDONE 21` (`npc_states.h:72`).
    assert_eq!(
        runtime
            .player_for_character(CharacterId(2))
            .unwrap()
            .area1_brithildie_state(),
        21
    );
    assert!(runtime
        .player_for_character(CharacterId(2))
        .unwrap()
        .quest_log
        .is_done(QLOG_BRITHILDIE));
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| {
        text.character_id == CharacterId(2)
            && text.message == "Well done. Thou hast killed the big bad spider."
    }));
}

#[test]
fn lethal_bigbadspider_hurt_ignores_players_not_awaiting_the_reward() {
    let mut world = World::default();
    let mut bigbadspider = login_character(CharacterId(1), &login_block("Spider"), 1, 10, 10);
    bigbadspider.flags.remove(CharacterFlags::PLAYER);
    bigbadspider.driver = CDR_BIGBADSPIDER;
    bigbadspider.hp = POWERSCALE;
    let killer = login_character(CharacterId(2), &login_block("Killer"), 1, 11, 10);
    world.add_character(bigbadspider);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    // C `BRITHILDIE_STATE_ENTRY 0` (`npc_states.h:51`) - never took the
    // quest.
    player.set_area1_brithildie_state(0);
    runtime.players.insert(1, player);

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    assert_eq!(
        runtime
            .player_for_character(CharacterId(2))
            .unwrap()
            .area1_brithildie_state(),
        0
    );
    assert!(world.drain_pending_system_texts().is_empty());
}

#[test]
fn lethal_forest_monster_hurt_counts_camhermit_kills_and_reports_at_ten() {
    let mut world = World::default();
    let killer = login_character(CharacterId(99), &login_block("Killer"), 1, 20, 60);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(99));
    // C `CAMHERMIT_STATE_QUEST1DO 5` (`npc_states.h:16`).
    player.set_area1_camhermit_state(5);
    player.set_area1_camhermit_kills(9);
    runtime.players.insert(1, player);

    let mut bear = login_character(CharacterId(1), &login_block("Bear"), 1, 10, 10);
    bear.flags.remove(CharacterFlags::PLAYER);
    bear.driver = CDR_CAMERON_FORESTMONSTER;
    bear.hp = POWERSCALE;
    world.add_character(bear);

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(99)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    assert_eq!(
        runtime
            .player_for_character(CharacterId(99))
            .unwrap()
            .area1_camhermit_kills(),
        10
    );
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| {
        text.character_id == CharacterId(99)
            && text.message
                == "Thou hast killed 10 big bears as requested by the sweet Hermit. go back to him and claim thy reward."
    }));
}

#[test]
fn hurt_events_start_legacy_player_fightback_for_nearby_attacker() {
    let mut world = World::default();
    let target = login_character(CharacterId(1), &login_block("Target"), 1, 10, 10);
    let mut attacker = login_character(CharacterId(2), &login_block("Attacker"), 1, 11, 10);
    attacker.serial = 99;
    world.add_character(target);
    world.add_character(attacker);
    world.tick.0 = TICKS_PER_SECOND * 4;

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    runtime.players.insert(1, player);

    world.apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), 0, 1, 0, 0);
    assert_eq!(
        apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new()),
        0
    );

    let player = runtime.player_for_character(CharacterId(1)).unwrap();
    assert_eq!(player.action.action, PlayerActionCode::Kill);
    assert_eq!((player.action.arg1, player.action.arg2), (2, 99));
}

#[test]
fn hurt_events_defer_legacy_player_fightback_while_busy() {
    let mut world = World::default();
    let target = login_character(CharacterId(1), &login_block("Target"), 1, 10, 10);
    let mut attacker = login_character(CharacterId(2), &login_block("Attacker"), 1, 11, 10);
    attacker.serial = 99;
    world.add_character(target);
    world.add_character(attacker);
    world.tick.0 = TICKS_PER_SECOND * 4;

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.driver_move(12, 10);
    runtime.players.insert(1, player);

    world.apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), 0, 1, 0, 0);
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    let player = runtime.player_for_character(CharacterId(1)).unwrap();
    assert_eq!(player.action.action, PlayerActionCode::Move);
    assert_eq!(player.next_fightback_character, Some(CharacterId(2)));
    assert_eq!(player.next_fightback_serial, 99);
}

#[test]
fn setup_world_actions_promotes_deferred_legacy_player_fightback() {
    let mut world = World::default();
    let mut target = login_character(CharacterId(1), &login_block("Target"), 1, 10, 10);
    target.x = 10;
    target.y = 10;
    let mut attacker = login_character(CharacterId(2), &login_block("Attacker"), 1, 11, 10);
    attacker.flags.remove(CharacterFlags::PLAYER);
    attacker.x = 11;
    attacker.y = 10;
    attacker.serial = 99;
    world.map.tile_mut(10, 10).unwrap().character = 1;
    world.map.tile_mut(11, 10).unwrap().character = 2;
    world.add_character(target);
    world.add_character(attacker);
    world.tick.0 = TICKS_PER_SECOND * 4 + 1;

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.next_fightback_character = Some(CharacterId(2));
    player.next_fightback_serial = 99;
    player.next_fightback_tick = TICKS_PER_SECOND * 4;
    runtime.players.insert(1, player);

    assert_eq!(runtime.setup_world_actions(&mut world, 1), 1);

    let target = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(target.action, action::ATTACK1);
    assert_eq!(target.act1, 2);
}

#[test]
fn hurt_events_respect_legacy_pk_hate_level_gate() {
    let mut world = World::default();
    let mut target = login_character(CharacterId(1), &login_block("Target"), 1, 10, 10);
    target
        .flags
        .insert(CharacterFlags::PK | CharacterFlags::LAG);
    target.level = 10;
    let mut attacker = login_character(CharacterId(2), &login_block("Attacker"), 1, 11, 10);
    attacker.flags.insert(CharacterFlags::PK);
    attacker.level = 14;
    world.add_character(target);
    world.add_character(attacker);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    runtime.players.insert(1, player);

    world.apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), 0, 1, 0, 0);

    assert_eq!(
        apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 123, &ZoneLoader::new()),
        0
    );
    assert!(!runtime
        .player_for_character(CharacterId(1))
        .unwrap()
        .has_pk_hate_for(2));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .flags
        .contains(CharacterFlags::LAG));
}

#[test]
fn lethal_pk_hurt_events_update_kill_and_death_counters() {
    let mut world = World::default();
    let mut target = login_character(CharacterId(1), &login_block("Target"), 1, 10, 10);
    target.flags.insert(CharacterFlags::PK);
    target.level = 10;
    target.hp = 100;
    let mut attacker = login_character(CharacterId(2), &login_block("Attacker"), 1, 11, 10);
    attacker.flags.insert(CharacterFlags::PK);
    attacker.level = 11;
    world.add_character(target);
    world.add_character(attacker);

    let mut runtime = ServerRuntime::default();
    let mut target_player = PlayerRuntime::connected(1, 0);
    target_player.character_id = Some(CharacterId(1));
    let mut attacker_player = PlayerRuntime::connected(2, 0);
    attacker_player.character_id = Some(CharacterId(2));
    runtime.players.insert(1, target_player);
    runtime.players.insert(2, attacker_player);

    world.apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), 1_000, 1, 0, 0);

    assert_eq!(
        apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 12_345, &ZoneLoader::new()),
        1
    );
    let target_player = runtime.player_for_character(CharacterId(1)).unwrap();
    assert_eq!(target_player.pk_deaths, 1);
    assert_eq!(target_player.pk_last_death, 12_345);
    let attacker_player = runtime.player_for_character(CharacterId(2)).unwrap();
    assert_eq!(attacker_player.pk_kills, 1);
    assert_eq!(attacker_player.pk_last_kill, 12_345);
}

#[test]
fn lethal_gate_fight_hurt_grants_arch_warrior_and_teleports_killer() {
    let mut world = World::default();
    let mut opponent = login_character(CharacterId(1), &login_block("Gatekeeper"), 1, 190, 200);
    opponent.flags.remove(CharacterFlags::PLAYER);
    opponent.driver = CDR_GATE_FIGHT;
    opponent.hp = POWERSCALE;
    let killer = login_character(CharacterId(2), &login_block("Godmode"), 1, 191, 200);
    world.add_character(opponent);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    player.gate_target_class = 5;
    runtime.players.insert(1, player);

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    let killer = world.characters.get(&CharacterId(2)).unwrap();
    assert!(killer.flags.contains(CharacterFlags::ARCH));
    assert_eq!((killer.x, killer.y), (181, 198));
    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message == "You are an Arch-Warrior now."));
}

#[test]
fn lethal_gate_fight_hurt_by_non_player_does_not_grant_reward() {
    let mut world = World::default();
    let mut opponent = login_character(CharacterId(1), &login_block("Gatekeeper"), 1, 190, 200);
    opponent.flags.remove(CharacterFlags::PLAYER);
    opponent.driver = CDR_GATE_FIGHT;
    opponent.hp = POWERSCALE;
    let mut other_npc = login_character(CharacterId(2), &login_block("Other"), 1, 191, 200);
    other_npc.flags.remove(CharacterFlags::PLAYER);
    world.add_character(opponent);
    world.add_character(other_npc);

    let mut runtime = ServerRuntime::default();

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    let other_npc = world.characters.get(&CharacterId(2)).unwrap();
    assert!(!other_npc.flags.contains(CharacterFlags::ARCH));
    let texts = world.drain_pending_system_texts();
    assert!(!texts.iter().any(|text| text.message == "Well done."));
}

#[test]
fn lethal_gate_fight_hurt_class_eight_turns_killer_seyan_and_clears_turn_seyan_ppd() {
    let mut world = World::default();
    let mut opponent = login_character(CharacterId(1), &login_block("Gatekeeper"), 1, 190, 200);
    opponent.flags.remove(CharacterFlags::PLAYER);
    opponent.driver = CDR_GATE_FIGHT;
    opponent.hp = POWERSCALE;
    let mut killer = login_character(CharacterId(2), &login_block("Godmode"), 40, 191, 200);
    killer.flags.insert(CharacterFlags::ARCH);
    killer.exp = 500_000;
    world.add_character(opponent);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    player.gate_target_class = 8;
    player.demonshrines.push(77);
    runtime.players.insert(1, player);

    let mut loader = ZoneLoader::new();
    loader
        .load_character_templates_str(
            r#"
                seyan_m:
                  name="Seyan'Du"
                  description="A Seyan'Du"
                  V_HP=10
                  V_ENDURANCE=8
                  V_MANA=6
                ;
            "#,
        )
        .unwrap();

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &loader);

    let killer = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(killer.level, 1);
    assert_eq!(killer.exp, 0);
    assert!(killer.flags.contains(CharacterFlags::MAGE));
    assert!(killer.flags.contains(CharacterFlags::WARRIOR));
    assert_eq!((killer.x, killer.y), (181, 198));

    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message == "You are a Seyan'Du now."));

    let player = runtime.player_for_character(CharacterId(2)).unwrap();
    assert!(player.demonshrines.is_empty());
}

#[test]
fn gate_welcome_death_is_handled_but_sends_no_client_message() {
    // C `ch_died_driver`/`CDR_GATE_WELCOME` -> `immortal_dead` just writes
    // a server-log-only line via `charlog`; nothing should reach the
    // client and no reward/teleport logic should run (unlike gate_fight).
    let mut world = World::default();
    let mut welcome_npc = login_character(CharacterId(1), &login_block("Gatekeeper"), 1, 190, 200);
    welcome_npc.flags.remove(CharacterFlags::PLAYER);
    welcome_npc.driver = CDR_GATE_WELCOME;
    welcome_npc.hp = POWERSCALE;
    let killer = login_character(CharacterId(2), &login_block("Godmode"), 1, 191, 200);
    world.add_character(welcome_npc);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    // No client-visible text should have been queued by the handler.
    let texts = world.drain_pending_system_texts();
    assert!(!texts.iter().any(|text| text.message.contains("IMMORTAL")));
}

#[test]
fn gate_welcome_death_handler_ignores_non_matching_driver_and_non_lethal_hits() {
    let mut world = World::default();
    let mut other_npc = login_character(CharacterId(1), &login_block("Other"), 1, 190, 200);
    other_npc.flags.remove(CharacterFlags::PLAYER);
    other_npc.hp = POWERSCALE * 5;
    world.add_character(other_npc);

    let non_lethal = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: false,
            ..Default::default()
        },
    };
    assert!(!apply_gate_welcome_death_from_hurt_event(
        &world, non_lethal
    ));

    let mut world2 = World::default();
    let mut welcome_npc = login_character(CharacterId(1), &login_block("Gatekeeper"), 1, 190, 200);
    welcome_npc.flags.remove(CharacterFlags::PLAYER);
    welcome_npc.driver = CDR_GATE_FIGHT; // not CDR_GATE_WELCOME
    world2.add_character(welcome_npc);

    let lethal_wrong_driver = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };
    assert!(!apply_gate_welcome_death_from_hurt_event(
        &world2,
        lethal_wrong_driver
    ));
}

#[test]
fn dungeonmaster_death_is_handled_but_sends_no_client_message() {
    // C `ch_died_driver`/`CDR_DUNGEONMASTER` -> `immortal_dead`
    // (`dungeon.c:1735-1737,2197-2200`) is the same `charlog`-only bug
    // line as `CDR_GATE_WELCOME`'s - no client message, no reward/
    // teleport side effects.
    let mut world = World::default();
    let mut dungeonmaster_npc =
        login_character(CharacterId(1), &login_block("Dungeonmaster"), 1, 190, 200);
    dungeonmaster_npc.flags.remove(CharacterFlags::PLAYER);
    dungeonmaster_npc.driver = CDR_DUNGEONMASTER;
    dungeonmaster_npc.hp = POWERSCALE;
    let killer = login_character(CharacterId(2), &login_block("Godmode"), 1, 191, 200);
    world.add_character(dungeonmaster_npc);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    let texts = world.drain_pending_system_texts();
    assert!(!texts.iter().any(|text| text.message.contains("IMMORTAL")));
}

#[test]
fn dungeonmaster_death_handler_ignores_non_matching_driver_and_non_lethal_hits() {
    let mut world = World::default();
    let mut other_npc = login_character(CharacterId(1), &login_block("Other"), 1, 190, 200);
    other_npc.flags.remove(CharacterFlags::PLAYER);
    other_npc.hp = POWERSCALE * 5;
    world.add_character(other_npc);

    let non_lethal = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: false,
            ..Default::default()
        },
    };
    assert!(!apply_dungeonmaster_death_from_hurt_event(
        &world, non_lethal
    ));

    let mut world2 = World::default();
    let mut dungeonmaster_npc =
        login_character(CharacterId(1), &login_block("Dungeonmaster"), 1, 190, 200);
    dungeonmaster_npc.flags.remove(CharacterFlags::PLAYER);
    dungeonmaster_npc.driver = CDR_GATE_FIGHT; // not CDR_DUNGEONMASTER
    world2.add_character(dungeonmaster_npc);

    let lethal_wrong_driver = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };
    assert!(!apply_dungeonmaster_death_from_hurt_event(
        &world2,
        lethal_wrong_driver
    ));
}

#[test]
fn area1_quest_giver_death_is_handled_but_sends_no_client_message() {
    // C `ch_died_driver`'s remaining area-1 dispatch branches all route
    // to `gwendylon_dead` (`gwendylon.c:6180-6206`), the same
    // `charlog`-only bug line as `CDR_GATE_WELCOME`/`CDR_DUNGEONMASTER`'s
    // - no client message, no reward/teleport side effects. Spot-check
    // one representative driver from the ten (`CDR_LOGAIN`, the last one
    // added).
    let mut world = World::default();
    let mut logain_npc = login_character(CharacterId(1), &login_block("Logain"), 1, 190, 200);
    logain_npc.flags.remove(CharacterFlags::PLAYER);
    logain_npc.driver = CDR_LOGAIN;
    logain_npc.hp = POWERSCALE;
    let killer = login_character(CharacterId(2), &login_block("Godmode"), 1, 191, 200);
    world.add_character(logain_npc);
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    let texts = world.drain_pending_system_texts();
    assert!(!texts.iter().any(|text| text.message.contains("IMMORTAL")));
}

#[test]
fn area1_quest_giver_death_handler_covers_every_listed_driver() {
    for driver in [
        CDR_TERION,
        CDR_JAMES,
        CDR_NOOK,
        CDR_LYDIA,
        CDR_GUIWYNN,
        CDR_LOGAIN,
        CDR_CAMHERMIT,
        CDR_GREETER,
        CDR_JESSICA,
        CDR_BRITHILDIE,
    ] {
        let mut world = World::default();
        let mut npc = login_character(CharacterId(1), &login_block("Npc"), 1, 190, 200);
        npc.flags.remove(CharacterFlags::PLAYER);
        npc.driver = driver;
        world.add_character(npc);

        let lethal = LegacyHurtEvent {
            target_id: CharacterId(1),
            cause_id: CharacterId(2),
            outcome: LegacyHurtOutcome {
                killed: true,
                ..Default::default()
            },
        };
        assert!(
            apply_area1_quest_giver_death_from_hurt_event(&world, lethal),
            "driver {driver} was not covered"
        );
    }
}

#[test]
fn area1_quest_giver_death_handler_ignores_non_matching_driver_and_non_lethal_hits() {
    let mut world = World::default();
    let mut other_npc = login_character(CharacterId(1), &login_block("Other"), 1, 190, 200);
    other_npc.flags.remove(CharacterFlags::PLAYER);
    other_npc.hp = POWERSCALE * 5;
    world.add_character(other_npc);

    let non_lethal = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: false,
            ..Default::default()
        },
    };
    assert!(!apply_area1_quest_giver_death_from_hurt_event(
        &world, non_lethal
    ));

    let mut world2 = World::default();
    let mut logain_npc = login_character(CharacterId(1), &login_block("Logain"), 1, 190, 200);
    logain_npc.flags.remove(CharacterFlags::PLAYER);
    logain_npc.driver = CDR_GATE_FIGHT; // not one of the listed drivers
    world2.add_character(logain_npc);

    let lethal_wrong_driver = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };
    assert!(!apply_area1_quest_giver_death_from_hurt_event(
        &world2,
        lethal_wrong_driver
    ));
}

#[tokio::test]
async fn clan_economy_tick_escalates_mutual_relation_request_immediately() {
    // `apply_clan_economy_tick`'s relation half wires `ClanRelations::
    // update` (`clan.c:936-1089`) into the live tick loop; the escalation/
    // de-escalation state machine itself is exhaustively unit-tested in
    // `ugaris-core`'s `clan.rs`, so this only checks the wiring: the
    // registry's live relation state actually advances and the returned
    // `applied` count reflects the one pair-level change.
    let mut world = World::default();
    let a = world.clan_registry.found_clan("Alpha", 0).unwrap();
    let b = world.clan_registry.found_clan("Beta", 0).unwrap();
    world
        .clan_registry
        .relations_mut()
        .set_relation(a, b, ugaris_core::clan::ClanRelation::War, 0)
        .unwrap();
    world
        .clan_registry
        .relations_mut()
        .set_relation(b, a, ugaris_core::clan::ClanRelation::War, 0)
        .unwrap();

    let applied = apply_clan_economy_tick(&mut world, &None, 0).await;

    assert_eq!(applied, 1);
    assert_eq!(
        world.clan_registry.relations().current_relation(a, b),
        ugaris_core::clan::ClanRelation::War
    );
}

#[tokio::test]
async fn clan_economy_tick_deletes_a_clan_that_goes_broke() {
    // Wires `ClanRegistry::update_treasure` (`clan.c:1105-1159`) into the
    // live tick loop: a freshly founded clan with no jewels and a huge
    // elapsed `payed_till` gap accrues enough debt in one tick to be
    // deleted, matching what `/killclan`'s huge-debt trick eventually
    // triggers in C (`kill_clan`, `clan.c:1413-1416`).
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Broke", 0).unwrap();
    assert!(world.clan_registry.exists(nr));

    // cost = 5000, step = 120; diff = 250_000 => n = 250000/120 + 1 = 2084,
    // landing debt at 2084 (>= 2000) with zero jewels to pay it off (same
    // arithmetic as `ugaris-core`'s own
    // `update_treasure_deletes_clan_that_goes_broke_with_no_jewels` test).
    let applied = apply_clan_economy_tick(&mut world, &None, 250_000).await;

    assert_eq!(applied, 1);
    assert!(!world.clan_registry.exists(nr));
}

#[tokio::test]
async fn clan_economy_tick_advances_training_update_timestamp_after_an_hour() {
    // Wires `ClanRegistry::update_training` (`clan.c:1166-1182`) into the
    // live tick loop. `training_score` itself only ever decays (nothing
    // feeds it yet - the dungeon system that would is unported, see the
    // module doc comment), so a freshly founded clan's score stays `0`
    // either way; `last_training_update` advancing is the observable
    // signal that the sub-tick actually ran (exact 5%-decay arithmetic
    // is unit-tested directly in `ugaris-core`'s
    // `update_training_decays_score_by_five_percent_after_one_hour`).
    let mut world = World::default();
    let nr = world.clan_registry.found_clan("Trainers", 0).unwrap();
    assert_eq!(
        world
            .clan_registry
            .identity(nr)
            .unwrap()
            .economy
            .last_training_update,
        0
    );

    apply_clan_economy_tick(&mut world, &None, 3_600).await;

    assert_eq!(
        world
            .clan_registry
            .identity(nr)
            .unwrap()
            .economy
            .last_training_update,
        3_600
    );
}

#[test]
fn apply_arena_master_events_falls_back_to_fighter_bots_own_ledger_when_a_combatant_has_no_player_runtime(
) {
    // A real player (winner) and a `CDR_ARENAFIGHTER` practice bot (loser,
    // no `PlayerRuntime`) just finished an arena fight - the bot fled the
    // box, so `check_fight` scores the player as the winner.
    let mut world = World::default();
    let master_id = CharacterId(1);
    let winner_id = CharacterId(2);
    let loser_id = CharacterId(3);

    let mut master = login_character(master_id, &login_block("Arenamaster"), 3, 236, 145);
    master.flags.remove(CharacterFlags::PLAYER);
    master.driver = CDR_ARENAMASTER;
    master.driver_state = Some(CharacterDriverState::ArenaMaster(ArenaMasterDriverData {
        state: MS_FIGHT,
        fight1: Some(winner_id),
        fight2: Some(loser_id),
        timeout: 1_000,
        ..Default::default()
    }));
    world.add_character(master);

    let mut winner = login_character(winner_id, &login_block("Godmode"), 3, 235, 140);
    winner.x = 235;
    winner.y = 140;
    world.add_character(winner);

    // The fighter bot fled the arena box (outside the `234..=242,
    // 133..=141` bounds), so it loses by default this tick.
    let mut loser = login_character(loser_id, &login_block("Fighter"), 3, 10, 10);
    loser.flags.remove(CharacterFlags::PLAYER);
    loser.x = 10;
    loser.y = 10;
    loser.driver = CDR_ARENAFIGHTER;
    loser.driver_state = Some(CharacterDriverState::ArenaFighter(
        ArenaFighterDriverData::default(),
    ));
    world.add_character(loser);

    let mut runtime = ServerRuntime::default();
    let mut winner_player = PlayerRuntime::connected(20, 0);
    winner_player.character_id = Some(winner_id);
    runtime.players.insert(20, winner_player);

    world.process_arena_master_actions(0, |character_id| {
        runtime
            .player_for_character(character_id)
            .map(|player| player.arena_score())
            .unwrap_or(ARENA_PPD_NEWCOMER_SCORE)
    });

    let applied = apply_arena_master_events(&mut world, &mut runtime, 1_000_000);

    assert_eq!(applied, 1);
    // The winner's real `PlayerRuntime` arena_ppd was updated.
    let new_winner_score = runtime
        .player_for_character(winner_id)
        .unwrap()
        .arena_score();
    assert_eq!(
        new_winner_score,
        ARENA_PPD_NEWCOMER_SCORE + ugaris_core::player::PlayerRuntime::arena_fight_worth(0)
    );
    // The loser has no `PlayerRuntime` at all - its own local ledger
    // (`ArenaFighterDriverData`) was updated instead.
    assert_eq!(
        world.arena_fighter_score(loser_id),
        Some(ARENA_PPD_NEWCOMER_SCORE - ugaris_core::player::PlayerRuntime::arena_fight_worth(0))
    );
    let entries = world.arena_toplist_entries();
    assert!(entries.iter().any(|e| e.name == "Godmode"));
    assert!(entries.iter().any(|e| e.name == "Fighter"));
}
