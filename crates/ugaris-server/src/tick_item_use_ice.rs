//! Completed-action-outcome handling: the Ice-area (`src/area/10/ice.c`)
//! family of `ItemDriverOutcome` variants (ice-item spawn/warm-fire/
//! back-to-fire/melting-key-timer), plus the neighboring Palace-door/
//! Islena-door variants (`src/area/11/palace.c`) that sit in the same
//! contiguous match span in the legacy C source. Split out of the giant
//! `match outcome { ... }` block that still lives inline in `main.rs`'s
//! `tick.tick()` arm (P0.5 "Finish main() phase decomposition" -
//! REMAINING note: the completed-action-outcome handling needs
//! splitting by completed-action-kind family across several files, not
//! just relocation, because the whole match is too large to move
//! verbatim into one file). Warp, chests, and dungeon were sliced
//! first; this is the fourth family slice. The rest of the match
//! (teufel, edemon/fdemon, shrines, ...) is still inline in `main.rs`
//! pending further slices.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_ice_outcome(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
    failed: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::IceItemSpawn {
            character_id,
            template,
            ..
        } => match grant_ice_itemspawn_to_cursor(world, zone_loader, character_id, template) {
            IceItemSpawnGrantResult::Granted { item_name } => {
                feedback.push((character_id, format!("You got a {item_name}.")));
                *executed += 1;
            }
            IceItemSpawnGrantResult::OneCarry { item_name } => {
                feedback.push((
                    character_id,
                    format!("You can only carry one {item_name} at a time!"),
                ));
                *blocked += 1;
            }
            IceItemSpawnGrantResult::CannotCarry => {
                *blocked += 1;
            }
            IceItemSpawnGrantResult::Bug => {
                feedback.push((
                    character_id,
                    "Congratulations, you have just discovered bug #4244C, please report it to the authorities!".to_string(),
                ));
                *failed += 1;
            }
        },
        ugaris_core::item_driver::ItemDriverOutcome::IceItemSpawnCursorOccupied {
            character_id,
            ..
        }
        | ugaris_core::item_driver::ItemDriverOutcome::WarmFireCursorOccupied {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "Please empty your 'hand' (mouse cursor) first.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::IceItemSpawnBug {
            character_id, kind, ..
        } => {
            feedback.push((
                character_id,
                format!(
                    "Congratulations, you have just discovered bug #4244B-{kind}, please report it to the authorities!"
                ),
            ));
            *failed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::WarmFire {
            character_id,
            create_scroll,
            removed_curse,
            ..
        } => {
            if create_scroll
                && grant_warmfire_scroll_to_cursor(world, zone_loader, character_id).is_some()
            {
                feedback.push((
                    character_id,
                    "Next to the fire, you find an ancient scroll. It seems to be a scroll of teleport which will take you back here.".to_string(),
                ));
            }
            if removed_curse {
                feedback.push((
                    character_id,
                    "You move close to the heat of the fire, and you feel the demon's cold leave you.".to_string(),
                ));
            } else {
                feedback.push((character_id, "You warm your hands on the fire.".to_string()));
            }
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::BackToFire { character_id, .. } => {
            feedback.push((character_id, "The scroll vanished.".to_string()));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::MeltingKeyTick { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::PalaceDoorKeyRequired {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "You need a key to open this gate.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::IslenaDoorBusy { character_id, .. } => {
            feedback.push((
                character_id,
                "You hear fighting behind the door. It seems Islena is killing somebody else at the moment. Please come back later so she can take care of you, too.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::IslenaDoorRespawning {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "Islena is being re-incarnated. Please try again soon.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::IslenaDoorResting { character_id, .. } => {
            feedback.push((
                character_id,
                "Islena is resting after killing your predecessor. Being well mannered, you wait for her.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::PalaceDoorTick { .. } => {
            *executed += 1;
        }
        _ => {}
    }
}
