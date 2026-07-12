//! Server-side wiring for area 37's Arkhata NPCs (`CDR_RAMMY`,
//! `ugaris_core::world::npc::area37::rammy::process_rammy_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established by
//! `area29.rs`: [`rammy_player_facts`] snapshots the per-player facts
//! `World` cannot see, [`apply_rammy_events`] applies the returned outcome
//! events. [`RammyOutcomeEvent::GiveFortressKeyAndLetter`] is the only
//! variant needing `loader` (item creation); every other variant only
//! touches `PlayerRuntime`/`World::characters`.

use std::collections::HashMap;

use super::*;
use ugaris_core::world::npc::area37::rammy::{
    qlog_rammy_crown, qlog_rammy_entrance_passes, RammyOutcomeEvent, RammyPlayerFacts,
};

pub(crate) fn rammy_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, RammyPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                RammyPlayerFacts {
                    rammy_state: player.arkhata_rammy_state(),
                    guardbran_state: player.staffer_guardbran_state(),
                    monk_state: player.arkhata_monk_state(),
                    letter_bits: player.arkhata_letter_bits(),
                },
            ))
        })
        .collect()
}

/// Applies each [`RammyOutcomeEvent`] queued by `World::
/// process_rammy_actions`.
pub(crate) async fn apply_rammy_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    events: Vec<RammyOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            RammyOutcomeEvent::UpdateRammyState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_arkhata_rammy_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, 65)` (`arkhata.c:395`).
            RammyOutcomeEvent::QuestOpen65 { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(qlog_rammy_crown());
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, 65)` (`arkhata.c:535`).
            RammyOutcomeEvent::QuestDone65 { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) =
                    player
                        .quest_log
                        .complete_legacy(qlog_rammy_crown(), level, level_val)
                {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;
                }
            }
            // C `questlog_open(co, 71)` (`arkhata.c:426`).
            RammyOutcomeEvent::QuestOpen71 { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(qlog_rammy_entrance_passes());
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, 71)` (`arkhata.c:471`).
            RammyOutcomeEvent::QuestDone71 { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) =
                    player
                        .quest_log
                        .complete_legacy(qlog_rammy_entrance_passes(), level, level_val)
                {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;
                }
            }
            // C `case 16:`'s item hand-out (`arkhata.c:448-459`).
            RammyOutcomeEvent::GiveFortressKeyAndLetter {
                player_id,
                give_key,
                give_letter,
            } => {
                if give_key {
                    if let Ok(item) =
                        loader.instantiate_item_template("key14_13_main", Some(player_id))
                    {
                        let item_id = item.id;
                        world.add_item(item);
                        if !world.give_char_item(player_id, item_id) {
                            world.destroy_item(item_id);
                        }
                    }
                }
                if give_letter {
                    if let Ok(item) = loader.instantiate_item_template("letter1", Some(player_id)) {
                        let item_id = item.id;
                        world.add_item(item);
                        if !world.give_char_item(player_id, item_id) {
                            world.destroy_item(item_id);
                        }
                    }
                }
                applied += 1;
            }
        }
    }
    applied
}
