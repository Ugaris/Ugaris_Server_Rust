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
use ugaris_core::world::npc::area29::brennethbran::{
    BrennethBranOutcomeEvent, BrennethBranPlayerFacts,
};
use ugaris_core::world::npc::area29::broklin::{
    BroklinOutcomeEvent, BroklinPlayerFacts, BroklinTradeReward,
};
use ugaris_core::world::npc::area29::countbran::{
    qlog_countbran, CountBranOutcomeEvent, CountBranPlayerFacts,
};
use ugaris_core::world::npc::area29::countessabran::{
    CountessaBranOutcomeEvent, CountessaBranPlayerFacts,
};
use ugaris_core::world::npc::area29::daughterbran::{
    DaughterBranOutcomeEvent, DaughterBranPlayerFacts,
};
use ugaris_core::world::npc::area29::forestbran::{ForestBranOutcomeEvent, ForestBranPlayerFacts};
use ugaris_core::world::npc::area29::grinnich::{GrinnichOutcomeEvent, GrinnichPlayerFacts};
use ugaris_core::world::npc::area29::guardbran::{
    qlog_guardbran, GuardBranOutcomeEvent, GuardBranPlayerFacts,
};
use ugaris_core::world::npc::area29::shanra::{ShanraOutcomeEvent, ShanraPlayerFacts};
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

pub(crate) fn guardbran_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, GuardBranPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                GuardBranPlayerFacts {
                    guardbran_state: player.staffer_guardbran_state(),
                    countbran_state: player.staffer_countbran_state(),
                    countbran_bits: player.staffer_countbran_bits(),
                    rammy_state: player.arkhata_rammy_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`GuardBranOutcomeEvent`] queued by `World::
/// process_guardbran_actions`. [`GuardBranOutcomeEvent::QuestDone`] needs
/// quest 64's full `complete_legacy` exp path (its own nominal exp is
/// `60000`, unlike `apply_spiritbran_events`'s `0`-exp save reward) plus
/// `award_great_explorer_achievement`, same precedent as
/// `apply_lydia_events`'s `QuestDone` achievement call.
pub(crate) async fn apply_guardbran_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    events: Vec<GuardBranOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            GuardBranOutcomeEvent::UpdateGuardBranState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_guardbran_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, 64)` (`src/system/questlog.c:204-217`):
            // sets the flag and unconditionally resends the questlog.
            GuardBranOutcomeEvent::QuestOpen { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(qlog_guardbran());
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, 64)` plus `achievement_award(co,
            // ACHIEVEMENT_GREAT_EXPLORER, 1)` (`brannington.c:1940-1942`).
            GuardBranOutcomeEvent::QuestDone { player_id } => {
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
                        .complete_legacy(qlog_guardbran(), level, level_val)
                {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    award_great_explorer_achievement(
                        world,
                        runtime,
                        achievement_repository,
                        player_id,
                    )
                    .await;
                    applied += 1;
                }
            }
        }
    }
    applied
}

pub(crate) fn brennethbran_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, BrennethBranPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                BrennethBranPlayerFacts {
                    brennethbran_state: player.staffer_brennethbran_state(),
                    quest42_is_done: player.quest_log.is_done(42),
                    quest43_is_done: player.quest_log.is_done(43),
                },
            ))
        })
        .collect()
}

/// Applies each [`BrennethBranOutcomeEvent`] queued by `World::
/// process_brennethbran_actions`. Unlike `apply_spiritbran_events`'s
/// `QuestDone` (which additionally grants a save on first completion),
/// C's three `questlog_done(co, 41/42/43)` call sites here have no extra
/// reward logic at all - just the standard `complete_legacy` exp/
/// questlog-resend bookkeeping.
pub(crate) fn apply_brennethbran_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    events: Vec<BrennethBranOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            BrennethBranOutcomeEvent::UpdateBrennethBranState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_brennethbran_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, 41/42/43)`.
            BrennethBranOutcomeEvent::QuestOpen { player_id, quest } => {
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
            // C `questlog_done(co, 41/42/43)`.
            BrennethBranOutcomeEvent::QuestDone { player_id, quest } => {
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
            // C `case 2:` (`brannington.c:1022-1039`): reset back to the
            // start of whichever mini quest is in progress.
            BrennethBranOutcomeEvent::ResetToMiniQuestStart {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_brennethbran_state(new_state);
                applied += 1;
            }
            // C `case 3:` (`brannington.c:1040-1045`): the god-only "reset
            // me" state wipe.
            BrennethBranOutcomeEvent::ResetBrennethBran { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_brennethbran_state(0);
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

pub(crate) fn forestbran_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, ForestBranPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                ForestBranPlayerFacts {
                    forestbran_state: player.staffer_forestbran_state(),
                    forestbran_done: player.forestbran_done(),
                },
            ))
        })
        .collect()
}

/// Applies each [`ForestBranOutcomeEvent`] queued by `World::
/// process_forestbran_actions`. This NPC has no exp/gold/item reward at
/// all, so both variants only touch `PlayerRuntime`'s `staffer_ppd`.
pub(crate) fn apply_forestbran_events(
    runtime: &mut ServerRuntime,
    events: Vec<ForestBranOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            ForestBranOutcomeEvent::UpdateForestBranState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_forestbran_state(new_state);
                applied += 1;
            }
            // C `case 3:` (`brannington.c:1421-1426`): the god-only "reset
            // me" wipe, clearing *both* `forestbran_state` and
            // `forestbran_done`.
            ForestBranOutcomeEvent::ResetForestBran { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_forestbran_state(0);
                player.clear_forestbran_done();
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn broklin_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, BroklinPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                BroklinPlayerFacts {
                    broklin_state: player.staffer_broklin_state(),
                    quest46_is_done: player.quest_log.is_done(46),
                },
            ))
        })
        .collect()
}

