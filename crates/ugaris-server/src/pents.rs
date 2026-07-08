//! Server-side orchestration for the pentagram-quest per-player reward
//! pipeline (`src/area/4/pents.c`'s `add_pentagram_to_player`/
//! `complete_pentagram_quest`/`distribute_rewards_to_player`).
//!
//! `ugaris-core`'s `World` owns the system-wide solve state and item
//! mutation (`ugaris_core::world::pents`) but has no access to the
//! session-owned `PlayerRuntime` that carries each player's
//! `pentagram_debug: PentagramDebugData` - the same architectural split
//! documented on `World::pending_lostcon_hurt_events`. `World` queues a
//! [`ugaris_core::world::PentagramActivationEvent`] per activation
//! ([`World::apply_pentagram_activate`]); [`process_pentagram_activations`]
//! drains that queue every tick and calls `ugaris_core::pentagram`'s pure
//! per-player functions, applying the resulting exp/achievements/clan
//! bonus/text feedback here where `World`, `ServerRuntime`, and the DB
//! achievement repository are all available.
//!
//! Demon spawning (`spawn_demons_at_pentagram`) is orchestrated here too:
//! `World::drain_pending_pentagram_demon_spawns` queues a planned spawn
//! per activation/timer-reset/random-tick, and
//! [`process_pentagram_demon_spawns`] instantiates each `penterN` zone
//! template (needs `ZoneLoader`, which `World` doesn't have - same split
//! as the edemon/fdemon gate spawns in `crate::spawns`), drops it at the
//! pentagram, and calls back into `World` to finish the elite/lesser
//! mutation, loot-table roll, and slot bookkeeping.
//!
//! [`save_pentagram_record_scheduled`] restart-persists the lifetime
//! "most pentagrams activated in one run" record (C
//! `save_pentagram_record_scheduled`,
//! `src/system/database/database_pent_record.c`) via
//! `ugaris_db::PgPentagramRecordRepository`; the caller (`main.rs`) is
//! responsible for the two C call sites this mirrors: the periodic
//! `add_scheduled_task(..., 3600 * 4, ...)` cadence and `/saveall`'s
//! explicit call (`command.c:7470`). Not ported: the `CDR_TESTER` QA-bot
//! character driver - see `ugaris_core::world::pents`'s module doc
//! comment.

use super::*;
use crate::achievement::{
    award_pentagram_demon_lords_demise_achievement, award_pentagram_favored_by_fortune_achievement,
    award_pentagram_five_in_a_row_achievement, award_pentagram_lucky_achievement,
    award_pentagram_solve_achievement,
};
use ugaris_core::world::{
    level_value, DemonType, PentagramActivationEvent, PentagramDemonSpawnRequest,
};
use ugaris_db::PentagramRecordRepository as _;

/// Drains `World::drain_pending_pentagram_activations` and applies the
/// per-player half of C's pipeline for each queued activation. Call once
/// per tick, after the item-use dispatch loop that produces
/// `PentagramActivate` outcomes (mirrors where `tick_item_use_
/// completion.rs`'s other post-dispatch drains run).
pub(crate) async fn process_pentagram_activations(
    world: &mut World,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
) {
    let events = world.drain_pending_pentagram_activations();
    for event in events {
        apply_pentagram_activation(world, runtime, achievement_repository, event).await;
    }
}

