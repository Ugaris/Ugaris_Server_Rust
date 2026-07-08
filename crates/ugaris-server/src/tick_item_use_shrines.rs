//! Completed-action-outcome handling: the shrine family (`src/area/2/
//! area2.c` zombie-shrine offerings, `src/area/14/random.c` random-shrine
//! kinds, `src/module/base.c` special-shrine/demon-shrine touch handlers)
//! of `ItemDriverOutcome` variants. Split out of the giant `match outcome
//! { ... }` block that still lives inline in `main.rs`'s `tick.tick()` arm
//! (P0.5 "Finish main() phase decomposition" - REMAINING note: the
//! completed-action-outcome handling needs splitting by completed-action-
//! kind family across several files, not just relocation, because the
//! whole match is too large to move verbatim into one file). Warp, chests,
//! dungeon, ice/palace, Teufel, skel-raise, Edemon/Fdemon, transport, and
//! clan-spawn/LQ/arena were sliced first; this is the tenth family slice.
//! The rest of the match (xmas, swamp, burndown, key-assembly) is still
//! inline in `main.rs` pending further slices.
//!
//! Unlike the other slices, several of this family's variants embed a
//! nested match whose innermost arms originally used `continue` to skip to
//! the next queued completed action in the enclosing `for completion in
//! &completed_actions` loop. Since this function is now called once per
//! outcome (not from inside that loop), each `continue` became a `return`
//! - the equivalent "stop processing this outcome" behavior at function
//! scope.

use super::*;

