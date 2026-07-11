//! `src/area/23_24/strategy.c`'s `it_driver` family (driver ids 105-109):
//! `IDR_STR_TICKER`'s zero-`cn` timer-call gate plus the self-reschedule
//! request, matching `lq_ticker_driver`'s shape exactly
//! (`item_driver/area20_lq.rs`) - the actual per-tick mission-lifecycle
//! logic (`did_party_lose`/`remove_party`/`close_area`/`reward_winner`)
//! lives in `crate::world::strategy`'s `World::str_ticker`, called from
//! `World::apply_item_driver_outcome`'s `StrTicker` arm the same way
//! `LqTicker` calls `discover_lq_doors_once`/`queue_due_lq_npc_respawns`.
//!
//! `IDR_STR_MINE`/`IDR_STR_STORAGE`/`IDR_STR_DEPOT`'s player-facing "look"
//! branches (`mine`/`storage`/`depot`, `strategy.c:1122-1241`) are also
//! ported here: reading the current Platinum total (mine/depot) and,
//! for storage, converting a carried `IDR_ENHANCE` mined gold/silver
//! stack into Platinum before reporting the new total. Two halves of
//! each of these three functions remain unported, both gated on the same
//! missing piece - C's `strategy_driver` (`DRD_STRATEGYDRIVER`, the
//! recruitable-worker NPC AI, `strategy.c:713-1120`) has no Rust
//! character-driver counterpart yet (no `CDR_*` id reserved, no
//! `CharacterDriverState` variant):
//!
//! - the `cn == 0` ambient branches: `mine`/`depot` just re-format the
//!   item's display name (cosmetic only), while `storage` grows its
//!   Platinum total by a fixed income once per 10 ticks and reschedules
//!   itself (`strategy.c:1154-1159`) - the same "needs a first-timer-call
//!   bootstrap" gap `IDR_LAB5_ITEM`'s fireface/lightface branches hit
//!   (see `World::schedule_existing_light_timers`'s own doc comment),
//!   deliberately deferred alongside the worker driver since both need
//!   the same kind of always-on scheduling wiring;
//! - the NPC-worker branches (`!(ch[cn].flags & CF_PLAYER)`): a worker
//!   ordered `OR_MINE`/`OR_TRANSFER`/`OR_TAKE`/`OR_TRAIN` carries Platinum
//!   (`struct strategy_data.platin`) between these three building types -
//!   entirely unreachable without `strategy_driver` spawning a worker in
//!   the first place, so there is nothing to port here yet.
//!
//! `IDR_STR_SPAWNER`'s `spawner`/`spawner_sub` (recruit-an-NPC-worker) and
//! the `ai_main`/`ai_init` AI-opponent driver remain full no-ops for the
//! same underlying reason.

use super::*;
use crate::world::str_item_gold;

pub(crate) fn str_ticker_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 != 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::StrTicker {
        item_id: item.id,
        schedule_after_ticks: TICKS_PER_SECOND,
    }
}

/// C `mine` (`strategy.c:1122-1148`)'s `ch[cn].flags & CF_PLAYER` branch
/// only - see this module's own doc comment for the two documented gaps.
pub(crate) fn str_mine_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 || !character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }
    ItemDriverOutcome::StrMineLook {
        item_id: item.id,
        character_id: character.id,
        platinum: str_item_gold(item),
    }
}

/// C `depot` (`strategy.c:1208-1241`)'s `ch[cn].flags & CF_PLAYER` branch
/// only - see this module's own doc comment for the two documented gaps.
pub(crate) fn str_depot_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 || !character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }
    ItemDriverOutcome::StrDepotLook {
        item_id: item.id,
        character_id: character.id,
        platinum: str_item_gold(item),
    }
}

/// C `storage` (`strategy.c:1150-1206`)'s `ch[cn].flags & CF_PLAYER`
/// branch only - see this module's own doc comment for the two documented
/// gaps. `context.cursor_driver`/`cursor_drdata0`/`cursor_drdata1_u32`
/// mirror C's `(in2 = ch[cn].citem) && it[in2].driver == IDR_ENHANCE`
/// check plus its `it[in2].drdata[0]`/`*(unsigned int*)(it[in2].drdata+1)`
/// reads - both already populated generically for any driver whenever the
/// character carries a cursor item (`World::
/// execute_item_driver_request_with_context`'s `cursor_context`).
pub(crate) fn str_storage_driver(
    character: &Character,
    item: &Item,
    context: &ItemDriverContext,
) -> ItemDriverOutcome {
    if character.id.0 == 0 || !character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }

    let current = str_item_gold(item);
    let conversion = match (character.cursor_item, context.cursor_driver) {
        (Some(cursor_item_id), Some(driver)) if driver == IDR_ENHANCE => {
            let amount = context.cursor_drdata1_u32.unwrap_or(0);
            let added = match context.cursor_drdata0 {
                Some(1) => amount / 50,
                Some(2) => amount / 5,
                _ => 0,
            };
            if added != 0 {
                StrStorageConversion::Converted {
                    cursor_item_id,
                    added,
                }
            } else {
                StrStorageConversion::WrongKind
            }
        }
        _ => StrStorageConversion::None,
    };
    let platinum = match conversion {
        StrStorageConversion::Converted { added, .. } => current + added,
        StrStorageConversion::None | StrStorageConversion::WrongKind => current,
    };

    ItemDriverOutcome::StrStorageInteract {
        item_id: item.id,
        character_id: character.id,
        conversion,
        platinum,
    }
}
