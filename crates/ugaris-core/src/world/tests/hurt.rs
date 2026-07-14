// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use super::*;

#[test]
fn legacy_hurt_applies_armor_lifeshield_and_hit_notifications() {
    let mut world = World::default();
    world.tick = Tick(1234);
    let mut target = character(1);
    target.hp = 5 * POWERSCALE;
    target.lifeshield = POWERSCALE;
    target.values[0][CharacterValue::Armor as usize] = 20;
    assert!(world.spawn_character(target, 10, 10));
    assert!(world.spawn_character(character(2), 11, 10));
    assert!(world.spawn_character(character(3), 12, 10));

    let outcome = world
        .apply_legacy_hurt(
            CharacterId(1),
            Some(CharacterId(2)),
            5 * POWERSCALE,
            5,
            90,
            75,
        )
        .unwrap();

    assert_eq!(outcome.damage_after_armor, 4_800);
    assert_eq!(outcome.shield_absorbed, POWERSCALE);
    assert_eq!(outcome.hp_damage, 3_800);
    let target = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(target.hp, 1_200);
    assert_eq!(target.lifeshield, 0);
    assert_eq!(target.regen_ticker, 1234);
    assert!(target.flags.contains(CharacterFlags::UPDATE));
    assert_eq!(target.driver_messages[0].message_type, NT_GOTHIT);
    assert_eq!(target.driver_messages[0].dat1, 2);
    assert_eq!(
        world.characters[&CharacterId(2)].driver_messages[0].message_type,
        NT_DIDHIT
    );
    assert_eq!(
        world.characters[&CharacterId(3)].driver_messages[0].message_type,
        NT_SEEHIT
    );
}

#[test]
fn legacy_hurt_queues_showattack_debug_text_when_enabled() {
    let mut world = World::default();
    world.show_attack_debug = true;
    let mut target = character(1);
    target.hp = 5 * POWERSCALE;
    target.values[0][CharacterValue::Armor as usize] = 20;
    assert!(world.spawn_character(target, 10, 10));
    assert!(world.spawn_character(character(2), 11, 10));

    world.apply_legacy_hurt(
        CharacterId(1),
        Some(CharacterId(2)),
        5 * POWERSCALE,
        5,
        90,
        75,
    );

    assert_eq!(
        world.drain_pending_system_texts(),
        vec![
            WorldSystemText {
                character_id: CharacterId(1),
                message: "hurt by Character, dam=5.00, armor=0.20 armorper=90 shieldper=75"
                    .to_string(),
            },
            WorldSystemText {
                character_id: CharacterId(1),
                message: "dam after armor: 4.80".to_string(),
            },
        ]
    );
}

#[test]
fn legacy_hurt_notifies_nearby_characters_on_death() {
    let mut world = World::default();
    let mut target = character(1);
    target.hp = POWERSCALE;
    assert!(world.spawn_character(target, 10, 10));
    assert!(world.spawn_character(character(2), 11, 10));
    assert!(world.spawn_character(character(3), 42, 10));
    assert!(world.spawn_character(character(4), 43, 10));

    let outcome = world
        .apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), POWERSCALE, 1, 0, 0)
        .unwrap();

    assert!(outcome.killed);
    for id in [1, 2, 3] {
        let character = world.characters.get(&CharacterId(id)).unwrap();
        let death_message = character
            .driver_messages
            .iter()
            .find(|message| message.message_type == NT_DEAD)
            .expect("nearby character should receive NT_DEAD");
        assert_eq!(death_message.dat1, 1);
        assert_eq!(death_message.dat2, 2);
        assert_eq!(death_message.dat3, 0);
    }
    assert!(world.characters[&CharacterId(4)]
        .driver_messages
        .iter()
        .all(|message| message.message_type != NT_DEAD));
}

