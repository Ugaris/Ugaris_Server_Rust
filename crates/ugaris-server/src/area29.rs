//! Server-side wiring for area 29's Spirit of Brannington NPC
//! (`CDR_SPIRITBRAN`,
//! `ugaris_core::world::npc::area29::spiritbran::process_spiritbran_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established by
//! `area28.rs`: [`apply_spiritbran_events`] applies the returned outcome
//! events. Unlike `apply_aristocrat_events`'s money reward (needs `loader`
//! to create an item), [`SpiritBranOutcomeEvent::QuestDone`]'s save reward
//! only touches `World::characters` (`Character::saves`), so no `loader`
//! parameter is needed here.

use std::collections::HashMap;

use super::*;
use ugaris_core::world::npc::area29::spiritbran::{
    spiritbran_save_cap, SpiritBranOutcomeEvent, SpiritBranPlayerFacts,
};

pub(crate) fn spiritbran_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, SpiritBranPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                SpiritBranPlayerFacts {
                    spiritbran_state: player.staffer_spiritbran_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`SpiritBranOutcomeEvent`] queued by `World::
/// process_spiritbran_actions`. [`SpiritBranOutcomeEvent::QuestDone`]'s
/// save reward (C `if (tmp == 1 && !(ch[co].flags & CF_HARDCORE) &&
/// ch[co].saves < 10) { ch[co].saves++; log_char(co, LOG_SYSTEM, 0, "You
/// received one save."); }`, `brannington.c:1270-1273`) is applied directly
/// on `World::characters` (`Character::saves`/`Character::flags` live on
/// `World`, unlike `apply_aristocrat_events`'s gold reward which needs
/// `PlayerRuntime`/`loader`).
pub(crate) fn apply_spiritbran_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    events: Vec<SpiritBranOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            SpiritBranOutcomeEvent::UpdateSpiritBranState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_spiritbran_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, 44)` (`src/system/questlog.c:204-217`):
            // sets the flag and unconditionally resends the questlog.
            SpiritBranOutcomeEvent::QuestOpen { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(44);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `tmp = questlog_done(co, 44); ... if (tmp == 1 &&
            // !(ch[co].flags & CF_HARDCORE) && ch[co].saves < 10) {
            // ch[co].saves++; log_char(co, LOG_SYSTEM, 0, "You received one
            // save."); }` (`brannington.c:1268-1273`).
            SpiritBranOutcomeEvent::QuestDone { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(44, level, level_val) {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;

                    if completion.times_done == 1 {
                        if let Some(character) = world.characters.get_mut(&player_id) {
                            if !character.flags.contains(CharacterFlags::HARDCORE)
                                && character.saves < spiritbran_save_cap()
                            {
                                character.saves += 1;
                                world.queue_system_text_bytes(
                                    player_id,
                                    b"You received one save.".to_vec(),
                                );
                            }
                        }
                    }
                }
            }
            // C `case 3:` (`brannington.c:1240-1245`): the god-only "reset
            // me" state wipe.
            SpiritBranOutcomeEvent::ResetSpiritBran { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_spiritbran_state(0);
                applied += 1;
            }
        }
    }
    applied
}
