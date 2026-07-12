//! Server-side wiring for area 36's Caligar NPCs (`CDR_CALIGARGUARD`/
//! `CDR_CALIGARGUARD2`/`CDR_CALIGARGLORI`/`CDR_CALIGARARQUIN`/
//! `CDR_CALIGARSMITH`/`CDR_CALIGARHOMDEN`, `ugaris_core::world::npc::
//! area36::{caligar_guard,caligar_guard2,glori,arquin,smith,homden}::
//! process_*_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established by
//! `area31.rs`: each `apply_*_events` applies the returned outcome events.
//! `caligar_guard`/`caligar_guard2`/`glori`/`arquin`/`homden` need no
//! `ZoneLoader` (no item is ever created by any of them); `smith` does
//! (`create_item("caligar_underground_key")`/`create_item("dictionary")`).

use std::collections::HashMap;

use super::*;
use ugaris_core::item_driver::{IID_CALIGARKEYP1, IID_CALIGARKEYP2, IID_CALIGARKEYP3};
use ugaris_core::world::npc::area36::arquin::{
    CaligarArquinOutcomeEvent, CaligarArquinPlayerFacts,
};
use ugaris_core::world::npc::area36::caligar_guard::{
    CaligarGuardOutcomeEvent, CaligarGuardPlayerFacts,
};
use ugaris_core::world::npc::area36::caligar_guard2::{
    CaligarGuard2OutcomeEvent, CaligarGuard2PlayerFacts,
};
use ugaris_core::world::npc::area36::glori::{CaligarGloriOutcomeEvent, CaligarGloriPlayerFacts};
use ugaris_core::world::npc::area36::homden::{
    CaligarHomdenOutcomeEvent, CaligarHomdenPlayerFacts,
};
use ugaris_core::world::npc::area36::smith::{CaligarSmithOutcomeEvent, CaligarSmithPlayerFacts};

pub(crate) fn caligar_guard_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, CaligarGuardPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                CaligarGuardPlayerFacts {
                    guard_state: player.caligar_guard_state(),
                    guard_last_talk: player.caligar_guard_last_talk(),
                },
            ))
        })
        .collect()
}

/// Applies each [`CaligarGuardOutcomeEvent`] queued by `World::
/// process_caligar_guard_actions`. Every variant only touches
/// `PlayerRuntime`.
pub(crate) fn apply_caligar_guard_events(
    runtime: &mut ServerRuntime,
    events: Vec<CaligarGuardOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            CaligarGuardOutcomeEvent::AdvanceGuardTalk {
                player_id,
                new_state,
                realtime_seconds,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_caligar_guard_talk(new_state, realtime_seconds);
                applied += 1;
            }
            CaligarGuardOutcomeEvent::ResetGuardStateTimeout { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.reset_caligar_guard_state_timeout();
                applied += 1;
            }
            CaligarGuardOutcomeEvent::ResetGuardStateIfThree { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.reset_caligar_guard_if_state_three();
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn caligar_guard2_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, CaligarGuard2PlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                CaligarGuard2PlayerFacts {
                    guard2_last_talk: player.caligar_guard2_last_talk(),
                },
            ))
        })
        .collect()
}

/// Applies each [`CaligarGuard2OutcomeEvent`] queued by `World::
/// process_caligar_guard2_actions`. Only variant touches `PlayerRuntime`.
pub(crate) fn apply_caligar_guard2_events(
    runtime: &mut ServerRuntime,
    events: Vec<CaligarGuard2OutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            CaligarGuard2OutcomeEvent::UpdateGuard2LastTalk {
                player_id,
                realtime_seconds,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_caligar_guard2_last_talk(realtime_seconds);
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn caligar_glori_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, CaligarGloriPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                CaligarGloriPlayerFacts {
                    glori_state: player.caligar_glori_state(),
                    glori_last_talk: player.caligar_glori_last_talk(),
                    watch_flag: player.caligar_watch_flag(),
                },
            ))
        })
        .collect()
}

/// Applies each [`CaligarGloriOutcomeEvent`] queued by `World::
/// process_caligar_glori_actions`. Every variant only touches
/// `PlayerRuntime`.
pub(crate) fn apply_caligar_glori_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    events: Vec<CaligarGloriOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            CaligarGloriOutcomeEvent::AdvanceGloriTalk {
                player_id,
                new_state,
                realtime_seconds,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_caligar_glori_talk(new_state, realtime_seconds);
                applied += 1;
            }
            // C `questlog_open(co, N)`.
            CaligarGloriOutcomeEvent::QuestOpen { player_id, quest } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(quest as usize);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, N)`.
            CaligarGloriOutcomeEvent::QuestDone { player_id, quest } => {
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
                        .complete_legacy(quest as usize, level, level_val)
                {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;
                }
            }
            CaligarGloriOutcomeEvent::ResetGloriMiniBlock { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.reset_caligar_glori_to_mini_block_start();
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn caligar_arquin_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, CaligarArquinPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                CaligarArquinPlayerFacts {
                    arquin_state: player.caligar_arquin_state(),
                    arquin_last_talk: player.caligar_arquin_last_talk(),
                    glori_state: player.caligar_glori_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`CaligarArquinOutcomeEvent`] queued by `World::
