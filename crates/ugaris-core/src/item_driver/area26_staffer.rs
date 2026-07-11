use super::*;

/// C `vault_shelf`'s `it[in].drdata[1]` reward selector
/// (`src/area/26/staffer.c:355-368`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VaultShelfFind {
    /// `drdata[1] == 2`: `create_item("vault_ritual")` (`IID_MAX_RITUAL`).
    Ritual,
    /// `drdata[1] == 1`: `create_item("vault_journal")`
    /// (`IID_MAX_CHRONICLES`).
    Journal,
    /// Any other `drdata[1]` value: "nothing of interest".
    Nothing,
}

pub(crate) fn staffer_driver(
    character: &Character,
    item: &mut Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    match drdata(item, 0) {
        1 => staffer_spiketrap_driver(character, item),
        2 => staffer_fireball_machine_driver(character, item, context),
        3 => staffer_block_driver(character, item),
        4 => vault_skull_driver(character, item, context),
        5 => vault_shelf_driver(character, item),
        _ => ItemDriverOutcome::Unsupported {
            driver: IDR_STAFFER,
            item_id: item.id,
            character_id: character.id,
        },
    }
}

/// C `vault_skull` (`src/area/26/staffer.c:327-345`). `cn == 0` (item
/// timer call) is a no-op in C (the `if (!cn) return;` guard) - the
/// caller never routes timer calls here (`vault_skulls` has no `IF_LOOP`
/// flag in the zone data), but the guard is kept for parity.
pub(crate) fn vault_skull_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    match context.rouven_state {
        Some(0..=5) => ItemDriverOutcome::VaultSkullOpened {
            item_id: item.id,
            character_id: character.id,
        },
        _ => ItemDriverOutcome::Noop,
    }
}

/// C `vault_shelf` (`src/area/26/staffer.c:348-372`).
pub(crate) fn vault_shelf_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    let find = match drdata(item, 1) {
        2 => VaultShelfFind::Ritual,
        1 => VaultShelfFind::Journal,
        _ => VaultShelfFind::Nothing,
    };
    ItemDriverOutcome::VaultShelfSearch {
        item_id: item.id,
        character_id: character.id,
        find,
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
