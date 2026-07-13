//! Completed-action-outcome handling: the Area 38 (`src/area/38/
//! shrike.c`) `IDR_SHRIKE` family - the amulet-assembly puzzle's dead
//! tree/pedestal/rock (fresh amulet component pickups), the level-65
//! Moon door, the Pool of the Moon talisman activation, and the sliding
//! puzzle cube's player-push branch. `IDR_SHRIKEAMULET`'s own family
//! (combining the three components on the cursor) is a separate,
//! already-ported slice living in `tick_item_use_keyassembly.rs`.
//!
//! `ShrikeGiveAmuletPiece`/`ShrikeRockDigSuccess` need `ZoneLoader::
//! instantiate_item_template` to create the fresh amulet component
//! `World::apply_item_driver_outcome` can't see (same precedent as
//! `VaultShelfSearch`, `tick_item_use_keyassembly.rs`) - every other
//! variant here was already fully applied by `World::apply_item_driver_
//! outcome` (called synchronously inside `World::execute_item_driver_
//! request_with_context` before this dispatcher ever runs), so those
//! arms only produce feedback text/counters.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_shrike_outcome(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::ShrikeAmbientRefresh { .. } => {
            // Timer-only branch; player `item_use` completions never
            // produce this variant, but it is still a real
            // `ItemDriverOutcome` value the match must cover.
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ShrikeGiveAmuletPiece {
            character_id,
            piece,
            ..
        } => {
            if give_shrike_amulet_piece(world, zone_loader, character_id, piece) {
                *executed += 1;
            } else {
                *blocked += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::ShrikeHandOccupied {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "Please empty your hand (mouse cursor) first.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ShrikeRockNoTool { character_id } => {
            feedback.push((
                character_id,
                "You cannot take the piece of silver. The stone is too heavy to move.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ShrikeRockWrongTool { character_id } => {
            feedback.push((
                character_id,
                "You cannot get enough leverage with this to move the stone. Maybe a long stick with something thin on its end would do the trick."
                    .to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ShrikeRockDigSuccess {
            character_id,
            cursor_item_id,
            piece,
            ..
        } => {
            world.destroy_item(cursor_item_id);
            feedback.push((
                character_id,
                "You use the spade as a lever to move the stone. You take the piece of silver. Just as you try to remove the spade it snaps."
                    .to_string(),
            ));
            if give_shrike_amulet_piece(world, zone_loader, character_id, piece) {
                *executed += 1;
            } else {
                *blocked += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::ShrikeDoorTooWeak { character_id } => {
            feedback.push((
                character_id,
                "You feel a tingling in your fingers and decide not to touch the door again until you have grown stronger (requires level 65)."
                    .to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ShrikeDoorNeedsTalisman { character_id } => {
            feedback.push((
                character_id,
                "Your fingers tingle as you run them over the metal of the door and a whispered voice can be heard \"The Moon Blade\" is the key."
                    .to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ShrikeDoorEnter { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ShrikePoolSweetWater { character_id } => {
            feedback.push((
                character_id,
                "The water is sweet and refreshing.".to_string(),
            ));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ShrikePoolWetItem {
            character_id,
            cursor_item_id,
        } => {
            let item_name = world
                .items
                .get(&cursor_item_id)
                .map(|item| item.name.to_lowercase())
                .unwrap_or_default();
            feedback.push((character_id, format!("Your {item_name} is wet now.")));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ShrikePoolTalismanCreated {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "The talisman seems to glow faintly.".to_string(),
            ));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ShrikeCubeBlocked { character_id } => {
            feedback.push((character_id, "It won't move.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ShrikeCubePush { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ShrikeCubeAmbientTick { .. } => {
            // Timer-only branch, same as `ShrikeAmbientRefresh` above.
            *executed += 1;
        }
        _ => {}
    }
}

/// C `tree_driver`/`pede_driver`'s `create_item(...); it[in2].carried =
/// cn; ch[cn].citem = in2; ch[cn].flags |= CF_ITEMS;` (`shrike.c:113-
/// 123`/`:156-166`) and `rock_driver`'s matching success-branch creation
/// (`:207-212`) - a fresh amulet component placed directly on the
/// character's (already confirmed empty) cursor, not the general
/// inventory-then-hand `give_char_item` path.
fn give_shrike_amulet_piece(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    character_id: CharacterId,
    piece: ugaris_core::item_driver::ShrikeAmuletPiece,
) -> bool {
    let template = match piece {
        ugaris_core::item_driver::ShrikeAmuletPiece::Crystal => "shrike_amulet1",
        ugaris_core::item_driver::ShrikeAmuletPiece::Chain => "shrike_amulet2",
        ugaris_core::item_driver::ShrikeAmuletPiece::Charm => "shrike_amulet3",
    };
    let Ok(mut item) = zone_loader.instantiate_item_template(template, Some(character_id)) else {
        return false;
    };
    let Some(character) = world.characters.get_mut(&character_id) else {
        return false;
    };
    if character.cursor_item.is_some() {
        return false;
    }
    let item_id = item.id;
    item.carried_by = Some(character_id);
    character.cursor_item = Some(item_id);
    character
        .flags
        .insert(ugaris_core::entity::CharacterFlags::ITEMS);
    world.add_item(item);
    true
}