/// process_caligar_arquin_actions`. Every variant only touches
/// `PlayerRuntime`.
pub(crate) fn apply_caligar_arquin_events(
    runtime: &mut ServerRuntime,
    events: Vec<CaligarArquinOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            CaligarArquinOutcomeEvent::AdvanceArquinTalk {
                player_id,
                new_state,
                realtime_seconds,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_caligar_arquin_talk(new_state, realtime_seconds);
                applied += 1;
            }
            CaligarArquinOutcomeEvent::ResetArquinMiniBlock { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.reset_caligar_arquin_to_mini_block_start();
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn caligar_smith_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, CaligarSmithPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                CaligarSmithPlayerFacts {
                    smith_state: player.caligar_smith_state(),
                    smith_last_talk: player.caligar_smith_last_talk(),
                    glori_state: player.caligar_glori_state(),
                    arkhata_monk_state: player.arkhata_monk_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`CaligarSmithOutcomeEvent`] queued by `World::
/// process_caligar_smith_actions`. [`CaligarSmithOutcomeEvent::
/// ForgeUndergroundKey`]/[`CaligarSmithOutcomeEvent::PurchaseDictionary`]
/// need `loader` (`create_item(...)`); every other variant only touches
/// `PlayerRuntime`.
pub(crate) fn apply_caligar_smith_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    events: Vec<CaligarSmithOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            CaligarSmithOutcomeEvent::AdvanceSmithTalk {
                player_id,
                new_state,
                realtime_seconds,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_caligar_smith_talk(new_state, realtime_seconds);
                applied += 1;
            }
            CaligarSmithOutcomeEvent::ResetSmithMiniBlock { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.reset_caligar_smith_to_mini_block_start();
                applied += 1;
            }
            // C `case 3:`'s successful path (`caligar.c:1123-1143`).
            CaligarSmithOutcomeEvent::ForgeUndergroundKey {
                smith_id,
                player_id,
            } => {
                let Ok(item) =
                    loader.instantiate_item_template("caligar_underground_key", Some(player_id))
                else {
                    world.npc_quiet_say(smith_id, "Oops. You found bug #1635t. Please report it.");
                    continue;
                };
                let item_id = item.id;
                world.add_item(item);
                if world.give_char_item(player_id, item_id) {
                    world.destroy_items_by_template_id(player_id, IID_CALIGARKEYP1);
                    world.destroy_items_by_template_id(player_id, IID_CALIGARKEYP2);
                    world.destroy_items_by_template_id(player_id, IID_CALIGARKEYP3);
                    if let Some(character) = world.characters.get_mut(&player_id) {
                        character.gold = character.gold.saturating_sub(5000 * 100);
                        character.flags.insert(CharacterFlags::ITEMS);
                    }
                    applied += 1;
                } else {
                    world.destroy_item(item_id);
                    world.npc_quiet_say(smith_id, "No space in inventory, please try again.");
                }
            }
            // C `case 5:`'s successful path (`caligar.c:1159-1170`).
            CaligarSmithOutcomeEvent::PurchaseDictionary {
                smith_id,
                player_id,
            } => {
                let Ok(item) = loader.instantiate_item_template("dictionary", Some(player_id))
                else {
                    world.npc_quiet_say(smith_id, "Oops. You found bug #1636t. Please report it.");
                    continue;
                };
                let item_id = item.id;
                world.add_item(item);
                if world.give_char_item(player_id, item_id) {
                    if let Some(character) = world.characters.get_mut(&player_id) {
                        character.gold = character.gold.saturating_sub(10_000 * 100);
                        character.flags.insert(CharacterFlags::ITEMS);
                    }
                    applied += 1;
                } else {
                    world.destroy_item(item_id);
                    world.npc_quiet_say(smith_id, "No space in inventory, please try again.");
                }
            }
        }
    }
    applied
}

pub(crate) fn caligar_homden_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, CaligarHomdenPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                CaligarHomdenPlayerFacts {
                    homden_state: player.caligar_homden_state(),
                    homden_last_talk: player.caligar_homden_last_talk(),
                },
            ))
        })
        .collect()
}

/// Applies each [`CaligarHomdenOutcomeEvent`] queued by `World::
/// process_caligar_homden_actions`.
pub(crate) fn apply_caligar_homden_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    events: Vec<CaligarHomdenOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            CaligarHomdenOutcomeEvent::AdvanceHomdenTalk {
                player_id,
                new_state,
                realtime_seconds,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_caligar_homden_talk(new_state, realtime_seconds);
                applied += 1;
            }
            // C `questlog_open(co, 59)` (`caligar.c:1252`).
            CaligarHomdenOutcomeEvent::QuestOpen { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(59);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            CaligarHomdenOutcomeEvent::ResetHomdenMiniBlock { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.reset_caligar_homden_to_mini_block_start();
                applied += 1;
            }
            // C `questlog_done(co, 59); ppd->homden_state = 5;`
            // (`caligar.c:1332-1333`) - `homden_last_talk` is deliberately
            // left untouched, matching C.
            CaligarHomdenOutcomeEvent::CompleteRingQuest { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                let last_talk = player.caligar_homden_last_talk();
                player.set_caligar_homden_talk(5, last_talk);
                if let Some(completion) = player.quest_log.complete_legacy(59, level, level_val) {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                }
                applied += 1;
            }
        }
    }
    applied
}
