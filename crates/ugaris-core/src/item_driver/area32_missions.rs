//! Area 32 governor job-board mission reward chest:
//! `src/area/32/missions.c::missionchest_driver` (`IDR_MISSIONCHEST`,
//! `:1790-1847`).
//!
//! The pure driver only gates `character.id != 0` (C `if (!cn) return;` -
//! `missionchest_driver` is never called for a `cn==0` timer tick, only a
//! real player use); every other check (key requirement, cursor-occupied,
//! the empty-chest `md->itemtemp == NULL` case, item creation, `ppd->
//! find_item[0]` bookkeeping, `mission_status`/`mission_done`) needs the
//! acting player's `governor: MissionPpd` and a `ZoneLoader`, so it is
//! fully deferred to `ugaris-server::area32::apply_mission_chest_open` -
//! same precedent as `chests::chest_driver`/`ChestTreasure`.

use super::*;

pub(crate) fn missionchest_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    ItemDriverOutcome::MissionChestOpen {
        item_id: item.id,
        character_id: character.id,
    }
}
