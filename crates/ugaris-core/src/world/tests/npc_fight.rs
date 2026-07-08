use super::*;

#[test]
fn simple_baddy_text_tabunga_emits_god_diagnostic_area_text() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.name = "Ratling".to_string();
    npc.level = 12;
    npc.hp = 7 * POWERSCALE;
    npc.mana = 3 * POWERSCALE;
    npc.endurance = 4 * POWERSCALE;
    npc.lifeshield = 2 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 10;
    npc.values[1][CharacterValue::Hp as usize] = 9;
    npc.values[0][CharacterValue::Wisdom as usize] = 5;
    npc.values[1][CharacterValue::Wisdom as usize] = 6;
    npc.professions[profession::DEMON] = 4;
    npc.flags |= CharacterFlags::ALIVE;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    npc.push_driver_text_message(CharacterId(2), "tabunga");
    assert!(world.spawn_character(npc, 10, 10));

    let mut god = character(2);
    god.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god, 11, 10));

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    let texts = world.drain_pending_area_texts();
    assert!(texts.iter().any(|text| text.message == "Ratling (12):"));
    assert!(texts
        .iter()
        .any(|text| text.message == "HP:          9/ 10 (7)"));
    assert!(texts
        .iter()
        .any(|text| text.message == "Wisdom:      6/  5"));
    assert!(texts.iter().any(|text| text.message == "P_DEMON:     4"));
    assert!(texts
        .iter()
        .all(|text| text.x == 10 && text.y == 10 && text.max_distance == SAY_DIST as u16));
}

#[test]
fn simple_baddy_text_tabunga_requires_nearby_god_and_keyword() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    npc.push_driver_text_message(CharacterId(2), "hello");
    npc.push_driver_text_message(CharacterId(3), "tabunga");
    npc.push_driver_text_message(CharacterId(4), "tabunga");
    assert!(world.spawn_character(npc, 10, 10));
    let mut god_far = character(3);
    god_far.flags |= CharacterFlags::GOD;
    assert!(world.spawn_character(god_far, 20, 20));
    assert!(world.spawn_character(character(4), 11, 10));

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop; 3]);
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn fight_driver_task_order_sorts_by_descending_legacy_value() {
    let all_legacy_task_kinds = [
        FightDriverTaskKind::Freeze,
        FightDriverTaskKind::Fireball,
        FightDriverTaskKind::Ball,
        FightDriverTaskKind::Flash,
        FightDriverTaskKind::Warcry,
        FightDriverTaskKind::Attack,
        FightDriverTaskKind::MoveRight,
        FightDriverTaskKind::MoveLeft,
        FightDriverTaskKind::MoveUp,
        FightDriverTaskKind::MoveDown,
        FightDriverTaskKind::Regenerate,
        FightDriverTaskKind::Distance3,
        FightDriverTaskKind::Distance7,
        FightDriverTaskKind::Bless,
        FightDriverTaskKind::EarthRain,
        FightDriverTaskKind::EarthMud,
        FightDriverTaskKind::Heal,
        FightDriverTaskKind::MagicShield,
        FightDriverTaskKind::Pulse,
        FightDriverTaskKind::AttackBack,
        FightDriverTaskKind::Flee,
        FightDriverTaskKind::FireRing,
    ];
    assert_eq!(all_legacy_task_kinds.len(), 22);

    let mut tasks = [
        FightDriverTask {
            kind: FightDriverTaskKind::Attack,
            value: FIGHT_DRIVER_LOW_PRIO + 20,
        },
        FightDriverTask {
            kind: FightDriverTaskKind::Fireball,
            value: FIGHT_DRIVER_MED_PRIO + 5,
        },
        FightDriverTask {
            kind: FightDriverTaskKind::Heal,
            value: FIGHT_DRIVER_HIGH_PRIO + 1,
        },
    ];

    order_fight_driver_tasks(&mut tasks, -10, |_| {
        unreachable!("no silliness at level -10")
    });

    assert_eq!(
        tasks.iter().map(|task| task.kind).collect::<Vec<_>>(),
        vec![
            FightDriverTaskKind::Heal,
            FightDriverTaskKind::Fireball,
            FightDriverTaskKind::Attack,
        ]
    );
}

#[test]
fn fight_driver_task_order_adds_c_silliness_rolls_before_sorting() {
    let mut tasks = [
        FightDriverTask {
            kind: FightDriverTaskKind::Attack,
            value: 100,
        },
        FightDriverTask {
            kind: FightDriverTaskKind::Flash,
            value: 103,
        },
    ];
    let mut rolls = [4, 0].into_iter();

    order_fight_driver_tasks(&mut tasks, 0, |below| {
        assert_eq!(below, 5);
        rolls.next().unwrap()
    });

    assert_eq!(tasks[0].kind, FightDriverTaskKind::Attack);
    assert_eq!(tasks[0].value, 104);
    assert_eq!(tasks[1].kind, FightDriverTaskKind::Flash);
    assert_eq!(tasks[1].value, 103);
}

#[test]
fn fight_driver_attackback_requires_attack_as_next_task_like_c() {
    let tasks = [
        FightDriverTask {
            kind: FightDriverTaskKind::AttackBack,
            value: FIGHT_DRIVER_HIGH_PRIO,
        },
        FightDriverTask {
            kind: FightDriverTaskKind::Fireball,
            value: FIGHT_DRIVER_MED_PRIO,
        },
        FightDriverTask {
            kind: FightDriverTaskKind::Attack,
            value: FIGHT_DRIVER_LOW_PRIO,
        },
    ];

    assert!(!fight_driver_attackback_may_run(&tasks, 0));
    assert!(!fight_driver_attackback_may_run(&tasks, 2));

    let tasks = [
        FightDriverTask {
            kind: FightDriverTaskKind::AttackBack,
            value: FIGHT_DRIVER_HIGH_PRIO,
        },
        FightDriverTask {
            kind: FightDriverTaskKind::Attack,
            value: FIGHT_DRIVER_MED_PRIO,
        },
    ];

    assert!(fight_driver_attackback_may_run(&tasks, 0));
}

#[test]
fn simple_baddy_fight_tasks_honor_legacy_nomove_attack_gate() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 13, 10);

    let moving_tasks = world.simple_baddy_fight_tasks(
        CharacterId(1),
        world.characters.get(&CharacterId(2)).unwrap(),
        1,
        false,
    );
    let no_move_tasks = world.simple_baddy_fight_tasks(
        CharacterId(1),
        world.characters.get(&CharacterId(2)).unwrap(),
        1,
        true,
    );

    assert!(moving_tasks
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Attack));
    assert!(!no_move_tasks
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Attack));
}

#[test]
fn simple_baddy_fight_tasks_allow_nomove_attack_at_distance_two_like_c() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);

    let tasks = world.simple_baddy_fight_tasks(
        CharacterId(1),
        world.characters.get(&CharacterId(2)).unwrap(),
        1,
        true,
    );

    assert!(tasks
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Attack));
}

