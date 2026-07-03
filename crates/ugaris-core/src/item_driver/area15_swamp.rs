use super::*;

pub(crate) fn swamparm_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 != 0 || !context.timer_call {
        return ItemDriverOutcome::Noop;
    }

    if item.driver_data.first().copied().unwrap_or_default() == 0
        && context.swamp_arm_triggered != Some(true)
    {
        return ItemDriverOutcome::SwampArmPulse {
            item_id: item.id,
            character_id: CharacterId(0),
            damage_now: false,
            schedule_after_ticks: 1,
        };
    }

    item.driver_data.resize(1, 0);
    item.driver_data[0] = item.driver_data[0].saturating_add(1);
    item.sprite += 1;
    let damage_now = item.driver_data[0] == 12;
    if item.driver_data[0] > 15 {
        item.driver_data[0] = 0;
        item.sprite -= 16;
    }

    ItemDriverOutcome::SwampArmPulse {
        item_id: item.id,
        character_id: CharacterId(0),
        damage_now,
        schedule_after_ticks: 1,
    }
}

pub(crate) const SWAMPWHISP_CIRCLE_LEFT: u8 = 10;

pub(crate) const SWAMPWHISP_CIRCLE_RIGHT: u8 = 11;

pub(crate) const SWAMPWHISP_DARK: u8 = 12;

pub(crate) fn swampwhisp_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 != 0 || !context.timer_call {
        return ItemDriverOutcome::Noop;
    }

    item.driver_data.resize(4, 0);
    if item.driver_data[1] == 0 {
        item.driver_data[1] = item.x as u8;
        item.driver_data[2] = item.y as u8;
        item.driver_data[3] = crate::direction::Direction::Down as u8;
    }

    let origin_x = i32::from(item.driver_data[1]);
    let origin_y = i32::from(item.driver_data[2]);
    let dx = i32::from(item.x) - origin_x;
    let dy = i32::from(item.y) - origin_y;
    if context.daylight > 50 {
        item.driver_data[3] = SWAMPWHISP_DARK;
    }

    let from = (item.x, item.y);
    let mut moved_to = None;
    let mut light_changed = false;
    let mut schedule_after_ticks = 2;

    match item.driver_data[3] {
        direction if direction == crate::direction::Direction::Down as u8 => {
            item.driver_data[0] = item.driver_data[0].wrapping_add(1);
            if item.driver_data[0] > 15 {
                item.driver_data[0] = 0;
            }
            if item.driver_data[0] == 12 {
                if context.swamp_whisp_move_succeeds == Some(true) {
                    item.y = item.y.saturating_add(1);
                    item.driver_data[0] = 1;
                    item.driver_data[3] = SWAMPWHISP_CIRCLE_LEFT;
                    moved_to = Some((item.x, item.y));
                } else {
                    item.driver_data[3] = SWAMPWHISP_CIRCLE_RIGHT;
                }
            }
        }
        direction if direction == crate::direction::Direction::Up as u8 => {
            item.driver_data[0] = item.driver_data[0].wrapping_sub(1);
            if item.driver_data[0] > 15 {
                item.driver_data[0] = 15;
            }
            if item.driver_data[0] == 2 {
                if context.swamp_whisp_move_succeeds == Some(true) {
                    item.y = item.y.saturating_sub(1);
                    item.driver_data[0] = 14;
                    item.driver_data[3] = SWAMPWHISP_CIRCLE_RIGHT;
                    moved_to = Some((item.x, item.y));
                } else {
                    item.driver_data[3] = SWAMPWHISP_CIRCLE_LEFT;
                }
            }
        }
        direction if direction == crate::direction::Direction::Left as u8 => {
            item.driver_data[0] = item.driver_data[0].wrapping_add(1);
            if item.driver_data[0] > 15 {
                item.driver_data[0] = 0;
            }
            if item.driver_data[0] == 0 {
                if context.swamp_whisp_move_succeeds == Some(true) {
                    item.x = item.x.saturating_add(1);
                    item.driver_data[0] = 7;
                    item.driver_data[3] = SWAMPWHISP_CIRCLE_LEFT;
                    moved_to = Some((item.x, item.y));
                } else {
                    item.driver_data[3] = SWAMPWHISP_CIRCLE_RIGHT;
                }
            }
        }
        direction if direction == crate::direction::Direction::Right as u8 => {
            item.driver_data[0] = item.driver_data[0].wrapping_sub(1);
            if item.driver_data[0] > 15 {
                item.driver_data[0] = 15;
            }
            if item.driver_data[0] == 6 {
                if context.swamp_whisp_move_succeeds == Some(true) {
                    item.x = item.x.saturating_sub(1);
                    item.driver_data[0] = 2;
                    item.driver_data[3] = SWAMPWHISP_CIRCLE_RIGHT;
                    moved_to = Some((item.x, item.y));
                } else {
                    item.driver_data[3] = SWAMPWHISP_CIRCLE_LEFT;
                }
            }
        }
        SWAMPWHISP_CIRCLE_LEFT => {
            item.driver_data[0] = item.driver_data[0].wrapping_sub(1);
            if item.driver_data[0] > 15 {
                item.driver_data[0] = 15;
            }
            if dx < 2 && context.swamp_whisp_turn_x {
                item.driver_data[3] = crate::direction::Direction::Right as u8;
            }
            if dy < 2 && context.swamp_whisp_turn_y {
                item.driver_data[3] = crate::direction::Direction::Down as u8;
            }
        }
        SWAMPWHISP_CIRCLE_RIGHT => {
            item.driver_data[0] = item.driver_data[0].wrapping_add(1);
            if item.driver_data[0] > 15 {
                item.driver_data[0] = 0;
            }
            if dx > -2 && context.swamp_whisp_turn_x {
                item.driver_data[3] = crate::direction::Direction::Left as u8;
            }
            if dy > -2 && context.swamp_whisp_turn_y {
                item.driver_data[3] = crate::direction::Direction::Up as u8;
            }
        }
        SWAMPWHISP_DARK => {
            if context.daylight < 50 {
                item.driver_data[3] = SWAMPWHISP_CIRCLE_LEFT;
                item.modifier_value[0] = 100;
                light_changed = true;
                schedule_after_ticks = 1;
            } else if item.sprite != 0 {
                item.sprite = 0;
                item.modifier_value[0] = 0;
                light_changed = true;
                schedule_after_ticks = TICKS_PER_SECOND;
            } else {
                schedule_after_ticks = TICKS_PER_SECOND;
            }
            return ItemDriverOutcome::SwampWhispPulse {
                item_id: item.id,
                character_id: CharacterId(0),
                moved_from: None,
                moved_to: None,
                light_changed,
                schedule_after_ticks,
            };
        }
        _ => {}
    }

    item.sprite = 20934 + i32::from(item.driver_data[0]);
    ItemDriverOutcome::SwampWhispPulse {
        item_id: item.id,
        character_id: CharacterId(0),
        moved_from: moved_to.map(|_| from),
        moved_to,
        light_changed,
        schedule_after_ticks,
    }
}