#[test]
fn legacy_hurt_creates_magicshield_visual_on_shield_absorption() {
    let mut world = World::default();
    world.tick = Tick(77);
    let mut target = character(1);
    target.hp = 5 * POWERSCALE;
    target.lifeshield = POWERSCALE;
    target.values[1][CharacterValue::MagicShield as usize] = 10;
    assert!(world.spawn_character(target, 10, 10));

    let outcome = world
        .apply_legacy_hurt(CharacterId(1), None, POWERSCALE, 1, 0, 100)
        .unwrap();

    assert_eq!(outcome.shield_absorbed, POWERSCALE);
    let effect = world
        .effects
        .values()
        .find(|effect| effect.effect_type == EF_MAGICSHIELD)
        .unwrap();
    assert_eq!(effect.target_character, Some(CharacterId(1)));
    assert_eq!(effect.start_tick, 77);
    assert_eq!(effect.stop_tick, 80);
    assert_eq!(effect.light, 16);
    assert_eq!(effect.strength, 0);
}

#[test]
fn legacy_hurt_does_not_duplicate_active_magicshield_visual() {
    let mut world = World::default();
    let mut target = character(1);
    target.hp = 5 * POWERSCALE;
    target.lifeshield = 2 * POWERSCALE;
    target.values[1][CharacterValue::MagicShield as usize] = 10;
    assert!(world.spawn_character(target, 10, 10));
    world.create_show_effect(EF_MAGICSHIELD, CharacterId(1), 1, 4, 16, 0);

    world.apply_legacy_hurt(CharacterId(1), None, POWERSCALE, 1, 0, 100);

    assert_eq!(
        world
            .effects
            .values()
            .filter(|effect| effect.effect_type == EF_MAGICSHIELD)
            .count(),
        1
    );
}

#[test]
fn legacy_hurt_ports_immortal_and_nodeath_guards() {
    let mut world = World::default();
    let mut immortal = character(1);
    immortal.flags |= CharacterFlags::IMMORTAL;
    immortal.hp = POWERSCALE;
    immortal.lifeshield = POWERSCALE;
    assert!(world.spawn_character(immortal, 10, 10));

    let outcome = world
        .apply_legacy_hurt(CharacterId(1), None, 5 * POWERSCALE, 1, 0, 100)
        .unwrap();

    let immortal = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(outcome.hp_damage, 0);
    assert_eq!(immortal.hp, POWERSCALE);
    assert_eq!(immortal.lifeshield, POWERSCALE);

    let mut nodeath = character(2);
    nodeath.flags |= CharacterFlags::NODEATH;
    nodeath.hp = 700;
    assert!(world.spawn_character(nodeath, 11, 10));

    let outcome = world
        .apply_legacy_hurt(CharacterId(2), None, POWERSCALE, 1, 0, 0)
        .unwrap();

    let nodeath = world.characters.get(&CharacterId(2)).unwrap();
    assert!(outcome.nodeath_saved);
    assert_eq!(nodeath.hp, 1);
    assert!(!nodeath.flags.contains(CharacterFlags::DEAD));
}

#[test]
fn legacy_hurt_ports_fdemon_back_attack_gate() {
    let mut world = World::default();
    let mut target = character(1);
    target.flags |= CharacterFlags::FDEMON;
    target.dir = Direction::Right as u8;
    target.hp = 20 * POWERSCALE;
    assert!(world.spawn_character(target, 10, 10));
    assert!(world.spawn_character(character(2), 11, 10));

    let outcome = world
        .apply_legacy_hurt(
            CharacterId(1),
            Some(CharacterId(2)),
            10 * POWERSCALE,
            1,
            0,
            0,
        )
        .unwrap();

    assert_eq!(outcome.damage_after_armor, 10 * POWERSCALE);
    assert_eq!(outcome.hp_damage, 100);
    assert_eq!(world.characters[&CharacterId(1)].hp, 19_900);

    world.remove_character(CharacterId(2));
    assert!(world.spawn_character(character(2), 9, 10));

    let outcome = world
        .apply_legacy_hurt(
            CharacterId(1),
            Some(CharacterId(2)),
            10 * POWERSCALE,
            1,
            0,
            0,
        )
        .unwrap();

    assert_eq!(outcome.damage_after_armor, 10 * POWERSCALE);
    assert_eq!(outcome.hp_damage, 10 * POWERSCALE);
    assert_eq!(world.characters[&CharacterId(1)].hp, 9_900);
}

