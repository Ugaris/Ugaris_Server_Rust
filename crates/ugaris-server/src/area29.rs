//! Server-side wiring for area 29's Brannington family NPCs
//! (`CDR_SPIRITBRAN`/`CDR_COUNTBRAN`/`CDR_COUNTESSABRAN`/
//! `CDR_DAUGHTERBRAN`,
//! `ugaris_core::world::npc::area29::{spiritbran,countbran,countessabran,
//! daughterbran}::process_*_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established by
//! `area28.rs`: each `apply_*_events` applies the returned outcome events.
//! Unlike `apply_aristocrat_events`'s money reward (needs `loader` to
//! create an item), [`SpiritBranOutcomeEvent::QuestDone`]'s save reward and
//! [`CountBranOutcomeEvent`]'s manual gold rewards only touch
//! `World::characters`, so only [`apply_countbran_events`] (mausoleum keys)
//! and [`apply_daughterbran_events`] (the lollipop reward) need `loader`.

use std::collections::HashMap;

use super::*;
use ugaris_core::world::npc::area29::countbran::{
    qlog_countbran, CountBranOutcomeEvent, CountBranPlayerFacts,
};
use ugaris_core::world::npc::area29::countessabran::{
    CountessaBranOutcomeEvent, CountessaBranPlayerFacts,
};
use ugaris_core::world::npc::area29::daughterbran::{
    DaughterBranOutcomeEvent, DaughterBranPlayerFacts,
};
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

pub(crate) fn countbran_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, CountBranPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                CountBranPlayerFacts {
                    countbran_state: player.staffer_countbran_state(),
                    countbran_bits: player.staffer_countbran_bits(),
                    quest40_count: player.quest_log.count(qlog_countbran()),
                    quest40_is_done: player.quest_log.is_done(qlog_countbran()),
                },
            ))
        })
        .collect()
}

/// Applies each [`CountBranOutcomeEvent`] queued by `World::
/// process_countbran_actions`. Every jewel reward's exp/gold is applied
/// directly inside `World` itself (see that module's doc comment); only
/// [`CountBranOutcomeEvent::GiveMausoleumKeys`] needs `loader` (C
/// `create_item("warr_mausoleumkeyN")`).
pub(crate) fn apply_countbran_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    events: Vec<CountBranOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            CountBranOutcomeEvent::UpdateCountBranState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_countbran_state(new_state);
                applied += 1;
            }
            // C `if (!questlog_isdone(co, 40)) { questlog_open(co, 40); }`
            // (`brannington.c:652-654`).
            CountBranOutcomeEvent::QuestOpen { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(qlog_countbran());
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `ppd->countbran_bits |= 1/2/4;`.
            CountBranOutcomeEvent::SetCountBranBit { player_id, bit } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_countbran_bits(player.staffer_countbran_bits() | bit);
                applied += 1;
            }
            // C `questlog_done(co, 40)` once all three jewel bits are set
            // (`brannington.c:751`/`782`/`810`) - quest 40's own nominal
            // exp is `0`, so only the bookkeeping half is needed (see
            // `world::npc::area29::countbran`'s module doc comment).
            CountBranOutcomeEvent::MarkQuestDone { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.mark_done(qlog_countbran());
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `countbran_give_keys` (`brannington.c:546-583`).
            CountBranOutcomeEvent::GiveMausoleumKeys { player_id, keys } => {
                for key in keys {
                    let template = match key {
                        1 => "warr_mausoleumkey1",
                        2 => "warr_mausoleumkey2",
                        _ => "warr_mausoleumkey3",
                    };
                    if let Ok(item) = loader.instantiate_item_template(template, Some(player_id)) {
                        let item_id = item.id;
                        world.add_item(item);
                        if !world.give_char_item(player_id, item_id) {
                            world.destroy_item(item_id);
                        }
                    }
                }
                applied += 1;
            }
            // C `case 3:` (`brannington.c:700-706`): the god-only "reset
            // me" wipe, clearing all four Brannington family quest-40
            // states at once.
            CountBranOutcomeEvent::ResetAllBranStates { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_countbran_bits(0);
                player.set_staffer_countbran_state(0);
                player.set_staffer_countessabran_state(0);
                player.set_staffer_daughterbran_state(0);
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn countessabran_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, CountessaBranPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                CountessaBranPlayerFacts {
                    countessabran_state: player.staffer_countessabran_state(),
                    countbran_bits: player.staffer_countbran_bits(),
                },
            ))
        })
        .collect()
}

/// Applies each [`CountessaBranOutcomeEvent`] queued by `World::
/// process_countessabran_actions`. The reward's exp/gold is applied
/// directly inside `World` itself; only `staffer_ppd` writes happen here.
pub(crate) fn apply_countessabran_events(
    runtime: &mut ServerRuntime,
    events: Vec<CountessaBranOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            CountessaBranOutcomeEvent::UpdateCountessaBranState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_countessabran_state(new_state);
                applied += 1;
            }
            // C `ppd->countbran_bits |= 8;` (`brannington.c:1597`).
            CountessaBranOutcomeEvent::SetCountessaBranRewardedBit { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_countbran_bits(player.staffer_countbran_bits() | 8);
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn daughterbran_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, DaughterBranPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                DaughterBranPlayerFacts {
                    daughterbran_state: player.staffer_daughterbran_state(),
                    countbran_bits: player.staffer_countbran_bits(),
                },
            ))
        })
        .collect()
}

/// Applies each [`DaughterBranOutcomeEvent`] queued by `World::
/// process_daughterbran_actions`. The reward's exp is applied directly
/// inside `World` itself; [`DaughterBranOutcomeEvent::GiveLollipop`] needs
/// `loader` (C `create_item("lollipop")`), same precedent as
/// `world::npc::area28::yoatin`'s `WS_Hunter_Belt` reward.
pub(crate) fn apply_daughterbran_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    events: Vec<DaughterBranOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            DaughterBranOutcomeEvent::UpdateDaughterBranState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_daughterbran_state(new_state);
                applied += 1;
            }
            // C `ppd->countbran_bits |= 16;` (`brannington.c:1757`).
            DaughterBranOutcomeEvent::SetDaughterBranRewardedBit { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_countbran_bits(player.staffer_countbran_bits() | 16);
                applied += 1;
            }
            // C `in = create_item("lollipop"); if (in) give_char_item(co,
            // in);` (`brannington.c:1759-1762`).
            DaughterBranOutcomeEvent::GiveLollipop { player_id } => {
                if let Ok(item) = loader.instantiate_item_template("lollipop", Some(player_id)) {
                    let item_id = item.id;
                    world.add_item(item);
                    if !world.give_char_item(player_id, item_id) {
                        world.destroy_item(item_id);
                    }
                }
                applied += 1;
            }
        }
    }
    applied
}
