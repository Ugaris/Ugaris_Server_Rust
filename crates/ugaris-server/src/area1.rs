//! Server-side wiring for area 1's forest hermit, hunter, lore,
//! town-greeter, robber-quest, and forest-ranger NPCs
//! (`CDR_CAMHERMIT`/`ugaris_core::world::camhermit::process_camhermit_actions`,
//! `CDR_YOAKIN`/`ugaris_core::world::yoakin::process_yoakin_actions`,
//! `CDR_TERION`/`ugaris_core::world::terion::process_terion_actions`,
//! `CDR_GREETER`/`ugaris_core::world::greeter::process_greeter_actions`,
//! `CDR_JESSICA`/`ugaris_core::world::jessica::process_jessica_actions`,
//! `CDR_JIU`/`ugaris_core::world::jiu::process_jiu_actions`,
//! `CDR_FOREST_RANGER`/`ugaris_core::world::forest_ranger::
//! process_forest_ranger_actions`,
//! `CDR_BRITHILDIE`/`ugaris_core::world::brithildie::
//! process_brithildie_actions`,
//! `CDR_NOOK`/`ugaris_core::world::nook::process_nook_actions`,
//! `CDR_LYDIA`/`ugaris_core::world::lydia::process_lydia_actions`).
//!
//! Mirrors the `World`/`PlayerRuntime` split already established for
//! `world::gatekeeper`'s `GateWelcomePlayerFacts`/`GateWelcomeOutcomeEvent`
//! (see `world::camhermit`'s module doc comment): [`camhermit_player_facts`]/
//! [`yoakin_player_facts`]/[`terion_player_facts`]/[`greeter_player_facts`]/
//! [`jessica_player_facts`]/[`jiu_player_facts`]/[`forest_ranger_player_facts`]/
//! [`brithildie_player_facts`]
//! snapshot the per-player `area1_ppd`/`quest_log` facts each NPC's
//! dialogue needs before the tick, and
//! [`apply_camhermit_events`]/[`apply_yoakin_events`]/[`apply_terion_events`]/
//! [`apply_greeter_events`]/[`apply_jessica_events`]/[`apply_jiu_events`]/
//! [`apply_forest_ranger_events`]/[`apply_brithildie_events`]
//! apply the returned events afterward, including the
//! `QLOG_HERMIT_QUEST1/2`/`QLOG_YOAKIN`/`QLOG_JESSICA_*`/`QLOG_JIU`/
//! `QLOG_BRITHILDIE`
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
//! `area1_ppd` fields. Brithildie only ever *opens* `QLOG_BRITHILDIE`
//! (`questlog_done` fires from a separate death hook,
//! `apply_bigbadspider_death_from_hurt_event` in
//! `world_events::death_hooks`, since it is `bigbadspider_dead`, not
//! `brithildie_driver` itself, that completes the quest - same split as
//! `world::jiu`'s `riverbeast_dead`), so [`apply_brithildie_events`] needs
//! no `QuestDone`/achievement handling either. Nook's own quest (the
//! stolen-cap side quest) carries no gold reward either (`nook_driver`'s
//! own turn-in line says so explicitly), so [`apply_nook_events`] needs no
//! achievement wiring, same as Jessica/Jiu. Lydia's own quest completion
//! carries no gold reward either, but does carry a class-conditional
//! reward *item* (`mana_potion1`/`healing_potion1`) that needs
//! `ZoneLoader::instantiate_item_template` (`World` has no template
//! access), so [`apply_lydia_events`] takes a `&mut ZoneLoader` parameter,
//! unlike every other `apply_*_events` function in this file (mirroring
//! `world_events::npc_events::apply_gate_welcome_events`'s own precedent
//! for the same reason), plus `ACHIEVEMENT_A_HELPING_HAND` on
//! `QuestDone`. Reskin's own quest completion (`QLOG_RESKIN`, index 17,
//! "The Unwanted Tenants") carries no gold reward on `QuestDone` itself,
//! but its separate alchemy-ingredient turn-in path (`ReskinOutcomeEvent::
//! GoldEarned`/`WellPaidGathererAchievement`) does, so
//! [`apply_reskin_events`] wires both `award_swap_money_converted_
//! achievement` (matching every other `GoldEarned` handler in this file)
//! and a dedicated `award_reskin_well_paid_gatherer_achievement`.

