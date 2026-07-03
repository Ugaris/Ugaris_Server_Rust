use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RandomShrineKind {
    Indecisiveness,
    Bribes,
    Welding,
    Edge,
    Kindness,
    Vitality,
    Death,
    Braveness,
    Security,
    Jobless,
    Continuity,
    Dormant,
}

pub(crate) fn randomshrine_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let shrine_type = drdata(item, 0);
    let level = drdata(item, 1);
    if shrine_type >= 255 {
        if shrine_type == 255 {
            if !context.has_matching_random_shrine_key {
                return ItemDriverOutcome::RandomShrineNeedsKey {
                    item_id: item.id,
                    character_id: character.id,
                    shrine_type,
                    level,
                };
            }
            return ItemDriverOutcome::RandomShrineUse {
                item_id: item.id,
                character_id: character.id,
                shrine_type,
                level,
                kind: RandomShrineKind::Continuity,
            };
        }
        return ItemDriverOutcome::RandomShrineBug {
            item_id: item.id,
            character_id: character.id,
            shrine_type,
        };
    }

    if !context.has_matching_random_shrine_key {
        return ItemDriverOutcome::RandomShrineNeedsKey {
            item_id: item.id,
            character_id: character.id,
            shrine_type,
            level,
        };
    }

    let kind = match shrine_type {
        0..=9 => RandomShrineKind::Indecisiveness,
        10..=19 => RandomShrineKind::Bribes,
        20..=29 => RandomShrineKind::Welding,
        30..=39 => RandomShrineKind::Edge,
        40..=49 => RandomShrineKind::Kindness,
        50 => RandomShrineKind::Vitality,
        51 => RandomShrineKind::Death,
        52 => RandomShrineKind::Braveness,
        53..=62 => RandomShrineKind::Security,
        63..=72 => RandomShrineKind::Jobless,
        73..=254 => RandomShrineKind::Dormant,
        255 => RandomShrineKind::Continuity,
    };

    if context.random_shrine_already_used && !matches!(kind, RandomShrineKind::Dormant) {
        return ItemDriverOutcome::RandomShrineAlreadyUsed {
            item_id: item.id,
            character_id: character.id,
            shrine_type,
            level,
        };
    }

    ItemDriverOutcome::RandomShrineUse {
        item_id: item.id,
        character_id: character.id,
        shrine_type,
        level,
        kind,
    }
}

pub(crate) fn trapdoor_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call || character.id.0 == 0 {
        if drdata(item, 0) == 1 {
            return ItemDriverOutcome::TrapdoorClose { item_id: item.id };
        }
        return ItemDriverOutcome::Noop;
    }

    if character.x == item.x && character.y == item.y {
        if drdata(item, 0) != 0 || !character.flags.contains(CharacterFlags::PLAYER) {
            return ItemDriverOutcome::Noop;
        }
        let Ok(direction) = crate::direction::Direction::try_from(character.dir) else {
            return ItemDriverOutcome::Noop;
        };
        let (dx, dy) = direction.delta();
        let target_x = i32::from(character.x) - i32::from(dx);
        let target_y = i32::from(character.y) - i32::from(dy);
        let (Ok(target_x), Ok(target_y)) = (u16::try_from(target_x), u16::try_from(target_y))
        else {
            return ItemDriverOutcome::Noop;
        };
        return ItemDriverOutcome::TrapdoorOpen {
            item_id: item.id,
            character_id: character.id,
            target_x,
            target_y,
            schedule_after_ticks: TICKS_PER_SECOND * 6,
        };
    }

    if drdata(item, 0) != 0 {
        return ItemDriverOutcome::TrapdoorBusy {
            item_id: item.id,
            character_id: character.id,
        };
    }

    if character.cursor_item.is_some() && context.cursor_template_id == Some(IID_AREA14_STEELBAR) {
        return ItemDriverOutcome::TrapdoorBlocked {
            item_id: item.id,
            character_id: character.id,
            cursor_item_id: character.cursor_item.expect("checked cursor above"),
        };
    }

    ItemDriverOutcome::TrapdoorNeedsStick {
        item_id: item.id,
        character_id: character.id,
    }
}

pub(crate) fn junkpile_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call || character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    if character.cursor_item.is_some() {
        return ItemDriverOutcome::JunkpileCursorOccupied {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::JunkpileSearch {
        item_id: item.id,
        character_id: character.id,
        level: drdata(item, 0),
    }
}

pub(crate) fn gastrap_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    let active = drdata(item, 1) != 0;
    let mut schedule_initial_trigger = false;
    if character.id.0 != 0 {
        if active {
            return ItemDriverOutcome::Noop;
        }
        schedule_initial_trigger = true;
    } else if !context.timer_call || !active {
        return ItemDriverOutcome::Noop;
    }

    item.driver_data.resize(2, 0);
    item.driver_data[1] = item.driver_data[1].saturating_add(1);
    let schedule_animation = if item.driver_data[1] == 9 {
        item.driver_data[1] = 0;
        false
    } else {
        true
    };

    ItemDriverOutcome::GasTrapPulse {
        item_id: item.id,
        character_id: character.id,
        power: drdata(item, 0),
        schedule_initial_trigger,
        schedule_animation,
    }
}
