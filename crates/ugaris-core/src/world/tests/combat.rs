use super::*;

#[test]
fn attack_driver_direct_attacks_adjacent_target() {
    let mut world = World::default();
    let attacker = character(1);
    let target = character(2);
    assert!(world.spawn_character(attacker, 10, 10));
    assert!(world.spawn_character(target, 11, 10));

    assert!(world.attack_driver_direct(CharacterId(1), CharacterId(2), 1));

    let attacker = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(attacker.action, action::ATTACK1);
    assert_eq!(attacker.dir, Direction::Right as u8);
    assert_eq!(attacker.act1, 2);
}

#[test]
fn attack_driver_direct_attacks_moving_target_tile() {
    let mut world = World::default();
    let attacker = character(1);
    let mut target = character(2);
    target.tox = 11;
    target.toy = 10;
    assert!(world.spawn_character(attacker, 10, 10));
    assert!(world.spawn_character(target, 12, 10));
    world.map.tile_mut(12, 10).unwrap().light = 255;

    assert!(world.attack_driver_direct(CharacterId(1), CharacterId(2), 1));

    let attacker = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(attacker.action, action::ATTACK1);
    assert_eq!(attacker.dir, Direction::Right as u8);
    assert_eq!(attacker.act1, 2);
}

#[test]
fn attack_driver_direct_walks_one_step_on_complete_path() {
    let mut world = World::default();
    let attacker = character(1);
    let target = character(2);
    assert!(world.spawn_character(attacker, 10, 10));
    assert!(world.spawn_character(target, 13, 10));
    world.map.tile_mut(13, 10).unwrap().light = 255;

    assert!(world.attack_driver_direct(CharacterId(1), CharacterId(2), 1));

    let attacker = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(attacker.action, action::WALK);
    assert_eq!(attacker.dir, Direction::Right as u8);
    assert_eq!((attacker.tox, attacker.toy), (11, 10));
}

#[test]
fn attack_driver_direct_does_not_idle_or_best_partial_when_no_path_exists() {
    let mut world = World::default();
    let attacker = character(1);
    let target = character(2);
    assert!(world.spawn_character(attacker, 10, 10));
    assert!(world.spawn_character(target, 13, 10));
    world.map.tile_mut(13, 10).unwrap().light = 255;
    for (x, y) in [(11, 10), (9, 10), (10, 11), (10, 9)] {
        world
            .map
            .tile_mut(x, y)
            .unwrap()
            .flags
            .insert(MapFlags::MOVEBLOCK);
    }

    assert!(!world.attack_driver_direct(CharacterId(1), CharacterId(2), 1));

    let attacker = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(attacker.action, 0);
    assert_eq!((attacker.tox, attacker.toy), (0, 0));
}

#[test]
fn world_blocks_player_kill_setup_without_pk_hate_entry() {
    let mut world = World::default();
    let mut attacker = character(1);
    attacker
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
    attacker.x = 10;
    attacker.y = 10;
    let mut defender = character(2);
    defender
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
    defender.x = 11;
    defender.y = 10;
    world.map.tile_mut(11, 10).unwrap().character = 2;
    world.add_character(attacker);
    world.add_character(defender);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Kill,
        arg1: 2,
        arg2: 0,
    };

    assert!(world.apply_player_action_setup(&mut player, 2));
    let attacker = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(attacker.action, action::IDLE);
    assert_eq!(player.action.action, PlayerActionCode::Idle);
}

#[test]
fn world_removes_stale_pk_hate_when_pvp_level_check_fails() {
    let mut world = World::default();
    let mut attacker = character(1);
    attacker
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
    attacker.level = 10;
    attacker.x = 10;
    attacker.y = 10;
    let mut defender = character(2);
    defender
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
    defender.level = 14;
    defender.x = 11;
    defender.y = 10;
    world.map.tile_mut(11, 10).unwrap().character = 2;
    world.add_character(attacker);
    world.add_character(defender);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    assert!(player.add_pk_hate(2));
    player.action = QueuedAction {
        action: PlayerActionCode::Kill,
        arg1: 2,
        arg2: 0,
    };

    assert!(world.apply_player_action_setup(&mut player, 2));

    assert!(!player.has_pk_hate_for(2));
    assert_eq!(player.action.action, PlayerActionCode::Idle);
}

