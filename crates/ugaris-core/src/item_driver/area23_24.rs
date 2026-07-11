//! `IDR_STR_TICKER` (`src/area/23_24/strategy.c`'s `str_ticker`, driver
//! id 109) item-driver boundary: the zero-`cn` timer-call gate plus the
//! self-reschedule request, matching `lq_ticker_driver`'s shape exactly
//! (`item_driver/area20_lq.rs`). The actual per-tick mission-lifecycle
//! logic (`did_party_lose`/`remove_party`/`close_area`/`reward_winner`)
//! lives in `crate::world::strategy`'s `World::str_ticker`, called from
//! `World::apply_item_driver_outcome`'s `StrTicker` arm the same way
//! `LqTicker` calls `discover_lq_doors_once`/`queue_due_lq_npc_respawns`.

use super::*;

pub(crate) fn str_ticker_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 != 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::StrTicker {
        item_id: item.id,
        schedule_after_ticks: TICKS_PER_SECOND,
    }
}
