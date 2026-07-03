use super::*;

pub(crate) fn teufel_arena_exit_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let max_hp = character
        .values
        .get(0)
        .and_then(|values| values.get(CharacterValue::Hp as usize))
        .copied()
        .unwrap_or_default() as i32
        * POWERSCALE;
    if character.hp < max_hp {
        return ItemDriverOutcome::TeufelArenaExitLowHealth {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::TeufelArenaExit {
        item_id: item.id,
        character_id: character.id,
        x: 206,
        y: 231,
    }
}

pub(crate) fn teufel_arena_destination(kind: u8, roll: u8) -> Option<(u16, u16)> {
    if kind != 1 {
        return None;
    }
    Some(match roll % 8 {
        0 => (154, 215),
        1 => (134, 220),
        2 => (167, 196),
        3 => (186, 221),
        4 => (212, 223),
        5 => (228, 224),
        6 => (247, 220),
        7 => (237, 198),
        _ => unreachable!(),
    })
}

pub(crate) fn teufel_arena_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let kind = item.driver_data.first().copied().unwrap_or_default();
    let Some((x, y)) =
        teufel_arena_destination(kind, context.teufel_arena_roll.unwrap_or_default())
    else {
        return ItemDriverOutcome::Noop;
    };

    if kind == 1 && character.sprite != 27 {
        return ItemDriverOutcome::TeufelArenaNeedsSuit {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if kind == 1 && character.level > 38 {
        return ItemDriverOutcome::TeufelArenaLevelTooHigh {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::TeufelArena {
        item_id: item.id,
        character_id: character.id,
        x,
        y,
    }
}

pub(crate) fn teufel_door_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    if !matches!(character.sprite, 27 | 157 | 39) {
        return ItemDriverOutcome::TeufelDoorNoHumans {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let door_kind = item.driver_data.first().copied().unwrap_or_default();
    if door_kind == 2 && character.sprite == 27 {
        return ItemDriverOutcome::TeufelDoorNoBeggars {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if door_kind == 3 && character.sprite != 39 {
        return ItemDriverOutcome::TeufelDoorOnlyNobles {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let dx = i32::from(character.x) - i32::from(item.x);
    let dy = i32::from(character.y) - i32::from(item.y);
    if dx != 0 && dy != 0 {
        return ItemDriverOutcome::Noop;
    }

    let x = i32::from(item.x) - dx;
    let y = i32::from(item.y) - dy;
    if x < 1 || x > MAX_MAP as i32 - 2 || y < 1 || y > MAX_MAP as i32 - 2 {
        return ItemDriverOutcome::TeufelDoorBug {
            item_id: item.id,
            character_id: character.id,
            x,
            y,
        };
    }

    ItemDriverOutcome::TeufelDoor {
        item_id: item.id,
        character_id: character.id,
        x: x as u16,
        y: y as u16,
    }
}

pub(crate) fn teufel_ratnest_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        if !context.timer_call {
            return ItemDriverOutcome::Noop;
        }

        if drdata(item, 2) > 0 {
            let remaining = drdata(item, 2).saturating_sub(1);
            set_drdata(item, 2, remaining);
            if remaining == 0 {
                item.sprite = 15281;
            } else {
                return ItemDriverOutcome::Noop;
            }
        }

        let mut wave = drdata_u16(item, 0);
        if wave > 0 {
            wave -= 1;
            set_drdata_u16(item, 0, wave);
        }
        let nest_kind = drdata(item, 4);
        let (level, template) = teufel_ratnest_spawn(nest_kind, wave);
        item.description = format!(
            "An Ice Rat nest[{}]. You could destroy it...[{},{}]",
            nest_kind, wave, level
        );

        return ItemDriverOutcome::TeufelRatNestSpawn {
            item_id: item.id,
            nest_kind,
            wave,
            level,
            template,
            schedule_after_ticks: TICKS_PER_SECOND * 20,
        };
    }

    if context.teufel_ratnest_guard_active {
        return ItemDriverOutcome::TeufelRatNestGuarded {
            item_id: item.id,
            character_id: character.id,
        };
    }

    set_drdata_u16(item, 0, 0);
    set_drdata(item, 2, 5);
    item.sprite = 0;
    ItemDriverOutcome::TeufelRatNestDestroyed {
        item_id: item.id,
        character_id: character.id,
    }
}

pub(crate) fn teufel_ratnest_spawn(nest_kind: u8, wave: u16) -> (u16, &'static str) {
    let tier = match wave {
        0..=99 => 0,
        100..=199 => 1,
        200..=399 => 2,
        400..=799 => 3,
        800..=1599 => 4,
        1600..=3199 => 5,
        3200..=6399 => 6,
        6400..=12799 => 7,
        12800..=25599 => 8,
        _ => 9,
    };

    match nest_kind {
        1 => {
            const LEVELS: [u16; 10] = [70, 74, 78, 83, 87, 92, 96, 100, 103, 108];
            const TEMPLATES: [&str; 10] = [
                "rat80", "rat80b", "rat81", "rat81b", "rat82", "rat82b", "rat83", "rat83b",
                "rat84", "rat84b",
            ];
            (LEVELS[tier], TEMPLATES[tier])
        }
        2 => {
            const LEVELS: [u16; 10] = [109, 113, 117, 121, 125, 129, 133, 137, 141, 145];
            const TEMPLATES: [&str; 10] = [
                "rat90", "rat90b", "rat91", "rat91b", "rat92", "rat92b", "rat93", "rat93b",
                "rat94", "rat94b",
            ];
            (LEVELS[tier], TEMPLATES[tier])
        }
        _ => {
            const LEVELS: [u16; 10] = [45, 47, 50, 53, 56, 60, 63, 66, 70, 73];
            const TEMPLATES: [&str; 10] = [
                "rat70", "rat70b", "rat71", "rat71b", "rat72", "rat72b", "rat73", "rat73b",
                "rat74", "rat74b",
            ];
            (LEVELS[tier], TEMPLATES[tier])
        }
    }
}
