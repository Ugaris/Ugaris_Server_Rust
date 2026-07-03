use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdemonGateSpawnContext {
    pub slot: usize,
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdemonLoaderBlockReason {
    CrystalAlreadyPresent,
    CrystalStuck,
    NeedsCrystal,
    WrongCrystal,
}

pub(crate) fn edemonball_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if !context.timer_call || character.id.0 != 0 {
        return ItemDriverOutcome::Noop;
    }

    let item_x = i32::from(item.x);
    let item_y = i32::from(item.y);
    let strength = i32::from(drdata(item, 2));
    let base_sprite = i32::from(drdata(item, 1));

    match drdata(item, 0) {
        0 if !context.edemon_fire_enabled.unwrap_or(true) => {
            item.sprite = 14160;
            return ItemDriverOutcome::EdemonBallInactive {
                item_id: item.id,
                character_id: character.id,
                schedule_after_ticks: TICKS_PER_SECOND,
            };
        }
        0 => {
            item.sprite = 14159;
        }
        2..=9 if !matches!(context.edemon_section_power, Some(1..=248)) => {
            item.sprite = 14160;
            return ItemDriverOutcome::EdemonBallInactive {
                item_id: item.id,
                character_id: character.id,
                schedule_after_ticks: TICKS_PER_SECOND,
            };
        }
        2..=9 => {
            item.sprite = 14161;
        }
        _ => {}
    }

    let shot = match drdata(item, 3) {
        0 => Some((item_x, item_y + 1, item_x, item_y + 10)),
        1 => Some((item_x, item_y + 1, item_x + 1, item_y + 10)),
        2 => Some((item_x, item_y + 1, item_x - 1, item_y + 10)),
        3 => Some((item_x, item_y - 1, item_x, item_y - 10)),
        4 => Some((item_x, item_y - 1, item_x + 1, item_y - 10)),
        5 => Some((item_x, item_y - 1, item_x - 1, item_y - 10)),
        6 => Some((item_x + 1, item_y, item_x + 10, item_y)),
        7 => Some((item_x + 1, item_y, item_x + 10, item_y + 1)),
        8 => Some((item_x + 1, item_y, item_x + 10, item_y - 1)),
        9 => Some((item_x - 1, item_y, item_x - 10, item_y)),
        10 => Some((item_x - 1, item_y, item_x - 10, item_y + 1)),
        11 => Some((item_x - 1, item_y, item_x - 10, item_y - 1)),
        _ => None,
    };

    let Some((start_x, start_y, target_x, target_y)) = shot else {
        set_drdata(item, 3, 0);
        return ItemDriverOutcome::Noop;
    };
    set_drdata(item, 3, drdata(item, 3).saturating_add(1));

    ItemDriverOutcome::EdemonBallProjectile {
        item_id: item.id,
        character_id: character.id,
        start_x: clamp_legacy_coordinate(start_x),
        start_y: clamp_legacy_coordinate(start_y),
        target_x: clamp_legacy_coordinate(target_x),
        target_y: clamp_legacy_coordinate(target_y),
        strength,
        base_sprite,
        schedule_after_ticks: TICKS_PER_SECOND * 16,
    }
}

pub(crate) fn edemon_switch_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    item.driver_data.resize(5, 0);
    let fire = item.driver_data[0] != 0;
    let pause_until = u32::from_le_bytes([
        item.driver_data[1],
        item.driver_data[2],
        item.driver_data[3],
        item.driver_data[4],
    ]);

    if context.timer_call || character.id.0 == 0 {
        if fire || context.current_tick <= pause_until {
            return ItemDriverOutcome::Noop;
        }
        item.driver_data[0] = 1;
        item.sprite -= 1;
        item.modifier_index[0] = V_LIGHT;
        item.modifier_value[0] = 64;
        return ItemDriverOutcome::LightChanged {
            item_id: item.id,
            character_id: character.id,
            schedule_after_ticks: None,
        };
    }

    if !fire {
        return ItemDriverOutcome::EdemonSwitchStuck {
            item_id: item.id,
            character_id: character.id,
        };
    }

    item.driver_data[0] = 0;
    let pause_until = context
        .current_tick
        .wrapping_add(EDEMON_SWITCH_COOLDOWN_TICKS as u32);
    item.driver_data[1..5].copy_from_slice(&pause_until.to_le_bytes());
    item.sprite += 1;
    item.modifier_value[0] = 0;

    ItemDriverOutcome::LightChanged {
        item_id: item.id,
        character_id: character.id,
        schedule_after_ticks: Some(EDEMON_SWITCH_COOLDOWN_TICKS + 1),
    }
}