use super::*;
use ugaris_core::item_ops::{give_item_to_character, GiveItemFlags, GiveItemResult};
use ugaris_core::quest::quest_exp::{
    MONEY_AREA1_MADKNIGHT, MONEY_AREA1_MADMAGE1, MONEY_AREA1_MADMAGE2,
};
use ugaris_core::quest::{
    QLOG_BRITHILDIE, QLOG_HERMIT_QUEST2, QLOG_JIU, QLOG_LYDIA, QLOG_NOOK, QLOG_RESKIN, QLOG_YOAKIN,
};
use ugaris_core::world::{
    AsturinOutcomeEvent, AsturinPlayerFacts, BrithildieOutcomeEvent, BrithildiePlayerFacts,
    CamhermitOutcomeEvent, CamhermitPlayerFacts, ForestRangerOutcomeEvent, ForestRangerPlayerFacts,
    GreeterOutcomeEvent, GreeterPlayerFacts, GuiwynnOutcomeEvent, GuiwynnPlayerFacts,
    GwendylonOutcomeEvent, GwendylonPlayerFacts, JamesOutcomeEvent, JamesPlayerFacts,
    JessicaOutcomeEvent, JessicaPlayerFacts, JiuOutcomeEvent, JiuPlayerFacts, LogainOutcomeEvent,
    LogainPlayerFacts, LydiaOutcomeEvent, LydiaPlayerFacts, NookOutcomeEvent, NookPlayerFacts,
    ReskinOutcomeEvent, ReskinPlayerFacts, TerionOutcomeEvent, TerionPlayerFacts,
    YoakinOutcomeEvent, YoakinPlayerFacts,
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

pub(crate) fn brithildie_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, BrithildiePlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                BrithildiePlayerFacts {
                    state: player.area1_brithildie_state(),
                    seen_timer: player.area1_brithildie_seen_timer(),
                },
            ))
        })
        .collect()
}

/// Applies each [`BrithildieOutcomeEvent`] queued by
/// `World::process_brithildie_actions`. See the module doc comment.
pub(crate) fn apply_brithildie_events(
    runtime: &mut ServerRuntime,
    events: Vec<BrithildieOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            BrithildieOutcomeEvent::UpdateState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_brithildie_state(new_state);
                applied += 1;
            }
            BrithildieOutcomeEvent::UpdateSeenTimer { player_id, value } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_brithildie_seen_timer(value);
                applied += 1;
            }
            // C `questlog_open(co, QLOG_BRITHILDIE)` (`src/system/
            // questlog.c:204-217`): sets the flag and unconditionally
            // resends the questlog.
            BrithildieOutcomeEvent::QuestOpen { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(QLOG_BRITHILDIE);
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

pub(crate) fn nook_player_facts(runtime: &ServerRuntime) -> HashMap<CharacterId, NookPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                NookPlayerFacts {
                    state: player.area1_nook_state(),
                    gwendy_state: player.area1_gwendy_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`NookOutcomeEvent`] queued by `World::process_nook_actions`.
/// See the module doc comment.
pub(crate) fn apply_nook_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    events: Vec<NookOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            NookOutcomeEvent::UpdateState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_nook_state(new_state);
                applied += 1;
            }
            // C `questlog_open(co, QLOG_NOOK)` (`src/system/questlog.c:204-
            // 217`): sets the flag and unconditionally resends the
            // questlog.
            NookOutcomeEvent::QuestOpen { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(QLOG_NOOK);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, QLOG_NOOK)` (`src/system/questlog.c:267-
            // 305`): full exp-reward port via `QuestLog::complete_legacy`,
            // applied through `World::give_exp` (matching every other
            // quest-completion exp grant in this codebase), plus the
            // unconditional questlog resend. Nook's own quest carries no
            // gold reward, so no achievement wiring is needed here.
            NookOutcomeEvent::QuestDone { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player
                    .quest_log
                    .complete_legacy(QLOG_NOOK, level, level_val)
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

pub(crate) fn lydia_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, LydiaPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                LydiaPlayerFacts {
                    state: player.area1_lydia_state(),
                    seen_timer: player.area1_lydia_seen_timer(),
                },
            ))
        })
        .collect()
}

