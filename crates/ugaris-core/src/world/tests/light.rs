use super::*;

#[test]
fn world_groundlight_marks_dirty_sector_when_light_changes() {
    let mut world = World {
        tick: Tick(17),
        map: MapGrid::new(24, 24),
        ..World::default()
    };
    world.map.tile_mut(10, 10).unwrap().ground_sprite = 14361;

    assert!(world.compute_groundlight_at(10, 10));
    assert_eq!(world.map.tile(10, 10).unwrap().light, 64);
    assert_eq!(world.skip_x_sector(10, 10, 17), 0);
    assert!(!world.compute_groundlight_at(40, 40));
}

#[test]
fn world_shadow_marks_dirty_sector_only_on_daylight_change() {
    let mut world = World {
        tick: Tick(23),
        map: MapGrid::new(24, 24),
        ..World::default()
    };

    assert!(world.compute_shadow_at(10, 10));
    assert_eq!(world.map.tile(10, 10).unwrap().daylight, 63);
    assert_eq!(world.skip_x_sector(10, 10, 23), 0);
    assert!(!world.compute_shadow_at(10, 10));
}

#[test]
fn world_reschedules_light_timer_after_lighting_torch() {
    let mut world = World::default();
    let mut character = character(1);
    character.inventory[30] = Some(ItemId(7));
    let mut torch = item(7, ItemFlags::USED | ItemFlags::USE);
    torch.carried_by = Some(CharacterId(1));
    torch.driver = IDR_TORCH;
    torch.driver_data = vec![0, 0, 10, 20];
    world.add_character(character);
    world.add_item(torch);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_TORCH,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert!(matches!(outcome, ItemDriverOutcome::LightChanged { .. }));
    assert_eq!(world.timers.used_timers(), 1);
}

#[test]
fn world_updates_map_light_when_timer_driven_map_item_changes() {
    let mut world = World::default();
    world.date.daylight = 40;
    let mut nightlight = item(7, ItemFlags::USED);
    nightlight.driver = IDR_NIGHTLIGHT;
    nightlight.driver_data = vec![0, 12];
    nightlight.x = 10;
    nightlight.y = 10;
    world.map.tile_mut(10, 10).unwrap().item = 7;
    world.add_item(nightlight);
    assert_eq!(world.map.tile(10, 10).unwrap().light, 0);

    assert!(world.schedule_item_driver_timer(ItemId(7), CharacterId(0), 1));
    world.advance();
    let outcomes = world.process_due_timers(1);

    assert!(matches!(
        outcomes[0],
        ItemDriverOutcome::LightChanged { .. }
    ));
    assert_eq!(world.map.tile(10, 10).unwrap().light, 12);
}

#[test]
fn world_schedules_existing_onofflight_and_preserves_first_timer_state() {
    let mut world = World::default();
    let mut light = item(7, ItemFlags::USED | ItemFlags::USE);
    light.driver = IDR_ONOFFLIGHT;
    light.driver_data = vec![1, 14];
    light.modifier_index[0] = CharacterValue::Light as i16;
    light.modifier_value[0] = 14;
    light.sprite = 101;
    light.x = 10;
    light.y = 10;
    world.map.tile_mut(10, 10).unwrap().item = 7;
    world.add_item(light);
    assert_eq!(world.map.tile(10, 10).unwrap().light, 14);

    assert_eq!(world.schedule_existing_light_timers(), 1);
    world.advance();
    let outcomes = world.process_due_timers(3);

    assert_eq!(outcomes, vec![ItemDriverOutcome::Noop]);
    let light = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(light.driver_data[6], 1);
    assert_eq!(light.driver_data[0], 1);
    assert_eq!(light.modifier_value[0], 14);
    assert_eq!(light.sprite, 101);
    assert_eq!(world.map.tile(10, 10).unwrap().light, 14);
}