#[test]
fn legacy_hurt_ports_hardkill_weapon_gate() {
    let mut world = World::default();
    let mut target = character(1);
    target.flags |= CharacterFlags::HARDKILL;
    target.hp = 10 * POWERSCALE;
    target.level = 8;
    assert!(world.spawn_character(target, 10, 10));
    assert!(world.spawn_character(character(2), 11, 10));

    let outcome = world
        .apply_legacy_hurt(
            CharacterId(1),
            Some(CharacterId(2)),
            5 * POWERSCALE,
            1,
            0,
            0,
        )
        .unwrap();

    assert_eq!(outcome.damage_after_armor, 5 * POWERSCALE);
    assert_eq!(outcome.hp_damage, 0);
    assert_eq!(world.characters[&CharacterId(1)].hp, 10 * POWERSCALE);

    let mut weak_weapon = item(7, ItemFlags::USED | ItemFlags::SWORD);
    weak_weapon.template_id = IID_HARDKILL;
    weak_weapon.driver_data.resize(38, 0);
    weak_weapon.driver_data[37] = 7;
    world.items.insert(ItemId(7), weak_weapon);
    world.characters.get_mut(&CharacterId(2)).unwrap().inventory[worn_slot::RIGHT_HAND] =
        Some(ItemId(7));

    let outcome = world
        .apply_legacy_hurt(
            CharacterId(1),
            Some(CharacterId(2)),
            5 * POWERSCALE,
            1,
            0,
            0,
        )
        .unwrap();

    assert_eq!(outcome.hp_damage, 0);
    assert_eq!(world.characters[&CharacterId(1)].hp, 10 * POWERSCALE);

    world.items.get_mut(&ItemId(7)).unwrap().driver_data[37] = 8;

    let outcome = world
        .apply_legacy_hurt(
            CharacterId(1),
            Some(CharacterId(2)),
            5 * POWERSCALE,
            1,
            0,
            0,
        )
        .unwrap();

    assert_eq!(outcome.damage_after_armor, 5 * POWERSCALE);
    assert_eq!(outcome.hp_damage, 5 * POWERSCALE);
    assert_eq!(world.characters[&CharacterId(1)].hp, 5 * POWERSCALE);
}

#[test]
fn legacy_hurt_god_saves_player_with_unspent_saves_instead_of_killing() {
    let mut world = World::default();
    let mut target = character(1);
    target
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::ALIVE);
    target.hp = POWERSCALE;
    target.saves = 3;
    target.rest_x = 20;
    target.rest_y = 20;
    assert!(world.spawn_character(target, 10, 10));
    // Cause is an NPC (not CF_PLAYER), so this is not a PK death.
    assert!(world.spawn_character(character(2), 11, 10));

    let outcome = world
        .apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), POWERSCALE, 1, 0, 0)
        .unwrap();

    assert!(outcome.god_saved);
    assert!(!outcome.killed);
    assert!(!outcome.nodeath_saved);
    let saved = world.characters.get(&CharacterId(1)).unwrap();
    assert!(!saved.flags.contains(CharacterFlags::DEAD));
    assert!(saved.flags.contains(CharacterFlags::ALIVE));
    assert_eq!(saved.hp, POWERSCALE);
    assert_eq!(saved.saves, 2);
    assert_eq!(saved.got_saved, 1);
    // Transferred to the rest position (mirrors C `transfer_to_restarea`).
    assert_eq!((saved.x, saved.y), (20, 20));
    assert_eq!(
        world.drain_pending_system_texts(),
        vec![
            WorldSystemText {
                character_id: CharacterId(1),
                message: "Ishtar's hand reaches down and saves thee from certain death."
                    .to_string(),
            },
            WorldSystemText {
                character_id: CharacterId(1),
                message: "Thou hast two saves left.".to_string(),
            },
        ]
    );
}