pub(crate) fn edemon_gate_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 != 0 || !context.timer_call {
        return ItemDriverOutcome::Noop;
    }

    let mode = item.driver_data.first().copied().unwrap_or_default();
    let schedule_after_ticks = match mode {
        0 => TICKS_PER_SECOND * 10,
        1 => TICKS_PER_SECOND * 20,
        _ => return ItemDriverOutcome::Noop,
    };

    let Some(spawn) = context.edemon_gate_spawn else {
        return ItemDriverOutcome::LightChanged {
            item_id: item.id,
            character_id: CharacterId(0),
            schedule_after_ticks: Some(schedule_after_ticks),
        };
    };

    ItemDriverOutcome::EdemonGateSpawn {
        item_id: item.id,
        character_id: CharacterId(0),
        template: if mode == 0 { "edemon2s" } else { "edemon6s" },
        slot: spawn.slot,
        x: spawn.x,
        y: spawn.y,
        schedule_after_ticks,
    }
}

pub(crate) fn edemon_light_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 != 0 || !context.timer_call {
        return ItemDriverOutcome::Noop;
    }

    let power = context.edemon_section_power.unwrap_or_default();
    let (light, sprite) = if power != 0 && power < 249 {
        (200, 14191)
    } else {
        (0, 14189)
    };

    item.modifier_index[0] = V_LIGHT;
    item.modifier_value[0] = light;
    item.sprite = sprite;

    if power > 250 {
        ItemDriverOutcome::EdemonTubePulse {
            item_id: item.id,
            character_id: character.id,
            x: drdata_u16(item, 2),
            y: drdata_u16(item, 4),
            schedule_after_ticks: TICKS_PER_SECOND,
        }
    } else {
        ItemDriverOutcome::LightChanged {
            item_id: item.id,
            character_id: character.id,
            schedule_after_ticks: Some(TICKS_PER_SECOND),
        }
    }
}

pub(crate) fn edemon_door_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if item.x == 0 {
        return ItemDriverOutcome::Noop;
    }

    item.driver_data.resize(40, 0);
    if context.timer_call || character.id.0 == 0 {
        if item.driver_data[39] != 0 {
            item.driver_data[39] -= 1;
        }
        if item.driver_data[0] == 0 || item.driver_data[39] != 0 || item.driver_data[5] != 0 {
            return ItemDriverOutcome::Noop;
        }
        return ItemDriverOutcome::EdemonDoorToggle {
            item_id: item.id,
            character_id: character.id,
            key_name: None,
            locking: false,
        };
    }

    let required_key_id = door_required_key_id(item);
    let mut key_name = None;
    if required_key_id != 0 {
        let Some(key) = context
            .door_key
            .as_ref()
            .filter(|key| key.source == DoorKeySource::Carried && key.key_id == required_key_id)
        else {
            return ItemDriverOutcome::EdemonDoorLocked {
                item_id: item.id,
                character_id: character.id,
            };
        };
        key_name = Some(outcome_item_name(&key.name));
    }

    if context.edemon_section_power.unwrap_or_default() == 0 {
        return ItemDriverOutcome::EdemonDoorLifeless {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::EdemonDoorToggle {
        item_id: item.id,
        character_id: character.id,
        key_name,
        locking: item.driver_data[0] != 0,
    }
}

pub(crate) fn edemon_block_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    item.driver_data.resize(8, 0);

    if context.timer_call || character.id.0 == 0 {
        if drdata_u16(item, 4) == 0 {
            set_drdata_u16(item, 4, item.x);
            set_drdata_u16(item, 6, item.y);
        }

        let last_touch = drdata_u32(item, 0);
        let origin_x = drdata_u16(item, 4);
        let origin_y = drdata_u16(item, 6);
        if context.current_tick.wrapping_sub(last_touch) > (TICKS_PER_SECOND * 60 * 15) as u32
            && (origin_x != item.x || origin_y != item.y)
        {
            return ItemDriverOutcome::EdemonBlockMove {
                item_id: item.id,
                character_id: CharacterId(0),
                target_x: origin_x,
                target_y: origin_y,
                schedule_after_ticks: Some(TICKS_PER_SECOND * 5),
            };
        }

        return ItemDriverOutcome::LightChanged {
            item_id: item.id,
            character_id: CharacterId(0),
            schedule_after_ticks: Some(TICKS_PER_SECOND * 5),
        };
    }

    let Ok(direction) = crate::direction::Direction::try_from(character.dir) else {
        return ItemDriverOutcome::EdemonBlockBlocked {
            item_id: item.id,
            character_id: character.id,
        };
    };
    let (dx, dy) = direction.delta();
    let target_x = i32::from(item.x) + i32::from(dx);
    let target_y = i32::from(item.y) + i32::from(dy);
    let (Ok(target_x), Ok(target_y)) = (u16::try_from(target_x), u16::try_from(target_y)) else {
        return ItemDriverOutcome::EdemonBlockBlocked {
            item_id: item.id,
            character_id: character.id,
        };
    };

    set_drdata_u32(item, 0, context.current_tick);
    ItemDriverOutcome::EdemonBlockMove {
        item_id: item.id,
        character_id: character.id,
        target_x,
        target_y,
        schedule_after_ticks: None,
    }
}

