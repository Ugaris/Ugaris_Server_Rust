use super::*;

pub(crate) fn freakdoor_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    ItemDriverOutcome::FreakDoorUse {
        item_id: item.id,
        character_id: character.id,
        link_group: drdata(item, 8),
        one_way: drdata(item, 14) != 0,
        recursion_guard: drdata(item, 9) != 0,
        cached_partner_id: match drdata_u32(item, 10) {
            0 => None,
            id => Some(ItemId(id)),
        },
        no_target: drdata(item, 15) != 0,
    }
}

pub(crate) fn is_ice_shared_area(area_id: u16) -> bool {
    matches!(area_id, 10 | 11)
}

pub(crate) fn itemspawn_template(kind: u8) -> Option<&'static str> {
    match kind {
        0 => Some("melting_key"),
        1 => Some("ice_boots1"),
        2 => Some("ice_cape1"),
        3 => Some("ice_belt1"),
        4 => Some("ice_ring1"),
        5 => Some("ice_amulet1"),
        6 => Some("melting_key2"),
        7 => Some("ice_boots2"),
        8 => Some("ice_cape2"),
        9 => Some("ice_belt2"),
        10 => Some("ice_ring2"),
        11 => Some("ice_amulet2"),
        12 => Some("ice_boots3"),
        13 => Some("ice_cape3"),
        14 => Some("ice_belt3"),
        15 => Some("ice_ring3"),
        16 => Some("ice_amulet3"),
        17 => Some("palace_bomb"),
        18 => Some("palace_cap"),
        _ => None,
    }
}

pub(crate) fn itemspawn_driver(
    character: &Character,
    item: &Item,
    area_id: u16,
) -> ItemDriverOutcome {
    if character.id.0 == 0 || !is_ice_shared_area(area_id) {
        return ItemDriverOutcome::Noop;
    }
    if character.cursor_item.is_some() {
        return ItemDriverOutcome::IceItemSpawnCursorOccupied {
            item_id: item.id,
            character_id: character.id,
        };
    }
    let kind = drdata(item, 0);
    let Some(template) = itemspawn_template(kind) else {
        return ItemDriverOutcome::IceItemSpawnBug {
            item_id: item.id,
            character_id: character.id,
            kind,
        };
    };
    ItemDriverOutcome::IceItemSpawn {
        item_id: item.id,
        character_id: character.id,
        template,
    }
}

pub(crate) fn warmfire_driver(
    character: &Character,
    item: &Item,
    area_id: u16,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 || !is_ice_shared_area(area_id) {
        return ItemDriverOutcome::Noop;
    }
    let create_scroll = drdata(item, 0) == 0;
    if create_scroll && character.cursor_item.is_some() {
        return ItemDriverOutcome::WarmFireCursorOccupied {
            item_id: item.id,
            character_id: character.id,
        };
    }
    ItemDriverOutcome::WarmFire {
        item_id: item.id,
        character_id: character.id,
        create_scroll,
        removed_curse: context.has_curse_spell,
    }
}

pub(crate) fn backtofire_driver(
    character: &Character,
    item: &Item,
    area_id: u16,
) -> ItemDriverOutcome {
    if character.id.0 == 0 || !is_ice_shared_area(area_id) || item.carried_by != Some(character.id)
    {
        return ItemDriverOutcome::Noop;
    }
    ItemDriverOutcome::BackToFire {
        item_id: item.id,
        character_id: character.id,
        x: u16::from(drdata(item, 0)),
        y: u16::from(drdata(item, 1)),
    }
}

pub(crate) fn meltingkey_driver(
    character: &Character,
    item: &mut Item,
    area_id: u16,
) -> ItemDriverOutcome {
    if character.id.0 != 0 || !is_ice_shared_area(area_id) || item.carried_by.is_none() {
        return ItemDriverOutcome::Noop;
    }
    let limit = drdata(item, 0);
    let next_age = drdata(item, 1).wrapping_add(1);
    set_drdata(item, 1, next_age);
    if next_age >= limit {
        return ItemDriverOutcome::MeltingKeyTick {
            item_id: item.id,
            character_id: item.carried_by.unwrap_or(CharacterId(0)),
            melted: true,
            started_melting: false,
            schedule_after_ticks: None,
        };
    }

    let old_sprite = item.sprite;
    let sprite = 50494 + i32::from(next_age) * 5 / i32::from(limit.max(1));
    item.sprite = sprite;
    ItemDriverOutcome::MeltingKeyTick {
        item_id: item.id,
        character_id: item.carried_by.unwrap_or(CharacterId(0)),
        melted: false,
        started_melting: old_sprite != sprite && sprite == 50495,
        schedule_after_ticks: Some(TICKS_PER_SECOND * 10),
    }
}
