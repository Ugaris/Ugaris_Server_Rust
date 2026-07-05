use super::*;

#[test]
fn world_executes_teufel_arena_entry_and_clears_spells() {
    let mut world = World::default();
    let mut actor = character(1);
    actor.sprite = 27;
    actor.level = 38;
    actor.inventory[0] = Some(ItemId(20));
    actor.inventory[12] = Some(ItemId(21));
    assert!(world.spawn_character(actor, 150, 220));

    let mut arena = item(8, ItemFlags::USED | ItemFlags::USE);
    arena.driver = crate::item_driver::IDR_TEUFELARENA;
    arena.driver_data = vec![1];
    world.add_item(arena);

    let mut suit = item(20, ItemFlags::WNHEAD);
    suit.sprite = 53001;
    suit.carried_by = Some(CharacterId(1));
    world.add_item(suit);
    let mut spell = item(21, ItemFlags::USED);
    spell.carried_by = Some(CharacterId(1));
    world.add_item(spell);

    let outcome = world.execute_item_driver_request_with_context(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_TEUFELARENA,
            item_id: ItemId(8),
            character_id: CharacterId(1),
            spec: 0,
        },
        34,
        &ItemDriverContext {
            teufel_arena_roll: Some(1),
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::TeufelArena {
            item_id: ItemId(8),
            character_id: CharacterId(1),
            x: 134,
            y: 220,
        }
    );
    let actor = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!((actor.x, actor.y), (134, 220));
    assert_eq!(actor.inventory[12], None);
    assert!(!world.items.contains_key(&ItemId(21)));
    assert_eq!(
        world.drain_pending_system_texts(),
        vec![WorldSystemText {
            character_id: CharacterId(1),
            message: "All your spells have been removed.".to_string(),
        }]
    );
}

#[test]
fn player_magicshield_spell_sets_up_and_completes_lifeshield_gain() {
    let mut world = World::default();
    let mut character = character(1);
    character.mana = 10 * POWERSCALE;
    character.values[0][CharacterValue::MagicShield as usize] = 8;
    character.values[0][CharacterValue::Speed as usize] = 24;
    world.add_character(character);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::MagicShield,
        arg1: 0,
        arg2: 0,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::MAGICSHIELD);
    assert_eq!(character.act1, 8 * POWERSCALE);
    assert_eq!(character.mana, 6 * POWERSCALE);

    world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
    let completed = world.tick_basic_actions();

    assert_eq!(completed.len(), 1);
    assert!(completed[0].ok);
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.lifeshield, 8 * POWERSCALE);
    assert_eq!(character.action, 0);
    let effect = world.effects.values().next().unwrap();
    assert_eq!(effect.effect_type, EF_MAGICSHIELD);
    assert_eq!(effect.target_character, Some(CharacterId(1)));
    assert_eq!(effect.stop_tick, 3);
    assert_eq!(effect.light, 25);
    // C `act_magicshield` (`act.c:1090-1093`): `NT_CHAR` gated on
    // `CF_NONOTIFY`, then unconditional `NT_SPELL` with a `0` payload.
    assert_eq!(character.driver_messages[0].message_type, NT_CHAR);
    assert_eq!(character.driver_messages[0].dat1, 1);
    assert_eq!(character.driver_messages[1].message_type, NT_SPELL);
    assert_eq!(character.driver_messages[1].dat1, 1);
    assert_eq!(
        character.driver_messages[1].dat2,
        CharacterValue::MagicShield as i32
    );
    assert_eq!(character.driver_messages[1].dat3, 0);
}

#[test]
fn player_heal_spell_restores_target_hp_on_completion() {
    let mut world = World::default();
    let mut caster = character(1);
    caster.x = 10;
    caster.y = 10;
    caster.mana = 10 * POWERSCALE;
    caster.values[0][CharacterValue::Heal as usize] = 10;
    caster.values[0][CharacterValue::Speed as usize] = 24;
    let mut target = character(2);
    target.x = 11;
    target.y = 10;
    target.hp = 5 * POWERSCALE;
    target.values[0][CharacterValue::Hp as usize] = 10;
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 11, 10);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Heal,
        arg1: 2,
        arg2: 0,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(caster.action, action::HEAL1);
    assert_eq!(caster.dir, Direction::Right as u8);
    assert_eq!(caster.act2, 5 * POWERSCALE);
    assert_eq!(caster.mana, 15 * POWERSCALE / 2);

    world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
    let completed = world.tick_basic_actions();

    assert_eq!(completed.len(), 1);
    assert!(completed[0].ok);
    let target = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(target.hp, 10 * POWERSCALE);
    let effect = world.effects.values().next().unwrap();
    assert_eq!(effect.effect_type, EF_HEAL);
    assert_eq!(effect.target_character, Some(CharacterId(2)));
    assert_eq!(effect.stop_tick, 8);
    // C `act_heal` (`act.c:1671-1674`): `NT_CHAR` gated on `CF_NONOTIFY`,
    // then unconditional `NT_SPELL`, broadcast from the caster's own
    // position (not the healed target's) - both sit inside the notify box.
    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(caster.driver_messages[0].message_type, NT_CHAR);
    assert_eq!(caster.driver_messages[0].dat1, 1);
    assert_eq!(caster.driver_messages[1].message_type, NT_SPELL);
    assert_eq!(caster.driver_messages[1].dat1, 1);
    assert_eq!(caster.driver_messages[1].dat2, CharacterValue::Heal as i32);
}

