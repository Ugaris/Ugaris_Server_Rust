//! Completed-action-outcome handling: the keyring/assemble/gathering/
//! alchemy-flask family of `ItemDriverOutcome` variants (`src/module/
//! keyring.c`'s keyring show/add-key/keyed-door toggle, the generic
//! `assemble_item` two-piece-combine driver, `src/module/park.c`'s park
//! shrine memorization, `src/area/31` berry picking and alchemy flower
//! picking, and `src/module/alchemy.c`'s flask ingredient-adding/mixing/
//! ruining). Split out of the giant `match outcome { ... }` block that
//! still lives inline in `main.rs`'s `tick.tick()` arm (P0.5 "Finish
//! main() phase decomposition" - REMAINING note: the completed-action-
//! outcome handling needs splitting by completed-action-kind family
//! across several files, not just relocation, because the whole match is
//! too large to move verbatim into one file). Warp, chests, dungeon,
//! ice/palace, Teufel, skel-raise, Edemon/Fdemon, transport, clan/LQ/
//! arena, shrines, burndown, xmas/swamp, Caligar, key-assembly, and
//! labyrinth were sliced first; this is the sixteenth family slice. The
//! rest of the match (the large no-op catch-all, ...) is still inline in
//! `main.rs` pending further slices.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn dispatch_crafting_outcome(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    realtime_seconds: u64,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    driver_context: &ugaris_core::item_driver::ItemDriverContext,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
    failed: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::KeyringShow { character_id, .. } => {
            for message in keyring_show_messages(runtime.player_for_character(character_id)) {
                feedback.push((character_id, message));
            }
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::Extinguish {
            character_id,
            extinguished,
            ..
        } => {
            feedback.push((
                character_id,
                if extinguished {
                    "You extinguish the flames."
                } else {
                    "Ahh. Sweet and refreshing."
                }
                .to_string(),
            ));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::KeyedDoorToggle {
            character_id,
            key_id,
            source,
            locking,
            ..
        } => {
            if source == ugaris_core::item_driver::DoorKeySource::Keyring {
                let action = if locking { "lock" } else { "unlock" };
                let key_name = driver_context
                    .door_key
                    .as_ref()
                    .map(|key| key.name.as_str())
                    .unwrap_or("a key");
                feedback.push((
                    character_id,
                    format!(
                        "You use {key_name} (ID: {key_id:08X}) from your keyring to {action} the door."
                    ),
                ));
            }
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::KeyringAddCursorItem {
            character_id,
            key_item_id,
            ..
        } => match apply_keyring_add_cursor_item(
            world,
            runtime.player_for_character_mut(character_id),
            character_id,
            key_item_id,
        ) {
            KeyringAddApplyResult::Added { key_name } => {
                feedback.push((character_id, format!("You add {key_name} to your keyring.")));
                *executed += 1;
            }
            KeyringAddApplyResult::Duplicate => {
                feedback.push((
                    character_id,
                    "This key is already on your keyring.".to_string(),
                ));
                *blocked += 1;
            }
            KeyringAddApplyResult::Full => {
                feedback.push((character_id, "Your keyring is full.".to_string()));
                *blocked += 1;
            }
            KeyringAddApplyResult::NotAKey => {
                feedback.push((
                    character_id,
                    "You can only add keys to the keyring.".to_string(),
                ));
                *blocked += 1;
            }
            KeyringAddApplyResult::MissingPlayer | KeyringAddApplyResult::MissingCursorItem => {
                *failed += 1;
            }
        },
        ugaris_core::item_driver::ItemDriverOutcome::AssembleItem {
            item_id,
            character_id,
            cursor_item_id,
            template,
        } => match apply_assemble_item(
            world,
            zone_loader,
            item_id,
            character_id,
            cursor_item_id,
            template.as_str(),
        ) {
            AssembleApplyResult::Assembled => {
                *executed += 1;
            }
            AssembleApplyResult::TemplateUnavailable => {
                feedback.push((character_id, "That doesn't seem to fit.".to_string()));
                *blocked += 1;
            }
            AssembleApplyResult::MissingPlayer | AssembleApplyResult::MissingItem => {
                *failed += 1;
            }
        },
        ugaris_core::item_driver::ItemDriverOutcome::AssembleNeedsCursor {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "You can only use this item with another item.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::AssembleDoesNotFit {
            character_id, ..
        } => {
            feedback.push((character_id, "That doesn't seem to fit.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::AssembleUnknownItem {
            character_id, ..
        } => {
            feedback.push((character_id, "Bug # 42556".to_string()));
            *failed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ParkShrine {
            character_id,
            shrine,
            ..
        } => {
            if let Some(player) = runtime.player_for_character_mut(character_id) {
                if player.memorize_park_shrine(shrine).unwrap_or(false) {
                    feedback.push((
                        character_id,
                        "You memorize the location of the shrine.".to_string(),
                    ));
                } else {
                    feedback.push((character_id, "This shrine seems familar.".to_string()));
                }
                *executed += 1;
            } else {
                *failed += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::ParkShrineBug { character_id, .. } => {
            feedback.push((character_id, "BUG #55343, please report".to_string()));
            *failed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::PickBerry {
            character_id,
            kind,
            location_id,
            ..
        } => match apply_pick_berry(
            world,
            zone_loader,
            runtime.player_for_character_mut(character_id),
            character_id,
            kind,
            location_id,
            realtime_seconds,
        ) {
            PickBerryApplyResult::Picked(_) => {
                *executed += 1;
            }
            PickBerryApplyResult::NotRipe => {
                feedback.push((character_id, "It's not ripe yet.".to_string()));
                *blocked += 1;
            }
            PickBerryApplyResult::CursorOccupied => {
                feedback.push((
                    character_id,
                    "Please empty your hand (mouse cursor) first.".to_string(),
                ));
                *blocked += 1;
            }
            PickBerryApplyResult::Bug => {
                feedback.push((character_id, "Bug # 4111c".to_string()));
                *failed += 1;
            }
            PickBerryApplyResult::MissingPlayer => {
                *failed += 1;
            }
        },
        ugaris_core::item_driver::ItemDriverOutcome::PickBerryCursorOccupied {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "Please empty your hand (mouse cursor) first.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::PickAlchemyFlower {
            character_id,
            kind,
            location_id,
            ..
        } => match apply_pick_alchemy_flower(
            world,
            zone_loader,
            runtime.player_for_character_mut(character_id),
            character_id,
            kind,
            location_id,
            realtime_seconds,
        ) {
            PickBerryApplyResult::Picked(_) => {
                award_gathering_achievement(
                    world,
                    runtime,
                    achievement_repository,
                    character_id,
                    kind,
                )
                .await;
                *executed += 1;
            }
            PickBerryApplyResult::NotRipe => {
                feedback.push((character_id, "It's not ripe yet.".to_string()));
                *blocked += 1;
            }
            PickBerryApplyResult::CursorOccupied => {
                feedback.push((
                    character_id,
                    "Please empty your hand (mouse cursor) first.".to_string(),
                ));
                *blocked += 1;
            }
            PickBerryApplyResult::Bug => {
                feedback.push((character_id, "Bug # 4111".to_string()));
                *failed += 1;
            }
            PickBerryApplyResult::MissingPlayer => {
                *failed += 1;
            }
        },
        ugaris_core::item_driver::ItemDriverOutcome::PickAlchemyFlowerCursorOccupied {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "Please empty your hand (mouse cursor) first.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::FlaskIngredientAdded {
            item_id,
            character_id,
            cursor_item_id,
            ingredient_kind,
        } => {
            if let Some(name) = apply_flask_ingredient_added(
                world,
                character_id,
                item_id,
                cursor_item_id,
                ingredient_kind,
            ) {
                feedback.push((character_id, format!("You put {name} into the flask.")));
                *executed += 1;
            } else {
                *failed += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::FlaskWrongCursor { character_id, .. } => {
            feedback.push((
                character_id,
                "That's not an ingredient you can use in a flask.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::FlaskFull { character_id, .. } => {
            feedback.push((character_id, "The Flask is full.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::FlaskFinishedNoMoreIngredients {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "This potion is finished. You cannot add more ingredients.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::FlaskEmptyShaken { character_id, .. } => {
            feedback.push((
                character_id,
                "You shake the empty bottle, but nothing happens.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::FlaskIngredientBug {
            character_id, ..
        } => {
            feedback.push((character_id, "BUG # 231...".to_string()));
            *failed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::FlaskMixed {
            character_id,
            ingredient_counts,
            ..
        } => {
            for message in flask_ingredient_feedback(ingredient_counts) {
                feedback.push((character_id, message));
            }
            feedback.push((character_id, "The potion seems finished.".to_string()));
            award_potion_brewed_achievement(world, runtime, achievement_repository, character_id)
                .await;
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::FlaskRuined {
            character_id,
            ingredient_counts,
            ..
        } => {
            for message in flask_ingredient_feedback(ingredient_counts) {
                feedback.push((character_id, message));
            }
            feedback.push((
                character_id,
                "You shake the bottle and create a stinking liquid which you throw away."
                    .to_string(),
            ));
            *executed += 1;
        }
        _ => {}
    }
}
