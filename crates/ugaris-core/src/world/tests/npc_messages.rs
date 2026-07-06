use super::*;

#[test]
fn simple_baddy_message_actions_use_inventory_hp_potion() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.hp = 40 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 100;
    npc.values[1][CharacterValue::Hp as usize] = 100;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        drink_inventory_potions: 1,
        ..SimpleBaddyDriverData::default()
    }));
    npc.push_driver_message(NT_GOTHIT, 0, 0, 0);
    npc.inventory[30] = Some(ItemId(7));
    let mut potion = item(7, ItemFlags::USED | ItemFlags::USE);
    potion.carried_by = Some(CharacterId(1));
    potion.driver = IDR_POTION;
    potion.driver_data = vec![0, 20, 0, 0];
    world.add_character(npc);
    world.items.insert(ItemId(7), potion);

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert_eq!(
        outcomes,
        vec![ItemDriverOutcome::PotionDrunk {
            item_id: ItemId(7),
            character_id: CharacterId(1),
            hp_added: 20 * POWERSCALE,
            mana_added: 0,
            endurance_added: 0,
        }]
    );
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.hp, 60 * POWERSCALE);
    assert_eq!(npc.inventory[30], None);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.last_hit, world.tick.0 as i32);
    assert!(npc.driver_messages.is_empty());
}

#[test]
fn simple_baddy_message_actions_wait_until_current_action_completes() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.action = action::WALK;
    npc.hp = 40 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 100;
    npc.values[1][CharacterValue::Hp as usize] = 100;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        drink_inventory_potions: 1,
        ..SimpleBaddyDriverData::default()
    }));
    npc.push_driver_message(NT_GOTHIT, 0, 0, 0);
    npc.inventory[30] = Some(ItemId(7));
    let mut potion = item(7, ItemFlags::USED | ItemFlags::USE);
    potion.carried_by = Some(CharacterId(1));
    potion.driver = IDR_POTION;
    potion.driver_data = vec![0, 20, 0, 0];
    world.add_character(npc);
    world.items.insert(ItemId(7), potion);

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert!(outcomes.is_empty());
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.hp, 40 * POWERSCALE);
    assert_eq!(npc.inventory[30], Some(ItemId(7)));
    assert_eq!(npc.driver_messages.len(), 1);
}

#[test]
fn simple_baddy_message_actions_skip_when_drink_inventory_potions_disabled() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.hp = 40 * POWERSCALE;
    npc.values[0][CharacterValue::Hp as usize] = 100;
    npc.values[1][CharacterValue::Hp as usize] = 100;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    npc.push_driver_message(NT_GOTHIT, 0, 0, 0);
    npc.inventory[30] = Some(ItemId(7));
    let mut potion = item(7, ItemFlags::USED | ItemFlags::USE);
    potion.carried_by = Some(CharacterId(1));
    potion.driver = IDR_POTION;
    potion.driver_data = vec![0, 20, 0, 0];
    world.add_character(npc);
    world.items.insert(ItemId(7), potion);

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert!(outcomes.is_empty());
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.hp, 40 * POWERSCALE);
    assert_eq!(npc.inventory[30], Some(ItemId(7)));
    assert!(npc.driver_messages.is_empty());
}

#[test]
fn simple_baddy_message_actions_remember_helper_bless_for_noncombat_flow() {
    let mut world = World::default();
    world.tick = Tick(1_000);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.group = 7;
    npc.mana = 10 * POWERSCALE;
    npc.values[0][CharacterValue::Bless as usize] = 40;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        helper: 1,
        ..SimpleBaddyDriverData::default()
    }));
    npc.push_driver_message(NT_CHAR, 2, 0, 0);
    let mut existing_bless = item(20, ItemFlags::empty());
    existing_bless.driver = IDR_BLESS;
    npc.inventory[SPELL_SLOT_START] = Some(existing_bless.id);
    let mut friend = character(2);
    friend.group = 7;
    world.items.insert(existing_bless.id, existing_bless);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(friend, 11, 10);

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, 0);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.pending_bless_friend, Some(CharacterId(2)));
    assert!(npc.driver_messages.is_empty());

    assert!(world.process_simple_baddy_noncombat_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::BLESS1);
    assert_eq!(npc.act1, 2);
    assert_eq!(npc.mana, 8 * POWERSCALE);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.pending_bless_friend, None);
}

