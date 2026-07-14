use super::*;

#[test]
fn simple_baddy_defaults_match_create_message_initialization() {
    let data = SimpleBaddyDriverData::default();
    assert_eq!(data.aggressive, 0);
    assert_eq!(data.helper, 0);
    assert_eq!(data.startdist, 20);
    assert_eq!(data.chardist, 0);
    assert_eq!(data.stopdist, 40);
    assert_eq!(data.scavenger, 0);
    assert_eq!(data.dir, 3);
    assert_eq!(data.last_hit, 0);
    assert_eq!(data.drink_inventory_potions, 0);
}

#[test]
fn parses_simple_baddy_legacy_arg_string() {
    let parsed = parse_simple_baddy_driver_args(
        " aggressive = 1; helper=2; startdist=12; poisonpower=-4; poisontype=3; poisonchance=25; drinkinvpots=1; unknown=99;",
    );

    assert_eq!(parsed.data.aggressive, 1);
    assert_eq!(parsed.data.helper, 2);
    assert_eq!(parsed.data.startdist, 12);
    assert_eq!(parsed.data.poison_power, -4);
    assert_eq!(parsed.data.poison_type, 3);
    assert_eq!(parsed.data.poison_chance, 25);
    assert_eq!(parsed.data.drink_inventory_potions, 1);
    assert_eq!(
        parsed.unknown,
        vec![UnknownSimpleBaddyArgument {
            name: "unknown".to_string(),
            value: "99".to_string(),
        }]
    );
}

#[test]
fn simple_baddy_arg_parser_stops_like_c_nextnv_on_malformed_pair() {
    let parsed = parse_simple_baddy_driver_args("aggressive=1; broken 7; helper=1;");

    assert_eq!(parsed.data.aggressive, 1);
    assert_eq!(parsed.data.helper, 0);
    assert!(parsed.unknown.is_empty());
}

#[test]
fn simple_baddy_create_initializes_state_and_item_body_flags() {
    let mut character = test_character();
    character.flags.insert(CharacterFlags::NOBODY);
    character.inventory[30] = Some(ItemId(77));
    character.push_driver_message(NT_CREATE, 0, 0, 0);

    let unknown = apply_simple_baddy_create_message(
        &mut character,
        Some("aggressive=1; startdist=9; drinkinvpots=1; unknown=7;"),
        1234,
    );

    assert_eq!(
        unknown,
        vec![UnknownSimpleBaddyArgument {
            name: "unknown".to_string(),
            value: "7".to_string(),
        }]
    );
    assert!(!character.flags.contains(CharacterFlags::NOBODY));
    assert!(character.flags.contains(CharacterFlags::ITEMDEATH));
    assert!(character.driver_messages.is_empty());

    let Some(CharacterDriverState::SimpleBaddy(data)) = character.driver_state else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.aggressive, 1);
    assert_eq!(data.startdist, 9);
    assert_eq!(data.drink_inventory_potions, 1);
    assert_eq!(data.creation_time, 1234);

    // C `fight_driver_set_dist(cn, dat->startdist, dat->chardist,
    // dat->stopdist)` (`simple_baddy.c:189`): the independent
    // `DRD_FIGHTDRIVER` slot gets seeded from the same freshly-parsed
    // distances, not just `simple_baddy`'s own copy.
    let fight_driver = character.fight_driver.expect("fight driver state missing");
    assert_eq!(fight_driver.start_dist, 9);
    assert_eq!(fight_driver.char_dist, 0);
    assert_eq!(fight_driver.stop_dist, 40);
}

#[test]
fn simple_baddy_create_reseeds_fight_driver_distances_without_clearing_enemies() {
    // C `fight_driver_set_dist` only ever writes `start_dist`/
    // `char_dist`/`stop_dist` - a re-creation (e.g. `#reset`-style
    // template reload) must not wipe out already-tracked enemies, home
    // position, or last-hit tick.
    let mut character = test_character();
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    character.fight_driver = Some(FightDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: crate::ids::CharacterId(2),
            priority: 1,
            last_seen_tick: 5,
            visible: true,
            last_x: 11,
            last_y: 12,
        }],
        start_dist: 20,
        stop_dist: 40,
        char_dist: 0,
        home_x: 11,
        home_y: 12,
        last_hit: 7,
    });
    character.push_driver_message(NT_CREATE, 0, 0, 0);

    apply_simple_baddy_create_message(&mut character, Some("startdist=6; stopdist=12;"), 42);

    let fight_driver = character.fight_driver.expect("fight driver state missing");
    assert_eq!(fight_driver.start_dist, 6);
    assert_eq!(fight_driver.stop_dist, 12);
    assert_eq!(fight_driver.char_dist, 0);
    assert_eq!(fight_driver.home_x, 11);
    assert_eq!(fight_driver.home_y, 12);
    assert_eq!(fight_driver.last_hit, 7);
    assert_eq!(fight_driver.enemies.len(), 1);
    assert_eq!(
        fight_driver.enemies[0].target_id,
        crate::ids::CharacterId(2)
    );
}