pub(crate) fn edemon_tube_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    item.driver_data.resize(6, 0);

    if character.id.0 != 0 && !context.timer_call {
        return ItemDriverOutcome::TeleportDoor {
            item_id: item.id,
            character_id: character.id,
            x: drdata_u16(item, 2),
            y: drdata_u16(item, 4),
        };
    }

    let power = context.edemon_section_power.unwrap_or_default();
    let (light, sprite) = if power != 0 && power < 249 {
        (200, 14138)
    } else {
        (0, 14137)
    };

    item.modifier_index[0] = V_LIGHT;
    item.modifier_value[0] = light;
    item.sprite = sprite;

    if drdata_u16(item, 2) == 0 {
        if let Some((x, y)) = context.edemon_tube_target {
            set_drdata_u16(item, 2, x);
            set_drdata_u16(item, 4, y);
        }
    }

    if power > 250 {
        ItemDriverOutcome::EdemonTubePulse {
            item_id: item.id,
            character_id: character.id,
            x: drdata_u16(item, 2),
            y: drdata_u16(item, 4),
            schedule_after_ticks: TICKS_PER_SECOND,
        }
    } else {
        ItemDriverOutcome::LightChanged {
            item_id: item.id,
            character_id: character.id,
            schedule_after_ticks: Some(TICKS_PER_SECOND),
        }
    }
}

pub(crate) fn edemon_loader_driver(
    character: &mut Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    item.driver_data.resize(3, 0);

    let mut power = item.driver_data[1];
    let mut animation = item.driver_data[2];
    let mut consumed_cursor_item_id = None;
    let mut sound_type = None;

    if context.timer_call || character.id.0 == 0 {
        power = power.saturating_sub((power != 0) as u8);
        animation = animation.saturating_sub((animation != 0) as u8);
    } else {
        if power != 0 {
            return ItemDriverOutcome::EdemonLoaderBlocked {
                item_id: item.id,
                character_id: character.id,
                reason: if character.cursor_item.is_some() {
                    EdemonLoaderBlockReason::CrystalAlreadyPresent
                } else {
                    EdemonLoaderBlockReason::CrystalStuck
                },
            };
        }
        let Some(cursor_item_id) = character.cursor_item else {
            return ItemDriverOutcome::EdemonLoaderBlocked {
                item_id: item.id,
                character_id: character.id,
                reason: EdemonLoaderBlockReason::NeedsCrystal,
            };
        };
        if context.cursor_template_id != Some(IID_AREA6_YELLOWCRYSTAL) {
            return ItemDriverOutcome::EdemonLoaderBlocked {
                item_id: item.id,
                character_id: character.id,
                reason: EdemonLoaderBlockReason::WrongCrystal,
            };
        }

        power = context.cursor_drdata0.unwrap_or_default();
        animation = 7;
        character.cursor_item = None;
        character.flags.insert(CharacterFlags::ITEMS);
        consumed_cursor_item_id = Some(cursor_item_id);
        sound_type = Some(41);
    }

    item.driver_data[1] = power;
    item.driver_data[2] = animation;

    let overlay = if animation != 0 {
        14247u32.saturating_sub(u32::from(animation))
    } else if power != 0 {
        14248
    } else {
        14240
    };

    let old_sprite = item.sprite;
    item.sprite = if power != 0 {
        14262 - (i32::from(power) / 43)
    } else {
        14234
    };
    if old_sprite != 14234 && item.sprite == 14234 {
        sound_type = Some(43);
    }

    ItemDriverOutcome::EdemonLoaderChanged {
        item_id: item.id,
        character_id: character.id,
        consumed_cursor_item_id,
        ground_overlay_sprite: overlay,
        sound_type,
        schedule_after_ticks: (context.timer_call || character.id.0 == 0)
            .then_some(TICKS_PER_SECOND),
    }
}
