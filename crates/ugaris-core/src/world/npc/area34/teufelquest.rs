//! `CDR_TEUFELQUEST` (Teufelheim rat-hunt quest giver), ports
//! `src/area/34/teufel.c::teufelquest_driver` (`:1476-1608`) plus
//! `special_rat_reward` (`:1442-1474`).
//!
//! The rat-kill scoring half (`teufelrat_dead`'s `DRD_TEUFELRAT_PPD`
//! `kills`/`score` accumulation) was already fully ported in an earlier
//! iteration (`PlayerRuntime::teufel_rat_kills`/`teufel_rat_score`,
//! `world_events::death_hooks::apply_teufel_rat_death_from_hurt_event`) -
//! this driver only *reads* those two fields (via [`TeufelQuestPlayerFacts`])
//! and *writes* them (via [`TeufelQuestOutcomeEvent`]), the same
//! `World`/`PlayerRuntime` split established by
//! `world::npc::area29::spiritbran`/`world::npc::area3::kassim`.
//!
//! Unlike most NPC-file ports, [`World::process_teufelquest_actions`]
//! takes `&mut ZoneLoader` directly (not through a deferred outcome
//! event) since `special_rat_reward`'s two highest reward tiers call the
//! already-ported [`World::create_special_item`], which itself needs
//! `ZoneLoader` template access - same precedent as
//! `world::npc::area1::robber::process_robber_actions`.
//!
//! Deviations/gaps (documented, not silent):
//! - C's `teufelquest_driver` never handles `NT_CREATE` (unlike its
//!   sibling `teufelgambler_driver`, which parses `arg` into `dat->nr`) -
//!   [`TeufelQuestDriverData`] therefore only carries `memcleartimer`.
//! - C's unconditional `do_idle(cn, TICKS)` tail call is not ported,
//!   matching the established `world::npc::area33::gorwin` precedent for
//!   stationary dialogue NPCs.
//! - `teufeldemon_driver`'s own `NT_CHAR` self-defense hook (this file's
//!   sibling `CDR_TEUFELDEMON`) and `teufelgambler_driver`/
//!   `teufelrat_driver` remain unported - see `PORTING_TODO.md`'s Area 34
//!   entry.

use std::collections::HashMap;

use crate::character_driver::{
    mem_add_driver, mem_check_driver, mem_erase_driver, TextAnalysisOutcome,
};
use crate::drvlib::offset2dx;
use crate::world::npc::area34::{is_demon, teufel_analyse_text, TeufelTextOutcome};
use crate::world::*;

/// C `mem_check_driver(cn, co, 7)`/`mem_add_driver(cn, co, 7)` (`teufel.c:
/// 1508,1529`): the conventional "greet once" memory slot shared by every
/// ported NPC that uses driver memory this way.
const TEUFELQUEST_GREET_MEMORY_SLOT: usize = 7;
/// C `char_dist(cn, co) > 16` (`teufel.c:1502`).
const TEUFELQUEST_TALK_DISTANCE: i32 = 16;
/// C `TICKS * 60 * 60 * 12` (`teufel.c:1600`): 12-hour memory-erase
/// cadence.
const TEUFELQUEST_MEMORY_CLEAR_TICKS: u64 = TICKS_PER_SECOND * 60 * 60 * 12;

/// Per-player facts [`World::process_teufelquest_actions`] needs from
/// `crate::player::PlayerRuntime`, which `World` cannot see.
#[derive(Debug, Clone, Copy)]
pub struct TeufelQuestPlayerFacts {
    /// `PlayerRuntime::teufel_rat_kills`.
    pub teufel_rat_kills: u32,
    /// `PlayerRuntime::teufel_rat_score`.
    pub teufel_rat_score: u32,
}

/// A side effect [`World::process_teufelquest_actions`] could not apply
/// directly because it touches `PlayerRuntime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeufelQuestOutcomeEvent {
    /// C `ppd->kills = ...; ppd->score = ...;` - either the cash-out reset
    /// (`teufel.c:1555-1556,1564-1565,1572-1573`, always `0`/`0`) or the
    /// god-only debug set (`:1577-1578`, `500`/`25000`).
    SetRatKillsScore {
        player_id: CharacterId,
        kills: u32,
        score: u32,
    },
}