#[test]
fn simple_baddy_message_actions_reject_helper_bless_for_other_group() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.group = 7;
    npc.mana = 10 * POWERSCALE;
    npc.values[0][CharacterValue::Bless as usize] = 40;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        helper: 1,
        ..SimpleBaddyDriverData::default()
    }));
    npc.push_driver_message(NT_CHAR, 2, 0, 0);
    let mut other = character(2);
    other.group = 8;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(other, 12, 10);

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, 0);
    assert_eq!(npc.mana, 10 * POWERSCALE);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.pending_bless_friend, None);
    assert!(npc.driver_messages.is_empty());
}

#[test]
fn simple_baddy_message_actions_reject_helper_bless_without_visibility() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.group = 7;
    npc.mana = 10 * POWERSCALE;
    npc.values[0][CharacterValue::Bless as usize] = 40;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        helper: 1,
        ..SimpleBaddyDriverData::default()
    }));
    npc.push_driver_message(NT_CHAR, 2, 0, 0);
    let mut friend = character(2);
    friend.group = 7;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(friend, 12, 10);
    world.map.tile_mut(12, 10).unwrap().light = 32;
    world.map.set_flags(11, 10, MapFlags::SIGHTBLOCK);

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, 0);
    assert_eq!(npc.mana, 10 * POWERSCALE);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.pending_bless_friend, None);
    assert!(npc.driver_messages.is_empty());
}

#[test]
fn simple_baddy_message_actions_keep_last_valid_helper_bless_candidate() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.group = 7;
    npc.mana = 10 * POWERSCALE;
    npc.values[0][CharacterValue::Bless as usize] = 40;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        helper: 1,
        pending_bless_friend: Some(CharacterId(99)),
        ..SimpleBaddyDriverData::default()
    }));
    npc.push_driver_message(NT_CHAR, 2, 0, 0);
    npc.push_driver_message(NT_CHAR, 3, 0, 0);
    let mut friend = character(2);
    friend.group = 7;
    let mut enemy = character(3);
    enemy.group = 8;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(friend, 11, 10);
    world.spawn_character(enemy, 12, 10);

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert_eq!(
        outcomes,
        vec![ItemDriverOutcome::Noop, ItemDriverOutcome::Noop]
    );
    let npc = world.characters.get(&CharacterId(1)).unwrap();
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.pending_bless_friend, Some(CharacterId(2)));
}

#[test]
fn simple_baddy_message_actions_poison_successful_hit() {
    let mut world = World::default();
    world.tick = Tick(1_000);
    let mut npc = character(1);
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        poison_power: 6,
        poison_type: 2,
        poison_chance: 50,
        ..SimpleBaddyDriverData::default()
    }));
    npc.push_driver_message(NT_DIDHIT, 2, 10, 0);
    let mut target = character(2);
    target.values[1][CharacterValue::Hp as usize] = 100;
    target.hp = 100 * POWERSCALE;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);

    let outcomes =
        world.process_simple_baddy_message_actions_with_random(CharacterId(1), 1, |_| 49);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    let target = world.characters.get(&CharacterId(2)).unwrap();
    let poison_id = target.inventory[29].expect("poison spell item");
    let poison = world.items.get(&poison_id).unwrap();
    assert_eq!(poison.driver, IDR_POISON0 + 2);
    assert!(target.flags.contains(CharacterFlags::UPDATE));
    assert!(world.characters[&CharacterId(1)].driver_messages.is_empty());
}

