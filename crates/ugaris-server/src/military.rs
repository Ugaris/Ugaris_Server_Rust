//! Server-side wiring for `ugaris-core`'s military mission-progress model
//! (`crate::world::MilitaryMissionKillCheck` / `PlayerRuntime::
//! check_military_solve`): drains the queue `World::kill_character_
//! followup` fills on every player kill and applies the resulting
//! `check_military_solve` (`src/system/death.c:290-383`) text feedback.
//!
//! Also wires the `CDR_MILITARY_MASTER` NPC driver
//! ([`apply_military_master_events`], see `ugaris-core`'s `world/
//! military.rs` sixth-slice doc comment for the `World`/`PlayerRuntime`
//! split this mirrors from `apply_bank_events`), and the
//! `CDR_MILITARY_ADVISOR` NPC driver ([`apply_military_advisor_events`],
//! see `ugaris-core`'s `world/military.rs` seventh-slice doc comment -
//! same shape).

use super::*;
use ugaris_core::world::{
    adv_favor_desc_lines, adv_introduction_text, army_rank_for_points, army_rank_name,
    calculate_advisor_index, display_mission_text, favor_size_name,
    military_mission_progress_message_should_display, mission_difficulty_name, mission_type_name,
    offer_missions_text, AcceptMissionOutcome, AdvisorRecommendationOutcome, GreetPlayerOutcome,
    MilitaryAdvisorEvent, MilitaryMasterEvent, MilitaryMissionKillCheck, MilitaryMissionProgress,
    MissionRequestOutcome, MissionRerollOutcome, OfferFavorOutcome, ProcessFavorPaymentOutcome,
    SingleMission, SpecificMissionRequestOutcome,
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
            MilitaryMasterEvent::Info {
                master_id,
                player_id,
            } => {
                if apply_military_master_info(world, runtime, master_id, player_id) {
                    applied += 1;
                }
            }
            MilitaryMasterEvent::Reset { player_id } => {
                if apply_military_master_reset(runtime, player_id) {
                    applied += 1;
                }
            }
            MilitaryMasterEvent::Raise { player_id } => {
                if apply_military_master_raise(runtime, player_id) {
                    applied += 1;
                }
            }
            MilitaryMasterEvent::Promote {
                master_id,
                player_id,
            } => {
                if apply_military_master_promote(world, runtime, master_id, player_id, area_id) {
                    applied += 1;
                }
            }
        }
    }
    applied
}

/// C `military_master_driver`'s `NT_CHAR` branch (`military.c:2153-2177`):
/// [`World::process_clan_recommendation`],
/// [`World::process_advisor_recommendation`],
/// [`crate::PlayerRuntime::greet_player`], the `master_state == 1`
/// rank-follow-up text, and [`World::complete_mission`].
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

    // C `process_clan_recommendation(cn, co, ppd, dat)` (`military.c:
    // 1654-1674`), called right before `process_advisor_recommendation`
    // in C's own `NT_CHAR` handler.
    if let Some(greeting) =
        world.process_clan_recommendation(master_id, player_id, player, &player_name)
    {
        world.npc_quiet_say(master_id, &greeting);
    }

    // C `process_advisor_recommendation(cn, co, ppd)` (`military.c:
    // 1685-1755`), called right before `greet_player` in C's own
    // `NT_CHAR` handler.
    let mut rng_seed = world.legacy_random_seed;
    match world.process_advisor_recommendation(player_id, player, yday, &mut rng_seed, &player_name)
    {
        AdvisorRecommendationOutcome::AlreadyProcessed => {}
        AdvisorRecommendationOutcome::SpecificMission {
            greeting,
            description,
            followup,
        } => {
            world.npc_quiet_say(master_id, &greeting);
            if let Some(description) = description {
                world.npc_quiet_say(master_id, &description);
            }
            world.npc_quiet_say(master_id, &followup);
        }
        AdvisorRecommendationOutcome::StandardRecommendations(lines) => {
            for line in lines {
                world.npc_quiet_say(master_id, &line);
            }
        }
    }
    world.legacy_random_seed = rng_seed;

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
    let _ = world.complete_mission(player_id, player, u32::from(area_id), master_id);

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
            world.record_mission_offered(master_id, difficulty);
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

