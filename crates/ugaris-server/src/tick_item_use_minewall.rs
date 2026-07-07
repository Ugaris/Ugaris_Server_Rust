//! Completed-action-outcome handling: the mine-wall digging (`MineWall*`)
//! family of `ItemDriverOutcome` variants (area 12's `src/area/12/mine.c`
//! diggable-wall mechanic: wall init, dig attempt, cursor-occupied/
//! exhausted blocks, and the post-collapse respawn timer). Split out of
//! the giant `match outcome { ... }` block that still lives inline in
//! `main.rs`'s `tick.tick()` arm (P0.5 "Finish main() phase decomposition"
//! - REMAINING note: the completed-action-outcome handling needs
//! splitting by completed-action-kind family across several files, not
//! just relocation, because the whole match is too large to move
//! verbatim into one file). Warp, chests, dungeon, ice/palace, Teufel,
//! skel-raise, Edemon/Fdemon, transport, clan-spawn/LQ/arena, shrines,
//! burndown, xmas/swamp, Caligar, key-assembly, and labyrinth were sliced
//! first; this is the sixteenth family slice. The rest of the match (the
//! large no-op catch-all and remaining scattered variants) is still
//! inline in `main.rs` pending further slices.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_minewall_outcome(
    world: &mut World,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
    deferred_templates: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::MineWallInitialized { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::MineWallDig {
            character_id,
            endurance_delta,
            opened,
            ..
        } => {
            if let Some(character) = world.characters.get_mut(&character_id) {
                character.endurance = character.endurance.saturating_add(endurance_delta);
            }
            if opened {
                *deferred_templates += 1;
            } else {
                *executed += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::MineWallCursorOccupied {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "Please empty your hand (mouse cursor) first.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::MineWallExhausted { character_id, .. } => {
            feedback.push((
                character_id,
                "You're too exhausted to continue digging.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::MineWallCollapse {
            item_id,
            schedule_after_ticks,
        } => {
            world.schedule_item_driver_timer(
                item_id,
                CharacterId(0),
                u64::from(schedule_after_ticks),
            );
            *executed += 1;
        }
        _ => {}
    }
}
