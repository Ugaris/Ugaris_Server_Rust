//! Server-side wiring for area 26's Smugglecom NPC (`CDR_SMUGGLECOM`,
//! `ugaris_core::world::npc::area26::smugglecom::process_smugglecom_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established by
//! `area3.rs`: [`apply_smugglecom_events`] applies the returned
//! [`SmuggleComOutcomeEvent`]s, all of which need `PlayerRuntime`
//! (`staffer_ppd.smugglecom_state`/`smugglecom_bits`, `quest_log`), none
//! of which `World` can see.

use std::collections::HashMap;

use super::*;
use ugaris_core::world::npc::area26::smugglecom::{SmuggleComOutcomeEvent, SmuggleComPlayerFacts};

pub(crate) fn smugglecom_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, SmuggleComPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                SmuggleComPlayerFacts {
                    smugglecom_state: player.staffer_smugglecom_state(),
                    smugglecom_bits: player.staffer_smugglecom_bits(),
                    quest36_count: player.quest_log.count(36),
                    quest36_done: player.quest_log.is_done(36),
                    quest37_done: player.quest_log.is_done(37),
                },
            ))
        })
        .collect()
}

/// Applies each [`SmuggleComOutcomeEvent`] queued by `World::
/// process_smugglecom_actions`.
pub(crate) fn apply_smugglecom_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    events: Vec<SmuggleComOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            SmuggleComOutcomeEvent::UpdateSmugglecomState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_smugglecom_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, ...)` (`src/system/questlog.c:204-217`):
            // sets the flag and unconditionally resends the questlog.
            SmuggleComOutcomeEvent::QuestOpen { player_id, quest } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(quest);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, ...)` (`src/system/questlog.c:267-305`):
            // grants the quest table's scaled/tapered exp (quest 36's
            // table entry is `exp: 0` - "exp awarded in driver" - so this
            // only marks it done and resends the questlog, matching C
            // exactly since `give_exp(cn, 0)` is a silent no-op).
            SmuggleComOutcomeEvent::QuestDone { player_id, quest } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(quest, level, level_val)
                {
                    let payload = legacy_questlog_payload(player);
                    if completion.granted_exp != 0 {
                        world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    }
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;
                }
            }
            // C `ppd->smugglecom_bits |= SMUGGLEBIT_*;`.
            SmuggleComOutcomeEvent::SetSmugglecomBit { player_id, bit } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_smugglecom_bits(player.staffer_smugglecom_bits() | bit);
                applied += 1;
            }
            // C `case 3:` (`staffer.c:555-559`): the god-only "reset me"
            // wipe.
            SmuggleComOutcomeEvent::ResetSmugglecom { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_smugglecom_bits(0);
                player.set_staffer_smugglecom_state(0);
                applied += 1;
            }
        }
    }
    applied
}