#[test]
fn simple_baddy_message_actions_default_path_uses_legacy_rng_seed_for_poison() {
    let mut world = World::default();
    world.tick = Tick(1_000);
    world.legacy_random_seed = 0;
    let mut npc = character(1);
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        poison_power: 6,
        poison_type: 1,
        poison_chance: 50,
        ..SimpleBaddyDriverData::default()
    }));
    npc.push_driver_message(NT_DIDHIT, 2, 10, 0);
    let mut target = character(2);
    target.values[1][CharacterValue::Hp as usize] = 100;
    target.hp = 100 * POWERSCALE;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    assert_eq!(world.legacy_random_seed, 12_345);
    let target = world.characters.get(&CharacterId(2)).unwrap();
    let poison_id = target.inventory[29].expect("poison spell item");
    assert_eq!(world.items.get(&poison_id).unwrap().driver, IDR_POISON0 + 1);
}

#[test]
fn simple_baddy_message_actions_poison_respects_chance_and_attack_policy() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        poison_power: 6,
        poison_type: 2,
        poison_chance: 50,
        ..SimpleBaddyDriverData::default()
    }));
    npc.push_driver_message(NT_DIDHIT, 2, 10, 0);
    let mut target = character(2);
    target.flags.insert(CharacterFlags::NOATTACK);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);

    let outcomes = world.process_simple_baddy_message_actions_with_random(CharacterId(1), 1, |_| 0);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    assert!(world.characters[&CharacterId(2)].inventory[29].is_none());

    world
        .characters
        .get_mut(&CharacterId(2))
        .unwrap()
        .flags
        .remove(CharacterFlags::NOATTACK);
    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .push_driver_message(NT_DIDHIT, 2, 10, 0);

    let outcomes =
        world.process_simple_baddy_message_actions_with_random(CharacterId(1), 1, |_| 50);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    assert!(world.characters[&CharacterId(2)].inventory[29].is_none());
}

#[test]
fn simple_baddy_message_actions_add_npc_alert_enemy_for_same_group_caller() {
    let mut world = World::default();
    world.tick = Tick(123);
    let mut npc = character(1);
    npc.group = 7;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        helpid: NTID_GLADIATOR,
        ..SimpleBaddyDriverData::default()
    }));
    npc.push_driver_message(NT_NPC, NTID_GLADIATOR, 2, 99);
    let mut caller = character(2);
    caller.group = 7;
    world.add_character(npc);
    world.add_character(caller);

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    let data = world.characters[&CharacterId(1)]
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert_eq!(
        data.enemies,
        vec![SimpleBaddyEnemy {
            target_id: CharacterId(99),
            priority: 1,
            last_seen_tick: 123,
            visible: false,
            last_x: 0,
            last_y: 0,
        }]
    );
}

#[test]
fn simple_baddy_npc_alert_enemy_is_recorded_hidden_like_c() {
    let mut world = World::default();
    world.tick = Tick(124);
    let mut npc = character(1);
    npc.group = 7;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        helpid: NTID_GLADIATOR,
        ..SimpleBaddyDriverData::default()
    }));
    npc.push_driver_message(NT_NPC, NTID_GLADIATOR, 2, 3);
    let mut caller = character(2);
    caller.group = 7;
    let target = character(3);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(caller, 10, 11);
    world.spawn_character(target, 11, 10);
    world.map.tile_mut(11, 10).unwrap().light = 255;

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    let data = world.characters[&CharacterId(1)]
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert_eq!(
        data.enemies,
        vec![SimpleBaddyEnemy {
            target_id: CharacterId(3),
            priority: 1,
            last_seen_tick: 124,
            visible: false,
            last_x: 11,
            last_y: 10,
        }]
    );
}

#[test]
fn simple_baddy_message_actions_remove_dead_enemy() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![
            SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 10,
                visible: true,
                last_x: 11,
                last_y: 10,
            },
            SimpleBaddyEnemy {
                target_id: CharacterId(3),
                priority: 0,
                last_seen_tick: 11,
                visible: true,
                last_x: 12,
                last_y: 10,
            },
        ],
        ..SimpleBaddyDriverData::default()
    }));
    npc.push_driver_message(NT_DEAD, 2, 99, 0);
    world.add_character(npc);

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    let data = world.characters[&CharacterId(1)]
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert_eq!(data.enemies.len(), 1);
    assert_eq!(data.enemies[0].target_id, CharacterId(3));
}

