//! Completed-action-outcome handling: the Teufel-area (`src/area/34/
//! teufel.c`) family of `ItemDriverOutcome` variants (arena entry/exit,
//! rat/gambler door checks, rat-nest spawn/destroy). Split out of the
//! giant `match outcome { ... }` block that still lives inline in
//! `main.rs`'s `tick.tick()` arm (P0.5 "Finish main() phase
//! decomposition" - REMAINING note: the completed-action-outcome
//! handling needs splitting by completed-action-kind family across
//! several files, not just relocation, because the whole match is too
//! large to move verbatim into one file). Warp, chests, dungeon, and
//! ice/palace were sliced first; this is the fifth family slice. The
//! rest of the match (skel-raise, transport, clan-spawn, lq, arena,
//! shrines, xmas, swamp, edemon/fdemon, burndown, key-assembly, ...) is
//! still inline in `main.rs` pending further slices.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_teufel_outcome(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
    failed: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::TeufelArena { .. }
        | ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaExit { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaNeedsSuit {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "You need to wear an earth demon suit.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaLevelTooHigh {
            character_id,
            ..
        } => {
            feedback.push((character_id, "Max Level 38, sorry.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaEquipmentEnhanced { .. }
        | ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaEquipmentBound { .. } => {
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaBusy { character_id, .. } => {
            feedback.push((
                character_id,
                "Please try again soon. Target is busy.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::TeufelArenaExitLowHealth {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "You cannot leave with less than full health.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::TeufelDoor { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::TeufelDoorNoHumans {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "A demon looks through the view-hole in the door and shouts: \"No humans allowed!\"".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::TeufelDoorNoBeggars {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "A demon looks through the view-hole in the door and shouts: \"No beggars allowed!\"".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::TeufelDoorOnlyNobles {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "A demon looks through the view-hole in the door and shouts: \"Only nobles allowed!\"".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::TeufelDoorBusy { character_id, .. } => {
            feedback.push((
                character_id,
                "Please try again soon. Target is busy.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::TeufelDoorBug {
            character_id, x, y, ..
        } => {
            feedback.push((
                character_id,
                format!("You touch a teleport object but nothing happens - BUG ({x},{y})."),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::TeufelRatNestSpawn {
            item_id,
            level,
            template,
            schedule_after_ticks,
            ..
        } => {
            world.schedule_item_driver_timer(item_id, CharacterId(0), schedule_after_ticks);
            if spawn_teufel_ratnest_character(world, zone_loader, runtime, item_id, level, template)
            {
                *executed += 1;
            } else {
                *failed += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::TeufelRatNestDestroyed {
            character_id,
            ..
        } => {
            feedback.push((character_id, "You destroy the rat nest.".to_string()));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::TeufelRatNestGuarded {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "You need a moment of peace to destroy the nest. There is still a guard left, distracting you.".to_string(),
            ));
            *blocked += 1;
        }
        _ => {}
    }
}