/// C `handle_pentagram_interaction`'s player-activation branch plus
/// `check_for_quest_completion` (`pents.c:1456-1461`, `538-552`): applies
/// `add_pentagram_to_player` for the activator, then - on a solve -
/// `complete_pentagram_quest`'s reward fan-out to every eligible online
/// player.
async fn apply_pentagram_activation(
    world: &mut World,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    event: PentagramActivationEvent,
) {
    let PentagramActivationEvent {
        item_id,
        character_id,
        level,
        color,
        number,
        is_quest_solved,
        active_pentagrams,
        total_pentagrams,
    } = event;

    let Some(activator_name) = world
        .characters
        .get(&character_id)
        .map(|character| character.name.clone())
    else {
        return;
    };

    let Some(player) = runtime.player_for_character_mut(character_id) else {
        return;
    };
    let mut seed = world.legacy_random_seed;
    let mut record = world.pentagram_quest.pentagram_record;
    let mut record_holder = world.pentagram_quest.pentagram_record_holder.clone();
    let outcome = ugaris_core::pentagram::add_pentagram_to_player(
        &mut player.pentagram_debug,
        &world.settings,
        &mut seed,
        &mut record,
        &mut record_holder,
        item_id.0 as i32,
        level,
        color,
        number,
        is_quest_solved,
        active_pentagrams,
        total_pentagrams,
        &activator_name,
    );
    world.legacy_random_seed = seed;
    world.pentagram_quest.pentagram_record = record;
    world.pentagram_quest.pentagram_record_holder = record_holder;

    for message in outcome.messages {
        world.queue_system_text(character_id, message);
    }
    if outcome.lucky_hit {
        award_pentagram_lucky_achievement(world, runtime, achievement_repository, character_id)
            .await;
    }
    if outcome.second_lucky_hit {
        award_pentagram_favored_by_fortune_achievement(
            world,
            runtime,
            achievement_repository,
            character_id,
        )
        .await;
    }

    if !is_quest_solved {
        return;
    }

    let area_id = world.area_id;
    for player_id in eligible_pentagram_reward_players(world, area_id) {
        distribute_pentagram_reward(
            world,
            runtime,
            achievement_repository,
            player_id,
            character_id,
        )
        .await;
    }
}

/// C `complete_pentagram_quest`'s reward loop gate (`pents.c:568-579`):
/// every currently-instantiated `CF_PLAYER` character, minus area 25's
/// "no messages for players in RWW" exclusion (`ch[player_id].x > 107`).
fn eligible_pentagram_reward_players(world: &World, area_id: u16) -> Vec<CharacterId> {
    world
        .characters
        .values()
        .filter(|character| character.flags.contains(CharacterFlags::PLAYER))
        .filter(|character| !(area_id == 25 && character.x > 107))
        .map(|character| character.id)
        .collect()
}

