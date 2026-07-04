//! Server-side wiring for `ugaris-core`'s military mission-progress model
//! (`crate::world::MilitaryMissionKillCheck` / `PlayerRuntime::
//! check_military_solve`): drains the queue `World::kill_character_
//! followup` fills on every player kill and applies the resulting
//! `check_military_solve` (`src/system/death.c:290-383`) text feedback.
//!
//! Also wires the `CDR_MILITARY_MASTER` NPC driver
//! ([`apply_military_master_events`], see `ugaris-core`'s `world/
//! military.rs` sixth-slice doc comment for the `World`/`PlayerRuntime`
//! split this mirrors from `apply_bank_events`).

use super::*;
use ugaris_core::world::{
    army_rank_for_points, army_rank_name, display_mission_text,
    military_mission_progress_message_should_display, offer_missions_text, AcceptMissionOutcome,
    GreetPlayerOutcome, MilitaryMasterEvent, MilitaryMissionKillCheck, MilitaryMissionProgress,
    MissionRequestOutcome, MissionRerollOutcome, SingleMission,
};

/// C `check_military_solve(co, cn)`'s killer-side (`co`, `check.killer_id`
/// here) mission-progress update, queued as a [`MilitaryMissionKillCheck`]
/// by `World::kill_character_followup` for every kill by a player
/// character. A no-op if the killer has no live `PlayerRuntime`, or if
/// [`ugaris_core::PlayerRuntime::check_military_solve`] reports
/// [`MilitaryMissionProgress::NoMatch`] (no active unsolved mission, or
/// the victim didn't match its type/class/level target).
pub(crate) fn apply_military_mission_kill_check(
    world: &mut World,
    runtime: &mut ServerRuntime,
    check: MilitaryMissionKillCheck,
) {
    let Some(player) = runtime.player_for_character_mut(check.killer_id) else {
        return;
    };
    let outcome = player.check_military_solve(check.victim_class, check.victim_level as i32);

    let message: Option<Vec<u8>> = match outcome {
        MilitaryMissionProgress::NoMatch => None,
        MilitaryMissionProgress::Progress {
            remaining,
            elite_count,
        } => {
            if !military_mission_progress_message_should_display(remaining) {
                None
            } else {
                let mut line = COL_DARK_GRAY.to_vec();
                if elite_count > 1 {
                    // C: `log_char(cn, LOG_SYSTEM, 0, COL_DARK_GRAY "Elite
                    // demon slain! Counts as %d. %d to go.", count_value,
                    // ppd->mis[nr].opt1)` (`death.c:343-344`).
                    line.extend_from_slice(
                        format!("Elite demon slain! Counts as {elite_count}. {remaining} to go.")
                            .as_bytes(),
                    );
                } else {
                    // C: `log_char(cn, LOG_SYSTEM, 0, COL_DARK_GRAY
                    // "Mission kill, %d to go.", ppd->mis[nr].opt1)`
                    // (`death.c:346` / `:371`).
                    line.extend_from_slice(format!("Mission kill, {remaining} to go.").as_bytes());
                }
                Some(line)
            }
        }
        MilitaryMissionProgress::Solved => {
            // C: `log_char(cn, LOG_SYSTEM, 0, "You solved your mission.
            // Talk to the governor to claim your reward.")` (`death.c:
            // 350-351` / `:374-375`) - no color prefix on this one.
            Some(b"You solved your mission. Talk to the governor to claim your reward.".to_vec())
        }
    };

    if let Some(message) = message {
        world.queue_system_text_bytes(check.killer_id, message);
    }
}

