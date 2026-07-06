//! Server-side wiring for area 1's forest hermit NPC (`CDR_CAMHERMIT`,
//! `ugaris_core::world::camhermit::process_camhermit_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established for
//! `world::gatekeeper`'s `GateWelcomePlayerFacts`/`GateWelcomeOutcomeEvent`
//! (see `world::camhermit`'s module doc comment): [`camhermit_player_facts`]
//! snapshots the per-player `area1_ppd`/`quest_log` facts the dialogue
//! needs before the tick, and [`apply_camhermit_events`] applies the
//! returned events afterward, including the `QLOG_HERMIT_QUEST1/2`
//! `questlog_open`/`questlog_done`/`questlog_reopen` calls C's own
//! `camhermit_driver` makes (each of which unconditionally resends the
//! legacy questlog packet, matching `apply_military_mission_kill_check`'s
//! precedent) and the teeth-reward gold's wealth-achievement tracking
//! (`give_money`'s `achievement_add_gold_earned` half - see
//! `World::give_char_item_smart`'s doc comment for why that split exists).

use super::*;
use ugaris_core::quest::QLOG_HERMIT_QUEST2;
use ugaris_core::world::{CamhermitOutcomeEvent, CamhermitPlayerFacts};

pub(crate) fn camhermit_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, CamhermitPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                CamhermitPlayerFacts {
                    state: player.area1_camhermit_state(),
                    seen_timer: player.area1_camhermit_seen_timer(),
                    kills: player.area1_camhermit_kills(),
                    quest2_done_count: player.quest_log.count(QLOG_HERMIT_QUEST2),
                },
            ))
        })
        .collect()
}

/// Applies each [`CamhermitOutcomeEvent`] queued by
/// `World::process_camhermit_actions`. See the module doc comment.
pub(crate) async fn apply_camhermit_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    events: Vec<CamhermitOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            CamhermitOutcomeEvent::UpdateState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_camhermit_state(new_state);
                applied += 1;
            }
            CamhermitOutcomeEvent::UpdateSeenTimer { player_id, value } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_camhermit_seen_timer(value);
                applied += 1;
            }
            // C `questlog_open(co, ...)` (`src/system/questlog.c:204-217`):
            // sets the flag and unconditionally resends the questlog.
            CamhermitOutcomeEvent::QuestOpen { player_id, quest } => {
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
            // full exp-reward port via `QuestLog::complete_legacy`,
            // applied through `World::give_exp` (matching every other
            // quest-completion exp grant in this codebase), plus the
            // unconditional questlog resend.
            CamhermitOutcomeEvent::QuestDone { player_id, quest } => {
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
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;
                }
            }
            // C `questlog_reopen(co, ...)` (`src/system/questlog.c:307-
            // 322`).
            CamhermitOutcomeEvent::QuestReopen { player_id, quest } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.reopen(quest);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            CamhermitOutcomeEvent::GoldEarned { player_id, amount } => {
                award_swap_money_converted_achievement(
                    world,
                    runtime,
                    achievement_repository,
                    player_id,
                    amount,
                )
                .await;
                applied += 1;
            }
        }
    }
    applied
}
