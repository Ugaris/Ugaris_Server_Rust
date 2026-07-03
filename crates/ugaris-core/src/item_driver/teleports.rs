use super::*;

pub const LEGACY_TRANSPORT_POINT_COUNT: u8 = 26;

pub const LEGACY_TRANSPORT_CLAN_EXIT: u8 = 255;

pub(crate) fn transport_driver(character: &Character, item: &Item, spec: i32) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    if spec != 0 {
        return ItemDriverOutcome::TransportTravel {
            item_id: item.id,
            character_id: character.id,
            spec,
        };
    }

    let point = drdata(item, 0);
    if point != LEGACY_TRANSPORT_CLAN_EXIT && point >= LEGACY_TRANSPORT_POINT_COUNT {
        return ItemDriverOutcome::TransportInvalid {
            item_id: item.id,
            character_id: character.id,
            point,
        };
    }

    ItemDriverOutcome::TransportOpen {
        item_id: item.id,
        character_id: character.id,
        point,
    }
}

pub(crate) fn recall_driver(
    character: &Character,
    item: &Item,
    area_id: u16,
    in_arena: bool,
) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }
    if character.action == action::DIE {
        return ItemDriverOutcome::Noop;
    }
    if character.level > u32::from(drdata(item, 0)) {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if area_id == 34 && in_arena {
        return ItemDriverOutcome::BlockedByArea {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::Recall {
        item_id: item.id,
        character_id: character.id,
        x: character.rest_x,
        y: character.rest_y,
        area_id: character.rest_area,
    }
}

pub(crate) fn city_recall_driver(
    character: &Character,
    item: &Item,
    area_id: u16,
    in_arena: bool,
) -> ItemDriverOutcome {
    if item.carried_by != Some(character.id) {
        return ItemDriverOutcome::Noop;
    }
    if character.action == action::DIE {
        return ItemDriverOutcome::Noop;
    }
    if area_id == 34 && in_arena {
        return ItemDriverOutcome::BlockedByArea {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let Some((x, y, area_id)) = city_recall_destination(drdata(item, 0)) else {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    };

    ItemDriverOutcome::CityRecall {
        item_id: item.id,
        character_id: character.id,
        x,
        y,
        area_id,
    }
}

pub(crate) fn city_recall_destination(scroll_type: u8) -> Option<(u16, u16, u16)> {
    Some(match scroll_type {
        0 => (126, 179, 1),
        1 => (167, 188, 3),
        2 => (229, 94, 3),
        3 => (236, 176, 3),
        4 => (41, 250, 14),
        5 => (231, 242, 12),
        6 => (67, 108, 17),
        7 => (203, 227, 29),
        8 => (226, 164, 29),
        9 => (27, 14, 37),
        10 => (120, 120, 36),
        11 => (210, 247, 31),
        12 => (224, 248, 34),
        _ => return None,
    })
}

pub(crate) fn teleport_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    let target_x = drdata_u16(item, 0);
    let target_y = drdata_u16(item, 2);
    let target_area = drdata_u16(item, 4);
    let arch_only = drdata(item, 10) != 0;
    let brannington_arch_gate = drdata(item, 11) != 0;
    let stop_driver = drdata(item, 12) != 0;
    let quiet = drdata(item, 6) != 0;

    if brannington_arch_gate || (arch_only && !character.flags.contains(CharacterFlags::ARCH)) {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if item.max_level != 0 && character.level > u32::from(item.max_level) {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if character.level < u32::from(item.min_level) {
        return ItemDriverOutcome::BlockedByRequirements {
            item_id: item.id,
            character_id: character.id,
        };
    }
    if target_x < 1 || target_y < 1 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::Teleport {
        item_id: item.id,
        character_id: character.id,
        x: target_x,
        y: target_y,
        area_id: target_area,
        stop_driver,
        quiet,
    }
}
