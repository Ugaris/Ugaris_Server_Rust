use super::*;
use ugaris_core::character_driver::{
    CDR_ARKHATAPRISON, CDR_ARKHATASKELLY, CDR_BOOKEATER, CDR_CENTINEL, CDR_CLANCLERK,
    CDR_CLANMASTER, CDR_NOP,
};
use ugaris_core::world::LegacyHurtOutcome;

fn centinel_npc(character_id: CharacterId) -> Character {
    let mut centinel = login_character(character_id, &login_block("Sentinel"), 1, 190, 200);
    centinel.flags.remove(CharacterFlags::PLAYER);
    centinel.driver = CDR_CENTINEL;
    centinel.hp = POWERSCALE;
    centinel
}

#[test]
fn lethal_centinel_hurt_reports_first_kill_milestone() {
    let mut world = World::default();
    world.add_character(centinel_npc(CharacterId(1)));
    let killer = login_character(CharacterId(2), &login_block("Godmode"), 1, 191, 200);
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
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    let player = runtime.player_for_character(CharacterId(2)).unwrap();
    assert_eq!(player.staffer_centinel_count(), 1);
    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message
            == "You have killed the first sentinel on this floor, kill 29 more!"));
}

#[test]
fn lethal_centinel_hurt_reports_progress_at_ten_and_twenty() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    player.set_staffer_centinel_count(9);
    runtime.players.insert(1, player);
    world.add_character(centinel_npc(CharacterId(1)));
    world.add_character(login_character(
        CharacterId(2),
        &login_block("Godmode"),
        1,
        191,
        200,
    ));

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    let player = runtime.player_for_character(CharacterId(2)).unwrap();
    assert_eq!(player.staffer_centinel_count(), 10);
    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message == "You have killed 10 sentinels, 20 more to go!"));

    let player = runtime.player_for_character_mut(CharacterId(2)).unwrap();
    player.set_staffer_centinel_count(19);
    world.add_character(centinel_npc(CharacterId(3)));
    world.apply_legacy_hurt(
        CharacterId(3),
        Some(CharacterId(2)),
        POWERSCALE * 2,
        1,
        0,
        0,
    );
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());

    let player = runtime.player_for_character(CharacterId(2)).unwrap();
    assert_eq!(player.staffer_centinel_count(), 20);
    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message == "You have killed 20 sentinels, 10 more to go!"));
}

#[test]
fn lethal_centinel_hurt_at_thirty_teleports_killer_and_resets_counter() {
    let mut world = World::default();
    let mut killer = login_character(CharacterId(2), &login_block("Godmode"), 1, 191, 200);
    killer.action = 0;
    world.add_character(centinel_npc(CharacterId(1)));
    world.add_character(killer);

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    player.set_staffer_centinel_count(29);
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

    let player = runtime.player_for_character(CharacterId(2)).unwrap();
    assert_eq!(player.staffer_centinel_count(), 0);
    let killer = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!((killer.x, killer.y), (33, 143));
    let texts = world.drain_pending_system_texts();
    assert!(texts.iter().any(|text| {
        text.message == "Congratulations, you have killed 30 sentinels! Continue your journey."
    }));
}

#[test]
fn centinel_death_handler_ignores_non_matching_driver_and_non_lethal_hits() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    runtime.players.insert(1, player);

    // Non-`CDR_CENTINEL` driver: no counter change even on a lethal hit.
    let mut other_npc = login_character(CharacterId(1), &login_block("Other"), 1, 190, 200);
    other_npc.flags.remove(CharacterFlags::PLAYER);
    other_npc.hp = POWERSCALE;
    world.add_character(other_npc);
    world.add_character(login_character(
        CharacterId(2),
        &login_block("Godmode"),
        1,
        191,
        200,
    ));
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
            .staffer_centinel_count(),
        0
    );

    // Non-lethal hit on a real centinel: no counter change.
    world.add_character(centinel_npc(CharacterId(3)));
    world.apply_legacy_hurt(CharacterId(3), Some(CharacterId(2)), 1, 1, 0, 0);
    apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new());
    assert_eq!(
        runtime
            .player_for_character(CharacterId(2))
            .unwrap()
            .staffer_centinel_count(),
        0
    );
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
fn arkhata_prisoner_death_says_the_secret_line() {
    // C `ch_died_driver`/`CDR_ARKHATAPRISON` -> `prisoner_dead` (`arkhata.
    // c:4490-4492`): a plain unconditional `say`, no `co`/killer checks.
    let mut world = World::default();
    let mut prisoner_npc = login_character(CharacterId(1), &login_block("Prisoner"), 1, 190, 200);
    prisoner_npc.flags.remove(CharacterFlags::PLAYER);
    prisoner_npc.driver = CDR_ARKHATAPRISON;

    let event = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };
    world.add_character(prisoner_npc);

    assert!(apply_arkhata_prisoner_death_from_hurt_event(
        &mut world, event
    ));
    let texts = world.drain_pending_area_texts();
    assert!(texts
        .iter()
        .any(|text| text.message.contains("I know the secret, it's right here!")));
}

