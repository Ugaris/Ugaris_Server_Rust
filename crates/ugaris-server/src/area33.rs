//! Server-side wiring for area 33's Gorwin NPC (`CDR_TUNNELER_GORWIN`,
//! `ugaris_core::world::npc::area33::gorwin::process_gorwin_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established by
//! `area29.rs`. Both [`GorwinOutcomeEvent`] variants only write scalar
//! `PlayerRuntime` fields (already-ported `crate::player::tunnel`
//! accessors), so unlike `area29.rs`'s `apply_countbran_events`/
//! `apply_daughterbran_events`, [`apply_gorwin_events`] needs no
//! `ZoneLoader`.

use std::collections::HashMap;

use super::*;
use ugaris_core::player::MAX_TUNNEL_LEVEL;
use ugaris_core::world::npc::area33::gorwin::{GorwinOutcomeEvent, GorwinPlayerFacts};

pub(crate) fn gorwin_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, GorwinPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            let tunnel_used = (0..=MAX_TUNNEL_LEVEL)
                .map(|level| player.tunnel_used(level))
                .collect();
            Some((
                character_id,
                GorwinPlayerFacts {
                    gorwin_tunnel_level: player.gorwin_tunnel_level(),
                    tunnel_clevel: player.tunnel_clevel(),
                    tunnel_used,
                },
            ))
        })
        .collect()
}

/// Applies each [`GorwinOutcomeEvent`] queued by `World::
/// process_gorwin_actions`.
pub(crate) fn apply_gorwin_events(
    runtime: &mut ServerRuntime,
    events: Vec<GorwinOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            GorwinOutcomeEvent::SetGorwinTunnelLevel { player_id, level } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_gorwin_tunnel_level(level);
                applied += 1;
            }
            GorwinOutcomeEvent::SetTunnelLevelBoth { player_id, level } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_gorwin_tunnel_level(level);
                player.set_tunnel_clevel(level);
                applied += 1;
            }
        }
    }
    applied
}