#[test]
fn player_bless_spell_installs_carried_spell_item_on_completion() {
    let mut world = World::default();
    world.tick = Tick(100);
    let mut character = character(1);
    character.flags.insert(CharacterFlags::PLAYER);
    character.mana = 10 * POWERSCALE;
    character.values[0][CharacterValue::Bless as usize] = 40;
    // C `update_char` caps item/spell modifiers (including bless) at 50%
    // of the character's own raised `value[1]` for that attribute
    // (`create.c:1815-1819`), so a raised baseline is needed here for the
    // +10 bless bonus below to be visible rather than capped to 0.
    character.values[1][CharacterValue::Intelligence as usize] = 20;
    character.values[1][CharacterValue::Wisdom as usize] = 20;
    character.values[1][CharacterValue::Agility as usize] = 20;
    character.values[1][CharacterValue::Strength as usize] = 20;
    world.add_character(character);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Bless,
        arg1: 1,
        arg2: 0,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::BLESS_SELF);
    assert_eq!(character.act1, 1);
    assert_eq!(character.mana, 8 * POWERSCALE);

    world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
    let completed = world.tick_basic_actions();

    assert_eq!(completed.len(), 1);
    assert!(completed[0].ok);
    let character = world.characters.get(&CharacterId(1)).unwrap();
    let spell_id = character.inventory[29].unwrap();
    let spell = world.items.get(&spell_id).unwrap();
    assert_eq!(spell.name, "Bless");
    assert_eq!(spell.driver, IDR_BLESS);
    assert_eq!(spell.carried_by, Some(CharacterId(1)));
    assert_eq!(spell.modifier_index[..4], [4, 3, 5, 6]);
    assert_eq!(spell.modifier_value[..4], [10, 10, 10, 10]);
    // Effective value = raised base (20) + bless bonus, capped at 50% of
    // the raised base (20 * 0.5 = 10) = 30.
    assert_eq!(
        character.values[0][CharacterValue::Intelligence as usize],
        30
    );
    assert_eq!(character.values[0][CharacterValue::Wisdom as usize], 30);
    assert_eq!(character.values[0][CharacterValue::Agility as usize], 30);
    assert_eq!(character.values[0][CharacterValue::Strength as usize], 30);
    assert_eq!(
        u32::from_le_bytes(spell.driver_data[0..4].try_into().unwrap()),
        2_980
    );
    assert_eq!(
        u32::from_le_bytes(spell.driver_data[4..8].try_into().unwrap()),
        100
    );
    assert_eq!(
        i32::from_le_bytes(spell.driver_data[8..12].try_into().unwrap()),
        40
    );
    let effect = world.effects.values().next().unwrap();
    assert_eq!(effect.effect_type, EF_BLESS);
    assert_eq!(effect.target_character, Some(CharacterId(1)));
    assert_eq!(effect.start_tick, 100);
    assert_eq!(effect.stop_tick, 2_980);
    assert_eq!(effect.strength, 10);
    assert_eq!(world.timers.used_timers(), 1);
    // C `act_bless` (`act.c:1237-1240`): `NT_CHAR` gated on `CF_NONOTIFY`,
    // then unconditional `NT_SPELL` with a `0` payload.
    assert_eq!(character.driver_messages[0].message_type, NT_CHAR);
    assert_eq!(character.driver_messages[0].dat1, 1);
    assert_eq!(character.driver_messages[1].message_type, NT_SPELL);
    assert_eq!(character.driver_messages[1].dat1, 1);
    assert_eq!(
        character.driver_messages[1].dat2,
        CharacterValue::Bless as i32
    );
    assert_eq!(
        world.drain_pending_sound_specials()[0].special.special_type,
        29
    );

    world.tick = Tick(2_980);
    world.process_due_timers(1);
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert!(character.inventory[29].is_none());
    // Back to the raised baseline (20) once the bless bonus expires.
    assert_eq!(
        character.values[0][CharacterValue::Intelligence as usize],
        20
    );
    assert_eq!(character.values[0][CharacterValue::Wisdom as usize], 20);
    assert_eq!(character.values[0][CharacterValue::Agility as usize], 20);
    assert_eq!(character.values[0][CharacterValue::Strength as usize], 20);
}

#[test]
fn player_flash_spell_installs_timed_speed_spell_on_self() {
    let mut world = World::default();
    world.tick = Tick(200);
    let mut character = character(1);
    character.flags.insert(CharacterFlags::PLAYER);
    character.mana = 10 * POWERSCALE;
    character.values[0][CharacterValue::Flash as usize] = 40;
    world.add_character(character);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Flash,
        arg1: 0,
        arg2: 0,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.action, action::FLASH);
    assert_eq!(character.mana, 7 * POWERSCALE);

    world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
    assert!(world.tick_basic_actions()[0].ok);

    let character = world.characters.get(&CharacterId(1)).unwrap();
    let spell_id = character.inventory[29].unwrap();
    let spell = world.items.get(&spell_id).unwrap();
    assert_eq!(spell.driver, IDR_FLASH);
    assert_eq!(spell.modifier_index[0], CharacterValue::Speed as i16);
    assert_eq!(spell.modifier_value[0], 100);
    assert_eq!(spell.carried_by, Some(CharacterId(1)));
    assert_eq!(character.values[0][CharacterValue::Speed as usize], 100);
    assert_eq!(
        u32::from_le_bytes(spell.driver_data[0..4].try_into().unwrap()),
        248
    );
    assert_eq!(
        u32::from_le_bytes(spell.driver_data[4..8].try_into().unwrap()),
        200
    );
    let effect = world.effects.values().next().unwrap();
    assert_eq!(effect.effect_type, EF_FLASH);
    assert_eq!(effect.target_character, Some(CharacterId(1)));
    assert_eq!(effect.start_tick, 200);
    assert_eq!(effect.stop_tick, 248);
    assert_eq!(effect.light, 50);
    assert_eq!(effect.strength, 40);
    assert_eq!(world.timers.used_timers(), 1);
    // C `act_flash` (`act.c:1041-1044`): `NT_CHAR` gated on `CF_NONOTIFY`,
    // then unconditional `NT_SPELL` with a `0` payload.
    assert_eq!(character.driver_messages[0].message_type, NT_CHAR);
    assert_eq!(character.driver_messages[0].dat1, 1);
    assert_eq!(character.driver_messages[1].message_type, NT_SPELL);
    assert_eq!(character.driver_messages[1].dat1, 1);
    assert_eq!(
        character.driver_messages[1].dat2,
        CharacterValue::Flash as i32
    );
}

#[test]
fn player_freeze_spell_installs_negative_speed_spell_on_nearby_target() {
    let mut world = World::default();
    world.tick = Tick(300);
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.mana = 10 * POWERSCALE;
    caster.values[0][CharacterValue::Freeze as usize] = 50;
    let mut target = character(2);
    target.flags.insert(CharacterFlags::PLAYER);
    target.values[0][CharacterValue::Immunity as usize] = 30;
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 12, 10);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Freeze,
        arg1: 0,
        arg2: 0,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(caster.action, action::FREEZE);
    assert_eq!(caster.mana, 8 * POWERSCALE);

    world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
    assert!(world.tick_basic_actions()[0].ok);

    let target = world.characters.get(&CharacterId(2)).unwrap();
    let spell_id = target.inventory[29].unwrap();
    let spell = world.items.get(&spell_id).unwrap();
    assert_eq!(spell.driver, IDR_FREEZE);
    assert_eq!(spell.modifier_index[0], CharacterValue::Speed as i16);
    assert_eq!(spell.modifier_value[0], -420);
    assert_eq!(spell.carried_by, Some(CharacterId(2)));
    assert_eq!(target.values[0][CharacterValue::Speed as usize], -420);
    assert_eq!(
        u32::from_le_bytes(spell.driver_data[0..4].try_into().unwrap()),
        396
    );
    let effect = world.effects.values().next().unwrap();
    assert_eq!(effect.effect_type, EF_FREEZE);
    assert_eq!(effect.target_character, Some(CharacterId(2)));
    assert_eq!(effect.start_tick, 300);
    assert_eq!(effect.stop_tick, 396);
    assert_eq!(world.timers.used_timers(), 1);
    let sounds = world.drain_pending_sound_specials();
    assert!(sounds.iter().any(|sound| sound.special.special_type == 31));
    // C `act_freeze` (`act.c:1556-1560`): `NT_CHAR` gated on `CF_NONOTIFY`,
    // then unconditional `NT_SPELL` with a `0` payload, broadcast from the
    // caster - the target also observes both since it's inside the box.
    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(caster.driver_messages[0].message_type, NT_CHAR);
    assert_eq!(caster.driver_messages[0].dat1, 1);
    assert_eq!(caster.driver_messages[1].message_type, NT_SPELL);
    assert_eq!(caster.driver_messages[1].dat1, 1);
    assert_eq!(
        caster.driver_messages[1].dat2,
        CharacterValue::Freeze as i32
    );
}

