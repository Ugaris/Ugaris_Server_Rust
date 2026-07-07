//! Completed-action-outcome handling: the skeleton-raising (`SkelRaise*`)
//! family of `ItemDriverOutcome` variants (area 1's chair/blood-raise
//! sequence: dust crumble, touch, raise-from-blood, and the raise timer's
//! own tick). Split out of the giant `match outcome { ... }` block that
//! still lives inline in `main.rs`'s `tick.tick()` arm (P0.5 "Finish
//! main() phase decomposition" - REMAINING note: the completed-action-
//! outcome handling needs splitting by completed-action-kind family
//! across several files, not just relocation, because the whole match is
//! too large to move verbatim into one file). Warp, chests, dungeon, ice/
//! palace, and Teufel were sliced first; this is the sixth family slice.
//! The rest of the match (transport, clan-spawn, lq, arena, shrines,
//! xmas, swamp, edemon/fdemon, burndown, key-assembly, ...) is still
//! inline in `main.rs` pending further slices.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_skelraise_outcome(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    failed: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::SkelRaiseDust {
            item_id,
            character_id,
        } => {
            world.apply_skelraise_dust(item_id);
            feedback.push((
                character_id,
                "The skeleton crumbles to dust as you touch it.".to_string(),
            ));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::SkelRaiseTouch { character_id, .. } => {
            feedback.push((character_id, "You touch the chair.".to_string()));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::SkelRaiseRaise {
            item_id,
            character_id,
            cursor_item_id,
            template,
        } => {
            if raise_skeleton_from_template(
                world,
                zone_loader,
                runtime,
                item_id,
                character_id,
                cursor_item_id,
                template,
            ) {
                feedback.push((
                    character_id,
                    "The skeleton comes to life as you pour the blood over it.".to_string(),
                ));
                *executed += 1;
            } else {
                *failed += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::SkelRaiseTimer { .. } => {
            *executed += 1;
        }
        _ => {}
    }
}
