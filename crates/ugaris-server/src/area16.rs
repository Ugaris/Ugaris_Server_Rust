//! Server-side wiring for area 16's forest quest NPCs (`CDR_FORESTIMP`/
//! `ugaris_core::world::npc::area16::imp::process_forest_imp_actions`,
//! `CDR_FORESTWILLIAM`/`ugaris_core::world::npc::area16::william::
//! process_forest_william_actions`, `CDR_FORESTHERMIT`/`ugaris_core::
//! world::npc::area16::hermit::process_forest_hermit_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established by
//! `area3.rs`: [`forest_imp_player_facts`]/[`forest_william_player_
//! facts`]/[`forest_hermit_player_facts`] snapshot the per-player
//! `area3_ppd` facts each NPC's dialogue needs before the tick, and
//! [`apply_forest_imp_events`]/[`apply_forest_william_events`]/
//! [`apply_forest_hermit_events`] apply the returned events afterward.
//! `apply_forest_william_events` needs `achievement_repository` (William's
//! mantis turn-in credits gold directly via `achievement::give_money`,
//! same precedent as `mine.rs`'s artifact-find rewards), so it - alone
//! among the three - is `async`.

use super::*;
use ugaris_core::world::npc::area16::imp::imp_hardkill_weapon_facts;
use ugaris_core::world::{
    ForestHermitOutcomeEvent, ForestHermitPlayerFacts, ForestImpOutcomeEvent, ForestImpPlayerFacts,
    ForestWilliamOutcomeEvent, ForestWilliamPlayerFacts,
};

pub(crate) fn forest_imp_player_facts(
    world: &World,
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, ForestImpPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            let (has_hardkill_item, hardkill_ritual_progress) =
                imp_hardkill_weapon_facts(world, character_id);
            Some((
                character_id,
                ForestImpPlayerFacts {
                    imp_state: player.area3_imp_state(),
                    hermit_state: player.area3_hermit_state(),
                    quest23_done: player.quest_log.is_done(23),
                    has_hardkill_item,
                    hardkill_ritual_progress,
                },
            ))
        })
        .collect()
}

/// Applies each [`ForestImpOutcomeEvent`] queued by
/// `World::process_forest_imp_actions`.
pub(crate) fn apply_forest_imp_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    events: Vec<ForestImpOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            ForestImpOutcomeEvent::UpdateImpState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area3_imp_state(new_state);
                applied += 1;
            }
            // C `ppd->imp_kills = 0` (`forest.c:288`).
            ForestImpOutcomeEvent::ResetImpKills { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area3_imp_kills(0);
                applied += 1;
            }
            // C `ppd->william_state = 3` (`forest.c:295`).
            ForestImpOutcomeEvent::UpdateWilliamState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area3_william_state(new_state);
                applied += 1;
            }
            // C `questlog_done(co, 22)` (`forest.c:289`): full exp-reward
            // port via `QuestLog::complete_legacy`, applied through
            // `World::give_exp`, same precedent as every other quest-
            // completion exp grant in this codebase.
            ForestImpOutcomeEvent::QuestDoneBearHunt { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(22, level, level_val) {
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

pub(crate) fn forest_william_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, ForestWilliamPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                ForestWilliamPlayerFacts {
                    william_state: player.area3_william_state(),
                    quest22_done: player.quest_log.is_done(22),
                    quest23_done: player.quest_log.is_done(23),
                },
            ))
        })
        .collect()
}

/// Applies each [`ForestWilliamOutcomeEvent`] queued by
/// `World::process_forest_william_actions`. See the module doc comment
/// for why this is `async` (William's mantis reward is a direct gold
/// credit via `achievement::give_money`, unlike every other area-16
/// reward in this file).
pub(crate) async fn apply_forest_william_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    events: Vec<ForestWilliamOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            ForestWilliamOutcomeEvent::UpdateWilliamState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area3_william_state(new_state);
                applied += 1;
            }
            // C `ppd->imp_state = 6` (`forest.c:589`).
            ForestWilliamOutcomeEvent::UpdateImpState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area3_imp_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, quest)` (`src/system/questlog.c:204-
            // 217`): sets the flag and unconditionally resends the
            // questlog.
            ForestWilliamOutcomeEvent::QuestOpen { player_id, quest } => {
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
            // C `tmp = questlog_done(co, 23); ... if (tmp == 1) {
            // give_money(co, 2000, "Imp mantis quest"); }` (`forest.c:591-
            // 594`).
            ForestWilliamOutcomeEvent::QuestDoneMantis { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(23, level, level_val) {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;

                    if completion.times_done == 1 {
                        let mut feedback_bytes = Vec::new();
                        achievement::give_money(
                            world,
                            runtime,
                            achievement_repository,
                            player_id,
                            2000,
                            &mut feedback_bytes,
                        )
                        .await;
                        for (recipient, message) in feedback_bytes {
                            world.queue_system_text_bytes(recipient, message);
                        }
                    }
                }
            }
        }
    }
    applied
}

pub(crate) fn forest_hermit_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, ForestHermitPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                ForestHermitPlayerFacts {
                    hermit_state: player.area3_hermit_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`ForestHermitOutcomeEvent`] queued by
/// `World::process_forest_hermit_actions`.
pub(crate) fn apply_forest_hermit_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    events: Vec<ForestHermitOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            ForestHermitOutcomeEvent::UpdateHermitState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area3_hermit_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, 24)` (`forest.c:700`).
            ForestHermitOutcomeEvent::QuestOpen { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(24);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, 24)` (`forest.c:738`).
            ForestHermitOutcomeEvent::QuestDone { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(24, level, level_val) {
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