#[test]
fn world_marks_dirty_sectors_for_lit_item_changes() {
    let mut world = World::default();
    let mut light_item = item(7, ItemFlags::USED | ItemFlags::TAKE);
    light_item.x = 11;
    light_item.y = 10;
    light_item.modifier_index[0] = CharacterValue::Light as i16;
    light_item.modifier_value[0] = 16;
    world.map.tile_mut(11, 10).unwrap().item = 7;

    assert!(world.skip_x_sector(11, 10, 1) > 0);
    world.add_item(light_item);

    assert_eq!(world.skip_x_sector(11, 10, 1), 0);
    assert_eq!(world.skip_x_sector(12, 10, 1), 0);
    assert!(world.skip_x_sector(40, 40, 1) > 0);
}

#[test]
fn world_refreshes_character_light_after_value_change_without_stale_light() {
    let mut world = World::default();
    let mut character = character(1);
    character.values[0][CharacterValue::Light as usize] = 16;
    assert!(world.spawn_character(character, 10, 10));

    let old_light = world.characters[&CharacterId(1)].values[0][CharacterValue::Light as usize];
    world.characters.get_mut(&CharacterId(1)).unwrap().values[0][CharacterValue::Light as usize] =
        25;

    assert!(world.refresh_character_light_after_value_change(CharacterId(1), old_light));
    assert_eq!(world.map.tile(10, 10).unwrap().light, 25);
    assert!(world.characters[&CharacterId(1)]
        .flags
        .contains(CharacterFlags::UPDATE));

    let old_light = world.characters[&CharacterId(1)].values[0][CharacterValue::Light as usize];
    world.characters.get_mut(&CharacterId(1)).unwrap().values[0][CharacterValue::Light as usize] =
        0;

    assert!(world.refresh_character_light_after_value_change(CharacterId(1), old_light));
    assert_eq!(world.map.tile(10, 10).unwrap().light, 0);
}

#[test]
fn world_marks_dirty_sectors_for_character_light_movement() {
    let mut world = World::default();
    let mut character = character(1);
    character.values[0][CharacterValue::Light as usize] = 16;

    assert!(world.spawn_character(character, 10, 10));
    assert_eq!(world.skip_x_sector(10, 10, 1), 0);

    let character = world.characters.get_mut(&CharacterId(1)).unwrap();
    character.tox = 12;
    character.toy = 10;
    assert!(world.complete_walk(CharacterId(1)));

    assert_eq!(world.skip_x_sector(10, 10, 1), 0);
    assert_eq!(world.skip_x_sector(12, 10, 1), 0);
}

#[test]
fn world_schedules_existing_timer_driven_light_items() {
    let mut world = World::default();
    let mut nightlight = item(7, ItemFlags::USED);
    nightlight.driver = IDR_NIGHTLIGHT;
    nightlight.driver_data = vec![0, 9];
    let mut burning_torch = item(8, ItemFlags::USED | ItemFlags::NODECAY);
    burning_torch.driver = IDR_TORCH;
    burning_torch.driver_data = vec![1, 0, 10, 20];
    let mut unlit_torch = item(9, ItemFlags::USED);
    unlit_torch.driver = IDR_TORCH;
    unlit_torch.driver_data = vec![0, 0, 10, 20];
    let mut edemon_light = item(10, ItemFlags::USED);
    edemon_light.driver = IDR_EDEMONLIGHT;
    let mut edemon_tube = item(14, ItemFlags::USED);
    edemon_tube.driver = IDR_EDEMONTUBE;
    let mut edemon_loader = item(13, ItemFlags::USED);
    edemon_loader.driver = IDR_EDEMONLOADER;
    let mut fdemon_loader = item(11, ItemFlags::USED);
    fdemon_loader.driver = IDR_FDEMONLOADER;
    let mut fdemon_farm = item(12, ItemFlags::USED);
    fdemon_farm.driver = IDR_FDEMONFARM;
    world.add_item(nightlight);
    world.add_item(burning_torch);
    world.add_item(unlit_torch);
    world.add_item(edemon_light);
    world.add_item(edemon_tube);
    world.add_item(edemon_loader);
    world.add_item(fdemon_loader);
    world.add_item(fdemon_farm);

    assert_eq!(world.schedule_existing_light_timers(), 7);
    assert_eq!(world.timers.used_timers(), 7);
}