#[test]
fn world_keeps_pk_hate_when_area_one_blocks_pvp() {
    let mut world = World::default();
    let mut attacker = character(1);
    attacker
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
    attacker.level = 10;
    attacker.x = 10;
    attacker.y = 10;
    let mut defender = character(2);
    defender
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
    defender.level = 10;
    defender.x = 11;
    defender.y = 10;
    world.map.tile_mut(11, 10).unwrap().character = 2;
    world.add_character(attacker);
    world.add_character(defender);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    assert!(player.add_pk_hate(2));
    player.action = QueuedAction {
        action: PlayerActionCode::Kill,
        arg1: 2,
        arg2: 0,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));

    assert!(player.has_pk_hate_for(2));
    assert_eq!(player.action.action, PlayerActionCode::Idle);
}

#[test]
fn world_allows_player_kill_setup_with_pk_hate_entry() {
    let mut world = World::default();
    let mut attacker = character(1);
    attacker
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
    attacker.x = 10;
    attacker.y = 10;
    let mut defender = character(2);
    defender
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
    defender.x = 11;
    defender.y = 10;
    world.map.tile_mut(11, 10).unwrap().character = 2;
    world.add_character(attacker);
    world.add_character(defender);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    assert!(player.add_pk_hate(2));
    player.action = QueuedAction {
        action: PlayerActionCode::Kill,
        arg1: 2,
        arg2: 0,
    };

    assert!(world.apply_player_action_setup(&mut player, 2));
    let attacker = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(attacker.action, action::ATTACK1);
    assert_eq!(attacker.act1, 2);
}

#[test]
fn world_completes_attack_action_with_damage() {
    let mut world = World::default();
    let mut attacker = character(1);
    attacker.flags.insert(CharacterFlags::PLAYER);
    attacker.x = 10;
    attacker.y = 10;
    attacker.dir = Direction::Right as u8;
    attacker.action = action::ATTACK1;
    attacker.duration = 1;
    attacker.act1 = 2;
    attacker.values[0][CharacterValue::Attack as usize] = 10;
    attacker.values[0][CharacterValue::Weapon as usize] = 10;
    let mut defender = character(2);
    defender.x = 11;
    defender.y = 10;
    defender.dir = Direction::Left as u8;
    defender.hp = 10_000;
    defender.values[0][CharacterValue::Parry as usize] = 10;
    world.map.tile_mut(11, 10).unwrap().character = 2;
    world.add_character(attacker);
    world.add_character(defender);

    let completed = world.tick_basic_actions();
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].action_id, action::ATTACK1);
    assert!(completed[0].ok);
    let defender = world.characters.get(&CharacterId(2)).unwrap();
    assert!(defender.hp < 10_000);
    assert_eq!(defender.driver_messages[0].message_type, NT_GOTHIT);
    assert_eq!(defender.driver_messages[0].dat1, 1);
    assert_eq!(
        world.characters[&CharacterId(1)].driver_messages[0].message_type,
        NT_DIDHIT
    );
    assert_eq!(
        world.drain_pending_sound_specials()[0].special.special_type,
        7
    );
}

#[test]
fn character_fireball_blocks_player_target_without_pk_hate_entry() {
    let mut world = World::default();
    let mut caster = character(1);
    caster
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
    caster.mana = 10 * POWERSCALE;
    caster.values[0][CharacterValue::Fireball as usize] = 50;
    let mut target = character(2);
    target
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 15, 10);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::FireballCharacter,
        arg1: 2,
        arg2: 2,
    };

    assert!(world.apply_player_action_setup(&mut player, 2));

    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(caster.action, action::IDLE);
    assert_eq!(caster.mana, 10 * POWERSCALE);
    assert_eq!(player.action.action, PlayerActionCode::Idle);
}

#[test]
fn character_fireball_allows_player_target_with_pk_hate_entry() {
    let mut world = World::default();
    let mut caster = character(1);
    caster
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
    caster.mana = 10 * POWERSCALE;
    caster.values[0][CharacterValue::Fireball as usize] = 50;
    let mut target = character(2);
    target
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 15, 10);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    assert!(player.add_pk_hate(2));
    player.action = QueuedAction {
        action: PlayerActionCode::FireballCharacter,
        arg1: 2,
        arg2: 2,
    };

    assert!(world.apply_player_action_setup(&mut player, 2));

    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(caster.action, action::FIREBALL1);
    assert_eq!((caster.act1, caster.act2), (15, 10));
}