/// C `distribute_rewards_to_player` (`pents.c:593-673`).
async fn distribute_pentagram_reward(
    world: &mut World,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    player_id: CharacterId,
    solver_id: CharacterId,
) {
    let Some(pent_it) = runtime
        .player_for_character(player_id)
        .map(|player| player.pentagram_debug.pent_it)
    else {
        return;
    };
    let had_combo_before_reset = runtime
        .player_for_character(player_id)
        .is_some_and(|player| player.pentagram_debug.status != 0);
    world.reset_pentagram_colors(&pent_it, had_combo_before_reset);

    let Some(player) = runtime.player_for_character_mut(player_id) else {
        return;
    };
    let (had_combo, exp_reward) =
        ugaris_core::pentagram::distribute_rewards_reset(&mut player.pentagram_debug);

    if had_combo {
        award_pentagram_five_in_a_row_achievement(
            world,
            runtime,
            achievement_repository,
            player_id,
        )
        .await;
    }

    let Some(character) = world.characters.get(&player_id) else {
        return;
    };
    let level = character.level;
    let is_hardcore = character.flags.contains(CharacterFlags::HARDCORE);

    let level_band = i64::from(level_value(level));
    let divisor_primary = i64::from(world.settings.get_exp_level_divisor_primary().max(1));
    let solve_multiplier = world.settings.get_exp_solve_multiplier();
    let actual_exp =
        (level_band / divisor_primary).min((f64::from(exp_reward) * solve_multiplier) as i64);
    world.give_exp(player_id, actual_exp, u32::from(world.area_id));

    let hardcore_bonus = if is_hardcore {
        world.settings.hardcore_exp_bonus
    } else {
        1.0
    };
    let displayed_exp = (actual_exp as f64 * hardcore_bonus * world.settings.exp_modifier) as i64;

    let Some(solver_name) = world
        .characters
        .get(&solver_id)
        .map(|character| character.name.clone())
    else {
        return;
    };
    world.queue_system_text(
        player_id,
        format!(
            "{solver_name} solved the pentagram quest (tm). You got {displayed_exp} experience points!"
        ),
    );

    let training_power = world.pentagram_quest.training_power;
    if training_power >= 0 {
        world.queue_system_text(
            player_id,
            format!(
                "Training area power setting now at {:.2}%.",
                100.0 / 32000.0 * f64::from(training_power)
            ),
        );
    } else {
        world.queue_system_text(
            player_id,
            format!(
                "Training area power setting down to 0.00%, {:.2}% underpowered.",
                -100.0 / 32000.0 * f64::from(training_power)
            ),
        );
    }

    world.queue_system_text(player_id, "#30  - Solved -".to_string());

    award_pentagram_solve_achievement(
        world,
        runtime,
        achievement_repository,
        player_id,
        i32::from(world.area_id),
    )
    .await;

    let solver_clan = match world.characters.get_mut(&solver_id) {
        Some(solver_character) => world.clan_registry.get_char_clan(solver_character),
        None => None,
    };
    if let Some(solver_clan_nr) = solver_clan {
        let clan_name = world
            .clan_registry
            .name(solver_clan_nr)
            .unwrap_or_default()
            .to_string();
        world.queue_system_text(
            player_id,
            format!("This solve goes to the {clan_name} clan!"),
        );

        let player_clan = match world.characters.get_mut(&player_id) {
            Some(player_character) => world.clan_registry.get_char_clan(player_character),
            None => None,
        };
        if player_clan == Some(solver_clan_nr) {
            let clan_bonus_count = world.clan_registry.bonus_level(solver_clan_nr, 0);
            if clan_bonus_count > 0 {
                let max_clan_bonus_percent = world.settings.get_max_clan_bonus_percent();
                let clan_exp = i64::from(exp_reward)
                    * i64::from(clan_bonus_count.min(max_clan_bonus_percent))
                    / 100;
                if clan_exp != 0 {
                    let divisor_secondary =
                        i64::from(world.settings.get_exp_level_divisor_secondary().max(1));
                    let clan_reflection_multiplier =
                        world.settings.get_exp_clan_reflection_multiplier();
                    let actual_clan_exp = (level_band / divisor_secondary)
                        .min((clan_exp as f64 * clan_reflection_multiplier) as i64);
                    world.give_exp(player_id, actual_clan_exp, u32::from(world.area_id));
                    let displayed_clan_exp =
                        (actual_clan_exp as f64 * hardcore_bonus * world.settings.exp_modifier)
                            as i64;
                    world.queue_system_text(
                        player_id,
                        format!(
                            "Your clan's jewels reflected {displayed_clan_exp} exp of the solve to you."
                        ),
                    );
                }
            }
        }
    }

    let pent_cnt = runtime
        .player_for_character(player_id)
        .map(|player| player.pentagram_debug.pent_cnt)
        .unwrap_or_default();
    let record = world.pentagram_quest.pentagram_record;
    let holder = world.pentagram_quest.pentagram_record_holder.clone();
    world.queue_system_text(
        player_id,
        format!(
            "The current record is {record} pentagrammas in one run, held by {holder}. You have {pent_cnt} pentagrammas so far."
        ),
    );
}

/// Drains `World::drain_pending_pentagram_demon_spawns` and instantiates
/// each planned demon from its `penterN` zone template. Call once per
/// tick, alongside [`process_pentagram_activations`].
pub(crate) fn process_pentagram_demon_spawns(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
) {
    for request in world.drain_pending_pentagram_demon_spawns() {
        spawn_demons_at_pentagram(world, loader, runtime, request);
    }
}

