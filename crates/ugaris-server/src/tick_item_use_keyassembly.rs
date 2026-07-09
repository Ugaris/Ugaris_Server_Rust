//! Completed-action-outcome handling: the "key-assembly" family of
//! `ItemDriverOutcome` variants that several area `.c` files share the
//! same two-piece-combine idiom for (staffer/saltmine mine-digging
//! puzzle `src/area/26/staffer.c`, bone-holder rune stand
//! `src/area/18/bones.c`, Arkhata clerk pool/stopwatch/key
//! `src/area/37/arkhata.c`, lizard-flower potion mixing, palace key
//! `src/area/11/palace.c`, mine gateway/mine-key door
//! `src/area/12/mine.c`, and Shrike amulet assembly
//! `src/area/38/shrike.c`). Split out of the giant `match outcome { ... }`
//! block that still lives inline in `main.rs`'s `tick.tick()` arm (P0.5
//! "Finish main() phase decomposition" - REMAINING note: the completed-
//! action-outcome handling needs splitting by completed-action-kind
//! family across several files, not just relocation, because the whole
//! match is too large to move verbatim into one file). Warp, chests,
//! dungeon, ice/palace, Teufel, skel-raise, Edemon/Fdemon, transport,
//! clan/LQ/arena, shrines, burndown, xmas/swamp, and Caligar were sliced
//! first; this is the fourteenth family slice. Like Caligar/Edemon-
//! Fdemon, this family's variants were scattered across 6 spots in
//! `main.rs`'s match (including several field-less no-op variants living
//! inside the shared no-op catch-all's or-pattern); each spot's lines
//! were removed and replaced (at the first spot) with one combined or-
//! pattern call arm. The `SaltmineSaltbagUse` branch's original
//! `continue` (valid inside the enclosing `for completion in
//! &completed_actions` loop) became `return`, the equivalent "stop
//! processing this outcome" behavior at function scope, matching the
//! precedent set by the shrines slice. The rest of the match (lab2/lab3,
//! the large no-op catch-all, ...) is still inline in `main.rs` pending
//! further slices.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn dispatch_keyassembly_outcome(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
    config: &ServerConfig,
    args: &Args,
    realtime_seconds: u64,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
    failed: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::StafferBookText {
            character_id, page, ..
        } => {
            if let Some(line) = ugaris_core::item_driver::staffer_book_text(page) {
                feedback.push((character_id, line.to_string()));
            }
            if let Some(line) = ugaris_core::item_driver::staffer_book_continue_text(page) {
                feedback.push((character_id, line.to_string()));
            }
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::StafferAnimationBook {
            character_id,
            exp_added,
            ..
        } => {
            let grant_exp = runtime
                .player_for_character_mut(character_id)
                .map(|player| player.mark_staffer_animation_book_seen())
                .unwrap_or(false);
            if grant_exp {
                // C `staffer_animation_book`
                // (`area/29/brannington.c:521`) grants exp via
                // `give_exp(cn, ...)`, not a raw mutation.
                world.give_exp(character_id, i64::from(exp_added), u32::from(args.area_id));
            }
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::StafferMineExhausted {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "You're too exhausted to continue digging.".to_string(),
            ));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::StafferBlockBlocked {
            character_id, ..
        } => {
            feedback.push((character_id, "It won't move.".to_string()));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::StafferSpecDoorLocked {
            character_id, ..
        } => {
            feedback.push((character_id, "The door is locked.".to_string()));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::StafferMineDig { .. }
        | ugaris_core::item_driver::ItemDriverOutcome::StafferMineTimer { .. }
        | ugaris_core::item_driver::ItemDriverOutcome::StafferBlockMove { .. }
        | ugaris_core::item_driver::ItemDriverOutcome::StafferBlockTimer { .. }
        | ugaris_core::item_driver::ItemDriverOutcome::StafferSpecDoorToggle { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::SaltmineDoorBlocked {
            character_id, ..
        } => {
            feedback.push((character_id, "Thou canst not enter there.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::SaltmineLadderUse {
            character_id,
            ladder_index,
            ..
        } => {
            if let Some(player) = runtime.player_for_character_mut(character_id) {
                if player.saltmine_ladder_ready(ladder_index, realtime_seconds) {
                    player.mark_saltmine_ladder_used(ladder_index, realtime_seconds);
                    feedback.push((
                        character_id,
                        "Thou signalst the monks to gather salt from this ladder.".to_string(),
                    ));
                    *executed += 1;
                } else {
                    feedback.push((character_id, "Thou already got all the Salt out of this, so thou have to wait until it is refilled again.".to_string()));
                    *blocked += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::SaltmineSaltbagUse {
            character_id, ..
        } => {
            if world
                .characters
                .get(&character_id)
                .is_some_and(|character| character.cursor_item.is_some())
            {
                *blocked += 1;
                return;
            }
            let units = runtime
                .player_for_character(character_id)
                .map(|player| player.saltmine_pending_salt.saturating_mul(1000))
                .unwrap_or(0);
            if units == 0 {
                feedback.push((
                    character_id,
                    "Thou feelst thou should bring salt to the monastery, before rewarding thinself."
                        .to_string(),
                ));
                *blocked += 1;
            } else if grant_salt_to_cursor(world, zone_loader, character_id, units) {
                if let Some(player) = runtime.player_for_character_mut(character_id) {
                    player.saltmine_pending_salt = 0;
                }
                feedback.push((
                    character_id,
                    format!("Thou took {units} units of salt, feeling thou have earned it."),
                ));
                *executed += 1;
            } else {
                *blocked += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::BoneHint {
            character_id,
            level,
            nr,
            pos,
            ..
        } => {
            if let Some(player) = runtime.player_for_character_mut(character_id) {
                match player.bone_hint(level, nr, pos, |limit| {
                    runtime_random_below(limit as i32).max(0) as u32
                }) {
                    ugaris_core::player::BoneHintResult::Hint {
                        page,
                        rune,
                        position,
                    } => {
                        feedback.push((character_id, format!("Rune Diary, Page {page}:")));
                        feedback.push((
                            character_id,
                            format!("Used the rune {rune} in the {position} position."),
                        ));
                    }
                    ugaris_core::player::BoneHintResult::Bug {
                        level,
                        nr,
                        pos,
                        value,
                    } => {
                        feedback.push((
                            character_id,
                            format!("You found bug #197-{level}-{nr}-{pos}-{value}"),
                        ));
                    }
                }
            }
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::PalaceKeySplit {
            item_id,
            character_id,
            cursor_part_sprite,
            carried_part_sprite,
        } => {
            match apply_palace_key_split(
                world,
                zone_loader,
                item_id,
                character_id,
                cursor_part_sprite,
                carried_part_sprite,
            ) {
                AssembleApplyResult::Assembled => {
                    *executed += 1;
                }
                AssembleApplyResult::TemplateUnavailable => {
                    feedback.push((character_id, "That doesn't fit.".to_string()));
                    *blocked += 1;
                }
                AssembleApplyResult::MissingPlayer | AssembleApplyResult::MissingItem => {
                    *failed += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::PalaceKeyNeedsCursor {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "The only thing you can think of to do with this key part is to add another key part to it."
                    .to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::PalaceKeyDoesNotFit {
            character_id, ..
        } => {
            feedback.push((character_id, "That doesn't fit.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::PalaceKeyCombine { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::EnchantNeedsCursor {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "You have to use another item on this one.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::EnchantCursorItem { .. }
        | ugaris_core::item_driver::ItemDriverOutcome::AntiEnchantCursorItem { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ShrikeAmuletNeedsCursor {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "You can only use this item with another item.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ShrikeAmuletDoesNotFit {
            character_id,
            ..
        } => {
            feedback.push((character_id, "It doesn't fit.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ShrikeAmuletAssemble { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::MineGatewayKeyNeedsCursor {
            character_id,
            ..
        } => {
            feedback.push((character_id, "Use, yes, but use it with what?".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::MineGatewayKeyDoesNotFit {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "Interesting idea. Really. Doesn't work, though.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::MineGatewayKeyAssemble { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::MineGateway {
            character_id,
            area_id,
            x,
            y,
            ..
        } => {
            if area_id != config.area_id {
                let transferred = attempt_cross_area_transfer(
                    world,
                    runtime,
                    character_repository,
                    area_repository,
                    config.area_id,
                    config.mirror_id,
                    character_id,
                    area_id,
                    u32::from(config.mirror_id),
                    x,
                    y,
                )
                .await;
                if transferred {
                    *executed += 1;
                } else {
                    feedback.push((
                        character_id,
                        "Nothing happens - target area server is down.".to_string(),
                    ));
                    *blocked += 1;
                }
            } else {
                *executed += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::MineGatewayNeedsKey {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "The door won't open. You notice an inscription: \"This door leads to the Dwarven town Grimroot. Only those who have proven their abilities as miners and fighters may enter.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::MineGatewayBug {
            character_id,
            x,
            y,
            area_id,
            ..
        } => {
            let name = world
                .characters
                .get(&character_id)
                .map(|character| character.name.as_str())
                .unwrap_or("Someone");
            feedback.push((
                character_id,
                format!("{name} touches a teleport object but nothing happens - BUG ({x},{y},{area_id})."),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::MineKeyDoorNeedsGold {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "You'll need to use 2000 gold units as a key to open the door.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::MineKeyDoorBusy { character_id, .. } => {
            feedback.push((
                character_id,
                "You hear fighting noises from behind the door. It won't open while the fight lasts."
                    .to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::MineKeyDoorOpened {
            character_id,
            golem_nr,
            room_x,
            room_y,
            ..
        } => {
            crate::mine::spawn_keyholder_golem(
                world,
                zone_loader,
                runtime,
                character_id,
                golem_nr,
                room_x,
                room_y,
            );
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ArkhataKeyNeedsCursor {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "You can only use this item with another item.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ArkhataKeyDoesNotFit {
            character_id, ..
        } => {
            feedback.push((character_id, "This doesn't seem to fit.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ArkhataKeyAssemble { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ArkhataPool {
            character_id,
            cursor_item_id,
            ..
        } => {
            match apply_arkhata_pool(
                world,
                zone_loader,
                character_id,
                cursor_item_id,
                runtime_random_seed(),
            ) {
                ArkhataPoolApplyResult::Gift(item_name) => {
                    feedback.push((character_id, format!("You got a {}.", item_name)));
                    *executed += 1;
                }
                ArkhataPoolApplyResult::Vanished => {
                    feedback.push((character_id, "It vanished in the pool. You sense that the idea was right, but more of the same is needed for a result.".to_string()));
                    *executed += 1;
                }
                ArkhataPoolApplyResult::MissingGift => {
                    *failed += 1;
                }
                ArkhataPoolApplyResult::MissingPlayer | ArkhataPoolApplyResult::MissingCursor => {
                    *failed += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::ArkhataPoolNeedsCursor {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "You sense that you have to use the pool with another item (put it on your mouse cursor).".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ArkhataPoolWrongCursor {
            character_id,
            cursor_item_id,
            ..
        } => {
            let cursor_name = world
                .items
                .get(&cursor_item_id)
                .map(|item| item.name.as_str())
                .unwrap_or("item");
            feedback.push((
                character_id,
                format!("Strangely, the {} floats on the surface of the pool. Since nothing happens to it, you take it back.", cursor_name),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ArkhataStopwatch { character_id, .. } => {
            if character_id.0 != 0 {
                if let Some(player) = runtime.player_for_character(character_id) {
                    let text = arkhata_stopwatch_feedback(player, realtime_seconds);
                    feedback.push((character_id, text));
                    *executed += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::BoneHolderBadCursor {
            character_id, ..
        } => {
            feedback.push((character_id, "That does not fit.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::BoneHolderOccupied {
            character_id, ..
        } => {
            feedback.push((character_id, "There is a rune already.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::BoneHolderEmptyTouch {
            character_id, ..
        } => {
            feedback.push((character_id, "You touch the stand.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::BoneHolderWrongOwner {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "This rune does not belong to you. You cannot take it.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::BoneHolderInsertRune { .. }
        | ugaris_core::item_driver::ItemDriverOutcome::BoneHolderExpired { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::BoneHolderRemoveRune {
            character_id,
            rune,
            ..
        } => {
            // C `boneholder`'s "remove rune from holder" branch
            // (`bones.c:759-768`): the driver already verified the cursor
            // is empty, so `give_char_item` (empty-cursor-first) lands on
            // the cursor exactly like C's direct `ch[cn].citem = in2`
            // assignment. `create_rune_from_holder` failing is C's "bug
            // #11970" branch.
            match zone_loader.instantiate_item_template(&format!("rune{rune}"), Some(character_id))
            {
                Ok(item) => {
                    let item_id = item.id;
                    world.add_item(item);
                    world.give_char_item(character_id, item_id);
                    *executed += 1;
                }
                Err(_) => {
                    feedback.push((character_id, "You found bug #11970".to_string()));
                    *failed += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::BoneHolderActivateResolved {
            character_id,
            last_holder,
            nr,
            cleared,
            ..
        } => {
            // C `remove_rune_from_holder`'s `give_char_item` variant
            // (`bones.c:678-688`), called once per stand the scan
            // cleared, regardless of whether `nr` ends up usable.
            for (_, rune) in cleared.into_iter().flatten() {
                if let Ok(item) = zone_loader
                    .instantiate_item_template(&format!("rune{rune}"), Some(character_id))
                {
                    let item_id = item.id;
                    world.add_item(item);
                    if !world.give_char_item(character_id, item_id) {
                        world.destroy_item(item_id);
                    }
                }
            }

            if nr == 0 {
                feedback.push((
                    character_id,
                    "You sense that you must place something on the stand before you can activate it."
                        .to_string(),
                ));
                *blocked += 1;
                return;
            }

            let Some(player) = runtime.player_for_character_mut(character_id) else {
                *blocked += 1;
                return;
            };
            player.ensure_rune_special_execs(|limit| {
                runtime_random_below(limit as i32).max(0) as u32
            });
            match player.rune_check(nr) {
                ugaris_core::player::RuneCheckResult::OutOfRange => {
                    feedback.push((character_id, "You have found bug #5136a.".to_string()));
                    *blocked += 1;
                }
                ugaris_core::player::RuneCheckResult::AlreadyUsed => {
                    feedback.push((
                        character_id,
                        "You cannot use this combination again.".to_string(),
                    ));
                    *blocked += 1;
                }
                ugaris_core::player::RuneCheckResult::Ok => {
                    let special_exec = player.rune_special_exec;
                    let flag = world.exec_rune(
                        character_id,
                        nr,
                        &special_exec,
                        last_holder,
                        u32::from(args.area_id),
                    );
                    if flag {
                        player.rune_set(nr);
                    }
                    *executed += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::LizardFlowerMixed {
            character_id,
            complete,
            bottle_message,
            ..
        } => {
            if bottle_message {
                feedback.push((
                    character_id,
                    "A bottle pops out of thin air as you try to combine the flowers. You're stunned for a moment, but then you mix the flowers in the bottle."
                        .to_string(),
                ));
            }
            if complete {
                feedback.push((character_id, "The potion seems finished.".to_string()));
            }
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::LizardFlowerNeedsCursor {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "No, eating this berry isn't a good idea.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::LizardFlowerDoesNotFit {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "This cannot be used together. Try something else.".to_string(),
            ));
            *blocked += 1;
        }
        _ => {}
    }
}
