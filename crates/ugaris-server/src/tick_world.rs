//! Per-tick world-stepping phase: the part of the legacy tick loop that
//! runs once per tick before queued client actions are processed (game
//! clock/regen, autonomous weather cycle, lostcon expiry, effect/timer
//! processing, item-driver spawn outcomes, and kill-hook drains). Extracted
//! verbatim from `main()`'s `tick.tick()` arm (P0.5 "Finish main() phase
//! decomposition") with a superset-params signature, preserving exact
//! execution order; `main.rs` must not grow when this phase changes.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn world_step(
    mut world: &mut World,
    mut runtime: &mut ServerRuntime,
    mut zone_loader: &mut ZoneLoader,
    config: &ServerConfig,
    args: &Args,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
) {
    world.advance();
    world.advance_date(
        current_unix_time(),
        config.area_id,
        (runtime.dlight_override != 0).then_some(runtime.dlight_override),
    );
    world.regenerate_characters(runtime.regen_time, config.area_id);
    // C `server.c:210`'s `update_weather()` + `act.c:2268`'s
    // per-player `apply_weather_effects` (`src/module/weather/
    // weather.c`): advance the autonomous seasonal weather
    // cycle every tick, broadcast an `SV_MOD2`/`SV_VIS_WEATHER`
    // packet to every connected player when it changes, and
    // roll the periodic outdoor damage tick.
    let weather_changed = update_weather_tick(
        &mut runtime.weather,
        &world.date,
        world.tick.0,
        runtime_random_below,
    );
    if weather_changed {
        broadcast_weather_packet(&world, &mut runtime, config.area_id);
    }
    // C `modify_movement_speed` (`module/weather/weather.c:
    // 477-493`): refresh the live movement-slow percent every
    // tick so `do_walk` (via `World.settings.
    // weather_movement_percent`) applies it exactly like C's
    // `speed()` call folds it in. Gated on `area_has_weather`
    // like the damage roll below - no-weather areas (indoor/
    // underground/arena) never apply the autonomous cycle's
    // current weather type to movement.
    world.settings.weather_movement_percent = if area_has_weather(i64::from(config.area_id)) {
        current_movement_percent(&runtime.weather)
    } else {
        100
    };
    if area_has_weather(i64::from(config.area_id)) {
        let player_character_ids: Vec<CharacterId> = world
            .characters
            .values()
            .filter(|character| character.flags.contains(CharacterFlags::PLAYER))
            .map(|character| character.id)
            .collect();
        if runtime.weather.weather_effects & WEATHER_EFFECT_DAMAGE != 0 {
            let damage = weather_damage_amount(
                runtime.weather.current_weather,
                runtime.weather.weather_intensity,
            );
            if damage > 0 {
                // C `handle_weather_damage` (`weather.c:435-471`):
                // each player rolls its own independent "Only
                // apply damage occasionally (every ~12 seconds)"
                // `RANDOM(TICKS * 12)` check every tick (the C
                // call site is inside the per-character
                // `tick_char` loop), so every player gets its own
                // chance rather than all-or-nothing for the
                // whole area. On an actual hit, also queues the
                // matching per-weather-type `log_char` message -
                // previously missing from this port, see
                // `weather_damage_message`.
                for &character_id in &player_character_ids {
                    if runtime_random_below((TICKS_PER_SECOND * 12) as i32) == 0
                        && world.apply_weather_damage(character_id, damage).is_some()
                    {
                        if let Some(message) =
                            weather_damage_message(runtime.weather.current_weather)
                        {
                            world.queue_system_text(character_id, message);
                        }
                    }
                }
            }
        }
        // C `handle_lightning_strike` (`weather.c:534-575`),
        // called from the same per-player `apply_weather_effects`
        // tick hook as the damage roll above: an independent
        // per-player `RANDOM(100*TICKS*60) < lightning_chance*100`
        // roll (only `MOD_WEATHER_STORM` ever has a nonzero
        // `lightning_chance`), gated on the same
        // `character_weather_eligible` guards (player-only,
        // never gods/immortals, never indoors) *before* the RNG
        // call so ineligible characters never consume a roll,
        // matching C's guard-before-roll order exactly.
        if runtime.weather.weather_effects & WEATHER_EFFECT_LIGHTNING != 0 {
            let lightning_chance = lightning_strike_chance(
                runtime.weather.current_weather,
                runtime.weather.weather_intensity,
            );
            if lightning_chance > 0 {
                for &character_id in &player_character_ids {
                    if !world.character_weather_eligible(character_id) {
                        continue;
                    }
                    if runtime_random_below(100 * TICKS_PER_SECOND as i32 * 60)
                        >= lightning_chance * 100
                    {
                        continue;
                    }
                    let base_damage = lightning_strike_damage_amount(
                        runtime.weather.weather_intensity,
                        &mut runtime_random_below,
                    );
                    if world
                        .apply_lightning_strike_damage(character_id, base_damage)
                        .is_some()
                    {
                        world.queue_system_text(character_id, "CRACK! Lightning strikes you!");
                        if let Some(character) = world.characters.get(&character_id) {
                            let (x, y) = (character.x, character.y);
                            let weather_intensity = runtime.weather.weather_intensity;
                            broadcast_weather_thunder_effect(
                                &world,
                                &mut runtime,
                                x,
                                y,
                                12,
                                weather_intensity,
                            );
                        }
                        // C's own nearby-players text broadcast
                        // (`log_char(co, LOG_INFO, 0, "Lightning
                        // strikes nearby with a thunderous
                        // crack!")`, `weather.c:606-608`) is
                        // intentionally NOT ported: `log_char`'s
                        // own `LOG_INFO` gate is `if (type ==
                        // LOG_INFO && !char_see_char(cn, dat1))
                        // return 0;`, and this call site hardcodes
                        // `dat1 = 0`, so `char_see_char(co, 0)`
                        // always returns `0` (its own `co == 0`
                        // early-return) - the gate always fails
                        // and the message is *never* delivered to
                        // anyone in the real C server. Verified:
                        // no other C caller passes `dat1 = 0` to
                        // `LOG_INFO`; every other `LOG_INFO` call
                        // site passes a real acting character id.
                    }
                }
            }
        }
        // C `apply_elemental_debuffs` (`weather.c:614-655`),
        // called from the same per-player `apply_weather_effects`
        // tick hook as the damage/lightning rolls above: a
        // periodic (at most once per 10 real seconds per
        // character) flavor-text notification while standing in
        // wet/cold/scorching weather. Gated on the same
        // `character_weather_eligible` guards - see
        // `elemental_debuff_message`'s doc comment for why only
        // this notification (not the persistent debuff/expire
        // state) is ported.
        if runtime.weather.weather_effects & WEATHER_EFFECT_ELEMENTAL != 0 {
            if let Some(message) = elemental_debuff_message(elemental_debuff_type(
                runtime.weather.current_weather,
                runtime.weather.weather_intensity,
            )) {
                for &character_id in &player_character_ids {
                    if !world.character_weather_eligible(character_id) {
                        continue;
                    }
                    let last_notify = runtime
                        .weather
                        .elemental_debuff_last_notify
                        .get(&character_id)
                        .copied()
                        .unwrap_or(0);
                    if should_notify_elemental_debuff(last_notify, world.tick.0) {
                        runtime
                            .weather
                            .elemental_debuff_last_notify
                            .insert(character_id, world.tick.0);
                        world.queue_system_text(character_id, message);
                    }
                }
            }
        }
    }
    // C `lostcon_driver`'s `!ch[cn].player && ticker >
    // dat->timeout` branch + `exit_char`/`kick_char`: save and
    // despawn characters whose disconnect linger expired
    // without being reclaimed by a reconnect, plus its earlier
    // early-exit gauntlet (rest-area/arena tile, karma cutoff -
    // `lostcon_early_exit_characters`'s doc comment has the
    // full list) that leaves at once regardless of the
    // ordinary lagout timeout.
    let mut expired_lostcon = take_expired_lostcon_characters(&world, &mut runtime, world.tick.0);
    expired_lostcon.extend(take_lostcon_early_exit_characters(
        &world,
        &mut runtime,
        config.area_id,
    ));
    if !expired_lostcon.is_empty() {
        let expired_count = expired_lostcon.len();
        for (character_id, player, account_depot) in expired_lostcon {
            if let Some(repository) = &character_repository {
                if let Some(character) = world.characters.get(&character_id) {
                    let request = character_save_request(
                        &world,
                        &player,
                        character,
                        account_depot.as_ref(),
                        config.area_id,
                        config.mirror_id,
                    );
                    match repository.save_character_snapshot(request).await {
                        Ok(true) => {
                            info!(
                                character_id = character_id.0,
                                "saved DB-backed character snapshot on lostcon expiry"
                            );
                        }
                        Ok(false) => {
                            warn!(
                                character_id = character_id.0,
                                "DB character snapshot save was skipped by area guard on lostcon expiry"
                            );
                        }
                        Err(err) => {
                            warn!(character_id = character_id.0, error = %err, "failed to save DB-backed character snapshot on lostcon expiry");
                        }
                    }
                }
            }
            world.remove_character(character_id);
        }
        info!(
            expired_count,
            tick = world.tick.0,
            "despawned expired lostcon characters"
        );
    }
    let clan_relations: ClanRelations = world.clan_registry.relations().clone();
    world.tick_effects_with_attack_policy(|caster_id, caster, target, map| {
        if let Some(player) = runtime.player_for_character_mut(caster_id) {
            let attack_policy = RuntimePlayerAttackPolicy {
                attacker_runtime: &*player,
                clan_relations: &clan_relations,
            };
            let can_attack = can_attack_in_area_with_clan_policy(
                caster,
                target,
                map,
                config.area_id,
                &attack_policy,
            );
            if !can_attack {
                remove_stale_pvp_hate_if_effect_check_fails(player, caster, target, config.area_id);
            }
            can_attack
        } else {
            can_attack_in_area(caster, target, map, config.area_id)
        }
    });
    let timer_outcomes = world.process_due_timers(config.area_id);
    if !timer_outcomes.is_empty() {
        info!(
            count = timer_outcomes.len(),
            tick = world.tick.0,
            "processed timer callbacks"
        );
    }
    let mut edemon_gate_spawns = 0;
    let mut fdemon_gate_spawns = 0;
    let mut chest_spawns = 0;
    let mut swamp_spawns = 0;
    for outcome in &timer_outcomes {
        if let ugaris_core::item_driver::ItemDriverOutcome::EdemonGateSpawn {
            item_id,
            template,
            slot,
            x,
            y,
            ..
        } = outcome
        {
            if spawn_edemon_gate_character(
                &mut world,
                &mut zone_loader,
                &mut runtime,
                *item_id,
                template,
                *slot,
                *x,
                *y,
            ) {
                edemon_gate_spawns += 1;
            }
        }
        if let ugaris_core::item_driver::ItemDriverOutcome::ChestSpawn {
            item_id,
            template,
            x,
            y,
            ..
        } = outcome
        {
            if spawn_chestspawn_character(
                &mut world,
                &mut zone_loader,
                &mut runtime,
                *item_id,
                template,
                *x,
                *y,
            ) {
                chest_spawns += 1;
            }
        }
        if let ugaris_core::item_driver::ItemDriverOutcome::SwampSpawn {
            item_id,
            template,
            x,
            y,
            ..
        } = outcome
        {
            if spawn_swampspawn_character(
                &mut world,
                &mut zone_loader,
                &mut runtime,
                *item_id,
                template,
                *x,
                *y,
            ) {
                swamp_spawns += 1;
            }
        }
        if let ugaris_core::item_driver::ItemDriverOutcome::FdemonGateSpawn {
            item_id,
            level,
            slot,
            x,
            y,
            ..
        } = outcome
        {
            if spawn_fdemon_gate_character(
                &mut world,
                &mut zone_loader,
                &mut runtime,
                *item_id,
                *level,
                *slot,
                *x,
                *y,
            ) {
                fdemon_gate_spawns += 1;
            }
        }
    }
    if edemon_gate_spawns != 0 {
        info!(
            count = edemon_gate_spawns,
            tick = world.tick.0,
            "spawned edemon gate characters"
        );
    }
    if chest_spawns != 0 {
        info!(
            count = chest_spawns,
            tick = world.tick.0,
            "spawned chestspawn characters"
        );
    }
    if swamp_spawns != 0 {
        info!(
            count = swamp_spawns,
            tick = world.tick.0,
            "spawned swampspawn characters"
        );
    }
    if fdemon_gate_spawns != 0 {
        info!(
            count = fdemon_gate_spawns,
            tick = world.tick.0,
            "spawned fdemon gate characters"
        );
    }
    let lq_spawn_requests = world.drain_pending_lq_npc_spawns();
    if !lq_spawn_requests.is_empty() {
        let mut lq_spawns = 0;
        for request in &lq_spawn_requests {
            if spawn_lq_npc_character(&mut world, &mut zone_loader, &mut runtime, request) {
                lq_spawns += 1;
            }
        }
        if lq_spawns != 0 {
            info!(
                count = lq_spawns,
                tick = world.tick.0,
                "spawned LQ NPC characters"
            );
        }
    }
    // C respawn_callback: recreate dead template NPCs at their
    // spawn tile, retrying every ten seconds while blocked.
    let respawn_requests = world.drain_pending_npc_respawns();
    if !respawn_requests.is_empty() {
        let mut respawned = 0;
        for request in &respawn_requests {
            if respawn_npc_character(&mut world, &mut zone_loader, &mut runtime, request) {
                respawned += 1;
            } else {
                world.schedule_npc_respawn_retry(request.slot);
            }
        }
        if respawned != 0 {
            info!(
                count = respawned,
                tick = world.tick.0,
                "respawned NPC characters"
            );
        }
    }
    // C kill_char give_exp: route kill experience through the
    // shared runtime EXP modifiers.
    for award in world.drain_pending_kill_exp() {
        let area_id = args.area_id;
        give_exp_with_runtime_modifiers(
            &mut world,
            award.killer_id,
            i64::from(award.exp),
            u32::from(area_id),
        );
    }
    // C kill_char achievement_add_enemy_killed/achievement_add_demons.
    for award in world.drain_pending_kill_achievements() {
        award_enemy_killed_achievement(
            &mut world,
            &mut runtime,
            &achievement_repository,
            award.killer_id,
            award.area_id,
            award.target_is_demon,
        )
        .await;
    }
    // C check_levelup achievement_check_level.
    for check in world.drain_pending_level_achievements() {
        award_level_achievement(
            &mut world,
            &mut runtime,
            &achievement_repository,
            check.character_id,
            check.level as i32,
            check.is_hardcore,
        )
        .await;
    }
    // C kill_char give_first_kill.
    for check in world.drain_pending_first_kill_checks() {
        apply_first_kill_check(
            &mut world,
            &mut runtime,
            &achievement_repository,
            i32::from(args.area_id),
            check,
        )
        .await;
    }
    // C kill_char check_military_solve.
    for check in world.drain_pending_military_mission_checks() {
        apply_military_mission_kill_check(&mut world, &mut runtime, check);
    }
    // C die_char apply_death_loot_for_template.
    let death_loot_rolls = world.drain_pending_death_loot_rolls();
    if !death_loot_rolls.is_empty() {
        let added = apply_pending_death_loot_rolls(
            &mut world,
            &runtime,
            &mut zone_loader,
            death_loot_rolls,
        );
        if added != 0 {
            info!(
                added,
                tick = world.tick.0,
                "rolled death-mode loot into corpses"
            );
        }
    }
    let timer_feedback = timer_outcome_feedback(&timer_outcomes);
    if !timer_feedback.is_empty() {
        let mut feedback_sessions = 0;
        for (character_id, message) in timer_feedback {
            let payload = ugaris_protocol::packet::system_text(&message);
            for (session_id, _) in runtime.sessions_for_character(character_id) {
                if runtime.send_to_session(session_id, payload.clone()) {
                    feedback_sessions += 1;
                }
            }
        }
        info!(
            feedback_sessions,
            tick = world.tick.0,
            "queued timer feedback"
        );
    }
    let due_tasks = world.scheduler.due_tasks(world.tick.0);
    if !due_tasks.is_empty() {
        info!(
            count = due_tasks.len(),
            tick = world.tick.0,
            "scheduled tasks are due"
        );
    }
}