#[test]
fn simple_baddy_fight_tasks_suppress_movement_spacing_when_nomove_like_c() {
    let mut world = World::default();
    world.tick = Tick(100);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let mut target = character(2);
    target.dir = Direction::Right as u8;
    target.action = action::IDLE;
    target.regen_ticker = 0;
    let blocker = character(3);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 12, 10);
    world.spawn_character(blocker, 13, 10);

    let target = world.characters.get(&CharacterId(2)).unwrap();
    let moving_tasks = world.simple_baddy_fight_tasks(CharacterId(1), target, 1, false);
    let no_move_tasks = world.simple_baddy_fight_tasks(CharacterId(1), target, 1, true);

    assert!(moving_tasks
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::AttackBack));
    assert!(!no_move_tasks.iter().any(|task| matches!(
        task.kind,
        FightDriverTaskKind::Distance3
            | FightDriverTaskKind::Distance7
            | FightDriverTaskKind::AttackBack
    )));
}

// C `fight_driver_attack_enemy`'s 9 remaining positional `no*` suppression
// arguments (`src/system/drvlib.c:1682`, beyond the already-tested
// `nomove`): each gates exactly one task's inclusion in the scored task
// list. Not wired to any live caller yet (the NPC driver always passes
// all-`false`; the player/lostcon caller is a separate follow-up task -
// see `PORTING_TODO.md`), but the generalized `FightDriverSuppressions`
// engine this exercises is what that follow-up will plug into.

#[test]
fn simple_baddy_fight_tasks_honor_legacy_nofreeze_gate() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FREEZE_COST;
    npc.values[0][CharacterValue::Freeze as usize] = 20;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 12, 10);

    let target = world.characters.get(&CharacterId(2)).unwrap();
    let allowed = world.simple_baddy_fight_tasks(
        CharacterId(1),
        target,
        1,
        FightDriverSuppressions::default(),
    );
    let suppressed = world.simple_baddy_fight_tasks(
        CharacterId(1),
        target,
        1,
        FightDriverSuppressions {
            nofreeze: true,
            ..FightDriverSuppressions::default()
        },
    );

    assert!(allowed
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Freeze));
    assert!(!suppressed
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Freeze));
}

#[test]
fn simple_baddy_freeze_modifier_uses_ice_demons_base_not_current_value_like_c() {
    // C: freeze_value (tool.c:2166-2192) reads the caster's CF_IDEMON bonus
    // from `ch[cn].value[1][V_DEMON]` (the base/"present" value), not
    // `value[0]` (the current value, which update_char caps at
    // min(current, present) and which sunlight/combat can reduce). Two
    // otherwise-identical ice demons whose current V_DEMON differs from
    // their base V_DEMON must still see the same freeze modifier, matching
    // the base value.
    let mut world = World::default();
    let mut npc = character(1);
    npc.flags.insert(CharacterFlags::IDEMON);
    // Current (value[0]) V_DEMON is much lower than base (value[1]) -
    // simulating a demon whose current value has been reduced (e.g. by
    // update_char's cap or the earth-demon power-level mechanic).
    npc.values[0][CharacterValue::Demon as usize] = 0;
    npc.values[1][CharacterValue::Demon as usize] = 30;

    let mut target = character(2);
    target.values[0][CharacterValue::Cold as usize] = 40;

    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 12, 10);

    let attacker = world.characters.get(&CharacterId(1)).unwrap().clone();
    let target = world.characters.get(&CharacterId(2)).unwrap();

    let modifier = world.simple_baddy_freeze_modifier(&attacker, target);

    // C: str += (40 - 30) * 10 = +100 bonus term, using value[1]=30. Using
    // value[0]=0 instead would have produced a much larger (+400) bonus.
    let base = -(200 + 0 * 11 - 0 * 11);
    assert_eq!(modifier, base + (40 - 30) * 10);
}

#[test]
fn simple_baddy_fight_tasks_honor_legacy_noheal_gate() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = 4 * POWERSCALE;
    npc.hp = POWERSCALE;
    npc.values[0][CharacterValue::Heal as usize] = 10;
    npc.values[0][CharacterValue::Hp as usize] = 10;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 20, 20);

    let target = world.characters.get(&CharacterId(2)).unwrap();
    let allowed = world.simple_baddy_fight_tasks(
        CharacterId(1),
        target,
        1,
        FightDriverSuppressions::default(),
    );
    let suppressed = world.simple_baddy_fight_tasks(
        CharacterId(1),
        target,
        1,
        FightDriverSuppressions {
            noheal: true,
            ..FightDriverSuppressions::default()
        },
    );

    assert!(allowed
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Heal));
    assert!(!suppressed
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Heal));
}

#[test]
fn simple_baddy_fight_tasks_honor_legacy_noshield_gate() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = 4 * POWERSCALE;
    npc.lifeshield = 0;
    npc.values[0][CharacterValue::MagicShield as usize] = 20;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 20, 20);

    let target = world.characters.get(&CharacterId(2)).unwrap();
    let allowed = world.simple_baddy_fight_tasks(
        CharacterId(1),
        target,
        1,
        FightDriverSuppressions::default(),
    );
    let suppressed = world.simple_baddy_fight_tasks(
        CharacterId(1),
        target,
        1,
        FightDriverSuppressions {
            noshield: true,
            ..FightDriverSuppressions::default()
        },
    );

    assert!(allowed
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::MagicShield));
    assert!(!suppressed
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::MagicShield));
}

#[test]
fn simple_baddy_fight_tasks_honor_legacy_nobless_gate() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = BLESS_COST;
    npc.values[0][CharacterValue::Bless as usize] = 20;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 20, 20);

    let target = world.characters.get(&CharacterId(2)).unwrap();
    let allowed = world.simple_baddy_fight_tasks(
        CharacterId(1),
        target,
        1,
        FightDriverSuppressions::default(),
    );
    let suppressed = world.simple_baddy_fight_tasks(
        CharacterId(1),
        target,
        1,
        FightDriverSuppressions {
            nobless: true,
            ..FightDriverSuppressions::default()
        },
    );

    assert!(allowed
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Bless));
    assert!(!suppressed
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Bless));
}

#[test]
fn simple_baddy_fight_tasks_honor_legacy_nofireball_gate() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FIREBALL_COST;
    npc.values[0][CharacterValue::Fireball as usize] = 20;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 14, 10);
    world.map.tile_mut(14, 10).unwrap().light = 255;

    let target = world.characters.get(&CharacterId(2)).unwrap();
    let allowed = world.simple_baddy_fight_tasks(
        CharacterId(1),
        target,
        1,
        FightDriverSuppressions::default(),
    );
    let suppressed = world.simple_baddy_fight_tasks(
        CharacterId(1),
        target,
        1,
        FightDriverSuppressions {
            nofireball: true,
            ..FightDriverSuppressions::default()
        },
    );

    assert!(allowed
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Fireball));
    assert!(!suppressed.iter().any(|task| matches!(
        task.kind,
        FightDriverTaskKind::Fireball | FightDriverTaskKind::FireRing
    )));
}

