//! Server-side wiring for the anti-macro/anti-bot "Macro Daemon" activity
//! tracker (`ugaris-core`'s `macro_daemon` module, `src/module/base.c`'s
//! `macro_track_exp_gain`/`macro_track_combat`/`macro_track_gold_change`,
//! `src/system/tool.c:385-426` + `death.c:1112-1117`).
//!
//! [`apply_macro_activity_events`] drains the three `World` queues
//! `pending_exp_gain_events`/`pending_combat_events`/
//! `pending_gold_change_events` (see their doc comments on `ugaris-core`'s
//! `world/mod.rs`) and stamps the matching `MacroPpd::last_exp_gain`/
//! `last_combat`/`last_gold_change` field on each character's
//! `PlayerRuntime`, mirroring `apply_bank_events`'s `World`/
//! `PlayerRuntime` split - `World` cannot reach `PlayerRuntime` directly,
//! only `ugaris-server` (which owns `ServerRuntime`) can.
//!
//! [`apply_macro_events`] is that larger bridge: the live `CDR_MACRO`
//! "Macro Daemon" NPC driver itself (`base.c:802-1235`), ticked once per
//! live macro-daemon NPC per game tick. Nearly every state this driver
//! passes through touches a player's persistent `DRD_MACRO_PPD`
//! (immunity/next-check/activity gating the victim search, karma/
//! suspicion shaping the greeting and challenge difficulty, and the
//! correct-answer/failure `MacroPpd` mutations themselves), so - unlike
//! most drivers in this codebase, which keep their whole state machine in
//! `ugaris-core` and only defer isolated PPD-touching leaves to
//! `ugaris-server` - this one is wired directly here, reusing
//! `ugaris-core`'s already-ported pure "brain" (challenge generation/
//! checking/reward classification, `crate::macro_daemon`) and `World`'s
//! non-PPD helpers (`macro_search_candidates`/`macro_update_appearance`/
//! `macro_handle_give_message`/`macro_idle_mutter`/the `macro_*_seeded`
//! `PlayerRuntime`-plus-`World::legacy_random_seed` wrappers, all in
//! `ugaris-core/src/world/macro_npc.rs`) for everything that doesn't.
//!
//! The cross-server "challenge room" teleport-and-restore flow (`base.c:
//! 1054-1123`'s banishment, `840-891`'s return trip) *is* wired: the
//! suspicion/failure-triggered banishment in `MACRO_STATE_FOUND`, the
//! area-3-only cross-server pickup scan in `MACRO_STATE_IDLE`, the
//! correct-answer return teleport, and the "you remain in the challenge
//! room" failure message are all applied here, using `World::
//! macro_banish_to_challenge_room`/`macro_restore_original_respawn`/
//! `queue_macro_cross_area_transfer` (`ugaris-core/src/world/
//! macro_npc.rs`) plus `ugaris-core::macro_daemon`'s
//! `macro_should_banish_to_challenge_room`/
//! `macro_begin_challenge_room_banishment`/`macro_save_pentagram_progress`
//! pure helpers for the `MacroPpd` half. The actual cross-area hand-off
//! (`change_area`) itself happens one step later, in `world_events.rs::
//! apply_macro_cross_area_transfers`, via the shared
//! `attempt_cross_area_transfer` helper - `World` has no DB handle or
//! `ServerRuntime` of its own, same reason `world/jail.rs`'s
//! `JailCrossAreaTransfer` defers there too.
//!
//! Disclosed, deliberate simplifications (see `ugaris-core/src/
//! macro_daemon.rs` and `world/macro_npc.rs`'s own module doc comments for
//! the full list): the recent-login grace period is not applied; the
//! `isxmas` reskin *is* wired (via `is_xmas`, `ugaris-server`'s own xmas-
//! event awareness, `World` has none).

