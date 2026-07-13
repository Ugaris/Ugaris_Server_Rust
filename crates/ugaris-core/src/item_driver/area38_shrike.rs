//! Area 38 (`src/area/38/shrike.c`) `IDR_SHRIKE = 119` "misc items"
//! driver: the amulet-assembly puzzle's six map fixtures (dead
//! tree/pedestal that hand out fresh amulet components at full night, the
//! silver-under-rock dig, the level-65-gated Moon door, the Pool of the
//! Moon talisman activation, and the sliding puzzle cube), dispatched by
//! `it[in].drdata[0]` exactly like C's `shrike_driver` (`shrike.c:356-
//! 377`). `IDR_SHRIKEAMULET = 118` (combining the three amulet
//! components on the cursor) is a separate driver, already ported at
//! `item_driver/assemble.rs`'s `shrike_amulet_driver`.
//!
//! Every branch here is pure (`Character`/`Item`/`ItemDriverContext` in,
//! `ItemDriverOutcome` out) - map mutation, teleporting, and the fresh-
//! item-on-cursor creation the tree/pedestal/rock branches need all live
//! in `World`/`ugaris-server` (see `world::shrike` and `ugaris-server`'s
//! `tick_item_use_shrike`, which need `ZoneLoader`/full map access this
//! module deliberately does not have).

use super::*;

/// C `shrike_driver`'s `drdata[0]` switch (`shrike.c:356-377`).
const SHRIKE_TREE: u8 = 1;
const SHRIKE_ROCK: u8 = 2;
const SHRIKE_DOOR: u8 = 3;
const SHRIKE_POOL: u8 = 4;
const SHRIKE_CUBE: u8 = 5;
const SHRIKE_PEDE: u8 = 6;

/// C `is_fullnight()`'s 15-minute cube auto-reset idle threshold
/// (`TICKS * 60 * 15`, `shrike.c:334`).
const SHRIKE_CUBE_RESET_IDLE_TICKS: u32 = (TICKS_PER_SECOND as u32) * 60 * 15;
/// C `cube_driver`'s timer reschedule interval (`TICKS * 5`,
/// `shrike.c:342`).
const SHRIKE_CUBE_TIMER_INTERVAL: u64 = TICKS_PER_SECOND * 5;
/// C `tree_driver`/`rock_driver`/`door_driver`/`pede_driver`'s ambient
/// reschedule interval (`TICKS * 60`, `shrike.c:100`/`:139`/`:182`/
/// `:246`).
const SHRIKE_AMBIENT_TIMER_INTERVAL: u64 = TICKS_PER_SECOND * 60;
/// `drdata` byte offsets `cube_driver` uses for its remembered last-
/// touch tick / origin tile (`shrike.c:283-341`), mirroring
/// `world::shrike`'s copy of the same constants (kept in sync manually,
/// same precedent as every other `item_driver`/`world` offset pair).
const SHRIKE_CUBE_LAST_TOUCH_OFFSET: usize = 4;
const SHRIKE_CUBE_ORIGIN_X_OFFSET: usize = 8;
const SHRIKE_CUBE_ORIGIN_Y_OFFSET: usize = 10;

/// C `shrike_driver` (`shrike.c:356-377`).
pub(crate) fn shrike_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    match drdata(item, 0) {
        SHRIKE_TREE => tree_driver(character, item, context, ShrikeAmbientKind::Tree),
        SHRIKE_ROCK => rock_driver(character, item, context),
        SHRIKE_DOOR => door_driver(character, item, context),
        SHRIKE_POOL => pool_driver(character, item, context),
        SHRIKE_CUBE => cube_driver(character, item, context),
        SHRIKE_PEDE => tree_driver(character, item, context, ShrikeAmbientKind::Pede),
        _ => ItemDriverOutcome::Unsupported {
            driver: IDR_SHRIKE,
            item_id: item.id,
            character_id: character.id,
        },
    }
}

/// C `tree_driver`/`pede_driver` (`shrike.c:83-124`/`:126-166`) - the two
/// functions are identical except for the sprite/description pair and
/// which amulet component they hand out, so this one function covers
/// both (`kind` selects `ShrikeAmbientKind::Tree`/`Pede`).
fn tree_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
    kind: ShrikeAmbientKind,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::ShrikeAmbientRefresh {
            item_id: item.id,
            x: item.x,
            y: item.y,
            kind,
            night: context.is_fullnight,
            schedule_after_ticks: SHRIKE_AMBIENT_TIMER_INTERVAL,
        };
    }

    if !context.is_fullnight {
        return ItemDriverOutcome::Noop;
    }
    if character.cursor_item.is_some() {
        return ItemDriverOutcome::ShrikeHandOccupied {
            item_id: item.id,
            character_id: character.id,
        };
    }
    let piece = match kind {
        ShrikeAmbientKind::Tree => ShrikeAmuletPiece::Chain,
        ShrikeAmbientKind::Pede => ShrikeAmuletPiece::Crystal,
        _ => unreachable!("tree_driver only ever called with Tree/Pede"),
    };
    ItemDriverOutcome::ShrikeGiveAmuletPiece {
        item_id: item.id,
        character_id: character.id,
        piece,
    }
}