/// C: every successful `shrine_*` function in `random.c`
/// (`shrine_indecisiveness`/`_bribes`/`_welding`/`_edge`/`_kindness`/
/// `_vitality`/`_death`/`_braveness`/`_security`/`_jobless`/`_continuity`)
/// calls `sendquestlog(cn, ch[cn].player)` as its last line, right after
/// `shrine_set` - never on an early-return/blocked path. Reuses the same
/// resend pattern as `mine.rs::apply_military_mission_silver_check`.
fn resend_random_shrine_questlog(runtime: &mut ServerRuntime, character_id: CharacterId) {
    let Some(player) = runtime.player_for_character_mut(character_id) else {
        return;
    };
    let payload = legacy_questlog_payload(player);
    for (session_id, _) in runtime.sessions_for_character(character_id) {
        runtime.send_to_session(session_id, payload.clone());
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn dispatch_shrine_outcome(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    config: &ServerConfig,
    realtime_seconds: u64,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
    failed: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::ZombieShrine {
            item_id,
            character_id,
            shrine_type,
        } => {
            let random_seed =
                world.tick.0 ^ (u64::from(item_id.0) << 16) ^ u64::from(character_id.0);
            match apply_zombie_shrine(
                world,
                zone_loader,
                character_id,
                shrine_type,
                random_seed,
                u32::from(config.area_id),
            ) {
                ZombieShrineApplyResult::Gift(_) => {
                    feedback.push((character_id, "You received a gift.".to_string()));
                    *executed += 1;
                }
                ZombieShrineApplyResult::Experience(_) => {
                    feedback.push((
                        character_id,
                        "You have been blessed with experience.".to_string(),
                    ));
                    *executed += 1;
                }
                ZombieShrineApplyResult::Bonus { message, .. } => {
                    feedback.push((character_id, message.to_string()));
                    *executed += 1;
                }
                ZombieShrineApplyResult::NeedsOffering(shrine_type) => {
                    feedback.push((
                        character_id,
                        zombie_shrine_offering_message(shrine_type).to_string(),
                    ));
                    *blocked += 1;
                }
                ZombieShrineApplyResult::MissingGift => {
                    *failed += 1;
                }
                ZombieShrineApplyResult::MissingPlayer => {
                    *failed += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::ZombieShrineNeedsOffering {
            character_id,
            shrine_type,
            ..
        } => {
            feedback.push((
                character_id,
                zombie_shrine_offering_message(shrine_type).to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::RandomShrineNeedsKey {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "Nothing happens. You seem to need some kind of magical item to invoke the powers of the shrine.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::RandomShrineAlreadyUsed {
            character_id,
            ..
        } => {
            feedback.push((
                character_id,
                "The magic of this place will only work once.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::RandomShrineBug { character_id, .. } => {
            feedback.push((character_id, "You have found bug #2116a.".to_string()));
            *failed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::RandomShrineUse {
            character_id,
            shrine_type,
            level,
            kind,
            ..
        } => match kind {
            ugaris_core::item_driver::RandomShrineKind::Security => {
                let result = match (
                    runtime.player_for_character_mut(character_id),
                    world.characters.get_mut(&character_id),
                ) {
                    (Some(player), Some(character)) => {
                        apply_random_shrine_security(player, character, shrine_type)
                    }
                    _ => {
                        *failed += 1;
                        return;
                    }
                };
                match result {
                    RandomShrineSecurityApplyResult::Used { saves } => {
                        feedback.push((
                            character_id,
                            "A scared voice whispers: 'Thou shalt be secure.'".to_string(),
                        ));
                        feedback.push((
                            character_id,
                            format!(
                                "Thou hast {} save{}.",
                                legacy_save_number(saves),
                                if saves == 1 { "" } else { "s" }
                            ),
                        ));
                        resend_random_shrine_questlog(runtime, character_id);
                        *executed += 1;
                    }
                    RandomShrineSecurityApplyResult::SecureAlready => {
                        feedback.push((
                            character_id,
                            "A scared voice whispers: 'Thou art secure already.'".to_string(),
                        ));
                        *blocked += 1;
                    }
                    RandomShrineSecurityApplyResult::Hardcore => {
                        feedback.push((
                            character_id,
                            "A scared voice whispers: 'Thou wilt never be secure.'".to_string(),
                        ));
                        *blocked += 1;
                    }
                }
            }
            ugaris_core::item_driver::RandomShrineKind::Jobless => {
                let result = match (
                    runtime.player_for_character_mut(character_id),
                    world.characters.get_mut(&character_id),
                ) {
                    (Some(player), Some(character)) => {
                        apply_random_shrine_jobless(player, character, shrine_type)
                    }
                    _ => {
                        *failed += 1;
                        return;
                    }
                };
                match result {
                    RandomShrineJoblessApplyResult::Used => {
                        feedback.push((
                            character_id,
                            "A bored voice says: 'Thou shalt be jobless.'".to_string(),
                        ));
                        resend_random_shrine_questlog(runtime, character_id);
                        *executed += 1;
                    }
                    RandomShrineJoblessApplyResult::AlreadyJobless => {
                        feedback.push((
                            character_id,
                            "A bored voice says: 'Thou art jobless already.'".to_string(),
                        ));
                        *blocked += 1;
                    }
                }
            }
            ugaris_core::item_driver::RandomShrineKind::Edge => {
                let result = match (
                    runtime.player_for_character_mut(character_id),
                    world.characters.get_mut(&character_id),
                ) {
                    (Some(player), Some(character)) => {
                        apply_random_shrine_edge(player, character, shrine_type, level)
                    }
                    _ => {
                        *failed += 1;
                        return;
                    }
                };
                match result {
                    RandomShrineEdgeApplyResult::Used { exp } => {
                        // C `shrine_edge` (`random.c:2038`) grants
                        // `bonus` via `give_exp(cn, bonus)`.
                        world.give_exp(character_id, i64::from(exp), u32::from(config.area_id));
                        feedback.push((
                            character_id,
                            "A booming voice declares: 'Living on the edge has its merits - and its dangers!'".to_string(),
                        ));
                        feedback.push((character_id, "Thou hast no saves left.".to_string()));
                        resend_random_shrine_questlog(runtime, character_id);
                        *executed += 1;
                    }
                    RandomShrineEdgeApplyResult::AlreadyOnEdge => {
                        feedback.push((
                            character_id,
                            "A booming voice declares: 'Thou art living on the edge already!'"
                                .to_string(),
                        ));
                        *blocked += 1;
                    }
                    RandomShrineEdgeApplyResult::NoExp => {
                        feedback.push((
                            character_id,
                            "A deadly voice says: 'Thou canst live on the edge as long as thou has /noexp turned on.'".to_string(),
                        ));
                        *blocked += 1;
                    }
                }
            }
            ugaris_core::item_driver::RandomShrineKind::Kindness => {
                let result = match (
                    runtime.player_for_character_mut(character_id),
                    world.characters.get_mut(&character_id),
                ) {
                    (Some(player), Some(character)) => {
                        apply_random_shrine_kindness(player, character, shrine_type)
                    }
                    _ => {
                        *failed += 1;
                        return;
                    }
                };
                match result {
                    RandomShrineKindnessApplyResult::Used => {
                        feedback.push((
                            character_id,
                            "A tender voice whispers: 'Mayest thou find other ways to amuse thyself. Thou art not a killer henceforth.'".to_string(),
                        ));
                        resend_random_shrine_questlog(runtime, character_id);
                        *executed += 1;
                    }
                    RandomShrineKindnessApplyResult::AlreadyKind => {
                        feedback.push((
                            character_id,
                            "A tender voice whispers: 'But thou art a kind soul already...'"
                                .to_string(),
                        ));
                        *blocked += 1;
                    }
                }
            }
            ugaris_core::item_driver::RandomShrineKind::Death => {
                match runtime.player_for_character_mut(character_id) {
                    Some(player) => player.mark_random_shrine_used(shrine_type),
                    None => {
                        *failed += 1;
                        return;
                    }
                }
                if let Some(character) = world.characters.get_mut(&character_id) {
                    character.saves = 0;
                }
                resend_random_shrine_questlog(runtime, character_id);
                feedback.push((character_id, "You hear a manical laugh.".to_string()));
                world.apply_legacy_hurt(character_id, None, i32::MAX / 4, 1, 100, 100);
                *executed += 1;
            }
            ugaris_core::item_driver::RandomShrineKind::Vitality => {
                let result = match (
                    runtime.player_for_character_mut(character_id),
                    world.characters.get_mut(&character_id),
                ) {
                    (Some(player), Some(character)) => {
                        apply_random_shrine_vitality(player, character, shrine_type)
                    }
                    _ => {
                        *failed += 1;
                        return;
                    }
                };
                match result {
                    RandomShrineVitalityApplyResult::Used { cost, .. } => {
                        // C `shrine_vitality` (`random.c:2109-2110`)
                        // grants `cost` via `give_exp(cn, cost)` then
                        // `update_char(cn)`.
                        world.give_exp(character_id, i64::from(cost), u32::from(config.area_id));
                        world.update_character(character_id);
                        resend_random_shrine_questlog(runtime, character_id);
                        *executed += 1;
                    }
                    RandomShrineVitalityApplyResult::NoExp => {
                        feedback.push((
                            character_id,
                            "A lively voice says: 'Thou canst improve thine vitality any more as long as thou has /noexp turned on.'".to_string(),
                        ));
                        *blocked += 1;
                    }
                    RandomShrineVitalityApplyResult::Capped => {
                        feedback.push((
                            character_id,
                            "A lively voice says: 'Thou canst improve thine vitality any more.'"
                                .to_string(),
                        ));
                        *blocked += 1;
                    }
                }
            }
            ugaris_core::item_driver::RandomShrineKind::Braveness => {
                let result = match (
                    runtime.player_for_character_mut(character_id),
                    world.characters.get_mut(&character_id),
                ) {
                    (Some(player), Some(character)) => {
                        apply_random_shrine_braveness(player, character, shrine_type, level)
                    }
                    _ => {
                        *failed += 1;
                        return;
                    }
                };
                match result {
                    RandomShrineBravenessApplyResult::Used { exp, .. } => {
                        // C `shrine_braveness` (`random.c:2193`) grants
                        // `cost` via `give_exp(cn, cost)`.
                        world.give_exp(character_id, i64::from(exp), u32::from(config.area_id));
                        feedback.push((
                            character_id,
                            "A triumphant voice says: 'Thou art brave indeed!'".to_string(),
                        ));
                        resend_random_shrine_questlog(runtime, character_id);
                        *executed += 1;
                    }
                    RandomShrineBravenessApplyResult::Coward => {
                        feedback.push((
                            character_id,
                            "An insulting voice says: 'Thou art a coward, bother me not!"
                                .to_string(),
                        ));
                        *blocked += 1;
                    }
                }
            }
            ugaris_core::item_driver::RandomShrineKind::Continuity => {
                let result = match (
                    runtime.player_for_character_mut(character_id),
                    world.characters.get_mut(&character_id),
                ) {
                    (Some(player), Some(character)) => {
                        apply_random_shrine_continuity(player, character, level)
                    }
                    _ => {
                        *failed += 1;
                        return;
                    }
                };
                match result {
                    RandomShrineContinuityApplyResult::Used { exp, opens_gate } => {
                        // C `shrine_continuity` (`random.c:2154`) grants
                        // `cost` via `give_exp(cn, cost)` before the
                        // level-99 gate teleport.
                        world.give_exp(character_id, i64::from(exp), u32::from(config.area_id));
                        feedback.push((
                            character_id,
                            "A steady voice says: 'Continuity is power.'".to_string(),
                        ));
                        if opens_gate {
                            if world.teleport_character_same_area(character_id, 41, 250, false) {
                                feedback.push((
                                    character_id,
                                    "Thy continuity has opened a gate...".to_string(),
                                ));
                            } else {
                                feedback.push((
                                    character_id,
                                    "Target is busy, please try again soon.".to_string(),
                                ));
                            }
                        }
                        resend_random_shrine_questlog(runtime, character_id);
                        *executed += 1;
                    }
                    RandomShrineContinuityApplyResult::AlreadyVisited { opens_gate } => {
                        if opens_gate {
                            if world.teleport_character_same_area(character_id, 41, 250, false) {
                                feedback.push((
                                    character_id,
                                    "Thy continuity has opened a gate...".to_string(),
                                ));
                            } else {
                                feedback.push((
                                    character_id,
                                    "Target is busy, please try again soon.".to_string(),
                                ));
                            }
                        } else {
                            feedback.push((
                                character_id,
                                "A steady voice says: 'Thou hast visited me already.'".to_string(),
                            ));
                        }
                        *blocked += 1;
                    }
                    RandomShrineContinuityApplyResult::NeedYoungerBrother => {
                        feedback.push((
                            character_id,
                            "A steady voice says: 'Thou must visit mine younger brother first.'"
                                .to_string(),
                        ));
                        *blocked += 1;
                    }
                }
            }
            ugaris_core::item_driver::RandomShrineKind::Indecisiveness => {
                let result = match (
                    runtime.player_for_character_mut(character_id),
                    world.characters.get_mut(&character_id),
                ) {
                    (Some(player), Some(character)) => {
                        apply_random_shrine_indecisiveness(player, character, shrine_type)
                    }
                    _ => {
                        *failed += 1;
                        return;
                    }
                };
                match result {
                    RandomShrineIndecisivenessApplyResult::Used => {
                        resend_random_shrine_questlog(runtime, character_id);
                        *executed += 1;
                    }
                    RandomShrineIndecisivenessApplyResult::NoExp => {
                        feedback.push((
                            character_id,
                            "A indecisive voice says: 'Thou canst lower thy skills as long as thou has /noexp turned on.'".to_string(),
                        ));
                        *blocked += 1;
                    }
                }
            }
            ugaris_core::item_driver::RandomShrineKind::Bribes => {
                let result = match (
                    runtime.player_for_character_mut(character_id),
                    world.characters.get_mut(&character_id),
                ) {
                    (Some(player), Some(character)) => {
                        apply_random_shrine_bribes(player, character, shrine_type, level)
                    }
                    _ => {
                        *failed += 1;
                        return;
                    }
                };
                match result {
                    RandomShrineBribesApplyResult::Used {
                        gold: _,
                        exp,
                        almost_empty,
                    } => {
                        // C `shrine_bribes` (`random.c:1836`) grants
                        // `val / 4` via `give_exp(cn, val / 4)`.
                        world.give_exp(character_id, i64::from(exp), u32::from(config.area_id));
                        feedback.push((
                            character_id,
                            "You feel a hand reach into your pocket and touch your purse."
                                .to_string(),
                        ));
                        feedback.push((
                            character_id,
                            format!(
                                "Shocked, you reach for your purse and find it {}empty.",
                                if almost_empty { "almost " } else { "" }
                            ),
                        ));
                        resend_random_shrine_questlog(runtime, character_id);
                        *executed += 1;
                    }
                    RandomShrineBribesApplyResult::NoExp => {
                        feedback.push((
                            character_id,
                            "A golden voice says: 'Thou canst bribe for more experience as long as thou has /noexp turned on.'".to_string(),
                        ));
                        *blocked += 1;
                    }
                    RandomShrineBribesApplyResult::NotEnoughGold => {
                        feedback.push((
                            character_id,
                            "You feel a hand reach into your pocket and touch your purse. A second later, it is removed with a sneer.".to_string(),
                        ));
                        *blocked += 1;
                    }
                }
            }
            ugaris_core::item_driver::RandomShrineKind::Dormant => {
                *executed += 1;
            }
            ugaris_core::item_driver::RandomShrineKind::Welding => {
                match world.apply_random_shrine_welding(character_id, level) {
                    ugaris_core::world::RandomShrineWeldingResult::Used {
                        item1_name,
                        item2_name,
                    } => {
                        if let Some(player) = runtime.player_for_character_mut(character_id) {
                            player.mark_random_shrine_used(shrine_type);
                        }
                        feedback.push((
                            character_id,
                            format!(
                                "You feel a burning hand touch your {item1_name} and your {item2_name}."
                            ),
                        ));
                        resend_random_shrine_questlog(runtime, character_id);
                        *executed += 1;
                    }
                    ugaris_core::world::RandomShrineWeldingResult::NotPowerfulEnough => {
                        feedback.push((
                            character_id,
                            "You are not powerful enough to use this shrine.".to_string(),
                        ));
                        *blocked += 1;
                    }
                    ugaris_core::world::RandomShrineWeldingResult::NotPaying => {
                        feedback.push((
                            character_id,
                            "Only paying players can use this shrine.".to_string(),
                        ));
                        *blocked += 1;
                    }
                    ugaris_core::world::RandomShrineWeldingResult::Contempt => {
                        feedback.push((
                            character_id,
                            "You feel a cold hand touch your equipment. After it has touched all your items, it leaves with a laugh of contempt.".to_string(),
                        ));
                        *blocked += 1;
                    }
                    ugaris_core::world::RandomShrineWeldingResult::Regret => {
                        feedback.push((
                            character_id,
                            "You feel a cold hand touch your equipment. After it has touched all your items, it leaves with a laugh of regret.".to_string(),
                        ));
                        *blocked += 1;
                    }
                    ugaris_core::world::RandomShrineWeldingResult::Bug => {
                        feedback.push((character_id, "You found bug #337.".to_string()));
                        *failed += 1;
                    }
                }
            }
        },
        ugaris_core::item_driver::ItemDriverOutcome::SpecialShrine {
            character_id, kind, ..
        } => {
            let result = match (
                runtime.player_for_character_mut(character_id),
                world.characters.get_mut(&character_id),
            ) {
                (Some(player), Some(character)) => {
                    player.touch_special_shrine(character, kind, realtime_seconds)
                }
                _ => {
                    *failed += 1;
                    return;
                }
            };
            match result {
                ugaris_core::player::SpecialShrineResult::NothingHere => {
                    feedback.push((
                        character_id,
                        "A mild voice speaks: There is nothing for thee here.".to_string(),
                    ));
                    *blocked += 1;
                }
                ugaris_core::player::SpecialShrineResult::ConfirmRequired => {
                    feedback.push((
                        character_id,
                        "A mild voice says: I can remove the perils of living on the edge from thee. If this is your wish, touch me again.".to_string(),
                    ));
                    *blocked += 1;
                }
                ugaris_core::player::SpecialShrineResult::HardcoreRemoved => {
                    feedback.push((
                        character_id,
                        "A mild voice speaks: Thou art no longer living on the edge, Ishtar will again save thee when thou art in need. The benefits of a hardcore character shant be thine any more.".to_string(),
                    ));
                    *executed += 1;
                }
                ugaris_core::player::SpecialShrineResult::Unsupported => {
                    *blocked += 1;
                }
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::DemonShrine {
            character_id,
            location_id,
            ..
        } => {
            let result = match (
                runtime.player_for_character_mut(character_id),
                world.characters.get_mut(&character_id),
            ) {
                (Some(player), Some(character)) => player.touch_demonshrine(character, location_id),
                _ => {
                    *failed += 1;
                    return;
                }
            };
            match result {
                DemonShrineResult::Learned { exp_added } => {
                    // C `demonshrine_driver` (`base.c:3231-3235`):
                    // `update_char(cn)` after the Demon value bump,
                    // then `give_exp(cn, ...)`.
                    world.update_character(character_id);
                    world.give_exp(
                        character_id,
                        i64::from(exp_added),
                        u32::from(config.area_id),
                    );
                    feedback.push((
                        character_id,
                        "You study the old book and learn something about the ancient tribes. Your Ancient Knowledge went up by one and you gained experience.".to_string(),
                    ));
                    *executed += 1;
                }
                DemonShrineResult::AlreadyKnown => {
                    feedback.push((
                        character_id,
                        "You've been here before. You cannot learn more from this book."
                            .to_string(),
                    ));
                    *blocked += 1;
                }
                DemonShrineResult::Full => {
                    feedback.push((character_id, "Bug 771".to_string()));
                    *failed += 1;
                }
            }
        }
        _ => {}
    }
}
