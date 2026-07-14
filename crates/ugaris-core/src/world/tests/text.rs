// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use super::*;
use crate::clan::CLUB_OFFSET;

#[test]
fn legacy_hurt_queues_player_ouch_and_death_sounds() {
    let mut world = World::default();
    let mut male = character(1);
    male.flags |= CharacterFlags::PLAYER | CharacterFlags::MALE;
    male.hp = 5 * POWERSCALE;
    assert!(world.spawn_character(male, 10, 10));

    let outcome = world
        .apply_legacy_hurt(CharacterId(1), None, POWERSCALE, 1, 0, 0)
        .unwrap();

    assert_eq!(outcome.hp_damage, POWERSCALE);
    let sounds = world.drain_pending_sound_specials();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].character_id, CharacterId(1));
    assert_eq!(sounds[0].special.special_type, 9);

    let mut female = character(2);
    female.flags |= CharacterFlags::PLAYER | CharacterFlags::FEMALE;
    female.hp = POWERSCALE;
    assert!(world.spawn_character(female, 11, 10));

    let outcome = world
        .apply_legacy_hurt(CharacterId(2), None, POWERSCALE, 1, 0, 0)
        .unwrap();

    assert!(outcome.killed);
    let sounds = world.drain_pending_sound_specials();
    assert_eq!(sounds.len(), 4);
    assert_eq!(sounds[0].special.special_type, 32);
    assert_eq!(sounds[1].special.special_type, 32);
    assert_eq!(sounds[2].special.special_type, 33);
    assert_eq!(sounds[3].special.special_type, 33);
}

#[test]
fn legacy_hurt_nodeath_player_still_queues_death_sound() {
    let mut world = World::default();
    let mut target = character(1);
    target.flags |= CharacterFlags::PLAYER | CharacterFlags::MALE | CharacterFlags::NODEATH;
    target.hp = 700;
    assert!(world.spawn_character(target, 10, 10));

    let outcome = world
        .apply_legacy_hurt(CharacterId(1), None, 800, 1, 0, 0)
        .unwrap();

    assert!(outcome.nodeath_saved);
    let sounds = world.drain_pending_sound_specials();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].special.special_type, 4);
}

#[test]
fn sound_area_talk_type_is_sound_sector_gated() {
    let mut world = World {
        map: MapGrid::new(12, 12),
        ..World::default()
    };
    for y in 0..12 {
        world.map.set_flags(6, y, MapFlags::SOUNDBLOCK);
    }
    let mut listener = character(1);
    listener.flags.insert(CharacterFlags::PLAYER);
    listener.x = 8;
    listener.y = 4;
    world.add_character(listener);

    assert!(world
        .sound_area_specials(4, 4, u32::from(LOG_TALK))
        .is_empty());
    assert_eq!(world.sound_area_specials(4, 4, 7).len(), 1);
}

#[test]
fn queued_sound_area_drains_legacy_player_special_targets() {
    let mut world = World {
        map: MapGrid::new(24, 24),
        ..World::default()
    };
    let mut listener = character(1);
    listener.flags.insert(CharacterFlags::PLAYER);
    listener.x = 12;
    listener.y = 10;
    world.add_character(listener);

    world.queue_sound_area(10, 10, 5);

    let sounds = world.drain_pending_sound_specials();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].character_id, CharacterId(1));
    assert_eq!(sounds[0].special.special_type, 5);
    assert_eq!(sounds[0].special.opt1, -40);
    assert_eq!(sounds[0].special.opt2, 200);
    assert!(world.drain_pending_sound_specials().is_empty());
}