/// C qa code 18 ("info", admin-only, `military.c:2037-2059`): the
/// speaker's own `military_pts`/`normal_exp`, then this master NPC's
/// storage-scoped clan points (`clan_pts[1..32]`, only nonzero entries)
/// and per-difficulty quest statistics (`quests_given[n] > 0` gate),
/// each rendered as its own `say()` line via `npc_quiet_say`.
fn apply_military_master_info(
    world: &mut World,
    runtime: &mut ServerRuntime,
    master_id: CharacterId,
    player_id: CharacterId,
) -> bool {
    let Some(player) = runtime.player_for_character(player_id) else {
        return false;
    };
    let pts = player.military_pts();
    let exp = player.military_normal_exp_ppd();

    let Some(CharacterDriverState::MilitaryMaster(data)) = world
        .characters
        .get(&master_id)
        .and_then(|c| c.driver_state.as_ref())
    else {
        return false;
    };
    let storage_id = data.storage_id;

    world.npc_quiet_say(
        master_id,
        &format!("You have {pts} pts and you have gained {exp} exp."),
    );

    for clan_nr in 1..ugaris_core::clan::MAX_CLAN as u16 {
        let clan_pts = world.military_master_storage.clan_pts(storage_id, clan_nr);
        if clan_pts != 0 {
            world.npc_quiet_say(master_id, &format!("Clan {clan_nr} has {clan_pts} pts"));
        }
    }

    for difficulty in 0..5usize {
        let (given, solved, exp_given, _pts_given) = world
            .military_master_storage
            .quest_stats(storage_id, difficulty);
        if given > 0 {
            let solve_rate = 100.0 * f64::from(solved) / f64::from(given);
            let avg_exp = if solved > 0 {
                f64::from(exp_given) / f64::from(solved)
            } else {
                0.0
            };
            let diff_name = mission_difficulty_name(difficulty);
            world.npc_quiet_say(
                master_id,
                &format!(
                    "I have given {given} {diff_name} quests, {solved} of these have been \
                     solved ({solve_rate:.2}%) for a total of {exp_given} exp ({avg_exp:.2} exp \
                     per quest)"
                ),
            );
        }
    }
    true
}

/// C qa code 19 ("reset", admin-only, `military.c:2068-2075`):
/// `ppd->solved_yday = ppd->mission_yday = 0`, no text.
fn apply_military_master_reset(runtime: &mut ServerRuntime, player_id: CharacterId) -> bool {
    let Some(player) = runtime.player_for_character_mut(player_id) else {
        return false;
    };
    player.set_military_solved_yday(0);
    player.set_mission_yday(0);
    true
}

/// C qa code 20 ("raise", admin-only, `military.c:2076-2082`):
/// `ppd->military_pts += 1000`, no text.
fn apply_military_master_raise(runtime: &mut ServerRuntime, player_id: CharacterId) -> bool {
    let Some(player) = runtime.player_for_character_mut(player_id) else {
        return false;
    };
    player.set_military_pts(player.military_pts() + 1000);
    true
}

/// C qa code 21 ("promote", admin-only, `military.c:2083-2089`):
/// `give_military_pts(cn, co, 100, 1)` - reuses [`World::
/// give_military_pts`]'s point/rank math (the promotion-announcement
/// text goes through `World::queue_system_text` rather than this NPC's
/// own `npc_quiet_say`, the same pre-existing simplification already
/// documented on [`World::give_military_pts`] and on `complete_mission`'s
/// reward text - functionally correct promotion, just delivered as a
/// system message instead of an NPC speech bubble).
fn apply_military_master_promote(
    world: &mut World,
    runtime: &mut ServerRuntime,
    master_id: CharacterId,
    player_id: CharacterId,
    area_id: u16,
) -> bool {
    if runtime.player_for_character(player_id).is_none() {
        return false;
    }
    if world.characters.get(&master_id).is_none() {
        return false;
    }
    world.give_military_pts(player_id, 100, 1, u32::from(area_id));
    true
}

/// C `military_advisor_driver`'s message-handling body (`src/module/
/// military.c:2607-2699`), applying each [`MilitaryAdvisorEvent`] queued
/// by `World::process_military_advisor_actions` (mirrors
/// `apply_military_master_events`'s shape).
pub(crate) fn apply_military_advisor_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
) -> usize {
    let mut applied = 0;
    for event in world.drain_pending_military_advisor_events() {
        match event {
            MilitaryAdvisorEvent::NearbyPlayer {
                advisor_id,
                player_id,
            } => {
                if apply_military_advisor_nearby_player(world, runtime, advisor_id, player_id) {
                    applied += 1;
                }
            }
            MilitaryAdvisorEvent::Repeat { player_id, .. } => {
                if let Some(player) = runtime.player_for_character_mut(player_id) {
                    // C qa code 2 ("repeat"): `ppd->advisor_state = 0;`,
                    // no text (`military.c:2610-2612`).
                    player.set_advisor_state(0);
                    applied += 1;
                }
            }
            MilitaryAdvisorEvent::FavorDesc {
                advisor_id,
                player_id,
            } => {
                if apply_military_advisor_favor_desc(world, runtime, advisor_id, player_id) {
                    applied += 1;
                }
            }
            MilitaryAdvisorEvent::Favor {
                advisor_id,
                player_id,
                favor_size,
            } => {
                if apply_military_advisor_favor(world, runtime, advisor_id, player_id, favor_size) {
                    applied += 1;
                }
            }
            MilitaryAdvisorEvent::Pay {
                advisor_id,
                player_id,
            } => {
                if apply_military_advisor_pay(world, runtime, advisor_id, player_id) {
                    applied += 1;
                }
            }
            MilitaryAdvisorEvent::SpecificMissionRequest {
                advisor_id,
                player_id,
                difficulty,
                mission_type,
            } => {
                if apply_military_advisor_specific_mission_request(
                    world,
                    runtime,
                    advisor_id,
                    player_id,
                    difficulty,
                    mission_type,
                ) {
                    applied += 1;
                }
            }
        }
    }
    applied
}

