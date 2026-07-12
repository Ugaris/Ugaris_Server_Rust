//! Server-side wiring for area 36's Caligar entrance-guard NPCs
//! (`CDR_CALIGARGUARD`/`CDR_CALIGARGUARD2`,
//! `ugaris_core::world::npc::area36::{caligar_guard,caligar_guard2}::
//! process_*_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established by
//! `area31.rs`: each `apply_*_events` applies the returned outcome events.
//! Neither driver needs `ZoneLoader` (no item is ever created or handed
//! out by either), unlike most other area glue files.

use std::collections::HashMap;

use super::*;
use ugaris_core::world::npc::area36::caligar_guard::{
    CaligarGuardOutcomeEvent, CaligarGuardPlayerFacts,
};
use ugaris_core::world::npc::area36::caligar_guard2::{
    CaligarGuard2OutcomeEvent, CaligarGuard2PlayerFacts,
};

pub(crate) fn caligar_guard_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, CaligarGuardPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                CaligarGuardPlayerFacts {
                    guard_state: player.caligar_guard_state(),
                    guard_last_talk: player.caligar_guard_last_talk(),
                },
            ))
        })
        .collect()
}

/// Applies each [`CaligarGuardOutcomeEvent`] queued by `World::
/// process_caligar_guard_actions`. Every variant only touches
/// `PlayerRuntime`.
pub(crate) fn apply_caligar_guard_events(
    runtime: &mut ServerRuntime,
    events: Vec<CaligarGuardOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            CaligarGuardOutcomeEvent::AdvanceGuardTalk {
                player_id,
                new_state,
                realtime_seconds,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_caligar_guard_talk(new_state, realtime_seconds);
                applied += 1;
            }
            CaligarGuardOutcomeEvent::ResetGuardStateTimeout { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.reset_caligar_guard_state_timeout();
                applied += 1;
            }
            CaligarGuardOutcomeEvent::ResetGuardStateIfThree { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.reset_caligar_guard_if_state_three();
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn caligar_guard2_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, CaligarGuard2PlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                CaligarGuard2PlayerFacts {
                    guard2_last_talk: player.caligar_guard2_last_talk(),
                },
            ))
        })
        .collect()
}

/// Applies each [`CaligarGuard2OutcomeEvent`] queued by `World::
/// process_caligar_guard2_actions`. Only variant touches `PlayerRuntime`.
pub(crate) fn apply_caligar_guard2_events(
    runtime: &mut ServerRuntime,
    events: Vec<CaligarGuard2OutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            CaligarGuard2OutcomeEvent::UpdateGuard2LastTalk {
                player_id,
                realtime_seconds,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_caligar_guard2_last_talk(realtime_seconds);
                applied += 1;
            }
        }
    }
    applied
}