#[test]
fn arkhata_prisoner_death_handler_ignores_non_matching_driver_and_non_lethal_hits() {
    let mut world = World::default();
    let mut other_npc = login_character(CharacterId(1), &login_block("Other"), 1, 190, 200);
    other_npc.flags.remove(CharacterFlags::PLAYER);
    world.add_character(other_npc);

    let non_lethal = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: false,
            ..Default::default()
        },
    };
    assert!(!apply_arkhata_prisoner_death_from_hurt_event(
        &mut world, non_lethal
    ));

    let mut world2 = World::default();
    let mut wrong_driver_npc = login_character(CharacterId(1), &login_block("Other"), 1, 190, 200);
    wrong_driver_npc.flags.remove(CharacterFlags::PLAYER);
    wrong_driver_npc.driver = CDR_NOP; // not CDR_ARKHATAPRISON
    world2.add_character(wrong_driver_npc);
    let lethal_wrong_driver = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };
    assert!(!apply_arkhata_prisoner_death_from_hurt_event(
        &mut world2,
        lethal_wrong_driver
    ));
}

#[test]
fn arkhata_bookeater_death_completes_quest_70_when_monk_state_is_19() {
    // C `ch_died_driver`/`CDR_BOOKEATER` -> `bookeater_dead` (`arkhata.c:
    // 4333-4351`): killer must be a player with `monk_state == 19`;
    // completes quest 70 and advances `monk_state` to `20`.
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    player.set_arkhata_monk_state(19);
    runtime.players.insert(1, player);

    let mut bookeater_npc =
        login_character(CharacterId(1), &login_block("The Book Eater"), 37, 10, 10);
    bookeater_npc.flags.remove(CharacterFlags::PLAYER);
    bookeater_npc.driver = CDR_BOOKEATER;
    world.add_character(bookeater_npc);
    world.add_character(login_character(
        CharacterId(2),
        &login_block("Godmode"),
        37,
        10,
        11,
    ));

    let event = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };

    assert!(apply_arkhata_bookeater_death_from_hurt_event(
        &mut runtime,
        &mut world,
        event
    ));

    let player = runtime.player_for_character(CharacterId(2)).unwrap();
    assert_eq!(player.arkhata_monk_state(), 20);
    assert!(player.quest_log.is_done(70));
    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message
            == "Well done, you've solved Tracy's quest. Now report back to her."));
}

#[test]
fn arkhata_bookeater_death_ignores_wrong_monk_state_and_non_player_killer() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    player.set_arkhata_monk_state(5); // not 19
    runtime.players.insert(1, player);

    let mut bookeater_npc =
        login_character(CharacterId(1), &login_block("The Book Eater"), 37, 10, 10);
    bookeater_npc.flags.remove(CharacterFlags::PLAYER);
    bookeater_npc.driver = CDR_BOOKEATER;
    world.add_character(bookeater_npc);
    world.add_character(login_character(
        CharacterId(2),
        &login_block("Godmode"),
        37,
        10,
        11,
    ));

    let event = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };
    assert!(!apply_arkhata_bookeater_death_from_hurt_event(
        &mut runtime,
        &mut world,
        event
    ));

    // Non-lethal hit never dispatches, even with a matching driver/state.
    let player2 = runtime.player_for_character_mut(CharacterId(2)).unwrap();
    player2.set_arkhata_monk_state(19);
    let non_lethal = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: false,
            ..Default::default()
        },
    };
    assert!(!apply_arkhata_bookeater_death_from_hurt_event(
        &mut runtime,
        &mut world,
        non_lethal
    ));
}

