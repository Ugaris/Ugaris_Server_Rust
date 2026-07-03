use super::*;

pub(crate) fn lq_ticker_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 != 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::LqTicker {
        item_id: item.id,
        schedule_after_ticks: TICKS_PER_SECOND,
    }
}

pub(crate) fn lq_entrance_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    if !context.lq_open {
        return ItemDriverOutcome::LqEntranceClosed {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if u32::from(context.lq_min_level) > character.level
        || u32::from(context.lq_max_level) < character.level
    {
        return ItemDriverOutcome::LqEntranceLevelBlocked {
            item_id: item.id,
            character_id: character.id,
            min_level: context.lq_min_level,
            max_level: context.lq_max_level,
        };
    }
    let Some((x, y)) = context.lq_entrance else {
        return ItemDriverOutcome::LqEntranceUndefined {
            item_id: item.id,
            character_id: character.id,
        };
    };
    if let Some(remaining_seconds) = context.lq_death_penalty_seconds {
        if remaining_seconds != 0 {
            return ItemDriverOutcome::LqEntrancePenalty {
                item_id: item.id,
                character_id: character.id,
                remaining_seconds,
            };
        }
    }

    ItemDriverOutcome::Teleport {
        item_id: item.id,
        character_id: character.id,
        x,
        y,
        area_id: 20,
        stop_driver: true,
        quiet: true,
    }
}