#[test]
fn simple_baddy_message_actions_add_aggressive_seen_character_enemy() {
    let mut world = World::default();
    world.tick = Tick(321);
    let mut npc = character(1);
    npc.group = 7;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        aggressive: 1,
        ..SimpleBaddyDriverData::default()
    }));
    npc.push_driver_message(NT_CHAR, 2, 0, 0);
    let mut target = character(2);
    target.group = 8;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    let data = world.characters[&CharacterId(1)]
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert_eq!(
        data.enemies,
        vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 0,
            last_seen_tick: 321,
            visible: true,
            last_x: 11,
            last_y: 10,
        }]
    );
}

#[test]
fn simple_baddy_enemy_memory_sorts_and_caps_like_c_table() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: (2..14)
            .map(|id| SimpleBaddyEnemy {
                target_id: CharacterId(id),
                priority: if id == 12 { 1 } else { 0 },
                last_seen_tick: id as i32,
                visible: id != 13,
                last_x: 10 + id as u16,
                last_y: 10,
            })
            .collect(),
        ..SimpleBaddyDriverData::default()
    }));
    world.spawn_character(npc, 10, 10);
    for id in 2..14 {
        world.spawn_character(character(id), 10 + id as usize, 10);
    }

    world.sort_simple_baddy_enemies_like_c(CharacterId(1));

    let data = world.characters[&CharacterId(1)]
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert_eq!(data.enemies.len(), 10);
    assert_eq!(data.enemies[0].target_id, CharacterId(12));
    assert_eq!(data.enemies[1].target_id, CharacterId(2));
    assert!(!data
        .enemies
        .iter()
        .any(|enemy| enemy.target_id == CharacterId(13)));
}

#[test]
fn simple_baddy_message_actions_rejects_enemy_outside_start_or_char_distance() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.group = 7;
    npc.rest_x = 10;
    npc.rest_y = 10;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        aggressive: 1,
        startdist: 6,
        chardist: 4,
        ..SimpleBaddyDriverData::default()
    }));
    npc.push_driver_message(NT_CHAR, 2, 0, 0);
    let mut target = character(2);
    target.group = 8;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 14, 10);

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    let Some(CharacterDriverState::SimpleBaddy(data)) =
        world.characters[&CharacterId(1)].driver_state.as_ref()
    else {
        panic!("simple baddy state missing");
    };
    assert!(data.enemies.is_empty());
}

#[test]
fn simple_baddy_message_actions_use_explicit_fight_driver_home_for_start_distance() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.group = 7;
    npc.rest_x = 10;
    npc.rest_y = 10;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        aggressive: 1,
        startdist: 6,
        ..SimpleBaddyDriverData::default()
    }));
    npc.push_driver_message(NT_CHAR, 2, 0, 0);
    let mut target = character(2);
    target.group = 8;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 14, 10);
    world.map.tile_mut(14, 10).unwrap().light = 255;
    assert!(world.set_simple_baddy_home(CharacterId(1), 14, 10));

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    let data = world.characters[&CharacterId(1)]
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert_eq!(data.home_x, 14);
    assert_eq!(data.home_y, 10);
    assert_eq!(data.enemies.len(), 1);
    assert_eq!(data.enemies[0].target_id, CharacterId(2));
}

#[test]
fn simple_baddy_message_actions_rejects_non_hurt_enemy_in_neutral_zone() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.group = 7;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        aggressive: 1,
        ..SimpleBaddyDriverData::default()
    }));
    npc.push_driver_message(NT_CHAR, 2, 0, 0);
    let mut target = character(2);
    target.group = 8;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);
    world
        .map
        .tile_mut(11, 10)
        .unwrap()
        .flags
        .insert(MapFlags::NEUTRAL);

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    let Some(CharacterDriverState::SimpleBaddy(data)) =
        world.characters[&CharacterId(1)].driver_state.as_ref()
    else {
        panic!("simple baddy state missing");
    };
    assert!(data.enemies.is_empty());
}

