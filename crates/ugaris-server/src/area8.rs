//! Area 8 (`src/area/8/fdemon.c`) server-side glue for `CDR_FDEMON_BOSS`:
//! threads `PlayerRuntime`-resident `farmy_ppd` fields into `ugaris_core::
//! world::npc::area8::fdemon_boss`'s per-tick dialogue chain, which cannot
//! reach `PlayerRuntime` itself (see that module's own doc comment for the
//! full split rationale - same class of split as `FdemonLoaderChanged`'s
//! dispatch in `tick_item_use_edemon_fdemon.rs`).

use super::*;
use ugaris_core::world::{fdemon_boss_repeat_reset, FdemonBossPlayerFacts};

/// C's `boss_timer` comparisons use `realtime` (wall-clock seconds); this
/// port substitutes `World::tick` (game ticks, `TICKS_PER_SECOND` per real
/// second at normal speed) for the same "how long since we last spoke"
/// gate - the same tick-for-realtime substitution already established by
/// `world::fdemon`'s waypoint `last_enemy_tick` bookkeeping.
const FDEMON_BOSS_TIMER_THROTTLE_TICKS: i64 =
    (TICKS_PER_SECOND * ugaris_core::world::FDEMON_BOSS_TIMER_THROTTLE_SECONDS as u64) as i64;

/// C `ch_driver`'s `CDR_FDEMON_BOSS` case, run once per live Commander per
/// tick: the `NT_TEXT` "repeat" stage reset, then the throttled per-player
/// `NT_CHAR` greeting dialogue (see `fdemon_boss`'s module doc comment for
/// why the relative order between the two doesn't change any observable
/// per-player outcome). Returns the number of players whose `farmy_ppd`
/// state changed.
pub(crate) fn apply_fdemon_boss_tick(
    world: &mut World,
    runtime: &mut ServerRuntime,
    config: &ServerConfig,
) -> usize {
    let mut applied = 0;
    let area_id = u32::from(config.area_id);
    for boss_id in world.fdemon_boss_character_ids() {
        for player_id in world.fdemon_boss_process_text_messages(boss_id) {
            let Some(player) = runtime.player_for_character_mut(player_id) else {
                continue;
            };
            if let Some((new_stage, new_timer)) =
                fdemon_boss_repeat_reset(player.farmy_boss_stage())
            {
                player.set_farmy_boss_stage(new_stage);
                player.set_farmy_boss_timer(new_timer);
                applied += 1;
            }
        }

        let now_ticks = world.tick.0 as i32;
        for player_id in world.fdemon_boss_sighted_players(boss_id) {
            let Some(player) = runtime.player_for_character_mut(player_id) else {
                continue;
            };
            if i64::from(now_ticks.saturating_sub(player.farmy_boss_timer()))
                <= FDEMON_BOSS_TIMER_THROTTLE_TICKS
            {
                continue;
            }
            let facts = FdemonBossPlayerFacts {
                boss_stage: player.farmy_boss_stage(),
                boss_counter: player.farmy_boss_counter(),
                boss_reported: player.farmy_boss_reported(),
            };

            let update = world.fdemon_boss_greet_player(boss_id, player_id, facts, area_id);

            let Some(player) = runtime.player_for_character_mut(player_id) else {
                continue;
            };
            if let Some(new_stage) = update.new_stage {
                player.set_farmy_boss_stage(new_stage);
            }
            if let Some(new_counter) = update.new_counter {
                player.set_farmy_boss_counter(new_counter);
            }
            if let Some(new_reported) = update.new_reported {
                player.set_farmy_boss_reported(new_reported);
            }
            if update.timer_touched {
                player.set_farmy_boss_timer(now_ticks);
                applied += 1;
            }
        }
    }
    applied
}