#[test]
fn ice_demon_freeze_installs_legacy_curse_spell() {
    let mut world = World::default();
    world.tick = Tick(300);
    let mut caster = character(1);
    caster.name = "Ice Demon".into();
    caster
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::IDEMON);
    caster.mana = 10 * POWERSCALE;
    caster.values[0][CharacterValue::Freeze as usize] = 50;
    caster.values[1][CharacterValue::Demon as usize] = 10;
    let mut target = character(2);
    target.flags.insert(CharacterFlags::ALIVE);
    // C `V_COLD` is entirely item-driven (`update_char`, `create.c:1795-1798`:
    // `ch[cn].value[0][n] = ch[cn].value[1][n] = mod[n];`), so a Cold value
    // of 3 must come from a worn item's modifier, not a direct value poke -
    // otherwise the `update_char(co)` call inside `install_speed_spell`
    // (freeze) recomputes it back to 0 before the curse strength is read.
    target.inventory[0] = Some(ItemId(500));
    target.values[0][CharacterValue::Immunity as usize] = 30;
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 12, 10);
    let mut cold_item = item(500, ItemFlags::WNBODY);
    cold_item.carried_by = Some(CharacterId(2));
    cold_item.modifier_index[0] = CharacterValue::Cold as i16;
    cold_item.modifier_value[0] = 3;
    world.add_item(cold_item);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Freeze,
        arg1: 0,
        arg2: 0,
    };

    assert!(world.apply_player_action_setup(&mut player, 1));
    world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
    assert!(world.tick_basic_actions()[0].ok);

    let target = world.characters.get(&CharacterId(2)).unwrap();
    let curse_id = target.inventory[28].unwrap();
    let curse = world.items.get(&curse_id).unwrap();
    assert_eq!(curse.driver, IDR_CURSE);
    assert_eq!(curse.modifier_index[0], CharacterValue::Intelligence as i16);
    assert_eq!(curse.modifier_index[1], CharacterValue::Wisdom as i16);
    assert_eq!(curse.modifier_index[2], CharacterValue::Agility as i16);
    assert_eq!(curse.modifier_index[3], CharacterValue::Strength as i16);
    assert_eq!(curse.modifier_value[..4], [-7, -7, -7, -7]);
    assert_eq!(curse.carried_by, Some(CharacterId(2)));
    // C `update_char` (`create.c:1863-1865`) floors every base attribute
    // (`n <= V_STR`) at 0 once totalled; the target's raised value[1] is
    // 0 here, so the -7 curse modifier is visible on the item
    // (`curse.modifier_value` above) but the character's effective value
    // stays clamped at 0, not negative.
    assert_eq!(target.values[0][CharacterValue::Intelligence as usize], 0);
    assert_eq!(target.values[0][CharacterValue::Wisdom as usize], 0);
    assert_eq!(target.values[0][CharacterValue::Agility as usize], 0);
    assert_eq!(target.values[0][CharacterValue::Strength as usize], 0);
    assert_eq!(
        u32::from_le_bytes(curse.driver_data[0..4].try_into().unwrap()),
        43_500
    );
    assert_eq!(
        u32::from_le_bytes(curse.driver_data[4..8].try_into().unwrap()),
        300
    );
    let curse_effect = world
        .effects
        .values()
        .find(|effect| effect.effect_type == EF_CURSE)
        .unwrap();
    assert_eq!(curse_effect.target_character, Some(CharacterId(2)));
    assert_eq!(curse_effect.start_tick, 300);
    assert_eq!(curse_effect.stop_tick, 43_500);
    assert_eq!(curse_effect.strength, 7);
    assert_eq!(world.timers.used_timers(), 2);
    assert_eq!(
        world.drain_pending_system_texts(),
        vec![WorldSystemText {
            character_id: CharacterId(2),
            message: "You have been frozen by Ice Demon. You feel like you'll never thaw again."
                .into(),
        }]
    );
}

#[test]
fn curse_spell_stack_uses_existing_slot_and_caps_strength() {
    let mut world = World::default();
    world.tick = Tick(100);
    let mut target = character(2);
    target.flags.insert(CharacterFlags::ALIVE);
    world.spawn_character(target, 10, 10);

    assert!(world.install_curse_spell(CharacterId(2), 7, 10));
    assert!(world.install_curse_spell(CharacterId(2), 7, 10));

    let target = world.characters.get(&CharacterId(2)).unwrap();
    let curse_id = target.inventory[29].unwrap();
    let curse = world.items.get(&curse_id).unwrap();
    assert_eq!(curse.driver, IDR_CURSE);
    assert_eq!(curse.modifier_value[..4], [-10, -10, -10, -10]);
    assert!(target.inventory[28].is_none());
    // C `update_char` floors every base attribute at 0 (`create.c:1863-1865`,
    // `n <= V_STR`); with an unraised `value[1]` of 0, the -10 curse
    // modifier is visible on the item but clamps the character's
    // effective value at 0.
    assert_eq!(target.values[0][CharacterValue::Intelligence as usize], 0);
    assert_eq!(target.values[0][CharacterValue::Wisdom as usize], 0);
    assert_eq!(target.values[0][CharacterValue::Agility as usize], 0);
    assert_eq!(target.values[0][CharacterValue::Strength as usize], 0);
    let effects: Vec<_> = world
        .effects
        .values()
        .filter(|effect| effect.effect_type == EF_CURSE)
        .collect();
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].strength, 10);
    assert_eq!(world.timers.used_timers(), 1);
}

