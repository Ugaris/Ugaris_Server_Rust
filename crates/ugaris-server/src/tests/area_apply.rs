// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use super::*;

#[test]
fn random_shrine_security_increments_saves_and_marks_ppd() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut character = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);
    character.saves = 5;

    let result = apply_random_shrine_security(&mut player, &mut character, 53);

    assert_eq!(result, RandomShrineSecurityApplyResult::Used { saves: 6 });
    assert_eq!(character.saves, 6);
    assert!(player.has_used_random_shrine(53));
    assert_eq!(legacy_save_number(character.saves), "six");
}

#[test]
fn random_shrine_security_legacy_blocks_do_not_mark_ppd() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut secure = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);
    secure.saves = 6;

    let result = apply_random_shrine_security(&mut player, &mut secure, 54);

    assert_eq!(result, RandomShrineSecurityApplyResult::SecureAlready);
    assert_eq!(secure.saves, 6);
    assert!(!player.has_used_random_shrine(54));

    let mut hardcore = login_character(CharacterId(8), &login_block("Lisa"), 14, 10, 10);
    hardcore.flags.insert(CharacterFlags::HARDCORE);

    let result = apply_random_shrine_security(&mut player, &mut hardcore, 55);

    assert_eq!(result, RandomShrineSecurityApplyResult::Hardcore);
    assert_eq!(hardcore.saves, 0);
    assert!(!player.has_used_random_shrine(55));
}

#[test]
fn random_shrine_jobless_clears_professions_and_marks_ppd() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut character = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);
    character.professions[0] = 3;
    character.professions[4] = 7;

    let result = apply_random_shrine_jobless(&mut player, &mut character, 63);

    assert_eq!(result, RandomShrineJoblessApplyResult::Used);
    assert!(character
        .professions
        .iter()
        .all(|profession| *profession == 0));
    assert!(character
        .flags
        .contains(CharacterFlags::PROF | CharacterFlags::UPDATE));
    assert!(player.has_used_random_shrine(63));
}

#[test]
fn random_shrine_jobless_blocks_without_marking_when_already_jobless() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut character = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);

    let result = apply_random_shrine_jobless(&mut player, &mut character, 64);

    assert_eq!(result, RandomShrineJoblessApplyResult::AlreadyJobless);
    assert!(!character.flags.contains(CharacterFlags::PROF));
    assert!(!player.has_used_random_shrine(64));
}

#[test]
fn random_shrine_edge_spends_saves_for_legacy_exp_and_marks_ppd() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut character = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);
    character.level = 10;
    character.saves = 3;

    let result = apply_random_shrine_edge(&mut player, &mut character, 30, 20);

    let level_value = level_value(15);
    let expected = level_value / 3 + 3 * level_value / 30;
    assert_eq!(result, RandomShrineEdgeApplyResult::Used { exp: expected });
    // The exp grant itself now happens in the caller via `World::give_exp`
    // (C `shrine_edge` calls `give_exp(cn, bonus)`, not a raw mutation),
    // so this function only reports the amount via the result and does not
    // mutate `character.exp` - see `world/tests/exp.rs` for `give_exp`
    // coverage and `main.rs`'s `RandomShrineKind::Edge` arm for the wiring.
    assert_eq!(character.saves, 0);
    assert!(character.flags.contains(CharacterFlags::UPDATE));
    assert!(player.has_used_random_shrine(30));
}

#[test]
fn random_shrine_kindness_blocks_already_kind_without_marking() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut character = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);

    let result = apply_random_shrine_kindness(&mut player, &mut character, 41);

    assert_eq!(result, RandomShrineKindnessApplyResult::AlreadyKind);
    assert!(!player.has_used_random_shrine(41));
}

#[test]
fn random_shrine_vitality_raises_warrior_hp_and_marks_ppd() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut character = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);
    character.flags.insert(CharacterFlags::WARRIOR);
    character.values[0][CharacterValue::Hp as usize] = 95;
    character.values[1][CharacterValue::Hp as usize] = 95;

    let result = apply_random_shrine_vitality(&mut player, &mut character, 50);

    let mut expected_cost = 0;
    for current in 95..100 {
        expected_cost += legacy_raise_cost(CharacterValue::Hp as usize, current, false);
    }
    assert_eq!(
        result,
        RandomShrineVitalityApplyResult::Used {
            value: CharacterValue::Hp,
            amount: 5,
            cost: expected_cost,
        }
    );
    assert_eq!(character.values[1][CharacterValue::Hp as usize], 100);
    assert_eq!(character.values[0][CharacterValue::Hp as usize], 100);
    // The exp grant/`update_char` recompute now happen in the caller via
    // `World::give_exp`/`World::update_character` (C `shrine_vitality`
    // calls `give_exp(cn, cost)` then `update_char(cn)`), so this function
    // only reports `cost` via the result and does not mutate
    // `character.exp` - see `main.rs`'s `RandomShrineKind::Vitality` arm.
    assert_eq!(character.exp_used, expected_cost);
    assert!(character.flags.contains(CharacterFlags::UPDATE));
    assert!(player.has_used_random_shrine(50));
}

#[test]
fn random_shrine_vitality_raises_non_warrior_mana_to_legacy_cap() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut character = login_character(CharacterId(7), &login_block("Lisa"), 14, 10, 10);
    character.values[0][CharacterValue::Mana as usize] = 113;
    character.values[1][CharacterValue::Mana as usize] = 113;

    let result = apply_random_shrine_vitality(&mut player, &mut character, 50);

    let expected_cost = legacy_raise_cost(CharacterValue::Mana as usize, 113, false)
        + legacy_raise_cost(CharacterValue::Mana as usize, 114, false);
    assert_eq!(
        result,
        RandomShrineVitalityApplyResult::Used {
            value: CharacterValue::Mana,
            amount: 2,
            cost: expected_cost,
        }
    );
    assert_eq!(character.values[1][CharacterValue::Mana as usize], 115);
    assert!(player.has_used_random_shrine(50));
}