#[test]
fn lab2_undead_create_parses_legacy_args_and_graveyard_patrol() {
    let mut character = test_character();
    character.push_driver_message(NT_CREATE, 0, 0, 0);

    let unknown = apply_lab2_undead_create_message(
        &mut character,
        Some("aggressive=1; helper=1; patrol=1; undead=1; strange=7;"),
    );

    assert_eq!(
        unknown,
        vec![UnknownSimpleBaddyArgument {
            name: "strange".to_string(),
            value: "7".to_string(),
        }]
    );
    assert!(character.driver_messages.is_empty());
    let Some(CharacterDriverState::Lab2Undead(data)) = character.driver_state else {
        panic!("lab2 undead state missing");
    };
    assert_eq!(data.aggressive, 1);
    assert_eq!(data.helper, 0);
    assert_eq!(data.undead, 1);
    assert_eq!(data.patrol, 1);
    assert_eq!(data.patstep, 4);
    assert_eq!(&data.patx[..4], &[168, 168, 204, 204]);
    assert_eq!(&data.paty[..4], &[178, 218, 218, 178]);
}

#[test]
fn lab2_undead_crypt_patrol_matches_c_coordinate_table() {
    let mut character = test_character();

    apply_lab2_undead_create_message(&mut character, Some("helper=1; patrol=2;"));

    let Some(CharacterDriverState::Lab2Undead(data)) = character.driver_state else {
        panic!("lab2 undead state missing");
    };
    assert_eq!(data.helper, 0);
    assert_eq!(data.patstep, 8);
    assert_eq!(data.patx, [171, 138, 138, 165, 167, 138, 138, 171]);
    assert_eq!(data.paty, [164, 164, 146, 146, 146, 146, 164, 164]);
}

#[test]
fn simple_baddy_gothit_uses_matching_inventory_potions_when_low() {
    let mut character = test_character();
    character.values[1][CharacterValue::Hp as usize] = 20;
    character.values[1][CharacterValue::Mana as usize] = 20;
    character.hp = 9 * POWERSCALE;
    character.mana = 4 * POWERSCALE;
    character.inventory[30] = Some(ItemId(30));
    character.inventory[31] = Some(ItemId(31));
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        drink_inventory_potions: 1,
        ..SimpleBaddyDriverData::default()
    }));
    character.push_driver_message(NT_GOTHIT, 0, 0, 0);

    let outcomes = process_simple_baddy_messages(
        &mut character,
        &[
            test_item(ItemId(30), IDR_POTION, &[0, 1, 0]),
            test_item(ItemId(31), IDR_POTION, &[0, 0, 1]),
        ],
    );

    assert_eq!(
        outcomes,
        vec![
            SimpleBaddyMessageOutcome::UseInventoryPotion {
                item_id: ItemId(30),
                reason: PotionUseReason::LowHp,
            },
            SimpleBaddyMessageOutcome::UseInventoryPotion {
                item_id: ItemId(31),
                reason: PotionUseReason::LowMana,
            },
            SimpleBaddyMessageOutcome::NoteHit,
        ]
    );
    assert!(character.driver_messages.is_empty());
}

#[test]
fn simple_baddy_gothit_ignores_disabled_or_wrong_potions() {
    let mut character = test_character();
    character.values[1][CharacterValue::Hp as usize] = 20;
    character.hp = 9 * POWERSCALE;
    character.inventory[29] = Some(ItemId(29));
    character.inventory[30] = Some(ItemId(30));
    character.inventory[31] = Some(ItemId(31));
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        drink_inventory_potions: 1,
        ..SimpleBaddyDriverData::default()
    }));
    character.push_driver_message(NT_GOTHIT, 0, 0, 0);

    let outcomes = process_simple_baddy_messages(
        &mut character,
        &[
            test_item(ItemId(29), IDR_POTION, &[0, 1, 0]),
            test_item(ItemId(30), 999, &[0, 1, 0]),
            test_item(ItemId(31), IDR_POTION, &[0, 0, 1]),
        ],
    );

    assert_eq!(outcomes, vec![SimpleBaddyMessageOutcome::NoteHit]);
    assert!(character.driver_messages.is_empty());
}