/// C `military_advisor_driver`'s `NT_CHAR` greeting branch
/// (`military.c:2639-2661`): the "already recommended today"/fresh-
/// introduction pair, gated on `ppd->advisor_state == 0 ||
/// ppd->current_advisor != dat->storage_ID`.
fn apply_military_advisor_nearby_player(
    world: &mut World,
    runtime: &mut ServerRuntime,
    advisor_id: CharacterId,
    player_id: CharacterId,
) -> bool {
    let yday = world.date.yday as i32;
    let storage_id = world.advisor_storage_id(advisor_id);
    let idx = calculate_advisor_index(storage_id);
    let Some(player_name) = world.characters.get(&player_id).map(|c| c.name.clone()) else {
        return false;
    };
    let Some(player) = runtime.player_for_character_mut(player_id) else {
        return false;
    };

    if player.advisor_state() == 0 || player.current_advisor() != storage_id {
        if player.military_advisor_last(idx) == yday + 1 {
            world.npc_quiet_say(
                advisor_id,
                &format!("Ah, {player_name}. I haven't forgotten you."),
            );
        } else {
            let text = adv_introduction_text(storage_id, &player_name);
            world.npc_quiet_say(advisor_id, &text);
        }
        if let Some(player) = runtime.player_for_character_mut(player_id) {
            player.set_advisor_state(1);
            player.set_current_advisor(storage_id);
        }
    }
    true
}

/// C qa code 3 ("favor"): [`adv_favor_desc_lines`], gated on the same
/// "already recommended today" check every favor/mission-request path
/// shares (`military.c:2494-2499`).
fn apply_military_advisor_favor_desc(
    world: &mut World,
    runtime: &mut ServerRuntime,
    advisor_id: CharacterId,
    player_id: CharacterId,
) -> bool {
    let yday = world.date.yday as i32;
    let storage_id = world.advisor_storage_id(advisor_id);
    let idx = calculate_advisor_index(storage_id);
    let Some(player_name) = world.characters.get(&player_id).map(|c| c.name.clone()) else {
        return false;
    };
    let Some(player) = runtime.player_for_character(player_id) else {
        return false;
    };

    if player.military_advisor_last(idx) == yday + 1 {
        world.npc_quiet_say(
            advisor_id,
            &format!("Mentioning your name twice a day won't accomplish much, {player_name}."),
        );
    } else {
        for line in adv_favor_desc_lines() {
            world.npc_quiet_say(advisor_id, line);
        }
    }
    true
}

/// C qa codes 4-8 ("small".."vast"): [`World::offer_favor`].
fn apply_military_advisor_favor(
    world: &mut World,
    runtime: &mut ServerRuntime,
    advisor_id: CharacterId,
    player_id: CharacterId,
    favor_size: i32,
) -> bool {
    let yday = world.date.yday as i32;
    let storage_id = world.advisor_storage_id(advisor_id);
    let idx = calculate_advisor_index(storage_id);
    let Some(player_name) = world.characters.get(&player_id).map(|c| c.name.clone()) else {
        return false;
    };
    let Some(player) = runtime.player_for_character_mut(player_id) else {
        return false;
    };

    match world.offer_favor(player_id, player, idx, favor_size, yday) {
        OfferFavorOutcome::AlreadyUsedToday => {
            world.npc_quiet_say(
                advisor_id,
                &format!("Mentioning your name twice a day won't accomplish much, {player_name}."),
            );
        }
        // C's own `default: return 0;` bail-out - no text at all,
        // unreachable via the fixed qa-code mapping.
        OfferFavorOutcome::InvalidFavorSize => {}
        OfferFavorOutcome::Offered { favor_size, cost } => {
            world.npc_quiet_say(
                advisor_id,
                &format!(
                    "You can get a {} favor for the humble fee of {}G, {}S, {player_name}. Say \
                     pay if you want it.",
                    favor_size_name(favor_size),
                    cost / 100,
                    cost % 100
                ),
            );
        }
    }
    true
}

