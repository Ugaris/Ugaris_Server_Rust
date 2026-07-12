//! Server-side wiring for area 37's Arkhata NPCs (`CDR_RAMMY`,
//! `ugaris_core::world::npc::area37::rammy::process_rammy_actions`;
//! `CDR_JAZ`, `ugaris_core::world::npc::area37::jaz::process_jaz_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established by
//! `area29.rs`: [`rammy_player_facts`]/[`jaz_player_facts`] snapshot the
//! per-player facts `World` cannot see, [`apply_rammy_events`]/
//! [`apply_jaz_events`] apply the returned outcome events.
//! [`RammyOutcomeEvent::GiveFortressKeyAndLetter`] is the only variant
//! (across both NPCs) needing `loader` (item creation); every other
//! variant only touches `PlayerRuntime`/`World::characters`.

use std::collections::HashMap;

use super::*;
use ugaris_core::character_driver::{CharacterDriverState, FightDriverData, CDR_GLADIATOR};
use ugaris_core::world::npc::area37::arkhatamonk::{
    qlog_monk_bookeater, qlog_monk_dictionary, qlog_monk_keyparts, ArkhatamonkOutcomeEvent,
    ArkhatamonkPlayerFacts,
};
use ugaris_core::world::npc::area37::fiona::{
    qlog_fiona_ring, FionaOutcomeEvent, FionaPlayerFacts,
};
use ugaris_core::world::npc::area37::gladiator::GladiatorDriverData;
use ugaris_core::world::npc::area37::jaz::{qlog_jaz_bracelet, JazOutcomeEvent, JazPlayerFacts};
use ugaris_core::world::npc::area37::ramin::{
    qlog_ramin_shopkeeper, RaminOutcomeEvent, RaminPlayerFacts,
};
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

pub(crate) fn jaz_player_facts(runtime: &ServerRuntime) -> HashMap<CharacterId, JazPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                JazPlayerFacts {
                    jaz_state: player.arkhata_jaz_state(),
                    rammy_state: player.arkhata_rammy_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`JazOutcomeEvent`] queued by `World::process_jaz_actions`.
pub(crate) async fn apply_jaz_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    events: Vec<JazOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            JazOutcomeEvent::UpdateJazState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_arkhata_jaz_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, 66)` (`arkhata.c:635`).
            JazOutcomeEvent::QuestOpen66 { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(qlog_jaz_bracelet());
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, 66)` (`arkhata.c:720`).
            JazOutcomeEvent::QuestDone66 { player_id } => {
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
                        .complete_legacy(qlog_jaz_bracelet(), level, level_val)
                {
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

pub(crate) fn fiona_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, FionaPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                FionaPlayerFacts {
                    fiona_state: player.arkhata_fiona_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`FionaOutcomeEvent`] queued by `World::
/// process_fiona_actions`.
pub(crate) async fn apply_fiona_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    events: Vec<FionaOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            FionaOutcomeEvent::UpdateFionaState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_arkhata_fiona_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, 67)` (`arkhata.c:891`).
            FionaOutcomeEvent::QuestOpen67 { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(qlog_fiona_ring());
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, 67)` (`arkhata.c:1042`).
            FionaOutcomeEvent::QuestDone67 { player_id } => {
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
                        .complete_legacy(qlog_fiona_ring(), level, level_val)
                {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;
                }
            }
            // C `fight_student(cn, co, ppd->fiona_state - 6)`
            // (`arkhata.c:1014`).
            FionaOutcomeEvent::FightStudent {
                fiona_id,
                player_id,
                nr,
            } => {
                spawn_gladiator_student(world, runtime, loader, fiona_id, player_id, nr);
                applied += 1;
            }
        }
    }
    applied
}

