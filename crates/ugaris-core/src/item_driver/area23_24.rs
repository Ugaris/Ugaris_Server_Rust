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
//! stack into Platinum before reporting the new total. The NPC-worker
//! branches (`!(ch[cn].flags & CF_PLAYER)`) of all three - a
//! `CDR_STRATEGY` worker's `OR_MINE`/`OR_TRANSFER`/`OR_TAKE`/`OR_TRAIN`
//! orders mining/depositing/withdrawing/claiming Platinum, see
//! `world::npc::area23_24::worker`'s module doc comment - are now also
//! ported, dispatched the same way (`ItemDriverOutcome::StrMineWorkerDig`/
//! `StrBuildingWorkerTransfer`/`StrDepotWorkerTakeover`, applied by
//! `World::apply_item_driver_outcome`).
//!
//! Still a documented gap, both gated on a different missing piece (the
//! per-area "always-on ambient timer" bootstrap, unrelated to the worker
//! driver): the `cn == 0` ambient branches - `mine`/`depot` just
//! re-format the item's display name (cosmetic only), while `storage`
//! grows its Platinum total by a fixed income once per 10 ticks and
//! reschedules itself (`strategy.c:1154-1159`) - same class of gap as
//! `IDR_LAB5_ITEM`'s fireface/lightface branches (see `World::
//! schedule_existing_light_timers`'s own doc comment).
//!
//! `IDR_STR_SPAWNER`'s `ch[cn].flags & CF_PLAYER` branch
//! ([`str_spawner_driver`]) now triggers a real worker-recruit attempt -
//! see `ItemDriverOutcome::StrSpawnerUse`'s own doc comment for the full
//! `World`/`ugaris-server` split. The `cn == 0` ambient/AI-init branch and
//! the full `ai_main`/`ai_init` AI-opponent driver remain full no-ops (no
//! AI-opponent wiring exists yet).

use super::*;
use crate::world::{character_value_base, str_item_gold, str_item_owner};

/// C `spawner(int in, int cn)`'s `ch[cn].flags & CF_PLAYER` branch
/// trigger (`strategy.c:1355-1381`) - see `ItemDriverOutcome::
/// StrSpawnerUse`'s own doc comment for why all the actual business
/// logic (ownership/gold/eligibility checks, the fresh-character spawn)
/// lives in `World::try_dispatch_strategy_spawner_use`/`ugaris-server`
/// instead of here. The `cn == 0` ambient/AI-init branch (`:1298-1331`)
/// remains a documented gap (this module's own doc comment).
pub(crate) fn str_spawner_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 || !character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::Noop;
    }
    ItemDriverOutcome::StrSpawnerUse {
        item_id: item.id,
        character_id: character.id,
    }
}

pub(crate) fn str_ticker_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 != 0 {
        return ItemDriverOutcome::Noop;
    }

    ItemDriverOutcome::StrTicker {
        item_id: item.id,
        schedule_after_ticks: TICKS_PER_SECOND,
    }
}

/// C `mine` (`strategy.c:1122-1148`): the `ch[cn].flags & CF_PLAYER`
/// "look" branch, plus the `DRD_STRATEGYDRIVER` NPC-worker mining branch
/// (`:1135-1148`) - see this module's own doc comment for the remaining
/// `cn == 0` gap.
pub(crate) fn str_mine_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    if character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::StrMineLook {
            item_id: item.id,
            character_id: character.id,
            platinum: str_item_gold(item),
        };
    }

    let current = str_item_gold(item);
    let strength = character_value_base(character, CharacterValue::Strength).max(0) as u32;
    let mined = current.min(strength);
    if mined == 0 {
        return ItemDriverOutcome::Noop;
    }
    ItemDriverOutcome::StrMineWorkerDig {
        item_id: item.id,
        character_id: character.id,
        mined,
    }
}

/// C `depot` (`strategy.c:1206-1239`): the `ch[cn].flags & CF_PLAYER`
/// "look" branch, plus the `DRD_STRATEGYDRIVER` NPC-worker ownership-
/// takeover/transfer branches (`:1219-1238`) - see this module's own doc
/// comment for the remaining `cn == 0` gap.
pub(crate) fn str_depot_driver(character: &Character, item: &Item) -> ItemDriverOutcome {
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    if character.flags.contains(CharacterFlags::PLAYER) {
        return ItemDriverOutcome::StrDepotLook {
            item_id: item.id,
            character_id: character.id,
            platinum: str_item_gold(item),
        };
    }

    let owner = str_item_owner(item);
    if owner != u32::from(character.group) {
        return ItemDriverOutcome::StrDepotWorkerTakeover {
            item_id: item.id,
            character_id: character.id,
            owner: u32::from(character.group),
        };
    }
    str_building_worker_transfer(character, item)
}

/// C `storage`'s (`:1196-1203`) and `depot`'s (`:1231-1238`) shared
/// NPC-worker transfer body - byte-for-byte identical once `depot`'s own
/// ownership check has already passed.
fn str_building_worker_transfer(character: &Character, item: &Item) -> ItemDriverOutcome {
    let platin = match character.driver_state.as_ref() {
        Some(crate::character_driver::CharacterDriverState::StrategyWorker(data)) => data.platin,
        _ => 0,
    };
    if platin > 0 {
        return ItemDriverOutcome::StrBuildingWorkerTransfer {
            item_id: item.id,
            character_id: character.id,
            deposited: platin as u32,
            withdrawn: 0,
        };
    }
    let current = str_item_gold(item);
    let strength = character_value_base(character, CharacterValue::Strength).max(0) as u32;
    let withdrawn = current.min(strength);
    if withdrawn == 0 {
        return ItemDriverOutcome::Noop;
    }
    ItemDriverOutcome::StrBuildingWorkerTransfer {
        item_id: item.id,
        character_id: character.id,
        deposited: 0,
        withdrawn,
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
    if character.id.0 == 0 {
        return ItemDriverOutcome::Noop;
    }
    if !character.flags.contains(CharacterFlags::PLAYER) {
        // C `storage`'s `DRD_STRATEGYDRIVER` NPC-worker branch
        // (`strategy.c:1191-1204`) - byte-for-byte identical to `depot`'s
        // own transfer body once its ownership check has passed (storage
        // has none).
        return str_building_worker_transfer(character, item);
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
