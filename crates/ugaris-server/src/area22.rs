//! Server-side wiring for area 22's lab2 graveyard chapel keeper
//! (`CDR_LAB2HERALD`/`ugaris_core::world::npc::area22::lab2_herald::
//! process_lab2_herald_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established in
//! `area11.rs`: [`lab2_herald_player_facts`] snapshots the per-player
//! `ppd->herald_talkstep` the dialogue state machine needs before the
//! tick, and [`apply_lab2_herald_events`] writes the returned talkstep
//! update back afterward.

use super::*;
use ugaris_core::world::{Lab2HeraldOutcomeEvent, Lab2HeraldPlayerFacts};

pub(crate) fn lab2_herald_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, Lab2HeraldPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                Lab2HeraldPlayerFacts {
                    herald_talkstep: player.legacy_lab2_herald_talkstep(),
                },
            ))
        })
        .collect()
}

/// Applies each [`Lab2HeraldOutcomeEvent`] queued by
/// `World::process_lab2_herald_actions`.
pub(crate) fn apply_lab2_herald_events(
    runtime: &mut ServerRuntime,
    events: Vec<Lab2HeraldOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            Lab2HeraldOutcomeEvent::UpdateTalkstep {
                player_id,
                new_value,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_legacy_lab2_herald_talkstep(new_value);
                applied += 1;
            }
        }
    }
    applied
}
