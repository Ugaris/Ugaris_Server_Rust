use super::*;

pub(crate) fn balltrap_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 || character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }

    let dx = i16::from(drdata(item, 0)) - 128;
    let dy = i16::from(drdata(item, 1)) - 128;
    let dxs = dx.signum();
    let dys = dy.signum();
    let item_x = i32::from(item.x);
    let item_y = i32::from(item.y);

    ItemDriverOutcome::BallTrapProjectile {
        item_id: item.id,
        character_id: character.id,
        start_x: clamp_legacy_coordinate(item_x + i32::from(dxs)),
        start_y: clamp_legacy_coordinate(item_y + i32::from(dys)),
        target_x: clamp_legacy_coordinate(item_x + i32::from(dx)),
        target_y: clamp_legacy_coordinate(item_y + i32::from(dy)),
        power: drdata(item, 2),
    }
}

pub(crate) fn usetrap_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::TriggerMapItem {
        item_id: item.id,
        character_id: character.id,
        x: u16::from(drdata(item, 0)),
        y: u16::from(drdata(item, 1)),
        target_character_id: character.id,
        delay_ticks: TICKS_PER_SECOND / 2,
    }
}

pub(crate) fn steptrap_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call || character.id.0 == 0 {
        if drdata(item, 0) == 0 {
            return ItemDriverOutcome::StepTrapDiscoverTarget { item_id: item.id };
        }
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::TriggerMapItem {
        item_id: item.id,
        character_id: character.id,
        x: u16::from(drdata(item, 0)),
        y: u16::from(drdata(item, 1)),
        target_character_id: CharacterId(0),
        delay_ticks: 1,
    }
}
