use super::*;

pub(crate) fn caligar_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    match drdata(item, 0) {
        1 => caligar_training_driver(character, item),
        2 | 4 => caligar_weight_driver(character, item),
        3 => caligar_weight_door_driver(character, item),
        5..=9 => caligar_gun_driver(character, item),
        10 => caligar_key_assembly_driver(character, item, context),
        11 => extinguish_driver(character, item),
        12 => caligar_skelly_door_driver(character, item),
        _ => ItemDriverOutcome::Unsupported {
            driver: IDR_CALIGAR,
            item_id: item.id,
            character_id: character.id,
        },
    }
}

pub(crate) fn caligar_skelly_door_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::CaligarSkellyDoor {
        item_id: item.id,
        character_id: character.id,
        door_index: drdata(item, 1),
    }
}

pub(crate) fn caligar_key_assembly_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::CaligarKeyNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    };
    if context.cursor_template_id != Some(IID_CALIGAR_PALACE_KEY_PART) {
        return ItemDriverOutcome::CaligarKeyNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let sp1 = item.sprite.min(context.cursor_sprite.unwrap_or_default());
    let sp2 = item.sprite.max(context.cursor_sprite.unwrap_or_default());
    let result = match (sp1, sp2) {
        (13414, 13415) => Some((13421, false)),
        (13415, 13416) => Some((13420, false)),
        (13414, 13420) | (13416, 13421) => Some((0, true)),
        _ => None,
    };

    let Some((result_sprite, final_key)) = result else {
        return ItemDriverOutcome::CaligarKeyDoesNotFit {
            item_id: item.id,
            character_id: character.id,
        };
    };

    ItemDriverOutcome::CaligarKeyAssemble {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        result_sprite,
        final_key,
    }
}

pub(crate) fn caligar_training_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 || !character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }

    match drdata(item, 1) {
        1..=3 => ItemDriverOutcome::CaligarTraining {
            item_id: item.id,
            character_id: character.id,
            lesson: drdata(item, 1),
        },
        _ => ItemDriverOutcome::Noop,
    }
}

pub(crate) fn caligar_weight_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::CaligarWeightTimer { item_id: item.id };
    }

    ItemDriverOutcome::CaligarWeightMove {
        item_id: item.id,
        character_id: character.id,
    }
}

pub(crate) fn caligar_weight_door_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::CaligarWeightDoor {
        item_id: item.id,
        character_id: character.id,
    }
}

pub(crate) fn caligar_gun_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    ItemDriverOutcome::CaligarGunProjectile {
        item_id: item.id,
        character_id: character.id,
        direction: drdata(item, 0) - 4,
        schedule_after_ticks: 12,
    }
}
