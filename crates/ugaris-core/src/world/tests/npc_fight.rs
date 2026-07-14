// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
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
    // Intentional mirror of the legacy C formula with zeroed stat inputs.
    #[allow(clippy::erasing_op, clippy::identity_op)]
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
