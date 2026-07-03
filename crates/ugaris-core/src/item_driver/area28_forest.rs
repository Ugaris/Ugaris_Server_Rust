use super::*;

pub(crate) fn brannington_forest_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 || !character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }

    match drdata(item, 0) {
        1 => ItemDriverOutcome::BranningtonUnderwaterBerry {
            item_id: item.id,
            character_id: character.id,
            duration_ticks: TICKS_PER_SECOND * 30,
            installed: false,
        },
        _ => ItemDriverOutcome::Noop,
    }
}
