//! Server-side wiring for area 17's Two-City NPCs (`CDR_TWOSKELLY`/
//! `ugaris_core::world::npc::area17::two_skelly::process_two_skelly_
//! actions`, `CDR_TWOALCHEMIST`/`...::alchemist::process_two_alchemist_
//! actions`, `CDR_TWOSANWYN`/`...::sanwyn::process_two_sanwyn_actions`,
//! plus `CDR_TWOBARKEEPER`/`...::barkeeper::process_two_barkeeper_
//! actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established by
//! `area16.rs`: [`two_skelly_player_facts`]/[`two_alchemist_player_facts`]/
//! [`two_sanwyn_player_facts`]/[`two_barkeeper_player_facts`] snapshot the
//! per-player `twocity_ppd` facts each NPC's dialogue needs before the
//! tick, and [`apply_two_skelly_events`]/[`apply_two_alchemist_events`]/
//! [`apply_two_sanwyn_events`]/[`apply_two_barkeeper_events`] apply the
//! returned events afterward.

use super::*;
use ugaris_core::world::{
    TwoAlchemistOutcomeEvent, TwoAlchemistPlayerFacts, TwoBarkeeperOutcomeEvent,
    TwoBarkeeperPlayerFacts, TwoGuardOutcomeEvent, TwoGuardPlayerFacts, TwoSanwynOutcomeEvent,
    TwoSanwynPlayerFacts, TwoServantOutcomeEvent, TwoServantPlayerFacts, TwoSkellyOutcomeEvent,
    TwoSkellyPlayerFacts, TwoThiefGuardOutcomeEvent, TwoThiefGuardPlayerFacts, CS_GUEST, LS_CLEAN,
};

pub(crate) fn two_skelly_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, TwoSkellyPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                TwoSkellyPlayerFacts {
                    skelly_state: player.twocity_skelly_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`TwoSkellyOutcomeEvent`] queued by
/// `World::process_two_skelly_actions`.
pub(crate) fn apply_two_skelly_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    events: Vec<TwoSkellyOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            TwoSkellyOutcomeEvent::UpdateSkellyState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_twocity_skelly_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, 30)` (`two.c:2835`).
            TwoSkellyOutcomeEvent::QuestOpen { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(30);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, 30)` (`two.c:2897`).
            TwoSkellyOutcomeEvent::QuestDone { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(30, level, level_val) {
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

pub(crate) fn two_alchemist_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, TwoAlchemistPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                TwoAlchemistPlayerFacts {
                    alchemist_state: player.twocity_alchemist_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`TwoAlchemistOutcomeEvent`] queued by
/// `World::process_two_alchemist_actions`.
pub(crate) fn apply_two_alchemist_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    zone_loader: &mut ZoneLoader,
    events: Vec<TwoAlchemistOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            TwoAlchemistOutcomeEvent::UpdateAlchemistState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_twocity_alchemist_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, 31)` (`two.c:3012`).
            TwoAlchemistOutcomeEvent::QuestOpen { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(31);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, 31)` plus its reward branch
            // (`two.c:3092-3117`): the potion reward only fires on the
            // 1st/3rd/7th/10th completion (`tmp == 1 || 3 || 7 || 10`),
            // and the potion template depends on the giver's level.
            TwoAlchemistOutcomeEvent::QuestDone {
                player_id,
                alchemist_id,
            } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let giver_name = world
                    .characters
                    .get(&player_id)
                    .map(|c| c.name.clone())
                    .unwrap_or_default();
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(31, level, level_val) {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    // C `if (tmp == 1 || tmp == 3 || tmp == 7 || tmp ==
                    // 10)` (`two.c:3099`).
                    if matches!(completion.times_done, 1 | 3 | 7 | 10) {
                        world.npc_say(
                            alchemist_id,
                            &format!(
                                "Too little sulphur this time. I will... Oh, the poison! Very well, {giver_name}. Here, take this potion for thy trouble."
                            ),
                        );
                        // C `if (ch[co].level < 30) in = create_item(
                        // "combo_potion3"); else in = create_item(
                        // "security_potion");` (`two.c:3104-3108`).
                        let template = if level < 30 {
                            "combo_potion3"
                        } else {
                            "security_potion"
                        };
                        if let Ok(potion) =
                            zone_loader.instantiate_item_template(template, Some(player_id))
                        {
                            let potion_id = potion.id;
                            world.add_item(potion);
                            if !world.give_char_item(player_id, potion_id) {
                                world.destroy_item(potion_id);
                            }
                        }
                    } else {
                        world.npc_say(
                            alchemist_id,
                            &format!(
                                "Too little sulphur this time. I will... Oh, the poison! Very well, {giver_name}, I thank thee."
                            ),
                        );
                    }
                    applied += 1;
                }
            }
        }
    }
    applied
}