#[test]
fn beyond_potion_installs_timed_potion_spell_and_consumes_potion() {
    let mut world = World::default();
    world.tick = Tick(1_200);
    let mut character = character(1);
    character
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::WARRIOR);
    character.level = 20;
    character.inventory[30] = Some(ItemId(7));
    world.add_character(character);

    let mut potion = item(
        7,
        ItemFlags::USED | ItemFlags::USE | ItemFlags::BEYONDMAXMOD,
    );
    potion.driver = crate::item_driver::IDR_BEYONDPOTION;
    potion.carried_by = Some(CharacterId(1));
    potion.driver_data = vec![3];
    potion.modifier_index = [
        CharacterValue::Strength as i16,
        CharacterValue::Agility as i16,
        0,
        0,
        0,
    ];
    potion.modifier_value = [5, 6, 0, 0, 0];
    world.add_item(potion);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_BEYONDPOTION,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(outcome, ItemDriverOutcome::BeyondPotion { .. }));
    assert!(!world.items.contains_key(&ItemId(7)));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.inventory[30], None);
    let spell_id = character.inventory[29].unwrap();
    assert!(character.flags.contains(CharacterFlags::ITEMS));
    assert!(character.flags.contains(CharacterFlags::UPDATE));
    let spell = world.items.get(&spell_id).unwrap();
    assert_eq!(spell.driver, IDR_POTION_SP);
    assert_eq!(spell.carried_by, Some(CharacterId(1)));
    assert_eq!(spell.modifier_index[0], CharacterValue::Strength as i16);
    assert_eq!(spell.modifier_value[0], 5);
    assert!(spell.flags.contains(ItemFlags::BEYONDMAXMOD));
    assert_eq!(read_spell_expire_tick(&spell.driver_data), Some(5_520));
    let effect = world.effects.values().next().unwrap();
    assert_eq!(effect.effect_type, EF_POTION);
    assert_eq!(effect.target_character, Some(CharacterId(1)));
    assert_eq!(effect.start_tick, 1_200);
    assert_eq!(effect.stop_tick, 5_520);
    assert_eq!(effect.strength, 5);
    assert_eq!(world.timers.used_timers(), 1);
}

#[test]
fn beyond_potion_blocks_while_another_potion_spell_is_active() {
    let mut world = World::default();
    let mut character = character(1);
    character.flags.insert(CharacterFlags::PLAYER);
    character.inventory[12] = Some(ItemId(8));
    character.inventory[30] = Some(ItemId(7));
    world.add_character(character);

    let mut active = item(8, ItemFlags::USED);
    active.driver = IDR_POTION_SP;
    active.carried_by = Some(CharacterId(1));
    active.driver_data = 10_000_u32.to_le_bytes().to_vec();
    world.add_item(active);
    let mut potion = item(7, ItemFlags::USED | ItemFlags::USE);
    potion.driver = crate::item_driver::IDR_BEYONDPOTION;
    potion.carried_by = Some(CharacterId(1));
    potion.driver_data = vec![3];
    world.add_item(potion);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_BEYONDPOTION,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::BlockedByRequirements { .. }
    ));
    assert!(world.items.contains_key(&ItemId(7)));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.inventory[12], Some(ItemId(8)));
    assert_eq!(character.inventory[30], Some(ItemId(7)));
}

#[test]
fn finished_alchemy_flask_installs_timed_potion_spell_and_resets_flask() {
    let mut world = World::default();
    world.tick = Tick(2_000);
    let mut character = character(1);
    character
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::ARCH);
    character.inventory[30] = Some(ItemId(7));
    world.add_character(character);

    let mut flask = item(7, ItemFlags::USED | ItemFlags::USE);
    flask.driver = crate::item_driver::IDR_FLASK;
    flask.carried_by = Some(CharacterId(1));
    flask.name = "Magical Potion".to_string();
    flask.description = "A flask containing a magical liquid.".to_string();
    flask.sprite = 50214;
    flask.value = 999;
    flask.needs_class = 8;
    flask.driver_data = vec![2, 3, 1, 10];
    flask.modifier_index = [CharacterValue::Agility as i16, 0, 0, 0, 0];
    flask.modifier_value = [4, 0, 0, 0, 0];
    world.add_item(flask);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: crate::item_driver::IDR_FLASK,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(
        matches!(outcome, ItemDriverOutcome::AlchemyFlaskPotion { .. }),
        "unexpected outcome: {outcome:?}"
    );
    let reset_flask = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(reset_flask.name, "Empty Potion");
    assert_eq!(reset_flask.description, "A flask made of glass.");
    assert_eq!(reset_flask.sprite, 10294);
    assert_eq!(reset_flask.driver_data, vec![2]);
    assert_eq!(reset_flask.modifier_index, [0; MAX_MODIFIERS]);
    assert_eq!(reset_flask.modifier_value, [0; MAX_MODIFIERS]);
    assert_eq!(reset_flask.value, 10);
    assert_eq!(reset_flask.needs_class, 0);
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.inventory[30], Some(ItemId(7)));
    let spell_id = character.inventory[29].unwrap();
    let spell = world.items.get(&spell_id).unwrap();
    assert_eq!(spell.driver, IDR_POTION_SP);
    assert_eq!(spell.modifier_index[0], CharacterValue::Agility as i16);
    assert_eq!(spell.modifier_value[0], 4);
    assert!(!spell.flags.contains(ItemFlags::BEYONDMAXMOD));
    assert_eq!(read_spell_expire_tick(&spell.driver_data), Some(16_400));
    let effect = world.effects.values().next().unwrap();
    assert_eq!(effect.effect_type, EF_POTION);
    assert_eq!(effect.strength, 4);
}

#[test]
fn world_spell_timer_removes_carried_spell_at_expiry() {
    let mut world = World::default();
    world.tick = Tick(100);
    let mut character = character(1);
    character.flags.insert(CharacterFlags::PLAYER);
    character.mana = 10 * POWERSCALE;
    character.values[0][CharacterValue::Bless as usize] = 40;
    world.add_character(character);

    assert!(world.setup_bless_spell(CharacterId(1), CharacterId(1)));
    world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
    assert!(world.tick_basic_actions()[0].ok);
    let spell_id = world.characters.get(&CharacterId(1)).unwrap().inventory[29].unwrap();

    world.tick = Tick(2_979);
    assert!(world.process_due_timers(1).is_empty());
    assert!(world.items.contains_key(&spell_id));
    world.tick = Tick(2_980);
    assert!(world.process_due_timers(1).is_empty());

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.inventory[29], None);
    assert!(character.flags.contains(CharacterFlags::ITEMS));
    assert!(character.flags.contains(CharacterFlags::UPDATE));
    assert!(!world.items.contains_key(&spell_id));
}

#[test]
fn freeze_spell_timer_restores_speed_and_rescales_current_action() {
    let mut world = World::default();
    let mut character = character(1);
    character.inventory[12] = Some(ItemId(7));
    character.values[0][CharacterValue::Speed as usize] = -420;
    character.duration = 50;
    character.step = 25;
    let mut spell = item(7, ItemFlags::USED);
    spell.driver = IDR_FREEZE;
    spell.carried_by = Some(CharacterId(1));
    spell.modifier_index[0] = CharacterValue::Speed as i16;
    spell.modifier_value[0] = -420;
    spell.driver_data = 110_u32.to_le_bytes().to_vec();
    world.add_character(character);
    world.add_item(spell);

    assert_eq!(world.schedule_existing_spell_timers(), 1);
    world.tick = Tick(110);
    assert!(world.process_due_timers(1).is_empty());

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.inventory[12], None);
    assert_eq!(character.values[0][CharacterValue::Speed as usize], 0);
    assert_eq!(character.duration, 13);
    assert_eq!(character.step, 6);
    assert!(!world.items.contains_key(&ItemId(7)));
}