#[test]
fn legacy_hurt_god_save_removes_poison_and_burn_effects() {
    let mut world = World::default();
    let mut target = character(1);
    target.flags.insert(CharacterFlags::PLAYER);
    target.hp = POWERSCALE;
    target.saves = 1;
    assert!(world.spawn_character(target, 10, 10));
    world.create_show_effect(EF_BURN, CharacterId(1), 0, 10, 0, 0);
    assert!(world.poison_character(CharacterId(1), 0, 0));

    world.apply_legacy_hurt(CharacterId(1), None, POWERSCALE, 1, 0, 0);

    let saved = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(saved.saves, 0);
    assert_eq!(saved.got_saved, 1);
    assert!(!world
        .effects
        .values()
        .any(|effect| effect.effect_type == EF_BURN
            && effect.target_character == Some(CharacterId(1))));
    assert!(saved
        .inventory
        .iter()
        .skip(crate::spell::SPELL_SLOT_START)
        .take(crate::spell::SPELL_SLOT_END - crate::spell::SPELL_SLOT_START)
        .all(Option::is_none));
}

#[test]
fn legacy_hurt_caps_saves_at_ten_after_decrement() {
    let mut world = World::default();
    let mut target = character(1);
    target.flags.insert(CharacterFlags::PLAYER);
    target.hp = POWERSCALE;
    target.saves = 255;
    assert!(world.spawn_character(target, 10, 10));

    world.apply_legacy_hurt(CharacterId(1), None, POWERSCALE, 1, 0, 0);

    assert_eq!(world.characters[&CharacterId(1)].saves, 10);
}

#[test]
fn legacy_hurt_pk_death_ignores_saves_and_kills_normally() {
    let mut world = World::default();
    let mut target = character(1);
    target.flags.insert(CharacterFlags::PLAYER);
    target.hp = POWERSCALE;
    target.saves = 5;
    assert!(world.spawn_character(target, 10, 10));
    let mut killer = character(2);
    killer.flags.insert(CharacterFlags::PLAYER);
    assert!(world.spawn_character(killer, 11, 10));

    let outcome = world
        .apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), POWERSCALE, 1, 0, 0)
        .unwrap();

    assert!(outcome.killed);
    assert!(!outcome.god_saved);
    let target = world.characters.get(&CharacterId(1)).unwrap();
    assert!(target.flags.contains(CharacterFlags::DEAD));
    assert_eq!(target.saves, 5);
}

#[test]
fn legacy_hurt_no_saves_left_kills_normally() {
    let mut world = World::default();
    let mut target = character(1);
    target.flags.insert(CharacterFlags::PLAYER);
    target.hp = POWERSCALE;
    target.saves = 0;
    assert!(world.spawn_character(target, 10, 10));

    let outcome = world
        .apply_legacy_hurt(CharacterId(1), None, POWERSCALE, 1, 0, 0)
        .unwrap();

    assert!(outcome.killed);
    assert!(!outcome.god_saved);
}

#[test]
fn swamp_monster_death_driver_upgrades_midnight_stone_circle_weapon() {
    let mut world = World::default();
    world.date.hour = 0;
    let mut dead = character(1);
    dead.driver = CDR_SWAMPMONSTER;
    let mut killer = character(2);
    killer.flags.insert(CharacterFlags::PLAYER);
    killer.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(7));
    let mut weapon = item(7, ItemFlags::empty());
    weapon.name = "Sword".into();
    weapon.driver_data.resize(38, 0);
    assert!(world.spawn_character(dead, 10, 10));
    assert!(world.spawn_character(killer, 145, 88));
    world.add_item(weapon);

    assert!(world.apply_swamp_monster_death_driver(CharacterId(1), CharacterId(2)));

    let weapon = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(weapon.template_id, IID_HARDKILL);
    assert_eq!(weapon.driver_data[36], 1);
    assert_eq!(weapon.driver_data[37], 12);
    assert!(weapon.flags.contains(ItemFlags::QUEST));
    assert_eq!(
        world.drain_pending_system_texts(),
        vec![WorldSystemText {
            character_id: CharacterId(2),
            message: "Your Sword starts to glow.".into(),
        }]
    );
}

