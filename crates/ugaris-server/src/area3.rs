//! Server-side wiring for area 3's crypt-entrance/crypt-quest/astronomer/
//! army-enrollment NPCs (`CDR_THOMAS`/`ugaris_core::world::thomas::
//! process_thomas_actions`, `CDR_SIRJONES`/`ugaris_core::world::
//! sir_jones::process_sir_jones_actions`, `CDR_ASTRO2`/`ugaris_core::
//! world::astro2::process_astro2_actions`, `CDR_SEYMOUR`/`ugaris_core::
//! world::seymour::process_seymour_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established in
//! `area1.rs`: [`thomas_player_facts`]/[`sir_jones_player_facts`]/
//! [`astro2_player_facts`]/[`seymour_player_facts`] snapshot the
//! per-player `area3_ppd`/`quest_log` facts each NPC's dialogue needs
//! before the tick, and [`apply_thomas_events`]/[`apply_sir_jones_events`]/
//! [`apply_astro2_events`]/[`apply_seymour_events`] apply the returned
//! events afterward. Sir Jones's crypt-quest reward (`SirJonesOutcomeEvent
//! ::GoldEarned`) and Astro2's lost-notes reward (`Astro2OutcomeEvent::
//! QuestDone`) both need `ZoneLoader::instantiate_item_template` - C's
//! `create_money_item` + plain `give_char_item` (not the auto-gold-
//! converting `give_char_item_smart`) - so [`apply_sir_jones_events`]/
//! [`apply_astro2_events`] both take a `&mut ZoneLoader` parameter, same
//! precedent as `area1.rs`'s `apply_lydia_events`/`apply_logain_events`.
//! Seymour's rewards ([`SeymourOutcomeEvent::LoisanNoteQuestDone`]/
//! [`SeymourOutcomeEvent::ZombieSkull2QuestDone`]) are military points +
//! exp, not a carried item, so [`apply_seymour_events`] needs only
//! `world`/`runtime`, no `loader`.

use super::*;
use ugaris_core::quest::quest_exp::MONEY_AREA3_MOONIES;
use ugaris_core::world::{
    Astro2OutcomeEvent, Astro2PlayerFacts, SeymourOutcomeEvent, SeymourPlayerFacts,
    SirJonesOutcomeEvent, SirJonesPlayerFacts, ThomasOutcomeEvent, ThomasPlayerFacts,
};

pub(crate) fn thomas_player_facts(
    world: &World,
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, ThomasPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            let level = world
                .characters
                .get(&character_id)
                .map(|character| character.level)
                .unwrap_or_default();
            Some((
                character_id,
                ThomasPlayerFacts {
                    crypt_state: player.area3_crypt_state(),
                    level,
                },
            ))
        })
        .collect()
}

/// Applies each [`ThomasOutcomeEvent`] queued by
/// `World::process_thomas_actions`.
pub(crate) fn apply_thomas_events(
    runtime: &mut ServerRuntime,
    events: Vec<ThomasOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            ThomasOutcomeEvent::UpdateCryptState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area3_crypt_state(new_state);
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn sir_jones_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, SirJonesPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                SirJonesPlayerFacts {
                    crypt_state: player.area3_crypt_state(),
                    crypt_bonus: player.area3_crypt_bonus(),
                    quest18_count: player.quest_log.count(18),
                    quest19_done: player.quest_log.is_done(19),
                },
            ))
        })
        .collect()
}

