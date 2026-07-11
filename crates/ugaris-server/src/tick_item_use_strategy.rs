//! Completed-action-outcome handling: the `IDR_STR_MINE`/`IDR_STR_STORAGE`/
//! `IDR_STR_DEPOT` player-"look" family of `ItemDriverOutcome` variants
//! (`src/area/23_24/strategy.c`'s `mine`/`storage`/`depot`, `CF_PLAYER`
//! branch only - see `ugaris_core::item_driver::area23_24`'s module doc
//! comment for the two documented gaps every one of these three still
//! has). Split out of the giant `match outcome { ... }` block that still
//! lives inline in `main.rs`'s `tick.tick()` arm (P0.5 "Finish main()
//! phase decomposition"), same precedent as every other
//! `tick_item_use_*` family dispatcher.

use super::*;

pub(crate) fn dispatch_strategy_outcome(
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::StrMineLook {
            character_id,
            platinum,
            ..
        } => {
            feedback.push((
                character_id,
                format!("There are {platinum} units of Platinum left."),
            ));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::StrDepotLook {
            character_id,
            platinum,
            ..
        } => {
            feedback.push((
                character_id,
                format!("This depot contains {platinum} units of Platinum."),
            ));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::StrStorageInteract {
            character_id,
            conversion,
            platinum,
            ..
        } => {
            match conversion {
                ugaris_core::item_driver::StrStorageConversion::Converted { added, .. } => {
                    feedback.push((
                        character_id,
                        format!("Converted to {added} units of Platinum and added to storage."),
                    ));
                }
                ugaris_core::item_driver::StrStorageConversion::WrongKind => {
                    feedback.push((
                        character_id,
                        "You can only add mined gold or silver. The exchange rate is 5 to 1 \
                         for gold and 50 to 1 for silver."
                            .to_string(),
                    ));
                }
                ugaris_core::item_driver::StrStorageConversion::None => {}
            }
            feedback.push((
                character_id,
                format!("This storage contains {platinum} units of Platinum."),
            ));
            *executed += 1;
        }
        _ => {}
    }
}