use super::*;
use ugaris_core::character_driver::{MacroDriverData, MacroDriverState, NT_GIVE, NT_TEXT};
use ugaris_core::drvlib::offset2dx;
use ugaris_core::see::char_see_char;
use ugaris_core::world::{
    macro_ask_challenge_lines, macro_begin_challenge_room_banishment, macro_check_answer,
    macro_is_pents_area, macro_is_player_active, macro_reward_fallback, macro_reward_item_template,
    macro_reward_success_message, macro_roll_reward, macro_save_pentagram_progress,
    macro_should_banish_to_challenge_room, macro_xmas_reward_message, MacroReward,
    CHALLENGE_ROOM_AREA, CHALLENGE_ROOM_X, CHALLENGE_ROOM_Y, MACRO_CHALLENGE_TIME,
    MACRO_REPEAT_INTERVAL,
};

/// Drains `World`'s `pending_exp_gain_events`/`pending_combat_events`/
/// `pending_gold_change_events` queues and stamps the matching
/// `MacroPpd::last_exp_gain`/`last_combat`/`last_gold_change` field (each
/// to `now`) on every character with a live `PlayerRuntime`. A no-op for
/// any `CharacterId` with no online `PlayerRuntime` (matching C's own
/// `ppd = set_data(...)` "no PPD, do nothing" guard - a character with no
/// session simply has no `MacroPpd` to update). Returns how many of the
/// three counters were actually applied (for the same "log iff nonzero"
/// convention `apply_bank_events`/`apply_military_master_events` use).
pub(crate) fn apply_macro_activity_events(
    runtime: &mut ServerRuntime,
    world: &mut World,
    now: i64,
) -> usize {
    let mut applied = 0;

    for character_id in world.drain_exp_gain_events() {
        if let Some(player) = runtime.player_for_character_mut(character_id) {
            player.macro_ppd.last_exp_gain = now;
            applied += 1;
        }
    }

    for character_id in world.drain_combat_events() {
        if let Some(player) = runtime.player_for_character_mut(character_id) {
            player.macro_ppd.last_combat = now;
            applied += 1;
        }
    }

    for character_id in world.drain_gold_change_events() {
        if let Some(player) = runtime.player_for_character_mut(character_id) {
            player.macro_ppd.last_gold_change = now;
            applied += 1;
        }
    }

    applied
}

/// C `macro_driver`'s top-level per-NPC dispatch (`base.c:802-1235`):
/// every live `CDR_MACRO` NPC gets its appearance refreshed, its message
/// queue drained, and its state machine cascaded, once per game tick.
/// Returns how many macro-daemon NPCs actually changed state or victim
/// this tick (found/teleported/asked/answered/timed out), for the same
/// "log iff nonzero" convention every other `apply_*_events` in this
/// crate uses - a macro daemon idling with no eligible candidate is not
/// counted.
pub(crate) fn apply_macro_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    area_id: u16,
    is_xmas: bool,
    now: i64,
) -> usize {
    let mut applied = 0;
    for macro_id in world.macro_daemon_ids() {
        if tick_macro_daemon(world, runtime, loader, area_id, is_xmas, now, macro_id) {
            applied += 1;
        }
    }
    applied
}

/// One macro-daemon NPC's full per-tick body: appearance, message loop,
/// then the `MACRO_STATE_*` cascade - all sequential `if`s, not `else
/// if`s, matching C's own control flow exactly (a single tick can cascade
/// through several states, e.g. a just-answered `MACRO_STATE_CHALLENGING`
/// falling back to `MACRO_STATE_IDLE` immediately re-enters the search and
/// can find/teleport-to/challenge a *new* victim before this tick ends).
fn tick_macro_daemon(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    area_id: u16,
    is_xmas: bool,
    now: i64,
    macro_id: CharacterId,
) -> bool {
    world.macro_update_appearance(macro_id, is_xmas);

    let mut dat = match world
        .characters
        .get(&macro_id)
        .and_then(|character| character.driver_state.clone())
    {
        Some(CharacterDriverState::Macro(dat)) => dat,
        _ => MacroDriverData::default(),
    };
    let before_state = dat.state;
    let before_victim = dat.victim;

    macro_process_messages(
        world, runtime, loader, area_id, is_xmas, now, macro_id, &mut dat,
    );
    macro_run_state_machine(world, runtime, area_id, now, macro_id, &mut dat);

    let acted = dat.state != before_state || dat.victim != before_victim;

    if let Some(character) = world.characters.get_mut(&macro_id) {
        character.driver_state = Some(CharacterDriverState::Macro(dat));
    }
    acted
}