#[test]
fn simple_baddy_fight_tasks_honor_legacy_noball_gate() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FLASH_COST;
    npc.values[0][CharacterValue::Flash as usize] = 20;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 16, 10);
    world.map.tile_mut(16, 10).unwrap().light = 255;

    let target = world.characters.get(&CharacterId(2)).unwrap();
    let allowed = world.simple_baddy_fight_tasks(
        CharacterId(1),
        target,
        1,
        FightDriverSuppressions::default(),
    );
    let suppressed = world.simple_baddy_fight_tasks(
        CharacterId(1),
        target,
        1,
        FightDriverSuppressions {
            noball: true,
            ..FightDriverSuppressions::default()
        },
    );

    assert!(allowed
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Ball));
    assert!(!suppressed
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Ball));
}

#[test]
fn simple_baddy_fight_tasks_honor_legacy_noflash_gate() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FLASH_COST;
    npc.values[0][CharacterValue::Flash as usize] = 20;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 12, 10);
    world.map.tile_mut(12, 10).unwrap().light = 255;

    let target = world.characters.get(&CharacterId(2)).unwrap();
    let allowed = world.simple_baddy_fight_tasks(
        CharacterId(1),
        target,
        1,
        FightDriverSuppressions::default(),
    );
    let suppressed = world.simple_baddy_fight_tasks(
        CharacterId(1),
        target,
        1,
        FightDriverSuppressions {
            noflash: true,
            ..FightDriverSuppressions::default()
        },
    );

    assert!(allowed
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Flash));
    assert!(!suppressed
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Flash));
}

#[test]
fn simple_baddy_fight_tasks_honor_legacy_nowarcry_gate() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.endurance = 10 * POWERSCALE;
    npc.values[0][CharacterValue::Warcry as usize] = 20;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 12, 10);
    world.map.tile_mut(12, 10).unwrap().light = 255;

    let target = world.characters.get(&CharacterId(2)).unwrap();
    let allowed = world.simple_baddy_fight_tasks(
        CharacterId(1),
        target,
        1,
        FightDriverSuppressions::default(),
    );
    let suppressed = world.simple_baddy_fight_tasks(
        CharacterId(1),
        target,
        1,
        FightDriverSuppressions {
            nowarcry: true,
            ..FightDriverSuppressions::default()
        },
    );

    assert!(allowed
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Warcry));
    assert!(!suppressed
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Warcry));
}

#[test]
fn simple_baddy_fight_tasks_honor_legacy_nopulse_gate() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = POWERSCALE + 1;
    npc.values[0][CharacterValue::Mana as usize] = 1;
    npc.values[0][CharacterValue::Pulse as usize] = 2_000;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let mut target = character(2);
    target.flags.insert(CharacterFlags::ALIVE);
    target.hp = POWERSCALE + 100;
    target.lifeshield = 0;
    target.values[0][CharacterValue::Hp as usize] = 100;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 12, 10);
    world.map.tile_mut(12, 10).unwrap().light = 255;

    let target = world.characters.get(&CharacterId(2)).unwrap();
    let allowed = world.simple_baddy_fight_tasks(
        CharacterId(1),
        target,
        1,
        FightDriverSuppressions::default(),
    );
    let suppressed = world.simple_baddy_fight_tasks(
        CharacterId(1),
        target,
        1,
        FightDriverSuppressions {
            nopulse: true,
            ..FightDriverSuppressions::default()
        },
    );

    assert!(allowed
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Pulse));
    assert!(!suppressed
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Pulse));
}

#[test]
fn simple_baddy_fight_tasks_bool_nomove_still_converts_via_into() {
    // Every call site that predates `FightDriverSuppressions` passed a bare
    // `bool` for `nomove`; `From<bool>` keeps them compiling unchanged.
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 13, 10);

    let target = world.characters.get(&CharacterId(2)).unwrap();
    let via_bool = world.simple_baddy_fight_tasks(CharacterId(1), target, 1, true);
    let via_struct = world.simple_baddy_fight_tasks(
        CharacterId(1),
        target,
        1,
        FightDriverSuppressions {
            nomove: true,
            ..FightDriverSuppressions::default()
        },
    );

    assert_eq!(via_bool, via_struct);
}

#[test]
fn simple_baddy_attack_action_self_heals_before_offense_when_badly_hurt() {
    let mut world = World::default();
    world.tick = Tick(450);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.regen_ticker = 450;
    npc.hp = 40 * POWERSCALE;
    npc.mana = 10 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 100;
    npc.values[0][CharacterValue::Mana as usize] = 10;
    npc.values[0][CharacterValue::Heal as usize] = 20;
    npc.values[0][CharacterValue::Fireball as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 15,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 15, 10);
    world.map.tile_mut(15, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::HEAL_SELF);
    assert!(npc.mana < 10 * POWERSCALE);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 450);
}

#[test]
fn simple_baddy_visible_attack_queues_legacy_start_combat_sound_after_delay() {
    let mut world = World::default();
    world.tick = Tick(TICKS_PER_SECOND * 11);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        lastfight: 0,
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 11,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let mut target = character(2);
    target.flags.insert(CharacterFlags::PLAYER);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let sounds = world.drain_pending_sound_specials();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].character_id, CharacterId(2));
    assert_eq!(sounds[0].special.special_type, 1);

    let mut world = World::default();
    world.tick = Tick(TICKS_PER_SECOND * 11);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        lastfight: (TICKS_PER_SECOND * 11 - 1) as i32,
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 11,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let mut target = character(2);
    target.flags.insert(CharacterFlags::PLAYER);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    assert!(world.drain_pending_sound_specials().is_empty());
}

#[test]
fn simple_baddy_attack_action_restores_magicshield_before_melee() {
    let mut world = World::default();
    world.tick = Tick(451);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = 10 * POWERSCALE;
    npc.lifeshield = 0;
    npc.values[0][CharacterValue::MagicShield as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 11,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::MAGICSHIELD);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 451);
}

#[test]
fn simple_baddy_attack_action_self_blesses_when_unblessed() {
    let mut world = World::default();
    world.tick = Tick(452);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = BLESS_COST;
    npc.values[0][CharacterValue::Bless as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 11,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::BLESS_SELF);
    assert_eq!(npc.mana, 0);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 452);
}

#[test]
fn simple_baddy_attack_action_earth_demon_casts_useful_earthmud() {
    let mut world = World::default();
    world.tick = Tick(454);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.flags.insert(CharacterFlags::EDEMON);
    npc.hp = 100 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 100;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.values[1][CharacterValue::Demon as usize] = 30;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 15,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let mut target = character(2);
    target.action = action::WALK;
    target.tox = 16;
    target.toy = 10;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 15, 10);
    world.map.tile_mut(15, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::EARTHMUD);
    assert_eq!(npc.act1, 17 + 10 * MAX_MAP as i32);
    assert_eq!(npc.act2, 30);
    assert_eq!(npc.hp, 100 * POWERSCALE - 3000);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 454);
}