#[test]
fn random_shrine_continuity_enforces_sequence_and_grants_legacy_exp() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut character = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);
    character.level = 12;

    let result = apply_random_shrine_continuity(&mut player, &mut character, 11);

    assert_eq!(
        result,
        RandomShrineContinuityApplyResult::NeedYoungerBrother
    );
    assert_eq!(player.random_shrine_continuity, 10);

    let result = apply_random_shrine_continuity(&mut player, &mut character, 10);

    let expected = level_value(10) / 6;
    assert_eq!(
        result,
        RandomShrineContinuityApplyResult::Used {
            exp: expected,
            opens_gate: false,
        }
    );
    assert_eq!(player.random_shrine_continuity, 11);
    // The exp grant now happens in the caller via `World::give_exp` (C
    // `shrine_continuity` calls `give_exp(cn, cost)`, not a raw mutation),
    // so this function only reports `exp` via the result and does not
    // mutate `character.exp` - see `main.rs`'s `RandomShrineKind::Continuity`
    // arm.
    assert!(character.flags.contains(CharacterFlags::UPDATE));

    let result = apply_random_shrine_continuity(&mut player, &mut character, 10);

    assert_eq!(
        result,
        RandomShrineContinuityApplyResult::AlreadyVisited { opens_gate: false }
    );
    assert_eq!(player.random_shrine_continuity, 11);
}

#[test]
fn random_shrine_continuity_level_99_opens_gate_for_new_or_repeat_use() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut character = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);
    character.level = 200;
    player.random_shrine_continuity = 99;

    let result = apply_random_shrine_continuity(&mut player, &mut character, 99);

    assert!(matches!(
        result,
        RandomShrineContinuityApplyResult::Used {
            opens_gate: true,
            ..
        }
    ));
    assert_eq!(player.random_shrine_continuity, 100);

    let result = apply_random_shrine_continuity(&mut player, &mut character, 99);

    assert_eq!(
        result,
        RandomShrineContinuityApplyResult::AlreadyVisited { opens_gate: true }
    );
}

#[test]
fn random_shrine_indecisiveness_lowers_raisable_skills_three_times_and_marks_ppd() {
    let mut player = PlayerRuntime::connected(1, 0);
    // `login_character` seeds Hp/Endurance/Mana/Speed at 50; Hp/Endurance/
    // Mana (skill_start 10, raisable) get lowered 3x each (50 -> 47),
    // Speed (skill_raise_cost_factor 0, unraisable via `lower_value`) is
    // untouched, matching C `shrine_indecisiveness`'s `lower_value` no-op.
    let mut character = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);

    let result = apply_random_shrine_indecisiveness(&mut player, &mut character, 3);

    assert_eq!(result, RandomShrineIndecisivenessApplyResult::Used);
    assert_eq!(character.values[1][CharacterValue::Hp as usize], 47);
    assert_eq!(character.values[1][CharacterValue::Endurance as usize], 47);
    assert_eq!(character.values[1][CharacterValue::Mana as usize], 47);
    assert_eq!(character.values[1][CharacterValue::Speed as usize], 50);
    assert!(player.has_used_random_shrine(3));
}

#[test]
fn random_shrine_indecisiveness_blocks_without_marking_when_noexp() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut character = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);
    character.flags.insert(CharacterFlags::NOEXP);
    let hp_before = character.values[1][CharacterValue::Hp as usize];

    let result = apply_random_shrine_indecisiveness(&mut player, &mut character, 4);

    assert_eq!(result, RandomShrineIndecisivenessApplyResult::NoExp);
    assert_eq!(character.values[1][CharacterValue::Hp as usize], hp_before);
    assert!(!player.has_used_random_shrine(4));
}

#[test]
fn random_shrine_bribes_takes_partial_gold_for_legacy_exp_and_marks_ppd() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut character = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);
    character.level = 10;
    let level_value = level_value(15);
    let want = level_value * 4 / 3;
    character.gold = want + 500;

    let result = apply_random_shrine_bribes(&mut player, &mut character, 13, 20);

    assert_eq!(
        result,
        RandomShrineBribesApplyResult::Used {
            gold: want,
            exp: want / 4,
            almost_empty: true,
        }
    );
    assert_eq!(character.gold, 500);
    assert!(character
        .flags
        .contains(CharacterFlags::ITEMS | CharacterFlags::UPDATE));
    assert!(player.has_used_random_shrine(13));
}

#[test]
fn random_shrine_bribes_takes_all_gold_when_below_want_and_marks_ppd() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut character = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);
    character.level = 10;
    let level_value = level_value(15);
    let need = level_value * 4 / 10;
    character.gold = need; // above `need`, below `want` -> fully emptied

    let result = apply_random_shrine_bribes(&mut player, &mut character, 14, 20);

    assert_eq!(
        result,
        RandomShrineBribesApplyResult::Used {
            gold: need,
            exp: need / 4,
            almost_empty: false,
        }
    );
    assert_eq!(character.gold, 0);
    assert!(player.has_used_random_shrine(14));
}