#[test]
fn world_applies_fdemon_loader_cursor_ground_sound_and_timer() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.cursor_item = Some(ItemId(9));
    assert!(world.spawn_character(player, 11, 10));
    let mut loader = item(7, ItemFlags::USED | ItemFlags::USE);
    loader.driver = IDR_FDEMONLOADER;
    assert!(world.map.set_item_map(&mut loader, 10, 10));
    world.map.tile_mut(10, 10).unwrap().ground_sprite = 123;
    world.add_item(loader);
    let mut crystal = item(9, ItemFlags::USED);
    crystal.template_id = 0x0100004A;
    crystal.driver_data = vec![12];
    crystal.carried_by = Some(CharacterId(1));
    world.add_item(crystal);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_FDEMONLOADER,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        8,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::FdemonLoaderChanged { .. }
    ));
    assert!(!world.items.contains_key(&ItemId(9)));
    assert_eq!(world.characters[&CharacterId(1)].cursor_item, None);
    assert_eq!(world.items[&ItemId(7)].sprite, 59036);
    assert_eq!(
        world.map.tile(10, 10).unwrap().ground_sprite,
        (59021 << 16) | 123
    );
    assert_eq!(
        world.drain_pending_sound_specials()[0].special.special_type,
        41
    );
    assert_eq!(world.timers.used_timers(), 0);

    assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));
    world.advance();
    let outcomes = world.process_due_timers(8);

    assert_eq!(outcomes.len(), 1);
    assert!(matches!(
        outcomes[0],
        ItemDriverOutcome::FdemonLoaderChanged { .. }
    ));
    assert_eq!(world.timers.used_timers(), 1);
}

#[test]
fn palace_bomb_explosion_creates_effect_sound_burns_and_removes_item() {
    let mut world = World::default();

    let mut owner = character(1);
    owner.flags |= CharacterFlags::PLAYER;
    owner.hp = 10_000;
    assert!(world.spawn_character(owner, 10, 10));

    let mut npc = character(2);
    npc.hp = 10_000;
    assert!(world.spawn_character(npc, 11, 10));

    let mut other_player = character(3);
    other_player.flags |= CharacterFlags::PLAYER;
    other_player.hp = 10_000;
    assert!(world.spawn_character(other_player, 10, 11));

    let mut islena = character(4);
    islena.name = "Islena".to_string();
    islena.hp = 10_000;
    assert!(world.spawn_character(islena, 9, 10));

    let mut bomb = item(7, ItemFlags::USED);
    bomb.driver = IDR_PALACEBOMB;
    bomb.driver_data = vec![2, 1, 0, 0, 0];
    bomb.x = 10;
    bomb.y = 10;
    world.add_item(bomb);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_PALACEBOMB,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        11,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::PalaceBombExplode { .. }
    ));
    assert!(!world.items.contains_key(&ItemId(7)));
    assert_eq!(world.map.tile(10, 10).unwrap().item, 0);
    assert!(world.effects.values().any(|effect| {
        effect.effect_type == EF_EXPLODE
            && effect.base_sprite == 50050
            && effect
                .fields
                .iter()
                .any(|&field| field == world.map.legacy_index(10, 10).unwrap() as i32)
    }));
    let burn_targets: Vec<CharacterId> = world
        .effects
        .values()
        .filter(|effect| effect.effect_type == EF_BURN)
        .filter_map(|effect| effect.target_character)
        .collect();
    assert!(burn_targets.contains(&CharacterId(1)));
    assert!(burn_targets.contains(&CharacterId(2)));
    assert!(!burn_targets.contains(&CharacterId(3)));
    assert!(!burn_targets.contains(&CharacterId(4)));
    assert_eq!(world.characters[&CharacterId(1)].hp, 10_000);

    let sounds = world.drain_pending_sound_specials();
    assert!(!sounds.is_empty());
    assert!(sounds.iter().all(|sound| sound.special.special_type == 6));
}

