use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForestSpadeFind {
    ForestNote1,
    BranningtonTreasure { dig_index: u8 },
}

pub(crate) fn forest_spade_driver(
    character: &Character,
    item: &Item,
    area_id: u16,
) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }
    if character.cursor_item.is_some() {
        return ItemDriverOutcome::ForestSpadeCursorOccupied {
            item_id: item.id,
            character_id: character.id,
        };
    }

    match (area_id, character.x, character.y) {
        (16, 205, 234) => ItemDriverOutcome::ForestSpadeFind {
            item_id: item.id,
            character_id: character.id,
            find: ForestSpadeFind::ForestNote1,
        },
        (16, 130, 219) => ItemDriverOutcome::ForestSpadeCollapse {
            item_id: item.id,
            character_id: character.id,
            x: 44,
            y: 231,
        },
        (1, 93, 36) => ItemDriverOutcome::ForestSpadeCollapse {
            item_id: item.id,
            character_id: character.id,
            x: 106,
            y: 211,
        },
        (29, 83, 127) => forest_spade_treasure(item.id, character.id, 0),
        (29, 94, 222) => forest_spade_treasure(item.id, character.id, 1),
        (29, 214, 136) => forest_spade_treasure(item.id, character.id, 2),
        (29, 185, 22) => forest_spade_treasure(item.id, character.id, 3),
        (29, 165, 79) => forest_spade_treasure(item.id, character.id, 4),
        _ => ItemDriverOutcome::ForestSpadeNothing {
            item_id: item.id,
            character_id: character.id,
        },
    }
}

pub(crate) fn forest_spade_treasure(
    item_id: ItemId,
    character_id: CharacterId,
    dig_index: u8,
) -> ItemDriverOutcome {
    ItemDriverOutcome::ForestSpadeFind {
        item_id,
        character_id,
        find: ForestSpadeFind::BranningtonTreasure { dig_index },
    }
}

pub(crate) fn forest_chest_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    if character.cursor_item.is_some() {
        return ItemDriverOutcome::ForestChestCursorOccupied {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let (has_key, amount, imp_flag_mask) = if drdata(item, 0) == 0 {
        (context.has_area16_robber_key, 9_733, 1)
    } else {
        (context.has_area16_skelly_key, 17_587, 2)
    };

    if !has_key {
        return ItemDriverOutcome::ForestChestLocked {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::ForestChest {
        item_id: item.id,
        character_id: character.id,
        amount,
        imp_flag_mask,
    }
}