#[test]
fn world_spell_timer_serial_guard_preserves_refreshed_spell() {
    let mut world = World::default();
    let mut character = character(1);
    character.inventory[12] = Some(ItemId(7));
    let mut stale_spell = item(7, ItemFlags::USED);
    stale_spell.driver = IDR_BLESS;
    stale_spell.carried_by = Some(CharacterId(1));
    stale_spell.serial = 7;
    stale_spell.driver_data = 10_u32.to_le_bytes().to_vec();
    world.add_character(character);
    world.add_item(stale_spell);

    assert_eq!(world.schedule_existing_spell_timers(), 1);
    world.items.get_mut(&ItemId(7)).unwrap().serial = 8;
    world.tick = Tick(10);
    assert!(world.process_due_timers(1).is_empty());

    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().inventory[12],
        Some(ItemId(7))
    );
    assert!(world.items.contains_key(&ItemId(7)));
}

#[test]
fn player_bless_spell_replaces_near_expired_spell_in_same_slot() {
    let mut world = World::default();
    world.tick = Tick(1_000);
    let mut character = character(1);
    character.flags.insert(CharacterFlags::PLAYER);
    character.mana = 10 * POWERSCALE;
    character.values[0][CharacterValue::Bless as usize] = 80;
    character.inventory[18] = Some(ItemId(7));
    let mut old_spell = item(7, ItemFlags::USED);
    old_spell.driver = IDR_BLESS;
    old_spell.carried_by = Some(CharacterId(1));
    old_spell.driver_data = vec![0; 12];
    old_spell.driver_data[0..4].copy_from_slice(&1_100_u32.to_le_bytes());
    world.add_character(character);
    world.add_item(old_spell);

    assert!(world.setup_bless_spell(CharacterId(1), CharacterId(1)));
    world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
    assert!(world.tick_basic_actions()[0].ok);

    let character = world.characters.get(&CharacterId(1)).unwrap();
    let new_spell_id = character.inventory[18].unwrap();
    assert_ne!(new_spell_id, ItemId(7));
    assert!(!world.items.contains_key(&ItemId(7)));
    assert_eq!(
        world.items.get(&new_spell_id).unwrap().modifier_value[0],
        20
    );
}

#[test]
fn poison_character_installs_legacy_timed_poison_spell() {
    let mut world = World::default();
    world.tick = Tick(500);
    let mut character = character(1);
    character.hp = 10 * POWERSCALE;
    world.add_character(character);

    assert!(world.poison_character(CharacterId(1), 7, 2));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    let spell_id = character.inventory[29].unwrap();
    let spell = world.items.get(&spell_id).unwrap();
    assert_eq!(spell.name, "Poison");
    assert_eq!(spell.driver, IDR_POISON2);
    assert_eq!(spell.carried_by, Some(CharacterId(1)));
    assert_eq!(spell.modifier_index[0], CharacterValue::Hp as i16);
    assert_eq!(spell.modifier_value[0], -1);
    assert_eq!(read_spell_expire_tick(&spell.driver_data), Some(173_300));
    assert_eq!(read_poison_power(&spell.driver_data), Some(7));
    assert_eq!(read_poison_tick(&spell.driver_data), Some(9));
    assert_eq!(world.timers.used_timers(), 2);
}

#[test]
fn poison_callback_damages_and_reschedules_while_spell_is_carried() {
    let mut world = World::default();
    world.tick = Tick(1_000);
    let mut character = character(1);
    character.hp = 10 * POWERSCALE;
    // C `update_char` (`create.c:1863-1864`) clamps current HP down to
    // the newly recomputed max whenever it's lower; give the character a
    // large enough raised HP baseline that installing the poison spell's
    // small `-1` modifier doesn't itself clamp `hp` before the damage tick
    // below runs.
    character.values[1][CharacterValue::Hp as usize] = 100;
    world.add_character(character);
    assert!(world.poison_character(CharacterId(1), 4, 0));
    let spell_id = world.characters[&CharacterId(1)].inventory[29].unwrap();

    world.tick = Tick(1_000 + TICKS_PER_SECOND);
    world.process_due_timers(1);

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.hp, 10 * POWERSCALE - POWERSCALE / 3);
    let spell = world.items.get(&spell_id).unwrap();
    assert_eq!(read_poison_tick(&spell.driver_data), Some(8));
    assert_eq!(spell.modifier_value[0], -1);
    assert_eq!(world.timers.used_timers(), 2);
}

#[test]
fn poison_callback_weakens_hp_modifier_every_tenth_tick() {
    let mut world = World::default();
    world.tick = Tick(2_000);
    let mut character = character(1);
    character.hp = 10 * POWERSCALE;
    world.add_character(character);
    assert!(world.poison_character(CharacterId(1), 20, 3));
    let spell_id = world.characters[&CharacterId(1)].inventory[29].unwrap();
    write_poison_tick(&mut world.items.get_mut(&spell_id).unwrap().driver_data, 0);

    world.tick = Tick(2_000 + TICKS_PER_SECOND);
    world.process_due_timers(1);

    let spell = world.items.get(&spell_id).unwrap();
    assert_eq!(spell.driver, IDR_POISON3);
    assert_eq!(spell.modifier_value[0], -2);
    assert_eq!(read_poison_tick(&spell.driver_data), Some(9));
}

#[test]
fn remove_poison_helpers_clear_spell_slots() {
    let mut world = World::default();
    world.add_character(character(1));
    assert!(world.poison_character(CharacterId(1), 5, 1));
    let spell_id = world.characters[&CharacterId(1)].inventory[29].unwrap();

    assert!(world.remove_poison(CharacterId(1), 1));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.inventory[29], None);
    assert!(!world.items.contains_key(&spell_id));
    assert!(character.flags.contains(CharacterFlags::ITEMS));
}

#[test]
fn special_potion_antidote_clears_matching_poison_and_consumes_item() {
    let mut world = World::default();
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(10));
    world.add_character(character);
    assert!(world.poison_character(CharacterId(1), 5, 2));
    let poison_id = world.characters[&CharacterId(1)].inventory[29].unwrap();
    let mut potion = item(10, ItemFlags::USED);
    potion.carried_by = Some(CharacterId(1));
    potion.driver = IDR_SPECIAL_POTION;
    potion.driver_data = vec![2];
    world.items.insert(ItemId(10), potion);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_SPECIAL_POTION,
            item_id: ItemId(10),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::SpecialPotionAntidote {
            kind: 2,
            poison_removed: true,
            ..
        }
    ));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.inventory[29], None);
    assert_eq!(character.inventory[30], None);
    assert!(!world.items.contains_key(&poison_id));
    assert!(!world.items.contains_key(&ItemId(10)));
}

