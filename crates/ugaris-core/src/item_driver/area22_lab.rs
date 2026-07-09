use super::*;

pub(crate) fn labtorch_driver(character: &Character, item: &mut Item) -> ItemDriverOutcome {
    item.driver_data.resize(2, 0);

    if character.id.0 == 0 {
        item.driver_data[1] = item.modifier_value[0].clamp(0, u8::MAX as i16) as u8;
        return ItemDriverOutcome::Noop;
    }

    if item.driver_data[0] == 0 {
        if character.flags.contains(CharacterFlags::PLAYER) {
            return ItemDriverOutcome::Noop;
        }
        item.sprite += 1;
        item.driver_data[0] = 1;
        item.modifier_index[0] = V_LIGHT;
        item.modifier_value[0] = i16::from(item.driver_data[1]);
    } else {
        item.sprite -= 1;
        item.driver_data[0] = 0;
        item.modifier_value[0] = 0;
    }

    ItemDriverOutcome::LightChanged {
        item_id: item.id,
        character_id: character.id,
        schedule_after_ticks: None,
    }
}

/// C `deathfibrin` (`src/area/22/lab1.c:482-590`). The same driver id
/// (`IDR_DEATHFIBRIN = 198`) backs two very different objects
/// distinguished by sprite, exactly like C: `deathfibrin_shrine`
/// (`sprite == 10428`, a fixed map dispenser) and the carried/dropped
/// `deathfibrin` staff itself (`struct deathfibrin_data`, cast onto
/// `it[in].drdata`).
///
/// Deviations/gaps (documented, not silent):
/// - The zero-character passive ticker (`lab1.c:548-588`: light-based
///   `amount` decay while sitting lit on the ground or being carried,
///   auto-vanish after 10 minutes unattended, and the `dat->tickerused`
///   cooldown that pauses decay right after a strike) is not ported -
///   nothing schedules `call_item(IDR_DEATHFIBRIN, in, 0, ...)` for this
///   driver in this port. Without that ticker ever running, C's own
///   lazy `dat->init` (only set the first time the zero-character path
///   runs) would never fire either, so this port instead lazily
///   initializes `amount = 10000` on the *first player strike* instead
///   (byte 4 of `driver_data` doubles as the "already initialized"
///   flag) - the one piece of `dat->init` this port actually needs, so
///   a freshly created staff still starts at 100% charge instead of
///   incorrectly reading as already-spent.
/// - `dat->used`/`dat->tickerused`/`dat->tickervanish` (all only
///   meaningful to the unported ticker) are not represented at all.
pub(crate) fn deathfibrin_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    // C `lab1.c:487-510`: the shrine dispenser.
    if item.sprite == 10428 {
        if character.id.0 == 0 {
            return ItemDriverOutcome::Noop;
        }
        if character.cursor_item.is_some() {
            return ItemDriverOutcome::DeathfibrinShrineOccupied {
                character_id: character.id,
            };
        }
        return ItemDriverOutcome::DeathfibrinShrineGive {
            item_id: item.id,
            character_id: character.id,
        };
    }

    // C `lab1.c:548-588`: the passive ticker is not ported - see the
    // driver's own doc comment.
    if context.timer_call || character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    // C `lab1.c:516-519`.
    if item.carried_by.is_none() {
        return ItemDriverOutcome::DeathfibrinNeedsCarry {
            character_id: character.id,
        };
    }

    // C `lab1.c:521-526`.
    let Some(master_id) = context.deathfibrin_master else {
        return ItemDriverOutcome::DeathfibrinNoMaster {
            character_id: character.id,
            tile_light: context.deathfibrin_tile_light,
        };
    };

    // C `lab1.c:513-543`: lazy `dat->init` substitute (see doc comment)
    // plus the unconditional `dat->amount = max(0, dat->amount - 1000)`.
    item.driver_data.resize(item.driver_data.len().max(5), 0);
    let already_initialized = item.driver_data[4] != 0;
    let amount = if already_initialized {
        u32::from_le_bytes(item.driver_data[0..4].try_into().unwrap_or_default())
    } else {
        item.driver_data[4] = 1;
        10_000
    };
    let amount = amount.saturating_sub(1000);
    item.driver_data[0..4].copy_from_slice(&amount.to_le_bytes());
    let vanished = deathfibrin_check(item, amount);

    ItemDriverOutcome::DeathfibrinStrike {
        item_id: item.id,
        character_id: character.id,
        master_id,
        item_name: outcome_item_name(&item.name),
        vanished,
    }
}