/// C `military_master_driver`'s message-handling body (`src/module/
/// military.c:2108-2206`), applying each [`MilitaryMasterEvent`] queued
/// by `World::process_military_master_actions` (see `ugaris-core`'s
/// `world/military.rs` sixth-slice doc comment for why nearly every
/// branch needs `PlayerRuntime`, mirroring `apply_bank_events`'s shape).
pub(crate) fn apply_military_master_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    area_id: u16,
) -> usize {
    let mut applied = 0;
    for event in world.drain_pending_military_master_events() {
        match event {
            MilitaryMasterEvent::NearbyPlayer {
                master_id,
                player_id,
            } => {
                if apply_military_master_nearby_player(
                    world, runtime, master_id, player_id, area_id,
                ) {
                    applied += 1;
                }
            }
            MilitaryMasterEvent::Repeat { player_id, .. } => {
                if let Some(player) = runtime.player_for_character_mut(player_id) {
                    // C qa code 2 ("repeat"): `ppd->master_state = 0;`,
                    // no text (`military.c:1989-1991`).
                    player.set_master_state(0);
                    applied += 1;
                }
            }
            MilitaryMasterEvent::MissionRequest {
                master_id,
                player_id,
            } => {
                if apply_military_master_mission_request(world, runtime, master_id, player_id) {
                    applied += 1;
                }
            }
            MilitaryMasterEvent::AcceptMission {
                master_id,
                player_id,
                difficulty,
            } => {
                if apply_military_master_accept_mission(
                    world, runtime, master_id, player_id, difficulty,
                ) {
                    applied += 1;
                }
            }
            MilitaryMasterEvent::Failed {
                master_id,
                player_id,
            } => {
                if apply_military_master_failed(world, runtime, master_id, player_id) {
                    applied += 1;
                }
            }
            MilitaryMasterEvent::Hear {
                master_id,
                player_id,
            } => {
                if apply_military_master_hear(world, runtime, master_id, player_id) {
                    applied += 1;
                }
            }
            MilitaryMasterEvent::Reroll {
                master_id,
                player_id,
            } => {
                if apply_military_master_reroll(world, runtime, master_id, player_id) {
                    applied += 1;
                }
            }
        }
    }
    applied
}

/// C `military_master_driver`'s `NT_CHAR` branch (`military.c:2153-2177`,
/// minus the still-unported clan/advisor recommendation calls - see this
/// module's parent doc comment): [`crate::PlayerRuntime::greet_player`],
/// the `master_state == 1` rank-follow-up text, and
/// [`World::complete_mission`].
fn apply_military_master_nearby_player(
    world: &mut World,
    runtime: &mut ServerRuntime,
    master_id: CharacterId,
    player_id: CharacterId,
    area_id: u16,
) -> bool {
    let yday = world.date.yday as i32;
    let Some(player_name) = world.characters.get(&player_id).map(|c| c.name.clone()) else {
        return false;
    };
    let has_army_rank = world
        .characters
        .get(&player_id)
        .is_some_and(|character| army_rank_for_points(character.military_points) > 0);
    let Some(player) = runtime.player_for_character_mut(player_id) else {
        return false;
    };

    match player.greet_player(has_army_rank, yday) {
        GreetPlayerOutcome::AlreadyGreeted
        | GreetPlayerOutcome::AdvisorRecommendationAlreadyShown => {}
        GreetPlayerOutcome::HasActiveMission => {
            world.npc_quiet_say(
                master_id,
                &format!(
                    "Ah, hello {player_name}. Any luck with your mission? Or would you like to \
                     hear it again? Or have you failed to complete it?"
                ),
            );
        }
        GreetPlayerOutcome::AlreadyCompletedToday => {
            world.npc_quiet_say(
                master_id,
                &format!("I don't have another mission for you today, {player_name}."),
            );
        }
        GreetPlayerOutcome::HasRank => {
            world.npc_quiet_say(
                master_id,
                &format!(
                    "Hello, {player_name}. I might have a mission for you. If you don't like \
                     the available missions, you can reroll for 200 gold."
                ),
            );
        }
        GreetPlayerOutcome::NewPlayer => {
            world.npc_quiet_say(master_id, &format!("Greetings, {player_name}."));
        }
    }

    // C `military_master_driver`'s `master_state == 1` rank-follow-up
    // (`military.c:2172-2176`): the player was greeted as a new recruit
    // last visit but has since gained an army rank elsewhere.
    if player.master_state() == 1 && has_army_rank {
        world.npc_quiet_say(
            master_id,
            &format!("Hello again, {player_name}. I might have a mission for you."),
        );
        player.set_master_state(2);
    }

    // C `complete_mission`'s own reward text already goes through
    // `World::queue_system_text`/`queue_system_text_bytes` (see that
    // function's doc comment) rather than `npc_quiet_say` from this NPC -
    // a pre-existing simplification, not tightened here.
    let _ = world.complete_mission(player_id, player, u32::from(area_id));

    true
}

