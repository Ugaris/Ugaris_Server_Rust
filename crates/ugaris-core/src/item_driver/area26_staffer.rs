use super::*;

pub(crate) fn staffer_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    match drdata(item, 0) {
        1 => staffer_spiketrap_driver(character, item),
        2 => staffer_fireball_machine_driver(character, item, context),
        3 => staffer_block_driver(character, item),
        // Vault skull/shelf quest PPD and template rewards are intentionally left for a later slice.
        4 | 5 => ItemDriverOutcome::Noop,
        _ => ItemDriverOutcome::Unsupported {
            driver: IDR_STAFFER,
            item_id: item.id,
            character_id: character.id,
        },
    }
}

pub(crate) fn staffer_spiketrap_driver(
    character: &Character,
    item: &mut Item,
) -> ItemDriverOutcome {
    if character.id.0 != 0 && drdata(item, 1) == 0 {
        item.sprite += 1;
        set_drdata(item, 1, 1);
        return ItemDriverOutcome::SpikeTrapTriggered {
            item_id: item.id,
            character_id: character.id,
            damage: i32::from(drdata(item, 2)) * POWERSCALE,
            reset_after_ticks: TICKS_PER_SECOND,
        };
    }

    if character.id.0 == 0 && drdata(item, 1) != 0 {
        item.sprite -= 1;
        set_drdata(item, 1, 0);
        return ItemDriverOutcome::SpikeTrapReset { item_id: item.id };
    }

    ItemDriverOutcome::Noop
}

pub(crate) fn staffer_fireball_machine_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 != 0 {
        return ItemDriverOutcome::Noop;
    }

    let dx = i16::from(drdata(item, 1)) - 128;
    let dy = i16::from(drdata(item, 2)) - 128;
    let dxs = dx.signum();
    let dys = dy.signum();
    let item_x = i32::from(item.x);
    let item_y = i32::from(item.y);
    let frequency = u64::from(drdata(item, 4));

    ItemDriverOutcome::FireballMachineProjectile {
        item_id: item.id,
        character_id: character.id,
        start_x: clamp_legacy_coordinate(item_x + i32::from(dxs)),
        start_y: clamp_legacy_coordinate(item_y + i32::from(dys)),
        target_x: clamp_legacy_coordinate(item_x + i32::from(dx)),
        target_y: clamp_legacy_coordinate(item_y + i32::from(dy)),
        power: drdata(item, 3),
        schedule_after_ticks: (context.timer_call && frequency != 0).then_some(frequency),
    }
}
