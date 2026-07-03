use super::*;

pub(crate) fn parkshrine_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let shrine = drdata(item, 0);
    if !(1..=3).contains(&shrine) {
        return ItemDriverOutcome::ParkShrineBug {
            item_id: item.id,
            character_id: character.id,
            shrine,
        };
    }

    ItemDriverOutcome::ParkShrine {
        item_id: item.id,
        character_id: character.id,
        shrine,
    }
}

pub(crate) fn fireball_machine_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    let dx = i16::from(drdata(item, 0)) - 128;
    let dy = i16::from(drdata(item, 1)) - 128;
    let dxs = dx.signum();
    let dys = dy.signum();
    let item_x = i32::from(item.x);
    let item_y = i32::from(item.y);
    let frequency = u64::from(drdata(item, 3));

    ItemDriverOutcome::FireballMachineProjectile {
        item_id: item.id,
        character_id: character.id,
        start_x: clamp_legacy_coordinate(item_x + i32::from(dxs)),
        start_y: clamp_legacy_coordinate(item_y + i32::from(dys)),
        target_x: clamp_legacy_coordinate(item_x + i32::from(dx)),
        target_y: clamp_legacy_coordinate(item_y + i32::from(dy)),
        power: drdata(item, 2),
        schedule_after_ticks: (context.timer_call && frequency != 0).then_some(frequency),
    }
}

pub(crate) fn flamethrow_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if !context.timer_call || character.id.0 != 0 {
        return ItemDriverOutcome::Noop;
    }

    let fire = drdata(item, 0);
    if fire != 0 {
        set_drdata(item, 0, fire.saturating_sub(1));
        if drdata(item, 2) == 0 {
            item.sprite += 1;
            set_drdata(item, 2, 1);
            item.modifier_index[4] = V_LIGHT;
            item.modifier_value[4] = 250;
        }
        return ItemDriverOutcome::FlameThrowerPulse {
            item_id: item.id,
            character_id: character.id,
            direction: drdata(item, 1),
            schedule_after_ticks: 1,
        };
    }

    item.sprite -= 1;
    set_drdata(item, 0, TICKS_PER_SECOND as u8);
    set_drdata(item, 2, 0);
    item.modifier_index[4] = 0;
    item.modifier_value[4] = 0;
    let delay_seconds = drdata(item, 3);

    ItemDriverOutcome::FlameThrowerExtinguished {
        item_id: item.id,
        character_id: character.id,
        schedule_after_ticks: (delay_seconds != 0)
            .then_some(TICKS_PER_SECOND.saturating_mul(u64::from(delay_seconds))),
    }
}

pub(crate) fn extinguish_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::Extinguish {
        item_id: item.id,
        character_id: character.id,
        extinguished: false,
    }
}

pub(crate) fn spiketrap_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call || character.id.0 == 0 {
        if drdata(item, 0) != 0 {
            item.sprite -= 1;
            set_drdata(item, 0, 0);
            return ItemDriverOutcome::SpikeTrapReset { item_id: item.id };
        }
        return ItemDriverOutcome::Noop;
    }

    if drdata(item, 0) != 0 {
        return ItemDriverOutcome::Noop;
    }

    item.sprite += 1;
    set_drdata(item, 0, 1);
    ItemDriverOutcome::SpikeTrapTriggered {
        item_id: item.id,
        character_id: character.id,
        damage: i32::from(drdata(item, 1)) * crate::entity::POWERSCALE,
        reset_after_ticks: TICKS_PER_SECOND,
    }
}

pub(crate) fn zombie_shrine_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let shrine_type = drdata(item, 0);
    let required_skull = match shrine_type {
        0 => IID_AREA2_ZOMBIESKULL1,
        1 => IID_AREA2_ZOMBIESKULL2,
        _ => IID_AREA2_ZOMBIESKULL3,
    };
    if context.cursor_template_id != Some(required_skull) {
        return ItemDriverOutcome::ZombieShrineNeedsOffering {
            item_id: item.id,
            character_id: character.id,
            shrine_type,
        };
    }

    ItemDriverOutcome::ZombieShrine {
        item_id: item.id,
        character_id: character.id,
        shrine_type,
    }
}