/// Applies each [`LydiaOutcomeEvent`] queued by
/// `World::process_lydia_actions`. See the module doc comment for why
/// this one (uniquely among this file's `apply_*_events` functions) needs
/// a `&mut ZoneLoader`.
pub(crate) async fn apply_lydia_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    events: Vec<LydiaOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            LydiaOutcomeEvent::UpdateState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_lydia_state(new_state);
                applied += 1;
            }
            LydiaOutcomeEvent::UpdateSeenTimer { player_id, value } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_lydia_seen_timer(value);
                applied += 1;
            }
            // C `questlog_open(co, 0)` (`src/system/questlog.c:204-217`):
            // sets the flag and unconditionally resends the questlog.
            LydiaOutcomeEvent::QuestOpen { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(QLOG_LYDIA);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, QLOG_LYDIA)` (`src/system/
            // questlog.c:267-305`) plus `achievement_award(co,
            // ACHIEVEMENT_A_HELPING_HAND, 1)` (`gwendylon.c:3607`).
            LydiaOutcomeEvent::QuestDone { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player
                    .quest_log
                    .complete_legacy(QLOG_LYDIA, level, level_val)
                {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    award_lydia_helping_hand_achievement(
                        world,
                        runtime,
                        achievement_repository,
                        player_id,
                    )
                    .await;
                    applied += 1;
                }
            }
            // C's class-conditional `create_item(...)`/`give_char_item(co,
            // in)` reward (`gwendylon.c:614-621`) - the plain
            // (non-drop-fallback) give, matching C's own plain
            // `give_char_item`. `grant_clan_jewel`'s own precedent: on any
            // non-`Ok` result the freshly instantiated `Item` is simply
            // never registered in `world.items`, which is the Rust
            // equivalent of C's `destroy_item(in)` on failure (there is
            // nothing to destroy since it was never added).
            LydiaOutcomeEvent::GrantPotion {
                player_id,
                template,
            } => {
                if let Ok(mut item) = loader.instantiate_item_template(template, Some(player_id)) {
                    if let Some(character) = world.characters.get_mut(&player_id) {
                        if let GiveItemResult::Ok =
                            give_item_to_character(character, &mut item, GiveItemFlags::NONE)
                        {
                            world.add_item(item);
                            applied += 1;
                        }
                    }
                }
            }
        }
    }
    applied
}

pub(crate) fn reskin_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, ReskinPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                ReskinPlayerFacts {
                    state: player.area1_reskin_state(),
                    seen_timer: player.area1_reskin_seen_timer(),
                    gwendy_state: player.area1_gwendy_state(),
                    terion_state: player.area1_terion_state(),
                    logain_state: player.area1_logain_state(),
                    got_bits: player.area1_reskin_got_bits() as u32,
                    killed_guild_master: player.has_first_kill(16),
                },
            ))
        })
        .collect()
}