#[test]
fn arkhataskelly_death_completes_quest_68_once_all_are_dead() {
    // C `ch_died_driver`/`CDR_ARKHATASKELLY` -> `arkhataskelly_dead`
    // (`arkhata.c:1612-1646`): killer must be a player with
    // `ramin_state == 6`; with no other living skellies left, completes
    // quest 68 and advances `ramin_state` to `7`.
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    player.set_arkhata_ramin_state(6);
    runtime.players.insert(1, player);

    let mut skelly_npc = login_character(CharacterId(1), &login_block("Skeleton"), 37, 10, 10);
    skelly_npc.flags.remove(CharacterFlags::PLAYER);
    skelly_npc.driver = CDR_ARKHATASKELLY;
    world.add_character(skelly_npc);
    world.add_character(login_character(
        CharacterId(2),
        &login_block("Godmode"),
        37,
        10,
        11,
    ));

    let event = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };

    assert!(apply_arkhataskelly_death_from_hurt_event(
        &mut runtime,
        &mut world,
        event
    ));

    let player = runtime.player_for_character(CharacterId(2)).unwrap();
    assert_eq!(player.arkhata_ramin_state(), 7);
    assert!(player.quest_log.is_done(68));
    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message
            == "Well done, you've solved Ramin's quest. Now report back to him."));
}

#[test]
fn arkhataskelly_death_reports_progress_while_others_remain() {
    // Other living `CDR_ARKHATASKELLY` characters (excluding the one that
    // just died) keep the quest open; a progress message is shown only
    // when `undead % 5 == 0 || undead < 10` - 9 remaining skellies (< 10)
    // triggers the message, quest 68 stays undone.
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    player.set_arkhata_ramin_state(6);
    runtime.players.insert(1, player);

    let mut dying_skelly = login_character(CharacterId(1), &login_block("Skeleton"), 37, 10, 10);
    dying_skelly.flags.remove(CharacterFlags::PLAYER);
    dying_skelly.driver = CDR_ARKHATASKELLY;
    world.add_character(dying_skelly);
    for n in 0..9 {
        let mut other = login_character(
            CharacterId(100 + n),
            &login_block("Skeleton"),
            37,
            10 + n as usize,
            10,
        );
        other.flags.remove(CharacterFlags::PLAYER);
        other.driver = CDR_ARKHATASKELLY;
        world.add_character(other);
    }
    world.add_character(login_character(
        CharacterId(2),
        &login_block("Godmode"),
        37,
        10,
        11,
    ));

    let event = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };

    assert!(apply_arkhataskelly_death_from_hurt_event(
        &mut runtime,
        &mut world,
        event
    ));

    let player = runtime.player_for_character(CharacterId(2)).unwrap();
    assert_eq!(player.arkhata_ramin_state(), 6); // unchanged
    assert!(!player.quest_log.is_done(68));
    let texts = world.drain_pending_system_texts();
    assert!(texts
        .iter()
        .any(|text| text.message == "71 down, 9 to go. Beware of respawns!"));
}