#[test]
fn swamp_monster_death_driver_rejects_repeated_or_non_midnight_circles() {
    let mut world = World::default();
    world.date.hour = 1;
    let mut dead = character(1);
    dead.driver = CDR_SWAMPMONSTER;
    let mut killer = character(2);
    killer.flags.insert(CharacterFlags::PLAYER);
    killer.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(7));
    let mut weapon = item(7, ItemFlags::empty());
    weapon.driver_data.resize(38, 0);
    assert!(world.spawn_character(dead, 10, 10));
    assert!(world.spawn_character(killer, 145, 88));
    world.add_item(weapon);

    assert!(!world.apply_swamp_monster_death_driver(CharacterId(1), CharacterId(2)));
    world.date.hour = 0;
    world.items.get_mut(&ItemId(7)).unwrap().driver_data[36] = 1;

    assert!(!world.apply_swamp_monster_death_driver(CharacterId(1), CharacterId(2)));
    assert!(world.drain_pending_system_texts().is_empty());
}

#[test]
fn forest_monster_death_driver_upgrades_midnight_stone_circle_weapon() {
    let mut world = World::default();
    world.date.hour = 0;
    let mut dead = character(1);
    dead.driver = CDR_FORESTMONSTER;
    let mut killer = character(2);
    killer.flags.insert(CharacterFlags::PLAYER);
    killer.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(7));
    let mut weapon = item(7, ItemFlags::empty());
    weapon.name = "Sword".into();
    weapon.driver_data.resize(38, 0);
    assert!(world.spawn_character(dead, 10, 10));
    assert!(world.spawn_character(killer, 185, 188));
    world.add_item(weapon);

    assert!(world.apply_forest_monster_death_driver(CharacterId(1), CharacterId(2)));

    let weapon = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(weapon.template_id, IID_HARDKILL);
    assert_eq!(weapon.driver_data[36], 8);
    assert_eq!(weapon.driver_data[37], 6);
    assert!(weapon.flags.contains(ItemFlags::QUEST));
    assert_eq!(
        world.drain_pending_system_texts(),
        vec![WorldSystemText {
            character_id: CharacterId(2),
            message: "Your Sword starts to glow.".into(),
        }]
    );
}

#[test]
fn forest_monster_death_driver_rejects_repeated_or_non_midnight_circles() {
    let mut world = World::default();
    world.date.hour = 1;
    let mut dead = character(1);
    dead.driver = CDR_FORESTMONSTER;
    let mut killer = character(2);
    killer.flags.insert(CharacterFlags::PLAYER);
    killer.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(7));
    let mut weapon = item(7, ItemFlags::empty());
    weapon.driver_data.resize(38, 0);
    assert!(world.spawn_character(dead, 10, 10));
    assert!(world.spawn_character(killer, 185, 188));
    world.add_item(weapon);

    assert!(!world.apply_forest_monster_death_driver(CharacterId(1), CharacterId(2)));
    world.date.hour = 0;
    world.items.get_mut(&ItemId(7)).unwrap().driver_data[36] = 8;

    assert!(!world.apply_forest_monster_death_driver(CharacterId(1), CharacterId(2)));
    assert!(world.drain_pending_system_texts().is_empty());
}

#[test]
fn area1_monster_death_driver_upgrades_noon_stone_circle_weapon() {
    let mut world = World::default();
    world.date.hour = 12;
    let mut dead = character(1);
    dead.driver = CDR_CAMERON_FORESTMONSTER;
    let mut killer = character(2);
    killer.flags.insert(CharacterFlags::PLAYER);
    killer.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(7));
    let mut weapon = item(7, ItemFlags::empty());
    weapon.name = "Sword".into();
    weapon.driver_data.resize(38, 0);
    assert!(world.spawn_character(dead, 10, 10));
    assert!(world.spawn_character(killer, 15, 55));
    world.add_item(weapon);

    assert!(world.apply_area1_monster_death_driver(CharacterId(1), CharacterId(2)));

    let weapon = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(weapon.template_id, IID_HARDKILL);
    assert_eq!(weapon.driver_data[36], 16);
    assert_eq!(weapon.driver_data[37], 5);
    assert!(weapon.flags.contains(ItemFlags::QUEST));
    assert_eq!(
        world.drain_pending_system_texts(),
        vec![WorldSystemText {
            character_id: CharacterId(2),
            message: "Your Sword starts to glow.".into(),
        }]
    );
}

