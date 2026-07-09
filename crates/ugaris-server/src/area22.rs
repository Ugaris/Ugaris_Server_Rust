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
use ugaris_core::world::{
    Lab2DeamonOutcomeEvent, Lab2DeamonPlayerFacts, Lab2HeraldOutcomeEvent, Lab2HeraldPlayerFacts,
};

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

/// Server-side wiring for area 22's family-vault guardian
/// (`CDR_LAB2DEAMON`/`ugaris_core::world::npc::area22::lab2_deamon::
/// process_lab2_deamon_actions`). Same `World`/`PlayerRuntime` split as
/// [`lab2_herald_player_facts`]/[`apply_lab2_herald_events`] above:
/// [`lab2_deamon_player_facts`] snapshots `PlayerRuntime::
/// lab2_deamon_checked` before the tick, [`apply_lab2_deamon_events`]
/// writes it back plus applies the player-halt side effect afterward.
pub(crate) fn lab2_deamon_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, Lab2DeamonPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                Lab2DeamonPlayerFacts {
                    deamon_checked: player.lab2_deamon_checked,
                },
            ))
        })
        .collect()
}

/// Applies each [`Lab2DeamonOutcomeEvent`] queued by
/// `World::process_lab2_deamon_actions`.
pub(crate) fn apply_lab2_deamon_events(
    runtime: &mut ServerRuntime,
    events: Vec<Lab2DeamonOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            Lab2DeamonOutcomeEvent::MarkDeamonChecked { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.lab2_deamon_checked = true;
                applied += 1;
            }
            Lab2DeamonOutcomeEvent::HaltPlayer { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.driver_halt();
                applied += 1;
            }
        }
    }
    applied
}