/// C `deathfibrin_check` (`lab1.c:460-480`): updates the staff's sprite/
/// description for its new `amount`, returning whether it should vanish
/// (C's `remove_item`/`destroy_item`, applied by the caller since this
/// pure function has no `World` access).
fn deathfibrin_check(item: &mut Item, amount: u32) -> bool {
    if amount == 0 {
        return true;
    }
    item.sprite = (10428 - 10 * (amount as i32 + 500) / 10000).min(10427);
    item.description = format!("Staff containing {}% Deathfibrin", amount / 100);
    false
}

pub(crate) fn lab2_water_driver(character: &Character, item: &mut Item) -> ItemDriverOutcome {
    item.driver_data.resize(1, 0);

    if character.id.0 == 0 {
        if item.driver_data[0] == 0 {
            item.driver_data[0] = match item.sprite {
                11008..=11010 => 2,
                20793..=20796 => 1,
                11011 => 3,
                11012 => 4,
                11013 => 5,
                _ => 0,
            };
        }
        return ItemDriverOutcome::Noop;
    }

    match item.driver_data[0] {
        1 => {
            if character.cursor_item.is_some() {
                ItemDriverOutcome::Lab2WaterCursorOccupied {
                    item_id: item.id,
                    character_id: character.id,
                }
            } else {
                ItemDriverOutcome::Lab2WaterWell {
                    item_id: item.id,
                    character_id: character.id,
                }
            }
        }
        2 => ItemDriverOutcome::Lab2WaterAltar {
            item_id: item.id,
            character_id: character.id,
        },
        4 | 5 => ItemDriverOutcome::Lab2WaterDrink {
            item_id: item.id,
            character_id: character.id,
        },
        _ => ItemDriverOutcome::Noop,
    }
}

pub(crate) fn lab2_stepaction_driver(character: &Character, item: &mut Item) -> ItemDriverOutcome {
    let step_kind = item.driver_data.first().copied().unwrap_or_default();
    if !matches!(step_kind, 1 | 2) {
        return ItemDriverOutcome::Noop;
    }

    if character.id.0 == 0 {
        item.sprite = 0;
        return ItemDriverOutcome::Lab2StepActionClear { item_id: item.id };
    }

    if !character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }

    match step_kind {
        1 if character.dir == Direction::Up as u8 => {
            ItemDriverOutcome::Lab2StepActionDaemonWarning {
                item_id: item.id,
                character_id: character.id,
                x: item.x,
                y: item.y.saturating_sub(5),
            }
        }
        2 => ItemDriverOutcome::Lab2StepActionDaemonCheck {
            item_id: item.id,
            character_id: character.id,
        },
        _ => ItemDriverOutcome::Noop,
    }
}

pub(crate) fn lab2_grave_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    let grave_item = item.driver_data.first().copied().unwrap_or_default();
    let grave_open_character = item
        .driver_data
        .get(4..8)
        .and_then(|bytes| bytes.try_into().ok())
        .map(i32::from_le_bytes)
        .unwrap_or_default();
    let grave_open_serial = item
        .driver_data
        .get(8..12)
        .and_then(|bytes| bytes.try_into().ok())
        .map(i32::from_le_bytes)
        .unwrap_or_default();

    if (context.timer_call || character.id.0 == 0) && grave_open_character == 0 {
        return ItemDriverOutcome::Noop;
    }

    if (context.timer_call || character.id.0 == 0)
        && grave_open_character != 0
        && grave_open_serial == -1
    {
        return ItemDriverOutcome::Lab2GraveClose { item_id: item.id };
    }

    if (context.timer_call || character.id.0 == 0) && grave_open_character > 0 {
        return ItemDriverOutcome::Lab2GraveCheckOpen {
            item_id: item.id,
            undead_id: CharacterId(grave_open_character as u32),
            undead_serial: grave_open_serial as u32,
            schedule_after_ticks: TICKS_PER_SECOND * 5,
        };
    }

    if character.id.0 != 0 {
        if !character.flags.contains(CharacterFlags::PLAYER) {
            return ItemDriverOutcome::Noop;
        }

        if grave_open_character != 0 {
            return ItemDriverOutcome::Noop;
        }

        if matches!(grave_item, 1..=4) {
            return ItemDriverOutcome::Lab2GraveClueBook {
                item_id: item.id,
                character_id: character.id,
                book: grave_item,
            };
        }

        return ItemDriverOutcome::Lab2GraveOpen {
            item_id: item.id,
            character_id: character.id,
            fixed_item: grave_item,
        };
    }

    ItemDriverOutcome::Noop
}