#[test]
fn area1_monster_death_driver_rejects_repeated_or_non_noon_kills() {
    let mut world = World::default();
    world.date.hour = 13;
    let mut dead = character(1);
    dead.driver = CDR_CAMERON_FORESTMONSTER;
    let mut killer = character(2);
    killer.flags.insert(CharacterFlags::PLAYER);
    killer.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(7));
    let mut weapon = item(7, ItemFlags::empty());
    weapon.driver_data.resize(38, 0);
    assert!(world.spawn_character(dead, 10, 10));
    assert!(world.spawn_character(killer, 15, 55));
    world.add_item(weapon);

    assert!(!world.apply_area1_monster_death_driver(CharacterId(1), CharacterId(2)));
    world.date.hour = 12;
    world.items.get_mut(&ItemId(7)).unwrap().driver_data[36] = 16;

    assert!(!world.apply_area1_monster_death_driver(CharacterId(1), CharacterId(2)));
    assert!(world.drain_pending_system_texts().is_empty());
}

#[test]
fn world_spiketrap_damage_uses_legacy_hurt_reduction() {
    let mut world = World::default();
    let mut character = character(1);
    character.hp = 10_000;
    character.lifeshield = 1_000;
    character.values[0][CharacterValue::Armor as usize] = 20;
    world.add_character(character);
    let mut trap = item(7, ItemFlags::USED | ItemFlags::USE);
    trap.driver = IDR_SPIKETRAP;
    trap.driver_data = vec![0, 4];
    world.add_item(trap);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_SPIKETRAP,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::SpikeTrapTriggered { .. }
    ));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.hp, 8_000);
    assert_eq!(character.lifeshield, 0);
    assert_eq!(character.driver_messages[0].message_type, NT_GOTHIT);
    assert_eq!(character.driver_messages[0].dat2, 2_000);
}

#[test]
fn world_applies_player_kill_setup_to_adjacent_character() {
    let mut world = World::default();
    let mut attacker = character(1);
    attacker.x = 10;
    attacker.y = 10;
    let mut defender = character(2);
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

    assert!(world.apply_player_action_setup(&mut player, 1));
    let attacker = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(attacker.action, action::ATTACK1);
    assert_eq!(attacker.act1, 2);
    assert_eq!(attacker.dir, Direction::Right as u8);
}

#[test]
fn world_blocks_player_kill_setup_against_area_one_player() {
    let mut world = World::default();
    let mut attacker = character(1);
    attacker.flags.insert(CharacterFlags::PLAYER);
    attacker.x = 10;
    attacker.y = 10;
    let mut defender = character(2);
    defender.flags.insert(CharacterFlags::PLAYER);
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

    assert!(world.apply_player_action_setup(&mut player, 1));
    let attacker = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(attacker.action, action::IDLE);
    assert_eq!(player.action.action, PlayerActionCode::Idle);
}

#[test]
fn world_applies_player_kill_setup_by_walking_toward_target() {
    let mut world = World::default();
    let mut attacker = character(1);
    attacker.x = 10;
    attacker.y = 10;
    world.map.tile_mut(10, 10).unwrap().character = 1;
    let mut defender = character(2);
    defender.x = 13;
    defender.y = 10;
    world.map.tile_mut(13, 10).unwrap().character = 2;
    world.add_character(attacker);
    world.add_character(defender);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Kill,
        arg1: 2,
        arg2: 0,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let attacker = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(attacker.action, action::WALK);
    assert_eq!((attacker.tox, attacker.toy), (11, 10));
    assert_eq!(player.action.action, PlayerActionCode::Kill);
}

