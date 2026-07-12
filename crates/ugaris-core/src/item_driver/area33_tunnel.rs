//! Area 33 (`src/area/33/tunnel.c`) item drivers: the Long Tunnels exit
//! pillars (`IDR_TUNNELDOOR`) and the "mean door" (`IDR_TUNNELDOOR2`).
//!
//! `tunneldoor`'s `DOOR_EXIT_EXP`/`DOOR_EXIT_MILITARY` branches (`:630-636`,
//! the "cash in a completed tunnel section for a reward" pillar) are ported
//! here - see [`ItemDriverOutcome::TunnelDoorExitReward`] and its
//! reward-math counterpart, `World::apply_tunnel_reward`
//! (`crate::world::tunnel`). `mean_door` (`:736-748`, `IDR_TUNNELDOOR2`) is
//! also ported - see [`mean_door_driver`]. The `DOOR_ENTRY`/`DOOR_CONTINUE`
//! branches of `tunneldoor` itself (the procedural creeper-dungeon
//! generator: `build_fighter`/`handle_block_marker`/`handle_creeper_marker`/
//! `find_unused_sector`) are a separate, not-yet-ported slice - see
//! `PORTING_TODO.md`'s Area 33 entry. Until that lands, a player has no
//! live-gameplay way to reach either of these doors; this slice ports
//! `give_reward`'s full reward/promotion math and `mean_door`'s periodic
//! area-clear/open-door logic ahead of that wiring, tested directly.

use super::*;

/// C `enum TunnelDoorType` (`src/area/33/tunnel.h:16`).
const DOOR_EXIT_EXP: u8 = 2;
const DOOR_EXIT_MILITARY: u8 = 3;

/// C `DOOR_CHECK_INTERVAL` (`src/area/33/tunnel.h:30`, `(TICKS)` - one
/// second).
const DOOR_CHECK_INTERVAL: u64 = TICKS_PER_SECOND;

/// C `tunneldoor` (`src/area/33/tunnel.c:603-734`), narrowed to the
/// `DOOR_EXIT_EXP`/`DOOR_EXIT_MILITARY` branches (`:630-636`). The
/// `DOOR_ENTRY`/`DOOR_CONTINUE` branches fall through to `Unsupported`
/// (documented gap, not a regression - this driver was entirely
/// undispatched before this port).
pub(crate) fn tunneldoor_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    // C `if (!cn) return; // automatic call` (`:608-610`).
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let door_type = drdata(item, 0);
    match door_type {
        DOOR_EXIT_EXP | DOOR_EXIT_MILITARY => ItemDriverOutcome::TunnelDoorExitReward {
            item_id: item.id,
            character_id: character.id,
            door_type,
        },
        _ => ItemDriverOutcome::Unsupported {
            driver: IDR_TUNNELDOOR,
            item_id: item.id,
            character_id: character.id,
        },
    }
}

/// C `mean_door` (`src/area/33/tunnel.c:736-748`, `IDR_TUNNELDOOR2`).
///
/// The automatic (`cn == 0`) branch always reschedules itself
/// (`DOOR_CHECK_INTERVAL`, unconditionally - C's own `call_item` call runs
/// before the `check_area_clear` check, not gated on its result) and opens
/// the door once [`ItemDriverContext::tunnel_door_area_clear`] (C's
/// `check_area_clear`) reports the room beyond it is free of non-player
/// characters. The player-interaction branch is C's own "not implemented"
/// flavor line - this door never reacts to being touched.
pub(crate) fn mean_door_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if context.timer_call || character.id.0 == 0 {
        return ItemDriverOutcome::TunnelDoorAreaCheck {
            item_id: item.id,
            x: item.x,
            y: item.y,
            opened: context.tunnel_door_area_clear.unwrap_or(false),
            schedule_after_ticks: DOOR_CHECK_INTERVAL,
        };
    }

    ItemDriverOutcome::TunnelDoorFlavor {
        item_id: item.id,
        character_id: character.id,
    }
}