pub(crate) fn lab2_regenerate_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 != 0 || !context.timer_call {
        return ItemDriverOutcome::Noop;
    }

    let speed = drdata(item, 0);
    let schedule_after_ticks = u64::from(speed).saturating_mul(TICKS_PER_SECOND) / 24;
    ItemDriverOutcome::Lab2RegenerateTick {
        item_id: item.id,
        target_id: CharacterId(drdata_u32(item, 4)),
        start_tick: drdata_u32(item, 8),
        regen_percent: drdata(item, 1),
        schedule_after_ticks,
    }
}

pub(crate) fn lab3_plant_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call && character.id.0 == 0 && drdata(item, 0) == 10 {
        return ItemDriverOutcome::Lab3WhiteBerryLightTick {
            item_id: item.id,
            destroyed: false,
        };
    }

    if character.id.0 == 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    match drdata(item, 0) {
        5 => {
            const OXYGEN_SECONDS: [u64; 5] = [3, 8, 10, 12, 15];
            let freshness = usize::from(drdata(item, 2).min(4));
            let count = u64::from(drdata(item, 1));
            ItemDriverOutcome::Lab3YellowBerry {
                item_id: item.id,
                character_id: character.id,
                duration_ticks: OXYGEN_SECONDS[freshness] * count * TICKS_PER_SECOND,
                installed: false,
            }
        }
        6 => {
            const LIGHT_POWER: [i16; 5] = [10, 30, 40, 45, 50];
            let freshness = usize::from(drdata(item, 2).min(4));
            let count = i16::from(drdata(item, 1));
            ItemDriverOutcome::Lab3WhiteBerry {
                item_id: item.id,
                character_id: character.id,
                light_power: LIGHT_POWER[freshness].saturating_mul(count),
                started_emit: false,
                installed: false,
            }
        }
        11 => ItemDriverOutcome::Lab3BrownBerry {
            item_id: item.id,
            character_id: character.id,
            duration_ticks: 10 * TICKS_PER_SECOND,
            installed: false,
        },
        _ => ItemDriverOutcome::Noop,
    }
}

pub(crate) fn legacy_lab_destination(lab_level: u8) -> Option<(u16, u16, u16, u16)> {
    match lab_level {
        10 => Some((10, 22, 27, 242)),
        15 => Some((12, 22, 69, 105)),
        20 => Some((15, 22, 227, 250)),
        25 => Some((20, 22, 144, 103)),
        30 => Some((25, 22, 163, 243)),
        _ => None,
    }
}

pub(crate) fn labentrance_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    for lab_level in 0..64_u8 {
        let bit = 1_u64 << lab_level;
        if context.lab_solved_bits & bit != 0 {
            continue;
        }
        let Some((required_level, area_id, x, y)) = legacy_lab_destination(lab_level) else {
            continue;
        };
        if character.level < u32::from(required_level) {
            return ItemDriverOutcome::LabEntranceTooLow {
                item_id: item.id,
                character_id: character.id,
                required_level,
            };
        }
        return ItemDriverOutcome::Teleport {
            item_id: item.id,
            character_id: character.id,
            x,
            y,
            area_id,
            stop_driver: true,
            quiet: false,
        };
    }

    ItemDriverOutcome::LabEntranceSolvedAll {
        item_id: item.id,
        character_id: character.id,
    }
}

pub(crate) fn labexit_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call && character.id.0 == 0 {
        let frame = drdata_u32(item, 8);
        if frame < 24 {
            item.sprite = 1060 + (frame % 24) as i32;
        } else if frame < 240 {
            item.sprite = 1060 + (frame % 24) as i32 + 24;
        } else if frame < 240 + 24 {
            item.sprite = 1060 + (frame % 24) as i32 + 48;
        } else {
            return ItemDriverOutcome::LabExitExpired { item_id: item.id };
        }

        let next_frame = frame.saturating_add(1);
        set_drdata_u32(item, 8, next_frame);
        return ItemDriverOutcome::LabExitAnimating {
            item_id: item.id,
            sprite: item.sprite,
            frame: next_frame,
            schedule_after_ticks: 2,
        };
    }

    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let owner_id = drdata_u32(item, 0);
    if character.id.0 != owner_id {
        return ItemDriverOutcome::LabExitWrongOwner {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let frame = drdata_u32(item, 8);
    let close_frame = 240 - 24 + (frame % 24);
    set_drdata_u32(item, 8, close_frame);

    ItemDriverOutcome::LabExitUse {
        item_id: item.id,
        character_id: character.id,
        lab_nr: drdata(item, 4),
        frame: close_frame,
        target_area: 3,
        target_x: 183,
        target_y: 199,
    }
}