#[test]
fn world_applies_edemon_loader_and_powers_matching_section_light() {
    let mut world = World::default();
    let mut player = character(1);
    player.flags.insert(CharacterFlags::PLAYER);
    player.cursor_item = Some(ItemId(9));
    assert!(world.spawn_character(player, 11, 10));
    let mut loader = item(7, ItemFlags::USED | ItemFlags::USE);
    loader.driver = IDR_EDEMONLOADER;
    loader.driver_data = vec![2, 0, 0];
    assert!(world.map.set_item_map(&mut loader, 10, 10));
    world.map.tile_mut(10, 10).unwrap().ground_sprite = 123;
    world.add_item(loader);
    let mut crystal = item(9, ItemFlags::USED);
    crystal.template_id = 0x01000049;
    crystal.driver_data = vec![86];
    crystal.carried_by = Some(CharacterId(1));
    world.add_item(crystal);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_EDEMONLOADER,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        6,
    );

    assert!(matches!(
        outcome,
        ItemDriverOutcome::EdemonLoaderChanged { .. }
    ));
    assert!(!world.items.contains_key(&ItemId(9)));
    assert_eq!(world.characters[&CharacterId(1)].cursor_item, None);
    assert_eq!(world.items[&ItemId(7)].driver_data, vec![2, 86, 7]);
    assert_eq!(world.items[&ItemId(7)].sprite, 14260);
    assert_eq!(
        world.map.tile(10, 10).unwrap().ground_sprite,
        (14240 << 16) | 123
    );
    assert_eq!(
        world.drain_pending_sound_specials()[0].special.special_type,
        41
    );

    let mut light = item(10, ItemFlags::USED);
    light.driver = IDR_EDEMONLIGHT;
    light.driver_data = vec![2];
    assert!(world.map.set_item_map(&mut light, 12, 10));
    world.add_item(light);
    assert!(world.schedule_item_driver_timer(ItemId(10), CharacterId(0), 1));
    world.advance();
    let light_outcomes = world.process_due_timers(6);

    assert!(light_outcomes
        .iter()
        .any(|outcome| matches!(outcome, ItemDriverOutcome::LightChanged { .. })));
    assert_eq!(world.items[&ItemId(10)].sprite, 14191);
    assert_eq!(world.items[&ItemId(10)].modifier_value[0], 200);
}

#[test]
fn world_processes_zero_character_nightlight_timer_callback() {
    let mut world = World::default();
    world.date.daylight = 40;
    let mut nightlight = item(7, ItemFlags::USED);
    nightlight.driver = IDR_NIGHTLIGHT;
    nightlight.driver_data = vec![0, 9];
    world.add_item(nightlight);
    assert_eq!(world.schedule_existing_light_timers(), 1);

    world.advance();
    let outcomes = world.process_due_timers(1);

    assert_eq!(outcomes.len(), 1);
    assert!(matches!(
        outcomes[0],
        ItemDriverOutcome::LightChanged {
            character_id: CharacterId(0),
            ..
        }
    ));
    let nightlight = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(nightlight.driver_data[0], 1);
    assert_eq!(nightlight.modifier_value[0], 9);
    assert_eq!(nightlight.sprite, 1);
    assert_eq!(world.timers.used_timers(), 1);
}

#[test]
fn schedule_existing_light_timers_includes_caligar_flames() {
    let mut world = World::default();
    let mut flame = item(7, ItemFlags::USED | ItemFlags::USE);
    flame.driver = IDR_CALIGARFLAME;
    flame.driver_data = vec![1, 3, 0, 0];
    world.add_item(flame);

    assert_eq!(world.schedule_existing_light_timers(), 1);

    world.advance();
    let outcomes = world.process_due_timers(36);

    assert_eq!(outcomes.len(), 1);
    assert!(matches!(
        outcomes[0],
        ItemDriverOutcome::FlameThrowerPulse {
            item_id: ItemId(7),
            direction: 3,
            ..
        }
    ));
}

