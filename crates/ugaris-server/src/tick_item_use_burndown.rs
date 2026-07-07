//! Completed-action-outcome handling: the two-city burndown barrel
//! (`Burndown*`) family of `ItemDriverOutcome` variants (area 17's
//! `twocity.c` heat-barrel puzzle: too-hot/already-burned blocks, the
//! touch flavor line, ignition marking the burndown-kill PPD flag, and
//! the timer tick that cools it back down). Split out of the giant
//! `match outcome { ... }` block that still lives inline in `main.rs`'s
//! `tick.tick()` arm (P0.5 "Finish main() phase decomposition" -
//! REMAINING note: the completed-action-outcome handling needs
//! splitting by completed-action-kind family across several files, not
//! just relocation, because the whole match is too large to move
//! verbatim into one file). Warp, chests, dungeon, ice/palace, Teufel,
//! skel-raise, edemon/fdemon, transport, clan/lq/arena, and shrines were
//! sliced first; this is the eleventh family slice. The rest of the
//! match (xmas, swamp, key-assembly, ...) is still inline in `main.rs`
//! pending further slices.

use super::*;

pub(crate) fn dispatch_burndown_outcome(
    runtime: &mut ServerRuntime,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::BurndownTooHot { character_id, .. } => {
            feedback.push((character_id, "It is too hot to touch.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::BurndownAlreadyBurned {
            character_id, ..
        } => {
            feedback.push((character_id, "It was burned down already.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::BurndownTouch { character_id, .. } => {
            feedback.push((character_id, "You touch the barrel.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::BurndownIgnite { character_id, .. } => {
            if let Some(player) = runtime.player_for_character_mut(character_id) {
                player.mark_twocity_burndown_kill();
            }
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::BurndownTimerTick { .. } => {
            *executed += 1;
        }
        _ => {}
    }
}