#[test]
fn simple_baddy_message_actions_keeps_hurt_enemy_in_neutral_zone() {
    let mut world = World::default();
    world.tick = Tick(324);
    let mut npc = character(1);
    npc.group = 7;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    npc.push_driver_message(NT_GOTHIT, 2, 10, 0);
    let mut attacker = character(2);
    attacker.group = 8;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(attacker, 11, 10);
    world
        .map
        .tile_mut(11, 10)
        .unwrap()
        .flags
        .insert(MapFlags::NEUTRAL);

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    let data = world.characters[&CharacterId(1)]
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert_eq!(
        data.enemies,
        vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 324,
            visible: true,
            last_x: 11,
            last_y: 10,
        }]
    );
}

#[test]
fn simple_baddy_message_actions_keeps_hurt_enemy_outside_start_or_char_distance() {
    let mut world = World::default();
    world.tick = Tick(325);
    let mut npc = character(1);
    npc.group = 7;
    npc.rest_x = 10;
    npc.rest_y = 10;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        startdist: 6,
        chardist: 4,
        ..SimpleBaddyDriverData::default()
    }));
    npc.push_driver_message(NT_GOTHIT, 2, 10, 0);
    let mut attacker = character(2);
    attacker.group = 8;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(attacker, 14, 10);

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    let data = world.characters[&CharacterId(1)]
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert_eq!(data.last_hit, 325);
    assert_eq!(data.enemies.len(), 1);
    assert_eq!(data.enemies[0].target_id, CharacterId(2));
    assert_eq!(data.enemies[0].priority, 1);
}

#[test]
fn simple_baddy_message_actions_rejects_legacy_out_of_map_enemy_coordinate() {
    let mut world = World::default();
    world.tick = Tick(326);
    let mut npc = character(1);
    npc.group = 7;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    npc.push_driver_message(NT_GOTHIT, 2, 10, 0);
    let mut attacker = character(2);
    attacker.group = 8;
    attacker.x = 0;
    attacker.y = 10;
    world.spawn_character(npc, 10, 10);
    world.add_character(attacker);

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    let data = world.characters[&CharacterId(1)]
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert_eq!(data.last_hit, 326);
    assert!(data.enemies.is_empty());
}

#[test]
fn simple_baddy_attack_keeps_hidden_enemy_beyond_stopdist_like_c() {
    let mut world = World::default();
    world.tick = Tick(326);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.rest_x = 10;
    npc.rest_y = 10;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        stopdist: 3,
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 300,
            visible: false,
            last_x: 10,
            last_y: 12,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let mut target = character(2);
    target.group = 8;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 10, 20);
    world
        .map
        .tile_mut(10, 15)
        .unwrap()
        .flags
        .insert(MapFlags::SIGHTBLOCK);

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!((npc.tox, npc.toy), (10, 11));
    let data = npc
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert_eq!(data.enemies.len(), 1);
    assert_eq!(data.enemies[0].target_id, CharacterId(2));
    assert!(!data.enemies[0].visible);
}

#[test]
fn simple_baddy_attack_drops_visible_enemy_beyond_stopdist_like_c() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.rest_x = 10;
    npc.rest_y = 10;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        stopdist: 3,
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 300,
            visible: true,
            last_x: 10,
            last_y: 14,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let mut target = character(2);
    target.group = 8;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 10, 14);
    world.map.tile_mut(10, 14).unwrap().light = 255;

    assert!(!world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let data = world.characters[&CharacterId(1)]
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert!(data.enemies.is_empty());
}

#[test]
fn simple_baddy_message_actions_add_defensive_gothit_enemy_without_sight() {
    let mut world = World::default();
    world.tick = Tick(322);
    let mut npc = character(1);
    npc.group = 7;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(
        SimpleBaddyDriverData::default(),
    ));
    npc.push_driver_message(NT_GOTHIT, 2, 10, 0);
    let mut attacker = character(2);
    attacker.group = 8;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(attacker, 12, 10);
    world
        .map
        .tile_mut(11, 10)
        .unwrap()
        .flags
        .insert(MapFlags::SIGHTBLOCK);

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    let data = world.characters[&CharacterId(1)]
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert_eq!(data.last_hit, 322);
    assert_eq!(
        data.enemies,
        vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 322,
            visible: false,
            last_x: 12,
            last_y: 10,
        }]
    );
}