/// Applies each [`ReskinOutcomeEvent`] queued by
/// `World::process_reskin_actions`. See the module doc comment.
pub(crate) async fn apply_reskin_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    achievement_repository: &Option<ugaris_db::PgAchievementRepository>,
    events: Vec<ReskinOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            ReskinOutcomeEvent::UpdateState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_reskin_state(new_state);
                applied += 1;
            }
            ReskinOutcomeEvent::UpdateSeenTimer { player_id, value } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_reskin_seen_timer(value);
                applied += 1;
            }
            ReskinOutcomeEvent::UpdateGotBits { player_id, value } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_reskin_got_bits(value);
                applied += 1;
            }
            // C `questlog_open(co, 17)` (`src/system/questlog.c:204-217`):
            // sets the flag and unconditionally resends the questlog.
            ReskinOutcomeEvent::QuestOpen { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(QLOG_RESKIN);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, 17)` (`src/system/questlog.c:267-305`):
            // full exp-reward port via `QuestLog::complete_legacy`,
            // applied through `World::give_exp` (matching every other
            // quest-completion exp grant in this codebase), plus the
            // unconditional questlog resend. Reskin's own quest carries no
            // gold reward on completion itself (unlike the separate
            // alchemy-ingredient turn-in path below), so no achievement
            // wiring is needed here.
            ReskinOutcomeEvent::QuestDone { player_id } => {
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
                        .complete_legacy(QLOG_RESKIN, level, level_val)
                {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;
                }
            }
            ReskinOutcomeEvent::GoldEarned { player_id, amount } => {
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
            // C `achievement_award(co, ACHIEVEMENT_WELL_PAID_GATHERER, 1)`
            // (`gwendylon.c:4351`).
            ReskinOutcomeEvent::WellPaidGathererAchievement { player_id } => {
                award_reskin_well_paid_gatherer_achievement(
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
    applied
}

pub(crate) fn asturin_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, AsturinPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                AsturinPlayerFacts {
                    state: player.area1_asturin_state(),
                    seen_timer: player.area1_asturin_seen_timer(),
                },
            ))
        })
        .collect()
}

/// Applies each [`AsturinOutcomeEvent`] queued by
/// `World::process_asturin_actions`. See the module doc comment.
pub(crate) fn apply_asturin_events(
    runtime: &mut ServerRuntime,
    events: Vec<AsturinOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            AsturinOutcomeEvent::UpdateState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_asturin_state(new_state);
                applied += 1;
            }
            AsturinOutcomeEvent::UpdateSeenTimer { player_id, value } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_asturin_seen_timer(value);
                applied += 1;
            }
        }
    }
    applied
}

pub(crate) fn guiwynn_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, GuiwynnPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                GuiwynnPlayerFacts {
                    state: player.area1_guiwynn_state(),
                    seen_timer: player.area1_guiwynn_seen_timer(),
                    gwendy_state: player.area1_gwendy_state(),
                    quest8_done: player.quest_log.is_done(8),
                },
            ))
        })
        .collect()
}

/// C `create_money_item`'s sprite ladder (`src/system/tool.c:2222-2253`).
fn create_money_item_sprite(amount: u32) -> i32 {
    if amount > 9_999_999 {
        109
    } else if amount > 999_999 {
        108
    } else if amount > 99_999 {
        107
    } else if amount > 9_999 {
        106
    } else if amount > 999 {
        105
    } else if amount > 99 {
        104
    } else if amount > 9 {
        103
    } else if amount > 2 {
        102
    } else if amount == 2 {
        101
    } else if amount == 1 {
        100
    } else {
        0
    }
}

/// Applies each [`GuiwynnOutcomeEvent`] queued by
/// `World::process_guiwynn_actions`. See `world::guiwynn`'s module doc
/// comment for why [`GuiwynnOutcomeEvent::QuestDone`]'s money reward (and
/// [`GuiwynnOutcomeEvent::GrantKeyItem`]) need the `&mut ZoneLoader`
/// parameter this function (uniquely among this file's simpler
/// `apply_*_events` functions besides `apply_lydia_events`) takes.
pub(crate) async fn apply_guiwynn_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    events: Vec<GuiwynnOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            GuiwynnOutcomeEvent::UpdateState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_guiwynn_state(new_state);
                applied += 1;
            }
            GuiwynnOutcomeEvent::UpdateSeenTimer { player_id, value } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_guiwynn_seen_timer(value);
                applied += 1;
            }
            // C `questlog_open(co, 7)`/`questlog_open(co, 8)`
            // (`src/system/questlog.c:204-217`): sets the flag and
            // unconditionally resends the questlog.
            GuiwynnOutcomeEvent::QuestOpen { player_id, quest } => {
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
            // C `questlog_done(co, 7)`/`questlog_done(co, 8)`
            // (`src/system/questlog.c:267-305`) plus, only on first
            // completion (C's `if (tmp == 1)`), `create_money_item(...)`
            // + plain `give_char_item` (`gwendylon.c:4800-4805`/`4822-
            // 4827`) - see the module doc comment for why the money
            // reward can't stay a literal carried item via
            // `give_item_to_character` (that helper auto-converts
            // `IF_MONEY` items to gold, unlike C's plain `give_char_item`
            // here).
            GuiwynnOutcomeEvent::QuestDone { player_id, quest } => {
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

                    if completion.times_done == 1 {
                        let amount: u32 = (if quest == 7 {
                            MONEY_AREA1_MADMAGE1
                        } else {
                            MONEY_AREA1_MADMAGE2
                        })
                        .max(0) as u32;
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
            // C `!has_item(co, IID_AREA1_MADKEY1)` +
            // `create_item("mad_key1")` + plain `give_char_item`
            // (`gwendylon.c:4658-4664`, `4691-4697`, `4719-4725`).
            GuiwynnOutcomeEvent::GrantKeyItem { player_id } => {
                if let Ok(item) = loader.instantiate_item_template("mad_key1", Some(player_id)) {
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

pub(crate) fn logain_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, LogainPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                LogainPlayerFacts {
                    state: player.area1_logain_state(),
                    seen_timer: player.area1_logain_seen_timer(),
                    guiwynn_state: player.area1_guiwynn_state(),
                },
            ))
        })
        .collect()
}