#[test]
fn simple_baddy_attack_action_skips_earthmud_without_useful_tiles() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.flags.insert(CharacterFlags::EDEMON);
    npc.hp = 100 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 100;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.values[1][CharacterValue::Demon as usize] = 30;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 15,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 15, 10);
    for (x, y) in [(15, 10), (16, 10), (14, 10), (15, 11), (15, 9)] {
        world.map.set_flags(x, y, MapFlags::SIGHTBLOCK);
    }

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_ne!(npc.action, action::EARTHMUD);
}

#[test]
fn simple_baddy_fight_tasks_keep_c_commented_earthrain_disabled() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.flags.insert(CharacterFlags::EDEMON);
    npc.hp = 100 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 100;
    npc.values[1][CharacterValue::Demon as usize] = 30;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 15,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 15, 10);
    world.map.tile_mut(15, 10).unwrap().light = 255;

    let tasks = world.simple_baddy_fight_tasks(
        CharacterId(1),
        world.characters.get(&CharacterId(2)).unwrap(),
        1,
        false,
    );

    assert!(!tasks
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::EarthRain));
    assert!(tasks
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::EarthMud));
}

#[test]
fn simple_baddy_fight_tasks_add_c_low_hp_flee_branch() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.hp = POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 10;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);

    let tasks = world.simple_baddy_fight_tasks(
        CharacterId(1),
        world.characters.get(&CharacterId(2)).unwrap(),
        1,
        false,
    );

    assert!(tasks
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Flee));
}

#[test]
fn simple_baddy_attack_action_can_choose_low_hp_flee() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.hp = POWERSCALE;
    npc.endurance = 10 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 10;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 11,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);

    assert!(world.process_simple_baddy_attack_action_with_random(CharacterId(1), 1, |_| 0));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!(npc.speed_mode, SpeedMode::Fast);
    assert_ne!(npc.dir, Direction::Right as u8);
}

#[test]
fn simple_baddy_firering_helper_respects_active_spell_blocker() {
    let mut world = World::default();
    world.tick = Tick(456);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FIREBALL_COST;
    npc.values[0][CharacterValue::Fireball as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let mut blocker = item(20, ItemFlags::empty());
    blocker.driver = IDR_FIRERING;
    npc.inventory[SPELL_SLOT_START] = Some(blocker.id);
    let target = character(2);
    world.items.insert(blocker.id, blocker);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target.clone(), 11, 10);

    assert!(!world.setup_simple_baddy_firering_attack(CharacterId(1), &target));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, 0);
    assert_eq!(npc.mana, FIREBALL_COST);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 0);
}

#[test]
fn simple_baddy_fireball_repositions_for_blocked_line_of_fire() {
    let mut world = World::default();
    world.tick = Tick(467);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FIREBALL_COST;
    npc.values[0][CharacterValue::Fireball as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 14,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 14, 10);
    world
        .map
        .tile_mut(12, 10)
        .unwrap()
        .flags
        .insert(MapFlags::MOVEBLOCK);
    world.map.tile_mut(14, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!(npc.tox, 10);
    assert_eq!(npc.toy, 11);
    assert_eq!(npc.mana, FIREBALL_COST);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 467);
}

#[test]
fn simple_baddy_fireball_does_not_cast_through_blocked_line_without_lane() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FIREBALL_COST;
    npc.values[0][CharacterValue::Fireball as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 14,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 14, 10);
    for (x, y) in [(12, 10), (10, 9), (10, 11), (11, 10), (9, 10)] {
        world
            .map
            .tile_mut(x, y)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);
    }
    world.map.tile_mut(14, 10).unwrap().light = 255;

    let target = world.characters[&CharacterId(2)].clone();
    assert!(!world.setup_simple_baddy_fireball_attack(CharacterId(1), &target, 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, 0);
    assert_eq!(npc.mana, FIREBALL_COST);
}

#[test]
fn simple_baddy_fireball_line_rejects_friendly_blast() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 15,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    world.spawn_character(npc, 10, 10);
    world.spawn_character(character(2), 15, 10);
    world.spawn_character(character(3), 12, 11);
    world
        .map
        .tile_mut(12, 10)
        .unwrap()
        .flags
        .insert(MapFlags::MOVEBLOCK);

    assert!(!world.fireball_line_hits_target(CharacterId(1), CharacterId(2), 10, 10, 15, 10));
}

#[test]
fn simple_baddy_attack_action_applies_legacy_task_silliness_rolls() {
    let mut world = World::default();
    world.tick = Tick(459);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FLASH_COST;
    npc.values[0][CharacterValue::Attack as usize] = 100;
    npc.values[1][CharacterValue::Attack as usize] = 100;
    npc.values[0][CharacterValue::Flash as usize] = 26;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 11,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);
    world.map.tile_mut(11, 10).unwrap().light = 255;
    let mut rolls = [0, 4].into_iter();

    assert!(
        world.process_simple_baddy_attack_action_with_random(CharacterId(1), 1, |below| {
            assert_eq!(below, 5);
            rolls.next().unwrap_or(0)
        })
    );

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::ATTACK1);
    assert_eq!(npc.mana, FLASH_COST);
}

#[test]
fn simple_baddy_attack_task_uses_c_attack_skill_with_weapon_skill() {
    let mut character = character(1);
    character.level = 20;
    character.values[0][CharacterValue::Attack as usize] = 30;
    character.values[1][CharacterValue::Attack as usize] = 30;
    character.values[0][CharacterValue::Tactics as usize] = 12;
    character.values[0][CharacterValue::Hand as usize] = 5;
    character.values[0][CharacterValue::Sword as usize] = 40;
    character.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(7));
    let weapon = item(7, ItemFlags::SWORD);
    let items = HashMap::from([(weapon.id, weapon)]);

    assert_eq!(simple_baddy_attack_skill(&character, &items), 104);
    assert_eq!(simple_baddy_attack_task_value(&character, &items), 539);
}

#[test]
fn simple_baddy_attack_task_falls_back_to_hand_without_weapon() {
    let mut character = character(1);
    character.level = 20;
    character.values[0][CharacterValue::Hand as usize] = 9;
    character.values[0][CharacterValue::Bless as usize] = 8;
    character.values[0][CharacterValue::Heal as usize] = 8;
    character.values[0][CharacterValue::Freeze as usize] = 8;
    character.values[0][CharacterValue::MagicShield as usize] = 8;
    character.values[0][CharacterValue::Flash as usize] = 8;
    character.values[0][CharacterValue::Fireball as usize] = 8;
    character.values[0][CharacterValue::Pulse as usize] = 8;

    let items = HashMap::new();

    assert_eq!(simple_baddy_attack_skill(&character, &items), 3);
    assert_eq!(simple_baddy_attack_task_value(&character, &items), 2);
}

#[test]
fn simple_baddy_attack_action_uses_warcry_when_close_and_unshielded() {
    let mut world = World::default();
    world.tick = Tick(460);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.endurance = 10 * POWERSCALE;
    npc.values[0][CharacterValue::Warcry as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 12,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 12, 10);
    world.map.tile_mut(12, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WARCRY);
    assert_eq!(npc.endurance, 10 * POWERSCALE - 20 * POWERSCALE / 3);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 460);
}