#[test]
fn special_potion_infravision_installs_timed_spell_and_consumes_item() {
    let mut world = World::default();
    world.tick = Tick(42);
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(10));
    world.add_character(character);
    let mut potion = item(10, ItemFlags::USED);
    potion.carried_by = Some(CharacterId(1));
    potion.driver = IDR_SPECIAL_POTION;
    potion.driver_data = vec![6];
    world.items.insert(ItemId(10), potion);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_SPECIAL_POTION,
            item_id: ItemId(10),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::SpecialPotionInfravision {
            installed: true,
            ..
        }
    ));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    let spell_id = character.inventory[29].unwrap();
    let spell = world.items.get(&spell_id).unwrap();
    assert_eq!(spell.name, "Infravision");
    assert_eq!(spell.driver, IDR_INFRARED);
    assert_eq!(read_spell_expire_tick(&spell.driver_data), Some(14_442));
    assert_eq!(character.inventory[30], None);
    assert!(character.flags.contains(CharacterFlags::INFRAVISION));
    assert!(!world.items.contains_key(&ItemId(10)));

    world.tick = Tick(14_442);
    world.process_due_timers(1);
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert!(!character.flags.contains(CharacterFlags::INFRAVISION));
    assert!(character.flags.contains(CharacterFlags::UPDATE));
}

#[test]
fn oxy_potion_installs_one_minute_oxygen_spell_and_consumes_item() {
    let mut world = World::default();
    world.tick = Tick(77);
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(10));
    world.add_character(character);
    let mut potion = item(10, ItemFlags::USED);
    potion.carried_by = Some(CharacterId(1));
    potion.driver = IDR_OXYPOTION;
    world.items.insert(ItemId(10), potion);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_OXYPOTION,
            item_id: ItemId(10),
            character_id: CharacterId(1),
            spec: 0,
        },
        31,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::OxygenPotion {
            installed: true,
            ..
        }
    ));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    let spell_id = character.inventory[29].unwrap();
    let spell = world.items.get(&spell_id).unwrap();
    assert_eq!(spell.name, "Oxygen");
    assert_eq!(spell.driver, IDR_OXYGEN);
    assert_eq!(read_spell_expire_tick(&spell.driver_data), Some(1_517));
    assert_eq!(character.inventory[30], None);
    assert!(character.flags.contains(CharacterFlags::OXYGEN));
    assert!(character.flags.contains(CharacterFlags::ITEMS));
    assert!(character.flags.contains(CharacterFlags::UPDATE));
    assert!(!world.items.contains_key(&ItemId(10)));

    world.tick = Tick(1_517);
    world.process_due_timers(31);
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert!(!character.flags.contains(CharacterFlags::OXYGEN));
    assert!(character.flags.contains(CharacterFlags::UPDATE));
}

#[test]
fn brannington_underwater_berry_installs_thirty_second_oxygen_spell_and_consumes_item() {
    let mut world = World::default();
    world.tick = Tick(200);
    let mut character = character(1);
    character.flags.insert(CharacterFlags::PLAYER);
    character.inventory[30] = Some(ItemId(10));
    world.add_character(character);

    let mut berry = item(10, ItemFlags::USED | ItemFlags::USE);
    berry.carried_by = Some(CharacterId(1));
    berry.driver = IDR_BRANNINGTONFOREST;
    berry.driver_data = vec![1];
    world.items.insert(ItemId(10), berry);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_BRANNINGTONFOREST,
            item_id: ItemId(10),
            character_id: CharacterId(1),
            spec: 0,
        },
        28,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::BranningtonUnderwaterBerry {
            duration_ticks,
            installed: true,
            ..
        } if duration_ticks == 30 * TICKS_PER_SECOND
    ));
    assert!(!world.items.contains_key(&ItemId(10)));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.inventory[30], None);
    let spell_id = character.inventory[29].unwrap();
    let spell = world.items.get(&spell_id).unwrap();
    assert_eq!(spell.driver, IDR_OXYGEN);
    assert_eq!(
        read_spell_expire_tick(&spell.driver_data),
        Some(200 + (30 * TICKS_PER_SECOND) as u32)
    );
    assert!(character.flags.contains(CharacterFlags::OXYGEN));
}

#[test]
fn lab3_yellow_berry_replaces_existing_oxygen_with_fresh_duration() {
    let mut world = World::default();
    world.tick = Tick(100);
    let mut character = character(1);
    character.inventory[12] = Some(ItemId(12));
    character.inventory[30] = Some(ItemId(10));
    world.add_character(character);

    let mut old_oxygen = item(12, ItemFlags::USED);
    old_oxygen.carried_by = Some(CharacterId(1));
    old_oxygen.driver = IDR_OXYGEN;
    old_oxygen.driver_data = 10_000u32.to_le_bytes().to_vec();
    world.items.insert(ItemId(12), old_oxygen);

    let mut berry = item(10, ItemFlags::USED | ItemFlags::USE);
    berry.carried_by = Some(CharacterId(1));
    berry.driver = IDR_LAB3_PLANT;
    berry.driver_data = vec![5, 2, 3];
    world.items.insert(ItemId(10), berry);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_LAB3_PLANT,
            item_id: ItemId(10),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::Lab3YellowBerry {
            duration_ticks,
            installed: true,
            ..
        } if duration_ticks == 24 * TICKS_PER_SECOND
    ));
    assert!(!world.items.contains_key(&ItemId(12)));
    assert!(!world.items.contains_key(&ItemId(10)));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.inventory[12], None);
    assert_eq!(character.inventory[30], None);
    let spell_id = character.inventory[29].unwrap();
    let spell = world.items.get(&spell_id).unwrap();
    assert_eq!(spell.driver, IDR_OXYGEN);
    assert_eq!(
        read_spell_expire_tick(&spell.driver_data),
        Some(100 + (24 * TICKS_PER_SECOND) as u32)
    );
    assert!(character.flags.contains(CharacterFlags::OXYGEN));
}

#[test]
fn lab3_brown_berry_installs_timed_underwater_talk_spell() {
    let mut world = World::default();
    world.tick = Tick(200);
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(10));
    world.add_character(character);

    let mut berry = item(10, ItemFlags::USED | ItemFlags::USE);
    berry.carried_by = Some(CharacterId(1));
    berry.driver = IDR_LAB3_PLANT;
    berry.driver_data = vec![11];
    world.items.insert(ItemId(10), berry);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_LAB3_PLANT,
            item_id: ItemId(10),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::Lab3BrownBerry {
            duration_ticks,
            installed: true,
            ..
        } if duration_ticks == 10 * TICKS_PER_SECOND
    ));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    let spell_id = character.inventory[29].unwrap();
    let spell = world.items.get(&spell_id).unwrap();
    assert_eq!(spell.driver, IDR_UWTALK);
    assert_eq!(
        read_spell_expire_tick(&spell.driver_data),
        Some(200 + (10 * TICKS_PER_SECOND) as u32)
    );
    assert_eq!(character.inventory[30], None);

    world.tick = Tick(200 + 10 * TICKS_PER_SECOND);
    world.process_due_timers(22);
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.inventory[29], None);
}