#[test]
fn arkhataskelly_death_handler_ignores_wrong_ramin_state_and_non_player_killer() {
    let mut world = World::default();
    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(2));
    player.set_arkhata_ramin_state(3); // not 6
    runtime.players.insert(1, player);

    let mut skelly_npc = login_character(CharacterId(1), &login_block("Skeleton"), 37, 10, 10);
    skelly_npc.flags.remove(CharacterFlags::PLAYER);
    skelly_npc.driver = CDR_ARKHATASKELLY;
    world.add_character(skelly_npc);
    world.add_character(login_character(
        CharacterId(2),
        &login_block("Godmode"),
        37,
        10,
        11,
    ));

    let event = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };
    assert!(!apply_arkhataskelly_death_from_hurt_event(
        &mut runtime,
        &mut world,
        event
    ));

    // Non-matching driver never dispatches either, even with the right
    // ramin_state and a lethal hit.
    let player2 = runtime.player_for_character_mut(CharacterId(2)).unwrap();
    player2.set_arkhata_ramin_state(6);
    let wrong_driver_npc = world.characters.get_mut(&CharacterId(1)).unwrap();
    wrong_driver_npc.driver = CDR_NOP; // not CDR_ARKHATASKELLY
    let wrong_driver_event = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };
    assert!(!apply_arkhataskelly_death_from_hurt_event(
        &mut runtime,
        &mut world,
        wrong_driver_event
    ));

    // Non-lethal hit never dispatches, even with a matching driver/state.
    let right_driver_npc = world.characters.get_mut(&CharacterId(1)).unwrap();
    right_driver_npc.driver = CDR_ARKHATASKELLY;
    let non_lethal = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: false,
            ..Default::default()
        },
    };
    assert!(!apply_arkhataskelly_death_from_hurt_event(
        &mut runtime,
        &mut world,
        non_lethal
    ));
}

#[test]
fn arkhata_nop_death_is_handled_but_sends_no_client_message() {
    // C `ch_died_driver`/`CDR_NOP` -> `immortal_dead` (`arkhata.c:4486-
    // 4488,4657-4659`): same `charlog`-only bug line as `CDR_GATE_WELCOME`
    // above - no client message.
    let mut world = World::default();
    let mut student_npc = login_character(CharacterId(1), &login_block("Student"), 1, 190, 200);
    student_npc.flags.remove(CharacterFlags::PLAYER);
    student_npc.driver = CDR_NOP;
    student_npc.hp = POWERSCALE;
    let killer = login_character(CharacterId(2), &login_block("Godmode"), 1, 191, 200);
    world.add_character(student_npc);
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
fn arkhata_nop_death_handler_ignores_non_matching_driver_and_non_lethal_hits() {
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
    assert!(!apply_arkhata_immortal_death_from_hurt_event(
        &world, non_lethal
    ));

    let mut world2 = World::default();
    let mut wrong_driver_npc = login_character(CharacterId(1), &login_block("Other"), 1, 190, 200);
    wrong_driver_npc.flags.remove(CharacterFlags::PLAYER);
    wrong_driver_npc.driver = CDR_ARKHATAPRISON; // not CDR_NOP
    world2.add_character(wrong_driver_npc);
    let lethal_wrong_driver = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };
    assert!(!apply_arkhata_immortal_death_from_hurt_event(
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

#[test]
fn area30_clan_npc_death_handler_covers_both_drivers() {
    // C `ch_died_driver`'s `CDR_CLANMASTER`/`CDR_CLANCLERK` branches both
    // route to `clanmaster_dead` (`clanmaster.c:1215-1217,1537-1549`), the
    // same `charlog`-only bug line as the other immortal quest NPCs.
    for driver in [CDR_CLANMASTER, CDR_CLANCLERK] {
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
            apply_area30_clan_npc_death_from_hurt_event(&world, lethal),
            "driver {driver} was not covered"
        );
    }
}

#[test]
fn area30_clan_npc_death_handler_ignores_non_matching_driver_and_non_lethal_hits() {
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
    assert!(!apply_area30_clan_npc_death_from_hurt_event(
        &world, non_lethal
    ));

    let mut world2 = World::default();
    let mut clanmaster_npc =
        login_character(CharacterId(1), &login_block("Clanmaster"), 1, 190, 200);
    clanmaster_npc.flags.remove(CharacterFlags::PLAYER);
    clanmaster_npc.driver = CDR_GATE_FIGHT; // not CDR_CLANMASTER/CDR_CLANCLERK
    world2.add_character(clanmaster_npc);

    let lethal_wrong_driver = LegacyHurtEvent {
        target_id: CharacterId(1),
        cause_id: CharacterId(2),
        outcome: LegacyHurtOutcome {
            killed: true,
            ..Default::default()
        },
    };
    assert!(!apply_area30_clan_npc_death_from_hurt_event(
        &world2,
        lethal_wrong_driver
    ));
}