#[test]
fn random_shrine_bribes_blocks_without_marking_when_gold_below_need() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut character = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);
    character.level = 10;
    let level_value = level_value(15);
    let need = level_value * 4 / 10;
    character.gold = need.saturating_sub(1);

    let result = apply_random_shrine_bribes(&mut player, &mut character, 15, 20);

    assert_eq!(result, RandomShrineBribesApplyResult::NotEnoughGold);
    assert_eq!(character.gold, need.saturating_sub(1));
    assert!(!player.has_used_random_shrine(15));
}

#[test]
fn random_shrine_bribes_blocks_without_marking_when_noexp() {
    let mut player = PlayerRuntime::connected(1, 0);
    let mut character = login_character(CharacterId(7), &login_block("Ralph"), 14, 10, 10);
    character.flags.insert(CharacterFlags::NOEXP);
    character.gold = 1_000_000;

    let result = apply_random_shrine_bribes(&mut player, &mut character, 16, 20);

    assert_eq!(result, RandomShrineBribesApplyResult::NoExp);
    assert_eq!(character.gold, 1_000_000);
    assert!(!player.has_used_random_shrine(16));
}

#[test]
fn pick_berry_runtime_grants_template_and_marks_flower_ppd() {
    let mut world = World::default();
    world.add_character(login_character(
        CharacterId(1),
        &login_block("Ralph"),
        31,
        10,
        10,
    ));
    let mut loader = ZoneLoader::new();
    loader.item_templates.insert(
        "picked_flower_h".to_string(),
        ugaris_core::zone::ItemTemplate {
            key: "picked_flower_h".to_string(),
            name: "Flower H".to_string(),
            description: String::new(),
            flags: ItemFlags::USED,
            sprite: 11190,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0x1f02,
            modifier_index: [0; ugaris_core::entity::MAX_MODIFIERS],
            modifier_value: [0; ugaris_core::entity::MAX_MODIFIERS],
            driver: ugaris_core::item_driver::IDR_LIZARDFLOWER,
            driver_data: vec![2],
        },
    );
    let mut player = PlayerRuntime::connected(1, 0);

    let result = apply_pick_berry(
        &mut world,
        &mut loader,
        Some(&mut player),
        CharacterId(1),
        2,
        0x001f_2030,
        50_000,
    );

    assert_eq!(result, PickBerryApplyResult::Picked("Flower H".to_string()));
    assert_eq!(player.flower_last_used_seconds(0x001f_2030), Some(50_000));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    let cursor_item = character.cursor_item.unwrap();
    assert_eq!(world.items.get(&cursor_item).unwrap().name, "Flower H");
}

#[test]
fn flask_ingredient_feedback_matches_legacy_order_and_names() {
    let mut counts = [0; 29];
    counts[0] = 2;
    counts[20] = 1;
    counts[24] = 7;

    assert_eq!(
        flask_ingredient_feedback(counts),
        vec![
            "Contains 2 parts Adygalah.".to_string(),
            "Contains 1 parts Fiery Stone.".to_string(),
        ]
    );
}

#[test]
fn pick_berry_runtime_enforces_herbalist_ripe_time() {
    let mut world = World::default();
    let mut character = login_character(CharacterId(1), &login_block("Ralph"), 31, 10, 10);
    character.professions[profession::HERBALIST] = 20;
    world.add_character(character);
    let mut loader = ZoneLoader::new();
    loader.item_templates.insert(
        "picked_flower_h".to_string(),
        ugaris_core::zone::ItemTemplate {
            key: "picked_flower_h".to_string(),
            name: "Flower H".to_string(),
            description: String::new(),
            flags: ItemFlags::USED,
            sprite: 11190,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0x1f02,
            modifier_index: [0; ugaris_core::entity::MAX_MODIFIERS],
            modifier_value: [0; ugaris_core::entity::MAX_MODIFIERS],
            driver: ugaris_core::item_driver::IDR_LIZARDFLOWER,
            driver_data: vec![2],
        },
    );
    let mut player = PlayerRuntime::connected(1, 0);
    player.mark_flower_used(7, 100);

    let blocked = apply_pick_berry(
        &mut world,
        &mut loader,
        Some(&mut player),
        CharacterId(1),
        2,
        7,
        100 + 8 * 60 * 60 - 1,
    );

    assert_eq!(blocked, PickBerryApplyResult::NotRipe);

    let picked = apply_pick_berry(
        &mut world,
        &mut loader,
        Some(&mut player),
        CharacterId(1),
        2,
        7,
        100 + 8 * 60 * 60,
    );

    assert_eq!(picked, PickBerryApplyResult::Picked("Flower H".to_string()));
}

#[test]
fn alchemy_flower_runtime_grants_ingredient_and_marks_flower_ppd() {
    let mut world = World::default();
    world.add_character(login_character(
        CharacterId(1),
        &login_block("Ralph"),
        1,
        10,
        10,
    ));
    let mut loader = ZoneLoader::new();
    loader.item_templates.insert(
        "alc_berry1".to_string(),
        ugaris_core::zone::ItemTemplate {
            key: "alc_berry1".to_string(),
            name: "Berry A".to_string(),
            description: String::new(),
            flags: ItemFlags::USED,
            sprite: 50280,
            value: 0,
            min_level: 0,
            max_level: 0,
            needs_class: 0,
            template_id: 0x1f11,
            modifier_index: [0; ugaris_core::entity::MAX_MODIFIERS],
            modifier_value: [0; ugaris_core::entity::MAX_MODIFIERS],
            driver: ugaris_core::item_driver::IDR_FLOWER,
            driver_data: vec![17],
        },
    );
    let mut player = PlayerRuntime::connected(1, 0);

    let result = apply_pick_alchemy_flower(
        &mut world,
        &mut loader,
        Some(&mut player),
        CharacterId(1),
        17,
        0x0001_2814,
        60_000,
    );

    assert_eq!(result, PickBerryApplyResult::Picked("Berry A".to_string()));
    assert_eq!(player.flower_last_used_seconds(0x0001_2814), Some(60_000));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    let cursor_item = character.cursor_item.unwrap();
    assert_eq!(world.items.get(&cursor_item).unwrap().name, "Berry A");
}