/// C qa code 10 ("mission"): [`World::handle_mission_request`].
fn apply_military_master_mission_request(
    world: &mut World,
    runtime: &mut ServerRuntime,
    master_id: CharacterId,
    player_id: CharacterId,
) -> bool {
    let yday = world.date.yday as i32;
    let Some(player_name) = world.characters.get(&player_id).map(|c| c.name.clone()) else {
        return false;
    };
    let Some(player) = runtime.player_for_character_mut(player_id) else {
        return false;
    };

    let mut rng_seed = world.legacy_random_seed;
    let outcome =
        world.handle_mission_request(player_id, player, yday, &mut rng_seed, &player_name);
    world.legacy_random_seed = rng_seed;

    match outcome {
        MissionRequestOutcome::AlreadyHasMission => {
            world.npc_quiet_say(
                master_id,
                "You already have a mission. Would you like to hear it again?",
            );
        }
        MissionRequestOutcome::AlreadyCompletedToday => {
            world.npc_quiet_say(
                master_id,
                &format!("I don't have another mission for you today, {player_name}."),
            );
        }
        MissionRequestOutcome::NotEnrolled => {
            world.npc_quiet_say(
                master_id,
                &format!(
                    "But you don't even belong to the army, {player_name}. Talk to Seymour \
                     about enrollment."
                ),
            );
        }
        MissionRequestOutcome::AdvisorRecommendation {
            description,
            prompt,
        } => {
            world.npc_quiet_say(master_id, &description);
            world.npc_quiet_say(master_id, &prompt);
        }
        MissionRequestOutcome::Offered(lines) => {
            for line in lines {
                world.npc_quiet_say(master_id, &line);
            }
        }
    }
    true
}

/// C qa codes 11-15 ("easy".."insane"): [`crate::PlayerRuntime::
/// accept_mission`].
fn apply_military_master_accept_mission(
    world: &mut World,
    runtime: &mut ServerRuntime,
    master_id: CharacterId,
    player_id: CharacterId,
    difficulty: usize,
) -> bool {
    let yday = world.date.yday as i32;
    let Some(player_name) = world.characters.get(&player_id).map(|c| c.name.clone()) else {
        return false;
    };
    let Some(player) = runtime.player_for_character_mut(player_id) else {
        return false;
    };

    match player.accept_mission(difficulty, yday) {
        AcceptMissionOutcome::AlreadyHasMission => {
            world.npc_quiet_say(
                master_id,
                &format!(
                    "You already have a mission, {player_name}. Would you like to hear it again?"
                ),
            );
        }
        AcceptMissionOutcome::AlreadyCompletedToday => {
            world.npc_quiet_say(
                master_id,
                &format!("I don't have another mission for you today, {player_name}."),
            );
        }
        AcceptMissionOutcome::MissionsNotOfferedToday => {
            world.npc_quiet_say(
                master_id,
                &format!("I haven't offered you that kind of mission today, {player_name}."),
            );
        }
        AcceptMissionOutcome::InsufficientPoints => {
            world.npc_quiet_say(
                master_id,
                &format!("I have not offered you that kind of mission, {player_name}."),
            );
        }
        AcceptMissionOutcome::MissionUnavailable => {
            world.npc_quiet_say(
                master_id,
                &format!("I'm sorry, {player_name}, but that mission is not available."),
            );
        }
        AcceptMissionOutcome::Accepted(mission) => {
            let text = display_mission_text(&mission).unwrap_or_else(|| {
                format!("I'm sorry, {player_name}, but that mission is not available.")
            });
            world.npc_quiet_say(master_id, &text);
        }
    }
    true
}

