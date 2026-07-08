//! Server-side wiring for area 17's Two-City NPCs (`CDR_TWOSKELLY`/
//! `ugaris_core::world::npc::area17::two_skelly::process_two_skelly_
//! actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established by
//! `area16.rs`: [`two_skelly_player_facts`] snapshots the per-player
//! `twocity_ppd` facts the skelly's dialogue needs before the tick, and
//! [`apply_two_skelly_events`] applies the returned events afterward.

use super::*;
use ugaris_core::world::{TwoSkellyOutcomeEvent, TwoSkellyPlayerFacts};

pub(crate) fn two_skelly_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, TwoSkellyPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                TwoSkellyPlayerFacts {
                    skelly_state: player.twocity_skelly_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`TwoSkellyOutcomeEvent`] queued by
/// `World::process_two_skelly_actions`.
pub(crate) fn apply_two_skelly_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    events: Vec<TwoSkellyOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            TwoSkellyOutcomeEvent::UpdateSkellyState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_twocity_skelly_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, 30)` (`two.c:2835`).
            TwoSkellyOutcomeEvent::QuestOpen { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(30);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, 30)` (`two.c:2897`).
            TwoSkellyOutcomeEvent::QuestDone { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(30, level, level_val) {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;
                }
            }
        }
    }
    applied
}