#[test]
fn area_message_sessions_match_legacy_square_distance() {
    let mut world = World::default();
    let mut origin = login_character(CharacterId(1), &login_block("Ralph"), 1, 10, 10);
    origin.x = 10;
    origin.y = 10;
    let mut edge = login_character(CharacterId(2), &login_block("Lisa"), 1, 26, 26);
    edge.x = 26;
    edge.y = 26;
    let mut outside = login_character(CharacterId(3), &login_block("Milhouse"), 1, 27, 10);
    outside.x = 27;
    outside.y = 10;
    world.add_character(origin);
    world.add_character(edge);
    world.add_character(outside);

    let mut runtime = ServerRuntime::default();
    let mut origin_player = PlayerRuntime::connected(10, 0);
    origin_player.character_id = Some(CharacterId(1));
    let mut edge_player = PlayerRuntime::connected(20, 0);
    edge_player.character_id = Some(CharacterId(2));
    let mut outside_player = PlayerRuntime::connected(30, 0);
    outside_player.character_id = Some(CharacterId(3));
    runtime.players.insert(10, origin_player);
    runtime.players.insert(20, edge_player);
    runtime.players.insert(30, outside_player);

    let mut sessions = runtime.sessions_for_area_message(&world, CharacterId(1), 16);
    sessions.sort_unstable_by_key(|(session_id, _)| *session_id);

    assert_eq!(sessions, vec![(10, CharacterId(1)), (20, CharacterId(2))]);
}

#[test]
fn area_leave_cleanup_removes_arkhata_stopwatch_from_live_cursor() {
    let mut world = World::default();
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 37, 10, 10);
    character.cursor_item = Some(ItemId(10));
    character.current_container = Some(ItemId(10));
    world.add_character(character);

    let mut stopwatch = test_item_with_driver(ItemId(10), IDR_ARKHATA);
    stopwatch.carried_by = Some(character_id);
    stopwatch.driver_data = vec![1];
    world.add_item(stopwatch);

    let removed = remove_area_leave_vanishing_items(&mut world, character_id);

    assert_eq!(removed, vec![ItemId(10)]);
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.cursor_item, None);
    assert_eq!(character.current_container, None);
    assert!(!world.items.contains_key(&ItemId(10)));
}

#[test]
fn lab2_grave_open_spawns_undead_and_attaches_described_grave_item() {
    let mut loader = ZoneLoader::new();
    loader
        .load_character_templates_str(
            r#"
                lab2_undead:
                  name="Undead"
                  V_HP=40
                  V_ENDURANCE=20
                  V_MANA=0
                ;
                lab2_skeleton:
                  name="Skeleton"
                  V_HP=30
                  V_ENDURANCE=10
                  V_MANA=0
                ;
            "#,
        )
        .unwrap();
    loader
        .load_item_templates_str(
            r#"
                lab2_elias_hat:
                  name="Elias Hat"
                  flag=IF_TAKE
                ;
            "#,
        )
        .unwrap();

    let mut world = World::default();
    let actor_id = CharacterId(1);
    let login = LoginBlock {
        name: "Tester".to_string(),
        password: String::new(),
        vendor: 0,
        client_version: None,
        his_ip: 0,
        our_ip: 0,
        unique: 0,
    };
    assert!(world.spawn_character(login_character(actor_id, &login, 22, 10, 10), 10, 10));

    let mut grave = test_item_with_driver(ItemId(8), ugaris_core::item_driver::IDR_LAB2_GRAVE);
    grave.x = 194;
    grave.y = 183;
    grave.sprite = 11000;
    grave.driver_data = vec![0; 16];
    world.add_item(grave);

    let mut runtime = ServerRuntime::default();
    runtime.next_character_id = 100;
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(actor_id);
    runtime.players.insert(1, player);

    assert!(apply_lab2_grave_open(
        &mut world,
        &mut runtime,
        &mut loader,
        ItemId(8),
        actor_id,
        0,
    ));

    let grave = world.items.get(&ItemId(8)).unwrap();
    assert_eq!(grave.sprite, 11001);
    assert_eq!(
        i32::from_le_bytes(grave.driver_data[4..8].try_into().unwrap()),
        100
    );
    assert_eq!(
        i32::from_le_bytes(grave.driver_data[8..12].try_into().unwrap()),
        100
    );

    let undead = world.characters.get(&CharacterId(100)).unwrap();
    assert_eq!(undead.name, "Undead");
    assert_eq!(undead.driver, CDR_LAB2UNDEAD);
    assert_eq!((undead.x, undead.y), (194, 183));
    let CharacterDriverState::Lab2Undead(data) = undead.driver_state.as_ref().unwrap() else {
        panic!("Lab 2 grave spawn must retain undead driver state");
    };
    assert_eq!(data.grave_item_id, Some(ItemId(8)));
    assert_eq!(data.opened_by_character_id, Some(actor_id));
    assert_eq!(data.opened_by_serial, 1);
    let hat_id = undead.inventory[INVENTORY_START_INVENTORY].unwrap();
    assert_eq!(world.items.get(&hat_id).unwrap().name, "Elias Hat");
}