#[test]
fn completed_attack_queues_legacy_unarmed_miss_sound() {
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
    let mut defender = character(2);
    defender.x = 11;
    defender.y = 10;
    defender.values[0][CharacterValue::Parry as usize] = 10;
    world.spawn_character(attacker, 10, 10);
    world.spawn_character(defender, 11, 10);

    assert!(world.complete_attack_with_rolls(CharacterId(1), CharacterId(2), 100, 1));

    assert_eq!(world.characters[&CharacterId(2)].hp, 0);
    assert_eq!(
        world.drain_pending_sound_specials()[0].special.special_type,
        8
    );
}

#[test]
fn completed_attack_queues_legacy_weapon_clash_miss_sound() {
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
    defender.values[0][CharacterValue::Parry as usize] = 10;
    let mut attacker_weapon = item(10, ItemFlags::USED | ItemFlags::WNRHAND);
    attacker_weapon.carried_by = Some(CharacterId(1));
    let mut defender_weapon = item(11, ItemFlags::USED | ItemFlags::WNRHAND);
    defender_weapon.carried_by = Some(CharacterId(2));
    attacker.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(10));
    defender.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(11));
    world.spawn_character(attacker, 10, 10);
    world.spawn_character(defender, 11, 10);
    world.add_item(attacker_weapon);
    world.add_item(defender_weapon);

    assert!(world.complete_attack_with_rolls(CharacterId(1), CharacterId(2), 100, 1));
    assert_eq!(
        world.drain_pending_sound_specials()[0].special.special_type,
        34
    );
    assert!(world.complete_attack_with_rolls(CharacterId(1), CharacterId(2), 99, 1));
    assert_eq!(
        world.drain_pending_sound_specials()[0].special.special_type,
        35
    );
}

#[test]
fn completed_attack_weapon_clash_sound_uses_independent_legacy_roll() {
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
    defender.values[0][CharacterValue::Parry as usize] = 10;
    let mut attacker_weapon = item(10, ItemFlags::USED | ItemFlags::WNRHAND);
    attacker_weapon.carried_by = Some(CharacterId(1));
    let mut defender_weapon = item(11, ItemFlags::USED | ItemFlags::WNRHAND);
    defender_weapon.carried_by = Some(CharacterId(2));
    attacker.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(10));
    defender.inventory[worn_slot::RIGHT_HAND] = Some(ItemId(11));
    world.spawn_character(attacker, 10, 10);
    world.spawn_character(defender, 11, 10);
    world.add_item(attacker_weapon);
    world.add_item(defender_weapon);

    assert!(world.complete_attack_with_rolls_and_clash_roll(
        CharacterId(1),
        CharacterId(2),
        100,
        1,
        1,
    ));
    assert_eq!(
        world.drain_pending_sound_specials()[0].special.special_type,
        35
    );
    assert!(world.complete_attack_with_rolls_and_clash_roll(
        CharacterId(1),
        CharacterId(2),
        99,
        1,
        0,
    ));
    assert_eq!(
        world.drain_pending_sound_specials()[0].special.special_type,
        34
    );
}

#[test]
fn ball_strike_sound_keeps_legacy_eighth_tick_cadence() {
    let mut world = World {
        tick: Tick(1),
        ..World::default()
    };
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.x = 10;
    caster.y = 10;
    caster.act1 = 15;
    caster.act2 = 10;
    caster.values[0][CharacterValue::Flash as usize] = 50;
    caster.values[0][CharacterValue::Tactics as usize] = 24;
    let mut target = character(2);
    target.flags.insert(CharacterFlags::ALIVE);
    target.hp = 30 * POWERSCALE;
    target.values[0][CharacterValue::Immunity as usize] = 20;
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 12, 10);
    let caster = world.characters.get(&CharacterId(1)).unwrap().clone();
    world.create_ball_effect(&caster);

    world.tick_effects();

    assert!(world.drain_pending_sound_specials().is_empty());
}

