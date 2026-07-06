//! Server-side wiring for area 1's forest hermit, hunter, lore,
//! town-greeter, robber-quest, and forest-ranger NPCs
//! (`CDR_CAMHERMIT`/`ugaris_core::world::camhermit::process_camhermit_actions`,
//! `CDR_YOAKIN`/`ugaris_core::world::yoakin::process_yoakin_actions`,
//! `CDR_TERION`/`ugaris_core::world::terion::process_terion_actions`,
//! `CDR_GREETER`/`ugaris_core::world::greeter::process_greeter_actions`,
//! `CDR_JESSICA`/`ugaris_core::world::jessica::process_jessica_actions`,
//! `CDR_JIU`/`ugaris_core::world::jiu::process_jiu_actions`,
//! `CDR_FOREST_RANGER`/`ugaris_core::world::forest_ranger::
//! process_forest_ranger_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established for
//! `world::gatekeeper`'s `GateWelcomePlayerFacts`/`GateWelcomeOutcomeEvent`
//! (see `world::camhermit`'s module doc comment): [`camhermit_player_facts`]/
//! [`yoakin_player_facts`]/[`terion_player_facts`]/[`greeter_player_facts`]/
//! [`jessica_player_facts`]/[`jiu_player_facts`]/[`forest_ranger_player_facts`]
//! snapshot the per-player `area1_ppd`/`quest_log` facts each NPC's
//! dialogue needs before the tick, and
//! [`apply_camhermit_events`]/[`apply_yoakin_events`]/[`apply_terion_events`]/
//! [`apply_greeter_events`]/[`apply_jessica_events`]/[`apply_jiu_events`]/
//! [`apply_forest_ranger_events`]
//! apply the returned events afterward, including the
//! `QLOG_HERMIT_QUEST1/2`/`QLOG_YOAKIN`/`QLOG_JESSICA_*`/`QLOG_JIU`
//! `questlog_open`/`questlog_done`/`questlog_reopen` calls C's own drivers
//! make (each of which unconditionally resends the legacy questlog packet,
//! matching `apply_military_mission_kill_check`'s precedent) and each
//! quest's reward gold wealth-achievement tracking (`give_money`'s
//! `achievement_add_gold_earned` half - see `World::
//! give_char_item_smart`'s doc comment for why that split exists). Terion
//! and the greeter are pure ambient/tutorial dialogue (no quest log
//! writes), so their own facts/events are the simplest of the group - the
//! greeter only *reads* `QLOG_LYDIA`'s completion flag, never writes it.
//! Jessica's and Jiu's own quest completions (unlike every sibling above)
//! carry no gold reward at all, so [`apply_jessica_events`]/
//! [`apply_jiu_events`] need no achievement wiring. See `world::jiu`'s own
//! module doc comment for the still-missing `riverbeast_dead` death-hook
//! gap this NPC's quest completion depends on. The forest ranger has no
//! quest log at all (a pure ambient warning NPC, like Terion), so
//! [`apply_forest_ranger_events`] only ever writes the two plain
//! `area1_ppd` fields.

use super::*;
use ugaris_core::quest::{QLOG_HERMIT_QUEST2, QLOG_JIU, QLOG_LYDIA, QLOG_NOOK, QLOG_YOAKIN};
use ugaris_core::world::{
    CamhermitOutcomeEvent, CamhermitPlayerFacts, ForestRangerOutcomeEvent, ForestRangerPlayerFacts,
    GreeterOutcomeEvent, GreeterPlayerFacts, GwendylonOutcomeEvent, GwendylonPlayerFacts,
    JessicaOutcomeEvent, JessicaPlayerFacts, JiuOutcomeEvent, JiuPlayerFacts, TerionOutcomeEvent,
    TerionPlayerFacts, YoakinOutcomeEvent, YoakinPlayerFacts,
};

pub(crate) fn camhermit_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, CamhermitPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                CamhermitPlayerFacts {
                    state: player.area1_camhermit_state(),
                    seen_timer: player.area1_camhermit_seen_timer(),
                    kills: player.area1_camhermit_kills(),
                    quest2_done_count: player.quest_log.count(QLOG_HERMIT_QUEST2),
                },
            ))
        })
        .collect()
}