#[test]
fn lab2_undead_death_marks_opener_grave_cleared_with_serial_guard() {
    let mut world = World::default();
    let actor_id = CharacterId(1);
    let login = LoginBlock {
        name: "Tester".to_string(),
        password: String::new(),
        vendor: 0,
        client_version: None,
        his_ip: 0,
        our_ip: 0,
        unique: 0,
    };
    let mut actor = login_character(actor_id, &login, 22, 10, 10);
    actor.serial = 77;
    assert!(world.spawn_character(actor, 10, 10));

    let mut first_grave =
        test_item_with_driver(ItemId(7), ugaris_core::item_driver::IDR_LAB2_GRAVE);
    first_grave.x = 10;
    first_grave.y = 10;
    first_grave.driver_data = vec![0; 16];
    world.add_item(first_grave);
    let mut grave = test_item_with_driver(ItemId(8), ugaris_core::item_driver::IDR_LAB2_GRAVE);
    grave.x = 11;
    grave.y = 10;
    grave.driver_data = vec![0; 16];
    world.add_item(grave);

    let mut undead = login_character(CharacterId(20), &login, 22, 11, 10);
    undead.driver = CDR_LAB2UNDEAD;
    undead.hp = 1;
    undead.flags.insert(CharacterFlags::ALIVE);
    undead.driver_state = Some(CharacterDriverState::Lab2Undead(
        ugaris_core::character_driver::Lab2UndeadDriverData {
            grave_item_id: Some(ItemId(8)),
            opened_by_character_id: Some(actor_id),
            opened_by_serial: 77,
            ..Default::default()
        },
    ));
    assert!(world.spawn_character(undead, 11, 10));

    let mut runtime = ServerRuntime::default();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(actor_id);
    runtime.players.insert(1, player);

    world.apply_legacy_hurt(CharacterId(20), Some(actor_id), 1000, 1, 0, 0);
    assert_eq!(
        apply_pk_hate_from_hurt_events(&mut runtime, &mut world, 0, &ZoneLoader::new()),
        0
    );
    assert!(runtime
        .player_for_character_mut(actor_id)
        .unwrap()
        .legacy_lab2_grave_cleared(1));
}

#[test]
fn lab2_grave_open_reopens_cleared_normal_grave_empty() {
    let mut loader = ZoneLoader::new();
    loader
        .load_character_templates_str(
            r#"
                lab2_undead:
                  name="Undead"
                  V_HP=40
                  V_ENDURANCE=20
                  V_MANA=0
                ;
                lab2_skeleton:
                  name="Skeleton"
                  V_HP=30
                  V_ENDURANCE=10
                  V_MANA=0
                ;
            "#,
        )
        .unwrap();

    let mut world = World::default();
    let actor_id = CharacterId(1);
    let login = LoginBlock {
        name: "Tester".to_string(),
        password: String::new(),
        vendor: 0,
        client_version: None,
        his_ip: 0,
        our_ip: 0,
        unique: 0,
    };
    assert!(world.spawn_character(login_character(actor_id, &login, 22, 10, 10), 10, 10));

    let mut earlier_grave =
        test_item_with_driver(ItemId(7), ugaris_core::item_driver::IDR_LAB2_GRAVE);
    earlier_grave.x = 10;
    earlier_grave.y = 10;
    earlier_grave.driver_data = vec![0; 16];
    world.add_item(earlier_grave);

    let mut grave = test_item_with_driver(ItemId(8), ugaris_core::item_driver::IDR_LAB2_GRAVE);
    grave.x = 11;
    grave.y = 10;
    grave.sprite = 11000;
    grave.driver_data = vec![0; 16];
    world.add_item(grave);

    let mut runtime = ServerRuntime::default();
    runtime.next_character_id = 100;
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(actor_id);
    assert!(player.mark_legacy_lab2_grave_cleared(1));
    runtime.players.insert(1, player);

    assert!(apply_lab2_grave_open(
        &mut world,
        &mut runtime,
        &mut loader,
        ItemId(8),
        actor_id,
        0,
    ));

    assert!(!world.characters.contains_key(&CharacterId(100)));
    let grave = world.items.get(&ItemId(8)).unwrap();
    assert_eq!(grave.sprite, 11001);
    assert_eq!(
        i32::from_le_bytes(grave.driver_data[4..8].try_into().unwrap()),
        -1
    );
    assert_eq!(
        i32::from_le_bytes(grave.driver_data[8..12].try_into().unwrap()),
        -1
    );
    let texts = world.drain_pending_system_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].character_id, actor_id);
    assert_eq!(texts[0].message, "This grave is empty");
}

#[test]
fn apply_zombie_shrine_requires_matching_skull() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(20));
    let mut world = World::default();
    world.add_character(character);
    let mut skull = test_item(ItemId(20), 1, ItemFlags::USED | ItemFlags::TAKE);
    skull.template_id = IID_AREA2_ZOMBIESKULL1;
    skull.carried_by = Some(character_id);
    world.add_item(skull);
    let mut loader = ZoneLoader::new();

    assert_eq!(
        apply_zombie_shrine(&mut world, &mut loader, character_id, 1, 0, 0),
        ZombieShrineApplyResult::NeedsOffering(1)
    );
    assert_eq!(
        world.characters.get(&character_id).unwrap().cursor_item,
        Some(ItemId(20))
    );
    assert!(world.items.contains_key(&ItemId(20)));
}

