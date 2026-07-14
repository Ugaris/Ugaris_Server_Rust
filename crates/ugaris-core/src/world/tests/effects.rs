// Test setups intentionally mirror the C sources' memset-then-assign
// initialization pattern.
#![allow(clippy::field_reassign_with_default)]
use super::*;

#[test]
fn world_updates_map_light_when_effect_enters_and_leaves_tile() {
    let mut world = World::default();
    let mut effect = Effect::new(EF_BALL, 42, 0, 10);
    effect.light = 30;
    world.effects.insert(42, effect);

    assert!(world.set_effect_on_map(42, 10, 10));
    assert_eq!(world.map.tile(10, 10).unwrap().light, 30);

    world.remove_effect_from_map(42);
    assert_eq!(world.map.tile(10, 10).unwrap().light, 0);
}

#[test]
fn world_marks_dirty_sectors_for_effect_light_changes() {
    let mut world = World::default();
    let mut effect = Effect::new(EF_BALL, 42, 0, 10);
    effect.light = 30;
    world.effects.insert(42, effect);

    assert!(world.set_effect_on_map(42, 10, 10));
    assert_eq!(world.skip_x_sector(10, 10, 1), 0);
    assert_eq!(world.skip_x_sector(11, 10, 1), 0);

    world.remove_effect_from_map(42);
    assert_eq!(world.skip_x_sector(10, 10, 1), 0);
}

#[test]
fn world_create_mist_effect_uses_legacy_duration_without_light() {
    let mut world = World::default();
    world.tick.0 = 5;

    let effect_id = world.create_mist_effect(10, 10);

    let effect = world.effects.get(&effect_id).unwrap();
    assert_eq!(effect.effect_type, EF_MIST);
    assert_eq!(effect.start_tick, 5);
    assert_eq!(effect.stop_tick, 29);
    assert_eq!(effect.light, 0);
    assert_eq!(world.map.tile(10, 10).unwrap().effects[0], effect_id as u16);
    assert_eq!(world.map.tile(10, 10).unwrap().light, 0);
}

#[test]
fn world_create_bubble_effect_stores_legacy_y_offset_as_strength() {
    let mut world = World::default();
    world.tick.0 = 100;

    let effect_id = world.create_bubble_effect(10, 10, -14, 12);

    let effect = world.effects.get(&effect_id).unwrap();
    assert_eq!(effect.effect_type, EF_BUBBLE);
    assert_eq!(effect.strength, -14);
    assert_eq!(effect.start_tick, 100);
    assert_eq!(effect.stop_tick, 112);
    assert_eq!(world.map.tile(10, 10).unwrap().effects[0], effect_id as u16);
}

#[test]
fn palace_cap_timer_activates_idle_head_cap_and_refreshes_effect() {
    let mut world = World::default();
    world.tick = Tick(TICKS_PER_SECOND * 5);
    world.add_character(character(0));
    let mut wearer = character(1);
    wearer.inventory[worn_slot::HEAD] = Some(ItemId(7));
    wearer.regen_ticker = 0;
    world.spawn_character(wearer, 10, 10);
    let mut cap = item(7, ItemFlags::USED);
    cap.driver = IDR_PALACECAP;
    cap.carried_by = Some(CharacterId(1));
    cap.driver_data = vec![0];
    cap.sprite = 12_345;
    world.items.insert(ItemId(7), cap);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_PALACECAP,
            item_id: ItemId(7),
            character_id: CharacterId(0),
            spec: 0,
        },
        11,
    );

    assert!(matches!(outcome, ItemDriverOutcome::PalaceCapTimer { .. }));
    let cap = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(cap.driver_data[0], 1);
    assert_eq!(cap.sprite, 12_346);
    assert!(world.characters[&CharacterId(1)]
        .flags
        .contains(CharacterFlags::ITEMS));
    let cap_effect = world
        .effects
        .values()
        .find(|effect| effect.effect_type == EF_CAP)
        .unwrap();
    assert_eq!(cap_effect.target_character, Some(CharacterId(1)));
    assert_eq!(
        cap_effect.stop_tick,
        (TICKS_PER_SECOND * 5 + TICKS_PER_SECOND / 4 + 1) as i32
    );
    assert_eq!(cap_effect.strength, 1);
}

