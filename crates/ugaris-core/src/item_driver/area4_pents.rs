use super::*;

pub(crate) fn pentagram_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    let level = drdata(item, 0);
    let status = drdata(item, 1);
    let color = drdata(item, 2);
    let area_status = drdata(item, 4);

    if context.timer_call || character.id.0 == 0 {
        return ItemDriverOutcome::PentagramTimer {
            item_id: item.id,
            level,
            status,
            area_status,
        };
    }

    if status != 0 {
        return ItemDriverOutcome::PentagramAlreadyActive {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::PentagramActivate {
        item_id: item.id,
        character_id: character.id,
        level,
        color,
    }
}

pub(crate) fn pent_boss_door_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let offset_x = i32::from(character.x) - i32::from(item.x);
    let offset_y = i32::from(character.y) - i32::from(item.y);

    let access_ticks = context
        .pent_demon_lord_access_seconds
        .unwrap_or(120)
        .saturating_mul(TICKS_PER_SECOND as u32);
    let recently_solved = context
        .pent_last_solve_tick
        .is_some_and(|last| context.current_tick.saturating_sub(last) <= access_ticks);
    if !recently_solved && (offset_x > 0 || offset_y > 0) {
        return ItemDriverOutcome::PentBossDoorLocked {
            item_id: item.id,
            character_id: character.id,
        };
    }

    if offset_x != 0 && offset_y != 0 {
        return ItemDriverOutcome::Noop;
    }

    let target_x = i32::from(item.x) - offset_x;
    let target_y = i32::from(item.y) - offset_y;
    if target_x < 1
        || target_x > MAX_MAP as i32 - 2
        || target_y < 1
        || target_y > MAX_MAP as i32 - 2
    {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::PentBossDoor {
        item_id: item.id,
        character_id: character.id,
        x: target_x as u16,
        y: target_y as u16,
    }
}
