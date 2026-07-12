//! Server-side wiring for area 34's Teufel Quest NPC (`CDR_TEUFELQUEST`,
//! `ugaris_core::world::npc::area34::teufelquest::process_teufelquest_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established by
//! `area33.rs`.

use std::collections::HashMap;

use super::*;
use ugaris_core::world::npc::area34::teufelquest::{
    TeufelQuestOutcomeEvent, TeufelQuestPlayerFacts,
};

pub(crate) fn teufelquest_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, TeufelQuestPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                TeufelQuestPlayerFacts {
                    teufel_rat_kills: player.teufel_rat_kills,
                    teufel_rat_score: player.teufel_rat_score,
                },
            ))
        })
        .collect()
}

/// Applies each [`TeufelQuestOutcomeEvent`] queued by `World::
/// process_teufelquest_actions`.
pub(crate) fn apply_teufelquest_events(
    runtime: &mut ServerRuntime,
    events: Vec<TeufelQuestOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            TeufelQuestOutcomeEvent::SetRatKillsScore {
                player_id,
                kills,
                score,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.teufel_rat_kills = kills;
                player.teufel_rat_score = score;
                applied += 1;
            }
        }
    }
    applied
}