/// Applies each [`SirJonesOutcomeEvent`] queued by
/// `World::process_sir_jones_actions`. See the module doc comment for why
/// this needs `loader`.
pub(crate) async fn apply_sir_jones_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    events: Vec<SirJonesOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            SirJonesOutcomeEvent::UpdateCryptState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area3_crypt_state(new_state);
                applied += 1;
            }
            // C `ppd->crypt_bonus = 1;` (`area3.c:2012`).
            SirJonesOutcomeEvent::SetCryptBonus { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area3_crypt_bonus(1);
                applied += 1;
            }
            // C `questlog_open(co, ...)` (`src/system/questlog.c:204-217`):
            // sets the flag and unconditionally resends the questlog.
            SirJonesOutcomeEvent::QuestOpen { player_id, quest } => {
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
            // C `create_money_item(MONEY_AREA3_VAMPIRE1)` + plain
            // `give_char_item(co, in)` (`area3.c:1946-1951`) - see the
            // module doc comment for why this stays a literal carried
            // item instead of an instant gold credit.
            SirJonesOutcomeEvent::GoldEarned { player_id, amount } => {
                if let Ok(mut item) = loader.instantiate_item_template("money", None) {
                    item.value = amount;
                    item.sprite = create_money_item_sprite(amount);
                    item.description = format!("{:.2}G.", f64::from(amount) / 100.0);
                    let item_id = item.id;
                    world.add_item(item);
                    if !world.give_char_item(player_id, item_id) {
                        world.destroy_item(item_id);
                    }
                    applied += 1;
                }
            }
        }
    }
    applied
}

pub(crate) fn astro2_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, Astro2PlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                Astro2PlayerFacts {
                    astro2_state: player.area3_astro2_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`Astro2OutcomeEvent`] queued by
/// `World::process_astro2_actions`. See the module doc comment for why
/// this needs `loader`.
pub(crate) fn apply_astro2_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    events: Vec<Astro2OutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            Astro2OutcomeEvent::UpdateAstro2State {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area3_astro2_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, 16)` (`src/system/questlog.c:204-217`):
            // sets the flag and unconditionally resends the questlog.
            Astro2OutcomeEvent::QuestOpen { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(16);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `tmp = questlog_done(co, 16); ... if (tmp == 1) {
            // create_money_item(MONEY_AREA3_MOONIES) + give_char_item }`
            // (`area3.c:1633-1641`).
            Astro2OutcomeEvent::QuestDone { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(16, level, level_val) {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;

                    if completion.times_done == 1 {
                        let amount: u32 = MONEY_AREA3_MOONIES.max(0) as u32;
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
        }
    }
    applied
}

pub(crate) fn seymour_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, SeymourPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                SeymourPlayerFacts {
                    seymour_state: player.area3_seymour_state(),
                    quest11_done: player.quest_log.is_done(11),
                    quest12_done: player.quest_log.is_done(12),
                },
            ))
        })
        .collect()
}

/// Applies each [`SeymourOutcomeEvent`] queued by
/// `World::process_seymour_actions`. Unlike Sir Jones's/Astro2's item
/// rewards, Seymour's rewards are military points + exp
/// (`World::give_military_pts_from_npc`), so this needs only `world`/
/// `runtime`, no `ZoneLoader`.
pub(crate) fn apply_seymour_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    events: Vec<SeymourOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            SeymourOutcomeEvent::UpdateSeymourState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area3_seymour_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, ...)` (`src/system/questlog.c:204-217`):
            // sets the flag and unconditionally resends the questlog.
            SeymourOutcomeEvent::QuestOpen { player_id, quest } => {
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
            // C `tmp = questlog_done(co, 12); ... if (tmp == 1) {
            // give_military_pts(cn, co, 2, 1); }` (`area3.c:894-900`).
            SeymourOutcomeEvent::LoisanNoteQuestDone {
                player_id,
                seymour_id,
            } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(12, level, level_val) {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;

                    if completion.times_done == 1 {
                        world.give_military_pts_from_npc(
                            player_id,
                            seymour_id,
                            2,
                            1,
                            u32::from(world.area_id),
                        );
                    }
                }
            }
            // C `questlog_done(co, 10);` (`area3.c:907`) - return value
            // unused, no conditional point reward for this one.
            SeymourOutcomeEvent::ZombieSkull1QuestDone { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(10, level, level_val) {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;
                }
            }
            // C `tmp = questlog_done(co, 11); ... if (tmp == 1) {
            // give_military_pts(cn, co, 1, 1); }` (`area3.c:921-926`).
            SeymourOutcomeEvent::ZombieSkull2QuestDone {
                player_id,
                seymour_id,
            } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(11, level, level_val) {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;

                    if completion.times_done == 1 {
                        world.give_military_pts_from_npc(
                            player_id,
                            seymour_id,
                            1,
                            1,
                            u32::from(world.area_id),
                        );
                    }
                }
            }
        }
    }
    applied
}