/// Applies each [`CamhermitOutcomeEvent`] queued by
/// `World::process_camhermit_actions`. See the module doc comment.
pub(crate) async fn apply_camhermit_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    events: Vec<CamhermitOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            CamhermitOutcomeEvent::UpdateState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_camhermit_state(new_state);
                applied += 1;
            }
            CamhermitOutcomeEvent::UpdateSeenTimer { player_id, value } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_camhermit_seen_timer(value);
                applied += 1;
            }
            // C `questlog_open(co, ...)` (`src/system/questlog.c:204-217`):
            // sets the flag and unconditionally resends the questlog.
            CamhermitOutcomeEvent::QuestOpen { player_id, quest } => {
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
            // C `questlog_done(co, ...)` (`src/system/questlog.c:267-305`):
            // full exp-reward port via `QuestLog::complete_legacy`,
            // applied through `World::give_exp` (matching every other
            // quest-completion exp grant in this codebase), plus the
            // unconditional questlog resend.
            CamhermitOutcomeEvent::QuestDone { player_id, quest } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(quest, level, level_val)
                {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;
                }
            }
            // C `questlog_reopen(co, ...)` (`src/system/questlog.c:307-
            // 322`).
            CamhermitOutcomeEvent::QuestReopen { player_id, quest } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.reopen(quest);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            CamhermitOutcomeEvent::GoldEarned { player_id, amount } => {
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
        }
    }
    applied
}

pub(crate) fn yoakin_player_facts(
    world: &World,
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, YoakinPlayerFacts> {
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
                YoakinPlayerFacts {
                    state: player.area1_yoakin_state(),
                    seen_timer: player.area1_yoakin_seen_timer(),
                    logain_state: player.area1_logain_state(),
                    quest_done_count: player.quest_log.count(QLOG_YOAKIN),
                    shrike_state: player.area1_shrike_state(),
                    shrike_fails: player.area1_shrike_fails(),
                    level,
                },
            ))
        })
        .collect()
}