#[test]
fn lab3_white_berry_creates_and_refreshes_decaying_light_item() {
    let mut world = World::default();
    world.tick = Tick(300);
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(10));
    world.add_character(character);

    let mut berry = item(10, ItemFlags::USED | ItemFlags::USE);
    berry.carried_by = Some(CharacterId(1));
    berry.driver = IDR_LAB3_PLANT;
    berry.driver_data = vec![6, 2, 1];
    world.items.insert(ItemId(10), berry);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_LAB3_PLANT,
            item_id: ItemId(10),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::Lab3WhiteBerry {
            light_power: 60,
            started_emit: true,
            installed: true,
            ..
        }
    ));
    assert!(!world.items.contains_key(&ItemId(10)));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    let light_id = character.inventory[29].unwrap();
    assert_eq!(character.values[0][CharacterValue::Light as usize], 80);
    let light = world.items.get(&light_id).unwrap();
    assert_eq!(light.driver, IDR_LAB3_PLANT);
    assert_eq!(light.driver_data.first(), Some(&10));
    assert_eq!(light.modifier_index[0], CharacterValue::Light as i16);
    assert_eq!(light.modifier_value[0], 80);

    let mut second = item(20, ItemFlags::USED | ItemFlags::USE);
    second.carried_by = Some(CharacterId(1));
    second.driver = IDR_LAB3_PLANT;
    second.driver_data = vec![6, 1, 0];
    world.items.insert(ItemId(20), second);
    if let Some(character) = world.characters.get_mut(&CharacterId(1)) {
        character.inventory[30] = Some(ItemId(20));
    }

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_LAB3_PLANT,
            item_id: ItemId(20),
            character_id: CharacterId(1),
            spec: 0,
        },
        22,
    );

    match outcome {
        ItemDriverOutcome::Lab3WhiteBerry {
            light_power: 10,
            started_emit: false,
            installed: true,
            ..
        } => {}
        other => panic!("unexpected whiteberry refresh outcome: {other:?}"),
    }
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.inventory[29], Some(light_id));
    assert_eq!(character.values[0][CharacterValue::Light as usize], 90);
    assert_eq!(world.items.get(&light_id).unwrap().modifier_value[0], 90);
}

#[test]
fn lab3_whiteberry_light_timer_decays_and_destroys_low_light() {
    let mut world = World::default();
    world.tick = Tick(10);
    let mut character = character(1);
    character.inventory[12] = Some(ItemId(12));
    character.values[0][CharacterValue::Light as usize] = 12;
    world.add_character(character);
    let mut light = item(12, ItemFlags::USED);
    light.carried_by = Some(CharacterId(1));
    light.driver = IDR_LAB3_PLANT;
    light.driver_data = vec![10, 0, 0, 12];
    light.modifier_index[0] = CharacterValue::Light as i16;
    light.modifier_value[0] = 12;
    world.items.insert(ItemId(12), light);

    assert!(world.schedule_item_driver_timer_with_context(
        ItemId(12),
        CharacterId(0),
        20 * TICKS_PER_SECOND,
        true,
    ));
    world.tick = Tick(10 + 20 * TICKS_PER_SECOND);
    let outcomes = world.process_due_timers(22);
    assert_eq!(
        outcomes,
        vec![ItemDriverOutcome::Lab3WhiteBerryLightTick {
            item_id: ItemId(12),
            destroyed: false,
        }]
    );
    assert_eq!(world.items.get(&ItemId(12)).unwrap().modifier_value[0], 9);
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().values[0][CharacterValue::Light as usize],
        9
    );

    world.tick = Tick(10 + 40 * TICKS_PER_SECOND);
    let outcomes = world.process_due_timers(22);
    assert_eq!(
        outcomes,
        vec![ItemDriverOutcome::Lab3WhiteBerryLightTick {
            item_id: ItemId(12),
            destroyed: true,
        }]
    );
    assert!(!world.items.contains_key(&ItemId(12)));
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.inventory[12], None);
    assert_eq!(character.values[0][CharacterValue::Light as usize], 0);
}

#[test]
fn existing_driver_spell_items_refresh_legacy_character_flags() {
    let mut world = World::default();
    let mut character = character(1);
    character.inventory[12] = Some(ItemId(12));
    character.inventory[13] = Some(ItemId(13));
    character.inventory[30] = Some(ItemId(30));
    world.add_character(character);

    let mut nonomagic = item(12, ItemFlags::USED);
    nonomagic.carried_by = Some(CharacterId(1));
    nonomagic.driver = IDR_NONOMAGIC;
    nonomagic.driver_data = 100u32.to_le_bytes().to_vec();
    world.items.insert(ItemId(12), nonomagic);

    let mut oxygen = item(13, ItemFlags::USED);
    oxygen.carried_by = Some(CharacterId(1));
    oxygen.driver = IDR_OXYGEN;
    oxygen.driver_data = 200u32.to_le_bytes().to_vec();
    world.items.insert(ItemId(13), oxygen);

    let mut ignored_infravision = item(30, ItemFlags::USED);
    ignored_infravision.carried_by = Some(CharacterId(1));
    ignored_infravision.driver = IDR_INFRARED;
    ignored_infravision.driver_data = 300u32.to_le_bytes().to_vec();
    world.items.insert(ItemId(30), ignored_infravision);

    assert_eq!(world.schedule_existing_spell_timers(), 3);
    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert!(character.flags.contains(CharacterFlags::NONOMAGIC));
    assert!(character.flags.contains(CharacterFlags::OXYGEN));
    assert!(!character.flags.contains(CharacterFlags::INFRAVISION));
}

#[test]
fn action_tick_attack_policy_can_block_area_spell_targets() {
    let mut world = World::default();
    world.tick = Tick(500);
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.mana = 100 * POWERSCALE;
    caster.values[0][CharacterValue::Mana as usize] = 100;
    caster.values[0][CharacterValue::Pulse as usize] = 200;
    let mut target = character(2);
    target
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::PK);
    target.hp = 10 * POWERSCALE;
    target.lifeshield = POWERSCALE;
    target.values[0][CharacterValue::Hp as usize] = 100;
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 12, 10);
    let mut player = PlayerRuntime::connected(1, 0);
    player.character_id = Some(CharacterId(1));
    player.action = QueuedAction {
        action: PlayerActionCode::Pulse,
        arg1: 0,
        arg2: 0,
    };

    assert!(world.apply_player_action_setup(&mut player, 2));
    world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
    let completed =
        world.tick_basic_actions_with_attack_policy(|_caster_id, _caster, target, _map| {
            target.id != CharacterId(2)
        });

    assert!(completed[0].ok);
    let target = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(target.hp, 10 * POWERSCALE);
    assert_eq!(target.lifeshield, POWERSCALE);
    // C `act_pulse` (`act.c:1637-1640`): `NT_CHAR`/`NT_SPELL` are an
    // unconditional area broadcast from the caster's position, independent
    // of whether any individual target's damage was blocked by the attack
    // policy - so the blocked target still observes both messages, but no
    // `NT_GOTHIT`/`NT_SEEHIT` (which only fire from `pulse_someone` when
    // damage is actually applied).
    assert_eq!(
        target.driver_messages,
        vec![
            crate::character_driver::CharacterDriverMessage {
                message_type: NT_CHAR,
                dat1: 1,
                dat2: 0,
                dat3: 0,
                text: None,
            },
            crate::character_driver::CharacterDriverMessage {
                message_type: NT_SPELL,
                dat1: 1,
                dat2: CharacterValue::Pulse as i32,
                dat3: 0,
                text: None,
            },
        ]
    );
    assert!(world
        .effects
        .values()
        .any(|effect| effect.effect_type == EF_PULSE && effect.x == 10 && effect.y == 10));
    assert!(!world
        .effects
        .values()
        .any(|effect| effect.effect_type == EF_PULSEBACK));
}

