//! Server-side wiring for area 31's Warr Mines/Grimroot NPCs
//! (`CDR_DWARFCHIEF`/`CDR_LOSTDWARF`/`CDR_DWARFSHAMAN`/`CDR_DWARFSMITH`,
//! `ugaris_core::world::npc::area31::{dwarfchief,lostdwarf,dwarfshaman,
//! dwarfsmith}::process_*_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established by
//! `area29.rs`: each `apply_*_events` applies the returned outcome events.

use std::collections::HashMap;

use super::*;
use ugaris_core::world::npc::area31::dwarfchief::{
    DwarfRecallScroll, DwarfchiefOutcomeEvent, DwarfchiefPlayerFacts,
};
use ugaris_core::world::npc::area31::dwarfshaman::{
    DwarfshamanOutcomeEvent, DwarfshamanPlayerFacts,
};
use ugaris_core::world::npc::area31::dwarfsmith::{
    DwarfsmithEliteKey, DwarfsmithOutcomeEvent, DwarfsmithPlayerFacts,
};
use ugaris_core::world::npc::area31::lostdwarf::{LostdwarfOutcomeEvent, LostdwarfPlayerFacts};

pub(crate) fn dwarfchief_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, DwarfchiefPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                DwarfchiefPlayerFacts {
                    dwarfchief_state: player.staffer_dwarfchief_state(),
                    quest48_is_done: player.quest_log.is_done(48),
                    quest49_is_done: player.quest_log.is_done(49),
                    quest50_is_done: player.quest_log.is_done(50),
                },
            ))
        })
        .collect()
}

/// Applies each [`DwarfchiefOutcomeEvent`] queued by `World::
/// process_dwarfchief_actions`. [`DwarfchiefOutcomeEvent::
/// GrantRecallScroll`] needs `loader` (`create_item("dwarf_recallNN")`);
/// every other variant only touches `PlayerRuntime`.
pub(crate) fn apply_dwarfchief_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    events: Vec<DwarfchiefOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            DwarfchiefOutcomeEvent::UpdateDwarfchiefState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_dwarfchief_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, 47/48/49/50)`.
            DwarfchiefOutcomeEvent::QuestOpen { player_id, quest } => {
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
            // C `questlog_done(co, 47/48/49/50)`.
            DwarfchiefOutcomeEvent::QuestDone { player_id, quest } => {
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
            // C `create_item("dwarf_recallNN")` + `give_char_item`.
            DwarfchiefOutcomeEvent::GrantRecallScroll { player_id, scroll } => {
                let template = match scroll {
                    DwarfRecallScroll::Recall90 => "dwarf_recall90",
                    DwarfRecallScroll::Recall100 => "dwarf_recall100",
                    DwarfRecallScroll::Recall110 => "dwarf_recall110",
                    DwarfRecallScroll::Recall120 => "dwarf_recall120",
                };
                if let Ok(item) = loader.instantiate_item_template(template, Some(player_id)) {
                    let item_id = item.id;
                    world.add_item(item);
                    if !world.give_char_item(player_id, item_id) {
                        world.destroy_item(item_id);
                    }
                }
                applied += 1;
            }
            // C `case 2:`: reset back to the start of whichever mini quest
            // is in progress.
            DwarfchiefOutcomeEvent::ResetToMiniQuestStart {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_dwarfchief_state(new_state);
                applied += 1;
            }
            // C `case 3:`: the god-only "reset me" state wipe.
            DwarfchiefOutcomeEvent::ResetDwarfchief { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_dwarfchief_state(0);
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn lostdwarf_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, LostdwarfPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                LostdwarfPlayerFacts {
                    dwarfchief_state: player.staffer_dwarfchief_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`LostdwarfOutcomeEvent`] queued by `World::
