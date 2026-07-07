//! Completed-action-outcome handling: the xmas (`Xmas*`) and swamp-spawn
//! (`Swamp*`) family of `ItemDriverOutcome` variants (the yearly xmas-pop
//! maker, the xmas tree gift roll, and area 15's swamp-spawn timer/pulse
//! item drivers). Split out of the giant `match outcome { ... }` block
//! that still lives inline in `main.rs`'s `tick.tick()` arm (P0.5 "Finish
//! main() phase decomposition" - REMAINING note: the completed-action-
//! outcome handling needs splitting by completed-action-kind family
//! across several files, not just relocation, because the whole match is
//! too large to move verbatim into one file). Warp, chests, dungeon,
//! ice/palace, Teufel, skel-raise, edemon/fdemon, transport, clan/lq/
//! arena, shrines, and burndown were sliced first; this is the twelfth
//! family slice. The rest of the match (key-assembly, ...) is still
//! inline in `main.rs` pending further slices.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_xmas_swamp_outcome(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    area_id: u16,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
    failed: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::XmasMaker { character_id, .. } => {
            if apply_xmasmaker(world, zone_loader, character_id) {
                *executed += 1;
            } else {
                *failed += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::SwampSpawn {
            item_id,
            character_id: _,
            template,
            x,
            y,
            ..
        } => {
            if spawn_swampspawn_character(world, zone_loader, runtime, item_id, template, x, y) {
                *executed += 1;
            } else {
                *failed += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::SwampSpawnPulse { .. } => {}
        ugaris_core::item_driver::ItemDriverOutcome::XmasTree { character_id, .. } => {
            let (is_xmas, event_year) = runtime_effective_xmas_event(runtime);
            let gift_seed = world.tick.0;
            let result = match runtime.player_for_character_mut(character_id) {
                Some(player) => apply_xmastree(
                    world,
                    zone_loader,
                    player,
                    character_id,
                    area_id,
                    is_xmas,
                    event_year,
                    gift_seed,
                ),
                None => XmasTreeApplyResult::MissingPlayer,
            };
            match result {
                XmasTreeApplyResult::Dormant => {
                    feedback.push((
                        character_id,
                        "The tree seems dormant outside the holiday season.".to_string(),
                    ));
                    *blocked += 1;
                }
                XmasTreeApplyResult::AlreadyGranted => {
                    feedback.push((
                        character_id,
                        "The tree's magic has already granted you a gift.".to_string(),
                    ));
                    *blocked += 1;
                }
                XmasTreeApplyResult::NeedsHolidayTreat => {
                    feedback.push((
                        character_id,
                        "The tree awaits a special holiday treat before bestowing its gift."
                            .to_string(),
                    ));
                    *blocked += 1;
                }
                XmasTreeApplyResult::GiftGranted(item_name) => {
                    feedback.push((
                        character_id,
                        format!("The tree glows brightly as you receive a {item_name}!"),
                    ));
                    *executed += 1;
                }
                XmasTreeApplyResult::NoSpace => {
                    feedback.push((
                        character_id,
                        "You need more space in your inventory for the gift!".to_string(),
                    ));
                    *blocked += 1;
                }
                XmasTreeApplyResult::MissingPlayer => {
                    *failed += 1;
                }
            }
        }
        _ => {}
    }
}
