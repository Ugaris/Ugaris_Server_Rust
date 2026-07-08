//! Server-side wiring for area 3's crypt-entrance/crypt-quest/astronomer/
//! army-enrollment/park-shrine NPCs (`CDR_THOMAS`/`ugaris_core::world::
//! thomas::process_thomas_actions`, `CDR_SIRJONES`/`ugaris_core::world::
//! sir_jones::process_sir_jones_actions`, `CDR_ASTRO2`/`ugaris_core::
//! world::astro2::process_astro2_actions`, `CDR_SEYMOUR`/`ugaris_core::
//! world::seymour::process_seymour_actions`, `CDR_KELLY`/`ugaris_core::
//! world::kelly::process_kelly_actions`).
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
use crate::achievement::{award_dragonsbane_achievement, award_swap_money_converted_achievement};
use ugaris_core::quest::quest_exp::MONEY_AREA3_MOONIES;
use ugaris_core::world::{
    Astro2OutcomeEvent, Astro2PlayerFacts, CarlosOutcomeEvent, CarlosPlayerFacts,
    KassimOutcomeEvent, KassimPlayerFacts, KellyOutcomeEvent, KellyPlayerFacts,
    SeymourOutcomeEvent, SeymourPlayerFacts, SirJonesOutcomeEvent, SirJonesPlayerFacts,
    ThomasOutcomeEvent, ThomasPlayerFacts,
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

pub(crate) fn kelly_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, KellyPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                KellyPlayerFacts {
                    kelly_state: player.area3_kelly_state(),
                    seymour_state: player.area3_seymour_state(),
                    quest14_done: player.quest_log.is_done(14),
                    quest15_done: player.quest_log.is_done(15),
                    clara_state: player.area3_clara_state(),
                    found1: player.area3_kelly_found1(),
                    found2: player.area3_kelly_found2(),
                    found3: player.area3_kelly_found3(),
                    found_cnt: player.area3_kelly_found_cnt(),
                    quest54_count: player.quest_log.count(54),
                    quest60_count: player.quest_log.count(60),
                },
            ))
        })
        .collect()
}

/// Applies each [`KellyOutcomeEvent`] queued by
/// `World::process_kelly_actions`. Needs `loader` for
/// [`KellyOutcomeEvent::GrantCaligarLetter`] (same `ZoneLoader::
/// instantiate_item_template` precedent as `apply_sir_jones_events`/
/// `apply_astro2_events`) and `achievement_repository` for
/// [`KellyOutcomeEvent::GoldEarned`]'s wealth-ladder half (same
/// `award_swap_money_converted_achievement` precedent as `area1.rs`'s
/// `GwendylonOutcomeEvent::GoldEarned`/`ReskinOutcomeEvent::GoldEarned`).
pub(crate) async fn apply_kelly_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    events: Vec<KellyOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            KellyOutcomeEvent::UpdateKellyState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area3_kelly_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, ...)` (`src/system/questlog.c:204-217`):
            // sets the flag and unconditionally resends the questlog.
            KellyOutcomeEvent::QuestOpen { player_id, quest } => {
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
            // C `ppd->kelly_found_cnt = cnt;` (`area3.c:1116`).
            KellyOutcomeEvent::UpdateFoundCnt {
                player_id,
                new_found_cnt,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area3_kelly_found_cnt(new_found_cnt);
                applied += 1;
            }
            // C `tmp = questlog_done(co, 13); ... if (tmp == 1) {
            // give_military_pts(cn, co, 4, 1); }` (`area3.c:1328-1333`).
            KellyOutcomeEvent::CreeperHeadQuestDone {
                player_id,
                kelly_id,
            } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(13, level, level_val) {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;

                    if completion.times_done == 1 {
                        world.give_military_pts_from_npc(
                            player_id,
                            kelly_id,
                            4,
                            1,
                            u32::from(world.area_id),
                        );
                    }
                }
            }
            // C `questlog_done(co, 14);` (`area3.c:1123`) - return value
            // unused, no conditional point reward (quest 14's own table
            // `exp` is `0`; the real reward already came from `case 9`'s
            // per-shrine `give_military_pts` calls, applied directly in
            // `World`).
            KellyOutcomeEvent::ParkShrinesQuestDone { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(14, level, level_val) {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;
                }
            }
            // C `questlog_done(co, 15); give_military_pts(cn, co, 3, 1);`
            // (`area3.c:1176-1177`) - unconditional, unlike the `NT_GIVE`
            // completions above.
            KellyOutcomeEvent::ClaraReportDone {
                player_id,
                kelly_id,
            } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(15, level, level_val) {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;
                }
                world.give_military_pts_from_npc(
                    player_id,
                    kelly_id,
                    3,
                    1,
                    u32::from(world.area_id),
                );
            }
            // C `give_money`'s `achievement_add_gold_earned` wealth-ladder
            // half - see the module doc comment and `KellyOutcomeEvent::
            // GoldEarned`'s own doc comment.
            KellyOutcomeEvent::GoldEarned { player_id, amount } => {
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
            // C `case 24`'s conditional letter grant (`area3.c:1239-1244`):
            // `create_item("caligar_letter")` + `give_char_item`.
            KellyOutcomeEvent::GrantCaligarLetter { player_id } => {
                if let Ok(item) = loader.instantiate_item_template("caligar_letter", None) {
                    let item_id = item.id;
                    world.add_item(item);
                    if !world.give_char_item(player_id, item_id) {
                        world.destroy_item(item_id);
                    }
                    applied += 1;
                }
            }
            // C `questlog_done(co, 60);` (`area3.c:1339`) - the exp/resend
            // half; the `give_money` reward is applied directly in `World`
            // (see `KellyOutcomeEvent::GoldEarned`).
            KellyOutcomeEvent::PlaqueQuestDone { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(60, level, level_val) {
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

pub(crate) fn carlos_player_facts(
    world: &World,
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, CarlosPlayerFacts> {
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
                CarlosPlayerFacts {
                    carlos_state: player.staffer_carlos_state(),
                    carlos2_state: player.staffer_carlos2_state(),
                    level,
                    quest61_count: player.quest_log.count(61),
                },
            ))
        })
        .collect()
}

