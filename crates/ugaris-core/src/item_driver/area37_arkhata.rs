use super::*;

pub(crate) fn arkhata_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    match drdata(item, 0) {
        0 => arkhata_pool_driver(character, item, context),
        1 => arkhata_stopwatch_driver(character, item),
        2 => arkhata_key_assemble_driver(character, item, context),
        _ => ItemDriverOutcome::Unsupported {
            driver: IDR_ARKHATA,
            item_id: item.id,
            character_id: character.id,
        },
    }
}

pub(crate) fn arkhata_stopwatch_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 != 0 {
        return ItemDriverOutcome::Noop;
    }

    let Some(character_id) = item.carried_by.filter(|id| id.0 != 0) else {
        return ItemDriverOutcome::Noop;
    };

    ItemDriverOutcome::ArkhataStopwatch {
        item_id: item.id,
        character_id,
        schedule_after_ticks: 10,
    }
}

pub(crate) fn arkhata_pool_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::ArkhataPoolNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    };
    if context.cursor_template_id != Some(IID_ARKHATA_SCROLL1) {
        return ItemDriverOutcome::ArkhataPoolWrongCursor {
            item_id: item.id,
            character_id: character.id,
            cursor_item_id,
        };
    }

    ItemDriverOutcome::ArkhataPool {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
    }
}

pub(crate) fn arkhata_key_assemble_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::ArkhataKeyNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    };
    let Some(cursor_template_id) = context.cursor_template_id else {
        return ItemDriverOutcome::ArkhataKeyDoesNotFit {
            item_id: item.id,
            character_id: character.id,
        };
    };

    let result = match (item.template_id, cursor_template_id) {
        (IID_ARKHATA_AKEY1, IID_ARKHATA_AKEY2) | (IID_ARKHATA_AKEY2, IID_ARKHATA_AKEY1) => {
            Some((IID_ARKHATA_AKEY12, 13421, false))
        }
        (IID_ARKHATA_AKEY2, IID_ARKHATA_AKEY3) | (IID_ARKHATA_AKEY3, IID_ARKHATA_AKEY2) => {
            Some((IID_ARKHATA_AKEY23, 13420, false))
        }
        (IID_ARKHATA_AKEY1, IID_ARKHATA_AKEY23)
        | (IID_ARKHATA_AKEY23, IID_ARKHATA_AKEY1)
        | (IID_ARKHATA_AKEY12, IID_ARKHATA_AKEY3)
        | (IID_ARKHATA_AKEY3, IID_ARKHATA_AKEY12) => Some((IID_ARKHATA_AKEY, 13413, true)),
        _ => None,
    };

    let Some((result_template_id, result_sprite, final_key)) = result else {
        return ItemDriverOutcome::ArkhataKeyDoesNotFit {
            item_id: item.id,
            character_id: character.id,
        };
    };

    ItemDriverOutcome::ArkhataKeyAssemble {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        result_template_id,
        result_sprite,
        final_key,
    }
}