/// C qa code 9 ("pay"): [`World::process_favor_payment`].
fn apply_military_advisor_pay(
    world: &mut World,
    runtime: &mut ServerRuntime,
    advisor_id: CharacterId,
    player_id: CharacterId,
) -> bool {
    let yday = world.date.yday as i32;
    let storage_id = world.advisor_storage_id(advisor_id);
    let idx = calculate_advisor_index(storage_id);
    let Some(player_name) = world.characters.get(&player_id).map(|c| c.name.clone()) else {
        return false;
    };
    let Some(player) = runtime.player_for_character_mut(player_id) else {
        return false;
    };

    match world.process_favor_payment(player_id, player, idx, storage_id, yday) {
        ProcessFavorPaymentOutcome::NothingAgreed => {
            world.npc_quiet_say(
                advisor_id,
                "Pay for what? We haven't agreed on anything yet.",
            );
        }
        ProcessFavorPaymentOutcome::InsufficientGold => {
            world.npc_quiet_say(advisor_id, "Alas, you do not have enough money.");
        }
        ProcessFavorPaymentOutcome::SpecificMissionArranged {
            mission_type,
            difficulty,
        } => {
            world.npc_quiet_say(
                advisor_id,
                &format!(
                    "Excellent! I've arranged an {} {} mission for you. The military governor \
                     will have your orders ready at daybreak.",
                    mission_difficulty_name(difficulty as usize),
                    mission_type_name(mission_type)
                ),
            );
        }
        ProcessFavorPaymentOutcome::FavorArranged { .. } => {
            world.npc_quiet_say(
                advisor_id,
                &format!(
                    "Alright, I'll mention your name to the military governor, {player_name}."
                ),
            );
        }
    }
    true
}

/// C qa codes 30-44 ("easy demon".."insane silver"):
/// [`World::handle_specific_mission_request`].
fn apply_military_advisor_specific_mission_request(
    world: &mut World,
    runtime: &mut ServerRuntime,
    advisor_id: CharacterId,
    player_id: CharacterId,
    difficulty: i32,
    mission_type: i32,
) -> bool {
    let yday = world.date.yday as i32;
    let storage_id = world.advisor_storage_id(advisor_id);
    let idx = calculate_advisor_index(storage_id);
    let Some(player_name) = world.characters.get(&player_id).map(|c| c.name.clone()) else {
        return false;
    };
    let Some(player) = runtime.player_for_character_mut(player_id) else {
        return false;
    };

    match world.handle_specific_mission_request(
        player_id,
        player,
        idx,
        difficulty,
        mission_type,
        yday,
    ) {
        SpecificMissionRequestOutcome::AlreadyUsedToday => {
            world.npc_quiet_say(
                advisor_id,
                &format!(
                    "I've already used my influence for you today, {player_name}. Come back \
                     tomorrow."
                ),
            );
        }
        SpecificMissionRequestOutcome::InvalidMissionType => {
            world.npc_quiet_say(
                advisor_id,
                &format!("I don't know about that type of mission, {player_name}."),
            );
        }
        SpecificMissionRequestOutcome::InvalidDifficulty => {
            world.npc_quiet_say(
                advisor_id,
                &format!("I don't know that difficulty level, {player_name}."),
            );
        }
        SpecificMissionRequestOutcome::RatlingLevelGate => {
            world.npc_quiet_say(
                advisor_id,
                &format!(
                    "Ratling missions are only available at odd levels between 9 and 39, \
                     {player_name}."
                ),
            );
        }
        SpecificMissionRequestOutcome::SilverLevelGate => {
            world.npc_quiet_say(
                advisor_id,
                &format!(
                    "Silver missions are only available at level 12 and above, {player_name}."
                ),
            );
        }
        SpecificMissionRequestOutcome::Offered {
            difficulty,
            mission_type,
            cost,
            already_completed_today,
            has_active_mission,
        } => {
            if already_completed_today {
                world.npc_quiet_say(
                    advisor_id,
                    &format!(
                        "I should warn you, {player_name} - you've already completed a mission \
                         today. My recommendation will carry over to tomorrow, but you won't be \
                         able to use it until then."
                    ),
                );
            }
            if has_active_mission {
                world.npc_quiet_say(
                    advisor_id,
                    &format!(
                        "Keep in mind, {player_name}, you already have an active mission. \
                         You'll need to complete or abandon it before you can take the one I \
                         recommend."
                    ),
                );
            }
            world.npc_quiet_say(
                advisor_id,
                &format!(
                    "I can recommend you for an {} {} mission for {}G, {}S. Say pay if you want \
                     it.",
                    mission_difficulty_name(difficulty as usize),
                    mission_type_name(mission_type),
                    cost / 100,
                    cost % 100
                ),
            );
        }
    }
    true
}