#[test]
fn apply_zombie_shrine_consumes_skull_and_grants_item_to_cursor() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(20));
    let mut world = World::default();
    world.add_character(character);
    let mut skull = test_item(ItemId(20), 1, ItemFlags::USED | ItemFlags::TAKE);
    skull.template_id = IID_AREA2_ZOMBIESKULL1;
    skull.carried_by = Some(character_id);
    world.add_item(skull);
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(r#"zombie_skull2: name="Silver Skull" ID=01000026 flag=IF_TAKE ;"#)
        .unwrap();

    assert_eq!(
        apply_zombie_shrine(
            &mut world,
            &mut loader,
            character_id,
            0,
            seed_for_legacy_random(22, 0),
            0,
        ),
        ZombieShrineApplyResult::Gift("Silver Skull".to_string())
    );
    assert!(!world.items.contains_key(&ItemId(20)));
    let cursor_item_id = world
        .characters
        .get(&character_id)
        .unwrap()
        .cursor_item
        .unwrap();
    let gift = world.items.get(&cursor_item_id).unwrap();
    assert_eq!(gift.name, "Silver Skull");
    assert_eq!(gift.carried_by, Some(character_id));
}

#[test]
fn arkhata_stopwatch_feedback_matches_clerk_timer_state() {
    let mut player = PlayerRuntime::connected(1, 0);

    assert_eq!(arkhata_stopwatch_feedback(&player, 100), "#92 ");

    player.set_arkhata_clerk_timer(5, 100);
    assert_eq!(
        arkhata_stopwatch_feedback(&player, 550),
        "#91 Time: 90 Astonian Minutes"
    );
    assert_eq!(
        arkhata_stopwatch_feedback(&player, 1_001),
        "#92 YOU FAILED!"
    );
}

#[test]
fn apply_zombie_shrine_consumes_skull_and_grants_experience() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(20));
    character.exp = 100;
    let mut world = World::default();
    world.add_character(character);
    let mut skull = test_item(ItemId(20), 1, ItemFlags::USED | ItemFlags::TAKE);
    skull.template_id = IID_AREA2_ZOMBIESKULL3;
    skull.carried_by = Some(character_id);
    world.add_item(skull);
    let mut loader = ZoneLoader::new();

    assert_eq!(
        apply_zombie_shrine(
            &mut world,
            &mut loader,
            character_id,
            2,
            seed_for_legacy_random(7, 4),
            0,
        ),
        ZombieShrineApplyResult::Experience(2250)
    );
    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.cursor_item, None);
    assert_eq!(character.exp, 2350);
    assert!(!world.items.contains_key(&ItemId(20)));
}

#[test]
fn apply_zombie_shrine_experience_routes_through_give_exp_and_honors_noexp_and_modifier() {
    // C `area2.c:390` grants the zombie shrine experience via
    // `give_exp(cn, 2250)`, not a raw `ch[cn].exp += ...`, so it must
    // respect `CF_NOEXP` and the runtime `exp_modifier` like every other
    // `give_exp` call site.
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(20));
    character.exp = 100;
    character.flags.insert(CharacterFlags::NOEXP);
    let mut world = World::default();
    world.add_character(character);
    let mut skull = test_item(ItemId(20), 1, ItemFlags::USED | ItemFlags::TAKE);
    skull.template_id = IID_AREA2_ZOMBIESKULL3;
    skull.carried_by = Some(character_id);
    world.add_item(skull);
    let mut loader = ZoneLoader::new();

    assert_eq!(
        apply_zombie_shrine(
            &mut world,
            &mut loader,
            character_id,
            2,
            seed_for_legacy_random(7, 4),
            0,
        ),
        ZombieShrineApplyResult::Experience(2250)
    );
    // NOEXP blocks the grant entirely - exp stays untouched.
    assert_eq!(world.characters.get(&character_id).unwrap().exp, 100);

    // Re-run without NOEXP but with a doubled exp_modifier: the grant must
    // scale, matching `give_exp`'s multiplier math.
    let character_id = CharacterId(8);
    let mut character = login_character(character_id, &login_block("Tester2"), 1, 10, 10);
    character.cursor_item = Some(ItemId(21));
    character.exp = 100;
    world.add_character(character);
    let mut skull = test_item(ItemId(21), 1, ItemFlags::USED | ItemFlags::TAKE);
    skull.template_id = IID_AREA2_ZOMBIESKULL3;
    skull.carried_by = Some(character_id);
    world.add_item(skull);
    world.settings.exp_modifier = 2.0;

    assert_eq!(
        apply_zombie_shrine(
            &mut world,
            &mut loader,
            character_id,
            2,
            seed_for_legacy_random(7, 4),
            0,
        ),
        ZombieShrineApplyResult::Experience(2250)
    );
    assert_eq!(
        world.characters.get(&character_id).unwrap().exp,
        100 + 2250 * 2
    );
}

#[test]
fn apply_zombie_shrine_installs_timed_bonus_spell() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.cursor_item = Some(ItemId(20));
    let mut world = World::default();
    world.add_character(character);
    let mut skull = test_item(ItemId(20), 1, ItemFlags::USED | ItemFlags::TAKE);
    skull.template_id = IID_AREA2_ZOMBIESKULL1;
    skull.carried_by = Some(character_id);
    world.add_item(skull);
    let mut loader = ZoneLoader::new();

    assert_eq!(
        apply_zombie_shrine(
            &mut world,
            &mut loader,
            character_id,
            0,
            seed_for_legacy_random(22, 16),
            0,
        ),
        ZombieShrineApplyResult::Bonus {
            message: "You have been protected for a short while.",
            driver: IDR_ARMOR,
            strength: 100,
            duration_ticks: TICKS_PER_SECOND as i32 * 60 * 5,
        }
    );

    let character = world.characters.get(&character_id).unwrap();
    assert_eq!(character.cursor_item, None);
    let spell_id = character.inventory[29].unwrap();
    let spell = world.items.get(&spell_id).unwrap();
    assert_eq!(spell.driver, IDR_ARMOR);
    assert_eq!(spell.modifier_index[0], CharacterValue::Armor as i16);
    assert_eq!(spell.modifier_value[0], 100);
    assert_eq!(
        spell.driver_data,
        (TICKS_PER_SECOND as u32 * 60 * 5).to_le_bytes().to_vec()
    );
    assert_eq!(character.values[0][CharacterValue::Armor as usize], 100);
    assert!(!world.items.contains_key(&ItemId(20)));
}

