use super::*;

pub(crate) fn palace_bomb_driver(character: &Character, item: &mut Item) -> ItemDriverOutcome {
    match (
        character.id.0 != 0,
        drdata(item, 0),
        item.carried_by.is_some(),
    ) {
        (true, 2, _) => ItemDriverOutcome::PalaceBombExplode {
            item_id: item.id,
            character_id: character.id,
            owner_id: drdata_u32(item, 1),
            x: item.x,
            y: item.y,
        },
        (true, 1, true) => {
            set_drdata(item, 0, 0);
            item.sprite -= 1;
            ItemDriverOutcome::PalaceBombToggled {
                item_id: item.id,
                character_id: character.id,
                active: false,
            }
        }
        (true, 0, true) => {
            set_drdata(item, 0, 1);
            write_drdata_u32(item, 1, character.id.0);
            item.sprite += 1;
            ItemDriverOutcome::PalaceBombToggled {
                item_id: item.id,
                character_id: character.id,
                active: true,
            }
        }
        (true, _, _) => ItemDriverOutcome::Noop,
        (false, 1, false) => {
            set_drdata(item, 0, 2);
            item.sprite += 1;
            item.flags.insert(ItemFlags::STEPACTION);
            item.flags.remove(ItemFlags::TAKE | ItemFlags::USE);
            ItemDriverOutcome::PalaceBombTimer {
                item_id: item.id,
                character_id: character.id,
                armed: true,
                schedule_after_ticks: TICKS_PER_SECOND * 5,
            }
        }
        (false, _, _) => ItemDriverOutcome::PalaceBombTimer {
            item_id: item.id,
            character_id: character.id,
            armed: false,
            schedule_after_ticks: TICKS_PER_SECOND * 5,
        },
    }
}

pub(crate) fn palace_cap_driver(
    character: &Character,
    item: &Item,
    _context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 != 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::PalaceCapTimer {
        item_id: item.id,
        character_id: item.carried_by.unwrap_or(character.id),
        active: drdata(item, 0) != 0,
        schedule_after_ticks: TICKS_PER_SECOND / 4,
    }
}

pub(crate) const PALACE_KEY_COMBINATIONS: &[(i32, i32, i32)] = &[
    (51015, 51016, 51021),
    (51015, 51017, 51027),
    (51015, 51022, 51023),
    (51015, 51024, 51026),
    (51015, 51025, 51027),
    (51015, 51029, 51032),
    (51015, 51030, 51033),
    (51015, 51034, 51031),
    (51015, 51036, 51038),
    (51015, 51039, 51014),
    (51015, 51040, 51037),
    (51016, 51018, 51022),
    (51016, 51025, 51024),
    (51016, 51027, 51041),
    (51016, 51028, 51026),
    (51016, 51030, 51034),
    (51016, 51032, 51042),
    (51016, 51033, 51031),
    (51016, 51037, 51014),
    (51016, 51038, 51043),
    (51016, 51040, 51039),
    (51017, 51018, 51025),
    (51017, 51019, 51029),
    (51017, 51021, 51041),
    (51017, 51022, 51024),
    (51017, 51023, 51026),
    (51017, 51035, 51036),
    (51017, 51022, 51024),
    (51018, 51021, 51023),
    (51018, 51027, 51028),
    (51018, 51029, 51030),
    (51018, 51032, 51033),
    (51018, 51036, 51040),
    (51018, 51038, 51037),
    (51018, 51041, 51026),
    (51018, 51042, 51031),
    (51018, 51043, 51014),
    (51019, 51020, 51035),
    (51019, 51024, 51034),
    (51019, 51025, 51030),
    (51019, 51026, 51031),
    (51019, 51027, 51032),
    (51019, 51028, 51033),
    (51019, 51041, 51042),
    (51020, 51029, 51036),
    (51020, 51030, 51040),
    (51020, 51031, 51014),
    (51020, 51032, 51038),
    (51020, 51033, 51037),
    (51020, 51034, 51039),
    (51021, 51025, 51026),
    (51021, 51030, 51031),
    (51021, 51036, 51043),
    (51021, 51040, 51014),
    (51022, 51027, 51026),
    (51022, 51029, 51034),
    (51022, 51032, 51031),
    (51022, 51036, 51039),
    (51022, 51038, 51014),
    (51023, 51029, 51031),
    (51023, 51036, 51014),
    (51024, 51035, 51039),
    (51025, 51035, 51040),
    (51026, 51035, 51014),
    (51027, 51035, 51038),
    (51028, 51035, 51037),
    (51035, 51041, 51037),
];