#[test]
fn simple_baddy_warcry_task_does_not_precheck_modifier_like_c() {
    let mut world = World::default();
    world.tick = Tick(460);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.endurance = 10 * POWERSCALE;
    npc.lifeshield = 10 * POWERSCALE;
    npc.values[0][CharacterValue::Warcry as usize] = 2;
    npc.values[0][CharacterValue::MagicShield as usize] = 10;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.values[1][CharacterValue::MagicShield as usize] = 10;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 12,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let mut target = character(2);
    target.values[0][CharacterValue::Immunity as usize] = 100;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 12, 10);
    world.map.tile_mut(12, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WARCRY);
    assert_eq!(npc.endurance, 10 * POWERSCALE - 2 * POWERSCALE / 3);
}

#[test]
fn simple_baddy_warcry_task_requires_more_than_exact_endurance_cost_like_c() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Warcry as usize] = 9;
    npc.values[0][CharacterValue::MagicShield as usize] = 10;
    npc.values[1][CharacterValue::MagicShield as usize] = 10;
    npc.lifeshield = 0;
    npc.endurance = 9 * POWERSCALE / 3;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 12, 10);

    let exact_cost_tasks = world.simple_baddy_fight_tasks(
        CharacterId(1),
        world.characters.get(&CharacterId(2)).unwrap(),
        1,
        false,
    );

    assert!(!exact_cost_tasks
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Warcry));

    world.characters.get_mut(&CharacterId(1)).unwrap().endurance += 1;
    let above_cost_tasks = world.simple_baddy_fight_tasks(
        CharacterId(1),
        world.characters.get(&CharacterId(2)).unwrap(),
        1,
        false,
    );

    assert!(above_cost_tasks
        .iter()
        .any(|task| task.kind == FightDriverTaskKind::Warcry));
}

#[test]
fn simple_baddy_ball_task_requires_unblocked_legacy_intercept_steps() {
    let mut world = World::default();
    world.tick = Tick(461);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FLASH_COST;
    npc.values[0][CharacterValue::Flash as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 16,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 16, 10);
    world.map.tile_mut(16, 10).unwrap().light = 255;
    world
        .map
        .tile_mut(12, 10)
        .unwrap()
        .flags
        .insert(MapFlags::MOVEBLOCK);

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_ne!(npc.action, action::BALL1);
}

#[test]
fn simple_baddy_ball_attack_uses_legacy_random_target_offset() {
    let mut world = World::default();
    world.tick = Tick(461);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FLASH_COST;
    npc.values[0][CharacterValue::Flash as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 16,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 16, 10);
    world.map.tile_mut(16, 10).unwrap().light = 255;
    let mut rolls = [0, 0, 0, 2].into_iter();

    assert!(
        world.process_simple_baddy_attack_action_with_random(CharacterId(1), 1, |_| {
            rolls.next().unwrap()
        })
    );

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::BALL1);
    assert_eq!(npc.act1, 15);
    assert_eq!(npc.act2, 11);
}

#[test]
fn simple_baddy_attack_batch_threads_runtime_random() {
    let mut world = World::default();
    world.tick = Tick(461);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FLASH_COST;
    npc.values[0][CharacterValue::Flash as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 16,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 16, 10);
    world.map.tile_mut(16, 10).unwrap().light = 255;
    let mut rolls = [0, 0, 0, 2].into_iter();

    assert_eq!(
        world.process_simple_baddy_attack_actions_with_random(1, |_| rolls.next().unwrap()),
        1
    );

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::BALL1);
    assert_eq!(npc.act1, 15);
    assert_eq!(npc.act2, 11);
}

#[test]
fn simple_baddy_default_attack_action_consumes_world_rng_seed() {
    let mut world = World::default();
    world.tick = Tick(461);
    world.legacy_random_seed = 7;
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FLASH_COST;
    npc.values[0][CharacterValue::Flash as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 16,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 16, 10);
    world.map.tile_mut(16, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    assert_ne!(world.legacy_random_seed, 7);
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::BALL1);
}

#[test]
fn simple_baddy_attack_action_does_not_pulse_healthy_targets() {
    let mut world = World::default();
    world.tick = Tick(463);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = 100 * POWERSCALE;
    npc.values[0][CharacterValue::Mana as usize] = 100;
    npc.values[0][CharacterValue::Pulse as usize] = 200;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 11,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let mut target = character(2);
    target.hp = 100 * POWERSCALE;
    target.values[0][CharacterValue::Hp as usize] = 100;
    target.values[0][CharacterValue::Attack as usize] = 1;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::ATTACK1);
    assert_eq!(npc.mana, 100 * POWERSCALE);
}

#[test]
fn simple_baddy_attack_action_idles_when_already_at_flash_spacing_distance() {
    let mut world = World::default();
    world.tick = Tick(464);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = 4 * POWERSCALE;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.values[1][CharacterValue::Flash as usize] = 20;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 13,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let mut active_flash = item(50, ItemFlags::empty());
    active_flash.driver = IDR_FLASH;
    world.items.insert(active_flash.id, active_flash);
    npc.inventory[12] = Some(ItemId(50));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 13, 10);
    world.map.tile_mut(13, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::IDLE);
    assert_eq!(npc.duration, (TICKS_PER_SECOND / 4) as i32);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 464);
}

#[test]
fn simple_baddy_attack_action_does_not_distance_idle_without_active_flash_spell_slot() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = 4 * POWERSCALE;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.values[1][CharacterValue::Flash as usize] = 20;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 13,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 13, 10);
    world.map.tile_mut(13, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!(npc.tox, 11);
    assert_eq!(npc.toy, 10);
}

#[test]
fn simple_baddy_fireball_spacing_moves_toward_distance_seven() {
    let mut world = World::default();
    world.tick = Tick(466);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FIREBALL_COST + 1;
    npc.values[0][CharacterValue::Fireball as usize] = 1;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.values[1][CharacterValue::Fireball as usize] = 20;
    npc.values[1][CharacterValue::Flash as usize] = 5;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 20, 10);

    let target = world.characters.get(&CharacterId(2)).cloned().unwrap();
    assert!(world.setup_simple_baddy_fireball_distance_attack(CharacterId(1), &target, 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!(npc.tox, 11);
    assert_eq!(npc.toy, 10);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 466);
}

#[test]
fn simple_baddy_fireball_spacing_requires_fireball_above_flash() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FIREBALL_COST + 1;
    npc.values[0][CharacterValue::Fireball as usize] = 1;
    npc.values[1][CharacterValue::Fireball as usize] = 5;
    npc.values[1][CharacterValue::Flash as usize] = 5;
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 20, 10);

    let target = world.characters.get(&CharacterId(2)).cloned().unwrap();
    assert!(!world.setup_simple_baddy_fireball_distance_attack(CharacterId(1), &target, 1));
    assert_eq!(world.characters[&CharacterId(1)].action, 0);
}