/// Applies each [`LogainOutcomeEvent`] queued by
/// `World::process_logain_actions`. See `world::logain`'s module doc
/// comment for why [`LogainOutcomeEvent::QuestDone`]'s money reward (and
/// [`LogainOutcomeEvent::GrantMadKey6`]/[`LogainOutcomeEvent::
/// GrantMadKey9`]) need the `&mut ZoneLoader` parameter this function
/// takes.
pub(crate) async fn apply_logain_events(
    world: &mut World,
    runtime: &mut ServerRuntime,
    loader: &mut ZoneLoader,
    events: Vec<LogainOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            LogainOutcomeEvent::UpdateState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_logain_state(new_state);
                applied += 1;
            }
            LogainOutcomeEvent::UpdateSeenTimer { player_id, value } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_logain_seen_timer(value);
                applied += 1;
            }
            // C `questlog_open(co, 9)` (`src/system/questlog.c:204-217`):
            // sets the flag and unconditionally resends the questlog.
            LogainOutcomeEvent::QuestOpen { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(9);
                let payload = legacy_questlog_payload(player);
                for (session_id, _) in runtime.sessions_for_character(player_id) {
                    runtime.send_to_session(session_id, payload.clone());
                }
                applied += 1;
            }
            // C `questlog_done(co, 9)` (`src/system/questlog.c:267-305`)
            // plus, only on first completion (C's `if (tmp == 1)`),
            // `create_money_item(MONEY_AREA1_MADKNIGHT)` + plain
            // `give_char_item` (`gwendylon.c:5140-5145`) - see the module
            // doc comment for why the money reward can't stay a literal
            // carried item via `give_item_to_character` (that helper
            // auto-converts `IF_MONEY` items to gold, unlike C's plain
            // `give_char_item` here).
            LogainOutcomeEvent::QuestDone { player_id } => {
                let Some(level) = world.characters.get(&player_id).map(|c| c.level) else {
                    continue;
                };
                let level_val = level_value(level);
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                if let Some(completion) = player.quest_log.complete_legacy(9, level, level_val) {
                    let payload = legacy_questlog_payload(player);
                    world.give_exp(player_id, completion.granted_exp, u32::from(world.area_id));
                    for (session_id, _) in runtime.sessions_for_character(player_id) {
                        runtime.send_to_session(session_id, payload.clone());
                    }
                    applied += 1;

                    if completion.times_done == 1 {
                        let amount: u32 = MONEY_AREA1_MADKNIGHT.max(0) as u32;
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
            // C's `!has_item(co, IID_AREA1_MADKEY6)` + `create_item
            // ("mad_key6")` + plain `give_char_item` (`gwendylon.c:5009-
            // 5015`, `5029-5035`).
            LogainOutcomeEvent::GrantMadKey6 { player_id } => {
                if let Ok(item) = loader.instantiate_item_template("mad_key6", Some(player_id)) {
                    let item_id = item.id;
                    world.add_item(item);
                    if !world.give_char_item(player_id, item_id) {
                        world.destroy_item(item_id);
                    }
                    applied += 1;
                }
            }
            // C's `!has_item(co, IID_AREA1_MADKEY9)` + `create_item
            // ("mad_key9")` + plain `give_char_item` (`gwendylon.c:5022-
            // 5028`).
            LogainOutcomeEvent::GrantMadKey9 { player_id } => {
                if let Ok(item) = loader.instantiate_item_template("mad_key9", Some(player_id)) {
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

pub(crate) fn james_player_facts(
    runtime: &ServerRuntime,
) -> HashMap<CharacterId, JamesPlayerFacts> {
    runtime
        .players
        .values()
        .filter_map(|player| {
            let character_id = player.character_id?;
            Some((
                character_id,
                JamesPlayerFacts {
                    james_state: player.area1_james_state(),
                    lydia_state: player.area1_lydia_state(),
                    area1_flags: player.area1_flags(),
                },
            ))
        })
        .collect()
}

/// Applies each [`JamesOutcomeEvent`] queued by
/// `World::process_james_actions`. See `world::james`'s module doc
/// comment - James never touches `ZoneLoader` or achievements, unlike
/// Lydia/Guiwynn above.
pub(crate) fn apply_james_events(
    runtime: &mut ServerRuntime,
    events: Vec<JamesOutcomeEvent>,
) -> usize {
    let mut applied = 0;
    for event in events {
        match event {
            JamesOutcomeEvent::UpdateState {
                player_id,
                new_state,
            } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_james_state(new_state);
                applied += 1;
            }
            JamesOutcomeEvent::SetStorageHint { player_id } => {
                // C `#define AF1_STORAGE_HINT (1u << 1)` (`src/area/1/
                // area1.h:21`).
                const AF1_STORAGE_HINT: i32 = 1 << 1;
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.set_area1_flags(player.area1_flags() | AF1_STORAGE_HINT);
                applied += 1;
            }
            // C `questlog_open(co, QLOG_LYDIA)` (`src/system/
            // questlog.c:204-217`): sets the flag and unconditionally
            // resends the questlog.
            JamesOutcomeEvent::QuestOpen { player_id } => {
                let Some(player) = runtime.player_for_character_mut(player_id) else {
                    continue;
                };
                player.quest_log.open(QLOG_LYDIA);
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