/// C `macro_driver`'s message loop (`base.c:816-910`): the `NT_TEXT`
/// answer-checking branch (plus the generic `tabunga` GM stat-dump hook,
/// `base.c:898`) and the `NT_GIVE` gift-destroying branch.
fn macro_process_messages(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    area_id: u16,
    is_xmas: bool,
    now: i64,
    macro_id: CharacterId,
    dat: &mut MacroDriverData,
) {
    let messages = world
        .characters
        .get_mut(&macro_id)
        .map(|character| std::mem::take(&mut character.driver_messages))
        .unwrap_or_default();

    for message in &messages {
        match message.message_type {
            NT_TEXT => {
                let speaker_id = CharacterId(message.dat3.max(0) as u32);
                if dat.victim != Some(speaker_id) {
                    continue;
                }
                let Some(text) = message.text.as_deref() else {
                    continue;
                };
                world.apply_tabunga_text_notification(macro_id, speaker_id, text);

                let is_correct = dat
                    .challenge
                    .as_ref()
                    .is_some_and(|challenge| macro_check_answer(challenge, text));
                if is_correct {
                    macro_resolve_correct_answer(
                        world, runtime, loader, area_id, is_xmas, now, macro_id, speaker_id, dat,
                    );
                } else if !text.trim().is_empty() {
                    if let Some(name) = world.characters.get(&speaker_id).map(|c| c.name.clone()) {
                        world.npc_say(
                            macro_id,
                            &format!("That's not quite right, {name}. Try again!"),
                        );
                    }
                    dat.start = dat.start.saturating_sub(TICKS_PER_SECOND * 15);
                }
            }
            NT_GIVE => world.macro_handle_give_message(macro_id),
            _ => {}
        }
    }
}