#[test]
fn completed_attack_queues_legacy_showattack_pre_hurt_line() {
    let mut world = World::default();
    world.show_attack_debug = true;
    let mut attacker = character(1);
    attacker.x = 10;
    attacker.y = 10;
    attacker.dir = Direction::Right as u8;
    attacker.act1 = 2;
    // `get_attack_skill` (`tool.c:1224-1244`) only reads the raised Attack
    // stat when its "present" flag (`value[1][V_ATTACK]`) is set; otherwise
    // it falls back to the spellcaster formula. Set both "present" flags so
    // this test exercises the raised-stat branch, matching a real fighter.
    attacker.values[1][CharacterValue::Attack as usize] = 1;
    attacker.values[0][CharacterValue::Attack as usize] = 10;
    attacker.values[0][CharacterValue::Weapon as usize] = 10;
    let mut defender = character(2);
    defender.name = "Target".to_string();
    defender.x = 11;
    defender.y = 10;
    defender.dir = Direction::Left as u8;
    defender.values[1][CharacterValue::Parry as usize] = 1;
    defender.values[0][CharacterValue::Parry as usize] = 10;
    world.spawn_character(attacker, 10, 10);
    world.spawn_character(defender, 11, 10);

    assert!(world.complete_attack_with_rolls(CharacterId(1), CharacterId(2), 49, 6));

    let texts = world.drain_pending_system_texts();
    assert_eq!(texts[0].character_id, CharacterId(1));
    assert_eq!(
        texts[0].message,
        "attack Target, diff=0 (20 20), chan=50, percent=90, dam=16"
    );
}

#[test]
fn poison_callback_uses_legacy_hurt_shield_reduction() {
    let mut world = World::default();
    world.tick = Tick(1_000);
    let mut character = character(1);
    character.hp = 10 * POWERSCALE;
    character.lifeshield = POWERSCALE;
    // C `update_char` clamps current HP to the recomputed max; give the
    // character a raised HP baseline large enough that installing the
    // poison spell's `-1` modifier doesn't itself clamp `hp` down.
    character.values[1][CharacterValue::Hp as usize] = 100;
    world.add_character(character);
    assert!(world.poison_character(CharacterId(1), 4, 0));

    world.tick = Tick(1_000 + TICKS_PER_SECOND);
    world.process_due_timers(1);

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.hp, 10 * POWERSCALE - 167);
    assert_eq!(character.lifeshield, POWERSCALE - 166);
    assert!(character.flags.contains(CharacterFlags::UPDATE));
    assert_eq!(character.driver_messages.len(), 1);
    assert_eq!(character.driver_messages[0].dat2, 167);
}

#[test]
fn tile_special_check_drowns_player_without_oxygen_on_underwater_slowdeath() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.hp = 1_000;
    assert!(world.spawn_character(player, 10, 10));
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::SLOWDEATH | MapFlags::UNDERWATER);

    let outcome = world.tile_special_check(CharacterId(1));

    assert_eq!(outcome.damage, 50);
    assert_eq!(outcome.bubble_effect_id, None);
    assert_eq!(outcome.sound_type, None);
    let player = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(player.hp, 950);
    assert!(player.flags.contains(CharacterFlags::UPDATE));
}

#[test]
fn tile_special_check_applies_non_underwater_slowdeath_damage() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.hp = 1_000;
    assert!(world.spawn_character(player, 10, 10));
    let tile = world.map.tile_mut(10, 10).unwrap();
    tile.flags.insert(MapFlags::SLOWDEATH);
    tile.ground_sprite = 59706;

    let outcome = world.tile_special_check(CharacterId(1));

    assert_eq!(outcome.damage, 250);
    assert_eq!(outcome.bubble_effect_id, None);
    assert_eq!(outcome.sound_type, None);
    assert!(world.drain_pending_sound_specials().is_empty());
    let player = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(player.hp, 750);
    assert!(player.flags.contains(CharacterFlags::UPDATE));
}

// `pending_lostcon_hurt_events`/`drain_lostcon_hurt_events` (C
// `death.c:1213-1217`'s `player_use_potion`/`player_use_recall` reaction
// gate).