pub(crate) fn swampspawn_template(kind: u8) -> Option<&'static str> {
    match kind {
        0 => Some("swamp25n"),
        1 => Some("swamp27n"),
        2 => Some("swamp29n"),
        3 => Some("swamp31n"),
        _ => None,
    }
}

pub(crate) fn swampspawn_stop_frame(ground_sprite: u32) -> u8 {
    match ground_sprite & 0xffff {
        59405..=59413 => 6,
        59414..=59422 => 5,
        59423..=59431 => 3,
        _ => 7,
    }
}

pub(crate) fn swampspawn_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 != 0 || !context.timer_call {
        return ItemDriverOutcome::Noop;
    }

    item.driver_data.resize(20, 0);
    if item.driver_data[1] == 0 {
        item.driver_data[1] = 1;
        let base_sprite = item.sprite.saturating_sub(8) as u32;
        set_drdata_u32(item, 16, base_sprite);
        item.sprite = 0;
    }

    if item.driver_data[2] == 0 && context.swamp_spawn_live == Some(true) {
        return ItemDriverOutcome::SwampSpawnPulse {
            item_id: item.id,
            character_id: CharacterId(0),
            schedule_after_ticks: TICKS_PER_SECOND,
        };
    }

    let last_tick = drdata_u32(item, 12);
    let current_tick = context.current_tick;
    if item.driver_data[2] == 0
        && last_tick != 0
        && current_tick.wrapping_sub(last_tick) < (TICKS_PER_SECOND * 60 * 2) as u32
    {
        return ItemDriverOutcome::SwampSpawnPulse {
            item_id: item.id,
            character_id: CharacterId(0),
            schedule_after_ticks: TICKS_PER_SECOND,
        };
    }

    if item.driver_data[2] != 0 || context.swamp_spawn_player_close == Some(true) {
        item.driver_data[2] = item.driver_data[2].saturating_add(1);
        let base_sprite = drdata_u32(item, 16) as i32;
        item.sprite = base_sprite + i32::from(item.driver_data[2]);

        let ground_sprite = context.swamp_spawn_ground_sprite.unwrap_or_default();
        if item.driver_data[2] > swampspawn_stop_frame(ground_sprite) {
            item.driver_data[2] = 0;
            item.sprite = 0;

            if let Some(template) = swampspawn_template(item.driver_data[0]) {
                return ItemDriverOutcome::SwampSpawn {
                    item_id: item.id,
                    character_id: CharacterId(0),
                    template,
                    x: item.x,
                    y: item.y,
                    schedule_after_ticks: 3,
                };
            }
        }

        return ItemDriverOutcome::SwampSpawnPulse {
            item_id: item.id,
            character_id: CharacterId(0),
            schedule_after_ticks: 3,
        };
    }

    ItemDriverOutcome::SwampSpawnPulse {
        item_id: item.id,
        character_id: CharacterId(0),
        schedule_after_ticks: TICKS_PER_SECOND,
    }
}
