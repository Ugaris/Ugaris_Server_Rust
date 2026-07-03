use super::*;

pub(crate) fn ratchest_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    if character.cursor_item.is_some() {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::RatChest {
        item_id: item.id,
        character_id: character.id,
    }
}
