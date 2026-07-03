use super::*;

pub(crate) fn orbspawn_driver(character: &Character, item: &Item, anti: bool) -> ItemDriverOutcome {
    if character.cursor_item.is_some()
        || character.level < u32::from(item.min_level)
        || !character.flags.contains(CharacterFlags::PAID)
    {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::OrbSpawn {
        item_id: item.id,
        character_id: character.id,
        anti,
        special: anti && drdata(item, 0) == 1,
    }
}

pub(crate) fn enchant_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::EnchantNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    };

    ItemDriverOutcome::EnchantCursorItem {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        modifier: i16::from(drdata(item, 0)),
        amount: i16::from(drdata(item, 1)),
    }
}

pub(crate) fn anti_enchant_driver(
    character: &Character,
    item: &Item,
    extract_orb: bool,
) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) else {
        return ItemDriverOutcome::EnchantNeedsCursor {
            item_id: item.id,
            character_id: character.id,
        };
    };

    ItemDriverOutcome::AntiEnchantCursorItem {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        modifier: i16::from(drdata(item, 0)),
        amount: i16::from(drdata(item, 1)),
        extract_orb,
    }
}
