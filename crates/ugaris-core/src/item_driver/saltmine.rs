use super::*;

pub(crate) fn saltmine_item_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    match drdata(item, 0) {
        1 if character.flags.contains(CharacterFlags::PLAYER) => {
            ItemDriverOutcome::SaltmineLadderUse {
                item_id: item.id,
                character_id: character.id,
                ladder_index: drdata(item, 1),
            }
        }
        2 if character.flags.contains(CharacterFlags::PLAYER) => {
            ItemDriverOutcome::SaltmineSaltbagUse {
                item_id: item.id,
                character_id: character.id,
            }
        }
        // C saltmine door: monk workers are removed; every other user is rejected.
        // Worker state is still deferred until the saltmine character driver is ported.
        3 => ItemDriverOutcome::SaltmineDoorBlocked {
            item_id: item.id,
            character_id: character.id,
        },
        _ => ItemDriverOutcome::Noop,
    }
}