impl World {
    /// C `teufelquest_driver`'s per-tick body (`teufel.c:1476-1608`).
    pub fn process_teufelquest_actions(
        &mut self,
        loader: &mut ZoneLoader,
        player_facts: &HashMap<CharacterId, TeufelQuestPlayerFacts>,
        area_id: u16,
    ) -> Vec<TeufelQuestOutcomeEvent> {
        let quest_ids: Vec<CharacterId> = self
            .characters
            .values()
            .filter(|character| {
                character.driver == CDR_TEUFELQUEST
                    && character.flags.contains(CharacterFlags::USED)
                    && !character.flags.contains(CharacterFlags::DEAD)
            })
            .map(|character| character.id)
            .collect();

        let mut events = Vec::new();
        for quest_id in quest_ids {
            self.process_teufelquest_messages(loader, quest_id, player_facts, area_id, &mut events);
        }
        events
    }

    fn process_teufelquest_messages(
        &mut self,
        loader: &mut ZoneLoader,
        quest_id: CharacterId,
        player_facts: &HashMap<CharacterId, TeufelQuestPlayerFacts>,
        area_id: u16,
        events: &mut Vec<TeufelQuestOutcomeEvent>,
    ) {
        let Some(CharacterDriverState::TeufelQuest(mut data)) = self
            .characters
            .get(&quest_id)
            .and_then(|character| character.driver_state.clone())
        else {
            return;
        };

        let messages = self
            .characters
            .get_mut(&quest_id)
            .map(|character| std::mem::take(&mut character.driver_messages))
            .unwrap_or_default();

        let mut face_target: Option<(i32, i32)> = None;

        for message in &messages {
            match message.message_type {
                NT_CHAR => {
                    self.teufelquest_handle_char_message(quest_id, message, &mut face_target)
                }
                NT_TEXT => self.teufelquest_handle_text_message(
                    loader,
                    quest_id,
                    message,
                    player_facts,
                    area_id,
                    &mut face_target,
                    events,
                ),
                _ => {}
            }
        }

        // C `if (talkdir) turn(cn, talkdir);` (`teufel.c:1603-1605`).
        if let (Some(quest), Some((tx, ty))) =
            (self.characters.get(&quest_id).cloned(), face_target)
        {
            if let Some(direction) = offset2dx(i32::from(quest.x), i32::from(quest.y), tx, ty) {
                if let Some(quest_mut) = self.characters.get_mut(&quest_id) {
                    let _ = turn(quest_mut, direction as u8);
                }
            }
        }

        // C `if (ticker > dat->memcleartimer) { mem_erase_driver(cn, 7);
        // dat->memcleartimer = ticker + TICKS*60*60*12; }`
        // (`teufel.c:1598-1601`).
        let tick = self.tick.0;
        if tick > data.memcleartimer {
            if let Some(quest) = self.characters.get_mut(&quest_id) {
                mem_erase_driver(&mut quest.driver_memory, TEUFELQUEST_GREET_MEMORY_SLOT);
            }
            data.memcleartimer = tick + TEUFELQUEST_MEMORY_CLEAR_TICKS;
        }

        if let Some(quest) = self.characters.get_mut(&quest_id) {
            quest.driver_state = Some(CharacterDriverState::TeufelQuest(data));
        }
    }

    /// C `teufelquest_driver`'s `NT_CHAR` branch (`teufel.c:1492-1530`).
    fn teufelquest_handle_char_message(
        &mut self,
        quest_id: CharacterId,
        message: &CharacterDriverMessage,
        face_target: &mut Option<(i32, i32)>,
    ) {
        let seen_id = CharacterId(message.dat1.max(0) as u32);
        let Some(quest) = self.characters.get(&quest_id).cloned() else {
            return;
        };
        let Some(seen) = self.characters.get(&seen_id).cloned() else {
            return;
        };

        // C `if (!char_see_char(cn, co) || cn == co) { remove_message;
        // continue; }` (`teufel.c:1496-1499`).
        if quest_id == seen_id || !char_see_char(&quest, &seen, &self.map, self.date.daylight) {
            return;
        }
        // C `if (char_dist(cn, co) > 16) { remove_message; continue; }`
        // (`teufel.c:1502-1505`).
        if char_dist(&quest, &seen) > TEUFELQUEST_TALK_DISTANCE {
            return;
        }
        // C `if (mem_check_driver(cn, co, 7)) { remove_message; continue;
        // }` (`teufel.c:1508-1511`).
        if mem_check_driver(
            &quest.driver_memory,
            TEUFELQUEST_GREET_MEMORY_SLOT,
            seen_id.0,
        ) {
            return;
        }
        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`teufel.c:1513-1516`).
        if !seen.flags.contains(CharacterFlags::PLAYER) {
            return;
        }

        // C `if (!is_demon(co)) say(...); else say(...);`
        // (`teufel.c:1518-1526`).
        if !is_demon(seen.sprite) {
            self.npc_say(quest_id, "Ah human? AAAAAHHHHHHHHHH! HELP!");
        } else {
            self.npc_say_bytes(
                quest_id,
                &format!(
                    "Hello, {}! We have a slight rat problem in the caverns to the north. There's a nice \u{E0C4}reward\u{E0C0} for killing some rats.",
                    seen.name
                ),
            );
        }