#[test]
fn world_blocks_lighting_torch_underwater() {
    let mut world = World::default();
    let mut character = character(1);
    character.x = 10;
    character.y = 10;
    character.inventory[30] = Some(ItemId(7));
    let mut torch = item(7, ItemFlags::USED | ItemFlags::USE);
    torch.carried_by = Some(CharacterId(1));
    torch.driver = IDR_TORCH;
    torch.driver_data = vec![0, 0, 10, 20];
    world.add_character(character);
    world.add_item(torch);
    world
        .map
        .tile_mut(10, 10)
        .unwrap()
        .flags
        .insert(MapFlags::UNDERWATER);

    let outcome = world.execute_item_driver_request(
        ItemDriverRequest::Driver {
            driver: IDR_TORCH,
            item_id: ItemId(7),
            character_id: CharacterId(1),
            spec: 0,
        },
        1,
    );

    assert_eq!(
        outcome,
        ItemDriverOutcome::BlockedByRequirements {
            item_id: ItemId(7),
            character_id: CharacterId(1),
        }
    );
    let torch = world.items.get(&ItemId(7)).unwrap();
    assert_eq!(torch.driver_data[0], 0);
    assert_eq!(torch.modifier_value[0], 0);
    assert!(!torch.flags.contains(ItemFlags::NODECAY));
}

#[test]
fn schedule_existing_light_timers_includes_caligar_weights() {
    let mut world = World::default();
    let mut weight = item(8, ItemFlags::USED | ItemFlags::USE | ItemFlags::MOVEBLOCK);
    weight.driver = IDR_CALIGAR;
    weight.driver_data = vec![2];
    weight.x = 10;
    weight.y = 10;
    world.add_item(weight);

    assert_eq!(world.schedule_existing_light_timers(), 1);
    world.tick = Tick(1);
    let outcomes = world.process_due_timers(36);
    assert_eq!(
        outcomes,
        vec![ItemDriverOutcome::CaligarWeightTimer { item_id: ItemId(8) }]
    );
}

#[test]
fn schedule_existing_light_timers_includes_lab5_fireface_and_lightface_but_not_other_flavors() {
    let mut world = World::default();
    let mut fireface = item(9, ItemFlags::USED | ItemFlags::USE);
    fireface.driver = IDR_LAB5_ITEM;
    fireface.driver_data = vec![2, 0];
    fireface.x = 10;
    fireface.y = 10;
    fireface.sprite = 11135;
    world.add_item(fireface);
    let mut lightface = item(10, ItemFlags::USED | ItemFlags::USE);
    lightface.driver = IDR_LAB5_ITEM;
    lightface.driver_data = vec![13, 0, 0];
    lightface.x = 20;
    lightface.y = 20;
    lightface.sprite = 11136;
    world.add_item(lightface);
    // A non-ambient lab5 flavor (obelisk) must not be primed.
    let mut obelisk = item(11, ItemFlags::USED | ItemFlags::USE);
    obelisk.driver = IDR_LAB5_ITEM;
    obelisk.driver_data = vec![1];
    world.add_item(obelisk);

    assert_eq!(world.schedule_existing_light_timers(), 2);

    world.advance();
    let outcomes = world.process_due_timers(22);

    assert_eq!(outcomes.len(), 2);
    assert!(outcomes.iter().any(|outcome| matches!(
        outcome,
        ItemDriverOutcome::FireballMachineProjectile {
            item_id: ItemId(9),
            character_id: CharacterId(0),
            ..
        }
    )));
    assert!(outcomes.iter().any(|outcome| matches!(
        outcome,
        ItemDriverOutcome::BallTrapProjectile {
            item_id: ItemId(10),
            character_id: CharacterId(0),
            ..
        }
    )));
}