pub(crate) fn two_sanwyn_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, TwoSanwynPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                TwoSanwynPlayerFacts {
                    sanwyn_state: player.twocity_sanwyn_state(),
                    sanwyn_bits: player.twocity_sanwyn_bits(),
                },
            ))
        })
        .collect()
}

/// Applies each [`TwoSanwynOutcomeEvent`] queued by
/// `World::process_two_sanwyn_actions`. Unlike `TwoAlchemistOutcomeEvent`,
/// the military-points reward is applied directly inside `World` (it only
/// needs `Character::level`), so `QuestDone` here is just the plain
/// `questlog_done(co, 29)` bookkeeping (quest 29's `exp` is `0` -
/// "awarded in driver" per `questlog.c`'s own table comment).
pub(crate) fn apply_two_sanwyn_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    events: Vec<TwoSanwynOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            TwoSanwynOutcomeEvent::UpdateSanwynState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_twocity_sanwyn_state(new_state);
                applied += 1;
            }
            TwoSanwynOutcomeEvent::UpdateSanwynBits {
                player_id,
                new_bits,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_twocity_sanwyn_bits(new_bits);
                applied += 1;
            }
            // C `questlog_open(co, 29)` (`two.c:2318`).
            TwoSanwynOutcomeEvent::QuestOpen { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(29);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, 29)` (`two.c:2401`).
            TwoSanwynOutcomeEvent::QuestDone { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(29, level, level_val) {
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
        }
    }
    applied
}

pub(crate) fn two_barkeeper_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, TwoBarkeeperPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                TwoBarkeeperPlayerFacts {
                    barkeeper_state: player.twocity_barkeeper_state(),
                    citizen_status: player.twocity_citizen_status(),
                    legal_status: player.twocity_legal_status(),
                    legal_fine: player.twocity_legal_fine(),
                },
            ))
        })
        .collect()
}

/// Applies each [`TwoBarkeeperOutcomeEvent`] queued by
/// `World::process_two_barkeeper_actions`. Unlike its siblings, this NPC
/// has no quest of its own - every event is a plain `twocity_ppd`
/// writeback.
pub(crate) fn apply_two_barkeeper_events(
    runtime: &mut ServerRuntime,
    events: Vec<TwoBarkeeperOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            TwoBarkeeperOutcomeEvent::UpdateBarkeeperState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_twocity_barkeeper_state(new_state);
                applied += 1;
            }
            // C `ppd->barkeeper_last = realtime;` (`two.c:858`).
            TwoBarkeeperOutcomeEvent::UpdateBarkeeperLast {
                player_id,
                realtime,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_twocity_barkeeper_last(realtime);
                applied += 1;
            }
            // C `ppd->citizen_status = CS_GUEST; ppd->legal_status =
            // LS_CLEAN; ppd->legal_fine = 0;` (`two.c:912-914`).
            TwoBarkeeperOutcomeEvent::BuyPass { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_twocity_citizen_status(CS_GUEST);
                player.set_twocity_legal_status(LS_CLEAN);
                player.set_twocity_legal_fine(0);
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn two_guard_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, TwoGuardPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                TwoGuardPlayerFacts {
                    legal_status: player.twocity_legal_status(),
                    legal_fine: player.twocity_legal_fine(),
                    citizen_status: player.twocity_citizen_status(),
                    current_guard: player.twocity_current_guard(),
                    current_guard_time: player.twocity_current_guard_time(),
                    last_attack: player.twocity_last_attack(),
                    guard_intro: player.twocity_guard_intro(),
                    bank_gold: player.bank_gold as i32,
                },
            ))
        })
        .collect()
}

