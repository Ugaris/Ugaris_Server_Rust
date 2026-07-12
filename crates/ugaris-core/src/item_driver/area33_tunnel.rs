//! Area 33 (`src/area/33/tunnel.c`) item drivers: the Long Tunnels exit
//! pillars (`IDR_TUNNELDOOR`).
//!
//! Only `tunneldoor`'s `DOOR_EXIT_EXP`/`DOOR_EXIT_MILITARY` branches
//! (`:630-636`, the "cash in a completed tunnel section for a reward"
//! pillar) are ported here - see [`ItemDriverOutcome::TunnelDoorExitReward`]
//! and its reward-math counterpart, `World::apply_tunnel_reward`
//! (`crate::world::tunnel`). The `DOOR_ENTRY`/`DOOR_CONTINUE` branches (the
//! procedural creeper-dungeon generator: `build_fighter`/
//! `handle_block_marker`/`handle_creeper_marker`/`find_unused_sector`) and
//! `IDR_TUNNELDOOR2` (`mean_door`) are a separate, not-yet-ported slice -
//! see `PORTING_TODO.md`'s Area 33 entry. Until that lands, a player has no
//! live-gameplay way to reach one of these exit doors; this slice ports
//! `give_reward`'s full reward/promotion math ahead of that wiring, tested
//! directly.

use super::*;

/// C `enum TunnelDoorType` (`src/area/33/tunnel.h:16`).
const DOOR_EXIT_EXP: u8 = 2;
const DOOR_EXIT_MILITARY: u8 = 3;

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