#[test]
fn simple_baddy_message_actions_helper_seen_hit_adds_enemy_for_friend() {
    let mut world = World::default();
    world.tick = Tick(323);
    let mut npc = character(1);
    npc.group = 7;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        helper: 1,
        ..SimpleBaddyDriverData::default()
    }));
    npc.push_driver_message(NT_SEEHIT, 2, 3, 0);
    let mut attacker = character(2);
    attacker.group = 8;
    let mut victim = character(3);
    victim.group = 7;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(attacker, 11, 10);
    world.spawn_character(victim, 12, 10);

    let outcomes = world.process_simple_baddy_message_actions(CharacterId(1), 1);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    let data = world.characters[&CharacterId(1)]
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert_eq!(
        data.enemies,
        vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 323,
            visible: true,
            last_x: 11,
            last_y: 10,
        }]
    );
}

#[test]
fn simple_baddy_attack_action_uses_firering_against_adjacent_recorded_enemy() {
    let mut world = World::default();
    world.tick = Tick(455);
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
    assert_eq!(npc.action, action::FIRERING);
    assert_eq!(npc.mana, 0);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 455);
}

#[test]
fn simple_baddy_attack_action_uses_fireball_against_visible_recorded_enemy() {
    let mut world = World::default();
    world.tick = Tick(455);
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
    assert_eq!(npc.action, action::FIREBALL1);
    assert_eq!(npc.act1, 15);
    assert_eq!(npc.act2, 10);
    assert_eq!(npc.dir, Direction::Right as u8);
    assert_eq!(npc.mana, 0);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 455);
}

#[test]
fn simple_baddy_fireball_line_accepts_recorded_enemy_blast() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![
            SimpleBaddyEnemy {
                target_id: CharacterId(2),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 15,
                last_y: 10,
            },
            SimpleBaddyEnemy {
                target_id: CharacterId(3),
                priority: 1,
                last_seen_tick: 123,
                visible: true,
                last_x: 12,
                last_y: 11,
            },
        ],
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

    assert!(world.fireball_line_hits_target(CharacterId(1), CharacterId(2), 10, 10, 15, 10));
}

#[test]
fn simple_baddy_attack_action_uses_freeze_against_close_recorded_enemy() {
    let mut world = World::default();
    world.tick = Tick(458);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = FREEZE_COST;
    npc.values[0][CharacterValue::Freeze as usize] = 20;
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
    assert_eq!(npc.action, action::FREEZE);
    assert_eq!(npc.mana, 0);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 458);
}

#[test]
fn simple_baddy_attack_action_uses_flash_against_close_recorded_enemy() {
    let mut world = World::default();
    world.tick = Tick(459);
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
    assert_eq!(npc.action, action::FLASH);
    assert_eq!(npc.mana, 0);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 459);
}

#[test]
fn simple_baddy_attack_action_uses_ball_against_distant_recorded_enemy() {
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

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::BALL1);
    assert_eq!(npc.act1, 16);
    assert_eq!(npc.act2, 9);
    assert_eq!(npc.mana, 0);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 461);
}

#[test]
fn simple_baddy_attack_action_uses_pulse_when_nearby_enemy_is_finishable() {
    let mut world = World::default();
    world.tick = Tick(462);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.mana = POWERSCALE + 1;
    npc.values[0][CharacterValue::Mana as usize] = 1;
    npc.values[0][CharacterValue::Pulse as usize] = 2_000;
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
    target.flags.insert(CharacterFlags::ALIVE);
    target.hp = POWERSCALE + 100;
    target.lifeshield = 0;
    target.values[0][CharacterValue::Hp as usize] = 100;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 12, 10);
    world.map.tile_mut(12, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::PULSE);
    assert_eq!(npc.mana, 1);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 462);
}