/// C qa code 16 ("failed"): abandon the active mission.
fn apply_military_master_failed(
    world: &mut World,
    runtime: &mut ServerRuntime,
    master_id: CharacterId,
    player_id: CharacterId,
) -> bool {
    let Some(character) = world.characters.get(&player_id) else {
        return false;
    };
    let player_name = character.name.clone();
    let rank_name = army_rank_name(army_rank_for_points(character.military_points)).to_string();
    let Some(player) = runtime.player_for_character_mut(player_id) else {
        return false;
    };

    if player.military_took_mission() == 0 {
        // C: `say(cn, "But you did not take any mission, %s.",
        // ch[co].name);` - this particular branch uses the player's own
        // name, unlike the branch below.
        world.npc_quiet_say(
            master_id,
            &format!("But you did not take any mission, {player_name}."),
        );
        return true;
    }

    // C: `say(cn, "So, you failed? ...", get_army_rank_string(co));` -
    // this branch substitutes the army rank *title*, not the player's
    // name (a genuine C quirk, preserved verbatim).
    world.npc_quiet_say(
        master_id,
        &format!(
            "So, you failed? Well, {rank_name}, I'll remove that mission from your record. \
             Would you like to get another mission?"
        ),
    );
    player.set_military_took_mission(0);
    true
}

/// C qa code 17 ("hear"): repeat the active mission's description.
fn apply_military_master_hear(
    world: &mut World,
    runtime: &mut ServerRuntime,
    master_id: CharacterId,
    player_id: CharacterId,
) -> bool {
    let Some(character) = world.characters.get(&player_id) else {
        return false;
    };
    let player_name = character.name.clone();
    let rank_name = army_rank_name(army_rank_for_points(character.military_points)).to_string();
    let Some(player) = runtime.player_for_character_mut(player_id) else {
        return false;
    };

    let took_mission = player.military_took_mission();
    if took_mission == 0 {
        // C: `say(cn, "But you do not have a mission yet, %s.",
        // get_army_rank_string(co));` - substitutes the army rank title,
        // same quirk as the "failed" branch above.
        world.npc_quiet_say(
            master_id,
            &format!("But you do not have a mission yet, {rank_name}."),
        );
        return true;
    }

    let difficulty = (took_mission - 1).clamp(0, 4) as usize;
    let mission = player.military_mission(difficulty);
    let text = display_mission_text(&mission)
        .unwrap_or_else(|| format!("I'm sorry, {player_name}, but that mission is not available."));
    world.npc_quiet_say(master_id, &text);
    true
}

/// C qa codes 22/"decline"/"new missions": [`World::mission_reroll`].
fn apply_military_master_reroll(
    world: &mut World,
    runtime: &mut ServerRuntime,
    master_id: CharacterId,
    player_id: CharacterId,
) -> bool {
    let yday = world.date.yday as i32;
    let Some(player_name) = world.characters.get(&player_id).map(|c| c.name.clone()) else {
        return false;
    };
    let Some(player) = runtime.player_for_character_mut(player_id) else {
        return false;
    };

    let mut rng_seed = world.legacy_random_seed;
    let outcome = world.mission_reroll(player_id, player, yday, &mut rng_seed);
    world.legacy_random_seed = rng_seed;

    match outcome {
        MissionRerollOutcome::AlreadyRerolledToday => {
            world.npc_quiet_say(
                master_id,
                &format!(
                    "I've already offered you a different set of missions today, \
                     {player_name}. Come back tomorrow if you want more options."
                ),
            );
        }
        MissionRerollOutcome::HasActiveMission => {
            world.npc_quiet_say(
                master_id,
                &format!(
                    "You already accepted a mission, {player_name}. You must either complete \
                     it or report your failure before requesting new missions."
                ),
            );
        }
        MissionRerollOutcome::InsufficientGold => {
            world.npc_quiet_say(
                master_id,
                &format!(
                    "Generating new mission plans costs 200 gold, {player_name}, which you \
                     don't seem to have."
                ),
            );
        }
        MissionRerollOutcome::ConfirmationRequested => {
            world.npc_quiet_say(
                master_id,
                &format!(
                    "I can prepare a different set of missions for you, {player_name}, but it \
                     will cost 200 gold. Say reroll again to confirm."
                ),
            );
        }
        MissionRerollOutcome::Rerolled => {
            world.npc_quiet_say(
                master_id,
                &format!("Very well, {player_name}. Here are your new mission options:"),
            );
            if let Some(player) = runtime.player_for_character(player_id) {
                let missions: [SingleMission; 5] =
                    std::array::from_fn(|i| player.military_mission(i));
                let lines =
                    offer_missions_text(&missions, player.military_current_pts(), &player_name);
                for line in lines {
                    world.npc_quiet_say(master_id, &line);
                }
            }
        }
    }
    true
}