#[test]
fn simple_baddy_distance_driver_uses_best_partial_when_exact_spacing_blocked() {
    let mut world = World::default();
    world.tick = Tick(467);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FIREBALL_COST + 1;
    npc.values[0][CharacterValue::Fireball as usize] = 1;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.values[1][CharacterValue::Fireball as usize] = 20;
    npc.values[1][CharacterValue::Flash as usize] = 5;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 20, 10);
    for y in 1..MAX_MAP - 1 {
        world.map.set_flags(13, y, MapFlags::MOVEBLOCK);
    }

    let target = world.characters.get(&CharacterId(2)).cloned().unwrap();
    assert!(world.setup_simple_baddy_fireball_distance_attack(CharacterId(1), &target, 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!(npc.tox, 11);
    assert_eq!(npc.toy, 10);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 467);
}

#[test]
fn simple_baddy_attack_action_attacks_moving_target_destination() {
    let mut world = World::default();
    world.tick = Tick(457);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 12,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let mut target = character(2);
    target.tox = 11;
    target.toy = 10;
    target.values[0][CharacterValue::Attack as usize] = 1;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 12, 10);
    world.map.tile_mut(12, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::ATTACK1);
    assert_eq!(npc.act1, 2);
    assert_eq!(npc.dir, Direction::Right as u8);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 457);
}

#[test]
fn simple_baddy_attack_action_walks_toward_visible_non_adjacent_enemies() {
    let mut world = World::default();
    world.tick = Tick(458);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 15,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 15, 10);
    world.map.tile_mut(15, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!(npc.tox, 11);
    assert_eq!(npc.toy, 10);
    assert_eq!(npc.dir, Direction::Right as u8);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 458);
}

#[test]
fn simple_baddy_attack_action_ignores_hurtme_priority_for_visible_score_like_c() {
    let mut world = World::default();
    world.tick = Tick(459);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![
            SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 999,
                visible: true,
                last_x: 14,
                last_y: 10,
            },
            SimpleBaddyEnemy {
                target_id: CharacterId(3),
                priority: 0,
                last_seen_tick: 1,
                visible: true,
                last_x: 10,
                last_y: 11,
            },
        ],
        ..SimpleBaddyDriverData::default()
    }));
    let mut hurt_target = character(2);
    hurt_target.values[0][CharacterValue::Attack as usize] = 1;
    let mut seen_target = character(3);
    seen_target.values[0][CharacterValue::Attack as usize] = 1;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(hurt_target, 14, 10);
    world.spawn_character(seen_target, 10, 11);
    world.map.tile_mut(14, 10).unwrap().light = 255;
    world.map.tile_mut(10, 11).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::ATTACK1);
    assert_eq!(npc.act1, 3);
}

#[test]
fn simple_baddy_attack_action_moves_to_target_back_when_front_is_occupied() {
    let mut world = World::default();
    world.tick = Tick(458);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 10,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let mut target = character(2);
    target.dir = Direction::Right as u8;
    let front_blocker = character(3);
    world.spawn_character(npc, 9, 9);
    world.spawn_character(target, 10, 10);
    world.spawn_character(front_blocker, 11, 10);
    world.map.tile_mut(10, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!(npc.tox, 9);
    assert_eq!(npc.toy, 10);
    assert_eq!(npc.dir, Direction::Down as u8);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 458);
}

#[test]
fn simple_baddy_attack_action_skips_back_move_when_back_tile_is_blocked() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 10,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let mut target = character(2);
    target.dir = Direction::Right as u8;
    let front_blocker = character(3);
    world.spawn_character(npc, 9, 9);
    world.spawn_character(target, 10, 10);
    world.spawn_character(front_blocker, 11, 10);
    world
        .map
        .tile_mut(9, 10)
        .unwrap()
        .flags
        .insert(MapFlags::MOVEBLOCK);
    world.map.tile_mut(10, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_ne!((npc.tox, npc.toy), (9, 10));
}

#[test]
fn simple_baddy_attack_back_move_rejects_front_position_like_c() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.group = 7;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let mut target = character(2);
    target.dir = Direction::Right as u8;
    world.spawn_character(npc, 11, 10);
    world.spawn_character(target.clone(), 10, 10);
    target.x = 10;
    target.y = 10;

    assert!(!world.setup_simple_baddy_attack_back_move(CharacterId(1), &target, 1));
}

#[test]
fn simple_baddy_attack_back_move_rejects_same_group_side_occupant_like_c() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.group = 7;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    let mut target = character(2);
    target.dir = Direction::Right as u8;
    let front_blocker = character(3);
    let mut side_ally = character(4);
    side_ally.group = 7;
    world.spawn_character(npc, 9, 9);
    world.spawn_character(target.clone(), 10, 10);
    world.spawn_character(front_blocker, 11, 10);
    world.spawn_character(side_ally, 10, 11);
    target.x = 10;
    target.y = 10;

    assert!(!world.setup_simple_baddy_attack_back_move(CharacterId(1), &target, 1));
}

#[test]
fn simple_baddy_flee_action_scores_blocked_escape_path() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.endurance = 5 * POWERSCALE;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 13,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 13, 10);
    world.map.tile_mut(13, 10).unwrap().light = 255;
    world
        .map
        .tile_mut(8, 10)
        .unwrap()
        .flags
        .insert(MapFlags::MOVEBLOCK);

    assert!(world.setup_simple_baddy_flee_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert!(npc.tox < 10);
}

#[test]
fn simple_baddy_attack_action_uses_best_partial_path_when_target_unreachable() {
    let mut world = World::default();
    world.tick = Tick(460);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 15,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 15, 10);
    world.map.tile_mut(15, 10).unwrap().light = 255;
    for y in 1..MAX_MAP - 1 {
        world
            .map
            .tile_mut(12, y)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);
    }

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!((npc.tox, npc.toy), (11, 10));
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 460);
}

#[test]
fn simple_baddy_attack_action_uses_adjacent_blocker_when_path_fails() {
    let mut world = World::default();
    world.tick = Tick(461);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 13,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    let mut blocker = item(10, ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK);
    blocker.x = 11;
    blocker.y = 10;

    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 13, 10);
    world.map.tile_mut(13, 10).unwrap().light = 255;
    world.items.insert(blocker.id, blocker);
    let tile = world.map.tile_mut(11, 10).unwrap();
    tile.item = 10;
    tile.flags.insert(MapFlags::TMOVEBLOCK);
    for y in 1..MAX_MAP - 1 {
        world
            .map
            .tile_mut(12, y)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);
    }

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::USE);
    assert_eq!(npc.dir, Direction::Right as u8);
    assert_eq!(npc.act1, 10);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 461);
}

#[test]
fn simple_baddy_attack_action_idles_when_unreachable_path_does_not_improve() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 15,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 15, 10);
    world.map.tile_mut(15, 10).unwrap().light = 255;
    for y in 1..MAX_MAP - 1 {
        world
            .map
            .tile_mut(11, y)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);
    }

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::IDLE);
    assert_eq!(npc.duration, (TICKS_PER_SECOND / 4) as i32);
}