#[test]
fn completed_firering_notifies_nearby_characters_with_nt_char_and_nt_spell() {
    // C `act_firering` (`act.c:935-941`): `NT_CHAR` gated on `CF_NONOTIFY`,
    // then unconditional `NT_SPELL` carrying the firering effect id, guarded
    // by an "is the caster still alive" check (`if (ch[cn].flags)`) since
    // `hurt` might kill the caster indirectly.
    let mut world = World::default();
    world.tick = Tick(700);
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.values[0][CharacterValue::Fireball as usize] = 20;
    let mut target = character(2);
    target.hp = 1_000_000;
    world.spawn_character(caster, 10, 10);
    world.spawn_character(target, 11, 10);
    world.characters.get_mut(&CharacterId(1)).unwrap().action = action::FIRERING;
    world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;

    let completed = world.tick_basic_actions();
    assert!(completed[0].ok);

    let caster = world.characters.get(&CharacterId(1)).unwrap();
    // Index 0 is the unrelated `NT_DIDHIT` that `apply_legacy_hurt` queues to
    // the attacker when the target actually took damage.
    assert_eq!(caster.driver_messages[0].message_type, NT_DIDHIT);
    assert_eq!(caster.driver_messages[1].message_type, NT_CHAR);
    assert_eq!(caster.driver_messages[1].dat1, 1);
    assert_eq!(caster.driver_messages[2].message_type, NT_SPELL);
    assert_eq!(caster.driver_messages[2].dat1, 1);
    assert_eq!(
        caster.driver_messages[2].dat2,
        CharacterValue::Fireball as i32
    );
    let target = world.characters.get(&CharacterId(2)).unwrap();
    assert!(target.hp < 1_000_000);
    assert!(target
        .driver_messages
        .iter()
        .any(|message| message.message_type == NT_SPELL
            && message.dat2 == CharacterValue::Fireball as i32));
}

#[test]
fn world_fdemon_cannon_pulse_shoots_target_and_drains_loader() {
    let mut world = World::default();
    let mut cannon = item(7, ItemFlags::USED);
    cannon.driver = IDR_FDEMONCANNON;
    cannon.x = 10;
    cannon.y = 10;
    cannon.sprite = 14210;
    cannon.driver_data = vec![0; 13];
    cannon.driver_data[12] = Direction::Right as u8;
    world.add_item(cannon);

    for (id, nr) in [(11, 1), (12, 2), (13, 3)] {
        let mut loader = item(id, ItemFlags::USED);
        loader.driver = IDR_FDEMONLOADER;
        loader.x = 8 + nr;
        loader.y = 12;
        loader.driver_data = vec![nr as u8, 0, 0];
        loader.driver_data[1..3].copy_from_slice(&300u16.to_le_bytes());
        world.add_item(loader);
    }

    let target = character(1);
    assert!(world.spawn_character(target, 15, 10));

    let outcome = world.execute_item_driver_timer_request(
        ItemDriverRequest::Driver {
            driver: IDR_FDEMONCANNON,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        8,
        &ItemDriverContext {
            timer_call: true,
            ..ItemDriverContext::default()
        },
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::FdemonCannonPulse {
            item_id: ItemId(7),
            character_id: CharacterId(0),
            schedule_after_ticks: TICKS_PER_SECOND,
        }
    );
    let effect = world.effects.values().next().unwrap();
    assert_eq!(effect.effect_type, EF_EDEMONBALL);
    assert_eq!((effect.from_x, effect.from_y), (11, 10));
    assert_eq!((effect.to_x, effect.to_y), (15, 10));
    assert_eq!(effect.strength, 7);
    assert_eq!(effect.base_sprite, 2);
    assert_eq!(world.items[&ItemId(7)].sprite & 1, 1);
    assert_eq!(
        read_driver_data_u16(&world.items[&ItemId(11)].driver_data, 1),
        Some(293)
    );
}

#[test]
fn kill_bless_item_destroys_carried_bless_and_effect() {
    let mut world = World::default();
    world.tick = Tick(100);
    let mut character = character(1);
    character.flags.insert(CharacterFlags::PLAYER);
    character.mana = 10 * POWERSCALE;
    character.values[0][CharacterValue::Bless as usize] = 40;
    character.values[1][CharacterValue::Intelligence as usize] = 20;
    character.values[1][CharacterValue::Wisdom as usize] = 20;
    character.values[1][CharacterValue::Agility as usize] = 20;
    character.values[1][CharacterValue::Strength as usize] = 20;
    world.add_character(character);

    assert!(world.install_bless_spell(CharacterId(1), 40, 2_880));
    let spell_id = world.characters.get(&CharacterId(1)).unwrap().inventory[29].unwrap();
    assert!(world.items.contains_key(&spell_id));
    assert_eq!(world.effects.len(), 1);
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().values[0]
            [CharacterValue::Intelligence as usize],
        30
    );

    assert!(world.kill_bless_item(CharacterId(1)));

    let character = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(character.inventory[29], None);
    assert!(!world.items.contains_key(&spell_id));
    assert!(world.effects.is_empty());
    // Back to the raised baseline (20) once the bless item is destroyed
    // and `update_char` recomputes modifiers.
    assert_eq!(
        character.values[0][CharacterValue::Intelligence as usize],
        20
    );
}

#[test]
fn kill_bless_item_returns_false_and_is_a_no_op_without_a_bless_item() {
    let mut world = World::default();
    let character = character(1);
    world.add_character(character);

    assert!(!world.kill_bless_item(CharacterId(1)));
    assert!(world
        .characters
        .get(&CharacterId(1))
        .unwrap()
        .inventory
        .iter()
        .all(Option::is_none));
}

#[test]
fn kill_bless_item_ignores_non_bless_items_in_equip_slots() {
    let mut world = World::default();
    let mut character = character(1);
    character.inventory[15] = Some(ItemId(5));
    world.add_character(character);
    let mut sword = item(5, ItemFlags::WNRHAND);
    sword.carried_by = Some(CharacterId(1));
    world.add_item(sword);

    assert!(!world.kill_bless_item(CharacterId(1)));
    assert!(world.items.contains_key(&ItemId(5)));
    assert_eq!(
        world.characters.get(&CharacterId(1)).unwrap().inventory[15],
        Some(ItemId(5))
    );
}
