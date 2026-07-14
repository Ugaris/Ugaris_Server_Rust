use super::*;

pub(crate) fn mine_door_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
    area_id: u16,
) -> ItemDriverOutcome {
    if context.timer_call || character.id.0 == 0 {
        return ItemDriverOutcome::MineDoorTimer { item_id: item.id };
    }

    let Some((door_x, door_y, direction)) = context.mine_door_target else {
        return ItemDriverOutcome::MineDoorMissingTarget {
            item_id: item.id,
            character_id: character.id,
        };
    };

    let (target_x, target_y) = mine_door_destination(door_x, door_y, direction);
    let (fallback_x, fallback_y) = if area_id == 31 {
        (211, 231)
    } else {
        (230, 240)
    };
    ItemDriverOutcome::MineDoorTeleport {
        item_id: item.id,
        character_id: character.id,
        target_x,
        target_y,
        fallback_x,
        fallback_y,
    }
}

pub(crate) fn minewall_driver(character: &Character, item: &mut Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        if drdata(item, 4) == 0 {
            set_drdata(item, 4, 1);
            item.sprite = match (u32::from(item.x) + u32::from(item.y)) % 3 {
                0 => 15070,
                1 => 15078,
                _ => 15086,
            };
        }
        if drdata(item, 3) == 8 {
            return ItemDriverOutcome::MineWallCollapse {
                item_id: item.id,
                schedule_after_ticks: TICKS_PER_SECOND as u32,
            };
        }
        return ItemDriverOutcome::MineWallInitialized {
            item_id: item.id,
            sprite: item.sprite,
        };
    }

    if character.cursor_item.is_some() {
        return ItemDriverOutcome::MineWallCursorOccupied {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if character.endurance < POWERSCALE {
        return ItemDriverOutcome::MineWallExhausted {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let miner = character
        .professions
        .get(profession::MINER)
        .copied()
        .unwrap_or_default()
        .max(0) as i32;
    let endurance_delta = -(POWERSCALE / 4 - miner * POWERSCALE / (4 * 25));
    let stage = drdata(item, 3).saturating_add(1);
    set_drdata(item, 3, stage);
    set_drdata(item, 5, 0);
    item.sprite += 1;

    ItemDriverOutcome::MineWallDig {
        item_id: item.id,
        character_id: character.id,
        endurance_delta,
        stage,
        opened: stage >= 8,
    }
}

pub(crate) fn mine_door_destination(x: u16, y: u16, direction: u8) -> (u16, u16) {
    match direction {
        7 => (x, y.saturating_sub(1)),
        3 => (x, y.saturating_add(1)),
        1 => (x.saturating_sub(1), y),
        5 => (x.saturating_add(1), y),
        _ => (x, y),
    }
}

pub(crate) fn mine_gateway_key_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::MineGatewayKeyNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    };
    if context.cursor_driver != Some(IDR_MINEGATEWAYKEY) {
        return ItemDriverOutcome::MineGatewayKeyDoesNotFit {
            item_id: item.id,
            character_id: character.id,
        };
    }
    let combined_bits = drdata(item, 0) | context.cursor_drdata0.unwrap_or(0);
    ItemDriverOutcome::MineGatewayKeyAssemble {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        combined_bits,
    }
}

pub(crate) fn mine_key_door_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::MineKeyDoorNeedsGold {
            item_id: item.id,
            character_id: character.id,
        };
    };
    if context.cursor_driver != Some(IDR_ENHANCE)
        || context.cursor_drdata0 != Some(2)
        || context.cursor_drdata1_u32 != Some(2000)
    {
        return ItemDriverOutcome::MineKeyDoorNeedsGold {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::MineKeyDoor {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        golem_nr: drdata(item, 0),
    }
}

pub(crate) fn mine_gateway_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 || !character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }
    if !context.has_mine_gateway_key {
        return ItemDriverOutcome::MineGatewayNeedsKey {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let x = drdata_u16(item, 0);
    let y = drdata_u16(item, 2);
    let area_id = drdata_u16(item, 4);
    if !(1..=254).contains(&x) || !(1..=254).contains(&y) || area_id == 0 {
        return ItemDriverOutcome::MineGatewayBug {
            item_id: item.id,
            character_id: character.id,
            x,
            y,
            area_id,
        };
    }

    ItemDriverOutcome::MineGateway {
        item_id: item.id,
        character_id: character.id,
        x,
        y,
        area_id,
    }
}