#[test]
fn simple_baddy_attack_action_attacks_visible_adjacent_recorded_enemy() {
    let mut world = World::default();
    world.tick = Tick(456);
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
            last_x: 11,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let mut target = character(2);
    target.values[0][CharacterValue::Attack as usize] = 1;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 11, 10);

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::ATTACK1);
    assert_eq!(npc.act1, 2);
    assert_eq!(npc.dir, Direction::Right as u8);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 456);
}

#[test]
fn simple_baddy_attack_action_prefers_c_visible_enemy_score_over_priority() {
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
                priority: 99,
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
                last_x: 11,
                last_y: 10,
            },
        ],
        ..SimpleBaddyDriverData::default()
    }));
    let mut far_target = character(2);
    far_target.values[0][CharacterValue::Attack as usize] = 1;
    let mut close_target = character(3);
    close_target.values[0][CharacterValue::Attack as usize] = 1;
    world.spawn_character(npc, 10, 10);
    world.spawn_character(far_target, 14, 10);
    world.spawn_character(close_target, 11, 10);
    world.map.tile_mut(14, 10).unwrap().light = 255;
    world.map.tile_mut(11, 10).unwrap().light = 255;

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::ATTACK1);
    assert_eq!(npc.act1, 3);
    assert_eq!(npc.dir, Direction::Right as u8);
}

#[test]
fn simple_baddy_flee_action_moves_away_from_visible_enemy() {
    let mut world = World::default();
    world.tick = Tick(459);
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

    assert!(world.setup_simple_baddy_flee_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert!(npc.tox < 10);
    assert_eq!(npc.dir, Direction::Left as u8);
    assert_eq!(npc.speed_mode, SpeedMode::Fast);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 459);
}

#[test]
fn simple_baddy_flee_action_uses_stealth_when_enemy_is_distant() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: true,
            last_x: 20,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let target = character(2);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 20, 10);
    world.map.tile_mut(20, 10).unwrap().light = 255;

    assert!(world.setup_simple_baddy_flee_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.speed_mode, SpeedMode::Stealth);
    assert!(npc.tox < 10);
}

#[test]
fn simple_baddy_attack_action_removes_visible_enemy_past_stop_distance() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.rest_x = 10;
    npc.rest_y = 10;
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

    assert!(!world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let data = world.characters[&CharacterId(1)]
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert!(data.enemies.is_empty());
}

#[test]
fn simple_baddy_attack_action_follows_invisible_enemy_last_position() {
    let mut world = World::default();
    world.tick = Tick(468);
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.values[0][CharacterValue::Attack as usize] = 20;
    npc.values[0][CharacterValue::Speed as usize] = 50;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: false,
            last_x: 15,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let mut target = character(2);
    target.flags.insert(CharacterFlags::INVISIBLE);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 15, 10);

    assert!(world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let npc = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(npc.action, action::WALK);
    assert_eq!(npc.tox, 11);
    assert_eq!(npc.toy, 10);
    assert_eq!(npc.dir, Direction::Right as u8);
    let Some(CharacterDriverState::SimpleBaddy(data)) = npc.driver_state.as_ref() else {
        panic!("simple baddy state missing");
    };
    assert_eq!(data.lastfight, 468);
}

#[test]
fn simple_baddy_attack_action_drops_invisible_enemy_at_last_position() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.driver = CDR_SIMPLEBADDY;
    npc.driver_state = Some(CharacterDriverState::SimpleBaddy(SimpleBaddyDriverData {
        enemies: vec![SimpleBaddyEnemy {
            target_id: CharacterId(2),
            priority: 1,
            last_seen_tick: 123,
            visible: false,
            last_x: 10,
            last_y: 10,
        }],
        ..SimpleBaddyDriverData::default()
    }));
    let mut target = character(2);
    target.flags.insert(CharacterFlags::INVISIBLE);
    world.spawn_character(npc, 10, 10);
    world.spawn_character(target, 15, 10);

    assert!(!world.process_simple_baddy_attack_action(CharacterId(1), 1));

    let data = world.characters[&CharacterId(1)]
        .fight_driver
        .as_ref()
        .expect("fight driver state missing");
    assert!(data.enemies.is_empty());
}
