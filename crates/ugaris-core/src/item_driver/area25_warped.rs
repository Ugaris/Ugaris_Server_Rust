use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarpTrialDoorContext {
    pub xs: u16,
    pub ys: u16,
    pub xe: u16,
    pub ye: u16,
    pub partner_x: u16,
    pub partner_y: u16,
    pub room_has_non_simple_baddy: bool,
}

pub(crate) fn warpteleport_keyed_destination(
    portal_kind: u8,
    sphere_kind: u8,
) -> Option<(u16, u16)> {
    const TARGETS: [(u16, u16); 25] = [
        (247, 243),
        (226, 91),
        (215, 41),
        (197, 41),
        (191, 41),
        (247, 243),
        (179, 41),
        (251, 41),
        (173, 41),
        (161, 41),
        (247, 243),
        (111, 48),
        (161, 49),
        (207, 7),
        (206, 250),
        (247, 243),
        (207, 227),
        (201, 149),
        (176, 250),
        (167, 192),
        (247, 243),
        (169, 251),
        (145, 251),
        (127, 251),
        (151, 251),
    ];
    if !(1..=5).contains(&portal_kind) || !(1..=5).contains(&sphere_kind) {
        return None;
    }
    let index = usize::from(portal_kind - 1) * 5 + usize::from(sphere_kind - 1);
    Some(TARGETS[index])
}

pub(crate) fn warpteleport_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    if drdata(item, 0) != 0 {
        let Some(cursor_item_id) = character.cursor_item else {
            return ItemDriverOutcome::WarpTeleportMissingSphere {
                item_id: item.id,
                character_id: character.id,
            };
        };
        if context.cursor_template_id != Some(IID_AREA25_TELEKEY) {
            return ItemDriverOutcome::WarpTeleportMissingSphere {
                item_id: item.id,
                character_id: character.id,
            };
        }
        let Some((x, y)) =
            warpteleport_keyed_destination(drdata(item, 0), context.cursor_drdata0.unwrap_or(0))
        else {
            return ItemDriverOutcome::Unsupported {
                driver: IDR_WARPTELEPORT,
                item_id: item.id,
                character_id: character.id,
            };
        };
        return ItemDriverOutcome::WarpTeleportSpheres {
            item_id: item.id,
            character_id: character.id,
            cursor_item_id,
            x,
            y,
        };
    }

    let Some((x, y)) = (match drdata(item, 1) {
        1 => Some((242, 252)),
        2 => Some((247, 66)),
        3 => Some((251, 16)),
        4 => Some((152, 7)),
        5 => Some((183, 250)),
        _ => None,
    }) else {
        return ItemDriverOutcome::WarpTeleportBug {
            item_id: item.id,
            character_id: character.id,
        };
    };

    ItemDriverOutcome::Teleport {
        item_id: item.id,
        character_id: character.id,
        x,
        y,
        area_id: 25,
        stop_driver: true,
        quiet: true,
    }
}