#[test]
fn character_ball_blocks_player_target_without_pk_hate_entry() {
    let mut world = World::default();
    let mut caster = character(1);
    caster
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
    caster.mana = 10 * POWERSCALE;
    caster.values[0][CharacterValue::Flash as usize] = 50;
    let mut target = character(2);
    target
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 15, 10);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::BallCharacter,
        arg1: 2,
        arg2: 2,
    };

    assert!(world.apply_player_action_setup(&mut player, 2));

    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(caster.action, action::IDLE);
    assert_eq!(caster.mana, 10 * POWERSCALE);
    assert_eq!(player.action.action, PlayerActionCode::Idle);
}

#[test]
fn completed_attack_notifies_nearby_characters_with_nt_char_on_hit_and_miss() {
    // C `act_attack` (act.c:763-793): `notify_area(ch[cn].x, ch[cn].y,
    // NT_CHAR, cn, 0, 0)` fires from the attacker's position after
    // `sub_attack`, regardless of whether the attack rolled a hit or a miss.
    let mut world = World::default();
    let mut attacker = character(1);
    attacker.flags.insert(CharacterFlags::PLAYER);
    attacker.x = 10;
    attacker.y = 10;
    attacker.dir = Direction::Right as u8;
    attacker.act1 = 2;
    attacker.values[0][CharacterValue::Attack as usize] = 10;
    let mut defender = character(2);
    defender.x = 11;
    defender.y = 10;
    defender.hp = 1_000_000;
    defender.values[0][CharacterValue::Parry as usize] = 10;
    let mut bystander = character(3);
    bystander.x = 12;
    bystander.y = 10;
    world.spawn_character(attacker, 10, 10);
    world.spawn_character(defender, 11, 10);
    world.spawn_character(bystander, 12, 10);

    // Hits (roll 1 < hit_chance). The bystander also receives an unrelated
    // `NT_SEEHIT` from `apply_legacy_hurt` (C `hurt()`'s own unconditional
    // area notify) since it sits within that call's 16-tile radius, so this
    // only asserts the `NT_CHAR` message specifically is present exactly
    // once.
    assert!(world.complete_attack_with_rolls(CharacterId(1), CharacterId(2), 1, 1));
    let bystander = world.characters.get(&CharacterId(3)).unwrap();
    let nt_char: Vec<_> = bystander
        .driver_messages
        .iter()
        .filter(|message| message.message_type == NT_CHAR)
        .collect();
    assert_eq!(nt_char.len(), 1);
    assert_eq!(nt_char[0].dat1, 1);
    world
        .characters
        .get_mut(&CharacterId(3))
        .unwrap()
        .driver_messages
        .clear();

    // Misses (roll 100 >= any hit_chance): `apply_legacy_hurt` isn't called
    // on a miss, so only the `NT_CHAR` message is queued.
    assert!(world.complete_attack_with_rolls(CharacterId(1), CharacterId(2), 100, 1));
    let bystander = world.characters.get(&CharacterId(3)).unwrap();
    assert_eq!(bystander.driver_messages.len(), 1);
    assert_eq!(bystander.driver_messages[0].message_type, NT_CHAR);
    assert_eq!(bystander.driver_messages[0].dat1, 1);
}

#[test]
fn completed_attack_skips_notify_when_cf_nonotify_set() {
    // C `act_attack`: `if (!(ch[cn].flags & CF_NONOTIFY)) notify_area(...)`.
    let mut world = World::default();
    let mut attacker = character(1);
    attacker
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::NONOTIFY);
    attacker.x = 10;
    attacker.y = 10;
    attacker.dir = Direction::Right as u8;
    attacker.act1 = 2;
    attacker.values[0][CharacterValue::Attack as usize] = 10;
    let mut defender = character(2);
    defender.x = 11;
    defender.y = 10;
    defender.hp = 1_000_000;
    defender.values[0][CharacterValue::Parry as usize] = 10;
    let mut bystander = character(3);
    bystander.x = 12;
    bystander.y = 10;
    world.spawn_character(attacker, 10, 10);
    world.spawn_character(defender, 11, 10);
    world.spawn_character(bystander, 12, 10);

    // Roll a miss so `apply_legacy_hurt` (and its unrelated, unconditional
    // `NT_SEEHIT` broadcast) never runs, isolating the `NT_CHAR` gate.
    assert!(world.complete_attack_with_rolls(CharacterId(1), CharacterId(2), 100, 1));

    let bystander = world.characters.get(&CharacterId(3)).unwrap();
    assert!(bystander.driver_messages.is_empty());
}