#[test]
fn simple_baddy_char_messages_emit_ordered_helper_bless_candidates() {
    let mut character = test_character();
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        helper: 1,
        ..SimpleBaddyDriverData::default()
    }));
    character.push_driver_message(NT_CHAR, 2, 0, 0);
    character.push_driver_message(NT_CHAR, 3, 0, 0);

    let outcomes = process_simple_baddy_messages(&mut character, &[]);

    assert_eq!(
        outcomes,
        vec![
            SimpleBaddyMessageOutcome::BlessFriend {
                target_id: crate::ids::CharacterId(2),
            },
            SimpleBaddyMessageOutcome::BlessFriend {
                target_id: crate::ids::CharacterId(3),
            },
        ]
    );
    assert!(character.driver_messages.is_empty());
}

#[test]
fn simple_baddy_char_message_ignores_bless_when_helper_disabled() {
    let mut character = test_character();
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    character.push_driver_message(NT_CHAR, 2, 0, 0);

    assert!(process_simple_baddy_messages(&mut character, &[]).is_empty());
    assert!(character.driver_messages.is_empty());
}

#[test]
fn simple_baddy_standard_messages_emit_aggro_outcomes() {
    let mut character = test_character();
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        aggressive: 1,
        helper: 1,
        ..SimpleBaddyDriverData::default()
    }));
    character.push_driver_message(NT_CHAR, 2, 0, 0);
    character.push_driver_message(NT_SEEHIT, 3, 4, 0);
    character.push_driver_message(NT_GOTHIT, 5, 10, 0);

    let outcomes = process_simple_baddy_messages(&mut character, &[]);

    assert_eq!(
        outcomes,
        vec![
            SimpleBaddyMessageOutcome::BlessFriend {
                target_id: crate::ids::CharacterId(2),
            },
            SimpleBaddyMessageOutcome::StandardAggro {
                target_id: crate::ids::CharacterId(2),
                priority: 0,
                require_visible: true,
                hurtme: false,
            },
            SimpleBaddyMessageOutcome::StandardSeenHit {
                attacker_id: crate::ids::CharacterId(3),
                victim_id: crate::ids::CharacterId(4),
            },
            SimpleBaddyMessageOutcome::NoteHit,
            SimpleBaddyMessageOutcome::StandardAggro {
                target_id: crate::ids::CharacterId(5),
                priority: 1,
                require_visible: false,
                hurtme: true,
            },
        ]
    );
    assert!(character.driver_messages.is_empty());
}

#[test]
fn simple_baddy_text_message_preserves_tabunga_notification_boundary() {
    let mut character = test_character();
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    character.push_driver_message(NT_TEXT, 0, 12345, 7);

    let outcomes = process_simple_baddy_messages(&mut character, &[]);

    assert_eq!(
        outcomes,
        vec![SimpleBaddyMessageOutcome::TextNotification {
            speaker_id: crate::ids::CharacterId(7),
            text_token: 12345,
            text: None,
        }]
    );
    assert!(character.driver_messages.is_empty());
}

#[test]
fn simple_baddy_text_message_preserves_optional_text_payload() {
    let mut character = test_character();
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    character.push_driver_text_message(crate::ids::CharacterId(7), "Tabunga please");

    let outcomes = process_simple_baddy_messages(&mut character, &[]);

    assert_eq!(
        outcomes,
        vec![SimpleBaddyMessageOutcome::TextNotification {
            speaker_id: crate::ids::CharacterId(7),
            text_token: 0,
            text: Some("Tabunga please".to_string()),
        }]
    );
    assert!(character.driver_messages.is_empty());
}

