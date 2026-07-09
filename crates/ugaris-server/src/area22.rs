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
    Lab3PassguardOutcomeEvent, Lab3PassguardPlayerFacts, Lab3PrisonerOutcomeEvent,
    Lab3PrisonerPlayerFacts,
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

/// Server-side wiring for area 22's lab3 password-gate guard
/// (`CDR_LAB3PASSGUARD`/`ugaris_core::world::npc::area22::lab3_passguard::
/// process_lab3_passguard_actions`). Same `World`/`PlayerRuntime` split as
/// above: [`lab3_passguard_player_facts`] snapshots `ppd->guard_talkstep`/
/// `password1`/`password2` before the tick, [`apply_lab3_passguard_events`]
/// writes the talkstep back afterward.
pub(crate) fn lab3_passguard_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, Lab3PassguardPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            let mut password1 = [0u8; 8];
            let bytes = player.legacy_lab3_password1();
            password1[..bytes.len().min(8)].copy_from_slice(&bytes[..bytes.len().min(8)]);
            let mut password2 = [0u8; 8];
            let bytes = player.legacy_lab3_password2();
            password2[..bytes.len().min(8)].copy_from_slice(&bytes[..bytes.len().min(8)]);
            Some((
                character_id,
                Lab3PassguardPlayerFacts {
                    guard_talkstep: player.legacy_lab3_guard_talkstep(),
                    password1,
                    password2,
                },
            ))
        })
        .collect()
}

/// Applies each [`Lab3PassguardOutcomeEvent`] queued by
/// `World::process_lab3_passguard_actions`.
pub(crate) fn apply_lab3_passguard_events(
    runtime: &mut ServerRuntime,
    events: Vec<Lab3PassguardOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            Lab3PassguardOutcomeEvent::SetGuardTalkstep { player_id, value } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_legacy_lab3_guard_talkstep(value);
                applied += 1;
            }
        }
    }
    applied
}

/// Server-side wiring for area 22's lab3 mute prisoner
/// (`CDR_LAB3PRISONER`/`ugaris_core::world::npc::area22::lab3_prisoner::
/// process_lab3_prisoner_actions`). Same split as above; additionally
/// applies [`Lab3PrisonerOutcomeEvent::CreateNoteOnCursor`] via
/// `area_apply::create_lab3_note_on_cursor` (needs `ZoneLoader`).
pub(crate) fn lab3_prisoner_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, Lab3PrisonerPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                Lab3PrisonerPlayerFacts {
                    prisoner_talkstep: player.legacy_lab3_prisoner_talkstep(),
                },
            ))
        })
        .collect()
}

/// Applies each [`Lab3PrisonerOutcomeEvent`] queued by
/// `World::process_lab3_prisoner_actions`.
pub(crate) fn apply_lab3_prisoner_events(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    runtime: &mut ServerRuntime,
    events: Vec<Lab3PrisonerOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            Lab3PrisonerOutcomeEvent::SetPrisonerTalkstep { player_id, value } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_legacy_lab3_prisoner_talkstep(value);
                applied += 1;
            }
            Lab3PrisonerOutcomeEvent::CreateNoteOnCursor { npc_id } => {
                if crate::area_apply::create_lab3_note_on_cursor(world, zone_loader, npc_id) {
                    applied += 1;
                }
            }
        }
    }
    applied
}