#[test]
fn nomad_dice_roll_uses_three_lucky_six_sided_dice() {
    let seed = 42;
    let luck = 2;
    let expected = [
        legacy_lucky_die_from_rolls(
            6,
            luck,
            (0..=luck).map(|offset| legacy_random(seed + u64::from(offset), 6) as u8 + 1),
        ),
        legacy_lucky_die_from_rolls(
            6,
            luck,
            (3..=5).map(|offset| legacy_random(seed + offset, 6) as u8 + 1),
        ),
        legacy_lucky_die_from_rolls(
            6,
            luck,
            (6..=8).map(|offset| legacy_random(seed + offset, 6) as u8 + 1),
        ),
    ];

    let (dice, total) = legacy_nomad_dice_roll(seed, luck);

    assert_eq!(dice, expected);
    assert_eq!(total, expected.iter().copied().sum::<u8>());
    assert!(dice.iter().all(|die| (1..=6).contains(die)));
}

#[test]
fn apply_orb_spawn_grants_orb_and_records_cooldown() {
    let character_id = CharacterId(7);
    let mut character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    character.flags.insert(CharacterFlags::PAID);
    let mut world = World::default();
    world.add_character(character);
    let mut spawner = test_item(ItemId(77), 123, ItemFlags::USED | ItemFlags::USE);
    spawner.x = 5;
    spawner.y = 6;
    world.add_item(spawner);
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(r#"empty_orb: name="Empty Orb" ;"#)
        .unwrap();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(character_id);

    assert_eq!(
        apply_orb_spawn(
            &mut world,
            &mut loader,
            Some(&mut player),
            ItemId(77),
            character_id,
            1,
            10_000,
            false,
            false,
            0,
        ),
        OrbSpawnApplyResult::Granted {
            item_name: "Orb of Endurance".to_string(),
            special: false,
        }
    );
    let character = world.characters.get(&character_id).unwrap();
    let orb_id = character.cursor_item.expect("orb should be on cursor");
    let orb = world.items.get(&orb_id).unwrap();
    assert_eq!(orb.name, "Orb of Endurance");
    assert_eq!(orb.driver_data[0], CharacterValue::Endurance as u8);
    assert_eq!(orb.driver_data[1], 1);
    assert_eq!(
        player.orb_spawn_last_used_seconds(0x0001_0605),
        Some(10_000)
    );
}

#[test]
fn apply_orb_spawn_enforces_legacy_respawn_cooldown() {
    let character_id = CharacterId(7);
    let character = login_character(character_id, &login_block("Tester"), 1, 10, 10);
    let mut world = World::default();
    world.add_character(character);
    let mut spawner = test_item(ItemId(77), 123, ItemFlags::USED | ItemFlags::USE);
    spawner.x = 5;
    spawner.y = 6;
    world.add_item(spawner);
    let mut loader = ZoneLoader::new();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(character_id);
    player.mark_orb_spawn_used(0x0001_0605, 10_000);

    assert_eq!(
        apply_orb_spawn(
            &mut world,
            &mut loader,
            Some(&mut player),
            ItemId(77),
            character_id,
            1,
            10_000 + 60 * 60 * 24,
            false,
            false,
            1,
        ),
        OrbSpawnApplyResult::Cooldown {
            days_left: "29.00".to_string(),
        }
    );
}

#[test]
fn apply_anti_orb_spawn_marks_extracting_anti_orb() {
    let character_id = CharacterId(7);
    let mut world = World::default();
    world.add_character(login_character(
        character_id,
        &login_block("Tester"),
        1,
        10,
        10,
    ));
    let mut spawner = test_item(ItemId(77), 123, ItemFlags::USED | ItemFlags::USE);
    spawner.x = 5;
    spawner.y = 6;
    world.add_item(spawner);
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(r#"empty_anti_orb: name="Empty Anti-Orb" ;"#)
        .unwrap();
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(character_id);

    assert_eq!(
        apply_orb_spawn(
            &mut world,
            &mut loader,
            Some(&mut player),
            ItemId(77),
            character_id,
            1,
            10_000,
            true,
            true,
            2,
        ),
        OrbSpawnApplyResult::Granted {
            item_name: "Extracting Anti-Orb of Mana".to_string(),
            special: true,
        }
    );
    let orb_id = world
        .characters
        .get(&character_id)
        .unwrap()
        .cursor_item
        .unwrap();
    let orb = world.items.get(&orb_id).unwrap();
    assert_eq!(orb.driver_data[0], CharacterValue::Mana as u8);
    assert_eq!(orb.driver_data[1], 1);
    assert_eq!(orb.driver_data[2], 1);
    assert_eq!(
        orb.description,
        "A dark orb that extracts Mana from items and crystallizes it."
    );
}

#[test]
fn ice_itemspawn_melting_key_starts_timer_when_granted() {
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
                melting_key:
                    name="Melting Key"
                    sprite=50494
                    flag=IF_TAKE
                    driver=52
                ;
                "#,
        )
        .unwrap();
    let mut world = World::default();
    world.add_character(login_character(
        CharacterId(7),
        &login_block("Tester"),
        10,
        10,
        10,
    ));

    assert_eq!(
        grant_ice_itemspawn_to_cursor(&mut world, &mut loader, CharacterId(7), "melting_key"),
        IceItemSpawnGrantResult::Granted {
            item_name: "Melting Key".to_string()
        }
    );

    let item_id = world
        .characters
        .get(&CharacterId(7))
        .unwrap()
        .cursor_item
        .unwrap();
    let item = world.items.get_mut(&item_id).unwrap();
    item.driver_data.resize(2, 0);
    item.driver_data[0] = 3;
    assert_eq!(world.timers.used_timers(), 1);

    world.tick.0 = TICKS_PER_SECOND * 10;
    let outcomes = world.process_due_timers(10);
    assert_eq!(outcomes.len(), 1);
    assert_eq!(world.items.get(&item_id).unwrap().driver_data[1], 1);
    assert_eq!(world.timers.used_timers(), 1);
}

#[test]
fn ice_itemspawn_rejects_duplicate_onecarry_like_c_can_carry() {
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
                palace_bomb:
                    name="Palace Bomb"
                    sprite=50496
                    flag=IF_TAKE
                    driver=56
                ;
                "#,
        )
        .unwrap();
    let mut world = World::default();
    let mut character = login_character(CharacterId(7), &login_block("Tester"), 10, 10, 10);
    character.inventory[30] = Some(ItemId(30));
    world.add_character(character);
    let mut existing = test_item(ItemId(30), 50496, ItemFlags::USED | ItemFlags::TAKE);
    existing.name = "Palace Bomb".to_string();
    existing.driver = ugaris_core::item_driver::IDR_PALACEBOMB;
    existing.carried_by = Some(CharacterId(7));
    world.add_item(existing);

    assert_eq!(
        grant_ice_itemspawn_to_cursor(&mut world, &mut loader, CharacterId(7), "palace_bomb"),
        IceItemSpawnGrantResult::OneCarry {
            item_name: "Palace Bomb".to_string()
        }
    );
    assert!(world
        .characters
        .get(&CharacterId(7))
        .unwrap()
        .cursor_item
        .is_none());
    assert_eq!(world.items.len(), 1);
}