#[test]
fn scheduling_existing_bless_spell_restores_show_effect() {
    let mut world = World::default();
    let mut character = character(1);
    character.inventory[12] = Some(ItemId(7));
    world.add_character(character);

    let mut spell = item(7, ItemFlags::USED);
    spell.driver = IDR_BLESS;
    spell.carried_by = Some(CharacterId(1));
    spell.modifier_value[0] = 15;
    spell.driver_data = Vec::new();
    spell.driver_data.extend_from_slice(&500_u32.to_le_bytes());
    spell.driver_data.extend_from_slice(&100_u32.to_le_bytes());
    world.add_item(spell);

    assert_eq!(world.schedule_existing_spell_timers(), 1);

    let effect = world.effects.values().next().unwrap();
    assert_eq!(effect.effect_type, EF_BLESS);
    assert_eq!(effect.target_character, Some(CharacterId(1)));
    assert_eq!(effect.start_tick, 100);
    assert_eq!(effect.stop_tick, 500);
    assert_eq!(effect.strength, 15);
}

#[test]
fn player_pulse_damages_low_health_target_and_creates_visible_effects() {
    let mut world = World::default();
    world.tick = Tick(500);
    let mut caster = character(1);
    caster.flags.insert(CharacterFlags::PLAYER);
    caster.mana = 100 * POWERSCALE;
    caster.values[0][CharacterValue::Mana as usize] = 100;
    caster.values[0][CharacterValue::Pulse as usize] = 200;
    let mut target = character(2);
    target.flags.insert(CharacterFlags::ALIVE);
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

    assert!(world.apply_player_action_setup(&mut player, 1));
    let mana_after_setup = world.characters.get(&CharacterId(1)).unwrap().mana;
    assert!(mana_after_setup < 100 * POWERSCALE);

    world.characters.get_mut(&CharacterId(1)).unwrap().duration = 1;
    assert!(world.tick_basic_actions()[0].ok);

    let caster = world.characters.get(&CharacterId(1)).unwrap();
    assert!(caster.mana > mana_after_setup);
    let target = world.characters.get(&CharacterId(2)).unwrap();
    assert_eq!(target.lifeshield, 0);
    assert!(target.hp <= 0);
    assert!(target.flags.contains(CharacterFlags::UPDATE));
    assert_eq!(target.driver_messages[0].message_type, NT_GOTHIT);
    // C `act_pulse` (`act.c:1637-1640`): unconditional area `NT_CHAR`/
    // `NT_SPELL` broadcast from the caster after the per-target loop and
    // `create_pulse`; both caster and target sit inside the 32-tile notify
    // box, so both see these two messages (in addition to whatever
    // `apply_legacy_hurt`'s own unconditional `NT_SEEHIT` broadcast added).
    let nt_spell: Vec<_> = target
        .driver_messages
        .iter()
        .filter(|message| message.message_type == NT_SPELL)
        .collect();
    assert_eq!(nt_spell.len(), 1);
    assert_eq!(nt_spell[0].dat1, 1);
    assert_eq!(nt_spell[0].dat2, CharacterValue::Pulse as i32);
    assert_eq!(
        target
            .driver_messages
            .iter()
            .filter(|message| message.message_type == NT_CHAR)
            .count(),
        1
    );
    let caster_after = &world.characters[&CharacterId(1)];
    assert_eq!(caster_after.driver_messages[0].message_type, NT_DIDHIT);
    assert!(caster_after
        .driver_messages
        .iter()
        .any(|message| message.message_type == NT_CHAR));
    assert!(caster_after
        .driver_messages
        .iter()
        .any(|message| message.message_type == NT_SPELL));
    assert!(world
        .effects
        .values()
        .any(|effect| effect.effect_type == EF_PULSE && effect.x == 10 && effect.y == 10));
    assert!(world.effects.values().any(|effect| {
        effect.effect_type == EF_PULSEBACK
            && effect.target_character == Some(CharacterId(2))
            && effect.x == 10
            && effect.y == 10
    }));
}

#[test]
fn tile_special_check_creates_legacy_bubble_cadence_for_oxygen_player() {
    let mut world = World::default();
    world.tick.0 = 40;
    let mut player = character(1);
    player
        .flags
        .insert(CharacterFlags::PLAYER | CharacterFlags::OXYGEN);
    player.hp = 1_000;
    assert!(world.spawn_character(player, 10, 10));
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::SLOWDEATH | MapFlags::UNDERWATER);

    let outcome = world.tile_special_check(CharacterId(1));

    let effect_id = outcome.bubble_effect_id.unwrap();
    assert_eq!(outcome.damage, 0);
    assert_eq!(outcome.sound_type, Some(45));
    assert_eq!(
        world.drain_pending_sound_specials()[0].special.special_type,
        45
    );
    let player = world.characters.get(&CharacterId(1)).unwrap();
    assert_eq!(player.hp, 1_000);
    let effect = world.effects.get(&effect_id).unwrap();
    assert_eq!(effect.effect_type, EF_BUBBLE);
    assert_eq!(effect.strength, 45);
    assert_eq!(effect.stop_tick - effect.start_tick, 1);
    assert!(world
        .map
        .tile(10, 10)
        .unwrap()
        .effects
        .contains(&(effect_id as u16)));
}
