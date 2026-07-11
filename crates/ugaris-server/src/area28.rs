//! Server-side wiring for area 28's Aristocrat and Yoatin NPCs
//! (`CDR_ARISTOCRAT`/`CDR_YOATIN`,
//! `ugaris_core::world::npc::area28::{aristocrat,yoatin}::process_*_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established by
//! `area26.rs`: [`apply_aristocrat_events`]/[`apply_yoatin_events`] apply
//! the returned outcome events, all of which need `PlayerRuntime`
//! (`staffer_ppd.aristocrat_state`/`yoatin_state`, `quest_log`), none of
//! which `World` can see.

use std::collections::HashMap;

use super::*;
use ugaris_core::world::npc::area28::aristocrat::{AristocratOutcomeEvent, AristocratPlayerFacts};
use ugaris_core::world::npc::area28::yoatin::{YoatinOutcomeEvent, YoatinPlayerFacts};

pub(crate) fn aristocrat_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, AristocratPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                AristocratPlayerFacts {
                    aristocrat_state: player.staffer_aristocrat_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`AristocratOutcomeEvent`] queued by `World::
/// process_aristocrat_actions`. [`AristocratOutcomeEvent::QuestDone`] needs
/// `loader` (C `create_money_item(1000 * 100)`), same precedent as
/// `apply_astro2_events`'s money reward.
pub(crate) fn apply_aristocrat_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    events: Vec<AristocratOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            AristocratOutcomeEvent::UpdateAristocratState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_aristocrat_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, 38)` (`src/system/questlog.c:204-217`):
            // sets the flag and unconditionally resends the questlog.
            AristocratOutcomeEvent::QuestOpen { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(38);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `tmp = questlog_done(co, 38); ... if (tmp == 1 && (in =
            // create_money_item(1000 * 100))) { give_char_item(co, in); }`
            // (`brannington_forest.c:380-388`).
            AristocratOutcomeEvent::QuestDone { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(38, level, level_val) {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;

                    if completion.times_done == 1 {
                        let amount: u32 = ARISTOCRAT_REWARD_GOLD;
                        if let Ok(mut item) = loader.instantiate_item_template("money", None) {
                            item.value = amount;
                            item.sprite = create_money_item_sprite(amount);
                            item.description = format!("{:.2}G.", f64::from(amount) / 100.0);
                            let item_id = item.id;
                            world.add_item(item);
                            if !world.give_char_item(player_id, item_id) {
                                world.destroy_item(item_id);
                            }
                        }
                    }
                }
            }
            // C `case 3:` (`brannington_forest.c:355-360`): the god-only
            // "reset me" state wipe.
            AristocratOutcomeEvent::ResetAristocrat { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_aristocrat_state(0);
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn yoatin_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, YoatinPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                YoatinPlayerFacts {
                    yoatin_state: player.staffer_yoatin_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`YoatinOutcomeEvent`] queued by `World::
/// process_yoatin_actions`. [`YoatinOutcomeEvent::QuestDone`] needs
/// `loader` (C `create_item("WS_Hunter_Belt")`), same precedent as
/// `apply_rouven_events`'s vault-key reward.
pub(crate) fn apply_yoatin_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    events: Vec<YoatinOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            YoatinOutcomeEvent::UpdateYoatinState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_yoatin_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, 39)` (`src/system/questlog.c:204-217`):
            // sets the flag and unconditionally resends the questlog.
            YoatinOutcomeEvent::QuestOpen { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(39);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, 39); ... if ((in = create_item("WS_
            // Hunter_Belt"))) { give_char_item(co, in); }`
            // (`brannington_forest.c:589-595`) - unconditional on every
            // completion, unlike `world::npc::area28::aristocrat`'s
            // `times_done == 1`-gated gold.
            YoatinOutcomeEvent::QuestDone { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(39, level, level_val) {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;
                }
                if let Ok(item) =
                    loader.instantiate_item_template("WS_Hunter_Belt", Some(player_id))
                {
                    let item_id = item.id;
                    world.add_item(item);
                    if !world.give_char_item(player_id, item_id) {
                        world.destroy_item(item_id);
                    }
                }
            }
            // C `case 3:` (`brannington_forest.c:562-567`): the god-only
            // "reset me" state wipe.
            YoatinOutcomeEvent::ResetYoatin { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_yoatin_state(0);
                applied += 1;
            }
        }
    }
    applied
}

/// C `create_money_item(1000 * 100)` (`brannington_forest.c:384`).
const ARISTOCRAT_REWARD_GOLD: u32 = 1000 * 100;