/// C `macro_driver`'s `NT_TEXT` correct-answer branch (`base.c:840-891`):
/// records the passed challenge, improves karma/lowers suspicion/
/// reschedules the next check (`macro_apply_correct_answer_seeded`), then
/// rolls and grants a reward. The challenge-room return-teleport
/// (`dat->teleported_to_jail && ppd->in_challenge_room`) is not applied -
/// `in_challenge_room` never becomes `true` in this slice (see this
/// module's doc comment), so that branch is unreachable in practice.
#[allow(clippy::too_many_arguments)]
fn macro_resolve_correct_answer(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    area_id: u16,
    is_xmas: bool,
    now: i64,
    macro_id: CharacterId,
    victim_id: CharacterId,
    dat: &mut MacroDriverData,
) {
    let challenge_type = dat
        .challenge
        .as_ref()
        .map(|challenge| challenge.challenge_type)
        .unwrap_or(0);
    let response_time = (world.tick.0 as i64 - dat.start as i64) / TICKS_PER_SECOND as i64;

    // Challenge-room return trip (`base.c:840-853`): `dat.teleported_to_
    // jail && ppd.in_challenge_room` - restore the original respawn point
    // and teleport (locally or cross-server) back to where the victim
    // was banished from, before the reward is rolled.
    let mut return_trip: Option<(bool, i32, i32, u16)> = None;
    let karma = if let Some(player) = runtime.player_for_character_mut(victim_id) {
        world.macro_apply_correct_answer_seeded(
            &mut player.macro_ppd,
            now,
            response_time as i32,
            challenge_type,
        );
        if dat.teleported_to_jail && player.macro_ppd.in_challenge_room {
            return_trip = Some((
                player.macro_ppd.original_area == i32::from(area_id),
                player.macro_ppd.original_x,
                player.macro_ppd.original_y,
                player.macro_ppd.original_area as u16,
            ));
            world.macro_restore_original_respawn(
                victim_id,
                player.macro_ppd.original_restx,
                player.macro_ppd.original_resty,
                player.macro_ppd.original_resta,
            );
            player.macro_ppd.in_challenge_room = false;
        }
        Some(player.macro_ppd.karma)
    } else {
        None
    };

    if let Some((same_area, target_x, target_y, target_area)) = return_trip {
        world.npc_say(macro_id, "Excellent! Let me send you back now.");
        if same_area {
            world.teleport_char_driver(victim_id, target_x as u16, target_y as u16);
        } else {
            world.queue_macro_cross_area_transfer(
                victim_id,
                target_area,
                target_x as u16,
                target_y as u16,
            );
        }
    }

    let victim_level = world
        .characters
        .get(&victim_id)
        .map(|c| c.level)
        .unwrap_or(0);
    let victim_name = world
        .characters
        .get(&victim_id)
        .map(|c| c.name.clone())
        .unwrap_or_default();

    if is_xmas {
        if grant_template_item_smart(world, loader, victim_id, "xmaspop").is_some() {
            world.npc_say(macro_id, &macro_xmas_reward_message(&victim_name));
        }
    } else {
        let reward_roll = world.macro_roll_reward_type();
        let reward = macro_roll_reward(reward_roll, victim_level, karma);
        match macro_reward_item_template(&reward) {
            Some(template) => {
                if grant_template_item_smart(world, loader, victim_id, template).is_some() {
                    world.npc_say(macro_id, &macro_reward_success_message(&reward, None));
                } else if let Some((fallback_exp, message)) = macro_reward_fallback(&reward) {
                    world.give_exp(victim_id, i64::from(fallback_exp), u32::from(area_id));
                    world.npc_say(macro_id, message);
                }
            }
            None => {
                if let MacroReward::Gold { base, random_span } = reward {
                    let gold = world.macro_roll_gold_reward(base, random_span);
                    if let Some(character) = world.characters.get_mut(&victim_id) {
                        character.gold = character.gold.saturating_add(gold);
                        character.flags.insert(CharacterFlags::ITEMS);
                    }
                    world.npc_say(macro_id, &macro_reward_success_message(&reward, Some(gold)));
                } else {
                    // `Experience`/`Consolation` - the only other variants
                    // with no item template.
                    let exp = match reward {
                        MacroReward::Experience { exp } | MacroReward::Consolation { exp } => exp,
                        _ => 0,
                    };
                    world.give_exp(victim_id, i64::from(exp), u32::from(area_id));
                    world.npc_say(macro_id, &macro_reward_success_message(&reward, None));
                }
            }
        }
    }

    dat.challenge = None;
    dat.teleported_to_jail = false;
    dat.search_cursor = victim_id.0.saturating_add(1);
    dat.victim = None;
    dat.state = MacroDriverState::Idle;
}