#[test]
fn apply_legacy_hurt_queues_a_lostcon_hurt_event_for_a_damaged_lingering_player() {
    let mut world = World::default();
    let mut target = character(1);
    target.flags.insert(CharacterFlags::PLAYER);
    target.driver = CDR_LOSTCON;
    target.hp = 5 * POWERSCALE;
    assert!(world.spawn_character(target, 10, 10));

    world.apply_legacy_hurt(CharacterId(1), None, POWERSCALE, 1, 0, 0);

    assert_eq!(world.drain_lostcon_hurt_events(), vec![CharacterId(1)]);
    // Draining empties the queue.
    assert!(world.drain_lostcon_hurt_events().is_empty());
}

#[test]
fn apply_legacy_hurt_does_not_queue_a_lostcon_event_for_a_normal_player() {
    let mut world = World::default();
    let mut target = character(1);
    target.flags.insert(CharacterFlags::PLAYER);
    target.hp = 5 * POWERSCALE;
    assert!(world.spawn_character(target, 10, 10));

    world.apply_legacy_hurt(CharacterId(1), None, POWERSCALE, 1, 0, 0);

    assert!(world.drain_lostcon_hurt_events().is_empty());
}

#[test]
fn apply_legacy_hurt_does_not_queue_a_lostcon_event_for_a_non_player_lostcon_npc() {
    let mut world = World::default();
    let mut target = character(1);
    target.driver = CDR_LOSTCON;
    target.hp = 5 * POWERSCALE;
    assert!(world.spawn_character(target, 10, 10));

    world.apply_legacy_hurt(CharacterId(1), None, POWERSCALE, 1, 0, 0);

    assert!(world.drain_lostcon_hurt_events().is_empty());
}

#[test]
fn apply_legacy_hurt_does_not_queue_a_lostcon_event_when_damage_is_fully_blocked() {
    let mut world = World::default();
    let mut target = character(1);
    target.flags.insert(CharacterFlags::PLAYER);
    target.driver = CDR_LOSTCON;
    target.hp = 5 * POWERSCALE;
    assert!(world.spawn_character(target, 10, 10));

    // Zero damage in, zero damage out - C's `if (dam) {...}` gate around
    // both the hp reduction and the `player_use_potion`/`player_use_recall`
    // reaction never fires.
    world.apply_legacy_hurt(CharacterId(1), None, 0, 1, 0, 0);

    assert!(world.drain_lostcon_hurt_events().is_empty());
}

// `pending_combat_events`/`drain_combat_events` (C `death.c:1112-1117`'s
// `if (dam > 0) { macro_track_combat(cn); if (cc > 0)
// macro_track_combat(cc); }`).

#[test]
fn apply_legacy_hurt_queues_combat_events_for_both_defender_and_attacker() {
    let mut world = World::default();
    let target = character(1);
    let attacker = character(2);
    assert!(world.spawn_character(target, 10, 10));
    assert!(world.spawn_character(attacker, 11, 11));

    world.apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), POWERSCALE, 1, 0, 0);

    let mut events = world.drain_combat_events();
    events.sort_by_key(|id| id.0);
    assert_eq!(events, vec![CharacterId(1), CharacterId(2)]);
}

#[test]
fn apply_legacy_hurt_queues_only_the_defender_when_there_is_no_attacker() {
    let mut world = World::default();
    let target = character(1);
    assert!(world.spawn_character(target, 10, 10));

    world.apply_legacy_hurt(CharacterId(1), None, POWERSCALE, 1, 0, 0);

    assert_eq!(world.drain_combat_events(), vec![CharacterId(1)]);
}

#[test]
fn apply_legacy_hurt_does_not_queue_a_combat_event_for_zero_damage() {
    let mut world = World::default();
    let target = character(1);
    let attacker = character(2);
    assert!(world.spawn_character(target, 10, 10));
    assert!(world.spawn_character(attacker, 11, 11));

    world.apply_legacy_hurt(CharacterId(1), Some(CharacterId(2)), 0, 1, 0, 0);

    assert!(world.drain_combat_events().is_empty());
}
