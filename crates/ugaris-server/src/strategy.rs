//! Server-side wiring for `ugaris-core`'s Areas 23/24 strategy minigame
//! per-tick mission-lifecycle driver (`World::str_ticker`, see
//! `ugaris-core`'s `world/strategy.rs` module doc comment for the C
//! source cross-reference).
//!
//! `World::str_reward_winner` (C `reward_winner`, `strategy.c:428-454`)
//! can't reach session-owned `PlayerRuntime::strategy` directly, so it
//! only queues a [`ugaris_core::world::StrategyRewardEvent`] with the
//! winning character id; [`apply_strategy_reward_events`] drains that
//! queue and applies `ugaris_core::world::apply_strategy_mission_win`
//! against the real `PlayerRuntime::strategy`, rendering the exact C
//! `log_char` text this port's pure function couldn't (same
//! `World`/`PlayerRuntime` split as `crate::military`'s
//! `apply_military_mission_kill_check`).

use super::*;
use ugaris_core::world::{apply_strategy_mission_win, StrategyWinOutcome};

pub(crate) fn apply_strategy_reward_events(world: &mut World, runtime: &mut ServerRuntime) {
    for event in world.drain_pending_strategy_rewards() {
        let Some(player) = runtime.player_for_character_mut(event.character_id) else {
            continue;
        };

        // C `reward_winner`: the "Congratulations, you won!" `log_char`
        // fires unconditionally, before even checking `current_mission`'s
        // range (`strategy.c:436-437`).
        world.queue_system_text(event.character_id, "Congratulations, you won!".to_string());

        let mission_index = player.strategy.current_mission;
        match apply_strategy_mission_win(&mut player.strategy, mission_index) {
            StrategyWinOutcome::BadMissionIndex => {
                world.queue_system_text(event.character_id, "Please report bug #443f".to_string());
            }
            StrategyWinOutcome::Rewarded { exp } => {
                world.queue_system_text(
                    event.character_id,
                    format!("You received {exp} strategy experience points."),
                );
            }
            StrategyWinOutcome::NoReward => {}
        }
    }
}