/// process_lostdwarf_actions`. Every variant only touches `PlayerRuntime`
/// (`World`-side effects - `CF_INVISIBLE`, `log_area`/`log_char` messages,
/// item destruction - already happened directly inside `World` itself, see
/// `world::npc::area31::lostdwarf`'s own module doc comment).
pub(crate) fn apply_lostdwarf_events(
    runtime: &mut ServerRuntime,
    events: Vec<LostdwarfOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            LostdwarfOutcomeEvent::UpdateDwarfchiefState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_dwarfchief_state(new_state);
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn dwarfshaman_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, DwarfshamanPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                DwarfshamanPlayerFacts {
                    dwarfshaman_state: player.staffer_dwarfshaman_state(),
                    dwarfshaman_count: player.staffer_dwarfshaman_count(),
                    quest52_is_done: player.quest_log.is_done(52),
                    quest53_is_done: player.quest_log.is_done(53),
                },
            ))
        })
        .collect()
}

/// Applies each [`DwarfshamanOutcomeEvent`] queued by `World::
/// process_dwarfshaman_actions`. No variant needs `loader` - unlike
/// `dwarfchief`, no item is ever handed out by this driver.
pub(crate) fn apply_dwarfshaman_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    events: Vec<DwarfshamanOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            DwarfshamanOutcomeEvent::UpdateDwarfshamanState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_dwarfshaman_state(new_state);
                applied += 1;
            }
            DwarfshamanOutcomeEvent::UpdateDwarfshamanCount {
                player_id,
                new_count,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_dwarfshaman_count(new_count);
                applied += 1;
            }
            // C `questlog_open(co, 51/52/53)`.
            DwarfshamanOutcomeEvent::QuestOpen { player_id, quest } => {
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
            // C `questlog_done(co, 51/52/53)`.
            DwarfshamanOutcomeEvent::QuestDone { player_id, quest } => {
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
            DwarfshamanOutcomeEvent::ResetToMiniQuestStart {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_dwarfshaman_state(new_state);
                applied += 1;
            }
            DwarfshamanOutcomeEvent::ResetDwarfshaman { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_dwarfshaman_state(0);
                player.set_staffer_dwarfshaman_count(0);
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn dwarfsmith_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, DwarfsmithPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                DwarfsmithPlayerFacts {
                    dwarfsmith_state: player.staffer_dwarfsmith_state(),
                    dwarfsmith_type: player.staffer_dwarfsmith_type(),
                },
            ))
        })
        .collect()
}

/// Applies each [`DwarfsmithOutcomeEvent`] queued by `World::
/// process_dwarfsmith_actions`. [`DwarfsmithOutcomeEvent::GrantEliteKey`]
/// needs `loader` (`create_item("lizard_elite_keyN")`); every other
/// variant only touches `PlayerRuntime`.
pub(crate) fn apply_dwarfsmith_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    events: Vec<DwarfsmithOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            DwarfsmithOutcomeEvent::UpdateDwarfsmithState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_dwarfsmith_state(new_state);
                applied += 1;
            }
            DwarfsmithOutcomeEvent::UpdateDwarfsmithType {
                player_id,
                new_type,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_dwarfsmith_type(new_type);
                applied += 1;
            }
            DwarfsmithOutcomeEvent::GrantEliteKey { player_id, key } => {
                let template = match key {
                    DwarfsmithEliteKey::Key1 => "lizard_elite_key1",
                    DwarfsmithEliteKey::Key2 => "lizard_elite_key2",
                    DwarfsmithEliteKey::Key3 => "lizard_elite_key3",
                };
                if let Ok(item) = loader.instantiate_item_template(template, Some(player_id)) {
                    let item_id = item.id;
                    world.add_item(item);
                    if !world.give_char_item(player_id, item_id) {
                        world.destroy_item(item_id);
                    }
                }
                applied += 1;
            }
            DwarfsmithOutcomeEvent::ResetDwarfsmith { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_dwarfsmith_state(0);
                applied += 1;
            }
        }
    }
    applied
}