#[test]
fn simple_baddy_didhit_emits_poison_hit_outcome() {
    let mut character = test_character();
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        poison_power: 7,
        poison_type: 2,
        poison_chance: 35,
        ..SimpleBaddyDriverData::default()
    }));
    character.push_driver_message(NT_DIDHIT, 42, 3, 0);

    let outcomes = process_simple_baddy_messages(&mut character, &[]);

    assert_eq!(
        outcomes,
        vec![SimpleBaddyMessageOutcome::PoisonHit {
            target_id: crate::ids::CharacterId(42),
            power: 7,
            poison_type: 2,
            chance: 35,
        }]
    );
    assert!(character.driver_messages.is_empty());
}

#[test]
fn simple_baddy_didhit_requires_power_target_and_damage() {
    let mut character = test_character();
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        poison_power: 7,
        poison_type: 2,
        poison_chance: 100,
        ..SimpleBaddyDriverData::default()
    }));
    character.push_driver_message(NT_DIDHIT, 0, 3, 0);
    character.push_driver_message(NT_DIDHIT, 42, 0, 0);

    assert!(process_simple_baddy_messages(&mut character, &[]).is_empty());
    assert!(character.driver_messages.is_empty());
}

#[test]
fn simple_baddy_npc_message_emits_helpid_enemy_outcome() {
    let mut character = test_character();
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        helpid: NTID_GLADIATOR,
        ..SimpleBaddyDriverData::default()
    }));
    character.push_driver_message(NT_NPC, NTID_MERCHANT, 2, 99);
    character.push_driver_message(NT_NPC, NTID_GLADIATOR, 2, 99);

    let outcomes = process_simple_baddy_messages(&mut character, &[]);

    assert_eq!(
        outcomes,
        vec![SimpleBaddyMessageOutcome::AddEnemy {
            caller_id: crate::ids::CharacterId(2),
            target_id: crate::ids::CharacterId(99),
        }]
    );
    assert!(character.driver_messages.is_empty());
}

#[test]
fn simple_baddy_npc_message_preserves_zero_target_like_c() {
    let mut character = test_character();
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        helpid: NTID_GLADIATOR,
        ..SimpleBaddyDriverData::default()
    }));
    character.push_driver_message(NT_NPC, NTID_GLADIATOR, 2, 0);

    let outcomes = process_simple_baddy_messages(&mut character, &[]);

    assert_eq!(
        outcomes,
        vec![SimpleBaddyMessageOutcome::AddEnemy {
            caller_id: crate::ids::CharacterId(2),
            target_id: crate::ids::CharacterId(0),
        }]
    );
    assert!(character.driver_messages.is_empty());
}

#[test]
fn simple_baddy_dead_message_emits_remove_enemy_outcome() {
    let mut character = test_character();
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    character.push_driver_message(NT_DEAD, 42, 7, 0);
    character.push_driver_message(NT_DEAD, 0, 7, 0);

    let outcomes = process_simple_baddy_messages(&mut character, &[]);

    assert_eq!(
        outcomes,
        vec![SimpleBaddyMessageOutcome::RemoveEnemy {
            target_id: crate::ids::CharacterId(42),
        }]
    );
    assert!(character.driver_messages.is_empty());
}

#[test]
fn add_simple_baddy_enemy_requires_same_group_caller_and_updates_existing() {
    let mut character = test_character();
    character.group = 7;
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let mut caller = test_character();
    caller.id = crate::ids::CharacterId(2);
    caller.group = 8;

    assert!(!add_simple_baddy_enemy(
        &mut character,
        &caller,
        crate::ids::CharacterId(99),
        10,
    ));

    caller.group = 7;
    assert!(add_simple_baddy_enemy(
        &mut character,
        &caller,
        crate::ids::CharacterId(99),
        10,
    ));
    assert!(!add_simple_baddy_enemy(
        &mut character,
        &caller,
        crate::ids::CharacterId(99),
        12,
    ));

    let data = character.fight_driver.expect("fight driver state missing");
    assert_eq!(
        data.enemies,
        vec![SimpleBaddyEnemy {
            target_id: crate::ids::CharacterId(99),
            priority: 1,
            last_seen_tick: 12,
            visible: false,
            last_x: 0,
            last_y: 0,
        }]
    );
}

