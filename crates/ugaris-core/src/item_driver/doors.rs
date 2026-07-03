use super::*;

pub(crate) fn double_door_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if item.x == 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::DoubleDoorToggle {
        item_id: item.id,
        character_id: character.id,
    }
}

pub(crate) fn teleport_door_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if item.x == 0 || item.y == 0 {
        return ItemDriverOutcome::Noop;
    }

    let dx = i32::from(character.x) - i32::from(item.x);
    let dy = i32::from(character.y) - i32::from(item.y);
    if (dx != 0 && dy != 0) || (dx == 0 && dy == 0) {
        return ItemDriverOutcome::Noop;
    }

    match drdata(item, 0) {
        1 if dx == 1 => return ItemDriverOutcome::Noop,
        2 if dx == -1 => return ItemDriverOutcome::Noop,
        3 if dy == 1 => return ItemDriverOutcome::Noop,
        4 if dy == -1 => return ItemDriverOutcome::Noop,
        _ => {}
    }

    let max_level = drdata(item, 1);
    if max_level != 0 && character.level > u32::from(max_level) {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let target_x = i32::from(item.x) - dx;
    let target_y = i32::from(item.y) - dy;
    if target_x < 1
        || target_y < 1
        || target_x > i32::from(u16::MAX)
        || target_y > i32::from(u16::MAX)
    {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::TeleportDoor {
        item_id: item.id,
        character_id: character.id,
        x: target_x as u16,
        y: target_y as u16,
    }
}

pub(crate) fn door_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if item.x == 0 {
        return ItemDriverOutcome::Noop;
    }

    if context.timer_call {
        item.driver_data.resize(40, 0);
        if item.driver_data[39] != 0 {
            item.driver_data[39] -= 1;
        }
        if drdata(item, 0) == 0 || item.driver_data[39] != 0 || drdata(item, 5) != 0 {
            return ItemDriverOutcome::Noop;
        }
    }

    let required_key_id = door_required_key_id(item);
    if !context.timer_call && required_key_id != 0 {
        if let Some(key) = context
            .door_key
            .as_ref()
            .filter(|key| key.key_id == required_key_id || key.key_id == IID_SKELETON_KEY)
        {
            return ItemDriverOutcome::KeyedDoorToggle {
                item_id: item.id,
                character_id: character.id,
                key_id: key.key_id,
                source: key.source,
                locking: drdata(item, 0) != 0,
            };
        }
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::DoorToggle {
        item_id: item.id,
        character_id: character.id,
    }
}

pub(crate) fn door_required_key_id(item: &Item) -> u32 {
    u32::from(drdata(item, 1))
        | (u32::from(drdata(item, 2)) << 8)
        | (u32::from(drdata(item, 3)) << 16)
        | (u32::from(drdata(item, 4)) << 24)
}
