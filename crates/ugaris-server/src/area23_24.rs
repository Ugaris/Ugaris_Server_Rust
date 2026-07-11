//! Areas 23/24 (`src/area/23_24/strategy.c`) server-side glue for
//! `CDR_STRATEGY_BOSS`: threads `PlayerRuntime`-resident `StrategyPpd`
//! into `ugaris_core::world::npc::area23_24::boss`'s per-tick dialogue
//! chain, which cannot reach `PlayerRuntime` itself (`World` only sees
//! `Character`, not the session-owned `PlayerRuntime` - same split as
//! `area8.rs`'s identical `CDR_FDEMON_BOSS` glue).

use super::*;

/// C `ch_driver`'s `CDR_STRATEGY_BOSS` case (`strategy.c:1614-1616`), run
/// once per live Cinciac per tick: the `NT_TEXT` "repeat"/"military
/// rank"/"levels and experience" commands, then the throttled per-player
/// `NT_CHAR` greeting dialogue. Returns the number of players whose
/// `StrategyPpd` state changed.
pub(crate) fn apply_strategy_boss_tick(
    world: &mut World,
    runtime: &mut ServerRuntime,
    config: &ServerConfig,
) -> usize {
    let mut applied = 0;
    let area_id = u32::from(config.area_id);
    let now_ticks = world.tick.0 as i64;

    for boss_id in world.strategy_boss_character_ids() {
        let text_commands = world.strategy_boss_process_text_messages(boss_id);
        for (player_id, command) in text_commands {
            if let Some(player) = runtime.player_for_character_mut(player_id) {
                world.strategy_boss_apply_text_command(
                    boss_id,
                    player_id,
                    &mut player.strategy,
                    command,
                    area_id,
                );
                applied += 1;
            }
        }

        for player_id in world.strategy_boss_sighted_players(boss_id) {
            if let Some(player) = runtime.player_for_character_mut(player_id) {
                let before = player.strategy.clone();
                world.strategy_boss_greet_player(
                    boss_id,
                    player_id,
                    &mut player.strategy,
                    now_ticks,
                );
                if player.strategy != before {
                    applied += 1;
                }
            }
        }
    }
    applied
}