/// Applies each [`YoakinOutcomeEvent`] queued by
/// `World::process_yoakin_actions`. See the module doc comment.
pub(crate) async fn apply_yoakin_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    events: Vec<YoakinOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            YoakinOutcomeEvent::UpdateState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_yoakin_state(new_state);
                applied += 1;
            }
            YoakinOutcomeEvent::UpdateSeenTimer { player_id, value } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_yoakin_seen_timer(value);
                applied += 1;
            }
            // C `questlog_open(co, 5)` (`src/system/questlog.c:204-217`):
            // sets the flag and unconditionally resends the questlog.
            YoakinOutcomeEvent::QuestOpen { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(QLOG_YOAKIN);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, 5)` (`src/system/questlog.c:267-305`):
            // full exp-reward port via `QuestLog::complete_legacy`,
            // applied through `World::give_exp` (matching every other
            // quest-completion exp grant in this codebase), plus the
            // unconditional questlog resend.
            YoakinOutcomeEvent::QuestDone { player_id } => {
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
                        .complete_legacy(QLOG_YOAKIN, level, level_val)
                {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;
                }
            }
            YoakinOutcomeEvent::GoldEarned { player_id, amount } => {
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
            YoakinOutcomeEvent::UpdateShrikeState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_shrike_state(new_state);
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn terion_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, TerionPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                TerionPlayerFacts {
                    state: player.area1_terion_state(),
                    gwendy_state: player.area1_gwendy_state(),
                    reskin_state: player.area1_reskin_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`TerionOutcomeEvent`] queued by
/// `World::process_terion_actions`. See the module doc comment.
pub(crate) fn apply_terion_events(
    runtime: &mut ServerRuntime,
    events: Vec<TerionOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            TerionOutcomeEvent::UpdateState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_terion_state(new_state);
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn gwendylon_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, GwendylonPlayerFacts> {
    use ugaris_core::quest::{
        QLOG_GWENDY_FIRST_SKULL, QLOG_GWENDY_FOUL_MAGICIAN, QLOG_GWENDY_SECOND_SKULL,
        QLOG_GWENDY_THIRD_SKULL,
    };
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                GwendylonPlayerFacts {
                    state: player.area1_gwendy_state(),
                    seen_timer: player.area1_gwendy_seen_timer(),
                    quest2_isdone: player.quest_log.is_done(QLOG_GWENDY_SECOND_SKULL),
                    quest3_isdone: player.quest_log.is_done(QLOG_GWENDY_THIRD_SKULL),
                    quest4_isdone: player.quest_log.is_done(QLOG_GWENDY_FOUL_MAGICIAN),
                    quest1_done_count: player.quest_log.count(QLOG_GWENDY_FIRST_SKULL),
                    quest2_done_count: player.quest_log.count(QLOG_GWENDY_SECOND_SKULL),
                    quest3_done_count: player.quest_log.count(QLOG_GWENDY_THIRD_SKULL),
                    quest4_done_count: player.quest_log.count(QLOG_GWENDY_FOUL_MAGICIAN),
                },
            ))
        })
        .collect()
}

/// Applies each [`GwendylonOutcomeEvent`] queued by
/// `World::process_gwendylon_actions`. See the module doc comment.
pub(crate) async fn apply_gwendylon_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    events: Vec<GwendylonOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            GwendylonOutcomeEvent::UpdateState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_gwendy_state(new_state);
                applied += 1;
            }
            GwendylonOutcomeEvent::UpdateSeenTimer { player_id, value } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_gwendy_seen_timer(value);
                applied += 1;
            }
            // C `questlog_open(co, ...)` (`src/system/questlog.c:204-217`):
            // sets the flag and unconditionally resends the questlog.
            GwendylonOutcomeEvent::QuestOpen { player_id, quest } => {
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
            // C `questlog_done(co, ...)` (`src/system/questlog.c:267-305`):
            // full exp-reward port via `QuestLog::complete_legacy`,
            // applied through `World::give_exp` (matching every other
            // quest-completion exp grant in this codebase), plus the
            // unconditional questlog resend.
            GwendylonOutcomeEvent::QuestDone { player_id, quest } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(quest, level, level_val)
                {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;
                }
            }
            GwendylonOutcomeEvent::GoldEarned { player_id, amount } => {
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
        }
    }
    applied
}

/// Resolves every `IID_CALIGARLETTER` cross-area hand-off queued by
/// `World::process_gwendylon_actions` (C `change_area(co, 36, 240, 10)`,
/// `src/area/1/gwendylon.c:637`) via the shared `attempt_cross_area_transfer`
/// helper, same as every other cross-area call site. On failure, Gwendylon
/// herself says the "rift in the space-time continuum" line (C `quiet_say
/// (cn, ...)`, `gwendylon.c:638-639`) rather than a private message to the
/// caller, since C's own fallback here is an audible NPC line, not a
/// targeted system text.
pub(crate) async fn apply_gwendylon_cross_area_transfers(
    world: &mut World,
    runtime: &mut ServerRuntime,
    character_repository: &Option<ugaris_db::PgCharacterRepository>,
    area_repository: &Option<ugaris_db::PgAreaRepository>,
    area_id: u16,
    mirror_id: u16,
) -> usize {
    let transfers = world.drain_pending_gwendylon_cross_area_transfers();
    if transfers.is_empty() {
        return 0;
    }
    let mut applied = 0;
    for transfer in transfers {
        let transferred = attempt_cross_area_transfer(
            world,
            runtime,
            character_repository,
            area_repository,
            area_id,
            mirror_id,
            transfer.player_id,
            36,
            u32::from(mirror_id),
            240,
            10,
        )
        .await;
        if !transferred {
            world.npc_quiet_say(
                transfer.gwendylon_id,
                "Uh-Oh. There seems to be a rift in the space-time continuum. Please come again later so we can try again.",
            );
        }
        applied += 1;
    }
    applied
}

pub(crate) fn greeter_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, GreeterPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                GreeterPlayerFacts {
                    state: player.area1_greeter_state(),
                    seen_timer: player.area1_greeter_seen_timer(),
                    james_state: player.area1_james_state(),
                    lydia_quest_done: player.quest_log.is_done(QLOG_LYDIA),
                },
            ))
        })
        .collect()
}