#[test]
fn freeze_completion_succeeds_and_sounds_without_targets() {
    let mut world = World::default();
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.action = action::FREEZE;
    caster.duration = 1;
    caster.values[0][CharacterValue::Freeze as usize] = 50;
    world.spawn_character(caster, 10, 10);

    let completed = world.tick_basic_actions();

    assert_eq!(completed.len(), 1);
    assert!(completed[0].ok);
    let sounds = world.drain_pending_sound_specials();
    assert_eq!(sounds.len(), 1);
    assert_eq!(sounds[0].character_id, CharacterId(1));
    assert_eq!(sounds[0].special.special_type, 31);
}

#[test]
fn player_warcry_sets_up_and_debuffs_sound_reachable_targets() {
    let mut world = World::default();
    world.tick = Tick(400);
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.endurance = 30 * POWERSCALE;
    caster.values[0][CharacterValue::Warcry as usize] = 60;
    let mut target = character(2);
    target.flags.insert(CharacterFlags::ALIVE);
    target.hp = 20 * POWERSCALE;
    target.lifeshield = POWERSCALE;
    target.values[0][CharacterValue::Immunity as usize] = 20;
    // C `update_char` (via `install_speed_spell`'s warcry install) clamps
    // current HP to the recomputed max; give the target a raised HP
    // baseline large enough that the warcry install itself doesn't clamp
    // `hp` before the `hurt()` damage below is applied.
    target.values[1][CharacterValue::Hp as usize] = 100;
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 13, 10);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Warcry,
        arg1: 0,
        arg2: 0,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(caster.action, action::WARCRY);
    assert_eq!(caster.endurance, 10 * POWERSCALE);

    world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
    assert!(world.tick_basic_actions()[0].ok);

    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(caster.lifeshield, 30 * POWERSCALE);
    let target = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(target.hp, 16_400);
    assert_eq!(target.lifeshield, POWERSCALE);
    assert_eq!(target.driver_messages[0].message_type, NT_GOTHIT);
    assert_eq!(target.driver_messages[0].dat1, 1);
    assert_eq!(target.driver_messages[0].dat2, 3_600);
    // C `act_warcry` (`act.c:1399-1402`): unconditional area `NT_CHAR`/
    // `NT_SPELL` broadcast from the caster after the per-target loop, so the
    // target also observes these two messages (indices 1-2) after its own
    // `NT_GOTHIT` (index 0) from `warcry_someone`'s `hurt` call.
    assert_eq!(target.driver_messages[1].message_type, NT_CHAR);
    assert_eq!(target.driver_messages[1].dat1, 1);
    assert_eq!(target.driver_messages[2].message_type, NT_SPELL);
    assert_eq!(target.driver_messages[2].dat1, 1);
    assert_eq!(
        target.driver_messages[2].dat2,
        CharacterValue::Warcry as i32
    );
    let spell_id = target.inventory[29].unwrap();
    let spell = world.items.get(&spell_id).unwrap();
    assert_eq!(spell.driver, IDR_WARCRY);
    assert_eq!(spell.modifier_index[0], CharacterValue::Speed as i16);
    assert_eq!(spell.modifier_value[0], -340);
    assert_eq!(spell.carried_by, Some(CharacterId(2)));
    assert_eq!(
        u32::from_le_bytes(spell.driver_data[0..4].try_into().unwrap()),
        496
    );
    let effect = world.effects.values().next().unwrap();
    assert_eq!(effect.effect_type, EF_WARCRY);
    assert_eq!(effect.target_character, Some(CharacterId(2)));
    assert_eq!(effect.start_tick, 400);
    assert_eq!(effect.stop_tick, 496);
    assert_eq!(world.timers.used_timers(), 1);
    assert!(world.drain_pending_sound_specials().is_empty());
}