#[test]
fn add_simple_baddy_enemy_keeps_legacy_ten_entry_table() {
    let mut character = test_character();
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));

    for target in 10..22 {
        assert!(add_simple_baddy_enemy_unchecked(
            &mut character,
            crate::ids::CharacterId(target),
            0,
            target as i32,
        ));
    }

    let data = character.fight_driver.expect("fight driver state missing");
    assert_eq!(data.enemies.len(), 10);
    assert_eq!(data.enemies[0].target_id, crate::ids::CharacterId(10));
    assert_eq!(data.enemies[8].target_id, crate::ids::CharacterId(18));
    assert_eq!(data.enemies[9].target_id, crate::ids::CharacterId(21));
}

#[test]
fn add_simple_baddy_enemy_matches_c_slot_nine_overflow_semantics() {
    let mut character = test_character();
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));

    for target in 1..=10 {
        assert!(add_simple_baddy_enemy_unchecked(
            &mut character,
            crate::ids::CharacterId(target),
            0,
            target as i32,
        ));
    }

    assert!(add_simple_baddy_enemy_unchecked(
        &mut character,
        crate::ids::CharacterId(10),
        1,
        99,
    ));

    let data = character.fight_driver.expect("fight driver state missing");
    assert_eq!(data.enemies.len(), 10);
    assert_eq!(data.enemies[9].target_id, crate::ids::CharacterId(10));
    assert_eq!(data.enemies[9].priority, 1);
    assert_eq!(data.enemies[9].last_seen_tick, 99);
}

#[test]
fn add_simple_baddy_enemy_overwrites_priority_like_c_hurtme_flag() {
    let mut character = test_character();
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));

    assert!(add_simple_baddy_enemy_unchecked(
        &mut character,
        crate::ids::CharacterId(2),
        1,
        10,
    ));
    assert!(!add_simple_baddy_enemy_unchecked(
        &mut character,
        crate::ids::CharacterId(2),
        0,
        11,
    ));

    let data = character.fight_driver.expect("fight driver state missing");
    assert_eq!(data.enemies[0].priority, 0);
    assert_eq!(data.enemies[0].last_seen_tick, 11);
}

#[test]
fn remove_simple_baddy_enemy_matches_fight_driver_remove_boundary() {
    let mut character = test_character();
    character.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    character.fight_driver = Some(FightDriverData {
        enemies: vec![
            SimpleBaddyEnemy {
                target_id: crate::ids::CharacterId(2),
                priority: 0,
                last_seen_tick: 10,
                visible: true,
                last_x: 20,
                last_y: 21,
            },
            SimpleBaddyEnemy {
                target_id: crate::ids::CharacterId(3),
                priority: 1,
                last_seen_tick: 11,
                visible: false,
                last_x: 30,
                last_y: 31,
            },
        ],
        ..FightDriverData::default()
    });

    assert!(remove_simple_baddy_enemy(
        &mut character,
        crate::ids::CharacterId(2),
    ));
    assert!(!remove_simple_baddy_enemy(
        &mut character,
        crate::ids::CharacterId(99),
    ));

    let data = character.fight_driver.expect("fight driver state missing");
    assert_eq!(data.enemies.len(), 1);
    assert_eq!(data.enemies[0].target_id, crate::ids::CharacterId(3));
}

#[test]
fn remove_simple_baddy_enemy_ignores_missing_fight_driver_data() {
    // No `driver_state` gate anymore (matches C's driver-independent
    // `DRD_FIGHTDRIVER` slot) - this now only exercises the "no
    // `fight_driver` data at all yet" early return.
    let mut character = test_character();

    assert!(!remove_simple_baddy_enemy(
        &mut character,
        crate::ids::CharacterId(2),
    ));
}

#[test]
fn add_and_remove_simple_baddy_enemy_work_without_simple_baddy_driver_state() {
    // C `fight_driver_add_enemy`/`fight_driver_remove_enemy` operate on
    // any character's independent `DRD_FIGHTDRIVER` slot - a
    // `CDR_LOSTCON` lingering character (or, eventually, a normal
    // playing character) has no `SimpleBaddyDriverData` at all.
    let mut character = test_character();
    character.driver_state = Some(CharacterDriverState::Lostcon(LostconDriverData {
        deadline: 0,
    }));

    assert!(add_simple_baddy_enemy_unchecked(
        &mut character,
        crate::ids::CharacterId(2),
        1,
        10,
    ));
    assert_eq!(character.fight_driver.as_ref().unwrap().enemies.len(), 1);

    assert!(remove_simple_baddy_enemy(
        &mut character,
        crate::ids::CharacterId(2),
    ));
    assert!(character.fight_driver.unwrap().enemies.is_empty());
}