/// Applies each [`TwoGuardOutcomeEvent`] queued by
/// `World::process_two_guard_actions`. Every event carries a full new
/// `twocity_ppd` snapshot (see that type's own doc comment for why),
/// plus an optional bank-balance deduction for the "pay" text command's
/// bank-account fallback.
pub(crate) fn apply_two_guard_events(
    runtime: &mut ServerRuntime,
    events: Vec<TwoGuardOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        let Some(player) = runtime.player_for_character_mut(event.player_id) else {
            continue;
        };
        player.set_twocity_legal_status(event.legal_status);
        player.set_twocity_legal_fine(event.legal_fine);
        player.set_twocity_citizen_status(event.citizen_status);
        player.set_twocity_current_guard(event.current_guard);
        player.set_twocity_current_guard_time(event.current_guard_time);
        player.set_twocity_last_attack(event.last_attack);
        player.set_twocity_guard_intro(event.guard_intro);
        if let Some(need) = event.bank_gold_deduction {
            player.bank_gold = player.bank_gold.saturating_sub(need.max(0) as u32);
        }
        applied += 1;
    }
    applied
}

pub(crate) fn two_servant_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, TwoServantPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                TwoServantPlayerFacts {
                    citizen_status: player.twocity_citizen_status(),
                },
            ))
        })
        .collect()
}

/// Applies each [`TwoServantOutcomeEvent`] queued by
/// `World::process_two_servant_actions`: both variants create a secret-
/// passage key via `ZoneLoader` (`create_item("palace_key1"/"palace_
/// key2")`, `two.c:1200-1203`/`:1236-1239`/`:1250-1253`) and hand it to
/// the player, falling back to destroying it if the inventory is full -
/// matching C's plain `give_char_item` (not `give_char_item_smart`).
pub(crate) fn apply_two_servant_events(
    world: &mut World,
    zone_loader: &mut ZoneLoader,
    events: Vec<TwoServantOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        let (player_id, template) = match event {
            TwoServantOutcomeEvent::GivePalaceKey1 { player_id } => (player_id, "palace_key1"),
            TwoServantOutcomeEvent::GivePalaceKey2 { player_id } => (player_id, "palace_key2"),
        };
        if let Ok(key) = zone_loader.instantiate_item_template(template, Some(player_id)) {
            let key_id = key.id;
            world.add_item(key);
            if !world.give_char_item(player_id, key_id) {
                world.destroy_item(key_id);
            }
            applied += 1;
        }
    }
    applied
}

pub(crate) fn two_thiefguard_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, TwoThiefGuardPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                TwoThiefGuardPlayerFacts {
                    thief_state: player.twocity_thief_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`TwoThiefGuardOutcomeEvent`] queued by
/// `World::process_two_thiefguard_actions`: plain `twocity_ppd.thief_state`
/// bookkeeping, same shape as `apply_two_skelly_events`.
pub(crate) fn apply_two_thiefguard_events(
    runtime: &mut ServerRuntime,
    events: Vec<TwoThiefGuardOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        let TwoThiefGuardOutcomeEvent::UpdateThiefState {
            player_id,
            new_state,
        } = event;
        let Some(player) = runtime.player_for_character_mut(player_id) else {
            continue;
        };
        player.set_twocity_thief_state(new_state);
        applied += 1;
    }
    applied
}
