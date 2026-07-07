//! Completed-action-outcome handling: the Dungeon-area (`src/area/13/
//! dungeon.c`) family of `ItemDriverOutcome` variants (teleport/fake-
//! chest/key-spawn/clan-jewel-door). Split out of the giant
//! `match outcome { ... }` block that still lives inline in `main.rs`'s
//! `tick.tick()` arm (P0.5 "Finish main() phase decomposition" -
//! REMAINING note: the completed-action-outcome handling needs
//! splitting by completed-action-kind family across several files, not
//! just relocation, because the whole match is too large to move
//! verbatim into one file). Warp and chests were sliced first; this is
//! the third family slice. The rest of the match (teufel, edemon/
//! fdemon, shrines, ...) is still inline in `main.rs` pending further
//! slices.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn dispatch_dungeon_outcome(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
    failed: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::DungeonTeleport {
            item_id,
            character_id,
            x,
            y,
            ..
        } => {
            let teleported = world.teleport_character_same_area(character_id, x, y, false)
                || world.teleport_character_same_area(character_id, 240, 250, false)
                || world.teleport_character_same_area(character_id, 235, 250, false)
                || world.teleport_character_same_area(character_id, 230, 250, false);
            if teleported {
                world.destroy_item(item_id);
                *executed += 1;
            } else {
                *failed += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::DungeonFake { item_id, .. } => {
            if world.destroy_item(item_id) {
                *executed += 1;
            } else {
                *failed += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::DungeonKey {
            character_id,
            template,
            key_id,
            ..
        } => {
            match grant_template_item_to_cursor(world, zone_loader, character_id, template) {
                Some(_) => {
                    if let Some(cursor_item_id) = world
                        .characters
                        .get(&character_id)
                        .and_then(|character| character.cursor_item)
                    {
                        if let Some(cursor_item) = world.items.get_mut(&cursor_item_id) {
                            // C `dungeonkey` (`dungeon.c:1913-1937`) wraps the
                            // spawn's raw stored `keyid` into the real key's `ID`
                            // so it can later match a `dungeon_door`'s own wrapped
                            // `key1`/`key2` requirement (`dungeon.c:820,825`).
                            cursor_item.template_id =
                                dungeon::dungeon_key_item_id(template, key_id);
                        }
                    }
                    *executed += 1;
                }
                None => {
                    *failed += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::DungeonKeyCursorOccupied {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "Please empty your 'hand' (mouse cursor) first.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::DungeonDoorMissingKeys {
            character_id,
            missing,
            both_required,
            ..
        } => {
            if both_required {
                feedback.push((
                    character_id,
                    format!(
                        "You need {missing} more key{}.",
                        if missing > 1 { "s" } else { "" }
                    ),
                ));
            } else {
                feedback.push((character_id, "You need a key.".to_string()));
            }
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::DungeonDoorTooManyDefenders {
            character_id,
            alive,
            max_allowed,
            ..
        } => {
            feedback.push((
                character_id,
                format!("Too many Defenders are still alive ({alive} vs {max_allowed})."),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::DungeonDoorSolved { .. } => {
            *executed += 1;
        }
        _ => {}
    }
}
