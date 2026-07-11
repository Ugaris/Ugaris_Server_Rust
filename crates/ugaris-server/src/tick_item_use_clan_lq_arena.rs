//! Completed-action-outcome handling: the clan-spawn/LQ/arena family
//! (`src/area/30/clanmaster.c` clan-spawner jewel machinery, `src/area/20/
//! lq.c` live-quest entrance gating, `src/system/arena.c` toplist request)
//! of `ItemDriverOutcome` variants. Split out of the giant `match outcome
//! { ... }` block that still lives inline in `main.rs`'s `tick.tick()` arm
//! (P0.5 "Finish main() phase decomposition" - REMAINING note: the
//! completed-action-outcome handling needs splitting by completed-action-
//! kind family across several files, not just relocation, because the
//! whole match is too large to move verbatim into one file). Warp, chests,
//! dungeon, ice/palace, Teufel, skel-raise, Edemon/Fdemon, and transport
//! were sliced first; this is the ninth family slice. The rest of the
//! match (shrines, xmas, swamp, burndown, key-assembly) is still inline in
//! `main.rs` pending further slices.

use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn dispatch_clan_lq_arena_outcome(
    world: &mut World,
    runtime: &mut ServerRuntime,
    zone_loader: &mut ZoneLoader,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
    config: &ServerConfig,
    outcome: ugaris_core::item_driver::ItemDriverOutcome,
    feedback: &mut Vec<(CharacterId, String)>,
    executed: &mut i32,
    blocked: &mut i32,
) {
    match outcome {
        ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnExit {
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
        ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnExitBusy { character_id, .. } => {
            feedback.push((
                character_id,
                "Please try again soon. Target is busy".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnLevelTooHigh {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "Thou mayest not use this clan spawner for thy level is too great.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnContested {
            character_id, ..
        } => {
            feedback.push((
                character_id,
                "Thou mayest not use this clan spawner while others can touch it.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnCountdown {
            character_id,
            remaining_minutes,
            freq_hours,
            god_added,
            ..
        } => {
            if god_added {
                feedback.push((
                    character_id,
                    "A jewel has been added to the clan spawner.".to_string(),
                ));
            }
            feedback.push((
                character_id,
                format!(
                    "{:02}:{:02} to go, about one jewel every {} hours.",
                    remaining_minutes / 60,
                    remaining_minutes % 60,
                    freq_hours
                ),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnAward {
            character_id,
            level,
            ..
        } => {
            // C fires the "won a Jewel" broadcast/clan-log
            // (`clanmaster.c:1373-1397`) unconditionally, before
            // even calling `award_clan_jewel` - it never checks
            // that call's return value, so the announcement
            // still fires even if item delivery fails (e.g. a
            // full inventory).
            world.resolve_clan_spawn_jewel_award(character_id, level);
            if grant_clan_jewel(world, zone_loader, character_id) {
                *executed += 1;
            } else {
                *blocked += 1;
            }
        }
        ugaris_core::item_driver::ItemDriverOutcome::ClanSpawnTimer { .. } => {
            *executed += 1;
        }
        // `LqTicker`/`StrTicker` are only ever produced by their driver
        // when `character_id == 0` (a timer callback), a case that never
        // reaches this player-`item_use`-completion dispatcher in
        // practice - the real dispatch point (`World::
        // apply_item_driver_outcome`, called from `process_due_timers`)
        // already applies `discover_lq_doors_once`/`queue_due_lq_npc_
        // respawns`/`str_ticker` *and* the self-reschedule for both. This
        // arm only exists so the shared `match` in
        // `tick_item_use_completion.rs` compiles against the full
        // `ItemDriverOutcome` union it dispatches through; `str_ticker`'s
        // reward-event drain (`apply_strategy_reward_events`, needs
        // `ServerRuntime`, unavailable to the pure `World` dispatch point
        // above) is applied from the real call site instead -
        // `tick_world.rs`'s `timer_outcomes` loop, the same "match the
        // outcome after `process_due_timers`" precedent as
        // `EdemonGateSpawn`/`ChestSpawn`/etc.
        ugaris_core::item_driver::ItemDriverOutcome::LqTicker { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::StrTicker { .. } => {
            *executed += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::LqEntranceClosed { character_id, .. } => {
            feedback.push((
                character_id,
                "No quest is in progress, you may not enter.".to_string(),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::LqEntranceLevelBlocked {
            character_id,
            min_level,
            max_level,
            ..
        } => {
            feedback.push((
                character_id,
                format!("This quest is for levels {min_level} to {max_level}, you may not enter."),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::LqEntranceUndefined {
            character_id, ..
        } => {
            feedback.push((character_id, "No entrance defined, bad quest.".to_string()));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::LqEntrancePenalty {
            character_id,
            remaining_seconds,
            ..
        } => {
            feedback.push((
                character_id,
                format!(
                    "You may not enter again yet. Your remaining penalty is: {:.2} minutes.",
                    remaining_seconds as f64 / 60.0
                ),
            ));
            *blocked += 1;
        }
        ugaris_core::item_driver::ItemDriverOutcome::ArenaToplist { character_id, .. } => {
            // C `toplist_driver` (`arena.c:1045-1087`): top-10 lines,
            // a +/-5 window around the reader's own rank, then their
            // own score/wins/losses summary line.
            if let Some(player) = runtime.player_for_character(character_id) {
                let entries = world.arena_toplist_entries();
                let lines = ugaris_core::item_driver::arena_toplist_lines(
                    &entries,
                    player.arena_score(),
                    player.arena_wins(),
                    player.arena_losses(),
                    player.arena_fights(),
                );
                for line in lines {
                    feedback.push((character_id, line));
                }
            }
            *executed += 1;
        }
        _ => {}
    }
}