pub(crate) fn warpbonus_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
    area_id: u16,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let base = context.warp_bonus_base.unwrap_or(40);
    if base > 139 {
        return ItemDriverOutcome::WarpBonusFinished {
            item_id: item.id,
            character_id: character.id,
        };
    }

    if context
        .warp_bonus_used_at_base
        .is_some_and(|used| used >= base)
    {
        return ItemDriverOutcome::WarpBonusAlreadyUsed {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let needed_points = base / 4;
    let reward_sphere_kind = if context.warp_bonus_points + 1 >= needed_points {
        if context.cursor_template_id != Some(IID_AREA25_TELEKEY) {
            return ItemDriverOutcome::WarpBonusNeedsSphere {
                item_id: item.id,
                character_id: character.id,
            };
        }
        context.cursor_drdata0
    } else {
        None
    };

    let next_points = if context.warp_bonus_points + 1 >= needed_points {
        0
    } else {
        context.warp_bonus_points + 1
    };
    let advanced = next_points == 0;
    let reward_level = character.level.min((base * 80) / 100);

    ItemDriverOutcome::WarpBonus {
        item_id: item.id,
        character_id: character.id,
        location_id: u32::from(item.x) + (u32::from(item.y) << 8) + (u32::from(area_id) << 16),
        base,
        next_points,
        advanced,
        reward_sphere_kind,
        reward_level,
    }
}

pub(crate) fn warpkeyspawn_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    if character.cursor_item.is_some() {
        return ItemDriverOutcome::WarpKeySpawnCursorOccupied {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::WarpKeySpawn {
        item_id: item.id,
        character_id: character.id,
        sphere_kind: drdata(item, 0),
    }
}

pub(crate) fn warptrialdoor_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let Some(trial) = context.warp_trial_door else {
        return ItemDriverOutcome::WarpTrialDoorBug {
            item_id: item.id,
            character_id: character.id,
        };
    };

    if character.x >= trial.xs
        && character.x <= trial.xe
        && character.y >= trial.ys
        && character.y <= trial.ye
    {
        return ItemDriverOutcome::WarpTrialDoorWrongSide {
            item_id: item.id,
            character_id: character.id,
        };
    }

    if trial.room_has_non_simple_baddy {
        return ItemDriverOutcome::WarpTrialDoorBusy {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let dx = (i32::from(trial.partner_x) - i32::from(item.x)).signum();
    let dy = (i32::from(trial.partner_y) - i32::from(item.y)).signum();
    if (dx == 0 && dy == 0) || (dx != 0 && dy != 0) {
        return ItemDriverOutcome::WarpTrialDoorBug {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let Some(player_x) = i32::from(item.x)
        .checked_add(dx)
        .and_then(|x| u16::try_from(x).ok())
    else {
        return ItemDriverOutcome::WarpTrialDoorBug {
            item_id: item.id,
            character_id: character.id,
        };
    };
    let Some(player_y) = i32::from(item.y)
        .checked_add(dy)
        .and_then(|y| u16::try_from(y).ok())
    else {
        return ItemDriverOutcome::WarpTrialDoorBug {
            item_id: item.id,
            character_id: character.id,
        };
    };
    let Some(fighter_target_x) = i32::from(trial.partner_x)
        .checked_add(dx)
        .and_then(|x| u16::try_from(x).ok())
    else {
        return ItemDriverOutcome::WarpTrialDoorBug {
            item_id: item.id,
            character_id: character.id,
        };
    };
    let Some(fighter_target_y) = i32::from(trial.partner_y)
        .checked_add(dy)
        .and_then(|y| u16::try_from(y).ok())
    else {
        return ItemDriverOutcome::WarpTrialDoorBug {
            item_id: item.id,
            character_id: character.id,
        };
    };

    ItemDriverOutcome::WarpTrialDoor {
        item_id: item.id,
        character_id: character.id,
        spawn_x: (trial.xs + trial.xe) / 2,
        spawn_y: (trial.ys + trial.ye) / 2,
        player_x,
        player_y,
        fighter_target_x,
        fighter_target_y,
        xs: trial.xs,
        ys: trial.ys,
        xe: trial.xe,
        ye: trial.ye,
        template: "warped_fighter",
    }
}

pub(crate) fn warpkeydoor_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let dx = i32::from(item.x) - i32::from(character.x);
    let dy = i32::from(item.y) - i32::from(character.y);
    if dx == 0 && dy == 0 {
        return ItemDriverOutcome::WarpKeyDoorBug {
            item_id: item.id,
            character_id: character.id,
        };
    }

    let Some((key_item_id, key_name)) = context.area25_door_key.clone() else {
        return ItemDriverOutcome::WarpKeyDoorMissingKey {
            item_id: item.id,
            character_id: character.id,
        };
    };

    let target_x = i32::from(item.x) + dx;
    let target_y = i32::from(item.y) + dy;
    if !(0..=u16::MAX as i32).contains(&target_x) || !(0..=u16::MAX as i32).contains(&target_y) {
        return ItemDriverOutcome::WarpKeyDoorBug {
            item_id: item.id,
            character_id: character.id,
        };
    }

    ItemDriverOutcome::WarpKeyDoor {
        item_id: item.id,
        character_id: character.id,
        key_item_id,
        key_name: outcome_item_name(&key_name),
        x: target_x as u16,
        y: target_y as u16,
    }
}