#[test]
fn player_warcry_does_not_pass_soundblocking_tiles() {
    let mut world = World::default();
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.endurance = 30 * POWERSCALE;
    caster.values[0][CharacterValue::Warcry as usize] = 60;
    let mut target = character(2);
    target.flags.insert(CharacterFlags::ALIVE);
    target.values[0][CharacterValue::Immunity as usize] = 20;
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 13, 10);
    for y in 0..world.map.height() {
        world.map.set_flags(11, y, MapFlags::SOUNDBLOCK);
    }
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Warcry,
        arg1: 0,
        arg2: 0,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
    assert!(world.tick_basic_actions()[0].ok);

    assert!(world.drain_pending_sound_specials().is_empty());
    let target = world.characters.get(&CharacterId(2)).unwrap();
    assert!(target.inventory[12..30].iter().all(Option::is_none));
    // The `NT_CHAR`/`NT_SPELL` area broadcast is unconditional (not gated on
    // sound-blocking, unlike the per-target warcry effect itself), so the
    // target still observes it even though the soundblock wall stopped the
    // warcry from actually reaching it.
    assert_eq!(target.driver_messages.len(), 2);
    assert_eq!(target.driver_messages[0].message_type, NT_CHAR);
    assert_eq!(target.driver_messages[1].message_type, NT_SPELL);
    assert_eq!(
        target.driver_messages[1].dat2,
        CharacterValue::Warcry as i32
    );
}

fn look_char_pair(looker_flags: CharacterFlags, target_flags: CharacterFlags) -> World {
    let mut world = World::default();
    let mut looker = character(1);
    looker.flags = CharacterFlags::USED | looker_flags | CharacterFlags::PLAYER;
    let mut target = character(2);
    target.flags = CharacterFlags::USED | target_flags;
    target.name = "Bob".into();
    target.description = "A tall warrior.".into();
    target.level = 10;
    target.karma = 5;
    world.spawn_character(looker, 10, 10);
    world.spawn_character(target, 11, 10);
    world
}

#[test]
fn look_character_text_reports_saves_deaths_mirror_and_karma_for_player_target() {
    let mut world = look_char_pair(
        CharacterFlags::empty(),
        CharacterFlags::PLAYER | CharacterFlags::MALE,
    );
    {
        let target = world.characters.get_mut(&CharacterId(2)).unwrap();
        target.saves = 3;
        target.got_saved = 1;
        target.deaths = 2;
    }

    let result = world
        .look_character_text(CharacterId(1), CharacterId(2), false, 7, 0, 0)
        .expect("visible player target should produce look text");

    assert_eq!(result.header, "#1Bob (10):");
    assert_eq!(
        result.body,
        "#2A tall warrior. He has 3 saves, was saved 1 times and died 2 times. Mirror=7. Karma: 5"
    );
}

#[test]
fn look_character_text_uses_singular_save_wording_for_exactly_one_save() {
    let mut world = look_char_pair(
        CharacterFlags::empty(),
        CharacterFlags::PLAYER | CharacterFlags::FEMALE,
    );
    {
        let target = world.characters.get_mut(&CharacterId(2)).unwrap();
        target.saves = 1;
    }

    let result = world
        .look_character_text(CharacterId(1), CharacterId(2), false, 0, 0, 0)
        .unwrap();

    assert!(result
        .body
        .contains("She has 1 save, was saved 0 times and died 0 times."));
}

#[test]
fn look_character_text_reports_hardcore_death_count_instead_of_saves() {
    let mut world = look_char_pair(
        CharacterFlags::empty(),
        CharacterFlags::PLAYER | CharacterFlags::HARDCORE | CharacterFlags::MALE,
    );
    {
        let target = world.characters.get_mut(&CharacterId(2)).unwrap();
        target.deaths = 4;
    }

    let result = world
        .look_character_text(CharacterId(1), CharacterId(2), false, 0, 0, 0)
        .unwrap();

    assert!(result
        .body
        .contains("He is a hardcore character and died 4 times."));
    assert!(!result.body.contains("save"));
}

