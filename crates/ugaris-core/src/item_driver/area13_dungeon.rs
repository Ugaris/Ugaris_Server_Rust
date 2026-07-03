use super::*;

pub(crate) fn dungeon_teleport_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if !character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::DungeonTeleport {
        item_id: item.id,
        character_id: character.id,
        x: drdata_u16(item, 0),
        y: drdata_u16(item, 2),
        clan_number: drdata_u16(item, 4),
    }
}

pub(crate) fn dungeon_fake_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    ItemDriverOutcome::DungeonFake {
        item_id: item.id,
        character_id: character.id,
        clan_number: drdata_u16(item, 0),
    }
}

pub(crate) fn dungeon_key_driver(character: &Character, item: &mut Item) -> ItemDriverOutcome {
    if character.cursor_item.is_some() {
        return ItemDriverOutcome::DungeonKeyCursorOccupied {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let first_taken = drdata(item, 2) == 0;
    if first_taken {
        set_drdata(item, 2, 1);
    }

    ItemDriverOutcome::DungeonKey {
        item_id: item.id,
        character_id: character.id,
        template: if drdata(item, 0) == 1 {
            "maze_key1"
        } else {
            "maze_key2"
        },
        key_id: drdata_u32(item, 4),
        clan_number: drdata(item, 1),
        first_taken,
    }
}

pub(crate) fn dungeon_door_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let key1 = drdata_u32(item, 0);
    let key2 = drdata_u32(item, 4);
    let mut missing = 0;
    if key1 != 0 && !context.has_dungeon_door_key1 {
        missing += 1;
    }
    if key2 != 0 && !context.has_dungeon_door_key2 {
        missing += 1;
    }
    if missing != 0 {
        return ItemDriverOutcome::DungeonDoorMissingKeys {
            item_id: item.id,
            character_id: character.id,
            missing,
            both_required: key1 != 0 && key2 != 0,
        };
    }

    let alive = context.dungeon_defender_count.unwrap_or(0);
    if alive > 20 {
        return ItemDriverOutcome::DungeonDoorTooManyDefenders {
            item_id: item.id,
            character_id: character.id,
            alive,
            max_allowed: 20,
        };
    }

    let catacomb = (((u32::from(item.x).saturating_sub(2)) / 81)
        + ((u32::from(item.y).saturating_sub(2)) / 81) * 3) as u8;
    let first_solve = drdata(item, 12) == 0;
    if first_solve {
        set_drdata_u32(item, 0, 0);
        set_drdata_u32(item, 4, 0);
        set_drdata(item, 12, 1);
    }

    ItemDriverOutcome::DungeonDoorSolved {
        item_id: item.id,
        character_id: character.id,
        clan_number: drdata_u32(item, 8),
        catacomb,
        first_solve,
    }
}