/// C `rock_driver` (`shrike.c:169-222`).
fn rock_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::ShrikeAmbientRefresh {
            item_id: item.id,
            x: item.x,
            y: item.y,
            kind: ShrikeAmbientKind::Rock,
            night: context.is_fullnight,
            schedule_after_ticks: SHRIKE_AMBIENT_TIMER_INTERVAL,
        };
    }

    if !context.is_fullnight {
        return ItemDriverOutcome::Noop;
    }
    let Some(cursor_item_id) = character.cursor_item else {
        return ItemDriverOutcome::ShrikeRockNoTool {
            character_id: character.id,
        };
    };
    if context.cursor_driver != Some(IDR_FORESTSPADE) {
        return ItemDriverOutcome::ShrikeRockWrongTool {
            character_id: character.id,
        };
    }
    ItemDriverOutcome::ShrikeRockDigSuccess {
        item_id: item.id,
        character_id: character.id,
        cursor_item_id,
        piece: ShrikeAmuletPiece::Charm,
    }
}

/// C `door_driver` (`shrike.c:224-260`).
fn door_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::ShrikeAmbientRefresh {
            item_id: item.id,
            x: item.x,
            y: item.y,
            kind: ShrikeAmbientKind::Door,
            night: context.is_fullnight,
            schedule_after_ticks: SHRIKE_AMBIENT_TIMER_INTERVAL,
        };
    }

    if character.level < 65 {
        return ItemDriverOutcome::ShrikeDoorTooWeak {
            character_id: character.id,
        };
    }
    // C checks `strstr(it[in2].description, " of the Moon.")` on the
    // cursor item; the Talisman of the Moon (`pool_driver`'s own success
    // branch) is the only item that description is ever set to, so this
    // checks the equivalent, simpler `template_id` instead - see the
    // `ShrikeDoorNeedsTalisman` doc comment.
    if context.cursor_template_id != Some(IID_SHRIKE_TALISMAN) {
        return ItemDriverOutcome::ShrikeDoorNeedsTalisman {
            character_id: character.id,
        };
    }
    ItemDriverOutcome::ShrikeDoorEnter {
        character_id: character.id,
    }
}

/// C `pool_driver` (`shrike.c:262-281`). No ambient/automatic branch -
/// C's own `if (!cn) return;` is the very first line.
fn pool_driver(
    character: &Character,
    _item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let Some(cursor_item_id) = character.cursor_item else {
        return ItemDriverOutcome::ShrikePoolSweetWater {
            character_id: character.id,
        };
    };
    if context.cursor_driver != Some(IDR_SHRIKEAMULET)
        || context.cursor_drdata0 != Some(7)
        || !context.is_fullnight
    {
        return ItemDriverOutcome::ShrikePoolWetItem {
            character_id: character.id,
            cursor_item_id,
        };
    }
    ItemDriverOutcome::ShrikePoolTalismanCreated {
        character_id: character.id,
        cursor_item_id,
    }
}

/// C `cube_driver` (`shrike.c:283-343`).
fn cube_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 != 0 {
        let Some((to_x, to_y)) = context.shrike_cube_push_target else {
            return ItemDriverOutcome::ShrikeCubeBlocked {
                character_id: character.id,
            };
        };
        return ItemDriverOutcome::ShrikeCubePush {
            item_id: item.id,
            character_id: character.id,
            from_x: item.x,
            from_y: item.y,
            to_x,
            to_y,
        };
    }

    // C: `if (!(*(unsigned int *)(it[in].drdata + 8))) { remember coords
    // }` - `drdata + 8` is read as one `unsigned int`, so C's own check
    // is really "are *both* the x and y halves zero"; matched here via
    // the two `u16` halves directly.
    let origin_x = drdata_u16(item, SHRIKE_CUBE_ORIGIN_X_OFFSET);
    let origin_y = drdata_u16(item, SHRIKE_CUBE_ORIGIN_Y_OFFSET);
    let origin_unset = origin_x == 0 && origin_y == 0;
    let set_origin = origin_unset.then_some((item.x, item.y));
    let (effective_origin_x, effective_origin_y) = if origin_unset {
        (item.x, item.y)
    } else {
        (origin_x, origin_y)
    };

    let last_touch = drdata_u32(item, SHRIKE_CUBE_LAST_TOUCH_OFFSET);
    let moved_from_origin = effective_origin_x != item.x || effective_origin_y != item.y;
    let idle_long_enough = last_touch != 0
        && context.current_tick.saturating_sub(last_touch) > SHRIKE_CUBE_RESET_IDLE_TICKS;
    let reset_to =
        (idle_long_enough && moved_from_origin && context.shrike_cube_origin_clear == Some(true))
            .then_some((effective_origin_x, effective_origin_y));

    ItemDriverOutcome::ShrikeCubeAmbientTick {
        item_id: item.id,
        set_origin,
        reset_to,
        schedule_after_ticks: SHRIKE_CUBE_TIMER_INTERVAL,
    }
}