#[test]
fn ice_itemspawn_silently_rejects_wrong_owner_bondtake_like_c_can_carry() {
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
                bonded_prize:
                    name="Bonded Prize"
                    sprite=50500
                    flag=IF_TAKE
                    flag=IF_BONDTAKE
                ;
                "#,
        )
        .unwrap();
    let mut world = World::default();
    world.add_character(login_character(
        CharacterId(7),
        &login_block("Tester"),
        10,
        10,
        10,
    ));

    assert_eq!(
        grant_ice_itemspawn_to_cursor(&mut world, &mut loader, CharacterId(7), "bonded_prize"),
        IceItemSpawnGrantResult::CannotCarry
    );
    assert!(world
        .characters
        .get(&CharacterId(7))
        .unwrap()
        .cursor_item
        .is_none());
    assert!(world.items.is_empty());
}

#[test]
fn junkpile_search_grants_steelbar_and_destroys_pile() {
    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(
            r#"
                steelbar:
                    name="Steel Bar"
                    sprite=321
                    flag=IF_TAKE
                ;
                "#,
        )
        .unwrap();
    let mut world = World::default();
    world.add_character(login_character(
        CharacterId(7),
        &login_block("Tester"),
        1,
        10,
        10,
    ));
    let mut pile = test_item(ItemId(70), 1, ItemFlags::USED | ItemFlags::USE);
    assert!(world.map.set_item_map(&mut pile, 10, 10));
    world.add_item(pile);

    let result = apply_junkpile_search(
        &mut world,
        &mut loader,
        ItemId(70),
        CharacterId(7),
        5,
        seed_for_legacy_random(10, 1),
    );

    assert_eq!(
        result,
        JunkpileApplyResult::Found {
            item_name: "Steel Bar".to_string(),
        }
    );
    assert!(!world.items.contains_key(&ItemId(70)));
    assert_eq!(world.map.tile(10, 10).unwrap().item, 0);
    let character = world.characters.get(&CharacterId(7)).unwrap();
    let item = world.items.get(&character.cursor_item.unwrap()).unwrap();
    assert_eq!(item.name, "Steel Bar");
    assert_eq!(item.carried_by, Some(CharacterId(7)));
}

#[test]
fn apply_assemble_item_replaces_used_item_and_consumes_cursor() {
    let character_id = CharacterId(7);
    let used_id = ItemId(70);
    let cursor_id = ItemId(71);
    let mut character = login_character(character_id, &login_block("Assembler"), 1, 10, 10);
    character.inventory[30] = Some(used_id);
    character.cursor_item = Some(cursor_id);

    let mut world = World::default();
    world.add_character(character);
    let mut used = test_item(used_id, 100, ItemFlags::USED | ItemFlags::USE);
    used.carried_by = Some(character_id);
    world.add_item(used);
    let mut cursor = test_item(cursor_id, 101, ItemFlags::USED | ItemFlags::TAKE);
    cursor.carried_by = Some(character_id);
    world.add_item(cursor);

    let mut loader = ZoneLoader::new();
    loader
        .load_item_templates_str(r#"sun_amulet123: name="Sun Amulet" sprite=444 ;"#)
        .unwrap();

    assert_eq!(
        apply_assemble_item(
            &mut world,
            &mut loader,
            used_id,
            character_id,
            cursor_id,
            "sun_amulet123",
        ),
        AssembleApplyResult::Assembled
    );

    let character = world.characters.get(&character_id).unwrap();
    let new_id = character.inventory[30].unwrap();
    assert_eq!(character.cursor_item, None);
    assert!(!world.items.contains_key(&used_id));
    assert!(!world.items.contains_key(&cursor_id));
    assert_eq!(world.items.get(&new_id).unwrap().name, "Sun Amulet");
}
