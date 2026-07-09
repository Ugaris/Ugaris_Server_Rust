//! Completed-action-outcome handling: the labyrinth family of
//! `ItemDriverOutcome` variants (`src/area/22/lab*.c`'s Brannington
//! underwater berry, Lab3 yellow/white/brown berries and the white-berry
//! light timer, Lab2 water well/altar/drink/cursor, Lab2 step-action
//! clear/daemon-check/daemon-warning spawn, Lab2 grave clue-book/close/
//! check-open/open, the shared lab-entrance solved-all/too-low and
//! lab-exit wrong-owner blocks, and the shared `labexit` reward loop
//! itself, `LabExitUse` - C `set_solved_lab` (`src/system/lab.c:114-135`)
//! plus `labexit`'s trailing `change_area(cn, 3, 183, 199)`
//! (`src/module/base.c:4749-4778`), common to every one of the five
//! `lab*.c` files' `create_lab_exit` reward drops). Split out of the
//! giant `match outcome { ... }` block that still lives inline in
//! `main.rs`'s `tick.tick()` arm (P0.5 "Finish main() phase
//! decomposition" - REMAINING note: the completed-action-outcome
//! handling needs splitting by completed-action-kind family across
//! several files, not just relocation, because the whole match is too
//! large to move verbatim into one file). Warp, chests, dungeon, ice/
//! palace, Teufel, skel-raise, Edemon/Fdemon, transport, clan/LQ/arena,
//! shrines, burndown, xmas/swamp, Caligar, and key-assembly were sliced
//! first; this is the fifteenth family slice. The rest of the match (the
//! large no-op catch-all, ...) is still inline in `main.rs` pending
//! further slices.
//!
//! `LabExitAnimating`/`LabExitExpired` need no handling here: their
//! mutations (rescheduling the sprite-cycle timer / destroying the
//! expired gate) already happen in `ugaris-core`'s
//! `World::apply_item_driver_outcome` before this dispatcher ever sees
//! them, so they stay in `tick_item_use_completion.rs`'s generic
//! executed-only bucket. `LabExitUse` is different: C's `set_solved_lab`
//! needs the DB-backed `PlayerRuntime::lab_solved_bits`/`give_exp`
//! (`World` alone can't see `PlayerRuntime`), and `change_area` needs
//! the cross-area-transfer machinery - both only reachable from
//! `ugaris-server`, hence this dispatcher and not `apply_item_driver_
//! outcome`.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn dispatch_lab_outcome(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
    config: &ServerConfig,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
    failed: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::BranningtonUnderwaterBerry {
            installed,
            ..
        } => {
            if installed {
                *executed += 1;
            } else {
                *blocked += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab3YellowBerry {
            character_id,
            installed,
            ..
        } => {
            if installed {
                *executed += 1;
            } else {
                feedback.push((
                    character_id,
                    "Due to some strange reasons thou canst not eat those berries now.".to_string(),
                ));
                *blocked += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab3WhiteBerry {
            character_id,
            installed,
            ..
        } => {
            if installed {
                *executed += 1;
            } else {
                feedback.push((
                    character_id,
                    "Due to some strange reasons thou canst not eat those berries now.".to_string(),
                ));
                *blocked += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab3WhiteBerryLightTick { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab3BrownBerry {
            character_id,
            installed,
            ..
        } => {
            if installed {
                *executed += 1;
            } else {
                feedback.push((
                    character_id,
                    "Thou art still chewing a brown berry.".to_string(),
                ));
                *blocked += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab2WaterWell { character_id, .. } => {
            if let Some(item_name) =
                grant_template_item_to_cursor(world, zone_loader, character_id, "lab2_waterbowl")
            {
                feedback.push((character_id, format!("You received a {item_name}.")));
                *executed += 1;
            } else {
                *failed += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab2WaterAltar { character_id, .. } => {
            match apply_lab2_water_altar(world, zone_loader, character_id) {
                Lab2WaterApplyResult::Converted(0) => {
                    feedback.push((
                        character_id,
                        "You feel the holyness of the Altar. Water would be holy now, if you had some."
                            .to_string(),
                    ));
                    *blocked += 1;
                }
                Lab2WaterApplyResult::Converted(1) => {
                    feedback.push((
                        character_id,
                        "The water inside your bowl is holy now.".to_string(),
                    ));
                    *executed += 1;
                }
                Lab2WaterApplyResult::Converted(count) => {
                    feedback.push((
                        character_id,
                        format!("The water inside your {count} bowls is holy now."),
                    ));
                    *executed += 1;
                }
                Lab2WaterApplyResult::MissingPlayer | Lab2WaterApplyResult::TemplateMissing => {
                    *failed += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab2WaterDrink { character_id, .. } => {
            feedback.push((character_id, "Skoll!".to_string()));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab2WaterCursorOccupied {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "You won't throw this into the water, will you?".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab2StepActionClear { .. }
        | ugaris_core::item_driver::ItemDriverOutcome::Lab2StepActionDaemonCheck { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab2StepActionDaemonWarning {
            character_id: triggering_id,
            x,
            y,
            ..
        } => {
            // C `lab2_deamon_create`'s dedup loop (`lab2.c:376-388`): don't
            // spawn a second daemon already tracking this exact player.
            let already_tracked = world
                .characters
                .get(&triggering_id)
                .is_some_and(|triggering| {
                    world.lab2_deamon_already_tracking(triggering_id, triggering.serial)
                });
            if already_tracked {
                *blocked += 1;
            } else {
                let character_id = runtime.allocate_character_id();
                match zone_loader.instantiate_character_template("lab2_daemon", character_id) {
                    Ok((daemon, inventory_items)) => {
                        // C `drop_char(cn, x, y, 0) || drop_char(cn, x, y+3,
                        // 0)` (`lab2.c:405-409`).
                        let placed =
                            world.spawn_character(daemon.clone(), usize::from(x), usize::from(y))
                                || world.spawn_character(
                                    daemon,
                                    usize::from(x),
                                    usize::from(y.saturating_add(3)),
                                );
                        if placed {
                            for item in inventory_items {
                                world.items.insert(item.id, item);
                            }
                            let serial = world
                                .characters
                                .get(&triggering_id)
                                .map(|triggering| triggering.serial)
                                .unwrap_or_default();
                            world.init_lab2_deamon(character_id, triggering_id, serial);
                            *executed += 1;
                        } else {
                            *failed += 1;
                        }
                    }
                    _ => {
                        *failed += 1;
                    }
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab2GraveClueBook {
            character_id,
            book,
            ..
        } => {
            let text = lab2_grave_clue_text(runtime, character_id, book);
            feedback.push((character_id, text));
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab2GraveClose { .. }
        | ugaris_core::item_driver::ItemDriverOutcome::Lab2GraveCheckOpen { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab2GraveOpen {
            item_id,
            character_id,
            fixed_item,
        } => {
            if apply_lab2_grave_open(
                world,
                runtime,
                zone_loader,
                item_id,
                character_id,
                fixed_item,
            ) {
                *executed += 1;
            } else {
                *failed += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::LabEntranceSolvedAll {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "You have solved all existing labyrinths already. You can now fight the gatekeeper."
                    .to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::LabEntranceTooLow {
            character_id,
            required_level,
            ..
        } => {
            feedback.push((
                character_id,
                format!("You may not enter before reaching level {required_level}."),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::LabExitWrongOwner { character_id, .. } => {
            feedback.push((
                character_id,
                "This gate has not been created for you. You cannot use it.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::LabExitUse {
            character_id,
            lab_nr,
            target_area,
            target_x,
            target_y,
            ..
        } => {
            // C `set_solved_lab(cn, it[in].drdata[4])`
            // (`src/system/lab.c:114-135`): only the *first* use of a
            // given `lab_nr`'s gate awards exp/the congratulations
            // message - `ppd->solved_bits` gates both.
            let bit = 1u64 << (lab_nr & 63);
            let already_solved = runtime
                .player_for_character(character_id)
                .is_some_and(|player| player.lab_solved_bits & bit != 0);
            if !already_solved {
                if let Some(player) = runtime.player_for_character_mut(character_id) {
                    player.lab_solved_bits |= bit;
                }
                world.give_exp(
                    character_id,
                    i64::from(level_value(u32::from(lab_nr))) / 5,
                    u32::from(config.area_id),
                );
                if let Some(character) = world.characters.get(&character_id) {
                    let name = character.name.clone();
                    feedback.push((
                        character_id,
                        format!(
                            "Congratulations, {name}, you have solved this part of the labyrinth."
                        ),
                    ));
                }
            }

            // C `labexit`'s trailing `change_area(cn, 3, 183, 199)`
            // (`src/module/base.c:4776-4778`); the gate's own
            // `drdata[8]` close-frame write already happened inside
            // `labexit_driver`.
            let transferred = if target_area == config.area_id {
                world.teleport_character_same_area(character_id, target_x, target_y, false)
            } else {
                attempt_cross_area_transfer(
                    world,
                    runtime,
                    character_repository,
                    area_repository,
                    config.area_id,
                    config.mirror_id,
                    character_id,
                    target_area,
                    u32::from(config.mirror_id),
                    target_x,
                    target_y,
                )
                .await
            };
            if transferred {
                *executed += 1;
            } else {
                feedback.push((
                    character_id,
                    "Sorry, Aston is down. Please try again soon.".to_string(),
                ));
                *blocked += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::DeathfibrinShrineGive {
            character_id, ..
        } => {
            if let Some(item_name) =
                grant_template_item_to_cursor(world, zone_loader, character_id, "deathfibrin")
            {
                feedback.push((character_id, format!("You received a {item_name}.")));
                *executed += 1;
            } else {
                *failed += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::DeathfibrinShrineOccupied { character_id } => {
            feedback.push((
                character_id,
                "The Shrine of Deathfibrin seems to ignore everything. It may want to give you something."
                    .to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::DeathfibrinNeedsCarry { character_id } => {
            feedback.push((
                character_id,
                "You need to carry this to use it.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::DeathfibrinNoMaster {
            character_id,
            tile_light,
        } => {
            feedback.push((
                character_id,
                format!("Nothing happens. There is no immortal close enough. {tile_light}"),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::DeathfibrinStrike {
            character_id,
            item_name,
            vanished,
            ..
        } => {
            if vanished {
                let item_name = String::from_utf8_lossy(&item_name)
                    .trim_end_matches('\0')
                    .to_string();
                feedback.push((character_id, format!("Your {item_name} vanished.")));
            }
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab3TeleportDoorLocked { character_id } => {
            let name = world
                .characters
                .get(&character_id)
                .map(|character| character.name.clone())
                .unwrap_or_default();
            feedback.push((
                character_id,
                format!("The Guard has not opened the door for thee yet, {name}."),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab3TeleportDoorBusy { character_id } => {
            feedback.push((
                character_id,
                "Hm. It seems there is a crowd behind the door. Please try again later."
                    .to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab3TeleportDoor {
            character_id,
            extinguished_count,
            ..
        } => {
            if extinguished_count > 0 {
                let suffix = if extinguished_count == 1 { "" } else { "es" };
                feedback.push((
                    character_id,
                    format!("Thine torch{suffix} extinguished due to the water."),
                ));
            }
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab3NoteGivingBlocked { character_id } => {
            feedback.push((character_id, "Nothing happens.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab3NoteGivingSkeleton {
            character_id,
            note_value,
            ..
        } => {
            if create_lab3_note_for_character(world, zone_loader, character_id, note_value) {
                *executed += 1;
            } else {
                *failed += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab3NoteRead {
            character_id,
            note_value,
            ..
        } => {
            if let Some(text) = lab3_note_text(runtime, character_id, note_value) {
                feedback.push((character_id, text));
            }
            *executed += 1;
        }
        // C `lab4_item`'s `if (ch[cn].citem) return;` (`lab4.c:657-659`):
        // a truly silent no-op in C, no `log_char` call - see the item
        // driver's own doc comment.
        ugaris_core::item_driver::ItemDriverOutcome::Lab4FireplaceKeyBlocked { .. } => {
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::Lab4FireplaceKeyGive {
            character_id, ..
        } => {
            if grant_template_item_to_cursor(world, zone_loader, character_id, "lab4_mage_key")
                .is_some()
            {
                // C `log_char(cn, LOG_SYSTEM, 0, "You took the key out of
                // the fire.");` (`lab4.c:662`).
                feedback.push((
                    character_id,
                    "You took the key out of the fire.".to_string(),
                ));
                *executed += 1;
            } else {
                *failed += 1;
            }
        }
        _ => {}
    }
}

/// C `password[][8]` (`src/area/22/lab3.c:876-886`): 70 two-part word
/// pairs `lab3_init_password` picks a random one from.
const LAB3_PASSWORD_PAIRS: [(&str, &str); 70] = [
    ("Gero", "nimo"),
    ("Ban", "zai"),
    ("Yum", "my"),
    ("Jun", "ker"),
    ("Jun", "ction"),
    ("Jun", "gle"),
    ("Ea", "gle"),
    ("Ban", "gle"),
    ("An", "gle"),
    ("An", "gel"),
    ("E", "el"),
    ("He", "el"),
    ("Re", "el"),
    ("Lab", "el"),
    ("Ba", "nd"),
    ("Ba", "nns"),
    ("Seal", "skin"),
    ("Bu", "skin"),
    ("Sheep", "skin"),
    ("Sheep", "ish"),
    ("Era", "sure"),
    ("Era", "sing"),
    ("Era", "ser"),
    ("Sen", "sing"),
    ("Rai", "sing"),
    ("Rai", "der"),
    ("Rai", "son"),
    ("Per", "son"),
    ("Pri", "son"),
    ("Per", "mit"),
    ("Per", "iod"),
    ("Per", "ch"),
    ("Sw", "itch"),
    ("Fet", "ch"),
    ("Wre", "nch"),
    ("Bra", "nch"),
    ("Be", "nch"),
    ("Was", "te"),
    ("Da", "te"),
    ("Te", "st"),
    ("Sum", "moner"),
    ("Sum", "pter"),
    ("Sta", "ck"),
    ("Sta", "ff"),
    ("Sta", "te"),
    ("Gru", "nt"),
    ("Gru", "dge"),
    ("Ti", "bet"),
    ("Gob", "bet"),
    ("Gib", "bet"),
    ("Sor", "bet"),
    ("Sor", "b"),
    ("Sor", "ghum"),
    ("Sc", "um"),
    ("Al", "um"),
    ("Atr", "ium"),
    ("Atr", "ophy"),
    ("Tal", "on"),
    ("Ta", "le"),
    ("Tal", "ker"),
    ("Wa", "sh"),
    ("Tal", "ent"),
    ("In", "tent"),
    ("Stu", "dy"),
    ("Stu", "ff"),
    ("Ti", "me"),
    ("Na", "me"),
    ("Du", "st"),
    ("Al", "to"),
    ("Fra", "me"),
];

/// C `lab3_init_password` (`lab3.c:873-895`): assigns a random password
/// pair only if `password1` isn't already set - the password then
/// persists across every future read (both note-reading and the
/// `CDR_LAB3PASSGUARD` challenge check the same stored value).
fn lab3_init_password(player: &mut PlayerRuntime) {
    if !player.legacy_lab3_password1().is_empty() {
        return;
    }
    let index = runtime_random_below(LAB3_PASSWORD_PAIRS.len() as i32).max(0) as usize
        % LAB3_PASSWORD_PAIRS.len();
    let (part1, part2) = LAB3_PASSWORD_PAIRS[index];
    player.set_legacy_lab3_password1(part1.as_bytes());
    player.set_legacy_lab3_password2(part2.as_bytes());
}

/// C `lab3_special`'s `drdata[0]==3` note-reading switch
/// (`lab3.c:1001-1067`). Cases `1..=6` are canned lore text; `20`/`21`
/// need `lab3_init_password` (reads/writes `PlayerRuntime`'s
/// `legacy_lab3_password1`/`2`) to reveal half of the teleport door's
/// password. Any other value matches C's `default: xlog(...)` branch -
/// no player-visible text, `None`.
fn lab3_note_text(
    runtime: &mut ServerRuntime,
    character_id: CharacterId,
    note_value: u8,
) -> Option<String> {
    match note_value {
        1 => Some(
            "I have to find a way of holding my breath for longer.\u{fffd} Too bad you can\u{fffd}t breathe under water."
                .to_string(),
        ),
        2 => Some(
            "The yellow berries seem to release oxygen. I have finally figured out how to stay underwater for longer. Now I simply need to manage fighting these hordes of crustaceans in order to find the exit to this part of the labyrinth. The exit is supposed to be somewhere in the south."
                .to_string(),
        ),
        3 => Some(
            "Behind the southern caves I discovered a rare brown berry. Encouraged by my experience with the yellow berries I ate it. Nothing much happened, but when I expressed my disappointment, I could understand my own words. Very interesting, might even come in handy."
                .to_string(),
        ),
        4 => Some(
            "In the south I only discovered the entrance to some caves. I will explore them later on, for the time being I just want to find the exit to this part of the labyrinth. It must be further to the east."
                .to_string(),
        ),
        5 => Some(
            "These large crustaceans are too strong, but fortunately very slow.".to_string(),
        ),
        6 => Some(
            "These berries are incredible. When you eat the white ones, you start glowing. Thus I can finally explore the darker regions."
                .to_string(),
        ),
        20 => {
            let player = runtime.player_for_character_mut(character_id)?;
            lab3_init_password(player);
            let password1 = String::from_utf8_lossy(&player.legacy_lab3_password1()).into_owned();
            Some(format!("Thou can read the incomplete word \"{password1}...\"."))
        }
        21 => {
            let player = runtime.player_for_character_mut(character_id)?;
            lab3_init_password(player);
            let password2 = String::from_utf8_lossy(&player.legacy_lab3_password2()).into_owned();
            Some(format!("Thou can read the incomplete word \"...{password2}\"."))
        }
        _ => None,
    }
}