#[test]
fn look_character_text_the_brave_header_variant_when_shrine_flag_set() {
    let mut world = look_char_pair(CharacterFlags::empty(), CharacterFlags::PLAYER);

    let result = world
        .look_character_text(CharacterId(1), CharacterId(2), true, 0, 0, 0)
        .unwrap();

    assert_eq!(result.header, "#1Bob the Brave (10):");
}

#[test]
fn look_character_text_title_prefix_for_won_male_and_female() {
    let mut world = look_char_pair(
        CharacterFlags::empty(),
        CharacterFlags::PLAYER | CharacterFlags::WON,
    );
    assert_eq!(
        world
            .look_character_text(CharacterId(1), CharacterId(2), false, 0, 0, 0)
            .unwrap()
            .header,
        "#1Sir Bob (10):"
    );

    world.characters.get_mut(&CharacterId(2)).unwrap().flags |= CharacterFlags::FEMALE;
    assert_eq!(
        world
            .look_character_text(CharacterId(1), CharacterId(2), false, 0, 0, 0)
            .unwrap()
            .header,
        "#1Lady Bob (10):"
    );
}

#[test]
fn look_character_text_omits_player_only_lines_for_npc_targets() {
    let mut world = look_char_pair(CharacterFlags::empty(), CharacterFlags::empty());

    let result = world
        .look_character_text(CharacterId(1), CharacterId(2), false, 99, 0, 0)
        .unwrap();

    assert_eq!(result.header, "#1Bob (10):");
    assert_eq!(result.body, "#2A tall warrior. ");
    assert!(!result.body.contains("Mirror"));
    assert!(!result.body.contains("Karma"));
}

#[test]
fn look_character_text_includes_profession_lines_regardless_of_player_flag() {
    let mut world = look_char_pair(CharacterFlags::empty(), CharacterFlags::empty());
    {
        let target = world.characters.get_mut(&CharacterId(2)).unwrap();
        target.professions[0] = 20; // Athlete, 20/30 = 66% -> "a skilled "
        target.flags.insert(CharacterFlags::MALE);
    }

    let result = world
        .look_character_text(CharacterId(1), CharacterId(2), false, 0, 0, 0)
        .unwrap();

    assert!(result.body.contains("He is a skilled Athlete. "));
}

#[test]
fn look_character_text_none_when_looker_is_not_player() {
    let world = look_char_pair(CharacterFlags::empty(), CharacterFlags::PLAYER);
    // Force the looker to not be a player (C requires ch[cn].flags & CF_PLAYER).
    let mut world = world;
    world
        .characters
        .get_mut(&CharacterId(1))
        .unwrap()
        .flags
        .remove(CharacterFlags::PLAYER);

    assert!(world
        .look_character_text(CharacterId(1), CharacterId(2), false, 0, 0, 0)
        .is_none());
}

#[test]
fn look_character_text_none_when_target_invisible() {
    let mut world = look_char_pair(CharacterFlags::empty(), CharacterFlags::PLAYER);
    world
        .characters
        .get_mut(&CharacterId(2))
        .unwrap()
        .flags
        .insert(CharacterFlags::INVISIBLE);

    assert!(world
        .look_character_text(CharacterId(1), CharacterId(2), false, 0, 0, 0)
        .is_none());
}

#[test]
fn look_character_text_none_for_unknown_looker_or_target() {
    let mut world = look_char_pair(CharacterFlags::empty(), CharacterFlags::PLAYER);

    assert!(world
        .look_character_text(CharacterId(99), CharacterId(2), false, 0, 0, 0)
        .is_none());
    assert!(world
        .look_character_text(CharacterId(1), CharacterId(99), false, 0, 0, 0)
        .is_none());
}