pub(crate) fn palace_key_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        if let Some(&(part1, part2, _)) = PALACE_KEY_COMBINATIONS
            .iter()
            .find(|(_, _, result)| *result == item.sprite)
        {
            return ItemDriverOutcome::PalaceKeySplit {
                item_id: item.id,
                character_id: character.id,
                cursor_part_sprite: part1,
                carried_part_sprite: part2,
            };
        }
        return ItemDriverOutcome::PalaceKeyNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    };

    if context.cursor_template_id != Some(IID_AREA11_PALACEKEYPART) {
        return ItemDriverOutcome::PalaceKeyDoesNotFit {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let cursor_sprite = context.cursor_sprite.unwrap_or_default();
    let Some(&(_, _, result_sprite)) =
        PALACE_KEY_COMBINATIONS.iter().find(|&&(part1, part2, _)| {
            (item.sprite == part1 && cursor_sprite == part2)
                || (cursor_sprite == part1 && item.sprite == part2)
        })
    else {
        return ItemDriverOutcome::PalaceKeyDoesNotFit {
            item_id: item.id,
            character_id: character.id,
        };
    };

    ItemDriverOutcome::PalaceKeyCombine {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        result_sprite,
        final_key: result_sprite == 51014,
    }
}

pub(crate) fn palace_gate_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if !context.timer_call || character.id.0 != 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::PalaceGateTick {
        item_id: item.id,
        opened: false,
        closed: false,
        blocked: false,
    }
}

pub(crate) fn palace_door_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    item.driver_data.resize(2, 0);

    if context.timer_call || character.id.0 == 0 {
        match item.driver_data[1] {
            1 => {
                item.driver_data[1] = 2;
                ItemDriverOutcome::PalaceDoorTick {
                    item_id: item.id,
                    character_id: CharacterId(0),
                    state: 2,
                    frame: item.driver_data[0],
                    sprite: item.sprite,
                    set_tmoveblock: Some(true),
                    schedule_after_ticks: Some(3),
                }
            }
            2 => {
                item.driver_data[0] = item.driver_data[0].saturating_sub(1);
                item.sprite = 15196 + i32::from(item.driver_data[0]);
                let schedule_after_ticks = if item.driver_data[0] != 0 {
                    Some(3)
                } else {
                    item.driver_data[1] = 0;
                    None
                };
                ItemDriverOutcome::PalaceDoorTick {
                    item_id: item.id,
                    character_id: CharacterId(0),
                    state: item.driver_data[1],
                    frame: item.driver_data[0],
                    sprite: item.sprite,
                    set_tmoveblock: None,
                    schedule_after_ticks,
                }
            }
            3 => {
                item.driver_data[0] = item.driver_data[0].saturating_add(1);
                item.sprite = 15196 + i32::from(item.driver_data[0]);
                let (state, set_tmoveblock, schedule_after_ticks) = if item.driver_data[0] < 15 {
                    (3, None, Some(3))
                } else {
                    item.driver_data[1] = 1;
                    (1, Some(false), Some(TICKS_PER_SECOND * 10))
                };
                ItemDriverOutcome::PalaceDoorTick {
                    item_id: item.id,
                    character_id: CharacterId(0),
                    state,
                    frame: item.driver_data[0],
                    sprite: item.sprite,
                    set_tmoveblock,
                    schedule_after_ticks,
                }
            }
            _ => ItemDriverOutcome::Noop,
        }
    } else {
        if item.driver_data[1] != 0 {
            return ItemDriverOutcome::Noop;
        }
        if !context.has_area11_palace_key {
            return ItemDriverOutcome::PalaceDoorKeyRequired {
                item_id: item.id,
                character_id: character.id,
            };
        }

        item.driver_data[1] = 3;
        ItemDriverOutcome::PalaceDoorTick {
            item_id: item.id,
            character_id: character.id,
            state: 3,
            frame: item.driver_data[0],
            sprite: item.sprite,
            set_tmoveblock: None,
            schedule_after_ticks: Some(2),
        }
    }
}

pub(crate) fn islena_door_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 || context.timer_call {
        return ItemDriverOutcome::Noop;
    }

    if character.x == 144 && character.y == 56 {
        return ItemDriverOutcome::TeleportDoor {
            item_id: item.id,
            character_id: character.id,
            x: 144,
            y: 58,
        };
    }

    if context.islena_room_has_player {
        return ItemDriverOutcome::IslenaDoorBusy {
            item_id: item.id,
            character_id: character.id,
        };
    }

    if !context.islena_present {
        return ItemDriverOutcome::IslenaDoorRespawning {
            item_id: item.id,
            character_id: character.id,
        };
    }

    if context.islena_resting {
        return ItemDriverOutcome::IslenaDoorResting {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::TeleportDoor {
        item_id: item.id,
        character_id: character.id,
        x: 143,
        y: 55,
    }
}