#[test]
fn distance_driver_prefers_moving_target_position_like_c() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.values[0][CharacterValue::Speed as usize] = 50;
    let mut target = character(2);
    target.tox = 10;
    target.toy = 14;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 16, 10);
    world.map.tile_mut(16, 10).unwrap().light = 255;

    assert!(world.distance_driver(CharacterId(1), CharacterId(2), 1, 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!((npc.tox, npc.toy), (10, 11));
    assert_eq!(npc.dir, Direction::Down as u8);
}

#[test]
fn distance_driver_returns_false_when_already_at_requested_distance() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.values[0][CharacterValue::Speed as usize] = 50;
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 18, 10);
    world.map.tile_mut(18, 10).unwrap().light = 255;

    assert!(!world.distance_driver(CharacterId(1), CharacterId(2), 8, 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, 0);
}

#[test]
fn distance_driver_uses_best_partial_path_when_exact_distance_unreachable() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.values[0][CharacterValue::Speed as usize] = 50;
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 15, 10);
    world.map.tile_mut(15, 10).unwrap().light = 255;
    for y in 1..MAX_MAP - 1 {
        world
            .map
            .tile_mut(12, y)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);
    }

    assert!(world.distance_driver(CharacterId(1), CharacterId(2), 1, 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!((npc.tox, npc.toy), (11, 10));
}

#[test]
fn simple_baddy_attack_action_uses_explicit_fight_driver_home_for_stop_distance() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.rest_x = 10;
    npc.rest_y = 10;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        stopdist: 6,
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 14,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 14, 10);
    world.map.tile_mut(14, 10).unwrap().light = 255;
    assert!(world.set_simple_baddy_home(CharacterId(1), 14, 10));

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    let data = npc
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert_eq!(data.home_x, 14);
    assert_eq!(data.home_y, 10);
    assert_eq!(data.enemies.len(), 1);
}

#[test]
fn simple_baddy_notsecure_day_post_walks_to_rest_home_like_c() {
    let mut world = World::default();
    world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
    world.date.hour = 12;
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.rest_x = 15;
    npc.rest_y = 10;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        dayx: 30,
        dayy: 10,
        nightx: 35,
        nighty: 10,
        notsecure: 1,
        ..SimpleBaddyDriverData::default()
    }));
    world.spawn_character(npc, 10, 10);

    assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!((npc.tox, npc.toy), (11, 10));
    assert_eq!(npc.dir, Direction::Right as u8);
}

#[test]
fn simple_baddy_drinkspecial_removes_poison_when_poison0_is_active() {
    let mut world = World::default();
    world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.hp = 10 * POWERSCALE;
    npc.mana = 10 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 10;
    npc.values[0][CharacterValue::Mana as usize] = 10;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        drinkspecial: 1,
        ..SimpleBaddyDriverData::default()
    }));
    let mut poison0 = item(10, ItemFlags::empty());
    poison0.driver = IDR_POISON0;
    let mut poison1 = item(11, ItemFlags::empty());
    poison1.driver = IDR_POISON1;
    npc.inventory[SPELL_SLOT_START] = Some(poison0.id);
    npc.inventory[SPELL_SLOT_START + 1] = Some(poison1.id);
    world.items.insert(poison0.id, poison0);
    world.items.insert(poison1.id, poison1);
    world.spawn_character(npc, 10, 10);

    assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert!(npc.inventory[SPELL_SLOT_START].is_none());
    assert!(npc.inventory[SPELL_SLOT_START + 1].is_none());
    assert!(!world.items.contains_key(&ItemId(10)));
    assert!(!world.items.contains_key(&ItemId(11)));
    assert!(npc
        .flags
        .contains(CharacterFlags::ITEMS | CharacterFlags::UPDATE));
    assert_eq!(npc.action, action::IDLE);
    assert_eq!(
        world.drain_pending_area_texts(),
        vec![WorldAreaText {
            x: 10,
            y: 10,
            max_distance: (SAY_DIST / 2) as u16,
            message: "Character drinks a potion.".to_string(),
        }]
    );
}

#[test]
fn simple_baddy_at_day_post_drinkspecial_runs_before_idle() {
    let mut world = World::default();
    world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
    world.date.hour = 12;
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.hp = 10 * POWERSCALE;
    npc.mana = 10 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 10;
    npc.values[0][CharacterValue::Mana as usize] = 10;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        dayx: 10,
        dayy: 10,
        daydir: Direction::Down as i32,
        drinkspecial: 1,
        ..SimpleBaddyDriverData::default()
    }));
    let mut poison0 = item(10, ItemFlags::empty());
    poison0.driver = IDR_POISON0;
    npc.inventory[SPELL_SLOT_START] = Some(poison0.id);
    world.items.insert(poison0.id, poison0);
    world.spawn_character(npc, 10, 10);

    assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.dir, Direction::Down as u8);
    assert!(npc.inventory[SPELL_SLOT_START].is_none());
    assert!(!world.items.contains_key(&ItemId(10)));
    assert_eq!(npc.action, action::IDLE);
}

#[test]
fn simple_baddy_drinkspecial_requires_poison0_trigger() {
    let mut world = World::default();
    world.tick = Tick((TICKS_PER_SECOND * 2) as u64);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        drinkspecial: 1,
        ..SimpleBaddyDriverData::default()
    }));
    let mut poison1 = item(11, ItemFlags::empty());
    poison1.driver = IDR_POISON1;
    npc.inventory[SPELL_SLOT_START] = Some(poison1.id);
    world.items.insert(poison1.id, poison1);
    world.spawn_character(npc, 10, 10);

    assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.inventory[SPELL_SLOT_START], Some(ItemId(11)));
    assert!(world.items.contains_key(&ItemId(11)));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn simple_baddy_death_driver_creates_earth_demon_effects_at_killer() {
    let mut world = World::default();
    let mut dead = character(1);
    dead.driver = CDR_SIMPLEBADDY;
    dead.flags.insert(CharacterFlags::EDEMON);
    dead.flags.insert(CharacterFlags::GOD);
    dead.values[1][CharacterValue::Demon as usize] = 6;
    let killer = character(2);
    assert!(world.spawn_character(dead, 10, 10));
    assert!(world.spawn_character(killer, 12, 10));
    world.map.tile_mut(12, 10).unwrap().light = 255;

    let effect_ids = world.apply_character_death_driver(CharacterId(1), CharacterId(2));

    assert_eq!(effect_ids.len(), 2);
    let mud = world.effects.get(&effect_ids[0]).unwrap();
    assert_eq!(mud.effect_type, EF_EARTHMUD);
    assert_eq!(mud.strength, 6);
    let rain = world.effects.get(&effect_ids[1]).unwrap();
    assert_eq!(rain.effect_type, EF_EARTHRAIN);
    assert_eq!(rain.strength, 6);
    let killer_tile = world.map.tile(12, 10).unwrap();
    assert!(killer_tile.effects.contains(&(effect_ids[0] as u16)));
    assert!(killer_tile.effects.contains(&(effect_ids[1] as u16)));
}

