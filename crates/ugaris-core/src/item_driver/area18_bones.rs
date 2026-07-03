use super::*;

pub(crate) fn bonebridge_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call {
        if drdata(item, 1) == 0 {
            return ItemDriverOutcome::Noop;
        }
        return ItemDriverOutcome::BoneBridgeTimerTick { item_id: item.id };
    }

    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    if drdata(item, 0) != 0 && drdata(item, 1) == 0 {
        // Adding/removing bones from a partial carried bridge depends on creating
        // the generic "bone" template and is applied as a later area-18 slice.
        return ItemDriverOutcome::Noop;
    }

    let Some(cursor_item_id) = character.cursor_item else {
        return ItemDriverOutcome::Noop;
    };
    if context.cursor_template_id != Some(IID_AREA18_BONE) || context.cursor_drdata0 != Some(5) {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::BoneBridgePlace {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
    }
}

pub(crate) fn bonehint_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 || item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }

    if drdata(item, 1) == 0 {
        let Some(nr) = context.bone_hint_nr else {
            return ItemDriverOutcome::Noop;
        };
        let Some(pos) = context.bone_hint_pos else {
            return ItemDriverOutcome::Noop;
        };
        set_drdata(item, 1, 1);
        set_drdata(item, 2, nr.min(4));
        set_drdata(item, 3, pos.min(2));
    }

    ItemDriverOutcome::BoneHint {
        item_id: item.id,
        character_id: character.id,
        level: drdata(item, 0),
        nr: drdata(item, 2),
        pos: drdata(item, 3),
    }
}

pub(crate) fn boneladder_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let (dx, dy) = if drdata(item, 0) != 0 {
        (-4, -3)
    } else {
        (4, 3)
    };
    let x = (i32::from(item.x) + dx).max(0) as u16;
    let y = (i32::from(item.y) + dy).max(0) as u16;

    ItemDriverOutcome::Teleport {
        item_id: item.id,
        character_id: character.id,
        x,
        y,
        area_id: 0,
        stop_driver: false,
        quiet: false,
    }
}

pub(crate) fn boneholder_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    let holder_kind = drdata(item, 1);
    if holder_kind == 2 || holder_kind == 3 {
        if character.id.0 == 0 {
            return ItemDriverOutcome::Noop;
        }
        return ItemDriverOutcome::BoneHolderActivate {
            item_id: item.id,
            character_id: character.id,
            last_holder: holder_kind == 3,
        };
    }

    if context.timer_call || character.id.0 == 0 {
        let placed_tick = drdata_u32(item, 12);
        let expiry_ticks = (TICKS_PER_SECOND as u32) * 120;
        if context.current_tick.saturating_sub(placed_tick) < expiry_ticks {
            return ItemDriverOutcome::Noop;
        }
        if drdata(item, 0) == 0 {
            return ItemDriverOutcome::Noop;
        }
        set_drdata(item, 0, 0);
        return ItemDriverOutcome::BoneHolderExpired { item_id: item.id };
    }

    if let Some(cursor_item_id) = character.cursor_item.filter(|cursor| *cursor != item.id) {
        let Some(cursor_template_id) = context.cursor_template_id else {
            return ItemDriverOutcome::BoneHolderBadCursor {
                item_id: item.id,
                character_id: character.id,
            };
        };
        if !(IID_AREA18_RUNE1..=IID_AREA18_RUNE9).contains(&cursor_template_id) {
            return ItemDriverOutcome::BoneHolderBadCursor {
                item_id: item.id,
                character_id: character.id,
            };
        }
        if drdata(item, 0) != 0 {
            return ItemDriverOutcome::BoneHolderOccupied {
                item_id: item.id,
                character_id: character.id,
            };
        }

        let rune = (cursor_template_id - IID_AREA18_RUNE1 + 1) as u8;
        set_drdata(item, 0, rune);
        set_drdata_u32(item, 8, character.id.0);
        set_drdata_u32(item, 12, context.current_tick);
        return ItemDriverOutcome::BoneHolderInsertRune {
            item_id: item.id,
            character_id: character.id,
            cursor_item_id,
            rune,
            owner_character_id: character.id.0,
            placed_tick: context.current_tick,
            schedule_after_ticks: (TICKS_PER_SECOND as u32) * 120 + 1,
        };
    }

    let rune = drdata(item, 0);
    if rune == 0 {
        return ItemDriverOutcome::BoneHolderEmptyTouch {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if drdata_u32(item, 8) != character.id.0 {
        return ItemDriverOutcome::BoneHolderWrongOwner {
            item_id: item.id,
            character_id: character.id,
        };
    }

    set_drdata(item, 0, 0);
    ItemDriverOutcome::BoneHolderRemoveRune {
        item_id: item.id,
        character_id: character.id,
        rune,
    }
}

pub(crate) fn bonewall_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    let state = drdata(item, 0);
    if context.timer_call && character.id.0 == 0 && state == 0 {
        return ItemDriverOutcome::Noop;
    }
    if !context.timer_call && character.id.0 != 0 && state != 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::BoneWallTick {
        item_id: item.id,
        character_id: character.id,
    }
}
