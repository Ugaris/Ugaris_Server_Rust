//! Area 33 (`src/area/33/tunnel.c`) item drivers: the Long Tunnels exit
//! pillars (`IDR_TUNNELDOOR`) and the "mean door" (`IDR_TUNNELDOOR2`).
//!
//! `tunneldoor`'s `DOOR_EXIT_EXP`/`DOOR_EXIT_MILITARY` branches (`:630-636`,
//! the "cash in a completed tunnel section for a reward" pillar) are ported
//! here - see [`ItemDriverOutcome::TunnelDoorExitReward`] and its
//! reward-math counterpart, `World::apply_tunnel_reward`
//! (`crate::world::tunnel`). `mean_door` (`:736-748`, `IDR_TUNNELDOOR2`) is
//! also ported - see [`mean_door_driver`]. `tunneldoor`'s `DOOR_ENTRY`/
//! `DOOR_CONTINUE` branches (`:638-734`, the creeper-dungeon instance
//! scan/spawn) dispatch to [`ItemDriverOutcome::TunnelDoorEnter`] here -
//! both need `PlayerRuntime` (`gorwin_ppd`/`tunnel_ppd`) to compute the
//! target level before the actual map scan can run, so the map-mutation/
//! creeper-spawn-planning logic itself lives in `World::
//! plan_tunnel_entry` (`crate::world::tunnel`), driven from
//! `ugaris-server`'s `dispatch_tunnel_enter_outcome`.

use super::*;

/// C `enum TunnelDoorType` (`src/area/33/tunnel.h:16`).
const DOOR_ENTRY: u8 = 0;
const DOOR_CONTINUE: u8 = 1;
const DOOR_EXIT_EXP: u8 = 2;
const DOOR_EXIT_MILITARY: u8 = 3;

/// C `DOOR_CHECK_INTERVAL` (`src/area/33/tunnel.h:30`, `(TICKS)` - one
/// second).
const DOOR_CHECK_INTERVAL: u64 = TICKS_PER_SECOND;

/// C `tunneldoor` (`src/area/33/tunnel.c:603-734`)'s dispatch: which
/// `enum TunnelDoorType` this particular door column is (`it[in].
/// drdata[0]`) decides which outcome variant to hand back.
pub(crate) fn tunneldoor_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    // C `if (!cn) return; // automatic call` (`:608-610`).
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }

    let door_type = drdata(item, 0);
    match door_type {
        DOOR_ENTRY | DOOR_CONTINUE => ItemDriverOutcome::TunnelDoorEnter {
            item_id: item.id,
            character_id: character.id,
            door_type,
        },
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