/// C `fight_student(cc, cn, nr)` (`arkhata.c:756-804`): `cc` (the speaking
/// NPC) is `fiona_id` here, `cn` (the player being teleported/enrolled) is
/// `player_id`. Needs `ZoneLoader` to instantiate `"Gladiator_<nr>"` - see
/// `world::npc::area37::fiona`'s module doc comment.
fn spawn_gladiator_student(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    fiona_id: CharacterId,
    player_id: CharacterId,
    nr: i32,
) {
    // C `for (x=9;x<=24;x++) for(y=238;y<=252;y++) if
    // ((co=map[x+y*MAXMAP].ch)) { say(cc, "The arena is busy. Please try
    // again later."); return; }` (`arkhata.c:759-765`).
    if world.arkhata_arena_is_busy() {
        world.npc_say(fiona_id, "The arena is busy. Please try again later.");
        return;
    }

    let template_key = format!("Gladiator_{nr}");
    let character_id = runtime.allocate_character_id();
    let Ok((mut gladiator, inventory_items)) =
        loader.instantiate_character_template(&template_key, character_id)
    else {
        // C `if (!co) { say(cc, "Oops. Bug #5317a"); return; }`
        // (`arkhata.c:772-775`).
        world.npc_say(fiona_id, "Oops. Bug #5317a");
        return;
    };

    // C `ch[co].driver = 135;` (`arkhata.c:788`) - the zone template's own
    // `driver=136` (`CDR_NOP`, the background "Student") is overridden to
    // the combat driver here.
    gladiator.driver = CDR_GLADIATOR;
    gladiator.driver_state = Some(CharacterDriverState::Gladiator(GladiatorDriverData {
        last_talk: world.tick.0,
    }));
    gladiator.dir = Direction::RightDown as u8;
    gladiator.rest_x = 14;
    gladiator.rest_y = 244;
    // C never calls `fight_driver_set_dist` for `CDR_GLADIATOR` anywhere -
    // its `struct fight_driver_data` is entirely `set_data`'s own
    // zero-initialize-on-first-touch auto-vivification (`fight_driver_add_
    // enemy`'s `dat = set_data(cn, DRD_FIGHTDRIVER, ...)`), which the
    // driver-independent `Character::fight_driver` field would only
    // replicate implicitly via `add_simple_baddy_enemy_unchecked`'s own
    // `get_or_insert_with` - except `World::simple_baddy_enemy_within_
    // start_limits` (needed by the `NT_CHAR` auto-aggro path) treats a
    // still-`None` `fight_driver` as "reject" rather than "no limits set",
    // so it must be seeded eagerly here (same precedent as `spawns::
    // spawn_warp_trial_fighter`'s explicit `fight_driver_set_dist`
    // reproduction, minus the nonzero start/stop-dist values C never sets
    // for this driver).
    gladiator.fight_driver = Some(FightDriverData::default());
    // C `ch[co].flags &= ~(CF_RESPAWN|CF_NOATTACK|CF_IMMORTAL);`
    // (`arkhata.c:789`).
    gladiator
        .flags
        .remove(CharacterFlags::RESPAWN | CharacterFlags::NOATTACK | CharacterFlags::IMMORTAL);

    if !world.spawn_character(gladiator, 14, 244) {
        // C `if (!drop_char(co, 14, 244, 0)) { destroy_char(co); say(cc,
        // "Oops. Bug #5317b"); return; }` (`arkhata.c:793-797`).
        world.npc_say(fiona_id, "Oops. Bug #5317b");
        return;
    }
    for item in inventory_items {
        world.items.insert(item.id, item);
    }
    world.update_character(character_id);
    if let Some(character) = world.characters.get_mut(&character_id) {
        character.hp = i32::from(character.values[0][CharacterValue::Hp as usize]) * POWERSCALE;
        character.endurance =
            i32::from(character.values[0][CharacterValue::Endurance as usize]) * POWERSCALE;
        character.mana = i32::from(character.values[0][CharacterValue::Mana as usize]) * POWERSCALE;
        character.lifeshield =
            i32::from(character.values[0][CharacterValue::MagicShield as usize]) * POWERSCALE;
    }

    // C `teleport_char_driver(cn, 16, 244);` (`arkhata.c:803`).
    world.teleport_char_driver(player_id, 16, 244);
}

pub(crate) fn ramin_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, RaminPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                RaminPlayerFacts {
                    ramin_state: player.arkhata_ramin_state(),
                    fiona_state: player.arkhata_fiona_state(),
                    monk_state: player.arkhata_monk_state(),
                    rammy_state: player.arkhata_rammy_state(),
                    letter_bits: player.arkhata_letter_bits(),
                },
            ))
        })
        .collect()
}

/// Applies each [`RaminOutcomeEvent`] queued by `World::
/// process_ramin_actions`. Unlike `apply_rammy_events`/`apply_jaz_events`,
/// no variant here needs `World` - `ramin_driver` never itself completes
/// quest 68 (`world_events::death_hooks::
/// apply_arkhataskelly_death_from_hurt_event` does, needing `give_exp`) or
/// creates an item.
pub(crate) async fn apply_ramin_events(
    runtime: &mut ServerRuntime,
    events: Vec<RaminOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            RaminOutcomeEvent::UpdateRaminState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_arkhata_ramin_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, 68)` (`arkhata.c:1406`).
            RaminOutcomeEvent::QuestOpen68 { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(qlog_ramin_shopkeeper());
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `ppd->letter_bits |= 2` (`arkhata.c:1552`).
            RaminOutcomeEvent::GiveLetter2Bit { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                let new_bits = player.arkhata_letter_bits() | 2;
                player.set_arkhata_letter_bits(new_bits);
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn arkhatamonk_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, ArkhatamonkPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                ArkhatamonkPlayerFacts {
                    monk_state: player.arkhata_monk_state(),
                    monk_bits: player.arkhata_monk_bits(),
                    ramin_state: player.arkhata_ramin_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`ArkhatamonkOutcomeEvent`] queued by `World::
/// process_arkhatamonk_actions`.
pub(crate) async fn apply_arkhatamonk_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    events: Vec<ArkhatamonkOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            ArkhatamonkOutcomeEvent::UpdateMonkState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_arkhata_monk_state(new_state);
                applied += 1;
            }
            ArkhatamonkOutcomeEvent::UpdateMonkBits {
                player_id,
                new_bits,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_arkhata_monk_bits(new_bits);
                applied += 1;
            }
            // C `questlog_open(co, 69)` (`arkhata.c:1784`).
            ArkhatamonkOutcomeEvent::QuestOpen69 { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(qlog_monk_keyparts());
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, 69)` (`arkhata.c:1998,2011,2023`).
            ArkhatamonkOutcomeEvent::QuestDone69 { player_id } => {
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
                        .complete_legacy(qlog_monk_keyparts(), level, level_val)
                {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;
                }
            }
            // C `questlog_open(co, 70)` (`arkhata.c:1835`). Completion
            // (`questlog_done(co, 70)`) lives in `world_events::
            // death_hooks::apply_arkhata_bookeater_death_from_hurt_event`,
            // not here - `qlog_monk_bookeater` is only referenced here to
            // document that connection.
            ArkhatamonkOutcomeEvent::QuestOpen70 { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(qlog_monk_bookeater());
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_open(co, 78)` (`arkhata.c:1879`). Completion
            // (`questlog_done(co, 78)`) lives in the still-unported
            // `kidnappee_driver` (`arkhata.c:4269`).
            ArkhatamonkOutcomeEvent::QuestOpen78 { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(qlog_monk_dictionary());
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
        }
    }
    applied
}