/// Applies each [`CarlosOutcomeEvent`] queued by
/// `World::process_carlos_actions`. Needs `loader` for
/// [`CarlosOutcomeEvent::GrantCarlosKey`] (same `ZoneLoader::
/// instantiate_item_template` precedent as `apply_sir_jones_events`) and
/// `achievement_repository` for [`CarlosOutcomeEvent::
/// DragonStaffQuestDone`]'s unconditional Dragonsbane award.
pub(crate) async fn apply_carlos_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    events: Vec<CarlosOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            CarlosOutcomeEvent::UpdateCarlosState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_carlos_state(new_state);
                applied += 1;
            }
            CarlosOutcomeEvent::UpdateCarlos2State {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_staffer_carlos2_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, ...)` (`src/system/questlog.c:204-217`):
            // sets the flag and unconditionally resends the questlog.
            CarlosOutcomeEvent::QuestOpen { player_id, quest } => {
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
            // C `case 4`'s conditional key grant (`area3.c:2205-2210`):
            // `create_item("carlos_key")` + `give_char_item`, speaking
            // "Thou wilt need this key to unlock the door in front of the
            // stairs down." only on success (see the `world::carlos`
            // module doc comment for why that follow-up line lives here,
            // not in `World`).
            CarlosOutcomeEvent::GrantCarlosKey {
                player_id,
                carlos_id,
            } => {
                if let Ok(item) = loader.instantiate_item_template("carlos_key", None) {
                    let item_id = item.id;
                    world.add_item(item);
                    if world.give_char_item(player_id, item_id) {
                        world.npc_quiet_say(
                            carlos_id,
                            "Thou wilt need this key to unlock the door in front of the stairs down.",
                        );
                    } else {
                        world.destroy_item(item_id);
                    }
                    applied += 1;
                }
            }
            // C `tmp = questlog_done(co, 20); ... achievement_award(co,
            // ACHIEVEMENT_DRAGONSBANE, 1);` (`area3.c:2266-2267`) -
            // unlike every other quest-completion event in this file, the
            // achievement award is unconditional (not gated on
            // `times_done == 1`), matching quest 20's `QLF_REPEATABLE`
            // flag and C's own unconditional call.
            CarlosOutcomeEvent::DragonStaffQuestDone { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(20, level, level_val) {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;
                }
                award_dragonsbane_achievement(world, runtime, achievement_repository, player_id)
                    .await;
            }
            // C `questlog_done(co, 61);` (`area3.c:2280`) - the exp/resend
            // half; no achievement or extra reward attached.
            CarlosOutcomeEvent::RitualQuestDone { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(61, level, level_val) {
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

pub(crate) fn kassim_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, KassimPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                KassimPlayerFacts {
                    kassim_state: player.area3_kassim_state(),
                    kassim_seen_timer: player.area3_kassim_seen_timer(),
                    kassim_item_wait_starttime: player.area3_kassim_item_wait_starttime(),
                },
            ))
        })
        .collect()
}

/// Applies each [`KassimOutcomeEvent`] queued by
/// `World::process_kassim_actions`. Unlike Sir Jones's/Astro2's item
/// rewards, none of Kassim's events touch `ZoneLoader` or achievements -
/// the gold charge and the item engraving itself both happen directly in
/// `World` (see `world::kassim`'s own module doc comment).
pub(crate) fn apply_kassim_events(
    runtime: &mut ServerRuntime,
    events: Vec<KassimOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            KassimOutcomeEvent::UpdateKassimState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area3_kassim_state(new_state);
                applied += 1;
            }
            KassimOutcomeEvent::UpdateSeenTimer { player_id, value } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area3_kassim_seen_timer(value);
                applied += 1;
            }
            KassimOutcomeEvent::UpdateItemWaitStart { player_id, value } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area3_kassim_item_wait_starttime(value);
                applied += 1;
            }
        }
    }
    applied
}
