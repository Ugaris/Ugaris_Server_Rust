use super::*;

pub(crate) fn oxy_potion_driver(
    character: &Character,
    item: &Item,
    area_id: u16,
) -> ItemDriverOutcome {
    if area_id != 31 {
        return ItemDriverOutcome::BlockedByArea {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::OxygenPotion {
        item_id: item.id,
        character_id: character.id,
        installed: false,
    }
}

pub(crate) fn pick_berry_driver(
    character: &Character,
    item: &Item,
    area_id: u16,
) -> ItemDriverOutcome {
    if area_id != 31 {
        return ItemDriverOutcome::BlockedByArea {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    if character.cursor_item.is_some() {
        return ItemDriverOutcome::PickBerryCursorOccupied {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::PickBerry {
        item_id: item.id,
        character_id: character.id,
        kind: drdata(item, 0),
        location_id: u32::from(item.x) + (u32::from(item.y) << 8) + (u32::from(area_id) << 16),
    }
}

pub(crate) fn alchemy_flower_driver(
    character: &Character,
    item: &Item,
    area_id: u16,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    if character.cursor_item.is_some() {
        return ItemDriverOutcome::PickAlchemyFlowerCursorOccupied {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::PickAlchemyFlower {
        item_id: item.id,
        character_id: character.id,
        kind: item.driver_data.first().copied().unwrap_or_default(),
        location_id: u32::from(item.x) + (u32::from(item.y) << 8) + (u32::from(area_id) << 16),
    }
}

pub(crate) fn lizard_flower_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
    area_id: u16,
) -> ItemDriverOutcome {
    if area_id != 31 {
        return ItemDriverOutcome::BlockedByArea {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if character.id.0 == 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::LizardFlowerNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    };

    if context.cursor_driver != Some(IDR_LIZARDFLOWER) {
        return ItemDriverOutcome::LizardFlowerDoesNotFit {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let combined_bits = drdata(item, 0) | context.cursor_drdata0.unwrap_or_default();
    ItemDriverOutcome::LizardFlowerMixed {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        combined_bits,
        complete: combined_bits == 7,
        bottle_message: item.sprite != 11189 && context.cursor_sprite != Some(11189),
    }
}