/// C `spawn_demons_at_pentagram` (`pents.c:1004-1091`): fills up to
/// `World::pentagram_max_spawns` empty/stale slots (see
/// `World::pentagram_spawn_slot_is_stale`) with newly created `penterN`
/// demons, stopping early - matching C's `break` - the moment a template
/// instantiation or map placement fails.
fn spawn_demons_at_pentagram(
    world: &mut World,
    loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    request: PentagramDemonSpawnRequest,
) {
    let PentagramDemonSpawnRequest {
        item_id,
        mut spawn_count,
        level,
    } = request;
    let max_spawns = world.pentagram_max_spawns(level).max(0) as usize;

    for index in 0..max_spawns {
        if spawn_count <= 0 {
            break;
        }
        if !world.pentagram_spawn_slot_is_stale(item_id, index) {
            continue;
        }

        let demon_type = world.roll_pentagram_demon_type();
        let template = world.pentagram_demon_template_name(level);
        let character_id = runtime.allocate_character_id();
        let Ok((character, inventory_items)) =
            loader.instantiate_character_template(&template, character_id)
        else {
            // C `if (!character_id) break;`.
            break;
        };
        let serial = character.serial;
        let original_class = character.class;
        let demon_level = character.level;

        // C `if (item_drop_char(item_id, character_id)) { ... } else {
        // destroy_char(character_id); break; }` - the character was never
        // added to `world.characters` on failure, so there's nothing to
        // destroy here.
        if world
            .spawn_character_from_item_drop(character, item_id)
            .is_none()
        {
            break;
        }
        for item in inventory_items {
            world.items.insert(item.id, item);
        }

        world.finish_pentagram_demon_spawn(character_id, demon_type, original_class);

        let loot_table = World::pentagram_demon_loot_table_id(
            demon_level,
            matches!(demon_type, DemonType::Elite),
        );
        world.loot_apply_to_npc(loader, character_id, loot_table);

        world.apply_pentagram_spawn_result(item_id, index, character_id, serial);
        spawn_count -= 1;
    }
}

/// Drains `World::drain_pending_penter_demon_lords_demise_awards` and
/// awards `ACHIEVEMENT_DEMON_LORDS_DEMISE` to each queued killer. Call
/// once per tick, alongside [`process_pentagram_activations`].
pub(crate) async fn process_penter_demon_lords_demise_awards(
    world: &mut World,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
) {
    for player_id in world.drain_pending_penter_demon_lords_demise_awards() {
        award_pentagram_demon_lords_demise_achievement(
            world,
            runtime,
            achievement_repository,
            player_id,
        )
        .await;
    }
}

/// C `load_pentagram_record` (`database_pent_record.c:62-90`), called
/// once before the game loop starts (`initialize_pentagram_system`,
/// `pents.c:369`). Leaves `world.pentagram_quest` at its `Default` (`0`/
/// `"Nobody"`) when nothing has ever been saved for this area, matching
/// C's `if (!load_pentagram_record(...)) { ... }` no-op fallthrough.
pub(crate) async fn load_pentagram_record_at_startup(
    world: &mut World,
    repository: &Option<ugaris_db::PgPentagramRecordRepository>,
) {
    let Some(repository) = repository else {
        return;
    };
    match repository.load(i32::from(world.area_id)).await {
        Ok(Some(row)) => {
            world.pentagram_quest.pentagram_record = row.record_count;
            world.pentagram_quest.pentagram_record_holder = row.char_name;
        }
        Ok(None) => {}
        Err(err) => {
            warn!(error = %err, "failed to load pentagram record from database");
        }
    }
}

/// C `save_pentagram_record_scheduled` (`database_pent_record.c:134-
/// 146`): unconditionally re-saves the current in-memory record whenever
/// `record > 0` - C has no "did it actually change" check, so neither
/// does this. Wired to two of its three C call sites: the periodic
/// `add_scheduled_task(..., 3600 * 4, "PentagramRecords", 1)` cadence
/// (caller checks `world.tick.0 % (TICKS_PER_SECOND * 3600 * 4) == 0`)
/// and `/saveall`'s explicit call (`command.c:7470`, wired in
/// `tick_client_actions::process_queued_client_actions`'s
/// `result.save_all_requested` arm). The third call site,
/// `exit_database`'s unconditional shutdown flush, has no Rust
/// equivalent yet - no repository in this codebase has a shutdown-save
/// routine to mirror it against.
pub(crate) async fn save_pentagram_record_scheduled(
    world: &World,
    repository: &Option<ugaris_db::PgPentagramRecordRepository>,
) {
    if world.pentagram_quest.pentagram_record <= 0 {
        return;
    }
    let Some(repository) = repository else {
        return;
    };
    let row = ugaris_db::PentagramRecordRow {
        record_count: world.pentagram_quest.pentagram_record,
        char_name: world.pentagram_quest.pentagram_record_holder.clone(),
    };
    if let Err(err) = repository.save(i32::from(world.area_id), &row).await {
        warn!(error = %err, "failed to save pentagram record to database");
    }
}