#[test]
fn simple_baddy_death_driver_respects_earth_demon_gates() {
    let mut world = World::default();
    let mut dead = character(1);
    dead.driver = CDR_SIMPLEBADDY;
    dead.flags.insert(CharacterFlags::EDEMON);
    dead.flags.insert(CharacterFlags::GOD);
    dead.values[1][CharacterValue::Demon as usize] = 5;
    let killer = character(2);
    assert!(world.spawn_character(dead, 10, 10));
    assert!(world.spawn_character(killer, 12, 10));
    world.map.tile_mut(12, 10).unwrap().light = 255;

    let effect_ids = world.apply_simple_baddy_death_driver(CharacterId(1), CharacterId(2));

    assert_eq!(effect_ids.len(), 1);
    assert_eq!(world.effects[&effect_ids[0]].effect_type, EF_EARTHRAIN);

    world
        .map
        .tile_mut(11, 10)
        .unwrap()
        .flags
        .insert(MapFlags::SIGHTBLOCK);
    let effect_ids = world.apply_simple_baddy_death_driver(CharacterId(1), CharacterId(2));

    assert!(effect_ids.is_empty());
}

#[test]
fn legacy_hurt_invokes_simple_baddy_death_driver_for_earth_demons() {
    let mut world = World::default();
    let mut dead = character(1);
    dead.driver = CDR_SIMPLEBADDY;
    dead.flags.insert(CharacterFlags::EDEMON);
    dead.flags.insert(CharacterFlags::GOD);
    dead.values[1][CharacterValue::Demon as usize] = 6;
    dead.hp = POWERSCALE;
    let killer = character(2);
    assert!(world.spawn_character(dead, 10, 10));
    assert!(world.spawn_character(killer, 12, 10));
    world.map.tile_mut(12, 10).unwrap().light = 255;

    let outcome = world
        .apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), POWERSCALE, 1, 0, 0)
        .unwrap();

    assert!(outcome.killed);
    let dead = world.characters.get(&CharacterId(1)).unwrap();
    assert!(dead.flags.contains(CharacterFlags::DEAD));
    assert!(world
        .effects
        .values()
        .any(|effect| effect.effect_type == EF_EARTHRAIN && effect.strength == 6));
    assert!(world
        .effects
        .values()
        .any(|effect| effect.effect_type == EF_EARTHMUD && effect.strength == 6));
}

#[test]
fn sound_area_specials_match_legacy_distance_and_pan() {
    let mut world = World {
        map: MapGrid::new(40, 40),
        ..World::default()
    };
    let mut nearby = character(1);
    nearby.flags.insert(CharacterFlags::PLAYER);
    nearby.x = 13;
    nearby.y = 14;
    let mut outside = character(2);
    outside.flags.insert(CharacterFlags::PLAYER);
    outside.x = 31;
    outside.y = 10;
    let mut npc = character(3);
    npc.x = 12;
    npc.y = 10;

    world.add_character(nearby);
    world.add_character(outside);
    world.add_character(npc);

    let specials = world.sound_area_specials(10, 10, 7);

    assert_eq!(specials.len(), 1);
    assert_eq!(specials[0].character_id, CharacterId(1));
    assert_eq!(specials[0].special.special_type, 7);
    assert_eq!(specials[0].special.opt1, -250);
    assert_eq!(specials[0].special.opt2, 300);
}

// `process_lostcon_attack_action_with_random` (C `lostcon_driver`'s
// `fight_driver_update(cn); if (fight_driver_attack_visible(cn,
// ppd->nomove)) return; if (!ppd->nomove &&
// fight_driver_follow_invisible(cn)) return;` cascade, `lostcon.c:200-203`)
// reuses `fight_driver_attack_visible_and_follow`'s generalized engine
// (see `PORTING_TODO.md`'s "Player-side fight-driver auto-combat" task).

#[test]
fn lostcon_attack_action_ignores_a_normal_playing_character() {
    let mut world = World::default();
    let npc = character(1);
    world.spawn_character(npc, 10, 10);

    assert!(!world.process_lostcon_attack_action_with_random(
        CharacterId(1),
        1,
        FightDriverSuppressions::default(),
        |_| 0,
    ));
}

#[test]
fn lostcon_attack_action_fights_back_a_visible_enemy() {
    let mut world = World::default();
    let mut lingering = character(1);
    lingering.driver = CDR_LOSTCON;
    lingering.driver_state = Some(CharacterDriverState::Lostcon(LostconDriverData {
        deadline: 1_000,
    }));
    lingering.fight_driver = Some(FightDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 1,
            visible: true,
            last_x: 11,
            last_y: 10,
        }],
        ..FightDriverData::default()
    });
    let target = character(2);
    world.spawn_character(lingering, 10, 10);
    world.spawn_character(target, 11, 10);

    assert!(world.process_lostcon_attack_action_with_random(
        CharacterId(1),
        1,
        FightDriverSuppressions::default(),
        |_| 0,
    ));

    let lingering = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(lingering.action, action::ATTACK1);
}

#[test]
fn lostcon_attack_action_nomove_suppresses_attack_task_and_invisible_follow() {
    // C: `!ppd->nomove` gates both the `Attack` task inside
    // `fight_driver_attack_enemy` (`drvlib.c:1682`'s own `if (!nomove ||
    // dist(cn,co)==2)` guard) and the whole `fight_driver_follow_invisible`
    // call. A lone adjacent enemy with nothing else to do means "nomove"
    // leaves the lingering character with no action to take at all.
    let mut world = World::default();
    let mut lingering = character(1);
    lingering.driver = CDR_LOSTCON;
    lingering.driver_state = Some(CharacterDriverState::Lostcon(LostconDriverData {
        deadline: 1_000,
    }));
    lingering.fight_driver = Some(FightDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 1,
            visible: false,
            last_x: 15,
            last_y: 10,
        }],
        ..FightDriverData::default()
    });
    let mut target = character(2);
    target.flags.insert(CharacterFlags::INVISIBLE);
    world.spawn_character(lingering, 10, 10);
    world.spawn_character(target, 15, 10);

    assert!(!world.process_lostcon_attack_action_with_random(
        CharacterId(1),
        1,
        FightDriverSuppressions {
            nomove: true,
            ..FightDriverSuppressions::default()
        },
        |_| 0,
    ));

    let lingering = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(lingering.action, action::IDLE);
}

#[test]
fn lostcon_attack_action_follows_an_invisible_enemy_toward_its_last_position() {
    let mut world = World::default();
    let mut lingering = character(1);
    lingering.driver = CDR_LOSTCON;
    lingering.driver_state = Some(CharacterDriverState::Lostcon(LostconDriverData {
        deadline: 1_000,
    }));
    lingering.fight_driver = Some(FightDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 1,
            visible: false,
            last_x: 15,
            last_y: 10,
        }],
        ..FightDriverData::default()
    });
    let mut target = character(2);
    target.flags.insert(CharacterFlags::INVISIBLE);
    world.spawn_character(lingering, 10, 10);
    world.spawn_character(target, 15, 10);

    assert!(world.process_lostcon_attack_action_with_random(
        CharacterId(1),
        1,
        FightDriverSuppressions::default(),
        |_| 0,
    ));

    let lingering = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(lingering.action, action::WALK);
}