/// Applies each [`GreeterOutcomeEvent`] queued by
/// `World::process_greeter_actions`. See the module doc comment.
pub(crate) fn apply_greeter_events(
    runtime: &mut ServerRuntime,
    events: Vec<GreeterOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            GreeterOutcomeEvent::UpdateState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_greeter_state(new_state);
                applied += 1;
            }
            GreeterOutcomeEvent::UpdateSeenTimer { player_id, value } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_greeter_seen_timer(value);
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn jessica_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, JessicaPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                JessicaPlayerFacts {
                    state: player.area1_jessica_state(),
                    seen_timer: player.area1_jessica_seen_timer(),
                    nook_quest_done: player.quest_log.is_done(QLOG_NOOK),
                },
            ))
        })
        .collect()
}

/// Applies each [`JessicaOutcomeEvent`] queued by
/// `World::process_jessica_actions`. See the module doc comment.
pub(crate) fn apply_jessica_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    events: Vec<JessicaOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            JessicaOutcomeEvent::UpdateState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_jessica_state(new_state);
                applied += 1;
            }
            JessicaOutcomeEvent::UpdateSeenTimer { player_id, value } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_jessica_seen_timer(value);
                applied += 1;
            }
            // C `questlog_open(co, ...)` (`src/system/questlog.c:204-217`):
            // sets the flag and unconditionally resends the questlog.
            JessicaOutcomeEvent::QuestOpen { player_id, quest } => {
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
            // C `questlog_done(co, ...)` (`src/system/questlog.c:267-305`):
            // full exp-reward port via `QuestLog::complete_legacy`,
            // applied through `World::give_exp` (matching every other
            // quest-completion exp grant in this codebase), plus the
            // unconditional questlog resend. Jessica's own two quests
            // carry no gold reward (unlike her siblings above), so no
            // achievement wiring is needed here.
            JessicaOutcomeEvent::QuestDone { player_id, quest } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(quest, level, level_val)
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

pub(crate) fn jiu_player_facts(runtime: &ServerRuntime) -> HashMap<CharacterId, JiuPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                JiuPlayerFacts {
                    state: player.area1_jiu_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`JiuOutcomeEvent`] queued by `World::process_jiu_actions`.
/// See the module doc comment. Jiu's own quest (unlike her Yoakin/
/// Camhermit/Gwendylon siblings) carries no gold reward, so - like
/// Jessica's - no achievement wiring is needed here.
pub(crate) fn apply_jiu_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    events: Vec<JiuOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            JiuOutcomeEvent::UpdateState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_jiu_state(new_state);
                applied += 1;
            }
            JiuOutcomeEvent::UpdateSeenTimer { player_id, value } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_jiu_seen_timer(value);
                applied += 1;
            }
            // C `questlog_open(co, QLOG_JIU)` (`src/system/questlog.c:204-
            // 217`): sets the flag and unconditionally resends the
            // questlog.
            JiuOutcomeEvent::QuestOpen { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(QLOG_JIU);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, QLOG_JIU)` (`src/system/questlog.c:267-
            // 305`): full exp-reward port via `QuestLog::complete_legacy`,
            // applied through `World::give_exp` (matching every other
            // quest-completion exp grant in this codebase), plus the
            // unconditional questlog resend.
            JiuOutcomeEvent::QuestDone { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) =
                    player.quest_log.complete_legacy(QLOG_JIU, level, level_val)
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

pub(crate) fn forest_ranger_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, ForestRangerPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                ForestRangerPlayerFacts {
                    state: player.area1_forest_ranger_state(),
                    seen_timer: player.area1_forest_ranger_seen_timer(),
                },
            ))
        })
        .collect()
}

/// Applies each [`ForestRangerOutcomeEvent`] queued by
/// `World::process_forest_ranger_actions`. See the module doc comment.
pub(crate) fn apply_forest_ranger_events(
    runtime: &mut ServerRuntime,
    events: Vec<ForestRangerOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            ForestRangerOutcomeEvent::UpdateState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_forest_ranger_state(new_state);
                applied += 1;
            }
            ForestRangerOutcomeEvent::UpdateSeenTimer { player_id, value } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_forest_ranger_seen_timer(value);
                applied += 1;
            }
        }
    }
    applied
}
