use super::*;

pub(crate) fn xmasmaker_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0
        || !character
            .flags
            .intersects(CharacterFlags::STAFF | CharacterFlags::GOD)
    {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::XmasMaker {
        item_id: item.id,
        character_id: character.id,
    }
}

pub(crate) fn xmastree_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::XmasTree {
        item_id: item.id,
        character_id: character.id,
    }
}
