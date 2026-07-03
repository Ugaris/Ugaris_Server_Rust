use super::*;

pub(crate) fn special_shrine_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::SpecialShrine {
        item_id: item.id,
        character_id: character.id,
        kind: drdata(item, 0),
    }
}

pub(crate) fn demonshrine_driver(
    character: &Character,
    item: &Item,
    area_id: u16,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    if character.level < u32::from(item.min_level) {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::DemonShrine {
        item_id: item.id,
        character_id: character.id,
        location_id: u32::from(item.x) + (u32::from(item.y) << 8) + (u32::from(area_id) << 16),
    }
}
