use super::*;

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
