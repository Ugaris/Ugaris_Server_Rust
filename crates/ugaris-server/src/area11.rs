//! Server-side wiring for area 11's Islena boss NPC (`CDR_PALACEISLENA`/
//! `ugaris_core::world::islena::process_islena_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established in
//! `area3.rs`: [`islena_player_facts`] snapshots the per-player
//! `islena_state` the dialogue/aggro state machine needs before the tick,
//! and [`apply_islena_events`] writes the returned state updates back
//! afterward. The `ACHIEVEMENT_LADYKILLER` award itself is a separate,
//! once-per-tick drain ([`process_islena_ladykiller_awards`]) since it
//! needs the async DB-backed achievement repository that `World::
//! apply_islena_death` (queued straight from `kill_character_followup`,
//! not from this tick pass) doesn't have access to - see that function's
//! own doc comment.

use super::*;
use crate::achievement::award_islena_ladykiller_achievement;
use ugaris_core::world::{IslenaOutcomeEvent, IslenaPlayerFacts};

pub(crate) fn islena_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, IslenaPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                IslenaPlayerFacts {
                    islena_state: player.islena_state,
                },
            ))
        })
        .collect()
}

/// Applies each [`IslenaOutcomeEvent`] queued by
/// `World::process_islena_actions`.
pub(crate) fn apply_islena_events(
    runtime: &mut ServerRuntime,
    events: Vec<IslenaOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            IslenaOutcomeEvent::UpdateState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.islena_state = new_state;
                applied += 1;
            }
        }
    }
    applied
}

/// Drains `World::drain_pending_islena_ladykiller_awards` and awards
/// `ACHIEVEMENT_LADYKILLER` to each queued winner. Call once per tick,
/// alongside the pentagram award drains in `tick_item_use_completion.rs`.
pub(crate) async fn process_islena_ladykiller_awards(
    world: &mut World,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
) {
    for player_id in world.drain_pending_islena_ladykiller_awards() {
        award_islena_ladykiller_achievement(world, runtime, achievement_repository, player_id)
            .await;
    }
}