#[test]
fn look_character_text_reports_army_rank_when_positive() {
    let mut world = look_char_pair(
        CharacterFlags::empty(),
        CharacterFlags::PLAYER | CharacterFlags::MALE,
    );
    {
        let target = world.characters.get_mut(&CharacterId(2)).unwrap();
        // cbrt(1_000_000) = 100 -> clamped to MAX_ARMY_RANK (40).
        target.military_points = 1_000_000;
    }

    let result = world
        .look_character_text(CharacterId(1), CharacterId(2), false, 0, 0, 0)
        .unwrap();

    assert!(result
        .body
        .contains("Bob is a Avatar of Astonia in the Imperial Army. "));
}

#[test]
fn look_character_text_omits_army_rank_line_when_zero() {
    let mut world = look_char_pair(CharacterFlags::empty(), CharacterFlags::PLAYER);

    let result = world
        .look_character_text(CharacterId(1), CharacterId(2), false, 0, 0, 0)
        .unwrap();

    assert!(!result.body.contains("Imperial Army"));
}

#[test]
fn look_character_text_reports_pk_info_for_player_killers() {
    let mut world = look_char_pair(
        CharacterFlags::empty(),
        CharacterFlags::PLAYER | CharacterFlags::PK | CharacterFlags::MALE,
    );

    let result = world
        .look_character_text(CharacterId(1), CharacterId(2), false, 0, 3, 5)
        .unwrap();

    assert!(result.body.contains(
        "Bob is a player killer. He killed 3 players and died 5 times through the hands of other players. "
    ));
}

#[test]
fn look_character_text_omits_pk_info_when_not_pk() {
    let mut world = look_char_pair(CharacterFlags::empty(), CharacterFlags::PLAYER);

    let result = world
        .look_character_text(CharacterId(1), CharacterId(2), false, 0, 3, 5)
        .unwrap();

    assert!(!result.body.contains("player killer"));
}

#[test]
fn look_character_text_reports_clan_membership() {
    let mut world = look_char_pair(
        CharacterFlags::empty(),
        CharacterFlags::PLAYER | CharacterFlags::MALE,
    );
    let clan = world.clan_registry.found_clan("Black Rose", 0).unwrap();
    world
        .clan_registry
        .set_rankname(clan, 2, "Veteran")
        .unwrap();
    {
        let target = world.characters.get_mut(&CharacterId(2)).unwrap();
        target.clan = clan;
        target.clan_serial = world.clan_registry.serial(clan);
        target.clan_rank = 2;
    }

    let result = world
        .look_character_text(CharacterId(1), CharacterId(2), false, 0, 0, 0)
        .unwrap();

    assert!(result
        .body
        .contains("He is a member of the clan 'Black Rose', his rank is Veteran. "));
}

#[test]
fn look_character_text_resets_stale_clan_reference() {
    let mut world = look_char_pair(
        CharacterFlags::empty(),
        CharacterFlags::PLAYER | CharacterFlags::MALE,
    );
    let clan = world.clan_registry.found_clan("Black Rose", 0).unwrap();
    {
        let target = world.characters.get_mut(&CharacterId(2)).unwrap();
        target.clan = clan;
        target.clan_serial = world.clan_registry.serial(clan) + 1; // stale
        target.clan_rank = 1;
    }

    let result = world
        .look_character_text(CharacterId(1), CharacterId(2), false, 0, 0, 0)
        .unwrap();

    assert!(!result.body.contains("member of the clan"));
    let target = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(target.clan, 0);
    assert_eq!(target.clan_rank, 0);
    assert_eq!(target.clan_serial, 0);
}

#[test]
fn look_character_text_reports_club_membership() {
    let mut world = look_char_pair(
        CharacterFlags::empty(),
        CharacterFlags::PLAYER | CharacterFlags::MALE,
    );
    let club = world.club_registry.create_club("Rangers", 0).unwrap();
    {
        let target = world.characters.get_mut(&CharacterId(2)).unwrap();
        target.clan = CLUB_OFFSET + club;
        target.clan_serial = world.club_registry.serial(club);
        target.clan_rank = 1;
    }

    let result = world
        .look_character_text(CharacterId(1), CharacterId(2), false, 0, 0, 0)
        .unwrap();

    assert!(result.body.contains(&format!(
        "He is a member of the club 'Rangers' ({club}), his rank is Private. "
    )));
}