        *face_target = Some((i32::from(seen.x), i32::from(seen.y)));
        if let Some(quest_mut) = self.characters.get_mut(&quest_id) {
            mem_add_driver(
                &mut quest_mut.driver_memory,
                TEUFELQUEST_GREET_MEMORY_SLOT,
                seen_id.0,
            );
        }
    }

    /// C `teufelquest_driver`'s `NT_TEXT` branch (`teufel.c:1533-1587`),
    /// wired through the shared [`teufel_analyse_text`] matcher.
    #[allow(clippy::too_many_arguments)]
    fn teufelquest_handle_text_message(
        &mut self,
        loader: &mut ZoneLoader,
        quest_id: CharacterId,
        message: &CharacterDriverMessage,
        player_facts: &HashMap<CharacterId, TeufelQuestPlayerFacts>,
        area_id: u16,
        face_target: &mut Option<(i32, i32)>,
        events: &mut Vec<TeufelQuestOutcomeEvent>,
    ) {
        let speaker_id = CharacterId(message.dat3.max(0) as u32);
        // C `if (!(ch[co].flags & CF_PLAYER)) { remove_message; continue;
        // }` (`teufel.c:1536-1539`).
        let Some(speaker) = self.characters.get(&speaker_id).cloned() else {
            return;
        };
        if !speaker.flags.contains(CharacterFlags::PLAYER) {
            return;
        }
        // C `ppd = set_data(co, DRD_TEUFELRAT_PPD, ...); if (!ppd) {
        // remove_message; continue; }` (`teufel.c:1540-1544`).
        let Some(facts) = player_facts.get(&speaker_id).copied() else {
            return;
        };
        let Some(quest) = self.characters.get(&quest_id).cloned() else {
            return;
        };
        let Some(text) = message.text.as_deref() else {
            return;
        };

        // C `if ((n = analyse_text_driver(...))) { ... }`
        // (`teufel.c:1546-1586`) - see `teufel_analyse_text`'s own doc
        // comment for why `TeufelTextOutcome::Recognized` covers every
        // non-filtered outcome, matched or not.
        let TeufelTextOutcome::Recognized(outcome) =
            teufel_analyse_text(self, &quest, &speaker, text)
        else {
            return;
        };

        // C `talkdir = offset2dx(ch[cn].x, ch[cn].y, ch[co].x, ch[co].y);`
        // (`teufel.c:1547`).
        *face_target = Some((i32::from(speaker.x), i32::from(speaker.y)));

        let mut final_kills = facts.teufel_rat_kills;
        let mut final_score = facts.teufel_rat_score;
        let mut mutated = false;
        let mut reward_score: Option<u32> = None;

        match outcome {
            TextAnalysisOutcome::Said(reply) => {
                self.npc_quiet_say_bytes(quest_id, &reply);
            }
            // C's own `case 1:` inside `analyse_text_driver` itself
            // (`teufel.c:338-340`), not `teufelquest_driver`'s switch.
            TextAnalysisOutcome::Matched(1) => {
                self.npc_quiet_say(quest_id, &format!("I'm {}.", quest.name));
            }
            // C `case 5:` (`teufel.c:1549-1557`): experience.
            TextAnalysisOutcome::Matched(5) => {
                let tmp = (facts.teufel_rat_score / 20) as i64 * i64::from(quest.level);
                self.npc_say(
                    quest_id,
                    &format!(
                        "Experience it is. You killed {} rats for a total score of {}.",
                        facts.teufel_rat_kills, facts.teufel_rat_score
                    ),
                );
                self.give_exp(speaker_id, tmp, u32::from(area_id));
                reward_score = Some(facts.teufel_rat_score);
                final_kills = 0;
                final_score = 0;
                mutated = true;
            }
            // C `case 6:` (`teufel.c:1558-1566`): military.
            TextAnalysisOutcome::Matched(6) => {
                let tmp = (facts.teufel_rat_score / 1250) as i32;
                self.npc_say(
                    quest_id,
                    &format!(
                        "Military knowledge it is. You killed {} rats for a total score of {}.",
                        facts.teufel_rat_kills, facts.teufel_rat_score
                    ),
                );
                self.give_military_pts(speaker_id, tmp, 1, u32::from(area_id));
                reward_score = Some(facts.teufel_rat_score);
                final_kills = 0;
                final_score = 0;
                mutated = true;
            }
            // C `case 7:` (`teufel.c:1567-1574`): money.
            TextAnalysisOutcome::Matched(7) => {
                let tmp = facts.teufel_rat_score.saturating_mul(12);
                self.npc_say(
                    quest_id,
                    &format!(
                        "Money it is. You killed {} rats for a total score of {}.",
                        facts.teufel_rat_kills, facts.teufel_rat_score
                    ),
                );
                self.teufelquest_give_money(speaker_id, tmp);
                reward_score = Some(facts.teufel_rat_score);
                final_kills = 0;
                final_score = 0;
                mutated = true;
            }
            // C `case 8:` (`teufel.c:1575-1580`): god-only debug set.
            TextAnalysisOutcome::Matched(8) => {
                if speaker.flags.contains(CharacterFlags::GOD) {
                    final_kills = 500;
                    final_score = 25_000;
                    mutated = true;
                }
            }
            TextAnalysisOutcome::Matched(_) | TextAnalysisOutcome::NoMatch => {}
        }

        if mutated {
            events.push(TeufelQuestOutcomeEvent::SetRatKillsScore {
                player_id: speaker_id,
                kills: final_kills,
                score: final_score,
            });
        }
        if let Some(score) = reward_score {
            self.teufelquest_special_rat_reward(loader, quest_id, speaker_id, score);
        }
        // C `if (!ppd->score) { log_char(co, LOG_SYSTEM, 0, "#90");
        // log_char(co, LOG_SYSTEM, 0, "#80"); }` (`teufel.c:1582-1585`) -
        // checked against the *post-switch* value, so it also fires for
        // an already-zero score on an unrelated recognized message (a
        // real, harmless C quirk, kept verbatim).
        if final_score == 0 {
            self.queue_system_text(speaker_id, "#90");
            self.queue_system_text(speaker_id, "#80");
        }
    }

    /// C `special_rat_reward(cn, co, ppd)` (`teufel.c:1442-1474`).
    fn teufelquest_special_rat_reward(
        &mut self,
        loader: &mut ZoneLoader,
        quest_id: CharacterId,
        player_id: CharacterId,
        score: u32,
    ) {
        let (pts, item) = if score >= 100_000 {
            (100_000, self.create_special_item(loader, 20, 90, 1, 50))
        } else if score >= 50_000 {
            (50_000, self.create_special_item(loader, 20, 90, 1, 250))
        } else if score >= 25_000 {
            (25_000, self.create_special_item(loader, 20, 90, 1, 1_000))
        } else if score >= 10_000 {
            (10_000, self.create_special_item(loader, 20, 90, 1, 10_000))
        } else if score >= 5_000 {
            (
                5_000,
                loader
                    .instantiate_item_template("healing_potion3", None)
                    .ok(),
            )
        } else if score >= 2_500 {
            (
                2_500,
                loader
                    .instantiate_item_template("healing_potion2", None)
                    .ok(),
            )
        } else if score >= 1_000 {
            (
                1_000,
                loader
                    .instantiate_item_template("healing_potion1", None)
                    .ok(),
            )
        } else {
            return;
        };

        let Some(item) = item else {
            return;
        };
        let item_name = item.name.clone();
        let item_id = item.id;
        self.items.insert(item_id, item);
        self.npc_say(
            quest_id,
            &format!("Here's a little extra for scoring {pts} points in one go: {item_name}!"),
        );
        if !self.give_char_item(player_id, item_id) {
            self.items.remove(&item_id);
        }
    }

    /// C `give_money(cn, val, reason)` (`src/system/tool.c:1460-1474`),
    /// same local-method shape as `world::npc::area29::countbran`'s
    /// `countbran_give_money` (no achievement-ladder tracking - that's the
    /// `ugaris-server`-side `achievement::give_money`, unreachable from a
    /// pure `World` NPC driver).
    fn teufelquest_give_money(&mut self, player_id: CharacterId, amount: u32) {
        if let Some(player) = self.characters.get_mut(&player_id) {
            player.gold = player.gold.saturating_add(amount);
            player.flags.insert(CharacterFlags::ITEMS);
        }
        self.queue_system_text_bytes(player_id, give_money_message(amount));
    }
}

// ---- legacy driver registry surface ----

use crate::character_driver::CDR_TEUFELQUEST;

/// C `struct gamble_data { int memcleartimer; int nr; }` (`teufel.c:
/// 1229-1232`), narrowed to the one field `teufelquest_driver` actually
/// reads/writes (`nr` is only ever set by `teufelgambler_driver`'s own
/// `NT_CREATE` handler - see this module's doc comment).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TeufelQuestDriverData {
    #[serde(default)]
    pub memcleartimer: u64,
}