/// C `macro_driver`'s `MACRO_STATE_*` cascade (`base.c:923-1234`): the
/// victim search (force-summon scan, the area-3-only cross-server
/// challenge-room pickup, then the normal candidate search), the
/// teleport-to-victim/challenge-room-banishment step, asking/repeating
/// the challenge, the timeout, and the idle mutterings.
fn macro_run_state_machine(
    world: &mut World,
    runtime: &mut ServerRuntime,
    area_id: u16,
    now: i64,
    macro_id: CharacterId,
    dat: &mut MacroDriverData,
) {
    if dat.state == MacroDriverState::Idle {
        // Forced-summon scan (`base.c:925-938`): closes `/summonmacro`'s
        // loop end-to-end.
        let mut player_ids: Vec<CharacterId> = world
            .characters
            .values()
            .filter(|character| character.flags.contains(CharacterFlags::PLAYER))
            .map(|character| character.id)
            .collect();
        player_ids.sort_by_key(|id| id.0);

        let mut found: Option<CharacterId> = None;
        for candidate in &player_ids {
            if let Some(player) = runtime.player_for_character_mut(*candidate) {
                if player.macro_ppd.force_summon {
                    player.macro_ppd.force_summon = false;
                    found = Some(*candidate);
                    break;
                }
            }
        }

        // Cross-server "challenge room" pickup (`base.c:940-960`): only
        // this area server's own challenge room (area 3) ever scans for
        // arrivals; a player's `in_challenge_room && needs_challenge`
        // both become `true` together in the `MACRO_STATE_FOUND`
        // banishment step below (the cross-server branch), and only
        // become visible here once the player has actually reconnected
        // to area 3 (normal login, `change_area`'s hand-off already
        // wrote their `resta`/`restx`/`resty` to the challenge room).
        let mut cross_server_pickup = false;
        if found.is_none() && area_id == CHALLENGE_ROOM_AREA {
            for candidate in &player_ids {
                if let Some(player) = runtime.player_for_character_mut(*candidate) {
                    if player.macro_ppd.in_challenge_room && player.macro_ppd.needs_challenge {
                        player.macro_ppd.needs_challenge = false;
                        found = Some(*candidate);
                        cross_server_pickup = true;
                        break;
                    }
                }
            }
        }

        if found.is_none() {
            for candidate in world.macro_search_candidates(area_id, dat.search_cursor) {
                let Some(player) = runtime.player_for_character_mut(candidate) else {
                    continue;
                };
                if player.macro_ppd.immune_until > now {
                    continue;
                }
                if now < player.macro_ppd.nextcheck {
                    continue;
                }
                if !macro_is_player_active(&player.macro_ppd, now) {
                    player.macro_ppd.nextcheck = now + 60 * 30;
                    continue;
                }
                found = Some(candidate);
                break;
            }
        }

        let Some(victim_id) = found else {
            dat.search_cursor = 0;
            if let Some(character) = world.characters.get(&macro_id).cloned() {
                world.teleport_char_driver(macro_id, character.rest_x, character.rest_y);
            }
            return;
        };

        dat.victim = Some(victim_id);
        dat.search_cursor = victim_id.0;

        if cross_server_pickup {
            // Cross-server pickup: the player is already sitting in the
            // challenge room (`change_area` put them there), so skip
            // `MACRO_STATE_FOUND` entirely and teleport the daemon
            // straight to them (`base.c:997-1008`).
            dat.teleported_to_jail = true;
            let Some(victim) = world.characters.get(&victim_id).cloned() else {
                dat.victim = None;
                dat.state = MacroDriverState::Idle;
                return;
            };
            if world.teleport_char_driver(macro_id, victim.x, victim.y) {
                dat.state = MacroDriverState::Teleported;
            } else {
                dat.victim = None;
                dat.state = MacroDriverState::Idle;
                return;
            }
        } else {
            dat.state = MacroDriverState::Found;
            dat.teleported_to_jail = false;
        }
    }

    if dat.state == MacroDriverState::Found {
        let Some(victim_id) = dat.victim else {
            dat.state = MacroDriverState::Idle;
            return;
        };

        // Suspicion/failure-triggered challenge-room banishment
        // (`base.c:1054-1123`).
        let (suspicion, challenge_failures) = runtime
            .player_for_character(victim_id)
            .map(|player| {
                (
                    player.macro_ppd.suspicion,
                    player.macro_ppd.challenge_failures,
                )
            })
            .unwrap_or((0, 0));
        if macro_should_banish_to_challenge_room(suspicion, challenge_failures) {
            if let Some((orig_x, orig_y, orig_restx, orig_resty, orig_resta)) =
                world.macro_banish_to_challenge_room(victim_id)
            {
                let pent_data = runtime
                    .player_for_character(victim_id)
                    .map(|player| player.pentagram_debug);
                if let Some(player) = runtime.player_for_character_mut(victim_id) {
                    macro_begin_challenge_room_banishment(
                        &mut player.macro_ppd,
                        orig_x,
                        orig_y,
                        i32::from(area_id),
                        orig_restx,
                        orig_resty,
                        orig_resta,
                    );
                    macro_save_pentagram_progress(
                        &mut player.macro_ppd,
                        pent_data.as_ref().filter(|_| macro_is_pents_area(area_id)),
                    );
                }

                if area_id == CHALLENGE_ROOM_AREA {
                    world.teleport_char_driver(victim_id, CHALLENGE_ROOM_X, CHALLENGE_ROOM_Y);
                    dat.teleported_to_jail = true;
                    if let Some(player) = runtime.player_for_character_mut(victim_id) {
                        player.macro_ppd.needs_challenge = false;
                    }
                    world.queue_system_text(
                        victim_id,
                        "You've been brought to a challenge room. Answer correctly to return."
                            .to_string(),
                    );
                } else {
                    if let Some(player) = runtime.player_for_character_mut(victim_id) {
                        player.macro_ppd.needs_challenge = true;
                    }
                    dat.teleported_to_jail = false;
                    world.queue_macro_cross_area_transfer(
                        victim_id,
                        CHALLENGE_ROOM_AREA,
                        CHALLENGE_ROOM_X,
                        CHALLENGE_ROOM_Y,
                    );
                    dat.search_cursor = victim_id.0.saturating_add(1);
                    dat.victim = None;
                    dat.state = MacroDriverState::Idle;
                    return;
                }
            }
        }

        let Some(victim) = world.characters.get(&victim_id).cloned() else {
            dat.search_cursor = victim_id.0.saturating_add(1);
            dat.victim = None;
            dat.state = MacroDriverState::Idle;
            return;
        };
        if world.teleport_char_driver(macro_id, victim.x, victim.y) {
            dat.state = MacroDriverState::Teleported;
        } else {
            dat.search_cursor = victim_id.0.saturating_add(1);
            dat.victim = None;
            dat.state = MacroDriverState::Idle;
            return;
        }
    }

    let mut talkdir = None;

    if dat.state == MacroDriverState::Teleported {
        let Some(victim_id) = dat.victim else {
            dat.state = MacroDriverState::Idle;
            return;
        };
        let Some(victim) = world.characters.get(&victim_id).cloned() else {
            dat.search_cursor = victim_id.0.saturating_add(1);
            dat.victim = None;
            dat.state = MacroDriverState::Idle;
            return;
        };

        dat.start = world.tick.0;
        dat.last = dat
            .start
            .saturating_sub(TICKS_PER_SECOND * (MACRO_REPEAT_INTERVAL as u64 + 1));

        let (suspicion, karma, failures) = runtime
            .player_for_character(victim_id)
            .map(|player| {
                (
                    player.macro_ppd.suspicion,
                    player.macro_ppd.karma,
                    player.macro_ppd.challenge_failures,
                )
            })
            .unwrap_or((0, 0, 0));
        dat.challenge = Some(world.macro_roll_challenge(suspicion, failures));
        dat.state = MacroDriverState::Challenging;

        let macro_name = world
            .characters
            .get(&macro_id)
            .map(|character| character.name.clone())
            .unwrap_or_default();
        let greeting = if karma > 50 {
            format!(
                "Hello again, {}! Just a quick check - I have a question for you.",
                victim.name
            )
        } else if suspicion > 30 {
            format!(
                "Greetings, {}. I need to verify you're playing fairly. Please answer my \
                 question.",
                victim.name
            )
        } else {
            format!(
                "Hello, {}! I'm {macro_name}. Mind answering a quick question?",
                victim.name
            )
        };
        world.npc_say(macro_id, &greeting);
        if let Some(character) = world.characters.get(&macro_id) {
            talkdir = offset2dx(
                i32::from(character.x),
                i32::from(character.y),
                i32::from(victim.x),
                i32::from(victim.y),
            );
        }
    }

    if dat.state == MacroDriverState::Challenging {
        let Some(victim_id) = dat.victim else {
            dat.state = MacroDriverState::Idle;
            return;
        };
        let macro_character = world.characters.get(&macro_id).cloned();
        let victim = world.characters.get(&victim_id).cloned();
        let visible = match (&macro_character, &victim) {
            (Some(m), Some(v)) => char_see_char(m, v, &world.map, world.date.daylight),
            _ => false,
        };
        if victim.is_none() || !visible {
            // In-challenge-room failure (`base.c:1170-1174`): unlike
            // `MACRO_STATE_TIMEOUT` below (which always calls
            // `macro_handle_failure`), C only penalizes a victim who
            // vanishes/goes invisible mid-challenge if they were banished
            // to the challenge room (`ppd->in_challenge_room`) - an
            // ordinary victim who simply steps out of sight is let go
            // with no suspicion/failure consequence at all.
            if let Some(v) = victim.as_ref() {
                let in_challenge_room = runtime
                    .player_for_character(victim_id)
                    .map(|player| player.macro_ppd.in_challenge_room)
                    .unwrap_or(false);
                if in_challenge_room {
                    let challenge_type = dat
                        .challenge
                        .as_ref()
                        .map(|challenge| challenge.challenge_type)
                        .unwrap_or(0);
                    if let Some(player) = runtime.player_for_character_mut(victim_id) {
                        let update = world.macro_apply_failure_seeded(
                            &mut player.macro_ppd,
                            &v.name,
                            now,
                            challenge_type,
                        );
                        world.npc_say(macro_id, &update.victim_message);
                        world.queue_system_text(victim_id, update.log_message);
                        if update.kicked {
                            if let Some(vc) = world.characters.get_mut(&victim_id) {
                                vc.flags.insert(CharacterFlags::KICKED);
                            }
                        }
                        if dat.teleported_to_jail && player.macro_ppd.in_challenge_room {
                            world.queue_system_text(
                                victim_id,
                                "You remain in the challenge room. Answer correctly to leave."
                                    .to_string(),
                            );
                        }
                    }
                }
            }
            dat.search_cursor = victim_id.0.saturating_add(1);
            dat.victim = None;
            dat.state = MacroDriverState::Idle;
            return;
        }
        let victim = victim.expect("checked above");
        let macro_character = macro_character.expect("checked above");

        if world.tick.0.saturating_sub(dat.start) > TICKS_PER_SECOND * MACRO_CHALLENGE_TIME as u64 {
            dat.state = MacroDriverState::Timeout;
        } else if world.tick.0.saturating_sub(dat.last)
            > TICKS_PER_SECOND * MACRO_REPEAT_INTERVAL as u64
        {
            if let Some(challenge) = dat.challenge.as_ref() {
                for line in macro_ask_challenge_lines(challenge, &victim.name) {
                    world.npc_say(macro_id, &line);
                }
            }
            talkdir = offset2dx(
                i32::from(macro_character.x),
                i32::from(macro_character.y),
                i32::from(victim.x),
                i32::from(victim.y),
            );
            dat.last = world.tick.0;
        }
    }

    if dat.state == MacroDriverState::Timeout {
        let Some(victim_id) = dat.victim else {
            dat.state = MacroDriverState::Idle;
            return;
        };
        let Some(victim) = world.characters.get(&victim_id).cloned() else {
            dat.search_cursor = victim_id.0.saturating_add(1);
            dat.victim = None;
            dat.state = MacroDriverState::Idle;
            return;
        };
        if let Some(character) = world.characters.get(&macro_id) {
            talkdir = offset2dx(
                i32::from(character.x),
                i32::from(character.y),
                i32::from(victim.x),
                i32::from(victim.y),
            );
        }

        let challenge_type = dat
            .challenge
            .as_ref()
            .map(|challenge| challenge.challenge_type)
            .unwrap_or(0);
        if let Some(player) = runtime.player_for_character_mut(victim_id) {
            let update = world.macro_apply_failure_seeded(
                &mut player.macro_ppd,
                &victim.name,
                now,
                challenge_type,
            );
            world.npc_say(macro_id, &update.victim_message);
            world.queue_system_text(victim_id, update.log_message);
            if update.kicked {
                if let Some(v) = world.characters.get_mut(&victim_id) {
                    v.flags.insert(CharacterFlags::KICKED);
                }
            }
            // "You remain in the challenge room" (`base.c:782-785`):
            // `dat.teleported_to_jail && ppd.in_challenge_room` - the
            // daemon will pick them up again next tick with a new
            // question; only `/unjail` or a correct answer gets them out.
            if dat.teleported_to_jail && player.macro_ppd.in_challenge_room {
                world.queue_system_text(
                    victim_id,
                    "You remain in the challenge room. Answer correctly to leave.".to_string(),
                );
            }
        }

        dat.search_cursor = victim_id.0.saturating_add(1);
        dat.victim = None;
        dat.challenge = None;
        dat.state = MacroDriverState::Idle;
    }

    if let Some(direction) = talkdir {
        if let Some(character) = world.characters.get_mut(&macro_id) {
            character.dir = direction as u8;
        }
    }

    if dat.state == MacroDriverState::Idle {
        world.macro_idle_mutter(macro_id);
    }
}