#[test]
fn look_character_paperdoll_reports_sprite_colors_and_worn_item_sprites() {
    let mut world = World::default();
    let mut target = character(2);
    target.flags = CharacterFlags::USED | CharacterFlags::PLAYER;
    target.sprite = 1234;
    target.c1 = 1;
    target.c2 = 2;
    target.c3 = 3;
    let mut sword = item(10, ItemFlags::USED);
    sword.sprite = 555;
    world.items.insert(sword.id, sword);
    target.inventory[0] = Some(ItemId(10));
    world.characters.insert(target.id, target);

    let paperdoll = world
        .look_character_paperdoll(CharacterId(2))
        .expect("target should exist");

    assert_eq!(paperdoll.sprite, 1234);
    assert_eq!(paperdoll.colors, [1, 2, 3]);
    assert_eq!(paperdoll.worn_sprites[0], 555);
    assert!(paperdoll.worn_sprites[1..]
        .iter()
        .all(|&sprite| sprite == 0));
}

#[test]
fn look_character_paperdoll_none_for_unknown_target() {
    let world = World::default();
    assert!(world.look_character_paperdoll(CharacterId(42)).is_none());
}

#[test]
fn npc_say_broadcasts_at_say_dist_and_never_rejects_quotes() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.name = "Gwen".into();
    assert!(world.spawn_character(npc, 10, 10));

    // C `say()` has its quote-rejecting check commented out, unlike
    // `quiet_say`/`emote`/`murmur`.
    assert!(world.npc_say(CharacterId(1), "hi \"there\""));
    let texts = world.drain_pending_area_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].message, "Gwen says: \"hi \"there\"\"");
    assert_eq!(texts[0].max_distance, world.settings.say_dist as u16);

    assert!(!world.npc_say(CharacterId(99), "nobody"));
}

#[test]
fn npc_quiet_say_uses_quietsay_dist_and_rejects_quotes() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.name = "Gwen".into();
    assert!(world.spawn_character(npc, 10, 10));

    assert!(world.npc_quiet_say(CharacterId(1), "Hello there!"));
    let texts = world.drain_pending_area_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].message, "Gwen says: \"Hello there!\"");
    assert_eq!(texts[0].max_distance, world.settings.quietsay_dist as u16);

    assert!(!world.npc_quiet_say(CharacterId(1), "bad \"quote\""));
    assert!(world.drain_pending_area_texts().is_empty());
    assert!(!world.npc_quiet_say(CharacterId(99), "nobody"));
}

#[test]
fn npc_emote_uses_emote_dist_and_rejects_quotes() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.name = "Gwen".into();
    assert!(world.spawn_character(npc, 10, 10));

    assert!(world.npc_emote(CharacterId(1), "waves"));
    let texts = world.drain_pending_area_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].message, "Gwen waves.");
    assert_eq!(texts[0].max_distance, world.settings.emote_dist as u16);

    assert!(!world.npc_emote(CharacterId(1), "bad \"quote\""));
    assert!(world.drain_pending_area_texts().is_empty());
}

#[test]
fn npc_murmur_uses_whisper_dist_and_rejects_quotes() {
    let mut world = World::default();
    let mut npc = character(1);
    npc.name = "Gwen".into();
    assert!(world.spawn_character(npc, 10, 10));

    assert!(world.npc_murmur(CharacterId(1), "psst"));
    let texts = world.drain_pending_area_texts();
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].message, "Gwen murmurs: \"psst\"");
    assert_eq!(texts[0].max_distance, world.settings.whisper_dist as u16);

    assert!(!world.npc_murmur(CharacterId(1), "bad \"quote\""));
    assert!(world.drain_pending_area_texts().is_empty());
}