/// Applies each [`BroklinOutcomeEvent`] queued by `World::
/// process_broklin_actions`. [`BroklinOutcomeEvent::QuestDonePickaxe`] needs
/// both `loader` (C `create_item("gold_2000")`) and `world` (to speak the
/// times_done-dependent reply through `broklin_id`), same precedent as
/// `apply_aristocrat_events`'s money reward plus `world::npc::area29::
/// broklin`'s own module doc comment for why the reply text can't be
/// decided inside `World`. [`BroklinOutcomeEvent::GrantSewerKey`]/
/// [`BroklinOutcomeEvent::GrantTradeReward`] need `loader` only (`World`
/// already confirmed the precondition and, for trades, already
/// decremented/destroyed the paid-in stack).
pub(crate) fn apply_broklin_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    events: Vec<BroklinOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            BroklinOutcomeEvent::UpdateBroklinState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_broklin_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, 45)` (`brannington.c:2180`).
            BroklinOutcomeEvent::QuestOpen45 { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(45);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_open(co, 46)` (`brannington.c:2210`).
            BroklinOutcomeEvent::QuestOpen46 { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(46);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `tmp = questlog_done(co, 45); ... if (tmp == 1 && (in =
            // create_item("gold_2000"))) { give_char_item(co, in); }`
            // (`brannington.c:2346-2361`).
            BroklinOutcomeEvent::QuestDonePickaxe {
                player_id,
                broklin_id,
            } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(45, level, level_val) {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;

                    if completion.times_done == 1 {
                        world.npc_quiet_say(
                            broklin_id,
                            "Thank you! Take these 2,000 gu - I am sure it will be useful to you.",
                        );
                        if let Ok(item) =
                            loader.instantiate_item_template("gold_2000", Some(player_id))
                        {
                            let item_id = item.id;
                            world.add_item(item);
                            if !world.give_char_item(player_id, item_id) {
                                world.destroy_item(item_id);
                            }
                        }
                    } else {
                        world.npc_quiet_say(broklin_id, "Thank you!");
                    }
                }
            }
            // C `case 2:` (`brannington.c:2302-2315`): reset back to the
            // start of whichever dialogue span is in progress.
            BroklinOutcomeEvent::ResetToMiniQuestStart {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_broklin_state(new_state);
                applied += 1;
            }
            // C `case 3:` (`brannington.c:2316-2321`): the god-only "reset
            // me" state wipe.
            BroklinOutcomeEvent::ResetBroklin { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_broklin_state(0);
                applied += 1;
            }
            // C `case 8:`'s key-giveaway branch (`brannington.c:2225-
            // 2232`).
            BroklinOutcomeEvent::GrantSewerKey { player_id } => {
                if let Ok(item) =
                    loader.instantiate_item_template("WS_Robber_Key_Area2", Some(player_id))
                {
                    let item_id = item.id;
                    world.add_item(item);
                    if !world.give_char_item(player_id, item_id) {
                        world.destroy_item(item_id);
                    }
                }
                applied += 1;
            }
            // C `broklin_trade_gold`/`broklin_trade_silver`'s
            // `create_item(...)` call (`brannington.c:2061`/`2105`).
            BroklinOutcomeEvent::GrantTradeReward { player_id, reward } => {
                let template = match reward {
                    BroklinTradeReward::Silver4000 => "silver_4000",
                    BroklinTradeReward::Gold1000 => "gold_1000",
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
        }
    }
    applied
}

pub(crate) fn grinnich_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, GrinnichPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                GrinnichPlayerFacts {
                    grinnich_state: player.staffer_grinnich_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`GrinnichOutcomeEvent`] queued by `World::
/// process_grinnich_actions`. Unlike `apply_spiritbran_events`, neither
/// event here touches `World::characters` (no quest, no exp, no item
/// reward) - both are plain `PlayerRuntime` state writes.
pub(crate) fn apply_grinnich_events(
    runtime: &mut ServerRuntime,
    events: Vec<GrinnichOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            GrinnichOutcomeEvent::UpdateGrinnichState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_grinnich_state(new_state);
                applied += 1;
            }
            // C `case 3:` (`brannington.c:2513-2518`): the god-only "reset
            // me" state wipe.
            GrinnichOutcomeEvent::ResetGrinnich { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_grinnich_state(0);
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn shanra_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, ShanraPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                ShanraPlayerFacts {
                    shanra_state: player.staffer_shanra_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`ShanraOutcomeEvent`] queued by `World::
/// process_shanra_actions`. Like `apply_grinnich_events`, both variants
/// here are plain `PlayerRuntime` state writes - the teleports themselves
/// already happened directly on `World` inside `process_shanra_actions`
/// (see that module's doc comment).
pub(crate) fn apply_shanra_events(
    runtime: &mut ServerRuntime,
    events: Vec<ShanraOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            ShanraOutcomeEvent::UpdateShanraState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_shanra_state(new_state);
                applied += 1;
            }
            // C `case 3:` (`brannington.c:2679-2683`): the god-only "reset
            // me" state wipe.
            ShanraOutcomeEvent::ResetShanra { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_shanra_state(0);
                applied += 1;
            }
        }
    }
    applied
}
